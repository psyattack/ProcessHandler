//! GitBook-style dual-pane documentation and editor guide modal.
//! Provides comprehensive architectural documentation, shortcuts,
//! trigger/action deep-dive, token reference, output piping, and recipes.

use egui::{pos2, vec2, CornerRadius, Rect, RichText, Sense, Stroke, Ui};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Glyph};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::i18n::Language;

#[derive(Clone, Copy, PartialEq, Eq)]
enum GuideTopic {
    Architecture,
    Shortcuts,
    Triggers,
    Actions,
    Tokens,
    OutputPiping,
    Recipes,
}

impl GuideTopic {
    fn all() -> &'static [GuideTopic] {
        &[
            GuideTopic::Architecture,
            GuideTopic::Shortcuts,
            GuideTopic::Triggers,
            GuideTopic::Actions,
            GuideTopic::Tokens,
            GuideTopic::OutputPiping,
            GuideTopic::Recipes,
        ]
    }

    fn title(&self, lang: Language) -> &'static str {
        match (self, lang) {
            (GuideTopic::Architecture, Language::Russian) => "Концепция и Архитектура",
            (GuideTopic::Architecture, Language::English) => "Concepts & Architecture",
            (GuideTopic::Shortcuts, Language::Russian) => "Горячие клавиши и Запись",
            (GuideTopic::Shortcuts, Language::English) => "Shortcuts & Recording",
            (GuideTopic::Triggers, Language::Russian) => "Все виды Триггеров",
            (GuideTopic::Triggers, Language::English) => "Triggers Deep-Dive",
            (GuideTopic::Actions, Language::Russian) => "Пайплайн Действий",
            (GuideTopic::Actions, Language::English) => "Actions Pipeline",
            (GuideTopic::Tokens, Language::Russian) => "Токены и Контекст",
            (GuideTopic::Tokens, Language::English) => "Tokens & Context",
            (GuideTopic::OutputPiping, Language::Russian) => "Output Piping (Вывод)",
            (GuideTopic::OutputPiping, Language::English) => "Output Piping & Data Flow",
            (GuideTopic::Recipes, Language::Russian) => "Практические Рецепты",
            (GuideTopic::Recipes, Language::English) => "Practical Recipes",
        }
    }

    fn glyph(&self) -> &'static str {
        match self {
            GuideTopic::Architecture => egui_phosphor::regular::TREE_STRUCTURE,
            GuideTopic::Shortcuts => egui_phosphor::regular::COMMAND,
            GuideTopic::Triggers => egui_phosphor::regular::LIGHTNING,
            GuideTopic::Actions => egui_phosphor::regular::PLAY_CIRCLE,
            GuideTopic::Tokens => egui_phosphor::regular::CODE,
            GuideTopic::OutputPiping => egui_phosphor::regular::ARROWS_LEFT_RIGHT,
            GuideTopic::Recipes => egui_phosphor::regular::LIGHTBULB,
        }
    }
}

pub fn render_guide_modal(ctx: &egui::Context, state: &mut GuiState) {
    let is_open = state.show_guide_modal;
    let t = Tokens::get(ctx);
    let lang = state.language;
    let loc = state.loc();

    let modal_res = Modal::new("guide-modal", is_open)
        .width_fraction(0.86)
        .height_fraction(0.84)
        .min_size(vec2(800.0, 520.0))
        .max_size(vec2(1120.0, 780.0))
        .header_height(48.0)
        .header_sunken(true)
        .accent_bar(t.accent_ink)
        .show(ctx, &t, |frame| {
            // --- 1. HEADER ZONE ---
            frame.header(|ui, close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(egui_phosphor::regular::BOOK_OPEN)
                            .font(egui::FontId::new(18.0, theme::phosphor()))
                            .color(t.accent_ink),
                    );
                    ui.add_space(8.0);
                    widgets::eyebrow(ui, "GUIDE", loc.guide_eyebrow_sub);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.macro_studio_close_tip).clicked() {
                            *close_req = true;
                        }
                    });
                });
            });

            // --- 2. DUAL-PANE BODY ZONE ---
            frame.body(|ui| {
                let body_rect = ui.max_rect();
                let sidebar_w = 230.0;
                let sidebar_rect = Rect::from_min_size(body_rect.min, vec2(sidebar_w, body_rect.height()));
                let content_rect = Rect::from_min_max(
                    pos2(sidebar_rect.right() + 1.0, body_rect.top()),
                    body_rect.max,
                );

                // Divider line between sidebar and content
                ui.painter().vline(sidebar_rect.right(), body_rect.y_range(), t.hairline());

                // Left Sidebar (Table of Contents)
                ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar_rect), |ui| {
                    egui::Frame::new()
                        .fill(t.bg)
                        .inner_margin(egui::Margin { left: 10, right: 10, top: 12, bottom: 12 })
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 3.0;

                            widgets::hud(ui, loc.guide_chapters_hud, t.muted);
                            ui.add_space(4.0);

                            let topics = GuideTopic::all();
                            for (idx, topic) in topics.iter().enumerate() {
                                let is_selected = state.guide_selected_topic == idx;
                                let title = topic.title(lang);
                                let glyph = topic.glyph();

                                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                                let hovered = resp.hovered();

                                let bg_color = if is_selected {
                                    t.accent_ink.gamma_multiply(0.12)
                                } else if hovered {
                                    t.sunken
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                if bg_color != egui::Color32::TRANSPARENT {
                                    ui.painter().rect_filled(rect, CornerRadius::same(2), bg_color);
                                }

                                if is_selected {
                                    ui.painter().rect_filled(
                                        Rect::from_min_size(rect.left_top(), vec2(3.0, rect.height())),
                                        CornerRadius::same(1),
                                        t.accent_ink,
                                    );
                                }

                                let icon_color = if is_selected { t.accent_ink } else if hovered { t.ink } else { t.muted };
                                let text_color = if is_selected { t.ink_strong } else if hovered { t.ink } else { t.muted };

                                ui.painter().text(
                                    rect.left_center() + vec2(10.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    glyph,
                                    egui::FontId::new(14.0, theme::phosphor()),
                                    icon_color,
                                );

                                ui.painter().text(
                                    rect.left_center() + vec2(32.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    title,
                                    egui::FontId::new(12.0, if is_selected { theme::semibold() } else { theme::medium() }),
                                    text_color,
                                );

                                if resp.clicked() {
                                    state.guide_selected_topic = idx;
                                }
                            }
                        });
                });

                // Right Content Area
                ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                    egui::Frame::new()
                        .fill(t.bg)
                        .inner_margin(egui::Margin::symmetric(26, 20))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("guide_content_scroll")
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.spacing_mut().item_spacing.y = 8.0;

                                    let cur_topic = match state.guide_selected_topic {
                                        0 => GuideTopic::Architecture,
                                        1 => GuideTopic::Shortcuts,
                                        2 => GuideTopic::Triggers,
                                        3 => GuideTopic::Actions,
                                        4 => GuideTopic::Tokens,
                                        5 => GuideTopic::OutputPiping,
                                        _ => GuideTopic::Recipes,
                                    };

                                    render_topic_content(ui, cur_topic, &t, lang);
                                });
                        });
                });
            });
        });

    if let Some(res) = modal_res {
        if res.close_requested {
            state.show_guide_modal = false;
        }
    }
}

