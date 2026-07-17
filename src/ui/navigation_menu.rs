//! Controlled navigation links and disclosure content in one shared viewport.
//!
//! Iced does not expose DOM navigation/menu roles or browser links. This
//! component therefore emits route IDs and implements the expected focus,
//! pointer, keyboard, and collision behavior without claiming semantics the
//! runtime cannot publish. The application remains responsible for changing
//! routes and for returning [`NavigationMenuEvent::focus_task`] from `update`.

use std::rc::Rc;

use super::direction::Direction;
use super::focus_control::{self, FocusControl, Status};
use super::menu::MENU_PANEL_PADDING;
use super::popover::{
    Alignment, FloatingConfig, FloatingContent, FocusFlag, PanelKind, Placement, focus_within,
    panel, panel_style,
};
use super::theme::{Theme, alpha, mix};
use super::tooltip::event_time;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::time::{Duration, Instant};
use iced::widget::text::LineHeight;
use iced::widget::{Column, Row, Space, container, text};
use iced::{
    Alignment as IcedAlignment, Background, Border, Element, Event, Length, Padding, Pixels,
    Rectangle, Size, Task, Vector, touch,
};

pub const NAVIGATION_MENU_TRIGGER_HEIGHT: f32 = 36.0;
pub const NAVIGATION_MENU_CONTENT_MIN_WIDTH: f32 = 240.0;
pub const NAVIGATION_MENU_CONTENT_WIDTH: f32 = 480.0;
pub const NAVIGATION_MENU_CONTENT_MAX_WIDTH: f32 = 600.0;
pub const NAVIGATION_MENU_INDICATOR_WIDTH: f32 = 12.0;
pub const NAVIGATION_MENU_INDICATOR_HEIGHT: f32 = 2.0;
pub const NAVIGATION_MENU_VIEWPORT_GAP: f32 = 4.0;
pub const NAVIGATION_MENU_VIEWPORT_PADDING: f32 = 8.0;
pub const NAVIGATION_MENU_HOVER_DELAY: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NavigationMenuOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Caller-owned navigation, disclosure, and active-route state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NavigationMenuState {
    pub focused: Option<usize>,
    pub open: Option<usize>,
    pub active: Option<String>,
}

impl NavigationMenuState {
    pub fn initial(items: &[NavigationMenuItemInfo]) -> Self {
        Self {
            focused: items.iter().position(NavigationMenuItemInfo::enabled),
            open: None,
            active: None,
        }
    }

    #[must_use]
    pub fn active(mut self, route: impl Into<String>) -> Self {
        self.active = Some(route.into());
        self
    }
}

/// Cloneable metadata used by the reducer and responsive/mobile views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationMenuItemInfo {
    pub id: String,
    pub disabled: bool,
    pub disclosure: bool,
}

impl NavigationMenuItemInfo {
    pub fn link(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
            disclosure: false,
        }
    }

    pub fn disclosure(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
            disclosure: true,
        }
    }

    pub const fn enabled(&self) -> bool {
        !self.disabled
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Caller-owned content for one top-level route or disclosure trigger.
pub struct NavigationMenuItem<'a, Message> {
    id: String,
    label: String,
    content: Option<Element<'a, Message>>,
    disabled: bool,
}

