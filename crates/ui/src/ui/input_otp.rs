use super::theme::{Theme, alpha};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Row, container, text, text_input};
use iced::{Background, Border, Color, Element, Event, Length, Rectangle, Size, Vector, touch};

const SLOT_SIZE: f32 = 40.0;
const SLOT_GAP: f32 = 2.0;
const SEPARATOR_WIDTH: f32 = 18.0;
type RendererParagraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph;

#[derive(Clone, Copy)]
pub enum OtpPattern {
    Digits,
    Alphanumeric,
    Custom(fn(char) -> bool),
}

impl OtpPattern {
    pub fn accepts(self, character: char) -> bool {
        match self {
            Self::Digits => character.is_ascii_digit(),
            Self::Alphanumeric => character.is_ascii_alphanumeric(),
            Self::Custom(accepts) => accepts(character),
        }
    }
}

/// Filters pasted or typed input and bounds it to the requested slot count.
pub fn normalize(value: &str, length: usize, pattern: OtpPattern) -> String {
    value
        .chars()
        .filter(|character| pattern.accepts(*character))
        .take(length)
        .collect()
}

pub fn is_complete(value: &str, length: usize, pattern: OtpPattern) -> bool {
    length > 0 && normalize(value, length, pattern).chars().count() == length
}

/// A controlled, copy/paste-capable one-time-password input.
///
/// One transparent native iced text input owns focus, selection, typing,
/// backspace, and paste. The slot layer renders the controlled
/// value without splitting keyboard state across several fields.
pub struct InputOtp<'a, Message> {
    value: &'a str,
    length: usize,
    pattern: OtpPattern,
    on_change: Box<dyn Fn(String) -> Message + 'a>,
    groups: Vec<usize>,
    id: Option<iced::widget::Id>,
    invalid: bool,
    disabled: bool,
    separator: Option<Box<dyn Fn() -> Element<'a, Message> + 'a>>,
    theme: Theme,
}

pub fn input_otp<'a, Message>(
    value: &'a str,
    length: usize,
    pattern: OtpPattern,
    on_change: impl Fn(String) -> Message + 'a,
    theme: &Theme,
) -> InputOtp<'a, Message> {
    InputOtp {
        value,
        length: length.max(1),
        pattern,
        on_change: Box::new(on_change),
        groups: Vec::new(),
        id: None,
        invalid: false,
        disabled: false,
        separator: None,
        theme: *theme,
    }
}

impl<'a, Message> InputOtp<'a, Message>
where
    Message: Clone + 'a,
{
    /// Adds visual separators after each group except the final group.
    #[must_use]
    pub fn groups(mut self, groups: impl IntoIterator<Item = usize>) -> Self {
        self.groups = groups.into_iter().filter(|size| *size > 0).collect();
        self
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<iced::widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Replaces the visual separator rendered between configured groups.
    #[must_use]
    pub fn separator(mut self, separator: impl Fn() -> Element<'a, Message> + 'a) -> Self {
        self.separator = Some(Box::new(separator));
        self
    }

    fn into_element(self) -> Element<'a, Message> {
        let value = normalize(self.value, self.length, self.pattern);
        let characters = value.chars().collect::<Vec<_>>();
        let separators = separator_indices(self.length, &self.groups);
        let separator_count = separators.len();
        let width = self.length as f32 * SLOT_SIZE
            + (self.length + separator_count).saturating_sub(1) as f32 * SLOT_GAP
            + separator_count as f32 * SEPARATOR_WIDTH;

        let mut slots = Row::new().spacing(SLOT_GAP).height(SLOT_SIZE);
        for index in 0..self.length {
            let character = characters.get(index).copied();
            slots = slots.push(slot(character, self.invalid, self.disabled, &self.theme));
            if separators.contains(&(index + 1)) {
                let separator = self.separator.as_ref().map_or_else(
                    || {
                        text("–")
                            .font(self.theme.typography.font)
                            .color(self.theme.palette.muted_foreground)
                            .into()
                    },
                    |separator| separator(),
                );
                slots = slots.push(
                    container(separator)
                        .width(SEPARATOR_WIDTH)
                        .height(SLOT_SIZE)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center),
                );
            }
        }

        if self.disabled {
            return slots.into();
        }

        let pattern = self.pattern;
        let length = self.length;
        let on_change = self.on_change;
        let mut input = text_input("", &value)
            .on_input(move |raw: String| on_change(normalize(&raw, length, pattern)))
            .width(width)
            .padding([10, 0])
            .style(move |_iced_theme, status| overlay_style(status));
        if let Some(id) = self.id {
            input = input.id(id);
        }

        OtpWidget {
            slots: slots.into(),
            input: input.into(),
            value,
            length: self.length,
            separators,
            width,
            invalid: self.invalid,
            theme: self.theme,
        }
        .into()
    }
}

impl<'a, Message> From<InputOtp<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(input: InputOtp<'a, Message>) -> Self {
        input.into_element()
    }
}

fn separator_indices(length: usize, groups: &[usize]) -> Vec<usize> {
    let mut end: usize = 0;
    groups
        .iter()
        .filter_map(|size| {
            end = end.saturating_add(*size);
            (end < length).then_some(end)
        })
        .collect()
}

