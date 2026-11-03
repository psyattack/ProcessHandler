//! Automation rules view in the craft-launcher aesthetic:
//! precision segmented cells, hairline cards, numbered section eyebrows,
//! uppercase mono badges, and inverted action buttons.

use egui::{CornerRadius, pos2, Rect, Sense, Stroke, Ui, vec2};
use crate::gui::components::widgets::{self, Glyph, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{Action, ConditionOperator, Rule, RuleCondition, Trigger};

#[derive(Clone, Copy, Debug, Default)]
struct RuleDndSession {
    source_idx: usize,
    target_idx: usize,
}

pub fn render_rules_view(ui: &mut Ui, state: &mut GuiState) {
    let t = Tokens::get(ui.ctx());
    let loc = state.loc();

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;

        let rules = state
            .daemon_state
            .as_ref()
            .map(|s| s.config.rules.clone())
            .unwrap_or_default();

        let total = rules.len();

        // 1. Top Bar: Tag Filter ComboBox + Sort ComboBox + Search Box + Import/Export/New Rule
        ui.horizontal(|ui| {
            // Tag Filter dropdown (clean text without leading glyph)
            let all_tags = state.all_distinct_tags();
            let selected_tag = state.selected_tag_filter.as_ref().and_then(|t_name| {
                all_tags.iter().find(|t| t.name.eq_ignore_ascii_case(t_name))
            });

            let current_tag_label = if let Some(tag) = selected_tag {
                tag.name.clone()
            } else {
                loc.rules_tag_filter_all.to_string()
            };

            egui::ComboBox::from_id_salt("rules_tag_filter_combo")
                .selected_text(&current_tag_label)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(state.selected_tag_filter.is_none(), loc.rules_tag_filter_all).clicked() {
                        state.selected_tag_filter = None;
                    }
                    if !all_tags.is_empty() {
                        ui.separator();
                        for tag in &all_tags {
                            let is_sel = state.selected_tag_filter.as_deref() == Some(&tag.name);
                            if ui.selectable_label(is_sel, &tag.name).clicked() {
                                state.selected_tag_filter = Some(tag.name.clone());
                            }
                        }
                    }
                });

            ui.add_space(6.0);

            // Sort dropdown (clean text without leading glyph)
            let current_sort_label = match state.rule_sort_mode {
                crate::gui::state::RuleSortMode::Default => loc.rules_sort_default,
                crate::gui::state::RuleSortMode::NameAsc => loc.rules_sort_name_asc,
                crate::gui::state::RuleSortMode::NameDesc => loc.rules_sort_name_desc,
                crate::gui::state::RuleSortMode::TriggerType => loc.rules_sort_trigger,
                crate::gui::state::RuleSortMode::ActionCount => loc.rules_sort_actions,
                crate::gui::state::RuleSortMode::Status => loc.rules_sort_status,
            };

            egui::ComboBox::from_id_salt("rules_sort_mode_combo")
                .selected_text(current_sort_label)
                .show_ui(ui, |ui| {
                    let sort_options = [
                        (crate::gui::state::RuleSortMode::Default, loc.rules_sort_default),
                        (crate::gui::state::RuleSortMode::NameAsc, loc.rules_sort_name_asc),
                        (crate::gui::state::RuleSortMode::NameDesc, loc.rules_sort_name_desc),
                        (crate::gui::state::RuleSortMode::TriggerType, loc.rules_sort_trigger),
                        (crate::gui::state::RuleSortMode::ActionCount, loc.rules_sort_actions),
                        (crate::gui::state::RuleSortMode::Status, loc.rules_sort_status),
                    ];
                    for (mode, label) in sort_options {
                        if ui.selectable_label(state.rule_sort_mode == mode, label).clicked() {
                            state.rule_sort_mode = mode;
                        }
                    }
                });

            ui.add_space(6.0);

            // Search box (pinned to right of combo-boxes)
            ui.add(
                egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text(loc.rules_search_placeholder)
                    .desired_width(180.0)
                    .min_size(vec2(0.0, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(8.0, 7.0)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Primary "+ NEW AUTOMATION" Button
                if widgets::button(ui, loc.rules_btn_new, Kind::Primary, 0.0).clicked() {
                    state.start_new_rule();
                }

                ui.add_space(4.0);

                // Export all automations to clipboard
                if widgets::button(ui, loc.rules_btn_copy_all, Kind::Secondary, 0.0)
                    .on_hover_text(loc.rules_btn_copy_all_tip)
                    .clicked()
                {
                    state.copy_all_rules_to_clipboard(ui.ctx());
                }

                ui.add_space(4.0);

                // Import automations from clipboard
                if widgets::button(ui, loc.rules_btn_import, Kind::Secondary, 0.0)
                    .on_hover_text(loc.rules_btn_import_tip)
                    .clicked()
                {
                    state.import_automations_from_clipboard(ui.ctx());
                }
            });
        });

        // 2. Filter and sort rules
        let needle = state.search_query.trim().to_lowercase();
        let mut filtered: Vec<_> = rules
            .into_iter()
            .filter(|r| {
                if let Some(tag_name) = &state.selected_tag_filter {
                    r.tags.iter().any(|t| t.name.eq_ignore_ascii_case(tag_name))
                } else {
                    true
                }
            })
            .filter(|r| {
                if needle.is_empty() {
                    true
                } else {
                    r.name.to_lowercase().contains(&needle)
                        || r.tags.iter().any(|t| t.name.to_lowercase().contains(&needle))
                }
            })
            .collect();

        // Apply sort
        match state.rule_sort_mode {
            crate::gui::state::RuleSortMode::Default => {
                // Keep manual user drag & drop ordering
            }
            crate::gui::state::RuleSortMode::NameAsc => {
                filtered.sort_by_key(|a| a.name.to_lowercase());
            }
            crate::gui::state::RuleSortMode::NameDesc => {
                filtered.sort_by_key(|a| std::cmp::Reverse(a.name.to_lowercase()));
            }
            crate::gui::state::RuleSortMode::TriggerType => {
                filtered.sort_by_key(|r| trigger_sort_key(&r.trigger));
            }
            crate::gui::state::RuleSortMode::ActionCount => {
                filtered.sort_by_key(|r| std::cmp::Reverse(r.actions.len() + r.on_exit_actions.len()));
            }
            crate::gui::state::RuleSortMode::Status => {
                filtered.sort_by_key(|r| (!r.is_enabled, r.name.to_lowercase()));
            }
        }

        if filtered.is_empty() {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                if total == 0 {
                    widgets::hud(ui, loc.rules_empty_title, t.muted);
                    ui.add_space(12.0);
                    if widgets::button(ui, loc.rules_btn_create_first, Kind::Secondary, 0.0).clicked() {
                        state.start_new_rule();
                    }
                } else {
                    widgets::hud(ui, loc.rules_no_filtered_matches, t.muted);
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(loc.rules_no_filtered_matches_tip).color(t.faint));
                }
            });
            return;
        }

        // 3. Streamlined Track-List with Drag & Drop Reordering & Horizontal Wheel Routing
        let is_dnd_allowed = state.rule_sort_mode == crate::gui::state::RuleSortMode::Default
            && state.selected_tag_filter.is_none()
            && needle.is_empty();

        let dnd_id = egui::Id::new("rules_view_dnd");
        let mut dnd_session = if is_dnd_allowed {
            ui.data(|d| d.get_temp::<RuleDndSession>(dnd_id))
        } else {
            None
        };
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        let pointer_released = ui.input(|i| i.pointer.any_released());

        // Mouse wheel scrolling while dragging items vertically
        if dnd_session.is_some() {
            let wheel_delta_y = ui.input(|i| i.smooth_scroll_delta.y);
            if wheel_delta_y != 0.0 {
                ui.scroll_with_delta(vec2(0.0, wheel_delta_y * 1.5));
                ui.ctx().request_repaint();
            }
        }

        let is_dnd_active = dnd_session.is_some();
        let active_src = dnd_session.as_ref().map(|s| s.source_idx);
        let active_dst = dnd_session.as_ref().map(|s| s.target_idx);

        let mut drag_started_idx = None;
        let mut current_centers = Vec::with_capacity(filtered.len());

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;

                for (idx, rule) in filtered.iter().enumerate() {
                    // Drop slot indicator before this row
                    if is_dnd_allowed {
                        render_rule_drop_slot(ui, idx, active_dst, active_src, &t);
                    }

                    let is_selected = state.selected_rule_id.as_deref() == Some(&rule.id);
                    let is_being_dragged = active_src == Some(idx);

                    let (row_rect, grip_started) = render_track_row(
                        ui,
                        state,
                        rule,
                        idx,
                        is_selected,
                        is_being_dragged,
                        is_dnd_allowed,
                        is_dnd_active,
                        &t,
                    );
                    current_centers.push(row_rect.center().y);

                    if grip_started && is_dnd_allowed {
                        drag_started_idx = Some(idx);
                    }
                }

                // Trailing drop slot after last row
                if is_dnd_allowed && !filtered.is_empty() {
                    render_rule_drop_slot(ui, filtered.len(), active_dst, active_src, &t);
                }
            });

        // Update target_idx using fresh current_centers
        if is_dnd_allowed {
            if let (Some(session), Some(pointer)) = (&mut dnd_session, pointer_pos) {
                let n = current_centers.len();
                if n > 0 {
                    if pointer.y < current_centers[0] {
                        session.target_idx = 0;
                    } else if pointer.y >= current_centers[n - 1] {
                        session.target_idx = n;
                    } else {
                        for i in 0..(n - 1) {
                            if pointer.y >= current_centers[i] && pointer.y < current_centers[i + 1] {
                                session.target_idx = i + 1;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Commit reorder on mouse release
        if pointer_released && is_dnd_allowed {
            if let Some(session) = dnd_session {
                let src = session.source_idx;
                let dst = session.target_idx;
                if src < filtered.len() && dst <= filtered.len() && src != dst && dst != src + 1 {
                    let src_rule_id = filtered[src].id.clone();
                    let (target_id, insert_after) = if dst == filtered.len() {
                        (Some(filtered[filtered.len() - 1].id.clone()), true)
                    } else if dst > src {
                        (Some(filtered[dst - 1].id.clone()), true)
                    } else {
                        (Some(filtered[dst].id.clone()), false)
                    };
                    state.reorder_rule_by_id(&src_rule_id, target_id.as_deref(), insert_after);
                }
                dnd_session = None;
                ui.data_mut(|d| d.remove_temp::<RuleDndSession>(dnd_id));
            }
        }

        // Handle newly started drag
        if is_dnd_allowed {
            if let Some(src_idx) = drag_started_idx {
                dnd_session = Some(RuleDndSession {
                    source_idx: src_idx,
                    target_idx: src_idx,
                });
            }
        }

        // Save session & paint floating preview
        if let Some(session) = dnd_session {
            ui.data_mut(|d| d.insert_temp(dnd_id, session));
            ui.ctx().request_repaint();
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            if let Some(pointer) = pointer_pos {
                if session.source_idx < filtered.len() {
                    let dragged_rule = &filtered[session.source_idx];
                    let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("rule_drag_preview")));
                    let float_rect = Rect::from_min_size(pointer + vec2(14.0, 10.0), vec2(180.0, 26.0));
                    p.rect_filled(float_rect, CornerRadius::same(2), t.bg);
                    p.rect_stroke(float_rect, CornerRadius::same(2), Stroke::new(1.0, t.accent_ink), egui::StrokeKind::Inside);

                    let preview_text = format!("{} {}", dragged_rule.default_icon_glyph(), dragged_rule.name);
                    p.text(
                        float_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        preview_text,
                        egui::FontId::new(11.5, theme::semibold()),
                        t.ink_strong,
                    );
                }
            }
        }
    });
}

fn render_rule_drop_slot(
    ui: &mut Ui,
    slot_idx: usize,
    active_dst: Option<usize>,
    active_src: Option<usize>,
    t: &Tokens,
) {
    let is_targeted = if let (Some(dst), Some(src)) = (active_dst, active_src) {
        dst == slot_idx && slot_idx != src && slot_idx != src + 1
    } else {
        false
    };

    let anim_id = ui.id().with(("rule_slot_anim", slot_idx));
    let anim = ui.ctx().animate_bool_with_time_and_easing(anim_id, is_targeted, 0.16, egui::emath::easing::cubic_out);

    if anim > 0.001 {
        let slot_h = 16.0 * anim;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), slot_h), Sense::hover());
        let p = ui.painter();
        let mid_y = rect.center().y;

        p.rect_filled(
            rect,
            CornerRadius::ZERO,
            t.accent_ink.gamma_multiply(0.08 * anim),
        );

        let line_stroke = Stroke::new(2.0 * anim, t.accent_ink.gamma_multiply(anim));
        p.hline(rect.left() + 16.0..=rect.right() - 16.0, mid_y, line_stroke);

        let bead_pos = pos2(rect.left() + 20.0, mid_y);
        p.circle_filled(bead_pos, 3.0 * anim, t.accent_ink.gamma_multiply(anim));
    }
}

