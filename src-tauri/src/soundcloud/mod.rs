mod browser;
mod fetch;
mod write;

use serde::Serialize;
use serde_json::Value;
use std::{fs, path::PathBuf};
use tauri::AppHandle;

use crate::settings;

pub fn sc_covers_dir(app: &AppHandle) -> PathBuf {
    settings::app_data_dir(app).join("sc_covers")
}

fn sc_likes_file(app: &AppHandle) -> PathBuf {
    settings::app_data_dir(app).join("sc_likes.json")
}

fn file_url(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[derive(Serialize)]
pub struct ScCreds {
    pub token: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
}

#[tauri::command]
pub async fn sc_login(app: AppHandle) -> Result<Option<ScCreds>, String> {
    write::close_bridge(&app);
    // Login happens inside an aoi WebView window (not the system browser),
    // so Chrome/Edge users get working likes/follows the same as Firefox.
    browser::wait_for_login(&app).await
}

#[tauri::command]
pub async fn sc_fetch(
    app: AppHandle,
    url: String,
    token: String,
    client_id: String,
    method: Option<String>,
    http_method: Option<String>,
) -> Result<Value, String> {
    let method = http_method
        .or(method)
        .unwrap_or_else(|| "GET".into())
        .to_ascii_uppercase();
    let is_write = matches!(method.as_str(), "PUT" | "POST" | "DELETE" | "PATCH");
    if is_write {
        browser::refresh_soundcloud_cookies(&app);
    }
    let result = match fetch::sc_fetch(&app, &url, &token, &client_id, &method).await {
        Ok(v) if is_write && write_blocked(&v) => {
            write::sc_write(&app, &url, &token, &client_id, &method).await?
        }
        Ok(v) => v,
        Err(_) if is_write => write::sc_write(&app, &url, &token, &client_id, &method).await?,
        Err(e) => return Err(e),
    };
    Ok(result)
}

fn write_blocked(v: &Value) -> bool {
    match v.get("error") {
        Some(Value::Number(n)) => matches!(n.as_u64().unwrap_or(0), 0 | 401 | 403 | 429),
        Some(Value::String(s)) => !s.is_empty(),
        _ => false,
    }
}

fn safe_cover_id(id: &str) -> Option<&str> {
    if id.is_empty() || id.len() > 40 {
        return None;
    }
    id.bytes()
        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        .then_some(id)
}

fn coerce_id(id: &Value) -> String {
    match id {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string().trim_matches('"').to_string(),
    }
}

#[tauri::command]
pub fn sc_check_covers(app: AppHandle, ids: Vec<Value>) -> Result<Value, String> {
    let dir = sc_covers_dir(&app);
    let existing: std::collections::HashSet<String> = fs::read_dir(&dir)
        .ok()
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    let mut result = serde_json::Map::new();
    for raw in ids {
        let id = coerce_id(&raw);
        let Some(id) = safe_cover_id(&id).map(str::to_string) else { continue };
        let name = format!("{id}.jpg");
        if existing.contains(&name) {
            result.insert(id, Value::String(file_url(&dir.join(&name))));
        }
    }
    Ok(Value::Object(result))
}

#[tauri::command]
pub async fn sc_cache_cover(app: AppHandle, id: Value, url: String) -> Result<Option<String>, String> {
    let id = coerce_id(&id);
    let Some(id) = safe_cover_id(&id).map(str::to_string) else {
        return Ok(None);
    };
    let dir = sc_covers_dir(&app);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("{id}.jpg"));
    if file.exists() {
        return Ok(Some(file_url(&file)));
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0")
        .header("Referer", "https://soundcloud.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 2_000_000 {
        return Ok(None);
    }
    fs::write(&file, bytes).map_err(|e| e.to_string())?;
    Ok(Some(file_url(&file)))
}

#[tauri::command]
pub fn sc_clear_covers_cache(app: AppHandle) -> Result<u64, String> {
    let dir = sc_covers_dir(&app);
    let mut count = 0u64;
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if fs::remove_file(entry.path()).is_ok() {
                count += 1;
            }
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn sc_clear_session(app: AppHandle) -> Result<bool, String> {
    write::close_bridge(&app);
    fetch::clear_cookie_jar(&app);
    Ok(true)
}

#[tauri::command]
pub fn sc_clear_likes_cache(app: AppHandle) -> Result<bool, String> {
    match fs::remove_file(sc_likes_file(&app)) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn sc_load_likes_cache(app: AppHandle) -> Result<Value, String> {
    let path = sc_likes_file(&app);
    let text = fs::read_to_string(path).unwrap_or_else(|_| "[]".into());
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sc_save_likes_cache(app: AppHandle, data: Value) -> Result<(), String> {
    let dir = settings::app_data_dir(&app);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string(&data).map_err(|e| e.to_string())?;
    fs::write(sc_likes_file(&app), text).map_err(|e| e.to_string())
}
