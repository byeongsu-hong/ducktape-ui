//! The todo, driven natively: it loads from storage at boot, typing and
//! Add append a row, and every change is written back then announced.

use app_store_sdk::frame::{Event, Key};
use app_store_sdk::testing::{answer, click, has_text, redraw, texts};
use app_store_todo::items::{Item, decode, encode};
use app_store_todo::{boot_native, tick_native};

fn boot_with(stored: &[Item]) -> app_store_sdk::frame::Frame {
    boot_native();
    let frame = tick_native(vec![
        Event::Resized {
            width: 760.0,
            height: 500.0,
        },
        redraw(),
    ]);
    let [load] = frame.requests.as_slice() else {
        panic!("one load at boot, got {:?}", frame.requests);
    };
    assert_eq!(load.kind, "storage.get");
    assert_eq!(load.payload, b"items");
    tick_native(vec![answer(load.id, &encode(stored)), redraw()])
}

fn type_text(text: &str) -> Vec<Event> {
    text.chars()
        .flat_map(|c| {
            [
                Event::KeyPressed {
                    key: Key::Character(c.to_string()),
                    modifiers: 0,
                    text: Some(c.to_string()),
                },
                Event::KeyReleased {
                    key: Key::Character(c.to_string()),
                    modifiers: 0,
                },
            ]
        })
        .chain([redraw()])
        .collect()
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

    // Focus the input by clicking it, then type and press Add.
    let (x, y) = app_store_sdk::testing::find(&frame, "What needs doing?");
    tick_native(vec![
        Event::CursorMoved { x, y: y + 40.0 },
        Event::ButtonPressed(app_store_sdk::frame::Button::Left),
        Event::ButtonReleased(app_store_sdk::frame::Button::Left),
        redraw(),
    ]);
    let frame = tick_native(type_text("Hello"));
    let frame = tick_native(click(&frame, "Add"));
    assert!(has_text(&frame, "Hello"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "1 left"), "{:?}", texts(&frame));

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
    let frame = tick_native(vec![answer(save.id, &[]), redraw()]);
    let [publish] = frame.requests.as_slice() else {
        panic!("one publish after the save, got {:?}", frame.requests);
    };
    assert_eq!(publish.kind, "bus.publish");
    assert_eq!(publish.payload, b"todo\n2 items, 1 left");
    let frame = tick_native(vec![answer(publish.id, &[]), redraw()]);
    assert!(has_text(&frame, "saved 2 items"), "{:?}", texts(&frame));
}
