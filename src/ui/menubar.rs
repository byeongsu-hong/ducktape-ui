//! Controlled menubar with top-level roving focus and shared child menus.

use std::rc::Rc;

use super::direction::Direction;
use super::focus_control::{self, FocusControl, Status};
use super::menu::{
    MENU_PANEL_PADDING, MenuEntry, MenuEvent, MenuState, TopLevelMove, focus_menu_state, menu,
};
use super::popover::{
    Alignment, FloatingConfig, FloatingContent, FocusFlag, PanelKind, Placement, panel,
};
use super::theme::{Theme, alpha, mix};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::widget::text::LineHeight;
use iced::widget::{Row, container, text};
use iced::{
    Alignment as IcedAlignment, Background, Border, Element, Event, Length, Padding, Pixels,
    Rectangle, Size, Task, Vector, touch,
};

pub const MENUBAR_HEIGHT: f32 = 36.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenubarMenu {
    pub id: String,
    pub label: String,
    pub entries: Vec<MenuEntry>,
    pub disabled: bool,
}

impl MenubarMenu {
    pub fn new(id: impl Into<String>, label: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            entries,
            disabled: false,
        }
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenubarState {
    pub focused: Option<usize>,
    pub open: Option<usize>,
}

impl MenubarState {
    pub fn initial(menus: &[MenubarMenu]) -> Self {
        Self {
            focused: menus.iter().position(|menu| !menu.disabled),
            open: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenubarCommand {
    Left,
    Right,
    First,
    Last,
    Open,
    Close,
}

pub fn reduce_menubar(
    state: &MenubarState,
    enabled: &[bool],
    command: MenubarCommand,
    direction: Direction,
) -> MenubarState {
    let mut next = state.clone();
    if !enabled.iter().any(|enabled| *enabled) {
        next.focused = None;
        next.open = None;
        return next;
    }
    let current = state
        .focused
        .filter(|index| *index < enabled.len() && enabled[*index]);
    let physical_next = match (command, direction) {
        (MenubarCommand::Right, Direction::LeftToRight)
        | (MenubarCommand::Left, Direction::RightToLeft) => Some(true),
        (MenubarCommand::Left, Direction::LeftToRight)
        | (MenubarCommand::Right, Direction::RightToLeft) => Some(false),
        _ => None,
    };
    let target = match command {
        MenubarCommand::First => enabled.iter().position(|enabled| *enabled),
        MenubarCommand::Last => enabled.iter().rposition(|enabled| *enabled),
        MenubarCommand::Left | MenubarCommand::Right => {
            let forward = physical_next.expect("left/right always have a direction");
            let start = current.unwrap_or_else(|| if forward { enabled.len() - 1 } else { 0 });
            (1..=enabled.len())
                .map(|distance| {
                    if forward {
                        (start + distance) % enabled.len()
                    } else {
                        (start + enabled.len() - distance % enabled.len()) % enabled.len()
                    }
                })
                .find(|index| enabled[*index])
        }
        MenubarCommand::Open => current,
        MenubarCommand::Close => {
            next.open = None;
            return next;
        }
    };
    if let Some(target) = target {
        next.focused = Some(target);
        if state.open.is_some() || command == MenubarCommand::Open {
            next.open = Some(target);
        }
    }
    next
}

pub fn menubar_command(key: &keyboard::Key) -> Option<MenubarCommand> {
    match key {
        keyboard::Key::Named(Named::ArrowLeft) => Some(MenubarCommand::Left),
        keyboard::Key::Named(Named::ArrowRight) => Some(MenubarCommand::Right),
        keyboard::Key::Named(Named::Home) => Some(MenubarCommand::First),
        keyboard::Key::Named(Named::End) => Some(MenubarCommand::Last),
        keyboard::Key::Named(Named::ArrowDown) => Some(MenubarCommand::Open),
        keyboard::Key::Named(Named::Escape) => Some(MenubarCommand::Close),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenubarEvent {
    StateChanged(MenubarState),
    Menu { menu_id: String, event: MenuEvent },
}

impl MenubarEvent {
    pub fn state(&self, current: &MenubarState) -> MenubarState {
        match self {
            Self::StateChanged(state) => state.clone(),
            Self::Menu {
                event: MenuEvent::Activated(_) | MenuEvent::Dismiss,
                ..
            } => MenubarState {
                open: None,
                ..current.clone()
            },
            Self::Menu { .. } => current.clone(),
        }
    }

    pub fn focus_task<Message>(
        &self,
        id: &str,
        menus: &[MenubarMenu],
        menu_state: &MenuState,
    ) -> Task<Message> {
        match self {
            Self::StateChanged(state) => {
                if let Some(index) = state.open
                    && let Some(menu) = menus.get(index)
                {
                    return focus_menu_state(
                        &format!("menubar:{id}:{}", menu.id),
                        &menu.entries,
                        menu_state,
                    );
                }
                state.focused.map_or_else(Task::none, |index| {
                    iced::widget::operation::focus(menubar_trigger_id(id, index))
                })
            }
            Self::Menu { menu_id, event } => menus
                .iter()
                .enumerate()
                .find(|(_, menu)| &menu.id == menu_id)
                .map_or_else(Task::none, |(index, menu)| match event {
                    MenuEvent::StateChanged(state) => {
                        focus_menu_state(&format!("menubar:{id}:{}", menu.id), &menu.entries, state)
                    }
                    MenuEvent::Activated(_) | MenuEvent::Dismiss => {
                        iced::widget::operation::focus(menubar_trigger_id(id, index))
                    }
                    MenuEvent::MoveTopLevel(_) => Task::none(),
                }),
        }
    }
}

pub struct Menubar<'a, Message>
where
    Message: Clone + 'a,
{
    id: String,
    menus: Vec<MenubarMenu>,
    state: MenubarState,
    menu_state: MenuState,
    on_event: Rc<dyn Fn(MenubarEvent) -> Message + 'a>,
    direction: Direction,
    menu_width: f32,
    disabled: bool,
    theme: Theme,
}

pub fn menubar<'a, Message>(
    id: impl Into<String>,
    menus: impl IntoIterator<Item = MenubarMenu>,
    state: &MenubarState,
    menu_state: &MenuState,
    on_event: impl Fn(MenubarEvent) -> Message + 'a,
    theme: &Theme,
) -> Menubar<'a, Message>
where
    Message: Clone + 'a,
{
    Menubar {
        id: id.into(),
        menus: menus.into_iter().collect(),
        state: state.clone(),
        menu_state: menu_state.clone(),
        on_event: Rc::new(on_event),
        direction: Direction::LeftToRight,
        menu_width: 224.0,
        disabled: false,
        theme: *theme,
    }
}

impl<Message> Menubar<'_, Message>
where
    Message: Clone,
{
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn menu_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.menu_width = width;
        }
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, Message> Menubar<'a, Message>
where
    Message: Clone + 'a,
{
    fn into_widget(self) -> MenubarWidget<'a, Message> {
        let enabled: Rc<[bool]> = self
            .menus
            .iter()
            .map(|menu| !self.disabled && !menu.disabled)
            .collect();
        let tab_stop = self
            .state
            .focused
            .filter(|index| enabled.get(*index) == Some(&true))
            .or_else(|| enabled.iter().position(|enabled| *enabled));
        let triggers = self
            .menus
            .iter()
            .enumerate()
            .map(|(index, menu)| {
                let opened = self.state.open == Some(index);
                let mut open_state = self.state.clone();
                open_state.focused = Some(index);
                open_state.open = Some(index);
                let activate = (self.on_event)(MenubarEvent::StateChanged(open_state));
                let key_state = self.state.clone();
                let key_enabled = Rc::clone(&enabled);
                let key_event = Rc::clone(&self.on_event);
                let direction = self.direction;
                let trigger_theme = self.theme;
                let content = container(
                    text(menu.label.clone())
                        .size(self.theme.typography.sm)
                        .line_height(LineHeight::Absolute(Pixels(16.0))),
                )
                .height(MENUBAR_HEIGHT - 4.0)
                .padding([0.0, 12.0])
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);
                Element::from(
                    FocusControl::new(
                        menubar_trigger_id(&self.id, index),
                        content,
                        activate,
                        &self.theme,
                    )
                    .disabled(!enabled[index])
                    .tab_stop(tab_stop == Some(index))
                    .on_key_press(move |key, _modifiers| {
                        let command = menubar_command(&key)?;
                        Some(key_event(MenubarEvent::StateChanged(reduce_menubar(
                            &key_state,
                            &key_enabled,
                            command,
                            direction,
                        ))))
                    })
                    .style(move |_iced_theme, status| {
                        menubar_trigger_style(&trigger_theme, opened, status)
                    }),
                )
            })
            .collect::<Vec<_>>();
        let row: Element<'a, Message> = container(
            Row::with_children(match self.direction {
                Direction::LeftToRight => triggers,
                Direction::RightToLeft => triggers.into_iter().rev().collect(),
            })
            .spacing(2)
            .align_y(IcedAlignment::Center),
        )
        .padding(2)
        .height(MENUBAR_HEIGHT)
        .style({
            let theme = self.theme;
            move |_iced_theme| menubar_style(&theme)
        })
        .into();

        let open_index = self
            .state
            .open
            .filter(|index| enabled.get(*index) == Some(&true));
        let content: Element<'a, Message> = if let Some(index) = open_index {
            let current = &self.menus[index];
            let menu_id = current.id.clone();
            let child_event = Rc::clone(&self.on_event);
            let state = self.state.clone();
            let child_enabled = Rc::clone(&enabled);
            let direction = self.direction;
            menu(
                format!("menubar:{}:{}", self.id, current.id),
                &current.entries,
                &self.menu_state,
                move |event| match event {
                    MenuEvent::MoveTopLevel(movement) => {
                        let command = match (movement, direction) {
                            (TopLevelMove::Previous, Direction::LeftToRight)
                            | (TopLevelMove::Next, Direction::RightToLeft) => MenubarCommand::Left,
                            (TopLevelMove::Next, Direction::LeftToRight)
                            | (TopLevelMove::Previous, Direction::RightToLeft) => {
                                MenubarCommand::Right
                            }
                        };
                        child_event(MenubarEvent::StateChanged(reduce_menubar(
                            &state,
                            &child_enabled,
                            command,
                            direction,
                        )))
                    }
                    MenuEvent::Dismiss => {
                        let mut state = state.clone();
                        state.open = None;
                        child_event(MenubarEvent::StateChanged(state))
                    }
                    event => child_event(MenubarEvent::Menu {
                        menu_id: menu_id.clone(),
                        event,
                    }),
                },
                &self.theme,
            )
            .direction(self.direction)
            .into()
        } else {
            Row::new().into()
        };
        let content = panel(
            content,
            PanelKind::Popover,
            Some(self.menu_width),
            self.menu_width,
            Padding::new(MENU_PANEL_PADDING),
            &self.theme,
        );

        MenubarWidget {
            id: self.id,
            bar: row,
            content,
            state: self.state,
            open_index,
            logical_to_visual: logical_to_visual_map(self.menus.len(), self.direction),
            on_event: self.on_event,
            config: menubar_floating_config(self.direction),
        }
    }
}

impl<'a, Message> From<Menubar<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(menubar: Menubar<'a, Message>) -> Self {
        Element::new(menubar.into_widget())
    }
}

