use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use ducktape_ui::ui::candle_chart::{
    SharedCandles, candle_chart_shared, format_price, format_volume,
};
use ducktape_ui::ui::theme;
use iced::{Element, Length, Subscription};
use ui_lang_runtime::{Role, StableId, accessible};

pub use ducktape_ui::ui::candle_chart::{Candle, CandleHit};

const TICK: Duration = Duration::from_millis(250);
const HISTORY_CAP: usize = 100_000;
const EVICT_CHUNK: usize = 4_096;
const NEW_CANDLE_ODDS: f64 = 1.0 / 16.0;
const OUTAGE_TICKS: u32 = 16; // 4s of dropout at the 250ms tick rate
const START_TS: i64 = 1_735_689_600; // 2025-01-01T00:00:00Z

fn next(state: &mut u64) -> f64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
}

/// A mock exchange connection: the tape lives behind one lock that both the
/// tick subscription and the chart share, so a tick is an in-place O(1)
/// mutation and nothing is copied per frame or per update.
#[derive(Clone)]
pub struct MarketFeed {
    tape: SharedCandles,
    generator: Arc<Mutex<Generator>>,
}

struct Generator {
    seed: u64,
    interval_secs: i64,
    revision: i64,
    /// Prices mean-revert toward this level so a long-running tape never
    /// compounds off to absurd magnitudes.
    anchor: f64,
    /// Ticks until the next simulated disconnect.
    outage_in: u32,
    /// Remaining ticks of the current disconnect (0 = connected).
    outage_left: u32,
}

impl Generator {
    fn schedule_next_outage(&mut self) {
        self.outage_in = 150 + (next(&mut self.seed) * 100.0) as u32;
    }
}

impl PartialEq for MarketFeed {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.tape, &other.tape)
    }
}

impl std::hash::Hash for MarketFeed {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.tape), state);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// "REST backfill": builds `history` candles of deterministic random walk,
/// seeded from the symbol so each symbol has its own tape.
pub fn market_connect(symbol: String, interval_secs: i64, history: i64) -> MarketFeed {
    let mut seed = symbol.bytes().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    let interval_secs = interval_secs.max(1);
    let mut close = 20_000.0 + next(&mut seed) * 40_000.0;
    let anchor = close;
    let candles = (0..history.max(1))
        .map(|index| {
            let open = close;
            let drift_pct = (next(&mut seed) - 0.5) * 1.6;
            let pull = (anchor - close) / anchor * 0.004;
            close = (open * (1.0 + drift_pct / 100.0 + pull)).max(1.0);
            Candle {
                ts: START_TS + index * interval_secs,
                open,
                high: open.max(close) * (1.0 + next(&mut seed) * 0.006),
                low: open.min(close) * (1.0 - next(&mut seed) * 0.006),
                close,
                volume: (400.0 + next(&mut seed) * 4_600.0) * (1.0 + drift_pct.abs()),
            }
        })
        .collect();
    MarketFeed {
        tape: Arc::new(Mutex::new(candles)),
        generator: Arc::new(Mutex::new({
            let mut generator = Generator {
                seed,
                interval_secs,
                revision: 0,
                anchor,
                outage_in: 0,
                outage_left: 0,
            };
            generator.schedule_next_outage();
            generator
        })),
    }
}

/// One market tick applied to the shared tape, reported to Ice as a
/// lightweight notice; the candles themselves never cross the boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub revision: i64,
    pub last: f64,
    pub up: bool,
    pub connected: bool,
}

/// One market advance of the tape (a mid-candle update or a roll).
fn advance(generator: &mut Generator, tape: &mut Vec<Candle>) {
    let Some(last) = tape.last().copied() else {
        return;
    };
    if next(&mut generator.seed) < NEW_CANDLE_ODDS {
        let open = last.close;
        tape.push(Candle {
            ts: last.ts + generator.interval_secs,
            open,
            high: open,
            low: open,
            close: open,
            volume: 0.0,
        });
        if tape.len() > HISTORY_CAP {
            tape.drain(..EVICT_CHUNK);
        }
    } else {
        let pull = (generator.anchor - last.close) / generator.anchor * 0.0008;
        let step = ((next(&mut generator.seed) - 0.5) * 0.003 + pull) * last.close;
        let entry = tape.last_mut().expect("tape is non-empty");
        entry.close = (entry.close + step).max(1.0);
        entry.high = entry.high.max(entry.close);
        entry.low = entry.low.min(entry.close);
        entry.volume += next(&mut generator.seed) * 40.0;
    }
}

