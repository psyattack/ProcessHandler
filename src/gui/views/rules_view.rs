//! Automation rules view in the craft-launcher aesthetic:
//! precision segmented cells, hairline cards, numbered section eyebrows,
//! uppercase mono badges, and inverted action buttons.

use egui::{CornerRadius, Ui, vec2};
use crate::gui::components::widgets::{self, Cell, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{Action, ProcessAudioActionType, Rule, Trigger};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum RuleFilter {
    #[default]
    All,
    ActiveOnly,
    PausedOnly,
}

pub fn render_rules_view(ui: &mut Ui, state: &mut GuiState) {
    let t = Tokens::get(ui.ctx());

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;

        let rules = state
            .daemon_state
            .as_ref()
            .map(|s| s.config.rules.clone())
            .unwrap_or_default();

        let total = rules.len();
        let active_count = rules.iter().filter(|r| r.is_enabled).count();
        let paused_count = total.saturating_sub(active_count);

        // 1. Toolbar with Filter Cells & New Rule Button
        ui.horizontal(|ui| {
            let mut filter = match ui.data(|d| d.get_temp::<RuleFilter>(egui::Id::new("rule_filter"))) {
                Some(f) => f,
                None => RuleFilter::All,
            };

            let options = [
                (
                    RuleFilter::All,
                    Cell {
                        label: "ALL RULES",
                        tip: "Show all automation pipelines",
                        count: Some((total, None)),
                    },
                ),
                (
                    RuleFilter::ActiveOnly,
                    Cell {
                        label: "ACTIVE",
                        tip: "Show actively evaluated rules",
                        count: Some((active_count, Some(t.good))),
                    },
                ),
                (
                    RuleFilter::PausedOnly,
                    Cell {
                        label: "PAUSED",
                        tip: "Show disabled rules",
                        count: Some((paused_count, None)),
                    },
                ),
            ];

            if widgets::cells(ui, &mut filter, &options) {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("rule_filter"), filter));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Primary "+ NEW AUTOMATION" Button
                if widgets::button(ui, "+ NEW AUTOMATION", Kind::Primary, 0.0).clicked() {
                    state.selected_rule_id = None;
                    state.is_creating_new_rule = true;
                    state.editing_rule = Some(Rule {
                        id: format!("rule_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                        name: "New Automation Flow".into(),
                        description: String::new(),
                        is_enabled: true,
                        cooldown_seconds: 0.0,
                        trigger: Some(crate::models::Trigger::Process {
                            process_name: "notepad.exe".into(),
                            match_mode: crate::models::ProcessMatchMode::Exact,
                            single_instance: false,
                        }),
                        actions: Vec::new(),
                        on_exit_actions: Vec::new(),
                        execution_count: 0,
                    });
                }

                ui.add_space(8.0);

                // Search box
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("SEARCH AUTOMATIONS...")
                        .desired_width(240.0)
                        .min_size(vec2(0.0, theme::CONTROL_H))
                        .vertical_align(egui::Align::Center)
                        .margin(vec2(10.0, 8.0)),
                );
            });
        });

        // 2. Filter rules
        let filter = ui.data(|d| d.get_temp::<RuleFilter>(egui::Id::new("rule_filter"))).unwrap_or_default();
        let needle = state.search_query.trim().to_lowercase();
        let filtered: Vec<_> = rules
            .into_iter()
            .filter(|r| match filter {
                RuleFilter::All => true,
                RuleFilter::ActiveOnly => r.is_enabled,
                RuleFilter::PausedOnly => !r.is_enabled,
            })
            .filter(|r| {
                if needle.is_empty() {
                    true
                } else {
                    r.name.to_lowercase().contains(&needle)
                        || r.description.to_lowercase().contains(&needle)
                }
            })
            .collect();

        if filtered.is_empty() {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                widgets::hud(ui, "NO AUTOMATION RULES CONFIGURED", t.muted);
                ui.add_space(12.0);
                if widgets::button(ui, "CREATE FIRST RULE", Kind::Secondary, 0.0).clicked() {
                    state.selected_rule_id = None;
                    state.is_creating_new_rule = true;
                    state.editing_rule = Some(Rule {
                        id: format!("rule_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                        name: "New Automation Flow".into(),
                        description: String::new(),
                        is_enabled: true,
                        cooldown_seconds: 0.0,
                        trigger: Some(crate::models::Trigger::Process {
                            process_name: "notepad.exe".into(),
                            match_mode: crate::models::ProcessMatchMode::Exact,
                            single_instance: false,
                        }),
                        actions: Vec::new(),
                        on_exit_actions: Vec::new(),
                        execution_count: 0,
                    });
                }
            });
            return;
        }

        // 3. Rules List
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                for (idx, rule) in filtered.into_iter().enumerate() {
                    let is_selected = state.selected_rule_id.as_deref() == Some(&rule.id);
                    render_rule_card(ui, state, &rule, idx, is_selected, &t);
                }
            });
    });
}

