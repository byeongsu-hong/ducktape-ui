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

pub(super) fn render(
    key: &str,
    generation: u64,
    content: &ui_lang_wire::Node,
    kept: &super::Kept<'_>,
) -> IceElement<'static, Output> {
    use std::{collections::HashMap, rc::Rc};
    let mut inputs = super::Inputs {
        instance: kept.inputs.instance,
        fields: HashMap::new(),
        editors: HashMap::new(),
    };
    let mut pictures = super::Pictures::default();
    let mut surfaces = super::Surfaces::new();
    fn retain(
        node: &ui_lang_wire::Node,
        kept: &super::Kept<'_>,
        inputs: &mut super::Inputs,
        pictures: &mut super::Pictures,
        surfaces: &mut super::Surfaces,
    ) {
        match node {
            ui_lang_wire::Node::Input { key, .. } => {
                if let Some(field) = kept.inputs.fields.get(key) {
                    inputs.fields.insert(key.clone(), field.clone());
                }
            }
            ui_lang_wire::Node::Editor { key, .. } => {
                if let Some(field) = kept.inputs.editors.get(key) {
                    inputs.editors.insert(key.clone(), field.clone());
                }
            }
            ui_lang_wire::Node::Svg { hash, .. } => {
                if let Some(handle) = kept.pictures.handles.get(hash) {
                    pictures.handles.insert(*hash, handle.clone());
                }
            }
            ui_lang_wire::Node::Image { hash, .. } => {
                if let Some(handle) = kept.pictures.images.get(hash) {
                    pictures.images.insert(*hash, handle.clone());
                }
            }
            ui_lang_wire::Node::Surface { name, .. } => {
                if let Some(surface) = kept.surfaces.get(name) {
                    surfaces.insert(name.clone(), surface.clone());
                }
            }
            _ => {}
        }
        for child in node.children() {
            retain(child, kept, inputs, pictures, surfaces);
        }
    }
    retain(content, kept, &mut inputs, &mut pictures, &mut surfaces);
    let mut providers: Vec<_> = surfaces
        .iter()
        .map(|(name, surface)| {
            (
                name.clone(),
                std::sync::Arc::as_ptr(surface) as *const () as usize,
            )
        })
        .collect();
    providers.sort_unstable();
    let mut images: Vec<_> = pictures
        .handles
        .keys()
        .map(|hash| (false, *hash))
        .chain(pictures.images.keys().map(|hash| (true, *hash)))
        .collect();
    images.sort_unstable();
    let mut values: Vec<_> = inputs
        .fields
        .iter()
        .map(|(key, field)| (key.clone(), field.text.clone()))
        .collect();
    values.extend(
        inputs
            .editors
            .iter()
            .map(|(key, field)| (key.clone(), super::lock(&field.content).text())),
    );
    values.sort_unstable();
    let (canvas, canvas_revision) = kept.canvas.subset(content);
    let canvas = Rc::new(canvas);
    let containers = kept.containers.clone();
    let mut dimensions: Vec<_> = containers
        .iter()
        .map(|(key, size)| (key.clone(), size.map(f64::to_bits)))
        .collect();
    dimensions.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let ink = kept.button_ink.clone();
    let ink_identity = ink.as_ref().map(|ink| Rc::as_ptr(ink) as usize);
    let content = content.clone();
    let handle = kept.memo.clone();
    let parking = handle.clone();
    crate::memo_lazy(
        (
            generation,
            content.fingerprint(),
            dimensions,
            ink_identity,
            canvas_revision,
            providers,
            images,
            values,
        ),
        move |_| {
            super::render_node(
                &content,
                &super::Kept {
                    memo: handle.clone(),
                    button_ink: ink.clone(),
                    inputs: &inputs,
                    pictures: &pictures,
                    surfaces: &surfaces,
                    canvas: canvas.clone(),
                    containers: &containers,
                },
            )
        },
        0,
        key,
    )
    .parking(parking)
    .into()
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

    fn headless() -> iced::Renderer {
        use iced::advanced::renderer::Headless;
        iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap()
    }

    #[test]
    fn lazy_does_not_readmit_a_picture_rejected_by_the_instance_budget() {
        use super::super::{Inputs, MAX_PICTURE_BYTES, Pictures, Surfaces};
        let svg = ui_lang_wire::Node::Svg {
            key: "picture".into(), hash: 7,
            bytes: Some(br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10"/></svg>"#.to_vec()),
            inherit_button_ink: false, label: None, color: None, hover: None,
            fit: None, rotation: None, opacity: None, width: None, height: None,
        };
        let mut pictures = Pictures {
            bytes: MAX_PICTURE_BYTES,
            ..Pictures::default()
        };
        pictures.adopt(&svg);
        assert!(!pictures.handles.contains_key(&7));
        let node = ui_lang_wire::Node::Lazy {
            key: "lazy".into(),
            generation: 1,
            content: Box::new(svg),
        };
        let mut view = super::super::render(&node, &Inputs::default(), &pictures, &Surfaces::new());
        let mut tree = Tree::new(view.as_widget());
        let layout = view
            .as_widget_mut()
            .layout(&mut tree, &headless(), &layout::Limits::NONE);
        assert_eq!(
            layout.size(),
            Size::ZERO,
            "rejected SVG must remain empty space under lazy"
        );
    }

    #[test]
    fn lazy_tracks_host_sanitization_even_when_guest_generation_is_unchanged() {
        use super::super::{Inputs, Pictures, Surfaces};
        use ui_lang_wire as wire;
        fn text(key: &str, content: String) -> wire::Node {
            wire::Node::Text {
                key: key.into(),
                content,
                options: Default::default(),
                size: None,
                color: None,
                font: Default::default(),
                width: None,
                align_x: None,
            }
        }
        let node = |spent: usize| {
            let mut frame = wire::Frame {
                root: Some(wire::Node::Linear {
                    max_width: None,
                    clip: false,
                    key: "root".into(),
                    axis: wire::Axis::Column,
                    wrap: None,
                    spacing: None,
                    padding: None,
                    width: None,
                    height: None,
                    align: None,
                    background: None,
                    border: None,
                    children: vec![
                        text("prefix", "x".repeat(spent)),
                        wire::Node::Lazy {
                            key: "lazy".into(),
                            generation: 1,
                            content: Box::new(text("tail", "tail".into())),
                        },
                    ],
                }),
                ..Default::default()
            };
            wire::sanitize(&mut frame);
            frame.root.as_ref().unwrap().children()[1].clone()
        };
        let before = node(0);
        let after = node(wire::MAX_TEXT_BYTES_PER_FRAME);
        assert_ne!(
            before, after,
            "shared frame budget must change the lazy wire content"
        );
        let inputs = Inputs::default();
        let pictures = Pictures::default();
        let surfaces = Surfaces::new();
        let renderer = headless();
        let mut view = super::super::render(&before, &inputs, &pictures, &surfaces);
        let mut tree = Tree::new(view.as_widget());
        let first = view
            .as_widget_mut()
            .layout(&mut tree, &renderer, &layout::Limits::NONE);
        assert!(first.size().width > 0.0);
        drop(view);
        let mut view = super::super::render(&after, &inputs, &pictures, &surfaces);
        tree.diff(view.as_widget());
        let after = view
            .as_widget_mut()
            .layout(&mut tree, &renderer, &layout::Limits::NONE);
        assert_eq!(
            after.size().width,
            0.0,
            "host-truncated text must replace cached native text"
        );
    }

    #[test]
    fn wire_lazy_reuses_view_and_layout_until_revision_or_limits_change() {
        use super::super::{Inputs, Pictures, Surfaces};
        use iced::advanced::renderer::Headless;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let builds = Arc::new(AtomicUsize::new(0));
        let counter = builds.clone();
        let mut surfaces = Surfaces::new();
        surfaces.insert(
            "counted".into(),
            Arc::new(move |_, _| {
                counter.fetch_add(1, Ordering::Relaxed);
                iced::widget::text("cached native surface").into()
            }),
        );
        let inputs = Inputs::default();
        let pictures = Pictures::default();
        let node = |generation| ui_lang_wire::Node::Lazy {
            key: "lazy".into(),
            generation,
            content: Box::new(ui_lang_wire::Node::Surface {
                key: "surface".into(),
                name: "counted".into(),
                args: vec![],
                on_event: None,
            }),
        };
        let render =
            |generation| super::super::render(&node(generation), &inputs, &pictures, &surfaces);
        let limits = layout::Limits::new(Size::ZERO, Size::new(300.0, 100.0));
        let mut first = render(1);
        let mut tree = Tree::new(first.as_widget());
        crate::take_memo_lazy_counts();
        first.as_widget_mut().layout(&mut tree, &renderer, &limits);
        assert_eq!(crate::take_memo_lazy_counts(), (0, 1));
        drop(first);
        let mut same = render(1);
        tree.diff(same.as_widget());
        same.as_widget_mut().layout(&mut tree, &renderer, &limits);
        assert_eq!(
            builds.load(Ordering::Relaxed),
            1,
            "unchanged wire boundary must reuse its native surface"
        );
        assert_eq!(
            crate::take_memo_lazy_counts(),
            (1, 0),
            "unchanged limits must reuse layout"
        );
        let smaller = layout::Limits::new(Size::ZERO, Size::new(120.0, 100.0));
        same.as_widget_mut().layout(&mut tree, &renderer, &smaller);
        assert_eq!(
            crate::take_memo_lazy_counts(),
            (0, 1),
            "new limits must reflow"
        );
        drop(same);
        let hidden =
            super::super::render(&ui_lang_wire::Node::empty(), &inputs, &pictures, &surfaces);
        tree.diff(hidden.as_widget());
        drop(hidden);
        let again = render(1);
        tree.diff(again.as_widget());
        assert_eq!(
            builds.load(Ordering::Relaxed),
            1,
            "wire unmount/remount must reclaim native content"
        );
        drop(again);
        let changed = render(2);
        tree.diff(changed.as_widget());
        assert_eq!(
            builds.load(Ordering::Relaxed),
            2,
            "fresh guest generation must rebuild its surface"
        );
        drop(changed);
        let next_instance =
            super::super::render(&node(2), &Inputs::default(), &pictures, &surfaces);
        tree.diff(next_instance.as_widget());
        assert_eq!(
            builds.load(Ordering::Relaxed),
            3,
            "a restarted module must not reclaim old content"
        );
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
