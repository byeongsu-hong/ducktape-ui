//! Controlled calendar selection and keyboard-complete day-grid navigation.
//!
//! Day cells expose stable iced focus IDs and support arrows, Home/End, and
//! PageUp/PageDown. Iced does not expose ARIA grid/date semantics; this module
//! implements the interaction contract without claiming roles it cannot emit.
//! Optional month/year pick lists inherit iced 0.14's pointer/touch limitation.

use std::fmt;
use std::rc::Rc;

use super::button::{Button, ButtonSize, ButtonVariant, button};
use super::direction::Direction;
use super::focus_control::{FocusControl, Status, Style as FocusStyle};
use super::native_select::native_select;
use super::theme::{Theme, alpha, mix};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::widget::text::LineHeight;
use iced::widget::{Column, Container, Row, Space, container, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Pixels, Shadow, Task, widget};

pub const MIN_YEAR: i32 = 1;
pub const MAX_YEAR: i32 = 9999;
pub const DAY_CELL_SIZE: f32 = 36.0;
pub const WEEKDAY_COUNT: usize = 7;
pub const CALENDAR_WIDTH: f32 = DAY_CELL_SIZE * WEEKDAY_COUNT as f32;

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
        days_in_month(self.year, self.number)
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
        self.year == date.year && self.number == date.month
    }

    pub const fn first(self) -> Date {
        Date {
            year: self.year,
            month: self.number,
            day: 1,
        }
    }

    pub const fn last(self) -> Date {
        Date {
            year: self.year,
            month: self.number,
            day: self.days(),
        }
    }

    /// Dates displayed by a fixed six-week, Sunday-first calendar.
    pub fn visible_dates(self) -> [Option<Date>; 42] {
        let mut dates = [None; 42];
        let leading = usize::from(self.first().weekday().index_from_sunday());
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
        Weekday::from_sunday_index((self.ordinal() % 7 + 1) as u8 % 7)
    }

    /// Zero-based day number since 0001-01-01.
    pub const fn ordinal(self) -> i32 {
        let previous_year = self.year - 1;
        let mut days =
            previous_year * 365 + previous_year / 4 - previous_year / 100 + previous_year / 400;
        let mut month = 1;
        while month < self.month {
            days += days_in_month(self.year, month) as i32;
            month += 1;
        }
        days + self.day as i32 - 1
    }

    pub fn checked_add_days(self, days: i32) -> Option<Self> {
        date_from_ordinal(self.ordinal().checked_add(days)?)
    }

    pub fn checked_add_months(self, months: i32) -> Option<Self> {
        let absolute = (self.year - 1)
            .checked_mul(12)?
            .checked_add(i32::from(self.month) - 1)?
            .checked_add(months)?;
        if !(0..MAX_YEAR * 12).contains(&absolute) {
            return None;
        }
        let year = absolute / 12 + 1;
        let month = (absolute % 12 + 1) as u8;
        Self::new(year, month, self.day.min(days_in_month(year, month))).ok()
    }

    pub fn checked_add_years(self, years: i32) -> Option<Self> {
        let year = self.year.checked_add(years)?;
        Self::new(
            year,
            self.month,
            self.day.min(days_in_month(year, self.month)),
        )
        .ok()
    }

    pub fn iso_week(self) -> u8 {
        let monday_index = (self.weekday().index_from_sunday() + 6) % 7;
        let thursday = self
            .checked_add_days(3 - i32::from(monday_index))
            .unwrap_or(self);
        ((thursday.ordinal() - Date::new(thursday.year(), 1, 1).unwrap().ordinal()) / 7 + 1) as u8
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

pub const MONTH_NAMES: [&str; 12] = [
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

fn date_from_ordinal(ordinal: i32) -> Option<Date> {
    if !(0..=Date::new(MAX_YEAR, 12, 31).ok()?.ordinal()).contains(&ordinal) {
        return None;
    }

    let mut low = MIN_YEAR;
    let mut high = MAX_YEAR;
    while low < high {
        let middle = low + (high - low + 1) / 2;
        let start = Date::new(middle, 1, 1).ok()?.ordinal();
        if start <= ordinal {
            low = middle;
        } else {
            high = middle - 1;
        }
    }

    let year = low;
    let mut remaining = ordinal - Date::new(year, 1, 1).ok()?.ordinal();
    let mut month = 1;
    while remaining >= i32::from(days_in_month(year, month)) {
        remaining -= i32::from(days_in_month(year, month));
        month += 1;
    }
    Date::new(year, month, (remaining + 1) as u8).ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub start: Date,
    pub end: Option<Date>,
}

impl DateRange {
    pub const fn open(start: Date) -> Self {
        Self { start, end: None }
    }

    pub fn inclusive(first: Date, second: Date) -> Self {
        let (start, end) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };
        Self {
            start,
            end: Some(end),
        }
    }

    pub fn contains(self, date: Date) -> bool {
        self.end
            .is_some_and(|end| (self.start..=end).contains(&date))
    }

    pub const fn is_complete(self) -> bool {
        self.end.is_some()
    }
}

/// The three controlled selection models supported by shadcn's calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarSelection {
    Single(Option<Date>),
    Multiple(Vec<Date>),
    Range(Option<DateRange>),
}

