use std::sync::Mutex;
use std::time::{Duration, Instant};
use egui::Color32;
use crate::config::AppConfig;
use crate::gui::components::overlay_renderer;
use crate::gui::theme;
use crate::models::OverlayDefinition;

#[derive(Clone)]
struct OverlayRuntimeState {
    id: String,
    calibration: bool,
    draggable: bool,
    click_through: bool,
    has_fade_out: bool,
}

static OVERLAY_RUNTIME_STATE: Mutex<Option<OverlayRuntimeState>> = Mutex::new(None);
static LAST_MOVE_IPC: Mutex<Option<Instant>> = Mutex::new(None);
static FADEOUT_TRIGGERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn overlay_subclass_proc(
    hwnd: windows::Win32::Foundation::HWND,
    umsg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _uid: usize,
    _ref_data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::{LRESULT, RECT};
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, GetWindowRect, HTCLIENT, HTTRANSPARENT, MA_NOACTIVATE, STYLESTRUCT, WM_CLOSE,
        WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_LBUTTONDBLCLK, WM_MOUSEACTIVATE, WM_MOVE, WM_NCACTIVATE,
        WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDBLCLK, WM_NCPAINT, WM_STYLECHANGED, WM_STYLECHANGING,
        WM_SYSCOMMAND, WS_BORDER, WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_THICKFRAME,
    };

    let runtime = OVERLAY_RUNTIME_STATE.lock().ok().and_then(|g| g.clone());

    match umsg {
        WM_STYLECHANGING => {
            let style_struct = &mut *(lparam.0 as *mut STYLESTRUCT);
            if wparam.0 as i32 == windows::Win32::UI::WindowsAndMessaging::GWL_STYLE.0 {
                style_struct.styleNew = (style_struct.styleNew | WS_POPUP.0)
                    & !(WS_BORDER.0 | WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MAXIMIZEBOX.0 | WS_MINIMIZEBOX.0);
                return LRESULT(0);
            } else if wparam.0 as i32 == windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE.0 {
                let mut ex = style_struct.styleNew
                    | WS_EX_LAYERED.0
                    | WS_EX_TOOLWINDOW.0
                    | WS_EX_NOACTIVATE.0;
                if let Some(ref rt) = runtime {
                    if rt.click_through && !rt.calibration {
                        ex |= WS_EX_TRANSPARENT.0;
                    } else {
                        ex &= !WS_EX_TRANSPARENT.0;
                    }
                }
                style_struct.styleNew = ex;
                return LRESULT(0);
            }
        }
        WM_STYLECHANGED => {
            return LRESULT(0);
        }
        WM_MOUSEACTIVATE => {
            // Allow mouse click to be processed without activating/focusing the window,
            // preventing the overlay from stealing focus from games or active apps!
            return LRESULT(MA_NOACTIVATE as isize);
        }
        WM_NCACTIVATE => {
            // Prevent Windows from drawing active/inactive title bar
            return LRESULT(1);
        }
        WM_ERASEBKGND => {
            // Tell Windows the background has been erased; prevents GDI from painting the default white COLOR_WINDOW brush
            return LRESULT(1);
        }
        WM_NCPAINT => {
            // Suppress standard Windows non-client frame/border painting
            return LRESULT(0);
        }
        WM_NCCALCSIZE => {
            if wparam.0 != 0 {
                // Return 0: The entire window rectangle is the client area.
                // This completely bypasses and eliminates the 1px non-client top offset and black strip!
                return LRESULT(0);
            }
        }
        WM_NCHITTEST => {
            if let Some(ref rt) = runtime {
                if rt.click_through && !rt.calibration {
                    return LRESULT(HTTRANSPARENT as isize);
                }
            }
            // Treat entire window as client area.
            // This prevents Windows DWM from intercepting clicks as non-client caption events,
            // allowing egui to receive all mouse clicks, hovers, and focus,
            // while preventing double-click from maximizing the window!
            return LRESULT(HTCLIENT as isize);
        }
        WM_NCLBUTTONDBLCLK | WM_LBUTTONDBLCLK => {
            // Completely disable double-click maximizing behavior on overlay
            return LRESULT(0);
        }
        WM_SYSCOMMAND => {
            // Prevent SC_MAXIMIZE (0xF030)
            if (wparam.0 & 0xFFF0) == 0xF030 {
                return LRESULT(0);
            }
            // Directly forward SC_MOVE / SC_DRAGMOVE (0xF010 / 0xF012) to DefWindowProcW,
            // completely bypassing winit's event loop so winit cannot cancel the modal move!
            if (wparam.0 & 0xFFF0) == 0xF010 {
                return DefWindowProcW(hwnd, umsg, wparam, lparam);
            }
        }
        WM_CLOSE => {
            let has_fade_out = runtime.as_ref().is_some_and(|rt| rt.has_fade_out);
            if has_fade_out && !FADEOUT_TRIGGERED.swap(true, std::sync::atomic::Ordering::SeqCst) {
                // Intercept close so that egui::App::ui can execute the smooth fade-out animation
                unsafe {
                    use windows::Win32::Graphics::Gdi::InvalidateRect;
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                return LRESULT(0);
            }
        }
        WM_MOVE => {
            if let Some(ref rt) = runtime {
                if (rt.calibration || rt.draggable) && !rt.id.is_empty() {
                    let should_send = {
                        let mut last = LAST_MOVE_IPC.lock().unwrap_or_else(|e| e.into_inner());
                        let now = Instant::now();
                        match *last {
                            Some(t) if now.duration_since(t) < Duration::from_millis(100) => false,
                            _ => {
                                *last = Some(now);
                                true
                            }
                        }
                    };
                    if should_send {
                        let mut rect = RECT::default();
                        if GetWindowRect(hwnd, &mut rect).is_ok() {
                            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::UpdateOverlayPosition {
                                id: rt.id.clone(),
                                x: rect.left as f32,
                                y: rect.top as f32,
                            });
                        }
                    }
                }
            }
        }
        WM_EXITSIZEMOVE => {
            if let Some(ref rt) = runtime {
                if (rt.calibration || rt.draggable) && !rt.id.is_empty() {
                    let mut rect = RECT::default();
                    if GetWindowRect(hwnd, &mut rect).is_ok() {
                        let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::UpdateOverlayPosition {
                            id: rt.id.clone(),
                            x: rect.left as f32,
                            y: rect.top as f32,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
}

pub struct OverlayWindowApp {
    pub overlay: OverlayDefinition,
    pub calibration: bool,
    pub override_text: Option<String>,
    pub start_time: Instant,
    pub duration: Option<Duration>,
    pub hw_configured: bool,
    pub current_opacity: f32,
    pub fade_in_duration: Option<Duration>,
    pub fade_out_duration: Option<Duration>,
    pub fade_out_start: Option<(Instant, f32)>,
    pub is_closing: bool,
    pub hwnd: Option<isize>,
    pub last_window_track: Instant,
    pub frame_count: u32,
}

impl OverlayWindowApp {
    pub fn new(
        overlay: OverlayDefinition,
        calibration: bool,
        override_text: Option<String>,
        duration_ms: Option<u32>,
        ctx: &egui::Context,
    ) -> Self {
        theme::install_fonts(ctx);
        egui_extras::install_image_loaders(ctx);

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.window_fill = Color32::TRANSPARENT;
        visuals.extreme_bg_color = Color32::TRANSPARENT;
        ctx.set_visuals(visuals);

        let fade_in_duration = if !calibration {
            overlay.fade_in_ms.filter(|&ms| ms > 0).map(|ms| Duration::from_millis(ms as u64))
        } else {
            None
        };

        let fade_out_duration = if !calibration {
            overlay.fade_out_ms.filter(|&ms| ms > 0).map(|ms| Duration::from_millis(ms as u64))
        } else {
            None
        };

        let initial_opacity = if fade_in_duration.is_some() {
            0.0
        } else {
            overlay.opacity
        };

        FADEOUT_TRIGGERED.store(false, std::sync::atomic::Ordering::SeqCst);

        if let Ok(mut lock) = OVERLAY_RUNTIME_STATE.lock() {
            *lock = Some(OverlayRuntimeState {
                id: overlay.id.clone(),
                calibration,
                draggable: overlay.draggable,
                click_through: overlay.click_through,
                has_fade_out: fade_out_duration.is_some(),
            });
        }

        let duration = duration_ms
            .map(|ms| Duration::from_millis(ms as u64))
            .or_else(|| {
                overlay
                    .auto_hide_seconds
                    .filter(|&s| s > 0.0)
                    .map(Duration::from_secs_f32)
            });

        let mut app = Self {
            overlay,
            calibration,
            override_text,
            start_time: Instant::now(),
            duration,
            hw_configured: false,
            current_opacity: initial_opacity,
            fade_in_duration,
            fade_out_duration,
            fade_out_start: None,
            is_closing: false,
            hwnd: None,
            last_window_track: Instant::now(),
            frame_count: 0,
        };

        app.apply_win32_window_styles();

        app
    }

    fn apply_win32_window_styles(&mut self) {
        #[cfg(windows)]
        unsafe {
            use windows::core::BOOL;
            use windows::Win32::Foundation::{HWND, LPARAM};
            use windows::Win32::Graphics::Dwm::{
                DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_BORDER_COLOR,
                DWMWA_NCRENDERING_POLICY, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
                DWMWCP_DONOTROUND,
            };
            use windows::Win32::System::Threading::GetCurrentThreadId;
            use windows::Win32::UI::Controls::MARGINS;
            use windows::Win32::UI::Shell::SetWindowSubclass;
            use windows::Win32::UI::WindowsAndMessaging::{
                EnumThreadWindows, GetWindowLongPtrW, SetClassLongPtrW, SetLayeredWindowAttributes,
                SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, GCLP_HBRBACKGROUND, GWL_EXSTYLE,
                GWL_STYLE, LWA_ALPHA, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
                WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WS_BORDER, WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE,
                WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_THICKFRAME,
            };

            struct ThreadHwndContext {
                target_id: String,
                hwnd: Option<HWND>,
            }

            unsafe extern "system" fn enum_hwnd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let ctx = &mut *(lparam.0 as *mut ThreadHwndContext);
                let mut title_buf = [0u16; 128];
                let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut title_buf);
                if len > 0 {
                    let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                    let expected = format!("[{}]", ctx.target_id);
                    if title.contains(&expected) || title.contains("Synector Overlay") {
                        ctx.hwnd = Some(hwnd);
                        return BOOL(0);
                    }
                }
                if ctx.hwnd.is_none() {
                    ctx.hwnd = Some(hwnd);
                }
                BOOL(1)
            }

            let mut context = ThreadHwndContext {
                target_id: self.overlay.id.clone(),
                hwnd: None,
            };
            let tid = GetCurrentThreadId();
            let _ = EnumThreadWindows(tid, Some(enum_hwnd_proc), LPARAM(&mut context as *mut _ as isize));

            if let Some(hwnd) = context.hwnd {
                self.hwnd = Some(hwnd.0 as isize);

                // 1. Clear window class background brush to prevent GDI from painting the default white COLOR_WINDOW brush
                let _ = SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, 0);

                // 2. Extend DWM frame into client area to enable seamless desktop composition transparency
                let margins = MARGINS {
                    cxLeftWidth: -1,
                    cxRightWidth: -1,
                    cyTopHeight: -1,
                    cyBottomHeight: -1,
                };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                // 3. Hide from Alt+Tab and configure Layered Window for flicker-free alpha composition
                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let mut clean_ex = ex | WS_EX_TOOLWINDOW.0 as isize | WS_EX_LAYERED.0 as isize | WS_EX_NOACTIVATE.0 as isize;
                if self.overlay.click_through && !self.calibration {
                    clean_ex |= WS_EX_TRANSPARENT.0 as isize;
                } else {
                    clean_ex &= !(WS_EX_TRANSPARENT.0 as isize);
                }
                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, clean_ex);

                // Start with window alpha = 0 (100% transparent) so no white uninitialized frame is ever visible
                let _ = SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), 0, LWA_ALPHA);

                // 4. Remove standard window borders, captions, and sizing borders
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let clean_style = (style | WS_POPUP.0 as isize)
                    & !(WS_BORDER.0 as isize | WS_CAPTION.0 as isize | WS_THICKFRAME.0 as isize | WS_MAXIMIZEBOX.0 as isize | WS_MINIMIZEBOX.0 as isize);
                let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, clean_style);

                // 5. Disable DWM active accent borders and corner rounding (Windows 11 / 10)
                let color_none: u32 = 0xFFFFFFFE;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_BORDER_COLOR,
                    &color_none as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );

                let corner_pref = DWMWCP_DONOTROUND.0 as u32;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &corner_pref as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );

                // 6. Disable DWM non-client rendering (prevents unwanted DWM non-client shadows and margins)
                let policy = DWMNCRP_DISABLED.0 as u32;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_NCRENDERING_POLICY,
                    &policy as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );

                // 7. Enable immersive dark mode so fallback composition is black rather than white
                let dark_mode = BOOL(1);
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &dark_mode as *const _ as *const _,
                    std::mem::size_of::<BOOL>() as u32,
                );

                // 8. Install window subclass to intercept WM_NCCALCSIZE (eliminating 1px top line) and handle dragging
                let _ = SetWindowSubclass(hwnd, Some(overlay_subclass_proc), 1001, 0);

                // 9. Notify Windows that the window frame has changed (keep hidden, do NOT pass SWP_SHOWWINDOW)
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                );

                // 7. Privacy screen capture affinity
                let affinity = if self.overlay.hide_from_capture {
                    WDA_EXCLUDEFROMCAPTURE
                } else {
                    WDA_NONE
                };
                let _ = SetWindowDisplayAffinity(hwnd, affinity);
                self.hw_configured = true;
            }
        }
    }
}

