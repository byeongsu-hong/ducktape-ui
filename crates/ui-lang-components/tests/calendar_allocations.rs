#![cfg(feature = "calendar")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::calendar::{
    CalendarSelection, CalendarState, Month, controlled_calendar,
};
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn render(state: &CalendarState) {
    let element: Element<'_, ()> =
        controlled_calendar("allocation-contract", state, |_| (), &LIGHT)
            .month_dropdown(true)
            .year_dropdown(true)
            .direction(Direction::RightToLeft)
            .into();
    black_box(element);
}

#[test]
fn performance_contract_calendar_reuses_rtl_caption_storage() {
    const RENDERS: usize = 256;
    let state = CalendarState::new(
        Month::new(2026, 8).unwrap(),
        CalendarSelection::Single(None),
    );

    render(&state);
    let region = Region::new(GLOBAL);
    for _ in 0..RENDERS {
        render(&state);
    }
    let stats = region.change();

    eprintln!(
        "{RENDERS} RTL dropdown calendar renders: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 122_624, "{stats:?}");
    assert!(stats.reallocations <= 13_312, "{stats:?}");
    assert!(stats.bytes_allocated <= 34_959_616, "{stats:?}");
}
