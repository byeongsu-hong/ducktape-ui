//! Controlled accordion with stable, keyboard-complete disclosure headers.
//!
//! Iced does not expose accordion accessibility roles. This component owns the
//! interaction contract without claiming roles the runtime cannot publish:
//! pointer, touch, Enter, Space, ArrowUp/Down, Home, and End all work through
//! [`super::focus_control::FocusControl`].

use std::rc::Rc;

use super::focus_control::{FocusControl, Status, Style as FocusStyle};
use super::theme::{Theme, alpha, mix};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::widget::rule::{FillMode, Style as RuleStyle};
use iced::widget::{Column, Row, container, rule, text};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Task, widget};

/// Controlled accordion state matching shadcn's single and multiple modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccordionState<Id> {
    Single(Option<Id>),
    Multiple(Vec<Id>),
}

impl<Id: Eq> AccordionState<Id> {
    pub fn is_open(&self, id: &Id) -> bool {
        match self {
            Self::Single(open) => open.as_ref() == Some(id),
            Self::Multiple(open) => open.contains(id),
        }
    }
}

impl<Id: Clone + Eq> AccordionState<Id> {
    /// Returns a new controlled state after activating one header.
    #[must_use]
    pub fn toggled(&self, id: Id) -> Self {
        match self {
            Self::Single(open) if open.as_ref() == Some(&id) => Self::Single(None),
            Self::Single(_) => Self::Single(Some(id)),
            Self::Multiple(open) => {
                let mut next = open.clone();
                if self.is_open(&id) {
                    next.retain(|open_id| open_id != &id);
                } else {
                    next.push(id);
                }
                Self::Multiple(next)
            }
        }
    }

    /// Applies a disclosure event and reports whether open state changed.
    pub fn apply(&mut self, event: &AccordionEvent<Id>) -> bool {
        let AccordionEvent::Toggle(id) = event else {
            return false;
        };
        let next = self.toggled(id.clone());
        if *self == next {
            false
        } else {
            *self = next;
            true
        }
    }
}

/// A header activation or focus-navigation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccordionEvent<Id> {
    Toggle(Id),
    Navigate { target: Id, focus_id: widget::Id },
}

impl<Id> AccordionEvent<Id> {
    pub fn focus_id(&self) -> Option<&widget::Id> {
        match self {
            Self::Toggle(_) => None,
            Self::Navigate { focus_id, .. } => Some(focus_id),
        }
    }

    pub fn focus_task<Message>(&self) -> Task<Message> {
        self.focus_id()
            .map_or_else(Task::none, |id| iced::widget::operation::focus(id.clone()))
    }
}

/// A caller-owned accordion label and its controlled content.
pub struct AccordionItem<'a, Message, Id> {
    id: Id,
    focus_id: widget::Id,
    header: Element<'a, Message>,
    content: Element<'a, Message>,
    disabled: bool,
}

/// Creates an accordion item with a stable header focus ID.
pub fn accordion_item<'a, Message, Id>(
    id: Id,
    focus_id: widget::Id,
    header: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
) -> AccordionItem<'a, Message, Id>
where
    Message: 'a,
{
    AccordionItem {
        id,
        focus_id,
        header: header.into(),
        content: content.into(),
        disabled: false,
    }
}

