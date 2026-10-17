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

    ui.scope(|ui| {
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

            let name = match cell.count {
                Some((n, _)) => format!("{} {n}", cell.label),
                None => cell.label.to_string(),
            };
            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &name));
            if resp.on_hover_text(cell.tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
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
    resp.on_hover_text(tip)
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
    Clone,
    Expand,
}

/// Small outlined square button with a drawn glyph (X, Clone, Expand); inverts on hover.
pub fn glyph_button(ui: &mut Ui, glyph: Glyph, tip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(theme::CONTROL_H, theme::CONTROL_H), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    if hovered {
        p.rect_filled(rect, CornerRadius::ZERO, t.invert_bg);
    } else {
        p.rect_stroke(rect, CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);
    }
    let c = if hovered { t.invert_fg } else { t.muted };
    let m = rect.center();
    match glyph {
        Glyph::Close => {
            let s = 4.5;
            p.line_segment([m + vec2(-s, -s), m + vec2(s, s)], Stroke::new(1.5, c));
            p.line_segment([m + vec2(-s, s), m + vec2(s, -s)], Stroke::new(1.5, c));
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
            let s = 4.2;
            p.line_segment([m + vec2(s - 3.2, -s), m + vec2(s, -s)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(s, -s), m + vec2(s, -s + 3.2)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(-s, s - 3.2), m + vec2(-s, s)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(-s, s), m + vec2(-s + 3.2, s)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(0.5, -0.5), m + vec2(s - 0.5, -s + 0.5)], Stroke::new(1.3, c));
            p.line_segment([m + vec2(-0.5, 0.5), m + vec2(-s + 0.5, s - 0.5)], Stroke::new(1.3, c));
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand)
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
    resp.on_hover_text(tip).on_hover_cursor(cursor)
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
