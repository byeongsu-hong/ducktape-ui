//! The clock subscribes at boot and shows every tick the host streams.

use app_store_clock::{boot_native, tick_native};
use app_store_sdk::frame::Event;
use app_store_sdk::testing::{has_text, item, redraw, texts};

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
    let [subscribe] = frame.requests.as_slice() else {
        panic!("one subscription at boot, got {:?}", frame.requests);
    };
    assert_eq!(subscribe.kind, "clock.ticks");
    assert_eq!(subscribe.payload, 1000_i64.to_le_bytes());
    assert!(has_text(&frame, "00:00"), "{:?}", texts(&frame));

    let frame = tick_native(vec![item(subscribe.id, &5_000_u64.to_le_bytes()), redraw()]);
    assert!(has_text(&frame, "00:05"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "1 ticks received"), "{:?}", texts(&frame));

    let frame = tick_native(vec![
        item(subscribe.id, &65_000_u64.to_le_bytes()),
        redraw(),
    ]);
    assert!(has_text(&frame, "01:05"), "{:?}", texts(&frame));
    assert!(has_text(&frame, "○○●○○○○○○○"), "{:?}", texts(&frame));
    assert!(
        frame.requests.is_empty(),
        "one subscription serves forever: {:?}",
        frame.requests
    );
}
