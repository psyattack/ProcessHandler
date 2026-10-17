//! Inspector Drawer: Slides in from the right edge with cubic easing,
//! dimmed background scrim, and craft-launcher styling for editing rules
//! and process details.

use egui::{
    CornerRadius, Id, LayerId, Order, Pos2, Rect, Sense, Stroke, Ui, emath::easing, pos2, vec2,
};
use crate::gui::components::widgets::{self, Glyph, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    Action, ProcessAudioActionType, ProcessMatchMode, ProcessSnapshotItem, Rule, ScheduleType, SoundActionType,
    SystemEventType, SystemPowerType, Trigger,
};
use chrono::{Datelike, Timelike};

pub fn render_inspector_drawer(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);

    let is_open = state.editing_rule.is_some() || state.selected_process.is_some();

    // Handle Esc key to close inspector
    if is_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.editing_rule = None;
        state.selected_process = None;
        state.is_creating_new_rule = false;
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
                    state.editing_rule = None;
                    state.selected_process = None;
                    state.is_creating_new_rule = false;
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

            if let Some(rule) = &state.editing_rule {
                let rule_name = rule.name.clone();
                egui::Frame::new()
                    .fill(t.sunken)
                    .stroke(t.hairline())
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(egui::Margin::symmetric(20, 14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_max_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            widgets::eyebrow(ui, "01", "AUTOMATION BUILDER");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Close, "Close").clicked() {
                                    close_editor = true;
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
                        ui.set_width(ui.available_width());
                        ui.set_max_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            widgets::eyebrow(ui, "01", "TASK DETAILS");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Close, "Close").clicked() {
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

            if close_editor {
                state.editing_rule = None;
                state.is_creating_new_rule = false;
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

    egui::Frame::new().inner_margin(egui::Margin::same(20)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_max_width(ui.available_width());
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.set_max_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;

            // Basic Info
            widgets::hud(ui, "RULE NAME:", t.muted);
            ui.add(
                egui::TextEdit::singleline(&mut rule.name)
                    .desired_width(ui.available_width())
                    .min_size(vec2(0.0, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(10.0, 8.0)),
            );

            widgets::hud(ui, "DESCRIPTION:", t.muted);
            ui.add(
                egui::TextEdit::singleline(&mut rule.description)
                    .hint_text("Optional memo, notes, or usage details...")
                    .desired_width(ui.available_width())
                    .min_size(vec2(0.0, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(10.0, 8.0)),
            );

            ui.horizontal_wrapped(|ui| {
                widgets::hud(ui, "COOLDOWN:", t.muted);
                ui.add(egui::DragValue::new(&mut rule.cooldown_seconds).speed(0.1).range(0.0..=60.0).suffix(" s"));
                ui.add_space(16.0);
                ui.checkbox(&mut rule.is_enabled, "Rule Active");
            });

            widgets::hrule(ui);

            // 1. Trigger
            widgets::eyebrow(ui, "02", "TRIGGER CONDITION");

            if rule.trigger.is_none() {
                rule.trigger = Some(Trigger::Process {
                    process_name: "notepad.exe".into(),
                    match_mode: ProcessMatchMode::Exact,
                    single_instance: false,
                });
            }

            let trigger_types = ["Process Event", "System Event", "Periodic Timer"];
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

            if let Some(trig) = &mut rule.trigger {
                match trig {
                    Trigger::Process { process_name, match_mode, single_instance } => {
                        is_process_trigger = true;

                        widgets::hud(ui, "TARGET PROCESS:", t.muted);
                        ui.horizontal(|ui| {
                            let combo_w = 175.0;
                            let input_w = (ui.available_width() - combo_w - ui.spacing().item_spacing.x).max(60.0);
                            ui.add(
                                egui::TextEdit::singleline(process_name)
                                    .hint_text("e.g. game.exe, chrome.exe, *discord*")
                                    .desired_width(input_w)
                                    .min_size(vec2(0.0, theme::CONTROL_H))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 8.0)),
                            );
                            render_process_picker(ui, state, process_name, "rule_proc_combo");
                        });

                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;

                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, "MATCH MODE:", t.muted);
                                ui.radio_value(match_mode, ProcessMatchMode::Exact, "Exact");
                                ui.radio_value(match_mode, ProcessMatchMode::Wildcard, "Wildcard");
                                ui.radio_value(match_mode, ProcessMatchMode::Regex, "Contains");
                            });

                            ui.checkbox(single_instance, "Single Instance");
                        });
                    }
                    Trigger::SystemEvent { event_type, idle_duration_sec, .. } => {
                        widgets::hud(ui, "SYSTEM EVENT TYPE:", t.muted);
                        let events = [
                            (SystemEventType::SessionLock, "Lock Screen"),
                            (SystemEventType::SessionUnlock, "Unlock Screen"),
                            (SystemEventType::SystemSleep, "PC Sleep"),
                            (SystemEventType::SystemResume, "PC Resume from Sleep"),
                            (SystemEventType::PowerSourceAc, "Switched to AC Power"),
                            (SystemEventType::PowerSourceBattery, "Switched to Battery"),
                            (SystemEventType::UserIdle, "User Inactive / Idle"),
                            (SystemEventType::UserActive, "User Active"),
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
                                widgets::hud(ui, "IDLE DURATION:", t.muted);
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

            widgets::hrule(ui);

            // 2. Action Sequence Builder (Execution on trigger)
            let action_header = if is_process_trigger {
                "ACTIONS ON PROCESS START"
            } else {
                "ACTION SEQUENCE PIPELINE"
            };
            widgets::eyebrow(ui, "03", action_header);

            render_actions_pipeline(
                ui,
                state,
                &mut rule.actions,
                "main_actions",
                false,
                t,
            );

            // 3. Actions on Process Exit
            if is_process_trigger {
                widgets::hrule(ui);
                widgets::eyebrow(ui, "04", "ACTIONS ON PROCESS TERMINATION");

                render_actions_pipeline(
                    ui,
                    state,
                    &mut rule.on_exit_actions,
                    "exit_actions",
                    true,
                    t,
                );
            }

            widgets::hrule(ui);

            // Save / Cancel Actions
            ui.horizontal_wrapped(|ui| {
                if widgets::button(ui, "SAVE RULE", Kind::Primary, 120.0).clicked() {
                    state.save_rule(rule.clone());
                }

                if !state.is_creating_new_rule
                    && widgets::button(ui, "CLONE", Kind::Secondary, 0.0)
                        .on_hover_text("Duplicate this rule with a new ID")
                        .clicked()
                {
                    state.clone_rule(rule);
                }

                if widgets::button(ui, "CANCEL", Kind::Secondary, 0.0).clicked() {
                    state.editing_rule = None;
                    state.is_creating_new_rule = false;
                }
            });
        });
    });
}

