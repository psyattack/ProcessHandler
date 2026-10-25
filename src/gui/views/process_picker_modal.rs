//! Process Picker Modal: Studio modal for browsing and selecting an active Windows process
//! with segmented hairline filter cells, instant search, multi-column sorting,
//! Win32 icons, and 1-click select binding for triggers and actions.

use egui::{
    Color32, CornerRadius, Margin, Rect, Sense, Stroke,
    pos2, vec2,
};
use egui_extras::{Column, TableBuilder};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Cell, Glyph, Kind};
use crate::gui::state::{GuiState, ProcessCategoryFilter, ProcessSortColumn};
use crate::gui::theme::{self, Tokens};
use crate::models::ProcessSnapshotItem;

pub fn render_process_picker_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.picker_target.is_some();
    let loc = state.loc();
    let mut chosen_process: Option<String> = None;

    let modal_res = Modal::new("process-picker-modal", is_open)
        .width_fraction(0.85)
        .height_fraction(0.82)
        .min_size(vec2(720.0, 480.0))
        .max_size(vec2(960.0, 680.0))
        .header_height(46.0)
        .header_sunken(true)
        .accent_bar(t.accent_ink)
        .show(ctx, &t, |frame| {
            // Header: Category Cells on the left, Search + Close on the right
            frame.header(|ui, close_req| {
                egui::Frame::new()
                    .inner_margin(Margin { left: 14, right: 12, top: 7, bottom: 7 })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;

                            // Left side: Segmented category filter cells
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

                            // Right side: Close button, Search Input
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_close_tip).clicked() {
                                    *close_req = true;
                                }

                                ui.add_space(8.0);

                                ui.add(
                                    egui::TextEdit::singleline(&mut state.process_search_query)
                                        .hint_text(loc.hunter_search_placeholder)
                                        .desired_width(240.0)
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                );
                            });
                        });
                    });
            });

            // Body Content (Table)
            frame.body(|ui| {
                egui::Frame::new()
                    .inner_margin(Margin { left: 14, right: 14, top: 4, bottom: 8 })
                    .show(ui, |ui| {
                        let processes = filter_and_sort_processes(state);
                        if processes.is_empty() {
                            ui.add_space(60.0);
                            ui.vertical_centered(|ui| {
                                widgets::hud(ui, loc.hunter_empty_title, t.muted);
                            });
                        } else {
                            let table_h = ui.available_height().max(100.0);
                            TableBuilder::new(ui)
                                .striped(false)
                                .resizable(false)
                                .min_scrolled_height(table_h)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(Column::exact(300.0).clip(true))
                                .column(Column::exact(65.0))
                                .column(Column::remainder().at_least(200.0).clip(true))
                                .column(Column::exact(125.0))
                                .header(32.0, |mut header| {
                                    header.col(|ui| {
                                        render_sortable_header(ui, state, ProcessSortColumn::Name, loc.hunter_col_process_name, &t);
                                    });
                                    header.col(|ui| {
                                        render_sortable_header(ui, state, ProcessSortColumn::Pid, loc.hunter_col_pid, &t);
                                    });
                                    header.col(|ui| {
                                        render_sortable_header(ui, state, ProcessSortColumn::WindowTitle, loc.hunter_col_window_title, &t);
                                    });
                                    header.col(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                                        let galley = widgets::caps(ui, loc.hunter_col_quick_actions, theme::HUD_SIZE, theme::mono_bold(), theme::HUD_SPACING, t.faint);
                                        widgets::paint_centred(ui, rect, galley, t.faint);
                                    });
                                })
                                .body(|mut body| {
                                    for proc in processes {
                                        body.row(36.0, |mut row| {
                                            let mut row_double_clicked = false;

                                            // Name Column
                                            row.col(|ui| {
                                                let (row_rect, row_resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
                                                if row_resp.double_clicked() {
                                                    row_double_clicked = true;
                                                }

                                                let p = ui.painter();
                                                if row_resp.hovered() {
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

                                                let galley = widgets::caps(ui, &proc.name, theme::HUD_SIZE + 1.0, theme::mono_bold(), theme::HUD_SPACING, t.ink_strong);
                                                let text_pos = pos2(row_rect.left() + 32.0, row_rect.center().y - galley.size().y / 2.0);
                                                p.galley(text_pos, galley, t.ink_strong);
                                            });

                                            // PID Column
                                            row.col(|ui| {
                                                let (cell_rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
                                                if resp.double_clicked() {
                                                    row_double_clicked = true;
                                                }
                                                ui.painter().hline(cell_rect.x_range(), cell_rect.bottom() - 0.5, t.hairline());
                                                let galley = widgets::hud_galley(ui, &format!("{:05}", proc.pid), t.muted);
                                                widgets::paint_left(ui, cell_rect, galley, t.muted);
                                            });

                                            // Window Title Column (expanded width, elided)
                                            row.col(|ui| {
                                                let (cell_rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
                                                if resp.double_clicked() {
                                                    row_double_clicked = true;
                                                }
                                                ui.painter().hline(cell_rect.x_range(), cell_rect.bottom() - 0.5, t.hairline());

                                                let title = if proc.window_title.is_empty() { "—" } else { &proc.window_title };
                                                let color = if proc.window_title.is_empty() { t.faint } else { t.muted };
                                                let format = egui::TextFormat {
                                                    font_id: egui::FontId::new(theme::HUD_SIZE, theme::mono()),
                                                    color,
                                                    extra_letter_spacing: theme::HUD_SPACING,
                                                    ..Default::default()
                                                };
                                                let mut job = egui::text::LayoutJob::single_section(title.to_uppercase(), format);
                                                job.wrap = egui::text::TextWrapping::truncate_at_width(cell_rect.width());
                                                let galley = ui.painter().layout_job(job);
                                                let elided = galley.elided;
                                                widgets::paint_left(ui, cell_rect, galley, color);
                                                if elided {
                                                    resp.on_hover_text(title);
                                                }
                                            });

                                            // Actions Column (SELECT button)
                                            row.col(|ui| {
                                                ui.vertical_centered(|ui| {
                                                    ui.add_space((ui.available_height() - theme::CONTROL_H).max(0.0) / 2.0);
                                                    if widgets::button(ui, loc.hunter_btn_select, Kind::Primary, 0.0)
                                                        .clicked()
                                                        || row_double_clicked
                                                    {
                                                        chosen_process = Some(proc.name.clone());
                                                    }
                                                });
                                            });
                                        });
                                    }
                                });
                        }
                    });
            });
        });

    if let Some(name) = chosen_process {
        state.apply_selected_process(&name);
    } else if let Some(res) = modal_res {
        if res.close_requested {
            state.close_process_picker();
        }
    } else if !is_open {
        state.last_picker_target = None;
    }
}

