//! A partial repaint paints text inside the region it was given, and nowhere
//! else.
//!
//! The software renderer decides per glyph whether the clip mask applies: a
//! glyph inside the region is drawn without it, one outside is skipped, and
//! one crossing it goes through the mask. Every headless capture and every
//! `tiny-skia` window repaints this way, and the decision is invisible in a
//! full redraw — only a partial one can tell a glyph that stayed from a
//! glyph that leaked. The workspace patches the crate
//! (`vendor/iced_tiny_skia`); this pins the behaviour the patch exists for.
use iced::advanced::graphics::Viewport;
use iced::advanced::text::{Alignment, LineHeight, Renderer as _, Shaping, Text, Wrapping};
use iced::alignment::Vertical;
use iced::{Color, Font, Pixels, Point, Rectangle, Size};
use iced_tiny_skia::Renderer;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 80;
/// The repaint covers the left of the window only.
const REGION: f32 = 120.0;
/// What the window held before the repaint, in a colour nothing draws.
fn untouched() -> tiny_skia::PremultipliedColorU8 {
    tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 255, 255).expect("opaque magenta")
}

fn pixel(pixmap: &tiny_skia::Pixmap, x: u32, y: u32) -> tiny_skia::PremultipliedColorU8 {
    pixmap.pixels()[(y * WIDTH + x) as usize]
}

#[test]
fn a_partial_repaint_leaves_the_text_outside_its_region_alone() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));

    // One line of text across the whole window, so glyphs fall inside the
    // region, across its edge and well outside it.
    renderer.fill_text(
        Text {
            content: "MMMMMMMMMMMMMMMMMMMMMMMM".to_owned(),
            bounds: Size::new(WIDTH as f32, HEIGHT as f32),
            size: Pixels(28.0),
            line_height: LineHeight::Absolute(Pixels(34.0)),
            font: Font::DEFAULT,
            align_x: Alignment::Left,
            align_y: Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        },
        Point::new(0.0, 10.0),
        Color::BLACK,
        Rectangle::new(Point::ORIGIN, Size::new(WIDTH as f32, HEIGHT as f32)),
    );

    let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).expect("pixel map");
    pixmap.pixels_mut().fill(untouched());
    let mut mask = iced_tiny_skia::ClipMask::new(WIDTH, HEIGHT).expect("clip mask");
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);

    renderer.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[Rectangle::new(
            Point::ORIGIN,
            Size::new(REGION, HEIGHT as f32),
        )],
        Color::WHITE,
    );

    let ink = (0..REGION as u32)
        .flat_map(|x| (0..HEIGHT).map(move |y| (x, y)))
        .filter(|(x, y)| pixel(&pixmap, *x, *y).red() < 128)
        .count();
    assert!(
        ink > 200,
        "the repainted region must carry the text, found {ink} dark pixels"
    );

    for x in REGION as u32..WIDTH {
        for y in 0..HEIGHT {
            assert_eq!(
                pixel(&pixmap, x, y),
                untouched(),
                "({x}, {y}) is outside the repainted region and must be untouched"
            );
        }
    }
}
