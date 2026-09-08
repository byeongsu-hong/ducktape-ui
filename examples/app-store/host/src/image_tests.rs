//! The same copied raster sources through the native process and Wasm ABI.
use super::layers_tests::{Ui, build, click, container_bounds, redraw, renderer};
use super::reload::finish_reload;
use super::*;
use iced::advanced::renderer::Headless;
use iced::{Color, Rectangle, Size, mouse};
use iced_test::runtime::{UserInterface, user_interface};

fn entry(native: bool) -> CatalogEntry {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/image-native"
    } else {
        "../target/image-fixture"
    });
    let entries = crate::catalog::scan_dir(&directory);
    assert_eq!(
        entries.len(),
        1,
        "build the image fixture for both backends first"
    );
    entries.into_iter().next().unwrap()
}
fn guest(native: bool) -> Arc<Mutex<Guest>> {
    Arc::new(Mutex::new(Guest::load(&entry(native)).unwrap()))
}
fn key(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Container { key, .. } = node
        && key.ends_with(suffix)
    {
        return Some(key.clone());
    }
    node.children().iter().find_map(|node| key(node, suffix))
}
fn draw(ui: &mut Ui, renderer: &mut iced::Renderer) -> Vec<u8> {
    ui.draw(
        renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    renderer.screenshot(Size::new(600, 600), 1.0, Color::WHITE)
}
fn crop(pixels: &[u8], rect: Rectangle) -> Vec<u8> {
    let mut out = Vec::new();
    for y in rect.y.floor() as usize..(rect.y + rect.height).ceil().min(600.0) as usize {
        let x = rect.x.floor() as usize;
        let end = (rect.x + rect.width).ceil().min(600.0) as usize;
        out.extend_from_slice(&pixels[(y * 600 + x) * 4..(y * 600 + end) * 4]);
    }
    out
}
fn reference(
    rectangles: &[(&str, Rectangle)],
    changed: bool,
    renderer: &mut iced::Renderer,
) -> Vec<u8> {
    let images: Vec<iced::Element<'static, String>> = rectangles
        .iter()
        .map(|(name, rect)| {
            let handle = if *name == "rgba" {
                let (width, height) = if changed { (1, 2) } else { (2, 1) };
                iced::widget::image::Handle::from_rgba(
                    width,
                    height,
                    vec![255, 0, 0, 255, 0, 0, 255, 255],
                )
            } else {
                iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../tests/image-guest/src/ui/pair.png").as_slice(),
                )
            };
            let mut image: iced::widget::Image<iced::widget::image::Handle> =
                iced::widget::image(handle)
                    .width(rect.width)
                    .height(rect.height)
                    .content_fit(iced::ContentFit::Fill)
                    .filter_method(iced::widget::image::FilterMethod::Nearest);
            if *name == "options" {
                image = image
                    .content_fit(iced::ContentFit::Contain)
                    .opacity(0.5_f32)
                    .rotation(iced::Rotation::Solid(iced::Radians(
                        std::f32::consts::FRAC_PI_2,
                    )));
            }
            iced::widget::pin(image).x(rect.x).y(rect.y).into()
        })
        .collect();
    // Compare the whole native image scene. The renderer can paint beyond a
    // widget's nominal bounds, so isolated reference crops omit neighboring ink.
    let mut ui: Ui = UserInterface::build(
        iced::widget::stack(images),
        Size::new(600.0, 600.0),
        user_interface::Cache::default(),
        renderer,
    );
    draw(&mut ui, renderer)
}
fn assert_pictures(
    ui: &mut Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    changed: bool,
) {
    let rectangles: Vec<_> = ["rgba", "embedded", "encoded", "options"]
        .into_iter()
        .map(|name| {
            let node_key = key(
                guest.lock().unwrap().frame.root.as_ref().unwrap(),
                &format!("/{name}"),
            )
            .unwrap();
            (name, container_bounds(ui, renderer, &node_key))
        })
        .collect();
    let actual = draw(ui, renderer);
    let expected = reference(&rectangles, changed, renderer);
    for (name, rect) in rectangles {
        let actual = crop(&actual, rect);
        let expected = crop(&expected, rect);
        assert!(
            actual
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3] != [255, 255, 255]),
            "{name} must paint colored pixels"
        );
        let difference = actual.iter().zip(&expected).position(|(a, b)| a != b);
        assert_eq!(
            difference, None,
            "{name} must match native pixels after changed={changed}, bounds={rect:?}"
        );
    }
}

#[test]
#[ignore = "requires bundled and native image fixtures"]
fn bundled_images_preserve_native_pixels_shape_lazy_resync_and_instance_lifetime() {
    for native in [false, true] {
        let guest = guest(native);
        let mut renderer = renderer();
        let mut ui = build(
            &guest,
            user_interface::Cache::default(),
            &mut renderer,
            600.0,
        );
        let mut now = Instant::now();
        for _ in 0..4 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert_pictures(&mut ui, &guest, &mut renderer, false);
        if !native && let Ok(path) = std::env::var("ICE_IMAGE_CAPTURE") {
            std::fs::write(path, draw(&mut ui, &mut renderer)).unwrap();
        }
        click(&mut ui, &mut renderer, "Change shape");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert_pictures(&mut ui, &guest, &mut renderer, true);
        click(&mut ui, &mut renderer, "Toggle picture");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert!(
            key(guest.lock().unwrap().frame.root.as_ref().unwrap(), "/rgba").is_none(),
            "the native toggle must unmount the lazy picture before remounting it"
        );
        click(&mut ui, &mut renderer, "Toggle picture");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert_pictures(&mut ui, &guest, &mut renderer, true);
        guest.lock().unwrap().pending.push(wire::Event::Resync);
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert_pictures(&mut ui, &guest, &mut renderer, true);
        let entry = entry(native);
        let running = Running {
            id: entry.id.clone(),
            name: entry.name.clone(),
            surface: Surface(guest.clone()),
            window: iced::window::Id::unique(),
        };
        let prepared =
            iced::futures::executor::block_on(prepare_reload(entry, vec![running.clone()], 1));
        finish_reload(std::slice::from_ref(&running), 1, prepared)
            .expect("reload must restore image-producing state and adopt the first frame");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        assert_pictures(&mut ui, &guest, &mut renderer, true);
        drop(running);
        drop(ui);
        drop(guest);
        let replacement = self::guest(native);
        let mut ui = build(
            &replacement,
            user_interface::Cache::default(),
            &mut renderer,
            600.0,
        );
        for _ in 0..3 {
            ui = redraw(ui, &replacement, &mut renderer, &mut now, 600.0);
        }
        assert_pictures(&mut ui, &replacement, &mut renderer, false);
    }
}
