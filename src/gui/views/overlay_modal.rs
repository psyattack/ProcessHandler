use egui::{pos2, vec2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use crate::gui::components::modal::Modal;
use crate::gui::components::widgets::{self, Glyph, Kind};
use crate::gui::state::{ColorPickerTarget, GuiState, OverlayStudioState};
use crate::gui::theme::{self, Tokens};
use crate::models::{
    CrosshairStyle, OverlayAnchor, OverlayClickAction, OverlayDefinition, OverlayElement, ShapeKind,
};

pub fn get_element_size(elem: &OverlayElement) -> (f32, f32) {
    match elem {
        OverlayElement::Crosshair { size, gap, .. } => {
            let s = (*size + *gap) * 2.0;
            (s, s)
        }
        OverlayElement::Text { text, font_size, .. } => {
            let char_len = (text.chars().count() as f32).max(1.0);
            let est_w = (char_len * (*font_size * 0.6).max(8.0) + 12.0).max(30.0);
            let est_h = (*font_size + 6.0).max(18.0);
            (est_w, est_h)
        }
        OverlayElement::Badge { text, .. } => {
            let est_w = (text.len() as f32 * 7.5 + 18.0).max(40.0);
            (est_w, 22.0)
        }
        OverlayElement::ProgressBar { width, height, .. } => {
            (*width, (*height).max(10.0))
        }
        OverlayElement::Image { width, height, .. } => {
            (*width, *height)
        }
        OverlayElement::Shape { width, height, .. } => {
            (*width, *height)
        }
    }
}

fn rotate_vec(v: egui::Vec2, angle_rad: f32) -> egui::Vec2 {
    let cos = angle_rad.cos();
    let sin = angle_rad.sin();
    vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

fn rotate_point(p: egui::Pos2, center: egui::Pos2, angle_rad: f32) -> egui::Pos2 {
    let offset = p - center;
    center + rotate_vec(offset, angle_rad)
}

fn get_rotated_aabb(center: egui::Pos2, half_w: f32, half_h: f32, angle_rad: f32) -> egui::Rect {
    let cos = angle_rad.cos().abs();
    let sin = angle_rad.sin().abs();
    let bounding_half_w = half_w * cos + half_h * sin;
    let bounding_half_h = half_w * sin + half_h * cos;
    Rect::from_center_size(center, vec2(bounding_half_w * 2.0, bounding_half_h * 2.0))
}

fn draw_filled_convex_polygon(
    painter: &egui::Painter,
    center: egui::Pos2,
    points: &[egui::Pos2],
    fill_color: Color32,
) {
    if points.len() < 3 || fill_color.a() == 0 {
        return;
    }
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, fill_color);
    for (i, &p) in points.iter().enumerate() {
        mesh.colored_vertex(p, fill_color);
        let next_i = ((i + 1) % points.len()) as u32;
        mesh.add_triangle(0, (i as u32) + 1, next_i + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn draw_polygon_stroke(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    stroke: Stroke,
) {
    if points.len() < 2 || stroke.width <= 0.0 || stroke.color.a() == 0 {
        return;
    }
    for i in 0..points.len() {
        let next_i = (i + 1) % points.len();
        painter.line_segment([points[i], points[next_i]], stroke);
    }
}

pub fn clamp_elements_to_canvas(overlay: &mut OverlayDefinition) {
    let canvas_w = overlay.rect.width.max(20.0);
    let canvas_h = overlay.rect.height.max(20.0);

    for elem in &mut overlay.elements {
        match elem {
            OverlayElement::ProgressBar { width, height, x, y, .. } => {
                *width = width.clamp(10.0, canvas_w);
                *height = height.clamp(4.0, canvas_h);
                let max_x = (canvas_w - *width).max(0.0);
                let max_y = (canvas_h - *height).max(0.0);
                *x = x.clamp(0.0, max_x);
                *y = y.clamp(0.0, max_y);
            }
            OverlayElement::Image { width, height, lock_aspect, x, y, .. } => {
                if *lock_aspect {
                    let w = *width;
                    let h = (*height).max(1.0);
                    if w > canvas_w || h > canvas_h {
                        let scale = (canvas_w / w).min(canvas_h / h);
                        *width = (w * scale).round().clamp(8.0, canvas_w);
                        *height = (h * scale).round().clamp(8.0, canvas_h);
                    } else {
                        *width = w.clamp(8.0, canvas_w);
                        *height = h.clamp(8.0, canvas_h);
                    }
                } else {
                    *width = width.clamp(8.0, canvas_w);
                    *height = height.clamp(8.0, canvas_h);
                }
                let max_x = (canvas_w - *width).max(0.0);
                let max_y = (canvas_h - *height).max(0.0);
                *x = x.clamp(0.0, max_x);
                *y = y.clamp(0.0, max_y);
            }
            OverlayElement::Shape { width, height, lock_aspect, x, y, .. } => {
                if *lock_aspect {
                    let w = *width;
                    let h = (*height).max(1.0);
                    if w > canvas_w || h > canvas_h {
                        let scale = (canvas_w / w).min(canvas_h / h);
                        *width = (w * scale).round().clamp(4.0, canvas_w);
                        *height = (h * scale).round().clamp(4.0, canvas_h);
                    } else {
                        *width = w.clamp(4.0, canvas_w);
                        *height = h.clamp(4.0, canvas_h);
                    }
                } else {
                    *width = width.clamp(4.0, canvas_w);
                    *height = height.clamp(4.0, canvas_h);
                }
                let max_x = (canvas_w - *width).max(0.0);
                let max_y = (canvas_h - *height).max(0.0);
                *x = x.clamp(0.0, max_x);
                *y = y.clamp(0.0, max_y);
            }
            OverlayElement::Crosshair { size, gap, x, y, .. } => {
                let max_dim = canvas_w.min(canvas_h);
                if (*size + *gap) * 2.0 > max_dim {
                    *size = ((max_dim / 2.0) - *gap).clamp(2.0, 128.0);
                }
                *x = x.clamp(0.0, canvas_w);
                *y = y.clamp(0.0, canvas_h);
            }
            OverlayElement::Text { font_size, text, x, y, .. } => {
                let char_len = (text.chars().count() as f32).max(1.0);
                let target_w = (canvas_w - 12.0).max(10.0);
                let target_h = (canvas_h - 6.0).max(10.0);
                let max_font_w = target_w / (char_len * 0.6);
                let max_font = max_font_w.min(target_h);
                if *font_size > max_font {
                    *font_size = (*font_size).min(max_font).clamp(6.0, 72.0);
                } else {
                    *font_size = font_size.clamp(6.0, 72.0);
                }
                let est_w = (char_len * (*font_size * 0.6).max(8.0) + 12.0).max(30.0);
                let est_h = (*font_size + 6.0).max(18.0);
                let max_x = (canvas_w - est_w).max(0.0);
                let max_y = (canvas_h - est_h).max(0.0);
                *x = x.clamp(0.0, max_x);
                *y = y.clamp(0.0, max_y);
            }
            OverlayElement::Badge { text, x, y, .. } => {
                let est_w = (text.len() as f32 * 7.5 + 18.0).max(40.0);
                let est_h = 22.0;
                let max_x = (canvas_w - est_w).max(0.0);
                let max_y = (canvas_h - est_h).max(0.0);
                *x = x.clamp(0.0, max_x);
                *y = y.clamp(0.0, max_y);
            }
        }
    }
}

pub fn render_overlay_modal(ctx: &egui::Context, state: &mut GuiState) {
    let is_open = state.overlay_studio.is_some();
    let active_studio = state
        .overlay_studio
        .as_ref()
        .or(state.last_overlay_studio.as_ref())
        .cloned();

    let Some(mut studio) = active_studio else { return };
    clamp_elements_to_canvas(&mut studio.overlay);
    let t = Tokens::get(ctx);
    let loc = state.loc();
    let mut initial_overlay = studio.overlay.clone();

    let mut toggle_calibration = false;

    // Synchronize live on-screen calibration coordinates from daemon via IPC
    // Only poll from daemon if the user is not actively interacting/dragging with egui
    let is_gui_interacting = ctx.input(|i| i.pointer.any_down());
    if studio.is_calibrating && !is_gui_interacting {
        if let Ok(crate::ipc::IpcResponse::OverlayPosition(Some((x, y)))) =
            crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::GetOverlayPosition(studio.overlay.id.clone()))
        {
            let pos_differs_from_current = (studio.overlay.rect.x - x).abs() > 0.5 || (studio.overlay.rect.y - y).abs() > 0.5;
            let pos_differs_from_last_synced = studio.last_synced_pos.is_none_or(|(lx, ly)| (lx - x).abs() > 0.5 || (ly - y).abs() > 0.5);

            if pos_differs_from_current && pos_differs_from_last_synced {
                studio.overlay.anchor = OverlayAnchor::Custom;
                studio.overlay.rect.x = x;
                studio.overlay.rect.y = y;
                studio.last_synced_pos = Some((x, y));
                initial_overlay = studio.overlay.clone();
                ctx.request_repaint();
            }
        }
    }

    let rules_list: Vec<(String, String)> = state
        .daemon_state
        .as_ref()
        .map(|s| s.config.rules.iter().map(|r| (r.id.clone(), r.name.clone())).collect())
        .unwrap_or_default();

    // Studio Keyboard Shortcuts (processed before UI rendering for 0-lag and instant consumption)
    let is_typing = ctx.text_edit_focused();

    if is_open && !is_typing {
        // Tab / Shift+Tab -> Cycle selected element forward / backward
        let mut tab_nav: Option<bool> = None;
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } = event {
                    tab_nav = Some(!modifiers.shift);
                }
            }
        });

        if let Some(forward) = tab_nav {
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
            });
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
                m.move_focus(egui::FocusDirection::None);
            });
            let total = studio.overlay.elements.len();
            if total > 0 {
                studio.selected_element_idx = match studio.selected_element_idx {
                    Some(cur) => {
                        let cur = cur.min(total - 1);
                        if forward {
                            Some((cur + 1) % total)
                        } else {
                            Some((cur + total - 1) % total)
                        }
                    }
                    None => {
                        if forward {
                            Some(0)
                        } else {
                            Some(total - 1)
                        }
                    }
                };
                ctx.request_repaint();
            }
        }

        // Delete or Backspace -> remove selected element
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete) || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)) {
            if let Some(sel) = studio.selected_element_idx {
                if sel < studio.overlay.elements.len() {
                    studio.overlay.elements.remove(sel);
                    studio.selected_element_idx = if studio.overlay.elements.is_empty() {
                        None
                    } else {
                        Some(sel.min(studio.overlay.elements.len() - 1))
                    };
                    studio.active_color_target = None;
                }
            }
        }

        // Ctrl+D -> duplicate/clone selected element
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::D) || i.consume_key(egui::Modifiers::CTRL, egui::Key::D)) {
            if let Some(sel) = studio.selected_element_idx {
                if sel < studio.overlay.elements.len() {
                    let mut cloned = studio.overlay.elements[sel].clone();
                    let (ew, eh) = get_element_size(&cloned);
                    match &mut cloned {
                        OverlayElement::Text { id, x, y, .. }
                        | OverlayElement::Badge { id, x, y, .. }
                        | OverlayElement::ProgressBar { id, x, y, .. }
                        | OverlayElement::Image { id, x, y, .. }
                        | OverlayElement::Shape { id, x, y, .. }
                        | OverlayElement::Crosshair { id, x, y, .. } => {
                            *id = format!("{}_copy", id);
                            let cand_x = *x + 16.0;
                            let cand_y = *y + 16.0;
                            if cand_x + ew > studio.overlay.rect.width || cand_y + eh > studio.overlay.rect.height {
                                *x = 0.0;
                                *y = 0.0;
                            } else {
                                *x = cand_x;
                                *y = cand_y;
                            }
                        }
                    }
                    studio.overlay.elements.insert(sel + 1, cloned);
                    clamp_elements_to_canvas(&mut studio.overlay);
                    studio.selected_element_idx = Some(sel + 1);
                }
            }
        }

        // Arrow keys -> nudge selected element
        let step = if studio.snap_to_grid || ctx.input(|i| i.modifiers.shift) { 16.0 } else { 1.0 };
        let mut nudge_x = 0.0f32;
        let mut nudge_y = 0.0f32;
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) { nudge_x -= step; }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) { nudge_x += step; }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) { nudge_y -= step; }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) { nudge_y += step; }

        if nudge_x != 0.0 || nudge_y != 0.0 {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            if let Some(sel) = studio.selected_element_idx {
                if sel < studio.overlay.elements.len() {
                    let (ew, eh) = get_element_size(&studio.overlay.elements[sel]);
                    match &mut studio.overlay.elements[sel] {
                        OverlayElement::Text { x, y, .. }
                        | OverlayElement::Badge { x, y, .. }
                        | OverlayElement::ProgressBar { x, y, .. }
                        | OverlayElement::Image { x, y, .. }
                        | OverlayElement::Shape { x, y, .. } => {
                            *x = (*x + nudge_x).clamp(0.0, (studio.overlay.rect.width - ew).max(0.0));
                            *y = (*y + nudge_y).clamp(0.0, (studio.overlay.rect.height - eh).max(0.0));
                        }
                        OverlayElement::Crosshair { x, y, .. } => {
                            *x = (*x + nudge_x).clamp(0.0, studio.overlay.rect.width);
                            *y = (*y + nudge_y).clamp(0.0, studio.overlay.rect.height);
                        }
                    }
                }
            }
        }

        // [ / ] -> Layer reordering (move layer down / up)
        if ctx.input(|i| i.key_pressed(egui::Key::OpenBracket)) {
            if let Some(sel) = studio.selected_element_idx {
                if sel > 0 && sel < studio.overlay.elements.len() {
                    let item = studio.overlay.elements.remove(sel);
                    studio.overlay.elements.insert(sel - 1, item);
                    studio.selected_element_idx = Some(sel - 1);
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::CloseBracket)) {
            if let Some(sel) = studio.selected_element_idx {
                if sel + 1 < studio.overlay.elements.len() {
                    let item = studio.overlay.elements.remove(sel);
                    studio.overlay.elements.insert(sel + 1, item);
                    studio.selected_element_idx = Some(sel + 1);
                }
            }
        }

        // Escape: deselect element or close color picker
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if studio.active_color_target.is_some() {
                studio.active_color_target = None;
            } else if studio.selected_element_idx.is_some() {
                studio.selected_element_idx = None;
            }
        }
    } else if studio.active_color_target.is_some() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        studio.active_color_target = None;
    }

    let modal_res = Modal::new("overlay-studio-modal", is_open)
        .width_fraction(0.92)
        .height_fraction(0.92)
        .min_size(vec2(920.0, 600.0))
        .max_size(vec2(1360.0, 940.0))
        .accent_bar(t.accent_ink)
        .header_height(50.0)
        .close_on_escape(studio.active_color_target.is_none())
        .show(ctx, &t, |frame| {
            // ---------------- 1. HEADER ----------------
            frame.header(|ui, close_req| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    ui.label(
                        RichText::new(egui_phosphor::regular::FRAME_CORNERS)
                            .family(theme::phosphor())
                            .size(16.0)
                            .color(t.accent_ink),
                    );
                    ui.add_space(6.0);
                    widgets::hud(ui, loc.overlay_studio_title, t.ink);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.modal_tip_close).clicked() {
                            *close_req = true;
                        }

                        // Calibrate Position on Screen Button
                        ui.add_space(8.0);
                        let cal_text = if studio.is_calibrating {
                            loc.overlay_studio_btn_calibrate_hide
                        } else {
                            loc.overlay_studio_btn_calibrate
                        };
                        let cal_kind = if studio.is_calibrating { Kind::Danger } else { Kind::Secondary };
                        if widgets::button(ui, cal_text, cal_kind, 0.0).clicked() {
                            toggle_calibration = true;
                        }
                    });
                });
            });

            // ---------------- 2. BODY: SPLIT LAYOUT ----------------
            frame.body(|ui| {
                let body_rect = ui.max_rect();
                let left_w = (body_rect.width() * 0.44).clamp(380.0, 520.0);
                let left_rect = Rect::from_min_size(body_rect.min, vec2(left_w, body_rect.height()));
                let right_rect = Rect::from_min_max(
                    pos2(left_rect.right() + 1.0, body_rect.min.y),
                    body_rect.max,
                );

                // Vertical Divider between panels
                ui.painter().vline(left_rect.right(), body_rect.y_range(), t.hairline());

                // LEFT PANEL: Overlay Settings & Elements Inspector
                ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                    ui.set_width(left_rect.width());
                    ui.set_height(left_rect.height());
                    ui.set_clip_rect(left_rect);

                    egui::Frame::new()
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("overlay_studio_left_scroll")
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    render_overlay_inspector(
                                        ui,
                                        &mut studio.overlay,
                                        studio.is_calibrating,
                                        &mut studio.card_lock_aspect,
                                        &mut studio.selected_element_idx,
                                        &mut studio.active_color_target,
                                        &rules_list,
                                        &t,
                                        loc,
                                    );
                                });
                        });
                });

                // RIGHT PANEL: Canvas Preview
                ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                    ui.set_width(right_rect.width());
                    ui.set_height(right_rect.height());
                    ui.set_clip_rect(right_rect);

                    egui::Frame::new()
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            render_overlay_canvas(
                                ui,
                                &mut studio,
                                &t,
                                loc,
                            );
                        });
                });
            });
        });

    if modal_res.is_some() {
        if modal_res.as_ref().is_some_and(|r| r.close_requested) {
            state.close_overlay_studio();
        } else if is_open {
            if toggle_calibration {
                studio.is_calibrating = !studio.is_calibrating;
                if studio.is_calibrating {
                    let (screen_w, screen_h) = get_screen_resolution();
                    let (csx, csy) = compute_overlay_screen_pos(&studio.overlay, screen_w, screen_h);
                    studio.last_synced_pos = Some((csx, csy));
                    let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ShowOverlayPreview {
                        overlay: studio.overlay.clone(),
                        calibration_mode: true,
                    });
                } else {
                    studio.last_synced_pos = None;
                    let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::HideOverlayPreview(
                        studio.overlay.id.clone(),
                    ));
                }
            }

            if studio.is_calibrating && studio.overlay != initial_overlay {
                let (screen_w, screen_h) = get_screen_resolution();
                let (csx, csy) = compute_overlay_screen_pos(&studio.overlay, screen_w, screen_h);
                studio.last_synced_pos = Some((csx, csy));
                let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ShowOverlayPreview {
                    overlay: studio.overlay.clone(),
                    calibration_mode: true,
                });
            }

            // Real-time auto-saving directly into editing_rule
            if let Some(target_idx) = studio.action_target_idx {
                if let Some(rule) = state.editing_rule.as_mut() {
                    let actions = if studio.is_exit_pipeline {
                        &mut rule.on_exit_actions
                    } else {
                        &mut rule.actions
                    };
                    if let Some(crate::models::Action::Overlay { custom_overlay, overlay_id, .. }) = actions.get_mut(target_idx) {
                        *custom_overlay = Some(Box::new(studio.overlay.clone()));
                        *overlay_id = studio.overlay.id.clone();
                    }
                }
            }

            state.overlay_studio = Some(studio);
        }
    } else if !is_open {
        state.last_overlay_studio = None;
    }
}

fn render_rotation_control(
    ui: &mut Ui,
    rotation: &mut f32,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(loc.overlay_studio_rotation).family(theme::semibold()).size(10.5).color(t.faint));
        ui.add(
            egui::DragValue::new(rotation)
                .range(-360.0..=360.0)
                .speed(1.0)
                .suffix("°")
        );
    });
}

