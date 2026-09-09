#[path = "../src/message_scroller_adapter.rs"]
mod message_scroller_adapter;

mod backend {
    pub use crate::message_scroller_adapter::{
        MessageScrollerTransition as Transition, message_scroller_apply as apply,
        message_scroller_effects as effects,
    };
    use iced::{Element, Length};
    pub use ui_lang_components::ui::message_scroller::{
        MessageScrollerEvent as Event, MessageScrollerState as State,
    };
    use ui_lang_components::ui::message_scroller::{
        MessageScrollerInitialPosition, MessageScrollerItemMeta, controlled_message_scroller,
        message_scroller_item,
    };

    pub fn initial() -> State {
        State::new("reading")
            .initial_position(MessageScrollerInitialPosition::Start)
            .scroll_previous_item_peek(0.0)
    }
    pub fn items(rows: Vec<i64>) -> Event {
        Event::ItemsChanged(
            rows.iter()
                .map(|id| MessageScrollerItemMeta::new(id.to_string()))
                .collect(),
        )
    }
    pub fn prepend(mut rows: Vec<i64>) -> Vec<i64> {
        rows.insert(0, rows[0] - 1);
        rows.pop();
        rows
    }
    pub fn remove_first(mut rows: Vec<i64>) -> Vec<i64> {
        rows.remove(0);
        rows.push(rows.last().expect("nonempty transcript") + 1);
        rows
    }
    pub fn remove_visible(mut rows: Vec<i64>, state: &State) -> Vec<i64> {
        if let Some(id) = state
            .visible_message_ids()
            .first()
            .and_then(|id| id.parse::<i64>().ok())
        {
            rows.retain(|row| *row != id);
            rows.push(rows.last().expect("nonempty transcript") + 2);
        }
        rows
    }
    pub fn transcript(rows: &[i64], state: &State) -> Element<'static, Event> {
        let items = rows.iter().map(|id| {
            let row = iced::widget::container(iced::widget::text(format!("Message {id}")))
                .height(if id % 2 == 0 { 40 } else { 24 })
                .width(Length::Fill);
            message_scroller_item(id.to_string(), row)
        });
        controlled_message_scroller(
            state,
            items,
            |event| event,
            &ui_lang_components::ui::theme::LIGHT,
        )
        .into()
    }
}

ui_lang::include_app!("tests/cases/ui/scroll_reading_anchor.ice");

#[test]
fn a_capped_transcript_keeps_the_reading_row() {
    use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton, Platform, WheelDelta};
    const HERE: Location =
        Location::new("scroll_reading_anchor.rs", 1, 1, "preserve the reading row");
    let mut driver = Driver::new(
        ScrollReadingAnchor::__program(),
        Config::new("a_capped_transcript_keeps_the_reading_row")
            .viewport(400.0, 320.0)
            .scale_factor(1.0)
            .locale("en-US")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    driver.move_to("ScrollReadingAnchor/page/root/transcript", HERE);
    driver.wheel_delta(WheelDelta::Pixels { x: 0.0, y: -160.0 }, HERE);
    let row = driver.target("ducktape-message-scroller:7:reading:item:1:6", HERE);
    assert!(row.visible());
    assert!((row.visible_height() - row.height()).abs() < 0.01);
    let before = row.visible_y();
    assert_eq!(driver.state().version, 0);
    driver.capture("reading_before", HERE);
    for (button, version, capture) in [
        ("prepend", 1, "reading_after_prepend"),
        ("remove", 2, "reading_after_remove"),
    ] {
        driver.click_with(
            &format!("ScrollReadingAnchor/page/root/{button}"),
            MouseButton::Left,
            1,
            HERE,
        );
        assert_eq!(driver.state().version, version);
        let row = driver.target("ducktape-message-scroller:7:reading:item:1:6", HERE);
        assert!(row.visible(), "{button} hid the reading row");
        assert!((row.visible_height() - row.height()).abs() < 0.01);
        assert!(
            (row.visible_y() - before).abs() < 0.01,
            "{button} moved the reading row from {before} to {}",
            row.visible_y()
        );
        driver.capture(capture, HERE);
    }
}

#[test]
fn deleting_the_visible_anchor_keeps_the_next_row() {
    use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton, Platform, WheelDelta};
    const HERE: Location = Location::new("scroll_reading_anchor.rs", 1, 1, "delete visible anchor");
    let mut driver = Driver::new(
        ScrollReadingAnchor::__program(),
        Config::new("deleting_the_visible_anchor_keeps_the_next_row")
            .viewport(400.0, 320.0)
            .scale_factor(1.0)
            .locale("en-US")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    driver.move_to("ScrollReadingAnchor/page/root/transcript", HERE);
    driver.wheel_delta(WheelDelta::Pixels { x: 0.0, y: -160.0 }, HERE);
    assert_eq!(
        driver
            .state()
            .scroller
            .visible_message_ids()
            .first()
            .map(String::as_str),
        Some("4")
    );
    let row = driver.target("ducktape-message-scroller:7:reading:item:1:5", HERE);
    assert!((row.visible_height() - row.height()).abs() < 0.01);
    let before = row.visible_y();
    driver.capture("before_anchor_deletion", HERE);
    driver.click_with(
        "ScrollReadingAnchor/page/root/delete-visible",
        MouseButton::Left,
        1,
        HERE,
    );
    assert!(!driver.state().rows.contains(&4));
    assert_eq!(driver.state().rows.len(), 12);
    let row = driver.target("ducktape-message-scroller:7:reading:item:1:5", HERE);
    assert!(row.visible(), "deleting the anchor hid the next row");
    assert!((row.visible_height() - row.height()).abs() < 0.01);
    assert!(
        (row.visible_y() - before).abs() < 0.01,
        "deleted anchor moved the next row from {before} to {}",
        row.visible_y()
    );
    driver.capture("after_anchor_deletion", HERE);
}
