//! Preferences and System Settings:
//! Modern Master-Detail layout with clean sidebar categories, toggle switches,
//! theme palette cards, live tray icon preview, and direct configuration folder access.

use egui::{Color32, CornerRadius, Rect, Sense, Stroke, Ui, pos2, vec2};
use crate::gui::components::widgets::{self, Kind};
use crate::gui::state::{GuiState, SettingsTab};
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
            let is_wide = available_w >= 620.0;

            if is_wide {
                let start_top = ui.cursor().top();
                let initial_avail_h = ui.available_height();

                let mut sidebar_rect = Rect::NOTHING;
                let mut content_rect = Rect::NOTHING;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;

                    // Left Column: Navigation Sidebar
                    let sidebar_w = 190.0;
                    let resp_side = ui.allocate_ui_with_layout(
                        vec2(sidebar_w, ui.available_height()),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            render_sidebar(ui, state, loc, &t, sidebar_w);
                        },
                    );
                    sidebar_rect = resp_side.response.rect;

                    // Space reserved for the vertical divider
                    ui.add_space(1.0);

                    // Right Column: Active Content Panel
                    let content_w = ui.available_width();
                    let resp_content = ui.allocate_ui_with_layout(
                        vec2(content_w, ui.available_height()),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 12.0;
                            render_content_panel(ui, state, &mut cfg, &mut changed, current_autorun, &t, loc);
                        },
                    );
                    content_rect = resp_content.response.rect;
                });

                // Vertical divider from header edge to footer edge (exact 1px filled rect, DPI-independent)
                let div_x = sidebar_rect.right() + 12.0;
                let top_y = theme::HEADER_H;
                let bottom_y = (ui.ctx().content_rect().bottom() - theme::FOOTER_H)
                    .max(content_rect.bottom())
                    .max(start_top + initial_avail_h);

                if bottom_y > top_y {
                    let v_divider = Rect::from_min_max(
                        pos2(div_x, top_y),
                        pos2(div_x + 1.0, bottom_y),
                    );
                    ui.painter()
                        .with_clip_rect(ui.ctx().content_rect())
                        .rect_filled(v_divider, CornerRadius::ZERO, t.hairline().color);
                }
            } else {
                // Narrow / Compact Responsive Fallback: Top segmented tabs + panel
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    let tab_options = [
                        (SettingsTab::General, widgets::Cell::new(loc.settings_tab_general, "")),
                        (SettingsTab::Appearance, widgets::Cell::new(loc.settings_tab_appearance, "")),
                        (SettingsTab::Engine, widgets::Cell::new(loc.settings_tab_engine, "")),
                        (SettingsTab::Updates, widgets::Cell::new(loc.settings_tab_updates, "")),
                    ];
                    widgets::cells(ui, &mut state.settings_tab, &tab_options);
                    widgets::hrule(ui);
                    render_content_panel(ui, state, &mut cfg, &mut changed, current_autorun, &t, loc);
                });
            }
        });

    if changed {
        crate::daemon::tray::update_tray_icon(cfg.tray_icon_path.clone());
        crate::daemon::tray::set_tray_visible(!cfg.hide_tray_icon);
        let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::SaveConfig(cfg));
        state.set_toast(loc.toast_config_saved);
        state.sync_with_daemon();
    }
}

fn render_sidebar(ui: &mut Ui, state: &mut GuiState, loc: &crate::i18n::Strings, t: &Tokens, w: f32) {
    let tabs = [
        (SettingsTab::General, "01", loc.settings_tab_general),
        (SettingsTab::Appearance, "02", loc.settings_tab_appearance),
        (SettingsTab::Engine, "03", loc.settings_tab_engine),
        (SettingsTab::Updates, "04", loc.settings_tab_updates),
    ];

    for (tab, idx, label) in tabs {
        let is_active = state.settings_tab == tab;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
        if resp.clicked() {
            state.settings_tab = tab;
        }
        let enabled = ui.is_enabled();
        let hovered = resp.hovered() && enabled;

        let bg_fill = if is_active {
            t.sunken
        } else if hovered {
            t.sunken.gamma_multiply(0.6)
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, CornerRadius::ZERO, bg_fill);

        if is_active {
            let bar_rect = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
            ui.painter().rect_filled(bar_rect, CornerRadius::ZERO, t.accent);
        }

        let text_y = rect.center().y;
        let idx_color = if is_active { t.accent_ink } else { t.faint };
        let idx_galley = widgets::caps(ui, idx, 10.0, theme::mono(), 1.0, idx_color);
        let sep_galley = widgets::caps(ui, " / ", 10.0, theme::mono(), 1.0, t.faint);
        let label_color = if is_active {
            t.ink_strong
        } else if hovered {
            t.ink
        } else {
            t.muted
        };
        let label_galley = widgets::caps(ui, label, 11.0, theme::mono_bold(), theme::BUTTON_SPACING, label_color);

        let mut x = rect.left() + 14.0;
        widgets::paint_left(ui, Rect::from_min_size(pos2(x, text_y - idx_galley.size().y / 2.0), idx_galley.size()), idx_galley.clone(), idx_color);
        x += idx_galley.size().x;
        widgets::paint_left(ui, Rect::from_min_size(pos2(x, text_y - sep_galley.size().y / 2.0), sep_galley.size()), sep_galley.clone(), t.faint);
        x += sep_galley.size().x;
        widgets::paint_left(ui, Rect::from_min_size(pos2(x, text_y - label_galley.size().y / 2.0), label_galley.size()), label_galley, label_color);

        if enabled {
            resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }
}

fn render_content_panel(
    ui: &mut Ui,
    state: &mut GuiState,
    cfg: &mut crate::config::AppConfig,
    changed: &mut bool,
    current_autorun: bool,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    egui::Frame::new()
        .fill(t.sunken)
        .stroke(t.hairline())
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;

            match state.settings_tab {
                SettingsTab::General => {
                    render_general_content(ui, state, cfg, changed, current_autorun, t, loc);
                }
                SettingsTab::Appearance => {
                    render_appearance_content(ui, state, cfg, t, loc);
                }
                SettingsTab::Engine => {
                    render_engine_content(ui, cfg, changed, t, loc);
                }
                SettingsTab::Updates => {
                    render_updates_content(ui, state, cfg, changed, t, loc);
                }
            }
        });
}

