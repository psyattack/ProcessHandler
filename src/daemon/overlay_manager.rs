use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use crate::models::{
    CrosshairStyle, OverlayAnchor, OverlayDefinition, OverlayElement, ShapeKind,
};

#[cfg(windows)]
use windows::core::{w, PCWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    AddFontMemResourceEx, BeginPaint, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, GetDeviceCaps, GetStockObject, InvalidateRect,
    LineTo, LOGPIXELSY, MoveToEx, Rectangle, RoundRect, SelectObject, SetBkMode, SetBrushOrgEx, SetStretchBltMode, SetTextColor, StretchDIBits,
    GetWorldTransform, SetGraphicsMode, SetWorldTransform, GM_ADVANCED, XFORM, HDC, DT_CALCRECT,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DT_CENTER, DT_LEFT,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FONT_CHARSET, FONT_CLIP_PRECISION,
    FONT_OUTPUT_PRECISION, FONT_QUALITY, FW_BOLD, FW_NORMAL, COLORONCOLOR, NULL_BRUSH, PAINTSTRUCT,
    PS_SOLID, SRCCOPY, TRANSPARENT,
};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
    GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId, PeekMessageW,
    PostThreadMessageW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos,
    ShowWindow, TranslateMessage, HTCAPTION, LWA_ALPHA, LWA_COLORKEY, MSG, PM_REMOVE, SM_CXSCREEN, SM_CYSCREEN,
    SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW, SW_SHOWNOACTIVATE,
    SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    WM_APP, WM_DESTROY, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOVE, WM_NCHITTEST, WM_PAINT,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

const WM_OVERLAY_CMD: u32 = WM_APP + 50;
const COLOR_KEY: u32 = 0x00030303; // RGB(3, 3, 3) transparent key

#[derive(Debug, Clone)]
pub enum OverlayCommand {
    Show {
        overlay: OverlayDefinition,
        calibration: bool,
        override_text: Option<String>,
        duration_ms: Option<u32>,
    },
    Hide(String),
    Toggle {
        overlay: OverlayDefinition,
        override_text: Option<String>,
    },
    Flash {
        overlay: OverlayDefinition,
        duration_ms: u32,
        override_text: Option<String>,
    },
    UpdateText {
        id: String,
        text: String,
    },
    #[allow(dead_code)]
    HideAll,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FadeState {
    None,
    FadeIn {
        start_time: Instant,
        duration: Duration,
        start_alpha: u8,
        target_alpha: u8,
    },
    FadeOut {
        start_time: Instant,
        duration: Duration,
        start_alpha: u8,
    },
}

struct ActiveOverlay {
    definition: OverlayDefinition,
    hwnd: isize,
    calibration: bool,
    override_text: Option<String>,
    hide_at: Option<Instant>,
    fade_state: FadeState,
    current_alpha: u8,
}

type CalibratedCoordsMap = Arc<Mutex<HashMap<String, (f32, f32)>>>;
type ActiveOverlaysSet = Arc<Mutex<HashSet<String>>>;

static OVERLAY_SENDER: OnceLock<std::sync::mpsc::Sender<OverlayCommand>> = OnceLock::new();
static CALIBRATED_COORDS: OnceLock<CalibratedCoordsMap> = OnceLock::new();
static ACTIVE_OVERLAYS: OnceLock<ActiveOverlaysSet> = OnceLock::new();

fn get_coords_map() -> &'static CalibratedCoordsMap {
    CALIBRATED_COORDS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn get_active_overlays_set() -> &'static ActiveOverlaysSet {
    ACTIVE_OVERLAYS.get_or_init(|| Arc::new(Mutex::new(HashSet::new())))
}

pub fn get_active_overlay_ids() -> HashSet<String> {
    get_active_overlays_set().lock().map(|set| set.clone()).unwrap_or_default()
}

#[allow(dead_code)]
pub fn is_overlay_active(id: &str) -> bool {
    get_active_overlays_set().lock().map(|set| set.contains(id)).unwrap_or(false)
}

#[cfg(windows)]
fn sync_active_overlay_ids(active_overlays: &HashMap<String, ActiveOverlay>) {
    if let Ok(mut set) = get_active_overlays_set().lock() {
        set.clear();
        for (id, item) in active_overlays {
            if !item.calibration && !matches!(item.fade_state, FadeState::FadeOut { .. }) {
                set.insert(id.clone());
            }
        }
    }
}

#[cfg(windows)]
fn install_bundled_fonts() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        unsafe {
            const INTER: &[u8] = include_bytes!("../../assets/fonts/Inter-Variable.ttf");
            const ARCHIVO: &[u8] = include_bytes!("../../assets/fonts/Archivo-Variable.ttf");
            const GEIST_MONO: &[u8] = include_bytes!("../../assets/fonts/GeistMono-Variable.ttf");

            let count = 0u32;
            let _ = AddFontMemResourceEx(INTER.as_ptr() as *mut _, INTER.len() as u32, None, &count);
            let _ = AddFontMemResourceEx(ARCHIVO.as_ptr() as *mut _, ARCHIVO.len() as u32, None, &count);
            let _ = AddFontMemResourceEx(GEIST_MONO.as_ptr() as *mut _, GEIST_MONO.len() as u32, None, &count);
        }
    });
}

/// Initialize the background overlay thread
pub fn init() {
    #[cfg(windows)]
    install_bundled_fonts();

    OVERLAY_SENDER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("ph-overlay-runner".to_string())
            .spawn(move || {
                run_overlay_event_loop(rx);
            })
            .expect("Failed to spawn overlay runner thread");
        tx
    });
}

/// Public API to show an overlay
pub fn show_overlay(
    overlay: &OverlayDefinition,
    calibration_mode: bool,
    override_text: Option<String>,
) {
    init();
    if let Some(tx) = OVERLAY_SENDER.get() {
        let _ = tx.send(OverlayCommand::Show {
            overlay: overlay.clone(),
            calibration: calibration_mode,
            override_text,
            duration_ms: None,
        });
        wake_overlay_thread();
    }
}

/// Public API to hide an overlay
pub fn hide_overlay(id: &str) {
    init();
    if let Some(tx) = OVERLAY_SENDER.get() {
        let _ = tx.send(OverlayCommand::Hide(id.to_string()));
        wake_overlay_thread();
    }
}

/// Public API to toggle an overlay
pub fn toggle_overlay(overlay: &OverlayDefinition, override_text: Option<String>) {
    init();
    if let Some(tx) = OVERLAY_SENDER.get() {
        let _ = tx.send(OverlayCommand::Toggle {
            overlay: overlay.clone(),
            override_text,
        });
        wake_overlay_thread();
    }
}

/// Public API to flash an overlay for a specific duration
pub fn flash_overlay(
    overlay: &OverlayDefinition,
    duration_ms: u32,
    override_text: Option<String>,
) {
    init();
    if let Some(tx) = OVERLAY_SENDER.get() {
        let _ = tx.send(OverlayCommand::Flash {
            overlay: overlay.clone(),
            duration_ms,
            override_text,
        });
        wake_overlay_thread();
    }
}