fn slot<'a, Message>(
    character: Option<char>,
    invalid: bool,
    disabled: bool,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let copy = character.map_or_else(String::new, |character| character.to_string());
    let foreground = if disabled {
        alpha(theme.palette.foreground, 0.5)
    } else {
        theme.palette.foreground
    };
    let style_theme = *theme;

    container(
        text(copy)
            .size(theme.typography.section_title)
            .font(theme.typography.font)
            .color(foreground),
    )
    .width(SLOT_SIZE)
    .height(SLOT_SIZE)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .style(move |_iced_theme| slot_style(&style_theme, invalid, disabled))
    .into()
}

pub fn slot_style(theme: &Theme, invalid: bool, disabled: bool) -> iced::widget::container::Style {
    let border = if invalid {
        theme.palette.destructive
    } else {
        theme.palette.input
    };

    iced::widget::container::Style {
        background: Some(Background::Color(if disabled {
            alpha(theme.palette.muted, 0.5)
        } else {
            theme.palette.background
        })),
        border: Border {
            color: border,
            width: if invalid { 2.0 } else { 1.0 },
            radius: theme.radius.button.into(),
        },
        ..Default::default()
    }
}

pub fn overlay_style(_status: iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: Color::TRANSPARENT,
        placeholder: Color::TRANSPARENT,
        value: Color::TRANSPARENT,
        selection: Color::TRANSPARENT,
    }
}