fn render_setting_row<F: FnOnce(&mut Ui)>(
    ui: &mut Ui,
    title: &str,
    subtitle: &str,
    t: &Tokens,
    add_control: F,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        let available_w = ui.available_width();
        let control_w_estimate = 260.0;
        let text_w = (available_w - control_w_estimate).max(140.0);

        ui.allocate_ui_with_layout(
            vec2(text_w, 36.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add(egui::Label::new(
                    egui::RichText::new(title)
                        .font(egui::FontId::new(13.0, theme::medium()))
                        .color(t.ink_strong),
                ));
                ui.add(egui::Label::new(
                    egui::RichText::new(subtitle)
                        .font(egui::FontId::new(11.0, egui::FontFamily::Proportional))
                        .color(t.muted),
                ));
            },
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            add_control(ui);
        });
    });
}

fn render_general_content(
    ui: &mut Ui,
    state: &mut GuiState,
    cfg: &mut crate::config::AppConfig,
    changed: &mut bool,
    current_autorun: bool,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // 1. Windows Startup Toggle
    render_setting_row(ui, loc.settings_autorun_title, loc.settings_autorun_desc, t, |ui| {
        let mut autorun_val = current_autorun;
        if widgets::toggle_switch(ui, &mut autorun_val).changed() {
            state.toggle_autorun(autorun_val);
        }
    });

    widgets::hrule(ui);

    // 2. Custom Tray Icon with live thumbnail preview
    render_setting_row(ui, loc.settings_tray_icon_title, loc.settings_tray_icon_desc, t, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // Live preview button matching the exact size of the input line (theme::CONTROL_H x theme::CONTROL_H)
            let preview_size = vec2(theme::CONTROL_H, theme::CONTROL_H);
            let (p_rect, p_resp) = ui.allocate_exact_size(preview_size, Sense::click());
            let p_hovered = p_resp.hovered();
            let p_stroke = if p_hovered { Stroke::new(1.0, t.line_strong) } else { t.hairline() };
            ui.painter().rect(p_rect, CornerRadius::ZERO, t.bg, p_stroke, egui::StrokeKind::Inside);

            let mut icon_rendered = false;
            if let Some(ref path) = cfg.tray_icon_path {
                if let Some(tex) = state.icon_cache.get_icon_from_path(ui.ctx(), path) {
                    let img_rect = Rect::from_center_size(p_rect.center(), vec2(22.0, 22.0));
                    ui.painter().image(
                        tex.id(),
                        img_rect,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    icon_rendered = true;
                }
            }

            // By default, display the official application logo icon (not a simple glyph)
            if !icon_rendered {
                if let Some(app_tex) = state.icon_cache.get_app_icon(ui.ctx()) {
                    let img_rect = Rect::from_center_size(p_rect.center(), vec2(22.0, 22.0));
                    ui.painter().image(
                        app_tex.id(),
                        img_rect,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    icon_rendered = true;
                }
            }

            if !icon_rendered {
                let icon_glyph = egui_phosphor::regular::APP_WINDOW;
                let galley = egui::WidgetText::from(
                    egui::RichText::new(icon_glyph)
                        .font(egui::FontId::new(16.0, theme::phosphor()))
                        .color(t.muted),
                ).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::FontSelection::Default);
                widgets::paint_centred(ui, p_rect, galley, t.muted);
            }

            // Clicking the preview box opens the file picker
            if p_resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Icons & Executables", &["ico", "exe", "dll"])
                    .pick_file()
                {
                    cfg.tray_icon_path = Some(file.to_string_lossy().to_string());
                    *changed = true;
                }
            }

            let has_reset = cfg.tray_icon_path.is_some();
            let reset_w = if has_reset {
                widgets::button_width(ui, loc.settings_btn_reset, 0.0) + 8.0
            } else {
                0.0
            };
            let input_w = (ui.available_width() - reset_w - 8.0).clamp(60.0, 240.0);

            let mut path = cfg.tray_icon_path.clone().unwrap_or_default();
            if ui.add(
                egui::TextEdit::singleline(&mut path)
                    .hint_text(loc.settings_tray_icon_placeholder)
                    .desired_width(input_w)
                    .min_size(vec2(input_w, theme::CONTROL_H))
                    .vertical_align(egui::Align::Center)
                    .margin(vec2(8.0, 6.0)),
            ).changed() {
                cfg.tray_icon_path = if path.trim().is_empty() { None } else { Some(path) };
                *changed = true;
            }

            if has_reset && widgets::button(ui, loc.settings_btn_reset, Kind::Secondary, 0.0).clicked() {
                cfg.tray_icon_path = None;
                *changed = true;
            }
        });
    });

    widgets::hrule(ui);

    // 3. Hide Tray Icon Toggle
    render_setting_row(ui, loc.settings_hide_tray_icon_title, loc.settings_hide_tray_icon_desc, t, |ui| {
        if widgets::toggle_switch(ui, &mut cfg.hide_tray_icon).changed() {
            *changed = true;
        }
    });

    widgets::hrule(ui);

    // 4. Mask Daemon as System Service Toggle
    render_setting_row(ui, loc.settings_mask_daemon_title, loc.settings_mask_daemon_desc, t, |ui| {
        if widgets::toggle_switch(ui, &mut cfg.mask_daemon_process).changed() {
            *changed = true;
            let _ = cfg.save_to_file(crate::config::AppConfig::config_path());
            #[cfg(windows)]
            {
                let mask = cfg.mask_daemon_process;
                let _ = crate::ipc::client::IpcClient::call(&crate::ipc::protocol::IpcRequest::ShutdownDaemon);
                for _ in 0..30 {
                    if !crate::ipc::client::IpcClient::is_daemon_running() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                if mask {
                    let _ = crate::daemon::pe_mask::prepare_masked_daemon();
                } else {
                    crate::daemon::pe_mask::cleanup_masked_daemon();
                }
                crate::ensure_daemon_running();
                if current_autorun {
                    let _ = crate::autorun::set_autorun(crate::autorun::APP_NAME, true);
                }
            }
        }
    });

    widgets::hrule(ui);

    // 5. Open Configuration and Storage Folder
    render_setting_row(ui, loc.settings_config_folder_title, loc.settings_config_folder_desc, t, |ui| {
        if widgets::button(ui, loc.settings_btn_open_folder, Kind::Secondary, 0.0).clicked() {
            let dir = crate::config::AppConfig::config_dir();
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("explorer").arg(&dir).spawn();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
            }
        }
    });
}

