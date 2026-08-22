#![cfg(feature = "accordion")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::{self, text};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::accordion::{AccordionState, accordion, accordion_item};
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

fn render(focus_ids: &[widget::Id]) {
    let view: Element<'static, ()> =
        accordion(
            focus_ids.iter().cloned().enumerate().map(|(id, focus_id)| {
                accordion_item(id, focus_id, text("Header"), text("Content"))
            }),
            &AccordionState::Single(None),
            |_| (),
            &LIGHT,
        );
    drop(black_box(view));
}

#[test]
fn performance_contract_accordion_preallocates_section_storage() {
    const RENDERS: usize = 256;
    const ITEMS: usize = 64;

    let focus_ids = (0..ITEMS).map(|_| widget::Id::unique()).collect::<Vec<_>>();
    render(&focus_ids);
    let stats = clean_window((247_296, 55_883_776), || {
        for _ in 0..RENDERS {
            render(&focus_ids);
        }
    });

    eprintln!(
        "{RENDERS} accordion renders with {ITEMS} items: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 247_296, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 55_883_776, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
