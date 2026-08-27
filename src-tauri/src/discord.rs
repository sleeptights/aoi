use discord_rich_presence::{
    activity::{Activity, ActivityType, Assets, Party, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use serde_json::Value;
use std::{
    sync::{mpsc, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::state;

pub const DEFAULT_APP_ID: &str = "1539444732248203345";

enum Cmd {
    Connect(String),
    Disconnect,
    Clear,
    Update(Value),
    Shutdown,
}

static TX: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();

pub fn init() {
    let _ = sender();
}

pub fn shutdown() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Cmd::Shutdown);
    }
}

fn sender() -> mpsc::Sender<Cmd> {
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let _ = thread::Builder::new()
            .name("aoi-discord".into())
            .spawn(move || worker(rx));
        tx
    })
    .clone()
}

fn post(cmd: Cmd) {
    let _ = sender().send(cmd);
}

fn resolve_app_id(raw: &str) -> String {
    let id = raw.trim();
    if id.is_empty() {
        DEFAULT_APP_ID.to_string()
    } else {
        id.to_string()
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn clamp_rpc(s: &str) -> String {
    let mut t: String = s.chars().take(128).collect();
    if t.chars().count() < 2 {
        t.push(' ');
    }
    t
}

fn public_image_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.len() < 12 || !url.starts_with("https://") {
        return None;
    }
    let lower = url.to_ascii_lowercase();
    if lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("asset:") {
        return None;
    }
    if url.contains("wsrv.nl") || url.contains("weserv.nl") {
        return if url.len() <= 256 { Some(url.to_string()) } else { None };
    }
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let proxied = format!("https://wsrv.nl/?url={stripped}&w=512&h=512&output=jpg");
    if proxied.len() <= 256 {
        Some(proxied)
    } else if url.len() <= 256 {
        Some(url.to_string())
    } else {
        None
    }
}

fn connect_now(id: &str) -> Option<DiscordIpcClient> {
    let mut client = DiscordIpcClient::new(id).ok()?;
    client.connect().ok()?;
    Some(client)
}

fn close_client(client: &mut Option<DiscordIpcClient>) {
    if let Some(mut c) = client.take() {
        let _ = c.clear_activity();
        let _ = c.close();
    }
}

fn push(client: &mut DiscordIpcClient, act: Activity<'_>) -> bool {
    if client.set_activity(act).is_err() {
        return false;
    }
    let _ = client.recv();
    true
}

fn build_activity<'a>(
    details: &'a str,
    state_line: &'a str,
    timestamps: Option<Timestamps>,
    cover: Option<&'a str>,
    is_playing: bool,
    listening: bool,
    instance: &'a str,
) -> Activity<'a> {
    // Playing always draws an elapsed clock if we omit timestamps after a start/end pair.
    // Pause must be a new instance without timestamps so Discord does not keep counting.
    let mut act = Activity::new()
        .details(details)
        .state(state_line)
        .party(Party::new().id(instance))
        .activity_type(if listening || !is_playing {
            ActivityType::Listening
        } else {
            ActivityType::Playing
        });
    if is_playing {
        if let Some(ts) = timestamps {
            act = act.timestamps(ts);
        }
    }
    if let Some(url) = cover {
        act = act.assets(Assets::new().large_image(url).large_text("aoi"));
    }
    act
}

fn wipe_clock(client: &mut DiscordIpcClient) {
    let _ = client.clear_activity();
    let _ = client.recv();
    thread::sleep(Duration::from_millis(180));
}

struct PresenceMem {
    playing: bool,
    pause_wiped: bool,
}

fn room_state_line(data: &Value, is_playing: bool) -> Option<String> {
    let room = data.get("room")?;
    if room.is_null() {
        return None;
    }
    let host = room
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if host.is_empty() {
        return None;
    }
    let peers: Vec<&str> = room
        .get("peers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && *s != host)
                .collect()
        })
        .unwrap_or_default();
    let with = if peers.is_empty() {
        String::new()
    } else if peers.len() == 1 {
        format!(" · с {}", peers[0])
    } else {
        format!(" · с {}", peers.join(", "))
    };
    if is_playing {
        Some(clamp_rpc(&format!("{host}{with}")))
    } else {
        Some(clamp_rpc(&format!("paused · {host}{with}")))
    }
}

