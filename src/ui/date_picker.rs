//! Single- and range-date pickers composed from the shared popover and calendar.
//!
//! State stays controlled. Opening focuses a stable day ID; Escape, outside
//! press, and completed selection restore the stable trigger ID. The day grid
//! has full keyboard navigation, while optional month/year pick lists are also
//! focusable and keyboard-capable.

use std::rc::Rc;

use super::calendar::{
    CALENDAR_WIDTH, CalendarEvent, CalendarSelection, CalendarState, Date, DateRange, Month,
    controlled_calendar, focus_calendar_day,
};
use super::direction::Direction;
use super::popover::{
    Alignment as PopoverAlignment, DismissReason, Placement, PopoverEvent, PopoverIds, popover,
};
use super::theme::{Theme, alpha};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::LineHeight;
use iced::widget::{Row, container, text};
use iced::{Alignment, Background, Border, Element, Length, Padding, Pixels, Task};

pub const DATE_PICKER_HEIGHT: f32 = 36.0;
pub const DATE_PICKER_WIDTH: f32 = 240.0;
pub const DATE_PICKER_PANEL_PADDING: f32 = 12.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatePickerIds {
    popover: PopoverIds,
    calendar: String,
}

impl DatePickerIds {
    pub fn new(key: impl ToString) -> Self {
        let key = key.to_string();
        Self {
            popover: PopoverIds::new(format!("date-picker:{key}")),
            calendar: format!("date-picker:{key}:calendar"),
        }
    }

    pub fn popover(&self) -> &PopoverIds {
        &self.popover
    }

