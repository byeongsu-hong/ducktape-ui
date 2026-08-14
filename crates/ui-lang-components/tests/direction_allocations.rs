#![cfg(feature = "sheet")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::sheet::sheet_panel;
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn view() -> Element<'static, ()> {
    sheet_panel(text("Body"), &LIGHT)
        .header(text("Settings"))
        .close(text("Close"))
        .into_widget()
        .into()
}

#[test]
fn performance_contract_directed_rows_stream_default_order() {
    const RENDERS: usize = 1_024;

    drop(black_box(view()));

    let region = Region::new(GLOBAL);
    for _ in 0..RENDERS {
        drop(black_box(view()));
    }
    let stats = region.change();

    eprintln!(
        "{RENDERS} directed sheet headers: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 9_220, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 828_292, "{stats:?}");
}
