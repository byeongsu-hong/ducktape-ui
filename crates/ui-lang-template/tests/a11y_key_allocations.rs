use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_template::A11y;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn accessibility_keys_allocate_the_exact_output_once() {
    const KEYS: usize = 4_096;
    let parent = format!("App/{}", "parent-scope/".repeat(8));
    let a11y = A11y {
        segment: "named-control".repeat(4),
        named: true,
        source: None,
    };
    let output_len = parent.len() + 1 + a11y.segment.len();

    let region = Region::new(GLOBAL);
    for _ in 0..KEYS {
        std::hint::black_box(a11y.key(std::hint::black_box(&parent)));
    }
    let stats = region.change();

    eprintln!(
        "{KEYS} accessibility keys: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, KEYS, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, KEYS * output_len, "{stats:?}");
}
