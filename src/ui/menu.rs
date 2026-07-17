//! Shared, controlled interaction and presentation for menu-like components.
//!
//! Iced does not publish DOM-style menu roles. This module therefore exposes
//! stable focus IDs and the expected pointer/keyboard behavior without making
//! accessibility claims the runtime cannot uphold.

use std::rc::Rc;

use super::direction::Direction;
use super::focus_control::{self, FocusControl, Status};
use super::theme::{Theme, alpha, mix};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::widget::text::LineHeight;
use iced::widget::{Column, Row, Space, container, rule, text};
use iced::{Alignment, Background, Border, Element, Length, Padding, Pixels, Task};

pub const MENU_ROW_HEIGHT: f32 = 32.0;
pub const MENU_PANEL_PADDING: f32 = 8.0;
pub const MENU_INDENT: f32 = 20.0;

pub type MenuPath = Vec<usize>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuState {
    pub focused: Option<MenuPath>,
    pub open_submenus: Vec<MenuPath>,
    pub typeahead: String,
}

impl MenuState {
    pub fn initial(entries: &[MenuEntry]) -> Self {
        let mut state = Self::default();
        state.focused = visible_items(entries, &state)
            .into_iter()
            .find(|(_, item)| !item.disabled)
            .map(|(path, _)| path);
        state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    Item(MenuItem),
    Label(MenuLabel),
    Separator { id: String },
    Group(MenuGroup),
}

impl MenuEntry {
    pub fn item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Item(MenuItem::new(id, label))
    }

    pub fn label(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Label(MenuLabel {
            id: id.into(),
            label: label.into(),
            inset: false,
        })
    }

    pub fn separator(id: impl Into<String>) -> Self {
        Self::Separator { id: id.into() }
    }

