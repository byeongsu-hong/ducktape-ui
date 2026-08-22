#![cfg(feature = "accordion")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use iced::widget::{self, text};
use ui_lang_components::ui::accordion::{AccordionState, accordion, accordion_item};
use ui_lang_components::ui::theme::LIGHT;

fn render(focus_ids: &[widget::Id]) {
    let view: Element<'static, ()> =
        accordion(
            focus_ids.iter().cloned().enumerate().map(|(id, focus_id)| {
                accordion_item(id, focus_id, text("Header"), text("Content"))
            }),
            &AccordionState::Single(None),
            |_| (),
            &LIGHT,
        );
    drop(black_box(view));
}

#[test]
fn performance_contract_accordion_preallocates_section_storage() {
    const RENDERS: usize = 256;
    const ITEMS: usize = 64;

    let focus_ids = (0..ITEMS).map(|_| widget::Id::unique()).collect::<Vec<_>>();
    render(&focus_ids);
    let stats = clean_window((247_296, 55_883_776), || {
        for _ in 0..RENDERS {
            render(&focus_ids);
        }
    });

    eprintln!(
        "{RENDERS} accordion renders with {ITEMS} items: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 247_296, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 55_883_776, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
