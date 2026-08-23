//! Interactive candlestick chart for time-series market data.
//!
//! Grid, candles, volume, and axes render into a cached layer that is only
//! rebuilt when the data or the visible range changes; the crosshair and its
//! axis tags draw on a per-frame overlay. The wheel zooms around the cursor,
//! dragging pans, and the price scale follows the visible candles.
//!
//! Anything drawn over the candles is a [`ChartOverlay`]: it receives the
//! frame's time and price mapping through [`ChartCoords`] and reports a
//! `stamp` that keys the geometry cache. [`CandleChart::markers`] and
//! [`CandleChart::price_lines`] are the built-in ones, so a caller adds a
//! band, a bracket, or a trend line the same way the chart adds its own.

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

use super::theme::{Theme as UiTheme, alpha};
use iced::advanced::text::Alignment as TextAlignment;
use iced::alignment::Vertical;
use iced::keyboard::{self, key::Named};
use iced::mouse;
use iced::widget::Canvas;
use iced::widget::canvas::{self, Path};
use iced::window;
use iced::{Color, Element, Length, Pixels, Point, Rectangle, Size};

const DEFAULT_HEIGHT: f32 = 320.0;
/// How many candles the chart opens showing. Public because it is also what a
/// caller needs to know to decide whether a tape is worth opening on: a width
/// carrying fewer bars than this opens on a plot that is mostly empty.
pub const DEFAULT_BARS: usize = 120;
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
/// How close to the oldest candle a pan has to come before the chart asks for
/// more history: far enough out that the fetch lands before the gap shows.
const HISTORY_MARGIN_BARS: f64 = 20.0;
const PRICE_TICK_TARGET: usize = 6;
const TIME_TICK_TARGET: usize = 6;
const TAG_HEIGHT: f32 = 16.0;
const DASH: [f32; 2] = [3.0, 3.0];
const MARKER_WIDTH: f32 = 9.0;
const MARKER_HEIGHT: f32 = 8.0;

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

/// How many candles were loaded in front of the tape since the last draw,
/// found by looking up where the candle that used to be oldest now sits.
/// Every index behind them moved by this much, and so must the pinned view.
fn prepended(candles: &[Candle], seen_first_ts: Option<i64>) -> usize {
    seen_first_ts.map_or(0, |seen| candles.partition_point(|candle| candle.ts < seen))
}

/// Whether `candles` is still the series the view was pinned to, grown at
/// either end: the candle that used to be oldest is where the prepend count
/// says it is, and nothing was dropped off the back.
///
/// Another market or another interval fails both halves. Its indices mean
/// nothing here, and a view carried over would clamp against the new length
/// and strand the tape at the left edge of an otherwise empty plot.
fn continues(
    candles: &[Candle],
    seen_first_ts: Option<i64>,
    seen_len: usize,
    loaded: usize,
) -> bool {
    let Some(seen) = seen_first_ts else {
        return true;
    };
    candles.get(loaded).is_some_and(|candle| candle.ts == seen)
        && candles.len() >= seen_len + loaded
}

