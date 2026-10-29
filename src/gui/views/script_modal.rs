//! Expanded Script Canvas Modal:
//! Centered studio canvas for editing PowerShell and CMD scripts
//! with syntax highlighting, line metrics, clipboard copy, and smooth animations.

use egui::{pos2, vec2, Stroke};
use crate::gui::components::modal::Modal;
use crate::gui::components::script_highlighter::create_script_layouter;
use crate::gui::components::widgets::{self, Glyph, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::Action;

pub fn render_script_editor_canvas(ctx: &egui::Context, state: &mut GuiState) {
    let is_open = state.expanded_script_editor.is_some();
    let active_editor = state
        .expanded_script_editor
        .as_ref()
        .or(state.last_script_editor.as_ref())
        .cloned();

    let Some(modal) = active_editor else { return };
    let t = Tokens::get(ctx);

    // Retrieve script string from state.editing_rule
    let mut script_buffer = String::new();
    let mut found = false;
    if let Some(rule) = &state.editing_rule {
        let actions = if modal.is_exit_pipeline {
            &rule.on_exit_actions
        } else {
            &rule.actions
        };
        if let Some(Action::ExecuteScript { script, .. }) = actions.get(modal.step_idx) {
            script_buffer = script.clone();
            found = true;
        }
    }

    if !found && is_open {
        state.close_script_editor();
        return;
    }

    let mut copy_clicked = false;
    let mut text_changed = false;

    let modal_res = Modal::new("script-canvas-modal", is_open)
        .width_fraction(0.84)
        .height_fraction(0.82)
        .min_size(vec2(620.0, 420.0))
        .max_size(vec2(1150.0, 840.0))
        .header_height(52.0)
        .body_sunken(true)
        .show(ctx, &t, |frame| {
            let loc = state.loc();

            // 1. Header slice
            frame.header(|ui, close_req| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    let prefix = if modal.is_exit_pipeline { loc.inspector_exit_prefix } else { loc.inspector_step_prefix };
                    widgets::hud(ui, &format!("{} {:02}", prefix, modal.step_idx + 1), t.accent_ink);
                    ui.add_space(8.0);
                    widgets::badge(ui, modal.language.badge_name(), t.accent_ink);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.modal_tip_close).clicked() {
                            *close_req = true;
                        }

                        ui.add_space(8.0);
                        if widgets::button(ui, loc.modal_btn_copy_script, Kind::Secondary, 0.0).clicked() {
                            copy_clicked = true;
                        }
                    });
                });
            });

            // 2. Editor body slice
            frame.body(|ui| {
                let body_rect = ui.max_rect();
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

                // Floating semi-transparent metrics label in bottom-right corner of editor canvas
                {
                    let line_count = script_buffer.lines().count().max(1);
                    let char_count = script_buffer.chars().count();
                    let metrics_text = format!("{} {}  •  {} {}", line_count, loc.modal_lines, char_count, loc.modal_chars);
                    let text_color = t.faint.gamma_multiply(0.5);
                    let galley = widgets::caps(ui, &metrics_text, 9.5, theme::mono(), 9.5 * 0.10, text_color);
                    let text_pos = pos2(
                        inner_body.right() - 8.0 - galley.size().x,
                        inner_body.bottom() - 4.0 - galley.size().y,
                    );
                    ui.painter().galley_with_override_text_color(text_pos, galley, text_color);
                }
            });
        });

    if copy_clicked {
        ctx.copy_text(script_buffer.clone());
        let loc = state.loc();
        state.set_toast(loc.toast_script_copied);
    }

    if text_changed {
        if let Some(rule) = &mut state.editing_rule {
            let actions = if modal.is_exit_pipeline {
                &mut rule.on_exit_actions
            } else {
                &mut rule.actions
            };
            if let Some(Action::ExecuteScript { ref mut script, .. }) = actions.get_mut(modal.step_idx) {
                *script = script_buffer;
            }
        }
    }

    if let Some(res) = modal_res {
        if res.close_requested {
            state.close_script_editor();
        }
    } else if !is_open {
        state.last_script_editor = None;
    }
}
