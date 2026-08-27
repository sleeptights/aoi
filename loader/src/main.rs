#![cfg_attr(windows, windows_subsystem = "windows")]

use flate2::read::GzDecoder;
use std::{
    cell::RefCell,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering},
        Mutex, OnceLock,
    },
    thread,
    time::Duration,
};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/aoi.exe.gz"));
const UNCOMPRESSED: u64 = match u64::from_str_radix(env!("AOI_UNCOMPRESSED"), 10) {
    Ok(n) => n,
    Err(_) => 0,
};

const WM_SETUP_DONE: u32 = 0x0400 + 1;
const WM_SETUP_ERR: u32 = 0x0400 + 2;

static HWND_PTR: AtomicIsize = AtomicIsize::new(0);
static PROGRESS: AtomicU32 = AtomicU32::new(0);
static UI_GEN: AtomicU32 = AtomicU32::new(1);
static LAST_PAINTED: AtomicU32 = AtomicU32::new(0);
static UI_SCALE: AtomicU32 = AtomicU32::new(100);
static CANCELLED: AtomicBool = AtomicBool::new(false);
static STATUS: Mutex<String> = Mutex::new(String::new());
static INSTALL_ERROR: Mutex<Option<String>> = Mutex::new(None);
static DONE_EXE: Mutex<Option<PathBuf>> = Mutex::new(None);
static FONT_BODY: OnceLock<isize> = OnceLock::new();

fn install_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aoi")
}

fn has_webview2() -> bool {
    const GUID: &str =
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    const GUID_NATIVE: &str =
        r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    let roots = [
        RegKey::predef(HKEY_LOCAL_MACHINE),
        RegKey::predef(HKEY_CURRENT_USER),
    ];
    roots.iter().any(|root| {
        [GUID, GUID_NATIVE].iter().any(|key| {
            root.open_subkey(key)
                .ok()
                .and_then(|k| k.get_value::<String, _>("pv").ok())
                .is_some_and(|pv| !pv.is_empty() && pv != "0.0.0.0")
        })
    })
}

fn hidden_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

fn aoi_still_running() -> bool {
    hidden_cmd("tasklist")
        .args(["/FI", "IMAGENAME eq aoi.exe", "/NH"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.to_ascii_lowercase().contains("aoi.exe"))
        .unwrap_or(false)
}

fn stop_running_player() {
    for _ in 0..12 {
        if !aoi_still_running() {
            return;
        }
        let _ = hidden_cmd("taskkill").args(["/IM", "aoi.exe", "/F"]).status();
        thread::sleep(Duration::from_millis(250));
    }
}

fn bump_ui() {
    UI_GEN.fetch_add(1, Ordering::Relaxed);
}

fn set_progress(p: u32) {
    if PROGRESS.swap(p, Ordering::Relaxed) != p {
        bump_ui();
    }
}

fn set_status(text: &str) {
    if let Ok(mut s) = STATUS.lock() {
        if s.as_str() != text {
            *s = text.to_string();
            bump_ui();
        }
    }
}

fn write_exe(dest: &Path) -> io::Result<()> {
    if CANCELLED.load(Ordering::SeqCst) {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("exe.part");
    let mut decoder = GzDecoder::new(PAYLOAD);
    {
        let mut file = fs::File::create(&tmp)?;
        let mut buf = [0u8; 64 * 1024];
        let mut written = 0u64;
        loop {
            if CANCELLED.load(Ordering::SeqCst) {
                drop(file);
                let _ = fs::remove_file(&tmp);
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
            let n = decoder.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            written += n as u64;
            if UNCOMPRESSED > 0 {
                set_progress(((written.saturating_mul(1000)) / UNCOMPRESSED).min(1000) as u32);
            }
        }
        file.flush()?;
    }
    if CANCELLED.load(Ordering::SeqCst) {
        let _ = fs::remove_file(&tmp);
        return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
    }
    // Overwrite in place. Never delete dest first — a failed copy must leave the old player.
    let mut last_err = None;
    for _ in 0..8 {
        match fs::copy(&tmp, dest) {
            Ok(_) => {
                let _ = fs::remove_file(&tmp);
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(200));
            }
        }
    }
    let _ = fs::remove_file(&tmp);
    Err(last_err.unwrap_or_else(|| io::Error::new(io::ErrorKind::Other, "не смог заменить aoi.exe")))
}

fn powershell_hidden(script: &str) -> io::Result<()> {
    let status = hidden_cmd("powershell")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", script])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "powershell shortcut failed",
        ))
    }
}

fn escape_ps(s: &str) -> String {
    s.replace('\'', "''")
}

