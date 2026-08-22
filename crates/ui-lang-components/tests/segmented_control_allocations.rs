#![cfg(feature = "segmented-control")]

mod common;

use common::clean_window;

use std::hint::black_box;

use ui_lang_components::ui::segmented_control::segmented_control;
use ui_lang_components::ui::theme::LIGHT;

fn render(segments: usize) {
    black_box(segmented_control(
        (0..segments).map(|index| (index, "Segment")),
        0,
        |_| (),
        &LIGHT,
    ));
}

#[test]
fn performance_contract_segmented_control_preallocates_child_storage() {
    const RENDERS: usize = 256;
    const SEGMENTS: usize = 64;

    render(SEGMENTS);
    let stats = clean_window((99_072, 44_327_936), || {
        for _ in 0..RENDERS {
            render(SEGMENTS);
        }
    });

    eprintln!(
        "{RENDERS} segmented control renders with {SEGMENTS} segments: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 99_072, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 44_327_936, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
