//! The counter's tasks, driven natively: a click produces a request in the
//! frame, the matching response completes the task, and the view shows it.

use app_store_counter::{boot_native, tick_native};
use app_store_sdk::frame::{Button, Event, Frame};

fn texts(frame: &Frame) -> Vec<String> {
    frame
        .layers
        .iter()
        .flat_map(|layer| layer.texts.iter().map(|text| text.content.clone()))
        .collect()
}

/// Where a line of text starts, so a test can click the button drawn around it.
fn find(frame: &Frame, content: &str) -> (f32, f32) {
    frame
        .layers
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .find(|text| text.content == content)
        .map(|text| (text.x + 4.0, text.y + 8.0))
        .unwrap_or_else(|| panic!("no text {content:?} in {:?}", texts(frame)))
}

fn click(frame: &Frame, content: &str) -> Vec<Event> {
    let (x, y) = find(frame, content);
    vec![
        Event::CursorMoved { x, y },
        Event::ButtonPressed(Button::Left),
        Event::ButtonReleased(Button::Left),
        Event::Redraw,
    ]
}

#[test]
fn a_question_goes_out_as_a_request_and_the_answer_comes_back_into_the_view() {
    boot_native();
    let frame = tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        Event::Redraw,
    ]);
    assert!(
        frame.requests.is_empty(),
        "nothing asked at boot: {:?}",
        frame.requests
    );

    let frame = tick_native(click(&frame, "Ask host"));
    let [request] = frame.requests.as_slice() else {
        panic!("one request after Ask host, got {:?}", frame.requests);
    };
    assert_eq!(request.kind, "echo");
    assert_eq!(request.payload, b"The count is 0. Still there?");

    let frame = tick_native(vec![
        Event::Response {
            id: request.id,
            payload: b"The store says: still here.".to_vec(),
        },
        Event::Redraw,
    ]);
    assert!(
        texts(&frame)
            .iter()
            .any(|t| t == "The store says: still here."),
        "{:?}",
        texts(&frame)
    );
}

#[test]
fn auto_mode_is_a_chain_of_timer_requests_that_stops_when_switched_off() {
    boot_native();
    let frame = tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        Event::Redraw,
    ]);

    let frame = tick_native(click(&frame, "Auto: off"));
    let [sleep] = frame.requests.as_slice() else {
        panic!("one timer request after Auto, got {:?}", frame.requests);
    };
    assert_eq!(sleep.kind, "sleep");
    assert_eq!(sleep.payload, 1000_i64.to_le_bytes());

    // The timer fires: the count moves and the next timer is requested.
    let frame = tick_native(vec![
        Event::Response {
            id: sleep.id,
            payload: Vec::new(),
        },
        Event::Redraw,
    ]);
    assert!(
        texts(&frame).iter().any(|t| t == "1"),
        "{:?}",
        texts(&frame)
    );
    let [next] = frame.requests.as_slice() else {
        panic!("the chain continues, got {:?}", frame.requests);
    };
    assert_eq!(next.kind, "sleep");

    // Switched off before the timer fires: the fire is ignored, no new timer.
    let frame = tick_native(click(&frame, "Auto: on"));
    assert!(
        frame.requests.is_empty(),
        "switching off asks for nothing: {:?}",
        frame.requests
    );
    let frame = tick_native(vec![
        Event::Response {
            id: next.id,
            payload: Vec::new(),
        },
        Event::Redraw,
    ]);
    assert!(
        texts(&frame).iter().any(|t| t == "1"),
        "count unchanged: {:?}",
        texts(&frame)
    );
    assert!(
        frame.requests.is_empty(),
        "chain ended: {:?}",
        frame.requests
    );
}
