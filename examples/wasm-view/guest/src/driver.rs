//! Drives the generated application headlessly — the loop a window would run,
//! minus the window: translate the host's events, build the widget tree, lay
//! it out, draw into iced's recording layers, and flatten those into a
//! [`wire::Frame`].

use iced::advanced::graphics::text::{Text as GraphicsText, font_system};
use iced::advanced::renderer::Style;
use iced::{Background, Font, Pixels, Point, Rectangle, Size, Transformation, keyboard, mouse};
use iced_runtime::user_interface::{self, UserInterface};
use iced_tiny_skia::layer::Item;
use wasm_view_frame as wire;

use crate::Todo;

/// Both sides must resolve the default font to the same bytes: natively fontdb
/// walks the system font list, in wasm only the embedded family exists.
pub const DEFAULT_FONT: &str = "Fira Sans";

pub struct Driver {
    app: Todo,
    cache: user_interface::Cache,
    renderer: iced::Renderer,
    size: Size,
    cursor: mouse::Cursor,
}

impl Default for Driver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver {
    pub fn new() -> Self {
        let (app, _boot) = Todo::__boot();
        Self {
            app,
            cache: user_interface::Cache::default(),
            renderer: iced::Renderer::new(Font::with_name(DEFAULT_FONT), Pixels(16.0)),
            size: Size::new(640.0, 480.0),
            cursor: mouse::Cursor::Unavailable,
        }
    }

    /// Applies one batch of host events and returns the frame that results.
    pub fn tick(&mut self, events: Vec<wire::Event>) -> wire::Frame {
        let events = self.translate(events);
        let Self {
            app,
            cache,
            renderer,
            size,
            cursor,
        } = self;
        let mut ui = UserInterface::build(app.__view(), *size, std::mem::take(cache), renderer);
        let mut messages = Vec::new();
        if !events.is_empty() {
            let _ = ui.update(
                &events,
                *cursor,
                renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
        }
        // A message rewrites state the tree was built from; drop the tree,
        // apply, rebuild. Tasks cannot run here — the guest is synchronous.
        if !messages.is_empty() {
            *cache = ui.into_cache();
            for message in messages {
                let _task = app.__update(message);
            }
            ui = UserInterface::build(app.__view(), *size, std::mem::take(cache), renderer);
        }
        let theme = app.__theme();
        ui.draw(
            renderer,
            &theme,
            &Style {
                text_color: theme.palette().text,
            },
            *cursor,
        );
        let frame = flatten(renderer);
        *cache = ui.into_cache();
        frame
    }

    fn translate(&mut self, events: Vec<wire::Event>) -> Vec<iced::Event> {
        let mut out = Vec::with_capacity(events.len());
        for event in events {
            match event {
                wire::Event::Resized { width, height } => self.size = Size::new(width, height),
                wire::Event::CursorMoved { x, y } => {
                    let position = Point::new(x, y);
                    self.cursor = mouse::Cursor::Available(position);
                    out.push(iced::Event::Mouse(mouse::Event::CursorMoved { position }));
                }
                wire::Event::CursorLeft => {
                    self.cursor = mouse::Cursor::Unavailable;
                    out.push(iced::Event::Mouse(mouse::Event::CursorLeft));
                }
                wire::Event::ButtonPressed(button) => {
                    out.push(iced::Event::Mouse(mouse::Event::ButtonPressed(
                        mouse_button(button),
                    )));
                }
                wire::Event::ButtonReleased(button) => {
                    out.push(iced::Event::Mouse(mouse::Event::ButtonReleased(
                        mouse_button(button),
                    )));
                }
                wire::Event::WheelLines { x, y } => {
                    out.push(iced::Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Lines { x, y },
                    }))
                }
                wire::Event::WheelPixels { x, y } => {
                    out.push(iced::Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Pixels { x, y },
                    }))
                }
                wire::Event::KeyPressed {
                    key,
                    modifiers,
                    text,
                } => {
                    let key = keyboard_key(key);
                    out.push(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key: key.clone(),
                        modified_key: key,
                        physical_key: keyboard::key::Physical::Unidentified(
                            keyboard::key::NativeCode::Unidentified,
                        ),
                        location: keyboard::Location::Standard,
                        modifiers: keyboard::Modifiers::from_bits_truncate(modifiers),
                        text: text.map(|text| text.as_str().into()),
                        repeat: false,
                    }));
                }
                wire::Event::KeyReleased { key, modifiers } => {
                    let key = keyboard_key(key);
                    out.push(iced::Event::Keyboard(keyboard::Event::KeyReleased {
                        key: key.clone(),
                        modified_key: key,
                        physical_key: keyboard::key::Physical::Unidentified(
                            keyboard::key::NativeCode::Unidentified,
                        ),
                        location: keyboard::Location::Standard,
                        modifiers: keyboard::Modifiers::from_bits_truncate(modifiers),
                    }));
                }
                wire::Event::Redraw => out.push(iced::Event::Window(
                    iced::window::Event::RedrawRequested(iced::time::Instant::now()),
                )),
            }
        }
        out
    }
}

