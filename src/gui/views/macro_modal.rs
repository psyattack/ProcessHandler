use eframe::emath::vec2;
use egui::{
    pos2, Color32, CornerRadius, Id, Rect, RichText, Sense, Stroke, Ui,
};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Cell, Kind};
use crate::gui::macro_recorder::{egui_key_to_string, run_macro_test_safe, MacroRecorder};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};
use crate::models::{Action, MacroEvent, MouseButton, MouseCoordMode};

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, MESSAGEBOX_STYLE};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_RETURN, VK_SPACE};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::Debug::MessageBeep;

#[derive(Clone, Debug, PartialEq)]
enum TimelineItem {
    Single(usize),
    MoveGroup {
        start_idx: usize,
        count: usize,
    },
    FullKeyPress {
        start_idx: usize,
        count: usize,
    },
    FullMouseClick {
        start_idx: usize,
        count: usize,
    },
}

fn partition_timeline_items(events: &[MacroEvent], group_presses: bool) -> Vec<TimelineItem> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < events.len() {
        if matches!(events[i], MacroEvent::MouseMove { .. }) {
            let start = i;
            let mut count = 1;
            while i + 1 < events.len() && matches!(events[i + 1], MacroEvent::MouseMove { .. }) {
                count += 1;
                i += 1;
            }
            if count >= 2 {
                items.push(TimelineItem::MoveGroup { start_idx: start, count });
            } else {
                items.push(TimelineItem::Single(start));
            }
            i += 1;
        } else if group_presses {
            // Check for KeyDown + [Delay] + KeyUp with matching key
            if let MacroEvent::KeyDown { key: ref k_down } = events[i] {
                if i + 2 < events.len() {
                    if let (MacroEvent::Delay { .. }, MacroEvent::KeyUp { key: ref k_up }) = (&events[i + 1], &events[i + 2]) {
                        if k_down == k_up {
                            items.push(TimelineItem::FullKeyPress { start_idx: i, count: 3 });
                            i += 3;
                            continue;
                        }
                    }
                }
                if i + 1 < events.len() {
                    if let MacroEvent::KeyUp { key: ref k_up } = &events[i + 1] {
                        if k_down == k_up {
                            items.push(TimelineItem::FullKeyPress { start_idx: i, count: 2 });
                            i += 2;
                            continue;
                        }
                    }
                }
                items.push(TimelineItem::Single(i));
                i += 1;
            } else if let MacroEvent::MouseDown { button: ref b_down } = events[i] {
                // Check for MouseDown + [Delay] + MouseUp with matching button
                if i + 2 < events.len() {
                    if let (MacroEvent::Delay { .. }, MacroEvent::MouseUp { button: ref b_up }) = (&events[i + 1], &events[i + 2]) {
                        if b_down == b_up {
                            items.push(TimelineItem::FullMouseClick { start_idx: i, count: 3 });
                            i += 3;
                            continue;
                        }
                    }
                }
                if i + 1 < events.len() {
                    if let MacroEvent::MouseUp { button: ref b_up } = &events[i + 1] {
                        if b_down == b_up {
                            items.push(TimelineItem::FullMouseClick { start_idx: i, count: 2 });
                            i += 2;
                            continue;
                        }
                    }
                }
                items.push(TimelineItem::Single(i));
                i += 1;
            } else {
                items.push(TimelineItem::Single(i));
                i += 1;
            }
        } else {
            items.push(TimelineItem::Single(i));
            i += 1;
        }
    }
    items
}

