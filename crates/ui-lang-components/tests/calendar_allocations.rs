#![cfg(feature = "calendar")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use ui_lang_components::ui::calendar::{
    CalendarSelection, CalendarState, Month, controlled_calendar,
};
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::theme::LIGHT;

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
    let stats = clean_window((122_112, 9_553_152), || {
        for _ in 0..RENDERS {
            render(&state);
        }
    });

    eprintln!(
        "{RENDERS} RTL dropdown calendar renders: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 122_624, "{stats:?}");
    assert!(stats.reallocations <= 13_312, "{stats:?}");
    assert!(stats.bytes_allocated <= 9_754_880, "{stats:?}");
}