/// Brings the pinned view up to date with a tape that has changed under it,
/// before anything is measured against it. `candles` must not be empty.
///
/// Candles loaded in front of the tape shift every index behind them, so the
/// view moves by as much to stay on the bars it was showing, and that shift
/// counts into `seen_len` so the auto-follow only sees genuine appends. A
/// tape that is not the same series at all keeps none of it.
fn reconcile(state: &CandleState, candles: &[Candle]) {
    let seen_first_ts = state.seen_first_ts.get();
    let loaded = prepended(candles, seen_first_ts);
    if continues(candles, seen_first_ts, state.seen_len.get(), loaded) {
        if loaded > 0 {
            if let Some(pinned) = state.viewport.get() {
                state
                    .viewport
                    .set(Some(pinned.pan(loaded as f64, candles.len())));
            }
            state.seen_len.set(state.seen_len.get() + loaded);
        }
        if let Some(pinned) = state.viewport.get() {
            let followed = follow_appended(pinned, state.seen_len.get(), candles.len());
            if followed != pinned {
                state.viewport.set(Some(followed));
            }
        }
    } else {
        // A different market or interval. Everything pinned to the old series
        // — the view, the walked cursor, the history request it already spent
        // — describes candles that are not here, so this one opens at its own
        // tip.
        state.viewport.set(None);
        state.key_cursor.set(None);
        state.requested_len.set(None);
    }
    state.seen_first_ts.set(Some(candles[0].ts));
    state.seen_len.set(candles.len());
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

fn autoscale(
    pyramid: &Pyramid,
    candles: &[Candle],
    range: Range<usize>,
    overlays: &[Box<dyn ChartOverlay + '_>],
) -> PriceScale {
    let (mut lo, mut hi) = if range.is_empty() {
        (f64::INFINITY, f64::NEG_INFINITY)
    } else {
        pyramid.extremes(candles, range.clone())
    };
    for overlay in overlays {
        if let Some((overlay_lo, overlay_hi)) = overlay.price_range(candles, range.clone())
            && overlay_lo.is_finite()
            && overlay_hi.is_finite()
            && overlay_lo <= overlay_hi
        {
            lo = lo.min(overlay_lo);
            hi = hi.max(overlay_hi);
        }
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
    last_index: usize,
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
    pyramid: &Pyramid,
) -> Option<Vec<Option<ColumnAgg>>> {
    let plot = chrome.plot;
    let bar_width = plot.width / viewport.span() as f32;
    if bar_width >= 1.0 || plot.width < 1.0 {
        return None;
    }
    let width = plot.width.ceil() as usize;
    let mut columns: Vec<Option<ColumnAgg>> = vec![None; width];
    let column_of = |index: usize| (chrome.x(viewport, index as f64) - plot.x).floor() as i64;
    let mut cursor = range.start;
    while cursor < range.end {
        let column = column_of(cursor);
        if column >= width as i64 {
            break;
        }
        if column < 0 {
            cursor += 1;
            continue;
        }
        // Analytic guess for where the next column starts, corrected by at
        // most a step or two of float error; columns are monotone in index.
        let guess = viewport.from + (column + 1) as f64 * viewport.span() / f64::from(plot.width);
        let mut next = (guess.ceil() as i64).clamp(cursor as i64 + 1, range.end as i64) as usize;
        while next > cursor + 1 && column_of(next - 1) > column {
            next -= 1;
        }
        while next < range.end && column_of(next) <= column {
            next += 1;
        }
        let (low, high) = pyramid.extremes(candles, cursor..next);
        columns[column as usize] = Some(ColumnAgg {
            open: candles[cursor].open,
            high,
            low,
            close: candles[next - 1].close,
            volume: pyramid.volume_sum(cursor..next),
            last_index: next - 1,
        });
        cursor = next;
    }
    Some(columns)
}

/// Mipmap-style block size for the tape summaries.
const PYRAMID_BLOCK: usize = 64;

/// Incrementally maintained summaries over the tape: block min/max levels for
/// O(log) range extremes and prefix sums for O(1) close/volume range sums, so
/// a rebuild costs O(plot width * log candles) instead of O(visible candles).
/// Memory is ~2 f64 per candle (prefixes) plus ~2/64th for the levels.
struct Pyramid {
    /// The whole first candle, not just its timestamp: two markets on the
    /// same interval share a timestamp grid, so a tape that swaps symbols
    /// keeps its length and its front stamp while every number under it
    /// changes.
    first: Option<Candle>,
    last: Option<Candle>,
    /// level k summarizes blocks of PYRAMID_BLOCK^(k+1) candles.
    low_min: Vec<Vec<f64>>,
    high_max: Vec<Vec<f64>>,
    close_prefix: Vec<f64>,
    volume_prefix: Vec<f64>,
}

impl Pyramid {
    fn empty() -> Self {
        Self {
            first: None,
            last: None,
            low_min: Vec::new(),
            high_max: Vec::new(),
            close_prefix: vec![0.0],
            volume_prefix: vec![0.0],
        }
    }

    fn len(&self) -> usize {
        self.close_prefix.len() - 1
    }

    /// Brings the summaries in line with `candles`: appends extend, a changed
    /// last candle updates its block path, an evicted front rebuilds.
    fn sync(&mut self, candles: &[Candle]) {
        let same_front = self.first.is_some() && self.first == candles.first().copied();
        if candles.len() < self.len() || (!candles.is_empty() && !same_front) {
            *self = Self::empty();
        }
        if candles.is_empty() {
            *self = Self::empty();
            return;
        }
        self.first = candles.first().copied();
        // A tick rewrote the last known candle in place.
        let known = self.len();
        if known > 0 && self.last != Some(candles[known - 1]) {
            let candle = candles[known - 1];
            let close_base = self.close_prefix[known - 1];
            let volume_base = self.volume_prefix[known - 1];
            self.close_prefix[known] = close_base + candle.close;
            self.volume_prefix[known] = volume_base + candle.volume;
            self.refresh_blocks(candles, known - 1);
        }
        // Appended candles extend the prefixes and their block paths.
        for index in known..candles.len() {
            let candle = candles[index];
            self.close_prefix
                .push(self.close_prefix[index] + candle.close);
            self.volume_prefix
                .push(self.volume_prefix[index] + candle.volume);
            self.refresh_blocks(candles, index);
        }
        self.last = candles.last().copied();
    }

    /// Recomputes every level block containing `index` from the level below.
    fn refresh_blocks(&mut self, candles: &[Candle], index: usize) {
        let mut block = index / PYRAMID_BLOCK;
        for level in 0.. {
            if self.low_min.len() == level {
                self.low_min.push(Vec::new());
                self.high_max.push(Vec::new());
            }
            let (lows, highs) = (&mut self.low_min[level], &mut self.high_max[level]);
            if lows.len() <= block {
                lows.resize(block + 1, f64::INFINITY);
                highs.resize(block + 1, f64::NEG_INFINITY);
            }
            let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
            if level == 0 {
                let start = block * PYRAMID_BLOCK;
                let end = ((block + 1) * PYRAMID_BLOCK).min(candles.len());
                for candle in &candles[start..end] {
                    low = low.min(candle.low);
                    high = high.max(candle.high);
                }
            } else {
                let start = block * PYRAMID_BLOCK;
                let end = ((block + 1) * PYRAMID_BLOCK).min(self.low_min[level - 1].len());
                for child in start..end {
                    low = low.min(self.low_min[level - 1][child]);
                    high = high.max(self.high_max[level - 1][child]);
                }
            }
            self.low_min[level][block] = low;
            self.high_max[level][block] = high;
            // Keep going until a single root block exists, so upper levels
            // are never left as stale placeholders when the tape grows.
            if self.low_min[level].len() <= 1 {
                break;
            }
            block /= PYRAMID_BLOCK;
        }
    }

    /// Min low and max high over `range`: partial edges scan directly and
    /// aligned interiors consume whole summary blocks, level by level.
    fn extremes(&self, candles: &[Candle], range: Range<usize>) -> (f64, f64) {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        let mut start = range.start;
        let mut end = range.end;

        let aligned_start = start.next_multiple_of(PYRAMID_BLOCK).min(end);
        let aligned_end = (end / PYRAMID_BLOCK * PYRAMID_BLOCK).max(aligned_start);
        for candle in candles[start..aligned_start]
            .iter()
            .chain(&candles[aligned_end..end])
        {
            low = low.min(candle.low);
            high = high.max(candle.high);
        }
        start = aligned_start / PYRAMID_BLOCK;
        end = aligned_end / PYRAMID_BLOCK;

        for level in 0..self.low_min.len() {
            if start >= end {
                break;
            }
            let aligned_start = start.next_multiple_of(PYRAMID_BLOCK).min(end);
            let aligned_end = (end / PYRAMID_BLOCK * PYRAMID_BLOCK).max(aligned_start);
            let is_top = level + 1 >= self.low_min.len() || aligned_start >= aligned_end;
            let blocks = if is_top {
                (start..end).chain(0..0)
            } else {
                (start..aligned_start).chain(aligned_end..end)
            };
            for block in blocks {
                low = low.min(self.low_min[level][block]);
                high = high.max(self.high_max[level][block]);
            }
            if is_top {
                break;
            }
            start = aligned_start / PYRAMID_BLOCK;
            end = aligned_end / PYRAMID_BLOCK;
        }
        (low, high)
    }

    fn close_sum(&self, range: Range<usize>) -> f64 {
        self.close_prefix[range.end] - self.close_prefix[range.start]
    }

    fn volume_sum(&self, range: Range<usize>) -> f64 {
        self.volume_prefix[range.end] - self.volume_prefix[range.start]
    }
}

/// Overlay line colors rotate through these theme roles per moving average.
const MA_STROKE_WIDTH: f32 = 1.5;

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
struct Axes {
    ticks: Vec<f64>,
    /// Decimals on axis tick labels.
    precision: usize,
    /// Decimals on the last-price and crosshair tags.
    tag_precision: usize,
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
    pyramid: &'a Pyramid,
}

/// Median spacing between the candles in `range`, for time-label
/// granularity. Wide ranges are stride-sampled (a median over an even sample
/// is the same label-granularity signal) so this stays O(1)-bounded.
fn time_step_secs(candles: &[Candle], range: Range<usize>) -> i64 {
    const SAMPLE_CAP: usize = 1_024;
    let candles = &candles[range];
    if candles.len() < 2 {
        return 86_400;
    }
    let pairs = candles.len() - 1;
    let stride = pairs.div_ceil(SAMPLE_CAP).max(1);
    let mut deltas: Vec<i64> = (0..pairs)
        .step_by(stride)
        .map(|i| (candles[i + 1].ts - candles[i].ts).max(1))
        .collect();
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
pub fn format_ts(ts: i64, step_secs: i64) -> String {
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

/// A time-axis tick label. Intraday ticks read as clock time, except the
/// first one on screen and the first of each day, which carry the date —
/// a wall of bare clock times says nothing about which day you are looking
/// at. `previous` is the last tick actually drawn, not the last considered.
fn format_tick(ts: i64, step_secs: i64, previous: Option<i64>) -> String {
    let day = ts.div_euclid(86_400);
    if step_secs >= 86_400 || previous.is_some_and(|prior| prior.div_euclid(86_400) == day) {
        return format_ts(ts, step_secs);
    }
    let (_, month, date) = civil_from_days(day);
    let secs = ts.rem_euclid(86_400);
    format!(
        "{month:02}-{date:02} {:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60
    )
}

/// The crosshair's time tag, which always carries its date: the axis only
/// prints one at a day boundary, and the tag is what a reader points at to
/// settle exactly which candle this is.
fn format_ts_dated(ts: i64, step_secs: i64) -> String {
    let (year, month, date) = civil_from_days(ts.div_euclid(86_400));
    if step_secs >= 28 * 86_400 {
        format!("{year}-{month:02}")
    } else if step_secs >= 86_400 {
        format!("{year}-{month:02}-{date:02}")
    } else {
        let secs = ts.rem_euclid(86_400);
        format!(
            "{month:02}-{date:02} {:02}:{:02}",
            secs / 3600,
            (secs % 3600) / 60
        )
    }
}

fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x0000_0100_0000_01b3)
}

fn label_hash(label: Option<&str>) -> u64 {
    label.map_or(0, |text| {
        text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            mix(hash, u64::from(byte))
        })
    })
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
    overlays: u64,
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
    // Overlays report their own stamp; mixing it in means a moved marker or
    // a new price line rebuilds the cached layer like a new candle does.
    hash = mix(hash, overlays);
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

/// The glyph for one marker, positioned so an arrow's tip lands on `at` and a
/// centred shape is centred on it.
fn marker_path(shape: MarkerShape, at: Point) -> Path {
    let half = MARKER_WIDTH / 2.0;
    match shape {
        MarkerShape::ArrowUp | MarkerShape::ArrowDown => {
            let direction = if shape == MarkerShape::ArrowUp {
                1.0
            } else {
                -1.0
            };
            let base = at.y + direction * MARKER_HEIGHT;
            Path::new(|builder| {
                builder.move_to(at);
                builder.line_to(Point::new(at.x - half, base));
                builder.line_to(Point::new(at.x + half, base));
                builder.close();
            })
        }
        MarkerShape::Diamond => Path::new(|builder| {
            builder.move_to(Point::new(at.x, at.y - half));
            builder.line_to(Point::new(at.x + half, at.y));
            builder.line_to(Point::new(at.x, at.y + half));
            builder.line_to(Point::new(at.x - half, at.y));
            builder.close();
        }),
        MarkerShape::Dot => Path::circle(at, half * 0.8),
    }
}

/// The candle a fill landed in: the last one that opened at or before it.
/// Fills older than the loaded tape have none.
fn marker_index(candles: &[Candle], ts: i64) -> Option<usize> {
    candles
        .partition_point(|candle| candle.ts <= ts)
        .checked_sub(1)
}

/// What an overlay is given to draw with: the plot rectangle and the same
/// time/price coordinate mapping the chart itself uses this frame.
pub struct ChartCoords<'a> {
    theme: &'a UiTheme,
    candles: &'a [Candle],
    chrome: Chrome,
    viewport: Viewport,
    scale: PriceScale,
    visible: Range<usize>,
}

impl<'a> ChartCoords<'a> {
    /// The candle area, excluding the price and time axis strips.
    pub fn plot(&self) -> Rectangle {
        self.chrome.plot
    }

    /// The candles currently in view, as indices into [`ChartCoords::candles`].
    pub fn visible(&self) -> Range<usize> {
        self.visible.clone()
    }

    pub fn candles(&self) -> &'a [Candle] {
        self.candles
    }

    pub fn theme(&self) -> &'a UiTheme {
        self.theme
    }

    /// The candle a timestamp falls in: the last one that opened at or before
    /// it. Timestamps older than the loaded data have none.
    pub fn index_at(&self, ts: i64) -> Option<usize> {
        marker_index(self.candles, ts)
    }

    /// Horizontal centre of a candle. Fractional indices interpolate, so an
    /// overlay can place something between two candles.
    pub fn x_for_index(&self, index: f64) -> f32 {
        self.chrome.x(self.viewport, index)
    }

    /// Horizontal position of a timestamp, `None` when it predates the data.
    pub fn x_for_ts(&self, ts: i64) -> Option<f32> {
        Some(self.x_for_index(self.index_at(ts)? as f64))
    }

    pub fn y_for_price(&self, price: f64) -> f32 {
        self.scale.y(self.chrome.plot, price)
    }

    pub fn price_at_y(&self, y: f32) -> f64 {
        self.scale.price_at(self.chrome.plot, y)
    }

    /// Chart text in the chart's own font, size, and alignment rules.
    pub fn label(
        &self,
        frame: &mut canvas::Frame,
        content: &str,
        position: Point,
        align_x: TextAlignment,
        align_y: Vertical,
        color: Color,
    ) {
        draw_label(
            self.theme, frame, content, position, align_x, align_y, color,
        );
    }

    /// A dashed line in the chart's crosshair style.
    pub fn dashed(&self, frame: &mut canvas::Frame, from: Point, to: Point, color: Color) {
        draw_dashed(frame, from, to, color);
    }

    /// A filled label in the price-axis strip, level with `y`.
    pub fn tag(
        &self,
        frame: &mut canvas::Frame,
        content: &str,
        y: f32,
        background: Color,
        foreground: Color,
    ) {
        draw_tag(
            self.theme,
            frame,
            content,
            y,
            self.chrome.plot,
            background,
            foreground,
        );
    }
}