/// Public API to update dynamic text of an overlay
pub fn update_overlay_text(id: &str, text: &str) {
    init();
    if let Some(tx) = OVERLAY_SENDER.get() {
        let _ = tx.send(OverlayCommand::UpdateText {
            id: id.to_string(),
            text: text.to_string(),
        });
        wake_overlay_thread();
    }
}

/// Public API to hide all overlays
#[allow(dead_code)]
pub fn hide_all() {
    init();
    if let Some(tx) = OVERLAY_SENDER.get() {
        let _ = tx.send(OverlayCommand::HideAll);
        wake_overlay_thread();
    }
}

/// Retrieve calibrated coordinates for an overlay after user moved it on screen
pub fn get_calibrated_position(id: &str) -> Option<(f32, f32)> {
    get_coords_map().lock().ok()?.get(id).copied()
}

static OVERLAY_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn wake_overlay_thread() {
    #[cfg(windows)]
    {
        let tid = OVERLAY_THREAD_ID.load(std::sync::atomic::Ordering::Relaxed);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_OVERLAY_CMD, WPARAM(0), LPARAM(0));
            }
        }
    }
}

#[cfg(not(windows))]
fn run_overlay_event_loop(_rx: std::sync::mpsc::Receiver<OverlayCommand>) {}

#[cfg(windows)]
fn run_overlay_event_loop(rx: std::sync::mpsc::Receiver<OverlayCommand>) {
    let tid = unsafe { GetCurrentThreadId() };
    OVERLAY_THREAD_ID.store(tid, std::sync::atomic::Ordering::Relaxed);

    // Register Win32 Overlay Window Class
    let class_name = w!("Overlay_Window");
    let hinst = match unsafe { GetModuleHandleW(None) } {
        Ok(h) => h,
        Err(_) => return,
    };

    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(overlay_wnd_proc),
        hInstance: hinst.into(),
        lpszClassName: class_name,
        hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(std::ptr::null_mut()),
        ..Default::default()
    };
    unsafe {
        RegisterClassW(&wnd_class);
    }

    let mut active_overlays: HashMap<String, ActiveOverlay> = HashMap::new();
    let mut last_tick = Instant::now();

    loop {
        // 1. Process all pending commands
        let mut had_commands = false;
        while let Ok(cmd) = rx.try_recv() {
            handle_overlay_command(cmd, &mut active_overlays, hinst);
            had_commands = true;
        }
        if had_commands {
            sync_active_overlay_ids(&active_overlays);
        }

        // 2. Win32 message pump with PeekMessage/GetMessage
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                active_overlays.clear();
                sync_active_overlay_ids(&active_overlays);
                return;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
        }

        // 3. Process fade animations (60 FPS tick)
        let now = Instant::now();
        let mut completed_fadeouts = Vec::new();
        for (id, item) in active_overlays.iter_mut() {
            match item.fade_state {
                FadeState::FadeIn {
                    start_time,
                    duration,
                    start_alpha,
                    target_alpha,
                } => {
                    let elapsed = now.saturating_duration_since(start_time);
                    if elapsed >= duration {
                        item.current_alpha = target_alpha;
                        item.fade_state = FadeState::None;
                    } else {
                        let t = (elapsed.as_secs_f32() / duration.as_secs_f32().max(0.001)).clamp(0.0, 1.0);
                        let alpha = (start_alpha as f32 + (target_alpha as f32 - start_alpha as f32) * t).round() as u8;
                        item.current_alpha = alpha;
                    }
                    unsafe {
                        let _ = SetLayeredWindowAttributes(
                            HWND(item.hwnd as _),
                            COLORREF(COLOR_KEY),
                            item.current_alpha,
                            LWA_COLORKEY | LWA_ALPHA,
                        );
                    }
                }
                FadeState::FadeOut {
                    start_time,
                    duration,
                    start_alpha,
                } => {
                    let elapsed = now.saturating_duration_since(start_time);
                    if elapsed >= duration {
                        item.current_alpha = 0;
                        completed_fadeouts.push(id.clone());
                    } else {
                        let t = (elapsed.as_secs_f32() / duration.as_secs_f32().max(0.001)).clamp(0.0, 1.0);
                        let alpha = (start_alpha as f32 * (1.0 - t)).round() as u8;
                        item.current_alpha = alpha;
                        unsafe {
                            let _ = SetLayeredWindowAttributes(
                                HWND(item.hwnd as _),
                                COLORREF(COLOR_KEY),
                                item.current_alpha,
                                LWA_COLORKEY | LWA_ALPHA,
                            );
                        }
                    }
                }
                FadeState::None => {}
            }
        }

        if !completed_fadeouts.is_empty() {
            for id in completed_fadeouts {
                if let Some(item) = active_overlays.remove(&id) {
                    unsafe {
                        let _ = DestroyWindow(HWND(item.hwnd as _));
                    }
                }
            }
            sync_active_overlay_ids(&active_overlays);
        }

        // 4. Periodic housekeeping (auto-hide timers, window-relative updates)
        if last_tick.elapsed() >= Duration::from_millis(50) {
            last_tick = Instant::now();

            // Check auto-hide
            let now = Instant::now();
            let mut to_hide = Vec::new();
            for (id, item) in &active_overlays {
                if let Some(hide_time) = item.hide_at {
                    if now >= hide_time {
                        to_hide.push(id.clone());
                    }
                }
            }
            let mut auto_hid = false;
            for id in to_hide {
                if let Some(item) = active_overlays.get_mut(&id) {
                    if item.definition.fade_out_ms.unwrap_or(0) > 0 && !item.calibration {
                        if !matches!(item.fade_state, FadeState::FadeOut { .. }) {
                            let fade_out_ms = item.definition.fade_out_ms.unwrap();
                            let target_alpha = (item.definition.opacity.clamp(0.05, 1.0) * 255.0) as u8;
                            let ratio = if target_alpha > 0 {
                                (item.current_alpha as f32 / target_alpha as f32).clamp(0.0, 1.0)
                            } else {
                                1.0
                            };
                            let duration = Duration::from_millis(((fade_out_ms as f32) * ratio).round().max(1.0) as u64);
                            item.fade_state = FadeState::FadeOut {
                                start_time: Instant::now(),
                                duration,
                                start_alpha: item.current_alpha,
                            };
                            item.hide_at = None;
                            auto_hid = true;
                        }
                    } else if let Some(removed) = active_overlays.remove(&id) {
                        unsafe {
                            let _ = DestroyWindow(HWND(removed.hwnd as _));
                        }
                        auto_hid = true;
                    }
                }
            }
            if auto_hid {
                sync_active_overlay_ids(&active_overlays);
            }

            // Window-relative tracking: if target process window moves, sync overlay position
            update_window_relative_overlays(&active_overlays);
        }

        std::thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(windows)]