impl<'a, Message> NavigationMenuItem<'a, Message> {
    pub fn link(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            content: None,
            disabled: false,
        }
    }

    pub fn disclosure(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            content: Some(content.into()),
            disabled: false,
        }
    }

    pub fn info(&self) -> NavigationMenuItemInfo {
        NavigationMenuItemInfo {
            id: self.id.clone(),
            disabled: self.disabled,
            disclosure: self.content.is_some(),
        }
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationMenuCommand {
    Previous,
    Next,
    First,
    Last,
    Activate,
    OpenContent,
    CloseContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationMenuFocus {
    Trigger(usize),
    Content(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationMenuEvent {
    StateChanged {
        state: NavigationMenuState,
        focus: Option<NavigationMenuFocus>,
    },
    LinkActivated {
        id: String,
        state: NavigationMenuState,
    },
}

impl NavigationMenuEvent {
    pub const fn state(&self) -> &NavigationMenuState {
        match self {
            Self::StateChanged { state, .. } | Self::LinkActivated { state, .. } => state,
        }
    }

    pub const fn focus(&self) -> Option<NavigationMenuFocus> {
        match self {
            Self::StateChanged { focus, .. } => *focus,
            Self::LinkActivated { .. } => None,
        }
    }

    pub fn focus_task<Message>(&self, id: &str) -> Task<Message> {
        match self.focus() {
            Some(NavigationMenuFocus::Trigger(index)) => {
                iced::widget::operation::focus(navigation_menu_trigger_id(id, index))
            }
            Some(NavigationMenuFocus::Content(index)) => {
                iced::widget::operation::focus(navigation_menu_content_id(id, index))
            }
            None => Task::none(),
        }
    }
}

/// Applies one semantic command, skipping disabled entries and preserving an
/// active disclosure while roving onto another disclosure.
pub fn reduce_navigation_menu(
    state: &NavigationMenuState,
    items: &[NavigationMenuItemInfo],
    command: NavigationMenuCommand,
) -> NavigationMenuEvent {
    let mut next = state.clone();
    let enabled = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.enabled())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    if enabled.is_empty() {
        next.focused = None;
        next.open = None;
        return changed(next, None);
    }

    let current = state
        .focused
        .filter(|index| enabled.contains(index))
        .unwrap_or(enabled[0]);

    match command {
        NavigationMenuCommand::Previous
        | NavigationMenuCommand::Next
        | NavigationMenuCommand::First
        | NavigationMenuCommand::Last => {
            let position = enabled
                .iter()
                .position(|index| *index == current)
                .unwrap_or(0);
            let target = match command {
                NavigationMenuCommand::Previous => {
                    enabled[(position + enabled.len() - 1) % enabled.len()]
                }
                NavigationMenuCommand::Next => enabled[(position + 1) % enabled.len()],
                NavigationMenuCommand::First => enabled[0],
                NavigationMenuCommand::Last => enabled[enabled.len() - 1],
                _ => unreachable!("navigation branch only receives movement commands"),
            };
            next.focused = Some(target);
            if state.open.is_some() {
                next.open = items[target].disclosure.then_some(target);
            }
            changed(next, Some(NavigationMenuFocus::Trigger(target)))
        }
        NavigationMenuCommand::Activate => activate_item(state, items, current),
        NavigationMenuCommand::OpenContent => {
            next.focused = Some(current);
            let focus = if items[current].disclosure {
                next.open = Some(current);
                Some(NavigationMenuFocus::Content(current))
            } else {
                None
            };
            changed(next, focus)
        }
        NavigationMenuCommand::CloseContent => {
            let restore = state
                .open
                .filter(|index| enabled.contains(index) && items[*index].disclosure)
                .unwrap_or(current);
            next.focused = Some(restore);
            next.open = None;
            changed(next, Some(NavigationMenuFocus::Trigger(restore)))
        }
    }
}

fn activate_item(
    state: &NavigationMenuState,
    items: &[NavigationMenuItemInfo],
    index: usize,
) -> NavigationMenuEvent {
    let mut next = state.clone();
    next.focused = Some(index);
    let item = &items[index];
    if item.disclosure {
        next.open = (state.open != Some(index)).then_some(index);
        changed(next, None)
    } else {
        next.open = None;
        next.active = Some(item.id.clone());
        NavigationMenuEvent::LinkActivated {
            id: item.id.clone(),
            state: next,
        }
    }
}

fn activate_at(
    state: &NavigationMenuState,
    items: &[NavigationMenuItemInfo],
    index: usize,
) -> NavigationMenuEvent {
    let mut state = state.clone();
    state.focused = Some(index);
    reduce_navigation_menu(&state, items, NavigationMenuCommand::Activate)
}

fn changed(state: NavigationMenuState, focus: Option<NavigationMenuFocus>) -> NavigationMenuEvent {
    NavigationMenuEvent::StateChanged { state, focus }
}

/// Maps physical keys to semantic movement for expanded and collapsed/mobile
/// layouts. `collapsed` uses a vertical list and reading-direction lateral key.
pub fn navigation_menu_command(
    key: &keyboard::Key,
    orientation: NavigationMenuOrientation,
    direction: Direction,
    content_open: bool,
) -> Option<NavigationMenuCommand> {
    match (orientation, key) {
        (NavigationMenuOrientation::Horizontal, keyboard::Key::Named(Named::ArrowLeft)) => {
            Some(match direction {
                Direction::LeftToRight => NavigationMenuCommand::Previous,
                Direction::RightToLeft => NavigationMenuCommand::Next,
            })
        }
        (NavigationMenuOrientation::Horizontal, keyboard::Key::Named(Named::ArrowRight)) => {
            Some(match direction {
                Direction::LeftToRight => NavigationMenuCommand::Next,
                Direction::RightToLeft => NavigationMenuCommand::Previous,
            })
        }
        (NavigationMenuOrientation::Horizontal, keyboard::Key::Named(Named::ArrowDown)) => {
            Some(NavigationMenuCommand::OpenContent)
        }
        (NavigationMenuOrientation::Horizontal, keyboard::Key::Named(Named::ArrowUp))
            if content_open =>
        {
            Some(NavigationMenuCommand::CloseContent)
        }
        (NavigationMenuOrientation::Vertical, keyboard::Key::Named(Named::ArrowUp))
            if content_open =>
        {
            Some(NavigationMenuCommand::CloseContent)
        }
        (NavigationMenuOrientation::Vertical, keyboard::Key::Named(Named::ArrowUp)) => {
            Some(NavigationMenuCommand::Previous)
        }
        (NavigationMenuOrientation::Vertical, keyboard::Key::Named(Named::ArrowDown)) => {
            Some(NavigationMenuCommand::Next)
        }
        (NavigationMenuOrientation::Vertical, keyboard::Key::Named(Named::ArrowRight))
            if direction == Direction::LeftToRight =>
        {
            Some(NavigationMenuCommand::OpenContent)
        }
        (NavigationMenuOrientation::Vertical, keyboard::Key::Named(Named::ArrowLeft))
            if direction == Direction::RightToLeft =>
        {
            Some(NavigationMenuCommand::OpenContent)
        }
        (_, keyboard::Key::Named(Named::Home)) => Some(NavigationMenuCommand::First),
        (_, keyboard::Key::Named(Named::End)) => Some(NavigationMenuCommand::Last),
        (_, keyboard::Key::Named(Named::Escape)) if content_open => {
            Some(NavigationMenuCommand::CloseContent)
        }
        _ => None,
    }
}

pub fn navigation_menu_trigger_id(id: &str, index: usize) -> widget::Id {
    widget::Id::from(format!("ducktape-navigation:{id}:trigger:{index}"))
}

pub fn navigation_menu_content_id(id: &str, index: usize) -> widget::Id {
    widget::Id::from(format!("ducktape-navigation:{id}:content:{index}"))
}

pub struct NavigationMenu<'a, Message>
where
    Message: Clone + 'a,
{
    id: String,
    items: Vec<NavigationMenuItem<'a, Message>>,
    state: NavigationMenuState,
    on_event: Rc<dyn Fn(NavigationMenuEvent) -> Message + 'a>,
    orientation: NavigationMenuOrientation,
    direction: Direction,
    viewport: bool,
    collapsed: bool,
    width: Length,
    content_width: f32,
    content_min_width: f32,
    content_max_width: f32,
    hover_delay: Duration,
    disabled: bool,
    theme: Theme,
}

pub fn navigation_menu<'a, Message>(
    id: impl Into<String>,
    items: impl IntoIterator<Item = NavigationMenuItem<'a, Message>>,
    state: &NavigationMenuState,
    on_event: impl Fn(NavigationMenuEvent) -> Message + 'a,
    theme: &Theme,
) -> NavigationMenu<'a, Message>
where
    Message: Clone + 'a,
{
    NavigationMenu {
        id: id.into(),
        items: items.into_iter().collect(),
        state: state.clone(),
        on_event: Rc::new(on_event),
        orientation: NavigationMenuOrientation::Horizontal,
        direction: Direction::LeftToRight,
        viewport: true,
        collapsed: false,
        width: Length::Shrink,
        content_width: NAVIGATION_MENU_CONTENT_WIDTH,
        content_min_width: NAVIGATION_MENU_CONTENT_MIN_WIDTH,
        content_max_width: NAVIGATION_MENU_CONTENT_MAX_WIDTH,
        hover_delay: NAVIGATION_MENU_HOVER_DELAY,
        disabled: false,
        theme: *theme,
    }
}

impl<Message> NavigationMenu<'_, Message>
where
    Message: Clone,
{
    #[must_use]
    pub fn orientation(mut self, orientation: NavigationMenuOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Uses one shared, collision-aware viewport when true; false anchors the
    /// same panel directly to the open trigger.
    #[must_use]
    pub fn viewport(mut self, viewport: bool) -> Self {
        self.viewport = viewport;
        self
    }

    /// Responsive/mobile mode: a full-width vertical roving list with content
    /// anchored to its disclosure trigger.
    #[must_use]
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        if collapsed {
            self.orientation = NavigationMenuOrientation::Vertical;
            self.viewport = false;
            self.width = Length::Fill;
        }
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn content_width(mut self, width: f32) -> Self {
        self.content_width = width;
        self
    }

    #[must_use]
    pub fn content_min_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.content_min_width = width;
        }
        self
    }

    #[must_use]
    pub fn content_max_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.content_max_width = width;
        }
        self
    }

    #[must_use]
    pub fn hover_delay(mut self, delay: Duration) -> Self {
        self.hover_delay = delay;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, Message> NavigationMenu<'a, Message>
where
    Message: Clone + 'a,
{
    fn into_widget(self) -> NavigationMenuWidget<'a, Message> {
        let orientation = if self.collapsed {
            NavigationMenuOrientation::Vertical
        } else {
            self.orientation
        };
        let infos: Rc<[NavigationMenuItemInfo]> = self
            .items
            .iter()
            .map(NavigationMenuItem::info)
            .map(|mut item| {
                item.disabled |= self.disabled;
                item
            })
            .collect();
        let tab_stop = self
            .state
            .focused
            .filter(|index| {
                infos
                    .get(*index)
                    .is_some_and(NavigationMenuItemInfo::enabled)
            })
            .or_else(|| infos.iter().position(NavigationMenuItemInfo::enabled));
        let open_index = self.state.open.filter(|index| {
            infos
                .get(*index)
                .is_some_and(|item| item.enabled() && item.disclosure)
        });
        let mut open_content = None;
        let mut triggers = Vec::with_capacity(self.items.len());

        for (index, item) in self.items.into_iter().enumerate() {
            let NavigationMenuItem {
                id,
                label,
                mut content,
                ..
            } = item;
            if open_index == Some(index) {
                open_content = content.take();
            }
            let opened = open_index == Some(index);
            let active = self.state.active.as_deref() == Some(id.as_str());
            let activate = (self.on_event)(activate_at(&self.state, &infos, index));
            let key_event = Rc::clone(&self.on_event);
            let key_state = self.state.clone();
            let key_infos = Rc::clone(&infos);
            let direction = self.direction;
            let trigger_theme = self.theme;
            let trigger = trigger_content(
                label,
                infos[index].disclosure,
                opened || active,
                orientation,
                direction,
                &self.theme,
            );
            triggers.push(Element::from(
                FocusControl::new(
                    navigation_menu_trigger_id(&self.id, index),
                    trigger,
                    activate,
                    &self.theme,
                )
                .disabled(!infos[index].enabled())
                .tab_stop(tab_stop == Some(index))
                .on_key_press(move |key, _modifiers| {
                    let command = navigation_menu_command(
                        &key,
                        orientation,
                        direction,
                        key_state.open.is_some(),
                    )?;
                    Some(key_event(reduce_navigation_menu(
                        &key_state, &key_infos, command,
                    )))
                })
                .style(move |_iced_theme, status| {
                    navigation_menu_trigger_style(&trigger_theme, opened, active, status)
                }),
            ));
        }

        let logical_to_visual = logical_to_visual_map(infos.len(), orientation, self.direction);
        let visual_triggers = match (orientation, self.direction) {
            (NavigationMenuOrientation::Horizontal, Direction::RightToLeft) => {
                triggers.into_iter().rev().collect()
            }
            _ => triggers,
        };
        let list: Element<'a, Message> = match orientation {
            NavigationMenuOrientation::Horizontal => Row::with_children(visual_triggers)
                .spacing(4)
                .align_y(IcedAlignment::Center)
                .into(),
            NavigationMenuOrientation::Vertical => Column::with_children(visual_triggers)
                .spacing(2)
                .width(Length::Fill)
                .into(),
        };
        let bar: Element<'a, Message> = container(list)
            .width(self.width)
            .style(|_iced_theme| iced::widget::container::Style::default())
            .into();
        let (_, max_width, content_width) = resolved_content_widths(
            self.content_min_width,
            self.content_max_width,
            self.content_width,
        );
        let content = panel(
            open_content.unwrap_or_else(|| Row::new().into()),
            PanelKind::Popover,
            Some(content_width),
            max_width,
            Padding::new(MENU_PANEL_PADDING),
            &self.theme,
        );

        NavigationMenuWidget {
            id: self.id,
            bar,
            content,
            state: self.state,
            items: infos,
            open_index,
            logical_to_visual,
            on_event: self.on_event,
            config: navigation_menu_floating_config(orientation, self.direction, max_width),
            viewport: self.viewport && !self.collapsed,
            hover_delay: self.hover_delay,
        }
    }
}

