use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::Url;

use super::{fetch, write, ScCreds};

const LOGIN_URL: &str = "https://soundcloud.com/signin";
const LOGIN_WIN: &str = "sc-login";

const LOGIN_HOOK: &str = r#"
(function(){
  if (window.__aoiHooked) return;
  window.__aoiHooked = true;
  window.__aoiAuth = window.__aoiAuth || { token: null, clientId: null };
  function fromAuth(v){
    if (!v) return;
    v = String(v);
    if (v.indexOf("OAuth ") === 0) v = v.slice(6);
    v = v.trim();
    if (/^\d-\d+-\d+-[A-Za-z0-9]+$/.test(v)) window.__aoiAuth.token = v;
  }
  function fromUrl(u){
    try {
      var m = String(u).match(/client_id=([A-Za-z0-9]{32})/);
      if (m) window.__aoiAuth.clientId = m[1];
    } catch (e) {}
  }
  function fromHeaders(h){
    if (!h) return;
    if (typeof h.get === "function") fromAuth(h.get("Authorization") || h.get("authorization"));
    else fromAuth(h.Authorization || h.authorization);
  }
  var ofetch = window.fetch;
  window.fetch = function(input, init){
    try {
      var url = typeof input === "string" ? input : (input && input.url);
      fromUrl(url);
      fromHeaders(init && init.headers);
      if (input && typeof input !== "string") fromHeaders(input.headers);
    } catch (e) {}
    return ofetch.apply(this, arguments);
  };
  var xo = XMLHttpRequest.prototype.setRequestHeader;
  XMLHttpRequest.prototype.setRequestHeader = function(k, v){
    try { if (String(k).toLowerCase() === "authorization") fromAuth(v); } catch (e) {}
    return xo.apply(this, arguments);
  };
  var xoOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function(m, u){
    fromUrl(u);
    return xoOpen.apply(this, arguments);
  };
})();
"#;

/// In-app WebView login. Token is taken from page requests (Authorization: OAuth …),
/// not only from cookies — WebView2 cookie reads can hang on Windows.
pub async fn wait_for_login(app: &AppHandle) -> Result<Option<ScCreds>, String> {
    write::close_bridge(app);
    close_login(app);

    let win = open_login_window(app)?;
    let deadline = Instant::now() + Duration::from_secs(300);
    let mut client_id: Option<String> = None;

    while Instant::now() < deadline {
        if app.get_webview_window(LOGIN_WIN).is_none() {
            return Ok(None);
        }

        if client_id.is_none() {
            client_id = fetch_sc_client_id().await;
        }
        let _ = win.eval(LOGIN_HOOK);

        if let Some(creds) = try_creds_from_window(app, &win, client_id.as_deref()).await {
            let _ = win.close();
            close_login(app);
            return Ok(Some(creds));
        }

        if let (Some(cid), Some(hit)) = (
            client_id.as_ref(),
            scan_gecko_sc_session(find_default_browser().as_deref()),
        ) {
            let _ = fetch::import_session_cookie(app, &hit.cookie_value).await;
            refresh_soundcloud_cookies(app);
            if fetch::token_works(app, &hit.token, cid).await {
                let _ = win.close();
                close_login(app);
                return Ok(Some(ScCreds {
                    token: hit.token.clone(),
                    client_id: cid.clone(),
                }));
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let _ = win.close();
    close_login(app);
    Ok(None)
}

fn close_login(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LOGIN_WIN) {
        let _ = win.close();
    }
}

fn open_login_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(existing) = app.get_webview_window(LOGIN_WIN) {
        let _ = existing.set_focus();
        return Ok(existing);
    }
    let parsed = Url::parse(LOGIN_URL).map_err(|e| e.to_string())?;
    let win = WebviewWindowBuilder::new(app, LOGIN_WIN, WebviewUrl::External(parsed))
        .title("SoundCloud — aoi")
        .inner_size(980.0, 720.0)
        .center()
        .visible(true)
        .focused(true)
        .initialization_script_for_all_frames(LOGIN_HOOK)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = win.eval(LOGIN_HOOK);
    Ok(win)
}

async fn eval_js(win: &WebviewWindow, js: &str) -> Option<serde_json::Value> {
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = std::sync::Mutex::new(Some(tx));
    win.eval_with_callback(js, move |s| {
        if let Ok(mut guard) = tx.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(s);
            }
        }
    })
    .ok()?;
    let s = tokio::time::timeout(Duration::from_secs(2), rx)
        .await
        .ok()?
        .ok()?;
    if s.is_empty() || s == "null" || s == "undefined" {
        return None;
    }
    let parsed = serde_json::from_str::<serde_json::Value>(&s)
        .unwrap_or_else(|_| serde_json::json!({ "raw": s }));
    if let Some(inner) = parsed.as_str() {
        return serde_json::from_str(inner).ok().or(Some(parsed));
    }
    Some(parsed)
}

