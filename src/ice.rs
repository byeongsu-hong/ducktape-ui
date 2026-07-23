//! Typed adapters for using the default components from Ice.

use crate::ui::{
    alert_dialog::{
        AlertDialogActionVariant, AlertDialogEvent as UiAlertDialogEvent, AlertDialogFocus,
        alert_dialog as ui_alert_dialog, next_open as next_alert_dialog_open,
    },
    button::ButtonVariant,
    calendar::{CalendarSelection, Date, Month, controlled_calendar},
    chart::{
        CartesianKind, ChartColor, ChartConfig, ChartData, ChartDatum, SeriesConfig,
        cartesian_chart, companion_content, companion_model, legend_content, tooltip_content,
        tooltip_model,
    },
    command::{command as ui_command, command_group, command_item},
    context_menu::{ContextMenuIds, context_menu as ui_context_menu},
    data_table::{DataTableState, Sort, SortDirection},
    date_picker::{DateFormat, DatePickerIds, DatePickerValue, date_picker as ui_date_picker},
    drawer::{
        DrawerEvent as UiDrawerEvent, DrawerState as UiDrawerState, drawer as ui_drawer,
        drawer_panel,
    },
    dropdown_menu::{DropdownMenuIds, dropdown_menu as ui_dropdown_menu},
    focus_control::FocusControl,
    hover_card::{HoverCardId, hover_card as ui_hover_card},
    input::InputVariant,
    input_otp::{OtpPattern, input_otp as ui_input_otp},
    menu::{MenuEntry, MenuEvent, MenuItem, MenuState},
    menubar::{MenubarMenu, MenubarState as UiMenubarState, menubar as ui_menubar},
    message_scroller::{
        MessageScrollerItemMeta, controlled_message_scroller, message_scroller_item,
    },
    modal::FocusScope,
    navigation_menu::{
        NavigationMenuItem, NavigationMenuItemInfo, navigation_menu as ui_navigation_menu,
    },
    popover::{Alignment, Placement, PopoverIds, next_open, popover},
    progress::ProgressVariant,
    radio_group::{RadioOrientation, focus_radio, radio_group as ui_radio_group, radio_option},
    resizable::resizable,
    select::{SelectGroup, SelectIds, SelectOption, select as ui_select},
    sidebar::{
        SidebarAction, SidebarCollapsible, SidebarId, SidebarMenuButtonId,
        SidebarState as UiSidebarState, SidebarViewport, sidebar as ui_sidebar, sidebar_footer,
        sidebar_group, sidebar_header, sidebar_layout, sidebar_menu, sidebar_menu_button,
    },
    slider::slider as ui_slider,
    sonner::{
        SonnerEvent as UiSonnerEvent, SonnerState as UiSonnerState, ToastPlacement,
        sonner as ui_sonner,
    },
    spinner::spinner as ui_spinner,
    surface::{SurfaceVariant, surface},
    switch::switch as ui_switch,
    theme::LIGHT,
};
use iced::widget::{column, container, text};
use iced::{Color, Element, Length};

pub use crate::ui::{
    calendar::{CalendarEvent, CalendarState},
    chart::ChartHit,
    command::CommandState,
    context_menu::ContextMenuEvent,
    date_picker::DatePickerEvent,
    dropdown_menu::DropdownMenuEvent,
    menubar::MenubarEvent,
    message_scroller::{MessageScrollerEvent, MessageScrollerState},
    navigation_menu::{NavigationMenuEvent, NavigationMenuState},
    popover::PopoverEvent,
};

pub type CommandEvent = crate::ui::command::CommandEvent<String>;
pub type SelectEvent = crate::ui::select::SelectEvent<String>;

#[derive(Debug, Clone)]
pub enum AlertDialogEvent {
    Open,
    Dialog(UiAlertDialogEvent),
}

#[derive(Debug, Clone)]
pub struct DatePickerState {
    ids: DatePickerIds,
    month: Month,
    focused: Option<Date>,
    value: DatePickerValue,
    open: bool,
}

#[derive(Debug, Clone)]
pub struct SelectState {
    ids: SelectIds,
    menu: crate::ui::menu::MenuState,
    selected: Option<String>,
    open: bool,
}

#[derive(Debug, Clone)]
pub struct DropdownMenuState {
    ids: DropdownMenuIds,
    entries: Vec<MenuEntry>,
    menu: MenuState,
    open: bool,
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    ids: ContextMenuIds,
    entries: Vec<MenuEntry>,
    menu: MenuState,
    open: bool,
    anchor: Option<iced::Point>,
}

#[derive(Debug, Clone)]
pub struct AlertDialogState {
    focus: AlertDialogFocus,
    open: bool,
}

#[derive(Debug, Clone)]
pub struct SidebarState {
    navigation: UiSidebarState,
    selected: String,
}

#[derive(Debug, Clone)]
pub enum SidebarEvent {
    Action(SidebarAction),
    Select(String),
}

#[derive(Debug, Clone)]
pub struct SonnerState {
    queue: UiSonnerState,
    shown: i64,
}

