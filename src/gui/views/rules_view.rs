//! Automation rules view in the craft-launcher aesthetic:
//! precision segmented cells, hairline cards, numbered section eyebrows,
//! uppercase mono badges, and inverted action buttons.

use egui::{CornerRadius, pos2, Rect, Ui, vec2};
use crate::gui::components::widgets::{self, Cell, Glyph, Kind};
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
    let loc = state.loc();

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

        // 1. Toolbar with Filter Cells, Search, Import/Export & New Rule Button
        ui.horizontal(|ui| {
            let mut filter = match ui.data(|d| d.get_temp::<RuleFilter>(egui::Id::new("rule_filter"))) {
                Some(f) => f,
                None => RuleFilter::All,
            };

            let options = [
                (
                    RuleFilter::All,
                    Cell {
                        label: loc.rules_cell_all,
                        tip: loc.rules_cell_all_tip,
                        count: Some((total, None)),
                    },
                ),
                (
                    RuleFilter::ActiveOnly,
                    Cell {
                        label: loc.rules_cell_active,
                        tip: loc.rules_cell_active_tip,
                        count: Some((active_count, Some(t.good))),
                    },
                ),
                (
                    RuleFilter::PausedOnly,
                    Cell {
                        label: loc.rules_cell_paused,
                        tip: loc.rules_cell_paused_tip,
                        count: Some((paused_count, None)),
                    },
                ),
            ];

            if widgets::cells(ui, &mut filter, &options) {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("rule_filter"), filter));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Primary "+ NEW AUTOMATION" Button
                if widgets::button(ui, loc.rules_btn_new, Kind::Primary, 0.0).clicked() {
                    state.selected_rule_id = None;
                    state.is_creating_new_rule = true;
                    state.editing_rule = Some(Rule {
                        id: format!("rule_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                        name: loc.rules_default_new_name.into(),
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
                        custom_icon_path: None,
                        auto_process_icon: false,
                    });
                }

                ui.add_space(4.0);

                // Export All Rules Button
                if widgets::button(ui, loc.rules_btn_copy_all, Kind::Secondary, 0.0)
                    .on_hover_text(loc.rules_btn_copy_all_tip)
                    .clicked()
                {
                    state.copy_all_rules_to_clipboard(ui.ctx());
                }

                ui.add_space(4.0);

                // Import Automations Button
                if widgets::button(ui, loc.rules_btn_import, Kind::Secondary, 0.0)
                    .on_hover_text(loc.rules_btn_import_tip)
                    .clicked()
                {
                    state.import_automations_from_clipboard(ui.ctx());
                }

                ui.add_space(8.0);

                // Search box
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text(loc.rules_search_placeholder)
                        .desired_width(200.0)
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
                widgets::hud(ui, loc.rules_empty_title, t.muted);
                ui.add_space(12.0);
                if widgets::button(ui, loc.rules_btn_create_first, Kind::Secondary, 0.0).clicked() {
                    state.selected_rule_id = None;
                    state.is_creating_new_rule = true;
                    state.editing_rule = Some(Rule {
                        id: format!("rule_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                        name: loc.rules_default_new_name.into(),
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
                        custom_icon_path: None,
                        auto_process_icon: false,
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
    let loc = state.loc();
    let frame = egui::Frame::new()
        .fill(t.bg)
        .stroke(t.hairline())
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(16, 14));

    let exec_opt = state.daemon_state.as_ref().and_then(|s| s.active_rule_executions.get(&rule.id)).copied();

    frame.show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            // 1. Top strip: Eyebrow + Status Badge + Cooldown + Execution badge + Actions
            ui.horizontal(|ui| {
                widgets::eyebrow(ui, &format!("{:02}", idx + 1), loc.rules_badge_automation);
                ui.add_space(8.0);
                widgets::badge(
                    ui,
                    if rule.is_enabled { loc.rules_badge_active } else { loc.rules_badge_paused },
                    if rule.is_enabled { t.good } else { t.muted },
                );
                ui.add_space(6.0);
                widgets::badge(ui, &format!("{:.1}{}", rule.cooldown_seconds, loc.rules_badge_cooldown_suffix), t.accent_ink);

                if let Some(exec) = exec_opt {
                    ui.add_space(6.0);
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
                    ui.spacing_mut().item_spacing.x = 6.0;

                    let anim_id = egui::Id::new(("rule_card_actions_expanded", &rule.id));
                    let mut is_expanded = ui.data(|d| d.get_temp::<bool>(anim_id)).unwrap_or(false);

                    // 1. Delete button (always on far right)
                    if widgets::button(ui, loc.rules_btn_delete, Kind::Danger, 0.0)
                        .on_hover_text(loc.rules_btn_delete_tip)
                        .clicked()
                    {
                        state.delete_rule(&rule.id);
                    }

                    // 2. Toggle Pause / Resume (always visible, directly to the left of Delete)
                    let toggle_label = if rule.is_enabled { loc.rules_btn_pause } else { loc.rules_btn_resume };
                    if widgets::button(ui, toggle_label, Kind::Secondary, 0.0).clicked() {
                        state.toggle_rule(&rule.id, !rule.is_enabled);
                    }

                    // 3. More icon button (located to the left of Pause / Resume)
                    if widgets::glyph_button_active(ui, Glyph::More, is_expanded, loc.rules_btn_more_tip).clicked() {
                        is_expanded = !is_expanded;
                        ui.data_mut(|d| d.insert_temp(anim_id, is_expanded));
                    }

                    // 4. Smooth horizontal expansion to the left of More icon button
                    let k = ui.ctx().animate_bool_with_time_and_easing(anim_id, is_expanded, 0.20, egui::emath::easing::cubic_out);
                    if k > 0.001 {
                        let btn_spacing = 6.0;
                        let w_edit = widgets::button_width(ui, loc.rules_btn_edit, 0.0);
                        let w_clone = widgets::button_width(ui, loc.rules_btn_clone, 0.0);
                        let w_test = widgets::button_width(ui, loc.rules_btn_test_run, 0.0);
                        let w_share = widgets::button_width(ui, loc.rules_btn_copy_single, 0.0);
                        let full_w = w_edit + w_clone + w_test + w_share + 3.0 * btn_spacing;
                        let anim_w = (full_w * k).min(full_w);

                        ui.allocate_ui_with_layout(
                            vec2(anim_w, theme::CONTROL_H),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.set_clip_rect(ui.max_rect());
                                ui.spacing_mut().item_spacing.x = btn_spacing;

                                // In right-to-left, Edit is closest to More button:
                                if widgets::button(ui, loc.rules_btn_edit, Kind::Secondary, 0.0)
                                    .on_hover_text(loc.rules_btn_edit_tip)
                                    .clicked()
                                {
                                    state.selected_rule_id = Some(rule.id.clone());
                                    state.editing_rule = Some(rule.clone());
                                    state.is_creating_new_rule = false;
                                }

                                // Clone
                                if widgets::button(ui, loc.rules_btn_clone, Kind::Secondary, 0.0)
                                    .on_hover_text(loc.rules_btn_clone_tip)
                                    .clicked()
                                {
                                    state.clone_rule(rule);
                                }

                                // Test Run
                                if widgets::button(ui, loc.rules_btn_test_run, Kind::Secondary, 0.0)
                                    .on_hover_text(loc.rules_btn_test_run_tip)
                                    .clicked()
                                {
                                    state.test_run_rule(&rule.id);
                                }

                                // Share / Copy Single Rule
                                if widgets::button(ui, loc.rules_btn_copy_single, Kind::Secondary, 0.0)
                                    .on_hover_text(loc.rules_btn_copy_single_tip)
                                    .clicked()
                                {
                                    state.copy_single_rule_to_clipboard(ui.ctx(), rule);
                                }
                            },
                        );
                    }
                });
            });

            // 2. Title row with Rule Icon
            let has_desc = !rule.description.is_empty();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;

                // Rule Icon Box (28x28)
                let icon_size = vec2(28.0, 28.0);
                let (icon_rect, icon_resp) = ui.allocate_exact_size(icon_size, egui::Sense::click());
                ui.painter().rect_filled(icon_rect, CornerRadius::ZERO, t.sunken);
                ui.painter().rect_stroke(icon_rect, CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);

                let mut loaded_texture = false;
                if let Some(path) = &rule.custom_icon_path {
                    if let Some(tex) = state.icon_cache.get_icon_from_path(ui.ctx(), path) {
                        let size = tex.size_vec2();
                        let max_dim = size.x.max(size.y).max(1.0);
                        let scale = 20.0 / max_dim;
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
                    ui.painter().text(
                        icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        glyph,
                        egui::FontId::proportional(15.0),
                        t.ink_strong,
                    );
                }

                if icon_resp.on_hover_text(loc.rules_btn_edit_tip).clicked() {
                    state.selected_rule_id = Some(rule.id.clone());
                    state.editing_rule = Some(rule.clone());
                    state.is_creating_new_rule = false;
                }

                if has_desc {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;

                        let title_resp = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&rule.name)
                                    .family(theme::semibold())
                                    .size(15.0)
                                    .color(t.ink_strong),
                            ).sense(egui::Sense::click())
                        );
                        if title_resp.on_hover_text(loc.rules_btn_edit_tip).clicked() {
                            state.selected_rule_id = Some(rule.id.clone());
                            state.editing_rule = Some(rule.clone());
                            state.is_creating_new_rule = false;
                        }

                        ui.label(
                            egui::RichText::new(&rule.description)
                                .family(theme::medium())
                                .size(12.0)
                                .color(t.muted),
                        );
                    });
                } else {
                    let format = egui::TextFormat {
                        font_id: egui::FontId::new(15.0, theme::semibold()),
                        color: t.ink_strong,
                        ..Default::default()
                    };
                    let max_w = ui.available_width().max(60.0);
                    let mut job = egui::text::LayoutJob::single_section(rule.name.clone(), format);
                    job.wrap = egui::text::TextWrapping::truncate_at_width(max_w);
                    let galley = ui.painter().layout_job(job);
                    let (title_rect, title_resp) = ui.allocate_exact_size(
                        vec2(galley.size().x, icon_size.y),
                        egui::Sense::click(),
                    );
                    widgets::paint_left(ui, title_rect, galley, t.ink_strong);
                    let title_resp = title_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if title_resp.on_hover_text(loc.rules_btn_edit_tip).clicked() {
                        state.selected_rule_id = Some(rule.id.clone());
                        state.editing_rule = Some(rule.clone());
                        state.is_creating_new_rule = false;
                    }
                }
            });

            // 3. Pipeline summary with step highlighting
            ui.add_space(2.0);
            render_pipeline_summary(ui, loc, rule, exec_opt, t);
        });
    });
}

