#![cfg(feature = "sheet")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::sheet::sheet_panel;
use ui_lang_components::ui::theme::LIGHT;

fn view(direction: Direction) -> Element<'static, ()> {
    sheet_panel(text("Body"), &LIGHT)
        .direction(direction)
        .header(text("Settings"))
        .close(text("Close"))
        .into_widget()
        .into()
}

#[test]
fn performance_contract_directed_sheet_headers_reuse_exact_storage() {
    const RENDERS: usize = 1_024;

    drop(black_box(view(Direction::LeftToRight)));
    drop(black_box(view(Direction::RightToLeft)));

    let stats = clean_window((18_432, 1_589_248), || {
        for _ in 0..RENDERS {
            drop(black_box(view(Direction::LeftToRight)));
            drop(black_box(view(Direction::RightToLeft)));
        }
    });

    eprintln!(
        "{RENDERS} LTR/RTL sheet header pairs: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 18_432, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 1_589_248, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
