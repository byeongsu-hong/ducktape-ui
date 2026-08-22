#![cfg(feature = "button-group")]

mod common;

use common::clean_window_allocations;

use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use ui_lang_components::ui::button_group::{ButtonGroupOrientation, button_group};
use ui_lang_components::ui::theme::LIGHT;

fn child() -> Element<'static, ()> {
    text("Child").into()
}

fn render(children: usize) {
    black_box(button_group(
        (0..children).map(|_| child()),
        ButtonGroupOrientation::Horizontal,
        &LIGHT,
    ));
}

#[test]
fn performance_contract_button_group_streams_child_storage() {
    const RENDERS: usize = 256;
    const CHILDREN: usize = 64;

    render(CHILDREN);
    let stats = clean_window_allocations(17_152, || {
        for _ in 0..RENDERS {
            render(CHILDREN);
        }
    });

    eprintln!(
        "{RENDERS} button group renders with {CHILDREN} children: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 17_152, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