fn looks_like_token(s: &str) -> bool {
    Regex::new(r"^\d-\d+-\d+-[A-Za-z0-9]+$")
        .ok()
        .is_some_and(|re| re.is_match(s))
}

async fn try_creds_from_window(
    app: &AppHandle,
    win: &WebviewWindow,
    client_id: Option<&str>,
) -> Option<ScCreds> {
    let href = win.url().ok().map(|u| u.to_string()).unwrap_or_default();
    if !href.contains("soundcloud.com") && !href.contains("captcha") {
        return None;
    }

    let page = eval_js(
        win,
        r#"(function(){
  var a = window.__aoiAuth || {};
  var token = a.token || null;
  try {
    var c = document.cookie || "";
    var om = c.match(/(?:^|; )oauth_token=([^;]+)/);
    if (om && !token) token = decodeURIComponent(om[1]);
    var keys = Object.keys(localStorage || {});
    for (var i = 0; i < keys.length && !token; i++) {
      var val = String(localStorage.getItem(keys[i]) || "");
      var m = val.match(/\d-\d+-\d+-[A-Za-z0-9]+/);
      if (m) token = m[0];
    }
  } catch (e) {}
  return { href: location.href, token: token, clientId: a.clientId || null };
})()"#,
    )
    .await;

    let mut token = page
        .as_ref()
        .and_then(|v| v.get("token"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| looks_like_token(s));

    if token.is_none() {
        harvest_webview_cookies(app, win).await;
        if let Some(raw) = fetch::cookie_named(app, "oauth_token") {
            if looks_like_token(&raw) {
                token = Some(raw);
            }
        }
        if token.is_none() {
            if let Some(session) = fetch::session_cookie_value(app) {
                token = token_from_session_value(&session);
            }
        }
    } else {
        harvest_webview_cookies(app, win).await;
    }

    let token = token?;
    let cid = page
        .as_ref()
        .and_then(|v| v.get("clientId"))
        .and_then(|v| v.as_str())
        .filter(|s| s.len() == 32)
        .map(|s| s.to_string())
        .or_else(|| client_id.map(|s| s.to_string()))
        .or(fetch_sc_client_id().await)?;

    if fetch::token_works(app, &token, &cid).await {
        Some(ScCreds {
            token,
            client_id: cid,
        })
    } else {
        None
    }
}