impl<Message, Id> AccordionItem<'_, Message, Id> {
    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn focus_id(&self) -> &widget::Id {
        &self.focus_id
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Composes aligned headers and only the currently open content.
///
/// Feed emitted events to [`AccordionState::apply`] and return
/// [`AccordionEvent::focus_task`] from `update`.
pub fn accordion<'a, Message, Id>(
    items: impl IntoIterator<Item = AccordionItem<'a, Message, Id>>,
    state: &AccordionState<Id>,
    on_event: impl Fn(AccordionEvent<Id>) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
    Id: Clone + Eq + 'a,
{
    struct Target<Id> {
        id: Id,
        focus_id: widget::Id,
    }

    let items: Vec<_> = items.into_iter().collect();
    let enabled: Rc<[bool]> = items.iter().map(|item| !item.disabled).collect();
    let targets: Rc<[Target<Id>]> = items
        .iter()
        .map(|item| Target {
            id: item.id.clone(),
            focus_id: item.focus_id.clone(),
        })
        .collect();
    let on_event = Rc::new(on_event);
    let mut result = Column::new().width(Length::Fill);

    for (index, item) in items.into_iter().enumerate() {
        let open = state.is_open(&item.id);
        let header = Row::new()
            .push(
                container(item.header)
                    .width(Length::Fill)
                    .align_x(Horizontal::Left)
                    .align_y(Vertical::Center),
            )
            .push(
                container(text(if open { "−" } else { "+" }).size(theme.typography.base))
                    .width(20)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .padding(
                Padding::default()
                    .vertical(theme.spacing.md)
                    .horizontal(theme.spacing.sm),
            )
            .width(Length::Fill)
            .align_y(iced::Alignment::Center);
        let key_enabled = Rc::clone(&enabled);
        let key_targets = Rc::clone(&targets);
        let key_event = Rc::clone(&on_event);
        let trigger_theme = *theme;
        let trigger: Element<'a, Message> = FocusControl::new(
            item.focus_id,
            header,
            on_event(AccordionEvent::Toggle(item.id.clone())),
            theme,
        )
        .disabled(item.disabled)
        .on_key_press(move |key, _modifiers| {
            let target = navigation_target(index, &key_enabled, &key)?;
            let target = &key_targets[target];
            Some(key_event(AccordionEvent::Navigate {
                target: target.id.clone(),
                focus_id: target.focus_id.clone(),
            }))
        })
        .style(move |_iced_theme, status| trigger_style(&trigger_theme, status))
        .into();
        let mut section = Column::new().width(Length::Fill).push(trigger);
        if open {
            section = section.push(
                container(item.content)
                    .padding(
                        Padding::default()
                            .horizontal(theme.spacing.sm)
                            .bottom(theme.spacing.lg),
                    )
                    .width(Length::Fill),
            );
        }
        result = result.push(section).push(divider(theme));
    }

    result.into()
}

fn divider<'a, Message>(theme: &Theme) -> Element<'a, Message>
where
    Message: 'a,
{
    let color = theme.palette.border;
    rule::horizontal::<'a, iced::Theme>(1)
        .style(move |_| RuleStyle {
            color,
            radius: 0.0.into(),
            fill_mode: FillMode::Full,
            snap: true,
        })
        .into()
}

pub fn trigger_style(theme: &Theme, status: Status) -> FocusStyle {
    let disabled = status == Status::Disabled;
    let background = match status {
        Status::Hovered => Some(Background::Color(alpha(theme.palette.accent, 0.55))),
        Status::Pressed => Some(Background::Color(mix(
            theme.palette.accent,
            theme.palette.foreground,
            0.08,
        ))),
        Status::Active | Status::Focused | Status::Disabled => None,
    };

    FocusStyle {
        background,
        text_color: Some(if disabled {
            alpha(theme.palette.foreground, 0.5)
        } else {
            theme.palette.foreground
        }),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: theme.radius.md.into(),
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: 2.0,
            radius: (theme.radius.md + 2.0).into(),
        },
        focus_offset: 1.0,
    }
}

/// Compatibility key model for callers with an external reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccordionKey {
    ArrowUp,
    ArrowDown,
    Home,
    End,
}

/// Returns a wrapped header index without disabled-item knowledge.
pub fn header_target(current: usize, count: usize, key: AccordionKey) -> Option<usize> {
    if count == 0 || current >= count {
        return None;
    }

    Some(match key {
        AccordionKey::ArrowUp => current.checked_sub(1).unwrap_or(count - 1),
        AccordionKey::ArrowDown => (current + 1) % count,
        AccordionKey::Home => 0,
        AccordionKey::End => count - 1,
    })
}

