//! Root-level modal composition and focus containment for iced.
//!
//! Iced does not currently expose dialog accessibility roles. This module
//! implements the interaction contract without claiming semantics the runtime
//! cannot publish: controlled visibility, inert underlay, backdrop dismissal,
//! Escape, explicit initial/restore focus, and a wrapping focus order.

use super::theme::{Theme, alpha};
use iced::advanced::Renderer as _;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::keyboard::{self, key::Named};
use iced::{
    Alignment, Background, Color, Element, Event, Length, Point, Rectangle, Size, Task, Vector,
    touch,
};

const VIEWPORT_INSET: f32 = 16.0;

/// Why a modal requested dismissal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DismissReason {
    Backdrop,
    Escape,
}

/// Which casual dismissal gestures a modal accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DismissRules {
    pub backdrop: bool,
    pub escape: bool,
}

impl DismissRules {
    pub const DIALOG: Self = Self {
        backdrop: true,
        escape: true,
    };

    pub const ALERT_DIALOG: Self = Self {
        backdrop: false,
        escape: true,
    };

    pub const fn allows(self, reason: DismissReason) -> bool {
        match reason {
            DismissReason::Backdrop => self.backdrop,
            DismissReason::Escape => self.escape,
        }
    }
}

/// A dismissal or focus move emitted by [`modal`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalEvent {
    Dismiss(DismissReason),
    Focus(widget::Id),
}

impl ModalEvent {
    /// Completes focus events and otherwise does no work.
    pub fn focus_task<Message>(&self) -> Task<Message> {
        match self {
            Self::Focus(id) => iced::widget::operation::focus(id.clone()),
            Self::Dismiss(_) => Task::none(),
        }
    }
}

/// Stable modal focus order and the control restored after closing.
///
/// Keep this value in application state instead of recreating IDs in `view`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusScope {
    order: Vec<widget::Id>,
    restore: widget::Id,
}

impl FocusScope {
    /// Starts a non-empty focus scope.
    pub fn new(first: widget::Id, restore: widget::Id) -> Self {
        Self {
            order: vec![first],
            restore,
        }
    }

    /// Appends another focusable control in Tab order.
    #[must_use]
    pub fn push(mut self, id: widget::Id) -> Self {
        if !self.order.contains(&id) {
            self.order.push(id);
        }
        self
    }

    pub fn first(&self) -> &widget::Id {
        &self.order[0]
    }

    pub fn restore(&self) -> &widget::Id {
        &self.restore
    }

    pub fn order(&self) -> &[widget::Id] {
        &self.order
    }

    /// Focuses the first control after the caller changes `open` to `true`.
    pub fn open_task<Message>(&self) -> Task<Message> {
        iced::widget::operation::focus(self.first().clone())
    }

    /// Restores the opening control after the caller changes `open` to `false`.
    pub fn restore_task<Message>(&self) -> Task<Message> {
        iced::widget::operation::focus(self.restore.clone())
    }

    /// Returns the focus task matching a controlled visibility transition.
    pub fn transition_task<Message>(&self, was_open: bool, open: bool) -> Task<Message> {
        match (was_open, open) {
            (false, true) => self.open_task(),
            (true, false) => self.restore_task(),
            _ => Task::none(),
        }
    }
}

/// Places `content` above an inert underlay when `open` is true.
///
/// Put this widget at the root of the application view. Every focusable modal
/// control needs a stable ID listed in `focus`; undeclared controls are skipped
/// by Tab. Feed [`ModalEvent::focus_task`] back from `update` for focus events,
/// and use [`FocusScope::transition_task`] after changing controlled visibility.
pub fn modal<'a, Message>(
    underlay: impl Into<Element<'a, Message>>,
    open: bool,
    content: impl Into<Element<'a, Message>>,
    focus: &FocusScope,
    dismiss: DismissRules,
    on_event: impl Fn(ModalEvent) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let underlay = underlay.into();
    if !open {
        return underlay;
    }

    Element::new(Modal {
        underlay,
        content: content.into(),
        focus: focus.clone(),
        dismiss,
        on_event: Box::new(on_event),
        backdrop: backdrop_color(theme),
    })
}

/// Backdrop treatment tuned independently for light and dark canvases.
pub fn backdrop_color(theme: &Theme) -> Color {
    let background = theme.palette.background;
    let light = background.r + background.g + background.b > 1.5;
    alpha(Color::BLACK, if light { 0.52 } else { 0.68 })
}

