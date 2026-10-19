//! Changelog & Release Notes Modal:
//! Centered studio canvas for displaying GitHub release notes
//! with HTML & Markdown parsing, clickable links, and one-click update installation.

use egui::{
    CornerRadius, Id, Order, Rect, RichText, Sense, Stroke, emath::easing, pos2, vec2, Ui,
};
use crate::gui::components::widgets::{self, Glyph, Kind};
use crate::gui::state::GuiState;
use crate::gui::theme::{self, Tokens};

pub fn render_changelog_modal(ctx: &egui::Context, state: &mut GuiState) {
    let t = Tokens::get(ctx);
    let is_open = state.show_changelog_modal && state.available_update.is_some();

    // Close on Escape key
    if is_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.show_changelog_modal = false;
    }

    let anim = ctx.animate_bool_with_time_and_easing(
        Id::new("changelog-modal-anim"),
        is_open,
        0.20,
        easing::cubic_out,
    );

    if anim <= 0.001 && !is_open {
        return;
    }

    let Some(info) = state.available_update.clone() else { return };
    let loc = state.loc();
    let full = ctx.content_rect();

    // 1. Scrim overlay
    let scrim_alpha = (anim * 195.0) as u8;
    egui::Area::new(Id::new("changelog-modal-scrim"))
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
                state.show_changelog_modal = false;
            }
        });

    // 2. Centered Modal Box geometry
    let modal_w = (full.width() * 0.76).clamp(580.0, 920.0).min(full.width() - 32.0);
    let modal_h = (full.height() * 0.78).clamp(380.0, 720.0).min(full.height() - 48.0);
    let center = full.center();

    let scale = 0.95 + 0.05 * anim;
    let y_shift = (1.0 - anim) * 12.0;
    let modal_rect = Rect::from_center_size(
        pos2(center.x, center.y + y_shift),
        vec2(modal_w * scale, modal_h * scale),
    );

    let header_h = 56.0;
    let header_rect = Rect::from_min_size(modal_rect.min, vec2(modal_rect.width(), header_h));
    let body_rect = Rect::from_min_max(
        pos2(modal_rect.left(), header_rect.bottom()),
        modal_rect.max,
    );

    let mut close_clicked = false;
    let mut update_clicked = false;

    // 3. Modal Canvas Window
    egui::Area::new(Id::new("changelog-modal-window"))
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

                p.rect_filled(header_rect, CornerRadius::ZERO, t.bg);
                p.hline(header_rect.x_range(), header_rect.bottom(), t.hairline());

                p.rect_filled(body_rect, CornerRadius::ZERO, t.sunken);

                // CRITICAL: Paint outer border stroke AFTER all inner fills so left, right, and bottom sides are consistently visible
                p.rect_stroke(
                    modal_rect,
                    CornerRadius::same(2),
                    Stroke::new(1.0, t.line_strong),
                    egui::StrokeKind::Inside,
                );

                // Top accent stripe (blue) painted AFTER outer border stroke so white does not paint over it
                p.hline(modal_rect.x_range(), modal_rect.top() + 1.0, Stroke::new(2.5, t.accent_ink));
            }

            // --- 1. HEADER ZONE ---
            ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.add_space(20.0);

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        widgets::eyebrow(ui, "01", loc.changelog_eyebrow);
                        ui.label(
                            RichText::new(&info.name)
                                .family(theme::semibold())
                                .size(16.0)
                                .color(t.ink_strong),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if widgets::glyph_button(ui, Glyph::Close, loc.changelog_btn_close).clicked() {
                            close_clicked = true;
                        }

                        ui.add_space(8.0);

                        // Update Now Button
                        let is_busy = state.is_downloading_update;
                        let update_btn_label = if is_busy {
                            format!("{} {:.0}%", loc.changelog_btn_update, (state.update_download_progress * 100.0).clamp(0.0, 100.0))
                        } else {
                            loc.changelog_btn_update.to_string()
                        };

                        ui.add_enabled_ui(!is_busy, |ui| {
                            if widgets::button(ui, &update_btn_label, Kind::Primary, 0.0).clicked() {
                                update_clicked = true;
                            }
                        });

                        if !info.html_url.is_empty() {
                            ui.add_space(8.0);
                            let open_label = format!("{} {}", egui_phosphor::regular::ARROW_SQUARE_OUT, loc.changelog_open_github);
                            if widgets::button(ui, &open_label, Kind::Secondary, 0.0).clicked() {
                                ui.ctx().open_url(egui::OpenUrl::same_tab(&info.html_url));
                            }
                        }

                        // File size info
                        let size_mb = info.asset_size as f64 / 1_048_576.0;
                        if size_mb > 0.05 {
                            ui.add_space(10.0);
                            widgets::hud(ui, &format!("{:.1} MB", size_mb), t.faint);
                        }
                    });
                });
            });

            // --- 2. BODY ZONE (Rich HTML & Markdown) ---
            let inner_body = body_rect.shrink2(vec2(22.0, 16.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner_body), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("changelog_scroll_area")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 10.0;

                        let body_text = info.body.trim();
                        if body_text.is_empty() {
                            widgets::hud(ui, loc.changelog_no_notes, t.muted);
                        } else {
                            render_rich_changelog(ui, body_text, &t);
                        }
                    });
            });
        });

    if close_clicked {
        state.show_changelog_modal = false;
    }

    if update_clicked {
        state.show_changelog_modal = false;
        state.start_update(info);
    }
}

