#![cfg(feature = "toggle-group")]

mod common;

use common::clean_window;

use std::hint::black_box;

use iced::Element;
use iced::widget::{self, text};
use ui_lang_components::ui::theme::LIGHT;
use ui_lang_components::ui::toggle_group::{
    ToggleGroupOrientation, ToggleGroupState, toggle_group, toggle_group_item,
};

fn render(
    ids: &[widget::Id],
    state: &ToggleGroupState<usize>,
    orientation: ToggleGroupOrientation,
) {
    let items = ids
        .iter()
        .enumerate()
        .map(|(index, id)| toggle_group_item(id.clone(), index, text("Toggle"), ()));
    let group: Element<'_, ()> = toggle_group(items, state, orientation, |_| (), &LIGHT).into();
    drop(black_box(group));
}

#[test]
fn performance_contract_toggle_group_reuses_control_storage() {
    const RENDERS: usize = 128;
    const ITEMS: usize = 64;
    let ids = (0..ITEMS).map(|_| widget::Id::unique()).collect::<Vec<_>>();
    let state = ToggleGroupState::Single(Some(0));
    let orientations = [
        ToggleGroupOrientation::Horizontal,
        ToggleGroupOrientation::Vertical,
    ];

    for orientation in orientations {
        render(&ids, &state, orientation);
    }
    let stats = clean_window((132_096, 45_130_752), || {
        for _ in 0..RENDERS {
            for orientation in orientations {
                render(&ids, &state, orientation);
            }
        }
    });

    eprintln!(
        "{RENDERS} horizontal and vertical toggle groups with {ITEMS} controls: \
         {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 132_096, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 45_130_752, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
