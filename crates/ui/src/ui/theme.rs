use iced::theme::Palette as IcedPalette;
use iced::{Color, Shadow, Theme as IcedTheme, Vector};

/// Semantic colors shared by every component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub background: Color,
    pub foreground: Color,
    pub card: Color,
    pub card_foreground: Color,
    pub popover: Color,
    pub popover_foreground: Color,
    pub primary: Color,
    pub primary_foreground: Color,
    pub secondary: Color,
    pub secondary_foreground: Color,
    pub muted: Color,
    pub muted_foreground: Color,
    pub accent: Color,
    pub accent_foreground: Color,
    pub brand: Color,
    pub brand_foreground: Color,
    pub brand_background: Color,
    pub brand_line: Color,
    pub destructive: Color,
    pub destructive_foreground: Color,
    pub destructive_background: Color,
    pub destructive_line: Color,
    pub destructive_dot: Color,
    pub border: Color,
    pub control_line: Color,
    pub input: Color,
    pub ring: Color,
    pub success: Color,
    pub success_foreground: Color,
    pub success_background: Color,
    pub success_line: Color,
    pub success_dot: Color,
    pub warning: Color,
    pub warning_foreground: Color,
    pub warning_background: Color,
    pub warning_line: Color,
    pub warning_dot: Color,
    pub avatar: Color,
    pub avatar_foreground: Color,
    pub toast_background: Color,
    pub toast_foreground: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radius {
    pub chip: f32,
    pub row: f32,
    pub button: f32,
    pub card: f32,
    pub modal: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Typography {
    pub display: f32,
    pub screen_title: f32,
    pub section_title: f32,
    pub pane_header: f32,
    pub body: f32,
    pub list: f32,
    pub caption: f32,
    pub machine: f32,
    pub meta: f32,
    pub meta_compact: f32,
    pub field_label: f32,
    pub nav_label: f32,
    pub badge: f32,
}

/// Translucent surface roles. Blur remains renderer-owned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glass {
    pub thin: Color,
    pub regular: Color,
    pub sheet: Color,
}

/// Canonical single-shadow roles plus the two-layer application-window shadow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Elevation {
    pub popover: Shadow,
    pub toast: Shadow,
    pub modal: Shadow,
    pub app_window: [Shadow; 2],
}

/// Application-owned design tokens consumed through semantic roles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub palette: Palette,
    pub radius: Radius,
    pub spacing: Spacing,
    pub typography: Typography,
    pub glass: Glass,
    pub elevation: Elevation,
}

pub const LIGHT: Theme = Theme {
    name: "Ducktape Light",
    palette: Palette {
        background: hex(0xfdfdfb),
        foreground: hex(0x2c2b27),
        card: hex(0xffffff),
        card_foreground: hex(0x2c2b27),
        popover: hex(0xffffff),
        popover_foreground: hex(0x2c2b27),
        primary: hex(0x26251f),
        primary_foreground: Color::WHITE,
        secondary: Color::WHITE,
        secondary_foreground: hex(0x5e5c55),
        muted: hex(0xf6f5f2),
        muted_foreground: hex(0x6b6962),
        accent: hex(0xf3f2ef),
        accent_foreground: hex(0x3f3e39),
        brand: hex(0xa05a3c),
        brand_foreground: Color::WHITE,
        brand_background: hex(0xf9f1ea),
        brand_line: hex(0xe7d2c4),
        destructive: hex(0xb8544c),
        destructive_foreground: Color::WHITE,
        destructive_background: hex(0xfdf4f3),
        destructive_line: hex(0xefd6d3),
        destructive_dot: hex(0xe0655c),
        border: hex(0xe7e6e2),
        control_line: hex(0xe0dfd7),
        input: hex(0x8a8983),
        ring: hex(0x26251f),
        success: hex(0x5f9e74),
        success_foreground: hex(0x151410),
        success_background: hex(0xeef5f0),
        success_line: hex(0xcfe3d7),
        success_dot: hex(0x5cb45f),
        warning: hex(0xa07b32),
        warning_foreground: hex(0x151410),
        warning_background: hex(0xfbf4e6),
        warning_line: hex(0xecdcae),
        warning_dot: hex(0xe3b443),
        avatar: hex(0xd2d0c7),
        avatar_foreground: hex(0x4f4d47),
        toast_background: hex(0x26251f),
        toast_foreground: hex(0xf3f1ea),
    },
    radius: RADIUS,
    spacing: SPACING,
    typography: TYPOGRAPHY,
    glass: GLASS,
    elevation: ELEVATION,
};

