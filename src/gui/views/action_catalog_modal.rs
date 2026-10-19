//! Action Catalog Modal:
//! Studio modal for browsing and selecting rule actions
//! with categorized tabs, instant fuzzy keyword search, and rich capability cards.

use egui::{
    Color32, CornerRadius, FontId, Id, Margin, Order, Rect, RichText, Sense, Stroke,
    emath::easing, pos2, vec2,
};
use crate::gui::components::widgets::{self, Glyph};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    Action, MicrophoneActionType, ProcessAudioActionType, SoundActionType, SystemPowerType,
};

#[allow(dead_code)]
struct CatalogItem {
    id: &'static str,
    category_idx: usize, // 1: Process, 2: Audio, 3: Automation, 4: System
    badge: &'static str,
    title: fn(&crate::i18n::Strings) -> &'static str,
    description: fn(&crate::i18n::Strings) -> &'static str,
    glyph: &'static str,
    tint: fn(&Tokens) -> Color32,
    keywords: &'static [&'static str],
    constructor: fn() -> Action,
}

const CATALOG_ITEMS: [CatalogItem; 11] = [
    // 1. Process Management
    CatalogItem {
        id: "start_process",
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
        id: "kill_process",
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
            process_name: "target.exe".into(),
            kill_tree: true,
            force: true,
        },
    },

    // 2. Audio & Media
    CatalogItem {
        id: "sound",
        category_idx: 2,
        badge: "AUDIO",
        title: |loc| loc.inspector_menu_sound,
        description: |loc| loc.catalog_desc_sound,
        glyph: egui_phosphor::regular::SPEAKER_HIGH,
        tint: |t| t.warn,
        keywords: &[
            "sound", "audio", "volume", "mute", "unmute", "beep", "master", "speaker",
            "звук", "громкость", "мутить", "замутить", "динамик", "сигнал", "аудио", "бип",
        ],
        constructor: || Action::Sound {
            sound_type: SoundActionType::PlayBeep,
            volume_level: 50,
        },
    },
    CatalogItem {
        id: "microphone",
        category_idx: 2,
        badge: "MIC",
        title: |loc| loc.inspector_menu_microphone,
        description: |loc| loc.catalog_desc_microphone,
        glyph: egui_phosphor::regular::MICROPHONE,
        tint: |t| t.accent_ink,
        keywords: &[
            "microphone", "mic", "mute", "unmute", "toggle", "volume", "input", "recording",
            "микрофон", "микро", "звук", "глушение", "замутить", "вход",
        ],
        constructor: || Action::Microphone {
            action_type: MicrophoneActionType::ToggleMute,
            volume_level: 100,
        },
    },
    CatalogItem {
        id: "process_audio",
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
            process_name: "notepad.exe".into(),
            action_type: ProcessAudioActionType::Mute,
            volume_level: 100,
        },
    },

    // 3. Automation & Web
    CatalogItem {
        id: "powershell",
        category_idx: 3,
        badge: "SCRIPT",
        title: |loc| loc.inspector_menu_powershell,
        description: |loc| loc.catalog_desc_powershell,
        glyph: egui_phosphor::regular::CODE,
        tint: |t| t.accent_ink,
        keywords: &[
            "powershell", "pwsh", "ps1", "script", "code", "automate", "posh",
            "павершелл", "скрипт", "код", "автоматизация", "сценарий",
        ],
        constructor: || Action::PowerShell {
            script: "Write-Output 'ProcessHandler'".into(),
            run_in_window: false,
        },
    },
    CatalogItem {
        id: "cmd",
        category_idx: 3,
        badge: "SCRIPT",
        title: |loc| loc.inspector_menu_cmd,
        description: |loc| loc.catalog_desc_cmd,
        glyph: egui_phosphor::regular::TERMINAL_WINDOW,
        tint: |t| t.muted,
        keywords: &[
            "cmd", "command prompt", "batch", "bat", "terminal", "console", "dos",
            "командная строка", "терминал", "консоль", "батник", "скрипт", "cmd.exe",
        ],
        constructor: || Action::Cmd {
            command: "echo ProcessHandler".into(),
            run_in_window: false,
        },
    },
    CatalogItem {
        id: "notification",
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
            title: "ProcessHandler Alert".into(),
            message: "Process event triggered".into(),
        },
    },
    CatalogItem {
        id: "webhook",
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
            url: "http://localhost:8080/hook".into(),
            method: "POST".into(),
            headers: None,
            payload: None,
        },
    },

    // 4. System & Flow Control
    CatalogItem {
        id: "system_power",
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
        id: "delay",
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
];