fn logical_to_visual_map(count: usize, direction: Direction) -> Vec<usize> {
    match direction {
        Direction::LeftToRight => (0..count).collect(),
        Direction::RightToLeft => (0..count).rev().collect(),
    }
}

pub fn menubar_trigger_id(id: &str, index: usize) -> widget::Id {
    widget::Id::from(format!("ducktape-menubar:{id}:trigger:{index}"))
}

pub fn menubar_floating_config(direction: Direction) -> FloatingConfig {
    FloatingConfig {
        placement: Placement::Bottom,
        alignment: match direction {
            Direction::LeftToRight => Alignment::Start,
            Direction::RightToLeft => Alignment::End,
        },
        side_offset: 4.0,
        alignment_offset: 0.0,
        viewport_padding: 8.0,
        max_width: 360.0,
    }
}

pub fn menubar_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.background)),
        border: Border {
            color: theme.palette.border,
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        ..Default::default()
    }
}

pub fn menubar_trigger_style(theme: &Theme, opened: bool, status: Status) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match status {
        Status::Hovered | Status::Focused => Some(Background::Color(theme.palette.accent)),
        Status::Pressed => Some(Background::Color(mix(
            theme.palette.accent,
            theme.palette.foreground,
            0.08,
        ))),
        _ if opened => Some(Background::Color(theme.palette.accent)),
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
    style.focus_offset = 0.0;
    style
}

