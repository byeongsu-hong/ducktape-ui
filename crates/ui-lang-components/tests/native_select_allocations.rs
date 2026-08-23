#![cfg(feature = "native-select")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::widget;
use ui_lang_components::ui::native_select::native_select_with_id;
use ui_lang_components::ui::theme::LIGHT;

fn render(options: &[usize], id: &widget::Id) {
    black_box(native_select_with_id(
        id.clone(),
        options,
        None::<&usize>,
        |value| value,
        &LIGHT,
    ));
}

#[test]
fn performance_contract_native_select_clones_options_directly() {
    const RENDERS: usize = 4_096;
    const OPTIONS: usize = 64;
    let options = (0..OPTIONS).collect::<Vec<_>>();
    let id = widget::Id::new("native-select-allocation-contract");

    render(&options, &id);
    let stats = clean_window((RENDERS * 5, RENDERS * 1_168), || {
        for _ in 0..RENDERS {
            render(&options, &id);
        }
    });

    eprintln!(
        "{RENDERS} native select renders with {OPTIONS} options: {} allocations / \
         {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );

    assert_eq!(stats.allocations, RENDERS * 5, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, RENDERS * 1_168, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
