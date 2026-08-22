#![cfg(feature = "command")]

mod common;

use common::clean_window;

use std::hint::black_box;

use ui_lang_components::ui::command::{command_group, command_item, filter_items};

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
    let stats = clean_window((ITEMS + 1, 70_910), || {
        let matches = filter_items(black_box(&groups), black_box("  missing   command "));
        assert!(matches.is_empty());
    });

    eprintln!(
        "{ITEMS} command filters: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.allocations, ITEMS + 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 70_910, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