struct Modal<'a, Message> {
    underlay: Element<'a, Message>,
    content: Element<'a, Message>,
    focus: FocusScope,
    dismiss: DismissRules,
    on_event: Box<dyn Fn(ModalEvent) -> Message + 'a>,
    backdrop: Color,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct State {
    focused: Option<usize>,
    backdrop_press: Option<BackdropPress>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackdropPress {
    Mouse,
    Touch(touch::Finger),
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Modal<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.underlay),
            widget::Tree::new(&self.content),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.underlay, &self.content]);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);
        let underlay =
            self.underlay
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &limits);
        let size = limits.resolve(Length::Fill, Length::Fill, underlay.size());
        let available = Size::new(
            (size.width - VIEWPORT_INSET * 2.0).max(0.0),
            (size.height - VIEWPORT_INSET * 2.0).max(0.0),
        );
        let content = self.content.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(Size::ZERO, available),
        );
        let content = centered_node(content, size);

        layout::Node::with_children(size, vec![underlay, content])
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let content_layout = layout.children().nth(1).expect("modal content layout");
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[1],
                content_layout,
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
        let content_layout = layout.children().nth(1).expect("modal content layout");
        let content_bounds = content_layout.bounds();
        let state = tree.state.downcast_mut::<State>();
        state.focused = focused_index(
            &mut self.content,
            &mut tree.children[1],
            content_layout,
            renderer,
            self.focus.order(),
        );

        if let Some(backwards) = tab_direction(event) {
            let index = next_focus(state.focused, self.focus.order().len(), backwards);
            shell.publish((self.on_event)(ModalEvent::Focus(
                self.focus.order()[index].clone(),
            )));
            shell.capture_event();
            return;
        }

        if is_escape(event) {
            if self.dismiss.escape {
                shell.publish((self.on_event)(ModalEvent::Dismiss(DismissReason::Escape)));
            }
            shell.capture_event();
            return;
        }

        self.content.as_widget_mut().update(
            &mut tree.children[1],
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        handle_backdrop(
            state,
            event,
            cursor,
            content_bounds,
            self.dismiss,
            &self.on_event,
            shell,
        );

        if matches!(event, Event::Keyboard(_)) {
            shell.capture_event();
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
        let content_layout = layout.children().nth(1).expect("modal content layout");
        if cursor.is_over(content_layout.bounds()) {
            self.content.as_widget().mouse_interaction(
                &tree.children[1],
                content_layout,
                cursor,
                viewport,
                renderer,
            )
        } else if self.dismiss.backdrop && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
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
        let mut children = layout.children();
        let underlay_layout = children.next().expect("modal underlay layout");
        let content_layout = children.next().expect("modal content layout");

        self.underlay.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            underlay_layout,
            mouse::Cursor::Unavailable,
            viewport,
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            Background::Color(self.backdrop),
        );
        self.content.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        let content_layout = layout.children().nth(1).expect("modal content layout");
        self.content.as_widget_mut().overlay(
            &mut tree.children[1],
            content_layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn centered_node(node: layout::Node, viewport: Size) -> layout::Node {
    node.align(Alignment::Center, Alignment::Center, viewport)
}

fn next_focus(current: Option<usize>, count: usize, backwards: bool) -> usize {
    debug_assert!(count > 0, "FocusScope is always non-empty");
    match (current.filter(|index| *index < count), backwards) {
        (Some(0), true) | (None, true) => count - 1,
        (Some(index), true) => index - 1,
        (Some(index), false) => (index + 1) % count,
        (None, false) => 0,
    }
}

fn focused_index<Message>(
    content: &mut Element<'_, Message>,
    tree: &mut widget::Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    ids: &[widget::Id],
) -> Option<usize> {
    struct FindFocused<'a> {
        ids: &'a [widget::Id],
        focused: Option<usize>,
    }

    impl widget::Operation for FindFocused<'_> {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            id: Option<&widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn widget::operation::Focusable,
        ) {
            if state.is_focused() {
                self.focused =
                    id.and_then(|id| self.ids.iter().position(|candidate| candidate == id));
            }
        }
    }

    let mut operation = FindFocused { ids, focused: None };
    content
        .as_widget_mut()
        .operate(tree, layout, renderer, &mut operation);
    operation.focused
}

fn tab_direction(event: &Event) -> Option<bool> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::Tab),
            modifiers,
            repeat: false,
            ..
        }) => Some(modifiers.shift()),
        _ => None,
    }
}

fn is_escape(event: &Event) -> bool {
    matches!(
        event,
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::Escape),
            repeat: false,
            ..
        })
    )
}

