use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;

const AUTH_WIN: &str = "lfm-login";
const CALLBACK: &str = "http://aoi.player/callback";
const API_ROOT: &str = "https://ws.audioscrobbler.com/2.0/";

/* параметр callback должен быть URL; перехватываем навигацию до того,
   как webview попытается его загрузить, и вытаскиваем token из query */

fn md5_hex(s: &str) -> String {
    format!("{:x}", md5::compute(s.as_bytes()))
}

fn sign(params: &mut Vec<(String, String)>, secret: &str) {
    params.sort_by(|a, b| a.0.cmp(&b.0));
    let base: String = params.iter().map(|(k, v)| format!("{k}{v}")).collect();
    let sig = md5_hex(&format!("{base}{secret}"));
    params.push(("api_sig".to_string(), sig));
}

async fn lfm_request(
    api_key: &str,
    secret: &str,
    method: &str,
    extra: Vec<(String, String)>,
    sk: Option<&str>,
) -> Result<Value, String> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    pairs.push(("method".into(), method.into()));
    pairs.push(("api_key".into(), api_key.into()));
    if let Some(sk) = sk {
        pairs.push(("sk".into(), sk.into()));
    }
    for (k, v) in extra {
        pairs.push((k, v));
    }
    pairs.push(("format".into(), "json".into()));
    sign(&mut pairs, secret);

    let client = reqwest::Client::new();
    let resp = client
        .post(API_ROOT)
        .form(&pairs)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Открывает окно авторизации Last.fm, ждёт token из callback-редиректа,
/// сразу меняет его на сессию. Возвращает { name, key }.
#[tauri::command]
pub async fn lfm_auth(app: AppHandle, api_key: String, secret: String) -> Result<Value, String> {
    if api_key.trim().is_empty() || secret.trim().is_empty() {
        return Err("api key / secret required".into());
    }
    if let Some(w) = app.get_webview_window(AUTH_WIN) {
        let _ = w.set_focus();
        return Err("login window already open".into());
    }

    let auth_url = format!(
        "https://www.last.fm/api/auth/?api_key={}&callback={}",
        urlencoding_min(&api_key),
        urlencoding_min(CALLBACK)
    );

    let token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let tok2 = token.clone();
    let win = WebviewWindowBuilder::new(
        &app,
        AUTH_WIN,
        WebviewUrl::External(Url::parse(&auth_url).map_err(|e| e.to_string())?),
    )
    .title("Last.fm — aoi")
    .inner_size(920.0, 640.0)
    .center()
    .on_navigation(move |uri| {
        let s = uri.as_str();
        if s.contains("/callback") || uri.host_str() == Some("aoi.player") {
            for (k, v) in uri.query_pairs() {
                if k == "token" {
                    let mut g = tok2.lock().unwrap();
                    *g = Some(v.into_owned());
                }
            }
            return false;
        }
        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        if token.lock().unwrap().is_some() {
            break;
        }
        if app.get_webview_window(AUTH_WIN).is_none() {
            return Err("cancelled".into());
        }
        if Instant::now() > deadline {
            let _ = win.close();
            return Err("timeout".into());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    let _ = win.close();

    let token = token.lock().unwrap().clone().unwrap();
    let res = lfm_request(
        &api_key,
        &secret,
        "auth.getsession",
        vec![("token".into(), token)],
        None,
    )
    .await?;

    let session = res
        .get("session")
        .cloned()
        .ok_or_else(|| "no session in response".to_string())?;
    let name = session.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let key = session
        .get("key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if key.is_empty() {
        return Err("empty session key".into());
    }
    Ok(serde_json::json!({ "name": name, "key": key }))
}

/// Универсальный подписанный вызов Last.fm API от лица пользователя.
#[tauri::command]
pub async fn lfm_call(
    api_key: String,
    secret: String,
    sk: String,
    method: String,
    params: Value,
) -> Result<Value, String> {
    let mut extra: Vec<(String, String)> = Vec::new();
    if let Some(obj) = params.as_object() {
        for (k, v) in obj {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => continue,
            };
            if s.is_empty() {
                continue;
            }
            extra.push((k.clone(), s));
        }
    }
    lfm_request(&api_key, &secret, &method, extra, Some(&sk)).await
}

fn urlencoding_min(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F")
}
