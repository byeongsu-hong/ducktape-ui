//! UI-thread ownership for a module's parked native subtrees.
use super::{IceElement, Output};
#[path = "focus.rs"]
mod focus;
pub(super) use focus::Cache as FocusCache;
#[path = "scroll.rs"]
mod scroll;
use crate::{MemoParking, MemoParkingHandle};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Event, Length, Rectangle, Size, Vector};

pub(super) fn scope(
    content: IceElement<'static, Output>,
    instance: u64,
    handle: MemoParkingHandle,
    root: &ui_lang_wire::Node,
    focus: &FocusCache,
) -> IceElement<'static, Output> {
    iced::Element::new(Scope {
        content,
        instance,
        handle,
        focus: focus.get(root),
        scroll: scroll::Targets::new(root),
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
        focus_cache: kept.inputs.focus_cache.clone(),
        fields: HashMap::new(),
        editors: HashMap::new(),
        editor_references: kept.inputs.editor_references.clone(),
        editor_transfer: kept.inputs.editor_transfer.clone(),
        editor_outgoing: kept.inputs.editor_outgoing.clone(),
        editor_document_fault: kept.inputs.editor_document_fault.clone(),
        editor_revision: kept.inputs.editor_revision,
        editor_sequence: kept.inputs.editor_sequence.clone(),
        combos: HashMap::new(),
        editor_transactions: kept.inputs.editor_transactions.clone(),
        editor_bindings: kept.inputs.editor_bindings.clone(),
        editor_reported: kept.inputs.editor_reported.clone(),
        editor_notifications: Vec::new(),
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
            ui_lang_wire::Node::ComboBox { state_key: key, .. } => {
                if let Some(field) = kept.inputs.combos.get(key) {
                    inputs.combos.insert(key.clone(), field.clone());
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
            ui_lang_wire::Node::Image { hash, .. }
            | ui_lang_wire::Node::ImageViewer { hash, .. } => {
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
    focus: std::sync::Arc<focus::Targets>,
    scroll: scroll::Targets,
}

struct State {
    instance: u64,
    owner: MemoParking,
    focused: Option<focus::Target>,
    restore: Option<focus::Target>,
    positions: scroll::Positions,
    restore_positions: scroll::Positions,
}

impl Scope {
    fn observe_focus(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &iced::Renderer) {
        let focused = self.focus.capture(|operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
        tree.state.downcast_mut::<State>().focused = focused;
    }

    fn observe_scroll(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &iced::Renderer) {
        let positions = self.scroll.capture(|operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
        tree.state.downcast_mut::<State>().positions = positions;
    }

    fn fresh_state(&self) -> State {
        let owner = MemoParking::default();
        self.handle.attach(&owner);
        State {
            instance: self.instance,
            owner,
            focused: None,
            restore: None,
            positions: Default::default(),
            restore_positions: Default::default(),
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
            let restore = state
                .focused
                .take()
                .filter(|target| self.focus.contains(target));
            let positions = std::mem::take(&mut state.positions);
            *state = self.fresh_state();
            state.restore_positions = positions;
            state.restore = restore;
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
        let layout = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        if let Some(target) = tree.state.downcast_mut::<State>().restore.take() {
            focus::restore(&target, &self.focus, |operation| {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    Layout::new(&layout),
                    renderer,
                    operation,
                );
            });
        }
        let positions = std::mem::take(&mut tree.state.downcast_mut::<State>().restore_positions);
        self.scroll.restore(positions, |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                Layout::new(&layout),
                renderer,
                operation,
            );
        });
        self.observe_focus(tree, Layout::new(&layout), renderer);
        self.observe_scroll(tree, Layout::new(&layout), renderer);
        layout
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
        self.observe_focus(tree, layout, renderer);
        self.observe_scroll(tree, layout, renderer);
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
        self.observe_scroll(tree, layout, renderer);
        // Pointer observation and redraw ticks do not change a control's focus.
        // Layout and explicit widget operations record focus separately.
        let inert = matches!(
            event,
            Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::WheelScrolled { .. })
                | Event::Window(iced::window::Event::RedrawRequested(_))
        );
        if !inert {
            self.observe_focus(tree, layout, renderer);
        }
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
        scope(
            content,
            instance,
            handle,
            &ui_lang_wire::Node::empty(),
            &FocusCache::default(),
        )
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

    struct FocusInput {
        id: iced::widget::Id,
        set: bool,
        focused: bool,
    }
    impl Operation for FocusInput {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn focusable(
            &mut self,
            id: Option<&iced::widget::Id>,
            _: Rectangle,
            state: &mut dyn iced::advanced::widget::operation::Focusable,
        ) {
            if id == Some(&self.id) {
                if self.set {
                    state.focus();
                }
                self.focused |= state.is_focused();
            }
        }
    }

    fn input(key: &str, disabled: bool) -> ui_lang_wire::Node {
        ui_lang_wire::Node::Input {
            options: ui_lang_wire::InputOptions {
                disabled,
                ..Default::default()
            },
            key: key.into(),
            placeholder: String::new(),
            value: "new state".into(),
            on_input: 1,
            on_submit: None,
            width: None,
            secure: false,
            style: Box::default(),
        }
    }

    fn column(children: Vec<ui_lang_wire::Node>) -> ui_lang_wire::Node {
        ui_lang_wire::Node::Linear {
            key: "root".into(),
            axis: ui_lang_wire::Axis::Column,
            max_width: None,
            clip: false,
            wrap: None,
            spacing: None,
            padding: None,
            width: None,
            height: None,
            align: None,
            background: None,
            border: None,
            children,
        }
    }

    fn replaced_focus(
        before: &ui_lang_wire::Node,
        after: &ui_lang_wire::Node,
        focused: Option<&str>,
        observed: &str,
    ) -> bool {
        replaced_focus_with_surfaces(
            before,
            after,
            focused,
            observed,
            &super::super::Surfaces::new(),
        )
    }

    fn replaced_focus_with_surfaces(
        before: &ui_lang_wire::Node,
        after: &ui_lang_wire::Node,
        focused: Option<&str>,
        observed: &str,
        surfaces: &super::super::Surfaces,
    ) -> bool {
        use super::super::{Inputs, Pictures};
        let renderer = headless();
        let mut first =
            super::super::render(before, &Inputs::default(), &Pictures::default(), surfaces);
        let mut tree = Tree::new(first.as_widget());
        let layout = first
            .as_widget_mut()
            .layout(&mut tree, &renderer, &layout::Limits::NONE);
        if let Some(id) = focused {
            let mut focus = FocusInput {
                id: iced::widget::Id::from(id.to_owned()),
                set: true,
                focused: false,
            };
            first
                .as_widget_mut()
                .operate(&mut tree, Layout::new(&layout), &renderer, &mut focus);
            assert!(focus.focused, "old native control is actually focused");
        }
        drop(first);
        let mut next =
            super::super::render(after, &Inputs::default(), &Pictures::default(), surfaces);
        tree.diff(next.as_widget());
        let layout = next
            .as_widget_mut()
            .layout(&mut tree, &renderer, &layout::Limits::NONE);
        let mut focus = FocusInput {
            id: iced::widget::Id::from(observed.to_owned()),
            set: false,
            focused: false,
        };
        next.as_widget_mut()
            .operate(&mut tree, Layout::new(&layout), &renderer, &mut focus);
        focus.focused
    }

    fn scroll_node(
        key: &str,
        height: f32,
        anchor_y: ui_lang_wire::ScrollAnchor,
    ) -> ui_lang_wire::Node {
        use ui_lang_wire::{Length, Node, ScrollAnchor, ScrollDirection};
        Node::Scroll {
            key: key.into(),
            on_scroll: None,
            virtual_rows: false,
            direction: ScrollDirection::Vertical,
            width: Some(Length::Fixed(100.0)),
            height: Some(Length::Fixed(100.0)),
            bar_hidden: true,
            bar_width: None,
            bar_margin: None,
            scroller_width: None,
            bar_spacing: None,
            anchor_x: ScrollAnchor::Start,
            anchor_y,
            auto_scroll: false,
            background: None,
            border: None,
            content: Box::new(Node::Space {
                width: Some(Length::Fixed(100.0)),
                height: Some(Length::Fixed(height)),
            }),
        }
    }

    #[derive(Default)]
    struct ScrollPosition(Vec<(iced::widget::Id, f32)>);
    impl Operation for ScrollPosition {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&iced::widget::Id>,
            _: Rectangle,
            _: Rectangle,
            translation: Vector,
            _: &mut dyn iced::advanced::widget::operation::Scrollable,
        ) {
            if let Some(id) = id {
                self.0.push((id.clone(), translation.y));
            }
        }
    }

    fn replaced_scroll(
        after: &ui_lang_wire::Node,
        anchor: ui_lang_wire::ScrollAnchor,
    ) -> Vec<(iced::widget::Id, f32)> {
        replaced_scroll_with_surfaces(after, anchor, &super::super::Surfaces::new())
    }

    fn replaced_scroll_with_surfaces(
        after: &ui_lang_wire::Node,
        anchor: ui_lang_wire::ScrollAnchor,
        surfaces: &super::super::Surfaces,
    ) -> Vec<(iced::widget::Id, f32)> {
        use super::super::{Inputs, Pictures, Surfaces, render};
        use iced_test::runtime::{UserInterface, user_interface};
        let mut renderer = headless();
        let before = scroll_node("history", 600.0, anchor);
        let mut ui = UserInterface::build(
            render(
                &before,
                &Inputs::default(),
                &Pictures::default(),
                &Surfaces::new(),
            ),
            Size::new(100.0, 200.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        ui.update(
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels {
                    x: 0.0,
                    y: if anchor == ui_lang_wire::ScrollAnchor::End {
                        60.0
                    } else {
                        -60.0
                    },
                },
            })],
            mouse::Cursor::Available(iced::Point::new(25.0, 25.0)),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
        // Do not inspect/operate/layout before replacement: wheel observation
        // must record the position without another chance to capture it.
        let mut ui = UserInterface::build(
            render(after, &Inputs::default(), &Pictures::default(), surfaces),
            Size::new(100.0, 200.0),
            ui.into_cache(),
            &mut renderer,
        );
        let mut position = ScrollPosition::default();
        ui.operate(&renderer, &mut position);
        position.0
    }

    #[test]
    fn replaced_instance_retains_immediate_wheel_scroll() {
        let node = scroll_node("history", 600.0, ui_lang_wire::ScrollAnchor::Start);
        assert_eq!(
            replaced_scroll(&node, ui_lang_wire::ScrollAnchor::Start),
            vec![(iced::widget::Id::new("history"), 60.0)]
        );
    }

    #[test]
    fn replaced_scroll_preserves_both_axes() {
        use super::super::{Inputs, Pictures, Surfaces, render};
        use iced_test::runtime::{UserInterface, user_interface};
        let mut renderer = headless();
        let mut node = scroll_node("history", 600.0, ui_lang_wire::ScrollAnchor::Start);
        if let ui_lang_wire::Node::Scroll {
            direction, content, ..
        } = &mut node
        {
            *direction = ui_lang_wire::ScrollDirection::Both;
            if let ui_lang_wire::Node::Space { width, .. } = content.as_mut() {
                *width = Some(ui_lang_wire::Length::Fixed(600.0));
            }
        }
        let view = || {
            render(
                &node,
                &Inputs::default(),
                &Pictures::default(),
                &Surfaces::new(),
            )
        };
        let mut ui = UserInterface::build(
            view(),
            Size::new(100.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        ui.update(
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: -45.0, y: -60.0 },
            })],
            mouse::Cursor::Available(iced::Point::new(25.0, 25.0)),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
        let mut ui = UserInterface::build(
            view(),
            Size::new(100.0, 100.0),
            ui.into_cache(),
            &mut renderer,
        );
        #[derive(Default)]
        struct Position(Option<Vector>);
        impl Operation for Position {
            fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
                visit(self);
            }
            fn scrollable(
                &mut self,
                _: Option<&iced::widget::Id>,
                _: Rectangle,
                _: Rectangle,
                translation: Vector,
                _: &mut dyn iced::advanced::widget::operation::Scrollable,
            ) {
                self.0 = Some(translation);
            }
        }
        let mut position = Position::default();
        ui.operate(&renderer, &mut position);
        assert_eq!(position.0, Some(Vector::new(45.0, 60.0)));
    }

    #[test]
    fn replaced_scroll_clamps_and_preserves_anchor_distance() {
        use ui_lang_wire::ScrollAnchor::{End, Start};
        assert_eq!(
            replaced_scroll(&scroll_node("history", 130.0, Start), Start)[0].1,
            30.0
        );
        assert_eq!(
            replaced_scroll(&scroll_node("history", 800.0, End), End)[0].1,
            640.0
        );
        assert_eq!(
            replaced_scroll(&scroll_node("history", 130.0, End), End)[0].1,
            0.0
        );
    }

    #[test]
    fn replaced_scroll_rejects_removed_ambiguous_and_changed_identities() {
        use ui_lang_wire::ScrollAnchor::{End, Start};
        assert!(replaced_scroll(&ui_lang_wire::Node::empty(), Start).is_empty());
        assert_eq!(
            replaced_scroll(&scroll_node("different", 600.0, Start), Start)[0].1,
            0.0
        );
        let duplicate = column(vec![
            scroll_node("history", 600.0, Start),
            scroll_node("history", 600.0, Start),
        ]);
        assert!(
            replaced_scroll(&duplicate, Start)
                .iter()
                .all(|(_, y)| *y == 0.0)
        );
        assert_eq!(
            replaced_scroll(&scroll_node("history", 600.0, End), Start)[0].1,
            500.0
        );
        let mut direction = scroll_node("history", 600.0, Start);
        if let ui_lang_wire::Node::Scroll { direction, .. } = &mut direction {
            *direction = ui_lang_wire::ScrollDirection::Both;
        }
        assert_eq!(replaced_scroll(&direction, Start)[0].1, 0.0);
    }

    #[test]
    fn replaced_scroll_does_not_target_a_colliding_host_surface() {
        use ui_lang_wire::ScrollAnchor::Start;
        let mut surfaces = super::super::Surfaces::new();
        surfaces.insert(
            "collision".into(),
            std::sync::Arc::new(|_, _| {
                iced::widget::scrollable(iced::widget::Space::new().height(600.0))
                    .id("history")
                    .height(100.0)
                    .into()
            }),
        );
        let surface = ui_lang_wire::Node::Surface {
            key: "native".into(),
            name: "collision".into(),
            args: vec![],
            on_event: None,
        };
        let mut after = column(vec![scroll_node("history", 600.0, Start), surface]);
        let offsets = replaced_scroll_with_surfaces(&after, Start, &surfaces);
        assert_eq!(offsets.len(), 2);
        assert!(offsets.iter().all(|(_, y)| *y == 0.0));
        // The declared guest ID is now hidden; the lone native match belongs
        // to the host surface and must not inherit guest state either.
        if let ui_lang_wire::Node::Linear { children, .. } = &mut after {
            children[0] = ui_lang_wire::Node::Tooltip {
                key: "tip".into(),
                position: ui_lang_wire::TooltipPosition::Top,
                gap: 0.0,
                padding: 0.0,
                delay_ms: 0,
                snap: false,
                style: Default::default(),
                children: vec![
                    ui_lang_wire::Node::empty(),
                    scroll_node("history", 600.0, Start),
                ],
            };
        }
        let offsets = replaced_scroll_with_surfaces(&after, Start, &surfaces);
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0].1, 0.0);
    }

    #[test]
    fn rendering_changed_roots_without_adopt_refreshes_focus_metadata() {
        let inputs = super::super::Inputs::default();
        let before = input("before", false);
        let after = input("after", false);
        let pictures = super::super::Pictures::default();
        let surfaces = super::super::Surfaces::new();
        drop(super::super::render(&before, &inputs, &pictures, &surfaces));
        let first = inputs
            .focus_cache
            .snapshot()
            .expect("render initialized metadata");
        drop(super::super::render(&after, &inputs, &pictures, &surfaces));
        let second = inputs.focus_cache.snapshot().unwrap();
        assert!(!std::sync::Arc::ptr_eq(&first, &second));
        drop(super::super::render(&after, &inputs, &pictures, &surfaces));
        assert!(std::sync::Arc::ptr_eq(
            &second,
            &inputs.focus_cache.snapshot().unwrap()
        ));
    }

    #[test]
    fn replaced_instance_focuses_only_the_same_native_input_identity() {
        assert!(
            replaced_focus(
                &input("draft", false),
                &input("draft", false),
                Some("draft"),
                "draft"
            ),
            "replacement must transfer focus into new native state",
        );
    }

    #[test]
    fn replacement_never_substitutes_another_or_disabled_control() {
        let before = input("draft", false);
        assert!(!replaced_focus(
            &before,
            &input("other", false),
            Some("draft"),
            "other"
        ));
        assert!(!replaced_focus(
            &before,
            &input("draft", true),
            Some("draft"),
            "draft"
        ));
        let changed_kind = ui_lang_wire::Node::Toggle {
            key: "draft".into(),
            kind: ui_lang_wire::ToggleKind::Checkbox,
            label: "different control".into(),
            checked: false,
            on_toggle: Some(2),
            width: None,
            style: Default::default(),
        };
        assert!(!replaced_focus(
            &before,
            &changed_kind,
            Some("draft"),
            "draft"
        ));
        let duplicate = column(vec![input("draft", false), input("draft", false)]);
        assert!(!replaced_focus(&before, &duplicate, Some("draft"), "draft"));
        assert!(!replaced_focus(&before, &before, None, "draft"));
    }

    #[test]
    fn replacement_does_not_focus_a_colliding_host_surface_descendant() {
        use super::super::Surfaces;
        let mut surfaces = Surfaces::new();
        surfaces.insert(
            "collision".into(),
            std::sync::Arc::new(|_, _| {
                iced::widget::text_input("host surface", "")
                    .id(iced::widget::Id::from("draft"))
                    .on_input(|_| ui_lang_wire::SurfaceValue::Bool(false))
                    .into()
            }),
        );
        let after = column(vec![
            input("draft", false),
            ui_lang_wire::Node::Surface {
                key: "native".into(),
                name: "collision".into(),
                args: Vec::new(),
                on_event: None,
            },
        ]);
        assert!(
            !replaced_focus_with_surfaces(
                &input("draft", false),
                &after,
                Some("draft"),
                "draft",
                &surfaces,
            ),
            "an ambiguous native identity must not receive replacement focus"
        );
        let mut hidden = after;
        if let ui_lang_wire::Node::Linear { children, .. } = &mut hidden {
            children[0] = ui_lang_wire::Node::Tooltip {
                key: "tip".into(),
                position: ui_lang_wire::TooltipPosition::Top,
                gap: 0.0,
                padding: 0.0,
                delay_ms: 0,
                snap: false,
                style: Default::default(),
                children: vec![ui_lang_wire::Node::empty(), input("draft", false)],
            };
        }
        assert!(
            !replaced_focus_with_surfaces(
                &input("draft", false),
                &hidden,
                Some("draft"),
                "draft",
                &surfaces,
            ),
            "an unmounted wire identity must not authorize a host surface control"
        );
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
            wire::sanitize(&mut frame).unwrap();
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