impl CalendarSelection {
    pub fn is_selected(&self, date: Date) -> bool {
        match self {
            Self::Single(selected) => *selected == Some(date),
            Self::Multiple(selected) => selected.contains(&date),
            Self::Range(Some(range)) => range.contains(date) || range.start == date,
            Self::Range(None) => false,
        }
    }

    pub fn selected(self, date: Date) -> Self {
        match self {
            Self::Single(_) => Self::Single(Some(date)),
            Self::Multiple(mut selected) => {
                if selected.contains(&date) {
                    selected.retain(|selected| *selected != date);
                } else {
                    selected.push(date);
                    selected.sort_unstable();
                }
                Self::Multiple(selected)
            }
            Self::Range(None) | Self::Range(Some(DateRange { end: Some(_), .. })) => {
                Self::Range(Some(DateRange::open(date)))
            }
            Self::Range(Some(DateRange { start, end: None })) => {
                Self::Range(Some(DateRange::inclusive(start, date)))
            }
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::Single(selected) => *selected = None,
            Self::Multiple(selected) => selected.clear(),
            Self::Range(selected) => *selected = None,
        }
    }

    pub fn range(&self) -> Option<DateRange> {
        match self {
            Self::Range(range) => *range,
            Self::Single(_) | Self::Multiple(_) => None,
        }
    }
}

#[derive(Clone, Default)]
pub struct CalendarConstraints<'a> {
    min: Option<Date>,
    max: Option<Date>,
    disabled: Option<Rc<dyn Fn(Date) -> bool + 'a>>,
}

