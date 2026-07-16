mod ui;

use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length, Theme as IcedTheme};
use ui::alert::{AlertVariant, alert};
use ui::badge::{BadgeSize, BadgeVariant, badge};
use ui::button::{ButtonSize, ButtonVariant, button};
use ui::card::{card, card_header};
use ui::checkbox::checkbox;
use ui::empty_state::empty_state;
use ui::field::{FieldHint, field};
use ui::input::{InputVariant, input, input_with_variant};
use ui::progress::{ProgressVariant, progress};
use ui::segmented_control::segmented_control;
use ui::surface::{SurfaceVariant, surface};
use ui::textarea::{TextareaVariant, textarea};
use ui::theme::{ACCENTS, DARK, LIGHT, Theme};

#[derive(Default)]
struct Showcase {
    dark: bool,
    email: String,
    clicks: u32,
    accent: usize,
    section: Section,
    accepted: bool,
    notes: iced::widget::text_editor::Content,
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