#[allow(clippy::too_many_arguments)]
fn render_track_row(
    ui: &mut Ui,
    state: &mut GuiState,
    rule: &Rule,
    _idx: usize,
    is_selected: bool,
    is_being_dragged: bool,
    is_dnd_allowed: bool,
    is_dnd_active: bool,
    t: &Tokens,
) -> (Rect, bool) {
    let loc = state.loc();
    let is_manual = rule.trigger.as_ref().is_none_or(|t| matches!(t, crate::models::Trigger::Manual));
    let exec_opt = state.get_rule_execution_state(rule);
    let is_test_running = state.active_test_runs.contains_key(&rule.id);
    let is_running = exec_opt.is_some() || is_test_running;

    if is_running {
        ui.ctx().request_repaint();
    }

    let stroke = if is_being_dragged {
        Stroke::new(1.5, t.accent_ink.gamma_multiply(0.7))
    } else if is_selected {
        Stroke::new(1.5, t.accent)
    } else if is_running {
        Stroke::new(1.5, t.good)
    } else if !rule.is_enabled && !is_manual {
        Stroke::new(1.0, t.line.gamma_multiply(0.4))
    } else {
        t.hairline()
    };

    let fill = if is_being_dragged {
        t.sunken.gamma_multiply(0.6)
    } else if !rule.is_enabled && !is_manual {
        t.bg.gamma_multiply(0.65)
    } else {
        t.bg
    };

    let row_inner_h = 28.0;

    let frame = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(10, 6));

    let mut grip_started = false;

    let response = frame.show(ui, |ui| {
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), row_inner_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.spacing_mut().item_spacing.y = 0.0;

                // 1. Drag & Drop handle on far left
                let grip_tip = if is_dnd_allowed { loc.rules_tip_drag_rule } else { loc.rules_tip_dnd_disabled_sort };
                let grip_resp = widgets::drag_handle_custom_interactive(ui, 26.0, grip_tip, is_dnd_allowed);
                if grip_resp.drag_started() && is_dnd_allowed {
                    grip_started = true;
                }

                // 2. Rule Icon (28x28)
                let icon_size = vec2(28.0, 28.0);
                let (icon_rect, icon_resp) = ui.allocate_exact_size(icon_size, Sense::click());
                ui.painter().rect_filled(icon_rect, CornerRadius::ZERO, t.sunken);
                ui.painter().rect_stroke(icon_rect, CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);

                let mut loaded_texture = false;
                if let Some(path) = &rule.custom_icon_path {
                    if let Some(tex) = state.icon_cache.get_icon_from_path(ui.ctx(), path) {
                        let size = tex.size_vec2();
                        let max_dim = size.x.max(size.y).max(1.0);
                        let scale = 18.0 / max_dim;
                        let render_size = vec2((size.x * scale).round().max(1.0), (size.y * scale).round().max(1.0));
                        let img_rect = Rect::from_center_size(icon_rect.center(), render_size);
                        ui.painter().image(
                            tex.id(),
                            img_rect,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        loaded_texture = true;
                    }
                }

                if !loaded_texture {
                    let glyph = rule.default_icon_glyph();
                    let icon_color = if rule.is_enabled || is_manual { t.ink_strong } else { t.muted };
                    ui.painter().text(
                        icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        glyph,
                        egui::FontId::new(14.0, theme::phosphor()),
                        icon_color,
                    );
                }

                if icon_resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    state.edit_rule(rule.clone());
                }

                // 3. Rule Name and optional Tag Pills (Strictly fixed height - never alters card size)
                let title_color = if rule.is_enabled || is_manual { t.ink_strong } else { t.muted };
                let title_font = if !rule.tags.is_empty() {
                    egui::FontId::new(11.5, theme::semibold())
                } else {
                    egui::FontId::new(13.0, theme::semibold())
                };

                let natural_title_w = ui.painter().layout_no_wrap(
                    rule.name.clone(),
                    title_font.clone(),
                    title_color,
                ).size().x;

                let tags_w = if !rule.tags.is_empty() {
                    measure_tags_width(ui, &rule.tags)
                } else {
                    0.0
                };
                let name_col_w = natural_title_w.max(tags_w).clamp(60.0, 200.0);

                let format = egui::TextFormat {
                    font_id: title_font,
                    color: title_color,
                    ..Default::default()
                };
                let mut job = egui::text::LayoutJob::single_section(rule.name.clone(), format);
                job.wrap = egui::text::TextWrapping::truncate_at_width(name_col_w);
                let title_galley = ui.painter().layout_job(job);
                let is_elided = title_galley.elided;

                let (name_rect, mut name_col_resp) = ui.allocate_exact_size(vec2(name_col_w, row_inner_h), Sense::click());
                name_col_resp = name_col_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if is_elided {
                    name_col_resp = name_col_resp.on_hover_text(&rule.name);
                }
                if name_col_resp.clicked() {
                    state.edit_rule(rule.clone());
                }

                if !rule.tags.is_empty() {
                    ui.painter().galley(pos2(name_rect.left(), name_rect.top() + 1.0), title_galley, title_color);

                    render_card_tags_at(
                        ui,
                        &rule.tags,
                        name_rect.left(),
                        name_rect.top() + 15.0,
                        name_col_w,
                        &rule.id,
                        t,
                    );
                } else {
                    let title_pos = pos2(name_rect.left(), name_rect.center().y - title_galley.size().y / 2.0);
                    ui.painter().galley(title_pos, title_galley, title_color);
                }

                // Spacing from title to trigger badge
                ui.add_space(8.0);

                // 4. Center Pipeline Stream in Horizontal ScrollArea (Pixel-perfect vertical centering)
                let right_controls_w = if is_manual { 165.0 } else { 205.0 };
                let center_w = (ui.available_width() - right_controls_w).max(40.0);

                ui.allocate_ui_with_layout(
                    vec2(center_w, row_inner_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.style_mut().always_scroll_the_only_direction = true;
                        ui.spacing_mut().scroll.floating = true;
                        ui.spacing_mut().scroll.floating_allocated_width = 0.0;
                        ui.spacing_mut().scroll.floating_width = 4.0;
                        ui.spacing_mut().scroll.bar_width = 8.0;
                        ui.spacing_mut().scroll.bar_inner_margin = 0.0;
                        ui.spacing_mut().scroll.bar_outer_margin = 0.0;
                        ui.spacing_mut().scroll.handle_min_length = 24.0;

                        let scroll_source = egui::containers::scroll_area::ScrollSource {
                            scroll_bar: !is_dnd_active,
                            drag: if is_dnd_active {
                                egui::containers::scroll_area::DragScroll::Never
                            } else {
                                egui::containers::scroll_area::DragScroll::Always
                            },
                            mouse_wheel: !is_dnd_active,
                        };

                        egui::ScrollArea::horizontal()
                            .id_salt(("rule_pipe_scroll", &rule.id))
                            .auto_shrink([false, false])
                            .wheel_scroll_multiplier(vec2(1.5, 1.0))
                            .scroll_source(scroll_source)
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                            .show(ui, |ui| {
                                ui.allocate_ui_with_layout(
                                    vec2(0.0, row_inner_h),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0;
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        render_track_pipeline(ui, loc, rule, exec_opt, is_test_running, t);
                                    },
                                );
                            });
                    },
                );

                // 5. Far Right Controls: Toggle switch + RUN + EDIT + CLONE + SHARE + DELETE
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), row_inner_h),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        // Delete (far right)
                        if widgets::glyph_button_custom(ui, Glyph::Trash, 26.0, false, loc.rules_btn_delete).clicked() {
                            state.delete_rule(&rule.id);
                        }

                        // Share
                        if widgets::glyph_button_custom(ui, Glyph::Share, 26.0, false, loc.rules_btn_copy_single_tip).clicked() {
                            state.copy_single_rule_to_clipboard(ui.ctx(), rule);
                        }

                        // Clone
                        if widgets::glyph_button_custom(ui, Glyph::Clone, 26.0, false, loc.rules_btn_clone).clicked() {
                            state.clone_rule(rule);
                        }

                        // Edit
                        if widgets::glyph_button_custom(ui, Glyph::Edit, 26.0, false, loc.rules_btn_edit).clicked() {
                            state.edit_rule(rule.clone());
                        }

                        // Run / Stop (Renamed to RUN)
                        let is_stoppable = if is_manual { is_running } else { is_test_running };
                        let (run_glyph, run_tip) = if is_stoppable {
                            (Glyph::Stop, loc.rules_btn_stop)
                        } else {
                            (Glyph::Play, "RUN")
                        };
                        if widgets::glyph_button_custom(ui, run_glyph, 26.0, is_stoppable, run_tip).clicked() {
                            if is_stoppable {
                                state.stop_rule_execution(&rule.id);
                            } else {
                                state.test_run_rule(&rule.id);
                            }
                        }

                        if !is_manual {
                            ui.add_space(6.0);

                            // Pause / Resume Toggle Switch
                            let mut is_on = rule.is_enabled;
                            let switch_tip = if is_on { loc.rules_btn_pause } else { loc.rules_btn_resume };
                            if widgets::toggle_switch(ui, &mut is_on).on_hover_text(switch_tip).changed() {
                                state.toggle_rule(&rule.id, is_on);
                            }
                        }
                    },
                );
            },
        );
    });

    (response.response.rect, grip_started)
}

