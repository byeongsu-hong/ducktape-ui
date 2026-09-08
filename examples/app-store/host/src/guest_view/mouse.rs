//! Window mouse observations translated into this guest's coordinate space.
use super::*;

pub(super) fn forward(
    guest: &mut Guest,
    event: mouse::Event,
    origin: iced::Point,
    captured: bool,
) -> bool {
    if !guest.frame.mouse_interest {
        return false;
    }
    let mut event: ui_lang_wire::mouse::Event = event.into();
    if let ui_lang_wire::mouse::Event::CursorMoved { x, y } = &mut event {
        *x -= origin.x;
        *y -= origin.y;
    }
    let Some(event) = event.sanitize() else {
        return false;
    };
    enqueue(&mut guest.pending, event, captured);
    true
}

fn enqueue(
    pending: &mut Vec<ui_lang_wire::Event>,
    event: ui_lang_wire::mouse::Event,
    captured: bool,
) {
    if matches!(event, ui_lang_wire::mouse::Event::CursorMoved { .. }) {
        // Keep the latest observation at its own position in the event sequence.
        // Discrete events retain their order; no frame delivers a second move.
        pending.retain(|event| {
            !matches!(
                event,
                ui_lang_wire::Event::Mouse {
                    event: ui_lang_wire::mouse::Event::CursorMoved { .. },
                    ..
                }
            )
        });
    }
    pending.push(ui_lang_wire::Event::Mouse { event, captured });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_lang_wire::mouse::{Button, Event as M};

    #[test]
    fn latest_move_keeps_its_position_among_discrete_events() {
        let mut pending = Vec::new();
        for event in [
            M::CursorEntered,
            M::CursorMoved { x: 1.0, y: 2.0 },
            M::ButtonPressed(Button::Other(17)),
            M::CursorMoved { x: -3.0, y: 4.0 },
            M::ButtonReleased(Button::Other(17)),
            M::CursorLeft,
        ] {
            enqueue(&mut pending, event, false);
        }
        let events: Vec<_> = pending
            .into_iter()
            .map(|event| match event {
                ui_lang_wire::Event::Mouse { event, .. } => event,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            events,
            vec![
                M::CursorEntered,
                M::ButtonPressed(Button::Other(17)),
                M::CursorMoved { x: -3.0, y: 4.0 },
                M::ButtonReleased(Button::Other(17)),
                M::CursorLeft
            ]
        );
    }
}
