use super::theme::Theme;
use super::toggle::{ToggleSize, ToggleVariant, toggle};
use iced::advanced::widget;
use iced::keyboard::{self, key::Named};
use iced::widget::{Column, Row};
use iced::{Alignment, Element, Length, border};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToggleGroupOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Controlled selection for a single- or multiple-value toggle group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleGroupState<Value> {
    Single(Option<Value>),
    Multiple(Vec<Value>),
}

impl<Value: Eq> ToggleGroupState<Value> {
    pub fn is_selected(&self, value: &Value) -> bool {
        match self {
            Self::Single(selected) => selected.as_ref() == Some(value),
            Self::Multiple(selected) => selected.contains(value),
        }
    }
}

impl<Value: Clone + Eq> ToggleGroupState<Value> {
    #[must_use]
    pub fn toggled(&self, value: Value) -> Self {
        match self {
            Self::Single(selected) if selected.as_ref() == Some(&value) => Self::Single(None),
            Self::Single(_) => Self::Single(Some(value)),
            Self::Multiple(selected) => {
                let mut next = selected.clone();
                if self.is_selected(&value) {
                    next.retain(|selected| selected != &value);
                } else {
                    next.push(value);
                }
                Self::Multiple(next)
            }
        }
    }
}

/// One controlled item rendered by [`toggle_group`].
pub struct ToggleGroupItem<'a, Message, Value> {
    id: widget::Id,
    value: Value,
    content: Element<'a, Message>,
    on_toggle: Message,
    disabled: bool,
}

pub fn toggle_group_item<'a, Message, Value>(
    id: widget::Id,
    value: Value,
    content: impl Into<Element<'a, Message>>,
    on_toggle: Message,
) -> ToggleGroupItem<'a, Message, Value>
where
    Message: 'a,
{
    ToggleGroupItem {
        id,
        value,
        content: content.into(),
        on_toggle,
        disabled: false,
    }
}

impl<Message, Value> ToggleGroupItem<'_, Message, Value> {
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

type NavigateFn<'a, Message> = dyn Fn(usize) -> Message + 'a;

/// A controlled group of keyboard-focusable toggles.
///
/// `on_navigate` receives the item index selected by arrow, Home, or End. The
/// caller should focus that item's stable `widget::Id` with an iced operation.
pub struct ToggleGroup<'a, Message, Value>
where
    Message: Clone + 'a,
{
    items: Vec<ToggleGroupItem<'a, Message, Value>>,
    state: &'a ToggleGroupState<Value>,
    orientation: ToggleGroupOrientation,
    on_navigate: Rc<NavigateFn<'a, Message>>,
    variant: ToggleVariant,
    size: ToggleSize,
    spacing: f32,
    disabled: bool,
    theme: Theme,
}

pub fn toggle_group<'a, Message, Value>(
    items: impl IntoIterator<Item = ToggleGroupItem<'a, Message, Value>>,
    state: &'a ToggleGroupState<Value>,
    orientation: ToggleGroupOrientation,
    on_navigate: impl Fn(usize) -> Message + 'a,
    theme: &Theme,
) -> ToggleGroup<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Eq + 'a,
{
    ToggleGroup {
        items: items.into_iter().collect(),
        state,
        orientation,
        on_navigate: Rc::new(on_navigate),
        variant: ToggleVariant::Default,
        size: ToggleSize::Default,
        spacing: 2.0,
        disabled: false,
        theme: *theme,
    }
}

impl<'a, Message, Value> ToggleGroup<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Eq + 'a,
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
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn into_widget(self) -> Element<'a, Message> {
        let count = self.items.len();
        let enabled: Rc<[bool]> = self
            .items
            .iter()
            .map(|item| !self.disabled && !item.disabled)
            .collect();
        let controls = self
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let on_navigate = Rc::clone(&self.on_navigate);
                let enabled = Rc::clone(&enabled);
                let orientation = self.orientation;
                toggle(
                    item.id,
                    item.content,
                    self.state.is_selected(&item.value),
                    item.on_toggle,
                    &self.theme,
                )
                .variant(self.variant)
                .size(self.size)
                .radius(item_radius(
                    index,
                    count,
                    self.orientation,
                    self.spacing,
                    self.theme.radius.md,
                ))
                .disabled(self.disabled || item.disabled)
                .on_key_press(move |key, _modifiers| {
                    enabled_item_target(index, &enabled, &key, orientation)
                        .map(|target| on_navigate(target))
                })
                .into()
            })
            .collect::<Vec<Element<'a, Message>>>();

        match self.orientation {
            ToggleGroupOrientation::Horizontal => Row::with_children(controls)
                .spacing(layout_spacing(self.spacing, self.variant))
                .align_y(Alignment::Center)
                .width(Length::Shrink)
                .into(),
            ToggleGroupOrientation::Vertical => Column::with_children(controls)
                .spacing(layout_spacing(self.spacing, self.variant))
                .align_x(Alignment::Center)
                .width(Length::Shrink)
                .into(),
        }
    }
}