fn render_track_pipeline(
    ui: &mut Ui,
    loc: &crate::i18n::Strings,
    rule: &Rule,
    exec_opt: Option<crate::ipc::protocol::ActiveRuleExecutionState>,
    is_test_running: bool,
    t: &Tokens,
) {
    ui.spacing_mut().item_spacing.x = 6.0;

    // 0. Live RUNNING Badge before Trigger
    let is_running = exec_opt.is_some() || is_test_running;
    if is_running {
        let running_text = if let Some(exec) = &exec_opt {
            if exec.active_count > 1 {
                format!("{} {}x", loc.rules_badge_running, exec.active_count)
            } else {
                loc.rules_badge_running.to_string()
            }
        } else {
            loc.rules_badge_running.to_string()
        };
        widgets::badge(ui, &running_text, t.good);
    }

    // 1. Trigger Chip (No "TRIGGER:" prefix! Clean glyph and detail)
    let (trig_glyph, trig_label) = format_trigger_display(&rule.trigger, loc);
    let trig_chip = format!("{} {}", trig_glyph, trig_label);
    let galley = widgets::caps(ui, &trig_chip, 9.5, theme::mono(), 0.5, t.accent_ink);
    let chip_w = galley.size().x + 12.0;
    let (chip_rect, chip_resp) = ui.allocate_exact_size(vec2(chip_w, 22.0), Sense::hover());
    ui.painter().rect(chip_rect, CornerRadius::ZERO, t.sunken, t.hairline(), egui::StrokeKind::Inside);
    widgets::paint_centred(ui, chip_rect, galley, t.accent_ink);
    chip_resp.on_hover_text(&trig_label);

    // 2. Pre-conditions (if any) - NO arrow before IF
    if !rule.conditions.is_empty() {
        widgets::hud(ui, loc.rules_summary_cond_if, t.faint);
        for (c_idx, item) in rule.conditions.iter().enumerate() {
            if c_idx > 0 {
                let op_str = match item.operator {
                    ConditionOperator::And => loc.inspector_cond_op_and,
                    ConditionOperator::Or => loc.inspector_cond_op_or,
                };
                widgets::hud(ui, op_str, t.faint);
            }
            let label = format_single_condition_brief(&item.condition);
            let tip = format_single_condition_tooltip(&item.condition, c_idx + 1, loc);
            widgets::badge(ui, &label, t.faint).on_hover_text(tip);
        }
    }

    // 3. Arrow divider to actions
    widgets::hud(ui, "➔", t.faint);

    // 4. Action Steps (No step numbering! Active highlight with ▶ prefix!)
    if rule.actions.is_empty() {
        widgets::hud(ui, loc.rules_summary_no_actions, t.muted);
    } else {
        for (a_idx, action) in rule.actions.iter().enumerate() {
            if a_idx > 0 {
                widgets::hud(ui, "➔", t.faint);
            }

            let is_current = exec_opt.is_some_and(|e| !e.is_exit && e.current_step == a_idx);
            let brief = format_action_brief(action, loc);
            let icon = action_icon_glyph(action);

            let (chip_fill, chip_stroke, chip_ink, font) = if is_current {
                (
                    t.accent.gamma_multiply(0.35),
                    Stroke::new(1.5, t.accent_ink),
                    t.accent_ink,
                    theme::mono_bold(),
                )
            } else {
                (t.sunken, t.hairline(), t.ink, theme::mono())
            };

            let step_label = if is_current {
                format!("▶ {} {}", icon, brief)
            } else {
                format!("{} {}", icon, brief)
            };
            let galley = widgets::caps(ui, &step_label, 9.5, font, 0.5, chip_ink);
            let chip_w = galley.size().x + 12.0;
            let (chip_rect, chip_resp) = ui.allocate_exact_size(vec2(chip_w, 22.0), Sense::hover());
            ui.painter().rect(chip_rect, CornerRadius::ZERO, chip_fill, chip_stroke, egui::StrokeKind::Inside);
            widgets::paint_centred(ui, chip_rect, galley, chip_ink);

            if is_current {
                chip_resp.on_hover_text(loc.rules_summary_step_active);
            } else {
                chip_resp.on_hover_text(&brief);
            }
        }
    }

    // 5. On-Exit Actions (if any)
    if !rule.on_exit_actions.is_empty() {
        widgets::hud(ui, "➔", t.faint);
        widgets::hud(ui, loc.rules_summary_on_exit, t.accent_ink);

        for (e_idx, action) in rule.on_exit_actions.iter().enumerate() {
            if e_idx > 0 {
                widgets::hud(ui, "➔", t.faint);
            }

            let is_current = exec_opt.is_some_and(|e| e.is_exit && e.current_step == e_idx);
            let brief = format_action_brief(action, loc);
            let icon = action_icon_glyph(action);

            let (chip_fill, chip_stroke, chip_ink, font) = if is_current {
                (
                    t.accent.gamma_multiply(0.35),
                    Stroke::new(1.5, t.accent_ink),
                    t.accent_ink,
                    theme::mono_bold(),
                )
            } else {
                (t.sunken, t.hairline(), t.faint, theme::mono())
            };

            let step_label = if is_current {
                format!("▶ {} {}", icon, brief)
            } else {
                format!("{} {}", icon, brief)
            };
            let galley = widgets::caps(ui, &step_label, 9.5, font, 0.5, chip_ink);
            let chip_w = galley.size().x + 12.0;
            let (chip_rect, chip_resp) = ui.allocate_exact_size(vec2(chip_w, 22.0), Sense::hover());
            ui.painter().rect(chip_rect, CornerRadius::ZERO, chip_fill, chip_stroke, egui::StrokeKind::Inside);
            widgets::paint_centred(ui, chip_rect, galley, chip_ink);

            if is_current {
                chip_resp.on_hover_text(loc.rules_summary_step_active);
            } else {
                chip_resp.on_hover_text(&brief);
            }
        }
    }

    // 6. Companion mode
    let has_companion = rule.actions.iter().any(|a| matches!(a, Action::StartProcess { close_on_exit: true, .. }));
    if has_companion {
        widgets::badge(ui, loc.rules_summary_bound_satellite, t.good);
    }
}

