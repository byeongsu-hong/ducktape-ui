//! Helpers for driving an app natively in its tests: find a line of text in
//! a frame and click the widget drawn around it.

use crate::frame::{Button, Event, Frame};

/// Every line of text in the frame, in draw order.
pub fn texts(frame: &Frame) -> Vec<String> {
    frame
        .layers
        .iter()
        .flat_map(|layer| layer.texts.iter().map(|text| text.content.clone()))
        .collect()
}

pub fn has_text(frame: &Frame, content: &str) -> bool {
    texts(frame).iter().any(|text| text == content)
}

/// A point just inside the given line of text.
pub fn find(frame: &Frame, content: &str) -> (f32, f32) {
    frame
        .layers
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .find(|text| text.content == content)
        .map(|text| (text.x + 4.0, text.y + 8.0))
        .unwrap_or_else(|| panic!("no text {content:?} in {:?}", texts(frame)))
}

/// The events of one left click on the text, followed by a redraw.
pub fn click(frame: &Frame, content: &str) -> Vec<Event> {
    let (x, y) = find(frame, content);
    vec![
        Event::CursorMoved { x, y },
        Event::ButtonPressed(Button::Left),
        Event::ButtonReleased(Button::Left),
        redraw(),
    ]
}

pub fn redraw() -> Event {
    Event::Redraw { elapsed_ms: 0 }
}

/// The host's answer to a one-shot request.
pub fn answer(id: u64, payload: &[u8]) -> Event {
    Event::Response {
        id,
        result: Ok(payload.to_vec()),
        done: true,
    }
}

/// One item of a subscription.
pub fn item(id: u64, payload: &[u8]) -> Event {
    Event::Response {
        id,
        result: Ok(payload.to_vec()),
        done: false,
    }
}

/// The host's refusal of a request.
pub fn refuse(id: u64, message: &str) -> Event {
    Event::Response {
        id,
        result: Err(message.to_string()),
        done: true,
    }
}
