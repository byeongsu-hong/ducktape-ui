#![cfg(feature = "popover")]

use std::alloc::System;
use std::hint::black_box;

use iced::advanced::{Layout, layout, renderer::Headless as _, widget};
use iced::widget::text;
use iced::{Element, Font, Pixels, Point, Rectangle, Size, Vector};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::popover::{PopoverEvent, PopoverIds, popover};
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// 4-core runner that lands inside the region as +4 allocations. Code under
/// test that allocated would dirty *every* window; a one-time foreign block
/// dirties at most one. So the batch runs in its own window, up to
/// [`WINDOWS`] times, and the contract asks for one clean window rather than a
/// clean process.
const WINDOWS: usize = 4;

/// Runs `batch` in a fresh allocator window, up to [`WINDOWS`] times, and
/// returns the first window whose `(allocations, bytes_allocated)` equal
/// `expected` — or the last window's stats, when none did.
fn clean_window(expected: (usize, usize), mut batch: impl FnMut()) -> stats_alloc::Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if (stats.allocations, stats.bytes_allocated) == expected {
            break;
        }
    }
    stats
}

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
