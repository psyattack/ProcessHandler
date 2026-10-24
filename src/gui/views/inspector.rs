//! Inspector Drawer: Slides in from the right edge with cubic easing,
//! dimmed background scrim, and craft-launcher styling for editing rules
//! and process details.

use egui::{
    CornerRadius, Id, LayerId, Order, Pos2, Rect, Sense, Stroke, Ui, emath::easing, pos2, vec2,
};
use crate::gui::components::script_highlighter::{create_script_layouter, ScriptLanguage};
use crate::gui::components::widgets::{self, Cell, Glyph, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    Action, ConditionOperator, PowerPlanType, ProcessMatchMode,
    ProcessPriorityLevel, ProcessSnapshotItem, Rule, RuleCondition, RuleConditionItem, ScheduleType,
    SystemEventType, SystemPowerType, Trigger, WindowActionType,
};
use chrono::{Datelike, Timelike};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Copy, Clone, PartialEq, Eq)]
enum TriggerCategory {
    Process,
    Window,
    Schedule,
    Events,
    System,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum SystemSubMode {
    Resource,
    Hotkey,
    FileWatcher,
    Webhook,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum EventTriggerKind {
    SessionLock,
    SessionUnlock,
    SystemSleep,
    SystemResume,
    PowerAc,
    PowerBattery,
    UserIdle,
    UserActive,
    AppStartup,
    AppShutdown,
}


fn egui_key_to_hotkey_name(key: egui::Key) -> Option<&'static str> {
    match key {
        egui::Key::A => Some("A"),
        egui::Key::B => Some("B"),
        egui::Key::C => Some("C"),
        egui::Key::D => Some("D"),
        egui::Key::E => Some("E"),
        egui::Key::F => Some("F"),
        egui::Key::G => Some("G"),
        egui::Key::H => Some("H"),
        egui::Key::I => Some("I"),
        egui::Key::J => Some("J"),
        egui::Key::K => Some("K"),
        egui::Key::L => Some("L"),
        egui::Key::M => Some("M"),
        egui::Key::N => Some("N"),
        egui::Key::O => Some("O"),
        egui::Key::P => Some("P"),
        egui::Key::Q => Some("Q"),
        egui::Key::R => Some("R"),
        egui::Key::S => Some("S"),
        egui::Key::T => Some("T"),
        egui::Key::U => Some("U"),
        egui::Key::V => Some("V"),
        egui::Key::W => Some("W"),
        egui::Key::X => Some("X"),
        egui::Key::Y => Some("Y"),
        egui::Key::Z => Some("Z"),
        egui::Key::Num0 => Some("0"),
        egui::Key::Num1 => Some("1"),
        egui::Key::Num2 => Some("2"),
        egui::Key::Num3 => Some("3"),
        egui::Key::Num4 => Some("4"),
        egui::Key::Num5 => Some("5"),
        egui::Key::Num6 => Some("6"),
        egui::Key::Num7 => Some("7"),
        egui::Key::Num8 => Some("8"),
        egui::Key::Num9 => Some("9"),
        egui::Key::F1 => Some("F1"),
        egui::Key::F2 => Some("F2"),
        egui::Key::F3 => Some("F3"),
        egui::Key::F4 => Some("F4"),
        egui::Key::F5 => Some("F5"),
        egui::Key::F6 => Some("F6"),
        egui::Key::F7 => Some("F7"),
        egui::Key::F8 => Some("F8"),
        egui::Key::F9 => Some("F9"),
        egui::Key::F10 => Some("F10"),
        egui::Key::F11 => Some("F11"),
        egui::Key::F12 => Some("F12"),
        egui::Key::Enter => Some("Enter"),
        egui::Key::Tab => Some("Tab"),
        egui::Key::Space => Some("Space"),
        egui::Key::Backspace => Some("Backspace"),
        egui::Key::Delete => Some("Delete"),
        egui::Key::Insert => Some("Insert"),
        egui::Key::Home => Some("Home"),
        egui::Key::End => Some("End"),
        egui::Key::PageUp => Some("PageUp"),
        egui::Key::PageDown => Some("PageDown"),
        egui::Key::ArrowUp => Some("Up"),
        egui::Key::ArrowDown => Some("Down"),
        egui::Key::ArrowLeft => Some("Left"),
        egui::Key::ArrowRight => Some("Right"),
        _ => None,
    }
}

fn format_hotkey_combination(key: egui::Key, modifiers: egui::Modifiers) -> Option<String> {
    let key_name = egui_key_to_hotkey_name(key)?;
    let mut parts = Vec::new();
    if modifiers.ctrl || modifiers.command {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.mac_cmd {
        parts.push("Win");
    }
    parts.push(key_name);
    Some(parts.join("+"))
}

pub fn render_inspector_drawer(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);

    let is_open = state.editing_rule.is_some() || state.selected_process.is_some();

    // Handle Esc key to close inspector (only if child modals/overlays are NOT open)
    if is_open
        && !state.is_modal_open()
        && ctx.input(|i| i.key_pressed(egui::Key::Escape))
    {
        state.close_editor();
        state.selected_process = None;
    }

    let k = ctx.animate_bool_with_time_and_easing(Id::new("inspector-slide"), is_open, 0.22, easing::cubic_out);
    if k <= 0.001 && !is_open {
        return;
    }

    let full = ctx.content_rect();
    let top = (full.top() + theme::HEADER_H).min(full.bottom());
    let bottom = (full.bottom() - theme::FOOTER_H).max(top);
    let area = Rect::from_min_max(pos2(full.left(), top), pos2(full.right(), bottom));

    let width = (area.width() * 0.42).clamp(460.0, 720.0).min(area.width());
    let left = area.right() - width * k;

    // 1. Scrim dimming over background (strictly to the left of the drawer)
    let scrim_w = (left - area.left()).max(0.0);
    if scrim_w > 0.0 {
        egui::Area::new(Id::new("inspector-scrim"))
            .order(Order::Middle)
            .fixed_pos(area.min)
            .show(ctx, |ui| {
                let scrim_size = vec2(scrim_w, area.height());
                let (rect, resp) = ui.allocate_exact_size(scrim_size, Sense::click());
                ui.painter().rect_filled(rect, CornerRadius::ZERO, theme::scrim(&t, k));
                if resp.on_hover_cursor(egui::CursorIcon::Default).clicked() {
                    state.close_editor();
                    state.selected_process = None;
                }
            });
    }

    // 2. Sliding Drawer Area
    egui::Area::new(Id::new("inspector-drawer"))
        .order(Order::Foreground)
        .fixed_pos(pos2(left, area.top()))
        .constrain(false)
        .show(ctx, |ui| {
            let panel = Rect::from_min_size(pos2(left, area.top()), vec2(width, area.height()));
            let p = ui.painter();
            p.add(theme::shadow(&t).as_shape(panel, CornerRadius::ZERO));
            p.rect_filled(panel, CornerRadius::ZERO, t.bg);
            p.vline(panel.left() + 0.5, panel.y_range(), Stroke::new(1.0, t.line_strong));

            ui.set_min_size(panel.size());
            ui.set_max_size(panel.size());
            ui.set_width(width);
            ui.set_max_width(width);
            ui.set_clip_rect(panel);

            // 1. Top Header Strip (only for selected_process inspection)
            let mut close_process = false;

            let header_resp = if let Some(proc) = &state.selected_process {
                let proc_name = proc.name.clone();
                let proc_pid = proc.pid;
                Some(egui::Frame::new()
                    .fill(t.sunken)
                    .stroke(t.hairline())
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(egui::Margin {
                        left: 20,
                        right: 14,
                        top: 3,
                        bottom: 3,
                    })
                    .show(ui, |ui| {
                        let loc = state.loc();
                        ui.spacing_mut().interact_size.y = 0.0;
                        ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
                        ui.set_width(ui.available_width());
                        ui.set_max_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spacing_mut().interact_size.y = 0.0;
                            ui.set_width(ui.available_width());
                            ui.vertical(|ui| {
                                ui.spacing_mut().interact_size.y = 0.0;
                                ui.spacing_mut().item_spacing.y = 1.0;
                                widgets::hud(ui, loc.inspector_eyebrow_task_details, t.muted);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().interact_size.y = 0.0;
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&proc_name)
                                                .family(theme::semibold())
                                                .size(14.5)
                                                .color(t.ink_strong),
                                        )
                                        .truncate(),
                                    );
                                    widgets::hud(ui, &format!("#{}", proc_pid), t.muted);
                                });
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_close_tip).clicked() {
                                    close_process = true;
                                }
                            });
                        });
                    }))
            } else {
                None
            };

            if close_process {
                state.selected_process = None;
            }

            let header_bottom = header_resp.map(|r| r.response.rect.bottom()).unwrap_or(panel.top());
            let is_editing_rule = state.editing_rule.is_some();
            let footer_h = if is_editing_rule { 52.0 } else { 0.0 };
            let body_rect = Rect::from_min_max(
                pos2(panel.left(), header_bottom),
                pos2(panel.right(), (panel.bottom() - footer_h).max(header_bottom)),
            );
            let footer_rect = Rect::from_min_max(
                pos2(panel.left(), panel.bottom() - footer_h),
                panel.max,
            );

            // 2. Scrollable Body with strictly pinned width and height
            ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                ui.set_width(body_rect.width());
                ui.set_max_width(body_rect.width());
                ui.set_height(body_rect.height());
                ui.set_max_height(body_rect.height());

                ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                egui::ScrollArea::vertical()
                    .id_salt("inspector_scroll_area")
                    .max_width(body_rect.width())
                    .max_height(body_rect.height())
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_max_width(ui.available_width());

                        // Keyboard shortcuts for Undo (Ctrl+Z) and Redo (Ctrl+Y or Ctrl+Shift+Z)
                        let (do_undo, do_redo) = ui.input_mut(|i| {
                            let u = i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)
                                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
                            let r = i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
                                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
                                || i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
                                || i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
                            (u, r)
                        });

                        let mut undid_or_redid = false;
                        if do_undo {
                            state.undo();
                            undid_or_redid = true;
                        } else if do_redo {
                            state.redo();
                            undid_or_redid = true;
                        }

                        if let Some(mut rule) = state.editing_rule.clone() {
                            let pre_rule = rule.clone();
                            render_rule_editor(ui, state, &mut rule, &t);
                            if state.editing_rule.is_some() {
                                if !undid_or_redid && rule != pre_rule && state.undo_stack.last() != Some(&pre_rule) {
                                    state.push_undo_snapshot_of(&pre_rule);
                                }
                                state.editing_rule = Some(rule);
                            }
                        } else if let Some(proc) = state.selected_process.clone() {
                            render_process_inspector(ui, state, &proc, &t);
                        }
                    });
            });

            // 3. Fixed Bottom Action Strip (Save / Cancel / Clone always visible and strictly pinned to drawer bottom)
            if let Some(rule) = state.editing_rule.clone() {
                ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                    egui::Frame::new()
                        .fill(t.sunken)
                        .stroke(t.hairline())
                        .corner_radius(CornerRadius::ZERO)
                        .inner_margin(egui::Margin::symmetric(20, 10))
                        .show(ui, |ui| {
                            let loc = state.loc();
                            ui.set_width(ui.available_width());
                            ui.set_max_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;

                                if widgets::button(ui, loc.inspector_btn_save_rule, Kind::Primary, 120.0).clicked() {
                                    state.save_rule(rule.clone());
                                }

                                if !state.is_creating_new_rule
                                    && widgets::button(ui, loc.inspector_btn_clone, Kind::Secondary, 0.0)
                                        .on_hover_text(loc.inspector_btn_clone_tip)
                                        .clicked()
                                {
                                    state.clone_rule(&rule);
                                }

                                if state.is_creating_new_rule
                                    && widgets::button(ui, loc.inspector_btn_reset, Kind::Secondary, 0.0)
                                        .on_hover_text(loc.inspector_btn_reset_tip)
                                        .clicked()
                                {
                                    state.reset_builder_state();
                                }

                                if widgets::button(ui, loc.inspector_btn_cancel, Kind::Secondary, 0.0).clicked() {
                                    state.close_editor();
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if widgets::glyph_button(ui, Glyph::Question, loc.footer_guide_tip).clicked() {
                                        state.show_guide_modal = true;
                                    }
                                });
                            });
                        });
                });
            }
        });
}