fn render_color_chip(
    ui: &mut Ui,
    label: &str,
    color: &mut [u8; 4],
    target: ColorPickerTarget,
    active_target: &mut Option<ColorPickerTarget>,
    t: &Tokens,
) {
    let is_active = *active_target == Some(target);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).family(theme::semibold()).size(10.5).color(t.faint));

        // Compact rounded rectangle swatch (vec2(28.0, 15.0)), no large square, no hex text
        let swatch_size = vec2(28.0, 15.0);
        let id = ui.id().with(("swatch", target, label));
        let (rect, _) = ui.allocate_exact_size(swatch_size, Sense::hover());
        let resp = ui.interact(rect, id, Sense::click());
        let p = ui.painter();

        // 1. Checkerboard pattern for alpha transparency
        let half_w = rect.width() / 2.0;
        let half_h = rect.height() / 2.0;
        p.rect_filled(rect, CornerRadius::same(3), Color32::from_gray(50));
        p.rect_filled(Rect::from_min_size(rect.min, vec2(half_w, half_h)), CornerRadius::ZERO, Color32::from_gray(90));
        p.rect_filled(Rect::from_min_size(rect.min + vec2(half_w, half_h), vec2(half_w, half_h)), CornerRadius::ZERO, Color32::from_gray(90));

        // 2. Color fill with alpha
        let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
        p.rect_filled(rect, CornerRadius::same(3), col);

        // 3. Border stroke: accent ink if active, subtle hairline otherwise
        let stroke = if is_active {
            Stroke::new(1.5, t.accent_ink)
        } else if resp.hovered() {
            Stroke::new(1.0, Color32::WHITE)
        } else {
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30))
        };
        p.rect_stroke(rect, CornerRadius::same(3), stroke, egui::StrokeKind::Inside);

        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            if is_active {
                *active_target = None;
            } else {
                *active_target = Some(target);
            }
        }
    });
}

fn render_inline_color_editor(
    ui: &mut Ui,
    color: &mut [u8; 4],
    has_alpha: bool,
    t: &Tokens,
) {
    egui::Frame::new()
        .fill(t.sunken)
        .stroke(Stroke::new(1.0, t.accent_ink))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            // Maintain stable HSVA state across frames to prevent hue/saturation snapping at extremes
            let hsva_id = ui.id().with("custom_picker_hsva");
            let last_rgba_id = ui.id().with("custom_picker_last_rgba");

            let last_rgba: [u8; 4] = ui.ctx().data(|d| d.get_temp(last_rgba_id)).unwrap_or([255, 255, 255, 255]);
            let mut hsva: egui::ecolor::Hsva = ui.ctx().data(|d| d.get_temp(hsva_id)).unwrap_or_else(|| {
                egui::ecolor::Hsva::from_srgba_unmultiplied(*color)
            });

            if *color != last_rgba {
                hsva = egui::ecolor::Hsva::from_srgba_unmultiplied(*color);
            }

            // Row 1: Quick Presets Palette + Direct HEX input in single row
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let presets: [(&str, [u8; 4]); 7] = [
                    ("Cyan", [0, 212, 255, 255]),
                    ("Green", [34, 197, 94, 255]),
                    ("Red", [239, 68, 68, 255]),
                    ("Amber", [245, 158, 11, 255]),
                    ("White", [255, 255, 255, 255]),
                    ("Dark", [24, 28, 38, 220]),
                    ("Clear", [0, 0, 0, 0]),
                ];
                for (name, col) in presets {
                    let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                    let c32 = Color32::from_rgba_unmultiplied(col[0], col[1], col[2], col[3]);
                    ui.painter().rect_filled(rect, CornerRadius::same(2), c32);
                    ui.painter().rect_stroke(rect, CornerRadius::same(2), t.hairline(), egui::StrokeKind::Inside);
                    if resp.on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        color[0] = col[0];
                        color[1] = col[1];
                        color[2] = col[2];
                        if has_alpha {
                            color[3] = col[3];
                        }
                        hsva = egui::ecolor::Hsva::from_srgba_unmultiplied(*color);
                    }
                }

                ui.add_space(2.0);

                let hex_id = ui.id().with("hex_edit_buf");
                let mut hex_val: String = ui.ctx().data(|d| d.get_temp(hex_id)).unwrap_or_else(|| {
                    if has_alpha {
                        format!("#{:02X}{:02X}{:02X}{:02X}", color[0], color[1], color[2], color[3])
                    } else {
                        format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
                    }
                });
                let prev_hex = hex_val.clone();
                let txt_resp = ui.add(
                    egui::TextEdit::singleline(&mut hex_val)
                        .desired_width(ui.available_width())
                        .font(FontId::new(11.0, theme::mono()))
                );
                if txt_resp.changed() {
                    let clean = hex_val.trim().trim_start_matches('#');
                    if clean.len() == 6 {
                        if let Ok(v) = u32::from_str_radix(clean, 16) {
                            color[0] = ((v >> 16) & 0xFF) as u8;
                            color[1] = ((v >> 8) & 0xFF) as u8;
                            color[2] = (v & 0xFF) as u8;
                            hsva = egui::ecolor::Hsva::from_srgba_unmultiplied(*color);
                        }
                    } else if clean.len() == 8 {
                        if let Ok(v) = u32::from_str_radix(clean, 16) {
                            color[0] = ((v >> 24) & 0xFF) as u8;
                            color[1] = ((v >> 16) & 0xFF) as u8;
                            color[2] = ((v >> 8) & 0xFF) as u8;
                            color[3] = (v & 0xFF) as u8;
                            hsva = egui::ecolor::Hsva::from_srgba_unmultiplied(*color);
                        }
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(hex_id, hex_val.clone()));
                } else if !txt_resp.has_focus() {
                    let latest_hex = if has_alpha {
                        format!("#{:02X}{:02X}{:02X}{:02X}", color[0], color[1], color[2], color[3])
                    } else {
                        format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
                    };
                    if latest_hex != prev_hex {
                        ui.ctx().data_mut(|d| d.insert_temp(hex_id, latest_hex));
                    }
                }
            });

            // Row 2: Full-width 2D Saturation / Value gradient canvas
            let full_w = ui.available_width();
            let sv_h = 110.0;
            let (sv_rect, sv_resp) = ui.allocate_exact_size(vec2(full_w, sv_h), Sense::click_and_drag());

            let pure_hue = Color32::from(egui::ecolor::Hsva::new(hsva.h, 1.0, 1.0, 1.0));
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(pos2(sv_rect.left(), sv_rect.top()), Color32::WHITE);
            mesh.colored_vertex(pos2(sv_rect.right(), sv_rect.top()), pure_hue);
            mesh.colored_vertex(pos2(sv_rect.left(), sv_rect.bottom()), Color32::BLACK);
            mesh.colored_vertex(pos2(sv_rect.right(), sv_rect.bottom()), Color32::BLACK);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 1, 3);
            ui.painter().add(egui::Shape::mesh(mesh));
            ui.painter().rect_stroke(sv_rect, CornerRadius::same(3), t.hairline(), egui::StrokeKind::Inside);

            let pointer_pos = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));
            if sv_resp.clicked() || sv_resp.dragged() {
                if let Some(pos) = pointer_pos {
                    hsva.s = ((pos.x - sv_rect.left()) / sv_rect.width()).clamp(0.0, 1.0);
                    hsva.v = (1.0 - (pos.y - sv_rect.top()) / sv_rect.height()).clamp(0.0, 1.0);
                }
            }

            // Crosshair indicator on current (s, v)
            let cx = sv_rect.left() + hsva.s * sv_rect.width();
            let cy = sv_rect.top() + (1.0 - hsva.v) * sv_rect.height();
            ui.painter().circle(pos2(cx, cy), 4.5, Color32::TRANSPARENT, Stroke::new(1.8, Color32::BLACK));
            ui.painter().circle(pos2(cx, cy), 3.5, Color32::TRANSPARENT, Stroke::new(1.5, Color32::WHITE));

            // Row 3: Full-width Hue spectrum bar
            let (hue_rect, hue_resp) = ui.allocate_exact_size(vec2(full_w, 13.0), Sense::click_and_drag());
            let stops: [(f32, Color32); 7] = [
                (0.0, Color32::from_rgb(255, 0, 0)),
                (0.1666, Color32::from_rgb(255, 255, 0)),
                (0.3333, Color32::from_rgb(0, 255, 0)),
                (0.5, Color32::from_rgb(0, 255, 255)),
                (0.6666, Color32::from_rgb(0, 0, 255)),
                (0.8333, Color32::from_rgb(255, 0, 255)),
                (1.0, Color32::from_rgb(255, 0, 0)),
            ];
            let mut h_mesh = egui::Mesh::default();
            for i in 0..6 {
                let (p0, c0) = stops[i];
                let (p1, c1) = stops[i + 1];
                let x0 = hue_rect.left() + p0 * hue_rect.width();
                let x1 = hue_rect.left() + p1 * hue_rect.width();
                let v_start = h_mesh.vertices.len() as u32;
                h_mesh.colored_vertex(pos2(x0, hue_rect.top()), c0);
                h_mesh.colored_vertex(pos2(x1, hue_rect.top()), c1);
                h_mesh.colored_vertex(pos2(x0, hue_rect.bottom()), c0);
                h_mesh.colored_vertex(pos2(x1, hue_rect.bottom()), c1);
                h_mesh.add_triangle(v_start, v_start + 1, v_start + 2);
                h_mesh.add_triangle(v_start + 2, v_start + 1, v_start + 3);
            }
            ui.painter().add(egui::Shape::mesh(h_mesh));
            ui.painter().rect_stroke(hue_rect, CornerRadius::same(2), t.hairline(), egui::StrokeKind::Inside);

            if hue_resp.clicked() || hue_resp.dragged() {
                if let Some(pos) = pointer_pos {
                    hsva.h = ((pos.x - hue_rect.left()) / hue_rect.width()).clamp(0.0, 1.0);
                }
            }

            let h_thumb_x = hue_rect.left() + hsva.h * hue_rect.width();
            ui.painter().rect(
                Rect::from_center_size(pos2(h_thumb_x, hue_rect.center().y), vec2(5.0, 15.0)),
                CornerRadius::same(2),
                Color32::WHITE,
                Stroke::new(1.0, Color32::BLACK),
                egui::StrokeKind::Middle,
            );

            // Row 4: Full-width Alpha bar (if has_alpha)
            if has_alpha {
                let (alpha_rect, alpha_resp) = ui.allocate_exact_size(vec2(full_w, 13.0), Sense::click_and_drag());
                let half_sq = 4.0;
                let mut bx = alpha_rect.left();
                let mut row = 0;
                while bx < alpha_rect.right() {
                    let mut by = alpha_rect.top();
                    let mut col = 0;
                    while by < alpha_rect.bottom() {
                        let fill = if (row + col) % 2 == 0 { Color32::from_gray(60) } else { Color32::from_gray(100) };
                        let r = Rect::from_min_size(pos2(bx, by), vec2(half_sq, half_sq)).intersect(alpha_rect);
                        ui.painter().rect_filled(r, CornerRadius::ZERO, fill);
                        by += half_sq;
                        col += 1;
                    }
                    bx += half_sq;
                    row += 1;
                }

                let col_0 = Color32::from(egui::ecolor::Hsva::new(hsva.h, hsva.s, hsva.v, 0.0));
                let col_1 = Color32::from(egui::ecolor::Hsva::new(hsva.h, hsva.s, hsva.v, 1.0));
                let mut a_mesh = egui::Mesh::default();
                a_mesh.colored_vertex(pos2(alpha_rect.left(), alpha_rect.top()), col_0);
                a_mesh.colored_vertex(pos2(alpha_rect.right(), alpha_rect.top()), col_1);
                a_mesh.colored_vertex(pos2(alpha_rect.left(), alpha_rect.bottom()), col_0);
                a_mesh.colored_vertex(pos2(alpha_rect.right(), alpha_rect.bottom()), col_1);
                a_mesh.add_triangle(0, 1, 2);
                a_mesh.add_triangle(2, 1, 3);
                ui.painter().add(egui::Shape::mesh(a_mesh));
                ui.painter().rect_stroke(alpha_rect, CornerRadius::same(2), t.hairline(), egui::StrokeKind::Inside);

                if alpha_resp.clicked() || alpha_resp.dragged() {
                    if let Some(pos) = pointer_pos {
                        hsva.a = ((pos.x - alpha_rect.left()) / alpha_rect.width()).clamp(0.0, 1.0);
                    }
                }

                let a_thumb_x = alpha_rect.left() + hsva.a * alpha_rect.width();
                ui.painter().rect(
                    Rect::from_center_size(pos2(a_thumb_x, alpha_rect.center().y), vec2(5.0, 15.0)),
                    CornerRadius::same(2),
                    Color32::WHITE,
                    Stroke::new(1.0, Color32::BLACK),
                    egui::StrokeKind::Middle,
                );
            }

            // Sync updated HSVA back to RGBA
            let c32 = Color32::from(hsva);
            color[0] = c32.r();
            color[1] = c32.g();
            color[2] = c32.b();
            if has_alpha {
                color[3] = (hsva.a * 255.0).round() as u8;
            }

            ui.ctx().data_mut(|d| {
                d.insert_temp(hsva_id, hsva);
                d.insert_temp(last_rgba_id, *color);
            });
        });
}

#[derive(Clone, Copy, Debug, Default)]
struct ElementDndSession {
    source_idx: usize,
    target_idx: usize,
}

fn render_elem_drop_slot(
    ui: &mut Ui,
    slot_idx: usize,
    active_dst: Option<usize>,
    active_src: Option<usize>,
    total_elems: usize,
    t: &Tokens,
) {
    let is_targeted = if let (Some(dst), Some(src)) = (active_dst, active_src) {
        if slot_idx == total_elems {
            dst == slot_idx && src + 1 < total_elems
        } else {
            dst == slot_idx && slot_idx != src && slot_idx != src + 1
        }
    } else {
        false
    };

    let anim_id = ui.id().with(("elem_drop_anim", slot_idx));
    let anim = ui.ctx().animate_bool_with_time_and_easing(anim_id, is_targeted, 0.16, egui::emath::easing::cubic_out);
    if anim > 0.001 {
        let slot_h = 20.0 * anim;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), slot_h), Sense::hover());
        let p = ui.painter();
        let mid_y = rect.center().y;
        p.rect_filled(rect, CornerRadius::same(2), t.accent_ink.gamma_multiply(0.08 * anim));
        p.hline(rect.left() + 20.0..=rect.right() - 8.0, mid_y, Stroke::new(2.0 * anim, t.accent_ink.gamma_multiply(anim)));
        p.circle_filled(pos2(rect.left() + 10.0, mid_y), 3.5 * anim, t.accent_ink.gamma_multiply(anim));
    }
}