impl<'a, Message> From<NavigationMenu<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(menu: NavigationMenu<'a, Message>) -> Self {
        Element::new(menu.into_widget())
    }
}

fn logical_to_visual_map(
    count: usize,
    orientation: NavigationMenuOrientation,
    direction: Direction,
) -> Vec<usize> {
    match (orientation, direction) {
        (NavigationMenuOrientation::Horizontal, Direction::RightToLeft) => {
            (0..count).rev().collect()
        }
        _ => (0..count).collect(),
    }
}

fn resolved_content_widths(min: f32, max: f32, width: f32) -> (f32, f32, f32) {
    let min = if min.is_finite() && min > 0.0 {
        min
    } else {
        NAVIGATION_MENU_CONTENT_MIN_WIDTH
    };
    let max = if max.is_finite() && max >= min {
        max
    } else {
        min.max(NAVIGATION_MENU_CONTENT_MAX_WIDTH)
    };
    let width = if width.is_finite() && width > 0.0 {
        width.clamp(min, max)
    } else {
        NAVIGATION_MENU_CONTENT_WIDTH.clamp(min, max)
    };
    (min, max, width)
}

fn trigger_content<'a, Message>(
    label: String,
    disclosure: bool,
    indicator: bool,
    orientation: NavigationMenuOrientation,
    direction: Direction,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let foreground = theme.palette.foreground;
    let label = container(
        text(label)
            .size(theme.typography.sm)
            .line_height(LineHeight::Absolute(Pixels(16.0)))
            .color(foreground),
    )
    .width(if orientation == NavigationMenuOrientation::Vertical {
        Length::Fill
    } else {
        Length::Shrink
    })
    .align_x(direction.start())
    .align_y(Vertical::Center);
    let chevron = disclosure.then(|| {
        let glyph = match orientation {
            NavigationMenuOrientation::Horizontal => "⌄",
            NavigationMenuOrientation::Vertical if direction == Direction::LeftToRight => "›",
            NavigationMenuOrientation::Vertical => "‹",
        };
        container(
            text(glyph)
                .size(theme.typography.base)
                .line_height(LineHeight::Absolute(Pixels(16.0)))
                .color(alpha(foreground, 0.72)),
        )
        .width(16)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
    });
    let mut main = Row::new().align_y(IcedAlignment::Center).spacing(4);
    match direction {
        Direction::LeftToRight => {
            main = main.push(label);
            if let Some(chevron) = chevron {
                main = main.push(chevron);
            }
        }
        Direction::RightToLeft => {
            if let Some(chevron) = chevron {
                main = main.push(chevron);
            }
            main = main.push(label);
        }
    }
    if orientation == NavigationMenuOrientation::Vertical {
        main = main.width(Length::Fill);
    }
    let main = container(main)
        .height(match orientation {
            NavigationMenuOrientation::Horizontal => {
                NAVIGATION_MENU_TRIGGER_HEIGHT - NAVIGATION_MENU_INDICATOR_HEIGHT
            }
            NavigationMenuOrientation::Vertical => NAVIGATION_MENU_TRIGGER_HEIGHT,
        })
        .padding([0.0, 12.0])
        .align_y(Vertical::Center);
    let marker = indicator_element(indicator, orientation, theme);

    match (orientation, direction) {
        (NavigationMenuOrientation::Horizontal, _) => Column::new()
            .push(main)
            .push(
                container(marker)
                    .height(NAVIGATION_MENU_INDICATOR_HEIGHT)
                    .align_x(Horizontal::Center),
            )
            .align_x(IcedAlignment::Center)
            .height(NAVIGATION_MENU_TRIGGER_HEIGHT)
            .into(),
        (NavigationMenuOrientation::Vertical, Direction::LeftToRight) => Row::new()
            .push(
                container(marker)
                    .width(NAVIGATION_MENU_INDICATOR_HEIGHT)
                    .height(NAVIGATION_MENU_TRIGGER_HEIGHT)
                    .align_y(Vertical::Center),
            )
            .push(main)
            .width(Length::Fill)
            .height(NAVIGATION_MENU_TRIGGER_HEIGHT)
            .into(),
        (NavigationMenuOrientation::Vertical, Direction::RightToLeft) => Row::new()
            .push(main)
            .push(
                container(marker)
                    .width(NAVIGATION_MENU_INDICATOR_HEIGHT)
                    .height(NAVIGATION_MENU_TRIGGER_HEIGHT)
                    .align_y(Vertical::Center),
            )
            .width(Length::Fill)
            .height(NAVIGATION_MENU_TRIGGER_HEIGHT)
            .into(),
    }
}