fn create_shortcut(exe: &Path, link: &Path) -> io::Result<()> {
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    let work = exe.parent().unwrap_or(exe);
    let script = format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}'); $s.TargetPath = '{}'; $s.WorkingDirectory = '{}'; $s.IconLocation = '{}'; $s.Save()",
        escape_ps(&link.to_string_lossy()),
        escape_ps(&exe.to_string_lossy()),
        escape_ps(&work.to_string_lossy()),
        escape_ps(&exe.to_string_lossy()),
    );
    powershell_hidden(&script)
}

fn shortcuts(exe: &Path) {
    if let Some(desktop) = dirs::desktop_dir() {
        let _ = create_shortcut(exe, &desktop.join("aoi.lnk"));
    }
    if let Some(data) = dirs::data_dir() {
        let start = data
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("aoi.lnk");
        let _ = create_shortcut(exe, &start);
    }
}

fn launch(exe: &Path) -> io::Result<()> {
    if CANCELLED.load(Ordering::SeqCst) {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
    }
    let mut cmd = Command::new(exe);
    cmd.current_dir(exe.parent().unwrap_or(exe));
    #[cfg(windows)]
    {
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB);
    }
    cmd.spawn().map(|_| ())
}

fn sp(v: i32) -> i32 {
    ((v as i64) * UI_SCALE.load(Ordering::Relaxed) as i64 / 100) as i32
}

fn post_to_window(msg: u32) {
    for _ in 0..50 {
        let hwnd = HWND_PTR.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(windows::Win32::Foundation::HWND(hwnd as *mut std::ffi::c_void)),
                    msg,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn install() {
    let go = || -> Result<PathBuf, String> {
        if CANCELLED.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        set_status("останавливаем старый aoi");
        stop_running_player();
        if CANCELLED.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }

        let exe = install_dir().join("aoi.exe");
        set_status("распаковываем плеер");
        write_exe(&exe).map_err(|e| {
            if e.kind() == io::ErrorKind::Interrupted {
                "cancelled".into()
            } else {
                format!("не смог записать aoi.exe: {e}")
            }
        })?;

        if CANCELLED.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }

        set_progress(1000);
        set_status("ярлык на рабочий стол");
        shortcuts(&exe);

        if !has_webview2() {
            set_status("нужен WebView2 — открываю установщик");
            let _ = hidden_cmd("cmd")
                .args([
                    "/C",
                    "start",
                    "",
                    "https://go.microsoft.com/fwlink/p/?LinkId=2124703",
                ])
                .spawn();
        }

        Ok(exe)
    };

    match go() {
        Ok(exe) => {
            if CANCELLED.load(Ordering::SeqCst) {
                return;
            }
            set_status("готово");
            *DONE_EXE.lock().unwrap() = Some(exe);
            post_to_window(WM_SETUP_DONE);
        }
        Err(err) => {
            if err == "cancelled" || CANCELLED.load(Ordering::SeqCst) {
                return;
            }
            *INSTALL_ERROR.lock().unwrap() = Some(err);
            bump_ui();
            post_to_window(WM_SETUP_ERR);
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn status_text() -> String {
    if let Some(err) = INSTALL_ERROR.lock().ok().and_then(|e| e.clone()) {
        return err;
    }
    STATUS.lock().map(|s| s.clone()).unwrap_or_default()
}

thread_local! {
    static BG_BRUSH: RefCell<Option<windows::Win32::Graphics::Gdi::HBRUSH>> = const { RefCell::new(None) };
    static BAR_BRUSH: RefCell<Option<windows::Win32::Graphics::Gdi::HBRUSH>> = const { RefCell::new(None) };
    static TRACK_BRUSH: RefCell<Option<windows::Win32::Graphics::Gdi::HBRUSH>> = const { RefCell::new(None) };
    static LOGO_BRUSH: RefCell<Option<windows::Win32::Graphics::Gdi::HBRUSH>> = const { RefCell::new(None) };
}

unsafe fn draw_bars(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    center_x: i32,
    top: i32,
    height: i32,
) {
    use windows::Win32::Graphics::Gdi::*;

    // Same mark as ui/assets/icon-bars.svg, group 149×157.
    let bars = [
        (0.0f32, 50.0, 37.0, 107.0),
        (56.0, 0.0, 37.0, 157.0),
        (112.0, 29.0, 37.0, 128.0),
    ];
    let scale = height as f32 / 157.0;
    let width = (149.0 * scale).round() as i32;
    let left = center_x - width / 2;
    let brush = LOGO_BRUSH.with(|slot| {
        *slot
            .borrow_mut()
            .get_or_insert_with(|| CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00F4_EDED)))
    });
    let old_brush = SelectObject(hdc, brush.into());
    let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN));
    for (x, y, w, h) in bars {
        let l = left + (x * scale).round() as i32;
        let t = top + (y * scale).round() as i32;
        let r = l + (w * scale).round() as i32;
        let b = t + (h * scale).round() as i32;
        let rad = (r - l).max(2);
        let _ = RoundRect(hdc, l, t, r, b, rad, rad);
    }
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
}

