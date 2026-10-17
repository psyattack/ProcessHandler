//! Expanded Script Canvas Modal:
//! Centered studio canvas for editing PowerShell and CMD scripts
//! with syntax highlighting, line metrics, clipboard copy, and smooth animations.

use egui::{
    CornerRadius, Id, Order, Rect, Sense, Stroke, emath::easing, pos2, vec2,
};
use crate::gui::components::script_highlighter::create_script_layouter;
use crate::gui::components::widgets::{self, Glyph, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::Action;

pub fn render_script_editor_canvas(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.expanded_script_editor.is_some();

    // Handle Esc key to close modal
    if is_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.close_script_editor();
    }

    let anim = ctx.animate_bool_with_time_and_easing(
        Id::new("script-canvas-modal-anim"),
        is_open,
        0.20,
        easing::cubic_out,
    );

    if anim <= 0.001 && !is_open {
        state.last_script_editor = None;
        return;
    }

    let active_editor = state
        .expanded_script_editor
        .as_ref()
        .or(state.last_script_editor.as_ref())
        .cloned();

    let Some(modal) = active_editor else { return };

    let full = ctx.content_rect();

    // 1. Full-screen scrim with subtle fade
    let scrim_alpha = (anim * 195.0) as u8;
    egui::Area::new(Id::new("script-modal-scrim"))
        .order(Order::Foreground)
        .fixed_pos(full.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(full.size(), Sense::click());
            ui.painter().rect_filled(
                rect,
                CornerRadius::ZERO,
                egui::Color32::from_black_alpha(scrim_alpha),
            );
            if resp.clicked() {
                state.close_script_editor();
            }
        });

    // 2. Centered Canvas geometry
    let modal_w = (full.width() * 0.84).clamp(620.0, 1150.0).min(full.width() - 32.0);
    let modal_h = (full.height() * 0.82).clamp(420.0, 840.0).min(full.height() - 48.0);
    let center = full.center();

    let scale = 0.95 + 0.05 * anim;
    let y_shift = (1.0 - anim) * 12.0;
    let modal_rect = Rect::from_center_size(
        pos2(center.x, center.y + y_shift),
        vec2(modal_w * scale, modal_h * scale),
    );

    let header_h = 52.0;
    let footer_h = 52.0;
    let header_rect = Rect::from_min_size(modal_rect.min, vec2(modal_rect.width(), header_h));
    let footer_rect = Rect::from_min_size(
        pos2(modal_rect.left(), modal_rect.bottom() - footer_h),
        vec2(modal_rect.width(), footer_h),
    );
    let body_rect = Rect::from_min_max(
        pos2(modal_rect.left(), header_rect.bottom()),
        pos2(modal_rect.right(), footer_rect.top()),
    );

    // Retrieve script string from state.editing_rule
    let mut script_buffer = String::new();
    let mut found = false;
    if let Some(rule) = &state.editing_rule {
        let actions = if modal.is_exit_pipeline {
            &rule.on_exit_actions
        } else {
            &rule.actions
        };
        if let Some(action) = actions.get(modal.step_idx) {
            match action {
                Action::PowerShell { script, .. } => {
                    script_buffer = script.clone();
                    found = true;
                }
                Action::Cmd { command, .. } => {
                    script_buffer = command.clone();
                    found = true;
                }
                _ => {}
            }
        }
    }

    if !found && is_open {
        state.close_script_editor();
        return;
    }

    let mut close_clicked = false;
    let mut copy_clicked = false;
    let mut text_changed = false;

    // 3. Modal Canvas Window
    egui::Area::new(Id::new("script-modal-window"))
        .order(Order::Tooltip)
        .fixed_pos(modal_rect.min)
        .show(ctx, |ui| {
            ui.set_width(modal_rect.width());
            ui.set_height(modal_rect.height());
            ui.set_max_width(modal_rect.width());
            ui.set_max_height(modal_rect.height());
            ui.set_clip_rect(modal_rect);

            // Paint background surfaces and precision dividers
            {
                let p = ui.painter();
                // Elevated studio drop shadow
                p.add(theme::shadow(&t).as_shape(modal_rect, CornerRadius::same(2)));

                // Panel background & precision border
                p.rect_filled(modal_rect, CornerRadius::same(2), t.bg);
                p.rect_stroke(
                    modal_rect,
                    CornerRadius::same(2),
                    Stroke::new(1.0, t.line_strong),
                    egui::StrokeKind::Inside,
                );

                // 1. Header zone background and bottom hairline
                p.rect_filled(header_rect, CornerRadius::ZERO, t.bg);
                p.hline(header_rect.x_range(), header_rect.bottom(), t.hairline());
                p.hline(modal_rect.x_range(), modal_rect.top() + 1.0, Stroke::new(2.5, t.accent_ink));

                // 2. Editor body sunken well
                p.rect_filled(body_rect, CornerRadius::ZERO, t.sunken);

                // 3. Footer zone background and top hairline
                p.rect_filled(footer_rect, CornerRadius::ZERO, t.bg);
                p.hline(footer_rect.x_range(), footer_rect.top(), t.hairline());
            }

            // --- 1. HEADER ZONE ---
            ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    let prefix = if modal.is_exit_pipeline { "EXIT STEP" } else { "STEP" };
                    widgets::hud(ui, &format!("{} {:02}", prefix, modal.step_idx + 1), t.accent_ink);
                    ui.add_space(8.0);
                    widgets::badge(ui, modal.language.badge_name(), t.accent_ink);
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(modal.language.subtitle())
                            .family(theme::medium())
                            .size(13.0)
                            .color(t.muted),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if widgets::glyph_button(ui, Glyph::Close, "Close canvas (Esc)").clicked() {
                            close_clicked = true;
                        }
                        ui.add_space(14.0);
                        let line_count = script_buffer.lines().count().max(1);
                        let char_count = script_buffer.len();
                        widgets::hud(
                            ui,
                            &format!("{} LINES  •  {} BYTES", line_count, char_count),
                            t.faint,
                        );
                    });
                });
            });

            // --- 2. EDITOR BODY ZONE ---
            let inner_body = body_rect.shrink2(vec2(20.0, 16.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner_body), |ui| {
                let mut layouter = create_script_layouter(modal.language, t, 13.0);

                egui::ScrollArea::vertical()
                    .id_salt("expanded_canvas_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let text_resp = ui.add(
                            egui::TextEdit::multiline(&mut script_buffer)
                                .desired_width(ui.available_width())
                                .desired_rows(24)
                                .font(egui::FontId::new(13.0, theme::mono()))
                                .frame(egui::Frame::new().fill(egui::Color32::TRANSPARENT).stroke(Stroke::NONE))
                                .margin(vec2(0.0, 0.0))
                                .layouter(&mut layouter),
                        );
                        if text_resp.changed() {
                            text_changed = true;
                        }
                    });
            });

            // --- 3. FOOTER ZONE ---
            ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    widgets::hud(ui, "PRESS ESC OR CLICK OUTSIDE TO CLOSE", t.faint);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if widgets::button(ui, "DONE", Kind::Primary, 0.0).clicked() {
                            close_clicked = true;
                        }
                        ui.add_space(10.0);
                        if widgets::button(ui, "COPY SCRIPT", Kind::Secondary, 0.0).clicked() {
                            copy_clicked = true;
                        }
                    });
                });
            });
        });

    if copy_clicked {
        ctx.copy_text(script_buffer.clone());
        state.set_toast("Script copied to clipboard");
    }

    if text_changed {
        if let Some(rule) = &mut state.editing_rule {
            let actions = if modal.is_exit_pipeline {
                &mut rule.on_exit_actions
            } else {
                &mut rule.actions
            };
            if let Some(action) = actions.get_mut(modal.step_idx) {
                match action {
                    Action::PowerShell { ref mut script, .. } => *script = script_buffer,
                    Action::Cmd { ref mut command, .. } => *command = script_buffer,
                    _ => {}
                }
            }
        }
    }

    if close_clicked {
        state.close_script_editor();
    }
}