/// Finds the enabled header targeted by ArrowUp/Down, Home, or End.
pub fn navigation_target(current: usize, enabled: &[bool], key: &keyboard::Key) -> Option<usize> {
    if current >= enabled.len() || !enabled.iter().any(|enabled| *enabled) {
        return None;
    }

    match key {
        keyboard::Key::Named(Named::Home) => enabled.iter().position(|enabled| *enabled),
        keyboard::Key::Named(Named::End) => enabled.iter().rposition(|enabled| *enabled),
        keyboard::Key::Named(Named::ArrowUp) => (1..=enabled.len())
            .map(|distance| (current + enabled.len() - distance) % enabled.len())
            .find(|index| enabled[*index]),
        keyboard::Key::Named(Named::ArrowDown) => (1..=enabled.len())
            .map(|distance| (current + distance) % enabled.len())
            .find(|index| enabled[*index]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};
    use iced::advanced::widget::Tree;

    #[test]
    fn single_and_multiple_modes_toggle_and_apply_independently() {
        let mut single = AccordionState::Single(Some("one"));
        assert!(single.apply(&AccordionEvent::Toggle("one")));
        assert_eq!(single, AccordionState::Single(None));
        assert!(!single.apply(&AccordionEvent::Navigate {
            target: "two",
            focus_id: widget::Id::new("two"),
        }));

        let multiple = AccordionState::Multiple(vec!["one", "two"]);
        assert_eq!(
            multiple.toggled("one"),
            AccordionState::Multiple(vec!["two"])
        );
        assert_eq!(
            multiple.toggled("three"),
            AccordionState::Multiple(vec!["one", "two", "three"])
        );
    }

    #[test]
    fn navigation_wraps_and_skips_disabled_headers() {
        let enabled = [true, false, true, false];
        assert_eq!(
            navigation_target(0, &enabled, &keyboard::Key::Named(Named::ArrowUp)),
            Some(2)
        );
        assert_eq!(
            navigation_target(0, &enabled, &keyboard::Key::Named(Named::ArrowDown)),
            Some(2)
        );
        assert_eq!(
            navigation_target(2, &enabled, &keyboard::Key::Named(Named::Home)),
            Some(0)
        );
        assert_eq!(
            navigation_target(0, &enabled, &keyboard::Key::Named(Named::End)),
            Some(2)
        );
        assert_eq!(
            navigation_target(0, &[false, false], &keyboard::Key::Named(Named::Home)),
            None
        );
    }

    #[test]
    fn header_navigation_compatibility_reducer_keeps_edges() {
        assert_eq!(header_target(0, 3, AccordionKey::ArrowUp), Some(2));
        assert_eq!(header_target(2, 3, AccordionKey::ArrowDown), Some(0));
        assert_eq!(header_target(2, 3, AccordionKey::Home), Some(0));
        assert_eq!(header_target(0, 3, AccordionKey::End), Some(2));
        assert_eq!(header_target(0, 0, AccordionKey::Home), None);
    }

    #[test]
    fn item_ids_are_stable_and_disabled_state_is_explicit() {
        let id = widget::Id::new("header");
        let item: AccordionItem<'_, (), _> =
            accordion_item("one", id.clone(), text("One"), text("Body")).disabled(true);
        assert_eq!(item.id(), &"one");
        assert_eq!(item.focus_id(), &id);
        assert!(item.is_disabled());
    }

    #[test]
    fn widget_tree_contains_one_focus_control_per_header() {
        let view: Element<'_, ()> = accordion(
            [
                accordion_item("one", widget::Id::new("one"), text("One"), text("Body")),
                accordion_item("two", widget::Id::new("two"), text("Two"), text("Body")),
            ],
            &AccordionState::Single(Some("one")),
            |_| (),
            &LIGHT,
        );
        let tree = Tree::new(&view);
        assert_eq!(tree.children.len(), 4);
    }

    #[test]
    fn focus_and_disabled_styles_remain_semantic_in_both_themes() {
        for theme in [LIGHT, DARK] {
            let focused = trigger_style(&theme, Status::Focused);
            let disabled = trigger_style(&theme, Status::Disabled);
            assert_eq!(focused.focus_ring.color, theme.palette.ring);
            assert_eq!(focused.focus_ring.width, 2.0);
            assert!(disabled.text_color.unwrap().a < 1.0);
        }
    }
}
