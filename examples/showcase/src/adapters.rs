//! Fixed retained-widget adapters for the component catalog.

use iced::alignment::{Horizontal, Vertical};
use iced::font::{Style as FontStyle, Weight};
use iced::widget::{column, container, row, text};
use iced::{Background, Border, Element, Font, Length};
use std::collections::HashMap;
use std::sync::Arc;
use ui_lang_components::ui::{
    alert_dialog::{
        AlertDialogActionVariant, AlertDialogEvent as UiAlertDialogEvent, AlertDialogFocus,
        alert_dialog as ui_alert_dialog, next_open as next_alert_dialog_open,
    },
    calendar::{CalendarSelection, Date, Month, controlled_calendar},
    chart::{
        CartesianCurve, CartesianKind, ChartColor, ChartConfig, ChartData, ChartDatum,
        SeriesConfig, cartesian_chart, companion_content, companion_model, legend_content,
        tooltip_content, tooltip_model,
    },
    command::{command as ui_command, command_group, command_item},
    context_menu::{ContextMenuIds, context_menu as ui_context_menu},
    data_grid::{
        AccessibilitySortDirection, DataGridCellId, DataGridColumn, DataGridConfig,
        DataGridEvent as UiDataGridEvent, DataGridId, DataGridState as UiDataGridState,
        data_grid as ui_data_grid,
    },
    data_table::{DataTableState, Sort, SortDirection},
    date_picker::{DateFormat, DatePickerIds, DatePickerValue, date_picker as ui_date_picker},
    direction::Direction,
    drawer::{
        DrawerEvent as UiDrawerEvent, DrawerState as UiDrawerState, drawer as ui_drawer,
        drawer_panel,
    },
    dropdown_menu::{DropdownMenuIds, dropdown_menu as ui_dropdown_menu},
    focus_control::FocusControl,
    hover_card::{HoverCardId, hover_card as ui_hover_card},
    input_otp::{OtpPattern, input_otp as ui_input_otp},
    log_timeline::{
        LogTimelineEvent as UiLogTimelineEvent, LogTimelineState as UiLogTimelineState,
        log_timeline as ui_log_timeline,
    },
    menu::{MenuEntry, MenuEvent, MenuGroup, MenuItem, MenuState},
    menubar::{MenubarMenu, MenubarState as UiMenubarState, menubar as ui_menubar},
    message_scroller::{
        MessageScrollerItemMeta, controlled_message_scroller, message_scroller_item,
    },
    modal::FocusScope,
    navigation_menu::{
        NavigationMenuItem, NavigationMenuItemInfo, navigation_menu as ui_navigation_menu,
        navigation_menu_list, navigation_menu_list_link,
    },
    popover::{Alignment, Placement, PopoverIds, next_open, popover},
    progress::ProgressVariant,
    radio_group::{RadioOrientation, focus_radio, radio_group as ui_radio_group, radio_option},
    resizable::resizable,
    select::{SelectGroup, SelectIds, SelectOption, select as ui_select},
    sidebar::{
        SIDEBAR_METRICS, SidebarAction, SidebarCollapsible, SidebarId, SidebarMenuButtonId,
        SidebarMetrics, SidebarState as UiSidebarState, SidebarVariant, SidebarViewport,
        sidebar as ui_sidebar, sidebar_footer, sidebar_group, sidebar_group_label, sidebar_header,
        sidebar_layout, sidebar_menu, sidebar_menu_badge, sidebar_menu_button,
        sidebar_menu_button_content,
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
    tree_view::{
        TreeViewConfig, TreeViewEvent as UiTreeViewEvent, TreeViewId, TreeViewNode,
        TreeViewState as UiTreeViewState, tree_view as ui_tree_view,
    },
    virtual_list::{
        VirtualListConfig, VirtualListEvent as UiVirtualListEvent, VirtualListId,
        VirtualListState as UiVirtualListState, virtual_list as ui_virtual_list,
    },
};
use ui_lang_runtime::{Role, StableId, accessible};

const ACTION_HEIGHT: f32 = 36.0;
const DEMO_STAGE_HEIGHT: f32 = 208.0;
const SIDEBAR_STAGE_HEIGHT: f32 = 404.0;
const DATA_TABLE_PAGE_SIZE: usize = 3;

pub fn initial_showcase_page() -> String {
    if std::env::var(ui_lang_runtime::dev::REQUIRED_DRAW_ENV)
        .is_ok_and(|required| required.split(',').any(|name| name == "virtual-list"))
    {
        "retained".to_owned()
    } else {
        "components".to_owned()
    }
}

pub use ui_lang_components::ui::{
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

pub type CommandEvent = ui_lang_components::ui::command::CommandEvent<String>;
pub type SelectEvent = ui_lang_components::ui::select::SelectEvent<String>;
pub type LogTimelineEvent = UiLogTimelineEvent<u64>;
pub type VirtualListEvent = UiVirtualListEvent<u64>;
pub type TreeViewEvent = UiTreeViewEvent<u64>;

#[derive(Debug, Clone, PartialEq)]
pub enum DataGridEvent {
    Grid(UiDataGridEvent<u64, u8>),
    EditChanged(String),
}

#[derive(Debug)]
pub struct LogTimelineState {
    timeline: UiLogTimelineState<u64>,
    rows: Arc<[u64]>,
}

impl Clone for LogTimelineState {
    fn clone(&self) -> Self {
        Self {
            timeline: self.timeline.update_snapshot(),
            rows: Arc::clone(&self.rows),
        }
    }
}

#[derive(Debug)]
pub struct VirtualListState {
    list: UiVirtualListState<u64>,
    items: Arc<[u64]>,
}

impl Clone for VirtualListState {
    fn clone(&self) -> Self {
        Self {
            list: self.list.update_snapshot(),
            items: Arc::clone(&self.items),
        }
    }
}

#[derive(Debug, Clone)]
struct TreeNode {
    key: u64,
    parent: Option<u64>,
    has_children: bool,
}

#[derive(Debug)]
pub struct TreeViewState {
    tree: UiTreeViewState<u64>,
    items: Arc<[TreeNode]>,
    renames: Arc<HashMap<u64, String>>,
    focus_target: TreeFocusTarget,
}

#[derive(Debug, Clone, Copy)]
enum TreeFocusTarget {
    None,
    Rename,
    Tree,
}

impl Clone for TreeViewState {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.update_snapshot(),
            items: Arc::clone(&self.items),
            renames: Arc::clone(&self.renames),
            focus_target: self.focus_target,
        }
    }
}

impl TreeViewState {
    fn label(&self, item: &TreeNode) -> String {
        self.renames
            .get(&item.key)
            .cloned()
            .unwrap_or_else(|| tree_label(item))
    }
}

#[derive(Debug)]
pub struct DataGridState {
    grid: UiDataGridState<u64, u8>,
    rows: Arc<[u64]>,
    columns: Arc<[DataGridColumn<u8>]>,
    sort: Option<(u8, AccessibilitySortDirection)>,
    edits: Arc<HashMap<(u64, u8), String>>,
    draft: String,
    focus_target: DataGridFocusTarget,
}

#[derive(Debug, Clone, Copy)]
enum DataGridFocusTarget {
    None,
    Editor,
    Grid,
}

impl Clone for DataGridState {
    fn clone(&self) -> Self {
        Self {
            grid: self.grid.update_snapshot(),
            rows: Arc::clone(&self.rows),
            columns: Arc::clone(&self.columns),
            sort: self.sort,
            edits: Arc::clone(&self.edits),
            draft: self.draft.clone(),
            focus_target: self.focus_target,
        }
    }
}

