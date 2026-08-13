use std::alloc::System;
use std::hint::black_box;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_core::SymbolKind;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn validating_component_names_does_not_allocate() {
    const VALIDATIONS: usize = 4_000;

    black_box(SymbolKind::Component.accepts(black_box("catalog::controls::Card.Header")));
    let region = Region::new(GLOBAL);
    let accepted = (0..VALIDATIONS)
        .filter(|_| {
            black_box(SymbolKind::Component.accepts(black_box("catalog::controls::Card.Header")))
        })
        .count();
    let stats = region.change();

    assert_eq!(accepted, VALIDATIONS);
    eprintln!(
        "{VALIDATIONS} component-name validations: {} allocations, {} bytes",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(
        (stats.allocations, stats.bytes_allocated),
        (0, 0),
        "{VALIDATIONS} component-name validations allocated"
    );
}
