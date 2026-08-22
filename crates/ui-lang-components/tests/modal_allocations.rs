#![cfg(feature = "modal")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::advanced::widget;
use iced::widget::text;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::modal::{DismissRules, FocusScope, modal};
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

fn build(focus: &FocusScope) -> Element<'static, ()> {
    modal(
        text("page"),
        true,
        text("dialog"),
        focus,
        DismissRules::DIALOG,
        |_| (),
        &LIGHT,
    )
}

#[test]
fn performance_contract_modal_shares_focus_order() {
    const BUILDS: usize = 4_000;
    let focus = FocusScope::new(widget::Id::new("first"), widget::Id::new("restore"))
        .push(widget::Id::new("second"));
    let element = build(&focus);
    assert_eq!(element.as_widget().children().len(), 2);
    drop(element);

    let stats = clean_window((12_000, 1_312_000), || {
        for _ in 0..BUILDS {
            drop(black_box(build(black_box(&focus))));
        }
    });

    eprintln!(
        "{BUILDS} modal builds: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.allocations, 12_000, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 1_312_000, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
