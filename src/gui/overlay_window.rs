use std::sync::Mutex;
use std::time::{Duration, Instant};
use egui::Color32;
use crate::config::AppConfig;
use crate::gui::components::overlay_renderer;
use crate::gui::theme;
use crate::models::OverlayDefinition;

static OVERLAY_RUNTIME_STATE: Mutex<Option<(String, bool, bool, bool)>> = Mutex::new(None);
static LAST_MOVE_IPC: Mutex<Option<Instant>> = Mutex::new(None);

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
        GetWindowRect, HTCLIENT, HTTRANSPARENT, WM_EXITSIZEMOVE, WM_LBUTTONDBLCLK, WM_MOVE,
        WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDBLCLK, WM_SYSCOMMAND,
    };

    let runtime = OVERLAY_RUNTIME_STATE.lock().ok().and_then(|g| g.clone());

    match umsg {
        WM_NCCALCSIZE => {
            if wparam.0 != 0 {
                // Return 0: The entire window rectangle is the client area.
                // This completely bypasses and eliminates the 1px non-client top offset and black strip!
                return LRESULT(0);
            }
        }
        WM_NCHITTEST => {
            if let Some((_, calibration, _, click_through)) = runtime {
                if click_through && !calibration {
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
        }
        WM_MOVE => {
            if let Some((ref id, calibration, draggable, _)) = runtime {
                if (calibration || draggable) && !id.is_empty() {
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
                                id: id.clone(),
                                x: rect.left as f32,
                                y: rect.top as f32,
                            });
                        }
                    }
                }
            }
        }
        WM_EXITSIZEMOVE => {
            if let Some((ref id, calibration, draggable, _)) = runtime {
                if (calibration || draggable) && !id.is_empty() {
                    let mut rect = RECT::default();
                    if GetWindowRect(hwnd, &mut rect).is_ok() {
                        let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::UpdateOverlayPosition {
                            id: id.clone(),
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

        if let Ok(mut lock) = OVERLAY_RUNTIME_STATE.lock() {
            *lock = Some((overlay.id.clone(), calibration, overlay.draggable, overlay.click_through));
        }

        let duration = duration_ms.map(|ms| Duration::from_millis(ms as u64));

        Self {
            overlay,
            calibration,
            override_text,
            start_time: Instant::now(),
            duration,
            hw_configured: false,
        }
    }

    fn apply_win32_window_styles(&mut self) {
        #[cfg(windows)]
        unsafe {
            use windows::core::BOOL;
            use windows::Win32::Foundation::{HWND, LPARAM};
            use windows::Win32::Graphics::Dwm::{
                DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_NCRENDERING_POLICY,
            };
            use windows::Win32::System::Threading::GetCurrentThreadId;
            use windows::Win32::UI::Shell::SetWindowSubclass;
            use windows::Win32::UI::WindowsAndMessaging::{
                EnumThreadWindows, GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowDisplayAffinity,
                SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE, LWA_ALPHA, SWP_FRAMECHANGED,
                SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
                WS_BORDER, WS_CAPTION, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX,
                WS_MINIMIZEBOX, WS_POPUP, WS_THICKFRAME,
            };

            struct ThreadHwndContext {
                hwnd: Option<HWND>,
            }

            unsafe extern "system" fn enum_hwnd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let ctx = &mut *(lparam.0 as *mut ThreadHwndContext);
                let mut title_buf = [0u16; 128];
                let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut title_buf);
                if len > 0 {
                    let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                    if title.contains("ProcessHandler Overlay") {
                        ctx.hwnd = Some(hwnd);
                        return BOOL(0);
                    }
                }
                if ctx.hwnd.is_none() && windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool() {
                    ctx.hwnd = Some(hwnd);
                }
                BOOL(1)
            }

            let mut context = ThreadHwndContext { hwnd: None };
            let tid = GetCurrentThreadId();
            let _ = EnumThreadWindows(tid, Some(enum_hwnd_proc), LPARAM(&mut context as *mut _ as isize));

            if let Some(hwnd) = context.hwnd {
                // 1. Hide from Alt+Tab and set click-through if enabled
                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let mut clean_ex = ex | WS_EX_TOOLWINDOW.0 as isize;
                if self.overlay.click_through && !self.calibration {
                    clean_ex |= (WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0) as isize;
                }
                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, clean_ex);
                if self.overlay.click_through && !self.calibration {
                    let _ = SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), 255, LWA_ALPHA);
                }

                // 2. Remove standard window borders, captions, and sizing borders
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let clean_style = (style | WS_POPUP.0 as isize)
                    & !(WS_BORDER.0 as isize | WS_CAPTION.0 as isize | WS_THICKFRAME.0 as isize | WS_MAXIMIZEBOX.0 as isize | WS_MINIMIZEBOX.0 as isize);
                let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, clean_style);

                // 3. Disable DWM active accent borders (Windows 11 / 10)
                let color_none: u32 = 0xFFFFFFFE;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_BORDER_COLOR,
                    &color_none as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );

                // 4. Disable DWM non-client rendering (prevents unwanted DWM non-client shadows and margins)
                let policy = DWMNCRP_DISABLED.0 as u32;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_NCRENDERING_POLICY,
                    &policy as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );

                // 5. Install window subclass to intercept WM_NCCALCSIZE (eliminating 1px top line) and handle dragging
                let _ = SetWindowSubclass(hwnd, Some(overlay_subclass_proc), 1001, 0);

                // 6. Notify Windows that the window frame has changed so borders are stripped immediately
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

impl eframe::App for OverlayWindowApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.hw_configured {
            self.apply_win32_window_styles();
        }

        // Auto close after duration
        if let Some(dur) = self.duration {
            if self.start_time.elapsed() >= dur {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        let card_rect = ui.max_rect();

        let painter = ui.painter();

        // Card background
        overlay_renderer::render_overlay_card_bg(
            painter,
            card_rect,
            &self.overlay,
            1.0,
            self.overlay.opacity,
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
                    crate::models::OverlayElement::Text { on_click: Some(_), .. }
                    | crate::models::OverlayElement::Image { on_click: Some(_), .. }
                    | crate::models::OverlayElement::Badge { on_click: Some(_), .. }
                    | crate::models::OverlayElement::Shape { on_click: Some(_), .. }
                    | crate::models::OverlayElement::SystemTelemetry { on_click: Some(_), .. } => true,
                    _ => false,
                };
                if !is_interactive {
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

        if (self.calibration || self.overlay.draggable) && pointer_down && !is_over_interactive {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let mut render_ctx = overlay_renderer::RenderContext {
            painter,
            card_canvas_rect: card_rect,
            zoom: 1.0,
            global_opacity: self.overlay.opacity,
            is_interactive: !self.overlay.click_through || self.calibration,
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

    let abs_x = mon_x + rel_x;
    let abs_y = mon_y + rel_y;

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("ProcessHandler Overlay")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_has_shadow(false)
        .with_taskbar(false)
        .with_resizable(false)
        .with_maximize_button(false)
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
        "ProcessHandler_Overlay",
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