fn render_rule_editor(ui: &mut Ui, state: &mut GuiState, rule: &mut Rule, t: &Tokens) {
    state.ensure_processes();

    let scrollbar_w = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin + ui.spacing().scroll.bar_outer_margin;
    let right_m = (20.0 - scrollbar_w).max(0.0).round() as i8;

    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 20,
            right: right_m,
            top: 14,
            bottom: 20,
        })
        .show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_max_width(ui.available_width());
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.set_max_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;

            // 1. Basic Info & Icon Avatar
            let loc = state.loc();

            widgets::eyebrow(ui, "01", loc.inspector_eyebrow_task_details);

            egui::Frame::new()
                .fill(t.sunken)
                .stroke(t.hairline())
                .corner_radius(CornerRadius::ZERO)
                .inner_margin(egui::Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_max_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;

                            // Interactive Icon Avatar Box (38x38)
                            let icon_box_size = vec2(38.0, 38.0);
                            let (icon_rect, icon_resp) = ui.allocate_exact_size(icon_box_size, Sense::click());
                            let is_hovered = icon_resp.hovered();
                            let p = ui.painter();

                            p.rect_filled(icon_rect, CornerRadius::ZERO, if is_hovered { t.sunken } else { t.bg });
                            let stroke_color = if is_hovered { t.accent_ink } else { t.line_strong };
                            p.rect_stroke(icon_rect, CornerRadius::ZERO, Stroke::new(1.0, stroke_color), egui::StrokeKind::Inside);

                            let mut rendered = false;
                            if let Some(path) = &rule.custom_icon_path {
                                if let Some(tex) = state.icon_cache.get_icon_from_path(ui.ctx(), path) {
                                    let size = tex.size_vec2();
                                    let max_dim = size.x.max(size.y).max(1.0);
                                    let scale = 26.0 / max_dim;
                                    let render_size = vec2((size.x * scale).round().max(1.0), (size.y * scale).round().max(1.0));
                                    let img_rect = Rect::from_center_size(icon_rect.center(), render_size);
                                    p.image(tex.id(), img_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), egui::Color32::WHITE);
                                    rendered = true;
                                }
                            }
                            if !rendered {
                                let glyph = rule.default_icon_glyph();
                                p.text(icon_rect.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::new(18.0, theme::phosphor()), if is_hovered { t.accent_ink } else { t.ink_strong });
                            }

                            let icon_tooltip = match &rule.custom_icon_path {
                                Some(p) => {
                                    let fname = std::path::Path::new(p).file_name().unwrap_or_default().to_string_lossy();
                                    format!("{}: {}\n{}", loc.inspector_icon_custom_label, fname, loc.inspector_icon_browse_tip)
                                }
                                None => format!("{}\n{}", loc.inspector_icon_default_label, loc.inspector_icon_browse_tip),
                            };
                            let icon_resp = icon_resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(icon_tooltip);

                            if icon_resp.clicked() {
                                if let Some(picked) = rfd::FileDialog::new()
                                    .set_title("Select Rule Icon")
                                    .add_filter("Icon / Image / Exe", &["exe", "ico", "png", "jpg", "jpeg", "dll"])
                                    .pick_file()
                                {
                                    rule.custom_icon_path = Some(picked.to_string_lossy().to_string());
                                }
                            }
                            if icon_resp.secondary_clicked() && rule.custom_icon_path.is_some() {
                                rule.custom_icon_path = None;
                            }

                            // Name Input taking remaining available width
                            let input_w = if rule.custom_icon_path.is_some() {
                                (ui.available_width() - 36.0).max(100.0)
                            } else {
                                ui.available_width()
                            };

                            ui.add(
                                egui::TextEdit::singleline(&mut rule.name)
                                    .hint_text(loc.rules_default_new_name)
                                    .desired_width(input_w)
                                    .min_size(vec2(0.0, 38.0))
                                    .font(egui::FontId::new(14.0, theme::semibold()))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 9.0)),
                            );

                            if rule.custom_icon_path.is_some()
                                && widgets::glyph_button(ui, Glyph::Reset, loc.inspector_icon_reset_tip).clicked()
                            {
                                rule.custom_icon_path = None;
                            }
                        });

                        // Description Input
                        ui.add(
                            egui::TextEdit::singleline(&mut rule.description)
                                .hint_text(loc.inspector_field_description_hint)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );

                        // Metadata row: Cooldown + Status + Auto process icon
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 16.0;
                            ui.horizontal(|ui| {
                                widgets::hud(ui, loc.inspector_field_cooldown, t.muted);
                                ui.add(egui::DragValue::new(&mut rule.cooldown_seconds).speed(0.1).range(0.0..=60.0).suffix(" s"));
                            });
                            ui.checkbox(&mut rule.is_enabled, loc.inspector_checkbox_rule_active);
                            ui.checkbox(&mut rule.auto_process_icon, loc.inspector_icon_auto_process)
                                .on_hover_text(loc.inspector_icon_auto_process_tip);
                        });

                        // Execution Quota (auto-pause when reached)
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            let mut has_quota = rule.max_executions.is_some();
                            if ui.checkbox(&mut has_quota, loc.inspector_field_max_executions).changed() {
                                rule.max_executions = if has_quota { Some(10) } else { None };
                            }
                            if let Some(limit) = &mut rule.max_executions {
                                if widgets::glyph_button(ui, Glyph::Reset, "Reset execution counter").clicked() {
                                    rule.execution_count = 0;
                                }
                                ui.add(egui::DragValue::new(limit).range(1..=100000).suffix(" runs"));
                                let remaining = limit.saturating_sub(rule.execution_count);
                                let note = loc.inspector_runs_remaining
                                    .replacen("{}", &remaining.to_string(), 1)
                                    .replacen("{}", &limit.to_string(), 1);
                                widgets::hud(ui, &note, t.faint);
                            }
                            ui.checkbox(&mut rule.stop_on_error, loc.inspector_field_stop_on_error)
                                .on_hover_text(loc.inspector_field_stop_on_error_tip);
                        });
                    });
                });

            // 2. Trigger Section
            widgets::eyebrow(ui, "02", loc.inspector_eyebrow_trigger);

            let mut switch_trigger_to_hunter = false;

            egui::Frame::new()
                .fill(t.sunken)
                .stroke(t.hairline())
                .corner_radius(CornerRadius::ZERO)
                .inner_margin(egui::Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_max_width(ui.available_width());

                    let mut category = match &rule.trigger {
                        Some(Trigger::Process { .. }) => TriggerCategory::Process,
                        Some(Trigger::WindowFocus { .. }) => TriggerCategory::Window,
                        Some(Trigger::Schedule { schedule_type, .. }) => match schedule_type {
                            ScheduleType::AppStartup | ScheduleType::AppShutdown => TriggerCategory::Events,
                            _ => TriggerCategory::Schedule,
                        },
                        Some(Trigger::SystemEvent { .. }) => TriggerCategory::Events,
                        Some(Trigger::ResourceThreshold { .. }) | Some(Trigger::GlobalHotkey { .. }) => TriggerCategory::System,
                        Some(Trigger::FileWatcher { .. }) | Some(Trigger::Webhook { .. }) => TriggerCategory::System,
                        None => TriggerCategory::Process,
                    };

                    let cat_options = [
                        (TriggerCategory::Process, Cell::new("PROCESS", "")),
                        (TriggerCategory::Window, Cell::new("WINDOW", "")),
                        (TriggerCategory::Schedule, Cell::new(loc.inspector_trigger_tab_schedule, "")),
                        (TriggerCategory::Events, Cell::new(loc.inspector_trigger_tab_events, "")),
                        (TriggerCategory::System, Cell::new("SYSTEM", "")),
                    ];

                    let prev_cat = category;
                    if widgets::cells(ui, &mut category, &cat_options) && category != prev_cat {
                        match category {
                            TriggerCategory::Process => {
                                let proc = match &rule.trigger {
                                    Some(Trigger::WindowFocus { process_name, .. }) => process_name.clone(),
                                    _ => String::new(),
                                };
                                rule.trigger = Some(Trigger::Process {
                                    process_name: proc,
                                    match_mode: ProcessMatchMode::Exact,
                                    single_instance: false,
                                });
                            }
                            TriggerCategory::Window => {
                                let proc = match &rule.trigger {
                                    Some(Trigger::Process { process_name, .. }) => process_name.clone(),
                                    _ => String::new(),
                                };
                                rule.trigger = Some(Trigger::WindowFocus {
                                    process_name: proc,
                                    on_focus_gained: true,
                                    on_focus_lost: false,
                                });
                            }
                            TriggerCategory::Schedule => {
                                rule.trigger = Some(Trigger::Schedule {
                                    schedule_type: ScheduleType::Interval,
                                    interval_sec: 60,
                                    daily_time: Some("12:00:00".into()),
                                    days_of_week: Vec::new(),
                                    delay_sec: 0,
                                    target_datetime: None,
                                });
                            }
                            TriggerCategory::Events => {
                                rule.trigger = Some(Trigger::Schedule {
                                    schedule_type: ScheduleType::AppStartup,
                                    interval_sec: 0,
                                    daily_time: None,
                                    days_of_week: Vec::new(),
                                    delay_sec: 0,
                                    target_datetime: None,
                                });
                            }
                            TriggerCategory::System => {
                                rule.trigger = Some(Trigger::ResourceThreshold {
                                    process_name: String::new(),
                                    cpu_percent: Some(80.0),
                                    ram_mb: None,
                                    duration_sec: 5,
                                });
                            }
                        }
                    }

                    ui.add_space(10.0);

                    match category {
                        TriggerCategory::Process => {
                                    if let Some(Trigger::Process { process_name, match_mode, single_instance }) = &mut rule.trigger {
                                        widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                                        ui.horizontal(|ui| {
                                            ui.set_width(ui.available_width());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if render_process_picker(ui, loc) {
                                                    switch_trigger_to_hunter = true;
                                                }
                                                ui.add(
                                                    egui::TextEdit::singleline(process_name)
                                                        .hint_text(loc.inspector_field_target_process_hint)
                                                        .desired_width(ui.available_width())
                                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                                        .vertical_align(egui::Align::Center)
                                                        .margin(vec2(10.0, 8.0)),
                                                );
                                            });
                                        });

                                        ui.add_space(6.0);
                                        widgets::hud(ui, loc.inspector_field_match_mode, t.muted);
                                        let match_options = [
                                            (ProcessMatchMode::Exact, Cell::new(loc.inspector_match_tab_exact, "")),
                                            (ProcessMatchMode::Wildcard, Cell::new(loc.inspector_match_tab_wildcard, "")),
                                            (ProcessMatchMode::Regex, Cell::new(loc.inspector_match_tab_regex, "")),
                                        ];
                                        widgets::cells(ui, match_mode, &match_options);

                                        ui.add_space(6.0);
                                        ui.checkbox(single_instance, loc.inspector_checkbox_single_instance);
                            }
                        }

                        TriggerCategory::Window => {
                            if let Some(Trigger::WindowFocus { process_name, on_focus_gained, on_focus_lost }) = &mut rule.trigger {
                                        widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                                        ui.horizontal(|ui| {
                                            ui.set_width(ui.available_width());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if render_process_picker(ui, loc) {
                                                    switch_trigger_to_hunter = true;
                                                }
                                                ui.add(
                                                    egui::TextEdit::singleline(process_name)
                                                        .hint_text(loc.inspector_field_target_process_hint)
                                                        .desired_width(ui.available_width())
                                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                                        .vertical_align(egui::Align::Center)
                                                        .margin(vec2(10.0, 8.0)),
                                                );
                                            });
                                        });

                                        ui.add_space(4.0);
                                        widgets::hud(ui, "EVENT TYPE", t.muted);
                                        ui.horizontal_wrapped(|ui| {
                                            if ui.checkbox(on_focus_gained, "Gained Focus").changed()
                                                && !*on_focus_gained && !*on_focus_lost
                                            {
                                                *on_focus_lost = true;
                                            }
                                            if ui.checkbox(on_focus_lost, "Lost Focus").changed()
                                                && !*on_focus_lost && !*on_focus_gained
                                            {
                                                *on_focus_gained = true;
                                            }
                                        });
                                    }
                                }

                        TriggerCategory::Schedule => {
                            if let Some(Trigger::Schedule { schedule_type, interval_sec, daily_time, days_of_week, target_datetime, .. }) = &mut rule.trigger {
                                render_schedule_trigger(
                                    ui,
                                    loc,
                                    schedule_type,
                                    interval_sec,
                                    daily_time,
                                    days_of_week,
                                    target_datetime,
                                    t,
                                );
                            }
                        }

                        TriggerCategory::Events => {
                            let mut current_event = match &rule.trigger {
                                Some(Trigger::Schedule { schedule_type: ScheduleType::AppStartup, .. }) => EventTriggerKind::AppStartup,
                                Some(Trigger::Schedule { schedule_type: ScheduleType::AppShutdown, .. }) => EventTriggerKind::AppShutdown,
                                Some(Trigger::SystemEvent { event_type, .. }) => match event_type {
                                    SystemEventType::SessionLock => EventTriggerKind::SessionLock,
                                    SystemEventType::SessionUnlock => EventTriggerKind::SessionUnlock,
                                    SystemEventType::SystemSleep => EventTriggerKind::SystemSleep,
                                    SystemEventType::SystemResume => EventTriggerKind::SystemResume,
                                    SystemEventType::PowerSourceAc => EventTriggerKind::PowerAc,
                                    SystemEventType::PowerSourceBattery => EventTriggerKind::PowerBattery,
                                    SystemEventType::UserIdle => EventTriggerKind::UserIdle,
                                    SystemEventType::UserActive => EventTriggerKind::UserActive,
                                    _ => EventTriggerKind::AppStartup,
                                },
                                _ => EventTriggerKind::AppStartup,
                            };

                            let events = [
                                (EventTriggerKind::AppStartup, format!("{} {}", egui_phosphor::regular::ROCKET_LAUNCH, loc.inspector_schedule_app_start)),
                                (EventTriggerKind::AppShutdown, format!("{} {}", egui_phosphor::regular::POWER, loc.inspector_schedule_app_close)),
                                (EventTriggerKind::SessionLock, format!("{} {}", egui_phosphor::regular::LOCK, loc.inspector_sys_event_lock)),
                                (EventTriggerKind::SessionUnlock, format!("{} {}", egui_phosphor::regular::LOCK_OPEN, loc.inspector_sys_event_unlock)),
                                (EventTriggerKind::SystemSleep, format!("{} {}", egui_phosphor::regular::MOON, loc.inspector_sys_event_sleep)),
                                (EventTriggerKind::SystemResume, format!("{} {}", egui_phosphor::regular::SUN, loc.inspector_sys_event_resume)),
                                (EventTriggerKind::PowerAc, format!("{} {}", egui_phosphor::regular::PLUG, loc.inspector_sys_event_ac)),
                                (EventTriggerKind::PowerBattery, format!("{} {}", egui_phosphor::regular::BATTERY_CHARGING, loc.inspector_sys_event_battery)),
                                (EventTriggerKind::UserIdle, format!("{} {}", egui_phosphor::regular::HOURGLASS, loc.inspector_sys_event_idle)),
                                (EventTriggerKind::UserActive, format!("{} {}", egui_phosphor::regular::ACTIVITY, loc.inspector_sys_event_active)),
                            ];

                            widgets::hud(ui, loc.inspector_field_sys_event_type, t.muted);

                            let selected_label = events.iter().find(|(k, _)| *k == current_event).map(|(_, l)| l.as_str()).unwrap_or("");
                            let prev_event = current_event;

                            egui::ComboBox::from_id_salt("unified_events_combo")
                                .selected_text(selected_label)
                                .width(ui.available_width() - 8.0)
                                .show_ui(ui, |ui| {
                                    for (k, label) in &events {
                                        ui.selectable_value(&mut current_event, *k, label);
                                    }
                                });

                            if current_event != prev_event {
                                match current_event {
                                    EventTriggerKind::AppStartup => {
                                        rule.trigger = Some(Trigger::Schedule {
                                            schedule_type: ScheduleType::AppStartup,
                                            interval_sec: 0,
                                            daily_time: None,
                                            days_of_week: Vec::new(),
                                            delay_sec: 0,
                                            target_datetime: None,
                                        });
                                    }
                                    EventTriggerKind::AppShutdown => {
                                        rule.trigger = Some(Trigger::Schedule {
                                            schedule_type: ScheduleType::AppShutdown,
                                            interval_sec: 0,
                                            daily_time: None,
                                            days_of_week: Vec::new(),
                                            delay_sec: 0,
                                            target_datetime: None,
                                        });
                                    }
                                    _ => {
                                        let sys_ev = match current_event {
                                            EventTriggerKind::SessionLock => SystemEventType::SessionLock,
                                            EventTriggerKind::SessionUnlock => SystemEventType::SessionUnlock,
                                            EventTriggerKind::SystemSleep => SystemEventType::SystemSleep,
                                            EventTriggerKind::SystemResume => SystemEventType::SystemResume,
                                            EventTriggerKind::PowerAc => SystemEventType::PowerSourceAc,
                                            EventTriggerKind::PowerBattery => SystemEventType::PowerSourceBattery,
                                            EventTriggerKind::UserIdle => SystemEventType::UserIdle,
                                            EventTriggerKind::UserActive => SystemEventType::UserActive,
                                            _ => SystemEventType::SessionLock,
                                        };
                                        rule.trigger = Some(Trigger::SystemEvent {
                                            event_type: sys_ev,
                                            idle_duration_sec: 300,
                                            device_id_filter: None,
                                        });
                                    }
                                }
                            }

                            if let Some(Trigger::SystemEvent { event_type: SystemEventType::UserIdle, idle_duration_sec, .. }) = &mut rule.trigger {
                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    widgets::hud(ui, loc.inspector_field_idle_duration, t.muted);
                                    ui.add(egui::DragValue::new(idle_duration_sec).range(10..=3600).suffix(" s"));
                                });
                            }
                        }

                        TriggerCategory::System => {
                            let mut submode = match &rule.trigger {
                                Some(Trigger::GlobalHotkey { .. }) => SystemSubMode::Hotkey,
                                Some(Trigger::FileWatcher { .. }) => SystemSubMode::FileWatcher,
                                Some(Trigger::Webhook { .. }) => SystemSubMode::Webhook,
                                _ => SystemSubMode::Resource,
                            };
                            let sub_options = [
                                (SystemSubMode::Resource, Cell::new("RESOURCE", "Trigger on CPU / RAM threshold")),
                                (SystemSubMode::Hotkey, Cell::new("HOTKEY", "Trigger on keyboard shortcut")),
                                (SystemSubMode::FileWatcher, Cell::new("FILE WATCHER", "Trigger on file or folder change")),
                                (SystemSubMode::Webhook, Cell::new("WEBHOOK", "Trigger via local HTTP endpoint")),
                            ];
                            let prev_sub = submode;
                            if widgets::cells(ui, &mut submode, &sub_options) && submode != prev_sub {
                                state.push_undo_snapshot();
                                match submode {
                                    SystemSubMode::Resource => {
                                        rule.trigger = Some(Trigger::ResourceThreshold {
                                            process_name: String::new(),
                                            cpu_percent: Some(80.0),
                                            ram_mb: None,
                                            duration_sec: 5,
                                        });
                                    }
                                    SystemSubMode::Hotkey => {
                                        rule.trigger = Some(Trigger::GlobalHotkey {
                                            key_combination: String::new(),
                                            hold_mode: false,
                                            repeat_while_held: false,
                                        });
                                    }
                                    SystemSubMode::FileWatcher => {
                                        rule.trigger = Some(Trigger::FileWatcher {
                                            folder_path: String::new(),
                                            filter_extension: None,
                                            on_created: true,
                                            on_modified: false,
                                            on_deleted: false,
                                        });
                                    }
                                    SystemSubMode::Webhook => {
                                        rule.trigger = Some(Trigger::Webhook {
                                            endpoint_path: String::new(),
                                        });
                                    }
                                }
                            }
                            ui.add_space(8.0);

                            match submode {
                                SystemSubMode::Resource => {
                                    if let Some(Trigger::ResourceThreshold { process_name, cpu_percent, ram_mb, duration_sec }) = &mut rule.trigger {
                                        widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                                        ui.horizontal(|ui| {
                                            ui.set_width(ui.available_width());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if render_process_picker(ui, loc) {
                                                    switch_trigger_to_hunter = true;
                                                }
                                                ui.add(
                                                    egui::TextEdit::singleline(process_name)
                                                        .hint_text("Process executable")
                                                        .desired_width(ui.available_width())
                                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                                        .vertical_align(egui::Align::Center)
                                                        .margin(vec2(10.0, 8.0)),
                                                );
                                            });
                                        });

                                        ui.add_space(6.0);
                                        ui.horizontal_wrapped(|ui| {
                                            let mut has_cpu = cpu_percent.is_some();
                                            if ui.checkbox(&mut has_cpu, "CPU >").changed() {
                                                *cpu_percent = if has_cpu { Some(80.0) } else { None };
                                            }
                                            if let Some(cpu) = cpu_percent {
                                                ui.add(egui::DragValue::new(cpu).range(1.0..=100.0).suffix("%"));
                                            }

                                            ui.add_space(10.0);
                                            let mut has_mem = ram_mb.is_some();
                                            if ui.checkbox(&mut has_mem, "RAM >").changed() {
                                                *ram_mb = if has_mem { Some(1024) } else { None };
                                            }
                                            if let Some(mem) = ram_mb {
                                                ui.add(egui::DragValue::new(mem).range(10..=65536).suffix(" MB"));
                                            }
                                        });

                                        ui.add_space(4.0);
                                        ui.horizontal_wrapped(|ui| {
                                            widgets::hud(ui, "DURATION THRESHOLD:", t.muted);
                                            ui.add(egui::DragValue::new(duration_sec).range(1..=3600).suffix(" s"));
                                        });
                                    }
                                }
                                SystemSubMode::Hotkey => {
                                    if let Some(Trigger::GlobalHotkey { key_combination, hold_mode, repeat_while_held }) = &mut rule.trigger {
                                        widgets::hud(ui, "HOTKEY COMBINATION", t.muted);
                                        let rec_id = ui.id().with("rec_global_hotkey");
                                        let mut is_recording: bool = ui.data_mut(|d| d.get_temp(rec_id).unwrap_or(false));

                                        if is_recording {
                                            ui.ctx().input(|i| {
                                                for event in &i.raw.events {
                                                    match event {
                                                        egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } => {
                                                            if *key == egui::Key::Escape {
                                                                is_recording = false;
                                                                break;
                                                            }
                                                            if let Some(combo) = format_hotkey_combination(*key, *modifiers) {
                                                                *key_combination = combo;
                                                                is_recording = false;
                                                                break;
                                                            }
                                                        }
                                                        egui::Event::PointerButton { button, pressed: true, modifiers, .. } => {
                                                            let btn_str = match button {
                                                                egui::PointerButton::Primary => "LButton",
                                                                egui::PointerButton::Secondary => "RButton",
                                                                egui::PointerButton::Middle => "MButton",
                                                                egui::PointerButton::Extra1 => "M4",
                                                                egui::PointerButton::Extra2 => "M5",
                                                            };
                                                            let mut parts = Vec::new();
                                                            if modifiers.ctrl || modifiers.command { parts.push("Ctrl"); }
                                                            if modifiers.alt { parts.push("Alt"); }
                                                            if modifiers.shift { parts.push("Shift"); }
                                                            parts.push(btn_str);
                                                            *key_combination = parts.join("+");
                                                            is_recording = false;
                                                            break;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            });
                                            ui.data_mut(|d| d.insert_temp(rec_id, is_recording));
                                            ui.ctx().request_repaint();
                                        }

                                        ui.horizontal(|ui| {
                                            ui.set_width(ui.available_width());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let rec_label = if is_recording { "● LISTENING..." } else { "RECORD" };
                                                let rec_kind = if is_recording { Kind::Primary } else { Kind::Secondary };
                                                if widgets::button(ui, rec_label, rec_kind, 0.0)
                                                    .on_hover_text("Press keys or mouse buttons (M4/M5/RMB/etc), Esc to cancel")
                                                    .clicked()
                                                {
                                                    is_recording = !is_recording;
                                                    ui.data_mut(|d| d.insert_temp(rec_id, is_recording));
                                                }
                                                ui.add(
                                                    egui::TextEdit::singleline(key_combination)
                                                        .hint_text("Ctrl+Shift+P or M4, M5...")
                                                        .desired_width(ui.available_width())
                                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                                        .vertical_align(egui::Align::Center)
                                                        .margin(vec2(10.0, 8.0)),
                                                );
                                            });
                                        });

                                        ui.add_space(6.0);
                                        ui.checkbox(hold_mode, loc.inspector_hotkey_hold_mode).on_hover_text(loc.inspector_hotkey_hold_mode_tip);

                                        ui.add_space(4.0);
                                        if *hold_mode {
                                            ui.checkbox(repeat_while_held, loc.inspector_hotkey_repeat_while_held).on_hover_text(loc.inspector_hotkey_repeat_while_held_tip);
                                        } else {
                                            ui.checkbox(repeat_while_held, loc.inspector_hotkey_repeat_in_loop).on_hover_text(loc.inspector_hotkey_repeat_in_loop_tip);
                                        }
                                    }
                                }
                                SystemSubMode::FileWatcher => {
                                    if let Some(Trigger::FileWatcher { folder_path, filter_extension, on_created, on_modified, on_deleted }) = &mut rule.trigger {
                                        widgets::hud(ui, "WATCH PATH", t.muted);
                                        ui.horizontal(|ui| {
                                            ui.set_width(ui.available_width());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if widgets::button(ui, "FILE", Kind::Secondary, 0.0)
                                                    .on_hover_text("Select a specific file to monitor")
                                                    .clicked()
                                                {
                                                    if let Some(f) = rfd::FileDialog::new().pick_file() {
                                                        *folder_path = f.to_string_lossy().to_string();
                                                    }
                                                }
                                                if widgets::button(ui, "FOLDER", Kind::Secondary, 0.0)
                                                    .on_hover_text("Select a directory/folder to monitor")
                                                    .clicked()
                                                {
                                                    if let Some(f) = rfd::FileDialog::new().pick_folder() {
                                                        *folder_path = f.to_string_lossy().to_string();
                                                    }
                                                }
                                                ui.add(
                                                    egui::TextEdit::singleline(folder_path)
                                                        .hint_text("C:\\Path\\To\\Folder")
                                                        .desired_width(ui.available_width())
                                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                                        .vertical_align(egui::Align::Center)
                                                        .margin(vec2(10.0, 8.0)),
                                                );
                                            });
                                        });

                                        ui.add_space(4.0);
                                        widgets::hud(ui, "EXTENSION FILTER", t.muted);
                                        let mut ext = filter_extension.clone().unwrap_or_default();
                                        let max_w = ui.available_width().min(240.0);
                                        if ui.add(
                                            egui::TextEdit::singleline(&mut ext)
                                                .hint_text(".log, .png")
                                                .desired_width(max_w)
                                                .min_size(vec2(0.0, theme::CONTROL_H))
                                                .vertical_align(egui::Align::Center)
                                                .margin(vec2(10.0, 8.0)),
                                        ).changed() {
                                            *filter_extension = if ext.trim().is_empty() { None } else { Some(ext.trim().to_string()) };
                                        }

                                        ui.add_space(4.0);
                                        ui.horizontal_wrapped(|ui| {
                                            ui.checkbox(on_created, "On Created");
                                            ui.checkbox(on_modified, "On Modified");
                                            ui.checkbox(on_deleted, "On Deleted");
                                        });
                                    }
                                }
                                SystemSubMode::Webhook => {
                                    if let Some(Trigger::Webhook { endpoint_path }) = &mut rule.trigger {
                                        widgets::hud(ui, "ENDPOINT ID", t.muted);
                                        ui.add(
                                            egui::TextEdit::singleline(endpoint_path)
                                                .hint_text("endpoint_id")
                                                .desired_width(ui.available_width())
                                                .min_size(vec2(0.0, theme::CONTROL_H))
                                                .vertical_align(egui::Align::Center)
                                                .margin(vec2(10.0, 8.0)),
                                        );

                                        let endpoint_display = if endpoint_path.trim().is_empty() {
                                            rule.id.as_str()
                                        } else {
                                            endpoint_path.as_str()
                                        };
                                        let full_url = format!("http://127.0.0.1:48123/api/v1/trigger/{}", endpoint_display.trim_start_matches('/'));

                                        ui.add_space(6.0);
                                        let url_label = egui::RichText::new(&full_url)
                                            .font(egui::FontId::new(12.0, theme::mono()))
                                            .color(t.accent_ink)
                                            .underline();
                                        let resp = ui.add(egui::Label::new(url_label).sense(egui::Sense::click()))
                                            .on_hover_text("Click to copy endpoint URL")
                                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                                        if resp.clicked() {
                                            crate::sharing::copy_text_to_clipboard(ui.ctx(), &full_url);
                                            state.set_toast("URL copied to clipboard");
                                        }
                                    }
                                }
                            }
                        }
                    }
                });

            if switch_trigger_to_hunter {
                state.open_process_picker(crate::gui::state::ProcessPickerTarget::Trigger);
            }

            ui.add_space(8.0);

            // 3. Pre-conditions (Section 03)
            widgets::eyebrow(ui, "03", loc.inspector_eyebrow_conditions);
            render_conditions_section(ui, state, rule, t);

            ui.add_space(8.0);

            let is_process_trigger = matches!(rule.trigger, Some(Trigger::Process { .. }));

            // 4. Action Sequence Builder (Execution on trigger)
            let action_header = if is_process_trigger {
                loc.inspector_eyebrow_actions_start
            } else {
                loc.inspector_eyebrow_actions_pipeline
            };
            widgets::eyebrow(ui, "04", action_header);

            let exec_opt = state.daemon_state.as_ref().and_then(|s| s.active_rule_executions.get(&rule.id)).copied();

            if let Some(idx) = render_actions_pipeline(
                ui,
                state,
                &mut rule.actions,
                "main_actions",
                false,
                exec_opt,
                t,
            ) {
                state.open_process_picker(crate::gui::state::ProcessPickerTarget::Action(idx));
            }

            // 5. Actions on Process Exit
            if is_process_trigger {
                ui.add_space(8.0);
                widgets::eyebrow(ui, "05", loc.inspector_eyebrow_actions_exit);

                if let Some(idx) = render_actions_pipeline(
                    ui,
                    state,
                    &mut rule.on_exit_actions,
                    "exit_actions",
                    true,
                    exec_opt,
                    t,
                ) {
                    state.open_process_picker(crate::gui::state::ProcessPickerTarget::ExitAction(idx));
                }
            }

        });
    });
}

