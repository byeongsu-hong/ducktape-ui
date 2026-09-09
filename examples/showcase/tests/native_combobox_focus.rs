//! Native Combobox keyboard, viewport, and focus lifecycle (no adapter simulation).
use iced::advanced::renderer::Headless as _;
use iced::advanced::widget::{self, Operation};
use iced::keyboard::{self, key::Named};
use iced::{Element, Event, Length, Rectangle, Size};
use ui_lang_components::ui::{combobox::combobox, theme::LIGHT};

type NativeUi<'a> = iced_runtime::UserInterface<'a, usize, iced::Theme, iced::Renderer>;
fn with_combo(test: impl FnOnce(&mut NativeUi<'_>, &mut iced::Renderer)) {
    let state = iced::widget::combo_box::State::new((0..30).collect::<Vec<_>>());
    let combo = combobox(&state, "Choose a result", None, |value| value, &LIGHT)
        .width(220)
        .menu_style(|_| iced::widget::overlay::menu::Style {
            selected_background: iced::Color::from_rgb(0.0, 1.0, 0.0).into(),
            ..ui_lang_components::ui::theme::menu_style(&LIGHT)
        });
    let view: Element<'_, usize> = iced::widget::container(combo)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Bottom)
        .into();
    let mut renderer = iced::futures::executor::block_on(iced::Renderer::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    ))
    .unwrap();
    let mut ui = NativeUi::build(
        view,
        Size::new(320.0, 260.0),
        iced_runtime::user_interface::Cache::default(),
        &mut renderer,
    );
    test(&mut ui, &mut renderer);
}
#[derive(Default)]
struct FocusCount {
    total: usize,
    focused: usize,
}
impl Operation for FocusCount {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn widget::operation::Focusable,
    ) {
        self.total += 1;
        self.focused += usize::from(state.is_focused());
    }
}
fn focused(ui: &mut NativeUi<'_>, renderer: &iced::Renderer) -> FocusCount {
    let mut count = FocusCount::default();
    ui.operate(renderer, &mut count);
    count
}
fn key(ui: &mut NativeUi<'_>, renderer: &mut iced::Renderer, key: Named) -> Vec<usize> {
    let key = keyboard::Key::Named(key);
    let mut messages = vec![];
    let _ = ui.update(
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        })],
        iced::mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut messages,
    );
    messages
}
fn open_with_pointer(ui: &mut NativeUi<'_>, renderer: &mut iced::Renderer) {
    let mut messages = vec![];
    let _ = ui.update(
        &[
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ],
        iced::mouse::Cursor::Available(iced::Point::new(200.0, 245.0)),
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut messages,
    );
}
fn highlighted_pixels(ui: &mut NativeUi<'_>, renderer: &mut iced::Renderer) -> usize {
    let _ = ui.update(
        &[Event::Window(iced::window::Event::RedrawRequested(
            iced::time::Instant::now(),
        ))],
        iced::mouse::Cursor::Unavailable,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut vec![],
    );
    ui.draw(
        renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: iced::Color::BLACK,
        },
        iced::mouse::Cursor::Unavailable,
    );
    renderer
        .screenshot(Size::new(320, 260), 1.0, iced::Color::WHITE)
        .chunks_exact(4)
        .filter(|pixel| pixel[0] < 10 && pixel[1] > 245 && pixel[2] < 10 && pixel[3] == 255)
        .count()
}
#[test]
fn native_combobox_participates_in_keyboard_focus() {
    with_combo(|ui, renderer| {
        let mut operation: Box<dyn Operation> =
            Box::new(widget::operation::focusable::focus_next::<()>());
        loop {
            ui.operate(renderer, operation.as_mut());
            match operation.finish() {
                widget::operation::Outcome::Chain(next) => operation = next,
                _ => break,
            }
        }
        let count = focused(ui, renderer);
        assert_eq!(
            count.total, 1,
            "native Combobox must expose its input to focus operations"
        );
        assert_eq!(count.focused, 1);
    });
}
#[test]
fn native_combobox_keyboard_highlight_remains_visible_at_window_edge() {
    with_combo(|ui, renderer| {
        open_with_pointer(ui, renderer);
        assert!(
            highlighted_pixels(ui, renderer) > 100,
            "initial option highlight establishes the paint oracle"
        );
        for _ in 0..29 {
            key(ui, renderer, Named::ArrowDown);
        }
        assert!(
            highlighted_pixels(ui, renderer) > 100,
            "keyboard active option must remain visibly highlighted after scrolling"
        );
        assert_eq!(key(ui, renderer, Named::Enter), [29]);
    });
}
#[test]
fn native_combobox_selection_and_escape_keep_input_focus() {
    with_combo(|ui, renderer| {
        open_with_pointer(ui, renderer);
        assert_eq!(
            focused(ui, renderer).focused,
            1,
            "opened input must be focused"
        );
        assert_eq!(key(ui, renderer, Named::Enter), [0]);
        assert_eq!(
            focused(ui, renderer).focused,
            1,
            "selection must return to the input"
        );
        assert_eq!(
            highlighted_pixels(ui, renderer),
            0,
            "selection must close the menu"
        );
        key(ui, renderer, Named::ArrowDown);
        assert!(
            highlighted_pixels(ui, renderer) > 100,
            "arrow navigation must reopen the focused combobox"
        );
        key(ui, renderer, Named::Escape);
        assert_eq!(
            focused(ui, renderer).focused,
            1,
            "Escape must retain input focus"
        );
        assert_eq!(highlighted_pixels(ui, renderer), 0);
    });
}