/// A drawing attached to the chart, painted over the candles in the order the
/// overlays were added.
///
/// The chart caches its geometry, so an overlay reports a `stamp` that changes
/// whenever its drawing would: the chart mixes it into the cache key and
/// repaints when it moves. A stamp that never changes pins stale geometry on
/// screen.
pub trait ChartOverlay {
    fn draw(&self, frame: &mut canvas::Frame, coords: &ChartCoords<'_>);

    fn stamp(&self) -> u64;

    /// Additional price bounds this overlay chooses to keep visible.
    /// The chart asks only when rebuilding its cached geometry; overlays may
    /// return `None` for annotations that should not expand the candle scale,
    /// and invalid or reversed bounds are ignored.
    fn price_range(&self, _candles: &[Candle], _visible: Range<usize>) -> Option<(f64, f64)> {
        None
    }
}

/// Where a marker sits relative to the price it annotates, and which way it
/// points. The chart draws the glyph; what it means is the caller's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerShape {
    /// Triangle under the price, tip pointing up at it.
    ArrowUp,
    /// Triangle over the price, tip pointing down at it.
    ArrowDown,
    /// Diamond centred on the price.
    Diamond,
    /// Dot centred on the price.
    Dot,
}

/// A point annotation: one glyph at `price` on the candle holding `ts` (unix
/// seconds), with an optional short label beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartMarker {
    pub ts: i64,
    pub price: f64,
    pub shape: MarkerShape,
    pub color: Color,
    pub label: Option<String>,
}

impl ChartMarker {
    pub fn new(ts: i64, price: f64, shape: MarkerShape, color: Color) -> Self {
        Self {
            ts,
            price,
            shape,
            color,
            label: None,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// The built-in marker overlay, installed by [`CandleChart::markers`].
pub struct Markers {
    markers: Vec<ChartMarker>,
    stamp: u64,
}

impl Markers {
    pub fn new(markers: impl IntoIterator<Item = ChartMarker>) -> Self {
        let mut markers: Vec<ChartMarker> = markers.into_iter().collect();
        markers.sort_by_key(|marker| marker.ts);
        let stamp = markers.iter().fold(0xcbf2_9ce4_8422_2325, |hash, marker| {
            let hash = mix(mix(hash, marker.ts as u64), marker.price.to_bits());
            let hash = mix(hash, marker.shape as u64);
            mix(
                mix_color(hash, marker.color),
                label_hash(marker.label.as_deref()),
            )
        });
        Self { markers, stamp }
    }
}

impl ChartOverlay for Markers {
    fn draw(&self, frame: &mut canvas::Frame, coords: &ChartCoords<'_>) {
        let visible = coords.visible();
        if self.markers.is_empty() || visible.is_empty() {
            return;
        }
        let outline = coords.theme().palette.background;
        frame.with_clip(coords.plot(), |frame| {
            for marker in &self.markers {
                let Some(index) = coords.index_at(marker.ts) else {
                    continue;
                };
                if !visible.contains(&index) {
                    continue;
                }
                let at = Point::new(
                    coords.x_for_index(index as f64),
                    coords.y_for_price(marker.price),
                );
                let path = marker_path(marker.shape, at);
                frame.fill(&path, marker.color);
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_color(outline)
                        .with_width(1.0),
                );
                if let Some(text) = &marker.label {
                    let (offset, align_y) = match marker.shape {
                        MarkerShape::ArrowDown => (-MARKER_HEIGHT - 2.0, Vertical::Bottom),
                        _ => (MARKER_HEIGHT + 2.0, Vertical::Top),
                    };
                    coords.label(
                        frame,
                        text,
                        Point::new(at.x, at.y + offset),
                        TextAlignment::Center,
                        align_y,
                        marker.color,
                    );
                }
            }
        });
    }

    fn stamp(&self) -> u64 {
        self.stamp
    }
}

/// A horizontal annotation across the plot at `price`, tagged on the price
/// axis: an entry, a liquidation level, a target, an alert.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceLine {
    pub price: f64,
    pub color: Color,
    pub label: Option<String>,
    pub dashed: bool,
}

impl PriceLine {
    pub fn new(price: f64, color: Color) -> Self {
        Self {
            price,
            color,
            label: None,
            dashed: true,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn solid(mut self) -> Self {
        self.dashed = false;
        self
    }
}

/// The built-in price-line overlay, installed by [`CandleChart::price_lines`].
pub struct PriceLines {
    lines: Vec<PriceLine>,
    stamp: u64,
}

impl PriceLines {
    pub fn new(lines: impl IntoIterator<Item = PriceLine>) -> Self {
        let lines: Vec<PriceLine> = lines.into_iter().collect();
        let stamp = lines.iter().fold(0xcbf2_9ce4_8422_2325, |hash, line| {
            let hash = mix(hash, line.price.to_bits());
            let hash = mix(mix_color(hash, line.color), u64::from(line.dashed));
            mix(hash, label_hash(line.label.as_deref()))
        });
        Self { lines, stamp }
    }
}

impl ChartOverlay for PriceLines {
    fn draw(&self, frame: &mut canvas::Frame, coords: &ChartCoords<'_>) {
        let plot = coords.plot();
        for line in &self.lines {
            let y = coords.y_for_price(line.price);
            if y < plot.y || y > plot.y + plot.height {
                continue;
            }
            let from = Point::new(plot.x, y);
            if line.dashed {
                coords.dashed(frame, from, Point::new(plot.x + plot.width, y), line.color);
            } else {
                frame.fill_rectangle(from, Size::new(plot.width, 1.0), line.color);
            }
            if let Some(label) = &line.label {
                coords.tag(
                    frame,
                    label,
                    y,
                    line.color,
                    coords.theme().palette.background,
                );
            }
        }
    }

    fn stamp(&self) -> u64 {
        self.stamp
    }
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
    on_reach_start: Option<Rc<dyn Fn(Option<CandleHit>) -> Message + 'a>>,
    width: Length,
    height: Length,
    initial_bars: usize,
    precision: Option<usize>,
    time_offset_secs: i64,
    moving_averages: Vec<usize>,
    overlays: Vec<Box<dyn ChartOverlay + 'a>>,
    live: Option<Duration>,
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
        on_reach_start: None,
        width: Length::Fill,
        height: Length::Fixed(DEFAULT_HEIGHT),
        initial_bars: DEFAULT_BARS,
        precision: None,
        time_offset_secs: 0,
        moving_averages: Vec::new(),
        overlays: Vec::new(),
        live: None,
    }
}

impl<'a, Message> CandleChart<'a, Message> {
    #[must_use]
    pub fn on_hover(mut self, on_hover: impl Fn(Option<CandleHit>) -> Message + 'a) -> Self {
        self.on_hover = Some(Rc::new(on_hover));
        self
    }

