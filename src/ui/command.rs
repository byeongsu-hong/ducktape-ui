//! A controlled command palette with a native Iced text input.
//!
//! The input is wrapped only to intercept list-navigation keys while it owns
//! focus. All editing, paste, selection, and input-method events are delegated
//! unchanged to Iced's native [`iced::widget::TextInput`].

use std::rc::Rc;

use super::focus_control::{self, FocusControl, Status};
use super::input::{InputVariant, style as base_input_style};
use super::scroll_area::scroll_area;
use super::theme::{Theme, alpha, mix};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, text as advanced_text,
    widget,
};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::widget::text::LineHeight;
use iced::widget::{Column, Container, Row, Space, TextInput, container, text, text_input};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Pixels, Rectangle, Shadow, Size,
    Task, Vector,
};

pub const COMMAND_INPUT_HEIGHT: f32 = 36.0;
pub const COMMAND_ITEM_HEIGHT: f32 = 36.0;
pub const COMMAND_DEFAULT_RESULTS_HEIGHT: f32 = 280.0;

/// Query and active item owned by the application.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandState {
    query: String,
    active: Option<String>,
}

impl CommandState {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            active: None,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.active = None;
    }

    pub fn set_active(&mut self, active: impl Into<String>) {
        self.active = Some(active.into());
    }

    pub fn clear_active(&mut self) {
        self.active = None;
    }

    /// Applies the controlled portion of a command event.
    pub fn apply<Value>(&mut self, event: &CommandEvent<Value>) -> bool {
        match event {
            CommandEvent::QueryChanged(query) => {
                if self.query == *query && self.active.is_none() {
                    false
                } else {
                    self.query.clone_from(query);
                    self.active = None;
                    true
                }
            }
            CommandEvent::Navigate { item_id, .. }
            | CommandEvent::Selected { item_id, value: _ } => {
                if self.active.as_ref() == Some(item_id) {
                    false
                } else {
                    self.active = Some(item_id.clone());
                    true
                }
            }
        }
    }
}

/// A query update, keyboard highlight move, or item selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEvent<Value> {
    QueryChanged(String),
    Navigate {
        item_id: String,
        /// `true` when navigation started on a focused result item. The caller
        /// should then run [`CommandEvent::focus_task`] after applying the event.
        focus_item: bool,
    },
    Selected {
        item_id: String,
        value: Value,
    },
}

impl<Value> CommandEvent<Value> {
    pub fn item_id(&self) -> Option<&str> {
        match self {
            Self::QueryChanged(_) => None,
            Self::Navigate { item_id, .. } | Self::Selected { item_id, .. } => Some(item_id),
        }
    }

    pub fn selection(&self) -> Option<&Value> {
        match self {
            Self::Selected { value, .. } => Some(value),
            Self::QueryChanged(_) | Self::Navigate { .. } => None,
        }
    }

    /// Moves focus only for navigation that originated on a result item.
    /// Input-originated navigation deliberately keeps native text editing focus.
    pub fn focus_task<Message>(&self, command_id: &str) -> Task<Message> {
        match self {
            Self::Navigate {
                item_id,
                focus_item: true,
            } => focus_command_item(command_id, item_id),
            Self::QueryChanged(_)
            | Self::Navigate {
                focus_item: false, ..
            }
            | Self::Selected { .. } => Task::none(),
        }
    }
}

/// One searchable command result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandItem<Value> {
    id: String,
    value: Value,
    label: String,
    keywords: Vec<String>,
    shortcut: Option<String>,
    disabled: bool,
}

pub fn command_item<Value>(
    id: impl Into<String>,
    value: Value,
    label: impl Into<String>,
) -> CommandItem<Value> {
    CommandItem {
        id: id.into(),
        value,
        label: label.into(),
        keywords: Vec::new(),
        shortcut: None,
        disabled: false,
    }
}

impl<Value> CommandItem<Value> {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    #[must_use]
    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
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

    pub fn matches(&self, query: &str) -> bool {
        let query = normalize_query(query);
        if query.is_empty() {
            return true;
        }

        let mut haystack = normalize_query(&self.label);
        for keyword in &self.keywords {
            haystack.push(' ');
            haystack.push_str(&normalize_query(keyword));
        }

        query.split(' ').all(|term| haystack.contains(term))
    }
}

/// An ordered result group with an optional heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandGroup<Value> {
    heading: Option<String>,
    items: Vec<CommandItem<Value>>,
}