fn measure_tags_width(ui: &egui::Ui, tags: &[crate::models::RuleTag]) -> f32 {
    let font_text = egui::FontId::new(8.5, theme::mono());
    let font_icon = egui::FontId::new(9.0, theme::phosphor());
    let mut total_w = 0.0;
    for (i, tag) in tags.iter().enumerate() {
        if i >= 3 {
            total_w += 24.0;
            break;
        }
        let text_w = ui.painter().layout_no_wrap(tag.name.clone(), font_text.clone(), egui::Color32::WHITE).size().x;
        let icon_w = if let Some(ico) = &tag.icon {
            if !ico.is_empty() {
                ui.painter().layout_no_wrap(ico.clone(), font_icon.clone(), egui::Color32::WHITE).size().x + 3.5
            } else {
                0.0
            }
        } else {
            0.0
        };
        let pill_w = (9.0 + icon_w + text_w).max(16.0);
        total_w += pill_w + 3.0;
    }
    total_w
}

fn render_card_tags_at(
    ui: &mut Ui,
    tags: &[crate::models::RuleTag],
    start_x: f32,
    top_y: f32,
    max_w: f32,
    rule_id: &str,
    t: &Tokens,
) {
    let font_text = egui::FontId::new(8.5, theme::mono());
    let font_icon = egui::FontId::new(9.0, theme::phosphor());

    let mut curr_x = start_x;
    let pill_h = 13.0;
    let total_tags = tags.len();
    let mut shown_count = 0;

    for (i, tag) in tags.iter().enumerate() {
        let tag_color = tag.parse_color(t.accent_ink);
        let has_ico = tag.icon.as_ref().is_some_and(|ico| !ico.is_empty());

        let text_galley = ui.painter().layout_no_wrap(tag.name.clone(), font_text.clone(), tag_color);
        let (icon_galley, icon_w) = if let Some(ico) = &tag.icon {
            if !ico.is_empty() {
                let g = ui.painter().layout_no_wrap(ico.clone(), font_icon.clone(), tag_color);
                let w = g.size().x;
                (Some(g), w)
            } else {
                (None, 0.0)
            }
        } else {
            (None, 0.0)
        };

        let pad_x = 4.5;
        let gap = if has_ico { 3.5 } else { 0.0 };
        let pill_w = (pad_x + icon_w + gap + text_galley.size().x + pad_x).max(16.0);
        let more_badge_w = 20.0;

        let has_more = i + 1 < total_tags;
        let needed_w = if has_more { pill_w + 3.0 + more_badge_w } else { pill_w };

        if curr_x - start_x + needed_w > max_w && shown_count > 0 {
            break;
        }

        let pill_rect = Rect::from_min_size(pos2(curr_x, top_y), vec2(pill_w, pill_h));
        let resp = ui.interact(pill_rect, ui.id().with(("card_tag", rule_id, i)), Sense::hover());

        ui.painter().rect(
            pill_rect,
            CornerRadius::same(3),
            tag_color.gamma_multiply(0.18),
            Stroke::new(0.8, tag_color.gamma_multiply(0.50)),
            egui::StrokeKind::Inside,
        );

        let mut draw_x = pill_rect.left() + pad_x;
        if let Some(ig) = icon_galley {
            let icon_pos = pos2(draw_x, pill_rect.center().y - ig.size().y / 2.0);
            ui.painter().galley(icon_pos, ig, tag_color);
            draw_x += icon_w + gap;
        }

        let text_pos = pos2(draw_x, pill_rect.center().y - text_galley.size().y / 2.0);
        ui.painter().galley(text_pos, text_galley, tag_color);

        resp.on_hover_text(&tag.name);

        curr_x += pill_w + 3.0;
        shown_count += 1;
    }

    let hidden_count = total_tags.saturating_sub(shown_count);
    if hidden_count > 0 {
        let more_text = format!("+{}", hidden_count);
        let more_galley = ui.painter().layout_no_wrap(more_text, egui::FontId::new(8.0, theme::mono()), t.muted);
        let more_w = more_galley.size().x + 8.0;
        let more_rect = Rect::from_min_size(pos2(curr_x, top_y), vec2(more_w, pill_h));
        let more_resp = ui.interact(more_rect, ui.id().with(("card_tag_more", rule_id)), Sense::hover());

        ui.painter().rect(
            more_rect,
            CornerRadius::same(3),
            t.sunken,
            Stroke::new(0.8, t.line),
            egui::StrokeKind::Inside,
        );
        let more_pos = pos2(more_rect.center().x - more_galley.size().x / 2.0, more_rect.center().y - more_galley.size().y / 2.0);
        ui.painter().galley(more_pos, more_galley, t.muted);

        let hidden_names: Vec<&str> = tags.iter().skip(shown_count).map(|t| t.name.as_str()).collect();
        more_resp.on_hover_text(hidden_names.join(", "));
    }
}