unsafe fn paint_into(hdc: windows::Win32::Graphics::Gdi::HDC, rc: &windows::Win32::Foundation::RECT) {
    use windows::Win32::{
        Foundation::{COLORREF, RECT},
        Graphics::Gdi::*,
    };

    let bg = BG_BRUSH.with(|slot| {
        *slot.borrow_mut().get_or_insert_with(|| CreateSolidBrush(COLORREF(0x000A_0707)))
    });
    FillRect(hdc, rc, bg);

    let body = *FONT_BODY.get().unwrap_or(&0);
    SetBkMode(hdc, TRANSPARENT);
    draw_bars(hdc, rc.right / 2, sp(28), sp(64));

    if body != 0 {
        SelectObject(hdc, HFONT(body as *mut std::ffi::c_void).into());
    }
    let err = INSTALL_ERROR.lock().ok().and_then(|e| e.clone());
    if err.is_some() {
        SetTextColor(hdc, COLORREF(0x008A_8AFF));
    } else {
        SetTextColor(hdc, COLORREF(0x0078_7878));
    }
    let mut line = wide(&status_text());
    let mut line_rc = RECT {
        left: sp(24),
        top: sp(108),
        right: rc.right - sp(24),
        bottom: sp(140),
    };
    DrawTextW(
        hdc,
        &mut line,
        &mut line_rc,
        DT_CENTER | DT_TOP | DT_SINGLELINE | DT_END_ELLIPSIS,
    );

    let bar_w = sp(220);
    let bar_h = sp(4).max(3);
    let bar_x = (rc.right - bar_w) / 2;
    let bar_y = sp(158);
    let track = RECT {
        left: bar_x,
        top: bar_y,
        right: bar_x + bar_w,
        bottom: bar_y + bar_h,
    };
    let track_br = TRACK_BRUSH.with(|slot| {
        *slot.borrow_mut().get_or_insert_with(|| CreateSolidBrush(COLORREF(0x0018_1818)))
    });
    FillRect(hdc, &track, track_br);
    if err.is_none() {
        let fill_w = (bar_w as u32 * PROGRESS.load(Ordering::Relaxed) / 1000) as i32;
        if fill_w > 0 {
            let fill = RECT {
                left: bar_x,
                top: bar_y,
                right: bar_x + fill_w,
                bottom: bar_y + bar_h,
            };
            let bar = BAR_BRUSH.with(|slot| {
                *slot
                    .borrow_mut()
                    .get_or_insert_with(|| CreateSolidBrush(COLORREF(0x00B2_B2B2)))
            });
            FillRect(hdc, &fill, bar);
        }
    }

    SetTextColor(hdc, COLORREF(0x0048_4848));
    let mut hint = wide("потом просто открой ярлык на рабочем столе");
    let mut hint_rc = RECT {
        left: sp(20),
        top: sp(186),
        right: rc.right - sp(20),
        bottom: sp(220),
    };
    DrawTextW(
        hdc,
        &mut hint,
        &mut hint_rc,
        DT_CENTER | DT_TOP | DT_SINGLELINE,
    );

    SetTextColor(hdc, COLORREF(0x0044_4444));
    let mut xlbl = wide("×");
    let mut xrc = RECT {
        left: rc.right - sp(36),
        top: sp(8),
        right: rc.right - sp(12),
        bottom: sp(32),
    };
    DrawTextW(hdc, &mut xlbl, &mut xrc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
}

unsafe fn paint(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::{
        Foundation::RECT,
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    };

    let mut ps = PAINTSTRUCT::default();
    let win = BeginPaint(hwnd, &mut ps);
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    let w = rc.right.max(1);
    let h = rc.bottom.max(1);

    let mem = CreateCompatibleDC(Some(win));
    if !mem.is_invalid() {
        let hdc = mem;
        let bmp = CreateCompatibleBitmap(win, w, h);
        let old = SelectObject(hdc, bmp.into());
        paint_into(hdc, &rc);
        let _ = BitBlt(win, 0, 0, w, h, Some(hdc), 0, 0, SRCCOPY);
        SelectObject(hdc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(hdc);
    } else {
        paint_into(win, &rc);
    }

    LAST_PAINTED.store(UI_GEN.load(Ordering::Relaxed), Ordering::Relaxed);
    let _ = EndPaint(hwnd, &ps);
}

unsafe extern "system" fn wndproc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::InvalidateRect,
        UI::WindowsAndMessaging::*,
    };

    match msg {
        WM_CREATE => {
            HWND_PTR.store(hwnd.0 as isize, Ordering::SeqCst);
            let _ = SetTimer(Some(hwnd), 1, 100, None);
            let _ = InvalidateRect(Some(hwnd), None, false);
            if thread::Builder::new()
                .name("aoi-setup".into())
                .spawn(install)
                .is_err()
            {
                *INSTALL_ERROR.lock().unwrap() = Some("не смог запустить установку".into());
                bump_ui();
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if UI_GEN.load(Ordering::Relaxed) != LAST_PAINTED.load(Ordering::Relaxed) {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_NCHITTEST => {
            let mut pt = POINT {
                x: (lparam.0 as i32) & 0xFFFF,
                y: ((lparam.0 as i32) >> 16) & 0xFFFF,
            };
            // signed coords
            pt.x = lparam.0 as i16 as i32;
            pt.y = ((lparam.0 >> 16) as i16) as i32;
            let _ = windows::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut pt);
            let mut rc = windows::Win32::Foundation::RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            if pt.x >= rc.right - sp(40) && pt.y <= sp(32) {
                return LRESULT(HTCLIENT as isize);
            }
            LRESULT(HTCAPTION as isize)
        }
        WM_LBUTTONUP => {
            let x = (lparam.0 as i16) as i32;
            let y = ((lparam.0 >> 16) as i16) as i32;
            let mut rc = windows::Win32::Foundation::RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            if x >= rc.right - sp(40) && y <= sp(32) {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        m if m == WM_SETUP_DONE => {
            thread::spawn(|| {
                thread::sleep(Duration::from_millis(700));
                if CANCELLED.load(Ordering::SeqCst) {
                    return;
                }
                if let Some(exe) = DONE_EXE.lock().ok().and_then(|g| g.clone()) {
                    if let Err(e) = launch(&exe) {
                        *INSTALL_ERROR.lock().unwrap() = Some(format!("плеер записан, но не запустился: {e}"));
                        bump_ui();
                        let hwnd = HWND_PTR.load(Ordering::SeqCst);
                        if hwnd != 0 {
                            unsafe {
                                let _ = PostMessageW(
                                    Some(HWND(hwnd as *mut std::ffi::c_void)),
                                    WM_SETUP_ERR,
                                    WPARAM(0),
                                    LPARAM(0),
                                );
                            }
                        }
                        return;
                    }
                }
                thread::sleep(Duration::from_millis(250));
                let hwnd = HWND_PTR.load(Ordering::SeqCst);
                if hwnd != 0 {
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(hwnd as *mut std::ffi::c_void)),
                            WM_CLOSE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            });
            LRESULT(0)
        }
        m if m == WM_SETUP_ERR => {
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_DESTROY => {
            CANCELLED.store(true, Ordering::SeqCst);
            HWND_PTR.store(0, Ordering::SeqCst);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn already_installing() -> bool {
    use windows::{
        core::w,
        Win32::{
            Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND},
            System::Threading::CreateMutexW,
            UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE},
        },
    };
    unsafe {
        match CreateMutexW(None, true, w!("Local\\aoi-setup")) {
            Ok(_handle) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    if let Ok(existing) = FindWindowW(w!("aoi-setup"), None) {
                        if existing != HWND::default() {
                            let _ = ShowWindow(existing, SW_RESTORE);
                            let _ = SetForegroundWindow(existing);
                        }
                    }
                    return true;
                }
            }
            Err(_) => {}
        }
    }
    false
}

fn main() {
    if already_installing() {
        return;
    }
    set_status("собираемся");

    unsafe {
        use windows::{
            core::w,
            Win32::{
                Foundation::{COLORREF, HWND, RECT},
                Graphics::Gdi::*,
                System::LibraryLoader::GetModuleHandleW,
                UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
                UI::WindowsAndMessaging::*,
            },
        };

        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hdc = GetDC(None);
        let dpi = GetDeviceCaps(Some(hdc), LOGPIXELSX).max(96);
        let _ = ReleaseDC(None, hdc);
        UI_SCALE.store(((dpi as u32) * 100 / 96).clamp(100, 300), Ordering::Relaxed);

        FONT_BODY.get_or_init(|| {
            CreateFontW(
                sp(15),
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                false.into(),
                false.into(),
                false.into(),
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                w!("Segoe UI"),
            )
            .0 as isize
        });

        let class = w!("aoi-setup");
        let icon = LoadIconW(
            Some(hinstance.into()),
            windows::core::PCWSTR(1usize as *const u16),
        )
        .unwrap_or_default();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            hIcon: icon,
            hIconSm: icon,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: class,
            hbrBackground: CreateSolidBrush(COLORREF(0x000A_0707)),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        let ww = sp(420);
        let wh = sp(240);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!("aoi"),
            WS_POPUP | WS_VISIBLE,
            (sw - ww) / 2,
            (sh - wh) / 2,
            ww,
            wh,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW");

        HWND_PTR.store(hwnd.0 as isize, Ordering::SeqCst);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = hwnd;
        let _ = RECT::default();
        let _ = HWND::default();
    }
}
