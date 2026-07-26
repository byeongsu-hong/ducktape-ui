//! Keyboard focus and activation for content.
//!
//! This wrapper owns pointer, touch, Enter, and Space activation. Interactive
//! children may keep their native pointer behavior; captured events focus the
//! wrapper without activating it a second time. Passive regions keep the same
//! focus ring and key routing without adding activation behavior.

use super::theme::Theme as UiTheme;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::keyboard::{self, key};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Shadow, Size, Vector, touch,
    window,
};

/// The interaction state supplied to [`FocusControl::style`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Active,
    Hovered,
    Focused,
    Pressed,
    Disabled,
}

/// Visuals drawn around the wrapped content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub background: Option<Background>,
    pub text_color: Option<Color>,
    pub border: Border,
    pub shadow: Shadow,
    pub focus_ring: Border,
    pub focus_offset: f32,
}

/// The default transparent shell and semantic focus ring.
pub fn style(theme: &UiTheme, _status: Status) -> Style {
    Style {
        background: None,
        text_color: None,
        border: Border::default(),
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: 2.0,
            radius: (theme.radius.md + 4.0).into(),
        },
        focus_offset: 2.0,
    }
}

type StyleFn<'a, Theme> = dyn Fn(&Theme, Status) -> Style + 'a;
type KeyPressFn<'a, Message> = dyn Fn(keyboard::Key, keyboard::Modifiers) -> Option<Message> + 'a;

/// Persistent focus and press state exposed through iced widget operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct State {
    focused: bool,
    press: Option<Press>,
}

impl State {
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn is_pressed(&self) -> bool {
        self.press.is_some()
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn unfocus(&mut self) {
        self.focused = false;
        self.press = None;
    }
}

impl widget::operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.is_focused()
    }

    fn focus(&mut self) {
        self.focus();
    }

    fn unfocus(&mut self) {
        self.unfocus();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    Mouse,
    Touch(touch::Finger),
    Enter,
    Space,
}

/// A focusable shell for controls and passive keyboard regions.
///
/// The caller must route Tab and Shift+Tab to iced's
/// `iced::widget::operation::focus_next` and `focus_previous` tasks. A stable
/// [`widget::Id`] lets callers focus or query this control with the matching
/// iced widget operations.
pub struct FocusControl<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: renderer::Renderer,
{
    id: widget::Id,
    targetable: bool,
    tab_stop: bool,
    content: Element<'a, Message, Theme, Renderer>,
    on_activate: Option<Message>,
    on_key_press: Option<Box<KeyPressFn<'a, Message>>>,
    on_scroll_intent: Option<Message>,
    repeat_key_presses: bool,
    disabled: bool,
    style: Box<StyleFn<'a, Theme>>,
}

/// Wraps content in a keyboard- and pointer-activatable control.
pub fn focus_control<'a, Message>(
    id: widget::Id,
    content: impl Into<Element<'a, Message>>,
    on_activate: Message,
    theme: &UiTheme,
) -> FocusControl<'a, Message>
where
    Message: Clone + 'a,
{
    FocusControl::new(id, content, on_activate, theme)
}

