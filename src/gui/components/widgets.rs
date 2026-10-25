//! Precision studio widgets: mono HUD labels, square buttons with hover inversion,
//! segmented hairline cell groups, section eyebrows, and custom window chrome.

use std::sync::Arc;
use egui::{
    Color32, CornerRadius, Galley, Rect, Response, Sense, Stroke, Ui, pos2, vec2,
};
use crate::gui::theme::{self, Tokens};

/// `text` laid out uppercase in `family`, `spacing` points between letters.
pub fn caps(
    ui: &Ui,
    text: &str,
    size: f32,
    family: egui::FontFamily,
    spacing: f32,
    color: Color32,
) -> Arc<Galley> {
    let format = egui::TextFormat {
        font_id: egui::FontId::new(size, family),
        color,
        extra_letter_spacing: spacing,
        ..Default::default()
    };
    ui.painter().layout_job(egui::text::LayoutJob::single_section(text.to_uppercase(), format))
}

/// A `.hud-label`: 11 px Geist Mono, uppercase, widely spaced.
pub fn hud_galley(ui: &Ui, text: &str, color: Color32) -> Arc<Galley> {
    caps(ui, text, theme::HUD_SIZE, theme::mono(), theme::HUD_SPACING, color)
}

/// Paints `galley` at the left of `rect`, centered vertically.
pub fn paint_left(ui: &Ui, rect: Rect, galley: Arc<Galley>, color: Color32) {
    let pos = pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley_with_override_text_color(pos, galley, color);
}

/// Paints `galley` centered in `rect`.
pub fn paint_centred(ui: &Ui, rect: Rect, galley: Arc<Galley>, color: Color32) {
    ui.painter().galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, color);
}

/// A precision mono HUD label.
pub fn hud(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let galley = hud_galley(ui, text, color);
    let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::hover());
    paint_left(ui, rect, galley, color);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
    resp
}

/// A mono label elided with "…" if wider than `max_width`. Full text on hover.
#[allow(dead_code)]
pub fn hud_elided(ui: &mut Ui, text: &str, color: Color32, max_width: f32) -> Response {
    let format = egui::TextFormat {
        font_id: egui::FontId::new(theme::HUD_SIZE, theme::mono()),
        color,
        extra_letter_spacing: theme::HUD_SPACING,
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::single_section(text.to_uppercase(), format);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width.min(ui.available_width()).max(0.0));
    let galley = ui.painter().layout_job(job);
    let elided = galley.elided;
    let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::hover());
    paint_left(ui, rect, galley, color);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
    if elided {
        resp.on_hover_text(text)
    } else {
        resp
    }
}

/// "01 / AUTOMATIONS": a section eyebrow, the index and separator faint.
pub fn eyebrow(ui: &mut Ui, index: &str, label: &str) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        hud(ui, index, t.faint);
        hud(ui, " / ", t.faint);
        hud(ui, label, t.muted);
    });
}

/// A small square status badge: 10 px mono on sunken surface with hairline border.
pub fn badge(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let t = Tokens::get(ui.ctx());
    let galley = caps(ui, text, 10.0, theme::mono(), 1.0, color);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 12.0, 20.0), Sense::hover());
    ui.painter().rect(rect, CornerRadius::ZERO, t.sunken, t.hairline(), egui::StrokeKind::Inside);
    paint_centred(ui, rect, galley, color);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
    resp
}

/// A vertical hairline divider.
pub fn rule(ui: &mut Ui, height: f32) {
    let t = Tokens::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, height), Sense::hover());
    ui.painter().vline(rect.center().x, rect.y_range(), t.hairline());
}

/// A horizontal hairline divider across available width.
pub fn hrule(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.top() + 0.5, t.hairline());
}

/// Which button variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Solid block in inverted colors (white bg in dark theme); dims to 80% on hover.
    Primary,
    /// Outlined; inverts on hover.
    Secondary,
    /// Solid red; dims on hover.
    Danger,
}