    pub fn calendar(&self) -> &str {
        &self.calendar
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatePickerValue {
    Single(Option<Date>),
    Range(Option<DateRange>),
}

impl DatePickerValue {
    pub fn selected(&self, date: Date) -> Self {
        match self {
            Self::Single(_) => Self::Single(Some(date)),
            Self::Range(range) => {
                let selection = CalendarSelection::Range(*range).selected(date);
                match selection {
                    CalendarSelection::Range(range) => Self::Range(range),
                    CalendarSelection::Single(_) | CalendarSelection::Multiple(_) => unreachable!(),
                }
            }
        }
    }

    pub const fn is_complete(&self) -> bool {
        match self {
            Self::Single(selected) => selected.is_some(),
            Self::Range(Some(range)) => range.is_complete(),
            Self::Range(None) => false,
        }
    }

    pub const fn anchor(&self) -> Option<Date> {
        match self {
            Self::Single(selected) => *selected,
            Self::Range(Some(range)) => match range.end {
                Some(end) => Some(end),
                None => Some(range.start),
            },
            Self::Range(None) => None,
        }
    }

    pub fn as_calendar_selection(&self) -> CalendarSelection {
        match self {
            Self::Single(selected) => CalendarSelection::Single(*selected),
            Self::Range(range) => CalendarSelection::Range(*range),
        }
    }

    pub fn from_calendar(selection: &CalendarSelection) -> Option<Self> {
        match selection {
            CalendarSelection::Single(selected) => Some(Self::Single(*selected)),
            CalendarSelection::Range(range) => Some(Self::Range(*range)),
            CalendarSelection::Multiple(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatePickerEvent {
    OpenChanged {
        open: bool,
        reason: Option<DismissReason>,
        focus: Option<Date>,
    },
    Calendar(CalendarEvent),
}

impl DatePickerEvent {
    pub const fn next_open(&self, current: bool) -> bool {
        match self {
            Self::OpenChanged { open, .. } => *open,
            Self::Calendar(CalendarEvent::SelectionChanged { selection, .. }) => {
                match selection_complete(selection) {
                    Some(true) => false,
                    Some(false) | None => current,
                }
            }
            Self::Calendar(CalendarEvent::FocusMoved { .. } | CalendarEvent::MonthChanged(_)) => {
                current
            }
        }
    }

    pub fn value(&self) -> Option<DatePickerValue> {
        match self {
            Self::Calendar(CalendarEvent::SelectionChanged { selection, .. }) => {
                DatePickerValue::from_calendar(selection)
            }
            Self::OpenChanged { .. }
            | Self::Calendar(CalendarEvent::FocusMoved { .. } | CalendarEvent::MonthChanged(_)) => {
                None
            }
        }
    }

    pub const fn month(&self) -> Option<Month> {
        match self {
            Self::Calendar(event) => event.month(),
            Self::OpenChanged { .. } => None,
        }
    }

    pub const fn focused(&self) -> Option<Date> {
        match self {
            Self::OpenChanged {
                open: true, focus, ..
            } => *focus,
            Self::Calendar(event) => event.focused(),
            Self::OpenChanged { open: false, .. } => None,
        }
    }

    /// Completes day-grid focus movement and all popover focus handoffs.
    pub fn focus_task<Message>(&self, ids: &DatePickerIds) -> Task<Message> {
        match self {
            Self::OpenChanged {
                open: true,
                focus: Some(date),
                ..
            } => focus_calendar_day(ids.calendar(), *date),
            Self::OpenChanged {
                open: true,
                focus: None,
                ..
            } => PopoverEvent::Open.focus_task(ids.popover()),
            Self::OpenChanged {
                open: false,
                reason,
                ..
            } => PopoverEvent::Close(reason.unwrap_or(DismissReason::Outside))
                .focus_task(ids.popover()),
            Self::Calendar(event @ CalendarEvent::FocusMoved { .. }) => {
                event.focus_task(ids.calendar())
            }
            Self::Calendar(CalendarEvent::SelectionChanged { selection, .. })
                if selection_complete(selection) == Some(true) =>
            {
                PopoverEvent::Close(DismissReason::Trigger).focus_task(ids.popover())
            }
            Self::Calendar(
                CalendarEvent::SelectionChanged { .. } | CalendarEvent::MonthChanged(_),
            ) => Task::none(),
        }
    }
}

const fn selection_complete(selection: &CalendarSelection) -> Option<bool> {
    match selection {
        CalendarSelection::Single(selected) => Some(selected.is_some()),
        CalendarSelection::Range(Some(range)) => Some(range.is_complete()),
        CalendarSelection::Range(None) => Some(false),
        CalendarSelection::Multiple(_) => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DateFormat {
    Iso,
    MonthDayYear,
    DayMonthYear,
    #[default]
    Long,
}

impl DateFormat {
    pub fn format(self, date: Date) -> String {
        match self {
            Self::Iso => date.to_string(),
            Self::MonthDayYear => format!(
                "{:02}/{:02}/{:04}",
                date.month_number(),
                date.day(),
                date.year()
            ),
            Self::DayMonthYear => format!(
                "{:02}/{:02}/{:04}",
                date.day(),
                date.month_number(),
                date.year()
            ),
            Self::Long => format!(
                "{} {}, {}",
                super::calendar::MONTH_NAMES[usize::from(date.month_number() - 1)],
                date.day(),
                date.year()
            ),
        }
    }
}

pub fn format_value(value: &DatePickerValue, formatter: impl Fn(Date) -> String) -> Option<String> {
    match value {
        DatePickerValue::Single(Some(date)) => Some(formatter(*date)),
        DatePickerValue::Single(None) | DatePickerValue::Range(None) => None,
        DatePickerValue::Range(Some(DateRange { start, end: None })) => {
            Some(format!("{} – …", formatter(*start)))
        }
        DatePickerValue::Range(Some(DateRange {
            start,
            end: Some(end),
        })) => Some(format!("{} – {}", formatter(*start), formatter(*end))),
    }
}

pub struct DatePicker<'a, Message>
where
    Message: Clone + 'a,
{
    ids: DatePickerIds,
    month: Month,
    focused: Option<Date>,
    value: DatePickerValue,
    open: bool,
    on_event: Rc<dyn Fn(DatePickerEvent) -> Message + 'a>,
    min: Option<Date>,
    max: Option<Date>,
    disabled_dates: Option<Rc<dyn Fn(Date) -> bool + 'a>>,
    today: Option<Date>,
    show_outside_days: bool,
    week_numbers: bool,
    month_dropdown: bool,
    year_dropdown: bool,
    year_range: Option<(i32, i32)>,
    placeholder: String,
    date_format: DateFormat,
    formatter: Option<Rc<dyn Fn(Date) -> String + 'a>>,
    direction: Direction,
    width: f32,
    disabled: bool,
    invalid: bool,
    theme: Theme,
}

/// Apply emitted events to controlled state, then return
/// [`DatePickerEvent::focus_task`] from `update`.
#[allow(clippy::too_many_arguments)]
pub fn date_picker<'a, Message>(
    ids: DatePickerIds,
    month: Month,
    focused: Option<Date>,
    value: &DatePickerValue,
    open: bool,
    on_event: impl Fn(DatePickerEvent) -> Message + 'a,
    theme: &Theme,
) -> DatePicker<'a, Message>
where
    Message: Clone + 'a,
{
    DatePicker {
        ids,
        month,
        focused,
        value: value.clone(),
        open,
        on_event: Rc::new(on_event),
        min: None,
        max: None,
        disabled_dates: None,
        today: None,
        show_outside_days: true,
        week_numbers: false,
        month_dropdown: false,
        year_dropdown: false,
        year_range: None,
        placeholder: "Pick a date".into(),
        date_format: DateFormat::Long,
        formatter: None,
        direction: Direction::LeftToRight,
        width: DATE_PICKER_WIDTH,
        disabled: false,
        invalid: false,
        theme: *theme,
    }
}

impl<'a, Message> DatePicker<'a, Message>
where
    Message: Clone + 'a,
{
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
        self.disabled_dates = Some(Rc::new(disabled));
        self
    }

    #[must_use]
    pub fn today(mut self, today: Option<Date>) -> Self {
        self.today = today;
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
            self.year_range = Some((start, end));
        }
        self
    }

    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    #[must_use]
    pub fn format(mut self, format: DateFormat) -> Self {
        self.date_format = format;
        self.formatter = None;
        self
    }

    #[must_use]
    pub fn format_with(mut self, formatter: impl Fn(Date) -> String + 'a) -> Self {
        self.formatter = Some(Rc::new(formatter));
        self
    }

    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let preferred_focus = preferred_focus(
            self.month,
            &self.value,
            self.focused,
            self.today,
            self.min,
            self.max,
            self.disabled_dates.as_deref(),
        );
        let formatter: Rc<dyn Fn(Date) -> String + 'a> = self.formatter.unwrap_or_else(|| {
            let format = self.date_format;
            Rc::new(move |date| format.format(date))
        });
        let value_label = format_value(&self.value, |date| formatter(date));
        let foreground = if self.disabled {
            alpha(self.theme.palette.foreground, 0.5)
        } else if value_label.is_some() {
            self.theme.palette.foreground
        } else {
            self.theme.palette.muted_foreground
        };
        let label = container(
            text(value_label.unwrap_or(self.placeholder))
                .size(self.theme.typography.sm)
                .line_height(LineHeight::Absolute(Pixels(16.0)))
                .color(foreground),
        )
        .width(Length::Fill)
        .align_x(self.direction.start())
        .align_y(Vertical::Center);
        let icon = container(
            text("▦")
                .size(self.theme.typography.base)
                .line_height(LineHeight::Absolute(Pixels(16.0)))
                .color(alpha(foreground, 0.82)),
        )
        .width(16)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);
        let trigger_content = match self.direction {
            Direction::LeftToRight => Row::new().push(icon).push(label),
            Direction::RightToLeft => Row::new().push(label).push(icon),
        }
        .align_y(Alignment::Center)
        .spacing(self.theme.spacing.sm)
        .width(Length::Fill);
        let theme = self.theme;
        let invalid = self.invalid;
        let disabled = self.disabled;
        let trigger = container(trigger_content)
            .width(self.width)
            .height(DATE_PICKER_HEIGHT)
            .padding([0.0, 12.0])
            .align_y(Vertical::Center)
            .style(move |_iced_theme| trigger_style(&theme, invalid, disabled));

        let calendar_state = calendar_state(self.month, &self.value, self.focused);
        let calendar_event = Rc::clone(&self.on_event);
        let disabled_dates = self.disabled_dates.clone();
        let mut calendar = controlled_calendar(
            self.ids.calendar.clone(),
            &calendar_state,
            move |event| calendar_event(DatePickerEvent::Calendar(event)),
            &self.theme,
        )
        .today(self.today)
        .min(self.min)
        .max(self.max)
        .disabled_dates(move |date| disabled_dates.as_ref().is_some_and(|test| test(date)))
        .show_outside_days(self.show_outside_days)
        .week_numbers(self.week_numbers)
        .month_dropdown(self.month_dropdown)
        .year_dropdown(self.year_dropdown)
        .direction(self.direction);
        if let Some((start, end)) = self.year_range {
            calendar = calendar.year_range(start, end);
        }
        let calendar_width = CALENDAR_WIDTH
            + if self.week_numbers {
                super::calendar::DAY_CELL_SIZE
            } else {
                0.0
            };
        let popover_event = Rc::clone(&self.on_event);
        let alignment = match self.direction {
            Direction::LeftToRight => PopoverAlignment::Start,
            Direction::RightToLeft => PopoverAlignment::End,
        };

        popover(
            self.ids.popover,
            trigger,
            calendar,
            self.open,
            move |event| match event {
                PopoverEvent::Open => popover_event(DatePickerEvent::OpenChanged {
                    open: true,
                    reason: None,
                    focus: preferred_focus,
                }),
                PopoverEvent::Close(reason) => popover_event(DatePickerEvent::OpenChanged {
                    open: false,
                    reason: Some(reason),
                    focus: None,
                }),
            },
            &self.theme,
        )
        .placement(Placement::Bottom)
        .alignment(alignment)
        .width(calendar_width + DATE_PICKER_PANEL_PADDING * 2.0)
        .max_width(calendar_width + DATE_PICKER_PANEL_PADDING * 2.0)
        .padding(Padding::new(DATE_PICKER_PANEL_PADDING))
        .disabled(self.disabled)
        .into()
    }
}

impl<'a, Message> From<DatePicker<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(picker: DatePicker<'a, Message>) -> Self {
        picker.into_element()
    }
}