fn render_add_action_button(
    ui: &mut Ui,
    state: &mut GuiState,
    loc: &crate::i18n::Strings,
    is_exit_pipeline: bool,
    t: &Tokens,
) {
    ui.add_space(2.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let is_hov = resp.hovered();
    let p = ui.painter();

    let border = if is_hov {
        Stroke::new(1.0, t.accent_ink)
    } else {
        Stroke::new(1.0, t.muted.gamma_multiply(0.35))
    };
    p.rect_stroke(rect, CornerRadius::same(2), border, egui::StrokeKind::Inside);

    let text_color = if is_hov { t.accent_ink } else { t.faint };
    let plus_galley = p.layout_no_wrap(
        egui_phosphor::regular::PLUS.to_string(),
        egui::FontId::new(12.0, theme::phosphor()),
        text_color,
    );
    let label_galley = widgets::caps(ui, loc.inspector_btn_open_catalog, 10.0, theme::mono(), 0.8, text_color);

    let spacing = 8.0;
    let plus_w = plus_galley.size().x;
    let plus_h = plus_galley.size().y;
    let total_w = plus_w + spacing + label_galley.size().x;
    let start_x = rect.center().x - total_w * 0.5;

    let plus_pos = pos2(start_x, rect.center().y - plus_h * 0.5);
    p.galley(plus_pos, plus_galley, text_color);

    let label_pos = pos2(start_x + plus_w + spacing, rect.center().y - label_galley.size().y * 0.5);
    p.galley_with_override_text_color(label_pos, label_galley, text_color);

    let resp = resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(loc.inspector_btn_add_action_tip);

    if resp.clicked() {
        state.open_action_catalog(is_exit_pipeline, None);
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct DndDragSession {
    source_idx: usize,
    target_idx: usize,
    card_width: f32,
}

fn action_type_badge(action: &Action, loc: &crate::i18n::Strings) -> &'static str {
    match action {
        Action::StartProcess { close_on_exit, .. } => {
            if *close_on_exit {
                loc.inspector_badge_launch_companion
            } else {
                loc.inspector_badge_launch_process
            }
        }
        Action::KillProcess { .. } => loc.inspector_badge_kill_process,
        Action::Notification { .. } => loc.inspector_badge_notification,
        Action::SystemPower { .. } => loc.inspector_badge_system_power,
        Action::Delay { .. } => loc.inspector_badge_delay,
        Action::HttpWebhook { .. } => loc.inspector_badge_webhook,
        Action::Window { .. } => loc.inspector_badge_window,
        Action::ProcessTuning { .. } => loc.inspector_badge_tuning,
        Action::PowerPlan { .. } => loc.inspector_badge_power_plan,
        Action::PlaySound { .. } => loc.inspector_badge_play_sound,
        Action::Firewall { .. } => loc.inspector_badge_firewall,
        Action::Display { .. } => loc.inspector_badge_display,
        Action::AudioControl { .. } => loc.inspector_badge_audio_control,
        Action::DefaultDevice { .. } => loc.inspector_badge_default_device,
        Action::ExecuteScript { .. } => loc.inspector_badge_execute_script,
        Action::Macro { .. } => loc.inspector_badge_macro,
        Action::IfElse { .. } => loc.inspector_badge_if_else,
        Action::AudioFormat { .. } => loc.inspector_badge_audio_format,
    }
}

fn action_summary_preview(action: &Action) -> String {
    match action {
        Action::StartProcess { file_path, .. } => {
            let name = std::path::Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path);
            if name.is_empty() {
                "Launch executable".to_string()
            } else {
                format!("Launch {}", name)
            }
        }
        Action::KillProcess { process_name, .. } => format!("Kill {}", process_name),
        Action::Notification { title, .. } => {
            if title.is_empty() {
                "Notification toast".to_string()
            } else {
                format!("Notify \"{}\"", title)
            }
        }
        Action::SystemPower { power_type } => format!("Power: {:?}", power_type),
        Action::Delay { seconds } => format!("Wait {:.1} seconds", seconds),
        Action::HttpWebhook { url, method, .. } => {
            if url.is_empty() {
                "HTTP Webhook".to_string()
            } else {
                format!("{} {}", method, url)
            }
        }
        Action::Window { window_action, target_process, .. } => {
            let proc = target_process.as_deref().unwrap_or("Active window");
            format!("{:?} {}", window_action, proc)
        }
        Action::ProcessTuning { target_process, .. } => {
            let proc = target_process.as_deref().unwrap_or("Trigger process");
            format!("Tuning {}", proc)
        }
        Action::PowerPlan { plan } => format!("Power Plan: {:?}", plan),
        Action::PlaySound { file_path, play_beep, beep_frequency, beep_duration_ms } => {
            if *play_beep {
                if let Some(f) = beep_frequency {
                    format!("Beep {}Hz ({}ms)", f, beep_duration_ms.unwrap_or(200))
                } else {
                    "System Beep".to_string()
                }
            } else {
                let f = std::path::Path::new(file_path).file_name().and_then(|s| s.to_str()).unwrap_or(file_path);
                format!("Play sound: {}", f)
            }
        }
        Action::Firewall { process_name, block } => {
            if *block {
                format!("Block network for {}", process_name)
            } else {
                format!("Allow network for {}", process_name)
            }
        }
        Action::Display { width, height, refresh_rate, monitor_index } => {
            let res = match (width, height) {
                (Some(w), Some(h)) => format!("{}x{}", w, h),
                _ => "Auto Resolution".to_string(),
            };
            let mon = monitor_index.map(|m| format!("[Monitor {}] ", m)).unwrap_or_default();
            refresh_rate.map(|r| format!("Display: {}{} @ {}Hz", mon, res, r)).unwrap_or_else(|| format!("Display: {}{}", mon, res))
        }
        Action::AudioControl { target, action_type, volume_level, left_volume, right_volume, process_name } => {
            let target_str = match target {
                crate::models::AudioTarget::Playback => "Playback",
                crate::models::AudioTarget::Microphone => "Microphone",
                crate::models::AudioTarget::Process => "Process",
            };
            match action_type {
                crate::models::AudioVolumeAction::Mute => format!("Mute {}", target_str),
                crate::models::AudioVolumeAction::Unmute => format!("Unmute {}", target_str),
                crate::models::AudioVolumeAction::ToggleMute => format!("Toggle {} Mute", target_str),
                crate::models::AudioVolumeAction::SetVolume => {
                    if let (Some(l), Some(r)) = (left_volume, right_volume) {
                        format!("{} Vol L:{}% R:{}%", target_str, l, r)
                    } else if let Some(p) = process_name {
                        format!("{} ({}) Vol {}%", target_str, p, volume_level)
                    } else {
                        format!("{} Volume {}%", target_str, volume_level)
                    }
                }
                crate::models::AudioVolumeAction::PlayBeep => "Play System Beep".into(),
            }
        }
        Action::AudioFormat { target, sample_rate, bit_depth, channels, .. } => {
            let target_str = match target {
                crate::models::AudioTarget::Playback | crate::models::AudioTarget::Process => "Output",
                crate::models::AudioTarget::Microphone => "Input",
            };
            let ch_str = if *channels == 1 { "Mono" } else { "Stereo" };
            format!("{} Format: {}Hz, {}-bit, {}", target_str, sample_rate, bit_depth, ch_str)
        }
        Action::DefaultDevice { target, device_name } => {
            let pfx = match target {
                crate::models::AudioTarget::Playback => "Audio Output",
                crate::models::AudioTarget::Microphone => "Microphone",
                crate::models::AudioTarget::Process => "Process Audio",
            };
            format!("Default {}: {}", pfx, device_name)
        }
        Action::ExecuteScript { interpreter, script, .. } => {
            let name = match interpreter {
                crate::models::ScriptType::PowerShell => "PowerShell",
                crate::models::ScriptType::Batch => "CMD",
                crate::models::ScriptType::Lua => "Lua",
            };
            let first = script.lines().next().unwrap_or("").trim();
            if first.is_empty() {
                format!("Execute {}", name)
            } else {
                format!("{}: {}", name, first.chars().take(35).collect::<String>())
            }
        }
        Action::Macro { events, repeat, speed, infinite } => {
            let rep_str = if *infinite {
                " (∞)".to_string()
            } else if *repeat > 1 {
                format!(" (x{})", repeat)
            } else {
                String::new()
            };
            let spd_str = if (*speed - 1.0).abs() > 0.05 { format!(" @{:.1}x", speed) } else { String::new() };
            format!("Macro: {} events{}{}", events.len(), rep_str, spd_str)
        }
        Action::IfElse { conditions, then_actions, else_if_branches, else_actions } => {
            let glyph = conditions.first().map(|c| c.condition.icon_glyph()).unwrap_or("❓");
            let count = conditions.len();
            if else_if_branches.is_empty() {
                format!("If {} ({} cond) [then: {}, else: {}]", glyph, count, then_actions.len(), else_actions.len())
            } else {
                format!("If {} ({} cond) [then: {}, elif: {}, else: {}]", glyph, count, then_actions.len(), else_if_branches.len(), else_actions.len())
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_drop_slot(
    ui: &mut Ui,
    _loc: &crate::i18n::Strings,
    slot_idx: usize,
    active_dst: Option<usize>,
    active_src: Option<usize>,
    pipeline_id: &str,
    prefix: &str,
    t: &Tokens,
) {
    let is_targeted = if let (Some(dst), Some(src)) = (active_dst, active_src) {
        dst == slot_idx && slot_idx != src && slot_idx != src + 1
    } else {
        false
    };

    let anim_id = ui.id().with(("slot_anim", pipeline_id, slot_idx));
    let anim = ui.ctx().animate_bool_with_time_and_easing(anim_id, is_targeted, 0.18, easing::cubic_out);

    if anim > 0.001 {
        let slot_h = 22.0 * anim;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), slot_h), Sense::hover());
        let p = ui.painter();
        let mid_y = rect.center().y;

        // Background subtle accent tint
        p.rect_filled(
            rect,
            CornerRadius::same(2),
            t.accent_ink.gamma_multiply(0.08 * anim),
        );

        // Glowing laser line
        let line_stroke = Stroke::new(1.8 * anim, t.accent_ink.gamma_multiply(anim));
        p.hline(rect.left() + 24.0..=rect.right() - 8.0, mid_y, line_stroke);

        // Left indicator bead with halo
        let bead_pos = pos2(rect.left() + 12.0, mid_y);
        p.circle_filled(bead_pos, 3.5 * anim, t.accent_ink.gamma_multiply(anim));
        p.circle_stroke(
            bead_pos,
            5.5 * anim,
            Stroke::new(1.0, t.accent_ink.gamma_multiply(0.4 * anim)),
        );

        // Step number label at the right of the drop line
        let step_num = if let Some(src) = active_src {
            if slot_idx > src {
                slot_idx
            } else {
                slot_idx + 1
            }
        } else {
            slot_idx + 1
        };
        let label = format!("{} {:02}", prefix, step_num);
        let text_color = t.accent_ink.gamma_multiply(anim);
        let galley = widgets::caps(ui, &label, 9.0, theme::mono_bold(), 1.0, text_color);
        let text_pos = pos2(rect.right() - galley.size().x - 12.0, mid_y - galley.size().y * 0.5);
        p.galley_with_override_text_color(text_pos, galley, text_color);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_floating_drag_preview(
    ui: &Ui,
    loc: &crate::i18n::Strings,
    action: &Action,
    session: DndDragSession,
    pointer_pos: Option<Pos2>,
    prefix: &str,
    pipeline_id: &str,
    t: &Tokens,
) {
    let Some(pointer) = pointer_pos else { return };

    let card_w = session.card_width.clamp(280.0, 520.0);
    let card_h = 56.0;
    let preview_pos = pos2(
        pointer.x - card_w * 0.5,
        pointer.y - card_h * 0.5,
    );
    let preview_rect = Rect::from_min_size(preview_pos, vec2(card_w, card_h));

    let painter = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, Id::new(("dnd_float_preview", pipeline_id))));

    // Elevated drop shadow
    painter.add(theme::shadow(t).as_shape(preview_rect, CornerRadius::same(2)));

    // Card background & vibrant border
    painter.rect_filled(preview_rect, CornerRadius::same(2), t.bg);
    painter.rect_stroke(
        preview_rect,
        CornerRadius::same(2),
        Stroke::new(1.5, t.accent_ink),
        egui::StrokeKind::Inside,
    );

    // Top accent stripe
    painter.hline(preview_rect.x_range(), preview_rect.top() + 1.0, Stroke::new(2.0, t.accent_ink));

    // Grip dots on the left of floating card
    let grip_m = pos2(preview_rect.left() + 14.0, preview_rect.center().y);
    for x in [-2.8, 2.8] {
        for y in [-4.0, 0.0, 4.0] {
            painter.circle_filled(grip_m + vec2(x, y), 1.2, t.accent_ink);
        }
    }

    // Step indicator: "STEP 01"
    let step_txt = format!("{} {:02}", prefix, session.source_idx + 1);
    let step_galley = widgets::caps(ui, &step_txt, 11.0, theme::mono_bold(), 1.0, t.accent_ink);
    let step_w = step_galley.size().x;
    painter.galley_with_override_text_color(
        pos2(preview_rect.left() + 26.0, preview_rect.top() + 8.0),
        step_galley,
        t.accent_ink,
    );

    // Type badge
    let type_badge = action_type_badge(action, loc);
    let badge_galley = widgets::caps(ui, type_badge, 10.0, theme::mono(), 1.0, t.ink_strong);
    let badge_rect = Rect::from_min_size(
        pos2(preview_rect.left() + 26.0 + step_w + 8.0, preview_rect.top() + 7.0),
        vec2(badge_galley.size().x + 10.0, 18.0),
    );
    painter.rect_filled(badge_rect, CornerRadius::ZERO, t.sunken);
    painter.rect_stroke(badge_rect, CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);
    painter.galley_with_override_text_color(
        badge_rect.center() - badge_galley.size() * 0.5,
        badge_galley,
        t.ink_strong,
    );

    // Live destination indicator on far right: "➜ STEP 02"
    let dest_num = if session.target_idx > session.source_idx {
        session.target_idx
    } else {
        session.target_idx + 1
    };
    let dest_txt = format!("➜ {} {:02}", prefix, dest_num);
    let dest_galley = widgets::caps(ui, &dest_txt, 10.0, theme::mono_bold(), 1.0, t.accent_ink);
    painter.galley_with_override_text_color(
        pos2(preview_rect.right() - dest_galley.size().x - 12.0, preview_rect.top() + 9.0),
        dest_galley,
        t.accent_ink,
    );

    // Action summary line
    let summary = action_summary_preview(action);
    let summary_galley = ui.painter().layout_no_wrap(
        summary,
        egui::FontId::new(11.0, theme::mono()),
        t.muted,
    );
    painter.galley_with_override_text_color(
        pos2(preview_rect.left() + 26.0, preview_rect.top() + 32.0),
        summary_galley,
        t.muted,
    );
}

fn render_actions_pipeline(
    ui: &mut Ui,
    state: &mut GuiState,
    actions: &mut Vec<Action>,
    pipeline_id: &str,
    is_exit_pipeline: bool,
    exec_opt: Option<crate::ipc::protocol::ActiveRuleExecutionState>,
    t: &Tokens,
) -> Option<usize> {
    let dnd_id = ui.id().with(("action_pipeline_dnd", pipeline_id));

    let mut dnd_session: Option<DndDragSession> = ui.data_mut(|d| d.get_temp(dnd_id));

    // Cancel on Escape key
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        dnd_session = None;
        ui.data_mut(|d| d.remove_temp::<DndDragSession>(dnd_id));
    }

    let pointer_released = ui.input(|i| i.pointer.any_released());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));

    // When dragging: handle mouse wheel scrolling and edge auto-scrolling
    if dnd_session.is_some() {
        // 1. Mouse wheel scrolling while holding drag
        let wheel_delta_y = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel_delta_y != 0.0 {
            ui.scroll_with_delta(vec2(0.0, wheel_delta_y * 1.5));
            ui.ctx().request_repaint();
        }

        // 2. Edge auto-scrolling when pointer is near top or bottom of viewport
        if let Some(pointer) = pointer_pos {
            let viewport = ui.clip_rect();
            // Check if pointer is horizontally within or near the drawer column
            if pointer.x >= viewport.left() - 60.0 && pointer.x <= viewport.right() + 60.0 {
                let edge_zone = 90.0;
                let view_top = viewport.top();
                let view_bottom = viewport.bottom();

                // Near top edge: auto-scroll up
                if pointer.y < view_top + edge_zone {
                    let factor = ((view_top + edge_zone - pointer.y) / edge_zone).clamp(0.1, 3.0);
                    let scroll_speed = 8.0 + 26.0 * factor;
                    ui.scroll_with_delta(vec2(0.0, scroll_speed));
                    ui.ctx().request_repaint();
                }
                // Near bottom edge: auto-scroll down
                else if pointer.y > view_bottom - edge_zone {
                    let factor = ((pointer.y - (view_bottom - edge_zone)) / edge_zone).clamp(0.1, 3.0);
                    let scroll_speed = 8.0 + 26.0 * factor;
                    ui.scroll_with_delta(vec2(0.0, -scroll_speed));
                    ui.ctx().request_repaint();
                }
            }
        }
    }

    let active_src = dnd_session.as_ref().map(|s| s.source_idx);
    let active_dst = dnd_session.as_ref().map(|s| s.target_idx);

    let mut remove_idx = None;
    let mut clone_idx = None;
    let mut drag_started_idx = None;
    let mut switch_to_hunter_idx = None;
    let mut current_centers = Vec::with_capacity(actions.len());

    let loc = state.loc();
    let total = actions.len();
    let mut has_infinite_prior = false;

    let prefix = if is_exit_pipeline { loc.inspector_exit_prefix } else { loc.inspector_step_prefix };

    for (idx, action) in actions.iter_mut().enumerate() {
        // Render drop slot BEFORE card idx
        render_drop_slot(ui, loc, idx, active_dst, active_src, pipeline_id, prefix, t);

        // Render card idx
        let is_ghost = active_src == Some(idx);
        let is_executing = exec_opt.is_some_and(|e| e.is_exit == is_exit_pipeline && e.current_step == idx);
        let is_dimmed = has_infinite_prior;
        let resp = render_action_card(
            ui,
            state,
            action,
            idx,
            t,
            is_exit_pipeline,
            is_ghost,
            is_executing,
            is_dimmed,
        );

        current_centers.push(resp.card_rect.center().y);

        let is_infinite_macro = matches!(action, Action::Macro { infinite: true, .. });
        if is_infinite_macro {
            has_infinite_prior = true;
            render_infinite_macro_warning(ui, state, t);
        }

        if resp.remove_clicked {
            remove_idx = Some(idx);
        }
        if resp.clone_clicked {
            clone_idx = Some(idx);
        }
        if let Some(lang) = resp.expand_script {
            state.open_script_editor(idx, is_exit_pipeline, lang);
        }
        if resp.expand_macro {
            state.open_macro_editor(idx, is_exit_pipeline);
        }
        if resp.drag_started {
            drag_started_idx = Some(idx);
        }
        if resp.switch_to_hunter {
            switch_to_hunter_idx = Some(idx);
        }
        if let Some(target) = resp.open_catalog_nested {
            state.open_action_catalog_nested(is_exit_pipeline, target);
        }
        if let Some(target) = resp.pick_target {
            state.open_process_picker(target);
        }
    }

    // Render trailing drop slot (after last card)
    if total > 0 {
        render_drop_slot(ui, loc, total, active_dst, active_src, pipeline_id, prefix, t);
    }

    // Update target_idx using fresh current_centers from this exact frame
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

    // Commit reorder on mouse release
    if pointer_released {
        if let Some(session) = dnd_session {
            let src = session.source_idx;
            let dst = session.target_idx;
            if src < actions.len() && dst <= actions.len() {
                if dst < src {
                    state.push_undo_snapshot();
                    let item = actions.remove(src);
                    actions.insert(dst, item);
                } else if dst > src + 1 {
                    state.push_undo_snapshot();
                    let item = actions.remove(src);
                    actions.insert(dst - 1, item);
                }
            }
            dnd_session = None;
            ui.data_mut(|d| d.remove_temp::<DndDragSession>(dnd_id));
        }
    }

    // Handle step cloning
    if let Some(idx) = clone_idx {
        if idx < actions.len() {
            state.push_undo_snapshot();
            let cloned = actions[idx].clone();
            actions.insert(idx + 1, cloned);
        }
    }
    if let Some(idx) = remove_idx {
        if idx < actions.len() {
            state.push_undo_snapshot();
            actions.remove(idx);
        }
    }

    // Handle newly started drag
    if let Some(src_idx) = drag_started_idx {
        dnd_session = Some(DndDragSession {
            source_idx: src_idx,
            target_idx: src_idx,
            card_width: ui.available_width(),
        });
    }

    // Save or clear dnd_session
    if let Some(session) = dnd_session {
        ui.data_mut(|d| d.insert_temp(dnd_id, session));
        ui.ctx().request_repaint();

        // Render floating drag preview card!
        render_floating_drag_preview(
            ui,
            loc,
            &actions[session.source_idx],
            session,
            pointer_pos,
            prefix,
            pipeline_id,
            t,
        );
    }

    // Add action button at the bottom of the pipeline
    render_add_action_button(
        ui,
        state,
        loc,
        is_exit_pipeline,
        t,
    );

    switch_to_hunter_idx
}

