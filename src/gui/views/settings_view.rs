//! Preferences and System Settings:
//! Windows Registry autorun, tray icon picker, engine polling intervals,
//! memory architecture status, language, appearance, and application updates.

use egui::{CornerRadius, Ui, vec2};
use crate::gui::components::widgets::{self, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};

pub fn render_settings_view(ui: &mut Ui, state: &mut GuiState) {
    let t = Tokens::get(ui.ctx());
    let loc = state.loc();

    let (mut cfg, current_autorun) = match &state.daemon_state {
        Some(ds) => (ds.config.clone(), ds.autorun_enabled),
        None => {
            ui.add_space(32.0);
            widgets::hud(ui, loc.settings_daemon_offline, t.bad);
            return;
        }
    };

    let mut changed = false;

    egui::ScrollArea::vertical()
        .id_salt("settings_scroll_area")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available_w = ui.available_width();
            let two_columns = available_w >= 680.0;

            if two_columns {
                ui.spacing_mut().item_spacing.y = 16.0;

                // ROW 1: System (01) opposite Engine (02)
                let row1_id = egui::Id::new("settings_row1_h");
                let prev_target1: f32 = ui.data(|d| d.get_temp(row1_id)).unwrap_or(0.0);
                let mut nat_system = 0.0;
                let mut nat_engine = 0.0;

                ui.columns(2, |cols| {
                    nat_system = render_card(&mut cols[0], "01", loc.settings_eyebrow_system, prev_target1, &t, |ui| {
                        render_system_content(ui, state, &mut cfg, &mut changed, current_autorun, &t, loc);
                    });

                    nat_engine = render_card(&mut cols[1], "02", loc.settings_eyebrow_kernel, prev_target1, &t, |ui| {
                        render_kernel_content(ui, &mut cfg, &mut changed, &t, loc);
                    });
                });

                let new_target1 = nat_system.max(nat_engine);
                if (new_target1 - prev_target1).abs() > 0.5 {
                    ui.data_mut(|d| d.insert_temp(row1_id, new_target1));
                    ui.ctx().request_repaint();
                }

                // ROW 2: Appearance (03) opposite Updates (04)
                let row2_id = egui::Id::new("settings_row2_h");
                let prev_target2: f32 = ui.data(|d| d.get_temp(row2_id)).unwrap_or(0.0);
                let mut nat_appearance = 0.0;
                let mut nat_updates = 0.0;

                ui.columns(2, |cols| {
                    nat_appearance = render_card(&mut cols[0], "03", loc.settings_eyebrow_appearance, prev_target2, &t, |ui| {
                        render_appearance_content(ui, state, &mut cfg, &t, loc);
                    });

                    nat_updates = render_card(&mut cols[1], "04", loc.settings_eyebrow_updates, prev_target2, &t, |ui| {
                        render_updates_content(ui, state, &mut cfg, &mut changed, &t, loc);
                    });
                });

                let new_target2 = nat_appearance.max(nat_updates);
                if (new_target2 - prev_target2).abs() > 0.5 {
                    ui.data_mut(|d| d.insert_temp(row2_id, new_target2));
                    ui.ctx().request_repaint();
                }
            } else {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 16.0;
                    render_card(ui, "01", loc.settings_eyebrow_system, 0.0, &t, |ui| {
                        render_system_content(ui, state, &mut cfg, &mut changed, current_autorun, &t, loc);
                    });
                    render_card(ui, "02", loc.settings_eyebrow_kernel, 0.0, &t, |ui| {
                        render_kernel_content(ui, &mut cfg, &mut changed, &t, loc);
                    });
                    render_card(ui, "03", loc.settings_eyebrow_appearance, 0.0, &t, |ui| {
                        render_appearance_content(ui, state, &mut cfg, &t, loc);
                    });
                    render_card(ui, "04", loc.settings_eyebrow_updates, 0.0, &t, |ui| {
                        render_updates_content(ui, state, &mut cfg, &mut changed, &t, loc);
                    });
                });
            }
        });

    if changed {
        crate::daemon::tray::update_tray_icon(cfg.tray_icon_path.clone());
        let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::SaveConfig(cfg));
        state.set_toast(loc.toast_config_saved);
        state.sync_with_daemon();
    }
}

