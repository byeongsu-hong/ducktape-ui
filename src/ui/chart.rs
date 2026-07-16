use std::collections::{BTreeMap, HashSet};
use std::f32::consts::{PI, TAU};
use std::fmt;
use std::rc::Rc;

use super::theme::{Theme as UiTheme, alpha, mix};
use iced::advanced::text::Alignment as TextAlignment;
use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{Canvas, Column, Container, Row, Space, Stack, container, row, text};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Pixels, Point, Radians, Rectangle,
    Shadow, Vector,
};

const DEFAULT_HEIGHT: f32 = 260.0;
const LINE_WIDTH: f32 = 2.0;
const GRID_WIDTH: f32 = 1.0;
const MARKER_RADIUS: f32 = 3.0;
const ACTIVE_MARKER_RADIUS: f32 = 5.0;
const BAR_GAP: f32 = 2.0;
const HIT_RADIUS: f32 = 12.0;

/// A stable semantic color or an exact light/dark pair for one data series.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartColor {
    Primary,
    Accent,
    Ring,
    Success,
    Warning,
    Destructive,
    LightDark { light: Color, dark: Color },
}

impl ChartColor {
    pub fn resolve(self, theme: &UiTheme) -> Color {
        match self {
            Self::Primary => theme.palette.primary,
            Self::Accent => theme.palette.accent_foreground,
            Self::Ring => theme.palette.ring,
            Self::Success => theme.palette.success,
            Self::Warning => theme.palette.warning,
            Self::Destructive => theme.palette.destructive,
            Self::LightDark { light, dark } => {
                if luminance(theme.palette.background) < luminance(theme.palette.foreground) {
                    dark
                } else {
                    light
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeriesConfig {
    pub key: String,
    pub label: String,
    pub color: ChartColor,
}

impl SeriesConfig {
    pub fn new(key: impl Into<String>, label: impl Into<String>, color: ChartColor) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            color,
        }
    }
}

/// Visual metadata, deliberately separate from the chart's numeric data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChartConfig {
    pub series: Vec<SeriesConfig>,
}

impl ChartConfig {
    pub fn new(series: impl IntoIterator<Item = SeriesConfig>) -> Self {
        Self {
            series: series.into_iter().collect(),
        }
    }

    pub fn validate(&self) -> Result<(), ChartError> {
        if self.series.is_empty() {
            return Err(ChartError::EmptyConfig);
        }

        let mut keys = HashSet::with_capacity(self.series.len());
        for series in &self.series {
            if series.key.trim().is_empty() {
                return Err(ChartError::EmptySeriesKey);
            }
            if !keys.insert(series.key.as_str()) {
                return Err(ChartError::DuplicateSeriesKey(series.key.clone()));
            }
        }

        Ok(())
    }

    pub fn series(&self, key: &str) -> Option<&SeriesConfig> {
        self.series.iter().find(|series| series.key == key)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChartDatum {
    pub x: f32,
    pub label: String,
    pub values: BTreeMap<String, f32>,
    pub metadata: BTreeMap<String, String>,
}

impl ChartDatum {
    pub fn new(x: f32, label: impl Into<String>) -> Self {
        Self {
            x,
            label: label.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_value(mut self, series_key: impl Into<String>, value: f32) -> Self {
        self.values.insert(series_key.into(), value);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Adds a per-series name for [`TooltipOptions::name_key`].
    #[must_use]
    pub fn with_series_name(
        mut self,
        name_key: &str,
        series_key: &str,
        value: impl Into<String>,
    ) -> Self {
        self.metadata
            .insert(format!("{name_key}:{series_key}"), value.into());
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChartData {
    pub points: Vec<ChartDatum>,
}

impl ChartData {
    pub fn new(points: impl IntoIterator<Item = ChartDatum>) -> Self {
        Self {
            points: points.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartError {
    EmptyConfig,
    EmptySeriesKey,
    DuplicateSeriesKey(String),
    InvalidDomain(&'static str),
    InvalidInnerRadius,
    TooSmall,
}

impl fmt::Display for ChartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyConfig => formatter.write_str("chart config has no series"),
            Self::EmptySeriesKey => formatter.write_str("chart series keys cannot be empty"),
            Self::DuplicateSeriesKey(key) => write!(formatter, "duplicate chart series key: {key}"),
            Self::InvalidDomain(axis) => write!(formatter, "invalid {axis}-axis domain"),
            Self::InvalidInnerRadius => {
                formatter.write_str("pie inner radius must be between 0 and 1")
            }
            Self::TooSmall => formatter.write_str("chart bounds are too small for its padding"),
        }
    }
}

impl std::error::Error for ChartError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataState {
    Empty,
    Ready,
    Partial,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataReport {
    pub state: DataState,
    pub accepted_points: usize,
    pub dropped_points: usize,
    pub dropped_values: usize,
    pub unknown_values: usize,
}

impl DataReport {
    fn finish(mut self, source_is_empty: bool) -> Self {
        self.state = if source_is_empty {
            DataState::Empty
        } else if self.accepted_points == 0 {
            DataState::Invalid
        } else if self.dropped_points + self.dropped_values + self.unknown_values > 0 {
            DataState::Partial
        } else {
            DataState::Ready
        };
        self
    }
}

impl Default for DataReport {
    fn default() -> Self {
        Self {
            state: DataState::Empty,
            accepted_points: 0,
            dropped_points: 0,
            dropped_values: 0,
            unknown_values: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisDomain {
    pub min: f32,
    pub max: f32,
}

impl AxisDomain {
    pub const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    fn validate(self, axis: &'static str) -> Result<Self, ChartError> {
        if self.min.is_finite() && self.max.is_finite() && self.min < self.max {
            Ok(self)
        } else {
            Err(ChartError::InvalidDomain(axis))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DomainSpec {
    pub x: Option<AxisDomain>,
    pub y: Option<AxisDomain>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartDomain {
    pub x: AxisDomain,
    pub y: AxisDomain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartPadding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for ChartPadding {
    fn default() -> Self {
        Self {
            top: 16.0,
            right: 16.0,
            bottom: 34.0,
            left: 48.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BarLayout {
    #[default]
    Grouped,
    Stacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartesianKind {
    Line { points: bool },
    Area { points: bool },
    Bar(BarLayout),
}

impl Default for CartesianKind {
    fn default() -> Self {
        Self::Line { points: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartesianOptions {
    pub kind: CartesianKind,
    pub padding: ChartPadding,
    pub domain: DomainSpec,
    pub tick_count: usize,
    pub show_grid: bool,
}

impl Default for CartesianOptions {
    fn default() -> Self {
        Self {
            kind: CartesianKind::default(),
            padding: ChartPadding::default(),
            domain: DomainSpec::default(),
            tick_count: 5,
            show_grid: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlotPoint {
    pub datum_index: usize,
    pub position: Point,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CartesianMark {
    Line {
        series_key: String,
        points: Vec<PlotPoint>,
    },
    Area {
        series_key: String,
        points: Vec<PlotPoint>,
        baseline_y: f32,
    },
    Bar {
        series_key: String,
        datum_index: usize,
        value: f32,
        bounds: Rectangle,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatumGeometry {
    pub datum_index: usize,
    pub x: f32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CartesianGeometry {
    pub plot: Rectangle,
    pub domain: ChartDomain,
    pub marks: Vec<CartesianMark>,
    pub datums: Vec<DatumGeometry>,
    pub report: DataReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartHit {
    pub datum_index: usize,
    pub series_key: String,
}

impl CartesianGeometry {
    pub fn hit_test(&self, point: Point, radius: f32) -> Option<ChartHit> {
        for mark in self.marks.iter().rev() {
            if let CartesianMark::Bar {
                series_key,
                datum_index,
                bounds,
                ..
            } = mark
                && bounds.contains(point)
            {
                return Some(ChartHit {
                    datum_index: *datum_index,
                    series_key: series_key.clone(),
                });
            }
        }

        let radius_squared = radius.max(0.0).powi(2);
        self.marks
            .iter()
            .filter_map(|mark| match mark {
                CartesianMark::Line { series_key, points }
                | CartesianMark::Area {
                    series_key, points, ..
                } => Some((series_key, points)),
                CartesianMark::Bar { .. } => None,
            })
            .flat_map(|(series_key, points)| {
                points.iter().map(move |point_geometry| {
                    let delta = point - point_geometry.position;
                    (
                        delta.x * delta.x + delta.y * delta.y,
                        series_key,
                        point_geometry,
                    )
                })
            })
            .filter(|(distance, _, _)| *distance <= radius_squared)
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, series_key, point)| ChartHit {
                datum_index: point.datum_index,
                series_key: series_key.clone(),
            })
    }
}

#[derive(Debug)]
struct PreparedDatum {
    source_index: usize,
    x: f32,
    label: String,
    values: Vec<Option<f32>>,
}

fn prepare_data(config: &ChartConfig, data: &ChartData) -> (Vec<PreparedDatum>, DataReport) {
    let known: HashSet<&str> = config
        .series
        .iter()
        .map(|series| series.key.as_str())
        .collect();
    let mut report = DataReport::default();
    let mut points = Vec::with_capacity(data.points.len());

    for (source_index, datum) in data.points.iter().enumerate() {
        if !datum.x.is_finite() {
            report.dropped_points += 1;
            continue;
        }

        report.unknown_values += datum
            .values
            .keys()
            .filter(|key| !known.contains(key.as_str()))
            .count();

        let mut has_value = false;
        let values = config
            .series
            .iter()
            .map(|series| match datum.values.get(&series.key).copied() {
                Some(value) if value.is_finite() => {
                    has_value = true;
                    Some(value)
                }
                Some(_) => {
                    report.dropped_values += 1;
                    None
                }
                None => None,
            })
            .collect();

        if has_value {
            report.accepted_points += 1;
            points.push(PreparedDatum {
                source_index,
                x: datum.x,
                label: datum.label.clone(),
                values,
            });
        } else {
            report.dropped_points += 1;
        }
    }

    (points, report.finish(data.points.is_empty()))
}

pub fn cartesian_geometry(
    config: &ChartConfig,
    data: &ChartData,
    bounds: Rectangle,
    options: CartesianOptions,
) -> Result<CartesianGeometry, ChartError> {
    config.validate()?;
    let plot = plot_bounds(bounds, options.padding)?;
    let (points, report) = prepare_data(config, data);
    let domain = resolve_domain(&points, options)?;
    let datums = points
        .iter()
        .map(|datum| DatumGeometry {
            datum_index: datum.source_index,
            x: map_x(datum.x, domain.x, plot),
            label: datum.label.clone(),
        })
        .collect();
    let marks = marks(config, &points, plot, domain, options.kind);

    Ok(CartesianGeometry {
        plot,
        domain,
        marks,
        datums,
        report,
    })
}

fn plot_bounds(bounds: Rectangle, padding: ChartPadding) -> Result<Rectangle, ChartError> {
    if ![padding.top, padding.right, padding.bottom, padding.left]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
    {
        return Err(ChartError::TooSmall);
    }

    let width = bounds.width - padding.left - padding.right;
    let height = bounds.height - padding.top - padding.bottom;
    if width <= 1.0 || height <= 1.0 {
        return Err(ChartError::TooSmall);
    }

    Ok(Rectangle {
        x: bounds.x + padding.left,
        y: bounds.y + padding.top,
        width,
        height,
    })
}

fn resolve_domain(
    points: &[PreparedDatum],
    options: CartesianOptions,
) -> Result<ChartDomain, ChartError> {
    let computed_x = if points.is_empty() {
        AxisDomain::new(-1.0, 1.0)
    } else {
        let min = points
            .iter()
            .map(|datum| datum.x)
            .fold(f32::INFINITY, f32::min);
        let max = points
            .iter()
            .map(|datum| datum.x)
            .fold(f32::NEG_INFINITY, f32::max);
        match options.kind {
            CartesianKind::Bar(_) => bar_x_domain(points, min, max),
            _ => expanded_domain(min, max),
        }
    };

    let (mut y_min, mut y_max) = (f32::INFINITY, f32::NEG_INFINITY);
    match options.kind {
        CartesianKind::Bar(BarLayout::Stacked) => {
            for datum in points {
                let positive: f32 = datum
                    .values
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|v| *v > 0.0)
                    .sum();
                let negative: f32 = datum
                    .values
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|v| *v < 0.0)
                    .sum();
                y_min = y_min.min(negative).min(0.0);
                y_max = y_max.max(positive).max(0.0);
            }
        }
        kind => {
            for value in points
                .iter()
                .flat_map(|datum| datum.values.iter().flatten())
            {
                y_min = y_min.min(*value);
                y_max = y_max.max(*value);
            }
            if matches!(kind, CartesianKind::Area { .. } | CartesianKind::Bar(_)) {
                y_min = y_min.min(0.0);
                y_max = y_max.max(0.0);
            }
        }
    }
    let computed_y = if y_min.is_finite() && y_max.is_finite() {
        expanded_domain(y_min, y_max)
    } else {
        AxisDomain::new(-1.0, 1.0)
    };

    Ok(ChartDomain {
        x: options
            .domain
            .x
            .map_or(Ok(computed_x), |domain| domain.validate("x"))?,
        y: options
            .domain
            .y
            .map_or(Ok(computed_y), |domain| domain.validate("y"))?,
    })
}

fn expanded_domain(min: f32, max: f32) -> AxisDomain {
    if min < max {
        AxisDomain::new(min, max)
    } else {
        let padding = (min.abs() * 0.1).max(1.0);
        AxisDomain::new(min - padding, max + padding)
    }
}

fn bar_x_domain(points: &[PreparedDatum], min: f32, max: f32) -> AxisDomain {
    if min == max {
        return AxisDomain::new(min - 0.5, max + 0.5);
    }

    let mut xs: Vec<f32> = points.iter().map(|datum| datum.x).collect();
    xs.sort_by(f32::total_cmp);
    xs.dedup_by(|left, right| *left == *right);
    let gap = xs
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .fold(f32::INFINITY, f32::min);
    let edge = if gap.is_finite() {
        gap * 0.5
    } else {
        (max - min) * 0.5
    };
    AxisDomain::new(min - edge, max + edge)
}

fn marks(
    config: &ChartConfig,
    data: &[PreparedDatum],
    plot: Rectangle,
    domain: ChartDomain,
    kind: CartesianKind,
) -> Vec<CartesianMark> {
    match kind {
        CartesianKind::Line { .. } | CartesianKind::Area { .. } => config
            .series
            .iter()
            .enumerate()
            .filter_map(|(series_index, series)| {
                let points: Vec<_> = data
                    .iter()
                    .filter_map(|datum| {
                        datum.values[series_index].map(|value| PlotPoint {
                            datum_index: datum.source_index,
                            position: Point::new(
                                map_x(datum.x, domain.x, plot),
                                map_y(value, domain.y, plot),
                            ),
                            value,
                        })
                    })
                    .collect();
                if points.is_empty() {
                    None
                } else if matches!(kind, CartesianKind::Line { .. }) {
                    Some(CartesianMark::Line {
                        series_key: series.key.clone(),
                        points,
                    })
                } else {
                    Some(CartesianMark::Area {
                        series_key: series.key.clone(),
                        points,
                        baseline_y: map_y(
                            0.0_f32.clamp(domain.y.min, domain.y.max),
                            domain.y,
                            plot,
                        ),
                    })
                }
            })
            .collect(),
        CartesianKind::Bar(layout) => bar_marks(config, data, plot, domain, layout),
    }
}

fn bar_marks(
    config: &ChartConfig,
    data: &[PreparedDatum],
    plot: Rectangle,
    domain: ChartDomain,
    layout: BarLayout,
) -> Vec<CartesianMark> {
    let mut marks = Vec::new();
    if data.is_empty() {
        return marks;
    }

    let cluster_width = bar_cluster_width(data, plot, domain.x);
    for datum in data {
        let center_x = map_x(datum.x, domain.x, plot);
        match layout {
            BarLayout::Grouped => {
                let count = config.series.len().max(1);
                let width = ((cluster_width - BAR_GAP * (count.saturating_sub(1) as f32))
                    / count as f32)
                    .max(1.0);
                let start_x =
                    center_x - (width * count as f32 + BAR_GAP * (count - 1) as f32) / 2.0;
                for (series_index, series) in config.series.iter().enumerate() {
                    let Some(value) = datum.values[series_index] else {
                        continue;
                    };
                    let zero_y = map_y(0.0_f32.clamp(domain.y.min, domain.y.max), domain.y, plot);
                    let value_y = map_y(value, domain.y, plot);
                    marks.push(CartesianMark::Bar {
                        series_key: series.key.clone(),
                        datum_index: datum.source_index,
                        value,
                        bounds: Rectangle {
                            x: start_x + series_index as f32 * (width + BAR_GAP),
                            y: zero_y.min(value_y),
                            width,
                            height: (zero_y - value_y).abs().max(1.0),
                        },
                    });
                }
            }
            BarLayout::Stacked => {
                let mut positive = 0.0;
                let mut negative = 0.0;
                for (series_index, series) in config.series.iter().enumerate() {
                    let Some(value) = datum.values[series_index] else {
                        continue;
                    };
                    let start = if value >= 0.0 { positive } else { negative };
                    let end = start + value;
                    if value >= 0.0 {
                        positive = end;
                    } else {
                        negative = end;
                    }
                    let start_y = map_y(start, domain.y, plot);
                    let end_y = map_y(end, domain.y, plot);
                    marks.push(CartesianMark::Bar {
                        series_key: series.key.clone(),
                        datum_index: datum.source_index,
                        value,
                        bounds: Rectangle {
                            x: center_x - cluster_width / 2.0,
                            y: start_y.min(end_y),
                            width: cluster_width,
                            height: (start_y - end_y).abs().max(1.0),
                        },
                    });
                }
            }
        }
    }
    marks
}

fn bar_cluster_width(data: &[PreparedDatum], plot: Rectangle, x_domain: AxisDomain) -> f32 {
    let mut xs: Vec<f32> = data
        .iter()
        .map(|datum| map_x(datum.x, x_domain, plot))
        .collect();
    xs.sort_by(f32::total_cmp);
    xs.dedup_by(|left, right| *left == *right);
    let gap = xs
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .fold(plot.width, f32::min);
    (gap * 0.72).clamp(8.0, 64.0)
}

fn map_x(value: f32, domain: AxisDomain, plot: Rectangle) -> f32 {
    plot.x + (value - domain.min) / (domain.max - domain.min) * plot.width
}

fn map_y(value: f32, domain: AxisDomain, plot: Rectangle) -> f32 {
    plot.y + plot.height - (value - domain.min) / (domain.max - domain.min) * plot.height
}

pub struct CartesianChart<'a, Message> {
    config: ChartConfig,
    data: ChartData,
    options: CartesianOptions,
    hovered: Option<ChartHit>,
    on_hover: Option<Rc<dyn Fn(Option<ChartHit>) -> Message + 'a>>,
    width: Length,
    height: Length,
    theme: UiTheme,
}

pub fn cartesian_chart<'a, Message>(
    config: &ChartConfig,
    data: &ChartData,
    theme: &UiTheme,
) -> CartesianChart<'a, Message> {
    CartesianChart {
        config: config.clone(),
        data: data.clone(),
        options: CartesianOptions::default(),
        hovered: None,
        on_hover: None,
        width: Length::Fill,
        height: Length::Fixed(DEFAULT_HEIGHT),
        theme: *theme,
    }
}

impl<'a, Message> CartesianChart<'a, Message>
where
    Message: 'a,
{
    #[must_use]
    pub fn kind(mut self, kind: CartesianKind) -> Self {
        self.options.kind = kind;
        self
    }

    #[must_use]
    pub fn domain(mut self, domain: DomainSpec) -> Self {
        self.options.domain = domain;
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: ChartPadding) -> Self {
        self.options.padding = padding;
        self
    }

    #[must_use]
    pub fn ticks(mut self, count: usize) -> Self {
        self.options.tick_count = count.clamp(2, 10);
        self
    }

    #[must_use]
    pub fn grid(mut self, show: bool) -> Self {
        self.options.show_grid = show;
        self
    }

    #[must_use]
    pub fn hovered(mut self, hovered: Option<ChartHit>) -> Self {
        self.hovered = hovered;
        self
    }

    #[must_use]
    pub fn on_hover(mut self, on_hover: impl Fn(Option<ChartHit>) -> Message + 'a) -> Self {
        self.on_hover = Some(Rc::new(on_hover));
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    fn into_widget(self) -> Stack<'a, Message> {
        let width = self.width;
        let height = self.height;
        let canvas = Canvas::new(CartesianProgram {
            config: self.config,
            data: self.data,
            options: self.options,
            hovered: self.hovered,
            on_hover: self.on_hover,
            theme: self.theme,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        Stack::new()
            .width(width)
            .height(height)
            .clip(true)
            .push(Space::new().width(width).height(height))
            .push(canvas)
    }
}

impl<'a, Message> From<CartesianChart<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(chart: CartesianChart<'a, Message>) -> Self {
        Element::new(chart.into_widget())
    }
}

#[derive(Debug, Default)]
struct CartesianState {
    hovered: Option<ChartHit>,
}

struct CartesianProgram<'a, Message> {
    config: ChartConfig,
    data: ChartData,
    options: CartesianOptions,
    hovered: Option<ChartHit>,
    on_hover: Option<Rc<dyn Fn(Option<ChartHit>) -> Message + 'a>>,
    theme: UiTheme,
}

impl<Message> canvas::Program<Message> for CartesianProgram<'_, Message> {
    type State = CartesianState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let on_hover = self.on_hover.as_ref()?;
        if !matches!(
            event,
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. })
                | canvas::Event::Mouse(mouse::Event::CursorLeft)
        ) {
            return None;
        }

        let local_bounds = Rectangle::with_size(bounds.size());
        let next = cursor.position_in(bounds).and_then(|point| {
            cartesian_geometry(&self.config, &self.data, local_bounds, self.options)
                .ok()
                .and_then(|geometry| geometry.hit_test(point, HIT_RADIUS))
        });
        if next == state.hovered {
            None
        } else {
            state.hovered = next.clone();
            Some(canvas::Action::publish(on_hover(next)))
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        match cartesian_geometry(
            &self.config,
            &self.data,
            Rectangle::with_size(bounds.size()),
            self.options,
        ) {
            Ok(geometry) if !geometry.marks.is_empty() => {
                draw_cartesian(
                    &mut frame,
                    &geometry,
                    &self.config,
                    self.options,
                    self.hovered.as_ref(),
                    &self.theme,
                );
            }
            Ok(geometry) => draw_empty(&mut frame, geometry.report.state, &self.theme),
            Err(_) => draw_error(&mut frame, &self.theme),
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let local_bounds = Rectangle::with_size(bounds.size());
        let hit = cursor.position_in(bounds).and_then(|point| {
            cartesian_geometry(&self.config, &self.data, local_bounds, self.options)
                .ok()
                .and_then(|geometry| geometry.hit_test(point, HIT_RADIUS))
        });
        if self.on_hover.is_some() && hit.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn draw_cartesian(
    frame: &mut canvas::Frame,
    geometry: &CartesianGeometry,
    config: &ChartConfig,
    options: CartesianOptions,
    hovered: Option<&ChartHit>,
    theme: &UiTheme,
) {
    draw_axes(frame, geometry, options, theme);
    // Keep Cartesian meshes in the parent frame. A nested `Frame::with_clip`
    // drops its child mesh under translated/scrolled WGPU canvases; normal
    // domain geometry already stays inside `plot` and the Canvas clips its edge.
    for mark in &geometry.marks {
        let key = match mark {
            CartesianMark::Line { series_key, .. }
            | CartesianMark::Area { series_key, .. }
            | CartesianMark::Bar { series_key, .. } => series_key,
        };
        let Some(series) = config.series(key) else {
            continue;
        };
        let color = series.color.resolve(theme);
        match mark {
            CartesianMark::Line { points, .. } => {
                stroke_points(frame, points, color);
                if points.len() == 1 || matches!(options.kind, CartesianKind::Line { points: true })
                {
                    draw_points(frame, points, color);
                }
            }
            CartesianMark::Area {
                points, baseline_y, ..
            } => {
                if let (Some(first), Some(last)) = (points.first(), points.last()) {
                    let path = Path::new(|path| {
                        path.move_to(Point::new(first.position.x, *baseline_y));
                        for point in points {
                            path.line_to(point.position);
                        }
                        path.line_to(Point::new(last.position.x, *baseline_y));
                        path.close();
                    });
                    frame.fill(&path, alpha(color, 0.16));
                }
                stroke_points(frame, points, color);
                if points.len() == 1 || matches!(options.kind, CartesianKind::Area { points: true })
                {
                    draw_points(frame, points, color);
                }
            }
            CartesianMark::Bar { bounds, .. } => {
                frame.fill_rectangle(bounds.position(), bounds.size(), color);
            }
        }
    }
    if let Some(hit) = hovered {
        draw_active_mark(frame, geometry, config, hit, theme);
    }
}

fn draw_axes(
    frame: &mut canvas::Frame,
    geometry: &CartesianGeometry,
    options: CartesianOptions,
    theme: &UiTheme,
) {
    let ticks = options.tick_count.clamp(2, 10);
    let grid = mix(theme.palette.background, theme.palette.foreground, 0.12);
    let axis = mix(theme.palette.background, theme.palette.foreground, 0.28);
    let label = theme.palette.muted_foreground;
    for index in 0..ticks {
        let ratio = index as f32 / (ticks - 1) as f32;
        let y = geometry.plot.y + geometry.plot.height * ratio;
        if options.show_grid {
            frame.stroke(
                &Path::line(
                    Point::new(geometry.plot.x, y),
                    Point::new(geometry.plot.x + geometry.plot.width, y),
                ),
                Stroke::default().with_color(grid).with_width(GRID_WIDTH),
            );
        }
        let value = geometry.domain.y.max - (geometry.domain.y.max - geometry.domain.y.min) * ratio;
        frame.fill_text(canvas::Text {
            content: format_number(value),
            position: Point::new(geometry.plot.x - 8.0, y),
            color: label,
            size: Pixels(theme.typography.xs),
            align_x: TextAlignment::Right,
            align_y: Vertical::Center,
            ..canvas::Text::default()
        });
    }
    frame.stroke(
        &Path::line(
            Point::new(geometry.plot.x, geometry.plot.y + geometry.plot.height),
            Point::new(
                geometry.plot.x + geometry.plot.width,
                geometry.plot.y + geometry.plot.height,
            ),
        ),
        Stroke::default().with_color(axis).with_width(GRID_WIDTH),
    );

    let stride = geometry.datums.len().div_ceil(6).max(1);
    for (index, datum) in geometry.datums.iter().enumerate() {
        if index % stride != 0 && index + 1 != geometry.datums.len() {
            continue;
        }
        frame.fill_text(canvas::Text {
            content: datum.label.clone(),
            position: Point::new(datum.x, geometry.plot.y + geometry.plot.height + 10.0),
            color: label,
            size: Pixels(theme.typography.xs),
            align_x: TextAlignment::Center,
            align_y: Vertical::Top,
            ..canvas::Text::default()
        });
    }
}

fn stroke_points(frame: &mut canvas::Frame, points: &[PlotPoint], color: Color) {
    if points.len() < 2 {
        return;
    }
    let path = Path::new(|path| {
        path.move_to(points[0].position);
        for point in &points[1..] {
            path.line_to(point.position);
        }
    });
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(color)
            .with_width(LINE_WIDTH)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round),
    );
}

fn draw_points(frame: &mut canvas::Frame, points: &[PlotPoint], color: Color) {
    for point in points {
        frame.fill(&Path::circle(point.position, MARKER_RADIUS), color);
    }
}

fn draw_active_mark(
    frame: &mut canvas::Frame,
    geometry: &CartesianGeometry,
    config: &ChartConfig,
    hit: &ChartHit,
    theme: &UiTheme,
) {
    let Some(series) = config.series(&hit.series_key) else {
        return;
    };
    let color = series.color.resolve(theme);
    if let Some(datum) = geometry
        .datums
        .iter()
        .find(|datum| datum.datum_index == hit.datum_index)
    {
        const DASHES: [f32; 2] = [3.0, 3.0];
        frame.stroke(
            &Path::line(
                Point::new(datum.x, geometry.plot.y),
                Point::new(datum.x, geometry.plot.y + geometry.plot.height),
            ),
            Stroke {
                style: canvas::Style::Solid(alpha(theme.palette.muted_foreground, 0.55)),
                width: 1.0,
                line_dash: canvas::LineDash {
                    segments: &DASHES,
                    offset: 0,
                },
                ..Stroke::default()
            },
        );
    }
    for mark in &geometry.marks {
        match mark {
            CartesianMark::Line { series_key, points }
            | CartesianMark::Area {
                series_key, points, ..
            } if series_key == &hit.series_key => {
                if let Some(point) = points
                    .iter()
                    .find(|point| point.datum_index == hit.datum_index)
                {
                    let circle = Path::circle(point.position, ACTIVE_MARKER_RADIUS);
                    frame.fill(&circle, theme.palette.card);
                    frame.stroke(&circle, Stroke::default().with_color(color).with_width(2.0));
                }
            }
            CartesianMark::Bar {
                series_key,
                datum_index,
                bounds,
                ..
            } if series_key == &hit.series_key && *datum_index == hit.datum_index => {
                frame.stroke_rectangle(
                    bounds.position(),
                    bounds.size(),
                    Stroke::default()
                        .with_color(theme.palette.foreground)
                        .with_width(2.0),
                );
            }
            _ => {}
        }
    }
}

fn draw_empty(frame: &mut canvas::Frame, state: DataState, theme: &UiTheme) {
    let content = if state == DataState::Empty {
        "No chart data"
    } else {
        "No valid chart data"
    };
    draw_centered_text(frame, content, theme.palette.muted_foreground, theme);
}

fn draw_error(frame: &mut canvas::Frame, theme: &UiTheme) {
    draw_centered_text(
        frame,
        "Invalid chart configuration",
        theme.palette.destructive,
        theme,
    );
}

fn draw_centered_text(frame: &mut canvas::Frame, content: &str, color: Color, theme: &UiTheme) {
    frame.fill_text(canvas::Text {
        content: content.to_owned(),
        position: frame.center(),
        color,
        size: Pixels(theme.typography.sm),
        align_x: TextAlignment::Center,
        align_y: Vertical::Center,
        ..canvas::Text::default()
    });
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PieData {
    pub values: BTreeMap<String, f32>,
}

impl PieData {
    pub fn new(values: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PieSliceGeometry {
    pub series_key: String,
    pub value: f32,
    pub start_angle: f32,
    pub end_angle: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PieGeometry {
    pub center: Point,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub slices: Vec<PieSliceGeometry>,
    pub report: DataReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieHit {
    pub series_key: String,
}

impl PieGeometry {
    pub fn hit_test(&self, point: Point) -> Option<PieHit> {
        let delta = point - self.center;
        let distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
        if distance < self.inner_radius || distance > self.outer_radius {
            return None;
        }
        let angle = normalize_angle(delta.y.atan2(delta.x));
        self.slices
            .iter()
            .find(|slice| angle_in_slice(angle, slice.start_angle, slice.end_angle))
            .map(|slice| PieHit {
                series_key: slice.series_key.clone(),
            })
    }
}

pub fn pie_geometry(
    config: &ChartConfig,
    data: &PieData,
    bounds: Rectangle,
    inner_ratio: f32,
) -> Result<PieGeometry, ChartError> {
    config.validate()?;
    if !inner_ratio.is_finite() || !(0.0..1.0).contains(&inner_ratio) {
        return Err(ChartError::InvalidInnerRadius);
    }

    let known: HashSet<&str> = config
        .series
        .iter()
        .map(|series| series.key.as_str())
        .collect();
    let mut report = DataReport {
        unknown_values: data
            .values
            .keys()
            .filter(|key| !known.contains(key.as_str()))
            .count(),
        ..DataReport::default()
    };
    let valid: Vec<_> = config
        .series
        .iter()
        .filter_map(|series| match data.values.get(&series.key).copied() {
            Some(value) if value.is_finite() && value > 0.0 => {
                report.accepted_points += 1;
                Some((series.key.clone(), value))
            }
            Some(_) => {
                report.dropped_values += 1;
                None
            }
            None => None,
        })
        .collect();
    let report = report.finish(data.values.is_empty());
    let total: f32 = valid.iter().map(|(_, value)| value).sum();
    let mut angle = -PI / 2.0;
    let slices = valid
        .into_iter()
        .map(|(series_key, value)| {
            let start_angle = angle;
            angle += value / total * TAU;
            PieSliceGeometry {
                series_key,
                value,
                start_angle,
                end_angle: angle,
            }
        })
        .collect();
    let outer_radius = (bounds.width.min(bounds.height) / 2.0 - 12.0).max(0.0);
    if outer_radius <= 1.0 {
        return Err(ChartError::TooSmall);
    }

    Ok(PieGeometry {
        center: bounds.center(),
        outer_radius,
        inner_radius: outer_radius * inner_ratio,
        slices,
        report,
    })
}

pub struct PieChart<'a, Message> {
    config: ChartConfig,
    data: PieData,
    inner_ratio: f32,
    hovered: Option<PieHit>,
    on_hover: Option<Rc<dyn Fn(Option<PieHit>) -> Message + 'a>>,
    width: Length,
    height: Length,
    theme: UiTheme,
}

pub fn pie_chart<'a, Message>(
    config: &ChartConfig,
    data: &PieData,
    theme: &UiTheme,
) -> PieChart<'a, Message> {
    PieChart {
        config: config.clone(),
        data: data.clone(),
        inner_ratio: 0.0,
        hovered: None,
        on_hover: None,
        width: Length::Fill,
        height: Length::Fixed(DEFAULT_HEIGHT),
        theme: *theme,
    }
}

impl<'a, Message> PieChart<'a, Message>
where
    Message: 'a,
{
    #[must_use]
    pub fn donut(mut self, inner_ratio: f32) -> Self {
        if inner_ratio.is_finite() && (0.0..1.0).contains(&inner_ratio) {
            self.inner_ratio = inner_ratio;
        }
        self
    }

    #[must_use]
    pub fn hovered(mut self, hovered: Option<PieHit>) -> Self {
        self.hovered = hovered;
        self
    }

    #[must_use]
    pub fn on_hover(mut self, on_hover: impl Fn(Option<PieHit>) -> Message + 'a) -> Self {
        self.on_hover = Some(Rc::new(on_hover));
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    fn into_widget(self) -> Stack<'a, Message> {
        let width = self.width;
        let height = self.height;
        let canvas = Canvas::new(PieProgram {
            config: self.config,
            data: self.data,
            inner_ratio: self.inner_ratio,
            hovered: self.hovered,
            on_hover: self.on_hover,
            theme: self.theme,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        Stack::new()
            .width(width)
            .height(height)
            .clip(true)
            .push(Space::new().width(width).height(height))
            .push(canvas)
    }
}

impl<'a, Message> From<PieChart<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(chart: PieChart<'a, Message>) -> Self {
        Element::new(chart.into_widget())
    }
}

#[derive(Debug, Default)]
struct PieState {
    hovered: Option<PieHit>,
}

struct PieProgram<'a, Message> {
    config: ChartConfig,
    data: PieData,
    inner_ratio: f32,
    hovered: Option<PieHit>,
    on_hover: Option<Rc<dyn Fn(Option<PieHit>) -> Message + 'a>>,
    theme: UiTheme,
}

impl<Message> canvas::Program<Message> for PieProgram<'_, Message> {
    type State = PieState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let on_hover = self.on_hover.as_ref()?;
        if !matches!(
            event,
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. })
                | canvas::Event::Mouse(mouse::Event::CursorLeft)
        ) {
            return None;
        }
        let next = cursor.position_in(bounds).and_then(|point| {
            pie_geometry(
                &self.config,
                &self.data,
                Rectangle::with_size(bounds.size()),
                self.inner_ratio,
            )
            .ok()
            .and_then(|geometry| geometry.hit_test(point))
        });
        if next == state.hovered {
            None
        } else {
            state.hovered = next.clone();
            Some(canvas::Action::publish(on_hover(next)))
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        match pie_geometry(
            &self.config,
            &self.data,
            Rectangle::with_size(bounds.size()),
            self.inner_ratio,
        ) {
            Ok(geometry) if !geometry.slices.is_empty() => {
                draw_pie(
                    &mut frame,
                    &geometry,
                    &self.config,
                    self.hovered.as_ref(),
                    &self.theme,
                );
            }
            Ok(geometry) => draw_empty(&mut frame, geometry.report.state, &self.theme),
            Err(_) => draw_error(&mut frame, &self.theme),
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let hit = cursor.position_in(bounds).and_then(|point| {
            pie_geometry(
                &self.config,
                &self.data,
                Rectangle::with_size(bounds.size()),
                self.inner_ratio,
            )
            .ok()
            .and_then(|geometry| geometry.hit_test(point))
        });
        if self.on_hover.is_some() && hit.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn draw_pie(
    frame: &mut canvas::Frame,
    geometry: &PieGeometry,
    config: &ChartConfig,
    hovered: Option<&PieHit>,
    theme: &UiTheme,
) {
    for slice in &geometry.slices {
        let Some(series) = config.series(&slice.series_key) else {
            continue;
        };
        let active = hovered.is_some_and(|hit| hit.series_key == slice.series_key);
        let outer = if active {
            geometry.outer_radius + 3.0
        } else {
            geometry.outer_radius
        };
        let path = pie_path(
            geometry.center,
            outer,
            geometry.inner_radius,
            slice.start_angle,
            slice.end_angle,
        );
        frame.fill(&path, series.color.resolve(theme));
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(theme.palette.card)
                .with_width(2.0)
                .with_line_join(canvas::LineJoin::Round),
        );
    }
}

fn pie_path(center: Point, outer: f32, inner: f32, start: f32, end: f32) -> Path {
    Path::new(|path| {
        path.arc(canvas::path::Arc {
            center,
            radius: outer,
            start_angle: Radians(start),
            end_angle: Radians(end),
        });
        if inner > 0.0 {
            path.line_to(radial_point(center, inner, end));
            for point in reverse_arc_points(center, inner, start, end) {
                path.line_to(point);
            }
        } else {
            path.line_to(center);
        }
        path.close();
    })
}

fn reverse_arc_points(center: Point, radius: f32, start: f32, end: f32) -> Vec<Point> {
    // ponytail: 64 segments per circle; use Bezier arcs if zoomable charts need subpixel curves.
    let segments = (((end - start).abs() / TAU * 64.0).ceil() as usize).max(2);
    (1..=segments)
        .map(|step| {
            let ratio = step as f32 / segments as f32;
            radial_point(center, radius, end + (start - end) * ratio)
        })
        .collect()
}

fn radial_point(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + angle.cos() * radius,
        center.y + angle.sin() * radius,
    )
}

fn normalize_angle(angle: f32) -> f32 {
    (angle + PI / 2.0).rem_euclid(TAU) - PI / 2.0
}

fn angle_in_slice(angle: f32, start: f32, end: f32) -> bool {
    let span = end - start;
    if span >= TAU - f32::EPSILON {
        return true;
    }
    let relative = (angle - start).rem_euclid(TAU);
    relative <= span
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TooltipIndicator {
    #[default]
    Dot,
    Line,
    Dashed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TooltipOptions {
    pub indicator: TooltipIndicator,
    pub label_key: Option<String>,
    pub name_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipEntry {
    pub key: String,
    pub name: String,
    pub value: f32,
    pub color: Color,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipModel {
    pub label: String,
    pub indicator: TooltipIndicator,
    pub entries: Vec<TooltipEntry>,
}

pub fn tooltip_model(
    config: &ChartConfig,
    data: &ChartData,
    hit: &ChartHit,
    options: &TooltipOptions,
    theme: &UiTheme,
) -> Option<TooltipModel> {
    let datum = data.points.get(hit.datum_index)?;
    let label = options
        .label_key
        .as_ref()
        .and_then(|key| datum.metadata.get(key))
        .cloned()
        .unwrap_or_else(|| datum.label.clone());
    let entries = config
        .series
        .iter()
        .filter_map(|series| {
            let value = *datum.values.get(&series.key)?;
            if !value.is_finite() {
                return None;
            }
            let name = options
                .name_key
                .as_ref()
                .and_then(|key| datum.metadata.get(&format!("{key}:{}", series.key)))
                .cloned()
                .unwrap_or_else(|| series.label.clone());
            Some(TooltipEntry {
                key: series.key.clone(),
                name,
                value,
                color: series.color.resolve(theme),
                active: series.key == hit.series_key,
            })
        })
        .collect();
    Some(TooltipModel {
        label,
        indicator: options.indicator,
        entries,
    })
}

pub fn pie_tooltip_model(
    config: &ChartConfig,
    data: &PieData,
    hit: &PieHit,
    indicator: TooltipIndicator,
    theme: &UiTheme,
) -> Option<TooltipModel> {
    let series = config.series(&hit.series_key)?;
    let value = *data.values.get(&hit.series_key)?;
    value.is_finite().then(|| TooltipModel {
        label: series.label.clone(),
        indicator,
        entries: vec![TooltipEntry {
            key: series.key.clone(),
            name: series.label.clone(),
            value,
            color: series.color.resolve(theme),
            active: true,
        }],
    })
}

pub fn tooltip_content<'a, Message>(model: &TooltipModel, theme: &UiTheme) -> Container<'a, Message>
where
    Message: 'a,
{
    let mut content = Column::new().spacing(theme.spacing.sm).push(
        text(model.label.clone())
            .size(theme.typography.sm)
            .color(theme.palette.foreground),
    );
    for entry in &model.entries {
        content = content.push(
            row![
                tooltip_indicator(entry.color, model.indicator),
                text(entry.name.clone())
                    .size(theme.typography.sm)
                    .color(theme.palette.muted_foreground)
                    .width(Length::Fill),
                text(format_number(entry.value))
                    .size(theme.typography.sm)
                    .color(theme.palette.foreground)
                    .align_x(Horizontal::Right),
            ]
            .spacing(theme.spacing.sm)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        );
    }
    let theme = *theme;
    container(content)
        .width(Length::Fixed(180.0))
        .padding(theme.spacing.md)
        .style(move |_| tooltip_style(&theme))
}

fn tooltip_indicator<'a, Message>(color: Color, indicator: TooltipIndicator) -> Element<'a, Message>
where
    Message: 'a,
{
    let block = move |width, height| {
        container(Space::new().width(width).height(height)).style(move |_| {
            iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 2.0.into(),
                    ..Border::default()
                },
                ..Default::default()
            }
        })
    };
    match indicator {
        TooltipIndicator::Dot => block(8.0, 8.0).into(),
        TooltipIndicator::Line => block(2.0, 12.0).into(),
        TooltipIndicator::Dashed => Column::new()
            .push(block(2.0, 3.0))
            .push(block(2.0, 3.0))
            .spacing(2.0)
            .height(Length::Fixed(8.0))
            .into(),
    }
}

pub fn tooltip_style(theme: &UiTheme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.popover)),
        text_color: Some(theme.palette.popover_foreground),
        border: Border {
            color: theme.palette.border,
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        shadow: Shadow {
            color: alpha(
                Color::BLACK,
                if luminance(theme.palette.background) < 0.5 {
                    0.35
                } else {
                    0.12
                },
            ),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegendEntry {
    pub key: String,
    pub label: String,
    pub color: Color,
}

pub fn legend_entries(config: &ChartConfig, theme: &UiTheme) -> Vec<LegendEntry> {
    config
        .series
        .iter()
        .map(|series| LegendEntry {
            key: series.key.clone(),
            label: series.label.clone(),
            color: series.color.resolve(theme),
        })
        .collect()
}

pub fn legend_content<'a, Message>(config: &ChartConfig, theme: &UiTheme) -> Row<'a, Message>
where
    Message: 'a,
{
    config.series.iter().fold(
        Row::new()
            .spacing(theme.spacing.lg)
            .align_y(Alignment::Center),
        |legend, series| {
            let color = series.color.resolve(theme);
            legend.push(
                row![
                    container(Space::new().width(8.0).height(8.0)).style(move |_| {
                        iced::widget::container::Style {
                            background: Some(Background::Color(color)),
                            border: Border {
                                radius: 2.0.into(),
                                ..Border::default()
                            },
                            ..Default::default()
                        }
                    }),
                    text(series.label.clone())
                        .size(theme.typography.sm)
                        .color(theme.palette.muted_foreground),
                ]
                .spacing(theme.spacing.sm)
                .align_y(Alignment::Center),
            )
        },
    )
}

/// Visible text equivalent for a canvas chart. Pair every chart with this (or
/// an equivalent caller-owned table): iced Canvas does not expose chart marks
/// as a semantic accessibility tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartCompanion {
    pub caption: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn companion_model(
    caption: impl Into<String>,
    config: &ChartConfig,
    data: &ChartData,
) -> Result<ChartCompanion, ChartError> {
    config.validate()?;
    let mut headers = vec!["Label".to_owned()];
    headers.extend(config.series.iter().map(|series| series.label.clone()));
    let rows = data
        .points
        .iter()
        .map(|datum| {
            let mut row = vec![datum.label.clone()];
            row.extend(config.series.iter().map(|series| {
                datum
                    .values
                    .get(&series.key)
                    .filter(|value| value.is_finite())
                    .map_or_else(|| "—".to_owned(), |value| format_number(*value))
            }));
            row
        })
        .collect();
    Ok(ChartCompanion {
        caption: caption.into(),
        headers,
        rows,
    })
}

pub fn companion_content<'a, Message>(
    model: &ChartCompanion,
    theme: &UiTheme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let mut body = Column::new().spacing(0).push(
        text(model.caption.clone())
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
    );
    body = body.push(companion_row(&model.headers, true, theme));
    for values in &model.rows {
        body = body.push(companion_row(values, false, theme));
    }
    let theme = *theme;
    container(body.spacing(theme.spacing.sm))
        .width(Length::Fill)
        .padding(theme.spacing.md)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(theme.palette.card)),
            text_color: Some(theme.palette.card_foreground),
            border: Border {
                color: theme.palette.border,
                width: 1.0,
                radius: theme.radius.lg.into(),
            },
            ..Default::default()
        })
}

fn companion_row<'a, Message>(values: &[String], header: bool, theme: &UiTheme) -> Row<'a, Message>
where
    Message: 'a,
{
    values.iter().enumerate().fold(
        Row::new().spacing(theme.spacing.md).width(Length::Fill),
        |row, (index, value)| {
            row.push(
                text(value.clone())
                    .size(theme.typography.sm)
                    .color(if header {
                        theme.palette.muted_foreground
                    } else {
                        theme.palette.foreground
                    })
                    .align_x(if index == 0 {
                        Horizontal::Left
                    } else {
                        Horizontal::Right
                    })
                    .width(Length::FillPortion(if index == 0 { 2 } else { 1 })),
            )
        },
    )
}

fn format_number(value: f32) -> String {
    if value.abs() >= 1000.0 || value.fract().abs() < 0.0001 {
        format!("{value:.0}")
    } else {
        let formatted = format!("{value:.2}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

fn luminance(color: Color) -> f32 {
    let channel = |value: f32| {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};
    use iced::Size;

    fn config() -> ChartConfig {
        ChartConfig::new([
            SeriesConfig::new("desktop", "Desktop", ChartColor::Ring),
            SeriesConfig::new(
                "mobile",
                "Mobile",
                ChartColor::LightDark {
                    light: Color::from_rgb(0.1, 0.3, 0.8),
                    dark: Color::from_rgb(0.5, 0.7, 1.0),
                },
            ),
        ])
    }

    fn data() -> ChartData {
        ChartData::new([
            ChartDatum::new(0.0, "Jan")
                .with_value("desktop", 20.0)
                .with_value("mobile", 10.0),
            ChartDatum::new(1.0, "Feb")
                .with_value("desktop", 30.0)
                .with_value("mobile", -5.0),
        ])
    }

    fn bounds() -> Rectangle {
        Rectangle::new(Point::ORIGIN, Size::new(400.0, 240.0))
    }

    #[test]
    fn config_rejects_missing_and_duplicate_stable_keys() {
        assert_eq!(
            ChartConfig::default().validate(),
            Err(ChartError::EmptyConfig)
        );
        assert_eq!(
            ChartConfig::new([SeriesConfig::new("", "Empty", ChartColor::Primary)]).validate(),
            Err(ChartError::EmptySeriesKey)
        );
        assert_eq!(
            ChartConfig::new([
                SeriesConfig::new("same", "One", ChartColor::Primary),
                SeriesConfig::new("same", "Two", ChartColor::Ring),
            ])
            .validate(),
            Err(ChartError::DuplicateSeriesKey("same".to_owned()))
        );
    }

    #[test]
    fn line_and_bar_domains_are_stable_and_overridable() {
        let line =
            cartesian_geometry(&config(), &data(), bounds(), CartesianOptions::default()).unwrap();
        assert_eq!(line.domain.x, AxisDomain::new(0.0, 1.0));
        assert_eq!(line.domain.y, AxisDomain::new(-5.0, 30.0));

        let bars = cartesian_geometry(
            &config(),
            &data(),
            bounds(),
            CartesianOptions {
                kind: CartesianKind::Bar(BarLayout::Grouped),
                domain: DomainSpec {
                    x: None,
                    y: Some(AxisDomain::new(-10.0, 40.0)),
                },
                ..CartesianOptions::default()
            },
        )
        .unwrap();
        assert_eq!(bars.domain.x, AxisDomain::new(-0.5, 1.5));
        assert_eq!(bars.domain.y, AxisDomain::new(-10.0, 40.0));
    }

    #[test]
    fn grouped_bars_do_not_overlap_and_stacks_share_an_edge() {
        let grouped = cartesian_geometry(
            &config(),
            &data(),
            bounds(),
            CartesianOptions {
                kind: CartesianKind::Bar(BarLayout::Grouped),
                ..CartesianOptions::default()
            },
        )
        .unwrap();
        let group: Vec<_> = grouped
            .marks
            .iter()
            .filter_map(|mark| match mark {
                CartesianMark::Bar {
                    datum_index: 0,
                    bounds,
                    ..
                } => Some(*bounds),
                _ => None,
            })
            .collect();
        assert_eq!(group.len(), 2);
        assert!(group[0].x + group[0].width < group[1].x);

        let stacked = cartesian_geometry(
            &config(),
            &data(),
            bounds(),
            CartesianOptions {
                kind: CartesianKind::Bar(BarLayout::Stacked),
                ..CartesianOptions::default()
            },
        )
        .unwrap();
        let positive: Vec<_> = stacked
            .marks
            .iter()
            .filter_map(|mark| match mark {
                CartesianMark::Bar {
                    datum_index: 0,
                    bounds,
                    ..
                } => Some(*bounds),
                _ => None,
            })
            .collect();
        assert_eq!(positive.len(), 2);
        assert!((positive[0].y - (positive[1].y + positive[1].height)).abs() < 0.01);
    }

    #[test]
    fn point_and_bar_hit_testing_returns_stable_source_identity() {
        let line =
            cartesian_geometry(&config(), &data(), bounds(), CartesianOptions::default()).unwrap();
        let point = match &line.marks[0] {
            CartesianMark::Line { points, .. } => points[0].position,
            _ => unreachable!(),
        };
        assert_eq!(
            line.hit_test(point, 1.0),
            Some(ChartHit {
                datum_index: 0,
                series_key: "desktop".to_owned(),
            })
        );

        let bars = cartesian_geometry(
            &config(),
            &data(),
            bounds(),
            CartesianOptions {
                kind: CartesianKind::Bar(BarLayout::Grouped),
                ..CartesianOptions::default()
            },
        )
        .unwrap();
        let (key, index, center) = match &bars.marks[0] {
            CartesianMark::Bar {
                series_key,
                datum_index,
                bounds,
                ..
            } => (series_key.clone(), *datum_index, bounds.center()),
            _ => unreachable!(),
        };
        assert_eq!(
            bars.hit_test(center, 0.0),
            Some(ChartHit {
                datum_index: index,
                series_key: key
            })
        );
    }

    #[test]
    fn cartesian_geometry_uses_parent_frame_coordinates() {
        let frame = bounds();
        let geometry =
            cartesian_geometry(&config(), &data(), frame, CartesianOptions::default()).unwrap();

        assert!(frame.contains(geometry.plot.position()));
        assert!(frame.contains(Point::new(
            geometry.plot.x + geometry.plot.width,
            geometry.plot.y + geometry.plot.height,
        )));
        let in_plot = |point: Point| {
            point.x >= geometry.plot.x
                && point.x <= geometry.plot.x + geometry.plot.width
                && point.y >= geometry.plot.y
                && point.y <= geometry.plot.y + geometry.plot.height
        };
        for mark in &geometry.marks {
            match mark {
                CartesianMark::Line { points, .. } | CartesianMark::Area { points, .. } => {
                    assert!(points.iter().all(|point| in_plot(point.position)));
                }
                CartesianMark::Bar { bounds, .. } => {
                    assert!(in_plot(bounds.position()));
                    assert!(in_plot(Point::new(
                        bounds.x + bounds.width,
                        bounds.y + bounds.height,
                    )));
                }
            }
        }
    }

    #[test]
    fn invalid_values_are_reported_and_never_create_geometry() {
        let invalid = ChartData::new([
            ChartDatum::new(f32::NAN, "bad x").with_value("desktop", 1.0),
            ChartDatum::new(1.0, "bad y").with_value("desktop", f32::INFINITY),
        ]);
        let geometry =
            cartesian_geometry(&config(), &invalid, bounds(), CartesianOptions::default()).unwrap();
        assert_eq!(geometry.report.state, DataState::Invalid);
        assert_eq!(geometry.report.dropped_points, 2);
        assert_eq!(geometry.report.dropped_values, 1);
        assert!(geometry.marks.is_empty());
        assert_eq!(
            cartesian_geometry(
                &config(),
                &data(),
                bounds(),
                CartesianOptions {
                    domain: DomainSpec {
                        x: Some(AxisDomain::new(1.0, 1.0)),
                        y: None,
                    },
                    ..CartesianOptions::default()
                },
            ),
            Err(ChartError::InvalidDomain("x"))
        );
    }

    #[test]
    fn semantic_and_light_dark_colors_resolve_in_both_themes() {
        let entries_light = legend_entries(&config(), &LIGHT);
        let entries_dark = legend_entries(&config(), &DARK);
        assert_eq!(entries_light[0].color, LIGHT.palette.ring);
        assert_eq!(entries_dark[0].color, DARK.palette.ring);
        assert_eq!(entries_light[1].color, Color::from_rgb(0.1, 0.3, 0.8));
        assert_eq!(entries_dark[1].color, Color::from_rgb(0.5, 0.7, 1.0));
    }

    #[test]
    fn tooltip_supports_indicator_and_label_name_overrides() {
        let data = ChartData::new([ChartDatum::new(0.0, "January")
            .with_value("desktop", 12.0)
            .with_value("mobile", 8.0)
            .with_metadata("month_short", "Jan")
            .with_series_name("device", "desktop", "Desk")]);
        let model = tooltip_model(
            &config(),
            &data,
            &ChartHit {
                datum_index: 0,
                series_key: "desktop".to_owned(),
            },
            &TooltipOptions {
                indicator: TooltipIndicator::Dashed,
                label_key: Some("month_short".to_owned()),
                name_key: Some("device".to_owned()),
            },
            &LIGHT,
        )
        .unwrap();
        assert_eq!(model.label, "Jan");
        assert_eq!(model.indicator, TooltipIndicator::Dashed);
        assert_eq!(model.entries[0].name, "Desk");
        assert!(model.entries[0].active);
        assert_eq!(model.entries[1].name, "Mobile");
    }

    #[test]
    fn donut_geometry_and_hit_testing_respect_the_hole() {
        let pie = PieData::new([("desktop", 3.0), ("mobile", 1.0)]);
        let geometry = pie_geometry(&config(), &pie, bounds(), 0.5).unwrap();
        assert_eq!(geometry.slices.len(), 2);
        assert!(
            (geometry.slices[0].end_angle - geometry.slices[0].start_angle - TAU * 0.75).abs()
                < 0.001
        );
        assert_eq!(geometry.hit_test(geometry.center), None);
        let point = radial_point(
            geometry.center,
            (geometry.inner_radius + geometry.outer_radius) / 2.0,
            geometry.slices[0].start_angle + 0.1,
        );
        assert_eq!(
            geometry.hit_test(point),
            Some(PieHit {
                series_key: "desktop".to_owned(),
            })
        );
        assert_eq!(
            pie_geometry(&config(), &pie, bounds(), 1.0),
            Err(ChartError::InvalidInnerRadius)
        );

        let inner = reverse_arc_points(Point::ORIGIN, 10.0, 0.0, PI / 2.0);
        assert!((inner.last().unwrap().x - 10.0).abs() < 0.001);
        assert!(inner.last().unwrap().y.abs() < 0.001);
    }

    #[test]
    fn companion_keeps_missing_values_visible_instead_of_hiding_them() {
        let missing = ChartData::new([ChartDatum::new(0.0, "Jan").with_value("desktop", 42.0)]);
        let companion = companion_model("Visitors by device", &config(), &missing).unwrap();
        assert_eq!(companion.headers, ["Label", "Desktop", "Mobile"]);
        assert_eq!(companion.rows[0], ["Jan", "42", "—"]);
    }
}