pub fn render_macro_modal(ctx: &egui::Context, state: &mut GuiState) {
    let is_open = state.macro_editor.is_some();

    let recorder = MacroRecorder::global();
    let is_recording = recorder.is_active();
    if is_open {
        crate::gui::macro_recorder::set_studio_active(true);
        if is_recording || crate::gui::macro_recorder::is_macro_test_running() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    if is_recording {
        // Direct capture of keyboard events in GUI event loop (ensures immediate capture if modal has focus)
        if recorder.capture_keyboard.load(std::sync::atomic::Ordering::SeqCst) {
            let key_events: Vec<(String, bool)> = ctx.input(|i| {
                let mut list = Vec::new();
                for ev in &i.raw.events {
                    if let egui::Event::Key { key, pressed, repeat, .. } = ev {
                        let k_str = egui_key_to_string(*key);
                        if !repeat && !k_str.is_empty() && *key != egui::Key::F9 && *key != egui::Key::Escape && *key != egui::Key::F10 {
                            list.push((k_str.to_string(), *pressed));
                        }
                    }
                }
                list
            });

            for (k, pressed) in key_events {
                recorder.record_key(&k, pressed);
            }
        }
    }

    // Hotkey F9 toggle on platforms without background global listener
    #[cfg(not(windows))]
    if is_open && ctx.input(|i| i.key_pressed(egui::Key::F9)) {
        if is_recording {
            recorder.stop_recording();
            let more = recorder.drain_events();
            if let Some(edit_info) = state.macro_editor.clone().or_else(|| state.last_macro_editor.clone()) {
                if let Some(rule) = state.editing_rule.as_mut() {
                    let action_list = if edit_info.is_exit_pipeline { &mut rule.on_exit_actions } else { &mut rule.actions };
                    if let Some(Action::Macro { events, .. }) = action_list.get_mut(edit_info.step_idx) {
                        events.extend(more);
                    }
                }
            }
        } else {
            recorder.start();
        }
    }

    // Stop running test on Escape key
    if is_open && crate::gui::macro_recorder::is_macro_test_running() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        crate::gui::macro_recorder::stop_macro_test();
    }

    // Target action resolution
    let loc = state.loc();
    let editor_state = state.macro_editor.clone().or_else(|| state.last_macro_editor.clone());
    let Some(edit_info) = editor_state else { return; };

    let editing_rule = state.editing_rule.as_mut();
    let Some(rule) = editing_rule else { return; };

    let action_list = if edit_info.is_exit_pipeline {
        &mut rule.on_exit_actions
    } else {
        &mut rule.actions
    };

    let Some(action) = action_list.get_mut(edit_info.step_idx) else { return; };
    action.migrate_to_unified();

    if !matches!(action, Action::Macro { .. }) {
        *action = Action::Macro {
            events: Vec::new(),
            repeat: 1,
            speed: 1.0,
            infinite: false,
        };
    }

    let Action::Macro { events, repeat, speed, infinite } = action else { return; };

    // Real-time live draining: any newly captured events stream directly into the list
    let new_events = recorder.drain_events();
    if !new_events.is_empty() {
        events.extend(new_events);
    }
    if let Some(bg_events) = crate::gui::macro_recorder::take_studio_drained_events() {
        events.extend(bg_events);
    }

    // Sync with studio payload for background hotkeys
    crate::gui::macro_recorder::update_studio_payload(events.clone(), *speed);

    let t = Tokens::get(ctx);
    let mut group_full_presses = state.group_full_presses;

    let modal_res = Modal::new("macro-modal", is_open)
        .width_fraction(0.88)
        .height_fraction(0.84)
        .min_size(vec2(920.0, 560.0))
        .max_size(vec2(1180.0, 800.0))
        .header_height(46.0)
        .toolbar_height(48.0)
        .footer_height(46.0)
        .header_sunken(true)
        .footer_sunken(true)
        .close_on_escape(!is_recording && !crate::gui::macro_recorder::is_macro_test_running())
        .close_on_scrim(!is_recording)
        .accent_bar(t.accent_ink)
        .show(ctx, &t, |frame| {
            // --- HEADER ---
            frame.header(|ui, close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(egui_phosphor::regular::GAME_CONTROLLER)
                            .font(egui::FontId::new(18.0, theme::phosphor()))
                            .color(t.accent_ink),
                    );
                    ui.add_space(8.0);
                    widgets::eyebrow(
                        ui,
                        "MACRO STUDIO",
                        loc.macro_studio_title,
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(14.0);
                        if widgets::glyph_button(ui, widgets::Glyph::Close, loc.macro_studio_close_tip).clicked() {
                            *close_req = true;
                        }

                        if widgets::glyph_button(ui, widgets::Glyph::Minimize, loc.macro_studio_minimize_tip).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }

                        // Test run button (F10) - toggle start / stop on safe background runner
                        let is_testing = crate::gui::macro_recorder::is_macro_test_running();
                        let test_lbl = if is_testing {
                            loc.macro_studio_btn_stop_test
                        } else {
                            loc.macro_studio_btn_test_run
                        };
                        let test_kind = if is_testing { Kind::Danger } else { Kind::Secondary };
                        let btn_w = widgets::button_width(ui, test_lbl, 0.0);
                        let test_clicked = widgets::button(ui, test_lbl, test_kind, btn_w)
                            .on_hover_text(if is_testing {
                                loc.macro_studio_btn_stop_test_tip
                            } else {
                                loc.macro_studio_btn_test_run_tip
                            })
                            .clicked();

                        #[cfg(not(windows))]
                        let f10_hit = ctx.input(|i| i.key_pressed(egui::Key::F10));
                        #[cfg(windows)]
                        let f10_hit = false;

                        if test_clicked || f10_hit {
                            if is_testing {
                                crate::gui::macro_recorder::stop_macro_test();
                            } else {
                                run_macro_test_safe(events.clone(), *speed);
                            }
                        }
                    });
                });
            });

            // --- TOOLBAR: CLEAN, REORGANIZED WITH DISTINCT FUNCTIONAL ZONES ---
            frame.toolbar(|ui, _close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(14.0);

                    // ZONE 1: RECORD BUTTON & CAPTURE TOGGLE CHIPS
                    if is_recording {
                        let rec_label = loc.macro_studio_btn_stop_record;
                        let btn_w = widgets::button_width(ui, rec_label, 0.0);
                        if widgets::button(ui, rec_label, Kind::Danger, btn_w).clicked() {
                            recorder.stop_recording();
                            let more = recorder.drain_events();
                            events.extend(more);
                            // If user stopped via GUI button with mouse, cleanly strip the trailing stop click
                            if let Some(MacroEvent::MouseUp { button: MouseButton::Left }) = events.last() {
                                events.pop();
                                if let Some(MacroEvent::MouseDown { button: MouseButton::Left }) = events.last() {
                                    events.pop();
                                }
                                if let Some(MacroEvent::Delay { .. }) = events.last() {
                                    events.pop();
                                }
                            }
                        }
                    } else {
                        let rec_label = loc.macro_studio_btn_record;
                        let rec_tip = loc.macro_studio_btn_record_tip;
                        let btn_w = widgets::button_width(ui, rec_label, 0.0);
                        if widgets::button(ui, rec_label, Kind::Primary, btn_w)
                            .on_hover_text(rec_tip)
                            .clicked()
                        {
                            recorder.start();
                        }
                    }

                    ui.add_space(8.0);

                    // Filter toggle chips (compact, stylish DAW-like pills)
                    render_filter_chip(ui, loc.macro_studio_chip_keys, &recorder.capture_keyboard, loc.macro_studio_chip_keys_tip, &t);
                    ui.add_space(2.0);
                    render_filter_chip(ui, loc.macro_studio_chip_clicks, &recorder.capture_mouse_clicks, loc.macro_studio_chip_clicks_tip, &t);
                    ui.add_space(2.0);
                    render_filter_chip(ui, loc.macro_studio_chip_moves, &recorder.capture_mouse_moves, loc.macro_studio_chip_moves_tip, &t);
                    ui.add_space(2.0);
                    render_filter_chip(ui, loc.macro_studio_chip_wheel, &recorder.capture_mouse_wheel, loc.macro_studio_chip_wheel_tip, &t);
                    ui.add_space(2.0);
                    render_filter_chip(ui, loc.macro_studio_chip_delays, &recorder.capture_delays, loc.macro_studio_chip_delays_tip, &t);

                    ui.add_space(10.0);
                    widgets::rule(ui, 20.0);
                    ui.add_space(10.0);

                    // ZONE 2: ADD STEP ACTIONS
                    if widgets::compact_button(ui, loc.macro_studio_btn_add_key, Kind::Secondary, 24.0)
                        .on_hover_text(loc.macro_studio_btn_add_key_tip)
                        .clicked()
                    {
                        events.push(MacroEvent::KeyDown { key: "Enter".to_string() });
                        events.push(MacroEvent::Delay { ms: 40, organic: false, jitter_percent: 30 });
                        events.push(MacroEvent::KeyUp { key: "Enter".to_string() });
                    }

                    if widgets::compact_button(ui, loc.macro_studio_btn_add_mouse, Kind::Secondary, 24.0)
                        .on_hover_text(loc.macro_studio_btn_add_mouse_tip)
                        .clicked()
                    {
                        events.push(MacroEvent::MouseDown { button: MouseButton::Left });
                        events.push(MacroEvent::Delay { ms: 40, organic: false, jitter_percent: 30 });
                        events.push(MacroEvent::MouseUp { button: MouseButton::Left });
                    }

                    if widgets::compact_button(ui, loc.macro_studio_btn_add_move, Kind::Secondary, 24.0)
                        .on_hover_text(loc.macro_studio_btn_add_move_tip)
                        .clicked()
                    {
                        events.push(MacroEvent::MouseMove {
                            x: 960,
                            y: 540,
                            coord_mode: MouseCoordMode::Absolute,
                            smooth: true,
                            duration_ms: Some(250),
                            organic: false,
                        });
                    }

                    if widgets::compact_button(ui, loc.macro_studio_btn_add_delay, Kind::Secondary, 24.0)
                        .on_hover_text(loc.macro_studio_btn_add_delay_tip)
                        .clicked()
                    {
                        events.push(MacroEvent::Delay { ms: 100, organic: false, jitter_percent: 30 });
                    }

                    if widgets::compact_button(ui, loc.macro_studio_btn_add_text, Kind::Secondary, 24.0)
                        .on_hover_text(loc.macro_studio_btn_add_text_tip)
                        .clicked()
                    {
                        events.push(MacroEvent::Text { text: "Hello".to_string(), delay_ms: 20, organic: false });
                    }

                    if widgets::compact_button(ui, loc.macro_studio_btn_add_wheel, Kind::Secondary, 24.0)
                        .on_hover_text(loc.macro_studio_btn_add_wheel_tip)
                        .clicked()
                    {
                        events.push(MacroEvent::MouseWheel { delta: 120 });
                    }

                    // ZONE 3: UTILITIES (RIGHT-ALIGNED)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(14.0);

                        if !events.is_empty() {
                            let clr_lbl = loc.macro_studio_btn_clear_all;
                            let clr_tip = loc.macro_studio_btn_clear_all_tip;
                            let btn_w = widgets::button_width(ui, clr_lbl, 0.0);
                            if widgets::button(ui, clr_lbl, Kind::Secondary, btn_w).on_hover_text(clr_tip).clicked() {
                                events.clear();
                            }
                        }
                    });
                });
            });

            // --- BODY: UNIFIED TIMELINE TABLE WITH SYMMETRIC MARGINS & ROCK-SOLID DRAG & DROP ---
            frame.body(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("macro_timeline_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(6.0);
                        ui.spacing_mut().item_spacing.y = 4.0;

                        if events.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(90.0);
                                ui.label(
                                    RichText::new(egui_phosphor::regular::MOUSE_SIMPLE)
                                        .font(egui::FontId::new(42.0, theme::phosphor()))
                                        .color(t.faint),
                                );
                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new(loc.macro_studio_empty_text)
                                    .font(egui::FontId::new(13.0, theme::medium()))
                                    .color(t.muted),
                                );
                            });
                            return;
                        }

                        let items = partition_timeline_items(events, state.group_full_presses);

                        // Robust Drag & Drop tracking (using continuous vertical Y-centers pattern)
                        let dnd_id = ui.id().with("macro_timeline_dnd_src");
                        let active_src: Option<usize> = ui.data(|d| d.get_temp(dnd_id));
                        let pointer_pos = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()));
                        let pointer_released = ui.input(|i| i.pointer.any_released());

                        // Cancel drag on Esc key
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            ui.data_mut(|d| d.remove_temp::<usize>(dnd_id));
                        }

                        if active_src.is_some() {
                            ui.ctx().request_repaint();

                            // 1. Mouse wheel scrolling while dragging
                            let wheel_delta_y = ui.input(|i| i.smooth_scroll_delta.y);
                            if wheel_delta_y != 0.0 {
                                ui.scroll_with_delta(vec2(0.0, wheel_delta_y * 1.5));
                            }

                            // 2. Edge auto-scrolling when pointer is near top or bottom of scroll area
                            if let Some(pointer) = pointer_pos {
                                let viewport = ui.clip_rect();
                                let edge_zone = 70.0;
                                let view_top = viewport.top();
                                let view_bottom = viewport.bottom();

                                // Near top edge: auto-scroll up
                                if pointer.y < view_top + edge_zone {
                                    let factor = ((view_top + edge_zone - pointer.y) / edge_zone).clamp(0.1, 3.0);
                                    let scroll_speed = 8.0 + 26.0 * factor;
                                    ui.scroll_with_delta(vec2(0.0, scroll_speed));
                                }
                                // Near bottom edge: auto-scroll down
                                else if pointer.y > view_bottom - edge_zone {
                                    let factor = ((pointer.y - (view_bottom - edge_zone)) / edge_zone).clamp(0.1, 3.0);
                                    let scroll_speed = 8.0 + 26.0 * factor;
                                    ui.scroll_with_delta(vec2(0.0, -scroll_speed));
                                }
                            }
                        }

                        let expand_id = ui.id().with("macro_expanded_move_groups");
                        let mut expanded_groups: std::collections::HashSet<usize> = ui.data(|d| d.get_temp(expand_id).unwrap_or_default());

                        let mut drag_started_idx = None;
                        let mut to_remove: Option<TimelineItem> = None;
                        let mut to_duplicate: Option<TimelineItem> = None;
                        let mut row_centers = Vec::with_capacity(items.len());
                        let mut row_rects = Vec::with_capacity(items.len());

                        // Enclose rows in a padded frame so borders are never clipped against the modal edges
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(16, 6))
                            .show(ui, |ui| {
                                for (item_idx, item) in items.iter().enumerate() {
                                    let is_being_dragged = active_src == Some(item_idx);

                                    // Allocate row with symmetric width matching available frame width (compact 36px height)
                                    let row_w = ui.available_width();
                                    let (row_rect, row_resp) = ui.allocate_exact_size(vec2(row_w, 36.0), Sense::click_and_drag());
                                    row_centers.push(row_rect.center().y);
                                    row_rects.push(row_rect);

                                    // Drag started if row or grip was dragged
                                    if row_resp.drag_started() {
                                        drag_started_idx = Some(item_idx);
                                    }

                                    // Card background & border stroke (INSIDE so it never bleeds or gets clipped)
                                    let bg_color = if is_being_dragged {
                                        t.accent_ink.gamma_multiply(0.2)
                                    } else {
                                        match item {
                                            TimelineItem::Single(idx) if matches!(events[*idx], MacroEvent::Delay { .. }) => t.bg.gamma_multiply(0.4),
                                            _ => t.sunken,
                                        }
                                    };

                                    let border_color = if is_being_dragged {
                                        t.accent_ink
                                    } else {
                                        t.hairline().color
                                    };

                                    ui.painter().rect_filled(row_rect, CornerRadius::same(2), bg_color);
                                    ui.painter().rect_stroke(row_rect, CornerRadius::same(2), Stroke::new(1.0, border_color), egui::StrokeKind::Inside);

                                    ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                                        ui.spacing_mut().interact_size.y = 19.0;
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        ui.horizontal_centered(|ui| {
                                             ui.add_space(8.0);

                                            // Drag handle on far left
                                            let grip = widgets::drag_handle(ui, loc.macro_studio_drag_handle_tip);
                                            if grip.drag_started() {
                                                drag_started_idx = Some(item_idx);
                                            }

                                            match item {
                                                TimelineItem::Single(ev_idx) => {
                                                    // Step index
                                                    widgets::hud(ui, &format!("{:02}", ev_idx + 1), t.faint);
                                                    ui.add_space(4.0);

                                                    // Render clear, focused inline row by type
                                                    render_timeline_row_inline(ui, &mut events[*ev_idx], *ev_idx, loc, &t);

                                                    // Action buttons on the right (sleek ghost icon buttons, NO ugly square borders)
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.add_space(8.0);
                                                        if ghost_icon_button(ui, widgets::Glyph::Close, loc.macro_studio_tip_delete_step, true).clicked() {
                                                            to_remove = Some(item.clone());
                                                        }
                                                        if ghost_icon_button(ui, widgets::Glyph::Clone, loc.macro_studio_tip_duplicate_step, false).clicked() {
                                                            to_duplicate = Some(item.clone());
                                                        }
                                                    });
                                                }
                                                TimelineItem::MoveGroup { start_idx, count } => {
                                                    // Step index range
                                                    widgets::hud(ui, &format!("{:02}-{:02}", start_idx + 1, start_idx + count), t.faint);
                                                    ui.add_space(4.0);

                                                    // Expand / Collapse toggle button
                                                    let is_expanded = expanded_groups.contains(start_idx);
                                                    let arrow = if is_expanded { "▼" } else { "▶" };
                                                    let toggle_resp = ui.add(
                                                        egui::Button::new(RichText::new(arrow).font(egui::FontId::new(10.0, theme::mono_bold())).color(t.accent_ink))
                                                            .frame(false)
                                                            .min_size(vec2(16.0, 18.0)),
                                                    ).on_hover_text(if is_expanded {
                                                        loc.macro_studio_tip_collapse_moves
                                                    } else {
                                                        loc.macro_studio_tip_expand_moves
                                                    });
                                                    if toggle_resp.clicked() {
                                                        if is_expanded {
                                                            expanded_groups.remove(start_idx);
                                                        } else {
                                                            expanded_groups.insert(*start_idx);
                                                        }
                                                    }
                                                    ui.add_space(2.0);

                                                    // Stacked Badge
                                                    let moves_title = format!("{} ({})", loc.macro_studio_badge_moves, count);
                                                    render_chip(ui, &moves_title, t.accent_ink, t.bg);
                                                    ui.add_space(4.0);

                                                    // Trajectory summary
                                                    let p_start = match &events[*start_idx] { MacroEvent::MouseMove { x, y, .. } => (*x, *y), _ => (0, 0) };
                                                    let p_end = match &events[*start_idx + *count - 1] { MacroEvent::MouseMove { x, y, .. } => (*x, *y), _ => (0, 0) };
                                                    let summary = format!("A: ({}, {}) → B: ({}, {}) • {} {}", p_start.0, p_start.1, p_end.0, p_end.1, count, loc.macro_studio_pts_suffix);
                                                    widgets::hud(ui, &summary, t.ink_strong);

                                                    // Action buttons on the right
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.add_space(8.0);
                                                        if ghost_icon_button(ui, widgets::Glyph::Close, loc.macro_studio_tip_delete_group, true).clicked() {
                                                            to_remove = Some(item.clone());
                                                        }
                                                        if ghost_icon_button(ui, widgets::Glyph::Clone, loc.macro_studio_tip_duplicate_group, false).clicked() {
                                                            to_duplicate = Some(item.clone());
                                                        }
                                                    });
                                                }
                                                TimelineItem::FullKeyPress { start_idx, count } => {
                                                    // Step index range
                                                    widgets::hud(ui, &format!("{:02}-{:02}", start_idx + 1, start_idx + count), t.faint);
                                                    ui.add_space(4.0);

                                                    render_chip(ui, loc.macro_studio_badge_key_press, t.accent_ink, t.bg);
                                                    ui.add_space(4.0);

                                                    let mut key_val = match &events[*start_idx] {
                                                        MacroEvent::KeyDown { key } => key.clone(),
                                                        _ => String::new(),
                                                    };
                                                    widgets::hud(ui, loc.macro_studio_label_key, t.muted);
                                                    let edit_resp = ui.add(
                                                        egui::TextEdit::singleline(&mut key_val)
                                                            .desired_width(110.0)
                                                            .min_size(vec2(0.0, 20.0))
                                                            .font(egui::FontId::new(11.0, theme::mono())),
                                                    );
                                                    if edit_resp.changed() {
                                                        if let MacroEvent::KeyDown { key } = &mut events[*start_idx] {
                                                            *key = key_val.clone();
                                                        }
                                                        let up_idx = start_idx + count - 1;
                                                        if let MacroEvent::KeyUp { key } = &mut events[up_idx] {
                                                            *key = key_val;
                                                        }
                                                    }

                                                    if *count == 3 {
                                                        if let MacroEvent::Delay { ms, organic, .. } = &mut events[start_idx + 1] {
                                                            ui.add_space(4.0);
                                                            widgets::hud(ui, loc.macro_studio_label_delay, t.muted);
                                                            ui.add(egui::DragValue::new(ms).range(0..=60000).suffix(" ms").prefix("⏱ "));
                                                            ui.checkbox(organic, loc.macro_studio_label_organic);
                                                        }
                                                    }

                                                    // Action buttons on the right
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.add_space(8.0);
                                                        if ghost_icon_button(ui, widgets::Glyph::Close, loc.macro_studio_tip_delete_press, true).clicked() {
                                                            to_remove = Some(item.clone());
                                                        }
                                                        if ghost_icon_button(ui, widgets::Glyph::Clone, loc.macro_studio_tip_duplicate_press, false).clicked() {
                                                            to_duplicate = Some(item.clone());
                                                        }
                                                    });
                                                }
                                                TimelineItem::FullMouseClick { start_idx, count } => {
                                                    // Step index range
                                                    widgets::hud(ui, &format!("{:02}-{:02}", start_idx + 1, start_idx + count), t.faint);
                                                    ui.add_space(4.0);

                                                    render_chip(ui, loc.macro_studio_badge_click, t.accent_ink, t.bg);
                                                    ui.add_space(4.0);

                                                    let mut btn_val = match events[*start_idx] {
                                                        MacroEvent::MouseDown { button } => button,
                                                        _ => MouseButton::Left,
                                                    };
                                                    widgets::hud(ui, loc.macro_studio_label_button, t.muted);
                                                    let prev_btn = btn_val;
                                                    render_button_cells(ui, &mut btn_val, loc);
                                                    if btn_val != prev_btn {
                                                        if let MacroEvent::MouseDown { button } = &mut events[*start_idx] {
                                                            *button = btn_val;
                                                        }
                                                        let up_idx = start_idx + count - 1;
                                                        if let MacroEvent::MouseUp { button } = &mut events[up_idx] {
                                                            *button = btn_val;
                                                        }
                                                    }

                                                    if *count == 3 {
                                                        if let MacroEvent::Delay { ms, organic, .. } = &mut events[start_idx + 1] {
                                                            ui.add_space(4.0);
                                                            widgets::hud(ui, loc.macro_studio_label_delay, t.muted);
                                                            ui.add(egui::DragValue::new(ms).range(0..=60000).suffix(" ms").prefix("⏱ "));
                                                            ui.checkbox(organic, loc.macro_studio_label_organic);
                                                        }
                                                    }

                                                    // Action buttons on the right
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.add_space(8.0);
                                                        if ghost_icon_button(ui, widgets::Glyph::Close, loc.macro_studio_tip_delete_click, true).clicked() {
                                                            to_remove = Some(item.clone());
                                                        }
                                                        if ghost_icon_button(ui, widgets::Glyph::Clone, loc.macro_studio_tip_duplicate_click, false).clicked() {
                                                            to_duplicate = Some(item.clone());
                                                        }
                                                    });
                                                }
                                            }
                                        });
                                    });

                                    // If MoveGroup is expanded, unfold the list of individual movements right below
                                    if let TimelineItem::MoveGroup { start_idx, count } = item {
                                        if expanded_groups.contains(start_idx) {
                                            ui.add_space(2.0);
                                            egui::Frame::new()
                                                .fill(t.sunken)
                                                .corner_radius(CornerRadius::same(2))
                                                .stroke(Stroke::new(1.0, t.hairline().color))
                                                .inner_margin(egui::Margin { left: 32, right: 14, top: 4, bottom: 4 })
                                                .show(ui, |ui| {
                                                    ui.spacing_mut().interact_size.y = 19.0;
                                                    ui.spacing_mut().item_spacing.y = 2.0;
                                                    for sub_i in 0..*count {
                                                        let sub_ev_idx = start_idx + sub_i;
                                                        if let MacroEvent::MouseMove { x, y, coord_mode, .. } = &mut events[sub_ev_idx] {
                                                            ui.horizontal(|ui| {
                                                                widgets::hud(ui, &format!("#{:02}", sub_i + 1), t.faint);
                                                                ui.add_space(4.0);
                                                                let is_rel = *coord_mode == MouseCoordMode::Relative;
                                                                if is_rel {
                                                                    ui.add(egui::DragValue::new(x).prefix("ΔX: ").speed(2.0));
                                                                    ui.add(egui::DragValue::new(y).prefix("ΔY: ").speed(2.0));
                                                                } else {
                                                                    ui.add(egui::DragValue::new(x).prefix("X: "));
                                                                    ui.add(egui::DragValue::new(y).prefix("Y: "));
                                                                    let mut ox = Some(*x);
                                                                    let mut oy = Some(*y);
                                                                    let p_id = ui.id().with(sub_ev_idx).with("group_sub_picker");
                                                                    render_compact_coord_picker(ui, p_id, &mut ox, &mut oy, loc);
                                                                    if let (Some(nx), Some(ny)) = (ox, oy) {
                                                                        *x = nx;
                                                                        *y = ny;
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    }
                                                });
                                            ui.add_space(2.0);
                                        }
                                    }
                                }
                            });

                        ui.data_mut(|d| d.insert_temp(expand_id, expanded_groups));

                        // Calculate target drop index across continuous vertical space
                        let mut target_idx = None;
                        if let (Some(_src), Some(ptr)) = (active_src, pointer_pos) {
                            let n = row_centers.len();
                            if n > 0 {
                                let t_idx = if ptr.y < row_centers[0] {
                                    0
                                } else if ptr.y >= row_centers[n - 1] {
                                    n
                                } else {
                                    let mut found = n;
                                    for i in 0..(n - 1) {
                                        if ptr.y >= row_centers[i] && ptr.y < row_centers[i + 1] {
                                            found = i + 1;
                                            break;
                                        }
                                    }
                                    found
                                };
                                target_idx = Some(t_idx);

                                // Draw glowing insertion line with endpoints
                                if t_idx < n {
                                    let rect = row_rects[t_idx];
                                    let y = rect.top() - 2.0;
                                    ui.painter().hline(rect.x_range(), y, Stroke::new(2.5, t.accent_ink));
                                    ui.painter().circle_filled(pos2(rect.left(), y), 3.0, t.accent_ink);
                                    ui.painter().circle_filled(pos2(rect.right(), y), 3.0, t.accent_ink);
                                } else if let Some(last_rect) = row_rects.last() {
                                    let y = last_rect.bottom() + 2.0;
                                    ui.painter().hline(last_rect.x_range(), y, Stroke::new(2.5, t.accent_ink));
                                    ui.painter().circle_filled(pos2(last_rect.left(), y), 3.0, t.accent_ink);
                                    ui.painter().circle_filled(pos2(last_rect.right(), y), 3.0, t.accent_ink);
                                }
                            }
                        }

                        // Commit reordering on release
                        if pointer_released {
                            if let (Some(src_item_idx), Some(dst_item_idx)) = (active_src, target_idx) {
                                if src_item_idx != dst_item_idx && src_item_idx < items.len() && dst_item_idx <= items.len() {
                                    let (src_start, src_len) = match items[src_item_idx] {
                                        TimelineItem::Single(idx) => (idx, 1),
                                        TimelineItem::MoveGroup { start_idx, count }
                                        | TimelineItem::FullKeyPress { start_idx, count }
                                        | TimelineItem::FullMouseClick { start_idx, count } => (start_idx, count),
                                    };
                                    let mut dst_event_idx = if dst_item_idx < items.len() {
                                        match items[dst_item_idx] {
                                            TimelineItem::Single(idx) => idx,
                                            TimelineItem::MoveGroup { start_idx, .. }
                                            | TimelineItem::FullKeyPress { start_idx, .. }
                                            | TimelineItem::FullMouseClick { start_idx, .. } => start_idx,
                                        }
                                    } else {
                                        events.len()
                                    };

                                    if src_start + src_len <= events.len() {
                                        let moving_slice: Vec<_> = events.drain(src_start..src_start + src_len).collect();
                                        if dst_event_idx > src_start {
                                            dst_event_idx = dst_event_idx.saturating_sub(src_len);
                                        }
                                        let ins_pos = dst_event_idx.min(events.len());
                                        for (offset, ev) in moving_slice.into_iter().enumerate() {
                                            events.insert(ins_pos + offset, ev);
                                        }
                                    }
                                }
                            }
                            ui.data_mut(|d| d.remove_temp::<usize>(dnd_id));
                        } else if let Some(new_src) = drag_started_idx {
                            ui.data_mut(|d| d.insert_temp(dnd_id, new_src));
                        }

                        if let Some(item) = to_remove {
                            match item {
                                TimelineItem::Single(idx) => {
                                    if idx < events.len() {
                                        events.remove(idx);
                                    }
                                }
                                TimelineItem::MoveGroup { start_idx, count }
                                | TimelineItem::FullKeyPress { start_idx, count }
                                | TimelineItem::FullMouseClick { start_idx, count } => {
                                    if start_idx + count <= events.len() {
                                        events.drain(start_idx..start_idx + count);
                                    }
                                }
                            }
                        } else if let Some(item) = to_duplicate {
                            match item {
                                TimelineItem::Single(idx) => {
                                    if idx < events.len() {
                                        let clone_ev = events[idx].clone();
                                        events.insert(idx + 1, clone_ev);
                                    }
                                }
                                TimelineItem::MoveGroup { start_idx, count }
                                | TimelineItem::FullKeyPress { start_idx, count }
                                | TimelineItem::FullMouseClick { start_idx, count } => {
                                    if start_idx + count <= events.len() {
                                        let slice: Vec<_> = events[start_idx..start_idx + count].to_vec();
                                        for (offset, ev) in slice.into_iter().enumerate() {
                                            events.insert(start_idx + count + offset, ev);
                                        }
                                    }
                                }
                            }
                        }

                        ui.add_space(8.0);
                    });
            });

            // --- FOOTER ---
            frame.footer(|ui, close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);

                    // Repeat count (hidden when infinite mode is enabled)
                    if !*infinite {
                        widgets::hud(ui, loc.macro_studio_repeat_label, t.muted);
                        ui.add(egui::DragValue::new(repeat).range(1..=10000).suffix("x"));
                        ui.add_space(20.0);
                    }

                    widgets::hud(ui, loc.macro_studio_speed_label, t.muted);
                    ui.add(egui::DragValue::new(speed).range(0.1..=10.0).speed(0.1).suffix("x"));

                    ui.add_space(20.0);

                    let can_toggle_infinite = matches!(rule.trigger, Some(crate::models::Trigger::GlobalHotkey { .. }));
                    if !can_toggle_infinite {
                        *infinite = false;
                    }

                    // Checkboxes group
                    let inf_cb = egui::Checkbox::new(
                        infinite,
                        loc.macro_studio_checkbox_infinite,
                    );
                    let inf_tip = if !can_toggle_infinite {
                        loc.macro_studio_infinite_disabled_tip
                    } else {
                        loc.macro_studio_infinite_tip
                    };

                    ui.add_enabled(can_toggle_infinite, inf_cb).on_hover_text(inf_tip);

                    ui.add_space(16.0);

                    // Global organic delays toggle across entire macro (delays + text typing)
                    let has_organic_targets = events.iter().any(|e| matches!(
                        e,
                        MacroEvent::Delay { .. } | MacroEvent::Text { .. }
                    ));
                    let all_organic = has_organic_targets && events.iter().all(|e| match e {
                        MacroEvent::Delay { organic, .. } => *organic,
                        MacroEvent::Text { organic, .. } => *organic,
                        _ => true,
                    });
                    let mut org_toggle = all_organic;
                    let org_resp = ui.add_enabled(
                        has_organic_targets,
                        egui::Checkbox::new(&mut org_toggle, loc.macro_studio_checkbox_global_organic),
                    ).on_hover_text(if has_organic_targets {
                        loc.macro_studio_global_organic_active_tip
                    } else {
                        loc.macro_studio_global_organic_disabled_tip
                    });

                    if org_resp.changed() {
                        for ev in events.iter_mut() {
                            match ev {
                                MacroEvent::Delay { organic, .. } => *organic = org_toggle,
                                MacroEvent::Text { organic, .. } => *organic = org_toggle,
                                _ => {}
                            }
                        }
                    }

                    ui.add_space(16.0);
                    ui.checkbox(
                        &mut group_full_presses,
                        loc.macro_studio_checkbox_group_presses,
                    ).on_hover_text(loc.macro_studio_group_presses_tip);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        let done_lbl = loc.macro_studio_btn_save_close;
                        let btn_w = widgets::button_width(ui, done_lbl, 0.0);
                        if widgets::button(ui, done_lbl, Kind::Primary, btn_w).clicked() {
                            *close_req = true;
                        }
                    });
                });
            });
        });

    state.group_full_presses = group_full_presses;

    if let Some(res) = modal_res {
        if res.close_requested {
            if recorder.is_active() {
                recorder.stop_recording();
                let more = recorder.drain_events();
                let editor_state = state.macro_editor.clone().or_else(|| state.last_macro_editor.clone());
                if let Some(edit_info) = editor_state {
                    if let Some(rule) = state.editing_rule.as_mut() {
                        let action_list = if edit_info.is_exit_pipeline {
                            &mut rule.on_exit_actions
                        } else {
                            &mut rule.actions
                        };
                        if let Some(Action::Macro { events, .. }) = action_list.get_mut(edit_info.step_idx) {
                            events.extend(more);
                        }
                    }
                }
            }
            crate::gui::macro_recorder::stop_macro_test();
            state.close_macro_editor();
        }
    } else if !is_open {
        crate::gui::macro_recorder::set_studio_active(false);
        state.last_macro_editor = None;
    }
}

