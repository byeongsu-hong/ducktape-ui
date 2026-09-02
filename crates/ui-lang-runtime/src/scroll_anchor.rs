//! Keeps a scrollable's visible rows still when content lands *above* them.
//!
//! A live list that puts the newest row on top — a trade tape, a fills list —
//! grows at the end a reader is not looking at. iced stores a scroll offset as
//! an absolute distance from the top of the content (`Anchor::Start`) and
//! never revises it when the content changes: `diff` touches the child tree and
//! nothing else. So a beat that prepends four rows leaves the offset where it
//! was and moves every row under the reader down by four rows' worth of pixels.
//! Measured on the trading terminal, a reader 120px into the recent fills had
//! the row they were on move from y=1024 to y=1128 on one beat.
//!
//! `Anchor::End` is iced's answer to this and it is the wrong one here: it
//! stores the offset as a distance from the *bottom*, which does hold the rows
//! still, but it also makes offset zero mean the bottom — so a list resting
//! where it is supposed to rest, on the newest row, would open on its oldest.
//! A list wants the start anchor's resting place and the end anchor's
//! correction, which is what this widget is.
//!
//! It wraps one scrollable, watches its content height across layout passes,
//! and when the content has grown while the reader is scrolled away from the
//! top it scrolls by exactly the growth. A reader sitting at the top is left
//! alone: offset zero already means "the newest row", and that is the one place
//! where following the content is what a reader wants.
//!
//! What it reads is *growth*, which is the whole of what a wrapper around a
//! scrollable can know: the widget below it is a box of pixels with a height,
//! not a list with row identities. A list that has reached a cap and is now
//! sliding rows off its far end has a constant height and no growth to read, so
//! a reader scrolled into one still watches it slide — correcting that needs
//! the row model, which is where `virtual_list` already keeps its own anchor
//! (`RowsMeasured`, the `anchor`/`anchor_gap` pair). Fixing it here would mean
//! carrying keys through the scroll boundary; reach for the virtualized list
//! instead when a capped live list has to be read while it moves.

use iced::advanced::widget::operation::Outcome;
use iced::advanced::widget::operation::scrollable::{AbsoluteOffset, RelativeOffset};
use iced::advanced::widget::{Id, Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// A scroll-anchoring wrapper around a single scrollable.
pub struct ScrollAnchor<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

/// Wraps `content` — a scrollable — so that content arriving above the
/// viewport does not move what the reader is looking at.
pub fn scroll_anchor<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> ScrollAnchor<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    ScrollAnchor {
        content: content.into(),
    }
}

/// What the last layout pass saw. Both halves matter: the correction is only
/// sound while the *viewport* holds still, because a narrower viewport reflows
/// rows and grows the content without anything having been inserted.
#[derive(Default)]
struct AnchorState {
    content_height: Option<f32>,
    viewport_height: Option<f32>,
}

/// Reads the wrapped scrollable's geometry and applies the correction in the
/// same walk — `Operation::scrollable` hands over the content bounds, the
/// current translation and the scroll state together, so nothing has to be
/// carried between two passes.
struct Anchor {
    previous_content: Option<f32>,
    previous_viewport: Option<f32>,
    seen: Option<(f32, f32)>,
}

impl Operation for Anchor {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        // The wrapped scrollable is the first this walk reaches — a container
        // operates on itself before its children — so a nested scrollable
        // inside the list keeps its own offset.
        if self.seen.is_some() {
            return;
        }
        self.seen = Some((content_bounds.height, bounds.height));

        let (Some(previous_content), Some(previous_viewport)) =
            (self.previous_content, self.previous_viewport)
        else {
            // The first pass has nothing to compare against.
            return;
        };
        // Half a logical pixel: layout arithmetic on fills and fractional
        // scales does not land on the same float twice.
        if (bounds.height - previous_viewport).abs() > 0.5 {
            return;
        }
        let grown = content_bounds.height - previous_content;
        if grown <= 0.5 || translation.y <= 0.5 {
            return;
        }
        state.scroll_by(AbsoluteOffset { x: 0.0, y: grown }, bounds, content_bounds);
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ScrollAnchor<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<AnchorState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(AnchorState::default())
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
        // Layout is where the growth becomes visible and the only place it can
        // be corrected before the frame is drawn — the offset is applied as a
        // translation at draw time, so a correction landing here still shows
        // this frame. `virtual_list` reaches its own scrollable the same way.
        let state = tree.state.downcast_ref::<AnchorState>();
        let mut anchor = Anchor {
            previous_content: state.content_height,
            previous_viewport: state.viewport_height,
            seen: None,
        };
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            Layout::new(&node),
            renderer,
            &mut anchor,
        );
        if let Some((content_height, viewport_height)) = anchor.seen {
            let state = tree.state.downcast_mut::<AnchorState>();
            state.content_height = Some(content_height);
            state.viewport_height = Some(viewport_height);
        }
        node
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
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

impl<'a, Message, Theme, Renderer> From<ScrollAnchor<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(anchor: ScrollAnchor<'a, Message, Theme, Renderer>) -> Self {
        Self::new(anchor)
    }
}

