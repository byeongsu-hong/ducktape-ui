#![cfg(feature = "command")]

use std::alloc::System;
use std::hint::black_box;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::command::{command_group, command_item, filter_items};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn performance_contract_command_filter_normalizes_query_once() {
    const ITEMS: usize = 4_000;
    let groups = [command_group(
        "Commands",
        (0..ITEMS).map(|index| {
            command_item(
                format!("command-{index}"),
                index,
                format!("Open calendar {index}"),
            )
        }),
    )];

    drop(filter_items(
        black_box(&groups),
        black_box("  missing   command "),
    ));
    let region = Region::new(GLOBAL);
    let matches = filter_items(black_box(&groups), black_box("  missing   command "));
    let stats = region.change();

    assert!(matches.is_empty());
    eprintln!(
        "{ITEMS} command filters: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 12_003, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 397_874, "{stats:?}");
}
