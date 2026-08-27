use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::window::Color;

use crate::settings::{self, app_data_dir, settings_path};

/* ── вспомогательный GET для публичных JSON (waveform SoundCloud) ─────────── */

#[tauri::command]
pub async fn fetch_json(url: String) -> Result<Value, String> {
    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
    let host = parsed.host_str().unwrap_or("");
    let scheme = parsed.scheme();
    let allowed = (scheme == "https" || scheme == "http")
        && (host == "sndcdn.com"
            || host.ends_with(".sndcdn.com")
            || host == "lrclib.net"
            || host.ends_with(".lrclib.net"));
    if !allowed {
        return Err("host not allowed".into());
    }
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) aoi/0.1",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 6_000_000 {
        return Err("response too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/* ── LRCLIB: поиск синхронизированного текста ────────────────────────────── */

#[tauri::command]
pub async fn lrclib_lookup(
    title: String,
    artist: Option<String>,
    duration: Option<u64>,
) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let mut req = client
        .get("https://lrclib.net/api/search")
        .query(&[("track_name", title.as_str())])
        .header("User-Agent", "aoi/0.1 (music player; lrclib integration)");
    if let Some(a) = artist.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        req = req.query(&[("artist_name", a)]);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&resp).unwrap_or(Value::Null);
    let arr = match v.as_array() {
        Some(a) if !a.is_empty() => a.clone(),
        _ => return Ok(Value::Null),
    };

    let dur = duration.unwrap_or(0) as f64;
    let mut best: Option<(Value, i64)> = None;
    for it in &arr {
        let has_synced = it
            .get("syncedLyrics")
            .and_then(|x| x.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let d = it.get("duration").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let mut score = if has_synced { 100 } else { 0 };
        if dur > 0.0 && d > 0.0 {
            let diff = (d - dur).abs();
            if diff <= 3.0 {
                score += 50 - (diff as i64) * 5;
            } else {
                score -= (diff as i64 - 3).min(60);
            }
        }
        if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
            best = Some((it.clone(), score));
        }
    }
    Ok(best.map(|(it, _)| it).unwrap_or(Value::Null))
}

/* ── .lrc рядом с локальным файлом ───────────────────────────────────────── */

#[tauri::command]
pub fn read_lrc(path: String) -> Option<String> {
    let p = PathBuf::from(&path);
    let stem = p.file_stem()?.to_str()?.to_string();
    let dir = p.parent()?;
    let lower = stem.to_lowercase();
    let upper = stem.to_uppercase();
    for name in [
        format!("{stem}.lrc"),
        format!("{stem}.LRC"),
        format!("{lower}.lrc"),
        format!("{upper}.LRC"),
    ] {
        let f = dir.join(name);
        if f.is_file() {
            if let Ok(bytes) = fs::read(&f) {
                return Some(String::from_utf8_lossy(&bytes).to_string());
            }
        }
    }
    None
}

/* ── статистика прослушиваний: plays.jsonl ───────────────────────────────── */

fn plays_path(app: &AppHandle) -> PathBuf {
    app_data_dir(app).join("plays.jsonl")
}

#[tauri::command]
pub fn log_play(app: AppHandle, entry: Value) -> Result<(), String> {
    let dir = app_data_dir(&app);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(plays_path(&app))
        .map_err(|e| e.to_string())?;
    let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    writeln!(f, "{line}").map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_plays(app: AppHandle) -> Value {
    let path = plays_path(&app);
    let Ok(f) = fs::File::open(&path) else {
        return Value::Array(Vec::new());
    };
    let mut out = Vec::new();
    for line in std::io::BufReader::new(f).lines().map_while(Result::ok) {
        if let Ok(v) = serde_json::from_str::<Value>(&line) {
            if v.is_object() {
                out.push(v);
            }
        }
    }
    Value::Array(out)
}

#[tauri::command]
pub fn clear_plays(app: AppHandle) -> Result<(), String> {
    let _ = fs::remove_file(plays_path(&app));
    Ok(())
}

/* ── мини-плеер: отдельное always-on-top окно на том же index.html#mini ───── */

fn show_mini_hide_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    if let Some(mini) = app.get_webview_window("mini") {
        let _ = mini.set_ignore_cursor_events(false);
        let _ = mini.show();
        let _ = mini.unminimize();
        let _ = mini.set_focus();
    }
}

