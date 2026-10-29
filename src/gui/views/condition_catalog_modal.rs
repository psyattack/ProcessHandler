//! Condition Catalog Modal:
//! Studio modal for browsing and selecting rule pre-conditions
//! with categorized tabs, instant fuzzy keyword search, and rich capability cards.

use egui::{
    pos2, vec2, Color32, CornerRadius, FontId, Margin, Rect, RichText, Sense, Stroke,
};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Glyph};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    ProcessMatchMode, RuleCondition, SystemEventType,
};

struct ConditionCatalogItem {
    category_idx: usize, // 1: Processes, 2: System, 3: Time, 4: Events
    badge: &'static str,
    title: fn(&crate::i18n::Strings) -> &'static str,
    description: fn(&crate::i18n::Strings) -> &'static str,
    glyph: &'static str,
    tint: fn(&Tokens) -> Color32,
    keywords: &'static [&'static str],
    constructor: fn() -> RuleCondition,
}

const CONDITION_CATALOG_ITEMS: &[ConditionCatalogItem] = &[
    // 1. Process & Window
    ConditionCatalogItem {
        category_idx: 1,
        badge: "PROCESS",
        title: |loc| loc.catalog_cond_process_title,
        description: |loc| loc.catalog_cond_process_desc,
        glyph: egui_phosphor::regular::CPU,
        tint: |t| t.good,
        keywords: &["process", "running", "closed", "state", "status", "exe", "app", "процесс", "запущен", "закрыт", "состояние", "работает"],
        constructor: || RuleCondition::ProcessRunning {
            process_name: String::new(),
            match_mode: ProcessMatchMode::Exact,
            must_be_running: true,
        },
    },
    ConditionCatalogItem {
        category_idx: 1,
        badge: "WINDOW",
        title: |loc| loc.catalog_cond_window_title,
        description: |loc| loc.catalog_cond_window_desc,
        glyph: egui_phosphor::regular::EYE,
        tint: |t| t.accent_ink,
        keywords: &["window", "focus", "active", "foreground", "background", "blur", "окно", "фокус", "активно", "фон"],
        constructor: || RuleCondition::WindowFocused {
            process_name: String::new(),
            match_mode: ProcessMatchMode::Exact,
            must_be_focused: true,
        },
    },

    // 2. Hardware & Resource
    ConditionCatalogItem {
        category_idx: 2,
        badge: "RESOURCE",
        title: |loc| loc.catalog_cond_resource_title,
        description: |loc| loc.catalog_cond_resource_desc,
        glyph: egui_phosphor::regular::CHART_LINE,
        tint: |t| t.bad,
        keywords: &["resource", "cpu", "ram", "memory", "load", "threshold", "нагрузка", "процессор", "память"],
        constructor: || RuleCondition::ResourceThreshold {
            process_name: String::new(),
            cpu_percent: Some(80.0),
            ram_mb: None,
            duration_sec: 5,
        },
    },
    ConditionCatalogItem {
        category_idx: 2,
        badge: "FILES",
        title: |loc| loc.catalog_cond_file_title,
        description: |loc| loc.catalog_cond_file_desc,
        glyph: egui_phosphor::regular::FOLDER_NOTCH_OPEN,
        tint: |t| t.good,
        keywords: &["file", "folder", "directory", "path", "exists", "файл", "папка", "слежение"],
        constructor: || RuleCondition::FileWatcher {
            path: String::new(),
            filter_extension: None,
            must_exist: true,
        },
    },

    // 3. Time & Schedule
    ConditionCatalogItem {
        category_idx: 3,
        badge: "TIME",
        title: |loc| loc.catalog_cond_time_title,
        description: |loc| loc.catalog_cond_time_desc,
        glyph: egui_phosphor::regular::CLOCK,
        tint: |t| t.good,
        keywords: &["time", "hours", "days", "window", "schedule", "время", "часы", "дни", "интервал"],
        constructor: || RuleCondition::TimeRange {
            start_time: "09:00".into(),
            end_time: "18:00".into(),
            days_of_week: vec![1, 2, 3, 4, 5],
        },
    },

    // 4. System Events & States
    ConditionCatalogItem {
        category_idx: 4,
        badge: "EVENT",
        title: |loc| loc.catalog_cond_lock_title,
        description: |loc| loc.catalog_cond_lock_desc,
        glyph: egui_phosphor::regular::LOCK,
        tint: |t| t.accent_ink,
        keywords: &["locked", "unlocked", "lock", "screen", "session", "блокировка", "разблокировка", "экран"],
        constructor: || RuleCondition::Event {
            event_type: SystemEventType::SessionLock,
            idle_duration_sec: 0,
        },
    },
    ConditionCatalogItem {
        category_idx: 4,
        badge: "POWER",
        title: |loc| loc.catalog_cond_sleep_title,
        description: |loc| loc.catalog_cond_sleep_desc,
        glyph: egui_phosphor::regular::MOON,
        tint: |t| t.accent_ink,
        keywords: &["sleep", "wake", "suspend", "hibernate", "resume", "сон", "пробуждение", "гибернация"],
        constructor: || RuleCondition::Event {
            event_type: SystemEventType::SystemSleep,
            idle_duration_sec: 0,
        },
    },
    ConditionCatalogItem {
        category_idx: 4,
        badge: "POWER",
        title: |loc| loc.catalog_cond_power_title,
        description: |loc| loc.catalog_cond_power_desc,
        glyph: egui_phosphor::regular::PLUG,
        tint: |t| t.good,
        keywords: &["power", "ac", "wall", "charger", "battery", "laptop", "сеть", "розетка", "зарядка", "батарея"],
        constructor: || RuleCondition::Event {
            event_type: SystemEventType::PowerSourceAc,
            idle_duration_sec: 0,
        },
    },
    ConditionCatalogItem {
        category_idx: 4,
        badge: "IDLE",
        title: |loc| loc.catalog_cond_idle_title,
        description: |loc| loc.catalog_cond_idle_desc,
        glyph: egui_phosphor::regular::HOURGLASS,
        tint: |t| t.muted,
        keywords: &["idle", "afk", "inactive", "away", "active", "input", "бездействие", "неактивность", "активность"],
        constructor: || RuleCondition::Event {
            event_type: SystemEventType::UserIdle,
            idle_duration_sec: 300,
        },
    },
    ConditionCatalogItem {
        category_idx: 4,
        badge: "DISPLAY",
        title: |loc| loc.catalog_cond_display_title,
        description: |loc| loc.catalog_cond_display_desc,
        glyph: egui_phosphor::regular::DESKTOP,
        tint: |t| t.good,
        keywords: &["monitor", "display", "screen", "on", "off", "монитор", "дисплей", "экран"],
        constructor: || RuleCondition::Event {
            event_type: SystemEventType::MonitorOn,
            idle_duration_sec: 0,
        },
    },
    ConditionCatalogItem {
        category_idx: 4,
        badge: "DEVICE",
        title: |loc| loc.catalog_cond_device_title,
        description: |loc| loc.catalog_cond_device_desc,
        glyph: egui_phosphor::regular::HARD_DRIVES,
        tint: |t| t.good,
        keywords: &["device", "usb", "hardware", "connect", "disconnect", "устройство", "подключение", "отключение"],
        constructor: || RuleCondition::Event {
            event_type: SystemEventType::DeviceConnected,
            idle_duration_sec: 0,
        },
    },
    ConditionCatalogItem {
        category_idx: 4,
        badge: "NETWORK",
        title: |loc| loc.catalog_cond_network_title,
        description: |loc| loc.catalog_cond_network_desc,
        glyph: egui_phosphor::regular::WIFI_HIGH,
        tint: |t| t.good,
        keywords: &["network", "wifi", "ethernet", "internet", "online", "offline", "сеть", "интернет", "подключение", "отключение"],
        constructor: || RuleCondition::Event {
            event_type: SystemEventType::NetworkConnected,
            idle_duration_sec: 0,
        },
    },
];