fn render_add_action_button(ui: &mut Ui, actions: &mut Vec<Action>, combo_id: &str, label: &str) {
    egui::ComboBox::from_id_salt(combo_id)
        .selected_text(label)
        .width(240.0)
        .show_ui(ui, |ui| {
            if ui.button("Launch Process").clicked() {
                actions.push(Action::StartProcess {
                    file_path: String::new(),
                    arguments: String::new(),
                    working_directory: None,
                    run_as_admin: false,
                    only_if_not_running: false,
                    run_hidden: false,
                    close_on_exit: false,
                });
            }
            if ui.button("Kill Process").clicked() {
                actions.push(Action::KillProcess {
                    process_name: "target.exe".into(),
                    kill_tree: true,
                    force: true,
                });
            }
            if ui.button("PowerShell Script").clicked() {
                actions.push(Action::PowerShell {
                    script: "Write-Output 'ProcessHandler'".into(),
                    run_in_window: false,
                });
            }
            if ui.button("Command Prompt").clicked() {
                actions.push(Action::Cmd {
                    command: "echo ProcessHandler".into(),
                    run_in_window: false,
                });
            }
            if ui.button("Notification Toast").clicked() {
                actions.push(Action::Notification {
                    title: "ProcessHandler Alert".into(),
                    message: "Process event triggered".into(),
                });
            }
            if ui.button("Sound / Volume").clicked() {
                actions.push(Action::Sound {
                    sound_type: SoundActionType::PlayBeep,
                    volume_level: 50,
                });
            }
            if ui.button("Process Audio (Mute/Vol)").clicked() {
                actions.push(Action::ProcessAudio {
                    process_name: "notepad.exe".into(),
                    action_type: ProcessAudioActionType::Mute,
                    volume_level: 100,
                });
            }
            if ui.button("System Power / Lock").clicked() {
                actions.push(Action::SystemPower {
                    power_type: SystemPowerType::Lock,
                });
            }
            if ui.button("Delay Pause").clicked() {
                actions.push(Action::Delay { seconds: 1.0 });
            }
            if ui.button("HTTP Webhook").clicked() {
                actions.push(Action::HttpWebhook {
                    url: "http://localhost:8080/hook".into(),
                    method: "POST".into(),
                    headers: None,
                    payload: None,
                });
            }
        });
}