// ----------------- OVERLAY INSPECTOR PANEL -----------------
#[allow(clippy::too_many_arguments)]
fn render_overlay_inspector(
    ui: &mut Ui,
    overlay: &mut OverlayDefinition,
    is_calibrating: bool,
    card_lock_aspect: &mut bool,
    selected_idx: &mut Option<usize>,
    active_color_target: &mut Option<ColorPickerTarget>,
    rules_list: &[(String, String)],
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    ui.spacing_mut().item_spacing.y = 8.0;

    // Anchor Picker
    widgets::hud(ui, loc.overlay_studio_field_anchor, t.faint);
    let anchor_label = match overlay.anchor {
        OverlayAnchor::ScreenCenter => loc.overlay_studio_anchor_center,
        OverlayAnchor::TopLeft => loc.overlay_studio_anchor_top_left,
        OverlayAnchor::TopRight => loc.overlay_studio_anchor_top_right,
        OverlayAnchor::BottomLeft => loc.overlay_studio_anchor_bottom_left,
        OverlayAnchor::BottomRight => loc.overlay_studio_anchor_bottom_right,
        OverlayAnchor::Custom => loc.overlay_studio_anchor_custom,
        OverlayAnchor::WindowRelative => loc.overlay_studio_anchor_window,
    };
    let prev_anchor = overlay.anchor;
    egui::ComboBox::from_id_salt("overlay_anchor_dropdown")
        .selected_text(anchor_label)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut overlay.anchor, OverlayAnchor::ScreenCenter, loc.overlay_studio_anchor_center);
            ui.selectable_value(&mut overlay.anchor, OverlayAnchor::TopLeft, loc.overlay_studio_anchor_top_left);
            ui.selectable_value(&mut overlay.anchor, OverlayAnchor::TopRight, loc.overlay_studio_anchor_top_right);
            ui.selectable_value(&mut overlay.anchor, OverlayAnchor::BottomLeft, loc.overlay_studio_anchor_bottom_left);
            ui.selectable_value(&mut overlay.anchor, OverlayAnchor::BottomRight, loc.overlay_studio_anchor_bottom_right);
            ui.selectable_value(&mut overlay.anchor, OverlayAnchor::Custom, loc.overlay_studio_anchor_custom);
            ui.selectable_value(&mut overlay.anchor, OverlayAnchor::WindowRelative, loc.overlay_studio_anchor_window);
        });

    let (screen_w, screen_h) = get_screen_resolution();
    if overlay.anchor != prev_anchor {
        match overlay.anchor {
            OverlayAnchor::ScreenCenter => {
                overlay.rect.x = 0.0;
                overlay.rect.y = 0.0;
            }
            OverlayAnchor::TopLeft | OverlayAnchor::TopRight | OverlayAnchor::BottomLeft | OverlayAnchor::BottomRight => {
                overlay.rect.x = 16.0;
                overlay.rect.y = 16.0;
            }
            OverlayAnchor::Custom => {
                let (csx, csy) = compute_overlay_screen_pos_with_anchor(prev_anchor, overlay, screen_w, screen_h);
                overlay.rect.x = csx.clamp(0.0, (screen_w - overlay.rect.width).max(0.0));
                overlay.rect.y = csy.clamp(0.0, (screen_h - overlay.rect.height).max(0.0));
            }
            OverlayAnchor::WindowRelative => {
                overlay.rect.x = 16.0;
                overlay.rect.y = 16.0;
            }
        }
        clamp_card_to_monitor(overlay, screen_w, screen_h);
    }

    if overlay.anchor == OverlayAnchor::Custom || is_calibrating {
        let max_x = (screen_w - overlay.rect.width).max(0.0);
        let max_y = (screen_h - overlay.rect.height).max(0.0);

        let (mut cur_x, mut cur_y) = if overlay.anchor == OverlayAnchor::Custom {
            (overlay.rect.x, overlay.rect.y)
        } else {
            compute_overlay_screen_pos(overlay, screen_w, screen_h)
        };

        let mut pos_changed = false;
        ui.columns(2, |cols| {
            if cols[0].add(
                egui::DragValue::new(&mut cur_x)
                    .range(0.0..=max_x)
                    .prefix("Screen X: ")
                    .suffix(" px")
                    .speed(1.0),
            ).changed() {
                overlay.anchor = OverlayAnchor::Custom;
                overlay.rect.x = cur_x;
                pos_changed = true;
            }
            if cols[1].add(
                egui::DragValue::new(&mut cur_y)
                    .range(0.0..=max_y)
                    .prefix("Screen Y: ")
                    .suffix(" px")
                    .speed(1.0),
            ).changed() {
                overlay.anchor = OverlayAnchor::Custom;
                overlay.rect.y = cur_y;
                pos_changed = true;
            }
        });
        if pos_changed {
            clamp_card_to_monitor(overlay, screen_w, screen_h);
        }
    }

    if overlay.anchor == OverlayAnchor::WindowRelative {
        let mut proc_target = overlay.target_process.clone().unwrap_or_default();
        if ui.add(
            egui::TextEdit::singleline(&mut proc_target)
                .desired_width(ui.available_width())
                .hint_text(loc.overlay_studio_hint_process)
        ).changed() {
            overlay.target_process = if proc_target.trim().is_empty() { None } else { Some(proc_target.trim().to_string()) };
        }
    }

    // Geometry Dimensions (W x H) with Lock Aspect Ratio button
    ui.add_space(6.0);
    widgets::hud(ui, loc.overlay_studio_card_dimensions, t.faint);

    let (max_w, max_h) = max_card_dimensions(overlay, screen_w, screen_h);

    let total_w = ui.available_width();
    let btn_w = theme::CONTROL_H;
    let spacing = ui.spacing().item_spacing.x;
    let field_w = ((total_w - btn_w - spacing * 2.0 - 4.0) / 2.0).max(40.0).floor();

    let prev_w = overlay.rect.width;
    let prev_h = overlay.rect.height;
    let aspect = if prev_h > 0.0 { prev_w / prev_h } else { 1.0 };

    let mut dim_changed = false;
    ui.horizontal(|ui| {
        let w_resp = ui.add_sized(
            vec2(field_w, theme::CONTROL_H),
            egui::DragValue::new(&mut overlay.rect.width)
                .range(20.0..=max_w)
                .prefix("Width: ")
                .suffix(" px")
                .speed(1.0),
        );
        let h_resp = ui.add_sized(
            vec2(field_w, theme::CONTROL_H),
            egui::DragValue::new(&mut overlay.rect.height)
                .range(20.0..=max_h)
                .prefix("Height: ")
                .suffix(" px")
                .speed(1.0),
        );

        if *card_lock_aspect {
            let effective_max_w = max_w.min(max_h * aspect);
            let effective_max_h = max_h.min(max_w / aspect.max(0.01));
            if w_resp.changed() && aspect > 0.0 {
                overlay.rect.width = overlay.rect.width.clamp(20.0, effective_max_w);
                overlay.rect.height = (overlay.rect.width / aspect).round().clamp(20.0, effective_max_h);
                dim_changed = true;
            } else if h_resp.changed() {
                overlay.rect.height = overlay.rect.height.clamp(20.0, effective_max_h);
                overlay.rect.width = (overlay.rect.height * aspect).round().clamp(20.0, effective_max_w);
                dim_changed = true;
            }
        } else {
            if w_resp.changed() {
                overlay.rect.width = overlay.rect.width.clamp(20.0, max_w);
                dim_changed = true;
            }
            if h_resp.changed() {
                overlay.rect.height = overlay.rect.height.clamp(20.0, max_h);
                dim_changed = true;
            }
        }

        let lock_glyph = if *card_lock_aspect {
            egui_phosphor::regular::LOCK
        } else {
            egui_phosphor::regular::LOCK_OPEN
        };
        let lock_color = if *card_lock_aspect { t.accent_ink } else { t.faint };
        let lock_tip = if *card_lock_aspect {
            loc.overlay_studio_card_lock_aspect
        } else {
            loc.overlay_studio_card_unlock_aspect
        };
        let (lock_rect, lock_resp) = ui.allocate_exact_size(vec2(btn_w, theme::CONTROL_H), Sense::click());
        if *card_lock_aspect {
            ui.painter().rect_filled(lock_rect, CornerRadius::same(3), t.accent_ink.gamma_multiply(0.2));
        } else if lock_resp.hovered() {
            ui.painter().rect_filled(lock_rect, CornerRadius::same(3), t.sunken);
        }
        ui.painter().rect_stroke(lock_rect, CornerRadius::same(3), t.hairline(), egui::StrokeKind::Inside);
        ui.painter().text(
            lock_rect.center(),
            egui::Align2::CENTER_CENTER,
            lock_glyph,
            FontId::new(14.0, theme::phosphor()),
            lock_color,
        );
        if lock_resp.on_hover_text(lock_tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            *card_lock_aspect = !*card_lock_aspect;
        }
    });
    if dim_changed {
        clamp_elements_to_canvas(overlay);
    }

    // Global Opacity Slider
    ui.add_space(6.0);
    widgets::hud(ui, loc.overlay_studio_opacity, t.faint);
    ui.add(
        egui::Slider::new(&mut overlay.opacity, 0.05..=1.0)
            .show_value(true)
            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
    );

    // Fade Animation (Fade In / Fade Out in 2 columns)
    ui.add_space(4.0);
    widgets::hud(ui, loc.inspector_overlay_fade_title, t.faint);
    ui.columns(2, |cols| {
        let mut fade_in = overlay.fade_in_ms.unwrap_or(0);
        if cols[0].add_sized(
            vec2(cols[0].available_width(), theme::CONTROL_H),
            egui::DragValue::new(&mut fade_in)
                .range(0..=5000)
                .speed(10.0)
                .prefix(format!("{}: ", loc.inspector_overlay_fade_in))
                .suffix(" ms"),
        ).changed() {
            overlay.fade_in_ms = if fade_in == 0 { None } else { Some(fade_in) };
        }

        let mut fade_out = overlay.fade_out_ms.unwrap_or(0);
        if cols[1].add_sized(
            vec2(cols[1].available_width(), theme::CONTROL_H),
            egui::DragValue::new(&mut fade_out)
                .range(0..=5000)
                .speed(10.0)
                .prefix(format!("{}: ", loc.inspector_overlay_fade_out))
                .suffix(" ms"),
        ).changed() {
            overlay.fade_out_ms = if fade_out == 0 { None } else { Some(fade_out) };
        }
    });

    // Style Card Settings: Colors in 2 columns
    ui.add_space(6.0);
    widgets::hud(ui, loc.overlay_studio_card_colors, t.faint);
    ui.columns(2, |cols| {
        render_color_chip(&mut cols[0], "BG:", &mut overlay.style.bg_color, ColorPickerTarget::CardBg, active_color_target, t);
        render_color_chip(&mut cols[1], "BORDER:", &mut overlay.style.border_color, ColorPickerTarget::CardBorder, active_color_target, t);
    });
    if *active_color_target == Some(ColorPickerTarget::CardBg) {
        render_inline_color_editor(ui, &mut overlay.style.bg_color, true, t);
    } else if *active_color_target == Some(ColorPickerTarget::CardBorder) {
        render_inline_color_editor(ui, &mut overlay.style.border_color, true, t);
    }

    // Border Width & Corner Radius in 2 columns
    ui.columns(2, |cols| {
        cols[0].add(egui::DragValue::new(&mut overlay.style.border_width).range(0.0..=12.0).prefix("Border: ").suffix(" px"));
        cols[1].add(egui::DragValue::new(&mut overlay.style.corner_radius).range(0.0..=32.0).prefix("Radius: ").suffix(" px"));
    });

    widgets::hrule(ui);

    // ----------------- SECTION 03: ELEMENTS -----------------

    // Elements Section Header with Add Element ComboBox (Point 3)
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        widgets::hud(ui, loc.overlay_studio_elements_title, t.ink);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut add_elem_choice: Option<&str> = None;
            egui::ComboBox::from_id_salt("add_overlay_element_dropdown")
                .selected_text(RichText::new(loc.overlay_studio_btn_add_element).family(theme::semibold()).color(t.accent_ink))
                .width(150.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, loc.overlay_studio_elem_text).clicked() {
                        add_elem_choice = Some("text");
                    }
                    if ui.selectable_label(false, loc.overlay_studio_elem_crosshair).clicked() {
                        add_elem_choice = Some("crosshair");
                    }
                    if ui.selectable_label(false, loc.overlay_studio_elem_image).clicked() {
                        add_elem_choice = Some("image");
                    }
                    if ui.selectable_label(false, loc.overlay_studio_elem_shape).clicked() {
                        add_elem_choice = Some("shape");
                    }
                });

            if let Some(kind) = add_elem_choice {
                let id = overlay.elements.len() + 1;
                let cand_x = 14.0;
                let cand_y = 12.0 + (overlay.elements.len() as f32 * 22.0);
                match kind {
                    "text" => {
                        let (spawn_x, spawn_y) = if cand_x + 80.0 > overlay.rect.width || cand_y + 20.0 > overlay.rect.height {
                            (0.0, 0.0)
                        } else {
                            (cand_x, cand_y)
                        };
                        overlay.elements.push(OverlayElement::Text {
                            id: format!("text_{id}"),
                            text: "New Text".to_string(),
                            font_size: 13.0,
                            color: [240, 240, 250, 255],
                            x: spawn_x,
                            y: spawn_y,
                            width: None,
                            height: None,
                            lock_aspect: false,
                            bold: false,
                            font_family: "Segoe UI".to_string(),
                            rotation: 0.0,
                            on_click: None,
                        });
                    }
                    "crosshair" => {
                        let s = 14.0;
                        let gap = 4.0;
                        let ew = (s + gap) * 2.0;
                        let (spawn_x, spawn_y) = if (overlay.rect.width / 2.0) + (ew / 2.0) > overlay.rect.width
                            || (overlay.rect.height / 2.0) + (ew / 2.0) > overlay.rect.height
                        {
                            (0.0, 0.0)
                        } else {
                            ((overlay.rect.width / 2.0).round(), (overlay.rect.height / 2.0).round())
                        };
                        overlay.elements.push(OverlayElement::Crosshair {
                            id: format!("cross_{id}"),
                            style: CrosshairStyle::Cross,
                            size: s,
                            color: [0, 255, 136, 255],
                            center_dot: true,
                            thickness: 2.0,
                            gap,
                            x: spawn_x,
                            y: spawn_y,
                            rotation: 0.0,
                        });
                    }
                    "image" => {
                        let ew = 32.0;
                        let eh = 32.0;
                        let (spawn_x, spawn_y) = if cand_x + ew > overlay.rect.width || cand_y + eh > overlay.rect.height {
                            (0.0, 0.0)
                        } else {
                            (cand_x, cand_y)
                        };
                        overlay.elements.push(OverlayElement::Image {
                            id: format!("img_{id}"),
                            path: String::new(),
                            x: spawn_x,
                            y: spawn_y,
                            width: ew,
                            height: eh,
                            opacity: 1.0,
                            lock_aspect: true,
                            rotation: 0.0,
                            on_click: None,
                        });
                    }
                    "shape" => {
                        let ew = 48.0;
                        let eh = 48.0;
                        let (spawn_x, spawn_y) = if cand_x + ew > overlay.rect.width || cand_y + eh > overlay.rect.height {
                            (0.0, 0.0)
                        } else {
                            (cand_x, cand_y)
                        };
                        overlay.elements.push(OverlayElement::Shape {
                            id: format!("shape_{id}"),
                            shape_type: ShapeKind::Rectangle,
                            x: spawn_x,
                            y: spawn_y,
                            width: ew,
                            height: eh,
                            thickness: 2.0,
                            color: [80, 160, 255, 255],
                            filled: false,
                            fill_color: [80, 160, 255, 60],
                            corner_radius: 4.0,
                            lock_aspect: true,
                            rotation: 0.0,
                            on_click: None,
                        });
                    }
                    _ => {}
                }
                clamp_elements_to_canvas(overlay);
                *selected_idx = Some(overlay.elements.len() - 1);
            }
        });
    });

    ui.add_space(4.0);

    let dnd_id = ui.id().with("overlay_element_dnd");
    let mut dnd_session: Option<ElementDndSession> = ui.ctx().data(|d| d.get_temp(dnd_id));
    let pointer_pos = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));
    let pointer_released = ui.input(|i| i.pointer.any_released());

    // Cancel on Escape
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        dnd_session = None;
        ui.ctx().data_mut(|d| d.remove_temp::<ElementDndSession>(dnd_id));
    }

    let mut remove_idx = None;
    let mut clone_idx = None;
    let mut drag_started_idx = None;
    let mut card_centers = Vec::new();

    // Edge auto-scrolling while dragging elements
    if dnd_session.is_some() {
        let wheel_delta_y = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel_delta_y != 0.0 {
            ui.scroll_with_delta(vec2(0.0, wheel_delta_y * 1.5));
            ui.ctx().request_repaint();
        }

        if let Some(pointer) = pointer_pos {
            let viewport = ui.clip_rect();
            let edge_zone = 70.0;
            let view_top = viewport.top();
            let view_bottom = viewport.bottom();

            if pointer.y < view_top + edge_zone {
                let factor = ((view_top + edge_zone - pointer.y) / edge_zone).clamp(0.1, 3.0);
                let scroll_speed = 8.0 + 26.0 * factor;
                ui.scroll_with_delta(vec2(0.0, scroll_speed));
                ui.ctx().request_repaint();
            } else if pointer.y > view_bottom - edge_zone {
                let factor = ((pointer.y - (view_bottom - edge_zone)) / edge_zone).clamp(0.1, 3.0);
                let scroll_speed = 8.0 + 26.0 * factor;
                ui.scroll_with_delta(vec2(0.0, -scroll_speed));
                ui.ctx().request_repaint();
            }
        }
    }

    let active_src = dnd_session.map(|s| s.source_idx);
    let active_dst = dnd_session.map(|s| s.target_idx);

    let total_elems = overlay.elements.len();
    if total_elems == 0 {
        ui.add_space(8.0);
        egui::Frame::new()
            .fill(t.sunken)
            .stroke(t.hairline())
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_max_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(loc.overlay_studio_no_elements)
                                .font(FontId::proportional(11.5))
                                .color(t.faint),
                        )
                        .wrap()
                        .selectable(false),
                    );
                });
            });
        ui.add_space(8.0);
    }
    for idx in 0..total_elems {
        // Drop slot indicator before each item
        render_elem_drop_slot(ui, idx, active_dst, active_src, total_elems, t);

        let is_selected = *selected_idx == Some(idx);
        let frame_stroke = if is_selected {
            Stroke::new(1.5, t.accent_ink)
        } else {
            t.hairline()
        };
        let frame_fill = if is_selected {
            t.sunken.gamma_multiply(1.5)
        } else {
            t.sunken
        };

        let card_resp = egui::Frame::new()
            .fill(frame_fill)
            .stroke(frame_stroke)
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;

                let elem = &mut overlay.elements[idx];

                // Header row of the element card: Drag Grip + Clean Badge Label + Clone / Remove
                ui.horizontal(|ui| {
                    let grip_resp = widgets::drag_handle(ui, loc.inspector_tip_drag_step);
                    if grip_resp.drag_started() {
                        drag_started_idx = Some(idx);
                    }

                    let (badge_text, badge_color) = match elem {
                        OverlayElement::Crosshair { .. } => ("CROSSHAIR", t.accent_ink),
                        OverlayElement::Text { .. } => ("TEXT", t.ink),
                        OverlayElement::Badge { .. } => ("BADGE", t.good),
                        OverlayElement::ProgressBar { .. } => ("PROGRESS", t.warn),
                        OverlayElement::Image { .. } => ("IMAGE", t.faint),
                        OverlayElement::Shape { shape_type, .. } => (
                            match shape_type {
                                ShapeKind::Rectangle => "SQUARE",
                                ShapeKind::Circle => "CIRCLE",
                                ShapeKind::Arrow => "ARROW",
                                ShapeKind::Line => "LINE",
                            },
                            t.accent_ink,
                        ),
                    };

                    // Clean badge label without selectable_label background highlight
                    let badge_resp = ui.add(
                        egui::Label::new(
                            RichText::new(badge_text)
                                .family(theme::semibold())
                                .size(10.5)
                                .color(badge_color)
                        ).sense(Sense::click())
                    ).on_hover_cursor(egui::CursorIcon::PointingHand);

                    if badge_resp.clicked() || grip_resp.clicked() {
                        *selected_idx = Some(idx);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::glyph_button(ui, Glyph::Close, loc.inspector_tip_remove_step).clicked() {
                            remove_idx = Some(idx);
                        }
                        if widgets::glyph_button(ui, Glyph::Clone, loc.inspector_tip_clone_step).clicked() {
                            clone_idx = Some(idx);
                        }
                    });
                });

                // Body rows of the element card
                match elem {
                    OverlayElement::Crosshair { style, size, color, thickness, gap, center_dot, x, y, rotation, .. } => {
                        let styles = [
                            (CrosshairStyle::Cross, "Cross"),
                            (CrosshairStyle::TShaped, "T-Shape"),
                            (CrosshairStyle::Circle, "Circle"),
                            (CrosshairStyle::Dot, "Dot"),
                        ];
                        ui.horizontal(|ui| {
                            for (s, lbl) in styles {
                                let is_s = *style == s;
                                let kind = if is_s { Kind::Primary } else { Kind::Secondary };
                                if widgets::button(ui, lbl, kind, 0.0).clicked() {
                                    *style = s;
                                }
                            }
                        });

                        render_color_chip(ui, "COLOR:", color, ColorPickerTarget::ElementPrimary(idx), active_color_target, t);
                        if *active_color_target == Some(ColorPickerTarget::ElementPrimary(idx)) {
                            render_inline_color_editor(ui, color, false, t);
                        }

                        ui.columns(3, |cols| {
                            cols[0].add(egui::DragValue::new(size).range(4.0..=128.0).prefix("Size: "));
                            cols[1].add(egui::DragValue::new(thickness).range(1.0..=8.0).prefix("Thick: "));
                            cols[2].add(egui::DragValue::new(gap).range(0.0..=32.0).prefix("Gap: "));
                        });
                        ui.checkbox(center_dot, loc.overlay_studio_crosshair_center_dot);

                        render_rotation_control(ui, rotation, t, loc);

                        ui.columns(2, |cols| {
                            cols[0].add(egui::DragValue::new(x).prefix("X: ").suffix(" px"));
                            cols[1].add(egui::DragValue::new(y).prefix("Y: ").suffix(" px"));
                        });
                    }
                    OverlayElement::Text {
                        text,
                        font_size,
                        color,
                        x,
                        y,
                        bold,
                        font_family,
                        rotation,
                        on_click,
                        ..
                    } => {
                        ui.add(
                            egui::TextEdit::singleline(text)
                                .desired_width(ui.available_width())
                                .hint_text(loc.overlay_studio_hint_text)
                        );

                        // Font Family ComboBox
                        let font_opts = [
                            "Inter",
                            "Archivo",
                            "Geist Mono",
                            "Segoe UI",
                            "Consolas",
                            "Arial",
                            "Impact",
                            "Tahoma",
                            "Verdana",
                            "Comic Sans MS",
                            "Trebuchet MS",
                            "Georgia",
                            "Courier New",
                        ];
                        egui::ComboBox::from_id_salt(ui.id().with(("font_combo", idx)))
                            .selected_text(font_family.as_str())
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for f in font_opts {
                                    ui.selectable_value(font_family, f.to_string(), f);
                                }
                            });

                        render_color_chip(ui, "TEXT COLOR:", color, ColorPickerTarget::ElementPrimary(idx), active_color_target, t);
                        if *active_color_target == Some(ColorPickerTarget::ElementPrimary(idx)) {
                            render_inline_color_editor(ui, color, true, t);
                        }

                        ui.columns(2, |cols| {
                            cols[0].add(egui::DragValue::new(font_size).range(8.0..=72.0).prefix("Size: ").suffix(" pt"));
                            cols[1].checkbox(bold, loc.overlay_studio_text_bold);
                        });

                        render_rotation_control(ui, rotation, t, loc);

                        let char_len = (text.chars().count() as f32).max(1.0);
                        let nat_w = (char_len * (*font_size * 0.6).max(8.0) + 12.0).max(30.0);
                        let nat_h = (*font_size + 6.0).max(18.0);
                        ui.columns(2, |cols| {
                            let max_x = (overlay.rect.width - nat_w).max(0.0);
                            let max_y = (overlay.rect.height - nat_h).max(0.0);
                            cols[0].add(egui::DragValue::new(x).range(0.0..=max_x).prefix("X: ").suffix(" px"));
                            cols[1].add(egui::DragValue::new(y).range(0.0..=max_y).prefix("Y: ").suffix(" px"));
                        });

                        // On-Click Action
                        if !overlay.click_through {
                            render_element_click_action_ui(ui, on_click, rules_list, idx, t, loc);
                        }
                    }
                    OverlayElement::Badge { text, color, bg_color, x, y, rotation, on_click, .. } => {
                        ui.add(
                            egui::TextEdit::singleline(text)
                                .desired_width(ui.available_width())
                                .hint_text(loc.overlay_studio_hint_badge)
                        );

                        ui.columns(2, |cols| {
                            render_color_chip(&mut cols[0], "TEXT:", color, ColorPickerTarget::ElementPrimary(idx), active_color_target, t);
                            render_color_chip(&mut cols[1], "BG:", bg_color, ColorPickerTarget::ElementSecondary(idx), active_color_target, t);
                        });
                        if *active_color_target == Some(ColorPickerTarget::ElementPrimary(idx)) {
                            render_inline_color_editor(ui, color, true, t);
                        } else if *active_color_target == Some(ColorPickerTarget::ElementSecondary(idx)) {
                            render_inline_color_editor(ui, bg_color, true, t);
                        }

                        render_rotation_control(ui, rotation, t, loc);

                        ui.columns(2, |cols| {
                            cols[0].add(egui::DragValue::new(x).prefix("X: ").suffix(" px"));
                            cols[1].add(egui::DragValue::new(y).prefix("Y: ").suffix(" px"));
                        });

                        if !overlay.click_through {
                            render_element_click_action_ui(ui, on_click, rules_list, idx, t, loc);
                        }
                    }
                    OverlayElement::ProgressBar { progress, fill_color, bg_color, x, y, width, height, rotation, .. } => {
                        ui.add(
                            egui::Slider::new(progress, 0.0..=1.0)
                                .text(loc.overlay_studio_elem_progress_fill)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                        );

                        ui.columns(2, |cols| {
                            render_color_chip(&mut cols[0], "FILL:", fill_color, ColorPickerTarget::ElementPrimary(idx), active_color_target, t);
                            render_color_chip(&mut cols[1], "BG:", bg_color, ColorPickerTarget::ElementSecondary(idx), active_color_target, t);
                        });
                        if *active_color_target == Some(ColorPickerTarget::ElementPrimary(idx)) {
                            render_inline_color_editor(ui, fill_color, true, t);
                        } else if *active_color_target == Some(ColorPickerTarget::ElementSecondary(idx)) {
                            render_inline_color_editor(ui, bg_color, true, t);
                        }

                        ui.columns(2, |cols| {
                            cols[0].add(egui::DragValue::new(width).range(20.0..=1920.0).prefix("Width: ").suffix(" px"));
                            cols[1].add(egui::DragValue::new(height).range(2.0..=120.0).prefix("Height: ").suffix(" px"));
                        });

                        render_rotation_control(ui, rotation, t, loc);

                        ui.columns(2, |cols| {
                            cols[0].add(egui::DragValue::new(x).prefix("X: ").suffix(" px"));
                            cols[1].add(egui::DragValue::new(y).prefix("Y: ").suffix(" px"));
                        });
                    }
                    OverlayElement::Image { path, x, y, width, height, opacity, lock_aspect, rotation, on_click, .. } => {
                        ui.horizontal(|ui| {
                            if widgets::button(ui, loc.overlay_studio_btn_browse, Kind::Secondary, 0.0).clicked() {
                                if let Some(picked) = rfd::FileDialog::new()
                                    .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "ico", "webp"])
                                    .pick_file()
                                {
                                    *path = picked.to_string_lossy().to_string();
                                    if let Ok(img) = image::open(&picked) {
                                        let (nw, nh) = (img.width() as f32, img.height() as f32);
                                        if nw > 0.0 && nh > 0.0 {
                                            let max_w = (overlay.rect.width - 16.0).max(8.0);
                                            let max_h = (overlay.rect.height - 16.0).max(8.0);
                                            if nw > max_w || nh > max_h {
                                                let scale_factor = (max_w / nw).min(max_h / nh);
                                                *width = (nw * scale_factor).round().max(8.0);
                                                *height = (nh * scale_factor).round().max(8.0);
                                            } else {
                                                *width = nw;
                                                *height = nh;
                                            }
                                        }
                                    }
                                }
                            }
                            ui.add(
                                egui::TextEdit::singleline(path)
                                    .desired_width(ui.available_width())
                                    .min_size(vec2(0.0, theme::CONTROL_H))
                                    .margin(vec2(8.0, 5.0))
                                    .hint_text(loc.overlay_studio_hint_img_path),
                            );
                        });

                        // Width & Height with Aspect Ratio Lock
                        ui.horizontal(|ui| {
                            let old_w = *width;
                            let old_h = *height;
                            let aspect = if old_h > 0.0 { old_w / old_h } else { 1.0 };

                            let total_w = ui.available_width();
                            let btn_w = theme::CONTROL_H;
                            let spacing = ui.spacing().item_spacing.x;
                            let field_w = ((total_w - btn_w - spacing * 2.0 - 4.0) / 2.0).max(40.0).floor();
                            let w_resp = ui.add_sized(
                                vec2(field_w, theme::CONTROL_H),
                                egui::DragValue::new(width).range(8.0..=overlay.rect.width).prefix("W: ").suffix(" px").speed(1.0),
                            );
                            let h_resp = ui.add_sized(
                                vec2(field_w, theme::CONTROL_H),
                                egui::DragValue::new(height).range(8.0..=overlay.rect.height).prefix("H: ").suffix(" px").speed(1.0),
                            );

                            if *lock_aspect {
                                if w_resp.changed() && aspect > 0.0 {
                                    *height = (*width / aspect).round().clamp(8.0, overlay.rect.height);
                                } else if h_resp.changed() {
                                    *width = (*height * aspect).round().clamp(8.0, overlay.rect.width);
                                }
                            }

                            let lock_glyph = if *lock_aspect {
                                egui_phosphor::regular::LOCK
                            } else {
                                egui_phosphor::regular::LOCK_OPEN
                            };
                            let lock_color = if *lock_aspect { t.accent_ink } else { t.faint };
                            let lock_tip = if *lock_aspect {
                                loc.overlay_studio_img_lock_aspect
                            } else {
                                loc.overlay_studio_img_unlock_aspect
                            };
                            let (lock_rect, lock_resp) = ui.allocate_exact_size(vec2(btn_w, theme::CONTROL_H), Sense::click());
                            if *lock_aspect {
                                ui.painter().rect_filled(lock_rect, CornerRadius::same(3), t.accent_ink.gamma_multiply(0.2));
                            } else if lock_resp.hovered() {
                                ui.painter().rect_filled(lock_rect, CornerRadius::same(3), t.sunken);
                            }
                            ui.painter().rect_stroke(lock_rect, CornerRadius::same(3), t.hairline(), egui::StrokeKind::Inside);
                            ui.painter().text(
                                lock_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                lock_glyph,
                                FontId::new(13.0, theme::phosphor()),
                                lock_color,
                            );
                            if lock_resp.on_hover_text(lock_tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                *lock_aspect = !*lock_aspect;
                            }
                        });

                        ui.add(
                            egui::Slider::new(opacity, 0.0..=1.0)
                                .text(loc.overlay_studio_elem_img_opacity)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                        );

                        render_rotation_control(ui, rotation, t, loc);

                        ui.columns(2, |cols| {
                            let max_x = (overlay.rect.width - *width).max(0.0);
                            let max_y = (overlay.rect.height - *height).max(0.0);
                            cols[0].add(egui::DragValue::new(x).range(0.0..=max_x).prefix("X: ").suffix(" px"));
                            cols[1].add(egui::DragValue::new(y).range(0.0..=max_y).prefix("Y: ").suffix(" px"));
                        });

                        if !overlay.click_through {
                            render_element_click_action_ui(ui, on_click, rules_list, idx, t, loc);
                        }
                    }
                    OverlayElement::Shape {
                        shape_type,
                        x,
                        y,
                        width,
                        height,
                        thickness,
                        color,
                        filled,
                        fill_color,
                        corner_radius,
                        lock_aspect,
                        rotation,
                        on_click,
                        ..
                    } => {
                        let shapes = [
                            (ShapeKind::Rectangle, loc.overlay_studio_shape_rect),
                            (ShapeKind::Circle, loc.overlay_studio_shape_circle),
                            (ShapeKind::Arrow, loc.overlay_studio_shape_arrow),
                            (ShapeKind::Line, loc.overlay_studio_shape_line),
                        ];
                        ui.horizontal(|ui| {
                            for (s, lbl) in shapes {
                                let is_s = *shape_type == s;
                                let kind = if is_s { Kind::Primary } else { Kind::Secondary };
                                if widgets::button(ui, lbl, kind, 0.0).clicked() {
                                    *shape_type = s;
                                }
                            }
                        });

                        let is_fillable = *shape_type == ShapeKind::Rectangle || *shape_type == ShapeKind::Circle;
                        if is_fillable {
                            ui.columns(2, |cols| {
                                render_color_chip(&mut cols[0], "COLOR:", color, ColorPickerTarget::ElementPrimary(idx), active_color_target, t);
                                if *filled {
                                    render_color_chip(&mut cols[1], "FILL:", fill_color, ColorPickerTarget::ElementSecondary(idx), active_color_target, t);
                                } else {
                                    cols[1].checkbox(filled, loc.overlay_studio_shape_fill);
                                }
                            });
                            if *filled {
                                ui.checkbox(filled, loc.overlay_studio_shape_fill);
                            }
                        } else {
                            render_color_chip(ui, "COLOR:", color, ColorPickerTarget::ElementPrimary(idx), active_color_target, t);
                        }

                        if *active_color_target == Some(ColorPickerTarget::ElementPrimary(idx)) {
                            render_inline_color_editor(ui, color, true, t);
                        } else if *active_color_target == Some(ColorPickerTarget::ElementSecondary(idx)) && is_fillable {
                            render_inline_color_editor(ui, fill_color, true, t);
                        }

                        let can_round = *shape_type == ShapeKind::Rectangle || *shape_type == ShapeKind::Line;
                        if can_round {
                            ui.columns(2, |cols| {
                                cols[0].add(egui::DragValue::new(thickness).range(1.0..=32.0).prefix("Thick: ").suffix(" px"));
                                cols[1].add(egui::DragValue::new(corner_radius).range(0.0..=64.0).prefix("Radius: ").suffix(" px"));
                            });
                        } else {
                            ui.add(egui::DragValue::new(thickness).range(1.0..=32.0).prefix("Thick: ").suffix(" px"));
                        }

                        ui.horizontal(|ui| {
                            let old_w = *width;
                            let old_h = *height;
                            let aspect = if old_h > 0.0 { old_w / old_h } else { 1.0 };

                            let total_w = ui.available_width();
                            let btn_w = theme::CONTROL_H;
                            let spacing = ui.spacing().item_spacing.x;
                            let field_w = ((total_w - btn_w - spacing * 2.0 - 4.0) / 2.0).max(40.0).floor();
                            let w_resp = ui.add_sized(
                                vec2(field_w, theme::CONTROL_H),
                                egui::DragValue::new(width).range(4.0..=overlay.rect.width).prefix("W: ").suffix(" px").speed(1.0),
                            );
                            let h_resp = ui.add_sized(
                                vec2(field_w, theme::CONTROL_H),
                                egui::DragValue::new(height).range(4.0..=overlay.rect.height).prefix("H: ").suffix(" px").speed(1.0),
                            );

                            if *lock_aspect {
                                if w_resp.changed() && aspect > 0.0 {
                                    *height = (*width / aspect).round().clamp(4.0, overlay.rect.height);
                                } else if h_resp.changed() {
                                    *width = (*height * aspect).round().clamp(4.0, overlay.rect.width);
                                }
                            }

                            let lock_glyph = if *lock_aspect {
                                egui_phosphor::regular::LOCK
                            } else {
                                egui_phosphor::regular::LOCK_OPEN
                            };
                            let lock_color = if *lock_aspect { t.accent_ink } else { t.faint };
                            let lock_tip = if *lock_aspect {
                                loc.overlay_studio_img_lock_aspect
                            } else {
                                loc.overlay_studio_img_unlock_aspect
                            };
                            let (lock_rect, lock_resp) = ui.allocate_exact_size(vec2(btn_w, theme::CONTROL_H), Sense::click());
                            if *lock_aspect {
                                ui.painter().rect_filled(lock_rect, CornerRadius::same(3), t.accent_ink.gamma_multiply(0.2));
                            } else if lock_resp.hovered() {
                                ui.painter().rect_filled(lock_rect, CornerRadius::same(3), t.sunken);
                            }
                            ui.painter().rect_stroke(lock_rect, CornerRadius::same(3), t.hairline(), egui::StrokeKind::Inside);
                            ui.painter().text(
                                lock_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                lock_glyph,
                                FontId::new(13.0, theme::phosphor()),
                                lock_color,
                            );
                            if lock_resp.on_hover_text(lock_tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                *lock_aspect = !*lock_aspect;
                            }
                        });

                        render_rotation_control(ui, rotation, t, loc);

                        ui.columns(2, |cols| {
                            let max_x = (overlay.rect.width - *width).max(0.0);
                            let max_y = (overlay.rect.height - *height).max(0.0);
                            cols[0].add(egui::DragValue::new(x).range(0.0..=max_x).prefix("X: ").suffix(" px"));
                            cols[1].add(egui::DragValue::new(y).range(0.0..=max_y).prefix("Y: ").suffix(" px"));
                        });

                        if !overlay.click_through {
                            render_element_click_action_ui(ui, on_click, rules_list, idx, t, loc);
                        }
                    }
                }
            });

        card_centers.push(card_resp.response.rect.center().y);
        ui.add_space(4.0);
    }

    // Determine target slot while dragging using fresh centers from this frame
    if let (Some(session), Some(pointer)) = (&mut dnd_session, pointer_pos) {
        let n = card_centers.len();
        if n > 0 {
            if pointer.y < card_centers[0] {
                session.target_idx = 0;
            } else if pointer.y >= *card_centers.last().unwrap() {
                session.target_idx = n;
            } else {
                for i in 0..n - 1 {
                    if pointer.y >= card_centers[i] && pointer.y < card_centers[i + 1] {
                        session.target_idx = i + 1;
                        break;
                    }
                }
            }
        }
    }

    let active_dst = dnd_session.as_ref().map(|s| s.target_idx).or(active_dst);

    // Trailing drop slot after all elements
    if total_elems > 0 {
        render_elem_drop_slot(ui, total_elems, active_dst, active_src, total_elems, t);
    }

    // Commit reorder on mouse release
    if pointer_released {
        if let Some(session) = dnd_session {
            let src = session.source_idx;
            let dst = session.target_idx;
            if src < overlay.elements.len() && dst <= overlay.elements.len() {
                if dst < src {
                    let item = overlay.elements.remove(src);
                    overlay.elements.insert(dst, item);
                    *selected_idx = Some(dst);
                } else if dst > src + 1 {
                    let item = overlay.elements.remove(src);
                    overlay.elements.insert(dst - 1, item);
                    *selected_idx = Some(dst - 1);
                }
            }
            dnd_session = None;
            ui.ctx().data_mut(|d| d.remove_temp::<ElementDndSession>(dnd_id));
        }
    }

    if let Some(src_idx) = drag_started_idx {
        dnd_session = Some(ElementDndSession {
            source_idx: src_idx,
            target_idx: src_idx,
        });
    }

    if let Some(session) = dnd_session {
        ui.ctx().data_mut(|d| d.insert_temp(dnd_id, session));
        ui.ctx().request_repaint();
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

        // Floating drag preview badge following the pointer
        if let Some(pointer) = pointer_pos {
            if session.source_idx < overlay.elements.len() {
                let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("elem_drag_float")));
                let (badge_text, _) = match &overlay.elements[session.source_idx] {
                    OverlayElement::Crosshair { .. } => ("CROSSHAIR", t.accent_ink),
                    OverlayElement::Text { .. } => ("TEXT", t.ink),
                    OverlayElement::Badge { .. } => ("BADGE", t.good),
                    OverlayElement::ProgressBar { .. } => ("PROGRESS", t.warn),
                    OverlayElement::Image { .. } => ("IMAGE", t.faint),
                    OverlayElement::Shape { shape_type, .. } => (
                        match shape_type {
                            ShapeKind::Rectangle => "SQUARE",
                            ShapeKind::Circle => "CIRCLE",
                            ShapeKind::Arrow => "ARROW",
                            ShapeKind::Line => "LINE",
                        },
                        t.accent_ink,
                    ),
                };
                let float_rect = Rect::from_min_size(pointer + vec2(12.0, 10.0), vec2(110.0, 26.0));
                p.rect_filled(float_rect, CornerRadius::same(4), t.bg);
                p.rect_stroke(float_rect, CornerRadius::same(4), Stroke::new(1.0, t.accent_ink), egui::StrokeKind::Inside);
                p.text(
                    float_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    badge_text,
                    FontId::new(11.0, theme::semibold()),
                    t.ink,
                );
            }
        }
    }

    // Handle cloning
    if let Some(idx) = clone_idx {
        if idx < overlay.elements.len() {
            let mut cloned = overlay.elements[idx].clone();
            let (ew, eh) = get_element_size(&cloned);
            match &mut cloned {
                OverlayElement::Text { id, x, y, .. }
                | OverlayElement::Badge { id, x, y, .. }
                | OverlayElement::ProgressBar { id, x, y, .. }
                | OverlayElement::Image { id, x, y, .. }
                | OverlayElement::Shape { id, x, y, .. }
                | OverlayElement::Crosshair { id, x, y, .. } => {
                    *id = format!("{}_copy", id);
                    let cand_x = *x + 16.0;
                    let cand_y = *y + 16.0;
                    if cand_x + ew > overlay.rect.width || cand_y + eh > overlay.rect.height {
                        *x = 0.0;
                        *y = 0.0;
                    } else {
                        *x = cand_x;
                        *y = cand_y;
                    }
                }
            }
            overlay.elements.insert(idx + 1, cloned);
            clamp_elements_to_canvas(overlay);
            *selected_idx = Some(idx + 1);
        }
    }

    // Handle removal
    if let Some(r_idx) = remove_idx {
        if r_idx < overlay.elements.len() {
            overlay.elements.remove(r_idx);
            if *selected_idx == Some(r_idx) {
                *selected_idx = None;
            } else if let Some(sel) = *selected_idx {
                if sel > r_idx {
                    *selected_idx = Some(sel - 1);
                }
            }
        }
    }

    ui.add_space(4.0);
}

