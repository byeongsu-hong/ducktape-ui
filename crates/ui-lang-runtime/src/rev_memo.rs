//! The layout memo the compiler inserts at a component use: the element is
//! still built on every pass, and the layout walk below it is skipped while
//! the key — the revisions of everything the use reads — and the `Limits`
//! hold. Unlike `memo_lazy` it caches no element, so a body that borrows app
//! state lives under it unchanged; what it saves is the walk, which is most
//! of a frame.

use crate::memo_lazy::{MemoLayout, child_layout, shallow};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{self, Clipboard, Shell, overlay};
use iced::{Element, Event, Length, Rectangle, Size, Vector, mouse};
use rustc_hash::FxHasher;
use std::cell::Cell;
use std::hash::{Hash, Hasher as _};

thread_local! {
    static COUNTS: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}

/// `(hits, misses)` of every memo's `layout` on this thread since the last
/// call, which resets them; a probe reads it around one frame.
pub fn take_rev_memo_counts() -> (u64, u64) {
    COUNTS.with(|counts| counts.replace((0, 0)))
}

fn count(hit: bool) {
    COUNTS.with(|counts| {
        let (hits, misses) = counts.get();
        counts.set(if hit {
            (hits + 1, misses)
        } else {
            (hits, misses + 1)
        });
    });
}

pub struct RevMemo<'a, Message, Theme, Renderer> {
    site: u64,
    key: u64,
    content: Element<'a, Message, Theme, Renderer>,
}

/// Wraps `content` in a layout memo keyed by `key`; `site` tells two uses
/// that can land in the same tree slot (the arms of a `match`) apart.
pub fn rev_memo<'a, Message, Theme, Renderer>(
    site: u64,
    key: impl Hash,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> RevMemo<'a, Message, Theme, Renderer> {
    let mut hasher = FxHasher::default();
    key.hash(&mut hasher);
    RevMemo {
        site,
        key: hasher.finish(),
        content: content.into(),
    }
}

struct State {
    site: u64,
    key: u64,
    layout: MemoLayout,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for RevMemo<'_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            site: self.site,
            key: self.key,
            layout: MemoLayout::none(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        // Same site, same revisions: the element below is the one this tree
        // was last walked against, node for node, so the walk has nothing
        // to find. The compiler keeps anything whose `diff` must run every
        // pass — a `lazy`, which hands its cached element to each new
        // instance there — out of a memoized body.
        if state.site == self.site && state.key == self.key {
            return;
        }
        if state.site != self.site {
            state.site = self.site;
            state.layout.clear();
        }
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
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        if state.key == self.key {
            if let Some(node) = state.layout.hit(limits) {
                count(true);
                return shallow(node);
            }
        } else {
            state.key = self.key;
            state.layout.clear();
        }
        count(false);
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut children[0], renderer, limits);
        let handed_up = shallow(&node);
        state.layout.store(*limits, node);
        handed_up
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        // `BustMemoLayouts` drops the memo when a virtual window below
        // re-aims; the relayout that follows recomputes through here.
        operation.custom(None, layout.bounds(), &mut state.layout);
        if state.layout.is_empty() {
            return;
        }
        let layout = child_layout(&state.layout, layout);
        self.content
            .as_widget_mut()
            .operate(&mut children[0], layout, renderer, operation);
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
        let Tree {
            state, children, ..
        } = tree;
        let layout = child_layout(&state.downcast_ref::<State>().layout, layout);
        self.content.as_widget_mut().update(
            &mut children[0],
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
        let layout = child_layout(&tree.state.downcast_ref::<State>().layout, layout);
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
        let layout = child_layout(&tree.state.downcast_ref::<State>().layout, layout);
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
        let Tree {
            state, children, ..
        } = tree;
        let layout = child_layout(&state.downcast_ref::<State>().layout, layout);
        self.content.as_widget_mut().overlay(
            &mut children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<RevMemo<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: advanced::Renderer + 'a,
{
    fn from(memo: RevMemo<'a, Message, Theme, Renderer>) -> Self {
        Element::new(memo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    type TestRenderer = iced_test::renderer::Renderer;

    struct Measured {
        laid_out: Rc<Cell<usize>>,
    }

    impl Widget<(), iced::Theme, TestRenderer> for Measured {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fixed(10.0), Length::Fixed(10.0))
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &TestRenderer,
            _limits: &layout::Limits,
        ) -> layout::Node {
            self.laid_out.set(self.laid_out.get() + 1);
            layout::Node::new(Size::new(10.0, 10.0))
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut TestRenderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    fn memo(
        site: u64,
        key: u64,
        laid_out: &Rc<Cell<usize>>,
    ) -> RevMemo<'static, (), iced::Theme, TestRenderer> {
        rev_memo(
            site,
            key,
            Element::new(Measured {
                laid_out: laid_out.clone(),
            }),
        )
    }

    fn renderer() -> TestRenderer {
        use iced::advanced::renderer::Headless as _;
        iced_test::futures::futures::executor::block_on(TestRenderer::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            None,
        ))
        .expect("headless renderer")
    }

    /// One pass: diff the fresh element into the tree, then lay it out.
    fn pass(
        widget: &mut RevMemo<'static, (), iced::Theme, TestRenderer>,
        tree: &mut Tree,
        renderer: &TestRenderer,
        limits: &layout::Limits,
    ) {
        widget.diff(tree);
        widget.layout(tree, renderer, limits);
    }

    /// A child that counts how often it is diffed, which is what the walk
    /// below a memo costs on a frame whose layout is already cached.
    struct Diffed {
        diffs: Rc<Cell<usize>>,
    }

    impl Widget<(), iced::Theme, TestRenderer> for Diffed {
        fn diff(&self, _tree: &mut Tree) {
            self.diffs.set(self.diffs.get() + 1);
        }

        fn size(&self) -> Size<Length> {
            Size::new(Length::Fixed(10.0), Length::Fixed(10.0))
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &TestRenderer,
            _limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::new(10.0, 10.0))
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut TestRenderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    #[test]
    fn a_held_key_skips_the_diff_below_and_a_moved_one_walks_it() {
        let renderer = renderer();
        let diffs = Rc::new(Cell::new(0));
        let limits = layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0));
        let memo = |site: u64, key: u64| -> RevMemo<'static, (), iced::Theme, TestRenderer> {
            rev_memo(
                site,
                key,
                Element::new(Diffed {
                    diffs: diffs.clone(),
                }),
            )
        };

        let mut first = memo(1, 10);
        let mut tree = Tree::new(&first as &dyn Widget<(), iced::Theme, TestRenderer>);
        pass(&mut first, &mut tree, &renderer, &limits);
        assert_eq!(
            diffs.get(),
            0,
            "a tree built from the element needs no diff"
        );

        let mut same = memo(1, 10);
        pass(&mut same, &mut tree, &renderer, &limits);
        assert_eq!(diffs.get(), 0, "a held key must not diff the child");

        let mut changed = memo(1, 11);
        pass(&mut changed, &mut tree, &renderer, &limits);
        assert_eq!(diffs.get(), 1, "a moved revision must diff the child");

        let mut elsewhere = memo(2, 11);
        pass(&mut elsewhere, &mut tree, &renderer, &limits);
        assert_eq!(diffs.get(), 2, "another site's element must diff the child");
    }

    #[test]
    fn an_unchanged_key_serves_the_cached_node_and_a_changed_key_relays_out() {
        let renderer = renderer();
        let laid_out = Rc::new(Cell::new(0));
        let limits = layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0));

