//! Public custom semantics and native gesture ownership, without replacement widgets.
use iced::keyboard::key::Named;
use iced::widget::{Column, container, text};
use iced::{Element, Length, Task};
use ui_lang_components::ui::{
    carousel::{
        CarouselBoundary, CarouselCommand, CarouselEvent, CarouselState,
        carousel_next_with_content, carousel_viewport,
    },
    drawer::{DrawerEvent, DrawerState, drawer},
    focus_control::FocusControl,
    item::item,
    menu::{MenuEvent, MenuState},
    modal::FocusScope,
    select::{SelectEvent, SelectGroup, SelectIds, SelectOption, select},
    theme::LIGHT,
};
use ui_lang_runtime::testing::{
    Config, Driver, Key, Location, MouseButton, Platform, ThemeMode, TouchPhase,
};
use ui_lang_runtime::{Role, StableId, accessible};

const HERE: Location = Location::new(
    "custom_interaction_contracts.rs",
    1,
    1,
    "custom interaction",
);
fn config(name: &'static str) -> Config {
    Config::new(name)
        .viewport(360.0, 260.0)
        .scale_factor(1.0)
        .locale("en-US")
        .platform(Platform::Linux)
        .theme(ThemeMode::Light)
        .reduced_motion(true)
}

#[derive(Default)]
struct RowState {
    selected: bool,
    activations: usize,
}
#[derive(Clone, Debug)]
enum RowMessage {
    Activate,
}
fn row_update(state: &mut RowState, _: RowMessage) -> Task<RowMessage> {
    state.selected = !state.selected;
    state.activations += 1;
    Task::none()
}
fn row_view(state: &RowState) -> Element<'_, RowMessage> {
    let visual = item(
        Some(text(if state.selected { "●" } else { "○" }).into()),
        "Workspace Alpha",
        Some("Custom leading and trailing content"),
        Some(text("Local").into()),
        &LIGHT,
    );
    let control = FocusControl::new("custom-row".into(), visual, RowMessage::Activate, &LIGHT);
    container(
        accessible(control, StableId::new("custom-row"), Role::CheckBox)
            .logical_id("custom-row")
            .focus_id("custom-row")
            .focus_descendant()
            .label("Select workspace Alpha")
            .checked(state.selected)
            .on_activate(RowMessage::Activate),
    )
    .padding(16)
    .into()
}
#[test]
fn custom_row_preserves_accessible_and_keyboard_selection() {
    let mut driver = Driver::new(
        iced::application(RowState::default, row_update, row_view),
        config("accessible_custom_row"),
    );
    let row = driver.target("custom-row", HERE);
    assert_eq!(row.accessibility_role(), Role::CheckBox);
    assert_eq!(row.accessibility_name(), "Select workspace Alpha");
    assert!(!row.accessibility_checked());
    assert!(row.accessibility_supports_activate());
    driver.accessibility_activate("custom-row", HERE);
    assert!(driver.state().selected);
    assert_eq!(driver.state().activations, 1);
    assert!(driver.target("custom-row", HERE).accessibility_checked());
    driver.accessibility_focus("custom-row", HERE);
    assert!(driver.target("custom-row", HERE).focused());
    driver.key(Key::named(Named::Space), HERE);
    assert!(!driver.state().selected);
    assert_eq!(driver.state().activations, 2);
    driver.capture("custom_row", HERE);
}
#[test]
fn second_touch_cannot_steal_a_custom_rows_activation() {
    let mut driver = Driver::new(
        iced::application(RowState::default, row_update, row_view),
        config("custom_row_touch_owner"),
    );
    driver.touch(TouchPhase::Down, 1, 120.0, 40.0, HERE);
    driver.touch(TouchPhase::Down, 2, 120.0, 40.0, HERE);
    driver.touch(TouchPhase::Up, 2, 120.0, 40.0, HERE);
    assert_eq!(
        driver.state().activations,
        0,
        "a second finger must not steal the active press"
    );
    driver.touch(TouchPhase::Cancel, 1, 120.0, 40.0, HERE);
    assert!(!driver.state().selected);
    driver.accessibility_focus("custom-row", HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert!(driver.state().selected);
    assert_eq!(driver.state().activations, 1);
}

fn carousel_state() -> CarouselState {
    CarouselState::new(0, 4, CarouselBoundary::Bounded)
}
fn carousel_update(state: &mut CarouselState, event: CarouselEvent) -> Task<CarouselEvent> {
    state.apply(event);
    Task::none()
}
fn carousel_view(state: &CarouselState) -> Element<'_, CarouselEvent> {
    let next = CarouselEvent::Navigate(CarouselCommand::Next);
    let control = carousel_next_with_content(
        "custom-next".into(),
        *state,
        next,
        text("Next preview →"),
        &LIGHT,
    );
    let control = accessible(control, StableId::new("custom-next"), Role::Button)
        .logical_id("custom-next")
        .focus_id("custom-next")
        .focus_descendant()
        .label("Next preview")
        .disabled(!state.can_next())
        .on_activate(next);
    container(
        Column::new()
            .spacing(8)
            .push(
                accessible(
                    carousel_viewport(
                        "carousel".into(),
                        *state,
                        container(text(format!("Preview {}", state.index() + 1)))
                            .center_x(Length::Fill)
                            .center_y(160),
                        |event| event,
                        &LIGHT,
                    )
                    .width(Length::Fill)
                    .height(160),
                    StableId::new("carousel"),
                    Role::Group,
                )
                .logical_id("carousel")
                .focus_id("carousel")
                .focus_descendant()
                .label("Preview carousel"),
            )
            .push(control),
    )
    .padding(16)
    .into()
}
#[test]
fn explicit_selection_interrupts_an_older_carousel_swipe() {
    let mut driver = Driver::new(
        iced::application(carousel_state, carousel_update, carousel_view),
        config("carousel_interrupted_swipe"),
    );
    driver.touch(TouchPhase::Down, 1, 270.0, 80.0, HERE);
    driver.touch(TouchPhase::Move, 1, 160.0, 80.0, HERE);
    assert_eq!(driver.state().index(), 0);
    driver.accessibility_activate("custom-next", HERE);
    assert_eq!(driver.state().index(), 1);
    driver.touch(TouchPhase::Up, 1, 160.0, 80.0, HERE);
    assert_eq!(
        driver.state().index(),
        1,
        "an interrupted swipe must not override an explicit selection"
    );
}
#[test]
fn cancelled_carousel_touch_keeps_custom_keyboard_alternative() {
    let mut driver = Driver::new(
        iced::application(carousel_state, carousel_update, carousel_view),
        config("carousel_cancelled_touch"),
    );
    driver.touch(TouchPhase::Down, 1, 270.0, 80.0, HERE);
    driver.touch(TouchPhase::Move, 1, 160.0, 80.0, HERE);
    driver.touch(TouchPhase::Cancel, 1, 160.0, 80.0, HERE);
    assert_eq!(driver.state().index(), 0);
    driver.touch(TouchPhase::Down, 2, 270.0, 80.0, HERE);
    driver.touch(TouchPhase::Move, 2, 160.0, 80.0, HERE);
    driver.touch(TouchPhase::Up, 2, 160.0, 80.0, HERE);
    assert_eq!(
        driver.state().index(),
        1,
        "cancel must release ownership for a fresh swipe"
    );
    driver.accessibility_focus("custom-next", HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert_eq!(driver.state().index(), 2);
    driver.focus("carousel", HERE);
    driver.key(Key::named(Named::ArrowRight), HERE);
    assert_eq!(driver.state().index(), 3);
    driver.capture("custom_carousel", HERE);
}

#[derive(Default)]
struct DrawerFixture {
    drawer: DrawerState,
    checked: bool,
    snap_animate: Option<bool>,
    animated: bool,
}
#[derive(Clone, Debug)]
enum DrawerMessage {
    Open,
    Close,
    Toggle,
    Event(DrawerEvent),
}
fn drawer_focus() -> FocusScope {
    FocusScope::new("drawer-close".into(), "drawer-trigger".into()).push("drawer-check".into())
}
fn drawer_update(state: &mut DrawerFixture, message: DrawerMessage) -> Task<DrawerMessage> {
    match message {
        DrawerMessage::Open => state.drawer.set_open(true, &drawer_focus()),
        DrawerMessage::Close => state.drawer.set_open(false, &drawer_focus()),
        DrawerMessage::Toggle => {
            state.checked = !state.checked;
            Task::none()
        }
        DrawerMessage::Event(event) => {
            if let DrawerEvent::SnapBack { animate } = event {
                state.snap_animate = Some(animate);
            }
            state.drawer.apply(&event);
            event.focus_task(&drawer_focus())
        }
    }
}
fn drawer_button(
    id: &'static str,
    label: &'static str,
    message: DrawerMessage,
) -> Element<'static, DrawerMessage> {
    accessible(
        FocusControl::new(id.into(), text(label), message.clone(), &LIGHT),
        StableId::new(id),
        Role::Button,
    )
    .logical_id(id)
    .focus_id(id)
    .focus_descendant()
    .label(label)
    .on_activate(message)
    .into()
}
fn drawer_view(state: &DrawerFixture) -> Element<'_, DrawerMessage> {
    let check = accessible(
        FocusControl::new(
            "drawer-check".into(),
            text(if state.checked {
                "[x] Keep offline"
            } else {
                "[ ] Keep offline"
            }),
            DrawerMessage::Toggle,
            &LIGHT,
        ),
        StableId::new("drawer-check"),
        Role::CheckBox,
    )
    .logical_id("drawer-check")
    .focus_id("drawer-check")
    .focus_descendant()
    .label("Keep offline")
    .checked(state.checked)
    .on_activate(DrawerMessage::Toggle);
    let body = accessible(
        Column::new()
            .spacing(8)
            .padding(12)
            .push(
                container(iced::widget::Space::new())
                    .id("drawer-grip-area")
                    .width(Length::Fill)
                    .height(40),
            )
            .push(check)
            .push(drawer_button(
                "drawer-close",
                "Close settings",
                DrawerMessage::Close,
            )),
        StableId::new("drawer-body"),
        Role::Group,
    )
    .logical_id("drawer-body")
    .label("Preview settings");
    drawer(
        drawer_button("drawer-trigger", "Open settings", DrawerMessage::Open),
        &state.drawer,
        body,
        &drawer_focus(),
        DrawerMessage::Event,
        &LIGHT,
    )
    .size(160.0)
    .reduced_motion(!state.animated)
    .into()
}
#[test]
fn drawer_blur_cancels_a_pending_custom_body_activation() {
    let mut driver = Driver::new(
        iced::application(DrawerFixture::default, drawer_update, drawer_view),
        config("drawer_blur_pending_action"),
    );
    driver.accessibility_activate("drawer-trigger", HERE);
    driver.accessibility_focus("drawer-check", HERE);
    driver.key_down(Key::named(Named::Space), HERE);
    driver.touch(TouchPhase::Down, 1, 180.0, 110.0, HERE);
    driver.touch(TouchPhase::Move, 1, 180.0, 150.0, HERE);
    assert!(driver.state().drawer.offset() > 0.0);
    driver.window_focus(false, HERE);
    assert_eq!(driver.state().drawer.offset(), 0.0);
    assert_eq!(driver.state().snap_animate, Some(false));
    driver.window_focus(true, HERE);
    driver.key_up(Key::named(Named::Space), HERE);
    assert!(
        !driver.state().checked,
        "window blur must cancel the body's pending Space press during a drawer drag"
    );
    driver.touch(TouchPhase::Cancel, 1, 180.0, 150.0, HERE);
    driver.accessibility_focus("drawer-check", HERE);
    driver.key(Key::named(Named::Space), HERE);
    assert!(driver.state().checked);
    driver.key(Key::named(Named::Escape), HERE);
    assert!(!driver.state().drawer.is_open());
    assert!(driver.target("drawer-trigger", HERE).focused());
}

