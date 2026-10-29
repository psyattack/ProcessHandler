//! Element Catalog Modal:
//! Studio modal for browsing and selecting overlay elements and widgets
//! with categorized tabs, instant fuzzy keyword search, and rich capability cards.

use egui::{
    pos2, vec2, Color32, CornerRadius, FontId, Margin, Rect, RichText, Sense, Stroke,
};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Glyph};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{
    CrosshairStyle, GraphMetric, GraphStyle, InteractiveControlKind, KeyVisualizerLayout,
    OverlayClickAction, OverlayElement, ShapeKind, SliderBindTarget, TelemetryKind, TelemetryStyle,
    TimerMode,
};

struct CatalogItem {
    category_idx: usize, // 1: Visual, 2: Interactive, 3: Telemetry, 4: Gaming & Media
    #[allow(dead_code)]
    badge: &'static str,
    title: fn(&crate::i18n::Strings) -> &'static str,
    description: fn(&crate::i18n::Strings) -> &'static str,
    glyph: &'static str,
    tint: fn(&Tokens) -> Color32,
    keywords: &'static [&'static str],
    constructor: fn(id_num: usize) -> OverlayElement,
}

const CATALOG_ITEMS: [CatalogItem; 14] = [
    // 1. Visual & Display
    CatalogItem {
        category_idx: 1,
        badge: "DISPLAY",
        title: |loc| loc.elem_cat_title_text,
        description: |loc| loc.elem_cat_desc_text,
        glyph: egui_phosphor::regular::TEXT_T,
        tint: |t| t.accent_ink,
        keywords: &[
            "text", "label", "string", "font", "title", "caption",
            "текст", "надпись", "метка", "шрифт", "заголовок",
        ],
        constructor: |id_num| OverlayElement::Text {
            id: format!("text_{id_num}"),
            text: "Text Label".to_string(),
            font_size: 14.0,
            color: [255, 255, 255, 255],
            x: 16.0,
            y: 16.0,
            width: None,
            height: None,
            lock_aspect: false,
            bold: false,
            font_family: "Segoe UI".to_string(),
            rotation: 0.0,
            on_click: None,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "IMAGE",
        title: |loc| loc.elem_cat_title_image,
        description: |loc| loc.elem_cat_desc_image,
        glyph: egui_phosphor::regular::IMAGE,
        tint: |t| t.accent_ink,
        keywords: &[
            "image", "picture", "photo", "png", "jpg", "ico", "icon", "logo",
            "изображение", "картинка", "фото", "логотип", "иконка",
        ],
        constructor: |id_num| OverlayElement::Image {
            id: format!("image_{id_num}"),
            path: "assets/app.png".to_string(),
            x: 20.0,
            y: 20.0,
            width: 64.0,
            height: 64.0,
            opacity: 1.0,
            lock_aspect: true,
            rotation: 0.0,
            on_click: None,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "BADGE",
        title: |loc| loc.elem_cat_title_badge,
        description: |loc| loc.elem_cat_desc_badge,
        glyph: egui_phosphor::regular::TAG,
        tint: |t| t.good,
        keywords: &[
            "badge", "pill", "tag", "chip", "status", "indicator",
            "бейдж", "метка", "тег", "плашка", "статус",
        ],
        constructor: |id_num| OverlayElement::Badge {
            id: format!("badge_{id_num}"),
            text: "STATUS: ACTIVE".to_string(),
            color: [255, 255, 255, 255],
            bg_color: [20, 110, 190, 220],
            x: 16.0,
            y: 16.0,
            rotation: 0.0,
            on_click: None,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "PROGRESS",
        title: |loc| loc.elem_cat_title_progress,
        description: |loc| loc.elem_cat_desc_progress,
        glyph: egui_phosphor::regular::PERCENT,
        tint: |t| t.warn,
        keywords: &[
            "progress", "bar", "meter", "gauge", "fill", "level",
            "прогресс", "индикатор", "шкала", "уровень", "полоса",
        ],
        constructor: |id_num| OverlayElement::ProgressBar {
            id: format!("progress_{id_num}"),
            label: "Progress".to_string(),
            progress: 0.65,
            fill_color: [59, 130, 246, 255],
            bg_color: [20, 25, 35, 180],
            x: 16.0,
            y: 16.0,
            width: 140.0,
            height: 16.0,
            rotation: 0.0,
        },
    },
    CatalogItem {
        category_idx: 1,
        badge: "SHAPE",
        title: |loc| loc.elem_cat_title_shape,
        description: |loc| loc.elem_cat_desc_shape,
        glyph: egui_phosphor::regular::SHAPES,
        tint: |t| t.faint,
        keywords: &[
            "shape", "rect", "rectangle", "circle", "line", "arrow", "geometry",
            "фигура", "прямоугольник", "круг", "линия", "стрелка", "геометрия",
        ],
        constructor: |id_num| OverlayElement::Shape {
            id: format!("shape_{id_num}"),
            shape_type: ShapeKind::Rectangle,
            x: 16.0,
            y: 16.0,
            width: 80.0,
            height: 40.0,
            thickness: 2.0,
            color: [255, 255, 255, 200],
            filled: true,
            fill_color: [40, 50, 70, 160],
            corner_radius: 6.0,
            lock_aspect: false,
            rotation: 0.0,
            on_click: None,
        },
    },

    // 2. Interactive & Controls
    CatalogItem {
        category_idx: 2,
        badge: "CONTROL",
        title: |loc| loc.elem_cat_title_switch,
        description: |loc| loc.elem_cat_desc_switch,
        glyph: egui_phosphor::regular::TOGGLE_RIGHT,
        tint: |t| t.accent_ink,
        keywords: &[
            "switch", "toggle", "checkbox", "interactive", "knob",
            "переключатель", "свитч", "тумблер", "вкл", "выкл",
        ],
        constructor: |id_num| OverlayElement::InteractiveControl {
            id: format!("switch_{id_num}"),
            control: InteractiveControlKind::ToggleSwitch {
                label: "Turbo Mode".to_string(),
                is_active: true,
                bind_rule_id: None,
                on_enable: None,
                on_disable: None,
            },
            color: [255, 255, 255, 255],
            bg_color: [40, 45, 55, 200],
            active_color: [59, 130, 246, 255],
            x: 16.0,
            y: 16.0,
            width: 120.0,
            height: 26.0,
            rotation: 0.0,
        },
    },
    CatalogItem {
        category_idx: 2,
        badge: "SLIDER",
        title: |loc| loc.elem_cat_title_slider,
        description: |loc| loc.elem_cat_desc_slider,
        glyph: egui_phosphor::regular::SLIDERS,
        tint: |t| t.warn,
        keywords: &[
            "slider", "volume", "sound", "track", "drag", "value",
            "ползунок", "слайдер", "громкость", "звук", "значение",
        ],
        constructor: |id_num| OverlayElement::InteractiveControl {
            id: format!("slider_{id_num}"),
            control: InteractiveControlKind::Slider {
                label: "Volume".to_string(),
                min: 0.0,
                max: 100.0,
                value: 75.0,
                bind_target: SliderBindTarget::MasterVolume,
            },
            color: [255, 255, 255, 255],
            bg_color: [35, 40, 50, 200],
            active_color: [245, 158, 11, 255],
            x: 16.0,
            y: 16.0,
            width: 150.0,
            height: 32.0,
            rotation: 0.0,
        },
    },
    CatalogItem {
        category_idx: 2,
        badge: "BUTTON",
        title: |loc| loc.elem_cat_title_button,
        description: |loc| loc.elem_cat_desc_button,
        glyph: egui_phosphor::regular::CURSOR_CLICK,
        tint: |t| t.good,
        keywords: &[
            "button", "click", "mute", "action", "press", "trigger",
            "кнопка", "клик", "действие", "нажатие", "мут",
        ],
        constructor: |id_num| OverlayElement::InteractiveControl {
            id: format!("button_{id_num}"),
            control: InteractiveControlKind::Button {
                label: "Mute Sound".to_string(),
                icon: None,
                action: Some(OverlayClickAction::ToggleMute),
            },
            color: [255, 255, 255, 255],
            bg_color: [35, 40, 50, 200],
            active_color: [37, 99, 235, 220],
            x: 16.0,
            y: 16.0,
            width: 110.0,
            height: 28.0,
            rotation: 0.0,
        },
    },

    // 3. Hardware & Telemetry
    CatalogItem {
        category_idx: 3,
        badge: "GRAPH",
        title: |loc| loc.elem_cat_title_graph,
        description: |loc| loc.elem_cat_desc_graph,
        glyph: egui_phosphor::regular::CHART_LINE,
        tint: |t| t.accent_ink,
        keywords: &[
            "graph", "chart", "history", "cpu", "ram", "network", "fps", "histogram",
            "график", "чарт", "история", "память", "сеть", "фпс", "гистограмма",
        ],
        constructor: |id_num| OverlayElement::Graph {
            id: format!("graph_{id_num}"),
            metric: GraphMetric::SystemCpu,
            style: GraphStyle::AreaFilled,
            sample_count: 30,
            color: [59, 130, 246, 255],
            fill_color: [37, 99, 235, 90],
            bg_color: [18, 22, 32, 210],
            show_current_val: true,
            x: 16.0,
            y: 16.0,
            width: 160.0,
            height: 65.0,
            rotation: 0.0,
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "CLOCK",
        title: |loc| loc.elem_cat_title_clock,
        description: |loc| loc.elem_cat_desc_clock,
        glyph: egui_phosphor::regular::CLOCK,
        tint: |t| t.warn,
        keywords: &[
            "clock", "time", "stopwatch", "timer", "countdown", "uptime",
            "часы", "время", "секундомер", "таймер", "аптайм",
        ],
        constructor: |id_num| OverlayElement::ClockTimer {
            id: format!("clock_{id_num}"),
            mode: TimerMode::Clock,
            format_str: "%H:%M:%S".to_string(),
            countdown_total_ms: 300_000,
            countdown_remaining_ms: 300_000,
            stopwatch_elapsed_ms: 0,
            is_running: false,
            font_size: 16.0,
            color: [255, 255, 255, 255],
            bg_color: [18, 22, 32, 200],
            bold: true,
            font_family: "Inter".to_string(),
            x: 16.0,
            y: 16.0,
            width: 100.0,
            height: 32.0,
            rotation: 0.0,
            on_finish: None,
        },
    },
    CatalogItem {
        category_idx: 3,
        badge: "TELEMETRY",
        title: |loc| loc.elem_cat_title_telemetry,
        description: |loc| loc.elem_cat_desc_telemetry,
        glyph: egui_phosphor::regular::ACTIVITY,
        tint: |t| t.good,
        keywords: &[
            "telemetry", "battery", "audio", "mic", "engine", "status", "pulse",
            "телеметрия", "батарея", "звук", "микрофон", "движок", "пульс",
        ],
        constructor: |id_num| OverlayElement::SystemTelemetry {
            id: format!("telemetry_{id_num}"),
            kind: TelemetryKind::EngineStatus,
            style: TelemetryStyle::Compact,
            color: [255, 255, 255, 255],
            bg_color: [18, 22, 32, 200],
            x: 16.0,
            y: 16.0,
            width: 130.0,
            height: 26.0,
            rotation: 0.0,
            on_click: None,
        },
    },

    // 4. Gaming & Media
    CatalogItem {
        category_idx: 4,
        badge: "KEYS",
        title: |loc| loc.elem_cat_title_keys,
        description: |loc| loc.elem_cat_desc_keys,
        glyph: egui_phosphor::regular::KEYBOARD,
        tint: |t| t.accent_ink,
        keywords: &[
            "key", "keyboard", "mouse", "visualizer", "wasd", "rpg", "input",
            "клавиатура", "мышь", "визуализатор", "нажатия", "гейминг",
        ],
        constructor: |id_num| OverlayElement::KeyVisualizer {
            id: format!("keys_{id_num}"),
            layout: KeyVisualizerLayout::WasdMouse,
            max_keys: 5,
            decay_seconds: 1.5,
            show_modifiers_only: false,
            color: [255, 255, 255, 255],
            bg_color: [16, 20, 28, 220],
            active_color: [59, 130, 246, 255],
            x: 16.0,
            y: 16.0,
            width: 220.0,
            height: 110.0,
            rotation: 0.0,
        },
    },
    CatalogItem {
        category_idx: 4,
        badge: "RETICLE",
        title: |loc| loc.elem_cat_title_crosshair,
        description: |loc| loc.elem_cat_desc_crosshair,
        glyph: egui_phosphor::regular::CROSSHAIR,
        tint: |t| t.good,
        keywords: &[
            "crosshair", "reticle", "aim", "sight", "fps", "gaming", "dot", "cross",
            "прицел", "перекрестие", "точка", "аим", "шутер", "гейминг",
        ],
        constructor: |id_num| OverlayElement::Crosshair {
            id: format!("crosshair_{id_num}"),
            style: CrosshairStyle::Cross,
            size: 12.0,
            color: [34, 197, 94, 255],
            center_dot: true,
            thickness: 2.0,
            gap: 4.0,
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
    },
    CatalogItem {
        category_idx: 4,
        badge: "MEDIA",
        title: |loc| loc.elem_cat_title_media,
        description: |loc| loc.elem_cat_desc_media,
        glyph: egui_phosphor::regular::PLAY_CIRCLE,
        tint: |t| t.accent_ink,
        keywords: &[
            "media", "player", "music", "song", "track", "youtube", "spotify", "equalizer",
            "медиа", "плеер", "музыка", "трек", "песня", "эквалайзер", "спотифай",
        ],
        constructor: |id_num| OverlayElement::MediaWidget {
            id: format!("media_{id_num}"),
            show_controls: true,
            compact: false,
            color: [255, 255, 255, 255],
            bg_color: [18, 22, 32, 220],
            x: 16.0,
            y: 16.0,
            width: 210.0,
            height: 85.0,
            rotation: 0.0,
        },
    },
];

pub fn render_element_catalog_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.element_catalog.is_some();

    let active_modal = state
        .element_catalog
        .as_ref()
        .or(state.last_element_catalog.as_ref())
        .cloned();

    let Some(mut modal_state) = active_modal else { return };

    let loc = state.loc();
    let mut chosen_constructor: Option<fn(usize) -> OverlayElement> = None;

    let modal_res = Modal::new("element-catalog-modal", is_open)
        .width_fraction(0.72)
        .height_fraction(0.74)
        .min_size(vec2(700.0, 480.0))
        .max_size(vec2(820.0, 620.0))
        .header_height(46.0)
        .show(ctx, &t, |frame| {
            // ---------------- 1. HEADER ----------------
            frame.header(|ui, close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    widgets::hud(ui, loc.element_catalog_modal_eyebrow, t.ink);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.inspector_btn_cancel).clicked() {
                            *close_req = true;
                        }
                    });
                });
            });

            // ---------------- 2. BODY (CATEGORIES + SEARCH + ELEMENT CARDS) ----------------
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
                                CATALOG_ITEMS
                                    .iter()
                                    .filter(|i| cat_idx == 0 || i.category_idx == cat_idx)
                                    .count()
                            };

                            let categories: [(usize, &'static str, &str, usize); 5] = [
                                (0, egui_phosphor::regular::STACK, loc.element_catalog_cat_all, count_for(0)),
                                (1, egui_phosphor::regular::TEXT_T, loc.element_catalog_cat_visual, count_for(1)),
                                (2, egui_phosphor::regular::TOGGLE_RIGHT, loc.element_catalog_cat_interactive, count_for(2)),
                                (3, egui_phosphor::regular::ACTIVITY, loc.element_catalog_cat_telemetry, count_for(3)),
                                (4, egui_phosphor::regular::GAME_CONTROLLER, loc.element_catalog_cat_gaming_media, count_for(4)),
                            ];

                            for (idx, glyph, label, count) in categories {
                                let is_active = modal_state.selected_category == idx;
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
                                    glyph,
                                    FontId::new(14.0, theme::phosphor()),
                                    fg,
                                );

                                ui.painter().text(
                                    pos2(rect.left() + 32.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    label,
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
                                    modal_state.selected_category = idx;
                                }
                            }
                        });
                });

                // Right: Cards Grid in ScrollArea with Search Bar above
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
                                widgets::catalog_search_box(ui, &mut modal_state.search_query, loc.element_catalog_search_hint);
                            },
                        );
                    });
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let query = modal_state.search_query.trim().to_lowercase();
                            let filtered_items: Vec<&CatalogItem> = CATALOG_ITEMS
                                .iter()
                                .filter(|item| {
                                    if modal_state.selected_category != 0 && item.category_idx != modal_state.selected_category {
                                        return false;
                                    }
                                    if query.is_empty() {
                                        return true;
                                    }
                                    let title_match = (item.title)(loc).to_lowercase().contains(&query);
                                    let desc_match = (item.description)(loc).to_lowercase().contains(&query);
                                    let kw_match = item.keywords.iter().any(|kw| kw.to_lowercase().contains(&query));
                                    title_match || desc_match || kw_match
                                })
                                .collect();

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
                                                chosen_constructor = Some(item.constructor);
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

    if let Some(constructor) = chosen_constructor {
        if let Some(studio) = &mut state.overlay_studio {
            studio.push_undo_snapshot();
            let next_id = studio.overlay.elements.len() + 1;
            let mut elem = constructor(next_id);

            // Compute smart placement on card
            let (ew, eh) = crate::gui::components::overlay_renderer::get_element_size(&elem);
            let offset = (studio.overlay.elements.len() as f32 * 18.0) % (studio.overlay.rect.height * 0.4).max(30.0);
            let spawn_x = ((studio.overlay.rect.width - ew) * 0.5).max(12.0);
            let spawn_y = (((studio.overlay.rect.height - eh) * 0.5) + offset).max(12.0);

            if !matches!(elem, OverlayElement::Crosshair { .. }) {
                *elem.x_mut() = spawn_x.round();
                *elem.y_mut() = spawn_y.round();
            }

            studio.overlay.elements.push(elem);
            studio.selected_element_idx = Some(studio.overlay.elements.len() - 1);
        }
        state.close_element_catalog();
    } else if modal_res.as_ref().is_some_and(|r| r.close_requested) {
        state.close_element_catalog();
    } else if state.element_catalog.is_some() {
        state.element_catalog = Some(modal_state.clone());
        state.last_element_catalog = Some(modal_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n;

    #[test]
    fn test_element_catalog_items_construct_and_categories() {
        assert_eq!(CATALOG_ITEMS.len(), 14);
        let loc = i18n::get(i18n::Language::English);

        for (idx, item) in CATALOG_ITEMS.iter().enumerate() {
            let elem = (item.constructor)(idx + 1);
            let (w, h) = crate::gui::components::overlay_renderer::get_element_size(&elem);
            assert!(w > 0.0, "Element {} has invalid width: {}", elem.id(), w);
            assert!(h > 0.0, "Element {} has invalid height: {}", elem.id(), h);

            let title = (item.title)(loc);
            let desc = (item.description)(loc);
            assert!(!title.is_empty(), "Title empty for item {}", idx);
            assert!(!desc.is_empty(), "Description empty for item {}", idx);
            assert!(item.category_idx >= 1 && item.category_idx <= 4);
        }
    }

    #[test]
    fn test_overlay_studio_undo_redo_lifecycle() {
        let mut state = GuiState::new();
        state.open_overlay_studio("overlay_custom");
        let studio = state.overlay_studio.as_mut().unwrap();

        assert!(!studio.can_undo());
        assert!(!studio.can_redo());
        let initial_count = studio.overlay.elements.len();

        // Snapshot 1: before adding an element
        studio.push_undo_snapshot();
        assert!(studio.can_undo());
        assert!(!studio.can_redo());

        let elem = (CATALOG_ITEMS[0].constructor)(initial_count + 1);
        studio.overlay.elements.push(elem);
        assert_eq!(studio.overlay.elements.len(), initial_count + 1);

        // Undo -> should revert to initial count
        studio.undo();
        assert_eq!(studio.overlay.elements.len(), initial_count);
        assert!(!studio.can_undo());
        assert!(studio.can_redo());

        // Redo -> should restore added element
        studio.redo();
        assert_eq!(studio.overlay.elements.len(), initial_count + 1);
        assert!(studio.can_undo());
        assert!(!studio.can_redo());
    }
}