#[derive(Debug, Clone)]
pub enum SonnerEvent {
    Show,
    Toast(UiSonnerEvent),
}

#[derive(Debug, Clone)]
pub struct DrawerState {
    drawer: UiDrawerState,
    focus: FocusScope,
}

#[derive(Debug, Clone)]
pub struct MenubarState {
    bar: UiMenubarState,
    menu: MenuState,
    menus: Vec<MenubarMenu>,
}

#[derive(Debug, Clone)]
pub struct MessageScrollerResult {
    state: MessageScrollerState,
    followup: Option<MessageScrollerEvent>,
}

#[derive(Debug, Clone)]
pub enum DrawerEvent {
    Open,
    Close,
    Drawer(UiDrawerEvent),
}

pub fn button_style(
    _iced_theme: &iced::Theme,
    status: iced::widget::button::Status,
    variant: String,
    accent: Color,
) -> iced::widget::button::Style {
    crate::ui::button::style(&theme(accent), button_variant(&variant), status)
}

pub fn checkbox_style(
    _iced_theme: &iced::Theme,
    status: iced::widget::checkbox::Status,
    accent: Color,
) -> iced::widget::checkbox::Style {
    crate::ui::checkbox::style(&theme(accent), status)
}

pub fn input_style(
    _iced_theme: &iced::Theme,
    status: iced::widget::text_input::Status,
    invalid: bool,
    accent: Color,
) -> iced::widget::text_input::Style {
    crate::ui::input::style(
        &theme(accent),
        if invalid {
            InputVariant::Invalid
        } else {
            InputVariant::Default
        },
        status,
    )
}

pub fn switch(id: &str, checked: bool, disabled: bool, accent: Color) -> Element<'static, bool> {
    ui_switch(
        iced::widget::Id::from(id.to_owned()),
        checked,
        !checked,
        &theme(accent),
    )
    .disabled(disabled)
    .into()
}

pub fn progress_style(
    _iced_theme: &iced::Theme,
    variant: String,
    accent: Color,
) -> iced::widget::progress_bar::Style {
    crate::ui::progress::style(&theme(accent), progress_variant(&variant))
}

pub fn input_otp<'a>(
    id: &str,
    value: &'a str,
    invalid: bool,
    disabled: bool,
    accent: Color,
) -> Element<'a, String> {
    ui_input_otp(value, 6, OtpPattern::Digits, String::from, &theme(accent))
        .groups([3, 3])
        .id(iced::widget::Id::from(id.to_owned()))
        .invalid(invalid)
        .disabled(disabled)
        .into()
}

pub fn spinner(frame: i64, reduced_motion: bool, accent: Color) -> Element<'static, ()> {
    ui_spinner(
        frame.rem_euclid(crate::ui::spinner::FRAME_COUNT.into()) as u8,
        reduced_motion,
        &theme(accent),
    )
    .into()
}

pub fn calendar_state() -> CalendarState {
    CalendarState::new(
        Month::new(2026, 7).expect("fixed calendar month is valid"),
        CalendarSelection::Single(None),
    )
    .focused(Date::new(2026, 7, 23).ok())
}

pub fn calendar_apply(mut state: CalendarState, event: CalendarEvent) -> iced::Task<CalendarState> {
    state.apply(&event);
    iced::Task::done(state).chain(event.focus_task("ice-default-calendar"))
}

pub fn calendar(state: &CalendarState, accent: Color) -> Element<'static, CalendarEvent> {
    controlled_calendar("ice-default-calendar", state, |event| event, &theme(accent))
        .today(Date::new(2026, 7, 23).ok())
        .month_dropdown(true)
        .year_dropdown(true)
        .year_range(2024, 2028)
        .into()
}

pub fn date_picker_state() -> DatePickerState {
    DatePickerState {
        ids: DatePickerIds::new("ice-default"),
        month: Month::new(2026, 7).expect("fixed date picker month is valid"),
        focused: Date::new(2026, 7, 23).ok(),
        value: DatePickerValue::Single(None),
        open: false,
    }
}

pub fn date_picker_apply(
    mut state: DatePickerState,
    event: DatePickerEvent,
) -> iced::Task<DatePickerState> {
    state.open = event.next_open(state.open);
    if let Some(value) = event.value() {
        state.value = value;
    }
    if let Some(month) = event.month() {
        state.month = month;
    }
    if let Some(focused) = event.focused() {
        state.focused = Some(focused);
    }
    let focus = event.focus_task(&state.ids);
    iced::Task::done(state).chain(focus)
}

pub fn date_picker(state: &DatePickerState, accent: Color) -> Element<'static, DatePickerEvent> {
    ui_date_picker(
        state.ids.clone(),
        state.month,
        state.focused,
        &state.value,
        state.open,
        |event| event,
        &theme(accent),
    )
    .today(Date::new(2026, 7, 23).ok())
    .month_dropdown(true)
    .year_dropdown(true)
    .year_range(2024, 2028)
    .placeholder("Choose a date")
    .format(DateFormat::MonthDayYear)
    .width(272.0)
    .into()
}