fn handle_overlay_command(
    cmd: OverlayCommand,
    active_overlays: &mut HashMap<String, ActiveOverlay>,
    hinst: windows::Win32::Foundation::HMODULE,
) {
    match cmd {
        OverlayCommand::Show {
            overlay,
            calibration,
            override_text,
            duration_ms,
        } => {
            let (x, y, w, h) = calculate_overlay_rect(&overlay, active_overlays);
            if calibration {
                if let Ok(mut coords) = get_coords_map().lock() {
                    coords.insert(overlay.id.clone(), (x as f32, y as f32));
                }
            }

            let target_alpha = (overlay.opacity.clamp(0.05, 1.0) * 255.0) as u8;

            // If existing window exists and calibration mode is the same, update in-place with SetWindowPos
            if let Some(mut existing) = active_overlays.remove(&overlay.id) {
                let hwnd = HWND(existing.hwnd as _);
                let ptr = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    ) as *mut OverlayWndData
                };
                if !ptr.is_null() && existing.calibration == calibration {
                    unsafe {
                        (*ptr).overlay = overlay.clone();
                        (*ptr).override_text = override_text.clone();
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            x,
                            y,
                            w,
                            h,
                            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                        );
                        let _ = InvalidateRect(Some(hwnd), None, true);
                    }
                    existing.definition = overlay.clone();
                    existing.override_text = override_text;

                    // If existing overlay was fading out, smoothly reverse to fade in!
                    if matches!(existing.fade_state, FadeState::FadeOut { .. }) {
                        let fade_in_ms = overlay.fade_in_ms.unwrap_or(0);
                        if fade_in_ms > 0 && existing.current_alpha < target_alpha {
                            let ratio = ((target_alpha - existing.current_alpha) as f32 / target_alpha.max(1) as f32).clamp(0.0, 1.0);
                            let duration = Duration::from_millis(((fade_in_ms as f32) * ratio).round().max(1.0) as u64);
                            existing.fade_state = FadeState::FadeIn {
                                start_time: Instant::now(),
                                duration,
                                start_alpha: existing.current_alpha,
                                target_alpha,
                            };
                        } else {
                            existing.fade_state = FadeState::None;
                            existing.current_alpha = target_alpha;
                            unsafe {
                                let _ = SetLayeredWindowAttributes(
                                    hwnd,
                                    COLORREF(COLOR_KEY),
                                    target_alpha,
                                    LWA_COLORKEY | LWA_ALPHA,
                                );
                            }
                        }
                    }

                    existing.hide_at = duration_ms
                        .map(|d| Instant::now() + Duration::from_millis(d as u64))
                        .or_else(|| {
                            overlay
                                .auto_hide_seconds
                                .map(|s| Instant::now() + Duration::from_secs_f32(s))
                        });

                    active_overlays.insert(overlay.id.clone(), existing);
                    return;
                } else {
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                }
            }

            if let Some(hwnd) = create_overlay_window(&overlay, calibration, override_text.clone(), hinst, active_overlays) {
                let hide_at = duration_ms
                    .map(|d| Instant::now() + Duration::from_millis(d as u64))
                    .or_else(|| {
                        overlay
                            .auto_hide_seconds
                            .map(|s| Instant::now() + Duration::from_secs_f32(s))
                    });

                let fade_in_ms = overlay.fade_in_ms.unwrap_or(0);
                let (fade_state, current_alpha) = if !calibration && fade_in_ms > 0 {
                    (
                        FadeState::FadeIn {
                            start_time: Instant::now(),
                            duration: Duration::from_millis(fade_in_ms as u64),
                            start_alpha: 0,
                            target_alpha,
                        },
                        0,
                    )
                } else {
                    (FadeState::None, target_alpha)
                };

                let active = ActiveOverlay {
                    definition: overlay.clone(),
                    hwnd: hwnd.0 as isize,
                    calibration,
                    override_text,
                    hide_at,
                    fade_state,
                    current_alpha,
                };
                active_overlays.insert(overlay.id.clone(), active);

                unsafe {
                    let show_cmd = if calibration { SW_SHOW } else { SW_SHOWNOACTIVATE };
                    let _ = ShowWindow(hwnd, show_cmd);
                    let _ = InvalidateRect(Some(hwnd), None, true);
                }
            }
        }

        OverlayCommand::Hide(id) => {
            if let Ok(mut coords) = get_coords_map().lock() {
                coords.remove(&id);
            }
            if let Some(item) = active_overlays.get_mut(&id) {
                if item.definition.fade_out_ms.unwrap_or(0) > 0 && !item.calibration {
                    if !matches!(item.fade_state, FadeState::FadeOut { .. }) {
                        let fade_out_ms = item.definition.fade_out_ms.unwrap();
                        let target_alpha = (item.definition.opacity.clamp(0.05, 1.0) * 255.0) as u8;
                        let ratio = if target_alpha > 0 {
                            (item.current_alpha as f32 / target_alpha as f32).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        let duration = Duration::from_millis(((fade_out_ms as f32) * ratio).round().max(1.0) as u64);
                        item.fade_state = FadeState::FadeOut {
                            start_time: Instant::now(),
                            duration,
                            start_alpha: item.current_alpha,
                        };
                        item.hide_at = None;
                    }
                } else if let Some(removed) = active_overlays.remove(&id) {
                    unsafe {
                        let _ = DestroyWindow(HWND(removed.hwnd as _));
                    }
                }
            }
        }

        OverlayCommand::Toggle { overlay, override_text } => {
            if let Some(item) = active_overlays.get_mut(&overlay.id) {
                if matches!(item.fade_state, FadeState::FadeOut { .. }) {
                    // It is currently fading out! Smoothly reverse to FadeIn!
                    let target_alpha = (overlay.opacity.clamp(0.05, 1.0) * 255.0) as u8;
                    let fade_in_ms = overlay.fade_in_ms.unwrap_or(0);
                    if fade_in_ms > 0 && item.current_alpha < target_alpha {
                        let ratio = ((target_alpha - item.current_alpha) as f32 / target_alpha.max(1) as f32).clamp(0.0, 1.0);
                        let duration = Duration::from_millis(((fade_in_ms as f32) * ratio).round().max(1.0) as u64);
                        item.fade_state = FadeState::FadeIn {
                            start_time: Instant::now(),
                            duration,
                            start_alpha: item.current_alpha,
                            target_alpha,
                        };
                    } else {
                        item.fade_state = FadeState::None;
                        item.current_alpha = target_alpha;
                        unsafe {
                            let _ = SetLayeredWindowAttributes(
                                HWND(item.hwnd as _),
                                COLORREF(COLOR_KEY),
                                target_alpha,
                                LWA_COLORKEY | LWA_ALPHA,
                            );
                        }
                    }
                    item.definition = overlay;
                    if override_text.is_some() {
                        item.override_text = override_text;
                    }
                } else if overlay.fade_out_ms.unwrap_or(0) > 0 && !item.calibration {
                    let fade_out_ms = overlay.fade_out_ms.unwrap();
                    let target_alpha = (item.definition.opacity.clamp(0.05, 1.0) * 255.0) as u8;
                    let ratio = if target_alpha > 0 {
                        (item.current_alpha as f32 / target_alpha as f32).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let duration = Duration::from_millis(((fade_out_ms as f32) * ratio).round().max(1.0) as u64);
                    item.fade_state = FadeState::FadeOut {
                        start_time: Instant::now(),
                        duration,
                        start_alpha: item.current_alpha,
                    };
                    item.hide_at = None;
                } else if let Some(removed) = active_overlays.remove(&overlay.id) {
                    unsafe {
                        let _ = DestroyWindow(HWND(removed.hwnd as _));
                    }
                }
            } else {
                handle_overlay_command(
                    OverlayCommand::Show {
                        overlay,
                        calibration: false,
                        override_text,
                        duration_ms: None,
                    },
                    active_overlays,
                    hinst,
                );
            }
        }

        OverlayCommand::Flash {
            overlay,
            duration_ms,
            override_text,
        } => {
            handle_overlay_command(
                OverlayCommand::Show {
                    overlay,
                    calibration: false,
                    override_text,
                    duration_ms: Some(duration_ms),
                },
                active_overlays,
                hinst,
            );
        }

        OverlayCommand::UpdateText { id, text } => {
            if let Some(item) = active_overlays.get_mut(&id) {
                item.override_text = Some(text.clone());
                unsafe {
                    let ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                        HWND(item.hwnd as _),
                        windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    ) as *mut OverlayWndData;
                    if !ptr.is_null() {
                        (*ptr).override_text = Some(text);
                    }
                    let _ = InvalidateRect(Some(HWND(item.hwnd as _)), None, true);
                }
            }
        }

        OverlayCommand::HideAll => {
            if let Ok(mut coords) = get_coords_map().lock() {
                coords.clear();
            }
            for (_, item) in active_overlays.drain() {
                unsafe {
                    let _ = DestroyWindow(HWND(item.hwnd as _));
                }
            }
        }
    }
}