impl<'a> CalendarConstraints<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn min(mut self, min: Option<Date>) -> Self {
        self.min = min;
        self
    }

    #[must_use]
    pub fn max(mut self, max: Option<Date>) -> Self {
        self.max = max;
        self
    }

    #[must_use]
    pub fn disabled_dates(mut self, disabled: impl Fn(Date) -> bool + 'a) -> Self {
        self.disabled = Some(Rc::new(disabled));
        self
    }

    pub fn minimum(&self) -> Option<Date> {
        self.min
    }

    pub fn maximum(&self) -> Option<Date> {
        self.max
    }

    pub fn is_disabled(&self, date: Date) -> bool {
        self.min.is_some_and(|min| date < min)
            || self.max.is_some_and(|max| date > max)
            || self.disabled.as_ref().is_some_and(|test| test(date))
    }

    fn month_in_bounds(&self, month: Month) -> bool {
        self.min.is_none_or(|min| month >= min.month())
            && self.max.is_none_or(|max| month <= max.month())
    }

    pub fn month_has_enabled_day(&self, month: Month) -> bool {
        (1..=month.days())
            .any(|day| !self.is_disabled(Date::new(month.year(), month.number(), day).unwrap()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarCommand {
    PreviousDay,
    NextDay,
    PreviousWeek,
    NextWeek,
    WeekStart,
    WeekEnd,
    PreviousMonth,
    NextMonth,
    PreviousYear,
    NextYear,
}

pub fn keyboard_command(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
    direction: Direction,
) -> Option<CalendarCommand> {
    match key {
        keyboard::Key::Named(Named::ArrowLeft) => Some(match direction {
            Direction::LeftToRight => CalendarCommand::PreviousDay,
            Direction::RightToLeft => CalendarCommand::NextDay,
        }),
        keyboard::Key::Named(Named::ArrowRight) => Some(match direction {
            Direction::LeftToRight => CalendarCommand::NextDay,
            Direction::RightToLeft => CalendarCommand::PreviousDay,
        }),
        keyboard::Key::Named(Named::ArrowUp) => Some(CalendarCommand::PreviousWeek),
        keyboard::Key::Named(Named::ArrowDown) => Some(CalendarCommand::NextWeek),
        keyboard::Key::Named(Named::Home) => Some(CalendarCommand::WeekStart),
        keyboard::Key::Named(Named::End) => Some(CalendarCommand::WeekEnd),
        keyboard::Key::Named(Named::PageUp) if modifiers.shift() => {
            Some(CalendarCommand::PreviousYear)
        }
        keyboard::Key::Named(Named::PageDown) if modifiers.shift() => {
            Some(CalendarCommand::NextYear)
        }
        keyboard::Key::Named(Named::PageUp) => Some(CalendarCommand::PreviousMonth),
        keyboard::Key::Named(Named::PageDown) => Some(CalendarCommand::NextMonth),
        _ => None,
    }
}

/// Finds the next enabled date for one keyboard command.
pub fn navigation_target(
    current: Date,
    command: CalendarCommand,
    constraints: &CalendarConstraints<'_>,
) -> Option<Date> {
    let (candidate, direction, week_bounds) = match command {
        CalendarCommand::PreviousDay => (current.checked_add_days(-1)?, -1, None),
        CalendarCommand::NextDay => (current.checked_add_days(1)?, 1, None),
        CalendarCommand::PreviousWeek => (current.checked_add_days(-7)?, -1, None),
        CalendarCommand::NextWeek => (current.checked_add_days(7)?, 1, None),
        CalendarCommand::WeekStart => {
            let start =
                current.checked_add_days(-i32::from(current.weekday().index_from_sunday()))?;
            (start, 1, Some((start, start.checked_add_days(6)?)))
        }
        CalendarCommand::WeekEnd => {
            let end =
                current.checked_add_days(i32::from(6 - current.weekday().index_from_sunday()))?;
            (end, -1, Some((end.checked_add_days(-6)?, end)))
        }
        CalendarCommand::PreviousMonth => (current.checked_add_months(-1)?, -1, None),
        CalendarCommand::NextMonth => (current.checked_add_months(1)?, 1, None),
        CalendarCommand::PreviousYear => (current.checked_add_years(-1)?, -1, None),
        CalendarCommand::NextYear => (current.checked_add_years(1)?, 1, None),
    };

    enabled_from(candidate, direction, week_bounds, constraints)
}

fn enabled_from(
    mut candidate: Date,
    direction: i32,
    bounds: Option<(Date, Date)>,
    constraints: &CalendarConstraints<'_>,
) -> Option<Date> {
    loop {
        if bounds.is_some_and(|(start, end)| !(start..=end).contains(&candidate)) {
            return None;
        }
        if !constraints.is_disabled(candidate) {
            return Some(candidate);
        }
        candidate = candidate.checked_add_days(direction)?;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarState {
    month: Month,
    selection: CalendarSelection,
    focused: Option<Date>,
}

impl CalendarState {
    pub const fn new(month: Month, selection: CalendarSelection) -> Self {
        Self {
            month,
            selection,
            focused: None,
        }
    }

    #[must_use]
    pub fn focused(mut self, focused: Option<Date>) -> Self {
        self.focused = focused;
        if let Some(date) = focused {
            self.month = date.month();
        }
        self
    }

    pub const fn month(&self) -> Month {
        self.month
    }

    pub const fn selection(&self) -> &CalendarSelection {
        &self.selection
    }

    pub const fn focused_date(&self) -> Option<Date> {
        self.focused
    }

    pub fn apply(&mut self, event: &CalendarEvent) -> bool {
        let previous = self.clone();
        match event {
            CalendarEvent::SelectionChanged { selection, focused } => {
                self.month = focused.month();
                self.selection = selection.clone();
                self.focused = Some(*focused);
            }
            CalendarEvent::FocusMoved { date, month } => {
                self.month = *month;
                self.focused = Some(*date);
            }
            CalendarEvent::MonthChanged(month) => {
                self.month = *month;
                self.focused = self.focused.map(|date| {
                    Date::new(month.year(), month.number(), date.day().min(month.days())).unwrap()
                });
            }
        }
        *self != previous
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarEvent {
    SelectionChanged {
        selection: CalendarSelection,
        focused: Date,
    },
    FocusMoved {
        date: Date,
        month: Month,
    },
    MonthChanged(Month),
}

impl CalendarEvent {
    pub fn selection(&self) -> Option<&CalendarSelection> {
        match self {
            Self::SelectionChanged { selection, .. } => Some(selection),
            Self::FocusMoved { .. } | Self::MonthChanged(_) => None,
        }
    }

    pub const fn month(&self) -> Option<Month> {
        match self {
            Self::FocusMoved { month, .. } | Self::MonthChanged(month) => Some(*month),
            Self::SelectionChanged { focused, .. } => Some(focused.month()),
        }
    }

    pub const fn focused(&self) -> Option<Date> {
        match self {
            Self::FocusMoved { date, .. } | Self::SelectionChanged { focused: date, .. } => {
                Some(*date)
            }
            Self::MonthChanged(_) => None,
        }
    }

    pub fn focus_task<Message>(&self, calendar_id: &str) -> Task<Message> {
        self.focused()
            .map_or_else(Task::none, |date| focus_calendar_day(calendar_id, date))
    }
}

pub fn day_focus_id(calendar_id: &str, date: Date) -> widget::Id {
    widget::Id::from(format!("ducktape-calendar:{calendar_id}:day:{date}"))
}

pub fn focus_calendar_day<Message>(calendar_id: &str, date: Date) -> Task<Message> {
    iced::widget::operation::focus(day_focus_id(calendar_id, date))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthOption(u8);

impl MonthOption {
    pub const fn new(number: u8) -> Option<Self> {
        if number >= 1 && number <= 12 {
            Some(Self(number))
        } else {
            None
        }
    }

    pub const fn number(self) -> u8 {
        self.0
    }
}

impl fmt::Display for MonthOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(MONTH_NAMES[usize::from(self.0 - 1)])
    }
}

pub fn month_options() -> Vec<MonthOption> {
    (1..=12).map(MonthOption).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearOption(i32);

impl YearOption {
    pub const fn year(self) -> i32 {
        self.0
    }
}

impl fmt::Display for YearOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

pub fn year_options(start: i32, end: i32) -> Vec<YearOption> {
    let start = start.clamp(MIN_YEAR, MAX_YEAR);
    let end = end.clamp(MIN_YEAR, MAX_YEAR);
    if start > end {
        Vec::new()
    } else {
        (start..=end).map(YearOption).collect()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DayVisualState {
    pub today: bool,
    pub selected: bool,
    pub range_start: bool,
    pub range_middle: bool,
    pub range_end: bool,
    pub outside: bool,
    pub disabled: bool,
    pub focused: bool,
}

pub fn day_visual_state(
    date: Date,
    month: Month,
    selection: &CalendarSelection,
    today: Option<Date>,
    focused: Option<Date>,
    disabled: bool,
) -> DayVisualState {
    let range = selection.range();
    let range_end = range.and_then(|range| range.end);
    DayVisualState {
        today: today == Some(date),
        selected: selection.is_selected(date),
        range_start: range.is_some_and(|range| range.start == date),
        range_middle: range.is_some_and(|range| {
            range
                .end
                .is_some_and(|end| date > range.start && date < end)
        }),
        range_end: range_end == Some(date),
        outside: !month.contains(date),
        disabled,
        focused: focused == Some(date),
    }
}

pub fn day_style(theme: &Theme, state: DayVisualState, status: Status) -> FocusStyle {
    let disabled = state.disabled || status == Status::Disabled;
    let endpoint = state.selected && (state.range_start || state.range_end);
    let selected = state.selected && (!state.range_middle || endpoint);
    let mut style = FocusStyle {
        background: if selected {
            Some(Background::Color(theme.palette.primary))
        } else if state.range_middle {
            Some(Background::Color(theme.palette.accent))
        } else {
            None
        },
        text_color: Some(if selected {
            theme.palette.primary_foreground
        } else if disabled || state.outside {
            alpha(theme.palette.muted_foreground, 0.5)
        } else {
            theme.palette.foreground
        }),
        border: Border {
            color: if state.today && !selected {
                theme.palette.ring
            } else {
                Color::TRANSPARENT
            },
            width: if state.today && !selected { 1.0 } else { 0.0 },
            radius: if state.range_middle && !endpoint {
                0.0.into()
            } else {
                theme.radius.md.into()
            },
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: 2.0,
            radius: (theme.radius.md + 2.0).into(),
        },
        focus_offset: 1.0,
    };

    if !disabled && !selected && !state.range_middle {
        style.background = match status {
            Status::Hovered => Some(Background::Color(theme.palette.accent)),
            Status::Pressed => Some(Background::Color(mix(
                theme.palette.accent,
                theme.palette.foreground,
                0.08,
            ))),
            Status::Active | Status::Focused | Status::Disabled => style.background,
        };
    }
    style
}

/// Builder for a controlled calendar. The caller applies events to one
/// [`CalendarState`] and returns [`CalendarEvent::focus_task`] from update.
pub struct Calendar<'a, Message>
where
    Message: Clone + 'a,
{
    id: String,
    state: CalendarState,
    today: Option<Date>,
    on_event: Rc<dyn Fn(CalendarEvent) -> Message + 'a>,
    constraints: CalendarConstraints<'a>,
    show_outside_days: bool,
    week_numbers: bool,
    month_dropdown: bool,
    year_dropdown: bool,
    year_range: (i32, i32),
    direction: Direction,
    theme: Theme,
}

pub fn controlled_calendar<'a, Message>(
    id: impl Into<String>,
    state: &CalendarState,
    on_event: impl Fn(CalendarEvent) -> Message + 'a,
    theme: &Theme,
) -> Calendar<'a, Message>
where
    Message: Clone + 'a,
{
    Calendar {
        id: id.into(),
        state: state.clone(),
        today: None,
        on_event: Rc::new(on_event),
        constraints: CalendarConstraints::default(),
        show_outside_days: true,
        week_numbers: false,
        month_dropdown: false,
        year_dropdown: false,
        year_range: (
            (state.month.year() - 50).max(MIN_YEAR),
            (state.month.year() + 50).min(MAX_YEAR),
        ),
        direction: Direction::LeftToRight,
        theme: *theme,
    }
}

impl<'a, Message> Calendar<'a, Message>
where
    Message: Clone + 'a,
{
    #[must_use]
    pub fn today(mut self, today: Option<Date>) -> Self {
        self.today = today;
        self
    }

    #[must_use]
    pub fn min(mut self, min: Option<Date>) -> Self {
        self.constraints.min = min;
        self
    }

    #[must_use]
    pub fn max(mut self, max: Option<Date>) -> Self {
        self.constraints.max = max;
        self
    }

    #[must_use]
    pub fn disabled_dates(mut self, disabled: impl Fn(Date) -> bool + 'a) -> Self {
        self.constraints.disabled = Some(Rc::new(disabled));
        self
    }

    #[must_use]
    pub fn show_outside_days(mut self, show: bool) -> Self {
        self.show_outside_days = show;
        self
    }

    #[must_use]
    pub fn week_numbers(mut self, show: bool) -> Self {
        self.week_numbers = show;
        self
    }

    #[must_use]
    pub fn month_dropdown(mut self, show: bool) -> Self {
        self.month_dropdown = show;
        self
    }

    #[must_use]
    pub fn year_dropdown(mut self, show: bool) -> Self {
        self.year_dropdown = show;
        self
    }

    #[must_use]
    pub fn year_range(mut self, start: i32, end: i32) -> Self {
        if start <= end {
            self.year_range = (
                start.clamp(MIN_YEAR, MAX_YEAR),
                end.clamp(MIN_YEAR, MAX_YEAR),
            );
        }
        self
    }

    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn width(&self) -> f32 {
        CALENDAR_WIDTH
            + if self.week_numbers {
                DAY_CELL_SIZE
            } else {
                0.0
            }
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let width = self.width();
        let header = self.header(width);
        let weekdays = self.weekday_header(width);
        let days = self.day_grid(width);

        Column::new()
            .push(header)
            .push(weekdays)
            .push(days)
            .spacing(0)
            .width(width)
            .into()
    }

    fn header(&self, width: f32) -> Element<'a, Message> {
        let previous_month = self.state.month.previous().filter(|month| {
            self.constraints
                .minimum()
                .is_none_or(|minimum| *month >= minimum.month())
        });
        let next_month = self.state.month.next().filter(|month| {
            self.constraints
                .maximum()
                .is_none_or(|maximum| *month <= maximum.month())
        });
        let previous = button("‹", &self.theme)
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Small)
            .width(32)
            .disabled(previous_month.is_none())
            .on_press((self.on_event)(CalendarEvent::MonthChanged(
                previous_month.unwrap_or(self.state.month),
            )));
        let next = button("›", &self.theme)
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Small)
            .width(32)
            .disabled(next_month.is_none())
            .on_press((self.on_event)(CalendarEvent::MonthChanged(
                next_month.unwrap_or(self.state.month),
            )));
        let caption = self.caption();
        let items: Vec<Element<'a, Message>> = match self.direction {
            Direction::LeftToRight => vec![
                previous.into(),
                container(caption)
                    .width(Length::Fill)
                    .height(DAY_CELL_SIZE)
                    .align_y(Vertical::Center)
                    .into(),
                next.into(),
            ],
            Direction::RightToLeft => vec![
                next.into(),
                container(caption)
                    .width(Length::Fill)
                    .height(DAY_CELL_SIZE)
                    .align_y(Vertical::Center)
                    .into(),
                previous.into(),
            ],
        };

        items
            .into_iter()
            .fold(Row::new(), Row::push)
            .align_y(Alignment::Center)
            .width(width)
            .into()
    }

    fn caption(&self) -> Element<'a, Message> {
        if !self.month_dropdown && !self.year_dropdown {
            return container(
                text(self.state.month.to_string())
                    .size(self.theme.typography.sm)
                    .line_height(LineHeight::Absolute(Pixels(16.0)))
                    .color(self.theme.palette.foreground),
            )
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into();
        }

        let mut parts: Vec<Element<'a, Message>> = Vec::with_capacity(2);
        if self.month_dropdown {
            let event = Rc::clone(&self.on_event);
            let year = self.state.month.year();
            let selected = MonthOption(self.state.month.number());
            parts.push(
                native_select(
                    self.caption_month_options(),
                    Some(selected),
                    move |month: MonthOption| {
                        event(CalendarEvent::MonthChanged(
                            Month::new(year, month.number()).unwrap(),
                        ))
                    },
                    &self.theme,
                )
                .width(104)
                .into(),
            );
        } else {
            parts.push(
                container(
                    text(MONTH_NAMES[usize::from(self.state.month.number() - 1)])
                        .size(self.theme.typography.sm),
                )
                .width(104)
                .height(DAY_CELL_SIZE)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
            );
        }
        if self.year_dropdown {
            let event = Rc::clone(&self.on_event);
            let month = self.state.month.number();
            parts.push(
                native_select(
                    self.caption_year_options(),
                    Some(YearOption(self.state.month.year())),
                    move |year: YearOption| {
                        event(CalendarEvent::MonthChanged(
                            Month::new(year.year(), month).unwrap(),
                        ))
                    },
                    &self.theme,
                )
                .width(80)
                .into(),
            );
        } else {
            parts.push(
                container(text(self.state.month.year()).size(self.theme.typography.sm))
                    .width(80)
                    .height(DAY_CELL_SIZE)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into(),
            );
        }

        let parts = if self.direction == Direction::RightToLeft {
            parts.into_iter().rev().collect()
        } else {
            parts
        };
        container(
            parts
                .into_iter()
                .fold(Row::new().spacing(4), Row::push)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .into()
    }

    fn weekday_header(&self, width: f32) -> Element<'a, Message> {
        let mut cells: Vec<Element<'a, Message>> = WEEKDAYS
            .into_iter()
            .map(|weekday| weekday_cell(weekday.short_name(), &self.theme).into())
            .collect();
        if self.week_numbers {
            cells.insert(0, weekday_cell("Wk", &self.theme).into());
        }
        if self.direction == Direction::RightToLeft {
            cells.reverse();
        }
        cells
            .into_iter()
            .fold(Row::new(), Row::push)
            .width(width)
            .height(DAY_CELL_SIZE)
            .into()
    }

    fn caption_month_options(&self) -> Vec<MonthOption> {
        let year = self.state.month.year();
        month_options()
            .into_iter()
            .filter(|option| {
                self.constraints
                    .month_in_bounds(Month::new(year, option.number()).unwrap())
            })
            .collect()
    }

    fn caption_year_options(&self) -> Vec<YearOption> {
        let month = self.state.month.number();
        year_options(self.year_range.0, self.year_range.1)
            .into_iter()
            .filter(|option| {
                self.constraints
                    .month_in_bounds(Month::new(option.year(), month).unwrap())
            })
            .collect()
    }

    fn day_grid(&self, width: f32) -> Element<'a, Message> {
        let tab_stop = self.day_tab_stop();

        self.state
            .month
            .visible_dates()
            .chunks_exact(WEEKDAY_COUNT)
            .fold(Column::new().width(width), |column, week| {
                let mut cells = week
                    .iter()
                    .map(|date| self.day_cell(*date, tab_stop))
                    .collect::<Vec<_>>();
                if self.week_numbers {
                    let week_number = week
                        .iter()
                        .flatten()
                        .next()
                        .map_or(String::new(), |date| date.iso_week().to_string());
                    cells.insert(0, week_number_cell(week_number, &self.theme).into());
                }
                if self.direction == Direction::RightToLeft {
                    cells.reverse();
                }
                column.push(
                    cells
                        .into_iter()
                        .fold(Row::new(), Row::push)
                        .width(width)
                        .height(DAY_CELL_SIZE),
                )
            })
            .into()
    }

    fn day_tab_stop(&self) -> Option<Date> {
        let enabled =
            |date: Date| self.state.month.contains(date) && !self.constraints.is_disabled(date);
        let visible = self.state.month.visible_dates();

        self.state
            .focused
            .filter(|date| enabled(*date))
            .or_else(|| {
                visible
                    .iter()
                    .flatten()
                    .copied()
                    .find(|date| enabled(*date) && self.state.selection.is_selected(*date))
            })
            .or_else(|| self.today.filter(|date| enabled(*date)))
            .or_else(|| visible.into_iter().flatten().find(|date| enabled(*date)))
    }

    fn day_cell(&self, date: Option<Date>, tab_stop: Option<Date>) -> Element<'a, Message> {
        let Some(date) = date else {
            return Space::new()
                .width(DAY_CELL_SIZE)
                .height(DAY_CELL_SIZE)
                .into();
        };
        if !self.show_outside_days && !self.state.month.contains(date) {
            return Space::new()
                .width(DAY_CELL_SIZE)
                .height(DAY_CELL_SIZE)
                .into();
        }

        let outside = !self.state.month.contains(date);
        let disabled = outside || self.constraints.is_disabled(date);
        let visual = day_visual_state(
            date,
            self.state.month,
            &self.state.selection,
            self.today,
            self.state.focused,
            disabled,
        );
        let content = container(
            text(date.day().to_string())
                .size(self.theme.typography.sm)
                .line_height(LineHeight::Absolute(Pixels(16.0))),
        )
        .width(DAY_CELL_SIZE)
        .height(DAY_CELL_SIZE)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);
        let selection = self.state.selection.clone();
        let activate = (self.on_event)(CalendarEvent::SelectionChanged {
            selection: selection.selected(date),
            focused: date,
        });
        let constraints = self.constraints.clone();
        let direction = self.direction;
        let key_event = Rc::clone(&self.on_event);
        let theme = self.theme;

        FocusControl::new(day_focus_id(&self.id, date), content, activate, &self.theme)
            .disabled(disabled)
            .tab_stop(tab_stop == Some(date))
            .on_key_press(move |key, modifiers| {
                let command = keyboard_command(&key, modifiers, direction)?;
                let target = navigation_target(date, command, &constraints)?;
                Some(key_event(CalendarEvent::FocusMoved {
                    date: target,
                    month: target.month(),
                }))
            })
            .style(move |_iced_theme, status| day_style(&theme, visual, status))
            .into()
    }
}

