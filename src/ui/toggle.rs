use super::focus_control::{FocusControl, Status, Style};
use super::theme::{Theme, alpha, mix};
use iced::advanced::{Layout, Widget, layout, mouse, renderer, widget};
use iced::keyboard;
use iced::widget::container;
use iced::{
    Background, Border, Color, Element, Length, Padding, Rectangle, Shadow, Size, alignment, border,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToggleVariant {
    #[default]
    Default,
    Outline,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToggleSize {
    Small,
    #[default]
    Default,
    Large,
}

type KeyPressFn<'a, Message> = dyn Fn(keyboard::Key, keyboard::Modifiers) -> Option<Message> + 'a;

/// A controlled two-state control with pointer, touch, and keyboard activation.
///
/// `content` must be passive. The caller owns `pressed` and supplies the
/// message that replaces it, normally with the opposite value.
pub struct Toggle<'a, Message>
where
    Message: Clone + 'a,
{
    id: widget::Id,
    content: Element<'a, Message>,
    pressed: bool,
    on_toggle: Message,
    on_key_press: Option<Box<KeyPressFn<'a, Message>>>,
    variant: ToggleVariant,
    size: ToggleSize,
    disabled: bool,
    radius: border::Radius,
    theme: Theme,
}

pub fn toggle<'a, Message>(
    id: widget::Id,
    content: impl Into<Element<'a, Message>>,
    pressed: bool,
    on_toggle: Message,
    theme: &Theme,
) -> Toggle<'a, Message>
where
    Message: Clone + 'a,
{
    Toggle {
        id,
        content: content.into(),
        pressed,
        on_toggle,
        on_key_press: None,
        variant: ToggleVariant::Default,
        size: ToggleSize::Default,
        disabled: false,
        radius: theme.radius.md.into(),
        theme: *theme,
    }
}

impl<'a, Message> Toggle<'a, Message>
where
    Message: Clone + 'a,
{
    #[must_use]
    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub(crate) fn radius(mut self, radius: border::Radius) -> Self {
        self.radius = radius;
        self
    }

    /// Adds focused key handling for compound controls such as toggle groups.
    #[must_use]
    pub fn on_key_press(
        mut self,
        handler: impl Fn(keyboard::Key, keyboard::Modifiers) -> Option<Message> + 'a,
    ) -> Self {
        self.on_key_press = Some(Box::new(handler));
        self
    }

    pub fn into_widget(self) -> FocusControl<'a, Message> {
        let metrics = metrics(self.size);
        let content = container(self.content)
            .padding(Padding::default().horizontal(metrics.horizontal))
            .align_x(alignment::Horizontal::Center)
            .center_y(Length::Fixed(metrics.height));
        let content = MinimumWidth::new(content, metrics.minimum_width);
        let theme = self.theme;
        let variant = self.variant;
        let pressed = self.pressed;
        let radius = self.radius;
        let mut control = FocusControl::new(self.id, content, self.on_toggle, &theme)
            .disabled(self.disabled)
            .style(move |_iced_theme, status| {
                style_with_radius(&theme, variant, pressed, status, radius)
            });

        if let Some(handler) = self.on_key_press {
            control = control.on_key_press(handler);
        }

        control
    }
}

impl<'a, Message> From<Toggle<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(toggle: Toggle<'a, Message>) -> Self {
        toggle.into_widget().into()
    }
}

pub fn style(theme: &Theme, variant: ToggleVariant, pressed: bool, status: Status) -> Style {
    style_with_radius(theme, variant, pressed, status, theme.radius.md.into())
}

fn style_with_radius(
    theme: &Theme,
    variant: ToggleVariant,
    pressed: bool,
    status: Status,
    radius: border::Radius,
) -> Style {
    let palette = theme.palette;
    let disabled = status == Status::Disabled;
    let active_background = pressed.then_some(palette.accent);
    let mut background = match status {
        Status::Hovered if !pressed && variant == ToggleVariant::Default => Some(palette.muted),
        Status::Hovered if !pressed => Some(palette.accent),
        Status::Hovered => active_background.map(|color| mix(color, palette.foreground, 0.05)),
        Status::Pressed => Some(mix(palette.accent, palette.foreground, 0.10)),
        _ => active_background,
    };
    let mut foreground =
        if !pressed && status == Status::Hovered && variant == ToggleVariant::Default {
            palette.muted_foreground
        } else if pressed || matches!(status, Status::Hovered | Status::Pressed) {
            palette.accent_foreground
        } else {
            palette.foreground
        };
    let mut border_color = match variant {
        ToggleVariant::Default => Color::TRANSPARENT,
        ToggleVariant::Outline => palette.input,
    };

    if status == Status::Focused && variant == ToggleVariant::Outline {
        border_color = palette.ring;
    }

    if disabled {
        background = background.map(|color| alpha(color, 0.5));
        foreground = alpha(foreground, 0.5);
        border_color = alpha(border_color, 0.5);
    }

    Style {
        background: background.map(Background::Color),
        text_color: Some(foreground),
        border: Border {
            color: border_color,
            width: f32::from(variant == ToggleVariant::Outline),
            radius,
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: 2.0,
            radius: expanded_radius(radius, 2.0),
        },
        focus_offset: 0.0,
    }
}

