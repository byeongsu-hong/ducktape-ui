//! The host side of the boundary: a wasmtime instance behind an iced widget.
//!
//! The widget forwards every event it sees into the guest in the guest's own
//! coordinates, ticks it once per redraw, and replays the frame it gets back
//! through the host's renderer — quads as quads, text as one shaped line each.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use iced::advanced::text::{self as core_text, LineHeight, Shaping, Wrapping};
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer};
use iced::{
    Border, Color, Element, Event, Length, Pixels, Point, Rectangle, Shadow, Size, Vector, keyboard,
};
use wasm_view_frame as wire;
use wasmtime::{Config, Engine, Linker, Module, OptLevel, Store, TypedFunc};

/// Where the guest's bytes come from. Build them first:
/// `cargo build -p wasm-view-guest --release --target wasm32-unknown-unknown`.
const DEFAULT_GUEST: &str = "target/wasm32-unknown-unknown/release/wasm_view_guest.wasm";

/// The host-side handle the view holds. Identity is the instance: two
/// surfaces compare equal only when they are the same guest.
#[derive(Clone, Debug)]
pub struct Surface(Rc<RefCell<Guest>>);

impl PartialEq for Surface {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Surface {}

impl Hash for Surface {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0) as usize).hash(state);
    }
}

pub fn load_surface() -> Surface {
    let path = std::env::var("WASM_VIEW_GUEST").unwrap_or_else(|_| DEFAULT_GUEST.to_string());
    Surface(Rc::new(RefCell::new(Guest::load(&path))))
}

pub fn wasm_view(surface: &Surface) -> Element<'_, ()> {
    Element::new(WasmView {
        guest: surface.0.clone(),
    })
}

// ---------- the guest ----------

pub struct Guest {
    store: Store<()>,
    memory: wasmtime::Memory,
    input_ptr: TypedFunc<u32, u32>,
    tick: TypedFunc<u32, u32>,
    output_ptr: TypedFunc<(), u32>,
    size: Size,
    pending: Vec<wire::Event>,
    frame: wire::Frame,
}

impl std::fmt::Debug for Guest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guest")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl Guest {
    fn load(path: &str) -> Self {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        let engine = Engine::new(&config).expect("wasmtime engine");
        let module = Module::from_file(&engine, path)
            .unwrap_or_else(|error| panic!("load guest wasm from {path}: {error}"));
        let mut store = Store::new(&engine, ());
        // The guest links web_time's wasm-bindgen shims for `Instant::now`;
        // nothing on the frame path calls them, so they answer zero.
        let mut linker = Linker::new(&engine);
        linker
            .define_unknown_imports_as_default_values(&mut store, &module)
            .expect("stub guest imports");
        let instance = linker
            .instantiate(&mut store, &module)
            .expect("instantiate guest");
        let init = instance
            .get_typed_func::<(), ()>(&mut store, "init")
            .expect("init");
        let input_ptr = instance
            .get_typed_func::<u32, u32>(&mut store, "input_ptr")
            .expect("input_ptr");
        let tick = instance
            .get_typed_func::<u32, u32>(&mut store, "tick")
            .expect("tick");
        let output_ptr = instance
            .get_typed_func::<(), u32>(&mut store, "output_ptr")
            .expect("output_ptr");
        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("guest memory");
        init.call(&mut store, ()).expect("guest init");
        Self {
            store,
            memory,
            input_ptr,
            tick,
            output_ptr,
            size: Size::ZERO,
            pending: Vec::new(),
            frame: wire::Frame::default(),
        }
    }

    fn tick(&mut self) {
        let events = std::mem::take(&mut self.pending);
        let bytes = wire::encode(&events);
        let ptr = self
            .input_ptr
            .call(&mut self.store, bytes.len() as u32)
            .expect("input_ptr") as usize;
        self.memory.data_mut(&mut self.store)[ptr..ptr + bytes.len()].copy_from_slice(&bytes);
        let len = self
            .tick
            .call(&mut self.store, bytes.len() as u32)
            .expect("tick") as usize;
        let ptr = self
            .output_ptr
            .call(&mut self.store, ())
            .expect("output_ptr") as usize;
        let frame = &self.memory.data(&self.store)[ptr..ptr + len];
        self.frame = wire::decode(frame).expect("guest frame");
    }
}

// ---------- the widget ----------

struct WasmView {
    guest: Rc<RefCell<Guest>>,
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
        let mut guest = self.guest.borrow_mut();
        if guest.size != bounds.size() {
            guest.size = bounds.size();
            guest.pending.push(wire::Event::Resized {
                width: bounds.width,
                height: bounds.height,
            });
        }
        let origin = bounds.position();
        let relative = |position: Point| Point::new(position.x - origin.x, position.y - origin.y);
        match event {
            Event::Mouse(event) => {
                let translated = match event {
                    mouse::Event::CursorMoved { position } => {
                        let position = relative(*position);
                        Some(wire::Event::CursorMoved {
                            x: position.x,
                            y: position.y,
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
            Event::Window(iced::window::Event::RedrawRequested(_)) => {
                guest.pending.push(wire::Event::Redraw);
                guest.tick();
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
        let guest = self.guest.borrow();
        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
                for layer in &guest.frame.layers {
                    renderer
                        .with_layer(rect(layer.bounds), |renderer| replay_layer(renderer, layer));
                }
            });
        });
    }
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
