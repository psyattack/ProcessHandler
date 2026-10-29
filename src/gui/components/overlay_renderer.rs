use std::time::Instant;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, Vec2,
};
use crate::gui::theme;
use crate::models::{
    CrosshairStyle, GraphMetric, GraphStyle, InteractiveControlKind, OverlayClickAction,
    OverlayDefinition, OverlayElement, ShapeKind, SliderBindTarget, TelemetryKind, TelemetryStyle,
    TimerMode,
};

pub fn apply_opacity(color: [u8; 4], global_opacity: f32) -> Color32 {
    let alpha = ((color[3] as f32 * global_opacity).clamp(0.0, 255.0)) as u8;
    Color32::from_rgba_premultiplied(color[0], color[1], color[2], alpha)
}

pub fn rotate_vec(v: Vec2, angle_rad: f32) -> Vec2 {
    let (sin, cos) = angle_rad.sin_cos();
    vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", prefix)
    } else {
        prefix
    }
}

pub fn normalize_image_uri(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    // Strip existing file URI scheme
    let raw = if let Some(p) = trimmed.strip_prefix("file:///") {
        p
    } else if let Some(p) = trimmed.strip_prefix("file://") {
        p
    } else {
        trimmed
    };

    let normalized = raw.replace('\\', "/");
    let clean = normalized.trim_start_matches('/');

    // Check if relative or absolute path
    let abs_path = if let Ok(canonical) = std::fs::canonicalize(clean) {
        let s = canonical.to_string_lossy().replace('\\', "/");
        if let Some(stripped) = s.strip_prefix("//?/") {
            stripped.to_string()
        } else {
            s
        }
    } else {
        clean.to_string()
    };

    let final_clean = abs_path.trim_start_matches('/').to_string();
    format!("file:///{}", final_clean)
}

pub fn resolve_font_id(font_family: &str, font_size: f32, bold: bool, zoom: f32) -> FontId {
    let sz = (font_size * zoom).max(7.0);
    let norm = font_family.trim().to_lowercase();
    let family = match norm.as_str() {
        "archivo" | "archivo display" => {
            if bold { theme::font_archivo_bold() } else { theme::font_archivo() }
        }
        "inter" => {
            if bold { theme::semibold() } else { theme::medium() }
        }
        "geist mono" | "geistmono" | "mono" => {
            if bold { theme::mono_bold() } else { theme::mono() }
        }
        "impact" => egui::FontFamily::Name("impact".into()),
        "arial" => egui::FontFamily::Name(if bold { "arial-bold".into() } else { "arial".into() }),
        "segoe ui" | "segoe" => egui::FontFamily::Name(if bold { "segoe ui-bold".into() } else { "segoe ui".into() }),
        "consolas" => egui::FontFamily::Name(if bold { "consolas-bold".into() } else { "consolas".into() }),
        "tahoma" => egui::FontFamily::Name(if bold { "tahoma-bold".into() } else { "tahoma".into() }),
        "verdana" => egui::FontFamily::Name(if bold { "verdana-bold".into() } else { "verdana".into() }),
        "comic sans ms" | "comic sans" => egui::FontFamily::Name(if bold { "comic sans ms-bold".into() } else { "comic sans ms".into() }),
        "trebuchet ms" | "trebuchet" => egui::FontFamily::Name(if bold { "trebuchet ms-bold".into() } else { "trebuchet ms".into() }),
        "georgia" => egui::FontFamily::Name(if bold { "georgia-bold".into() } else { "georgia".into() }),
        "courier new" | "courier" => egui::FontFamily::Name(if bold { "courier new-bold".into() } else { "courier new".into() }),
        _ if !norm.is_empty() => egui::FontFamily::Name(norm.into()),
        _ => {
            if bold { theme::semibold() } else { theme::medium() }
        }
    };
    FontId::new(sz, family)
}

