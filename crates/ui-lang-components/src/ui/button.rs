use super::focus_control::FocusControl;
use super::theme::{Theme, alpha, mix};
use iced::alignment::{Horizontal, Vertical};
use iced::font::Weight;
use iced::widget::text::IntoFragment;
use iced::widget::{button as iced_button, container, text};
use iced::{Background, Border, Color, Element, Font, Length, Padding};

type StyleFn<'a> = Box<dyn Fn(&iced::Theme, iced_button::Status) -> iced_button::Style + 'a>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
    Link,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    #[default]
    Default,
    Large,
    Icon,
}

/// A thin builder that becomes a native iced button.
pub struct Button<'a, Message>
where
    Message: Clone + 'a,
{
    content: Element<'a, Message>,
    on_press: Option<Message>,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    width: Length,
    height: Option<Length>,
    padding: Option<Padding>,
    alignment: Horizontal,
    style: Option<StyleFn<'a>>,
    theme: Theme,
}

pub fn button<'a, Message>(label: impl IntoFragment<'a>, theme: &Theme) -> Button<'a, Message>
where
    Message: Clone + 'a,
{
    Button::new(
        text(label)
            .size(theme.typography.caption)
            .font(label_font(theme)),
        theme,
    )
}

fn label_font(theme: &Theme) -> Font {
    Font {
        weight: Weight::Semibold,
        ..theme.typography.font
    }
}

impl<'a, Message> Button<'a, Message>
where
    Message: Clone + 'a,
{
    pub fn new(content: impl Into<Element<'a, Message>>, theme: &Theme) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            disabled: false,
            width: Length::Shrink,
            height: None,
            padding: None,
            alignment: Horizontal::Center,
            style: None,
            theme: *theme,
        }
    }

    #[must_use]
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    #[must_use]
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Sets horizontal content alignment when the button is wider than its label.
    #[must_use]
    pub fn align_x(mut self, alignment: Horizontal) -> Self {
        self.alignment = alignment;
        self
    }

    #[must_use]
    pub fn style(
        mut self,
        style: impl Fn(&iced::Theme, iced_button::Status) -> iced_button::Style + 'a,
    ) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn into_widget(self) -> Element<'a, Message> {
        let geometry = geometry(&self.theme, self.variant, self.size);
        let width = if self.size == ButtonSize::Icon {
            Length::Fixed(self.theme.controls.icon_size)
        } else {
            self.width
        };
        let content_width = if width == Length::Shrink {
            Length::Shrink
        } else {
            Length::Fill
        };
        let content = container(self.content)
            .width(content_width)
            .height(Length::Fill)
            .align_x(self.alignment)
            .align_y(Vertical::Center);
        let theme = self.theme;
        let tokens = ButtonTokens::from(&theme);
        let variant = self.variant;
        let on_press = (!self.disabled).then_some(self.on_press).flatten();
        let widget = iced_button(content)
            .padding(self.padding.unwrap_or(geometry.padding))
            .width(width)
            .on_press_maybe(on_press.clone());
        let height = self.height.or(geometry.height.map(Length::Fixed));
        let widget = if let Some(height) = height {
            widget.height(height)
        } else {
            widget
        };
        let widget = match self.style {
            Some(custom) => widget.style(custom),
            None => widget.style(move |_iced_theme, status| style(tokens, variant, status)),
        };

        match on_press {
            Some(message) => FocusControl::anonymous(widget, message, &theme).into(),
            None => widget.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ButtonGeometry {
    padding: Padding,
    height: Option<f32>,
}

fn geometry(theme: &Theme, variant: ButtonVariant, size: ButtonSize) -> ButtonGeometry {
    let controls = theme.controls;
    let (padding, height) = match size {
        ButtonSize::Small => (controls.small_padding, Some(controls.small_height)),
        ButtonSize::Large => (controls.large_padding, Some(controls.large_height)),
        ButtonSize::Icon => ([0.0, 0.0], Some(controls.icon_size)),
        ButtonSize::Default => match variant {
            ButtonVariant::Default | ButtonVariant::Destructive => {
                (controls.primary_padding, controls.default_height)
            }
            ButtonVariant::Secondary => (controls.secondary_padding, controls.default_height),
            ButtonVariant::Outline | ButtonVariant::Ghost => {
                (controls.compact_padding, controls.default_height)
            }
            ButtonVariant::Link => ([0.0, 0.0], controls.default_height),
        },
    };
    ButtonGeometry {
        padding: padding.into(),
        height,
    }
}

impl<'a, Message> From<Button<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(button: Button<'a, Message>) -> Self {
        button.into_widget()
    }
}

