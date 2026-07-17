//! Context menu anchored to the exact mouse or touch press point.

use std::rc::Rc;

use super::direction::Direction;
use super::menu::{MENU_PANEL_PADDING, MenuEntry, MenuEvent, MenuState, focus_menu_state, menu};
use super::popover::{
    Alignment, DismissReason, FloatingConfig, FloatingContent, FocusFlag, PanelKind, Placement,
    draw_focus_ring, panel,
};
use super::theme::Theme;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::keyboard::{self, key::Named};
use iced::{Element, Event, Length, Padding, Point, Rectangle, Size, Task, Vector, touch};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuIds {
    region: widget::Id,
    content: widget::Id,
    pub menu: String,
}

impl ContextMenuIds {
    pub fn new(key: impl ToString) -> Self {
        let key = key.to_string();
        Self {
            region: widget::Id::from(format!("ducktape-context:{key}:region")),
            content: widget::Id::from(format!("ducktape-context:{key}:content")),
            menu: format!("context:{key}:menu"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextMenuEvent {
    OpenAt(Point),
    Close(DismissReason),
    Menu(MenuEvent),
}

impl ContextMenuEvent {
    pub const fn open(&self, current: bool) -> bool {
        match self {
            Self::OpenAt(_) => true,
            Self::Close(_) | Self::Menu(MenuEvent::Activated(_) | MenuEvent::Dismiss) => false,
            Self::Menu(_) => current,
        }
    }

    pub fn anchor(&self, current: Option<Point>) -> Option<Point> {
        match self {
            Self::OpenAt(point) => Some(*point),
            Self::Close(_) => current,
            Self::Menu(_) => current,
        }
    }

    pub fn focus_task<Message>(
        &self,
        ids: &ContextMenuIds,
        entries: &[MenuEntry],
        state: &MenuState,
    ) -> Task<Message> {
        match self {
            Self::OpenAt(_) => focus_menu_state(&ids.menu, entries, state),
            Self::Menu(MenuEvent::StateChanged(state)) => {
                focus_menu_state(&ids.menu, entries, state)
            }
            Self::Close(_) | Self::Menu(MenuEvent::Activated(_) | MenuEvent::Dismiss) => {
                iced::widget::operation::focus(ids.region.clone())
            }
            Self::Menu(MenuEvent::MoveTopLevel(_)) => Task::none(),
        }
    }
}

pub struct ContextMenu<'a, Message>
where
    Message: Clone + 'a,
{
    ids: ContextMenuIds,
    region: Element<'a, Message>,
    entries: Vec<MenuEntry>,
    state: MenuState,
    open: bool,
    anchor: Option<Point>,
    on_event: Rc<dyn Fn(ContextMenuEvent) -> Message + 'a>,
    width: f32,
    touch: bool,
    disabled: bool,
    direction: Direction,
    theme: Theme,
}

/// Apply emitted events to visibility, anchor, and [`MenuState`], then return
/// [`ContextMenuEvent::focus_task`] from `update`.
#[allow(clippy::too_many_arguments)]
pub fn context_menu<'a, Message>(
    ids: ContextMenuIds,
    region: impl Into<Element<'a, Message>>,
    entries: &'a [MenuEntry],
    state: &'a MenuState,
    open: bool,
    anchor: Option<Point>,
    on_event: impl Fn(ContextMenuEvent) -> Message + 'a,
    theme: &Theme,
) -> ContextMenu<'a, Message>
where
    Message: Clone + 'a,
{
    let on_event = Rc::new(on_event);
    ContextMenu {
        ids,
        region: region.into(),
        entries: entries.to_vec(),
        state: state.clone(),
        open,
        anchor,
        on_event,
        width: 224.0,
        touch: true,
        disabled: false,
        direction: Direction::LeftToRight,
        theme: *theme,
    }
}

impl<Message> ContextMenu<'_, Message>
where
    Message: Clone,
{
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
        self
    }

    /// Enables immediate touch-press opening at the finger position.
    #[must_use]
    pub fn touch(mut self, touch: bool) -> Self {
        self.touch = touch;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, Message> ContextMenu<'a, Message>
where
    Message: Clone + 'a,
{
    fn into_widget(self) -> ContextMenuWidget<'a, Message> {
        let menu_event = Rc::clone(&self.on_event);
        let content = menu(
            self.ids.menu.clone(),
            &self.entries,
            &self.state,
            move |event| menu_event(ContextMenuEvent::Menu(event)),
            &self.theme,
        )
        .direction(self.direction)
        .into();
        let content = panel(
            content,
            PanelKind::Popover,
            Some(self.width),
            self.width,
            Padding::new(MENU_PANEL_PADDING),
            &self.theme,
        );
        ContextMenuWidget {
            ids: self.ids,
            region: self.region,
            content,
            open: self.open,
            anchor: self.anchor,
            on_event: self.on_event,
            config: context_floating_config(self.direction),
            touch: self.touch,
            disabled: self.disabled,
            theme: self.theme,
        }
    }
}

impl<'a, Message> From<ContextMenu<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(menu: ContextMenu<'a, Message>) -> Self {
        Element::new(menu.into_widget())
    }
}

pub fn context_floating_config(direction: Direction) -> FloatingConfig {
    FloatingConfig {
        placement: Placement::Bottom,
        alignment: match direction {
            Direction::LeftToRight => Alignment::Start,
            Direction::RightToLeft => Alignment::End,
        },
        side_offset: 0.0,
        alignment_offset: 0.0,
        viewport_padding: 8.0,
        max_width: 360.0,
    }
}

pub fn point_anchor(point: Point) -> Rectangle {
    Rectangle::new(point, Size::ZERO)
}

struct ContextMenuWidget<'a, Message> {
    ids: ContextMenuIds,
    region: Element<'a, Message>,
    content: Element<'a, Message>,
    open: bool,
    anchor: Option<Point>,
    on_event: Rc<dyn Fn(ContextMenuEvent) -> Message + 'a>,
    config: FloatingConfig,
    touch: bool,
    disabled: bool,
    theme: Theme,
}

