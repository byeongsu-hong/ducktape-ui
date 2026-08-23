use std::alloc::System;

use iced::{Element, Task, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::testing::{Config, Driver, Location};
use ui_lang_runtime::{Role, StableId, accessible};

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
/// returns the first window reporting `expected` allocations — or the last
/// window's stats, when none did.
fn clean_window(expected: usize, mut batch: impl FnMut()) -> stats_alloc::Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if stats.allocations == expected {
            break;
        }
    }
    stats
}

const HERE: Location = Location::new(
    "accessibility-role-name.ice",
    1,
    1,
    "read the semantic role name",
);

#[test]
fn repeated_public_role_name_reads_do_not_collect_characters() {
    const READS: usize = 32;

    let mut driver = Driver::new(
        iced::application::<(), (), Theme, iced::Renderer>(boot, update, view),
        Config::new("accessibility_role_name_allocations").viewport(160.0, 80.0),
    );
    let target = driver.target("App/item", HERE);
    assert_eq!(target.accessibility_role_name(), "tree-item");

    let stats = clean_window(READS * 2, || {
        for _ in 0..READS {
            assert_eq!(
                std::hint::black_box(target.accessibility_role_name()),
                "tree-item"
            );
        }
    });

    eprintln!(
        "{READS} role name reads: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, READS * 2, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}

fn boot() {}

fn update(_state: &mut (), _message: ()) -> Task<()> {
    Task::none()
}

fn view(_state: &()) -> Element<'_, ()> {
    accessible(
        iced::widget::text("item"),
        StableId::new("App/item"),
        Role::TreeItem,
    )
    .logical_id("App/item")
    .into()
}
