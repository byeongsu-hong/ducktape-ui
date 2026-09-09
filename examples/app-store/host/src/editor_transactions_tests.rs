//! Real native/Wasm editor transactions, driven through the native widget.
use super::*;
use iced::advanced::{clipboard, renderer::Headless, widget::Operation};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, keyboard, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};
type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;

#[derive(Default)]
struct Clipboard(String);
impl clipboard::Clipboard for Clipboard {
    fn read(&self, _: clipboard::Kind) -> Option<String> {
        Some(self.0.clone())
    }
    fn write(&mut self, _: clipboard::Kind, value: String) {
        self.0 = value;
    }
}
fn build(
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    cache: user_interface::Cache,
) -> Ui {
    UserInterface::build(
        wasm_view(Surface(guest.clone()), false),
        Size::new(300.0, 480.0),
        cache,
        renderer,
    )
}
fn settle(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut std::time::Instant,
    clipboard: &mut Clipboard,
) -> Ui {
    // Several frames are required for request → decision → commit → reducer → ack.
    for _ in 0..16 {
        *now += std::time::Duration::from_millis(16);
        let mut messages = vec![];
        ui.update(
            &[Event::Window(window::Event::RedrawRequested(*now))],
            mouse::Cursor::Unavailable,
            renderer,
            clipboard,
            &mut messages,
        );
        assert!(
            guest.lock().unwrap().fault.is_none(),
            "transaction guest fault"
        );
        if messages.iter().any(|message| message == "wake") {
            ui = build(guest, renderer, ui.into_cache());
        }
    }
    ui
}
fn send(
    ui: &mut Ui,
    renderer: &mut iced::Renderer,
    clipboard: &mut Clipboard,
    point: Point,
    event: Event,
) -> iced::event::Status {
    ui.update(
        &[event],
        mouse::Cursor::Available(point),
        renderer,
        clipboard,
        &mut vec![],
    )
    .1[0]
}
fn key(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Event {
    let text = match &key {
        keyboard::Key::Character(value) => Some(value.clone()),
        _ => None,
    };
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text,
        repeat: false,
    })
}
fn named(value: keyboard::key::Named) -> Event {
    key(keyboard::Key::Named(value), keyboard::Modifiers::empty())
}
fn editor(node: &wire::Node) -> Option<&wire::Node> {
    if matches!(node, wire::Node::Editor { .. }) {
        Some(node)
    } else {
        node.children().iter().find_map(editor)
    }
}
fn state(guest: &Arc<Mutex<Guest>>) -> wire::EditorState {
    let guest = guest.lock().unwrap();
    let wire::Node::Editor { key, .. } = editor(guest.frame.root.as_ref().unwrap()).unwrap() else {
        unreachable!()
    };
    let document = guest
        .inputs
        .editor_document(key)
        .expect("complete editor document");
    let reference = document.reference();
    wire::EditorState {
        text: document.text().to_owned(),
        cursor: reference.cursor,
        reset: reference.reset,
        revision: reference.revision,
    }
}

fn assert_document_identities(guest: &Arc<Mutex<Guest>>) {
    fn editors<'a>(node: &'a wire::Node, output: &mut Vec<&'a wire::Node>) {
        if matches!(node, wire::Node::Editor { .. }) {
            output.push(node);
        }
        for child in node.children() {
            editors(child, output);
        }
    }
    let guest = guest.lock().unwrap();
    let mut nodes = vec![];
    editors(guest.frame.root.as_ref().unwrap(), &mut nodes);
    let mut independent = vec![];
    let mut shared = vec![];
    for node in nodes {
        let wire::Node::Editor {
            options, document, ..
        } = node
        else {
            unreachable!()
        };
        if document.byte_len == 0 {
            assert!(
                options.binding.is_some(),
                "component binding reached the real guest"
            );
            independent.push((&document.document, document.reset, document.revision));
        } else {
            assert_eq!(document.byte_len, 2);
            shared.push(&document.document);
        }
    }
    assert_eq!(independent.len(), 2, "two mounted component instances");
    assert_eq!(independent[0].1, independent[1].1);
    assert_eq!(independent[0].2, independent[1].2);
    assert_ne!(
        independent[0].0, independent[1].0,
        "equal empty states belong to distinct component lanes"
    );
    assert_eq!(shared.len(), 2, "one app state rendered twice");
    assert_eq!(
        shared[0], shared[1],
        "widget identity must not split one state into two lanes"
    );
    assert_ne!(shared[0], independent[0].0);
    assert_ne!(shared[0], independent[1].0);
}
fn texts(node: &wire::Node, output: &mut Vec<String>) {
    if let wire::Node::Text { content, .. } = node {
        output.push(content.clone());
    }
    for child in node.children() {
        texts(child, output);
    }
}
fn commits(guest: &Arc<Mutex<Guest>>) -> u64 {
    let guest = guest.lock().unwrap();
    let mut labels = vec![];
    texts(guest.frame.root.as_ref().unwrap(), &mut labels);
    assert!(
        labels
            .iter()
            .any(|label| label == "state accepted before route"),
        "route observed stale editor: {labels:?}"
    );
    assert!(
        labels.iter().any(|label| label == "commits ordered"),
        "duplicate/out-of-order commit: {labels:?}"
    );
    labels
        .iter()
        .find_map(|label| label.parse().ok())
        .expect("rendered commit counter")
}
struct Bounds(String, Option<Rectangle>);
impl Operation for Bounds {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn focusable(
        &mut self,
        id: Option<&iced::widget::Id>,
        bounds: Rectangle,
        _: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        if id == Some(&iced::widget::Id::from(self.0.clone())) {
            self.1 = Some(bounds);
        }
    }
}

