//! A quad's shadow is repainted when the quad changes.
//!
//! A shadow is its own rectangle: it is offset from the quad and blurred
//! wider than it, so it covers pixels the quad's own bounds never reach.
//! `iced_tiny_skia 0.14.0` measures a changed quad's damage from those
//! bounds alone, so under the software renderer a card that loses its shadow
//! — or takes a different one — keeps the old shadow on screen. Every
//! headless capture and every `tiny-skia` window repaints this way, and a
//! full redraw cannot tell the two apart. The workspace patches the crate
//! (`vendor/iced_tiny_skia`); this pins the behaviour the patch exists for.
use iced::advanced::graphics::Viewport;
use iced::advanced::renderer::{Quad, Renderer as _};
use iced::{Color, Font, Pixels, Point, Rectangle, Shadow, Size, Vector};
use iced_tiny_skia::Renderer;
use iced_tiny_skia::layer::Layer;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 160;
/// The card, well above the ground its shadow falls on.
const CARD: Rectangle = Rectangle {
    x: 60.0,
    y: 20.0,
    width: 80.0,
    height: 40.0,
};
/// A pixel under the shadow and clear of the card.
const UNDER: (u32, u32) = (100, 100);

fn window() -> Rectangle {
    Rectangle::new(Point::ORIGIN, Size::new(WIDTH as f32, HEIGHT as f32))
}

/// A renderer holding the card, with `shadow` under it.
fn card(shadow: Shadow) -> Renderer {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));

    renderer.fill_quad(
        Quad {
            bounds: CARD,
            shadow,
            ..Quad::default()
        },
        Color::from_rgb(0.2, 0.2, 0.2),
    );

    renderer
}

fn cast() -> Shadow {
    Shadow {
        color: Color::BLACK,
        offset: Vector::new(0.0, 60.0),
        blur_radius: 8.0,
    }
}

#[test]
fn a_card_that_loses_its_shadow_loses_it_on_screen() {
    let mut before = card(cast());
    let mut after = card(Shadow::default());

    let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).expect("pixel map");
    let mut mask = iced_tiny_skia::ClipMask::new(WIDTH, HEIGHT).expect("clip mask");
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);

    before.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[window()],
        Color::WHITE,
    );

    let under =
        |pixmap: &tiny_skia::Pixmap| pixmap.pixels()[(UNDER.1 * WIDTH + UNDER.0) as usize].red();
    assert!(
        under(&pixmap) < 250,
        "the shadow must darken the ground to begin with"
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
        "the shadow went away, so something must be repainted"
    );

    after.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &damage,
        Color::WHITE,
    );

    assert_eq!(
        under(&pixmap),
        255,
        "the old shadow is still on screen: the damage measured the card alone"
    );
}
