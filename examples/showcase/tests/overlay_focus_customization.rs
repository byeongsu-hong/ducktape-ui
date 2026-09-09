//! Native public-component contracts at constrained viewports.
use iced::keyboard::key::Named;
use iced::widget::{Column, container, row, text};
use iced::{Element, Length, Task};
use ui_lang_components::ui::{
    dialog::dialog,
    focus_control::FocusControl,
    menu::{MenuEvent, MenuState},
    modal::{FocusScope, ModalEvent},
    select::{SelectEvent, SelectGroup, SelectIds, SelectOption, select},
    theme::LIGHT,
};
use ui_lang_runtime::testing::{Config, Driver, Key, Location, MouseButton, WheelDelta};
const HERE: Location = Location::new(
    "overlay_focus_customization.rs",
    1,
    1,
    "public overlay contract",
);

fn config(name: &'static str) -> Config {
    Config::new(name)
        .scale_factor(1.0)
        .locale("en-US")
        .platform(ui_lang_runtime::testing::Platform::Linux)
        .reduced_motion(true)
        .theme(ui_lang_runtime::testing::ThemeMode::Light)
}

#[derive(Default)]
struct DialogState {
    open: bool,
}
#[derive(Clone, Debug)]
enum DialogMessage {
    Open,
    Close,
    Modal(ModalEvent),
    BodyAction,
}
fn dialog_focus() -> FocusScope {
    FocusScope::new("cancel".into(), "trigger".into())
        .push("body-action".into())
        .push("confirm".into())
}
fn dialog_update(state: &mut DialogState, event: DialogMessage) -> Task<DialogMessage> {
    let was_open = state.open;
    let effect = match event {
        DialogMessage::BodyAction => Task::none(),
        DialogMessage::Open => {
            state.open = true;
            Task::none()
        }
        DialogMessage::Close => {
            state.open = false;
            Task::none()
        }
        DialogMessage::Modal(event) => {
            state.open = ui_lang_components::ui::dialog::next_open(state.open, &event);
            event.focus_task()
        }
    };
    Task::batch([effect, dialog_focus().transition_task(was_open, state.open)])
}
fn dialog_view(state: &DialogState) -> Element<'_, DialogMessage> {
    let body = (0..24).fold(Column::new().spacing(8), |body, i| {
        body.push(
            container(text(format!("Custom body line {i}")))
                .id(format!("line-{i}"))
                .height(24),
        )
    });
    let body = body.push(FocusControl::new(
        "body-action".into(),
        text("Custom body action"),
        DialogMessage::BodyAction,
        &LIGHT,
    ));
    dialog(
        FocusControl::new(
            "trigger".into(),
            text("Open custom dialog"),
            DialogMessage::Open,
            &LIGHT,
        ),
        state.open,
        &dialog_focus(),
        "Review details",
        "Actions remain reachable.",
        body,
        row![
            FocusControl::new(
                "cancel".into(),
                text("Cancel"),
                DialogMessage::Close,
                &LIGHT
            ),
            FocusControl::new(
                "confirm".into(),
                text("Confirm"),
                DialogMessage::Close,
                &LIGHT
            ),
        ]
        .spacing(8),
        DialogMessage::Modal,
        &LIGHT,
    )
}
#[test]
fn long_custom_dialog_keeps_actions_visible_and_body_scrollable() {
    let mut driver = Driver::new(
        iced::application(DialogState::default, dialog_update, dialog_view),
        config("long_custom_dialog").viewport(320.0, 300.0),
    );
    driver.click_with("trigger", MouseButton::Left, 1, HERE);
    driver.capture("dialog_open", HERE);
    let cancel = driver.target("cancel", HERE);

    assert!(cancel.focused());
    assert!(
        cancel.height() > 0.0 && (cancel.visible_height() - cancel.height()).abs() < 0.01,
        "dialog action must remain fully visible"
    );
    let first = driver.target("line-0", HERE);
    assert!(first.visible_height() > 0.0);
    driver.move_to("line-0", HERE);
    driver.wheel_delta(WheelDelta::Pixels { x: 0.0, y: -1000.0 }, HERE);
    let last = driver.target("line-23", HERE);
    assert!(
        (last.visible_height() - last.height()).abs() <= 0.5,
        "the last custom body line must be scrollable into view (native scroll rounds to pixels)"
    );
    driver.capture("long_dialog", HERE);
    driver.key(Key::named(Named::Tab), HERE);
    assert!(driver.target("body-action", HERE).focused());
    driver.key(Key::named(Named::Tab), HERE);
    assert!(driver.target("confirm", HERE).focused());
    driver.key(Key::named(Named::Tab), HERE);
    assert!(driver.target("cancel", HERE).focused());
    driver.key(Key::named(Named::Escape), HERE);
    assert!(!driver.state().open);
    assert!(driver.target("trigger", HERE).focused());
}

