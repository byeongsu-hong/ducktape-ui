//! Authored interaction faces rendered by both guest backends against native Iced.
use super::image_tests::{crop, draw_at};
use super::layers_tests::{Ui, build, container_bounds, redraw, renderer};
use super::*;
use iced::{Background, Color, Event, Point, Rectangle, Size, mouse, widget};
use iced_test::runtime::{UserInterface, user_interface};

fn send(ui: &mut Ui, renderer: &mut iced::Renderer, event: mouse::Event, at: Point) {
    ui.update(
        &[Event::Mouse(event)],
        mouse::Cursor::Available(at),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
}
fn key(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Container { key, .. } = node
        && key.ends_with(suffix)
    {
        return Some(key.clone());
    }
    node.children().iter().find_map(|node| key(node, suffix))
}
fn reference(renderer: &mut iced::Renderer, round: bool) -> Ui {
    let slider = widget::slider(0.0..=100.0, 50.0, |_| String::new())
        .step(1.0)
        .width(160.0)
        .height(24.0)
        .style(move |theme, status| {
            use widget::slider::{HandleShape, Status};
            let mut style = widget::slider::default(theme, status);
            style.rail.backgrounds = (
                Background::Color(Color::from_rgb8(0x24, 0x68, 0xc4)),
                Background::Color(Color::from_rgb8(0x33, 0x33, 0x33)),
            );
            style.rail.width = 3.0;
            style.rail.border.radius = 0.0.into();
            style.handle.background = Background::Color(Color::from_rgb8(0xe1, 0x2d, 0x39));
            style.handle.border_width = 0.0;
            style.handle.shape = if round {
                HandleShape::Circle {
                    radius: match status {
                        Status::Active => 0.0,
                        Status::Hovered => 4.0,
                        Status::Dragged => 5.0,
                    },
                }
            } else {
                let (width, radius) = match status {
                    Status::Active => (12, 3.0),
                    Status::Hovered => (18, 5.0),
                    Status::Dragged => (12, 0.0),
                };
                HandleShape::Rectangle {
                    width,
                    border_radius: radius.into(),
                }
            };
            style
        });
    UserInterface::build(
        slider,
        Size::new(600.0, 600.0),
        user_interface::Cache::default(),
        renderer,
    )
}
fn assert_pixels(
    ui: &mut Ui,
    reference: &mut Ui,
    renderer: &mut iced::Renderer,
    bounds: Rectangle,
    point: Option<Point>,
    stage: &str,
) -> Vec<u8> {
    let cursor = point.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available);
    let relative = point
        .map(|p| Point::new(p.x - bounds.x, p.y - bounds.y))
        .map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available);
    // Iced commits slider Status on RedrawRequested, before painting.
    for (tree, at) in [(&mut *ui, cursor), (&mut *reference, relative)] {
        tree.update(
            &[Event::Window(iced::window::Event::RedrawRequested(
                Instant::now(),
            ))],
            at,
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
    }
    let actual = crop(&draw_at(ui, renderer, cursor), bounds);
    assert!(
        actual
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] != [255, 255, 255]),
        "slider must paint colored pixels: {stage}"
    );
    let expected = crop(
        &draw_at(reference, renderer, relative),
        Rectangle::with_size(bounds.size()),
    );
    assert_eq!(
        actual.iter().zip(&expected).position(|(a, b)| a != b),
        None,
        "slider handle native reference pixels: {stage}"
    );
    if let Ok(directory) = std::env::var("SLIDER_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join(format!("{stage}.rgba")), &actual).unwrap();
    }
    actual
}
#[test]
#[ignore = "requires bundled and native slider handle fixtures"]
fn bundled_slider_handles_match_native_faces_and_drag_routes() {
    for native in [false, true] {
        for round in [true, false] {
            let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
                "../target/slider-handles-native"
            } else {
                "../target/slider-handles-fixture"
            });
            let entries = crate::catalog::scan_dir(&directory);
            assert_eq!(entries.len(), 1, "build slider handle fixtures first");
            let guest = Arc::new(Mutex::new(Guest::load(&entries[0]).unwrap()));
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
            let suffix = if round { "/round-view" } else { "/rect-view" };
            let key = key(guest.lock().unwrap().frame.root.as_ref().unwrap(), suffix).unwrap();
            let bounds = container_bounds(&mut ui, &renderer, &key);
            assert_eq!(bounds.size(), Size::new(160.0, 24.0));
            let mut expected = reference(&mut renderer, round);
            let idle = assert_pixels(
                &mut ui,
                &mut expected,
                &mut renderer,
                bounds,
                None,
                &format!("native-{native}-round-{round}-active"),
            );
            let at = bounds.center();
            let relative = Point::new(80.0, 12.0);
            send(
                &mut ui,
                &mut renderer,
                mouse::Event::CursorMoved { position: at },
                at,
            );
            send(
                &mut expected,
                &mut renderer,
                mouse::Event::CursorMoved { position: relative },
                relative,
            );
            let hover = assert_pixels(
                &mut ui,
                &mut expected,
                &mut renderer,
                bounds,
                Some(at),
                &format!("native-{native}-round-{round}-hovered"),
            );
            assert!(idle != hover, "hover must change the handle geometry");
            send(
                &mut ui,
                &mut renderer,
                mouse::Event::ButtonPressed(mouse::Button::Left),
                at,
            );
            send(
                &mut expected,
                &mut renderer,
                mouse::Event::ButtonPressed(mouse::Button::Left),
                relative,
            );
            let drag = assert_pixels(
                &mut ui,
                &mut expected,
                &mut renderer,
                bounds,
                Some(at),
                &format!("native-{native}-round-{round}-dragged"),
            );
            assert!(hover != drag, "drag must change the handle geometry");
            let to = Point::new(bounds.x + 120.0, at.y);
            send(
                &mut ui,
                &mut renderer,
                mouse::Event::CursorMoved { position: to },
                to,
            );
            send(
                &mut ui,
                &mut renderer,
                mouse::Event::ButtonReleased(mouse::Button::Left),
                to,
            );
            for _ in 0..3 {
                ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
            }
            fn value(node: &wire::Node) -> Option<f32> {
                if let wire::Node::Slider { value, .. } = node {
                    return Some(*value);
                }
                node.children().iter().find_map(value)
            }
            assert_eq!(
                value(guest.lock().unwrap().frame.root.as_ref().unwrap()),
                Some(75.0),
                "real drag must reach the authored slide handler, native={native}"
            );
        }
    }
}