#[test]
#[ignore = "requires current native and Wasm editor-transactions fixtures"]
fn editor_transactions_native_and_wasm_order_input_and_unify_guest_history() {
    use keyboard::key::Named;
    for native in [false, true] {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
            "../target/editor-transactions-native"
        } else {
            "../target/editor-transactions-fixture"
        });
        let entries = crate::catalog::scan_dir(&directory);
        assert_eq!(
            entries.len(),
            1,
            "build current transaction fixture ({native})"
        );
        let guest = Arc::new(Mutex::new(Guest::load(&entries[0]).unwrap()));
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut ui = build(&guest, &mut renderer, user_interface::Cache::default());
        let mut now = std::time::Instant::now();
        let mut clipboard = Clipboard::default();
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_document_identities(&guest);
        let mut bounds = Bounds(
            editor(guest.lock().unwrap().frame.root.as_ref().unwrap())
                .unwrap()
                .key()
                .unwrap()
                .to_owned(),
            None,
        );
        ui.operate(&renderer, &mut bounds);
        let point = bounds.1.expect("actual editor bounds").center();
        for button in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            send(
                &mut ui,
                &mut renderer,
                &mut clipboard,
                point,
                Event::Mouse(button),
            );
        }
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            named(Named::End),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(state(&guest).text, "ab");
        assert_eq!(state(&guest).cursor.position.column, 2);
        let initial_commits = commits(&guest);
        // No redraw between these: typing must wait behind the structural claim.
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            named(Named::Tab),
        );
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            key(
                keyboard::Key::Character("x".into()),
                keyboard::Modifiers::empty(),
            ),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(
            state(&guest).text,
            "  abx",
            "Tab then typing, native={native}"
        );
        assert_eq!(state(&guest).cursor.position.column, 5);
        assert_eq!(commits(&guest) - initial_commits, 2, "one commit per input");
        let command = if cfg!(target_os = "macos") {
            keyboard::Modifiers::LOGO
        } else {
            keyboard::Modifiers::CTRL
        };
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            key(keyboard::Key::Character("z".into()), command),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(
            state(&guest).text,
            "ab",
            "one guest undo includes structural patch and native typing"
        );
        assert_eq!(state(&guest).cursor.position.column, 2);
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            key(
                keyboard::Key::Character("z".into()),
                command | keyboard::Modifiers::SHIFT,
            ),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(
            state(&guest).text,
            "  abx",
            "redo restores the same group once"
        );
        let before_noop = commits(&guest);
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            named(Named::Backspace),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(state(&guest).text, "  abx");
        assert_eq!(
            commits(&guest),
            before_noop + 1,
            "Noop still has exactly one accepted commit"
        );
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            named(Named::Enter),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(
            state(&guest).text,
            "  abx\n",
            "saved native Enter action executes"
        );
        let before_burst = commits(&guest);
        clipboard.0 = "한".into();
        for event in [
            named(Named::Tab),
            key(
                keyboard::Key::Character("x".into()),
                keyboard::Modifiers::empty(),
            ),
            named(Named::ArrowLeft),
            key(keyboard::Key::Character("v".into()), command),
        ] {
            send(&mut ui, &mut renderer, &mut clipboard, point, event);
        }
        for event in [
            Event::Keyboard(keyboard::Event::ModifiersChanged(
                keyboard::Modifiers::SHIFT,
            )),
            Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Character("x".into()),
                modified_key: keyboard::Key::Character("x".into()),
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::SHIFT,
            }),
            Event::Keyboard(keyboard::Event::ModifiersChanged(
                keyboard::Modifiers::empty(),
            )),
        ] {
            assert_eq!(
                send(&mut ui, &mut renderer, &mut clipboard, point, event),
                iced::event::Status::Captured,
                "release and modifier transitions join the pending editor lane"
            );
        }
        // Paste owns its admission-time bytes; delayed replay must not reread it.
        clipboard.0 = "LATE".into();
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(
            state(&guest).text,
            "    abx\n한x",
            "pending Tab → typing → caret → captured paste"
        );
        assert_eq!(
            state(&guest).cursor.position,
            wire::EditorPosition { line: 1, column: 3 }
        );
        assert_eq!(state(&guest).cursor.selection, None);
        assert_eq!(commits(&guest) - before_burst, 4);
        let before_ime = commits(&guest);
        use iced::advanced::input_method::Event as Ime;
        for event in [
            named(Named::Tab),
            Event::InputMethod(Ime::Opened),
            Event::InputMethod(Ime::Preedit("글".into(), Some(0..3))),
            Event::InputMethod(Ime::Preedit(String::new(), None)),
            Event::InputMethod(Ime::Commit("글".into())),
            Event::InputMethod(Ime::Closed),
        ] {
            send(&mut ui, &mut renderer, &mut clipboard, point, event);
        }
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(
            state(&guest).text,
            "      abx\n한글x",
            "IME commit follows pending structural key exactly once"
        );
        assert_eq!(
            commits(&guest) - before_ime,
            2,
            "preedit lifecycle is not a text/history commit"
        );
        assert_eq!(
            state(&guest).cursor.position,
            wire::EditorPosition { line: 1, column: 6 }
        );
        fn sibling_key(node: &wire::Node) -> Option<String> {
            if node
                .key()
                .is_some_and(|key| key.ends_with("/same-document"))
            {
                return node.key().map(str::to_owned);
            }
            node.children().iter().find_map(sibling_key)
        }
        let sibling = sibling_key(guest.lock().unwrap().frame.root.as_ref().unwrap()).unwrap();
        let mut sibling_bounds = Bounds(sibling, None);
        ui.operate(&renderer, &mut sibling_bounds);
        let sibling_point = sibling_bounds.1.unwrap().center();
        for button in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            send(
                &mut ui,
                &mut renderer,
                &mut clipboard,
                sibling_point,
                Event::Mouse(button),
            );
        }
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        clipboard.0 = "shared".into();
        let before_sibling = commits(&guest);
        let selection_pixels = |ui: &mut Ui, renderer: &mut iced::Renderer| {
            ui.draw(
                renderer,
                &iced::Theme::Light,
                &iced::advanced::renderer::Style {
                    text_color: iced::Color::BLACK,
                },
                mouse::Cursor::Unavailable,
            );
            renderer
                .screenshot(Size::new(300, 480), 1.0, iced::Color::WHITE)
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[..3] == [255, 0, 0])
                .count()
        };
        let before_selection = selection_pixels(&mut ui, &mut renderer);
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            sibling_point,
            key(keyboard::Key::Character("a".into()), command),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert!(
            selection_pixels(&mut ui, &mut renderer) > before_selection + 20,
            "native selection paint follows queued shared-document caret state"
        );
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            sibling_point,
            key(keyboard::Key::Character("v".into()), command),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(
            state(&guest).text,
            "shared",
            "a rendering without claims shares document history and queue"
        );
        assert_eq!(commits(&guest) - before_sibling, 2);
        // Return to the binding-bearing rendering. Its first commit changes
        // claims; the queued F2 is consumed only by the new factory value.
        for button in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            send(
                &mut ui,
                &mut renderer,
                &mut clipboard,
                point,
                Event::Mouse(button),
            );
        }
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        let before_claim_change = commits(&guest);
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            named(Named::Tab),
        );
        send(
            &mut ui,
            &mut renderer,
            &mut clipboard,
            point,
            named(Named::F2),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, &mut clipboard);
        assert_eq!(state(&guest).text, "  shared");
        assert_eq!(
            commits(&guest) - before_claim_change,
            2,
            "second queued key sees claims updated by the first commit reducer"
        );
        drop(ui);
    }
}