fn render_topic_content(ui: &mut Ui, topic: GuideTopic, t: &Tokens, lang: Language) {
    match topic {
        GuideTopic::Architecture => render_architecture_guide(ui, t, lang),
        GuideTopic::Shortcuts => render_shortcuts_guide(ui, t, lang),
        GuideTopic::Triggers => render_triggers_guide(ui, t, lang),
        GuideTopic::Actions => render_actions_guide(ui, t, lang),
        GuideTopic::Tokens => render_tokens_guide(ui, t, lang),
        GuideTopic::OutputPiping => render_output_piping_guide(ui, t, lang),
        GuideTopic::Recipes => render_recipes_guide(ui, t, lang),
    }
}

// ----------------------------------------------------------------------------
// Helpers for GitBook-style formatting
// ----------------------------------------------------------------------------

fn doc_header(ui: &mut Ui, title: &str, t: &Tokens) {
    ui.add(
        egui::Label::new(
            RichText::new(title)
                .family(theme::display())
                .size(17.0)
                .color(t.ink_strong),
        )
        .wrap()
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
    ui.add_space(3.0);
    ui.painter().hline(ui.available_rect_before_wrap().x_range(), ui.cursor().top() + 1.0, t.hairline());
    ui.add_space(6.0);
}

fn doc_section_title(ui: &mut Ui, text: &str, t: &Tokens) {
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .family(theme::semibold())
                .size(13.0)
                .color(t.ink_strong),
        )
        .wrap()
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
    ui.add_space(2.0);
}

fn doc_paragraph(ui: &mut Ui, text: &str, t: &Tokens) {
    ui.add(
        egui::Label::new(RichText::new(text).size(12.5).color(t.ink))
            .wrap()
            .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

fn doc_callout(ui: &mut Ui, title: &str, body: &str, t: &Tokens, is_tip: bool) {
    let border_color = if is_tip { t.accent_ink } else { t.line_strong };
    let bg = t.sunken;

    ui.add_space(3.0);
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border_color.gamma_multiply(0.35)))
        .corner_radius(CornerRadius::same(2))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let icon = if is_tip { egui_phosphor::regular::LIGHTBULB } else { egui_phosphor::regular::INFO };
                ui.label(
                    RichText::new(icon)
                        .font(egui::FontId::new(15.0, theme::phosphor()))
                        .color(border_color),
                );
                ui.label(RichText::new(title).family(theme::semibold()).size(13.0).color(t.ink_strong));
            });
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(RichText::new(body).size(12.0).color(t.ink))
                    .wrap()
                    .wrap_mode(egui::TextWrapMode::Wrap),
            );
        });
    ui.add_space(3.0);
}

fn doc_code_block(ui: &mut Ui, code: &str, t: &Tokens) {
    ui.add_space(3.0);
    egui::Frame::new()
        .fill(t.sunken)
        .stroke(t.hairline())
        .corner_radius(CornerRadius::same(2))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(code)
                        .family(theme::mono())
                        .size(11.5)
                        .color(t.accent_ink),
                )
                .wrap()
                .wrap_mode(egui::TextWrapMode::Wrap),
            );
        });
    ui.add_space(3.0);
}

fn doc_table_row(ui: &mut Ui, col1: &str, col2: &str, t: &Tokens) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let chip = RichText::new(col1).family(theme::mono_bold()).size(11.5).color(t.accent_ink);
        ui.label(chip);
        ui.label(RichText::new("—").color(t.muted));
        ui.add(
            egui::Label::new(RichText::new(col2).size(12.0).color(t.ink))
                .wrap()
                .wrap_mode(egui::TextWrapMode::Wrap),
        );
    });
}