impl DataGridState {
    fn value(&self, row: u64, column: u8) -> String {
        self.edits
            .get(&(row, column))
            .cloned()
            .unwrap_or_else(|| match column {
                0 => format!("Repository item {row:05}"),
                1 => format!("#{row:05}"),
                2 => {
                    if row.is_multiple_of(3) {
                        "Ready".to_owned()
                    } else {
                        "Review".to_owned()
                    }
                }
                3 => format!("owner-{}", row % 17),
                _ => format!("value {row}:{}", column + 1),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItem {
    pub name: String,
    pub source: String,
}

fn semantic<'a, Message: Clone + 'static>(
    content: impl Into<Element<'a, Message>>,
    key: &str,
    role: Role,
) -> Element<'a, Message> {
    accessible(content, StableId::new(key), role)
        .logical_id(key)
        .into()
}

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
    menu: ui_lang_components::ui::menu::MenuState,
    selected: Option<String>,
    open: bool,
}

#[derive(Debug, Clone)]
pub struct DropdownMenuState {
    ids: DropdownMenuIds,
    entries: Vec<MenuEntry>,
    menu: MenuState,
    open: bool,
    last_action: String,
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    ids: ContextMenuIds,
    entries: Vec<MenuEntry>,
    menu: MenuState,
    open: bool,
    anchor: Option<iced::Point>,
    last_action: String,
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
    elapsed: std::time::Duration,
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
    last_action: String,
}

#[derive(Debug, Clone)]
pub enum DrawerEvent {
    Open,
    Close,
    Drawer(UiDrawerEvent),
}

pub fn checkbox_style(
    _iced_theme: &iced::Theme,
    status: iced::widget::checkbox::Status,
) -> iced::widget::checkbox::Style {
    ui_lang_components::ui::checkbox::style(
        ui_lang_components::ui::checkbox::CheckboxTokens::from(&theme()),
        status,
    )
}

pub fn switch(id: &str, checked: bool, disabled: bool) -> Element<'static, bool> {
    let control = ui_switch(
        iced::widget::Id::from(id.to_owned()),
        checked,
        !checked,
        &theme(),
    )
    .disabled(disabled);
    accessible(control, StableId::new(id), Role::Switch)
        .logical_id(id)
        .label("Product notifications")
        .checked(checked)
        .disabled(disabled)
        .on_activate_maybe((!disabled).then_some(!checked))
        .into()
}

pub fn progress_style(_iced_theme: &iced::Theme) -> iced::widget::progress_bar::Style {
    ui_lang_components::ui::progress::style(&theme(), ProgressVariant::Default)
}

pub fn progress_success_style(_iced_theme: &iced::Theme) -> iced::widget::progress_bar::Style {
    ui_lang_components::ui::progress::style(&theme(), ProgressVariant::Success)
}

pub fn progress_warning_style(_iced_theme: &iced::Theme) -> iced::widget::progress_bar::Style {
    ui_lang_components::ui::progress::style(&theme(), ProgressVariant::Warning)
}

pub fn progress_destructive_style(_iced_theme: &iced::Theme) -> iced::widget::progress_bar::Style {
    ui_lang_components::ui::progress::style(&theme(), ProgressVariant::Destructive)
}

pub fn input_otp<'a>(
    id: &str,
    value: &'a str,
    invalid: bool,
    disabled: bool,
) -> Element<'a, String> {
    let input = ui_input_otp(value, 6, OtpPattern::Digits, String::from, &theme())
        .groups([3, 3])
        .id(iced::widget::Id::from(id.to_owned()))
        .invalid(invalid)
        .disabled(disabled);
    let semantic_id = format!("{id}-semantic");
    accessible(input, StableId::new(semantic_id.clone()), Role::TextInput)
        .logical_id(semantic_id)
        .label("Verification code")
        .value(value)
        .disabled(disabled)
        .into()
}

pub fn spinner(frame: i64, reduced_motion: bool) -> Element<'static, ()> {
    let spinner = ui_spinner(
        frame.rem_euclid(ui_lang_components::ui::spinner::FRAME_COUNT.into()) as u8,
        reduced_motion,
        &theme(),
    );
    semantic(spinner, "showcase-spinner", Role::ProgressIndicator)
}

/// A borrowed-`editor` extern component. The interesting part is the
/// SIGNATURE: `&editor` fed from component state is the shape that, before
/// the read became a place expression, borrowed through a temporary and
/// failed to compile in the generated crate (E0716). rustc is the only gate
/// that catches it, so the showcase mounts one.
pub fn draft_length(document: &iced::widget::text_editor::Content) -> Element<'_, ()> {
    let label = format!("{} characters", document.text().trim().chars().count());
    let text: Element<'_, ()> = iced::widget::text(label).size(12.0).into();
    semantic(text, "showcase-draft-length", Role::Label)
}

fn current_date() -> Option<Date> {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs()
        / 86_400;
    let z = i64::try_from(days).ok()?.checked_add(719_468)?;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let day_of_year = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month + 2) / 5 + 1;
    let month = month + if month < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    Date::new(
        year.try_into().ok()?,
        month.try_into().ok()?,
        day.try_into().ok()?,
    )
    .ok()
}

pub fn calendar_state() -> CalendarState {
    let today = current_date();
    CalendarState::new(
        today.map_or_else(
            || Month::new(2026, 7).expect("fallback calendar month is valid"),
            Date::month,
        ),
        CalendarSelection::Single(None),
    )
    .focused(today)
}

pub fn calendar_apply(mut state: CalendarState, event: CalendarEvent) -> iced::Task<CalendarState> {
    state.apply(&event);
    iced::Task::done(state).chain(event.focus_task("ice-default-calendar"))
}

pub fn calendar(state: &CalendarState) -> Element<'static, CalendarEvent> {
    let calendar = controlled_calendar("ice-default-calendar", state, |event| event, &theme())
        .today(current_date())
        .month_dropdown(true)
        .year_dropdown(true)
        .year_range(
            current_date().map_or(2024, |date| date.year() - 2),
            current_date().map_or(2028, |date| date.year() + 2),
        );
    semantic(calendar, "ice-default-calendar", Role::Group)
}

pub fn date_picker_state() -> DatePickerState {
    let today = current_date();
    DatePickerState {
        ids: DatePickerIds::new("ice-default"),
        month: today.map_or_else(
            || Month::new(2026, 7).expect("fallback date picker month is valid"),
            Date::month,
        ),
        focused: today,
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

pub fn date_picker(state: &DatePickerState) -> Element<'static, DatePickerEvent> {
    let picker = ui_date_picker(
        state.ids.clone(),
        state.month,
        state.focused,
        &state.value,
        state.open,
        |event| event,
        &theme(),
    )
    .today(current_date())
    .month_dropdown(true)
    .year_dropdown(true)
    .year_range(
        current_date().map_or(2024, |date| date.year() - 2),
        current_date().map_or(2028, |date| date.year() + 2),
    )
    .placeholder("Choose a date")
    .format(DateFormat::MonthDayYear)
    .width(272.0);
    accessible(
        picker,
        StableId::new("ice-default-date-picker"),
        Role::DateInput,
    )
    .logical_id("ice-default-date-picker")
    .label("Choose a date")
    .value_maybe(state.value.anchor().map(|date| date.to_string()))
    .into()
}

pub fn aspect_ratio_demo() -> Element<'static, ()> {
    let theme = theme();
    let view = ui_lang_components::ui::aspect_ratio::aspect_ratio(16.0 / 9.0, move || {
        container(
            text("16 / 9")
                .size(20)
                .color(theme.palette.primary)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    })
    .width(Length::Fill);
    semantic(view, "showcase-aspect-ratio", Role::GenericContainer)
}

pub fn chart(hovered: Option<ChartHit>) -> Element<'static, Option<ChartHit>> {
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
        ChartDatum::new(4.0, "May")
            .with_value("desktop", 289.0)
            .with_value("mobile", 210.0),
        ChartDatum::new(5.0, "Jun")
            .with_value("desktop", 342.0)
            .with_value("mobile", 238.0),
    ]);
    let theme = theme();
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
        row![
            column![
                text("Traffic overview")
                    .size(16)
                    .font(ui_font(Weight::Bold)),
                text("Monthly unique visitors")
                    .size(11)
                    .font(ui_font(Weight::Normal))
                    .color(theme.palette.muted_foreground),
            ]
            .spacing(2)
            .width(Length::Fill),
            column![
                text("+20.1%")
                    .size(16)
                    .font(ui_font(Weight::Bold))
                    .color(theme.palette.success),
                text("from last month")
                    .size(10)
                    .font(italic_font())
                    .color(theme.palette.muted_foreground),
            ]
            .spacing(2)
            .align_x(iced::Alignment::End),
        ]
        .align_y(iced::Alignment::Center),
        cartesian_chart(&config, &data, &theme)
            .kind(CartesianKind::Area { points: false })
            .curve(CartesianCurve::Monotone)
            .hovered(hovered)
            .on_hover(|hit| hit)
            .height(228),
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

pub fn command(state: &CommandState) -> Element<'static, CommandEvent> {
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

    let palette = ui_command(
        "ice-default-command",
        state,
        groups,
        |event| event,
        &theme(),
    )
    .results_height(180.0)
    .into_element();
    let status = state.active().map_or_else(
        || "No command selected".to_owned(),
        |item| format!("Active: {item}"),
    );
    semantic(
        column![palette, text(status).size(12)].spacing(8),
        "ice-default-command",
        Role::Search,
    )
}

pub fn select_state() -> SelectState {
    let groups = select_groups();
    SelectState {
        ids: SelectIds::new("ice-default"),
        menu: ui_lang_components::ui::menu::MenuState::initial(
            &ui_lang_components::ui::select::select_entries(&groups, None),
        ),
        selected: None,
        open: false,
    }
}

pub fn select_apply(mut state: SelectState, event: SelectEvent) -> iced::Task<SelectState> {
    state.open = event.open(state.open);
    if let SelectEvent::Selected(value) = &event {
        state.selected = Some(value.clone());
    }
    if let SelectEvent::Menu(ui_lang_components::ui::menu::MenuEvent::StateChanged(menu)) = &event {
        state.menu.clone_from(menu);
    }
    let focus = event.focus_task(&state.ids, &select_groups(), &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn select<'a>(state: &'a SelectState) -> Element<'a, SelectEvent> {
    let control = ui_select(
        state.ids.clone(),
        select_groups(),
        state.selected.clone(),
        "Choose a component",
        &state.menu,
        state.open,
        |event| event,
        &theme(),
    )
    .width(272.0)
    .content_width(272.0);
    let selected = state
        .selected
        .as_deref()
        .map_or("No component selected", |value| value);
    accessible(
        column![control, text(format!("Selected: {selected}"))].spacing(4),
        StableId::new("ice-default-select"),
        Role::ComboBox,
    )
    .logical_id("ice-default-select")
    .label("Choose a component")
    .value(selected)
    .into()
}