/// Helper to calculate exact button width including padding and spacing
pub fn button_width(ui: &Ui, text: &str, min_width: f32) -> f32 {
    let t = Tokens::get(ui.ctx());
    let galley = caps(ui, text, theme::HUD_SIZE, theme::mono_bold(), theme::BUTTON_SPACING, t.ink);
    (galley.size().x + 28.0).max(min_width)
}

/// A square button with a mono uppercase label, at least `width` wide.
pub fn button(ui: &mut Ui, text: &str, kind: Kind, width: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let galley = caps(ui, text, theme::HUD_SIZE, theme::mono_bold(), theme::BUTTON_SPACING, t.ink);
    let size = vec2((galley.size().x + 28.0).max(width), theme::CONTROL_H);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    let hovered = resp.hovered() && enabled;

    let (fill, stroke, color) = match kind {
        Kind::Primary => (
            t.invert_bg.gamma_multiply(if hovered { 0.85 } else { 1.0 }),
            Stroke::NONE,
            t.invert_fg,
        ),
        Kind::Danger => (
            t.bad.gamma_multiply(if hovered { 0.85 } else { 1.0 }),
            Stroke::NONE,
            Color32::WHITE,
        ),
        Kind::Secondary if hovered => (t.invert_bg, Stroke::NONE, t.invert_fg),
        Kind::Secondary => (t.bg, Stroke::new(1.0, t.line_strong), t.ink),
    };

    let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.4) };
    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        fade(fill),
        Stroke::new(stroke.width, fade(stroke.color)),
        egui::StrokeKind::Inside,
    );
    paint_centred(ui, rect, galley, fade(color));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// Precision toggle switch with smooth animated track and thumb, and hover feedback.
pub fn toggle_switch(ui: &mut Ui, on: &mut bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let width = 38.0;
    let height = 20.0;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let enabled = ui.is_enabled();
    let hovered = resp.hovered() && enabled;

    // Smooth ease-out animation for on/off transition
    let how_on = ui.ctx().animate_bool_responsive(resp.id, *on);

    let bg_color = {
        let off_bg = if hovered { t.line_strong } else { t.sunken };
        let on_bg = if hovered { t.accent.gamma_multiply(0.85) } else { t.accent };
        let r = egui::lerp((off_bg.r() as f32)..=(on_bg.r() as f32), how_on) as u8;
        let g = egui::lerp((off_bg.g() as f32)..=(on_bg.g() as f32), how_on) as u8;
        let b = egui::lerp((off_bg.b() as f32)..=(on_bg.b() as f32), how_on) as u8;
        let a = egui::lerp((off_bg.a() as f32)..=(on_bg.a() as f32), how_on) as u8;
        Color32::from_rgba_unmultiplied(r, g, b, a)
    };

    let border_stroke = if how_on >= 0.98 {
        Stroke::NONE
    } else {
        let border_color = if hovered { t.line_strong } else { t.line };
        Stroke::new(1.0, border_color.gamma_multiply(1.0 - how_on))
    };

    let corner_radius = CornerRadius::same(height as u8 / 2);
    ui.painter().rect(rect, corner_radius, bg_color, border_stroke, egui::StrokeKind::Inside);

    let thumb_radius = (height - 6.0) / 2.0;
    let min_x = rect.left() + 3.0 + thumb_radius;
    let max_x = rect.right() - 3.0 - thumb_radius;
    let x_pos = egui::lerp(min_x..=max_x, how_on);
    let center = pos2(x_pos, rect.center().y);

    let thumb_color = if how_on > 0.5 {
        Color32::WHITE
    } else if hovered {
        t.ink
    } else {
        t.muted
    };

    ui.painter().circle_filled(center, thumb_radius, thumb_color);

    if enabled {
        resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, *on, enabled, ""));
    resp
}

