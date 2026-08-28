//! What the host will not do comes back as an ordinary error the handler
//! routes: a capability the manifest never declared, and more requests than
//! one tick allows.

use app_store_chaos::chaos::FLOOD;
use app_store_chaos::{boot_native, tick_native};
use app_store_sdk::frame::{Event, Frame};
use app_store_sdk::testing::{click, has_text, redraw, refuse, texts};

fn boot() -> Frame {
    boot_native();
    tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        redraw(),
    ])
}

#[test]
fn the_refusal_is_an_ordinary_error() {
    let frame = boot();
    let frame = tick_native(click(&frame, "Use the clock"));
    let [request] = frame.requests.as_slice() else {
        panic!("one request, got {:?}", frame.requests);
    };
    assert_eq!(request.kind, "clock.sleep");
    let frame = tick_native(vec![
        refuse(
            request.id,
            "`clock.sleep` needs the `clock` capability, which chaos does not declare",
        ),
        redraw(),
    ]);
    assert!(
        has_text(
            &frame,
            "`clock.sleep` needs the `clock` capability, which chaos does not declare"
        ),
        "{:?}",
        texts(&frame)
    );
}

#[test]
fn a_flood_is_one_tick_of_a_thousand_requests() {
    let frame = boot();
    let frame = tick_native(click(&frame, "Flood"));
    assert_eq!(frame.requests.len(), FLOOD, "one tick, every ask");
    assert!(
        frame
            .requests
            .iter()
            .all(|request| request.kind == "host.echo"),
        "{:?}",
        frame.requests
    );

    // Only the last one is waited for, and past the host's cap that is what
    // it hears back.
    let last = frame.requests.last().expect("the thousandth");
    let frame = tick_native(vec![
        refuse(last.id, "too many requests this tick"),
        redraw(),
    ]);
    assert!(
        has_text(&frame, "too many requests this tick"),
        "{:?}",
        texts(&frame)
    );
}
