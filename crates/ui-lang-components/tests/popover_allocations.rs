#![cfg(feature = "popover")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::advanced::{Layout, layout, renderer::Headless as _, widget};
use iced::widget::text;
use iced::{Element, Font, Pixels, Point, Rectangle, Size, Vector};
use ui_lang_components::ui::popover::{PopoverEvent, PopoverIds, popover};
use ui_lang_components::ui::theme::LIGHT;

#[test]
fn performance_contract_popover_returns_sole_overlay_directly() {
    const OVERLAYS: usize = 4_096;
    let renderer = iced::futures::executor::block_on(iced::Renderer::new(
        Font::default(),
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let viewport = Rectangle::new(Point::ORIGIN, Size::new(320.0, 240.0));
    let mut element: Element<'_, PopoverEvent> = popover(
        PopoverIds::new("allocation-contract"),
        text("trigger"),
        text("content"),
        true,
        |event| event,
        &LIGHT,
    )
    .into();
    let mut tree = widget::Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );

    assert!(
        element
            .as_widget_mut()
            .overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &viewport,
                Vector::ZERO,
            )
            .is_some()
    );
    let stats = clean_window((OVERLAYS, OVERLAYS * 120), || {
        for _ in 0..OVERLAYS {
            let overlay = element.as_widget_mut().overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &viewport,
                Vector::ZERO,
            );
            assert!(overlay.is_some());
            black_box(overlay);
        }
    });

    eprintln!(
        "{OVERLAYS} open popover overlay queries: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );

    assert_eq!(stats.allocations, OVERLAYS, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, OVERLAYS * 120, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