fn event_position(event: &Event, cursor: mouse::Cursor) -> Option<Point> {
    match event {
        Event::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. }
            | touch::Event::FingerLifted { position, .. }
            | touch::Event::FingerLost { position, .. },
        ) => Some(*position),
        _ => cursor.position(),
    }
}

fn handle_backdrop<Message>(
    state: &mut State,
    event: &Event,
    cursor: mouse::Cursor,
    content_bounds: Rectangle,
    rules: DismissRules,
    on_event: &dyn Fn(ModalEvent) -> Message,
    shell: &mut Shell<'_, Message>,
) {
    let outside =
        event_position(event, cursor).is_some_and(|point| !content_bounds.contains(point));

    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if outside => {
            state.backdrop_press = Some(BackdropPress::Mouse);
            shell.capture_event();
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            let clicked = state.backdrop_press.take() == Some(BackdropPress::Mouse) && outside;
            if clicked && rules.backdrop {
                shell.publish(on_event(ModalEvent::Dismiss(DismissReason::Backdrop)));
            }
            shell.capture_event();
        }
        Event::Touch(touch::Event::FingerPressed { id, .. }) if outside => {
            state.backdrop_press = Some(BackdropPress::Touch(*id));
            shell.capture_event();
        }
        Event::Touch(touch::Event::FingerLifted { id, .. }) => {
            let clicked = state.backdrop_press.take() == Some(BackdropPress::Touch(*id)) && outside;
            if clicked && rules.backdrop {
                shell.publish(on_event(ModalEvent::Dismiss(DismissReason::Backdrop)));
            }
            shell.capture_event();
        }
        Event::Touch(touch::Event::FingerLost { id, .. }) => {
            if state.backdrop_press == Some(BackdropPress::Touch(*id)) {
                state.backdrop_press = None;
            }
            shell.capture_event();
        }
        Event::Mouse(_) | Event::Touch(_) => shell.capture_event(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};
    use iced::widget::text;

    #[test]
    fn focus_navigation_wraps_and_recovers_from_unknown_focus() {
        assert_eq!(next_focus(Some(0), 3, true), 2);
        assert_eq!(next_focus(Some(2), 3, false), 0);
        assert_eq!(next_focus(Some(1), 3, false), 2);
        assert_eq!(next_focus(None, 3, false), 0);
        assert_eq!(next_focus(None, 3, true), 2);
    }

    #[test]
    fn focus_scope_deduplicates_ids_and_keeps_restore_separate() {
        let first = widget::Id::new("dialog-first");
        let second = widget::Id::new("dialog-second");
        let restore = widget::Id::new("dialog-trigger");
        let scope = FocusScope::new(first.clone(), restore.clone())
            .push(second.clone())
            .push(first.clone());

        assert_eq!(scope.order(), &[first, second]);
        assert_eq!(scope.restore(), &restore);
    }

    #[test]
    fn dialog_and_alert_dismissal_rules_differ_on_backdrop() {
        assert!(DismissRules::DIALOG.allows(DismissReason::Backdrop));
        assert!(DismissRules::DIALOG.allows(DismissReason::Escape));
        assert!(!DismissRules::ALERT_DIALOG.allows(DismissReason::Backdrop));
        assert!(DismissRules::ALERT_DIALOG.allows(DismissReason::Escape));
    }

    #[test]
    fn centered_geometry_preserves_content_size() {
        let node = centered_node(
            layout::Node::new(Size::new(320.0, 180.0)),
            Size::new(1000.0, 700.0),
        );
        assert_eq!(
            node.bounds(),
            Rectangle::new(Point::new(340.0, 260.0), Size::new(320.0, 180.0))
        );
    }

    #[test]
    fn only_open_modal_adds_the_two_layer_widget_tree() {
        let focus = FocusScope::new(widget::Id::new("first"), widget::Id::new("restore"));
        let closed: Element<'_, ()> = modal(
            text("page"),
            false,
            text("dialog"),
            &focus,
            DismissRules::DIALOG,
            |_| (),
            &LIGHT,
        );
        let open: Element<'_, ()> = modal(
            text("page"),
            true,
            text("dialog"),
            &focus,
            DismissRules::DIALOG,
            |_| (),
            &LIGHT,
        );

        assert!(closed.as_widget().children().is_empty());
        assert_eq!(open.as_widget().children().len(), 2);
    }

    #[test]
    fn dark_canvas_gets_the_stronger_backdrop() {
        assert_eq!(backdrop_color(&LIGHT).a, 0.52);
        assert_eq!(backdrop_color(&DARK).a, 0.68);
    }
}
