//! Real native child and Wasm viewers against the same Iced widget's pixels.
use super::image_tests::{crop, draw};
use super::layers_tests::{Ui, build, click, container_bounds, redraw, renderer};
use super::*;
use iced::{Event, Point, Rectangle, Size, mouse};
use iced_test::runtime::{UserInterface, user_interface};

fn guest(native: bool) -> Arc<Mutex<Guest>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/viewer-native"
    } else {
        "../target/viewer-fixture"
    });
    let entries = crate::catalog::scan_dir(&directory);
    assert_eq!(entries.len(), 1, "bundle the current viewer fixture first");
    Arc::new(Mutex::new(Guest::load(&entries[0]).unwrap()))
}
fn viewports(guest: &Arc<Mutex<Guest>>, ui: &mut Ui, renderer: &iced::Renderer) -> Vec<Rectangle> {
    fn keys(node: &wire::Node, out: &mut Vec<String>) {
        if let wire::Node::Container { key, .. } = node
            && key.ends_with("/viewport")
        {
            out.push(key.clone());
        }
        for child in node.children() {
            keys(child, out);
        }
    }
    let mut ids = Vec::new();
    keys(guest.lock().unwrap().frame.root.as_ref().unwrap(), &mut ids);
    assert_eq!(ids.len(), 2);
    ids.into_iter()
        .map(|key| container_bounds(ui, renderer, &key))
        .collect()
}
fn send(ui: &mut Ui, renderer: &mut iced::Renderer, event: Event, at: Point) {
    ui.update(
        &[event],
        mouse::Cursor::Available(at),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn wheel(ui: &mut Ui, renderer: &mut iced::Renderer, at: Point, y: f32) {
    send(
        ui,
        renderer,
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y },
        }),
        at,
    );
}
fn pan(ui: &mut Ui, renderer: &mut iced::Renderer, from: Point, to: Point) {
    send(
        ui,
        renderer,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        from,
    );
    send(
        ui,
        renderer,
        Event::Mouse(mouse::Event::CursorMoved { position: to }),
        to,
    );
    send(
        ui,
        renderer,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        to,
    );
}
fn reference(renderer: &mut iced::Renderer) -> Ui {
    let colors = [
        [220, 30, 40, 255],
        [20, 180, 60, 255],
        [30, 70, 220, 255],
        [240, 190, 20, 255],
    ];
    let pixels: Vec<u8> = (0..8)
        .flat_map(|y| (0..8).flat_map(move |x| colors[(x + 2 * y) % 4]))
        .collect();
    let element = iced::widget::image::viewer(iced::widget::image::Handle::from_rgba(8, 8, pixels))
        .width(160.0)
        .height(120.0)
        .padding(4.0)
        .content_fit(iced::ContentFit::Contain)
        .filter_method(iced::widget::image::FilterMethod::Nearest)
        .min_scale(0.5)
        .max_scale(2.0)
        .scale_step(1.0);
    UserInterface::build(
        element,
        Size::new(600.0, 600.0),
        user_interface::Cache::default(),
        renderer,
    )
}
fn assert_reference(
    ui: &mut Ui,
    reference: &mut Ui,
    renderer: &mut iced::Renderer,
    bounds: Rectangle,
    stage: &str,
) -> Vec<u8> {
    let actual = crop(&draw(ui, renderer), bounds);
    let expected = crop(
        &draw(reference, renderer),
        Rectangle::with_size(bounds.size()),
    );
    assert!(actual == expected, "native viewer pixels: {stage}");
    if let Ok(directory) = std::env::var("VIEWER_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(
            directory.join(format!("{}.rgba", stage.replace(' ', "-"))),
            &actual,
        )
        .unwrap();
    }
    actual
}