/// The task behind `task widget snap-end`, and the driver's `snap-end` step:
/// leaves the scrollable named `target` showing the end of its content.
///
/// iced's own `snap_to_end` is relative offset one, and an offset counts from
/// wherever the scrollable is anchored. Under the end anchor `anchor-y=end`
/// asks for, offset zero is the newest row and offset one is the oldest, so
/// snapping "to the end" of a chat walked the reader back to the start of the
/// conversation — the opposite of what the step is named for.
///
/// A scrollable does not report its anchor to an operation, so this asks
/// instead of assuming. It snaps every axis to offset zero and then reads the
/// translation that produced: offset zero translates to zero under
/// `Anchor::Start` and to the whole overflow under `Anchor::End`. An axis that
/// did not move is therefore start-anchored and wants offset one; an axis that
/// did is already showing its end. Both passes run inside one `Action::Widget`,
/// against one layout, so nothing is drawn at the intermediate position.
pub fn snap_to_content_end<Message: Send + 'static>(target: Id) -> iced::Task<Message> {
    iced::advanced::widget::operate(content_end_operation::<Message>(target))
}

/// The same operation, unwrapped, for the headless driver: it runs operations
/// itself rather than through a task.
pub(crate) fn content_end_operation<T: 'static>(target: Id) -> impl Operation<T> + 'static {
    SnapToOffsetZero {
        target,
        found: false,
    }
}

/// First pass: put every axis at offset zero, whichever end that turns out to
/// be.
struct SnapToOffsetZero {
    target: Id,
    found: bool,
}

impl<T: 'static> Operation<T> for SnapToOffsetZero {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id != Some(&self.target) {
            return;
        }
        state.snap_to(RelativeOffset {
            x: Some(0.0),
            y: Some(0.0),
        });
        self.found = true;
    }

    fn finish(&self) -> Outcome<T> {
        if self.found {
            Outcome::Chain(Box::new(SnapStartAnchoredAxes {
                target: self.target.clone(),
            }))
        } else {
            Outcome::None
        }
    }
}

/// Second pass: correct the axes offset zero left sitting at their start.
struct SnapStartAnchoredAxes {
    target: Id,
}

impl<T: 'static> Operation<T> for SnapStartAnchoredAxes {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id != Some(&self.target) {
            return;
        }
        // An axis with nothing to scroll translates to zero from either anchor
        // and shows the same pixels at either offset, so it carries no answer
        // and is left where it is.
        let x = (content_bounds.width > bounds.width && translation.x == 0.0).then_some(1.0);
        let y = (content_bounds.height > bounds.height && translation.y == 0.0).then_some(1.0);
        if x.is_some() || y.is_some() {
            state.snap_to(RelativeOffset { x, y });
        }
    }
}

/// Puts a scrollable `y` logical pixels down its content, whichever end its
/// offsets count from.
///
/// A position inside content — a row's top, say — is measured from the top of
/// that content, the way every row top is. An iced scroll offset is measured
/// from the scroll's own anchor, and under `Anchor::End` the two run in
/// opposite directions, so writing a content position straight into `scroll_to`
/// lands an `anchor-y=end` list nowhere near the row that was asked for.
///
/// Like [`snap_to_content_end`], this asks rather than assumes, and in the same
/// two passes: `probe` puts the axis at offset zero, and the pass after it
/// reads the translation that produced. Zero means the axis counts from the
/// start and already wants `y`; the whole overflow means it counts from the end
/// and wants the rest of that overflow instead.
///
/// `target` names the scrollable; `None` takes the first one the walk meets,
/// which is what a reveal running inside its own scrollable's layout wants.
pub(crate) struct ScrollContentTo {
    target: Option<Id>,
    y: f32,
    /// False on the probe pass, true on the pass that writes.
    placing: bool,
    /// One scrollable per pass, so a nested one does not chase the same offset.
    done: bool,
}

impl ScrollContentTo {
    pub(crate) fn probe(target: Option<Id>, y: f32) -> Self {
        Self {
            target,
            y,
            placing: false,
            done: false,
        }
    }

    /// The second pass, for a caller driving both itself rather than chaining.
    pub(crate) fn place(&self) -> Self {
        Self {
            target: self.target.clone(),
            y: self.y,
            placing: true,
            done: false,
        }
    }
}

impl<T: 'static> Operation<T> for ScrollContentTo {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if self.done
            || self
                .target
                .as_ref()
                .is_some_and(|target| id != Some(target))
        {
            return;
        }
        self.done = true;
        if !self.placing {
            state.snap_to(RelativeOffset {
                x: None,
                y: Some(0.0),
            });
            return;
        }
        let overflow = (content_bounds.height - bounds.height).max(0.0);
        let y = if translation.y == 0.0 {
            self.y
        } else {
            overflow - self.y
        };
        state.scroll_to(AbsoluteOffset {
            x: None,
            y: Some(y),
        });
    }

    fn finish(&self) -> Outcome<T> {
        if self.placing || !self.done {
            Outcome::None
        } else {
            Outcome::Chain(Box::new(self.place()))
        }
    }
}
