//! Interactive candlestick chart for time-series market data.
//!
//! Grid, candles, volume, and axes render into a cached layer that is only
//! rebuilt when the data or the visible range changes; the crosshair and its
//! axis tags draw on a per-frame overlay. The wheel zooms around the cursor,
//! dragging pans, and the price scale follows the visible candles.

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

use super::theme::{Theme as UiTheme, alpha};
use iced::advanced::text::Alignment as TextAlignment;
use iced::alignment::Vertical;
use iced::mouse;
use iced::widget::Canvas;
use iced::widget::canvas::{self, Path};
use iced::{Color, Element, Length, Pixels, Point, Rectangle, Size};

const DEFAULT_HEIGHT: f32 = 320.0;
const DEFAULT_BARS: usize = 120;
const PRICE_AXIS_WIDTH: f32 = 64.0;
const TIME_AXIS_HEIGHT: f32 = 22.0;
const VOLUME_RATIO: f32 = 0.2;
const BODY_RATIO: f32 = 0.72;
const MIN_SPAN: f64 = 5.0;
const MIN_EDGE_BARS: f64 = 2.0;
const RIGHT_MARGIN_RATIO: f64 = 0.05;
const PRICE_PAD_RATIO: f64 = 0.05;
const ZOOM_PER_LINE: f64 = 1.12;
const WHEEL_PIXELS_PER_LINE: f32 = 40.0;
const PRICE_TICK_TARGET: usize = 6;
const TIME_TICK_TARGET: usize = 6;
const TAG_HEIGHT: f32 = 16.0;
const DASH: [f32; 2] = [3.0, 3.0];

/// One OHLCV bar. `ts` is unix seconds; candles must be sorted ascending.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// The candle under the cursor, reported through [`CandleChart::on_hover`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleHit {
    pub index: i64,
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl CandleHit {
    fn new(index: usize, candle: &Candle) -> Self {
        Self {
            index: index as i64,
            ts: candle.ts,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            volume: candle.volume,
        }
    }
}

/// Visible range in fractional candle-index space: candle `i` spans
/// `i - 0.5 ..= i + 0.5`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Viewport {
    from: f64,
    to: f64,
}

impl Viewport {
    fn initial(len: usize, bars: usize) -> Self {
        // `clamped` enforces the MIN_SPAN floor, so short data may show fewer
        // than MIN_SPAN candles here without a degenerate span.
        let shown = bars.min(len).max(1);
        let from = len as f64 - shown as f64 - 0.5;
        let to = from + shown as f64 * (1.0 + RIGHT_MARGIN_RATIO);
        Self { from, to }.clamped(len)
    }

    fn span(self) -> f64 {
        self.to - self.from
    }

    /// Enforces the span limits and keeps at least [`MIN_EDGE_BARS`] candles
    /// on screen when panned past either end of the data.
    fn clamped(self, len: usize) -> Self {
        let n = len.max(1) as f64;
        let span = self.span().clamp(MIN_SPAN, (n * 2.0).max(MIN_SPAN));
        let mut from = self.from;
        let min_to = MIN_EDGE_BARS - 0.5;
        let max_from = n - MIN_EDGE_BARS - 0.5;
        if from + span < min_to {
            from = min_to - span;
        }
        if from > max_from {
            from = max_from;
        }
        Self {
            from,
            to: from + span,
        }
    }

    /// Scales the span by `factor`, keeping `anchor` at the same position.
    fn zoom(self, anchor: f64, factor: f64, len: usize) -> Self {
        Self {
            from: anchor - (anchor - self.from) * factor,
            to: anchor + (self.to - anchor) * factor,
        }
        .clamped(len)
    }

    fn pan(self, bars: f64, len: usize) -> Self {
        Self {
            from: self.from + bars,
            to: self.to + bars,
        }
        .clamped(len)
    }

    /// The candle index nearest to canvas x, unbounded by the data length.
    fn index_near(self, chrome: Chrome, x: f32) -> Option<usize> {
        let index = chrome.index_at(self, x).round();
        (index >= 0.0 && index.is_finite()).then_some(index as usize)
    }
}

/// When candles were appended while the right edge of the view was at (or
/// past) the previous last candle, shift the view right so it keeps
/// following the tape; a view scrolled into history stays put.
fn follow_appended(viewport: Viewport, seen_len: usize, len: usize) -> Viewport {
    let appended = len.saturating_sub(seen_len);
    if appended == 0 || seen_len == 0 {
        return viewport;
    }
    if viewport.to >= seen_len as f64 - 1.5 {
        viewport.pan(appended as f64, len)
    } else {
        viewport
    }
}

/// Bottom-right chip that resumes following the latest candle.
fn latest_chip(plot: Rectangle) -> Rectangle {
    Rectangle {
        x: plot.x + plot.width - 82.0,
        y: plot.y + plot.height - 30.0,
        width: 72.0,
        height: 20.0,
    }
}

fn chip_visible(viewport: Option<Viewport>, len: usize) -> bool {
    viewport.is_some_and(|viewport| viewport.to < len as f64 - 0.5)
}