pub fn render_condition_catalog_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.condition_catalog.is_some();
    let active_modal = state
        .condition_catalog
        .as_ref()
        .or(state.last_condition_catalog.as_ref())
        .cloned();

    let Some(mut modal_state) = active_modal else { return };
    let loc = state.loc();
    let mut chosen_condition: Option<RuleCondition> = None;

    let modal_res = Modal::new("condition-catalog-modal", is_open)
        .width_fraction(0.74)
        .height_fraction(0.76)
        .min_size(vec2(720.0, 500.0))
        .max_size(vec2(840.0, 640.0))
        .header_height(46.0)
        .show(ctx, &t, |frame| {
            // ---------------- 1. HEADER ----------------
            frame.header(|ui, close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    widgets::hud(ui, loc.catalog_condition_modal_title, t.ink);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_cancel).clicked() {
                            *close_req = true;
                        }
                    });
                });
            });

            // ---------------- 2. BODY (CATEGORIES + CONDITION CARDS) ----------------
            frame.body(|ui| {
                let body_rect = ui.max_rect();
                let sidebar_w = 195.0;
                let sidebar_rect = Rect::from_min_size(
                    pos2(body_rect.min.x, body_rect.top() + 1.0),
                    vec2(sidebar_w, body_rect.height() - 2.0),
                );
                let cards_rect = Rect::from_min_max(
                    pos2(sidebar_rect.right() + 1.0, body_rect.top() + 1.0),
                    pos2(body_rect.right() - 1.0, body_rect.bottom() - 1.0),
                );

                // Body background surfaces & divider
                let p = ui.painter();
                p.rect_filled(sidebar_rect, CornerRadius::ZERO, t.bg);
                p.rect_filled(cards_rect, CornerRadius::ZERO, t.sunken);

                // Seamless 1px geometric vertical divider without line-cap protrusion (DPI-independent)
                let v_divider = Rect::from_min_max(
                    pos2(sidebar_rect.right(), body_rect.top() + 1.0),
                    pos2(sidebar_rect.right() + 1.0, body_rect.bottom() - 1.0),
                );
                p.rect_filled(v_divider, CornerRadius::ZERO, t.hairline().color);

                // Left: Categories List
                ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar_rect), |ui| {
                    egui::Frame::new()
                        .inner_margin(Margin::symmetric(10, 10))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;

                            let count_for = |cat_idx: usize| -> usize {
                                CONDITION_CATALOG_ITEMS
                                    .iter()
                                    .filter(|i| cat_idx == 0 || i.category_idx == cat_idx)
                                    .count()
                            };

                            let categories = [
                                (0, loc.catalog_cat_all, egui_phosphor::regular::SPARKLE, count_for(0)),
                                (1, loc.catalog_cat_process, egui_phosphor::regular::CPU, count_for(1)),
                                (2, loc.catalog_cat_system, egui_phosphor::regular::HARD_DRIVE, count_for(2)),
                                (3, loc.catalog_cat_time, egui_phosphor::regular::CLOCK, count_for(3)),
                                (4, loc.catalog_cat_events, egui_phosphor::regular::LIGHTNING, count_for(4)),
                            ];

                            for (cat_idx, cat_name, icon, count) in categories {
                                let is_active = modal_state.selected_category == cat_idx;
                                let bg = if is_active { t.accent_ink.gamma_multiply(0.12) } else { Color32::TRANSPARENT };
                                let fg = if is_active { t.accent_ink } else { t.muted };

                                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                                if resp.hovered() && !is_active {
                                    ui.painter().rect_filled(rect, CornerRadius::same(4), t.sunken);
                                }
                                if is_active {
                                    ui.painter().rect_filled(rect, CornerRadius::same(4), bg);
                                }

                                ui.painter().text(
                                    pos2(rect.left() + 10.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    icon,
                                    FontId::new(14.0, theme::phosphor()),
                                    fg,
                                );

                                ui.painter().text(
                                    pos2(rect.left() + 32.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    cat_name,
                                    FontId::new(12.5, theme::semibold()),
                                    fg,
                                );

                                ui.painter().text(
                                    pos2(rect.right() - 8.0, rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    format!("{count}"),
                                    FontId::new(11.0, theme::mono()),
                                    t.faint,
                                );

                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    modal_state.selected_category = cat_idx;
                                }
                            }
                        });
                });

                // Filter conditions
                let query_lower = modal_state.search_query.trim().to_lowercase();
                let filtered_items: Vec<&ConditionCatalogItem> = CONDITION_CATALOG_ITEMS
                    .iter()
                    .filter(|item| {
                        if !query_lower.is_empty() {
                            let title = (item.title)(loc).to_lowercase();
                            let desc = (item.description)(loc).to_lowercase();
                            let badge = item.badge.to_lowercase();
                            let match_kw = item.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower));
                            if !title.contains(&query_lower) && !desc.contains(&query_lower) && !badge.contains(&query_lower) && !match_kw {
                                return false;
                            }
                        }

                        if modal_state.selected_category != 0 && query_lower.is_empty() && item.category_idx != modal_state.selected_category {
                            return false;
                        }
                        true
                    })
                    .collect();

                // Right: Condition Cards List
                ui.scope_builder(egui::UiBuilder::new().max_rect(cards_rect), |ui| {
                    ui.add_space(10.0);

                    // Search box placed directly above cards scroll area
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(10.0);
                        let search_w = (ui.available_width() - 10.0).max(60.0);
                        ui.allocate_ui_with_layout(
                            vec2(search_w, 32.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                widgets::catalog_search_box(ui, &mut modal_state.search_query, loc.catalog_condition_search_hint);
                            },
                        );
                    });
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .id_salt("condition_catalog_cards_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if filtered_items.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(60.0);
                                    ui.label(
                                        RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                                            .family(theme::phosphor())
                                            .size(32.0)
                                            .color(t.faint),
                                    );
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new(loc.catalog_no_matches)
                                            .family(theme::medium())
                                            .size(13.0)
                                            .color(t.muted),
                                    );
                                });
                            } else {
                                let total_w = ui.available_width() - 20.0;
                                let col_count = 2;
                                let gap = 10.0;
                                let card_w = ((total_w - gap) / (col_count as f32)).max(160.0);
                                let card_h = 68.0;

                                for chunk in filtered_items.chunks(col_count) {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        ui.add_space(10.0);
                                        for (idx, item) in chunk.iter().enumerate() {
                                            if idx > 0 {
                                                ui.add_space(gap);
                                            }
                                            let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
                                            let is_hovered = resp.hovered();

                                            let bg_col = if is_hovered { t.sunken } else { t.bg };
                                            let stroke_col = if is_hovered { t.accent_ink } else { t.line };

                                            ui.painter().rect_filled(rect, CornerRadius::same(6), bg_col);
                                            ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, stroke_col), egui::StrokeKind::Inside);

                                            // Icon Chip on Left
                                            let chip_rect = Rect::from_center_size(
                                                pos2(rect.left() + 26.0, rect.center().y),
                                                vec2(34.0, 34.0),
                                            );
                                            let item_tint = (item.tint)(&t);
                                            ui.painter().rect_filled(chip_rect, CornerRadius::same(6), item_tint.gamma_multiply(0.18));
                                            ui.painter().text(
                                                chip_rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                item.glyph,
                                                FontId::new(18.0, theme::phosphor()),
                                                item_tint,
                                            );

                                            // Text content on Right (vertically centered relative to card & icon chip)
                                            let text_left = rect.left() + 52.0;
                                            let title_text = (item.title)(loc);
                                            let desc_text = (item.description)(loc);
                                            let desc_w = (rect.right() - text_left - 10.0).max(40.0);

                                            let title_galley = ui.painter().layout_no_wrap(
                                                title_text.to_string(),
                                                FontId::new(13.0, theme::semibold()),
                                                t.ink,
                                            );
                                            let desc_galley = ui.painter().layout(
                                                desc_text.to_string(),
                                                FontId::new(11.0, theme::medium()),
                                                t.muted,
                                                desc_w,
                                            );

                                            let title_h = title_galley.size().y;
                                            let desc_h = desc_galley.size().y;
                                            let text_spacing = 3.0;
                                            let total_text_h = title_h + text_spacing + desc_h;
                                            let text_start_y = (rect.center().y - total_text_h * 0.5).round();

                                            ui.painter().galley(pos2(text_left, text_start_y), title_galley, t.ink);
                                            ui.painter().galley(pos2(text_left, text_start_y + title_h + text_spacing), desc_galley, t.muted);

                                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                                chosen_condition = Some((item.constructor)());
                                            }
                                        }
                                    });
                                    ui.add_space(6.0);
                                }
                            }
                            ui.add_space(10.0);
                        });
                });
            });
        });

    if let Some(cond) = chosen_condition {
        state.add_condition_from_catalog(cond);
    } else if let Some(res) = modal_res {
        if res.close_requested {
            state.close_condition_catalog();
        } else if is_open {
            state.condition_catalog = Some(modal_state.clone());
            state.last_condition_catalog = Some(modal_state);
        }
    } else if !is_open {
        state.last_condition_catalog = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_catalog_items_and_categories() {
        // Category 1: Processes (Process State, Window Focus)
        let cat1_items: Vec<_> = CONDITION_CATALOG_ITEMS.iter().filter(|i| i.category_idx == 1).collect();
        assert_eq!(cat1_items.len(), 2, "Category 1 should have exactly 2 combined items");
        assert_eq!(cat1_items[0].badge, "PROCESS");
        assert_eq!(cat1_items[1].badge, "WINDOW");

        // Category 4: Events (7 stateful system events from Trigger library)
        let cat4_items: Vec<_> = CONDITION_CATALOG_ITEMS.iter().filter(|i| i.category_idx == 4).collect();
        assert_eq!(cat4_items.len(), 7, "Category 4 should have 7 system events");

        // Test constructor outputs
        for item in CONDITION_CATALOG_ITEMS {
            let cond = (item.constructor)();
            match &cond {
                RuleCondition::ProcessRunning { .. } => assert_eq!(item.category_idx, 1),
                RuleCondition::WindowFocused { .. } => assert_eq!(item.category_idx, 1),
                RuleCondition::ResourceThreshold { .. } | RuleCondition::FileWatcher { .. } => {
                    assert_eq!(item.category_idx, 2);
                }
                RuleCondition::TimeRange { .. } => assert_eq!(item.category_idx, 3),
                RuleCondition::Event { .. } => assert_eq!(item.category_idx, 4),
            }
        }
    }

    #[test]
    fn test_render_condition_catalog_modal_no_crash() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);

        let mut state = GuiState::new();
        state.open_condition_catalog();

        for _ in 0..3 {
            let mut out = ctx.run_ui(Default::default(), |ctx| {
                render_condition_catalog_modal(ctx, &mut state);
            });
            out.textures_delta.clear();
        }

        state.close_condition_catalog();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ctx| {
                render_condition_catalog_modal(ctx, &mut state);
            });
            out.textures_delta.clear();
        }
    }
}
