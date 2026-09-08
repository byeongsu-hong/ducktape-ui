//! Executes only the fixed commands the tree protocol carries.
use iced::advanced::widget::operation::{self, Outcome, focusable, scrollable, text_input};
use iced::advanced::widget::{Id, Operation};
use ui_lang_wire::{self as wire, WidgetCommand as C};

/// Execute a guest command using a traversal confined to its mounted tree.
/// The embedding host owns request/frame validation, cancellation and budgets.
pub fn execute_widget_command(
    mut command: C,
    mut traverse: impl FnMut(&mut dyn Operation),
) -> Result<Vec<u8>, String> {
    command.validate()?;
    let query = matches!(command, C::Focused { .. });
    let mut operation: Box<dyn Operation<Vec<u8>>> = match command {
        C::FocusPrevious => Box::new(focusable::focus_previous()),
        C::FocusNext => Box::new(focusable::focus_next()),
        C::Focus { target } => Box::new(focusable::focus(Id::from(target))),
        C::Focused { target } => Box::new(operation::map(
            focusable::is_focused(Id::from(target)),
            |value| wire::encode(&value),
        )),
        C::CursorFront { target } => Box::new(text_input::move_cursor_to_front(Id::from(target))),
        C::CursorEnd { target } => Box::new(text_input::move_cursor_to_end(Id::from(target))),
        C::Cursor { target, position } => Box::new(text_input::move_cursor_to(
            Id::from(target),
            position as usize,
        )),
        C::SelectAll { target } => Box::new(text_input::select_all(Id::from(target))),
        C::Select { target, start, end } => Box::new(text_input::select_range(
            Id::from(target),
            start as usize,
            end as usize,
        )),
        C::Snap { target, x, y } => Box::new(scrollable::snap_to(
            Id::from(target),
            scrollable::RelativeOffset {
                x: Some(x),
                y: Some(y),
            },
        )),
        C::SnapEnd { target } => Box::new(crate::scroll_anchor::content_end_operation(Id::from(
            target,
        ))),
        C::ScrollTo { target, x, y } => Box::new(scrollable::scroll_to(
            Id::from(target),
            scrollable::AbsoluteOffset {
                x: Some(x),
                y: Some(y),
            },
        )),
        C::ScrollToKey { target, key } => Box::new(
            crate::virtual_children::scroll_to_key_operation(Id::from(target), key),
        ),
        C::ScrollBy { target, x, y } => Box::new(scrollable::scroll_by(
            Id::from(target),
            scrollable::AbsoluteOffset { x, y },
        )),
    };
    // Native focus traversal and content-end snapping use chained passes.
    // Keyed row reveals add another pass; cap traversal regardless.
    for _ in 0..4 {
        traverse(&mut operation::black_box(operation.as_mut()));
        match operation.finish() {
            Outcome::None => {
                return Ok(if query {
                    wire::encode(&false)
                } else {
                    wire::encode(&())
                });
            }
            Outcome::Some(value) => return Ok(value),
            Outcome::Chain(next) => operation = next,
        }
    }
    Err("widget operation exceeded its traversal limit".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced::{Element, Event, Font, Pixels, Rectangle, Size, Vector, keyboard, mouse, widget};
    use iced_test::runtime::{UserInterface, user_interface};

    fn renderer() -> iced::Renderer {
        iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap()
    }
    fn fields() -> Element<'static, String> {
        widget::column![
            widget::text_input("First", "abcd")
                .id("draft")
                .on_input(std::convert::identity),
            widget::text_input("Second", "other")
                .id("second")
                .on_input(std::convert::identity),
        ]
        .into()
    }
    fn run(
        ui: &mut UserInterface<'_, String, iced::Theme, iced::Renderer>,
        renderer: &iced::Renderer,
        command: C,
    ) -> Vec<u8> {
        execute_widget_command(command, |operation| ui.operate(renderer, operation)).unwrap()
    }
    fn focused(
        ui: &mut UserInterface<'_, String, iced::Theme, iced::Renderer>,
        renderer: &iced::Renderer,
        target: &str,
    ) -> bool {
        wire::decode(&run(
            ui,
            renderer,
            C::Focused {
                target: target.into(),
            },
        ))
        .unwrap()
    }

    #[test]
    fn mounted_operations_preserve_focus_scope_and_input_selection() {
        let mut renderer = renderer();
        let mut first = UserInterface::build(
            fields(),
            Size::new(240.0, 120.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut second = UserInterface::build(
            fields(),
            Size::new(240.0, 120.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        assert!(!focused(&mut first, &renderer, "draft"));
        run(
            &mut first,
            &renderer,
            C::Focus {
                target: "draft".into(),
            },
        );
        assert!(
            focused(&mut first, &renderer, "draft"),
            "focus command must reach its mounted input"
        );
        assert!(
            !focused(&mut second, &renderer, "draft"),
            "another mounted tree with the same key must stay unfocused"
        );
        run(&mut first, &renderer, C::FocusNext);
        assert!(
            focused(&mut first, &renderer, "second"),
            "focus-next must finish its chained traversal"
        );
        run(&mut first, &renderer, C::FocusPrevious);
        assert!(focused(&mut first, &renderer, "draft"));
        assert!(!focused(&mut first, &renderer, "missing"));
        run(
            &mut first,
            &renderer,
            C::Select {
                target: "draft".into(),
                start: 1,
                end: 3,
            },
        );
        let mut messages = vec![];
        first.update(
            &[Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character("X".into()),
                modified_key: keyboard::Key::Character("X".into()),
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::default(),
                text: Some("X".into()),
                repeat: false,
            })],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec!["aXd"],
            "typing must replace the requested native selection"
        );
    }

    #[derive(Default)]
    struct Translation(Option<f32>);
    impl Operation for Translation {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _bounds: Rectangle,
            _content: Rectangle,
            translation: Vector,
            _state: &mut dyn operation::Scrollable,
        ) {
            if id == Some(&Id::new("list")) {
                self.0 = Some(translation.y);
            }
        }
    }
    fn translation(
        ui: &mut UserInterface<'_, String, iced::Theme, iced::Renderer>,
        renderer: &iced::Renderer,
    ) -> f32 {
        let mut read = Translation::default();
        ui.operate(renderer, &mut read);
        read.0.expect("native scroll region")
    }

    #[test]
    fn mounted_scroll_operations_preserve_offsets_and_both_end_anchors() {
        let mut renderer = renderer();
        for anchor in [
            widget::scrollable::Anchor::Start,
            widget::scrollable::Anchor::End,
        ] {
            let element = widget::scrollable::<String, iced::Theme, iced::Renderer>(
                widget::Space::new().height(600.0),
            )
            .id("list")
            .anchor_y(anchor)
            .height(100.0);
            let mut ui = UserInterface::build(
                element,
                Size::new(240.0, 100.0),
                user_interface::Cache::default(),
                &mut renderer,
            );
            run(
                &mut ui,
                &renderer,
                C::SnapEnd {
                    target: "list".into(),
                },
            );
            assert_eq!(
                translation(&mut ui, &renderer),
                500.0,
                "snap-end must show the content end under either anchor"
            );
        }
        let element = widget::scrollable::<String, iced::Theme, iced::Renderer>(
            widget::Space::new().height(600.0),
        )
        .id("list")
        .height(100.0);
        let mut ui = UserInterface::build(
            element,
            Size::new(240.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        assert_eq!(translation(&mut ui, &renderer), 0.0);
        run(
            &mut ui,
            &renderer,
            C::ScrollTo {
                target: "list".into(),
                x: 0.0,
                y: 100.0,
            },
        );
        assert_eq!(translation(&mut ui, &renderer), 100.0);
        run(
            &mut ui,
            &renderer,
            C::ScrollBy {
                target: "list".into(),
                x: 0.0,
                y: -24.0,
            },
        );
        assert_eq!(
            translation(&mut ui, &renderer),
            76.0,
            "negative scroll-by must move toward the start"
        );
        run(
            &mut ui,
            &renderer,
            C::Snap {
                target: "list".into(),
                x: 0.0,
                y: 0.5,
            },
        );
        assert_eq!(translation(&mut ui, &renderer), 250.0);
    }
}
