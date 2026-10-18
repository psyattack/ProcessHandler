//! Design tokens, typography and visuals matching the high-precision studio aesthetic:
//! hard geometric edges, hairline rules, Archivo display headings, Inter body text,
//! and Geist Mono labels and HUD widgets.

use std::sync::Arc;
use egui::epaint::text::VariationCoords;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, FontTweak, Stroke, TextStyle, Visuals};
pub use crate::config::AppTheme;

/// Height of buttons, inputs and filter cells.
pub const CONTROL_H: f32 = 34.0;

/// The precision `.hud-label`: 11 px Geist Mono 500, uppercase, 0.15 em letter-spacing.
pub const HUD_SIZE: f32 = 11.0;
pub const HUD_SPACING: f32 = HUD_SIZE * 0.15;

/// Button labels: 11 px Geist Mono 700, uppercase, 0.12 em letter-spacing.
pub const BUTTON_SPACING: f32 = HUD_SIZE * 0.12;

/// Height of the top navigation/title bar.
pub const HEADER_H: f32 = 48.0;

/// Height of the bottom telemetry/status bar.
pub const FOOTER_H: f32 = 34.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub dark: bool,
    /// Page and card surface.
    pub bg: Color32,
    /// Recessed wells, badges and row hovers.
    pub sunken: Color32,
    /// Standard body text.
    pub ink: Color32,
    /// High-emphasis headings and labels.
    pub ink_strong: Color32,
    /// Secondary text.
    pub muted: Color32,
    /// Faint tertiary text, separators, indices.
    pub faint: Color32,
    /// Subtle hairlines (15% opacity).
    pub line: Color32,
    /// Emphasized hairlines / active outlines (40% opacity).
    pub line_strong: Color32,
    /// Primary brand blue (for surfaces).
    pub accent: Color32,
    /// Primary brand blue (for text/icons).
    pub accent_ink: Color32,
    /// Inversion fill (solid white in dark mode).
    pub invert_bg: Color32,
    /// Inversion ink (near black in dark mode).
    pub invert_fg: Color32,
    /// Status emerald green.
    pub good: Color32,
    /// Status amber warning.
    pub warn: Color32,
    /// Status crimson danger.
    pub bad: Color32,
}

const fn ink_alpha(rgb: u32, alpha: u8) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color32::from_rgba_unmultiplied_const(r, g, b, alpha)
}

const fn hex(rgb: u32) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

impl Tokens {
    pub const DARK: Self = Self {
        dark: true,
        bg: hex(0x121316),
        sunken: hex(0x060607),
        ink: hex(0xf2f1ee),
        ink_strong: hex(0xffffff),
        muted: ink_alpha(0xf2f1ee, 153),
        faint: ink_alpha(0xf2f1ee, 97),
        line: ink_alpha(0xf2f1ee, 38),
        line_strong: ink_alpha(0xf2f1ee, 102),
        accent: hex(0x2d81ff),
        accent_ink: hex(0x74aaff),
        invert_bg: hex(0xffffff),
        invert_fg: hex(0x0b0b0c),
        good: hex(0x3fd6b8),
        warn: hex(0xffc14d),
        bad: hex(0xff6259),
    };

    pub const LIGHT: Self = Self {
        dark: false,
        bg: hex(0xf2f1ee),
        sunken: hex(0xe9e8e4),
        ink: hex(0x101014),
        ink_strong: hex(0x000000),
        muted: ink_alpha(0x101014, 158),
        faint: ink_alpha(0x101014, 107),
        line: ink_alpha(0x101014, 41),
        line_strong: ink_alpha(0x101014, 102),
        accent: hex(0x2d81ff),
        accent_ink: hex(0x1659c4),
        invert_bg: hex(0x101014),
        invert_fg: hex(0xf2f1ee),
        good: hex(0x0a7563),
        warn: hex(0x8a5800),
        bad: hex(0xb3261e),
    };

    pub const CHOCOLATE: Self = Self {
        dark: true,
        bg: hex(0x201714),
        sunken: hex(0x130d0a),
        ink: hex(0xf2e8dc),
        ink_strong: hex(0xfff7ee),
        muted: ink_alpha(0xf2e8dc, 153),
        faint: ink_alpha(0xf2e8dc, 97),
        line: ink_alpha(0xf2e8dc, 38),
        line_strong: ink_alpha(0xf2e8dc, 102),
        accent: hex(0xd97736),
        accent_ink: hex(0xf29652),
        invert_bg: hex(0xf2e8dc),
        invert_fg: hex(0x1c1310),
        good: hex(0x45cf9c),
        warn: hex(0xfcb447),
        bad: hex(0xf25f52),
    };

    pub fn get(ctx: &egui::Context) -> Self {
        let theme = ctx.data(|d| d.get_temp::<AppTheme>(egui::Id::new("app_theme"))).unwrap_or(AppTheme::Dark);
        match theme {
            AppTheme::Dark => Self::DARK,
            AppTheme::Light => Self::LIGHT,
            AppTheme::Chocolate => Self::CHOCOLATE,
        }
    }

    pub fn hairline(&self) -> Stroke {
        Stroke::new(1.0, self.line)
    }

}


pub fn shadow(t: &Tokens) -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if t.dark { 140 } else { 36 }),
    }
}

pub fn scrim(t: &Tokens, amount: f32) -> Color32 {
    let alpha = if t.dark { 170.0 } else { 105.0 } * amount.clamp(0.0, 1.0);
    Color32::from_black_alpha(alpha.round() as u8)
}