#[cfg(windows)]
pub fn get_target_window_rect(target_process: Option<&str>) -> Option<(i32, i32, i32, i32)> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    };

    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if !fg_hwnd.is_invalid() && IsWindowVisible(fg_hwnd).as_bool() && !IsIconic(fg_hwnd).as_bool() {
            if let Some(target) = target_process {
                let mut pid = 0u32;
                GetWindowThreadProcessId(fg_hwnd, Some(&mut pid));
                let snap = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
                if let Some(name) = snap.get(&pid) {
                    let clean_target = std::path::Path::new(target)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(target);
                    if name.eq_ignore_ascii_case(clean_target) {
                        let mut rect = RECT::default();
                        if GetWindowRect(fg_hwnd, &mut rect).is_ok() {
                            let w = rect.right - rect.left;
                            let h = rect.bottom - rect.top;
                            if w > 50 && h > 50 {
                                return Some((rect.left, rect.top, w, h));
                            }
                        }
                    }
                }
            } else {
                let mut rect = RECT::default();
                if GetWindowRect(fg_hwnd, &mut rect).is_ok() {
                    let w = rect.right - rect.left;
                    let h = rect.bottom - rect.top;
                    if w > 50 && h > 50 {
                        return Some((rect.left, rect.top, w, h));
                    }
                }
            }
        }

        if let Some(target) = target_process {
            let snap = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
            let clean_target = std::path::Path::new(target)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(target);

            let matching_pids: Vec<u32> = snap
                .iter()
                .filter(|(_, name)| name.eq_ignore_ascii_case(clean_target))
                .map(|(&pid, _)| pid)
                .collect();

            if matching_pids.is_empty() {
                return None;
            }

            struct SearchCtx<'a> {
                pids: &'a [u32],
                rect: Option<(i32, i32, i32, i32)>,
            }

            unsafe extern "system" fn enum_wnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let ctx = &mut *(lparam.0 as *mut SearchCtx);
                if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                    return BOOL(1);
                }
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if ctx.pids.contains(&pid) {
                    let mut r = RECT::default();
                    if GetWindowRect(hwnd, &mut r).is_ok() {
                        let w = r.right - r.left;
                        let h = r.bottom - r.top;
                        if w > 100 && h > 100 {
                            ctx.rect = Some((r.left, r.top, w, h));
                            return BOOL(0);
                        }
                    }
                }
                BOOL(1)
            }

            let mut search = SearchCtx {
                pids: &matching_pids,
                rect: None,
            };
            let _ = EnumWindows(Some(enum_wnd), LPARAM(&mut search as *mut _ as isize));
            return search.rect;
        }

        None
    }
}