/// The theme tokens a button's style closure reads.
///
/// Every styled widget boxes its style closure, so capturing these instead of
/// the whole 1.1 KB `Theme` keeps the copy off the heap once per button per
/// frame.
#[derive(Debug, Clone, Copy)]
pub struct ButtonTokens {
    accent: Color,
    accent_foreground: Color,
    border: Color,
    brand: Color,
    card: Color,
    control_line: Color,
    destructive: Color,
    destructive_foreground: Color,
    disabled: Color,
    disabled_foreground: Color,
    foreground: Color,
    primary: Color,
    primary_foreground: Color,
    primary_hover: Color,
    secondary: Color,
    secondary_foreground: Color,
    radius: f32,
}

impl From<&Theme> for ButtonTokens {
    fn from(theme: &Theme) -> Self {
        let palette = theme.palette;
        Self {
            accent: palette.accent,
            accent_foreground: palette.accent_foreground,
            border: palette.border,
            brand: palette.brand,
            card: palette.card,
            control_line: palette.control_line,
            destructive: palette.destructive,
            destructive_foreground: palette.destructive_foreground,
            disabled: palette.disabled,
            disabled_foreground: palette.disabled_foreground,
            foreground: palette.foreground,
            primary: palette.primary,
            primary_foreground: palette.primary_foreground,
            primary_hover: palette.primary_hover,
            secondary: palette.secondary,
            secondary_foreground: palette.secondary_foreground,
            radius: theme.radius.button,
        }
    }
}

