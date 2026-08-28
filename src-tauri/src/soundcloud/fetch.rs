use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::redirect::Policy;
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};
use tauri::AppHandle;

use crate::settings;

pub fn sanitize_cookie_value(value: &str) -> String {
    let mut v = value.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
    for marker in [
        ".soundcloud.",
        ".octocaptcha.",
        ".captcha-delivery.",
        ".datadome.co",
        ".api-v2.",
    ] {
        if let Some(i) = v.find(marker) {
            v.truncate(i);
        }
    }
    v.trim_end_matches('/').trim().to_string()
}

pub fn upsert_cookie(app: &AppHandle, name: &str, value: &str, domain: &str) -> Result<(), String> {
    let value = sanitize_cookie_value(value);
    if value.is_empty() {
        return Ok(());
    }
    let store = cookies_path(app);
    let mut jar = load_cookies(&store);
    jar.insert(
        name.to_string(),
        json!({
            "value": value,
            "domain": domain,
            "path": "/",
            "secure": true,
            "httpOnly": true,
        }),
    );
    save_cookies(&store, &jar)
}

pub async fn import_session_cookie(app: &AppHandle, cookie_value: &str) -> Result<(), String> {
    upsert_cookie(app, "_soundcloud_session", cookie_value, ".soundcloud.com")
}

pub async fn token_works(app: &AppHandle, token: &str, client_id: &str) -> bool {
    let url = format!("https://api-v2.soundcloud.com/me?client_id={client_id}");
    match sc_fetch(app, &url, token, client_id, "GET").await {
        Ok(v) => v.get("data").and_then(|d| d.get("id")).is_some(),
        Err(_) => false,
    }
}

fn cookies_path(app: &AppHandle) -> PathBuf {
    settings::app_data_dir(app).join("sc_cookies.json")
}

fn load_cookies(path: &PathBuf) -> serde_json::Map<String, Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
        .as_object()
        .cloned()
        .unwrap_or_default()
}

fn save_cookies(path: &PathBuf, jar: &serde_json::Map<String, Value>) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(jar).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

const SC_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0";

fn build_client(is_write: bool) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(12));
    if is_write {
        builder = builder.redirect(Policy::none());
    }
    builder.build().map_err(|e| e.to_string())
}

pub fn log_write_line(app: &AppHandle, method: &str, url: &str, status: u16, body: &str) {
    log_write(app, method, url, status, body);
}

pub fn session_cookie_value(app: &AppHandle) -> Option<String> {
    cookie_named(app, "_soundcloud_session")
}

pub fn cookie_named(app: &AppHandle, name: &str) -> Option<String> {
    let jar = load_cookies(&cookies_path(app));
    jar.get(name)
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .map(sanitize_cookie_value)
        .filter(|v| !v.is_empty())
}