fn format_action_brief(action: &Action, loc: &crate::i18n::Strings) -> String {
    match action {
        Action::StartProcess { file_path, .. } => {
            let fname = std::path::Path::new(file_path).file_name().unwrap_or_default().to_string_lossy();
            format!("{} {}", loc.summary_action_launch, fname)
        }
        Action::KillProcess { process_name, .. } => format!("{} {}", loc.summary_action_kill, process_name),
        Action::PowerShell { .. } => "POWERSHELL".to_string(),
        Action::Cmd { .. } => "CMD".to_string(),
        Action::Notification { title, .. } => format!("NOTIFY \"{}\"", title),
        Action::Sound { .. } => "SOUND".to_string(),
        Action::ProcessAudio { process_name, action_type, volume_level } => {
            match action_type {
                ProcessAudioActionType::Mute => format!("{} {}", loc.summary_action_mute, process_name),
                ProcessAudioActionType::Unmute => format!("{} {}", loc.summary_action_unmute, process_name),
                ProcessAudioActionType::ToggleMute => format!("{} {}", loc.summary_action_toggle, process_name),
                ProcessAudioActionType::SetVolume => format!("{} {}% {}", loc.summary_action_vol, volume_level, process_name),
            }
        }
        Action::SystemPower { power_type } => format!("{} {:?}", loc.summary_action_power, power_type),
        Action::Delay { seconds } => format!("{} {:.1}S", loc.summary_action_wait, seconds),
        Action::HttpWebhook { .. } => "WEBHOOK".to_string(),
    }
}