pub fn command_group<Value>(
    heading: impl Into<String>,
    items: impl IntoIterator<Item = CommandItem<Value>>,
) -> CommandGroup<Value> {
    CommandGroup {
        heading: Some(heading.into()),
        items: items.into_iter().collect(),
    }
}

pub fn command_group_without_heading<Value>(
    items: impl IntoIterator<Item = CommandItem<Value>>,
) -> CommandGroup<Value> {
    CommandGroup {
        heading: None,
        items: items.into_iter().collect(),
    }
}

impl<Value> CommandGroup<Value> {
    pub fn heading(&self) -> Option<&str> {
        self.heading.as_deref()
    }

    pub fn items(&self) -> &[CommandItem<Value>] {
        &self.items
    }
}

/// One filtered item and its original group/item positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMatch<'a, Value> {
    pub group: usize,
    pub item: usize,
    pub command: &'a CommandItem<Value>,
}

/// Filters without reordering, so stable IDs and visual group order survive a query.
pub fn filter_items<'a, Value>(
    groups: &'a [CommandGroup<Value>],
    query: &str,
) -> Vec<CommandMatch<'a, Value>> {
    groups
        .iter()
        .enumerate()
        .flat_map(|(group, commands)| {
            commands
                .items
                .iter()
                .enumerate()
                .filter(move |(_, command)| command.matches(query))
                .map(move |(item, command)| CommandMatch {
                    group,
                    item,
                    command,
                })
        })
        .collect()
}

/// Lowercases Unicode and collapses whitespace for predictable matching.
pub fn normalize_query(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandMove {
    Previous,
    Next,
    First,
    Last,
}

pub fn command_move(key: &keyboard::Key) -> Option<CommandMove> {
    match key {
        keyboard::Key::Named(Named::ArrowUp) => Some(CommandMove::Previous),
        keyboard::Key::Named(Named::ArrowDown) => Some(CommandMove::Next),
        keyboard::Key::Named(Named::Home) => Some(CommandMove::First),
        keyboard::Key::Named(Named::End) => Some(CommandMove::Last),
        _ => None,
    }
}

/// Finds the next enabled result, wrapping for arrows and skipping disabled rows.
pub fn navigation_target(
    current: Option<usize>,
    enabled: &[bool],
    movement: CommandMove,
) -> Option<usize> {
    if enabled.iter().all(|enabled| !enabled) {
        return None;
    }

    match movement {
        CommandMove::First => enabled.iter().position(|enabled| *enabled),
        CommandMove::Last => enabled.iter().rposition(|enabled| *enabled),
        CommandMove::Next | CommandMove::Previous => {
            let len = enabled.len();
            let Some(start) = current.filter(|index| *index < len && enabled[*index]) else {
                return match movement {
                    CommandMove::Next => enabled.iter().position(|enabled| *enabled),
                    CommandMove::Previous => enabled.iter().rposition(|enabled| *enabled),
                    CommandMove::First | CommandMove::Last => unreachable!(),
                };
            };

            (1..=len)
                .map(|distance| match movement {
                    CommandMove::Next => (start + distance) % len,
                    CommandMove::Previous => (start + len - distance % len) % len,
                    CommandMove::First | CommandMove::Last => unreachable!(),
                })
                .find(|index| enabled[*index])
        }
    }
}

/// Builder for a controlled command palette/list.
pub struct Command<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + 'a,
{
    id: String,
    query: String,
    active: Option<String>,
    groups: Vec<CommandGroup<Value>>,
    on_event: Rc<dyn Fn(CommandEvent<Value>) -> Message + 'a>,
    placeholder: String,
    empty: String,
    results_height: f32,
    width: Length,
    group_separators: bool,
    theme: Theme,
}

pub fn command<'a, Message, Value>(
    id: impl Into<String>,
    state: &CommandState,
    groups: impl IntoIterator<Item = CommandGroup<Value>>,
    on_event: impl Fn(CommandEvent<Value>) -> Message + 'a,
    theme: &Theme,
) -> Command<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + 'a,
{
    Command {
        id: id.into(),
        query: state.query.clone(),
        active: state.active.clone(),
        groups: groups.into_iter().collect(),
        on_event: Rc::new(on_event),
        placeholder: "Type a command or search…".into(),
        empty: "No results found.".into(),
        results_height: COMMAND_DEFAULT_RESULTS_HEIGHT,
        width: Length::Fill,
        group_separators: true,
        theme: *theme,
    }
}