// ----------------- ELEMENT ON-CLICK ACTION UI (POINT 7) -----------------
fn render_element_click_action_ui(
    ui: &mut Ui,
    on_click: &mut Option<OverlayClickAction>,
    rules_list: &[(String, String)],
    idx: usize,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(loc.overlay_studio_on_click).size(10.0).family(theme::semibold()).color(t.faint));

        let current_lbl: String = match on_click {
            None => loc.overlay_studio_on_click_none.to_string(),
            Some(OverlayClickAction::CloseOverlay) => loc.overlay_studio_on_click_close.to_string(),
            Some(OverlayClickAction::TriggerRule { rule_id } | OverlayClickAction::ToggleRule { rule_id }) => {
                let name = rules_list.iter().find(|(id, _)| id == rule_id).map(|(_, n)| n.as_str()).unwrap_or(rule_id);
                format!("{}: {name}", loc.overlay_studio_on_click_trigger)
            }
        };

        egui::ComboBox::from_id_salt(ui.id().with(("elem_onclick", idx)))
            .selected_text(current_lbl)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(on_click.is_none(), loc.overlay_studio_on_click_none).clicked() {
                    *on_click = None;
                }
                if ui.selectable_label(matches!(on_click, Some(OverlayClickAction::CloseOverlay)), loc.overlay_studio_on_click_close).clicked() {
                    *on_click = Some(OverlayClickAction::CloseOverlay);
                }
                if !rules_list.is_empty() {
                    ui.separator();
                    for (r_id, r_name) in rules_list {
                        let is_run = matches!(on_click, Some(OverlayClickAction::TriggerRule { rule_id } | OverlayClickAction::ToggleRule { rule_id }) if rule_id == r_id);
                        if ui.selectable_label(is_run, format!("{}: {r_name}", loc.overlay_studio_on_click_trigger)).clicked() {
                            *on_click = Some(OverlayClickAction::TriggerRule { rule_id: r_id.clone() });
                        }
                    }
                }
            });
    });
}

