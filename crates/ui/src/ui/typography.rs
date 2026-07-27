use super::theme::Theme;
use iced::font::Weight;
use iced::widget::text::IntoFragment;
use iced::widget::{Container, Text, container, text};
use iced::{Background, Border, Color, Font};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextRole {
    Display,
    ScreenTitle,
    SectionTitle,
    PaneHeader,
    #[default]
    Body,
    List,
    Caption,
    Machine,
    Meta,
    MetaCompact,
    FieldLabel,
    NavLabel,
    Badge,
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
    container(typography(content, TextRole::Machine, theme))
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
    let (size, line_height, color, font, weight) = match role {
        TextRole::Display => (
            typography.display,
            1.2,
            palette.foreground,
            typography.font,
            Weight::Semibold,
        ),
        TextRole::ScreenTitle => (
            typography.screen_title,
            1.2,
            palette.foreground,
            typography.font,
            Weight::Semibold,
        ),
        TextRole::SectionTitle => (
            typography.section_title,
            1.2,
            palette.foreground,
            typography.font,
            Weight::Semibold,
        ),
        TextRole::PaneHeader => (
            typography.pane_header,
            1.3,
            palette.foreground,
            typography.font,
            Weight::Semibold,
        ),
        TextRole::Body => (
            typography.body,
            1.55,
            palette.accent_foreground,
            typography.font,
            Weight::Normal,
        ),
        TextRole::List => (
            typography.list,
            1.4,
            palette.accent_foreground,
            typography.font,
            Weight::Medium,
        ),
        TextRole::Caption => (
            typography.caption,
            1.4,
            palette.muted_foreground,
            typography.font,
            Weight::Normal,
        ),
        TextRole::Machine => (
            typography.machine,
            1.4,
            palette.secondary_foreground,
            typography.monospace_font,
            Weight::Normal,
        ),
        TextRole::Meta => (
            typography.meta,
            1.4,
            palette.muted_foreground,
            typography.monospace_font,
            Weight::Medium,
        ),
        TextRole::MetaCompact => (
            typography.meta_compact,
            1.4,
            palette.muted_foreground,
            typography.monospace_font,
            Weight::Medium,
        ),
        TextRole::FieldLabel => (
            typography.field_label,
            1.3,
            palette.muted_foreground,
            typography.monospace_font,
            Weight::Semibold,
        ),
        TextRole::NavLabel => (
            typography.nav_label,
            1.3,
            palette.accent_foreground,
            typography.font,
            Weight::Semibold,
        ),
        TextRole::Badge => (
            typography.badge,
            1.3,
            palette.foreground,
            typography.monospace_font,
            Weight::Semibold,
        ),
    };

    RoleStyle {
        size,
        line_height,
        color,
        font: Font { weight, ..font },
    }
}

fn inline_code_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.muted)),
        text_color: Some(theme.palette.foreground),
        border: Border {
            radius: theme.radius.chip.into(),
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
        let mut theme = LIGHT;
        theme.typography.font = Font::with_name("Geist");
        theme.typography.monospace_font = Font::with_name("Geist Mono");
        assert_eq!(
            role_style(TextRole::Display, &theme).size,
            LIGHT.typography.display
        );
        assert_eq!(
            role_style(TextRole::Body, &theme).size,
            LIGHT.typography.body
        );
        assert_eq!(
            role_style(TextRole::Caption, &theme).color,
            LIGHT.palette.muted_foreground
        );
        assert_eq!(
            role_style(TextRole::Machine, &theme).font.family,
            Font::with_name("Geist Mono").family
        );
        assert_eq!(
            role_style(TextRole::Display, &theme).font.family,
            Font::with_name("Geist").family
        );
        assert_eq!(
            role_style(TextRole::FieldLabel, &theme).font.weight,
            Weight::Semibold
        );
        assert_eq!(
            role_style(TextRole::Meta, &theme).font.weight,
            Weight::Medium
        );
        assert_eq!(role_style(TextRole::Badge, &theme).size, 9.0);
    }

    #[test]
    fn inline_code_box_uses_semantic_surface_tokens() {
        let style = inline_code_style(&LIGHT);
        assert_eq!(
            style.background,
            Some(Background::Color(LIGHT.palette.muted))
        );
        assert_eq!(style.text_color, Some(LIGHT.palette.foreground));
        assert_eq!(style.border.radius, LIGHT.radius.chip.into());
    }
}