fn apply_tick(feed: &MarketFeed) -> Tick {
    let mut generator = lock(&feed.generator);
    let mut tape = lock(&feed.tape);
    generator.revision += 1;

    // Simulated disconnect: the market keeps moving, we just do not see it
    // until the reconnect backfills every missed advance in one burst.
    if generator.outage_left > 0 {
        generator.outage_left -= 1;
        if generator.outage_left == 0 {
            let missed = OUTAGE_TICKS;
            for _ in 0..missed {
                advance(&mut generator, &mut tape);
            }
            generator.schedule_next_outage();
        } else {
            let last = tape.last().map_or(0.0, |candle| candle.close);
            return Tick {
                revision: generator.revision,
                last,
                up: true,
                connected: false,
            };
        }
    } else if generator.outage_in == 0 {
        generator.outage_left = OUTAGE_TICKS;
        let last = tape.last().map_or(0.0, |candle| candle.close);
        return Tick {
            revision: generator.revision,
            last,
            up: true,
            connected: false,
        };
    } else {
        generator.outage_in -= 1;
        advance(&mut generator, &mut tape);
    }

    let Some(entry) = tape.last().copied() else {
        return Tick {
            revision: generator.revision,
            last: 0.0,
            up: true,
            connected: true,
        };
    };
    Tick {
        revision: generator.revision,
        last: entry.close,
        up: entry.close >= entry.open,
        connected: true,
    }
}

/// The "WebSocket": a timer-driven subscription that advances the shared
/// tape and emits a notice per tick.
pub fn market_events(feed: MarketFeed) -> Subscription<Tick> {
    iced::time::every(TICK)
        .with(feed)
        .map(|(feed, _)| apply_tick(&feed))
}

pub fn fmt_price(value: f64) -> String {
    format_price(value, 2)
}

pub fn fmt_volume(value: f64) -> String {
    format_volume(value)
}

pub fn chart(feed: &MarketFeed) -> Element<'static, Option<CandleHit>> {
    let chart = candle_chart_shared(feed.tape.clone(), &theme::DARK)
        .height(Length::Fill)
        .moving_averages([20, 60])
        .on_hover(|hit| hit);
    accessible(chart, StableId::new("candles-chart"), Role::Image)
        .label("Mock market candlestick chart")
        .logical_id("candles-chart")
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_mutate_in_place_and_evict_at_cap() {
        let feed = market_connect("TEST".into(), 60, 100);
        let before_ptr = Arc::as_ptr(&feed.tape);
        let len_before = lock(&feed.tape).len();
        let tick = apply_tick(&feed);
        assert_eq!(tick.revision, 1);
        assert!(tick.last > 0.0);
        assert_eq!(Arc::as_ptr(&feed.tape), before_ptr);
        let len_after = lock(&feed.tape).len();
        assert!(len_after == len_before || len_after == len_before + 1);

        lock(&feed.tape).resize(
            HISTORY_CAP + 1,
            Candle {
                ts: 0,
                open: 1.0,
                high: 1.0,
                low: 1.0,
                close: 1.0,
                volume: 0.0,
            },
        );
        for _ in 0..64 {
            apply_tick(&feed);
            assert!(lock(&feed.tape).len() <= HISTORY_CAP + 1);
        }
    }

    #[test]
    fn outage_freezes_the_tape_then_backfills_continuously() {
        let feed = market_connect("TEST".into(), 60, 100);
        // Fast-forward to the scheduled outage deterministically.
        let mut guard_ticks = 0;
        loop {
            let before = lock(&feed.tape).clone();
            let tick = apply_tick(&feed);
            if !tick.connected {
                // Disconnected: the tape must not have advanced.
                assert_eq!(*lock(&feed.tape), before);
                break;
            }
            guard_ticks += 1;
            assert!(guard_ticks < 1_000, "outage never started");
        }
        // Ride out the outage; every tick stays disconnected and frozen.
        let frozen = lock(&feed.tape).clone();
        let mut reconnect = None;
        for _ in 0..OUTAGE_TICKS + 2 {
            let tick = apply_tick(&feed);
            if tick.connected {
                reconnect = Some(tick);
                break;
            }
            assert_eq!(*lock(&feed.tape), frozen);
        }
        let reconnect = reconnect.expect("feed reconnects after the outage");
        assert!(reconnect.connected);
        // The backfill advanced the tape and timestamps stay continuous.
        let tape = lock(&feed.tape);
        assert!(tape.len() >= frozen.len());
        assert!(tape.last().unwrap() != frozen.last().unwrap() || tape.len() > frozen.len());
        for pair in tape.windows(2) {
            assert_eq!(pair[1].ts - pair[0].ts, 60, "gap in backfilled tape");
        }
    }

    #[test]
    fn symbols_get_distinct_deterministic_tapes() {
        let first = market_connect("DUCK-USD".into(), 60, 50);
        let second = market_connect("DUCK-USD".into(), 60, 50);
        let other = market_connect("TAPE-KRW".into(), 60, 50);
        assert_eq!(*lock(&first.tape), *lock(&second.tape));
        assert_ne!(*lock(&first.tape), *lock(&other.tape));
    }
}