// ----------------------------------------------------------------------------
// Chapter 01: Architecture
// ----------------------------------------------------------------------------
fn render_architecture_guide(ui: &mut Ui, t: &Tokens, lang: Language) {
    if lang == Language::Russian {
        doc_header(ui, "Концепция и Архитектура Правил", t);
        doc_paragraph(ui, "ProcessHandler работает на базе детерминированного асинхронного движка событий. Каждое правило (Rule) — это независимый контур автоматизации со своим жизненным циклом.", t);

        doc_callout(
            ui,
            "Анатомия Правила",
            "1. Триггер (Trigger) — единственный источник события (запуск процесса, фокус окна, таймер, горячая клавиша, файл или вебхук).\n\
             2. Actions Pipeline — последовательная цепочка основных шагов, исполняемых сверху вниз.\n\
             3. Exit Actions — завершающие шаги, исполняемые при закрытии отслеживаемого процесса или завершении задачи.",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_section_title(ui, "Параметры контроля и защиты:", t);
        doc_table_row(ui, "Cooldown (сек)", "Минимальный интервал между повторными срабатываниями (защита от спама и циклов).", t);
        doc_table_row(ui, "Квота выполнений", "Максимальное число запусков правила, после которого оно автоматически становится на паузу.", t);
        doc_table_row(ui, "Остановка при ошибке", "Прерывание цепочки действий, если один из шагов завершился аварийно.", t);
        doc_table_row(ui, "Single Instance", "Защита от запуска дубликатов процесса (блокирует создание второго окна приложения).", t);
        doc_table_row(ui, "Test Rule / Step", "Проверка всей цепочки или отдельного шага прямо в редакторе без ожидания триггера.", t);
        doc_table_row(ui, "Backup & Sharing", "Экспорт и импорт правил в формате JSON для резервных копий и переноса настроек.", t);

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Companion Mode (Сопровождение процесса)",
            "Если триггером выбран процесс, ProcessHandler переходит в режим компаньона: удерживает контекст PID процесса во время его работы и автоматически вызывает Exit Actions при его завершении.",
            t,
            false,
        );
    } else {
        doc_header(ui, "Concepts & Architecture", t);
        doc_paragraph(ui, "ProcessHandler operates on a deterministic, asynchronous event loop. Each automation is configured as an autonomous Rule with an isolated lifecycle.", t);

        doc_callout(
            ui,
            "Rule Anatomy",
            "1. Trigger — the single origin event (process start, window focus, schedule, hotkey, file, or webhook).\n\
             2. Actions Pipeline — ordered sequential pipeline of steps executed top-to-bottom.\n\
             3. Exit Actions — optional clean-up actions executed when the tracked process terminates.",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_section_title(ui, "Execution Controls:", t);
        doc_table_row(ui, "Cooldown (sec)", "Minimum elapsed time before the rule can trigger again (prevents event flooding).", t);
        doc_table_row(ui, "Execution Quota", "Auto-disables the rule after completing N executions.", t);
        doc_table_row(ui, "Stop on Error", "Aborts pipeline execution immediately if any action reports a failure.", t);
        doc_table_row(ui, "Single Instance", "Prevents duplicate instances of tracked process from running.", t);
        doc_table_row(ui, "Test Rule / Step", "Test the full pipeline or single actions directly in the studio without waiting for real triggers.", t);
        doc_table_row(ui, "Backup & Sharing", "Export and import automations as JSON files for easy migration and sharing.", t);

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Companion Mode",
            "When configured with a Process trigger, ProcessHandler tracks the lifecycle of the executable, maintaining runtime PID context and triggering Exit Actions upon process exit.",
            t,
            false,
        );
    }
}