impl<'a, Message, Theme, Renderer> FocusControl<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    pub fn new(
        id: widget::Id,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_activate: Message,
        theme: &UiTheme,
    ) -> Self {
        let theme = *theme;

        Self {
            id,
            targetable: true,
            tab_stop: true,
            content: content.into(),
            on_activate: Some(on_activate),
            on_key_press: None,
            on_scroll_intent: None,
            repeat_key_presses: false,
            disabled: false,
            style: Box::new(move |_iced_theme, status| style(&theme, status)),
        }
    }

    /// Creates a focusable region without pointer or Enter/Space activation.
    pub fn passive(
        id: widget::Id,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        theme: &UiTheme,
    ) -> Self {
        let theme = *theme;

        Self {
            id,
            targetable: true,
            tab_stop: true,
            content: content.into(),
            on_activate: None,
            on_key_press: None,
            on_scroll_intent: None,
            repeat_key_presses: false,
            disabled: false,
            style: Box::new(move |_iced_theme, status| style(&theme, status)),
        }
    }

    pub(crate) fn anonymous(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_activate: Message,
        theme: &UiTheme,
    ) -> Self {
        let mut control = Self::new(widget::Id::unique(), content, on_activate, theme);
        control.targetable = false;
        control
    }

    pub fn id(&self) -> &widget::Id {
        &self.id
    }

    /// Includes this control in sequential Tab focus traversal.
    ///
    /// Compound widgets should enable this for only their current entry item.
    #[must_use]
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Publishes a caller-selected message for additional focused key presses.
    ///
    /// Use this for compound-control navigation such as arrow keys, Home, and
    /// End. If the callback returns a message, the key event is captured and
    /// normal Enter/Space activation does not run for that press.
    #[must_use]
    pub fn on_key_press(
        mut self,
        handler: impl Fn(keyboard::Key, keyboard::Modifiers) -> Option<Message> + 'a,
    ) -> Self {
        self.on_key_press = Some(Box::new(handler));
        self
    }

    /// Allows held keys to repeat through [`Self::on_key_press`].
    #[must_use]
    pub fn repeat_key_presses(mut self, repeat: bool) -> Self {
        self.repeat_key_presses = repeat;
        self
    }

    /// Publishes a message when a passive region receives native scroll input.
    #[must_use]
    pub fn on_scroll_intent(mut self, message: Message) -> Self {
        self.on_scroll_intent = Some(message);
        self
    }

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self {
        self.style = Box::new(style);
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for FocusControl<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        if self.disabled || !self.tab_stop {
            tree.state.downcast_mut::<State>().unfocus();
        }
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
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
        {
            let state = tree.state.downcast_mut::<State>();

            if self.disabled || !self.tab_stop {
                state.unfocus();
            }

            if !self.disabled && self.tab_stop {
                operation.focusable(self.targetable.then_some(&self.id), layout.bounds(), state);
            }
        }

        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let cursor = pointer_cursor(event, cursor);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let is_over = cursor.is_over(layout.bounds());
        // Native scrolling captures presses without a cursor interaction;
        // interactive descendants advertise one and should own focus.
        let child_captured = shell.is_event_captured();
        let child_captured_interactive_press = child_captured
            && self.on_activate.is_none()
            && is_pointer_press(event)
            && is_over
            && self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            ) != mouse::Interaction::None;
        let native_scroll_press = child_captured
            && self.on_activate.is_none()
            && matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)))
            && is_over
            && !child_captured_interactive_press;
        if !self.disabled
            && (native_scroll_press || is_over && is_scroll_gesture(event))
            && let Some(message) = self.on_scroll_intent.as_ref()
        {
            shell.publish(message.clone());
        }
        let state = tree.state.downcast_mut::<State>();

        if child_captured {
            if is_pointer_press(event) {
                if self.on_activate.is_none() {
                    if is_over && !self.disabled && !child_captured_interactive_press {
                        state.focus();
                    } else {
                        state.unfocus();
                    }
                    shell.request_redraw();
                } else if is_over && !self.disabled {
                    state.focus();
                    shell.request_redraw();
                } else {
                    state.unfocus();
                }
            }

            return;
        }

        if handle_key_press(
            state,
            event,
            !self.disabled,
            self.repeat_key_presses,
            self.on_key_press.as_deref(),
            shell,
        ) {
            return;
        }

        if let Some(on_activate) = self.on_activate.as_ref() {
            handle_event(state, event, is_over, !self.disabled, on_activate, shell);
        } else {
            handle_passive_event(state, event, is_over, !self.disabled, shell);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let interaction = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );

        if interaction == mouse::Interaction::None
            && self.on_activate.is_some()
            && !self.disabled
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Pointer
        } else {
            interaction
        }
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
        let state = tree.state.downcast_ref::<State>();
        let status = status(state, self.disabled, cursor.is_over(layout.bounds()));
        let style = (self.style)(theme, status);

        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: style.border,
                    shadow: style.shadow,
                    ..renderer::Quad::default()
                },
                style
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style.text_color.unwrap_or(renderer_style.text_color),
            },
            layout,
            cursor,
            viewport,
        );

        if state.is_focused()
            && !self.disabled
            && style.focus_ring.width > 0.0
            && style.focus_ring.color.a > 0.0
        {
            let expansion = style.focus_offset.max(0.0) + style.focus_ring.width;

            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds().expand(expansion),
                    border: style.focus_ring,
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
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<FocusControl<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(control: FocusControl<'a, Message, Theme, Renderer>) -> Self {
        Element::new(control)
    }
}

