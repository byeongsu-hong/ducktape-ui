//! Delayed pointer- and keyboard-triggered tooltips.
//!
//! By default the trigger is passive because this wrapper owns its focus stop.
//! Use [`Tooltip::focusable`] with `false` when a nested trigger already owns focus;
//! that mode observes the child's focus without adding a duplicate stop.
//! Tooltip content is deliberately noninteractive and never receives events.

use super::popover::{
    Alignment, FloatingConfig, FloatingContent, FocusFlag, PanelKind, Placement, draw_focus_ring,
    panel,
};
use super::theme::Theme as UiTheme;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::keyboard::{self, key};
use iced::time::{Duration, Instant};
use iced::widget::text::LineHeight;
use iced::widget::{Text, text};
use iced::{Element, Event, Length, Padding, Pixels, Rectangle, Size, Vector};

const DEFAULT_OPEN_DELAY: Duration = Duration::from_millis(700);
const DEFAULT_CLOSE_DELAY: Duration = Duration::ZERO;
const DEFAULT_MAX_WIDTH: f32 = 320.0;
const DEFAULT_TEXT_SIZE: f32 = 12.0;
const DEFAULT_LINE_HEIGHT: f32 = 16.0;

/// A baseline-stable tooltip label matching the default 12/16 type metrics.
pub fn tooltip_text<'a>(content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    text(content)
        .size(DEFAULT_TEXT_SIZE)
        .line_height(LineHeight::Absolute(Pixels(DEFAULT_LINE_HEIGHT)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TooltipId(widget::Id);

impl TooltipId {
    pub fn new(key: impl ToString) -> Self {
        Self(widget::Id::from(format!(
            "ducktape-tooltip:{}:trigger",
            key.to_string()
        )))
    }
}

pub struct Tooltip<'a, Message> {
    id: TooltipId,
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    config: FloatingConfig,
    open_delay: Duration,
    close_delay: Duration,
    padding: Padding,
    disabled: bool,
    focusable: bool,
    theme: UiTheme,
}

pub fn tooltip<'a, Message>(
    id: TooltipId,
    trigger: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    theme: &UiTheme,
) -> Tooltip<'a, Message> {
    Tooltip {
        id,
        trigger: trigger.into(),
        content: content.into(),
        config: FloatingConfig {
            placement: Placement::Top,
            max_width: DEFAULT_MAX_WIDTH,
            ..FloatingConfig::default()
        },
        open_delay: DEFAULT_OPEN_DELAY,
        close_delay: DEFAULT_CLOSE_DELAY,
        padding: Padding::new(6.0).horizontal(12.0),
        disabled: false,
        focusable: true,
        theme: *theme,
    }
}

impl<Message> Tooltip<'_, Message> {
    #[must_use]
    pub fn placement(mut self, placement: Placement) -> Self {
        self.config.placement = placement;
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.config.alignment = alignment;
        self
    }

    #[must_use]
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.config.side_offset = offset;
        self
    }

    #[must_use]
    pub fn alignment_offset(mut self, offset: f32) -> Self {
        self.config.alignment_offset = offset;
        self
    }

    #[must_use]
    pub fn viewport_padding(mut self, padding: f32) -> Self {
        self.config.viewport_padding = padding;
        self
    }

    #[must_use]
    pub fn max_width(mut self, width: f32) -> Self {
        self.config.max_width = width;
        self
    }

    #[must_use]
    pub fn open_delay(mut self, delay: Duration) -> Self {
        self.open_delay = delay;
        self
    }

    #[must_use]
    pub fn close_delay(mut self, delay: Duration) -> Self {
        self.close_delay = delay;
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Disables the wrapper's focus stop when the trigger already owns one.
    #[must_use]
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }
}

impl<'a, Message> Tooltip<'a, Message>
where
    Message: 'a,
{
    fn into_widget(self) -> TooltipWidget<'a, Message> {
        let config = self.config.sanitized();
        TooltipWidget {
            id: self.id,
            trigger: self.trigger,
            content: panel(
                self.content,
                PanelKind::Tooltip,
                None,
                config.max_width,
                self.padding,
                &self.theme,
            ),
            config,
            open_delay: self.open_delay,
            close_delay: self.close_delay,
            disabled: self.disabled,
            focusable: self.focusable,
            theme: self.theme,
        }
    }
}

impl<'a, Message> From<Tooltip<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(tooltip: Tooltip<'a, Message>) -> Self {
        Element::new(tooltip.into_widget())
    }
}

