#![cfg(feature = "sonner")]

mod common;

use common::clean_window;

use std::hint::black_box;
use std::time::Duration;

use iced::Element;
use ui_lang_components::ui::sonner::{SonnerState, ToastPlacement, sonner_with_content};
use ui_lang_components::ui::theme::LIGHT;

fn render(state: &SonnerState) {
    black_box(sonner_with_content(
        state,
        |_| (),
        |entry, _controls, _theme| Element::from(iced::widget::text(entry.data().title())),
        &LIGHT,
    ));
}

#[test]
fn performance_contract_sonner_preallocates_visible_entries() {
    const RENDERS: usize = 256;
    const VISIBLE: usize = 32;
    let mut state = SonnerState::new(VISIBLE, ToastPlacement::TopRight);
    for index in 0..VISIBLE {
        state.show(format!("Toast {index}"), Duration::ZERO);
    }

    for _ in 0..RENDERS {
        render(&state);
    }

    let stats = clean_window((41_984, 2_494_464), || {
        for _ in 0..RENDERS {
            render(&state);
        }
    });

    eprintln!(
        "{RENDERS} Sonner renders: {} allocations / {} reallocations / {} bytes / \
         {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 41_984, "{stats:?}");
    assert_eq!(stats.reallocations, 768, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 2_494_464, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 229_376, "{stats:?}");
}