struct ActionCardResponse {
    pub remove_clicked: bool,
    pub clone_clicked: bool,
    pub expand_script: Option<ScriptLanguage>,
    pub expand_macro: bool,
    pub drag_started: bool,
    pub switch_to_hunter: bool,
    pub open_catalog_nested: Option<crate::gui::state::IfElseCatalogTarget>,
    pub pick_target: Option<crate::gui::state::ProcessPickerTarget>,
    pub card_rect: Rect,
}

#[derive(Default)]
struct ActionFieldsResponse {
    pub expand_script: Option<ScriptLanguage>,
    pub expand_macro: bool,
    pub switch_to_hunter: bool,
    pub open_catalog_nested: Option<crate::gui::state::IfElseCatalogTarget>,
    pub pick_target: Option<crate::gui::state::ProcessPickerTarget>,
}

fn render_infinite_macro_warning(ui: &mut Ui, state: &GuiState, t: &Tokens) {
    let loc = state.loc();
    ui.add_space(2.0);
    egui::Frame::new()
        .fill(t.warn.gamma_multiply(0.12))
        .stroke(Stroke::new(1.0, t.warn.gamma_multiply(0.4)))
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(egui_phosphor::regular::WARNING)
                        .font(egui::FontId::new(13.0, theme::phosphor()))
                        .color(t.warn),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(loc.inspector_infinite_macro_warning)
                        .family(theme::mono())
                        .size(11.0)
                        .color(t.warn),
                );
            });
        });
    ui.add_space(2.0);
}

