//! Process Picker Modal: Studio modal for browsing and selecting an active Windows process
//! with segmented hairline filter cells, instant search, multi-column sorting,
//! Win32 icons, and 1-click select binding for triggers and actions.

use egui::{
    Color32, CornerRadius, Id, Margin, Order, Rect, Sense, Stroke, Ui,
    emath::easing, pos2, vec2,
};
use egui_extras::{Column, TableBuilder};
use crate::gui::components::widgets::{self, Cell, Glyph, Kind};
use crate::gui::state::{GuiState, ProcessCategoryFilter, ProcessSortColumn};
use crate::gui::theme::{self, Tokens};
use crate::models::ProcessSnapshotItem;

pub fn render_process_picker_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.picker_target.is_some();
    let mut close_requested = false;

    // Handle Esc key to close modal
    if is_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close_requested = true;
    }

    let anim = ctx.animate_bool_with_time_and_easing(
        Id::new("process-picker-modal-anim"),
        is_open,
        0.20,
        easing::cubic_out,
    );

    if anim <= 0.001 && !is_open {
        state.last_picker_target = None;
        return;
    }

    let target = state
        .picker_target
        .or(state.last_picker_target);

    let Some(_target) = target else { return };

    let full = ctx.content_rect();

    // 1. Full-screen scrim overlay
    let scrim_alpha = (anim * 195.0) as u8;
    egui::Area::new(Id::new("process-picker-modal-scrim"))
        .order(Order::Foreground)
        .fixed_pos(full.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(full.size(), Sense::click());
            ui.painter().rect_filled(
                rect,
                CornerRadius::ZERO,
                egui::Color32::from_black_alpha(scrim_alpha),
            );
            if resp.clicked() {
                close_requested = true;
            }
        });

    // 2. Centered Modal Box geometry
    let modal_w = (full.width() * 0.85).clamp(720.0, 960.0).min(full.width() - 32.0);
    let modal_h = (full.height() * 0.82).clamp(480.0, 680.0).min(full.height() - 48.0);
    let center = full.center();

    let scale = 0.95 + 0.05 * anim;
    let y_shift = (1.0 - anim) * 12.0;
    let modal_rect = Rect::from_center_size(
        pos2(center.x, center.y + y_shift),
        vec2(modal_w * scale, modal_h * scale),
    );

    let top_bar_h = 46.0;
    let top_bar_rect = Rect::from_min_size(modal_rect.min, vec2(modal_rect.width(), top_bar_h));
    let body_rect = Rect::from_min_max(
        pos2(modal_rect.left(), top_bar_rect.bottom()),
        modal_rect.max,
    );

    let loc = state.loc();
    let mut chosen_process: Option<String> = None;

    // 3. Modal Surface
    egui::Area::new(Id::new("process-picker-modal-window"))
        .order(Order::Tooltip)
        .fixed_pos(modal_rect.min)
        .show(ctx, |ui| {
            ui.set_width(modal_rect.width());
            ui.set_height(modal_rect.height());
            ui.set_max_width(modal_rect.width());
            ui.set_max_height(modal_rect.height());
            ui.set_clip_rect(modal_rect);

            // Paint background surfaces, dividers, and outer border stroke
            {
                let p = ui.painter();
                p.add(theme::shadow(&t).as_shape(modal_rect, CornerRadius::same(2)));
                p.rect_filled(modal_rect, CornerRadius::same(2), t.bg);
                p.rect_filled(top_bar_rect, CornerRadius::same(2), t.sunken);
                p.hline(modal_rect.x_range(), top_bar_rect.bottom(), t.hairline());

                // CRITICAL: Paint outer border stroke AFTER all inner fills so all 4 sides are consistently visible
                p.rect_stroke(
                    modal_rect,
                    CornerRadius::same(2),
                    Stroke::new(1.0, t.line_strong),
                    egui::StrokeKind::Inside,
                );
            }

            // Top Bar: Category Cells on the left, Search + Rescan + Close on the right (same row)
            ui.scope_builder(egui::UiBuilder::new().max_rect(top_bar_rect), |ui| {
                ui.set_width(top_bar_rect.width());
                ui.set_height(top_bar_rect.height());
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

                            // Right side: Close button, Rescan button, Search Input
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_close_tip).clicked() {
                                    close_requested = true;
                                }

                                ui.add_space(6.0);

                                if widgets::button(ui, loc.hunter_btn_rescan, Kind::Secondary, 0.0).clicked() {
                                    if let Ok(crate::ipc::IpcResponse::Processes(procs)) =
                                        crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::GetProcesses)
                                    {
                                        state.cached_processes = procs;
                                        state.set_toast(loc.toast_process_refreshed);
                                    }
                                }

                                ui.add_space(6.0);

                                ui.add(
                                    egui::TextEdit::singleline(&mut state.process_search_query)
                                        .hint_text(loc.hunter_search_placeholder)
                                        .desired_width(220.0)
                                        .min_size(vec2(0.0, theme::CONTROL_H))
                                        .vertical_align(egui::Align::Center)
                                        .margin(vec2(10.0, 8.0)),
                                );
                            });
                        });
                    });
            });

            // Body Content (Table)
            ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                ui.set_width(body_rect.width());
                ui.set_height(body_rect.height());
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
                                .column(Column::remainder().at_least(180.0).clip(true))
                                .column(Column::exact(65.0))
                                .column(Column::exact(120.0))
                                .column(Column::exact(150.0).clip(true))
                                .column(Column::exact(125.0))
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
                                        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                                        let galley = widgets::caps(ui, loc.hunter_col_quick_actions, theme::HUD_SIZE, theme::mono_bold(), theme::HUD_SPACING, t.faint);
                                        let text_w = galley.size().x;
                                        let text_x = if text_w <= rect.width() {
                                            rect.right() - text_w
                                        } else {
                                            rect.left()
                                        };
                                        let text_pos = pos2(text_x, rect.center().y - galley.size().y / 2.0);
                                        ui.painter().galley(text_pos, galley, t.faint);
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

                                            // Window Title Column (reduced width, elided)
                                            row.col(|ui| {
                                                let title = if proc.window_title.is_empty() { "—" } else { &proc.window_title };
                                                widgets::hud_elided(ui, title, if proc.window_title.is_empty() { t.faint } else { t.muted }, 140.0);
                                            });

                                            // Actions Column (SELECT button)
                                            row.col(|ui| {
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if widgets::button(ui, loc.hunter_btn_select, Kind::Primary, 0.0)
                                                        .on_hover_text(loc.hunter_btn_select_tip)
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
    } else if close_requested {
        state.close_process_picker();
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
