use std::collections::{HashMap, HashSet};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex, OnceLock};

use crate::models::{MonitorInfo, OverlayDefinition};
#[cfg(test)]
use crate::models::OverlayAnchor;

pub struct ActiveOverlayProcess {
    pub id: String,
    pub child: Option<Child>,
    pub close_with_target: bool,
    pub target_process: String,
    pub fade_out_ms: Option<u32>,
}

static ACTIVE_PROCESSES: OnceLock<Arc<Mutex<HashMap<String, ActiveOverlayProcess>>>> = OnceLock::new();
type CalibratedPositionsMap = Arc<Mutex<HashMap<String, (f32, f32)>>>;
static CALIBRATED_POSITIONS: OnceLock<CalibratedPositionsMap> = OnceLock::new();

fn active_map() -> &'static Arc<Mutex<HashMap<String, ActiveOverlayProcess>>> {
    ACTIVE_PROCESSES.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn calibrated_map() -> &'static Arc<Mutex<HashMap<String, (f32, f32)>>> {
    CALIBRATED_POSITIONS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

pub fn init() {
    let _ = active_map();
    let _ = calibrated_map();
}

pub fn show_overlay(overlay: &OverlayDefinition, calibration: bool, override_text: Option<String>) {
    let b64 = crate::gui::overlay_window::encode_overlay_b64(overlay);

    let mut lock = active_map().lock().unwrap();
    if let Some(mut existing) = lock.remove(&overlay.id) {
        if let Some(mut ch) = existing.child.take() {
            let _ = ch.kill();
        }
    }

    let child = if cfg!(test) {
        None
    } else {
        let exe = std::env::current_exe().unwrap_or_default();
        let mut cmd = Command::new(&exe);
        cmd.arg("--overlay-window").arg(&overlay.id);

        if calibration {
            cmd.arg("--calibration");
        }
        if let Some(txt) = override_text {
            cmd.arg("--override-text").arg(txt);
        }
        if let Some(b64_str) = b64 {
            cmd.arg("--overlay-data").arg(b64_str);
        }
        if !calibration {
            if let Some(s) = overlay.auto_hide_seconds {
                if s > 0.0 {
                    cmd.arg("--duration").arg(((s * 1000.0).round() as u32).to_string());
                }
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.spawn().ok()
    };

    lock.insert(
        overlay.id.clone(),
        ActiveOverlayProcess {
            id: overlay.id.clone(),
            child,
            close_with_target: overlay.close_with_target,
            target_process: overlay.target_process.clone().unwrap_or_default(),
            fade_out_ms: overlay.fade_out_ms,
        },
    );
}

#[cfg(windows)]
pub fn find_overlay_hwnd(id: &str, pid: Option<u32>) -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, FindWindowW, GetWindowThreadProcessId};
    use windows::core::PCWSTR;

    let title_wide: Vec<u16> = format!("Synector Overlay [{id}]\0").encode_utf16().collect();
    let found = unsafe { FindWindowW(None, PCWSTR(title_wide.as_ptr())) };
    if let Ok(h) = found {
        if !h.is_invalid() {
            return Some(h);
        }
    }

    if let Some(target_pid) = pid {
        struct SearchContext {
            pid: u32,
            hwnd: Option<HWND>,
        }
        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
            let ctx = &mut *(lparam.0 as *mut SearchContext);
            let mut win_pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut win_pid));
            if win_pid == ctx.pid {
                ctx.hwnd = Some(hwnd);
                return windows::core::BOOL(0);
            }
            windows::core::BOOL(1)
        }
        let mut ctx = SearchContext {
            pid: target_pid,
            hwnd: None,
        };
        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut _ as isize));
        }
        return ctx.hwnd;
    }
    None
}

