#![cfg(feature = "modal")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use iced::advanced::widget;
use iced::widget::text;
use ui_lang_components::ui::modal::{DismissRules, FocusScope, modal};
use ui_lang_components::ui::theme::LIGHT;

fn build(focus: &FocusScope) -> Element<'static, ()> {
    modal(
        text("page"),
        true,
        text("dialog"),
        focus,
        DismissRules::DIALOG,
        |_| (),
        &LIGHT,
    )
}

#[test]
fn performance_contract_modal_shares_focus_order() {
    const BUILDS: usize = 4_000;
    let focus = FocusScope::new(widget::Id::new("first"), widget::Id::new("restore"))
        .push(widget::Id::new("second"));
    let element = build(&focus);
    assert_eq!(element.as_widget().children().len(), 2);
    drop(element);

    let stats = clean_window((12_000, 1_312_000), || {
        for _ in 0..BUILDS {
            drop(black_box(build(black_box(&focus))));
        }
    });

    eprintln!(
        "{BUILDS} modal builds: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.allocations, 12_000, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 1_312_000, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