fn visible_indices(viewport: Viewport, len: usize) -> Range<usize> {
    let start = ((viewport.from - 0.5).ceil() as i64).clamp(0, len as i64) as usize;
    let end = (((viewport.to + 0.5).floor() as i64) + 1).clamp(start as i64, len as i64) as usize;
    start..end
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PriceScale {
    lo: f64,
    hi: f64,
}

impl PriceScale {
    fn y(self, plot: Rectangle, price: f64) -> f32 {
        let ratio = ((self.hi - price) / (self.hi - self.lo)) as f32;
        plot.y + plot.height * ratio
    }

    fn price_at(self, plot: Rectangle, y: f32) -> f64 {
        let ratio = f64::from((y - plot.y) / plot.height);
        self.hi - (self.hi - self.lo) * ratio
    }
}

fn autoscale(candles: &[Candle], range: Range<usize>) -> PriceScale {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for candle in &candles[range] {
        lo = lo.min(candle.low);
        hi = hi.max(candle.high);
    }
    if !lo.is_finite() || !hi.is_finite() {
        return PriceScale { lo: 0.0, hi: 1.0 };
    }
    let pad = if hi > lo {
        // The floor keeps ulp-thin ranges from collapsing the scale.
        ((hi - lo) * PRICE_PAD_RATIO).max(hi.abs().max(1.0) * 1e-9)
    } else {
        hi.abs().max(1.0) * 0.01
    };
    PriceScale {
        lo: lo - pad,
        hi: hi + pad,
    }
}

/// One pixel column of aggregated candles: the union of every candle whose
/// center maps into that column (M4-style min/max plus first/last).
#[derive(Debug, Clone, Copy, PartialEq)]
struct ColumnAgg {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// Folds the visible candles into per-pixel columns once candles are narrower
/// than a pixel, bounding tessellation at O(plot width) regardless of how many
/// candles are visible. Returns `None` while candles are at least a pixel wide
/// — the per-candle path is exact and cheap there.
fn aggregate_columns(
    candles: &[Candle],
    range: Range<usize>,
    viewport: Viewport,
    chrome: Chrome,
) -> Option<Vec<Option<ColumnAgg>>> {
    let plot = chrome.plot;
    let bar_width = plot.width / viewport.span() as f32;
    if bar_width >= 1.0 || plot.width < 1.0 {
        return None;
    }
    let mut columns: Vec<Option<ColumnAgg>> = vec![None; plot.width.ceil() as usize];
    for index in range {
        let candle = &candles[index];
        let column = (chrome.x(viewport, index as f64) - plot.x).floor();
        if column < 0.0 || column >= columns.len() as f32 {
            continue;
        }
        match &mut columns[column as usize] {
            Some(agg) => {
                agg.high = agg.high.max(candle.high);
                agg.low = agg.low.min(candle.low);
                agg.close = candle.close;
                agg.volume += candle.volume;
            }
            slot => {
                *slot = Some(ColumnAgg {
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: candle.volume,
                });
            }
        }
    }
    Some(columns)
}

/// Plot area: the canvas minus the price axis strip on the right and the
/// time axis strip at the bottom.
#[derive(Debug, Clone, Copy)]
struct Chrome {
    plot: Rectangle,
}

fn chrome(size: Size) -> Chrome {
    Chrome {
        plot: Rectangle {
            x: 0.0,
            y: 0.0,
            width: (size.width - PRICE_AXIS_WIDTH).max(0.0),
            height: (size.height - TIME_AXIS_HEIGHT).max(0.0),
        },
    }
}

impl Chrome {
    fn x(self, viewport: Viewport, index: f64) -> f32 {
        let ratio = ((index - viewport.from) / viewport.span()) as f32;
        self.plot.x + self.plot.width * ratio
    }

    fn index_at(self, viewport: Viewport, x: f32) -> f64 {
        let ratio = f64::from((x - self.plot.x) / self.plot.width);
        viewport.from + viewport.span() * ratio
    }
}

/// Per-frame axis metadata shared by the cached and overlay layers.
#[derive(Clone)]
struct Axes {
    ticks: Vec<f64>,
    precision: usize,
    /// `(x, ts)` of each time tick.
    time_ticks: Vec<(f32, i64)>,
    time_step_secs: i64,
    last_close_y: Option<f32>,
}

/// Everything one frame's drawing shares.
struct FrameCtx<'a> {
    candles: &'a [Candle],
    chrome: Chrome,
    viewport: Viewport,
    scale: PriceScale,
    axes: &'a Axes,
}

/// Median spacing between the candles in `range`, for time-label granularity.
fn time_step_secs(candles: &[Candle], range: Range<usize>) -> i64 {
    let candles = &candles[range];
    let mut deltas: Vec<i64> = candles
        .windows(2)
        .map(|pair| (pair[1].ts - pair[0].ts).max(1))
        .collect();
    if deltas.is_empty() {
        return 86_400;
    }
    let middle = deltas.len() / 2;
    *deltas.select_nth_unstable(middle).1
}

fn nice_step(raw: f64) -> f64 {
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let magnitude = 10f64.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    let nice = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * magnitude
}

fn price_ticks(scale: PriceScale, target: usize) -> (Vec<f64>, f64) {
    let range = scale.hi - scale.lo;
    if !(range.is_finite() && range > 0.0) {
        return (Vec::new(), 1.0);
    }
    let step = nice_step(range / target.max(2) as f64);
    let mut ticks = Vec::new();
    let mut value = (scale.lo / step).ceil() * step;
    while value <= scale.hi {
        ticks.push(value);
        let next = value + step;
        // A step below one ulp of `value` would never advance the loop.
        if next == value {
            break;
        }
        value = next;
    }
    (ticks, step)
}

fn decimals(step: f64) -> usize {
    if step >= 1.0 {
        0
    } else {
        (-step.log10().floor()) as usize
    }
    .min(8)
}

/// Formats a price with a fixed number of decimals, e.g. for hover readouts.
pub fn format_price(value: f64, decimals: usize) -> String {
    let raw = format!("{value:.decimals$}");
    let (sign, digits) = raw
        .strip_prefix('-')
        .map_or(("", raw.as_str()), |rest| ("-", rest));
    let (integer, fraction) = digits
        .split_once('.')
        .map_or((digits, None), |(integer, fraction)| {
            (integer, Some(fraction))
        });
    let mut grouped = String::with_capacity(raw.len() + integer.len() / 3);
    grouped.push_str(sign);
    for (offset, digit) in integer.chars().enumerate() {
        if offset > 0 && (integer.len() - offset) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    if let Some(fraction) = fraction {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    grouped
}

/// Formats a volume compactly: `950`, `12.5K`, `3.4M`, `2.1B`.
pub fn format_volume(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude >= 1e9 {
        format!("{:.1}B", value / 1e9)
    } else if magnitude >= 1e6 {
        format!("{:.1}M", value / 1e6)
    } else if magnitude >= 1e3 {
        format!("{:.1}K", value / 1e3)
    } else {
        format!("{value:.0}")
    }
}

/// Days since 1970-01-01 to a `(year, month, day)` civil date
/// (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Formats a unix timestamp at a granularity matching the spacing between
/// time-axis ticks.
fn format_ts(ts: i64, step_secs: i64) -> String {
    let (year, month, day) = civil_from_days(ts.div_euclid(86_400));
    if step_secs >= 28 * 86_400 {
        format!("{year}-{month:02}")
    } else if step_secs >= 86_400 {
        format!("{month:02}-{day:02}")
    } else {
        let secs = ts.rem_euclid(86_400);
        format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
    }
}

fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x0000_0100_0000_01b3)
}

fn mix_color(hash: u64, color: Color) -> u64 {
    let rg = (u64::from(color.r.to_bits()) << 32) | u64::from(color.g.to_bits());
    let ba = (u64::from(color.b.to_bits()) << 32) | u64::from(color.a.to_bits());
    mix(mix(hash, rg), ba)
}

// ponytail: samples the endpoints only, so an in-place edit of a middle candle
// that keeps len and both ends identical is missed; hash every candle if data
// ever mutates that way.
fn fingerprint(
    candles: &[Candle],
    viewport: Viewport,
    size: Size,
    palette: &super::theme::Palette,
) -> u64 {
    let mut hash = mix(0xcbf2_9ce4_8422_2325, candles.len() as u64);
    hash = mix(
        hash,
        (u64::from(size.width.to_bits()) << 32) | u64::from(size.height.to_bits()),
    );
    if let Some(first) = candles.first() {
        hash = mix(hash, first.ts as u64);
    }
    if let Some(last) = candles.last() {
        hash = mix(hash, last.ts as u64);
        hash = mix(hash, last.high.to_bits());
        hash = mix(hash, last.low.to_bits());
        hash = mix(hash, last.close.to_bits());
        hash = mix(hash, last.volume.to_bits());
    }
    hash = mix(hash, viewport.from.to_bits());
    hash = mix(hash, viewport.to.to_bits());
    // The cached layer paints with these palette colors, so a theme change
    // must invalidate it.
    for color in [
        palette.border,
        palette.success,
        palette.destructive,
        palette.background,
        palette.muted_foreground,
        palette.foreground,
    ] {
        hash = mix_color(hash, color);
    }
    hash
}

/// Chart data shared with a live producer. The chart locks it briefly per
/// frame, so a feed mutates candles in place and no copy ever crosses the
/// view boundary.
pub type SharedCandles = std::sync::Arc<std::sync::Mutex<Vec<Candle>>>;

enum Data<'a> {
    Borrowed(&'a [Candle]),
    Shared(SharedCandles),
}

impl Data<'_> {
    fn with<R>(&self, read: impl FnOnce(&[Candle]) -> R) -> R {
        match self {
            Data::Borrowed(candles) => read(candles),
            Data::Shared(shared) => read(
                &shared
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            ),
        }
    }
}