fn trigger_sort_key(trig: &Option<crate::models::Trigger>) -> usize {
    match trig {
        Some(crate::models::Trigger::Process { .. }) => 0,
        Some(crate::models::Trigger::WindowFocus { .. }) => 1,
        Some(crate::models::Trigger::GlobalHotkey { .. }) => 2,
        Some(crate::models::Trigger::Schedule { .. }) => 3,
        Some(crate::models::Trigger::Webhook { .. }) => 4,
        Some(crate::models::Trigger::FileWatcher { .. }) => 5,
        Some(crate::models::Trigger::ResourceThreshold { .. }) => 6,
        Some(crate::models::Trigger::SystemEvent { .. }) => 7,
        Some(crate::models::Trigger::Manual) | None => 8,
    }
}

fn format_single_condition_brief(cond: &RuleCondition) -> String {
    match cond {
        RuleCondition::ProcessRunning { process_name, must_be_running, .. } => {
            let name = if process_name.is_empty() { "PROC" } else { process_name.as_str() };
            if *must_be_running { format!("RUN:{}", name) } else { format!("!RUN:{}", name) }
        }
        RuleCondition::WindowFocused { process_name, must_be_focused, .. } => {
            let name = if process_name.is_empty() { "WINDOW" } else { process_name.as_str() };
            if *must_be_focused { format!("FOCUS:{}", name) } else { format!("!FOCUS:{}", name) }
        }
        RuleCondition::TimeRange { start_time, end_time, .. } => {
            format!("{}-{}", start_time, end_time)
        }
        RuleCondition::Event { event_type, .. } => {
            format!("EVENT:{:?}", event_type)
        }
        RuleCondition::FileWatcher { path, must_exist, .. } => {
            let fname = std::path::Path::new(path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    if path.is_empty() { "FILE".to_string() } else { path.clone() }
                });
            if *must_exist { format!("FILE:{}", fname) } else { format!("!FILE:{}", fname) }
        }
        RuleCondition::ResourceThreshold { process_name, cpu_percent, ram_mb, .. } => {
            let name = if process_name.is_empty() { "RES" } else { process_name.as_str() };
            let mut limits = Vec::new();
            if let Some(c) = cpu_percent { limits.push(format!("CPU>{:.0}%", c)); }
            if let Some(m) = ram_mb { limits.push(format!("RAM>{}MB", m)); }
            if limits.is_empty() {
                format!("RES:{}", name)
            } else {
                format!("RES:{}({})", name, limits.join(","))
            }
        }
    }
}

