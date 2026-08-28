//! Drives the app natively, the way the host drives it in wasm: the same
//! events in, the same frame out. Proves the loop without a window or a
//! wasm runtime, so a broken key path fails here first.

use app_store_sdk::frame::{Button, Event, Frame, Key};
use app_store_todo::{boot_native, tick_native};

fn texts(frame: &Frame) -> Vec<String> {
    frame
        .layers
        .iter()
        .flat_map(|layer| layer.texts.iter().map(|text| text.content.clone()))
        .collect()
}

fn click(x: f32, y: f32) -> Vec<Event> {
    vec![
        Event::CursorMoved { x, y },
        Event::ButtonPressed(Button::Left),
        Event::ButtonReleased(Button::Left),
    ]
}

fn typed(text: &str) -> Vec<Event> {
    text.chars()
        .map(|c| Event::KeyPressed {
            key: Key::Character(c.to_string()),
            modifiers: 0,
            text: Some(c.to_string()),
        })
        .collect()
}

#[test]
fn typing_into_the_input_and_adding_appends_a_row() {
    boot_native();
    let frame = tick_native(vec![
        Event::Resized {
            width: 760.0,
            height: 500.0,
        },
        Event::Redraw,
    ]);
    let initial = texts(&frame);
    assert!(
        initial.iter().any(|t| t == "Ship the recording renderer"),
        "{initial:?}"
    );
    assert!(
        initial.iter().any(|t| t == "2"),
        "two remaining at start: {initial:?}"
    );

    // The input sits under the title, spanning most of the width.
    let mut events = click(200.0, 116.0);
    events.extend(typed("Hello"));
    events.push(Event::Redraw);
    let frame = tick_native(events);
    let after_typing = texts(&frame);
    assert!(
        after_typing.iter().any(|t| t == "Hello"),
        "draft is drawn: {after_typing:?}"
    );

    let mut events = click(715.0, 102.0);
    events.push(Event::Redraw);
    let frame = tick_native(events);
    let after_add = texts(&frame);
    assert_eq!(
        after_add.iter().filter(|t| *t == "Hello").count(),
        1,
        "one row, empty draft: {after_add:?}"
    );
    assert!(
        after_add.iter().any(|t| t == "3"),
        "three remaining: {after_add:?}"
    );
}