pub fn render_action_catalog_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.action_catalog.is_some();
    let mut close_requested = false;

    // Handle Esc key to close modal
    if is_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close_requested = true;
    }

    let anim = ctx.animate_bool_with_time_and_easing(
        Id::new("action-catalog-modal-anim"),
        is_open,
        0.20,
        easing::cubic_out,
    );

    if anim <= 0.001 && !is_open {
        state.last_action_catalog = None;
        return;
    }

    let active_modal = state
        .action_catalog
        .as_ref()
        .or(state.last_action_catalog.as_ref())
        .cloned();

    let Some(mut modal_state) = active_modal else { return };

    let full = ctx.content_rect();

    // 1. Full-screen scrim overlay
    let scrim_alpha = (anim * 195.0) as u8;
    egui::Area::new(Id::new("action-catalog-modal-scrim"))
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
    let modal_w = (full.width() * 0.74).clamp(720.0, 840.0).min(full.width() - 32.0);
    let modal_h = (full.height() * 0.76).clamp(500.0, 640.0).min(full.height() - 48.0);
    let center = full.center();

    let scale = 0.95 + 0.05 * anim;
    let y_shift = (1.0 - anim) * 12.0;
    let modal_rect = Rect::from_center_size(
        pos2(center.x, center.y + y_shift),
        vec2(modal_w * scale, modal_h * scale),
    );

    let header_h = 46.0;
    let search_h = 46.0;

    let header_rect = Rect::from_min_size(modal_rect.min, vec2(modal_rect.width(), header_h));
    let search_rect = Rect::from_min_size(
        pos2(modal_rect.left(), header_rect.bottom()),
        vec2(modal_rect.width(), search_h),
    );
    let body_rect = Rect::from_min_max(
        pos2(modal_rect.left(), search_rect.bottom()),
        modal_rect.max,
    );

    let sidebar_w = 195.0;
    let sidebar_rect = Rect::from_min_size(body_rect.min, vec2(sidebar_w, body_rect.height()));
    let cards_rect = Rect::from_min_max(
        pos2(sidebar_rect.right() + 1.0, body_rect.top()),
        body_rect.max,
    );

    let loc = state.loc();
    let mut chosen_action = None;

    // 3. Modal Surface
    egui::Area::new(Id::new("action-catalog-modal-window"))
        .order(Order::Tooltip)
        .fixed_pos(modal_rect.min)
        .show(ctx, |ui| {
            ui.set_width(modal_rect.width());
            ui.set_height(modal_rect.height());
            ui.set_max_width(modal_rect.width());
            ui.set_max_height(modal_rect.height());
            ui.set_clip_rect(modal_rect);

            // Paint background surfaces and precision dividers
            {
                let p = ui.painter();
                p.add(theme::shadow(&t).as_shape(modal_rect, CornerRadius::same(2)));
                p.rect_filled(modal_rect, CornerRadius::same(2), t.bg);

                // Section background fills
                p.rect_filled(header_rect, CornerRadius::ZERO, t.bg);
                p.rect_filled(search_rect, CornerRadius::ZERO, t.sunken);
                p.rect_filled(sidebar_rect, CornerRadius::ZERO, t.bg);
                p.rect_filled(cards_rect, CornerRadius::ZERO, t.sunken);

                // Horizontal dividers and sidebar border
                p.hline(modal_rect.x_range(), header_rect.bottom(), t.hairline());
                p.hline(modal_rect.x_range(), search_rect.bottom(), t.hairline());
                p.vline(sidebar_rect.right(), body_rect.y_range(), t.hairline());

                // CRITICAL: Paint outer border stroke AFTER all inner fills so all 4 sides are consistently visible
                p.rect_stroke(
                    modal_rect,
                    CornerRadius::same(2),
                    Stroke::new(1.0, t.line_strong),
                    egui::StrokeKind::Inside,
                );
            }

            // ---------------- 1. HEADER ----------------
            ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    widgets::hud(ui, loc.catalog_modal_eyebrow, t.ink);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_cancel).clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

            // ---------------- 2. SEARCH BAR ----------------
            ui.scope_builder(egui::UiBuilder::new().max_rect(search_rect), |ui| {
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
            // Left: Categories List
            ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar_rect), |ui| {
                egui::Frame::new()
                    .inner_margin(Margin::symmetric(10, 10))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;

                        let categories = [
                            (0, loc.catalog_cat_all, egui_phosphor::regular::SPARKLE, 11),
                            (1, loc.catalog_cat_process, egui_phosphor::regular::CPU, 2),
                            (2, loc.catalog_cat_audio, egui_phosphor::regular::SPEAKER_HIGH, 3),
                            (3, loc.catalog_cat_automation, egui_phosphor::regular::CODE, 4),
                            (4, loc.catalog_cat_system, egui_phosphor::regular::GEAR, 2),
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
            let filtered_items: Vec<&CatalogItem> = CATALOG_ITEMS
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

                    if modal_state.selected_category != 0 && query_lower.is_empty() {
                        if item.category_idx != modal_state.selected_category {
                            return false;
                        }
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

    if let Some(act) = chosen_action {
        state.add_action_from_catalog(act, modal_state.is_exit_pipeline, modal_state.insert_idx);
    } else if close_requested {
        state.close_action_catalog();
    } else if is_open {
        state.action_catalog = Some(modal_state.clone());
        state.last_action_catalog = Some(modal_state);
    }
}
