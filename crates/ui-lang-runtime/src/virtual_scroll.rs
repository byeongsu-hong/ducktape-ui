//! Keeps `virtual-row` content mounted over the viewport its scrollable
//! actually shows.
//!
//! A virtual column seeds its window from the viewport it REMEMBERS, and two
//! moments leave that memory wrong with nothing else re-opening layout:
//!
//! - **The first frame, and every children replacement.** Before any event
//!   lands the column fills a screen's worth from the strip's top — but an
//!   end-anchored scrollable (a chat timeline, a transcript) shows the
//!   strip's BOTTOM, so the mounted window and the visible strip never
//!   intersect and the frame draws no rows. This wrapper re-reads the
//!   scrollable's real translation inside `layout` and lays out once more
//!   when the window escaped it; the second pass costs one screenful of rows
//!   and only runs on mismatch frames.
//! - **A rapid wheel transaction.** Iced deliberately stops forwarding
//!   consecutive wheel events to the scrollable's descendants, so a fast
//!   trackpad burst can translate every mounted row out of the viewport.
//!   The wrapper synchronizes after the scrollable consumes each wheel event
//!   and requests layout only when the mounted overscan no longer covers it.

use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::virtual_children::sync_virtual_columns;

pub struct VirtualScroll<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

pub fn virtual_scroll<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> VirtualScroll<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    VirtualScroll {
        content: content.into(),
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for VirtualScroll<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        // The columns below seeded their windows from remembered viewports;
        // the scrollable's translation in THIS layout is where they really
        // are. One more pass on the frames where a window escaped it — the
        // first frame and children replacements — keeps every drawn frame
        // aligned without waiting on an invalidation nothing else raises.
        let escaped = sync_virtual_columns(
            &mut self.content,
            &mut tree.children[0],
            Layout::new(&node),
            renderer,
        );
        if !escaped {
            return node;
        }
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
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
        if matches!(event, Event::Mouse(mouse::Event::WheelScrolled { .. }))
            && shell.is_event_captured()
            && sync_virtual_columns(&mut self.content, &mut tree.children[0], layout, renderer)
        {
            shell.invalidate_layout();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
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
        tree: &'b mut Tree,
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

impl<'a, Message, Theme, Renderer> From<VirtualScroll<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(scroll: VirtualScroll<'a, Message, Theme, Renderer>) -> Self {
        Self::new(scroll)
    }
}