// ----------------- INTERACTIVE VISUAL PREVIEW CANVAS (POINT 6) -----------------
#[derive(Debug, Clone, Copy, PartialEq)]
enum CanvasDragMode {
    None,
    Pan {
        start_pointer: egui::Pos2,
        start_pan: egui::Vec2,
    },
    MoveCard {
        start_pointer: egui::Pos2,
        start_card_sx: f32,
        start_card_sy: f32,
    },
    ResizeCard {
        start_pointer: egui::Pos2,
        start_w: f32,
        start_h: f32,
    },
    Move {
        idx: usize,
        start_x: f32,
        start_y: f32,
        start_pointer: egui::Pos2,
    },
    Resize {
        idx: usize,
        start_w: f32,
        start_h: f32,
        start_pointer: egui::Pos2,
    },
    Rotate {
        idx: usize,
        center: egui::Pos2,
        base_angle_rad: f32,
    },
}

#[cfg(windows)]
fn get_screen_resolution() -> (f32, f32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if w > 0 && h > 0 {
        (w as f32, h as f32)
    } else {
        (1920.0, 1080.0)
    }
}

#[cfg(not(windows))]
fn get_screen_resolution() -> (f32, f32) {
    (1920.0, 1080.0)
}

pub fn compute_overlay_screen_pos_with_anchor(anchor: OverlayAnchor, overlay: &OverlayDefinition, screen_w: f32, screen_h: f32) -> (f32, f32) {
    let w = overlay.rect.width;
    let h = overlay.rect.height;
    let mut rx = overlay.rect.x;
    let mut ry = overlay.rect.y;
    match anchor {
        OverlayAnchor::ScreenCenter => {
            if rx.abs() > screen_w * 0.35 { rx = 0.0; }
            if ry.abs() > screen_h * 0.35 { ry = 0.0; }
            ((screen_w - w) * 0.5 + rx, (screen_h - h) * 0.5 + ry)
        }
        OverlayAnchor::TopLeft => {
            if rx > screen_w * 0.4 { rx = 16.0; }
            if ry > screen_h * 0.4 { ry = 16.0; }
            (rx, ry)
        }
        OverlayAnchor::TopRight => {
            if rx > screen_w * 0.4 { rx = 16.0; }
            if ry > screen_h * 0.4 { ry = 16.0; }
            (screen_w - w - rx, ry)
        }
        OverlayAnchor::BottomLeft => {
            if rx > screen_w * 0.4 { rx = 16.0; }
            if ry > screen_h * 0.4 { ry = 16.0; }
            (rx, (screen_h - h - ry).max(0.0))
        }
        OverlayAnchor::BottomRight => {
            if rx > screen_w * 0.4 { rx = 16.0; }
            if ry > screen_h * 0.4 { ry = 16.0; }
            (screen_w - w - rx, (screen_h - h - ry).max(0.0))
        }
        OverlayAnchor::Custom => (rx, ry),
        OverlayAnchor::WindowRelative => (rx, ry),
    }
}

pub fn compute_overlay_screen_pos(overlay: &OverlayDefinition, screen_w: f32, screen_h: f32) -> (f32, f32) {
    compute_overlay_screen_pos_with_anchor(overlay.anchor, overlay, screen_w, screen_h)
}

pub fn max_card_dimensions(overlay: &OverlayDefinition, screen_w: f32, screen_h: f32) -> (f32, f32) {
    let rx = overlay.rect.x.max(0.0);
    let ry = overlay.rect.y.max(0.0);
    match overlay.anchor {
        OverlayAnchor::ScreenCenter => {
            let mw = (screen_w - rx * 2.0).clamp(20.0, screen_w);
            let mh = (screen_h - ry * 2.0).clamp(20.0, screen_h);
            (mw, mh)
        }
        OverlayAnchor::TopLeft | OverlayAnchor::TopRight | OverlayAnchor::BottomLeft | OverlayAnchor::BottomRight => {
            ((screen_w - rx).clamp(20.0, screen_w), (screen_h - ry).clamp(20.0, screen_h))
        }
        OverlayAnchor::Custom => {
            ((screen_w - overlay.rect.x).clamp(20.0, screen_w), (screen_h - overlay.rect.y).clamp(20.0, screen_h))
        }
        OverlayAnchor::WindowRelative => (screen_w, screen_h),
    }
}

pub fn clamp_card_to_monitor(overlay: &mut OverlayDefinition, screen_w: f32, screen_h: f32) {
    let (max_w, max_h) = max_card_dimensions(overlay, screen_w, screen_h);
    overlay.rect.width = overlay.rect.width.clamp(20.0, max_w);
    overlay.rect.height = overlay.rect.height.clamp(20.0, max_h);
    if overlay.anchor == OverlayAnchor::Custom {
        overlay.rect.x = overlay.rect.x.clamp(0.0, (screen_w - overlay.rect.width).max(0.0));
        overlay.rect.y = overlay.rect.y.clamp(0.0, (screen_h - overlay.rect.height).max(0.0));
    }
}

