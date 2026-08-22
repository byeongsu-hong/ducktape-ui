#![cfg(feature = "sidebar")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::sidebar::sidebar_menu_button_content;
use ui_lang_components::ui::theme::LIGHT;

fn render() {
    let content: Element<'_, ()> = sidebar_menu_button_content(
        Some(text("01").into()),
        "Overview",
        Some(text("LIVE").into()),
        false,
        Direction::LeftToRight,
        &LIGHT,
    );
    drop(black_box(content));
}

#[test]
fn performance_contract_sidebar_streams_menu_button_content() {
    const RENDERS: usize = 4_096;

    render();
    let stats = clean_window((24_576, 2_326_528), || {
        for _ in 0..RENDERS {
            render();
        }
    });

    eprintln!(
        "{RENDERS} expanded sidebar menu rows: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 24_576, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 2_326_528, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
