use super::focus_control::{FocusControl, Status, Style};
use super::theme::{Theme, alpha, mix};
use iced::advanced::widget;
use iced::widget::{Row, Space, container};
use iced::{Background, Border, Color, Element, Length, Shadow};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SwitchSize {
    Small,
    #[default]
    Default,
}

/// A controlled switch with pointer, touch, Enter, and Space activation.
///
/// The caller owns `checked` and supplies the message that replaces it.
pub struct Switch<Message> {
    id: widget::Id,
    checked: bool,
    on_toggle: Message,
    size: SwitchSize,
    disabled: bool,
    theme: Theme,
}

pub fn switch<Message>(
    id: widget::Id,
    checked: bool,
    on_toggle: Message,
    theme: &Theme,
) -> Switch<Message>
where
    Message: Clone,
{
    Switch {
        id,
        checked,
        on_toggle,
        size: SwitchSize::Default,
        disabled: false,
        theme: *theme,
    }
}

impl<Message> Switch<Message>
where
    Message: Clone,
{
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    pub fn into_widget<'a>(self) -> FocusControl<'a, Message>
    where
        Message: 'a,
    {
        let metrics = metrics(self.size);
        let thumb_color = if self.disabled {
            alpha(
                if self.checked {
                    self.theme.palette.primary_foreground
                } else {
                    unchecked_thumb(&self.theme)
                },
                0.65,
            )
        } else if self.checked {
            self.theme.palette.primary_foreground
        } else {
            unchecked_thumb(&self.theme)
        };
        let thumb = container(Space::new())
            .width(metrics.thumb)
            .height(metrics.thumb)
            .style(move |_iced_theme| thumb_style(thumb_color));
        let spacer = Space::new().width(Length::Fill);
        let track = if self.checked {
            Row::new().push(spacer).push(thumb)
        } else {
            Row::new().push(thumb).push(spacer)
        };
        let content = container(track)
            .padding((metrics.height - metrics.thumb) / 2.0)
            .width(metrics.width)
            .height(metrics.height);
        let theme = self.theme;
        let checked = self.checked;

        FocusControl::new(self.id, content, self.on_toggle, &theme)
            .disabled(self.disabled)
            .style(move |_iced_theme, status| style(&theme, checked, status))
    }
}

impl<'a, Message> From<Switch<Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(switch: Switch<Message>) -> Self {
        switch.into_widget().into()
    }
}

pub fn style(theme: &Theme, checked: bool, status: Status) -> Style {
    let palette = theme.palette;
    let base = if checked {
        palette.primary
    } else {
        palette.input
    };
    let mut background = match status {
        Status::Hovered => mix(base, palette.foreground, 0.07),
        Status::Pressed => mix(base, palette.foreground, 0.14),
        _ => base,
    };
    let mut border_color = base;

    if status == Status::Disabled {
        background = alpha(background, 0.5);
        border_color = alpha(border_color, 0.5);
    }

    Style {
        background: Some(Background::Color(background)),
        text_color: None,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 999.0.into(),
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: palette.ring,
            width: 2.0,
            radius: 999.0.into(),
        },
        focus_offset: 0.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    width: f32,
    height: f32,
    thumb: f32,
}

const fn metrics(size: SwitchSize) -> Metrics {
    match size {
        SwitchSize::Small => Metrics {
            width: 24.0,
            height: 14.0,
            thumb: 12.0,
        },
        SwitchSize::Default => Metrics {
            width: 32.0,
            height: 18.4,
            thumb: 16.0,
        },
    }
}

fn unchecked_thumb(theme: &Theme) -> Color {
    if luminance(theme.palette.background) >= luminance(theme.palette.foreground) {
        theme.palette.background
    } else {
        theme.palette.foreground
    }
}

fn luminance(color: Color) -> f32 {
    let channel = |value: f32| {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn thumb_style(color: Color) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..iced::widget::container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};

    #[derive(Debug, Clone)]
    enum Message {
        Toggle,
    }

    #[test]
    fn switch_styles_distinguish_state_and_preserve_focus_color() {
        let off = style(&LIGHT, false, Status::Active);
        let on = style(&LIGHT, true, Status::Active);
        let hovered = style(&LIGHT, true, Status::Hovered);
        let disabled = style(&LIGHT, true, Status::Disabled);

        assert_eq!(off.background, Some(Background::Color(LIGHT.palette.input)));
        assert_eq!(
            on.background,
            Some(Background::Color(LIGHT.palette.primary))
        );
        assert_ne!(hovered.background, on.background);
        assert_eq!(on.focus_ring.color, LIGHT.palette.ring);
        assert!(disabled.border.color.a < on.border.color.a);

        for theme in [LIGHT, DARK] {
            let focused = style(&theme, true, Status::Focused);
            assert!(contrast(focused.focus_ring.color, theme.palette.background) >= 3.0);
            assert_eq!(focused.focus_offset, 0.0);

            let disabled = style(&theme, false, Status::Disabled);
            assert!((0.45..=0.5).contains(&disabled.border.color.a));
        }
    }

    #[test]
    fn switch_tree_is_a_focus_control_around_a_passive_track() {
        for checked in [false, true] {
            let switch: Element<'_, Message> =
                switch(widget::Id::unique(), checked, Message::Toggle, &LIGHT)
                    .disabled(false)
                    .into();

            let children = switch.as_widget().children();
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].children.len(), 2);
        }
    }

    #[test]
    fn track_geometry_leaves_equal_thumb_insets() {
        let small = metrics(SwitchSize::Small);
        let default = metrics(SwitchSize::Default);
        assert_eq!((small.width, small.height, small.thumb), (24.0, 14.0, 12.0));
        assert_eq!(
            (default.width, default.height, default.thumb),
            (32.0, 18.4, 16.0)
        );
        assert_eq!((small.height - small.thumb) / 2.0, 1.0);
        assert!(((default.height - default.thumb) / 2.0 - 1.2).abs() < 0.001);
    }

    #[test]
    fn unchecked_thumb_follows_light_and_dark_semantics() {
        assert_eq!(unchecked_thumb(&LIGHT), LIGHT.palette.background);
        assert_eq!(unchecked_thumb(&DARK), DARK.palette.foreground);
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}