#[derive(Default)]
struct SelectFixture {
    open: bool,
    menu: MenuState,
    selected: Option<usize>,
}
fn select_options() -> Vec<SelectGroup<usize>> {
    vec![SelectGroup::new(
        "workspaces",
        (0..3)
            .map(|i| SelectOption::new(format!("workspace-{i}"), i, format!("Workspace {i}")))
            .collect(),
    )]
}
fn select_update(state: &mut SelectFixture, event: SelectEvent<usize>) -> Task<SelectEvent<usize>> {
    state.open = event.open(state.open);
    match &event {
        SelectEvent::Selected(value) => state.selected = Some(*value),
        SelectEvent::Menu(MenuEvent::StateChanged(menu)) => state.menu = menu.clone(),
        _ => {}
    }
    event.focus_task(&SelectIds::new("custom"), &select_options(), &state.menu)
}
const SELECT_TRIGGER: &str = "ducktape-popover:select:custom:trigger";
fn select_view(state: &SelectFixture) -> Element<'_, SelectEvent<usize>> {
    let value = state.selected.map_or_else(
        || "Choose workspace".to_owned(),
        |i| format!("Workspace {i}"),
    );
    let control = select(
        SelectIds::new("custom"),
        select_options(),
        state.selected,
        "Choose workspace",
        &state.menu,
        state.open,
        |event| event,
        &LIGHT,
    )
    .trigger(container(text(format!("◎ {value} ▾"))).padding(8))
    .width(240.0);
    container(
        accessible(control, StableId::new(SELECT_TRIGGER), Role::ComboBox)
            .logical_id(SELECT_TRIGGER)
            .focus_id(SELECT_TRIGGER)
            .focus_descendant()
            .label("Workspace selector")
            .value(value)
            .expanded(state.open)
            .on_activate(SelectEvent::OpenChanged {
                open: !state.open,
                reason: None,
            }),
    )
    .padding(16)
    .into()
}
#[test]
fn custom_select_trigger_preserves_semantics_selection_and_escape_restore() {
    let mut driver = Driver::new(
        iced::application(SelectFixture::default, select_update, select_view),
        config("accessible_custom_select"),
    );
    let trigger = driver.target(SELECT_TRIGGER, HERE);
    assert_eq!(trigger.accessibility_role(), Role::ComboBox);
    assert_eq!(trigger.accessibility_name(), "Workspace selector");
    assert_eq!(trigger.accessibility_value(), "Choose workspace");
    assert!(!trigger.accessibility_expanded());
    driver.accessibility_activate(SELECT_TRIGGER, HERE);
    assert!(driver.state().open);
    assert!(driver.target(SELECT_TRIGGER, HERE).accessibility_expanded());
    driver.key(Key::named(Named::End), HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert_eq!(driver.state().selected, Some(2));
    assert!(!driver.state().open);
    assert_eq!(
        driver.target(SELECT_TRIGGER, HERE).accessibility_value(),
        "Workspace 2"
    );
    assert!(driver.target(SELECT_TRIGGER, HERE).focused());
    driver.key(Key::named(Named::Space), HERE);
    assert!(driver.state().open);
    driver.key(Key::named(Named::Escape), HERE);
    assert!(!driver.state().open);
    assert!(driver.target(SELECT_TRIGGER, HERE).focused());
    assert_eq!(driver.state().selected, Some(2));
    driver.capture("custom_select", HERE);
}

#[test]
fn mouse_press_cannot_steal_an_active_touch() {
    let mut driver = Driver::new(
        iced::application(RowState::default, row_update, row_view),
        config("custom_row_mixed_press"),
    );
    driver.touch(TouchPhase::Down, 1, 120.0, 40.0, HERE);
    driver.press_with("custom-row", MouseButton::Left, HERE);
    driver.release_button(MouseButton::Left, HERE);
    assert_eq!(
        driver.state().activations,
        0,
        "mouse release must not commit an earlier touch"
    );
    driver.touch(TouchPhase::Up, 1, 120.0, 40.0, HERE);
    assert_eq!(driver.state().activations, 1);
}
#[test]
fn custom_drawer_body_survives_touch_cancel_and_reduced_motion() {
    for animated in [false, true] {
        let mut driver = Driver::new(
            iced::application(
                move || DrawerFixture {
                    animated,
                    ..Default::default()
                },
                drawer_update,
                drawer_view,
            ),
            config("custom_drawer_motion").reduced_motion(!animated),
        );
        driver.accessibility_activate("drawer-trigger", HERE);
        assert_eq!(
            driver.target("drawer-body", HERE).accessibility_name(),
            "Preview settings"
        );
        assert_eq!(
            driver.target("drawer-body", HERE).accessibility_role(),
            Role::Group
        );
        driver.accessibility_activate("drawer-check", HERE);
        assert!(driver.state().checked);
        assert!(driver.target("drawer-check", HERE).accessibility_checked());
        driver.touch(TouchPhase::Down, 1, 180.0, 110.0, HERE);
        driver.touch(TouchPhase::Move, 1, 180.0, 150.0, HERE);
        assert!(driver.state().drawer.offset() > 0.0);
        driver.touch(TouchPhase::Cancel, 1, 180.0, 150.0, HERE);
        assert_eq!(driver.state().drawer.offset(), 0.0);
        assert_eq!(driver.state().snap_animate, Some(animated));
        assert!(driver.state().drawer.is_open());
        assert!(driver.target("drawer-check", HERE).accessibility_checked());
        driver.accessibility_focus("drawer-check", HERE);
        driver.key(Key::named(Named::Space), HERE);
        assert!(!driver.state().checked);
        driver.accessibility_focus("drawer-close", HERE);
        assert!(driver.target("drawer-close", HERE).visible());
        if !animated {
            driver.capture("custom_drawer", HERE);
        }
        driver.key(Key::named(Named::Enter), HERE);
        assert!(!driver.state().drawer.is_open());
        assert!(driver.target("drawer-trigger", HERE).focused());
    }
}

#[test]
fn pointer_window_blur_cancels_carousel_and_drawer_gestures() {
    let mut carousel = Driver::new(
        iced::application(carousel_state, carousel_update, carousel_view),
        config("carousel_pointer_cancel"),
    );
    carousel.press_with("carousel", MouseButton::Left, HERE);
    carousel.move_to_point(60.0, 80.0, HERE);
    carousel.window_focus(false, HERE);
    carousel.window_focus(true, HERE);
    carousel.release_button(MouseButton::Left, HERE);
    assert_eq!(
        carousel.state().index(),
        0,
        "blur must cancel the mouse swipe before release"
    );
    carousel.accessibility_focus("custom-next", HERE);
    carousel.key(Key::named(Named::Enter), HERE);
    assert_eq!(carousel.state().index(), 1);

    let mut drawer = Driver::new(
        iced::application(DrawerFixture::default, drawer_update, drawer_view),
        config("drawer_pointer_cancel"),
    );
    drawer.accessibility_activate("drawer-trigger", HERE);
    drawer.press_with("drawer-grip-area", MouseButton::Left, HERE);
    drawer.move_to_point(180.0, 150.0, HERE);
    assert!(drawer.state().drawer.offset() > 0.0);
    drawer.window_focus(false, HERE);
    assert_eq!(drawer.state().drawer.offset(), 0.0);
    drawer.window_focus(true, HERE);
    drawer.release_button(MouseButton::Left, HERE);
    assert!(drawer.state().drawer.is_open());
    drawer.accessibility_focus("drawer-close", HERE);
    drawer.key(Key::named(Named::Enter), HERE);
    assert!(!drawer.state().drawer.is_open());
    assert!(drawer.target("drawer-trigger", HERE).focused());
}

#[test]
fn an_outside_pointer_press_does_not_cancel_the_first_finger() {
    for second_is_mouse in [false, true] {
        let mut driver = Driver::new(
            iced::application(RowState::default, row_update, row_view),
            config("custom_row_outside_pointer"),
        );
        driver.touch(TouchPhase::Down, 1, 120.0, 40.0, HERE);
        if second_is_mouse {
            driver.click_at(120.0, 220.0, MouseButton::Left, HERE);
        } else {
            driver.touch(TouchPhase::Down, 2, 120.0, 220.0, HERE);
        }
        driver.touch(TouchPhase::Up, 1, 120.0, 40.0, HERE);
        assert_eq!(
            driver.state().activations,
            1,
            "an unrelated outside pointer must not cancel the first finger's release"
        );
        if !second_is_mouse {
            driver.touch(TouchPhase::Cancel, 2, 120.0, 220.0, HERE);
        }
        assert!(
            !driver.target("custom-row", HERE).focused(),
            "outside press still moves focus away"
        );
    }
}