fn status(state: &State, disabled: bool, hovered: bool) -> Status {
    if disabled {
        Status::Disabled
    } else if state.is_pressed()
        && state
            .press
            .is_some_and(|press| !matches!(press, Press::Mouse | Press::Touch(_)) || hovered)
    {
        Status::Pressed
    } else if hovered {
        Status::Hovered
    } else if state.is_focused() {
        Status::Focused
    } else {
        Status::Active
    }
}

fn is_pointer_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

fn is_scroll_gesture(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::WheelScrolled { .. })
            | Event::Touch(touch::Event::FingerMoved { .. })
    )
}

fn pointer_cursor(event: &Event, cursor: mouse::Cursor) -> mouse::Cursor {
    match event {
        Event::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. }
            | touch::Event::FingerLifted { position, .. },
        ) => mouse::Cursor::Available(*position),
        _ => cursor,
    }
}

fn activation_key(key: &keyboard::Key) -> Option<Press> {
    match key {
        keyboard::Key::Named(key::Named::Enter) => Some(Press::Enter),
        keyboard::Key::Named(key::Named::Space) => Some(Press::Space),
        _ => None,
    }
}

fn handle_event<Message: Clone>(
    state: &mut State,
    event: &Event,
    is_over: bool,
    enabled: bool,
    on_activate: &Message,
    shell: &mut Shell<'_, Message>,
) {
    if !enabled {
        state.unfocus();
        return;
    }

    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
            if is_over {
                state.focus();
                state.press = Some(Press::Mouse);
                shell.capture_event();
                shell.request_redraw();
            } else {
                state.unfocus();
            }
        }
        Event::Touch(touch::Event::FingerPressed { id, .. }) => {
            if is_over {
                state.focus();
                state.press = Some(Press::Touch(*id));
                shell.capture_event();
                shell.request_redraw();
            } else {
                state.unfocus();
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            if state.press == Some(Press::Mouse) {
                state.press = None;

                if is_over {
                    shell.publish(on_activate.clone());
                }

                shell.capture_event();
                shell.request_redraw();
            }
        }
        Event::Touch(touch::Event::FingerLifted { id, .. })
            if state.press == Some(Press::Touch(*id)) =>
        {
            state.press = None;

            if is_over {
                shell.publish(on_activate.clone());
            }

            shell.capture_event();
            shell.request_redraw();
        }
        Event::Touch(touch::Event::FingerLost { id, .. })
            if state.press == Some(Press::Touch(*id)) =>
        {
            state.press = None;
            shell.request_redraw();
        }
        Event::Window(window::Event::Unfocused) => {
            if state.is_focused() || state.is_pressed() {
                state.unfocus();
                shell.request_redraw();
            }
        }
        Event::Keyboard(keyboard::Event::KeyPressed { key, repeat, .. }) if state.is_focused() => {
            if let Some(press) = activation_key(key) {
                if !repeat && state.press.is_none() {
                    state.press = Some(press);
                    shell.request_redraw();
                }

                shell.capture_event();
            }
        }
        Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) if state.is_focused() => {
            if let Some(press) = activation_key(key)
                && state.press == Some(press)
            {
                state.press = None;
                shell.publish(on_activate.clone());
                shell.capture_event();
                shell.request_redraw();
            }
        }
        _ => {}
    }
}

fn handle_passive_event<Message>(
    state: &mut State,
    event: &Event,
    is_over: bool,
    enabled: bool,
    shell: &mut Shell<'_, Message>,
) {
    if !enabled {
        state.unfocus();
        return;
    }

    if is_pointer_press(event) {
        if is_over {
            state.focus();
            shell.request_redraw();
        } else {
            state.unfocus();
        }
    } else if matches!(event, Event::Window(window::Event::Unfocused)) {
        state.unfocus();
        shell.request_redraw();
    }
}

