//! Inspector Drawer: Slides in from the right edge with cubic easing,
//! dimmed background scrim, and craft-launcher styling for editing rules
//! and process details.

use egui::{
    CornerRadius, Id, LayerId, Order, Pos2, Rect, Sense, Stroke, Ui, emath::easing, pos2, vec2,
};
use crate::gui::components::script_highlighter::{create_script_layouter, ScriptLanguage};
use crate::gui::components::widgets::{self, Glyph, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    Action, MicrophoneActionType, ProcessAudioActionType, ProcessMatchMode, ProcessSnapshotItem, Rule, ScheduleType, SoundActionType,
    SystemEventType, SystemPowerType, Trigger,
};
use chrono::{Datelike, Timelike};

pub fn render_inspector_drawer(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);

    let is_open = state.editing_rule.is_some() || state.selected_process.is_some();

    // Handle Esc key to close inspector (only if script canvas and action catalog are NOT open)
    if is_open
        && state.expanded_script_editor.is_none()
        && state.action_catalog.is_none()
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

            // 1. Fixed Top Header Strip (anchored, spans full drawer width, never jumps)
            let mut close_editor = false;
            let mut close_process = false;
            let mut reset_builder = false;

            if let Some(rule) = &state.editing_rule {
                let rule_name = rule.name.clone();
                let exec_opt = state.daemon_state.as_ref().and_then(|s| s.active_rule_executions.get(&rule.id)).copied();
                egui::Frame::new()
                    .fill(t.sunken)
                    .stroke(t.hairline())
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(egui::Margin::symmetric(20, 14))
                    .show(ui, |ui| {
                        let loc = state.loc();
                        ui.set_width(ui.available_width());
                        ui.set_max_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            widgets::eyebrow(ui, "01", loc.inspector_eyebrow_builder);
                            if let Some(exec) = exec_opt {
                                ui.add_space(8.0);
                                let count_suffix = if exec.active_count > 1 {
                                    format!(" (x{})", exec.active_count)
                                } else {
                                    String::new()
                                };
                                let running_label = format!(
                                    "{}{}: {}/{}",
                                    loc.rules_badge_running,
                                    count_suffix,
                                    exec.current_step + 1,
                                    exec.total_steps
                                );
                                widgets::badge(ui, &running_label, t.accent_ink);
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_close_tip).clicked() {
                                    close_editor = true;
                                }
                                if state.is_creating_new_rule {
                                    ui.add_space(4.0);
                                    if widgets::glyph_button(ui, Glyph::Reset, loc.inspector_btn_reset_tip).clicked() {
                                        reset_builder = true;
                                    }
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(&rule_name)
                                .family(theme::semibold())
                                .size(17.0)
                                .color(t.ink_strong),
                        );
                    });
            } else if let Some(proc) = &state.selected_process {
                let proc_name = proc.name.clone();
                let proc_pid = proc.pid;
                egui::Frame::new()
                    .fill(t.sunken)
                    .stroke(t.hairline())
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(egui::Margin::symmetric(20, 14))
                    .show(ui, |ui| {
                        let loc = state.loc();
                        ui.set_width(ui.available_width());
                        ui.set_max_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            widgets::eyebrow(ui, "01", loc.inspector_eyebrow_task_details);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_close_tip).clicked() {
                                    close_process = true;
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&proc_name)
                                    .family(theme::semibold())
                                    .size(17.0)
                                    .color(t.ink_strong),
                            );
                            widgets::hud(ui, &format!("#{}", proc_pid), t.muted);
                        });
                    });
            }

            if reset_builder {
                state.reset_builder_state();
            }
            if close_editor {
                state.close_editor();
            }
            if close_process {
                state.selected_process = None;
            }

            // 2. Scrollable Body with strictly pinned width
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .id_salt("inspector_scroll_area")
                .max_width(width)
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_max_width(ui.available_width());

                    if let Some(mut rule) = state.editing_rule.clone() {
                        render_rule_editor(ui, state, &mut rule, &t);
                        if state.editing_rule.is_some() {
                            state.editing_rule = Some(rule);
                        }
                    } else if let Some(proc) = state.selected_process.clone() {
                        render_process_inspector(ui, state, &proc, &t);
                    }
                });
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
            top: 20,
            bottom: 20,
        })
        .show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_max_width(ui.available_width());
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.set_max_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;

            // Basic Info
            let loc = state.loc();
            widgets::hud(ui, loc.inspector_field_rule_name, t.muted);
            ui.add(
                egui::TextEdit::singleline(&mut rule.name)
                    .desired_width(ui.available_width())
                    .min_size(vec2(0.0, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(10.0, 8.0)),
            );

            widgets::hud(ui, loc.inspector_field_description, t.muted);
            ui.add(
                egui::TextEdit::singleline(&mut rule.description)
                    .hint_text(loc.inspector_field_description_hint)
                    .desired_width(ui.available_width())
                    .min_size(vec2(0.0, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(10.0, 8.0)),
            );

            ui.horizontal_wrapped(|ui| {
                widgets::hud(ui, loc.inspector_field_cooldown, t.muted);
                ui.add(egui::DragValue::new(&mut rule.cooldown_seconds).speed(0.1).range(0.0..=60.0).suffix(" s"));
                ui.add_space(16.0);
                ui.checkbox(&mut rule.is_enabled, loc.inspector_checkbox_rule_active);
            });

            widgets::hrule(ui);

            // Rule Icon
            widgets::hud(ui, loc.inspector_sec_icon, t.muted);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

                // Icon Preview Box (36x36)
                let icon_box_size = vec2(36.0, 36.0);
                let (icon_rect, _) = ui.allocate_exact_size(icon_box_size, Sense::hover());
                ui.painter().rect_filled(icon_rect, CornerRadius::ZERO, t.sunken);
                ui.painter().rect_stroke(icon_rect, CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);

                let mut rendered = false;
                if let Some(path) = &rule.custom_icon_path {
                    if let Some(tex) = state.icon_cache.get_icon_from_path(ui.ctx(), path) {
                        let size = tex.size_vec2();
                        let max_dim = size.x.max(size.y).max(1.0);
                        let scale = 26.0 / max_dim;
                        let render_size = vec2((size.x * scale).round().max(1.0), (size.y * scale).round().max(1.0));
                        let img_rect = Rect::from_center_size(icon_rect.center(), render_size);
                        ui.painter().image(
                            tex.id(),
                            img_rect,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        rendered = true;
                    }
                }

                if !rendered {
                    let glyph = rule.default_icon_glyph();
                    ui.painter().text(
                        icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        glyph,
                        egui::FontId::new(18.0, theme::phosphor()),
                        t.ink_strong,
                    );
                }

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;

                    let icon_status_text = match &rule.custom_icon_path {
                        Some(p) => {
                            let fname = std::path::Path::new(p).file_name().unwrap_or_default().to_string_lossy();
                            format!("{}: {}", loc.inspector_icon_custom_label, fname)
                        }
                        None => loc.inspector_icon_default_label.to_string(),
                    };
                    widgets::hud(ui, &icon_status_text, t.muted);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;

                        if widgets::button(ui, loc.inspector_icon_browse, Kind::Secondary, 0.0)
                            .on_hover_text(loc.inspector_icon_browse_tip)
                            .clicked()
                        {
                            if let Some(picked) = rfd::FileDialog::new()
                                .set_title("Select Rule Icon")
                                .add_filter("Icon / Image / Exe", &["exe", "ico", "png", "jpg", "jpeg", "dll"])
                                .pick_file()
                            {
                                rule.custom_icon_path = Some(picked.to_string_lossy().to_string());
                            }
                        }

                        if rule.custom_icon_path.is_some() {
                            if widgets::button(ui, loc.inspector_icon_reset, Kind::Danger, 0.0)
                                .on_hover_text(loc.inspector_icon_reset_tip)
                                .clicked()
                            {
                                rule.custom_icon_path = None;
                            }
                        }
                    });
                });
            });

            ui.checkbox(&mut rule.auto_process_icon, loc.inspector_icon_auto_process)
                .on_hover_text(loc.inspector_icon_auto_process_tip);

            widgets::hrule(ui);

            // 1. Trigger
            widgets::eyebrow(ui, "02", loc.inspector_eyebrow_trigger);

            if rule.trigger.is_none() {
                rule.trigger = Some(Trigger::Process {
                    process_name: "notepad.exe".into(),
                    match_mode: ProcessMatchMode::Exact,
                    single_instance: false,
                });
            }

            let trigger_types = [loc.inspector_trigger_process, loc.inspector_trigger_system, loc.inspector_trigger_timer];
            let mut current_idx = match &rule.trigger {
                Some(Trigger::Process { .. }) => 0,
                Some(Trigger::SystemEvent { .. }) => 1,
                Some(Trigger::Schedule { .. }) => 2,
                None => 0,
            };

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for (idx, label) in trigger_types.iter().enumerate() {
                    let on = current_idx == idx;
                    if widgets::button(ui, label, if on { Kind::Primary } else { Kind::Secondary }, 0.0).clicked() {
                        current_idx = idx;
                        rule.trigger = match idx {
                            0 => Some(Trigger::Process {
                                process_name: "notepad.exe".into(),
                                match_mode: ProcessMatchMode::Exact,
                                single_instance: false,
                            }),
                            1 => Some(Trigger::SystemEvent {
                                event_type: SystemEventType::SessionLock,
                                idle_duration_sec: 300,
                                device_id_filter: None,
                            }),
                            2 => Some(Trigger::Schedule {
                                schedule_type: ScheduleType::Interval,
                                interval_sec: 60,
                                daily_time: Some("12:00:00".into()),
                                days_of_week: Vec::new(),
                                delay_sec: 0,
                                target_datetime: None,
                            }),
                            _ => None,
                        };
                    }
                }
            });

            let mut is_process_trigger = false;
            let mut switch_trigger_to_hunter = false;

            if let Some(trig) = &mut rule.trigger {
                match trig {
                    Trigger::Process { process_name, match_mode, single_instance } => {
                        is_process_trigger = true;

                        ui.add_space(8.0);
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

                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;

                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, loc.inspector_field_match_mode, t.muted);
                                ui.radio_value(match_mode, ProcessMatchMode::Exact, loc.inspector_match_mode_exact);
                                ui.radio_value(match_mode, ProcessMatchMode::Wildcard, loc.inspector_match_mode_wildcard);
                                ui.radio_value(match_mode, ProcessMatchMode::Regex, loc.inspector_match_mode_contains);
                            });

                            ui.checkbox(single_instance, loc.inspector_checkbox_single_instance);
                        });
                    }
                    Trigger::SystemEvent { event_type, idle_duration_sec, .. } => {
                        ui.add_space(8.0);
                        widgets::hud(ui, loc.inspector_field_sys_event_type, t.muted);
                        let events = [
                            (SystemEventType::SessionLock, loc.inspector_sys_event_lock),
                            (SystemEventType::SessionUnlock, loc.inspector_sys_event_unlock),
                            (SystemEventType::SystemSleep, loc.inspector_sys_event_sleep),
                            (SystemEventType::SystemResume, loc.inspector_sys_event_resume),
                            (SystemEventType::PowerSourceAc, loc.inspector_sys_event_ac),
                            (SystemEventType::PowerSourceBattery, loc.inspector_sys_event_battery),
                            (SystemEventType::UserIdle, loc.inspector_sys_event_idle),
                            (SystemEventType::UserActive, loc.inspector_sys_event_active),
                        ];

                        egui::ComboBox::from_id_salt("sys_event_combo")
                            .selected_text(format!("{:?}", event_type))
                            .width(260.0)
                            .show_ui(ui, |ui| {
                                for (ev, label) in events {
                                    ui.selectable_value(event_type, ev, label);
                                }
                            });

                        if *event_type == SystemEventType::UserIdle {
                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, loc.inspector_field_idle_duration, t.muted);
                                ui.add(egui::DragValue::new(idle_duration_sec).range(10..=3600).suffix(" s"));
                            });
                        }
                    }
                    Trigger::Schedule {
                        schedule_type,
                        interval_sec,
                        daily_time,
                        days_of_week,
                        target_datetime,
                        ..
                    } => {
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
            }

            if switch_trigger_to_hunter {
                state.editing_rule = Some(rule.clone());
                state.switch_to_hunter_from_picker(crate::gui::state::ProcessPickerTarget::Trigger);
            }

            widgets::hrule(ui);

            // 2. Action Sequence Builder (Execution on trigger)
            let action_header = if is_process_trigger {
                loc.inspector_eyebrow_actions_start
            } else {
                loc.inspector_eyebrow_actions_pipeline
            };
            widgets::eyebrow(ui, "03", action_header);

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
                state.editing_rule = Some(rule.clone());
                state.switch_to_hunter_from_picker(crate::gui::state::ProcessPickerTarget::Action(idx));
            }

            // 3. Actions on Process Exit
            if is_process_trigger {
                widgets::hrule(ui);
                widgets::eyebrow(ui, "04", loc.inspector_eyebrow_actions_exit);

                if let Some(idx) = render_actions_pipeline(
                    ui,
                    state,
                    &mut rule.on_exit_actions,
                    "exit_actions",
                    true,
                    exec_opt,
                    t,
                ) {
                    state.editing_rule = Some(rule.clone());
                    state.switch_to_hunter_from_picker(crate::gui::state::ProcessPickerTarget::ExitAction(idx));
                }
            }

            widgets::hrule(ui);

            // Save / Cancel Actions
            ui.horizontal_wrapped(|ui| {
                if widgets::button(ui, loc.inspector_btn_save_rule, Kind::Primary, 120.0).clicked() {
                    state.save_rule(rule.clone());
                }

                if !state.is_creating_new_rule
                    && widgets::button(ui, loc.inspector_btn_clone, Kind::Secondary, 0.0)
                        .on_hover_text(loc.inspector_btn_clone_tip)
                        .clicked()
                {
                    state.clone_rule(rule);
                }

                if widgets::button(ui, loc.inspector_btn_cancel, Kind::Secondary, 0.0).clicked() {
                    state.close_editor();
                }
            });
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
        Action::PowerShell { .. } => loc.inspector_badge_powershell,
        Action::Cmd { .. } => loc.inspector_badge_cmd,
        Action::Notification { .. } => loc.inspector_badge_notification,
        Action::Sound { .. } => loc.inspector_badge_audio_volume,
        Action::Microphone { .. } => loc.inspector_badge_microphone,
        Action::ProcessAudio { .. } => loc.inspector_badge_process_audio,
        Action::SystemPower { .. } => loc.inspector_badge_system_power,
        Action::Delay { .. } => loc.inspector_badge_delay,
        Action::HttpWebhook { .. } => loc.inspector_badge_webhook,
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
        Action::PowerShell { script, .. } => {
            let first_line = script.lines().next().unwrap_or("PowerShell script").trim();
            if first_line.is_empty() {
                "PowerShell script".to_string()
            } else {
                first_line.chars().take(40).collect()
            }
        }
        Action::Cmd { command, .. } => {
            let c = command.trim();
            if c.is_empty() {
                "Command prompt".to_string()
            } else {
                c.chars().take(40).collect()
            }
        }
        Action::Notification { title, .. } => {
            if title.is_empty() {
                "Notification toast".to_string()
            } else {
                format!("Notify \"{}\"", title)
            }
        }
        Action::Sound { sound_type, volume_level } => match sound_type {
            SoundActionType::PlayBeep => "Play System Beep".into(),
            SoundActionType::Mute => "Mute Master Audio".into(),
            SoundActionType::Unmute => "Unmute Master Audio".into(),
            SoundActionType::ToggleMute => "Toggle Master Mute".into(),
            SoundActionType::SetVolume => format!("Master Volume {}%", volume_level),
        },
        Action::Microphone { action_type, volume_level } => match action_type {
            MicrophoneActionType::Mute => "Mute Microphone".into(),
            MicrophoneActionType::Unmute => "Unmute Microphone".into(),
            MicrophoneActionType::ToggleMute => "Toggle Microphone Mute".into(),
            MicrophoneActionType::SetVolume => format!("Microphone Volume {}%", volume_level),
        },
        Action::ProcessAudio {
            process_name,
            action_type,
            volume_level,
        } => match action_type {
            ProcessAudioActionType::Mute => format!("Mute {}", process_name),
            ProcessAudioActionType::Unmute => format!("Unmute {}", process_name),
            ProcessAudioActionType::ToggleMute => format!("Toggle mute {}", process_name),
            ProcessAudioActionType::SetVolume => format!("Volume {}% {}", volume_level, process_name),
        },
        Action::SystemPower { power_type } => format!("Power: {:?}", power_type),
        Action::Delay { seconds } => format!("Wait {:.1} seconds", seconds),
        Action::HttpWebhook { url, method, .. } => {
            if url.is_empty() {
                "HTTP Webhook".to_string()
            } else {
                format!("{} {}", method, url)
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_drop_slot(
    ui: &mut Ui,
    loc: &crate::i18n::Strings,
    slot_idx: usize,
    active_dst: Option<usize>,
    active_src: Option<usize>,
    pipeline_id: &str,
    is_exit_pipeline: bool,
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
        let prefix = if is_exit_pipeline { loc.inspector_exit_prefix } else { loc.inspector_step_prefix };
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
    is_exit_pipeline: bool,
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
    let prefix = if is_exit_pipeline { loc.inspector_exit_prefix } else { loc.inspector_step_prefix };
    let step_txt = format!("{} {:02}", prefix, session.source_idx + 1);
    let step_galley = widgets::caps(ui, &step_txt, 11.0, theme::mono_bold(), 1.0, t.accent_ink);
    painter.galley_with_override_text_color(
        pos2(preview_rect.left() + 26.0, preview_rect.top() + 8.0),
        step_galley,
        t.accent_ink,
    );

    // Type badge
    let type_badge = action_type_badge(action, loc);
    let badge_galley = widgets::caps(ui, type_badge, 10.0, theme::mono(), 1.0, t.ink_strong);
    let badge_rect = Rect::from_min_size(
        pos2(preview_rect.left() + 84.0, preview_rect.top() + 7.0),
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
    for (idx, action) in actions.iter_mut().enumerate() {
        // Render drop slot BEFORE card idx
        render_drop_slot(ui, loc, idx, active_dst, active_src, pipeline_id, is_exit_pipeline, t);

        // Render card idx
        let is_ghost = active_src == Some(idx);
        let is_executing = exec_opt.map_or(false, |e| e.is_exit == is_exit_pipeline && e.current_step == idx);
        let resp = render_action_card(
            ui,
            state,
            action,
            idx,
            t,
            is_exit_pipeline,
            is_ghost,
            is_executing,
        );

        current_centers.push(resp.card_rect.center().y);

        if resp.remove_clicked {
            remove_idx = Some(idx);
        }
        if resp.clone_clicked {
            clone_idx = Some(idx);
        }
        if let Some(lang) = resp.expand_script {
            state.open_script_editor(idx, is_exit_pipeline, lang);
        }
        if resp.drag_started {
            drag_started_idx = Some(idx);
        }
        if resp.switch_to_hunter {
            switch_to_hunter_idx = Some(idx);
        }
    }

    // Render trailing drop slot (after last card)
    if total > 0 {
        render_drop_slot(ui, loc, total, active_dst, active_src, pipeline_id, is_exit_pipeline, t);
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
                    let item = actions.remove(src);
                    actions.insert(dst, item);
                } else if dst > src + 1 {
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
            let cloned = actions[idx].clone();
            actions.insert(idx + 1, cloned);
        }
    }
    if let Some(idx) = remove_idx {
        if idx < actions.len() {
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
            is_exit_pipeline,
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
    pub drag_started: bool,
    pub switch_to_hunter: bool,
    pub card_rect: Rect,
}

fn render_action_card(
    ui: &mut Ui,
    state: &GuiState,
    action: &mut Action,
    idx: usize,
    t: &Tokens,
    is_exit_pipeline: bool,
    is_being_dragged: bool,
    is_executing: bool,
) -> ActionCardResponse {
    let loc = state.loc();
    let mut remove_clicked = false;
    let mut clone_clicked = false;
    let mut expand_script = None;
    let mut drag_started = false;
    let mut switch_to_hunter = false;

    let ghost_id = ui.id().with(("card_ghost", is_exit_pipeline, idx));
    let ghost_anim = ui.ctx().animate_bool_with_time_and_easing(ghost_id, is_being_dragged, 0.15, easing::cubic_out);

    let (fill, stroke) = if is_executing {
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

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            // Drag grip handle on far left
                            let grip_resp = widgets::drag_handle(ui, loc.inspector_tip_drag_step);
                            if grip_resp.drag_started() {
                                drag_started = true;
                            }

                            let prefix = if is_exit_pipeline { loc.inspector_exit_prefix } else { loc.inspector_step_prefix };
                            widgets::hud(ui, &format!("{} {:02}", prefix, idx + 1), t.accent_ink);

                            let type_badge = action_type_badge(action, loc);
                            if is_executing {
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

                // 2. CARD BODY: Full-width stacked parameter fields
                ui.add_enabled_ui(!is_being_dragged, |ui| {
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
                                    switch_to_hunter = true;
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

                    Action::PowerShell { script, run_in_window } => {
                        ui.horizontal(|ui| {
                            widgets::hud(ui, loc.inspector_field_powershell_script, t.muted);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Expand, loc.inspector_tip_expand_pwsh).clicked() {
                                    expand_script = Some(ScriptLanguage::PowerShell);
                                }
                            });
                        });
                        let mut layouter = create_script_layouter(ScriptLanguage::PowerShell, *t, 12.0);
                        ui.add(
                            egui::TextEdit::multiline(script)
                                .desired_width(ui.available_width())
                                .desired_rows(3)
                                .font(egui::FontId::new(12.0, theme::mono()))
                                .margin(vec2(10.0, 8.0))
                                .layouter(&mut layouter),
                        );

                        ui.checkbox(run_in_window, loc.inspector_checkbox_visible_console);
                    }

                    Action::Cmd { command, run_in_window } => {
                        ui.horizontal(|ui| {
                            widgets::hud(ui, loc.inspector_field_cmd_prompt, t.muted);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Expand, loc.inspector_tip_expand_cmd).clicked() {
                                    expand_script = Some(ScriptLanguage::Batch);
                                }
                            });
                        });
                        let mut layouter = create_script_layouter(ScriptLanguage::Batch, *t, 12.0);
                        ui.add(
                            egui::TextEdit::multiline(command)
                                .desired_width(ui.available_width())
                                .desired_rows(2)
                                .font(egui::FontId::new(12.0, theme::mono()))
                                .margin(vec2(10.0, 8.0))
                                .layouter(&mut layouter),
                        );
                        ui.checkbox(run_in_window, loc.inspector_checkbox_visible_console);
                    }

                    Action::Notification { title, message } => {
                        widgets::hud(ui, loc.inspector_field_notif_title, t.muted);
                        ui.add(
                            egui::TextEdit::singleline(title)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );

                        widgets::hud(ui, loc.inspector_field_notif_message, t.muted);
                        ui.add(
                            egui::TextEdit::singleline(message)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );
                    }

                    Action::Sound { sound_type, volume_level } => {
                        widgets::hud(ui, loc.inspector_field_audio_action_type, t.muted);
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(sound_type, SoundActionType::PlayBeep, loc.inspector_audio_beep);
                            ui.radio_value(sound_type, SoundActionType::Mute, loc.inspector_audio_mute);
                            ui.radio_value(sound_type, SoundActionType::Unmute, loc.inspector_audio_unmute);
                            ui.radio_value(sound_type, SoundActionType::ToggleMute, loc.inspector_audio_toggle_mute);
                            ui.radio_value(sound_type, SoundActionType::SetVolume, loc.inspector_audio_set_volume);
                        });
                        if *sound_type == SoundActionType::SetVolume {
                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, loc.inspector_field_volume_level, t.muted);
                                ui.add(egui::Slider::new(volume_level, 0..=100).text("%"));
                            });
                        }
                    }

                    Action::Microphone { action_type, volume_level } => {
                        widgets::hud(ui, loc.inspector_field_mic_action_type, t.muted);
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(action_type, MicrophoneActionType::Mute, loc.inspector_audio_mute);
                            ui.radio_value(action_type, MicrophoneActionType::Unmute, loc.inspector_audio_unmute);
                            ui.radio_value(action_type, MicrophoneActionType::ToggleMute, loc.inspector_audio_toggle_mute);
                            ui.radio_value(action_type, MicrophoneActionType::SetVolume, loc.inspector_audio_set_volume);
                        });
                        if *action_type == MicrophoneActionType::SetVolume {
                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, loc.inspector_field_volume_level, t.muted);
                                ui.add(egui::Slider::new(volume_level, 0..=100).text("%"));
                            });
                        }
                    }

                    Action::ProcessAudio {
                        process_name,
                        action_type,
                        volume_level,
                    } => {
                        widgets::hud(ui, loc.inspector_field_target_process, t.muted);
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if render_process_picker(ui, loc) {
                                    switch_to_hunter = true;
                                }
                                ui.add(
                                    egui::TextEdit::singleline(process_name)
                                        .hint_text("e.g. notepad.exe or game.exe")
                                        .desired_width(ui.available_width())
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                );
                            });
                        });

                        widgets::hud(ui, loc.inspector_field_process_audio_action, t.muted);
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(action_type, ProcessAudioActionType::Mute, loc.inspector_audio_mute);
                            ui.radio_value(action_type, ProcessAudioActionType::Unmute, loc.inspector_audio_unmute);
                            ui.radio_value(action_type, ProcessAudioActionType::ToggleMute, loc.inspector_audio_toggle_mute);
                            ui.radio_value(action_type, ProcessAudioActionType::SetVolume, loc.inspector_audio_set_volume);
                        });
                        if *action_type == ProcessAudioActionType::SetVolume {
                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, loc.inspector_field_volume_level, t.muted);
                                ui.add(egui::Slider::new(volume_level, 0..=100).text("%"));
                            });
                        }
                    }

                    Action::SystemPower { power_type } => {
                        widgets::hud(ui, loc.inspector_field_power_action, t.muted);
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(power_type, SystemPowerType::Lock, loc.inspector_power_lock);
                            ui.radio_value(power_type, SystemPowerType::Sleep, loc.inspector_power_sleep);
                            ui.radio_value(power_type, SystemPowerType::TurnScreenOff, loc.inspector_power_screen_off);
                            ui.radio_value(power_type, SystemPowerType::Restart, loc.inspector_power_restart);
                            ui.radio_value(power_type, SystemPowerType::Shutdown, loc.inspector_power_shutdown);
                        });
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
                                .desired_width(ui.available_width())
                                .desired_rows(2)
                                .font(egui::FontId::new(12.0, theme::mono()))
                                .margin(vec2(10.0, 8.0)),
                        ).changed() {
                            *payload = if p_str.trim().is_empty() { None } else { Some(p_str) };
                        }
                    }
                }
            });
            });
        });

    ActionCardResponse {
        remove_clicked,
        clone_clicked,
        expand_script,
        drag_started,
        switch_to_hunter,
        card_rect: card_response.response.rect,
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
            top: 20,
            bottom: 20,
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
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;

        ui.horizontal_wrapped(|ui| {
            let modes = [
                (ScheduleType::Interval, loc.inspector_schedule_interval),
                (ScheduleType::Daily, loc.inspector_schedule_daily),
                (ScheduleType::OneShot, loc.inspector_schedule_oneshot),
            ];
            for (m, label) in modes {
                let prev = *schedule_type;
                if ui.radio_value(schedule_type, m, label).clicked()
                    && *schedule_type != prev
                    && *schedule_type == ScheduleType::Interval
                {
                    *target_datetime = None;
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            let modes = [
                (ScheduleType::AppStartup, loc.inspector_schedule_app_start),
                (ScheduleType::AppShutdown, loc.inspector_schedule_app_close),
            ];
            for (m, label) in modes {
                let prev = *schedule_type;
                if ui.radio_value(schedule_type, m, label).clicked() && *schedule_type != prev {
                    *target_datetime = None;
                }
            }
        });
    });

    ui.add_space(4.0);

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

        ScheduleType::AppStartup => {
            widgets::badge(
                ui,
                loc.inspector_badge_app_startup,
                t.good,
            );
        }

        ScheduleType::AppShutdown => {
            widgets::badge(
                ui,
                loc.inspector_badge_app_shutdown,
                t.muted,
            );
        }
    }
}