fn mouse_button(button: wire::Button) -> mouse::Button {
    match button {
        wire::Button::Left => mouse::Button::Left,
        wire::Button::Right => mouse::Button::Right,
        wire::Button::Middle => mouse::Button::Middle,
    }
}

fn keyboard_key(key: wire::Key) -> keyboard::Key {
    use keyboard::key::Named;
    let named = match key {
        wire::Key::Character(text) => return keyboard::Key::Character(text.as_str().into()),
        wire::Key::Unidentified => return keyboard::Key::Unidentified,
        wire::Key::Enter => Named::Enter,
        wire::Key::Tab => Named::Tab,
        wire::Key::Space => Named::Space,
        wire::Key::Backspace => Named::Backspace,
        wire::Key::Delete => Named::Delete,
        wire::Key::Escape => Named::Escape,
        wire::Key::ArrowUp => Named::ArrowUp,
        wire::Key::ArrowDown => Named::ArrowDown,
        wire::Key::ArrowLeft => Named::ArrowLeft,
        wire::Key::ArrowRight => Named::ArrowRight,
        wire::Key::Home => Named::Home,
        wire::Key::End => Named::End,
        wire::Key::PageUp => Named::PageUp,
        wire::Key::PageDown => Named::PageDown,
        wire::Key::Shift => Named::Shift,
        wire::Key::Control => Named::Control,
        wire::Key::Alt => Named::Alt,
        wire::Key::Super => Named::Super,
    };
    keyboard::Key::Named(named)
}

// ---------- layers → wire ----------

fn rect(r: Rectangle) -> wire::Rect {
    [r.x, r.y, r.width, r.height]
}

fn rgba(c: iced::Color) -> wire::Rgba {
    [c.r, c.g, c.b, c.a]
}

fn flatten(renderer: &mut iced::Renderer) -> wire::Frame {
    let mut frame = wire::Frame::default();
    for layer in renderer.layers() {
        let mut out = wire::Layer {
            bounds: rect(layer.bounds),
            ..Default::default()
        };
        for (quad, background) in &layer.quads {
            let background = match background {
                Background::Color(color) => rgba(*color),
                // ponytail: gradients flatten to their first stop; add a
                // gradient primitive when a view uses one.
                Background::Gradient(gradient) => match gradient {
                    iced::Gradient::Linear(linear) => linear
                        .stops
                        .iter()
                        .flatten()
                        .next()
                        .map(|stop| rgba(stop.color))
                        .unwrap_or([0.0; 4]),
                },
            };
            let radius = quad.border.radius;
            out.quads.push(wire::Quad {
                bounds: rect(quad.bounds),
                background,
                border_color: rgba(quad.border.color),
                border_width: quad.border.width,
                radius: [
                    radius.top_left,
                    radius.top_right,
                    radius.bottom_right,
                    radius.bottom_left,
                ],
                shadow_color: rgba(quad.shadow.color),
                shadow_offset: [quad.shadow.offset.x, quad.shadow.offset.y],
                shadow_blur: quad.shadow.blur_radius,
                snap: quad.snap,
            });
        }
        for item in &layer.text {
            match item {
                Item::Live(text) => push_text(text, Transformation::IDENTITY, &mut out),
                Item::Group(texts, _, transformation) => {
                    texts
                        .iter()
                        .for_each(|text| push_text(text, *transformation, &mut out));
                }
                Item::Cached(texts, _, transformation) => {
                    texts
                        .iter()
                        .for_each(|text| push_text(text, *transformation, &mut out));
                }
            }
        }
        frame.layers.push(out);
    }
    frame
}

