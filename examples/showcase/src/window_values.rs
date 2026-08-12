ui_lang::include_app!("tests/cases/ui/window_values.ice");

pub fn direction_round_trip(value: iced::window::Direction) -> iced::window::Direction {
    value
}

pub fn level_round_trip(value: iced::window::Level) -> iced::window::Level {
    value
}

pub fn mode_round_trip(value: iced::window::Mode) -> iced::window::Mode {
    value
}

pub fn attention_round_trip(value: iced::window::UserAttention) -> iced::window::UserAttention {
    value
}

#[test]
fn preserves_every_native_window_value() {
    use iced::window::{Direction, Level, Mode, UserAttention};

    let (mut app, _) = NativeWindowValues::__boot();
    let _ = app.__update(__NativeWindowValuesMessage::Inspect);

    assert!(matches!(
        app.cardinal.as_slice(),
        [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West
        ]
    ));
    assert!(matches!(
        app.diagonal_north.as_slice(),
        [Direction::NorthEast, Direction::NorthWest]
    ));
    assert!(matches!(
        app.diagonal_south.as_slice(),
        [Direction::SouthEast, Direction::SouthWest]
    ));
    assert_eq!(app.default_levels, vec![Level::default(), Level::Normal]);
    assert_eq!(
        app.stacked_levels,
        vec![Level::AlwaysOnBottom, Level::AlwaysOnTop]
    );
    assert_eq!(
        app.modes,
        vec![Mode::Windowed, Mode::Fullscreen, Mode::Hidden]
    );
    assert!(matches!(
        app.attentions.as_slice(),
        [UserAttention::Critical, UserAttention::Informational]
    ));
    assert!(matches!(app.returned_direction, Direction::SouthWest));
    assert_eq!(app.returned_level, Level::AlwaysOnTop);
    assert_eq!(app.returned_mode, Mode::Fullscreen);
    assert!(matches!(
        app.returned_attention,
        UserAttention::Informational
    ));
    assert_eq!(app.direction_kind, "south-west");
    assert_eq!(app.level_kind, "always-on-top");
    assert_eq!(app.mode_kind, "fullscreen");
    assert_eq!(app.attention_kind, "informational");
    assert!(app.levels_equal);
    assert!(app.modes_equal);
    let _ = app.__view();
}