fn layout_spacing(spacing: f32, variant: ToggleVariant) -> f32 {
    if spacing == 0.0 && variant == ToggleVariant::Outline {
        -1.0
    } else {
        spacing
    }
}

fn item_radius(
    index: usize,
    count: usize,
    orientation: ToggleGroupOrientation,
    spacing: f32,
    radius: f32,
) -> border::Radius {
    if spacing > 0.0 || count <= 1 {
        return radius.into();
    }

    match orientation {
        ToggleGroupOrientation::Horizontal => border::Radius {
            top_left: if index == 0 { radius } else { 0.0 },
            top_right: if index + 1 == count { radius } else { 0.0 },
            bottom_right: if index + 1 == count { radius } else { 0.0 },
            bottom_left: if index == 0 { radius } else { 0.0 },
        },
        ToggleGroupOrientation::Vertical => border::Radius {
            top_left: if index == 0 { radius } else { 0.0 },
            top_right: if index == 0 { radius } else { 0.0 },
            bottom_right: if index + 1 == count { radius } else { 0.0 },
            bottom_left: if index + 1 == count { radius } else { 0.0 },
        },
    }
}

impl<'a, Message, Value> From<ToggleGroup<'a, Message, Value>> for Element<'a, Message>
where
    Message: Clone + 'a,
    Value: Eq + 'a,
{
    fn from(group: ToggleGroup<'a, Message, Value>) -> Self {
        group.into_widget()
    }
}

/// Returns the item index for arrow, Home, and End focus navigation.
///
/// Arrow navigation wraps. Horizontal groups use Left/Right; vertical groups
/// use Up/Down. Invalid current indices and unrelated keys return `None`.
pub fn item_target(
    current: usize,
    count: usize,
    key: &keyboard::Key,
    orientation: ToggleGroupOrientation,
) -> Option<usize> {
    enabled_item_target(current, &vec![true; count], key, orientation)
}

/// Returns the next enabled item for arrow, Home, and End navigation.
pub fn enabled_item_target(
    current: usize,
    enabled: &[bool],
    key: &keyboard::Key,
    orientation: ToggleGroupOrientation,
) -> Option<usize> {
    if current >= enabled.len() || !enabled.iter().any(|enabled| *enabled) {
        return None;
    }

    match key {
        keyboard::Key::Named(Named::Home) => enabled.iter().position(|enabled| *enabled),
        keyboard::Key::Named(Named::End) => enabled.iter().rposition(|enabled| *enabled),
        keyboard::Key::Named(Named::ArrowLeft)
            if orientation == ToggleGroupOrientation::Horizontal =>
        {
            previous_enabled(current, enabled)
        }
        keyboard::Key::Named(Named::ArrowRight)
            if orientation == ToggleGroupOrientation::Horizontal =>
        {
            next_enabled(current, enabled)
        }
        keyboard::Key::Named(Named::ArrowUp) if orientation == ToggleGroupOrientation::Vertical => {
            previous_enabled(current, enabled)
        }
        keyboard::Key::Named(Named::ArrowDown)
            if orientation == ToggleGroupOrientation::Vertical =>
        {
            next_enabled(current, enabled)
        }
        _ => None,
    }
}

fn previous_enabled(current: usize, enabled: &[bool]) -> Option<usize> {
    (1..=enabled.len())
        .map(|distance| (current + enabled.len() - distance) % enabled.len())
        .find(|index| enabled[*index])
}

