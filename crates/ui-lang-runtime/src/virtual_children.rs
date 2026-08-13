//! A column that lays out only the children the viewport can see.
//!
//! [`crate::virtual_list`] avoids building offscreen rows at all, which costs
//! the caller a builder closure and a state reducer. That price buys very
//! little: constructing a chat row is ~0.24µs while laying one out and shaping
//! its text is ~87µs, so **construction is under half a percent of the bill**
//! (`tests/frame_probe.rs` measures both). Text is shaped in `layout`, not in
//! `Element` construction — so a column can accept every child, hand back
//! placeholder nodes for the ones offscreen, and never shape them.
//!
//! That makes this usable from anywhere a plain column is, including a
//! generated `for` body, with no closure, no key type, and no caller-owned
//! state. Offscreen children keep their widget state (they stay in the tree);
//! they are simply not measured, drawn, or offered events.
//!
//! Mount it inside a vertical `scrollable`. The window comes from the viewport
//! the previous pass observed, and a viewport change re-opens layout — the
//! same trick the rich-text editor uses to bound its highlighting.
//!
//! Measuring a child above the viewport moves everything below it. Anchor the
//! scrollable to the end ([`iced::widget::scrollable::Anchor::End`]) whenever
//! the content is read by scrolling backwards through children that have never
//! been measured: an end-anchored scrollable stores its offset as a distance
//! from the bottom, so content growing above carries the offset with it and the
//! visible rows hold still. Scrolling forwards needs nothing, because children
//! enter from the end and correct only what is already past.
//!
//! # Keyboard focus is kept; accessibility is not
//!
//! **Focus survives scrolling.** Whichever child holds keyboard focus is
//! measured wherever it sits, in addition to the window, so it keeps receiving
//! key presses and stays visible to focus operations. That is not a nicety: an
//! [`iced::advanced::widget::operation::focusable::focus_next`] counts
//! focusables before it moves focus, and a pass that cannot see the focused
//! child concludes nothing is focused and focuses a *second* widget, leaving
//! two focus rings and two widgets answering Enter. Focus is re-read whenever
//! the viewport moves and at the end of every [`Operation`] — the two moments
//! it can change — while the outgoing child still has a layout to be asked
//! through.
//!
//! **Offscreen children are absent from the accessibility tree.** Publishing a
//! child's semantics means running an [`Operation`] over it; an operation needs
//! a real layout subtree (a container's `operate` unwraps its child node, so a
//! placeholder panics); and building one means laying the child out, which is
//! the shaping this widget exists to skip. So a screen reader sees only the
//! mounted slice, and this column publishes no `size_of_set`/`position_in_set`
//! to compensate, because it does not own its children's semantics. `.ice`
//! tests read that same snapshot, so `click` and `expect a11y` cannot target an
//! offscreen child either. When a collection has to read correctly to assistive
//! tech, use [`crate::virtual_list`], which owns its rows and publishes set
//! metadata and an active descendant for them. A `virtual-row` column is for
//! long, read-mostly content.
//!
//! # Rows can carry keys
//!
//! [`virtual_keyed_children`] is the same widget with identity: the caller
//! hands a key beside every child and per-child state — the widget [`Tree`],
//! the measured height, which row holds focus — follows the key instead of the
//! index, exactly as [`iced::widget::keyed_column`] does. That is what lets a
//! list whose newest row arrives on **top** be virtualized at all: under index
//! diffing a prepend shifts every row's state down one, which rebuilds every
//! memoized row and hands every measured height to its neighbour. Mounting is
//! still a window over the viewport; keys decide only whose state is whose.

use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Id, Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};
use rustc_hash::FxHashMap;

/// Extra rows kept live on each side of the viewport so a scroll of a row or
/// two reveals something already measured.
const OVERSCAN_ROWS: usize = 4;

/// Stands in for the viewport height on the one pass that precedes any event
/// reporting a real one. Only the order of magnitude matters: the first
/// `update` corrects it.
const NOMINAL_SCREEN_HEIGHT: f32 = 1_080.0;