fn indicator_element<'a, Message>(
    visible: bool,
    orientation: NavigationMenuOrientation,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let size = navigation_menu_indicator_size(orientation);
    let color = if visible {
        theme.palette.ring
    } else {
        iced::Color::TRANSPARENT
    };
    container(Space::new().width(size.width).height(size.height))
        .width(size.width)
        .height(size.height)
        .style(move |_iced_theme| iced::widget::container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: 1.0.into(),
                ..Border::default()
            },
            ..Default::default()
        })
        .into()
}

pub const fn navigation_menu_indicator_size(orientation: NavigationMenuOrientation) -> Size {
    match orientation {
        NavigationMenuOrientation::Horizontal => Size::new(
            NAVIGATION_MENU_INDICATOR_WIDTH,
            NAVIGATION_MENU_INDICATOR_HEIGHT,
        ),
        NavigationMenuOrientation::Vertical => Size::new(
            NAVIGATION_MENU_INDICATOR_HEIGHT,
            NAVIGATION_MENU_INDICATOR_WIDTH,
        ),
    }
}

pub fn navigation_menu_floating_config(
    orientation: NavigationMenuOrientation,
    direction: Direction,
    max_width: f32,
) -> FloatingConfig {
    let (placement, alignment) = match orientation {
        NavigationMenuOrientation::Horizontal => (
            Placement::Bottom,
            match direction {
                Direction::LeftToRight => Alignment::Start,
                Direction::RightToLeft => Alignment::End,
            },
        ),
        NavigationMenuOrientation::Vertical => (
            match direction {
                Direction::LeftToRight => Placement::Right,
                Direction::RightToLeft => Placement::Left,
            },
            Alignment::Start,
        ),
    };
    FloatingConfig {
        placement,
        alignment,
        side_offset: NAVIGATION_MENU_VIEWPORT_GAP,
        alignment_offset: 0.0,
        viewport_padding: NAVIGATION_MENU_VIEWPORT_PADDING,
        max_width,
    }
}