pub fn chart(hovered: Option<ChartHit>, accent: Color) -> Element<'static, Option<ChartHit>> {
    let config = ChartConfig::new([
        SeriesConfig::new("desktop", "Desktop", ChartColor::Primary),
        SeriesConfig::new("mobile", "Mobile", ChartColor::Success),
    ]);
    let data = ChartData::new([
        ChartDatum::new(0.0, "Jan")
            .with_value("desktop", 186.0)
            .with_value("mobile", 80.0),
        ChartDatum::new(1.0, "Feb")
            .with_value("desktop", 305.0)
            .with_value("mobile", 200.0),
        ChartDatum::new(2.0, "Mar")
            .with_value("desktop", 237.0)
            .with_value("mobile", 120.0),
        ChartDatum::new(3.0, "Apr")
            .with_value("desktop", 173.0)
            .with_value("mobile", 190.0),
    ]);
    let theme = theme(accent);
    let tooltip: Element<'static, Option<ChartHit>> = hovered
        .as_ref()
        .and_then(|hit| tooltip_model(&config, &data, hit, &Default::default(), &theme))
        .map_or_else(
            || text("Hover a mark to inspect it.").into(),
            |model| tooltip_content(&model, &theme).into(),
        );
    let companion =
        companion_model("Traffic by month", &config, &data).expect("fixed chart data is valid");

    column![
        cartesian_chart(&config, &data, &theme)
            .kind(CartesianKind::Line { points: true })
            .hovered(hovered)
            .on_hover(|hit| hit)
            .height(240),
        legend_content(&config, &theme),
        tooltip,
        companion_content(&companion, &theme),
    ]
    .spacing(12)
    .into()
}

pub fn command_state() -> CommandState {
    CommandState::default()
}

pub fn command_apply(mut state: CommandState, event: CommandEvent) -> iced::Task<CommandState> {
    state.apply(&event);
    iced::Task::done(state).chain(event.focus_task("ice-default-command"))
}

pub fn command(state: &CommandState, accent: Color) -> Element<'static, CommandEvent> {
    let groups = [
        command_group(
            "Components",
            [
                command_item("calendar", "calendar".to_owned(), "Calendar").shortcut("C"),
                command_item("chart", "chart".to_owned(), "Chart").shortcut("G"),
                command_item("dialog", "dialog".to_owned(), "Dialog").shortcut("D"),
            ],
        ),
        command_group(
            "Actions",
            [
                command_item("settings", "settings".to_owned(), "Open settings").shortcut("⌘,"),
                command_item("help", "help".to_owned(), "Show help").shortcut("?"),
            ],
        ),
    ];

    ui_command(
        "ice-default-command",
        state,
        groups,
        |event| event,
        &theme(accent),
    )
    .results_height(180.0)
    .into_element()
}

pub fn select_state() -> SelectState {
    let groups = select_groups();
    SelectState {
        ids: SelectIds::new("ice-default"),
        menu: crate::ui::menu::MenuState::initial(&crate::ui::select::select_entries(
            &groups, None,
        )),
        selected: None,
        open: false,
    }
}

pub fn select_apply(mut state: SelectState, event: SelectEvent) -> iced::Task<SelectState> {
    state.open = event.open(state.open);
    if let SelectEvent::Selected(value) = &event {
        state.selected = Some(value.clone());
    }
    if let SelectEvent::Menu(crate::ui::menu::MenuEvent::StateChanged(menu)) = &event {
        state.menu.clone_from(menu);
    }
    let focus = event.focus_task(&state.ids, &select_groups(), &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn select<'a>(state: &'a SelectState, accent: Color) -> Element<'a, SelectEvent> {
    ui_select(
        state.ids.clone(),
        select_groups(),
        state.selected.clone(),
        "Choose a component",
        &state.menu,
        state.open,
        |event| event,
        &theme(accent),
    )
    .width(272.0)
    .content_width(272.0)
    .into()
}

pub fn dropdown_menu_state() -> DropdownMenuState {
    let entries = dropdown_entries();
    DropdownMenuState {
        ids: DropdownMenuIds::new("ice-default"),
        menu: MenuState::initial(&entries),
        entries,
        open: false,
    }
}

