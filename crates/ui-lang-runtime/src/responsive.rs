//! `responsive` builds its subtree from the size it is given, and that size is
//! only known in `layout` — so the subtree is built there, on every pass.
//! iced lays the whole tree out again inside `update` whenever a widget
//! invalidates layout (a scroll, a keystroke), which with iced's own widget
//! builds the subtree twice in one frame. The app's state cannot have moved
//! between two layout passes of one element instance, so a pass that sees the
//! size it already built for reuses that subtree instead.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector, mouse};

/// A widget that builds its content from the size its parent gives it.
pub fn responsive<'a, Message, Theme, Renderer>(
    view: impl Fn(Size) -> Element<'a, Message, Theme, Renderer> + 'a,
) -> Responsive<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    Responsive::new(view)
}

thread_local! {
    static BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times a `responsive` built its subtree since the last call —
/// what a probe prints per frame to show a relayout no longer rebuilds.
pub fn take_responsive_builds() -> u64 {
    BUILDS.with(|builds| builds.replace(0))
}

pub struct Responsive<'a, Message, Theme, Renderer> {
    view: Box<dyn Fn(Size) -> Element<'a, Message, Theme, Renderer> + 'a>,
    width: Length,
    height: Length,
    /// The subtree and the size it was built for; `None` before the first
    /// layout of this instance.
    content: Option<(Size, Element<'a, Message, Theme, Renderer>)>,
}

impl<'a, Message, Theme, Renderer> Responsive<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn new(view: impl Fn(Size) -> Element<'a, Message, Theme, Renderer> + 'a) -> Self {
        Self {
            view: Box::new(view),
            width: Length::Fill,
            height: Length::Fill,
            content: None,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    fn content(&self) -> &Element<'a, Message, Theme, Renderer> {
        &self
            .content
            .as_ref()
            .expect("`layout` runs before any phase that walks the tree")
            .1
    }

    fn content_mut(&mut self) -> &mut Element<'a, Message, Theme, Renderer> {
        &mut self
            .content
            .as_mut()
            .expect("`layout` runs before any phase that walks the tree")
            .1
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Responsive<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// The child tree is diffed in `layout`, against the content built for
    /// the size known there; the default `diff` would clear it here and
    /// hand every pass a fresh subtree with no memo to hit.
    fn diff(&self, _tree: &mut Tree) {}

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.max();
        if self
            .content
            .as_ref()
            .is_none_or(|(built, _)| *built != size)
        {
            let content = (self.view)(size);
            BUILDS.with(|builds| builds.set(builds.get() + 1));
            tree.diff_children(std::slice::from_ref(&content));
            self.content = Some((size, content));
        }
        let node = self.content_mut().as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.loose(),
        );
        let size = limits.resolve(self.width, self.height, node.size());
        layout::Node::with_children(size, vec![node])
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
        self.content_mut().as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        self.content().as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
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
        self.content().as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content_mut().as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            operation,
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
        self.content_mut().as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Responsive<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(responsive: Responsive<'a, Message, Theme, Renderer>) -> Self {
        Self::new(responsive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    type TestRenderer = iced_test::renderer::Renderer;

    fn renderer() -> TestRenderer {
        use iced::advanced::renderer::Headless as _;
        iced_test::futures::futures::executor::block_on(TestRenderer::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            None,
        ))
        .expect("headless renderer")
    }

    #[test]
    fn the_child_tree_survives_the_diff_a_parent_runs_each_pass() {
        let renderer = renderer();
        let mut first: Responsive<'_, (), iced::Theme, TestRenderer> =
            responsive(|_| iced::widget::space().width(10.0).height(10.0).into());
        let mut tree = Tree::new(&first as &dyn Widget<(), iced::Theme, TestRenderer>);
        let limits = layout::Limits::new(Size::ZERO, Size::new(200.0, 100.0));
        first.layout(&mut tree, &renderer, &limits);
        assert_eq!(tree.children.len(), 1);

        let next: Responsive<'_, (), iced::Theme, TestRenderer> =
            responsive(|_| iced::widget::space().width(10.0).height(10.0).into());
        next.diff(&mut tree);
        assert_eq!(
            tree.children.len(),
            1,
            "the next pass must find the subtree the last one built"
        );
    }

    #[test]
    fn a_second_layout_at_the_same_size_reuses_the_subtree_it_built() {
        let renderer = renderer();
        let builds = Rc::new(Cell::new(0));
        let counted = builds.clone();
        let mut widget: Responsive<'_, (), iced::Theme, TestRenderer> = responsive(move |_| {
            counted.set(counted.get() + 1);
            iced::widget::space().width(10.0).height(10.0).into()
        });
        let mut tree = Tree::new(&widget as &dyn Widget<(), iced::Theme, TestRenderer>);
        let wide = layout::Limits::new(Size::ZERO, Size::new(200.0, 100.0));
        let narrow = layout::Limits::new(Size::ZERO, Size::new(120.0, 100.0));

        widget.layout(&mut tree, &renderer, &wide);
        assert_eq!(builds.get(), 1);
        widget.layout(&mut tree, &renderer, &wide);
        assert_eq!(
            builds.get(),
            1,
            "a relayout at the same size must not rebuild"
        );
        widget.layout(&mut tree, &renderer, &narrow);
        assert_eq!(builds.get(), 2, "a new size must rebuild");
    }
}