#[derive(Clone, Copy, Debug, Default)]
struct DndDragSession {
    source_idx: usize,
    target_idx: usize,
    card_width: f32,
}

fn action_type_badge(action: &Action) -> &'static str {
    match action {
        Action::StartProcess { close_on_exit, .. } => {
            if *close_on_exit {
                "LAUNCH COMPANION"
            } else {
                "LAUNCH PROCESS"
            }
        }
        Action::KillProcess { .. } => "KILL PROCESS",
        Action::PowerShell { .. } => "POWERSHELL",
        Action::Cmd { .. } => "CMD",
        Action::Notification { .. } => "NOTIFICATION",
        Action::Sound { .. } => "AUDIO / VOLUME",
        Action::ProcessAudio { .. } => "PROCESS AUDIO",
        Action::SystemPower { .. } => "SYSTEM POWER",
        Action::Delay { .. } => "DELAY",
        Action::HttpWebhook { .. } => "WEBHOOK",
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

fn render_drop_slot(
    ui: &mut Ui,
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
        let prefix = if is_exit_pipeline { "EXIT" } else { "STEP" };
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

fn render_floating_drag_preview(
    ui: &Ui,
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
    let prefix = if is_exit_pipeline { "EXIT" } else { "STEP" };
    let step_txt = format!("{} {:02}", prefix, session.source_idx + 1);
    let step_galley = widgets::caps(ui, &step_txt, 11.0, theme::mono_bold(), 1.0, t.accent_ink);
    painter.galley_with_override_text_color(
        pos2(preview_rect.left() + 26.0, preview_rect.top() + 8.0),
        step_galley,
        t.accent_ink,
    );

    // Type badge
    let type_badge = action_type_badge(action);
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
    t: &Tokens,
) {
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
    let mut move_up_idx = None;
    let mut move_down_idx = None;
    let mut drag_started_idx = None;
    let mut current_centers = Vec::with_capacity(actions.len());

    let total = actions.len();
    for (idx, action) in actions.iter_mut().enumerate() {
        // Render drop slot BEFORE card idx
        render_drop_slot(ui, idx, active_dst, active_src, pipeline_id, is_exit_pipeline, t);

        // Render card idx
        let is_ghost = active_src == Some(idx);
        let resp = render_action_card(
            ui,
            state,
            action,
            idx,
            total,
            t,
            is_exit_pipeline,
            is_ghost,
        );

        current_centers.push(resp.card_rect.center().y);

        if resp.remove_clicked {
            remove_idx = Some(idx);
        }
        if resp.move_up_clicked {
            move_up_idx = Some(idx);
        }
        if resp.move_down_clicked {
            move_down_idx = Some(idx);
        }
        if resp.drag_started {
            drag_started_idx = Some(idx);
        }
    }

    // Render trailing drop slot (after last card)
    if total > 0 {
        render_drop_slot(ui, total, active_dst, active_src, pipeline_id, is_exit_pipeline, t);
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

    // Handle button moves
    if let Some(idx) = move_up_idx {
        if idx > 0 && idx < actions.len() {
            actions.swap(idx, idx - 1);
        }
    }
    if let Some(idx) = move_down_idx {
        if idx + 1 < actions.len() {
            actions.swap(idx, idx + 1);
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
        actions,
        &format!("add_action_{}", pipeline_id),
        "+ ADD ACTION...",
    );
}

struct ActionCardResponse {
    pub remove_clicked: bool,
    pub move_up_clicked: bool,
    pub move_down_clicked: bool,
    pub drag_started: bool,
    pub card_rect: Rect,
}

#[allow(clippy::too_many_arguments)]
fn render_action_card(
    ui: &mut Ui,
    state: &GuiState,
    action: &mut Action,
    idx: usize,
    total_count: usize,
    t: &Tokens,
    is_exit_pipeline: bool,
    is_being_dragged: bool,
) -> ActionCardResponse {
    let mut remove_clicked = false;
    let mut move_up_clicked = false;
    let mut move_down_clicked = false;
    let mut drag_started = false;

    let ghost_id = ui.id().with(("card_ghost", is_exit_pipeline, idx));
    let ghost_anim = ui.ctx().animate_bool_with_time_and_easing(ghost_id, is_being_dragged, 0.15, easing::cubic_out);

    let (fill, stroke) = if ghost_anim > 0.001 {
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

                    // Drag grip handle on far left
                    let grip_resp = widgets::drag_handle(ui, "Drag to reorder step");
                    if grip_resp.drag_started() {
                        drag_started = true;
                    }

                    let prefix = if is_exit_pipeline { "EXIT" } else { "STEP" };
                    widgets::hud(ui, &format!("{} {:02}", prefix, idx + 1), t.accent_ink);

                    let type_badge = action_type_badge(action);
                    if is_being_dragged {
                        widgets::badge(ui, "MOVING STEP...", t.accent_ink);
                    } else {
                        widgets::badge(ui, type_badge, t.muted);
                    }

                    // Right-to-left layout for buttons: [Close] [Down] [Up]
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::glyph_button(ui, Glyph::Close, "Remove Step").clicked() {
                            remove_clicked = true;
                        }

                        if total_count > 1 {
                            if idx + 1 < total_count && widgets::glyph_button(ui, Glyph::Down, "Move Step Down").clicked() {
                                move_down_clicked = true;
                            }
                            if idx > 0 && widgets::glyph_button(ui, Glyph::Up, "Move Step Up").clicked() {
                                move_up_clicked = true;
                            }
                        }

                        // Middle blank space on top strip is also draggable
                        let empty_w = (ui.available_width() - 8.0).max(0.0);
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
                        widgets::hud(ui, "EXECUTABLE PATH:", t.muted);
                        ui.horizontal(|ui| {
                            let btn_w = widgets::button_width(ui, "BROWSE...", 0.0);
                            let input_w = (ui.available_width() - btn_w - ui.spacing().item_spacing.x).max(60.0);
                            ui.add(
                                egui::TextEdit::singleline(file_path)
                                    .hint_text("C:\\Path\\To\\Application.exe")
                                    .desired_width(input_w)
                                    .min_size(vec2(0.0, theme::CONTROL_H))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 8.0)),
                            );
                            if widgets::button(ui, "BROWSE...", Kind::Secondary, btn_w).clicked() {
                                if let Some(file) = rfd::FileDialog::new()
                                    .add_filter("Executables", &["exe", "bat", "cmd", "lnk", "ps1"])
                                    .pick_file()
                                {
                                    *file_path = file.to_string_lossy().to_string();
                                }
                            }
                        });

                        widgets::hud(ui, "ARGUMENTS:", t.muted);
                        ui.add(
                            egui::TextEdit::singleline(arguments)
                                .hint_text("e.g. -windowed -port 8080 {ProcessId}")
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );

                        widgets::hud(ui, "WORKING DIRECTORY:", t.muted);
                        ui.horizontal(|ui| {
                            let mut wd_str = working_directory.clone().unwrap_or_default();
                            let btn_w = widgets::button_width(ui, "FOLDER...", 0.0);
                            let input_w = (ui.available_width() - btn_w - ui.spacing().item_spacing.x).max(60.0);
                            if ui.add(
                                egui::TextEdit::singleline(&mut wd_str)
                                    .hint_text("Default executable directory")
                                    .desired_width(input_w)
                                    .min_size(vec2(0.0, theme::CONTROL_H))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 8.0)),
                            ).changed() {
                                *working_directory = if wd_str.trim().is_empty() { None } else { Some(wd_str.clone()) };
                            }
                            if widgets::button(ui, "FOLDER...", Kind::Secondary, btn_w).clicked() {
                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                    *working_directory = Some(folder.to_string_lossy().to_string());
                                }
                            }
                        });

                        ui.checkbox(run_hidden, "Run hidden (no console window)");
                        ui.checkbox(only_if_not_running, "Don't launch if process is already running");
                        ui.checkbox(run_as_admin, "Run as Administrator");
                        if !is_exit_pipeline {
                            ui.checkbox(close_on_exit, "Auto-close when tracked process exits");
                        }
                    }

                    Action::KillProcess { process_name, kill_tree, force } => {
                        widgets::hud(ui, "TERMINATE PROCESS NAME:", t.muted);
                        ui.horizontal(|ui| {
                            let combo_w = 175.0;
                            let input_w = (ui.available_width() - combo_w - ui.spacing().item_spacing.x).max(60.0);
                            ui.add(
                                egui::TextEdit::singleline(process_name)
                                    .hint_text("e.g. soundkeeper.exe or target.exe")
                                    .desired_width(input_w)
                                    .min_size(vec2(0.0, theme::CONTROL_H))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 8.0)),
                            );
                            render_process_picker(ui, state, process_name, &format!("kill_proc_{idx}"));
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(kill_tree, "Kill process tree");
                            ui.checkbox(force, "Force terminate");
                        });
                    }

                    Action::PowerShell { script, run_in_window } => {
                        widgets::hud(ui, "POWERSHELL SCRIPT:", t.muted);
                        ui.add(
                            egui::TextEdit::multiline(script)
                                .desired_width(ui.available_width())
                                .desired_rows(3)
                                .font(egui::FontId::new(12.0, theme::mono()))
                                .margin(vec2(10.0, 8.0)),
                        );

                        ui.checkbox(run_in_window, "Run in visible console window");
                    }

                    Action::Cmd { command, run_in_window } => {
                        widgets::hud(ui, "COMMAND LINE PROMPT:", t.muted);
                        ui.add(
                            egui::TextEdit::singleline(command)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );
                        ui.checkbox(run_in_window, "Run in visible console window");
                    }

                    Action::Notification { title, message } => {
                        widgets::hud(ui, "NOTIFICATION TITLE:", t.muted);
                        ui.add(
                            egui::TextEdit::singleline(title)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );

                        widgets::hud(ui, "NOTIFICATION MESSAGE:", t.muted);
                        ui.add(
                            egui::TextEdit::singleline(message)
                                .desired_width(ui.available_width())
                                .min_size(vec2(0.0, theme::CONTROL_H))
                                .vertical_align(egui::Align::Center)
                                .margin(vec2(10.0, 8.0)),
                        );
                    }

                    Action::Sound { sound_type, volume_level } => {
                        widgets::hud(ui, "AUDIO ACTION TYPE:", t.muted);
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(sound_type, SoundActionType::PlayBeep, "System Beep");
                            ui.radio_value(sound_type, SoundActionType::Mute, "Mute");
                            ui.radio_value(sound_type, SoundActionType::Unmute, "Unmute");
                            ui.radio_value(sound_type, SoundActionType::ToggleMute, "Toggle Mute");
                            ui.radio_value(sound_type, SoundActionType::SetVolume, "Set Volume");
                        });
                        if *sound_type == SoundActionType::SetVolume {
                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, "VOLUME LEVEL:", t.muted);
                                ui.add(egui::Slider::new(volume_level, 0..=100).text("%"));
                            });
                        }
                    }

                    Action::ProcessAudio {
                        process_name,
                        action_type,
                        volume_level,
                    } => {
                        widgets::hud(ui, "TARGET PROCESS:", t.muted);
                        ui.horizontal(|ui| {
                            let combo_w = 175.0;
                            let input_w = (ui.available_width() - combo_w - ui.spacing().item_spacing.x).max(60.0);
                            ui.add(
                                egui::TextEdit::singleline(process_name)
                                    .hint_text("e.g. notepad.exe or game.exe")
                                    .desired_width(input_w)
                                    .min_size(vec2(0.0, theme::CONTROL_H))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 8.0)),
                            );
                            render_process_picker(ui, state, process_name, &format!("audio_proc_{idx}"));
                        });

                        widgets::hud(ui, "PROCESS AUDIO ACTION:", t.muted);
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(action_type, ProcessAudioActionType::Mute, "Mute");
                            ui.radio_value(action_type, ProcessAudioActionType::Unmute, "Unmute");
                            ui.radio_value(action_type, ProcessAudioActionType::ToggleMute, "Toggle Mute");
                            ui.radio_value(action_type, ProcessAudioActionType::SetVolume, "Set Volume");
                        });
                        if *action_type == ProcessAudioActionType::SetVolume {
                            ui.horizontal_wrapped(|ui| {
                                widgets::hud(ui, "VOLUME LEVEL:", t.muted);
                                ui.add(egui::Slider::new(volume_level, 0..=100).text("%"));
                            });
                        }
                    }

                    Action::SystemPower { power_type } => {
                        widgets::hud(ui, "POWER ACTION:", t.muted);
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(power_type, SystemPowerType::Lock, "Lock Screen");
                            ui.radio_value(power_type, SystemPowerType::Sleep, "Sleep PC");
                            ui.radio_value(power_type, SystemPowerType::TurnScreenOff, "Turn Screen Off");
                            ui.radio_value(power_type, SystemPowerType::Restart, "Restart");
                            ui.radio_value(power_type, SystemPowerType::Shutdown, "Shutdown");
                        });
                    }

                    Action::Delay { seconds } => {
                        ui.horizontal_wrapped(|ui| {
                            widgets::hud(ui, "PAUSE DELAY:", t.muted);
                            ui.add(egui::DragValue::new(seconds).speed(0.1).range(0.1..=120.0).suffix(" s"));
                        });
                    }

                    Action::HttpWebhook { url, method, payload, .. } => {
                        widgets::hud(ui, "WEBHOOK URL:", t.muted);
                        ui.horizontal(|ui| {
                            let combo_w = 85.0;
                            let input_w = (ui.available_width() - combo_w - ui.spacing().item_spacing.x).max(60.0);
                            ui.add(
                                egui::TextEdit::singleline(url)
                                    .hint_text("https://api.example.com/webhook")
                                    .desired_width(input_w)
                                    .min_size(vec2(0.0, theme::CONTROL_H))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 8.0)),
                            );
                            let combo_id = ui.id().with(idx).with("meth_combo");
                            egui::ComboBox::from_id_salt(combo_id)
                                .selected_text(method.as_str())
                                .width(65.0)
                                .show_ui(ui, |ui| {
                                    for m in &["POST", "GET", "PUT", "DELETE"] {
                                        ui.selectable_value(method, m.to_string(), *m);
                                    }
                                });
                        });

                        widgets::hud(ui, "JSON BODY PAYLOAD:", t.muted);
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
        move_up_clicked,
        move_down_clicked,
        drag_started,
        card_rect: card_response.response.rect,
    }
}

