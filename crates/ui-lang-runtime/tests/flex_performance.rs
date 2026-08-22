use std::alloc::System;

use iced::advanced::Widget;
use iced::advanced::widget::Tree;
use iced::{Element, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::{flex, flex_item};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// 4-core runner that lands inside the region as +4 allocations / ~900 bytes.
/// A diff that allocated would dirty *every* window; a one-time foreign block
/// dirties at most one. So the frames run in their own window, up to
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

fn items(count: usize) -> Vec<ui_lang_runtime::FlexItem<'static, (), Theme, Renderer>> {
    (0..count)
        .map(|_| {
            let child: Element<'static, (), Theme, Renderer> = iced::widget::Space::new().into();
            flex_item(child)
        })
        .collect()
}

#[test]
fn repeated_flex_diff_does_not_allocate_a_child_reference_vector() {
    const ITEMS: usize = 1_000;
    const FRAMES: usize = 32;

    let initial = flex(items(ITEMS));
    let mut tree = Tree::new(&initial as &dyn Widget<(), Theme, Renderer>);
    let unchanged = flex(items(ITEMS));
    unchanged.diff(std::hint::black_box(&mut tree));

    let stats = clean_window((0, 0), || {
        for _ in 0..FRAMES {
            unchanged.diff(std::hint::black_box(&mut tree));
        }
    });

    assert_eq!(stats.allocations, 0, "{stats:?}");
}
