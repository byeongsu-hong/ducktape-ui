mod ui;

use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length, Theme as IcedTheme};
use ui::badge::{BadgeVariant, badge};
use ui::button::{ButtonSize, ButtonVariant, button};
use ui::card::{card, card_header};
use ui::input::{InputVariant, input, input_with_variant};
use ui::theme::{ACCENTS, DARK, LIGHT, Theme};

#[derive(Default)]
struct Showcase {
    dark: bool,
    email: String,
    clicks: u32,
    accent: usize,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    EmailChanged(String),
    Clicked,
    CycleAccent,
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
            badge("Outline", BadgeVariant::Outline, &theme),
        ]
        .spacing(theme.spacing.sm);

        let invalid = self.email.is_empty();
        let form = column![
            card_header(
                "Create account",
                "Enter an email address to continue.",
                &theme
            ),
            input("name@example.com", &self.email, &theme).on_input(Message::EmailChanged),
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
            text("Card + input").size(theme.typography.xl),
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
