use std::alloc::System;
use std::fmt::Write as _;
use std::hint::black_box;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
#[ignore = "parser allocation contract; run alone with --test-threads=1"]
fn performance_contract_line_tree_moves_owned_lines() {
    const LINES: usize = 4_000;
    let mut source = String::from("app LineTree\nview\n  col\n");
    for index in 0..LINES {
        writeln!(source, "    text \"line {index}\"").unwrap();
    }

    let region = Region::new(GLOBAL);
    let document = black_box(ui_lang_core::parse(black_box(&source))).unwrap();
    let stats = region.change();

    assert_eq!(document.app, "LineTree");
    eprintln!(
        "{LINES} view lines: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 37_000, "{stats:?}");
}