fn render_appearance_content(
    ui: &mut Ui,
    state: &mut GuiState,
    cfg: &mut crate::config::AppConfig,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // 1. Interface Language
    render_setting_row(ui, loc.settings_language_title, loc.settings_language_desc, t, |ui| {
        let mut lang_idx = match state.language {
            crate::i18n::Language::English => 0usize,
            crate::i18n::Language::Russian => 1usize,
        };
        let lang_options = [
            (0usize, widgets::Cell::new("ENGLISH", "English interface")),
            (1usize, widgets::Cell::new("РУССКИЙ", "Русский интерфейс")),
        ];

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

    // 2. Color Theme with visual palette cards
    render_setting_row(ui, loc.settings_theme_title, loc.settings_theme_desc, t, |ui| {
        let themes = [
            (
                crate::config::AppTheme::Dark,
                loc.settings_theme_dark,
                Color32::from_rgb(0x12, 0x13, 0x16),
                Color32::from_rgb(0xf2, 0xf1, 0xee),
                Color32::from_rgb(0x2d, 0x81, 0xff),
            ),
            (
                crate::config::AppTheme::Light,
                loc.settings_theme_light,
                Color32::from_rgb(0xf2, 0xf1, 0xee),
                Color32::from_rgb(0x10, 0x10, 0x14),
                Color32::from_rgb(0x2d, 0x81, 0xff),
            ),
            (
                crate::config::AppTheme::Chocolate,
                loc.settings_theme_chocolate,
                Color32::from_rgb(0x20, 0x17, 0x14),
                Color32::from_rgb(0xf2, 0xe8, 0xdc),
                Color32::from_rgb(0xd9, 0x77, 0x36),
            ),
        ];

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            for (thm, label, bg_col, ink_col, accent_col) in themes {
                let is_selected = state.theme == thm;
                let card_size = vec2(90.0, 40.0);
                let (rect, resp) = ui.allocate_exact_size(card_size, Sense::click());
                if resp.clicked() && !is_selected {
                    state.set_theme(thm);
                    crate::gui::theme::apply_theme(ui.ctx(), thm);
                    cfg.theme = thm;
                    let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::SaveConfig(cfg.clone()));
                }

                let hovered = resp.hovered();
                let border_stroke = if is_selected {
                    Stroke::new(2.0, t.accent)
                } else if hovered {
                    Stroke::new(1.0, t.line_strong)
                } else {
                    t.hairline()
                };

                ui.painter().rect(rect, CornerRadius::ZERO, bg_col, border_stroke, egui::StrokeKind::Inside);

                // Accent dot + theme name
                let dot_center = pos2(rect.left() + 14.0, rect.center().y);
                ui.painter().circle_filled(dot_center, 4.0, accent_col);

                let text_pos = pos2(rect.left() + 24.0, rect.center().y);
                let galley = widgets::caps(ui, label, 10.0, theme::mono_bold(), 1.0, ink_col);
                widgets::paint_left(ui, Rect::from_min_size(pos2(text_pos.x, text_pos.y - galley.size().y / 2.0), galley.size()), galley, ink_col);

                resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            }
        });
    });
}

