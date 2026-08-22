#![cfg(feature = "breadcrumb")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use ui_lang_components::ui::breadcrumb::{BreadcrumbItem, breadcrumb};
use ui_lang_components::ui::theme::LIGHT;

fn render<const ITEMS: usize>() {
    let row: Element<'_, ()> = breadcrumb(
        (0..ITEMS).map(|index| {
            if index + 1 == ITEMS {
                BreadcrumbItem::current(text("Current"))
            } else {
                BreadcrumbItem::link(text("Ancestor"))
            }
        }),
        || text("/").into(),
        &LIGHT,
    )
    .into();
    drop(black_box(row));
}

#[test]
fn performance_contract_breadcrumb_preallocates_row_storage() {
    const ITEMS: usize = 64;
    const RENDERS: usize = 256;

    render::<ITEMS>();
    let stats = clean_window((98_048, 8_079_360), || {
        for _ in 0..RENDERS {
            render::<ITEMS>();
        }
    });

    eprintln!(
        "{RENDERS} breadcrumbs with {ITEMS} items: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 98_048, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 8_079_360, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
