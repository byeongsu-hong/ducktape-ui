//! The todo, driven natively: it loads from storage at boot, typing and
//! Add append a row, and every change is written back then announced.

use app_store_todo::items::{Item, decode, encode};
use app_store_todo::{boot_native, tick_native};
use ui_lang_guest::testing::{
    answer, edit, find, has_text, item, keys, press, slide, texts, toggle, type_into,
};
use ui_lang_guest::wire::{Frame, Node, Request, Rgba};

fn boot_with(stored: &[Item]) -> Frame {
    boot_with_notes(stored, "")
}

fn boot_with_notes(stored: &[Item], notes: &str) -> Frame {
    boot_native();
    let frame = tick_native(Vec::new());
    // The two loads and the colour-mode subscription, in whichever order
    // the parallel group started them.
    assert_eq!(frame.requests.len(), 3, "{:?}", frame.requests);
    request_for(&frame.requests, "host.theme", b"");
    let load = request_for(&frame.requests, "storage.get", b"items");
    let load_notes = request_for(&frame.requests, "storage.get", b"notes");
    tick_native(vec![
        answer(load.id, &encode(stored)),
        answer(load_notes.id, notes.as_bytes()),
    ])
}

/// The request of `kind` whose payload is `payload`; any payload for the
/// one kind sent once.
fn request_for<'a>(requests: &'a [Request], kind: &str, payload: &[u8]) -> &'a Request {
    requests
        .iter()
        .find(|request| {
            request.kind == kind
                && (request.payload == payload
                    || requests.iter().filter(|other| other.kind == kind).count() == 1)
        })
        .unwrap_or_else(|| panic!("no {kind} {payload:?} in {requests:?}"))
}

#[test]
fn an_empty_store_shows_the_seed_list() {
    let frame = boot_with(&[]);
    assert!(
        has_text(&frame, "Ship the recording renderer"),
        "{:?}",
        texts(&frame)
    );
    assert!(has_text(&frame, "2 left"), "{:?}", texts(&frame));
}

#[test]
fn typing_into_the_input_and_adding_appends_a_row_and_saves_it() {
    let stored = vec![Item {
        id: 4,
        text: "Already here".into(),
        done: true,
        priority: 0,
    }];
    let frame = boot_with(&stored);
    assert!(has_text(&frame, "Already here"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "0 left"), "{:?}", texts(&frame));

    // The host owns the text; the guest hears the whole value, then Add.
    let frame = tick_native(type_into(&frame, "What needs doing?", "Hello"));
    assert!(
        has_text(&frame, "Hello"),
        "the draft echoes: {:?}",
        texts(&frame)
    );
    let frame = tick_native(press(&frame, "Add"));
    assert!(has_text(&frame, "Hello"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "1 left"), "{:?}", texts(&frame));
    assert!(
        has_text(&frame, "What needs doing?"),
        "the draft is cleared: {:?}",
        texts(&frame)
    );

    let [save] = frame.requests.as_slice() else {
        panic!("one save after Add, got {:?}", frame.requests);
    };
    assert_eq!(save.kind, "storage.set");
    let (key, body) = save.payload.split_at(6);
    assert_eq!(key, b"items\n");
    let written = decode(body);
    assert_eq!(written.len(), 2);
    assert_eq!(written[1].id, 5, "ids continue after the stored ones");
    assert_eq!(written[1].text, "Hello");

    // The write is acknowledged, then the bus hears about it.
    let frame = tick_native(vec![answer(save.id, &[])]);
    let [publish] = frame.requests.as_slice() else {
        panic!("one publish after the save, got {:?}", frame.requests);
    };
    assert_eq!(publish.kind, "bus.publish");
    assert_eq!(publish.payload, b"todo\n2 items, 1 left");
    let frame = tick_native(vec![answer(publish.id, &[])]);
    assert!(has_text(&frame, "saved 2 items"), "{:?}", texts(&frame));
}

/// The notes are an `editor` the host edits: the guest hears the whole text
/// and echoes it, and Save writes it under its own storage key.
#[test]
fn editing_the_notes_echoes_them_and_save_writes_them() {
    let frame = boot_with_notes(&[], "carried over");
    let Some(Node::Editor { text, on_edit, .. }) = find(&frame, "Todo/app/content/notes") else {
        panic!("no editor in {:?}", keys(&frame));
    };
    assert_eq!(text, "carried over");
    assert!(on_edit.is_some(), "the editor is enabled");
    assert!(has_text(&frame, "carried over"), "{:?}", texts(&frame));

    let frame = tick_native(edit(&frame, "Notes", "buy milk\nand eggs"));
    let Some(Node::Editor { text, .. }) = find(&frame, "Todo/app/content/notes") else {
        panic!("no editor in {:?}", keys(&frame));
    };
    assert_eq!(
        text, "buy milk\nand eggs",
        "the guest echoes the host's text"
    );
    assert!(
        frame.requests.is_empty(),
        "typing saves nothing: {:?}",
        frame.requests
    );

    let frame = tick_native(press(&frame, "Save notes"));
    let [save] = frame.requests.as_slice() else {
        panic!("one save after Save notes, got {:?}", frame.requests);
    };
    assert_eq!(save.kind, "storage.set");
    assert_eq!(save.payload, b"notes\nbuy milk\nand eggs");
    let frame = tick_native(vec![answer(save.id, &[])]);
    assert!(has_text(&frame, "saved notes"), "{:?}", texts(&frame));
}

