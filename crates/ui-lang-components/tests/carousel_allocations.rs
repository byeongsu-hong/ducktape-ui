#![cfg(feature = "carousel")]

mod common;

use common::clean_window_allocations;

use std::hint::black_box;

use iced::widget;
use ui_lang_components::ui::carousel::{
    CarouselBoundary, CarouselOrientation, CarouselState, carousel_indicators,
};
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::theme::LIGHT;

fn render(ids: &[widget::Id], state: CarouselState) {
    black_box(carousel_indicators(
        state,
        |index| ids[index].clone(),
        |index| index,
        CarouselOrientation::Horizontal,
        Direction::RightToLeft,
        &LIGHT,
    ));
}

#[test]
fn performance_contract_carousel_preallocates_indicator_storage() {
    const RENDERS: usize = 256;
    const SLIDES: usize = 64;
    let ids = (0..SLIDES)
        .map(|_| widget::Id::unique())
        .collect::<Vec<_>>();
    let state = CarouselState::new(0, SLIDES, CarouselBoundary::Bounded);

    render(&ids, state);
    let stats = clean_window_allocations(99_072, || {
        for _ in 0..RENDERS {
            render(&ids, state);
        }
    });

    eprintln!(
        "{RENDERS} carousel renders with {SLIDES} indicators: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
