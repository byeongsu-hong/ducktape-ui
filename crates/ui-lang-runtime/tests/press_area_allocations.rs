use std::alloc::System;

use iced::{Element, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::press_area;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// loaded runner that lands inside the region as +4 allocations. Code under
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

type Renderer = iced_test::renderer::Renderer;

#[test]
fn press_callbacks_use_the_widget_allocation() {
    const FRAMES: usize = 64;

    // One batch's worth of contents per window, built before any window opens
    // so the measurement only ever sees the press areas.
    let mut contents: Vec<Element<'static, u64, Theme, Renderer>> = (0..FRAMES * WINDOWS)
        .map(|_| iced::widget::Space::new().into())
        .collect();

    let stats = clean_window((FRAMES, 2_048), || {
        for (index, content) in contents.drain(..FRAMES).enumerate() {
            let area: Element<'static, u64, Theme, Renderer> = press_area(content)
                .on_press_at(move |_| index as u64)
                .into();
            drop(std::hint::black_box(area));
        }
    });

    assert_eq!(
        stats.allocations, FRAMES,
        "{FRAMES} press areas allocated {} times ({} bytes)",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(
        stats.bytes_allocated, 2_048,
        "{FRAMES} press areas allocated {} bytes",
        stats.bytes_allocated
    );
}
