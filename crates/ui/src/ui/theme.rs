use iced::theme::Palette as IcedPalette;
use iced::{Color, Font, Shadow, Theme as IcedTheme, Vector};

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
    pub primary_hover: Color,
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
    pub disabled: Color,
    pub disabled_foreground: Color,
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

/// Control geometry that changes with a visual profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Controls {
    /// `[vertical, horizontal]` padding.
    pub primary_padding: [f32; 2],
    pub secondary_padding: [f32; 2],
    pub compact_padding: [f32; 2],
    pub small_padding: [f32; 2],
    pub large_padding: [f32; 2],
    pub input_padding: [f32; 2],
    pub default_height: Option<f32>,
    pub small_height: f32,
    pub large_height: f32,
    pub icon_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Typography {
    pub font: Font,
    pub monospace_font: Font,
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
    pub controls: Controls,
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
        primary_hover: hex(0x322f28),
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
        disabled: hex(0xecebe6),
        disabled_foreground: hex(0xb3b1a8),
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
    controls: CONTROLS,
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
        primary_hover: hex(0xdad9d2),
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
        disabled: hex(0x2b2a25),
        disabled_foreground: hex(0x6b6a63),
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
    controls: CONTROLS,
    typography: TYPOGRAPHY,
    glass: GLASS,
    elevation: ELEVATION,
};

/// Neutral shadcn-style profile using the current official semantic token scale.
pub const SHADCN_LIGHT: Theme = Theme {
    name: "shadcn Neutral Light",
    palette: Palette {
        background: Color::WHITE,
        foreground: hex(0x171717),
        card: Color::WHITE,
        card_foreground: hex(0x171717),
        popover: Color::WHITE,
        popover_foreground: hex(0x171717),
        primary: hex(0x262626),
        primary_hover: hex(0x3c3c3c),
        primary_foreground: hex(0xfafafa),
        secondary: hex(0xf5f5f5),
        secondary_foreground: hex(0x262626),
        muted: hex(0xf5f5f5),
        muted_foreground: hex(0x6b6b6b),
        accent: hex(0xf5f5f5),
        accent_foreground: hex(0x262626),
        brand: hex(0x262626),
        brand_foreground: hex(0xfafafa),
        brand_background: hex(0xf5f5f5),
        brand_line: hex(0xe5e5e5),
        destructive: hex(0xdc2626),
        destructive_foreground: Color::WHITE,
        destructive_background: hex(0xfef2f2),
        destructive_line: hex(0xfecaca),
        destructive_dot: hex(0xdc2626),
        border: hex(0xe5e5e5),
        control_line: hex(0xe5e5e5),
        input: hex(0xe5e5e5),
        ring: hex(0x737373),
        disabled: hex(0xf5f5f5),
        disabled_foreground: hex(0xa3a3a3),
        success: hex(0x16a34a),
        success_foreground: hex(0x052e16),
        success_background: hex(0xf0fdf4),
        success_line: hex(0xbbf7d0),
        success_dot: hex(0x22c55e),
        warning: hex(0xd97706),
        warning_foreground: hex(0x171717),
        warning_background: hex(0xfffbeb),
        warning_line: hex(0xfde68a),
        warning_dot: hex(0xf59e0b),
        avatar: hex(0xe5e5e5),
        avatar_foreground: hex(0x404040),
        toast_background: hex(0x171717),
        toast_foreground: hex(0xfafafa),
    },
    radius: SHADCN_RADIUS,
    spacing: SHADCN_SPACING,
    controls: SHADCN_CONTROLS,
    typography: SHADCN_TYPOGRAPHY,
    glass: SHADCN_GLASS_LIGHT,
    elevation: ELEVATION,
};