fn render_rule_card(
    ui: &mut Ui,
    state: &mut GuiState,
    rule: &Rule,
    idx: usize,
    _is_selected: bool,
    t: &Tokens,
) {
    let frame = egui::Frame::new()
        .fill(t.bg)
        .stroke(t.hairline())
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(16, 14));

    frame.show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            // Top strip: Eyebrow + Status Badge + Actions
            ui.horizontal(|ui| {
                widgets::eyebrow(ui, &format!("{:02}", idx + 1), "AUTOMATION");
                ui.add_space(8.0);
                widgets::badge(
                    ui,
                    if rule.is_enabled { "ACTIVE" } else { "PAUSED" },
                    if rule.is_enabled { t.good } else { t.muted },
                );
                ui.add_space(6.0);
                widgets::badge(ui, &format!("{:.1}S COOLDOWN", rule.cooldown_seconds), t.accent_ink);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // Delete
                    if widgets::button(ui, "DELETE", Kind::Danger, 0.0)
                        .on_hover_text("Delete automation rule")
                        .clicked()
                    {
                        state.delete_rule(&rule.id);
                    }

                    // Edit
                    if widgets::button(ui, "EDIT", Kind::Secondary, 0.0)
                        .on_hover_text("Edit rule in inspector drawer")
                        .clicked()
                    {
                        state.selected_rule_id = Some(rule.id.clone());
                        state.editing_rule = Some(rule.clone());
                        state.is_creating_new_rule = false;
                    }

                    // Clone
                    if widgets::button(ui, "CLONE", Kind::Secondary, 0.0)
                        .on_hover_text("Duplicate this rule")
                        .clicked()
                    {
                        state.clone_rule(rule);
                    }

                    // Toggle Pause / Enable
                    let toggle_label = if rule.is_enabled { "PAUSE" } else { "RESUME" };
                    if widgets::button(ui, toggle_label, Kind::Secondary, 0.0).clicked() {
                        state.toggle_rule(&rule.id, !rule.is_enabled);
                    }

                    // Test Run
                    if widgets::button(ui, "TEST RUN", Kind::Secondary, 0.0)
                        .on_hover_text("Trigger rule action pipeline immediately")
                        .clicked()
                    {
                        state.test_run_rule(&rule.id);
                    }
                });
            });

            // Rule Title
            ui.label(
                egui::RichText::new(&rule.name)
                    .family(theme::semibold())
                    .size(15.0)
                    .color(t.ink_strong),
            );

            if !rule.description.is_empty() {
                ui.label(
                    egui::RichText::new(&rule.description)
                        .family(theme::medium())
                        .size(12.5)
                        .color(t.muted),
                );
            }

            // Pipeline summary
            ui.add_space(4.0);
            render_pipeline_summary(ui, rule, t);
        });
    });
}