#[derive(Debug, Default)]
struct State {
    region_focus: FocusFlag,
    content_focus: FocusFlag,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for ContextMenuWidget<'_, Message>
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
            widget::Tree::new(&self.region),
            widget::Tree::new(&self.content),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.region.as_widget(), self.content.as_widget()]);
        let state = tree.state.downcast_mut::<State>();
        if self.open {
            state.region_focus.unfocus();
        } else {
            state.content_focus.unfocus();
        }
        if self.disabled {
            state.region_focus.unfocus();
        }
    }

    fn size(&self) -> Size<Length> {
        self.region.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.region.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.region
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
        let state = tree.state.downcast_mut::<State>();
        if self.disabled || self.open {
            state.region_focus.unfocus();
        } else {
            operation.focusable(
                Some(&self.ids.region),
                layout.bounds(),
                &mut state.region_focus,
            );
        }
        operation.traverse(&mut |operation| {
            self.region
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
        self.region.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if self.disabled || self.open || shell.is_event_captured() {
            return;
        }

        let point = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => cursor.position(),
            Event::Touch(touch::Event::FingerPressed { position, .. }) if self.touch => {
                Some(*position)
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::ContextMenu),
                ..
            }) if tree.state.downcast_ref::<State>().region_focus.is_focused() => {
                Some(layout.bounds().center())
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::F10),
                modifiers,
                ..
            }) if modifiers.shift()
                && tree.state.downcast_ref::<State>().region_focus.is_focused() =>
            {
                Some(layout.bounds().center())
            }
            _ => None,
        };
        if let Some(point) = point.filter(|point| layout.bounds().contains(*point)) {
            shell.publish((self.on_event)(ContextMenuEvent::OpenAt(point)));
            shell.capture_event();
            shell.request_redraw();
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
        self.region.as_widget().mouse_interaction(
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
        self.region.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        let state = tree.state.downcast_ref::<State>();
        draw_focus_ring(
            renderer,
            layout.bounds(),
            state.region_focus.is_focused() && !self.disabled,
            &self.theme,
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
        let anchor = self.anchor.filter(|_| self.open)?;
        let state = tree.state.downcast_mut::<State>();
        let content_tree = tree.children.get_mut(1)?;
        Some(overlay::Element::new(Box::new(ContextOverlay {
            floating: FloatingContent {
                content: &mut self.content,
                tree: content_tree,
                anchor: translated_bounds(point_anchor(anchor), translation),
                viewport: *viewport,
                config: self.config,
            },
            region: translated_bounds(layout.bounds(), translation),
            translation,
            content_focus: &mut state.content_focus,
            content_id: &self.ids.content,
            on_event: self.on_event.as_ref(),
        })))
    }
}

fn translated_bounds(bounds: Rectangle, translation: Vector) -> Rectangle {
    Rectangle::new(bounds.position() + translation, bounds.size())
}

struct ContextOverlay<'a, 'b, Message> {
    floating: FloatingContent<'a, 'b, Message>,
    region: Rectangle,
    translation: Vector,
    content_focus: &'b mut FocusFlag,
    content_id: &'b widget::Id,
    on_event: &'b dyn Fn(ContextMenuEvent) -> Message,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for ContextOverlay<'_, '_, Message>
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
            Some(self.content_id),
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
        let action = context_overlay_action(
            event,
            cursor,
            self.region,
            self.floating.bounds(layout),
            self.translation,
        );
        if let Some(action) = action {
            self.content_focus.unfocus();
            shell.publish((self.on_event)(action));
            shell.capture_event();
            shell.request_redraw();
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
        20.0
    }
}

fn context_overlay_action(
    event: &Event,
    cursor: mouse::Cursor,
    region: Rectangle,
    content: Rectangle,
    translation: Vector,
) -> Option<ContextMenuEvent> {
    if matches!(
        event,
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::Escape),
            ..
        })
    ) {
        return Some(ContextMenuEvent::Close(DismissReason::Escape));
    }

    let point = match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor.position(),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
        _ => None,
    }?;
    if content.contains(point) {
        return None;
    }
    if matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
    ) && region.contains(point)
    {
        Some(ContextMenuEvent::OpenAt(point - translation))
    } else {
        Some(ContextMenuEvent::Close(DismissReason::Outside))
    }
}