pub fn dropdown_menu_apply(
    mut state: DropdownMenuState,
    event: DropdownMenuEvent,
) -> iced::Task<DropdownMenuState> {
    state.open = event.open(state.open);
    if let DropdownMenuEvent::Menu(MenuEvent::StateChanged(menu)) = &event {
        state.menu.clone_from(menu);
    }
    let focus = event.focus_task(&state.ids, &state.entries, &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn dropdown_menu(state: &DropdownMenuState, accent: Color) -> Element<'_, DropdownMenuEvent> {
    let theme = theme(accent);
    ui_dropdown_menu(
        state.ids.clone(),
        surface(text("Open menu"), SurfaceVariant::Default, &theme).padding([8, 12]),
        &state.entries,
        &state.menu,
        state.open,
        |event| event,
        &theme,
    )
    .width(248.0)
    .into()
}

pub fn context_menu_state() -> ContextMenuState {
    let entries = dropdown_entries();
    ContextMenuState {
        ids: ContextMenuIds::new("ice-default"),
        menu: MenuState::initial(&entries),
        entries,
        open: false,
        anchor: None,
    }
}

pub fn context_menu_apply(
    mut state: ContextMenuState,
    event: ContextMenuEvent,
) -> iced::Task<ContextMenuState> {
    state.open = event.open(state.open);
    state.anchor = event.anchor(state.anchor);
    if let ContextMenuEvent::Menu(MenuEvent::StateChanged(menu)) = &event {
        state.menu.clone_from(menu);
    }
    let focus = event.focus_task(&state.ids, &state.entries, &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn context_menu(state: &ContextMenuState, accent: Color) -> Element<'_, ContextMenuEvent> {
    let theme = theme(accent);
    ui_context_menu(
        state.ids.clone(),
        surface(
            text("Right-click or touch this region"),
            SurfaceVariant::Muted,
            &theme,
        )
        .padding([12, 16]),
        &state.entries,
        &state.menu,
        state.open,
        state.anchor,
        |event| event,
        &theme,
    )
    .width(248.0)
    .into()
}

pub fn alert_dialog_state() -> AlertDialogState {
    AlertDialogState {
        focus: AlertDialogFocus::new(
            iced::widget::Id::from("ice-alert-cancel"),
            iced::widget::Id::from("ice-alert-confirm"),
            iced::widget::Id::from("ice-alert-trigger"),
        ),
        open: false,
    }
}

pub fn alert_dialog_apply(
    mut state: AlertDialogState,
    event: AlertDialogEvent,
) -> iced::Task<AlertDialogState> {
    let was_open = state.open;
    let focus = match &event {
        AlertDialogEvent::Open => iced::Task::none(),
        AlertDialogEvent::Dialog(event) => event.focus_task(),
    };
    state.open = match &event {
        AlertDialogEvent::Open => true,
        AlertDialogEvent::Dialog(event) => next_alert_dialog_open(state.open, event),
    };
    let transition = state.focus.scope().transition_task(was_open, state.open);
    iced::Task::done(state).chain(focus).chain(transition)
}

pub fn alert_dialog(state: &AlertDialogState, accent: Color) -> Element<'static, AlertDialogEvent> {
    let theme = theme(accent);
    let trigger = FocusControl::new(
        state.focus.restore().clone(),
        surface(text("Delete component"), SurfaceVariant::Muted, &theme).padding([8, 12]),
        AlertDialogEvent::Open,
        &theme,
    );
    ui_alert_dialog(
        trigger,
        state.open,
        &state.focus,
        "Delete this component?",
        "This action cannot be undone. The alert keeps the safest action focused.",
        "Cancel",
        "Delete",
        AlertDialogActionVariant::Destructive,
        AlertDialogEvent::Dialog,
        &theme,
    )
}

pub fn sidebar_state() -> SidebarState {
    SidebarState {
        navigation: UiSidebarState::default(),
        selected: "overview".to_owned(),
    }
}

pub fn sidebar_apply(mut state: SidebarState, event: SidebarEvent) -> SidebarState {
    match event {
        SidebarEvent::Action(action) => state.navigation = state.navigation.reduced(action),
        SidebarEvent::Select(selected) => state.selected = selected,
    }
    state
}

pub fn sidebar(state: &SidebarState, accent: Color) -> Element<'static, SidebarEvent> {
    let theme = theme(accent);
    let collapsed = state
        .navigation
        .is_collapsed(SidebarViewport::Desktop, SidebarCollapsible::Icon);
    let items = [
        ("overview", "⌂", "Overview"),
        ("components", "◇", "Components"),
        ("settings", "⚙", "Settings"),
    ]
    .map(|(id, icon, label)| {
        let content: Element<'static, SidebarEvent> = if collapsed {
            text(icon).into()
        } else {
            text(format!("{icon}  {label}")).into()
        };
        sidebar_menu_button(
            SidebarMenuButtonId::new(id),
            content,
            SidebarEvent::Select(id.to_owned()),
            &theme,
        )
        .active(state.selected == id)
        .collapsed(collapsed)
        .tooltip(text(label))
        .into()
    });
    let panel = ui_sidebar(
        SidebarId::new("ice-default"),
        state.navigation,
        sidebar_group(sidebar_menu(items)),
        SidebarEvent::Action(SidebarAction::Toggle(SidebarViewport::Desktop)),
        &theme,
    )
    .header(sidebar_header(text(if collapsed {
        "UI"
    } else {
        "ducktape-ui"
    })))
    .footer(sidebar_footer(text(if collapsed {
        "?"
    } else {
        "Default navigation"
    })))
    .collapsible(SidebarCollapsible::Icon);
    let main = container(
        column![
            text("Workspace").size(18),
            text(format!("Selected: {}", state.selected)),
            text("Use the rail to collapse the navigation.")
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(16);

    container(sidebar_layout(
        main,
        panel,
        state.navigation,
        SidebarViewport::Desktop,
        Default::default(),
        SidebarEvent::Action(SidebarAction::CloseMobile),
        &theme,
    ))
    .height(240)
    .into()
}

pub fn sonner_state() -> SonnerState {
    let mut queue = UiSonnerState::new(3, ToastPlacement::TopRight);
    queue.info(
        "Ice owns this notification queue.",
        std::time::Duration::ZERO,
    );
    SonnerState { queue, shown: 1 }
}

pub fn sonner_apply(mut state: SonnerState, event: SonnerEvent) -> SonnerState {
    match event {
        SonnerEvent::Show => {
            state.shown += 1;
            state.queue.success(
                format!("Default notification #{}", state.shown),
                std::time::Duration::ZERO,
            );
        }
        SonnerEvent::Toast(event) => {
            state.queue.update(event, std::time::Duration::ZERO);
        }
    }
    state
}

pub fn sonner(state: &SonnerState, accent: Color) -> Element<'_, SonnerEvent> {
    let theme = theme(accent);
    let underlay = container(
        column![
            text("Queue, pause, focus, action, dismiss, and swipe behavior remain native."),
            crate::ui::button::button("Show notification", &theme).on_press(SonnerEvent::Show),
        ]
        .spacing(12),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(16);

    iced::widget::Stack::new()
        .push(underlay)
        .push(ui_sonner(&state.queue, SonnerEvent::Toast, &theme))
        .width(Length::Fill)
        .height(220)
        .into()
}

pub fn drawer_state() -> DrawerState {
    DrawerState {
        drawer: UiDrawerState::new(false),
        focus: FocusScope::new(
            iced::widget::Id::from("ice-drawer-close"),
            iced::widget::Id::from("ice-drawer-trigger"),
        ),
    }
}

pub fn drawer_apply(mut state: DrawerState, event: DrawerEvent) -> iced::Task<DrawerState> {
    let focus = match &event {
        DrawerEvent::Open => state.drawer.set_open(true, &state.focus),
        DrawerEvent::Close => state.drawer.set_open(false, &state.focus),
        DrawerEvent::Drawer(event) => {
            state.drawer.apply(event);
            event.focus_task(&state.focus)
        }
    };
    iced::Task::done(state).chain(focus)
}

pub fn drawer(state: &DrawerState, accent: Color) -> Element<'static, DrawerEvent> {
    let theme = theme(accent);
    let trigger = FocusControl::new(
        state.focus.restore().clone(),
        surface(text("Open drawer"), SurfaceVariant::Default, &theme).padding([8, 12]),
        DrawerEvent::Open,
        &theme,
    );
    let close = FocusControl::new(
        state.focus.first().clone(),
        surface(text("Close"), SurfaceVariant::Muted, &theme).padding([8, 12]),
        DrawerEvent::Close,
        &theme,
    );
    let panel = drawer_panel(
        column![
            text("Default drawer").size(20),
            text("Drag downward, press Escape, use the backdrop, or activate Close.")
        ]
        .spacing(8),
        &theme,
    )
    .close(close);

    ui_drawer(
        trigger,
        &state.drawer,
        panel,
        &state.focus,
        DrawerEvent::Drawer,
        &theme,
    )
    .size(220.0)
    .into()
}