    /// Called when a pan or zoom brings the view within a candle or two of
    /// the oldest one loaded, so the app can fetch what came before it. It
    /// carries the hovered candle like [`Self::on_hover`] does, so a single
    /// message type can answer both without losing the readout.
    ///
    /// Fires at most once per tape length: loading more history changes the
    /// length and arms it again, while a drag that reaches the edge and stays
    /// there asks only once.
    #[must_use]
    pub fn on_reach_start(
        mut self,
        on_reach_start: impl Fn(Option<CandleHit>) -> Message + 'a,
    ) -> Self {
        self.on_reach_start = Some(Rc::new(on_reach_start));
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

    /// How many decimals this instrument is quoted to.
    ///
    /// The tag on the price line is written to exactly that, because its job
    /// is to say which price the market is at, and a market quoted to a
    /// millionth cannot say so in five decimals. The axis is written to
    /// whatever its own gridline step needs, capped here: labels finer than
    /// the instrument trades are digits that cannot differ, and labels
    /// coarser are two gridlines that read the same.
    #[must_use]
    pub fn precision(mut self, decimals: usize) -> Self {
        self.precision = Some(decimals.min(8));
        self
    }

    /// Shifts time-axis labels by a fixed UTC offset in seconds (candle
    /// timestamps stay UTC; only label rendering moves).
    #[must_use]
    pub fn time_offset(mut self, seconds: i64) -> Self {
        self.time_offset_secs = seconds;
        self
    }

    /// Overlays simple moving averages of the close, one line per period.
    #[must_use]
    pub fn moving_averages(mut self, periods: impl IntoIterator<Item = usize>) -> Self {
        self.moving_averages = periods.into_iter().filter(|p| *p > 1).collect();
        self
    }

    /// Attaches a drawing to the chart. Overlays paint over the candles in
    /// the order they were added, each with the frame's coordinate mapping.
    #[must_use]
    pub fn overlay(mut self, overlay: impl ChartOverlay + 'a) -> Self {
        self.overlays.push(Box::new(overlay));
        self
    }

    /// Point annotations pinned to a time and a price: fills, signals, alerts.
    #[must_use]
    pub fn markers(self, markers: impl IntoIterator<Item = ChartMarker>) -> Self {
        self.overlay(Markers::new(markers))
    }

    /// Horizontal annotations across the plot: entry, liquidation, target.
    #[must_use]
    pub fn price_lines(self, lines: impl IntoIterator<Item = PriceLine>) -> Self {
        self.overlay(PriceLines::new(lines))
    }
    /// Repaints on its own beat so a live feed renders without any app
    /// message or view rebuild: shared-tape mutations are picked up by the
    /// data fingerprint on each beat (the LiveSurface scheduling idea,
    /// expressed inside the canvas program).
    #[must_use]
    pub fn live(mut self, interval: Duration) -> Self {
        self.live = Some(interval);
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
                on_reach_start: chart.on_reach_start,
                initial_bars: chart.initial_bars,
                precision: chart.precision,
                time_offset_secs: chart.time_offset_secs,
                moving_averages: chart.moving_averages,
                overlays: chart.overlays,
                live: chart.live,
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
    /// Keyboard-walked candle index; mouse movement clears it.
    key_cursor: Cell<Option<usize>>,
    /// Data length at the last draw, to detect appends for auto-follow.
    seen_len: Cell<usize>,
    /// Timestamp of the oldest candle at the last draw, to detect a history
    /// load prepending to the front and shifting every index.
    seen_first_ts: Cell<Option<i64>>,
    /// Tape length the last history request was made at, so reaching the
    /// oldest candle asks once rather than on every dragged pixel.
    requested_len: Cell<Option<usize>>,
    drag: Option<Drag>,
    hovered: Option<usize>,
    layers: canvas::Cache,
    derived: RefCell<Option<DerivedFrame>>,
    pyramid: RefCell<Pyramid>,
}

impl Default for CandleState {
    fn default() -> Self {
        Self {
            viewport: Cell::new(None),
            key_cursor: Cell::new(None),
            seen_len: Cell::new(0),
            seen_first_ts: Cell::new(None),
            requested_len: Cell::new(None),
            drag: None,
            hovered: None,
            layers: canvas::Cache::new(),
            derived: RefCell::new(None),
            pyramid: RefCell::new(Pyramid::empty()),
        }
    }
}

struct CandleProgram<'a, Message> {
    data: Data<'a>,
    theme: UiTheme,
    on_hover: Option<Rc<dyn Fn(Option<CandleHit>) -> Message + 'a>>,
    on_reach_start: Option<Rc<dyn Fn(Option<CandleHit>) -> Message + 'a>>,
    initial_bars: usize,
    precision: Option<usize>,
    time_offset_secs: i64,
    moving_averages: Vec<usize>,
    overlays: Vec<Box<dyn ChartOverlay + 'a>>,
    live: Option<Duration>,
}

impl<Message> CandleProgram<'_, Message> {
    fn viewport(&self, candles: &[Candle], state: &CandleState) -> Viewport {
        state
            .viewport
            .get()
            .unwrap_or_else(|| Viewport::initial(candles.len(), self.initial_bars))
            .clamped(candles.len())
    }

    /// The history request a freshly moved viewport earns, if any: the view
    /// has reached the oldest candle and this tape length has not asked yet.
    fn history_request(
        &self,
        candles: &[Candle],
        state: &CandleState,
        viewport: Viewport,
    ) -> Option<canvas::Action<Message>> {
        let on_reach_start = self.on_reach_start.as_ref()?;
        if viewport.from > HISTORY_MARGIN_BARS || state.requested_len.get() == Some(candles.len()) {
            return None;
        }
        state.requested_len.set(Some(candles.len()));
        let hovered = state
            .hovered
            .and_then(|index| Some(CandleHit::new(index, candles.get(index)?)));
        Some(canvas::Action::publish(on_reach_start(hovered)).and_capture())
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
                let zoomed = viewport.zoom(anchor, factor, len);
                state.viewport.set(Some(zoomed));
                self.history_request(candles, state, zoomed)
                    .or_else(|| Some(canvas::Action::request_redraw().and_capture()))
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
                    let panned = drag.viewport.pan(bars, len);
                    state.viewport.set(Some(panned));
                    return self
                        .history_request(candles, state, panned)
                        .or_else(|| Some(canvas::Action::request_redraw().and_capture()));
                }
                state.key_cursor.set(None);
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
            canvas::Event::Window(window::Event::RedrawRequested(now)) => {
                let interval = self.live?;
                Some(canvas::Action::request_redraw_at(*now + interval))
            }
            canvas::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                // Keyboard crosshair only while the pointer is on the chart,
                // so arrows keep working for the rest of the app otherwise.
                cursor.position_in(bounds)?;
                let step: i64 = match key {
                    keyboard::Key::Named(Named::ArrowLeft) => -1,
                    keyboard::Key::Named(Named::ArrowRight) => 1,
                    keyboard::Key::Named(Named::Escape) => {
                        if state.key_cursor.take().is_some() {
                            return Some(canvas::Action::request_redraw().and_capture());
                        }
                        return None;
                    }
                    _ => return None,
                };
                let viewport = self.viewport(candles, state);
                let current = state
                    .key_cursor
                    .get()
                    .unwrap_or_else(|| visible_indices(viewport, len).end.saturating_sub(1));
                let target = (current as i64 + step).clamp(0, len as i64 - 1) as usize;
                state.key_cursor.set(Some(target));
                // Keep the walked candle on screen.
                let visible = visible_indices(viewport, len);
                if !visible.contains(&target) {
                    state.viewport.set(Some(viewport.pan(step as f64, len)));
                }
                if state.hovered != Some(target) {
                    state.hovered = Some(target);
                    if let Some(on_hover) = &self.on_hover {
                        let hit = Some(CandleHit::new(target, &candles[target]));
                        return Some(canvas::Action::publish(on_hover(hit)).and_capture());
                    }
                }
                Some(canvas::Action::request_redraw().and_capture())
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
        reconcile(state, candles);
        let viewport = self.viewport(candles, state);
        let range = visible_indices(viewport, candles.len());

