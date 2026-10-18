//! Process Hunter: High-density Windows task manager & automation inspector
//! with segmented hairline filter cells, instant search, multi-column sorting,
//! TableBuilder, and 1-click rule/companion generator actions.

use egui::{Color32, CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};
use egui_extras::{Column, TableBuilder};
use crate::gui::components::widgets::{self, Cell, Kind};
use crate::gui::state::{GuiState, ProcessCategoryFilter, ProcessSortColumn};
use crate::gui::theme::{self, Tokens};
use crate::models::ProcessSnapshotItem;

pub fn render_processes_view(ui: &mut Ui, state: &mut GuiState) {
    let t = Tokens::get(ui.ctx());
    let loc = state.loc();

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;

        // 1. Toolbar with segmented cells & search edit
        render_toolbar(ui, state, &t);

        // 2. Filter & Sort processes
        let processes = filter_and_sort_processes(state);

        if processes.is_empty() {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                widgets::hud(ui, loc.hunter_empty_title, t.muted);
            });
            return;
        }

        // 3. Precision Table using TableBuilder
        let table_h = ui.available_height().max(200.0);
        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .min_scrolled_height(table_h)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::remainder().at_least(200.0).clip(true))
            .column(Column::exact(80.0))
            .column(Column::exact(130.0))
            .column(Column::exact(220.0).clip(true))
            .column(Column::exact(160.0))
            .header(32.0, |mut header| {
                header.col(|ui| {
                    render_sortable_header(ui, state, ProcessSortColumn::Name, loc.hunter_col_process_name, &t);
                });
                header.col(|ui| {
                    render_sortable_header(ui, state, ProcessSortColumn::Pid, loc.hunter_col_pid, &t);
                });
                header.col(|ui| {
                    render_sortable_header(ui, state, ProcessSortColumn::Memory, loc.hunter_col_working_set, &t);
                });
                header.col(|ui| {
                    render_sortable_header(ui, state, ProcessSortColumn::WindowTitle, loc.hunter_col_window_title, &t);
                });
                header.col(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        widgets::hud(ui, loc.hunter_col_quick_actions, t.faint);
                    });
                });
            })
            .body(|mut body| {
                for proc in processes {
                    let is_selected = state.selected_process.as_ref().map(|p| p.pid == proc.pid).unwrap_or(false);
                    body.row(36.0, |mut row| {
                        // Name Column
                        row.col(|ui| {
                            let (row_rect, row_resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
                            if row_resp.clicked() {
                                state.selected_process = Some(proc.clone());
                            }

                            let p = ui.painter();
                            if is_selected || row_resp.hovered() {
                                p.rect_filled(row_rect, CornerRadius::ZERO, t.sunken);
                            }
                            p.hline(row_rect.x_range(), row_rect.bottom() - 0.5, t.hairline());

                            // Draw real Win32 process icon
                            let icon_rect = Rect::from_min_size(pos2(row_rect.left() + 8.0, row_rect.center().y - 8.0), vec2(16.0, 16.0));
                            if let Some(tex) = state.icon_cache.get_icon(ui.ctx(), proc.pid, &proc.name) {
                                p.image(tex.id(), icon_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            } else {
                                p.rect_stroke(icon_rect.shrink(2.0), CornerRadius::ZERO, Stroke::new(1.0, t.line_strong), egui::StrokeKind::Inside);
                                p.circle_filled(icon_rect.center(), 2.0, t.muted);
                            }

                            let galley = widgets::caps(ui, &proc.name, theme::HUD_SIZE + 1.0, theme::mono_bold(), theme::HUD_SPACING, if is_selected { t.ink_strong } else { t.ink });
                            let text_pos = pos2(row_rect.left() + 32.0, row_rect.center().y - galley.size().y / 2.0);
                            p.galley(text_pos, galley, if is_selected { t.ink_strong } else { t.ink });
                        });

                        // PID Column
                        row.col(|ui| {
                            let (cell_rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                            ui.painter().hline(cell_rect.x_range(), cell_rect.bottom() - 0.5, t.hairline());
                            let galley = widgets::hud_galley(ui, &format!("{:05}", proc.pid), t.muted);
                            widgets::paint_left(ui, cell_rect, galley, t.muted);
                        });

                        // Memory Column
                        row.col(|ui| {
                            let mb = proc.memory_bytes as f64 / (1024.0 * 1024.0);
                            let mem_color = if mb > 1024.0 {
                                t.accent_ink
                            } else if mb > 400.0 {
                                t.warn
                            } else {
                                t.good
                            };

                            ui.horizontal(|ui| {
                                widgets::hud(ui, &format!("{:.1} MB", mb), mem_color);
                                let ratio = (mb / 1024.0).min(1.0) as f32;
                                let (bar_rect, _) = ui.allocate_exact_size(vec2(32.0, 4.0), Sense::hover());
                                ui.painter().rect_filled(bar_rect, CornerRadius::ZERO, t.line);
                                let mut fill_rect = bar_rect;
                                fill_rect.set_width(bar_rect.width() * ratio.max(0.08));
                                ui.painter().rect_filled(fill_rect, CornerRadius::ZERO, mem_color);
                            });
                        });

                        // Window Title Column
                        row.col(|ui| {
                            let title = if proc.window_title.is_empty() { "—" } else { &proc.window_title };
                            widgets::hud_elided(ui, title, if proc.window_title.is_empty() { t.faint } else { t.muted }, 210.0);
                        });

                        // Actions Column
                        row.col(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;

                                if widgets::button(ui, loc.hunter_btn_rule, Kind::Secondary, 0.0)
                                    .on_hover_text(loc.hunter_btn_rule_tip)
                                    .clicked()
                                {
                                    state.apply_process_to_draft_or_new_rule(&proc.name);
                                }
                            });
                        });
                    });
                }
            });
    });
}

