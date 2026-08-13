#![cfg(feature = "tabs")]

use std::alloc::System;
use std::hint::black_box;

use iced::widget::{self, text};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::tabs::{
    TabsActivation, TabsOrientation, TabsState, TabsVariant, tab, tabs,
};
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn render(ids: &[widget::Id], state: &TabsState<usize>) {
    let items = ids.iter().enumerate().map(|(index, id)| {
        tab(
            index,
            id.clone(),
            text(format!("Tab {index}")),
            text(format!("Panel {index}")),
        )
    });
    black_box(tabs(
        state,
        items,
        TabsOrientation::Horizontal,
        TabsActivation::Automatic,
        TabsVariant::Default,
        |_| (),
        &LIGHT,
    ));
}

#[test]
fn performance_contract_tabs_reuse_trigger_storage() {
    const RENDERS: usize = 256;
    const TABS: usize = 64;
    let ids = (0..TABS).map(|_| widget::Id::unique()).collect::<Vec<_>>();
    let state = TabsState::new(0);

    render(&ids, &state);
    let region = Region::new(GLOBAL);
    for _ in 0..RENDERS {
        render(&ids, &state);
    }
    let stats = region.change();

    eprintln!(
        "{RENDERS} tab renders with {TABS} triggers: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 150_276, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 47_653_764, "{stats:?}");
}