pub fn navigation_menu_viewport_style(theme: &Theme) -> iced::widget::container::Style {
    panel_style(theme, PanelKind::Popover)
}

pub fn navigation_menu_trigger_style(
    theme: &Theme,
    opened: bool,
    active: bool,
    status: Status,
) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match status {
        Status::Hovered | Status::Focused => Some(Background::Color(theme.palette.accent)),
        Status::Pressed => Some(Background::Color(mix(
            theme.palette.accent,
            theme.palette.foreground,
            0.08,
        ))),
        _ if opened || active => Some(Background::Color(theme.palette.accent)),
        _ => None,
    };
    style.text_color = Some(if status == Status::Disabled {
        alpha(theme.palette.foreground, 0.5)
    } else {
        theme.palette.foreground
    });
    style.border.radius = theme.radius.md.into();
    style.focus_ring.width = 1.0;
    style.focus_ring.radius = theme.radius.md.into();
    style.focus_offset = 1.0;
    style
}

/// A compact, leading-aligned content-list composition for disclosure panels.
pub fn navigation_menu_list<'a, Message>(
    items: impl IntoIterator<Item = Element<'a, Message>>,
) -> Column<'a, Message>
where
    Message: 'a,
{
    Column::with_children(items).spacing(4).width(Length::Fill)
}

/// One keyboard- and pointer-activatable title/description row for
/// [`navigation_menu_list`].
pub fn navigation_menu_list_link<'a, Message>(
    focus_id: widget::Id,
    title: impl Into<String>,
    description: impl Into<String>,
    on_activate: Message,
    direction: Direction,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let theme_value = *theme;
    let content = container(
        Column::new()
            .push(
                text(title.into())
                    .size(theme.typography.sm)
                    .line_height(LineHeight::Absolute(Pixels(16.0)))
                    .color(theme.palette.foreground),
            )
            .push(
                text(description.into())
                    .size(theme.typography.sm)
                    .line_height(LineHeight::Absolute(Pixels(18.0)))
                    .color(theme.palette.muted_foreground),
            )
            .spacing(2)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding([10.0, 12.0])
    .align_x(direction.start());
    FocusControl::new(focus_id, content, on_activate, theme)
        .style(move |_iced_theme, status| navigation_menu_list_link_style(&theme_value, status))
        .into()
}

pub fn navigation_menu_list_link_style(theme: &Theme, status: Status) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match status {
        Status::Hovered | Status::Focused => Some(Background::Color(theme.palette.accent)),
        Status::Pressed => Some(Background::Color(mix(
            theme.palette.accent,
            theme.palette.foreground,
            0.08,
        ))),
        _ => None,
    };
    style.text_color = Some(if status == Status::Disabled {
        alpha(theme.palette.foreground, 0.5)
    } else {
        theme.palette.foreground
    });
    style.border.radius = theme.radius.sm.into();
    style.focus_ring.width = 1.0;
    style.focus_ring.radius = theme.radius.sm.into();
    style.focus_offset = 1.0;
    style
}

struct NavigationMenuWidget<'a, Message> {
    id: String,
    bar: Element<'a, Message>,
    content: Element<'a, Message>,
    state: NavigationMenuState,
    items: Rc<[NavigationMenuItemInfo]>,
    open_index: Option<usize>,
    logical_to_visual: Vec<usize>,
    on_event: Rc<dyn Fn(NavigationMenuEvent) -> Message + 'a>,
    config: FloatingConfig,
    viewport: bool,
    hover_delay: Duration,
}

#[derive(Debug, Default)]
struct State {
    content_focus: FocusFlag,
    hover: HoverIntent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HoverIntent {
    observed: Option<usize>,
    pending: Option<(usize, Instant)>,
}

impl HoverIntent {
    fn observe(&mut self, target: Option<usize>, now: Instant, delay: Duration) {
        if self.observed == target {
            return;
        }
        self.observed = target;
        self.pending = target.map(|target| (target, now + delay));
    }

    fn take_ready(&mut self, now: Instant) -> Option<usize> {
        let (target, deadline) = self.pending?;
        if now < deadline {
            return None;
        }
        self.pending = None;
        Some(target)
    }

