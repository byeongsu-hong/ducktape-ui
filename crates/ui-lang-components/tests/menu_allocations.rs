#![cfg(feature = "menu")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use ui_lang_components::ui::menu::{MenuEntry, MenuState, menu};
use ui_lang_components::ui::theme::LIGHT;

fn render(entries: &[MenuEntry], state: &MenuState) {
    let menu: Element<'_, ()> = menu("allocation-contract", entries, state, |_| (), &LIGHT).into();
    drop(black_box(menu));
}

#[test]
fn performance_contract_menu_reuses_child_storage() {
    const RENDERS: usize = 128;
    const ITEMS: usize = 64;
    let entries = (0..ITEMS)
        .map(|index| MenuEntry::item(format!("item-{index}"), "Item"))
        .collect::<Vec<_>>();
    let state = MenuState::initial(&entries);

    render(&entries, &state);
    // The scrollable wrapper and stable scroll ID add three allocations and
    // 1,332 bytes per menu, independent of item count. Child-storage
    // reallocations remain unchanged.
    let stats = clean_window((1_279_488, 52_478_336), || {
        for _ in 0..RENDERS {
            render(&entries, &state);
        }
    });

    eprintln!(
        "{RENDERS} menus with {ITEMS} items: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 1_279_488, "{stats:?}");
    assert_eq!(stats.reallocations, 74_752, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 52_478_336, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 20_307_968, "{stats:?}");
}
