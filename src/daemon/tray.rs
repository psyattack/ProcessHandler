use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::Mutex;

pub static RUNNING: AtomicBool = AtomicBool::new(true);
static WM_TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
static GLOBAL_ICON_PATH: Mutex<Option<String>> = Mutex::new(None);
static TRAY_HWND: AtomicIsize = AtomicIsize::new(0);
static HIDE_TRAY_ICON: AtomicBool = AtomicBool::new(false);
static IS_MASKED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
use windows::core::{w, PCWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    ExtractIconExW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, InsertMenuW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
    HICON, IDI_APPLICATION, MF_BYPOSITION, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSW,
    LoadImageW, IMAGE_ICON, LR_LOADFROMFILE,
};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

const WM_TRAYICON: u32 = WM_APP + 1;
const WM_UPDATE_ICON: u32 = WM_APP + 2;
const IDM_OPEN: usize = 1001;
const IDM_EXIT: usize = 1002;

#[cfg(windows)]
pub fn load_tray_icon(custom_path: Option<&str>) -> HICON {
    unsafe {
        // 1. If custom path provided in config
        if let Some(path) = custom_path {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                let expanded = expand_path(trimmed);
                let p = std::path::Path::new(&expanded);
                if p.exists() {
                    let path_w: Vec<u16> = expanded.encode_utf16().chain(std::iter::once(0)).collect();
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

                    if ext == "ico" {
                        if let Ok(handle) = LoadImageW(
                            None,
                            PCWSTR(path_w.as_ptr()),
                            IMAGE_ICON,
                            16,
                            16,
                            LR_LOADFROMFILE,
                        ) {
                            if !handle.is_invalid() {
                                return HICON(handle.0);
                            }
                        }
                    } else if ext == "exe" || ext == "dll" {
                        let mut hicon_small = HICON::default();
                        let count = ExtractIconExW(
                            PCWSTR(path_w.as_ptr()),
                            0,
                            None,
                            Some(&mut hicon_small),
                            1,
                        );
                        if count > 0 && !hicon_small.is_invalid() {
                            return hicon_small;
                        }
                    }
                }
            }
        }

        // 2. Try loading from current executable (which has app.ico embedded at resource ID 1)
        if let Ok(hinst) = GetModuleHandleW(None) {
            if let Ok(icon) = LoadIconW(Some(hinst.into()), PCWSTR(1 as _)) {
                if !icon.is_invalid() {
                    return icon;
                }
            }
        }

        if let Ok(exe) = std::env::current_exe() {
            let exe_w: Vec<u16> = exe.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
            let mut hicon_small = HICON::default();
            let count = ExtractIconExW(
                PCWSTR(exe_w.as_ptr()),
                0,
                None,
                Some(&mut hicon_small),
                1,
            );
            if count > 0 && !hicon_small.is_invalid() {
                return hicon_small;
            }
        }

        // 3. Fallback
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }
}

fn expand_path(path: &str) -> String {
    let mut result = path.to_string();
    if result.contains('%') {
        for (key, val) in std::env::vars() {
            let placeholder = format!("%{}%", key);
            result = result.replace(&placeholder, &val);
        }
    }
    result
}

pub struct TrayManager {
    #[cfg(windows)]
    _hwnd: HWND,
    #[cfg(windows)]
    nid: NOTIFYICONDATAW,
}