/// A compact button with specified height (e.g. for status/footer bars).
pub fn compact_button(ui: &mut Ui, text: &str, kind: Kind, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let galley = caps(ui, text, 10.0, theme::mono_bold(), theme::BUTTON_SPACING, t.ink);
    let size = vec2(galley.size().x + 14.0, height);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    let hovered = resp.hovered() && enabled;

    let (fill, stroke, color) = match kind {
        Kind::Primary => (
            t.accent.gamma_multiply(if hovered { 0.85 } else { 1.0 }),
            Stroke::NONE,
            Color32::WHITE,
        ),
        Kind::Danger => (
            t.bad.gamma_multiply(if hovered { 0.85 } else { 1.0 }),
            Stroke::NONE,
            Color32::WHITE,
        ),
        Kind::Secondary if hovered => (t.invert_bg, Stroke::NONE, t.invert_fg),
        Kind::Secondary => (t.sunken, Stroke::new(1.0, t.line_strong), t.ink),
    };

    let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.4) };
    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        fade(fill),
        Stroke::new(stroke.width, fade(stroke.color)),
        egui::StrokeKind::Inside,
    );
    paint_centred(ui, rect, galley, fade(color));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// One option in a segmented [`cells`] group.
pub struct Cell<'a> {
    pub label: &'a str,
    pub tip: &'a str,
    pub count: Option<(usize, Option<Color32>)>,
}

impl<'a> Cell<'a> {
    pub fn new(label: &'a str, tip: &'a str) -> Self {
        Self {
            label,
            tip,
            count: None,
        }
    }
}

/// Mutually exclusive options in a row of hairline cells. The chosen one is inverted.
pub fn cells<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, Cell<'_>)]) -> bool {
    let t = Tokens::get(ui.ctx());
    let mut changed = false;
    let mut rects = Vec::with_capacity(options.len());

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (opt, cell) in options {
            let on = *value == *opt;
            let label = hud_galley(ui, cell.label, t.ink);
            let count = cell.count.map(|(n, highlight)| {
                let c = if on {
                    t.invert_fg.gamma_multiply(0.65)
                } else {
                    highlight.filter(|_| n > 0).unwrap_or(t.faint)
                };
                (hud_galley(ui, &n.to_string(), c), c)
            });
            let w = label.size().x + count.as_ref().map_or(0.0, |(g, _)| g.size().x + 8.0) + 24.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(w, theme::CONTROL_H), Sense::click());
            let hovered = resp.hovered();

            let (fill, ink) = match (on, hovered) {
                (true, _) => (t.invert_bg, t.invert_fg),
                (false, true) => (t.sunken, t.ink_strong),
                (false, false) => (Color32::TRANSPARENT, t.muted),
            };

            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
            let text_rect = rect.shrink2(vec2(12.0, 0.0));
            let label_w = label.size().x;
            paint_left(ui, text_rect, label, ink);
            if let Some((g, c)) = count {
                paint_left(ui, text_rect.with_min_x(text_rect.left() + label_w + 8.0), g, c);
            }

            let resp = if !cell.tip.trim().is_empty() {
                resp.on_hover_text(cell.tip)
            } else {
                resp
            };
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
                *value = *opt;
                changed = true;
            }
            rects.push(rect);
        }
    });

    if let (Some(first), Some(last)) = (rects.first(), rects.last()) {
        let p = ui.painter();
        p.rect_stroke(first.union(*last), CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);
        let (top, bottom) = (first.top(), first.bottom());
        for r in rects.iter().skip(1) {
            p.vline(r.left(), top..=bottom, t.hairline());
        }
    }

    changed
}

const RESIZE_EDGE: f32 = 5.0;