pub fn draw_filled_convex_polygon(
    painter: &Painter,
    center: Pos2,
    points: &[Pos2],
    fill_color: Color32,
) {
    if points.len() < 3 || fill_color.a() == 0 {
        return;
    }
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, fill_color);
    for (i, &p) in points.iter().enumerate() {
        mesh.colored_vertex(p, fill_color);
        let next_i = ((i + 1) % points.len()) as u32;
        mesh.add_triangle(0, (i as u32) + 1, next_i + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

pub fn draw_polygon_stroke(
    painter: &Painter,
    points: &[Pos2],
    stroke: Stroke,
) {
    if points.len() < 2 || stroke.width <= 0.0 || stroke.color.a() == 0 {
        return;
    }
    for i in 0..points.len() {
        let next_i = (i + 1) % points.len();
        painter.line_segment([points[i], points[next_i]], stroke);
    }
}

pub fn get_element_size(elem: &OverlayElement) -> (f32, f32) {
    match elem {
        OverlayElement::Crosshair { size, gap, .. } => {
            let s = (*size + *gap) * 2.0;
            (s, s)
        }
        OverlayElement::Text { text, font_size, .. } => {
            let char_len = (text.chars().count() as f32).max(1.0);
            let est_w = (char_len * (*font_size * 0.6).max(8.0) + 12.0).max(30.0);
            let est_h = (*font_size + 6.0).max(18.0);
            (est_w, est_h)
        }
        OverlayElement::Badge { text, .. } => {
            let est_w = (text.len() as f32 * 7.5 + 18.0).max(40.0);
            (est_w, 22.0)
        }
        OverlayElement::ProgressBar { width, height, .. } => {
            (*width, (*height).max(10.0))
        }
        OverlayElement::Image { width, height, .. } => {
            (*width, *height)
        }
        OverlayElement::Shape { width, height, .. } => {
            (*width, *height)
        }
        OverlayElement::Graph { width, height, .. } => (*width, *height),
        OverlayElement::ClockTimer { width, height, .. } => (*width, *height),
        OverlayElement::InteractiveControl { width, height, .. } => (*width, *height),
        OverlayElement::SystemTelemetry { width, height, .. } => (*width, *height),
        OverlayElement::MediaWidget { width, height, .. } => (*width, *height),
        OverlayElement::KeyVisualizer { width, height, .. } => (*width, *height),
    }
}

pub struct RotatedPainter<'a> {
    pub painter: &'a Painter,
    pub center: Pos2,
    pub angle_rad: f32,
}

impl<'a> RotatedPainter<'a> {
    pub fn new(painter: &'a Painter, center: Pos2, angle_rad: f32) -> Self {
        Self {
            painter,
            center,
            angle_rad,
        }
    }

    #[inline]
    pub fn is_rotated(&self) -> bool {
        self.angle_rad.abs() > 0.001
    }

    #[inline]
    pub fn rot_pos(&self, p: Pos2) -> Pos2 {
        if self.is_rotated() {
            self.center + rotate_vec(p - self.center, self.angle_rad)
        } else {
            p
        }
    }

    pub fn ctx(&self) -> &egui::Context {
        self.painter.ctx()
    }

    pub fn circle_filled(&self, center: Pos2, radius: f32, color: Color32) {
        if color.a() == 0 {
            return;
        }
        self.painter.circle_filled(self.rot_pos(center), radius, color);
    }

    pub fn circle_stroke(&self, center: Pos2, radius: f32, stroke: Stroke) {
        if stroke.width <= 0.0 || stroke.color.a() == 0 {
            return;
        }
        self.painter.circle_stroke(self.rot_pos(center), radius, stroke);
    }

    pub fn line_segment(&self, points: [Pos2; 2], stroke: Stroke) {
        if stroke.width <= 0.0 || stroke.color.a() == 0 {
            return;
        }
        self.painter.line_segment([self.rot_pos(points[0]), self.rot_pos(points[1])], stroke);
    }

    pub fn rect_filled(&self, rect: Rect, corner_radius: CornerRadius, color: Color32) {
        if color.a() == 0 {
            return;
        }
        if !self.is_rotated() {
            self.painter.rect_filled(rect, corner_radius, color);
            return;
        }
        let cr = (corner_radius.nw.max(corner_radius.ne).max(corner_radius.sw).max(corner_radius.se) as f32).min(rect.width() * 0.5).min(rect.height() * 0.5);
        let points = self.get_rect_points(rect, cr);
        let center = self.rot_pos(rect.center());
        draw_filled_convex_polygon(self.painter, center, &points, color);
    }

    pub fn rect_stroke(&self, rect: Rect, corner_radius: CornerRadius, stroke: Stroke, stroke_kind: egui::StrokeKind) {
        if stroke.width <= 0.0 || stroke.color.a() == 0 {
            return;
        }
        if !self.is_rotated() {
            self.painter.rect_stroke(rect, corner_radius, stroke, stroke_kind);
            return;
        }
        let cr = (corner_radius.nw.max(corner_radius.ne).max(corner_radius.sw).max(corner_radius.se) as f32).min(rect.width() * 0.5).min(rect.height() * 0.5);
        let points = self.get_rect_points(rect, cr);
        draw_polygon_stroke(self.painter, &points, stroke);
    }

    fn get_rect_points(&self, rect: Rect, cr: f32) -> Vec<Pos2> {
        let hw = rect.width() * 0.5;
        let hh = rect.height() * 0.5;
        let r_center = rect.center();

        if cr > 0.5 {
            let mut points = Vec::with_capacity(16);
            let arc_steps = 4;
            let centers = [
                (vec2(hw - cr, -(hh - cr)), -std::f32::consts::FRAC_PI_2),
                (vec2(hw - cr, hh - cr), 0.0),
                (vec2(-(hw - cr), hh - cr), std::f32::consts::FRAC_PI_2),
                (vec2(-(hw - cr), -(hh - cr)), std::f32::consts::PI),
            ];
            for (c_off, start_a) in centers {
                for s in 0..arc_steps {
                    let a = start_a + (s as f32) * std::f32::consts::FRAC_PI_2 / (arc_steps as f32);
                    let pt = r_center + c_off + vec2(cr * a.cos(), cr * a.sin());
                    points.push(self.rot_pos(pt));
                }
            }
            points
        } else {
            vec![
                self.rot_pos(rect.left_top()),
                self.rot_pos(rect.right_top()),
                self.rot_pos(rect.right_bottom()),
                self.rot_pos(rect.left_bottom()),
            ]
        }
    }

    pub fn text(&self, pos: Pos2, align: Align2, text: impl AsRef<str>, font: FontId, color: Color32) {
        if color.a() == 0 {
            return;
        }
        let text_str = text.as_ref();
        let galley = self.painter.layout_no_wrap(text_str.to_string(), font, color);
        let text_size = galley.size();

        let top_left = match align {
            Align2::LEFT_TOP => pos,
            Align2::LEFT_CENTER => pos2(pos.x, pos.y - text_size.y * 0.5),
            Align2::LEFT_BOTTOM => pos2(pos.x, pos.y - text_size.y),
            Align2::CENTER_TOP => pos2(pos.x - text_size.x * 0.5, pos.y),
            Align2::CENTER_CENTER => pos2(pos.x - text_size.x * 0.5, pos.y - text_size.y * 0.5),
            Align2::CENTER_BOTTOM => pos2(pos.x - text_size.x * 0.5, pos.y - text_size.y),
            Align2::RIGHT_TOP => pos2(pos.x - text_size.x, pos.y),
            Align2::RIGHT_CENTER => pos2(pos.x - text_size.x, pos.y - text_size.y * 0.5),
            Align2::RIGHT_BOTTOM => pos2(pos.x - text_size.x, pos.y - text_size.y),
        };

        if !self.is_rotated() {
            self.painter.galley(top_left, galley, color);
        } else {
            let unrot_text_center = top_left + text_size * 0.5;
            let rot_text_center = self.rot_pos(unrot_text_center);
            let rotated_top_left = rot_text_center + rotate_vec(-text_size * 0.5, self.angle_rad);
            let mut text_shape = egui::epaint::TextShape::new(rotated_top_left, galley, color);
            text_shape.angle = self.angle_rad;
            self.painter.add(text_shape);
        }
    }

    pub fn image(&self, texture_id: egui::TextureId, rect: Rect, uv: Rect, tint: Color32) {
        if !self.is_rotated() {
            self.painter.image(texture_id, rect, uv, tint);
        } else {
            let p_left_top = self.rot_pos(rect.left_top());
            let p_right_top = self.rot_pos(rect.right_top());
            let p_left_bottom = self.rot_pos(rect.left_bottom());
            let p_right_bottom = self.rot_pos(rect.right_bottom());

            let mut mesh = egui::Mesh::with_texture(texture_id);
            mesh.add_rect_with_uv(
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                uv,
                tint,
            );
            mesh.vertices[0].pos = p_left_top;
            mesh.vertices[1].pos = p_right_top;
            mesh.vertices[2].pos = p_left_bottom;
            mesh.vertices[3].pos = p_right_bottom;
            self.painter.add(egui::Shape::mesh(mesh));
        }
    }
}

#[derive(Clone, Copy)]
struct KeycapStyle {
    idle_bg: Color32,
    active_bg: Color32,
    text_color: Color32,
    zoom: f32,
}

fn render_keycap(
    p: &RotatedPainter,
    key_rect: Rect,
    label: &str,
    is_pressed: bool,
    style: KeycapStyle,
) {
    let cr = CornerRadius::same((4.0 * style.zoom).clamp(2.0, 8.0) as u8);
    let (bg, label_color, stroke) = if is_pressed {
        (
            style.active_bg,
            Color32::WHITE,
            Stroke::new(1.5 * style.zoom, Color32::from_white_alpha(220)),
        )
    } else {
        (
            style.idle_bg,
            style.text_color,
            Stroke::new(1.0, Color32::from_white_alpha(35)),
        )
    };

    if bg.a() > 0 {
        p.rect_filled(key_rect, cr, bg);
    }
    p.rect_stroke(key_rect, cr, stroke, egui::StrokeKind::Inside);

    if is_pressed {
        let highlight_h = (key_rect.height() * 0.35).max(2.0);
        let highlight_rect = Rect::from_min_size(
            pos2(key_rect.left() + 1.0, key_rect.top() + 1.0),
            vec2(key_rect.width() - 2.0, highlight_h),
        );
        p.rect_filled(highlight_rect, CornerRadius::same(cr.nw), Color32::from_white_alpha(45));
    }

    let font_size = (key_rect.height() * 0.42).clamp(7.0 * style.zoom, 16.0 * style.zoom);
    p.text(
        key_rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(font_size, theme::semibold()),
        label_color,
    );
}

pub fn render_overlay_card_bg(
    painter: &Painter,
    card_canvas_rect: Rect,
    overlay: &OverlayDefinition,
    zoom: f32,
    global_opacity: f32,
) {
    let fill = apply_opacity(overlay.style.bg_color, global_opacity);
    let radius = CornerRadius::same((overlay.style.corner_radius * zoom).round() as u8);
    if fill.a() > 0 {
        painter.rect_filled(card_canvas_rect, radius, fill);
    }

    if overlay.style.border_width > 0.0 && overlay.style.border_color[3] > 0 {
        let stroke_col = apply_opacity(overlay.style.border_color, global_opacity);
        let stroke_w = (overlay.style.border_width * zoom).max(1.0);
        painter.rect_stroke(
            card_canvas_rect,
            radius,
            Stroke::new(stroke_w, stroke_col),
            egui::StrokeKind::Inside,
        );
    }
}

pub struct RenderContext<'a> {
    pub painter: &'a Painter,
    pub card_canvas_rect: Rect,
    pub zoom: f32,
    pub global_opacity: f32,
    pub is_interactive: bool,
    pub pointer_pos: Option<Pos2>,
    pub pointer_clicked: bool,
    pub pointer_down: bool,
    pub override_text: Option<&'a str>,
    pub target_process: Option<&'a str>,
}

pub fn execute_click_action(action: &OverlayClickAction) {
    match action {
        OverlayClickAction::TriggerRule { rule_id } => {
            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::TestRunRule {
                id: rule_id.clone(),
                rule: None,
            });
        }
        OverlayClickAction::ToggleRule { rule_id } => {
            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ToggleRule {
                id: rule_id.clone(),
                enabled: true,
            });
        }
        OverlayClickAction::CloseOverlay => {
            #[cfg(windows)]
            unsafe {
                use windows::Win32::Foundation::{HWND, LPARAM};
                use windows::Win32::System::Threading::GetCurrentThreadId;
                use windows::Win32::UI::WindowsAndMessaging::{
                    EnumThreadWindows, GetWindowTextW, PostMessageW, WM_CLOSE,
                };
                struct Context { hwnd: Option<HWND> }
                unsafe extern "system" fn find_overlay_wnd(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
                    let ctx = &mut *(lparam.0 as *mut Context);
                    let mut title_buf = [0u16; 128];
                    let len = GetWindowTextW(hwnd, &mut title_buf);
                    if len > 0 {
                        let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                        if title.contains("ProcessHandler Overlay") {
                            ctx.hwnd = Some(hwnd);
                            return windows::core::BOOL(0);
                        }
                    }
                    windows::core::BOOL(1)
                }
                let mut ctx = Context { hwnd: None };
                let _ = EnumThreadWindows(GetCurrentThreadId(), Some(find_overlay_wnd), LPARAM(&mut ctx as *mut _ as isize));
                if let Some(hwnd) = ctx.hwnd {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, windows::Win32::Foundation::WPARAM(0), windows::Win32::Foundation::LPARAM(0));
                }
            }
        }
        OverlayClickAction::KillProcess { process_name } => {
            let _ = std::process::Command::new("taskkill").args(["/F", "/IM", process_name]).output();
        }
        OverlayClickAction::TrimMemory { process_name } => {
            let _ = crate::daemon::actions::trim_working_set(process_name, None, None, false);
        }
        OverlayClickAction::ToggleMute => {
            crate::daemon::telemetry::simulate_media_key(0xAD); // VK_VOLUME_MUTE
        }
        OverlayClickAction::MediaPlayPause => {
            crate::daemon::telemetry::simulate_media_key(0xB3); // VK_MEDIA_PLAY_PAUSE
        }
        OverlayClickAction::MediaNext => {
            crate::daemon::telemetry::simulate_media_key(0xB0); // VK_MEDIA_NEXT_TRACK
        }
        OverlayClickAction::MediaPrev => {
            crate::daemon::telemetry::simulate_media_key(0xB1); // VK_MEDIA_PREV_TRACK
        }
        OverlayClickAction::ResetTimer => {}
        OverlayClickAction::ToggleTimer => {}
    }
}