fn filter_and_sort_processes(state: &GuiState) -> Vec<ProcessSnapshotItem> {
    let needle = state.process_search_query.trim().to_lowercase();

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
        ProcessSortColumn::Name => filtered.sort_by(|a, b| {
            let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
            if asc { cmp } else { cmp.reverse() }
        }),
        ProcessSortColumn::Pid => filtered.sort_by(|a, b| {
            let cmp = a.pid.cmp(&b.pid);
            if asc { cmp } else { cmp.reverse() }
        }),
        ProcessSortColumn::WindowTitle => filtered.sort_by(|a, b| {
            let cmp = a.window_title.to_lowercase().cmp(&b.window_title.to_lowercase());
            if asc { cmp } else { cmp.reverse() }
        }),
    }

    filtered
}

fn render_sortable_header(
    ui: &mut egui::Ui,
    state: &mut GuiState,
    col: ProcessSortColumn,
    label: &str,
    t: &Tokens,
) {
    let is_active = state.process_sort_column == col;
    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());

    if resp.clicked() {
        if is_active {
            state.process_sort_ascending = !state.process_sort_ascending;
        } else {
            state.process_sort_column = col;
            state.process_sort_ascending = true;
        }
    }

    let color = if resp.hovered() {
        t.ink_strong
    } else if is_active {
        t.accent_ink
    } else {
        t.faint
    };

    let title = if is_active {
        if state.process_sort_ascending {
            format!("{} ▲", label)
        } else {
            format!("{} ▼", label)
        }
    } else {
        label.to_string()
    };

    let galley = widgets::caps(ui, &title, theme::HUD_SIZE, theme::mono_bold(), theme::HUD_SPACING, color);
    widgets::paint_left(ui, rect, galley, color);
}