fn render_filter_chip(
    ui: &mut Ui,
    label: &str,
    state: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    tip: &str,
    t: &Tokens,
) {
    let mut is_active = state.load(std::sync::atomic::Ordering::SeqCst);
    let chip_h = 24.0;
    let text_w = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::new(10.5, theme::semibold()), Color32::WHITE).size().x;
    let (rect, resp) = ui.allocate_exact_size(vec2(text_w + 14.0, chip_h), Sense::click());

    if resp.clicked() {
        is_active = !is_active;
        state.store(is_active, std::sync::atomic::Ordering::SeqCst);
    }

    let p = ui.painter();
    let (bg, border, text_color) = if is_active {
        (t.accent_ink.gamma_multiply(0.18), t.accent_ink, t.accent_ink)
    } else {
        (t.sunken, t.hairline().color, t.muted)
    };

    p.rect_filled(rect, CornerRadius::same(2), bg);
    p.rect_stroke(rect, CornerRadius::same(2), Stroke::new(1.0, border), egui::StrokeKind::Inside);
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(10.5, theme::semibold()),
        text_color,
    );

    resp.on_hover_text(tip);
}

fn render_compact_cells<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, Cell<'_>)]) -> bool {
    let t = Tokens::get(ui.ctx());
    let mut changed = false;
    let mut rects = Vec::with_capacity(options.len());
    let h = 20.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (opt, cell) in options {
            let on = *value == *opt;
            let galley = ui.painter().layout_no_wrap(cell.label.to_string(), egui::FontId::new(10.0, theme::mono_bold()), t.ink);
            let w = galley.size().x + 14.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            let hovered = resp.hovered();

            let (fill, ink) = match (on, hovered) {
                (true, _) => (t.accent_ink, t.bg),
                (false, true) => (t.sunken, t.ink_strong),
                (false, false) => (Color32::TRANSPARENT, t.muted),
            };

            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
            let text_rect = rect.shrink2(vec2(6.0, 0.0));
            widgets::paint_left(ui, text_rect, galley, ink);
            let resp = if !cell.tip.trim().is_empty() {
                resp.on_hover_text(cell.tip)
            } else {
                resp
            };
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
                *value = *opt;
                changed = true;
            }
            rects.push(rect);
        }
    });

    if let (Some(first), Some(last)) = (rects.first(), rects.last()) {
        let union = first.union(*last);
        ui.painter().rect_stroke(union, CornerRadius::same(2), t.hairline(), egui::StrokeKind::Inside);
    }

    changed
}

