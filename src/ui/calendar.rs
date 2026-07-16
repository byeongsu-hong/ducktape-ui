use super::button::{Button, ButtonSize, ButtonVariant, button};
use super::theme::Theme;
use iced::widget::{Column, Grid, Row, Space, container, text};
use iced::{Alignment, Element, Length};
use std::fmt;

pub const MIN_YEAR: i32 = 1;
pub const MAX_YEAR: i32 = 9999;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateError {
    Year(i32),
    Month(u8),
    Day { year: i32, month: u8, day: u8 },
}

impl fmt::Display for DateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Year(year) => {
                write!(formatter, "year {year} is outside {MIN_YEAR}..={MAX_YEAR}")
            }
            Self::Month(month) => write!(formatter, "month {month} is outside 1..=12"),
            Self::Day { year, month, day } => {
                write!(
                    formatter,
                    "day {day} does not exist in {year:04}-{month:02}"
                )
            }
        }
    }
}

impl std::error::Error for DateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month {
    year: i32,
    number: u8,
}

impl Month {
    pub const fn new(year: i32, number: u8) -> Result<Self, DateError> {
        if year < MIN_YEAR || year > MAX_YEAR {
            return Err(DateError::Year(year));
        }
        if number < 1 || number > 12 {
            return Err(DateError::Month(number));
        }
        Ok(Self { year, number })
    }

    pub const fn year(self) -> i32 {
        self.year
    }

    pub const fn number(self) -> u8 {
        self.number
    }

    pub const fn days(self) -> u8 {
        match self.number {
            2 if self.is_leap_year() => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    pub const fn is_leap_year(self) -> bool {
        is_leap_year(self.year)
    }

    pub const fn previous(self) -> Option<Self> {
        if self.number > 1 {
            Some(Self {
                year: self.year,
                number: self.number - 1,
            })
        } else if self.year > MIN_YEAR {
            Some(Self {
                year: self.year - 1,
                number: 12,
            })
        } else {
            None
        }
    }

    pub const fn next(self) -> Option<Self> {
        if self.number < 12 {
            Some(Self {
                year: self.year,
                number: self.number + 1,
            })
        } else if self.year < MAX_YEAR {
            Some(Self {
                year: self.year + 1,
                number: 1,
            })
        } else {
            None
        }
    }

    pub const fn contains(self, date: Date) -> bool {
        let month = date.month();
        self.year == month.year && self.number == month.number
    }

    /// Dates displayed by a fixed six-week calendar. At the supported year
    /// boundaries, cells that would cross the boundary are `None`.
    pub fn visible_dates(self) -> [Option<Date>; 42] {
        let mut dates = [None; 42];
        let first = Date {
            year: self.year,
            month: self.number,
            day: 1,
        };
        let leading = usize::from(first.weekday().index_from_sunday());
        let current_days = usize::from(self.days());
        let previous = self.previous();
        let next = self.next();

        for (cell, date) in dates.iter_mut().enumerate() {
            *date = if cell < leading {
                previous.map(|month| Date {
                    year: month.year(),
                    month: month.number(),
                    day: month.days() - (leading - cell) as u8 + 1,
                })
            } else {
                let day = cell - leading + 1;
                if day <= current_days {
                    Some(Date {
                        year: self.year,
                        month: self.number,
                        day: day as u8,
                    })
                } else {
                    next.map(|month| Date {
                        year: month.year(),
                        month: month.number(),
                        day: (day - current_days) as u8,
                    })
                }
            };
        }

        dates
    }
}

impl fmt::Display for Month {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}",
            MONTH_NAMES[usize::from(self.number() - 1)],
            self.year()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    pub const fn new(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        if year < MIN_YEAR || year > MAX_YEAR {
            return Err(DateError::Year(year));
        }
        if month < 1 || month > 12 {
            return Err(DateError::Month(month));
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(DateError::Day { year, month, day });
        }
        Ok(Self { year, month, day })
    }

    pub const fn year(self) -> i32 {
        self.year
    }

    pub const fn month_number(self) -> u8 {
        self.month
    }

    pub const fn day(self) -> u8 {
        self.day
    }

    pub const fn month(self) -> Month {
        Month {
            year: self.year,
            number: self.month,
        }
    }

    pub const fn weekday(self) -> Weekday {
        let previous_year = self.year() - 1;
        let mut days =
            previous_year * 365 + previous_year / 4 - previous_year / 100 + previous_year / 400;
        let mut month = 1;
        while month < self.month_number() {
            days += days_in_month(self.year(), month) as i32;
            month += 1;
        }
        days += self.day() as i32 - 1;

        Weekday::from_sunday_index(((days + 1) % 7) as u8)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year(),
            self.month_number(),
            self.day()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    pub const fn index_from_sunday(self) -> u8 {
        self as u8
    }

    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Sunday => "Su",
            Self::Monday => "Mo",
            Self::Tuesday => "Tu",
            Self::Wednesday => "We",
            Self::Thursday => "Th",
            Self::Friday => "Fr",
            Self::Saturday => "Sa",
        }
    }

    const fn from_sunday_index(index: u8) -> Self {
        match index {
            0 => Self::Sunday,
            1 => Self::Monday,
            2 => Self::Tuesday,
            3 => Self::Wednesday,
            4 => Self::Thursday,
            5 => Self::Friday,
            6 => Self::Saturday,
            _ => unreachable!(),
        }
    }
}

