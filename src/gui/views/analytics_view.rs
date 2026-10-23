//! Activity Log Stream & Analytics View: High-density monospace telemetry terminal
//! with 3 core KPI metrics (Total Executions, Active Rules, System Uptime),
//! category filter cells, instant query filtering, level badges, and hairline rules.

use egui::{CornerRadius, Margin, Ui, pos2, vec2};
use crate::gui::components::widgets::{self, Cell, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum LogFilter {
    #[default]
    All,
    Rules,
    Processes,
    Daemon,
    Errors,
}

pub fn render_analytics_view(ui: &mut Ui, state: &mut GuiState) {
    let t = Tokens::get(ui.ctx());
    let loc = state.loc();

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;

        let daemon = state.daemon_state.as_ref();
        let total_executions = daemon.map(|s| s.analytics.total_executions).unwrap_or(0);
        let running_rules = daemon.map(|s| s.active_rule_executions.len()).unwrap_or(0);
        let uptime_secs = daemon.map(|s| s.uptime_secs).unwrap_or(0);

        let logs = state
            .daemon_state
            .as_ref()
            .map(|s| s.logs.clone())
            .unwrap_or_default();

        let total = logs.len().min(500);
        let error_count = logs.iter().take(500).filter(|l| l.level == "ERROR").count();

        // 1. Toolbar with Filter Cells, Search Box, & Clear Action
        ui.horizontal(|ui| {
            let mut filter = match ui.data(|d| d.get_temp::<LogFilter>(egui::Id::new("log_filter"))) {
                Some(f) => f,
                None => LogFilter::All,
            };

            let options = [
                (
                    LogFilter::All,
                    Cell {
                        label: loc.logs_cell_all,
                        tip: loc.logs_cell_all_tip,
                        count: Some((total, None)),
                    },
                ),
                (
                    LogFilter::Rules,
                    Cell {
                        label: loc.logs_cell_rules,
                        tip: loc.logs_cell_rules_tip,
                        count: None,
                    },
                ),
                (
                    LogFilter::Processes,
                    Cell {
                        label: loc.logs_cell_processes,
                        tip: loc.logs_cell_processes_tip,
                        count: None,
                    },
                ),
                (
                    LogFilter::Daemon,
                    Cell {
                        label: loc.logs_cell_daemon,
                        tip: loc.logs_cell_daemon_tip,
                        count: None,
                    },
                ),
                (
                    LogFilter::Errors,
                    Cell {
                        label: loc.logs_cell_errors,
                        tip: loc.logs_cell_errors_tip,
                        count: Some((error_count, if error_count > 0 { Some(t.bad) } else { None })),
                    },
                ),
            ];

            if widgets::cells(ui, &mut filter, &options) {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("log_filter"), filter));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Clear Logs Action
                if widgets::button(ui, loc.logs_btn_clear, Kind::Secondary, 0.0).clicked() {
                    let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ClearLogs);
                    if let Some(ds) = &mut state.daemon_state {
                        ds.logs.clear();
                    }
                    state.set_toast(loc.toast_stream_cleared);
                }

                ui.add_space(8.0);

                // Search edit
                ui.add(
                    egui::TextEdit::singleline(&mut state.analytics_log_filter)
                        .hint_text(loc.logs_search_placeholder)
                        .desired_width(240.0)
                        .min_size(vec2(0.0, theme::CONTROL_H))
                        .vertical_align(egui::Align::Center)
                        .margin(vec2(10.0, 8.0)),
                );
            });
        });

        // 3. Filter Logs
        let filter = ui.data(|d| d.get_temp::<LogFilter>(egui::Id::new("log_filter"))).unwrap_or_default();
        let needle = state.analytics_log_filter.trim().to_lowercase();
        let filtered: Vec<_> = logs
            .into_iter()
            .take(500)
            .filter(|l| match filter {
                LogFilter::All => true,
                LogFilter::Rules => l.category.eq_ignore_ascii_case("RULE"),
                LogFilter::Processes => l.category.eq_ignore_ascii_case("PROCESS"),
                LogFilter::Daemon => {
                    !l.category.eq_ignore_ascii_case("RULE")
                        && !l.category.eq_ignore_ascii_case("PROCESS")
                        && !l.level.eq_ignore_ascii_case("ERROR")
                }
                LogFilter::Errors => l.level.eq_ignore_ascii_case("ERROR"),
            })
            .filter(|l| {
                if needle.is_empty() {
                    true
                } else {
                    l.message.to_lowercase().contains(&needle)
                        || l.category.to_lowercase().contains(&needle)
                        || l.level.to_lowercase().contains(&needle)
                }
            })
            .collect();

        let kpi_h = 56.0;
        let spacing_y = ui.spacing().item_spacing.y;
        let frame_margins_y = 22.0; // 10px top + 10px bottom inner_margin + 2px stroke
        let bottom_padding = 10.0; // clean breathing room above CentralPanel bottom
        let logs_inner_h = (ui.available_height() - kpi_h - spacing_y - frame_margins_y - bottom_padding).max(80.0);

        if filtered.is_empty() {
            egui::Frame::new()
                .fill(t.sunken)
                .stroke(t.hairline())
                .corner_radius(CornerRadius::ZERO)
                .inner_margin(Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(logs_inner_h);
                    ui.vertical_centered(|ui| {
                        ui.add_space((logs_inner_h / 2.0 - 24.0).max(20.0));
                        widgets::hud(ui, loc.logs_empty_title, t.muted);
                        ui.add_space(6.0);
                        widgets::hud(ui, loc.logs_empty_subtitle, t.faint);
                    });
                });
        } else {
            // 2. Monospace Terminal Well (Top to Bottom Chronological Stream)
            egui::Frame::new()
                .fill(t.sunken)
                .stroke(t.hairline())
                .corner_radius(CornerRadius::ZERO)
                .inner_margin(Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(logs_inner_h);

                    egui::ScrollArea::vertical()
                        .max_height(logs_inner_h)
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;

                            // Display top-to-bottom (chronological)
                            for log in &filtered {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;

                                    // Timestamp
                                    widgets::hud(ui, &log.timestamp, t.faint);

                                    // Level Badge
                                    let level_color = match log.level.as_str() {
                                        "ERROR" => t.bad,
                                        "WARN" => t.warn,
                                        "TRIGGER" => t.accent_ink,
                                        _ => t.good,
                                    };
                                    widgets::badge(ui, &log.level, level_color);

                                    // Category Badge
                                    let cat_color = match log.category.as_str() {
                                        "PROCESS" => t.accent_ink,
                                        "RULE" => t.warn,
                                        _ => t.muted,
                                    };
                                    widgets::badge(ui, &log.category, cat_color);

                                    // Message
                                    let msg_color = if log.level == "ERROR" {
                                        t.bad
                                    } else {
                                        t.ink
                                    };
                                    ui.label(
                                        egui::RichText::new(&log.message)
                                            .family(theme::mono())
                                            .size(11.5)
                                            .color(msg_color),
                                    );
                                });
                            }
                        });
                });
        }

        // 3. BOTTOM METRIC STRIP (3 Minimalist KPI Cards: Total, Active, Uptime)
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            let total_avail = ui.available_width();
            let card_w = ((total_avail - spacing * 2.0) / 3.0).floor().max(100.0);

            render_kpi_card(
                ui,
                loc.analytics_kpi_total_executions,
                &total_executions.to_string(),
                t.accent_ink,
                card_w,
                kpi_h,
                &t,
            );
            render_kpi_card(
                ui,
                loc.analytics_kpi_active_rules,
                &running_rules.to_string(),
                t.good,
                card_w,
                kpi_h,
                &t,
            );
            let last_card_w = ui.available_width().max(100.0);
            render_kpi_card(
                ui,
                loc.analytics_kpi_uptime,
                &format_uptime(uptime_secs),
                t.ink_strong,
                last_card_w,
                kpi_h,
                &t,
            );
        });
    });
}

fn render_kpi_card(
    ui: &mut Ui,
    title: &str,
    value: &str,
    color: egui::Color32,
    width: f32,
    height: f32,
    t: &Tokens,
) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::ZERO, t.sunken);
    p.rect_stroke(rect, CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);

    // Title HUD label
    p.text(
        pos2(rect.left() + 10.0, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        title.to_uppercase(),
        egui::FontId::new(theme::HUD_SIZE, theme::mono()),
        t.muted,
    );

    // Value Mono text
    p.text(
        pos2(rect.left() + 10.0, rect.bottom() - 10.0),
        egui::Align2::LEFT_BOTTOM,
        value,
        egui::FontId::new(16.0, theme::mono_bold()),
        color,
    );
}

fn format_uptime(uptime_secs: u64) -> String {
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;
    let mins = (uptime_secs % 3600) / 60;
    let secs = uptime_secs % 60;
    if days > 0 {
        format!("{}d {:02}h {:02}m", days, hours, mins)
    } else if hours > 0 {
        format!("{:02}h {:02}m {:02}s", hours, mins, secs)
    } else {
        format!("{:02}m {:02}s", mins, secs)
    }
}