pub fn navigation_menu_state() -> NavigationMenuState {
    NavigationMenuState::initial(&navigation_menu_infos()).active("home")
}

pub fn navigation_menu_apply(event: NavigationMenuEvent) -> iced::Task<NavigationMenuState> {
    let state = event.state().clone();
    iced::Task::done(state).chain(event.focus_task("ice-default-navigation"))
}

pub fn navigation_menu(
    state: &NavigationMenuState,
    accent: Color,
) -> Element<'static, NavigationMenuEvent> {
    let theme = theme(accent);
    ui_navigation_menu(
        "ice-default-navigation",
        [
            NavigationMenuItem::link("home", "Home"),
            NavigationMenuItem::disclosure(
                "components",
                "Components",
                column![
                    text("Inputs").size(14),
                    text("Navigation").size(14),
                    text("Overlays").size(14)
                ]
                .spacing(8),
            ),
            NavigationMenuItem::link("docs", "Documentation"),
        ],
        state,
        |event| event,
        &theme,
    )
    .content_width(320.0)
    .into()
}

pub fn menubar_state() -> MenubarState {
    let menus = menubar_menus();
    MenubarState {
        bar: UiMenubarState::initial(&menus),
        menu: MenuState::initial(&menus[0].entries),
        menus,
    }
}

pub fn menubar_apply(mut state: MenubarState, event: MenubarEvent) -> iced::Task<MenubarState> {
    state.bar = event.state(&state.bar);
    if let MenubarEvent::Menu {
        event: MenuEvent::StateChanged(menu),
        ..
    } = &event
    {
        state.menu.clone_from(menu);
    }
    let focus = event.focus_task("ice-default-menubar", &state.menus, &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn menubar(state: &MenubarState, accent: Color) -> Element<'static, MenubarEvent> {
    ui_menubar(
        "ice-default-menubar",
        state.menus.clone(),
        &state.bar,
        &state.menu,
        |event| event,
        &theme(accent),
    )
    .into()
}

pub fn hover_card(accent: Color) -> Element<'static, ()> {
    let theme = theme(accent);
    ui_hover_card(
        HoverCardId::new("ice-default"),
        surface(
            text("Hover or focus profile"),
            SurfaceVariant::Muted,
            &theme,
        )
        .padding([8, 12]),
        column![
            text("ducktape-ui").size(16),
            text("Default components authored from Ice."),
            crate::ui::button::button("Open profile", &theme).on_press(())
        ]
        .spacing(8),
        &theme,
    )
    .width(280.0)
    .into()
}

