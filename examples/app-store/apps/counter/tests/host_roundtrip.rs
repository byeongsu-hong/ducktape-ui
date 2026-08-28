//! The counter's tasks, driven natively: a click produces a request in the
//! frame, the matching response completes the task, and the view shows it.

use app_store_counter::{boot_native, tick_native};
use app_store_sdk::frame::{Event, Request};
use app_store_sdk::testing::{answer, click, has_text, redraw, texts};

fn boot() -> app_store_sdk::frame::Frame {
    boot_native();
    tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        redraw(),
    ])
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
    assert!(
        frame.requests.is_empty(),
        "nothing asked at boot: {:?}",
        frame.requests
    );

    let frame = tick_native(click(&frame, "Ask host"));
    let [request] = frame.requests.as_slice() else {
        panic!("one request after Ask host, got {:?}", frame.requests);
    };
    assert_eq!(request.kind, "host.echo");
    assert_eq!(request.payload, b"The count is 0. Still there?");

    let frame = tick_native(vec![
        answer(request.id, b"The store says: still here."),
        redraw(),
    ]);
    assert!(
        has_text(&frame, "The store says: still here."),
        "{:?}",
        texts(&frame)
    );
}

#[test]
fn a_change_is_published_on_the_bus_and_logged() {
    let frame = boot();
    let frame = tick_native(click(&frame, "+"));
    let [log, publish] = frame.requests.as_slice() else {
        panic!("a log and a publish after +, got {:?}", frame.requests);
    };
    assert_eq!(log.kind, "host.log");
    assert_eq!(log.payload, b"count is now 1");
    assert_eq!(publish.kind, "bus.publish");
    assert_eq!(publish.payload, b"counter\n1");
}

#[test]
fn auto_mode_is_a_chain_of_timer_requests_that_stops_when_switched_off() {
    let frame = boot();

    let frame = tick_native(click(&frame, "Auto: off"));
    let [sleep] = frame.requests.as_slice() else {
        panic!("one timer request after Auto, got {:?}", frame.requests);
    };
    assert_eq!(sleep.kind, "clock.sleep");
    assert_eq!(sleep.payload, 1000_i64.to_le_bytes());

    // The timer fires: the count moves, the next timer and a publish go out.
    let frame = tick_native(vec![answer(sleep.id, &[]), redraw()]);
    assert!(has_text(&frame, "1"), "{:?}", texts(&frame));
    let mut expected = kinds(&frame.requests);
    expected.sort();
    assert_eq!(
        expected,
        ["bus.publish", "clock.sleep", "host.log"],
        "{:?}",
        frame.requests
    );
    let next = frame
        .requests
        .iter()
        .find(|request| request.kind == "clock.sleep")
        .expect("the chain continues");

    // Switched off before the timer fires: the fire is ignored, no new timer.
    let frame = tick_native(click(&frame, "Auto: on"));
    assert!(
        frame.requests.is_empty(),
        "switching off asks for nothing: {:?}",
        frame.requests
    );
    let frame = tick_native(vec![answer(next.id, &[]), redraw()]);
    assert!(
        has_text(&frame, "1"),
        "count unchanged: {:?}",
        texts(&frame)
    );
    assert!(
        frame.requests.is_empty(),
        "chain ended: {:?}",
        frame.requests
    );
}