#[allow(clippy::too_many_arguments)]
fn render_action_card(
    ui: &mut Ui,
    state: &GuiState,
    action: &mut Action,
    idx: usize,
    t: &Tokens,
    is_exit_pipeline: bool,
    is_being_dragged: bool,
    is_executing: bool,
    is_dimmed: bool,
) -> ActionCardResponse {
    let loc = state.loc();
    let mut remove_clicked = false;
    let mut clone_clicked = false;
    let mut drag_started = false;
    let mut fields_resp = ActionFieldsResponse::default();

    let ghost_id = ui.id().with(("card_ghost", is_exit_pipeline, idx));
    let ghost_anim = ui.ctx().animate_bool_with_time_and_easing(ghost_id, is_being_dragged, 0.15, easing::cubic_out);

    let (fill, stroke) = if is_dimmed {
        (t.sunken.gamma_multiply(0.4), Stroke::new(1.0, t.muted.gamma_multiply(0.2)))
    } else if is_executing {
        (t.sunken, Stroke::new(1.5, t.accent_ink))
    } else if ghost_anim > 0.001 {
        (
            t.sunken,
            Stroke::new(1.0, t.accent_ink.gamma_multiply(0.45 * ghost_anim)),
        )
    } else {
        (t.sunken, t.hairline())
    };

    let card_response = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_max_width(ui.available_width());
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.set_max_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 8.0;

                // 1. CARD TOP STRIP: Step number + Type badge on left; Close button exclusively on far right
                ui.horizontal(|ui| {
                    ui.set_width(ui.available_width());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::glyph_button(ui, Glyph::Close, loc.inspector_tip_remove_step).clicked() {
                            remove_clicked = true;
                        }

                        if widgets::glyph_button(ui, Glyph::Clone, loc.inspector_tip_clone_step).clicked() {
                            clone_clicked = true;
                        }

                        if widgets::glyph_button(ui, Glyph::Play, loc.inspector_btn_test_step).clicked() {
                            let (ok, msg) = match action {
                                Action::Macro { events, speed, .. } => {
                                    if events.is_empty() {
                                        (false, loc.inspector_macro_test_empty.to_string())
                                    } else {
                                        crate::gui::macro_recorder::run_macro_test_safe(events.clone(), *speed);
                                        (true, loc.inspector_macro_testing_actions.replace("{}", &events.len().to_string()))
                                    }
                                }
                                _ => {
                                    let mut ctx = HashMap::new();
                                    ctx.insert("ProcessName".to_string(), "test.exe".to_string());
                                    ctx.insert("Timestamp".to_string(), chrono::Local::now().to_rfc3339());
                                    let res = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::TestStep {
                                        action: action.clone(),
                                        context: ctx,
                                    });
                                    let ok = matches!(res, Ok(crate::ipc::IpcResponse::TestStepResult(Ok(()))));
                                    let msg = match res {
                                        Ok(crate::ipc::IpcResponse::TestStepResult(Ok(()))) => "Step passed".to_string(),
                                        Ok(crate::ipc::IpcResponse::TestStepResult(Err(e))) => format!("Failed: {e}"),
                                        Ok(crate::ipc::IpcResponse::Error(e)) => format!("Error: {e}"),
                                        Err(e) => format!("IPC Error: {e}"),
                                        _ => "Unknown result".to_string(),
                                    };
                                    (ok, msg)
                                }
                            };
                            let key = if is_exit_pipeline { idx + 1000 } else { idx };
                            ui.data_mut(|d| d.insert_temp(Id::new(("test_step_res", key)), (ok, msg, Instant::now())));
                        }

                        let test_key = if is_exit_pipeline { idx + 1000 } else { idx };
                        if let Some((ok, msg, at)) = ui.data(|d| d.get_temp::<(bool, String, Instant)>(Id::new(("test_step_res", test_key)))) {
                            if at.elapsed().as_secs() < 8 {
                                let badge_color = if ok { t.good } else { t.bad };
                                widgets::badge(ui, if ok { "PASS" } else { "FAIL" }, badge_color).on_hover_text(msg);
                            }
                        }

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            // Drag grip handle on far left
                            let grip_resp = widgets::drag_handle(ui, loc.inspector_tip_drag_step);
                            if grip_resp.drag_started() {
                                drag_started = true;
                            }

                            let prefix = if is_exit_pipeline { loc.inspector_exit_prefix } else { loc.inspector_step_prefix };
                            widgets::hud(ui, &format!("{} {:02}", prefix, idx + 1), if is_dimmed { t.muted } else { t.accent_ink });

                            let type_badge = action_type_badge(action, loc);
                            if is_dimmed {
                                widgets::badge(ui, loc.inspector_badge_unreachable, t.warn.gamma_multiply(0.8))
                                    .on_hover_text(loc.inspector_badge_unreachable_tip);
                            } else if is_executing {
                                widgets::badge(ui, loc.inspector_badge_executing_now, t.accent_ink);
                            } else if is_being_dragged {
                                widgets::badge(ui, loc.inspector_badge_moving_step, t.accent_ink);
                            } else {
                                widgets::badge(ui, type_badge, t.muted);
                            }

                            // Middle blank space on top strip is also draggable
                            let empty_w = (ui.available_width() - 4.0).max(0.0);
                            if empty_w > 0.0 {
                                let (_, strip_resp) = ui.allocate_exact_size(vec2(empty_w, theme::CONTROL_H), Sense::click_and_drag());
                                if strip_resp.drag_started() {
                                    drag_started = true;
                                }
                                if strip_resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                }
                            }
                        });
                    });
                });

                widgets::hrule(ui);

                let default_picker = if is_exit_pipeline {
                    crate::gui::state::ProcessPickerTarget::ExitAction(idx)
                } else {
                    crate::gui::state::ProcessPickerTarget::Action(idx)
                };

                // 2. CARD BODY: Full-width stacked parameter fields
                fields_resp = ui.add_enabled_ui(!is_being_dragged && !is_dimmed, |ui| {
                    render_action_parameter_fields(
                        ui,
                        state,
                        action,
                        idx,
                        is_exit_pipeline,
                        default_picker,
                        t,
                        0,
                    )
                }).inner;
            });
        });

    ActionCardResponse {
        remove_clicked,
        clone_clicked,
        expand_script: fields_resp.expand_script,
        expand_macro: fields_resp.expand_macro,
        drag_started,
        switch_to_hunter: fields_resp.switch_to_hunter,
        open_catalog_nested: fields_resp.open_catalog_nested,
        pick_target: fields_resp.pick_target,
        card_rect: card_response.response.rect,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_action_parameter_fields(
    ui: &mut Ui,
    state: &GuiState,
    action: &mut Action,
    idx: usize,
    is_exit_pipeline: bool,
    default_picker_target: crate::gui::state::ProcessPickerTarget,
    t: &Tokens,
    depth: usize,
) -> ActionFieldsResponse {
    let loc = state.loc();
    let mut resp = ActionFieldsResponse::default();

    match action {
                    Action::StartProcess {
                        file_path,
                        arguments,
                        working_directory,
                        run_as_admin,
                        only_if_not_running,
                        run_hidden,
                        close_on_exit,
                    } => {
                        widgets::hud(ui, loc.inspector_field_executable_path, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn_w = widgets::button_width(ui, loc.settings_btn_browse, 0.0);
                                if widgets::button(ui, loc.settings_btn_browse, Kind::Secondary, btn_w).clicked() {
                                    if let Some(file) = rfd::FileDialog::new()
                                        .add_filter("Executables", &["exe", "bat", "cmd", "lnk", "ps1"])
                                        .pick_file()
                                    {
                                        *file_path = file.to_string_lossy().to_string();
                                    }
                                }
                                ui.add(
                                    egui::TextEdit::singleline(file_path)
                                        .hint_text(loc.inspector_field_executable_hint)
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                );
                            });
                        });

                        widgets::hud(ui, loc.inspector_field_arguments, t.muted);
                        ui.add(
                            egui::TextEdit::singleline(arguments)
                                .hint_text(loc.inspector_field_arguments_hint)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );

                        widgets::hud(ui, loc.inspector_field_working_directory, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn_w = widgets::button_width(ui, loc.inspector_btn_folder, 0.0);
                                if widgets::button(ui, loc.inspector_btn_folder, Kind::Secondary, btn_w).clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        *working_directory = Some(folder.to_string_lossy().to_string());
                                    }
                                }
                                let mut wd_str = working_directory.clone().unwrap_or_default();
                                if ui.add(
                                    egui::TextEdit::singleline(&mut wd_str)
                                        .hint_text(loc.inspector_field_working_directory_hint)
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                ).changed() {
                                    *working_directory = if wd_str.trim().is_empty() { None } else { Some(wd_str.clone()) };
                                }
                            });
                        });

                        ui.checkbox(run_hidden, loc.inspector_checkbox_run_hidden);
                        ui.checkbox(only_if_not_running, loc.inspector_checkbox_only_if_not_running);
                        ui.checkbox(run_as_admin, loc.inspector_checkbox_run_as_admin);
                        if !is_exit_pipeline {
                            ui.checkbox(close_on_exit, loc.inspector_checkbox_close_on_exit);
                        }
                    }

                    Action::KillProcess { process_name, kill_tree, force } => {
                        widgets::hud(ui, loc.inspector_field_terminate_process_name, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if render_process_picker(ui, loc) {
                                    resp.pick_target = Some(default_picker_target);
                                }
                                ui.add(
                                    egui::TextEdit::singleline(process_name)
                                        .hint_text(loc.inspector_field_terminate_hint)
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                );
                            });
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(kill_tree, loc.inspector_checkbox_kill_tree);
                            ui.checkbox(force, loc.inspector_checkbox_force_terminate);
                        });
                    }

                    Action::Notification { title, message } => {
                        widgets::hud(ui, loc.inspector_field_notif_title, t.muted);
                        ui.add(
                            egui::TextEdit::singleline(title)
                                .hint_text(loc.inspector_field_notif_title_hint)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );

                        widgets::hud(ui, loc.inspector_field_notif_message, t.muted);
                        ui.add(
                            egui::TextEdit::singleline(message)
                                .hint_text(loc.inspector_field_notif_message_hint)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );
                    }

                    Action::SystemPower { power_type } => {
                        widgets::hud(ui, loc.inspector_field_power_action, t.muted);
                        let power_options = [
                            (SystemPowerType::Lock, Cell::new(loc.inspector_cell_lock, loc.inspector_power_lock)),
                            (SystemPowerType::Sleep, Cell::new(loc.inspector_cell_sleep, loc.inspector_power_sleep)),
                            (SystemPowerType::TurnScreenOff, Cell::new(loc.inspector_cell_screen_off, loc.inspector_power_screen_off)),
                            (SystemPowerType::Restart, Cell::new(loc.inspector_cell_restart, loc.inspector_power_restart)),
                            (SystemPowerType::Shutdown, Cell::new(loc.inspector_cell_shutdown, loc.inspector_power_shutdown)),
                        ];
                        widgets::cells(ui, power_type, &power_options);
                    }

                    Action::Delay { seconds } => {
                        ui.horizontal_wrapped(|ui| {
                            widgets::hud(ui, loc.inspector_field_pause_delay, t.muted);
                            ui.add(egui::DragValue::new(seconds).speed(0.1).range(0.1..=120.0).suffix(" s"));
                        });
                    }

                    Action::HttpWebhook { url, method, payload, .. } => {
                        widgets::hud(ui, loc.inspector_field_webhook_url, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let combo_id = ui.id().with(idx).with("meth_combo");
                                egui::ComboBox::from_id_salt(combo_id)
                                    .selected_text(method.as_str())
                                    .width(65.0)
                                    .show_ui(ui, |ui| {
                                        for m in &["POST", "GET", "PUT", "DELETE"] {
                                            ui.selectable_value(method, m.to_string(), *m);
                                        }
                                    });
                                ui.add(
                                    egui::TextEdit::singleline(url)
                                        .hint_text(loc.inspector_field_webhook_url_hint)
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                );
                            });
                        });

                        widgets::hud(ui, loc.inspector_field_json_body, t.muted);
                        let mut p_str = payload.clone().unwrap_or_default();
                        if ui.add(
                            egui::TextEdit::multiline(&mut p_str)
                                .hint_text("{\"event\": \"triggered\", \"status\": \"ok\"}")
                                .desired_width(ui.available_width())
                                .desired_rows(2)
                                .font(egui::FontId::new(12.0, theme::mono()))
                                .margin(vec2(10.0, 8.0)),
                        ).changed() {
                            *payload = if p_str.trim().is_empty() { None } else { Some(p_str) };
                        }
                    }

                    Action::Window {
                        window_action,
                        target_process,
                        x,
                        y,
                        width,
                        height,
                        transparency,
                    } => {
                        let mut proc_str = target_process.clone().unwrap_or_default();
                        widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if render_process_picker(ui, loc) {
                                    resp.pick_target = Some(default_picker_target);
                                }
                                if ui.add(
                                    egui::TextEdit::singleline(&mut proc_str)
                                        .hint_text("Process executable or title (empty = active)")
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                ).changed() {
                                    *target_process = if proc_str.trim().is_empty() { None } else { Some(proc_str) };
                                }
                            });
                        });

                        widgets::hud(ui, "WINDOW OPERATION", t.muted);
                        let combo_id = ui.id().with(idx).with("win_act_combo");
                        let win_act_label = match window_action {
                            WindowActionType::Minimize => "Minimize",
                            WindowActionType::Maximize => "Maximize",
                            WindowActionType::Restore => "Restore",
                            WindowActionType::Hide => "Hide",
                            WindowActionType::Show => "Show",
                            WindowActionType::Close => "Close",
                            WindowActionType::AlwaysOnTop => "Always On Top",
                            WindowActionType::RemoveAlwaysOnTop => "Disable Always On Top",
                            WindowActionType::ToggleAlwaysOnTop => "Toggle Always On Top",
                            WindowActionType::SetTransparency => "Set Transparency",
                            WindowActionType::SnapToPosition => "Coordinates & Size",
                        };
                        egui::ComboBox::from_id_salt(combo_id)
                            .selected_text(win_act_label)
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(window_action, WindowActionType::Minimize, "Minimize");
                                ui.selectable_value(window_action, WindowActionType::Maximize, "Maximize");
                                ui.selectable_value(window_action, WindowActionType::Restore, "Restore");
                                ui.selectable_value(window_action, WindowActionType::Hide, "Hide");
                                ui.selectable_value(window_action, WindowActionType::Show, "Show");
                                ui.selectable_value(window_action, WindowActionType::Close, "Close");
                                ui.selectable_value(window_action, WindowActionType::AlwaysOnTop, "Always On Top");
                                ui.selectable_value(window_action, WindowActionType::RemoveAlwaysOnTop, "Disable Always On Top");
                                ui.selectable_value(window_action, WindowActionType::ToggleAlwaysOnTop, "Toggle Always On Top");
                                ui.selectable_value(window_action, WindowActionType::SetTransparency, "Set Transparency");
                                ui.selectable_value(window_action, WindowActionType::SnapToPosition, "Coordinates & Size");
                            });

                        if *window_action == WindowActionType::SetTransparency {
                            widgets::hud(ui, "OPACITY", t.muted);
                            let mut val = transparency.unwrap_or(255);
                            if ui.add(egui::Slider::new(&mut val, 10..=255)).changed() {
                                *transparency = Some(val);
                            }
                        } else if *window_action == WindowActionType::SnapToPosition {
                            ui.horizontal(|ui| {
                                widgets::hud(ui, "POS X:", t.muted);
                                let mut xv = x.unwrap_or(0);
                                if ui.add(egui::DragValue::new(&mut xv).range(0..=7680)).changed() { *x = Some(xv); }
                                ui.add_space(10.0);
                                widgets::hud(ui, "POS Y:", t.muted);
                                let mut yv = y.unwrap_or(0);
                                if ui.add(egui::DragValue::new(&mut yv).range(0..=4320)).changed() { *y = Some(yv); }
                            });
                            ui.horizontal(|ui| {
                                widgets::hud(ui, "WIDTH:", t.muted);
                                let mut wv = width.unwrap_or(800);
                                if ui.add(egui::DragValue::new(&mut wv).range(100..=7680)).changed() { *width = Some(wv); }
                                ui.add_space(10.0);
                                widgets::hud(ui, "HEIGHT:", t.muted);
                                let mut hv = height.unwrap_or(600);
                                if ui.add(egui::DragValue::new(&mut hv).range(100..=4320)).changed() { *height = Some(hv); }
                            });
                        }
                    }

                    Action::ProcessTuning {
                        target_process,
                        priority,
                        affinity_mask: _,
                        trim_memory,
                    } => {
                        let mut proc_str = target_process.clone().unwrap_or_default();
                        widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if render_process_picker(ui, loc) {
                                    resp.pick_target = Some(default_picker_target);
                                }
                                if ui.add(
                                    egui::TextEdit::singleline(&mut proc_str)
                                        .hint_text("Target process")
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                ).changed() {
                                    *target_process = if proc_str.trim().is_empty() { None } else { Some(proc_str) };
                                }
                            });
                        });

                        widgets::hud(ui, "PRIORITY", t.muted);
                        let combo_id = ui.id().with(idx).with("pri_combo");
                        let pri_text = priority.map(|p| format!("{:?}", p)).unwrap_or_else(|| "Unchanged".to_string());
                        egui::ComboBox::from_id_salt(combo_id)
                            .selected_text(pri_text)
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(priority, None, "Unchanged");
                                ui.selectable_value(priority, Some(ProcessPriorityLevel::Idle), "Idle");
                                ui.selectable_value(priority, Some(ProcessPriorityLevel::BelowNormal), "Below Normal");
                                ui.selectable_value(priority, Some(ProcessPriorityLevel::Normal), "Normal");
                                ui.selectable_value(priority, Some(ProcessPriorityLevel::AboveNormal), "Above Normal");
                                ui.selectable_value(priority, Some(ProcessPriorityLevel::High), "High");
                                ui.selectable_value(priority, Some(ProcessPriorityLevel::Realtime), "Realtime");
                            });

                        ui.add_space(4.0);
                        ui.checkbox(trim_memory, "Trim Working Set RAM");
                    }

                    Action::PowerPlan { plan } => {
                        widgets::hud(ui, "ENERGY SCHEME", t.muted);
                        let plan_options = [
                            (PowerPlanType::HighPerformance, Cell::new("HIGH PERFORMANCE", "Максимальная производительность")),
                            (PowerPlanType::Balanced, Cell::new("BALANCED", "Сбалансированная схема питания")),
                            (PowerPlanType::PowerSaver, Cell::new("POWER SAVER", "Энергосбережение")),
                        ];
                        widgets::cells(ui, plan, &plan_options);
                    }

                    Action::PlaySound { file_path, play_beep, beep_frequency, beep_duration_ms } => {
                        let mode_options = [
                            (false, Cell::new(loc.inspector_sound_mode_file, "")),
                            (true, Cell::new(loc.inspector_sound_mode_beep, "")),
                        ];
                        widgets::cells(ui, play_beep, &mode_options);
                        ui.add_space(4.0);

                        if *play_beep {
                            let mut is_custom = beep_frequency.is_some();
                            let tone_options = [
                                (false, Cell::new("WINDOWS ALERT", "Default Windows notification alert beep")),
                                (true, Cell::new("CUSTOM TONE", "Custom frequency & duration beep tone")),
                            ];
                            if widgets::cells(ui, &mut is_custom, &tone_options) {
                                if is_custom {
                                    *beep_frequency = Some(750);
                                    *beep_duration_ms = Some(200);
                                } else {
                                    *beep_frequency = None;
                                    *beep_duration_ms = None;
                                }
                            }

                            if is_custom {
                                ui.add_space(4.0);
                                let mut freq = beep_frequency.unwrap_or(750);
                                let mut dur = beep_duration_ms.unwrap_or(200);

                                widgets::hud(ui, loc.inspector_sound_beep_frequency, t.muted);
                                if ui.add(egui::Slider::new(&mut freq, 100..=5000).suffix(" Hz")).changed() {
                                    *beep_frequency = Some(freq);
                                }

                                ui.add_space(4.0);
                                widgets::hud(ui, loc.inspector_sound_beep_duration, t.muted);
                                if ui.add(egui::Slider::new(&mut dur, 50..=3000).suffix(" ms")).changed() {
                                    *beep_duration_ms = Some(dur);
                                }
                            }
                        } else {
                            widgets::hud(ui, "AUDIO FILE PATH (WAV, MP3, WMA)", t.muted);
                            ui.horizontal(|ui| {
                                ui.set_width(ui.available_width());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let btn_w = widgets::button_width(ui, loc.settings_btn_browse, 0.0);
                                    if widgets::button(ui, loc.settings_btn_browse, Kind::Secondary, btn_w).clicked() {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .add_filter("Audio File", &["wav", "mp3", "wma", "m4a", "ogg"])
                                            .pick_file()
                                        {
                                            *file_path = file.to_string_lossy().to_string();
                                        }
                                    }
                                    ui.add(
                                        egui::TextEdit::singleline(file_path)
                                            .hint_text("C:\\Windows\\Media\\notify.wav")
                                            .desired_width(ui.available_width())
                                            .min_size(vec2(0.0, theme::CONTROL_H))
                                            .vertical_align(egui::Align::Center)
                                            .margin(vec2(10.0, 8.0)),
                                    );
                                });
                            });
                        }
                    }

                    Action::Firewall { process_name, block } => {
                        widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if render_process_picker(ui, loc) {
                                    resp.pick_target = Some(default_picker_target);
                                }
                                ui.add(
                                    egui::TextEdit::singleline(process_name)
                                        .hint_text("Application .exe")
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                );
                            });
                        });

                        ui.add_space(4.0);
                        let fw_options = [
                            (true, Cell::new(loc.inspector_cell_block_traffic, "")),
                            (false, Cell::new(loc.inspector_cell_allow_traffic, "")),
                        ];
                        widgets::cells(ui, block, &fw_options);
                    }

                    Action::Display { width, height, refresh_rate, monitor_index } => {
                        widgets::hud(ui, "MONITOR", t.muted);
                        ui.horizontal(|ui| {
                            let mut has_mon = monitor_index.is_some();
                            if ui.checkbox(&mut has_mon, "Target Specific Monitor").changed() {
                                *monitor_index = if has_mon { Some(1) } else { None };
                            }
                            if let Some(m) = monitor_index {
                                ui.add(egui::DragValue::new(m).range(1..=16).prefix("Monitor #"));
                            }
                        });

                        ui.add_space(4.0);
                        widgets::hud(ui, "RESOLUTION", t.muted);
                        ui.horizontal(|ui| {
                            let mut w_val = width.unwrap_or(1920);
                            let mut h_val = height.unwrap_or(1080);
                            ui.add(egui::DragValue::new(&mut w_val).range(640..=7680).prefix("W: ").suffix(" px"));
                            ui.add(egui::DragValue::new(&mut h_val).range(480..=4320).prefix("H: ").suffix(" px"));
                            *width = Some(w_val);
                            *height = Some(h_val);
                        });

                        ui.add_space(4.0);
                        widgets::hud(ui, "REFRESH RATE", t.muted);
                        ui.horizontal(|ui| {
                            let mut has_hz = refresh_rate.is_some();
                            if ui.checkbox(&mut has_hz, "Set Refresh Rate").changed() {
                                *refresh_rate = if has_hz { Some(144) } else { None };
                            }
                            if let Some(hz) = refresh_rate {
                                ui.add(egui::DragValue::new(hz).range(30..=360).suffix(" Hz"));
                            }
                        });
                    }

                    Action::AudioControl {
                        target,
                        process_name,
                        action_type,
                        volume_level,
                        left_volume,
                        right_volume,
                    } => {
                        widgets::hud(ui, loc.inspector_audio_target_label, t.muted);
                        let target_options = [
                            (crate::models::AudioTarget::Playback, Cell::new(
                                loc.inspector_audio_target_playback,
                                loc.inspector_audio_target_playback_tip,
                            )),
                            (crate::models::AudioTarget::Microphone, Cell::new(
                                loc.inspector_audio_target_mic,
                                loc.inspector_audio_target_mic_tip,
                            )),
                            (crate::models::AudioTarget::Process, Cell::new(
                                loc.inspector_audio_target_process,
                                loc.inspector_audio_target_process_tip,
                            )),
                        ];
                        widgets::cells(ui, target, &target_options);

                        if *target == crate::models::AudioTarget::Process {
                            ui.add_space(4.0);
                            widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                            ui.horizontal(|ui| {
                                ui.set_width(ui.available_width());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if render_process_picker(ui, loc) {
                                        resp.pick_target = Some(default_picker_target);
                                    }
                                    let mut p_buf = process_name.clone().unwrap_or_default();
                                    if ui.add(
                                        egui::TextEdit::singleline(&mut p_buf)
                                            .hint_text("Application .exe")
                                            .desired_width(ui.available_width())
                                            .min_size(vec2(0.0, theme::CONTROL_H))
                                            .vertical_align(egui::Align::Center)
                                            .margin(vec2(10.0, 8.0)),
                                    ).changed() {
                                        let trimmed = p_buf.trim().to_string();
                                        *process_name = if trimmed.is_empty() { None } else { Some(trimmed) };
                                    }
                                });
                            });
                        }

                        ui.add_space(4.0);
                        widgets::hud(ui, loc.inspector_field_audio_action_type, t.muted);
                        let audio_options = [
                            (crate::models::AudioVolumeAction::Mute, Cell::new(
                                loc.inspector_cell_mute,
                                loc.inspector_audio_mute,
                            )),
                            (crate::models::AudioVolumeAction::Unmute, Cell::new(
                                loc.inspector_cell_unmute,
                                loc.inspector_audio_unmute,
                            )),
                            (crate::models::AudioVolumeAction::ToggleMute, Cell::new(
                                loc.inspector_cell_toggle,
                                loc.inspector_audio_toggle_mute,
                            )),
                            (crate::models::AudioVolumeAction::SetVolume, Cell::new(
                                loc.inspector_cell_volume,
                                loc.inspector_audio_set_volume,
                            )),
                        ];
                        widgets::cells(ui, action_type, &audio_options);

                        if *action_type == crate::models::AudioVolumeAction::SetVolume {
                            ui.add_space(4.0);
                            if *target == crate::models::AudioTarget::Playback {
                                let mut separate = left_volume.is_some() || right_volume.is_some();
                                if ui.checkbox(&mut separate, loc.inspector_audio_separate_channels).changed() {
                                    if separate {
                                        *left_volume = Some(*volume_level);
                                        *right_volume = Some(*volume_level);
                                    } else {
                                        *left_volume = None;
                                        *right_volume = None;
                                    }
                                }

                                if separate {
                                    let mut l = left_volume.unwrap_or(*volume_level);
                                    let mut r = right_volume.unwrap_or(*volume_level);
                                    ui.add_space(2.0);
                                    widgets::hud(ui, loc.inspector_audio_left_channel, t.muted);
                                    if ui.add(egui::Slider::new(&mut l, 0..=100).suffix("%")).changed() {
                                        *left_volume = Some(l);
                                    }
                                    ui.add_space(2.0);
                                    widgets::hud(ui, loc.inspector_audio_right_channel, t.muted);
                                    if ui.add(egui::Slider::new(&mut r, 0..=100).suffix("%")).changed() {
                                        *right_volume = Some(r);
                                    }
                                } else {
                                    widgets::hud(ui, loc.inspector_field_volume_level, t.muted);
                                    ui.horizontal(|ui| {
                                        ui.add(egui::Slider::new(volume_level, 0..=100).suffix("%"));
                                    });
                                }
                            } else {
                                widgets::hud(ui, loc.inspector_field_volume_level, t.muted);
                                ui.horizontal(|ui| {
                                    ui.add(egui::Slider::new(volume_level, 0..=100).suffix("%"));
                                });
                            }
                        }
                    }

                    Action::AudioFormat {
                        target,
                        device_name: _,
                        sample_rate,
                        bit_depth,
                        channels,
                    } => {
                        widgets::hud(ui, loc.inspector_audio_target_label, t.muted);
                        let target_options = [
                            (crate::models::AudioTarget::Playback, Cell::new(loc.inspector_audio_target_playback, loc.inspector_audio_target_playback_tip)),
                            (crate::models::AudioTarget::Microphone, Cell::new(loc.inspector_audio_target_mic, loc.inspector_audio_target_mic_tip)),
                        ];
                        widgets::cells(ui, target, &target_options);

                        ui.add_space(4.0);
                        widgets::hud(ui, loc.inspector_audio_format_sample_rate, t.muted);
                        let rate_options = [
                            (44100, Cell::new("44.1 kHz", "CD Quality")),
                            (48000, Cell::new("48.0 kHz", "Studio Standard")),
                            (96000, Cell::new("96.0 kHz", "High Resolution")),
                            (192000, Cell::new("192.0 kHz", "Ultra High Resolution")),
                        ];
                        widgets::cells(ui, sample_rate, &rate_options);

                        ui.add_space(4.0);
                        widgets::hud(ui, loc.inspector_audio_format_bit_depth, t.muted);
                        let depth_options = [
                            (16, Cell::new("16-bit", "Standard Depth")),
                            (24, Cell::new("24-bit", "Studio Quality")),
                            (32, Cell::new("32-bit", "Float / Extended")),
                        ];
                        widgets::cells(ui, bit_depth, &depth_options);

                        ui.add_space(4.0);
                        widgets::hud(ui, loc.inspector_audio_format_channels, t.muted);
                        let ch_options = [
                            (2, Cell::new("Stereo (2.0)", "Two channels (Left & Right)")),
                            (1, Cell::new("Mono (1.0)", "Single channel microphone / headset")),
                        ];
                        widgets::cells(ui, channels, &ch_options);
                    }

                    Action::DefaultDevice { target, device_name } => {
                        widgets::hud(ui, loc.inspector_audio_target_label, t.muted);
                        let dev_target_options = [
                            (crate::models::AudioTarget::Playback, Cell::new(
                                loc.inspector_audio_dev_playback,
                                loc.inspector_audio_dev_playback_tip,
                            )),
                            (crate::models::AudioTarget::Microphone, Cell::new(
                                loc.inspector_audio_dev_mic,
                                loc.inspector_audio_dev_mic_tip,
                            )),
                        ];
                        widgets::cells(ui, target, &dev_target_options);

                        ui.add_space(4.0);
                        let dev_hint = match target {
                            crate::models::AudioTarget::Playback => "Speakers (Realtek Audio), Headphones...",
                            crate::models::AudioTarget::Microphone => "Microphone (Realtek Audio), Headset...",
                            crate::models::AudioTarget::Process => "Process name...",
                        };
                        widgets::hud(ui, "DEVICE NAME", t.muted);
                        ui.add(
                            egui::TextEdit::singleline(device_name)
                                .hint_text(dev_hint)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );
                    }

                    Action::ExecuteScript { interpreter, script, run_in_window } => {
                        ui.horizontal(|ui| {
                            widgets::hud(ui, loc.inspector_script_interpreter, t.muted);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Expand, loc.inspector_script_expand_tip).clicked() {
                                    resp.expand_script = Some(match interpreter {
                                        crate::models::ScriptType::PowerShell => ScriptLanguage::PowerShell,
                                        crate::models::ScriptType::Batch => ScriptLanguage::Batch,
                                        crate::models::ScriptType::Lua => ScriptLanguage::Lua,
                                    });
                                }
                            });
                        });

                        let script_options = [
                            (crate::models::ScriptType::PowerShell, Cell::new("POWERSHELL", "Windows PowerShell (.ps1)")),
                            (crate::models::ScriptType::Batch, Cell::new("CMD / BATCH", "Command Prompt (.cmd / .bat)")),
                            (crate::models::ScriptType::Lua, Cell::new("LUA 5.4", "Embedded Lua 5.4 Script")),
                        ];
                        widgets::cells(ui, interpreter, &script_options);

                        ui.add_space(4.0);
                        let lang = match interpreter {
                            crate::models::ScriptType::PowerShell => ScriptLanguage::PowerShell,
                            crate::models::ScriptType::Batch => ScriptLanguage::Batch,
                            crate::models::ScriptType::Lua => ScriptLanguage::Lua,
                        };
                        let mut layouter = create_script_layouter(lang, *t, 12.0);
                        ui.add(
                            egui::TextEdit::multiline(script)
                                .hint_text(loc.inspector_script_placeholder)
                                .font(egui::FontId::new(12.0, theme::mono()))
                                .desired_width(ui.available_width())
                                .desired_rows(4)
                                .margin(vec2(8.0, 6.0))
                                .layouter(&mut layouter),
                        );

                        if *interpreter != crate::models::ScriptType::Lua {
                            ui.checkbox(run_in_window, loc.inspector_checkbox_visible_console);
                        }
                    }

                    Action::Macro { .. } => {
                        let banner_h = 44.0;
                        let (rect, macro_btn_resp) = ui.allocate_exact_size(vec2(ui.available_width(), banner_h), Sense::click());
                        let hovered = macro_btn_resp.hovered();

                        if macro_btn_resp.clicked() {
                            resp.expand_macro = true;
                        }

                        let p = ui.painter();
                        let bg = if hovered { t.sunken } else { t.bg };
                        let border_color = if hovered { t.accent_ink } else { t.hairline().color };
                        p.rect_filled(rect, CornerRadius::same(2), bg);
                        p.rect_stroke(rect, CornerRadius::same(2), Stroke::new(1.0, border_color), egui::StrokeKind::Inside);

                        // Icon chip on left
                        let icon_box = Rect::from_min_size(rect.min + vec2(8.0, 7.0), vec2(30.0, 30.0));
                        let icon_bg = if hovered { t.accent_ink.gamma_multiply(0.22) } else { t.accent_ink.gamma_multiply(0.12) };
                        p.rect_filled(icon_box, CornerRadius::same(2), icon_bg);
                        p.text(
                            icon_box.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::GAME_CONTROLLER,
                            egui::FontId::new(18.0, theme::phosphor()),
                            t.accent_ink,
                        );

                        // Middle title (clean, centered vertically)
                        let text_left = icon_box.right() + 10.0;
                        let title_pos = pos2(text_left, rect.center().y);
                        p.text(
                            title_pos,
                            egui::Align2::LEFT_CENTER,
                            loc.inspector_macro_studio_title,
                            egui::FontId::new(13.0, theme::semibold()),
                            if hovered { t.accent_ink } else { t.ink_strong },
                        );

                        // Action arrow pill on right
                        let arrow_pos = pos2(rect.right() - 14.0, rect.center().y);
                        p.text(
                            arrow_pos,
                            egui::Align2::RIGHT_CENTER,
                            loc.inspector_macro_studio_open,
                            egui::FontId::new(11.0, theme::mono_bold()),
                            if hovered { t.accent_ink } else { t.faint },
                        );
                    }

        Action::IfElse { conditions, then_actions, else_if_branches, else_actions } => {
            if depth > 0 {
                ui.label(egui::RichText::new(loc.inspector_ifelse_nested_not_supported).color(t.muted));
            } else {
                if conditions.is_empty() {
                    conditions.push(RuleConditionItem::new(RuleCondition::default_process()));
                }

                // ── 1. PRIMARY IF BLOCK ──
                ui.horizontal(|ui| {
                    widgets::hud(ui, loc.inspector_ifelse_then_badge, t.good);
                });

                let mut remove_if_cond_idx = None;
                let total_if_conds = conditions.len();
                for (c_idx, c_item) in conditions.iter_mut().enumerate() {
                    if c_idx > 0 {
                        render_condition_operator_connector(ui, &mut c_item.operator, loc, t);
                    }
                    if render_condition_card(
                        ui,
                        c_idx,
                        c_item,
                        loc,
                        t,
                        true,
                        total_if_conds > 1,
                        || {
                            resp.pick_target = Some(crate::gui::state::ProcessPickerTarget::ActionCondition {
                                action_idx: idx,
                                cond_idx: c_idx,
                                is_exit: is_exit_pipeline,
                            });
                        },
                    ) && total_if_conds > 1 {
                        remove_if_cond_idx = Some(c_idx);
                    }
                }
                if let Some(c_idx) = remove_if_cond_idx {
                    if c_idx < conditions.len() {
                        conditions.remove(c_idx);
                    }
                }

                if render_add_condition_button(ui, loc, t) {
                    conditions.push(RuleConditionItem::new(RuleCondition::default_process()));
                }

                ui.add_space(6.0);

                // THEN branch actions
                ui.horizontal(|ui| {
                    widgets::hud(ui, loc.inspector_ifelse_then_execute, t.good);
                });

                render_subactions_branch_pipeline(
                    ui,
                    state,
                    then_actions,
                    idx,
                    "then",
                    "THEN",
                    is_exit_pipeline,
                    BranchPickerKind::Then,
                    loc.inspector_ifelse_add_then,
                    t,
                    depth + 1,
                    &mut resp,
                    Some(crate::gui::state::IfElseCatalogTarget::Then(idx)),
                );

                // ── 2. ELSE IF BRANCHES ──
                let mut remove_branch_idx = None;
                for (b_idx, branch) in else_if_branches.iter_mut().enumerate() {
                    ui.add_space(8.0);
                    widgets::hrule(ui);

                    ui.horizontal(|ui| {
                        let b_lbl = format!("{} {:02}", loc.inspector_ifelse_elseif_badge, b_idx + 1);
                        widgets::hud(ui, &b_lbl, t.warn);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if widgets::glyph_button(ui, Glyph::Close, loc.inspector_ifelse_remove_branch_tip).clicked() {
                                remove_branch_idx = Some(b_idx);
                            }
                        });
                    });

                    if branch.conditions.is_empty() {
                        branch.conditions.push(RuleConditionItem::new(RuleCondition::default_process()));
                    }

                    let mut remove_elif_cond_idx = None;
                    let total_elif_conds = branch.conditions.len();
                    for (c_idx, c_item) in branch.conditions.iter_mut().enumerate() {
                        if c_idx > 0 {
                            render_condition_operator_connector(ui, &mut c_item.operator, loc, t);
                        }
                        if render_condition_card(
                            ui,
                            c_idx,
                            c_item,
                            loc,
                            t,
                            true,
                            total_elif_conds > 1,
                            || {
                                resp.pick_target = Some(crate::gui::state::ProcessPickerTarget::ActionElseIfCondition {
                                    action_idx: idx,
                                    branch_idx: b_idx,
                                    cond_idx: c_idx,
                                    is_exit: is_exit_pipeline,
                                });
                            },
                        ) && total_elif_conds > 1 {
                            remove_elif_cond_idx = Some(c_idx);
                        }
                    }
                    if let Some(c_idx) = remove_elif_cond_idx {
                        if c_idx < branch.conditions.len() {
                            branch.conditions.remove(c_idx);
                        }
                    }

                    if render_add_condition_button(ui, loc, t) {
                        branch.conditions.push(RuleConditionItem::new(RuleCondition::default_process()));
                    }

                    ui.add_space(6.0);

                    // Branch actions header
                    let branch_lbl = format!("ELIF {:02}", b_idx + 1);
                    ui.horizontal(|ui| {
                        widgets::hud(ui, loc.inspector_ifelse_then_execute, t.warn);
                    });

                    render_subactions_branch_pipeline(
                        ui,
                        state,
                        &mut branch.actions,
                        idx,
                        &format!("elif_{}", b_idx),
                        &branch_lbl,
                        is_exit_pipeline,
                        BranchPickerKind::ElseIf(b_idx),
                        loc.inspector_ifelse_add_then,
                        t,
                        depth + 1,
                        &mut resp,
                        Some(crate::gui::state::IfElseCatalogTarget::ElseIf {
                            parent_idx: idx,
                            branch_idx: b_idx,
                        }),
                    );
                }
                if let Some(b_idx) = remove_branch_idx {
                    if b_idx < else_if_branches.len() {
                        else_if_branches.remove(b_idx);
                    }
                }

                // Full-width button to add an Else If branch with separator above and below
                ui.add_space(8.0);
                widgets::hrule(ui);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    widgets::hud(ui, loc.inspector_ifelse_elseif_badge, t.warn);
                });
                ui.add_space(4.0);
                if render_full_width_action_button(ui, loc.inspector_ifelse_add_elseif, loc.inspector_ifelse_add_elseif, t) {
                    else_if_branches.push(crate::models::ElseIfBranch::new());
                }
                ui.add_space(6.0);
                widgets::hrule(ui);
                ui.add_space(8.0);

                // ── 3. ELSE BRANCH ──
                ui.horizontal(|ui| {
                    widgets::hud(ui, loc.inspector_ifelse_else_badge, t.bad);
                });

                render_subactions_branch_pipeline(
                    ui,
                    state,
                    else_actions,
                    idx,
                    "else",
                    "ELSE",
                    is_exit_pipeline,
                    BranchPickerKind::Else,
                    loc.inspector_ifelse_add_else,
                    t,
                    depth + 1,
                    &mut resp,
                    Some(crate::gui::state::IfElseCatalogTarget::Else(idx)),
                );
            }
        }
    }

    resp
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConditionKind {
    Process,
    Window,
    Time,
    Event,
    System,
}

