use super::theme::Theme;
use iced::font::{Family, Weight};
use iced::widget::text::IntoFragment;
use iced::widget::{Container, Text, container, text};
use iced::{Background, Border, Color, Font};

const UI_FONT_FAMILY: Family = Family::Name("Geist");
const MACHINE_FONT_FAMILY: Family = Family::Name("Geist Mono");

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
    let (size, line_height, color, family, weight) = match role {
        TextRole::Display => (
            typography.display,
            1.2,
            palette.foreground,
            UI_FONT_FAMILY,
            Weight::Semibold,
        ),
        TextRole::ScreenTitle => (
            typography.screen_title,
            1.2,
            palette.foreground,
            UI_FONT_FAMILY,
            Weight::Semibold,
        ),
        TextRole::SectionTitle => (
            typography.section_title,
            1.2,
            palette.foreground,
            UI_FONT_FAMILY,
            Weight::Semibold,
        ),
        TextRole::PaneHeader => (
            typography.pane_header,
            1.3,
            palette.foreground,
            UI_FONT_FAMILY,
            Weight::Semibold,
        ),
        TextRole::Body => (
            typography.body,
            1.55,
            palette.accent_foreground,
            UI_FONT_FAMILY,
            Weight::Normal,
        ),
        TextRole::List => (
            typography.list,
            1.4,
            palette.accent_foreground,
            UI_FONT_FAMILY,
            Weight::Medium,
        ),
        TextRole::Caption => (
            typography.caption,
            1.4,
            palette.muted_foreground,
            UI_FONT_FAMILY,
            Weight::Normal,
        ),
        TextRole::Machine => (
            typography.machine,
            1.4,
            palette.secondary_foreground,
            MACHINE_FONT_FAMILY,
            Weight::Normal,
        ),
        TextRole::Meta => (
            typography.meta,
            1.4,
            palette.muted_foreground,
            MACHINE_FONT_FAMILY,
            Weight::Medium,
        ),
        TextRole::MetaCompact => (
            typography.meta_compact,
            1.4,
            palette.muted_foreground,
            MACHINE_FONT_FAMILY,
            Weight::Medium,
        ),
        TextRole::FieldLabel => (
            typography.field_label,
            1.3,
            palette.muted_foreground,
            MACHINE_FONT_FAMILY,
            Weight::Semibold,
        ),
        TextRole::NavLabel => (
            typography.nav_label,
            1.3,
            palette.accent_foreground,
            UI_FONT_FAMILY,
            Weight::Semibold,
        ),
        TextRole::Badge => (
            typography.badge,
            1.3,
            palette.foreground,
            MACHINE_FONT_FAMILY,
            Weight::Semibold,
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
        assert_eq!(
            role_style(TextRole::Display, &LIGHT).size,
            LIGHT.typography.display
        );
        assert_eq!(
            role_style(TextRole::Body, &LIGHT).size,
            LIGHT.typography.body
        );
        assert_eq!(
            role_style(TextRole::Caption, &LIGHT).color,
            LIGHT.palette.muted_foreground
        );
        assert_eq!(
            role_style(TextRole::Machine, &LIGHT).font.family,
            Family::Name("Geist Mono")
        );
        assert_eq!(
            role_style(TextRole::Display, &LIGHT).font.family,
            Family::Name("Geist")
        );
        assert_eq!(
            role_style(TextRole::FieldLabel, &LIGHT).font.weight,
            Weight::Semibold
        );
        assert_eq!(
            role_style(TextRole::Meta, &LIGHT).font.weight,
            Weight::Medium
        );
        assert_eq!(role_style(TextRole::Badge, &LIGHT).size, 9.0);
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
