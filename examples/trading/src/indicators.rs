use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};

use iced::widget::canvas::{self, Path};
use iced::{Color, Point};
use ui_lang_components::ui::candle_chart::{Candle, ChartCoords, ChartOverlay};
use ui_lang_components::ui::theme;

use crate::ChartIndicator;

const PERIOD: usize = 20;
const BOLLINGER_DEVIATIONS: f64 = 2.0;
const STROKE_WIDTH: f32 = 1.25;
type Values = Vec<Option<f64>>;
type Bands = (Values, Values, Values);

pub fn chart_indicator_active(indicators: &[ChartIndicator], target: ChartIndicator) -> bool {
    indicators.contains(&target)
}

pub fn chart_indicator_name(indicator: ChartIndicator) -> String {
    match indicator {
        ChartIndicator::Sma20 => "SMA 20",
        ChartIndicator::Sma60 => "SMA 60",
        ChartIndicator::Ema20 => "EMA 20",
        ChartIndicator::Bollinger20 => "BB 20 / 2σ",
        ChartIndicator::Vwma20 => "VWMA 20",
    }
    .to_owned()
}

pub fn chart_indicator_action(indicator: ChartIndicator, active: bool) -> String {
    let action = if active { "Hide" } else { "Show" };
    let name = match indicator {
        ChartIndicator::Sma20 => "the SMA 20 indicator",
        ChartIndicator::Sma60 => "the SMA 60 indicator",
        ChartIndicator::Ema20 => "the EMA 20 indicator",
        ChartIndicator::Bollinger20 => "the Bollinger Bands 20, two standard deviations indicator",
        ChartIndicator::Vwma20 => "the VWMA 20 indicator",
    };
    format!("{action} {name}")
}

pub fn chart_indicator_picker_label(indicators: &[ChartIndicator]) -> String {
    format!("Choose chart indicators, {} selected", indicators.len())
}

pub fn focus_chart_indicators(window: Option<iced::window::Id>) -> iced::Task<()> {
    let Some(window) = window else {
        return iced::Task::none();
    };
    // Mounted daemon views qualify every logical widget ID by window. Ice app
    // handlers cannot name that scope directly, so restore this one launcher
    // through the same rendered path the accessibility test observes.
    let target = iced::widget::Id::from(format!(
        "Trading/{window:?}/app/terminal-fit/trade/chart-bar/indicators"
    ));
    iced::widget::operation::focus(target)
}

pub fn toggle_chart_indicator(
    mut indicators: Vec<ChartIndicator>,
    target: ChartIndicator,
) -> Vec<ChartIndicator> {
    if let Some(index) = indicators.iter().position(|indicator| *indicator == target) {
        indicators.remove(index);
    } else {
        indicators.push(target);
        indicators.sort_by_key(|indicator| indicator_rank(*indicator));
    }
    indicators
}