pub struct CandleChart<'a, Message> {
    data: Data<'a>,
    theme: UiTheme,
    on_hover: Option<Rc<dyn Fn(Option<CandleHit>) -> Message + 'a>>,
    width: Length,
    height: Length,
    initial_bars: usize,
}

pub fn candle_chart<'a, Message>(
    candles: &'a [Candle],
    theme: &UiTheme,
) -> CandleChart<'a, Message> {
    with_data(Data::Borrowed(candles), theme)
}

/// A chart over [`SharedCandles`]: the returned element owns a handle, so it
/// can outlive the caller (`Element<'static>`) while a feed keeps ticking
/// the same data.
pub fn candle_chart_shared<Message>(
    candles: SharedCandles,
    theme: &UiTheme,
) -> CandleChart<'static, Message> {
    with_data(Data::Shared(candles), theme)
}

fn with_data<'a, Message>(data: Data<'a>, theme: &UiTheme) -> CandleChart<'a, Message> {
    CandleChart {
        data,
        theme: *theme,
        on_hover: None,
        width: Length::Fill,
        height: Length::Fixed(DEFAULT_HEIGHT),
        initial_bars: DEFAULT_BARS,
    }
}

impl<'a, Message> CandleChart<'a, Message> {
    #[must_use]
    pub fn on_hover(mut self, on_hover: impl Fn(Option<CandleHit>) -> Message + 'a) -> Self {
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

    /// How many candles the chart shows before any zoom or pan.
    #[must_use]
    pub fn initial_bars(mut self, bars: usize) -> Self {
        self.initial_bars = bars.max(MIN_SPAN as usize);
        self
    }
}

impl<'a, Message> From<CandleChart<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(chart: CandleChart<'a, Message>) -> Self {
        let width = chart.width;
        let height = chart.height;
        Element::new(
            Canvas::new(CandleProgram {
                data: chart.data,
                theme: chart.theme,
                on_hover: chart.on_hover,
                initial_bars: chart.initial_bars,
            })
            .width(width)
            .height(height),
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    x: f32,
    viewport: Viewport,
}

/// Everything derived from (data, viewport, size, theme), memoized under the
/// same fingerprint that keys the cached geometry layer so cached frames do
/// no O(visible candles) work.
struct DerivedFrame {
    stamp: u64,
    scale: PriceScale,
    axes: Axes,
}

pub struct CandleState {
    viewport: Cell<Option<Viewport>>,
    /// Data length at the last draw, to detect appends for auto-follow.
    seen_len: Cell<usize>,
    drag: Option<Drag>,
    hovered: Option<usize>,
    layers: canvas::Cache,
    derived: RefCell<Option<DerivedFrame>>,
}

impl Default for CandleState {
    fn default() -> Self {
        Self {
            viewport: Cell::new(None),
            seen_len: Cell::new(0),
            drag: None,
            hovered: None,
            layers: canvas::Cache::new(),
            derived: RefCell::new(None),
        }
    }
}

struct CandleProgram<'a, Message> {
    data: Data<'a>,
    theme: UiTheme,
    on_hover: Option<Rc<dyn Fn(Option<CandleHit>) -> Message + 'a>>,
    initial_bars: usize,
}

impl<Message> CandleProgram<'_, Message> {
    fn viewport(&self, candles: &[Candle], state: &CandleState) -> Viewport {
        state
            .viewport
            .get()
            .unwrap_or_else(|| Viewport::initial(candles.len(), self.initial_bars))
            .clamped(candles.len())
    }

    fn hover_index(
        &self,
        candles: &[Candle],
        state: &CandleState,
        chrome: Chrome,
        position: Point,
    ) -> Option<usize> {
        if !chrome.plot.contains(position) {
            return None;
        }
        let index = self
            .viewport(candles, state)
            .index_near(chrome, position.x)?;
        (index < candles.len()).then_some(index)
    }
}

