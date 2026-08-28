//! The feed subscribes to every topic at boot and shows messages newest first.

use app_store_activity::{boot_native, tick_native};
use app_store_sdk::frame::Event;
use app_store_sdk::testing::{has_text, item, redraw, texts};

#[test]
fn bus_messages_become_rows_newest_first() {
    boot_native();
    let frame = tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        redraw(),
    ]);
    let [subscribe] = frame.requests.as_slice() else {
        panic!("one subscription at boot, got {:?}", frame.requests);
    };
    assert_eq!(subscribe.kind, "bus.subscribe");
    assert_eq!(subscribe.payload, b"*");
    assert!(has_text(&frame, "0 events"), "{:?}", texts(&frame));

    let frame = tick_native(vec![
        item(subscribe.id, b"counter\n3"),
        item(subscribe.id, b"todo\n2 items, 1 left"),
        redraw(),
    ]);
    let rows = texts(&frame);
    assert!(has_text(&frame, "2 events"), "{rows:?}");
    let todo = rows
        .iter()
        .position(|t| t == "2 items, 1 left")
        .expect("todo row");
    let counter = rows.iter().position(|t| t == "3").expect("counter row");
    assert!(todo < counter, "newest first: {rows:?}");
}
