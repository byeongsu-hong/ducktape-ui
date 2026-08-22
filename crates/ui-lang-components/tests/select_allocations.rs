#![cfg(feature = "select")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::menu::MenuState;
use ui_lang_components::ui::select::{SelectGroup, SelectIds, SelectOption, select};
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

const OPTIONS: usize = 100;

fn groups() -> Vec<SelectGroup<usize>> {
    vec![SelectGroup::new(
        "commands",
        (0..OPTIONS)
            .map(|index| SelectOption::new(format!("option-{index}"), index, "Option"))
            .collect(),
    )]
}

fn build<'a>(groups: Vec<SelectGroup<usize>>, state: &'a MenuState) -> Element<'a, ()> {
    select(
        SelectIds::new("allocation-contract"),
        groups,
        None,
        String::new(),
        state,
        false,
        |_| (),
        &LIGHT,
    )
    .trigger(text("Select"))
    .into()
}

#[test]
fn performance_contract_select_reuses_owned_groups() {
    let state = MenuState::default();
    drop(build(groups(), &state));

    let mut pending: Vec<_> = (0..WINDOWS).map(|_| groups()).collect();
    let mut element = None;
    let stats = clean_window((23_428, 1_370_158), || {
        element = Some(build(black_box(pending.pop().unwrap()), black_box(&state)));
    });
    let element = element.unwrap();

    assert_eq!(element.as_widget().children().len(), 2);
    eprintln!(
        "{OPTIONS} select options: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.allocations, 23_428, "{stats:?}");
    assert_eq!(stats.reallocations, 11_314, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 1_370_158, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 750_290, "{stats:?}");
}