fn expanded_radius(radius: border::Radius, amount: f32) -> border::Radius {
    border::Radius {
        top_left: radius.top_left + amount,
        top_right: radius.top_right + amount,
        bottom_right: radius.bottom_right + amount,
        bottom_left: radius.bottom_left + amount,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    horizontal: f32,
    height: f32,
    minimum_width: f32,
}

const fn metrics(size: ToggleSize) -> Metrics {
    match size {
        ToggleSize::Small => Metrics {
            horizontal: 6.0,
            height: 32.0,
            minimum_width: 32.0,
        },
        ToggleSize::Default => Metrics {
            horizontal: 8.0,
            height: 36.0,
            minimum_width: 36.0,
        },
        ToggleSize::Large => Metrics {
            horizontal: 10.0,
            height: 40.0,
            minimum_width: 40.0,
        },
    }
}

/// Iced has no public `min_width` builder. This passive wrapper only adds the
/// shadcn minimum hit width, then delegates layout and drawing to its child.
struct MinimumWidth<'a, Message> {
    content: Element<'a, Message>,
    minimum: f32,
}

impl<'a, Message> MinimumWidth<'a, Message> {
    fn new(content: impl Into<Element<'a, Message>>, minimum: f32) -> Self {
        Self {
            content: content.into(),
            minimum,
        }
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for MinimumWidth<'_, Message> {
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, self.content.as_widget().size().height)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.min_width(self.minimum),
        )
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }
}

impl<'a, Message: 'a> From<MinimumWidth<'a, Message>> for Element<'a, Message> {
    fn from(content: MinimumWidth<'a, Message>) -> Self {
        Element::new(content)
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::widget::text;

    #[derive(Debug, Clone)]
    enum Message {
        Toggle,
    }

    #[test]
    fn styles_expose_selection_feedback_and_semantic_focus() {
        let idle = style(&LIGHT, ToggleVariant::Default, false, Status::Active);
        let selected = style(&LIGHT, ToggleVariant::Default, true, Status::Active);
        let hovered = style(&LIGHT, ToggleVariant::Default, false, Status::Hovered);
        let outline = style(&LIGHT, ToggleVariant::Outline, false, Status::Active);
        let disabled = style(&LIGHT, ToggleVariant::Default, true, Status::Disabled);

        assert_eq!(idle.background, None);
        assert_eq!(
            selected.background,
            Some(Background::Color(LIGHT.palette.accent))
        );
        assert_eq!(
            hovered.background,
            Some(Background::Color(LIGHT.palette.muted))
        );
        assert_eq!(outline.border.color, LIGHT.palette.input);
        assert_eq!(outline.border.width, 1.0);
        assert_eq!(selected.focus_ring.color, LIGHT.palette.ring);
        assert!(disabled.text_color.unwrap().a < selected.text_color.unwrap().a);

        for theme in [LIGHT, DARK] {
            let focused = style(&theme, ToggleVariant::Outline, false, Status::Focused);
            assert!(contrast(focused.focus_ring.color, theme.palette.background) >= 3.0);
            assert_eq!(focused.border.color, theme.palette.ring);

            let disabled = style(&theme, ToggleVariant::Outline, true, Status::Disabled);
            assert!((0.45..=0.5).contains(&disabled.text_color.unwrap().a));
        }
    }

    #[test]
    fn toggle_tree_has_one_passive_child() {
        let toggle: Element<'_, Message> = toggle(
            widget::Id::unique(),
            text("Bold"),
            false,
            Message::Toggle,
            &LIGHT,
        )
        .disabled(true)
        .into();

        let children = toggle.as_widget().children();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].children.len(), 1);
        assert!(children[0].children[0].children.is_empty());
    }

    #[test]
    fn size_metrics_are_monotonic() {
        let small = metrics(ToggleSize::Small);
        let default = metrics(ToggleSize::Default);
        let large = metrics(ToggleSize::Large);

        assert!(small.height < default.height && default.height < large.height);
        assert!(small.horizontal < default.horizontal && default.horizontal < large.horizontal);
        assert_eq!(
            (small.horizontal, small.height, small.minimum_width),
            (6.0, 32.0, 32.0)
        );
        assert_eq!(
            (default.horizontal, default.height, default.minimum_width),
            (8.0, 36.0, 36.0)
        );
        assert_eq!(
            (large.horizontal, large.height, large.minimum_width),
            (10.0, 40.0, 40.0)
        );
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let luminance = |color: Color| {
            let channel = |value: f32| {
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}