pub fn slider(values: &[f64], accent: Color) -> Element<'static, Vec<f64>> {
    ui_slider(
        "ice-default-slider",
        values.iter().map(|value| *value as f32).collect::<Vec<_>>(),
        0.0..=100.0,
        1.0,
        |values| values.into_iter().map(f64::from).collect(),
        &theme(accent),
    )
    .into()
}

pub fn radio_group(selected: &str, accent: Color) -> Element<'static, String> {
    let theme = theme(accent);
    ui_radio_group(
        "ice-default-radio",
        ["default", "comfortable", "compact"]
            .map(|value| radio_option(value.to_owned(), value, &theme)),
        Some(selected.to_owned()),
        |value| value,
        &theme,
    )
    .orientation(RadioOrientation::Horizontal)
    .into()
}

pub fn radio_apply(next: String) -> iced::Task<String> {
    let index = match next.as_str() {
        "comfortable" => 1,
        "compact" => 2,
        _ => 0,
    };
    iced::Task::done(next).chain(focus_radio("ice-default-radio", index))
}

pub fn message_scroller_state() -> MessageScrollerState {
    let mut state = MessageScrollerState::new("ice-default-transcript").auto_scroll(true);
    let _ = state.update(MessageScrollerEvent::ItemsChanged(transcript_metadata()));
    state
}

pub fn message_scroller_apply(
    mut state: MessageScrollerState,
    event: MessageScrollerEvent,
) -> iced::Task<MessageScrollerResult> {
    let followup = state.update(event);
    let immediate = MessageScrollerResult {
        state: state.clone(),
        followup: None,
    };
    iced::Task::done(immediate).chain(followup.map(move |followup| MessageScrollerResult {
        state: state.clone(),
        followup: Some(followup),
    }))
}

pub fn message_scroller_result_state(result: MessageScrollerResult) -> MessageScrollerState {
    result.state
}

pub fn message_scroller_result() -> MessageScrollerResult {
    MessageScrollerResult {
        state: message_scroller_state(),
        followup: None,
    }
}

pub fn message_scroller_continue(
    state: MessageScrollerState,
    result: MessageScrollerResult,
) -> iced::Task<MessageScrollerResult> {
    result.followup.map_or_else(iced::Task::none, |event| {
        message_scroller_apply(state, event)
    })
}

pub fn message_scroller(
    state: &MessageScrollerState,
    accent: Color,
) -> Element<'_, MessageScrollerEvent> {
    let theme = theme(accent);
    controlled_message_scroller(
        state,
        [
            message_scroller_item(
                "welcome",
                surface(
                    text("Welcome — the transcript starts at its live edge."),
                    SurfaceVariant::Muted,
                    &theme,
                )
                .padding([10, 12]),
            ),
            message_scroller_item(
                "components",
                surface(
                    text("Ice owns scroll mode, unread count, and anchor identity."),
                    SurfaceVariant::Default,
                    &theme,
                )
                .padding([10, 12]),
            )
            .scroll_anchor(true),
            message_scroller_item(
                "tasks",
                surface(
                    text("Native measurement tasks route back through the Ice handler."),
                    SurfaceVariant::Muted,
                    &theme,
                )
                .padding([10, 12]),
            ),
            message_scroller_item(
                "latest",
                surface(
                    text("This is the latest message."),
                    SurfaceVariant::Default,
                    &theme,
                )
                .padding([10, 12]),
            ),
        ],
        |event| event,
        &theme,
    )
    .height(220)
    .into()
}

pub fn data_table_rows(query: String, sort: String, page: i64) -> Vec<String> {
    let mut state = DataTableState::new(3);
    state.set_query(query.clone());
    state.sort = match sort.as_str() {
        "ascending" => Some(Sort {
            column: "name",
            direction: SortDirection::Ascending,
        }),
        "descending" => Some(Sort {
            column: "name",
            direction: SortDirection::Descending,
        }),
        _ => None,
    };
    let mut rows = catalog_items(&query);
    match state.sort.as_ref().map(|sort| sort.direction) {
        Some(SortDirection::Ascending) => rows.sort(),
        Some(SortDirection::Descending) => rows.sort_by(|left, right| right.cmp(left)),
        None => {}
    }
    state.set_page(page.max(0) as usize, rows.len());
    rows[state.visible_range(rows.len())].to_vec()
}

pub fn data_table_next_sort(sort: String) -> String {
    match sort.as_str() {
        "none" => "ascending",
        "ascending" => "descending",
        _ => "none",
    }
    .to_owned()
}

pub fn data_table_can_next(query: String, page: i64) -> bool {
    let state = DataTableState::<()>::new(3);
    (page.max(0) as usize + 1) < state.page_count(catalog_items(&query).len())
}

