//! A changed canvas text damages the canvas, not the window, and damages the
//! pixels its alignment puts it on.
//!
//! `iced_tiny_skia 0.14.0` records a canvas text with an infinite clip
//! rectangle. The damage between two frames multiplies that rectangle by the
//! layer's transformation, and a 4x4 multiply turns an infinite size into a
//! NaN one: `0.0 * f32::INFINITY`. A NaN rectangle then survives every
//! comparison it should lose — `f32::min(NaN, edge)` is `edge` — so a label
//! whose number changed asked for the whole window to be repainted. It also
//! measures every text rightwards and downwards from its position, whatever
//! its alignment, while a chart anchors a price at the right of its column
//! and centres it on its line. Under the software renderer, which every
//! headless capture and every `tiny-skia` window uses, that is the
//! difference between repainting a chart's price column and repainting the
//! screen — and between a price that changes and one that does not. The
//! workspace patches the crate (`vendor/iced_tiny_skia`); this pins the
//! behaviour the patch exists for.
use iced::advanced::graphics::Viewport;
use iced::advanced::graphics::geometry::frame::Backend as _;
use iced::advanced::graphics::geometry::{Renderer as _, Text};
use iced::advanced::text::{Alignment, LineHeight, Shaping};
use iced::alignment::Vertical;
use iced::{Color, Font, Pixels, Point, Rectangle, Size};
use iced_tiny_skia::Renderer;
use iced_tiny_skia::geometry::Frame;
use iced_tiny_skia::layer::Layer;

const CANVAS: Rectangle = Rectangle {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 120.0,
};
/// Where a price column ends, with room to its left for the longest price.
const ANCHOR: Point = Point { x: 120.0, y: 60.0 };

/// A renderer holding one canvas whose only content is `content`, drawn
/// where a chart would put a label.
fn canvas_with(content: &str) -> Renderer {
    canvas_aligned(content, Alignment::Left, Vertical::Top)
}

/// The same canvas, with the label anchored the way a chart anchors a price:
/// at the right of its column, centred on its line.
fn canvas_aligned(content: &str, align_x: Alignment, align_y: Vertical) -> Renderer {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let mut frame = Frame::new(CANVAS);

    frame.fill_text(Text {
        content: content.to_owned(),
        position: ANCHOR,
        max_width: f32::INFINITY,
        color: Color::BLACK,
        size: Pixels(14.0),
        line_height: LineHeight::default(),
        font: Font::DEFAULT,
        align_x,
        align_y,
        shaping: Shaping::Advanced,
    });

    renderer.draw_geometry(frame.into_geometry());
    renderer
}

/// The damage between two renderers holding one canvas each.
fn damage_between(before: &mut Renderer, after: &mut Renderer) -> Vec<Rectangle> {
    let before = before.layers();
    let after = after.layers();
    assert_eq!(before.len(), after.len(), "one layer either way");

    before
        .iter()
        .zip(after)
        .flat_map(|(before, after)| Layer::damage(before, after))
        .collect()
}

#[test]
fn a_changed_canvas_text_damages_the_canvas_it_is_in() {
    let mut before = canvas_with("41.20");
    let mut after = canvas_with("41.21");

    let damage = damage_between(&mut before, &mut after);

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

#[test]
fn a_changed_price_label_damages_its_line_and_not_the_canvas() {
    let mut before = canvas_aligned("41.20", Alignment::Right, Vertical::Center);
    let mut after = canvas_aligned("41.21", Alignment::Right, Vertical::Center);

    let damage = damage_between(&mut before, &mut after);
    assert!(
        !damage.is_empty(),
        "the label changed, so something must be repainted"
    );

    // A label with no maximum width cannot wrap, so one line of content is
    // one line of canvas: 14px of text on a default line height is 18px.
    for region in &damage {
        assert!(
            region.height <= 40.0,
            "{region:?} is taller than the line the label sits on"
        );
    }
}

#[test]
fn a_shorter_price_label_erases_the_longer_one_it_replaces() {
    // The band the damage covers has to hold the glyphs, not merely be
    // narrow: a repaint that misses them leaves the old price on screen.
    let mut before = canvas_aligned("888888", Alignment::Right, Vertical::Center);
    let mut after = canvas_aligned("8", Alignment::Right, Vertical::Center);

    let width = CANVAS.width as u32;
    let height = CANVAS.height as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("pixel map");
    let mut mask = iced_tiny_skia::ClipMask::new(width, height).expect("clip mask");
    let viewport = Viewport::with_physical_size(Size::new(width, height), 1.0);

    before.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[CANVAS],
        Color::WHITE,
    );

    // Both labels end at the anchor, so only the longer one reaches the
    // column's left half: what is dark there is the price that has moved on.
    let darkest = |pixmap: &tiny_skia::Pixmap| {
        (0..height)
            .flat_map(|y| (60..100).map(move |x| (x, y)))
            .map(|(x, y)| pixmap.pixels()[(y * width + x) as usize].red())
            .min()
            .expect("a pixel")
    };
    assert!(
        darkest(&pixmap) < 128,
        "the long label must reach the left of the column to begin with"
    );

    let damage = damage_between(&mut before, &mut after);

    after.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &damage,
        Color::WHITE,
    );

    assert_eq!(
        darkest(&pixmap),
        255,
        "the long label's glyphs are still on screen: the damage missed them"
    );
}