fn push_text(text: &GraphicsText, group: Transformation, out: &mut wire::Layer) {
    match text {
        GraphicsText::Paragraph {
            paragraph,
            position,
            color,
            clip_bounds,
            transformation,
        } => {
            let Some(paragraph) = paragraph.upgrade() else {
                return;
            };
            let transformation = *transformation * group;
            push_runs(
                paragraph.buffer(),
                *position,
                *color,
                *clip_bounds,
                transformation,
                out,
            );
        }
        GraphicsText::Editor {
            editor,
            position,
            color,
            clip_bounds,
            transformation,
        } => {
            let Some(editor) = editor.upgrade() else {
                return;
            };
            let transformation = *transformation * group;
            push_runs(
                editor.buffer(),
                *position,
                *color,
                *clip_bounds,
                transformation,
                out,
            );
        }
        GraphicsText::Raw {
            raw,
            transformation,
        } => {
            let Some(buffer) = raw.buffer.upgrade() else {
                return;
            };
            let transformation = *transformation * group;
            push_runs(
                &buffer,
                raw.position,
                raw.color,
                raw.clip_bounds,
                transformation,
                out,
            );
        }
        GraphicsText::Cached {
            content,
            bounds,
            color,
            size,
            line_height,
            font,
            clip_bounds,
            align_x,
            align_y,
            ..
        } => {
            let anchor = wire::Anchor {
                x: match align_x {
                    iced::advanced::text::Alignment::Center => wire::AlignX::Center,
                    iced::advanced::text::Alignment::Right => wire::AlignX::Right,
                    _ => wire::AlignX::Left,
                },
                y: match align_y {
                    iced::alignment::Vertical::Center => wire::AlignY::Center,
                    iced::alignment::Vertical::Bottom => wire::AlignY::Bottom,
                    iced::alignment::Vertical::Top => wire::AlignY::Top,
                },
            };
            out.texts.push(wire::Text {
                content: content.to_string(),
                x: bounds.x,
                y: bounds.y,
                anchor,
                size: size.0,
                line_height: line_height.0,
                font: wire_font_from_iced(*font),
                color: rgba(*color),
                clip: rect(*clip_bounds),
            });
        }
    }
}

fn push_runs(
    buffer: &cosmic_text::Buffer,
    position: Point,
    color: iced::Color,
    clip: Rectangle,
    transformation: Transformation,
    out: &mut wire::Layer,
) {
    let scale = transformation.scale_factor();
    let origin = position * transformation;
    let mut system = font_system().write().expect("font system");
    for run in buffer.layout_runs() {
        let Some(first) = run.glyphs.first() else {
            continue;
        };
        let start = run
            .glyphs
            .iter()
            .map(|glyph| glyph.start)
            .min()
            .unwrap_or(0);
        let end = run.glyphs.iter().map(|glyph| glyph.end).max().unwrap_or(0);
        let font = system
            .raw()
            .db()
            .face(first.font_id)
            .map(|face| wire::Font {
                family: face.families.first().map(|(name, _)| name.clone()),
                weight: face.weight.0,
                italic: face.style != cosmic_text::fontdb::Style::Normal,
                monospace: face.monospaced,
            })
            .unwrap_or(wire::Font {
                family: None,
                weight: 400,
                italic: false,
                monospace: false,
            });
        out.texts.push(wire::Text {
            content: run.text[start..end].to_string(),
            x: origin.x + first.x * scale,
            y: origin.y + run.line_top * scale,
            anchor: wire::Anchor::TOP_LEFT,
            size: first.font_size * scale,
            line_height: run.line_height * scale,
            font,
            color: rgba(color),
            clip: rect(clip * transformation),
        });
    }
}

fn wire_font_from_iced(font: Font) -> wire::Font {
    wire::Font {
        family: match font.family {
            iced::font::Family::Name(name) => Some(name.to_string()),
            iced::font::Family::Monospace => Some("Fira Mono".into()),
            _ => None,
        },
        weight: weight_value(font.weight),
        italic: !matches!(font.style, iced::font::Style::Normal),
        monospace: matches!(font.family, iced::font::Family::Monospace),
    }
}

fn weight_value(weight: iced::font::Weight) -> u16 {
    use iced::font::Weight;
    match weight {
        Weight::Thin => 100,
        Weight::ExtraLight => 200,
        Weight::Light => 300,
        Weight::Normal => 400,
        Weight::Medium => 500,
        Weight::Semibold => 600,
        Weight::Bold => 700,
        Weight::ExtraBold => 800,
        Weight::Black => 900,
    }
}