struct TooltipWidget<'a, Message> {
    id: TooltipId,
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    config: FloatingConfig,
    open_delay: Duration,
    close_delay: Duration,
    disabled: bool,
    focusable: bool,
    theme: UiTheme,
}

#[derive(Debug, Default)]
struct State {
    focus: FocusFlag,
    trigger_focused: bool,
    hovered: bool,
    presence: DelayedPresence,
}

impl State {
    fn active(&self) -> bool {
        self.hovered || self.focus.is_focused() || self.trigger_focused
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for TooltipWidget<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.trigger),
            widget::Tree::new(&self.content),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.trigger.as_widget(), self.content.as_widget()]);
        if self.disabled {
            let state = tree.state.downcast_mut::<State>();
            state.focus.unfocus();
            state.trigger_focused = false;
            state.hovered = false;
            state.presence.close_now();
        } else if !self.focusable {
            tree.state.downcast_mut::<State>().focus.unfocus();
        }
    }

    fn size(&self) -> Size<Length> {
        self.trigger.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.trigger.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.trigger
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
        if self.disabled || !self.focusable {
            state.focus.unfocus();
        } else {
            operation.focusable(Some(&self.id.0), layout.bounds(), &mut state.focus);
        }
        operation.traverse(&mut |operation| {
            self.trigger.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
        state.trigger_focused = !self.disabled
            && focus_within(|operation| {
                self.trigger.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    operation,
                );
            });
        sync_presence(
            state,
            !self.disabled,
            Instant::now(),
            self.open_delay,
            self.close_delay,
        );
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
        let now = event_time(event);
        let state = tree.state.downcast_mut::<State>();
        let was_visible = state.presence.is_visible();
        let was_present = state.presence.is_present();

        if self.disabled || matches!(event, Event::Window(iced::window::Event::Unfocused)) {
            state.hovered = false;
            state.focus.unfocus();
        } else if matches!(event, Event::Mouse(_) | Event::Window(_)) {
            state.hovered = cursor.is_over(layout.bounds());
        }
        self.trigger.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        state.trigger_focused = !self.disabled
            && focus_within(|operation| {
                self.trigger.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    operation,
                );
            });
        sync_presence(
            state,
            !self.disabled,
            now,
            self.open_delay,
            self.close_delay,
        );
        request_transition(&state.presence, was_visible, was_present, shell);
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.trigger.as_widget().mouse_interaction(
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
        self.trigger.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        draw_focus_ring(
            renderer,
            layout.bounds(),
            tree.state.downcast_ref::<State>().focus.is_focused() && !self.disabled,
            &self.theme,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        let state = tree.state.downcast_mut::<State>();
        sync_presence(
            state,
            !self.disabled,
            Instant::now(),
            self.open_delay,
            self.close_delay,
        );
        let active = state.active();
        let [trigger_tree, content_tree] = tree.children.as_mut_slice() else {
            return None;
        };
        let trigger_overlay = self.trigger.as_widget_mut().overlay(
            trigger_tree,
            layout,
            renderer,
            viewport,
            translation,
        );
        let tooltip_overlay = state.presence.is_present().then(|| {
            let anchor = Rectangle::new(
                layout.bounds().position() + translation,
                layout.bounds().size(),
            );
            overlay::Element::new(Box::new(TooltipOverlay {
                floating: FloatingContent {
                    content: &mut self.content,
                    tree: content_tree,
                    anchor,
                    viewport: *viewport,
                    config: self.config,
                },
                presence: &mut state.presence,
                active,
                open_delay: self.open_delay,
                close_delay: self.close_delay,
            }))
        });
        let overlays = trigger_overlay
            .into_iter()
            .chain(tooltip_overlay)
            .collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

struct TooltipOverlay<'a, 'b, Message> {
    floating: FloatingContent<'a, 'b, Message>,
    presence: &'b mut DelayedPresence,
    active: bool,
    open_delay: Duration,
    close_delay: Duration,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for TooltipOverlay<'_, '_, Message>
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
        if self.presence.is_visible() {
            self.floating.draw(renderer, theme, style, layout, cursor);
        }
    }

    fn update(
        &mut self,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let was_visible = self.presence.is_visible();
        let was_present = self.presence.is_present();
        if is_escape(event) {
            self.presence.dismiss();
            shell.capture_event();
            request_transition(self.presence, was_visible, was_present, shell);
            return;
        }
        self.presence.set_active(
            self.active,
            event_time(event),
            self.open_delay,
            self.close_delay,
        );
        request_transition(self.presence, was_visible, was_present, shell);
    }

    // A tooltip cannot intercept pointer input or contain focusable controls.
    fn mouse_interaction(
        &self,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn index(&self) -> f32 {
        20.0
    }
}

pub(crate) fn event_time(event: &Event) -> Instant {
    match event {
        Event::Window(iced::window::Event::RedrawRequested(now)) => *now,
        _ => Instant::now(),
    }
}

pub(crate) fn is_escape(event: &Event) -> bool {
    matches!(
        event,
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            ..
        })
    )
}

pub(crate) fn focus_within(operate: impl FnOnce(&mut dyn widget::Operation)) -> bool {
    #[derive(Default)]
    struct FindFocused(bool);

    impl widget::Operation for FindFocused {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn widget::operation::Focusable,
        ) {
            self.0 |= state.is_focused();
        }
    }

    let mut query = FindFocused::default();
    operate(&mut query);
    query.0
}

