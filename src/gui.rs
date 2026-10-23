pub mod components;
pub mod macro_recorder;
pub mod state;
pub mod theme;
pub mod views;

use eframe::App;
use egui::{Color32, CornerRadius, Stroke, Ui, vec2};
use components::widgets;
use state::{ActiveView, GuiState};
use theme::{Tokens, apply_theme};
use views::action_catalog_modal::render_action_catalog_modal;
use views::analytics_view::render_analytics_view;
use views::changelog_modal::render_changelog_modal;
use views::guide_modal::render_guide_modal;
use views::header_bar::render_header_bar;
use views::inspector::render_inspector_drawer;
use views::macro_modal::render_macro_modal;
use views::process_picker_modal::render_process_picker_modal;
use views::rules_view::render_rules_view;
use views::script_modal::render_script_editor_canvas;
use views::settings_view::render_settings_view;
use crate::config::{AppConfig, WindowState};

pub struct ProcessHandlerApp {
    state: GuiState,
    fonts_ready: bool,
    current_theme: Option<crate::config::AppTheme>,
    last_window_state: Option<WindowState>,
}

impl ProcessHandlerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut state = GuiState::new();
        state.sync_with_daemon();
        let config = AppConfig::load_from_file(AppConfig::config_path());
        theme::apply_theme(&cc.egui_ctx, config.theme);
        if config.check_updates_on_startup {
            state.check_for_updates(false);
        }
        let last_window_state = config.window;
        Self {
            state,
            fonts_ready: false,
            current_theme: Some(config.theme),
            last_window_state,
        }
    }

    fn render_footer(&mut self, ui: &mut Ui, t: &Tokens) {
        let loc = self.state.loc();
        let r = ui.max_rect();
        // Hairline top border across footer
        ui.painter().hline(r.x_range(), r.top() + 0.5, t.hairline());

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            if let Some(ds) = &self.state.daemon_state {
                let active = ds.active_rules_count;
                let total = ds.total_rules_count;
                widgets::hud(ui, &format!("{}/{} {}", active, total, loc.footer_rules_active), t.faint);
            }

            // Right-aligned status
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let autorun = self.state.daemon_state.as_ref().map(|s| s.autorun_enabled).unwrap_or(false);
                widgets::hud(
                    ui,
                    if autorun { loc.footer_autorun_on } else { loc.footer_autorun_off },
                    if autorun { t.good } else { t.muted },
                );

                widgets::rule(ui, 14.0);
                widgets::hud(ui, &format!("v{}", env!("CARGO_PKG_VERSION")), t.faint);

                if self.state.is_downloading_update {
                    widgets::rule(ui, 14.0);
                    let pct = (self.state.update_download_progress * 100.0).clamp(0.0, 100.0);
                    let progress_text = loc.footer_downloading_progress.replace("{:.0}", &format!("{:.0}", pct));
                    widgets::hud(ui, &progress_text, t.accent_ink);
                } else if let Some(info) = self.state.available_update.clone() {
                    widgets::rule(ui, 14.0);

                    if widgets::compact_button(ui, loc.footer_btn_update, widgets::Kind::Primary, 22.0)
                        .on_hover_text(loc.footer_btn_update_tip)
                        .clicked()
                    {
                        self.state.start_update(info.clone());
                    }

                    if widgets::compact_button(ui, loc.footer_btn_info, widgets::Kind::Secondary, 22.0)
                        .on_hover_text(loc.footer_btn_info_tip)
                        .clicked()
                    {
                        self.state.show_changelog_modal = true;
                    }

                    let update_label = loc.footer_update_available.replace("{}", &info.version);
                    widgets::hud(ui, &update_label, t.good);
                }
            });
        });
    }

    fn render_toast(&self, ui: &mut Ui, _t: &Tokens) {
        if let Some((msg, time)) = &self.state.toast {
            let elapsed = time.elapsed().as_secs_f32();
            if elapsed < 3.0 {
                // Smooth frame repainting during fade in/out
                ui.ctx().request_repaint();

                let alpha = if elapsed < 0.25 {
                    (elapsed / 0.25).min(1.0)
                } else if elapsed > 2.5 {
                    ((3.0 - elapsed) / 0.5).max(0.0)
                } else {
                    1.0
                };

                let fill = Color32::from_black_alpha((alpha * 240.0) as u8);
                let stroke = Stroke::new(1.0, Color32::from_white_alpha((alpha * 120.0) as u8));
                let text_color = Color32::from_white_alpha((alpha * 255.0) as u8);

                egui::Area::new(egui::Id::new("app-toast"))
                    .order(egui::Order::Tooltip)
                    .anchor(egui::Align2::CENTER_BOTTOM, vec2(0.0, -theme::FOOTER_H - 24.0))
                    .interactable(false)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new()
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(CornerRadius::ZERO)
                            .inner_margin(egui::Margin::symmetric(24, 10))
                            .show(ui, |ui| {
                                let galley = widgets::caps(
                                    ui,
                                    msg,
                                    theme::HUD_SIZE,
                                    theme::mono_bold(),
                                    theme::HUD_SPACING,
                                    text_color,
                                );
                                let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                                widgets::paint_centred(ui, rect, galley, text_color);
                            });
                    });
            }
        }
    }
}