impl ConditionKind {
    fn from_condition(c: &RuleCondition) -> Self {
        match c {
            RuleCondition::ProcessRunning { .. } => ConditionKind::Process,
            RuleCondition::WindowFocused { .. } => ConditionKind::Window,
            RuleCondition::TimeRange { .. } => ConditionKind::Time,
            RuleCondition::Event { .. } => ConditionKind::Event,
            RuleCondition::ResourceThreshold { .. } | RuleCondition::FileWatcher { .. } => ConditionKind::System,
        }
    }

    fn label(&self, loc: &crate::i18n::Strings) -> &'static str {
        match self {
            ConditionKind::Process => loc.inspector_cond_type_process,
            ConditionKind::Window => loc.inspector_cond_type_window,
            ConditionKind::Time => loc.inspector_cond_type_time,
            ConditionKind::Event => loc.inspector_cond_type_event,
            ConditionKind::System => loc.inspector_cond_type_system,
        }
    }

    fn default_condition(&self) -> RuleCondition {
        match self {
            ConditionKind::Process => RuleCondition::default_process(),
            ConditionKind::Window => RuleCondition::default_window(),
            ConditionKind::Time => RuleCondition::default_time(),
            ConditionKind::Event => RuleCondition::default_event(),
            ConditionKind::System => RuleCondition::default_resource(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemConditionSubMode {
    Resource,
    FileWatcher,
}

#[allow(clippy::too_many_arguments)]
fn render_condition_card(
    ui: &mut Ui,
    idx: usize,
    item: &mut RuleConditionItem,
    loc: &crate::i18n::Strings,
    t: &Tokens,
    is_nested: bool,
    can_remove: bool,
    on_pick_process: impl FnMut(),
) -> bool {
    let mut remove_clicked = false;
    let mut kind = ConditionKind::from_condition(&item.condition);

    let (fill, stroke) = if is_nested {
        (t.bg, Stroke::new(1.0, t.line_strong.gamma_multiply(0.45)))
    } else {
        (t.sunken, t.hairline())
    };

    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                // 1. Top Strip: Step index + Badge on left; Close button on right (hidden if cannot be removed)
                ui.horizontal(|ui| {
                    let pfx = format!("{} {:02}", loc.inspector_conditions_badge, idx + 1);
                    widgets::hud(ui, &pfx, t.accent_ink);
                    widgets::badge(ui, kind.label(loc), t.muted);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if can_remove && widgets::glyph_button(ui, Glyph::Close, loc.inspector_cond_remove_tip).clicked() {
                            remove_clicked = true;
                        }
                    });
                });

                widgets::hrule(ui);

                // 2. Condition Kind Selector using widgets::cells
                widgets::hud(ui, loc.inspector_cond_type_label, t.muted);
                let kind_options = [
                    (ConditionKind::Process, Cell::new(loc.inspector_cond_tab_process, "")),
                    (ConditionKind::Window, Cell::new(loc.inspector_cond_tab_window, "")),
                    (ConditionKind::Time, Cell::new(loc.inspector_cond_tab_time, "")),
                    (ConditionKind::Event, Cell::new(loc.inspector_cond_tab_event, "")),
                    (ConditionKind::System, Cell::new(loc.inspector_cond_tab_system, "")),
                ];
                let prev_kind = kind;
                if widgets::cells(ui, &mut kind, &kind_options) && kind != prev_kind {
                    item.condition = kind.default_condition();
                }

                ui.add_space(4.0);

                // 3. Condition Parameter Fields
                render_single_condition_editor(ui, loc, &mut item.condition, on_pick_process, t);
            });
        });

    remove_clicked
}

fn render_condition_operator_connector(
    ui: &mut Ui,
    op: &mut ConditionOperator,
    loc: &crate::i18n::Strings,
    t: &Tokens,
) {
    ui.add_space(2.0);
    let avail_w = ui.available_width();
    let op_options = [
        (ConditionOperator::And, Cell::new(loc.inspector_cond_op_and, loc.inspector_cond_op_and_tip)),
        (ConditionOperator::Or, Cell::new(loc.inspector_cond_op_or, loc.inspector_cond_op_or_tip)),
    ];

    let mut cells_w = 0.0;
    for (_, c) in &op_options {
        let galley = widgets::caps(ui, c.label, theme::HUD_SIZE, theme::mono_bold(), theme::BUTTON_SPACING, t.ink);
        cells_w += (galley.size().x + 24.0).max(theme::CONTROL_H);
    }

    let side_w = ((avail_w - cells_w - 12.0) * 0.5).max(12.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        let (rect_left, _) = ui.allocate_exact_size(vec2(side_w, theme::CONTROL_H), Sense::hover());
        ui.painter().line_segment(
            [pos2(rect_left.left(), rect_left.center().y), pos2(rect_left.right(), rect_left.center().y)],
            Stroke::new(1.0, t.muted.gamma_multiply(0.25)),
        );

        widgets::cells(ui, op, &op_options);

        let rem_w = (ui.available_width() - 2.0).max(0.0);
        if rem_w > 0.0 {
            let (rect_right, _) = ui.allocate_exact_size(vec2(rem_w, theme::CONTROL_H), Sense::hover());
            ui.painter().line_segment(
                [pos2(rect_right.left(), rect_right.center().y), pos2(rect_right.right(), rect_right.center().y)],
                Stroke::new(1.0, t.muted.gamma_multiply(0.25)),
            );
        }
    });
    ui.add_space(2.0);
}

fn render_full_width_action_button(
    ui: &mut Ui,
    label: &str,
    tip: &str,
    t: &Tokens,
) -> bool {
    ui.add_space(2.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let is_hov = resp.hovered();
    let p = ui.painter();

    let border = if is_hov {
        Stroke::new(1.0, t.accent_ink)
    } else {
        Stroke::new(1.0, t.muted.gamma_multiply(0.35))
    };
    p.rect_stroke(rect, CornerRadius::same(2), border, egui::StrokeKind::Inside);

    let text_color = if is_hov { t.accent_ink } else { t.faint };
    let plus_galley = p.layout_no_wrap(
        egui_phosphor::regular::PLUS.to_string(),
        egui::FontId::new(12.0, theme::phosphor()),
        text_color,
    );
    let label_galley = widgets::caps(ui, label, 10.0, theme::mono(), 0.8, text_color);

    let spacing = 8.0;
    let plus_w = plus_galley.size().x;
    let plus_h = plus_galley.size().y;
    let total_w = plus_w + spacing + label_galley.size().x;
    let start_x = rect.center().x - total_w * 0.5;

    let plus_pos = pos2(start_x, rect.center().y - plus_h * 0.5);
    p.galley(plus_pos, plus_galley, text_color);

    let label_pos = pos2(start_x + plus_w + spacing, rect.center().y - label_galley.size().y * 0.5);
    p.galley_with_override_text_color(label_pos, label_galley, text_color);

    let resp = resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tip);

    resp.clicked()
}

fn render_add_condition_button(
    ui: &mut Ui,
    loc: &crate::i18n::Strings,
    t: &Tokens,
) -> bool {
    render_full_width_action_button(ui, loc.inspector_conditions_btn_add, loc.inspector_conditions_btn_add, t)
}