/// Dark counterpart to [`SHADCN_LIGHT`].
pub const SHADCN_DARK: Theme = Theme {
    name: "shadcn Neutral Dark",
    palette: Palette {
        background: hex(0x171717),
        foreground: hex(0xfafafa),
        card: hex(0x262626),
        card_foreground: hex(0xfafafa),
        popover: hex(0x262626),
        popover_foreground: hex(0xfafafa),
        primary: hex(0xe5e5e5),
        primary_hover: hex(0xd1d1d1),
        primary_foreground: hex(0x262626),
        secondary: hex(0x404040),
        secondary_foreground: hex(0xfafafa),
        muted: hex(0x404040),
        muted_foreground: hex(0xb3b3b3),
        accent: hex(0x404040),
        accent_foreground: hex(0xfafafa),
        brand: hex(0xe5e5e5),
        brand_foreground: hex(0x262626),
        brand_background: hex(0x404040),
        brand_line: hex(0x525252),
        destructive: hex(0xff6467),
        destructive_foreground: hex(0x171717),
        destructive_background: hex(0x450a0a),
        destructive_line: hex(0x7f1d1d),
        destructive_dot: hex(0xff6467),
        border: hex(0x2e2e2e),
        control_line: hex(0x2e2e2e),
        input: hex(0x393939),
        ring: hex(0x737373),
        disabled: hex(0x404040),
        disabled_foreground: hex(0x737373),
        success: hex(0x4ade80),
        success_foreground: hex(0x171717),
        success_background: hex(0x052e16),
        success_line: hex(0x166534),
        success_dot: hex(0x4ade80),
        warning: hex(0xfbbf24),
        warning_foreground: hex(0x171717),
        warning_background: hex(0x451a03),
        warning_line: hex(0x92400e),
        warning_dot: hex(0xfbbf24),
        avatar: hex(0x404040),
        avatar_foreground: hex(0xe5e5e5),
        toast_background: hex(0xfafafa),
        toast_foreground: hex(0x171717),
    },
    radius: SHADCN_RADIUS,
    spacing: SHADCN_SPACING,
    controls: SHADCN_CONTROLS,
    typography: SHADCN_TYPOGRAPHY,
    glass: SHADCN_GLASS_DARK,
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

const CONTROLS: Controls = Controls {
    primary_padding: [11.0, 16.0],
    secondary_padding: [11.0, 16.0],
    compact_padding: [8.0, 12.0],
    small_padding: [6.0, 12.0],
    large_padding: [10.0, 24.0],
    input_padding: [8.0, 12.0],
    default_height: None,
    small_height: 32.0,
    large_height: 40.0,
    icon_size: 30.0,
};

const SHADCN_RADIUS: Radius = Radius {
    chip: 6.0,
    row: 8.0,
    button: 8.0,
    card: 14.0,
    modal: 14.0,
};

const SHADCN_SPACING: Spacing = Spacing {
    xs: 4.0,
    sm: 8.0,
    md: 12.0,
    lg: 16.0,
    xl: 24.0,
    xxl: 32.0,
};

const SHADCN_CONTROLS: Controls = Controls {
    primary_padding: [8.0, 16.0],
    secondary_padding: [8.0, 16.0],
    compact_padding: [6.0, 12.0],
    small_padding: [6.0, 12.0],
    large_padding: [10.0, 24.0],
    input_padding: [8.0, 12.0],
    default_height: Some(36.0),
    small_height: 32.0,
    large_height: 40.0,
    icon_size: 36.0,
};

const TYPOGRAPHY: Typography = Typography {
    font: Font::DEFAULT,
    monospace_font: Font::MONOSPACE,
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
    field_label: 12.5,
    nav_label: 9.5,
    badge: 9.0,
};

const SHADCN_TYPOGRAPHY: Typography = Typography {
    font: Font::DEFAULT,
    monospace_font: Font::MONOSPACE,
    display: 24.0,
    screen_title: 20.0,
    section_title: 18.0,
    pane_header: 14.0,
    body: 14.0,
    list: 14.0,
    caption: 14.0,
    machine: 12.0,
    meta: 12.0,
    meta_compact: 11.0,
    field_label: 14.0,
    nav_label: 12.0,
    badge: 12.0,
};

const GLASS: Glass = Glass {
    thin: rgba(0xfdfcfa, 0.50),
    regular: rgba(0xfdfcfa, 0.62),
    sheet: rgba(0xfdfcfa, 0.86),
};

const SHADCN_GLASS_LIGHT: Glass = Glass {
    thin: rgba(0xffffff, 0.80),
    regular: rgba(0xffffff, 0.90),
    sheet: rgba(0xffffff, 0.96),
};

const SHADCN_GLASS_DARK: Glass = Glass {
    thin: rgba(0x262626, 0.80),
    regular: rgba(0x262626, 0.90),
    sheet: rgba(0x262626, 0.96),
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
        let brand = Color { a: 1.0, ..brand };
        let light_foreground = Color::WHITE;
        let dark_foreground = Color::BLACK;
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
        assert_eq!(LIGHT.palette.primary_hover, hex(0x322f28));
        assert_eq!(LIGHT.palette.disabled, hex(0xecebe6));
        assert_eq!(LIGHT.palette.disabled_foreground, hex(0xb3b1a8));
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
        assert_eq!(LIGHT.controls, CONTROLS);
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
                22.0, 20.0, 16.0, 14.0, 13.5, 13.0, 12.5, 12.0, 11.0, 10.5, 12.5, 9.5, 9.0,
            ]
        );
        assert_eq!(LIGHT.glass, GLASS);
        assert_eq!(LIGHT.elevation, ELEVATION);
    }

    #[test]
    fn runtime_brand_does_not_recolor_actions_or_focus() {
        for base in [LIGHT, DARK, SHADCN_LIGHT, SHADCN_DARK] {
            for brand in BRANDS
                .into_iter()
                .chain([hex(0x777777), Color::TRANSPARENT])
            {
                let alternate = base.with_brand(brand);
                assert_eq!(alternate.palette.brand, Color { a: 1.0, ..brand });
                assert!(
                    alternate
                        .palette
                        .brand_foreground
                        .relative_contrast(alternate.palette.brand)
                        >= 4.5
                );
                assert_eq!(alternate.palette.primary, base.palette.primary);
                assert_eq!(alternate.palette.primary_hover, base.palette.primary_hover);
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
            ("primary_hover", palette.primary_hover),
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
            ("disabled", palette.disabled),
            ("disabled_fg", palette.disabled_foreground),
        ] {
            assert_eq!(default_ice_color(name), color, "{name}");
        }
    }

    #[test]
    fn shadcn_profiles_replace_color_and_metric_scales_together() {
        assert_eq!(SHADCN_LIGHT.palette.background, Color::WHITE);
        assert_eq!(SHADCN_LIGHT.palette.foreground, hex(0x171717));
        assert_eq!(SHADCN_DARK.palette.background, hex(0x171717));
        assert_eq!(SHADCN_DARK.palette.foreground, hex(0xfafafa));
        assert_eq!(SHADCN_LIGHT.radius, SHADCN_RADIUS);
        assert_eq!(SHADCN_LIGHT.spacing, SHADCN_SPACING);
        assert_eq!(SHADCN_LIGHT.controls, SHADCN_CONTROLS);
        assert_eq!(SHADCN_LIGHT.typography, SHADCN_TYPOGRAPHY);
        assert_eq!(SHADCN_LIGHT.controls.default_height, Some(36.0));
        assert_ne!(SHADCN_LIGHT.radius.card, LIGHT.radius.card);
        assert_ne!(SHADCN_LIGHT.typography.caption, LIGHT.typography.caption);
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
        assert_eq!(recipes.matches("text-12.5px font-semibold px-").count(), 5);
        assert!(recipes.contains("px-16px py-11px"));
        assert!(recipes.contains("disabled:bg-disabled disabled:text-disabled_fg"));
        assert!(recipes.contains("px-12px py-8px"));
        assert!(recipes.contains("text-13.5px"));
        assert!(recipes.contains("font-mono font-medium"));
        assert!(recipes.contains("font-mono font-semibold"));
        assert!(recipes.contains("font-semibold text-primary"));
        assert!(components.contains("      px=6.0\n      py=3.0\n      bg=brand\n      r=4.0"));
        assert!(components.contains(
            "    text label\n      with\n        size=8.0\n        @badge_label\n        @text-brand_fg"
        ));
        assert!(components.contains("      bg=success_bg\n      border=success_line"));
        assert!(components.contains(
            "      w=30.0\n      h=30.0\n      align-x=center\n      align-y=center\n      bg=avatar_bg"
        ));
        assert!(
            components
                .contains("      shadow=shadow_toast\n      shadow-y=6.0\n      shadow-blur=18.0")
        );
        assert_eq!(
            components
                .matches(
                    "          w=6.0\n          h=6.0\n          bg=success_dot\n          r=3.0"
                )
                .count(),
            2
        );
        assert!(
            components
                .contains("      shadow=shadow_modal\n      shadow-y=24.0\n      shadow-blur=60.0")
        );
        assert!(components.contains(
            "      w=fill\n      h=fill\n      p=22.0\n      align-x=center\n      align-y=center"
        ));
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
        for theme in [LIGHT, DARK, SHADCN_LIGHT, SHADCN_DARK] {
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