/// A row's checkbox and the footer's toggler are the form controls the wire
/// carries: the checkbox's bool comes back as the item's toggle, the
/// toggler's bool hides the done rows, and the progress bar follows.
#[test]
fn checking_an_item_marks_it_done_and_hiding_done_removes_its_row() {
    let stored = vec![
        Item {
            id: 1,
            text: "Already here".into(),
            done: false,
            priority: 0,
        },
        Item {
            id: 2,
            text: "Long gone".into(),
            done: true,
            priority: 0,
        },
    ];
    let frame = boot_with(&stored);
    assert!(has_text(&frame, "1 left"), "{:?}", texts(&frame));
    assert_eq!(progress(&frame), 0.5);

    let frame = tick_native(toggle(&frame, "Already here", true));
    assert!(has_text(&frame, "0 left"), "{:?}", texts(&frame));
    assert_eq!(progress(&frame), 1.0);
    let Some(Node::Toggle { checked: true, .. }) = row_checkbox(&frame, "Already here") else {
        panic!("the row's checkbox follows the item: {:?}", texts(&frame));
    };
    // The change is saved like any other.
    assert_eq!(frame.requests.len(), 1, "{:?}", frame.requests);
    assert_eq!(frame.requests[0].kind, "storage.set");

    let frame = tick_native(toggle(&frame, "Hide done", true));
    assert!(
        !has_text(&frame, "Already here") && !has_text(&frame, "Long gone"),
        "done rows are hidden: {:?}",
        texts(&frame)
    );
    let Some(Node::Toggle { checked: true, .. }) = find(&frame, "Todo/app/content/hide") else {
        panic!("the toggler shows its state: {:?}", texts(&frame));
    };
    let frame = tick_native(toggle(&frame, "Hide done", false));
    assert!(has_text(&frame, "Long gone"), "{:?}", texts(&frame));
}

/// Each row's slider routes with the row's item id, an argument bound by the
/// `for` around it: dragging the second row's slider changes that item alone,
/// and the change is saved with the priority in its line.
#[test]
fn dragging_a_rows_slider_sets_that_items_priority() {
    let stored = vec![
        Item {
            id: 1,
            text: "First".into(),
            done: false,
            priority: 0,
        },
        Item {
            id: 2,
            text: "Second".into(),
            done: false,
            priority: 0,
        },
    ];
    let frame = boot_with(&stored);
    assert_eq!(priorities(&frame), [0.0, 0.0]);

    let second = row_sliders(&frame).remove(1);
    let frame = tick_native(slide(&frame, &second, 2.0));
    assert_eq!(priorities(&frame), [0.0, 2.0]);
    assert_eq!(frame.requests.len(), 1, "{:?}", frame.requests);
    assert_eq!(frame.requests[0].kind, "storage.set");
    let mut saved = stored;
    saved[1].priority = 2;
    assert_eq!(
        frame.requests[0].payload,
        [b"items\n".as_slice(), &encode(&saved)].concat()
    );
}

/// The row sliders' keys, in row order; rows are unidentified, so the key
/// is the loop scope's plus the slider's origin.
fn row_sliders(frame: &Frame) -> Vec<String> {
    keys(frame)
        .into_iter()
        .filter(|key| key.contains("@slider"))
        .collect()
}

fn priorities(frame: &Frame) -> Vec<f32> {
    row_sliders(frame)
        .iter()
        .map(|key| match find(frame, key) {
            Some(Node::Slider { value, .. }) => *value,
            other => panic!("{key} is not a slider: {other:?}"),
        })
        .collect()
}

fn progress(frame: &Frame) -> f32 {
    let Some(Node::Progress { value, .. }) = find(frame, "Todo/app/content/done") else {
        panic!("no progress bar in {:?}", texts(frame));
    };
    *value
}

fn row_checkbox<'a>(frame: &'a Frame, label: &str) -> Option<&'a Node> {
    // Rows are unidentified: the checkbox is found by its label.
    let mut stack = vec![frame.root.as_ref()?];
    while let Some(node) = stack.pop() {
        match node {
            Node::Toggle { label: text, .. } if text == label => return Some(node),
            Node::Container { content, .. } | Node::Scroll { content, .. } => stack.push(content),
            Node::Linear { children, .. } => stack.extend(children.iter()),
            _ => {}
        }
    }
    None
}

/// The app's own backdrop: the root container's background.
fn backdrop(frame: &Frame) -> Option<Rgba> {
    match frame.root.as_ref()? {
        Node::Container { background, .. } => *background,
        _ => None,
    }
}

/// The colour mode is a host stream like any other: one item repaints the app
/// in the host's palette.
#[test]
fn the_hosts_dark_mode_repaints_the_app() {
    boot_native();
    let light = tick_native(Vec::new());
    let theme = request_for(&light.requests, "host.theme", b"");
    let lit = backdrop(&light).expect("the boot frame paints the app");
    let dark = tick_native(vec![item(theme.id, b"dark")]);
    assert_ne!(
        backdrop(&dark).expect("still painted"),
        lit,
        "the app's backdrop follows the host's colour mode"
    );
}