fn apply(client: &mut DiscordIpcClient, data: &Value, mem: &mut PresenceMem) -> bool {
    let title = state::opt_str(data, "title").unwrap_or_default();
    if title.trim().is_empty() {
        mem.playing = false;
        mem.pause_wiped = true;
        return client.clear_activity().is_ok();
    }
    let artist = state::opt_str(data, "artist").unwrap_or_default();
    let is_playing = state::opt_bool(data, "isPlaying");
    let in_room = data.get("room").map(|v| !v.is_null()).unwrap_or(false);
    let details = clamp_rpc(&title);
    let state_line = room_state_line(data, is_playing).unwrap_or_else(|| {
        if is_playing {
            clamp_rpc(&artist)
        } else {
            clamp_rpc("paused")
        }
    });
    let cover = state::opt_str(data, "coverUrl")
        .as_deref()
        .and_then(public_image_url);
    let duration = state::opt_f64(data, "duration");
    let progress = state::opt_f64(data, "progress").clamp(0.0, 1.0);
    let timestamp_mode = state::opt_str(data, "timestamp").unwrap_or_else(|| "progress".into());

    // Discord merges omitted timestamps into the last start/end.
    // After `end` it switches to elapsed and counts forever.
    // Only send `start` while playing. On pause, wipe once, then never send timestamps.
    if !is_playing && (mem.playing || !mem.pause_wiped) {
        wipe_clock(client);
        mem.pause_wiped = true;
    }
    mem.playing = is_playing;
    if is_playing {
        mem.pause_wiped = false;
    }

    let timestamps = if is_playing && duration > 0.0 && timestamp_mode != "none" {
        let elapsed_ms = (progress * duration * 1000.0) as i64;
        Some(Timestamps::new().start(now_ms() - elapsed_ms))
    } else {
        None
    };
    let instance = if is_playing { "aoi-play" } else { "aoi-pause" };

    let with_art = build_activity(
        &details,
        &state_line,
        timestamps.clone(),
        cover.as_deref(),
        is_playing,
        in_room,
        instance,
    );
    if cover.is_some() && push(client, with_art) {
        return true;
    }
    push(
        client,
        build_activity(
            &details,
            &state_line,
            timestamps,
            None,
            is_playing,
            in_room,
            instance,
        ),
    )
}

fn worker(rx: mpsc::Receiver<Cmd>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut app_id = DEFAULT_APP_ID.to_string();
    let mut enabled = false;
    let mut last: Option<Value> = None;
    let mut next_retry = Instant::now();
    let mut mem = PresenceMem {
        playing: true,
        pause_wiped: false,
    };

    loop {
        let msg = rx.recv_timeout(Duration::from_secs(4));
        match msg {
            Ok(Cmd::Shutdown) => {
                close_client(&mut client);
                break;
            }
            Ok(Cmd::Disconnect) => {
                enabled = false;
                last = None;
                mem = PresenceMem { playing: true, pause_wiped: false };
                close_client(&mut client);
            }
            Ok(Cmd::Clear) => {
                last = None;
                mem = PresenceMem { playing: true, pause_wiped: false };
                if let Some(c) = client.as_mut() {
                    if c.clear_activity().is_err() {
                        close_client(&mut client);
                    }
                }
            }
            Ok(Cmd::Connect(id)) => {
                enabled = true;
                if app_id != id {
                    close_client(&mut client);
                    app_id = id;
                }
                if client.is_none() {
                    client = connect_now(&app_id);
                    next_retry = Instant::now() + Duration::from_secs(8);
                    mem = PresenceMem { playing: true, pause_wiped: false };
                    if let (Some(c), Some(data)) = (client.as_mut(), last.as_ref()) {
                        if !apply(c, data, &mut mem) {
                            close_client(&mut client);
                        }
                    }
                }
            }
            Ok(Cmd::Update(data)) => {
                last = Some(data);
                if !enabled {
                    continue;
                }
                if client.is_none() && Instant::now() >= next_retry {
                    client = connect_now(&app_id);
                    next_retry = Instant::now() + Duration::from_secs(8);
                }
                if let (Some(c), Some(data)) = (client.as_mut(), last.as_ref()) {
                    if !apply(c, data, &mut mem) {
                        close_client(&mut client);
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if enabled && client.is_none() && Instant::now() >= next_retry {
                    client = connect_now(&app_id);
                    next_retry = Instant::now() + Duration::from_secs(8);
                    mem = PresenceMem { playing: true, pause_wiped: false };
                    if let (Some(c), Some(data)) = (client.as_mut(), last.as_ref()) {
                        if !apply(c, data, &mut mem) {
                            close_client(&mut client);
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[tauri::command]
pub fn discord_connect(client_id: String) -> Result<serde_json::Value, String> {
    let id = resolve_app_id(&client_id);
    post(Cmd::Connect(id.clone()));
    Ok(serde_json::json!({ "ok": true, "id": id }))
}

#[tauri::command]
pub fn discord_disconnect() {
    post(Cmd::Disconnect);
}

#[tauri::command]
pub fn discord_clear() {
    post(Cmd::Clear);
}

#[tauri::command]
pub fn discord_update(data: Value) {
    post(Cmd::Update(data));
}