impl<'a, Message> From<Calendar<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(calendar: Calendar<'a, Message>) -> Self {
        calendar.into_element()
    }
}

fn weekday_cell<'a, Message>(label: &'static str, theme: &Theme) -> Container<'a, Message>
where
    Message: 'a,
{
    container(
        text(label)
            .size(theme.typography.xs)
            .line_height(LineHeight::Absolute(Pixels(14.0)))
            .color(theme.palette.muted_foreground),
    )
    .width(DAY_CELL_SIZE)
    .height(DAY_CELL_SIZE)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
}

fn week_number_cell<'a, Message>(label: String, theme: &Theme) -> Container<'a, Message>
where
    Message: 'a,
{
    container(
        text(label)
            .size(theme.typography.xs)
            .line_height(LineHeight::Absolute(Pixels(14.0)))
            .color(alpha(theme.palette.muted_foreground, 0.8)),
    )
    .width(DAY_CELL_SIZE)
    .height(DAY_CELL_SIZE)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
}

/// Compatibility calendar using shared keyboard-focusable buttons and caller-supplied
/// previous/next messages. New code should use [`controlled_calendar`].
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
                    .line_height(LineHeight::Absolute(Pixels(16.0)))
                    .color(theme.palette.foreground),
            )
            .width(Length::Fill)
            .height(DAY_CELL_SIZE)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
        )
        .push(
            button("›", theme)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Icon)
                .disabled(month.next().is_none())
                .on_press(next),
        )
        .align_y(Alignment::Center)
        .width(CALENDAR_WIDTH);
    let weekdays = WEEKDAYS.into_iter().fold(
        Row::new().width(CALENDAR_WIDTH).height(DAY_CELL_SIZE),
        |row, weekday| row.push(weekday_cell(weekday.short_name(), theme)),
    );
    let days = month.visible_dates().chunks_exact(7).fold(
        Column::new().width(CALENDAR_WIDTH),
        |column, week| {
            column.push(week.iter().fold(Row::new(), |row, date| {
                row.push(match date {
                    Some(date) => legacy_day_button(
                        *date,
                        month.contains(*date),
                        selected == Some(*date),
                        &on_select,
                        theme,
                    ),
                    None => Space::new()
                        .width(DAY_CELL_SIZE)
                        .height(DAY_CELL_SIZE)
                        .into(),
                })
            }))
        },
    );

    Column::new()
        .push(header)
        .push(weekdays)
        .push(days)
        .width(CALENDAR_WIDTH)
}

