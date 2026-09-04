//! The clock subscribes at boot, asks the wall clock once, and shows every
//! tick the host streams against both.

use app_store_clock::{boot_native, tick_native};
use ui_lang_guest::frame::{Event, Frame, Request};
use ui_lang_guest::testing::{answer, has_text, item, redraw, texts};

/// 2025-01-01T13:45:00Z.
const NOW_MS: u64 = 1_735_739_100_000;

/// The store had been up five minutes when it answered: the app must anchor
/// the wall clock to zero uptime, not to the moment it asked, or every label
/// is those five minutes fast for the life of the instance.
const UPTIME_AT_ANSWER_MS: u64 = 300_000;

/// The background of the first quad drawn, which is the app's own backdrop.
/// An `unchanged` frame carries no layers at all — the host keeps the ones it
/// already has — so `None` means the backdrop is still whatever was drawn
/// last.
fn backdrop(frame: &Frame) -> Option<[f32; 4]> {
    frame
        .layers
        .iter()
        .flat_map(|layer| layer.quads.iter())
        .map(|quad| quad.background)
        .next()
}

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
    // Two subscriptions and one question: the wall clock is asked for once,
    // never per tick.
    assert_eq!(frame.requests.len(), 3, "{:?}", frame.requests);
    request_for(&frame.requests, "host.theme");
    let subscribe = request_for(&frame.requests, "clock.ticks");
    assert_eq!(subscribe.payload, 1000_i64.to_le_bytes());
    let now = request_for(&frame.requests, "clock.now");
    assert!(now.payload.is_empty(), "{:?}", now);
    assert!(has_text(&frame, "00:00"), "{:?}", texts(&frame));

    let mut when = NOW_MS.to_le_bytes().to_vec();
    when.extend_from_slice(&UPTIME_AT_ANSWER_MS.to_le_bytes());
    let frame = tick_native(vec![answer(now.id, &when), redraw()]);
    // The first tick is a whole second away, so until it lands the uptime the
    // answer carried is the app's own: the wall clock reads what the host just
    // read, not what it read five minutes ago.
    assert!(has_text(&frame, "13:45:00 UTC"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "05:00"), "{:?}", texts(&frame));

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

/// The colour mode is a host stream like any other: one item repaints the app
/// in the host's palette.
#[test]
fn the_hosts_dark_mode_repaints_the_app() {
    boot_native();
    let light = tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        redraw(),
    ]);
    let theme = request_for(&light.requests, "host.theme");
    let lit = backdrop(&light).expect("the boot frame draws the app");
    let dark = tick_native(vec![item(theme.id, b"dark"), redraw()]);
    assert_ne!(
        backdrop(&dark).unwrap_or(lit),
        lit,
        "the app's backdrop follows the host's colour mode"
    );
}
