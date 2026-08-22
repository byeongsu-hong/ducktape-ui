#![cfg(feature = "message-scroller")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use iced::widget::Space;
use ui_lang_components::ui::message_scroller::{
    MessageScrollerState, controlled_message_scroller_with_end_content, message_scroller_item,
};
use ui_lang_components::ui::theme::LIGHT;

fn render(state: &MessageScrollerState) {
    let items = (0..256).map(|_| message_scroller_item("", Space::new()));
    let element: Element<'_, ()> = controlled_message_scroller_with_end_content(
        state,
        items,
        |_| (),
        |_, _| Space::new().into(),
        &LIGHT,
    )
    .into();
    black_box(element);
}

#[test]
fn performance_contract_message_scroller_reuses_row_buffer() {
    const RENDERS: usize = 64;
    let state = MessageScrollerState::new("allocation-contract");
    render(&state);

    let stats = clean_window((50_368, 4_590_208), || {
        for _ in 0..RENDERS {
            render(&state);
        }
    });

    eprintln!(
        "{RENDERS} message-scroller renders: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 50_368, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 4_590_208, "{stats:?}");
}
