#![cfg(feature = "select")]

mod common;

use common::{WINDOWS, clean_window};

use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use ui_lang_components::ui::menu::MenuState;
use ui_lang_components::ui::select::{SelectGroup, SelectIds, SelectOption, select};
use ui_lang_components::ui::theme::LIGHT;

const OPTIONS: usize = 100;

fn groups() -> Vec<SelectGroup<usize>> {
    vec![SelectGroup::new(
        "commands",
        (0..OPTIONS)
            .map(|index| SelectOption::new(format!("option-{index}"), index, "Option"))
            .collect(),
    )]
}

fn build<'a>(groups: Vec<SelectGroup<usize>>, state: &'a MenuState) -> Element<'a, ()> {
    select(
        SelectIds::new("allocation-contract"),
        groups,
        None,
        String::new(),
        state,
        false,
        |_| (),
        &LIGHT,
    )
    .trigger(text("Select"))
    .into()
}

#[test]
fn performance_contract_select_reuses_owned_groups() {
    let state = MenuState::default();
    drop(build(groups(), &state));

    let mut pending: Vec<_> = (0..WINDOWS).map(|_| groups()).collect();
    let mut element = None;
    let stats = clean_window((23_428, 1_370_158), || {
        element = Some(build(black_box(pending.pop().unwrap()), black_box(&state)));
    });
    let element = element.unwrap();

    assert_eq!(element.as_widget().children().len(), 2);
    eprintln!(
        "{OPTIONS} select options: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.allocations, 23_428, "{stats:?}");
    assert_eq!(stats.reallocations, 11_314, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 1_370_158, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 750_290, "{stats:?}");
}