fn render_process_inspector(ui: &mut Ui, state: &mut GuiState, proc: &ProcessSnapshotItem, t: &Tokens) {
    egui::Frame::new().inner_margin(egui::Margin::same(20)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;

            // Window Title
            widgets::hud(ui, "WINDOW TITLE:", t.muted);
            let wtitle = if proc.window_title.is_empty() {
                "No active top-level window"
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
            widgets::hud(ui, "WORKING SET RAM:", t.muted);
            ui.horizontal_wrapped(|ui| {
                widgets::badge(ui, &format!("{:.1} MB", mb), t.good);
                widgets::hud(ui, &format!("{} BYTES", proc.memory_bytes), t.faint);
            });

            widgets::hrule(ui);

            widgets::eyebrow(ui, "02", "1-CLICK AUTOMATION ACTIONS");

            if widgets::button(ui, "+ CREATE AUTOMATION RULE", Kind::Secondary, 0.0).clicked() {
                state.active_view = crate::gui::state::ActiveView::Rules;
                state.is_creating_new_rule = true;
                state.editing_rule = Some(Rule {
                    id: format!("rule_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                    name: format!("Track {}", proc.name),
                    description: String::new(),
                    is_enabled: true,
                    cooldown_seconds: 0.0,
                    trigger: Some(Trigger::Process {
                        process_name: proc.name.clone(),
                        match_mode: ProcessMatchMode::Exact,
                        single_instance: false,
                    }),
                    actions: Vec::new(),
                    on_exit_actions: Vec::new(),
                    execution_count: 0,
                });
            }

            widgets::hrule(ui);

            if widgets::button(ui, "TERMINATE PROCESS", Kind::Danger, 0.0).clicked() {
                let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::KillProcess {
                    pid: proc.pid,
                    force: true,
                });
                state.set_toast(format!("Terminated {} #{}", proc.name, proc.pid));
                state.selected_process = None;
            }
        });
    });
}