#[cfg(windows)]
fn create_overlay_window(
    overlay: &OverlayDefinition,
    calibration: bool,
    override_text: Option<String>,
    hinst: windows::Win32::Foundation::HMODULE,
    active_overlays: &HashMap<String, ActiveOverlay>,
) -> Option<HWND> {
    unsafe {
        let (x, y, w, h) = calculate_overlay_rect(overlay, active_overlays);

        // Extended window styles:
        // WS_EX_LAYERED: Enables per-pixel / color-key transparency
        // WS_EX_TOPMOST: Stays on top of games / full-screen borderless windows
        // WS_EX_TOOLWINDOW: Hidden from Alt-Tab task switcher
        // WS_EX_NOACTIVATE: Does not steal focus from games
        // WS_EX_TRANSPARENT: Mouse clicks pass straight through to game (omitted in calibration mode)
        let mut ex_style = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
        if !calibration {
            ex_style |= WS_EX_NOACTIVATE;
        }
        if overlay.click_through && !calibration {
            ex_style |= WS_EX_TRANSPARENT;
        }

        let class_name = w!("Overlay_Window");
        let title_wide: Vec<u16> = format!("Overlay_{}", overlay.id)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let hwnd = CreateWindowExW(
            ex_style,
            class_name,
            PCWSTR(title_wide.as_ptr()),
            WS_POPUP,
            x,
            y,
            w,
            h,
            None,
            None,
            Some(hinst.into()),
            None,
        ).ok()?;

        // Configure layered window:
        let target_alpha = (overlay.opacity.clamp(0.05, 1.0) * 255.0) as u8;
        let initial_alpha = if !calibration && overlay.fade_in_ms.unwrap_or(0) > 0 {
            0
        } else {
            target_alpha
        };
        let _ = SetLayeredWindowAttributes(
            hwnd,
            COLORREF(COLOR_KEY),
            initial_alpha,
            LWA_COLORKEY | LWA_ALPHA,
        );

        // Screen capture exclusion (e.g. bypass OBS, Discord, etc.)
        let affinity = if overlay.hide_from_capture {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };
        let _ = SetWindowDisplayAffinity(hwnd, affinity);

        // Store boxed context pointer in GWLP_USERDATA for WndProc
        let boxed_data = Box::new(OverlayWndData {
            overlay: overlay.clone(),
            calibration,
            override_text,
        });
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            Box::into_raw(boxed_data) as isize,
        );

        Some(hwnd)
    }
}

struct OverlayWndData {
    overlay: OverlayDefinition,
    calibration: bool,
    override_text: Option<String>,
}