pub async fn harvest_webview_cookies(app: &AppHandle, win: &WebviewWindow) {
    let win2 = win.clone();
    let cookies = tokio::time::timeout(Duration::from_millis(600), async move {
        tokio::task::spawn_blocking(move || win2.cookies())
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    for cookie in cookies {
        let name = cookie.name().to_string();
        let value = fetch::sanitize_cookie_value(cookie.value());
        if name.is_empty() || value.is_empty() {
            continue;
        }
        if matches!(
            name.as_str(),
            "_soundcloud_session" | "datadome" | "sc_anonymous_id" | "oauth_token"
        ) {
            let domain = cookie.domain().unwrap_or(".soundcloud.com");
            let _ = fetch::upsert_cookie(app, &name, &value, domain);
        }
    }
}

struct SessionHit {
    token: String,
    cookie_value: String,
}

#[cfg(windows)]
fn find_default_browser() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let prog_id: String = hkcu
        .open_subkey(r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice")
        .ok()
        .and_then(|k| k.get_value("ProgId").ok())?;

    let cmd: String = hkcu
        .open_subkey(format!(r"Software\Classes\{prog_id}\shell\open\command"))
        .ok()
        .and_then(|k| k.get_value("").ok())
        .or_else(|| {
            RegKey::predef(HKEY_CLASSES_ROOT)
                .open_subkey(format!(r"{prog_id}\shell\open\command"))
                .ok()
                .and_then(|k| k.get_value("").ok())
        })?;

    exe_from_command(&cmd)
}

#[cfg(not(windows))]
fn find_default_browser() -> Option<String> {
    None
}

fn exe_from_command(cmd: &str) -> Option<String> {
    let re = Regex::new(r#""([^"]+\.exe)""#).ok()?;
    if let Some(caps) = re.captures(cmd) {
        let p = caps.get(1)?.as_str();
        if Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    None
}

fn gecko_profiles_root(exe: Option<&str>) -> PathBuf {
    let appdata = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    match exe {
        Some(e) if e.to_lowercase().contains("librewolf") => appdata.join("librewolf"),
        Some(e) if e.to_lowercase().contains("waterfox") => appdata.join("Waterfox"),
        Some(e) if e.to_lowercase().contains("floorp") => appdata.join("Floorp"),
        Some(e) if e.to_lowercase().contains("zen.exe") => appdata.join("zen"),
        _ => appdata.join("Mozilla").join("Firefox"),
    }
}

fn gecko_default_profile(exe: Option<&str>) -> Option<PathBuf> {
    let root = gecko_profiles_root(exe);
    let ini = fs::read_to_string(root.join("profiles.ini")).ok()?;

    if let Some(cap) = Regex::new(r"(?m)\[Install[\s\S]*?^Default=([^\r\n]+)")
        .ok()?
        .captures(&ini)
    {
        let rel = cap.get(1)?.as_str().trim();
        let p = root.join(rel.replace('/', "\\"));
        if p.exists() {
            return Some(p);
        }
    }

    let block_re = Regex::new(r"(?m)\[Profile\d+\][\s\S]*?(?=\[Profile|\z)").ok()?;
    let mut fallback = None;
    for block in block_re.find_iter(&ini) {
        let text = block.as_str();
        let path = Regex::new(r"Path=([^\r\n]+)")
            .ok()?
            .captures(text)?
            .get(1)?
            .as_str()
            .trim();
        let is_rel = Regex::new(r"IsRelative=(\d)")
            .ok()
            .and_then(|re| re.captures(text))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str() == "1")
            .unwrap_or(true);
        let full = if is_rel {
            root.join(path.replace('/', "\\"))
        } else {
            PathBuf::from(path)
        };
        if text.contains("Default=1") {
            return Some(full);
        }
        if fallback.is_none() {
            fallback = Some(full);
        }
    }
    fallback
}

fn read_file_copy(path: &Path) -> Option<Vec<u8>> {
    fs::read(path).ok().or_else(|| {
        let tmp = std::env::temp_dir().join(format!(
            "aoi-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            path.file_name()?.to_string_lossy()
        ));
        fs::copy(path, &tmp).ok()?;
        let data = fs::read(&tmp).ok()?;
        let _ = fs::remove_file(tmp);
        Some(data)
    })
}

pub fn token_from_session_value(raw: &str) -> Option<String> {
    let mut v = raw.trim().to_string();
    if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) {
        v = v[1..v.len() - 1].to_string();
    }
    let b64 = v.split("--").next()?.to_string();
    if let Ok(decoded) = STANDARD.decode(b64.as_bytes()) {
        if let Ok(text) = String::from_utf8(decoded) {
            if let Some(m) = Regex::new(r"\d-\d+-\d+-[A-Za-z0-9]+")
                .ok()?
                .find(&text)
            {
                return Some(m.as_str().to_string());
            }
        }
    }
    Regex::new(r"\d-\d+-\d+-[A-Za-z0-9]+")
        .ok()?
        .find(&v)
        .map(|m| m.as_str().to_string())
}

fn scan_gecko_sc_session(exe: Option<&str>) -> Option<SessionHit> {
    let profile = gecko_default_profile(exe)?;
    let needle = b"_soundcloud_session";
    let cookie_re = Regex::new(r#""([A-Za-z0-9+/=]+(?:--[A-Za-z0-9+/=]+)?)""#).ok()?;

    for name in [
        "cookies.sqlite-wal",
        "cookies.sqlite",
        "cookies.sqlite-journal",
    ] {
        let Some(buf) = read_file_copy(&profile.join(name)) else {
            continue;
        };
        let mut idx = 0usize;
        while let Some(found) = buf[idx..].windows(needle.len()).position(|w| w == needle) {
            let pos = idx + found + needle.len();
            let slice = String::from_utf8_lossy(&buf[pos..buf.len().min(pos + 400)]);
            if let Some(caps) = cookie_re.captures(&slice) {
                let cookie_value = caps
                    .get(1)
                    .or_else(|| caps.get(0))
                    .map(|m| m.as_str().trim_matches(|c| c == '"' || c == '\''))
                    .unwrap_or("")
                    .to_string();
                if cookie_value.is_empty() {
                    idx = pos;
                    continue;
                }
                if let Some(token) = token_from_session_value(&cookie_value) {
                    return Some(SessionHit {
                        token,
                        cookie_value,
                    });
                }
            }
            idx = pos;
        }
    }
    None
}

pub fn refresh_soundcloud_cookies(app: &AppHandle) {
    let browser = find_default_browser();
    let profile = gecko_default_profile(browser.as_deref()).or_else(|| gecko_default_profile(None));
    let Some(profile) = profile else { return };

    for file in ["cookies.sqlite-wal", "cookies.sqlite", "cookies.sqlite-journal"] {
        let Some(buf) = read_file_copy(&profile.join(file)) else { continue };
        if let Some(val) = extract_cookie_value(&buf, "datadome") {
            let _ = fetch::upsert_cookie(app, "datadome", &val, ".soundcloud.com");
        }
        if let Some(val) = extract_cookie_value(&buf, "sc_anonymous_id") {
            let _ = fetch::upsert_cookie(app, "sc_anonymous_id", &val, ".soundcloud.com");
        }
    }
}

fn extract_cookie_value(buf: &[u8], name: &str) -> Option<String> {
    let needle = name.as_bytes();
    let quoted = Regex::new(r#"^[\x00-\x20"]*([A-Za-z0-9+/=_~-]{8,})"#).ok()?;
    let mut idx = 0usize;
    let mut best: Option<String> = None;
    while let Some(found) = buf[idx..].windows(needle.len()).position(|w| w == needle) {
        let pos = idx + found + needle.len();
        let slice = String::from_utf8_lossy(&buf[pos..buf.len().min(pos + 500)]);
        if let Some(caps) = quoted.captures(slice.as_ref()) {
            if let Some(m) = caps.get(1) {
                let val = fetch::sanitize_cookie_value(m.as_str());
                if best.as_ref().map(|b| val.len() > b.len()).unwrap_or(true) {
                    best = Some(val);
                }
            }
        }
        idx = pos;
    }
    best
}

pub async fn fetch_sc_client_id() -> Option<String> {
    let client = reqwest::Client::new();
    let html = client
        .get("https://soundcloud.com")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0",
        )
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    if let Some(cap) = Regex::new(r#"client_id["'=:\s]+["']([A-Za-z0-9]{32})["']"#)
        .ok()?
        .captures(&html)
    {
        return Some(cap.get(1)?.as_str().to_string());
    }

    let asset_re = Regex::new(r#"src="(https://[^"]+/assets/[^"]+\.js)""#).ok()?;
    for cap in asset_re.captures_iter(&html).take(24) {
        let url = cap.get(1)?.as_str();
        if let Ok(resp) = client.get(url).send().await {
            if let Ok(js) = resp.text().await {
                for pat in [
                    r#"client_id\s*[:=]\s*"([A-Za-z0-9]{32})""#,
                    r"client_id=([A-Za-z0-9]{32})",
                ] {
                    if let Some(m) = Regex::new(pat).ok().and_then(|re| re.captures(&js)) {
                        return Some(m.get(1)?.as_str().to_string());
                    }
                }
            }
        }
    }
    None
}