impl App for ProcessHandlerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.fonts_ready {
            self.fonts_ready = true;
            ctx.request_repaint();
            return;
        }
        if self.current_theme != Some(self.state.theme) {
            apply_theme(&ctx, self.state.theme);
            self.current_theme = Some(self.state.theme);
        }

        // Sync with daemon state
        self.state.sync_with_daemon();
        self.state.poll_update_events();
        if self.state.is_any_rule_executing() {
            ctx.request_repaint();
        }

        // Handle GUI window close while update is downloading:
        // Cancel process termination, hide window, and allow download to finish in background.
        if ctx.input(|i| i.viewport().close_requested()) && self.state.is_downloading_update {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.state.gui_closed_during_update = true;
            crate::updater::set_gui_closed_during_update(true);
        }

        if self.state.is_downloading_update {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        let tokens = Tokens::get(&ctx);

        // 1. Top Header Bar (Title Bar & Navigation)
        egui::Panel::top("header")
            .exact_size(theme::HEADER_H)
            .resizable(false)
            .frame(egui::Frame::new().fill(tokens.bg))
            .show(ui, |ui| {
                render_header_bar(ui, &mut self.state);
            });

        // 2. Bottom Footer Bar (Telemetry, Memory, Autorun)
        egui::Panel::bottom("footer")
            .exact_size(theme::FOOTER_H)
            .resizable(false)
            .frame(egui::Frame::new().fill(tokens.bg).inner_margin(egui::Margin::symmetric(20, 0)))
            .show(ui, |ui| {
                self.render_footer(ui, &tokens);
            });

        // 3. Central Stage (Wide, comfortable, elegant)
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(tokens.bg).inner_margin(egui::Margin { left: 24, right: 24, top: 18, bottom: 8 }))
            .show(ui, |ui| {
                match self.state.active_view {
                    ActiveView::Rules => render_rules_view(ui, &mut self.state),
                    ActiveView::Analytics => render_analytics_view(ui, &mut self.state),
                    ActiveView::Settings => render_settings_view(ui, &mut self.state),
                }

                // Toast notification
                self.render_toast(ui, &tokens);
            });

        // 4. Sliding Inspector Drawer (slides in from right with cubic easing)
        render_inspector_drawer(&ctx, &mut self.state);

        // 5. Action Catalog Modal (library for selecting rule actions)
        render_action_catalog_modal(&ctx, &mut self.state);

        // 6. Process Picker Modal (Hunter inside studio modal for selecting a running process)
        render_process_picker_modal(&ctx, &mut self.state);

        // 7. Centered Expanded Script Canvas (modal popup for pwsh / cmd editing)
        render_script_editor_canvas(&ctx, &mut self.state);

        // 8. Centered Interactive Macro Studio Modal (record / edit keystrokes & mouse)
        render_macro_modal(&ctx, &mut self.state);

        // 9. Centered Changelog & Release Notes Modal
        render_changelog_modal(&ctx, &mut self.state);

        // 10. Centered Editor Guide & Documentation Modal
        render_guide_modal(&ctx, &mut self.state);

        // 7. Borderless Window Edges (8-direction border resize & hairline stroke)
        widgets::window_edges(&ctx);

        // Track window size and position for persistence across restarts
        let vp = ctx.input(|i| i.viewport().clone());
        let is_max = vp.maximized.unwrap_or(false);
        let mut new_state = None;

        if is_max {
            if let Some(ws) = &self.last_window_state {
                let mut updated = ws.clone();
                updated.is_maximized = true;
                new_state = Some(updated);
            }
        } else if let Some(rect) = vp.inner_rect {
            if rect.width() >= 400.0 && rect.height() >= 300.0 {
                new_state = Some(WindowState {
                    x: rect.min.x,
                    y: rect.min.y,
                    width: rect.width(),
                    height: rect.height(),
                    is_maximized: false,
                });
            }
        }

        if let Some(new_ws) = new_state {
            let changed = match &self.last_window_state {
                Some(old) => {
                    (old.x - new_ws.x).abs() > 2.0
                        || (old.y - new_ws.y).abs() > 2.0
                        || (old.width - new_ws.width).abs() > 2.0
                        || (old.height - new_ws.height).abs() > 2.0
                        || old.is_maximized != new_ws.is_maximized
                }
                None => true,
            };
            if changed {
                persist_window_state(&new_ws);
                self.last_window_state = Some(new_ws);
            }
        }

        // Repaint periodically for telemetry
        ctx.request_repaint_after(std::time::Duration::from_millis(300));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(ws) = &self.last_window_state {
            persist_window_state(ws);
        }
    }
}

impl Drop for ProcessHandlerApp {
    fn drop(&mut self) {
        if let Some(ws) = &self.last_window_state {
            persist_window_state(ws);
        }
    }
}

fn persist_window_state(ws: &WindowState) {
    if let Ok(crate::ipc::IpcResponse::Ok) = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::UpdateWindowState(ws.clone())) {
        return;
    }
    AppConfig::save_window_state(ws);
}

pub fn run_gui() -> eframe::Result<()> {
    let _instance_guard = match crate::single_instance::SingleInstanceGuard::acquire_gui() {
        Some(guard) => guard,
        None => {
            crate::single_instance::focus_existing_gui_window();
            return Ok(());
        }
    };

    let config = AppConfig::load_from_file(AppConfig::config_path());
    let saved_state = config.window;

    let mut viewport = egui::ViewportBuilder::default()
        .with_min_inner_size([880.0, 580.0])
        .with_title("ProcessHandler")
        .with_decorations(false);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/app.png")) {
        viewport = viewport.with_icon(icon);
    }

    if let Some(ws) = &saved_state {
        viewport = viewport
            .with_inner_size([ws.width, ws.height])
            .with_position([ws.x, ws.y]);
        if ws.is_maximized {
            viewport = viewport.with_maximized(true);
        }
    } else {
        viewport = viewport.with_inner_size([1180.0, 760.0]);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ProcessHandler",
        native_options,
        Box::new(|cc| Ok(Box::new(ProcessHandlerApp::new(cc)))),
    )
}