fn render_card<F: FnOnce(&mut Ui)>(
    ui: &mut Ui,
    eyebrow_idx: &str,
    eyebrow_label: &str,
    target_height: f32,
    t: &Tokens,
    add_contents: F,
) -> f32 {
    widgets::eyebrow(ui, eyebrow_idx, eyebrow_label);

    let mut natural_frame_h = 0.0;
    egui::Frame::new()
        .fill(t.sunken)
        .stroke(t.hairline())
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if target_height > 24.0 {
                ui.set_min_height(target_height - 24.0);
            }
            let inner = ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 10.0;
                add_contents(ui);
            });
            natural_frame_h = inner.response.rect.height() + 24.0;
        });

    natural_frame_h
}

fn render_system_content(
    ui: &mut Ui,
    state: &mut GuiState,
    cfg: &mut crate::config::AppConfig,
    changed: &mut bool,
    current_autorun: bool,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // Windows Registry Autorun
    let btn_label = if current_autorun { loc.settings_btn_disable_autorun } else { loc.settings_btn_enable_autorun };
    let btn_kind = if current_autorun { Kind::Secondary } else { Kind::Primary };

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if widgets::button(ui, btn_label, btn_kind, 0.0).clicked() {
            state.toggle_autorun(!current_autorun);
        }
        let status_badge = if current_autorun { "[ACTIVE]" } else { "[DISABLED]" };
        let status_color = if current_autorun { t.good } else { t.faint };
        widgets::hud(ui, status_badge, status_color);
    });

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
                *changed = true;
            }

            if widgets::button(ui, loc.settings_btn_browse, Kind::Secondary, browse_w).clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Icons & Executables", &["ico", "exe", "dll"])
                    .pick_file()
                {
                    cfg.tray_icon_path = Some(file.to_string_lossy().to_string());
                    *changed = true;
                }
            }

            if cfg.tray_icon_path.is_some() && widgets::button(ui, loc.settings_btn_reset, Kind::Secondary, 0.0).clicked() {
                cfg.tray_icon_path = None;
                *changed = true;
            }
        });
    });
}

fn render_kernel_content(
    ui: &mut Ui,
    cfg: &mut crate::config::AppConfig,
    changed: &mut bool,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // Mode Switching
    let mut mode_idx = if cfg.monitoring_mode.to_lowercase() == "wmi" { 1usize } else { 0usize };
    let mode_options = [
        (0usize, widgets::Cell::new(loc.settings_mode_polling, loc.settings_mode_polling_tip)),
        (1usize, widgets::Cell::new(loc.settings_mode_wmi, loc.settings_mode_wmi_tip)),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        widgets::hud(ui, loc.settings_monitoring_mode, t.muted);
        if widgets::cells(ui, &mut mode_idx, &mode_options) {
            cfg.monitoring_mode = if mode_idx == 1 { "wmi".to_string() } else { "polling".to_string() };
            *changed = true;
        }
    });

    // Scan Interval: only show if periodic polling is selected
    if cfg.monitoring_mode.to_lowercase() == "polling" {
        widgets::hrule(ui);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            widgets::hud(ui, loc.settings_scan_interval, t.muted);
            if ui.add(
                egui::DragValue::new(&mut cfg.poll_interval_sec)
                    .speed(0.1)
                    .range(0.5..=10.0)
                    .suffix(" s"),
            ).changed() {
                *changed = true;
            }
        });
    }
}