fn render_timeline_row_inline(
    ui: &mut Ui,
    event: &mut MacroEvent,
    idx: usize,
    loc: &crate::i18n::Strings,
    t: &Tokens,
) {
    // Seamlessly normalize legacy TAP / Click events to clean physical down events
    if let MacroEvent::KeyPress { key } = event {
        *event = MacroEvent::KeyDown { key: key.clone() };
    }
    if let MacroEvent::MouseClick { button, .. } = event {
        *event = MacroEvent::MouseDown { button: *button };
    }

    let picker_id = ui.id().with(idx).with("row_coord_picker");
    ui.spacing_mut().interact_size.y = 19.0;
    ui.spacing_mut().item_spacing.y = 0.0;

    let mut toggle_to = None;

    match event {
        // --- KEY DOWN ---
        MacroEvent::KeyDown { key } => {
            let tip = loc.macro_studio_tip_toggle_to_release;
            let chip_txt = loc.macro_studio_chip_key_down;
            if render_clickable_chip(ui, chip_txt, t.accent_ink, t.bg, tip) {
                toggle_to = Some(MacroEvent::KeyUp { key: key.clone() });
            }
            widgets::hud(ui, loc.macro_studio_label_down, t.muted);
            ui.add(egui::TextEdit::singleline(key).desired_width(110.0).min_size(vec2(0.0, 20.0)).font(egui::FontId::new(11.0, theme::mono())));
        }

        // --- KEY UP ---
        MacroEvent::KeyUp { key } => {
            let tip = loc.macro_studio_tip_toggle_to_press;
            let chip_txt = loc.macro_studio_chip_key_up;
            if render_clickable_chip(ui, chip_txt, t.muted, t.bg, tip) {
                toggle_to = Some(MacroEvent::KeyDown { key: key.clone() });
            }
            widgets::hud(ui, loc.macro_studio_label_up, t.muted);
            ui.add(egui::TextEdit::singleline(key).desired_width(110.0).min_size(vec2(0.0, 20.0)).font(egui::FontId::new(11.0, theme::mono())));
        }

        // --- MOUSE DOWN ---
        MacroEvent::MouseDown { button } => {
            let tip = loc.macro_studio_tip_toggle_to_mouse_release;
            let chip_txt = loc.macro_studio_chip_mouse_down;
            if render_clickable_chip(ui, chip_txt, t.accent_ink, t.bg, tip) {
                toggle_to = Some(MacroEvent::MouseUp { button: *button });
            }
            widgets::hud(ui, loc.macro_studio_label_down, t.muted);
            render_button_cells(ui, button, loc);
        }

        // --- MOUSE UP ---
        MacroEvent::MouseUp { button } => {
            let tip = loc.macro_studio_tip_toggle_to_mouse_press;
            let chip_txt = loc.macro_studio_chip_mouse_up;
            if render_clickable_chip(ui, chip_txt, t.muted, t.bg, tip) {
                toggle_to = Some(MacroEvent::MouseDown { button: *button });
            }
            widgets::hud(ui, loc.macro_studio_label_up, t.muted);
            render_button_cells(ui, button, loc);
        }

        // Fallbacks for any other click/press variants
        MacroEvent::KeyPress { key } => {
            toggle_to = Some(MacroEvent::KeyDown { key: key.clone() });
        }
        MacroEvent::MouseClick { button, .. } => {
            toggle_to = Some(MacroEvent::MouseDown { button: *button });
        }

        // --- MOUSE MOVE & GAME CAMERA TURN ---
        MacroEvent::MouseMove { x, y, coord_mode, smooth, duration_ms, .. } => {
            let is_rel = *coord_mode == MouseCoordMode::Relative;
            let chip_title = if is_rel {
                loc.macro_studio_chip_turn
            } else {
                loc.macro_studio_chip_move
            };
            let chip_tip = if is_rel {
                loc.macro_studio_tip_rel_turn
            } else {
                loc.macro_studio_tip_abs_move
            };

            if render_clickable_chip(ui, chip_title, t.accent_ink, t.bg, chip_tip) {
                *coord_mode = if is_rel {
                    MouseCoordMode::Absolute
                } else {
                    MouseCoordMode::Relative
                };
            }

            if is_rel {
                ui.add(egui::DragValue::new(x).prefix("ΔX: ").speed(2.0));
                ui.add(egui::DragValue::new(y).prefix("ΔY: ").speed(2.0));
            } else {
                ui.add(egui::DragValue::new(x).prefix("X: "));
                ui.add(egui::DragValue::new(y).prefix("Y: "));

                let mut opt_x = Some(*x);
                let mut opt_y = Some(*y);
                render_compact_coord_picker(ui, picker_id, &mut opt_x, &mut opt_y, loc);
                if let (Some(nx), Some(ny)) = (opt_x, opt_y) {
                    *x = nx;
                    *y = ny;
                }
            }

            ui.checkbox(smooth, loc.macro_studio_checkbox_smooth);

            if *smooth {
                let mut dur = duration_ms.unwrap_or(200);
                if ui.add(egui::DragValue::new(&mut dur).range(10..=5000).suffix(" ms").prefix("⏱ ")).changed() {
                    *duration_ms = Some(dur);
                }
            }
        }

        // --- PAUSE / DELAY (clean input without redundant preset buttons) ---
        MacroEvent::Delay { ms, organic, .. } => {
            render_chip(ui, loc.macro_studio_chip_delay, t.faint, t.bg);
            widgets::hud(ui, loc.macro_studio_label_delay, t.muted);
            ui.add(egui::DragValue::new(ms).range(0..=60000).suffix(" ms").prefix("⏱ "));
            ui.checkbox(organic, loc.macro_studio_label_organic);
        }

        // --- TEXT TYPING (with built-in char delay and organic jitter) ---
        MacroEvent::Text { text, delay_ms, organic } => {
            render_chip(ui, loc.macro_studio_chip_text, t.accent_ink, t.bg);
            widgets::hud(ui, loc.macro_studio_label_text, t.muted);
            ui.add(egui::TextEdit::singleline(text).desired_width(170.0).min_size(vec2(0.0, 20.0)).font(egui::FontId::new(11.0, theme::mono())));
            ui.add_space(4.0);
            widgets::hud(ui, loc.macro_studio_label_delay, t.muted);
            ui.add(egui::DragValue::new(delay_ms).range(0..=5000).suffix(" ms").prefix("⏱ "));
            ui.checkbox(organic, loc.macro_studio_label_organic);
        }

        // --- SCROLL WHEEL ---
        MacroEvent::MouseWheel { delta } => {
            render_chip(ui, loc.macro_studio_chip_wheel_badge, t.muted, t.bg);
            widgets::hud(ui, loc.macro_studio_label_wheel, t.muted);
            ui.add(egui::DragValue::new(delta).range(-5000..=5000));
        }
    }

    if let Some(new_ev) = toggle_to {
        *event = new_ev;
    }
}

