//! Preferences and System Settings:
//! Windows Registry autorun, tray icon picker, engine polling intervals,
//! memory architecture status, and daemon shutdown.

use egui::{CornerRadius, Ui, vec2};
use crate::gui::components::widgets::{self, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};

pub fn render_settings_view(ui: &mut Ui, state: &mut GuiState) {
    let t = Tokens::get(ui.ctx());
    let loc = state.loc();

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 16.0;

        let (mut cfg, current_autorun) = match &state.daemon_state {
            Some(ds) => (ds.config.clone(), ds.autorun_enabled),
            None => {
                ui.add_space(32.0);
                widgets::hud(ui, loc.settings_daemon_offline, t.bad);
                return;
            }
        };

        let mut changed = false;

        // SECTION 01: Windows Logon & System Integration
        widgets::eyebrow(ui, "01", loc.settings_eyebrow_system);

        egui::Frame::new()
            .fill(t.sunken)
            .stroke(t.hairline())
            .corner_radius(CornerRadius::ZERO)
            .inner_margin(egui::Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 12.0;

                    // Windows Registry Autorun: only the toggle button
                    let btn_label = if current_autorun { loc.settings_btn_disable_autorun } else { loc.settings_btn_enable_autorun };
                    let btn_kind = if current_autorun { Kind::Secondary } else { Kind::Primary };

                    if widgets::button(ui, btn_label, btn_kind, 0.0).clicked() {
                        state.toggle_autorun(!current_autorun);
                    }

                    widgets::hrule(ui);

                    // Custom Tray Icon Override
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        widgets::hud(ui, loc.settings_tray_icon_override, t.muted);

                        let mut path = cfg.tray_icon_path.clone().unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;

                            let browse_w = widgets::button_width(ui, loc.settings_btn_browse, 0.0);
                            let has_reset = cfg.tray_icon_path.is_some();
                            let reset_w = if has_reset {
                                widgets::button_width(ui, loc.settings_btn_reset, 0.0) + 8.0
                            } else {
                                0.0
                            };
                            let input_width = (ui.available_width() - browse_w - reset_w - 8.0).max(80.0);

                            if ui.add(
                                egui::TextEdit::singleline(&mut path)
                                    .hint_text(loc.settings_tray_icon_placeholder)
                                    .desired_width(input_width)
                                    .min_size(vec2(input_width, theme::CONTROL_H))
                                    .vertical_align(egui::Align::Center)
                                    .margin(vec2(10.0, 8.0)),
                            ).changed() {
                                cfg.tray_icon_path = if path.trim().is_empty() { None } else { Some(path) };
                                changed = true;
                            }

                            if widgets::button(ui, loc.settings_btn_browse, Kind::Secondary, browse_w).clicked() {
                                if let Some(file) = rfd::FileDialog::new()
                                    .add_filter("Icons & Executables", &["ico", "exe", "dll"])
                                    .pick_file()
                                {
                                    cfg.tray_icon_path = Some(file.to_string_lossy().to_string());
                                    changed = true;
                                }
                            }

                            if cfg.tray_icon_path.is_some() && widgets::button(ui, loc.settings_btn_reset, Kind::Secondary, 0.0).clicked() {
                                cfg.tray_icon_path = None;
                                changed = true;
                            }
                        });
                    });
                });
            });

        // SECTION 02: Engine Polling & Monitoring Mode
        widgets::eyebrow(ui, "02", loc.settings_eyebrow_kernel);

        egui::Frame::new()
            .fill(t.sunken)
            .stroke(t.hairline())
            .corner_radius(CornerRadius::ZERO)
            .inner_margin(egui::Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 10.0;

                    // Mode Switching
                    let mut mode_idx = if cfg.monitoring_mode.to_lowercase() == "wmi" { 1usize } else { 0usize };
                    let mode_options = [
                        (0usize, widgets::Cell::new(loc.settings_mode_polling, loc.settings_mode_polling_tip)),
                        (1usize, widgets::Cell::new(loc.settings_mode_wmi, loc.settings_mode_wmi_tip)),
                    ];

                    ui.horizontal_wrapped(|ui| {
                        widgets::hud(ui, loc.settings_monitoring_mode, t.muted);
                        if widgets::cells(ui, &mut mode_idx, &mode_options) {
                            cfg.monitoring_mode = if mode_idx == 1 { "wmi".to_string() } else { "polling".to_string() };
                            changed = true;
                        }
                    });

                    // Scan Interval: only show if periodic polling is selected
                    if cfg.monitoring_mode.to_lowercase() == "polling" {
                        widgets::hrule(ui);
                        ui.horizontal_wrapped(|ui| {
                            widgets::hud(ui, loc.settings_scan_interval, t.muted);
                            if ui.add(
                                egui::DragValue::new(&mut cfg.poll_interval_sec)
                                    .speed(0.1)
                                    .range(0.5..=10.0)
                                    .suffix(" s"),
                            ).changed() {
                                changed = true;
                            }
                        });
                    }
                });
            });

        // SECTION 03: User Interface Language
        widgets::eyebrow(ui, "03", loc.settings_eyebrow_language);

        egui::Frame::new()
            .fill(t.sunken)
            .stroke(t.hairline())
            .corner_radius(CornerRadius::ZERO)
            .inner_margin(egui::Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 10.0;

                    let mut lang_idx = match state.language {
                        crate::i18n::Language::English => 0usize,
                        crate::i18n::Language::Russian => 1usize,
                    };
                    let lang_options = [
                        (0usize, widgets::Cell::new("ENGLISH", "English interface")),
                        (1usize, widgets::Cell::new("РУССКИЙ", "Русский интерфейс")),
                    ];

                    ui.horizontal_wrapped(|ui| {
                        widgets::hud(ui, loc.settings_language_label, t.muted);
                        if widgets::cells(ui, &mut lang_idx, &lang_options) {
                            let new_lang = if lang_idx == 1 {
                                crate::i18n::Language::Russian
                            } else {
                                crate::i18n::Language::English
                            };
                            state.set_language(new_lang);
                            cfg.language = new_lang;
                            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::SaveConfig(cfg.clone()));
                        }
                    });
                });
            });

        if changed {
            crate::daemon::tray::update_tray_icon(cfg.tray_icon_path.clone());
            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::SaveConfig(cfg));
            state.set_toast(loc.toast_config_saved);
            state.sync_with_daemon();
        }
    });
}
