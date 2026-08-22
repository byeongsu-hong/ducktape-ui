#![cfg(feature = "toggle-group")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::{self, text};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::theme::LIGHT;
use ui_lang_components::ui::toggle_group::{
    ToggleGroupOrientation, ToggleGroupState, toggle_group, toggle_group_item,
};

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

fn render(
    ids: &[widget::Id],
    state: &ToggleGroupState<usize>,
    orientation: ToggleGroupOrientation,
) {
    let items = ids
        .iter()
        .enumerate()
        .map(|(index, id)| toggle_group_item(id.clone(), index, text("Toggle"), ()));
    let group: Element<'_, ()> = toggle_group(items, state, orientation, |_| (), &LIGHT).into();
    drop(black_box(group));
}

#[test]
fn performance_contract_toggle_group_reuses_control_storage() {
    const RENDERS: usize = 128;
    const ITEMS: usize = 64;
    let ids = (0..ITEMS).map(|_| widget::Id::unique()).collect::<Vec<_>>();
    let state = ToggleGroupState::Single(Some(0));
    let orientations = [
        ToggleGroupOrientation::Horizontal,
        ToggleGroupOrientation::Vertical,
    ];

    for orientation in orientations {
        render(&ids, &state, orientation);
    }
    let stats = clean_window((132_096, 45_130_752), || {
        for _ in 0..RENDERS {
            for orientation in orientations {
                render(&ids, &state, orientation);
            }
        }
    });

    eprintln!(
        "{RENDERS} horizontal and vertical toggle groups with {ITEMS} controls: \
         {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 132_096, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 45_130_752, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
