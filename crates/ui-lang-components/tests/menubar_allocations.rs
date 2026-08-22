#![cfg(feature = "menubar")]

mod common;

use common::GLOBAL;

use std::hint::black_box;

use iced::Element;
use stats_alloc::Region;
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::menu::MenuState;
use ui_lang_components::ui::menubar::{MenubarMenu, MenubarState, menubar};
use ui_lang_components::ui::theme::LIGHT;

fn render(menus: &[MenubarMenu], state: &MenubarState, menu_state: &MenuState) {
    let element: Element<'_, ()> = menubar(
        "allocation-contract",
        menus.iter().cloned(),
        state,
        menu_state,
        |_| (),
        &LIGHT,
    )
    .direction(Direction::RightToLeft)
    .into();
    black_box(element);
}

#[test]
fn performance_contract_menubar_reverses_trigger_storage_in_place() {
    const MENUS: usize = 64;
    const RENDERS: usize = 256;
    let menus = (0..MENUS)
        .map(|index| MenubarMenu::new(format!("menu-{index}"), format!("Menu {index}"), vec![]))
        .collect::<Vec<_>>();
    let state = MenubarState::initial(&menus);
    let menu_state = MenuState::default();

    render(&menus, &state, &menu_state);
    let region = Region::new(GLOBAL);
    for _ in 0..RENDERS {
        render(&menus, &state, &menu_state);
    }
    let stats = region.change();

    eprintln!(
        "{RENDERS} RTL menubar renders with {MENUS} triggers: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 167_426, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 47_547_792, "{stats:?}");
}
