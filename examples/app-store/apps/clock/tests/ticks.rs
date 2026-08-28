//! The clock subscribes at boot, asks the wall clock once, and shows every
//! tick the host streams against both.

use app_store_clock::{boot_native, tick_native};
use app_store_sdk::frame::{Event, Request};
use app_store_sdk::testing::{answer, has_text, item, redraw, texts};

/// 2025-01-01T13:45:00Z.
const NOW_MS: u64 = 1_735_739_100_000;

fn request_for<'a>(requests: &'a [Request], kind: &str) -> &'a Request {
    requests
        .iter()
        .find(|request| request.kind == kind)
        .unwrap_or_else(|| panic!("no {kind} in {requests:?}"))
}

#[test]
fn ticks_arrive_as_a_stream_and_move_the_display() {
    boot_native();
    let frame = tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        redraw(),
    ]);
    // One subscription and one question: the wall clock is asked for once,
    // never per tick.
    assert_eq!(frame.requests.len(), 2, "{:?}", frame.requests);
    let subscribe = request_for(&frame.requests, "clock.ticks");
    assert_eq!(subscribe.payload, 1000_i64.to_le_bytes());
    let now = request_for(&frame.requests, "clock.now");
    assert!(now.payload.is_empty(), "{:?}", now);
    assert!(has_text(&frame, "00:00"), "{:?}", texts(&frame));

    let frame = tick_native(vec![answer(now.id, &NOW_MS.to_le_bytes()), redraw()]);
    assert!(has_text(&frame, "13:45:00 UTC"), "{:?}", texts(&frame));

    let frame = tick_native(vec![item(subscribe.id, &5_000_u64.to_le_bytes()), redraw()]);
    assert!(has_text(&frame, "00:05"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "13:45:05 UTC"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "1 ticks received"), "{:?}", texts(&frame));

    let frame = tick_native(vec![
        item(subscribe.id, &65_000_u64.to_le_bytes()),
        redraw(),
    ]);
    assert!(has_text(&frame, "01:05"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "13:46:05 UTC"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "○○●○○○○○○○"), "{:?}", texts(&frame));
    assert!(
        frame.requests.is_empty(),
        "one subscription serves forever: {:?}",
        frame.requests
    );
}