pub const WEEKDAYS: [Weekday; 7] = [
    Weekday::Sunday,
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
];

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A controlled Sunday-first month calendar.
///
/// The caller owns the displayed month and selection. Previous, next, and date
/// buttons emit only the messages supplied here, including for visible dates in
/// adjacent months.
///
/// Controls are native iced buttons. Iced 0.14 does not expose stable focus IDs
/// for its button widget, so arrow-key roving focus must be composed with the
/// forthcoming `focus_control` primitive; this function does not claim keyboard
/// grid semantics in the meantime.
pub fn calendar<'a, Message>(
    month: Month,
    selected: Option<Date>,
    previous: Message,
    next: Message,
    on_select: impl Fn(Date) -> Message,
    theme: &Theme,
) -> Column<'a, Message>
where
    Message: Clone + 'a,
{
    const WIDTH: f32 = 280.0;

    let header = Row::new()
        .push(
            button("‹", theme)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Icon)
                .disabled(month.previous().is_none())
                .on_press(previous),
        )
        .push(
            container(
                text(month.to_string())
                    .size(theme.typography.sm)
                    .color(theme.palette.foreground),
            )
            .center_x(Length::Fill)
            .center_y(36),
        )
        .push(
            button("›", theme)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Icon)
                .disabled(month.next().is_none())
                .on_press(next),
        )
        .align_y(Alignment::Center)
        .width(WIDTH);

    let weekdays = WEEKDAYS.into_iter().fold(
        Grid::new().columns(7).width(WIDTH).height(28.0),
        |grid, weekday| {
            grid.push(
                container(
                    text(weekday.short_name())
                        .size(theme.typography.xs)
                        .color(theme.palette.muted_foreground),
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            )
        },
    );

    let days = month.visible_dates().into_iter().fold(
        Grid::new().columns(7).width(WIDTH).height(240.0),
        |grid, date| {
            grid.push(match date {
                Some(date) => container(day_button(
                    date,
                    month.contains(date),
                    selected == Some(date),
                    &on_select,
                    theme,
                ))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
                None => Element::from(Space::new().width(Length::Fill).height(Length::Fill)),
            })
        },
    );

    Column::new().push(header).push(weekdays).push(days)
}

fn day_button<'a, Message>(
    date: Date,
    in_month: bool,
    selected: bool,
    on_select: &impl Fn(Date) -> Message,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let color = if selected {
        theme.palette.primary_foreground
    } else if in_month {
        theme.palette.foreground
    } else {
        theme.palette.muted_foreground
    };
    let variant = if selected {
        ButtonVariant::Default
    } else {
        ButtonVariant::Ghost
    };

    Button::new(
        text(date.day().to_string())
            .size(theme.typography.sm)
            .color(color),
        theme,
    )
    .variant(variant)
    .size(ButtonSize::Icon)
    .on_press(on_select(date))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(year: i32, number: u8) -> Month {
        Month::new(year, number).unwrap()
    }

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn validation_covers_leap_years_and_centuries() {
        assert!(Date::new(2024, 2, 29).is_ok());
        assert!(Date::new(2023, 2, 29).is_err());
        assert!(Date::new(1900, 2, 29).is_err());
        assert!(Date::new(2000, 2, 29).is_ok());
        assert!(!month(1900, 2).is_leap_year());
        assert_eq!(month(1900, 2).days(), 28);
        assert!(month(2000, 2).is_leap_year());
        assert_eq!(month(2000, 2).days(), 29);
        assert!(Date::new(0, 1, 1).is_err());
        assert!(Month::new(2024, 13).is_err());
    }

    #[test]
    fn month_navigation_crosses_years_without_crossing_supported_bounds() {
        assert_eq!(month(2024, 1).previous(), Some(month(2023, 12)));
        assert_eq!(month(2024, 12).next(), Some(month(2025, 1)));
        assert_eq!(month(MIN_YEAR, 1).previous(), None);
        assert_eq!(month(MAX_YEAR, 12).next(), None);
    }

    #[test]
    fn weekdays_match_known_dates() {
        assert_eq!(date(1, 1, 1).weekday(), Weekday::Monday);
        assert_eq!(date(2000, 1, 1).weekday(), Weekday::Saturday);
        assert_eq!(date(2024, 2, 29).weekday(), Weekday::Thursday);
    }

    #[test]
    fn six_week_grid_includes_adjacent_months() {
        let dates = month(2024, 9).visible_dates();
        assert_eq!(dates.len(), 42);
        assert_eq!(dates[0], Some(date(2024, 9, 1)));
        assert_eq!(dates[29], Some(date(2024, 9, 30)));
        assert_eq!(dates[30], Some(date(2024, 10, 1)));
        assert_eq!(dates[41], Some(date(2024, 10, 12)));

        let leap = month(2024, 3).visible_dates();
        assert_eq!(leap[4], Some(date(2024, 2, 29)));
        assert_eq!(leap[5], Some(date(2024, 3, 1)));
    }

    #[test]
    fn supported_boundary_uses_empty_cells_instead_of_invalid_dates() {
        let dates = month(MIN_YEAR, 1).visible_dates();
        assert_eq!(dates[0], None);
        assert_eq!(dates[1], Some(date(1, 1, 1)));
        assert!(
            dates
                .into_iter()
                .flatten()
                .all(|date| date.year() >= MIN_YEAR)
        );

        let dates = month(MAX_YEAR, 12).visible_dates();
        assert_eq!(dates[33], Some(date(MAX_YEAR, 12, 31)));
        assert!(dates[34..].iter().all(Option::is_none));
    }
}
