mod ui;

use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length, Theme as IcedTheme};
use ui::accordion::{AccordionKey, AccordionState, accordion, accordion_item, header_target};
use ui::alert::{AlertVariant, alert};
use ui::aspect_ratio::aspect_ratio;
use ui::attachment::attachment;
use ui::avatar::{AvatarSize, avatar_fallback};
use ui::badge::{BadgeSize, BadgeVariant, badge};
use ui::breadcrumb::{BreadcrumbItem, breadcrumb, breadcrumb_separator};
use ui::bubble::{BubbleVariant, bubble};
use ui::button::{ButtonSize, ButtonVariant, button};
use ui::button_group::{ButtonGroupOrientation, button_group};
use ui::calendar::{Date, Month, calendar};
use ui::card::{card, card_header};
use ui::carousel::{
    CarouselBoundary, CarouselCommand, CarouselOrientation, CarouselState, carousel,
    keyboard_command,
};
use ui::checkbox::checkbox;
use ui::collapsible::{CollapsibleChange, collapsible, next_open};
use ui::combobox::combobox;
use ui::data_table::DataTableState;
use ui::direction::{Direction, directed_row};
use ui::empty_state::empty_state;
use ui::field::{FieldHint, field};
use ui::input::{InputVariant, input, input_with_variant};
use ui::input_group::{group_input, input_group};
use ui::input_otp::{OtpPattern, input_otp, is_complete, normalize};
use ui::item::item;
use ui::kbd::kbd;
use ui::label::label;
use ui::marker::{MarkerVariant, marker};
use ui::message::{MessageSide, message};
use ui::message_scroller::message_scroller;
use ui::native_select::native_select;
use ui::pagination::{PaginationItem, pagination};
use ui::progress::{ProgressVariant, progress};
use ui::radio_group::{RadioOption, RadioOrientation, focus_radio, radio_group, radio_option};
use ui::scroll_area::scroll_area;
use ui::segmented_control::segmented_control;
use ui::slider::{SliderOrientation, focus_slider_thumb, slider};
use ui::surface::{SurfaceVariant, surface};
use ui::switch::{SwitchSize, switch};
use ui::tabs::{TabsActivation, TabsEvent, TabsOrientation, TabsState, TabsVariant, tab, tabs};
use ui::textarea::{TextareaVariant, textarea};
use ui::theme::{ACCENTS, DARK, LIGHT, Theme};
use ui::toggle::{ToggleSize, ToggleVariant, toggle};
use ui::toggle_group::{ToggleGroupOrientation, ToggleGroupState, toggle_group, toggle_group_item};
use ui::typography::{TextRole, inline_code, typography};

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
    calendar_month: Month,
    calendar_selected: Option<Date>,
    carousel_state: CarouselState,
    tabs_automatic: TabsState<&'static str>,
    tabs_manual: TabsState<&'static str>,
    otp: String,
    radio_selected: &'static str,
    slider_values: Vec<f32>,
    standalone_toggle: bool,
    toggle_group_state: ToggleGroupState<&'static str>,
    switch_on: bool,
}

