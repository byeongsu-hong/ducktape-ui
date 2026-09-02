//! A changed stroke is repainted over the width it was drawn with.
//!
//! A stroke paints half its width either side of the path it follows, and a
//! path that is a straight line has no width at all: the bounds of a
//! horizontal line are a rectangle of zero height. `iced_tiny_skia 0.14.0`
//! measures a changed primitive by those bounds, so under the software
//! renderer a chart's grid line, wick or crosshair asks for nothing to be
//! repainted and stays on screen after it moves. Every headless capture and
//! every `tiny-skia` window repaints this way, and a full redraw cannot tell
//! the two apart. The workspace patches the crate
//! (`vendor/iced_tiny_skia`); this pins the behaviour the patch exists for.
use iced::advanced::graphics::Viewport;
use iced::advanced::graphics::geometry::frame::Backend as _;
use iced::advanced::graphics::geometry::{Renderer as _, Stroke};
use iced::widget::canvas::Path;
use iced::{Color, Font, Pixels, Point, Rectangle, Size};
use iced_tiny_skia::Renderer;
use iced_tiny_skia::geometry::Frame;
use iced_tiny_skia::layer::Layer;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 120;
const CANVAS: Rectangle = Rectangle {
    x: 0.0,
    y: 0.0,
    width: WIDTH as f32,
    height: HEIGHT as f32,
};
/// A pixel the long line covers and the short one does not.
const BEYOND: (u32, u32) = (150, 60);

/// A canvas holding one horizontal rule that ends at `x`.
fn rule(x: f32) -> Renderer {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let mut frame = Frame::new(CANVAS);

    frame.stroke(
        &Path::line(Point::new(20.0, 60.0), Point::new(x, 60.0)),
        Stroke::default().with_color(Color::BLACK).with_width(20.0),
    );

    renderer.draw_geometry(frame.into_geometry());
    renderer
}

#[test]
fn a_rule_that_shortens_is_erased_where_it_used_to_reach() {
    let mut before = rule(180.0);
    let mut after = rule(100.0);

    let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).expect("pixel map");
    let mut mask = iced_tiny_skia::ClipMask::new(WIDTH, HEIGHT).expect("clip mask");
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);

    before.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[CANVAS],
        Color::WHITE,
    );

    let beyond =
        |pixmap: &tiny_skia::Pixmap| pixmap.pixels()[(BEYOND.1 * WIDTH + BEYOND.0) as usize].red();
    assert!(
        beyond(&pixmap) < 128,
        "the long rule must reach past the short one to begin with"
    );

    let damage: Vec<Rectangle> = {
        let before = before.layers();
        let after = after.layers();
        assert_eq!(before.len(), after.len(), "one layer either way");

        before
            .iter()
            .zip(after)
            .flat_map(|(before, after)| Layer::damage(before, after))
            .collect()
    };
    assert!(
        !damage.is_empty(),
        "the rule shortened, so something must be repainted"
    );

    after.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &damage,
        Color::WHITE,
    );

    assert_eq!(
        beyond(&pixmap),
        255,
        "the rule is still on screen where it used to reach"
    );
}
