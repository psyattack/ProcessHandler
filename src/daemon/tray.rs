use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::Mutex;

pub static RUNNING: AtomicBool = AtomicBool::new(true);
static WM_TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
static GLOBAL_ICON_PATH: Mutex<Option<String>> = Mutex::new(None);
static TRAY_HWND: AtomicIsize = AtomicIsize::new(0);
static HIDE_TRAY_ICON: AtomicBool = AtomicBool::new(false);
static IS_MASKED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
use windows::core::{w, PCWSTR, PWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateDIBSection, DeleteObject, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP,
};
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    ExtractIconExW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, GetSystemMetrics, InsertMenuItemW, LoadIconW, LoadImageW,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    SetMenuDefaultItem, TrackPopupMenu, TranslateMessage, HICON, IDI_APPLICATION, IMAGE_ICON,
    LR_LOADFROMFILE, MENUITEMINFOW, MFT_SEPARATOR, MIIM_BITMAP, MIIM_FTYPE, MIIM_ID,
    MIIM_STRING, SM_CXSMICON, SM_CYSMICON, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON,
    WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};

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
                w!("SynectorTrayClass")
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
                w!("SynectorTrayWindow")
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
            let tip_str = if is_masked { "Windows Service Host\0" } else { "Synector\0" };
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

            let config = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
            let is_dark = config.theme != crate::config::AppTheme::Light;
            apply_menu_theme(Some(hwnd), is_dark);

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
                let tip_str = if IS_MASKED.load(Ordering::SeqCst) { "Windows Service Host\0" } else { "Synector\0" };
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
            let tip_str = if IS_MASKED.load(Ordering::SeqCst) { "Windows Service Host\0" } else { "Synector\0" };
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
                let is_dark = config.theme != crate::config::AppTheme::Light;
                apply_menu_theme(Some(hwnd), is_dark);

                let loc = crate::i18n::get(config.language);
                let mut open_w: Vec<u16> = loc.tray_menu_open.encode_utf16().chain(std::iter::once(0)).collect();
                let mut exit_w: Vec<u16> = loc.tray_menu_exit.encode_utf16().chain(std::iter::once(0)).collect();

                let icon_size = GetSystemMetrics(SM_CXSMICON)
                    .max(GetSystemMetrics(SM_CYSMICON))
                    .max(16) as u32;
                let hbmp_open = generate_play_menu_bitmap(icon_size);
                let hbmp_exit = generate_exit_menu_bitmap(icon_size);

                if let Ok(hmenu) = CreatePopupMenu() {
                    // 1. Open Synector (Bold default item with play launch icon)
                    let mut mask_open = MIIM_ID | MIIM_STRING;
                    if hbmp_open.is_some() {
                        mask_open |= MIIM_BITMAP;
                    }
                    let item_open = MENUITEMINFOW {
                        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                        fMask: mask_open,
                        wID: IDM_OPEN as u32,
                        dwTypeData: PWSTR(open_w.as_mut_ptr()),
                        cch: open_w.len() as u32,
                        hbmpItem: hbmp_open.unwrap_or_default(),
                        ..Default::default()
                    };
                    let _ = InsertMenuItemW(hmenu, 0, true, &item_open);
                    let _ = SetMenuDefaultItem(hmenu, IDM_OPEN as u32, 0);

                    // 2. Separator
                    let sep = MENUITEMINFOW {
                        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                        fMask: MIIM_FTYPE,
                        fType: MFT_SEPARATOR,
                        ..Default::default()
                    };
                    let _ = InsertMenuItemW(hmenu, 1, true, &sep);

                    // 3. Exit (with power/exit icon)
                    let mut mask_exit = MIIM_ID | MIIM_STRING;
                    if hbmp_exit.is_some() {
                        mask_exit |= MIIM_BITMAP;
                    }
                    let item_exit = MENUITEMINFOW {
                        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                        fMask: mask_exit,
                        wID: IDM_EXIT as u32,
                        dwTypeData: PWSTR(exit_w.as_mut_ptr()),
                        cch: exit_w.len() as u32,
                        hbmpItem: hbmp_exit.unwrap_or_default(),
                        ..Default::default()
                    };
                    let _ = InsertMenuItemW(hmenu, 2, true, &item_exit);

                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    let _ = SetForegroundWindow(hwnd);
                    let _ = TrackPopupMenu(
                        hmenu,
                        TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
                        pt.x,
                        pt.y,
                        None,
                        hwnd,
                        None,
                    );
                    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
                    let _ = DestroyMenu(hmenu);
                }

                if let Some(bmp) = hbmp_open {
                    let _ = DeleteObject(bmp.into());
                }
                if let Some(bmp) = hbmp_exit {
                    let _ = DeleteObject(bmp.into());
                }
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

