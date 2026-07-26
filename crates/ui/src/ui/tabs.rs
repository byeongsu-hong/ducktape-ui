//! Controlled tabs with stable trigger focus IDs.
//!
//! Iced does not expose tab/list/tab-panel accessibility roles. This component
//! therefore implements the interaction contract without claiming semantic
//! roles the toolkit cannot publish: pointer, touch, Enter, Space, arrow, Home,
//! and End all work through [`super::focus_control::FocusControl`].

use super::focus_control::{self, FocusControl, Status};
use super::theme::{Theme, alpha, mix};
use iced::keyboard::{self, key::Named};
use iced::widget::rule::{FillMode, Style as RuleStyle};
use iced::widget::{Column, Row, Space, container, rule};
use iced::{Background, Border, Element, Length, Shadow, Task};
use std::rc::Rc;

/// The axis used by the tab list and its arrow-key navigation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TabsOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Whether moving focus also selects the focused tab.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TabsActivation {
    #[default]
    Automatic,
    Manual,
}

/// The filled default treatment or its selected-line alternative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TabsVariant {
    #[default]
    Default,
    Line,
}

/// Selection owned by the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabsState<Id> {
    selected: Option<Id>,
    focused: Option<Id>,
}

impl<Id> Default for TabsState<Id> {
    fn default() -> Self {
        Self {
            selected: None,
            focused: None,
        }
    }
}

impl<Id> TabsState<Id> {
    pub fn new(selected: Id) -> Self
    where
        Id: Clone,
    {
        Self {
            selected: Some(selected.clone()),
            focused: Some(selected),
        }
    }

    pub fn selected(&self) -> Option<&Id> {
        self.selected.as_ref()
    }

    pub fn clear(&mut self) {
        self.selected = None;
        self.focused = None;
    }
}

impl<Id: Clone + Eq> TabsState<Id> {
    pub fn select(&mut self, selected: Id) {
        self.selected = Some(selected.clone());
        self.focused = Some(selected);
    }

    /// Applies the selection and roving-focus state carried by an event.
    pub fn apply(&mut self, event: &TabsEvent<Id>) -> bool {
        let previous = self.clone();

        match event {
            TabsEvent::Select(id) => self.select(id.clone()),
            TabsEvent::Navigate { target, select, .. } => {
                self.focused = Some(target.clone());
                if *select {
                    self.selected = Some(target.clone());
                }
            }
        }

        *self != previous
    }
}

/// An activation or keyboard-navigation request emitted by [`tabs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabsEvent<Id> {
    /// Pointer, touch, Enter, or Space selected a trigger already holding focus.
    Select(Id),
    /// Keyboard navigation requested focus on another trigger.
    Navigate {
        target: Id,
        focus_id: iced::widget::Id,
        select: bool,
    },
}

impl<Id> TabsEvent<Id> {
    /// The tab that should become selected, if this event changes selection.
    pub fn selection(&self) -> Option<&Id> {
        match self {
            Self::Select(id)
            | Self::Navigate {
                target: id,
                select: true,
                ..
            } => Some(id),
            Self::Navigate { select: false, .. } => None,
        }
    }

    /// The stable trigger ID keyboard navigation should focus.
    pub fn focus_id(&self) -> Option<&iced::widget::Id> {
        match self {
            Self::Select(_) => None,
            Self::Navigate { focus_id, .. } => Some(focus_id),
        }
    }

    /// Produces the iced task that completes a keyboard focus move.
    pub fn focus_task<Message>(&self) -> Task<Message> {
        self.focus_id()
            .map_or_else(Task::none, |id| iced::widget::operation::focus(id.clone()))
    }
}

/// Caller-owned trigger and panel content for one tab.
pub struct Tab<'a, Message, Id> {
    id: Id,
    focus_id: iced::widget::Id,
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    disabled: bool,
}

/// Creates a tab with a stable focus ID.
///
/// Keep `focus_id` stable between views: store it in application state or use a
/// fixed [`iced::widget::Id::new`] value.
pub fn tab<'a, Message, Id>(
    id: Id,
    focus_id: iced::widget::Id,
    trigger: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
) -> Tab<'a, Message, Id>
where
    Message: 'a,
{
    Tab {
        id,
        focus_id,
        trigger: trigger.into(),
        content: content.into(),
        disabled: false,
    }
}

