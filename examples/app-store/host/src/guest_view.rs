//! The widget that shows a running guest: it forwards every event it sees
//! into the guest in the guest's own coordinates, ticks it once per redraw,
//! and replays the frame it gets back through the host's renderer — quads as
//! quads, text as one shaped line each. A status line in the corner shows
//! what the last tick cost; a guest the host had to end shows why.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use app_store_frame as wire;
use iced::advanced::text::{self as core_text, LineHeight, Shaping, Wrapping};
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer};
use iced::{
    Border, Color, Element, Event, Length, Pixels, Point, Rectangle, Shadow, Size, Vector, keyboard,
};

use crate::store::{Guest, Surface};

pub fn wasm_view(surface: &Surface) -> Element<'_, ()> {
    Element::new(WasmView {
        guest: surface.0.clone(),
    })
}

struct WasmView {
    guest: Arc<Mutex<Guest>>,
}

impl<Theme, Renderer> Widget<(), Theme, Renderer> for WasmView
where
    Renderer: core_text::Renderer<Font = iced::Font>,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, ()>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let mut guest = self.guest.lock().expect("guest lock");
        if guest.size != bounds.size() {
            guest.size = bounds.size();
            guest.pending.push(wire::Event::Resized {
                width: bounds.width,
                height: bounds.height,
            });
        }
        match event {
            Event::Mouse(event) => {
                let translated = match event {
                    // The widget may sit inside a scrollable: `cursor` is
                    // already translated into this layout's space, while the
                    // event's position is the raw window position.
                    mouse::Event::CursorMoved { position } => {
                        let position = cursor.position().unwrap_or(*position);
                        Some(wire::Event::CursorMoved {
                            x: position.x - bounds.x,
                            y: position.y - bounds.y,
                        })
                    }
                    mouse::Event::CursorLeft => Some(wire::Event::CursorLeft),
                    mouse::Event::CursorEntered => None,
                    mouse::Event::ButtonPressed(button) => {
                        wire_button(*button).map(wire::Event::ButtonPressed)
                    }
                    mouse::Event::ButtonReleased(button) => {
                        wire_button(*button).map(wire::Event::ButtonReleased)
                    }
                    mouse::Event::WheelScrolled { delta } => Some(match delta {
                        mouse::ScrollDelta::Lines { x, y } => {
                            wire::Event::WheelLines { x: *x, y: *y }
                        }
                        mouse::ScrollDelta::Pixels { x, y } => {
                            wire::Event::WheelPixels { x: *x, y: *y }
                        }
                    }),
                };
                if let Some(translated) = translated {
                    // A press that lands on the guest is the guest's.
                    if cursor.is_over(bounds) && matches!(event, mouse::Event::ButtonPressed(_)) {
                        shell.capture_event();
                    }
                    guest.pending.push(translated);
                    shell.request_redraw();
                }
            }
            Event::Keyboard(event) => {
                let translated = match event {
                    keyboard::Event::KeyPressed {
                        key,
                        modifiers,
                        text,
                        ..
                    } => wire::Event::KeyPressed {
                        key: wire_key(key),
                        modifiers: modifiers.bits(),
                        text: text.as_ref().map(|text| text.to_string()),
                    },
                    keyboard::Event::KeyReleased { key, modifiers, .. } => {
                        wire::Event::KeyReleased {
                            key: wire_key(key),
                            modifiers: modifiers.bits(),
                        }
                    }
                    keyboard::Event::ModifiersChanged(_) => return,
                };
                guest.pending.push(translated);
                shell.request_redraw();
            }
            Event::Window(iced::window::Event::RedrawRequested(now)) => {
                if let Some(at) = guest.redraw(*now) {
                    shell.request_redraw_at(at);
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let guest = self.guest.lock().expect("guest lock");
        if let Some(fault) = &guest.fault {
            renderer.with_layer(bounds, |renderer| draw_fault(renderer, bounds, fault));
            return;
        }
        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
                for layer in &guest.frame.layers {
                    renderer
                        .with_layer(rect(layer.bounds), |renderer| replay_layer(renderer, layer));
                }
            });
        });
        // Its own layer, pushed after the frame's: anything added to a parent
        // layer after a child was pushed is drawn beneath that child.
        let status = format!(
            "{:.0}k fuel · {:.2} ms",
            guest.fuel_used as f64 / 1000.0,
            guest.tick_time.as_secs_f64() * 1000.0
        );
        renderer.with_layer(bounds, |renderer| {
            small_text(
                renderer,
                status,
                Point::new(bounds.x + 12.0, bounds.y + bounds.height - 8.0),
                iced::alignment::Vertical::Bottom,
                Color::from_rgba(0.40, 0.44, 0.52, 0.9),
                bounds,
            );
        });
    }
}

/// The host ended this app. Its last frame is gone with its instance state;
/// what remains is the reason, in the app's place.
fn draw_fault<Renderer>(renderer: &mut Renderer, bounds: Rectangle, fault: &str)
where
    Renderer: core_text::Renderer<Font = iced::Font>,
{
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            ..renderer::Quad::default()
        },
        Color::from_rgba(0.79, 0.20, 0.27, 0.06),
    );
    let left = bounds.x + 24.0;
    let middle = bounds.y + bounds.height / 2.0;
    small_text(
        renderer,
        "The host ended this app.".to_string(),
        Point::new(left, middle - 12.0),
        iced::alignment::Vertical::Center,
        Color::from_rgb(0.79, 0.20, 0.27),
        bounds,
    );
    small_text(
        renderer,
        fault.to_string(),
        Point::new(left, middle + 12.0),
        iced::alignment::Vertical::Center,
        Color::from_rgba(0.40, 0.44, 0.52, 0.9),
        bounds,
    );
}

