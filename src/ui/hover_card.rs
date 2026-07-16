//! Delayed hover and focus previews with safe pointer transfer into the card.
//!
//! The trigger is passive and receives one wrapper-owned focus stop. Unlike a
//! tooltip, hover-card content is interactive and keeps the card open while
//! the pointer crosses from the trigger into the floating surface.

use super::popover::{
    Alignment, FloatingConfig, FloatingContent, FocusFlag, PanelKind, Placement, draw_focus_ring,
    panel,
};
use super::theme::Theme as UiTheme;
use super::tooltip::{DelayedPresence, event_time, request_transition};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::time::{Duration, Instant};
use iced::{Element, Event, Length, Padding, Rectangle, Size, Vector};

const DEFAULT_OPEN_DELAY: Duration = Duration::from_millis(700);
const DEFAULT_CLOSE_DELAY: Duration = Duration::from_millis(300);
const DEFAULT_WIDTH: f32 = 256.0;
const DEFAULT_MAX_WIDTH: f32 = 320.0;
const DEFAULT_PADDING: f32 = 16.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverCardId(widget::Id);

impl HoverCardId {
    pub fn new(key: impl ToString) -> Self {
        Self(widget::Id::from(format!(
            "ducktape-hover-card:{}:trigger",
            key.to_string()
        )))
    }
}

pub struct HoverCard<'a, Message> {
    id: HoverCardId,
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    config: FloatingConfig,
    open_delay: Duration,
    close_delay: Duration,
    width: f32,
    padding: Padding,
    disabled: bool,
    theme: UiTheme,
}

pub fn hover_card<'a, Message>(
    id: HoverCardId,
    trigger: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    theme: &UiTheme,
) -> HoverCard<'a, Message> {
    HoverCard {
        id,
        trigger: trigger.into(),
        content: content.into(),
        config: FloatingConfig {
            placement: Placement::Bottom,
            max_width: DEFAULT_MAX_WIDTH,
            ..FloatingConfig::default()
        },
        open_delay: DEFAULT_OPEN_DELAY,
        close_delay: DEFAULT_CLOSE_DELAY,
        width: DEFAULT_WIDTH,
        padding: Padding::new(DEFAULT_PADDING),
        disabled: false,
        theme: *theme,
    }
}

impl<Message> HoverCard<'_, Message> {
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
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
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
}

impl<'a, Message> HoverCard<'a, Message>
where
    Message: 'a,
{
    fn into_widget(self) -> HoverCardWidget<'a, Message> {
        let config = self.config.sanitized();
        HoverCardWidget {
            id: self.id,
            trigger: self.trigger,
            content: panel(
                self.content,
                PanelKind::HoverCard,
                Some(self.width.min(config.max_width)),
                config.max_width,
                self.padding,
                &self.theme,
            ),
            config,
            open_delay: self.open_delay,
            close_delay: self.close_delay,
            disabled: self.disabled,
            theme: self.theme,
        }
    }
}

impl<'a, Message> From<HoverCard<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(card: HoverCard<'a, Message>) -> Self {
        Element::new(card.into_widget())
    }
}

struct HoverCardWidget<'a, Message> {
    id: HoverCardId,
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    config: FloatingConfig,
    open_delay: Duration,
    close_delay: Duration,
    disabled: bool,
    theme: UiTheme,
}

#[derive(Debug, Default)]
struct State {
    focus: FocusFlag,
    trigger_hovered: bool,
    content_hovered: bool,
    presence: DelayedPresence,
}

impl State {
    fn active(&self) -> bool {
        self.trigger_hovered || self.content_hovered || self.focus.is_focused()
    }