pub const DARK: Theme = Theme {
    name: "Ducktape Dark",
    palette: Palette {
        background: hex(0x1b1a17),
        foreground: hex(0xeceae4),
        card: hex(0x1b1a17),
        card_foreground: hex(0xeceae4),
        popover: hex(0x1b1a17),
        popover_foreground: hex(0xeceae4),
        primary: hex(0xecebe5),
        primary_foreground: hex(0x1b1a17),
        secondary: hex(0x26251f),
        secondary_foreground: hex(0xeceae4),
        muted: hex(0x151410),
        muted_foreground: hex(0x9f9c95),
        accent: hex(0x2b2a25),
        accent_foreground: hex(0xeceae4),
        brand: hex(0xc87552),
        brand_foreground: hex(0x1b1a17),
        brand_background: hex(0x35231c),
        brand_line: hex(0x68402f),
        destructive: hex(0xd4655a),
        destructive_foreground: hex(0x1b1a17),
        destructive_background: hex(0x351d1b),
        destructive_line: hex(0x713b36),
        destructive_dot: hex(0xd4655a),
        border: hex(0x2e2d27),
        control_line: hex(0x4d4b45),
        input: hex(0x6b6a63),
        ring: hex(0xecebe5),
        success: hex(0x6cc06f),
        success_foreground: hex(0x1b1a17),
        success_background: hex(0x182a1d),
        success_line: hex(0x345b3b),
        success_dot: hex(0x6cc06f),
        warning: hex(0xd3a25c),
        warning_foreground: hex(0x1b1a17),
        warning_background: hex(0x302617),
        warning_line: hex(0x68512c),
        warning_dot: hex(0xd3a25c),
        avatar: hex(0x4d4b45),
        avatar_foreground: hex(0xeceae4),
        toast_background: hex(0x26251f),
        toast_foreground: hex(0xf3f1ea),
    },
    radius: RADIUS,
    spacing: SPACING,
    typography: TYPOGRAPHY,
    glass: GLASS,
    elevation: ELEVATION,
};

const RADIUS: Radius = Radius {
    chip: 5.0,
    row: 7.0,
    button: 9.0,
    card: 11.0,
    modal: 14.0,
};

const SPACING: Spacing = Spacing {
    xs: 3.0,
    sm: 7.0,
    md: 9.0,
    lg: 13.0,
    xl: 18.0,
    xxl: 22.0,
};

const TYPOGRAPHY: Typography = Typography {
    display: 22.0,
    screen_title: 20.0,
    section_title: 16.0,
    pane_header: 14.0,
    body: 13.5,
    list: 13.0,
    caption: 12.5,
    machine: 12.0,
    meta: 11.0,
    meta_compact: 10.5,
    field_label: 10.0,
    nav_label: 9.5,
    badge: 9.0,
};

const GLASS: Glass = Glass {
    thin: rgba(0xfdfcfa, 0.50),
    regular: rgba(0xfdfcfa, 0.62),
    sheet: rgba(0xfdfcfa, 0.86),
};

const ELEVATION: Elevation = Elevation {
    popover: shadow(0.13, 3.0, 12.0),
    toast: shadow(0.22, 6.0, 18.0),
    modal: shadow(0.30, 24.0, 60.0),
    app_window: [shadow(0.22, 26.0, 72.0), shadow(0.10, 4.0, 14.0)],
};

pub const BRANDS: [Color; 3] = [hex(0xa05a3c), hex(0x3d63b8), hex(0x3f7d54)];

impl Theme {
    /// Changes the sparse product brand without changing neutral actions or focus.
    pub fn with_brand(mut self, brand: Color) -> Self {
        let light_foreground = Color::WHITE;
        let dark_foreground = hex(0x1b1a17);
        let light_contrast = light_foreground.relative_contrast(brand);
        let dark_contrast = dark_foreground.relative_contrast(brand);
        let light_is_more_legible = light_contrast >= dark_contrast;

        self.palette.brand = brand;
        self.palette.brand_foreground = if light_is_more_legible {
            light_foreground
        } else {
            dark_foreground
        };
        self.palette.brand_background = mix(self.palette.background, brand, 0.09);
        self.palette.brand_line = mix(self.palette.background, brand, 0.25);
        self
    }

