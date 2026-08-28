//! Drives a generated application headlessly — the loop a window would run,
//! minus the window: translate the host's events, poll the app's tasks, build
//! the widget tree, lay it out, draw into iced's recording layers, and
//! flatten those into a [`wire::Frame`].
//!
//! The task executor is the simplest one that is correct here: every task is
//! polled on every tick, and re-polled while it keeps waking itself. A tick
//! happens on every host event and every redraw, and the only thing a task
//! can wait on from outside is a host response — which arrives as an event —
//! so nothing is ever ready without a tick to notice it.

use iced::advanced::graphics::text::cosmic_text;
use iced::advanced::graphics::text::{Text as GraphicsText, font_system};
use iced::advanced::renderer::Style;
use iced::{Background, Font, Pixels, Point, Rectangle, Size, Transformation, keyboard, mouse};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use iced_runtime::futures::BoxStream;
use iced_runtime::user_interface::{self, UserInterface};
use iced_runtime::{Action, task};
use iced_tiny_skia::layer::Item;

use crate::WasmApp;
use crate::frame as wire;

/// Both sides must resolve the default font to the same bytes: natively fontdb
/// walks the system font list, in wasm only the embedded family exists.
pub const DEFAULT_FONT: &str = "Fira Sans";

type Tasks<M> = Vec<BoxStream<Action<M>>>;

pub struct Driver<A: WasmApp> {
    app: A,
    cache: user_interface::Cache,
    renderer: iced::Renderer,
    size: Size,
    cursor: mouse::Cursor,
    tasks: Tasks<A::Message>,
    /// The cursor shape the last laid-out tree asked for. Remembered rather
    /// than recomputed: iced hands it back from `update`, and a tick that
    /// produced a message rebuilds the tree without one.
    interaction: u8,
}

impl<A: WasmApp> Default for Driver<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: WasmApp> Driver<A> {
    pub fn new() -> Self {
        let (app, boot) = A::boot();
        let mut tasks = Vec::new();
        spawn(&mut tasks, boot);
        Self {
            app,
            cache: user_interface::Cache::default(),
            renderer: iced::Renderer::new(Font::with_name(DEFAULT_FONT), Pixels(16.0)),
            size: Size::new(640.0, 480.0),
            cursor: mouse::Cursor::Unavailable,
            tasks,
            interaction: 0,
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
            tasks,
            interaction,
        } = self;
        // Answers that arrived with this batch may have completed a task.
        run_tasks(app, tasks);
        let mut ui = UserInterface::build(app.view(), *size, std::mem::take(cache), renderer);
        let mut messages = Vec::new();
        if !events.is_empty() {
            let (state, _) = ui.update(
                &events,
                *cursor,
                renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            if let user_interface::State::Updated {
                mouse_interaction, ..
            } = state
            {
                *interaction = wire_interaction(mouse_interaction);
            }
        }
        // A message rewrites state the tree was built from: drop the tree,
        // apply, run whatever the handlers started, rebuild.
        if !messages.is_empty() {
            *cache = ui.into_cache();
            for message in messages {
                spawn(tasks, app.update(message));
            }
            run_tasks(app, tasks);
            ui = UserInterface::build(app.view(), *size, std::mem::take(cache), renderer);
        }
        let theme = app.theme();
        ui.draw(
            renderer,
            &theme,
            &Style {
                text_color: theme.palette().text,
            },
            *cursor,
        );
        let mut frame = flatten(renderer);
        *cache = ui.into_cache();
        frame.interaction = *interaction;
        frame.requests = crate::host::drain_outbox();
        frame.cancels = crate::host::drain_cancels();
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
                // No position comes with it; the move that follows sets one.
                wire::Event::CursorEntered => {
                    out.push(iced::Event::Mouse(mouse::Event::CursorEntered));
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
                wire::Event::ModifiersChanged(modifiers) => {
                    out.push(iced::Event::Keyboard(keyboard::Event::ModifiersChanged(
                        keyboard::Modifiers::from_bits_truncate(modifiers),
                    )))
                }
                // In wasm `Instant::now()` is a stub that answers zero (the
                // web_time shims are unlinked), so host uptime added to it is
                // a monotonic clock — enough for iced's own animations.
                wire::Event::Redraw { elapsed_ms } => {
                    out.push(iced::Event::Window(iced::window::Event::RedrawRequested(
                        iced::time::Instant::now() + std::time::Duration::from_millis(elapsed_ms),
                    )))
                }
                wire::Event::Response { id, result, done } => {
                    crate::host::fulfill(id, result, done)
                }
            }
        }
        out
    }
}

fn spawn<M: iced_runtime::futures::MaybeSend + 'static>(tasks: &mut Tasks<M>, task: iced::Task<M>) {
    if let Some(stream) = task::into_stream(task) {
        tasks.push(stream);
    }
}

/// Polls every task; a message it produced goes through `update`, whose own
/// task joins the pool, until a pass produces nothing. Bounded so a handler
/// that re-emits synchronously forever cannot pin the frame.
fn run_tasks<A: WasmApp>(app: &mut A, tasks: &mut Tasks<A::Message>) {
    for _ in 0..8 {
        let messages = poll_tasks(tasks);
        if messages.is_empty() {
            return;
        }
        for message in messages {
            spawn(tasks, app.update(message));
        }
    }
}

/// Remembers that something asked to be polled again.
struct Woken(AtomicBool);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn poll_tasks<M>(tasks: &mut Tasks<M>) -> Vec<M> {
    let woken = Arc::new(Woken(AtomicBool::new(false)));
    let waker = Waker::from(woken.clone());
    let mut context = Context::from_waker(&waker);
    let mut messages = Vec::new();
    tasks.retain_mut(|stream| {
        // A task that yields (every `Task::stream` starts with one) wakes
        // itself; poll it again until it is waiting on something real.
        for _ in 0..64 {
            woken.0.store(false, Ordering::SeqCst);
            match stream.as_mut().poll_next(&mut context) {
                Poll::Ready(Some(Action::Output(message))) => messages.push(message),
                // Widget operations, clipboard, window and system actions
                // have no host to act on them yet.
                Poll::Ready(Some(_)) => {}
                Poll::Ready(None) => return false,
                Poll::Pending if woken.0.load(Ordering::SeqCst) => {}
                Poll::Pending => return true,
            }
        }
        true
    });
    messages
}

/// The shapes the host can set. Everything else is the host's own idle
/// cursor: a guest names a cursor, it does not get to install one.
fn wire_interaction(interaction: mouse::Interaction) -> u8 {
    match interaction {
        mouse::Interaction::Pointer => 1,
        mouse::Interaction::Text => 2,
        mouse::Interaction::Grab | mouse::Interaction::Grabbing => 3,
        _ => 0,
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
