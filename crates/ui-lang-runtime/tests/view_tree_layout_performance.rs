//! What a view module's frame costs the window thread.
//!
//! A guest's tree is laid out and shaped on the window thread every redraw,
//! and shaping is paid per character, not per node. Every frame here is the
//! frame a hostile guest would send — 8 MiB of text, in the two shapes the
//! wire allows: a few full-length strings, and a full screen of shorter
//! ones — put through `wire::sanitize` exactly as the host puts it. What is
//! measured is therefore what `MAX_TEXT_BYTES_PER_FRAME` leaves of it;
//! without that bound the same frames took 6.2 s and 4.4 s here.
//!
//! The budgets are wide, because the regression they guard is not subtle:
//! the bound going missing is two orders of magnitude, not a few percent.

#![cfg(not(debug_assertions))]

mod common;

use std::time::Instant;

use common::assert_wall_clock_budgets;
use iced::advanced::renderer;
use iced::{Font, Pixels, Size, Theme, mouse};
use iced_test::runtime::UserInterface;
use iced_test::runtime::user_interface;
use ui_lang_runtime::view_tree::{self, Inputs};
use ui_lang_wire as wire;

fn text_node(nth: usize, content: String) -> wire::Node {
    wire::Node::Text {
        key: format!("App/line/{nth}"),
        content,
        size: None,
        color: None,
        font: wire::Font::default(),
        width: None,
        align_x: None,
    }
}

/// A frame of `count` text nodes of `bytes` bytes each, sanitized exactly as
/// the host sanitizes what a module sends.
fn frame_of_texts(count: usize, filler: &str, bytes: usize) -> wire::Node {
    let mut content = String::new();
    while content.len() < bytes {
        content.push_str(filler);
    }
    let mut frame = wire::Frame {
        root: Some(wire::Node::Linear {
            key: "App/lines".into(),
            axis: wire::Axis::Column,
            spacing: None,
            padding: None,
            width: None,
            height: None,
            align: None,
            children: (0..count)
                .map(|nth| text_node(nth, content.clone()))
                .collect(),
        }),
        ..wire::Frame::default()
    };
    wire::sanitize(&mut frame);
    frame.root.expect("sanitized root")
}

fn renderer() -> iced_test::renderer::Renderer {
    iced_test::futures::futures::executor::block_on(
        <iced_test::renderer::Renderer as renderer::Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            None,
        ),
    )
    .expect("headless renderer")
}

/// One layout of the tree, from a cold cache each time: a guest that changes
/// a character makes the host reshape, so the contract measures the frame it
/// can force rather than the one iced can skip.
fn layout_frames(root: &wire::Node, samples: usize) -> Vec<u128> {
    let inputs = Inputs::default();
    let mut renderer = renderer();
    (0..samples)
        .map(|_| {
            let element = view_tree::render(root, &inputs);
            let started = Instant::now();
            let mut ui = UserInterface::build(
                element,
                Size::new(1280.0, 800.0),
                user_interface::Cache::default(),
                &mut renderer,
            );
            ui.draw(
                &mut renderer,
                &Theme::Light,
                &renderer::Style::default(),
                mouse::Cursor::Unavailable,
            );
            started.elapsed().as_micros()
        })
        .collect()
}

const ASCII: &str = "the quick brown fox jumps over the lazy dog 0123456789 ";
const MIXED: &str = "the quick 다람쥐 헌 쳇바퀴에 타고파 漢字混じり文 🦆🔥 ";

#[test]
fn a_hundred_and_twenty_eight_long_texts_lay_out_in_a_frame() {
    // 69 ms and 26 MiB locally; 6.2 s and 3.3 GiB with no text bound.
    const P50_BUDGET_US: u128 = 500_000;
    const P95_BUDGET_US: u128 = 1_500_000;

    let root = frame_of_texts(128, ASCII, 64 << 10);
    let (p50, p95) = assert_wall_clock_budgets(
        "128x64KiB ascii",
        layout_frames(&root, 8),
        P50_BUDGET_US,
        P95_BUDGET_US,
        || layout_frames(&root, 8),
    );
    eprintln!("128x64KiB ascii: p50={p50}us p95={p95}us");
}

#[test]
fn a_hundred_and_twenty_eight_long_mixed_script_texts_lay_out_in_a_frame() {
    // 1.6 s locally against 69 ms of the same bytes in Latin: what the
    // budget bounds is bytes, and a byte of Hangul, Han or emoji costs
    // some twenty of a byte of ASCII to shape. Fewer samples than the
    // others because each one is seconds.
    const P50_BUDGET_US: u128 = 25_000_000;
    const P95_BUDGET_US: u128 = 30_000_000;

    let root = frame_of_texts(128, MIXED, 64 << 10);
    let (p50, p95) = assert_wall_clock_budgets(
        "128x64KiB mixed",
        layout_frames(&root, 4),
        P50_BUDGET_US,
        P95_BUDGET_US,
        || layout_frames(&root, 4),
    );
    eprintln!("128x64KiB mixed: p50={p50}us p95={p95}us");
}

#[test]
fn a_full_screen_of_kilobyte_texts_lays_out_in_a_frame() {
    // 97 ms locally, nearly all of it the node count rather than the text:
    // the same frame took 4.4 s and 3.4 GiB with no text bound.
    const P50_BUDGET_US: u128 = 1_500_000;
    const P95_BUDGET_US: u128 = 2_500_000;

    let root = frame_of_texts(wire::MAX_NODES - 1, ASCII, 1024);
    let (p50, p95) = assert_wall_clock_budgets(
        "8191x1KiB ascii",
        layout_frames(&root, 8),
        P50_BUDGET_US,
        P95_BUDGET_US,
        || layout_frames(&root, 8),
    );
    eprintln!("8191x1KiB ascii: p50={p50}us p95={p95}us");
}
