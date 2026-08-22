#![cfg(feature = "radio-group")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use ui_lang_components::ui::radio_group::{radio_group, radio_option};
use ui_lang_components::ui::theme::LIGHT;

fn render(option_count: usize) {
    let element: Element<'static, usize> = radio_group(
        "storage-contract",
        (0..option_count).map(|value| radio_option(value, "Option", &LIGHT)),
        None,
        |value| value,
        &LIGHT,
    )
    .into();
    drop(black_box(element));
}

#[test]
fn performance_contract_radio_group_preallocates_child_storage() {
    const RENDERS: usize = 256;
    const OPTIONS: usize = 64;

    render(OPTIONS);
    let stats = clean_window((231_168, 71_026_688), || {
        for _ in 0..RENDERS {
            render(OPTIONS);
        }
    });

    eprintln!(
        "{RENDERS} radio group renders with {OPTIONS} options: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 231_168, "{stats:?}");
    assert!(stats.reallocations <= 16_384, "{stats:?}");
    assert!(stats.bytes_allocated <= 71_026_688, "{stats:?}");
    assert!(stats.bytes_reallocated <= 524_288, "{stats:?}");
}