/// Custom title bar dragging and double-click to maximize/restore.
pub fn title_bar_drag(ui: &mut Ui) {
    let r = ui.max_rect();
    let rect = Rect::from_min_max(r.min + vec2(RESIZE_EDGE, RESIZE_EDGE), r.max - vec2(RESIZE_EDGE, 0.0));
    let resp = ui.interact(rect, ui.id().with("title-bar-drag"), Sense::click_and_drag());
    if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if resp.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowButton {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Minimize, maximize (or restore) and close buttons for custom title bars.
pub fn window_buttons(ui: &mut Ui, height: f32, loc: &crate::i18n::Strings) {
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if window_button(ui, WindowButton::Close, height, loc.win_btn_close).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let (size, label) = if maximized {
        (WindowButton::Restore, loc.win_btn_restore)
    } else {
        (WindowButton::Maximize, loc.win_btn_maximize)
    };
    if window_button(ui, size, height, label).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if window_button(ui, WindowButton::Minimize, height, loc.win_btn_minimize).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
}

fn window_button(ui: &mut Ui, button: WindowButton, height: f32, tip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(46.0, height), Sense::hover());
    let right = if button == WindowButton::Close { RESIZE_EDGE } else { 0.0 };
    let hit = Rect::from_min_max(rect.min + vec2(0.0, RESIZE_EDGE), rect.max - vec2(right, 0.0));
    let resp = ui.interact(hit, ui.id().with(("window-button", button as u8)), Sense::click());

    let (fill, ink) = match (button, resp.hovered()) {
        (WindowButton::Close, true) => (Color32::from_rgb(0xc4, 0x2b, 0x1c), Color32::WHITE),
        (_, true) => (t.line, t.ink_strong),
        (_, false) => (Color32::TRANSPARENT, t.muted),
    };

    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::ZERO, fill);
    let c = rect.center();
    let stroke = Stroke::new(1.0, ink);

    let s = 5.0;
    match button {
        WindowButton::Minimize => {
            p.hline(c.x - s..=c.x + s, c.y, stroke);
        }
        WindowButton::Maximize => {
            p.rect_stroke(Rect::from_center_size(c, vec2(2.0 * s, 2.0 * s)), CornerRadius::ZERO, stroke, egui::StrokeKind::Inside);
        }
        WindowButton::Restore => {
            let front = Rect::from_min_size(c + vec2(-s, -s + 2.0), vec2(2.0 * s - 2.0, 2.0 * s - 2.0));
            p.rect_stroke(front, CornerRadius::ZERO, stroke, egui::StrokeKind::Inside);
            p.hline(c.x - s + 2.0..=c.x + s, c.y - s, stroke);
            p.vline(c.x + s, c.y - s..=c.y + s - 2.0, stroke);
        }
        WindowButton::Close => {
            p.line_segment([c + vec2(-s, -s), c + vec2(s, s)], stroke);
            p.line_segment([c + vec2(-s, s), c + vec2(s, -s)], stroke);
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    if !tip.trim().is_empty() {
        resp.on_hover_text(tip)
    } else {
        resp
    }
}

/// Custom title bars: hairline around the window and resizing from edges and corners.
pub fn window_edges(ctx: &egui::Context) {
    use egui::CursorIcon as C;
    use egui::viewport::ResizeDirection as D;
    let (maximized, fullscreen) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if maximized || fullscreen {
        return;
    }
    let t = Tokens::get(ctx);
    let r = ctx.content_rect();
    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("window-border"))).rect_stroke(
        r,
        CornerRadius::ZERO,
        t.hairline(),
        egui::StrokeKind::Inside,
    );
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()).filter(|p| r.contains(*p)) else { return };
    let near = |d: f32| d < RESIZE_EDGE;
    let corner = |d: f32| d < 3.0 * RESIZE_EDGE;
    let (left, right, top, bottom) = (p.x - r.left(), r.right() - p.x, p.y - r.top(), r.bottom() - p.y);
    let hit = if (near(top) && corner(left)) || (near(left) && corner(top)) {
        Some((D::NorthWest, C::ResizeNorthWest))
    } else if (near(top) && corner(right)) || (near(right) && corner(top)) {
        Some((D::NorthEast, C::ResizeNorthEast))
    } else if (near(bottom) && corner(left)) || (near(left) && corner(bottom)) {
        Some((D::SouthWest, C::ResizeSouthWest))
    } else if (near(bottom) && corner(right)) || (near(right) && corner(bottom)) {
        Some((D::SouthEast, C::ResizeSouthEast))
    } else if near(top) {
        Some((D::North, C::ResizeNorth))
    } else if near(bottom) {
        Some((D::South, C::ResizeSouth))
    } else if near(left) {
        Some((D::West, C::ResizeWest))
    } else if near(right) {
        Some((D::East, C::ResizeEast))
    } else {
        None
    };
    if let Some((direction, cursor)) = hit {
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Close,
    Minimize,
    Clone,
    Expand,
    More,
    Reset,
    Play,
    Question,
}

