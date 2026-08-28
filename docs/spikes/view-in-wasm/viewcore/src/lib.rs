//! THROWAWAY SPIKE — an Ice-runtime view driven headlessly the way a view wasm
//! would drive it: build the widget tree, lay it out, record draw primitives,
//! and hand a flat encoded primitive list across the boundary.

use iced::advanced::graphics::text::Text;
use iced::advanced::renderer::Style;
use iced::{Background, Color, Font, Pixels, Size, Theme, mouse, widget};
use iced_runtime::user_interface::{self, UserInterface};
use iced_tiny_skia::layer::Item;
use serde::{Deserialize, Serialize};
use ui_lang_runtime::{
    Role, StableId, accessible, bounded_fill_element, bounded_spacing, selectable_text,
};

pub type R = iced::Renderer; // == iced_tiny_skia::Renderer with only the tiny-skia feature on
pub type El<'a> = iced::Element<'a, Msg, Theme, R>;

#[derive(Clone, Debug)]
pub enum Msg {
    Press(usize),
}

pub const SIZE: Size = Size { width: 1024.0, height: 768.0 };
const SCOPE: &str = "Bench";

fn text_el<'a>(key: String, value: String) -> El<'a> {
    let text = widget::text(value.clone());
    accessible(selectable_text(text), StableId::new(&key), Role::Label)
        .logical_id(key)
        .value(value)
        .into()
}

fn button_el<'a>(key: String, label: String, msg: Msg) -> El<'a> {
    let content: El<'a> = widget::text(label.clone()).into();
    let button = widget::button(content)
        .on_press_maybe(Some(msg.clone()))
        .style(move |theme, status| widget::button::primary(theme, status));
    accessible(button, StableId::new(&key), Role::Button)
        .logical_id(key.clone())
        .focus_id(widget::Id::from(key))
        .label(label)
        .disabled(false)
        .on_activate_maybe(Some(msg))
        .into()
}

fn linear_el<'a>(key: String, children: Vec<El<'a>>, row: bool) -> El<'a> {
    let count = children.len();
    let rendered: Vec<El<'a>> = children
        .into_iter()
        .map(|child| bounded_fill_element(child, count, row))
        .collect();
    let spacing = bounded_spacing(4.0, count);
    let layout: El<'a> = if row {
        widget::row(rendered).spacing(spacing).into()
    } else {
        widget::column(rendered).spacing(spacing).into()
    };
    accessible(widget::container(layout), StableId::new(&key), Role::GenericContainer)
        .logical_id(key)
        .into()
}

/// The same row shape the template benchmark used, plus a per-row generation
/// so a "changed" frame reshapes every row's second text.
pub fn view<'a>(n: usize, generation: u64) -> El<'a> {
    let rows = (0..n)
        .map(|i| {
            linear_el(
                format!("{SCOPE}/@layout:{i}"),
                vec![
                    text_el(format!("{SCOPE}/@text:a{i}"), "Row".to_string()),
                    text_el(format!("{SCOPE}/@text:b{i}"), format!("value {i} gen {generation}")),
                    button_el(format!("{SCOPE}/@button:{i}"), "Go".to_string(), Msg::Press(i)),
                ],
                true,
            )
        })
        .collect();
    // A real list lives in a scrollable: rows keep their natural height and
    // only the ones inside the viewport draw, as in the app.
    widget::scrollable(linear_el(format!("{SCOPE}/@layout:root"), rows, false))
        .height(iced::Length::Fill)
        .into()
}

/// A flat draw primitive — what a recording renderer would ship to the host.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Prim {
    Quad {
        bounds: [f32; 4],
        color: [f32; 4],
        radius: [f32; 4],
        border_width: f32,
        border_color: [f32; 4],
    },
    Glyph {
        glyph: u16,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
    },
    Text {
        content: String,
        bounds: [f32; 4],
        size: f32,
        color: [f32; 4],
    },
    Image {
        bounds: [f32; 4],
    },
}

fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

fn rect(r: iced::Rectangle) -> [f32; 4] {
    [r.x, r.y, r.width, r.height]
}

