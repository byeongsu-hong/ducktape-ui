//! UI-thread ownership for a module's parked native subtrees.
use super::{IceElement, Output};
use crate::{MemoParking, MemoParkingHandle};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Event, Length, Rectangle, Size, Vector};

pub(super) fn scope(
    content: IceElement<'static, Output>,
    instance: u64,
    handle: MemoParkingHandle,
) -> IceElement<'static, Output> {
    iced::Element::new(Scope {
        content,
        instance,
        handle,
    })
}

struct Scope {
    content: IceElement<'static, Output>,
    instance: u64,
    handle: MemoParkingHandle,
}

struct State {
    instance: u64,
    owner: MemoParking,
}

impl Scope {
    fn fresh_state(&self) -> State {
        let owner = MemoParking::default();
        self.handle.attach(&owner);
        State {
            instance: self.instance,
            owner,
        }
    }
}

impl Widget<Output, iced::Theme, iced::Renderer> for Scope {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        // Iced initializes state before children: lazy child state can now
        // resolve this render's weak slot to the instance-owned lot.
        tree::State::new(self.fresh_state())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.content.as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.instance != self.instance {
            // Release the old owner before its mounted children, so neither
            // parked nor mounted nested memos can outlive the instance.
            *state = self.fresh_state();
            tree.children = self.children();
        } else {
            self.handle.attach(&state.owner);
            tree.children[0].diff(self.content.as_widget());
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
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
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
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Output>,
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

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
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

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Output, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    fn view(
        instance: u64,
        shown: bool,
        builds: Rc<Cell<u32>>,
        lease: Rc<()>,
    ) -> IceElement<'static, Output> {
        let handle = MemoParkingHandle::unbound();
        let content = if shown {
            crate::memo_lazy(
                1,
                move |_: &u32| {
                    builds.set(builds.get() + 1);
                    let lease = lease.clone();
                    IceElement::from(iced::widget::button("native view").on_press_with(move || {
                        let _keep_alive = &lease;
                        Output::Ignore
                    }))
                },
                1,
                "row",
            )
            .parking(handle.clone())
            .into()
        } else {
            iced::widget::text("hidden").into()
        };
        scope(content, instance, handle)
    }

    #[test]
    fn module_scope_reuses_inner_unmounts_and_releases_replaced_instances() {
        let builds = Rc::new(Cell::new(0));
        let old_lease = Rc::new(());
        let weak = Rc::downgrade(&old_lease);
        let first = view(1, true, builds.clone(), old_lease.clone());
        let mut tree = Tree::new(first.as_widget());
        drop(first);
        let hidden = view(1, false, builds.clone(), old_lease.clone());
        tree.diff(hidden.as_widget());
        drop(hidden);
        let again = view(1, true, builds.clone(), old_lease);
        tree.diff(again.as_widget());
        drop(again);
        assert_eq!(builds.get(), 1, "inner remount must reclaim native content");
        assert!(weak.upgrade().is_some());

        let next_lease = Rc::new(());
        let next_weak = Rc::downgrade(&next_lease);
        let replacement = view(2, true, builds.clone(), next_lease);
        tree.diff(replacement.as_widget());
        drop(replacement);
        assert_eq!(
            builds.get(),
            2,
            "restart must construct a fresh native view"
        );
        assert!(
            weak.upgrade().is_none(),
            "restart releases the old instance's resources"
        );
        assert!(next_weak.upgrade().is_some());
        drop(tree);
        assert!(
            next_weak.upgrade().is_none(),
            "unmounting the module releases its resources"
        );
    }
}
