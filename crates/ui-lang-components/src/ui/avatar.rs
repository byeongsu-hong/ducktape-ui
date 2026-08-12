use super::theme::Theme;
use iced::widget::text::IntoFragment;
use iced::widget::{Container, container, text};
use iced::{Background, Border, Element};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Small,
    #[default]
    Default,
    Large,
}

/// A circular human-avatar frame for caller-owned content.
///
/// The shared Ice `Avatar.Agent` component is the exact 30px, 8px-radius agent
/// treatment; the native API stays focused on image-capable human avatars.
pub fn avatar<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    size: AvatarSize,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let diameter = metrics(size, theme).diameter;
    let theme = *theme;

    container(content)
        .center(diameter)
        .clip(true)
        .style(move |_iced_theme| style(&theme))
}

/// Text fallback for an avatar. Use a short visible name or initials.
pub fn avatar_fallback<'a, Message>(
    label: impl IntoFragment<'a>,
    size: AvatarSize,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let metrics = metrics(size, theme);
    avatar(
        text(label)
            .size(metrics.text)
            .color(theme.palette.avatar_foreground),
        size,
        theme,
    )
}

pub fn style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.avatar)),
        text_color: Some(theme.palette.avatar_foreground),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    diameter: f32,
    text: f32,
}

fn metrics(size: AvatarSize, theme: &Theme) -> Metrics {
    match size {
        AvatarSize::Small => Metrics {
            diameter: 24.0,
            text: theme.typography.meta_compact,
        },
        AvatarSize::Default => Metrics {
            diameter: 30.0,
            text: theme.typography.meta,
        },
        AvatarSize::Large => Metrics {
            diameter: 40.0,
            text: theme.typography.caption,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn avatar_sizes_scale_frame_and_fallback_together() {
        let small = metrics(AvatarSize::Small, &LIGHT);
        let default = metrics(AvatarSize::Default, &LIGHT);
        let large = metrics(AvatarSize::Large, &LIGHT);

        assert!(small.diameter < default.diameter && default.diameter < large.diameter);
        assert!(small.text < default.text && default.text < large.text);
        assert_eq!(small.text, LIGHT.typography.meta_compact);
        assert_eq!(default.diameter, 30.0);
        assert_eq!(default.text, LIGHT.typography.meta);
    }
}