fn render_overlay_canvas(
    ui: &mut Ui,
    studio: &mut OverlayStudioState,
    t: &Tokens,
    loc: &crate::i18n::Strings,
) {
    let (screen_w, screen_h) = get_screen_resolution();
    clamp_card_to_monitor(&mut studio.overlay, screen_w, screen_h);

    let avail_canvas_w = (ui.available_width() - 4.0).max(80.0);
    let avail_canvas_h = (ui.available_height() - 44.0).max(80.0);

    // 1. Calculate default auto-fit zoom for monitor artboard
    let fit_margin = 32.0;
    let fit_scale_x = (avail_canvas_w - fit_margin * 2.0).max(40.0) / screen_w;
    let fit_scale_y = (avail_canvas_h - fit_margin * 2.0).max(40.0) / screen_h;
    let auto_zoom = fit_scale_x.min(fit_scale_y).clamp(0.05, 5.0);

    let cur_zoom = studio.canvas_zoom.unwrap_or(auto_zoom);

    // Header Strip with camera controls and snap toggle
    ui.horizontal(|ui| {
        widgets::hud(ui, loc.overlay_studio_canvas_title, t.faint);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(t.sunken)
                .stroke(t.hairline())
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.horizontal(|ui| {
                        // 1. Snap to Grid Toggle
                        let grid_glyph = egui_phosphor::regular::MAGNET;
                        let grid_color = if studio.snap_to_grid { t.accent_ink } else { t.faint };
                        let (grid_rect, grid_resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                        if studio.snap_to_grid {
                            ui.painter().rect_filled(grid_rect, CornerRadius::same(2), t.accent_ink.gamma_multiply(0.2));
                        } else if grid_resp.hovered() {
                            ui.painter().rect_filled(grid_rect, CornerRadius::same(2), t.bg);
                        }
                        ui.painter().text(
                            grid_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            grid_glyph,
                            FontId::new(12.0, theme::phosphor()),
                            grid_color,
                        );
                        if grid_resp.on_hover_text(loc.overlay_studio_snap_grid).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            studio.snap_to_grid = !studio.snap_to_grid;
                        }

                        // 2. Focus Overlay Card Button
                        let (focus_rect, focus_resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                        if focus_resp.hovered() {
                            ui.painter().rect_filled(focus_rect, CornerRadius::same(2), t.bg);
                        }
                        ui.painter().text(
                            focus_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::TARGET,
                            FontId::new(12.0, theme::phosphor()),
                            t.faint,
                        );
                        if focus_resp.on_hover_text(loc.overlay_studio_focus_card).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            let (csx, csy) = compute_overlay_screen_pos(&studio.overlay, screen_w, screen_h);
                            let focus_margin = 60.0;
                            let f_zoom_x = (avail_canvas_w - focus_margin * 2.0).max(40.0) / studio.overlay.rect.width.max(32.0);
                            let f_zoom_y = (avail_canvas_h - focus_margin * 2.0).max(40.0) / studio.overlay.rect.height.max(32.0);
                            let target_zoom = f_zoom_x.min(f_zoom_y).clamp(0.2, 5.0);
                            let card_center = pos2(csx + studio.overlay.rect.width * 0.5, csy + studio.overlay.rect.height * 0.5);
                            let offset = card_center - pos2(screen_w * 0.5, screen_h * 0.5);
                            studio.canvas_pan = -offset * target_zoom;
                            studio.canvas_zoom = Some(target_zoom);
                        }

                        // 4. Fit Screen Button
                        let (fit_rect, fit_resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                        if fit_resp.hovered() {
                            ui.painter().rect_filled(fit_rect, CornerRadius::same(2), t.bg);
                        }
                        ui.painter().text(
                            fit_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::ARROWS_OUT,
                            FontId::new(12.0, theme::phosphor()),
                            t.faint,
                        );
                        if fit_resp.on_hover_text(loc.overlay_studio_fit_screen).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            studio.canvas_pan = Vec2::ZERO;
                            studio.canvas_zoom = Some(auto_zoom);
                        }

                        // Divider
                        let (div_rect2, _) = ui.allocate_exact_size(vec2(1.0, 14.0), Sense::hover());
                        ui.painter().vline(div_rect2.center().x, div_rect2.y_range(), t.hairline());

                        // 5. Zoom Out (-) Button
                        let (out_rect, out_resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                        if out_resp.hovered() {
                            ui.painter().rect_filled(out_rect, CornerRadius::same(2), t.bg);
                        }
                        ui.painter().text(
                            out_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::MINUS,
                            FontId::new(12.0, theme::phosphor()),
                            t.faint,
                        );
                        if out_resp.on_hover_text(loc.overlay_studio_zoom_out).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            studio.canvas_zoom = Some((cur_zoom / 1.2).clamp(0.08, 10.0));
                        }

                        // 6. Zoom Level Display / Reset to 100%
                        let zoom_pct = (cur_zoom * 100.0).round() as i32;
                        let zoom_str = format!("{zoom_pct}%");
                        let z_text_w = ui.painter().layout_no_wrap(zoom_str.clone(), FontId::new(10.5, theme::mono_bold()), t.ink).size().x + 8.0;
                        let (z_rect, z_resp) = ui.allocate_exact_size(vec2(z_text_w, 18.0), Sense::click());
                        if z_resp.hovered() {
                            ui.painter().rect_filled(z_rect, CornerRadius::same(2), t.bg);
                        }
                        ui.painter().text(
                            z_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &zoom_str,
                            FontId::new(10.5, theme::mono_bold()),
                            t.accent_ink,
                        );
                        if z_resp.on_hover_text(loc.overlay_studio_reset_zoom).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            studio.canvas_zoom = Some(1.0);
                            studio.canvas_pan = Vec2::ZERO;
                        }

                        // 7. Zoom In (+) Button
                        let (in_rect, in_resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                        if in_resp.hovered() {
                            ui.painter().rect_filled(in_rect, CornerRadius::same(2), t.bg);
                        }
                        ui.painter().text(
                            in_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::PLUS,
                            FontId::new(12.0, theme::phosphor()),
                            t.faint,
                        );
                        if in_resp.on_hover_text(loc.overlay_studio_zoom_in).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            studio.canvas_zoom = Some((cur_zoom * 1.2).clamp(0.08, 10.0));
                        }
                    });
                });
        });
    });
    ui.add_space(4.0);

    let canvas_size = vec2(
        (ui.available_width() - 4.0).max(80.0),
        (ui.available_height() - 6.0).max(80.0),
    );
    let (canvas_rect, _resp) = ui.allocate_exact_size(canvas_size, Sense::hover());
    let painter = ui.painter().with_clip_rect(canvas_rect);

    let zoom = studio.canvas_zoom.unwrap_or(auto_zoom);
    let pan = studio.canvas_pan;

    // Monitor Artboard setup
    let monitor_size = vec2(screen_w * zoom, screen_h * zoom);
    let monitor_origin = canvas_rect.center() + pan - monitor_size * 0.5;
    let monitor_rect = Rect::from_min_size(monitor_origin, monitor_size);

    let screen_to_canvas = |sp: Pos2| -> Pos2 {
        monitor_origin + vec2(sp.x * zoom, sp.y * zoom)
    };

    // 2. Draw workspace canvas background (Infinite artboard background)
    painter.rect_filled(canvas_rect, CornerRadius::same(6), Color32::from_rgb(11, 12, 15));
    painter.rect_stroke(canvas_rect, CornerRadius::same(6), t.hairline(), egui::StrokeKind::Inside);

    // Workspace dot grid outside monitor
    let dot_step = 24.0;
    let mut gx = canvas_rect.left() + dot_step;
    while gx < canvas_rect.right() {
        let mut gy = canvas_rect.top() + dot_step;
        while gy < canvas_rect.bottom() {
            if !monitor_rect.contains(pos2(gx, gy)) {
                painter.circle_filled(pos2(gx, gy), 1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 12));
            }
            gy += dot_step;
        }
        gx += dot_step;
    }

    // 3. Draw Monitor Artboard (Drop shadow + monitor surface)
    painter.rect_filled(monitor_rect.expand(4.0), CornerRadius::same(8), Color32::from_rgba_unmultiplied(0, 0, 0, 90));
    painter.rect_filled(monitor_rect, CornerRadius::same(6), Color32::from_rgb(17, 19, 24));
    painter.rect_stroke(monitor_rect, CornerRadius::same(6), Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30)), egui::StrokeKind::Inside);

    // Monitor resolution label
    let mon_label = format!("🖥 SCREEN 1 ({}×{})", screen_w as i32, screen_h as i32);
    let mon_font = FontId::new((10.0f32 * zoom).clamp(8.0, 12.0), theme::mono());
    let mon_label_pos = monitor_rect.min + vec2(8.0, 6.0);
    if monitor_rect.height() > 30.0 {
        painter.text(mon_label_pos, egui::Align2::LEFT_TOP, mon_label, mon_font, Color32::from_rgba_unmultiplied(255, 255, 255, 45));
    }

    // Monitor center crosslines (subtle guide)
    let m_center = monitor_rect.center();
    let center_stroke = if studio.snap_to_grid {
        Stroke::new(1.0, t.accent_ink.gamma_multiply(0.35))
    } else {
        Stroke::new(0.8, Color32::from_rgba_unmultiplied(255, 255, 255, 18))
    };
    painter.vline(m_center.x, monitor_rect.y_range(), center_stroke);
    painter.hline(monitor_rect.x_range(), m_center.y, center_stroke);

    // Monitor grid if snap-to-grid is active
    if studio.snap_to_grid {
        let grid_step = 64.0 * zoom;
        if grid_step >= 12.0 {
            let mut mgx = monitor_rect.left() + grid_step;
            while mgx < monitor_rect.right() - 1.0 {
                painter.vline(mgx, monitor_rect.y_range(), Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 10)));
                mgx += grid_step;
            }
            let mut mgy = monitor_rect.top() + grid_step;
            while mgy < monitor_rect.bottom() - 1.0 {
                painter.hline(monitor_rect.x_range(), mgy, Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 10)));
                mgy += grid_step;
            }
        }
    }

    // 4. Compute Card position and geometry on canvas
    let (card_sx, card_sy) = compute_overlay_screen_pos(&studio.overlay, screen_w, screen_h);
    let card_screen_rect = Rect::from_min_size(pos2(card_sx, card_sy), vec2(studio.overlay.rect.width, studio.overlay.rect.height));
    let card_canvas_min = screen_to_canvas(card_screen_rect.min);
    let card_canvas_size = vec2(studio.overlay.rect.width * zoom, studio.overlay.rect.height * zoom);
    let card_canvas_rect = Rect::from_min_size(card_canvas_min, card_canvas_size);

    let pill_h = 16.0;
    let pill_w = 160.0;
    let pill_top = (card_canvas_rect.top() - pill_h - 3.0).max(canvas_rect.top() + 4.0);
    let pill_rect = Rect::from_min_size(pos2(card_canvas_rect.left(), pill_top), vec2(pill_w, pill_h));

    let card_resize_handle = Rect::from_center_size(card_canvas_rect.max, vec2(14.0, 14.0));

    // Global Opacity Helper
    let global_opacity = studio.overlay.opacity.clamp(0.0, 1.0);
    let apply_opacity = move |rgba: [u8; 4]| -> Color32 {
        let a = ((rgba[3] as f32) * global_opacity).round() as u8;
        Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], a)
    };

    // Calculate bounding boxes for all elements on the canvas (using rotated AABB)
    let elem_rects: Vec<(usize, Rect)> = studio.overlay.elements.iter().enumerate().map(|(i, elem)| {
        let (raw_w, raw_h) = get_element_size(elem);
        let ew = (raw_w * zoom).max(4.0);
        let eh = (raw_h * zoom).max(4.0);
        let angle_rad = elem.rotation().to_radians();
        let center = if matches!(elem, OverlayElement::Crosshair { .. }) {
            if elem.x() > 0.0 && elem.y() > 0.0 {
                card_canvas_rect.min + vec2(elem.x() * zoom, elem.y() * zoom)
            } else {
                card_canvas_rect.center()
            }
        } else {
            card_canvas_rect.min + vec2((elem.x() + raw_w * 0.5) * zoom, (elem.y() + raw_h * 0.5) * zoom)
        };
        (i, get_rotated_aabb(center, ew * 0.5, eh * 0.5, angle_rad))
    }).collect();

    let drag_mode_id = ui.id().with("canvas_drag_mode");
    let mut drag_mode: CanvasDragMode = ui.ctx().data(|d| d.get_temp(drag_mode_id)).unwrap_or(CanvasDragMode::None);

    let selected_handles = studio.selected_element_idx.and_then(|sel| {
        if sel < studio.overlay.elements.len() {
            let elem = &studio.overlay.elements[sel];
            let (raw_w, raw_h) = get_element_size(elem);
            let ew = (raw_w * zoom).max(4.0);
            let eh = (raw_h * zoom).max(4.0);
            let angle_rad = elem.rotation().to_radians();
            let center = if matches!(elem, OverlayElement::Crosshair { .. }) {
                if elem.x() > 0.0 && elem.y() > 0.0 {
                    card_canvas_rect.min + vec2(elem.x() * zoom, elem.y() * zoom)
                } else {
                    card_canvas_rect.center()
                }
            } else {
                card_canvas_rect.min + vec2((elem.x() + raw_w * 0.5) * zoom, (elem.y() + raw_h * 0.5) * zoom)
            };
            let half_w = ew * 0.5;
            let half_h = eh * 0.5;
            let p0 = rotate_point(center + vec2(-half_w - 3.0, -half_h - 3.0), center, angle_rad);
            let p2 = rotate_point(center + vec2(half_w + 3.0, half_h + 3.0), center, angle_rad);
            let rot_rect = Rect::from_center_size(p0, vec2(16.0, 16.0));
            let resize_rect = Rect::from_center_size(p2, vec2(16.0, 16.0));
            Some((rot_rect, resize_rect, center, half_w, half_h))
        } else {
            None
        }
    });

    // ---------------- CANVAS DIRECT MOUSE DRAG & SELECTION & RESIZING / ROTATING ----------------
    let canvas_sense_resp = ui.interact(canvas_rect, ui.id().with("canvas_mouse_sense"), Sense::click_and_drag());
    let pointer_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.interact_pos()));

    let is_space_down = ui.input(|i| i.key_down(egui::Key::Space));
    let is_middle_down = ui.input(|i| i.pointer.middle_down());
    let is_secondary_down = ui.input(|i| i.pointer.secondary_down());

    if canvas_sense_resp.drag_started() {
        if let Some(pos) = pointer_pos {
            if is_space_down || is_middle_down || is_secondary_down {
                drag_mode = CanvasDragMode::Pan { start_pointer: pos, start_pan: studio.canvas_pan };
            } else if let (Some(sel), Some((rot_rect, resize_rect, center, half_w, half_h))) = (studio.selected_element_idx, selected_handles) {
                if rot_rect.expand(4.0).contains(pos) {
                    let unrot_vec = vec2(-half_w - 3.0, -half_h - 3.0);
                    let base_angle_rad = unrot_vec.y.atan2(unrot_vec.x);
                    drag_mode = CanvasDragMode::Rotate {
                        idx: sel,
                        center,
                        base_angle_rad,
                    };
                } else if resize_rect.expand(4.0).contains(pos) {
                    let elem = &studio.overlay.elements[sel];
                    let (raw_w, raw_h) = match elem {
                        OverlayElement::Crosshair { size, .. } => (*size, *size),
                        OverlayElement::Text { font_size, .. } => (*font_size, *font_size),
                        _ => get_element_size(elem),
                    };
                    drag_mode = CanvasDragMode::Resize {
                        idx: sel,
                        start_w: raw_w,
                        start_h: raw_h,
                        start_pointer: pos,
                    };
                } else if let Some((hit_idx, _)) = elem_rects.iter().rev().find(|(_, r)| r.expand(3.0).contains(pos)) {
                    studio.selected_element_idx = Some(*hit_idx);
                    let (elem_x, elem_y) = (studio.overlay.elements[*hit_idx].x(), studio.overlay.elements[*hit_idx].y());
                    drag_mode = CanvasDragMode::Move {
                        idx: *hit_idx,
                        start_x: elem_x,
                        start_y: elem_y,
                        start_pointer: pos,
                    };
                } else if card_resize_handle.expand(4.0).contains(pos) {
                    studio.selected_element_idx = None;
                    drag_mode = CanvasDragMode::ResizeCard {
                        start_pointer: pos,
                        start_w: studio.overlay.rect.width,
                        start_h: studio.overlay.rect.height,
                    };
                } else if pill_rect.expand(2.0).contains(pos) || card_canvas_rect.contains(pos) {
                    studio.selected_element_idx = None;
                    drag_mode = CanvasDragMode::MoveCard {
                        start_pointer: pos,
                        start_card_sx: card_sx,
                        start_card_sy: card_sy,
                    };
                } else {
                    studio.selected_element_idx = None;
                    drag_mode = CanvasDragMode::Pan { start_pointer: pos, start_pan: studio.canvas_pan };
                }
            } else if let Some((hit_idx, _)) = elem_rects.iter().rev().find(|(_, r)| r.expand(3.0).contains(pos)) {
                studio.selected_element_idx = Some(*hit_idx);
                let (elem_x, elem_y) = (studio.overlay.elements[*hit_idx].x(), studio.overlay.elements[*hit_idx].y());
                drag_mode = CanvasDragMode::Move {
                    idx: *hit_idx,
                    start_x: elem_x,
                    start_y: elem_y,
                    start_pointer: pos,
                };
            } else if card_resize_handle.expand(4.0).contains(pos) {
                studio.selected_element_idx = None;
                drag_mode = CanvasDragMode::ResizeCard {
                    start_pointer: pos,
                    start_w: studio.overlay.rect.width,
                    start_h: studio.overlay.rect.height,
                };
            } else if pill_rect.expand(2.0).contains(pos) || card_canvas_rect.contains(pos) {
                studio.selected_element_idx = None;
                drag_mode = CanvasDragMode::MoveCard {
                    start_pointer: pos,
                    start_card_sx: card_sx,
                    start_card_sy: card_sy,
                };
            } else {
                studio.selected_element_idx = None;
                drag_mode = CanvasDragMode::Pan { start_pointer: pos, start_pan: studio.canvas_pan };
            }
        }
    } else if canvas_sense_resp.clicked() {
        if let Some(pos) = pointer_pos {
            let hit = elem_rects.iter().rev().find(|(_, r)| r.expand(3.0).contains(pos));
            studio.selected_element_idx = hit.map(|(i, _)| *i);
        }
    }

    let snap = |val: f32| -> f32 {
        if studio.snap_to_grid {
            (val / 16.0).round() * 16.0
        } else {
            val.round()
        }
    };

    if canvas_sense_resp.dragged() {
        if let Some(pos) = pointer_pos {
            match &mut drag_mode {
                CanvasDragMode::Pan { start_pointer, start_pan } => {
                    studio.canvas_pan = *start_pan + (pos - *start_pointer);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }
                CanvasDragMode::MoveCard { start_pointer, start_card_sx, start_card_sy } => {
                    // Safeguard: only auto-pan camera across canvas when the overlay card itself is selected (no element selected)
                    if studio.selected_element_idx.is_none() {
                        let edge_zone = 70.0;
                        let (cur_sx, cur_sy) = compute_overlay_screen_pos(&studio.overlay, screen_w, screen_h);
                        let max_card_x = (screen_w - studio.overlay.rect.width).max(0.0);
                        let max_card_y = (screen_h - studio.overlay.rect.height).max(0.0);

                        let can_pan_right = cur_sx < max_card_x - 0.5;
                        let can_pan_left = cur_sx > 0.5;
                        let can_pan_down = cur_sy < max_card_y - 0.5;
                        let can_pan_up = cur_sy > 0.5;

                        let mut pan_step = Vec2::ZERO;

                        if can_pan_right && pos.x > canvas_rect.right() - edge_zone {
                            let factor = ((pos.x - (canvas_rect.right() - edge_zone)) / edge_zone).clamp(0.0, 1.5);
                            pan_step.x -= 3.0 + 8.0 * factor;
                        } else if can_pan_left && pos.x < canvas_rect.left() + edge_zone {
                            let factor = ((canvas_rect.left() + edge_zone - pos.x) / edge_zone).clamp(0.0, 1.5);
                            pan_step.x += 3.0 + 8.0 * factor;
                        }

                        if can_pan_down && pos.y > canvas_rect.bottom() - edge_zone {
                            let factor = ((pos.y - (canvas_rect.bottom() - edge_zone)) / edge_zone).clamp(0.0, 1.5);
                            pan_step.y -= 3.0 + 8.0 * factor;
                        } else if can_pan_up && pos.y < canvas_rect.top() + edge_zone {
                            let factor = ((canvas_rect.top() + edge_zone - pos.y) / edge_zone).clamp(0.0, 1.5);
                            pan_step.y += 3.0 + 8.0 * factor;
                        }

                        if pan_step != Vec2::ZERO {
                            studio.canvas_pan += pan_step;
                            *start_pointer += pan_step;
                            ui.ctx().request_repaint();
                        }
                    }

                    let delta = (pos - *start_pointer) / zoom.max(0.05);
                    let mut new_sx = snap(*start_card_sx + delta.x);
                    let mut new_sy = snap(*start_card_sy + delta.y);

                    // Magnetic screen-center snap guide (within 12px)
                    let center_x = (screen_w - studio.overlay.rect.width) * 0.5;
                    let center_y = (screen_h - studio.overlay.rect.height) * 0.5;
                    let is_center_x = (new_sx - center_x).abs() < 12.0;
                    let is_center_y = (new_sy - center_y).abs() < 12.0;
                    if is_center_x { new_sx = center_x; }
                    if is_center_y { new_sy = center_y; }

                    new_sx = new_sx.clamp(0.0, (screen_w - studio.overlay.rect.width).max(0.0));
                    new_sy = new_sy.clamp(0.0, (screen_h - studio.overlay.rect.height).max(0.0));

                    if is_center_x && is_center_y {
                        studio.overlay.anchor = OverlayAnchor::ScreenCenter;
                        studio.overlay.rect.x = 0.0;
                        studio.overlay.rect.y = 0.0;
                    } else {
                        studio.overlay.anchor = OverlayAnchor::Custom;
                        studio.overlay.rect.x = new_sx;
                        studio.overlay.rect.y = new_sy;
                    }
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }
                CanvasDragMode::ResizeCard { start_pointer, start_w, start_h } => {
                    // Safeguard: only auto-pan camera across canvas when no element is selected
                    if studio.selected_element_idx.is_none() {
                        let edge_zone = 70.0;
                        let (cur_sx, cur_sy) = compute_overlay_screen_pos(&studio.overlay, screen_w, screen_h);
                        let can_pan_right = cur_sx + studio.overlay.rect.width < screen_w - 0.5;
                        let can_pan_down = cur_sy + studio.overlay.rect.height < screen_h - 0.5;

                        let mut pan_step = Vec2::ZERO;
                        if can_pan_right && pos.x > canvas_rect.right() - edge_zone {
                            let factor = ((pos.x - (canvas_rect.right() - edge_zone)) / edge_zone).clamp(0.0, 1.5);
                            pan_step.x -= 3.0 + 8.0 * factor;
                        }
                        if can_pan_down && pos.y > canvas_rect.bottom() - edge_zone {
                            let factor = ((pos.y - (canvas_rect.bottom() - edge_zone)) / edge_zone).clamp(0.0, 1.5);
                            pan_step.y -= 3.0 + 8.0 * factor;
                        }
                        if pan_step != Vec2::ZERO {
                            studio.canvas_pan += pan_step;
                            *start_pointer += pan_step;
                            ui.ctx().request_repaint();
                        }
                    }

                    let (max_w, max_h) = max_card_dimensions(&studio.overlay, screen_w, screen_h);
                    let delta = (pos - *start_pointer) / zoom.max(0.05);
                    let is_locked = studio.card_lock_aspect || ui.input(|i| i.modifiers.shift);
                    if is_locked {
                        let aspect = (*start_w / (*start_h).max(1.0)).clamp(0.05, 50.0);
                        let d = if delta.x.abs() > delta.y.abs() { delta.x } else { delta.y * aspect };
                        let effective_max_w = max_w.min(max_h * aspect);
                        let effective_max_h = max_h.min(max_w / aspect.max(0.01));
                        let new_w = snap(*start_w + d).clamp(20.0, effective_max_w);
                        let new_h = (new_w / aspect).round().clamp(20.0, effective_max_h);
                        studio.overlay.rect.width = new_w;
                        studio.overlay.rect.height = new_h;
                    } else {
                        studio.overlay.rect.width = snap(*start_w + delta.x).clamp(20.0, max_w);
                        studio.overlay.rect.height = snap(*start_h + delta.y).clamp(20.0, max_h);
                    }
                    clamp_elements_to_canvas(&mut studio.overlay);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                }
                CanvasDragMode::Move { idx, start_x, start_y, start_pointer } => {
                    if *idx < studio.overlay.elements.len() {
                        let total_delta = (pos - *start_pointer) / zoom.max(0.05);
                        let (ew, eh) = get_element_size(&studio.overlay.elements[*idx]);
                        match &mut studio.overlay.elements[*idx] {
                            OverlayElement::Text { x, y, .. }
                            | OverlayElement::Badge { x, y, .. }
                            | OverlayElement::ProgressBar { x, y, .. }
                            | OverlayElement::Image { x, y, .. }
                            | OverlayElement::Shape { x, y, .. } => {
                                *x = snap(*start_x + total_delta.x).clamp(0.0, (studio.overlay.rect.width - ew).max(0.0));
                                *y = snap(*start_y + total_delta.y).clamp(0.0, (studio.overlay.rect.height - eh).max(0.0));
                            }
                            OverlayElement::Crosshair { x, y, .. } => {
                                *x = snap(*start_x + total_delta.x).clamp(0.0, studio.overlay.rect.width);
                                *y = snap(*start_y + total_delta.y).clamp(0.0, studio.overlay.rect.height);
                            }
                        }
                        clamp_elements_to_canvas(&mut studio.overlay);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }
                }
                CanvasDragMode::Rotate { idx, center, base_angle_rad } => {
                    if *idx < studio.overlay.elements.len() {
                        let v = pos - *center;
                        if v.length_sq() > 4.0 {
                            let current_angle = v.y.atan2(v.x);
                            let diff_rad = current_angle - *base_angle_rad;
                            let mut deg = diff_rad.to_degrees().rem_euclid(360.0);
                            if ui.input(|i| i.modifiers.shift) {
                                deg = (deg / 15.0).round() * 15.0;
                                deg = deg.rem_euclid(360.0);
                            } else {
                                deg = deg.round();
                            }
                            *studio.overlay.elements[*idx].rotation_mut() = deg;
                        }
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                }
                CanvasDragMode::Resize { idx, start_w, start_h, start_pointer } => {
                    if *idx < studio.overlay.elements.len() {
                        let total_delta = (pos - *start_pointer) / zoom.max(0.05);
                        match &mut studio.overlay.elements[*idx] {
                            OverlayElement::Text { font_size, .. } => {
                                let delta = (total_delta.x + total_delta.y) * 0.25;
                                *font_size = snap(*start_w + delta).clamp(6.0, 72.0);
                            }
                            OverlayElement::ProgressBar { width, height, x, y, .. } => {
                                let max_w = (studio.overlay.rect.width - *x).max(20.0);
                                let max_h = (studio.overlay.rect.height - *y).max(4.0);
                                *width = snap(*start_w + total_delta.x).clamp(20.0, max_w);
                                *height = snap(*start_h + total_delta.y).clamp(4.0, max_h);
                            }
                            OverlayElement::Image { width, height, lock_aspect, x, y, .. } => {
                                let max_w = (studio.overlay.rect.width - *x).max(8.0);
                                let max_h = (studio.overlay.rect.height - *y).max(8.0);
                                let is_locked = *lock_aspect || ui.input(|i| i.modifiers.shift);
                                if is_locked {
                                    let ratio = (*start_w / (*start_h).max(1.0)).clamp(0.05, 50.0);
                                    let delta = if total_delta.x.abs() > total_delta.y.abs() {
                                        total_delta.x
                                    } else {
                                        total_delta.y * ratio
                                    };
                                    let new_w = snap(*start_w + delta).clamp(8.0, max_w.min(max_h * ratio));
                                    *width = new_w;
                                    *height = (new_w / ratio).round().clamp(8.0, max_h);
                                } else {
                                    *width = snap(*start_w + total_delta.x).clamp(8.0, max_w);
                                    *height = snap(*start_h + total_delta.y).clamp(8.0, max_h);
                                }
                            }
                            OverlayElement::Shape { width, height, lock_aspect, x, y, .. } => {
                                let max_w = (studio.overlay.rect.width - *x).max(8.0);
                                let max_h = (studio.overlay.rect.height - *y).max(8.0);
                                let is_locked = *lock_aspect || ui.input(|i| i.modifiers.shift);
                                if is_locked {
                                    let ratio = (*start_w / (*start_h).max(1.0)).clamp(0.05, 50.0);
                                    let delta = if total_delta.x.abs() > total_delta.y.abs() {
                                        total_delta.x
                                    } else {
                                        total_delta.y * ratio
                                    };
                                    let new_w = snap(*start_w + delta).clamp(8.0, max_w.min(max_h * ratio));
                                    *width = new_w;
                                    *height = (new_w / ratio).round().clamp(8.0, max_h);
                                } else {
                                    *width = snap(*start_w + total_delta.x).clamp(8.0, max_w);
                                    *height = snap(*start_h + total_delta.y).clamp(8.0, max_h);
                                }
                            }
                            OverlayElement::Crosshair { size, x, y, .. } => {
                                let max_s = (studio.overlay.rect.width - *x).min(studio.overlay.rect.height - *y).max(8.0);
                                *size = snap(*start_w + total_delta.x * 0.5).clamp(4.0, max_s.min(128.0));
                            }
                            OverlayElement::Badge { .. } => {}
                        }
                        clamp_elements_to_canvas(&mut studio.overlay);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                    }
                }
                CanvasDragMode::None => {}
            }
        }
    } else {
        drag_mode = CanvasDragMode::None;
        if let Some(pos) = pointer_pos {
            if is_space_down {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            } else if let Some((rot_rect, resize_rect, _, _, _)) = selected_handles {
                if rot_rect.expand(3.0).contains(pos) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                } else if resize_rect.expand(3.0).contains(pos) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                } else if elem_rects.iter().any(|(_, r)| r.expand(3.0).contains(pos)) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                } else if card_resize_handle.expand(3.0).contains(pos) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                } else if pill_rect.contains(pos) || card_canvas_rect.contains(pos) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            } else if elem_rects.iter().any(|(_, r)| r.expand(3.0).contains(pos)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            } else if card_resize_handle.expand(3.0).contains(pos) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            } else if pill_rect.contains(pos) || card_canvas_rect.contains(pos) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
    }

    // Mouse wheel camera zooming (or Alt+wheel for fine element rotation)
    if canvas_sense_resp.hovered() {
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.5 {
            if ui.input(|i| i.modifiers.alt) {
                if let Some(idx) = studio.selected_element_idx {
                    if idx < studio.overlay.elements.len() {
                        let sign = scroll_y.signum();
                        let cur_rot = studio.overlay.elements[idx].rotation();
                        let new_rot = (cur_rot + sign * 5.0).rem_euclid(360.0);
                        *studio.overlay.elements[idx].rotation_mut() = new_rot;
                    }
                }
            } else {
                let zoom_delta = (scroll_y * 0.0016).clamp(-0.075, 0.075);
                let factor = 1.0 + zoom_delta;
                let new_zoom = (zoom * factor).clamp(0.08, 10.0);
                if let Some(pos) = pointer_pos {
                    let screen_pt = (pos - monitor_origin) / zoom;
                    let new_monitor_origin = pos - screen_pt * new_zoom;
                    studio.canvas_pan = new_monitor_origin + vec2(screen_w * 0.5 * new_zoom, screen_h * 0.5 * new_zoom) - canvas_rect.center();
                }
                studio.canvas_zoom = Some(new_zoom);
            }
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(drag_mode_id, drag_mode));

    // 5. Draw Preview Card Background with global opacity applied
    if studio.overlay.style.bg_color[3] > 0 {
        let bg_color = apply_opacity(studio.overlay.style.bg_color);
        let cr = CornerRadius::same((studio.overlay.style.corner_radius * zoom).round() as u8);
        painter.rect_filled(card_canvas_rect, cr, bg_color);
    }

    // Draw Preview Card Border
    let card_border_cr = CornerRadius::same((studio.overlay.style.corner_radius * zoom).round() as u8);
    if studio.overlay.style.border_color[3] > 0 && studio.overlay.style.border_width > 0.0 {
        let border_color = apply_opacity(studio.overlay.style.border_color);
        painter.rect_stroke(
            card_canvas_rect,
            card_border_cr,
            Stroke::new((studio.overlay.style.border_width * zoom).max(1.0), border_color),
            egui::StrokeKind::Inside,
        );
    } else {
        painter.rect_stroke(
            card_canvas_rect,
            card_border_cr,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 160, 255, 60)),
            egui::StrokeKind::Inside,
        );
    }

    // Card Header Pill
    painter.rect_filled(pill_rect, CornerRadius::same(3), Color32::from_rgba_unmultiplied(20, 26, 38, 220));
    painter.rect_stroke(pill_rect, CornerRadius::same(3), Stroke::new(1.0, t.accent_ink.gamma_multiply(0.4)), egui::StrokeKind::Inside);
    let pill_text = format!("⬚ OVERLAY ({}×{})", studio.overlay.rect.width as i32, studio.overlay.rect.height as i32);
    painter.text(pill_rect.center(), egui::Align2::CENTER_CENTER, pill_text, FontId::new(9.5, theme::mono_bold()), t.accent_ink);

    // Card Resize Handle at bottom-right corner
    painter.rect_filled(card_resize_handle, CornerRadius::same(2), t.sunken);
    painter.rect_stroke(card_resize_handle, CornerRadius::same(2), Stroke::new(1.0, t.accent_ink), egui::StrokeKind::Inside);
    painter.line_segment([pos2(card_resize_handle.left() + 3.0, card_resize_handle.bottom() - 3.0), pos2(card_resize_handle.right() - 3.0, card_resize_handle.top() + 3.0)], Stroke::new(1.5, t.accent_ink));

    // 6. Render Overlay Elements inside Card boundaries using Card Clipping
    let card_painter = painter.with_clip_rect(card_canvas_rect.intersect(canvas_rect));

    for (elem_idx, elem) in studio.overlay.elements.iter().enumerate() {
        match elem {
            OverlayElement::Crosshair { style, size, color, thickness, gap, center_dot, x, y, rotation, .. } => {
                let cc = if *x > 0.0 && *y > 0.0 {
                    card_canvas_rect.min + vec2(*x * zoom, *y * zoom)
                } else {
                    card_canvas_rect.center()
                };

                let stroke = Stroke::new((*thickness * zoom).max(1.0), apply_opacity(*color));
                let g = *gap * zoom;
                let s = *size * zoom;
                let angle_rad = rotation.to_radians();

                let rot = |v: egui::Vec2| rotate_vec(v, angle_rad);

                match style {
                    CrosshairStyle::Cross => {
                        card_painter.line_segment([cc + rot(vec2(-(g + s), 0.0)), cc + rot(vec2(-g, 0.0))], stroke);
                        card_painter.line_segment([cc + rot(vec2(g, 0.0)), cc + rot(vec2(g + s, 0.0))], stroke);
                        card_painter.line_segment([cc + rot(vec2(0.0, -(g + s))), cc + rot(vec2(0.0, -g))], stroke);
                        card_painter.line_segment([cc + rot(vec2(0.0, g)), cc + rot(vec2(0.0, g + s))], stroke);
                    }
                    CrosshairStyle::TShaped => {
                        card_painter.line_segment([cc + rot(vec2(-(g + s), 0.0)), cc + rot(vec2(-g, 0.0))], stroke);
                        card_painter.line_segment([cc + rot(vec2(g, 0.0)), cc + rot(vec2(g + s, 0.0))], stroke);
                        card_painter.line_segment([cc + rot(vec2(0.0, g)), cc + rot(vec2(0.0, g + s))], stroke);
                    }
                    CrosshairStyle::Circle => {
                        card_painter.circle_stroke(cc, s / 2.0, stroke);
                    }
                    CrosshairStyle::Dot => {}
                }

                if *center_dot {
                    card_painter.circle_filled(cc, (*thickness * zoom).max(1.0), apply_opacity(*color));
                }
            }
            OverlayElement::Text { text, font_size, color, x, y, bold, font_family, rotation, .. } => {
                let norm = font_family.trim().to_lowercase();
                let family = match norm.as_str() {
                    "archivo" | "archivo display" => {
                        if *bold { theme::font_archivo_bold() } else { theme::font_archivo() }
                    }
                    "inter" => {
                        if *bold { theme::semibold() } else { theme::medium() }
                    }
                    "geist mono" | "geistmono" | "mono" => {
                        if *bold { theme::mono_bold() } else { theme::mono() }
                    }
                    _ => {
                        egui::FontFamily::Name(norm.into())
                    }
                };

                let font = FontId::new((*font_size * zoom).max(7.0), family);
                let text_color = apply_opacity(*color);
                let galley = card_painter.layout_no_wrap(text.clone(), font, text_color);
                let text_size = galley.size();

                let (nat_w, nat_h) = get_element_size(elem);
                let center = card_canvas_rect.min + vec2((*x + nat_w * 0.5) * zoom, (*y + nat_h * 0.5) * zoom);
                let angle_rad = rotation.to_radians();

                let half_galley = text_size * 0.5;
                let pos = center - rotate_vec(half_galley, angle_rad);

                let mut text_shape = egui::epaint::TextShape::new(pos, galley, text_color);
                text_shape.angle = angle_rad;
                card_painter.add(text_shape);
            }
            OverlayElement::Badge { text, color, bg_color, x, y, rotation, .. } => {
                let (raw_w, raw_h) = get_element_size(elem);
                let b_size = vec2((raw_w * zoom).max(8.0), (raw_h * zoom).max(8.0));
                let center = card_canvas_rect.min + vec2((*x + raw_w * 0.5) * zoom, (*y + raw_h * 0.5) * zoom);
                let angle_rad = rotation.to_radians();

                let bg_c = apply_opacity(*bg_color);
                let fg_c = apply_opacity(*color);

                if angle_rad.abs() < 0.001 {
                    let b_rect = Rect::from_center_size(center, b_size);
                    card_painter.rect_filled(b_rect, CornerRadius::same(4), bg_c);
                    card_painter.rect_stroke(b_rect, CornerRadius::same(4), Stroke::new(1.0, fg_c), egui::StrokeKind::Inside);
                    card_painter.text(center, egui::Align2::CENTER_CENTER, text, FontId::new((10.5 * zoom).max(6.0), theme::semibold()), fg_c);
                } else {
                    let hw = b_size.x * 0.5;
                    let hh = b_size.y * 0.5;
                    let pts = [
                        center + rotate_vec(vec2(-hw, -hh), angle_rad),
                        center + rotate_vec(vec2(hw, -hh), angle_rad),
                        center + rotate_vec(vec2(hw, hh), angle_rad),
                        center + rotate_vec(vec2(-hw, hh), angle_rad),
                    ];
                    draw_filled_convex_polygon(&card_painter, center, &pts, bg_c);
                    draw_polygon_stroke(&card_painter, &pts, Stroke::new(1.0, fg_c));

                    let font = FontId::new((10.5 * zoom).max(6.0), theme::semibold());
                    let galley = card_painter.layout_no_wrap(text.clone(), font, fg_c);
                    let half_g = galley.size() * 0.5;
                    let text_pos = center - rotate_vec(half_g, angle_rad);
                    let mut text_shape = egui::epaint::TextShape::new(text_pos, galley, fg_c);
                    text_shape.angle = angle_rad;
                    card_painter.add(text_shape);
                }
            }
            OverlayElement::ProgressBar { progress, fill_color, bg_color, x, y, width, height, rotation, .. } => {
                let p_size = vec2((*width * zoom).max(8.0), (*height * zoom).max(2.0));
                let center = card_canvas_rect.min + vec2((*x + *width * 0.5) * zoom, (*y + *height * 0.5) * zoom);
                let angle_rad = rotation.to_radians();

                let bg_c = apply_opacity(*bg_color);
                let fg_c = apply_opacity(*fill_color);

                if angle_rad.abs() < 0.001 {
                    let p_rect = Rect::from_center_size(center, p_size);
                    card_painter.rect_filled(p_rect, CornerRadius::same(2), bg_c);
                    let fill_w = (p_size.x * progress.clamp(0.0, 1.0)).max(0.0);
                    let fill_rect = Rect::from_min_size(p_rect.min, vec2(fill_w, p_size.y));
                    card_painter.rect_filled(fill_rect, CornerRadius::same(2), fg_c);
                } else {
                    let hw = p_size.x * 0.5;
                    let hh = p_size.y * 0.5;
                    let pts = [
                        center + rotate_vec(vec2(-hw, -hh), angle_rad),
                        center + rotate_vec(vec2(hw, -hh), angle_rad),
                        center + rotate_vec(vec2(hw, hh), angle_rad),
                        center + rotate_vec(vec2(-hw, hh), angle_rad),
                    ];
                    draw_filled_convex_polygon(&card_painter, center, &pts, bg_c);

                    let prog = progress.clamp(0.0, 1.0);
                    if prog > 0.0 {
                        let fill_hw = hw * prog;
                        let fill_center_unrot = vec2(-hw + fill_hw, 0.0);
                        let fill_center = center + rotate_vec(fill_center_unrot, angle_rad);
                        let fill_pts = [
                            fill_center + rotate_vec(vec2(-fill_hw, -hh), angle_rad),
                            fill_center + rotate_vec(vec2(fill_hw, -hh), angle_rad),
                            fill_center + rotate_vec(vec2(fill_hw, hh), angle_rad),
                            fill_center + rotate_vec(vec2(-fill_hw, hh), angle_rad),
                        ];
                        draw_filled_convex_polygon(&card_painter, fill_center, &fill_pts, fg_c);
                    }
                }
            }
            OverlayElement::Image { path, x, y, width, height, opacity, rotation, .. } => {
                let img_size = vec2((*width * zoom).max(4.0), (*height * zoom).max(4.0));
                let center = card_canvas_rect.min + vec2((*x + *width * 0.5) * zoom, (*y + *height * 0.5) * zoom);
                let img_rect = Rect::from_center_size(center, img_size);
                let angle_rad = rotation.to_radians();

                if !path.is_empty() && std::path::Path::new(path).exists() {
                    let clean = path.replace('\\', "/");
                    let uri = if clean.starts_with('/') {
                        format!("file://{clean}")
                    } else {
                        format!("file:///{clean}")
                    };
                    let alpha = ((255.0 * opacity * studio.overlay.opacity) as u8).max(10);
                    let img = egui::Image::new(uri)
                        .tint(Color32::from_white_alpha(alpha))
                        .fit_to_exact_size(img_size)
                        .maintain_aspect_ratio(false)
                        .rotate(angle_rad, vec2(0.5, 0.5));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(card_canvas_rect), |ui| {
                        ui.set_clip_rect(card_canvas_rect);
                        ui.put(img_rect, img);
                    });
                } else {
                    let hw = img_size.x * 0.5;
                    let hh = img_size.y * 0.5;
                    let p0 = center + rotate_vec(vec2(-hw, -hh), angle_rad);
                    let p1 = center + rotate_vec(vec2(hw, -hh), angle_rad);
                    let p2 = center + rotate_vec(vec2(hw, hh), angle_rad);
                    let p3 = center + rotate_vec(vec2(-hw, hh), angle_rad);
                    let pts = [p0, p1, p2, p3];

                    draw_filled_convex_polygon(&card_painter, center, &pts, Color32::from_rgba_unmultiplied(40, 44, 56, 120));
                    draw_polygon_stroke(&card_painter, &pts, Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 110, 130, 100)));

                    let text_color = Color32::from_rgb(140, 150, 170);
                    let font = FontId::new((9.5 * zoom).max(6.0), theme::semibold());
                    let galley = card_painter.layout_no_wrap("IMG".to_string(), font, text_color);
                    let half_g = galley.size() * 0.5;
                    let text_pos = center - rotate_vec(half_g, angle_rad);
                    let mut text_shape = egui::epaint::TextShape::new(text_pos, galley, text_color);
                    text_shape.angle = angle_rad;
                    card_painter.add(text_shape);
                }
            }
            OverlayElement::Shape {
                shape_type,
                x,
                y,
                width,
                height,
                thickness,
                color,
                filled,
                fill_color,
                corner_radius,
                rotation,
                ..
            } => {
                let s_size = vec2((*width * zoom).max(2.0), (*height * zoom).max(2.0));
                let center = card_canvas_rect.min + vec2((*x + *width * 0.5) * zoom, (*y + *height * 0.5) * zoom);
                let s_rect = Rect::from_center_size(center, s_size);
                let stroke_c = apply_opacity(*color);
                let stroke_w = (*thickness * zoom).max(1.0);
                let stroke = Stroke::new(stroke_w, stroke_c);
                let angle_rad = rotation.to_radians();

                match shape_type {
                    ShapeKind::Rectangle => {
                        let hw = s_size.x * 0.5;
                        let hh = s_size.y * 0.5;
                        let cr = (*corner_radius * zoom).min(hw).min(hh);
                        let fill_c = if *filled { Some(apply_opacity(*fill_color)) } else { None };

                        if angle_rad.abs() < 0.001 {
                            let corner_rad = CornerRadius::same(cr.round() as u8);
                            if let Some(fc) = fill_c {
                                card_painter.rect_filled(s_rect, corner_rad, fc);
                            }
                            card_painter.rect_stroke(s_rect, corner_rad, stroke, egui::StrokeKind::Inside);
                        } else {
                            let mut points = Vec::new();
                            if cr > 0.5 {
                                let arc_steps = 4;
                                let centers = [
                                    (vec2(hw - cr, -(hh - cr)), -std::f32::consts::FRAC_PI_2),
                                    (vec2(hw - cr, hh - cr), 0.0),
                                    (vec2(-(hw - cr), hh - cr), std::f32::consts::FRAC_PI_2),
                                    (vec2(-(hw - cr), -(hh - cr)), std::f32::consts::PI),
                                ];
                                for (c_off, start_a) in centers {
                                    for s in 0..arc_steps {
                                        let a = start_a + (s as f32) * std::f32::consts::FRAC_PI_2 / (arc_steps as f32);
                                        let pt = c_off + vec2(cr * a.cos(), cr * a.sin());
                                        points.push(center + rotate_vec(pt, angle_rad));
                                    }
                                }
                            } else {
                                points.push(center + rotate_vec(vec2(-hw, -hh), angle_rad));
                                points.push(center + rotate_vec(vec2(hw, -hh), angle_rad));
                                points.push(center + rotate_vec(vec2(hw, hh), angle_rad));
                                points.push(center + rotate_vec(vec2(-hw, hh), angle_rad));
                            }

                            if let Some(fc) = fill_c {
                                draw_filled_convex_polygon(&card_painter, center, &points, fc);
                            }
                            draw_polygon_stroke(&card_painter, &points, stroke);
                        }
                    }
                    ShapeKind::Circle => {
                        let rx = s_size.x / 2.0;
                        let ry = s_size.y / 2.0;
                        let fill_c = if *filled { Some(apply_opacity(*fill_color)) } else { None };

                        if angle_rad.abs() < 0.001 && (rx - ry).abs() < 0.5 {
                            if let Some(fc) = fill_c {
                                card_painter.circle_filled(center, rx, fc);
                            }
                            card_painter.circle_stroke(center, rx, stroke);
                        } else {
                            let n = 48;
                            let mut points = Vec::with_capacity(n);
                            for i in 0..n {
                                let t = (i as f32) * std::f32::consts::TAU / (n as f32);
                                let unrot = vec2(rx * t.cos(), ry * t.sin());
                                let rot = rotate_vec(unrot, angle_rad);
                                points.push(center + rot);
                            }

                            if let Some(fc) = fill_c {
                                draw_filled_convex_polygon(&card_painter, center, &points, fc);
                            }
                            draw_polygon_stroke(&card_painter, &points, stroke);
                        }
                    }
                    ShapeKind::Line => {
                        let hw = s_size.x * 0.5;
                        let p_left = center + rotate_vec(vec2(-hw, 0.0), angle_rad);
                        let p_right = center + rotate_vec(vec2(hw, 0.0), angle_rad);
                        card_painter.line_segment([p_left, p_right], stroke);
                    }
                    ShapeKind::Arrow => {
                        let hw = s_size.x * 0.5;
                        let p_left = center + rotate_vec(vec2(-hw, 0.0), angle_rad);
                        let p_right = center + rotate_vec(vec2(hw, 0.0), angle_rad);
                        card_painter.line_segment([p_left, p_right], stroke);
                        let head_sz = (s_size.y * 0.4).clamp(4.0 * zoom, 24.0 * zoom);
                        let barb_top = p_right + rotate_vec(vec2(-head_sz, -head_sz), angle_rad);
                        let barb_bottom = p_right + rotate_vec(vec2(-head_sz, head_sz), angle_rad);
                        card_painter.line_segment([barb_top, p_right], stroke);
                        card_painter.line_segment([barb_bottom, p_right], stroke);
                    }
                }
            }
        }

        // Draw selection highlight box & interactive resize handle (rotated OBB)
        if studio.selected_element_idx == Some(elem_idx) {
            let (raw_w, raw_h) = get_element_size(elem);
            let ew = (raw_w * zoom).max(4.0);
            let eh = (raw_h * zoom).max(4.0);
            let angle_rad = elem.rotation().to_radians();
            let center = match elem {
                OverlayElement::Crosshair { x, y, .. } => {
                    if *x > 0.0 && *y > 0.0 {
                        card_canvas_rect.min + vec2(*x * zoom, *y * zoom)
                    } else {
                        card_canvas_rect.center()
                    }
                }
                OverlayElement::Text { x, y, .. }
                | OverlayElement::Badge { x, y, .. }
                | OverlayElement::ProgressBar { x, y, .. }
                | OverlayElement::Image { x, y, .. }
                | OverlayElement::Shape { x, y, .. } => {
                    card_canvas_rect.min + vec2((*x + raw_w * 0.5) * zoom, (*y + raw_h * 0.5) * zoom)
                }
            };

            let half_w = ew * 0.5;
            let half_h = eh * 0.5;
            let p0 = rotate_point(center + vec2(-half_w - 3.0, -half_h - 3.0), center, angle_rad);
            let p1 = rotate_point(center + vec2(half_w + 3.0, -half_h - 3.0), center, angle_rad);
            let p2 = rotate_point(center + vec2(half_w + 3.0, half_h + 3.0), center, angle_rad);
            let p3 = rotate_point(center + vec2(-half_w - 3.0, half_h + 3.0), center, angle_rad);

            let stroke = Stroke::new(1.5, t.accent_ink);
            painter.line_segment([p0, p1], stroke);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p0], stroke);

            // 1. Top-Left: Rotate handle (circular knob)
            painter.circle_filled(p0, 6.0, t.sunken);
            painter.circle_stroke(p0, 6.0, Stroke::new(1.5, t.accent_ink));
            painter.circle_filled(p0, 2.5, t.accent_ink);

            // 2. Bottom-Right: Resize handle (square knob with diagonal line)
            let h_rect = Rect::from_center_size(p2, vec2(12.0, 12.0));
            painter.rect_filled(h_rect, CornerRadius::same(2), t.accent_ink);
            painter.rect_stroke(h_rect, CornerRadius::same(2), Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Inside);
            painter.line_segment([pos2(h_rect.left() + 3.0, h_rect.bottom() - 3.0), pos2(h_rect.right() - 3.0, h_rect.top() + 3.0)], Stroke::new(1.0, Color32::WHITE));

            // 3. Live coordinate tag with element size and rotation, rotating in lockstep with container
            let coord_str = format!("X: {:.0} Y: {:.0} | {:.0}×{:.0} px | {:.0}°", elem.x(), elem.y(), raw_w, raw_h, elem.rotation());
            let font = FontId::new(9.5, theme::mono());
            let galley = painter.layout_no_wrap(coord_str, font, t.accent_ink);
            let tag_local = vec2(-half_w, half_h + 8.0);
            let tag_pos = center + rotate_vec(tag_local, angle_rad);
            let mut text_shape = egui::epaint::TextShape::new(tag_pos, galley, t.accent_ink);
            text_shape.angle = angle_rad;
            painter.add(text_shape);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_overlay_modal_no_crash() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);

        let presets = ["overlay_crosshair", "overlay_macro_badge", "overlay_telemetry", "unknown_custom"];
        for preset in presets {
            let mut state = GuiState::new();
            state.open_overlay_studio(preset);

            for _ in 0..5 {
                let mut out = ctx.run_ui(Default::default(), |ctx| {
                    render_overlay_modal(ctx, &mut state);
                });
                out.textures_delta.clear();
            }

            state.close_overlay_studio();
            for _ in 0..5 {
                let mut out = ctx.run_ui(Default::default(), |ctx| {
                    render_overlay_modal(ctx, &mut state);
                });
                out.textures_delta.clear();
            }
        }
    }

    #[test]
    fn test_overlay_elements_with_fonts_and_coordinates() {
        use crate::models::{OverlayDefinition, OverlayElement};

        let mut overlay = OverlayDefinition::default_crosshair();
        assert_eq!(overlay.elements.len(), 1);
        if let OverlayElement::Crosshair { x, y, .. } = overlay.elements[0] {
            assert_eq!(x, 24.0);
            assert_eq!(y, 24.0);
        } else {
            panic!("Expected crosshair element");
        }

        // Test text font family & click action
        overlay.elements.push(OverlayElement::Text {
            id: "txt_1".to_string(),
            text: "FPS: 144".to_string(),
            font_size: 14.0,
            font_family: "Impact".to_string(),
            bold: false,
            color: [255, 255, 255, 255],
            x: 10.0,
            y: 10.0,
            width: None,
            height: None,
            lock_aspect: true,
            rotation: 0.0,
            on_click: Some(crate::models::OverlayClickAction::CloseOverlay),
        });

        // Test image element
        overlay.elements.push(OverlayElement::Image {
            id: "img_1".to_string(),
            path: "C:/test/sample.png".to_string(),
            x: 5.0,
            y: 5.0,
            width: 32.0,
            height: 32.0,
            opacity: 0.9,
            lock_aspect: true,
            rotation: 0.0,
            on_click: None,
        });

        // Test shape element
        overlay.elements.push(OverlayElement::Shape {
            id: "shape_1".to_string(),
            shape_type: crate::models::ShapeKind::Rectangle,
            x: 20.0,
            y: 20.0,
            width: 40.0,
            height: 40.0,
            thickness: 2.0,
            color: [255, 0, 0, 255],
            filled: true,
            fill_color: [0, 255, 0, 100],
            corner_radius: 6.0,
            lock_aspect: true,
            rotation: 0.0,
            on_click: None,
        });

        assert_eq!(overlay.elements.len(), 4);
    }

    #[test]
    fn test_overlay_modal_tab_cycling() {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);

        let mut state = GuiState::new();
        state.open_overlay_studio("overlay_telemetry");

        // Overlay telemetry starts with elements
        let num_elements = state.overlay_studio.as_ref().unwrap().overlay.elements.len();
        assert!(num_elements > 1);

        // Frame 1: Initial render, no key pressed
        let mut out = ctx.run_ui(Default::default(), |ctx| {
            render_overlay_modal(ctx, &mut state);
        });
        out.textures_delta.clear();

        // Frame 2: Press Tab -> Selects element 0
        let mut raw_input = egui::RawInput::default();
        raw_input.events.push(egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });

        let mut out = ctx.run_ui(raw_input, |ctx| {
            render_overlay_modal(ctx, &mut state);
        });
        out.textures_delta.clear();

        assert_eq!(
            state.overlay_studio.as_ref().unwrap().selected_element_idx,
            Some(0)
        );

        // Frame 3: Press Tab again -> Selects element 1
        let mut raw_input = egui::RawInput::default();
        raw_input.events.push(egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });

        let mut out = ctx.run_ui(raw_input, |ctx| {
            render_overlay_modal(ctx, &mut state);
        });
        out.textures_delta.clear();

        assert_eq!(
            state.overlay_studio.as_ref().unwrap().selected_element_idx,
            Some(1)
        );

        // Frame 4: Press Shift+Tab -> Selects element 0
        let mut raw_input = egui::RawInput::default();
        raw_input.events.push(egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        });

        let mut out = ctx.run_ui(raw_input, |ctx| {
            render_overlay_modal(ctx, &mut state);
        });
        out.textures_delta.clear();

        assert_eq!(
            state.overlay_studio.as_ref().unwrap().selected_element_idx,
            Some(0)
        );

        // Frame 5: Press Shift+Tab from 0 -> Wraps around to last element (num_elements - 1)
        let mut raw_input = egui::RawInput::default();
        raw_input.events.push(egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        });

        let mut out = ctx.run_ui(raw_input, |ctx| {
            render_overlay_modal(ctx, &mut state);
        });
        out.textures_delta.clear();

        assert_eq!(
            state.overlay_studio.as_ref().unwrap().selected_element_idx,
            Some(num_elements - 1)
        );
    }

    #[test]
    fn test_canvas_element_clamping_and_image_fit() {
        use crate::models::{OverlayDefinition, OverlayElement};

        let mut overlay = OverlayDefinition::default_crosshair();
        overlay.rect.width = 100.0;
        overlay.rect.height = 100.0;

        // 1. Proportional Image (lock_aspect: true)
        overlay.elements.push(OverlayElement::Image {
            id: "img_proportional".to_string(),
            path: "".to_string(),
            x: 50.0,
            y: 50.0,
            width: 200.0,
            height: 100.0, // 2:1 aspect ratio
            opacity: 1.0,
            lock_aspect: true,
            rotation: 0.0,
            on_click: None,
        });

        // 2. Freeform Image (lock_aspect: false)
        overlay.elements.push(OverlayElement::Image {
            id: "img_freeform".to_string(),
            path: "".to_string(),
            x: 80.0,
            y: 80.0,
            width: 150.0,
            height: 250.0,
            opacity: 1.0,
            lock_aspect: false,
            rotation: 0.0,
            on_click: None,
        });

        // 3. Progress bar overflowing canvas
        overlay.elements.push(OverlayElement::ProgressBar {
            id: "bar_1".to_string(),
            label: "Bar".to_string(),
            progress: 0.5,
            fill_color: [0, 255, 0, 255],
            bg_color: [0, 0, 0, 255],
            x: 90.0,
            y: 90.0,
            width: 300.0,
            height: 50.0,
            rotation: 0.0,
        });

        // 4. Text with large font overflowing canvas
        overlay.elements.push(OverlayElement::Text {
            id: "txt_large".to_string(),
            text: "FPS: 144".to_string(),
            font_size: 40.0,
            color: [255, 255, 255, 255],
            x: 40.0,
            y: 40.0,
            width: None,
            height: None,
            lock_aspect: true,
            bold: false,
            font_family: "Segoe UI".to_string(),
            rotation: 0.0,
            on_click: None,
        });

        // 5. Standard Text scaling
        overlay.elements.push(OverlayElement::Text {
            id: "txt_scaled".to_string(),
            text: "Title".to_string(),
            font_size: 20.0,
            color: [255, 255, 255, 255],
            x: 60.0,
            y: 60.0,
            width: None,
            height: None,
            lock_aspect: false,
            bold: true,
            font_family: "Segoe UI".to_string(),
            rotation: 0.0,
            on_click: None,
        });

        // 6. Proportional Shape with locked aspect ratio
        overlay.elements.push(OverlayElement::Shape {
            id: "shape_clamp".to_string(),
            shape_type: crate::models::ShapeKind::Rectangle,
            x: 80.0,
            y: 80.0,
            width: 200.0,
            height: 100.0, // 2:1 aspect ratio
            thickness: 2.0,
            color: [255, 255, 255, 255],
            filled: false,
            fill_color: [0, 0, 0, 0],
            corner_radius: 0.0,
            lock_aspect: true,
            rotation: 0.0,
            on_click: None,
        });

        clamp_elements_to_canvas(&mut overlay);

        // Verify Image 1: proportional shrink (width <= 100, height <= 50, x + width <= 100, y + height <= 100)
        if let OverlayElement::Image { width, height, x, y, .. } = &overlay.elements[1] {
            assert!(*width <= 100.0);
            assert!(*height <= 50.0);
            assert!(*x >= 0.0 && *x + *width <= 100.0);
            assert!(*y >= 0.0 && *y + *height <= 100.0);
        } else {
            panic!("Expected Image element");
        }

        // Verify Image 2: freeform shrink (width <= 100, height <= 100)
        if let OverlayElement::Image { width, height, x, y, .. } = &overlay.elements[2] {
            assert!(*width <= 100.0);
            assert!(*height <= 100.0);
            assert!(*x >= 0.0 && *x + *width <= 100.0);
            assert!(*y >= 0.0 && *y + *height <= 100.0);
        } else {
            panic!("Expected Image element");
        }

        // Verify Progress Bar: width <= 100, height <= 100, coordinates clamped
        if let OverlayElement::ProgressBar { width, height, x, y, .. } = &overlay.elements[3] {
            assert!(*width <= 100.0);
            assert!(*height <= 100.0);
            assert!(*x >= 0.0 && *x + *width <= 100.0);
            assert!(*y >= 0.0 && *y + *height <= 100.0);
        } else {
            panic!("Expected ProgressBar element");
        }

        // Verify Text: font_size scaled down so text fits within 100x100
        if let OverlayElement::Text { font_size, x, y, text, .. } = &overlay.elements[4] {
            assert!(*font_size < 40.0);
            let (ew, eh) = get_element_size(&OverlayElement::Text {
                id: "".to_string(),
                text: text.clone(),
                font_size: *font_size,
                color: [0; 4],
                x: 0.0,
                y: 0.0,
                width: None,
                height: None,
                lock_aspect: false,
                bold: false,
                font_family: "".to_string(),
                rotation: 0.0,
                on_click: None,
            });
            assert!(*x >= 0.0 && *x + ew <= 100.0);
            assert!(*y >= 0.0 && *y + eh <= 100.0);
        } else {
            panic!("Expected Text element");
        }

        // Verify Standard Text 5: clamped to canvas
        if let OverlayElement::Text { font_size, x, y, text, .. } = &overlay.elements[5] {
            let (ew, eh) = get_element_size(&OverlayElement::Text {
                id: "".to_string(),
                text: text.clone(),
                font_size: *font_size,
                color: [0; 4],
                x: 0.0,
                y: 0.0,
                width: None,
                height: None,
                lock_aspect: false,
                bold: false,
                font_family: "".to_string(),
                rotation: 0.0,
                on_click: None,
            });
            assert!(*x >= 0.0 && *x + ew <= 100.0);
            assert!(*y >= 0.0 && *y + eh <= 100.0);
        } else {
            panic!("Expected Scaled Text element");
        }

        // Verify Proportional Shape 6: clamped to canvas with preserved aspect ratio (width <= 100, height <= 50)
        if let OverlayElement::Shape { width, height, x, y, .. } = &overlay.elements[6] {
            assert!(*width <= 100.0);
            assert!(*height <= 50.0);
            assert!(*x >= 0.0 && *x + *width <= 100.0);
            assert!(*y >= 0.0 && *y + *height <= 100.0);
        } else {
            panic!("Expected Shape element");
        }
    }

    #[test]
    fn test_monitor_artboard_and_card_aspect_ratio_lock() {
        use crate::models::{OverlayAnchor, OverlayDefinition};

        // 1. Verify compute_overlay_screen_pos for different anchors
        let mut overlay = OverlayDefinition::default_macro_badge();
        overlay.rect.width = 400.0;
        overlay.rect.height = 200.0;
        overlay.rect.x = 20.0;
        overlay.rect.y = 30.0;

        // ScreenCenter: ((1920 - 400) / 2 + 20, (1080 - 200) / 2 + 30) = (780, 470)
        overlay.anchor = OverlayAnchor::ScreenCenter;
        let (pos_x, pos_y) = compute_overlay_screen_pos(&overlay, 1920.0, 1080.0);
        assert_eq!(pos_x, 780.0);
        assert_eq!(pos_y, 470.0);

        // TopLeft: (20, 30)
        overlay.anchor = OverlayAnchor::TopLeft;
        let (pos_x, pos_y) = compute_overlay_screen_pos(&overlay, 1920.0, 1080.0);
        assert_eq!(pos_x, 20.0);
        assert_eq!(pos_y, 30.0);

        // Custom: (20, 30)
        overlay.anchor = OverlayAnchor::Custom;
        let (pos_x, pos_y) = compute_overlay_screen_pos(&overlay, 1920.0, 1080.0);
        assert_eq!(pos_x, 20.0);
        assert_eq!(pos_y, 30.0);

        // TopRight: (1920 - 400 - 20, 30) = (1500, 30)
        overlay.anchor = OverlayAnchor::TopRight;
        let (pos_x, pos_y) = compute_overlay_screen_pos(&overlay, 1920.0, 1080.0);
        assert_eq!(pos_x, 1500.0);
        assert_eq!(pos_y, 30.0);

        // 2. Verify Card Aspect Ratio Lock calculation
        let card_lock_aspect = true;
        let aspect = overlay.rect.width / overlay.rect.height; // 400 / 200 = 2.0
        assert_eq!(aspect, 2.0);

        // Resizing width from 400 to 600 maintains aspect ratio -> height = 300
        let new_w = 600.0;
        let new_h = if card_lock_aspect {
            (new_w / aspect).round()
        } else {
            overlay.rect.height
        };
        assert_eq!(new_h, 300.0);

        // 3. Verify OverlayStudioState default camera pan & zoom, centered default card
        let mut state = GuiState::new();
        state.open_overlay_studio("overlay_custom");
        let studio = state.overlay_studio.as_ref().unwrap();
        assert!(!studio.card_lock_aspect);
        assert_eq!(studio.canvas_pan, egui::Vec2::ZERO);
        assert_eq!(studio.canvas_zoom, None);
        assert_eq!(studio.overlay.anchor, OverlayAnchor::ScreenCenter);
        assert_eq!(studio.overlay.rect.width, 440.0);
        assert_eq!(studio.overlay.rect.height, 200.0);

        // Also verify preset macro badge retains its dedicated top-right geometry
        state.open_overlay_studio("overlay_macro_badge");
        let studio_badge = state.overlay_studio.as_ref().unwrap();
        assert_eq!(studio_badge.overlay.anchor, OverlayAnchor::TopRight);
        assert_eq!(studio_badge.overlay.rect.width, 220.0);
        assert_eq!(studio_badge.overlay.rect.height, 44.0);

        // 7. Verify 4-way auto-panning calculation logic
        let canvas_viewport = Rect::from_min_size(pos2(100.0, 100.0), vec2(800.0, 600.0));
        let edge_zone = 70.0;

        // Pointer near right edge -> pan step X is negative (slides camera right)
        let ptr_right = pos2(canvas_viewport.right() - 20.0, canvas_viewport.center().y);
        let factor_r = ((ptr_right.x - (canvas_viewport.right() - edge_zone)) / edge_zone).clamp(0.0, 1.5);
        let pan_x = -(3.0 + 8.0 * factor_r);
        assert!(pan_x < -3.0);

        // Pointer near left edge -> pan step X is positive (slides camera left)
        let ptr_left = pos2(canvas_viewport.left() + 20.0, canvas_viewport.center().y);
        let factor_l = ((canvas_viewport.left() + edge_zone - ptr_left.x) / edge_zone).clamp(0.0, 1.5);
        let pan_l = 3.0 + 8.0 * factor_l;
        assert!(pan_l > 3.0);

        // Pointer near bottom edge -> pan step Y is negative (slides camera down)
        let ptr_bottom = pos2(canvas_viewport.center().x, canvas_viewport.bottom() - 15.0);
        let factor_b = ((ptr_bottom.y - (canvas_viewport.bottom() - edge_zone)) / edge_zone).clamp(0.0, 1.5);
        let pan_y = -(3.0 + 8.0 * factor_b);
        assert!(pan_y < -3.0);

        // Pointer near top edge -> pan step Y is positive (slides camera up)
        let ptr_top = pos2(canvas_viewport.center().x, canvas_viewport.top() + 15.0);
        let factor_t = ((canvas_viewport.top() + edge_zone - ptr_top.y) / edge_zone).clamp(0.0, 1.5);
        let pan_t = 3.0 + 8.0 * factor_t;
        assert!(pan_t > 3.0);

        // Safeguard: when an element is selected (selected_element_idx is Some), camera auto-pan is disabled
        let selected_element_idx = Some(0usize);
        let pan_step = if selected_element_idx.is_none() {
            vec2(pan_x, pan_y)
        } else {
            Vec2::ZERO
        };
        assert_eq!(pan_step, Vec2::ZERO);

        // 4. Verify max_card_dimensions and clamp_card_to_monitor
        let mut custom_overlay = OverlayDefinition::default_macro_badge();
        custom_overlay.anchor = OverlayAnchor::Custom;
        custom_overlay.rect.x = 1500.0;
        custom_overlay.rect.y = 800.0;
        custom_overlay.rect.width = 600.0;
        custom_overlay.rect.height = 400.0;
        let (max_w, max_h) = max_card_dimensions(&custom_overlay, 1920.0, 1080.0);
        assert_eq!(max_w, 420.0); // 1920 - 1500
        assert_eq!(max_h, 280.0); // 1080 - 800
        clamp_card_to_monitor(&mut custom_overlay, 1920.0, 1080.0);
        assert_eq!(custom_overlay.rect.width, 420.0);
        assert_eq!(custom_overlay.rect.height, 280.0);

        // 5. Verify anchor coordinate sanitization for stale absolute values
        let mut center_overlay = OverlayDefinition::default_macro_badge();
        center_overlay.anchor = OverlayAnchor::ScreenCenter;
        center_overlay.rect.x = 1200.0; // Stale absolute screen coordinate
        center_overlay.rect.y = 500.0;
        let (pos_x, pos_y) = compute_overlay_screen_pos(&center_overlay, 1920.0, 1080.0);
        // Stale 1200.0 (> 1920 * 0.35 = 672) is treated as 0.0 offset -> exactly centered!
        assert_eq!(pos_x, (1920.0 - center_overlay.rect.width) * 0.5);
        assert_eq!(pos_y, (1080.0 - center_overlay.rect.height) * 0.5);

        // 6. Verify calibration synchronization matching: when last_synced_pos matches, no overwrite occurs
        let mut studio = state.overlay_studio.unwrap();
        studio.is_calibrating = true;
        studio.overlay.rect.x = 750.0;
        studio.overlay.rect.y = 350.0;
        studio.last_synced_pos = Some((750.0, 350.0));
        let daemon_pos = (750.0, 350.0);
        let pos_differs_from_last_synced = studio.last_synced_pos.is_none_or(|(lx, ly)| (lx - daemon_pos.0).abs() > 0.5 || (ly - daemon_pos.1).abs() > 0.5);
        assert!(!pos_differs_from_last_synced, "Position matching last_synced_pos must not trigger overwrite");
    }
}
