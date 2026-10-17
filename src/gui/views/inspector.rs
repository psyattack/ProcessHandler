//! Inspector Drawer: Slides in from the right edge with cubic easing,
//! dimmed background scrim, and craft-launcher styling for editing rules
//! and process details.

use egui::{
    CornerRadius, Id, Order, Rect, Sense, Stroke, Ui, emath::easing, pos2, vec2,
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

            let mut remove_action_idx = None;
            for (idx, action) in rule.actions.iter_mut().enumerate() {
                if render_action_card(ui, state, action, idx, t, false) {
                    remove_action_idx = Some(idx);
                }
            }

            if let Some(idx) = remove_action_idx {
                rule.actions.remove(idx);
            }

            // Add action step selector
            render_add_action_button(ui, &mut rule.actions, "add_action_drawer", "+ ADD ACTION...");

            // 3. Actions on Process Exit
            if is_process_trigger {
                widgets::hrule(ui);
                widgets::eyebrow(ui, "04", "ACTIONS ON PROCESS TERMINATION");

                let mut remove_exit_idx = None;
                for (idx, action) in rule.on_exit_actions.iter_mut().enumerate() {
                    if render_action_card(ui, state, action, idx, t, true) {
                        remove_exit_idx = Some(idx);
                    }
                }

                if let Some(idx) = remove_exit_idx {
                    rule.on_exit_actions.remove(idx);
                }

                render_add_action_button(ui, &mut rule.on_exit_actions, "add_exit_action_drawer", "+ ADD ACTION...");
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

fn render_action_card(
    ui: &mut Ui,
    state: &GuiState,
    action: &mut Action,
    idx: usize,
    t: &Tokens,
    is_exit_pipeline: bool,
) -> bool {
    let mut remove_clicked = false;

    egui::Frame::new()
        .fill(t.sunken)
        .stroke(t.hairline())
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
                    let prefix = if is_exit_pipeline { "EXIT" } else { "STEP" };
                    widgets::hud(ui, &format!("{} {:02}", prefix, idx + 1), t.accent_ink);

                    let type_badge = match action {
                        Action::StartProcess { close_on_exit, .. } => {
                            if *close_on_exit { "LAUNCH COMPANION" } else { "LAUNCH PROCESS" }
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
                    };
                    widgets::badge(ui, type_badge, t.muted);

                    // Close button aligned to right edge without colliding with any content below
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::glyph_button(ui, Glyph::Close, "Remove Step").clicked() {
                            remove_clicked = true;
                        }
                    });
                });

                widgets::hrule(ui);

                // 2. CARD BODY: Full-width stacked parameter fields
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

    remove_clicked
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