pub fn resizable_demo(sizes: &[f64], accent: Color) -> Element<'static, Vec<f64>> {
    let sizes = sizes.iter().map(|size| *size as f32).collect::<Vec<_>>();
    let panels = ["Navigation", "Canvas", "Inspector"]
        .map(|label| container(text(label)).center(Length::Fill).into());
    let theme = theme(accent);

    resizable(
        "ice-native-resizable",
        panels,
        sizes,
        vec![0.15; 3],
        |next| next.into_iter().map(f64::from).collect(),
        &theme,
    )
    .with_handles(true)
    .height(120)
    .into()
}

pub fn popover_apply(event: PopoverEvent) -> iced::Task<bool> {
    iced::Task::done(next_open(event))
        .chain(event.focus_task(&PopoverIds::new("ice-native-popover")))
}

pub fn popover_demo(open: bool, accent: Color) -> Element<'static, PopoverEvent> {
    let theme = theme(accent);
    popover(
        PopoverIds::new("ice-native-popover"),
        surface(text("Toggle popover"), SurfaceVariant::Default, &theme).padding([8, 12]),
        column![
            text("Native overlay"),
            text("Advanced collision and focus behavior stays in Rust.")
        ]
        .spacing(8),
        open,
        |event| event,
        &theme,
    )
    .placement(Placement::Right)
    .alignment(Alignment::Start)
    .side_offset(8.0)
    .width(280.0)
    .into()
}

fn button_variant(variant: &str) -> ButtonVariant {
    match variant {
        "destructive" => ButtonVariant::Destructive,
        "outline" => ButtonVariant::Outline,
        "secondary" => ButtonVariant::Secondary,
        "ghost" => ButtonVariant::Ghost,
        "link" => ButtonVariant::Link,
        _ => ButtonVariant::Default,
    }
}

fn progress_variant(variant: &str) -> ProgressVariant {
    match variant {
        "success" => ProgressVariant::Success,
        "warning" => ProgressVariant::Warning,
        "destructive" => ProgressVariant::Destructive,
        _ => ProgressVariant::Default,
    }
}

fn select_groups() -> [SelectGroup<String>; 2] {
    [
        SelectGroup::new(
            "inputs",
            vec![
                SelectOption::new("input", "input".to_owned(), "Input"),
                SelectOption::new("select", "select".to_owned(), "Select"),
                SelectOption::new("calendar", "calendar".to_owned(), "Calendar"),
            ],
        )
        .label("Inputs"),
        SelectGroup::new(
            "overlays",
            vec![
                SelectOption::new("dialog", "dialog".to_owned(), "Dialog"),
                SelectOption::new("popover", "popover".to_owned(), "Popover"),
            ],
        )
        .label("Overlays"),
    ]
}

fn dropdown_entries() -> Vec<MenuEntry> {
    vec![
        MenuItem::new("new", "New file").shortcut("⌘N").into(),
        MenuItem::new("open", "Open…").shortcut("⌘O").into(),
        MenuItem::new("share", "Share")
            .submenu(vec![
                MenuItem::new("copy-link", "Copy link").into(),
                MenuItem::new("invite", "Invite people").into(),
            ])
            .into(),
        MenuEntry::separator("file-separator"),
        MenuItem::new("delete", "Move to trash")
            .shortcut("⌫")
            .into(),
    ]
}

fn navigation_menu_infos() -> [NavigationMenuItemInfo; 3] {
    [
        NavigationMenuItemInfo::link("home"),
        NavigationMenuItemInfo::disclosure("components"),
        NavigationMenuItemInfo::link("docs"),
    ]
}

fn menubar_menus() -> Vec<MenubarMenu> {
    vec![
        MenubarMenu::new(
            "file",
            "File",
            vec![
                MenuItem::new("new", "New").shortcut("⌘N").into(),
                MenuItem::new("open", "Open…").shortcut("⌘O").into(),
                MenuEntry::separator("file-separator"),
                MenuItem::new("quit", "Quit").shortcut("⌘Q").into(),
            ],
        ),
        MenubarMenu::new(
            "edit",
            "Edit",
            vec![
                MenuItem::new("undo", "Undo").shortcut("⌘Z").into(),
                MenuItem::new("redo", "Redo").shortcut("⇧⌘Z").into(),
            ],
        ),
        MenubarMenu::new(
            "help",
            "Help",
            vec![MenuItem::new("docs", "Documentation").into()],
        ),
    ]
}

fn transcript_metadata() -> Vec<MessageScrollerItemMeta> {
    vec![
        MessageScrollerItemMeta::new("welcome"),
        MessageScrollerItemMeta::new("components").scroll_anchor(true),
        MessageScrollerItemMeta::new("tasks"),
        MessageScrollerItemMeta::new("latest"),
    ]
}

fn catalog_items(query: &str) -> Vec<String> {
    let query = query.to_lowercase();
    [
        "Button", "Input", "Dialog", "Calendar", "Chart", "Sidebar", "Sonner",
    ]
    .into_iter()
    .filter(|row| row.to_lowercase().contains(&query))
    .map(str::to_owned)
    .collect()
}

