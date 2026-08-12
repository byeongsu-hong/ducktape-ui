ui_lang::include_app!("tests/cases/ui/font_values.ice");

pub fn font_round_trip(value: iced::Font) -> iced::Font {
    value
}

pub fn family_round_trip(value: iced::font::Family) -> iced::font::Family {
    value
}

pub fn weight_round_trip(value: iced::font::Weight) -> iced::font::Weight {
    value
}

pub fn stretch_round_trip(value: iced::font::Stretch) -> iced::font::Stretch {
    value
}

pub fn style_round_trip(value: iced::font::Style) -> iced::font::Style {
    value
}

#[test]
fn preserves_every_native_font_operation() {
    let (mut app, _) = NativeFontValues::__boot();
    let _ = app.__update(__NativeFontValuesMessage::Inspect);

    let custom = iced::Font {
        family: iced::font::Family::Name("Display"),
        weight: iced::font::Weight::Bold,
        stretch: iced::font::Stretch::Expanded,
        style: iced::font::Style::Italic,
    };
    assert_eq!(app.default_font, iced::Font::default());
    assert_eq!(app.sans_font, iced::Font::DEFAULT);
    assert_eq!(app.monospace_font, iced::Font::MONOSPACE);
    assert_eq!(app.named_font, iced::Font::with_name("Inter"));
    assert_eq!(app.custom_font, custom);
    assert_eq!(app.returned_font, custom);

    assert_eq!(
        app.families_primary,
        vec![
            iced::font::Family::default(),
            iced::font::Family::Name("Inter"),
            iced::font::Family::Serif,
            iced::font::Family::SansSerif,
        ]
    );
    assert_eq!(
        app.families_secondary,
        vec![
            iced::font::Family::Cursive,
            iced::font::Family::Fantasy,
            iced::font::Family::Monospace,
        ]
    );
    assert_eq!(
        app.weights_light,
        vec![
            iced::font::Weight::default(),
            iced::font::Weight::Thin,
            iced::font::Weight::ExtraLight,
            iced::font::Weight::Light,
            iced::font::Weight::Normal,
        ]
    );
    assert_eq!(
        app.weights_heavy,
        vec![
            iced::font::Weight::Medium,
            iced::font::Weight::Semibold,
            iced::font::Weight::Bold,
            iced::font::Weight::ExtraBold,
            iced::font::Weight::Black,
        ]
    );
    assert_eq!(
        app.stretches_tight,
        vec![
            iced::font::Stretch::default(),
            iced::font::Stretch::UltraCondensed,
            iced::font::Stretch::ExtraCondensed,
        ]
    );
    assert_eq!(
        app.stretches_condensed,
        vec![
            iced::font::Stretch::Condensed,
            iced::font::Stretch::SemiCondensed,
        ]
    );
    assert_eq!(
        app.stretches_wide,
        vec![
            iced::font::Stretch::Normal,
            iced::font::Stretch::SemiExpanded,
            iced::font::Stretch::Expanded,
        ]
    );
    assert_eq!(
        app.stretches_expanded,
        vec![
            iced::font::Stretch::ExtraExpanded,
            iced::font::Stretch::UltraExpanded,
        ]
    );
    assert_eq!(
        app.styles,
        vec![
            iced::font::Style::default(),
            iced::font::Style::Normal,
            iced::font::Style::Italic,
            iced::font::Style::Oblique,
        ]
    );

    assert_eq!(app.returned_family, iced::font::Family::Name("Inter"));
    assert_eq!(app.returned_weight, iced::font::Weight::Bold);
    assert_eq!(app.returned_stretch, iced::font::Stretch::Expanded);
    assert_eq!(app.returned_style, iced::font::Style::Italic);
    assert_eq!(app.projected_family, custom.family);
    assert_eq!(app.projected_weight, custom.weight);
    assert_eq!(app.projected_stretch, custom.stretch);
    assert_eq!(app.projected_style, custom.style);
    assert_eq!(app.family_kind, "named");
    assert_eq!(app.family_name.as_deref(), Some("Inter"));
    assert_eq!(app.missing_name, None);
    assert_eq!(app.weight_kind, "bold");
    assert_eq!(app.stretch_kind, "expanded");
    assert_eq!(app.style_kind, "italic");
    assert!(app.fonts_equal);
    let _ = app.__view();
}
