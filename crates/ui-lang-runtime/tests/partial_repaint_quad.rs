//! A partial repaint fills a quad inside the region it was given, and keeps
//! the quad's corners round when the region holds one.
//!
//! The software renderer fills a quad that merely crosses the repainted
//! region over the region alone rather than over its whole area, which is
//! what makes a partial repaint cost the region and not the scene. A plain
//! rectangle may take that shortcut anywhere; a rounded one may take it only
//! where its corners are not, because there the shape is a rectangle and
//! nowhere else. A full redraw cannot tell the two apart. The workspace
//! patches the crate (`vendor/iced_tiny_skia`); this pins the behaviour the
//! patch exists for.
use iced::advanced::graphics::Viewport;
use iced::advanced::renderer::{Quad, Renderer as _};
use iced::border::Radius;
use iced::{Border, Color, Font, Pixels, Point, Rectangle, Size};
use iced_tiny_skia::Renderer;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 120;
/// The quad, inset so the window has room the quad never covers.
const QUAD: Rectangle = Rectangle {
    x: 20.0,
    y: 20.0,
    width: 160.0,
    height: 80.0,
};
const CORNER: f32 = 24.0;

/// What the window held before the repaint, in a colour nothing draws.
fn untouched() -> tiny_skia::PremultipliedColorU8 {
    tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 255, 255).expect("opaque magenta")
}

fn pixel(pixmap: &tiny_skia::Pixmap, x: u32, y: u32) -> tiny_skia::PremultipliedColorU8 {
    pixmap.pixels()[(y * pixmap.width() + x) as usize]
}

/// Repaints `region` of a window holding one rounded quad, over a buffer
/// pre-filled with a colour the scene never paints.
fn repaint(region: Rectangle) -> tiny_skia::Pixmap {
    repaint_at(region, 1.0)
}

/// The same window at `scale`, where the quad's corner radius is a physical
/// distance the renderer has to scale for itself.
fn repaint_at(region: Rectangle, scale: f32) -> tiny_skia::Pixmap {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));

    renderer.fill_quad(
        Quad {
            bounds: QUAD,
            border: Border {
                radius: Radius::new(CORNER),
                ..Border::default()
            },
            ..Quad::default()
        },
        Color::BLACK,
    );

    let physical = Size::new(
        (WIDTH as f32 * scale) as u32,
        (HEIGHT as f32 * scale) as u32,
    );
    let mut pixmap = tiny_skia::Pixmap::new(physical.width, physical.height).expect("pixel map");
    pixmap.pixels_mut().fill(untouched());
    let mut mask =
        iced_tiny_skia::ClipMask::new(physical.width, physical.height).expect("clip mask");
    let viewport = Viewport::with_physical_size(physical, scale);

    renderer.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[region],
        Color::WHITE,
    );

    pixmap
}

#[test]
fn a_partial_repaint_fills_the_quad_only_inside_its_region() {
    // Across the quad's waist, where the shape is a rectangle.
    let pixmap = repaint(Rectangle::new(
        Point::new(0.0, 50.0),
        Size::new(WIDTH as f32, 20.0),
    ));

    assert_eq!(
        pixel(&pixmap, 100, 60).red(),
        0,
        "the quad must be filled inside the repainted region"
    );
    assert_eq!(
        pixel(&pixmap, 100, 40),
        untouched(),
        "(100, 40) is above the repainted region and must be untouched"
    );
    assert_eq!(
        pixel(&pixmap, 100, 90),
        untouched(),
        "(100, 90) is below the repainted region and must be untouched"
    );
    // The fill is a pixel wider than the region on purpose, so that the mask
    // cuts the edge rather than the fill: the pixel it reaches must still be
    // the one the mask drops.
    assert_eq!(
        pixel(&pixmap, 100, 49),
        untouched(),
        "(100, 49) is one pixel above the repainted region and must be untouched"
    );
    assert_eq!(
        pixel(&pixmap, 5, 60).red(),
        255,
        "the region left of the quad carries the background, not the quad"
    );
}

#[test]
fn a_partial_repaint_that_holds_a_corner_keeps_it_round() {
    // The quad's top-left corner, where the shape is not a rectangle.
    let pixmap = repaint(Rectangle::new(Point::new(0.0, 0.0), Size::new(70.0, 60.0)));

    // The corner's arc is centred at (44, 44) with a radius of 24, so this
    // pixel lies outside it and inside the region: the repaint owns it and
    // must leave it as background.
    assert_eq!(
        pixel(&pixmap, 24, 24).red(),
        255,
        "the pixel outside the corner's arc must be background, not quad"
    );
    assert_eq!(
        pixel(&pixmap, 60, 50).red(),
        0,
        "the quad must still be filled where it is square"
    );
    assert_eq!(
        pixel(&pixmap, 150, 100),
        untouched(),
        "(150, 100) is outside the repainted region and must be untouched"
    );
}

#[test]
fn a_corner_stays_round_when_the_window_is_scaled() {
    // Twice the pixels for the same window: the corner's radius is 24
    // logical units and 48 physical ones, and the renderer decides where
    // the shape is square in physical pixels.
    //
    // The region starts to the right of a 24-pixel corner and inside a
    // 48-pixel one, so a renderer that forgot to scale the radius would
    // believe this region holds no corner and fill it square.
    let pixmap = repaint_at(
        Rectangle {
            x: 36.0,
            y: 0.0,
            width: 34.0,
            height: 60.0,
        },
        2.0,
    );

    // The corner's arc is centred at (88, 88) physical with a radius of 48.
    assert_eq!(
        pixel(&pixmap, 73, 41).red(),
        255,
        "the pixel outside the corner's arc must stay background"
    );
    assert_eq!(
        pixel(&pixmap, 100, 100).red(),
        0,
        "the quad must still be filled where it is square"
    );
}