fn legacy_day_button<'a, Message>(
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
            .line_height(LineHeight::Absolute(Pixels(16.0)))
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
    use super::super::focus_control::focusable_count;
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::advanced::renderer::Headless as _;
    use iced::advanced::{Layout, Shell, clipboard, layout, widget};
    use iced::{Event, Point, Rectangle, Size, mouse};

    fn month(year: i32, number: u8) -> Month {
        Month::new(year, number).unwrap()
    }

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn leap_years_centuries_and_supported_boundaries_are_exact() {
        assert!(Date::new(2024, 2, 29).is_ok());
        assert!(Date::new(2023, 2, 29).is_err());
        assert!(Date::new(1900, 2, 29).is_err());
        assert!(Date::new(2000, 2, 29).is_ok());
        assert_eq!(
            date(2024, 2, 29).checked_add_days(1),
            Some(date(2024, 3, 1))
        );
        assert_eq!(
            date(2024, 2, 29).checked_add_years(1),
            Some(date(2025, 2, 28))
        );
        assert_eq!(date(MIN_YEAR, 1, 1).checked_add_days(-1), None);
        assert_eq!(date(MAX_YEAR, 12, 31).checked_add_days(1), None);
    }

    #[test]
    fn month_navigation_and_six_week_grid_cover_edges() {
        assert_eq!(month(2024, 1).previous(), Some(month(2023, 12)));
        assert_eq!(month(2024, 12).next(), Some(month(2025, 1)));
        assert_eq!(month(MIN_YEAR, 1).previous(), None);
        assert_eq!(month(MAX_YEAR, 12).next(), None);

        let dates = month(2024, 9).visible_dates();
        assert_eq!(dates[0], Some(date(2024, 9, 1)));
        assert_eq!(dates[30], Some(date(2024, 10, 1)));
        assert_eq!(dates[41], Some(date(2024, 10, 12)));
        let minimum = month(MIN_YEAR, 1).visible_dates();
        assert_eq!(minimum[0], None);
        let maximum = month(MAX_YEAR, 12).visible_dates();
        assert!(maximum[34..].iter().all(Option::is_none));
    }

    #[test]
    fn weekdays_and_iso_weeks_match_known_dates() {
        assert_eq!(date(1, 1, 1).weekday(), Weekday::Monday);
        assert_eq!(date(2000, 1, 1).weekday(), Weekday::Saturday);
        assert_eq!(date(2024, 2, 29).weekday(), Weekday::Thursday);
        assert_eq!(date(2021, 1, 1).iso_week(), 53);
        assert_eq!(date(2024, 1, 1).iso_week(), 1);
    }

    #[test]
    fn selection_models_toggle_and_normalize_inclusive_ranges() {
        let first = date(2024, 5, 10);
        let second = date(2024, 5, 5);
        assert_eq!(
            CalendarSelection::Single(None).selected(first),
            CalendarSelection::Single(Some(first))
        );
        let multiple = CalendarSelection::Multiple(vec![first]).selected(second);
        assert_eq!(multiple, CalendarSelection::Multiple(vec![second, first]));
        assert_eq!(
            multiple.selected(first),
            CalendarSelection::Multiple(vec![second])
        );

        let open = CalendarSelection::Range(None).selected(first);
        assert_eq!(open, CalendarSelection::Range(Some(DateRange::open(first))));
        let completed = open.selected(second);
        assert_eq!(
            completed,
            CalendarSelection::Range(Some(DateRange::inclusive(second, first)))
        );
        assert!(completed.is_selected(date(2024, 5, 7)));
        assert_eq!(
            completed.selected(date(2024, 6, 1)),
            CalendarSelection::Range(Some(DateRange::open(date(2024, 6, 1))))
        );
    }

    #[test]
    fn one_state_reducer_keeps_month_selection_and_focus_together() {
        let focused = date(2024, 1, 31);
        let mut state =
            CalendarState::new(month(2024, 1), CalendarSelection::Single(Some(focused)))
                .focused(Some(focused));

        assert!(state.apply(&CalendarEvent::MonthChanged(month(2024, 2))));
        assert_eq!(state.month(), month(2024, 2));
        assert_eq!(state.focused_date(), Some(date(2024, 2, 29)));

        let selected = date(2024, 2, 20);
        assert!(state.apply(&CalendarEvent::SelectionChanged {
            selection: CalendarSelection::Single(Some(selected)),
            focused: selected,
        }));
        assert_eq!(
            state.selection(),
            &CalendarSelection::Single(Some(selected))
        );
        assert_eq!(state.focused_date(), Some(selected));
    }

    #[test]
    fn day_grid_exposes_one_sequential_focus_stop() {
        let selected = date(2024, 5, 17);
        let state = CalendarState::new(month(2024, 5), CalendarSelection::Single(Some(selected)));
        let calendar =
            controlled_calendar("booking", &state, |_| (), &LIGHT).today(Some(date(2024, 5, 18)));

        assert_eq!(calendar.day_tab_stop(), Some(selected));
        assert_eq!(focusable_count(calendar.day_grid(CALENDAR_WIDTH)), 1);
    }

    #[test]
    fn keyboard_navigation_changes_units_and_skips_disabled_dates() {
        let friday = date(2024, 3, 1);
        let constraints = CalendarConstraints::new()
            .min(Some(date(2024, 2, 1)))
            .disabled_dates(|date| date == Date::new(2024, 2, 29).unwrap());
        assert_eq!(
            navigation_target(friday, CalendarCommand::PreviousDay, &constraints),
            Some(date(2024, 2, 28))
        );
        assert_eq!(
            navigation_target(friday, CalendarCommand::NextWeek, &constraints),
            Some(date(2024, 3, 8))
        );
        assert_eq!(
            navigation_target(friday, CalendarCommand::WeekStart, &constraints),
            Some(date(2024, 2, 25))
        );
        assert_eq!(
            navigation_target(friday, CalendarCommand::NextMonth, &constraints),
            Some(date(2024, 4, 1))
        );
    }

    #[test]
    fn pointer_month_navigation_crosses_a_fully_disabled_month() {
        let july = month(2024, 7);
        let august = month(2024, 8);
        let state = CalendarState::new(july, CalendarSelection::Single(None));
        let calendar = controlled_calendar("disabled-month", &state, |event| event, &LIGHT)
            .disabled_dates(move |date| date.month() == august);
        let mut header = calendar.header(CALENDAR_WIDTH);
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(CALENDAR_WIDTH, DAY_CELL_SIZE));
        let mut tree = widget::Tree::new(header.as_widget());
        let node = header.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let layout = Layout::new(&node);
        let next = layout.children().last().unwrap().bounds().center();
        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();

        for event in [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            let mut shell = Shell::new(&mut messages);
            header.as_widget_mut().update(
                &mut tree,
                &event,
                layout,
                mouse::Cursor::Available(next),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }

        assert_eq!(messages, [CalendarEvent::MonthChanged(august)]);
    }

    #[test]
    fn month_navigation_stops_at_hard_date_bounds() {
        let august = month(2024, 8);
        let state = CalendarState::new(august, CalendarSelection::Single(None));
        let calendar = controlled_calendar("bounded-month", &state, |_| (), &LIGHT)
            .min(Some(date(2024, 8, 12)))
            .max(Some(date(2024, 8, 20)));

        assert_eq!(focusable_count(calendar.header(CALENDAR_WIDTH)), 0);
    }

    #[test]
    fn dropdown_options_stop_at_hard_date_bounds() {
        let state = CalendarState::new(month(2026, 7), CalendarSelection::Single(None));
        let calendar = controlled_calendar("bounded-dropdown", &state, |_| (), &LIGHT)
            .min(Some(date(2026, 6, 15)))
            .max(Some(date(2026, 8, 20)))
            .year_range(2024, 2028);

        assert_eq!(
            calendar.caption_month_options(),
            [MonthOption(6), MonthOption(7), MonthOption(8)]
        );
        assert_eq!(calendar.caption_year_options(), [YearOption(2026)]);
    }

    #[test]
    fn key_mapping_respects_shift_and_layout_direction() {
        let none = keyboard::Modifiers::empty();
        assert_eq!(
            keyboard_command(
                &keyboard::Key::Named(Named::ArrowLeft),
                none,
                Direction::RightToLeft
            ),
            Some(CalendarCommand::NextDay)
        );
        assert_eq!(
            keyboard_command(
                &keyboard::Key::Named(Named::PageUp),
                keyboard::Modifiers::SHIFT,
                Direction::LeftToRight
            ),
            Some(CalendarCommand::PreviousYear)
        );
    }

    #[test]
    fn day_ids_geometry_and_visual_states_are_stable() {
        let selected = CalendarSelection::Range(Some(DateRange::inclusive(
            date(2024, 5, 5),
            date(2024, 5, 7),
        )));
        let state = day_visual_state(
            date(2024, 5, 6),
            month(2024, 5),
            &selected,
            Some(date(2024, 5, 6)),
            Some(date(2024, 5, 6)),
            false,
        );
        assert!(state.today && state.selected && state.range_middle && state.focused);
        assert_eq!(CALENDAR_WIDTH, DAY_CELL_SIZE * 7.0);
        assert_eq!(
            day_focus_id("booking", date(2024, 5, 6)),
            day_focus_id("booking", date(2024, 5, 6))
        );
        assert_ne!(
            day_focus_id("booking", date(2024, 5, 6)),
            day_focus_id("booking", date(2024, 5, 7))
        );
    }

    #[test]
    fn day_styles_keep_center_state_contrast_in_light_and_dark() {
        for theme in [LIGHT, DARK] {
            let selected = day_style(
                &theme,
                DayVisualState {
                    selected: true,
                    range_start: true,
                    ..DayVisualState::default()
                },
                Status::Focused,
            );
            let disabled = day_style(
                &theme,
                DayVisualState {
                    disabled: true,
                    ..DayVisualState::default()
                },
                Status::Disabled,
            );
            assert_eq!(
                selected.background,
                Some(Background::Color(theme.palette.primary))
            );
            assert_eq!(selected.focus_ring.color, theme.palette.ring);
            assert!(disabled.text_color.unwrap().a < 1.0);
        }
    }

    #[test]
    fn dropdown_helpers_bound_options_without_invalid_dates() {
        assert_eq!(month_options().len(), 12);
        assert_eq!(year_options(2020, 2024).len(), 5);
        assert!(year_options(2024, 2020).is_empty());
        assert_eq!(MonthOption::new(12).unwrap().to_string(), "December");
        assert_eq!(MonthOption::new(13), None);
    }
}
