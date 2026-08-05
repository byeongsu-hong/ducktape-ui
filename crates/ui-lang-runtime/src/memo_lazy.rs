//! [`iced::widget::Lazy`] with LAYOUT memoization.
//!
//! iced's `Lazy` caches the built element while its dependency hash is
//! unchanged — but every layout pass still re-walks the cached subtree. In a
//! deep list (a chat stream of `lazy` rows) that walk dominates: profiling the
//! ducktape console showed ~150µs of layout per cached row per pass, so one
//! keystroke anywhere in the window re-laid a 150-row stream for ~23ms while
//! `view` itself cost ~1ms. This fork memoizes the layout node beside the
//! cached element: while the dependency hash AND the incoming `Limits` are
//! unchanged, `layout()` returns a clone of the stored node without touching
//! the subtree.
//!
//! Soundness rides on the contract `Lazy` already imposes: the content is a
//! pure function of the dependency, so anything that changes what the subtree
//! would lay out must change the dependency hash. Widget-internal state that
//! affects layout (a text editor's wrapped lines, say) would already be stale
//! under `Lazy`'s ELEMENT caching — such widgets don't belong under a lazy
//! boundary, memoized or not. Bounds changes arrive as different `Limits` and
//! recompute; a renderer swap cannot outlive the widget tree that owns the
//! cache.
//!
//! Everything else is a verbatim fork of `iced_widget::lazy` (0.14.2),
//! ouroboros overlay machinery included.
#![allow(clippy::type_complexity)]

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{self, Clipboard, Shell, overlay};
use iced::{Element, Event, Length, Rectangle, Size, Vector, mouse};

use ouroboros::self_referencing;
use rustc_hash::FxHasher;
use std::cell::RefCell;
use std::hash::{Hash, Hasher as _};
use std::rc::Rc;

/// A widget that only rebuilds — and only re-lays — its contents when
/// necessary.
pub struct MemoLazy<'a, Message, Theme, Renderer, Dependency, View> {
    dependency: Dependency,
    view: Box<dyn Fn(&Dependency) -> View + 'a>,
    element: RefCell<Option<Rc<RefCell<Option<Element<'static, Message, Theme, Renderer>>>>>>,
}

/// Creates a [`MemoLazy`] widget with the given dependency and view builder.
pub fn memo_lazy<'a, Message, Theme, Renderer, Dependency, View>(
    dependency: Dependency,
    view: impl Fn(&Dependency) -> View + 'a,
) -> MemoLazy<'a, Message, Theme, Renderer, Dependency, View>
where
    Dependency: Hash + 'a,
    View: Into<Element<'static, Message, Theme, Renderer>>,
{
    MemoLazy {
        dependency,
        view: Box::new(view),
        element: RefCell::new(None),
    }
}

impl<'a, Message, Theme, Renderer, Dependency, View>
    MemoLazy<'a, Message, Theme, Renderer, Dependency, View>
