//! Native reading-position contracts for the real chat composition.

use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton, Platform, WheelDelta};

use crate::{__AiChatMessage, AiChat, codex};

const HERE: Location = Location::new("content_workspace.rs", 1, 1, "read while streaming");
const TRANSCRIPT: &str = "AiChat/shell/app/transcript";

#[test]
fn a_background_row_update_keeps_the_reading_position() {
    let mut driver = Driver::new(
        AiChat::__program(),
        Config::new("background_row_reading_position")
            .preset("streaming")
            .viewport(920.0, 600.0)
            .scale_factor(1.0)
            .locale("en-US")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    let template = codex::sample_answer().remove(0);
    let rows: Vec<_> = (1..=12)
        .map(|id| codex::Entry {
            id,
            ..template.clone()
        })
        .collect();
    driver.dispatch(__AiChatMessage::Rows(rows.clone()), HERE);
    driver.move_to(TRANSCRIPT, HERE);
    driver.wheel_delta(WheelDelta::Pixels { x: 0.0, y: 350.0 }, HERE);
    assert!(!driver.state().following);
    let before = driver.target(TRANSCRIPT, HERE).scroll_y();
    assert!(
        before > 100.0,
        "the reader must be away from the latest reply"
    );
    driver.dispatch(__AiChatMessage::Rows(rows), HERE);
    let after = driver.target(TRANSCRIPT, HERE).scroll_y();
    assert!(
        (after - before).abs() < 0.5,
        "background rows moved reading offset from {before} to {after}"
    );
}

#[test]
fn streamed_text_keeps_a_visible_history_row_still() {
    let mut driver = Driver::new(
        AiChat::__program(),
        Config::new("streamed_text_reading_position")
            .preset("streaming")
            .viewport(920.0, 600.0)
            .scale_factor(1.0)
            .locale("en-US")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    let template = codex::sample_answer().remove(0);
    let rows = (1..=12)
        .map(|id| codex::Entry {
            id,
            ..template.clone()
        })
        .collect();
    driver.dispatch(__AiChatMessage::Rows(rows), HERE);
    driver.move_to(TRANSCRIPT, HERE);
    driver.wheel_delta(WheelDelta::Pixels { x: 0.0, y: 350.0 }, HERE);
    let row_id = "AiChat/shell/app/transcript/rows/key(8)/prompt(8)/root";
    let row = driver.target(row_id, HERE);
    assert!(row.visible(), "establish a visible reading row");
    let before = row.visible_y();
    driver.dispatch(
        __AiChatMessage::Streamed(codex::Chunk {
            answer: "\n\nAnother paragraph arrived while the earlier answer was being read.\n\n"
                .repeat(8),
            thinking: String::new(),
            thinking_ended: false,
            status: "Responding".into(),
        }),
        HERE,
    );
    let row = driver.target(row_id, HERE);
    assert!(row.visible(), "streaming hid the reading row");
    assert!(
        (row.visible_y() - before).abs() < 0.5,
        "streaming moved row from {before} to {}",
        row.visible_y()
    );
}

#[test]
fn latest_resumes_following_through_streaming_and_settlement() {
    let mut driver = Driver::new(
        AiChat::__program(),
        Config::new("resume_latest")
            .preset("streaming")
            .viewport(920.0, 600.0)
            .scale_factor(1.0)
            .locale("en-US")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    let append = || {
        __AiChatMessage::Streamed(codex::Chunk {
            answer: "\n\nA streamed paragraph with an explicit reading policy.\n\n".repeat(10),
            thinking: String::new(),
            thinking_ended: false,
            status: "Responding".into(),
        })
    };
    driver.dispatch(append(), HERE);
    assert!(
        driver.state().following,
        "a reader at the tail follows streamed growth"
    );
    let live = "AiChat/shell/app/transcript/live/live-body";
    assert!(
        driver.target(live, HERE).bottom() - driver.target(TRANSCRIPT, HERE).scroll_y()
            <= driver.target(TRANSCRIPT, HERE).bottom()
    );
    driver.move_to(TRANSCRIPT, HERE);
    driver.wheel_delta(WheelDelta::Pixels { x: 0.0, y: 150.0 }, HERE);
    assert!(!driver.state().following);
    driver.click_with("AiChat/shell/app/header/latest", MouseButton::Left, 1, HERE);
    assert!(driver.state().following);
    driver.dispatch(append(), HERE);
    assert!(driver.state().following);
    assert!(
        driver.target(live, HERE).bottom() - driver.target(TRANSCRIPT, HERE).scroll_y()
            <= driver.target(TRANSCRIPT, HERE).bottom()
    );
    let mut rows = codex::sample_answer();
    rows[1].body =
        "A settled paragraph with final details.\n\n".repeat(120) + "Final response marker";
    driver.dispatch(__AiChatMessage::Settled(rows), HERE);
    let last = driver.target(
        "AiChat/shell/app/transcript/rows/key(-31)/answer(-31)/root",
        HERE,
    );
    assert!(
        last.bottom() - driver.target(TRANSCRIPT, HERE).scroll_y()
            <= driver.target(TRANSCRIPT, HERE).bottom(),
        "settlement must keep the end visible"
    );
    driver.check_text("Final response marker", Some(TRANSCRIPT), false, HERE);
    driver.capture("latest_after_settlement", HERE);
}