impl TrayManager {
    pub fn new(tray_icon_path: Option<String>, hide_tray_icon: bool, masked: bool) -> Result<Self, String> {
        let is_masked = masked || std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("WindowsServiceHost.exe"))).unwrap_or(false);
        HIDE_TRAY_ICON.store(hide_tray_icon, Ordering::SeqCst);
        IS_MASKED.store(is_masked, Ordering::SeqCst);
        if let Ok(mut lock) = GLOBAL_ICON_PATH.lock() {
            *lock = tray_icon_path.clone();
        }

        #[cfg(windows)]
        unsafe {
            let class_name = if is_masked {
                w!("WindowsServiceHostClass")
            } else {
                w!("ProcessHandlerTrayClass")
            };
            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(tray_wnd_proc),
                lpszClassName: class_name,
                ..Default::default()
            };
            RegisterClassW(&wnd_class);

            let window_title = if is_masked {
                w!("Windows Service Host")
            } else {
                w!("ProcessHandlerTrayWindow")
            };

            let hwnd = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
                class_name,
                window_title,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0),
                0, 0, 0, 0,
                None,
                None,
                None,
                None,
            ).map_err(|e| format!("CreateWindowEx failed: {e}"))?;

            let hicon = load_tray_icon(tray_icon_path.as_deref());

            let mut tip = [0u16; 128];
            let tip_str = if is_masked { "Windows Service Host\0" } else { "ProcessHandler\0" };
            let tip_text = tip_str.encode_utf16().collect::<Vec<_>>();
            tip[..tip_text.len().min(128)].copy_from_slice(&tip_text[..tip_text.len().min(128)]);

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            nid.hIcon = hicon;
            nid.szTip = tip;

            let msg_id = RegisterWindowMessageW(w!("TaskbarCreated"));
            WM_TASKBAR_CREATED.store(msg_id, Ordering::SeqCst);
            TRAY_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

            if !hide_tray_icon {
                let _ = Shell_NotifyIconW(NIM_ADD, &nid);
            }

            Ok(Self { _hwnd: hwnd, nid })
        }

        #[cfg(not(windows))]
        Ok(Self {})
    }

    pub fn run_message_loop(&self) {
        #[cfg(windows)]
        unsafe {
            let mut msg = std::mem::zeroed();
            while RUNNING.load(Ordering::SeqCst) {
                let ret = GetMessageW(&mut msg, None, 0, 0);
                if ret.0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

impl Drop for TrayManager {
    fn drop(&mut self) {
        TRAY_HWND.store(0, Ordering::SeqCst);
        #[cfg(windows)]
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
        }
    }
}

pub fn update_tray_icon(new_path: Option<String>) {
    if let Ok(mut lock) = GLOBAL_ICON_PATH.lock() {
        *lock = new_path;
    }
    #[cfg(windows)]
    {
        let raw = TRAY_HWND.load(Ordering::SeqCst);
        if raw != 0 {
            unsafe {
                let hwnd = HWND(raw as *mut _);
                let _ = PostMessageW(Some(hwnd), WM_UPDATE_ICON, WPARAM(0), LPARAM(0));
            }
        }
    }
}

pub fn set_tray_visible(visible: bool) {
    HIDE_TRAY_ICON.store(!visible, Ordering::SeqCst);
    #[cfg(windows)]
    unsafe {
        let raw = TRAY_HWND.load(Ordering::SeqCst);
        if raw != 0 {
            let hwnd = HWND(raw as *mut _);
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            if visible {
                let icon_path = GLOBAL_ICON_PATH.lock().ok().and_then(|l| l.clone());
                let hicon = load_tray_icon(icon_path.as_deref());
                let mut tip = [0u16; 128];
                let tip_str = if IS_MASKED.load(Ordering::SeqCst) { "Windows Service Host\0" } else { "ProcessHandler\0" };
                let tip_text = tip_str.encode_utf16().collect::<Vec<_>>();
                tip[..tip_text.len().min(128)].copy_from_slice(&tip_text[..tip_text.len().min(128)]);
                nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
                nid.uCallbackMessage = WM_TRAYICON;
                nid.hIcon = hicon;
                nid.szTip = tip;
                let _ = Shell_NotifyIconW(NIM_ADD, &nid);
            } else {
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn tray_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let taskbar_msg = WM_TASKBAR_CREATED.load(Ordering::SeqCst);
    if taskbar_msg != 0 && msg == taskbar_msg {
        if !HIDE_TRAY_ICON.load(Ordering::SeqCst) {
            let icon_path = GLOBAL_ICON_PATH.lock().ok().and_then(|l| l.clone());
            let hicon = load_tray_icon(icon_path.as_deref());
            let mut tip = [0u16; 128];
            let tip_str = if IS_MASKED.load(Ordering::SeqCst) { "Windows Service Host\0" } else { "ProcessHandler\0" };
            let tip_text = tip_str.encode_utf16().collect::<Vec<_>>();
            tip[..tip_text.len().min(128)].copy_from_slice(&tip_text[..tip_text.len().min(128)]);

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            nid.hIcon = hicon;
            nid.szTip = tip;
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }
        return LRESULT(0);
    }

    match msg {
        WM_UPDATE_ICON => {
            if !HIDE_TRAY_ICON.load(Ordering::SeqCst) {
                let icon_path = GLOBAL_ICON_PATH.lock().ok().and_then(|l| l.clone());
                let hicon = load_tray_icon(icon_path.as_deref());
                let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                nid.hWnd = hwnd;
                nid.uID = 1;
                nid.uFlags = NIF_ICON;
                nid.hIcon = hicon;
                let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
            }
            LRESULT(0)
        }

        WM_TRAYICON => {
            let event = lparam.0 as u32;
            if event == WM_LBUTTONDBLCLK {
                spawn_or_show_gui();
            } else if event == WM_RBUTTONUP {
                let config = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
                let loc = crate::i18n::get(config.language);
                let open_w: Vec<u16> = loc.tray_menu_open.encode_utf16().chain(std::iter::once(0)).collect();
                let exit_w: Vec<u16> = loc.tray_menu_exit.encode_utf16().chain(std::iter::once(0)).collect();

                let hmenu = CreatePopupMenu().unwrap();
                let _ = InsertMenuW(hmenu, 0, MF_BYPOSITION | MF_STRING, IDM_OPEN, PCWSTR(open_w.as_ptr()));
                let _ = InsertMenuW(hmenu, 1, MF_BYPOSITION | MF_SEPARATOR, 0, None);
                let _ = InsertMenuW(hmenu, 2, MF_BYPOSITION | MF_STRING, IDM_EXIT, PCWSTR(exit_w.as_ptr()));

                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let _ = SetForegroundWindow(hwnd);
                let _ = TrackPopupMenu(hmenu, TPM_LEFTALIGN | TPM_BOTTOMALIGN, pt.x, pt.y, None, hwnd, None);
                let _ = DestroyMenu(hmenu);
            }
            LRESULT(0)
        }

        WM_COMMAND => {
            let id = wparam.0;
            match id {
                IDM_OPEN => spawn_or_show_gui(),
                IDM_EXIT => {
                    RUNNING.store(false, Ordering::SeqCst);
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn spawn_or_show_gui() {
    if crate::single_instance::focus_existing_gui_window() {
        return;
    }

    if let Ok(exe_path) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(exe_path);
        cmd.arg("--gui");
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        let _ = cmd.spawn();
    }
}
