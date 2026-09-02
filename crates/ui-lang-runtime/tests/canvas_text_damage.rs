//! A changed canvas text damages the canvas, not the window.
//!
//! `iced_tiny_skia 0.14.0` records a canvas text with an infinite clip
//! rectangle. The damage between two frames multiplies that rectangle by the
//! layer's transformation, and a 4x4 multiply turns an infinite size into a
//! NaN one: `0.0 * f32::INFINITY`. A NaN rectangle then survives every
//! comparison it should lose — `f32::min(NaN, edge)` is `edge` — so a label
//! whose number changed asked for the whole window to be repainted. Under
//! the software renderer, which every headless capture and every
//! `tiny-skia` window uses, that is the difference between repainting a
//! chart's price column and repainting the screen. The workspace patches the
//! crate (`vendor/iced_tiny_skia`); this pins the behaviour the patch exists
//! for.
use iced::advanced::graphics::geometry::frame::Backend as _;
use iced::advanced::graphics::geometry::{Renderer as _, Text};
use iced::advanced::text::{Alignment, LineHeight, Shaping};
use iced::alignment::Vertical;
use iced::{Color, Font, Pixels, Point, Rectangle};
use iced_tiny_skia::Renderer;
use iced_tiny_skia::geometry::Frame;
use iced_tiny_skia::layer::Layer;

const CANVAS: Rectangle = Rectangle {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 120.0,
};

/// A renderer holding one canvas whose only content is `content`, drawn
/// where a chart would put a label.
fn canvas_with(content: &str) -> Renderer {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let mut frame = Frame::new(CANVAS);

    frame.fill_text(Text {
        content: content.to_owned(),
        position: Point::new(8.0, 8.0),
        max_width: f32::INFINITY,
        color: Color::BLACK,
        size: Pixels(14.0),
        line_height: LineHeight::default(),
        font: Font::DEFAULT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
    });

    renderer.draw_geometry(frame.into_geometry());
    renderer
}

#[test]
fn a_changed_canvas_text_damages_the_canvas_it_is_in() {
    let mut before = canvas_with("41.20");
    let mut after = canvas_with("41.21");

    let before = before.layers();
    let after = after.layers();
    assert_eq!(before.len(), after.len(), "one layer either way");

    let damage: Vec<Rectangle> = before
        .iter()
        .zip(after)
        .flat_map(|(before, after)| Layer::damage(before, after))
        .collect();

    assert!(
        !damage.is_empty(),
        "the text changed, so something must be repainted"
    );

    for region in &damage {
        assert!(
            region.x.is_finite()
                && region.y.is_finite()
                && region.width.is_finite()
                && region.height.is_finite(),
            "a damage region must be a rectangle, not {region:?}"
        );
        assert!(
            region.width <= CANVAS.width && region.height <= CANVAS.height,
            "{region:?} reaches past the canvas it belongs to"
        );
    }
}