fn render_process_picker(
    ui: &mut Ui,
    state: &GuiState,
    target_process: &mut String,
    id_salt: &str,
) {
    let mut names: Vec<String> = state
        .cached_processes
        .iter()
        .map(|p| p.name.clone())
        .filter(|n| !n.trim().is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    names.sort_by_key(|a| a.to_lowercase());

    let filter_id = ui.id().with(id_salt).with("proc_search");
    let mut filter_text: String = ui.data_mut(|d| d.get_temp(filter_id).unwrap_or_default());

    egui::ComboBox::from_id_salt(id_salt)
        .selected_text("Pick running...")
        .width(160.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.set_max_height(320.0);
            ui.set_min_width(240.0);

            ui.horizontal(|ui| {
                ui.label("🔍");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut filter_text)
                        .hint_text("Search process...")
                        .desired_width(180.0),
                );
                if resp.changed() {
                    ui.data_mut(|d| d.insert_temp(filter_id, filter_text.clone()));
                }
            });

            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(260.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let search_lower = filter_text.to_lowercase();
                    let mut count = 0;
                    for name in &names {
                        if !search_lower.is_empty() && !name.to_lowercase().contains(&search_lower) {
                            continue;
                        }
                        count += 1;
                        let is_sel = target_process == name;
                        if ui.selectable_label(is_sel, name).clicked() {
                            *target_process = name.clone();
                            ui.close();
                        }
                    }
                    if count == 0 {
                        ui.label(
                            egui::RichText::new("No matching processes")
                                .italics()
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                });
        });
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

    widgets::hud(ui, "MINI CALENDAR:", t.muted);
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

        let month_name = match view_month {
            1 => "January", 2 => "February", 3 => "March", 4 => "April",
            5 => "May", 6 => "June", 7 => "July", 8 => "August",
            9 => "September", 10 => "October", 11 => "November", 12 => "December",
            _ => "",
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
                for d in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
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
        widgets::hud(ui, "EXECUTION TIME:", t.muted);
        let h_ch = ui.add(
            egui::DragValue::new(&mut sel_hour)
                .range(0..=23)
                .speed(0.1)
                .custom_formatter(|n, _| format!("{:02}", n as u32)),
        ).changed();
        ui.label(":");
        let m_ch = ui.add(
            egui::DragValue::new(&mut sel_min)
                .range(0..=59)
                .speed(0.1)
                .custom_formatter(|n, _| format!("{:02}", n as u32)),
        ).changed();
        ui.label(":");
        let s_ch = ui.add(
            egui::DragValue::new(&mut sel_sec)
                .range(0..=59)
                .speed(0.1)
                .custom_formatter(|n, _| format!("{:02}", n as u32)),
        ).changed();

        if h_ch || m_ch || s_ch {
            *target_datetime = Some(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                sel_year, sel_month, sel_day, sel_hour, sel_min, sel_sec
            ));
        }
    });
}