fn render_single_condition_editor(
    ui: &mut Ui,
    loc: &crate::i18n::Strings,
    cond: &mut RuleCondition,
    mut on_pick_process: impl FnMut(),
    t: &Tokens,
) {
    match cond {
        RuleCondition::ProcessRunning { process_name, match_mode, must_be_running } => {
            widgets::hud(ui, loc.inspector_field_target_process, t.muted);
            ui.horizontal(|ui| {
                ui.set_width(ui.available_width());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if render_process_picker(ui, loc) {
                        on_pick_process();
                    }
                    ui.add(
                        egui::TextEdit::singleline(process_name)
                            .hint_text("game.exe")
                            .desired_width(ui.available_width())
                            .min_size(vec2(0.0, theme::CONTROL_H))
                            .vertical_align(egui::Align::Center)
                            .margin(vec2(10.0, 8.0)),
                    );
                });
            });

            widgets::hud(ui, loc.inspector_field_match_mode, t.muted);
            let match_options = [
                (ProcessMatchMode::Exact, Cell::new(loc.inspector_match_tab_exact, "")),
                (ProcessMatchMode::Wildcard, Cell::new(loc.inspector_match_tab_wildcard, "")),
                (ProcessMatchMode::Regex, Cell::new(loc.inspector_match_tab_regex, "")),
            ];
            widgets::cells(ui, match_mode, &match_options);

            ui.add_space(2.0);
            widgets::hud(ui, loc.inspector_cond_state_label, t.muted);
            let state_options = [
                (true, Cell::new(loc.inspector_cond_state_running, "")),
                (false, Cell::new(loc.inspector_cond_state_not_running, "")),
            ];
            widgets::cells(ui, must_be_running, &state_options);
        }
        RuleCondition::WindowFocused { process_name, match_mode, must_be_focused } => {
            widgets::hud(ui, loc.inspector_field_target_process, t.muted);
            ui.horizontal(|ui| {
                ui.set_width(ui.available_width());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if render_process_picker(ui, loc) {
                        on_pick_process();
                    }
                    ui.add(
                        egui::TextEdit::singleline(process_name)
                            .hint_text("game.exe")
                            .desired_width(ui.available_width())
                            .min_size(vec2(0.0, theme::CONTROL_H))
                            .vertical_align(egui::Align::Center)
                            .margin(vec2(10.0, 8.0)),
                    );
                });
            });

            widgets::hud(ui, loc.inspector_field_match_mode, t.muted);
            let match_options = [
                (ProcessMatchMode::Exact, Cell::new(loc.inspector_match_tab_exact, "")),
                (ProcessMatchMode::Wildcard, Cell::new(loc.inspector_match_tab_wildcard, "")),
                (ProcessMatchMode::Regex, Cell::new(loc.inspector_match_tab_regex, "")),
            ];
            widgets::cells(ui, match_mode, &match_options);

            ui.add_space(2.0);
            widgets::hud(ui, loc.inspector_cond_focus_label, t.muted);
            let state_options = [
                (true, Cell::new(loc.inspector_cond_state_focused, "")),
                (false, Cell::new(loc.inspector_cond_state_not_focused, "")),
            ];
            widgets::cells(ui, must_be_focused, &state_options);
        }
        RuleCondition::TimeRange { start_time, end_time, days_of_week } => {
            widgets::hud(ui, loc.inspector_cond_time_range_label, t.muted);
            ui.horizontal(|ui| {
                widgets::hud(ui, loc.inspector_cond_time_from, t.muted);
                ui.add(
                    egui::TextEdit::singleline(start_time)
                        .hint_text("09:00")
                        .desired_width(65.0)
                        .min_size(vec2(0.0, theme::CONTROL_H))
                        .vertical_align(egui::Align::Center)
                        .margin(vec2(8.0, 6.0)),
                );
                widgets::hud(ui, loc.inspector_cond_time_to, t.muted);
                ui.add(
                    egui::TextEdit::singleline(end_time)
                        .hint_text("18:00")
                        .desired_width(65.0)
                        .min_size(vec2(0.0, theme::CONTROL_H))
                        .vertical_align(egui::Align::Center)
                        .margin(vec2(8.0, 6.0)),
                );
            });

            widgets::hud(ui, loc.inspector_active_days_of_week, t.muted);
            let dow_names = [
                (1, loc.inspector_dow_mon),
                (2, loc.inspector_dow_tue),
                (3, loc.inspector_dow_wed),
                (4, loc.inspector_dow_thu),
                (5, loc.inspector_dow_fri),
                (6, loc.inspector_dow_sat),
                (7, loc.inspector_dow_sun),
            ];
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for (val, label) in dow_names {
                    let is_on = days_of_week.is_empty() || days_of_week.contains(&val);
                    if widgets::button(ui, label, if is_on { Kind::Primary } else { Kind::Secondary }, 0.0).clicked() {
                        if days_of_week.is_empty() {
                            *days_of_week = vec![1, 2, 3, 4, 5, 6, 7];
                            days_of_week.retain(|&d| d != val);
                        } else if days_of_week.contains(&val) {
                            days_of_week.retain(|&d| d != val);
                        } else {
                            days_of_week.push(val);
                            days_of_week.sort();
                        }
                    }
                }
                let all_active = days_of_week.is_empty() || days_of_week.len() == 7;
                if widgets::button(ui, loc.inspector_btn_all_days, if all_active { Kind::Primary } else { Kind::Secondary }, 0.0).clicked() {
                    days_of_week.clear();
                }
            });
        }
        RuleCondition::Event { event_type, idle_duration_sec } => {
            widgets::hud(ui, loc.inspector_cond_type_event, t.muted);
            let sys_events = [
                (crate::models::SystemEventType::SessionLock, format!("{} {}", egui_phosphor::regular::LOCK, loc.inspector_sys_event_lock)),
                (crate::models::SystemEventType::SessionUnlock, format!("{} {}", egui_phosphor::regular::LOCK_OPEN, loc.inspector_sys_event_unlock)),
                (crate::models::SystemEventType::SystemSleep, format!("{} {}", egui_phosphor::regular::MOON, loc.inspector_sys_event_sleep)),
                (crate::models::SystemEventType::SystemResume, format!("{} {}", egui_phosphor::regular::SUN, loc.inspector_sys_event_resume)),
                (crate::models::SystemEventType::PowerSourceAc, format!("{} {}", egui_phosphor::regular::PLUG, loc.inspector_sys_event_ac)),
                (crate::models::SystemEventType::PowerSourceBattery, format!("{} {}", egui_phosphor::regular::BATTERY_CHARGING, loc.inspector_sys_event_battery)),
                (crate::models::SystemEventType::UserIdle, format!("{} {}", egui_phosphor::regular::HOURGLASS, loc.inspector_sys_event_idle)),
                (crate::models::SystemEventType::UserActive, format!("{} {}", egui_phosphor::regular::ACTIVITY, loc.inspector_sys_event_active)),
            ];

            let selected_label = sys_events.iter().find(|(k, _)| k == event_type).map(|(_, l)| l.as_str()).unwrap_or("");
            egui::ComboBox::from_id_salt(ui.id().with("cond_sys_event_combo"))
                .selected_text(selected_label)
                .width(ui.available_width() - 8.0)
                .show_ui(ui, |ui| {
                    for (k, label) in &sys_events {
                        ui.selectable_value(event_type, *k, label);
                    }
                });

            if matches!(event_type, crate::models::SystemEventType::UserIdle | crate::models::SystemEventType::UserActive) {
                ui.add_space(4.0);
                widgets::hud(ui, loc.inspector_field_idle_duration, t.muted);
                ui.add(egui::Slider::new(idle_duration_sec, 5..=3600).suffix(" s"));
            }
        }
        RuleCondition::ResourceThreshold { process_name, cpu_percent, ram_mb, duration_sec } => {
            let mut submode = SystemConditionSubMode::Resource;
            let sub_options = [
                (SystemConditionSubMode::Resource, Cell::new(loc.inspector_cond_sub_resource, "")),
                (SystemConditionSubMode::FileWatcher, Cell::new(loc.inspector_cond_sub_file_watcher, "")),
            ];
            if widgets::cells(ui, &mut submode, &sub_options) && submode == SystemConditionSubMode::FileWatcher {
                *cond = RuleCondition::default_file();
                return;
            }
            ui.add_space(4.0);

            widgets::hud(ui, loc.inspector_field_target_process, t.muted);
            ui.horizontal(|ui| {
                ui.set_width(ui.available_width());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if render_process_picker(ui, loc) {
                        on_pick_process();
                    }
                    ui.add(
                        egui::TextEdit::singleline(process_name)
                            .hint_text("Process executable")
                            .desired_width(ui.available_width())
                            .min_size(vec2(0.0, theme::CONTROL_H))
                            .vertical_align(egui::Align::Center)
                            .margin(vec2(10.0, 8.0)),
                    );
                });
            });

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let mut has_cpu = cpu_percent.is_some();
                if ui.checkbox(&mut has_cpu, loc.inspector_cond_res_cpu).changed() {
                    *cpu_percent = if has_cpu { Some(80.0) } else { None };
                }
                if let Some(cpu) = cpu_percent {
                    ui.add(egui::DragValue::new(cpu).range(1.0..=100.0).suffix("%"));
                }

                ui.add_space(10.0);
                let mut has_mem = ram_mb.is_some();
                if ui.checkbox(&mut has_mem, loc.inspector_cond_res_ram).changed() {
                    *ram_mb = if has_mem { Some(1024) } else { None };
                }
                if let Some(mem) = ram_mb {
                    ui.add(egui::DragValue::new(mem).range(10..=65536).suffix(" MB"));
                }
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                widgets::hud(ui, loc.inspector_cond_res_duration, t.muted);
                ui.add(egui::DragValue::new(duration_sec).range(1..=3600).suffix(" s"));
            });
        }
        RuleCondition::FileWatcher { path, filter_extension, must_exist } => {
            let mut submode = SystemConditionSubMode::FileWatcher;
            let sub_options = [
                (SystemConditionSubMode::Resource, Cell::new(loc.inspector_cond_sub_resource, "")),
                (SystemConditionSubMode::FileWatcher, Cell::new(loc.inspector_cond_sub_file_watcher, "")),
            ];
            if widgets::cells(ui, &mut submode, &sub_options) && submode == SystemConditionSubMode::Resource {
                *cond = RuleCondition::default_resource();
                return;
            }
            ui.add_space(4.0);

            widgets::hud(ui, loc.inspector_cond_file_path_label, t.muted);
            ui.horizontal(|ui| {
                ui.set_width(ui.available_width());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::button(ui, loc.inspector_cond_file_btn_folder, Kind::Secondary, 0.0)
                        .on_hover_text(loc.inspector_cond_file_btn_folder_tip)
                        .clicked()
                    {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            *path = folder.to_string_lossy().to_string();
                        }
                    }
                    if widgets::button(ui, loc.inspector_cond_file_btn_file, Kind::Secondary, 0.0)
                        .on_hover_text(loc.inspector_cond_file_btn_file_tip)
                        .clicked()
                    {
                        if let Some(file) = rfd::FileDialog::new().pick_file() {
                            *path = file.to_string_lossy().to_string();
                        }
                    }
                    ui.add(
                        egui::TextEdit::singleline(path)
                            .hint_text("C:\\path\\to\\file_or_folder")
                            .desired_width(ui.available_width())
                            .min_size(vec2(0.0, theme::CONTROL_H))
                            .vertical_align(egui::Align::Center)
                            .margin(vec2(10.0, 8.0)),
                    );
                });
            });

            ui.add_space(4.0);
            widgets::hud(ui, loc.inspector_cond_file_ext_filter, t.muted);
            let mut ext_buf = filter_extension.clone().unwrap_or_default();
            if ui.add(
                egui::TextEdit::singleline(&mut ext_buf)
                    .hint_text(loc.inspector_cond_file_ext_hint)
                    .desired_width(ui.available_width())
                    .min_size(vec2(0.0, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(10.0, 8.0)),
            ).changed() {
                let trimmed = ext_buf.trim().to_string();
                *filter_extension = if trimmed.is_empty() { None } else { Some(trimmed) };
            }

            ui.add_space(4.0);
            widgets::hud(ui, loc.inspector_cond_file_state_label, t.muted);
            let file_options = [
                (true, Cell::new(loc.inspector_cond_file_exists, "")),
                (false, Cell::new(loc.inspector_cond_file_not_exists, "")),
            ];
            widgets::cells(ui, must_exist, &file_options);
        }
    }
}

fn render_conditions_section(
    ui: &mut Ui,
    state: &mut GuiState,
    rule: &mut Rule,
    t: &Tokens,
) {
    let loc = state.loc();
    let mut remove_idx = None;
    let mut pick_cond_idx = None;

    for (idx, item) in rule.conditions.iter_mut().enumerate() {
        if idx > 0 {
            render_condition_operator_connector(ui, &mut item.operator, loc, t);
        }

        if render_condition_card(
            ui,
            idx,
            item,
            loc,
            t,
            false,
            true,
            || { pick_cond_idx = Some(idx); },
        ) {
            remove_idx = Some(idx);
        }
    }

    if let Some(idx) = remove_idx {
        state.push_undo_snapshot();
        if idx < rule.conditions.len() {
            rule.conditions.remove(idx);
        }
    }

    if let Some(idx) = pick_cond_idx {
        state.open_process_picker(crate::gui::state::ProcessPickerTarget::Condition(idx));
    }

    // 44px Add Condition button at the bottom of the section
    if render_add_condition_button(ui, loc, t) {
        state.push_undo_snapshot();
        rule.conditions.push(RuleConditionItem::new(
            RuleCondition::default_process(),
        ));
    }
}

struct SubActionResponse {
    pub remove_clicked: bool,
    pub clone_clicked: bool,
    pub expand_script: Option<ScriptLanguage>,
    pub expand_macro: bool,
    pub pick_target: Option<crate::gui::state::ProcessPickerTarget>,
    pub drag_started: bool,
    pub card_rect: Rect,
}

#[allow(clippy::too_many_arguments)]
fn render_subaction_card(
    ui: &mut Ui,
    state: &GuiState,
    sub_act: &mut Action,
    parent_idx: usize,
    sub_idx: usize,
    branch_label: &str,
    default_picker: crate::gui::state::ProcessPickerTarget,
    is_exit_pipeline: bool,
    is_being_dragged: bool,
    can_remove: bool,
    t: &Tokens,
    depth: usize,
) -> SubActionResponse {
    let loc = state.loc();
    let mut remove_clicked = false;
    let mut clone_clicked = false;
    let mut drag_started = false;
    let type_badge = action_type_badge(sub_act, loc);

    let mut fields_resp = ActionFieldsResponse::default();

    let ghost_id = ui.id().with(("sub_card_ghost", parent_idx, branch_label, sub_idx));
    let ghost_anim = ui.ctx().animate_bool_with_time_and_easing(ghost_id, is_being_dragged, 0.15, easing::cubic_out);

    let (fill, stroke) = if ghost_anim > 0.001 {
        (
            t.bg,
            Stroke::new(1.0, t.accent_ink.gamma_multiply(0.45 * ghost_anim)),
        )
    } else if depth > 0 {
        (t.bg, Stroke::new(1.0, t.line_strong.gamma_multiply(0.45)))
    } else {
        (t.sunken, t.hairline())
    };

    let card_response = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                // Sub-action top bar: Step # + Type badge on left; Test / Clone / Remove on right
                ui.horizontal(|ui| {
                    ui.set_width(ui.available_width());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if can_remove && widgets::glyph_button(ui, Glyph::Close, loc.inspector_tip_remove_step).clicked() {
                            remove_clicked = true;
                        }
                        if widgets::glyph_button(ui, Glyph::Clone, loc.inspector_tip_clone_step).clicked() {
                            clone_clicked = true;
                        }
                        if widgets::glyph_button(ui, Glyph::Play, loc.inspector_btn_test_step).clicked() {
                            let (ok, msg) = match sub_act {
                                Action::Macro { events, speed, .. } => {
                                    if events.is_empty() {
                                        (false, loc.inspector_macro_test_empty.to_string())
                                    } else {
                                        crate::gui::macro_recorder::run_macro_test_safe(events.clone(), *speed);
                                        (true, loc.inspector_macro_testing_actions.replace("{}", &events.len().to_string()))
                                    }
                                }
                                _ => {
                                    let mut ctx = std::collections::HashMap::new();
                                    ctx.insert("ProcessName".to_string(), "test.exe".to_string());
                                    ctx.insert("Timestamp".to_string(), chrono::Local::now().to_rfc3339());
                                    let res = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::TestStep {
                                        action: sub_act.clone(),
                                        context: ctx,
                                    });
                                    let ok = matches!(res, Ok(crate::ipc::IpcResponse::TestStepResult(Ok(()))));
                                    let msg = match res {
                                        Ok(crate::ipc::IpcResponse::TestStepResult(Ok(()))) => "Step passed".to_string(),
                                        Ok(crate::ipc::IpcResponse::TestStepResult(Err(e))) => format!("Failed: {e}"),
                                        Ok(crate::ipc::IpcResponse::Error(e)) => format!("Error: {e}"),
                                        Err(e) => format!("IPC Error: {e}"),
                                        _ => "Unknown result".to_string(),
                                    };
                                    (ok, msg)
                                }
                            };
                            let sub_test_key = ui.id().with(("sub_test_res", parent_idx, branch_label, sub_idx));
                            ui.data_mut(|d| d.insert_temp(sub_test_key, (ok, msg, std::time::Instant::now())));
                        }

                        let sub_test_key = ui.id().with(("sub_test_res", parent_idx, branch_label, sub_idx));
                        if let Some((ok, msg, at)) = ui.data(|d| d.get_temp::<(bool, String, std::time::Instant)>(sub_test_key)) {
                            if at.elapsed().as_secs() < 8 {
                                let badge_color = if ok { t.good } else { t.bad };
                                widgets::badge(ui, if ok { "PASS" } else { "FAIL" }, badge_color).on_hover_text(msg);
                            }
                        }

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            // Drag grip handle on far left
                            let grip_resp = widgets::drag_handle(ui, loc.inspector_tip_drag_step);
                            if grip_resp.drag_started() {
                                drag_started = true;
                            }

                            widgets::hud(ui, &format!("{} {:02}", branch_label, sub_idx + 1), t.accent_ink);

                            if is_being_dragged {
                                widgets::badge(ui, loc.inspector_badge_moving_step, t.accent_ink);
                            } else {
                                widgets::badge(ui, type_badge, t.muted);
                            }

                            // Middle blank space on top strip is also draggable
                            let empty_w = (ui.available_width() - 4.0).max(0.0);
                            if empty_w > 0.0 {
                                let (_, strip_resp) = ui.allocate_exact_size(vec2(empty_w, theme::CONTROL_H), Sense::click_and_drag());
                                if strip_resp.drag_started() {
                                    drag_started = true;
                                }
                                if strip_resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                }
                            }
                        });
                    });
                });

                widgets::hrule(ui);

                // Sub-action full parameter fields!
                fields_resp = ui.add_enabled_ui(!is_being_dragged, |ui| {
                    render_action_parameter_fields(
                        ui,
                        state,
                        sub_act,
                        parent_idx * 100 + sub_idx + 500,
                        is_exit_pipeline,
                        default_picker,
                        t,
                        depth,
                    )
                }).inner;
            });
        });

    SubActionResponse {
        remove_clicked,
        clone_clicked,
        expand_script: fields_resp.expand_script,
        expand_macro: fields_resp.expand_macro,
        pick_target: fields_resp.pick_target,
        drag_started,
        card_rect: card_response.response.rect,
    }
}

#[derive(Clone, Copy)]
enum BranchPickerKind {
    Then,
    ElseIf(usize),
    Else,
}

impl BranchPickerKind {
    fn make_target(self, parent_idx: usize, sub_idx: usize, is_exit: bool) -> crate::gui::state::ProcessPickerTarget {
        match self {
            BranchPickerKind::Then => crate::gui::state::ProcessPickerTarget::IfElseThen { parent_idx, sub_idx, is_exit },
            BranchPickerKind::ElseIf(branch_idx) => crate::gui::state::ProcessPickerTarget::IfElseElseIf { parent_idx, branch_idx, sub_idx, is_exit },
            BranchPickerKind::Else => crate::gui::state::ProcessPickerTarget::IfElseElse { parent_idx, sub_idx, is_exit },
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_subactions_branch_pipeline(
    ui: &mut Ui,
    state: &GuiState,
    sub_actions: &mut Vec<Action>,
    parent_idx: usize,
    branch_key: &str,
    branch_label: &str,
    is_exit_pipeline: bool,
    picker_kind: BranchPickerKind,
    add_btn_label: &str,
    t: &Tokens,
    depth: usize,
    resp: &mut ActionFieldsResponse,
    catalog_target: Option<crate::gui::state::IfElseCatalogTarget>,
) {
    let pipeline_id = format!("sub_{}_{}_{}", parent_idx, branch_key, if is_exit_pipeline { "exit" } else { "main" });
    let dnd_id = ui.id().with(("subaction_pipeline_dnd", &pipeline_id));

    let mut dnd_session: Option<DndDragSession> = ui.data_mut(|d| d.get_temp(dnd_id));

    // Cancel on Escape key
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        dnd_session = None;
        ui.data_mut(|d| d.remove_temp::<DndDragSession>(dnd_id));
    }

    let pointer_released = ui.input(|i| i.pointer.any_released());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));

