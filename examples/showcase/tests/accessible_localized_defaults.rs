mod backend {
    pub fn directed_actions(rtl: bool) -> iced::Element<'static, i64> {
        use ui_lang_components::ui::direction::{Direction, directed_row};
        use ui_lang_runtime::{Role, StableId, accessible};
        let action = |id, label, value| {
            accessible(
                iced::widget::button(iced::widget::text(label).size(20))
                    .width(150)
                    .height(48)
                    .on_press(value),
                StableId::new(id),
                Role::Button,
            )
            .logical_id(id)
            .label(label)
            .on_activate(value)
            .focus_ring(iced::Color::from_rgb8(0, 97, 255), 4.0)
            .into()
        };
        directed_row(
            [
                action("rtl-first", "ראשון", 1),
                action("rtl-second", "שני", 2),
            ],
            if rtl {
                Direction::RightToLeft
            } else {
                Direction::LeftToRight
            },
        )
        .gap(12.0)
        .into()
    }

    pub fn direction_matrix() -> iced::Element<'static, i64> {
        use iced::{Length, Padding};
        use ui_lang_components::ui::direction::{Direction, directed_row};
        use ui_lang_runtime::{FlexWrap, JustifyContent};
        use ui_lang_runtime::{Role, StableId, accessible};
        let actions = |prefix: &'static str| {
            [
                ("ראשון", 100),
                ("שני", 120),
                ("שלישי", 80),
                ("רביעי", 110),
                ("חמישי", 90),
            ]
            .into_iter()
            .enumerate()
            .map(move |(index, (label, width))| {
                let id = format!("{prefix}-{index}");
                accessible(
                    iced::widget::button(iced::widget::text(label).size(20))
                        .width(width)
                        .height(48)
                        .on_press(index as i64 + 1),
                    StableId::new(&id),
                    Role::Button,
                )
                .logical_id(id)
                .label(label)
                .on_activate(index as i64 + 1)
                .focus_ring(iced::Color::from_rgb8(0, 97, 255), 4.0)
                .into()
            })
        };
        let growing = |prefix: &'static str, direction| {
            let cells = [
                ("One share", Length::FillPortion(1), 20),
                ("Two shares", Length::FillPortion(2), 40),
                ("Fixed", Length::Fixed(60.0), 30),
            ]
            .into_iter()
            .enumerate()
            .map(|(index, (label, width, height))| {
                let id = format!("{prefix}-{index}");
                accessible(
                    iced::widget::container(iced::widget::text(label).size(12))
                        .width(width)
                        .height(height)
                        .align_y(iced::alignment::Vertical::Center),
                    StableId::new(&id),
                    Role::Label,
                )
                .logical_id(id)
                .label(label)
                .into()
            });
            directed_row(cells, direction)
                .width(400)
                .gap(12.0)
                .padding(Padding {
                    left: 13.0,
                    right: 27.0,
                    top: 7.0,
                    bottom: 11.0,
                })
                .align_items(ui_lang_runtime::AlignItems::Center)
        };
        let fixed = ["Wide first", "Wide second"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
                let id = format!("fixed-{index}");
                accessible(
                    iced::widget::container(iced::widget::text(label).size(12))
                        .width(120)
                        .height(30),
                    StableId::new(&id),
                    Role::Label,
                )
                .logical_id(id)
                .label(label)
                .into()
            });
        iced::widget::column![
            directed_row(actions("ltr"), Direction::LeftToRight)
                .gap(12.0)
                .width(400)
                .padding(Padding {
                    left: 13.0,
                    right: 27.0,
                    top: 7.0,
                    bottom: 11.0
                })
                .wrap(FlexWrap::Wrap)
                .justify_content(JustifyContent::End),
            directed_row(actions("rtl"), Direction::RightToLeft)
                .gap(12.0)
                .width(400)
                .padding(Padding {
                    left: 13.0,
                    right: 27.0,
                    top: 7.0,
                    bottom: 11.0
                })
                .wrap(FlexWrap::Wrap)
                .justify_content(JustifyContent::End),
            growing("grow-ltr", Direction::LeftToRight),
            growing("grow-rtl", Direction::RightToLeft),
            directed_row(fixed, Direction::RightToLeft)
                .width(200)
                .gap(12.0),
        ]
        .width(Length::Fill)
        .spacing(16)
        .into()
    }
}

ui_lang::include_app!("tests/cases/ui/accessible_localized_defaults.ice");