fn render_pipeline_summary(
    ui: &mut Ui,
    loc: &crate::i18n::Strings,
    rule: &Rule,
    exec_opt: Option<crate::ipc::protocol::ActiveRuleExecutionState>,
    t: &Tokens,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);

        let prefix = if loc.rules_summary_trigger_process.starts_with("ТРИГГЕР") { "ТРИГГЕР: " } else { "TRIGGER: " };

        // Trigger Pill
        let trigger_text = match &rule.trigger {
            Some(Trigger::Process { process_name, single_instance, .. }) => {
                let si_suffix = if *single_instance { loc.rules_summary_single_instance } else { "" };
                format!("{}{}{}", loc.rules_summary_trigger_process, process_name, si_suffix)
            }
            Some(Trigger::SystemEvent { event_type, .. }) => {
                format!("{}{:?}", loc.rules_summary_trigger_system, event_type)
            }
            Some(Trigger::Schedule { schedule_type, interval_sec, daily_time, target_datetime, .. }) => {
                match schedule_type {
                    crate::models::ScheduleType::Interval => {
                        let s = *interval_sec;
                        let text = if s >= 31536000 && s % 31536000 == 0 {
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
                        };
                        format!("{}{}", prefix, text)
                    }
                    crate::models::ScheduleType::Daily => {
                        loc.rules_summary_trigger_daily.replace("{}", daily_time.as_deref().unwrap_or("12:00"))
                    }
                    crate::models::ScheduleType::OneShot => {
                        loc.rules_summary_trigger_once.replace("{}", target_datetime.as_deref().unwrap_or("SPECIFIED TIME"))
                    }
                    crate::models::ScheduleType::AppStartup => loc.rules_summary_trigger_startup.to_string(),
                    crate::models::ScheduleType::AppShutdown => loc.rules_summary_trigger_shutdown.to_string(),
                }
            }
            None => loc.rules_summary_trigger_none.to_string(),
        };

        widgets::badge(ui, &trigger_text, t.faint);

        widgets::hud(ui, "➔", t.faint);

        // Actions Steps with Active Highlight
        if rule.actions.is_empty() {
            widgets::badge(ui, loc.rules_summary_no_actions, t.muted);
        } else {
            for (a_idx, action) in rule.actions.iter().enumerate() {
                if a_idx > 0 {
                    widgets::hud(ui, "→", t.faint);
                }
                let is_current = exec_opt.map_or(false, |e| !e.is_exit && e.current_step == a_idx);
                let brief = format_action_brief(action, loc);
                if is_current {
                    let label = format!("▶ [{}/{}] {}", a_idx + 1, rule.actions.len(), brief);
                    widgets::badge(ui, &label, t.accent_ink);
                } else {
                    let label = format!("[{}/{}] {}", a_idx + 1, rule.actions.len(), brief);
                    widgets::badge(ui, &label, t.muted);
                }
            }
        }

        let has_companion = rule.actions.iter().any(|a| matches!(a, Action::StartProcess { close_on_exit: true, .. }));
        if has_companion {
            widgets::badge(ui, loc.rules_summary_bound_satellite, t.good);
        }

        if !rule.on_exit_actions.is_empty() {
            widgets::hud(ui, "➔", t.faint);
            widgets::hud(ui, loc.rules_summary_on_exit, t.accent_ink);
            for (e_idx, action) in rule.on_exit_actions.iter().enumerate() {
                if e_idx > 0 {
                    widgets::hud(ui, "→", t.faint);
                }
                let is_current = exec_opt.map_or(false, |e| e.is_exit && e.current_step == e_idx);
                let brief = format_action_brief(action, loc);
                if is_current {
                    let label = format!("▶ [{}/{}] {}", e_idx + 1, rule.on_exit_actions.len(), brief);
                    widgets::badge(ui, &label, t.accent_ink);
                } else {
                    let label = format!("[{}/{}] {}", e_idx + 1, rule.on_exit_actions.len(), brief);
                    widgets::badge(ui, &label, t.faint);
                }
            }
        }
    });
}