    // When dragging: handle mouse wheel scrolling and edge auto-scrolling
    if dnd_session.is_some() {
        let wheel_delta_y = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel_delta_y != 0.0 {
            ui.scroll_with_delta(vec2(0.0, wheel_delta_y * 1.5));
            ui.ctx().request_repaint();
        }

        if let Some(pointer) = pointer_pos {
            let viewport = ui.clip_rect();
            if pointer.x >= viewport.left() - 60.0 && pointer.x <= viewport.right() + 60.0 {
                let edge_zone = 90.0;
                let view_top = viewport.top();
                let view_bottom = viewport.bottom();

                if pointer.y < view_top + edge_zone {
                    let factor = ((view_top + edge_zone - pointer.y) / edge_zone).clamp(0.1, 3.0);
                    let scroll_speed = 8.0 + 26.0 * factor;
                    ui.scroll_with_delta(vec2(0.0, scroll_speed));
                    ui.ctx().request_repaint();
                } else if pointer.y > view_bottom - edge_zone {
                    let factor = ((pointer.y - (view_bottom - edge_zone)) / edge_zone).clamp(0.1, 3.0);
                    let scroll_speed = 8.0 + 26.0 * factor;
                    ui.scroll_with_delta(vec2(0.0, -scroll_speed));
                    ui.ctx().request_repaint();
                }
            }
        }
    }

    let active_src = dnd_session.as_ref().map(|s| s.source_idx);
    let active_dst = dnd_session.as_ref().map(|s| s.target_idx);

    let mut remove_idx = None;
    let mut clone_idx = None;
    let mut drag_started_idx = None;
    let mut current_centers = Vec::with_capacity(sub_actions.len());

    let loc = state.loc();
    let total = sub_actions.len();

    for (s_idx, sub_act) in sub_actions.iter_mut().enumerate() {
        // Drop slot BEFORE card s_idx
        render_drop_slot(ui, loc, s_idx, active_dst, active_src, &pipeline_id, branch_label, t);

        let is_ghost = active_src == Some(s_idx);
        let picker_target = picker_kind.make_target(parent_idx, s_idx, is_exit_pipeline);

        let sub_res = render_subaction_card(
            ui,
            state,
            sub_act,
            parent_idx,
            s_idx,
            branch_label,
            picker_target,
            is_exit_pipeline,
            is_ghost,
            true,
            t,
            depth,
        );

        current_centers.push(sub_res.card_rect.center().y);

        if sub_res.remove_clicked {
            remove_idx = Some(s_idx);
        }
        if sub_res.clone_clicked {
            clone_idx = Some(s_idx);
        }
        if let Some(target) = sub_res.pick_target {
            resp.pick_target = Some(target);
        }
        if let Some(sc) = sub_res.expand_script {
            resp.expand_script = Some(sc);
        }
        if sub_res.expand_macro {
            resp.expand_macro = true;
        }
        if sub_res.drag_started {
            drag_started_idx = Some(s_idx);
        }
    }

    // Render trailing drop slot (after last card)
    if total > 0 {
        render_drop_slot(ui, loc, total, active_dst, active_src, &pipeline_id, branch_label, t);
    }

    // Update target_idx using fresh current_centers from this exact frame
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

    // Commit reorder on mouse release
    if pointer_released {
        if let Some(session) = dnd_session {
            let src = session.source_idx;
            let dst = session.target_idx;
            if src < sub_actions.len() && dst <= sub_actions.len() {
                if dst < src {
                    let item = sub_actions.remove(src);
                    sub_actions.insert(dst, item);
                } else if dst > src + 1 {
                    let item = sub_actions.remove(src);
                    sub_actions.insert(dst - 1, item);
                }
            }
            dnd_session = None;
            ui.data_mut(|d| d.remove_temp::<DndDragSession>(dnd_id));
        }
    }

    // Handle cloning
    if let Some(idx) = clone_idx {
        if idx < sub_actions.len() {
            let cloned = sub_actions[idx].clone();
            sub_actions.insert(idx + 1, cloned);
        }
    }

    // Handle removal
    if let Some(idx) = remove_idx {
        if idx < sub_actions.len() {
            sub_actions.remove(idx);
        }
    }

    // Handle newly started drag
    if let Some(src_idx) = drag_started_idx {
        dnd_session = Some(DndDragSession {
            source_idx: src_idx,
            target_idx: src_idx,
            card_width: ui.available_width(),
        });
    }

    // Save or clear dnd_session
    if let Some(session) = dnd_session {
        ui.data_mut(|d| d.insert_temp(dnd_id, session));
        ui.ctx().request_repaint();

        // Render floating drag preview card!
        render_floating_drag_preview(
            ui,
            loc,
            &sub_actions[session.source_idx],
            session,
            pointer_pos,
            branch_label,
            &pipeline_id,
            t,
        );
    }

    // Full-width 44px Add Step button at the bottom of the branch!
    if render_full_width_action_button(ui, add_btn_label, add_btn_label, t) {
        if let Some(target) = catalog_target {
            resp.open_catalog_nested = Some(target);
        }
    }
}

fn render_process_inspector(ui: &mut Ui, state: &mut GuiState, proc: &ProcessSnapshotItem, t: &Tokens) {
    let loc = state.loc();
    let scrollbar_w = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin + ui.spacing().scroll.bar_outer_margin;
    let right_m = (20.0 - scrollbar_w).max(0.0).round() as i8;

    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 20,
            right: right_m,
            top: 8,
            bottom: 8,
        })
        .show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;

            // Window Title
            widgets::hud(ui, loc.inspector_field_window_title, t.muted);
            let wtitle = if proc.window_title.is_empty() {
                loc.inspector_no_active_window
            } else {
                &proc.window_title
            };
            ui.label(
                egui::RichText::new(wtitle)
                    .family(theme::medium())
                    .size(13.0)
                    .color(t.ink),
            );

            // RAM Working Set
            let mb = proc.memory_bytes as f64 / 1_048_576.0;
            widgets::hud(ui, loc.inspector_field_working_set_ram, t.muted);
            ui.horizontal_wrapped(|ui| {
                widgets::badge(ui, &format!("{:.1} MB", mb), t.good);
                widgets::hud(ui, &format!("{} {}", proc.memory_bytes, loc.inspector_bytes_suffix), t.faint);
            });

            widgets::hrule(ui);

            widgets::eyebrow(ui, "02", loc.inspector_eyebrow_quick_actions);

            if widgets::button(ui, loc.inspector_btn_create_automation, Kind::Secondary, 0.0).clicked() {
                state.apply_process_to_draft_or_new_rule(&proc.name);
            }

            widgets::hrule(ui);

            if widgets::button(ui, loc.inspector_btn_terminate_process, Kind::Danger, 0.0).clicked() {
                let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::KillProcess {
                    pid: proc.pid,
                    force: true,
                });
                state.set_toast(format!("{} {} #{}", loc.toast_process_terminated, proc.name, proc.pid));
                state.selected_process = None;
            }
        });
    });
}

fn render_process_picker(ui: &mut Ui, loc: &crate::i18n::Strings) -> bool {
    widgets::button(ui, loc.inspector_pick_running, Kind::Secondary, 0.0)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn render_datetime_picker(
    ui: &mut Ui,
    loc: &crate::i18n::Strings,
    target_datetime: &mut Option<String>,
    t: &Tokens,
    id_salt: &str,
) {
    let now = chrono::Local::now();
    if target_datetime.is_none() {
        let tomorrow = now.date_naive().succ_opt().unwrap_or(now.date_naive());
        *target_datetime = Some(format!(
            "{:04}-{:02}-{:02} 12:00:00",
            tomorrow.year(),
            tomorrow.month(),
            tomorrow.day()
        ));
    }

    let (mut sel_year, mut sel_month, mut sel_day, mut sel_hour, mut sel_min, mut sel_sec) = {
        let dt_str = target_datetime.as_deref().unwrap_or("");
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S") {
            (dt.year(), dt.month(), dt.day(), dt.hour(), dt.minute(), dt.second())
        } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&format!("{}:00", dt_str), "%Y-%m-%d %H:%M:%S") {
            (dt.year(), dt.month(), dt.day(), dt.hour(), dt.minute(), 0)
        } else {
            (now.year(), now.month(), now.day(), 12, 0, 0)
        }
    };

    let view_month_id = ui.id().with(id_salt).with("cal_view_month");
    let view_year_id = ui.id().with(id_salt).with("cal_view_year");
    let mut view_month: u32 = ui.data_mut(|d| d.get_temp(view_month_id).unwrap_or(sel_month));
    let mut view_year: i32 = ui.data_mut(|d| d.get_temp(view_year_id).unwrap_or(sel_year));

    widgets::hud(ui, loc.inspector_mini_calendar, t.muted);
    ui.horizontal(|ui| {
        if widgets::button(ui, "◀", Kind::Secondary, 24.0).clicked() {
            if view_month == 1 {
                view_month = 12;
                view_year -= 1;
            } else {
                view_month -= 1;
            }
            ui.data_mut(|d| {
                d.insert_temp(view_month_id, view_month);
                d.insert_temp(view_year_id, view_year);
            });
        }

        let month_name = if (1..=12).contains(&view_month) {
            loc.inspector_cal_month_names[(view_month - 1) as usize]
        } else {
            ""
        };
        ui.label(egui::RichText::new(format!("{} {}", month_name, view_year)).strong());

        if widgets::button(ui, "▶", Kind::Secondary, 24.0).clicked() {
            if view_month == 12 {
                view_month = 1;
                view_year += 1;
            } else {
                view_month += 1;
            }
            ui.data_mut(|d| {
                d.insert_temp(view_month_id, view_month);
                d.insert_temp(view_year_id, view_year);
            });
        }
    });

    if let Some(first_day) = chrono::NaiveDate::from_ymd_opt(view_year, view_month, 1) {
        let start_col = first_day.weekday().number_from_monday() as usize;
        let days_in_cur_month = days_in_month(view_year, view_month);

        let two_digit_w = (widgets::caps(ui, "00", theme::HUD_SIZE, theme::mono_bold(), theme::BUTTON_SPACING, t.ink).size().x + 28.0).max(44.0);

        egui::Grid::new(ui.id().with(id_salt).with("mini_cal_grid"))
            .spacing(vec2(3.0, 3.0))
            .show(ui, |ui| {
                for d in loc.inspector_cal_day_names {
                    let (rect, _) = ui.allocate_exact_size(vec2(two_digit_w, 18.0), Sense::hover());
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        d,
                        egui::FontId::new(11.0, theme::mono()),
                        t.muted,
                    );
                }
                ui.end_row();

                let mut col = 1;
                for _ in 1..start_col {
                    ui.allocate_exact_size(vec2(two_digit_w, theme::CONTROL_H), Sense::hover());
                    col += 1;
                }

                for day in 1..=days_in_cur_month {
                    let is_selected = sel_year == view_year && sel_month == view_month && sel_day == day;
                    let day_str = format!("{}", day);
                    let btn = widgets::button(
                        ui,
                        &day_str,
                        if is_selected { Kind::Primary } else { Kind::Secondary },
                        two_digit_w,
                    );
                    if btn.clicked() {
                        sel_year = view_year;
                        sel_month = view_month;
                        sel_day = day;
                        *target_datetime = Some(format!(
                            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                            sel_year, sel_month, sel_day, sel_hour, sel_min, sel_sec
                        ));
                    }
                    col += 1;
                    if col > 7 {
                        ui.end_row();
                        col = 1;
                    }
                }
                if col != 1 {
                    ui.end_row();
                }
            });
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        widgets::hud(ui, loc.inspector_execution_time, t.muted);
        let h_ch = ui.add(
            egui::DragValue::new(&mut sel_hour)
                .range(0..=23)
                .speed(0.1)
                .suffix(" h"),
        ).changed();
        ui.label(":");
        let m_ch = ui.add(
            egui::DragValue::new(&mut sel_min)
                .range(0..=59)
                .speed(0.1)
                .suffix(" m"),
        ).changed();
        ui.label(":");
        let s_ch = ui.add(
            egui::DragValue::new(&mut sel_sec)
                .range(0..=59)
                .speed(0.1)
                .suffix(" s"),
        ).changed();

        if h_ch || m_ch || s_ch {
            *target_datetime = Some(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                sel_year, sel_month, sel_day, sel_hour, sel_min, sel_sec
            ));
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_schedule_trigger(
    ui: &mut Ui,
    loc: &crate::i18n::Strings,
    schedule_type: &mut ScheduleType,
    interval_sec: &mut u64,
    daily_time: &mut Option<String>,
    days_of_week: &mut Vec<u8>,
    target_datetime: &mut Option<String>,
    t: &Tokens,
) {
    let sub_options = [
        (ScheduleType::Interval, Cell::new(loc.inspector_sched_tab_interval, "")),
        (ScheduleType::Daily, Cell::new(loc.inspector_sched_tab_daily, "")),
        (ScheduleType::OneShot, Cell::new(loc.inspector_sched_tab_specific, "")),
    ];

    let prev_type = *schedule_type;
    if widgets::cells(ui, schedule_type, &sub_options) && *schedule_type != prev_type {
        match *schedule_type {
            ScheduleType::Interval => {
                *target_datetime = None;
            }
            ScheduleType::Daily => {}
            ScheduleType::OneShot if target_datetime.is_none() => {
                let now = chrono::Local::now();
                *target_datetime = Some(format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    now.year(), now.month(), now.day(), now.hour(), now.minute(), now.second()
                ));
            }
            _ => {}
        }
    }

    ui.add_space(8.0);

    match schedule_type {
        ScheduleType::Interval => {
            widgets::hud(ui, loc.inspector_preset_intervals, t.muted);
            let presets: &[(&str, u64)] = &[
                ("1S", 1),
                ("1M", 60),
                ("1H", 3600),
                ("1D", 86400),
                ("1W", 604800),
                ("1MO", 2592000),
                ("1Y", 31536000),
            ];

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for &(label, secs) in presets {
                    let on = *interval_sec == secs;
                    let tip = match label {
                        "1S" => loc.inspector_preset_tip_1s,
                        "1M" => loc.inspector_preset_tip_1m,
                        "1H" => loc.inspector_preset_tip_1h,
                        "1D" => loc.inspector_preset_tip_1d,
                        "1W" => loc.inspector_preset_tip_1w,
                        "1MO" => loc.inspector_preset_tip_1mo,
                        "1Y" => loc.inspector_preset_tip_1y,
                        _ => "",
                    };
                    let btn = widgets::button(ui, label, if on { Kind::Primary } else { Kind::Secondary }, 0.0);
                    let btn = if !tip.is_empty() { btn.on_hover_text(tip) } else { btn };
                    if btn.clicked() {
                        *interval_sec = secs;
                    }
                }
            });

            ui.add_space(4.0);
            widgets::hud(ui, loc.inspector_custom_interval, t.muted);

            let s = *interval_sec;
            let mut hours = s / 3600;
            let mut mins = (s % 3600) / 60;
            let mut secs = s % 60;

            ui.horizontal_wrapped(|ui| {
                let h_ch = ui.add(
                    egui::DragValue::new(&mut hours)
                        .range(0..=999_999)
                        .speed(0.2)
                        .suffix(" h"),
                ).changed();
                ui.label(":");
                let m_ch = ui.add(
                    egui::DragValue::new(&mut mins)
                        .range(0..=59)
                        .speed(0.2)
                        .suffix(" m"),
                ).changed();
                ui.label(":");
                let s_ch = ui.add(
                    egui::DragValue::new(&mut secs)
                        .range(0..=59)
                        .speed(0.2)
                        .suffix(" s"),
                ).changed();

                if h_ch || m_ch || s_ch {
                    let total = (hours * 3600) + (mins * 60) + secs;
                    *interval_sec = if total == 0 { 1 } else { total };
                }
            });

            ui.add_space(8.0);
            widgets::hrule(ui);
            let mut has_start_time = target_datetime.is_some();
            if ui.checkbox(&mut has_start_time, loc.inspector_checkbox_first_execution).changed() {
                if has_start_time {
                    let now = chrono::Local::now();
                    *target_datetime = Some(format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                        now.year(), now.month(), now.day(), now.hour(), now.minute(), now.second()
                    ));
                } else {
                    *target_datetime = None;
                }
            }

            if has_start_time {
                ui.add_space(4.0);
                render_datetime_picker(ui, loc, target_datetime, t, "interval_cal");
            }
        }

        ScheduleType::Daily => {
            widgets::hud(ui, loc.inspector_active_days_of_week, t.muted);
            let dow_names = [
                (1, loc.inspector_dow_mon),
                (2, loc.inspector_dow_tue),
                (3, loc.inspector_dow_wed),
                (4, loc.inspector_dow_thu),
                (5, loc.inspector_dow_fri),
                (6, loc.inspector_dow_sat),
                (7, loc.inspector_dow_sun),
            ];

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for (val, label) in dow_names {
                    let is_on = days_of_week.is_empty() || days_of_week.contains(&val);
                    if widgets::button(ui, label, if is_on { Kind::Primary } else { Kind::Secondary }, 0.0).clicked() {
                        if days_of_week.is_empty() {
                            *days_of_week = vec![1, 2, 3, 4, 5, 6, 7];
                            days_of_week.retain(|&d| d != val);
                        } else if days_of_week.contains(&val) {
                            days_of_week.retain(|&d| d != val);
                        } else {
                            days_of_week.push(val);
                            days_of_week.sort();
                        }
                    }
                }
                let all_active = days_of_week.is_empty() || days_of_week.len() == 7;
                if widgets::button(ui, loc.inspector_btn_all_days, if all_active { Kind::Primary } else { Kind::Secondary }, 0.0).clicked() {
                    days_of_week.clear();
                }
            });

            ui.add_space(4.0);

            if daily_time.is_none() {
                *daily_time = Some("12:00:00".into());
            }

            let cur_str = daily_time.as_deref().unwrap_or("12:00:00");
            let (mut hour, mut min, mut sec) = {
                let parts: Vec<&str> = cur_str.split(':').collect();
                let h = parts.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(12);
                let m = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                let s = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                (h, m, s)
            };

            widgets::hud(ui, loc.inspector_daily_execution_time, t.muted);
            ui.horizontal_wrapped(|ui| {
                let h_ch = ui.add(
                    egui::DragValue::new(&mut hour)
                        .range(0..=23)
                        .speed(0.1)
                        .suffix(" h"),
                ).changed();
                ui.label(":");
                let m_ch = ui.add(
                    egui::DragValue::new(&mut min)
                        .range(0..=59)
                        .speed(0.1)
                        .suffix(" m"),
                ).changed();
                ui.label(":");
                let s_ch = ui.add(
                    egui::DragValue::new(&mut sec)
                        .range(0..=59)
                        .speed(0.1)
                        .suffix(" s"),
                ).changed();

                if h_ch || m_ch || s_ch {
                    *daily_time = Some(format!("{:02}:{:02}:{:02}", hour, min, sec));
                }
            });
        }

        ScheduleType::OneShot => {
            render_datetime_picker(ui, loc, target_datetime, t, "oneshot_cal");
        }

        _ => {}
    }
}