#[test]
fn rtl_wrapping_keeps_logical_lines_and_physical_alignment() {
    use ui_lang_runtime::testing::{Config, Driver, Key, Location, MouseButton, Platform};
    const HERE: Location = Location::new(
        "accessible_localized_defaults.rs",
        1,
        1,
        "RTL layout and logical traversal",
    );
    let program = AccessibleLocalizedDefaults::__program();
    assert_eq!(
        iced_test::program::Program::settings(&program).fonts.len(),
        2,
        "both bundled Hebrew font faces reach native font loading"
    );
    let mut driver = Driver::new(
        program,
        Config::new("rtl_wrapping_logical_order")
            .viewport(520.0, 700.0)
            .scale_factor(1.0)
            .locale("he-IL")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    driver.capture("rtl_wrapping_before", HERE);
    let ltr: Vec<_> = (0..5)
        .map(|i| driver.target(&format!("ltr-{i}"), HERE))
        .collect();
    let rtl: Vec<_> = (0..5)
        .map(|i| driver.target(&format!("rtl-{i}"), HERE))
        .collect();
    assert_eq!(rtl[0].accessibility_name(), "ראשון");
    assert!(
        (rtl[0].text_width() - 49.7265625).abs() < 0.01,
        "loaded Hebrew font shapes real glyphs"
    );
    assert!(ltr[0].left() < ltr[1].left() && ltr[1].left() < ltr[2].left());
    assert!(rtl[0].left() > rtl[1].left() && rtl[1].left() > rtl[2].left());
    assert_eq!(
        rtl[0].top(),
        rtl[2].top(),
        "first three logical actions share a line"
    );
    assert!(rtl[3].top() > rtl[2].bottom());
    assert_eq!(rtl[3].top(), rtl[4].top());
    assert_eq!(
        rtl[0].right(),
        ltr[2].right(),
        "physical right alignment survives reversal"
    );
    assert_eq!(
        rtl[3].right(),
        ltr[4].right(),
        "short final line stays right aligned"
    );
    for prefix in ["grow-ltr", "grow-rtl"] {
        let cells: Vec<_> = (0..3)
            .map(|i| driver.target(&format!("{prefix}-{i}"), HERE))
            .collect();
        assert!(
            (cells[0].width() - 92.0).abs() < 0.01,
            "one FillPortion grows into remaining width"
        );
        assert!(
            (cells[1].width() - 184.0).abs() < 0.01,
            "two FillPortions receive twice the width"
        );
        assert_eq!(cells[2].width(), 60.0, "fixed child does not shrink");
        assert_eq!(cells[0].height(), 20.0);
        assert_eq!(cells[1].height(), 40.0);
        assert_eq!(cells[2].height(), 30.0);
        assert_eq!(
            cells[0].center_y(),
            cells[1].center_y(),
            "natural heights align vertically"
        );
        assert_eq!(cells[2].center_y(), cells[1].center_y());
        let (left, right) = if prefix == "grow-ltr" {
            (&cells[0], &cells[2])
        } else {
            (&cells[2], &cells[0])
        };
        assert_eq!(
            left.left(),
            ltr[0].left() - 36.0,
            "physical leading padding"
        );
        assert_eq!(right.right(), ltr[2].right(), "physical trailing padding");
    }
    let fixed_first = driver.target("fixed-0", HERE);
    let fixed_second = driver.target("fixed-1", HERE);
    assert_eq!(
        fixed_first.width(),
        120.0,
        "overconstrained fixed children retain their width until wrapping is requested"
    );
    assert_eq!(fixed_second.width(), 120.0);
    assert_eq!(
        fixed_second.left(),
        24.0,
        "no-wrap RTL preserves physical start alignment even when fixed children overflow"
    );
    assert_eq!(fixed_first.left() - fixed_second.right(), 12.0);
    for i in 0..10 {
        driver.key(Key::Named(iced::keyboard::key::Named::Tab), HERE);
        let prefix = if i < 5 { "ltr" } else { "rtl" };
        let target = driver.target(&format!("{prefix}-{}", i % 5), HERE);
        assert!(target.focused(), "logical tab stop {i}");
        if i == 5 {
            let focused = driver.capture("rtl_wrapping_keyboard_focus", HERE);
            let x = target.center_x() as usize;
            let y = target.top() as usize + 1;
            let pixel = (y * focused.width as usize + x) * 4;
            assert!(
                focused.rgba[pixel] < 5
                    && (i16::from(focused.rgba[pixel + 1]) - 97).abs() < 5
                    && focused.rgba[pixel + 2] > 250,
                "keyboard focus paints the blue border"
            );
        }
        driver.key(Key::Named(iced::keyboard::key::Named::Enter), HERE);
        assert_eq!(driver.state().chosen, i % 5 + 1);
    }
    driver.click_at(
        rtl[2].center_x() as f32,
        rtl[2].center_y() as f32,
        MouseButton::Left,
        HERE,
    );
    assert_eq!(driver.state().chosen, 3);
    driver.capture("rtl_wrapping_pointer", HERE);
}