impl<'a, Message, Value> Command<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + 'a,
{
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    #[must_use]
    pub fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = empty.into();
        self
    }

    #[must_use]
    pub fn results_height(mut self, height: f32) -> Self {
        self.results_height = height.max(COMMAND_ITEM_HEIGHT);
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn group_separators(mut self, group_separators: bool) -> Self {
        self.group_separators = group_separators;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        struct Target<Value> {
            id: String,
            value: Value,
            disabled: bool,
        }

        let normalized_query = normalize_query(&self.query);
        let groups: Vec<_> = self
            .groups
            .into_iter()
            .filter_map(|group| {
                let items: Vec<_> = group
                    .items
                    .into_iter()
                    .filter(|item| item.matches(&normalized_query))
                    .collect();
                (!items.is_empty()).then_some((group.heading, items))
            })
            .collect();
        let targets: Rc<[Target<Value>]> = groups
            .iter()
            .flat_map(|(_, items)| items)
            .map(|item| Target {
                id: item.id.clone(),
                value: item.value.clone(),
                disabled: item.disabled,
            })
            .collect();
        let enabled: Rc<[bool]> = targets.iter().map(|target| !target.disabled).collect();
        let active = self
            .active
            .as_deref()
            .and_then(|active| {
                targets
                    .iter()
                    .position(|target| !target.disabled && target.id == active)
            })
            .or_else(|| enabled.iter().position(|enabled| *enabled));
        let input_id = command_input_id(&self.id);
        let input_theme = self.theme;
        let on_query = Rc::clone(&self.on_event);
        let on_submit = active.map(|index| {
            let target = &targets[index];
            (self.on_event)(CommandEvent::Selected {
                item_id: target.id.clone(),
                value: target.value.clone(),
            })
        });
        let native_input: TextInput<'a, Message> = text_input(&self.placeholder, &self.query)
            .id(input_id)
            .on_input(move |query| on_query(CommandEvent::QueryChanged(query)))
            .on_submit_maybe(on_submit)
            .padding([8.0, 12.0])
            .size(self.theme.typography.sm)
            .line_height(LineHeight::Absolute(Pixels(20.0)))
            .width(Length::Fill)
            .style(move |_iced_theme, status| command_input_style(&input_theme, status));
        let input_targets = Rc::clone(&targets);
        let input_enabled = Rc::clone(&enabled);
        let input_event = Rc::clone(&self.on_event);
        let input: Element<'a, Message> = CommandInput::new(native_input.into(), move |movement| {
            let target = navigation_target(active, &input_enabled, movement)?;
            Some(input_event(CommandEvent::Navigate {
                item_id: input_targets[target].id.clone(),
                focus_item: false,
            }))
        })
        .into();

        let mut results = Column::new().padding(self.theme.spacing.xs);
        let mut flat_index = 0;
        for (group_index, (heading, items)) in groups.into_iter().enumerate() {
            if group_index > 0 && self.group_separators {
                results = results
                    .push(Space::new().height(self.theme.spacing.xs))
                    .push(command_separator(&self.theme))
                    .push(Space::new().height(self.theme.spacing.xs));
            }
            if let Some(heading) = heading {
                results = results.push(group_heading(heading, &self.theme));
            }

            for item in items {
                let index = flat_index;
                flat_index += 1;
                let selected = active == Some(index);
                let disabled = item.disabled;
                let shortcut_color = if disabled {
                    alpha(self.theme.palette.muted_foreground, 0.5)
                } else {
                    self.theme.palette.muted_foreground
                };
                let label = container(
                    text(item.label)
                        .size(self.theme.typography.sm)
                        .line_height(LineHeight::Absolute(Pixels(16.0))),
                )
                .width(Length::Fill)
                .align_x(Horizontal::Left)
                .align_y(Vertical::Center);
                let mut row = Row::new()
                    .push(label)
                    .spacing(self.theme.spacing.sm)
                    .align_y(Alignment::Center)
                    .width(Length::Fill);
                if let Some(shortcut) = item.shortcut {
                    row = row.push(
                        container(
                            text(shortcut)
                                .size(self.theme.typography.xs)
                                .line_height(LineHeight::Absolute(Pixels(16.0)))
                                .color(shortcut_color),
                        )
                        .align_x(Horizontal::Right)
                        .align_y(Vertical::Center),
                    );
                }
                let content = container(row)
                    .padding([0.0, self.theme.spacing.sm])
                    .width(Length::Fill)
                    .height(COMMAND_ITEM_HEIGHT)
                    .align_y(Vertical::Center);
                let activate_target = &targets[index];
                let activate = (self.on_event)(CommandEvent::Selected {
                    item_id: activate_target.id.clone(),
                    value: activate_target.value.clone(),
                });
                let key_targets = Rc::clone(&targets);
                let key_enabled = Rc::clone(&enabled);
                let key_event = Rc::clone(&self.on_event);
                let item_theme = self.theme;

                results = results.push(
                    FocusControl::new(
                        command_item_id(&self.id, &item.id),
                        content,
                        activate,
                        &self.theme,
                    )
                    .disabled(disabled)
                    .on_key_press(move |key, _modifiers| {
                        let movement = command_move(&key)?;
                        let target = navigation_target(Some(index), &key_enabled, movement)?;
                        Some(key_event(CommandEvent::Navigate {
                            item_id: key_targets[target].id.clone(),
                            focus_item: true,
                        }))
                    })
                    .style(move |_iced_theme, status| {
                        command_item_style(&item_theme, selected, status)
                    }),
                );
            }
        }

        let results: Element<'a, Message> = if targets.is_empty() {
            container(
                text(self.empty)
                    .size(self.theme.typography.sm)
                    .color(self.theme.palette.muted_foreground),
            )
            .center_x(Length::Fill)
            .center_y(72)
            .into()
        } else {
            results.into()
        };
        let results = scroll_area(results, &self.theme).height(self.results_height);
        let theme = self.theme;

        container(
            Column::new()
                .push(input)
                .push(command_separator(&self.theme))
                .push(results)
                .width(Length::Fill),
        )
        .width(self.width)
        .style(move |_iced_theme| command_surface_style(&theme))
        .into()
    }
}

