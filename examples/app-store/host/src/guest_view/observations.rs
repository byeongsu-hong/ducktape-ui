//! Copied lifecycle/IME observations after native widget delivery.
use super::*;

pub(super) fn forward(guest: &mut Guest, event: &Event, captured: bool) -> bool {
    if guest.frame.event_interest == Default::default() {
        return false;
    }
    // The final Closed event belongs to the host's drop-window boundary, where
    // it can be drained even though no widget or redraw remains.
    if matches!(event, Event::Window(window::Event::Closed)) {
        return false;
    }
    let event = match ui_lang_wire::events::Event::from_native(event) {
        Ok(Some(event)) => event,
        Ok(None) => return false,
        Err(reason) => {
            eprintln!("guest observation refused: {reason}");
            return false;
        }
    };
    if !guest.frame.event_interest.accepts(&event) {
        return false;
    }
    guest
        .pending
        .push(ui_lang_wire::Event::Observation { event, captured });
    true
}