#[cfg(windows)]
pub fn apply_menu_theme(hwnd: Option<HWND>, is_dark: bool) {
    unsafe {
        // uxtheme.dll ordinals:
        // 135: SetPreferredAppMode (0 = Default, 1 = AllowDark, 2 = ForceDark, 3 = ForceLight)
        // 136: FlushMenuThemes
        // 133: AllowDarkModeForWindow
        if let Ok(uxtheme) = LoadLibraryW(w!("uxtheme.dll")) {
            let mode = if is_dark { 2 } else { 3 };
            let set_preferred_app_mode: Option<unsafe extern "system" fn(i32) -> i32> =
                std::mem::transmute(GetProcAddress(uxtheme, windows::core::PCSTR(135 as _)));
            if let Some(set_mode) = set_preferred_app_mode {
                set_mode(mode);
            }

            let flush_menu_themes: Option<unsafe extern "system" fn()> =
                std::mem::transmute(GetProcAddress(uxtheme, windows::core::PCSTR(136 as _)));
            if let Some(flush) = flush_menu_themes {
                flush();
            }

            if let Some(h) = hwnd {
                let allow_dark_for_window: Option<unsafe extern "system" fn(HWND, bool) -> bool> =
                    std::mem::transmute(GetProcAddress(uxtheme, windows::core::PCSTR(133 as _)));
                if let Some(allow_wnd) = allow_dark_for_window {
                    let _ = allow_wnd(h, is_dark);
                }
            }
        }
    }
}

#[cfg(windows)]
unsafe fn create_pargb_bitmap(width: i32, height: i32, rgba_pixels: &[u8]) -> Option<HBITMAP> {
    if width <= 0 || height <= 0 || rgba_pixels.len() < (width * height * 4) as usize {
        return None;
    }

    let hdc_screen = GetDC(None);
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let hbmp = CreateDIBSection(Some(hdc_screen), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0).ok()?;
    ReleaseDC(None, hdc_screen);

    if bits_ptr.is_null() {
        let _ = DeleteObject(hbmp.into());
        return None;
    }

    let dest = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (width * height * 4) as usize);

    #[allow(clippy::chunks_exact_to_as_chunks)]
    for (src, dst) in rgba_pixels.chunks_exact(4).zip(dest.chunks_exact_mut(4)) {
        let r = src[0] as u32;
        let g = src[1] as u32;
        let b = src[2] as u32;
        let a = src[3] as u32;

        if a == 0 {
            dst[0] = 0;
            dst[1] = 0;
            dst[2] = 0;
            dst[3] = 0;
        } else if a == 255 {
            dst[0] = b as u8;
            dst[1] = g as u8;
            dst[2] = r as u8;
            dst[3] = 255;
        } else {
            dst[0] = ((b * a + 127) / 255) as u8;
            dst[1] = ((g * a + 127) / 255) as u8;
            dst[2] = ((r * a + 127) / 255) as u8;
            dst[3] = a as u8;
        }
    }

    Some(hbmp)
}

