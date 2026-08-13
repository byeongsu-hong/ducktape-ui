#![cfg(feature = "sonner")]

use std::alloc::System;
use std::hint::black_box;
use std::time::Duration;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::sonner::{SonnerState, ToastPlacement, sonner_with_content};
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

fn render(state: &SonnerState) {
    black_box(sonner_with_content(
        state,
        |_| (),
        |entry, _controls, _theme| Element::from(iced::widget::text(entry.data().title())),
        &LIGHT,
    ));
}

#[test]
fn performance_contract_sonner_preallocates_visible_entries() {
    const RENDERS: usize = 256;
    const VISIBLE: usize = 32;
    let mut state = SonnerState::new(VISIBLE, ToastPlacement::TopRight);
    for index in 0..VISIBLE {
        state.show(format!("Toast {index}"), Duration::ZERO);
    }

    for _ in 0..RENDERS {
        render(&state);
    }

    let stats = clean_window((41_984, 2_494_464), || {
        for _ in 0..RENDERS {
            render(&state);
        }
    });

    eprintln!(
        "{RENDERS} Sonner renders: {} allocations / {} reallocations / {} bytes / \
         {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 41_984, "{stats:?}");
    assert_eq!(stats.reallocations, 768, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 2_494_464, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 229_376, "{stats:?}");
}
