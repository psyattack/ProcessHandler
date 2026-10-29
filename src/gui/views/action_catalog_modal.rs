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
    Action, PowerPlanType,
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

const CATALOG_ITEMS: [CatalogItem; 24] = [
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
            "tuning", "priority", "affinity", "cpu", "cores",
            "приоритет", "ядра", "тюнинг", "оптимизация", "процесс",
        ],
        constructor: || Action::ProcessTuning {
            target_process: None,
            priority: None,
            affinity_mask: None,
            trim_memory: false,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "RAM",
        title: |loc| loc.catalog_title_trim_working_set,
        description: |loc| loc.catalog_desc_trim_working_set,
        glyph: egui_phosphor::regular::BROOM,
        tint: |t| t.good,
        keywords: &[
            "trim", "memory", "working set", "ram", "cleanup", "release", "empty",
            "очистка", "память", "рам", "озу", "сброс", "оптимизация", "рабочее множество",
        ],
        constructor: || Action::TrimWorkingSet {
            target_process: None,
            soft_limit_mb: None,
            skip_if_focused: false,
        },
    },

    // 2. Audio & Media
    CatalogItem {
        category_idx: 2,
        badge: "AUDIO",
        title: |loc| loc.action_audio_control_title,
        description: |loc| loc.action_audio_control_desc,
        glyph: egui_phosphor::regular::SPEAKER_HIGH,
        tint: |t| t.warn,
        keywords: &[
            "sound", "audio", "volume", "mute", "unmute", "process", "master", "speaker", "microphone", "mic",
            "звук", "громкость", "мутить", "замутить", "динамик", "процесс", "аудио", "микрофон",
        ],
        constructor: || Action::AudioControl {
            target: crate::models::AudioTarget::Playback,
            process_name: None,
            action_type: crate::models::AudioVolumeAction::Mute,
            volume_level: 50,
            left_volume: None,
            right_volume: None,
        },
    },
    CatalogItem {
        category_idx: 2,
        badge: "AUDIO",
        title: |loc| loc.catalog_title_audio_format,
        description: |loc| loc.catalog_desc_audio_format,
        glyph: egui_phosphor::regular::EQUALIZER,
        tint: |t| t.warn,
        keywords: &[
            "audio format", "sample rate", "hz", "bit", "depth", "channels", "frequency",
            "формат аудио", "частота", "герц", "бит", "каналы", "звук",
        ],
        constructor: || Action::AudioFormat {
            target: crate::models::AudioTarget::Playback,
            device_name: None,
            sample_rate: 48000,
            bit_depth: 24,
            channels: 2,
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
            "play sound", "audio", "wav", "mp3", "beep", "file", "play", "music", "alert", "effect",
            "звук", "воспроизвести", "аудио", "бип", "сигнал", "мелодия", "файл", "эффект",
        ],
        constructor: || Action::PlaySound {
            file_path: String::new(),
            play_beep: false,
            beep_frequency: None,
            beep_duration_ms: None,
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
        badge: "OVERLAY",
        title: |loc| loc.catalog_title_overlay,
        description: |loc| loc.catalog_desc_overlay,
        glyph: egui_phosphor::regular::FRAME_CORNERS,
        tint: |t| t.accent_ink,
        keywords: &[
            "overlay", "hud", "crosshair", "reticle", "widget", "onscreen", "display", "gui", "badge",
            "оверлей", "прицел", "хад", "виджет", "экран", "бейдж", "индикатор",
        ],
        constructor: || {
            static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let unique_id = format!("overlay_{:x}_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos(), count);
            let def = crate::models::OverlayDefinition {
                id: unique_id.clone(),
                ..Default::default()
            };
            Action::Overlay {
                overlay_id: unique_id,
                custom_overlay: Some(Box::new(def)),
                operation: crate::models::OverlayOperation::Toggle,
                override_text: None,
                duration_ms: None,
                close_on_exit: false,
            }
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
            conditions: vec![crate::models::RuleConditionItem::new(crate::models::RuleCondition::default_process())],
            then_actions: Vec::new(),
            else_if_branches: Vec::new(),
            else_actions: Vec::new(),
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "DISCORD",
        title: |loc| loc.catalog_title_discord_rpc_set,
        description: |loc| loc.catalog_desc_discord_rpc_set,
        glyph: egui_phosphor::regular::GAME_CONTROLLER,
        tint: |t| t.accent_ink,
        keywords: &[
            "discord", "rpc", "rich presence", "activity", "game", "status", "profile",
            "дискорд", "рпц", "статус", "игра", "профиль", "активность",
        ],
        constructor: || Action::DiscordRpcSet {
            app_id: String::new(),
            details: String::new(),
            state: String::new(),
            large_image: None,
            large_text: None,
            small_image: None,
            small_text: None,
            show_elapsed: true,
            buttons: Vec::new(),
            auto_clear_on_process_exit: true,
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "DISCORD",
        title: |loc| loc.catalog_title_discord_rpc_clear,
        description: |loc| loc.catalog_desc_discord_rpc_clear,
        glyph: egui_phosphor::regular::X_CIRCLE,
        tint: |t| t.bad,
        keywords: &[
            "discord", "rpc", "clear", "stop", "reset", "remove status",
            "дискорд", "сброс", "очистить", "удалить статус", "остановить",
        ],
        constructor: || Action::DiscordRpcClear,
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
        badge: "AUDIO",
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
    CatalogItem {
        category_idx: 4,
        badge: "ARCHIVE",
        title: |loc| loc.catalog_title_archive,
        description: |loc| loc.catalog_desc_archive,
        glyph: egui_phosphor::regular::ARCHIVE,
        tint: |t| t.warn,
        keywords: &[
            "archive", "zip", "rar", "7z", "tar", "compress", "extract", "unzip", "unpack", "pack",
            "архив", "сжатие", "распаковка", "зип", "рар", "паковать", "распаковать",
        ],
        constructor: || Action::Archive {
            operation: crate::models::ArchiveOperation::Pack,
            format: crate::models::ArchiveFormat::Zip,
            source_path: String::new(),
            destination_path: String::new(),
            compression_level: None,
            overwrite: true,
        },
    },
    CatalogItem {
        category_idx: 4,
        badge: "FILES",
        title: |loc| loc.catalog_title_file_system,
        description: |loc| loc.catalog_desc_file_system,
        glyph: egui_phosphor::regular::FOLDER,
        tint: |t| t.accent_ink,
        keywords: &[
            "file", "folder", "directory", "create", "delete", "rename", "copy", "clone", "write", "text", "append",
            "файл", "папка", "создать", "удалить", "переименовать", "копировать", "клонировать", "записать", "текст",
        ],
        constructor: || Action::FileSystem {
            operation: crate::models::FileSystemOperation::CreateFile,
            path: String::new(),
            target_path: None,
            content: None,
            create_parents: true,
            overwrite: true,
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

            // ---------------- 2. BODY (CATEGORIES + ACTION CARDS) ----------------
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
                                widgets::catalog_search_box(ui, &mut modal_state.search_query, loc.catalog_search_hint);
                            },
                        );
                    });
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .id_salt("action_catalog_cards_scroll")
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
                                                chosen_action = Some((item.constructor)());
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
