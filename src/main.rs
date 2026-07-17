pub mod ui;

use std::time::{Duration, Instant};

use iced::widget::{column, container, row, scrollable, stack, text};
use iced::{Background, Element, Length, Theme as IcedTheme};
use ui::accordion::{
    AccordionEvent, AccordionKey, AccordionState, accordion, accordion_item, header_target,
};
use ui::alert::{AlertVariant, alert};
use ui::alert_dialog::{
    AlertDialogActionVariant, AlertDialogEvent, AlertDialogFocus, alert_dialog,
    next_open as next_alert_dialog_open,
};
use ui::aspect_ratio::aspect_ratio;
use ui::attachment::attachment;
use ui::avatar::{AvatarSize, avatar_fallback};
use ui::badge::{BadgeSize, BadgeVariant, badge};
use ui::breadcrumb::{BreadcrumbItem, breadcrumb, breadcrumb_separator};
use ui::bubble::{BubbleVariant, bubble};
use ui::button::{ButtonSize, ButtonVariant, button};
use ui::button_group::{ButtonGroupOrientation, button_group};
use ui::calendar::{
    CalendarEvent, CalendarSelection, CalendarState, Date, Month, controlled_calendar,
};
use ui::card::{card, card_header};
use ui::carousel::{
    CarouselBoundary, CarouselCommand, CarouselEvent, CarouselOrientation, CarouselState,
    carousel_indicators, carousel_next, carousel_previous, controlled_carousel, keyboard_command,
};
use ui::chart::{
    AxisDomain, BarLayout, CartesianKind, ChartColor, ChartConfig, ChartData, ChartDatum, ChartHit,
    ChartPadding, DomainSpec, PieData, PieHit, SeriesConfig, TooltipIndicator, TooltipOptions,
    cartesian_chart, companion_content, companion_model, legend_content, legend_entries, pie_chart,
    pie_tooltip_model, tooltip_content, tooltip_model,
};
use ui::checkbox::checkbox;
use ui::collapsible::{CollapsibleChange, collapsible, next_open};
use ui::combobox::combobox;
use ui::command::{
    CommandEvent, CommandState, command, command_group, command_group_without_heading,
    command_item, focus_command_input,
};
use ui::context_menu::{ContextMenuEvent, ContextMenuIds, context_menu};
use ui::data_table::DataTableState;
use ui::date_picker::{
    DateFormat, DatePickerEvent, DatePickerIds, DatePickerValue, date_picker, format_value,
};
use ui::dialog::{DialogActionAlignment, DialogAlignment, dialog, dialog_panel};
use ui::direction::{Direction, directed_row};
use ui::drawer::{
    DrawerActionAlignment, DrawerEvent, DrawerState, DrawerTextAlignment, drawer, drawer_body,
    drawer_footer, drawer_header, drawer_panel,
};
use ui::dropdown_menu::{DropdownMenuEvent, DropdownMenuIds, dropdown_menu};
use ui::empty_state::empty_state;
use ui::field::{FieldHint, field};
use ui::hover_card::{HoverCardId, hover_card};
use ui::input::{InputVariant, input, input_with_variant};
use ui::input_group::{group_input, input_group};
use ui::input_otp::{OtpPattern, input_otp, is_complete, normalize};
use ui::item::item;
use ui::kbd::kbd;
use ui::label::label;
use ui::marker::{MarkerVariant, marker};
use ui::menu::{
    MenuActivation, MenuActivationKind, MenuEntry, MenuEvent, MenuItem, MenuState, focus_menu_state,
};
use ui::menubar::{MenubarEvent, MenubarMenu, MenubarState, menubar};
use ui::message::{MessageSide, message};
use ui::message_scroller::{
    MessageScrollerEvent, MessageScrollerItemMeta, MessageScrollerState,
    controlled_message_scroller, message_scroller_item,
};
use ui::modal::{DismissRules, FocusScope, ModalEvent};
use ui::native_select::native_select_with_id;
use ui::navigation_menu::{
    NavigationMenuEvent, NavigationMenuItem, NavigationMenuState, navigation_menu,
    navigation_menu_list, navigation_menu_list_link,
};
use ui::pagination::{PaginationItem, pagination};
use ui::popover::{
    Alignment as FloatingAlignment, Placement, PopoverEvent, PopoverIds,
    next_open as next_popover_open, popover,
};
use ui::progress::{ProgressVariant, progress};
use ui::radio_group::{RadioOption, RadioOrientation, focus_radio, radio_group, radio_option};
use ui::resizable::{
    ResizableHandle, ResizableLayout, ResizableOrientation, focus_resizable_handle, resizable,
};
use ui::scroll_area::scroll_area;
use ui::segmented_control::segmented_control;
use ui::select::{SelectEvent, SelectGroup, SelectIds, SelectOption, select};
use ui::sheet::{
    SheetActionAlignment, SheetMode, SheetSide, SheetTextAlignment, sheet, sheet_body,
    sheet_footer, sheet_header, sheet_panel,
};
use ui::sidebar::{
    SIDEBAR_METRICS, SidebarAction, SidebarCollapsible, SidebarId, SidebarMenuButtonId,
    SidebarMenuButtonSize, SidebarSide, SidebarState, SidebarVariant, SidebarViewport,
    shortcut_action, sidebar, sidebar_footer, sidebar_group, sidebar_group_action,
    sidebar_group_content, sidebar_group_heading, sidebar_header, sidebar_layout, sidebar_menu,
    sidebar_menu_badge, sidebar_menu_button, sidebar_menu_button_content, sidebar_menu_item,
    sidebar_menu_skeleton, sidebar_submenu, sidebar_submenu_button, sidebar_submenu_item,
};
use ui::slider::{SliderOrientation, focus_slider_thumb, slider};
use ui::sonner::{
    SonnerEvent, SonnerOutcome, SonnerState, SwipeDirection, ToastId, ToastPlacement, sonner,
};
use ui::surface::{SurfaceVariant, surface};
use ui::switch::{SwitchSize, switch};
use ui::tabs::{TabsActivation, TabsEvent, TabsOrientation, TabsState, TabsVariant, tab, tabs};
use ui::textarea::{TextareaVariant, textarea};
use ui::theme::{ACCENTS, DARK, LIGHT, Theme};
use ui::toast::{ToastData, ToastVariant, toast};
use ui::toggle::{ToggleSize, ToggleVariant, toggle};
use ui::toggle_group::{ToggleGroupOrientation, ToggleGroupState, toggle_group, toggle_group_item};
use ui::tooltip::{TooltipId, tooltip, tooltip_text};
use ui::typography::{TextRole, inline_code, typography};

struct TranscriptRow {
    id: String,
    copy: String,
    side: Option<MessageSide>,
}

struct Showcase {
    dark: bool,
    email: String,
    clicks: u32,
    accent: usize,
    section: Section,
    accepted: bool,
    notes: iced::widget::text_editor::Content,
    page: usize,
    combo: iced::widget::combo_box::State<String>,
    combo_selected: Option<String>,
    select_options: Vec<String>,
    native_selected: Option<String>,
    accordion_state: AccordionState<&'static str>,
    collapsible_open: bool,
    calendar_state: CalendarState,
    date_picker_month: Month,
    date_picker_value: DatePickerValue,
    date_picker_focused: Option<Date>,
    date_picker_open: bool,
    date_picker_ids: DatePickerIds,
    carousel_state: CarouselState,
    tabs_automatic: TabsState<&'static str>,
    tabs_manual: TabsState<&'static str>,
    otp: String,
    radio_selected: &'static str,
    slider_values: Vec<f32>,
    standalone_toggle: bool,
    toggle_group_state: ToggleGroupState<&'static str>,
    switch_on: bool,
    resizable_sizes: Vec<f32>,
    dialog_open: bool,
    dialog_focus: FocusScope,
    alert_dialog_open: bool,
    alert_dialog_focus: AlertDialogFocus,
    sonner: SonnerState,
    started_at: Instant,
    loading_toast: Option<ToastId>,
    popover_open: bool,
    popover_ids: PopoverIds,
    menu_entries: Vec<MenuEntry>,
    dropdown_open: bool,
    dropdown_state: MenuState,
    context_open: bool,
    context_anchor: Option<iced::Point>,
    context_state: MenuState,
    menubar_state: MenubarState,
    menubar_menu_state: MenuState,
    navigation_state: NavigationMenuState,
    select_open: bool,
    select_state: MenuState,
    select_value: Option<&'static str>,
    menu_bookmarked: bool,
    menu_density: &'static str,
    menu_last_action: Option<String>,
    chart_hover: Option<ChartHit>,
    pie_hover: Option<PieHit>,
    command_state: CommandState,
    command_selection: Option<&'static str>,
    sidebar_state: SidebarState,
    sheet_open: bool,
    sheet_focus: FocusScope,
    drawer_state: DrawerState,
    drawer_focus: FocusScope,
    message_scroller: MessageScrollerState,
    transcript: Vec<TranscriptRow>,
    next_transcript_id: usize,
    next_history_id: usize,
}