// ----------------------------------------------------------------------------
// Chapter 02: Shortcuts & Recording
// ----------------------------------------------------------------------------
fn render_shortcuts_guide(ui: &mut Ui, t: &Tokens, lang: Language) {
    if lang == Language::Russian {
        doc_header(ui, "Горячие Клавиши и Запись", t);
        doc_paragraph(ui, "В ProcessHandler встроен мощный движок истории изменений (Undo/Redo), глобальный аппаратный рекордер макросов и интерактивные редакторы.", t);

        doc_callout(
            ui,
            "Клавиатурные Сочетания Редактора",
            "• Ctrl + Z — Отмена последнего действия (Undo). Поддерживает историю до 50 снимков состояния.\n\
             • Ctrl + Y или Ctrl + Shift + Z — Повтор отмененного действия (Redo).\n\
             • F9 — Старт / стоп аппаратной записи макроса из любого окна Windows.\n\
             • F10 — Быстрый запуск пробного тестирования макроса (Test Run live).\n\
             • Escape — Мгновенная остановка записи макроса или закрытие модальных окон.",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_section_title(ui, "Студия Макросов (Macro Studio):", t);
        doc_paragraph(ui, "Единый шаг «Аппаратный Макрос» объединяет физические нажатия клавиш (KeyDown / KeyUp), управление кнопками мыши (MouseDown / MouseUp), ввод текста, паузы и плавные перемещения курсора. Нажатие F9 запускает низкоуровневый перехват Windows (WH_KEYBOARD_LL / WH_MOUSE_LL), фиксируя действия с фильтрами записи (клавиатура, клики, перемещения, колесо, задержки). Повторное нажатие F9 или Escape останавливает запись с подтверждающим звуковым сигналом.", t);

        ui.add_space(2.0);
        doc_section_title(ui, "Возможности Студии Макросов (Macro Studio):", t);
        doc_table_row(ui, "F9 / Кнопка Записи", "Низкоуровневая запись клавиатуры и мыши с гибкими фильтрами: клавиши, клики, курсор, колесо и паузы", t);
        doc_table_row(ui, "Drag & Drop со скроллом", "Изменение порядка шагов перетаскиванием; во время переноса работает колесико мыши и автоскролл у границ холста", t);
        doc_table_row(ui, "Группировка Движений", "Последовательные перемещения мыши без задержек автоматически объединяются в раскрываемую серию «A → B», сохраняя точную траекторию без захламления холста", t);
        doc_table_row(ui, "Вся Органика", "Случайный человеческий разброс задержек (±30% jitter) на все паузы и печать текста для обхода бот-детекторов", t);
        doc_table_row(ui, "Объединять нажатия", "Визуальное объединение связанных шагов (Зажатие + Пауза + Отпускание) в единые удобные карточки", t);
        doc_table_row(ui, "Бесконечный режим", "Непрерывное циклическое выполнение макроса; выключается повторным нажатием хоткея или отпусканием кнопки в Hold mode", t);
        doc_table_row(ui, "Захват Точки", "Инструмент захвата экранных координат: наведите мышь на экран и нажмите Space или Enter для фиксации", t);
        doc_table_row(ui, "Физические События", "Раздельные события зажатия (Down) и отпускания (Up) для клавиатуры и мыши", t);
        doc_table_row(ui, "Прокрутка Колеса", "Возможность записи и ручной вставки шагов прокрутки колесика мыши (+ Колесо)", t);
        doc_table_row(ui, "Дублирование и Удаление", "Быстрое клонирование шагов и удаление ненужных событий аккуратными значками без лишних рамок", t);
        doc_table_row(ui, "Повтор и Скорость", "Множитель скорости воспроизведения (0.1x – 10x) и цикл повторов макроса", t);
        doc_table_row(ui, "Повороты в 3D-Играх", "Запись и воспроизведение поворотов камеры в играх через Raw Input; переключение [КУРСОР] ↔ [ПОВОРОТ] кликом по бейджику", t);
        doc_table_row(ui, "Защита от Залипания", "При отмене макроса или отпускании хоткея движок мгновенно сбрасывает все физически зажатые клавиши и кнопки мыши", t);
        doc_table_row(ui, "DirectInput и ScanCodes", "Аппаратная симуляция скан-кодов с выдержкой нажатия (35+ мс), совместимая с играми и полноэкранными приложениями", t);
        doc_table_row(ui, "Безопасный Тест (F10)", "Запуск и остановка тестового прогона макроса на изолированном фоновом потоке без вылетов и зависаний", t);
    } else {
        doc_header(ui, "Keyboard Shortcuts & Recording", t);
        doc_paragraph(ui, "ProcessHandler features deep state history tracking (Undo/Redo), a global hardware macro recorder, and interactive studio editors.", t);

        doc_callout(
            ui,
            "Studio Editor Shortcuts",
            "• Ctrl + Z — Undo previous rule edit (supports up to 50 historical snapshots).\n\
             • Ctrl + Y or Ctrl + Shift + Z — Redo reverted modification.\n\
             • F9 — Start / Stop global hardware macro recording from any window in Windows.\n\
             • F10 — Start / Stop live safe test run of the macro sequence.\n\
             • Escape — Immediately stops recording, cancels active Drag & Drop, or closes modal window.",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_section_title(ui, "Macro Studio & Timeline:", t);
        doc_paragraph(ui, "The unified «Hardware Macro» action unifies physical key presses (KeyDown / KeyUp), mouse buttons (MouseDown / MouseUp), plain text typing, delays, scroll wheel steps, and smooth cursor motion. Pressing F9 activates low-level Windows hooks (WH_KEYBOARD_LL / WH_MOUSE_LL), capturing events with selective filters (keys, clicks, cursor moves, scroll wheel, delays). Pressing F9 or Escape cleanly stops recording with an audio chime.", t);

        ui.add_space(2.0);
        doc_section_title(ui, "Macro Studio Capabilities:", t);
        doc_table_row(ui, "F9 / Record Button", "Low-level hardware recording with modular filters for keys, clicks, moves, wheel, and delays", t);
        doc_table_row(ui, "Drag & Drop with Scrolling", "Intuitive step reordering; supports mouse wheel scrolling and boundary auto-scrolling while dragging", t);
        doc_table_row(ui, "Stacked Move Sequences", "Sequential cursor movements without delays automatically stack into an expandable «A → B» group, preserving exact coordinates without canvas clutter", t);
        doc_table_row(ui, "Global Organic Delay", "Applies humanized timing jitter (±30%) across all delays and text typing to bypass anti-bot detection", t);
        doc_table_row(ui, "Group Presses", "Visually groups connected Down, Delay, and Up events into unified full-press cards", t);
        doc_table_row(ui, "Infinite Mode", "Continuous macro looping; stops via hotkey toggle or immediately upon release in Hold mode", t);
        doc_table_row(ui, "Coordinate Picker", "Target position tool: hover anywhere on desktop and press Space or Enter to lock exact X/Y", t);
        doc_table_row(ui, "Physical Hardware Actions", "Separated Down / Up events for keys and mouse buttons matching real hardware physics", t);
        doc_table_row(ui, "Scroll Wheel", "Full support for capturing and manually adding mouse wheel scrolling steps (+ Wheel)", t);
        doc_table_row(ui, "Clone & Delete", "Clean ghost actions for fast duplication and deletion without clunky button outlines", t);
        doc_table_row(ui, "Repeat & Speed", "Playback speed multiplier (0.1x – 10x) and loop iteration repeat counts", t);
        doc_table_row(ui, "In-Game 3D Camera Turns", "Live capture and playback of camera rotation via Raw Input relative deltas; toggle [MOVE] ↔ [TURN] on the badge", t);
        doc_table_row(ui, "Anti-Stuck Key Safety", "Cancelling a macro or releasing a Hold hotkey immediately releases all simulated down keys and mouse buttons", t);
        doc_table_row(ui, "DirectInput ScanCodes", "Hardware scan-code simulation with frame-safe press durations (35+ ms) for compatibility with games and full-screen engines", t);
        doc_table_row(ui, "Safe Test Run (F10)", "Toggle test execution on an isolated background thread without freezing or crashes", t);
    }
}

// ----------------------------------------------------------------------------
// Chapter 03: Triggers Deep-Dive
// ----------------------------------------------------------------------------
fn render_triggers_guide(ui: &mut Ui, t: &Tokens, lang: Language) {
    if lang == Language::Russian {
        doc_header(ui, "Все Виды Триггеров", t);
        doc_paragraph(ui, "Триггер определяет, когда именно запускается автоматизация. Доступно 5 основных категорий:", t);

        doc_table_row(ui, "PROCESS", "Срабатывает при запуске исполняемого файла. Режимы: Exact (точное имя), Wildcard, Regex. Опция Single Instance предотвращает повторный запуск.", t);
        doc_table_row(ui, "WINDOW", "Срабатывает при смене активности окна: фокус получен (Gained Focus), потерян (Lost Focus) или оба события.", t);
        doc_table_row(ui, "SCHEDULE", "Запуск по таймеру: Interval (каждые N секунд), Daily (ежедневно в заданное время по дням недели) или One-Shot (в точную дату).", t);
        doc_table_row(ui, "EVENTS", "Системные события: запуск/закрытие ProcessHandler, блокировка Windows, спящий режим, переключение питания сеть/батарея, простой пользователя.", t);
        doc_table_row(ui, "SYSTEM", "Расширенные системные триггеры: порог ресурсов CPU/RAM, глобальный хоткей, файловый монитор (File Watcher) и локальный Webhook.", t);

        ui.add_space(2.0);
        doc_callout(
            ui,
            "File Watcher для папок и одиночных файлов",
            "File Watcher умеет следить как за целыми папками, так и за отдельным конкретным файлом. Отслеживаются события: On Created, On Modified, On Deleted.",
            t,
            false,
        );

        doc_callout(
            ui,
            "Resource Thresholds (Порог CPU и RAM)",
            "Позволяет отслеживать превышение процента CPU (%) или объема памяти RAM (MB) конкретным процессом. Правило срабатывает только если нагрузка удерживается дольше заданного времени в секундах.",
            t,
            false,
        );

        doc_callout(
            ui,
            "Webhook Trigger",
            "Локальный REST-сервер слушает вызовы по адресу http://127.0.0.1:48123/api/v1/trigger/<endpoint_id>. Совместим с Stream Deck, Home Assistant, curl и браузерными закладками.",
            t,
            false,
        );

        doc_callout(
            ui,
            "Глобальные Хоткеи и Режим Удержания (Hold mode)",
            "• Поддержка клавиш и мыши: в качестве хоткея можно задавать любые клавиатурные сочетания, а также кнопки мыши: M4, M5, ПКМ, СКМ, ЛКМ.\n\
             • Режим Hold mode: правило непрерывно выполняется пока физически зажата клавиша или кнопка мыши, и мгновенно прерывается при ее отпускании.\n\
             • Повтор при удержании (repeat_while_held): циклический повтор пайплайна пока удерживается хоткей, со строгой синхронизацией квоты (max_executions). При отпускании все зажатые макросом кнопки сбрасываются мгновенно.\n\
             • Обычный режим: повторное нажатие хоткея переключает состояние и останавливает текущее выполнение правила или бесконечного макроса.",
            t,
            false,
        );
    } else {
        doc_header(ui, "Triggers Deep-Dive", t);
        doc_paragraph(ui, "A trigger defines the precise condition that initiates automation. 5 main categories are supported:", t);

        doc_table_row(ui, "PROCESS", "Triggers when an executable starts. Match modes: Exact, Wildcard, Regex. Single Instance prevents multiple instances.", t);
        doc_table_row(ui, "WINDOW", "Triggers on foreground window changes: focus Gained, focus Lost, or both.", t);
        doc_table_row(ui, "SCHEDULE", "Time-based executions: Interval (every N seconds), Daily (at specified time on selected days), or One-Shot (exact date).", t);
        doc_table_row(ui, "EVENTS", "System OS hooks: App start/stop, session lock/unlock, system sleep/resume, AC/battery power, user idle.", t);
        doc_table_row(ui, "SYSTEM", "Advanced triggers: Resource thresholds (CPU/RAM), global keyboard/mouse hotkeys with Hold mode, File Watcher, and local HTTP Webhooks.", t);

        ui.add_space(2.0);
        doc_callout(
            ui,
            "File Watcher for Directories and Single Files",
            "File Watcher monitors either entire directories or single specific files. Supported events: On Created, On Modified, On Deleted.",
            t,
            false,
        );

        doc_callout(
            ui,
            "Resource Thresholds (CPU & RAM)",
            "Monitors CPU usage (%) or Working Set memory (MB) of a target process. The rule executes only when the consumption exceeds limits continuously for the configured duration.",
            t,
            false,
        );

        doc_callout(
            ui,
            "Webhook Trigger",
            "Local REST server listens for incoming calls at http://127.0.0.1:48123/api/v1/trigger/<endpoint_id>. Compatible with Stream Deck, Home Assistant, curl, and browser bookmarks.",
            t,
            false,
        );

        doc_callout(
            ui,
            "Global Hotkeys & Hold Mode",
            "• Keyboard & Mouse Buttons: hotkey combinations support keyboard keys as well as mouse buttons: M4, M5, RMB, MMB, LMB.\n\
             • Hold Mode: execution continues while the hotkey or mouse button is physically held down, and automatically terminates on release.\n\
             • Loop While Held: when Hold Mode is active, repeat pipeline execution continuously in a loop while held (respects execution quota).\n\
             • Toggle Off: in standard trigger mode, pressing the hotkey again toggles execution off, cleanly stopping running rules or infinite macros.",
            t,
            false,
        );
    }
}

// ----------------------------------------------------------------------------
// Chapter 04: Actions Pipeline
// ----------------------------------------------------------------------------
fn render_actions_guide(ui: &mut Ui, t: &Tokens, lang: Language) {
    if lang == Language::Russian {
        doc_header(ui, "Пайплайн Действий", t);
        doc_paragraph(ui, "Actions Pipeline позволяет составлять цепочки из унифицированных и высокоточных действий:", t);

        doc_table_row(ui, "Launch / Kill Process", "Запуск со скрытием окна или от админа; завершение процессов с деревом потоков.", t);
        doc_table_row(ui, "Скрипты (PS / CMD / Lua)", "Единый шаг для PowerShell, CMD/Batch и встроенного Lua с передачей вывода в {PrevOutput}.", t);
        doc_table_row(ui, "Аппаратный Макрос", "Единый воркфлоу клавиатуры и мыши: зажатия, клики, перемещения, плавный ход, текст, паузы и запись по F9.", t);
        doc_table_row(ui, "Управление Звуком", "Единый контроллер громкости динамиков и микрофона (Mute, Unmute, Toggle, Set Volume, Beep).", t);
        doc_table_row(ui, "Устройство по Умолчанию", "Мгновенное переключение основного аудиовыхода или микрофона по имени устройства.", t);
        doc_table_row(ui, "Window & Display", "Управление окнами (свернуть, прижать, TopMost), выбор целевого монитора, разрешение и Гц.", t);
        doc_table_row(ui, "Звук Процессов", "Регулировка громкости и глушение звука конкретных запущенных приложений.", t);
        doc_table_row(ui, "Tuning & Firewall", "Изменение приоритета процессов, сброс Working Set RAM и блокировка сетевого трафика.", t);
        doc_table_row(ui, "HTTP Webhook (Outbound)", "Отправка исходящих REST-запросов (GET, POST, PUT, DELETE) с заголовками и телом.", t);
        doc_table_row(ui, "Power & System", "Схемы питания Windows; переход в сон, гибернация, перезагрузка, выключение и блокировка.", t);
        doc_table_row(ui, "Delay & Notification", "Асинхронные паузы между шагами, всплывающие уведомления Windows и воспроизведение WAV.", t);

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Единая Студия Макросов и Управление Мышью",
            "• Объединенный конвейер: любые комбинации физических нажатий клавиш, кнопок мыши, прокрутки колеса, набора текста, пауз и перемещений курсора.\n\
             • Интерактивная Студия: модальное окно с визуальным таймлайном, Drag & Drop сортировкой со скроллом колесиком и безопасным тестом (F10).\n\
             • Повороты Камеры в Играх (Relative Mode): воспроизведение импульсов смещения ΔX, ΔY с линейной угловой скоростью и частотой 200–300 Гц для плавного движения камеры в 3D-играх.\n\
             • Запись по F9: глобальный аппаратный перехват в реальном времени с настраиваемыми фильтрами и звуковым подтверждением.\n\
             • Бесконечный Режим: циклическое воспроизведение макроса с возможностью прерывания по хоткею или при отпускании в Hold mode.\n\
             • Группировка Движений: последовательные перемещения курсора без задержек автоматически объединяются в раскрываемый блок «A → B» без потери координат.\n\
             • Вся Органика: включение случайных микро-флуктуаций (±30%) на все паузы для обхода бот-детекторов.\n\
             • Объединять Нажатия: компактное группирование связанных шагов (Зажатие + Пауза + Отпускание) в единую карточку.",
            t,
            false,
        );
    } else {
        doc_header(ui, "Actions Pipeline", t);
        doc_paragraph(ui, "The Actions Pipeline allows constructing sequences from clean, unified action modules:", t);

        doc_table_row(ui, "Launch / Kill Process", "Run hidden/admin; terminate processes with full process tree kill.", t);
        doc_table_row(ui, "Script (PS / CMD / Lua)", "Unified runner for PowerShell, CMD/Batch, and Lua with piped output in {PrevOutput}.", t);
        doc_table_row(ui, "Hardware Macro", "Unified keyboard and mouse workflow: press/release, clicks, cursor move, wheel scroll, typing, delays, and F9 recording.", t);
        doc_table_row(ui, "Audio & Mic Control", "Unified volume and mute controller for both playback speakers and microphones.", t);
        doc_table_row(ui, "Default Audio Device", "Switch default system playback audio output or recording microphone by name.", t);
        doc_table_row(ui, "Window & Display", "Window positioning, TopMost on/off, target monitor index selection, resolution & Hz.", t);
        doc_table_row(ui, "Per-Process Audio", "Adjust volume or mute specific running application audio sessions.", t);
        doc_table_row(ui, "Tuning & Firewall", "Process priority class, RAM memory trim, and Windows Firewall traffic blocking.", t);
        doc_table_row(ui, "HTTP Webhook (Outbound)", "Send outbound HTTP requests (GET, POST, PUT, DELETE) with custom headers and body.", t);
        doc_table_row(ui, "Power & System", "Windows power plans; sleep, hibernate, restart, shutdown, and workstation lock.", t);
        doc_table_row(ui, "Delay & Notification", "Non-blocking delay pauses in pipeline, Windows desktop toast notifications, and WAV audio chimes.", t);

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Unified Macro Studio & Smart Mouse Control",
            "• Unified Workflow: seamlessly combine physical key presses, mouse buttons, wheel scroll, text typing, delays, and cursor motion.\n\
             • Interactive Studio: full-featured modal canvas with visual event rows, smooth Drag & Drop reordering with wheel scroll, and test run (F10).\n\
             • In-Game 3D Camera Turns: relative displacement pulses ΔX, ΔY paced at 200–300 Hz with linear angular velocity for smooth camera rotations in 3D games.\n\
             • F9 Global Recording: real-time low-level hardware capture with customizable action filters and audio chime confirmation.\n\
             • Infinite Mode: continuous macro cycling with on-demand cancellation via hotkey toggle or release in Hold mode.\n\
             • Stacked Move Sequences: sequential mouse movements without delays automatically group into an expandable «A → B» block without losing precision.\n\
             • Global Organic Delays: optional random humanized jitter (±30%) across all pauses and typing to evade anti-bot detection.\n\
             • Group Presses: optional visual grouping of connected Down, Delay, and Up events into single cards.",
            t,
            false,
        );
    }
}

