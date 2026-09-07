//! A canvas drawn at an offset paints where it was laid out.
//!
//! `iced_tiny_skia 0.14.0` translated a canvas group's clip rectangle by the
//! group's own transformation twice, so geometry drawn under
//! `with_translation` was clipped to a rectangle displaced by that same
//! offset: a chart laid out at (40, 397) was only ever allowed to paint
//! inside a window starting at (80, 794). Every headless capture renders with
//! this backend, so the defect reached every `canvas` in every capture — a
//! scrolled chart drew nothing, and the terminal's candle chart lost its left
//! third. The workspace patches the crate (`vendor/iced_tiny_skia`); this
//! pins the behaviour the patch exists for, independent of any widget tree.
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::{Headless, Renderer as _};
use iced::widget::canvas;
use iced::{Color, Font, Pixels, Point, Size, Vector};

const CANVAS: f32 = 100.0;
const OFFSET: f32 = 150.0;
const SCREEN: u32 = 400;

fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 3] {
    let at = ((y * SCREEN + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2]]
}

#[test]
fn canvas_geometry_under_a_translation_paints_at_that_translation() {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");

    let mut frame = canvas::Frame::new(&renderer, Size::new(CANVAS, CANVAS));
    frame.fill_rectangle(
        Point::ORIGIN,
        Size::new(CANVAS, CANVAS),
        Color::from_rgb(1.0, 0.0, 0.0),
    );
    renderer.with_translation(Vector::new(OFFSET, OFFSET), |renderer| {
        renderer.draw_geometry(frame.into_geometry());
    });
    let rgba = renderer.screenshot(Size::new(SCREEN, SCREEN), 1.0, Color::BLACK);

    // The middle of where the canvas was laid out.
    let laid_out = (OFFSET + CANVAS / 2.0) as u32;
    assert_eq!(
        pixel(&rgba, laid_out, laid_out),
        [255, 0, 0],
        "the canvas must paint inside its own bounds"
    );
    // The middle of where a twice-translated clip would have let it paint.
    let displaced = (OFFSET * 2.0 + CANVAS / 2.0) as u32;
    assert_eq!(
        pixel(&rgba, displaced, displaced),
        [0, 0, 0],
        "nothing was laid out here"
    );
}

fn clipped_geometry(frame: &mut canvas::Frame) {
    frame.with_clip(
        iced::Rectangle::new(Point::new(10.0, 10.0), Size::new(10.0, 10.0)),
        |frame| {
            frame.fill_rectangle(Point::ORIGIN, Size::new(60.0, 60.0), Color::WHITE);
        },
    );
    // The next primitive must restore the parent mask after a clipped child.
    frame.fill_rectangle(
        Point::new(40.0, 40.0),
        Size::new(10.0, 10.0),
        Color::from_rgb(1.0, 0.0, 0.0),
    );
}

#[test]
fn clipped_child_and_following_sibling_survive_live_and_cached_geometry() {
    for cached in [false, true] {
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let cache = canvas::Cache::new();
        for pass in 0..2 {
            renderer.reset(iced::Rectangle::with_size(Size::new(
                SCREEN as f32,
                SCREEN as f32,
            )));
            let geometry = if cached {
                cache.draw(&renderer, Size::new(CANVAS, CANVAS), |frame| {
                    assert_eq!(pass, 0, "second draw must use the cached geometry");
                    clipped_geometry(frame);
                })
            } else {
                let mut frame = canvas::Frame::new(&renderer, Size::new(CANVAS, CANVAS));
                clipped_geometry(&mut frame);
                frame.into_geometry()
            };
            renderer.with_translation(Vector::new(OFFSET, OFFSET), |renderer| {
                renderer.draw_geometry(geometry);
            });
            let rgba = renderer.screenshot(Size::new(SCREEN, SCREEN), 1.0, Color::BLACK);
            assert_eq!(
                pixel(&rgba, 165, 165),
                [255; 3],
                "clip interior, cached={cached}, pass={pass}"
            );
            assert_eq!(
                pixel(&rgba, 175, 175),
                [0; 3],
                "clip exterior, cached={cached}, pass={pass}"
            );
            assert_eq!(
                pixel(&rgba, 195, 195),
                [255, 0, 0],
                "sibling mask, cached={cached}, pass={pass}"
            );
        }
    }
}