impl Default for Showcase {
    fn default() -> Self {
        let mut tabs_automatic = TabsState::new("overview");
        tabs_automatic.select("overview");
        let dialog_trigger = iced::widget::Id::new("showcase-dialog-trigger");
        let dialog_focus = FocusScope::new(
            iced::widget::Id::new("showcase-dialog-cancel"),
            dialog_trigger,
        )
        .push(iced::widget::Id::new("showcase-dialog-save"));
        let alert_dialog_focus = AlertDialogFocus::new(
            iced::widget::Id::new("showcase-alert-cancel"),
            iced::widget::Id::new("showcase-alert-action"),
            iced::widget::Id::new("showcase-alert-trigger"),
        );
        let mut sonner = SonnerState::new(3, ToastPlacement::BottomRight);
        sonner.set_max_visible(3, Duration::ZERO);
        sonner.set_default_duration(Duration::from_secs(4));
        sonner.set_offset(24.0);
        sonner.set_expanded(false);
        sonner.set_reduced_motion(false);
        sonner.set_swipe_direction(SwipeDirection::Right);
        sonner.set_swipe_threshold(72.0);
        let sheet_focus = FocusScope::new(
            iced::widget::Id::new("showcase-sheet-close"),
            iced::widget::Id::new("showcase-sheet-trigger"),
        )
        .push(iced::widget::Id::new("showcase-sheet-save"));
        let drawer_focus = FocusScope::new(
            iced::widget::Id::new("showcase-drawer-close"),
            iced::widget::Id::new("showcase-drawer-trigger"),
        )
        .push(iced::widget::Id::new("showcase-drawer-action"));
        let menu_entries = showcase_menu_entries(false, "comfortable");
        let menubar_menus = showcase_menubar_menus(false, "comfortable");
        let select_groups = showcase_select_groups();

        Self {
            dark: false,
            email: String::new(),
            clicks: 0,
            accent: 0,
            section: Section::default(),
            accepted: false,
            notes: iced::widget::text_editor::Content::default(),
            page: 0,
            combo: iced::widget::combo_box::State::new(vec![
                "Button".into(),
                "Dialog".into(),
                "Table".into(),
            ]),
            combo_selected: None,
            select_options: vec!["Light".into(), "Dark".into(), "System".into()],
            native_selected: None,
            accordion_state: AccordionState::Single(Some("install")),
            collapsible_open: false,
            calendar_state: CalendarState::new(
                Month::new(2026, 7).expect("showcase month is valid"),
                CalendarSelection::Multiple(vec![
                    Date::new(2026, 7, 16).expect("showcase selection is valid"),
                    Date::new(2026, 7, 18).expect("showcase selection is valid"),
                ]),
            )
            .focused(Some(
                Date::new(2026, 7, 16).expect("showcase focus is valid"),
            )),
            date_picker_month: Month::new(2026, 7).expect("showcase month is valid"),
            date_picker_value: DatePickerValue::Range(None),
            date_picker_focused: Some(Date::new(2026, 7, 16).expect("showcase focus is valid")),
            date_picker_open: false,
            date_picker_ids: DatePickerIds::new("showcase"),
            carousel_state: CarouselState::new(0, 3, CarouselBoundary::Wrap),
            tabs_automatic,
            tabs_manual: TabsState::new("account"),
            otp: "123".into(),
            radio_selected: "comfortable",
            slider_values: vec![25.0, 75.0],
            standalone_toggle: false,
            toggle_group_state: ToggleGroupState::Multiple(vec!["bold"]),
            switch_on: true,
            resizable_sizes: vec![0.25, 0.5, 0.25],
            dialog_open: false,
            dialog_focus,
            alert_dialog_open: false,
            alert_dialog_focus,
            sonner,
            started_at: Instant::now(),
            loading_toast: None,
            popover_open: false,
            popover_ids: PopoverIds::new("showcase-popover"),
            menu_entries: menu_entries.clone(),
            dropdown_open: false,
            dropdown_state: MenuState::initial(&menu_entries),
            context_open: false,
            context_anchor: None,
            context_state: MenuState::initial(&menu_entries),
            menubar_state: MenubarState::initial(&menubar_menus),
            menubar_menu_state: MenuState::initial(&menubar_menus[0].entries),
            navigation_state: NavigationMenuState {
                focused: Some(0),
                open: None,
                active: Some("home".into()),
            },
            select_open: false,
            select_state: MenuState::initial(&ui::select::select_entries(&select_groups, None)),
            select_value: None,
            menu_bookmarked: false,
            menu_density: "comfortable",
            menu_last_action: None,
            chart_hover: None,
            pie_hover: None,
            command_state: CommandState::new(""),
            command_selection: None,
            sidebar_state: SidebarState::default(),
            sheet_open: false,
            sheet_focus,
            drawer_state: DrawerState::new(false),
            drawer_focus,
            message_scroller: MessageScrollerState::new("showcase").auto_scroll(true),
            transcript: vec![
                TranscriptRow {
                    id: "previous".into(),
                    copy: "Previous".into(),
                    side: None,
                },
                TranscriptRow {
                    id: "message-1".into(),
                    copy: "I kept the reader's place while history loaded.".into(),
                    side: Some(MessageSide::Incoming),
                },
                TranscriptRow {
                    id: "message-2".into(),
                    copy: "Stable row IDs make that possible.".into(),
                    side: Some(MessageSide::Outgoing),
                },
                TranscriptRow {
                    id: "message-3".into(),
                    copy: "Scroll away from the live edge, then stream a message.".into(),
                    side: Some(MessageSide::Incoming),
                },
                TranscriptRow {
                    id: "today".into(),
                    copy: "Today".into(),
                    side: None,
                },
                TranscriptRow {
                    id: "message-4".into(),
                    copy: "The transcript stops following user scroll.".into(),
                    side: Some(MessageSide::Outgoing),
                },
                TranscriptRow {
                    id: "message-5".into(),
                    copy: "New rows become unread until you jump back.".into(),
                    side: Some(MessageSide::Incoming),
                },
                TranscriptRow {
                    id: "message-6".into(),
                    copy: "Keyboard navigation works after focusing the viewport.".into(),
                    side: Some(MessageSide::Outgoing),
                },
            ],
            next_transcript_id: 7,
            next_history_id: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Section {
    #[default]
    General,
    Advanced,
}

fn showcase_menu_entries(bookmarked: bool, density: &str) -> Vec<MenuEntry> {
    vec![
        MenuEntry::label("workspace-label", "Workspace"),
        MenuItem::new("new-file", "New file").shortcut("⌘ N").into(),
        MenuItem::new("open-file", "Open file…")
            .shortcut("⌘ O")
            .into(),
        MenuItem::new("bookmark", "Bookmark")
            .checkbox(bookmarked)
            .into(),
        MenuEntry::separator("workspace-separator"),
        MenuItem::new("density-comfortable", "Comfortable")
            .radio("density", density == "comfortable")
            .into(),
        MenuItem::new("density-compact", "Compact")
            .radio("density", density == "compact")
            .into(),
        MenuItem::new("share", "Share")
            .submenu(vec![
                MenuItem::new("share-link", "Copy link").into(),
                MenuItem::new("share-email", "Email invite").into(),
            ])
            .into(),
        MenuItem::new("unavailable", "Unavailable")
            .disabled(true)
            .into(),
    ]
}

fn showcase_menubar_menus(bookmarked: bool, density: &str) -> Vec<MenubarMenu> {
    vec![
        MenubarMenu::new(
            "file",
            "File",
            vec![
                MenuItem::new("new-file", "New file").shortcut("⌘ N").into(),
                MenuItem::new("open-file", "Open file…")
                    .shortcut("⌘ O")
                    .into(),
                MenuEntry::separator("file-separator"),
                MenuItem::new("bookmark", "Bookmark")
                    .checkbox(bookmarked)
                    .into(),
            ],
        ),
        MenubarMenu::new(
            "view",
            "View",
            vec![
                MenuItem::new("density-comfortable", "Comfortable")
                    .radio("density", density == "comfortable")
                    .into(),
                MenuItem::new("density-compact", "Compact")
                    .radio("density", density == "compact")
                    .into(),
            ],
        ),
        MenubarMenu::new(
            "help",
            "Help",
            vec![MenuItem::new("about", "About ducktape-ui").into()],
        ),
    ]
}

fn showcase_select_groups() -> Vec<SelectGroup<&'static str>> {
    vec![
        SelectGroup::new(
            "fruit",
            vec![
                SelectOption::new("apple", "apple", "Apple"),
                SelectOption::new("banana", "banana", "Banana"),
                SelectOption::new("mango", "mango", "Mango").disabled(true),
            ],
        )
        .label("Fruit"),
        SelectGroup::new("other", vec![SelectOption::new("water", "water", "Water")])
            .label("Other"),
    ]
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    EmailChanged(String),
    Clicked,
    CycleAccent,
    SectionSelected(Section),
    AcceptedChanged(bool),
    NotesChanged(iced::widget::text_editor::Action),
    PageSelected(usize),
    ComboSelected(String),
    NativeSelected(String),
    Accordion(AccordionEvent<&'static str>),
    Collapsible(CollapsibleChange),
    Calendar(CalendarEvent),
    DatePicker(DatePickerEvent),
    Carousel(CarouselEvent),
    PrependTranscriptHistory,
    StreamTranscript,
    TranscriptScroll(MessageScrollerEvent),
    FocusTraversal { backwards: bool },
    TabsAutomatic(TabsEvent<&'static str>),
    TabsManual(TabsEvent<&'static str>),
    OtpChanged(String),
    RadioSelected(&'static str),
    SliderChanged(Vec<f32>),
    FocusFirstSliderThumb,
    StandaloneToggle,
    ToggleGroupValue(&'static str),
    ToggleGroupNavigate(usize),
    SwitchToggle,
    Resized(Vec<f32>),
    FocusFirstResizeHandle,
    OpenDialog,
    CloseDialog,
    SaveDialog,
    Dialog(ModalEvent),
    OpenAlertDialog,
    AlertDialog(AlertDialogEvent),
    ShowToast(ToastVariant),
    ResolveLoadingToast,
    ClearToasts,
    CycleToastPlacement,
    ToggleToastExpanded,
    ToggleReducedMotion,
    ToggleSwipeDirection,
    Sonner(SonnerEvent),
    Tick(Instant),
    Popover(PopoverEvent),
    Dropdown(DropdownMenuEvent),
    Context(ContextMenuEvent),
    Menubar(MenubarEvent),
    Navigation(NavigationMenuEvent),
    Select(SelectEvent<&'static str>),
    ChartHover(Option<ChartHit>),
    PieHover(Option<PieHit>),
    Command(CommandEvent<&'static str>),
    FocusCommandInput,
    Sidebar(SidebarAction),
    OpenSheet,
    CloseSheet,
    Sheet(ModalEvent),
    OpenDrawer,
    CloseDrawer,
    Drawer(DrawerEvent),
}

fn main() -> iced::Result {
    iced::application(Showcase::boot, Showcase::update, Showcase::view)
        .title("ducktape-ui component showcase")
        .window(iced::window::Settings {
            min_size: Some(iced::Size::new(480.0, 480.0)),
            ..Default::default()
        })
        .subscription(Showcase::subscription)
        .theme(Showcase::iced_theme)
        .run()
}

impl Showcase {
    fn boot() -> (Self, iced::Task<Message>) {
        let mut showcase = Self::default();
        let seed = showcase.sync_message_scroller();
        (showcase, seed)
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::ToggleTheme => self.dark = !self.dark,
            Message::EmailChanged(value) => self.email = value,
            Message::Clicked => self.clicks += 1,
            Message::CycleAccent => self.accent = (self.accent + 1) % ACCENTS.len(),
            Message::SectionSelected(section) => self.section = section,
            Message::AcceptedChanged(accepted) => self.accepted = accepted,
            Message::NotesChanged(action) => self.notes.perform(action),
            Message::PageSelected(page) => self.page = page,
            Message::ComboSelected(value) => self.combo_selected = Some(value),
            Message::NativeSelected(value) => self.native_selected = Some(value),
            Message::Accordion(event) => {
                self.accordion_state.apply(&event);
                return event.focus_task();
            }
            Message::Collapsible(change) => {
                self.collapsible_open = next_open(self.collapsible_open, change);
            }
            Message::Calendar(event) => {
                let task = event.focus_task("showcase-calendar");
                self.calendar_state.apply(&event);
                return task;
            }
            Message::DatePicker(event) => {
                self.date_picker_open = event.next_open(self.date_picker_open);
                if let Some(value) = event.value() {
                    self.date_picker_value = value;
                }
                if let Some(month) = event.month() {
                    self.date_picker_month = month;
                }
                if let Some(focused) = event.focused() {
                    self.date_picker_focused = Some(focused);
                }
                return event.focus_task(&self.date_picker_ids);
            }
            Message::Carousel(event) => {
                self.carousel_state.apply(event);
            }
            Message::PrependTranscriptHistory => {
                let id = self.next_history_id;
                self.next_history_id += 1;
                self.transcript.insert(
                    0,
                    TranscriptRow {
                        id: format!("history-{id}"),
                        copy: format!("Loaded earlier message {id}."),
                        side: Some(MessageSide::Incoming),
                    },
                );
                return self.sync_message_scroller();
            }
            Message::StreamTranscript => {
                if let Some(row) = self
                    .transcript
                    .last_mut()
                    .filter(|row| row.id.starts_with("stream-"))
                {
                    row.copy
                        .push_str(" More content arrived in this same stable row.");
                } else {
                    let id = self.next_transcript_id;
                    self.next_transcript_id += 1;
                    self.transcript.extend([
                        TranscriptRow {
                            id: format!("turn-{id}"),
                            copy: format!("New turn {id}"),
                            side: None,
                        },
                        TranscriptRow {
                            id: format!("stream-{id}"),
                            copy: "Streaming into this stable row…".into(),
                            side: Some(if id.is_multiple_of(2) {
                                MessageSide::Outgoing
                            } else {
                                MessageSide::Incoming
                            }),
                        },
                    ]);
                }
                return self.sync_message_scroller();
            }
            Message::TranscriptScroll(event) => {
                return self
                    .message_scroller
                    .update(event)
                    .map(Message::TranscriptScroll);
            }
            Message::FocusTraversal { backwards } => {
                let focus = if backwards {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
                return focus.chain(self.sonner.focus_task().map(Message::Sonner));
            }
            Message::TabsAutomatic(event) => {
                self.tabs_automatic.apply(&event);
                return event.focus_task();
            }
            Message::TabsManual(event) => {
                self.tabs_manual.apply(&event);
                return event.focus_task();
            }
            Message::OtpChanged(value) => self.otp = value,
            Message::RadioSelected(value) => {
                self.radio_selected = value;
                let index = ["default", "comfortable", "compact", "disabled"]
                    .iter()
                    .position(|candidate| candidate == &value)
                    .unwrap_or(0);
                return focus_radio("density", index);
            }
            Message::SliderChanged(values) => self.slider_values = values,
            Message::FocusFirstSliderThumb => return focus_slider_thumb("temperature", 0),
            Message::StandaloneToggle => self.standalone_toggle = !self.standalone_toggle,
            Message::ToggleGroupValue(value) => {
                self.toggle_group_state = self.toggle_group_state.toggled(value);
            }
            Message::ToggleGroupNavigate(index) => {
                let id = ["bold", "italic", "underline"]
                    .get(index)
                    .copied()
                    .unwrap_or("bold");
                return iced::widget::operation::focus(iced::widget::Id::from(format!(
                    "toggle-group-{id}"
                )));
            }
            Message::SwitchToggle => self.switch_on = !self.switch_on,
            Message::Resized(sizes) => self.resizable_sizes = sizes,
            Message::FocusFirstResizeHandle => return focus_resizable_handle("workspace", 0),
            Message::OpenDialog => {
                let was_open = self.dialog_open;
                self.alert_dialog_open = false;
                self.dialog_open = true;
                return self
                    .dialog_focus
                    .transition_task(was_open, self.dialog_open);
            }
            Message::CloseDialog => {
                let was_open = self.dialog_open;
                self.dialog_open = false;
                return self
                    .dialog_focus
                    .transition_task(was_open, self.dialog_open);
            }
            Message::SaveDialog => {
                let was_open = self.dialog_open;
                self.dialog_open = false;
                let now = self.now();
                self.sonner.success("Dialog changes saved", now);
                return self
                    .dialog_focus
                    .transition_task(was_open, self.dialog_open);
            }
            Message::Dialog(event) => {
                let focus = event.focus_task();
                let was_open = self.dialog_open;
                self.dialog_open = ui::dialog::next_open(self.dialog_open, &event);
                return iced::Task::batch([
                    focus,
                    self.dialog_focus
                        .transition_task(was_open, self.dialog_open),
                ]);
            }
            Message::OpenAlertDialog => {
                let was_open = self.alert_dialog_open;
                self.dialog_open = false;
                self.alert_dialog_open = true;
                return self
                    .alert_dialog_focus
                    .scope()
                    .transition_task(was_open, self.alert_dialog_open);
            }
            Message::AlertDialog(event) => {
                let focus = event.focus_task();
                let was_open = self.alert_dialog_open;
                self.alert_dialog_open = next_alert_dialog_open(self.alert_dialog_open, &event);
                let restore = self
                    .alert_dialog_focus
                    .scope()
                    .transition_task(was_open, self.alert_dialog_open);
                if matches!(event, AlertDialogEvent::Action) {
                    let now = self.now();
                    self.sonner.success("Destructive action confirmed", now);
                }
                return iced::Task::batch([focus, restore]);
            }
            Message::ShowToast(variant) => {
                let now = self.now();
                let id = match variant {
                    ToastVariant::Default => self.sonner.show("Default notification", now),
                    ToastVariant::Success => self.sonner.success("Changes saved", now),
                    ToastVariant::Info => self.sonner.info("New version available", now),
                    ToastVariant::Warning => self.sonner.warning("Connection is unstable", now),
                    ToastVariant::Destructive => self.sonner.error("Could not save changes", now),
                    ToastVariant::Loading => self.sonner.loading("Publishing components…", now),
                };
                if variant == ToastVariant::Loading {
                    self.loading_toast = Some(id);
                }
            }
            Message::ResolveLoadingToast => {
                if let Some(id) = self.loading_toast.take() {
                    let now = self.now();
                    self.sonner.replace(
                        id,
                        ToastData::new("Published")
                            .description("The component batch is live.")
                            .action("View")
                            .variant(ToastVariant::Success)
                            .duration(Duration::from_secs(4)),
                        now,
                    );
                }
            }
            Message::ClearToasts => {
                self.sonner.clear();
                self.loading_toast = None;
            }
            Message::CycleToastPlacement => {
                const PLACEMENTS: [ToastPlacement; 6] = [
                    ToastPlacement::TopLeft,
                    ToastPlacement::TopCenter,
                    ToastPlacement::TopRight,
                    ToastPlacement::BottomLeft,
                    ToastPlacement::BottomCenter,
                    ToastPlacement::BottomRight,
                ];
                let current = PLACEMENTS
                    .iter()
                    .position(|placement| *placement == self.sonner.placement())
                    .unwrap_or(0);
                self.sonner
                    .set_placement(PLACEMENTS[(current + 1) % PLACEMENTS.len()]);
            }
            Message::ToggleToastExpanded => {
                self.sonner.set_expanded(!self.sonner.is_expanded());
            }
            Message::ToggleReducedMotion => {
                self.sonner
                    .set_reduced_motion(!self.sonner.reduced_motion());
            }
            Message::ToggleSwipeDirection => {
                self.sonner.set_swipe_direction(
                    if self.sonner.swipe_direction() == SwipeDirection::Right {
                        SwipeDirection::Left
                    } else {
                        SwipeDirection::Right
                    },
                );
            }
            Message::Sonner(event) => {
                let action = matches!(event, SonnerEvent::Action(_));
                let now = self.now();
                if let SonnerOutcome::Action(id) = self.sonner.update(event, now) {
                    self.sonner.replace(
                        id,
                        ToastData::new("Action completed").variant(ToastVariant::Success),
                        now,
                    );
                }
                if action {
                    return self.sonner.focus_task().map(Message::Sonner);
                }
            }
            Message::Tick(now) => {
                self.sonner
                    .tick(now.saturating_duration_since(self.started_at));
            }
            Message::Popover(event) => {
                self.popover_open = next_popover_open(event);
                return event.focus_task(&self.popover_ids);
            }
            Message::Dropdown(event) => {
                self.dropdown_open = event.open(self.dropdown_open);
                if let DropdownMenuEvent::Menu(menu_event) = &event {
                    self.apply_menu_event(menu_event);
                }
                let entries = showcase_menu_entries(self.menu_bookmarked, self.menu_density);
                return event.focus_task(
                    &DropdownMenuIds::new("showcase"),
                    &entries,
                    &self.dropdown_state,
                );
            }
            Message::Context(event) => {
                self.context_anchor = event.anchor(self.context_anchor);
                self.context_open = event.open(self.context_open);
                if let ContextMenuEvent::Menu(menu_event) = &event {
                    self.apply_context_menu_event(menu_event);
                }
                let entries = showcase_menu_entries(self.menu_bookmarked, self.menu_density);
                return event.focus_task(
                    &ContextMenuIds::new("showcase"),
                    &entries,
                    &self.context_state,
                );
            }
            Message::Menubar(event) => return self.update_menubar(event),
            Message::Navigation(event) => {
                self.navigation_state = event.state().clone();
                return event.focus_task("showcase-navigation");
            }
            Message::Select(event) => {
                self.select_open = event.open(self.select_open);
                match &event {
                    SelectEvent::Selected(value) => self.select_value = Some(*value),
                    SelectEvent::Menu(MenuEvent::StateChanged(state)) => {
                        self.select_state = state.clone();
                    }
                    SelectEvent::OpenChanged { .. } | SelectEvent::Menu(_) => {}
                }
                let groups = showcase_select_groups();
                return event.focus_task(&SelectIds::new("showcase"), &groups, &self.select_state);
            }
            Message::ChartHover(hit) => self.chart_hover = hit,
            Message::PieHover(hit) => self.pie_hover = hit,
            Message::Command(event) => {
                if let Some(selection) = event.selection() {
                    self.command_selection = Some(*selection);
                }
                self.command_state.apply(&event);
                return event.focus_task("showcase-command");
            }
            Message::FocusCommandInput => return focus_command_input("showcase-command"),
            Message::Sidebar(action) => {
                self.sidebar_state = self.sidebar_state.reduced(action);
            }
            Message::OpenSheet => {
                let was_open = self.sheet_open;
                self.sheet_open = true;
                return self.sheet_focus.transition_task(was_open, self.sheet_open);
            }
            Message::CloseSheet => {
                let was_open = self.sheet_open;
                self.sheet_open = false;
                return self.sheet_focus.transition_task(was_open, self.sheet_open);
            }
            Message::Sheet(event) => {
                let focus = event.focus_task();
                let was_open = self.sheet_open;
                self.sheet_open = ui::dialog::next_open(self.sheet_open, &event);
                return iced::Task::batch([
                    focus,
                    self.sheet_focus.transition_task(was_open, self.sheet_open),
                ]);
            }
            Message::OpenDrawer => return self.drawer_state.set_open(true, &self.drawer_focus),
            Message::CloseDrawer => return self.drawer_state.set_open(false, &self.drawer_focus),
            Message::Drawer(event) => {
                let focus = event.focus_task(&self.drawer_focus);
                self.drawer_state.apply(&event);
                return focus;
            }
        }

        iced::Task::none()
    }

    fn sync_message_scroller(&mut self) -> iced::Task<Message> {
        let items = self
            .transcript
            .iter()
            .map(|row| MessageScrollerItemMeta::new(&row.id).scroll_anchor(row.side.is_none()))
            .collect();
        self.message_scroller
            .update(MessageScrollerEvent::ItemsChanged(items))
            .map(Message::TranscriptScroll)
    }

    fn apply_menu_event(&mut self, event: &MenuEvent) {
        match event {
            MenuEvent::StateChanged(state) => self.dropdown_state = state.clone(),
            MenuEvent::Activated(activation) => self.apply_menu_activation(activation),
            MenuEvent::Dismiss | MenuEvent::MoveTopLevel(_) => {}
        }
    }

    fn apply_context_menu_event(&mut self, event: &MenuEvent) {
        match event {
            MenuEvent::StateChanged(state) => self.context_state = state.clone(),
            MenuEvent::Activated(activation) => self.apply_menu_activation(activation),
            MenuEvent::Dismiss | MenuEvent::MoveTopLevel(_) => {}
        }
    }

    fn apply_menu_activation(&mut self, activation: &MenuActivation) {
        match &activation.kind {
            MenuActivationKind::Checkbox { checked } if activation.id == "bookmark" => {
                self.menu_bookmarked = *checked;
            }
            MenuActivationKind::Radio { group } if group == "density" => {
                self.menu_density = match activation.id.as_str() {
                    "density-compact" => "compact",
                    _ => "comfortable",
                };
            }
            MenuActivationKind::Action
            | MenuActivationKind::Checkbox { .. }
            | MenuActivationKind::Radio { .. } => {}
        }
        self.menu_last_action = Some(activation.id.clone());
        self.menu_entries = showcase_menu_entries(self.menu_bookmarked, self.menu_density);
    }

    fn update_menubar(&mut self, event: MenubarEvent) -> iced::Task<Message> {
        let previous_open = self.menubar_state.open;
        if let MenubarEvent::Menu { event, .. } = &event {
            match event {
                MenuEvent::StateChanged(state) => self.menubar_menu_state = state.clone(),
                MenuEvent::Activated(activation) => self.apply_menu_activation(activation),
                MenuEvent::Dismiss | MenuEvent::MoveTopLevel(_) => {}
            }
        }

        let next = event.state(&self.menubar_state);
        self.menubar_state = next;
        let menus = showcase_menubar_menus(self.menu_bookmarked, self.menu_density);
        if self.menubar_state.open != previous_open
            && let Some(index) = self.menubar_state.open
        {
            self.menubar_menu_state = MenuState::initial(&menus[index].entries);
        }

        match &event {
            MenubarEvent::Menu {
                menu_id,
                event: MenuEvent::StateChanged(_),
            } => menus.iter().find(|menu| &menu.id == menu_id).map_or_else(
                iced::Task::none,
                |menu| {
                    focus_menu_state(
                        &format!("menubar:showcase:{}", menu.id),
                        &menu.entries,
                        &self.menubar_menu_state,
                    )
                },
            ),
            MenubarEvent::Menu {
                event: MenuEvent::Activated(_) | MenuEvent::Dismiss,
                ..
            } => self
                .menubar_state
                .focused
                .map_or_else(iced::Task::none, |index| {
                    iced::widget::operation::focus(ui::menubar::menubar_trigger_id(
                        "showcase", index,
                    ))
                }),
            _ => event.focus_task("showcase", &menus, &self.menubar_menu_state),
        }
    }

    fn now(&self) -> Duration {
        self.started_at.elapsed()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let keyboard = iced::event::listen_with(|event, status, _window| {
            if status != iced::event::Status::Ignored {
                return None;
            }

            match event {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    repeat: false,
                    ..
                }) => shortcut_action(&key, modifiers, SidebarViewport::Desktop)
                    .map(Message::Sidebar)
                    .or_else(|| {
                        matches!(
                            key,
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab)
                        )
                        .then_some(Message::FocusTraversal {
                            backwards: modifiers.shift(),
                        })
                    }),
                _ => None,
            }
        });
        let clock = iced::time::every(Duration::from_millis(100)).map(Message::Tick);

        iced::Subscription::batch([keyboard, clock])
    }

    fn ui_theme(&self) -> Theme {
        (if self.dark { DARK } else { LIGHT }).with_accent(ACCENTS[self.accent])
    }

    fn iced_theme(&self) -> IcedTheme {
        self.ui_theme().iced()
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = self.ui_theme();
        let button_examples = row![
            button("Default", &theme).on_press(Message::Clicked),
            button("Secondary", &theme)
                .variant(ButtonVariant::Secondary)
                .on_press(Message::Clicked),
            button("Outline", &theme)
                .variant(ButtonVariant::Outline)
                .on_press(Message::Clicked),
            button("Ghost", &theme)
                .variant(ButtonVariant::Ghost)
                .on_press(Message::Clicked),
            button("Destructive", &theme)
                .variant(ButtonVariant::Destructive)
                .on_press(Message::Clicked),
            button("Disabled", &theme).disabled(true),
        ]
        .spacing(theme.spacing.sm)
        .wrap();

        let badges = row![
            badge("Default", BadgeVariant::Default, &theme),
            badge("Secondary", BadgeVariant::Secondary, &theme),
            badge("Destructive", BadgeVariant::Destructive, &theme),
            badge("Success", BadgeVariant::Success, &theme),
            badge("Warning", BadgeVariant::Warning, &theme),
            badge("Outline", BadgeVariant::Outline, &theme),
            badge("Small", BadgeVariant::Secondary, &theme).size(BadgeSize::Small),
        ]
        .spacing(theme.spacing.sm)
        .wrap();

        let statuses = row![
            badge("Operational", BadgeVariant::Success, &theme).dot(),
            badge("Degraded", BadgeVariant::Warning, &theme)
                .size(BadgeSize::Small)
                .dot(),
            badge("Offline", BadgeVariant::Destructive, &theme).dot(),
        ]
        .spacing(theme.spacing.sm)
        .wrap();

        let surfaces = row![
            surface(text("Default"), SurfaceVariant::Default, &theme)
                .padding(theme.spacing.md)
                .width(Length::FillPortion(1)),
            surface(text("Card"), SurfaceVariant::Card, &theme)
                .padding(theme.spacing.md)
                .width(Length::FillPortion(1)),
            surface(text("Muted"), SurfaceVariant::Muted, &theme)
                .padding(theme.spacing.md)
                .width(Length::FillPortion(1)),
            surface(text("Popover"), SurfaceVariant::Popover, &theme)
                .padding(theme.spacing.md)
                .width(Length::FillPortion(1)),
        ]
        .spacing(theme.spacing.sm)
        .width(Length::Fill);

        let sections = segmented_control(
            [
                (Section::General, "General"),
                (Section::Advanced, "Advanced"),
            ],
            self.section,
            Message::SectionSelected,
            &theme,
        );

        let alerts = column![
            alert(
                text("Default: configuration is ready."),
                AlertVariant::Default,
                &theme,
            ),
            alert(
                text("Success: local checks passed."),
                AlertVariant::Success,
                &theme,
            ),
            alert(
                text("Warning: review the generated source."),
                AlertVariant::Warning,
                &theme,
            ),
            alert(
                text("Error: the operation could not finish."),
                AlertVariant::Destructive,
                &theme,
            ),
        ]
        .spacing(theme.spacing.sm);

        let progress_value = (self.clicks % 11) as f32 * 10.0;
        let progress_examples = column![
            progress(progress_value, ProgressVariant::Default, &theme),
            progress(progress_value, ProgressVariant::Success, &theme),
            progress(progress_value, ProgressVariant::Warning, &theme),
            progress(progress_value, ProgressVariant::Destructive, &theme),
            text(format!("{progress_value:.0}% — press a button to advance"))
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
        ]
        .spacing(theme.spacing.sm);

        let notes_invalid = self.notes.text().trim().is_empty();
        let notes = field(
            "Notes",
            textarea(
                &self.notes,
                "Write a multiline note…",
                Message::NotesChanged,
                if notes_invalid {
                    TextareaVariant::Invalid
                } else {
                    TextareaVariant::Default
                },
                &theme,
            ),
            notes_invalid.then_some(FieldHint::Error("A note is required.")),
            &theme,
        );

        let breadcrumb_example = breadcrumb(
            [
                BreadcrumbItem::link(
                    button("Home", &theme)
                        .variant(ButtonVariant::Link)
                        .on_press(Message::Clicked),
                ),
                BreadcrumbItem::link(
                    button("Components", &theme)
                        .variant(ButtonVariant::Link)
                        .on_press(Message::Clicked),
                ),
                BreadcrumbItem::current(text("Showcase")),
            ],
            || breadcrumb_separator(&theme),
            &theme,
        );

        let page = self.page.max(1);
        let pagination_example = pagination(
            [
                PaginationItem::Previous((page > 1).then_some(page - 1)),
                PaginationItem::Page {
                    number: 1,
                    current: page == 1,
                },
                PaginationItem::Page {
                    number: 2,
                    current: page == 2,
                },
                PaginationItem::Ellipsis,
                PaginationItem::Page {
                    number: 10,
                    current: page == 10,
                },
                PaginationItem::Next((page < 10).then_some((page + 1).min(10))),
            ],
            Message::PageSelected,
            &theme,
        );

        let item_example = item(
            Some(avatar_fallback("DU", AvatarSize::Small, &theme).into()),
            "ducktape-ui",
            Some("Source-owned iced components"),
            Some(badge("Local", BadgeVariant::Secondary, &theme).into()),
            &theme,
        );

        let preview_theme = theme;
        let ratio_example = aspect_ratio(16.0 / 9.0, move || {
            surface(
                text("16:9 responsive content"),
                SurfaceVariant::Muted,
                &preview_theme,
            )
            .center(Length::Fill)
            .into()
        })
        .width(Length::Fill)
        .height(180);

        let type_examples = column![
            typography("Heading one", TextRole::H1, &theme),
            typography("Heading two", TextRole::H2, &theme),
            typography("Heading three", TextRole::H3, &theme),
            typography("Heading four", TextRole::H4, &theme),
            typography("Paragraph role for body copy.", TextRole::Paragraph, &theme),
            typography("Lead copy introduces a section.", TextRole::Lead, &theme),
            typography("Large emphasized copy", TextRole::Large, &theme),
            typography("Small supporting copy", TextRole::Small, &theme),
            typography("Muted secondary copy", TextRole::Muted, &theme),
            typography("plain_inline_code", TextRole::InlineCode, &theme),
            inline_code("cargo add iced", &theme),
        ]
        .spacing(theme.spacing.sm);

        let horizontal_group = button_group(
            [
                button("One", &theme)
                    .variant(ButtonVariant::Ghost)
                    .on_press(Message::Clicked)
                    .into(),
                button("Two", &theme)
                    .variant(ButtonVariant::Ghost)
                    .on_press(Message::Clicked)
                    .into(),
                button("Three", &theme)
                    .variant(ButtonVariant::Ghost)
                    .on_press(Message::Clicked)
                    .into(),
            ],
            ButtonGroupOrientation::Horizontal,
            &theme,
        );
        let vertical_group = button_group(
            [
                button("Top", &theme)
                    .variant(ButtonVariant::Ghost)
                    .on_press(Message::Clicked)
                    .into(),
                button("Bottom", &theme)
                    .variant(ButtonVariant::Ghost)
                    .on_press(Message::Clicked)
                    .into(),
            ],
            ButtonGroupOrientation::Vertical,
            &theme,
        );
        let grouped_input = input_group(
            Some(text("@").color(theme.palette.muted_foreground).into()),
            group_input("username", &self.email, &theme).on_input(Message::EmailChanged),
            Some(
                button("Find", &theme)
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Small)
                    .on_press(Message::Clicked)
                    .into(),
            ),
            InputVariant::Default,
            &theme,
        );
        let invalid_grouped_input = input_group(
            None,
            group_input("required", &self.email, &theme).on_input(Message::EmailChanged),
            None,
            InputVariant::Invalid,
            &theme,
        );

        let attachment_example = attachment(
            Some(kbd("PDF", &theme).into()),
            "design-system.pdf",
            Some("2.4 MB"),
            Some(
                button("Remove", &theme)
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Small)
                    .on_press(Message::Clicked)
                    .into(),
            ),
            &theme,
        );
        let transcript_items = self.transcript.iter().map(|row| {
            let content: Element<'_, Message> = match row.side {
                Some(side) => message(
                    side,
                    (side == MessageSide::Incoming)
                        .then(|| avatar_fallback("D", AvatarSize::Small, &theme).into()),
                    Some(
                        text(if side == MessageSide::Incoming {
                            "Ducktape"
                        } else {
                            "You"
                        })
                        .size(theme.typography.sm)
                        .into(),
                    ),
                    bubble(
                        text(&row.copy),
                        if side == MessageSide::Incoming {
                            BubbleVariant::Incoming
                        } else {
                            BubbleVariant::Outgoing
                        },
                        &theme,
                    ),
                    None,
                    &theme,
                )
                .into(),
                None => marker(None, &row.copy, MarkerVariant::Separator, &theme).into(),
            };
            message_scroller_item(row.id.clone(), content).scroll_anchor(row.side.is_none())
        });
        let transcript = column![
            row![
                button("Prepend history", &theme)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Small)
                    .on_press(Message::PrependTranscriptHistory),
                button(
                    if self
                        .transcript
                        .last()
                        .is_some_and(|row| row.id.starts_with("stream-"))
                    {
                        "Grow streamed row"
                    } else {
                        "Start anchored stream"
                    },
                    &theme,
                )
                .size(ButtonSize::Small)
                .on_press(Message::StreamTranscript),
            ]
            .spacing(theme.spacing.sm),
            controlled_message_scroller(
                &self.message_scroller,
                transcript_items,
                Message::TranscriptScroll,
                &theme,
            )
            .height(260),
            text(format!(
                "following={} start={} end={} unread={} visible={:?} anchor={:?}",
                self.message_scroller.is_following(),
                self.message_scroller.can_scroll_start(),
                self.message_scroller.can_scroll_end(),
                self.message_scroller.unread_count(),
                self.message_scroller.visible_message_ids(),
                self.message_scroller.current_anchor_id(),
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
        ]
        .spacing(theme.spacing.sm);

        let table_theme_a = theme;
        let table_theme_b = theme;
        let component_table = ui::table::table(
            [
                ui::table::column(
                    ui::table::header("Component", &theme),
                    move |row: (&'static str, &'static str)| {
                        ui::table::cell(text(row.0), &table_theme_a)
                    },
                )
                .width(Length::Fill),
                ui::table::column(
                    ui::table::header("Status", &theme),
                    move |row: (&'static str, &'static str)| {
                        ui::table::cell(text(row.1), &table_theme_b)
                    },
                ),
            ],
            [
                ("Button", "Shipped"),
                ("Table", "Shipped"),
                ("Dialog", "Shipped"),
            ],
            &theme,
        );
        let table_example = column![
            ui::table::caption("Registry status", &theme),
            ui::table::frame(component_table, &theme),
        ]
        .spacing(theme.spacing.sm);

        let scrolling_example = scroll_area(
            column![
                text("Native scroll area").size(theme.typography.base),
                text("Line one"),
                text("Line two"),
                text("Line three"),
                text("Line four"),
                text("Line five"),
                text("Line six"),
            ]
            .spacing(theme.spacing.sm),
            &theme,
        )
        .height(100);
        let next_spinner_frame =
            ui::spinner::next_frame(self.clicks as u8 % ui::spinner::FRAME_COUNT, false);
        let spinner_examples = row![
            ui::spinner::spinner(next_spinner_frame, false, &theme),
            ui::spinner::spinner(0, true, &theme),
            text(format!(
                "Loading… ({} ms frame interval)",
                ui::spinner::TICK_INTERVAL.as_millis()
            )),
        ]
        .spacing(theme.spacing.sm)
        .align_y(iced::Alignment::Center);

        let rtl_items: [Element<'_, Message>; 3] = [
            text("first").into(),
            text("second").into(),
            text("third").into(),
        ];
        let direction_examples = column![
            text("LTR end")
                .width(Length::Fill)
                .align_x(Direction::LeftToRight.end()),
            text("RTL start")
                .width(Length::Fill)
                .align_x(Direction::RightToLeft.start()),
            directed_row(rtl_items, Direction::RightToLeft).spacing(theme.spacing.sm),
        ]
        .spacing(theme.spacing.sm);

        let combobox_example = combobox(
            &self.combo,
            "Search components…",
            self.combo_selected.as_ref(),
            Message::ComboSelected,
            &theme,
        );
        let native_select_example = native_select_with_id(
            iced::widget::Id::new("showcase-native-select"),
            self.select_options.as_slice(),
            self.native_selected.as_ref(),
            Message::NativeSelected,
            &theme,
        )
        .placeholder("Choose a theme…")
        .direction(if self.dark {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        })
        .disabled(false)
        .invalid(false);
        let command_groups = [
            command_group_without_heading([command_item("search", "search", "Search files")
                .keywords(["find", "open"])
                .shortcut("⌘ K")]),
            command_group(
                "Workspace",
                [
                    command_item("new", "new", "New workspace")
                        .keywords(["create", "project"])
                        .shortcut("⌘ N"),
                    command_item("settings", "settings", "Open settings")
                        .keywords(["preferences", "configuration"])
                        .shortcut("⌘ ,"),
                    command_item("archive", "archive", "Archive workspace")
                        .keywords(["delete", "remove"])
                        .disabled(true),
                ],
            ),
        ];
        let command_example = command(
            "showcase-command",
            &self.command_state,
            command_groups,
            Message::Command,
            &theme,
        )
        .placeholder("Type a command or search…")
        .empty("No commands found.")
        .results_height(220.0)
        .width(Length::Fill)
        .group_separators(true);
        let command_meta = format!(
            "query={:?}; active={:?}; selected={:?}",
            self.command_state.query(),
            self.command_state.active(),
            self.command_selection,
        );

        let accordion_example = accordion(
            [
                accordion_item(
                    "install",
                    iced::widget::Id::new("showcase-accordion-install"),
                    text("How is it installed?"),
                    text("The CLI copies editable Rust source into src/ui."),
                ),
                accordion_item(
                    "state",
                    iced::widget::Id::new("showcase-accordion-state"),
                    text("Who owns state?"),
                    text("The application owns controlled state and messages."),
                ),
            ],
            &self.accordion_state,
            Message::Accordion,
            &theme,
        );
        let accordion_targets = [
            header_target(0, 2, AccordionKey::ArrowUp),
            header_target(0, 2, AccordionKey::ArrowDown),
            header_target(1, 2, AccordionKey::Home),
            header_target(0, 2, AccordionKey::End),
        ];
        let multiple_accordion = AccordionState::Multiple(vec!["install"]);
        let collapsible_example = collapsible(
            self.collapsible_open,
            button(
                if self.collapsible_open {
                    "Hide details"
                } else {
                    "Show details"
                },
                &theme,
            )
            .variant(ButtonVariant::Outline)
            .on_press(Message::Collapsible(CollapsibleChange::Toggle)),
            surface(
                text("Controlled content is absent from the tree while closed."),
                SurfaceVariant::Muted,
                &theme,
            )
            .padding(theme.spacing.md),
            &theme,
        );
        let forced_collapsible_states = (
            next_open(false, CollapsibleChange::Open),
            next_open(true, CollapsibleChange::Close),
        );

        let mut data_table_state = DataTableState::new(2);
        data_table_state.set_query("ship");
        data_table_state.toggle_sort("component");
        data_table_state.toggle_sort("component");
        data_table_state.set_page(9, 5);
        let data_page_count = data_table_state.page_count(5);
        let data_range = data_table_state.visible_range(5);

        let calendar_example = controlled_calendar(
            "showcase-calendar",
            &self.calendar_state,
            Message::Calendar,
            &theme,
        )
        .today(Some(
            Date::new(2026, 7, 16).expect("showcase today is valid"),
        ))
        .min(Date::new(2026, 6, 15).ok())
        .max(Date::new(2026, 8, 20).ok())
        .disabled_dates(|date| date.day() == 13 || date.day() == 24)
        .show_outside_days(true)
        .week_numbers(self.dark)
        .month_dropdown(true)
        .year_dropdown(true)
        .year_range(2024, 2028)
        .direction(if self.dark {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        });
        let calendar_meta = format!(
            "month={:04}-{:02} days={} selection={:?} focused={:?} width={:.0}",
            self.calendar_state.month().year(),
            self.calendar_state.month().number(),
            self.calendar_state.month().days(),
            self.calendar_state.selection(),
            self.calendar_state.focused_date(),
            calendar_example.width(),
        );
        let date_picker_example = date_picker(
            self.date_picker_ids.clone(),
            self.date_picker_month,
            self.date_picker_focused,
            &self.date_picker_value,
            self.date_picker_open,
            Message::DatePicker,
            &theme,
        )
        .today(Some(
            Date::new(2026, 7, 16).expect("showcase today is valid"),
        ))
        .min(Date::new(2026, 6, 15).ok())
        .max(Date::new(2026, 8, 20).ok())
        .disabled_dates(|date| date.day() == 13 || date.day() == 24)
        .show_outside_days(true)
        .week_numbers(self.dark)
        .month_dropdown(true)
        .year_dropdown(true)
        .year_range(2024, 2028)
        .placeholder("Choose a date range…")
        .format(DateFormat::MonthDayYear)
        .direction(if self.dark {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        })
        .width(272.0);
        let date_picker_meta = format!(
            "open={} value={:?} label={:?} month={} focused={:?}",
            self.date_picker_open,
            self.date_picker_value,
            format_value(&self.date_picker_value, |date| DateFormat::Iso.format(date)),
            self.date_picker_month,
            self.date_picker_focused,
        );

        let slides: Vec<Element<'_, Message>> =
            ["Source-owned", "Focus-scoped keys", "Pointer + touch swipe"]
                .into_iter()
                .map(|copy| {
                    surface(text(copy), SurfaceVariant::Muted, &theme)
                        .center_x(Length::Fill)
                        .center_y(120)
                        .into()
                })
                .collect();
        let carousel_direction = if self.dark {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let carousel_orientation = CarouselOrientation::Horizontal;
        let previous_slide = carousel_previous(
            iced::widget::Id::new("showcase-carousel-previous"),
            self.carousel_state,
            Message::Carousel(CarouselEvent::Navigate(CarouselCommand::Previous)),
            carousel_orientation,
            carousel_direction,
            &theme,
        );
        let next_slide = carousel_next(
            iced::widget::Id::new("showcase-carousel-next"),
            self.carousel_state,
            Message::Carousel(CarouselEvent::Navigate(CarouselCommand::Next)),
            carousel_orientation,
            carousel_direction,
            &theme,
        );
        let carousel_view = controlled_carousel(
            iced::widget::Id::new("showcase-carousel-viewport"),
            self.carousel_state,
            slides,
            previous_slide,
            next_slide,
            carousel_orientation,
            carousel_direction,
            Message::Carousel,
            &theme,
        );
        let carousel_steps = container(carousel_indicators(
            self.carousel_state,
            |index| iced::widget::Id::from(format!("showcase-carousel-indicator-{index}")),
            |index| Message::Carousel(CarouselEvent::Select(index)),
            carousel_orientation,
            carousel_direction,
            &theme,
        ))
        .width(Length::Fill)
        .center_x(Length::Fill);
        let carousel_example = column![carousel_view, carousel_steps]
            .spacing(theme.spacing.sm)
            .align_x(carousel_direction.start());
        let bounded_edge = CarouselState::new(99, 3, CarouselBoundary::Bounded)
            .reduce(CarouselCommand::First)
            .reduce(CarouselCommand::Last);
        let carousel_key_examples = (
            keyboard_command(
                &iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight),
                CarouselOrientation::Horizontal,
            ),
            keyboard_command(
                &iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp),
                CarouselOrientation::Vertical,
            ),
            keyboard_command(
                &iced::keyboard::Key::Named(iced::keyboard::key::Named::Home),
                CarouselOrientation::Horizontal,
            ),
            keyboard_command(
                &iced::keyboard::Key::Named(iced::keyboard::key::Named::End),
                CarouselOrientation::Horizontal,
            ),
        );

        let focus_theme = theme;
        let focusable = ui::focus_control::focus_control(
            iced::widget::Id::new("showcase-focus-control"),
            container(text("Tab here, then press Enter or Space"))
                .padding(theme.spacing.md)
                .width(Length::Fill),
            Message::Clicked,
            &theme,
        );
        let _focus_id = focusable.id().clone();
        let focusable = focusable
            .disabled(false)
            .style(move |_iced_theme, status| ui::focus_control::style(&focus_theme, status));

        let disabled_reports = tab(
            "reports",
            iced::widget::Id::new("tabs-reports"),
            text("Reports").size(theme.typography.sm),
            text("Disabled reports panel"),
        )
        .disabled(true);
        let _disabled_focus_id = disabled_reports.focus_id().clone();
        let disabled_tab_info = format!(
            "{} disabled={}",
            disabled_reports.id(),
            disabled_reports.is_disabled()
        );
        let automatic_tabs = tabs(
            &self.tabs_automatic,
            [
                tab(
                    "overview",
                    iced::widget::Id::new("tabs-overview"),
                    text("Overview").size(theme.typography.sm),
                    surface(
                        text("Overview panel: arrow keys move focus and selection."),
                        SurfaceVariant::Muted,
                        &theme,
                    )
                    .padding(theme.spacing.md),
                ),
                tab(
                    "analytics",
                    iced::widget::Id::new("tabs-analytics"),
                    text("Analytics").size(theme.typography.sm),
                    surface(text("Analytics panel"), SurfaceVariant::Muted, &theme)
                        .padding(theme.spacing.md),
                ),
                disabled_reports,
            ],
            TabsOrientation::Horizontal,
            TabsActivation::Automatic,
            TabsVariant::Default,
            Message::TabsAutomatic,
            &theme,
        );
        let manual_tabs = tabs(
            &self.tabs_manual,
            [
                tab(
                    "account",
                    iced::widget::Id::new("tabs-manual-account"),
                    text("Account").size(theme.typography.sm),
                    text("Manual tabs move focus first; Enter or Space selects."),
                ),
                tab(
                    "password",
                    iced::widget::Id::new("tabs-manual-password"),
                    text("Password").size(theme.typography.sm),
                    text("Password panel"),
                ),
            ],
            TabsOrientation::Vertical,
            TabsActivation::Manual,
            TabsVariant::Line,
            Message::TabsManual,
            &theme,
        );
        let mut cleared_tabs = self.tabs_manual.clone();
        cleared_tabs.clear();

        let otp_complete = is_complete(&self.otp, 6, OtpPattern::Digits);
        let otp_example = input_otp(
            &self.otp,
            6,
            OtpPattern::Digits,
            Message::OtpChanged,
            &theme,
        )
        .groups([3, 3])
        .id(iced::widget::Id::new("showcase-otp"))
        .invalid(self.otp == "000000")
        .disabled(false);
        let disabled_otp = input_otp(
            "A1B2",
            4,
            OtpPattern::Alphanumeric,
            Message::OtpChanged,
            &theme,
        )
        .disabled(true);
        let custom_otp = normalize(
            "ABcd12",
            4,
            OtpPattern::Custom(|character| character.is_ascii_uppercase()),
        );

        let density_radio = radio_group(
            "density",
            [
                radio_option("default", "Default", &theme),
                RadioOption::new(
                    "comfortable",
                    column![
                        text("Comfortable").size(theme.typography.sm),
                        text("More room between items")
                            .size(theme.typography.xs)
                            .color(theme.palette.muted_foreground),
                    ]
                    .spacing(theme.spacing.xs),
                ),
                radio_option("compact", "Compact", &theme),
                radio_option("disabled", "Unavailable", &theme).disabled(true),
            ],
            Some(self.radio_selected),
            Message::RadioSelected,
            &theme,
        )
        .orientation(RadioOrientation::Vertical)
        .disabled(false)
        .invalid(false);
        let disabled_radio = radio_group(
            "disabled-density",
            [
                radio_option("one", "Disabled one", &theme),
                radio_option("two", "Disabled two", &theme),
            ],
            Some("one"),
            Message::RadioSelected,
            &theme,
        )
        .orientation(RadioOrientation::Horizontal)
        .disabled(true)
        .invalid(true);

        let temperature_slider = slider(
            "temperature",
            self.slider_values.clone(),
            0.0..=100.0,
            5.0,
            Message::SliderChanged,
            &theme,
        )
        .orientation(SliderOrientation::Horizontal)
        .reversed(false)
        .disabled(false)
        .invalid(false)
        .page_step(20.0)
        .width(Length::Fill)
        .height(32);
        let vertical_slider = slider(
            "vertical-disabled",
            vec![35.0],
            0.0..=100.0,
            1.0,
            Message::SliderChanged,
            &theme,
        )
        .orientation(SliderOrientation::Vertical)
        .reversed(true)
        .disabled(true)
        .invalid(true)
        .width(32)
        .height(120);

        let standalone_toggles = row![
            toggle(
                iced::widget::Id::new("toggle-bookmark"),
                text("Bookmark").size(theme.typography.sm),
                self.standalone_toggle,
                Message::StandaloneToggle,
                &theme,
            )
            .variant(ToggleVariant::Default)
            .size(ToggleSize::Small)
            .disabled(false),
            toggle(
                iced::widget::Id::new("toggle-outline"),
                text("Outline").size(theme.typography.sm),
                self.standalone_toggle,
                Message::StandaloneToggle,
                &theme,
            )
            .variant(ToggleVariant::Outline)
            .size(ToggleSize::Large),
            toggle(
                iced::widget::Id::new("toggle-disabled"),
                text("Disabled").size(theme.typography.sm),
                false,
                Message::StandaloneToggle,
                &theme,
            )
            .disabled(true),
        ]
        .spacing(theme.spacing.sm)
        .align_y(iced::Alignment::Center);

        let format_group = toggle_group(
            [
                toggle_group_item(
                    iced::widget::Id::new("toggle-group-bold"),
                    "bold",
                    text("Bold").size(theme.typography.sm),
                    Message::ToggleGroupValue("bold"),
                ),
                toggle_group_item(
                    iced::widget::Id::new("toggle-group-italic"),
                    "italic",
                    text("Italic").size(theme.typography.sm),
                    Message::ToggleGroupValue("italic"),
                ),
                toggle_group_item(
                    iced::widget::Id::new("toggle-group-underline"),
                    "underline",
                    text("Underline").size(theme.typography.sm),
                    Message::ToggleGroupValue("underline"),
                )
                .disabled(true),
            ],
            &self.toggle_group_state,
            ToggleGroupOrientation::Horizontal,
            Message::ToggleGroupNavigate,
            &theme,
        )
        .variant(ToggleVariant::Outline)
        .size(ToggleSize::Default)
        .spacing(0.0)
        .disabled(false);
        let single_toggle_state = ToggleGroupState::Single(Some("top"));
        let vertical_toggle_group = toggle_group(
            [
                toggle_group_item(
                    iced::widget::Id::new("toggle-group-top"),
                    "top",
                    text("Top").size(theme.typography.sm),
                    Message::Clicked,
                ),
                toggle_group_item(
                    iced::widget::Id::new("toggle-group-bottom"),
                    "bottom",
                    text("Bottom").size(theme.typography.sm),
                    Message::Clicked,
                ),
            ],
            &self.toggle_group_state,
            ToggleGroupOrientation::Vertical,
            Message::ToggleGroupNavigate,
            &theme,
        )
        .variant(ToggleVariant::Default)
        .size(ToggleSize::Large)
        .spacing(2.0)
        .disabled(true);

        let switches = row![
            switch(
                iced::widget::Id::new("switch-default"),
                self.switch_on,
                Message::SwitchToggle,
                &theme,
            )
            .size(SwitchSize::Default)
            .disabled(false),
            switch(
                iced::widget::Id::new("switch-small"),
                !self.switch_on,
                Message::SwitchToggle,
                &theme,
            )
            .size(SwitchSize::Small),
            switch(
                iced::widget::Id::new("switch-disabled"),
                true,
                Message::SwitchToggle,
                &theme,
            )
            .disabled(true),
        ]
        .spacing(theme.spacing.md)
        .align_y(iced::Alignment::Center);
        let toggle_style = ui::toggle::style(
            &theme,
            ToggleVariant::Outline,
            self.standalone_toggle,
            ui::focus_control::Status::Active,
        );
        let toggle_target = ui::toggle_group::item_target(
            0,
            3,
            &iced::keyboard::Key::Named(iced::keyboard::key::Named::End),
            ToggleGroupOrientation::Horizontal,
        );

        let resize_panels: Vec<Element<'_, Message>> = ["Navigation", "Editor", "Inspector"]
            .into_iter()
            .map(|label| {
                surface(text(label), SurfaceVariant::Muted, &theme)
                    .center(Length::Fill)
                    .into()
            })
            .collect();
        let workspace_resizable = resizable(
            "workspace",
            resize_panels,
            self.resizable_sizes.clone(),
            vec![0.15, 0.20, 0.15],
            Message::Resized,
            &theme,
        )
        .orientation(ResizableOrientation::Horizontal)
        .with_handles(true)
        .handle(0, ResizableHandle::new().disabled(false).with_grip(true))
        .handle(1, ResizableHandle::new().with_grip(false))
        .disabled(false)
        .keyboard_step(0.05)
        .pointer_hit_size(12.0)
        .touch_hit_size(32.0)
        .width(Length::Fill)
        .height(180);
        let vertical_resizable = resizable(
            "vertical-disabled",
            vec![
                surface(text("Top"), SurfaceVariant::Muted, &theme)
                    .center(Length::Fill)
                    .into(),
                surface(text("Bottom"), SurfaceVariant::Muted, &theme)
                    .center(Length::Fill)
                    .into(),
            ],
            vec![0.5, 0.5],
            vec![0.2, 0.2],
            Message::Resized,
            &theme,
        )
        .orientation(ResizableOrientation::Vertical)
        .handle(0, ResizableHandle::new().disabled(true).with_grip(true))
        .disabled(true)
        .width(160)
        .height(120);
        let normalized_resize = ResizableLayout::new(3, &self.resizable_sizes, &[0.15, 0.20, 0.15]);
        let resize_minimums = normalized_resize.minimums().to_vec();

        let modal_triggers = row![
            toggle(
                self.dialog_focus.restore().clone(),
                text("Open dialog"),
                false,
                Message::OpenDialog,
                &theme,
            )
            .variant(ToggleVariant::Outline),
            toggle(
                self.alert_dialog_focus.restore().clone(),
                text("Open alert dialog"),
                false,
                Message::OpenAlertDialog,
                &theme,
            )
            .variant(ToggleVariant::Outline),
            toggle(
                self.sheet_focus.restore().clone(),
                text("Open sheet"),
                false,
                Message::OpenSheet,
                &theme,
            )
            .variant(ToggleVariant::Outline),
            toggle(
                self.drawer_focus.restore().clone(),
                text("Open drawer"),
                false,
                Message::OpenDrawer,
                &theme,
            )
            .variant(ToggleVariant::Outline),
        ]
        .spacing(theme.spacing.sm)
        .wrap();
        let preview_actions: [Element<'_, Message>; 2] = [
            button("Cancel", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .into(),
            button("Continue", &theme).size(ButtonSize::Small).into(),
        ];
        let dialog_alignment_preview = dialog_panel(
            "Centered RTL preview",
            "Text and footer alignment are explicit instead of inherited from a DOM.",
            text("Dialog body content remains caller-owned."),
            directed_row(preview_actions, Direction::RightToLeft).spacing(theme.spacing.sm),
            Direction::RightToLeft,
            DialogAlignment::Center,
            if self.dark {
                DialogActionAlignment::Start
            } else {
                DialogActionAlignment::Center
            },
            &theme,
        );
        let popover_example = popover(
            self.popover_ids.clone(),
            surface(text("Toggle popover"), SurfaceVariant::Default, &theme)
                .padding([theme.spacing.sm, theme.spacing.md]),
            column![
                text("Anchored content").size(theme.typography.lg),
                text("Outside click and Escape restore trigger focus.")
                    .size(theme.typography.sm)
                    .color(theme.palette.muted_foreground),
                button("Close", &theme)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Small)
                    .on_press(Message::Popover(PopoverEvent::Close(
                        ui::popover::DismissReason::Trigger,
                    ))),
            ]
            .spacing(theme.spacing.sm),
            self.popover_open,
            Message::Popover,
            &theme,
        )
        .placement(Placement::Right)
        .alignment(FloatingAlignment::Start)
        .side_offset(8.0)
        .alignment_offset(2.0)
        .viewport_padding(12.0)
        .width(300.0)
        .max_width(340.0)
        .padding(theme.spacing.lg)
        .disabled(false);
        let tooltip_example = tooltip(
            TooltipId::new("showcase-tooltip"),
            surface(text("Focus or hover"), SurfaceVariant::Muted, &theme)
                .padding([theme.spacing.sm, theme.spacing.md]),
            tooltip_text("Noninteractive tooltip copy uses an exact 12/16 baseline."),
            &theme,
        )
        .placement(Placement::Top)
        .alignment(FloatingAlignment::Center)
        .side_offset(6.0)
        .alignment_offset(0.0)
        .viewport_padding(12.0)
        .max_width(280.0)
        .open_delay(Duration::from_millis(450))
        .close_delay(Duration::from_millis(50))
        .padding([6.0, 12.0])
        .disabled(false);
        let hover_card_example = hover_card(
            HoverCardId::new("showcase-hover-card"),
            surface(text("Hover profile"), SurfaceVariant::Default, &theme)
                .padding([theme.spacing.sm, theme.spacing.md]),
            column![
                text("ducktape-ui").size(theme.typography.lg),
                text("Source-owned iced components with gap-safe pointer transfer.")
                    .size(theme.typography.sm)
                    .color(theme.palette.muted_foreground),
                button("Follow", &theme)
                    .size(ButtonSize::Small)
                    .on_press(Message::ShowToast(ToastVariant::Success)),
            ]
            .spacing(theme.spacing.sm),
            &theme,
        )
        .placement(Placement::Bottom)
        .alignment(FloatingAlignment::End)
        .side_offset(8.0)
        .alignment_offset(0.0)
        .viewport_padding(12.0)
        .width(272.0)
        .max_width(320.0)
        .open_delay(Duration::from_millis(500))
        .close_delay(Duration::from_millis(250))
        .padding(theme.spacing.lg)
        .disabled(false);
        let floating_examples = row![popover_example, tooltip_example, hover_card_example]
            .spacing(theme.spacing.md)
            .align_y(iced::Alignment::Center);

        let menu_direction = if self.dark {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let dropdown_example = dropdown_menu(
            DropdownMenuIds::new("showcase"),
            surface(text("Open dropdown"), SurfaceVariant::Default, &theme)
                .padding([theme.spacing.sm, theme.spacing.md]),
            &self.menu_entries,
            &self.dropdown_state,
            self.dropdown_open,
            Message::Dropdown,
            &theme,
        )
        .direction(menu_direction)
        .alignment(if menu_direction == Direction::LeftToRight {
            FloatingAlignment::Start
        } else {
            FloatingAlignment::End
        })
        .width(232.0)
        .into_element();
        let context_example: Element<'_, Message> = context_menu(
            ContextMenuIds::new("showcase"),
            surface(
                column![
                    text("Context region").size(theme.typography.sm),
                    text("Right-click or touch")
                        .size(theme.typography.xs)
                        .color(theme.palette.muted_foreground),
                ]
                .spacing(theme.spacing.xs),
                SurfaceVariant::Muted,
                &theme,
            )
            .padding(theme.spacing.md)
            .width(232),
            &self.menu_entries,
            &self.context_state,
            self.context_open,
            self.context_anchor,
            Message::Context,
            &theme,
        )
        .direction(menu_direction)
        .width(232.0)
        .into();
        let menubar_example: Element<'_, Message> = menubar(
            "showcase",
            showcase_menubar_menus(self.menu_bookmarked, self.menu_density),
            &self.menubar_state,
            &self.menubar_menu_state,
            Message::Menubar,
            &theme,
        )
        .direction(menu_direction)
        .menu_width(232.0)
        .into();
        let select_example = select(
            SelectIds::new("showcase"),
            showcase_select_groups(),
            self.select_value,
            "Choose a value…",
            &self.select_state,
            self.select_open,
            Message::Select,
            &theme,
        )
        .direction(menu_direction)
        .width(232.0)
        .content_width(232.0)
        .into_element();
        let menu_meta = format!(
            "dropdown={} context={} anchor={:?} menubar={:?} select={:?} bookmarked={} density={} last={:?}",
            self.dropdown_open,
            self.context_open,
            self.context_anchor,
            self.menubar_state,
            self.select_value,
            self.menu_bookmarked,
            self.menu_density,
            self.menu_last_action,
        );
        let navigation_components = navigation_menu_list([
            navigation_menu_list_link(
                iced::widget::Id::new("navigation-components-buttons"),
                "Buttons and inputs",
                "Controls with exact sizing, focus, and disabled behavior.",
                Message::ShowToast(ToastVariant::Info),
                menu_direction,
                &theme,
            ),
            navigation_menu_list_link(
                iced::widget::Id::new("navigation-components-overlays"),
                "Overlays",
                "Dialogs, menus, sheets, and anchored floating content.",
                Message::ShowToast(ToastVariant::Success),
                menu_direction,
                &theme,
            ),
        ]);
        let navigation_docs = navigation_menu_list([
            navigation_menu_list_link(
                iced::widget::Id::new("navigation-docs-install"),
                "Installation",
                "Copy source-owned components with the offline CLI.",
                Message::ShowToast(ToastVariant::Default),
                menu_direction,
                &theme,
            ),
            navigation_menu_list_link(
                iced::widget::Id::new("navigation-docs-theme"),
                "Theming",
                "Warm light and dark tokens with runtime accent colors.",
                Message::ShowToast(ToastVariant::Warning),
                menu_direction,
                &theme,
            ),
        ]);
        let navigation_example: Element<'_, Message> = navigation_menu(
            "showcase-navigation",
            [
                NavigationMenuItem::link("home", "Home"),
                NavigationMenuItem::disclosure("components", "Components", navigation_components),
                NavigationMenuItem::disclosure("docs", "Docs", navigation_docs),
                NavigationMenuItem::link("changelog", "Changelog").disabled(true),
            ],
            &self.navigation_state,
            Message::Navigation,
            &theme,
        )
        .direction(menu_direction)
        .collapsed(self.dark)
        .viewport(!self.dark)
        .content_width(520.0)
        .content_min_width(320.0)
        .content_max_width(600.0)
        .width(Length::Fill)
        .into();
        let navigation_meta = format!(
            "focused={:?} open={:?} active={:?} collapsed={} direction={menu_direction:?}",
            self.navigation_state.focused,
            self.navigation_state.open,
            self.navigation_state.active,
            self.dark,
        );

        let sidebar_viewport = if self.dark {
            SidebarViewport::Mobile
        } else {
            SidebarViewport::Desktop
        };
        let sidebar_side = if self.accent == 2 {
            SidebarSide::Right
        } else {
            SidebarSide::Left
        };
        let sidebar_variant = match self.accent {
            0 => SidebarVariant::Sidebar,
            1 => SidebarVariant::Floating,
            _ => SidebarVariant::Inset,
        };
        let sidebar_direction = if self.dark {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let sidebar_collapsible = SidebarCollapsible::Icon;
        let sidebar_collapsed = self
            .sidebar_state
            .is_collapsed(sidebar_viewport, sidebar_collapsible);
        let dashboard_content = sidebar_menu_button_content(
            Some(text("⌂").into()),
            "Dashboard",
            None,
            sidebar_collapsed,
            sidebar_direction,
            &theme,
        );
        let dashboard = sidebar_menu_button(
            SidebarMenuButtonId::new("dashboard"),
            dashboard_content,
            Message::ShowToast(ToastVariant::Info),
            &theme,
        )
        .active(true)
        .disabled(false)
        .collapsed(sidebar_collapsed)
        .tooltip(tooltip_text("Dashboard"))
        .side(sidebar_side)
        .direction(sidebar_direction)
        .size(SidebarMenuButtonSize::Large);
        let projects_content = sidebar_menu_button_content(
            Some(text("◇").into()),
            "Projects",
            None,
            sidebar_collapsed,
            sidebar_direction,
            &theme,
        );
        let projects = sidebar_menu_button(
            SidebarMenuButtonId::new("projects"),
            projects_content,
            Message::ShowToast(ToastVariant::Default),
            &theme,
        )
        .collapsed(sidebar_collapsed)
        .tooltip(tooltip_text("Projects"))
        .side(sidebar_side)
        .direction(sidebar_direction);
        let project_badge: Element<'_, Message> =
            sidebar_menu_badge(text("3"), false, &theme).into();
        let project_item = sidebar_menu_item(
            projects,
            (!sidebar_collapsed).then_some(project_badge),
            sidebar_direction,
        );
        let submenu = sidebar_submenu(
            [sidebar_submenu_item(
                sidebar_submenu_button(
                    SidebarMenuButtonId::new("ducktape"),
                    sidebar_menu_button_content(
                        None,
                        "Ducktape",
                        None,
                        false,
                        sidebar_direction,
                        &theme,
                    ),
                    Message::ShowToast(ToastVariant::Success),
                    &theme,
                ),
                None,
                sidebar_direction,
            )
            .into()],
            sidebar_direction,
            &theme,
        );
        let sidebar_items: Vec<Element<'_, Message>> = vec![
            sidebar_menu_item(dashboard, None, sidebar_direction).into(),
            project_item.into(),
            if sidebar_collapsed {
                sidebar_menu_skeleton(true, true, &theme).into()
            } else {
                submenu.into()
            },
        ];
        let group_action = sidebar_group_action(
            iced::widget::Id::new("showcase-sidebar-group-action"),
            text("+"),
            Message::ShowToast(ToastVariant::Default),
            false,
            &theme,
        );
        let sidebar_group = sidebar_group(column![
            sidebar_group_heading(
                text("Platform"),
                (!sidebar_collapsed).then_some(group_action),
                sidebar_direction,
            ),
            sidebar_group_content(sidebar_menu(sidebar_items)),
        ]);
        let sidebar_header = sidebar_header(
            text(if sidebar_collapsed { "D" } else { "Ducktape" }).size(theme.typography.lg),
        );
        let sidebar_footer = sidebar_footer(
            text(if sidebar_collapsed {
                "BH"
            } else {
                "byeongsu-hong"
            })
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
        );
        let sidebar_panel = sidebar(
            SidebarId::new("showcase"),
            self.sidebar_state,
            sidebar_group,
            Message::Sidebar(SidebarAction::Toggle(sidebar_viewport)),
            &theme,
        )
        .header(sidebar_header)
        .footer(sidebar_footer)
        .side(sidebar_side)
        .variant(sidebar_variant)
        .collapsible(sidebar_collapsible)
        .viewport(sidebar_viewport)
        .rail(true)
        .metrics(SIDEBAR_METRICS);
        let sidebar_main = surface(
            column![
                text("Main content").size(theme.typography.lg),
                text(
                    "The desktop panel participates in layout; mobile uses a dismissible backdrop."
                )
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            ]
            .spacing(theme.spacing.sm),
            SurfaceVariant::Muted,
            &theme,
        )
        .padding(theme.spacing.xl)
        .width(Length::Fill)
        .height(Length::Fill);
        let sidebar_preview = container(sidebar_layout(
            sidebar_main,
            sidebar_panel,
            self.sidebar_state,
            sidebar_viewport,
            sidebar_side,
            Message::Sidebar(SidebarAction::CloseMobile),
            &theme,
        ))
        .width(Length::Fill)
        .height(360);
        let sidebar_controls = row![
            button("Toggle sidebar", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(Message::Sidebar(SidebarAction::Toggle(sidebar_viewport))),
            text(format!(
                "expanded={} mobile={} open={} collapsed={} side={sidebar_side:?} variant={sidebar_variant:?} viewport={sidebar_viewport:?}",
                self.sidebar_state.expanded,
                self.sidebar_state.mobile_open,
                self.sidebar_state.is_open(sidebar_viewport),
                sidebar_collapsed,
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
        ]
        .spacing(theme.spacing.sm)
        .align_y(iced::Alignment::Center);

        let chart_config = ChartConfig::new([
            SeriesConfig::new("desktop", "Desktop", ChartColor::Primary),
            SeriesConfig::new("mobile", "Mobile", ChartColor::Ring),
            SeriesConfig::new(
                "other",
                "Other",
                ChartColor::LightDark {
                    light: iced::Color::from_rgb8(63, 125, 84),
                    dark: iced::Color::from_rgb8(108, 192, 111),
                },
            ),
        ]);
        let chart_data = ChartData::new([
            ChartDatum::new(1.0, "Jan")
                .with_value("desktop", 186.0)
                .with_value("mobile", 80.0)
                .with_value("other", 42.0)
                .with_metadata("period", "January 2026")
                .with_series_name("display", "desktop", "Desktop visits"),
            ChartDatum::new(2.0, "Feb")
                .with_value("desktop", 305.0)
                .with_value("mobile", 200.0)
                .with_value("other", 88.0)
                .with_metadata("period", "February 2026")
                .with_series_name("display", "mobile", "Mobile visits"),
            ChartDatum::new(3.0, "Mar")
                .with_value("desktop", 237.0)
                .with_value("mobile", 120.0)
                .with_value("other", 74.0)
                .with_metadata("period", "March 2026"),
            ChartDatum::new(4.0, "Apr")
                .with_value("desktop", 273.0)
                .with_value("mobile", 190.0)
                .with_value("other", 96.0)
                .with_metadata("period", "April 2026"),
        ]);
        let chart_kind = match self.accent {
            0 => CartesianKind::Line { points: true },
            1 => CartesianKind::Area { points: true },
            _ => CartesianKind::Bar(if self.dark {
                BarLayout::Stacked
            } else {
                BarLayout::Grouped
            }),
        };
        let cartesian = cartesian_chart(&chart_config, &chart_data, &theme)
            .kind(chart_kind)
            .domain(DomainSpec {
                x: Some(AxisDomain::new(0.5, 4.5)),
                y: None,
            })
            .padding(ChartPadding {
                top: 16.0,
                right: 20.0,
                bottom: 36.0,
                left: 48.0,
            })
            .ticks(5)
            .grid(true)
            .hovered(self.chart_hover.clone())
            .on_hover(Message::ChartHover)
            .width(Length::FillPortion(2))
            .height(280);
        let pie_data = PieData::new([("desktop", 420.0), ("mobile", 280.0), ("other", 140.0)]);
        let donut = pie_chart(&chart_config, &pie_data, &theme)
            .donut(0.58)
            .hovered(self.pie_hover.clone())
            .on_hover(Message::PieHover)
            .width(Length::FillPortion(1))
            .height(280);
        let chart_visuals = row![cartesian, donut]
            .spacing(theme.spacing.lg)
            .width(Length::Fill);
        let tooltip_options = TooltipOptions {
            indicator: match self.accent {
                0 => TooltipIndicator::Dot,
                1 => TooltipIndicator::Line,
                _ => TooltipIndicator::Dashed,
            },
            label_key: Some("period".into()),
            name_key: Some("display".into()),
        };
        let cartesian_tooltip = self.chart_hover.as_ref().and_then(|hit| {
            tooltip_model(&chart_config, &chart_data, hit, &tooltip_options, &theme)
        });
        let pie_tooltip = self.pie_hover.as_ref().and_then(|hit| {
            pie_tooltip_model(
                &chart_config,
                &pie_data,
                hit,
                tooltip_options.indicator,
                &theme,
            )
        });
        let active_tooltip: Element<'_, Message> = if let Some(model) = cartesian_tooltip.as_ref() {
            tooltip_content(model, &theme).into()
        } else if let Some(model) = pie_tooltip.as_ref() {
            tooltip_content(model, &theme).into()
        } else {
            text("Hover a chart mark to inspect its aligned tooltip.")
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground)
                .into()
        };
        let legend = legend_content(&chart_config, &theme);
        let legend_keys = legend_entries(&chart_config, &theme)
            .into_iter()
            .map(|entry| entry.key)
            .collect::<Vec<_>>();
        let chart_companion = companion_model("Traffic by month", &chart_config, &chart_data)
            .expect("showcase chart config is valid");
        let chart_companion = companion_content(&chart_companion, &theme);

        let legacy_toast = toast(text("Legacy Toast"), &theme)
            .description(text(
                "Caller-owned copy and controls stay precisely aligned.",
            ))
            .action(
                button("Undo", &theme)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Small)
                    .on_press(Message::ShowToast(ToastVariant::Info)),
            )
            .dismiss(
                button("Dismiss", &theme)
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Small)
                    .on_press(Message::ClearToasts),
            )
            .variant(ToastVariant::Info)
            .width(Length::Fill);
        let toast_controls = row![
            button("Default", &theme)
                .size(ButtonSize::Small)
                .on_press(Message::ShowToast(ToastVariant::Default)),
            button("Success", &theme)
                .size(ButtonSize::Small)
                .on_press(Message::ShowToast(ToastVariant::Success)),
            button("Info", &theme)
                .size(ButtonSize::Small)
                .on_press(Message::ShowToast(ToastVariant::Info)),
            button("Warning", &theme)
                .size(ButtonSize::Small)
                .on_press(Message::ShowToast(ToastVariant::Warning)),
            button("Error", &theme)
                .size(ButtonSize::Small)
                .on_press(Message::ShowToast(ToastVariant::Destructive)),
            button("Loading", &theme)
                .size(ButtonSize::Small)
                .on_press(Message::ShowToast(ToastVariant::Loading)),
        ]
        .spacing(theme.spacing.xs);
        let toast_settings = row![
            button("Resolve", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(Message::ResolveLoadingToast),
            button("Clear", &theme)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Small)
                .on_press(Message::ClearToasts),
            button("Placement", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(Message::CycleToastPlacement),
            button("Stack", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(Message::ToggleToastExpanded),
            button("Motion", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(Message::ToggleReducedMotion),
            button("Swipe", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(Message::ToggleSwipeDirection),
        ]
        .spacing(theme.spacing.xs);
        let visible_toasts = self
            .sonner
            .visible()
            .map(|entry| {
                format!(
                    "{}:{} paused={} deadline={:?} remaining={:?} offset={:.0}",
                    entry.id().get(),
                    entry.data().title(),
                    entry.is_paused(),
                    entry.deadline(),
                    entry.remaining(),
                    entry.swipe_offset(),
                )
            })
            .collect::<Vec<_>>();
        let queued_toasts = self
            .sonner
            .queued()
            .map(|entry| entry.id().get())
            .collect::<Vec<_>>();
        let loading_visible = self
            .loading_toast
            .and_then(|id| self.sonner.get(id))
            .is_some_and(|entry| entry.is_visible());
        let toast_meta = format!(
            "count={} empty={} visible={visible_toasts:?} queued={queued_toasts:?} max={} placement={:?} duration={:?} offset={:.0} expanded={} reduced={} swipe={:?}/{:.0} loading-visible={loading_visible}",
            self.sonner.len(),
            self.sonner.is_empty(),
            self.sonner.max_visible(),
            self.sonner.placement(),
            self.sonner.default_duration(),
            self.sonner.offset(),
            self.sonner.is_expanded(),
            self.sonner.reduced_motion(),
            self.sonner.swipe_direction(),
            self.sonner.swipe_threshold(),
        );

        let invalid = self.email.is_empty();
        let form = column![
            card_header(
                "Create account",
                "Enter an email address to continue.",
                &theme
            ),
            field(
                "Email",
                input("name@example.com", &self.email, &theme).on_input(Message::EmailChanged),
                Some(FieldHint::Description(
                    "We'll only use this for account notices."
                )),
                &theme,
            ),
            field(
                "Required email",
                input_with_variant(
                    "Required field",
                    &self.email,
                    if invalid {
                        InputVariant::Invalid
                    } else {
                        InputVariant::Default
                    },
                    &theme,
                )
                .on_input(Message::EmailChanged),
                invalid.then_some(FieldHint::Error("Email is required.")),
                &theme,
            ),
            button("Submit", &theme)
                .width(Length::Fill)
                .on_press(Message::Clicked),
        ]
        .spacing(theme.spacing.md);

        let content = column![
            row![
                column![
                    text("ducktape-ui").size(32),
                    text("Source-owned components for iced")
                        .size(theme.typography.base)
                        .color(theme.palette.muted_foreground),
                ]
                .spacing(theme.spacing.xs),
                ui::separator::vertical(&theme),
                button(if self.dark { "Light" } else { "Dark" }, &theme)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Small)
                    .on_press(Message::ToggleTheme),
                button("Accent", &theme)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Small)
                    .on_press(Message::CycleAccent),
            ]
            .spacing(theme.spacing.xl)
            .align_y(iced::Alignment::Center),
            ui::separator::horizontal(&theme),
            text("Buttons").size(theme.typography.xl),
            button_examples,
            text(format!(
                "Pressed {0} time{1}",
                self.clicks,
                if self.clicks == 1 { "" } else { "s" }
            ))
            .color(theme.palette.muted_foreground),
            text("Badges").size(theme.typography.xl),
            badges,
            statuses,
            text("Surfaces").size(theme.typography.xl),
            surfaces,
            text("Segmented control").size(theme.typography.xl),
            row![
                sections,
                text(match self.section {
                    Section::General => "General selected",
                    Section::Advanced => "Advanced selected",
                })
                .color(theme.palette.muted_foreground),
            ]
            .spacing(theme.spacing.sm)
            .align_y(iced::Alignment::Center),
            text("Feedback").size(theme.typography.xl),
            alerts,
            progress_examples,
            text("Checkbox + textarea").size(theme.typography.xl),
            checkbox("I reviewed the generated source", self.accepted, &theme)
                .on_toggle(Message::AcceptedChanged),
            notes,
            text("Empty state").size(theme.typography.xl),
            empty_state(
                Some(badge("Ready", BadgeVariant::Success, &theme).into()),
                "No saved presets",
                "Create one when this configuration is ready to reuse.",
                &theme,
            ),
            text("Content primitives").size(theme.typography.xl),
            breadcrumb_example,
            row![
                avatar_fallback("S", AvatarSize::Small, &theme),
                avatar_fallback("D", AvatarSize::Default, &theme),
                avatar_fallback("L", AvatarSize::Large, &theme),
                label("Keyboard shortcut", &theme),
                kbd("Ctrl", &theme),
                text("+").color(theme.palette.muted_foreground),
                kbd("K", &theme),
            ]
            .spacing(theme.spacing.sm)
            .align_y(iced::Alignment::Center),
            item_example,
            pagination_example,
            text("Aspect ratio + skeleton").size(theme.typography.xl),
            ratio_example,
            row![
                ui::skeleton::skeleton(&theme).width(160).height(16),
                ui::skeleton::skeleton(&theme).width(48).height(48),
            ]
            .spacing(theme.spacing.sm),
            text("Typography").size(theme.typography.xl),
            type_examples,
            text("Grouped controls").size(theme.typography.xl),
            row![horizontal_group, vertical_group]
                .spacing(theme.spacing.md)
                .align_y(iced::Alignment::Start),
            grouped_input,
            invalid_grouped_input,
            text("Messaging").size(theme.typography.xl),
            attachment_example,
            transcript,
            text("Table + scrolling + spinner").size(theme.typography.xl),
            table_example,
            scrolling_example,
            spinner_examples,
            text("Direction").size(theme.typography.xl),
            direction_examples,
            text("Selection").size(theme.typography.xl),
            row![
                combobox_example.width(Length::FillPortion(1)),
                native_select_example.width(Length::FillPortion(1)),
            ]
            .spacing(theme.spacing.sm),
            text("Command").size(theme.typography.xl),
            button("Focus command input", &theme)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(Message::FocusCommandInput),
            command_example,
            text(command_meta)
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            text("Disclosure").size(theme.typography.xl),
            accordion_example,
            text(format!(
                "Header targets: {accordion_targets:?}; multiple open: {}; forced states: {forced_collapsible_states:?}",
                multiple_accordion.is_open(&"install")
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            collapsible_example,
            text("Data table state recipe").size(theme.typography.xl),
            text(format!(
                "query={:?}, sort={:?}, page={}, pages={data_page_count}, range={data_range:?}",
                data_table_state.query, data_table_state.sort, data_table_state.page,
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            text("Focus control").size(theme.typography.xl),
            focusable,
            text("Tabs").size(theme.typography.xl),
            automatic_tabs,
            text(format!(
                "selected={:?}; {disabled_tab_info}; cleared={:?}",
                self.tabs_automatic.selected(),
                cleared_tabs.selected(),
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            manual_tabs,
            text("Input OTP").size(theme.typography.xl),
            row![otp_example, disabled_otp]
                .spacing(theme.spacing.md)
                .align_y(iced::Alignment::Center)
                .wrap(),
            text(format!(
                "controlled={:?}, complete={otp_complete}, custom-filter={custom_otp:?}",
                self.otp,
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            text("Radio group").size(theme.typography.xl),
            density_radio,
            disabled_radio,
            text("Slider").size(theme.typography.xl),
            row![
                column![
                    temperature_slider,
                    text(format!("Range: {:?}", self.slider_values))
                        .size(theme.typography.sm)
                        .color(theme.palette.muted_foreground),
                    button("Focus first thumb", &theme)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Small)
                        .on_press(Message::FocusFirstSliderThumb),
                ]
                .spacing(theme.spacing.sm)
                .width(Length::Fill),
                vertical_slider,
            ]
            .spacing(theme.spacing.md)
            .align_y(iced::Alignment::Center),
            text("Toggle + Toggle Group").size(theme.typography.xl),
            standalone_toggles,
            row![format_group, vertical_toggle_group]
                .spacing(theme.spacing.md)
                .align_y(iced::Alignment::Start),
            text(format!(
                "multiple={:?}; single-next={:?}; end={toggle_target:?}; outline={:?}",
                self.toggle_group_state,
                single_toggle_state.toggled("bottom"),
                toggle_style.border,
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            text("Switch").size(theme.typography.xl),
            switches,
            text("Resizable").size(theme.typography.xl),
            workspace_resizable,
            row![
                button("Focus first handle", &theme)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Small)
                    .on_press(Message::FocusFirstResizeHandle),
                text(format!(
                    "sizes={:?}, minimums={resize_minimums:?}",
                    self.resizable_sizes,
                ))
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            ]
            .spacing(theme.spacing.sm)
            .align_y(iced::Alignment::Center),
            vertical_resizable,
            text("Dialog + Alert Dialog").size(theme.typography.xl),
            modal_triggers,
            dialog_alignment_preview,
            text("Popover + Tooltip + Hover Card").size(theme.typography.xl),
            floating_examples,
            text("Menu family").size(theme.typography.xl),
            menubar_example,
            row![dropdown_example, context_example, select_example]
                .spacing(theme.spacing.md)
                .align_y(iced::Alignment::Start)
                .wrap(),
            text(menu_meta)
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            text("Navigation Menu").size(theme.typography.xl),
            navigation_example,
            text(navigation_meta)
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            text("Date Picker").size(theme.typography.xl),
            date_picker_example,
            text(date_picker_meta)
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            text("Calendar").size(theme.typography.xl),
            calendar_example,
            text(calendar_meta)
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            text("Carousel").size(theme.typography.xl),
            carousel_example,
            text(format!(
                "slide={}/{}, boundary={:?}, empty={}, bounded-last={}, keys={carousel_key_examples:?}",
                self.carousel_state.index() + 1,
                self.carousel_state.slide_count(),
                self.carousel_state.boundary(),
                self.carousel_state.is_empty(),
                bounded_edge.index(),
            ))
            .width(Length::Fill)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            text("Sidebar").size(theme.typography.xl),
            sidebar_controls,
            sidebar_preview,
            text("Chart").size(theme.typography.xl),
            legend,
            chart_visuals,
            active_tooltip,
            chart_companion,
            text(format!("kind={chart_kind:?}; legend-keys={legend_keys:?}"))
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            text("Toast + Sonner").size(theme.typography.xl),
            legacy_toast,
            toast_controls,
            toast_settings,
            text(toast_meta)
                .width(Length::Fill)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
            text("Card + field").size(theme.typography.xl),
            card(form, &theme).width(Length::Fill),
        ]
        .max_width(900)
        .spacing(theme.spacing.lg)
        .padding(theme.spacing.xxl);

        let page_background = theme.palette.background;
        let page: Element<'_, Message> = container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(page_background)),
                ..Default::default()
            })
            .into();
        let page: Element<'_, Message> =
            stack![page, sonner(&self.sonner, Message::Sonner, &theme)]
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        let dialog_actions = row![
            toggle(
                self.dialog_focus.first().clone(),
                text("Cancel"),
                false,
                Message::CloseDialog,
                &theme,
            )
            .variant(ToggleVariant::Outline),
            toggle(
                self.dialog_focus.order()[1].clone(),
                text("Save changes"),
                false,
                Message::SaveDialog,
                &theme,
            )
            .variant(ToggleVariant::Default),
        ]
        .spacing(theme.spacing.sm);
        let page = dialog(
            page,
            self.dialog_open,
            &self.dialog_focus,
            "Edit workspace",
            "Dialog copy, controls, and focus stay inside the modal surface.",
            text("Press Tab or Shift+Tab to verify the wrapping focus order."),
            dialog_actions,
            Message::Dialog,
            &theme,
        );
        let action_variant = if self.accent == 0 {
            AlertDialogActionVariant::Destructive
        } else {
            AlertDialogActionVariant::Default
        };

        let page = alert_dialog(
            page,
            self.alert_dialog_open,
            &self.alert_dialog_focus,
            "Delete this workspace?",
            "This demonstrates the stricter alert-dialog dismissal and focus contract.",
            "Cancel",
            "Continue",
            action_variant,
            Message::AlertDialog,
            &theme,
        );
        let panel_direction = if self.dark {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let sheet_side = match self.accent {
            0 => SheetSide::Right,
            1 => SheetSide::Left,
            _ if self.dark => SheetSide::Top,
            _ => SheetSide::Bottom,
        };
        let sheet_actions = row![
            toggle(
                self.sheet_focus.order()[1].clone(),
                text("Save"),
                false,
                Message::CloseSheet,
                &theme,
            )
            .variant(ToggleVariant::Default),
        ];
        let sheet_panel = sheet_panel(
            sheet_body(text(
                "Sheet body content fills the remaining edge-panel space.",
            )),
            &theme,
        )
        .header(sheet_header(
            "Workspace settings",
            "A viewport-capped panel with explicit text and action alignment.",
            panel_direction,
            if self.dark {
                SheetTextAlignment::Center
            } else {
                SheetTextAlignment::Start
            },
            &theme,
        ))
        .footer(sheet_footer(
            sheet_actions,
            panel_direction,
            SheetActionAlignment::End,
        ))
        .close(
            toggle(
                self.sheet_focus.first().clone(),
                text("×"),
                false,
                Message::CloseSheet,
                &theme,
            )
            .variant(ToggleVariant::Default),
        )
        .direction(panel_direction)
        .padding(theme.spacing.xl)
        .spacing(theme.spacing.lg);
        let page = sheet(
            page,
            self.sheet_open,
            sheet_panel,
            &self.sheet_focus,
            Message::Sheet,
            &theme,
        )
        .side(sheet_side)
        .mode(if self.dark {
            SheetMode::NonModal
        } else {
            SheetMode::Modal
        })
        .dismiss_rules(DismissRules::DIALOG)
        .size(384.0)
        .max_size(512.0)
        .max_viewport_fraction(0.9)
        .into_element();
        let drawer_side = match sheet_side {
            SheetSide::Top => SheetSide::Bottom,
            SheetSide::Right => SheetSide::Left,
            SheetSide::Bottom => SheetSide::Top,
            SheetSide::Left => SheetSide::Right,
        };
        let drawer_actions = row![
            toggle(
                self.drawer_focus.order()[1].clone(),
                text("Continue"),
                false,
                Message::CloseDrawer,
                &theme,
            )
            .variant(ToggleVariant::Default),
        ];
        let drawer_panel = drawer_panel(
            drawer_body(text(
                "Drag the handle toward its edge to dismiss, or let it snap back.",
            )),
            &theme,
        )
        .header(drawer_header(
            "Move this drawer",
            "Pointer and touch offsets stay controlled by the application.",
            panel_direction,
            DrawerTextAlignment::Start,
            &theme,
        ))
        .footer(drawer_footer(
            drawer_actions,
            panel_direction,
            DrawerActionAlignment::End,
        ))
        .close(
            toggle(
                self.drawer_focus.first().clone(),
                text("×"),
                false,
                Message::CloseDrawer,
                &theme,
            )
            .variant(ToggleVariant::Default),
        )
        .direction(panel_direction);

        drawer(
            page,
            &self.drawer_state,
            drawer_panel,
            &self.drawer_focus,
            Message::Drawer,
            &theme,
        )
        .side(drawer_side)
        .size(320.0)
        .max_size(560.0)
        .dismiss_rules(DismissRules::DIALOG)
        .draggable(true)
        .distance_threshold(0.5)
        .velocity_threshold(700.0)
        .reduced_motion(self.sonner.reduced_motion())
        .into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showcase_seeds_and_resizes_one_stable_stream_row() {
        let (mut showcase, _) = Showcase::boot();
        assert_eq!(
            showcase.message_scroller.items().len(),
            showcase.transcript.len()
        );

        let original_len = showcase.transcript.len();
        let _ = showcase.update(Message::StreamTranscript);
        let stream_id = showcase.transcript.last().unwrap().id.clone();
        let original_copy_len = showcase.transcript.last().unwrap().copy.len();
        assert!(showcase.message_scroller.items()[original_len].is_scroll_anchor());

        let _ = showcase.update(Message::StreamTranscript);
        assert_eq!(showcase.transcript.len(), original_len + 2);
        assert_eq!(showcase.transcript.last().unwrap().id, stream_id);
        assert!(showcase.transcript.last().unwrap().copy.len() > original_copy_len);
    }
}
