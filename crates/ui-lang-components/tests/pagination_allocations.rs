#![cfg(feature = "pagination")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::pagination::{PaginationItem, pagination, pagination_with_content};
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

fn render(items: &[PaginationItem]) {
    let default: Element<'_, usize> = pagination(items.iter().copied(), |page| page, &LIGHT).into();
    drop(black_box(default));

    let custom: Element<'_, usize> =
        pagination_with_content(items.iter().copied(), |_| text("Page").into(), &LIGHT).into();
    drop(black_box(custom));
}

#[test]
fn performance_contract_pagination_preallocates_row_storage() {
    const ITEMS: usize = 64;
    const RENDERS: usize = 256;
    let items = (0..ITEMS)
        .map(|number| PaginationItem::Page {
            number,
            current: number == ITEMS / 2,
        })
        .collect::<Vec<_>>();

    render(&items);
    let stats = clean_window((132_096, 46_986_752), || {
        for _ in 0..RENDERS {
            render(&items);
        }
    });

    eprintln!(
        "{RENDERS} default and custom paginations with {ITEMS} items: {} allocations / \
         {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 132_096, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 46_986_752, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
