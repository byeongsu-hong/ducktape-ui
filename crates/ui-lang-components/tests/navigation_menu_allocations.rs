#![cfg(feature = "navigation-menu")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::navigation_menu::{
    NavigationMenuEvent, NavigationMenuItem, NavigationMenuState, navigation_menu,
};
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn view(state: &NavigationMenuState) -> Element<'static, NavigationMenuEvent> {
    navigation_menu(
        "allocation-contract",
        (0..32).map(|index| NavigationMenuItem::link(format!("route-{index}"), "Route")),
        state,
        |event| event,
        &LIGHT,
    )
    .into()
}

#[test]
fn performance_contract_navigation_menu_activates_known_triggers_directly() {
    const RENDERS: usize = 128;
    let state = NavigationMenuState::default().active("active-route");
    drop(black_box(view(&state)));

    let region = Region::new(GLOBAL);
    for _ in 0..RENDERS {
        drop(black_box(view(black_box(&state))));
    }
    let stats = region.change();

    eprintln!(
        "{RENDERS} navigation menus: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 100_096, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
