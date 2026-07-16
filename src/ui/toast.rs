use super::theme::{Theme, alpha, mix};
use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row, container};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Vector};
use std::time::Duration;

pub const DEFAULT_DURATION: Duration = Duration::from_secs(5);
pub const TOAST_WIDTH: f32 = 356.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastVariant {
    #[default]
    Default,
    Success,
    Info,
    Warning,
    Destructive,
    Loading,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastDuration {
    #[default]
    Default,
    Auto(Duration),
    Persistent,
}

/// Text and timing owned by the application and rendered by `sonner`.
///
/// The title must contain visible text. iced does not currently expose live
/// region roles, so a toast must not rely on an invisible announcement alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastData {
    title: String,
    description: Option<String>,
    action: Option<String>,
    variant: ToastVariant,
    duration: ToastDuration,
}

impl ToastData {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        assert!(
            !title.trim().is_empty(),
            "a toast needs a visible status title"
        );

        Self {
            title,
            description: None,
            action: None,
            variant: ToastVariant::Default,
            duration: ToastDuration::Default,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description_text(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn action_label(&self) -> Option<&str> {
        self.action.as_deref()
    }

    pub const fn toast_variant(&self) -> ToastVariant {
        self.variant
    }

    pub const fn toast_duration(&self) -> ToastDuration {
        self.duration
    }

    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        let description = description.into();
        self.description = (!description.trim().is_empty()).then_some(description);
        self
    }

    #[must_use]
    pub fn action(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        assert!(
            !label.trim().is_empty(),
            "a toast action needs a visible label"
        );
        self.action = Some(label);
        self
    }

    #[must_use]
    pub const fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = ToastDuration::Auto(duration);
        self
    }

    #[must_use]
    pub const fn persistent(mut self) -> Self {
        self.duration = ToastDuration::Persistent;
        self
    }
}

/// A compositional legacy toast surface.
///
/// The caller owns open state and supplies controls as elements. Keep the
/// title visible: iced 0.14 has no live-region role to announce hidden text.
pub struct Toast<'a, Message> {
    title: Element<'a, Message>,
    description: Option<Element<'a, Message>>,
    action: Option<Element<'a, Message>>,
    dismiss: Option<Element<'a, Message>>,
    variant: ToastVariant,
    width: Length,
    theme: Theme,
}

pub fn toast<'a, Message>(
    title: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Toast<'a, Message>
where
    Message: 'a,
{
    Toast {
        title: title.into(),
        description: None,
        action: None,
        dismiss: None,
        variant: ToastVariant::Default,
        width: Length::Fixed(TOAST_WIDTH),
        theme: *theme,
    }
}

impl<'a, Message> Toast<'a, Message>
where
    Message: 'a,
{
    #[must_use]
    pub fn description(mut self, description: impl Into<Element<'a, Message>>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.action = Some(action.into());
        self
    }

    #[must_use]
    pub fn dismiss(mut self, dismiss: impl Into<Element<'a, Message>>) -> Self {
        self.dismiss = Some(dismiss.into());
        self
    }

    #[must_use]
    pub const fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn into_widget(self) -> Container<'a, Message> {
        let mut copy = Column::new()
            .push(self.title)
            .spacing(4)
            .align_x(Horizontal::Left)
            .width(Length::Fill);
        if let Some(description) = self.description {
            copy = copy.push(description);
        }

        let has_controls = self.action.is_some() || self.dismiss.is_some();
        let mut controls = Row::new().spacing(8).align_y(Alignment::Center);
        if let Some(action) = self.action {
            controls = controls.push(action);
        }
        if let Some(dismiss) = self.dismiss {
            controls = controls.push(dismiss);
        }

        let mut content = Row::new().push(copy).align_y(Alignment::Center);
        if has_controls {
            content = content.push(controls).spacing(12);
        }
        let theme = self.theme;
        let variant = self.variant;

        container(content)
            .width(self.width)
            .padding([12, 16])
            .style(move |_iced_theme| style(&theme, variant))
    }
}

impl<'a, Message> From<Toast<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(toast: Toast<'a, Message>) -> Self {
        toast.into_widget().into()
    }
}

pub fn style(theme: &Theme, variant: ToastVariant) -> iced::widget::container::Style {
    let palette = theme.palette;
    let (background, foreground, border) = match variant {
        ToastVariant::Default => (palette.card, palette.card_foreground, palette.input),
        ToastVariant::Success => semantic_tint(theme, palette.success),
        ToastVariant::Info => semantic_tint(theme, palette.ring),
        ToastVariant::Warning => semantic_tint(theme, palette.warning),
        ToastVariant::Destructive => semantic_tint(theme, palette.destructive),
        ToastVariant::Loading => (palette.muted, palette.foreground, palette.border),
    };

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(foreground),
        border: Border {
            color: border,
            width: 1.0,
            radius: theme.radius.lg.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.18),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

fn semantic_tint(theme: &Theme, tone: Color) -> (Color, Color, Color) {
    (
        mix(theme.palette.background, tone, 0.10),
        theme.palette.foreground,
        mix(theme.palette.background, tone, 0.32),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};
    use iced::widget::text;

    #[test]
    fn data_requires_visible_status_and_normalizes_empty_description() {
        let data = ToastData::new("Saved").description("   ");
        assert_eq!(data.title(), "Saved");
        assert_eq!(data.description_text(), None);

        assert!(std::panic::catch_unwind(|| ToastData::new(" \n ")).is_err());
        assert!(std::panic::catch_unwind(|| ToastData::new("Saved").action("")).is_err());
    }

    #[test]
    fn composed_tree_keeps_copy_and_controls_in_separate_aligned_groups() {
        let toast: Element<'_, ()> = toast(text("Saved"), &LIGHT)
            .description(text("The file is on disk."))
            .action(text("Undo"))
            .dismiss(text("Dismiss"))
            .into();
        let root = toast.as_widget().children();

        assert_eq!(root.len(), 2);
        assert_eq!(root[0].children.len(), 2);
        assert_eq!(root[1].children.len(), 2);
    }

    #[test]
    fn all_variants_keep_normal_text_contrast_in_both_themes() {
        let variants = [
            ToastVariant::Default,
            ToastVariant::Success,
            ToastVariant::Info,
            ToastVariant::Warning,
            ToastVariant::Destructive,
            ToastVariant::Loading,
        ];

        for theme in [LIGHT, DARK] {
            for variant in variants {
                let appearance = style(&theme, variant);
                let Some(Background::Color(background)) = appearance.background else {
                    panic!("toast must have an opaque surface");
                };
                let foreground = appearance.text_color.expect("toast needs text color");
                assert_eq!(background.a, 1.0);
                assert!(
                    contrast(foreground, background) >= 4.5,
                    "{} {variant:?}",
                    theme.name
                );
            }
        }
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (lighter, darker) = if luminance(a) > luminance(b) {
            (luminance(a), luminance(b))
        } else {
            (luminance(b), luminance(a))
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn luminance(color: Color) -> f32 {
        fn channel(value: f32) -> f32 {
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }
}
