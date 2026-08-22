#![cfg(feature = "navigation-menu")]

mod common;

use common::clean_window_allocations as clean_window;

use std::hint::black_box;

use iced::Element;
use stats_alloc::Stats;
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::navigation_menu::{
    NavigationMenuEvent, NavigationMenuItem, NavigationMenuState, navigation_menu,
};
use ui_lang_components::ui::theme::LIGHT;

fn view(
    state: &NavigationMenuState,
    direction: Direction,
) -> Element<'static, NavigationMenuEvent> {
    navigation_menu(
        "allocation-contract",
        (0..32).map(|index| NavigationMenuItem::link(format!("route-{index}"), "Route")),
        state,
        |event| event,
        &LIGHT,
    )
    .direction(direction)
    .into()
}

fn render_stats(state: &NavigationMenuState, direction: Direction) -> Stats {
    const RENDERS: usize = 128;
    clean_window(100_096, || {
        for _ in 0..RENDERS {
            drop(black_box(view(black_box(state), direction)));
        }
    })
}

#[test]
fn performance_contract_navigation_menu_reuses_rtl_trigger_storage() {
    let state = NavigationMenuState::default().active("active-route");
    for direction in [Direction::LeftToRight, Direction::RightToLeft] {
        drop(black_box(view(&state, direction)));
    }
    let ltr = render_stats(&state, Direction::LeftToRight);
    let rtl = render_stats(&state, Direction::RightToLeft);

    eprintln!(
        "128 navigation menus: LTR {} allocations / {} reallocations / {} bytes; \
         RTL {} allocations / {} reallocations / {} bytes",
        ltr.allocations,
        ltr.reallocations,
        ltr.bytes_allocated,
        rtl.allocations,
        rtl.reallocations,
        rtl.bytes_allocated,
    );
    assert_eq!(rtl, ltr, "LTR={ltr:?}, RTL={rtl:?}");
    assert!(ltr.allocations <= 100_096, "{ltr:?}");
    assert_eq!(ltr.reallocations, 0, "{ltr:?}");
}