#[cfg(test)]
mod tests {
    use super::super::menu::MenuItem;
    use super::super::popover::resolve_position;
    use super::super::theme::LIGHT;
    use super::*;
    use iced::widget::text;

    #[test]
    fn open_event_preserves_the_exact_context_point() {
        let point = Point::new(137.25, 88.5);
        let event = ContextMenuEvent::OpenAt(point);
        assert_eq!(event.anchor(None), Some(point));
        assert_eq!(point_anchor(point), Rectangle::new(point, Size::ZERO));
    }

    #[test]
    fn point_placement_collides_inside_the_viewport_in_both_directions() {
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(300.0, 200.0));
        let anchor = point_anchor(Point::new(295.0, 195.0));
        for direction in [Direction::LeftToRight, Direction::RightToLeft] {
            let resolved = resolve_position(
                anchor,
                Size::new(160.0, 100.0),
                viewport,
                context_floating_config(direction),
            );
            assert!(resolved.bounds.x >= 8.0);
            assert!(resolved.bounds.y >= 8.0);
            assert!(resolved.bounds.x + resolved.bounds.width <= 292.0);
            assert!(resolved.bounds.y + resolved.bounds.height <= 192.0);
        }
    }

    #[test]
    fn controlled_context_menu_builds_region_and_content_trees() {
        let entries = vec![MenuItem::new("copy", "Copy").into()];
        let state = MenuState::initial(&entries);
        let element: Element<'_, ()> = context_menu(
            ContextMenuIds::new("document"),
            text("Right click"),
            &entries,
            &state,
            false,
            None,
            |_| (),
            &LIGHT,
        )
        .into();
        assert_eq!(element.as_widget().children().len(), 2);
    }

    #[test]
    fn right_click_reanchors_an_open_menu_inside_its_region() {
        let region = Rectangle::new(Point::ORIGIN, Size::new(300.0, 200.0));
        let content = Rectangle::new(Point::new(100.0, 50.0), Size::new(120.0, 100.0));
        let point = Point::new(40.0, 80.0);
        let event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));

        assert_eq!(
            context_overlay_action(
                &event,
                mouse::Cursor::Available(point),
                region,
                content,
                Vector::ZERO,
            ),
            Some(ContextMenuEvent::OpenAt(point))
        );
        assert_eq!(
            context_overlay_action(
                &event,
                mouse::Cursor::Available(Point::new(280.0, 220.0)),
                region,
                content,
                Vector::ZERO,
            ),
            Some(ContextMenuEvent::Close(DismissReason::Outside))
        );
    }

    #[test]
    fn translated_coordinates_reanchor_instead_of_dismiss() {
        let translation = Vector::new(300.0, 200.0);
        let region = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));
        let anchor = Point::new(20.0, 30.0);
        let rendered_anchor = translated_bounds(point_anchor(anchor), translation);
        let point = rendered_anchor.position();
        let event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));

        let action = context_overlay_action(
            &event,
            mouse::Cursor::Available(point),
            translated_bounds(region, translation),
            Rectangle::with_size(Size::ZERO),
            translation,
        );
        let Some(ContextMenuEvent::OpenAt(reanchored)) = action else {
            panic!("translated right click should re-anchor");
        };

        assert_eq!(reanchored, anchor);
        assert_eq!(
            translated_bounds(point_anchor(reanchored), translation),
            rendered_anchor
        );
    }
}
