//! A request for an undeclared capability comes back as an error the
//! handler routes like any other.

use app_store_chaos::{boot_native, tick_native};
use app_store_sdk::frame::Event;
use app_store_sdk::testing::{click, has_text, redraw, refuse, texts};

#[test]
fn the_refusal_is_an_ordinary_error() {
    boot_native();
    let frame = tick_native(vec![
        Event::Resized {
            width: 480.0,
            height: 320.0,
        },
        redraw(),
    ]);
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