fn format_single_condition_tooltip(cond: &RuleCondition, idx: usize, loc: &crate::i18n::Strings) -> String {
    let desc = match cond {
        RuleCondition::ProcessRunning { process_name, must_be_running, .. } => {
            let p = if process_name.is_empty() { "?" } else { process_name.as_str() };
            if *must_be_running {
                format!("Process '{}' is running", p)
            } else {
                format!("Process '{}' is not running", p)
            }
        }
        RuleCondition::WindowFocused { process_name, must_be_focused, .. } => {
            let p = if process_name.is_empty() { "?" } else { process_name.as_str() };
            if *must_be_focused {
                format!("Window '{}' is focused", p)
            } else {
                format!("Window '{}' is not focused", p)
            }
        }
        RuleCondition::TimeRange { start_time, end_time, .. } => {
            format!("Time range {} - {}", start_time, end_time)
        }
        RuleCondition::Event { event_type, .. } => {
            format!("System event: {:?}", event_type)
        }
        RuleCondition::FileWatcher { path, must_exist, .. } => {
            if *must_exist {
                format!("File '{}' exists", path)
            } else {
                format!("File '{}' does not exist", path)
            }
        }
        RuleCondition::ResourceThreshold { process_name, cpu_percent, ram_mb, .. } => {
            let p = if process_name.is_empty() { "?" } else { process_name.as_str() };
            format!("Resource threshold on '{}': CPU={:?}, RAM={:?}", p, cpu_percent, ram_mb)
        }
    };
    format!("{} [{}] {}", loc.inspector_conditions_badge, idx, desc)
}

fn format_action_brief(action: &Action, loc: &crate::i18n::Strings) -> String {
    match action {
        Action::StartProcess { file_path, .. } => {
            let fname = std::path::Path::new(file_path).file_name().unwrap_or_default().to_string_lossy();
            format!("{} {}", loc.summary_action_launch, fname)
        }
        Action::KillProcess { process_name, .. } => {
            format!("KILL {}", process_name)
        }
        Action::AudioControl { target, action_type, volume_level, process_name, left_volume, right_volume } => {
            let target_str = match target {
                crate::models::AudioTarget::Playback => "SOUND".to_string(),
                crate::models::AudioTarget::Microphone => "MIC".to_string(),
                crate::models::AudioTarget::Process => format!("PROC ({})", process_name.as_deref().unwrap_or("?")),
            };
            match action_type {
                crate::models::AudioVolumeAction::Mute => format!("{} {}", loc.summary_action_mute, target_str),
                crate::models::AudioVolumeAction::Unmute => format!("{} {}", loc.summary_action_unmute, target_str),
                crate::models::AudioVolumeAction::ToggleMute => format!("{} {}", loc.summary_action_toggle, target_str),
                crate::models::AudioVolumeAction::SetVolume => {
                    if let (Some(l), Some(r)) = (left_volume, right_volume) {
                        format!("VOL L:{}% R:{}% {}", l, r, target_str)
                    } else {
                        format!("{} {}% {}", loc.summary_action_vol, volume_level, target_str)
                    }
                }
                crate::models::AudioVolumeAction::PlayBeep => "BEEP".to_string(),
            }
        }
        Action::AudioFormat { target, sample_rate, bit_depth, .. } => {
            let t_str = match target {
                crate::models::AudioTarget::Playback | crate::models::AudioTarget::Process => "OUT",
                crate::models::AudioTarget::Microphone => "IN",
            };
            format!("FORMAT {} {}Hz/{}-bit", t_str, sample_rate, bit_depth)
        }
        Action::DefaultDevice { target, device_name } => {
            let pfx = match target {
                crate::models::AudioTarget::Playback => "AUDIO DEV",
                crate::models::AudioTarget::Microphone => "MIC DEV",
                crate::models::AudioTarget::Process => "PROC AUDIO",
            };
            format!("{} {}", pfx, device_name)
        }
        Action::ExecuteScript { interpreter, .. } => match interpreter {
            crate::models::ScriptType::PowerShell => "POWERSHELL".to_string(),
            crate::models::ScriptType::Batch => "CMD".to_string(),
            crate::models::ScriptType::Lua => "LUA".to_string(),
        }
        Action::Macro { events, repeat, speed, infinite } => {
            let rep_str = if *infinite {
                " ∞".to_string()
            } else if *repeat > 1 {
                format!(" {}x", repeat)
            } else {
                String::new()
            };
            let spd_str = if (*speed - 1.0).abs() > 0.05 { format!(" @{:.1}x", speed) } else { String::new() };
            format!("MACRO ({} ev){}{}", events.len(), rep_str, spd_str)
        }
        Action::Notification { title, .. } => format!("NOTIFY \"{}\"", title),
        Action::SystemPower { power_type } => format!("{} {:?}", loc.summary_action_power, power_type),
        Action::Delay { seconds } => format!("{} {:.1}S", loc.summary_action_wait, seconds),
        Action::HttpWebhook { .. } => "WEBHOOK".to_string(),
        Action::Window { window_action, target_process, .. } => {
            let proc = target_process.as_deref().unwrap_or("ACTIVE");
            format!("{:?} {}", window_action, proc)
        }
        Action::ProcessTuning { target_process, .. } => {
            let proc = target_process.as_deref().unwrap_or("ACTIVE");
            format!("TUNE {}", proc)
        }
        Action::PowerPlan { plan } => format!("{:?} POWER", plan),
        Action::PlaySound { .. } => "PLAY SOUND".to_string(),
        Action::Firewall { process_name, block } => if *block { format!("BLOCK {}", process_name) } else { format!("ALLOW {}", process_name) },
        Action::Display { width, height, refresh_rate, monitor_index } => {
            let res = match (width, height) {
                (Some(w), Some(h)) => format!("{}x{}", w, h),
                _ => "AUTO RES".to_string(),
            };
            let mon_prefix = monitor_index.map(|m| format!("[M{}] ", m)).unwrap_or_default();
            refresh_rate.map(|r| format!("DISPLAY {}{} @ {}Hz", mon_prefix, res, r)).unwrap_or_else(|| format!("DISPLAY {}{}", mon_prefix, res))
        }
        Action::IfElse { conditions, then_actions, else_actions, .. } => {
            let cond_summary = conditions
                .first()
                .map(|c| format_single_condition_brief(&c.condition))
                .unwrap_or_else(|| "COND".to_string());
            let count_suffix = if conditions.len() > 1 { format!("+{}", conditions.len() - 1) } else { String::new() };
            format!("IF {}{} [{} | {}]", cond_summary, count_suffix, then_actions.len(), else_actions.len())
        }
        Action::TrimWorkingSet { target_process, soft_limit_mb, .. } => {
            let proc = target_process.as_deref().unwrap_or("ACTIVE");
            match soft_limit_mb {
                Some(mb) => format!("RAM {} ({} MB)", proc, mb),
                None => format!("RAM {}", proc),
            }
        }
        Action::DiscordRpcSet { details, .. } => {
            if details.is_empty() {
                "DISCORD RPC".to_string()
            } else {
                format!("RPC: {}", details)
            }
        }
        Action::DiscordRpcClear => "CLEAR RPC".to_string(),
        Action::Archive { operation, format, destination_path, .. } => {
            let fname = std::path::Path::new(destination_path).file_name().unwrap_or_default().to_string_lossy();
            match operation {
                crate::models::ArchiveOperation::Pack => format!("PACK {:?} {}", format, fname),
                crate::models::ArchiveOperation::Unpack => format!("UNPACK {:?} {}", format, fname),
            }
        }
        Action::FileSystem { operation, path, .. } => {
            let fname = std::path::Path::new(path).file_name().unwrap_or_default().to_string_lossy();
            match operation {
                crate::models::FileSystemOperation::CreateFile => format!("MKFILE {}", fname),
                crate::models::FileSystemOperation::CreateDirectory => format!("MKDIR {}", fname),
                crate::models::FileSystemOperation::Rename => format!("RENAME {}", fname),
                crate::models::FileSystemOperation::Copy => format!("COPY {}", fname),
                crate::models::FileSystemOperation::Delete => format!("DEL {}", fname),
                crate::models::FileSystemOperation::WriteText => format!("WRITE {}", fname),
                crate::models::FileSystemOperation::AppendText => format!("APPEND {}", fname),
            }
        }
        Action::Overlay { operation, .. } => {
            match operation {
                crate::models::OverlayOperation::Toggle => "TOGGLE OVERLAY".to_string(),
                crate::models::OverlayOperation::Flash => "FLASH OVERLAY".to_string(),
                crate::models::OverlayOperation::Show => "SHOW OVERLAY".to_string(),
                crate::models::OverlayOperation::Hide => "HIDE OVERLAY".to_string(),
            }
        }
    }
}

