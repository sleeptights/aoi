mod discord;
mod extras;
mod lastfm;
mod media_protocol;
mod music;
mod settings;
mod soundcloud;
mod state;
mod update;

use parking_lot::Mutex;
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{fs::OpenOptions, io::Write, panic};

static QUIT_ARMED: AtomicBool = AtomicBool::new(false);
use tauri::{
    include_image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconEvent, TrayIconBuilder},
    Emitter, Manager, RunEvent, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_crash_hook();

    let app = match tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(Mutex::new(AppState::default()))
        .register_asynchronous_uri_scheme_protocol("media", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            std::thread::spawn(move || {
                responder.respond(media_protocol::response(&app, request));
            });
        })
        .invoke_handler(tauri::generate_handler![
            settings::load_settings,
            settings::save_settings,
            settings::backup_settings,
            settings::restore_settings_backup,
            settings::set_login_item,
            music::select_music_folder,
            music::select_player_bg_image,
            music::scan_music_folder,
            music::get_cover_art,
            soundcloud::sc_login,
            soundcloud::sc_fetch,
            soundcloud::sc_check_covers,
            soundcloud::sc_cache_cover,
            soundcloud::sc_clear_covers_cache,
            soundcloud::sc_clear_likes_cache,
            soundcloud::sc_load_likes_cache,
            soundcloud::sc_save_likes_cache,
            discord::discord_connect,
            discord::discord_disconnect,
            discord::discord_update,
            discord::discord_clear,
            extras::fetch_json,
            extras::lrclib_lookup,
            extras::read_lrc,
            extras::log_play,
            extras::get_plays,
            extras::clear_plays,
            extras::open_mini,
            extras::mini_expand,
            extras::mini_cmd,
            extras::mini_state,
            extras::mini_menu,
            extras::mini_always_on_top,
            extras::battery_pct,
            update::app_version,
            update::check_for_update,
            update::install_update,
            lastfm::lfm_auth,
            lastfm::lfm_call,
            win_minimize,
            win_maximize,
            win_close,
            win_quit,
        ])
        .setup(|app| {
            discord::init();
            log_boot("setup start");

            // tray: Открыть | Мини-плеер | Выйти
            let show = MenuItem::with_id(app, "show", "Открыть", true, None::<&str>)?;
            let mini = MenuItem::with_id(app, "mini", "Мини-плеер", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Выйти", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &mini, &quit])?;

            let tray = TrayIconBuilder::new()
                .tooltip("aoi")
                .icon(include_image!("icons/32x32.png"))
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if app
                            .get_webview_window("mini")
                            .and_then(|w| w.is_visible().ok())
                            .unwrap_or(false)
                        {
                            extras::mini_expand(app.clone());
                            return;
                        }
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "mini" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = extras::open_mini(handle).await {
                                log_boot(&format!("tray mini: {e}"));
                            }
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if app
                            .get_webview_window("mini")
                            .and_then(|w| w.is_visible().ok())
                            .unwrap_or(false)
                        {
                            extras::mini_expand(app.clone());
                            return;
                        }
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                });
            if let Err(e) = tray.build(app) {
                log_boot(&format!("tray skipped: {e}"));
            }

            if let Some(win) = app.get_webview_window("main") {
                disable_native_context_menu(&win);
            }

            // Media keys are nice-to-have; another app holding them must not kill aoi.
            if let Err(e) = register_media_shortcuts(app.handle()) {
                log_boot(&format!("media shortcuts skipped: {e}"));
            }
            log_boot("setup ok");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "mini" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(mini) = window.app_handle().get_webview_window("mini") {
                        let _ = mini.hide();
                    }
                    if let Some(main) = window.app_handle().get_webview_window("main") {
                        let _ = main.show();
                        let _ = main.unminimize();
                        let _ = main.set_focus();
                    }
                }
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                // трей-логика только для главного окна: мини-плеер и окна
                // логина должны закрываться по-настоящему
                if window.label() != "main" {
                    return;
                }
                let settings = settings::read_settings_file(window.app_handle());
                if settings
                    .get("minimizeToTray")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    let _ = window.hide();
                    api.prevent_close();
                } else {
                    api.prevent_close();
                    request_app_close(window.app_handle());
                }
            }
        })
        .build(tauri::generate_context!())
    {
        Ok(app) => app,
        Err(e) => {
            let msg = format!("aoi не смог запуститься:\n{e}\n\nОбычно это значит, что не установлен WebView2 Runtime.\nСкачай: https://go.microsoft.com/fwlink/p/?LinkId=2124703");
            log_boot(&msg);
            show_fatal(&msg);
            return;
        }
    };

    app.run(|_app, event| {
        if let RunEvent::Exit = event {
            discord::shutdown();
        }
    });
}

fn install_crash_hook() {
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let msg = format!("panic: {info}");
        log_boot(&msg);
        prev(info);
    }));
}

fn log_boot(line: &str) {
    let path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("aoi.player")
        .join("aoi.log");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{ts}] {line}");
    }
}

fn show_fatal(msg: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let text: Vec<u16> = std::ffi::OsStr::new(msg)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let title: Vec<u16> = std::ffi::OsStr::new("aoi")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                0x00000010, // MB_ICONERROR
            );
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("{msg}");
    }
}

fn disable_native_context_menu(win: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        let win = win.clone();
        let _ = win.with_webview(move |webview| {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                if let Ok(core) = webview.controller().CoreWebView2() {
                    if let Ok(settings) = core.Settings() {
                        let _ = settings.SetAreDefaultContextMenusEnabled(false);
                    }
                }
            }));
        });
    }
    let _ = win;
}

fn register_media_shortcuts(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

    let play = Shortcut::new(None, Code::MediaPlayPause);
    let next = Shortcut::new(None, Code::MediaTrackNext);
    let prev = Shortcut::new(None, Code::MediaTrackPrevious);

    app.global_shortcut().on_shortcut(play, move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            let _ = app.emit("media-play-pause", ());
        }
    })?;

    app.global_shortcut().on_shortcut(next, move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            let _ = app.emit("media-next", ());
        }
    })?;

    app.global_shortcut().on_shortcut(prev, move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            let _ = app.emit("media-prev", ());
        }
    })?;

    Ok(())
}

fn request_app_close(app: &tauri::AppHandle) {
    let settings = settings::read_settings_file(app);
    if settings
        .get("minimizeToTray")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
        return;
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit("aoi-close", ());
    } else {
        let _ = app.emit("aoi-close", ());
    }
    if QUIT_ARMED.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(5200));
        app.exit(0);
    });
}

#[tauri::command]
fn win_minimize(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn win_maximize(window: tauri::WebviewWindow) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

#[tauri::command]
fn win_close(window: tauri::WebviewWindow) {
    if window.label() != "main" {
        let _ = window.close();
        return;
    }
    request_app_close(window.app_handle());
}

#[tauri::command]
fn win_quit(app: tauri::AppHandle) {
    app.exit(0);
}
