#![cfg(feature = "input-otp")]

mod common;

use common::{clean_window, clean_window_allocations};

use std::hint::black_box;

use iced::Element;
use ui_lang_components::ui::input_otp::{OtpPattern, input_otp};
use ui_lang_components::ui::theme::LIGHT;

#[test]
#[ignore = "allocation contract; run alone with --test-threads=1"]
fn performance_contract_otp_render_streams_characters() {
    const RENDERS: usize = 4_000;

    // Measured through the shared window policy: this contract sorts first
    // in its binary, and libtest's main-thread setup landed +2 allocations in
    // a bare region on the CI runner.
    let stats = clean_window_allocations(172_000, || {
        for _ in 0..RENDERS {
            let element: Element<'_, ()> = input_otp(
                "한2三4A6",
                6,
                OtpPattern::Custom(char::is_alphanumeric),
                |_| (),
                &LIGHT,
            )
            .into();
            black_box(element);
        }
    });

    eprintln!(
        "{RENDERS} OTP renders: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 172_000, "{stats:?}");
    assert!(stats.reallocations <= 12_000, "{stats:?}");
}

fn render() {
    let element: Element<'_, ()> = input_otp(
        "1234567812345678123456781234567812345678123456781234567812345678",
        64,
        OtpPattern::Digits,
        |_| (),
        &LIGHT,
    )
    .groups([4; 16])
    .into();
    drop(black_box(element));
}

#[test]
fn performance_contract_otp_preallocates_slot_storage() {
    const RENDERS: usize = 256;

    render();
    let stats = clean_window((112_128, 10_667_008), || {
        for _ in 0..RENDERS {
            render();
        }
    });

    eprintln!(
        "{RENDERS} grouped 64-slot OTP renders: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 112_128, "{stats:?}");
    assert_eq!(stats.reallocations, 2_816, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 10_667_008, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 432_128, "{stats:?}");
}