fn sync_presence(
    state: &mut State,
    enabled: bool,
    now: Instant,
    open_delay: Duration,
    close_delay: Duration,
) {
    state
        .presence
        .set_active(enabled && state.active(), now, open_delay, close_delay);
}

pub(crate) fn request_transition<Message>(
    presence: &DelayedPresence,
    was_visible: bool,
    was_present: bool,
    shell: &mut Shell<'_, Message>,
) {
    if presence.is_visible() != was_visible || presence.is_present() != was_present {
        shell.invalidate_layout();
        shell.request_redraw();
    }
    if let Some(deadline) = presence.deadline() {
        shell.request_redraw_at(deadline);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DelayedPresence {
    phase: Phase,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Closed,
    Opening(Instant),
    Open,
    Closing(Instant),
    Dismissed,
}

impl DelayedPresence {
    pub(crate) const fn is_visible(self) -> bool {
        matches!(self.phase, Phase::Open | Phase::Closing(_))
    }

    pub(crate) const fn is_present(self) -> bool {
        !matches!(self.phase, Phase::Closed | Phase::Dismissed)
    }

    pub(crate) const fn deadline(self) -> Option<Instant> {
        match self.phase {
            Phase::Opening(deadline) | Phase::Closing(deadline) => Some(deadline),
            Phase::Closed | Phase::Open | Phase::Dismissed => None,
        }
    }

    pub(crate) fn close_now(&mut self) {
        self.phase = Phase::Closed;
    }

    pub(crate) fn dismiss(&mut self) {
        self.phase = Phase::Dismissed;
    }

    pub(crate) fn set_active(
        &mut self,
        active: bool,
        now: Instant,
        open_delay: Duration,
        close_delay: Duration,
    ) {
        self.advance(now);
        self.phase = match (active, self.phase) {
            (false, Phase::Dismissed) => Phase::Closed,
            (true, Phase::Dismissed) => Phase::Dismissed,
            (true, Phase::Closed) if open_delay.is_zero() => Phase::Open,
            (true, Phase::Closed) => Phase::Opening(now + open_delay),
            (true, Phase::Closing(_)) => Phase::Open,
            (true, phase) => phase,
            (false, Phase::Opening(_)) => Phase::Closed,
            (false, Phase::Open) if close_delay.is_zero() => Phase::Closed,
            (false, Phase::Open) => Phase::Closing(now + close_delay),
            (false, phase) => phase,
        };
    }

    fn advance(&mut self, now: Instant) {
        self.phase = match self.phase {
            Phase::Opening(deadline) if now >= deadline => Phase::Open,
            Phase::Closing(deadline) if now >= deadline => Phase::Closed,
            phase => phase,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::{FocusControl, focusable_count};
    use super::super::theme::LIGHT;
    use super::*;
    use iced::advanced::Widget;
    use iced::widget::text;

    #[test]
    fn open_and_close_delays_use_exact_deadlines() {
        let start = Instant::now();
        let mut presence = DelayedPresence::default();
        let open = Duration::from_millis(500);
        let close = Duration::from_millis(200);

        presence.set_active(true, start, open, close);
        assert_eq!(presence.deadline(), Some(start + open));
        assert!(!presence.is_visible());

        presence.set_active(true, start + open - Duration::from_millis(1), open, close);
        assert!(!presence.is_visible());
        presence.set_active(true, start + open, open, close);
        assert!(presence.is_visible());

        presence.set_active(false, start + open, open, close);
        assert_eq!(presence.deadline(), Some(start + open + close));
        assert!(presence.is_visible());
        presence.set_active(false, start + open + close, open, close);
        assert!(!presence.is_present());
    }

    #[test]
    fn hover_or_focus_keeps_one_shared_presence() {
        let start = Instant::now();
        let mut state = State {
            hovered: true,
            ..State::default()
        };
        sync_presence(&mut state, true, start, Duration::ZERO, Duration::ZERO);
        assert!(state.presence.is_visible());

        state.focus.focus();
        state.hovered = false;
        sync_presence(&mut state, true, start, Duration::ZERO, Duration::ZERO);
        assert!(state.presence.is_visible());

        state.focus.unfocus();
        sync_presence(&mut state, true, start, Duration::ZERO, Duration::ZERO);
        assert!(!state.presence.is_present());
    }

    #[test]
    fn leaving_before_open_cancels_without_a_flash() {
        let start = Instant::now();
        let mut presence = DelayedPresence::default();
        presence.set_active(true, start, Duration::from_secs(1), Duration::ZERO);
        presence.set_active(
            false,
            start + Duration::from_millis(999),
            Duration::from_secs(1),
            Duration::ZERO,
        );
        assert!(!presence.is_present());
        assert!(!presence.is_visible());
    }

    #[test]
    fn dismissal_rearms_only_after_deactivation_and_keeps_the_open_delay() {
        let start = Instant::now();
        let open = Duration::from_millis(500);
        let mut presence = DelayedPresence::default();
        presence.set_active(true, start, Duration::ZERO, Duration::ZERO);

        presence.dismiss();
        presence.set_active(true, start + open, open, Duration::ZERO);
        assert!(!presence.is_present());
        assert_eq!(presence.deadline(), None);

        presence.set_active(false, start + open, open, Duration::ZERO);
        presence.set_active(true, start + open, open, Duration::ZERO);
        assert_eq!(presence.deadline(), Some(start + open + open));
        assert!(!presence.is_visible());
    }

    #[test]
    fn tooltip_tree_keeps_noninteractive_content_out_of_trigger_layout() {
        let widget = tooltip(
            TooltipId::new("tree"),
            text("trigger"),
            text("hint"),
            &LIGHT,
        )
        .into_widget();
        assert_eq!(Widget::children(&widget).len(), 2);
        let tree = widget::Tree::new(&widget as &dyn Widget<(), _, _>);
        assert_eq!(tree.children.len(), 2);
    }

    #[test]
    fn an_interactive_trigger_can_remain_the_only_focus_stop() {
        let trigger: Element<'_, ()> = FocusControl::new(
            widget::Id::new("tooltip-child-trigger"),
            text("trigger"),
            (),
            &LIGHT,
        )
        .into();
        let element: Element<'_, ()> = tooltip(
            TooltipId::new("interactive-trigger"),
            trigger,
            text("hint"),
            &LIGHT,
        )
        .focusable(false)
        .into();

        assert_eq!(focusable_count(element), 1);
    }

    #[test]
    fn interactive_trigger_focus_opens_and_holds_without_a_wrapper_stop() {
        use iced::advanced::renderer::Headless as _;

        let trigger_id = widget::Id::new("tooltip-focused-child");
        let mut component = tooltip(
            TooltipId::new("focused-child"),
            FocusControl::new(trigger_id.clone(), text("trigger"), (), &LIGHT),
            text("hint"),
            &LIGHT,
        )
        .focusable(false)
        .open_delay(Duration::ZERO)
        .into_widget();
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let mut tree = widget::Tree::new(&component as &dyn Widget<(), _, _>);
        let node = component.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(320.0, 200.0)),
        );

        let mut focus = widget::operation::focusable::focus::<()>(trigger_id);
        component.operate(&mut tree, Layout::new(&node), &renderer, &mut focus);
        let state = tree.state.downcast_ref::<State>();
        assert!(state.trigger_focused);
        assert!(state.presence.is_visible());

        let mut unfocus = widget::operation::focusable::unfocus::<()>();
        component.operate(&mut tree, Layout::new(&node), &renderer, &mut unfocus);
        let state = tree.state.downcast_ref::<State>();
        assert!(!state.trigger_focused);
        assert!(!state.presence.is_present());
    }

    #[test]
    fn pixel_defaults_match_the_component_contract() {
        let component = tooltip::<()>(
            TooltipId::new("metrics"),
            text("trigger"),
            text("hint"),
            &LIGHT,
        );
        assert_eq!(component.config.placement, Placement::Top);
        assert_eq!(component.config.max_width, 320.0);
        assert_eq!(component.padding, Padding::new(6.0).horizontal(12.0));
        assert!(component.focusable);
        assert_eq!(DEFAULT_TEXT_SIZE, 12.0);
        assert_eq!(DEFAULT_LINE_HEIGHT, 16.0);
    }
}