pub fn chart_indicator_summary(indicators: &[ChartIndicator]) -> String {
    indicators
        .iter()
        .copied()
        .map(chart_indicator_name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn indicator_rank(indicator: ChartIndicator) -> u8 {
    match indicator {
        ChartIndicator::Sma20 => 0,
        ChartIndicator::Sma60 => 1,
        ChartIndicator::Ema20 => 2,
        ChartIndicator::Bollinger20 => 3,
        ChartIndicator::Vwma20 => 4,
    }
}

pub struct ChartIndicators {
    active: Vec<ChartIndicator>,
    colors: IndicatorColors,
    stamp: u64,
    // Autoscale and static drawing run back-to-back under one candle lock.
    // Hand the computed series across that boundary once, then discard it.
    prepared: RefCell<Option<Vec<Series>>>,
}

#[derive(Clone, Copy)]
struct IndicatorColors {
    sma_20: Color,
    sma_60: Color,
    ema_20: Color,
    bollinger_20: Color,
    vwma_20: Color,
}

impl IndicatorColors {
    fn from_palette(palette: &theme::Palette) -> Self {
        Self {
            sma_20: palette.accent,
            sma_60: palette.warning,
            ema_20: palette.brand,
            bollinger_20: palette.control_line,
            vwma_20: palette.primary,
        }
    }

    fn get(self, indicator: ChartIndicator) -> Color {
        match indicator {
            ChartIndicator::Sma20 => self.sma_20,
            ChartIndicator::Sma60 => self.sma_60,
            ChartIndicator::Ema20 => self.ema_20,
            ChartIndicator::Bollinger20 => self.bollinger_20,
            ChartIndicator::Vwma20 => self.vwma_20,
        }
    }
}

impl ChartIndicators {
    pub fn new(active: &[ChartIndicator], palette: &theme::Palette) -> Self {
        let colors = IndicatorColors::from_palette(palette);
        let mut hasher = DefaultHasher::new();
        for indicator in active {
            indicator_rank(*indicator).hash(&mut hasher);
            let color = colors.get(*indicator);
            color.r.to_bits().hash(&mut hasher);
            color.g.to_bits().hash(&mut hasher);
            color.b.to_bits().hash(&mut hasher);
            color.a.to_bits().hash(&mut hasher);
        }
        Self {
            active: active.to_vec(),
            colors,
            stamp: hasher.finish(),
            prepared: RefCell::new(None),
        }
    }
}

impl ChartOverlay for ChartIndicators {
    fn price_range(
        &self,
        candles: &[Candle],
        visible: std::ops::Range<usize>,
    ) -> Option<(f64, f64)> {
        if visible.is_empty() {
            return None;
        }
        let series = self.series(candles);
        let mut bounds: Option<(f64, f64)> = None;
        for line in &series {
            for value in line.values[visible.clone()]
                .iter()
                .flatten()
                .copied()
                .filter(|value| value.is_finite())
            {
                bounds = Some(match bounds {
                    Some((low, high)) => (low.min(value), high.max(value)),
                    None => (value, value),
                });
            }
        }
        *self.prepared.borrow_mut() = Some(series);
        bounds
    }

    fn draw(&self, frame: &mut canvas::Frame, coords: &ChartCoords<'_>) {
        let visible = coords.visible();
        if visible.is_empty() {
            return;
        }

        let series = self.series_for_draw(coords.candles());
        if series.is_empty() {
            return;
        }

        let max_points = coords.plot().width.max(1.0) as usize;
        let step = visible.len().div_ceil(max_points).max(1);
        let mut indices: Vec<usize> = visible.clone().step_by(step).collect();
        let last = visible.end - 1;
        if indices.last().copied() != Some(last) {
            indices.push(last);
        }

        frame.with_clip(coords.plot(), |frame| {
            stroke_series(
                series,
                &indices,
                |index, value| {
                    Point::new(coords.x_for_index(index as f64), coords.y_for_price(value))
                },
                |path, color, width| {
                    frame.stroke(
                        path,
                        canvas::Stroke::default()
                            .with_color(color)
                            .with_width(width),
                    );
                },
            );
        });
    }

    fn stamp(&self) -> u64 {
        self.stamp
    }
}

struct Series {
    values: Values,
    color: Color,
    width: f32,
}

fn stroke_series(
    series: Vec<Series>,
    indices: &[usize],
    mut point: impl FnMut(usize, f64) -> Point,
    mut stroke: impl FnMut(&Path, Color, f32),
) {
    for line in series {
        let mut segments = 0;
        let path = Path::new(|builder| {
            let mut drawing = false;
            let mut previous = None;
            for index in indices {
                if let Some(previous) = previous
                    && line.values[previous + 1..=*index]
                        .iter()
                        .any(Option::is_none)
                {
                    drawing = false;
                }
                let Some(value) = line.values[*index] else {
                    drawing = false;
                    previous = Some(*index);
                    continue;
                };
                let at = point(*index, value);
                if drawing {
                    builder.line_to(at);
                    segments += 1;
                } else {
                    builder.move_to(at);
                    drawing = true;
                }
                previous = Some(*index);
            }
        });
        if segments > 0 {
            stroke(&path, line.color, line.width);
        }
    }
}

impl ChartIndicators {
    fn series_for_draw(&self, candles: &[Candle]) -> Vec<Series> {
        self.prepared
            .borrow_mut()
            .take()
            .unwrap_or_else(|| self.series(candles))
    }

    fn series(&self, candles: &[Candle]) -> Vec<Series> {
        let mut series = Vec::new();
        for indicator in &self.active {
            let color = self.colors.get(*indicator);
            match indicator {
                ChartIndicator::Sma20 => series.push(Series {
                    values: simple_moving_average(candles, 20),
                    color,
                    width: STROKE_WIDTH,
                }),
                ChartIndicator::Sma60 => series.push(Series {
                    values: simple_moving_average(candles, 60),
                    color,
                    width: STROKE_WIDTH,
                }),
                ChartIndicator::Ema20 => series.push(Series {
                    values: exponential_moving_average(candles, PERIOD),
                    color,
                    width: STROKE_WIDTH,
                }),
                ChartIndicator::Bollinger20 => {
                    let (middle, upper, lower) =
                        bollinger_bands(candles, PERIOD, BOLLINGER_DEVIATIONS);
                    series.push(Series {
                        values: upper,
                        color,
                        width: STROKE_WIDTH,
                    });
                    series.push(Series {
                        values: middle,
                        color: Color { a: 0.55, ..color },
                        width: 1.0,
                    });
                    series.push(Series {
                        values: lower,
                        color,
                        width: STROKE_WIDTH,
                    });
                }
                ChartIndicator::Vwma20 => series.push(Series {
                    values: volume_weighted_moving_average(candles, PERIOD),
                    color,
                    width: STROKE_WIDTH,
                }),
            }
        }
        series
    }
}

fn simple_moving_average(candles: &[Candle], period: usize) -> Values {
    let mut values = vec![None; candles.len()];
    if period == 0 {
        return values;
    }
    let mut sum = 0.0;
    for (index, candle) in candles.iter().enumerate() {
        sum += candle.close;
        if index >= period {
            sum -= candles[index - period].close;
        }
        if index + 1 >= period {
            values[index] = Some(sum / period as f64);
        }
    }
    values
}

fn exponential_moving_average(candles: &[Candle], period: usize) -> Values {
    let mut values = vec![None; candles.len()];
    if period == 0 || candles.len() < period {
        return values;
    }
    let seed = candles[..period]
        .iter()
        .map(|candle| candle.close)
        .sum::<f64>()
        / period as f64;
    values[period - 1] = Some(seed);
    let multiplier = 2.0 / (period + 1) as f64;
    let mut previous = seed;
    for index in period..candles.len() {
        previous = (candles[index].close - previous) * multiplier + previous;
        values[index] = Some(previous);
    }
    values
}

fn bollinger_bands(candles: &[Candle], period: usize, deviations: f64) -> Bands {
    let middle = simple_moving_average(candles, period);
    let mut upper = vec![None; candles.len()];
    let mut lower = vec![None; candles.len()];
    if period == 0 || candles.len() < period {
        return (middle, upper, lower);
    }
    for index in period - 1..candles.len() {
        let mean = middle[index].expect("the complete window has a mean");
        let variance = candles[index + 1 - period..=index]
            .iter()
            .map(|candle| (candle.close - mean).powi(2))
            .sum::<f64>()
            / period as f64;
        let deviation = variance.sqrt();
        upper[index] = Some(mean + deviations * deviation);
        lower[index] = Some(mean - deviations * deviation);
    }
    (middle, upper, lower)
}

fn volume_weighted_moving_average(candles: &[Candle], period: usize) -> Values {
    let mut values = vec![None; candles.len()];
    if period == 0 {
        return values;
    }
    let mut weighted = 0.0;
    let mut volume = 0.0;
    let mut samples = 0;
    for (index, candle) in candles.iter().enumerate() {
        weighted += candle.close * candle.volume;
        volume += candle.volume;
        samples += usize::from(candle.volume > 0.0);
        if index >= period {
            let dropped = candles[index - period];
            weighted -= dropped.close * dropped.volume;
            volume -= dropped.volume;
            samples -= usize::from(dropped.volume > 0.0);
        }
        if samples == 0 {
            weighted = 0.0;
            volume = 0.0;
        } else if index + 1 >= period && volume > 0.0 {
            values[index] = Some(weighted / volume);
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles() -> Vec<Candle> {
        [(10.0, 1.0), (10.0, 1.0), (14.0, 3.0)]
            .into_iter()
            .enumerate()
            .map(|(index, (close, volume))| Candle {
                ts: index as i64,
                open: close,
                high: close,
                low: close,
                close,
                volume,
            })
            .collect()
    }

    fn long_candles() -> Vec<Candle> {
        (0..80)
            .map(|index| {
                let close = 100.0 + (index % 7) as f64;
                Candle {
                    ts: index,
                    open: close,
                    high: close,
                    low: close,
                    close,
                    volume: index as f64 + 1.0,
                }
            })
            .collect()
    }

    fn tail(values: Vec<Option<f64>>) -> f64 {
        values.last().copied().flatten().unwrap()
    }

    #[test]
    fn chart_indicator_math_uses_the_requested_window_and_volume() {
        let candles = candles();
        assert_eq!(tail(simple_moving_average(&candles, 2)), 12.0);
        assert!((tail(exponential_moving_average(&candles, 2)) - 12.666_666_666_7).abs() < 1e-9);

        let (middle, upper, lower) = bollinger_bands(&candles, 2, 2.0);
        assert_eq!(tail(middle), 12.0);
        assert_eq!(tail(upper), 16.0);
        assert_eq!(tail(lower), 8.0);
        assert_eq!(tail(volume_weighted_moving_average(&candles, 2)), 13.0);
    }

    #[test]
    fn vwma_waits_for_actual_volume() {
        let candles = candles()
            .into_iter()
            .map(|candle| Candle {
                volume: 0.0,
                ..candle
            })
            .collect::<Vec<_>>();
        assert_eq!(volume_weighted_moving_average(&candles, 2), vec![None; 3]);

        let decimal_then_empty = [(95.5, 0.1), (100.1, 0.2), (101.0, 0.0), (102.0, 0.0)]
            .into_iter()
            .enumerate()
            .map(|(index, (close, volume))| Candle {
                ts: index as i64,
                open: close,
                high: close,
                low: close,
                close,
                volume,
            })
            .collect::<Vec<_>>();
        let values = volume_weighted_moving_average(&decimal_then_empty, 2);
        assert!((values[1].expect("first full window") - 98.566_666_666_666_66).abs() < 1e-12);
        assert!((values[2].expect("one positive-volume sample remains") - 100.1).abs() < 1e-12);
        assert_eq!(
            values[3], None,
            "floating-point residue cannot invent volume in an empty window"
        );
    }

    #[test]
    fn active_membership_changes_the_stamp_and_every_series_is_stroked() {
        let palette = theme::DARK.palette;
        let defaults =
            ChartIndicators::new(&[ChartIndicator::Sma20, ChartIndicator::Sma60], &palette);
        let all = ChartIndicators::new(
            &[
                ChartIndicator::Sma20,
                ChartIndicator::Sma60,
                ChartIndicator::Ema20,
                ChartIndicator::Bollinger20,
                ChartIndicator::Vwma20,
            ],
            &palette,
        );
        assert_ne!(defaults.stamp(), all.stamp());
        let candles = long_candles();
        let indices = (0..candles.len()).collect::<Vec<_>>();
        let mut strokes = 0;
        stroke_series(
            all.series_for_draw(&candles),
            &indices,
            |index, value| Point::new(index as f32, value as f32),
            |_, _, _| strokes += 1,
        );
        assert_eq!(strokes, 7, "every configured line reaches the stroke sink");
    }

    #[test]
    fn downsampling_does_not_bridge_an_undefined_indicator_gap() {
        let series = vec![Series {
            values: vec![Some(1.0), None, Some(2.0)],
            color: Color::WHITE,
            width: 1.0,
        }];
        let mut strokes = 0;

        stroke_series(
            series,
            &[0, 2],
            |index, value| Point::new(index as f32, value as f32),
            |_, _, _| strokes += 1,
        );

        assert_eq!(strokes, 0, "an undefined skipped point breaks the line");
    }

    #[test]
    fn price_range_prepares_each_series_once_for_the_following_draw() {
        let candles = long_candles();
        let active = [
            ChartIndicator::Sma20,
            ChartIndicator::Sma60,
            ChartIndicator::Ema20,
            ChartIndicator::Bollinger20,
            ChartIndicator::Vwma20,
        ];
        let overlay = ChartIndicators::new(&active, &theme::DARK.palette);

        let _ = overlay.price_range(&candles, 60..80);
        let prepared = overlay.prepared.borrow();
        let prepared_ptr = prepared
            .as_ref()
            .expect("autoscale prepares the indicator series")
            .as_ptr();
        drop(prepared);

        let series = overlay.series_for_draw(&candles);
        assert_eq!(series.len(), 7);
        assert_eq!(
            series.as_ptr(),
            prepared_ptr,
            "draw reuses the allocation prepared during autoscale"
        );
        assert!(
            overlay.prepared.borrow().is_none(),
            "prepared series is a one-shot handoff, not a persistent cache"
        );
    }

    #[test]
    fn every_active_study_contributes_to_price_range_after_a_gap() {
        let mut candles = (0..60)
            .map(|index| Candle {
                ts: index,
                open: 100.0,
                high: 100.0,
                low: 100.0,
                close: 100.0,
                volume: 1.0,
            })
            .collect::<Vec<_>>();
        for (index, candle) in candles.iter_mut().enumerate().skip(58) {
            *candle = Candle {
                ts: index as i64,
                open: 0.0,
                high: 0.0,
                low: 0.0,
                close: 0.0,
                volume: 1.0,
            };
        }
        let palette = theme::DARK.palette;

        for indicator in [
            ChartIndicator::Sma20,
            ChartIndicator::Sma60,
            ChartIndicator::Ema20,
            ChartIndicator::Bollinger20,
            ChartIndicator::Vwma20,
        ] {
            let bounds = ChartIndicators::new(&[indicator], &palette)
                .price_range(&candles, 58..60)
                .expect("an active study has a visible value");
            assert!(
                bounds.1 > 1.0,
                "{indicator:?} must remain visible above the gapped candle"
            );
        }
    }
}
