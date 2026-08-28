//! The wire between a view running in wasm and the host that draws it.
//!
//! Two messages cross the boundary. Inward, a batch of [`Event`]s — the
//! pointer, keys and the viewport, already translated into the guest's own
//! coordinates. Outward, a [`Frame`]: what the guest's iced laid out and would
//! have drawn, as flat lists the host replays with its own renderer.
//!
//! Text crosses as laid-out lines, not glyphs: the guest has already decided
//! where every line breaks and sits, the host only shapes one line at a time
//! with the same font. That keeps the host renderer-agnostic — a wgpu host
//! draws the frame as well as a tiny-skia one — at the price of one shaping
//! pass per visible line, which iced's text cache dedups across frames.
//!
//! Externally tagged enums on purpose: bincode cannot decode an internally
//! tagged one, and serde's `Content` buffering for internal tags was where a
//! JSON node tree spent 90% of its decode time.

use serde::{Deserialize, Serialize};

pub type Rect = [f32; 4];
pub type Rgba = [f32; 4];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    /// The viewport the guest lays out into, in logical pixels.
    Resized {
        width: f32,
        height: f32,
    },
    CursorMoved {
        x: f32,
        y: f32,
    },
    CursorLeft,
    ButtonPressed(Button),
    ButtonReleased(Button),
    WheelLines {
        x: f32,
        y: f32,
    },
    WheelPixels {
        x: f32,
        y: f32,
    },
    KeyPressed {
        key: Key,
        modifiers: u32,
        text: Option<String>,
    },
    KeyReleased {
        key: Key,
        modifiers: u32,
    },
    /// Runs the guest's redraw-time work (animations, caret blink).
    Redraw,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// The keys a view actually acts on. Anything else arrives as `Unidentified`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Key {
    Character(String),
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Shift,
    Control,
    Alt,
    Super,
    Unidentified,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub layers: Vec<Layer>,
    /// The guest's mouse interaction (0 = idle, 1 = pointer, 2 = text, 3 = grab).
    pub interaction: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    /// Clip bounds in guest coordinates.
    pub bounds: Rect,
    pub quads: Vec<Quad>,
    pub texts: Vec<Text>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quad {
    pub bounds: Rect,
    pub background: Rgba,
    pub border_color: Rgba,
    pub border_width: f32,
    /// top-left, top-right, bottom-right, bottom-left
    pub radius: [f32; 4],
    pub shadow_color: Rgba,
    pub shadow_offset: [f32; 2],
    pub shadow_blur: f32,
    pub snap: bool,
}

/// One laid-out line of text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub content: String,
    /// The anchor point; which corner or edge it is depends on `anchor`.
    pub x: f32,
    pub y: f32,
    /// How the host must align the shaped text to `(x, y)`.
    pub anchor: Anchor,
    pub size: f32,
    pub line_height: f32,
    pub font: Font,
    pub color: Rgba,
    pub clip: Rect,
}

/// Text laid out by the guest's paragraphs is anchored top-left at its line
/// box; text the guest's widgets drew directly (a placeholder, a caret glyph)
/// keeps the alignment they asked for.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub x: AlignX,
    pub y: AlignY,
}

impl Anchor {
    pub const TOP_LEFT: Self = Self {
        x: AlignX::Left,
        y: AlignY::Top,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AlignX {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AlignY {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Font {
    /// Family name as fontdb knows it; `None` is the host's default family.
    pub family: Option<String>,
    pub weight: u16,
    pub italic: bool,
    pub monospace: bool,
}

pub fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("wire types are plain data")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
    bincode::deserialize(bytes).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trips() {
        let frame = Frame {
            layers: vec![Layer {
                bounds: [0.0, 0.0, 10.0, 10.0],
                quads: vec![Quad {
                    bounds: [1.0, 2.0, 3.0, 4.0],
                    background: [1.0; 4],
                    border_color: [0.0; 4],
                    border_width: 1.0,
                    radius: [2.0; 4],
                    shadow_color: [0.0; 4],
                    shadow_offset: [0.0; 2],
                    shadow_blur: 0.0,
                    snap: true,
                }],
                texts: vec![Text {
                    content: "hi".into(),
                    x: 1.0,
                    y: 2.0,
                    anchor: Anchor::TOP_LEFT,
                    size: 16.0,
                    line_height: 20.0,
                    font: Font {
                        family: Some("Fira Sans".into()),
                        weight: 400,
                        italic: false,
                        monospace: false,
                    },
                    color: [0.0, 0.0, 0.0, 1.0],
                    clip: [0.0, 0.0, 10.0, 10.0],
                }],
            }],
            interaction: 1,
        };
        let back: Frame = decode(&encode(&frame)).unwrap();
        assert_eq!(back, frame);
        let events = vec![Event::KeyPressed {
            key: Key::Character("a".into()),
            modifiers: 0,
            text: Some("a".into()),
        }];
        let back: Vec<Event> = decode(&encode(&events)).unwrap();
        assert_eq!(back, events);
    }
}