/// Borderless ghost icon button for delete / clone in timeline rows.
/// Completely eliminates the clunky box outlines on every step.
fn ghost_icon_button(ui: &mut Ui, glyph: widgets::Glyph, tip: &str, is_danger: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let size = 22.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let hovered = resp.hovered();

    let p = ui.painter();
    if hovered {
        let bg_hover = if is_danger {
            t.bad.gamma_multiply(0.18)
        } else {
            t.accent_ink.gamma_multiply(0.12)
        };
        p.rect_filled(rect, CornerRadius::same(2), bg_hover);
    }

    let glyph_color = if hovered {
        if is_danger { t.bad } else { t.accent_ink }
    } else {
        t.faint
    };

    let m = rect.center();
    match glyph {
        widgets::Glyph::Close => {
            let s = 3.5;
            p.line_segment([m + vec2(-s, -s), m + vec2(s, s)], Stroke::new(1.3, glyph_color));
            p.line_segment([m + vec2(-s, s), m + vec2(s, -s)], Stroke::new(1.3, glyph_color));
        }
        widgets::Glyph::Clone => {
            let back = Rect::from_center_size(m + vec2(-1.8, -1.8), vec2(6.5, 6.5));
            let front = Rect::from_center_size(m + vec2(1.8, 1.8), vec2(6.5, 6.5));
            p.rect_stroke(back, CornerRadius::ZERO, Stroke::new(1.0, glyph_color.gamma_multiply(0.6)), egui::StrokeKind::Inside);
            p.rect_filled(front, CornerRadius::ZERO, if hovered { t.bg } else { t.sunken });
            p.rect_stroke(front, CornerRadius::ZERO, Stroke::new(1.0, glyph_color), egui::StrokeKind::Inside);
        }
        _ => {}
    }

    resp.on_hover_text(tip)
}