pub fn style(
    tokens: ButtonTokens,
    variant: ButtonVariant,
    status: iced_button::Status,
) -> iced_button::Style {
    let palette = tokens;
    let (mut background, mut foreground, border_color, border_width) = match variant {
        ButtonVariant::Default => (
            Some(palette.primary),
            palette.primary_foreground,
            palette.primary,
            0.0,
        ),
        ButtonVariant::Destructive => (
            Some(palette.destructive),
            palette.destructive_foreground,
            palette.destructive,
            0.0,
        ),
        ButtonVariant::Secondary => (
            Some(palette.secondary),
            palette.secondary_foreground,
            palette.control_line,
            1.0,
        ),
        ButtonVariant::Outline => (
            Some(palette.card),
            palette.accent_foreground,
            palette.border,
            1.0,
        ),
        ButtonVariant::Ghost => (None, palette.foreground, Color::TRANSPARENT, 0.0),
        ButtonVariant::Link => (None, palette.brand, Color::TRANSPARENT, 0.0),
    };

    match status {
        iced_button::Status::Hovered => match variant {
            ButtonVariant::Outline | ButtonVariant::Ghost => {
                background = Some(palette.accent);
                foreground = palette.accent_foreground;
            }
            ButtonVariant::Link => foreground = mix(foreground, palette.foreground, 0.25),
            ButtonVariant::Destructive => {
                background = background.map(|color| mix(color, palette.foreground, 0.08));
            }
            ButtonVariant::Default => background = Some(palette.primary_hover),
            _ => background = background.map(|color| mix(color, foreground, 0.08)),
        },
        iced_button::Status::Pressed => match variant {
            ButtonVariant::Outline | ButtonVariant::Ghost => {
                background = Some(mix(palette.accent, palette.foreground, 0.08));
                foreground = palette.accent_foreground;
            }
            ButtonVariant::Link => foreground = mix(foreground, palette.foreground, 0.40),
            ButtonVariant::Destructive => {
                background = background.map(|color| mix(color, palette.foreground, 0.16));
            }
            _ => background = background.map(|color| mix(color, foreground, 0.16)),
        },
        iced_button::Status::Disabled => {
            if variant == ButtonVariant::Default {
                background = Some(palette.disabled);
                foreground = palette.disabled_foreground;
            } else {
                background = background.map(|color| alpha(color, 0.5));
                foreground = alpha(foreground, 0.5);
            }
        }
        iced_button::Status::Active => {}
    }

    iced_button::Style {
        background: background.map(Background::Color),
        text_color: foreground,
        border: Border {
            color: if status == iced_button::Status::Disabled {
                alpha(border_color, 0.5)
            } else {
                border_color
            },
            width: border_width,
            radius: if matches!(variant, ButtonVariant::Outline | ButtonVariant::Ghost) {
                8.0.into()
            } else {
                tokens.radius.into()
            },
        },
        ..iced_button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::State as FocusState;
    use super::super::theme::{DARK, LIGHT, SHADCN_DARK, SHADCN_LIGHT};
    use super::*;
    use iced::advanced::widget;

    #[test]
    fn disabled_button_uses_the_semantic_disabled_pair() {
        let active = style(
            ButtonTokens::from(&LIGHT),
            ButtonVariant::Default,
            iced_button::Status::Active,
        );
        let disabled = style(
            ButtonTokens::from(&LIGHT),
            ButtonVariant::Default,
            iced_button::Status::Disabled,
        );
        assert_ne!(disabled.text_color, active.text_color);
        assert_eq!(disabled.text_color, LIGHT.palette.disabled_foreground);
        assert_eq!(
            disabled.background,
            Some(Background::Color(LIGHT.palette.disabled))
        );
    }

    #[test]
    fn content_alignment_is_explicit_and_configurable() {
        let centered: Button<'_, ()> = button("Centered", &LIGHT);
        assert_eq!(centered.alignment, Horizontal::Center);

        let leading: Button<'_, ()> = button("Leading", &LIGHT).align_x(Horizontal::Left);
        assert_eq!(leading.alignment, Horizontal::Left);
    }

    #[test]
    fn label_weight_inherits_the_theme_font_family() {
        let mut theme = LIGHT;
        theme.typography.font = Font::with_name("Product Sans");

        let font = label_font(&theme);

        assert_eq!(font.family, Font::with_name("Product Sans").family);
        assert_eq!(font.weight, Weight::Semibold);
    }

    #[test]
    fn geometry_and_style_can_be_overridden_without_copying_source() {
        let custom: Button<'_, ()> = Button::new(text("Custom"), &LIGHT)
            .height(48)
            .padding(20)
            .style(|_theme, _status| iced_button::Style::default());

        assert_eq!(custom.height, Some(Length::Fixed(48.0)));
        assert_eq!(custom.padding, Some(Padding::new(20.0)));
        assert!(custom.style.is_some());
    }

    #[test]
    fn default_variants_match_canonical_button_geometry() {
        for (variant, vertical, horizontal) in [
            (ButtonVariant::Default, 11.0, 16.0),
            (ButtonVariant::Destructive, 11.0, 16.0),
            (ButtonVariant::Secondary, 11.0, 16.0),
            (ButtonVariant::Outline, 8.0, 12.0),
        ] {
            assert_eq!(
                geometry(&LIGHT, variant, ButtonSize::Default),
                ButtonGeometry {
                    padding: [vertical, horizontal].into(),
                    height: None,
                }
            );
        }
        assert_eq!(
            style(
                ButtonTokens::from(&LIGHT),
                ButtonVariant::Outline,
                iced_button::Status::Active
            )
            .border
            .radius,
            8.0.into()
        );
    }

    #[test]
    fn shadcn_profile_changes_control_geometry_without_changing_the_button_api() {
        assert_eq!(
            geometry(&SHADCN_LIGHT, ButtonVariant::Default, ButtonSize::Default),
            ButtonGeometry {
                padding: [8.0, 16.0].into(),
                height: Some(36.0),
            }
        );
        assert_eq!(
            style(
                ButtonTokens::from(&SHADCN_LIGHT),
                ButtonVariant::Default,
                iced_button::Status::Active
            )
            .border
            .radius,
            8.0.into()
        );
    }

    #[test]
    fn interactive_buttons_join_keyboard_focus_order() {
        let button: Element<'_, ()> = button("Save", &LIGHT).on_press(()).into_widget();
        let tree = widget::Tree::new(button.as_widget());
        assert!(!tree.state.downcast_ref::<FocusState>().is_focused());
    }

    #[test]
    fn enabled_button_labels_keep_normal_text_contrast() {
        for theme in [LIGHT, DARK, SHADCN_LIGHT, SHADCN_DARK] {
            for variant in [
                ButtonVariant::Default,
                ButtonVariant::Destructive,
                ButtonVariant::Secondary,
                ButtonVariant::Outline,
                ButtonVariant::Ghost,
                ButtonVariant::Link,
            ] {
                for status in [
                    iced_button::Status::Active,
                    iced_button::Status::Hovered,
                    iced_button::Status::Pressed,
                ] {
                    let appearance = style(ButtonTokens::from(&theme), variant, status);
                    let background = match appearance.background {
                        Some(Background::Color(color)) => color,
                        _ => theme.palette.background,
                    };
                    assert!(
                        appearance.text_color.relative_contrast(background) >= 4.5,
                        "{} {variant:?} {status:?}",
                        theme.name
                    );
                }
            }
        }
    }

    #[test]
    fn link_buttons_use_the_sparse_brand_action_color() {
        let link = style(
            ButtonTokens::from(&LIGHT),
            ButtonVariant::Link,
            iced_button::Status::Active,
        );

        assert_eq!(link.text_color, LIGHT.palette.brand);
    }
}