#[cfg(windows)]
fn generate_play_menu_bitmap(size: u32) -> Option<HBITMAP> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;

    let r_corner = (s * 0.08).max(1.2);
    // Base triangle vertices before dilation by r_corner
    let v0 = (s * 0.32, s * 0.26);
    let v1 = (s * 0.32, s * 0.74);
    let v2 = (s * 0.74, s * 0.50);

    fn dist_sq_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
        let vx = b.0 - a.0;
        let vy = b.1 - a.1;
        let wx = p.0 - a.0;
        let wy = p.1 - a.1;
        let c1 = wx * vx + wy * vy;
        if c1 <= 0.0 {
            return wx * wx + wy * wy;
        }
        let c2 = vx * vx + vy * vy;
        if c2 <= c1 {
            let dx = p.0 - b.0;
            let dy = p.1 - b.1;
            return dx * dx + dy * dy;
        }
        let t = c1 / c2;
        let qx = a.0 + t * vx;
        let qy = a.1 + t * vy;
        let dx = p.0 - qx;
        let dy = p.1 - qy;
        dx * dx + dy * dy
    }

    for y in 0..size {
        for x in 0..size {
            let p = (x as f32 + 0.5, y as f32 + 0.5);

            let d0 = dist_sq_to_segment(p, v0, v1);
            let d1 = dist_sq_to_segment(p, v1, v2);
            let d2 = dist_sq_to_segment(p, v2, v0);
            let min_d = d0.min(d1).min(d2).sqrt();

            let c0 = (v1.0 - v0.0) * (p.1 - v0.1) - (v1.1 - v0.1) * (p.0 - v0.0);
            let c1 = (v2.0 - v1.0) * (p.1 - v1.1) - (v2.1 - v1.1) * (p.0 - v1.0);
            let c2 = (v0.0 - v2.0) * (p.1 - v2.1) - (v0.1 - v2.1) * (p.0 - v2.0);

            let is_inside = (c0 >= 0.0 && c1 >= 0.0 && c2 >= 0.0) || (c0 <= 0.0 && c1 <= 0.0 && c2 <= 0.0);

            let signed_dist = if is_inside { -min_d } else { min_d };
            let dist_to_shape = signed_dist - r_corner;

            let alpha = (0.5 - dist_to_shape).clamp(0.0, 1.0);

            if alpha > 0.02 {
                let idx = ((y * size + x) * 4) as usize;
                let a_byte = (alpha * 255.0) as u8;
                // Vibrant Emerald Play Green: #22C55E (rgb 34, 197, 94)
                rgba[idx] = 34;
                rgba[idx + 1] = 197;
                rgba[idx + 2] = 94;
                rgba[idx + 3] = a_byte;
            }
        }
    }

    unsafe { create_pargb_bitmap(size as i32, size as i32, &rgba) }
}

#[cfg(windows)]
fn generate_exit_menu_bitmap(size: u32) -> Option<HBITMAP> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    let center = s / 2.0;
    let r_out = s * 0.40;
    let r_in = s * 0.24;
    let line_w = (s * 0.12).max(1.5);

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - center;
            let dy = py - center;
            let dist = (dx * dx + dy * dy).sqrt();

            // Power button arc (exclude top 60 degrees)
            let in_ring = dist >= (r_in - 0.7) && dist <= (r_out + 0.7);
            let angle = dy.atan2(dx).to_degrees(); // -180 to 180 (top is -90)
            let not_top_gap = angle > -60.0 || angle < -120.0;

            let mut alpha = 0.0f32;
            if in_ring && not_top_gap {
                let ring_center = (r_in + r_out) / 2.0;
                let ring_half_w = (r_out - r_in) / 2.0;
                let d_ring = (dist - ring_center).abs();
                let a_ring = (1.0 - (d_ring / ring_half_w).clamp(0.0, 1.0)).powf(0.5);
                alpha = alpha.max(a_ring);
            }

            // Vertical power bar at top
            if dx.abs() <= (line_w / 2.0 + 0.7) && py >= (center - r_out - 0.5) && py <= (center + 0.5) {
                let d_line = dx.abs() / (line_w / 2.0);
                let a_line = (1.0 - d_line.clamp(0.0, 1.0)).powf(0.5);
                alpha = alpha.max(a_line);
            }

            if alpha > 0.05 {
                let idx = ((y * size + x) * 4) as usize;
                let a_byte = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
                // Coral Red (#EF4444)
                rgba[idx] = 239;
                rgba[idx + 1] = 68;
                rgba[idx + 2] = 68;
                rgba[idx + 3] = a_byte;
            }
        }
    }

    unsafe { create_pargb_bitmap(size as i32, size as i32, &rgba) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_tray_menu_bitmaps_creation_and_cleanup() {
        let play_bmp = generate_play_menu_bitmap(16);
        assert!(play_bmp.is_some(), "Play menu bitmap should be generated successfully");
        if let Some(bmp) = play_bmp {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(bmp.into());
            }
        }

        let exit_bmp = generate_exit_menu_bitmap(16);
        assert!(exit_bmp.is_some(), "Exit menu bitmap should be generated successfully");
        if let Some(bmp) = exit_bmp {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(bmp.into());
            }
        }
    }
}
