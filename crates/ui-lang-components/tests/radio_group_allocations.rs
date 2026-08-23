#![cfg(feature = "radio-group")]

mod common;

use common::clean_window_allocations;

use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use iced::Element;
use ui_lang_components::ui::radio_group::{radio_group, radio_option};
use ui_lang_components::ui::theme::LIGHT;

static VALUE_CLONES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
struct Value(usize);

impl Clone for Value {
    fn clone(&self) -> Self {
        VALUE_CLONES.fetch_add(1, Ordering::Relaxed);
        Self(self.0)
    }
}

fn group(option_count: usize) -> Element<'static, Value> {
    radio_group(
        "allocation-contract",
        (0..option_count).map(|index| radio_option(Value(index), "Option", &LIGHT)),
        None,
        |value| value,
        &LIGHT,
    )
    .into()
}

#[test]
fn radio_group_build_shares_keyboard_snapshots() {
    const OPTIONS: usize = 8;
    const ALLOCATION_BUDGET: usize = 119;

    drop(black_box(group(OPTIONS)));

    let stats = clean_window_allocations(ALLOCATION_BUDGET, || {
        VALUE_CLONES.store(0, Ordering::Relaxed);
        drop(black_box(group(OPTIONS)));
    });
    let clones = VALUE_CLONES.load(Ordering::Relaxed);

    eprintln!(
        "{OPTIONS} radio options: {clones} value clones, {} allocations, {} bytes",
        stats.allocations, stats.bytes_allocated
    );
    assert!(
        stats.allocations <= ALLOCATION_BUDGET,
        "each key handler should share the group snapshots: {stats:?}"
    );
    assert!(
        stats.bytes_allocated <= 10_227,
        "a style closure should capture tokens, not a whole theme: {stats:?}"
    );
    assert_eq!(clones, OPTIONS, "each value should be snapshotted once");
}
