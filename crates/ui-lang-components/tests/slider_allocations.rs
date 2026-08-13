#![cfg(feature = "slider")]

use std::alloc::System;
use std::hint::black_box;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::slider::{SliderCommand, SliderSpec, reduce_thumb};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn performance_contract_slider_keyboard_update_reuses_normalized_values() {
    const UPDATES: usize = 4_000;
    let values = [20.0, 80.0];
    let spec = SliderSpec::new(0.0..=100.0, 5.0);

    drop(black_box(reduce_thumb(
        &values,
        1,
        SliderCommand::Increment,
        spec,
    )));

    let region = Region::new(GLOBAL);
    for _ in 0..UPDATES {
        drop(black_box(reduce_thumb(
            black_box(&values),
            1,
            SliderCommand::Increment,
            spec,
        )));
    }
    let stats = region.change();

    eprintln!(
        "{UPDATES} slider keyboard updates: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= UPDATES, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
