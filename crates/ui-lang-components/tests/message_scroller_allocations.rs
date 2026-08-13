#![cfg(feature = "message-scroller")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::Space;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::message_scroller::{
    MessageScrollerState, controlled_message_scroller_with_end_content, message_scroller_item,
};
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
        let mut region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if (stats.allocations, stats.bytes_allocated) == expected {
            break;
        }
    }
    stats
}

fn render(state: &MessageScrollerState) {
    let items = (0..256).map(|_| message_scroller_item("", Space::new()));
    let element: Element<'_, ()> = controlled_message_scroller_with_end_content(
        state,
        items,
        |_| (),
        |_, _| Space::new().into(),
        &LIGHT,
    )
    .into();
    black_box(element);
}

#[test]
fn performance_contract_message_scroller_reuses_row_buffer() {
    const RENDERS: usize = 64;
    let state = MessageScrollerState::new("allocation-contract");
    render(&state);

    let stats = clean_window((50_368, 4_590_208), || {
        for _ in 0..RENDERS {
            render(&state);
        }
    });

    eprintln!(
        "{RENDERS} message-scroller renders: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 50_368, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 4_590_208, "{stats:?}");
}
