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
    CursorEntered,
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
    /// The host's modifier state, so a guest that missed a key release does
    /// not keep thinking Shift is down.
    ModifiersChanged(u32),
    /// Runs the guest's redraw-time work (animations, caret blink).
    /// `elapsed_ms` is the host's uptime: the guest has no clock of its own.
    Redraw {
        elapsed_ms: u64,
    },
    /// One answer to a [`Request`]. A one-shot request gets exactly one with
    /// `done`; a subscription gets many, the last one `done`.
    Response {
        id: u64,
        result: Result<Vec<u8>, String>,
        done: bool,
    },
}

/// Something the guest asked the host for. The guest never blocks on it: a
/// future (or stream) inside the guest waits for the matching
/// [`Event::Response`]s, which the host delivers on its own schedule — the
/// next frame for an echo, a second later for a timer, whenever another app
/// publishes for a bus subscription.
///
/// `kind` is `<capability>.<operation>`; the host refuses a capability the
/// app's manifest did not declare.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub kind: String,
    pub payload: Vec<u8>,
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
    /// What the guest asked for while producing this frame.
    pub requests: Vec<Request>,
    /// Requests the guest stopped waiting on — a dropped future or stream.
    /// The host frees whatever it kept for them and sends no more answers.
    pub cancels: Vec<u64>,
    /// When the guest's own widgets want to draw again — a caret blink, a
    /// hover transition — independently of anything the host delivers. A
    /// guest with nothing pending is ticked only when this says so.
    pub redraw: Redraw,
    /// `layers` is empty because nothing drawn changed since the last frame:
    /// the host keeps the layers it already has instead of decoding them
    /// again. Requests, cancels and the redraw request still cross.
    pub unchanged: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Redraw {
    /// Nothing until the host has something to deliver.
    #[default]
    Wait,
    /// As soon as the host draws next.
    NextFrame,
    /// At this host uptime, in milliseconds.
    At(u64),
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

// ---------- what the host may draw ----------

/// How far outside its window a guest's drawing may reach, as a multiple of
/// the window's longest side. A scrolled row or a shadow legitimately hangs
/// off the window; a quad a thousand windows wide is a buffer the host would
/// have to allocate for nothing anyone can see.
const REACH: f32 = 2.0;

/// A shadow costs a buffer the size of the quad plus this on every side.
const MAX_BLUR: f32 = 64.0;

/// Text is rasterised glyph by glyph at this size and kept in the host's
/// glyph cache; zero is an assertion inside cosmic-text.
const MAX_TEXT: f32 = 128.0;

/// No display is this wide, and every geometry cap is a multiple of it.
const MAX_WINDOW: f32 = 8192.0;

/// What one frame may cost the host to *draw*, which the byte cap does not
/// bound: 8 MiB holds about 90 000 quads, and filling that many window-sized
/// ones is minutes of pixel writes per redraw.
const MAX_QUADS: usize = 16_384;

/// Every line is shaped by cosmic-text on the host's own thread.
const MAX_TEXTS: usize = 4_096;

/// One 8 MB line is 8 M glyphs to shape; a window holds a few thousand
/// characters. Counted over the frame, so neither one long line nor many
/// medium ones gets through.
const MAX_TEXT_BYTES: usize = 64 << 10;

/// A shadow is an SDF buffer the size of the quad plus its blur on every
/// side, built in full before anything is clipped — a megapixel of it is tens
/// of milliseconds of the window thread. Spent over the frame: a quad past
/// the budget still draws, without its shadow.
const MAX_SHADOW_PIXELS: f32 = 1024.0 * 1024.0;

/// A layer is a clip, and the host rebuilds a window-sized mask for every one
/// of them on every redraw. A real iced frame has a layer per overlay and
/// per scrollable — a few dozen at most.
const MAX_LAYERS: usize = 256;

/// Counting quads is not the same as bounding what they fill: 16 000 quads
/// the size of the window are billions of pixel writes per redraw. Spent over
/// the frame in draw order, charged for the part that lands inside the window
/// — a quad past the budget is dropped.
const MAX_FILL_PIXELS: f32 = 8.0 * 1024.0 * 1024.0;

/// Nor is counting text bytes the same as bounding rasterisation: a glyph is
/// rasterised and cached at its size, so 64 KiB of distinct 128 px text is
/// gigabytes of glyph cache. Charged as characters times size squared.
const MAX_GLYPH_PIXELS: f32 = 4.0 * 1024.0 * 1024.0;

/// Pulls every number in a frame into the range the host's renderer survives,
/// and every count into what it can afford to draw.
///
/// The host draws values the guest chose: tiny-skia `expect`s colours in
/// `0..=1` and finite rectangles, panics on a zero-sized pixmap, cosmic-text
/// asserts a non-zero font size, and both allocate by the sizes they are
/// given — the shadow pass builds a buffer the size of the quad. Clamped
/// rather than refused, because "not finite" is not the same as "hostile":
/// the base layer's clip is `Rectangle::INFINITE` in every iced frame.
pub fn sanitize(frame: &mut Frame, window: [f32; 2]) {
    // The window is the host's own, never the guest's, but it arrives as
    // plain floats: clamp it too rather than trust it.
    let width = bounded(window[0], MAX_WINDOW).max(0.0);
    let height = bounded(window[1], MAX_WINDOW).max(0.0);
    let reach = width.max(height).clamp(1.0, MAX_WINDOW) * REACH;
    frame.layers.truncate(MAX_LAYERS);
    let mut quads_left = MAX_QUADS;
    let mut texts_left = MAX_TEXTS;
    let mut text_bytes_left = MAX_TEXT_BYTES;
    let mut shadow_left = MAX_SHADOW_PIXELS;
    let mut fill_left = MAX_FILL_PIXELS;
    let mut glyph_left = MAX_GLYPH_PIXELS;
    for layer in &mut frame.layers {
        // A layer IS a clip, and iced's `push_clip` sets the new clip rather
        // than intersecting it with the one it nests in: the guest's own
        // window is the only thing that keeps its drawing off the store's
        // sidebar and off its neighbours' windows.
        clip_to_window(&mut layer.bounds, width, height);
        layer.quads.truncate(quads_left);
        quads_left -= layer.quads.len();
        layer.texts.truncate(texts_left);
        texts_left -= layer.texts.len();
        layer.quads.retain_mut(|quad| {
            clamp_rect(&mut quad.bounds, reach);
            clamp_rgba(&mut quad.background);
            clamp_rgba(&mut quad.border_color);
            clamp_rgba(&mut quad.shadow_color);
            quad.border_width = bounded(quad.border_width, reach).max(0.0);
            // A border on a sub-pixel quad is the one shape tiny-skia draws
            // through a pixmap of the quad's size, and a zero-sized pixmap is
            // an `unwrap` on `None`.
            if quad.bounds[2] < 1.0 || quad.bounds[3] < 1.0 {
                quad.border_width = 0.0;
            }
            for radius in &mut quad.radius {
                *radius = bounded(*radius, reach).max(0.0);
            }
            for offset in &mut quad.shadow_offset {
                *offset = bounded(*offset, reach);
            }
            quad.shadow_blur = bounded(quad.shadow_blur, MAX_BLUR).max(0.0);
            // Only what lands in the window costs anything to fill; the rest
            // is clipped away before a pixel is written.
            let fill = visible_area(quad.bounds, width, height);
            if fill > fill_left {
                return false;
            }
            fill_left -= fill;
            // What the shadow pass would allocate for this one quad.
            let pixels = (quad.bounds[2] + 2.0 * quad.shadow_blur)
                * (quad.bounds[3] + 2.0 * quad.shadow_blur);
            if quad.shadow_color[3] > 0.0 {
                match pixels <= shadow_left {
                    true => shadow_left -= pixels,
                    // Transparent is how the renderer skips the pass.
                    false => quad.shadow_color = [0.0; 4],
                }
            }
            true
        });
        layer.texts.retain_mut(|text| {
            text.x = bounded(text.x, reach);
            text.y = bounded(text.y, reach);
            clamp_rect(&mut text.clip, reach);
            clamp_rgba(&mut text.color);
            text.size = text_size(text.size);
            text.line_height = text_size(text.line_height);
            let glyphs = text.content.chars().count() as f32 * text.size * text.size;
            let affordable = text.content.len() <= text_bytes_left && glyphs <= glyph_left;
            if affordable {
                text_bytes_left -= text.content.len();
                glyph_left -= glyphs;
            }
            affordable
        });
    }
}

/// Cuts a clip down to the window it belongs to.
fn clip_to_window(rect: &mut Rect, width: f32, height: f32) {
    let left = bounded(rect[0], MAX_WINDOW).clamp(0.0, width);
    let top = bounded(rect[1], MAX_WINDOW).clamp(0.0, height);
    let right = bounded(rect[0] + rect[2], MAX_WINDOW).clamp(left, width);
    let bottom = bounded(rect[1] + rect[3], MAX_WINDOW).clamp(top, height);
    *rect = [left, top, right - left, bottom - top];
}

/// How much of a rectangle the host would actually paint.
fn visible_area(rect: Rect, width: f32, height: f32) -> f32 {
    let mut visible = rect;
    clip_to_window(&mut visible, width, height);
    visible[2] * visible[3]
}

fn clamp_rect(rect: &mut Rect, reach: f32) {
    let left = bounded(rect[0], reach);
    let top = bounded(rect[1], reach);
    let right = bounded(rect[0] + rect[2], reach).max(left);
    let bottom = bounded(rect[1] + rect[3], reach).max(top);
    *rect = [left, top, right - left, bottom - top];
}

fn clamp_rgba(color: &mut Rgba) {
    for channel in color {
        *channel = bounded(*channel, 1.0).max(0.0);
    }
}

fn bounded(value: f32, limit: f32) -> f32 {
    match value.is_nan() {
        true => 0.0,
        false => value.clamp(-limit, limit),
    }
}

fn text_size(value: f32) -> f32 {
    match value.is_nan() {
        true => 1.0,
        false => value.clamp(1.0, MAX_TEXT),
    }
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
    fn events_round_trip() {
        let events = vec![
            Event::Redraw { elapsed_ms: 1234 },
            Event::Response {
                id: 3,
                result: Ok(vec![1, 2]),
                done: false,
            },
            Event::Response {
                id: 4,
                result: Err("nope".into()),
                done: true,
            },
        ];
        assert_eq!(decode::<Vec<Event>>(&encode(&events)).unwrap(), events);
    }

    /// One quad and one text of everything the renderer would panic or
    /// allocate the host to death on: colours past 1, an infinite width, a
    /// sub-pixel bordered quad (a zero-sized pixmap inside tiny-skia), a
    /// 100000-pixel shadow, a zero font size (an assertion inside
    /// cosmic-text) and NaN.
    #[test]
    fn a_hostile_frame_is_pulled_into_range() {
        let mut frame = Frame {
            layers: vec![Layer {
                bounds: [
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    f32::INFINITY,
                ],
                quads: vec![
                    Quad {
                        bounds: [0.0, 0.0, f32::INFINITY, 100_000.0],
                        background: [1.5, -0.5, f32::NAN, 1.0],
                        border_color: [0.0; 4],
                        border_width: 0.0,
                        radius: [f32::NAN; 4],
                        shadow_color: [0.0, 0.0, 0.0, 1.0],
                        shadow_offset: [f32::INFINITY; 2],
                        shadow_blur: 100_000.0,
                        snap: false,
                    },
                    Quad {
                        bounds: [0.0, 0.0, 0.5, 0.5],
                        background: [0.0; 4],
                        border_color: [0.0; 4],
                        border_width: 1.0,
                        radius: [0.1; 4],
                        shadow_color: [0.0; 4],
                        shadow_offset: [0.0; 2],
                        shadow_blur: 0.0,
                        snap: false,
                    },
                ],
                texts: vec![Text {
                    content: "hi".into(),
                    x: f32::NAN,
                    y: 1e30,
                    anchor: Anchor::TOP_LEFT,
                    size: 0.0,
                    line_height: 20_000.0,
                    font: Font {
                        family: None,
                        weight: 400,
                        italic: false,
                        monospace: false,
                    },
                    color: [2.0, 0.5, 0.5, f32::NAN],
                    clip: [0.0, 0.0, f32::INFINITY, f32::INFINITY],
                }],
            }],
            ..Frame::default()
        };
        sanitize(&mut frame, [500.0, 380.0]);

        let reach = 1000.0;
        let layer = &frame.layers[0];
        for value in numbers(layer) {
            assert!(value.is_finite(), "{value} left in {layer:?}");
            assert!(value.abs() <= 2.0 * reach, "{value} left in {layer:?}");
        }
        let wide = &layer.quads[0];
        assert_eq!(wide.bounds, [0.0, 0.0, reach, reach]);
        assert!(wide.background.iter().all(|c| (0.0..=1.0).contains(c)));
        assert_eq!(wide.shadow_blur, MAX_BLUR);
        // A sub-pixel quad is drawn without its border: the pixmap tiny-skia
        // would build for it has no pixels.
        assert_eq!(layer.quads[1].border_width, 0.0);
        let text = &layer.texts[0];
        assert_eq!(text.size, 1.0);
        assert_eq!(text.line_height, MAX_TEXT);
        assert!(text.color.iter().all(|c| (0.0..=1.0).contains(c)));
    }

    /// The byte cap bounds what the host copies, not what drawing it costs:
    /// quads and lines are counted, long lines are dropped, and the shadow
    /// budget is spent in order — the first quads keep their shadow, the rest
    /// keep only their shape.
    #[test]
    fn a_frame_past_what_the_host_can_draw_is_cut_down() {
        let window = [500.0, 380.0];
        let quad = |width: f32, height: f32| Quad {
            bounds: [0.0, 0.0, width, height],
            shadow_color: [0.0, 0.0, 0.0, 1.0],
            shadow_blur: 0.0,
            ..ordinary_frame().layers[0].quads[0].clone()
        };
        let line = |content: &str, size: f32| Text {
            content: content.to_string(),
            size,
            ..ordinary_frame().layers[0].texts[0].clone()
        };
        let layer = |quads: Vec<Quad>, texts: Vec<Text>| Layer {
            bounds: [0.0, 0.0, window[0], window[1]],
            quads,
            texts,
        };

        // A layer is a clip: a guest cannot name one outside its own window,
        // and cannot hand the host more of them than a real frame has.
        let mut frame = Frame {
            layers: vec![layer(Vec::new(), Vec::new()); MAX_LAYERS + 8],
            ..Frame::default()
        };
        frame.layers[0].bounds = [-1000.0, -40.0, 3000.0, 3000.0];
        sanitize(&mut frame, window);
        assert_eq!(frame.layers.len(), MAX_LAYERS);
        assert_eq!(frame.layers[0].bounds, [0.0, 0.0, window[0], window[1]]);

        // Quads are bounded by what they fill, not only by their count, and
        // the shadow budget is spent separately.
        let mut frame = Frame {
            layers: vec![
                layer(
                    vec![quad(1000.0, 1000.0); MAX_QUADS],
                    vec![line("hi", 16.0); MAX_TEXTS],
                ),
                layer(vec![quad(1000.0, 1000.0)], vec![line("still here", 16.0)]),
            ],
            ..Frame::default()
        };
        sanitize(&mut frame, window);
        // The second layer is past both counts, so nothing of it is left.
        assert_eq!(frame.layers[1].quads.len(), 0);
        assert_eq!(frame.layers[1].texts.len(), 0);
        let per_quad = window[0] * window[1];
        assert_eq!(
            frame.layers[0].quads.len(),
            (MAX_FILL_PIXELS / per_quad) as usize,
            "a window-sized quad past the fill budget is dropped"
        );
        let with_shadow = frame.layers[0]
            .quads
            .iter()
            .filter(|quad| quad.shadow_color[3] > 0.0)
            .count();
        assert_eq!(
            with_shadow,
            (MAX_SHADOW_PIXELS / (1000.0 * 1000.0)) as usize
        );

        // Text is bounded by what it rasterises: characters times size
        // squared, spent over the frame.
        let big = "a".repeat(100);
        let mut frame = Frame {
            layers: vec![layer(Vec::new(), vec![line(&big, MAX_TEXT); 3])],
            ..Frame::default()
        };
        sanitize(&mut frame, window);
        let per_line = 100.0 * MAX_TEXT * MAX_TEXT;
        assert_eq!(
            frame.layers[0].texts.len(),
            (MAX_GLYPH_PIXELS / per_line) as usize
        );

        // One line longer than the whole frame's byte budget is dropped, and
        // the lines before it are not.
        let mut frame = Frame {
            layers: vec![layer(
                Vec::new(),
                vec![
                    line("hi", 16.0),
                    line(&"a".repeat(MAX_TEXT_BYTES + 1), 16.0),
                    line("!", 16.0),
                ],
            )],
            ..Frame::default()
        };
        sanitize(&mut frame, window);
        let kept: Vec<&str> = frame.layers[0]
            .texts
            .iter()
            .map(|text| text.content.as_str())
            .collect();
        assert_eq!(kept, ["hi", "!"]);
    }

    /// A frame an ordinary app draws inside its window is not touched — the
    /// one thing every iced frame carries is the base layer's infinite clip.
    #[test]
    fn an_ordinary_frame_survives_untouched() {
        let mut frame = ordinary_frame();
        let before = frame.clone();
        sanitize(&mut frame, [500.0, 380.0]);
        assert_eq!(frame.layers[0].quads, before.layers[0].quads);
        assert_eq!(frame.layers[0].texts, before.layers[0].texts);
    }

    fn numbers(layer: &Layer) -> Vec<f32> {
        let mut values: Vec<f32> = layer.bounds.to_vec();
        for quad in &layer.quads {
            values.extend(quad.bounds);
            values.extend(quad.background);
            values.extend(quad.border_color);
            values.extend(quad.radius);
            values.extend(quad.shadow_color);
            values.extend(quad.shadow_offset);
            values.push(quad.border_width);
            values.push(quad.shadow_blur);
        }
        for text in &layer.texts {
            values.extend([text.x, text.y, text.size, text.line_height]);
            values.extend(text.color);
            values.extend(text.clip);
        }
        values
    }

    /// One layer of one quad and one line, all inside a 500x380 window.
    fn ordinary_frame() -> Frame {
        Frame {
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
            requests: vec![Request {
                id: 7,
                kind: "host.echo".into(),
                payload: b"hi".to_vec(),
            }],
            cancels: vec![9],
            redraw: Redraw::At(1500),
            unchanged: false,
        }
    }

    #[test]
    fn frame_round_trips() {
        let frame = ordinary_frame();
        let back: Frame = decode(&encode(&frame)).unwrap();
        assert_eq!(back, frame);
        let events = vec![
            Event::KeyPressed {
                key: Key::Character("a".into()),
                modifiers: 0,
                text: Some("a".into()),
            },
            Event::ModifiersChanged(4),
            Event::CursorEntered,
        ];
        let back: Vec<Event> = decode(&encode(&events)).unwrap();
        assert_eq!(back, events);
    }
}