fn show_main_hide_mini(app: &AppHandle) {
    if let Some(mini) = app.get_webview_window("mini") {
        let _ = mini.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

#[tauri::command]
pub async fn open_mini(app: AppHandle) -> Result<(), String> {
    // Always recreate: stale mini HWND (e.g. old transparent build) keeps dead clicks.
    if let Some(old) = app.get_webview_window("mini") {
        let _ = old.destroy();
        // Brief yield so the label is free before rebuild.
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    }

    // Opaque window: transparent HWND on Windows passes clicks through.
    let settings = settings::read_settings_file(&app);
    let on_top = settings
        .get("miniAlwaysOnTop")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let win = WebviewWindowBuilder::new(&app, "mini", WebviewUrl::App("index.html#mini".into()))
        .title("aoi · mini")
        .inner_size(456.0, 148.0)
        .min_inner_size(456.0, 148.0)
        .decorations(false)
        .transparent(false)
        .background_color(Color(7, 7, 10, 255))
        .always_on_top(on_top)
        .resizable(false)
        .shadow(true)
        .skip_taskbar(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    let _ = win.set_skip_taskbar(false);
    let _ = win.set_ignore_cursor_events(false);
    let _ = win.eval("document.addEventListener('contextmenu',e=>e.preventDefault())");

    let monitor = win.current_monitor().ok().flatten();
    if let (Some(m), Ok(size)) = (monitor, win.outer_size()) {
        let work = m.size();
        let x = work.width as i32 - size.width as i32 - 28;
        let y = work.height as i32 - size.height as i32 - 36;
        let _ = win.set_position(tauri::PhysicalPosition::new(x.max(12), y.max(12)));
    }
    show_mini_hide_main(&app);
    Ok(())
}

#[tauri::command]
pub fn mini_expand(app: AppHandle) {
    show_main_hide_mini(&app);
}

const MINI_W: f64 = 456.0;
const MINI_H: f64 = 148.0;
const MINI_H_MENU: f64 = 256.0;

#[tauri::command]
pub fn mini_always_on_top(app: AppHandle, on: bool) -> Result<(), String> {
    if let Some(mini) = app.get_webview_window("mini") {
        mini.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    let mut settings = settings::read_settings_file(&app);
    if let Value::Object(ref mut map) = settings {
        map.insert("miniAlwaysOnTop".into(), Value::Bool(on));
        let dir = app_data_dir(&app);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(settings_path(&app), text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn mini_menu(app: AppHandle, open: bool) -> Result<(), String> {
    let Some(win) = app.get_webview_window("mini") else { return Ok(()) };
    let want_h = if open { MINI_H_MENU } else { MINI_H };
    let old = win.outer_size().ok();
    let pos = win.outer_position().ok();
    let size = tauri::LogicalSize::new(MINI_W, want_h);
    let _ = win.set_max_size(Option::<tauri::LogicalSize<f64>>::None);
    let _ = win.set_min_size(Some(tauri::LogicalSize::new(MINI_W, 80.0)));
    let _ = win.set_size(size);
    let _ = win.set_min_size(Some(size));
    if let (Some(old), Some(pos), Ok(new)) = (old, pos, win.outer_size()) {
        let dy = new.height as i32 - old.height as i32;
        if dy != 0 {
            let y = (pos.y - dy).max(12);
            let _ = win.set_position(tauri::PhysicalPosition::new(pos.x, y));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn mini_cmd(app: AppHandle, action: String, value: Option<Value>) -> Result<(), String> {
    let payload = serde_json::json!({ "action": action, "value": value });
    // Emit once. App-wide + main both fire the same listener and toggle cancels itself.
    if let Some(main) = app.get_webview_window("main") {
        main.emit("player-cmd", payload).map_err(|e| e.to_string())?;
        return Ok(());
    }
    app.emit("player-cmd", payload).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn mini_state(app: AppHandle, state: Value) -> Result<(), String> {
    if let Some(mini) = app.get_webview_window("mini") {
        let _ = mini.emit("player-state", &state);
    }
    app.emit("player-state", state).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn battery_pct() -> Option<u8> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct SystemPowerStatus {
            ac_line_status: u8,
            battery_flag: u8,
            battery_life_percent: u8,
            system_status_flag: u8,
            battery_life_time: u32,
            battery_full_life_time: u32,
        }
        extern "system" {
            fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
        }
        let mut s = SystemPowerStatus {
            ac_line_status: 0,
            battery_flag: 0,
            battery_life_percent: 255,
            system_status_flag: 0,
            battery_life_time: 0,
            battery_full_life_time: 0,
        };
        if unsafe { GetSystemPowerStatus(&mut s) } == 0 {
            return None;
        }
        if s.battery_life_percent > 100 {
            return None;
        }
        Some(s.battery_life_percent)
    }
    #[cfg(not(windows))]
    {
        None
    }
}