fn render_pipeline_summary(ui: &mut Ui, rule: &Rule, t: &Tokens) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 4.0);

        // Trigger Pill
        let trigger_text = match &rule.trigger {
            Some(Trigger::Process { process_name, single_instance, .. }) => {
                let si_suffix = if *single_instance { " (SINGLE INSTANCE)" } else { "" };
                format!("TRIGGER: PROCESS {}{}", process_name, si_suffix)
            }
            Some(Trigger::SystemEvent { event_type, .. }) => {
                format!("TRIGGER: SYSTEM {:?}", event_type)
            }
            Some(Trigger::Schedule { schedule_type, interval_sec, daily_time, target_datetime, .. }) => {
                match schedule_type {
                    crate::models::ScheduleType::Interval => {
                        let s = *interval_sec;
                        let text = if s >= 31536000 && s % 31536000 == 0 {
                            format!("EVERY {}Y", s / 31536000)
                        } else if s >= 2592000 && s % 2592000 == 0 {
                            format!("EVERY {}MO", s / 2592000)
                        } else if s >= 604800 && s % 604800 == 0 {
                            format!("EVERY {}W", s / 604800)
                        } else if s >= 86400 && s % 86400 == 0 {
                            format!("EVERY {}D", s / 86400)
                        } else if s >= 3600 && s % 3600 == 0 {
                            format!("EVERY {}H", s / 3600)
                        } else if s >= 60 && s % 60 == 0 {
                            format!("EVERY {}M", s / 60)
                        } else {
                            format!("EVERY {}S", s)
                        };
                        format!("TRIGGER: {}", text)
                    }
                    crate::models::ScheduleType::Daily => {
                        format!("TRIGGER: DAILY AT {}", daily_time.as_deref().unwrap_or("12:00"))
                    }
                    crate::models::ScheduleType::OneShot => {
                        format!("TRIGGER: ONCE AT {}", target_datetime.as_deref().unwrap_or("SPECIFIED TIME"))
                    }
                    crate::models::ScheduleType::AppStartup => "TRIGGER: ON APP STARTUP".to_string(),
                    crate::models::ScheduleType::AppShutdown => "TRIGGER: ON APP SHUTDOWN".to_string(),
                }
            }
            None => "TRIGGER: NONE CONFIGURED".to_string(),
        };

        widgets::badge(ui, &trigger_text, t.faint);

        widgets::hud(ui, "➔", t.faint);

        // Actions Count & Preview
        let actions_count = rule.actions.len();
        let actions_summary = if actions_count == 0 {
            "NO ACTIONS".to_string()
        } else {
            let names: Vec<_> = rule.actions.iter().map(|a| match a {
                Action::StartProcess { file_path, .. } => {
                    let fname = std::path::Path::new(file_path).file_name().unwrap_or_default().to_string_lossy();
                    format!("LAUNCH {}", fname)
                }
                Action::KillProcess { process_name, .. } => format!("KILL {}", process_name),
                Action::PowerShell { .. } => "POWERSHELL".to_string(),
                Action::Cmd { .. } => "CMD".to_string(),
                Action::Notification { title, .. } => format!("NOTIFY \"{}\"", title),
                Action::Sound { .. } => "SOUND".to_string(),
                Action::ProcessAudio { process_name, action_type, volume_level } => {
                    match action_type {
                        ProcessAudioActionType::Mute => format!("MUTE {}", process_name),
                        ProcessAudioActionType::Unmute => format!("UNMUTE {}", process_name),
                        ProcessAudioActionType::ToggleMute => format!("TOGGLE {}", process_name),
                        ProcessAudioActionType::SetVolume => format!("VOL {}% {}", volume_level, process_name),
                    }
                }
                Action::SystemPower { power_type } => format!("POWER {:?}", power_type),
                Action::Delay { seconds } => format!("WAIT {:.1}S", seconds),
                Action::HttpWebhook { .. } => "WEBHOOK".to_string(),
            }).collect();
            format!("ACTIONS {}: {}", actions_count, names.join(" | "))
        };
        widgets::badge(ui, &actions_summary, t.muted);

        let has_companion = rule.actions.iter().any(|a| matches!(a, Action::StartProcess { close_on_exit: true, .. }));
        if has_companion {
            widgets::badge(ui, "BOUND SATELLITE", t.good);
        }

        if !rule.on_exit_actions.is_empty() {
            widgets::hud(ui, "➔", t.faint);
            let exit_names: Vec<_> = rule.on_exit_actions.iter().map(|a| match a {
                Action::StartProcess { file_path, .. } => {
                    let fname = std::path::Path::new(file_path).file_name().unwrap_or_default().to_string_lossy();
                    format!("LAUNCH {}", fname)
                }
                Action::KillProcess { process_name, .. } => format!("KILL {}", process_name),
                Action::PowerShell { .. } => "POWERSHELL".to_string(),
                Action::Cmd { .. } => "CMD".to_string(),
                Action::Notification { title, .. } => format!("NOTIFY \"{}\"", title),
                Action::Sound { .. } => "SOUND".to_string(),
                Action::ProcessAudio { process_name, action_type, volume_level } => {
                    match action_type {
                        ProcessAudioActionType::Mute => format!("MUTE {}", process_name),
                        ProcessAudioActionType::Unmute => format!("UNMUTE {}", process_name),
                        ProcessAudioActionType::ToggleMute => format!("TOGGLE {}", process_name),
                        ProcessAudioActionType::SetVolume => format!("VOL {}% {}", volume_level, process_name),
                    }
                }
                Action::SystemPower { power_type } => format!("POWER {:?}", power_type),
                Action::Delay { seconds } => format!("WAIT {:.1}S", seconds),
                Action::HttpWebhook { .. } => "WEBHOOK".to_string(),
            }).collect();
            let exit_summary = format!("ON-EXIT {}: {}", rule.on_exit_actions.len(), exit_names.join(" | "));
            widgets::badge(ui, &exit_summary, t.accent_ink);
        }
    });
}
