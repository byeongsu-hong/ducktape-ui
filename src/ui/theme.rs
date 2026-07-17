use iced::theme::Palette as IcedPalette;
use iced::{Color, Theme as IcedTheme};

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
    pub destructive: Color,
    pub destructive_foreground: Color,
    pub border: Color,
    pub input: Color,
    pub ring: Color,
    pub success: Color,
    pub success_foreground: Color,
    pub warning: Color,
    pub warning_foreground: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radius {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
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
    pub xs: f32,
    pub sm: f32,
    pub base: f32,
    pub lg: f32,
    pub xl: f32,
}

/// Application-owned design tokens consumed through semantic roles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub palette: Palette,
    pub radius: Radius,
    pub spacing: Spacing,
    pub typography: Typography,
}

pub const LIGHT: Theme = Theme {
    name: "Ducktape Light",
    palette: Palette {
        background: hex(0xffffff),
        foreground: hex(0x2c2b27),
        card: hex(0xffffff),
        card_foreground: hex(0x2c2b27),
        popover: hex(0xffffff),
        popover_foreground: hex(0x2c2b27),
        primary: hex(0x26251f),
        primary_foreground: hex(0xefefef),
        secondary: hex(0xefefef),
        secondary_foreground: hex(0x2c2b27),
        muted: hex(0xf5f5f5),
        muted_foreground: hex(0x606060),
        accent: hex(0xededed),
        accent_foreground: hex(0x2c2b27),
        destructive: hex(0xc0483c),
        destructive_foreground: Color::WHITE,
        border: hex(0xe5e5e5),
        input: hex(0x949494),
        ring: hex(0xa05a3c),
        success: hex(0x4f9050),
        success_foreground: hex(0x151410),
        warning: hex(0xa67938),
        warning_foreground: hex(0x151410),
    },
    radius: RADIUS,
    spacing: SPACING,
    typography: TYPOGRAPHY,
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
        destructive: hex(0xd4655a),
        destructive_foreground: hex(0x1b1a17),
        border: hex(0x2e2d27),
        input: hex(0x6b6a63),
        ring: hex(0xa05a3c),
        success: hex(0x6cc06f),
        success_foreground: hex(0x1b1a17),
        warning: hex(0xd3a25c),
        warning_foreground: hex(0x1b1a17),
    },
    radius: RADIUS,
    spacing: SPACING,
    typography: TYPOGRAPHY,
};

const RADIUS: Radius = Radius {
    sm: 7.0,
    md: 9.0,
    lg: 11.0,
    xl: 13.0,
};

const SPACING: Spacing = Spacing {
    xs: 4.0,
    sm: 8.0,
    md: 12.0,
    lg: 16.0,
    xl: 24.0,
    xxl: 32.0,
};

const TYPOGRAPHY: Typography = Typography {
    xs: 10.5,
    sm: 12.0,
    base: 13.0,
    lg: 15.5,
    xl: 18.0,
};

pub const ACCENTS: [Color; 3] = [hex(0xa05a3c), hex(0x3d63b8), hex(0x3f7d54)];

impl Theme {
    /// Changes the runtime accent without changing neutral primary actions.
    pub const fn with_accent(mut self, accent: Color) -> Self {
        self.palette.ring = accent;
        self
    }

    /// Supplies iced's application background while components keep richer tokens.
    pub fn iced(self) -> IcedTheme {
        IcedTheme::custom(
            self.name,
            IcedPalette {
                background: self.palette.background,
                text: self.palette.foreground,
                primary: self.palette.ring,
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

    #[test]
    fn mix_keeps_endpoints_exact() {
        assert_eq!(mix(Color::BLACK, Color::WHITE, 0.0), Color::BLACK);
        assert_eq!(mix(Color::BLACK, Color::WHITE, 1.0), Color::WHITE);
    }

    #[test]
    fn defaults_match_ducktape_design_anchors() {
        assert_eq!(LIGHT.palette.foreground, hex(0x2c2b27));
        assert_eq!(DARK.palette.background, hex(0x1b1a17));
        assert_eq!(ACCENTS[0], hex(0xa05a3c));
        assert_eq!(LIGHT.radius.md, 9.0);
    }

    #[test]
    fn semantic_colors_clear_accessibility_contrast() {
        for theme in [LIGHT, DARK] {
            assert!(
                theme
                    .palette
                    .input
                    .relative_contrast(theme.palette.background)
                    >= 3.0,
                "{} input boundary",
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
            ] {
                assert!(
                    foreground.relative_contrast(background) >= 4.5,
                    "{} {name}",
                    theme.name
                );
            }

            for (index, accent) in ACCENTS.into_iter().enumerate() {
                assert!(
                    accent.relative_contrast(theme.palette.background) >= 3.0,
                    "{} accent {index} focus indicator",
                    theme.name
                );
            }
        }
    }
}