#[cfg(windows)]
fn handle_element_click_action(action: &crate::models::OverlayClickAction, overlay_id: &str) {
    match action {
        crate::models::OverlayClickAction::CloseOverlay => {
            hide_overlay(overlay_id);
        }
        crate::models::OverlayClickAction::TriggerRule { rule_id } => {
            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::TestRunRule {
                id: rule_id.clone(),
                rule: None,
            });
        }
        crate::models::OverlayClickAction::ToggleRule { rule_id } => {
            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ToggleRule { id: rule_id.clone(), enabled: true });
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
    ) as *mut OverlayWndData;

    match msg {
        WM_NCHITTEST => {
            if !ptr.is_null() && ((*ptr).calibration || ((*ptr).overlay.draggable && !(*ptr).overlay.click_through)) {
                // In calibration mode or draggable mode, let dragging anywhere move the window!
                return LRESULT(HTCAPTION as isize);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_LBUTTONDOWN => {
            if !ptr.is_null() && ((*ptr).calibration || ((*ptr).overlay.draggable && !(*ptr).overlay.click_through)) {
                unsafe {
                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                    let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN,
                        Some(WPARAM(HTCAPTION as usize)),
                        Some(LPARAM(0)),
                    );
                }
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_LBUTTONUP => {
            if !ptr.is_null() && !(*ptr).overlay.click_through && !(*ptr).calibration {
                let click_x = (lparam.0 & 0xFFFF) as i16 as f32;
                let click_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

                for elem in (*ptr).overlay.elements.iter().rev() {
                    let (ex, ey, ew, eh, action) = match elem {
                        OverlayElement::Text { x, y, font_size, text, width, height, on_click, .. } => {
                            let est_w = width.unwrap_or_else(|| (text.len() as f32 * (font_size * 0.6).max(8.0) + 12.0).max(30.0));
                            let est_h = height.unwrap_or_else(|| (font_size + 6.0).max(18.0));
                            (*x, *y, est_w, est_h, on_click.as_ref())
                        }
                        OverlayElement::Badge { x, y, text, on_click, .. } => {
                            let b_w = text.len() as f32 * 7.5 + 18.0;
                            (*x, *y, b_w, 22.0, on_click.as_ref())
                        }
                        OverlayElement::Image { x, y, width, height, on_click, .. } => {
                            (*x, *y, *width, *height, on_click.as_ref())
                        }
                        OverlayElement::Shape { x, y, width, height, on_click, .. } => {
                            (*x, *y, *width, *height, on_click.as_ref())
                        }
                        _ => (0.0, 0.0, 0.0, 0.0, None),
                    };

                    if let Some(act) = action {
                        if click_x >= ex && click_x <= ex + ew && click_y >= ey && click_y <= ey + eh {
                            handle_element_click_action(act, &(*ptr).overlay.id);
                            break;
                        }
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_EXITSIZEMOVE | WM_MOVE => {
            if !ptr.is_null() && ((*ptr).calibration || (*ptr).overlay.draggable) {
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    let mut coords = get_coords_map().lock().unwrap();
                    coords.insert((*ptr).overlay.id.clone(), (rect.left as f32, rect.top as f32));
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_ERASEBKGND => LRESULT(1),

        WM_PAINT => {
            if !ptr.is_null() {
                paint_overlay(hwnd, &*ptr);
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    0,
                );
            }
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(windows)]
unsafe fn apply_rotation_transform(hdc: HDC, center_x: f32, center_y: f32, angle_deg: f32) -> Option<XFORM> {
    if angle_deg.abs() < 0.01 {
        return None;
    }
    let mut old_xform = XFORM::default();
    let _ = GetWorldTransform(hdc, &mut old_xform);
    let _ = SetGraphicsMode(hdc, GM_ADVANCED);

    let rad = angle_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();
    let cx = center_x;
    let cy = center_y;

    let rot_xform = XFORM {
        eM11: cos,
        eM12: sin,
        eM21: -sin,
        eM22: cos,
        eDx: cx - cx * cos + cy * sin,
        eDy: cy - cx * sin - cy * cos,
    };

    let _ = SetWorldTransform(hdc, &rot_xform);
    Some(old_xform)
}

#[cfg(windows)]
unsafe fn restore_rotation_transform(hdc: HDC, old_xform: Option<XFORM>) {
    if let Some(xform) = old_xform {
        let _ = SetWorldTransform(hdc, &xform);
    }
}

#[cfg(windows)]
fn safe_colorref(r: u8, g: u8, b: u8) -> COLORREF {
    let r = if r == 3 && g == 3 && b == 3 { 4 } else { r };
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

#[cfg(windows)]
fn paint_overlay(hwnd: HWND, data: &OverlayWndData) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.is_invalid() {
            return;
        }

        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let client_rect = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };

        // 1. Fill entire window background with transparent color key
        let bg_fill = COLORREF(COLOR_KEY);
        let key_brush = CreateSolidBrush(bg_fill);
        FillRect(hdc, &client_rect, key_brush);
        let _ = DeleteObject(key_brush.into());

        // 2. Draw styled overlay background card (if not completely transparent)
        let bg_col = data.overlay.style.bg_color;
        if bg_col[3] > 0 {
            let card_brush = CreateSolidBrush(safe_colorref(bg_col[0], bg_col[1], bg_col[2]));
            let border_col = data.overlay.style.border_color;
            let border_pen = if border_col[3] > 0 && data.overlay.style.border_width > 0.0 {
                CreatePen(
                    PS_SOLID,
                    data.overlay.style.border_width as i32,
                    safe_colorref(border_col[0], border_col[1], border_col[2]),
                )
            } else {
                CreatePen(PS_SOLID, 0, COLORREF(COLOR_KEY))
            };

            let old_brush = SelectObject(hdc, card_brush.into());
            let old_pen = SelectObject(hdc, border_pen.into());

            let cr = (data.overlay.style.corner_radius as i32).max(0);
            let _ = RoundRect(hdc, 0, 0, width, height, cr * 2, cr * 2);

            SelectObject(hdc, old_brush);
            SelectObject(hdc, old_pen);
            let _ = DeleteObject(card_brush.into());
            let _ = DeleteObject(border_pen.into());
        }

        // 3. Draw each element
        let ctx_map = HashMap::new();
        for element in &data.overlay.elements {
            match element {
                OverlayElement::Crosshair {
                    style,
                    size,
                    color,
                    center_dot,
                    thickness,
                    gap,
                    x,
                    y,
                    rotation,
                    ..
                } => {
                    let cx = if *x > 0.0 { *x as i32 } else { width / 2 };
                    let cy = if *y > 0.0 { *y as i32 } else { height / 2 };
                    let old_xform = apply_rotation_transform(hdc, cx as f32, cy as f32, *rotation);

                    let pen = CreatePen(
                        PS_SOLID,
                        (*thickness as i32).max(1),
                        safe_colorref(color[0], color[1], color[2]),
                    );
                    let old_pen = SelectObject(hdc, pen.into());

                    let g = *gap as i32;
                    let s = *size as i32;

                    match style {
                        CrosshairStyle::Cross => {
                            // Left & Right
                            let _ = MoveToEx(hdc, cx - g - s, cy, None);
                            let _ = LineTo(hdc, cx - g, cy);
                            let _ = MoveToEx(hdc, cx + g, cy, None);
                            let _ = LineTo(hdc, cx + g + s, cy);
                            // Top & Bottom
                            let _ = MoveToEx(hdc, cx, cy - g - s, None);
                            let _ = LineTo(hdc, cx, cy - g);
                            let _ = MoveToEx(hdc, cx, cy + g, None);
                            let _ = LineTo(hdc, cx, cy + g + s);
                        }
                        CrosshairStyle::TShaped => {
                            let _ = MoveToEx(hdc, cx - g - s, cy, None);
                            let _ = LineTo(hdc, cx - g, cy);
                            let _ = MoveToEx(hdc, cx + g, cy, None);
                            let _ = LineTo(hdc, cx + g + s, cy);
                            let _ = MoveToEx(hdc, cx, cy + g, None);
                            let _ = LineTo(hdc, cx, cy + g + s);
                        }
                        CrosshairStyle::Circle => {
                            let r = s / 2;
                            let _ = Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
                        }
                        CrosshairStyle::Dot => {}
                    }

                    if *center_dot {
                        let brush = CreateSolidBrush(safe_colorref(color[0], color[1], color[2]));
                        let old_br = SelectObject(hdc, brush.into());
                        let dr = ((*thickness as i32) / 2).max(1);
                        let _ = Ellipse(hdc, cx - dr, cy - dr, cx + dr + 1, cy + dr + 1);
                        SelectObject(hdc, old_br);
                        let _ = DeleteObject(brush.into());
                    }

                    SelectObject(hdc, old_pen);
                    let _ = DeleteObject(pen.into());
                    restore_rotation_transform(hdc, old_xform);
                }

                OverlayElement::Text {
                    text,
                    font_size,
                    color,
                    x,
                    y,
                    bold,
                    font_family,
                    rotation,
                    ..
                } => {
                    let raw_text = data.override_text.as_ref().unwrap_or(text);
                    let display_text = crate::daemon::actions::ActionRunner::interpolate_tokens(raw_text, &ctx_map);
                    let mut wide_text: Vec<u16> = display_text.encode_utf16().collect();

                    let weight = if *bold { FW_BOLD } else { FW_NORMAL };
                    let font_name = match font_family.to_lowercase().as_str() {
                        "inter" => "Inter",
                        "archivo" | "archivo display" => "Archivo",
                        "geist mono" | "geistmono" | "mono" => "Geist Mono",
                        "consolas" => "Consolas",
                        "impact" => "Impact",
                        "arial" => "Arial",
                        "tahoma" => "Tahoma",
                        "verdana" => "Verdana",
                        "comic sans ms" | "comic sans" => "Comic Sans MS",
                        "trebuchet ms" | "trebuchet" => "Trebuchet MS",
                        "georgia" => "Georgia",
                        "courier new" | "courier" => "Courier New",
                        _ => font_family.as_str(),
                    };
                    let wide_family: Vec<u16> = font_name.encode_utf16().chain(std::iter::once(0)).collect();
                    let eff_font_size = *font_size;
                    let dpi_y = GetDeviceCaps(Some(hdc), LOGPIXELSY);
                    let font_h = if dpi_y > 0 {
                        -(((eff_font_size as i32) * dpi_y + 48) / 96)
                    } else {
                        -(eff_font_size as i32)
                    };
                    let font = CreateFontW(
                        font_h,
                        0,
                        0,
                        0,
                        weight.0 as i32,
                        0,
                        0,
                        0,
                        FONT_CHARSET(1),
                        FONT_OUTPUT_PRECISION(0),
                        FONT_CLIP_PRECISION(0),
                        FONT_QUALITY(5),
                        0,
                        PCWSTR(wide_family.as_ptr()),
                    );
                    let old_font = SelectObject(hdc, font.into());

                    SetBkMode(hdc, TRANSPARENT);
                    SetTextColor(
                        hdc,
                        safe_colorref(color[0], color[1], color[2]),
                    );

                    let mut calc_rect = RECT {
                        left: *x as i32,
                        top: *y as i32,
                        right: width,
                        bottom: height,
                    };
                    let _ = DrawTextW(
                        hdc,
                        &mut wide_text,
                        &mut calc_rect,
                        DT_LEFT | DT_NOPREFIX | DT_CALCRECT,
                    );

                    let cx = (calc_rect.left + calc_rect.right) as f32 / 2.0;
                    let cy = (calc_rect.top + calc_rect.bottom) as f32 / 2.0;
                    let old_xform = apply_rotation_transform(hdc, cx, cy, *rotation);

                    let mut text_rect = calc_rect;
                    let _ = DrawTextW(
                        hdc,
                        &mut wide_text,
                        &mut text_rect,
                        DT_LEFT | DT_NOPREFIX,
                    );

                    SelectObject(hdc, old_font);
                    let _ = DeleteObject(font.into());
                    restore_rotation_transform(hdc, old_xform);
                }

                OverlayElement::Badge {
                    text,
                    color,
                    bg_color,
                    x,
                    y,
                    rotation,
                    ..
                } => {
                    let raw_text = data.override_text.as_ref().unwrap_or(text);
                    let display_text = crate::daemon::actions::ActionRunner::interpolate_tokens(raw_text, &ctx_map);
                    let bx = *x as i32;
                    let by = *y as i32;
                    let bw = (display_text.len() as i32 * 8 + 18).clamp(60, 360);
                    let bh = 22;

                    let cx = bx as f32 + bw as f32 / 2.0;
                    let cy = by as f32 + bh as f32 / 2.0;
                    let old_xform = apply_rotation_transform(hdc, cx, cy, *rotation);

                    let brush = CreateSolidBrush(safe_colorref(bg_color[0], bg_color[1], bg_color[2]));
                    let old_br = SelectObject(hdc, brush.into());
                    let pen = CreatePen(PS_SOLID, 1, safe_colorref(color[0], color[1], color[2]));
                    let old_pen = SelectObject(hdc, pen.into());

                    let _ = RoundRect(hdc, bx, by, bx + bw, by + bh, 6, 6);

                    SelectObject(hdc, old_br);
                    SelectObject(hdc, old_pen);
                    let _ = DeleteObject(brush.into());
                    let _ = DeleteObject(pen.into());

                    // Badge text
                    let font = CreateFontW(
                        -12,
                        0,
                        0,
                        0,
                        FW_BOLD.0 as i32,
                        0,
                        0,
                        0,
                        FONT_CHARSET(1),
                        FONT_OUTPUT_PRECISION(4),
                        FONT_CLIP_PRECISION(0),
                        FONT_QUALITY(5),
                        0,
                        w!("Segoe UI"),
                    );
                    let old_font = SelectObject(hdc, font.into());
                    SetBkMode(hdc, TRANSPARENT);
                    SetTextColor(hdc, safe_colorref(color[0], color[1], color[2]));

                    let mut wide_badge: Vec<u16> = display_text.encode_utf16().collect();
                    let mut b_rect = RECT {
                        left: bx,
                        top: by,
                        right: bx + bw,
                        bottom: by + bh,
                    };
                    let _ = DrawTextW(
                        hdc,
                        &mut wide_badge,
                        &mut b_rect,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                    );

                    SelectObject(hdc, old_font);
                    let _ = DeleteObject(font.into());
                    restore_rotation_transform(hdc, old_xform);
                }

                OverlayElement::ProgressBar {
                    progress,
                    fill_color,
                    bg_color,
                    x,
                    y,
                    width: bar_w,
                    height: bar_h,
                    rotation,
                    ..
                } => {
                    let px = *x as i32;
                    let py = *y as i32;
                    let bw = *bar_w as i32;
                    let bh = *bar_h as i32;

                    let cx = px as f32 + bw as f32 / 2.0;
                    let cy = py as f32 + bh as f32 / 2.0;
                    let old_xform = apply_rotation_transform(hdc, cx, cy, *rotation);

                    // Bar Background
                    let bg_br = CreateSolidBrush(safe_colorref(bg_color[0], bg_color[1], bg_color[2]));
                    let bar_rect = RECT { left: px, top: py, right: px + bw, bottom: py + bh };
                    FillRect(hdc, &bar_rect, bg_br);
                    let _ = DeleteObject(bg_br.into());

                    // Bar Fill
                    let fill_w = ((bw as f32 * progress.clamp(0.0, 1.0)) as i32).clamp(0, bw);
                    if fill_w > 0 {
                        let fill_br = CreateSolidBrush(safe_colorref(fill_color[0], fill_color[1], fill_color[2]));
                        let fill_rect = RECT { left: px, top: py, right: px + fill_w, bottom: py + bh };
                        FillRect(hdc, &fill_rect, fill_br);
                        let _ = DeleteObject(fill_br.into());
                    }
                    restore_rotation_transform(hdc, old_xform);
                }

                OverlayElement::Image {
                    path,
                    x,
                    y,
                    width: img_w,
                    height: img_h,
                    opacity,
                    rotation,
                    ..
                } => {
                    let cx = *x + *img_w / 2.0;
                    let cy = *y + *img_h / 2.0;
                    let old_xform = apply_rotation_transform(hdc, cx, cy, *rotation);

                    if !path.is_empty() && std::path::Path::new(path).exists() {
                        if let Ok(img) = image::open(path) {
                            let rgba = img.to_rgba8();
                            let (src_w, src_h) = rgba.dimensions();
                            let dest_x = *x as i32;
                            let dest_y = *y as i32;
                            let dest_w = (*img_w as i32).max(1);
                            let dest_h = (*img_h as i32).max(1);

                            // Smoothly resize using pure-Rust filter to avoid GDI HALFTONE dithering artifacts
                            let (resized_rgba, final_w, final_h) = if src_w == dest_w as u32 && src_h == dest_h as u32 {
                                (std::borrow::Cow::Borrowed(&rgba), src_w, src_h)
                            } else {
                                let resized = image::imageops::resize(
                                    &rgba,
                                    dest_w as u32,
                                    dest_h as u32,
                                    image::imageops::FilterType::Triangle,
                                );
                                (std::borrow::Cow::Owned(resized), dest_w as u32, dest_h as u32)
                            };

                            let mut bmi = BITMAPINFO::default();
                            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                            bmi.bmiHeader.biWidth = final_w as i32;
                            bmi.bmiHeader.biHeight = -(final_h as i32); // Top-down DIB
                            bmi.bmiHeader.biPlanes = 1;
                            bmi.bmiHeader.biBitCount = 32;
                            bmi.bmiHeader.biCompression = BI_RGB.0;

                            let card_bg = data.overlay.style.bg_color;
                            let has_card_bg = card_bg[3] > 0;
                            let card_r = card_bg[0] as f32;
                            let card_g = card_bg[1] as f32;
                            let card_b = card_bg[2] as f32;

                            let elem_opacity = opacity.clamp(0.0, 1.0);
                            let mut bgra = Vec::with_capacity((final_w * final_h * 4) as usize);

                            #[allow(clippy::chunks_exact_to_as_chunks)]
                            for chunk in resized_rgba.chunks_exact(4) {
                                let alpha = ((chunk[3] as f32 / 255.0) * elem_opacity).clamp(0.0, 1.0);

                                if alpha < 0.02 {
                                    // Transparent pixel
                                    if has_card_bg {
                                        let mut r = card_bg[0];
                                        let g = card_bg[1];
                                        let b = card_bg[2];
                                        if r == 3 && g == 3 && b == 3 {
                                            r = 4;
                                        }
                                        bgra.push(b);
                                        bgra.push(g);
                                        bgra.push(r);
                                        bgra.push(255);
                                    } else {
                                        // Transparent HUD card: exact COLOR_KEY so desktop shows through
                                        bgra.push(3); // B
                                        bgra.push(3); // G
                                        bgra.push(3); // R
                                        bgra.push(255);
                                    }
                                } else {
                                    // Visible pixel
                                    let (mut r, g, b) = if has_card_bg {
                                        let r_val = ((chunk[0] as f32) * alpha + card_r * (1.0 - alpha)).round().clamp(0.0, 255.0) as u8;
                                        let g_val = ((chunk[1] as f32) * alpha + card_g * (1.0 - alpha)).round().clamp(0.0, 255.0) as u8;
                                        let b_val = ((chunk[2] as f32) * alpha + card_b * (1.0 - alpha)).round().clamp(0.0, 255.0) as u8;
                                        (r_val, g_val, b_val)
                                    } else if alpha >= 0.98 {
                                        (chunk[0], chunk[1], chunk[2])
                                    } else {
                                        let r_val = ((chunk[0] as f32) * alpha).round().clamp(0.0, 255.0) as u8;
                                        let g_val = ((chunk[1] as f32) * alpha).round().clamp(0.0, 255.0) as u8;
                                        let b_val = ((chunk[2] as f32) * alpha).round().clamp(0.0, 255.0) as u8;
                                        (r_val, g_val, b_val)
                                    };

                                    // Safeguard: Visible pixel must never match COLOR_KEY (3, 3, 3)
                                    if r == 3 && g == 3 && b == 3 {
                                        r = 4;
                                    }

                                    bgra.push(b);
                                    bgra.push(g);
                                    bgra.push(r);
                                    bgra.push(255);
                                }
                            }

                            let _ = SetStretchBltMode(hdc, COLORONCOLOR);
                            let _ = SetBrushOrgEx(hdc, 0, 0, None);

                            let _ = StretchDIBits(
                                hdc,
                                dest_x,
                                dest_y,
                                dest_w,
                                dest_h,
                                0,
                                0,
                                final_w as i32,
                                final_h as i32,
                                Some(bgra.as_ptr() as *const _),
                                &bmi,
                                DIB_RGB_COLORS,
                                SRCCOPY,
                            );
                        }
                    }
                    restore_rotation_transform(hdc, old_xform);
                }

                OverlayElement::Shape {
                    shape_type,
                    x,
                    y,
                    width: elem_w,
                    height: elem_h,
                    thickness,
                    color,
                    filled,
                    fill_color,
                    corner_radius,
                    rotation,
                    ..
                } => {
                    let cx = *x + *elem_w / 2.0;
                    let cy = *y + *elem_h / 2.0;
                    let old_xform = apply_rotation_transform(hdc, cx, cy, *rotation);

                    let stroke_c = safe_colorref(color[0], color[1], color[2]);
                    let pen_w = (*thickness).max(1.0).round() as i32;
                    let pen = CreatePen(PS_SOLID, pen_w, stroke_c);
                    let old_pen = SelectObject(hdc, pen.into());

                    let is_fillable = *shape_type == ShapeKind::Rectangle || *shape_type == ShapeKind::Circle;
                    let (brush, created_solid) = if *filled && is_fillable {
                        let fill_c = safe_colorref(fill_color[0], fill_color[1], fill_color[2]);
                        (CreateSolidBrush(fill_c).into(), true)
                    } else {
                        (GetStockObject(NULL_BRUSH), false)
                    };
                    let old_brush = SelectObject(hdc, brush);

                    let left = *x as i32;
                    let top = *y as i32;
                    let right = (*x + *elem_w) as i32;
                    let bottom = (*y + *elem_h) as i32;

                    match shape_type {
                        ShapeKind::Rectangle => {
                            if *corner_radius > 0.0 {
                                let cr = ((*corner_radius * 2.0).round() as i32).max(2);
                                let _ = RoundRect(hdc, left, top, right, bottom, cr, cr);
                            } else {
                                let _ = Rectangle(hdc, left, top, right, bottom);
                            }
                        }
                        ShapeKind::Circle => {
                            let _ = Ellipse(hdc, left, top, right, bottom);
                        }
                        ShapeKind::Line => {
                            let mid_y = top + (bottom - top) / 2;
                            let _ = MoveToEx(hdc, left, mid_y, None);
                            let _ = LineTo(hdc, right, mid_y);
                        }
                        ShapeKind::Arrow => {
                            let mid_y = top + (bottom - top) / 2;
                            let _ = MoveToEx(hdc, left, mid_y, None);
                            let _ = LineTo(hdc, right, mid_y);
                            let head_sz = ((*elem_h * 0.4).clamp(6.0, 24.0)).round() as i32;
                            let _ = MoveToEx(hdc, right - head_sz, mid_y - head_sz, None);
                            let _ = LineTo(hdc, right, mid_y);
                            let _ = LineTo(hdc, right - head_sz, mid_y + head_sz);
                        }
                    }

                    SelectObject(hdc, old_pen);
                    let _ = DeleteObject(pen.into());
                    SelectObject(hdc, old_brush);
                    if created_solid {
                        let _ = DeleteObject(brush);
                    }
                    restore_rotation_transform(hdc, old_xform);
                }
            }
        }

        let _ = EndPaint(hwnd, &ps);
    }
}

#[cfg(windows)]
fn calculate_overlay_rect(
    overlay: &OverlayDefinition,
    active_overlays: &HashMap<String, ActiveOverlay>,
) -> (i32, i32, i32, i32) {
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let w = overlay.rect.width as i32;
    let h = overlay.rect.height as i32;
    let rx = overlay.rect.x as i32;
    let ry = overlay.rect.y as i32;

    // Auto-stack multiple overlays sharing the same anchor
    let same_anchor_offset_y: i32 = active_overlays
        .values()
        .filter(|a| a.definition.id != overlay.id && a.definition.anchor == overlay.anchor && !a.calibration)
        .map(|a| a.definition.rect.height as i32 + 8)
        .sum();

    let (x, y) = match overlay.anchor {
        OverlayAnchor::ScreenCenter => {
            let cx = if rx.abs() > screen_w / 3 { 0 } else { rx };
            let cy = if ry.abs() > screen_h / 3 { 0 } else { ry };
            ((screen_w - w) / 2 + cx, (screen_h - h) / 2 + cy + same_anchor_offset_y)
        }
        OverlayAnchor::TopLeft => {
            let cx = if rx > screen_w / 3 { 16 } else { rx };
            let cy = if ry > screen_h / 3 { 16 } else { ry };
            (cx, cy + same_anchor_offset_y)
        }
        OverlayAnchor::TopRight => {
            let cx = if rx > screen_w / 3 { 16 } else { rx };
            let cy = if ry > screen_h / 3 { 16 } else { ry };
            (screen_w - w - cx, cy + same_anchor_offset_y)
        }
        OverlayAnchor::BottomLeft => {
            let cx = if rx > screen_w / 3 { 16 } else { rx };
            let cy = if ry > screen_h / 3 { 16 } else { ry };
            (cx, (screen_h - h - cy - same_anchor_offset_y).max(0))
        }
        OverlayAnchor::BottomRight => {
            let cx = if rx > screen_w / 3 { 16 } else { rx };
            let cy = if ry > screen_h / 3 { 16 } else { ry };
            (screen_w - w - cx, (screen_h - h - cy - same_anchor_offset_y).max(0))
        }
        OverlayAnchor::Custom => (rx, ry),
        OverlayAnchor::WindowRelative => {
            // Initial position based on top-right of foreground window if available
            if let Some((wx, wy, ww, _)) = get_target_window_rect(overlay.target_process.as_deref()) {
                (wx + ww - w - rx, wy + ry + same_anchor_offset_y)
            } else {
                (rx, ry + same_anchor_offset_y)
            }
        }
    };

    (x, y, w, h)
}

#[cfg(windows)]
fn get_target_window_rect(target_process: Option<&str>) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }

        if let Some(target) = target_process {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let snap = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
            let name = snap.get(&pid)?;
            if !name.eq_ignore_ascii_case(target) {
                return None;
            }
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Some((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top))
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn update_window_relative_overlays(active_overlays: &HashMap<String, ActiveOverlay>) {
    for item in active_overlays.values() {
        if item.definition.anchor == OverlayAnchor::WindowRelative && !item.calibration {
            if let Some((wx, wy, ww, _)) = get_target_window_rect(item.definition.target_process.as_deref()) {
                let w = item.definition.rect.width as i32;
                let target_x = wx + ww - w - item.definition.rect.x as i32;
                let target_y = wy + item.definition.rect.y as i32;
                unsafe {
                    let _ = SetWindowPos(
                        HWND(item.hwnd as _),
                        None,
                        target_x,
                        target_y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_font_creation_and_resolution() {
        install_bundled_fonts();
        unsafe {
            use windows::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, GetTextFaceW};
            let hdc = CreateCompatibleDC(None);
            assert!(!hdc.is_invalid());

            let fonts_to_test = ["Inter", "Archivo", "Geist Mono", "Segoe UI", "Consolas", "Impact", "Arial"];
            for &font_name in &fonts_to_test {
                for prec in [0u8, 4u8] {
                    let wide_family: Vec<u16> = font_name.encode_utf16().chain(std::iter::once(0)).collect();
                    let font = CreateFontW(
                        -16,
                        0, 0, 0,
                        FW_NORMAL.0 as i32,
                        0, 0, 0,
                        FONT_CHARSET(1),
                        FONT_OUTPUT_PRECISION(prec),
                        FONT_CLIP_PRECISION(0),
                        FONT_QUALITY(5),
                        0,
                        PCWSTR(wide_family.as_ptr()),
                    );
                    assert!(!font.is_invalid(), "CreateFontW failed for {font_name}");
                    let old_font = SelectObject(hdc, font.into());

                    let mut face_buf = [0u16; 64];
                    let len = GetTextFaceW(hdc, Some(&mut face_buf));
                    let face_str = String::from_utf16_lossy(&face_buf[..len as usize]).trim_matches('\0').to_string();
                    println!("Requested: '{font_name}' (prec {prec}) -> Resolved: '{face_str}'");

                    SelectObject(hdc, old_font);
                    let _ = DeleteObject(font.into());
                }
            }
            let _ = DeleteDC(hdc);
        }
    }

    #[test]
    fn test_safe_colorref_prevents_colorkey_puncture() {
        // Transparent colorkey is RGB(3, 3, 3) = 0x00030303
        let key = COLORREF(COLOR_KEY);

        // Safe colorref on (3, 3, 3) must nudge R from 3 to 4, ensuring it never matches COLOR_KEY
        let nudged = safe_colorref(3, 3, 3);
        assert_ne!(nudged, key, "RGB(3, 3, 3) must not equal transparent colorkey");
        assert_eq!(nudged, COLORREF(0x00030304));

        // Normal black RGB(0, 0, 0) must stay intact
        let black = safe_colorref(0, 0, 0);
        assert_ne!(black, key);
        assert_eq!(black, COLORREF(0));

        // Other shades should remain untouched
        let white = safe_colorref(255, 255, 255);
        assert_eq!(white, COLORREF(0x00ffffff));
    }
}