        let overlays = self
            .overlays
            .iter()
            .fold(0xcbf2_9ce4_8422_2325, |hash, overlay| {
                mix(hash, overlay.stamp())
            });
        let visuals = self.moving_averages.iter().fold(
            mix(overlays, self.moving_averages.len() as u64),
            |hash, period| mix(hash, *period as u64),
        );
        let visuals = if self.moving_averages.is_empty() {
            visuals
        } else {
            [
                self.theme.palette.accent,
                self.theme.palette.warning,
                self.theme.palette.brand,
            ]
            .into_iter()
            .fold(visuals, mix_color)
        };
        let mut pyramid = state.pyramid.borrow_mut();
        pyramid.sync(candles);
        let stamp = fingerprint(candles, visuals, viewport, size, &self.theme.palette);
        {
            let mut derived = state.derived.borrow_mut();
            if derived.as_ref().is_none_or(|cached| cached.stamp != stamp) {
                state.layers.clear();
                let scale = autoscale(&pyramid, candles, range.clone(), &self.overlays);
                let axes = self.axes(candles, chrome, viewport, range.clone(), scale);
                *derived = Some(DerivedFrame { stamp, scale, axes });
            }
        }
        let derived = state.derived.borrow();
        let cached = derived.as_ref().expect("derived frame initialized above");
        let ctx = FrameCtx {
            candles,
            chrome,
            viewport,
            scale: cached.scale,
            axes: &cached.axes,
            pyramid: &pyramid,
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
        let key_position = state.key_cursor.get().and_then(|index| {
            let candle = candles.get(index)?;
            let x = chrome.x(viewport, index as f64);
            chrome
                .plot
                .contains(Point::new(x, chrome.plot.y + 1.0))
                .then(|| Point::new(x, cached.scale.y(chrome.plot, candle.close)))
        });
        let crosshair = cursor_in_plot.or(key_position);
        self.draw_axis_labels(&ctx, &mut overlay, crosshair);
        if let Some(position) = crosshair {
            self.draw_crosshair(&ctx, &mut overlay, position);
        }
        drop(derived);
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

/// Chart text in the chart's own font and size.
fn draw_label(
    theme: &UiTheme,
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
        size: Pixels(theme.typography.meta_compact),
        font: theme.typography.font,
        align_x,
        align_y,
        ..canvas::Text::default()
    });
}

fn draw_dashed(frame: &mut canvas::Frame, from: Point, to: Point, color: Color) {
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

/// A filled label in the price-axis strip, level with `y`.
fn draw_tag(
    theme: &UiTheme,
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
    draw_label(
        theme,
        frame,
        content,
        Point::new(plot.x + plot.width + 6.0, y),
        TextAlignment::Left,
        Vertical::Center,
        foreground,
    );
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
        draw_label(
            &self.theme,
            frame,
            content,
            position,
            align_x,
            align_y,
            color,
        );
    }

    fn dashed(&self, frame: &mut canvas::Frame, from: Point, to: Point, color: Color) {
        draw_dashed(frame, from, to, color);
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
            .map(|index| {
                (
                    chrome.x(viewport, index as f64),
                    candles[index].ts + self.time_offset_secs,
                )
            })
            .collect();
        let last_close_y = candles.last().and_then(|last| {
            let y = scale.y(chrome.plot, last.close);
            (y >= chrome.plot.y && y <= chrome.plot.y + chrome.plot.height).then_some(y)
        });
        Axes {
            ticks,
            precision: self
                .precision
                .map_or_else(|| decimals(step), |quoted| quoted.min(decimals(step))),
            tag_precision: self.precision.unwrap_or_else(|| decimals(step).max(2)),
            time_ticks,
            time_step_secs: time_step_secs(candles, range),
            last_close_y,
        }
    }

    fn draw_static(&self, ctx: &FrameCtx<'_>, frame: &mut canvas::Frame, range: Range<usize>) {
        let overlay_range = range.clone();
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
        if let Some(columns) = aggregate_columns(
            ctx.candles,
            range.clone(),
            ctx.viewport,
            ctx.chrome,
            ctx.pyramid,
        ) {
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

        self.draw_moving_averages(ctx, frame, overlay_range.clone());

        if !self.overlays.is_empty() {
            let coords = ChartCoords {
                theme: &self.theme,
                candles: ctx.candles,
                chrome: ctx.chrome,
                viewport: ctx.viewport,
                scale: ctx.scale,
                visible: overlay_range,
            };
            for overlay in &self.overlays {
                overlay.draw(frame, &coords);
            }
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
                &format_price(last.close, ctx.axes.tag_precision),
                y,
                plot,
                color,
                palette.background,
            );
        }
    }

    /// One polyline per configured period; sampled per candle when bars are
    /// wide, per pixel column when aggregated, so the point count stays
    /// bounded by the plot width in both modes.
    fn draw_moving_averages(
        &self,
        ctx: &FrameCtx<'_>,
        frame: &mut canvas::Frame,
        range: Range<usize>,
    ) {
        if self.moving_averages.is_empty() || range.is_empty() {
            return;
        }
        let palette = self.theme.palette;
        let colors = [palette.accent, palette.warning, palette.brand];
        let sma = |index: usize, period: usize| -> Option<f64> {
            (index + 1 >= period)
                .then(|| ctx.pyramid.close_sum(index + 1 - period..index + 1) / period as f64)
        };
        let columns = aggregate_columns(
            ctx.candles,
            range.clone(),
            ctx.viewport,
            ctx.chrome,
            ctx.pyramid,
        );

        for (slot, period) in self.moving_averages.iter().enumerate() {
            let color = colors[slot % colors.len()];
            let mut points: Vec<Point> = Vec::new();
            match &columns {
                Some(columns) => {
                    for (offset, column) in columns.iter().enumerate() {
                        let Some(column) = column else { continue };
                        if let Some(mean) = sma(column.last_index, *period) {
                            points.push(Point::new(
                                ctx.chrome.plot.x + offset as f32,
                                ctx.scale.y(ctx.chrome.plot, mean),
                            ));
                        }
                    }
                }
                None => {
                    for index in range.clone() {
                        if let Some(mean) = sma(index, *period) {
                            points.push(Point::new(
                                ctx.chrome.x(ctx.viewport, index as f64),
                                ctx.scale.y(ctx.chrome.plot, mean),
                            ));
                        }
                    }
                }
            }
            if points.len() < 2 {
                continue;
            }
            let path = Path::new(|builder| {
                builder.move_to(points[0]);
                for point in &points[1..] {
                    builder.line_to(*point);
                }
            });
            frame.with_clip(ctx.chrome.plot, |frame| {
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_color(color)
                        .with_width(MA_STROKE_WIDTH),
                );
            });
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
        let mut drawn: Option<i64> = None;
        for (x, ts) in &ctx.axes.time_ticks {
            // Skip labels the plot edge would clip or the crosshair tag covers.
            if *x < 18.0 || *x > plot.width - 18.0 {
                continue;
            }
            if time_tag_x.is_some_and(|tag_x| (tag_x - *x).abs() < 48.0) {
                continue;
            }
            let content = format_tick(*ts, ctx.axes.time_step_secs, drawn);
            drawn = Some(*ts);
            self.label(
                frame,
                &content,
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
        draw_tag(&self.theme, frame, content, y, plot, background, foreground);
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
            &format_price(price, ctx.axes.tag_precision),
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
        let content = format_ts_dated(
            ctx.candles[index].ts + self.time_offset_secs,
            ctx.axes.time_step_secs,
        );
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
    use std::alloc::System;

    use super::*;
    use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

    #[global_allocator]
    static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

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
        let scale = autoscale(&pyramid_for(&data), &data, 0..10, &[]);
        assert!(scale.lo < 99.0);
        assert!(scale.hi > 111.0);

        let empty = autoscale(&pyramid_for(&data), &data, 3..3, &[]);
        assert_eq!(empty, PriceScale { lo: 0.0, hi: 1.0 });

        let flat_data = [Candle {
            ts: 0,
            open: 50.0,
            high: 50.0,
            low: 50.0,
            close: 50.0,
            volume: 0.0,
        }];
        let flat = autoscale(&pyramid_for(&flat_data), &flat_data, 0..1, &[]);
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
            let ulp_data = [Candle {
                ts: 0,
                open: lo,
                high: hi,
                low: lo,
                close: hi,
                volume: 0.0,
            }];
            let scale = autoscale(&pyramid_for(&ulp_data), &ulp_data, 0..1, &[]);
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

    /// The axis answers "tell these gridlines apart" and the tag answers
    /// "which price is this". They are not the same number of decimals, and
    /// writing one of them with the other's is how a market quoted to a
    /// millionth had its price tagged 0.00842 while every other panel on the
    /// screen said 0.008421.
    #[test]
    fn the_tag_is_quoted_to_the_instrument_and_the_axis_to_its_step() {
        let data = candles(50);
        let chrome = chrome(BOUNDS);
        let viewport = Viewport::initial(50, DEFAULT_BARS);
        let range = visible_indices(viewport, 50);
        let scale = autoscale(&pyramid_for(&data), &data, range.clone(), &[]);

        let derived = hover_program(&data);
        let axes = derived.axes(&data, chrome, viewport, range.clone(), scale);
        assert_eq!(axes.precision, 0);
        assert_eq!(axes.tag_precision, 2);

        // Quoting to four decimals does not put four decimals on gridlines a
        // whole unit apart; they would be four zeroes every label.
        let mut fixed = hover_program(&data);
        fixed.precision = Some(4);
        let axes = fixed.axes(&data, chrome, viewport, range, scale);
        assert_eq!(axes.precision, 0, "the step still decides the axis");
        assert_eq!(axes.tag_precision, 4, "the instrument decides the tag");
    }

    #[test]
    fn time_offset_shifts_labels_not_data() {
        let data = candles(50);
        let chrome = chrome(BOUNDS);
        let viewport = Viewport::initial(50, DEFAULT_BARS);
        let range = visible_indices(viewport, 50);
        let scale = autoscale(&pyramid_for(&data), &data, range.clone(), &[]);

        let mut seoul = hover_program(&data);
        seoul.time_offset_secs = 9 * 3_600;
        let shifted = seoul.axes(&data, chrome, viewport, range.clone(), scale);
        let utc = hover_program(&data).axes(&data, chrome, viewport, range, scale);
        for (offset_tick, utc_tick) in shifted.time_ticks.iter().zip(&utc.time_ticks) {
            assert_eq!(offset_tick.1, utc_tick.1 + 9 * 3_600);
            assert_eq!(offset_tick.0, utc_tick.0);
        }
        // The step derives from raw deltas, so the offset cancels out.
        assert_eq!(shifted.time_step_secs, utc.time_step_secs);
    }

    #[test]
    fn pyramid_extremes_and_sums_match_brute_force() {
        let data: Vec<Candle> = (0..5_000)
            .map(|i| {
                let base = 100.0 + ((i * 37) % 91) as f64;
                Candle {
                    ts: 1_700_000_000 + i as i64 * 60,
                    open: base,
                    high: base + ((i * 13) % 17) as f64,
                    low: base - ((i * 7) % 11) as f64,
                    close: base + ((i * 29) % 23) as f64 - 11.0,
                    volume: 10.0 + ((i * 3) % 97) as f64,
                }
            })
            .collect();
        let pyramid = pyramid_for(&data);
        for range in [
            0..1,
            0..63,
            10..64,
            63..65,
            0..4096,
            100..4999,
            4095..5000,
            0..5000,
        ] {
            let low = data[range.clone()]
                .iter()
                .map(|c| c.low)
                .fold(f64::MAX, f64::min);
            let high = data[range.clone()]
                .iter()
                .map(|c| c.high)
                .fold(f64::MIN, f64::max);
            let (p_low, p_high) = pyramid.extremes(&data, range.clone());
            assert_eq!((p_low, p_high), (low, high), "range {range:?}");
            let volume: f64 = data[range.clone()].iter().map(|c| c.volume).sum();
            assert!((pyramid.volume_sum(range.clone()) - volume).abs() < 1e-6);
            let closes: f64 = data[range.clone()].iter().map(|c| c.close).sum();
            assert!((pyramid.close_sum(range)).abs() - closes.abs() < 1e-6);
        }
    }

    #[test]
    fn pyramid_sync_is_incremental_and_survives_eviction() {
        let mut data = candles(200);
        let mut incremental = pyramid_for(&data[..150]);

        // Appends extend.
        incremental.sync(&data);
        let fresh = pyramid_for(&data);
        assert_eq!(
            incremental.extremes(&data, 0..200),
            fresh.extremes(&data, 0..200)
        );
        assert_eq!(incremental.close_sum(0..200), fresh.close_sum(0..200));

        // A tick to the last candle updates its block path.
        data[199].close += 50.0;
        data[199].high = data[199].high.max(data[199].close);
        incremental.sync(&data);
        let fresh = pyramid_for(&data);
        assert_eq!(
            incremental.extremes(&data, 150..200),
            fresh.extremes(&data, 150..200)
        );
        assert_eq!(incremental.close_sum(199..200), fresh.close_sum(199..200));

        // Front eviction rebuilds.
        data.drain(..37);
        incremental.sync(&data);
        let fresh = pyramid_for(&data);
        assert_eq!(incremental.len(), data.len());
        assert_eq!(
            incremental.extremes(&data, 0..data.len()),
            fresh.extremes(&data, 0..data.len())
        );
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

    fn pyramid_for(data: &[Candle]) -> Pyramid {
        let mut pyramid = Pyramid::empty();
        pyramid.sync(data);
        pyramid
    }

    /// Two markets on one interval share a timestamp grid, so a tape that
    /// swaps symbols can keep its length and its first timestamp while every
    /// price under it changes. The summaries have to notice, or the moving
    /// averages and the price scale keep reading the market you left.
    #[test]
    fn the_summaries_rebuild_when_the_tape_swaps_symbols() {
        let held = candles(200);
        let mut pyramid = pyramid_for(&held);
        assert_eq!(pyramid.extremes(&held, 0..200), (99.0, 301.0));

        let swapped: Vec<Candle> = held
            .iter()
            .map(|candle| Candle {
                open: candle.open / 100.0,
                high: candle.high / 100.0,
                low: candle.low / 100.0,
                close: candle.close / 100.0,
                ..*candle
            })
            .collect();
        pyramid.sync(&swapped);
        assert_eq!(
            pyramid.extremes(&swapped, 0..200),
            (0.99, 3.01),
            "the block summaries follow the new market"
        );
        assert!(
            (pyramid.close_sum(0..200) - swapped.iter().map(|c| c.close).sum::<f64>()).abs() < 1e-9,
            "so do the prefix sums the moving averages read"
        );
    }

    #[test]
    fn time_labels_carry_the_date_where_it_changes() {
        // Midnight UTC on 2023-11-14.
        let day = 1_700_000_000i64.div_euclid(86_400) * 86_400;

        // The first label on screen is dated, the rest of that day are not.
        assert_eq!(format_tick(day + 3_600, 900, None), "11-14 01:00");
        assert_eq!(format_tick(day + 7_200, 900, Some(day + 3_600)), "02:00");
        assert_eq!(
            format_tick(day + 86_400, 900, Some(day + 7_200)),
            "11-15 00:00",
            "midnight starts a new date"
        );
        // A daily view already carries its date in every label.
        assert_eq!(format_tick(day, 86_400, Some(day - 86_400)), "11-14");

        // The crosshair tag is never bare, whatever the axis is showing.
        assert_eq!(format_ts_dated(day + 3_600, 900), "11-14 01:00");
        assert_eq!(format_ts_dated(day, 86_400), "2023-11-14");
        assert_eq!(format_ts_dated(day, 30 * 86_400), "2023-11");
    }

    #[test]
    fn reaching_the_oldest_candle_asks_for_history_once() {
        use iced::widget::canvas::Program as _;

        let data = candles(300);
        let program = history_program(&data);
        let mut state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);
        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let at = |x: f32| mouse::Cursor::Available(Point::new(x, 300.0));
        let drag_to = |state: &mut CandleState, x: f32| {
            program
                .update(state, &moved(Point::new(x, 300.0)), bounds, at(x))
                .expect("a drag pans")
                .into_inner()
                .0
        };

        program.update(&mut state, &press, bounds, at(600.0));

        // A drag that stays in the recent tape asks for nothing.
        assert!(
            drag_to(&mut state, 700.0).is_none(),
            "the oldest candle is still far off"
        );

        // Dragging back past it publishes exactly one request.
        assert!(
            drag_to(&mut state, 3_000.0).is_some(),
            "the view reached the oldest candle"
        );
        assert!(
            drag_to(&mut state, 3_100.0).is_none(),
            "one request per tape length, not one per dragged pixel"
        );
    }

    /// Loading older candles renumbers every index. The pinned view has to
    /// move with them or the screen jumps back in time on its own.
    #[test]
    fn a_history_load_keeps_the_view_on_the_same_candles() {
        let data = candles(300);
        let older: Vec<Candle> = candles(100)
            .iter()
            .map(|candle| Candle {
                ts: candle.ts - 100 * 86_400,
                ..*candle
            })
            .chain(data.iter().copied())
            .collect();

        assert_eq!(prepended(&older, Some(data[0].ts)), 100);
        assert_eq!(prepended(&data, Some(data[0].ts)), 0, "a plain append");
        assert_eq!(prepended(&data, None), 0, "nothing seen yet");

        let pinned = Viewport {
            from: 10.0,
            to: 60.0,
        };
        let moved = pinned.pan(prepended(&older, Some(data[0].ts)) as f64, older.len());
        assert_eq!(
            (moved.from, moved.to),
            (110.0, 160.0),
            "the view follows its candles across the prepend"
        );
    }

    /// Switching the interval replaces the tape. The view that was following
    /// the old one clamps against the new length and leaves the whole tape
    /// squeezed against the left edge — an all but empty chart.
    #[test]
    fn a_replaced_tape_is_not_a_continuation_of_the_old_one() {
        let minutes = candles(1_000);
        // The same window at a coarser interval: fewer bars, reaching further
        // back, and sharing no timestamp with the tape it replaces.
        let days: Vec<Candle> = candles(300)
            .iter()
            .map(|candle| Candle {
                ts: candle.ts - 500 * 86_400 + 37,
                ..*candle
            })
            .collect();

        assert!(
            !continues(&days, Some(minutes[0].ts), minutes.len(), 0),
            "another interval is another series"
        );
        let loaded = prepended(&days, Some(minutes[0].ts));
        assert!(
            !continues(&days, Some(minutes[0].ts), minutes.len(), loaded),
            "and it is not a history load either, however far back it reaches"
        );

        // What carrying the view over would have done: the clamp drags the
        // view to the far end of the shorter tape, leaving a handful of
        // candles hard against the left edge and the rest of the plot empty.
        let pinned = Viewport {
            from: 880.0,
            to: 1_000.0,
        };
        let carried = visible_indices(pinned.clamped(days.len()), days.len());
        assert!(
            carried.len() <= MIN_EDGE_BARS as usize + 1,
            "the bug this test is here for: {} of {} candles left on screen",
            carried.len(),
            days.len()
        );
        assert_eq!(carried.end, days.len(), "and they are the newest ones");

        // Starting over puts the newest candle back on the right.
        let opened = Viewport::initial(days.len(), 120);
        assert!(
            opened.to >= days.len() as f64 - 1.5,
            "a fresh view opens at the tip"
        );
        assert!(
            visible_indices(opened, days.len()).len() > 100,
            "and shows a chart, not two bars"
        );
    }

    /// The sequence a user performs: watch one interval, switch to another.
    /// Between the two the tape is emptied and refilled, and what the chart
    /// remembers about the old one has to not survive that.
    #[test]
    fn switching_interval_reopens_the_chart_at_the_tip() {
        let state = CandleState::default();
        let minutes = candles(1_000);
        reconcile(&state, &minutes);
        // Settled on the tip of the minutes, as the first draw leaves it.
        state
            .viewport
            .set(Some(Viewport::initial(minutes.len(), 120)));
        state.key_cursor.set(Some(990));
        state.requested_len.set(Some(minutes.len()));

        let days: Vec<Candle> = candles(300)
            .iter()
            .map(|candle| Candle {
                ts: candle.ts - 500 * 86_400 + 37,
                ..*candle
            })
            .collect();
        reconcile(&state, &days);

        assert_eq!(state.viewport.get(), None, "the old view is not reused");
        assert_eq!(state.seen_len.get(), days.len());
        assert_eq!(state.key_cursor.get(), None, "nor the walked candle");
        assert_eq!(
            state.requested_len.get(),
            None,
            "and the new tape may ask for its own history"
        );

        let opened = Viewport::initial(days.len(), 120).clamped(days.len());
        assert!(
            visible_indices(opened, days.len()).len() > 100,
            "the chart opens full, not squeezed against the left edge"
        );
    }

    #[test]
    fn growing_at_either_end_is_still_the_same_series() {
        let data = candles(300);
        let older: Vec<Candle> = candles(100)
            .iter()
            .map(|candle| Candle {
                ts: candle.ts - 100 * 86_400,
                ..*candle
            })
            .chain(data.iter().copied())
            .collect();

        assert!(continues(&data, None, 0, 0), "nothing seen yet");
        assert!(
            continues(&data, Some(data[0].ts), 300, 0),
            "a redraw of the same tape"
        );
        assert!(
            continues(&data, Some(data[0].ts), 290, 0),
            "an append keeps the front"
        );
        assert!(
            continues(&older, Some(data[0].ts), 300, 100),
            "a history load moves the front and the view with it"
        );
        assert!(
            !continues(&data[..200], Some(data[0].ts), 300, 0),
            "a tape that lost its back is not the one that was pinned"
        );
    }

    fn hover_program(data: &[Candle]) -> CandleProgram<'_, Option<CandleHit>> {
        CandleProgram {
            data: Data::Borrowed(data),
            theme: super::super::theme::LIGHT,
            on_hover: Some(Rc::new(|hit| hit)),
            on_reach_start: None,
            initial_bars: DEFAULT_BARS,
            precision: None,
            time_offset_secs: 0,
            moving_averages: Vec::new(),
            overlays: Vec::new(),
            live: None,
        }
    }

    /// Publishes only when the view reaches the oldest candle, so any message
    /// out of this program is a history request.
    fn history_program(data: &[Candle]) -> CandleProgram<'_, Option<CandleHit>> {
        CandleProgram {
            on_hover: None,
            on_reach_start: Some(Rc::new(|hit| hit)),
            ..hover_program(data)
        }
    }

    /// A caller-defined overlay: the extension point the built-ins ride on.
    struct Ruler(f64);

    impl ChartOverlay for Ruler {
        fn draw(&self, _frame: &mut canvas::Frame, _coords: &ChartCoords<'_>) {}

        fn stamp(&self) -> u64 {
            self.0.to_bits()
        }
    }

    struct CountedOverlay {
        stamp: u64,
        range: (f64, f64),
        draws: Rc<Cell<usize>>,
        ranges: Rc<Cell<usize>>,
    }

    impl ChartOverlay for CountedOverlay {
        fn draw(&self, _frame: &mut canvas::Frame, _coords: &ChartCoords<'_>) {
            self.draws.set(self.draws.get() + 1);
        }

        fn stamp(&self) -> u64 {
            self.stamp
        }

        fn price_range(&self, _candles: &[Candle], _visible: Range<usize>) -> Option<(f64, f64)> {
            self.ranges.set(self.ranges.get() + 1);
            Some(self.range)
        }
    }

    fn coords_for<'a>(data: &'a [Candle], theme: &'a UiTheme) -> ChartCoords<'a> {
        let chrome = chrome(BOUNDS);
        let viewport = Viewport::initial(data.len(), DEFAULT_BARS);
        let visible = visible_indices(viewport, data.len());
        ChartCoords {
            theme,
            candles: data,
            chrome,
            viewport,
            scale: autoscale(&pyramid_for(data), data, visible.clone(), &[]),
            visible,
        }
    }

    #[test]
    fn coords_map_time_and_price_for_overlays() {
        let data = candles(40);
        let step = data[1].ts - data[0].ts;
        let coords = coords_for(&data, &super::super::theme::LIGHT);

        assert_eq!(coords.index_at(data[0].ts - 1), None, "before the tape");
        assert_eq!(coords.index_at(data[3].ts), Some(3));
        // Mid-candle marks stay on the candle they opened in.
        assert_eq!(coords.index_at(data[3].ts + step - 1), Some(3));
        assert_eq!(coords.index_at(data[39].ts + step * 99), Some(39));

        assert_eq!(coords.x_for_ts(data[0].ts - 1), None);
        assert_eq!(coords.x_for_ts(data[7].ts), Some(coords.x_for_index(7.0)));
        // Fractional indices land between their neighbours.
        let between = coords.x_for_index(7.5);
        assert!(between > coords.x_for_index(7.0) && between < coords.x_for_index(8.0));

        let price = data[20].close;
        let round_trip = coords.price_at_y(coords.y_for_price(price));
        assert!((round_trip - price).abs() < 1e-6, "{round_trip} != {price}");
        // Higher prices sit higher on screen.
        assert!(coords.y_for_price(price + 1.0) < coords.y_for_price(price));
    }

    #[test]
    fn overlays_stack_and_invalidate_the_cached_layer() {
        let data = candles(10);
        let theme = &super::super::theme::LIGHT;
        let palette = &theme.palette;
        let marks = || {
            [
                ChartMarker::new(
                    data[5].ts,
                    101.0,
                    MarkerShape::ArrowDown,
                    palette.destructive,
                ),
                ChartMarker::new(data[2].ts, 100.0, MarkerShape::ArrowUp, palette.success)
                    .label("0.5"),
            ]
        };

        let chart: CandleChart<'_, ()> = candle_chart(&data, theme)
            .markers(marks())
            .price_lines([PriceLine::new(100.5, palette.accent).label("entry")])
            .overlay(Ruler(1.0));
        assert_eq!(chart.overlays.len(), 3, "built-ins are overlays too");

        let markers = Markers::new(marks());
        assert_eq!(markers.markers[0].ts, data[2].ts, "markers sort by time");

        // Every annotation edit has to move the stamp, or the cached geometry
        // outlives the data it was drawn from.
        let relabelled = Markers::new([marks()[0].clone(), marks()[1].clone().label("1.0")]);
        let recoloured = Markers::new([
            marks()[0].clone(),
            ChartMarker::new(data[2].ts, 100.0, MarkerShape::Dot, palette.success).label("0.5"),
        ]);
        assert_ne!(markers.stamp(), Markers::new([]).stamp());
        assert_ne!(markers.stamp(), relabelled.stamp(), "label edits repaint");
        assert_ne!(markers.stamp(), recoloured.stamp(), "shape edits repaint");
        assert_ne!(
            PriceLines::new([PriceLine::new(100.5, palette.accent)]).stamp(),
            PriceLines::new([PriceLine::new(100.5, palette.accent).solid()]).stamp(),
            "line style edits repaint"
        );

        let viewport = Viewport::initial(data.len(), DEFAULT_BARS);
        let bare = fingerprint(&data, 0, viewport, BOUNDS, palette);
        assert_ne!(
            bare,
            fingerprint(&data, markers.stamp(), viewport, BOUNDS, palette),
            "overlay stamps reach the cache key"
        );
    }

    #[test]
    fn overlay_membership_and_moving_averages_invalidate_the_same_cache() {
        use iced::advanced::renderer::Headless as _;
        use iced::widget::canvas::Program as _;

        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            Pixels(14.0),
            Some("tiny-skia"),
        ))
        .expect("headless tiny-skia renderer");
        let data = candles(40);
        let state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);
        let cursor = mouse::Cursor::Unavailable;
        let first_draws = Rc::new(Cell::new(0));
        let first_ranges = Rc::new(Cell::new(0));
        let second_draws = Rc::new(Cell::new(0));
        let second_ranges = Rc::new(Cell::new(0));
        let first = || CountedOverlay {
            stamp: 1,
            range: (80.0, 160.0),
            draws: Rc::clone(&first_draws),
            ranges: Rc::clone(&first_ranges),
        };
        let second = || CountedOverlay {
            stamp: 2,
            range: (-1_000.0, 1_000.0),
            draws: Rc::clone(&second_draws),
            ranges: Rc::clone(&second_ranges),
        };

        let mut both = hover_program(&data);
        both.overlays = vec![Box::new(first()), Box::new(second())];
        let _ = both.draw(&state, &renderer, &iced::Theme::Light, bounds, cursor);
        assert_eq!((first_draws.get(), second_draws.get()), (1, 1));
        assert_eq!((first_ranges.get(), second_ranges.get()), (1, 1));
        let scale = state
            .derived
            .borrow()
            .as_ref()
            .expect("derived frame")
            .scale;
        assert!(scale.lo < -1_000.0 && scale.hi > 1_000.0);

        let _ = both.draw(&state, &renderer, &iced::Theme::Light, bounds, cursor);
        assert_eq!(
            (first_draws.get(), second_draws.get()),
            (1, 1),
            "a cache hit does not redraw overlays"
        );
        assert_eq!(
            (first_ranges.get(), second_ranges.get()),
            (1, 1),
            "a cache hit does not recompute overlay bounds"
        );

        let mut one = hover_program(&data);
        one.overlays = vec![Box::new(first())];
        let _ = one.draw(&state, &renderer, &iced::Theme::Light, bounds, cursor);
        assert_eq!(
            (first_draws.get(), second_draws.get()),
            (2, 1),
            "removing an overlay invalidates this CandleState"
        );
        assert_eq!((first_ranges.get(), second_ranges.get()), (2, 1));
        let scale = state
            .derived
            .borrow()
            .as_ref()
            .expect("derived frame")
            .scale;
        assert!(scale.lo > -1_000.0 && scale.hi < 1_000.0);

        let mut averaged = hover_program(&data);
        averaged.moving_averages = vec![20];
        averaged.overlays = vec![Box::new(first())];
        let _ = averaged.draw(&state, &renderer, &iced::Theme::Light, bounds, cursor);
        assert_eq!(
            first_draws.get(),
            3,
            "moving-average membership also invalidates the static layer"
        );
        assert_eq!(first_ranges.get(), 3);

        let mut recoloured = hover_program(&data);
        recoloured.theme.palette.accent = Color::from_rgb8(1, 2, 3);
        recoloured.moving_averages = vec![20];
        recoloured.overlays = vec![Box::new(first())];
        let _ = recoloured.draw(&state, &renderer, &iced::Theme::Light, bounds, cursor);
        assert_eq!(
            first_draws.get(),
            4,
            "a moving-average colour change invalidates its cached geometry"
        );
        assert_eq!(first_ranges.get(), 4);
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
        let columns =
            aggregate_columns(&data, range.clone(), viewport, chrome, &pyramid_for(&data))
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
        assert!(aggregate_columns(&data, 0..100, wide, chrome, &pyramid_for(&data)).is_none());

        let narrow = Viewport {
            from: -0.5,
            to: 799.5,
        };
        assert!(aggregate_columns(&data, 0..100, narrow, chrome, &pyramid_for(&data)).is_some());
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

    fn arrow(named: Named) -> canvas::Event {
        canvas::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modified_key: keyboard::Key::Named(named),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn arrow_keys_walk_candles_and_escape_returns_to_mouse() {
        use iced::widget::canvas::Program as _;

        let data = candles(300);
        let program = hover_program(&data);
        let mut state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);
        let over = mouse::Cursor::Available(Point::new(600.0, 300.0));

        // First arrow starts from the last visible candle.
        let action = program
            .update(&mut state, &arrow(Named::ArrowLeft), bounds, over)
            .expect("arrow acts");
        let (message, _, status) = action.into_inner();
        let hit = message.expect("publishes").expect("hits a candle");
        assert_eq!(hit.index, 298);
        assert_eq!(status, iced::event::Status::Captured);
        assert_eq!(state.key_cursor.get(), Some(298));

        // Walking right again moves forward and republishes.
        let action = program
            .update(&mut state, &arrow(Named::ArrowRight), bounds, over)
            .expect("arrow acts");
        assert_eq!(
            action
                .into_inner()
                .0
                .expect("publishes")
                .expect("hit")
                .index,
            299
        );

        // Right edge clamps.
        program.update(&mut state, &arrow(Named::ArrowRight), bounds, over);
        assert_eq!(state.key_cursor.get(), Some(299));

        // Escape hands control back; arrows without the pointer do nothing.
        program
            .update(&mut state, &arrow(Named::Escape), bounds, over)
            .expect("escape acts");
        assert_eq!(state.key_cursor.get(), None);
        assert!(
            program
                .update(
                    &mut state,
                    &arrow(Named::ArrowLeft),
                    bounds,
                    mouse::Cursor::Unavailable,
                )
                .is_none()
        );

        // Mouse movement also clears an active keyboard cursor.
        state.key_cursor.set(Some(200));
        program.update(
            &mut state,
            &moved(Point::new(601.0, 300.0)),
            bounds,
            mouse::Cursor::Available(Point::new(601.0, 300.0)),
        );
        assert_eq!(state.key_cursor.get(), None);
    }

    /// Timing evidence for the cached-layer design, not a pass/fail test.
    /// Run with:
    /// `cargo test -p ui-lang-components --release --features candle-chart,tiny-skia,x11 \`
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
                on_reach_start: None,
                initial_bars: DEFAULT_BARS,
                precision: None,
                time_offset_secs: 0,
                moving_averages: Vec::new(),
                overlays: Vec::new(),
                live: None,
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
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_cached_draw_borrows_derived_axes() {
        use iced::advanced::renderer::Headless as _;
        use iced::widget::canvas::Program as _;

        const DRAWS: usize = 100;
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            Pixels(14.0),
            Some("tiny-skia"),
        ))
        .expect("headless tiny-skia renderer");
        let data = candles(1_000);
        let draws = Rc::new(Cell::new(0));
        let ranges = Rc::new(Cell::new(0));
        let mut program = hover_program(&data);
        program.overlays.push(Box::new(CountedOverlay {
            stamp: 1,
            range: (1_000.0, 1_050.0),
            draws: Rc::clone(&draws),
            ranges: Rc::clone(&ranges),
        }));
        let state = CandleState::default();
        let bounds = Rectangle::with_size(BOUNDS);
        let draw = || {
            drop(std::hint::black_box(program.draw(
                &state,
                &renderer,
                &iced::Theme::Light,
                bounds,
                mouse::Cursor::Unavailable,
            )));
        };