fn render_engine_content(
    ui: &mut Ui,
    cfg: &mut crate::config::AppConfig,
    changed: &mut bool,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // 1. Process Detection Method
    render_setting_row(ui, loc.settings_engine_mode_title, loc.settings_engine_mode_desc, t, |ui| {
        let mut mode_idx = if cfg.monitoring_mode.to_lowercase() == "wmi" { 1usize } else { 0usize };
        let mode_options = [
            (0usize, widgets::Cell::new(loc.settings_mode_polling, loc.settings_mode_polling_tip)),
            (1usize, widgets::Cell::new(loc.settings_mode_wmi, loc.settings_mode_wmi_tip)),
        ];

        if widgets::cells(ui, &mut mode_idx, &mode_options) {
            cfg.monitoring_mode = if mode_idx == 1 { "wmi".to_string() } else { "polling".to_string() };
            *changed = true;
        }
    });

    // 2. Polling Scan Interval (only if polling mode is active)
    if cfg.monitoring_mode.to_lowercase() == "polling" {
        widgets::hrule(ui);
        render_setting_row(ui, loc.settings_poll_interval_title, loc.settings_poll_interval_desc, t, |ui| {
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

fn render_updates_content(
    ui: &mut Ui,
    state: &mut GuiState,
    cfg: &mut crate::config::AppConfig,
    changed: &mut bool,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    // 1. Check on Startup Toggle
    render_setting_row(ui, loc.settings_updates_startup_title, loc.settings_updates_startup_desc, t, |ui| {
        if widgets::toggle_switch(ui, &mut cfg.check_updates_on_startup).changed() {
            *changed = true;
        }
    });

    widgets::hrule(ui);

    // 2. Installed Version & Manual Check
    let version_subtitle = format!("Process Handler v{}", env!("CARGO_PKG_VERSION"));
    render_setting_row(ui, loc.settings_updates_check_title, &version_subtitle, t, |ui| {
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
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_tabs_rendering() {
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
            active_overlay_ids: std::collections::HashSet::new(),
            active_test_rule_ids: std::collections::HashSet::new(),
            analytics: Default::default(),
        });

        // Test rendering across all 4 tabs
        let all_tabs = [
            SettingsTab::General,
            SettingsTab::Appearance,
            SettingsTab::Engine,
            SettingsTab::Updates,
        ];

        for tab in all_tabs {
            state.settings_tab = tab;
            let mut out = ctx.run_ui(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(900.0);
                    render_settings_view(ui, &mut state);
                });
            });
            out.textures_delta.clear();
        }

        // Test responsive compact layout (< 620px width)
        let mut out_compact = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(500.0);
                render_settings_view(ui, &mut state);
            });
        });
        out_compact.textures_delta.clear();

        assert_eq!(state.settings_tab, SettingsTab::Updates);

        // Verify General tab renders full card height without being clipped
        state.settings_tab = SettingsTab::General;
        let mut general_height = 0.0;
        let mut out = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(900.0);
                let initial_top = ui.cursor().top();
                render_settings_view(ui, &mut state);
                general_height = ui.cursor().top() - initial_top;
            });
        });
        out.textures_delta.clear();
        assert!(general_height > 250.0, "Settings content card should not be clipped; was {general_height}");
    }
}