/// Small outlined square button with a drawn glyph (X, Clone, Expand, More, Reset); inverts on hover.
pub fn glyph_button(ui: &mut Ui, glyph: Glyph, tip: &str) -> Response {
    glyph_button_active(ui, glyph, false, tip)
}

/// Small outlined square button with a custom square size (e.g. 24.0 or 26.0 px).
#[allow(dead_code)]
pub fn glyph_button_size(ui: &mut Ui, glyph: Glyph, size: f32, tip: &str) -> Response {
    glyph_button_custom(ui, glyph, size, false, tip)
}

/// Small outlined square button with a drawn glyph; supports active state.
pub fn glyph_button_active(ui: &mut Ui, glyph: Glyph, active: bool, tip: &str) -> Response {
    glyph_button_custom(ui, glyph, theme::CONTROL_H, active, tip)
}

/// Small outlined square button with custom size and active state.
#[allow(dead_code)]
pub fn glyph_button_custom(ui: &mut Ui, glyph: Glyph, size: f32, active: bool, tip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    let (fill, stroke, c) = if active || hovered {
        (t.invert_bg, Stroke::NONE, t.invert_fg)
    } else {
        (t.bg, Stroke::new(1.0, t.line_strong), t.ink)
    };
    p.rect(rect, CornerRadius::ZERO, fill, stroke, egui::StrokeKind::Inside);
    let m = rect.center();
    match glyph {
        Glyph::Close => {
            let s = (size * 0.15).clamp(3.5, 4.5);
            p.line_segment([m + vec2(-s, -s), m + vec2(s, s)], Stroke::new(1.5, c));
            p.line_segment([m + vec2(-s, s), m + vec2(s, -s)], Stroke::new(1.5, c));
        }
        Glyph::Minimize => {
            let s = (size * 0.18).clamp(4.0, 5.0);
            p.line_segment([m + vec2(-s, 0.0), m + vec2(s, 0.0)], Stroke::new(1.5, c));
        }
        Glyph::Clone => {
            let back = Rect::from_center_size(m + vec2(-2.0, -2.0), vec2(7.0, 7.0));
            let front = Rect::from_center_size(m + vec2(1.8, 1.8), vec2(7.0, 7.0));
            p.line_segment([back.left_top(), back.right_top()], Stroke::new(1.2, c));
            p.line_segment([back.left_top(), back.left_bottom()], Stroke::new(1.2, c));
            p.line_segment([back.right_top(), back.right_top() + vec2(0.0, 2.0)], Stroke::new(1.2, c));
            p.line_segment([back.left_bottom(), back.left_bottom() + vec2(2.0, 0.0)], Stroke::new(1.2, c));
            p.rect_stroke(front, CornerRadius::ZERO, Stroke::new(1.2, c), egui::StrokeKind::Inside);
        }
        Glyph::Expand => {
            let s = (size * 0.14).clamp(3.2, 4.2);
            p.line_segment([m + vec2(s - 3.2, -s), m + vec2(s, -s)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(s, -s), m + vec2(s, -s + 3.2)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(-s, s - 3.2), m + vec2(-s, s)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(-s, s), m + vec2(-s + 3.2, s)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(0.5, -0.5), m + vec2(s - 0.5, -s + 0.5)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(-0.5, 0.5), m + vec2(-s + 0.5, s - 0.5)], Stroke::new(1.3, c));
        }
        Glyph::More => {
            let r = 1.35;
            p.circle_filled(m + vec2(0.0, -4.5), r, c);
            p.circle_filled(m, r, c);
            p.circle_filled(m + vec2(0.0, 4.5), r, c);
        }
        Glyph::Reset => {
            let font_size = (size * 0.48).clamp(12.0, 15.0);
            p.text(
                m,
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
                egui::FontId::new(font_size, theme::phosphor()),
                c,
            );
        }
        Glyph::Play => {
            let font_size = (size * 0.48).clamp(12.0, 15.0);
            p.text(
                m,
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::PLAY,
                egui::FontId::new(font_size, theme::phosphor()),
                c,
            );
        }
        Glyph::Question => {
            let font_size = (size * 0.48).clamp(12.0, 15.0);
            p.text(
                m,
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::QUESTION_MARK,
                egui::FontId::new(font_size, theme::phosphor()),
                c,
            );
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    let resp = if !tip.trim().is_empty() {
        resp.on_hover_text(tip)
    } else {
        resp
    };
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A precision drag handle with 6 studio dots; senses click and drag.
pub fn drag_handle(ui: &mut Ui, tip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(18.0, theme::CONTROL_H), Sense::click_and_drag());
    let hovered = resp.hovered();
    let dragged = resp.dragged();
    let p = ui.painter();
    if hovered || dragged {
        p.rect_filled(rect, CornerRadius::ZERO, t.sunken);
    }
    let c = if dragged {
        t.accent_ink
    } else if hovered {
        t.ink
    } else {
        t.muted.gamma_multiply(0.6)
    };
    let m = rect.center();
    let dx = 2.8;
    let dy = 3.8;
    for x in [-dx, dx] {
        for y in [-dy, 0.0, dy] {
            p.circle_filled(m + vec2(x, y), 1.2, c);
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    let cursor = if dragged { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab };
    let resp = if !tip.trim().is_empty() {
        resp.on_hover_text(tip)
    } else {
        resp
    };
    resp.on_hover_cursor(cursor)
}

/// App wordmark in display font with accent color suffix, vertically centered in container of given height.
pub fn wordmark(ui: &mut Ui, prefix: &str, suffix: &str, size: f32, height: f32) -> Response {
    let t = Tokens::get(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    let format = |color| egui::TextFormat {
        font_id: egui::FontId::new(size, theme::display()),
        color,
        extra_letter_spacing: -0.03 * size,
        ..Default::default()
    };
    job.append(prefix, 0.0, format(t.ink_strong));
    job.append(suffix, 0.0, format(t.accent_ink));
    let galley = ui.painter().layout_job(job);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x, height), Sense::hover());
    let pos = pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, Color32::WHITE);
    resp
}

/// Groups vertical checkboxes with tight, compact desktop line spacing (~20px height, 4px vertical gap).
pub fn checkbox_group<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let old_h = ui.spacing().interact_size.y;
    let old_spacing_y = ui.spacing().item_spacing.y;
    ui.spacing_mut().interact_size.y = 20.0;
    ui.spacing_mut().item_spacing.y = 4.0;
    let ret = add_contents(ui);
    ui.spacing_mut().interact_size.y = old_h;
    ui.spacing_mut().item_spacing.y = old_spacing_y;
    ret
}

/// A compact checkbox that does not inflate to 34px CONTROL_H.
pub fn checkbox(ui: &mut Ui, checked: &mut bool, text: impl Into<egui::WidgetText>) -> Response {
    let old_h = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = 20.0;
    let resp = ui.checkbox(checked, text);
    ui.spacing_mut().interact_size.y = old_h;
    resp
}
