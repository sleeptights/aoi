use serde_json::{json, Value};
use std::{sync::OnceLock, time::Duration};
use tauri::{webview::Cookie, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tokio::sync::Mutex;
use url::Url;

use super::fetch;

const BRIDGE: &str = "sc-bridge";

fn write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn close_bridge(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(BRIDGE) {
        let _ = win.close();
    }
}

pub async fn sc_write(
    app: &AppHandle,
    url: &str,
    token: &str,
    client_id: &str,
    method: &str,
) -> Result<Value, String> {
    let _guard = write_lock().lock().await;
    let mut full_url = if url.contains("client_id=") {
        url.to_string()
    } else {
        let sep = if url.contains('?') { '&' } else { '?' };
        format!("{url}{sep}client_id={client_id}")
    };
    if !full_url.contains("app_locale=") {
        full_url.push_str("&app_locale=en");
    }

    let win = ensure_bridge(app)?;
    let page = wait_until_ready(app, &win).await;
    fetch::log_write_line(app, "PAGE", &full_url, 0, &page.to_string());
    // Pull DataDome / session cookies from the live webview into our jar so
    // subsequent reqwest writes and follows have a chance without captcha.
    crate::soundcloud::browser::harvest_webview_cookies(app, &win).await;

    let payload = json!({
        "url": full_url,
        "method": method,
        "token": token,
    });
    let start_js = format!(
        r#"(function(){{
  var p = {payload};
  window.__aoiRes = undefined;
  var headers = {{
    Authorization: "OAuth " + p.token,
    Accept: "application/json, text/javascript, */*; q=0.1"
  }};
  var opts = {{ method: p.method, headers: headers, credentials: "include" }};
  if (p.method === "POST") {{
    headers["Content-Type"] = "application/json";
    opts.body = "{{}}";
  }}
  fetch(p.url, opts).then(function(r){{
    return r.text().then(function(text){{
      window.__aoiRes = {{ status: r.status, text: String(text || "").slice(0, 2000) }};
    }});
  }}).catch(function(e){{
    window.__aoiRes = {{ status: 0, text: String(e) }};
  }});
  return "started";
}})();"#
    );
    let started = eval_js(&win, &start_js).await;
    fetch::log_write_line(app, "START", &full_url, 0, &format!("{started:?}"));
    wait_fetch_result(app, &win, method, &full_url).await
}

fn ensure_bridge(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(existing) = app.get_webview_window(BRIDGE) {
        park_bridge(&existing);
        inject_session(&existing, app);
        return Ok(existing);
    }
    let win = open_bridge(app)?;
    inject_session(&win, app);
    let _ = win.eval("location.replace('https://soundcloud.com/')");
    Ok(win)
}

fn open_bridge(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(existing) = app.get_webview_window(BRIDGE) {
        park_bridge(&existing);
        return Ok(existing);
    }
    let parsed = Url::parse("https://soundcloud.com/").map_err(|e| e.to_string())?;
    let win = WebviewWindowBuilder::new(app, BRIDGE, WebviewUrl::External(parsed))
        .visible(true)
        .skip_taskbar(true)
        .focused(false)
        .decorations(false)
        .inner_size(1100.0, 760.0)
        .position(-2400.0, -2400.0)
        .title("aoi")
        .build()
        .map_err(|e| e.to_string())?;
    park_bridge(&win);
    Ok(win)
}

fn park_bridge(win: &WebviewWindow) {
    let _ = win.set_ignore_cursor_events(true);
    let _ = win.set_position(tauri::LogicalPosition::new(-2400.0, -2400.0));
    let _ = win.show();
}

fn reveal_for_captcha(win: &WebviewWindow) {
    let _ = win.set_ignore_cursor_events(false);
    let _ = win.set_size(tauri::LogicalSize::new(1100.0, 760.0));
    let _ = win.center();
    let _ = win.show();
    let _ = win.set_focus();
}

fn hide_bridge(win: &WebviewWindow) {
    let _ = win.set_ignore_cursor_events(true);
    let _ = win.set_position(tauri::LogicalPosition::new(-2400.0, -2400.0));
    let _ = win.hide();
}

async fn eval_js(win: &WebviewWindow, js: &str) -> Result<Value, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = std::sync::Mutex::new(Some(tx));
    win.eval_with_callback(js, move |s| {
        if let Ok(mut guard) = tx.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(s);
            }
        }
    })
    .map_err(|e| e.to_string())?;
    let s = tokio::time::timeout(Duration::from_secs(8), rx)
        .await
        .map_err(|_| "eval timeout".to_string())?
        .map_err(|_| "eval canceled".to_string())?;
    if s.is_empty() || s == "null" || s == "undefined" {
        return Ok(Value::Null);
    }
    Ok(serde_json::from_str(&s).unwrap_or_else(|_| json!({ "raw": s })))
}

