#![cfg(feature = "input-otp")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::input_otp::{OtpPattern, input_otp};
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// 4-core runner that lands inside the region as +4 allocations. Code under
/// test that allocated would dirty *every* window; a one-time foreign block
/// dirties at most one. So the batch runs in its own window, up to
/// [`WINDOWS`] times, and the contract asks for one clean window rather than a
/// clean process.
const WINDOWS: usize = 4;

/// Runs `batch` in a fresh allocator window, up to [`WINDOWS`] times, and
/// returns the first window whose `(allocations, bytes_allocated)` equal
/// `expected` — or the last window's stats, when none did.
fn clean_window(expected: (usize, usize), mut batch: impl FnMut()) -> stats_alloc::Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if (stats.allocations, stats.bytes_allocated) == expected {
            break;
        }
    }
    stats
}

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
    let stats = clean_window((112_128, 24_888_320), || {
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
    assert_eq!(stats.bytes_allocated, 24_888_320, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 432_128, "{stats:?}");
}