fn next_enabled(current: usize, enabled: &[bool]) -> Option<usize> {
    (1..=enabled.len())
        .map(|distance| (current + distance) % enabled.len())
        .find(|index| enabled[*index])
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;
    use iced::widget::text;

    #[derive(Debug, Clone)]
    enum Message {
        ToggleBold,
        ToggleItalic,
        Focus,
    }

    #[test]
    fn single_and_multiple_reducers_are_controlled_and_stable() {
        let single = ToggleGroupState::Single(Some("bold"));
        assert_eq!(single.toggled("bold"), ToggleGroupState::Single(None));
        assert_eq!(
            single.toggled("italic"),
            ToggleGroupState::Single(Some("italic"))
        );

        let multiple = ToggleGroupState::Multiple(vec!["bold", "italic"]);
        assert_eq!(
            multiple.toggled("bold"),
            ToggleGroupState::Multiple(vec!["italic"])
        );
        assert_eq!(
            multiple.toggled("underline"),
            ToggleGroupState::Multiple(vec!["bold", "italic", "underline"])
        );
    }

    #[test]
    fn navigation_wraps_and_respects_orientation() {
        let left = keyboard::Key::Named(Named::ArrowLeft);
        let right = keyboard::Key::Named(Named::ArrowRight);
        let up = keyboard::Key::Named(Named::ArrowUp);
        let down = keyboard::Key::Named(Named::ArrowDown);
        let home = keyboard::Key::Named(Named::Home);
        let end = keyboard::Key::Named(Named::End);

        assert_eq!(
            item_target(0, 3, &left, ToggleGroupOrientation::Horizontal),
            Some(2)
        );
        assert_eq!(
            item_target(2, 3, &right, ToggleGroupOrientation::Horizontal),
            Some(0)
        );
        assert_eq!(
            item_target(0, 3, &up, ToggleGroupOrientation::Vertical),
            Some(2)
        );
        assert_eq!(
            item_target(2, 3, &down, ToggleGroupOrientation::Vertical),
            Some(0)
        );
        assert_eq!(
            item_target(2, 3, &home, ToggleGroupOrientation::Vertical),
            Some(0)
        );
        assert_eq!(
            item_target(0, 3, &end, ToggleGroupOrientation::Horizontal),
            Some(2)
        );
        assert_eq!(
            item_target(0, 3, &up, ToggleGroupOrientation::Horizontal),
            None
        );
        assert_eq!(
            item_target(0, 0, &home, ToggleGroupOrientation::Horizontal),
            None
        );
        assert_eq!(
            item_target(3, 3, &home, ToggleGroupOrientation::Horizontal),
            None
        );
        assert_eq!(
            enabled_item_target(
                0,
                &[true, false, true],
                &right,
                ToggleGroupOrientation::Horizontal,
            ),
            Some(2)
        );
        assert_eq!(
            enabled_item_target(
                2,
                &[false, true, true],
                &home,
                ToggleGroupOrientation::Horizontal,
            ),
            Some(1)
        );
    }

    #[test]
    fn group_tree_contains_one_focus_control_per_item() {
        let state = ToggleGroupState::Multiple(vec!["bold"]);
        let items = vec![
            toggle_group_item(
                widget::Id::unique(),
                "bold",
                text("Bold"),
                Message::ToggleBold,
            ),
            toggle_group_item(
                widget::Id::unique(),
                "italic",
                text("Italic"),
                Message::ToggleItalic,
            )
            .disabled(true),
        ];
        let group: Element<'_, Message> = toggle_group(
            items,
            &state,
            ToggleGroupOrientation::Horizontal,
            |_| Message::Focus,
            &LIGHT,
        )
        .into();

        let children = group.as_widget().children();
        assert_eq!(children.len(), 2);
        assert!(children.iter().all(|child| child.children.len() == 1));
    }

    #[test]
    fn default_gap_and_zero_gap_outline_joins_match_shadcn() {
        assert_eq!(layout_spacing(2.0, ToggleVariant::Outline), 2.0);
        assert_eq!(layout_spacing(0.0, ToggleVariant::Default), 0.0);
        assert_eq!(layout_spacing(0.0, ToggleVariant::Outline), -1.0);

        let first = item_radius(0, 3, ToggleGroupOrientation::Horizontal, 0.0, 9.0);
        let middle = item_radius(1, 3, ToggleGroupOrientation::Horizontal, 0.0, 9.0);
        let last = item_radius(2, 3, ToggleGroupOrientation::Horizontal, 0.0, 9.0);
        assert_eq!((first.top_left, first.top_right), (9.0, 0.0));
        assert_eq!(middle, border::Radius::default());
        assert_eq!((last.top_left, last.top_right), (0.0, 9.0));

        let separated = item_radius(1, 3, ToggleGroupOrientation::Vertical, 2.0, 9.0);
        assert_eq!(separated, 9.0.into());
    }
}