fn format_trigger_display(
    trigger: &Option<Trigger>,
    loc: &crate::i18n::Strings,
) -> (&'static str, String) {
    match trigger {
        Some(Trigger::Process { process_name, single_instance, .. }) => {
            let si_suffix = if *single_instance { loc.rules_summary_single_instance } else { "" };
            (
                egui_phosphor::regular::APP_WINDOW,
                format!("PROC {}{}", process_name, si_suffix),
            )
        }
        Some(Trigger::WindowFocus { process_name, on_focus_gained, on_focus_lost }) => {
            let act = match (*on_focus_gained, *on_focus_lost) {
                (true, true) => "FOCUS/BLUR",
                (true, false) => "FOCUS",
                (false, true) => "BLUR",
                _ => "FOCUS",
            };
            (
                egui_phosphor::regular::BROWSERS,
                format!("WINDOW {} {}", act, process_name),
            )
        }
        Some(Trigger::ResourceThreshold { process_name, cpu_percent, ram_mb, .. }) => {
            let mut parts = Vec::new();
            if let Some(c) = cpu_percent { parts.push(format!("CPU > {}%", c)); }
            if let Some(m) = ram_mb { parts.push(format!("RAM > {}MB", m)); }
            (
                egui_phosphor::regular::GAUGE,
                format!("{} ({})", process_name, parts.join(", ")),
            )
        }
        Some(Trigger::GlobalHotkey { key_combination, hold_mode, repeat_while_held }) => {
            let hold_str = if *hold_mode {
                if *repeat_while_held { " [HOLD LOOP]" } else { " [HOLD]" }
            } else {
                ""
            };
            (
                egui_phosphor::regular::KEYBOARD,
                format!("{}{}", key_combination, hold_str),
            )
        }
        Some(Trigger::FileWatcher { folder_path, .. }) => {
            let name = std::path::Path::new(folder_path).file_name().and_then(|f| f.to_str()).unwrap_or(folder_path);
            (
                egui_phosphor::regular::FOLDER_NOTCH_OPEN,
                format!("FOLDER {}", name),
            )
        }
        Some(Trigger::Webhook { endpoint_path }) => {
            (
                egui_phosphor::regular::GLOBE,
                format!("/{}", endpoint_path.trim_start_matches('/')),
            )
        }
        Some(Trigger::SystemEvent { event_type, .. }) => {
            (
                egui_phosphor::regular::DESKTOP,
                format!("{:?}", event_type),
            )
        }
        Some(Trigger::Schedule { schedule_type, interval_sec, daily_time, day_of_month, target_datetime, .. }) => {
            let text = match schedule_type {
                crate::models::ScheduleType::Interval => {
                    let s = *interval_sec;
                    if s >= 31536000 && s % 31536000 == 0 {
                        loc.rules_summary_every_year.replace("{}", &(s / 31536000).to_string())
                    } else if s >= 2592000 && s % 2592000 == 0 {
                        loc.rules_summary_every_month.replace("{}", &(s / 2592000).to_string())
                    } else if s >= 604800 && s % 604800 == 0 {
                        loc.rules_summary_every_week.replace("{}", &(s / 604800).to_string())
                    } else if s >= 86400 && s % 86400 == 0 {
                        loc.rules_summary_every_day.replace("{}", &(s / 86400).to_string())
                    } else if s >= 3600 && s % 3600 == 0 {
                        loc.rules_summary_every_hour.replace("{}", &(s / 3600).to_string())
                    } else if s >= 60 && s % 60 == 0 {
                        loc.rules_summary_every_minute.replace("{}", &(s / 60).to_string())
                    } else {
                        loc.rules_summary_every_second.replace("{}", &s.to_string())
                    }
                }
                crate::models::ScheduleType::Daily => {
                    loc.rules_summary_trigger_daily.replace("{}", daily_time.as_deref().unwrap_or("12:00"))
                }
                crate::models::ScheduleType::Monthly => {
                    let day_str = day_of_month.unwrap_or(1).to_string();
                    let time_str = daily_time.as_deref().unwrap_or("12:00");
                    let res = loc.rules_summary_trigger_monthly.replacen("{}", &day_str, 1);
                    res.replacen("{}", time_str, 1)
                }
                crate::models::ScheduleType::OneShot => {
                    loc.rules_summary_trigger_once.replace("{}", target_datetime.as_deref().unwrap_or("SPECIFIED TIME"))
                }
                crate::models::ScheduleType::AppStartup => loc.rules_summary_trigger_startup.to_string(),
                crate::models::ScheduleType::AppShutdown => loc.rules_summary_trigger_shutdown.to_string(),
            };
            (egui_phosphor::regular::CLOCK, text)
        }
        Some(crate::models::Trigger::Manual) | None => (
            egui_phosphor::regular::HAND_TAP,
            loc.rules_badge_manual.to_string(),
        ),
    }
}