impl<Message, Id> Tab<'_, Message, Id> {
    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn focus_id(&self) -> &iced::widget::Id {
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

/// Composes a tab list and the selected panel.
///
/// Feed each emitted event to [`TabsState::apply`] and return
/// [`TabsEvent::focus_task`] from `update`. Only the selected panel is inserted
/// into the widget tree.
pub fn tabs<'a, Message, Id>(
    state: &TabsState<Id>,
    items: impl IntoIterator<Item = Tab<'a, Message, Id>>,
    orientation: TabsOrientation,
    activation: TabsActivation,
    variant: TabsVariant,
    on_event: impl Fn(TabsEvent<Id>) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
    Id: Clone + Eq + 'a,
{
    struct Target<Id> {
        id: Id,
        focus_id: iced::widget::Id,
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
    let selected_id = state.selected();
    let focused_id = state.focused.as_ref().or(selected_id);
    let tab_stop = items
        .iter()
        .position(|item| !item.disabled && focused_id == Some(&item.id))
        .or_else(|| enabled.iter().position(|enabled| *enabled));
    let mut selected_panel = None;
    let mut trigger_elements = Vec::with_capacity(items.len());

    for (index, item) in items.into_iter().enumerate() {
        let selected = selected_id == Some(&item.id);
        if selected && selected_panel.is_none() {
            selected_panel = Some(item.content);
        }

        let on_navigation = Rc::clone(&on_event);
        let enabled = Rc::clone(&enabled);
        let targets = Rc::clone(&targets);
        let trigger_theme = *theme;
        let label = container(item.trigger).padding([0, 12]).width(
            if orientation == TabsOrientation::Vertical {
                Length::Fill
            } else {
                Length::Shrink
            },
        );
        let content: Element<'a, Message> = match (variant, orientation) {
            (TabsVariant::Default, _) => label.center_y(36).into(),
            (TabsVariant::Line, TabsOrientation::Horizontal) => Column::new()
                .push(label.center_y(34))
                .push(indicator(theme, selected, orientation))
                .into(),
            (TabsVariant::Line, TabsOrientation::Vertical) => Row::new()
                .push(label.center_y(36))
                .push(indicator(theme, selected, orientation))
                .height(36)
                .into(),
        };
        let control: Element<'a, Message> = FocusControl::new(
            item.focus_id,
            content,
            on_event(TabsEvent::Select(item.id)),
            theme,
        )
        .disabled(item.disabled)
        .tab_stop(tab_stop == Some(index))
        .on_key_press(move |key, _modifiers| {
            let target = navigation_target(index, &enabled, &key, orientation)?;
            let target = &targets[target];

            Some(on_navigation(TabsEvent::Navigate {
                target: target.id.clone(),
                focus_id: target.focus_id.clone(),
                select: activation == TabsActivation::Automatic,
            }))
        })
        .style(move |_iced_theme, status| trigger_style(&trigger_theme, selected, variant, status))
        .into();

        trigger_elements.push(control);
    }

    let list: Element<'a, Message> = match orientation {
        TabsOrientation::Horizontal => trigger_elements
            .into_iter()
            .fold(Row::new().spacing(theme.spacing.xs), Row::push)
            .into(),
        TabsOrientation::Vertical => trigger_elements
            .into_iter()
            .fold(Column::new().spacing(theme.spacing.xs), Column::push)
            .width(Length::Fill)
            .into(),
    };
    let theme = *theme;
    let list = container(list)
        .padding(theme.spacing.xs)
        .style(move |_iced_theme| iced::widget::container::Style {
            background: (variant == TabsVariant::Default)
                .then_some(Background::Color(theme.palette.muted)),
            border: Border {
                radius: if variant == TabsVariant::Default {
                    theme.radius.lg.into()
                } else {
                    0.0.into()
                },
                ..Border::default()
            },
            ..iced::widget::container::Style::default()
        });
    let panel =
        container(selected_panel.unwrap_or_else(|| Space::new().into())).width(Length::Fill);

    match orientation {
        TabsOrientation::Horizontal => Column::new()
            .push(list)
            .push(panel)
            .spacing(theme.spacing.sm)
            .width(Length::Fill)
            .into(),
        TabsOrientation::Vertical => Row::new()
            .push(list.width(Length::Shrink))
            .push(panel)
            .spacing(theme.spacing.md)
            .width(Length::Fill)
            .into(),
    }
}

