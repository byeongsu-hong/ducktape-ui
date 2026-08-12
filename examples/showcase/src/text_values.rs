ui_lang::include_app!("tests/cases/ui/text_values.ice");

pub fn text_alignment_round_trip(
    value: iced::widget::text::Alignment,
) -> iced::widget::text::Alignment {
    value
}

pub fn text_shaping_round_trip(value: iced::widget::text::Shaping) -> iced::widget::text::Shaping {
    value
}

pub fn text_wrapping_round_trip(
    value: iced::widget::text::Wrapping,
) -> iced::widget::text::Wrapping {
    value
}

pub fn text_line_height_round_trip(
    value: iced::widget::text::LineHeight,
) -> iced::widget::text::LineHeight {
    value
}

#[test]
fn preserves_every_native_text_value_operation() {
    use iced::widget::text::{Alignment, LineHeight, Shaping, Wrapping};
    use std::hash::Hash;

    fn assert_hash<T: Hash>(_: T) {}

    let (mut app, _) = NativeTextValues::__boot();
    let _ = app.__update(__NativeTextValuesMessage::Inspect);

    assert_eq!(app.default_alignment, Alignment::default());
    assert_eq!(
        app.alignments,
        vec![
            Alignment::Left,
            Alignment::Center,
            Alignment::Right,
            Alignment::Justified,
        ]
    );
    assert_eq!(app.from_horizontal, Alignment::Center);
    assert_eq!(app.from_alignment, Alignment::Right);
    assert_eq!(app.horizontal, iced::alignment::Horizontal::Left);
    assert_eq!(app.returned_alignment, Alignment::Right);
    assert_eq!(app.alignment_kind, "right");

    assert_eq!(app.default_shaping, Shaping::default());
    assert_eq!(
        app.shapings,
        vec![Shaping::Auto, Shaping::Basic, Shaping::Advanced]
    );
    assert_eq!(app.returned_shaping, Shaping::Advanced);
    assert_eq!(app.shaping_kind, "advanced");

    assert_eq!(app.default_wrapping, Wrapping::default());
    assert_eq!(
        app.wrappings,
        vec![
            Wrapping::None,
            Wrapping::Word,
            Wrapping::Glyph,
            Wrapping::WordOrGlyph,
        ]
    );
    assert_eq!(app.returned_wrapping, Wrapping::Glyph);
    assert_eq!(app.wrapping_kind, "glyph");

    assert_eq!(app.default_line_height, LineHeight::default());
    assert_eq!(app.relative_height, LineHeight::Relative(1.5));
    assert_eq!(
        app.absolute_height,
        LineHeight::Absolute(iced::Pixels(24.0))
    );
    assert_eq!(app.from_f64, LineHeight::Relative(1.25));
    assert_eq!(app.from_pixels, LineHeight::Absolute(iced::Pixels(30.0)));
    assert_eq!(app.returned_line_height, LineHeight::Relative(1.5));
    assert_eq!(app.line_height_kind, "relative");
    assert_eq!(app.relative_value, Some(1.5));
    assert_eq!(app.absolute_value, Some(iced::Pixels(24.0)));
    assert_eq!(app.absolute_pixels, iced::Pixels(30.0));
    assert!(app.values_equal);

    assert_hash(app.returned_alignment);
    assert_hash(app.returned_shaping);
    assert_hash(app.returned_wrapping);
    assert_hash(app.returned_line_height);
    let _ = app.__view();
}