fn render_toolbar(ui: &mut Ui, state: &mut GuiState, t: &Tokens) {
    let loc = state.loc();
    ui.horizontal(|ui| {
        let total = state.cached_processes.len();
        let windowed_count = state.cached_processes.iter().filter(|p| !p.window_title.is_empty()).count();
        let bg_count = total.saturating_sub(windowed_count);

        let options = [
            (
                ProcessCategoryFilter::All,
                Cell {
                    label: loc.hunter_cell_all,
                    tip: loc.hunter_cell_all_tip,
                    count: Some((total, None)),
                },
            ),
            (
                ProcessCategoryFilter::WindowedOnly,
                Cell {
                    label: loc.hunter_cell_windowed,
                    tip: loc.hunter_cell_windowed_tip,
                    count: Some((windowed_count, Some(t.accent_ink))),
                },
            ),
            (
                ProcessCategoryFilter::BackgroundOnly,
                Cell {
                    label: loc.hunter_cell_services,
                    tip: loc.hunter_cell_services_tip,
                    count: Some((bg_count, None)),
                },
            ),
        ];

        widgets::cells(ui, &mut state.process_category_filter, &options);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::button(ui, loc.hunter_btn_rescan, Kind::Secondary, 0.0).clicked() {
                if let Ok(crate::ipc::IpcResponse::Processes(procs)) =
                    crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::GetProcesses)
                {
                    state.cached_processes = procs;
                    state.set_toast(loc.toast_process_refreshed);
                }
            }

            ui.add_space(8.0);

            ui.add(
                egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text(loc.hunter_search_placeholder)
                    .desired_width(280.0)
                    .min_size(vec2(0.0, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(10.0, 8.0)),
            );
        });
    });
}

fn filter_and_sort_processes(state: &GuiState) -> Vec<ProcessSnapshotItem> {
    let needle = state.search_query.trim().to_lowercase();

    let mut filtered: Vec<ProcessSnapshotItem> = state
        .cached_processes
        .iter()
        .filter(|p| {
            match state.process_category_filter {
                ProcessCategoryFilter::All => true,
                ProcessCategoryFilter::WindowedOnly => !p.window_title.is_empty(),
                ProcessCategoryFilter::BackgroundOnly => p.window_title.is_empty(),
            }
        })
        .filter(|p| {
            if needle.is_empty() {
                true
            } else {
                p.name.to_lowercase().contains(&needle)
                    || p.pid.to_string().contains(&needle)
                    || p.window_title.to_lowercase().contains(&needle)
            }
        })
        .cloned()
        .collect();

    let asc = state.process_sort_ascending;
    match state.process_sort_column {
        ProcessSortColumn::Name => {
            filtered.sort_by(|a, b| {
                let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                if asc { cmp } else { cmp.reverse() }
            });
        }
        ProcessSortColumn::Pid => {
            filtered.sort_by(|a, b| {
                let cmp = a.pid.cmp(&b.pid);
                if asc { cmp } else { cmp.reverse() }
            });
        }
        ProcessSortColumn::Memory => {
            filtered.sort_by(|a, b| {
                let cmp = a.memory_bytes.cmp(&b.memory_bytes);
                if asc { cmp } else { cmp.reverse() }
            });
        }
        ProcessSortColumn::WindowTitle => {
            filtered.sort_by(|a, b| {
                let cmp = a.window_title.to_lowercase().cmp(&b.window_title.to_lowercase());
                if asc { cmp } else { cmp.reverse() }
            });
        }
    }

    filtered
}

fn render_sortable_header(
    ui: &mut Ui,
    state: &mut GuiState,
    col: ProcessSortColumn,
    title: &str,
    t: &Tokens,
) {
    let is_active = state.process_sort_column == col;
    let label = if is_active {
        if state.process_sort_ascending {
            format!("{} ▲", title)
        } else {
            format!("{} ▼", title)
        }
    } else {
        title.to_string()
    };

    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
    let color = if is_active || resp.hovered() { t.ink_strong } else { t.faint };
    let galley = widgets::caps(ui, &label, theme::HUD_SIZE, theme::mono_bold(), theme::HUD_SPACING, color);
    widgets::paint_left(ui, rect, galley, color);

    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        if is_active {
            state.process_sort_ascending = !state.process_sort_ascending;
        } else {
            state.process_sort_column = col;
            state.process_sort_ascending = true;
        }
    }
}
