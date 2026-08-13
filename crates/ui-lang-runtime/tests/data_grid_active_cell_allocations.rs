use std::alloc::System;

use iced::{Element, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::{
    DataGridColumn, DataGridConfig, DataGridEvent, DataGridId, DataGridState, data_grid,
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

type Message = DataGridEvent<String, String>;
type Renderer = iced_test::renderer::Renderer;

#[test]
fn active_cell_moves_into_the_rendered_grid() {
    const FRAMES: usize = 256;
    const ALLOCATIONS: usize = 15_616;
    const ALLOCATED_BYTES: usize = 1_320_960;

    let config = DataGridConfig::new(20.0, 20.0).unwrap();
    let rows = [String::from("row-key-owned")];
    let columns = [DataGridColumn::new(
        String::from("column-key-owned"),
        "Column",
        100.0,
    )];
    let mut state = DataGridState::new(DataGridId::new("active-cell-allocation-contract"));
    state
        .reconcile(&rows, Clone::clone, &columns, config)
        .unwrap();
    state.apply(
        DataGridEvent::ViewportChanged {
            width: 100.0,
            height: 20.0,
        },
        config,
    );
    state.apply(
        DataGridEvent::FocusCell {
            row_index: 0,
            row: rows[0].clone(),
            column_index: 0,
            column: columns[0].key().clone(),
        },
        config,
    );

    let stats = clean_window((ALLOCATIONS, ALLOCATED_BYTES), || {
        for _ in 0..FRAMES {
            let element: Element<'_, Message, Theme, Renderer> = data_grid(
                &state,
                &rows,
                config,
                "Grid",
                Clone::clone,
                Clone::clone,
                |_, column| column.label().to_owned(),
                |_| None,
                |header| iced::widget::text(header.column.label()).into(),
                |_| iced::widget::space().into(),
                |event| event,
            );
            drop(std::hint::black_box(element));
        }
    });

    eprintln!(
        "{FRAMES} active-grid renders: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, ALLOCATIONS);
    assert_eq!(stats.bytes_allocated, ALLOCATED_BYTES);
}
