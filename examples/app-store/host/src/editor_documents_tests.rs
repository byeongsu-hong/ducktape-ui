//! Actual native/Wasm document transfer, visible native editing and no-init reload.
use super::reload::finish_reload;
use super::*;
use iced::advanced::{renderer::Headless, widget::Operation};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, keyboard, mouse, window};
use iced_test::runtime::{UserInterface, user_interface};
type Ui = UserInterface<'static, String, iced::Theme, iced::Renderer>;
const SIZE: Size = Size::new(720.0, 400.0);
fn build(
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    cache: user_interface::Cache,
) -> Ui {
    UserInterface::build(
        wasm_view(Surface(guest.clone()), false),
        SIZE,
        cache,
        renderer,
    )
}
fn settle(
    mut ui: Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    now: &mut Instant,
    phase: &str,
) -> Ui {
    // A transfer uses Begin + <=16 chunks + Complete + Ack, with a
    // possible redraw between ticks. Bound by the protocol's document count.
    let redraws = wire::editor_document::MAX_EDITOR_DOCUMENTS
        * (wire::editor_document::MAX_EDITOR_CHUNKS + 3)
        * 2;
    let mut max_fuel = 0;
    for _ in 0..redraws {
        *now = (*now + Duration::from_millis(16)).max(Instant::now());
        let events = {
            let state = guest.lock().unwrap();
            state
                .pending
                .iter()
                .map(|event| match event {
                    wire::Event::EditorKeyRequest { .. } => "decision",
                    wire::Event::EditorTransaction { .. } => "commit",
                    _ => "other",
                })
                .collect::<Vec<_>>()
        };
        let mut messages = vec![];
        ui.update(
            &[Event::Window(window::Event::RedrawRequested(*now))],
            mouse::Cursor::Unavailable,
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        {
            let state = guest.lock().unwrap();
            max_fuel = max_fuel.max(state.fuel_used);
            assert!(
                state.fault.is_none(),
                "document guest fault during {phase}, incoming={events:?}: {:?}",
                state.fault
            );
            if !state.inputs.editor_documents_status().unwrap() {
                assert!(
                    !state.quiet(*now),
                    "a live document transfer must keep producing chunks, not become a quiet guest"
                );
            }
        }
        if messages.iter().any(|message| message == "wake") {
            ui = build(guest, renderer, ui.into_cache());
        }
        let state = guest.lock().unwrap();
        assert!(
            !(state.inputs.editor_documents_status().unwrap()
                && state.inputs.editor_transactions_pending()
                && state.quiet(*now)),
            "a native input on a complete one-MiB document must not become a stranded faulted lane"
        );
        if state.frame.root.is_some()
            && state.inputs.editor_documents_status().unwrap()
            && !state.inputs.editor_transactions_pending()
            && state.pending.is_empty()
            && !state.staged_frame
            && !state.frame.busy
        {
            assert!(
                state.quiet(*now),
                "a completed document handshake returns to idle without spinning"
            );
            eprintln!("large document phase={phase} max_tick_fuel={max_fuel}");
            return ui;
        }
    }
    let state = guest.lock().unwrap();
    fn readiness(node: &wire::Node, inputs: &Inputs, out: &mut Vec<(String, Option<usize>)>) {
        if let wire::Node::Editor { key, .. } = node {
            out.push((
                key.clone(),
                inputs.editor_document(key).map(|doc| doc.text().len()),
            ));
        }
        for child in node.children() {
            readiness(child, inputs, out);
        }
    }
    let mut documents = vec![];
    if let Some(root) = &state.frame.root {
        readiness(root, &state.inputs, &mut documents);
    }
    panic!(
        "document did not settle: documents={documents:?} status={:?} lane={} pending={} staged={} busy={} ticks={}",
        state.inputs.editor_documents_status(),
        state.inputs.editor_transactions_pending(),
        state.pending.len(),
        state.staged_frame,
        state.frame.busy,
        state.ticks
    );
}
fn editor(node: &wire::Node) -> Option<(&str, &wire::editor_document::EditorDocumentRef)> {
    if let wire::Node::Editor { key, document, .. } = node {
        Some((key, document))
    } else {
        node.children().iter().find_map(editor)
    }
}
fn document(guest: &Arc<Mutex<Guest>>) -> (String, wire::editor_document::EditorDocumentRef) {
    let state = guest.lock().unwrap();
    let (key, _) = editor(state.frame.root.as_ref().unwrap()).expect("editor reference");
    let document = state
        .inputs
        .editor_document(key)
        .expect("complete canonical document");
    (document.text().to_owned(), document.reference())
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
fn key(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Event {
    let text = if let keyboard::Key::Character(text) = &key {
        Some(text.clone())
    } else {
        None
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
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, point: Point, event: Event) {
    ui.update(
        &[event],
        mouse::Cursor::Available(point),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
#[test]
#[ignore = "requires current native and Wasm editor-documents fixtures"]
fn native_and_wasm_one_mib_documents_edit_restore_and_undo_without_reinitializing() {
    use keyboard::key::Named;
    for native in [false, true] {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
            "../target/editor-documents-native"
        } else {
            "../target/editor-documents-fixture"
        });
        let entries = crate::catalog::scan_dir(&directory);
        assert_eq!(
            entries.len(),
            1,
            "build the current large document fixture ({native})"
        );
        let entry = entries[0].clone();
        let guest = Arc::new(Mutex::new(Guest::load(&entry).unwrap()));
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut ui = build(&guest, &mut renderer, user_interface::Cache::default());
        let mut now = Instant::now();
        ui = settle(ui, &guest, &mut renderer, &mut now, "initial transfer");
        eprintln!("large document backend native={native}: complete and idle");
        let (initial, initial_ref) = document(&guest);
        assert_eq!(
            initial.len(),
            wire::editor_document::MAX_EDITOR_DOCUMENT_BYTES
        );
        assert_eq!(initial_ref.byte_len as usize, initial.len());
        let key_id = editor(guest.lock().unwrap().frame.root.as_ref().unwrap())
            .unwrap()
            .0
            .to_owned();
        let mut bounds = Bounds(key_id, None);
        ui.operate(&renderer, &mut bounds);
        let bounds = bounds
            .1
            .expect("actual native editor, not a loading placeholder");
        ui.draw(
            &mut renderer,
            &iced::Theme::Light,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::BLACK,
            },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(720, 400), 1.0, iced::Color::WHITE);
        let mut ink = 0;
        for y in bounds.y as usize + 8..(bounds.y + bounds.height - 8.0) as usize {
            for x in bounds.x as usize + 8..(bounds.x + bounds.width - 8.0).min(720.0) as usize {
                let at = (y * 720 + x) * 4;
                if pixels[at..at + 3].iter().all(|value| *value < 100) {
                    ink += 1;
                }
            }
        }
        assert!(
            ink > 500,
            "one-MiB document paints native text within the editor ({ink} pixels)"
        );
        let point = Point::new(bounds.x + 20.0, bounds.y + 10.0);
        send(
            &mut ui,
            &mut renderer,
            point,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        );
        send(
            &mut ui,
            &mut renderer,
            point,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        );
        send(
            &mut ui,
            &mut renderer,
            point,
            key(keyboard::Key::Named(Named::Home), keyboard::Modifiers::CTRL),
        );
        for _ in 0..5 {
            send(
                &mut ui,
                &mut renderer,
                point,
                key(
                    keyboard::Key::Named(Named::ArrowRight),
                    keyboard::Modifiers::empty(),
                ),
            );
        }
        send(
            &mut ui,
            &mut renderer,
            point,
            key(
                keyboard::Key::Named(Named::Delete),
                keyboard::Modifiers::empty(),
            ),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, "native deletion");
        let (edited, edited_ref) = document(&guest);
        assert_eq!(edited.len(), initial.len() - 1);
        assert!(
            edited.starts_with("012346789"),
            "ordered native caret and deletion reached the large mirror"
        );
        assert_eq!(
            edited_ref.cursor.position,
            wire::EditorPosition { line: 0, column: 5 }
        );
        let running = Running {
            id: entry.id.clone(),
            name: entry.name.clone(),
            surface: Surface(guest.clone()),
            window: iced::window::Id::unique(),
        };
        let prepared =
            iced::futures::executor::block_on(prepare_reload(entry, vec![running.clone()], 1));
        finish_reload(std::slice::from_ref(&running), 1, prepared).expect(
            "large snapshot must restore and finish all document chunks before installation",
        );
        // Before any redraw/props: the installed candidate already owns the
        // non-default unsaved text and exact caret, with no boot replay.
        assert_eq!(document(&guest), (edited, edited_ref));
        ui = build(&guest, &mut renderer, ui.into_cache());
        // No focus operation or pointer input is injected after replacement:
        // the next real key must reach the newly created native editor.
        let command = if cfg!(target_os = "macos") {
            keyboard::Modifiers::LOGO
        } else {
            keyboard::Modifiers::CTRL
        };
        send(
            &mut ui,
            &mut renderer,
            point,
            key(keyboard::Key::Character("z".into()), command),
        );
        assert!(
            guest.lock().unwrap().inputs.editor_transactions_pending(),
            "Undo must enter the replacement editor lane"
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, "Undo after swap");
        let undone = document(&guest);
        assert!(
            undone.0 == initial,
            "guest-owned undo history survives the same no-init swap: bytes={} revision={}",
            undone.0.len(),
            undone.1.revision
        );
        send(
            &mut ui,
            &mut renderer,
            point,
            key(keyboard::Key::Character("a".into()), command),
        );
        send(
            &mut ui,
            &mut renderer,
            point,
            key(
                keyboard::Key::Named(Named::Delete),
                keyboard::Modifiers::empty(),
            ),
        );
        ui = settle(ui, &guest, &mut renderer, &mut now, "select-all deletion");
        assert!(
            document(&guest).0.is_empty(),
            "native select-all deletion commits the complete document"
        );
        send(
            &mut ui,
            &mut renderer,
            point,
            key(keyboard::Key::Character("z".into()), command),
        );
        let _ui = settle(ui, &guest, &mut renderer, &mut now, "whole-document Undo");
        assert_eq!(
            document(&guest).0,
            initial,
            "Undo atomically reinserts one MiB, exceeding the display string budget"
        );
    }
}