    pub fn group(id: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        Self::Group(MenuGroup {
            id: id.into(),
            label: None,
            entries,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuGroup {
    pub id: String,
    pub label: Option<String>,
    pub entries: Vec<MenuEntry>,
}

impl MenuGroup {
    pub fn new(id: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        Self {
            id: id.into(),
            label: None,
            entries,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuLabel {
    pub id: String,
    pub label: String,
    pub inset: bool,
}

impl MenuLabel {
    #[must_use]
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub inset: bool,
    pub kind: MenuItemKind,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            disabled: false,
            inset: false,
            kind: MenuItemKind::Action,
        }
    }

    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    #[must_use]
    pub fn checkbox(mut self, checked: bool) -> Self {
        self.kind = MenuItemKind::Checkbox { checked };
        self
    }

    #[must_use]
    pub fn radio(mut self, group: impl Into<String>, checked: bool) -> Self {
        self.kind = MenuItemKind::Radio {
            group: group.into(),
            checked,
        };
        self
    }

    #[must_use]
    pub fn submenu(mut self, entries: Vec<MenuEntry>) -> Self {
        self.kind = MenuItemKind::Submenu(entries);
        self
    }
}

impl From<MenuItem> for MenuEntry {
    fn from(item: MenuItem) -> Self {
        Self::Item(item)
    }
}

impl From<MenuGroup> for MenuEntry {
    fn from(group: MenuGroup) -> Self {
        Self::Group(group)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemKind {
    Action,
    Checkbox { checked: bool },
    Radio { group: String, checked: bool },
    Submenu(Vec<MenuEntry>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    Previous,
    Next,
    First,
    Last,
    Activate,
    Forward,
    Back,
    Escape,
    Character(char),
    ClearTypeahead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopLevelMove {
    Previous,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuActivation {
    pub id: String,
    pub path: MenuPath,
    pub kind: MenuActivationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuActivationKind {
    Action,
    Checkbox { checked: bool },
    Radio { group: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEvent {
    StateChanged(MenuState),
    Activated(MenuActivation),
    Dismiss,
    MoveTopLevel(TopLevelMove),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuTransition {
    pub state: MenuState,
    pub event: Option<MenuEvent>,
}

/// Applies one menu command. State remains caller-owned; the returned event is
/// the message a rendered [`Menu`] emits for the same command.
pub fn reduce_menu(
    state: &MenuState,
    entries: &[MenuEntry],
    command: MenuCommand,
) -> MenuTransition {
    let mut next = state.clone();
    let items = visible_items(entries, state);
    let enabled = items
        .iter()
        .filter(|(_, item)| !item.disabled)
        .collect::<Vec<_>>();

    let changed = |state: MenuState| MenuTransition {
        event: Some(MenuEvent::StateChanged(state.clone())),
        state,
    };

    match command {
        MenuCommand::Previous | MenuCommand::Next | MenuCommand::First | MenuCommand::Last => {
            let target = navigation_target(next.focused.as_ref(), &enabled, command);
            let Some(target) = target else {
                return MenuTransition {
                    state: next,
                    event: None,
                };
            };
            next.focused = Some(target);
            next.typeahead.clear();
            changed(next)
        }
        MenuCommand::Activate | MenuCommand::Forward => {
            let Some(path) = next.focused.clone() else {
                return MenuTransition {
                    state: next,
                    event: None,
                };
            };
            let Some(item) = item_at(entries, &path).filter(|item| !item.disabled) else {
                return MenuTransition {
                    state: next,
                    event: None,
                };
            };
            if let MenuItemKind::Submenu(children) = &item.kind {
                if !next.open_submenus.contains(&path) {
                    next.open_submenus.push(path.clone());
                }
                if let Some((child, _)) = first_enabled(children, &path, &next) {
                    next.focused = Some(child);
                }
                return changed(next);
            }
            if command == MenuCommand::Forward {
                return MenuTransition {
                    state: next,
                    event: Some(MenuEvent::MoveTopLevel(TopLevelMove::Next)),
                };
            }
            MenuTransition {
                state: next,
                event: Some(MenuEvent::Activated(activation(item, path))),
            }
        }
        MenuCommand::Back | MenuCommand::Escape => {
            if let Some(parent) = nearest_open_parent(next.focused.as_deref(), &next.open_submenus)
            {
                next.open_submenus
                    .retain(|open| !is_same_or_descendant(open, &parent));
                next.focused = Some(parent);
                next.typeahead.clear();
                changed(next)
            } else {
                MenuTransition {
                    state: next,
                    event: Some(if command == MenuCommand::Escape {
                        MenuEvent::Dismiss
                    } else {
                        MenuEvent::MoveTopLevel(TopLevelMove::Previous)
                    }),
                }
            }
        }
        MenuCommand::Character(character) => {
            if character.is_control() {
                return MenuTransition {
                    state: next,
                    event: None,
                };
            }
            let character = character.to_lowercase().collect::<String>();
            // ponytail: single-character cycling avoids a timer; add timestamped
            // buffering if multi-character typeahead becomes a real requirement.
            next.typeahead = character;
            let query = next.typeahead.as_str();
            let start = enabled
                .iter()
                .position(|(path, _)| Some(path) == next.focused.as_ref())
                .map_or(0, |index| index + 1);
            let target = enabled
                .iter()
                .cycle()
                .skip(start)
                .take(enabled.len())
                .find(|(_, item)| item.label.to_lowercase().starts_with(query))
                .map(|(path, _)| (*path).clone());
            if let Some(target) = target {
                next.focused = Some(target);
            }
            changed(next)
        }
        MenuCommand::ClearTypeahead => {
            if next.typeahead.is_empty() {
                MenuTransition {
                    state: next,
                    event: None,
                }
            } else {
                next.typeahead.clear();
                changed(next)
            }
        }
    }
}

fn navigation_target(
    current: Option<&MenuPath>,
    enabled: &[&(MenuPath, &MenuItem)],
    command: MenuCommand,
) -> Option<MenuPath> {
    if enabled.is_empty() {
        return None;
    }
    let current = current.and_then(|current| enabled.iter().position(|(path, _)| path == current));
    let index = match command {
        MenuCommand::First => 0,
        MenuCommand::Last => enabled.len() - 1,
        MenuCommand::Next => current.map_or(0, |index| (index + 1) % enabled.len()),
        MenuCommand::Previous => current.map_or(enabled.len() - 1, |index| {
            (index + enabled.len() - 1) % enabled.len()
        }),
        _ => return None,
    };
    Some(enabled[index].0.clone())
}

fn first_enabled<'a>(
    entries: &'a [MenuEntry],
    prefix: &[usize],
    state: &MenuState,
) -> Option<(MenuPath, &'a MenuItem)> {
    let mut paths = Vec::new();
    collect_visible(entries, prefix, state, &mut paths);
    paths.into_iter().find(|(_, item)| !item.disabled)
}

fn visible_items<'a>(entries: &'a [MenuEntry], state: &MenuState) -> Vec<(MenuPath, &'a MenuItem)> {
    let mut items = Vec::new();
    collect_visible(entries, &[], state, &mut items);
    items
}

fn resolved_focus<'a>(
    entries: &'a [MenuEntry],
    state: &MenuState,
) -> Option<(MenuPath, &'a MenuItem)> {
    let visible = visible_items(entries, state);
    if let Some(focused) = &state.focused
        && let Some((path, item)) = visible
            .iter()
            .find(|(path, item)| path == focused && !item.disabled)
    {
        return Some((path.clone(), *item));
    }
    visible.into_iter().find(|(_, item)| !item.disabled)
}

fn collect_visible<'a>(
    entries: &'a [MenuEntry],
    prefix: &[usize],
    state: &MenuState,
    output: &mut Vec<(MenuPath, &'a MenuItem)>,
) {
    for (index, entry) in entries.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(index);
        match entry {
            MenuEntry::Item(item) => {
                output.push((path.clone(), item));
                if let MenuItemKind::Submenu(children) = &item.kind
                    && !item.disabled
                    && state.open_submenus.contains(&path)
                {
                    collect_visible(children, &path, state, output);
                }
            }
            MenuEntry::Group(group) => collect_visible(&group.entries, &path, state, output),
            MenuEntry::Label(_) | MenuEntry::Separator { .. } => {}
        }
    }
}

fn item_at<'a>(entries: &'a [MenuEntry], path: &[usize]) -> Option<&'a MenuItem> {
    let (&index, rest) = path.split_first()?;
    match entries.get(index)? {
        MenuEntry::Item(item) if rest.is_empty() => Some(item),
        MenuEntry::Item(MenuItem {
            kind: MenuItemKind::Submenu(children),
            ..
        }) => item_at(children, rest),
        MenuEntry::Group(group) => item_at(&group.entries, rest),
        MenuEntry::Item(_) | MenuEntry::Label(_) | MenuEntry::Separator { .. } => None,
    }
}

fn nearest_open_parent(focused: Option<&[usize]>, open: &[MenuPath]) -> Option<MenuPath> {
    let focused = focused?;
    open.iter()
        .filter(|parent| focused.starts_with(parent.as_slice()))
        .max_by_key(|parent| parent.len())
        .cloned()
}

fn is_same_or_descendant(path: &[usize], parent: &[usize]) -> bool {
    path.starts_with(parent)
}

fn activation(item: &MenuItem, path: MenuPath) -> MenuActivation {
    let kind = match &item.kind {
        MenuItemKind::Action => MenuActivationKind::Action,
        MenuItemKind::Checkbox { checked } => MenuActivationKind::Checkbox { checked: !checked },
        MenuItemKind::Radio { group, .. } => MenuActivationKind::Radio {
            group: group.clone(),
        },
        MenuItemKind::Submenu(_) => unreachable!("submenus open instead of activating"),
    };
    MenuActivation {
        id: item.id.clone(),
        path,
        kind,
    }
}

pub fn menu_command(key: &keyboard::Key, direction: Direction) -> Option<MenuCommand> {
    match key {
        keyboard::Key::Named(Named::ArrowUp) => Some(MenuCommand::Previous),
        keyboard::Key::Named(Named::ArrowDown) => Some(MenuCommand::Next),
        keyboard::Key::Named(Named::Home) => Some(MenuCommand::First),
        keyboard::Key::Named(Named::End) => Some(MenuCommand::Last),
        keyboard::Key::Named(Named::Escape) => Some(MenuCommand::Escape),
        keyboard::Key::Named(Named::ArrowRight) if direction == Direction::LeftToRight => {
            Some(MenuCommand::Forward)
        }
        keyboard::Key::Named(Named::ArrowLeft) if direction == Direction::RightToLeft => {
            Some(MenuCommand::Forward)
        }
        keyboard::Key::Named(Named::ArrowLeft) if direction == Direction::LeftToRight => {
            Some(MenuCommand::Back)
        }
        keyboard::Key::Named(Named::ArrowRight) if direction == Direction::RightToLeft => {
            Some(MenuCommand::Back)
        }
        keyboard::Key::Character(value) => value.chars().next().map(MenuCommand::Character),
        _ => None,
    }
}

pub struct Menu<'a, Message>
where
    Message: Clone + 'a,
{
    id: String,
    entries: Rc<[MenuEntry]>,
    state: Rc<MenuState>,
    on_event: Rc<dyn Fn(MenuEvent) -> Message + 'a>,
    width: Length,
    direction: Direction,
    disabled: bool,
    theme: Theme,
}

pub fn menu<'a, Message>(
    id: impl Into<String>,
    entries: &[MenuEntry],
    state: &MenuState,
    on_event: impl Fn(MenuEvent) -> Message + 'a,
    theme: &Theme,
) -> Menu<'a, Message>
where
    Message: Clone + 'a,
{
    Menu {
        id: id.into(),
        entries: Rc::from(entries),
        state: Rc::new(state.clone()),
        on_event: Rc::new(on_event),
        width: Length::Fill,
        direction: Direction::LeftToRight,
        disabled: false,
        theme: *theme,
    }
}

impl<Message> Menu<'_, Message>
where
    Message: Clone,
{
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, Message> Menu<'a, Message>
where
    Message: Clone + 'a,
{
    pub fn into_element(self) -> Element<'a, Message> {
        let focused = resolved_focus(&self.entries, &self.state).map(|(path, _)| path);
        let state = if self.state.focused.as_ref() == focused.as_ref() {
            Rc::clone(&self.state)
        } else {
            let mut state = (*self.state).clone();
            state.focused = focused;
            Rc::new(state)
        };
        let mut children = Vec::new();
        render_entries(
            Rc::clone(&self.entries),
            &self.entries,
            state,
            &self.id,
            &[],
            0,
            self.direction,
            self.disabled,
            &self.theme,
            &self.on_event,
            &mut children,
        );
        Column::with_children(children).width(self.width).into()
    }
}

impl<'a, Message> From<Menu<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(menu: Menu<'a, Message>) -> Self {
        menu.into_element()
    }
}

#[allow(clippy::too_many_arguments)]
fn render_entries<'a, Message>(
    root: Rc<[MenuEntry]>,
    entries: &[MenuEntry],
    state: Rc<MenuState>,
    menu_id: &str,
    prefix: &[usize],
    depth: usize,
    direction: Direction,
    menu_disabled: bool,
    theme: &Theme,
    on_event: &Rc<dyn Fn(MenuEvent) -> Message + 'a>,
    output: &mut Vec<Element<'a, Message>>,
) where
    Message: Clone + 'a,
{
    for (index, entry) in entries.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(index);
        match entry {
            MenuEntry::Item(item) => {
                output.push(render_item(
                    Rc::clone(&root),
                    Rc::clone(&state),
                    menu_id,
                    path.clone(),
                    item,
                    depth,
                    direction,
                    menu_disabled,
                    theme,
                    on_event,
                ));
                if let MenuItemKind::Submenu(children) = &item.kind
                    && !menu_disabled
                    && !item.disabled
                    && state.open_submenus.contains(&path)
                {
                    render_entries(
                        Rc::clone(&root),
                        children,
                        Rc::clone(&state),
                        menu_id,
                        &path,
                        depth + 1,
                        direction,
                        menu_disabled || item.disabled,
                        theme,
                        on_event,
                        output,
                    );
                }
            }
            MenuEntry::Label(label) => output.push(render_label(
                &label.label,
                label.inset || depth > 0,
                direction,
                theme,
            )),
            MenuEntry::Separator { .. } => output.push(
                container(rule::horizontal(1).style({
                    let color = theme.palette.border;
                    move |_theme| iced::widget::rule::Style {
                        color,
                        radius: 0.0.into(),
                        fill_mode: iced::widget::rule::FillMode::Full,
                        snap: true,
                    }
                }))
                .padding([4, 0])
                .into(),
            ),
            MenuEntry::Group(group) => {
                if let Some(label) = &group.label {
                    output.push(render_label(label, depth > 0, direction, theme));
                }
                render_entries(
                    Rc::clone(&root),
                    &group.entries,
                    Rc::clone(&state),
                    menu_id,
                    &path,
                    depth,
                    direction,
                    menu_disabled,
                    theme,
                    on_event,
                    output,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_item<'a, Message>(
    root: Rc<[MenuEntry]>,
    state: Rc<MenuState>,
    menu_id: &str,
    path: MenuPath,
    item: &MenuItem,
    depth: usize,
    direction: Direction,
    menu_disabled: bool,
    theme: &Theme,
    on_event: &Rc<dyn Fn(MenuEvent) -> Message + 'a>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let disabled = menu_disabled || item.disabled;
    let selected = state.focused.as_ref() == Some(&path);
    let indicator = match &item.kind {
        MenuItemKind::Checkbox { checked: true } => "✓",
        MenuItemKind::Radio { checked: true, .. } => "●",
        _ => "",
    };
    let arrow = matches!(item.kind, MenuItemKind::Submenu(_)).then_some(match direction {
        Direction::LeftToRight => "›",
        Direction::RightToLeft => "‹",
    });
    let muted = if disabled {
        alpha(theme.palette.muted_foreground, 0.5)
    } else {
        theme.palette.muted_foreground
    };
    let leading: Element<'a, Message> = container(
        text(indicator)
            .size(theme.typography.sm)
            .line_height(LineHeight::Absolute(Pixels(16.0))),
    )
    .width(16)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .into();
    let label: Element<'a, Message> = container(
        text(item.label.clone())
            .size(theme.typography.sm)
            .line_height(LineHeight::Absolute(Pixels(16.0))),
    )
    .width(Length::Fill)
    .align_x(direction.start())
    .align_y(Vertical::Center)
    .into();
    let trailing: Element<'a, Message> = if let Some(shortcut) = &item.shortcut {
        container(
            text(shortcut.clone())
                .size(theme.typography.xs)
                .line_height(LineHeight::Absolute(Pixels(16.0)))
                .color(muted),
        )
        .align_x(direction.end())
        .align_y(Vertical::Center)
        .into()
    } else if let Some(arrow) = arrow {
        container(
            text(arrow)
                .size(theme.typography.base)
                .line_height(LineHeight::Absolute(Pixels(16.0)))
                .color(muted),
        )
        .width(16)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
    } else {
        Space::new().width(0).into()
    };
    let parts = match direction {
        Direction::LeftToRight => vec![leading, label, trailing],
        Direction::RightToLeft => vec![trailing, label, leading],
    };
    let row = Row::with_children(parts)
        .spacing(theme.spacing.sm)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    let inset = item.inset || depth > 0;
    let horizontal = 8.0
        + if inset {
            MENU_INDENT * (depth.max(1) as f32)
        } else {
            0.0
        };
    let padding = match direction {
        Direction::LeftToRight => Padding {
            left: horizontal,
            right: 8.0,
            top: 0.0,
            bottom: 0.0,
        },
        Direction::RightToLeft => Padding {
            left: 8.0,
            right: horizontal,
            top: 0.0,
            bottom: 0.0,
        },
    };
    let content = container(row)
        .height(MENU_ROW_HEIGHT)
        .width(Length::Fill)
        .padding(padding)
        .align_y(Vertical::Center);
    let mut click_state = (*state).clone();
    click_state.focused = Some(path.clone());
    let activate = reduce_menu(&click_state, &root, MenuCommand::Activate)
        .event
        .map(|event| on_event(event))
        .unwrap_or_else(|| on_event(MenuEvent::StateChanged((*state).clone())));
    let key_entries = Rc::clone(&root);
    let key_state = Rc::clone(&state);
    let key_event = Rc::clone(on_event);
    let key_direction = direction;
    let item_theme = *theme;

    FocusControl::new(
        menu_item_id(menu_id, &item.id, &path),
        content,
        activate,
        theme,
    )
    .disabled(disabled)
    .tab_stop(selected)
    .on_key_press(move |key, modifiers| {
        if modifiers.control() || modifiers.alt() || modifiers.logo() {
            return None;
        }
        let command = menu_command(&key, key_direction)?;
        reduce_menu(&key_state, &key_entries, command)
            .event
            .map(|event| key_event(event))
    })
    .style(move |_iced_theme, status| menu_item_style(&item_theme, selected, status))
    .into()
}

fn render_label<'a, Message>(
    label: &str,
    inset: bool,
    direction: Direction,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let start = 8.0 + if inset { MENU_INDENT } else { 0.0 };
    let padding = match direction {
        Direction::LeftToRight => Padding {
            left: start,
            right: 8.0,
            top: 4.0,
            bottom: 4.0,
        },
        Direction::RightToLeft => Padding {
            left: 8.0,
            right: start,
            top: 4.0,
            bottom: 4.0,
        },
    };
    container(
        text(label.to_owned())
            .size(theme.typography.xs)
            .line_height(LineHeight::Absolute(Pixels(16.0))),
    )
    .width(Length::Fill)
    .padding(padding)
    .align_x(direction.start())
    .align_y(Vertical::Center)
    .into()
}

/// Stable focus ID. `item_id` must be unique within one menu, including nested
/// submenus; paths are accepted for convenient callers but reordering does not
/// change the focus identity.
pub fn menu_item_id(menu_id: &str, item_id: &str, _path: &[usize]) -> iced::widget::Id {
    iced::widget::Id::from(format!("ducktape-menu:{menu_id}:{item_id}"))
}

pub fn focus_menu_item<Message>(menu_id: &str, item_id: &str, path: &[usize]) -> Task<Message> {
    iced::widget::operation::focus(menu_item_id(menu_id, item_id, path))
}

pub fn focus_menu_state<Message>(
    menu_id: &str,
    entries: &[MenuEntry],
    state: &MenuState,
) -> Task<Message> {
    resolved_focus(entries, state).map_or_else(Task::none, |(path, item)| {
        focus_menu_item(menu_id, &item.id, &path)
    })
}

pub fn menu_item_style(theme: &Theme, selected: bool, status: Status) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match status {
        Status::Hovered | Status::Focused => Some(Background::Color(theme.palette.accent)),
        Status::Pressed => Some(Background::Color(mix(
            theme.palette.accent,
            theme.palette.foreground,
            0.08,
        ))),
        Status::Active if selected => Some(Background::Color(theme.palette.accent)),
        Status::Active | Status::Disabled => None,
    };
    style.text_color = Some(if status == Status::Disabled {
        alpha(theme.palette.popover_foreground, 0.5)
    } else {
        theme.palette.popover_foreground
    });
    style.border = Border {
        radius: theme.radius.sm.into(),
        ..Border::default()
    };
    style.focus_ring = Border {
        color: theme.palette.ring,
        width: 1.0,
        radius: theme.radius.sm.into(),
    };
    style.focus_offset = 0.0;
    style
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::focusable_count;
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    fn entries() -> Vec<MenuEntry> {
        vec![
            MenuItem::new("new", "New").shortcut("⌘N").into(),
            MenuItem::new("save", "Save").disabled(true).into(),
            MenuItem::new("theme", "Theme")
                .submenu(vec![
                    MenuItem::new("light", "Light").radio("theme", true).into(),
                    MenuItem::new("dark", "Dark").radio("theme", false).into(),
                ])
                .into(),
            MenuItem::new("autosave", "Autosave").checkbox(true).into(),
        ]
    }

    #[test]
    fn navigation_wraps_and_skips_disabled_items() {
        let entries = entries();
        let mut state = MenuState::initial(&entries);
        assert_eq!(state.focused, Some(vec![0]));
        state = reduce_menu(&state, &entries, MenuCommand::Next).state;
        assert_eq!(state.focused, Some(vec![2]));
        state = reduce_menu(&state, &entries, MenuCommand::Last).state;
        assert_eq!(state.focused, Some(vec![3]));
        state = reduce_menu(&state, &entries, MenuCommand::Next).state;
        assert_eq!(state.focused, Some(vec![0]));
    }

    #[test]
    fn nested_paths_open_close_and_activate() {
        let entries = entries();
        let state = MenuState {
            focused: Some(vec![2]),
            ..MenuState::default()
        };
        let opened = reduce_menu(&state, &entries, MenuCommand::Forward);
        assert_eq!(opened.state.open_submenus, vec![vec![2]]);
        assert_eq!(opened.state.focused, Some(vec![2, 0]));
        let activated = reduce_menu(&opened.state, &entries, MenuCommand::Activate);
        assert!(matches!(
            activated.event,
            Some(MenuEvent::Activated(MenuActivation {
                id,
                kind: MenuActivationKind::Radio { group },
                ..
            })) if id == "light" && group == "theme"
        ));
        let closed = reduce_menu(&opened.state, &entries, MenuCommand::Back);
        assert_eq!(closed.state.open_submenus, Vec::<MenuPath>::new());
        assert_eq!(closed.state.focused, Some(vec![2]));
    }

    #[test]
    fn typeahead_matches_visible_enabled_labels() {
        let entries = entries();
        let state = MenuState::initial(&entries);
        let state = reduce_menu(&state, &entries, MenuCommand::Character('a')).state;
        assert_eq!(state.focused, Some(vec![3]));
        assert_eq!(state.typeahead, "a");
        let cleared = reduce_menu(&state, &entries, MenuCommand::ClearTypeahead).state;
        assert!(cleared.typeahead.is_empty());
    }

    #[test]
    fn repeated_typeahead_character_cycles_matches() {
        let entries = vec![
            MenuEntry::item("apple", "Apple"),
            MenuEntry::item("apricot", "Apricot"),
        ];
        let state = reduce_menu(
            &MenuState::initial(&entries),
            &entries,
            MenuCommand::Character('a'),
        )
        .state;
        assert_eq!(state.focused, Some(vec![1]));
        let state = reduce_menu(&state, &entries, MenuCommand::Character('a')).state;
        assert_eq!(state.focused, Some(vec![0]));
        assert_eq!(state.typeahead, "a");
    }

    #[test]
    fn a_new_typeahead_character_replaces_the_previous_one() {
        let entries = entries();
        let state = reduce_menu(
            &MenuState::initial(&entries),
            &entries,
            MenuCommand::Character('a'),
        )
        .state;
        let state = reduce_menu(&state, &entries, MenuCommand::Character('b')).state;
        assert_eq!(state.typeahead, "b");
    }

    #[test]
    fn direction_swaps_submenu_arrows() {
        let right = keyboard::Key::Named(Named::ArrowRight);
        let left = keyboard::Key::Named(Named::ArrowLeft);
        assert_eq!(
            menu_command(&right, Direction::LeftToRight),
            Some(MenuCommand::Forward)
        );
        assert_eq!(
            menu_command(&left, Direction::RightToLeft),
            Some(MenuCommand::Forward)
        );
        assert_eq!(
            menu_command(&left, Direction::LeftToRight),
            Some(MenuCommand::Back)
        );
    }

    #[test]
    fn stable_ids_follow_caller_item_ids_across_reordering() {
        assert_eq!(
            menu_item_id("file", "open", &[1, 2]),
            menu_item_id("file", "open", &[4])
        );
        assert_ne!(
            menu_item_id("file", "open", &[1, 2]),
            menu_item_id("file", "close", &[1, 2])
        );
    }

    #[test]
    fn exact_row_geometry_and_semantic_styles_hold() {
        assert_eq!(MENU_ROW_HEIGHT, 32.0);
        assert_eq!(MENU_PANEL_PADDING, 8.0);
        for theme in [LIGHT, DARK] {
            let active = menu_item_style(&theme, true, Status::Active);
            let disabled = menu_item_style(&theme, false, Status::Disabled);
            let focused = menu_item_style(&theme, false, Status::Focused);
            assert_eq!(
                active.background,
                Some(Background::Color(theme.palette.accent))
            );
            assert!(disabled.text_color.expect("disabled color").a < 1.0);
            assert_eq!(focused.focus_ring.color, theme.palette.ring);
            assert_eq!(focused.focus_ring.width, 1.0);
        }
    }

    #[test]
    fn menu_builds_groups_labels_separators_and_items() {
        let entries = vec![
            MenuGroup::new(
                "editing",
                vec![MenuEntry::item("copy", "Copy"), MenuEntry::separator("s")],
            )
            .label("Editing")
            .into(),
            MenuEntry::label("account-label", "Account"),
            MenuEntry::item("logout", "Log out"),
        ];
        let state = MenuState::initial(&entries);
        let element: Element<'_, ()> = menu("test", &entries, &state, |_| (), &LIGHT).into();
        assert_eq!(element.as_widget().children().len(), 5);
    }

    #[test]
    fn resolved_menu_focus_matches_its_rendered_tab_stop() {
        let entries = vec![
            MenuItem::new("disabled", "Disabled")
                .disabled(true)
                .submenu(vec![MenuEntry::item("hidden", "Hidden")])
                .into(),
            MenuEntry::item("enabled", "Enabled"),
        ];
        let default = MenuState::default();
        let stale = MenuState {
            focused: Some(vec![0, 0]),
            open_submenus: vec![vec![0]],
            ..MenuState::default()
        };
        let element: Element<'_, ()> = menu("file", &entries, &stale, |_| (), &LIGHT).into();

        assert_eq!(
            resolved_focus(&entries, &default).map(|(path, _)| path),
            Some(vec![1])
        );
        assert_eq!(
            focus_menu_state::<()>("file", &entries, &default).units(),
            1
        );
        assert_eq!(
            resolved_focus(&entries, &stale).map(|(path, _)| path),
            Some(vec![1])
        );
        assert_eq!(element.as_widget().children().len(), 2);
        assert_eq!(focusable_count(element), 1);
    }
}