impl Default for Showcase {
    fn default() -> Self {
        let mut tabs_automatic = TabsState::new("overview");
        tabs_automatic.select("overview");

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
            calendar_month: Month::new(2026, 7).expect("showcase month is valid"),
            calendar_selected: Some(Date::new(2026, 7, 16).expect("showcase selection is valid")),
            carousel_state: CarouselState::new(0, 3, CarouselBoundary::Wrap),
            tabs_automatic,
            tabs_manual: TabsState::new("account"),
            otp: "123".into(),
            radio_selected: "comfortable",
            slider_values: vec![25.0, 75.0],
            standalone_toggle: false,
            toggle_group_state: ToggleGroupState::Multiple(vec!["bold"]),
            switch_on: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Section {
    #[default]
    General,
    Advanced,
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
    AccordionToggle(&'static str),
    Collapsible(CollapsibleChange),
    CalendarPrevious,
    CalendarNext,
    CalendarSelected(Date),
    Carousel(CarouselCommand),
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
}

fn main() -> iced::Result {
    iced::application(Showcase::default, Showcase::update, Showcase::view)
        .title("ducktape-ui component showcase")
        .subscription(Showcase::subscription)
        .theme(Showcase::iced_theme)
        .run()
}

impl Showcase {
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
            Message::AccordionToggle(id) => {
                self.accordion_state = self.accordion_state.toggled(id);
            }
            Message::Collapsible(change) => {
                self.collapsible_open = next_open(self.collapsible_open, change);
            }
            Message::CalendarPrevious => {
                if let Some(month) = self.calendar_month.previous() {
                    self.calendar_month = month;
                }
            }
            Message::CalendarNext => {
                if let Some(month) = self.calendar_month.next() {
                    self.calendar_month = month;
                }
            }
            Message::CalendarSelected(date) => {
                self.calendar_selected = Some(date);
                self.calendar_month = date.month();
            }
            Message::Carousel(command) => {
                self.carousel_state = self.carousel_state.reduce(command);
            }
            Message::FocusTraversal { backwards } => {
                return if backwards {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
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
        }

        iced::Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen_with(|event, status, _window| {
            if status != iced::event::Status::Ignored {
                return None;
            }

            match event {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
                    modifiers,
                    ..
                }) => Some(Message::FocusTraversal {
                    backwards: modifiers.shift(),
                }),
                _ => None,
            }
        })
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
        let incoming = message(
            MessageSide::Incoming,
            Some(avatar_fallback("D", AvatarSize::Small, &theme).into()),
            Some(text("Ducktape").size(theme.typography.sm).into()),
            bubble(
                text("The source-owned components are ready."),
                BubbleVariant::Incoming,
                &theme,
            ),
            None,
            &theme,
        );
        let outgoing = message(
            MessageSide::Outgoing,
            None,
            Some(text("You").size(theme.typography.sm).into()),
            bubble(
                text("Ship the next batch."),
                BubbleVariant::Outgoing,
                &theme,
            ),
            Some(
                button("Copy", &theme)
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Small)
                    .on_press(Message::Clicked)
                    .into(),
            ),
            &theme,
        );
        let transcript = column![
            marker(None, "Today", MarkerVariant::Separator, &theme),
            incoming,
            marker(
                Some(badge("New", BadgeVariant::Secondary, &theme).into()),
                "Unread",
                MarkerVariant::Border,
                &theme,
            ),
            outgoing,
            marker(None, "End of transcript", MarkerVariant::Default, &theme),
        ]
        .spacing(theme.spacing.md);
        let transcript = message_scroller(
            transcript,
            iced::widget::Id::new("showcase-message-scroller"),
            &theme,
        )
        .height(260);

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
                ("Dialog", "Planned"),
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
        let native_select_example = native_select(
            self.select_options.as_slice(),
            self.native_selected.as_ref(),
            Message::NativeSelected,
            &theme,
        )
        .placeholder("Choose a theme…");

        let accordion_example = accordion(
            [
                accordion_item(
                    "install",
                    button("How is it installed?", &theme)
                        .variant(ButtonVariant::Ghost)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Left)
                        .on_press(Message::AccordionToggle("install")),
                    text("The CLI copies editable Rust source into src/ui."),
                ),
                accordion_item(
                    "state",
                    button("Who owns state?", &theme)
                        .variant(ButtonVariant::Ghost)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Left)
                        .on_press(Message::AccordionToggle("state")),
                    text("The application owns controlled state and messages."),
                ),
            ],
            &self.accordion_state,
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

        let calendar_example = calendar(
            self.calendar_month,
            self.calendar_selected,
            Message::CalendarPrevious,
            Message::CalendarNext,
            Message::CalendarSelected,
            &theme,
        );
        let selected_date = self
            .calendar_selected
            .expect("the showcase always owns a selected date");
        let calendar_meta = format!(
            "month={:04}-{:02} ({} days), selected={:04}-{:02}-{:02} {} in-month={} selected-month={}",
            self.calendar_month.year(),
            self.calendar_month.number(),
            self.calendar_month.days(),
            selected_date.year(),
            selected_date.month_number(),
            selected_date.day(),
            selected_date.weekday().short_name(),
            self.calendar_month.contains(selected_date),
            selected_date.month(),
        );

        let slides: Vec<Element<'_, Message>> =
            ["Source-owned", "Keyboard reducer", "No forced motion"]
                .into_iter()
                .map(|copy| {
                    surface(text(copy), SurfaceVariant::Muted, &theme)
                        .padding(theme.spacing.xl)
                        .width(Length::Fill)
                        .into()
                })
                .collect();
        let previous_slide = button("Previous", &theme)
            .variant(ButtonVariant::Outline)
            .disabled(!self.carousel_state.can_previous())
            .on_press(Message::Carousel(CarouselCommand::Previous));
        let next_slide = button("Next", &theme)
            .variant(ButtonVariant::Outline)
            .disabled(!self.carousel_state.can_next())
            .on_press(Message::Carousel(CarouselCommand::Next));
        let carousel_example = carousel(
            self.carousel_state,
            slides,
            previous_slide,
            next_slide,
            CarouselOrientation::Horizontal,
        );
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
            text("Disclosure").size(theme.typography.xl),
            accordion_example,
            text(format!(
                "Header targets: {accordion_targets:?}; multiple open: {}; forced states: {forced_collapsible_states:?}",
                multiple_accordion.is_open(&"install")
            ))
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            collapsible_example,
            text("Data table state recipe").size(theme.typography.xl),
            text(format!(
                "query={:?}, sort={:?}, page={}, pages={data_page_count}, range={data_range:?}",
                data_table_state.query, data_table_state.sort, data_table_state.page,
            ))
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
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            manual_tabs,
            text("Input OTP").size(theme.typography.xl),
            row![otp_example, disabled_otp]
                .spacing(theme.spacing.md)
                .align_y(iced::Alignment::Center),
            text(format!(
                "controlled={:?}, complete={otp_complete}, custom-filter={custom_otp:?}",
                self.otp,
            ))
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
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            text("Switch").size(theme.typography.xl),
            switches,
            text("Calendar").size(theme.typography.xl),
            calendar_example,
            text(calendar_meta)
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
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
            text("Card + field").size(theme.typography.xl),
            card(form, &theme).width(Length::Fill),
        ]
        .max_width(900)
        .spacing(theme.spacing.lg)
        .padding(theme.spacing.xxl);

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .into()
    }
}
