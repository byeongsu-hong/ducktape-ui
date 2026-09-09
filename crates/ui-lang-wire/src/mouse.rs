//! Mouse data in logical coordinates relative to the guest surface.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Button {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScrollDelta {
    Lines { x: f32, y: f32 },
    Pixels { x: f32, y: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    CursorEntered,
    CursorLeft,
    CursorMoved { x: f32, y: f32 },
    ButtonPressed(Button),
    ButtonReleased(Button),
    WheelScrolled { delta: ScrollDelta },
}

impl Event {
    /// Discards invalid numeric data without moving valid positions into bounds:
    /// a drag may legitimately continue outside the guest surface.
    pub fn sanitize(self) -> Option<Self> {
        let valid = match self {
            Self::CursorMoved { x, y }
            | Self::WheelScrolled {
                delta: ScrollDelta::Lines { x, y } | ScrollDelta::Pixels { x, y },
            } => x.is_finite() && y.is_finite(),
            _ => true,
        };
        valid.then_some(self)
    }
}

#[cfg(feature = "iced")]
impl From<Button> for iced_core::mouse::Button {
    fn from(button: Button) -> Self {
        match button {
            Button::Left => Self::Left,
            Button::Right => Self::Right,
            Button::Middle => Self::Middle,
            Button::Back => Self::Back,
            Button::Forward => Self::Forward,
            Button::Other(value) => Self::Other(value),
        }
    }
}

#[cfg(feature = "iced")]
impl From<iced_core::mouse::Button> for Button {
    fn from(button: iced_core::mouse::Button) -> Self {
        match button {
            iced_core::mouse::Button::Left => Self::Left,
            iced_core::mouse::Button::Right => Self::Right,
            iced_core::mouse::Button::Middle => Self::Middle,
            iced_core::mouse::Button::Back => Self::Back,
            iced_core::mouse::Button::Forward => Self::Forward,
            iced_core::mouse::Button::Other(value) => Self::Other(value),
        }
    }
}

#[cfg(feature = "iced")]
impl From<Event> for iced_core::mouse::Event {
    fn from(event: Event) -> Self {
        match event {
            Event::CursorEntered => Self::CursorEntered,
            Event::CursorLeft => Self::CursorLeft,
            Event::CursorMoved { x, y } => Self::CursorMoved {
                position: iced_core::Point::new(x, y),
            },
            Event::ButtonPressed(button) => Self::ButtonPressed(button.into()),
            Event::ButtonReleased(button) => Self::ButtonReleased(button.into()),
            Event::WheelScrolled { delta } => Self::WheelScrolled {
                delta: match delta {
                    ScrollDelta::Lines { x, y } => iced_core::mouse::ScrollDelta::Lines { x, y },
                    ScrollDelta::Pixels { x, y } => iced_core::mouse::ScrollDelta::Pixels { x, y },
                },
            },
        }
    }
}

#[cfg(feature = "iced")]
impl From<iced_core::mouse::Event> for Event {
    fn from(event: iced_core::mouse::Event) -> Self {
        match event {
            iced_core::mouse::Event::CursorEntered => Self::CursorEntered,
            iced_core::mouse::Event::CursorLeft => Self::CursorLeft,
            iced_core::mouse::Event::CursorMoved { position } => Self::CursorMoved {
                x: position.x,
                y: position.y,
            },
            iced_core::mouse::Event::ButtonPressed(button) => Self::ButtonPressed(button.into()),
            iced_core::mouse::Event::ButtonReleased(button) => Self::ButtonReleased(button.into()),
            iced_core::mouse::Event::WheelScrolled { delta } => Self::WheelScrolled {
                delta: match delta {
                    iced_core::mouse::ScrollDelta::Lines { x, y } => ScrollDelta::Lines { x, y },
                    iced_core::mouse::ScrollDelta::Pixels { x, y } => ScrollDelta::Pixels { x, y },
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_signed_positions_buttons_and_wheel_units() {
        for event in [
            Event::CursorEntered,
            Event::CursorLeft,
            Event::CursorMoved { x: -12.5, y: 42.25 },
            Event::ButtonPressed(Button::Other(u16::MAX)),
            Event::ButtonReleased(Button::Back),
            Event::WheelScrolled {
                delta: ScrollDelta::Lines { x: -1.5, y: 3.25 },
            },
            Event::WheelScrolled {
                delta: ScrollDelta::Pixels { x: 7.5, y: -24.25 },
            },
        ] {
            assert_eq!(event.sanitize(), Some(event));
            let encoded = crate::encode(&event);
            let decoded: Event = crate::decode(&encoded).unwrap();
            assert_eq!(decoded, event);
            #[cfg(feature = "iced")]
            assert_eq!(Event::from(iced_core::mouse::Event::from(event)), event);
        }
    }

    #[test]
    fn rejects_nonfinite_positions_and_scroll_deltas() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for event in [
                Event::CursorMoved { x: value, y: 0.0 },
                Event::CursorMoved { x: 0.0, y: value },
                Event::WheelScrolled {
                    delta: ScrollDelta::Lines { x: value, y: 0.0 },
                },
                Event::WheelScrolled {
                    delta: ScrollDelta::Pixels { x: 0.0, y: value },
                },
            ] {
                assert_eq!(event.sanitize(), None);
            }
        }
    }
}

/// Declarative native cursor, independent of window or OS handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cursor {
    None,
    Hidden,
    Idle,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    ResizingHorizontally,
    ResizingVertically,
    ResizingDiagonallyUp,
    ResizingDiagonallyDown,
    ResizingColumn,
    ResizingRow,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

#[cfg(feature = "iced")]
impl From<Cursor> for iced_core::mouse::Interaction {
    fn from(cursor: Cursor) -> Self {
        match cursor {
            Cursor::None => Self::None,
            Cursor::Hidden => Self::Hidden,
            Cursor::Idle => Self::Idle,
            Cursor::ContextMenu => Self::ContextMenu,
            Cursor::Help => Self::Help,
            Cursor::Pointer => Self::Pointer,
            Cursor::Progress => Self::Progress,
            Cursor::Wait => Self::Wait,
            Cursor::Cell => Self::Cell,
            Cursor::Crosshair => Self::Crosshair,
            Cursor::Text => Self::Text,
            Cursor::Alias => Self::Alias,
            Cursor::Copy => Self::Copy,
            Cursor::Move => Self::Move,
            Cursor::NoDrop => Self::NoDrop,
            Cursor::NotAllowed => Self::NotAllowed,
            Cursor::Grab => Self::Grab,
            Cursor::Grabbing => Self::Grabbing,
            Cursor::ResizingHorizontally => Self::ResizingHorizontally,
            Cursor::ResizingVertically => Self::ResizingVertically,
            Cursor::ResizingDiagonallyUp => Self::ResizingDiagonallyUp,
            Cursor::ResizingDiagonallyDown => Self::ResizingDiagonallyDown,
            Cursor::ResizingColumn => Self::ResizingColumn,
            Cursor::ResizingRow => Self::ResizingRow,
            Cursor::AllScroll => Self::AllScroll,
            Cursor::ZoomIn => Self::ZoomIn,
            Cursor::ZoomOut => Self::ZoomOut,
        }
    }
}
