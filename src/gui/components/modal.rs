use eframe::emath::vec2;
use egui::{
    pos2, Color32, CornerRadius, Id, Order, Rect, Sense, Stroke, Ui, Vec2,
};
use crate::gui::theme::{self, Tokens};

/// Response returned from `Modal::show`.
pub struct ModalResponse {
    /// True if the user pressed Esc, clicked the scrim, or an internal close button was triggered.
    pub close_requested: bool,
    #[cfg(test)]
    pub anim: f32,
}

/// A structured frame passed to the modal's contents closure, providing isolated `Ui` scopes
/// for the header, toolbar, body, and footer slices.
pub struct ModalFrame<'a> {
    pub ui: &'a mut Ui,
    pub header_rect: Option<Rect>,
    pub toolbar_rect: Option<Rect>,
    pub body_rect: Rect,
    pub footer_rect: Option<Rect>,
    pub close_requested: &'a mut bool,
}

impl<'a> ModalFrame<'a> {
    /// Render content in the header slice (if `header_height` was specified).
    /// Provides `&mut Ui` scoped to the header rect and `&mut bool` to trigger close.
    pub fn header<R>(&mut self, add_contents: impl FnOnce(&mut Ui, &mut bool) -> R) -> Option<R> {
        let rect = self.header_rect?;
        let close_req = &mut *self.close_requested;
        Some(self.ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.set_width(rect.width());
            ui.set_height(rect.height());
            ui.set_clip_rect(rect);
            add_contents(ui, close_req)
        }).inner)
    }

    /// Render content in the secondary toolbar slice below the header (if `toolbar_height` was specified).
    /// Provides `&mut Ui` scoped to the toolbar rect and `&mut bool` to trigger close.
    pub fn toolbar<R>(&mut self, add_contents: impl FnOnce(&mut Ui, &mut bool) -> R) -> Option<R> {
        let rect = self.toolbar_rect?;
        let close_req = &mut *self.close_requested;
        Some(self.ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.set_width(rect.width());
            ui.set_height(rect.height());
            ui.set_clip_rect(rect);
            add_contents(ui, close_req)
        }).inner)
    }

    /// Render content in the main body area.
    pub fn body<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.ui.scope_builder(egui::UiBuilder::new().max_rect(self.body_rect), |ui| {
            ui.set_width(self.body_rect.width());
            ui.set_height(self.body_rect.height());
            ui.set_clip_rect(self.body_rect);
            add_contents(ui)
        }).inner
    }

    /// Render content in the footer slice (if `footer_height` was specified).
    /// Provides `&mut Ui` scoped to the footer rect and `&mut bool` to trigger close.
    pub fn footer<R>(&mut self, add_contents: impl FnOnce(&mut Ui, &mut bool) -> R) -> Option<R> {
        let rect = self.footer_rect?;
        let close_req = &mut *self.close_requested;
        Some(self.ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.set_width(rect.width());
            ui.set_height(rect.height());
            ui.set_clip_rect(rect);
            add_contents(ui, close_req)
        }).inner)
    }
}

/// Unified, high-performance base modal dialog builder.
///
/// Features:
/// - **Modern Cinematic animation**: graceful rise (`+24px -> 0px`), depth zoom (`0.92 -> 1.00`),
///   and smooth card opacity fade (`0.0 -> 1.0`) preventing harsh dark pop-in.
/// - **Asymmetric timing**: smooth 240ms open deceleration, fast 160ms close response.
/// - **Consistent theming**: automatic depth shadow, background fills, border strokes, hairline dividers,
///   and optional signature top accent stripe.
/// - **Safe keyboard and backdrop interaction**: Escape key & backdrop click handling with optional inhibition.
pub struct Modal {
    id_str: &'static str,
    is_open: bool,
    width_fraction: f32,
    height_fraction: f32,
    min_size: Vec2,
    max_size: Vec2,
    close_on_escape: bool,
    close_on_scrim: bool,
    scrim_max_alpha: u8,
    open_duration: f32,
    close_duration: f32,
    header_height: Option<f32>,
    toolbar_height: Option<f32>,
    footer_height: Option<f32>,
    header_sunken: bool,
    toolbar_sunken: bool,
    body_sunken: bool,
    footer_sunken: bool,
}