// ----------------------------------------------------------------------------
// Chapter 05: Tokens & Context
// ----------------------------------------------------------------------------
fn render_tokens_guide(ui: &mut Ui, t: &Tokens, lang: Language) {
    if lang == Language::Russian {
        doc_header(ui, "Токены и Контекст Выполнения", t);
        doc_paragraph(ui, "В текстовых полях любого действия можно указывать плейсхолдеры в фигурных скобках {}. Движок автоматически заменит их перед исполнением.", t);

        doc_section_title(ui, "Токены процессов и окон:", t);
        doc_table_row(ui, "{ProcessName}", "Имя исполняемого файла процесса (напр. notepad.exe)", t);
        doc_table_row(ui, "{ProcessId}", "Числовой PID процесса в системе", t);
        doc_table_row(ui, "{ProcessPath}", "Полный путь к .exe файлу (C:\\Program Files\\...)", t);
        doc_table_row(ui, "{ProcessDir}", "Папка, в которой лежит исполняемый файл", t);
        doc_table_row(ui, "{WindowTitle}", "Заголовок текущего активного окна", t);

        ui.add_space(2.0);
        doc_section_title(ui, "Токены File Watcher:", t);
        doc_table_row(ui, "{FilePath}", "Полный путь к созданному или измененному файлу", t);
        doc_table_row(ui, "{FileName}", "Имя файла с расширением (напр. report.csv)", t);
        doc_table_row(ui, "{FileDir}", "Директория, где находится отслеживаемый файл", t);

        ui.add_space(2.0);
        doc_section_title(ui, "Глобальные токены:", t);
        doc_table_row(ui, "{Clipboard}", "Текущий текст из буфера обмена Windows", t);
        doc_table_row(ui, "{Date}", "Сегодняшняя дата (ГГГГ-ММ-ДД)", t);
        doc_table_row(ui, "{Time}", "Текущее время (ЧЧ:ММ:СС)", t);
        doc_table_row(ui, "{Env:NAME}", "Значение переменной окружения Windows (напр. {Env:USERPROFILE})", t);
    } else {
        doc_header(ui, "Available Tokens & Execution Context", t);
        doc_paragraph(ui, "You can embed placeholders inside braces {} in any text field. ProcessHandler interpolates them at runtime.", t);

        doc_section_title(ui, "Process & Window Tokens:", t);
        doc_table_row(ui, "{ProcessName}", "Process executable name (e.g. notepad.exe)", t);
        doc_table_row(ui, "{ProcessId}", "Operating system Process ID (PID)", t);
        doc_table_row(ui, "{ProcessPath}", "Absolute path to executable", t);
        doc_table_row(ui, "{ProcessDir}", "Directory hosting the executable", t);
        doc_table_row(ui, "{WindowTitle}", "Foreground window title text", t);

        ui.add_space(2.0);
        doc_section_title(ui, "File Watcher Tokens:", t);
        doc_table_row(ui, "{FilePath}", "Full path to created, modified, or deleted file", t);
        doc_table_row(ui, "{FileName}", "File name including extension", t);
        doc_table_row(ui, "{FileDir}", "Parent folder containing the file", t);

        ui.add_space(2.0);
        doc_section_title(ui, "Global Tokens:", t);
        doc_table_row(ui, "{Clipboard}", "Current text contents of Windows clipboard", t);
        doc_table_row(ui, "{Date}", "Current date (YYYY-MM-DD)", t);
        doc_table_row(ui, "{Time}", "Current time (HH:MM:SS)", t);
        doc_table_row(ui, "{Env:NAME}", "Environment variable value (e.g. {Env:USERPROFILE})", t);
    }
}