        draw();
        assert_eq!((draws.get(), ranges.get()), (1, 1));

        let region = Region::new(GLOBAL);
        for _ in 0..DRAWS {
            draw();
        }
        let stats = region.change();

        eprintln!(
            "{DRAWS} cached candle draws: {} allocations / {} reallocations / {} bytes",
            stats.allocations, stats.reallocations, stats.bytes_allocated
        );
        assert_eq!(
            (draws.get(), ranges.get()),
            (1, 1),
            "all draws hit the cache"
        );
        // The test process can charge two one-off renderer allocations to this region.
        // The previous axes-cloning path still fails at 2,000 allocations.
        assert!(stats.allocations <= 1_802, "{stats:?}");
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
        let base = fingerprint(&data, 0, viewport, size, &palette);
        let mut appended = data.clone();
        appended.push(Candle {
            ts: 1_700_000_000 + 10 * 86_400,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 10.0,
        });
        assert_ne!(base, fingerprint(&appended, 0, viewport, size, &palette));

        let mut ticked = data.clone();
        ticked[9].close += 0.01;
        assert_ne!(base, fingerprint(&ticked, 0, viewport, size, &palette));

        assert_ne!(
            base,
            fingerprint(
                &data,
                0,
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
            fingerprint(&data, 0, viewport, Size::new(1280.0, 640.0), &palette),
        );

        let dark = super::super::theme::DARK.palette;
        assert_ne!(base, fingerprint(&data, 0, viewport, size, &dark));
    }
}
