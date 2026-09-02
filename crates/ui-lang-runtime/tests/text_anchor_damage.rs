//! A right- or centre-anchored text damages the pixels it actually covers.
//!
//! `iced_graphics 0.14.0` measures a text's damage from its position
//! rightwards and downwards, but the software renderer draws a right-aligned
//! text to the left of that position and a centred one around it. A changed
//! label then asks for a repaint of ground it never covered, and the glyphs
//! it did cover stay on screen: the app store's guest status line read
//! "2/1/s" long after the numbers moved. Chart axis labels are drawn this
//! way (`ui-lang-components::ui::chart`), and so is the terminal's overlay.
//! A full redraw hides it, so only a damage-driven repaint can catch it.
//! The workspace patches the crate (`vendor/iced_tiny_skia`); this pins the
//! behaviour the patch exists for.
use iced::advanced::graphics::Viewport;
use iced::advanced::text::{Alignment, LineHeight, Renderer as _, Shaping, Text, Wrapping};
use iced::alignment::Vertical;
use iced::{Color, Font, Pixels, Point, Rectangle, Size};
use iced_tiny_skia::Renderer;
use iced_tiny_skia::layer::Layer;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 80;
/// Where the label ends, the way a price column's labels end together.
const ANCHOR: Point = Point { x: 200.0, y: 40.0 };

fn window() -> Rectangle {
    Rectangle::new(Point::ORIGIN, Size::new(WIDTH as f32, HEIGHT as f32))
}

/// A renderer holding one right-aligned, vertically centred label.
fn label(content: &str) -> Renderer {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));

    renderer.fill_text(
        Text {
            content: content.to_owned(),
            bounds: Size::new(160.0, 32.0),
            size: Pixels(24.0),
            line_height: LineHeight::Absolute(Pixels(28.0)),
            font: Font::DEFAULT,
            align_x: Alignment::Right,
            align_y: Vertical::Center,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        },
        ANCHOR,
        Color::BLACK,
        window(),
    );

    renderer
}

/// The darkest pixel left of `ANCHOR`, where only the longer label reaches.
fn darkest_left_of_anchor(pixmap: &tiny_skia::Pixmap) -> u8 {
    let mut darkest = 255;

    for y in 0..HEIGHT {
        for x in 0..(ANCHOR.x as u32 - 20) {
            let pixel = pixmap.pixels()[(y * WIDTH + x) as usize];
            darkest = darkest.min(pixel.red());
        }
    }

    darkest
}

#[test]
fn a_shorter_label_erases_the_longer_one_it_replaces() {
    let mut before = label("888888");
    let mut after = label("8");

    // The window as it stood: the long label drawn over white.
    let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).expect("pixel map");
    let mut mask = tiny_skia::Mask::new(WIDTH, HEIGHT).expect("clip mask");
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);

    before.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[window()],
        Color::WHITE,
    );

    assert!(
        darkest_left_of_anchor(&pixmap) < 128,
        "the long label must reach left of the anchor to begin with"
    );

    // What the renderer says has to be repainted for the short label.
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
        "the label changed, so something must be repainted"
    );

    after.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &damage,
        Color::WHITE,
    );

    assert_eq!(
        darkest_left_of_anchor(&pixmap),
        255,
        "the long label's glyphs are still on screen: the damage missed them"
    );
}