fn render_chip(ui: &mut Ui, label: &str, fg: Color32, bg: Color32) {
    let chip_h = 19.0;
    let text_w = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::new(9.5, theme::mono_bold()), fg).size().x;
    let (rect, _) = ui.allocate_exact_size(vec2(text_w + 12.0, chip_h), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(2), bg);
    ui.painter().rect_stroke(rect, CornerRadius::same(2), Stroke::new(1.0, fg.gamma_multiply(0.7)), egui::StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(9.5, theme::mono_bold()),
        fg,
    );
}

fn render_clickable_chip(ui: &mut Ui, label: &str, fg: Color32, bg: Color32, tooltip: &str) -> bool {
    let chip_h = 19.0;
    let text_w = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::new(9.5, theme::mono_bold()), fg).size().x;
    let (rect, resp) = ui.allocate_exact_size(vec2(text_w + 12.0, chip_h), Sense::click());
    let hovered = resp.hovered();

    let border_color = if hovered { fg } else { fg.gamma_multiply(0.7) };
    let bg_color = if hovered { bg.blend(fg.gamma_multiply(0.20)) } else { bg };

    ui.painter().rect_filled(rect, CornerRadius::same(2), bg_color);
    ui.painter().rect_stroke(rect, CornerRadius::same(2), Stroke::new(1.0, border_color), egui::StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(9.5, theme::mono_bold()),
        fg,
    );
    if !tooltip.is_empty() {
        resp.clone().on_hover_text(tooltip);
    }
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn render_button_cells(ui: &mut Ui, button: &mut MouseButton, loc: &crate::i18n::Strings) {
    let btn_options = [
        (MouseButton::Left, Cell::new(loc.macro_studio_btn_lmb, loc.macro_studio_btn_lmb_tip)),
        (MouseButton::Right, Cell::new(loc.macro_studio_btn_rmb, loc.macro_studio_btn_rmb_tip)),
        (MouseButton::Middle, Cell::new(loc.macro_studio_btn_mmb, loc.macro_studio_btn_mmb_tip)),
        (MouseButton::XButton1, Cell::new(loc.macro_studio_btn_m4, loc.macro_studio_btn_m4_tip)),
        (MouseButton::XButton2, Cell::new(loc.macro_studio_btn_m5, loc.macro_studio_btn_m5_tip)),
    ];
    render_compact_cells(ui, button, &btn_options);
}

