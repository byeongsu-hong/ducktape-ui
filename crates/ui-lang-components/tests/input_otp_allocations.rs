#![cfg(feature = "input-otp")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::input_otp::{OtpPattern, input_otp};
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
#[ignore = "allocation contract; run alone with --test-threads=1"]
fn performance_contract_otp_render_streams_characters() {
    const RENDERS: usize = 4_000;

    let region = Region::new(GLOBAL);
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
    let stats = region.change();

    eprintln!(
        "{RENDERS} OTP renders: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert!(stats.allocations <= 172_000, "{stats:?}");
    assert!(stats.reallocations <= 12_000, "{stats:?}");
}