/// Lays out only the visible slice of `children`, estimating the rest at
/// `estimated_height` until they are measured.
///
/// The estimate only has to be the right order of magnitude: every child that
/// enters the viewport is measured for real and remembered, so the scrollbar
/// converges as the reader moves.
pub fn virtual_children<'a, Message, Theme, Renderer>(
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    estimated_height: f32,
) -> VirtualChildren<'a, Message, Theme, Renderer> {
    VirtualChildren {
        children,
        keys: Vec::new(),
        estimated_height: estimated_height.max(1.0),
        spacing: 0.0,
    }
}

/// [`virtual_children`] with per-row identity: child state follows its key
/// through inserts and reordering rather than its position.
pub fn virtual_keyed_children<'a, Message, Theme, Renderer>(
    children: Vec<(u64, Element<'a, Message, Theme, Renderer>)>,
    estimated_height: f32,
) -> VirtualChildren<'a, Message, Theme, Renderer> {
    let (keys, children) = children.into_iter().unzip();
    VirtualChildren {
        children,
        keys,
        estimated_height: estimated_height.max(1.0),
        spacing: 0.0,
    }
}

/// An Ice keyed-column key as the 64 bits this widget identifies a row by.
///
/// Ice keys are `bool`, `i64`, or `f64`, and each has a lossless 64-bit image,
/// so this is identity rather than hashing: no collisions to accept. Bit
/// identity also settles the two values `PartialEq` — what
/// [`iced::widget::keyed_column`] compares keys with — answers badly for. A row
/// keyed `NaN` matches no key including its own, so under `PartialEq` it is a
/// different row every pass and can never keep state; here it keeps it. And
/// `-0.0` is its own row rather than `0.0`'s, which is what a list holding both
/// needs, since it holds two items either way.
pub trait VirtualKey: Copy {
    /// This key's 64-bit image.
    fn virtual_key(self) -> u64;
}

impl VirtualKey for bool {
    fn virtual_key(self) -> u64 {
        u64::from(self)
    }
}

impl VirtualKey for i64 {
    fn virtual_key(self) -> u64 {
        self.cast_unsigned()
    }
}

impl VirtualKey for f64 {
    fn virtual_key(self) -> u64 {
        self.to_bits()
    }
}

/// Mount this as the single child of an ordinary column, which keeps handling
/// padding, dimensions, and the rest; only per-child layout moves in here.
pub struct VirtualChildren<'a, Message, Theme, Renderer> {
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    /// One key per child, or empty for an unkeyed column whose rows are their
    /// own positions.
    keys: Vec<u64>,
    estimated_height: f32,
    spacing: f32,
}

impl<Message, Theme, Renderer> VirtualChildren<'_, Message, Theme, Renderer> {
    /// The gap between children, matching `column(..).spacing(..)`. It is
    /// counted in the geometry, so an offscreen child's slot is the same size
    /// whether or not it has been measured.
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }
}

impl<'a, Message, Theme, Renderer> From<VirtualChildren<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(virtual_children: VirtualChildren<'a, Message, Theme, Renderer>) -> Self {
        Self::new(virtual_children)
    }
}

/// The children the last `layout` measured for real: the viewport window, plus
/// the one holding keyboard focus wherever it happens to sit. Everything else
/// got a placeholder node, so only these can be drawn, updated, or operated on.
#[derive(Clone, Default)]
struct Live {
    mounted: std::ops::Range<usize>,
    focused: Option<usize>,
}

impl Live {
    fn contains(&self, index: usize) -> bool {
        self.mounted.contains(&index) || self.focused == Some(index)
    }
}

#[derive(Default)]
struct State {
    /// Real heights for children that have been laid out, `None` until then.
    measured: Vec<Option<f32>>,
    /// The keys the last pass ran against, so a keyed column can carry
    /// `measured` and `live.focused` across to whatever index a row moved to.
    /// Empty for an unkeyed column.
    keys: Vec<u64>,
    /// The visible region the last pass worked against.
    viewport: Rectangle,
    /// What layout actually measured, so draw and events agree with it.
    live: Live,
    estimated_height: f32,
    spacing: f32,
}

impl State {
    fn height_of(&self, index: usize, estimate: f32) -> f32 {
        self.measured
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(estimate)
    }

    /// Row tops, from measurements where they exist and the estimate
    /// elsewhere, with `spacing` between every pair.
    fn tops(&self, count: usize, estimate: f32, spacing: f32) -> Vec<f32> {
        let mut tops = Vec::with_capacity(count);
        let mut running = 0.0;
        for index in 0..count {
            tops.push(running);
            running += self.height_of(index, estimate) + spacing;
        }
        tops
    }

