//! Trigger Catalog Modal:
//! Studio modal for browsing and selecting rule triggers
//! with categorized tabs, instant fuzzy keyword search, and rich capability cards.

use egui::{
    pos2, vec2, Color32, CornerRadius, FontId, Margin, Rect, RichText, Sense, Stroke,
};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Glyph};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    ProcessMatchMode, ScheduleType, SystemEventType, Trigger,
};

struct TriggerCatalogItem {
    category_idx: usize, // 1: Processes, 2: Schedule, 3: System, 4: Events
    badge: &'static str,
    title: fn(&crate::i18n::Strings) -> &'static str,
    description: fn(&crate::i18n::Strings) -> &'static str,
    glyph: &'static str,
    tint: fn(&Tokens) -> Color32,
    keywords: &'static [&'static str],
    constructor: fn() -> Option<Trigger>,
}

const TRIGGER_CATALOG_ITEMS: &[TriggerCatalogItem] = &[
    // 0. Manual mode (On Demand)
    TriggerCatalogItem {
        category_idx: 3,
        badge: "MANUAL",
        title: |loc| loc.catalog_trigger_manual_title,
        description: |loc| loc.catalog_trigger_manual_desc,
        glyph: egui_phosphor::regular::PLAY_CIRCLE,
        tint: |t| t.accent_ink,
        keywords: &["manual", "demand", "run", "button", "click", "ручной", "кнопка", "старт", "клик"],
        constructor: || Some(Trigger::Manual),
    },

    // 1. Process Management
    TriggerCatalogItem {
        category_idx: 1,
        badge: "PROCESS",
        title: |loc| loc.catalog_trigger_process_title,
        description: |loc| loc.catalog_trigger_process_desc,
        glyph: egui_phosphor::regular::CPU,
        tint: |t| t.good,
        keywords: &["process", "launch", "start", "run", "exe", "app", "процесс", "запуск", "старт", "приложение"],
        constructor: || Some(Trigger::Process {
            process_name: String::new(),
            match_mode: ProcessMatchMode::Exact,
            single_instance: false,
        }),
    },
    TriggerCatalogItem {
        category_idx: 1,
        badge: "WINDOW",
        title: |loc| loc.catalog_trigger_window_title,
        description: |loc| loc.catalog_trigger_window_desc,
        glyph: egui_phosphor::regular::EYE,
        tint: |t| t.accent_ink,
        keywords: &["window", "focus", "foreground", "active", "blur", "окно", "фокус", "активное", "свернуто"],
        constructor: || Some(Trigger::WindowFocus {
            process_name: String::new(),
            on_focus_gained: true,
            on_focus_lost: false,
        }),
    },

    // 2. Schedule & Time
    TriggerCatalogItem {
        category_idx: 2,
        badge: "TIMER",
        title: |loc| loc.catalog_trigger_interval_title,
        description: |loc| loc.catalog_trigger_interval_desc,
        glyph: egui_phosphor::regular::TIMER,
        tint: |t| t.accent_ink,
        keywords: &["timer", "interval", "repeat", "seconds", "minutes", "hours", "таймер", "интервал", "повтор"],
        constructor: || Some(Trigger::Schedule {
            schedule_type: ScheduleType::Interval,
            interval_sec: 60,
            daily_time: None,
            days_of_week: Vec::new(),
            day_of_month: None,
            delay_sec: 0,
            target_datetime: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 2,
        badge: "SCHEDULE",
        title: |loc| loc.catalog_trigger_daily_title,
        description: |loc| loc.catalog_trigger_daily_desc,
        glyph: egui_phosphor::regular::CALENDAR,
        tint: |t| t.good,
        keywords: &["daily", "time", "clock", "hour", "alarm", "ежедневно", "время", "будильник", "часы"],
        constructor: || Some(Trigger::Schedule {
            schedule_type: ScheduleType::Daily,
            interval_sec: 0,
            daily_time: Some("12:00:00".into()),
            days_of_week: Vec::new(),
            day_of_month: None,
            delay_sec: 0,
            target_datetime: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 2,
        badge: "ONE-SHOT",
        title: |loc| loc.catalog_trigger_oneshot_title,
        description: |loc| loc.catalog_trigger_oneshot_desc,
        glyph: egui_phosphor::regular::CALENDAR_CHECK,
        tint: |t| t.muted,
        keywords: &["oneshot", "date", "calendar", "future", "once", "однократно", "дата", "календарь"],
        constructor: || Some(Trigger::Schedule {
            schedule_type: ScheduleType::OneShot,
            interval_sec: 0,
            daily_time: None,
            days_of_week: Vec::new(),
            day_of_month: None,
            delay_sec: 0,
            target_datetime: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 2,
        badge: "MONTHLY",
        title: |loc| loc.catalog_trigger_monthly_title,
        description: |loc| loc.catalog_trigger_monthly_desc,
        glyph: egui_phosphor::regular::CALENDAR_BLANK,
        tint: |t| t.accent_ink,
        keywords: &["monthly", "month", "day", "ежемесячно", "месяц", "число"],
        constructor: || Some(Trigger::Schedule {
            schedule_type: ScheduleType::Monthly,
            interval_sec: 0,
            daily_time: Some("12:00:00".into()),
            days_of_week: Vec::new(),
            day_of_month: Some(1),
            delay_sec: 0,
            target_datetime: None,
        }),
    },

    // 3. Hardware & System
    TriggerCatalogItem {
        category_idx: 3,
        badge: "HOTKEY",
        title: |loc| loc.catalog_trigger_hotkey_title,
        description: |loc| loc.catalog_trigger_hotkey_desc,
        glyph: egui_phosphor::regular::COMMAND,
        tint: |t| t.accent_ink,
        keywords: &["hotkey", "shortcut", "keyboard", "mouse", "macro", "клавиша", "хоткей", "сочетание", "бинд"],
        constructor: || Some(Trigger::GlobalHotkey {
            key_combination: "Ctrl+Shift+H".into(),
            hold_mode: false,
            repeat_while_held: false,
        }),
    },
    TriggerCatalogItem {
        category_idx: 3,
        badge: "RESOURCE",
        title: |loc| loc.catalog_trigger_resource_title,
        description: |loc| loc.catalog_trigger_resource_desc,
        glyph: egui_phosphor::regular::CHART_LINE,
        tint: |t| t.bad,
        keywords: &["resource", "cpu", "ram", "memory", "load", "threshold", "нагрузка", "процессор", "память"],
        constructor: || Some(Trigger::ResourceThreshold {
            process_name: String::new(),
            cpu_percent: Some(80.0),
            ram_mb: None,
            duration_sec: 5,
        }),
    },
    TriggerCatalogItem {
        category_idx: 3,
        badge: "FILES",
        title: |loc| loc.catalog_trigger_file_title,
        description: |loc| loc.catalog_trigger_file_desc,
        glyph: egui_phosphor::regular::FOLDER_NOTCH_OPEN,
        tint: |t| t.good,
        keywords: &["file", "folder", "directory", "watch", "created", "modified", "файл", "папка", "слежение"],
        constructor: || Some(Trigger::FileWatcher {
            folder_path: String::new(),
            filter_extension: None,
            on_created: true,
            on_modified: false,
            on_deleted: false,
        }),
    },
    TriggerCatalogItem {
        category_idx: 3,
        badge: "WEBHOOK",
        title: |loc| loc.catalog_trigger_webhook_title,
        description: |loc| loc.catalog_trigger_webhook_desc,
        glyph: egui_phosphor::regular::GLOBE_HEMISPHERE_WEST,
        tint: |t| t.accent_ink,
        keywords: &["webhook", "http", "api", "rest", "curl", "url", "вебхук", "сеть"],
        constructor: || Some(Trigger::Webhook {
            endpoint_path: String::new(),
        }),
    },

    // 4. Lifecycle & System Events
    TriggerCatalogItem {
        category_idx: 4,
        badge: "LIFECYCLE",
        title: |loc| loc.catalog_trigger_lifecycle_title,
        description: |loc| loc.catalog_trigger_lifecycle_desc,
        glyph: egui_phosphor::regular::ROCKET_LAUNCH,
        tint: |t| t.accent_ink,
        keywords: &["startup", "shutdown", "start", "close", "exit", "boot", "launch", "старт", "запуск", "закрытие", "выход", "завершение"],
        constructor: || Some(Trigger::Schedule {
            schedule_type: ScheduleType::AppStartup,
            interval_sec: 0,
            daily_time: None,
            days_of_week: Vec::new(),
            day_of_month: None,
            delay_sec: 0,
            target_datetime: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 4,
        badge: "EVENT",
        title: |loc| loc.catalog_trigger_lock_title,
        description: |loc| loc.catalog_trigger_lock_desc,
        glyph: egui_phosphor::regular::LOCK,
        tint: |t| t.accent_ink,
        keywords: &["lock", "unlock", "session", "screen", "workstation", "блокировка", "разблокировка", "экран"],
        constructor: || Some(Trigger::SystemEvent {
            event_type: SystemEventType::SessionLock,
            idle_duration_sec: 0,
            device_id_filter: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 4,
        badge: "POWER",
        title: |loc| loc.catalog_trigger_sleep_title,
        description: |loc| loc.catalog_trigger_sleep_desc,
        glyph: egui_phosphor::regular::MOON,
        tint: |t| t.accent_ink,
        keywords: &["sleep", "wake", "suspend", "hibernate", "resume", "сон", "пробуждение", "гибернация"],
        constructor: || Some(Trigger::SystemEvent {
            event_type: SystemEventType::SystemSleep,
            idle_duration_sec: 0,
            device_id_filter: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 4,
        badge: "POWER",
        title: |loc| loc.catalog_trigger_power_title,
        description: |loc| loc.catalog_trigger_power_desc,
        glyph: egui_phosphor::regular::PLUG,
        tint: |t| t.good,
        keywords: &["power", "ac", "wall", "charger", "battery", "laptop", "сеть", "розетка", "зарядка", "батарея"],
        constructor: || Some(Trigger::SystemEvent {
            event_type: SystemEventType::PowerSourceAc,
            idle_duration_sec: 0,
            device_id_filter: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 4,
        badge: "IDLE",
        title: |loc| loc.catalog_trigger_idle_title,
        description: |loc| loc.catalog_trigger_idle_desc,
        glyph: egui_phosphor::regular::HOURGLASS,
        tint: |t| t.muted,
        keywords: &["idle", "afk", "inactive", "away", "active", "input", "бездействие", "неактивность", "активность"],
        constructor: || Some(Trigger::SystemEvent {
            event_type: SystemEventType::UserIdle,
            idle_duration_sec: 300,
            device_id_filter: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 4,
        badge: "DEVICE",
        title: |loc| loc.catalog_trigger_device_title,
        description: |loc| loc.catalog_trigger_device_desc,
        glyph: egui_phosphor::regular::HARD_DRIVES,
        tint: |t| t.good,
        keywords: &["device", "usb", "hardware", "connect", "disconnect", "устройство", "подключение", "отключение"],
        constructor: || Some(Trigger::SystemEvent {
            event_type: SystemEventType::DeviceConnected,
            idle_duration_sec: 0,
            device_id_filter: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 4,
        badge: "DISPLAY",
        title: |loc| loc.catalog_trigger_display_title,
        description: |loc| loc.catalog_trigger_display_desc,
        glyph: egui_phosphor::regular::DESKTOP,
        tint: |t| t.good,
        keywords: &["monitor", "display", "screen", "on", "off", "монитор", "дисплей", "экран"],
        constructor: || Some(Trigger::SystemEvent {
            event_type: SystemEventType::MonitorOn,
            idle_duration_sec: 0,
            device_id_filter: None,
        }),
    },
    TriggerCatalogItem {
        category_idx: 4,
        badge: "NETWORK",
        title: |loc| loc.catalog_trigger_network_title,
        description: |loc| loc.catalog_trigger_network_desc,
        glyph: egui_phosphor::regular::WIFI_HIGH,
        tint: |t| t.good,
        keywords: &["network", "wifi", "ethernet", "internet", "online", "offline", "сеть", "интернет", "подключение", "отключение"],
        constructor: || Some(Trigger::SystemEvent {
            event_type: SystemEventType::NetworkConnected,
            idle_duration_sec: 0,
            device_id_filter: None,
        }),
    },
];

pub fn render_trigger_catalog_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.trigger_catalog.is_some();
    let active_modal = state
        .trigger_catalog
        .as_ref()
        .or(state.last_trigger_catalog.as_ref())
        .cloned();

    let Some(mut modal_state) = active_modal else { return };
    let loc = state.loc();
    let mut chosen_trigger: Option<Option<Trigger>> = None;

    let modal_res = Modal::new("trigger-catalog-modal", is_open)
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
                    widgets::hud(ui, loc.catalog_trigger_modal_title, t.ink);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_cancel).clicked() {
                            *close_req = true;
                        }
                    });
                });
            });

            // ---------------- 2. BODY (CATEGORIES + TRIGGER CARDS) ----------------
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
                                TRIGGER_CATALOG_ITEMS
                                    .iter()
                                    .filter(|i| cat_idx == 0 || i.category_idx == cat_idx)
                                    .count()
                            };

                            let categories = [
                                (0, loc.catalog_cat_all, egui_phosphor::regular::SPARKLE, count_for(0)),
                                (1, loc.catalog_cat_process, egui_phosphor::regular::CPU, count_for(1)),
                                (2, loc.catalog_cat_schedule, egui_phosphor::regular::CLOCK, count_for(2)),
                                (3, loc.catalog_cat_system, egui_phosphor::regular::GEAR, count_for(3)),
                                (4, loc.catalog_cat_events, egui_phosphor::regular::BELL, count_for(4)),
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

                // Filter triggers
                let query_lower = modal_state.search_query.trim().to_lowercase();
                let filtered_items: Vec<&TriggerCatalogItem> = TRIGGER_CATALOG_ITEMS
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

                // Right: Trigger Cards List
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
                                widgets::catalog_search_box(ui, &mut modal_state.search_query, loc.catalog_trigger_search_hint);
                            },
                        );
                    });
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .id_salt("trigger_catalog_cards_scroll")
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
                                                chosen_trigger = Some((item.constructor)());
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

    if let Some(res) = modal_res {
        if res.close_requested {
            state.close_trigger_catalog();
        } else if let Some(trigger_opt) = chosen_trigger {
            state.set_trigger_from_catalog(trigger_opt);
        } else if is_open {
            state.trigger_catalog = Some(modal_state.clone());
            state.last_trigger_catalog = Some(modal_state);
        }
    } else if !is_open {
        state.last_trigger_catalog = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_catalog_items_and_categories() {
        // Category 1: Processes (Process Launch, Window Focus)
        let cat1_items: Vec<_> = TRIGGER_CATALOG_ITEMS.iter().filter(|i| i.category_idx == 1).collect();
        assert_eq!(cat1_items.len(), 2, "Category 1 should have 2 process triggers");

        // Category 2: Schedule (Interval, Daily, Exact Date, Monthly)
        let cat2_items: Vec<_> = TRIGGER_CATALOG_ITEMS.iter().filter(|i| i.category_idx == 2).collect();
        assert_eq!(cat2_items.len(), 4, "Category 2 should have 4 schedule triggers");

        // Category 3: System / Hardware (Manual, Hotkey, System Load, Folder Watcher, Webhook)
        let cat3_items: Vec<_> = TRIGGER_CATALOG_ITEMS.iter().filter(|i| i.category_idx == 3).collect();
        assert_eq!(cat3_items.len(), 5, "Category 3 should have 5 system/hardware triggers");

        // Category 4: Events & Lifecycle (8 paired items)
        let cat4_items: Vec<_> = TRIGGER_CATALOG_ITEMS.iter().filter(|i| i.category_idx == 4).collect();
        assert_eq!(cat4_items.len(), 8, "Category 4 should have 8 event/lifecycle triggers");

        // Test constructor outputs
        for item in TRIGGER_CATALOG_ITEMS {
            let trig = (item.constructor)();
            match &trig {
                Some(Trigger::Process { .. }) | Some(Trigger::WindowFocus { .. }) => {
                    assert_eq!(item.category_idx, 1);
                }
                Some(Trigger::Schedule { schedule_type, .. }) => {
                    if matches!(schedule_type, ScheduleType::AppStartup | ScheduleType::AppShutdown) {
                        assert_eq!(item.category_idx, 4);
                    } else {
                        assert_eq!(item.category_idx, 2);
                    }
                }
                Some(Trigger::Manual) | Some(Trigger::GlobalHotkey { .. }) | Some(Trigger::ResourceThreshold { .. }) | Some(Trigger::FileWatcher { .. }) | Some(Trigger::Webhook { .. }) => {
                    assert_eq!(item.category_idx, 3);
                }
                Some(Trigger::SystemEvent { .. }) => {
                    assert_eq!(item.category_idx, 4);
                }
                None => {}
            }
        }
    }

    #[test]
    fn test_render_trigger_catalog_modal_no_crash() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);

        let mut state = GuiState::new();
        state.open_trigger_catalog();

        for _ in 0..3 {
            let mut out = ctx.run_ui(Default::default(), |ctx| {
                render_trigger_catalog_modal(ctx, &mut state);
            });
            out.textures_delta.clear();
        }

        state.close_trigger_catalog();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ctx| {
                render_trigger_catalog_modal(ctx, &mut state);
            });
            out.textures_delta.clear();
        }
    }
}