fn calendar_state(month: Month, value: &DatePickerValue, focused: Option<Date>) -> CalendarState {
    CalendarState::new(month, value.as_calendar_selection())
        .focused(focused.filter(|date| month.contains(*date)))
}

fn preferred_focus(
    month: Month,
    value: &DatePickerValue,
    focused: Option<Date>,
    today: Option<Date>,
    min: Option<Date>,
    max: Option<Date>,
    disabled: Option<&dyn Fn(Date) -> bool>,
) -> Option<Date> {
    let enabled = |date: Date| {
        month.contains(date)
            && min.is_none_or(|min| date >= min)
            && max.is_none_or(|max| date <= max)
            && disabled.is_none_or(|test| !test(date))
    };
    [value.anchor(), focused, today]
        .into_iter()
        .flatten()
        .find(|date| enabled(*date))
        .or_else(|| {
            (1..=month.days())
                .map(|day| Date::new(month.year(), month.number(), day).unwrap())
                .find(|date| enabled(*date))
        })
}

pub fn trigger_style(
    theme: &Theme,
    invalid: bool,
    disabled: bool,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(if disabled {
            alpha(theme.palette.foreground, 0.5)
        } else {
            theme.palette.foreground
        }),
        background: Some(Background::Color(if disabled {
            alpha(theme.palette.background, 0.7)
        } else {
            theme.palette.background
        })),
        border: Border {
            color: if invalid {
                theme.palette.destructive
            } else {
                theme.palette.input
            },
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    fn selection_event(selection: CalendarSelection, focused: Date) -> DatePickerEvent {
        DatePickerEvent::Calendar(CalendarEvent::SelectionChanged { selection, focused })
    }

    #[test]
    fn value_reducers_close_single_and_only_completed_ranges() {
        let single = DatePickerValue::Single(None).selected(date(2024, 7, 4));
        let single_event = selection_event(single.as_calendar_selection(), date(2024, 7, 4));
        assert!(!single_event.next_open(true));

        let open_range = DatePickerValue::Range(None).selected(date(2024, 7, 4));
        let range_event = selection_event(open_range.as_calendar_selection(), date(2024, 7, 4));
        assert!(range_event.next_open(true));
        let completed = open_range.selected(date(2024, 7, 1));
        let complete_event = selection_event(completed.as_calendar_selection(), date(2024, 7, 1));
        assert!(!complete_event.next_open(true));
        assert_eq!(complete_event.value(), Some(completed));
    }

    #[test]
    fn dismissal_and_navigation_reducers_preserve_the_right_state() {
        let close = DatePickerEvent::OpenChanged {
            open: false,
            reason: Some(DismissReason::Escape),
            focus: None,
        };
        assert!(!close.next_open(true));
        let moved = DatePickerEvent::Calendar(CalendarEvent::FocusMoved {
            date: date(2024, 8, 1),
            month: Month::new(2024, 8).unwrap(),
        });
        assert!(moved.next_open(true));
        assert_eq!(moved.focused(), Some(date(2024, 8, 1)));
        assert_eq!(moved.month(), Some(Month::new(2024, 8).unwrap()));
    }

    #[test]
    fn formatting_covers_placeholders_open_ranges_and_custom_models() {
        assert_eq!(DateFormat::Iso.format(date(2024, 2, 29)), "2024-02-29");
        assert_eq!(
            DateFormat::Long.format(date(2024, 2, 29)),
            "February 29, 2024"
        );
        let range = DatePickerValue::Range(Some(DateRange::open(date(2024, 2, 29))));
        assert_eq!(
            format_value(&range, |date| DateFormat::Iso.format(date)),
            Some("2024-02-29 – …".into())
        );
        assert_eq!(
            format_value(&DatePickerValue::Single(None), |date| date.to_string()),
            None
        );
    }

    #[test]
    fn preferred_focus_skips_out_of_month_and_disabled_candidates() {
        let month = Month::new(2024, 7).unwrap();
        let value = DatePickerValue::Single(Some(date(2024, 6, 30)));
        assert_eq!(
            preferred_focus(
                month,
                &value,
                Some(date(2024, 7, 1)),
                Some(date(2024, 7, 2)),
                None,
                None,
                Some(&|date| date.day() == 1),
            ),
            Some(date(2024, 7, 2))
        );
    }

    #[test]
    fn stale_focus_does_not_restore_the_previous_month() {
        let august = Month::new(2024, 8).unwrap();
        let state = calendar_state(
            august,
            &DatePickerValue::Single(None),
            Some(date(2024, 7, 16)),
        );
        assert_eq!(state.month(), august);
        assert_eq!(state.focused_date(), None);
    }

    #[test]
    fn ids_and_geometry_are_stable() {
        let first = DatePickerIds::new("booking");
        let second = DatePickerIds::new("booking");
        assert_eq!(first, second);
        assert_eq!(first.calendar(), "date-picker:booking:calendar");
        assert_eq!(DATE_PICKER_HEIGHT, 36.0);
        assert_eq!(DATE_PICKER_PANEL_PADDING, 12.0);
    }

    #[test]
    fn trigger_states_use_semantic_light_and_dark_tokens() {
        for theme in [LIGHT, DARK] {
            let normal = trigger_style(&theme, false, false);
            let invalid = trigger_style(&theme, true, false);
            let disabled = trigger_style(&theme, false, true);
            assert_eq!(normal.border.color, theme.palette.input);
            assert_eq!(invalid.border.color, theme.palette.destructive);
            assert!(disabled.text_color.unwrap().a < 1.0);
        }
    }
}
