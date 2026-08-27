use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use tauri::AppHandle;

const UPDATE_URL: &str = "https://aoi-rooms.elvishedcc.workers.dev/update/latest";
const MAX_BYTES: u64 = 80 * 1024 * 1024;

fn allowed_download_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == "github.com"
        || h.ends_with(".github.com")
        || h == "objects.githubusercontent.com"
        || h == "github-releases.githubusercontent.com"
        || h == "release-assets.githubusercontent.com"
        || h.ends_with(".githubusercontent.com")
}

fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let clean = v.trim().trim_start_matches('v');
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

fn is_newer(remote: &str, local: &str) -> bool {
    match (parse_semver(remote), parse_semver(local)) {
        (Some(r), Some(l)) => r > l,
        _ => false,
    }
}

fn normalize_sha(s: &str) -> String {
    s.trim().to_ascii_lowercase().chars().filter(|c| c.is_ascii_hexdigit()).collect()
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn check_for_update() -> Result<Value, String> {
    let local = app_version();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(format!("aoi/{}", local))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(UPDATE_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("update check http {}", resp.status()));
    }
    let mut data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let remote = data
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let url = data.get("url").and_then(|v| v.as_str()).unwrap_or("");
    if remote.is_empty() || url.is_empty() {
        return Err("invalid update manifest".into());
    }
    let parsed = url::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" {
        return Err("update url must be https".into());
    }
    let host = parsed.host_str().unwrap_or("");
    if !allowed_download_host(host) {
        return Err("update host not allowed".into());
    }
    let available = is_newer(&remote, &local) && !url.is_empty();
    if let Value::Object(ref mut map) = data {
        map.insert("local".into(), json!(local));
        map.insert("available".into(), json!(available));
    }
    Ok(data)
}

#[tauri::command]
pub async fn install_update(app: AppHandle, url: String, sha256: Option<String>) -> Result<(), String> {
    let local = app_version();
    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" {
        return Err("update url must be https".into());
    }
    let host = parsed.host_str().unwrap_or("");
    if !allowed_download_host(host) {
        return Err("update host not allowed".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .user_agent(format!("aoi/{}", local))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())?;

    let mut resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("download http {}", resp.status()));
    }
    if let Some(len) = resp.content_length() {
        if len > MAX_BYTES {
            return Err("update file too large".into());
        }
    }

    let tmp_dir = std::env::temp_dir().join("aoi-update");
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let dest: PathBuf = tmp_dir.join("aoi-setup-win-x64.exe");
    if dest.exists() {
        let _ = fs::remove_file(&dest);
    }

    let mut file = File::create(&dest).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        total += chunk.len() as u64;
        if total > MAX_BYTES {
            let _ = fs::remove_file(&dest);
            return Err("update file too large".into());
        }
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    if let Some(expected) = sha256 {
        let want = normalize_sha(&expected);
        if want.len() == 64 {
            let got = format!("{:x}", hasher.finalize());
            if got != want {
                let _ = fs::remove_file(&dest);
                return Err("sha256 mismatch — update aborted".into());
            }
        }
    }

    // sanity: PE header
    {
        let mut f = File::open(&dest).map_err(|e| e.to_string())?;
        let mut magic = [0u8; 2];
        f.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic != b"MZ" {
            let _ = fs::remove_file(&dest);
            return Err("downloaded file is not an executable".into());
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED: u32 = 0x00000008 | 0x00000200 | 0x08000000;
        Command::new(&dest)
            .creation_flags(DETACHED)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        Command::new(&dest).spawn().map_err(|e| e.to_string())?;
    }

    // give installer a moment to start, then quit
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        app.exit(0);
    });
    Ok(())
}