fn theme(accent: Color) -> crate::ui::theme::Theme {
    let mut theme = LIGHT;
    theme.palette.primary = accent;
    theme.palette.ring = accent;
    theme.palette.primary_foreground =
        if accent.relative_contrast(Color::WHITE) >= accent.relative_contrast(Color::BLACK) {
            Color::WHITE
        } else {
            Color::BLACK
        };
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_accept_safe_fallback_variants() {
        assert_eq!(button_variant("unknown"), ButtonVariant::Default);
        assert_eq!(progress_variant("unknown"), ProgressVariant::Default);

        let accent = Color::from_rgb8(37, 99, 235);
        assert_eq!(theme(accent).palette.primary, accent);

        let _: Element<'_, String> = input_otp("otp", "", false, false, Color::BLACK);
        let _: Element<'_, ()> = spinner(-1, false, Color::BLACK);
        let _ = button_style(
            &iced::Theme::Light,
            iced::widget::button::Status::Active,
            "default".to_owned(),
            Color::BLACK,
        );
        let _ = checkbox_style(
            &iced::Theme::Light,
            iced::widget::checkbox::Status::Active { is_checked: false },
            Color::BLACK,
        );
        let _ = input_style(
            &iced::Theme::Light,
            iced::widget::text_input::Status::Active,
            false,
            Color::BLACK,
        );
        let _ = progress_style(&iced::Theme::Light, "default".to_owned(), Color::BLACK);
        let calendar = calendar_state();
        let _: Element<'_, CalendarEvent> = super::calendar(&calendar, Color::BLACK);
        let _: iced::Task<CalendarState> = calendar_apply(
            calendar.clone(),
            CalendarEvent::MonthChanged(calendar.month()),
        );
        let date_picker = date_picker_state();
        let _: Element<'_, DatePickerEvent> = super::date_picker(&date_picker, Color::BLACK);
        let _: Element<'_, Option<ChartHit>> = chart(None, Color::BLACK);
        let command = command_state();
        let _: Element<'_, CommandEvent> = super::command(&command, Color::BLACK);
        let select = select_state();
        let _: Element<'_, SelectEvent> = super::select(&select, Color::BLACK);
        let dropdown = dropdown_menu_state();
        let _: Element<'_, DropdownMenuEvent> = dropdown_menu(&dropdown, Color::BLACK);
        let context = context_menu_state();
        let _: Element<'_, ContextMenuEvent> = context_menu(&context, Color::BLACK);
        let alert = alert_dialog_state();
        let _: Element<'_, AlertDialogEvent> = alert_dialog(&alert, Color::BLACK);
        let sidebar = sidebar_state();
        let _: Element<'_, SidebarEvent> = super::sidebar(&sidebar, Color::BLACK);
        let sonner = sonner_state();
        let _: Element<'_, SonnerEvent> = super::sonner(&sonner, Color::BLACK);
        let drawer = drawer_state();
        let _: Element<'_, DrawerEvent> = super::drawer(&drawer, Color::BLACK);
        let navigation = navigation_menu_state();
        let _: Element<'_, NavigationMenuEvent> = navigation_menu(&navigation, Color::BLACK);
        let menubar = menubar_state();
        let _: Element<'_, MenubarEvent> = super::menubar(&menubar, Color::BLACK);
        let _: Element<'_, ()> = hover_card(Color::BLACK);
        let _: Element<'_, Vec<f64>> = slider(&[25.0, 75.0], Color::BLACK);
        let _: Element<'_, String> = radio_group("default", Color::BLACK);
        let scroller = message_scroller_state();
        let _: Element<'_, MessageScrollerEvent> = message_scroller(&scroller, Color::BLACK);
        let _: Element<'_, Vec<f64>> = resizable_demo(&[0.25, 0.5, 0.25], Color::BLACK);
        let _: Element<'_, PopoverEvent> = popover_demo(false, Color::BLACK);
        let _: iced::Task<bool> = popover_apply(PopoverEvent::Open);
    }

    #[test]
    fn ice_owned_reducers_keep_navigation_and_notifications_controlled() {
        let sidebar = sidebar_apply(
            sidebar_state(),
            SidebarEvent::Select("components".to_owned()),
        );
        assert_eq!(sidebar.selected, "components");

        let sidebar = sidebar_apply(
            sidebar,
            SidebarEvent::Action(SidebarAction::Toggle(SidebarViewport::Desktop)),
        );
        assert!(!sidebar.navigation.expanded);

        let sonner = sonner_state();
        let initial = sonner.queue.len();
        let sonner = sonner_apply(sonner, SonnerEvent::Show);
        assert_eq!(sonner.queue.len(), initial + 1);

        assert_eq!(
            data_table_rows("a".to_owned(), "ascending".to_owned(), 0),
            ["Calendar", "Chart", "Dialog"]
        );
        assert!(data_table_can_next(String::new(), 0));
        assert!(!data_table_can_next("button".to_owned(), 0));
        assert_eq!(data_table_next_sort("ascending".to_owned()), "descending");
    }
}