#[test]
fn native_combobox_pointer_selection_keeps_input_focus() {
    with_combo(|ui, renderer| {
        open_with_pointer(ui, renderer);
        assert!(highlighted_pixels(ui, renderer) > 100);
        let mut messages = vec![];
        let _ = ui.update(
            &[
                Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
                Event::Mouse(iced::mouse::Event::ButtonReleased(
                    iced::mouse::Button::Left,
                )),
            ],
            iced::mouse::Cursor::Available(iced::Point::new(200.0, 18.0)),
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(messages, [0]);
        assert_eq!(
            focused(ui, renderer).focused,
            1,
            "pointer selection must return to input focus"
        );
        assert_eq!(highlighted_pixels(ui, renderer), 0);
    });
}
#[test]
fn native_combobox_empty_query_keeps_focus_without_selecting() {
    with_combo(|ui, renderer| {
        open_with_pointer(ui, renderer);
        assert!(highlighted_pixels(ui, renderer) > 100);
        let mut messages = vec![];
        let _ = ui.update(
            &iced_test::simulator::typewrite("absent").collect::<Vec<_>>(),
            iced::mouse::Cursor::Unavailable,
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(highlighted_pixels(ui, renderer), 0);
        assert!(key(ui, renderer, Named::Enter).is_empty());
        assert_eq!(focused(ui, renderer).focused, 1);
        key(ui, renderer, Named::Escape);
        assert_eq!(
            focused(ui, renderer).focused,
            1,
            "empty-result Escape must keep input focus"
        );
        assert_eq!(highlighted_pixels(ui, renderer), 0);
    });
}

#[test]
fn native_combobox_touch_reopens_after_selection() {
    with_combo(|ui, renderer| {
        open_with_pointer(ui, renderer);
        assert_eq!(key(ui, renderer, Named::Enter), [0]);
        assert_eq!(highlighted_pixels(ui, renderer), 0);
        let position = iced::Point::new(200.0, 245.0);
        let _ = ui.update(
            &[Event::Touch(iced::touch::Event::FingerPressed {
                id: iced::touch::Finger(0),
                position,
            })],
            iced::mouse::Cursor::Available(position),
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut vec![],
        );
        assert!(
            highlighted_pixels(ui, renderer) > 100,
            "touch must reopen the focused combobox after selection"
        );
    });
}
