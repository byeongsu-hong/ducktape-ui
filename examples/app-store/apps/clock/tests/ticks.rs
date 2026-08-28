//! The clock subscribes at boot, asks the wall clock once, and shows every
//! tick the host streams against both.

use app_store_clock::{boot_native, tick_native};
use app_store_sdk::frame::{Event, Request};
use app_store_sdk::testing::{answer, has_text, item, redraw, texts};

/// 2025-01-01T13:45:00Z.
const NOW_MS: u64 = 1_735_739_100_000;

/// The store had been up five minutes when it answered: the app must anchor
/// the wall clock to zero uptime, not to the moment it asked, or every label
/// is those five minutes fast for the life of the instance.
const UPTIME_AT_ANSWER_MS: u64 = 300_000;

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

    let mut when = NOW_MS.to_le_bytes().to_vec();
    when.extend_from_slice(&UPTIME_AT_ANSWER_MS.to_le_bytes());
    let frame = tick_native(vec![answer(now.id, &when), redraw()]);
    // No tick yet, so the app's own uptime is still zero: five minutes before
    // the wall clock the host just read.
    assert!(has_text(&frame, "13:40:00 UTC"), "{:?}", texts(&frame));

    let frame = tick_native(vec![item(subscribe.id, &5_000_u64.to_le_bytes()), redraw()]);
    assert!(has_text(&frame, "00:05"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "13:40:05 UTC"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "1 ticks received"), "{:?}", texts(&frame));

    let frame = tick_native(vec![
        item(subscribe.id, &305_000_u64.to_le_bytes()),
        redraw(),
    ]);
    assert!(has_text(&frame, "05:05"), "{:?}", texts(&frame));
    // Five minutes of uptime later the label is the wall clock the host read.
    assert!(has_text(&frame, "13:45:05 UTC"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "○○●○○○○○○○"), "{:?}", texts(&frame));
    assert!(
        frame.requests.is_empty(),
        "one subscription serves forever: {:?}",
        frame.requests
    );
}