pub fn dropdown_menu_state() -> DropdownMenuState {
    let entries = dropdown_entries();
    DropdownMenuState {
        ids: DropdownMenuIds::new("ice-default"),
        menu: MenuState::initial(&entries),
        entries,
        open: false,
        last_action: String::new(),
    }
}

pub fn dropdown_menu_is_open(state: &DropdownMenuState) -> bool {
    state.open
}

pub fn dropdown_menu_apply(
    mut state: DropdownMenuState,
    event: DropdownMenuEvent,
) -> iced::Task<DropdownMenuState> {
    state.open = event.open(state.open);
    if let DropdownMenuEvent::Menu(MenuEvent::StateChanged(menu)) = &event {
        state.menu.clone_from(menu);
    }
    if let DropdownMenuEvent::Menu(MenuEvent::Activated(action)) = &event {
        state.last_action.clone_from(&action.id);
    }
    let focus = event.focus_task(&state.ids, &state.entries, &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn dropdown_menu(state: &DropdownMenuState) -> Element<'_, DropdownMenuEvent> {
    let theme = theme();
    let menu = ui_dropdown_menu(
        state.ids.clone(),
        surface(text("Open menu"), SurfaceVariant::Default, &theme)
            .height(ACTION_HEIGHT)
            .padding([0, 12])
            .align_y(Vertical::Center),
        &state.entries,
        &state.menu,
        state.open,
        |event| event,
        &theme,
    )
    .width(248.0);
    let status = if state.last_action.is_empty() {
        "No menu action"
    } else {
        state.last_action.as_str()
    };
    semantic(
        column![menu, text(format!("Action: {status}"))].spacing(4),
        "ice-default-dropdown",
        Role::Button,
    )
}

pub fn context_menu_state() -> ContextMenuState {
    let entries = dropdown_entries();
    ContextMenuState {
        ids: ContextMenuIds::new("ice-default"),
        menu: MenuState::initial(&entries),
        entries,
        open: false,
        anchor: None,
        last_action: String::new(),
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
    if let ContextMenuEvent::Menu(MenuEvent::Activated(action)) = &event {
        state.last_action.clone_from(&action.id);
    }
    let focus = event.focus_task(&state.ids, &state.entries, &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn context_menu(state: &ContextMenuState) -> Element<'_, ContextMenuEvent> {
    let theme = theme();
    let menu = ui_context_menu(
        state.ids.clone(),
        surface(
            text("Right-click or touch this region"),
            SurfaceVariant::Muted,
            &theme,
        )
        .height(ACTION_HEIGHT)
        .padding([0, 16])
        .align_y(Vertical::Center),
        &state.entries,
        &state.menu,
        state.open,
        state.anchor,
        |event| event,
        &theme,
    )
    .width(248.0);
    let status = if state.last_action.is_empty() {
        "No context action"
    } else {
        state.last_action.as_str()
    };
    semantic(
        column![menu, text(format!("Action: {status}"))].spacing(4),
        "ice-default-context-menu",
        Role::Button,
    )
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

pub fn alert_dialog_is_open(state: &AlertDialogState) -> bool {
    state.open
}

pub fn alert_dialog(state: &AlertDialogState) -> Element<'static, AlertDialogEvent> {
    let theme = theme();
    let trigger_action: Element<'static, AlertDialogEvent> = FocusControl::new(
        state.focus.restore().clone(),
        surface(
            text("Delete component")
                .size(12)
                .font(ui_font(Weight::Semibold)),
            SurfaceVariant::Muted,
            &theme,
        )
        .height(ACTION_HEIGHT)
        .padding([0, 12])
        .align_y(Vertical::Center),
        AlertDialogEvent::Open,
        &theme,
    )
    .into();
    let trigger_content = surface(
        row![
            column![
                text("DESTRUCTIVE FLOW")
                    .size(9)
                    .font(ui_font(Weight::Semibold))
                    .color(theme.palette.destructive),
                text("Safe focus and explicit confirmation")
                    .size(12)
                    .font(ui_font(Weight::Normal)),
            ]
            .spacing(4)
            .width(Length::Fill),
            trigger_action,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
        SurfaceVariant::Card,
        &theme,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(12)
    .align_y(Vertical::Center);
    let dialog = ui_alert_dialog(
        trigger_content,
        state.open,
        &state.focus,
        "Delete this component?",
        "This action cannot be undone. The alert keeps the safest action focused.",
        "Cancel",
        "Delete",
        AlertDialogActionVariant::Destructive,
        AlertDialogEvent::Dialog,
        &theme,
    );
    semantic(dialog, "ice-default-alert-dialog", Role::AlertDialog)
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

pub fn sidebar(state: &SidebarState) -> Element<'static, SidebarEvent> {
    let theme = theme();
    let collapsed = state
        .navigation
        .is_collapsed(SidebarViewport::Desktop, SidebarCollapsible::Icon);
    let items = [
        ("overview", "01", "Overview", Some("LIVE")),
        ("components", "02", "Components", Some("18")),
        ("reports", "03", "Reports", Some("4")),
        ("settings", "04", "Settings", None),
    ]
    .map(|(id, icon, label, badge)| {
        let active = state.selected == id;
        let leading: Element<'static, SidebarEvent> = surface(
            container(text(icon).size(10))
                .width(24)
                .height(24)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            SurfaceVariant::Muted,
            &theme,
        )
        .width(24)
        .height(24)
        .into();
        let trailing =
            badge.map(|badge| sidebar_menu_badge(text(badge).size(9), active, &theme).into());
        let content = sidebar_menu_button_content(
            Some(leading),
            label,
            trailing,
            collapsed,
            Direction::LeftToRight,
            &theme,
        );
        sidebar_menu_button(
            SidebarMenuButtonId::new(id),
            content,
            SidebarEvent::Select(id.to_owned()),
            &theme,
        )
        .active(active)
        .collapsed(collapsed)
        .tooltip(text(label))
        .into()
    });
    let brand_theme = theme;
    let brand: Element<'static, SidebarEvent> = container(
        text("D")
            .size(13)
            .font(ui_font(Weight::Bold))
            .color(theme.palette.primary_foreground),
    )
    .width(32)
    .height(32)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .style(move |_iced_theme| iced::widget::container::Style {
        background: Some(Background::Color(brand_theme.palette.primary)),
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into();
    let toggle: Element<'static, SidebarEvent> = FocusControl::new(
        iced::widget::Id::from("ice-sidebar-demo-toggle"),
        surface(
            container(text(if collapsed { "›" } else { "‹" }).size(15))
                .width(32)
                .height(32)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            SurfaceVariant::Muted,
            &theme,
        )
        .width(32)
        .height(32),
        SidebarEvent::Action(SidebarAction::Toggle(SidebarViewport::Desktop)),
        &theme,
    )
    .into();
    let header: Element<'static, SidebarEvent> = if collapsed {
        column![brand, toggle]
            .spacing(8)
            .width(Length::Fill)
            .align_x(iced::Alignment::Center)
            .into()
    } else {
        row![
            brand,
            column![
                text("Ducktape").size(13).font(ui_font(Weight::Bold)),
                text("Component lab")
                    .size(10)
                    .font(italic_font())
                    .color(theme.palette.muted_foreground),
            ]
            .spacing(1)
            .width(Length::Fill),
            toggle,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .into()
    };
    let footer_avatar: Element<'static, SidebarEvent> = surface(
        container(text("UI").size(10))
            .width(28)
            .height(28)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
        SurfaceVariant::Muted,
        &theme,
    )
    .width(28)
    .height(28)
    .into();
    let footer: Element<'static, SidebarEvent> = if collapsed {
        container(footer_avatar)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .into()
    } else {
        row![
            footer_avatar,
            column![
                text("Preview workspace").size(11),
                text("Local").size(9).color(theme.palette.muted_foreground),
            ]
            .spacing(1),
        ]
        .spacing(9)
        .align_y(iced::Alignment::Center)
        .into()
    };
    let navigation = column![
        sidebar_group_label(
            text("WORKSPACE").size(9),
            collapsed,
            Direction::LeftToRight,
            &theme,
        ),
        sidebar_menu(items),
    ];
    let metrics = SidebarMetrics {
        desktop_width: 220.0,
        icon_width: 72.0,
        ..SIDEBAR_METRICS
    };
    let panel = ui_sidebar(
        SidebarId::new("ice-default"),
        state.navigation,
        sidebar_group(navigation),
        SidebarEvent::Action(SidebarAction::Toggle(SidebarViewport::Desktop)),
        &theme,
    )
    .header(sidebar_header(header))
    .footer(sidebar_footer(footer))
    .variant(SidebarVariant::Floating)
    .collapsible(SidebarCollapsible::Icon)
    .rail(false)
    .metrics(metrics);
    let (title, description, count, status) = match state.selected.as_str() {
        "components" => (
            "Components",
            "Browse the primitives that make up this workspace.",
            "18 ready",
            "Synced",
        ),
        "reports" => (
            "Reports",
            "Review visual checks, interaction coverage, and build health.",
            "42 checks",
            "Passing",
        ),
        "settings" => (
            "Settings",
            "Keep workspace preferences explicit and local.",
            "3 groups",
            "Saved",
        ),
        _ => (
            "Overview",
            "A compact snapshot of the current component workspace.",
            "18 ready",
            "Healthy",
        ),
    };
    let main = surface(
        column![
            text("ACTIVE ROUTE")
                .size(9)
                .font(ui_font(Weight::Semibold))
                .color(theme.palette.muted_foreground),
            text(title).size(20).font(ui_font(Weight::Bold)),
            text(description)
                .size(11)
                .font(italic_font())
                .color(theme.palette.muted_foreground),
            row![
                surface(
                    column![
                        text("COMPONENTS")
                            .size(8)
                            .font(ui_font(Weight::Semibold))
                            .color(theme.palette.muted_foreground),
                        text(count).size(13).font(ui_font(Weight::Bold)),
                    ]
                    .spacing(4),
                    SurfaceVariant::Card,
                    &theme,
                )
                .width(Length::Fill)
                .padding(10),
                surface(
                    column![
                        text("BUILD")
                            .size(8)
                            .font(ui_font(Weight::Semibold))
                            .color(theme.palette.muted_foreground),
                        text(status).size(13).font(ui_font(Weight::Bold)),
                    ]
                    .spacing(4),
                    SurfaceVariant::Card,
                    &theme,
                )
                .width(Length::Fill)
                .padding(10),
            ]
            .spacing(8),
            surface(
                column![
                    row![
                        container(
                            text("RELEASE READINESS")
                                .size(8)
                                .font(ui_font(Weight::Semibold))
                                .color(theme.palette.muted_foreground),
                        )
                        .width(Length::Fill),
                        text("92%")
                            .size(10)
                            .font(ui_font(Weight::Bold))
                            .color(theme.palette.success),
                    ]
                    .align_y(iced::Alignment::Center),
                    container(
                        row![
                            container(iced::widget::Space::new())
                                .width(Length::FillPortion(92))
                                .height(6)
                                .style(move |_iced_theme| iced::widget::container::Style {
                                    background: Some(Background::Color(theme.palette.primary)),
                                    border: Border {
                                        radius: 999.0.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                            iced::widget::Space::new().width(Length::FillPortion(8)),
                        ]
                        .spacing(0),
                    )
                    .width(Length::Fill)
                    .height(6)
                    .style(move |_iced_theme| iced::widget::container::Style {
                        background: Some(Background::Color(theme.palette.muted)),
                        border: Border {
                            radius: 999.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    text("Interactions, visual states, and accessibility evidence are current.")
                        .size(10)
                        .font(italic_font())
                        .color(theme.palette.muted_foreground),
                ]
                .spacing(7),
                SurfaceVariant::Card,
                &theme,
            )
            .padding(10),
            surface(
                column![
                    row![
                        container(
                            text("RECENT ACTIVITY")
                                .size(8)
                                .font(ui_font(Weight::Semibold))
                                .color(theme.palette.muted_foreground),
                        )
                        .width(Length::Fill),
                        text("LOCAL").size(8).color(theme.palette.muted_foreground),
                    ]
                    .align_y(iced::Alignment::Center),
                    row![
                        container(text("Component catalog").size(11)).width(Length::Fill),
                        text("Ready").size(10).color(theme.palette.muted_foreground),
                    ]
                    .align_y(iced::Alignment::Center),
                    row![
                        container(text("Typography audit").size(11)).width(Length::Fill),
                        text("Updated")
                            .size(10)
                            .color(theme.palette.muted_foreground),
                    ]
                    .align_y(iced::Alignment::Center),
                    row![
                        container(text("Accessibility tree").size(11)).width(Length::Fill),
                        text("Passed")
                            .size(10)
                            .color(theme.palette.muted_foreground),
                    ]
                    .align_y(iced::Alignment::Center),
                    row![
                        container(text("Navigation state").size(11)).width(Length::Fill),
                        text("Controlled")
                            .size(10)
                            .color(theme.palette.muted_foreground),
                    ]
                    .align_y(iced::Alignment::Center),
                ]
                .spacing(8),
                SurfaceVariant::Card,
                &theme,
            )
            .height(Length::Fill)
            .padding(10),
        ]
        .spacing(8),
        SurfaceVariant::Muted,
        &theme,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(16);

    let layout = container(sidebar_layout(
        main,
        panel,
        state.navigation,
        SidebarViewport::Desktop,
        Default::default(),
        SidebarEvent::Action(SidebarAction::CloseMobile),
        &theme,
    ))
    .height(SIDEBAR_STAGE_HEIGHT);
    semantic(layout, "ice-default-sidebar", Role::Navigation)
}

pub fn sonner_state() -> SonnerState {
    let mut queue = UiSonnerState::new(2, ToastPlacement::TopRight);
    let elapsed = std::time::Duration::ZERO;
    queue.set_default_duration(std::time::Duration::from_secs(4));
    queue.info("Ice owns this notification queue.", elapsed);
    SonnerState {
        queue,
        shown: 1,
        elapsed,
    }
}

pub fn sonner_apply(mut state: SonnerState, event: SonnerEvent) -> SonnerState {
    match event {
        SonnerEvent::Show => {
            state.shown = state.shown.saturating_add(1);
            state.queue.success(
                format!("Default notification #{}", state.shown),
                state.elapsed,
            );
        }
        SonnerEvent::Toast(event) => {
            state.queue.update(event, state.elapsed);
        }
    }
    state
}

pub fn sonner(state: &SonnerState) -> Element<'_, SonnerEvent> {
    let theme = theme();
    let count_label = if state.shown == 1 {
        "1 notification sent".to_owned()
    } else {
        format!("{} notifications sent", state.shown)
    };
    let underlay = container(
        column![
            iced::widget::Space::new().height(Length::Fill),
            text(count_label).size(12),
            ui_lang_components::ui::button::button("Show notification", &theme)
                .height(ACTION_HEIGHT)
                .on_press(SonnerEvent::Show),
        ]
        .spacing(12),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(16);

    let view = iced::widget::Stack::new()
        .push(underlay)
        .push(ui_sonner(&state.queue, SonnerEvent::Toast, &theme))
        .width(Length::Fill)
        .height(DEMO_STAGE_HEIGHT);
    semantic(view, "ice-default-sonner", Role::Log)
}

pub fn sonner_set_reduced_motion(
    mut state: SonnerState,
    reduced_motion: bool,
) -> iced::Task<SonnerState> {
    state.queue.set_reduced_motion(reduced_motion);
    iced::Task::done(state)
}

pub fn sonner_tick(mut state: SonnerState) -> iced::Task<SonnerState> {
    state.elapsed = state
        .elapsed
        .saturating_add(std::time::Duration::from_secs(1));
    state.queue.tick(state.elapsed);
    iced::Task::done(state)
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

pub fn drawer(state: &DrawerState, reduced_motion: bool) -> Element<'static, DrawerEvent> {
    let theme = theme();
    let trigger_content = surface(
        row![
            column![
                text("BOTTOM SHEET")
                    .size(9)
                    .color(theme.palette.muted_foreground),
                text("Drag, Escape, and focus restoration enabled").size(12),
            ]
            .spacing(4)
            .width(Length::Fill),
            surface(text("Open drawer").size(12), SurfaceVariant::Muted, &theme)
                .height(ACTION_HEIGHT)
                .padding([0, 12])
                .align_y(Vertical::Center),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
        SurfaceVariant::Card,
        &theme,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(14)
    .align_y(Vertical::Center);
    let trigger = FocusControl::new(
        state.focus.restore().clone(),
        trigger_content,
        DrawerEvent::Open,
        &theme,
    );
    let close = FocusControl::new(
        state.focus.first().clone(),
        surface(text("Close"), SurfaceVariant::Muted, &theme)
            .height(ACTION_HEIGHT)
            .padding([0, 12])
            .align_y(Vertical::Center),
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

    let drawer = ui_drawer(
        trigger,
        &state.drawer,
        panel,
        &state.focus,
        DrawerEvent::Drawer,
        &theme,
    )
    .size(220.0)
    .reduced_motion(reduced_motion);
    semantic(drawer, "ice-default-drawer", Role::Dialog)
}

pub fn navigation_menu_state() -> NavigationMenuState {
    NavigationMenuState::initial(&navigation_menu_infos()).active("home")
}

pub fn navigation_menu_is_open(state: NavigationMenuState) -> bool {
    state.open.is_some()
}

pub fn navigation_menu_route(state: NavigationMenuState) -> String {
    match state.active.as_deref() {
        Some("components") => "Components".to_owned(),
        Some("docs") => "Documentation".to_owned(),
        _ => "Home".to_owned(),
    }
}

pub fn navigation_menu_apply(event: NavigationMenuEvent) -> iced::Task<NavigationMenuState> {
    let state = event.state().clone();
    iced::Task::done(state).chain(event.focus_task("ice-default-navigation"))
}

pub fn navigation_menu(state: &NavigationMenuState) -> Element<'static, NavigationMenuEvent> {
    let theme = theme();
    let activate = |id: &'static str| NavigationMenuEvent::LinkActivated {
        id: id.to_owned(),
        state: NavigationMenuState {
            active: Some(id.to_owned()),
            open: None,
            ..state.clone()
        },
    };
    let component_links = navigation_menu_list([
        navigation_menu_list_link(
            iced::widget::Id::from("ice-nav-inputs"),
            "Inputs & forms",
            "Fields, selection, validation, and OTP patterns.",
            activate("inputs"),
            Direction::LeftToRight,
            &theme,
        ),
        navigation_menu_list_link(
            iced::widget::Id::from("ice-nav-navigation"),
            "Navigation",
            "Menus, pagination, breadcrumbs, and sidebars.",
            activate("navigation"),
            Direction::LeftToRight,
            &theme,
        ),
        navigation_menu_list_link(
            iced::widget::Id::from("ice-nav-overlays"),
            "Overlays & feedback",
            "Dialogs, drawers, popovers, and notifications.",
            activate("overlays"),
            Direction::LeftToRight,
            &theme,
        ),
    ]);
    let components_content =
        row![
            surface(
                column![
                text("COMPONENT LIBRARY")
                    .size(9)
                    .font(ui_font(Weight::Semibold))
                    .color(theme.palette.primary),
                text("Build consistent product surfaces")
                    .size(17)
                    .font(ui_font(Weight::Bold)),
                text("Composable defaults with explicit state, focus, and accessibility contracts.")
                    .size(11)
                    .font(ui_font(Weight::Normal))
                    .color(theme.palette.muted_foreground),
                text("18 catalog examples · Rust + Ice")
                    .size(10)
                    .font(italic_font())
                    .color(theme.palette.muted_foreground),
            ]
                .spacing(7),
                SurfaceVariant::Muted,
                &theme,
            )
            .width(200)
            .padding(14),
            component_links,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Start);
    let menu = ui_navigation_menu(
        "ice-default-navigation",
        [
            NavigationMenuItem::link("home", "Home"),
            NavigationMenuItem::disclosure("components", "Components", components_content),
            NavigationMenuItem::link("docs", "Documentation"),
        ],
        state,
        |event| event,
        &theme,
    )
    .content_width(520.0);
    semantic(menu, "ice-default-navigation", Role::Navigation)
}

pub fn menubar_state() -> MenubarState {
    let menus = menubar_menus();
    MenubarState {
        bar: UiMenubarState::initial(&menus),
        menu: MenuState::initial(&menus[0].entries),
        menus,
        last_action: String::new(),
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
    if let MenubarEvent::Menu {
        event: MenuEvent::Activated(action),
        ..
    } = &event
    {
        state.last_action.clone_from(&action.id);
    }
    let focus = event.focus_task("ice-default-menubar", &state.menus, &state.menu);
    iced::Task::done(state).chain(focus)
}

pub fn menubar(state: &MenubarState) -> Element<'static, MenubarEvent> {
    let menu = ui_menubar(
        "ice-default-menubar",
        state.menus.clone(),
        &state.bar,
        &state.menu,
        |event| event,
        &theme(),
    );
    let status = if state.last_action.is_empty() {
        "No menu action"
    } else {
        state.last_action.as_str()
    };
    semantic(
        column![menu, text(format!("Action: {status}"))].spacing(4),
        "ice-default-menubar",
        Role::MenuBar,
    )
}

pub fn hover_card() -> Element<'static, bool> {
    let theme = theme();
    let card = ui_hover_card(
        HoverCardId::new("ice-default"),
        surface(
            text("Hover or focus profile"),
            SurfaceVariant::Muted,
            &theme,
        )
        .height(ACTION_HEIGHT)
        .padding([0, 12])
        .align_y(Vertical::Center),
        column![
            text("ducktape-ui").size(16),
            text("Default components authored from Ice."),
            ui_lang_components::ui::button::button("Open profile", &theme)
                .height(ACTION_HEIGHT)
                .on_press(true)
        ]
        .spacing(8),
        &theme,
    )
    .width(280.0);
    semantic(card, "ice-default-hover-card", Role::Group)
}

pub fn slider(values: &[f64]) -> Element<'static, Vec<f64>> {
    ui_slider(
        "ice-default-slider",
        values.iter().map(|value| *value as f32).collect::<Vec<_>>(),
        0.0..=100.0,
        1.0,
        |values| values.into_iter().map(f64::from).collect(),
        &theme(),
    )
    .into()
}

pub fn radio_group(selected: &str) -> Element<'static, String> {
    let theme = theme();
    let control = ui_radio_group(
        "ice-default-radio",
        ["default", "comfortable", "compact"]
            .map(|value| radio_option(value.to_owned(), value, &theme)),
        Some(selected.to_owned()),
        |value| value,
        &theme,
    )
    .orientation(RadioOrientation::Horizontal);
    accessible(
        control,
        StableId::new("ice-default-radio"),
        Role::RadioGroup,
    )
    .logical_id("ice-default-radio")
    .label("Density")
    .value(selected)
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
    MessageScrollerState::new("ice-default-transcript").auto_scroll(true)
}

fn message_scroller_settle(
    mut state: MessageScrollerState,
    event: MessageScrollerEvent,
) -> iced::Task<MessageScrollerState> {
    let followup = state.update(event);
    let next = state.clone();
    iced::Task::done(next)
        .chain(followup.then(move |event| message_scroller_settle(state.clone(), event)))
}

pub fn message_scroller_bootstrap(state: MessageScrollerState) -> iced::Task<MessageScrollerState> {
    message_scroller_settle(
        state,
        MessageScrollerEvent::ItemsChanged(transcript_metadata()),
    )
}

pub fn message_scroller_apply(
    state: MessageScrollerState,
    event: MessageScrollerEvent,
) -> iced::Task<MessageScrollerState> {
    message_scroller_settle(state, event)
}

pub fn message_scroller(state: &MessageScrollerState) -> Element<'_, MessageScrollerEvent> {
    let theme = theme();
    let view = controlled_message_scroller(
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
    .height(DEMO_STAGE_HEIGHT - 16.0);
    semantic(view, "ice-default-transcript", Role::Log)
}

fn virtual_list_config() -> VirtualListConfig {
    VirtualListConfig::new(32.0)
        .expect("showcase virtual-list geometry is valid")
        .overscan(3)
}

fn log_timeline_config() -> VirtualListConfig {
    VirtualListConfig::new(24.0)
        .expect("showcase log-timeline geometry is valid")
        .overscan(3)
}

pub fn log_timeline_state() -> LogTimelineState {
    let rows: Arc<[u64]> = (0..100_000_u64).collect::<Vec<_>>().into();
    let mut timeline = UiLogTimelineState::new(VirtualListId::new("showcase-log-timeline"));
    timeline
        .reconcile(&rows, |row| *row, log_timeline_config())
        .expect("showcase log-timeline keys are unique and append-only");
    LogTimelineState { timeline, rows }
}

pub fn log_timeline_apply(
    mut state: LogTimelineState,
    event: LogTimelineEvent,
) -> LogTimelineState {
    state.timeline.apply(event, log_timeline_config());
    state
}

pub fn log_timeline_append(mut state: LogTimelineState) -> LogTimelineState {
    let mut rows = Vec::from(state.rows.as_ref());
    rows.push(
        rows.last()
            .copied()
            .map_or(0, |last| last.saturating_add(1)),
    );
    state
        .timeline
        .reconcile(&rows, |row| *row, log_timeline_config())
        .expect("showcase appends preserve log history");
    state.rows = rows.into();
    state
}

pub fn log_timeline_resume(mut state: LogTimelineState) -> LogTimelineState {
    state
        .timeline
        .apply(UiLogTimelineEvent::ResumeTail, log_timeline_config());
    state
}

pub fn log_timeline(state: &LogTimelineState) -> Element<'_, LogTimelineEvent> {
    let theme = theme();
    let inspection = state.timeline.inspect(log_timeline_config());
    let status = if inspection.following_tail {
        "following"
    } else {
        "paused"
    };
    let summary = row![
        text(format!("{status} · {} unread", inspection.unread_count))
            .size(11)
            .font(ui_font(Weight::Semibold)),
        iced::widget::Space::new().width(Length::Fill),
        text(format!(
            "mounted {}..{}",
            inspection.list.mounted_range.start, inspection.list.mounted_range.end
        ))
        .size(10)
        .color(theme.palette.muted_foreground),
    ]
    .align_y(iced::Alignment::Center);
    let timeline = ui_log_timeline(
        &state.timeline,
        &state.rows,
        log_timeline_config(),
        "Build output",
        |row| *row,
        |row| format!("Build log line {row}"),
        |_, row, selected| {
            row![
                text(format!("{row:06}"))
                    .size(11)
                    .font(ui_font(Weight::Semibold)),
                text(format!("worker: completed build step {row}"))
                    .size(11)
                    .color(theme.palette.muted_foreground),
                iced::widget::Space::new().width(Length::Fill),
                text(if selected { "selected" } else { "" })
                    .size(10)
                    .color(theme.palette.primary),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .into()
        },
        |event| event,
        &theme,
    );
    column![summary, timeline]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn virtual_list_state() -> VirtualListState {
    let items: Arc<[u64]> = (0..100_000_u64).collect::<Vec<_>>().into();
    let mut list = UiVirtualListState::new(VirtualListId::new("showcase-virtual-list"));
    list.reconcile(&items, |item| *item, virtual_list_config())
        .expect("showcase virtual-list keys are unique");
    VirtualListState { list, items }
}

pub fn virtual_list_apply(
    mut state: VirtualListState,
    event: VirtualListEvent,
) -> VirtualListState {
    state
        .list
        .apply(event, &state.items, |item| *item, virtual_list_config());
    state
}

pub fn virtual_list(state: &VirtualListState) -> Element<'_, VirtualListEvent> {
    let theme = theme();
    let range = state
        .list
        .mounted_range(state.items.len(), virtual_list_config());
    let selected = state.list.selected().map_or_else(
        || "No row selected".to_owned(),
        |key| format!("Selected #{key}"),
    );
    let summary = row![
        text("100,000 keyed rows")
            .size(11)
            .font(ui_font(Weight::Semibold)),
        iced::widget::Space::new().width(Length::Fill),
        text(format!(
            "mounted {}..{} · {selected}",
            range.start, range.end
        ))
        .size(10)
        .color(theme.palette.muted_foreground),
    ]
    .align_y(iced::Alignment::Center);
    let list = ui_virtual_list(
        &state.list,
        &state.items,
        virtual_list_config(),
        "Repository results",
        |item| *item,
        |item| format!("Repository result {item}"),
        |index, item, selected| {
            row![
                text(format!("#{item:05}"))
                    .size(11)
                    .font(ui_font(Weight::Semibold)),
                text(format!("Repository result row {index}"))
                    .size(11)
                    .color(theme.palette.muted_foreground),
                iced::widget::Space::new().width(Length::Fill),
                text(if selected { "selected" } else { "" })
                    .size(10)
                    .color(theme.palette.primary),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .into()
        },
        |event| event,
        &theme,
    );
    column![summary, list]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn tree_view_config() -> TreeViewConfig {
    TreeViewConfig::new(28.0)
        .expect("showcase tree-view geometry is valid")
        .overscan(3)
        .indentation(14.0)
}

fn tree_node(item: &TreeNode) -> TreeViewNode<u64> {
    TreeViewNode {
        key: item.key,
        parent: item.parent,
        has_children: item.has_children,
        children_loaded: true,
    }
}

fn tree_label(item: &TreeNode) -> String {
    if item.has_children {
        format!("Folder {:02}", item.key / 1_000)
    } else {
        format!("File {:05}.rs", item.key)
    }
}

pub fn tree_view_state() -> TreeViewState {
    let mut items = Vec::with_capacity(100_000);
    for folder in 0..100_u64 {
        let root = folder * 1_000;
        items.push(TreeNode {
            key: root,
            parent: None,
            has_children: true,
        });
        for offset in 1..1_000_u64 {
            items.push(TreeNode {
                key: root + offset,
                parent: Some(root),
                has_children: false,
            });
        }
    }
    let items: Arc<[TreeNode]> = items.into();
    let mut tree = UiTreeViewState::new(TreeViewId::new("showcase-tree-view"));
    tree.reconcile(&items, tree_node, tree_view_config())
        .expect("showcase tree data is valid preorder");
    tree.apply(UiTreeViewEvent::Toggle(0), tree_view_config());
    TreeViewState {
        tree,
        items,
        renames: Arc::new(HashMap::new()),
        focus_target: TreeFocusTarget::None,
    }
}

fn tree_rename_input_id(key: u64) -> iced::widget::Id {
    iced::widget::Id::from(format!("showcase-tree-rename-{key}"))
}

pub fn tree_view_focus(state: TreeViewState) -> iced::Task<()> {
    match state.focus_target {
        TreeFocusTarget::Rename => state
            .tree
            .editing()
            .map_or_else(iced::Task::none, |(key, _)| {
                iced::widget::operation::focus(tree_rename_input_id(*key))
            }),
        TreeFocusTarget::Tree => state.tree.focus_task(),
        TreeFocusTarget::None => iced::Task::none(),
    }
}

pub fn tree_view_apply(mut state: TreeViewState, event: TreeViewEvent) -> TreeViewState {
    state.focus_target = match &event {
        UiTreeViewEvent::BeginRename { .. } => TreeFocusTarget::Rename,
        UiTreeViewEvent::CommitRename | UiTreeViewEvent::CancelRename => TreeFocusTarget::Tree,
        _ => TreeFocusTarget::None,
    };
    let outcome = state.tree.apply(event, tree_view_config());
    if let Some(rename) = outcome.rename_committed {
        Arc::make_mut(&mut state.renames).insert(rename.key, rename.value);
    }
    state
}

pub fn tree_view_begin_selected_rename(mut state: TreeViewState) -> TreeViewState {
    let Some(key) = state.tree.selected().copied() else {
        return state;
    };
    let value = state
        .items
        .iter()
        .find(|item| item.key == key)
        .map(|item| state.label(item))
        .expect("selected tree key belongs to reconciled caller data");
    state.tree.apply(
        UiTreeViewEvent::BeginRename { key, value },
        tree_view_config(),
    );
    state.focus_target = TreeFocusTarget::Rename;
    state
}

pub fn tree_view_cancel_rename(mut state: TreeViewState) -> TreeViewState {
    state
        .tree
        .apply(UiTreeViewEvent::CancelRename, tree_view_config());
    state.focus_target = TreeFocusTarget::Tree;
    state
}

pub fn tree_view(state: &TreeViewState) -> Element<'_, TreeViewEvent> {
    let theme = theme();
    let inspection = state.tree.inspect(tree_view_config());
    let selected = state.tree.selected().map_or_else(
        || "No node selected".to_owned(),
        |key| format!("Selected {key}"),
    );
    let summary = row![
        text(format!(
            "{} visible / {} logical",
            inspection.visible_nodes, inspection.logical_nodes
        ))
        .size(11)
        .font(ui_font(Weight::Semibold)),
        iced::widget::Space::new().width(Length::Fill),
        text(selected)
            .size(10)
            .color(theme.palette.muted_foreground),
    ]
    .align_y(iced::Alignment::Center);
    let tree = ui_tree_view(
        &state.tree,
        &state.items,
        tree_view_config(),
        "Repository tree",
        |item| state.label(item),
        |row_meta, item, selected| {
            let disclosure = if row_meta.has_children() {
                if row_meta.expanded() { "▾" } else { "▸" }
            } else {
                " "
            };
            let toggle = iced::widget::button(text(disclosure).size(11)).padding([0, 4]);
            let toggle = if row_meta.has_children() {
                toggle.on_press(UiTreeViewEvent::Toggle(item.key))
            } else {
                toggle
            };
            let label: Element<'_, TreeViewEvent> = if row_meta.editing() {
                let value = state
                    .tree
                    .editing()
                    .filter(|(key, _)| *key == &item.key)
                    .map(|(_, value)| value)
                    .expect("editing row has retained rename state");
                let editor = iced::widget::text_input("Node name", value)
                    .id(tree_rename_input_id(item.key))
                    .on_input(UiTreeViewEvent::RenameChanged)
                    .on_submit(UiTreeViewEvent::CommitRename)
                    .size(11);
                let editor_id = format!("tree-rename-input-{}", item.key);
                let editor = accessible(editor, StableId::new(editor_id.clone()), Role::TextInput)
                    .logical_id(editor_id)
                    .label("Node name")
                    .value(value)
                    .focus_id(tree_rename_input_id(item.key));
                let cancel_id = format!("tree-rename-cancel-{}", item.key);
                let cancel = accessible(
                    iced::widget::button(text("Cancel").size(10))
                        .padding([0, 6])
                        .on_press(UiTreeViewEvent::CancelRename),
                    StableId::new(cancel_id.clone()),
                    Role::Button,
                )
                .logical_id(cancel_id)
                .label("Cancel rename")
                .on_activate(UiTreeViewEvent::CancelRename);
                row![editor, cancel,]
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
                    .into()
            } else {
                text(state.label(item)).size(11).into()
            };
            let rename: Element<'_, TreeViewEvent> = if selected && !row_meta.editing() {
                let rename_id = format!("tree-rename-{}", item.key);
                accessible(
                    iced::widget::button(text("Rename").size(10))
                        .padding([0, 6])
                        .on_press(UiTreeViewEvent::BeginRename {
                            key: item.key,
                            value: state.label(item),
                        }),
                    StableId::new(rename_id.clone()),
                    Role::Button,
                )
                .logical_id(rename_id)
                .label("Rename node")
                .on_activate(UiTreeViewEvent::BeginRename {
                    key: item.key,
                    value: state.label(item),
                })
                .into()
            } else {
                iced::widget::Space::new().into()
            };
            row![toggle, label, rename]
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .into()
        },
        |event| event,
        &theme,
    );
    column![summary, tree]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn data_grid_config() -> DataGridConfig {
    DataGridConfig::new(28.0, 32.0)
        .expect("showcase data-grid geometry is valid")
        .overscan(3)
}

fn data_grid_columns() -> Arc<[DataGridColumn<u8>]> {
    (0_u8..16)
        .map(|column| {
            let label = match column {
                0 => "Name".to_owned(),
                1 => "ID".to_owned(),
                2 => "Status".to_owned(),
                3 => "Owner".to_owned(),
                _ => format!("Field {}", column + 1),
            };
            DataGridColumn::new(column, label, if column == 0 { 184.0 } else { 104.0 })
                .sortable(column < 4)
                .editable(matches!(column, 0 | 3))
        })
        .collect::<Vec<_>>()
        .into()
}

fn data_grid_editor_id(cell: &DataGridCellId<u64, u8>) -> iced::widget::Id {
    iced::widget::Id::from(format!(
        "showcase-data-grid-edit-{}-{}",
        cell.row, cell.column
    ))
}

pub fn data_grid_state() -> DataGridState {
    let rows: Arc<[u64]> = (0..100_000_u64).collect::<Vec<_>>().into();
    let columns = data_grid_columns();
    let mut grid = UiDataGridState::new(DataGridId::new("showcase-data-grid"));
    grid.reconcile(&rows, |row| *row, &columns, data_grid_config())
        .expect("showcase data-grid keys and widths are valid");
    DataGridState {
        grid,
        rows,
        columns,
        sort: None,
        edits: Arc::new(HashMap::new()),
        draft: String::new(),
        focus_target: DataGridFocusTarget::None,
    }
}

pub fn data_grid_focus(state: DataGridState) -> iced::Task<()> {
    match state.focus_target {
        DataGridFocusTarget::Editor => state
            .grid
            .editing_cell()
            .map_or_else(iced::Task::none, |cell| {
                iced::widget::operation::focus(data_grid_editor_id(cell))
            }),
        DataGridFocusTarget::Grid => state.grid.focus_task(),
        DataGridFocusTarget::None => iced::Task::none(),
    }
}

pub fn data_grid_apply(mut state: DataGridState, event: DataGridEvent) -> DataGridState {
    let event = match event {
        DataGridEvent::Grid(event) => event,
        DataGridEvent::EditChanged(value) => {
            state.draft = value;
            state.focus_target = DataGridFocusTarget::None;
            return state;
        }
    };
    state.focus_target = match &event {
        UiDataGridEvent::BeginEdit(_) => DataGridFocusTarget::Editor,
        UiDataGridEvent::CommitEdit(_) | UiDataGridEvent::CancelEdit(_) => {
            DataGridFocusTarget::Grid
        }
        _ => DataGridFocusTarget::None,
    };
    let outcome = state.grid.apply(event, data_grid_config());
    if let Some(cell) = outcome.edit_started {
        state.draft = state.value(cell.row, cell.column);
    }
    if let Some(cell) = outcome.edit_committed {
        Arc::make_mut(&mut state.edits).insert((cell.row, cell.column), state.draft.clone());
    }
    if let Some(column) = outcome.sort_requested {
        state.sort = match state.sort {
            Some((active, AccessibilitySortDirection::Ascending)) if active == column => {
                Some((column, AccessibilitySortDirection::Descending))
            }
            Some((active, AccessibilitySortDirection::Descending)) if active == column => None,
            _ => Some((column, AccessibilitySortDirection::Ascending)),
        };
        let rows = Arc::make_mut(&mut state.rows);
        rows.sort_unstable();
        if let Some((column, direction)) = state.sort {
            rows.sort_by_key(|row| row.wrapping_mul(u64::from(column) + 17) % 100_003);
            if direction == AccessibilitySortDirection::Descending {
                rows.reverse();
            }
        }
        state
            .grid
            .reconcile(&state.rows, |row| *row, &state.columns, data_grid_config())
            .expect("caller-owned sorting preserves unique rows and fixed columns");
    }
    state
}

pub fn data_grid(state: &DataGridState) -> Element<'_, DataGridEvent> {
    let theme = theme();
    let inspection = state.grid.inspect(data_grid_config());
    let active = state.grid.active_cell().map_or_else(
        || "No active cell".to_owned(),
        |cell| format!("Active row {} · column {}", cell.row, cell.column + 1),
    );
    let summary = row![
        text(format!(
            "mounted {}..{} of {} rows",
            inspection.mounted_rows.start, inspection.mounted_rows.end, inspection.logical_rows
        ))
        .size(11)
        .font(ui_font(Weight::Semibold)),
        iced::widget::Space::new().width(Length::Fill),
        text(active).size(10).color(theme.palette.muted_foreground),
    ]
    .align_y(iced::Alignment::Center);
    let grid = ui_data_grid(
        &state.grid,
        &state.rows,
        data_grid_config(),
        "Repository data grid",
        |row| *row,
        |row| format!("Repository row {row}"),
        |row, column| format!("{}: {}", column.label(), state.value(*row, *column.key())),
        |column| {
            state
                .sort
                .filter(|(active, _)| active == column)
                .map(|(_, direction)| direction)
        },
        |header| {
            let marker = match header.sort_direction {
                Some(AccessibilitySortDirection::Ascending) => " ↑",
                Some(AccessibilitySortDirection::Descending) => " ↓",
                _ => "",
            };
            text(format!("{}{marker}", header.column.label()))
                .size(11)
                .font(ui_font(Weight::Semibold))
                .into()
        },
        |cell| {
            let cell_id = DataGridCellId {
                row: *cell.row,
                column: *cell.column.key(),
            };
            if cell.editing {
                let input_id = data_grid_editor_id(&cell_id);
                let editor = iced::widget::text_input("Cell value", &state.draft)
                    .id(input_id.clone())
                    .on_input(DataGridEvent::EditChanged)
                    .on_submit(DataGridEvent::Grid(UiDataGridEvent::CommitEdit(
                        cell_id.clone(),
                    )))
                    .size(11);
                let semantic_id = format!("data-grid-editor-{}-{}", cell_id.row, cell_id.column);
                accessible(editor, StableId::new(&semantic_id), Role::TextInput)
                    .logical_id(semantic_id)
                    .label("Cell value")
                    .value(&state.draft)
                    .focus_id(input_id)
                    .into()
            } else {
                text(state.value(*cell.row, *cell.column.key()))
                    .size(11)
                    .into()
            }
        },
        DataGridEvent::Grid,
        &theme,
    );
    column![summary, grid]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn data_table_rows(query: String, sort: String, page: i64) -> Vec<CatalogItem> {
    let mut state = DataTableState::new(DATA_TABLE_PAGE_SIZE);
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
        Some(SortDirection::Ascending) => rows.sort_by(|left, right| left.name.cmp(&right.name)),
        Some(SortDirection::Descending) => {
            rows.sort_by(|left, right| right.name.cmp(&left.name));
        }
        None => {}
    }
    state.set_page(data_table_page(query, page) as usize, rows.len());
    rows[state.visible_range(rows.len())].to_vec()
}

pub fn data_table_page(query: String, page: i64) -> i64 {
    let rows = catalog_items(&query);
    let pages = DataTableState::<()>::new(DATA_TABLE_PAGE_SIZE).page_count(rows.len());
    let page = page_index(page).min(pages.saturating_sub(1));
    i64::try_from(page).unwrap_or(i64::MAX)
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
    let state = DataTableState::<()>::new(DATA_TABLE_PAGE_SIZE);
    data_table_page(query.clone(), page) as usize + 1
        < state.page_count(catalog_items(&query).len())
}

pub fn data_table_page_count(query: String) -> i64 {
    let state = DataTableState::<()>::new(DATA_TABLE_PAGE_SIZE);
    i64::try_from(state.page_count(catalog_items(&query).len()).max(1)).unwrap_or(i64::MAX)
}

pub fn data_table_result_count(query: String) -> i64 {
    i64::try_from(catalog_items(&query).len()).unwrap_or(i64::MAX)
}

pub fn data_table_page_range(query: String, page: i64) -> Vec<i64> {
    const WINDOW: usize = 5;
    let total = usize::try_from(data_table_page_count(query)).unwrap_or(usize::MAX);
    let current = page_index(page).min(total.saturating_sub(1));
    let start = current
        .saturating_sub(WINDOW / 2)
        .min(total.saturating_sub(WINDOW));
    let end = start.saturating_add(WINDOW).min(total);
    (start..end)
        .map(|index| i64::try_from(index.saturating_add(1)).unwrap_or(i64::MAX))
        .collect()
}

pub fn data_table_page_label(page: i64, current: bool) -> String {
    if current {
        format!("Page {page}, current page")
    } else {
        format!("Go to page {page}")
    }
}

fn page_index(page: i64) -> usize {
    usize::try_from(page.max(0)).unwrap_or(usize::MAX)
}

pub fn resizable_demo(sizes: &[f64]) -> Element<'static, Vec<f64>> {
    let sizes = sizes.iter().map(|size| *size as f32).collect::<Vec<_>>();
    let panels = ["Navigation", "Canvas", "Inspector"]
        .map(|label| container(text(label)).center(Length::Fill).into());
    let theme = theme();

    let view = resizable(
        "ice-native-resizable",
        panels,
        sizes,
        vec![0.15; 3],
        |next| next.into_iter().map(f64::from).collect(),
        &theme,
    )
    .with_handles(true)
    .height(96);
    semantic(view, "ice-native-resizable", Role::Splitter)
}

pub fn popover_apply(event: PopoverEvent) -> iced::Task<bool> {
    iced::Task::done(next_open(event))
        .chain(event.focus_task(&PopoverIds::new("ice-native-popover")))
}

pub fn popover_demo(open: bool) -> Element<'static, PopoverEvent> {
    let theme = theme();
    let view = popover(
        PopoverIds::new("ice-native-popover"),
        surface(text("Toggle popover"), SurfaceVariant::Default, &theme)
            .height(ACTION_HEIGHT)
            .padding([0, 12])
            .align_y(Vertical::Center),
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
    .width(280.0);
    semantic(
        column![
            view,
            text(if open {
                "Popover open"
            } else {
                "Popover closed"
            })
        ]
        .spacing(4),
        "ice-native-popover",
        Role::Group,
    )
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
        MenuGroup::new(
            "file-actions",
            vec![
                MenuItem::new("new", "New file").shortcut("⌘N").into(),
                MenuItem::new("open", "Open…").shortcut("⌘O").into(),
            ],
        )
        .label("File")
        .into(),
        MenuEntry::separator("collaboration-separator"),
        MenuGroup::new(
            "collaboration",
            vec![
                MenuItem::new("share", "Share")
                    .submenu(vec![
                        MenuItem::new("copy-link", "Copy link").into(),
                        MenuItem::new("invite", "Invite people").into(),
                    ])
                    .into(),
            ],
        )
        .label("Collaboration")
        .into(),
        MenuEntry::separator("danger-separator"),
        MenuGroup::new(
            "danger-zone",
            vec![
                MenuItem::new("delete", "Move to trash")
                    .shortcut("⌫")
                    .into(),
            ],
        )
        .label("Danger zone")
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

fn catalog_items(query: &str) -> Vec<CatalogItem> {
    let query = query.to_lowercase();
    [
        "Accordion",
        "Alert dialog",
        "Button",
        "Calendar",
        "Chart",
        "Command",
        "Context menu",
        "Data table",
        "Date picker",
        "Dialog",
        "Dropdown menu",
        "Input",
        "Input OTP",
        "Message scroller",
        "Navigation menu",
        "Resizable",
        "Select",
        "Sidebar",
        "Sonner",
    ]
    .into_iter()
    .filter(|row| row.to_lowercase().contains(&query))
    .map(|name| CatalogItem {
        name: name.to_owned(),
        source: if name == "Button" || name == "Input" || name == "Dialog" {
            "Ice".to_owned()
        } else {
            "ducktape-ui".to_owned()
        },
    })
    .collect()
}

fn theme() -> ui_lang_components::ui::theme::Theme {
    LIGHT.with_fonts(
        crate::Showcase::default_font(),
        Font::with_name("Geist Mono"),
    )
}

fn ui_font(weight: Weight) -> Font {
    Font {
        weight,
        ..crate::Showcase::default_font()
    }
}

fn italic_font() -> Font {
    Font {
        style: FontStyle::Italic,
        ..crate::Showcase::default_font()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_build_the_checked_default_contracts() {
        let font = crate::Showcase::default_font();
        assert_eq!(theme().typography.font, font);
        assert_eq!(
            theme().typography.monospace_font,
            Font::with_name("Geist Mono")
        );
        assert_ne!(
            theme().typography.monospace_font.family,
            theme().typography.font.family
        );

        let _: Element<'_, String> = input_otp("otp", "", false, false);
        let _: Element<'_, ()> = spinner(-1, false);
        let _ = checkbox_style(
            &iced::Theme::Light,
            iced::widget::checkbox::Status::Active { is_checked: false },
        );
        let _ = progress_style(&iced::Theme::Light);
        let _ = progress_success_style(&iced::Theme::Light);
        let _ = progress_warning_style(&iced::Theme::Light);
        let _ = progress_destructive_style(&iced::Theme::Light);
        let calendar = calendar_state();
        let _: Element<'_, CalendarEvent> = super::calendar(&calendar);
        let _: iced::Task<CalendarState> = calendar_apply(
            calendar.clone(),
            CalendarEvent::MonthChanged(calendar.month()),
        );
        let date_picker = date_picker_state();
        let _: Element<'_, DatePickerEvent> = super::date_picker(&date_picker);
        let _: Element<'_, Option<ChartHit>> = chart(None);
        let command = command_state();
        let _: Element<'_, CommandEvent> = super::command(&command);
        let select = select_state();
        let _: Element<'_, SelectEvent> = super::select(&select);
        let dropdown = dropdown_menu_state();
        let _: Element<'_, DropdownMenuEvent> = dropdown_menu(&dropdown);
        let context = context_menu_state();
        let _: Element<'_, ContextMenuEvent> = context_menu(&context);
        let alert = alert_dialog_state();
        let _: Element<'_, AlertDialogEvent> = alert_dialog(&alert);
        let sidebar = sidebar_state();
        let _: Element<'_, SidebarEvent> = super::sidebar(&sidebar);
        let sonner = sonner_state();
        let _: Element<'_, SonnerEvent> = super::sonner(&sonner);
        let drawer = drawer_state();
        let _: Element<'_, DrawerEvent> = super::drawer(&drawer, false);
        let navigation = navigation_menu_state();
        let _: Element<'_, NavigationMenuEvent> = navigation_menu(&navigation);
        let menubar = menubar_state();
        let _: Element<'_, MenubarEvent> = super::menubar(&menubar);
        let _: Element<'_, bool> = hover_card();
        let _: Element<'_, Vec<f64>> = slider(&[25.0, 75.0]);
        let _: Element<'_, String> = radio_group("default");
        let scroller = message_scroller_state();
        let _: Element<'_, MessageScrollerEvent> = message_scroller(&scroller);
        let timeline = log_timeline_state();
        let _: Element<'_, LogTimelineEvent> = log_timeline(&timeline);
        let _: Element<'_, Vec<f64>> = resizable_demo(&[0.25, 0.5, 0.25]);
        let _: Element<'_, PopoverEvent> = popover_demo(false);
        let _: iced::Task<bool> = popover_apply(PopoverEvent::Open);
    }

    #[test]
    fn dropdown_groups_have_explicit_category_labels() {
        let labels = dropdown_entries()
            .into_iter()
            .filter_map(|entry| match entry {
                MenuEntry::Group(group) => group.label,
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(labels, ["File", "Collaboration", "Danger zone"]);
    }

    #[test]
    fn bundled_geist_families_have_real_regular_bold_and_italic_faces() {
        use iced::advanced::graphics::text::cosmic_text::fontdb::{
            Database, Family, Query, Stretch, Style, Weight,
        };

        let mut fonts = Database::new();
        fonts.load_font_data(include_bytes!("../../../assets/fonts/Geist-Regular.ttf").to_vec());
        fonts.load_font_data(include_bytes!("../../../assets/fonts/Geist-Bold.ttf").to_vec());
        fonts.load_font_data(include_bytes!("../../../assets/fonts/Geist-Italic.ttf").to_vec());
        fonts
            .load_font_data(include_bytes!("../../../assets/fonts/GeistMono-Regular.ttf").to_vec());
        fonts.load_font_data(include_bytes!("../../../assets/fonts/GeistMono-Bold.ttf").to_vec());
        fonts.load_font_data(include_bytes!("../../../assets/fonts/GeistMono-Italic.ttf").to_vec());

        for family in ["Geist", "Geist Mono"] {
            for weight in [Weight::NORMAL, Weight::BOLD] {
                let id = fonts
                    .query(&Query {
                        families: &[Family::Name(family)],
                        weight,
                        stretch: Stretch::Normal,
                        style: Style::Normal,
                    })
                    .expect("bundled Geist face must resolve");
                let face = fonts.face(id).expect("resolved face");
                assert_eq!(face.weight, weight);
                assert_eq!(face.monospaced, family == "Geist Mono");
            }

            let italic = fonts
                .query(&Query {
                    families: &[Family::Name(family)],
                    weight: Weight::NORMAL,
                    stretch: Stretch::Normal,
                    style: Style::Italic,
                })
                .expect("bundled Geist italic face must resolve");
            let face = fonts.face(italic).expect("resolved face");
            assert_eq!(face.style, Style::Italic);
            assert_eq!(face.monospaced, family == "Geist Mono");
        }
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
        assert_eq!(sonner.queue.max_visible(), 2);
        let initial = sonner.queue.len();
        let sonner = sonner_apply(sonner, SonnerEvent::Show);
        assert_eq!(sonner.queue.len(), initial + 1);
        let sonner = sonner_apply(sonner, SonnerEvent::Show);
        assert_eq!(sonner.queue.visible().count(), 2);
        let mut saturated = sonner_state();
        saturated.shown = i64::MAX;
        let saturated = sonner_apply(saturated, SonnerEvent::Show);
        assert_eq!(saturated.shown, i64::MAX);

        assert_eq!(
            data_table_rows("a".to_owned(), "ascending".to_owned(), 0)
                .into_iter()
                .map(|row| row.name)
                .collect::<Vec<_>>(),
            ["Accordion", "Alert dialog", "Calendar"]
        );
        assert!(data_table_can_next(String::new(), 0));
        assert!(!data_table_can_next("button".to_owned(), 0));
        assert!(!data_table_can_next(String::new(), i64::MAX));
        assert_eq!(
            data_table_rows(String::new(), "none".to_owned(), i64::MAX)
                .into_iter()
                .map(|row| row.name)
                .collect::<Vec<_>>(),
            ["Sonner"]
        );
        assert_eq!(data_table_next_sort("ascending".to_owned()), "descending");
        assert_eq!(data_table_page_count(String::new()), 7);
        assert_eq!(data_table_page_range(String::new(), 0), [1, 2, 3, 4, 5]);
        assert_eq!(data_table_page_range(String::new(), 6), [3, 4, 5, 6, 7]);
        assert_eq!(data_table_page_range("input".to_owned(), 0), [1]);
    }
}
