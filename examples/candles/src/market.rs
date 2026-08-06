use ducktape_ui::ui::candle_chart::{candle_chart, format_price, format_volume};
use ducktape_ui::ui::theme;
use iced::{Element, Length};
use ui_lang_runtime::{Role, StableId, accessible};

pub use ducktape_ui::ui::candle_chart::{Candle, CandleHit};

fn next(state: &mut u64) -> f64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
}

/// Deterministic hourly OHLCV random walk, seeded so every run draws the
/// same tape.
pub fn gen_candles(count: i64) -> Vec<Candle> {
    let start_ts = 1_735_689_600; // 2025-01-01T00:00:00Z
    let mut seed: u64 = 0x5eed_cafe_f00d_d0d0;
    let mut close = 30_000.0f64;
    (0..count.max(0))
        .map(|index| {
            let open = close;
            let drift_pct = (next(&mut seed) - 0.48) * 1.6;
            close = (open * (1.0 + drift_pct / 100.0)).max(1.0);
            let high = open.max(close) * (1.0 + next(&mut seed) * 0.006);
            let low = open.min(close) * (1.0 - next(&mut seed) * 0.006);
            let volume = (400.0 + next(&mut seed) * 4_600.0) * (1.0 + drift_pct.abs());
            Candle {
                ts: start_ts + index * 3_600,
                open,
                high,
                low,
                close,
                volume,
            }
        })
        .collect()
}

/// Advances the tape by one tick: usually nudges the last candle, sometimes
/// rolls a fresh one. Seeded from the evolving tape, so it stays deterministic.
pub fn next_tick(mut candles: Vec<Candle>) -> Vec<Candle> {
    let Some(last) = candles.last_mut() else {
        return candles;
    };
    let mut seed = (last.ts as u64) ^ last.close.to_bits();
    if next(&mut seed) < 0.125 {
        let open = last.close;
        let ts = last.ts + 3_600;
        candles.push(Candle {
            ts,
            open,
            high: open,
            low: open,
            close: open,
            volume: 0.0,
        });
    } else {
        let step = (next(&mut seed) - 0.5) * last.close * 0.003;
        last.close = (last.close + step).max(1.0);
        last.high = last.high.max(last.close);
        last.low = last.low.min(last.close);
        last.volume += next(&mut seed) * 40.0;
    }
    candles
}

pub fn fmt_price(value: f64) -> String {
    format_price(value, 2)
}

pub fn fmt_volume(value: f64) -> String {
    format_volume(value)
}

pub fn chart(candles: &[Candle]) -> Element<'_, Option<CandleHit>> {
    let chart = candle_chart(candles, &theme::DARK)
        .height(Length::Fill)
        .on_hover(|hit| hit);
    let label = match candles.last() {
        Some(last) => format!(
            "DUCK / USD hourly candlestick chart, last {:.2}",
            last.close
        ),
        None => "DUCK / USD hourly candlestick chart".to_owned(),
    };
    accessible(chart, StableId::new("candles-chart"), Role::Image)
        .label(label)
        .logical_id("candles-chart")
        .into()
}