pub fn hide_overlay(id: &str) {
    let mut lock = active_map().lock().unwrap();
    if let Some(mut proc) = lock.remove(id) {
        let fade_ms = proc.fade_out_ms.unwrap_or(0);
        let pid = proc.child.as_ref().map(|c| c.id());

        #[cfg(windows)]
        let signaled = if fade_ms > 0 {
            if let Some(hwnd) = find_overlay_hwnd(id, pid) {
                unsafe {
                    use windows::Win32::Foundation::{LPARAM, WPARAM};
                    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                true
            } else {
                false
            }
        } else {
            false
        };

        #[cfg(not(windows))]
        let signaled = false;

        if let Some(mut ch) = proc.child.take() {
            if signaled && fade_ms > 0 {
                std::thread::spawn(move || {
                    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(fade_ms as u64 + 400);
                    while std::time::Instant::now() < deadline {
                        if let Ok(Some(_)) = ch.try_wait() {
                            return;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    let _ = ch.kill();
                });
            } else {
                let _ = ch.kill();
            }
        }
    }
}

pub fn toggle_overlay(overlay: &OverlayDefinition, override_text: Option<String>) {
    if get_active_overlay_ids().contains(&overlay.id) {
        hide_overlay(&overlay.id);
    } else {
        show_overlay(overlay, false, override_text);
    }
}

pub fn flash_overlay(overlay: &OverlayDefinition, duration_ms: u32, override_text: Option<String>) {
    let b64 = crate::gui::overlay_window::encode_overlay_b64(overlay);

    let mut lock = active_map().lock().unwrap();
    if let Some(mut existing) = lock.remove(&overlay.id) {
        if let Some(mut ch) = existing.child.take() {
            let _ = ch.kill();
        }
    }

    let child = if cfg!(test) {
        None
    } else {
        let exe = std::env::current_exe().unwrap_or_default();
        let mut cmd = Command::new(&exe);
        cmd.arg("--overlay-window").arg(&overlay.id);
        cmd.arg("--duration").arg(duration_ms.to_string());

        if let Some(txt) = override_text {
            cmd.arg("--override-text").arg(txt);
        }
        if let Some(b64_str) = b64 {
            cmd.arg("--overlay-data").arg(b64_str);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.spawn().ok()
    };

    lock.insert(
        overlay.id.clone(),
        ActiveOverlayProcess {
            id: overlay.id.clone(),
            child,
            close_with_target: overlay.close_with_target,
            target_process: overlay.target_process.clone().unwrap_or_default(),
            fade_out_ms: overlay.fade_out_ms,
        },
    );
}

pub fn hide_all() {
    let to_hide: Vec<String> = {
        let lock = active_map().lock().unwrap();
        lock.keys().cloned().collect()
    };
    for id in to_hide {
        hide_overlay(&id);
    }
}

pub fn get_active_overlay_ids() -> HashSet<String> {
    let mut lock = active_map().lock().unwrap();
    let mut dead = Vec::new();

    for (id, proc) in lock.iter_mut() {
        if let Some(ch) = &mut proc.child {
            if let Ok(Some(_)) = ch.try_wait() {
                dead.push(id.clone());
            }
        }
    }

    for id in dead {
        lock.remove(&id);
    }

    lock.keys().cloned().collect()
}

pub fn get_calibrated_position(id: &str) -> Option<(f32, f32)> {
    calibrated_map().lock().ok()?.get(id).copied()
}

pub fn save_calibrated_position(id: &str, x: f32, y: f32) {
    if let Ok(mut lock) = calibrated_map().lock() {
        lock.insert(id.to_string(), (x, y));
    }
}

pub fn update_overlay_text(id: &str, text: &str) {
    let mut lock = active_map().lock().unwrap();
    if let Some(mut proc) = lock.remove(id) {
        if let Some(mut ch) = proc.child.take() {
            let _ = ch.kill();
        }
        // Relaunch with updated override text
        let config = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
        if let Some(def) = config.overlays.iter().find(|o| o.id == id) {
            let exe = std::env::current_exe().unwrap_or_default();
            let mut cmd = Command::new(&exe);
            cmd.arg("--overlay-window").arg(id).arg("--override-text").arg(text);
            if let Some(b64) = crate::gui::overlay_window::encode_overlay_b64(def) {
                cmd.arg("--overlay-data").arg(b64);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            let child = cmd.spawn().ok();
            lock.insert(
                id.to_string(),
                ActiveOverlayProcess {
                    id: id.to_string(),
                    child,
                    close_with_target: def.close_with_target,
                    target_process: def.target_process.clone().unwrap_or_default(),
                    fade_out_ms: def.fade_out_ms,
                },
            );
        }
    }
}

pub fn check_process_terminated(process_name: &str) {
    let clean_target = std::path::Path::new(process_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(process_name);

    let to_close: Vec<String> = {
        let lock = active_map().lock().unwrap();
        lock.values()
            .filter(|p| {
                if !p.close_with_target {
                    return false;
                }
                let clean_p = std::path::Path::new(&p.target_process)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&p.target_process);
                clean_p.eq_ignore_ascii_case(clean_target)
            })
            .map(|p| p.id.clone())
            .collect()
    };

    for id in to_close {
        hide_overlay(&id);
    }
}

#[cfg(windows)]
pub fn get_system_monitors() -> Vec<MonitorInfo> {
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows::Win32::Foundation::{LPARAM, RECT};
    use windows::core::BOOL;

    unsafe extern "system" fn enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _lprect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let list = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmonitor, &mut info as *mut MONITORINFOEXW as *mut _).as_bool() {
            let rect = info.monitorInfo.rcMonitor;
            let is_primary = (info.monitorInfo.dwFlags & 1) != 0;
            let w = (rect.right - rect.left).max(0) as u32;
            let h = (rect.bottom - rect.top).max(0) as u32;
            let raw_name = String::from_utf16_lossy(&info.szDevice).trim_matches('\0').to_string();
            let name = if raw_name.is_empty() {
                format!("Display {}", list.len() + 1)
            } else {
                raw_name
            };
            if w > 0 && h > 0 {
                list.push(MonitorInfo {
                    index: 0,
                    name,
                    x: rect.left,
                    y: rect.top,
                    width: w,
                    height: h,
                    is_primary,
                });
            }
        }
        BOOL(1)
    }

    let mut list: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_proc),
            LPARAM(&mut list as *mut _ as isize),
        );
    }

    if list.is_empty() {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let (sw, sh) = if w > 0 && h > 0 { (w as u32, h as u32) } else { (1920, 1080) };
        return vec![MonitorInfo {
            index: 0,
            name: "Primary Display".to_string(),
            x: 0,
            y: 0,
            width: sw,
            height: sh,
            is_primary: true,
        }];
    }

    list.sort_by(|a, b| {
        b.is_primary.cmp(&a.is_primary)
            .then_with(|| a.x.cmp(&b.x))
            .then_with(|| a.y.cmp(&b.y))
    });

    for (i, m) in list.iter_mut().enumerate() {
        m.index = i;
    }

    list
}