/// Returns the enabled trigger targeted by an arrow, Home, or End key.
///
/// Directional navigation wraps and skips disabled tabs. An arrow on an axis
/// that does not match `orientation` is ignored.
pub fn navigation_target(
    current: usize,
    enabled: &[bool],
    key: &keyboard::Key,
    orientation: TabsOrientation,
) -> Option<usize> {
    if current >= enabled.len() || !enabled.iter().any(|enabled| *enabled) {
        return None;
    }

    match key {
        keyboard::Key::Named(Named::Home) => enabled.iter().position(|enabled| *enabled),
        keyboard::Key::Named(Named::End) => enabled.iter().rposition(|enabled| *enabled),
        keyboard::Key::Named(Named::ArrowLeft) if orientation == TabsOrientation::Horizontal => {
            previous_enabled(current, enabled)
        }
        keyboard::Key::Named(Named::ArrowRight) if orientation == TabsOrientation::Horizontal => {
            next_enabled(current, enabled)
        }
        keyboard::Key::Named(Named::ArrowUp) if orientation == TabsOrientation::Vertical => {
            previous_enabled(current, enabled)
        }
        keyboard::Key::Named(Named::ArrowDown) if orientation == TabsOrientation::Vertical => {
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

fn indicator<'a, Message>(
    theme: &Theme,
    selected: bool,
    orientation: TabsOrientation,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let color = if selected {
        theme.palette.ring
    } else {
        iced::Color::TRANSPARENT
    };
    let style = move |_iced_theme: &iced::Theme| RuleStyle {
        color,
        radius: 1.0.into(),
        fill_mode: FillMode::Full,
        snap: true,
    };

    match orientation {
        TabsOrientation::Horizontal => rule::horizontal(2).style(style).into(),
        TabsOrientation::Vertical => rule::vertical(2).style(style).into(),
    }
}

fn trigger_style(
    theme: &Theme,
    selected: bool,
    variant: TabsVariant,
    status: Status,
) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match (variant, selected, status) {
        (TabsVariant::Default, true, Status::Disabled) => {
            Some(Background::Color(alpha(theme.palette.background, 0.7)))
        }
        (TabsVariant::Default, true, _) => Some(Background::Color(theme.palette.background)),
        (_, false, Status::Hovered | Status::Pressed) => {
            Some(Background::Color(theme.palette.accent))
        }
        _ => None,
    };
    style.text_color = Some(if status == Status::Disabled {
        mix(
            theme.palette.background,
            theme.palette.muted_foreground,
            0.8,
        )
    } else if selected {
        theme.palette.foreground
    } else {
        theme.palette.muted_foreground
    });
    style.border = Border {
        color: if variant == TabsVariant::Default && selected {
            theme.palette.border
        } else {
            iced::Color::TRANSPARENT
        },
        width: if variant == TabsVariant::Default && selected {
            1.0
        } else {
            0.0
        },
        radius: theme.radius.md.into(),
    };
    style.shadow = if variant == TabsVariant::Default && selected {
        Shadow {
            color: alpha(theme.palette.foreground, 0.08),
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 2.0,
        }
    } else {
        Shadow::default()
    };
    style
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::focusable_count;
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::widget::{Column, text};

    fn key(named: Named) -> keyboard::Key {
        keyboard::Key::Named(named)
    }

    #[test]
    fn state_applies_selection_but_manual_navigation_only_moves_focus() {
        struct NoDefault;
        assert!(TabsState::<NoDefault>::default().selected().is_none());

        let mut state = TabsState::new("account");
        let focus_id = iced::widget::Id::new("tabs-password");
        let manual = TabsEvent::Navigate {
            target: "password",
            focus_id: focus_id.clone(),
            select: false,
        };

        assert!(state.apply(&manual));
        assert_eq!(state.selected(), Some(&"account"));
        assert_eq!(state.focused, Some("password"));
        assert_eq!(manual.focus_id(), Some(&focus_id));

        let automatic = TabsEvent::Navigate {
            target: "password",
            focus_id,
            select: true,
        };
        assert!(state.apply(&automatic));
        assert_eq!(state.selected(), Some(&"password"));
        assert!(!state.apply(&automatic));

        state.clear();
        assert_eq!(state.selected(), None);
        assert_eq!(state.focused, None);
        assert!(state.apply(&TabsEvent::Select("account")));
    }

    #[test]
    fn navigation_wraps_and_skips_disabled_tabs_on_each_axis() {
        let enabled = [true, false, true, false];

        assert_eq!(
            navigation_target(
                0,
                &enabled,
                &key(Named::ArrowRight),
                TabsOrientation::Horizontal,
            ),
            Some(2)
        );
        assert_eq!(
            navigation_target(
                2,
                &enabled,
                &key(Named::ArrowRight),
                TabsOrientation::Horizontal,
            ),
            Some(0)
        );
        assert_eq!(
            navigation_target(0, &enabled, &key(Named::ArrowUp), TabsOrientation::Vertical,),
            Some(2)
        );
        assert_eq!(
            navigation_target(
                2,
                &enabled,
                &key(Named::ArrowDown),
                TabsOrientation::Vertical,
            ),
            Some(0)
        );
        assert_eq!(
            navigation_target(2, &enabled, &key(Named::Home), TabsOrientation::Vertical,),
            Some(0)
        );
        assert_eq!(
            navigation_target(0, &enabled, &key(Named::End), TabsOrientation::Horizontal,),
            Some(2)
        );
    }

    #[test]
    fn navigation_ignores_wrong_axes_and_invalid_lists() {
        assert_eq!(
            navigation_target(
                0,
                &[true, true],
                &key(Named::ArrowDown),
                TabsOrientation::Horizontal,
            ),
            None
        );
        assert_eq!(
            navigation_target(
                0,
                &[false, false],
                &key(Named::Home),
                TabsOrientation::Horizontal,
            ),
            None
        );
        assert_eq!(
            navigation_target(
                2,
                &[true, true],
                &key(Named::End),
                TabsOrientation::Horizontal,
            ),
            None
        );
    }

    #[test]
    fn tab_exposes_stable_id_and_disabled_state() {
        let focus_id = iced::widget::Id::new("tabs-account");
        let item: Tab<'_, (), _> = tab(
            "account",
            focus_id.clone(),
            text("Account"),
            text("Account panel"),
        )
        .disabled(true);

        assert_eq!(item.id(), &"account");
        assert_eq!(item.focus_id(), &focus_id);
        assert!(item.is_disabled());
    }

    #[test]
    fn only_selected_panel_enters_the_widget_tree() {
        let state = TabsState::new("password");
        let items = vec![
            tab(
                "account",
                iced::widget::Id::new("tabs-account"),
                text("Account"),
                Column::new()
                    .push(text("hidden one"))
                    .push(text("hidden two"))
                    .push(text("hidden three")),
            ),
            tab(
                "password",
                iced::widget::Id::new("tabs-password"),
                text("Password"),
                Column::new()
                    .push(text("visible one"))
                    .push(text("visible two")),
            ),
        ];
        let tabs: Element<'_, TabsEvent<&str>> = tabs(
            &state,
            items,
            TabsOrientation::Horizontal,
            TabsActivation::Automatic,
            TabsVariant::Default,
            |event| event,
            &LIGHT,
        );
        let root = tabs.as_widget().children();

        assert_eq!(root.len(), 2);
        assert_eq!(root[0].children.len(), 2);
        assert_eq!(root[1].children.len(), 2);
    }

    #[test]
    fn tabs_expose_one_sequential_focus_stop_in_both_activation_modes() {
        for activation in [TabsActivation::Automatic, TabsActivation::Manual] {
            let state = TabsState::new("password");
            let element = tabs(
                &state,
                [
                    tab(
                        "account",
                        iced::widget::Id::new("focus-account"),
                        text("Account"),
                        text("Account panel"),
                    ),
                    tab(
                        "password",
                        iced::widget::Id::new("focus-password"),
                        text("Password"),
                        text("Password panel"),
                    ),
                    tab(
                        "disabled",
                        iced::widget::Id::new("focus-disabled"),
                        text("Disabled"),
                        text("Disabled panel"),
                    )
                    .disabled(true),
                ],
                TabsOrientation::Horizontal,
                activation,
                TabsVariant::Default,
                |event| event,
                &LIGHT,
            );

            assert_eq!(focusable_count(element), 1);
        }
    }

    #[test]
    fn variants_keep_selected_and_disabled_details_distinct_in_both_themes() {
        fn luminance(color: iced::Color) -> f32 {
            fn channel(value: f32) -> f32 {
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            }

            0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
        }

        fn contrast(left: iced::Color, right: iced::Color) -> f32 {
            let (light, dark) = if luminance(left) > luminance(right) {
                (luminance(left), luminance(right))
            } else {
                (luminance(right), luminance(left))
            };
            (light + 0.05) / (dark + 0.05)
        }

        for theme in [LIGHT, DARK] {
            let selected = trigger_style(&theme, true, TabsVariant::Default, Status::Active);
            assert!(selected.background.is_some());
            assert_eq!(selected.border.width, 1.0);
            assert!(selected.shadow.color.a > 0.0);

            let line = trigger_style(&theme, true, TabsVariant::Line, Status::Active);
            assert!(line.background.is_none());
            assert_eq!(line.border.width, 0.0);
            assert_eq!(line.shadow, Shadow::default());

            let disabled = trigger_style(&theme, false, TabsVariant::Default, Status::Disabled);
            assert!(
                contrast(disabled.text_color.unwrap(), theme.palette.muted) >= 3.0,
                "disabled tab text should stay legible in {}",
                theme.name
            );
        }
    }
}