#[inline]
fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

impl eframe::App for OverlayWindowApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.hw_configured {
            self.apply_win32_window_styles();
        }

        // Ensure WS_EX_LAYERED is never stripped by winit when visibility changes or frames render
        #[cfg(windows)]
        if let Some(hwnd_val) = self.hwnd {
            unsafe {
                use windows::Win32::Foundation::HWND;
                use windows::Win32::UI::WindowsAndMessaging::{
                    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_NOACTIVATE,
                    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
                };
                let hwnd = HWND(hwnd_val as _);
                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                if (ex & WS_EX_LAYERED.0 as isize) == 0 {
                    let mut clean_ex = ex
                        | WS_EX_LAYERED.0 as isize
                        | WS_EX_TOOLWINDOW.0 as isize
                        | WS_EX_NOACTIVATE.0 as isize;
                    if self.overlay.click_through && !self.calibration {
                        clean_ex |= WS_EX_TRANSPARENT.0 as isize;
                    } else {
                        clean_ex &= !(WS_EX_TRANSPARENT.0 as isize);
                    }
                    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, clean_ex);
                }
            }
        }

        self.frame_count += 1;
        if self.frame_count == 1 {
            // First frame: let GPU pipeline render into backbuffer while window is still 100% hidden.
            ctx.request_repaint();
        } else if self.frame_count == 2 {
            // Second frame: backbuffer is ready and presented to DWM.
            // Reveal the window flicker-free without any uninitialized white background or caption artifacts!
            #[cfg(windows)]
            if let Some(hwnd_val) = self.hwnd {
                unsafe {
                    use windows::Win32::Foundation::{COLORREF, HWND};
                    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SetLayeredWindowAttributes, SW_SHOWNOACTIVATE, LWA_ALPHA};
                    if self.fade_in_duration.is_some() {
                        self.start_time = Instant::now();
                    }
                    let initial_alpha = if self.fade_in_duration.is_some() {
                        0
                    } else {
                        (self.current_opacity * 255.0).round().clamp(0.0, 255.0) as u8
                    };
                    let _ = SetLayeredWindowAttributes(HWND(hwnd_val as _), COLORREF(0), initial_alpha, LWA_ALPHA);
                    let _ = ShowWindow(HWND(hwnd_val as _), SW_SHOWNOACTIVATE);
                }
            }
        }

        // 1. External close/fadeout triggered (via WM_CLOSE to hwnd)
        if !self.is_closing && FADEOUT_TRIGGERED.load(std::sync::atomic::Ordering::SeqCst) {
            self.is_closing = true;
            if self.fade_out_duration.is_some() {
                self.fade_out_start = Some((Instant::now(), self.current_opacity));
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        // 2. Check auto-hide / flash duration timer
        if !self.is_closing {
            if let Some(dur) = self.duration {
                if self.start_time.elapsed() >= dur {
                    self.is_closing = true;
                    if self.fade_out_duration.is_some() {
                        self.fade_out_start = Some((Instant::now(), self.current_opacity));
                        FADEOUT_TRIGGERED.store(true, std::sync::atomic::Ordering::SeqCst);
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                }
            }
        }

        // 3. Compute opacity with smooth Ken Perlin smootherstep easing
        if let Some((fo_time, start_op)) = self.fade_out_start {
            if let Some(fo_dur) = self.fade_out_duration {
                let elapsed = fo_time.elapsed();
                if elapsed < fo_dur {
                    let t = (elapsed.as_secs_f32() / fo_dur.as_secs_f32().max(0.001)).clamp(0.0, 1.0);
                    let smooth = smootherstep(t);
                    let ease = 1.0 - smooth;
                    self.current_opacity = (start_op * ease).clamp(0.0, 1.0);
                    ctx.request_repaint();
                } else {
                    self.current_opacity = 0.0;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        } else if let Some(fi_dur) = self.fade_in_duration {
            if self.frame_count >= 2 {
                let elapsed = self.start_time.elapsed();
                if elapsed < fi_dur {
                    let t = (elapsed.as_secs_f32() / fi_dur.as_secs_f32().max(0.001)).clamp(0.0, 1.0);
                    let smooth = smootherstep(t);
                    self.current_opacity = (self.overlay.opacity * smooth).clamp(0.0, 1.0);
                    ctx.request_repaint();
                } else {
                    self.current_opacity = self.overlay.opacity;
                }
            }
        } else {
            self.current_opacity = self.overlay.opacity;
        }

        // Synchronize hardware OS compositor alpha on every frame
        #[cfg(windows)]
        if self.frame_count >= 2 {
            if let Some(hwnd_val) = self.hwnd {
                unsafe {
                    use windows::Win32::Foundation::{COLORREF, HWND};
                    use windows::Win32::UI::WindowsAndMessaging::{SetLayeredWindowAttributes, LWA_ALPHA};
                    let win_alpha = (self.current_opacity * 255.0).round().clamp(0.0, 255.0) as u8;
                    let _ = SetLayeredWindowAttributes(HWND(hwnd_val as _), COLORREF(0), win_alpha, LWA_ALPHA);
                }
            }
        }

        // 4. Track WindowRelative target window
        #[cfg(windows)]
        if self.overlay.anchor == crate::models::OverlayAnchor::WindowRelative
            && !self.calibration
            && !self.overlay.draggable
            && self.last_window_track.elapsed() >= Duration::from_millis(50)
        {
            self.last_window_track = Instant::now();
            if let Some((wx, wy, ww, _)) = get_target_window_rect(self.overlay.target_process.as_deref()) {
                let target_x = (wx + ww - self.overlay.rect.width as i32 - self.overlay.rect.x as i32) as f32;
                let target_y = (wy + self.overlay.rect.y as i32) as f32;
                if let Some(hwnd_val) = self.hwnd {
                    unsafe {
                        use windows::Win32::Foundation::HWND;
                        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};
                        let _ = SetWindowPos(
                            HWND(hwnd_val as _),
                            None,
                            target_x as i32,
                            target_y as i32,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            }
        }

        let card_rect = ui.max_rect();
        let painter = ui.painter();

        #[cfg(windows)]
        let egui_render_opacity = 1.0;
        #[cfg(not(windows))]
        let egui_render_opacity = self.current_opacity;

        // Card background
        overlay_renderer::render_overlay_card_bg(
            painter,
            card_rect,
            &self.overlay,
            1.0,
            egui_render_opacity,
        );

        let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
        let pointer_clicked = ctx.input(|i| i.pointer.primary_clicked());
        let pointer_down = ctx.input(|i| i.pointer.primary_down());

        let is_over_interactive = if let Some(pos) = pointer_pos {
            self.overlay.elements.iter().any(|elem| {
                let is_interactive = match elem {
                    crate::models::OverlayElement::InteractiveControl { .. } => true,
                    crate::models::OverlayElement::MediaWidget { .. } => true,
                    crate::models::OverlayElement::ClockTimer { mode, .. } => {
                        *mode == crate::models::TimerMode::Stopwatch || *mode == crate::models::TimerMode::Countdown
                    }
                    _ => false,
                };
                let has_click_action = matches!(
                    elem,
                    crate::models::OverlayElement::Text { on_click: Some(_), .. }
                        | crate::models::OverlayElement::Image { on_click: Some(_), .. }
                        | crate::models::OverlayElement::Shape { on_click: Some(_), .. }
                        | crate::models::OverlayElement::SystemTelemetry { on_click: Some(_), .. }
                );
                if !is_interactive && !has_click_action {
                    return false;
                }
                let (raw_w, raw_h) = overlay_renderer::get_element_size(elem);
                let center = card_rect.min + egui::vec2(elem.x() + raw_w * 0.5, elem.y() + raw_h * 0.5);
                let angle_rad = elem.rotation().to_radians();
                let local_pos = if angle_rad.abs() > 0.001 {
                    center + overlay_renderer::rotate_vec(pos - center, -angle_rad)
                } else {
                    pos
                };
                let elem_rect = egui::Rect::from_center_size(center, egui::vec2(raw_w, raw_h));
                elem_rect.expand(4.0).contains(local_pos)
            })
        } else {
            false
        };

        if is_over_interactive {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let is_draggable_mode = (self.calibration || self.overlay.draggable) && !self.is_closing;
        if is_draggable_mode && !is_over_interactive {
            if pointer_down {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            } else {
                ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }

            if pointer_down {
                #[cfg(windows)]
                if let Some(hwnd_val) = self.hwnd {
                    unsafe {
                        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
                        use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
                        use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_SYSCOMMAND};
                        let _ = ReleaseCapture();
                        let _ = SendMessageW(HWND(hwnd_val as _), WM_SYSCOMMAND, Some(WPARAM(0xF012)), Some(LPARAM(0)));
                    }
                }
            }
        }

        let mut render_ctx = overlay_renderer::RenderContext {
            painter,
            card_canvas_rect: card_rect,
            zoom: 1.0,
            global_opacity: egui_render_opacity,
            is_interactive: (!self.overlay.click_through || self.calibration) && !self.is_closing,
            pointer_pos,
            pointer_clicked,
            pointer_down,
            override_text: self.override_text.as_deref(),
            target_process: self.overlay.target_process.as_deref(),
        };

        for elem in &mut self.overlay.elements {
            overlay_renderer::render_element(&mut render_ctx, elem);
        }

        // Request repaint every 30ms for smooth 60fps animations (equalizer, key visualizer glow, graphs)
        ctx.request_repaint_after(Duration::from_millis(30));
    }
}

pub fn decode_overlay_b64(b64: &str) -> Option<OverlayDefinition> {
    use base64::Engine;
    use std::io::Read;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    let mut decoder = flate2::read::DeflateDecoder::new(&bytes[..]);
    let mut json_str = String::new();
    decoder.read_to_string(&mut json_str).ok()?;
    serde_json::from_str(&json_str).ok()
}

pub fn encode_overlay_b64(def: &OverlayDefinition) -> Option<String> {
    use base64::Engine;
    use std::io::Write;
    let json = serde_json::to_string(def).ok()?;
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(json.as_bytes()).ok()?;
    let compressed = encoder.finish().ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(compressed))
}

pub fn run_overlay_window(
    overlay_id: String,
    calibration: bool,
    override_text: Option<String>,
    duration_ms: Option<u32>,
    overlay_data_b64: Option<String>,
) -> eframe::Result<()> {
    // 1. Resolve overlay definition
    let overlay: OverlayDefinition = if let Some(b64) = overlay_data_b64 {
        decode_overlay_b64(&b64).unwrap_or_else(OverlayDefinition::default_new_overlay)
    } else {
        let config = AppConfig::load_from_file(AppConfig::config_path());
        config
            .overlays
            .into_iter()
            .find(|o| o.id == overlay_id)
            .unwrap_or_else(OverlayDefinition::default_new_overlay)
    };

    // 2. Compute screen position
    let monitors = crate::daemon::overlay_manager::get_system_monitors();
    let target_mon = if let Some(idx) = overlay.monitor_index {
        monitors.iter().find(|m| m.index == idx).or_else(|| monitors.iter().find(|m| m.is_primary))
    } else {
        monitors.iter().find(|m| m.is_primary)
    };

    let (screen_w, screen_h, mon_x, mon_y) = if let Some(m) = target_mon {
        (m.width as f32, m.height as f32, m.x as f32, m.y as f32)
    } else {
        (1920.0, 1080.0, 0.0, 0.0)
    };

    let (rel_x, rel_y) = crate::gui::views::overlay_modal::compute_overlay_screen_pos_with_anchor(
        overlay.anchor,
        &overlay,
        screen_w,
        screen_h,
    );

    let (abs_x, abs_y) = if overlay.anchor == crate::models::OverlayAnchor::WindowRelative && !calibration {
        #[cfg(windows)]
        {
            if let Some((wx, wy, ww, _)) = get_target_window_rect(overlay.target_process.as_deref()) {
                (
                    (wx + ww - overlay.rect.width as i32 - overlay.rect.x as i32) as f32,
                    (wy + overlay.rect.y as i32) as f32,
                )
            } else {
                (mon_x + rel_x, mon_y + rel_y)
            }
        }
        #[cfg(not(windows))]
        (mon_x + rel_x, mon_y + rel_y)
    } else {
        (mon_x + rel_x, mon_y + rel_y)
    };

    let title_str = format!("Synector Overlay [{}]", overlay.id);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(title_str)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_has_shadow(false)
        .with_taskbar(false)
        .with_resizable(false)
        .with_maximize_button(false)
        .with_visible(false)
        .with_inner_size([overlay.rect.width, overlay.rect.height])
        .with_position([abs_x, abs_y]);

    if overlay.click_through && !calibration {
        viewport = viewport.with_mouse_passthrough(true);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Synector_Overlay",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(OverlayWindowApp::new(
                overlay,
                calibration,
                override_text,
                duration_ms,
                &cc.egui_ctx,
            )))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_OVERLAY_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_overlay_window_fade_in_and_fade_out_initialization() {
        let _lock = TEST_OVERLAY_LOCK.lock().unwrap();
        let ctx = egui::Context::default();

        // 1. Overlay with fade in
        let mut def_fade = OverlayDefinition::default_new_overlay();
        def_fade.opacity = 0.85;
        def_fade.fade_in_ms = Some(300);
        def_fade.fade_out_ms = Some(400);

        let app_fade = OverlayWindowApp::new(def_fade, false, None, None, &ctx);
        assert_eq!(app_fade.current_opacity, 0.0, "Overlay with fade_in must start at 0.0 opacity");
        assert_eq!(app_fade.fade_in_duration, Some(Duration::from_millis(300)));
        assert_eq!(app_fade.fade_out_duration, Some(Duration::from_millis(400)));

        // 2. Overlay without fade in
        let mut def_instant = OverlayDefinition::default_new_overlay();
        def_instant.opacity = 0.75;
        def_instant.fade_in_ms = None;
        def_instant.fade_out_ms = None;

        let app_instant = OverlayWindowApp::new(def_instant, false, None, None, &ctx);
        assert_eq!(app_instant.current_opacity, 0.75, "Overlay without fade_in must start at defined opacity");
        assert_eq!(app_instant.fade_in_duration, None);
        assert_eq!(app_instant.fade_out_duration, None);

        // 3. Calibration mode must bypass fade in
        let mut def_calib = OverlayDefinition::default_new_overlay();
        def_calib.opacity = 0.9;
        def_calib.fade_in_ms = Some(500);

        let app_calib = OverlayWindowApp::new(def_calib, true, None, None, &ctx);
        assert_eq!(app_calib.current_opacity, 0.9, "Calibration mode must bypass fade in");
        assert_eq!(app_calib.fade_in_duration, None);
    }

    #[test]
    fn test_smootherstep_bounds_and_symmetry() {
        assert_eq!(smootherstep(0.0), 0.0);
        assert_eq!(smootherstep(0.5), 0.5);
        assert_eq!(smootherstep(1.0), 1.0);
        assert_eq!(smootherstep(-0.5), 0.0);
        assert_eq!(smootherstep(1.5), 1.0);
        // Test symmetry: f(x) + f(1 - x) == 1.0
        let s1 = smootherstep(0.25);
        let s2 = smootherstep(0.75);
        assert!((s1 + s2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_overlay_window_draggable_runtime_state() {
        let _lock = TEST_OVERLAY_LOCK.lock().unwrap();
        let ctx = egui::Context::default();
        let mut def = OverlayDefinition::default_new_overlay();
        def.draggable = true;
        def.click_through = false;

        let _app = OverlayWindowApp::new(def, false, None, None, &ctx);
        let rt = OVERLAY_RUNTIME_STATE.lock().unwrap().clone().expect("Runtime state set");
        assert!(rt.draggable, "Runtime state must reflect overlay.draggable");
        assert!(!rt.click_through, "Click-through must be false when draggable is enabled");
    }

    #[test]
    fn test_overlay_window_auto_hide_resolution() {
        let _lock = TEST_OVERLAY_LOCK.lock().unwrap();
        let ctx = egui::Context::default();

        // Priority 1: CLI duration_ms
        let mut def1 = OverlayDefinition::default_new_overlay();
        def1.auto_hide_seconds = Some(10.0);
        let app1 = OverlayWindowApp::new(def1, false, None, Some(2500), &ctx);
        assert_eq!(app1.duration, Some(Duration::from_millis(2500)));

        // Priority 2: overlay.auto_hide_seconds fallback
        let mut def2 = OverlayDefinition::default_new_overlay();
        def2.auto_hide_seconds = Some(4.5);
        let app2 = OverlayWindowApp::new(def2, false, None, None, &ctx);
        assert_eq!(app2.duration, Some(Duration::from_millis(4500)));

        // Neither: indefinite
        let mut def3 = OverlayDefinition::default_new_overlay();
        def3.auto_hide_seconds = None;
        let app3 = OverlayWindowApp::new(def3, false, None, None, &ctx);
        assert_eq!(app3.duration, None);
    }

    #[test]
    fn test_overlay_window_encode_decode_b64() {
        let mut def = OverlayDefinition::default_new_overlay();
        def.id = "b64_test".to_string();
        def.name = "Test B64 Overlay".to_string();
        def.fade_in_ms = Some(200);
        def.fade_out_ms = Some(350);

        let encoded = encode_overlay_b64(&def).expect("Encoding failed");
        let decoded = decode_overlay_b64(&encoded).expect("Decoding failed");

        assert_eq!(decoded.id, "b64_test");
        assert_eq!(decoded.name, "Test B64 Overlay");
        assert_eq!(decoded.fade_in_ms, Some(200));
        assert_eq!(decoded.fade_out_ms, Some(350));
    }
}