impl Modal {
    pub fn new(id_str: &'static str, is_open: bool) -> Self {
        Self {
            id_str,
            is_open,
            width_fraction: 0.80,
            height_fraction: 0.80,
            min_size: vec2(580.0, 380.0),
            max_size: vec2(1180.0, 840.0),
            close_on_escape: true,
            close_on_scrim: true,
            scrim_max_alpha: 215,
            open_duration: 0.24,
            close_duration: 0.16,
            header_height: None,
            toolbar_height: None,
            footer_height: None,
            header_sunken: false,
            toolbar_sunken: false,
            body_sunken: false,
            footer_sunken: false,
        }
    }

    pub fn width_fraction(mut self, frac: f32) -> Self {
        self.width_fraction = frac.clamp(0.2, 0.98);
        self
    }

    pub fn height_fraction(mut self, frac: f32) -> Self {
        self.height_fraction = frac.clamp(0.2, 0.98);
        self
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }

    pub fn max_size(mut self, size: Vec2) -> Self {
        self.max_size = size;
        self
    }

    pub fn close_on_escape(mut self, allow: bool) -> Self {
        self.close_on_escape = allow;
        self
    }

    pub fn close_on_scrim(mut self, allow: bool) -> Self {
        self.close_on_scrim = allow;
        self
    }

    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = Some(height);
        self
    }

    pub fn toolbar_height(mut self, height: f32) -> Self {
        self.toolbar_height = Some(height);
        self
    }

    pub fn footer_height(mut self, height: f32) -> Self {
        self.footer_height = Some(height);
        self
    }

    pub fn header_sunken(mut self, sunken: bool) -> Self {
        self.header_sunken = sunken;
        self
    }

    #[allow(dead_code)]
    pub fn toolbar_sunken(mut self, sunken: bool) -> Self {
        self.toolbar_sunken = sunken;
        self
    }

    pub fn body_sunken(mut self, sunken: bool) -> Self {
        self.body_sunken = sunken;
        self
    }

    pub fn footer_sunken(mut self, sunken: bool) -> Self {
        self.footer_sunken = sunken;
        self
    }

    /// Renders the modal. Returns `Some(ModalResponse)` if the modal is currently visible or animating,
    /// or `None` if it is completely closed.
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        t: &Tokens,
        render_content: impl FnOnce(&mut ModalFrame<'_>) -> R,
    ) -> Option<ModalResponse> {
        let anim_id = Id::new(self.id_str).with("modal_cinematic_anim");

        // If this modal ID has never been seen before and is opening for the first time,
        // egui's `animate_bool` would default to 1.0 immediately (skipping the opening animation).
        // To ensure it smoothly animates 0.0 -> 1.0, prime the animation state to 0.0 on the very first frame.
        let seen_id = anim_id.with("initialized");
        let was_seen = ctx.data(|d| d.get_temp::<bool>(seen_id)).unwrap_or(false);
        if !was_seen {
            ctx.data_mut(|d| d.insert_temp(seen_id, true));
            if self.is_open {
                ctx.animate_bool_with_time_and_easing(
                    anim_id,
                    false,
                    0.0001,
                    egui::emath::easing::linear,
                );
            }
        }

        let anim = ctx.animate_bool_with_time_and_easing(
            anim_id,
            self.is_open,
            if self.is_open { self.open_duration } else { self.close_duration },
            egui::emath::easing::cubic_out,
        );

        if anim <= 0.001 && !self.is_open {
            // Once fully closed, remove initialization marker so subsequent opens are always guaranteed
            // to prime from 0.0 even if egui clears animation memory during idle.
            ctx.data_mut(|d| d.remove_temp::<bool>(seen_id));
            return None;
        }

        // Keep repainting while animating so motion is silky 60/144fps smooth
        if anim > 0.001 && anim < 0.999 {
            ctx.request_repaint();
        }

        let full = ctx.content_rect();

        // 1. Scrim overlay with quadratic opacity ease
        let mut close_requested = false;
        let scrim_alpha = (anim.powf(0.85) * (self.scrim_max_alpha as f32)).round() as u8;

        if scrim_alpha > 0 {
            let scrim_id = Id::new(self.id_str).with("scrim");
            ctx.move_to_top(egui::LayerId::new(Order::Middle, scrim_id));
            egui::Area::new(scrim_id)
                .order(Order::Middle)
                .fixed_pos(full.min)
                .show(ctx, |ui| {
                    let (rect, resp) = ui.allocate_exact_size(full.size(), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::ZERO,
                        Color32::from_black_alpha(scrim_alpha),
                    );
                    if resp.clicked() && self.close_on_scrim {
                        close_requested = true;
                    }
                });
        }

        // Close on Escape key if allowed
        if self.close_on_escape && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close_requested = true;
        }

        // 2. Cinematic motion curves:
        // - Scale: 0.92 -> 1.00 (noticeable physical depth)
        // - Y-shift: +24px -> 0px (graceful upward glide)
        // - Card opacity: 0.0 -> 1.0 (smoothly fades in, avoiding the harsh dark pop)
        let scale = 0.92 + 0.08 * anim;
        let y_shift = (1.0 - anim) * 24.0;
        let card_alpha = anim;

        let raw_w = (full.width() * self.width_fraction)
            .clamp(self.min_size.x, self.max_size.x)
            .min(full.width() - 32.0);
        let raw_h = (full.height() * self.height_fraction)
            .clamp(self.min_size.y, self.max_size.y)
            .min(full.height() - 48.0);
        let center = full.center();

        let modal_w = (raw_w * scale).round();
        let modal_h = (raw_h * scale).round();
        let min_x = (center.x - modal_w * 0.5).round();
        let min_y = (center.y + y_shift - modal_h * 0.5).round();
        let modal_rect = Rect::from_min_size(pos2(min_x, min_y), vec2(modal_w, modal_h));

        // 3. Slice layout geometry
        let header_h = self.header_height.unwrap_or(0.0);
        let toolbar_h = self.toolbar_height.unwrap_or(0.0);
        let footer_h = self.footer_height.unwrap_or(0.0);

        let header_rect = if header_h > 0.0 {
            Some(Rect::from_min_size(modal_rect.min, vec2(modal_rect.width(), header_h)))
        } else {
            None
        };

        let toolbar_rect = if toolbar_h > 0.0 {
            let top = modal_rect.top() + header_h;
            Some(Rect::from_min_size(pos2(modal_rect.left(), top), vec2(modal_rect.width(), toolbar_h)))
        } else {
            None
        };

        let footer_rect = if footer_h > 0.0 {
            let top = modal_rect.bottom() - footer_h;
            Some(Rect::from_min_size(pos2(modal_rect.left(), top), vec2(modal_rect.width(), footer_h)))
        } else {
            None
        };

        let body_top = modal_rect.top() + header_h + toolbar_h;
        let body_bottom = modal_rect.bottom() - footer_h;
        let body_rect = Rect::from_min_max(
            pos2(modal_rect.left(), body_top),
            pos2(modal_rect.right(), body_bottom),
        );

        // 4. Modal Surface Window (Foreground order ensures child ComboBox & popups render on top)
        let window_id = Id::new(self.id_str).with("window");
        egui::Area::new(window_id)
            .order(Order::Foreground)
            .fixed_pos(modal_rect.min)
            .show(ctx, |ui| {
                ui.set_width(modal_rect.width());
                ui.set_height(modal_rect.height());
                ui.set_max_width(modal_rect.width());
                ui.set_max_height(modal_rect.height());
                // Expand clip rect so the 24px drop shadow and 1px outer stroke are never clipped
                ui.set_clip_rect(modal_rect.expand(32.0));

                // 1. Paint background surfaces and precision dividers first
                {
                    let p = ui.painter();

                    // Blooming drop shadow
                    if card_alpha > 0.02 {
                        let mut shadow = theme::shadow(t);
                        shadow.color = shadow.color.gamma_multiply(card_alpha);
                        p.add(shadow.as_shape(modal_rect, CornerRadius::same(2)));
                    }

                    // Base background
                    p.rect_filled(modal_rect, CornerRadius::same(2), t.bg.gamma_multiply(card_alpha));

                    // Header fill
                    if let Some(h_rect) = header_rect {
                        let fill = if self.header_sunken { t.sunken } else { t.bg };
                        p.rect_filled(h_rect, CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }, fill.gamma_multiply(card_alpha));
                    }

                    // Toolbar fill
                    if let Some(tb_rect) = toolbar_rect {
                        let fill = if self.toolbar_sunken { t.sunken } else { t.bg };
                        p.rect_filled(tb_rect, CornerRadius::ZERO, fill.gamma_multiply(card_alpha));
                    }

                    // Body fill if sunken
                    if self.body_sunken {
                        let cr = if footer_rect.is_some() {
                            CornerRadius::ZERO
                        } else {
                            CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 }
                        };
                        p.rect_filled(body_rect, cr, t.sunken.gamma_multiply(card_alpha));
                    }

                    // Footer fill
                    if let Some(f_rect) = footer_rect {
                        let fill = if self.footer_sunken { t.sunken } else { t.bg };
                        p.rect_filled(f_rect, CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 }, fill.gamma_multiply(card_alpha));
                    }
                }

                // 2. Render inner contents
                let mut frame = ModalFrame {
                    ui,
                    header_rect,
                    toolbar_rect,
                    body_rect,
                    footer_rect,
                    close_requested: &mut close_requested,
                };
                render_content(&mut frame);

                // 3. CRITICAL: Paint dividers, outer border stroke and top accent bar AFTER all inner contents
                // so no inner fills, table rows, frames, or sidebars can ever obscure or overwrite them!
                {
                    let p = ui.painter();
                    let div_color = t.hairline().color.gamma_multiply(card_alpha);

                    // Header divider (under header)
                    if let Some(h_rect) = header_rect {
                        let div_rect = Rect::from_min_max(
                            pos2(modal_rect.left() + 1.0, h_rect.bottom()),
                            pos2(modal_rect.right() - 1.0, h_rect.bottom() + 1.0),
                        );
                        p.rect_filled(div_rect, CornerRadius::ZERO, div_color);
                    }

                    // Toolbar divider (under toolbar)
                    if let Some(tb_rect) = toolbar_rect {
                        let div_rect = Rect::from_min_max(
                            pos2(modal_rect.left() + 1.0, tb_rect.bottom()),
                            pos2(modal_rect.right() - 1.0, tb_rect.bottom() + 1.0),
                        );
                        p.rect_filled(div_rect, CornerRadius::ZERO, div_color);
                    }

                    // Footer divider (above footer)
                    if let Some(f_rect) = footer_rect {
                        let div_rect = Rect::from_min_max(
                            pos2(modal_rect.left() + 1.0, f_rect.top() - 1.0),
                            pos2(modal_rect.right() - 1.0, f_rect.top()),
                        );
                        p.rect_filled(div_rect, CornerRadius::ZERO, div_color);
                    }

                    // Outer border stroke
                    p.rect_stroke(
                        modal_rect,
                        CornerRadius::same(2),
                        Stroke::new(1.0, t.line_strong.gamma_multiply(card_alpha)),
                        egui::StrokeKind::Inside,
                    );
                }
            });

        Some(ModalResponse {
            close_requested,
            #[cfg(test)]
            anim,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_animation_starts_from_zero_on_first_open_and_reopen() {
        let ctx = egui::Context::default();

        // 1. First open: must start from 0.0, NOT 1.0
        let raw_input = egui::RawInput { time: Some(0.0), ..Default::default() };
        let mut out = ctx.run_ui(raw_input, |ui| {
            let modal = Modal::new("test-modal-first-open", true);
            let res = modal.show(ui.ctx(), &Tokens::DARK, |_frame| {});
            assert!(res.is_some());
            let res = res.unwrap();
            assert!(res.anim < 0.2, "Expected anim to start near 0.0, but got {}", res.anim);
        });
        out.textures_delta.clear();

        // 2. Advance time until fully open (anim == 1.0)
        let raw_input = egui::RawInput { time: Some(0.35), ..Default::default() };
        let mut out = ctx.run_ui(raw_input, |ui| {
            let modal = Modal::new("test-modal-first-open", true);
            let res = modal.show(ui.ctx(), &Tokens::DARK, |_frame| {});
            assert!(res.is_some());
            let res = res.unwrap();
            assert!((res.anim - 1.0).abs() < 0.01, "Expected anim to reach 1.0, got {}", res.anim);
        });
        out.textures_delta.clear();

        // 3. Close modal and advance time until fully closed (returns None)
        let raw_input = egui::RawInput { time: Some(0.60), ..Default::default() };
        let mut out = ctx.run_ui(raw_input, |ui| {
            let modal = Modal::new("test-modal-first-open", false);
            let res = modal.show(ui.ctx(), &Tokens::DARK, |_frame| {});
            assert!(res.is_none(), "Expected modal to return None when fully closed");
        });
        out.textures_delta.clear();

        // 4. Reopen modal: must start from 0.0 again!
        let raw_input = egui::RawInput { time: Some(1.0), ..Default::default() };
        let mut out = ctx.run_ui(raw_input, |ui| {
            let modal = Modal::new("test-modal-first-open", true);
            let res = modal.show(ui.ctx(), &Tokens::DARK, |_frame| {});
            assert!(res.is_some());
            let res = res.unwrap();
            assert!(res.anim < 0.2, "Expected reopened modal anim to start near 0.0, but got {}", res.anim);
        });
        out.textures_delta.clear();
    }
}