    /// Moves the remembered viewport and reports whether it escaped the rows
    /// already laid out. Moving inside overscan needs no layout at all: the
    /// scrollable clips the extra rows and only changes their draw translation.
    fn sync_viewport(&mut self, viewport: Rectangle, bounds: Rectangle) -> bool {
        let visible = viewport - Vector::new(bounds.x, bounds.y);
        if self.viewport == visible {
            return false;
        }
        self.viewport = visible;

        let tops = self.tops(self.measured.len(), self.estimated_height, self.spacing);
        let first = tops
            .partition_point(|top| *top <= visible.y)
            .saturating_sub(1);
        let last = tops.partition_point(|top| *top < visible.y + visible.height);
        first < self.live.mounted.start || last > self.live.mounted.end
    }
}

/// Synchronizes virtual descendants with the first scrollable in `content`
/// after that scrollable consumes a wheel event. Iced suppresses descendant
/// wheel updates for the rest of a scroll transaction; this operation crosses
/// that deliberate event boundary without synthesizing pointer events.
pub(crate) fn sync_after_wheel<Message, Theme, Renderer>(
    content: &mut Element<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &Renderer,
) -> bool
where
    Renderer: iced::advanced::Renderer,
{
    let mut sync = SyncAfterWheel::default();
    content
        .as_widget_mut()
        .operate(tree, layout, renderer, &mut sync);
    sync.needs_layout
}

#[derive(Default)]
struct SyncAfterWheel {
    viewport: Option<Rectangle>,
    nested_scrollable: bool,
    skip_virtual_children: bool,
    needs_layout: bool,
}

impl Operation for SyncAfterWheel {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if self.nested_scrollable {
            self.nested_scrollable = false;
        } else if self.skip_virtual_children {
            self.skip_virtual_children = false;
        } else {
            operate(self);
        }
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if self.viewport.is_some() {
            self.nested_scrollable = true;
        } else {
            self.viewport = Some(Rectangle {
                x: bounds.x + translation.x,
                y: bounds.y + translation.y,
                ..bounds
            });
        }
    }

    fn custom(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn std::any::Any) {
        let (Some(viewport), Some(state)) = (self.viewport, state.downcast_mut::<State>()) else {
            return;
        };
        self.needs_layout |= state.sync_viewport(viewport, bounds);
        // The viewport sync is the operation's whole purpose. Do not walk the
        // mounted rows just to discover that none of them are virtual columns.
        self.skip_virtual_children = true;
    }
}

/// Reports whether anything in the subtree it is run over holds focus.
#[derive(Default)]
struct FindFocus {
    found: bool,
}

impl Operation for FindFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.found |= state.is_focused();
    }
}