/// Archivo 620 display font for headings and branding.
pub fn display() -> FontFamily {
    FontFamily::Name("display".into())
}

/// Inter 500 font for medium body copy.
pub fn medium() -> FontFamily {
    FontFamily::Name("medium".into())
}

/// Inter 600 font for semibold body copy.
pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Geist Mono 500 for precision labels and tags.
pub fn mono() -> FontFamily {
    FontFamily::Name("mono".into())
}

/// Geist Mono 700 for button titles and active counts.
pub fn mono_bold() -> FontFamily {
    FontFamily::Name("mono-bold".into())
}

/// Phosphor icon font family.
pub fn phosphor() -> FontFamily {
    FontFamily::Name("phosphor".into())
}

type Face<'a> = (&'a str, &'static [u8], &'a [([u8; 4], f32)], FontFamily);

pub fn install_fonts(ctx: &egui::Context) {
    const INTER: &[u8] = include_bytes!("../../assets/fonts/Inter-Variable.ttf");
    const ARCHIVO: &[u8] = include_bytes!("../../assets/fonts/Archivo-Variable.ttf");
    const GEIST_MONO: &[u8] = include_bytes!("../../assets/fonts/GeistMono-Variable.ttf");

    let mut fonts = FontDefinitions::default();
    
    // Register Phosphor Icons into the font system
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts.families.insert(phosphor(), vec!["phosphor".into()]);

    let fallback: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mono_fallback: Vec<String> = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();

    let faces: [Face<'_>; 7] = [
        ("Inter", INTER, &[(*b"wght", 400.0), (*b"opsz", 14.0)], FontFamily::Proportional),
        ("Inter-Medium", INTER, &[(*b"wght", 500.0), (*b"opsz", 14.0)], medium()),
        ("Inter-SemiBold", INTER, &[(*b"wght", 600.0), (*b"opsz", 14.0)], semibold()),
        ("Archivo-Display", ARCHIVO, &[(*b"wght", 620.0), (*b"wdth", 118.0)], display()),
        ("GeistMono", GEIST_MONO, &[(*b"wght", 400.0)], FontFamily::Monospace),
        ("GeistMono-Medium", GEIST_MONO, &[(*b"wght", 500.0)], mono()),
        ("GeistMono-Bold", GEIST_MONO, &[(*b"wght", 700.0)], mono_bold()),
    ];

    for (name, bytes, coords, family) in faces {
        let tweak = FontTweak {
            coords: VariationCoords::new(coords.iter().copied()),
            ..Default::default()
        };
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes).tweak(tweak)));
        
        let mut stack = vec![name.to_owned()];
        if family == FontFamily::Monospace || family == mono() || family == mono_bold() {
            stack.extend(mono_fallback.iter().cloned());
        } else {
            stack.extend(fallback.iter().cloned());
        }
        if !stack.iter().any(|f| f == "phosphor") {
            stack.push("phosphor".into());
        }
        fonts.families.insert(family, stack);
    }

    ctx.set_fonts(fonts);
}

fn visuals(t: &Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.bg;
    v.window_fill = t.bg;
    v.extreme_bg_color = t.sunken;
    v.text_edit_bg_color = Some(t.sunken);
    v.faint_bg_color = t.sunken;
    v.code_bg_color = t.sunken;
    v.window_stroke = Stroke::new(1.0, t.line_strong);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.window_shadow = shadow(t);
    v.popup_shadow = shadow(t);
    v.override_text_color = Some(t.ink);
    v.hyperlink_color = t.accent_ink;
    v.selection.bg_fill = t.accent;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.warn_fg_color = t.warn;
    v.error_fg_color = t.bad;

    for (w, fill, stroke, fg) in [
        (&mut v.widgets.noninteractive, t.bg, t.line, t.muted),
        (&mut v.widgets.inactive, t.bg, t.line, t.ink),
        (&mut v.widgets.hovered, t.sunken, t.line_strong, t.ink_strong),
        (&mut v.widgets.active, t.sunken, t.line_strong, t.ink_strong),
        (&mut v.widgets.open, t.sunken, t.line_strong, t.ink_strong),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, stroke);
        w.corner_radius = CornerRadius::ZERO;
        w.fg_stroke = Stroke::new(1.0, fg);
        w.expansion = 0.0;
    }
    v
}

pub fn apply_theme(ctx: &egui::Context, theme: AppTheme) {
    let tokens = match theme {
        AppTheme::Dark => Tokens::DARK,
        AppTheme::Light => Tokens::LIGHT,
        AppTheme::Chocolate => Tokens::CHOCOLATE,
    };
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("app_theme"), theme));
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&Tokens::DARK));
    ctx.set_visuals_of(egui::Theme::Light, visuals(&Tokens::LIGHT));
    ctx.set_visuals(visuals(&tokens));
    if tokens.dark {
        ctx.set_theme(egui::ThemePreference::Dark);
    } else {
        ctx.set_theme(egui::ThemePreference::Light);
    }

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.icon_spacing = 8.0;
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = CONTROL_H;
        style.spacing.menu_margin = egui::Margin::same(4);
        style.spacing.window_margin = egui::Margin::same(4);
        style.text_styles = [
            (TextStyle::Heading, FontId::new(22.0, display())),
            (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(12.5, medium())),
            (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        ]
        .into();
    });
}
