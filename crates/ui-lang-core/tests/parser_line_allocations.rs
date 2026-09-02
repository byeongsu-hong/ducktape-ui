use std::fmt::Write as _;
use std::hint::black_box;

mod common;

use common::clean_window_allocations;

#[test]
#[ignore = "parser allocation contract; run alone with --test-threads=1"]
fn performance_contract_line_tree_moves_owned_lines() {
    const LINES: usize = 4_000;
    const BUDGET: usize = 20_013;
    let mut source = String::from("app LineTree\nview\n  col\n");
    for index in 0..LINES {
        writeln!(source, "    text \"line {index}\"").unwrap();
    }

    let mut document = None;
    let stats = clean_window_allocations(BUDGET, || {
        document = Some(black_box(ui_lang_core::parse(black_box(&source))).unwrap());
    });

    assert_eq!(document.unwrap().app, "LineTree");
    eprintln!(
        "{LINES} view lines: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= BUDGET, "{stats:?}");
}
