//! Activity Log Stream View: High-density monospace telemetry terminal
//! with category filter cells, instant query filtering, level badges, and hairline rules.

use egui::{CornerRadius, Ui, vec2};
use crate::gui::components::widgets::{self, Cell, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum LogFilter {
    #[default]
    All,
    Rules,
    Processes,
    System,
    Errors,
}

pub fn render_logs_view(ui: &mut Ui, state: &mut GuiState) {
    let t = Tokens::get(ui.ctx());
    let loc = state.loc();

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;

        let logs = state
            .daemon_state
            .as_ref()
            .map(|s| s.logs.clone())
            .unwrap_or_default();

        let total = logs.len().min(100);
        let error_count = logs.iter().take(100).filter(|l| l.level == "ERROR").count();

        // 1. Toolbar with Filter Cells & Clear Action
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
                    LogFilter::System,
                    Cell {
                        label: loc.logs_cell_system,
                        tip: loc.logs_cell_system_tip,
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
                // Clear button
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
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text(loc.logs_search_placeholder)
                        .desired_width(240.0)
                        .min_size(vec2(0.0, theme::CONTROL_H))
                        .vertical_align(egui::Align::Center)
                        .margin(vec2(10.0, 8.0)),
                );
            });
        });

        // 2. Filter logs
        let filter = ui.data(|d| d.get_temp::<LogFilter>(egui::Id::new("log_filter"))).unwrap_or_default();
        let needle = state.search_query.trim().to_lowercase();
        let filtered: Vec<_> = logs
            .into_iter()
            .take(100)
            .filter(|l| match filter {
                LogFilter::All => true,
                LogFilter::Rules => l.category.eq_ignore_ascii_case("RULE"),
                LogFilter::Processes => l.category.eq_ignore_ascii_case("PROCESS"),
                LogFilter::System => l.category.eq_ignore_ascii_case("SYSTEM"),
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

        if filtered.is_empty() {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                widgets::hud(ui, loc.logs_empty_title, t.muted);
                ui.add_space(6.0);
                widgets::hud(ui, loc.logs_empty_subtitle, t.faint);
            });
            return;
        }

        // 3. Monospace Terminal Well
        egui::Frame::new()
            .fill(t.sunken)
            .stroke(t.hairline())
            .corner_radius(CornerRadius::ZERO)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for log in filtered.iter().rev() {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;

                                // Timestamp
                                widgets::hud(ui, &log.timestamp, t.faint);

                                // Level Badge
                                let level_color = match log.level.as_str() {
                                    "ERROR" => t.bad,
                                    "WARN" => t.warn,
                                    _ => t.good,
                                };
                                widgets::badge(ui, &log.level, level_color);

                                // Category Badge
                                let cat_color = match log.category.as_str() {
                                    "PROCESS" => t.accent_ink,
                                    "RULE" => t.warn,
                                    "SYSTEM" => t.muted,
                                    _ => t.faint,
                                };
                                widgets::badge(ui, &log.category, cat_color);

                                // Message
                                ui.label(
                                    egui::RichText::new(&log.message)
                                        .family(theme::mono())
                                        .size(12.0)
                                        .color(t.ink),
                                );
                            });
                        }
                    });
            });
    });
}