    fn sync(&mut self, enabled: bool, now: Instant, open: Duration, close: Duration) {
        self.presence
            .set_active(enabled && self.active(), now, open, close);
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for HoverCardWidget<'_, Message> {
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
            state.trigger_hovered = false;
            state.content_hovered = false;
            state.presence.close_now();
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
        if self.disabled {
            state.focus.unfocus();
        } else {
            operation.focusable(Some(&self.id.0), layout.bounds(), &mut state.focus);
        }
        state.sync(
            !self.disabled,
            Instant::now(),
            self.open_delay,
            self.close_delay,
        );

        operation.traverse(&mut |operation| {
            self.trigger.as_widget_mut().operate(
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
            state.focus.unfocus();
            state.trigger_hovered = false;
            state.content_hovered = false;
        } else if matches!(event, Event::Mouse(_) | Event::Window(_)) {
            state.trigger_hovered = cursor.is_over(layout.bounds());
        }
        state.sync(!self.disabled, now, self.open_delay, self.close_delay);
        request_transition(&state.presence, was_visible, was_present, shell);

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
        state.sync(
            !self.disabled,
            Instant::now(),
            self.open_delay,
            self.close_delay,
        );
        let active = state.active();
        let content_hovered = state.content_hovered;
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
        let card_overlay = state.presence.is_present().then(|| {
            let anchor = Rectangle::new(
                layout.bounds().position() + translation,
                layout.bounds().size(),
            );
            overlay::Element::new(Box::new(HoverCardOverlay {
                floating: FloatingContent {
                    content: &mut self.content,
                    tree: content_tree,
                    anchor,
                    viewport: *viewport,
                    config: self.config,
                },
                presence: &mut state.presence,
                content_hovered: &mut state.content_hovered,
                active_without_content: active && !content_hovered,
                open_delay: self.open_delay,
                close_delay: self.close_delay,
            }))
        });
        let overlays = trigger_overlay
            .into_iter()
            .chain(card_overlay)
            .collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

struct HoverCardOverlay<'a, 'b, Message> {
    floating: FloatingContent<'a, 'b, Message>,
    presence: &'b mut DelayedPresence,
    content_hovered: &'b mut bool,
    active_without_content: bool,
    open_delay: Duration,
    close_delay: Duration,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for HoverCardOverlay<'_, '_, Message>
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

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        if self.presence.is_visible() {
            operation.traverse(&mut |operation| {
                self.floating.operate(layout, renderer, operation);
            });
        }
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
        let was_visible = self.presence.is_visible();
        let was_present = self.presence.is_present();
        if matches!(event, Event::Mouse(_) | Event::Window(_)) {
            *self.content_hovered =
                self.presence.is_visible() && cursor.is_over(self.floating.bounds(layout));
        }
        self.presence.set_active(
            self.active_without_content || *self.content_hovered,
            event_time(event),
            self.open_delay,
            self.close_delay,
        );
        request_transition(self.presence, was_visible, was_present, shell);

        if self.presence.is_visible() {
            self.floating
                .update(event, layout, cursor, renderer, clipboard, shell);
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.floating
            .interaction(layout, cursor, renderer, self.presence.is_visible())
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        self.presence
            .is_visible()
            .then(|| self.floating.overlay(layout, renderer))
            .flatten()
    }

    fn index(&self) -> f32 {
        15.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::LIGHT;
    use iced::advanced::Widget;
    use iced::widget::text;

    #[test]
    fn pointer_transfer_cancels_a_pending_close() {
        let start = Instant::now();
        let mut state = State {
            trigger_hovered: true,
            ..State::default()
        };
        state.sync(true, start, Duration::ZERO, DEFAULT_CLOSE_DELAY);
        assert!(state.presence.is_visible());

        state.trigger_hovered = false;
        state.sync(
            true,
            start + Duration::from_millis(10),
            Duration::ZERO,
            DEFAULT_CLOSE_DELAY,
        );
        assert!(state.presence.deadline().is_some());

        state.content_hovered = true;
        state.sync(
            true,
            start + Duration::from_millis(100),
            Duration::ZERO,
            DEFAULT_CLOSE_DELAY,
        );
        assert!(state.presence.is_visible());
        assert_eq!(state.presence.deadline(), None);
    }

    #[test]
    fn focus_uses_the_same_deterministic_open_delay() {
        let start = Instant::now();
        let mut state = State::default();
        state.focus.focus();
        state.sync(true, start, DEFAULT_OPEN_DELAY, DEFAULT_CLOSE_DELAY);
        assert_eq!(state.presence.deadline(), Some(start + DEFAULT_OPEN_DELAY));
        state.sync(
            true,
            start + DEFAULT_OPEN_DELAY,
            DEFAULT_OPEN_DELAY,
            DEFAULT_CLOSE_DELAY,
        );
        assert!(state.presence.is_visible());
    }

    #[test]
    fn focus_holds_card_when_pointer_leaves_both_surfaces() {
        let start = Instant::now();
        let mut state = State::default();
        state.focus.focus();
        state.sync(true, start, Duration::ZERO, Duration::ZERO);
        state.trigger_hovered = false;
        state.content_hovered = false;
        state.sync(true, start, Duration::ZERO, Duration::ZERO);
        assert!(state.presence.is_visible());
    }

    #[test]
    fn tree_keeps_trigger_and_interactive_overlay_content() {
        let widget = hover_card(
            HoverCardId::new("tree"),
            text("trigger"),
            text("preview"),
            &LIGHT,
        )
        .into_widget();
        assert_eq!(Widget::children(&widget).len(), 2);
        let tree = widget::Tree::new(&widget as &dyn Widget<(), _, _>);
        assert_eq!(tree.children.len(), 2);
    }

    #[test]
    fn pixel_defaults_match_hover_card_content() {
        let component = hover_card::<()>(
            HoverCardId::new("metrics"),
            text("trigger"),
            text("preview"),
            &LIGHT,
        );
        assert_eq!(component.width, 256.0);
        assert_eq!(component.config.max_width, 320.0);
        assert_eq!(component.padding, Padding::new(16.0));
        assert_eq!(component.config.side_offset, 4.0);
    }
}
