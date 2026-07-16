mod ui;

use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length, Theme as IcedTheme};
use ui::alert::{AlertVariant, alert};
use ui::aspect_ratio::aspect_ratio;
use ui::attachment::attachment;
use ui::avatar::{AvatarSize, avatar_fallback};
use ui::badge::{BadgeSize, BadgeVariant, badge};
use ui::breadcrumb::{BreadcrumbItem, breadcrumb, breadcrumb_separator};
use ui::bubble::{BubbleVariant, bubble};
use ui::button::{ButtonSize, ButtonVariant, button};
use ui::button_group::{ButtonGroupOrientation, button_group};
use ui::card::{card, card_header};
use ui::checkbox::checkbox;
use ui::direction::{Direction, directed_row};
use ui::empty_state::empty_state;
use ui::field::{FieldHint, field};
use ui::input::{InputVariant, input, input_with_variant};
use ui::input_group::{group_input, input_group};
use ui::item::item;
use ui::kbd::kbd;
use ui::label::label;
use ui::marker::{MarkerVariant, marker};
use ui::message::{MessageSide, message};
use ui::message_scroller::message_scroller;
use ui::pagination::{PaginationItem, pagination};
use ui::progress::{ProgressVariant, progress};
use ui::scroll_area::scroll_area;
use ui::segmented_control::segmented_control;
use ui::surface::{SurfaceVariant, surface};
use ui::textarea::{TextareaVariant, textarea};
use ui::theme::{ACCENTS, DARK, LIGHT, Theme};
use ui::typography::{TextRole, inline_code, typography};

#[derive(Default)]
struct Showcase {
    dark: bool,
    email: String,
    clicks: u32,
    accent: usize,
    section: Section,
    accepted: bool,
    notes: iced::widget::text_editor::Content,
    page: usize,
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
}

fn main() -> iced::Result {
    iced::application(Showcase::default, Showcase::update, Showcase::view)
        .title("ducktape-ui component showcase")
        .theme(Showcase::iced_theme)
        .run()
}

impl Showcase {
    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => self.dark = !self.dark,
            Message::EmailChanged(value) => self.email = value,
            Message::Clicked => self.clicks += 1,
            Message::CycleAccent => self.accent = (self.accent + 1) % ACCENTS.len(),
            Message::SectionSelected(section) => self.section = section,
            Message::AcceptedChanged(accepted) => self.accepted = accepted,
            Message::NotesChanged(action) => self.notes.perform(action),
            Message::PageSelected(page) => self.page = page,
        }
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