impl<'a, Message, Value> From<Command<'a, Message, Value>> for Element<'a, Message>
where
    Message: Clone + 'a,
    Value: Clone + 'a,
{
    fn from(command: Command<'a, Message, Value>) -> Self {
        command.into_element()
    }
}

fn group_heading<'a, Message>(heading: String, theme: &Theme) -> Container<'a, Message>
where
    Message: 'a,
{
    container(
        text(heading)
            .size(theme.typography.xs)
            .line_height(LineHeight::Absolute(Pixels(16.0)))
            .color(theme.palette.muted_foreground),
    )
    .padding([0.0, theme.spacing.sm])
    .width(Length::Fill)
    .height(28)
    .align_x(Horizontal::Left)
    .align_y(Vertical::Center)
}

fn command_separator<'a, Message>(theme: &Theme) -> Container<'a, Message>
where
    Message: 'a,
{
    let color = theme.palette.border;
    container(Space::new().width(Length::Fill).height(1))
        .width(Length::Fill)
        .height(1)
        .style(move |_iced_theme| iced::widget::container::Style {
            background: Some(Background::Color(color)),
            ..Default::default()
        })
}

/// Stable native input ID for opening a palette and restoring typing focus.
pub fn command_input_id(command_id: &str) -> widget::Id {
    widget::Id::from(format!(
        "ducktape-command-input:{}:{command_id}",
        command_id.len()
    ))
}

/// Stable result ID. Item IDs must be unique within one command palette.
pub fn command_item_id(command_id: &str, item_id: &str) -> widget::Id {
    widget::Id::from(format!(
        "ducktape-command-item:{}:{command_id}:{}:{item_id}",
        command_id.len(),
        item_id.len()
    ))
}

pub fn focus_command_input<Message>(command_id: &str) -> Task<Message> {
    iced::widget::operation::focus(command_input_id(command_id))
}

pub fn focus_command_item<Message>(command_id: &str, item_id: &str) -> Task<Message> {
    iced::widget::operation::focus(command_item_id(command_id, item_id))
}

pub fn command_surface_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.popover)),
        text_color: Some(theme.palette.popover_foreground),
        border: Border {
            color: theme.palette.border,
            width: 1.0,
            radius: theme.radius.lg.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.14),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

pub fn command_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = base_input_style(theme, InputVariant::Default, status);
    style.border = Border {
        radius: theme.radius.lg.into(),
        ..Border::default()
    };
    style.background = match status {
        text_input::Status::Focused { .. } => Background::Color(alpha(theme.palette.ring, 0.07)),
        text_input::Status::Disabled => style.background,
        text_input::Status::Active | text_input::Status::Hovered => {
            Background::Color(Color::TRANSPARENT)
        }
    };
    style
}