#[test]
#[ignore = "requires actual native and Wasm viewer fixtures"]
fn bundled_viewer_zoom_pan_and_limits_match_native_pixels() {
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
        let bounds = viewports(&guest, &mut ui, &renderer)[0];
        assert_eq!(bounds.position(), Point::ORIGIN);
        let mut reference = reference(&mut renderer);
        let initial = assert_reference(
            &mut ui,
            &mut reference,
            &mut renderer,
            bounds,
            "initial copied options",
        );
        wheel(&mut ui, &mut renderer, bounds.center(), 1.0);
        wheel(&mut reference, &mut renderer, bounds.center(), 1.0);
        let zoomed = assert_reference(&mut ui, &mut reference, &mut renderer, bounds, "wheel zoom");
        assert_ne!(initial, zoomed, "wheel must visibly zoom the image");
        wheel(&mut ui, &mut renderer, bounds.center(), 1.0);
        wheel(&mut reference, &mut renderer, bounds.center(), 1.0);
        assert_eq!(
            zoomed,
            assert_reference(
                &mut ui,
                &mut reference,
                &mut renderer,
                bounds,
                "maximum scale"
            ),
            "max scale must stop further zoom"
        );
        let from = bounds.center();
        let to = from + iced::Vector::new(17.0, 11.0);
        pan(&mut ui, &mut renderer, from, to);
        pan(&mut reference, &mut renderer, from, to);
        let panned = assert_reference(
            &mut ui,
            &mut reference,
            &mut renderer,
            bounds,
            "pointer pan",
        );
        assert_ne!(zoomed, panned, "drag must visibly pan the zoomed image");
        for _ in 0..4 {
            wheel(&mut ui, &mut renderer, bounds.center(), -1.0);
            wheel(&mut reference, &mut renderer, bounds.center(), -1.0);
        }
        let minimum = assert_reference(
            &mut ui,
            &mut reference,
            &mut renderer,
            bounds,
            "minimum scale",
        );
        wheel(&mut ui, &mut renderer, bounds.center(), -1.0);
        assert_eq!(
            minimum,
            crop(&draw(&mut ui, &mut renderer), bounds),
            "minimum scale must stop zoom out"
        );
    }
}

#[test]
#[ignore = "requires actual native and Wasm viewer fixtures"]
fn bundled_viewer_retains_zoom_across_frames_and_keyed_reorder() {
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
        let bounds = viewports(&guest, &mut ui, &renderer);
        let initial = draw(&mut ui, &mut renderer);
        let plain = crop(&initial, bounds[0]);
        assert_eq!(plain, crop(&initial, bounds[1]));
        wheel(&mut ui, &mut renderer, bounds[0].center(), 1.0);
        let zoomed = crop(&draw(&mut ui, &mut renderer), bounds[0]);
        assert_ne!(plain, zoomed);
        click(&mut ui, &mut renderer, "Advance frame");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        fn advanced(node: &wire::Node) -> bool {
            matches!(node, wire::Node::Text { key, content, .. } if key.ends_with("/frame") && content == "1")
                || node.children().iter().any(advanced)
        }
        assert!(
            advanced(guest.lock().unwrap().frame.root.as_ref().unwrap()),
            "click must produce the next guest frame before retained-state assertion"
        );
        let current = draw(&mut ui, &mut renderer);
        assert_eq!(
            zoomed,
            crop(&current, bounds[0]),
            "same viewer identity must retain zoom after a guest frame"
        );
        assert_eq!(
            plain,
            crop(&current, bounds[1]),
            "another viewer must retain independent state"
        );
        click(&mut ui, &mut renderer, "Swap viewers");
        for _ in 0..3 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        let after = viewports(&guest, &mut ui, &renderer);
        let current = draw(&mut ui, &mut renderer);
        assert_eq!(
            plain,
            crop(&current, after[0]),
            "unzoomed keyed identity must move to the first slot"
        );
        assert_eq!(
            zoomed,
            crop(&current, after[1]),
            "zoom must follow keyed identity, not the old slot"
        );
    }
}