fn log_write(app: &AppHandle, method: &str, url: &str, status: u16, body: &str) {
    let path = settings::app_data_dir(app).join("sc_write.log");
    let path_only = url.split('?').next().unwrap_or(url);
    let snippet: String = body.chars().take(240).collect::<String>().replace(['\n', '\r'], " ");
    let line = format!("{method} {path_only} -> {status} {snippet}\n");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn cookie_entries(app: &AppHandle) -> Vec<(String, String)> {
    let path = cookies_path(app);
    let jar = load_cookies(&path);
    let mut out = Vec::new();
    let mut cleaned = jar.clone();
    let mut dirty = false;
    for (name, val) in &jar {
        let Some(raw) = val.get("value").and_then(|x| x.as_str()) else {
            continue;
        };
        let value = sanitize_cookie_value(raw);
        if value.is_empty() {
            continue;
        }
        if value != raw {
            dirty = true;
            if let Some(obj) = cleaned.get_mut(name).and_then(|x| x.as_object_mut()) {
                obj.insert("value".into(), json!(value.clone()));
            }
        }
        out.push((name.clone(), value));
    }
    if dirty {
        let _ = save_cookies(&path, &cleaned);
    }
    out
}

fn cookie_header(app: &AppHandle) -> Option<String> {
    let parts: Vec<String> = cookie_entries(app)
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn datadome_value(app: &AppHandle) -> Option<String> {
    cookie_entries(app)
        .into_iter()
        .find(|(name, _)| name == "datadome")
        .map(|(_, value)| value)
}

fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn proxy_base() -> String {
    std::env::var("AOI_ROOMS_URL")
        .unwrap_or_else(|_| "https://aoi-rooms.elvishedcc.workers.dev".into())
        .trim_end_matches('/')
        .to_string()
}

fn proxy_bases() -> Vec<String> {
    let primary = proxy_base();
    let mut bases = vec![primary];
    if let Ok(extra) = std::env::var("AOI_PROXY_MIRRORS") {
        for part in extra.split(',') {
            let p = part.trim().trim_end_matches('/');
            if !p.is_empty() && !bases.iter().any(|b| b == p) {
                bases.push(p.to_string());
            }
        }
    }
    bases
}

fn proxy_candidates(app: &AppHandle, url: &str) -> Vec<String> {
    let settings = settings::read_settings_file(app);
    let use_proxy = settings
        .get("useAoiProxy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !use_proxy {
        return vec![url.to_string()];
    }
    let mut out: Vec<String> = proxy_bases()
        .into_iter()
        .map(|b| format!("{b}/sc/proxy?url={}", encode_uri_component(url)))
        .collect();
    out.push(url.to_string());
    out
}

#[allow(dead_code)]
fn maybe_proxy_url(app: &AppHandle, url: &str) -> String {
    proxy_candidates(app, url)
        .into_iter()
        .next()
        .unwrap_or_else(|| url.to_string())
}

pub async fn sc_fetch(
    app: &AppHandle,
    url: &str,
    token: &str,
    client_id: &str,
    method: &str,
) -> Result<Value, String> {
    let method_up = method.to_ascii_uppercase();
    let is_write = matches!(method_up.as_str(), "PUT" | "POST" | "DELETE" | "PATCH");
    let is_public_api = url.contains("://api.soundcloud.com/");
    let mut full_url = if url.contains("client_id=") {
        url.to_string()
    } else {
        let sep = if url.contains('?') { '&' } else { '?' };
        format!("{url}{sep}client_id={client_id}")
    };
    if is_write && !is_public_api {
        if !full_url.contains("app_locale=") {
            full_url.push_str("&app_locale=en");
        }
        if !full_url.contains("app_version=") {
            full_url.push_str("&app_version=1766000000");
        }
    }
    let is_likes_collection = !is_write
        && (full_url.contains("/likes?")
            || full_url.contains("/likes&")
            || full_url.contains("/likes/")
            || full_url.ends_with("/likes"))
        && !full_url.contains("track_likes");
    if is_likes_collection && !full_url.contains("linked_partitioning=") {
        full_url.push_str("&linked_partitioning=1");
    }

    let client = build_client(is_write)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("OAuth {token}")).map_err(|e| e.to_string())?,
    );
    headers.insert(
        "Accept",
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert("Origin", HeaderValue::from_static("https://soundcloud.com"));
    headers.insert("Referer", HeaderValue::from_static("https://soundcloud.com/"));
    headers.insert(USER_AGENT, HeaderValue::from_static(SC_UA));
    headers.insert(
        "Accept-Language",
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    if is_write {
        headers.insert("X-Requested-With", HeaderValue::from_static("XMLHttpRequest"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-site"));
        if method_up == "POST" {
            headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        }
    }
    if let Some(cookie) = cookie_header(app) {
        if let Ok(v) = HeaderValue::from_str(&cookie) {
            headers.insert(reqwest::header::COOKIE, v);
        }
    }
    if let Some(dd) = datadome_value(app) {
        if let Ok(v) = HeaderValue::from_str(&dd) {
            headers.insert("x-datadome-clientid", v);
        }
    }

    let candidates = proxy_candidates(app, &full_url);
    let max_attempts = 120usize;
    let mut attempt = 0usize;

    loop {
        let try_url = &candidates[attempt % candidates.len()];
        let mut req = client
            .request(
                reqwest::Method::from_bytes(method_up.as_bytes()).unwrap_or(reqwest::Method::GET),
                try_url,
            )
            .headers(headers.clone());
        if method_up == "POST" {
            req = req.body("{}");
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                attempt += 1;
                if attempt >= max_attempts {
                    return Err(e.to_string());
                }
                tokio::time::sleep(std::time::Duration::from_millis(280)).await;
                continue;
            }
        };
        let status = resp.status();
        let code = status.as_u16();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        if is_write {
            log_write(app, &method_up, &full_url, code, &text);
        }

        let ok = status.is_success() || code == 409 || (code == 404 && method_up == "DELETE");
        if ok {
            if text.trim().is_empty() {
                return Ok(json!({ "data": null }));
            }
            return match serde_json::from_str::<Value>(&text) {
                Ok(data) => Ok(json!({ "data": data })),
                Err(_) => Ok(json!({ "data": null })),
            };
        }

        let retryable = matches!(code, 429 | 502 | 503 | 504);
        if retryable && attempt + 1 < max_attempts {
            attempt += 1;
            tokio::time::sleep(std::time::Duration::from_millis(280)).await;
            continue;
        }
        return Ok(json!({ "error": code }));
    }
}
