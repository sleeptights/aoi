use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

fn save_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("aoi.player")
    })
}

pub fn settings_path(app: &AppHandle) -> PathBuf {
    app_data_dir(app).join("settings.json")
}

fn is_settings_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

fn read_json(path: &PathBuf) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn roaming_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

fn import_legacy_data(app: &AppHandle) {
    let dest = app_data_dir(app);
    let dest_settings = dest.join("settings.json");
    // One-shot: never overwrite an existing aoi.player profile.
    if dest_settings.exists() {
        return;
    }
    let current_empty = read_json(&dest_settings)
        .map(|v| is_settings_empty(&v))
        .unwrap_or(true);
    if !current_empty {
        return;
    }
    let Some(roaming) = roaming_dir() else { return };
    for name in ["aoi", "sewer"] {
        let src = roaming.join(name);
        let src_settings = src.join("settings.json");
        if !src_settings.exists() {
            continue;
        }
        let _ = fs::create_dir_all(&dest);
        let _ = fs::copy(&src_settings, &dest_settings);
        let likes = src.join("sc_likes.json");
        if likes.exists() {
            let _ = fs::copy(&likes, dest.join("sc_likes.json"));
        }
        let cookies = src.join("sc_cookies.json");
        if cookies.exists() {
            let _ = fs::copy(&cookies, dest.join("sc_cookies.json"));
        }
        let covers = src.join("sc_covers");
        let dest_covers = dest.join("sc_covers");
        if covers.is_dir() {
            let _ = fs::create_dir_all(&dest_covers);
            if let Ok(entries) = fs::read_dir(&covers) {
                for entry in entries.flatten() {
                    let to = dest_covers.join(entry.file_name());
                    if !to.exists() {
                        let _ = fs::copy(entry.path(), to);
                    }
                }
            }
        }
        break;
    }
}

pub fn read_settings_file(app: &AppHandle) -> Value {
    import_legacy_data(app);
    read_json(&settings_path(app)).unwrap_or(json!({}))
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> Value {
    read_settings_file(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, data: Value) -> Result<(), String> {
    let _guard = save_lock().lock().unwrap_or_else(|e| e.into_inner());
    let dir = app_data_dir(&app);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = settings_path(&app);
    let tmp = dir.join("settings.json.tmp");
    let text = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn backup_settings(app: AppHandle) -> Result<String, String> {
    let src = settings_path(&app);
    if !src.exists() {
        return Err("settings.json missing".into());
    }
    let stamp = chrono_lite_stamp();
    let dest = app_data_dir(&app).join(format!("settings-backup-{stamp}.json"));
    fs::copy(&src, &dest).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn restore_settings_backup(app: AppHandle, path: String) -> Result<Value, String> {
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err("backup file not found".into());
    }
    // only restore from our app data dir
    let data_dir = app_data_dir(&app);
    let canon_p = p.canonicalize().map_err(|e| e.to_string())?;
    let canon_d = data_dir.canonicalize().map_err(|e| e.to_string())?;
    if !canon_p.starts_with(&canon_d) {
        return Err("backup path not allowed".into());
    }
    let data = read_json(&p).ok_or_else(|| "invalid backup json".to_string())?;
    save_settings(app, data.clone())?;
    Ok(data)
}

fn chrono_lite_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

#[tauri::command]
pub fn set_login_item(app: AppHandle, enable: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    if enable {
        autostart.enable().map_err(|e| e.to_string())
    } else {
        autostart.disable().map_err(|e| e.to_string())
    }
}
