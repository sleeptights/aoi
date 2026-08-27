use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tauri::http::{header::*, Request, Response, StatusCode};

const MAX_RANGE: u64 = 1000 * 1024;
const MAX_FULL: u64 = 32 * 1024 * 1024;

pub fn response(app: &tauri::AppHandle, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let origin = request
        .headers()
        .get(ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("*")
        .to_string();

    let deny = |status: StatusCode| {
        Response::builder()
            .status(status)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, &origin)
            .body(Vec::new())
            .unwrap_or_else(|_| Response::new(Vec::new()))
    };

    if request.method() == tauri::http::Method::OPTIONS {
        return Response::builder()
            .status(StatusCode::OK)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, &origin)
            .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")
            .header(ACCESS_CONTROL_ALLOW_HEADERS, "range")
            .header(ACCESS_CONTROL_EXPOSE_HEADERS, "content-range, accept-ranges")
            .body(Vec::new())
            .unwrap_or_else(|_| Response::new(Vec::new()));
    }

    let Some(path) = path_from_uri(request.uri()) else {
        return deny(StatusCode::BAD_REQUEST);
    };
    if !path_allowed(app, &path) {
        return deny(StatusCode::FORBIDDEN);
    }
    if !path.is_file() {
        return deny(StatusCode::NOT_FOUND);
    }

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return deny(StatusCode::NOT_FOUND),
    };
    let len = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => return deny(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let mime = mime_from_path(&path);

    let range_header = request
        .headers()
        .get(RANGE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let range_header = if range_header.is_none() && len > MAX_FULL {
        Some(format!("bytes=0-{}", MAX_RANGE.saturating_sub(1)))
    } else {
        range_header
    };

    if let Some(header) = range_header {
        let Some((start, mut end)) = parse_bytes_range(&header, len) else {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, &origin)
                .header(CONTENT_RANGE, format!("bytes */{len}"))
                .body(Vec::new())
                .unwrap_or_else(|_| Response::new(Vec::new()));
        };
        end = start + (end - start).min(len.saturating_sub(start)).min(MAX_RANGE.saturating_sub(1));
        if start >= len || end >= len || end < start {
            return deny(StatusCode::RANGE_NOT_SATISFIABLE);
        }
        let nbytes = end + 1 - start;
        let mut buf = vec![0u8; nbytes as usize];
        if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut buf).is_err() {
            return deny(StatusCode::INTERNAL_SERVER_ERROR);
        }
        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, &origin)
            .header(ACCESS_CONTROL_EXPOSE_HEADERS, "content-range, accept-ranges")
            .header(ACCEPT_RANGES, "bytes")
            .header(CONTENT_TYPE, mime)
            .header(CONTENT_RANGE, format!("bytes {start}-{end}/{len}"))
            .header(CONTENT_LENGTH, nbytes)
            .body(buf)
            .unwrap_or_else(|_| Response::new(Vec::new()));
    }

    let mut buf = Vec::with_capacity(len.min(8 * 1024 * 1024) as usize);
    if file.read_to_end(&mut buf).is_err() {
        return deny(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, &origin)
        .header(ACCEPT_RANGES, "bytes")
        .header(CONTENT_TYPE, mime)
        .header(CONTENT_LENGTH, buf.len() as u64)
        .body(buf)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

fn path_allowed(app: &tauri::AppHandle, path: &Path) -> bool {
    let Ok(canon) = path.canonicalize() else {
        return false;
    };
    let data = crate::settings::app_data_dir(app);
    if let Ok(d) = data.canonicalize() {
        if canon.starts_with(&d) {
            return true;
        }
    }
    let settings = crate::settings::read_settings_file(app);
    if let Some(folder) = settings.get("musicFolder").and_then(|v| v.as_str()) {
        if !folder.is_empty() {
            if let Ok(root) = PathBuf::from(folder).canonicalize() {
                if canon.starts_with(&root) {
                    return true;
                }
            }
        }
    }
    false
}

fn path_from_uri(uri: &tauri::http::Uri) -> Option<PathBuf> {
    let raw = uri.path();
    if raw.is_empty() {
        return None;
    }
    let decoded = percent_decode(raw.trim_start_matches('/'));
    if decoded.is_empty() || decoded.split(['/', '\\']).any(|p| p == "..") {
        return None;
    }
    let decoded = if cfg!(windows) {
        decoded.replace('/', "\\")
    } else if !decoded.starts_with('/') {
        format!("/{decoded}")
    } else {
        decoded
    };
    Some(PathBuf::from(decoded))
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_bytes_range(header: &str, size: u64) -> Option<(u64, u64)> {
    if size == 0 {
        return None;
    }
    let rest = header.trim().strip_prefix("bytes=")?;
    let first = rest.split(',').next()?.trim();
    if let Some(suffix) = first.strip_prefix('-') {
        let n: u64 = suffix.parse().ok()?;
        if n == 0 {
            return None;
        }
        let start = size.saturating_sub(n);
        return Some((start, size - 1));
    }
    let (start_s, end_s) = first.split_once('-')?;
    let start: u64 = start_s.parse().ok()?;
    let end = if end_s.is_empty() {
        size - 1
    } else {
        end_s.parse().ok()?
    };
    if start > end || start >= size {
        return None;
    }
    Some((start, end.min(size - 1)))
}

fn mime_from_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("wav") => "audio/wav",
        Some("ogg") | Some("oga") => "audio/ogg",
        Some("m4a") | Some("aac") => "audio/mp4",
        Some("opus") => "audio/opus",
        Some("wma") => "audio/x-ms-wma",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
