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

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// Extra rows kept live on each side of the viewport so a scroll of a row or
/// two reveals something already measured.
const OVERSCAN_ROWS: usize = 4;

/// Lays out only the visible slice of `children`, estimating the rest at
/// `estimated_height` until they are measured.
///
/// The estimate only has to be the right order of magnitude: every child that
/// enters the viewport is measured for real and remembered, so the scrollbar
/// converges as the reader moves.
pub fn virtual_children<'a, Message, Theme, Renderer>(
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    estimated_height: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    Element::new(VirtualChildren {
        children,
        estimated_height: estimated_height.max(1.0),
    })
}

struct VirtualChildren<'a, Message, Theme, Renderer> {
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    estimated_height: f32,
}

#[derive(Default)]
struct State {
    /// Real heights for children that have been laid out, `None` until then.
    measured: Vec<Option<f32>>,
    /// The visible region the last pass worked against.
    viewport: Rectangle,
    /// The slice laid out for real, so draw and events agree with layout.
    mounted: std::ops::Range<usize>,
}

impl State {
    fn height_of(&self, index: usize, estimate: f32) -> f32 {
        self.measured
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(estimate)
    }

    /// Row tops, from measurements where they exist and the estimate elsewhere.
    fn tops(&self, count: usize, estimate: f32) -> Vec<f32> {
        let mut tops = Vec::with_capacity(count);
        let mut running = 0.0;
        for index in 0..count {
            tops.push(running);
            running += self.height_of(index, estimate);
        }
        tops
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
            ..State::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        let state = tree.state.downcast_mut::<State>();
        state.measured.resize(self.children.len(), None);
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
        let tops = state.tops(count, self.estimated_height);

        // Before the first draw the viewport is unknown; fill a screen's worth
        // from the top rather than the whole document.
        let visible_top = state.viewport.y;
        let visible_height = if state.viewport.height > 0.0 {
            state.viewport.height
        } else {
            limits.max().height.max(self.estimated_height)
        };
        let first = tops
            .partition_point(|top| *top <= visible_top)
            .saturating_sub(1);
        let last = tops.partition_point(|top| *top < visible_top + visible_height);
        let mounted = first.saturating_sub(OVERSCAN_ROWS)..(last + OVERSCAN_ROWS).min(count);

        let width = limits.max().width;
        let mut nodes = Vec::with_capacity(count);
        let mut running = 0.0;
        for index in 0..count {
            let node = if mounted.contains(&index) {
                let child_limits =
                    layout::Limits::new(Size::new(0.0, 0.0), Size::new(width, f32::INFINITY));
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
            running += height;
        }

        state.mounted = mounted;
        layout::Node::with_children(Size::new(width, running), nodes)
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
        let state = tree.state.downcast_mut::<State>();
        // Scrolling moves the viewport without changing anything this widget
        // owns, so nothing else would re-open layout — and until it does, the
        // rows scrolled into view have never been measured.
        if state.viewport != *viewport {
            state.viewport = *viewport;
            shell.invalidate_layout();
        }

        let mounted = state.mounted.clone();
        for ((index, child), child_layout) in
            self.children.iter_mut().enumerate().zip(layout.children())
        {
            if !mounted.contains(&index) {
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
        let mounted = tree.state.downcast_ref::<State>().mounted.clone();
        for ((index, child), child_layout) in
            self.children.iter().enumerate().zip(layout.children())
        {
            if !mounted.contains(&index) {
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
        let mounted = tree.state.downcast_ref::<State>().mounted.clone();
        self.children
            .iter()
            .enumerate()
            .zip(layout.children())
            .filter(|((index, _), _)| mounted.contains(index))
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
        let mounted = tree.state.downcast_ref::<State>().mounted.clone();
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((index, child), child_layout) in
                self.children.iter_mut().enumerate().zip(layout.children())
            {
                if !mounted.contains(&index) {
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
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mounted = tree.state.downcast_ref::<State>().mounted.clone();
        // Only mounted children have a layout they actually produced, so only
        // they can be asked for an overlay.
        let overlays: Vec<_> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .enumerate()
            .filter(|(index, _)| mounted.contains(index))
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

    fn headless_renderer() -> iced_test::renderer::Renderer {
        use iced::advanced::renderer::Headless as _;

        iced_test::futures::futures::executor::block_on(iced_test::renderer::Renderer::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            None,
        ))
        .expect("headless renderer")
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
}