    const fn deadline(self) -> Option<Instant> {
        match self.pending {
            Some((_, deadline)) => Some(deadline),
            None => None,
        }
    }

    fn clear(&mut self) {
        self.observed = None;
        self.pending = None;
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for NavigationMenuWidget<'_, Message>
where
    Message: Clone,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.bar),
            widget::Tree::new(&self.content),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.bar.as_widget(), self.content.as_widget()]);
        if self.open_index.is_none() {
            tree.state.downcast_mut::<State>().content_focus.unfocus();
        }
        if self.items.iter().all(|item| !item.enabled()) {
            tree.state.downcast_mut::<State>().hover.clear();
        }
    }

    fn size(&self) -> Size<Length> {
        self.bar.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.bar.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.bar
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.traverse(&mut |operation| {
            self.bar
                .as_widget_mut()
                .operate(&mut tree.children[0], layout, renderer, operation);
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
        self.bar.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let triggers = trigger_bounds(layout, Vector::ZERO);
        update_hover(
            &mut tree.state.downcast_mut::<State>().hover,
            event,
            cursor.position(),
            &triggers,
            &self.logical_to_visual,
            &self.items,
            &self.state,
            self.hover_delay,
            self.on_event.as_ref(),
            shell,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.bar.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
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
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.bar.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        _renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        let logical = self.open_index?;
        let visual = *self.logical_to_visual.get(logical)?;
        let triggers = trigger_bounds(layout, translation);
        let trigger = *triggers.get(visual)?;
        let bar = translated_bounds(layout.bounds(), translation);
        let anchor = if self.viewport { bar } else { trigger };
        let state = tree.state.downcast_mut::<State>();
        let content_tree = tree.children.get_mut(1)?;
        Some(overlay::Element::new(Box::new(NavigationMenuOverlay {
            floating: FloatingContent {
                content: &mut self.content,
                tree: content_tree,
                anchor,
                viewport: *viewport,
                config: self.config,
            },
            triggers,
            logical_to_visual: &self.logical_to_visual,
            state: &self.state,
            items: &self.items,
            content_focus: &mut state.content_focus,
            hover: &mut state.hover,
            content_id: navigation_menu_content_id(&self.id, logical),
            on_event: self.on_event.as_ref(),
            hover_delay: self.hover_delay,
        })))
    }
}

fn translated_bounds(bounds: Rectangle, translation: Vector) -> Rectangle {
    Rectangle::new(bounds.position() + translation, bounds.size())
}

fn trigger_bounds(layout: Layout<'_>, translation: Vector) -> Vec<Rectangle> {
    layout
        .children()
        .next()
        .into_iter()
        .flat_map(Layout::children)
        .map(|layout| translated_bounds(layout.bounds(), translation))
        .collect()
}

fn logical_at_point(
    point: iced::Point,
    triggers: &[Rectangle],
    logical_to_visual: &[usize],
) -> Option<usize> {
    let visual = triggers.iter().position(|bounds| bounds.contains(point))?;
    logical_to_visual
        .iter()
        .position(|mapped| *mapped == visual)
}

#[allow(clippy::too_many_arguments)]
fn update_hover<Message>(
    hover: &mut HoverIntent,
    event: &Event,
    cursor: Option<iced::Point>,
    triggers: &[Rectangle],
    logical_to_visual: &[usize],
    items: &[NavigationMenuItemInfo],
    state: &NavigationMenuState,
    delay: Duration,
    on_event: &dyn Fn(NavigationMenuEvent) -> Message,
    shell: &mut Shell<'_, Message>,
) {
    if !matches!(event, Event::Mouse(_) | Event::Window(_)) {
        return;
    }
    let now = event_time(event);
    let target = cursor
        .and_then(|point| logical_at_point(point, triggers, logical_to_visual))
        .filter(|index| {
            items
                .get(*index)
                .is_some_and(|item| item.enabled() && item.disclosure)
                && state.open != Some(*index)
        });
    hover.observe(target, now, delay);
    if let Some(target) = hover.take_ready(now) {
        let mut next = state.clone();
        next.open = Some(target);
        shell.publish(on_event(changed(next, None)));
        shell.request_redraw();
    } else if let Some(deadline) = hover.deadline() {
        shell.request_redraw_at(deadline);
    }
}

struct NavigationMenuOverlay<'a, 'b, Message> {
    floating: FloatingContent<'a, 'b, Message>,
    triggers: Vec<Rectangle>,
    logical_to_visual: &'b [usize],
    state: &'b NavigationMenuState,
    items: &'b [NavigationMenuItemInfo],
    content_focus: &'b mut FocusFlag,
    hover: &'b mut HoverIntent,
    content_id: widget::Id,
    on_event: &'b dyn Fn(NavigationMenuEvent) -> Message,
    hover_delay: Duration,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for NavigationMenuOverlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        self.floating.layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.floating.draw(renderer, theme, style, layout, cursor);
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.focusable(
            Some(&self.content_id),
            self.floating.bounds(layout),
            self.content_focus,
        );
        operation.traverse(&mut |operation| self.floating.operate(layout, renderer, operation));
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        update_hover(
            self.hover,
            event,
            cursor.position(),
            &self.triggers,
            self.logical_to_visual,
            self.items,
            self.state,
            self.hover_delay,
            self.on_event,
            shell,
        );
        if matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape | Named::ArrowUp),
                ..
            })
        ) {
            self.content_focus.unfocus();
            shell.publish((self.on_event)(reduce_navigation_menu(
                self.state,
                self.items,
                NavigationMenuCommand::CloseContent,
            )));
            shell.capture_event();
            return;
        }
        let press = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor.position(),
            Event::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
            _ => None,
        };
        if let Some(point) = press
            && !self.floating.bounds(layout).contains(point)
        {
            let event = logical_at_point(point, &self.triggers, self.logical_to_visual)
                .filter(|index| self.items[*index].enabled())
                .map_or_else(
                    || {
                        let mut next = self.state.clone();
                        next.open = None;
                        changed(next, None)
                    },
                    |index| activate_at(self.state, self.items, index),
                );
            shell.publish((self.on_event)(event));
            shell.capture_event();
            return;
        }
        self.floating
            .update(event, layout, cursor, renderer, clipboard, shell);
        if self.content_focus.is_focused()
            && focus_within(|operation| self.floating.operate(layout, renderer, operation))
        {
            self.content_focus.unfocus();
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.floating.interaction(layout, cursor, renderer, true)
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        self.floating.overlay(layout, renderer)
    }

    fn index(&self) -> f32 {
        15.0
    }
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::focusable_count;
    use super::super::popover::resolve_position;
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::advanced::renderer::Headless as _;
    use iced::widget::text_input;
    use iced::{Point, Size};

    fn items() -> Vec<NavigationMenuItemInfo> {
        vec![
            NavigationMenuItemInfo::link("home"),
            NavigationMenuItemInfo::disclosure("products").disabled(true),
            NavigationMenuItemInfo::disclosure("docs"),
        ]
    }

    #[test]
    fn reducer_skips_disabled_wraps_and_preserves_open_switching() {
        let state = NavigationMenuState {
            focused: Some(0),
            open: Some(0),
            active: None,
        };
        let next = reduce_navigation_menu(&state, &items(), NavigationMenuCommand::Next);
        assert_eq!(next.state().focused, Some(2));
        assert_eq!(next.state().open, Some(2));
        assert_eq!(next.focus(), Some(NavigationMenuFocus::Trigger(2)));

        let wrapped = reduce_navigation_menu(next.state(), &items(), NavigationMenuCommand::Next);
        assert_eq!(wrapped.state().focused, Some(0));
        assert_eq!(wrapped.state().open, None);
    }

    #[test]
    fn link_activation_sets_active_route_and_disclosure_toggles() {
        let state = NavigationMenuState::initial(&items());
        let link = reduce_navigation_menu(&state, &items(), NavigationMenuCommand::Activate);
        assert!(matches!(
            link,
            NavigationMenuEvent::LinkActivated { ref id, .. } if id == "home"
        ));
        assert_eq!(link.state().active.as_deref(), Some("home"));

        let state = NavigationMenuState {
            focused: Some(2),
            ..NavigationMenuState::default()
        };
        let open = reduce_navigation_menu(&state, &items(), NavigationMenuCommand::Activate);
        assert_eq!(open.state().open, Some(2));
        let close = reduce_navigation_menu(open.state(), &items(), NavigationMenuCommand::Activate);
        assert_eq!(close.state().open, None);
    }

    #[test]
    fn down_enters_content_and_close_restores_the_stable_trigger() {
        let state = NavigationMenuState {
            focused: Some(2),
            ..NavigationMenuState::default()
        };
        let open = reduce_navigation_menu(&state, &items(), NavigationMenuCommand::OpenContent);
        assert_eq!(open.focus(), Some(NavigationMenuFocus::Content(2)));
        assert_eq!(open.state().open, Some(2));
        let close =
            reduce_navigation_menu(open.state(), &items(), NavigationMenuCommand::CloseContent);
        assert_eq!(close.focus(), Some(NavigationMenuFocus::Trigger(2)));
        assert_eq!(
            navigation_menu_trigger_id("main", 2),
            navigation_menu_trigger_id("main", 2)
        );
        assert_ne!(
            navigation_menu_trigger_id("main", 2),
            navigation_menu_content_id("main", 2)
        );
    }

    #[test]
    fn close_content_falls_back_when_the_open_item_was_removed() {
        let state = NavigationMenuState {
            focused: Some(2),
            open: Some(2),
            active: None,
        };
        let items = [NavigationMenuItemInfo::link("home")];

        let closed = reduce_navigation_menu(&state, &items, NavigationMenuCommand::CloseContent);

        assert_eq!(closed.state().focused, Some(0));
        assert_eq!(closed.state().open, None);
        assert_eq!(closed.focus(), Some(NavigationMenuFocus::Trigger(0)));
    }

    #[test]
    fn orientation_and_direction_map_physical_keys() {
        let left = keyboard::Key::Named(Named::ArrowLeft);
        let down = keyboard::Key::Named(Named::ArrowDown);
        assert_eq!(
            navigation_menu_command(
                &left,
                NavigationMenuOrientation::Horizontal,
                Direction::LeftToRight,
                false,
            ),
            Some(NavigationMenuCommand::Previous)
        );
        assert_eq!(
            navigation_menu_command(
                &left,
                NavigationMenuOrientation::Horizontal,
                Direction::RightToLeft,
                false,
            ),
            Some(NavigationMenuCommand::Next)
        );
        assert_eq!(
            navigation_menu_command(
                &down,
                NavigationMenuOrientation::Horizontal,
                Direction::LeftToRight,
                false,
            ),
            Some(NavigationMenuCommand::OpenContent)
        );
        assert_eq!(
            navigation_menu_command(
                &down,
                NavigationMenuOrientation::Vertical,
                Direction::LeftToRight,
                false,
            ),
            Some(NavigationMenuCommand::Next)
        );
    }

    #[test]
    fn hover_intent_uses_one_deadline_and_never_changes_keyboard_focus() {
        let start = Instant::now();
        let delay = Duration::from_millis(150);
        let mut hover = HoverIntent::default();
        hover.observe(Some(2), start, delay);
        assert_eq!(hover.deadline(), Some(start + delay));
        assert_eq!(
            hover.take_ready(start + delay - Duration::from_millis(1)),
            None
        );
        assert_eq!(hover.take_ready(start + delay), Some(2));

        let state = NavigationMenuState {
            focused: Some(0),
            ..NavigationMenuState::default()
        };
        let mut next = state.clone();
        next.open = Some(2);
        assert_eq!(next.focused, state.focused);
        hover.observe(None, start + delay, delay);
        assert_eq!(hover.deadline(), None);
    }

    #[test]
    fn viewport_collision_and_indicator_geometry_are_exact() {
        assert_eq!(
            navigation_menu_indicator_size(NavigationMenuOrientation::Horizontal),
            Size::new(12.0, 2.0)
        );
        assert_eq!(
            navigation_menu_indicator_size(NavigationMenuOrientation::Vertical),
            Size::new(2.0, 12.0)
        );
        let config = navigation_menu_floating_config(
            NavigationMenuOrientation::Horizontal,
            Direction::LeftToRight,
            NAVIGATION_MENU_CONTENT_MAX_WIDTH,
        );
        let resolved = resolve_position(
            Rectangle::new(Point::new(80.0, 170.0), Size::new(120.0, 36.0)),
            Size::new(320.0, 120.0),
            Rectangle::with_size(Size::new(360.0, 240.0)),
            config,
        );
        assert_eq!(resolved.placement, Placement::Top);
        assert!(resolved.bounds.x >= NAVIGATION_MENU_VIEWPORT_PADDING);
        assert!(
            resolved.bounds.x + resolved.bounds.width <= 360.0 - NAVIGATION_MENU_VIEWPORT_PADDING
        );
    }

    #[test]
    fn width_constraints_sanitize_and_clamp() {
        assert_eq!(
            resolved_content_widths(240.0, 600.0, 800.0),
            (240.0, 600.0, 600.0)
        );
        assert_eq!(
            resolved_content_widths(f32::NAN, f32::NAN, f32::NAN),
            (240.0, 600.0, 480.0)
        );
    }

    #[test]
    fn light_and_dark_styles_keep_border_shadow_focus_and_disabled_feedback() {
        for theme in [LIGHT, DARK] {
            let viewport = navigation_menu_viewport_style(&theme);
            let focused = navigation_menu_trigger_style(&theme, false, false, Status::Focused);
            let disabled = navigation_menu_trigger_style(&theme, false, false, Status::Disabled);
            assert_eq!(viewport.border.width, 1.0);
            assert!(viewport.shadow.blur_radius > 0.0);
            assert_eq!(focused.focus_ring.color, theme.palette.ring);
            assert_eq!(focused.focus_ring.width, 1.0);
            assert_eq!(
                disabled.text_color,
                Some(alpha(theme.palette.foreground, 0.5))
            );
        }
    }

    #[test]
    fn builds_one_trigger_list_and_one_shared_viewport_tree() {
        let infos = items();
        let state = NavigationMenuState::initial(&infos);
        let element: Element<'_, ()> = navigation_menu(
            "main",
            [
                NavigationMenuItem::link("home", "Home"),
                NavigationMenuItem::disclosure("docs", "Docs", text("Documentation")),
            ],
            &state,
            |_| (),
            &LIGHT,
        )
        .viewport(true)
        .into();
        assert_eq!(element.as_widget().children().len(), 2);
        assert_eq!(focusable_count(element), 1);
    }

    #[test]
    fn pointer_focused_content_releases_the_viewport_proxy() {
        struct FindBounds {
            id: widget::Id,
            bounds: Option<Rectangle>,
        }

        impl widget::Operation for FindBounds {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
                operate(self);
            }

            fn focusable(
                &mut self,
                id: Option<&widget::Id>,
                bounds: Rectangle,
                _state: &mut dyn widget::operation::Focusable,
            ) {
                if id == Some(&self.id) {
                    self.bounds = Some(bounds);
                }
            }
        }

        let menu_id = "focus-transfer";
        let child_id = widget::Id::new("navigation-content-child");
        let state = NavigationMenuState {
            focused: Some(0),
            open: Some(0),
            active: None,
        };
        let content = text_input("Action", "")
            .id(child_id.clone())
            .on_input(|_| ())
            .width(120.0);
        let mut widget = navigation_menu(
            menu_id,
            [NavigationMenuItem::disclosure("docs", "Docs", content)],
            &state,
            |_| (),
            &LIGHT,
        )
        .into_widget();
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let viewport = Rectangle::with_size(Size::new(640.0, 480.0));
        let mut tree = widget::Tree::new(&widget as &dyn Widget<_, _, _>);
        let node = widget.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let mut overlay = widget
            .overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &viewport,
                Vector::ZERO,
            )
            .expect("open navigation overlay");
        let overlay_node = overlay.as_overlay_mut().layout(&renderer, viewport.size());
        let mut focus_proxy =
            widget::operation::focusable::focus::<()>(navigation_menu_content_id(menu_id, 0));
        overlay
            .as_overlay_mut()
            .operate(Layout::new(&overlay_node), &renderer, &mut focus_proxy);
        let mut find = FindBounds {
            id: child_id.clone(),
            bounds: None,
        };
        overlay
            .as_overlay_mut()
            .operate(Layout::new(&overlay_node), &renderer, &mut find);
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        overlay.as_overlay_mut().update(
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&overlay_node),
            mouse::Cursor::Available(find.bounds.expect("child bounds").center()),
            &renderer,
            &mut clipboard,
            &mut shell,
        );

        let descendant_focused = focus_within(|operation| {
            overlay
                .as_overlay_mut()
                .operate(Layout::new(&overlay_node), &renderer, operation);
        });
        drop(overlay);

        assert!(
            !tree
                .state
                .downcast_ref::<State>()
                .content_focus
                .is_focused()
        );
        assert!(descendant_focused);
    }
}