pub fn command_item_style(theme: &Theme, selected: bool, status: Status) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match status {
        Status::Disabled => selected.then_some(Background::Color(alpha(theme.palette.accent, 0.5))),
        Status::Pressed => Some(Background::Color(mix(
            theme.palette.accent,
            theme.palette.foreground,
            0.08,
        ))),
        Status::Hovered => Some(Background::Color(if selected {
            mix(theme.palette.accent, theme.palette.foreground, 0.04)
        } else {
            theme.palette.accent
        })),
        Status::Focused | Status::Active if selected => {
            Some(Background::Color(theme.palette.accent))
        }
        Status::Focused => Some(Background::Color(alpha(theme.palette.accent, 0.55))),
        Status::Active => None,
    };
    style.text_color = Some(if status == Status::Disabled {
        alpha(theme.palette.popover_foreground, 0.5)
    } else {
        theme.palette.popover_foreground
    });
    style.border.radius = theme.radius.sm.into();
    style.focus_ring.radius = (theme.radius.sm + 2.0).into();
    style.focus_ring.width = 1.5;
    style.focus_offset = 1.0;
    style
}

type KeyHandler<'a, Message> = dyn Fn(CommandMove) -> Option<Message> + 'a;

/// A transparent event adapter around one native text input.
struct CommandInput<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: advanced_text::Renderer,
{
    input: Element<'a, Message, Theme, Renderer>,
    on_key: Box<KeyHandler<'a, Message>>,
}

