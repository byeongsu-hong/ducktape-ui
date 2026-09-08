//! Real guest gradients compared to Iced's native gradient construction.
use super::image_tests::{crop, draw};
use super::layers_tests::{Ui, build, click, container_bounds, redraw, renderer};
use super::*;
use iced::{Color, Rectangle, Size, widget};
use iced_test::runtime::{UserInterface, user_interface};
fn key(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Container { key, .. } = node
        && key.ends_with(suffix)
    {
        return Some(key.clone());
    }
    node.children().iter().find_map(|node| key(node, suffix))
}
fn compare(
    ui: &mut Ui,
    guest: &Arc<Mutex<Guest>>,
    renderer: &mut iced::Renderer,
    angle: f32,
    changed: bool,
    stage: &str,
) -> Vec<u8> {
    let primary = Color::from_rgb8(0x24, 0x68, 0xc4);
    let danger = Color::from_rgb8(0xe1, 0x2d, 0x39);
    let (start, end) = if changed {
        (danger, primary)
    } else {
        (primary, danger)
    };
    let actual = draw(ui, renderer);
    let mut all = vec![];
    for (name, angle, start, end) in [
        ("hero", angle, start, end),
        ("frame", 1.57, start, Color::WHITE),
        (
            "scrim",
            1.57,
            Color::from_rgba(0.0, 0.0, 0.0, 0.1),
            Color::from_rgba(0.0, 0.0, 0.0, 0.72),
        ),
    ] {
        let key = key(
            guest.lock().unwrap().frame.root.as_ref().unwrap(),
            &format!("/{name}"),
        )
        .unwrap();
        let bounds = container_bounds(ui, renderer, &key);
        assert!(
            bounds.width >= 160.0 && bounds.height == 60.0,
            "nonempty gradient bounds"
        );
        let gradient = iced::gradient::Linear::new(angle)
            .add_stop(0.0, start)
            .add_stop(1.0, end);
        let element = widget::container(widget::space().width(iced::Fill).height(iced::Fill))
            .width(bounds.width)
            .height(bounds.height)
            .style(move |_| widget::container::Style {
                background: Some(gradient.into()),
                ..Default::default()
            });
        let mut expected: Ui = UserInterface::build(
            element,
            Size::new(600.0, 600.0),
            user_interface::Cache::default(),
            renderer,
        );
        let actual = crop(&actual, bounds);
        let reference = crop(
            &draw(&mut expected, renderer),
            Rectangle::with_size(bounds.size()),
        );
        assert_eq!(
            actual.iter().zip(&reference).position(|(a, b)| a != b),
            None,
            "native gradient pixels {stage}/{name}"
        );
        assert!(
            actual
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel != &actual[..4]),
            "gradient must contain varying colors: {stage}/{name}"
        );
        if let Ok(directory) = std::env::var("GRADIENT_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(
                directory.join(format!("{stage}-{name}-{}x60.rgba", bounds.width as u32)),
                &actual,
            )
            .unwrap();
        }
        all.extend(actual);
    }
    all
}
#[test]
#[ignore = "requires bundled and native gradient fixtures"]
fn bundled_container_gradients_match_native_angle_alpha_palette_and_resize() {
    for native in [false, true] {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
            "../target/gradient-native"
        } else {
            "../target/gradient-fixture"
        });
        let entries = crate::catalog::scan_dir(&directory);
        assert_eq!(entries.len(), 1, "build actual gradient fixtures first");
        let guest = Arc::new(Mutex::new(Guest::load(&entries[0]).unwrap()));
        let mut renderer = renderer();
        let mut ui = build(
            &guest,
            user_interface::Cache::default(),
            &mut renderer,
            160.0,
        );
        let mut now = Instant::now();
        for _ in 0..4 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 160.0);
        }
        let initial = compare(
            &mut ui,
            &guest,
            &mut renderer,
            0.0,
            false,
            &format!("native-{native}-initial"),
        );
        click(&mut ui, &mut renderer, "Rotate");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 160.0);
        }
        let rotated = compare(
            &mut ui,
            &guest,
            &mut renderer,
            1.57,
            false,
            &format!("native-{native}-rotated"),
        );
        assert!(initial != rotated, "authored angle change must repaint");
        click(&mut ui, &mut renderer, "Recolor");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 160.0);
        }
        let recolored = compare(
            &mut ui,
            &guest,
            &mut renderer,
            1.57,
            true,
            &format!("native-{native}-recolored"),
        );
        assert!(rotated != recolored, "changed palette stops must repaint");
        ui = build(&guest, ui.into_cache(), &mut renderer, 280.0);
        let resized = compare(
            &mut ui,
            &guest,
            &mut renderer,
            1.57,
            true,
            &format!("native-{native}-resized"),
        );
        assert_eq!(
            resized.len(),
            280 * 60 * 4 * 3,
            "gradient fills new container bounds"
        );
    }
}