struct MenubarWidget<'a, Message> {
    id: String,
    bar: Element<'a, Message>,
    content: Element<'a, Message>,
    state: MenubarState,
    open_index: Option<usize>,
    logical_to_visual: Vec<usize>,
    on_event: Rc<dyn Fn(MenubarEvent) -> Message + 'a>,
    config: FloatingConfig,
}

#[derive(Debug, Default)]
struct State {
    content_focus: FocusFlag,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for MenubarWidget<'_, Message>
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
        let row_layout = layout.children().next()?;
        let trigger_layout = row_layout.children().nth(visual)?;
        let anchor = translated_bounds(trigger_layout.bounds(), translation);
        let triggers = row_layout
            .children()
            .map(|layout| translated_bounds(layout.bounds(), translation))
            .collect();
        let state = tree.state.downcast_mut::<State>();
        let content_tree = tree.children.get_mut(1)?;
        Some(overlay::Element::new(Box::new(MenubarOverlay {
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
            content_focus: &mut state.content_focus,
            content_id: widget::Id::from(format!("ducktape-menubar:{}:content", self.id)),
            on_event: self.on_event.as_ref(),
        })))
    }
}

fn translated_bounds(bounds: Rectangle, translation: Vector) -> Rectangle {
    Rectangle::new(bounds.position() + translation, bounds.size())
}