/// Renders changelog content supporting HTML tags (<p>, <br>, <h1>-<h6>, <b>, <i>, <code>, <a>, <ul>, <li>)
/// and Markdown syntax (#, ##, bullets, `code`, **bold**, links).
fn render_rich_changelog(ui: &mut Ui, text: &str, t: &Tokens) {
    let clean = text
        .replace("\r\n", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");

    let mut in_code_block = false;
    let mut code_block_buf = String::new();

    for raw_line in clean.lines() {
        let line = raw_line.trim_end();

        // 1. Code Block Fence (Markdown or HTML)
        if line.starts_with("```") || line.contains("<pre>") || line.contains("</pre>") {
            if in_code_block {
                // End code block
                render_code_block(ui, &code_block_buf, t);
                code_block_buf.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block_buf.push_str(line);
            code_block_buf.push('\n');
            continue;
        }

        // 2. Horizontal divider
        if line == "---" || line == "***" || line == "___" || line.contains("<hr>") || line.contains("<hr/>") {
            widgets::hrule(ui);
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            ui.add_space(4.0);
            continue;
        }

        // 3. Headings (Markdown: #, ##, ### or HTML: <h1>, <h2>, <h3>)
        if trimmed.starts_with("### ") || trimmed.starts_with("<h4>") || trimmed.starts_with("<h3>") {
            let title = strip_tags(trimmed.trim_start_matches("### "));
            ui.add_space(4.0);
            ui.label(
                RichText::new(title)
                    .family(theme::semibold())
                    .size(14.0)
                    .color(t.ink_strong),
            );
            continue;
        }

        if trimmed.starts_with("## ") || trimmed.starts_with("<h2>") {
            let title = strip_tags(trimmed.trim_start_matches("## "));
            ui.add_space(6.0);
            ui.label(
                RichText::new(title)
                    .family(theme::semibold())
                    .size(16.0)
                    .color(t.ink_strong),
            );
            continue;
        }

        if trimmed.starts_with("# ") || trimmed.starts_with("<h1>") {
            let title = strip_tags(trimmed.trim_start_matches("# "));
            ui.add_space(8.0);
            ui.label(
                RichText::new(title)
                    .family(theme::display())
                    .size(18.0)
                    .color(t.ink_strong),
            );
            continue;
        }

        // 4. Bullet list items
        let is_bullet = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("<li>")
            || trimmed.starts_with("• ");

        if is_bullet {
            let item_text = trimmed
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim_start_matches("• ")
                .trim_start_matches("<li>");

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new("•").family(theme::mono_bold()).color(t.accent_ink));
                render_inline_markup(ui, item_text, t);
            });
            continue;
        }

        // 5. Standard paragraph / line
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            render_inline_markup(ui, trimmed, t);
        });
    }

    if in_code_block && !code_block_buf.is_empty() {
        render_code_block(ui, &code_block_buf, t);
    }
}

/// Renders a framed monospace code block.
fn render_code_block(ui: &mut Ui, code: &str, t: &Tokens) {
    egui::Frame::new()
        .fill(t.sunken)
        .stroke(t.hairline())
        .corner_radius(CornerRadius::ZERO)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(code.trim_end())
                    .family(theme::mono())
                    .size(12.0)
                    .color(t.ink),
            );
        });
}