impl<Message> canvas::Program<Message> for CandleProgram<'_, Message> {
    type State = CandleState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        self.data
            .with(|candles| self.update_with(candles, state, event, bounds, cursor))
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        theme: &iced::Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        self.data
            .with(|candles| self.draw_with(candles, state, renderer, theme, bounds, cursor))
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        let in_plot = cursor
            .position_in(bounds)
            .is_some_and(|position| chrome(bounds.size()).plot.contains(position));
        if in_plot && !self.data.with(<[Candle]>::is_empty) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<Message> CandleProgram<'_, Message> {
    fn update_with(
        &self,
        candles: &[Candle],
        state: &mut CandleState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let chrome = chrome(bounds.size());
        let len = candles.len();
        if len == 0 || chrome.plot.width <= 0.0 || chrome.plot.height <= 0.0 {
            return None;
        }
        match event {
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let position = cursor.position_in(bounds)?;
                if !chrome.plot.contains(position) {
                    return None;
                }
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / WHEEL_PIXELS_PER_LINE,
                };
                if lines == 0.0 {
                    return None;
                }
                let viewport = self.viewport(candles, state);
                let anchor = chrome.index_at(viewport, position.x);
                let factor = ZOOM_PER_LINE.powf(f64::from(-lines));
                state.viewport.set(Some(viewport.zoom(anchor, factor, len)));
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                if !chrome.plot.contains(position) {
                    return None;
                }
                if chip_visible(state.viewport.get(), len)
                    && latest_chip(chrome.plot).contains(position)
                {
                    state.viewport.set(None);
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                state.drag = Some(Drag {
                    x: cursor.position()?.x,
                    viewport: self.viewport(candles, state),
                });
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.drag.take().map(|_| canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (Some(drag), Some(position)) = (state.drag, cursor.position()) {
                    let bars_per_pixel = drag.viewport.span() / f64::from(chrome.plot.width);
                    let bars = f64::from(drag.x - position.x) * bars_per_pixel;
                    state.viewport.set(Some(drag.viewport.pan(bars, len)));
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                let next = cursor
                    .position_in(bounds)
                    .and_then(|position| self.hover_index(candles, state, chrome, position));
                if next != state.hovered {
                    state.hovered = next;
                    if let Some(on_hover) = &self.on_hover {
                        let hit = next.map(|index| CandleHit::new(index, &candles[index]));
                        return Some(canvas::Action::publish(on_hover(hit)));
                    }
                }
                Some(canvas::Action::request_redraw())
            }
            canvas::Event::Mouse(mouse::Event::CursorLeft) => {
                if state.hovered.take().is_some()
                    && let Some(on_hover) = &self.on_hover
                {
                    return Some(canvas::Action::publish(on_hover(None)));
                }
                Some(canvas::Action::request_redraw())
            }
            _ => None,
        }
    }

    fn draw_with(
        &self,
        candles: &[Candle],
        state: &CandleState,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let size = bounds.size();
        if candles.is_empty() {
            let mut frame = canvas::Frame::new(renderer, size);
            self.label(
                &mut frame,
                "No data",
                Point::new(size.width / 2.0, size.height / 2.0),
                TextAlignment::Center,
                Vertical::Center,
                self.theme.palette.muted_foreground,
            );
            return vec![frame.into_geometry()];
        }
        let chrome = chrome(size);
        if chrome.plot.width < 16.0 || chrome.plot.height < 16.0 {
            return Vec::new();
        }
        if let Some(pinned) = state.viewport.get() {
            let followed = follow_appended(pinned, state.seen_len.get(), candles.len());
            if followed != pinned {
                state.viewport.set(Some(followed));
            }
        }
        state.seen_len.set(candles.len());
        let viewport = self.viewport(candles, state);
        let range = visible_indices(viewport, candles.len());

        let stamp = fingerprint(candles, viewport, size, &self.theme.palette);
        let (scale, axes) = {
            let mut derived = state.derived.borrow_mut();
            match derived.as_ref() {
                Some(cached) if cached.stamp == stamp => (cached.scale, cached.axes.clone()),
                _ => {
                    state.layers.clear();
                    let scale = autoscale(candles, range.clone());
                    let axes = self.axes(candles, chrome, viewport, range.clone(), scale);
                    *derived = Some(DerivedFrame {
                        stamp,
                        scale,
                        axes: axes.clone(),
                    });
                    (scale, axes)
                }
            }
        };
        let ctx = FrameCtx {
            candles,
            chrome,
            viewport,
            scale,
            axes: &axes,
        };
        let layers = state.layers.draw(renderer, size, |frame| {
            self.draw_static(&ctx, frame, range);
        });

        // Canvas text renders above canvas fills within a frame, so a tag
        // background can never cover a label already on screen. Axis labels
        // therefore live in the per-frame overlay, where the ones a tag
        // occludes are skipped instead of painted over.
        let mut overlay = canvas::Frame::new(renderer, size);
        let cursor_in_plot = cursor
            .position_in(bounds)
            .filter(|position| chrome.plot.contains(*position));
        self.draw_axis_labels(&ctx, &mut overlay, cursor_in_plot);
        if let Some(position) = cursor_in_plot {
            self.draw_crosshair(&ctx, &mut overlay, position);
        }
        if chip_visible(state.viewport.get(), candles.len()) {
            let chip = latest_chip(chrome.plot);
            overlay.fill_rectangle(
                Point::new(chip.x, chip.y),
                chip.size(),
                self.theme.palette.foreground,
            );
            self.label(
                &mut overlay,
                "Latest >",
                Point::new(chip.center_x(), chip.center_y()),
                TextAlignment::Center,
                Vertical::Center,
                self.theme.palette.background,
            );
        }
        vec![layers, overlay.into_geometry()]
    }
}

