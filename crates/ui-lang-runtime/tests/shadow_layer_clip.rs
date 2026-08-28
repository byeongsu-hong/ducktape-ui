//! A quad's shadow paints only where its layer allows.
//!
//! `iced_tiny_skia 0.14.0` chose the clip mask from the quad's own bounds and
//! then drew the shadow — a separate, larger rectangle — with no mask at all,
//! so a quad sitting well inside its clip could paint outside it through an
//! offset or a wide blur: over a neighbouring pane, over a sidebar, over
//! whatever owned those pixels. Every headless capture and every software
//! render draws with this backend. The workspace patches the crate
//! (`vendor/iced_tiny_skia`); this pins the behaviour the patch exists for,
//! independent of any widget tree.
use iced::advanced::renderer::{Headless, Quad, Renderer as _};
use iced::{Color, Font, Pixels, Rectangle, Shadow, Size, Vector};

const SCREEN: u32 = 200;
const CLIP: f32 = 100.0;
const OFFSET: f32 = 100.0;

fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 3] {
    let at = ((y * SCREEN + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2]]
}

#[test]
fn a_shadow_cannot_paint_outside_the_layer_that_clips_its_quad() {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");

    // The quad is inside the clip; its shadow is thrown a clip's width to the
    // right, which is entirely outside.
    let clip = Rectangle {
        x: 0.0,
        y: 0.0,
        width: CLIP,
        height: SCREEN as f32,
    };
    renderer.with_layer(clip, |renderer| {
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: 20.0,
                    y: 80.0,
                    width: 40.0,
                    height: 40.0,
                },
                shadow: Shadow {
                    color: Color::from_rgb(1.0, 0.0, 0.0),
                    offset: Vector::new(OFFSET, 0.0),
                    blur_radius: 4.0,
                },
                ..Quad::default()
            },
            Color::from_rgb(0.0, 0.0, 1.0),
        );
    });
    let rgba = renderer.screenshot(Size::new(SCREEN, SCREEN), 1.0, Color::WHITE);

    // The quad itself, inside the clip.
    assert_eq!(
        pixel(&rgba, 40, 100),
        [0, 0, 255],
        "the quad must paint inside its own layer"
    );
    // The middle of where the shadow was thrown, outside the clip.
    let thrown = (20.0 + OFFSET + 20.0) as u32;
    assert_eq!(
        pixel(&rgba, thrown, 100),
        [255, 255, 255],
        "a shadow may not paint outside the layer that clips its quad"
    );
}