where
    Dependency: Hash + 'a,
    View: Into<Element<'static, Message, Theme, Renderer>>,
{
    fn with_element<T>(&self, f: impl FnOnce(&Element<'_, Message, Theme, Renderer>) -> T) -> T {
        f(self
            .element
            .borrow()
            .as_ref()
            .unwrap()
            .borrow()
            .as_ref()
            .unwrap())
    }

    fn with_element_mut<T>(
        &self,
        f: impl FnOnce(&mut Element<'_, Message, Theme, Renderer>) -> T,
    ) -> T {
        f(self
            .element
            .borrow()
            .as_ref()
            .unwrap()
            .borrow_mut()
            .as_mut()
            .unwrap())
    }
}

struct Internal<Message, Theme, Renderer> {
    element: Rc<RefCell<Option<Element<'static, Message, Theme, Renderer>>>>,
    hash: u64,
    /// The memoized layout for the current `hash`: the `Limits` the node was
    /// computed under and the node itself. `None` after a rebuild.
    layout: Option<(layout::Limits, layout::Node)>,
}

impl<'a, Message, Theme, Renderer, Dependency, View> Widget<Message, Theme, Renderer>
    for MemoLazy<'a, Message, Theme, Renderer, Dependency, View>
where
    View: Into<Element<'static, Message, Theme, Renderer>> + 'static,
    Dependency: Hash + 'a,
    Message: 'static,
    Theme: 'static,
    Renderer: advanced::Renderer + 'static,
{
    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<View>>()
    }

    fn state(&self) -> tree::State {
        let hash = {
            let mut hasher = FxHasher::default();
            self.dependency.hash(&mut hasher);

            hasher.finish()
        };

        let element = Rc::new(RefCell::new(Some((self.view)(&self.dependency).into())));

        (*self.element.borrow_mut()) = Some(element.clone());

        tree::State::new(Internal::<Message, Theme, Renderer> {
            element,
            hash,
            layout: None,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.with_element(|element| vec![Tree::new(element.as_widget())])
    }

    fn diff(&self, tree: &mut Tree) {
        let current = tree
            .state
            .downcast_mut::<Internal<Message, Theme, Renderer>>();

        let new_hash = {
            let mut hasher = FxHasher::default();
            self.dependency.hash(&mut hasher);

            hasher.finish()
        };

        if current.hash != new_hash {
            current.hash = new_hash;
            current.layout = None;

            let element = (self.view)(&self.dependency).into();
            current.element = Rc::new(RefCell::new(Some(element)));

            (*self.element.borrow_mut()) = Some(current.element.clone());
            self.with_element(|element| {
                tree.diff_children(std::slice::from_ref(&element.as_widget()));
            });
        } else {
            (*self.element.borrow_mut()) = Some(current.element.clone());
        }
    }

    fn size(&self) -> Size<Length> {
        self.with_element(|element| element.as_widget().size())
    }

    fn size_hint(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let (state, children) = {
            let Tree {
                state, children, ..
            } = tree;
            (
                state.downcast_mut::<Internal<Message, Theme, Renderer>>(),
                children,
            )
        };

        if let Some((cached_limits, node)) = &state.layout
            && cached_limits == limits
        {
            return node.clone();
        }

        let node = self.with_element_mut(|element| {
            element
                .as_widget_mut()
                .layout(&mut children[0], renderer, limits)
        });
        state.layout = Some((*limits, node.clone()));
        node
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.with_element_mut(|element| {
            element
                .as_widget_mut()
                .operate(&mut tree.children[0], layout, renderer, operation);
        });
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
        self.with_element_mut(|element| {
            element.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.with_element(|element| {
            element.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        })
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
        self.with_element(|element| {
            element.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
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
        let overlay = InnerBuilder {
            cell: self.element.borrow().as_ref().unwrap().clone(),
            element: self
                .element
                .borrow()
                .as_ref()
                .unwrap()
                .borrow_mut()
                .take()
                .unwrap(),
            tree: &mut tree.children[0],
            layout,
            overlay_builder: |element, tree, layout| {
                element
                    .as_widget_mut()
                    .overlay(tree, *layout, renderer, viewport, translation)
                    .map(|overlay| RefCell::new(overlay::Nested::new(overlay)))
            },
        }
        .build();

        #[allow(clippy::redundant_closure_for_method_calls)]
        if overlay.with_overlay(|overlay| overlay.is_some()) {
            Some(overlay::Element::new(Box::new(Overlay(Some(overlay)))))
        } else {
            let heads = overlay.into_heads();

            *self.element.borrow().as_ref().unwrap().borrow_mut() = Some(heads.element);

            None
        }
    }
}

#[self_referencing]
struct Inner<'a, Message: 'a, Theme: 'a, Renderer: 'a> {
    cell: Rc<RefCell<Option<Element<'static, Message, Theme, Renderer>>>>,
    element: Element<'static, Message, Theme, Renderer>,
    tree: &'a mut Tree,
    layout: Layout<'a>,

    #[borrows(mut element, mut tree, layout)]
    #[not_covariant]
    overlay: Option<RefCell<overlay::Nested<'this, Message, Theme, Renderer>>>,
}

struct Overlay<'a, Message, Theme, Renderer>(Option<Inner<'a, Message, Theme, Renderer>>);

impl<Message, Theme, Renderer> Drop for Overlay<'_, Message, Theme, Renderer> {
    fn drop(&mut self) {
        let heads = self.0.take().unwrap().into_heads();
        (*heads.cell.borrow_mut()) = Some(heads.element);
    }
}

impl<Message, Theme, Renderer> Overlay<'_, Message, Theme, Renderer> {
    fn with_overlay_maybe<T>(
        &self,
        f: impl FnOnce(&mut overlay::Nested<'_, Message, Theme, Renderer>) -> T,
    ) -> Option<T> {
        self.0
            .as_ref()
            .unwrap()
            .with_overlay(|overlay| overlay.as_ref().map(|nested| (f)(&mut nested.borrow_mut())))
    }

    fn with_overlay_mut_maybe<T>(
        &mut self,
        f: impl FnOnce(&mut overlay::Nested<'_, Message, Theme, Renderer>) -> T,
    ) -> Option<T> {
        self.0
            .as_mut()
            .unwrap()
            .with_overlay_mut(|overlay| overlay.as_mut().map(|nested| (f)(nested.get_mut())))
    }
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        self.with_overlay_maybe(|overlay| overlay.layout(renderer, bounds))
            .unwrap_or_default()
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let _ = self.with_overlay_maybe(|overlay| {
            overlay.draw(renderer, theme, style, layout, cursor);
        });
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.with_overlay_maybe(|overlay| overlay.mouse_interaction(layout, cursor, renderer))
            .unwrap_or_default()
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let _ = self.with_overlay_mut_maybe(|overlay| {
            overlay.update(event, layout, cursor, renderer, clipboard, shell);
        });
    }
}

impl<'a, Message, Theme, Renderer, Dependency, View>
    From<MemoLazy<'a, Message, Theme, Renderer, Dependency, View>>
    for Element<'a, Message, Theme, Renderer>
where
    View: Into<Element<'static, Message, Theme, Renderer>> + 'static,
    Renderer: advanced::Renderer + 'static,
    Message: 'static,
    Theme: 'static,
    Dependency: Hash + 'a,
{
    fn from(lazy: MemoLazy<'a, Message, Theme, Renderer, Dependency, View>) -> Self {
        Self::new(lazy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestLazy<'a> = MemoLazy<
        'a,
        (),
        iced::Theme,
        iced::Renderer,
        i32,
        Element<'static, (), iced::Theme, iced::Renderer>,
    >;

    fn widget(dependency: i32) -> TestLazy<'static> {
        memo_lazy(dependency, |value: &i32| {
            Element::from(iced::widget::text(value.to_string()))
        })
    }

    fn internal(tree: &mut Tree) -> &mut Internal<(), iced::Theme, iced::Renderer> {
        tree.state.downcast_mut()
    }

    /// The memo's whole contract: a same-dependency diff keeps the cached
    /// layout, a changed dependency drops it (the element rebuild would make
    /// any kept node stale).
    #[test]
    fn diff_keeps_the_layout_memo_only_while_the_dependency_holds() {
        let same = widget(7);
        let mut tree = Tree::new(&same as &dyn Widget<(), iced::Theme, iced::Renderer>);
        assert!(internal(&mut tree).layout.is_none());

        let parked = layout::Node::new(Size::new(10.0, 10.0));
        internal(&mut tree).layout = Some((layout::Limits::NONE, parked.clone()));

        same.diff(&mut tree);
        assert!(
            internal(&mut tree).layout.is_some(),
            "an unchanged dependency must keep the memoized layout"
        );

        let changed = widget(8);
        changed.diff(&mut tree);
        assert!(
            internal(&mut tree).layout.is_none(),
            "a changed dependency rebuilds the element — a kept node would be stale"
        );
    }
}
