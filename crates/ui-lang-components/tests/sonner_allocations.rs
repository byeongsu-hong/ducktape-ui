#![cfg(feature = "sonner")]

use std::alloc::System;
use std::hint::black_box;
use std::time::Duration;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::sonner::{SonnerState, ToastPlacement, sonner_with_content};
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn performance_contract_sonner_streams_visible_entries() {
    const RENDERS: usize = 4_000;
    const VISIBLE: usize = 32;
    let mut state = SonnerState::new(VISIBLE, ToastPlacement::TopRight);
    for index in 0..VISIBLE {
        state.show(format!("Toast {index}"), Duration::ZERO);
    }

    let region = Region::new(GLOBAL);
    for _ in 0..RENDERS {
        let view = sonner_with_content(
            &state,
            |_| (),
            |entry, _controls, _theme| Element::from(iced::widget::text(entry.data().title())),
            &LIGHT,
        );
        black_box(view);
    }
    let stats = region.change();

    eprintln!(
        "{RENDERS} Sonner renders: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 656_000, "{stats:?}");
    assert!(stats.reallocations <= 24_000, "{stats:?}");
    assert!(stats.bytes_allocated <= 38_976_000, "{stats:?}");
}