#[cfg(not(windows))]
pub fn get_system_monitors() -> Vec<MonitorInfo> {
    vec![MonitorInfo {
        index: 0,
        name: "Primary Display".to_string(),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        is_primary: true,
    }]
}

#[cfg(test)]
pub fn calculate_overlay_rect(
    overlay: &OverlayDefinition,
    _active_overlays: &HashMap<String, (f32, f32)>,
) -> (i32, i32, i32, i32) {
    let monitors = get_system_monitors();
    let monitor = overlay
        .monitor_index
        .and_then(|idx| monitors.get(idx))
        .or_else(|| monitors.iter().find(|m| m.is_primary))
        .unwrap_or(&monitors[0]);

    let monitor_w = monitor.width as i32;
    let monitor_h = monitor.height as i32;
    let mon_x = monitor.x;
    let mon_y = monitor.y;

    let w = overlay.rect.width as i32;
    let h = overlay.rect.height as i32;
    let rx = overlay.rect.x as i32;
    let ry = overlay.rect.y as i32;

    let (rel_x, rel_y) = match overlay.anchor {
        OverlayAnchor::ScreenCenter => {
            let cx = if rx.abs() > monitor_w / 3 { 0 } else { rx };
            let cy = if ry.abs() > monitor_h / 3 { 0 } else { ry };
            ((monitor_w - w) / 2 + cx, (monitor_h - h) / 2 + cy)
        }
        OverlayAnchor::TopLeft => (rx, ry),
        OverlayAnchor::TopRight => (monitor_w - w - rx, ry),
        OverlayAnchor::BottomLeft => (rx, monitor_h - h - ry),
        OverlayAnchor::BottomRight => (monitor_w - w - rx, monitor_h - h - ry),
        OverlayAnchor::Custom => (rx, ry),
        OverlayAnchor::WindowRelative => (rx, ry),
    };

    (mon_x + rel_x, mon_y + rel_y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_system_monitors_and_rect() {
        let monitors = get_system_monitors();
        assert!(!monitors.is_empty(), "System monitors list must not be empty");
        assert!(monitors.iter().any(|m| m.is_primary), "Must include at least one primary monitor");
        assert_eq!(monitors[0].index, 0);
        assert!(monitors[0].is_primary, "First monitor must always be primary after sorting");

        let mut def = OverlayDefinition::default_new_overlay();
        def.rect.width = 200.0;
        def.rect.height = 100.0;
        def.anchor = OverlayAnchor::TopLeft;
        def.rect.x = 10.0;
        def.rect.y = 20.0;
        def.monitor_index = Some(0);

        let active_map = HashMap::new();
        let (x0, y0, w0, h0) = calculate_overlay_rect(&def, &active_map);
        assert_eq!(w0, 200);
        assert_eq!(h0, 100);
        assert_eq!(x0, monitors[0].x + 10);
        assert_eq!(y0, monitors[0].y + 20);
    }

    #[test]
    fn test_show_overlay_stores_fade_out_and_target_process() {
        init();

        let mut def = OverlayDefinition::default_new_overlay();
        def.id = "mgr_test_ov".to_string();
        def.target_process = Some("game.exe".to_string());
        def.close_with_target = true;
        def.fade_out_ms = Some(450);

        show_overlay(&def, false, None);

        let active_ids = get_active_overlay_ids();
        assert!(active_ids.contains("mgr_test_ov"));

        {
            let lock = active_map().lock().unwrap();
            let proc = lock.get("mgr_test_ov").unwrap();
            assert_eq!(proc.target_process, "game.exe");
            assert!(proc.close_with_target);
            assert_eq!(proc.fade_out_ms, Some(450));
        }

        hide_overlay("mgr_test_ov");
        let active_ids_after = get_active_overlay_ids();
        assert!(!active_ids_after.contains("mgr_test_ov"));
    }
}
