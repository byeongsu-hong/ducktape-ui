//! The todo, driven natively: it loads from storage at boot, typing and
//! Add append a row, and every change is written back then announced.

use app_store_todo::items::{Item, decode, encode};
use app_store_todo::{boot_native, tick_native};
use ui_lang_guest::testing::{answer, has_text, item, press, texts, type_into};
use ui_lang_guest::wire::{Frame, Node, Request, Rgba};

fn boot_with(stored: &[Item]) -> Frame {
    boot_native();
    let frame = tick_native(Vec::new());
    // The load and the colour-mode subscription, in whichever order the
    // parallel group started them.
    assert_eq!(frame.requests.len(), 2, "{:?}", frame.requests);
    request_for(&frame.requests, "host.theme");
    let load = request_for(&frame.requests, "storage.get");
    assert_eq!(load.payload, b"items");
    tick_native(vec![answer(load.id, &encode(stored))])
}

fn request_for<'a>(requests: &'a [Request], kind: &str) -> &'a Request {
    requests
        .iter()
        .find(|request| request.kind == kind)
        .unwrap_or_else(|| panic!("no {kind} in {requests:?}"))
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
    let theme = request_for(&light.requests, "host.theme");
    let lit = backdrop(&light).expect("the boot frame paints the app");
    let dark = tick_native(vec![item(theme.id, b"dark")]);
    assert_ne!(
        backdrop(&dark).expect("still painted"),
        lit,
        "the app's backdrop follows the host's colour mode"
    );
}