struct MenubarOverlay<'a, 'b, Message> {
    floating: FloatingContent<'a, 'b, Message>,
    triggers: Vec<Rectangle>,
    logical_to_visual: &'b [usize],
    state: &'b MenubarState,
    content_focus: &'b mut FocusFlag,
    content_id: widget::Id,
    on_event: &'b dyn Fn(MenubarEvent) -> Message,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for MenubarOverlay<'_, '_, Message>
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
        if matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape),
                ..
            })
        ) {
            let mut state = self.state.clone();
            state.open = None;
            shell.publish((self.on_event)(MenubarEvent::StateChanged(state)));
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
            if let Some(visual) = self
                .triggers
                .iter()
                .position(|bounds| bounds.contains(point))
                && let Some(logical) = self
                    .logical_to_visual
                    .iter()
                    .position(|mapped| *mapped == visual)
            {
                let mut state = self.state.clone();
                state.focused = Some(logical);
                state.open = (state.open != Some(logical)).then_some(logical);
                shell.publish((self.on_event)(MenubarEvent::StateChanged(state)));
            } else {
                let mut state = self.state.clone();
                state.open = None;
                shell.publish((self.on_event)(MenubarEvent::StateChanged(state)));
            }
            shell.capture_event();
            return;
        }
        self.floating
            .update(event, layout, cursor, renderer, clipboard, shell);
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
    use super::super::menu::{MenuActivation, MenuActivationKind, MenuItem};
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn top_level_navigation_wraps_skips_disabled_and_keeps_open() {
        let state = MenubarState {
            focused: Some(0),
            open: Some(0),
        };
        let enabled = [true, false, true];
        let next = reduce_menubar(
            &state,
            &enabled,
            MenubarCommand::Right,
            Direction::LeftToRight,
        );
        assert_eq!(next.focused, Some(2));
        assert_eq!(next.open, Some(2));
        let wrapped = reduce_menubar(
            &next,
            &enabled,
            MenubarCommand::Right,
            Direction::LeftToRight,
        );
        assert_eq!(wrapped.focused, Some(0));
        assert_eq!(wrapped.open, Some(0));
    }

    #[test]
    fn rtl_reverses_physical_left_and_right() {
        let state = MenubarState {
            focused: Some(0),
            open: None,
        };
        let enabled = [true, true, true];
        assert_eq!(
            reduce_menubar(
                &state,
                &enabled,
                MenubarCommand::Right,
                Direction::RightToLeft,
            )
            .focused,
            Some(2)
        );
        assert_eq!(
            reduce_menubar(
                &state,
                &enabled,
                MenubarCommand::Left,
                Direction::RightToLeft,
            )
            .focused,
            Some(1)
        );
    }

    #[test]
    fn exact_bar_geometry_and_styles_hold_in_both_themes() {
        assert_eq!(MENUBAR_HEIGHT, 36.0);
        for theme in [LIGHT, DARK] {
            let bar = menubar_style(&theme);
            let open = menubar_trigger_style(&theme, true, Status::Active);
            assert_eq!(bar.border.width, 1.0);
            assert_eq!(
                open.background,
                Some(Background::Color(theme.palette.accent))
            );
        }
    }

    #[test]
    fn menubar_builds_bar_and_floating_content_trees() {
        let menus = vec![
            MenubarMenu::new("file", "File", vec![MenuItem::new("new", "New").into()]),
            MenubarMenu::new("edit", "Edit", vec![MenuItem::new("copy", "Copy").into()]),
        ];
        let state = MenubarState::initial(&menus);
        let menu_state = MenuState::initial(&menus[0].entries);
        let element: Element<'_, ()> =
            menubar("app", menus, &state, &menu_state, |_| (), &LIGHT).into();
        assert_eq!(element.as_widget().children().len(), 2);
        assert_eq!(focusable_count(element), 1);
    }

    #[test]
    fn child_events_focus_items_and_restore_their_trigger() {
        let menus = [MenubarMenu::new(
            "view",
            "View",
            vec![MenuEntry::item("zoom", "Zoom")],
        )];
        let state = MenuState {
            focused: Some(vec![0]),
            ..MenuState::default()
        };
        let changed = MenubarEvent::Menu {
            menu_id: "view".into(),
            event: MenuEvent::StateChanged(state),
        };
        let activated = MenubarEvent::Menu {
            menu_id: "view".into(),
            event: MenuEvent::Activated(MenuActivation {
                id: "zoom".into(),
                path: vec![0],
                kind: MenuActivationKind::Action,
            }),
        };

        assert_eq!(
            changed
                .focus_task::<()>("app", &menus, &MenuState::default())
                .units(),
            1
        );
        assert_eq!(
            activated
                .focus_task::<()>("app", &menus, &MenuState::default())
                .units(),
            1
        );
    }
}