fn handle_key_press<Message>(
    state: &State,
    event: &Event,
    enabled: bool,
    allow_repeat: bool,
    handler: Option<&KeyPressFn<'_, Message>>,
    shell: &mut Shell<'_, Message>,
) -> bool {
    let Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        modifiers,
        repeat,
        ..
    }) = event
    else {
        return false;
    };

    if !enabled || !state.is_focused() || *repeat && !allow_repeat {
        return false;
    }

    let Some(message) = handler.and_then(|handler| handler(key.clone(), *modifiers)) else {
        return false;
    };

    shell.publish(message);
    shell.capture_event();
    true
}

#[cfg(test)]
pub(crate) fn focusable_count<Message>(mut element: Element<'_, Message>) -> usize {
    use iced::advanced::renderer::Headless as _;

    struct Count(usize);

    impl widget::Operation for Count {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            _state: &mut dyn widget::operation::Focusable,
        ) {
            self.0 += 1;
        }
    }

    let renderer = iced::futures::executor::block_on(iced::Renderer::new(
        iced::Font::default(),
        iced::Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let viewport = Size::new(1024.0, 1024.0);
    let mut tree = widget::Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport),
    );
    let mut operation = Count(0);
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut operation);
    operation.0
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;
    use iced::advanced::clipboard;
    use iced::event;
    use iced::keyboard::{Location, Modifiers};
    use iced::widget::{Space, button, column, scrollable};
    use iced::{Pixels, Point};

    fn passive_scroll_press(
        content: Element<'static, u8>,
        event: Event,
        cursor: mouse::Cursor,
        initially_focused: bool,
    ) -> (event::Status, bool, Vec<u8>) {
        use iced::advanced::renderer::Headless as _;

        let viewport = Rectangle::new(Point::ORIGIN, Size::new(120.0, 100.0));
        let scrollable = scrollable(content).width(Length::Fill).height(Length::Fill);
        let mut control: Element<'_, u8> =
            FocusControl::passive(widget::Id::new("passive-scroll-test"), scrollable, &LIGHT)
                .on_scroll_intent(9)
                .into();
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let mut tree = widget::Tree::new(control.as_widget());
        let node = control.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );

        if initially_focused {
            tree.state.downcast_mut::<State>().focus();
        }

        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        control.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        let status = shell.event_status();
        drop(shell);
        (
            status,
            tree.state.downcast_ref::<State>().is_focused(),
            messages,
        )
    }

    fn key_event(named: key::Named, pressed: bool) -> Event {
        let key = keyboard::Key::Named(named);
        let physical_key = key::Physical::Code(match named {
            key::Named::Enter => key::Code::Enter,
            key::Named::Space => key::Code::Space,
            _ => key::Code::Escape,
        });

        Event::Keyboard(if pressed {
            keyboard::Event::KeyPressed {
                key: key.clone(),
                modified_key: key,
                physical_key,
                location: Location::Standard,
                modifiers: Modifiers::default(),
                text: None,
                repeat: false,
            }
        } else {
            keyboard::Event::KeyReleased {
                key: key.clone(),
                modified_key: key,
                physical_key,
                location: Location::Standard,
                modifiers: Modifiers::default(),
            }
        })
    }

    #[test]
    fn enter_and_space_activate_only_after_focus() {
        for named in [key::Named::Enter, key::Named::Space] {
            let mut state = State::default();
            let mut messages = Vec::new();

            {
                let mut shell = Shell::new(&mut messages);
                handle_event(
                    &mut state,
                    &key_event(named, true),
                    false,
                    true,
                    &7,
                    &mut shell,
                );
                assert_eq!(shell.event_status(), event::Status::Ignored);
            }
            assert!(messages.is_empty());

            state.focus();
            {
                let mut shell = Shell::new(&mut messages);
                handle_event(
                    &mut state,
                    &key_event(named, true),
                    false,
                    true,
                    &7,
                    &mut shell,
                );
                assert!(state.is_pressed());
                assert_eq!(shell.event_status(), event::Status::Captured);
            }
            assert!(messages.is_empty());

            {
                let mut shell = Shell::new(&mut messages);
                handle_event(
                    &mut state,
                    &key_event(named, false),
                    false,
                    true,
                    &7,
                    &mut shell,
                );
                assert_eq!(shell.event_status(), event::Status::Captured);
            }
            assert_eq!(messages, [7]);
            assert!(!state.is_pressed());
        }
    }

    #[test]
    fn pointer_activates_on_release_inside_and_unfocuses_outside() {
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        let mut state = State::default();
        let mut messages = Vec::new();

        {
            let mut shell = Shell::new(&mut messages);
            handle_event(&mut state, &press, true, true, &9, &mut shell);
        }
        assert!(state.is_focused());
        assert!(state.is_pressed());

        {
            let mut shell = Shell::new(&mut messages);
            handle_event(&mut state, &release, true, true, &9, &mut shell);
        }
        assert_eq!(messages, [9]);

        {
            let mut shell = Shell::new(&mut messages);
            handle_event(&mut state, &press, false, true, &9, &mut shell);
        }
        assert!(!state.is_focused());
    }

    #[test]
    fn touch_release_uses_its_own_position_for_captured_children() {
        use iced::advanced::renderer::Headless as _;

        fn activate(renderer: &iced::Renderer, cursor: mouse::Cursor, release: Point) -> Vec<u8> {
            let viewport = Rectangle::with_size(Size::new(100.0, 40.0));
            let child = button("child")
                .on_press(1)
                .width(Length::Fill)
                .height(Length::Fill);
            let mut control: Element<'_, u8> =
                FocusControl::new(widget::Id::new("touch-release"), child, 2, &LIGHT).into();
            let mut tree = widget::Tree::new(control.as_widget());
            let node = control.as_widget_mut().layout(
                &mut tree,
                renderer,
                &layout::Limits::new(Size::ZERO, viewport.size()),
            );
            let finger = touch::Finger(1);
            let events = [
                Event::Touch(touch::Event::FingerPressed {
                    id: finger,
                    position: Point::new(20.0, 20.0),
                }),
                Event::Touch(touch::Event::FingerLifted {
                    id: finger,
                    position: release,
                }),
            ];
            let mut messages = Vec::new();

            for event in events {
                let mut clipboard = clipboard::Null;
                let mut shell = Shell::new(&mut messages);
                control.as_widget_mut().update(
                    &mut tree,
                    &event,
                    Layout::new(&node),
                    cursor,
                    renderer,
                    &mut clipboard,
                    &mut shell,
                    &viewport,
                );
                assert_eq!(shell.event_status(), event::Status::Captured);
            }

            assert!(!tree.state.downcast_ref::<State>().is_pressed());
            messages
        }

        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let inside = Point::new(20.0, 20.0);
        assert_eq!(activate(&renderer, mouse::Cursor::Unavailable, inside), [1]);
        assert!(
            activate(
                &renderer,
                mouse::Cursor::Available(inside),
                Point::new(120.0, 20.0),
            )
            .is_empty()
        );
    }

    #[test]
    fn passive_region_focuses_without_activating_or_capturing() {
        let mut state = State::default();
        let mut messages: Vec<()> = Vec::new();
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));

        {
            let mut shell = Shell::new(&mut messages);
            handle_passive_event(&mut state, &press, true, true, &mut shell);
            assert_eq!(shell.event_status(), event::Status::Ignored);
        }
        assert!(state.is_focused());
        assert!(messages.is_empty());

        let mut shell = Shell::new(&mut messages);
        handle_passive_event(
            &mut state,
            &Event::Window(window::Event::Unfocused),
            true,
            true,
            &mut shell,
        );
        assert!(!state.is_focused());
    }

    #[test]
    fn captured_native_scroll_presses_focus_passive_region() {
        let content = || {
            Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(400.0))
                .into()
        };

        let (status, focused, messages) = passive_scroll_press(
            content(),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(Point::new(119.0, 10.0)),
            false,
        );
        assert_eq!(status, event::Status::Captured);
        assert!(focused);
        assert_eq!(messages, [9]);

        let touch_position = Point::new(20.0, 60.0);
        let (status, focused, messages) = passive_scroll_press(
            content(),
            Event::Touch(touch::Event::FingerPressed {
                id: touch::Finger(1),
                position: touch_position,
            }),
            mouse::Cursor::Available(touch_position),
            false,
        );
        assert_eq!(status, event::Status::Captured);
        assert!(focused);
        assert!(messages.is_empty());

        let (_, _, messages) = passive_scroll_press(
            content(),
            Event::Touch(touch::Event::FingerMoved {
                id: touch::Finger(1),
                position: touch_position,
            }),
            mouse::Cursor::Available(touch_position),
            true,
        );
        assert_eq!(messages, [9]);
    }

    #[test]
    fn captured_interactive_child_press_releases_passive_focus() {
        let content = column![
            button("row action")
                .on_press(1)
                .width(Length::Fill)
                .height(Length::Fixed(40.0)),
            Space::new().height(Length::Fixed(400.0)),
        ]
        .width(Length::Fill)
        .into();
        let (status, focused, messages) = passive_scroll_press(
            content,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(Point::new(20.0, 20.0)),
            true,
        );

        assert_eq!(status, event::Status::Captured);
        assert!(!focused);
        assert!(messages.is_empty());
    }

    #[test]
    fn pointer_sources_must_match_and_window_blur_cancels_activation() {
        let finger = touch::Finger(1);
        let other = touch::Finger(2);
        let touch_press = Event::Touch(touch::Event::FingerPressed {
            id: finger,
            position: iced::Point::ORIGIN,
        });
        let other_lift = Event::Touch(touch::Event::FingerLifted {
            id: other,
            position: iced::Point::ORIGIN,
        });
        let mouse_release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        let mut state = State::default();
        let mut messages = Vec::new();

        for event in [touch_press, other_lift, mouse_release] {
            let mut shell = Shell::new(&mut messages);
            handle_event(&mut state, &event, true, true, &9, &mut shell);
        }
        assert!(state.is_pressed());
        assert!(messages.is_empty());

        let mut shell = Shell::new(&mut messages);
        handle_event(
            &mut state,
            &Event::Window(window::Event::Unfocused),
            true,
            true,
            &9,
            &mut shell,
        );
        assert!(!state.is_focused());
        assert!(!state.is_pressed());
        assert!(messages.is_empty());
    }

    #[test]
    fn disabled_controls_are_not_focused_or_activated() {
        let mut state = State::default();
        state.focus();
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        handle_event(
            &mut state,
            &key_event(key::Named::Enter, true),
            false,
            false,
            &1,
            &mut shell,
        );

        assert!(!state.is_focused());
        assert!(messages.is_empty());
    }

    #[test]
    fn status_preserves_focus_and_press_feedback() {
        let mut state = State::default();
        assert_eq!(status(&state, false, false), Status::Active);
        assert_eq!(status(&state, false, true), Status::Hovered);

        state.focus();
        assert_eq!(status(&state, false, false), Status::Focused);
        state.press = Some(Press::Space);
        assert_eq!(status(&state, false, false), Status::Pressed);
        assert_eq!(status(&state, true, true), Status::Disabled);
    }

    #[test]
    fn additional_key_binding_requires_focus_and_captures_a_match() {
        let handler = |key: keyboard::Key, _modifiers| {
            (key == keyboard::Key::Named(key::Named::ArrowRight)).then_some(11)
        };
        let mut state = State::default();
        let mut messages = Vec::new();

        {
            let mut shell = Shell::new(&mut messages);
            assert!(!handle_key_press(
                &state,
                &key_event(key::Named::ArrowRight, true),
                true,
                false,
                Some(&handler),
                &mut shell,
            ));
        }

        state.focus();
        {
            let mut shell = Shell::new(&mut messages);
            assert!(handle_key_press(
                &state,
                &key_event(key::Named::ArrowRight, true),
                true,
                false,
                Some(&handler),
                &mut shell,
            ));
            assert_eq!(shell.event_status(), event::Status::Captured);
        }
        assert_eq!(messages, [11]);
    }

    #[test]
    fn additional_key_binding_can_opt_into_repeats() {
        let handler = |_key: keyboard::Key, _modifiers| Some(7);
        let state = State {
            focused: true,
            ..State::default()
        };
        let mut event = key_event(key::Named::ArrowRight, true);
        if let Event::Keyboard(keyboard::Event::KeyPressed { repeat, .. }) = &mut event {
            *repeat = true;
        }
        let mut messages = Vec::new();

        let mut shell = Shell::new(&mut messages);
        assert!(handle_key_press(
            &state,
            &event,
            true,
            true,
            Some(&handler),
            &mut shell,
        ));
        assert_eq!(messages, [7]);
    }
}