impl<Message, Theme, Renderer> VirtualChildren<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// Re-reads which child holds focus so the next `layout` can keep measuring
    /// it after it leaves the window.
    ///
    /// Only children that were measured can be asked, which is enough: a child
    /// can only *take* focus while it is measured, so every acquisition is
    /// observed by the next call — and once observed, the child is measured
    /// from then on and keeps reporting.
    fn track_focus(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) {
        let live = tree.state.downcast_ref::<State>().live.clone();

        let mut focused = None;
        for ((index, child), child_layout) in
            self.children.iter_mut().enumerate().zip(layout.children())
        {
            if !live.contains(index) {
                continue;
            }
            let mut probe = FindFocus::default();
            child.as_widget_mut().operate(
                &mut tree.children[index],
                child_layout,
                renderer,
                &mut probe,
            );
            if probe.found {
                focused = Some(index);
            }
        }

        // Whatever this found is either inside the window or the child already
        // being kept for focus, so widening the measured set is never implied —
        // draw and events cannot start reaching a placeholder.
        tree.state.downcast_mut::<State>().live.focused = focused;
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for VirtualChildren<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            measured: vec![None; self.children.len()],
            keys: self.keys.clone(),
            estimated_height: self.estimated_height,
            spacing: self.spacing,
            ..State::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        {
            let state = tree.state.downcast_mut::<State>();
            state.estimated_height = self.estimated_height;
            state.spacing = self.spacing;
        }
        if self.keys.is_empty() {
            tree.diff_children(&self.children);
            let state = tree.state.downcast_mut::<State>();
            state.measured.resize(self.children.len(), None);
            if state
                .live
                .focused
                .is_some_and(|index| index >= self.children.len())
            {
                state.live.focused = None;
            }
            return;
        }
        if tree.state.downcast_ref::<State>().keys == self.keys {
            tree.diff_children(&self.children);
            return;
        }

        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let previous = std::mem::take(&mut state.keys);

        // Child widget state moves to wherever its key went, so a prepend does
        // not shift every row's memo, cursor, and hover onto its neighbour.
        tree::diff_children_custom_with_search(
            children,
            &self.children,
            |tree, child| tree.diff(child.as_widget()),
            |index| {
                self.keys.get(index).or_else(|| self.keys.last()).copied()
                    != previous.get(index).copied()
            },
            |child| Tree::new(child.as_widget()),
        );

        // Heights are the same kind of per-row state and move the same way. A
        // list that prepends would otherwise hand every row below the new one
        // the height of its predecessor, which is a visible jump the moment
        // rows are not all one height.
        let heights: FxHashMap<u64, f32> = previous
            .iter()
            .zip(&state.measured)
            .filter_map(|(key, height)| height.map(|height| (*key, height)))
            .collect();
        state.measured = self
            .keys
            .iter()
            .map(|key| heights.get(key).copied())
            .collect();
        // And so does the focused row: it is measured wherever it sits, and an
        // index pointing at its neighbour instead means the focused row stops
        // being measured, which is how focus gets lost for good.
        state.live.focused = state
            .live
            .focused
            .and_then(|index| previous.get(index))
            .and_then(|key| self.keys.iter().position(|candidate| candidate == key));
        state.keys.clone_from(&self.keys);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let count = self.children.len();
        let state = tree.state.downcast_mut::<State>();
        let tops = state.tops(count, self.estimated_height, self.spacing);

        // Before the first draw the viewport is unknown; fill a screen's worth
        // from the top rather than the whole document. A vertical scrollable
        // hands its content an *infinite* height limit, so the limit alone
        // would mean the whole document — which is the one moment a long list
        // can least afford to shape every row.
        let visible_top = state.viewport.y;
        let visible_height = if state.viewport.height > 0.0 {
            state.viewport.height
        } else if limits.max().height.is_finite() {
            limits.max().height.max(self.estimated_height)
        } else {
            NOMINAL_SCREEN_HEIGHT.max(self.estimated_height)
        };
        let first = tops
            .partition_point(|top| *top <= visible_top)
            .saturating_sub(1);
        let last = tops.partition_point(|top| *top < visible_top + visible_height);
        let live = Live {
            mounted: first.saturating_sub(OVERSCAN_ROWS)..(last + OVERSCAN_ROWS).min(count),
            // Measure the focused child wherever it is, so it keeps its key
            // events and stays visible to focus operations. It draws outside
            // the viewport and the enclosing scrollable clips it away.
            focused: state.live.focused,
        };

        let width = limits.max().width;
        let mut nodes = Vec::with_capacity(count);
        let mut running = 0.0;
        for index in 0..count {
            let node = if live.contains(index) {
                // Height-compressed, exactly as the enclosing scrollable hands
                // its content: flex then resolves a fill-height descendant — a
                // `rule vertical` in a row, say — against measured content
                // instead of against this infinite limit. Plain `Limits::new`
                // dropped that flag, and one such row measured infinite,
                // placing every row below it at y = ∞.
                let child_limits = layout::Limits::with_compression(
                    Size::ZERO,
                    Size::new(width, f32::INFINITY),
                    Size::new(limits.compression().width, true),
                );
                let node = self.children[index].as_widget_mut().layout(
                    &mut tree.children[index],
                    renderer,
                    &child_limits,
                );
                state.measured[index] = Some(node.size().height);
                node
            } else {
                // Never laid out, so never shaped. It still needs a node so the
                // layout tree stays parallel to the widget tree.
                layout::Node::new(Size::new(
                    width,
                    state.height_of(index, self.estimated_height),
                ))
            };
            let height = node.size().height;
            nodes.push(node.move_to(iced::Point::new(0.0, running)));
            running += height + self.spacing;
        }
        // `running` carries a trailing gap for every child, including the last.
        let content_height = (running - self.spacing).max(0.0);

        state.live = live;
        layout::Node::with_children(Size::new(width, content_height), nodes)
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
        // Row offsets are measured from this widget's own top, while the
        // viewport a scrollable reports is in screen coordinates. Rebase it, or
        // everything above the scrollable — a header, a toolbar — shifts the
        // window down by its own height and leaves that much of the list blank
        // at the top. Only a scrollable sitting at y=0 gets away without this.
        // The children still get the screen-space `viewport` they expect.
        let visible = *viewport - Vector::new(layout.bounds().x, layout.bounds().y);

        // Scrolling moves the viewport without changing anything this widget
        // owns, so nothing else would re-open layout — and until it does, the
        // rows scrolled into view have never been measured.
        if tree.state.downcast_ref::<State>().viewport != visible {
            // A click focuses a child from inside its own `update`, where no
            // operation can see it. This is the last pass where the outgoing
            // window still has layouts to ask through, so ask now.
            self.track_focus(tree, layout, renderer);
            tree.state.downcast_mut::<State>().viewport = visible;
            shell.invalidate_layout();
        }

        let live = tree.state.downcast_ref::<State>().live.clone();
        for ((index, child), child_layout) in
            self.children.iter_mut().enumerate().zip(layout.children())
        {
            if !live.contains(index) {
                continue;
            }
            child.as_widget_mut().update(
                &mut tree.children[index],
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
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
        let live = tree.state.downcast_ref::<State>().live.clone();
        for ((index, child), child_layout) in
            self.children.iter().enumerate().zip(layout.children())
        {
            if !live.contains(index) {
                continue;
            }
            child.as_widget().draw(
                &tree.children[index],
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
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
        let live = tree.state.downcast_ref::<State>().live.clone();
        self.children
            .iter()
            .enumerate()
            .zip(layout.children())
            .filter(|((index, _), _)| live.contains(*index))
            .map(|((index, child), child_layout)| {
                child.as_widget().mouse_interaction(
                    &tree.children[index],
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let live = tree.state.downcast_ref::<State>().live.clone();
        operation.custom(None, layout.bounds(), tree.state.downcast_mut::<State>());
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((index, child), child_layout) in
                self.children.iter_mut().enumerate().zip(layout.children())
            {
                if !live.contains(index) {
                    continue;
                }
                child.as_widget_mut().operate(
                    &mut tree.children[index],
                    child_layout,
                    renderer,
                    operation,
                );
            }
        });
        // The operation may well have been the one that moved focus.
        self.track_focus(tree, layout, renderer);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let live = tree.state.downcast_ref::<State>().live.clone();
        // Only measured children have a layout they actually produced, so only
        // they can be asked for an overlay.
        let overlays: Vec<_> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .enumerate()
            .filter(|(index, _)| live.contains(*index))
            .filter_map(|(_, ((child, state), child_layout))| {
                child
                    .as_widget_mut()
                    .overlay(state, child_layout, renderer, viewport, translation)
            })
            .collect();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::runtime::UserInterface;
    use iced_test::runtime::user_interface;
    use std::cell::Cell;
    use std::rc::Rc;

    /// A fixed-height child that records every time it is laid out — which is
    /// where a real row would shape its text.
    struct Counted {
        layouts: Rc<Cell<usize>>,
        height: f32,
    }

    impl Widget<(), iced::Theme, iced_test::renderer::Renderer> for Counted {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fill, Length::Fixed(self.height))
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &iced_test::renderer::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            self.layouts.set(self.layouts.get() + 1);
            layout::Node::new(Size::new(limits.max().width, self.height))
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut iced_test::renderer::Renderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    /// A fixed-height child that records whether it was actually asked to
    /// draw inside the scrollable's translated viewport.
    struct Visible {
        draws: Rc<Cell<usize>>,
        height: f32,
    }

    impl Widget<(), iced::Theme, iced_test::renderer::Renderer> for Visible {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fill, Length::Fixed(self.height))
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &iced_test::renderer::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::new(limits.max().width, self.height))
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut iced_test::renderer::Renderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            layout: Layout<'_>,
            _cursor: mouse::Cursor,
            viewport: &Rectangle,
        ) {
            if layout.bounds().intersects(viewport) {
                self.draws.set(self.draws.get() + 1);
            }
        }
    }

    /// A fixed-height child that can hold keyboard focus, like the hover
    /// buttons Ice puts on a chat row.
    struct FocusableRow {
        height: f32,
    }

    #[derive(Default)]
    struct FocusState {
        focused: bool,
    }

    impl Focusable for FocusState {
        fn is_focused(&self) -> bool {
            self.focused
        }

        fn focus(&mut self) {
            self.focused = true;
        }

        fn unfocus(&mut self) {
            self.focused = false;
        }
    }

    impl Widget<(), iced::Theme, iced_test::renderer::Renderer> for FocusableRow {
        fn tag(&self) -> tree::Tag {
            tree::Tag::of::<FocusState>()
        }

        fn state(&self) -> tree::State {
            tree::State::new(FocusState::default())
        }

        fn size(&self) -> Size<Length> {
            Size::new(Length::Fill, Length::Fixed(self.height))
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &iced_test::renderer::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::new(limits.max().width, self.height))
        }

        fn operate(
            &mut self,
            tree: &mut Tree,
            layout: Layout<'_>,
            _renderer: &iced_test::renderer::Renderer,
            operation: &mut dyn Operation,
        ) {
            operation.focusable(
                None,
                layout.bounds(),
                tree.state.downcast_mut::<FocusState>(),
            );
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut iced_test::renderer::Renderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    fn headless_renderer() -> iced_test::renderer::Renderer {
        use iced::advanced::renderer::Headless as _;

        iced_test::futures::futures::executor::block_on(iced_test::renderer::Renderer::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            None,
        ))
        .expect("headless renderer")
    }

    /// Spacing has to live in the geometry, not just between drawn children:
    /// an offscreen child's slot is sized from the estimate plus its gap, so
    /// the scrollbar and the mounted window agree with what is drawn.
    #[test]
    fn spacing_sits_between_children_and_in_the_total_height() {
        const COUNT: usize = 10;
        const ROW: f32 = 20.0;
        const GAP: f32 = 6.0;
        let layouts = Rc::new(Cell::new(0));
        let children: Vec<Element<'_, (), iced::Theme, iced_test::renderer::Renderer>> = (0..COUNT)
            .map(|_| {
                Element::new(Counted {
                    layouts: Rc::clone(&layouts),
                    height: ROW,
                })
            })
            .collect();

        let renderer = headless_renderer();
        let mut widget = virtual_children(children, ROW).spacing(GAP);
        let mut tree =
            Tree::new(&widget as &dyn Widget<(), iced::Theme, iced_test::renderer::Renderer>);
        // Tall enough that every child mounts, so these are measured heights.
        let node = widget.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(240.0, 1_000.0)),
        );

        assert_eq!(
            layouts.get(),
            COUNT,
            "every child fits, so every child is laid out"
        );
        for (index, child) in node.children().iter().enumerate() {
            assert_eq!(
                child.bounds().y,
                index as f32 * (ROW + GAP),
                "child {index} sits after its predecessors and their gaps"
            );
        }
        assert_eq!(
            node.size().height,
            COUNT as f32 * ROW + (COUNT - 1) as f32 * GAP,
            "the total counts nine gaps for ten children, not ten"
        );
    }

    /// Focus is state, and leaving it in a child nothing can reach is worse
    /// than not drawing that child: `focus_next` counts focusables before it
    /// moves focus, so a pass that misses the focused one concludes nothing is
    /// focused and focuses a second widget. Whichever child holds focus stays
    /// measured wherever it drifts to.
    #[test]
    fn the_focused_child_stays_reachable_after_scrolling_out_of_the_window() {
        const COUNT: usize = 100;
        const ROW: f32 = 20.0;
        let children: Vec<Element<'_, (), iced::Theme, iced_test::renderer::Renderer>> = (0..COUNT)
            .map(|_| Element::new(FocusableRow { height: ROW }))
            .collect();

        let renderer = headless_renderer();
        let mut widget = virtual_children(children, ROW);
        let mut tree =
            Tree::new(&widget as &dyn Widget<(), iced::Theme, iced_test::renderer::Renderer>);
        let limits = layout::Limits::new(Size::ZERO, Size::new(240.0, 100.0));
        let node = widget.layout(&mut tree, &renderer, &limits);

        // A click focuses a child from inside its own `update`, so put the flag
        // there the same way, without any operation having seen it.
        tree.children[0].state.downcast_mut::<FocusState>().focused = true;

        // Scroll far past it, then relayout the way the invalidation would.
        let scrolled = Rectangle::new(iced::Point::new(0.0, 600.0), Size::new(240.0, 100.0));
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        widget.update(
            &mut tree,
            &Event::Mouse(mouse::Event::CursorLeft),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &scrolled,
        );
        assert!(
            shell.is_layout_invalid(),
            "a moved viewport re-opens layout"
        );
        drop(shell);
        let node = widget.layout(&mut tree, &renderer, &limits);

        let live = tree.state.downcast_ref::<State>().live.clone();
        assert!(
            !live.mounted.contains(&0),
            "the window has moved off child 0, so this proves nothing otherwise"
        );
        assert_eq!(
            live.focused,
            Some(0),
            "the focused child is measured wherever it sits"
        );

        // The real payoff: a focus operation still reaches it. Today's failure
        // is that it does not, so the child keeps `focused` while a focus
        // operation lights up a second widget somewhere else.
        let mut unfocus = iced::advanced::widget::operation::focusable::unfocus::<()>();
        widget.operate(&mut tree, Layout::new(&node), &renderer, &mut unfocus);
        assert!(
            !tree.children[0].state.downcast_ref::<FocusState>().focused,
            "an offscreen focused child must still answer focus operations"
        );
        assert_eq!(
            tree.state.downcast_ref::<State>().live.focused,
            None,
            "and once it loses focus it stops being measured"
        );
    }

    /// Keys move per-row state to wherever the row went. The list this exists
    /// for grows on **top**, and under index diffing a prepend hands every row
    /// below the new one its predecessor's widget state, its measured height,
    /// and — worst — the focus flag that decides who stays measured. Each of
    /// those is asserted separately below, because the height remap and the
    /// tree remap are different code answering the same question.
    #[test]
    fn per_row_state_follows_the_key_when_a_row_is_prepended() {
        type Row = Element<'static, (), iced::Theme, iced_test::renderer::Renderer>;

        // Distinct heights, so a height that stayed on its index is a
        // different number rather than a coincidence.
        fn height_of(key: u64) -> f32 {
            key as f32 * 11.0
        }

        fn rows(keys: &[u64]) -> Vec<(u64, Row)> {
            keys.iter()
                .map(|key| {
                    (
                        *key,
                        Element::new(FocusableRow {
                            height: height_of(*key),
                        }),
                    )
                })
                .collect()
        }

        let renderer = headless_renderer();
        let mut widget = virtual_keyed_children(rows(&[1, 2, 3]), 11.0);
        let mut tree =
            Tree::new(&widget as &dyn Widget<(), iced::Theme, iced_test::renderer::Renderer>);
        // Tall enough that every row is measured for real.
        let _ = widget.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(240.0, 1_000.0)),
        );
        // Focus the middle row the way a click does: from inside its own
        // `update`, with the widget's own record of it set as `track_focus`
        // would have.
        tree.children[1].state.downcast_mut::<FocusState>().focused = true;
        tree.state.downcast_mut::<State>().live.focused = Some(1);

        // A newer row arrives on top.
        virtual_keyed_children(rows(&[9, 1, 2, 3]), 11.0).diff(&mut tree);

        assert_eq!(
            tree.state.downcast_ref::<State>().measured,
            vec![None, Some(11.0), Some(22.0), Some(33.0)],
            "measured heights follow their key, and the new row has none yet"
        );
        assert!(
            tree.children[2].state.downcast_ref::<FocusState>().focused,
            "the focused row's widget state moved down with its key"
        );
        assert!(
            !tree.children[1].state.downcast_ref::<FocusState>().focused,
            "and did not stay behind on the index it used to sit at"
        );
        assert_eq!(
            tree.state.downcast_ref::<State>().live.focused,
            Some(2),
            "so the row still measured off-window is the one actually focused"
        );
    }

    #[test]
    fn unchanged_keys_still_reconcile_child_tags_and_keep_row_state() {
        type Row = Element<'static, (), iced::Theme, iced_test::renderer::Renderer>;

        let renderer = headless_renderer();
        let initial: Vec<(u64, Row)> = (1..=3)
            .map(|key| {
                (
                    key,
                    Element::new(FocusableRow {
                        height: key as f32 * 10.0,
                    }),
                )
            })
            .collect();
        let mut widget = virtual_keyed_children(initial, 10.0);
        let mut tree =
            Tree::new(&widget as &dyn Widget<(), iced::Theme, iced_test::renderer::Renderer>);
        let _ = widget.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(240.0, 1_000.0)),
        );
        tree.children[2].state.downcast_mut::<FocusState>().focused = true;
        tree.state.downcast_mut::<State>().live.focused = Some(2);
        let measured = tree.state.downcast_ref::<State>().measured.clone();

        let replacement_layouts = Rc::new(Cell::new(0));
        let changed: Vec<(u64, Row)> = vec![
            (1, Element::new(FocusableRow { height: 10.0 })),
            (
                2,
                Element::new(Counted {
                    layouts: replacement_layouts,
                    height: 200.0,
                }),
            ),
            (3, Element::new(FocusableRow { height: 30.0 })),
        ];
        let changed = virtual_keyed_children(changed, 12.0).spacing(3.0);
        changed.diff(&mut tree);

        assert_eq!(
            tree.children[1].tag,
            tree::Tag::stateless(),
            "an unchanged row key must not suppress a changed widget tag"
        );
        assert!(
            matches!(tree.children[1].state, tree::State::None),
            "the replacement widget must receive its own fresh state"
        );
        let state = tree.state.downcast_ref::<State>();
        assert_eq!(
            state.measured, measured,
            "measurements stay with unchanged keys"
        );
        assert_eq!(state.live.focused, Some(2), "focus stays on the same key");
        assert_eq!(state.estimated_height, 12.0);
        assert_eq!(state.spacing, 3.0);
        assert!(
            tree.children[2].state.downcast_ref::<FocusState>().focused,
            "the unchanged focused child keeps its widget state"
        );
    }

    /// The whole point: a thousand children, and only a viewport's worth ever
    /// reaches `layout` — where the text of a real row would be shaped.
    #[test]
    fn only_the_visible_children_are_laid_out() {
        const COUNT: usize = 1_000;
        const ROW: f32 = 20.0;
        let layouts = Rc::new(Cell::new(0));
        let children: Vec<Element<'_, (), iced::Theme, iced_test::renderer::Renderer>> = (0..COUNT)
            .map(|_| {
                Element::new(Counted {
                    layouts: Rc::clone(&layouts),
                    height: ROW,
                })
            })
            .collect();

        let mut renderer = headless_renderer();

        let ui = UserInterface::build(
            virtual_children(children, ROW),
            Size::new(240.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        drop(ui);

        let laid_out = layouts.get();
        assert!(
            laid_out > 0,
            "the visible children still have to be laid out"
        );
        assert!(
            laid_out <= 32,
            "a 100px viewport over 20px rows should reach a handful of children, not {laid_out} of {COUNT}"
        );
    }

    /// Iced keeps consecutive wheel events in one scroll transaction and
    /// deliberately stops forwarding them to the scrollable's content. A
    /// virtual column must still move its mounted window during that burst;
    /// otherwise a fast trackpad reversal can translate every mounted row out
    /// of the viewport and draw one or more empty frames.
    #[test]
    fn rapid_wheel_scrolling_never_runs_past_the_mounted_rows() {
        const COUNT: usize = 100;
        const ROW: f32 = 20.0;
        let draws = Rc::new(Cell::new(0));
        let children: Vec<Element<'_, (), iced::Theme, iced_test::renderer::Renderer>> = (0..COUNT)
            .map(|_| {
                Element::new(Visible {
                    draws: Rc::clone(&draws),
                    height: ROW,
                })
            })
            .collect();
        let content = virtual_children(children, ROW);
        let mut renderer = headless_renderer();
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut messages = Vec::new();
        let mut ui = UserInterface::build(
            crate::virtual_scroll(iced::widget::scrollable(content)),
            Size::new(240.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let cursor = mouse::Cursor::Available(iced::Point::new(120.0, 50.0));
        let wheels = (0..20)
            .map(|_| {
                Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -100.0 },
                })
            })
            .collect::<Vec<_>>();
        let _ = ui.update(
            &wheels,
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );

        ui.draw(
            &mut renderer,
            &iced::Theme::Light,
            &renderer::Style::default(),
            cursor,
        );
        assert!(
            draws.get() > 0,
            "a rapid wheel burst scrolled beyond every mounted virtual row"
        );
    }
}