fn push_text(text: &Text, out: &mut Vec<Prim>) {
    match text {
        Text::Paragraph { paragraph, position, color, .. } => {
            let Some(paragraph) = paragraph.upgrade() else { return };
            for run in paragraph.buffer().layout_runs() {
                for glyph in run.glyphs {
                    out.push(Prim::Glyph {
                        glyph: glyph.glyph_id,
                        x: position.x + glyph.x,
                        y: position.y + run.line_y,
                        size: glyph.font_size,
                        color: rgba(*color),
                    });
                }
            }
        }
        Text::Cached { content, bounds, color, size, .. } => out.push(Prim::Text {
            content: content.to_string(),
            bounds: rect(*bounds),
            size: size.0,
            color: rgba(*color),
        }),
        _ => {}
    }
}

fn extract(renderer: &mut R) -> Vec<Prim> {
    let mut out = Vec::new();
    for layer in renderer.layers() {
        for (quad, background) in &layer.quads {
            let color = match background {
                Background::Color(color) => rgba(*color),
                Background::Gradient(_) => [0.0; 4],
            };
            let radius = quad.border.radius;
            out.push(Prim::Quad {
                bounds: rect(quad.bounds),
                color,
                radius: [radius.top_left, radius.top_right, radius.bottom_right, radius.bottom_left],
                border_width: quad.border.width,
                border_color: rgba(quad.border.color),
            });
        }
        for item in &layer.text {
            match item {
                Item::Live(text) => push_text(text, &mut out),
                Item::Group(texts, ..) => texts.iter().for_each(|t| push_text(t, &mut out)),
                Item::Cached(texts, ..) => texts.iter().for_each(|t| push_text(t, &mut out)),
            }
        }
        for image in &layer.images {
            out.push(Prim::Image { bounds: rect(image.bounds()) });
        }
    }
    out
}

/// One long-lived view session: the renderer (paragraph caches) and the widget
/// cache persist across frames exactly as they do in a running app.
pub struct Session {
    renderer: R,
    cache: Option<user_interface::Cache>,
    rows: usize,
    generation: u64,
}

impl Session {
    pub fn new(rows: usize) -> Self {
        Self {
            // Pinned by name: on native fontdb resolves `Font::DEFAULT` through the
            // system font list, in wasm only the embedded Fira exists.
            renderer: R::new(Font::with_name("Fira Sans"), Pixels(16.0)),
            cache: None,
            rows,
            generation: 0,
        }
    }

    /// Builds, lays out and records one frame; `changed` bumps the generation
    /// so every row's text differs from the last frame.
    pub fn frame(&mut self, changed: bool) -> Vec<Prim> {
        if changed {
            self.generation += 1;
        }
        let cache = self.cache.take().unwrap_or_default();
        let mut ui = UserInterface::build(view(self.rows, self.generation), SIZE, cache, &mut self.renderer);
        ui.draw(
            &mut self.renderer,
            &Theme::Light,
            &Style { text_color: Color::BLACK },
            mouse::Cursor::Unavailable,
        );
        let prims = extract(&mut self.renderer);
        self.cache = Some(ui.into_cache());
        prims
    }

    pub fn frame_bytes(&mut self, changed: bool) -> Vec<u8> {
        bincode::serialize(&self.frame(changed)).expect("encode")
    }

    /// Rasterizes the recorded frame into RGBA pixels at 1x — the "pixels out"
    /// option, measured for comparison only.
    pub fn raster(&mut self) -> Vec<u8> {
        let _ = self.frame(false);
        let (w, h) = (SIZE.width as u32, SIZE.height as u32);
        let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("pixmap");
        let mut mask = tiny_skia::Mask::new(w, h).expect("mask");
        let viewport = iced::advanced::graphics::Viewport::with_physical_size(Size::new(w, h), 1.0);
        self.renderer.draw(
            &mut pixmap.as_mut(),
            &mut mask,
            &viewport,
            &[iced::Rectangle::with_size(SIZE)],
            Color::WHITE,
        );
        pixmap.take()
    }
}

pub fn decode(bytes: &[u8]) -> Vec<Prim> {
    bincode::deserialize(bytes).expect("decode")
}