fn render_appearance_content(
    ui: &mut Ui,
    state: &mut GuiState,
    cfg: &mut crate::config::AppConfig,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // 1. Language
    let mut lang_idx = match state.language {
        crate::i18n::Language::English => 0usize,
        crate::i18n::Language::Russian => 1usize,
    };
    let lang_options = [
        (0usize, widgets::Cell::new("ENGLISH", "English interface")),
        (1usize, widgets::Cell::new("РУССКИЙ", "Русский интерфейс")),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
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

    widgets::hrule(ui);

    // 2. Color Theme
    let mut theme_val = state.theme;
    let theme_options = [
        (crate::config::AppTheme::Dark, widgets::Cell::new(loc.settings_theme_dark, loc.settings_theme_dark_tip)),
        (crate::config::AppTheme::Light, widgets::Cell::new(loc.settings_theme_light, loc.settings_theme_light_tip)),
        (crate::config::AppTheme::Chocolate, widgets::Cell::new(loc.settings_theme_chocolate, loc.settings_theme_chocolate_tip)),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        widgets::hud(ui, loc.settings_theme_label, t.muted);
        if widgets::cells(ui, &mut theme_val, &theme_options) {
            state.set_theme(theme_val);
            crate::gui::theme::apply_theme(ui.ctx(), theme_val);
            cfg.theme = theme_val;
            let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::SaveConfig(cfg.clone()));
        }
    });
}

fn render_updates_content(
    ui: &mut Ui,
    state: &mut GuiState,
    cfg: &mut crate::config::AppConfig,
    changed: &mut bool,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // 1. Checkbox: Check on startup
    if ui.checkbox(&mut cfg.check_updates_on_startup, loc.settings_check_startup_checkbox).changed() {
        *changed = true;
    }

    widgets::hrule(ui);

    // 2. Button & Version Label
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        let btn_label = if state.is_checking_update {
            loc.settings_btn_checking_updates
        } else {
            loc.settings_btn_check_updates
        };

        let can_click = !state.is_checking_update && !state.is_downloading_update;
        ui.add_enabled_ui(can_click, |ui| {
            if widgets::button(ui, btn_label, Kind::Secondary, 0.0).clicked() {
                state.check_for_updates(true);
            }
        });

        let cur_ver = format!("v{}", env!("CARGO_PKG_VERSION"));
        widgets::hud(ui, &loc.settings_last_checked.replace("{}", &cur_ver), t.faint);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_heights() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        let mut state = GuiState::new();
        state.daemon_state = Some(crate::ipc::DaemonState {
            autorun_enabled: true,
            uptime_secs: 100,
            daemon_memory_bytes: 1000,
            daemon_cpu_usage: 0.5,
            active_rules_count: 0,
            total_rules_count: 0,
            config: crate::config::AppConfig::default(),
            logs: vec![],
            active_rule_executions: std::collections::HashMap::new(),
            analytics: Default::default(),
        });

        // Run frame 1
        let mut out1 = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(1000.0);
                render_settings_view(ui, &mut state);
            });
        });
        out1.textures_delta.clear();

        // Run frame 2
        let mut out2 = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(1000.0);
                render_settings_view(ui, &mut state);
            });
        });
        out2.textures_delta.clear();

        // Switch to polling mode and test frame 3 & 4
        if let Some(ds) = &mut state.daemon_state {
            ds.config.monitoring_mode = "polling".to_string();
        }

        let mut out3 = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(1000.0);
                render_settings_view(ui, &mut state);
            });
        });
        out3.textures_delta.clear();

        let mut out4 = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(1000.0);
                render_settings_view(ui, &mut state);
            });
        });
        out4.textures_delta.clear();

        let t1: f32 = ctx.data(|d| d.get_temp(egui::Id::new("settings_row1_h"))).unwrap_or(0.0);
        let t2: f32 = ctx.data(|d| d.get_temp(egui::Id::new("settings_row2_h"))).unwrap_or(0.0);
        assert!(t1 > 50.0, "row 1 target height should be equalized");
        assert!(t2 > 50.0, "row 2 target height should be equalized");
    }
}