fn render_compact_coord_picker(
    ui: &mut Ui,
    picker_id: Id,
    x: &mut Option<i32>,
    y: &mut Option<i32>,
    loc: &crate::i18n::Strings,
) {
    #[cfg(windows)]
    {
        let is_picking: bool = ui.data_mut(|d| d.get_temp(picker_id).unwrap_or(false));
        if is_picking {
            ui.ctx().request_repaint();
            let mut pt = windows::Win32::Foundation::POINT::default();
            let _ = unsafe { GetCursorPos(&mut pt) };

            let hit_key = unsafe {
                GetAsyncKeyState(VK_SPACE.0 as i32) < 0 || GetAsyncKeyState(VK_RETURN.0 as i32) < 0
            };
            let cancel = unsafe {
                GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0
            };

            if cancel {
                ui.data_mut(|d| d.insert_temp(picker_id, false));
            } else if hit_key {
                *x = Some(pt.x);
                *y = Some(pt.y);
                ui.data_mut(|d| d.insert_temp(picker_id, false));
                unsafe {
                    let _ = MessageBeep(MESSAGEBOX_STYLE(0));
                }
            } else {
                let label = loc.macro_studio_space_capture_prefix.replacen("{}", &pt.x.to_string(), 1).replacen("{}", &pt.y.to_string(), 1);
                if widgets::compact_button(ui, &label, Kind::Primary, 18.0)
                    .on_hover_text(loc.macro_studio_space_capture_tip)
                    .clicked()
                {
                    *x = Some(pt.x);
                    *y = Some(pt.y);
                    ui.data_mut(|d| d.insert_temp(picker_id, false));
                }
            }
        } else {
            if widgets::compact_button(ui, loc.macro_studio_pick_btn, Kind::Secondary, 18.0)
                .on_hover_text(loc.macro_studio_pick_btn_tip)
                .clicked()
            {
                ui.data_mut(|d| d.insert_temp(picker_id, true));
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (ui, picker_id, x, y, loc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_timeline_items_grouping() {
        let events = vec![
            MacroEvent::KeyDown { key: "Enter".to_string() },
            MacroEvent::Delay { ms: 40, organic: false, jitter_percent: 30 },
            MacroEvent::KeyUp { key: "Enter".to_string() },
            MacroEvent::MouseDown { button: MouseButton::Left },
            MacroEvent::Delay { ms: 40, organic: false, jitter_percent: 30 },
            MacroEvent::MouseUp { button: MouseButton::Left },
            MacroEvent::MouseMove { x: 10, y: 10, coord_mode: MouseCoordMode::Absolute, smooth: false, duration_ms: None, organic: false },
            MacroEvent::MouseMove { x: 20, y: 20, coord_mode: MouseCoordMode::Absolute, smooth: false, duration_ms: None, organic: false },
        ];

        // Without group_presses: keys/mouse stay Single, moves group into MoveGroup
        let unpartitioned = partition_timeline_items(&events, false);
        assert_eq!(unpartitioned.len(), 7);
        assert_eq!(unpartitioned[0], TimelineItem::Single(0));
        assert_eq!(unpartitioned[1], TimelineItem::Single(1));
        assert_eq!(unpartitioned[2], TimelineItem::Single(2));
        assert_eq!(unpartitioned[3], TimelineItem::Single(3));
        assert_eq!(unpartitioned[4], TimelineItem::Single(4));
        assert_eq!(unpartitioned[5], TimelineItem::Single(5));
        assert_eq!(unpartitioned[6], TimelineItem::MoveGroup { start_idx: 6, count: 2 });

        // With group_presses: KeyDown+Delay+KeyUp -> FullKeyPress, MouseDown+Delay+MouseUp -> FullMouseClick
        let partitioned = partition_timeline_items(&events, true);
        assert_eq!(partitioned.len(), 3);
        assert_eq!(partitioned[0], TimelineItem::FullKeyPress { start_idx: 0, count: 3 });
        assert_eq!(partitioned[1], TimelineItem::FullMouseClick { start_idx: 3, count: 3 });
        assert_eq!(partitioned[2], TimelineItem::MoveGroup { start_idx: 6, count: 2 });
    }

    #[test]
    fn test_strip_trailing_gui_stop_click() {
        let mut events = vec![
            MacroEvent::KeyDown { key: "A".to_string() },
            MacroEvent::KeyUp { key: "A".to_string() },
            MacroEvent::Delay { ms: 500, organic: false, jitter_percent: 30 },
            MacroEvent::MouseDown { button: MouseButton::Left },
            MacroEvent::MouseUp { button: MouseButton::Left },
        ];

        // Simulate logic on GUI Stop button click
        if let Some(MacroEvent::MouseUp { button: MouseButton::Left }) = events.last() {
            events.pop();
            if let Some(MacroEvent::MouseDown { button: MouseButton::Left }) = events.last() {
                events.pop();
            }
            if let Some(MacroEvent::Delay { .. }) = events.last() {
                events.pop();
            }
        }

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], MacroEvent::KeyDown { .. }));
        assert!(matches!(events[1], MacroEvent::KeyUp { .. }));
    }
}