fn small_text<Renderer>(
    renderer: &mut Renderer,
    content: String,
    position: Point,
    align_y: iced::alignment::Vertical,
    color: Color,
    clip: Rectangle,
) where
    Renderer: core_text::Renderer<Font = iced::Font>,
{
    renderer.fill_text(
        core_text::Text {
            content,
            bounds: Size::new(clip.width - 48.0, 16.0),
            size: Pixels(11.0),
            line_height: LineHeight::Absolute(Pixels(16.0)),
            font: iced::Font::DEFAULT,
            align_x: core_text::Alignment::Left,
            align_y,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        },
        position,
        color,
        clip,
    );
}

fn replay_layer<Renderer>(renderer: &mut Renderer, layer: &wire::Layer)
where
    Renderer: core_text::Renderer<Font = iced::Font>,
{
    for quad in &layer.quads {
        renderer.fill_quad(
            renderer::Quad {
                bounds: rect(quad.bounds),
                border: Border {
                    color: color(quad.border_color),
                    width: quad.border_width,
                    radius: iced::border::Radius {
                        top_left: quad.radius[0],
                        top_right: quad.radius[1],
                        bottom_right: quad.radius[2],
                        bottom_left: quad.radius[3],
                    },
                },
                shadow: Shadow {
                    color: color(quad.shadow_color),
                    offset: Vector::new(quad.shadow_offset[0], quad.shadow_offset[1]),
                    blur_radius: quad.shadow_blur,
                },
                snap: quad.snap,
            },
            color(quad.background),
        );
    }
    for text in &layer.texts {
        renderer.fill_text(
            core_text::Text {
                content: text.content.clone(),
                bounds: Size::new(f32::INFINITY, text.line_height),
                size: Pixels(text.size),
                line_height: LineHeight::Absolute(Pixels(text.line_height)),
                font: font(&text.font),
                align_x: match text.anchor.x {
                    wire::AlignX::Left => core_text::Alignment::Left,
                    wire::AlignX::Center => core_text::Alignment::Center,
                    wire::AlignX::Right => core_text::Alignment::Right,
                },
                align_y: match text.anchor.y {
                    wire::AlignY::Top => iced::alignment::Vertical::Top,
                    wire::AlignY::Center => iced::alignment::Vertical::Center,
                    wire::AlignY::Bottom => iced::alignment::Vertical::Bottom,
                },
                shaping: Shaping::Advanced,
                wrapping: Wrapping::None,
            },
            Point::new(text.x, text.y),
            color(text.color),
            rect(text.clip),
        );
    }
}

fn rect(r: wire::Rect) -> Rectangle {
    Rectangle {
        x: r[0],
        y: r[1],
        width: r[2],
        height: r[3],
    }
}

fn color(c: wire::Rgba) -> Color {
    Color {
        r: c[0],
        g: c[1],
        b: c[2],
        a: c[3],
    }
}

fn wire_button(button: mouse::Button) -> Option<wire::Button> {
    match button {
        mouse::Button::Left => Some(wire::Button::Left),
        mouse::Button::Right => Some(wire::Button::Right),
        mouse::Button::Middle => Some(wire::Button::Middle),
        _ => None,
    }
}

fn wire_key(key: &keyboard::Key) -> wire::Key {
    use keyboard::key::Named;
    match key {
        keyboard::Key::Character(text) => wire::Key::Character(text.to_string()),
        keyboard::Key::Named(named) => match named {
            Named::Enter => wire::Key::Enter,
            Named::Tab => wire::Key::Tab,
            Named::Space => wire::Key::Space,
            Named::Backspace => wire::Key::Backspace,
            Named::Delete => wire::Key::Delete,
            Named::Escape => wire::Key::Escape,
            Named::ArrowUp => wire::Key::ArrowUp,
            Named::ArrowDown => wire::Key::ArrowDown,
            Named::ArrowLeft => wire::Key::ArrowLeft,
            Named::ArrowRight => wire::Key::ArrowRight,
            Named::Home => wire::Key::Home,
            Named::End => wire::Key::End,
            Named::PageUp => wire::Key::PageUp,
            Named::PageDown => wire::Key::PageDown,
            Named::Shift => wire::Key::Shift,
            Named::Control => wire::Key::Control,
            Named::Alt => wire::Key::Alt,
            Named::Super => wire::Key::Super,
            _ => wire::Key::Unidentified,
        },
        keyboard::Key::Unidentified => wire::Key::Unidentified,
    }
}

thread_local! {
    /// iced fonts name families by `&'static str`; each distinct family the
    /// guest names is interned once rather than leaked per frame.
    static FAMILIES: RefCell<HashMap<String, &'static str>> = RefCell::new(HashMap::new());
}

fn font(font: &wire::Font) -> iced::Font {
    use iced::font::{Family, Style, Weight};
    let family = match &font.family {
        Some(name) => Family::Name(FAMILIES.with(|families| {
            *families
                .borrow_mut()
                .entry(name.clone())
                .or_insert_with(|| Box::leak(name.clone().into_boxed_str()))
        })),
        None if font.monospace => Family::Monospace,
        None => Family::SansSerif,
    };
    let weight = match font.weight {
        0..=149 => Weight::Thin,
        150..=249 => Weight::ExtraLight,
        250..=349 => Weight::Light,
        350..=449 => Weight::Normal,
        450..=549 => Weight::Medium,
        550..=649 => Weight::Semibold,
        650..=749 => Weight::Bold,
        750..=849 => Weight::ExtraBold,
        _ => Weight::Black,
    };
    iced::Font {
        family,
        weight,
        style: if font.italic {
            Style::Italic
        } else {
            Style::Normal
        },
        ..iced::Font::DEFAULT
    }
}