        let mut first = memo(1, 10, &laid_out);
        let mut tree = Tree::new(&first as &dyn Widget<(), iced::Theme, TestRenderer>);
        pass(&mut first, &mut tree, &renderer, &limits);
        assert_eq!(laid_out.get(), 1);

        let mut same = memo(1, 10, &laid_out);
        pass(&mut same, &mut tree, &renderer, &limits);
        assert_eq!(laid_out.get(), 1, "a held key must not walk the child");

        let mut changed = memo(1, 11, &laid_out);
        pass(&mut changed, &mut tree, &renderer, &limits);
        assert_eq!(laid_out.get(), 2, "a moved revision must relay out");
    }

    #[test]
    fn limits_key_the_slots_and_a_flex_parents_passes_all_hit() {
        let renderer = renderer();
        let laid_out = Rc::new(Cell::new(0));
        let measure = layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0));
        let stretch = layout::Limits::new(Size::new(50.0, 0.0), Size::new(100.0, 100.0));

        let mut widget = memo(1, 10, &laid_out);
        let mut tree = Tree::new(&widget as &dyn Widget<(), iced::Theme, TestRenderer>);
        pass(&mut widget, &mut tree, &renderer, &measure);
        widget.layout(&mut tree, &renderer, &stretch);
        assert_eq!(laid_out.get(), 2);

        let mut next = memo(1, 10, &laid_out);
        pass(&mut next, &mut tree, &renderer, &measure);
        next.layout(&mut tree, &renderer, &stretch);
        assert_eq!(laid_out.get(), 2, "both limits of the last frame must hit");
    }

    #[test]
    fn another_site_in_the_same_slot_starts_cold() {
        let renderer = renderer();
        let laid_out = Rc::new(Cell::new(0));
        let limits = layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0));

        let mut arm_a = memo(1, 10, &laid_out);
        let mut tree = Tree::new(&arm_a as &dyn Widget<(), iced::Theme, TestRenderer>);
        pass(&mut arm_a, &mut tree, &renderer, &limits);

        let mut arm_b = memo(2, 10, &laid_out);
        pass(&mut arm_b, &mut tree, &renderer, &limits);
        assert_eq!(
            laid_out.get(),
            2,
            "a `match` arm switching to another use with an equal key must not inherit its node"
        );
    }
}