impl<Message> CandleProgram<'_, Message> {
    fn label(
        &self,
        frame: &mut canvas::Frame,
        content: &str,
        position: Point,
        align_x: TextAlignment,
        align_y: Vertical,
        color: Color,
    ) {
        frame.fill_text(canvas::Text {
            content: content.to_owned(),
            position,
            color,
            size: Pixels(self.theme.typography.meta_compact),
            font: self.theme.typography.font,
            align_x,
            align_y,
            ..canvas::Text::default()
        });
    }

    fn dashed(&self, frame: &mut canvas::Frame, from: Point, to: Point, color: Color) {
        frame.stroke(
            &Path::line(from, to),
            canvas::Stroke {
                line_dash: canvas::LineDash {
                    segments: &DASH,
                    offset: 0,
                },
                ..canvas::Stroke::default().with_color(color).with_width(1.0)
            },
        );
    }

    fn axes(
        &self,
        candles: &[Candle],
        chrome: Chrome,
        viewport: Viewport,
        range: Range<usize>,
        scale: PriceScale,
    ) -> Axes {
        let (ticks, step) = price_ticks(scale, PRICE_TICK_TARGET);
        let stride = range.len().div_ceil(TIME_TICK_TARGET).max(1);
        let time_ticks = range
            .clone()
            .step_by(stride)
            .map(|index| (chrome.x(viewport, index as f64), candles[index].ts))
            .collect();
        let last_close_y = candles.last().and_then(|last| {
            let y = scale.y(chrome.plot, last.close);
            (y >= chrome.plot.y && y <= chrome.plot.y + chrome.plot.height).then_some(y)
        });
        Axes {
            ticks,
            precision: decimals(step),
            time_ticks,
            time_step_secs: time_step_secs(candles, range),
            last_close_y,
        }
    }

    fn draw_static(&self, ctx: &FrameCtx<'_>, frame: &mut canvas::Frame, range: Range<usize>) {
        let palette = self.theme.palette;
        let plot = ctx.chrome.plot;
        let grid = alpha(palette.border, 0.55);

        for tick in &ctx.axes.ticks {
            let y = ctx.scale.y(plot, *tick);
            frame.fill_rectangle(Point::new(plot.x, y), Size::new(plot.width, 1.0), grid);
        }
        for (x, _) in &ctx.axes.time_ticks {
            frame.fill_rectangle(Point::new(*x, plot.y), Size::new(1.0, plot.height), grid);
        }

        let volume_height = plot.height * VOLUME_RATIO;

        // ponytail: the aggregation pass still scans every visible candle
        // (~ms-ctx.scale at 1M); precompute a min/max pyramid if that ever shows.
        if let Some(columns) =
            aggregate_columns(ctx.candles, range.clone(), ctx.viewport, ctx.chrome)
        {
            // Sub-pixel candles: one wick spanning the column's low..high (a
            // 1px body would be an invisible same-color subset of it) plus an
            // aggregated volume bar.
            let max_volume = columns
                .iter()
                .flatten()
                .fold(0f64, |max, column| max.max(column.volume));
            frame.with_clip(plot, |frame| {
                for (offset, column) in columns.iter().enumerate() {
                    let Some(column) = column else { continue };
                    let color = if column.close >= column.open {
                        palette.success
                    } else {
                        palette.destructive
                    };
                    let x = plot.x + offset as f32;
                    let wick_top = ctx.scale.y(plot, column.high);
                    let wick_bottom = ctx.scale.y(plot, column.low);
                    frame.fill_rectangle(
                        Point::new(x, wick_top),
                        Size::new(1.0, (wick_bottom - wick_top).max(1.0)),
                        color,
                    );
                    if max_volume > 0.0 {
                        let height = volume_height * (column.volume / max_volume) as f32;
                        frame.fill_rectangle(
                            Point::new(x, plot.y + plot.height - height),
                            Size::new(1.0, height),
                            alpha(color, 0.3),
                        );
                    }
                }
            });
        } else {
            let bar_width = plot.width / ctx.viewport.span() as f32;
            let body_width = (bar_width * BODY_RATIO).max(1.0);
            let max_volume = ctx.candles[range.clone()]
                .iter()
                .fold(0f64, |max, candle| max.max(candle.volume));
            frame.with_clip(plot, |frame| {
                for index in range {
                    let candle = &ctx.candles[index];
                    let bullish = candle.close >= candle.open;
                    let color = if bullish {
                        palette.success
                    } else {
                        palette.destructive
                    };
                    let x = ctx.chrome.x(ctx.viewport, index as f64);

                    let wick_top = ctx.scale.y(plot, candle.high);
                    let wick_bottom = ctx.scale.y(plot, candle.low);
                    frame.fill_rectangle(
                        Point::new(x - 0.5, wick_top),
                        Size::new(1.0, (wick_bottom - wick_top).max(1.0)),
                        color,
                    );

                    let body_top = ctx.scale.y(plot, candle.open.max(candle.close));
                    let body_bottom = ctx.scale.y(plot, candle.open.min(candle.close));
                    frame.fill_rectangle(
                        Point::new(x - body_width / 2.0, body_top),
                        Size::new(body_width, (body_bottom - body_top).max(1.0)),
                        color,
                    );

                    if max_volume > 0.0 {
                        let height = volume_height * (candle.volume / max_volume) as f32;
                        frame.fill_rectangle(
                            Point::new(x - body_width / 2.0, plot.y + plot.height - height),
                            Size::new(body_width, height),
                            alpha(color, 0.3),
                        );
                    }
                }
            });
        }

        if let (Some(last), Some(y)) = (ctx.candles.last(), ctx.axes.last_close_y) {
            let color = if last.close >= last.open {
                palette.success
            } else {
                palette.destructive
            };
            self.dashed(
                frame,
                Point::new(plot.x, y),
                Point::new(plot.x + plot.width, y),
                color,
            );
            self.tag(
                frame,
                &format_price(last.close, ctx.axes.precision.max(2)),
                y,
                plot,
                color,
                palette.background,
            );
        }
    }

    fn draw_axis_labels(
        &self,
        ctx: &FrameCtx<'_>,
        frame: &mut canvas::Frame,
        cursor_in_plot: Option<Point>,
    ) {
        let palette = self.theme.palette;
        let plot = ctx.chrome.plot;

        let tag_row = |y: f32| {
            ctx.axes
                .last_close_y
                .is_some_and(|tag_y| (tag_y - y).abs() < TAG_HEIGHT)
                || cursor_in_plot.is_some_and(|position| (position.y - y).abs() < TAG_HEIGHT)
        };
        for tick in &ctx.axes.ticks {
            let y = ctx.scale.y(plot, *tick);
            if tag_row(y) {
                continue;
            }
            self.label(
                frame,
                &format_price(*tick, ctx.axes.precision),
                Point::new(plot.x + plot.width + 6.0, y),
                TextAlignment::Left,
                Vertical::Center,
                palette.muted_foreground,
            );
        }

        let time_tag_x = cursor_in_plot
            .and_then(|position| ctx.viewport.index_near(ctx.chrome, position.x))
            .filter(|index| *index < ctx.candles.len())
            .map(|index| ctx.chrome.x(ctx.viewport, index as f64));
        for (x, ts) in &ctx.axes.time_ticks {
            // Skip labels the plot edge would clip or the crosshair tag covers.
            if *x < 18.0 || *x > plot.width - 18.0 {
                continue;
            }
            if time_tag_x.is_some_and(|tag_x| (tag_x - *x).abs() < 48.0) {
                continue;
            }
            self.label(
                frame,
                &format_ts(*ts, ctx.axes.time_step_secs),
                Point::new(*x, plot.y + plot.height + TIME_AXIS_HEIGHT / 2.0),
                TextAlignment::Center,
                Vertical::Center,
                palette.muted_foreground,
            );
        }
    }

    fn tag(
        &self,
        frame: &mut canvas::Frame,
        content: &str,
        y: f32,
        plot: Rectangle,
        background: Color,
        foreground: Color,
    ) {
        frame.fill_rectangle(
            Point::new(plot.x + plot.width, y - TAG_HEIGHT / 2.0),
            Size::new(PRICE_AXIS_WIDTH, TAG_HEIGHT),
            background,
        );
        self.label(
            frame,
            content,
            Point::new(plot.x + plot.width + 6.0, y),
            TextAlignment::Left,
            Vertical::Center,
            foreground,
        );
    }

    fn draw_crosshair(&self, ctx: &FrameCtx<'_>, frame: &mut canvas::Frame, position: Point) {
        let palette = self.theme.palette;
        let plot = ctx.chrome.plot;

        self.dashed(
            frame,
            Point::new(plot.x, position.y),
            Point::new(plot.x + plot.width, position.y),
            palette.muted_foreground,
        );
        let price = ctx.scale.price_at(plot, position.y);
        self.tag(
            frame,
            &format_price(price, ctx.axes.precision.max(2)),
            position.y,
            plot,
            palette.foreground,
            palette.background,
        );

        let Some(index) = ctx.viewport.index_near(ctx.chrome, position.x) else {
            return;
        };
        if index >= ctx.candles.len() {
            return;
        }
        let x = ctx.chrome.x(ctx.viewport, index as f64);
        self.dashed(
            frame,
            Point::new(x, plot.y),
            Point::new(x, plot.y + plot.height),
            palette.muted_foreground,
        );
        let content = format_ts(ctx.candles[index].ts, ctx.axes.time_step_secs);
        let width =
            (content.len() as f32 * self.theme.typography.meta_compact * 0.65 + 14.0).max(44.0);
        frame.fill_rectangle(
            Point::new(x - width / 2.0, plot.y + plot.height + 1.0),
            Size::new(width, TAG_HEIGHT),
            palette.foreground,
        );
        self.label(
            frame,
            &content,
            Point::new(x, plot.y + plot.height + 1.0 + TAG_HEIGHT / 2.0),
            TextAlignment::Center,
            Vertical::Center,
            palette.background,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles(n: usize) -> Vec<Candle> {
        (0..n)
            .map(|i| Candle {
                ts: 1_700_000_000 + i as i64 * 86_400,
                open: 100.0 + i as f64,
                high: 102.0 + i as f64,
                low: 99.0 + i as f64,
                close: 101.0 + i as f64,
                volume: 1_000.0 + i as f64,
            })
            .collect()
    }

    #[test]
    fn initial_viewport_shows_last_bars_with_margin() {
        let viewport = Viewport::initial(300, 120);
        assert_eq!(viewport.from, 180.0 - 0.5);
        assert!(viewport.to > 299.5);
        assert!(viewport.span() > 120.0 && viewport.span() < 130.0);

        let small = Viewport::initial(10, 120);
        assert!(small.from <= -0.5);
        assert!(small.to >= 9.5);
    }

    #[test]
    fn initial_viewport_survives_tiny_datasets() {
        // 1..=4 candles used to panic via Ord::clamp with min > max.
        for len in 1..=4 {
            let viewport = Viewport::initial(len, DEFAULT_BARS);
            assert!(viewport.span() >= MIN_SPAN);
            assert!(viewport.from < viewport.to);
            let range = visible_indices(viewport, len);
            assert!(!range.is_empty());
        }
    }

    #[test]
    fn zoom_preserves_anchor() {
        let viewport = Viewport {
            from: 10.0,
            to: 110.0,
        };
        let anchor = 80.0;
        let zoomed = viewport.zoom(anchor, 0.5, 1_000);
        let before = (anchor - viewport.from) / viewport.span();
        let after = (anchor - zoomed.from) / zoomed.span();
        assert!((before - after).abs() < 1e-9);
        assert!((zoomed.span() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_clamps_span() {
        let viewport = Viewport {
            from: 0.0,
            to: 100.0,
        };
        assert_eq!(viewport.zoom(50.0, 1e-6, 100).span(), MIN_SPAN);
        assert_eq!(viewport.zoom(50.0, 1e6, 100).span(), 200.0);
    }

    #[test]
    fn pan_keeps_candles_on_screen() {
        let viewport = Viewport {
            from: 0.0,
            to: 50.0,
        };
        let left = viewport.pan(-1e9, 100);
        assert!(left.to >= MIN_EDGE_BARS - 0.5);
        let right = viewport.pan(1e9, 100);
        assert!(right.from <= 100.0 - MIN_EDGE_BARS - 0.5);
    }

    #[test]
    fn visible_indices_covers_edges() {
        assert_eq!(
            visible_indices(
                Viewport {
                    from: -0.5,
                    to: 9.5
                },
                10
            ),
            0..10
        );
        // Candles partially inside the range are included: candle 2 spans
        // 1.5..2.5 and its right sliver is visible at from=2.4.
        assert_eq!(visible_indices(Viewport { from: 2.4, to: 4.6 }, 10), 2..6);
        assert_eq!(
            visible_indices(
                Viewport {
                    from: 50.0,
                    to: 60.0
                },
                10
            ),
            10..10
        );
    }

    #[test]
    fn autoscale_pads_range() {
        let data = candles(10);
        let scale = autoscale(&data, 0..10);
        assert!(scale.lo < 99.0);
        assert!(scale.hi > 111.0);

        let empty = autoscale(&data, 3..3);
        assert_eq!(empty, PriceScale { lo: 0.0, hi: 1.0 });

        let flat = autoscale(
            &[Candle {
                ts: 0,
                open: 50.0,
                high: 50.0,
                low: 50.0,
                close: 50.0,
                volume: 0.0,
            }],
            0..1,
        );
        assert!(flat.hi > flat.lo);
    }

    #[test]
    fn nice_steps_are_1_2_5() {
        assert_eq!(nice_step(0.7), 1.0);
        assert_eq!(nice_step(1.4), 2.0);
        assert_eq!(nice_step(3.9), 5.0);
        assert_eq!(nice_step(7.2), 10.0);
        assert_eq!(nice_step(0.032), 0.05);
    }

    #[test]
    fn price_ticks_stay_in_range() {
        let scale = PriceScale {
            lo: 98.3,
            hi: 114.9,
        };
        let (ticks, step) = price_ticks(scale, 6);
        assert!(!ticks.is_empty());
        assert!(ticks.iter().all(|t| *t >= scale.lo && *t <= scale.hi));
        assert_eq!(decimals(step), 0);
        assert_eq!(decimals(0.05), 2);
    }

    #[test]
    fn price_ticks_terminate_on_ulp_thin_ranges() {
        // A range of a few ulps once made `value += step` a no-op forever.
        for lo in [1.0f64, 60_000.0] {
            let hi = f64::from_bits(lo.to_bits() + 1);
            let scale = autoscale(
                &[Candle {
                    ts: 0,
                    open: lo,
                    high: hi,
                    low: lo,
                    close: hi,
                    volume: 0.0,
                }],
                0..1,
            );
            let (ticks, _) = price_ticks(scale, 6);
            assert!(ticks.len() < 1_000);
        }
    }

    #[test]
    fn time_step_reflects_median_spacing() {
        let mut data = candles(9);
        data[8].ts += 250_000; // one weekend-sized gap does not skew the median
        assert_eq!(time_step_secs(&data, 0..9), 86_400);
        assert_eq!(time_step_secs(&data, 3..4), 86_400);
        assert_eq!(time_step_secs(&data, 0..0), 86_400);

        let flat: Vec<Candle> = candles(3)
            .into_iter()
            .map(|mut candle| {
                candle.ts = 42;
                candle
            })
            .collect();
        assert_eq!(time_step_secs(&flat, 0..3), 1);
    }

    #[test]
    fn civil_dates_are_correct() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(format_ts(1_700_000_000, 86_400), "11-14");
        assert_eq!(format_ts(1_700_000_000, 30 * 86_400), "2023-11");
        assert_eq!(format_ts(1_700_000_000, 3_600), "22:13");
    }

    #[test]
    fn prices_group_thousands() {
        assert_eq!(format_price(0.05, 2), "0.05");
        assert_eq!(format_price(999.0, 0), "999");
        assert_eq!(format_price(1_234.5, 2), "1,234.50");
        assert_eq!(format_price(1_200_000.0, 0), "1,200,000");
        assert_eq!(format_price(-51_234.567, 2), "-51,234.57");
        // Rounding can carry into a new leading digit.
        assert_eq!(format_price(999.995, 2), "1,000.00");
    }

    #[test]
    fn volume_formats_compactly() {
        assert_eq!(format_volume(950.0), "950");
        assert_eq!(format_volume(12_500.0), "12.5K");
        assert_eq!(format_volume(3_400_000.0), "3.4M");
        assert_eq!(format_volume(2_100_000_000.0), "2.1B");
    }

    fn hover_program(data: &[Candle]) -> CandleProgram<'_, Option<CandleHit>> {
        CandleProgram {
            data: Data::Borrowed(data),
            theme: super::super::theme::LIGHT,
            on_hover: Some(Rc::new(|hit| hit)),
            initial_bars: DEFAULT_BARS,
        }
    }

    fn wheel(lines: f32) -> canvas::Event {
        canvas::Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
        })
    }

    fn moved(position: Point) -> canvas::Event {
        canvas::Event::Mouse(mouse::Event::CursorMoved { position })
    }

    const BOUNDS: Size = Size::new(1280.0, 720.0);

    #[test]
    fn wheel_zoom_anchors_the_cursor_and_captures() {
        use iced::widget::canvas::Program as _;

        let data = candles(300);
        let program = hover_program(&data);
        let mut state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);
        let plot = chrome(BOUNDS).plot;
        let point = Point::new(600.0, 300.0);
        let cursor = mouse::Cursor::Available(point);

        let before = program.viewport(&data, &state);
        let anchor = chrome(BOUNDS).index_at(before, point.x);
        let action = program
            .update(&mut state, &wheel(1.0), bounds, cursor)
            .expect("zoom acts");
        let (message, redraw, status) = action.into_inner();
        assert!(message.is_none());
        assert_eq!(redraw, iced::window::RedrawRequest::NextFrame);
        assert_eq!(status, iced::event::Status::Captured);

        let zoomed = state.viewport.get().expect("zoom sets the viewport");
        assert!(zoomed.span() < before.span());
        let anchor_after = chrome(BOUNDS).index_at(zoomed, point.x);
        assert!((anchor - anchor_after).abs() < 1e-6);

        program.update(&mut state, &wheel(-1.0), bounds, cursor);
        let restored = state.viewport.get().expect("zoom out keeps the viewport");
        assert!((restored.span() - before.span()).abs() < 1e-6);

        // Wheel over the price-axis gutter must not zoom.
        let over_axis = mouse::Cursor::Available(Point::new(plot.width + 10.0, 300.0));
        assert!(
            program
                .update(&mut state, &wheel(1.0), bounds, over_axis)
                .is_none()
        );
    }

    #[test]
    fn drag_pans_by_cursor_travel_and_releases() {
        use iced::widget::canvas::Program as _;

        let data = candles(300);
        let program = hover_program(&data);
        let mut state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);
        let plot = chrome(BOUNDS).plot;
        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let release = canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));

        let start = Point::new(600.0, 300.0);
        let action = program
            .update(&mut state, &press, bounds, mouse::Cursor::Available(start))
            .expect("press starts a drag");
        assert_eq!(action.into_inner().2, iced::event::Status::Captured);
        let origin = state.drag.expect("drag recorded").viewport;

        let dragged_to = Point::new(500.0, 300.0);
        let action = program
            .update(
                &mut state,
                &moved(dragged_to),
                bounds,
                mouse::Cursor::Available(dragged_to),
            )
            .expect("drag pans");
        assert_eq!(action.into_inner().2, iced::event::Status::Captured);
        let panned = state.viewport.get().expect("pan sets the viewport");
        let bars = f64::from(start.x - dragged_to.x) * origin.span() / f64::from(plot.width);
        assert!((panned.from - (origin.from + bars)).abs() < 1e-6);

        program
            .update(
                &mut state,
                &release,
                bounds,
                mouse::Cursor::Available(dragged_to),
            )
            .expect("release acts");
        assert!(state.drag.is_none());
    }

    #[test]
    fn hover_emits_once_per_candle_and_clears_on_leave() {
        use iced::widget::canvas::Program as _;

        let data = candles(300);
        let program = hover_program(&data);
        let mut state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);

        let point = Point::new(600.0, 300.0);
        let viewport = program.viewport(&data, &state);
        let expected = chrome(BOUNDS).index_at(viewport, point.x).round() as usize;
        let action = program
            .update(
                &mut state,
                &moved(point),
                bounds,
                mouse::Cursor::Available(point),
            )
            .expect("hover acts");
        let (message, _, status) = action.into_inner();
        let hit = message.expect("hover publishes").expect("a candle is hit");
        assert_eq!(hit.index, expected as i64);
        assert_eq!(hit.close, data[expected].close);
        // A plain hover must not capture — siblings still see the event.
        assert_eq!(status, iced::event::Status::Ignored);

        // One pixel over, same candle: crosshair redraw only, no re-publish.
        let nudged = Point::new(601.0, 300.0);
        let action = program
            .update(
                &mut state,
                &moved(nudged),
                bounds,
                mouse::Cursor::Available(nudged),
            )
            .expect("hover redraws");
        assert!(action.into_inner().0.is_none());

        let action = program
            .update(
                &mut state,
                &canvas::Event::Mouse(mouse::Event::CursorLeft),
                bounds,
                mouse::Cursor::Unavailable,
            )
            .expect("leave acts");
        assert_eq!(action.into_inner().0, Some(None));
        assert!(state.hovered.is_none());
    }

    #[test]
    fn aggregation_matches_brute_force_grouping() {
        use std::collections::HashMap;

        // Varied, deterministic data so columns mix bullish/bearish candles.
        let data: Vec<Candle> = (0..5_000)
            .map(|i| {
                let base = 100.0 + ((i * 37) % 91) as f64;
                Candle {
                    ts: 1_700_000_000 + i as i64 * 3_600,
                    open: base,
                    high: base + ((i * 13) % 17) as f64,
                    low: base - ((i * 7) % 11) as f64,
                    close: base + ((i * 29) % 23) as f64 - 11.0,
                    volume: 10.0 + ((i * 3) % 97) as f64,
                }
            })
            .collect();
        let chrome = chrome(Size::new(464.0, 300.0));
        let viewport = Viewport {
            from: -0.5,
            to: 4_999.5,
        };
        let range = visible_indices(viewport, data.len());
        let columns = aggregate_columns(&data, range.clone(), viewport, chrome)
            .expect("sub-pixel candles aggregate");
        assert_eq!(columns.len(), 400);

        let mut expected: HashMap<usize, Vec<usize>> = HashMap::new();
        for index in range {
            let column = (chrome.x(viewport, index as f64) - chrome.plot.x).floor();
            if (0.0..400.0).contains(&column) {
                expected.entry(column as usize).or_default().push(index);
            }
        }
        for (offset, column) in columns.iter().enumerate() {
            match (column, expected.get(&offset)) {
                (Some(agg), Some(members)) => {
                    let high = members
                        .iter()
                        .map(|i| data[*i].high)
                        .fold(f64::MIN, f64::max);
                    let low = members
                        .iter()
                        .map(|i| data[*i].low)
                        .fold(f64::MAX, f64::min);
                    let volume: f64 = members.iter().map(|i| data[*i].volume).sum();
                    assert_eq!(agg.open, data[members[0]].open);
                    assert_eq!(agg.close, data[*members.last().unwrap()].close);
                    assert_eq!(agg.high, high);
                    assert_eq!(agg.low, low);
                    assert!((agg.volume - volume).abs() < 1e-9);
                }
                (None, None) => {}
                (agg, members) => {
                    panic!("column {offset}: agg {agg:?} vs members {members:?}")
                }
            }
        }

        let aggregated: f64 = columns.iter().flatten().map(|c| c.volume).sum();
        let visible: f64 = data.iter().map(|c| c.volume).sum();
        assert!((aggregated - visible).abs() < 1e-6);
    }

    #[test]
    fn aggregation_only_engages_below_one_pixel_per_candle() {
        let data = candles(100);
        let chrome = chrome(Size::new(464.0, 300.0));
        let wide = Viewport {
            from: -0.5,
            to: 99.5,
        };
        assert!(aggregate_columns(&data, 0..100, wide, chrome).is_none());

        let narrow = Viewport {
            from: -0.5,
            to: 799.5,
        };
        assert!(aggregate_columns(&data, 0..100, narrow, chrome).is_some());
    }

    #[test]
    fn follow_tracks_appends_only_at_the_right_edge() {
        // Pinned at the right edge: 5 appended candles shift the view by 5.
        let pinned = Viewport {
            from: 179.5,
            to: 299.5,
        };
        let followed = follow_appended(pinned, 300, 305);
        assert!((followed.from - 184.5).abs() < 1e-9);
        assert!((followed.to - 304.5).abs() < 1e-9);

        // Scrolled into history: appends leave the view alone.
        let history = Viewport {
            from: 10.0,
            to: 130.0,
        };
        assert_eq!(follow_appended(history, 300, 305), history);

        // No append, or first observation: unchanged.
        assert_eq!(follow_appended(pinned, 300, 300), pinned);
        assert_eq!(follow_appended(pinned, 0, 300), pinned);
    }

    #[test]
    fn latest_chip_resets_to_follow() {
        use iced::widget::canvas::Program as _;

        let data = candles(300);
        let program = hover_program(&data);
        let mut state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);
        let plot = chrome(BOUNDS).plot;

        // Pan away from the right edge so the chip appears.
        state.viewport.set(Some(Viewport {
            from: 10.0,
            to: 130.0,
        }));
        assert!(chip_visible(state.viewport.get(), data.len()));
        assert!(!chip_visible(None, data.len()));

        let chip = latest_chip(plot);
        let inside = Point::new(chip.center_x(), chip.center_y());
        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let action = program
            .update(&mut state, &press, bounds, mouse::Cursor::Available(inside))
            .expect("chip press acts");
        assert_eq!(action.into_inner().2, iced::event::Status::Captured);
        assert!(state.viewport.get().is_none());
        assert!(state.drag.is_none());
    }

    /// Timing evidence for the cached-layer design, not a pass/fail test.
    /// Run with:
    /// `cargo test -p ducktape-ui --release --features candle-chart,tiny-skia,x11 \`
    /// `  --lib candle_chart::tests::bench_frame_costs -- --ignored --nocapture`
    #[test]
    #[ignore = "timing evidence; run explicitly in release mode"]
    fn bench_frame_costs() {
        use iced::advanced::renderer::Headless as _;
        use iced::widget::canvas::Program as _;
        use std::time::{Duration, Instant};

        // Debug-mode numbers are meaningless and CI runs ignored tests.
        if cfg!(debug_assertions) {
            eprintln!("bench_frame_costs: skipped; run with --release");
            return;
        }

        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            Pixels(14.0),
            Some("tiny-skia"),
        ))
        .expect("headless tiny-skia renderer");
        let theme = super::super::theme::LIGHT;
        let bounds = Rectangle::with_size(Size::new(1280.0, 720.0));
        let cursor = mouse::Cursor::Available(Point::new(600.0, 300.0));

        let per_frame = |state: &CandleState, program: &CandleProgram<'_, ()>, cold: bool| {
            let frame = || {
                if cold {
                    state.layers.clear();
                    *state.derived.borrow_mut() = None;
                }
                let _ = program.draw(state, &renderer, &iced::Theme::Light, bounds, cursor);
            };
            frame();
            let start = Instant::now();
            let mut iterations = 0u32;
            while start.elapsed() < Duration::from_millis(300) {
                frame();
                iterations += 1;
            }
            start.elapsed() / iterations.max(1)
        };

        println!("\n1280x720 frame build (tiny-skia geometry recording), per frame:");
        for count in [1_000usize, 10_000, 100_000, 1_000_000] {
            let data = candles(count);
            let program = CandleProgram::<()> {
                data: Data::Borrowed(&data),
                theme,
                on_hover: None,
                initial_bars: DEFAULT_BARS,
            };
            let views = [
                ("last-120", None),
                (
                    "all",
                    Some(
                        Viewport {
                            from: -0.5,
                            to: count as f64 - 0.5,
                        }
                        .clamped(count),
                    ),
                ),
            ];
            for (label, viewport) in views {
                let state = CandleState {
                    viewport: Cell::new(viewport),
                    ..CandleState::default()
                };
                let cold = per_frame(&state, &program, true);
                let warm = per_frame(&state, &program, false);
                println!(
                    "  {count:>9} candles  view={label:<8}  rebuild {cold:>10.1?}  cached {warm:>10.1?}"
                );
            }
        }
    }

    #[test]
    fn fingerprint_tracks_data_viewport_and_theme() {
        let data = candles(10);
        let viewport = Viewport {
            from: 0.0,
            to: 10.0,
        };
        let palette = super::super::theme::LIGHT.palette;
        let size = Size::new(1280.0, 720.0);
        let base = fingerprint(&data, viewport, size, &palette);
        let mut appended = data.clone();
        appended.push(Candle {
            ts: 1_700_000_000 + 10 * 86_400,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 10.0,
        });
        assert_ne!(base, fingerprint(&appended, viewport, size, &palette));

        let mut ticked = data.clone();
        ticked[9].close += 0.01;
        assert_ne!(base, fingerprint(&ticked, viewport, size, &palette));

        assert_ne!(
            base,
            fingerprint(
                &data,
                Viewport {
                    from: 1.0,
                    to: 11.0
                },
                size,
                &palette,
            )
        );

        assert_ne!(
            base,
            fingerprint(&data, viewport, Size::new(1280.0, 640.0), &palette),
        );

        let dark = super::super::theme::DARK.palette;
        assert_ne!(base, fingerprint(&data, viewport, size, &dark));
    }
}
