#![cfg(feature = "tabs")]

mod common;

use common::GLOBAL;

use stats_alloc::Region;
use ui_lang_components::ui::tabs::{TabsEvent, TabsState};

#[test]
fn performance_contract_tabs_skip_equal_state_replacement() {
    const UPDATES: usize = 4_096;
    let mut state = TabsState::new("account".to_owned());
    let event = TabsEvent::Select("account".to_owned());

    assert!(!state.apply(&event));
    let region = Region::new(GLOBAL);
    for _ in 0..UPDATES {
        assert!(!state.apply(&event));
    }
    let stats = region.change();

    eprintln!(
        "{UPDATES} equal tabs state updates: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );

    assert_eq!(stats.allocations, 0, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 0, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