fn render_schedule_trigger(
    ui: &mut Ui,
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
                (ScheduleType::Interval, "Periodic Interval"),
                (ScheduleType::Daily, "Daily at Time"),
                (ScheduleType::OneShot, "Specific Date & Time"),
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
                (ScheduleType::AppStartup, "On App Start"),
                (ScheduleType::AppShutdown, "On App Close"),
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
            widgets::hud(ui, "QUICK PRESET INTERVALS:", t.muted);
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
                        "1S" => "1 second",
                        "1M" => "1 minute",
                        "1H" => "1 hour",
                        "1D" => "1 day (24 hours)",
                        "1W" => "1 week (7 days)",
                        "1MO" => "1 month (30 days)",
                        "1Y" => "1 year (365 days)",
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
            widgets::hud(ui, "CUSTOM REPEAT INTERVAL:", t.muted);

            let s = *interval_sec;
            let mut hours = s / 3600;
            let mut mins = (s % 3600) / 60;
            let mut secs = s % 60;

            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Hours:").monospace().size(12.0).color(t.ink));
                let h_ch = ui.add(
                    egui::DragValue::new(&mut hours)
                        .range(0..=999_999)
                        .speed(0.2)
                        .suffix(" h"),
                ).changed();

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Minutes:").monospace().size(12.0).color(t.ink));
                let m_ch = ui.add(
                    egui::DragValue::new(&mut mins)
                        .range(0..=59)
                        .speed(0.2)
                        .suffix(" m"),
                ).changed();

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Seconds:").monospace().size(12.0).color(t.ink));
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
            if ui.checkbox(&mut has_start_time, "Specify first execution date and time").changed() {
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
                render_datetime_picker(ui, target_datetime, t, "interval_cal");
            }
        }

        ScheduleType::Daily => {
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

            widgets::hud(ui, "DAILY EXECUTION TIME:", t.muted);
            ui.horizontal_wrapped(|ui| {
                let h_ch = ui.add(
                    egui::DragValue::new(&mut hour)
                        .range(0..=23)
                        .speed(0.1)
                        .custom_formatter(|n, _| format!("{:02}", n as u32)),
                ).changed();
                ui.label(":");
                let m_ch = ui.add(
                    egui::DragValue::new(&mut min)
                        .range(0..=59)
                        .speed(0.1)
                        .custom_formatter(|n, _| format!("{:02}", n as u32)),
                ).changed();
                ui.label(":");
                let s_ch = ui.add(
                    egui::DragValue::new(&mut sec)
                        .range(0..=59)
                        .speed(0.1)
                        .custom_formatter(|n, _| format!("{:02}", n as u32)),
                ).changed();

                if h_ch || m_ch || s_ch {
                    *daily_time = Some(format!("{:02}:{:02}:{:02}", hour, min, sec));
                }
            });

            widgets::hud(ui, "ACTIVE DAYS OF THE WEEK:", t.muted);
            let dow_names = [
                (1, "Mon"),
                (2, "Tue"),
                (3, "Wed"),
                (4, "Thu"),
                (5, "Fri"),
                (6, "Sat"),
                (7, "Sun"),
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
                if widgets::button(ui, "ALL", if all_active { Kind::Primary } else { Kind::Secondary }, 0.0).clicked() {
                    days_of_week.clear();
                }
            });
        }

        ScheduleType::OneShot => {
            render_datetime_picker(ui, target_datetime, t, "oneshot_cal");
        }

        ScheduleType::AppStartup => {
            widgets::badge(
                ui,
                "Fires once automatically whenever ProcessHandler starts",
                t.good,
            );
        }

        ScheduleType::AppShutdown => {
            widgets::badge(
                ui,
                "Fires when ProcessHandler shuts down",
                t.muted,
            );
        }
    }
}
