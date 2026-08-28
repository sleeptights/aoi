use base64::{engine::general_purpose::STANDARD, Engine as _};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::Serialize;
use crate::settings;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use walkdir::WalkDir;

const AUDIO_EXTS: &[&str] = &[
    ".mp3", ".flac", ".wav", ".ogg", ".oga", ".m4a", ".aac", ".opus", ".wma", ".wv", ".aiff", ".aif",
];

fn normalize_fs_path(raw: &str) -> PathBuf {
    let s = raw.trim().trim_matches('"');
    let s = s.strip_prefix("file:///").or_else(|| s.strip_prefix("file://")).unwrap_or(s);
    let s = percent_decode(s);
    let s = if cfg!(windows) { s.replace('/', "\\") } else { s };
    PathBuf::from(s)
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &s[i + 1..i + 3];
            if let Ok(v) = u8::from_str_radix(hex, 16) {
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

#[derive(Serialize)]
pub struct LocalTrack {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub path: String,
    pub color: String,
    pub duration: u64,
}

fn stable_id(path: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    if h == 0 { 1 } else { h }
}

fn hash_color(input: &str) -> String {
    let mut h: i32 = 0;
    for c in input.chars() {
        h = h.wrapping_shl(5).wrapping_sub(h).wrapping_add(c as i32);
    }
    let hue = h.unsigned_abs() % 360;
    format!("hsl({hue},52%,40%)")
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTS.contains(&format!(".{}", e.to_lowercase()).as_str()))
        .unwrap_or(false)
}

fn parse_track(path: &Path, id: u64, min_duration: u64) -> Option<LocalTrack> {
    let base = path.file_stem()?.to_str()?;
    let full = path.to_string_lossy().to_string();

    let mut artist = "Неизвестно".to_string();
    let mut title = base.to_string();
    let mut duration = 0u64;

    if let Some(idx) = base.find(" - ") {
        artist = base[..idx].trim().to_string();
        title = base[idx + 3..].trim().to_string();
    }

    if let Ok(tagged) = Probe::open(path).and_then(|p| p.read()) {
        if let Some(tag) = tagged.primary_tag() {
            if let Some(t) = tag.title().as_deref() {
                title = t.to_string();
            }
            if let Some(a) = tag.artist().as_deref() {
                artist = a.to_string();
            }
        }
        duration = tagged.properties().duration().as_secs();
    }

    if duration == 0 {
        // tags missing / unreadable — keep the file; only drop short tracks we could measure
    } else if duration < min_duration {
        return None;
    }

    Some(LocalTrack {
        id,
        title,
        artist,
        path: full,
        color: hash_color(base),
        duration,
    })
}

#[tauri::command]
pub async fn select_music_folder(app: AppHandle) -> Result<Option<String>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("Выбрать папку с музыкой")
        .blocking_pick_folder();
    Ok(picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn scan_music_folder(
    folder_path: String,
    min_duration: Option<u64>,
) -> Result<Vec<LocalTrack>, String> {
    let min_duration = min_duration.unwrap_or(30);
    if folder_path.is_empty() {
        return Ok(vec![]);
    }

    let root = normalize_fs_path(&folder_path);
    if !root.is_dir() {
        return Ok(vec![]);
    }

    let files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| is_audio(p))
        .collect();

    let mut out = Vec::new();
    for file in files {
        let path = file.to_string_lossy().to_string();
        if let Some(track) = parse_track(&file, stable_id(&path), min_duration) {
            out.push(track);
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn get_cover_art(file_path: String) -> Result<Option<String>, String> {
    let path = normalize_fs_path(&file_path);
    let tagged = match Probe::open(&path).and_then(|p| p.read()) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    let tag = match tagged.primary_tag() {
        Some(t) => t,
        None => return Ok(None),
    };
    let picture = match tag.pictures().first() {
        Some(p) => p,
        None => return Ok(None),
    };
    let mime = picture
        .mime_type()
        .map(|m| m.as_str())
        .unwrap_or("image/jpeg");
    let b64 = STANDARD.encode(picture.data());
    Ok(Some(format!("data:{mime};base64,{b64}")))
}

const IMAGE_EXTS: &[&str] = &[".jpg", ".jpeg", ".png", ".webp", ".gif", ".bmp"];

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&format!(".{}", e.to_lowercase()).as_str()))
        .unwrap_or(false)
}

#[tauri::command]
pub async fn select_player_bg_image(app: AppHandle) -> Result<Option<String>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("Выбрать фон плеера")
        .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "gif", "bmp"])
        .blocking_pick_file();
    let Some(file) = picked else {
        return Ok(None);
    };
    let src = file.into_path().map_err(|e| e.to_string())?;
    if !src.is_file() || !is_image(&src) {
        return Err("not an image".into());
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let dest_dir = settings::app_data_dir(&app).join("player_bg");
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    for old in ["bg.jpg", "bg.jpeg", "bg.png", "bg.webp", "bg.gif", "bg.bmp"] {
        let p = dest_dir.join(old);
        if p.exists() {
            let _ = fs::remove_file(p);
        }
    }
    let dest = dest_dir.join(format!("bg.{ext}"));
    fs::copy(&src, &dest).map_err(|e| e.to_string())?;
    Ok(Some(dest.to_string_lossy().into_owned()))
}