async fn wait_fetch_result(
    app: &AppHandle,
    win: &WebviewWindow,
    method: &str,
    url: &str,
) -> Result<Value, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if tokio::time::Instant::now() > deadline {
            fetch::log_write_line(app, method, url, 0, "fetch timeout");
            hide_bridge(win);
            return Ok(json!({ "error": "timeout" }));
        }
        tokio::time::sleep(Duration::from_millis(120)).await;
        let v = eval_js(
            win,
            r#"window.__aoiRes === undefined ? null : (window.__aoiRes || {status:0,text:"empty"})"#,
        )
        .await
        .unwrap_or(Value::Null);
        if v.is_null() {
            continue;
        }
        let status = v.get("status").and_then(|x| x.as_u64()).unwrap_or(0) as u16;
        let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("");
        fetch::log_write_line(app, method, url, status, text);
        hide_bridge(win);
        let ok = (200..300).contains(&status)
            || status == 409
            || (status == 404 && method.eq_ignore_ascii_case("DELETE"));
        if ok {
            if text.trim().is_empty() {
                return Ok(json!({ "data": null }));
            }
            return match serde_json::from_str::<Value>(text) {
                Ok(data) => Ok(json!({ "data": data })),
                Err(_) => Ok(json!({ "data": null })),
            };
        }
        // status 0 must not leak to the UI as a number: JS treats `error: 0` as falsy,
        // which the frontend would mistake for success.
        if status == 0 {
            return Ok(json!({ "error": if text.is_empty() { "network" } else { text } }));
        }
        return Ok(json!({ "error": status }));
    }
}

async fn wait_until_ready(app: &AppHandle, win: &WebviewWindow) -> Value {
    let start = tokio::time::Instant::now();
    let mut last = json!({ "error": "no-state" });
    let mut saw_captcha = false;
    let mut i = 0u32;
    loop {
        let limit = if saw_captcha { 90 } else { 18 };
        if start.elapsed() > Duration::from_secs(limit) {
            return last;
        }
        let href = win.url().map(|u| u.to_string()).unwrap_or_default();
        let captcha_url = href.contains("captcha-delivery") || href.contains("octocaptcha");
        let ping = eval_js(
            win,
            r#"(function(){
  try {
    var html = document.documentElement ? document.documentElement.innerHTML.slice(0, 8000) : "";
    return {
      href: location.href,
      ready: document.readyState,
      captcha: html.indexOf("captcha-delivery") !== -1 || html.indexOf("octocaptcha") !== -1
    };
  } catch (e) {
    return { href: "", ready: "error", captcha: false, err: String(e) };
  }
})()"#,
        )
        .await
        .unwrap_or_else(|e| json!({ "error": e }));
        last = ping;
        if i == 0 || i % 6 == 0 {
            fetch::log_write_line(app, "STATE", "page", i as u16, &last.to_string());
        }
        let captcha = captcha_url
            || last
                .get("captcha")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let ready = last.get("ready").and_then(|v| v.as_str()).unwrap_or("");
        let href2 = last
            .get("href")
            .and_then(|v| v.as_str())
            .unwrap_or(href.as_str());
        if captcha {
            saw_captcha = true;
            reveal_for_captcha(win);
            tokio::time::sleep(Duration::from_millis(500)).await;
            i += 1;
            continue;
        }
        if ready == "complete" && href2.contains("soundcloud.com") {
            hide_bridge(win);
            return last;
        }
        if i == 6 {
            park_bridge(win);
        }
        tokio::time::sleep(Duration::from_millis(350)).await;
        i += 1;
    }
}

fn inject_session(win: &WebviewWindow, app: &AppHandle) {
    let Some(session) = fetch::session_cookie_value(app) else {
        return;
    };
    let session = fetch::sanitize_cookie_value(&session);
    for domain in [
        ".soundcloud.com",
        "soundcloud.com",
        "api-v2.soundcloud.com",
        "api.soundcloud.com",
        "api-auth.soundcloud.com",
    ] {
        let cookie = Cookie::build(("_soundcloud_session", session.as_str()))
            .domain(domain)
            .path("/")
            .secure(true)
            .http_only(true)
            .build();
        let _ = win.set_cookie(cookie);
    }
}