/// Parses inline HTML tags (<a>, <b>, <strong>, <i>, <code>) and Markdown (**bold**, `code`, links).
fn render_inline_markup(ui: &mut Ui, line: &str, t: &Tokens) {
    let mut rest = line;

    while !rest.is_empty() {
        // Find next tag or markup token
        if let Some(link_start) = rest.find("<a ") {
            if link_start > 0 {
                render_simple_text(ui, &rest[..link_start], t);
            }
            if let Some(link_end) = rest[link_start..].find("</a>") {
                let tag_slice = &rest[link_start..link_start + link_end + 4];
                render_html_link(ui, tag_slice, t);
                rest = &rest[link_start + link_end + 4..];
                continue;
            } else {
                render_simple_text(ui, &rest[link_start..], t);
                break;
            }
        }

        // Markdown link [text](url)
        if let Some(bracket_start) = rest.find('[') {
            if let Some(bracket_end) = rest[bracket_start..].find(']') {
                let text_inside = &rest[bracket_start + 1..bracket_start + bracket_end];
                let after_bracket = &rest[bracket_start + bracket_end + 1..];
                if after_bracket.starts_with('(') {
                    if let Some(paren_end) = after_bracket.find(')') {
                        let url_inside = &after_bracket[1..paren_end];
                        if bracket_start > 0 {
                            render_simple_text(ui, &rest[..bracket_start], t);
                        }
                        if ui.link(RichText::new(text_inside).color(t.accent_ink)).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::same_tab(url_inside));
                        }
                        rest = &after_bracket[paren_end + 1..];
                        continue;
                    }
                }
            }
        }

        // Raw URLs (https://... or http://...)
        if let Some(http_pos) = rest.find("http://").or_else(|| rest.find("https://")) {
            if http_pos > 0 {
                render_simple_text(ui, &rest[..http_pos], t);
            }
            let url_rest = &rest[http_pos..];
            let end_pos = url_rest
                .find(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == ')' || c == ']' || c == '"')
                .unwrap_or(url_rest.len());
            let url = &url_rest[..end_pos];
            if ui.link(RichText::new(url).color(t.accent_ink)).clicked() {
                ui.ctx().open_url(egui::OpenUrl::same_tab(url));
            }
            rest = &url_rest[end_pos..];
            continue;
        }

        // Bold or strong tags
        if let Some(bold_start) = rest.find("<b>").or_else(|| rest.find("<strong>")) {
            let is_b = rest[bold_start..].starts_with("<b>");
            let tag_len = if is_b { 3 } else { 8 };
            let close_tag = if is_b { "</b>" } else { "</strong>" };

            if bold_start > 0 {
                render_simple_text(ui, &rest[..bold_start], t);
            }
            if let Some(close_pos) = rest[bold_start + tag_len..].find(close_tag) {
                let bold_text = &rest[bold_start + tag_len..bold_start + tag_len + close_pos];
                ui.label(RichText::new(bold_text).family(theme::semibold()).color(t.ink_strong));
                rest = &rest[bold_start + tag_len + close_pos + close_tag.len()..];
                continue;
            }
        }

        // Markdown **bold**
        if let Some(bold_start) = rest.find("**") {
            if let Some(bold_end) = rest[bold_start + 2..].find("**") {
                if bold_start > 0 {
                    render_simple_text(ui, &rest[..bold_start], t);
                }
                let bold_text = &rest[bold_start + 2..bold_start + 2 + bold_end];
                ui.label(RichText::new(bold_text).family(theme::semibold()).color(t.ink_strong));
                rest = &rest[bold_start + 2 + bold_end + 2..];
                continue;
            }
        }

        // Monospace `code` or <code>
        if let Some(code_start) = rest.find("<code>").or_else(|| rest.find('`')) {
            let is_tag = rest[code_start..].starts_with("<code>");
            let open_len = if is_tag { 6 } else { 1 };
            let close_token = if is_tag { "</code>" } else { "`" };

            if code_start > 0 {
                render_simple_text(ui, &rest[..code_start], t);
            }
            if let Some(close_pos) = rest[code_start + open_len..].find(close_token) {
                let code_text = &rest[code_start + open_len..code_start + open_len + close_pos];
                ui.label(
                    RichText::new(code_text)
                        .family(theme::mono())
                        .size(11.5)
                        .color(t.accent_ink)
                        .background_color(t.sunken),
                );
                rest = &rest[code_start + open_len + close_pos + close_token.len()..];
                continue;
            }
        }

        // Fallback for remaining text
        render_simple_text(ui, rest, t);
        break;
    }
}

/// Renders plain text with stripped residual HTML tags.
fn render_simple_text(ui: &mut Ui, s: &str, t: &Tokens) {
    let clean = strip_tags(s);
    if !clean.is_empty() {
        ui.label(RichText::new(clean).color(t.ink).size(13.0));
    }
}

/// Parses `<a href="...">text</a>` and renders as a clickable link.
fn render_html_link(ui: &mut Ui, html: &str, t: &Tokens) {
    let href = if let Some(h_start) = html.find("href=\"") {
        let after = &html[h_start + 6..];
        after.find('"').map(|h_end| &after[..h_end])
    } else if let Some(h_start) = html.find("href='") {
        let after = &html[h_start + 6..];
        after.find('\'').map(|h_end| &after[..h_end])
    } else {
        None
    };

    let inner = if let Some(tag_close) = html.find('>') {
        if let Some(end_tag) = html.rfind("</a>") {
            if tag_close < end_tag {
                &html[tag_close + 1..end_tag]
            } else {
                html
            }
        } else {
            html
        }
    } else {
        html
    };

    let link_text = strip_tags(inner);
    if let Some(url) = href {
        if ui.link(RichText::new(link_text).color(t.accent_ink)).clicked() {
            ui.ctx().open_url(egui::OpenUrl::same_tab(url));
        }
    } else {
        ui.label(RichText::new(link_text).color(t.accent_ink));
    }
}

/// Helper to strip basic HTML tags from a string.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;

    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("<p>Hello <b>World</b>!</p>"), "Hello World!");
        assert_eq!(strip_tags("<a href=\"https://github.com\">Click here</a>"), "Click here");
        assert_eq!(strip_tags("Plain text with no tags"), "Plain text with no tags");
        assert_eq!(strip_tags("<code>let x = 10;</code>"), "let x = 10;");
    }
}