impl<'a, Message, Theme, Renderer> CommandInput<'a, Message, Theme, Renderer>
where
    Renderer: advanced_text::Renderer,
{
    fn new(
        input: Element<'a, Message, Theme, Renderer>,
        on_key: impl Fn(CommandMove) -> Option<Message> + 'a,
    ) -> Self {
        Self {
            input,
            on_key: Box::new(on_key),
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for CommandInput<'_, Message, Theme, Renderer>
where
    Renderer: advanced_text::Renderer,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.input)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.input));
    }

    fn size(&self) -> Size<Length> {
        self.input.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.input.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.input
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.traverse(&mut |operation| {
            self.input
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let focused = tree.children[0]
            .state
            .downcast_ref::<text_input::State<Renderer::Paragraph>>()
            .is_focused();

        if focused
            && !shell.is_event_captured()
            && let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && let Some(message) = command_move(key).and_then(|movement| (self.on_key)(movement))
        {
            shell.publish(message);
            shell.capture_event();
            return;
        }

        self.input.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.input.as_widget().mouse_interaction(
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
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.input.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.input.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<CommandInput<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: advanced_text::Renderer + 'a,
{
    fn from(input: CommandInput<'a, Message, Theme, Renderer>) -> Self {
        Element::new(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};

    fn groups() -> Vec<CommandGroup<u8>> {
        vec![
            command_group(
                "Suggestions",
                [
                    command_item("calendar", 1, "Open Calendar")
                        .keywords(["date", "schedule"])
                        .shortcut("⌘K"),
                    command_item("settings", 2, "Settings")
                        .keywords(["preferences"])
                        .disabled(true),
                ],
            ),
            command_group(
                "People",
                [command_item("zoe", 3, "Zoë Kravitz").keywords(["actor", "person"])],
            ),
        ]
    }

    #[test]
    fn filtering_uses_labels_and_keywords_without_reordering_groups() {
        let groups = groups();
        let label = filter_items(&groups, "  OPEN   calendar ");
        assert_eq!(label.len(), 1);
        assert_eq!((label[0].group, label[0].item), (0, 0));

        let keyword = filter_items(&groups, "person zoë");
        assert_eq!(keyword.len(), 1);
        assert_eq!(keyword[0].command.id(), "zoe");

        let all = filter_items(&groups, "");
        assert_eq!(
            all.iter().map(|item| item.command.id()).collect::<Vec<_>>(),
            ["calendar", "settings", "zoe"]
        );
    }

    #[test]
    fn normalization_is_unicode_case_aware_and_empty_lists_stay_empty() {
        assert_eq!(normalize_query("  ÅNGSTRÖM\tCAFÉ  "), "ångström café");
        assert!(command_item("unicode", (), "Ångström Café").matches("ÅNGSTRÖM"));
        assert!(filter_items::<()>(&[], "anything").is_empty());
    }

    #[test]
    fn navigation_wraps_and_skips_disabled_items() {
        let enabled = [true, false, true, false];
        assert_eq!(
            navigation_target(Some(0), &enabled, CommandMove::Next),
            Some(2)
        );
        assert_eq!(
            navigation_target(Some(2), &enabled, CommandMove::Next),
            Some(0)
        );
        assert_eq!(
            navigation_target(Some(0), &enabled, CommandMove::Previous),
            Some(2)
        );
        assert_eq!(
            navigation_target(None, &enabled, CommandMove::Previous),
            Some(2)
        );
        assert_eq!(
            navigation_target(None, &enabled, CommandMove::First),
            Some(0)
        );
        assert_eq!(
            navigation_target(None, &enabled, CommandMove::Last),
            Some(2)
        );
        assert_eq!(
            navigation_target(None, &[false, false], CommandMove::Next),
            None
        );
        assert_eq!(navigation_target(None, &[], CommandMove::Next), None);
    }

    #[test]
    fn controlled_events_update_active_state_and_expose_selection() {
        let mut state = CommandState::new("old");
        state.set_active("calendar");
        assert!(state.apply(&CommandEvent::<u8>::QueryChanged("new".into())));
        assert_eq!(state.query(), "new");
        assert_eq!(state.active(), None);

        let navigate = CommandEvent::<u8>::Navigate {
            item_id: "zoe".into(),
            focus_item: true,
        };
        assert!(state.apply(&navigate));
        assert_eq!(state.active(), Some("zoe"));
        assert_eq!(navigate.item_id(), Some("zoe"));

        let selected = CommandEvent::Selected {
            item_id: "calendar".into(),
            value: 7,
        };
        assert_eq!(selected.selection(), Some(&7));
        assert!(state.apply(&selected));
        assert_eq!(state.active(), Some("calendar"));
    }

    #[test]
    fn ids_are_stable_and_do_not_alias_colon_boundaries() {
        assert_eq!(command_input_id("palette"), command_input_id("palette"));
        assert_eq!(
            command_item_id("palette", "open"),
            command_item_id("palette", "open")
        );
        assert_ne!(command_item_id("a:b", "c"), command_item_id("a", "b:c"));
        assert_ne!(
            command_item_id("first", "open"),
            command_item_id("second", "open")
        );
    }

    #[test]
    fn geometry_and_alignment_anchors_match_the_component_contract() {
        assert_eq!(COMMAND_INPUT_HEIGHT, 36.0);
        assert_eq!(COMMAND_ITEM_HEIGHT, 36.0);
        assert_eq!(COMMAND_DEFAULT_RESULTS_HEIGHT, 280.0);
    }

    #[test]
    fn semantic_styles_cover_light_dark_selected_focus_and_disabled() {
        for theme in [LIGHT, DARK] {
            let surface = command_surface_style(&theme);
            assert_eq!(
                surface.background,
                Some(Background::Color(theme.palette.popover))
            );
            assert_eq!(surface.border.color, theme.palette.border);

            let selected = command_item_style(&theme, true, Status::Active);
            assert_eq!(
                selected.background,
                Some(Background::Color(theme.palette.accent))
            );

            let focused = command_item_style(&theme, false, Status::Focused);
            assert_eq!(focused.focus_ring.color, theme.palette.ring);
            assert!(focused.focus_ring.width > 0.0);

            let disabled = command_item_style(&theme, false, Status::Disabled);
            assert_eq!(
                disabled.text_color,
                Some(alpha(theme.palette.popover_foreground, 0.5))
            );

            let input =
                command_input_style(&theme, text_input::Status::Focused { is_hovered: false });
            assert_eq!(input.border.width, 0.0);
            assert_ne!(input.background, Background::Color(Color::TRANSPARENT));
        }
    }

    #[test]
    fn key_mapping_leaves_native_editing_keys_alone() {
        assert_eq!(
            command_move(&keyboard::Key::Named(Named::ArrowDown)),
            Some(CommandMove::Next)
        );
        assert_eq!(
            command_move(&keyboard::Key::Named(Named::Home)),
            Some(CommandMove::First)
        );
        assert_eq!(command_move(&keyboard::Key::Named(Named::Enter)), None);
        assert_eq!(command_move(&keyboard::Key::Character("a".into())), None);
    }
}
