//! The feed subscribes to every topic at boot and shows messages newest
//! first, each with who published it and under what topic.

use app_store_activity::{boot_native, tick_native};
use ui_lang_guest::testing::{has_text, item, keys, measure, texts};

#[test]
fn bus_messages_become_rows_newest_first() {
    boot_native();
    let frame = tick_native(Vec::new());
    let [subscribe, theme] = frame.requests.as_slice() else {
        panic!("the bus and the theme at boot, got {:?}", frame.requests);
    };
    assert_eq!(subscribe.kind, "bus.subscribe");
    assert_eq!(subscribe.payload, b"*");
    assert_eq!(theme.kind, "host.theme");
    assert!(has_text(&frame, "0 events"), "{:?}", texts(&frame));

    // `from\ntopic\ntext`: the host fills in `from` itself.
    let frame = tick_native(vec![
        item(subscribe.id, b"app_store_counter\ncounter\n3"),
        item(subscribe.id, b"app_store_todo\ntodo\n2 items, 1 left"),
    ]);
    let rows = texts(&frame);
    assert!(has_text(&frame, "2 events"), "{rows:?}");
    assert!(has_text(&frame, "app_store_counter · counter"), "{rows:?}");
    assert!(has_text(&frame, "app_store_todo · todo"), "{rows:?}");
    let todo = rows
        .iter()
        .position(|t| t == "2 items, 1 left")
        .expect("todo row");
    let counter = rows.iter().position(|t| t == "3").expect("counter row");
    assert!(todo < counter, "newest first: {rows:?}");
}

/// The host measures the feed after layout and the guest turns the height
/// into a row count; a re-measure with more room shows more rows.
#[test]
fn the_measured_feed_height_becomes_a_visible_row_count() {
    boot_native();
    let frame = tick_native(Vec::new());
    let subscribe = frame.requests[0].id;
    let frame = tick_native(
        (0..6)
            .map(|n| {
                item(
                    subscribe,
                    format!("app_store_counter\ncounter\n{n}").as_bytes(),
                )
            })
            .collect(),
    );
    assert!(has_text(&frame, "0 rows visible"), "{:?}", texts(&frame));
    let watch = keys(&frame)
        .into_iter()
        .find(|key| key.ends_with("/watch"))
        .expect("the feed sensor");

    let frame = tick_native(measure(&frame, &watch, 432.0, 200.0));
    assert!(has_text(&frame, "4 rows visible"), "{:?}", texts(&frame));
    let frame = tick_native(measure(&frame, &watch, 432.0, 1000.0));
    assert!(has_text(&frame, "6 rows visible"), "{:?}", texts(&frame));
}
