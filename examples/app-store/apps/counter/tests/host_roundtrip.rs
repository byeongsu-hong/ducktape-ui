//! The counter's tasks, driven natively: a press produces a request in the
//! frame, the matching response completes the task, and the view shows it.

use app_store_counter::{boot_native, tick_native};
use ui_lang_guest::testing::{answer, find, has_text, item, press, texts};
use ui_lang_guest::wire::{Frame, Request};

fn boot() -> Frame {
    boot_native();
    tick_native(Vec::new())
}

fn kinds(requests: &[Request]) -> Vec<&str> {
    requests
        .iter()
        .map(|request| request.kind.as_str())
        .collect()
}

#[test]
fn a_question_goes_out_as_a_request_and_the_answer_comes_back_into_the_view() {
    let frame = boot();
    assert_eq!(
        kinds(&frame.requests),
        ["host.theme"],
        "only the colour mode at boot: {:?}",
        frame.requests
    );

    let frame = tick_native(press(&frame, "Ask host"));
    let [request] = frame.requests.as_slice() else {
        panic!("one request after Ask host, got {:?}", frame.requests);
    };
    assert_eq!(request.kind, "host.echo");
    assert_eq!(request.payload, b"The count is 0. Still there?");

    let frame = tick_native(vec![answer(request.id, b"The store says: still here.")]);
    assert!(
        has_text(&frame, "The store says: still here."),
        "{:?}",
        texts(&frame)
    );
}

#[test]
fn a_change_is_published_on_the_bus_and_logged() {
    let frame = boot();
    let frame = tick_native(press(&frame, "+"));
    let [log, publish] = frame.requests.as_slice() else {
        panic!("a log and a publish after +, got {:?}", frame.requests);
    };
    assert_eq!(log.kind, "host.log");
    assert_eq!(log.payload, b"count is now 1");
    assert_eq!(publish.kind, "bus.publish");
    assert_eq!(publish.payload, b"counter\n1");
}

/// Auto is an Ice `subscribe every ... when auto`: switching it on starts one
/// host ticker, every state change in between keeps it — the recipe hashes
/// the same, so the stream lives on — and switching it off cancels it.
#[test]
fn auto_is_a_subscription_that_keeps_its_ticker_until_switched_off() {
    let frame = boot();

    let frame = tick_native(press(&frame, "Auto: off"));
    let [ticks] = frame.requests.as_slice() else {
        panic!("one ticker after Auto, got {:?}", frame.requests);
    };
    assert_eq!(ticks.kind, "clock.ticks");
    assert_eq!(ticks.payload, 1000_i64.to_le_bytes());
    assert!(frame.cancels.is_empty(), "{:?}", frame.cancels);

    // The ticker fires: the count moves and a publish goes out — and no new
    // timer, because one subscription serves for as long as `auto` holds.
    let frame = tick_native(vec![item(ticks.id, &1_000_u64.to_le_bytes())]);
    assert!(has_text(&frame, "1"), "{:?}", texts(&frame));
    let mut expected = kinds(&frame.requests);
    expected.sort();
    assert_eq!(
        expected,
        ["bus.publish", "host.log"],
        "{:?}",
        frame.requests
    );

    // Other state moves, the subscription does not: same recipe, same stream.
    let frame = tick_native(press(&frame, "+"));
    assert!(has_text(&frame, "2"), "{:?}", texts(&frame));
    assert!(
        !frame
            .requests
            .iter()
            .any(|request| request.kind == "clock.ticks"),
        "a subscription that stays the same keeps its ticker: {:?}",
        frame.requests
    );
    assert!(frame.cancels.is_empty(), "{:?}", frame.cancels);
    let frame = tick_native(vec![item(ticks.id, &2_000_u64.to_le_bytes())]);
    assert!(has_text(&frame, "3"), "{:?}", texts(&frame));

    // Off: the recipe is gone, so the stream is dropped and the host is told
    // to stop the ticker. A tick that was already on its way changes nothing.
    let frame = tick_native(press(&frame, "Auto: on"));
    assert_eq!(frame.cancels, vec![ticks.id], "{:?}", frame.cancels);
    assert!(frame.requests.is_empty(), "{:?}", frame.requests);
    let frame = tick_native(vec![item(ticks.id, &3_000_u64.to_le_bytes())]);
    assert!(
        has_text(&frame, "3"),
        "count unchanged: {:?}",
        texts(&frame)
    );
    assert!(frame.requests.is_empty(), "{:?}", frame.requests);
}

/// A tick that changes nothing says so, and the tree still reads the same.
#[test]
fn an_idle_tick_is_unchanged() {
    let frame = boot();
    assert!(!frame.unchanged, "the first tree is new");
    let frame = tick_native(Vec::new());
    assert!(frame.unchanged);
    assert!(has_text(&frame, "Counter"), "{:?}", texts(&frame));
}

/// The icon's bytes cross once: the first frame carries them under their
/// hash, and a later frame that rebuilds the tree names the hash alone.
#[test]
fn a_picture_crosses_once_and_its_hash_stands_for_it_after() {
    use ui_lang_guest::wire::Node;

    let frame = boot();
    let Some(Node::Svg {
        hash: first_hash,
        bytes: Some(bytes),
        ..
    }) = find(&frame, "Counter/app/content/icon")
    else {
        panic!("the first frame carries the icon: {:?}", frame.root);
    };
    assert!(bytes.starts_with(b"<svg"), "{:?}", &bytes[..8]);

    let frame = tick_native(press(&frame, "+"));
    let Some(Node::Svg {
        hash: second_hash,
        bytes: None,
        ..
    }) = find(&frame, "Counter/app/content/icon")
    else {
        panic!("the second frame names the hash alone: {:?}", frame.root);
    };
    assert_eq!(first_hash, second_hash);
}