// ----------------------------------------------------------------------------
// Chapter 06: Output Piping
// ----------------------------------------------------------------------------
fn render_output_piping_guide(ui: &mut Ui, t: &Tokens, lang: Language) {
    if lang == Language::Russian {
        doc_header(ui, "Output Piping (Конвейер Вывода)", t);
        doc_paragraph(ui, "Output Piping позволяет передавать данные между шагами в Actions Pipeline без использования временных файлов.", t);

        doc_callout(
            ui,
            "Как работает перехват вывода",
            "1. Если шаг (PowerShell, CMD или Lua) возвращает текст (stdout или return), движок сохраняет его в контекст.\n\
             2. {PrevOutput} — содержит вывод непосредственно предыдущего шага.\n\
             3. {Step1.Output}, {Step2.Output} — содержат вывод конкретного шага с номером N.",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_section_title(ui, "Пример использования:", t);
        doc_code_block(
            ui,
            "# Шаг 1: PowerShell (получение внешнего IP)\n\
             (Invoke-WebRequest -Uri 'https://api.ipify.org').Content\n\n\
             # Шаг 2: Notification\n\
             Заголовок: Внешний IP\n\
             Текст: Ваш публичный адрес: {PrevOutput}",
            t,
        );

        ui.add_space(2.0);
        doc_paragraph(ui, "В PowerShell все переменные контекста также доступны как переменные окружения с префиксом `$env:PH_...` (например `$env:PH_PROCESSNAME`, `$env:PH_PREVOUTPUT`).", t);
        ui.add_space(2.0);
        doc_paragraph(ui, "В скриптах Lua достаточно вернуть строку через `return \"результат\"`, чтобы движок передал её в {PrevOutput}. Также доступен метод `ph.get_prev_output()`.", t);
    } else {
        doc_header(ui, "Output Piping & Data Flow", t);
        doc_paragraph(ui, "Output Piping allows passing output between actions without intermediate files.", t);

        doc_callout(
            ui,
            "Output Flow Mechanics",
            "1. When PowerShell, CMD, or Lua executes and returns output, it is automatically captured.\n\
             2. {PrevOutput} — contains the output of the immediately preceding action.\n\
             3. {Step1.Output}, {Step2.Output} — contains the output of step number N.",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_section_title(ui, "Pipeline Example:", t);
        doc_code_block(
            ui,
            "# Step 1: PowerShell (fetches external IP)\n\
             (Invoke-WebRequest -Uri 'https://api.ipify.org').Content\n\n\
             # Step 2: Notification\n\
             Title: Public IP\n\
             Message: Current IP address: {PrevOutput}",
            t,
        );

        ui.add_space(2.0);
        doc_paragraph(ui, "In PowerShell, all context variables are also accessible as environment variables prefixed with `$env:PH_...` (e.g. `$env:PH_PROCESSNAME`, `$env:PH_PREVOUTPUT`).", t);
        ui.add_space(2.0);
        doc_paragraph(ui, "In Lua scripts, returning a string via `return \"result\"` captures it into {PrevOutput}. The API helper `ph.get_prev_output()` is also available.", t);
    }
}

// ----------------------------------------------------------------------------
// Chapter 07: Practical Recipes
// ----------------------------------------------------------------------------
fn render_recipes_guide(ui: &mut Ui, t: &Tokens, lang: Language) {
    if lang == Language::Russian {
        doc_header(ui, "Практические Рецепты Автоматизации", t);

        doc_callout(
            ui,
            "Рецепт: Игровой Турбо-Режим (Game Boost)",
            "• Триггер: Process (game.exe)\n\
             • Шаг 1: Power Plan -> High Performance\n\
             • Шаг 2: Process Tuning -> Priority: High\n\
             • Exit Action: Power Plan -> Balanced (автоматически вернет баланс при выходе из игры)",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Рецепт: Авто-резервное копирование документов",
            "• Триггер: File Watcher -> путь к папке C:\\MyWork, расширение: .docx, событие: On Modified\n\
             • Шаг 1: CMD -> copy \"{FilePath}\" \"D:\\Backups\\{FileName}_{Date}.bak\"\n\
             • Шаг 2: Notification -> Заголовок: \"Бэкап\", Сообщение: \"{FileName} сохранен в архив\"",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Рецепт: Управление через Stream Deck или Webhook",
            "• Триггер: Webhook -> Endpoint ID: \"toggle_work\"\n\
             • Шаг 1: Launch Process -> запуск IDE или корпоративного чата\n\
             • Шаг 2: Play Sound -> системный сигнал подтверждения",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Рецепт: Организация рабочего пространства",
            "• Триггер: Window Focus -> Gained Focus для code.exe\n\
             • Шаг 1: Window -> Snap Left (прижать окно к левой половине экрана)\n\
             • Шаг 2: Audio Device -> переключение звука на наушники\n\
             • Шаг 3: Process Tuning -> Priority: Above Normal",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Рецепт: Автоматическая очистка памяти при утечке",
            "• Триггер: Resource Threshold -> RAM > 3000 MB на протяжении 30 сек\n\
             • Шаг 1: Process Tuning -> Trim Working Set RAM (мгновенный сброс неиспользуемой памяти)\n\
             • Шаг 2: Notification -> Заголовок: \"Оптимизация\", Сообщение: \"Память {ProcessName} очищена\"",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Рецепт: Энергосбережение ноутбука при отключении от сети",
            "• Триггер: System Event -> Power: On Battery (переход на питание от аккумулятора)\n\
             • Шаг 1: Power Plan -> Power Saver (режим экономии батареи)\n\
             • Шаг 2: Display -> Частота 60 Hz (снижение с 144/240 Гц для сохранения заряда)\n\
             • Шаг 3: Audio Control -> Уровень громкости 25%",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Рецепт: Автоматизация кликов и ввода (Аппаратный Макрос)",
            "• Триггер: Global Hotkey -> Ctrl + Alt + M (или кнопка мыши, напр. M4 в режиме Hold mode)\n\
             • Шаг 1: Аппаратный Макрос (Hardware Macro) в Студии:\n\
               — Перемещение мыши в координаты (1400, 450) + Клик ЛКМ\n\
               — Набор текста «Status: Approved» с органической задержкой\n\
               — Физическое нажатие клавиши Enter\n\
               — Клик боковой кнопкой мыши M4 (Back) для навигации назад",
            t,
            false,
        );
    } else {
        doc_header(ui, "Practical Automation Recipes", t);

        doc_callout(
            ui,
            "Recipe: Game Booster",
            "• Trigger: Process (game.exe)\n\
             • Step 1: Power Plan -> High Performance\n\
             • Step 2: Process Tuning -> Priority: High\n\
             • Exit Action: Power Plan -> Balanced (restores automatically on game exit)",
            t,
            true,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Recipe: File Auto-Backup",
            "• Trigger: File Watcher -> C:\\WorkDocs, extension: .docx, event: On Modified\n\
             • Step 1: CMD -> copy \"{FilePath}\" \"D:\\Backups\\{FileName}_{Date}.bak\"\n\
             • Step 2: Notification -> Title: \"Backup\", Message: \"{FileName} archived\"",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Recipe: Stream Deck or Webhook Control",
            "• Trigger: Webhook -> Endpoint ID: \"toggle_work\"\n\
             • Step 1: Launch Process -> launch IDE or corporate chat\n\
             • Step 2: Play Sound -> system confirmation chime",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Recipe: Workspace Layout & Audio Routing",
            "• Trigger: Window Focus -> Gained Focus for code.exe\n\
             • Step 1: Window -> Snap Left (dock window to left half of screen)\n\
             • Step 2: Audio Device -> switch default playback device to Headphones\n\
             • Step 3: Process Tuning -> Priority: Above Normal",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Recipe: Auto-Heal Memory Leak",
            "• Trigger: Resource Threshold -> RAM > 3000 MB persisting for 30 sec\n\
             • Step 1: Process Tuning -> Trim Working Set RAM (instant memory compaction)\n\
             • Step 2: Notification -> Title: \"Optimized\", Message: \"RAM trimmed for {ProcessName}\"",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Recipe: Laptop Battery Saver",
            "• Trigger: System Event -> Power: On Battery (switched to battery power)\n\
             • Step 1: Power Plan -> Power Saver\n\
             • Step 2: Display -> Refresh Rate 60 Hz (reduced from 144/240 Hz to conserve battery)\n\
             • Step 3: Audio Control -> Set Volume 25%",
            t,
            false,
        );

        ui.add_space(2.0);
        doc_callout(
            ui,
            "Recipe: UI Automation (Hardware Macro)",
            "• Trigger: Global Hotkey -> Ctrl + Alt + M (or mouse button e.g. M4 in Hold mode)\n\
             • Step 1: Hardware Macro configured in Macro Studio:\n\
               — Move cursor to coordinates (1400, 450) + LMB click\n\
               — Type text \"Status: Approved\" with organic delay\n\
               — Physical key press Enter\n\
               — Mouse click Side Button 1 (Back / M4) to navigate back",
            t,
            false,
        );
    }
}
