//! Action Catalog Modal:
//! Studio modal for browsing and selecting rule actions
//! with categorized tabs, instant fuzzy keyword search, and rich capability cards.

use egui::{
    Color32, CornerRadius, FontId, Margin, Rect, RichText, Sense, Stroke,
    pos2, vec2,
};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Glyph};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    Action, PowerPlanType, ProcessAudioActionType,
    SystemPowerType, WindowActionType,
};

struct CatalogItem {
    category_idx: usize, // 1: Process, 2: Audio, 3: Automation, 4: System
    badge: &'static str,
    title: fn(&crate::i18n::Strings) -> &'static str,
    description: fn(&crate::i18n::Strings) -> &'static str,
    glyph: &'static str,
    tint: fn(&Tokens) -> Color32,
    keywords: &'static [&'static str],
    constructor: fn() -> Action,
}

const CATALOG_ITEMS: [CatalogItem; 18] = [
    // 1. Process Management
    CatalogItem {
        category_idx: 1,
        badge: "PROCESS",
        title: |loc| loc.inspector_menu_launch_process,
        description: |loc| loc.catalog_desc_launch_process,
        glyph: egui_phosphor::regular::ROCKET,
        tint: |t| t.good,
        keywords: &[
            "launch", "start", "run", "exe", "process", "app", "application", "binary",
            "запуск", "старт", "приложение", "программа", "файл", "процесс", "запустить",
        ],
        constructor: || Action::StartProcess {
            file_path: String::new(),
            arguments: String::new(),
            working_directory: None,
            run_as_admin: false,
            only_if_not_running: false,
            run_hidden: false,
            close_on_exit: false,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "PROCESS",
        title: |loc| loc.inspector_menu_kill_process,
        description: |loc| loc.catalog_desc_kill_process,
        glyph: egui_phosphor::regular::STOP,
        tint: |t| t.bad,
        keywords: &[
            "kill", "terminate", "stop", "close", "end", "taskkill", "force",
            "завершить", "убить", "закрыть", "остановить", "процесс", "снять",
        ],
        constructor: || Action::KillProcess {
            process_name: String::new(),
            kill_tree: true,
            force: true,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "WINDOW",
        title: |loc| loc.action_window_title,
        description: |loc| loc.action_window_desc,
        glyph: egui_phosphor::regular::APP_WINDOW,
        tint: |t| t.accent_ink,
        keywords: &[
            "window", "minimize", "maximize", "restore", "hide", "show", "transparency", "snap", "always on top",
            "окно", "свернуть", "развернуть", "скрыть", "прозрачность", "позиция", "поверх всех",
        ],
        constructor: || Action::Window {
            target_process: None,
            window_action: WindowActionType::Minimize,
            x: None,
            y: None,
            width: None,
            height: None,
            transparency: None,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "TUNING",
        title: |loc| loc.action_tuning_title,
        description: |loc| loc.action_tuning_desc,
        glyph: egui_phosphor::regular::CPU,
        tint: |t| t.good,
        keywords: &[
            "tuning", "priority", "affinity", "cpu", "cores", "trim", "memory", "working set",
            "приоритет", "ядра", "память", "тюнинг", "оптимизация", "процесс",
        ],
        constructor: || Action::ProcessTuning {
            target_process: None,
            priority: None,
            affinity_mask: None,
            trim_memory: true,
        },
    },

    // 2. Audio & Media
    CatalogItem {
        category_idx: 2,
        badge: "AUDIO / MIC",
        title: |loc| loc.action_audio_control_title,
        description: |loc| loc.action_audio_control_desc,
        glyph: egui_phosphor::regular::SPEAKER_HIGH,
        tint: |t| t.warn,
        keywords: &[
            "sound", "audio", "volume", "mute", "unmute", "beep", "master", "speaker", "microphone", "mic",
            "звук", "громкость", "мутить", "замутить", "динамик", "сигнал", "аудио", "бип", "микрофон",
        ],
        constructor: || Action::AudioControl {
            target: crate::models::AudioTarget::Playback,
            action_type: crate::models::AudioVolumeAction::SetVolume,
            volume_level: 50,
        },
    },
    CatalogItem {
        category_idx: 2,
        badge: "AUDIO",
        title: |loc| loc.inspector_menu_process_audio,
        description: |loc| loc.catalog_desc_process_audio,
        glyph: egui_phosphor::regular::FADERS,
        tint: |t| t.warn,
        keywords: &[
            "process audio", "app volume", "mixer", "fader", "mute app", "application audio",
            "микшер", "звук приложения", "громкость процесса", "глушить программу", "аудио",
        ],
        constructor: || Action::ProcessAudio {
            process_name: String::new(),
            action_type: ProcessAudioActionType::Mute,
            volume_level: 100,
        },
    },
    CatalogItem {
        category_idx: 2,
        badge: "AUDIO",
        title: |loc| loc.action_play_sound_title,
        description: |loc| loc.action_play_sound_desc,
        glyph: egui_phosphor::regular::MUSIC_NOTES,
        tint: |t| t.warn,
        keywords: &[
            "play sound", "audio", "wav", "file", "play", "music", "alert", "effect",
            "звук", "воспроизвести", "аудио", "мелодия", "файл", "эффект", "сигнал",
        ],
        constructor: || Action::PlaySound {
            file_path: String::new(),
        },
    },

    // 3. Automation & Web
    CatalogItem {
        category_idx: 3,
        badge: "SCRIPT",
        title: |loc| loc.action_execute_script_title,
        description: |loc| loc.action_execute_script_desc,
        glyph: egui_phosphor::regular::CODE,
        tint: |t| t.accent_ink,
        keywords: &[
            "powershell", "pwsh", "ps1", "script", "code", "cmd", "batch", "bat", "lua", "terminal", "console",
            "павершелл", "скрипт", "код", "автоматизация", "сценарий", "батник", "терминал", "луа",
        ],
        constructor: || Action::ExecuteScript {
            interpreter: crate::models::ScriptType::PowerShell,
            script: String::new(),
            run_in_window: false,
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "MACRO",
        title: |loc| loc.action_macro_title,
        description: |loc| loc.action_macro_desc,
        glyph: egui_phosphor::regular::KEYBOARD,
        tint: |t| t.warn,
        keywords: &[
            "macro", "input", "keyboard", "mouse", "click", "record", "replay", "keys", "type", "hotkey", "cursor",
            "макрос", "клавиатура", "мышь", "клик", "запись", "нажатие", "печать", "хоткей", "курсор", "эмуляция",
        ],
        constructor: || Action::Macro {
            events: Vec::new(),
            repeat: 1,
            speed: 1.0,
            infinite: false,
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "NOTIFY",
        title: |loc| loc.inspector_menu_notification,
        description: |loc| loc.catalog_desc_notification,
        glyph: egui_phosphor::regular::BELL,
        tint: |t| t.accent_ink,
        keywords: &[
            "notification", "toast", "alert", "message", "notify", "popup", "banner",
            "уведомление", "сообщение", "алерт", "баннер", "оповещение", "всплывающее",
        ],
        constructor: || Action::Notification {
            title: String::new(),
            message: String::new(),
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "NETWORK",
        title: |loc| loc.inspector_menu_webhook,
        description: |loc| loc.catalog_desc_webhook,
        glyph: egui_phosphor::regular::GLOBE,
        tint: |t| t.good,
        keywords: &[
            "webhook", "http", "api", "rest", "post", "get", "put", "request", "url", "json",
            "вебхук", "запрос", "сеть", "интернет", "урл", "апи",
        ],
        constructor: || Action::HttpWebhook {
            url: String::new(),
            method: "POST".into(),
            headers: None,
            payload: None,
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "LOGIC",
        title: |loc| loc.catalog_title_if_else,
        description: |loc| loc.catalog_desc_if_else,
        glyph: egui_phosphor::regular::GIT_FORK,
        tint: |t| t.accent_ink,
        keywords: &[
            "if", "else", "condition", "branch", "logic", "toggle", "check", "flow",
            "если", "иначе", "ветвление", "условие", "логика", "проверка",
        ],
        constructor: || Action::IfElse {
            condition: None,
            conditions: vec![crate::models::RuleConditionItem::new(crate::models::RuleCondition::default_process())],
            then_actions: Vec::new(),
            else_if_branches: Vec::new(),
            else_actions: Vec::new(),
        },
    },

    // 4. System & Flow Control
    CatalogItem {
        category_idx: 4,
        badge: "SYSTEM",
        title: |loc| loc.inspector_menu_system_power,
        description: |loc| loc.catalog_desc_system_power,
        glyph: egui_phosphor::regular::POWER,
        tint: |t| t.bad,
        keywords: &[
            "power", "lock", "sleep", "shutdown", "restart", "screen", "monitor", "display",
            "питание", "блокировка", "сон", "выключение", "экран", "перезагрузка", "монитор",
        ],
        constructor: || Action::SystemPower {
            power_type: SystemPowerType::Lock,
        },
    },
    CatalogItem {
        category_idx: 4,
        badge: "FLOW",
        title: |loc| loc.inspector_menu_delay,
        description: |loc| loc.catalog_desc_delay,
        glyph: egui_phosphor::regular::HOURGLASS,
        tint: |t| t.faint,
        keywords: &[
            "delay", "pause", "wait", "sleep", "timeout", "seconds", "hold",
            "задержка", "пауза", "ждать", "секунды", "таймаут", "время", "ожидание",
        ],
        constructor: || Action::Delay { seconds: 1.0 },
    },
    CatalogItem {
        category_idx: 4,
        badge: "POWER",
        title: |loc| loc.action_power_plan_title,
        description: |loc| loc.action_power_plan_desc,
        glyph: egui_phosphor::regular::BATTERY_CHARGING,
        tint: |t| t.warn,
        keywords: &[
            "power plan", "energy", "high performance", "balanced", "battery saver",
            "питание", "план", "схема", "энергосбережение", "производительность", "аккумулятор",
        ],
        constructor: || Action::PowerPlan {
            plan: PowerPlanType::HighPerformance,
        },
    },
    CatalogItem {
        category_idx: 4,
        badge: "SECURITY",
        title: |loc| loc.action_firewall_title,
        description: |loc| loc.action_firewall_desc,
        glyph: egui_phosphor::regular::SHIELD_CHECK,
        tint: |t| t.bad,
        keywords: &[
            "firewall", "block", "network", "internet", "security", "rule", "netsh",
            "файрвол", "брандмауэр", "сеть", "интернет", "трафик", "блокировка",
        ],
        constructor: || Action::Firewall {
            process_name: String::new(),
            block: true,
        },
    },
    CatalogItem {
        category_idx: 2,
        badge: "DEV",
        title: |loc| loc.action_default_device_title,
        description: |loc| loc.action_default_device_desc,
        glyph: egui_phosphor::regular::HEADPHONES,
        tint: |t| t.warn,
        keywords: &[
            "audio device", "playback", "speakers", "headphones", "sound output", "default device", "microphone", "mic", "recording", "input",
            "устройство", "вывод", "динамики", "наушники", "звук", "воспроизведение", "микрофон", "ввод", "запись",
        ],
        constructor: || Action::DefaultDevice {
            target: crate::models::AudioTarget::Playback,
            device_name: String::new(),
        },
    },
    CatalogItem {
        category_idx: 4,
        badge: "DISPLAY",
        title: |loc| loc.action_display_title,
        description: |loc| loc.action_display_desc,
        glyph: egui_phosphor::regular::MONITOR,
        tint: |t| t.accent_ink,
        keywords: &[
            "display", "resolution", "hz", "refresh rate", "monitor", "screen", "fps", "144hz", "1080p", "4k",
            "экран", "разрешение", "герцовка", "монитор", "частота", "дисплей",
        ],
        constructor: || Action::Display {
            width: Some(1920),
            height: Some(1080),
            refresh_rate: Some(144),
            monitor_index: None,
        },
    },
];

pub fn render_action_catalog_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.action_catalog.is_some();

    let active_modal = state
        .action_catalog
        .as_ref()
        .or(state.last_action_catalog.as_ref())
        .cloned();

    let Some(mut modal_state) = active_modal else { return };

    let loc = state.loc();
    let mut chosen_action = None;

    let modal_res = Modal::new("action-catalog-modal", is_open)
        .width_fraction(0.74)
        .height_fraction(0.76)
        .min_size(vec2(720.0, 500.0))
        .max_size(vec2(840.0, 640.0))
        .header_height(46.0)
        .toolbar_height(46.0)
        .toolbar_sunken(true)
        .show(ctx, &t, |frame| {
            // ---------------- 1. HEADER ----------------
            frame.header(|ui, close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    widgets::hud(ui, loc.catalog_modal_eyebrow, t.ink);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_cancel).clicked() {
                            *close_req = true;
                        }
                    });
                });
            });

            // ---------------- 2. SEARCH BAR ----------------
            frame.toolbar(|ui, _close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    ui.label(
                        RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                            .family(theme::phosphor())
                            .size(15.0)
                            .color(t.muted),
                    );
                    ui.add_space(6.0);

                    let has_query = !modal_state.search_query.is_empty();
                    let extra_w = if has_query { 40.0 } else { 14.0 };
                    let input_w = (ui.available_width() - extra_w).max(100.0);

                    let _search_resp = ui.add_sized(
                        vec2(input_w, 28.0),
                        egui::TextEdit::singleline(&mut modal_state.search_query)
                            .hint_text(
                                RichText::new(loc.catalog_search_hint)
                                    .family(theme::medium())
                                    .size(12.5)
                                    .color(t.faint),
                            )
                            .font(FontId::new(13.0, theme::medium()))
                            .text_color(t.ink)
                            .margin(Margin::symmetric(8, 5)),
                    );

                    if has_query {
                        ui.add_space(4.0);
                        let (btn_rect, mut btn_resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                        let is_h = btn_resp.hovered();
                        if is_h {
                            ui.painter().rect_filled(btn_rect, CornerRadius::same(2), t.bg);
                        }
                        ui.painter().text(
                            btn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::X,
                            FontId::new(13.0, theme::phosphor()),
                            if is_h { t.ink } else { t.muted },
                        );
                        btn_resp = btn_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        if btn_resp.clicked() {
                            modal_state.search_query.clear();
                        }
                    }
                });
            });

            // ---------------- 3. BODY (CATEGORIES + ACTION CARDS) ----------------
            frame.body(|ui| {
                let body_rect = ui.max_rect();
                let sidebar_w = 195.0;
                let sidebar_rect = Rect::from_min_size(body_rect.min, vec2(sidebar_w, body_rect.height()));
                let cards_rect = Rect::from_min_max(
                    pos2(sidebar_rect.right() + 1.0, body_rect.top()),
                    body_rect.max,
                );

                // Body background surfaces & divider
                let p = ui.painter();
                p.rect_filled(sidebar_rect, CornerRadius::ZERO, t.bg);
                p.rect_filled(cards_rect, CornerRadius::ZERO, t.sunken);
                p.vline(sidebar_rect.right(), body_rect.y_range(), t.hairline());

                // Left: Categories List
                ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar_rect), |ui| {
                    egui::Frame::new()
                        .inner_margin(Margin::symmetric(10, 10))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;

                            let is_item_allowed = |item: &CatalogItem| -> bool {
                                if modal_state.ifelse_target.is_some() && item.badge == "LOGIC" {
                                    return false;
                                }
                                true
                            };

                            let count_for = |cat_idx: usize| -> usize {
                                CATALOG_ITEMS
                                    .iter()
                                    .filter(|i| is_item_allowed(i) && (cat_idx == 0 || i.category_idx == cat_idx))
                                    .count()
                            };

                            let categories = [
                                (0, loc.catalog_cat_all, egui_phosphor::regular::SPARKLE, count_for(0)),
                                (1, loc.catalog_cat_process, egui_phosphor::regular::CPU, count_for(1)),
                                (2, loc.catalog_cat_audio, egui_phosphor::regular::SPEAKER_HIGH, count_for(2)),
                                (3, loc.catalog_cat_automation, egui_phosphor::regular::CODE, count_for(3)),
                                (4, loc.catalog_cat_system, egui_phosphor::regular::GEAR, count_for(4)),
                            ];

                            for (cat_idx, cat_name, icon, count) in categories {
                                let is_active = modal_state.selected_category == cat_idx;
                                let row_w = ui.available_width();
                                let (row_rect, mut row_resp) = ui.allocate_exact_size(vec2(row_w, 34.0), Sense::click());
                                let is_h = row_resp.hovered();
                                let p = ui.painter();

                                let bg_color = if is_active {
                                    t.accent_ink.gamma_multiply(0.12)
                                } else if is_h {
                                    t.sunken
                                } else {
                                    Color32::TRANSPARENT
                                };
                                if bg_color != Color32::TRANSPARENT {
                                    p.rect_filled(row_rect, CornerRadius::same(2), bg_color);
                                }

                                // Active indicator line on the left
                                if is_active {
                                    p.rect_filled(
                                        Rect::from_min_size(row_rect.left_top(), vec2(3.0, row_rect.height())),
                                        CornerRadius::same(1),
                                        t.accent_ink,
                                    );
                                }

                                let icon_color = if is_active {
                                    t.accent_ink
                                } else if is_h {
                                    t.ink
                                } else {
                                    t.muted
                                };
                                let text_color = if is_active {
                                    t.accent_ink
                                } else if is_h {
                                    t.ink
                                } else {
                                    t.muted
                                };

                                // Category icon
                                p.text(
                                    pos2(row_rect.left() + 10.0, row_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    icon,
                                    FontId::new(15.0, theme::phosphor()),
                                    icon_color,
                                );

                                // Category label
                                p.text(
                                    pos2(row_rect.left() + 32.0, row_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    cat_name,
                                    FontId::new(12.5, theme::medium()),
                                    text_color,
                                );

                                // Count badge
                                let count_str = count.to_string();
                                let badge_color = if is_active { t.accent_ink } else { t.faint };
                                p.text(
                                    pos2(row_rect.right() - 8.0, row_rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    count_str,
                                    FontId::new(11.0, theme::mono()),
                                    badge_color,
                                );

                                row_resp = row_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                if row_resp.clicked() {
                                    modal_state.selected_category = cat_idx;
                                }
                            }
                        });
                });

                // Filter actions
                let query_lower = modal_state.search_query.trim().to_lowercase();
                let is_item_allowed = |item: &CatalogItem| -> bool {
                    if modal_state.ifelse_target.is_some() && item.badge == "LOGIC" {
                        return false;
                    }
                    true
                };

                let filtered_items: Vec<&CatalogItem> = CATALOG_ITEMS
                    .iter()
                    .filter(|item| {
                        if !is_item_allowed(item) {
                            return false;
                        }

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

                // Right: Action Cards List
                ui.scope_builder(egui::UiBuilder::new().max_rect(cards_rect), |ui| {
                    egui::Frame::new()
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("catalog_cards_scroll")
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    if filtered_items.is_empty() {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(70.0);
                                            ui.label(
                                                RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                                                    .family(theme::phosphor())
                                                    .size(36.0)
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
                                        ui.spacing_mut().item_spacing.y = 8.0;
                                        for item in &filtered_items {
                                            let card_w = ui.available_width();
                                            let card_h = 60.0;
                                            let (rect, mut resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
                                            let is_hovered = resp.hovered();
                                            let p = ui.painter();

                                            let fill_color = t.bg;
                                            let border_stroke = if is_hovered {
                                                Stroke::new(1.2, t.accent_ink)
                                            } else {
                                                Stroke::new(1.0, t.line_strong)
                                            };
                                            p.rect_filled(rect, CornerRadius::same(2), fill_color);
                                            p.rect_stroke(rect, CornerRadius::same(2), border_stroke, egui::StrokeKind::Inside);

                                            let item_tint = (item.tint)(&t);

                                            // Left Icon Box: 38x38
                                            let icon_box = Rect::from_center_size(
                                                pos2(rect.left() + 28.0, rect.center().y),
                                                vec2(38.0, 38.0),
                                            );
                                            p.rect_filled(icon_box, CornerRadius::same(2), item_tint.gamma_multiply(0.12));
                                            p.rect_stroke(
                                                icon_box,
                                                CornerRadius::same(2),
                                                Stroke::new(1.0, item_tint.gamma_multiply(0.35)),
                                                egui::StrokeKind::Inside,
                                            );
                                            p.text(
                                                icon_box.center(),
                                                egui::Align2::CENTER_CENTER,
                                                item.glyph,
                                                FontId::new(18.0, theme::phosphor()),
                                                item_tint,
                                            );

                                            // Center Text column
                                            let text_left = icon_box.right() + 12.0;
                                            let title_text = (item.title)(loc);
                                            let title_color = if is_hovered { t.accent_ink } else { t.ink };

                                            // Title
                                            let title_galley = p.layout_no_wrap(
                                                title_text.to_string(),
                                                FontId::new(13.0, theme::semibold()),
                                                title_color,
                                            );
                                            let title_w = title_galley.size().x;
                                            let title_y = rect.top() + 12.0;
                                            p.galley(pos2(text_left, title_y), title_galley, title_color);

                                            // Category badge tag next to title
                                            let badge_x = text_left + title_w + 8.0;
                                            let badge_galley = widgets::caps(ui, item.badge, 9.0, theme::mono(), 0.8, item_tint);
                                            let badge_w = badge_galley.size().x + 8.0;
                                            let badge_rect = Rect::from_min_size(
                                                pos2(badge_x, title_y + 1.0),
                                                vec2(badge_w, 15.0),
                                            );
                                            p.rect_filled(badge_rect, CornerRadius::same(2), item_tint.gamma_multiply(0.12));
                                            p.galley_with_override_text_color(
                                                pos2(badge_rect.left() + 4.0, badge_rect.top() + 1.5),
                                                badge_galley,
                                                item_tint,
                                            );

                                            // Description row with safe width truncation
                                            let desc_text = (item.description)(loc);
                                            let right_reserve = 36.0;
                                            let max_desc_w = (rect.right() - right_reserve - text_left).max(50.0);
                                            let mut desc_job = egui::text::LayoutJob::single_section(
                                                desc_text.to_string(),
                                                egui::TextFormat {
                                                    font_id: FontId::new(11.5, theme::medium()),
                                                    color: t.muted,
                                                    ..Default::default()
                                                },
                                            );
                                            desc_job.wrap = egui::text::TextWrapping::truncate_at_width(max_desc_w);
                                            let desc_galley = p.layout_job(desc_job);
                                            p.galley(pos2(text_left, rect.top() + 33.0), desc_galley, t.muted);

                                            // Right Action indicator: Plus on hover, Caret right when idle
                                            let right_icon = if is_hovered {
                                                egui_phosphor::regular::PLUS
                                            } else {
                                                egui_phosphor::regular::CARET_RIGHT
                                            };
                                            let right_icon_color = if is_hovered { t.accent_ink } else { t.faint };
                                            p.text(
                                                pos2(rect.right() - 18.0, rect.center().y),
                                                egui::Align2::CENTER_CENTER,
                                                right_icon,
                                                FontId::new(15.0, theme::phosphor()),
                                                right_icon_color,
                                            );

                                            resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                            if resp.clicked() {
                                                chosen_action = Some((item.constructor)());
                                            }
                                        }
                                    }
                                });
                        });
                });
            });
        });

    if let Some(act) = chosen_action {
        state.add_action_from_catalog(act, modal_state.is_exit_pipeline, modal_state.insert_idx, modal_state.ifelse_target);
    } else if let Some(res) = modal_res {
        if res.close_requested {
            state.close_action_catalog();
        } else if is_open {
            state.action_catalog = Some(modal_state.clone());
            state.last_action_catalog = Some(modal_state);
        }
    } else if !is_open {
        state.last_action_catalog = None;
    }
}