#[derive(Default)]
struct SelectState {
    open: bool,
    menu: MenuState,
    selected: Option<usize>,
}
fn options() -> Vec<SelectGroup<usize>> {
    vec![SelectGroup::new(
        "options",
        (0..30)
            .map(|i| SelectOption::new(format!("option-{i}"), i, format!("Option {i}")))
            .collect(),
    )]
}
fn select_update(state: &mut SelectState, event: SelectEvent<usize>) -> Task<SelectEvent<usize>> {
    state.open = event.open(state.open);
    match &event {
        SelectEvent::Selected(value) => state.selected = Some(*value),
        SelectEvent::Menu(MenuEvent::StateChanged(menu)) => state.menu = menu.clone(),
        _ => {}
    }
    event.focus_task(&SelectIds::new("edge"), &options(), &state.menu)
}
fn select_view(state: &SelectState) -> Element<'_, SelectEvent<usize>> {
    container(
        select(
            SelectIds::new("edge"),
            options(),
            state.selected,
            "Choose",
            &state.menu,
            state.open,
            |event| event,
            &LIGHT,
        )
        .trigger(container(text("Custom option chooser")).height(36))
        .width(200.0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right)
    .align_y(iced::alignment::Vertical::Bottom)
    .into()
}
#[test]
fn long_custom_select_reveals_keyboard_result_and_restores_trigger() {
    let mut driver = Driver::new(
        iced::application(SelectState::default, select_update, select_view),
        config("long_custom_select").viewport(320.0, 260.0),
    );
    let trigger = "ducktape-popover:select:edge:trigger";
    driver.click_with(trigger, MouseButton::Left, 1, HERE);
    assert!(driver.state().open);
    driver.key(Key::named(Named::End), HERE);
    let last = driver.target("ducktape-menu:16:select:edge:menu:9:option-29", HERE);
    assert!(last.focused());
    assert!(
        last.visible()
            && last.height() > 0.0
            && (last.visible_height() - last.height()).abs() < 0.01,
        "active last option must be fully visible"
    );
    driver.capture("last_option", HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert_eq!(driver.state().selected, Some(29));
    assert!(!driver.state().open);
    assert!(driver.target(trigger, HERE).focused());
}

use ui_lang_components::ui::command::{
    CommandEvent, CommandState, command, command_group_without_heading, command_item,
    focus_command_input,
};
use ui_lang_components::ui::popover::{PopoverEvent, PopoverIds, popover};
#[derive(Default)]
struct SearchState {
    open: bool,
    left: bool,
    top: bool,
    command: CommandState,
    selected: Option<usize>,
}
#[derive(Clone, Debug)]
enum SearchMessage {
    Popover(PopoverEvent),
    Command(CommandEvent<usize>),
}
fn search_update(state: &mut SearchState, event: SearchMessage) -> Task<SearchMessage> {
    match event {
        SearchMessage::Popover(event) => {
            state.open = event.open();
            if state.open {
                focus_command_input("search")
            } else {
                event.focus_task(&PopoverIds::new("search"))
            }
        }
        SearchMessage::Command(event) => {
            state.command.apply(&event);
            if let Some(value) = event.selection() {
                state.selected = Some(*value);
                state.open = false;
                PopoverEvent::Close(ui_lang_components::ui::popover::DismissReason::Trigger)
                    .focus_task(&PopoverIds::new("search"))
            } else {
                event.focus_task("search")
            }
        }
    }
}
fn search_view(state: &SearchState) -> Element<'_, SearchMessage> {
    let results = command(
        "search",
        &state.command,
        [command_group_without_heading((0..30).map(|i| {
            command_item(format!("row-{i}"), i, format!("Result {i}")).disabled(i == 28)
        }))],
        SearchMessage::Command,
        &LIGHT,
    )
    .results_height(180.0)
    .empty_content(
        container(text("No matching custom result"))
            .id("empty")
            .height(60),
    )
    .item_content(|item, active, _, _| {
        container(text(format!(
            "{} {}",
            if active { "→" } else { "·" },
            item.label()
        )))
        .width(Length::Fill)
        .height(if item.value() % 2 == 0 { 32 } else { 44 })
        .into()
    });
    container(
        popover(
            PopoverIds::new("search"),
            container(text("Search custom results")).height(36),
            results,
            state.open,
            SearchMessage::Popover,
            &LIGHT,
        )
        .width(280.0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(if state.left {
        iced::alignment::Horizontal::Left
    } else {
        iced::alignment::Horizontal::Right
    })
    .align_y(if state.top {
        iced::alignment::Vertical::Top
    } else {
        iced::alignment::Vertical::Bottom
    })
    .into()
}
#[test]
fn searchable_custom_popover_reveals_results_without_stealing_input_focus() {
    let mut driver = Driver::new(
        iced::application(SearchState::default, search_update, search_view),
        config("search_custom_results").viewport(320.0, 260.0),
    );
    driver.click_with(
        "ducktape-popover:search:trigger",
        MouseButton::Left,
        1,
        HERE,
    );
    let input = "ducktape-command-input:6:search";
    assert!(driver.target(input, HERE).focused());
    for _ in 0..28 {
        driver.key(Key::named(Named::ArrowDown), HERE);
    }
    assert_eq!(driver.state().command.active(), Some("row-29"));
    let last = driver.target("ducktape-command-item:6:search:6:row-29", HERE);
    assert!(
        last.visible()
            && last.height() > 0.0
            && (last.visible_height() - last.height()).abs() < 0.01,
        "variable-height active result must be visible at the viewport edge"
    );
    assert!(driver.target(input, HERE).focused());
    driver.capture("custom_search_last", HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert_eq!(driver.state().selected, Some(29));
    assert!(!driver.state().open);
    assert!(
        driver
            .target("ducktape-popover:search:trigger", HERE)
            .focused()
    );
}
#[test]
fn searchable_custom_popover_handles_empty_results_and_escape_restore() {
    let mut driver = Driver::new(
        iced::application(SearchState::default, search_update, search_view),
        config("search_custom_empty").viewport(320.0, 260.0),
    );
    driver.click_with(
        "ducktape-popover:search:trigger",
        MouseButton::Left,
        1,
        HERE,
    );
    driver.typewrite("absent", HERE);
    assert_eq!(driver.state().command.query(), "absent");
    assert!(driver.target("empty", HERE).visible());
    driver.key(Key::named(Named::ArrowDown), HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert_eq!(driver.state().selected, None);
    assert!(driver.state().open);
    assert!(
        driver
            .target("ducktape-command-input:6:search", HERE)
            .focused()
    );
    driver.capture("custom_search_empty", HERE);
    driver.key(Key::named(Named::Escape), HERE);
    assert!(!driver.state().open);
    assert!(
        driver
            .target("ducktape-popover:search:trigger", HERE)
            .focused()
    );
}

use ui_lang_components::ui::alert_dialog::{
    AlertDialogActionVariant, AlertDialogEvent, AlertDialogFocus, alert_dialog_with_controls,
};
#[derive(Default)]
struct AlertState {
    open: bool,
    opens: usize,
    long_description: bool,
    confirmed: bool,
}
#[derive(Clone, Debug)]
enum AlertMessage {
    Open,
    Event(AlertDialogEvent),
}
fn alert_focus() -> AlertDialogFocus {
    AlertDialogFocus::new("safe".into(), "destructive".into(), "alert-trigger".into())
}
fn alert_update(state: &mut AlertState, message: AlertMessage) -> Task<AlertMessage> {
    let was_open = state.open;
    let task = match message {
        AlertMessage::Open => {
            state.open = true;
            state.opens += 1;
            Task::none()
        }
        AlertMessage::Event(event) => {
            match &event {
                AlertDialogEvent::Action => {
                    state.confirmed = true;
                    state.open = false;
                }
                AlertDialogEvent::Cancel(_) => state.open = false,
                AlertDialogEvent::Focus(_) => {}
            }
            event.focus_task()
        }
    };
    Task::batch([
        task,
        alert_focus().scope().transition_task(was_open, state.open),
    ])
}
fn alert_view(state: &AlertState) -> Element<'_, AlertMessage> {
    alert_dialog_with_controls(
        FocusControl::new(
            "alert-trigger".into(),
            text("Delete record"),
            AlertMessage::Open,
            &LIGHT,
        ),
        state.open,
        &alert_focus(),
        "Delete record?",
        if state.long_description {
            "Review this detail before deciding. ".repeat(80)
        } else {
            "This action cannot be undone.".into()
        },
        row![text("←"), text("Keep record")].spacing(4),
        row![text("×"), text("Delete permanently")].spacing(4),
        AlertDialogActionVariant::Destructive,
        AlertMessage::Event,
        &LIGHT,
    )
}
#[test]
fn custom_alert_controls_keep_safe_focus_trap_and_dismissal_policy() {
    let mut driver = Driver::new(
        iced::application(AlertState::default, alert_update, alert_view),
        config("custom_alert_focus").viewport(360.0, 260.0),
    );
    driver.click_with("alert-trigger", MouseButton::Left, 1, HERE);
    assert!(driver.state().open);
    assert!(driver.target("safe", HERE).focused());
    assert!(!driver.target("destructive", HERE).focused());
    let safe = driver.target("safe", HERE);
    let destructive = driver.target("destructive", HERE);
    assert!(
        destructive.y() >= safe.y() + safe.height(),
        "custom alert actions must wrap before compressing their labels"
    );
    driver.key(Key::named(Named::Tab), HERE);
    assert!(driver.target("destructive", HERE).focused());
    driver.key(Key::named(Named::Tab), HERE);
    assert!(driver.target("safe", HERE).focused());
    driver.click_at(2.0, 2.0, MouseButton::Left, HERE);
    assert!(driver.state().open, "alert backdrop must not dismiss");
    assert_eq!(
        driver.state().opens,
        1,
        "modal backdrop must keep the underlay inert"
    );
    assert!(!driver.state().confirmed);
    driver.key(Key::named(Named::Escape), HERE);
    assert!(!driver.state().open);
    assert!(!driver.state().confirmed);
    assert!(driver.target("alert-trigger", HERE).focused());
    driver.key(Key::named(Named::Enter), HERE);
    assert!(driver.state().open);
    assert!(driver.target("safe", HERE).focused());
    driver.capture("custom_alert", HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert!(!driver.state().open);
    assert!(
        !driver.state().confirmed,
        "Enter after open must choose the safe action"
    );
}

#[test]
fn changing_search_query_reveals_the_first_result_after_scrolling() {
    let mut driver = Driver::new(
        iced::application(SearchState::default, search_update, search_view),
        config("search_query_scroll").viewport(320.0, 260.0),
    );
    driver.click_with(
        "ducktape-popover:search:trigger",
        MouseButton::Left,
        1,
        HERE,
    );
    for _ in 0..28 {
        driver.key(Key::named(Named::ArrowDown), HERE);
    }
    let last = driver.target("ducktape-command-item:6:search:6:row-29", HERE);
    assert!((last.visible_height() - last.height()).abs() < 0.5);
    driver.typewrite("Result", HERE);
    assert_eq!(driver.state().command.query(), "Result");
    assert_eq!(driver.state().command.active(), None);
    let first = driver.target("ducktape-command-item:6:search:5:row-0", HERE);
    assert!(
        first.visible(),
        "query resets active result, so its first result must be visible"
    );
    assert!((first.visible_height() - first.height()).abs() < 0.5);
    assert!(
        driver
            .target("ducktape-command-input:6:search", HERE)
            .focused()
    );
}

#[test]
fn tab_reveals_a_custom_control_in_the_scrolling_dialog_body() {
    let mut driver = Driver::new(
        iced::application(DialogState::default, dialog_update, dialog_view),
        config("dialog_body_focus").viewport(320.0, 300.0),
    );
    driver.click_with("trigger", MouseButton::Left, 1, HERE);
    assert!(driver.target("cancel", HERE).focused());
    assert!(!driver.target("body-action", HERE).visible());
    driver.key(Key::named(Named::Tab), HERE);
    let control = driver.target("body-action", HERE);
    assert!(control.focused());
    assert!(
        control.visible(),
        "Tab must reveal the focused custom body control"
    );
    assert!((control.visible_height() - control.height()).abs() < 0.5);
    driver.capture("focused_body_action", HERE);
}

#[test]
fn searchable_custom_results_fit_each_viewport_corner() {
    for (left, top, name) in [
        (false, false, "search_bottom_right"),
        (false, true, "search_top_right"),
        (true, false, "search_bottom_left"),
        (true, true, "search_top_left"),
    ] {
        let mut driver = Driver::new(
            iced::application(
                move || SearchState {
                    left,
                    top,
                    ..SearchState::default()
                },
                search_update,
                search_view,
            ),
            config(name).viewport(240.0, 240.0),
        );
        driver.click_with(
            "ducktape-popover:search:trigger",
            MouseButton::Left,
            1,
            HERE,
        );
        for _ in 0..28 {
            driver.key(Key::named(Named::ArrowDown), HERE);
        }
        let last = driver.target("ducktape-command-item:6:search:6:row-29", HERE);
        assert!(last.visible());
        assert!((last.visible_height() - last.height()).abs() < 0.5);
        assert!(last.x() >= 0.0 && last.x() + last.width() <= 240.01);
        assert!(last.visible_y() >= 0.0 && last.visible_y() + last.visible_height() <= 240.01);
        driver.capture("corner_result", HERE);
        driver.key(Key::named(Named::Escape), HERE);
        assert!(!driver.state().open);
        assert!(
            driver
                .target("ducktape-popover:search:trigger", HERE)
                .focused()
        );
    }
}

#[test]
fn long_alert_copy_keeps_its_safe_action_reachable() {
    let mut driver = Driver::new(
        iced::application(
            || AlertState {
                long_description: true,
                ..AlertState::default()
            },
            alert_update,
            alert_view,
        ),
        config("long_alert_copy").viewport(360.0, 300.0),
    );
    driver.click_with("alert-trigger", MouseButton::Left, 1, HERE);
    let safe = driver.target("safe", HERE);
    assert!(safe.focused());
    assert!(
        safe.height() >= 36.0 && safe.visible(),
        "long alert copy must not squeeze out the safe action"
    );
    assert!((safe.visible_height() - safe.height()).abs() < 0.5);
    driver.capture("long_alert", HERE);
    driver.key(Key::named(Named::Enter), HERE);
    assert!(!driver.state().open);
    assert!(!driver.state().confirmed);
}

struct ComboDialogState {
    open: bool,
    options: iced::widget::combo_box::State<usize>,
    selected: Option<usize>,
}
impl Default for ComboDialogState {
    fn default() -> Self {
        Self {
            open: false,
            options: iced::widget::combo_box::State::new((0..30).collect()),
            selected: None,
        }
    }
}
#[derive(Clone, Debug)]
enum ComboDialogMessage {
    Open,
    Cancel,
    Selected(usize),
    Modal(ModalEvent),
}
fn combo_dialog_focus() -> FocusScope {
    FocusScope::new("combo-cancel".into(), "combo-trigger".into()).push("dialog-combo".into())
}
fn combo_dialog_update(
    state: &mut ComboDialogState,
    message: ComboDialogMessage,
) -> Task<ComboDialogMessage> {
    let was_open = state.open;
    let effect = match message {
        ComboDialogMessage::Open => {
            state.open = true;
            Task::none()
        }
        ComboDialogMessage::Cancel => {
            state.open = false;
            Task::none()
        }
        ComboDialogMessage::Selected(value) => {
            state.selected = Some(value);
            Task::none()
        }
        ComboDialogMessage::Modal(event) => {
            state.open = ui_lang_components::ui::dialog::next_open(state.open, &event);
            event.focus_task()
        }
    };
    Task::batch([
        effect,
        combo_dialog_focus().transition_task(was_open, state.open),
    ])
}
fn combo_dialog_view(state: &ComboDialogState) -> Element<'_, ComboDialogMessage> {
    dialog(
        FocusControl::new(
            "combo-trigger".into(),
            text("Open chooser dialog"),
            ComboDialogMessage::Open,
            &LIGHT,
        ),
        state.open,
        &combo_dialog_focus(),
        "Choose a number",
        "Custom native combobox body",
        ui_lang_components::ui::combobox::combobox(
            &state.options,
            "Number",
            state.selected.as_ref(),
            ComboDialogMessage::Selected,
            &LIGHT,
        )
        .id("dialog-combo"),
        FocusControl::new(
            "combo-cancel".into(),
            text("Cancel"),
            ComboDialogMessage::Cancel,
            &LIGHT,
        ),
        ComboDialogMessage::Modal,
        &LIGHT,
    )
}
#[test]
fn custom_combobox_body_closes_its_menu_before_the_dialog() {
    let mut driver = Driver::new(
        iced::application(
            ComboDialogState::default,
            combo_dialog_update,
            combo_dialog_view,
        ),
        config("combo_dialog_dismissal").viewport(360.0, 300.0),
    );
    driver.click_with("combo-trigger", MouseButton::Left, 1, HERE);
    assert!(driver.target("combo-cancel", HERE).focused());
    driver.key(Key::named(Named::Tab), HERE);
    assert!(
        driver.target("dialog-combo", HERE).focused(),
        "public Combobox ID must participate in the caller's focus scope"
    );
    driver.key(Key::named(Named::Escape), HERE);
    assert!(
        driver.state().open,
        "first Escape closes the nested menu, preserving the dialog"
    );
    assert!(driver.target("dialog-combo", HERE).focused());
    driver.key(Key::named(Named::Escape), HERE);
    assert!(!driver.state().open);
    assert!(driver.target("combo-trigger", HERE).focused());
}