fn action_icon_glyph(action: &Action) -> &'static str {
    match action {
        Action::AudioControl { target, .. } => match target {
            crate::models::AudioTarget::Playback => egui_phosphor::regular::SPEAKER_HIGH,
            crate::models::AudioTarget::Microphone => egui_phosphor::regular::MICROPHONE,
            crate::models::AudioTarget::Process => egui_phosphor::regular::FADERS,
        },
        Action::AudioFormat { .. } => egui_phosphor::regular::EQUALIZER,
        Action::DefaultDevice { target, .. } => match target {
            crate::models::AudioTarget::Playback => egui_phosphor::regular::HEADPHONES,
            crate::models::AudioTarget::Microphone => egui_phosphor::regular::MICROPHONE,
            crate::models::AudioTarget::Process => egui_phosphor::regular::FADERS,
        },
        Action::ExecuteScript { interpreter, .. } => match interpreter {
            crate::models::ScriptType::PowerShell | crate::models::ScriptType::Lua => egui_phosphor::regular::CODE,
            crate::models::ScriptType::Batch => egui_phosphor::regular::TERMINAL_WINDOW,
        },
        Action::Macro { .. } => egui_phosphor::regular::KEYBOARD,
        Action::StartProcess { .. } => egui_phosphor::regular::ROCKET,
        Action::KillProcess { .. } => egui_phosphor::regular::STOP,
        Action::Window { .. } => egui_phosphor::regular::APP_WINDOW,
        Action::ProcessTuning { .. } => egui_phosphor::regular::GAUGE,
        Action::PowerPlan { .. } => egui_phosphor::regular::PLUG,
        Action::PlaySound { .. } => egui_phosphor::regular::MUSIC_NOTES,
        Action::Firewall { .. } => egui_phosphor::regular::SHIELD,
        Action::Notification { .. } => egui_phosphor::regular::BELL,
        Action::SystemPower { .. } => egui_phosphor::regular::POWER,
        Action::Delay { .. } => egui_phosphor::regular::HOURGLASS,
        Action::HttpWebhook { .. } => egui_phosphor::regular::GLOBE,
        Action::Display { .. } => egui_phosphor::regular::MONITOR,
        Action::IfElse { .. } => egui_phosphor::regular::GIT_FORK,
        Action::TrimWorkingSet { .. } => egui_phosphor::regular::BROOM,
        Action::DiscordRpcSet { .. } | Action::DiscordRpcClear => egui_phosphor::regular::GAME_CONTROLLER,
        Action::Archive { .. } => egui_phosphor::regular::ARCHIVE,
        Action::FileSystem { .. } => egui_phosphor::regular::FOLDER,
        Action::Overlay { .. } => egui_phosphor::regular::FRAME_CORNERS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{self, Language};

    #[test]
    fn test_format_single_condition_brief_and_tooltip() {
        let loc = i18n::get(Language::English);

        // 1. Process running condition
        let cond1 = RuleCondition::ProcessRunning {
            process_name: "notepad.exe".into(),
            match_mode: crate::models::ProcessMatchMode::Exact,
            must_be_running: true,
        };
        let brief1 = format_single_condition_brief(&cond1);
        assert_eq!(brief1, "RUN:notepad.exe");
        let tip1 = format_single_condition_tooltip(&cond1, 1, loc);
        assert!(tip1.contains("notepad.exe"));
        assert!(tip1.contains("running"));

        // 2. Window focused condition
        let cond2 = RuleCondition::WindowFocused {
            process_name: "code.exe".into(),
            match_mode: crate::models::ProcessMatchMode::Exact,
            must_be_focused: true,
        };
        let brief2 = format_single_condition_brief(&cond2);
        assert_eq!(brief2, "FOCUS:code.exe");
        let tip2 = format_single_condition_tooltip(&cond2, 2, loc);
        assert!(tip2.contains("code.exe"));
        assert!(tip2.contains("focused"));
    }

    #[test]
    fn test_format_trigger_display_no_prefix() {
        let loc = i18n::get(Language::English);
        let (_, trig_manual) = format_trigger_display(&Some(Trigger::Manual), loc);
        assert!(!trig_manual.starts_with("TRIGGER:"));
        assert!(!trig_manual.starts_with("ТРИГГЕР:"));

        let (_, trig_proc) = format_trigger_display(
            &Some(Trigger::Process {
                process_name: "notepad.exe".into(),
                match_mode: crate::models::ProcessMatchMode::Exact,
                single_instance: false,
            }),
            loc,
        );
        assert!(!trig_proc.starts_with("TRIGGER:"));
        assert!(!trig_proc.starts_with("ТРИГГЕР:"));
        assert!(trig_proc.contains("notepad.exe"));
    }

    #[test]
    fn test_rule_sorting_and_tag_filtering() {
        use crate::models::{Action, Rule, RuleTag, Trigger};

        let tag_work = RuleTag::new("Work").with_color("#3B82F6");
        let tag_dev = RuleTag::new("Dev").with_color("#10B981");

        let mut r1 = Rule::default();
        r1.id = "r1".into();
        r1.name = "Beta Work".into();
        r1.is_enabled = true;
        r1.tags = vec![tag_work.clone()];
        r1.actions = vec![Action::Delay { seconds: 1.0 }];

        let mut r2 = Rule::default();
        r2.id = "r2".into();
        r2.name = "Alpha Dev".into();
        r2.is_enabled = false;
        r2.tags = vec![tag_dev.clone()];
        r2.actions = vec![Action::Delay { seconds: 1.0 }, Action::Delay { seconds: 2.0 }];

        let mut r3 = Rule::default();
        r3.id = "r3".into();
        r3.name = "Gamma Work & Dev".into();
        r3.is_enabled = true;
        r3.tags = vec![tag_work, tag_dev];
        r3.trigger = Some(Trigger::Manual);

        let rules = vec![r1.clone(), r2.clone(), r3.clone()];

        // Filter by tag "Work"
        let filtered_work: Vec<_> = rules.iter().filter(|r| r.tags.iter().any(|t| t.name.eq_ignore_ascii_case("Work"))).collect();
        assert_eq!(filtered_work.len(), 2);
        assert_eq!(filtered_work[0].id, "r1");
        assert_eq!(filtered_work[1].id, "r3");

        // Filter by tag "Dev"
        let filtered_dev: Vec<_> = rules.iter().filter(|r| r.tags.iter().any(|t| t.name.eq_ignore_ascii_case("Dev"))).collect();
        assert_eq!(filtered_dev.len(), 2);
        assert_eq!(filtered_dev[0].id, "r2");
        assert_eq!(filtered_dev[1].id, "r3");

        // Sort by Name Asc
        let mut sorted_asc = rules.clone();
        sorted_asc.sort_by_key(|a| a.name.to_lowercase());
        assert_eq!(sorted_asc[0].name, "Alpha Dev");
        assert_eq!(sorted_asc[1].name, "Beta Work");
        assert_eq!(sorted_asc[2].name, "Gamma Work & Dev");

        // Sort by Name Desc
        let mut sorted_desc = rules.clone();
        sorted_desc.sort_by_key(|a| std::cmp::Reverse(a.name.to_lowercase()));
        assert_eq!(sorted_desc[0].name, "Gamma Work & Dev");
        assert_eq!(sorted_desc[1].name, "Beta Work");
        assert_eq!(sorted_desc[2].name, "Alpha Dev");

        // Sort by Action Count (most actions first)
        let mut sorted_actions = rules.clone();
        sorted_actions.sort_by_key(|r| std::cmp::Reverse(r.actions.len() + r.on_exit_actions.len()));
        assert_eq!(sorted_actions[0].id, "r2"); // 2 actions
        assert_eq!(sorted_actions[1].id, "r1"); // 1 action
        assert_eq!(sorted_actions[2].id, "r3"); // 0 actions

        // Sort by Status (active first)
        let mut sorted_status = rules;
        sorted_status.sort_by_key(|r| (!r.is_enabled, r.name.to_lowercase()));
        assert!(sorted_status[0].is_enabled);
        assert!(sorted_status[1].is_enabled);
        assert!(!sorted_status[2].is_enabled);
    }
}