    /// Supplies iced's application background while components keep richer tokens.
    pub fn iced(self) -> IcedTheme {
        IcedTheme::custom(
            self.name,
            IcedPalette {
                background: self.palette.background,
                text: self.palette.foreground,
                primary: self.palette.primary,
                success: self.palette.success,
                warning: self.palette.warning,
                danger: self.palette.destructive,
            },
        )
    }
}

pub(crate) const fn hex(value: u32) -> Color {
    Color::from_rgb8(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}

const fn rgba(value: u32, alpha: f32) -> Color {
    Color {
        r: ((value >> 16) & 0xff) as f32 / 255.0,
        g: ((value >> 8) & 0xff) as f32 / 255.0,
        b: (value & 0xff) as f32 / 255.0,
        a: alpha,
    }
}

const fn shadow(alpha: f32, offset_y: f32, blur_radius: f32) -> Shadow {
    Shadow {
        color: rgba(0x282622, alpha),
        offset: Vector {
            x: 0.0,
            y: offset_y,
        },
        blur_radius,
    }
}

pub(crate) fn mix(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: from.a + (to.a - from.a) * amount,
    }
}

pub(crate) fn alpha(mut color: Color, amount: f32) -> Color {
    color.a *= amount;
    color
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_ice_color(name: &str) -> Color {
        let source = include_str!("../ice/default.ice");
        let value = source
            .lines()
            .find_map(|line| {
                let mut parts = line.split_ascii_whitespace();
                (parts.next() == Some(name)).then(|| parts.next()).flatten()
            })
            .unwrap_or_else(|| panic!("default.ice is missing `{name}`"));
        let value = value
            .strip_prefix('#')
            .expect("default Ice colors use hexadecimal literals");
        let byte = |range| {
            u8::from_str_radix(&value[range], 16)
                .expect("default Ice colors are valid hexadecimal literals")
        };
        let alpha = if value.len() == 8 {
            f32::from(byte(6..8)) / 255.0
        } else {
            1.0
        };
        Color::from_rgba8(byte(0..2), byte(2..4), byte(4..6), alpha)
    }

    #[test]
    fn mix_keeps_endpoints_exact() {
        assert_eq!(mix(Color::BLACK, Color::WHITE, 0.0), Color::BLACK);
        assert_eq!(mix(Color::BLACK, Color::WHITE, 1.0), Color::WHITE);
    }

    #[test]
    fn defaults_match_ducktape_design_anchors() {
        assert_eq!(LIGHT.palette.background, hex(0xfdfdfb));
        assert_eq!(LIGHT.palette.card, Color::WHITE);
        assert_eq!(LIGHT.palette.muted, hex(0xf6f5f2));
        assert_eq!(LIGHT.palette.foreground, hex(0x2c2b27));
        assert_eq!(LIGHT.palette.primary, hex(0x26251f));
        assert_eq!(LIGHT.palette.brand, hex(0xa05a3c));
        assert_eq!(LIGHT.palette.avatar_foreground, hex(0x4f4d47));
        assert_eq!(DARK.palette.background, hex(0x1b1a17));
        assert_eq!(BRANDS[0], hex(0xa05a3c));
        assert_eq!(LIGHT.radius.chip, 5.0);
        assert_eq!(
            [
                LIGHT.radius.chip,
                LIGHT.radius.row,
                LIGHT.radius.button,
                LIGHT.radius.card,
                LIGHT.radius.modal,
            ],
            [5.0, 7.0, 9.0, 11.0, 14.0]
        );
        assert_eq!(LIGHT.spacing, SPACING);
        assert_eq!(LIGHT.typography, TYPOGRAPHY);
        assert_eq!(
            [
                LIGHT.typography.display,
                LIGHT.typography.screen_title,
                LIGHT.typography.section_title,
                LIGHT.typography.pane_header,
                LIGHT.typography.body,
                LIGHT.typography.list,
                LIGHT.typography.caption,
                LIGHT.typography.machine,
                LIGHT.typography.meta,
                LIGHT.typography.meta_compact,
                LIGHT.typography.field_label,
                LIGHT.typography.nav_label,
                LIGHT.typography.badge,
            ],
            [
                22.0, 20.0, 16.0, 14.0, 13.5, 13.0, 12.5, 12.0, 11.0, 10.5, 10.0, 9.5, 9.0,
            ]
        );
        assert_eq!(LIGHT.glass, GLASS);
        assert_eq!(LIGHT.elevation, ELEVATION);
    }

    #[test]
    fn runtime_brand_does_not_recolor_actions_or_focus() {
        for base in [LIGHT, DARK] {
            for brand in BRANDS {
                let alternate = base.with_brand(brand);
                assert_eq!(alternate.palette.brand, brand);
                assert!(alternate.palette.brand_foreground.relative_contrast(brand) >= 4.5);
                assert_eq!(alternate.palette.primary, base.palette.primary);
                assert_eq!(alternate.palette.ring, base.palette.ring);
            }
        }

        let alternate = LIGHT.with_brand(BRANDS[1]);
        assert_ne!(
            alternate.palette.brand_background,
            LIGHT.palette.brand_background
        );
        assert_ne!(alternate.palette.brand_line, LIGHT.palette.brand_line);
    }

    #[test]
    fn default_ice_palette_matches_the_retained_light_theme() {
        let palette = LIGHT.palette;
        for (name, color) in [
            ("bg", palette.background),
            ("surface", palette.card),
            ("fg", palette.foreground),
            ("muted", palette.muted_foreground),
            ("muted_bg", palette.muted),
            ("primary", palette.primary),
            ("primary_fg", palette.primary_foreground),
            ("secondary", palette.secondary),
            ("secondary_fg", palette.secondary_foreground),
            ("accent", palette.accent),
            ("accent_fg", palette.accent_foreground),
            ("brand", palette.brand),
            ("brand_fg", palette.brand_foreground),
            ("brand_bg", palette.brand_background),
            ("brand_line", palette.brand_line),
            ("danger", palette.destructive),
            ("danger_fg", palette.destructive_foreground),
            ("danger_bg", palette.destructive_background),
            ("danger_line", palette.destructive_line),
            ("danger_dot", palette.destructive_dot),
            ("success", palette.success),
            ("success_fg", palette.success_foreground),
            ("success_bg", palette.success_background),
            ("success_line", palette.success_line),
            ("success_dot", palette.success_dot),
            ("warning", palette.warning),
            ("warning_fg", palette.warning_foreground),
            ("warning_bg", palette.warning_background),
            ("warning_line", palette.warning_line),
            ("warning_dot", palette.warning_dot),
            ("avatar_bg", palette.avatar),
            ("avatar_fg", palette.avatar_foreground),
            ("toast_bg", palette.toast_background),
            ("toast_fg", palette.toast_foreground),
            ("border", palette.border),
            ("control_line", palette.control_line),
            ("input", palette.input),
            ("ring", palette.ring),
        ] {
            assert_eq!(default_ice_color(name), color, "{name}");
        }
    }

    #[test]
    fn default_ice_glass_and_elevation_roles_match_alpha_bytes() {
        for (name, color) in [
            ("glass_thin", LIGHT.glass.thin),
            ("glass_regular", LIGHT.glass.regular),
            ("glass_sheet", LIGHT.glass.sheet),
        ] {
            let ice = default_ice_color(name);
            assert_eq!(ice.r, color.r, "{name} red");
            assert_eq!(ice.g, color.g, "{name} green");
            assert_eq!(ice.b, color.b, "{name} blue");
            assert_eq!(
                (ice.a * 255.0).round(),
                (color.a * 255.0).round(),
                "{name} alpha"
            );
        }

        let elevation = LIGHT.elevation;
        assert_eq!(elevation.popover, shadow(0.13, 3.0, 12.0));
        assert_eq!(elevation.toast, shadow(0.22, 6.0, 18.0));
        assert_eq!(elevation.modal, shadow(0.30, 24.0, 60.0));
        assert_eq!(elevation.app_window, ELEVATION.app_window);

        for (name, shadow) in [
            ("shadow_popover", elevation.popover),
            ("shadow_toast", elevation.toast),
            ("shadow_modal", elevation.modal),
            ("shadow_window", elevation.app_window[0]),
            ("shadow_window_secondary", elevation.app_window[1]),
        ] {
            let ice = default_ice_color(name);
            assert_eq!(ice.r, shadow.color.r, "{name} red");
            assert_eq!(ice.g, shadow.color.g, "{name} green");
            assert_eq!(ice.b, shadow.color.b, "{name} blue");
            assert_eq!(
                (ice.a * 255.0).round(),
                (shadow.color.a * 255.0).round(),
                "{name} alpha"
            );
        }
    }

    #[test]
    fn default_ice_exposes_only_the_reusable_authoring_surface() {
        let source = include_str!("../ice/default.ice");
        assert!(source.contains("use \"recipes.ice\""));
        assert!(source.contains("use \"components.ice\""));
        assert!(!source.contains("extern "));
    }

    #[test]
    fn ice_components_keep_brand_status_and_empty_state_roles() {
        let recipes = include_str!("../ice/recipes.ice");
        let components = include_str!("../ice/components.ice");

        assert!(recipes.contains("bg-primary text-primary_fg"));
        assert!(recipes.contains("px-16px py-11px"));
        assert!(recipes.contains("px-16px py-10px"));
        assert!(recipes.contains("px-12px py-7px"));
        assert!(recipes.contains("text-13.5px"));
        assert!(recipes.contains("font-mono font-medium"));
        assert!(recipes.contains("font-mono font-semibold"));
        assert!(recipes.contains("font-semibold text-primary"));
        assert!(components.contains("px=6.0 py=3.0 bg=brand r=4.0"));
        assert!(components.contains("text label size=8.0 @badge_label text-brand_fg"));
        assert!(components.contains("bg=success_bg border=success_line"));
        assert!(components.contains("w=30.0 h=30.0 align-x=center align-y=center bg=avatar_bg"));
        assert!(components.contains("shadow=shadow_toast shadow-y=6.0 shadow-blur=18.0"));
        assert_eq!(
            components
                .matches("box w=6.0 h=6.0 bg=success_dot r=3.0")
                .count(),
            2
        );
        assert!(components.contains("shadow=shadow_modal shadow-y=24.0 shadow-blur=60.0"));
        assert!(components.contains("w=fill h=fill p=22.0 align-x=center align-y=center"));
        assert_eq!(
            components
                .lines()
                .filter(|line| line.starts_with("component "))
                .count(),
            components.matches("#root").count()
        );
    }

    #[test]
    fn semantic_colors_clear_accessibility_contrast() {
        for theme in [LIGHT, DARK] {
            assert!(
                theme
                    .palette
                    .ring
                    .relative_contrast(theme.palette.background)
                    >= 3.0,
                "{} focus boundary",
                theme.name
            );

            for (name, foreground, background) in [
                (
                    "default text",
                    theme.palette.foreground,
                    theme.palette.background,
                ),
                (
                    "card text",
                    theme.palette.card_foreground,
                    theme.palette.card,
                ),
                (
                    "popover text",
                    theme.palette.popover_foreground,
                    theme.palette.popover,
                ),
                (
                    "primary text",
                    theme.palette.primary_foreground,
                    theme.palette.primary,
                ),
                (
                    "secondary text",
                    theme.palette.secondary_foreground,
                    theme.palette.secondary,
                ),
                (
                    "muted text",
                    theme.palette.muted_foreground,
                    theme.palette.muted,
                ),
                (
                    "accent text",
                    theme.palette.accent_foreground,
                    theme.palette.accent,
                ),
                (
                    "brand text",
                    theme.palette.brand_foreground,
                    theme.palette.brand,
                ),
                (
                    "destructive text",
                    theme.palette.destructive_foreground,
                    theme.palette.destructive,
                ),
                (
                    "success text",
                    theme.palette.success_foreground,
                    theme.palette.success,
                ),
                (
                    "warning text",
                    theme.palette.warning_foreground,
                    theme.palette.warning,
                ),
                (
                    "avatar text",
                    theme.palette.avatar_foreground,
                    theme.palette.avatar,
                ),
            ] {
                assert!(
                    foreground.relative_contrast(background) >= 4.5,
                    "{} {name}",
                    theme.name
                );
            }

            for (index, brand) in BRANDS.into_iter().enumerate() {
                assert!(
                    brand.relative_contrast(theme.palette.background) >= 3.0,
                    "{} brand {index}",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn canonical_low_contrast_success_color_is_not_a_body_text_role() {
        let success_state = LIGHT
            .palette
            .success
            .relative_contrast(LIGHT.palette.success_background);

        assert!((success_state - 2.86).abs() < 0.02);

        // Status labels use neutral foregrounds; the success role is reserved for a
        // redundant dot/icon.
        assert!(
            LIGHT
                .palette
                .foreground
                .relative_contrast(LIGHT.palette.success_background)
                >= 4.5
        );
    }
}
