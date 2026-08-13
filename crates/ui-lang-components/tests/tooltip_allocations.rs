#![cfg(feature = "tooltip")]

use std::alloc::System;
use std::hint::black_box;

use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer::Headless as _, widget};
use iced::time::Duration;
use iced::widget::text;
use iced::{Element, Event, Rectangle, Size, Vector};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::theme::LIGHT;
use ui_lang_components::ui::tooltip::{TooltipId, tooltip};

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
        let mut region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if (stats.allocations, stats.bytes_allocated) == expected {
            break;
        }
    }
    stats
}

#[test]
fn performance_contract_tooltip_returns_single_overlay_directly() {
    const PASSES: usize = 4_000;
    let mut element: Element<'_, ()> = tooltip(
        TooltipId::new("allocation-contract"),
        text("trigger"),
        text("hint"),
        &LIGHT,
    )
    .open_delay(Duration::ZERO)
    .into();
    let renderer = iced::futures::executor::block_on(iced::Renderer::new(
        iced::Font::default(),
        iced::Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let viewport = Rectangle::with_size(Size::new(320.0, 200.0));
    let mut tree = widget::Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let cursor = mouse::Cursor::Available(node.bounds().center());
    let mut messages = Vec::new();
    let mut clipboard = clipboard::Null;
    let mut shell = Shell::new(&mut messages);
    element.as_widget_mut().update(
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved {
            position: node.bounds().center(),
        }),
        Layout::new(&node),
        cursor,
        &renderer,
        &mut clipboard,
        &mut shell,
        &viewport,
    );

    let mut overlay = element
        .as_widget_mut()
        .overlay(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &viewport,
            Vector::ZERO,
        )
        .expect("open tooltip overlay");
    let overlay_node = overlay.as_overlay_mut().layout(&renderer, viewport.size());
    assert!(overlay_node.bounds().width > 0.0);
    drop(overlay);

    let stats = clean_window((PASSES, 120 * PASSES), || {
        for _ in 0..PASSES {
            drop(black_box(element.as_widget_mut().overlay(
                black_box(&mut tree),
                Layout::new(black_box(&node)),
                black_box(&renderer),
                black_box(&viewport),
                Vector::ZERO,
            )));
        }
    });

    eprintln!(
        "{PASSES} tooltip overlays: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.allocations, PASSES, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 120 * PASSES, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
