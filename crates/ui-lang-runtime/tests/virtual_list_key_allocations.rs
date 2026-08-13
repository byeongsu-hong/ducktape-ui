use iced::{Element, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;
use ui_lang_runtime::{
    VirtualListConfig, VirtualListEvent, VirtualListId, VirtualListState, virtual_list,
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

fn config() -> VirtualListConfig {
    VirtualListConfig::new(20.0).unwrap().overscan(2)
}

#[test]
#[ignore = "virtual-list allocation contract run explicitly in CI"]
fn performance_contract_string_key_render_moves_mounted_keys() {
    const SAMPLES: usize = 256;
    const ALLOCATIONS_PER_RENDER: usize = 53;
    const BYTES_PER_RENDER: usize = 5_372;

    let items = (0..16)
        .map(|index| format!("row-key-{index:02}"))
        .collect::<Vec<_>>();
    let mut state = VirtualListState::new(VirtualListId::new("key-allocation-contract"));
    state.reconcile(&items, Clone::clone, config()).unwrap();
    state.apply(
        VirtualListEvent::ViewportChanged { height: 60.0 },
        &items,
        Clone::clone,
        config(),
    );
    assert_eq!(state.mounted_range(items.len(), config()).len(), 5);

    let build = || -> Element<'_, (), Theme, iced_test::renderer::Renderer> {
        virtual_list(
            &state,
            &items,
            config(),
            "",
            Clone::clone,
            |_| String::new(),
            |_, item, _| iced::widget::text(item.as_str()).into(),
            |_| (),
        )
    };
    drop(build());

    let stats = clean_window(
        (SAMPLES * ALLOCATIONS_PER_RENDER, SAMPLES * BYTES_PER_RENDER),
        || {
            for _ in 0..SAMPLES {
                drop(std::hint::black_box(build()));
            }
        },
    );

    eprintln!(
        "256 string-key virtual-list renders: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, SAMPLES * ALLOCATIONS_PER_RENDER);
    assert_eq!(stats.bytes_allocated, SAMPLES * BYTES_PER_RENDER);
}
