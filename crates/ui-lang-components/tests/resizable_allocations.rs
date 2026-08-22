#![cfg(feature = "resizable")]

mod common;

use common::GLOBAL;

use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer::Headless as _, widget};
use iced::widget::{container, text};
use iced::{Element, Event, Point, Rectangle, Size, touch};
use stats_alloc::Region;
use ui_lang_components::ui::resizable::resizable;
use ui_lang_components::ui::theme::LIGHT;

fn update(
    element: &mut Element<'_, Vec<f32>>,
    tree: &mut widget::Tree,
    node: &layout::Node,
    renderer: &iced::Renderer,
    viewport: &Rectangle,
    event: Event,
    messages: &mut Vec<Vec<f32>>,
) -> iced::event::Status {
    let mut clipboard = clipboard::Null;
    let mut shell = Shell::new(messages);
    element.as_widget_mut().update(
        tree,
        &event,
        Layout::new(node),
        mouse::Cursor::Unavailable,
        renderer,
        &mut clipboard,
        &mut shell,
        viewport,
    );
    shell.event_status()
}

#[test]
fn performance_contract_resizable_drag_reuses_state() {
    const UPDATES: usize = 4_000;

    let panels = ["One", "Two"].map(|label| container(text(label)).into());
    let mut element: Element<'_, Vec<f32>> = resizable(
        "allocation-contract",
        panels,
        vec![0.5, 0.5],
        vec![0.1, 0.1],
        |sizes| sizes,
        &LIGHT,
    )
    .into();
    let renderer = iced::futures::executor::block_on(iced::Renderer::new(
        iced::Font::default(),
        iced::Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let viewport = Rectangle::with_size(Size::new(200.0, 100.0));
    let mut tree = widget::Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let finger = touch::Finger(7);
    let mut messages = Vec::with_capacity(UPDATES);
    let event = |x| {
        Event::Touch(touch::Event::FingerMoved {
            id: finger,
            position: Point::new(x, 50.0),
        })
    };
    macro_rules! send {
        ($event:expr) => {
            update(
                &mut element,
                &mut tree,
                &node,
                &renderer,
                &viewport,
                $event,
                &mut messages,
            )
        };
    }

    assert_eq!(
        send!(Event::Touch(touch::Event::FingerPressed {
            id: finger,
            position: Point::new(100.0, 50.0),
        })),
        iced::event::Status::Captured
    );
    assert_eq!(send!(event(110.0)), iced::event::Status::Captured);
    messages.clear();

    let region = Region::new(GLOBAL);
    for index in 0..UPDATES {
        assert_eq!(
            send!(event(if index % 2 == 0 { 120.0 } else { 130.0 })),
            iced::event::Status::Captured
        );
    }
    let stats = region.change();

    eprintln!(
        "{UPDATES} warmed resizable drag updates: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(messages.len(), UPDATES);
    assert!((messages.last().unwrap()[0] - 0.65).abs() < 0.000_01);
    assert!(stats.allocations <= UPDATES, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(
        stats.bytes_allocated <= UPDATES * 2 * size_of::<f32>(),
        "{stats:?}"
    );
}