struct OtpWidget<'a, Message> {
    slots: Element<'a, Message>,
    input: Element<'a, Message>,
    value: String,
    length: usize,
    separators: Vec<usize>,
    width: f32,
    invalid: bool,
    theme: Theme,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for OtpWidget<'_, Message> {
    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.slots),
            widget::Tree::new(&self.input),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.slots.as_widget(), self.input.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.width), Length::Fixed(SLOT_SIZE))
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(
            Length::Fixed(self.width),
            Length::Fixed(SLOT_SIZE),
            Size::new(self.width, SLOT_SIZE),
        );
        let child_limits = layout::Limits::new(size, size);
        let slots =
            self.slots
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &child_limits);
        let input =
            self.input
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &child_limits);

        layout::Node::with_children(size, vec![slots, input])
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.slots.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().expect("OTP slots layout"),
                renderer,
                operation,
            );
        });
        operation.traverse(&mut |operation| {
            self.input.as_widget_mut().operate(
                &mut tree.children[1],
                layout.children().nth(1).expect("OTP input layout"),
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let input_layout = layout.children().nth(1).expect("OTP input layout");
        self.input.as_widget_mut().update(
            &mut tree.children[1],
            event,
            input_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let press_position = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor.position(),
            Event::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
            _ => None,
        };
        if let Some(position) =
            press_position.filter(|position| layout.bounds().contains(*position))
        {
            let index = slot_index_at_x(
                position.x - layout.bounds().x,
                self.length,
                &self.separators,
            );
            let value_length = self.value.chars().count();
            let input_state = tree.children[1]
                .state
                .downcast_mut::<text_input::State<RendererParagraph>>();
            if index < value_length {
                input_state.select_range(index, index + 1);
            } else {
                input_state.move_cursor_to(value_length);
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.input.as_widget().mouse_interaction(
            &tree.children[1],
            layout.children().nth(1).expect("OTP input layout"),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut children = layout.children();
        let slots_layout = children.next().expect("OTP slots layout");
        let input_layout = children.next().expect("OTP input layout");
        self.slots.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            slots_layout,
            cursor,
            viewport,
        );
        self.input.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            renderer_style,
            input_layout,
            cursor,
            viewport,
        );

        let input_state = tree.children[1]
            .state
            .downcast_ref::<text_input::State<RendererParagraph>>();
        if let Some(index) = active_slot(input_state, &self.value, self.length) {
            let bounds = slot_bounds(layout.bounds(), index, &self.separators);
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        color: if self.invalid {
                            self.theme.palette.destructive
                        } else {
                            self.theme.palette.ring
                        },
                        width: 2.0,
                        radius: self.theme.radius.button.into(),
                    },
                    ..renderer::Quad::default()
                },
                Background::Color(Color::TRANSPARENT),
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.input.as_widget_mut().overlay(
            &mut tree.children[1],
            layout.children().nth(1).expect("OTP input layout"),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message> From<OtpWidget<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(widget: OtpWidget<'a, Message>) -> Self {
        Element::new(widget)
    }
}

fn slot_x(index: usize, separators: &[usize]) -> f32 {
    let preceding_separators = separators
        .iter()
        .filter(|boundary| **boundary <= index)
        .count();
    index as f32 * (SLOT_SIZE + SLOT_GAP)
        + preceding_separators as f32 * (SEPARATOR_WIDTH + SLOT_GAP)
}

fn slot_bounds(group: Rectangle, index: usize, separators: &[usize]) -> Rectangle {
    Rectangle {
        x: group.x + slot_x(index, separators),
        y: group.y,
        width: SLOT_SIZE,
        height: SLOT_SIZE,
    }
}

fn slot_index_at_x(x: f32, length: usize, separators: &[usize]) -> usize {
    (0..length)
        .min_by(|left, right| {
            let left_distance = (slot_x(*left, separators) + SLOT_SIZE / 2.0 - x).abs();
            let right_distance = (slot_x(*right, separators) + SLOT_SIZE / 2.0 - x).abs();
            left_distance.total_cmp(&right_distance)
        })
        .unwrap_or(0)
}

fn active_slot<P: iced::advanced::text::Paragraph>(
    state: &text_input::State<P>,
    value: &str,
    length: usize,
) -> Option<usize> {
    if !state.is_focused() || length == 0 {
        return None;
    }

    let value = text_input::Value::new(value);
    let index = match state.cursor().state(&value) {
        text_input::cursor::State::Index(index) => index,
        text_input::cursor::State::Selection { start, end } => start.min(end),
    };
    Some(index.min(length - 1))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::super::focus_control::focusable_count;
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn normalization_supports_digits_alphanumeric_and_custom_patterns() {
        assert_eq!(normalize("1 a2-3٤", 4, OtpPattern::Digits), "123");
        assert_eq!(normalize("a-1_B2", 4, OtpPattern::Alphanumeric), "a1B2");
        assert_eq!(
            normalize("ABcd12", 3, OtpPattern::Custom(char::is_uppercase)),
            "AB"
        );
        assert!(is_complete("12 34", 4, OtpPattern::Digits));
        assert!(!is_complete("123", 4, OtpPattern::Digits));
    }

    #[test]
    fn group_boundaries_never_add_a_trailing_separator() {
        assert_eq!(separator_indices(6, &[3, 3]), [3]);
        assert_eq!(separator_indices(6, &[2, 2, 2]), [2, 4]);
        assert_eq!(separator_indices(4, &[8]), Vec::<usize>::new());
    }

    #[test]
    fn native_input_stays_invisible_while_slots_own_focus_paint() {
        let empty = slot_style(&LIGHT, false, false);
        let invalid = slot_style(&LIGHT, true, false);
        let disabled = slot_style(&LIGHT, false, true);
        let focused =
            overlay_style(iced::widget::text_input::Status::Focused { is_hovered: false });

        assert_eq!(empty.border.color, LIGHT.palette.input);
        assert_eq!(empty.border.width, 1.0);
        assert_eq!(invalid.border.color, LIGHT.palette.destructive);
        assert_eq!(invalid.border.width, 2.0);
        assert_eq!(disabled.border.color, LIGHT.palette.input);
        assert_eq!(disabled.border.width, 1.0);
        assert_eq!(focused.border.width, 0.0);
        assert_eq!(focused.value, Color::TRANSPARENT);
    }

    #[test]
    fn active_slot_follows_the_native_caret() {
        let mut state = text_input::State::<RendererParagraph>::new();
        assert_eq!(active_slot(&state, "12", 6), None);

        state.focus();
        assert_eq!(active_slot(&state, "12", 6), Some(2));
        state.move_cursor_to(1);
        assert_eq!(active_slot(&state, "12", 6), Some(1));
        state.select_range(1, 2);
        assert_eq!(active_slot(&state, "12", 6), Some(1));
        state.move_cursor_to_end();
        assert_eq!(active_slot(&state, "123456", 6), Some(5));
    }

    #[test]
    fn separator_geometry_matches_the_row_and_pointer_targets() {
        let separators = [3];
        assert_eq!(slot_x(0, &separators), 0.0);
        assert_eq!(slot_x(2, &separators), 84.0);
        assert_eq!(slot_x(3, &separators), 146.0);
        assert_eq!(slot_index_at_x(20.0, 6, &separators), 0);
        assert_eq!(slot_index_at_x(166.0, 6, &separators), 3);

        let total_width = 6.0 * SLOT_SIZE
            + (6.0 + separators.len() as f32 - 1.0) * SLOT_GAP
            + separators.len() as f32 * SEPARATOR_WIDTH;
        assert_eq!(total_width, 270.0);
        assert_eq!(slot_x(5, &separators) + SLOT_SIZE, total_width);
    }

    #[test]
    fn disabled_input_has_no_hidden_focus_target() {
        let enabled: Element<'_, ()> =
            input_otp("12a", 4, OtpPattern::Digits, |_| (), &LIGHT).into();
        let disabled: Element<'_, ()> = input_otp("12a", 4, OtpPattern::Digits, |_| (), &LIGHT)
            .disabled(true)
            .into();

        assert_eq!(focusable_count(enabled), 1);
        assert_eq!(focusable_count(disabled), 0);
    }

    #[test]
    fn caller_separator_is_rendered_at_each_group_boundary() {
        let calls = Rc::new(Cell::new(0));
        let render_calls = Rc::clone(&calls);
        let _: Element<'_, ()> = input_otp("1234", 4, OtpPattern::Digits, |_| (), &LIGHT)
            .groups([2, 2])
            .separator(move || {
                render_calls.set(render_calls.get() + 1);
                text("custom").into()
            })
            .into();

        assert_eq!(calls.get(), 1);
    }
}
