//! A cover image's drawing rectangle may exceed its widget's clip rectangle.
//! Software rendering must honor both that clip and the enclosing layer.

use iced::advanced::image::{self, Renderer as _};
use iced::advanced::renderer::{Headless, Renderer as _};
use iced::{Color, Font, Pixels, Rectangle, Size, Vector};

const SCREEN: u32 = 160;

fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 3] {
    let at = ((y * SCREEN + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2]]
}

#[test]
fn cover_images_respect_their_clip_and_the_translated_parent_layer() {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let handle = image::Handle::from_rgba(1, 1, vec![255, 255, 255, 255]);
    renderer.with_translation(Vector::new(10.0, 10.0), |renderer| {
        renderer.with_layer(
            Rectangle::new(iced::Point::new(30.0, 30.0), Size::new(50.0, 100.0)),
            |renderer| {
                renderer.draw_image(
                    image::Image::new(handle),
                    Rectangle::new(iced::Point::new(20.0, 20.0), Size::new(100.0, 100.0)),
                    Rectangle::new(iced::Point::new(20.0, 50.0), Size::new(100.0, 40.0)),
                );
            },
        );
    });
    let rgba = renderer.screenshot(Size::new(SCREEN, SCREEN), 1.0, Color::BLACK);
    assert_eq!(
        pixel(&rgba, 60, 75),
        [255; 3],
        "inside the image and parent clips"
    );
    assert_eq!(
        pixel(&rgba, 60, 45),
        [0; 3],
        "cover must not paint above its widget"
    );
    assert_eq!(
        pixel(&rgba, 60, 110),
        [0; 3],
        "cover must not paint below its widget"
    );
    assert_eq!(
        pixel(&rgba, 110, 75),
        [0; 3],
        "the image clip must not replace the parent clip"
    );
}

#[test]
fn image_radius_rounds_the_widget_clip_instead_of_the_cover_bounds() {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let mut image = image::Image::new(image::Handle::from_rgba(1, 1, vec![255; 4]));
    image.border_radius = 16.0.into();
    renderer.draw_image(
        image,
        Rectangle::new(iced::Point::new(20.0, 20.0), Size::new(120.0, 120.0)),
        Rectangle::new(iced::Point::new(20.0, 60.0), Size::new(120.0, 40.0)),
    );
    let rgba = renderer.screenshot(Size::new(SCREEN, SCREEN), 1.0, Color::BLACK);
    assert_eq!(pixel(&rgba, 80, 80), [255; 3], "the image remains visible");
    assert_eq!(
        pixel(&rgba, 21, 61),
        [0; 3],
        "round the logical widget corner"
    );
    assert_eq!(pixel(&rgba, 80, 61), [255; 3], "keep the straight top edge");
}

#[test]
fn svg_cover_respects_its_own_clip_without_clipping_the_next_image() {
    use iced::advanced::svg::{self, Renderer as _};
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let svg = svg::Svg::new(svg::Handle::from_memory(
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="white"/></svg>"#,
    ));
    renderer.draw_svg(
        svg,
        Rectangle::new(iced::Point::new(20.0, 20.0), Size::new(120.0, 120.0)),
        Rectangle::new(iced::Point::new(20.0, 60.0), Size::new(120.0, 40.0)),
    );
    renderer.draw_image(
        image::Image::new(image::Handle::from_rgba(1, 1, vec![255, 0, 0, 255])),
        Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(20.0, 20.0)),
        Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(20.0, 20.0)),
    );
    let rgba = renderer.screenshot(Size::new(SCREEN, SCREEN), 1.0, Color::BLACK);
    assert_eq!(pixel(&rgba, 80, 80), [255; 3]);
    assert_eq!(
        pixel(&rgba, 80, 30),
        [0; 3],
        "SVG cover stays inside its widget"
    );
    assert_eq!(
        pixel(&rgba, 10, 10),
        [255, 0, 0],
        "a sibling keeps its own clip"
    );
}

/// Diagnostic raster cost, including each rounded cover's scratch-mask clear.
/// Explicitly run this probe; it is not a portable wall-clock CI assertion.
#[test]
#[ignore = "native raster measurement; run explicitly with --ignored --nocapture"]
fn twelve_rounded_covers_raster_cost() {
    use iced::advanced::graphics::Viewport;
    use std::time::Instant;

    let mut renderer = iced_tiny_skia::Renderer::new(Font::DEFAULT, Pixels(16.0));
    let handle = image::Handle::from_bytes(
        include_bytes!("../../../examples/apple-music/assets/cover-01.png").as_slice(),
    );
    for row in 0..3 {
        for column in 0..4 {
            let x = 20.0 + column as f32 * 285.0;
            let y = 30.0 + row as f32 * 225.0;
            let mut cover = image::Image::new(handle.clone());
            cover.border_radius = 12.0.into();
            renderer.draw_image(
                cover,
                Rectangle::new(iced::Point::new(x, y - 70.0), Size::new(270.0, 270.0)),
                Rectangle::new(iced::Point::new(x, y), Size::new(270.0, 130.0)),
            );
        }
    }
    let physical = Size::new(1180, 760);
    let mut pixels = tiny_skia::Pixmap::new(physical.width, physical.height).unwrap();
    let mut mask = iced_tiny_skia::ClipMask::new(physical.width, physical.height).unwrap();
    let viewport = Viewport::with_physical_size(physical, 1.0);
    let region = Rectangle::with_size(Size::new(1180.0, 760.0));
    let mut micros = Vec::with_capacity(60);
    for frame in 0..68 {
        let start = Instant::now();
        renderer.draw(
            &mut pixels.as_mut(),
            &mut mask,
            &viewport,
            &[region],
            Color::WHITE,
        );
        if frame >= 8 {
            micros.push(start.elapsed().as_micros());
        }
    }
    micros.sort_unstable();
    eprintln!(
        "12 rounded covers, 1180x760 scale 1, {} profile: raster p50={}us p95={}us max={}us (8 warmup, 60 full paints)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        micros[29],
        micros[56],
        micros[59],
    );
    if let Some(directory) = std::env::var_os("ICE_TEST_ARTIFACT_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        // The software framebuffer uses BGRA; PNG stores RGBA.
        for pixel in pixels.data_mut().chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        pixels
            .save_png(directory.join("twelve-rounded-covers.png"))
            .unwrap();
    }
}