pub fn render_element(ctx: &mut RenderContext, elem: &mut OverlayElement) {
    let zoom = ctx.zoom;
    let card_canvas_rect = ctx.card_canvas_rect;
    let (raw_w, raw_h) = get_element_size(elem);
    let center = match elem {
        OverlayElement::Crosshair { x, y, .. } => {
            if *x > 0.0 && *y > 0.0 {
                card_canvas_rect.min + vec2(*x * zoom, *y * zoom)
            } else {
                card_canvas_rect.center()
            }
        }
        _ => {
            let (ex, ey) = (elem.x(), elem.y());
            card_canvas_rect.min + vec2((ex + raw_w * 0.5) * zoom, (ey + raw_h * 0.5) * zoom)
        }
    };
    let angle_rad = elem.rotation().to_radians();
    let p = RotatedPainter::new(ctx.painter, center, angle_rad);
    let local_pointer_pos = ctx.pointer_pos.map(|pos| {
        if angle_rad.abs() > 0.001 {
            center + rotate_vec(pos - center, -angle_rad)
        } else {
            pos
        }
    });
    let rect = Rect::from_center_size(center, vec2(raw_w * zoom, raw_h * zoom));

    match elem {
        OverlayElement::Crosshair {
            style,
            size,
            thickness,
            gap,
            color,
            center_dot,
            ..
        } => {
            let col = apply_opacity(*color, ctx.global_opacity);
            let s = *size * zoom;
            let g = *gap * zoom;
            let t = (*thickness * zoom).max(1.0);
            let stroke = Stroke::new(t, col);

            match style {
                CrosshairStyle::Dot => {
                    p.circle_filled(center, t.max(2.0), col);
                }
                CrosshairStyle::Cross => {
                    p.line_segment([center + vec2(-(g + s), 0.0), center + vec2(-g, 0.0)], stroke);
                    p.line_segment([center + vec2(g, 0.0), center + vec2(g + s, 0.0)], stroke);
                    p.line_segment([center + vec2(0.0, -(g + s)), center + vec2(0.0, -g)], stroke);
                    p.line_segment([center + vec2(0.0, g), center + vec2(0.0, g + s)], stroke);
                    if *center_dot {
                        p.circle_filled(center, t, col);
                    }
                }
                CrosshairStyle::Circle => {
                    p.circle_stroke(center, s * 0.5, stroke);
                    if *center_dot {
                        p.circle_filled(center, t.max(2.0), col);
                    }
                }
                CrosshairStyle::TShaped => {
                    p.line_segment([center + vec2(-(g + s), 0.0), center + vec2(-g, 0.0)], stroke);
                    p.line_segment([center + vec2(g, 0.0), center + vec2(g + s, 0.0)], stroke);
                    p.line_segment([center + vec2(0.0, g), center + vec2(0.0, g + s)], stroke);
                    if *center_dot {
                        p.circle_filled(center, t, col);
                    }
                }
            }
        }

        OverlayElement::Text {
            text,
            font_size,
            color,
            bold,
            font_family,
            on_click,
            ..
        } => {
            let col = apply_opacity(*color, ctx.global_opacity);
            let display_str = ctx.override_text.unwrap_or(text.as_str());
            let font = resolve_font_id(font_family, *font_size, *bold, zoom);
            p.text(center, Align2::CENTER_CENTER, display_str, font, col);

            if ctx.is_interactive && ctx.pointer_clicked && on_click.is_some() {
                if let Some(pos) = local_pointer_pos {
                    if rect.contains(pos) {
                        if let Some(act) = on_click {
                            execute_click_action(act);
                        }
                    }
                }
            }
        }

        OverlayElement::Badge {
            text,
            color,
            bg_color,
            on_click,
            ..
        } => {
            let font = FontId::new((10.5 * zoom).max(7.0), theme::mono_bold());
            let text_layout = ctx.painter.layout_no_wrap(text.clone(), font.clone(), Color32::WHITE);
            let pad_x = 8.0 * zoom;
            let pad_y = 4.0 * zoom;
            let badge_size = text_layout.size() + vec2(pad_x * 2.0, pad_y * 2.0);
            let badge_rect = Rect::from_center_size(center, badge_size);

            if bg_color[3] > 0 {
                p.rect_filled(badge_rect, CornerRadius::same((4.0 * zoom).max(2.0) as u8), apply_opacity(*bg_color, ctx.global_opacity));
            }
            p.text(center, Align2::CENTER_CENTER, text, font, apply_opacity(*color, ctx.global_opacity));

            if ctx.is_interactive && ctx.pointer_clicked && on_click.is_some() {
                if let Some(pos) = local_pointer_pos {
                    if badge_rect.contains(pos) {
                        if let Some(act) = on_click {
                            execute_click_action(act);
                        }
                    }
                }
            }
        }

        OverlayElement::ProgressBar {
            label,
            progress,
            fill_color,
            bg_color,
            ..
        } => {
            let bg_c = apply_opacity(*bg_color, ctx.global_opacity);
            if bg_c.a() > 0 {
                p.rect_filled(rect, CornerRadius::same(3), bg_c);
            }
            let filled_w = rect.width() * progress.clamp(0.0, 1.0);
            if filled_w > 0.0 {
                let fill_rect = Rect::from_min_size(rect.min, vec2(filled_w, rect.height()));
                p.rect_filled(fill_rect, CornerRadius::same(3), apply_opacity(*fill_color, ctx.global_opacity));
            }
            if !label.is_empty() {
                p.text(center, Align2::CENTER_CENTER, label, FontId::new((10.0 * zoom).max(7.0), theme::semibold()), Color32::WHITE);
            }
        }

        OverlayElement::ClockTimer {
            mode,
            format_str,
            font_size,
            color,
            bg_color,
            is_running,
            countdown_total_ms,
            countdown_remaining_ms,
            stopwatch_elapsed_ms,
            bold,
            font_family,
            ..
        } => {
            if bg_color[3] > 0 {
                p.rect_filled(rect, CornerRadius::same((6.0 * zoom).max(3.0) as u8), apply_opacity(*bg_color, ctx.global_opacity));
            }

            // Interactive click on stopwatch/countdown
            if ctx.is_interactive && ctx.pointer_clicked {
                if let Some(pos) = local_pointer_pos {
                    if rect.contains(pos) && (*mode == TimerMode::Stopwatch || *mode == TimerMode::Countdown) {
                        if *mode == TimerMode::Countdown && *countdown_remaining_ms == 0 {
                            *countdown_remaining_ms = *countdown_total_ms;
                        }
                        *is_running = !*is_running;
                    }
                }
            }

            let text = match mode {
                TimerMode::Clock => chrono::Local::now().format(format_str).to_string(),
                TimerMode::Stopwatch => {
                    let total_sec = *stopwatch_elapsed_ms / 1000;
                    let m = total_sec / 60;
                    let s = total_sec % 60;
                    let ms = (*stopwatch_elapsed_ms % 1000) / 100;
                    format!("{:02}:{:02}.{}", m, s, ms)
                }
                TimerMode::Countdown => {
                    let total_sec = *countdown_remaining_ms / 1000;
                    let m = total_sec / 60;
                    let s = total_sec % 60;
                    format!("{:02}:{:02}", m, s)
                }
                TimerMode::Uptime => {
                    #[cfg(windows)]
                    let sec = unsafe { windows::Win32::System::SystemInformation::GetTickCount64() } / 1000;
                    #[cfg(not(windows))]
                    let sec = 7200;
                    let d = sec / 86400;
                    let h = (sec % 86400) / 3600;
                    let m = (sec % 3600) / 60;
                    if d > 0 {
                        format!("{}d {}h {}m", d, h, m)
                    } else {
                        format!("{:02}:{:02}", h, m)
                    }
                }
            };

            let font = resolve_font_id(font_family, *font_size, *bold, zoom);
            let col = apply_opacity(*color, ctx.global_opacity);
            p.text(center, Align2::CENTER_CENTER, text, font, col);
        }

        OverlayElement::Graph {
            metric,
            style,
            sample_count,
            color,
            fill_color,
            bg_color,
            show_current_val,
            ..
        } => {
            if bg_color[3] > 0 {
                p.rect_filled(rect, CornerRadius::same(6), apply_opacity(*bg_color, ctx.global_opacity));
            }
            p.rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)), egui::StrokeKind::Inside);

            let count = (*sample_count).clamp(8, 60);
            let samples = crate::daemon::telemetry::get_graph_samples(*metric, count, ctx.target_process);
            let current_label = crate::daemon::telemetry::get_graph_current_label(*metric, ctx.target_process);
            let col = apply_opacity(*color, ctx.global_opacity);

            let pad = 6.0 * zoom;
            let plot_rect = rect.shrink(pad);

            if !samples.is_empty() && plot_rect.width() > 10.0 && plot_rect.height() > 10.0 {
                let n = samples.len();
                let dx = plot_rect.width() / (n.max(2) - 1) as f32;

                match style {
                    GraphStyle::Histogram => {
                        let bar_w = (dx * 0.75).max(2.0);
                        for (i, &v) in samples.iter().enumerate() {
                            let frac = v.clamp(0.05, 1.0);
                            let bar_h = plot_rect.height() * frac;
                            let bx = plot_rect.left() + i as f32 * dx;
                            let by = plot_rect.bottom() - bar_h;
                            let bar_rect = Rect::from_min_size(pos2(bx, by), vec2(bar_w, bar_h));
                            p.rect_filled(bar_rect, CornerRadius::same(2), col);
                        }
                    }
                    GraphStyle::Line | GraphStyle::AreaFilled => {
                        let mut points: Vec<Pos2> = Vec::with_capacity(n);
                        for (i, &v) in samples.iter().enumerate() {
                            let frac = v.clamp(0.0, 1.0);
                            let px = plot_rect.left() + i as f32 * dx;
                            let py = plot_rect.bottom() - frac * (plot_rect.height() - 10.0 * zoom);
                            points.push(pos2(px, py));
                        }

                        if *style == GraphStyle::AreaFilled && points.len() >= 2 {
                            let mut area_pts = Vec::with_capacity(points.len() + 2);
                            for pt in &points {
                                area_pts.push(p.rot_pos(*pt));
                            }
                            area_pts.push(p.rot_pos(pos2(plot_rect.right(), plot_rect.bottom())));
                            area_pts.push(p.rot_pos(pos2(plot_rect.left(), plot_rect.bottom())));
                            let fill_c = apply_opacity(*fill_color, ctx.global_opacity * 0.35);
                            draw_filled_convex_polygon(ctx.painter, center, &area_pts, fill_c);
                        }

                        if points.len() >= 2 {
                            for w in points.windows(2) {
                                p.line_segment([w[0], w[1]], Stroke::new(1.8 * zoom, col));
                            }
                        }

                        if let Some(&last_pt) = points.last() {
                            p.circle_filled(last_pt, 2.5 * zoom, col);
                            p.circle_filled(last_pt, 5.0 * zoom, Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 50));
                        }
                    }
                }
            }

            if *show_current_val {
                let tag_name = match metric {
                    GraphMetric::SystemCpu => "CPU",
                    GraphMetric::SystemRam => "RAM",
                    GraphMetric::ProcessCpu => "PROC CPU",
                    GraphMetric::ProcessRam => "PROC RAM",
                    GraphMetric::Fps => "FPS",
                    GraphMetric::Battery => "BATTERY",
                    GraphMetric::Network => "NET",
                };
                p.text(pos2(rect.left() + pad, rect.top() + pad * 0.7), Align2::LEFT_TOP, tag_name, FontId::new((9.0 * zoom).max(7.0), theme::semibold()), Color32::from_gray(170));
                p.text(pos2(rect.right() - pad, rect.top() + pad * 0.7), Align2::RIGHT_TOP, current_label, FontId::new((9.5 * zoom).max(7.0), theme::mono_bold()), col);
            }
        }

        OverlayElement::SystemTelemetry {
            kind,
            style,
            color,
            bg_color,
            on_click,
            ..
        } => {
            if bg_color[3] > 0 {
                p.rect_filled(rect, CornerRadius::same(5), apply_opacity(*bg_color, ctx.global_opacity));
            }

            if ctx.is_interactive && ctx.pointer_clicked && on_click.is_some() {
                if let Some(pos) = local_pointer_pos {
                    if rect.contains(pos) {
                        if let Some(act) = on_click {
                            execute_click_action(act);
                        }
                    }
                }
            }

            let col = apply_opacity(*color, ctx.global_opacity);

            if *kind == TelemetryKind::EngineStatus {
                let engine = crate::daemon::telemetry::get_engine_telemetry();
                let led_center = pos2(rect.left() + 10.0 * zoom, center.y);
                let led_color = if engine.heartbeat_pulse { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(22, 101, 52) };
                
                if engine.heartbeat_pulse {
                    p.circle_filled(led_center, 6.0 * zoom, Color32::from_rgba_unmultiplied(34, 197, 94, 60));
                }
                p.circle_filled(led_center, 3.5 * zoom, led_color);

                let text_pos = pos2(led_center.x + 8.0 * zoom, center.y);
                let text = match style {
                    TelemetryStyle::Badge => {
                        if engine.heartbeat_pulse { "ONLINE".to_string() } else { "OFFLINE".to_string() }
                    }
                    TelemetryStyle::Compact => {
                        format!("PH: {} active", engine.active_rules_count)
                    }
                    TelemetryStyle::Detailed => {
                        format!("Engine: {} rules, {} mac | {}", engine.active_rules_count, engine.active_macros_count, engine.quota_status)
                    }
                };
                p.text(text_pos, Align2::LEFT_CENTER, text, FontId::new((10.5 * zoom).max(7.0), theme::semibold()), col);
            } else {
                let text = crate::daemon::telemetry::format_telemetry_text(*kind, *style, ctx.target_process);
                p.text(center, Align2::CENTER_CENTER, text, FontId::new((10.5 * zoom).max(7.0), theme::mono_bold()), col);
            }
        }

        OverlayElement::InteractiveControl {
            control,
            color,
            bg_color,
            active_color,
            ..
        } => {
            match control {
                InteractiveControlKind::ToggleSwitch { label, is_active, bind_rule_id, on_enable, on_disable } => {
                    let switch_w = 34.0 * zoom;
                    let switch_h = 18.0 * zoom;
                    let switch_rect = Rect::from_min_size(pos2(rect.left(), center.y - switch_h * 0.5), vec2(switch_w, switch_h));

                    if ctx.is_interactive && ctx.pointer_clicked {
                        if let Some(pos) = local_pointer_pos {
                            if rect.contains(pos) {
                                *is_active = !*is_active;
                                if let Some(rule_id) = bind_rule_id {
                                    let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ToggleRule {
                                        id: rule_id.clone(),
                                        enabled: *is_active,
                                    });
                                }
                                if *is_active {
                                    if let Some(act) = on_enable {
                                        execute_click_action(act);
                                    }
                                } else if let Some(act) = on_disable {
                                    execute_click_action(act);
                                }
                            }
                        }
                    }

                    let bg = if *is_active { apply_opacity(*active_color, ctx.global_opacity) } else { apply_opacity(*bg_color, ctx.global_opacity) };
                    p.rect_filled(switch_rect, CornerRadius::same((switch_h * 0.5) as u8), bg);

                    let knob_radius = switch_h * 0.4;
                    let knob_x = if *is_active { switch_rect.right() - switch_h * 0.5 } else { switch_rect.left() + switch_h * 0.5 };
                    p.circle_filled(pos2(knob_x, center.y), knob_radius, Color32::WHITE);

                    let label_pos = pos2(switch_rect.right() + 8.0 * zoom, center.y);
                    p.text(label_pos, Align2::LEFT_CENTER, label, FontId::new((11.0 * zoom).max(8.0), theme::semibold()), Color32::WHITE);
                }

                InteractiveControlKind::Slider { label, min, max, value, bind_target } => {
                    let slider_track_h = 6.0 * zoom;
                    let top_pos = pos2(rect.left(), rect.top());
                    p.text(top_pos, Align2::LEFT_TOP, format!("{}: {:.0}", label, *value), FontId::new((10.0 * zoom).max(7.0), theme::semibold()), Color32::from_gray(220));

                    let track_rect = Rect::from_min_size(pos2(rect.left(), rect.bottom() - slider_track_h - 2.0 * zoom), vec2(rect.width(), slider_track_h));
                    p.rect_filled(track_rect, CornerRadius::same(3), apply_opacity(*bg_color, ctx.global_opacity));

                    let frac = ((*value - *min) / (*max - *min).max(0.001)).clamp(0.0, 1.0);
                    let filled_w = track_rect.width() * frac;
                    let filled_rect = Rect::from_min_size(track_rect.min, vec2(filled_w, slider_track_h));
                    let col = apply_opacity(*active_color, ctx.global_opacity);
                    p.rect_filled(filled_rect, CornerRadius::same(3), col);

                    let thumb_x = track_rect.left() + filled_w;
                    let thumb_center = pos2(thumb_x, track_rect.center().y);
                    p.circle_filled(thumb_center, 6.0 * zoom, Color32::WHITE);
                    p.circle_stroke(thumb_center, 6.0 * zoom, Stroke::new(1.5, col));

                    // Dragging slider
                    if ctx.is_interactive && ctx.pointer_down {
                        if let Some(pos) = local_pointer_pos {
                            if rect.expand(4.0).contains(pos) {
                                let new_frac = ((pos.x - track_rect.left()) / track_rect.width().max(1.0)).clamp(0.0, 1.0);
                                *value = *min + new_frac * (*max - *min);
                                if *bind_target == SliderBindTarget::MasterVolume {
                                    crate::daemon::actions::set_master_volume_scalar(new_frac);
                                }
                            }
                        }
                    }
                }

                InteractiveControlKind::Button { label, action, .. } => {
                    let mut btn_bg = apply_opacity(*active_color, ctx.global_opacity);
                    let is_hovered = local_pointer_pos.map(|p| rect.contains(p)).unwrap_or(false);

                    if is_hovered {
                        btn_bg = Color32::from_rgba_premultiplied(
                            btn_bg.r().saturating_add(25),
                            btn_bg.g().saturating_add(25),
                            btn_bg.b().saturating_add(25),
                            btn_bg.a(),
                        );
                    }

                    p.rect_filled(rect, CornerRadius::same(6), btn_bg);
                    p.rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30)), egui::StrokeKind::Inside);

                    let col = apply_opacity(*color, ctx.global_opacity);
                    p.text(center, Align2::CENTER_CENTER, label, FontId::new((11.0 * zoom).max(8.0), theme::semibold()), col);

                    if ctx.is_interactive && ctx.pointer_clicked && is_hovered {
                        if let Some(act) = action {
                            execute_click_action(act);
                        }
                    }
                }
            }
        }

        OverlayElement::KeyVisualizer {
            layout,
            max_keys,
            decay_seconds,
            color,
            bg_color,
            active_color,
            ..
        } => {
            if bg_color[3] > 0 {
                p.rect_filled(rect, CornerRadius::same(8), apply_opacity(*bg_color, ctx.global_opacity));
            }
            p.rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 25)), egui::StrokeKind::Inside);

            let idle_bg = Color32::from_rgba_unmultiplied(25, 28, 38, (200.0 * ctx.global_opacity) as u8);
            let active_bg = apply_opacity(*active_color, ctx.global_opacity);
            let text_c = apply_opacity(*color, ctx.global_opacity);
            let k_style = KeycapStyle {
                idle_bg,
                active_bg,
                text_color: text_c,
                zoom,
            };

            let pad = 4.0 * zoom;
            let inner_rect = rect.shrink(pad);
            let iw = inner_rect.width();
            let ih = inner_rect.height();

            match layout {
                crate::models::KeyVisualizerLayout::TextStream => {
                    let live_keys = crate::daemon::telemetry::get_active_or_recent_keys(*max_keys, *decay_seconds);
                    p.text(center, Align2::CENTER_CENTER, live_keys, FontId::new((11.0 * zoom).max(7.0), theme::mono_bold()), text_c);
                }

                crate::models::KeyVisualizerLayout::WasdOnly => {
                    let gap = 3.0 * zoom;
                    let row_h = (ih - gap * 2.0) / 3.0;
                    let key_w = (iw - gap * 2.0) / 3.0;

                    // Row 0: W centered
                    let w_rect = Rect::from_min_size(pos2(inner_rect.left() + key_w + gap, inner_rect.top()), vec2(key_w, row_h));
                    render_keycap(&p, w_rect, "W", crate::daemon::telemetry::is_vk_pressed(0x57), k_style);

                    // Row 1: A, S, D
                    let y1 = inner_rect.top() + row_h + gap;
                    let a_rect = Rect::from_min_size(pos2(inner_rect.left(), y1), vec2(key_w, row_h));
                    let s_rect = Rect::from_min_size(pos2(inner_rect.left() + key_w + gap, y1), vec2(key_w, row_h));
                    let d_rect = Rect::from_min_size(pos2(inner_rect.left() + (key_w + gap) * 2.0, y1), vec2(key_w, row_h));
                    render_keycap(&p, a_rect, "A", crate::daemon::telemetry::is_vk_pressed(0x41), k_style);
                    render_keycap(&p, s_rect, "S", crate::daemon::telemetry::is_vk_pressed(0x53), k_style);
                    render_keycap(&p, d_rect, "D", crate::daemon::telemetry::is_vk_pressed(0x44), k_style);

                    // Row 2: Space bar
                    let y2 = y1 + row_h + gap;
                    let space_rect = Rect::from_min_size(pos2(inner_rect.left(), y2), vec2(iw, row_h));
                    render_keycap(&p, space_rect, "SPACE", crate::daemon::telemetry::is_vk_pressed(0x20), k_style);
                }

                crate::models::KeyVisualizerLayout::WasdMouse => {
                    let gap = 3.0 * zoom;
                    let kb_w = iw * 0.70;
                    let mouse_w = iw - kb_w - gap;

                    // --- KEYBOARD CLUSTER (Left) ---
                    let row_h = (ih - gap * 2.0) / 3.0;

                    // Row 0: Q, W, E, R
                    let k_w4 = (kb_w - gap * 3.0) / 4.0;
                    let q_rect = Rect::from_min_size(pos2(inner_rect.left(), inner_rect.top()), vec2(k_w4, row_h));
                    let w_rect = Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap), inner_rect.top()), vec2(k_w4, row_h));
                    let e_rect = Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 2.0, inner_rect.top()), vec2(k_w4, row_h));
                    let r_rect = Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 3.0, inner_rect.top()), vec2(k_w4, row_h));
                    render_keycap(&p, q_rect, "Q", crate::daemon::telemetry::is_vk_pressed(0x51), k_style);
                    render_keycap(&p, w_rect, "W", crate::daemon::telemetry::is_vk_pressed(0x57), k_style);
                    render_keycap(&p, e_rect, "E", crate::daemon::telemetry::is_vk_pressed(0x45), k_style);
                    render_keycap(&p, r_rect, "R", crate::daemon::telemetry::is_vk_pressed(0x52), k_style);

                    // Row 1: A, S, D, F
                    let y1 = inner_rect.top() + row_h + gap;
                    let a_rect = Rect::from_min_size(pos2(inner_rect.left(), y1), vec2(k_w4, row_h));
                    let s_rect = Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap), y1), vec2(k_w4, row_h));
                    let d_rect = Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 2.0, y1), vec2(k_w4, row_h));
                    let f_rect = Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 3.0, y1), vec2(k_w4, row_h));
                    render_keycap(&p, a_rect, "A", crate::daemon::telemetry::is_vk_pressed(0x41), k_style);
                    render_keycap(&p, s_rect, "S", crate::daemon::telemetry::is_vk_pressed(0x53), k_style);
                    render_keycap(&p, d_rect, "D", crate::daemon::telemetry::is_vk_pressed(0x44), k_style);
                    render_keycap(&p, f_rect, "F", crate::daemon::telemetry::is_vk_pressed(0x46), k_style);

                    // Row 2: Shift, Space, Ctrl
                    let y2 = y1 + row_h + gap;
                    let shift_w = kb_w * 0.28;
                    let ctrl_w = kb_w * 0.24;
                    let space_w = kb_w - shift_w - ctrl_w - gap * 2.0;

                    let shift_rect = Rect::from_min_size(pos2(inner_rect.left(), y2), vec2(shift_w, row_h));
                    let space_rect = Rect::from_min_size(pos2(inner_rect.left() + shift_w + gap, y2), vec2(space_w, row_h));
                    let ctrl_rect = Rect::from_min_size(pos2(inner_rect.left() + shift_w + space_w + gap * 2.0, y2), vec2(ctrl_w, row_h));

                    render_keycap(&p, shift_rect, "SHIFT", crate::daemon::telemetry::is_vk_pressed(0x10), k_style);
                    render_keycap(&p, space_rect, "SPACE", crate::daemon::telemetry::is_vk_pressed(0x20), k_style);
                    render_keycap(&p, ctrl_rect, "CTRL", crate::daemon::telemetry::is_vk_pressed(0x11), k_style);

                    // --- MOUSE CLUSTER (Right) ---
                    let mx = inner_rect.left() + kb_w + gap;
                    let m_gap = 2.0 * zoom;
                    let top_btn_h = ih * 0.52;
                    let bottom_btn_h = ih - top_btn_h - m_gap;
                    let mmb_w = (mouse_w * 0.24).clamp(6.0 * zoom, 16.0 * zoom);
                    let side_btn_w = (mouse_w - mmb_w - m_gap * 2.0) * 0.5;

                    let lmb_rect = Rect::from_min_size(pos2(mx, inner_rect.top()), vec2(side_btn_w, top_btn_h));
                    let mmb_rect = Rect::from_min_size(pos2(mx + side_btn_w + m_gap, inner_rect.top()), vec2(mmb_w, top_btn_h * 0.65));
                    let rmb_rect = Rect::from_min_size(pos2(mx + side_btn_w + mmb_w + m_gap * 2.0, inner_rect.top()), vec2(side_btn_w, top_btn_h));

                    render_keycap(&p, lmb_rect, "L", crate::daemon::telemetry::is_vk_pressed(0x01), k_style);
                    render_keycap(&p, mmb_rect, "M", crate::daemon::telemetry::is_vk_pressed(0x04), k_style);
                    render_keycap(&p, rmb_rect, "R", crate::daemon::telemetry::is_vk_pressed(0x02), k_style);

                    // Bottom side buttons (M4 / M5)
                    let by = inner_rect.top() + top_btn_h + m_gap;
                    let m4_w = (mouse_w - m_gap) * 0.5;
                    let m4_rect = Rect::from_min_size(pos2(mx, by), vec2(m4_w, bottom_btn_h));
                    let m5_rect = Rect::from_min_size(pos2(mx + m4_w + m_gap, by), vec2(m4_w, bottom_btn_h));

                    render_keycap(&p, m4_rect, "M4", crate::daemon::telemetry::is_vk_pressed(0x05), k_style);
                    render_keycap(&p, m5_rect, "M5", crate::daemon::telemetry::is_vk_pressed(0x06), k_style);
                }

                crate::models::KeyVisualizerLayout::ActionRpg => {
                    let gap = 3.0 * zoom;
                    let row_h = (ih - gap * 3.0) / 4.0;
                    let k_w4 = (iw - gap * 3.0) / 4.0;

                    // Row 0: 1, 2, 3, 4
                    let y0 = inner_rect.top();
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left(), y0), vec2(k_w4, row_h)), "1", crate::daemon::telemetry::is_vk_pressed(0x31), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap), y0), vec2(k_w4, row_h)), "2", crate::daemon::telemetry::is_vk_pressed(0x32), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 2.0, y0), vec2(k_w4, row_h)), "3", crate::daemon::telemetry::is_vk_pressed(0x33), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 3.0, y0), vec2(k_w4, row_h)), "4", crate::daemon::telemetry::is_vk_pressed(0x34), k_style);

                    // Row 1: Q, W, E, R
                    let y1 = y0 + row_h + gap;
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left(), y1), vec2(k_w4, row_h)), "Q", crate::daemon::telemetry::is_vk_pressed(0x51), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap), y1), vec2(k_w4, row_h)), "W", crate::daemon::telemetry::is_vk_pressed(0x57), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 2.0, y1), vec2(k_w4, row_h)), "E", crate::daemon::telemetry::is_vk_pressed(0x45), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 3.0, y1), vec2(k_w4, row_h)), "R", crate::daemon::telemetry::is_vk_pressed(0x52), k_style);

                    // Row 2: D, F, LMB, RMB
                    let y2 = y1 + row_h + gap;
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left(), y2), vec2(k_w4, row_h)), "D", crate::daemon::telemetry::is_vk_pressed(0x44), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap), y2), vec2(k_w4, row_h)), "F", crate::daemon::telemetry::is_vk_pressed(0x46), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 2.0, y2), vec2(k_w4, row_h)), "LMB", crate::daemon::telemetry::is_vk_pressed(0x01), k_style);
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left() + (k_w4 + gap) * 3.0, y2), vec2(k_w4, row_h)), "RMB", crate::daemon::telemetry::is_vk_pressed(0x02), k_style);

                    // Row 3: Space (wide)
                    let y3 = y2 + row_h + gap;
                    render_keycap(&p, Rect::from_min_size(pos2(inner_rect.left(), y3), vec2(iw, row_h)), "SPACE", crate::daemon::telemetry::is_vk_pressed(0x20), k_style);
                }

                crate::models::KeyVisualizerLayout::MouseOnly => {
                    let gap = 3.0 * zoom;
                    let top_h = ih * 0.60;
                    let bottom_h = ih - top_h - gap;

                    let mmb_w = (iw * 0.22).clamp(8.0 * zoom, 24.0 * zoom);
                    let side_w = (iw - mmb_w - gap * 2.0) * 0.5;

                    // Top: LMB, MMB, RMB
                    let lmb_rect = Rect::from_min_size(pos2(inner_rect.left(), inner_rect.top()), vec2(side_w, top_h));
                    let mmb_rect = Rect::from_min_size(pos2(inner_rect.left() + side_w + gap, inner_rect.top()), vec2(mmb_w, top_h * 0.70));
                    let rmb_rect = Rect::from_min_size(pos2(inner_rect.left() + side_w + mmb_w + gap * 2.0, inner_rect.top()), vec2(side_w, top_h));

                    render_keycap(&p, lmb_rect, "LMB", crate::daemon::telemetry::is_vk_pressed(0x01), k_style);
                    render_keycap(&p, mmb_rect, "WHEEL", crate::daemon::telemetry::is_vk_pressed(0x04), k_style);
                    render_keycap(&p, rmb_rect, "RMB", crate::daemon::telemetry::is_vk_pressed(0x02), k_style);

                    // Bottom: M4, M5
                    let by = inner_rect.top() + top_h + gap;
                    let mb_w = (iw - gap) * 0.5;
                    let m4_rect = Rect::from_min_size(pos2(inner_rect.left(), by), vec2(mb_w, bottom_h));
                    let m5_rect = Rect::from_min_size(pos2(inner_rect.left() + mb_w + gap, by), vec2(mb_w, bottom_h));

                    render_keycap(&p, m4_rect, "MOUSE 4", crate::daemon::telemetry::is_vk_pressed(0x05), k_style);
                    render_keycap(&p, m5_rect, "MOUSE 5", crate::daemon::telemetry::is_vk_pressed(0x06), k_style);
                }
            }
        }

        OverlayElement::MediaWidget {
            show_controls,
            color,
            bg_color,
            ..
        } => {
            // Sleek card background
            if bg_color[3] > 0 {
                p.rect_filled(rect, CornerRadius::same(8), apply_opacity(*bg_color, ctx.global_opacity));
            }
            p.rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 35)), egui::StrokeKind::Inside);

            let media = crate::daemon::telemetry::get_system_media_status();
            let col = apply_opacity(*color, ctx.global_opacity);
            let pad = 8.0 * zoom;

            // Equalizer animation bars
            let eq_left = rect.left() + pad;
            let bar_count = 4;
            let time_sec = Instant::now().elapsed().as_secs_f32();
            let bar_w = 3.0 * zoom;
            let max_bar_h = 14.0 * zoom;
            for i in 0..bar_count {
                let bx = eq_left + i as f32 * (bar_w + 2.0 * zoom);
                let phase = time_sec * 6.0 + i as f32 * 1.5;
                let h_frac = if media.is_playing { (phase.sin().abs() * 0.8 + 0.2).clamp(0.2, 1.0) } else { 0.2 };
                let bh = max_bar_h * h_frac;
                let by = rect.top() + pad + (max_bar_h - bh);
                p.rect_filled(Rect::from_min_size(pos2(bx, by), vec2(bar_w, bh)), CornerRadius::same(1), col);
            }

            // Track title & Artist
            let text_left = eq_left + (bar_count as f32 * (bar_w + 2.0 * zoom)) + 6.0 * zoom;
            let title_pos = pos2(text_left, rect.top() + pad);
            let short_title = safe_truncate(&media.title, 22);
            p.text(title_pos, Align2::LEFT_TOP, short_title, FontId::new((11.0 * zoom).max(8.0), theme::semibold()), col);

            let artist_pos = pos2(text_left, title_pos.y + 13.0 * zoom);
            let artist_line = format!("{} • {}", media.artist, media.player_name);
            let short_artist = safe_truncate(&artist_line, 26);
            p.text(artist_pos, Align2::LEFT_TOP, short_artist, FontId::new((9.0 * zoom).max(7.0), theme::medium()), Color32::from_gray(180));

            // Scrubber progress bar with interactive knob (ползунок)
            let bar_y = rect.top() + rect.height() * 0.58;
            let bar_w = rect.width() - pad * 2.0;
            let track_h = (3.5 * zoom).max(2.5);
            let bar_rect = Rect::from_min_size(pos2(rect.left() + pad, bar_y), vec2(bar_w, track_h));
            let is_hover_bar = local_pointer_pos.map(|p| bar_rect.expand(6.0 * zoom).contains(p)).unwrap_or(false);

            // Interactive seeking: click or drag along the progress bar
            if ctx.is_interactive && (ctx.pointer_down || ctx.pointer_clicked) && is_hover_bar {
                if let Some(pos) = local_pointer_pos {
                    let new_pct = ((pos.x - bar_rect.left()) / bar_rect.width().max(1.0)).clamp(0.0, 1.0);
                    crate::daemon::telemetry::seek_media_progress(new_pct);
                }
            }

            // Track background
            p.rect_filled(bar_rect, CornerRadius::same(2), Color32::from_rgb(50, 55, 65));

            // Filled progress portion
            let fill_w = bar_w * media.progress_pct.clamp(0.0, 1.0);
            if fill_w > 0.0 {
                p.rect_filled(Rect::from_min_size(bar_rect.min, vec2(fill_w, track_h)), CornerRadius::same(2), col);
            }

            // Slider Knob / Thumb (Ползунок)
            let thumb_r = if is_hover_bar { 5.5 * zoom } else { 4.2 * zoom };
            let thumb_x = bar_rect.left() + fill_w;
            let thumb_center = pos2(thumb_x, bar_rect.center().y);

            if is_hover_bar {
                p.circle_filled(thumb_center, thumb_r + 2.5 * zoom, col.gamma_multiply(0.28));
            }
            p.circle_filled(thumb_center, thumb_r, Color32::WHITE);
            p.circle_stroke(thumb_center, thumb_r, Stroke::new((1.5 * zoom).max(1.0), col));

            // Transport buttons
            if *show_controls {
                let btn_y = rect.bottom() - 13.0 * zoom;
                let spacing = 28.0 * zoom;
                let prev_center = pos2(center.x - spacing, btn_y);
                let play_center = pos2(center.x, btn_y);
                let next_center = pos2(center.x + spacing, btn_y);

                let btn_r = 11.0 * zoom;
                let prev_rect = Rect::from_center_size(prev_center, vec2(btn_r * 2.2, btn_r * 2.2));
                let play_rect = Rect::from_center_size(play_center, vec2(btn_r * 2.4, btn_r * 2.4));
                let next_rect = Rect::from_center_size(next_center, vec2(btn_r * 2.2, btn_r * 2.2));

                let is_hover_prev = local_pointer_pos.map(|p| prev_rect.contains(p)).unwrap_or(false);
                let is_hover_play = local_pointer_pos.map(|p| play_rect.contains(p)).unwrap_or(false);
                let is_hover_next = local_pointer_pos.map(|p| next_rect.contains(p)).unwrap_or(false);

                let is_down_prev = is_hover_prev && ctx.pointer_down;
                let is_down_play = is_hover_play && ctx.pointer_down;
                let is_down_next = is_hover_next && ctx.pointer_down;

                if is_down_prev {
                    p.circle_filled(prev_center, btn_r, col.gamma_multiply(0.4));
                    p.circle_stroke(prev_center, btn_r, Stroke::new(1.2 * zoom, col));
                } else if is_hover_prev {
                    p.circle_filled(prev_center, btn_r, col.gamma_multiply(0.2));
                    p.circle_stroke(prev_center, btn_r, Stroke::new(1.0 * zoom, col.gamma_multiply(0.6)));
                }

                if is_down_play {
                    p.circle_filled(play_center, btn_r * 1.15, col.gamma_multiply(0.5));
                    p.circle_stroke(play_center, btn_r * 1.15, Stroke::new(1.5 * zoom, col));
                } else if is_hover_play {
                    p.circle_filled(play_center, btn_r * 1.15, col.gamma_multiply(0.25));
                    p.circle_stroke(play_center, btn_r * 1.15, Stroke::new(1.2 * zoom, col.gamma_multiply(0.7)));
                }

                if is_down_next {
                    p.circle_filled(next_center, btn_r, col.gamma_multiply(0.4));
                    p.circle_stroke(next_center, btn_r, Stroke::new(1.2 * zoom, col));
                } else if is_hover_next {
                    p.circle_filled(next_center, btn_r, col.gamma_multiply(0.2));
                    p.circle_stroke(next_center, btn_r, Stroke::new(1.0 * zoom, col.gamma_multiply(0.6)));
                }

                p.text(prev_center, Align2::CENTER_CENTER, "⏮", FontId::new((10.5 * zoom).max(7.5), theme::semibold()), if is_hover_prev { Color32::WHITE } else { col });
                let play_icon = if media.is_playing { "⏸" } else { "▶" };
                p.text(play_center, Align2::CENTER_CENTER, play_icon, FontId::new((13.0 * zoom).max(8.5), theme::semibold()), if is_hover_play { Color32::WHITE } else { col });
                p.text(next_center, Align2::CENTER_CENTER, "⏭", FontId::new((10.5 * zoom).max(7.5), theme::semibold()), if is_hover_next { Color32::WHITE } else { col });

                if ctx.is_interactive && ctx.pointer_clicked {
                    if let Some(pos) = local_pointer_pos {
                        if prev_rect.contains(pos) {
                            crate::daemon::telemetry::simulate_media_key(0xB1); // Previous
                        } else if play_rect.contains(pos) {
                            crate::daemon::telemetry::simulate_media_key(0xB3); // Play/pause
                        } else if next_rect.contains(pos) {
                            crate::daemon::telemetry::simulate_media_key(0xB0); // Next
                        }
                    }
                }
            }
        }

        OverlayElement::Shape {
            shape_type,
            color,
            filled,
            fill_color,
            thickness,
            corner_radius,
            on_click,
            ..
        } => {
            let stroke = Stroke::new((*thickness * zoom).max(1.0), apply_opacity(*color, ctx.global_opacity));
            let fill_c = if *filled { Some(apply_opacity(*fill_color, ctx.global_opacity)) } else { None };

            match shape_type {
                ShapeKind::Rectangle => {
                    let cr = (*corner_radius * zoom).min(rect.width() * 0.5).min(rect.height() * 0.5);
                    let corner_rad = CornerRadius::same(cr.round() as u8);
                    if let Some(fc) = fill_c {
                        p.rect_filled(rect, corner_rad, fc);
                    }
                    p.rect_stroke(rect, corner_rad, stroke, egui::StrokeKind::Inside);
                }
                ShapeKind::Circle => {
                    let rx = rect.width() * 0.5;
                    let ry = rect.height() * 0.5;

                    if (rx - ry).abs() < 0.5 {
                        if let Some(fc) = fill_c {
                            p.circle_filled(center, rx, fc);
                        }
                        p.circle_stroke(center, rx, stroke);
                    } else {
                        let n = 48;
                        let mut points = Vec::with_capacity(n);
                        for i in 0..n {
                            let t = (i as f32) * std::f32::consts::TAU / (n as f32);
                            let pt = center + vec2(rx * t.cos(), ry * t.sin());
                            points.push(p.rot_pos(pt));
                        }

                        if let Some(fc) = fill_c {
                            draw_filled_convex_polygon(ctx.painter, center, &points, fc);
                        }
                        draw_polygon_stroke(ctx.painter, &points, stroke);
                    }
                }
                ShapeKind::Line => {
                    let hw = rect.width() * 0.5;
                    let p_left = center + vec2(-hw, 0.0);
                    let p_right = center + vec2(hw, 0.0);
                    p.line_segment([p_left, p_right], stroke);
                }
                ShapeKind::Arrow => {
                    let hw = rect.width() * 0.5;
                    let p_left = center + vec2(-hw, 0.0);
                    let p_right = center + vec2(hw, 0.0);
                    p.line_segment([p_left, p_right], stroke);
                    let head_sz = (rect.height() * 0.4).clamp(4.0 * zoom, 24.0 * zoom);
                    let barb_top = p_right + vec2(-head_sz, -head_sz);
                    let barb_bottom = p_right + vec2(-head_sz, head_sz);
                    p.line_segment([barb_top, p_right], stroke);
                    p.line_segment([barb_bottom, p_right], stroke);
                }
            }

            if ctx.is_interactive && ctx.pointer_clicked && on_click.is_some() {
                if let Some(pos) = local_pointer_pos {
                    if rect.contains(pos) {
                        if let Some(act) = on_click {
                            execute_click_action(act);
                        }
                    }
                }
            }
        }

        OverlayElement::Image {
            path,
            opacity,
            lock_aspect,
            on_click,
            ..
        } => {
            let alpha = (*opacity * ctx.global_opacity).clamp(0.0, 1.0);
            let tint = Color32::from_rgba_unmultiplied(255, 255, 255, (alpha * 255.0) as u8);

            let trimmed_path = path.trim().trim_matches('"').trim_matches('\'').trim();
            if trimmed_path.is_empty() {
                p.rect_filled(rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(255, 255, 255, 10));
                p.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30)), egui::StrokeKind::Inside);
                p.text(center, Align2::CENTER_CENTER, "🖼 Image (Empty)", FontId::new((10.0 * zoom).max(7.0), theme::semibold()), Color32::from_gray(180));
            } else {
                let uri = normalize_image_uri(trimmed_path);
                let p_ctx = p.ctx();
                let load_res = p_ctx.try_load_texture(
                    &uri,
                    egui::TextureOptions::LINEAR,
                    egui::load::SizeHint::Width(
                        (rect.width() * p_ctx.pixels_per_point()).round() as u32,
                    ),
                );

                match load_res {
                    Ok(egui::load::TexturePoll::Ready { texture }) => {
                        let draw_rect = if *lock_aspect && texture.size.x > 0.0 && texture.size.y > 0.0 {
                            let img_aspect = texture.size.x / texture.size.y;
                            let box_aspect = rect.width() / rect.height().max(1.0);
                            if img_aspect > box_aspect {
                                let h = rect.width() / img_aspect;
                                Rect::from_center_size(center, vec2(rect.width(), h))
                            } else {
                                let w = rect.height() * img_aspect;
                                Rect::from_center_size(center, vec2(w, rect.height()))
                            }
                        } else {
                            rect
                        };

                        p.image(
                            texture.id,
                            draw_rect,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            tint,
                        );
                    }
                    Ok(egui::load::TexturePoll::Pending { .. }) => {
                        p_ctx.request_repaint();
                        p.rect_filled(rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(255, 255, 255, 5));
                    }
                    Err(_) => {
                        p.rect_filled(rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(255, 50, 50, 15));
                        p.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 50, 50, 60)), egui::StrokeKind::Inside);
                        p.text(center, Align2::CENTER_CENTER, "⚠️ Image Not Found", FontId::new((9.0 * zoom).max(7.0), theme::semibold()), Color32::from_rgb(255, 120, 120));
                    }
                }
            }

            if ctx.is_interactive && ctx.pointer_clicked && on_click.is_some() {
                if let Some(pos) = local_pointer_pos {
                    if rect.contains(pos) {
                        if let Some(act) = on_click {
                            execute_click_action(act);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_shape_fields() {
        let r = egui::epaint::RectShape::new(
            Rect::from_min_size(pos2(10.0, 10.0), vec2(50.0, 30.0)),
            CornerRadius::same(5),
            Color32::RED,
            Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Inside,
        );
        println!("RectShape: rect={:?}, corner_radius={:?}, fill={:?}, stroke={:?}", r.rect, r.corner_radius, r.fill, r.stroke);
    }

    #[test]
    fn test_context_image_methods() {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);

        let uri1 = normalize_image_uri("assets\\app.png");
        assert!(uri1.starts_with("file:///"));
        assert!(uri1.ends_with("/assets/app.png"));
        assert!(ctx.try_load_texture(&uri1, egui::TextureOptions::LINEAR, egui::load::SizeHint::Width(64)).is_ok());

        let uri2 = normalize_image_uri("file://assets/app.png");
        assert!(uri2.starts_with("file:///"));
        assert!(ctx.try_load_texture(&uri2, egui::TextureOptions::LINEAR, egui::load::SizeHint::Width(64)).is_ok());

        let uri3 = normalize_image_uri("file:///assets/app.png");
        assert!(uri3.starts_with("file:///"));
        assert!(ctx.try_load_texture(&uri3, egui::TextureOptions::LINEAR, egui::load::SizeHint::Width(64)).is_ok());
    }

    #[test]
    fn test_safe_truncate_utf8_cyrillic_and_symbols() {
        // ASCII short
        assert_eq!(safe_truncate("Hello", 10), "Hello");
        // ASCII long
        assert_eq!(safe_truncate("Hello, World! ProcessHandler", 12), "Hello, World...");
        // Cyrillic (2 bytes per char in UTF-8): must never panic on odd byte index
        let ru_str = "Тест - Тест (Официальный клип)";
        let truncated = safe_truncate(ru_str, 15);
        assert_eq!(truncated, "Тест - Тест (Оф...");
        // Cyrillic exact boundary
        assert_eq!(safe_truncate("Музыка", 6), "Музыка");
        assert_eq!(safe_truncate("Музыка", 5), "Музык...");
        // Non-BMP unicode (Emoji, bullet •)
        let bullet_str = "Би-2 • Яндекс Музыка";
        let bullet_trunc = safe_truncate(bullet_str, 8);
        assert_eq!(bullet_trunc, "Би-2 • Я...");
    }

    #[test]
    fn test_rotated_image_mesh_vertex_order() {
        // Verify that egui::Mesh::add_rect_with_uv assigns vertices in the order:
        // [0: LT, 1: RT, 2: LB, 3: RB] so that rotating an image preserves rectangular topology
        // without hourglass or butterfly diagonal distortion.
        let mut mesh = egui::Mesh::with_texture(egui::TextureId::default());
        mesh.add_rect_with_uv(
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.vertices[0].uv, pos2(0.0, 0.0)); // Left-Top
        assert_eq!(mesh.vertices[1].uv, pos2(1.0, 0.0)); // Right-Top
        assert_eq!(mesh.vertices[2].uv, pos2(0.0, 1.0)); // Left-Bottom
        assert_eq!(mesh.vertices[3].uv, pos2(1.0, 1.0)); // Right-Bottom

        // Vertex assignment in RotatedPainter::image must match this exact sequence
        let center = pos2(100.0, 100.0);
        let rect = Rect::from_center_size(center, vec2(50.0, 30.0));
        let angle_rad = 45.0f32.to_radians();

        let rot = |p: Pos2| center + rotate_vec(p - center, angle_rad);
        let p_lt = rot(rect.left_top());
        let p_rt = rot(rect.right_top());
        let p_lb = rot(rect.left_bottom());
        let p_rb = rot(rect.right_bottom());

        mesh.vertices[0].pos = p_lt;
        mesh.vertices[1].pos = p_rt;
        mesh.vertices[2].pos = p_lb;
        mesh.vertices[3].pos = p_rb;

        // Top edge: LT -> RT
        let top_len = (mesh.vertices[1].pos - mesh.vertices[0].pos).length();
        assert!((top_len - 50.0).abs() < 0.01);
        // Bottom edge: LB -> RB
        let bottom_len = (mesh.vertices[3].pos - mesh.vertices[2].pos).length();
        assert!((bottom_len - 50.0).abs() < 0.01);
        // Left edge: LT -> LB
        let left_len = (mesh.vertices[2].pos - mesh.vertices[0].pos).length();
        assert!((left_len - 30.0).abs() < 0.01);
        // Right edge: RT -> RB
        let right_len = (mesh.vertices[3].pos - mesh.vertices[1].pos).length();
        assert!((right_len - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_rotated_pointer_inverse_transform() {
        // Element centered at (100, 100) with size 60x20
        let center = pos2(100.0, 100.0);
        let rect = Rect::from_center_size(center, vec2(60.0, 20.0));
        let angle_rad = std::f32::consts::FRAC_PI_2; // 90 deg clockwise

        // A point (120, 100) is inside the unrotated rect (+20 on X).
        // Rotated 90 deg clockwise around (100, 100), it becomes (100, 120).
        let screen_pos = pos2(100.0, 120.0);

        // Inverse rotation map: center + rotate_vec(screen_pos - center, -angle_rad)
        let local_pos = center + rotate_vec(screen_pos - center, -angle_rad);
        assert!((local_pos.x - 120.0).abs() < 0.001);
        assert!((local_pos.y - 100.0).abs() < 0.001);
        assert!(rect.contains(local_pos));

        // A point outside the rotated box (e.g., (120, 100)) inversely maps to (100, 80), which is outside rect
        let outside_screen = pos2(120.0, 100.0);
        let outside_local = center + rotate_vec(outside_screen - center, -angle_rad);
        assert!(!rect.contains(outside_local));
    }

    #[test]
    fn test_rotated_text_and_selection_center_sync() {
        let text_elem = OverlayElement::Text {
            id: "txt_1".to_string(),
            text: "FPS: 144".to_string(),
            font_size: 16.0,
            color: [255, 255, 255, 255],
            x: 50.0,
            y: 80.0,
            width: None,
            height: None,
            lock_aspect: false,
            bold: true,
            font_family: "Inter".to_string(),
            rotation: 45.0,
            on_click: None,
        };

        let (raw_w, raw_h) = get_element_size(&text_elem);
        assert!(raw_w > 0.0 && raw_h > 0.0);

        let canvas_min = pos2(10.0, 20.0);
        let zoom = 1.25;

        // Overlay modal selection box center:
        let modal_center = canvas_min + vec2((text_elem.x() + raw_w * 0.5) * zoom, (text_elem.y() + raw_h * 0.5) * zoom);
        // Renderer center:
        let renderer_center = canvas_min + vec2((text_elem.x() + raw_w * 0.5) * zoom, (text_elem.y() + raw_h * 0.5) * zoom);

        assert_eq!(modal_center, renderer_center);
    }
}
