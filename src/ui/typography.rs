use super::theme::Theme;
use iced::font::{Family, Weight};
use iced::widget::text::IntoFragment;
use iced::widget::{Container, Text, container, text};
use iced::{Background, Border, Color, Font};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextRole {
    H1,
    H2,
    H3,
    H4,
    #[default]
    Paragraph,
    Lead,
    Large,
    Small,
    Muted,
    InlineCode,
}

/// Applies a consistent visual text role without adding layout.
pub fn typography<'a>(content: impl IntoFragment<'a>, role: TextRole, theme: &Theme) -> Text<'a> {
    let style = role_style(role, theme);
    text(content)
        .size(style.size)
        .line_height(style.line_height)
        .font(style.font)
        .color(style.color)
}

/// Inline code with the background and padding a plain `Text` cannot provide.
pub fn inline_code<'a, Message>(
    content: impl IntoFragment<'a>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let styled_theme = *theme;
    container(typography(content, TextRole::InlineCode, theme))
        .padding([theme.spacing.xs / 2.0, theme.spacing.xs])
        .style(move |_iced_theme| inline_code_style(&styled_theme))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RoleStyle {
    size: f32,
    line_height: f32,
    color: Color,
    font: Font,
}

fn role_style(role: TextRole, theme: &Theme) -> RoleStyle {
    let palette = theme.palette;
    let typography = theme.typography;
    let (size, line_height, color, family, weight) = match role {
        TextRole::H1 => (
            typography.xl * 2.0,
            1.1,
            palette.foreground,
            Family::SansSerif,
            Weight::Bold,
        ),
        TextRole::H2 => (
            typography.xl * 1.67,
            1.15,
            palette.foreground,
            Family::SansSerif,
            Weight::Semibold,
        ),
        TextRole::H3 => (
            typography.xl * 1.33,
            1.2,
            palette.foreground,
            Family::SansSerif,
            Weight::Semibold,
        ),
        TextRole::H4 => (
            typography.xl,
            1.25,
            palette.foreground,
            Family::SansSerif,
            Weight::Semibold,
        ),
        TextRole::Paragraph => (
            typography.base,
            1.6,
            palette.foreground,
            Family::SansSerif,
            Weight::Normal,
        ),
        TextRole::Lead => (
            typography.lg,
            1.5,
            palette.muted_foreground,
            Family::SansSerif,
            Weight::Normal,
        ),
        TextRole::Large => (
            typography.lg,
            1.4,
            palette.foreground,
            Family::SansSerif,
            Weight::Semibold,
        ),
        TextRole::Small => (
            typography.sm,
            1.4,
            palette.foreground,
            Family::SansSerif,
            Weight::Medium,
        ),
        TextRole::Muted => (
            typography.sm,
            1.4,
            palette.muted_foreground,
            Family::SansSerif,
            Weight::Normal,
        ),
        TextRole::InlineCode => (
            typography.sm,
            1.4,
            palette.foreground,
            Family::Monospace,
            Weight::Medium,
        ),
    };

    RoleStyle {
        size,
        line_height,
        color,
        font: Font {
            family,
            weight,
            ..Font::DEFAULT
        },
    }
}

fn inline_code_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.muted)),
        text_color: Some(theme.palette.foreground),
        border: Border {
            radius: theme.radius.sm.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn roles_map_to_theme_typography_and_palette() {
        assert_eq!(
            role_style(TextRole::H1, &LIGHT).size,
            LIGHT.typography.xl * 2.0
        );
        assert_eq!(
            role_style(TextRole::Paragraph, &LIGHT).size,
            LIGHT.typography.base
        );
        assert_eq!(
            role_style(TextRole::Muted, &LIGHT).color,
            LIGHT.palette.muted_foreground
        );
        assert_eq!(
            role_style(TextRole::InlineCode, &LIGHT).font.family,
            Family::Monospace
        );
    }

    #[test]
    fn inline_code_box_uses_semantic_surface_tokens() {
        let style = inline_code_style(&LIGHT);
        assert_eq!(
            style.background,
            Some(Background::Color(LIGHT.palette.muted))
        );
        assert_eq!(style.text_color, Some(LIGHT.palette.foreground));
        assert_eq!(style.border.radius, LIGHT.radius.sm.into());
    }
}
