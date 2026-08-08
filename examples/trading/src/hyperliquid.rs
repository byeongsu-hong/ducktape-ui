//! The Hyperliquid API behind two shapes: the `info` endpoint as one blocking
//! POST moved off the UI thread with `smol::unblock`, and the websocket as a
//! thread per feed pushing into a channel Ice reads as a stream. Every
//! response is read as a `Value` and mapped by hand: the exchange sends all
//! numbers as JSON strings, so a derive would need a custom deserializer per
//! field anyway.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ducktape_ui::ui::candle_chart::{
    ChartMarker, MarkerShape, PriceLine, SharedCandles, candle_chart_shared, format_price,
    format_volume,
};
use ducktape_ui::ui::theme;
use iced::{Color, Element, Font, Length};
use serde_json::{Value, json};
use smol::channel::{Receiver, Sender};
use tungstenite::stream::MaybeTlsStream;
use ui_lang_runtime::{Role, StableId, accessible};

pub use ducktape_ui::ui::candle_chart::{Candle, CandleHit};

const INFO_URL: &str = "https://api.hyperliquid.xyz/info";
const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const TIMEOUT: Duration = Duration::from_secs(15);
/// Candles fetched when a market is opened, and when the chart is panned back
/// past the oldest one it holds.
const BACKFILL_BARS: i64 = 500;
const REFRESH_BARS: i64 = 3;
/// The exchange closes a socket that goes quiet for a minute. The pong that
/// answers each ping is also the only honest latency reading available: a
/// round trip needs no agreement between our clock and theirs.
const PING: Duration = Duration::from_secs(15);
/// How long a read blocks before the loop looks at the clock and at which
/// market the app is showing.
const POLL: Duration = Duration::from_millis(200);
/// Feed traffic coalesces into at most one app message per beat. The book of
/// a busy market reprints far faster than a screen can show it, and the chart
/// repaints off the shared tape on this same beat without any message at all.
const BEAT: Duration = Duration::from_millis(100);
/// Pause before a dropped socket is reopened.
const RETRY: Duration = Duration::from_secs(2);
/// Decay steps a freshly printed fill flashes for.
const FLASH_STEPS: i64 = 2;
/// Book levels shown per side, and the pixel width of a full depth bar.
const BOOK_DEPTH: usize = 10;
const BOOK_BAR_WIDTH: f64 = 196.0;
/// What the lower panel can be dragged between: enough for a position row,
/// and never so tall that the chart is gone.
const LOWER_MIN: f64 = 120.0;
const LOWER_MAX: f64 = 560.0;
/// Pixel width of the risk rail drawn under a position's liquidation price.
const RISK_RAIL_WIDTH: f64 = 80.0;

fn rgb(hex: u32) -> Color {
    Color::from_rgb8(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

/// The chart wears the terminal's palette rather than the library default:
/// a cold near-black ground, monochrome moving averages, and colour reserved
/// for the two things that mean money moved.
fn chart_theme() -> theme::Theme {
    let mut chart = theme::DARK;
    chart.palette.background = rgb(0x14_12_0f);
    chart.palette.foreground = rgb(0xed_e8_df);
    chart.palette.muted_foreground = rgb(0x93_89_7c);
    chart.palette.border = rgb(0x2f_28_23);
    chart.palette.success = rgb(0x5f_ae_7e);
    chart.palette.destructive = rgb(0xd0_64_5a);
    // The moving-average slots; ink rather than colour, so the fills and the
    // position levels are the only long/short marks on the plot.
    chart.palette.accent = rgb(0x4a_42_3b);
    chart.palette.warning = rgb(0x6b_61_57);
    chart.typography.font = Font::with_name("Monoplex KR");
    chart
}

/// One HTTP agent for the process: connection reuse plus a global timeout, so
/// a stalled request cannot wedge the polling loop forever.
fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .build()
            .into()
    })
}

/// Everything the exchange can tell us goes through this one endpoint.
async fn info(body: Value) -> Result<Value, HlError> {
    smol::unblock(move || {
        let mut response = agent()
            .post(INFO_URL)
            .send_json(&body)
            .map_err(|error| HlError::new(format!("Hyperliquid unreachable: {error}")))?;
        response
            .body_mut()
            .read_json::<Value>()
            .map_err(|error| HlError::new(format!("Hyperliquid sent bad JSON: {error}")))
    })
    .await
}

/// Prices, sizes, and PnL all arrive as strings. A missing or unparsable
/// field reads as zero rather than failing the whole response.
fn num(value: &Value, key: &str) -> f64 {
    match value.get(key) {
        Some(Value::String(text)) => text.parse().unwrap_or(0.0),
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Candle width in seconds for the intervals this app offers.
fn interval_secs(interval: &str) -> i64 {
    match interval {
        "1m" => 60,
        "5m" => 300,
        "15m" => 900,
        "1h" => 3_600,
        "4h" => 14_400,
        _ => 86_400,
    }
}

#[derive(Clone, Debug)]
pub struct HlError {
    pub message: String,
}

impl HlError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

/// One tradeable perp with its current context.
///
/// `Hash` is hand-written because every number here is an `f64`, which does not
/// implement it. A market row is the dependency of a `lazy` boundary in the
/// view, so it needs an identity that changes exactly when the rendered row
/// changes — hashing the bits is precisely that, once negative zero is folded
/// onto zero so two rows that render identically also cache identically.
#[derive(Clone, PartialEq)]
pub struct SymbolRow {
    pub name: String,
    pub price: f64,
    pub change_pct: f64,
    pub volume: f64,
    pub funding_pct: f64,
    pub leverage: f64,
    pub open_interest: f64,
    /// Yesterday's close, kept so a streamed mid price can be turned back
    /// into a 24h change without another request.
    pub prev: f64,
    /// The share of a position's value the margin engine holds against it.
    /// The only figure in the ticket's arithmetic that belongs to a venue
    /// rather than to arithmetic, so the venue publishes it per market and
    /// the shared math never learns one exchange's rule.
    pub maintenance: f64,
    /// Whether this row is the market on screen. Carried on the row rather than
    /// read from app state beside it so the row is the whole dependency of its
    /// own subtree, which is what lets the view cache it.
    pub selected: bool,
}

/// `-0.0` and `0.0` are the same price and must hash alike; `NaN` never
/// compares equal, so its bits are as good an identity as anything.
fn hash_f64(value: f64, state: &mut impl Hasher) {
    let value = if value == 0.0 { 0.0 } else { value };
    value.to_bits().hash(state);
}

impl Hash for SymbolRow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.selected.hash(state);
        for value in [
            self.price,
            self.change_pct,
            self.volume,
            self.funding_pct,
            self.leverage,
            self.open_interest,
            self.prev,
            self.maintenance,
        ] {
            hash_f64(value, state);
        }
    }
}

/// One open position, shaped the way the official app reads it out.
#[derive(Clone, PartialEq)]
pub struct Position {
    pub coin: String,
    pub size: f64,
    pub entry: f64,
    pub mark: f64,
    pub liq: f64,
    pub pnl: f64,
    pub roe_pct: f64,
    pub margin: f64,
    /// How far the mark has travelled from entry toward liquidation, already
    /// scaled to the rail's pixel width. Zero when there is no cliff to run at.
    pub risk: f64,
    pub leverage: f64,
    pub margin_mode: String,
    pub funding: f64,
}

/// The account summary plus its open positions.
#[derive(Clone, PartialEq)]
pub struct Account {
    pub value: f64,
    /// Equity the cross engine can actually spend: the total less whatever is
    /// posted behind isolated positions. This is what the maintenance
    /// requirement is measured against.
    pub cross_value: f64,
    pub pnl: f64,
    pub withdrawable: f64,
    pub notional: f64,
    pub maintenance: f64,
    /// How far the account's equity has fallen toward its maintenance
    /// requirement, already scaled to the rail's pixel width.
    pub health: f64,
    /// The same reading as a percentage, so the rail's length is also
    /// available as a number the accessibility tree can carry.
    pub margin_pct: f64,
    pub positions: Vec<Position>,
}

/// One print on the public tape: somebody else's trade, which is every
/// trade rather than only this account's.
#[derive(Clone, PartialEq)]
pub struct Trade {
    pub ts: i64,
    pub price: f64,
    pub size: f64,
    /// Which side crossed the spread. The tape reads as a column of these.
    pub buy: bool,
    /// How many prints the exchange filled from one aggressing order. A
    /// market order that eats four resting levels is one event to watch and
    /// four messages on the wire.
    pub sweep: i64,
    /// The exchange's trade id, which is how a repeated message is recognised.
    tid: i64,
}

/// One executed trade, which is what the chart marks.
#[derive(Clone, PartialEq)]
pub struct Fill {
    pub coin: String,
    pub ts: i64,
    pub price: f64,
    pub size: f64,
    pub buy: bool,
    pub closed_pnl: f64,
    /// Decay steps left on the highlight a just-printed fill wears, counted
    /// down to zero by the app. Fills already on the books arrive cold.
    pub heat: i64,
    /// The exchange's trade id, which is how a fill pushed by the feed is
    /// recognised as one the snapshot already listed.
    tid: i64,
}

/// One resting order, listed and drawn on the chart as a level.
#[derive(Clone, PartialEq)]
pub struct Order {
    pub coin: String,
    pub buy: bool,
    pub price: f64,
    pub size: f64,
    pub ts: i64,
}

/// One price level of the book, with the cumulative depth behind it already
/// resolved to a bar width so the view does no arithmetic.
#[derive(Clone, PartialEq)]
pub struct Level {
    pub price: f64,
    pub size: f64,
    pub total: f64,
    pub bar: f64,
}

/// The top of the book for one market.
#[derive(Clone, PartialEq)]
pub struct Book {
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
    pub spread: f64,
    pub spread_pct: f64,
    pub mid: f64,
}

/// The candle tape the chart renders, plus the symbol and interval it holds.
/// Both live behind locks the async fetches take briefly, so the chart reads
/// candles in place and nothing is copied per frame.
#[derive(Clone)]
pub struct Tape {
    candles: SharedCandles,
    focus: Arc<Mutex<String>>,
}

impl PartialEq for Tape {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.candles, &other.candles)
    }
}

fn focus_key(coin: &str, interval: &str) -> String {
    format!("{coin}:{interval}")
}

impl Tape {
    /// The market and interval the tape is currently holding, which is what
    /// the feed subscribes to. Empty until the app opens one.
    fn focus(&self) -> Option<(String, String)> {
        let focus = lock(&self.focus);
        let (coin, interval) = focus.split_once(':')?;
        Some((coin.to_owned(), interval.to_owned()))
    }
}

pub fn tape_new() -> Tape {
    Tape {
        candles: Arc::new(Mutex::new(Vec::new())),
        focus: Arc::new(Mutex::new(String::new())),
    }
}

/// Points the tape at a different market and empties it. Requests already in
/// flight for the old market land on a focus that no longer matches and are
/// dropped, so a fast symbol switch cannot show the previous coin's candles.
pub fn tape_focus(tape: Tape, coin: String, interval: String) -> Tape {
    *lock(&tape.focus) = focus_key(&coin, &interval);
    lock(&tape.candles).clear();
    tape
}

fn parse_candle(value: &Value) -> Candle {
    Candle {
        // Hyperliquid timestamps are milliseconds; the chart wants seconds.
        ts: value_i64(value, "t") / 1_000,
        open: num(value, "o"),
        high: num(value, "h"),
        low: num(value, "l"),
        close: num(value, "c"),
        volume: num(value, "v"),
    }
}

fn parse_candles(value: &Value) -> Vec<Candle> {
    value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(parse_candle)
        .collect()
}

fn value_i64(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default()
}

/// Folds a fresh snapshot into the tape: the live candle is replaced in
/// place, closed ones land in timestamp order.
fn merge(tape: &mut Vec<Candle>, fresh: Vec<Candle>) {
    for candle in fresh {
        match tape.binary_search_by_key(&candle.ts, |held| held.ts) {
            Ok(index) => tape[index] = candle,
            Err(index) => tape.insert(index, candle),
        }
    }
}

/// Brings the tape up to date. An empty tape backfills a full window; a
/// loaded one only asks for the candles that can still have changed, so the
/// caller never has to know which of the two it needs.
pub async fn hl_candles(tape: Tape, coin: String, interval: String) -> Result<i64, HlError> {
    let key = focus_key(&coin, &interval);
    let bars = if lock(&tape.candles).is_empty() {
        BACKFILL_BARS
    } else {
        REFRESH_BARS
    };
    let end = now_ms();
    let start = end - bars * interval_secs(&interval) * 1_000;
    let response = info(json!({
        "type": "candleSnapshot",
        "req": { "coin": coin, "interval": interval, "startTime": start, "endTime": end },
    }))
    .await?;

    let mut candles = lock(&tape.candles);
    let mut focus = lock(&tape.focus);
    if focus.is_empty() {
        // A fresh tape belongs to whoever fills it first.
        *focus = key;
    } else if *focus != key {
        // The user moved on while this was in flight.
        return Ok(candles.len() as i64);
    }
    merge(&mut candles, parse_candles(&response));
    Ok(candles.len() as i64)
}

/// Loads the window of candles that ends where the tape currently begins, so
/// a chart panned back to its oldest bar can keep going. Returns the tape
/// length, which is unchanged when the exchange has nothing older to give.
pub async fn hl_history(tape: Tape, coin: String, interval: String) -> Result<i64, HlError> {
    let key = focus_key(&coin, &interval);
    let end = { lock(&tape.candles).first().map(|candle| candle.ts * 1_000) };
    let Some(end) = end else {
        return Ok(0);
    };
    let start = end - BACKFILL_BARS * interval_secs(&interval) * 1_000;
    let response = info(json!({
        "type": "candleSnapshot",
        "req": { "coin": coin, "interval": interval, "startTime": start, "endTime": end },
    }))
    .await?;

    let mut candles = lock(&tape.candles);
    if *lock(&tape.focus) != key {
        // The user moved on while this was in flight.
        return Ok(candles.len() as i64);
    }
    merge(&mut candles, parse_candles(&response));
    Ok(candles.len() as i64)
}

/// A market's move against yesterday's close, as a percentage.
/// Hyperliquid holds half the margin at the market's maximum leverage, so a
/// 40x market maintains at 1/80th of a position's value.
fn maintenance_fraction(max_leverage: f64) -> f64 {
    if max_leverage <= 0.0 {
        return 0.0;
    }
    1.0 / (2.0 * max_leverage)
}

fn change_pct(price: f64, previous: f64) -> f64 {
    if previous > 0.0 {
        (price - previous) / previous * 100.0
    } else {
        0.0
    }
}

fn parse_symbols(value: &Value) -> Vec<SymbolRow> {
    let Some(pair) = value.as_array() else {
        return Vec::new();
    };
    let (Some(meta), Some(contexts)) = (pair.first(), pair.get(1)) else {
        return Vec::new();
    };
    let universe = list(meta, "universe");
    let contexts = contexts.as_array().map(Vec::as_slice).unwrap_or_default();
    let mut rows: Vec<SymbolRow> = universe
        .iter()
        .zip(contexts)
        .filter(|(asset, _)| {
            !asset
                .get("isDelisted")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .map(|(asset, context)| SymbolRow {
            leverage: max_leverage(asset),
            maintenance: maintenance_fraction(max_leverage(asset)),
            ..parse_context(text(asset, "name"), context)
        })
        .collect();
    rows.sort_by(|left, right| right.volume.total_cmp(&left.volume));
    rows
}

/// One market's context, which the universe lists once and the feed
/// republishes for whichever market is on screen. `maxLeverage` is not part
/// of it — it belongs to the asset, not to the day — so it reads as zero here
/// and the merge keeps whatever the universe said.
fn parse_context(name: String, context: &Value) -> SymbolRow {
    let price = num(context, "markPx");
    let previous = num(context, "prevDayPx");
    SymbolRow {
        name,
        price,
        change_pct: change_pct(price, previous),
        volume: num(context, "dayNtlVlm"),
        funding_pct: num(context, "funding") * 100.0,
        // The streamed context does not restate the asset's maximum, so the
        // universe's reading of both is kept by the caller.
        leverage: 0.0,
        maintenance: 0.0,
        open_interest: num(context, "openInterest"),
        prev: previous,
        selected: false,
    }
}

fn max_leverage(asset: &Value) -> f64 {
    asset
        .get("maxLeverage")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

pub async fn hl_symbols() -> Result<Vec<SymbolRow>, HlError> {
    Ok(parse_symbols(
        &info(json!({ "type": "metaAndAssetCtxs" })).await?,
    ))
}

/// What a position opened right now would be worth to the margin engine, and
/// where it would die. Nothing here is sent anywhere: it is the arithmetic a
/// ticket has to show before it is worth calling a ticket, and the same
/// arithmetic a real order would be checked against.
#[derive(Clone, PartialEq)]
pub struct Ticket {
    /// Price times size, which is what leverage divides.
    pub notional: f64,
    /// What the position would tie up at the chosen leverage.
    pub margin: f64,
    /// Where the margin engine would close it, or zero when the inputs do not
    /// yet describe a position.
    pub liquidation: f64,
    /// The leverage these figures were actually priced at, which is what the
    /// market allows rather than what the field says. Typing 40 into a 5x
    /// market has to show a liquidation for 5, and say 5.
    pub leverage: f64,
    /// Whether the numbers above describe anything at all.
    pub ready: bool,
    /// Whether the market's maintenance requirement is known. Without it
    /// there is no cliff to quote, and one computed as though the requirement
    /// were zero sits further from the entry than the real one — so the panel
    /// says it does not know rather than something reassuring.
    pub known: bool,
}

/// Liquidation for a position opened at `price` with `leverage`, isolated.
///
/// Equity is the margin posted plus what the position has made; the engine
/// closes when that reaches the maintenance requirement on the position's
/// current value. Solving for the price where those meet:
///
/// ```text
/// long:   P(1 - 1/L) / (1 - m)
/// short:  P(1 + 1/L) / (1 + m)
/// ```
///
/// with `m` the maintenance fraction. A cross position is not this — it dies
/// against the whole account's equity, which is what the header's rail reads —
/// so this is the isolated case and the ticket says so.
fn ticket_liquidation(price: f64, leverage: f64, maintenance: f64, buy: bool) -> f64 {
    if price <= 0.0 || leverage <= 0.0 {
        return 0.0;
    }
    let liquidation = if buy {
        price * (1.0 - 1.0 / leverage) / (1.0 - maintenance)
    } else {
        price * (1.0 + 1.0 / leverage) / (1.0 + maintenance)
    };
    // Leverage under 1x on a long puts the cliff below zero: there is none.
    if liquidation.is_finite() && liquidation > 0.0 {
        liquidation
    } else {
        0.0
    }
}

/// What to put in the price field when the ticket opens: the book's mid if
/// one has arrived, otherwise the market's last price. A ticket that opens
/// empty makes you type a number you are looking at.
pub fn ticket_seed(book: Option<Book>, focus: Option<SymbolRow>) -> String {
    let price = book
        .map(|depth| depth.mid)
        .filter(|mid| *mid > 0.0)
        .or_else(|| focus.map(|row| row.price))
        .unwrap_or(0.0);
    if price > 0.0 {
        format_price(price, price_decimals(price))
    } else {
        String::new()
    }
}

/// A number as typed into a ticket field. Anything that is not one reads as
/// zero, which every figure downstream already treats as "not yet".
fn amount(typed: &str) -> f64 {
    typed.trim().replace(',', "").parse().map_or(
        0.0,
        |value: f64| if value.is_finite() { value } else { 0.0 },
    )
}

/// Prices a ticket from what the panel has typed into it. Leverage is held
/// inside what the market allows, so a ticket cannot quote a liquidation the
/// exchange would never have opened.
pub fn price_ticket(
    price: String,
    size: String,
    leverage: String,
    market: Option<SymbolRow>,
    buy: bool,
    held: f64,
) -> Ticket {
    let (max_leverage, maintenance) =
        market.map_or((0.0, 0.0), |row| (row.leverage, row.maintenance));
    let price = amount(&price).max(0.0);
    let size = amount(&size).abs();
    let ceiling = if max_leverage > 0.0 {
        max_leverage
    } else {
        f64::INFINITY
    };
    let leverage = amount(&leverage).clamp(0.0, ceiling);
    let notional = price * size;
    // Only the part of the order that is not closing something opens a
    // position, and only an opened position ties up margin or has a cliff.
    // Selling into a long releases margin; quoting a requirement for it, and a
    // liquidation for a position that would not exist, is the panel describing
    // the wrong trade.
    let opening = if held == 0.0 || (held > 0.0) == buy {
        size
    } else {
        (size - held.abs()).max(0.0)
    };
    let ready = price > 0.0 && size > 0.0 && leverage > 0.0;
    let known = maintenance > 0.0;
    Ticket {
        notional,
        margin: if ready {
            price * opening / leverage
        } else {
            0.0
        },
        liquidation: if ready && known && opening > 0.0 {
            ticket_liquidation(price, leverage, maintenance, buy)
        } else {
            0.0
        },
        leverage,
        ready,
        known,
    }
}

/// What crossing the spread right now would actually cost: the size walked
/// through the resting side of the book, level by level, at the prices that
/// are really there.
///
/// The ticket quotes a price the reader typed. This is the other price — the
/// one a market order gets — and the difference between them is the whole
/// question of whether to cross or to rest.
#[derive(Clone, PartialEq)]
pub struct Impact {
    /// Size-weighted price the walk actually pays.
    pub paid: f64,
    /// How far that is from the mid, the wrong way, as a percent.
    pub slippage_pct: f64,
    /// How much of the size the visible book could fill.
    pub filled: f64,
    /// The book ran out before the size did.
    pub short: bool,
    pub ready: bool,
}

pub fn book_impact(book: Option<Book>, size: String, buy: bool) -> Impact {
    let empty = Impact {
        paid: 0.0,
        slippage_pct: 0.0,
        filled: 0.0,
        short: false,
        ready: false,
    };
    let wanted = amount(&size).abs();
    let Some(book) = book else {
        return empty;
    };
    // A buy lifts the asks and a sell hits the bids: the side that is resting
    // is the other one. The asks arrive reversed, because the panel draws them
    // downward into the spread, so the best of them is the last — and a walk
    // that started at the front would sweep from the worst price in the book.
    let side: Vec<&Level> = if buy {
        book.asks.iter().rev().collect()
    } else {
        book.bids.iter().collect()
    };
    if wanted <= 0.0 || side.is_empty() || book.mid <= 0.0 {
        return empty;
    }
    let mut left = wanted;
    let mut notional = 0.0;
    for level in side {
        if left <= 0.0 {
            break;
        }
        let taken = level.size.min(left);
        notional += taken * level.price;
        left -= taken;
    }
    let filled = wanted - left;
    if filled <= 0.0 {
        return empty;
    }
    let paid = notional / filled;
    // Slippage is what the crossing costs, so it reads positive either way.
    let slippage_pct = if buy {
        (paid - book.mid) / book.mid * 100.0
    } else {
        (book.mid - paid) / book.mid * 100.0
    };
    Impact {
        paid,
        slippage_pct,
        filled,
        short: left > 0.0,
        ready: true,
    }
}

/// What crossing right now would pay, as the panel reads it. Three thin
/// readings rather than one struct: the boundary carries what the view draws,
/// and the view draws a price, a distance, and a warning.
pub fn impact_price(book: Option<Book>, size: String, buy: bool) -> String {
    let impact = book_impact(book, size, buy);
    if impact.ready {
        fmt_px(impact.paid)
    } else {
        "—".to_owned()
    }
}

pub fn impact_slippage(book: Option<Book>, size: String, buy: bool) -> String {
    let impact = book_impact(book, size, buy);
    if impact.ready {
        fmt_bps(impact.slippage_pct)
    } else {
        String::new()
    }
}

pub fn impact_short(book: Option<Book>, size: String, buy: bool) -> bool {
    book_impact(book, size, buy).short
}

/// The share of the entry-to-liquidation distance the mark has already
/// covered: 0 at the entry price, 1 at the cliff. Works for either side
/// because both endpoints flip together, and reads 0 when the position has
/// no liquidation price at all.
fn liquidation_travel(entry: f64, mark: f64, liquidation: f64) -> f64 {
    let span = liquidation - entry;
    if !(span.is_finite() && span.abs() > f64::EPSILON) || liquidation <= 0.0 {
        return 0.0;
    }
    ((mark - entry) / span).clamp(0.0, 1.0)
}

fn parse_account(value: &Value) -> Account {
    let summary = value.get("marginSummary").cloned().unwrap_or(Value::Null);
    let positions: Vec<Position> = list(value, "assetPositions")
        .iter()
        .filter_map(|entry| entry.get("position"))
        .map(|position| {
            let size = num(position, "szi");
            let value = num(position, "positionValue");
            let entry = num(position, "entryPx");
            let mark = if size == 0.0 { 0.0 } else { value / size.abs() };
            Position {
                coin: text(position, "coin"),
                size,
                entry,
                // The exchange reports notional, not a mark price.
                mark,
                liq: num(position, "liquidationPx"),
                pnl: num(position, "unrealizedPnl"),
                roe_pct: num(position, "returnOnEquity") * 100.0,
                margin: num(position, "marginUsed"),
                risk: liquidation_travel(entry, mark, num(position, "liquidationPx"))
                    * RISK_RAIL_WIDTH,
                leverage: position
                    .get("leverage")
                    .and_then(|leverage| leverage.get("value"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                margin_mode: position
                    .get("leverage")
                    .map_or_else(String::new, |leverage| text(leverage, "type")),
                // Funding paid since the position was opened; negative is paid out.
                funding: position
                    .get("cumFunding")
                    .map_or(0.0, |funding| num(funding, "sinceOpen")),
            }
        })
        .collect();
    let equity = num(&summary, "accountValue");
    // The requirement is the cross one, so the equity it is measured against
    // has to be the cross one too. Total equity carries the margin posted
    // behind isolated positions, which the cross engine cannot spend and does
    // not count: dividing by it reads an account at its cliff as comfortable,
    // by however much is locked away.
    let cross = value
        .get("crossMarginSummary")
        .cloned()
        .unwrap_or(Value::Null);
    let cross_value = num(&cross, "accountValue");
    let maintenance = num(value, "crossMaintenanceMarginUsed");
    Account {
        value: equity,
        cross_value,
        pnl: positions.iter().map(|position| position.pnl).sum(),
        withdrawable: num(value, "withdrawable"),
        notional: num(&summary, "totalNtlPos"),
        maintenance,
        health: margin_load(cross_value, maintenance) * RISK_RAIL_WIDTH,
        margin_pct: margin_load(cross_value, maintenance) * 100.0,
        positions,
    }
}

/// The share of the account's equity the maintenance requirement has already
/// claimed: 0 with nothing at risk, 1 where the margin engine steps in. Cross
/// positions do not die one at a time — the whole account goes when its equity
/// falls under `crossMaintenanceMarginUsed` — so this is that one distance,
/// read the same way the per-position rail reads entry to liquidation.
///
/// Equity at or below zero is already past the cliff, and an account with no
/// maintenance requirement has no cliff to be near.
fn margin_load(equity: f64, maintenance: f64) -> f64 {
    if maintenance <= 0.0 {
        return 0.0;
    }
    if equity <= 0.0 {
        return 1.0;
    }
    (maintenance / equity).clamp(0.0, 1.0)
}

pub async fn hl_account(address: String) -> Result<Account, HlError> {
    Ok(parse_account(
        &info(json!({ "type": "clearinghouseState", "user": address })).await?,
    ))
}

/// The account's fills. `heat` is what the list flashes with, so the
/// snapshot the feed opens with reads cold and everything pushed after it
/// arrives lit.
/// The public tape, folded the way it was actually traded. One aggressing
/// order that eats four resting orders arrives as four messages sharing a
/// `hash`; four rows at the same price is the wire's bookkeeping rather than
/// the market's, so consecutive prints from one order become one row carrying
/// what that order paid on average and how many resting orders it took.
fn parse_trades(value: &Value, coin: &str) -> Vec<Trade> {
    let mut tape: Vec<Trade> = Vec::new();
    let mut aggressor = String::new();
    for print in value.as_array().map(Vec::as_slice).unwrap_or_default() {
        // A print from the market the app just left would read as this one's.
        // A print from the market the app just left would read as this one's.
        if text(print, "coin") != coin {
            continue;
        }
        let hash = text(print, "hash");
        let price = num(print, "px");
        let size = num(print, "sz");
        match tape.last_mut() {
            // An empty hash is not an identity, so it never merges.
            Some(held) if !hash.is_empty() && hash == aggressor => {
                let total = held.size + size;
                if total > 0.0 {
                    held.price = (held.price * held.size + price * size) / total;
                }
                held.size = total;
                held.sweep += 1;
            }
            _ => {
                aggressor = hash;
                tape.push(Trade {
                    ts: value_i64(print, "time") / 1_000,
                    price,
                    size,
                    // The same encoding the account's own fills use: "B" took
                    // the offer, "A" hit the bid.
                    buy: text(print, "side") == "B",
                    sweep: 1,
                    tid: value_i64(print, "tid"),
                });
            }
        }
    }
    tape
}

fn parse_fills(value: &Value, heat: i64) -> Vec<Fill> {
    value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|fill| Fill {
            coin: text(fill, "coin"),
            ts: value_i64(fill, "time") / 1_000,
            price: num(fill, "px"),
            size: num(fill, "sz"),
            // "B" is a buy, "A" hits the ask side and is a sell.
            buy: text(fill, "side") == "B",
            closed_pnl: num(fill, "closedPnl"),
            heat,
            tid: value_i64(fill, "tid"),
        })
        .collect()
}

fn parse_orders(value: &Value) -> Vec<Order> {
    value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|order| Order {
            coin: text(order, "coin"),
            buy: text(order, "side") == "B",
            price: num(order, "limitPx"),
            size: num(order, "sz"),
            ts: value_i64(order, "timestamp") / 1_000,
        })
        .collect()
}

/// One side of the book, nearest price first, with cumulative depth resolved
/// into the bar width the view draws behind each row.
fn parse_levels(side: Option<&Value>) -> Vec<Level> {
    let levels = side
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut total = 0.0;
    let mut rows: Vec<Level> = levels
        .iter()
        .take(BOOK_DEPTH)
        .map(|level| {
            total += num(level, "sz");
            Level {
                price: num(level, "px"),
                size: num(level, "sz"),
                total,
                bar: 0.0,
            }
        })
        .collect();
    let deepest = rows.last().map_or(0.0, |level| level.total);
    if deepest > 0.0 {
        for level in &mut rows {
            level.bar = level.total / deepest * BOOK_BAR_WIDTH;
        }
    }
    rows
}

fn parse_book(value: &Value) -> Book {
    let sides = value.get("levels").and_then(Value::as_array);
    let bids = parse_levels(sides.and_then(|sides| sides.first()));
    let asks = parse_levels(sides.and_then(|sides| sides.get(1)));
    let (best_bid, best_ask) = (
        bids.first().map_or(0.0, |level| level.price),
        asks.first().map_or(0.0, |level| level.price),
    );
    let spread = if best_bid > 0.0 && best_ask > 0.0 {
        best_ask - best_bid
    } else {
        0.0
    };
    let mid = if spread > 0.0 {
        (best_ask + best_bid) / 2.0
    } else {
        best_bid.max(best_ask)
    };
    Book {
        bids,
        asks,
        spread,
        spread_pct: if mid > 0.0 { spread / mid * 100.0 } else { 0.0 },
        mid,
    }
}

pub async fn hl_orders(address: String) -> Result<Vec<Order>, HlError> {
    Ok(parse_orders(
        &info(json!({ "type": "openOrders", "user": address })).await?,
    ))
}

/// One websocket connection, plus the set of subscriptions it is currently
/// holding open.
struct Socket {
    ws: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    open: Vec<Value>,
}

impl Socket {
    fn connect() -> Result<Self, HlError> {
        let (ws, _) = tungstenite::connect(WS_URL)
            .map_err(|error| HlError::new(format!("Hyperliquid feed unreachable: {error}")))?;
        // Reads have to time out, or the loop could never look at the clock
        // to ping, or at the app to see that it changed markets.
        let stream = match ws.get_ref() {
            MaybeTlsStream::Plain(stream) => stream,
            MaybeTlsStream::Rustls(tls) => tls.get_ref(),
            _ => return Err(HlError::new("Unknown Hyperliquid transport".to_owned())),
        };
        stream
            .set_read_timeout(Some(POLL))
            .map_err(|error| HlError::new(format!("Hyperliquid feed unreadable: {error}")))?;
        Ok(Self {
            ws,
            open: Vec::new(),
        })
    }

    fn send(&mut self, request: &Value) -> Result<(), HlError> {
        self.ws
            .send(tungstenite::Message::Text(request.to_string().into()))
            .map_err(|error| HlError::new(format!("Hyperliquid feed refused: {error}")))
    }

    /// Holds exactly `wanted` open, sending only the difference. Switching
    /// markets therefore costs two frames rather than a new connection, and
    /// asking for what is already open costs nothing.
    fn want(&mut self, wanted: Vec<Value>) -> Result<(), HlError> {
        let gone: Vec<Value> = self
            .open
            .iter()
            .filter(|held| !wanted.contains(held))
            .cloned()
            .collect();
        let fresh: Vec<Value> = wanted
            .iter()
            .filter(|want| !self.open.contains(want))
            .cloned()
            .collect();
        for subscription in gone {
            self.send(&json!({ "method": "unsubscribe", "subscription": subscription }))?;
        }
        for subscription in fresh {
            self.send(&json!({ "method": "subscribe", "subscription": subscription }))?;
        }
        self.open = wanted;
        Ok(())
    }
}

/// What a feed's reader is handed. `Beat` is the coalescing tick: a reader
/// that folds fast traffic into one update returns it there, and one with
/// nothing to fold ignores it.
enum Event<'a> {
    Payload(&'a str, &'a Value),
    /// Round trip to the exchange in milliseconds.
    Pong(i64),
    Beat,
}

/// One websocket on its own thread, pumping what `read` makes of it into a
/// channel that Ice consumes as a stream.
///
/// `subscribe` is asked what to hold open on every beat rather than once at
/// connect, so a feed follows the app's market by re-subscribing instead of
/// reconnecting. The thread reconnects through an error and stops when the
/// receiver is dropped, which is what aborting the stream does.
fn feed<T, S, R>(mut subscribe: S, mut read: R) -> Receiver<Result<T, HlError>>
where
    T: Send + 'static,
    S: FnMut() -> Vec<Value> + Send + 'static,
    R: FnMut(Event<'_>) -> Option<T> + Send + 'static,
{
    let (sender, receiver) = smol::channel::unbounded();
    std::thread::spawn(move || {
        while !sender.is_closed() {
            let Err(error) = pump(&mut subscribe, &mut read, &sender) else {
                return;
            };
            if sender.send_blocking(Err(error)).is_err() {
                return;
            }
            std::thread::sleep(RETRY);
        }
    });
    receiver
}

/// One connection's lifetime. Returns `Ok` only when the app has stopped
/// listening; anything else is an error the caller reconnects through.
fn pump<T, S, R>(
    subscribe: &mut S,
    read: &mut R,
    sender: &Sender<Result<T, HlError>>,
) -> Result<(), HlError>
where
    S: FnMut() -> Vec<Value>,
    R: FnMut(Event<'_>) -> Option<T>,
{
    let mut socket = Socket::connect()?;
    let mut beat = Instant::now();
    let mut ping = Instant::now();
    let mut sent: Option<Instant> = None;
    loop {
        if sender.is_closed() {
            return Ok(());
        }
        // Before the blocking read, so the first pass subscribes rather than
        // waiting out a timeout first.
        let now = Instant::now();
        if now >= beat {
            beat = now + BEAT;
            socket.want(subscribe())?;
            if let Some(item) = read(Event::Beat)
                && sender.send_blocking(Ok(item)).is_err()
            {
                return Ok(());
            }
        }
        if now >= ping {
            ping = now + PING;
            sent = Some(now);
            socket.send(&json!({ "method": "ping" }))?;
        }
        match socket.ws.read() {
            Ok(tungstenite::Message::Text(body)) => {
                let message: Value = serde_json::from_str(&body)
                    .map_err(|error| HlError::new(format!("Hyperliquid sent bad JSON: {error}")))?;
                let channel = text(&message, "channel");
                if channel == "error" {
                    // A rejected subscription is silence otherwise, and
                    // silence is indistinguishable from a quiet market.
                    return Err(HlError::new(format!(
                        "Hyperliquid feed rejected a request: {}",
                        text(&message, "data")
                    )));
                }
                let item = if channel == "pong" {
                    let round_trip = sent.take().map_or(0, |at| at.elapsed().as_millis() as i64);
                    read(Event::Pong(round_trip))
                } else {
                    match message.get("data") {
                        Some(data) => read(Event::Payload(&channel, data)),
                        None => None,
                    }
                };
                if let Some(item) = item
                    && sender.send_blocking(Ok(item)).is_err()
                {
                    return Ok(());
                }
            }
            Ok(_) => {}
            // A read that found nothing before its timeout, which is the
            // normal way out of the blocking read below the beat rate.
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => {
                return Err(HlError::new(format!("Hyperliquid feed dropped: {error}")));
            }
        }
    }
}

/// What one beat of the market feed carries. Every beat holds the latest of
/// everything rather than only what changed, so the app assigns it without
/// having to ask which kind of update this was.
#[derive(Clone, Default, PartialEq)]
pub struct MarketTick {
    /// The book, or none until the first one lands.
    pub book: Option<Book>,
    /// Round trip to the exchange in milliseconds.
    pub latency: i64,
    /// Every mid price the exchange last published, by market.
    mids: HashMap<String, f64>,
    /// Prints on the charted market since the last beat, oldest first.
    trades: Vec<Trade>,
    /// The charted market's republished context, when it arrived.
    context: Option<SymbolRow>,
}

/// The market data feed: every mid price on the exchange, and the book,
/// candles, and context of whatever market the tape is pointed at. Candles
/// are merged into the tape in place, so the chart follows them on its own
/// repaint beat without an app message per tick.
pub fn hl_market_feed(tape: Tape) -> Receiver<Result<MarketTick, HlError>> {
    let subscriptions = tape.clone();
    let mut tick = MarketTick::default();
    let mut changed = false;
    // Which market the per-market fields belong to. The book and the context
    // are the focused market's, and the app can move that focus between two
    // beats: without this the next beat republishes the book of the market
    // the reader just left, over the top of the one they are looking at.
    let mut held: Option<(String, String)> = None;
    feed(
        move || {
            let mut wanted = vec![json!({ "type": "allMids" })];
            if let Some((coin, interval)) = subscriptions.focus() {
                wanted.push(json!({ "type": "l2Book", "coin": coin }));
                wanted.push(json!({ "type": "activeAssetCtx", "coin": coin }));
                wanted.push(json!({ "type": "candle", "coin": coin, "interval": interval }));
                wanted.push(json!({ "type": "trades", "coin": coin }));
            }
            wanted
        },
        move |event| match event {
            Event::Payload("allMids", data) => {
                tick.mids = data
                    .get("mids")
                    .and_then(Value::as_object)?
                    .iter()
                    .filter_map(|(coin, price)| Some((coin.clone(), price.as_str()?.parse().ok()?)))
                    .collect();
                changed = true;
                None
            }
            Event::Beat if tape.focus() != held => {
                held = tape.focus();
                tick.book = None;
                tick.context = None;
                tick.trades.clear();
                changed = false;
                None
            }
            Event::Payload("l2Book", data) => {
                let mut book = parse_book(data);
                // The book renders from the top down, so the asks are
                // reversed here and the view just walks both lists.
                book.asks.reverse();
                changed |= tick.book.as_ref() != Some(&book);
                tick.book = Some(book);
                None
            }
            Event::Payload("activeAssetCtx", data) => {
                let context = parse_context(text(data, "coin"), data.get("ctx")?);
                changed |= tick.context.as_ref() != Some(&context);
                tick.context = Some(context);
                None
            }
            Event::Payload("trades", data) => {
                let (held, _) = tape.focus()?;
                let fresh = parse_trades(data, &held);
                changed |= !fresh.is_empty();
                tick.trades.extend(fresh);
                None
            }
            Event::Payload("candle", data) => {
                // A candle for the market the app just left would corrupt
                // the tape it is drawing now.
                let held = tape.focus()?;
                if held != (text(data, "s"), text(data, "i")) {
                    return None;
                }
                merge(&mut lock(&tape.candles), vec![parse_candle(data)]);
                None
            }
            Event::Payload(..) => None,
            Event::Pong(round_trip) => {
                changed |= tick.latency != round_trip;
                tick.latency = round_trip;
                None
            }
            Event::Beat => {
                let ready = changed;
                changed = false;
                // Nothing moved: an unchanged message would rebuild the view
                // for no reason.
                ready.then(|| MarketTick {
                    // Both are consumed once: the app folds them into what it
                    // already holds, so replaying them on a quiet beat would
                    // re-apply prices and repeat the tape.
                    mids: std::mem::take(&mut tick.mids),
                    trades: std::mem::take(&mut tick.trades),
                    ..tick.clone()
                })
            }
        },
    )
}

/// This account's fills as they print. The exchange opens with a snapshot of
/// the recent ones and pushes each new one after that; only the pushed ones
/// are lit, so the list flashes what just happened rather than its history.
pub fn hl_fill_feed(address: String) -> Receiver<Result<Vec<Fill>, HlError>> {
    feed(
        move || vec![json!({ "type": "userFills", "user": address })],
        move |event| {
            let Event::Payload("userFills", data) = event else {
                return None;
            };
            let snapshot = data
                .get("isSnapshot")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let fills = parse_fills(data.get("fills")?, if snapshot { 0 } else { FLASH_STEPS });
            (!fills.is_empty()).then_some(fills)
        },
    )
}

/// Folds one beat of the feed into the market list: the charted market's
/// republished context first, then every mid price the beat carried, which is
/// the fresher of the two. A ticker and its maximum leverage belong to the
/// asset rather than the day, so they stay whatever the universe said.
pub fn apply_feed(rows: Vec<SymbolRow>, tick: MarketTick) -> Vec<SymbolRow> {
    let mut rows = rows;
    if let Some(context) = tick.context
        && let Some(row) = rows.iter_mut().find(|row| row.name == context.name)
    {
        *row = SymbolRow {
            // Both belong to the asset rather than to the day, and the
            // streamed context does not carry them.
            leverage: row.leverage,
            maintenance: row.maintenance,
            ..context
        };
    }
    for row in &mut rows {
        if let Some(price) = tick.mids.get(&row.name) {
            row.price = *price;
            row.change_pct = change_pct(*price, row.prev);
        }
    }
    rows
}

/// The margin a position was opened with: what it was worth at its entry,
/// over its leverage. This is what the exchange's `returnOnEquity` divides
/// by, for a cross position and an isolated one alike — `marginUsed` is
/// something else, and for an isolated position something else again.
fn opening_margin(position: &Position) -> Option<f64> {
    if position.leverage <= 0.0 {
        return None;
    }
    let margin = position.entry * position.size.abs() / position.leverage;
    (margin > 0.0).then_some(margin)
}

/// Re-values open positions against the prices that just came in. Asking the
/// exchange to restate them is the one thing here that still polls, and
/// between polls the figures that move with the mark move: what the position
/// has made, what that is as a return, and how far the mark now sits from the
/// liquidation price.
///
/// The mark is the feed's mid, which is a tick either side of the mark price
/// the exchange values it at; the next poll settles the difference.
pub fn mark_positions(positions: Vec<Position>, tick: MarketTick) -> Vec<Position> {
    positions
        .into_iter()
        .map(|position| {
            let Some(mark) = tick.mids.get(&position.coin).copied() else {
                return position;
            };
            // Move what the exchange last reported by what the price has done
            // since, rather than recomputing it from the entry: the entry it
            // reports is an average rounded to five figures, and a position
            // of sixty million units turns that rounding into real money. A
            // delta carries none of it, and the next poll re-anchors.
            let gain = (mark - position.mark) * position.size;
            Position {
                mark,
                pnl: position.pnl + gain,
                roe_pct: position.roe_pct
                    + opening_margin(&position).map_or(0.0, |opening| gain / opening * 100.0),
                risk: liquidation_travel(position.entry, mark, position.liq) * RISK_RAIL_WIDTH,
                ..position
            }
        })
        .collect()
}

/// Re-totals the account from positions the feed has just re-valued. Equity
/// holds unrealized PnL, so it moves by the difference between the PnL the
/// last poll reported and the PnL now, which keeps this exact however many
/// times it runs between polls.
///
/// What is withdrawable, what margin is tied up, and what the maintenance
/// requirement is are the margin engine's own numbers rather than arithmetic
/// over positions — an isolated position posts collateral of its own, and the
/// account carries resting orders too — so those wait for the next poll.
pub fn mark_account(account: Option<Account>, positions: Vec<Position>) -> Option<Account> {
    let account = account?;
    let pnl: f64 = positions.iter().map(|position| position.pnl).sum();
    let cross_pnl = |rows: &[Position]| -> f64 {
        rows.iter()
            .filter(|held| held.margin_mode == "cross")
            .map(|held| held.pnl)
            .sum()
    };
    let value = account.value - account.pnl + pnl;
    // Only the cross positions move the equity the cross engine measures; an
    // isolated one gains and loses against its own posted margin.
    let cross_value = account.cross_value - cross_pnl(&account.positions) + cross_pnl(&positions);
    Some(Account {
        value,
        cross_value,
        pnl,
        health: margin_load(cross_value, account.maintenance) * RISK_RAIL_WIDTH,
        margin_pct: margin_load(cross_value, account.maintenance) * 100.0,
        notional: positions
            .iter()
            .map(|position| position.mark * position.size.abs())
            .sum(),
        positions,
        ..account
    })
}

/// Price decimals that suit the instrument: a four-figure coin and a
/// fraction-of-a-cent coin cannot share one setting.
fn price_decimals(value: f64) -> usize {
    let value = value.abs();
    if value >= 1_000.0 {
        2
    } else if value >= 1.0 {
        3
    } else {
        6
    }
}

pub fn fmt_px(value: f64) -> String {
    format_price(value, price_decimals(value))
}

pub fn fmt_usd(value: f64) -> String {
    format!("${}", format_price(value, 2))
}

pub fn fmt_signed_usd(value: f64) -> String {
    let sign = if value >= 0.0 { "+" } else { "-" };
    format!("{sign}${}", format_price(value.abs(), 2))
}

pub fn fmt_pct(value: f64) -> String {
    let sign = if value >= 0.0 { "+" } else { "" };
    format!("{sign}{value:.2}%")
}

pub fn fmt_size(value: f64) -> String {
    format_price(value.abs(), price_decimals(value * 1_000.0))
}

pub fn fmt_volume(value: f64) -> String {
    format_volume(value)
}

/// Wall-clock time of day in UTC, which is what a fills list is read by.
pub fn fmt_time(ts: i64) -> String {
    let secs = ts.rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

pub fn fmt_leverage(value: f64) -> String {
    format!("{value:.0}x")
}

/// The round trip to the exchange, as the feed's own readout. A dash until
/// the first pong comes back, so an empty reading never reads as instant.
pub fn fmt_latency(millis: i64) -> String {
    if millis <= 0 {
        return "—".to_owned();
    }
    format!("{millis}ms")
}

/// An hourly funding rate. Two decimals is the precision the rest of the
/// panel wants and the one funding cannot use: the rate is a hundredth of a
/// percent on most markets, so `fmt_pct` renders 166 of the exchange's 177
/// funded markets as an identical "+0.00%". Four decimals is what the venue
/// itself quotes, and it is the difference between a column and a row of
/// zeroes.
pub fn fmt_funding(percent: f64) -> String {
    let sign = if percent >= 0.0 { "+" } else { "" };
    format!("{sign}{percent:.4}%")
}

/// A plain share, for a figure that is a proportion rather than a move: no
/// sign, because a margin requirement is never a gain.
pub fn fmt_share(percent: f64) -> String {
    format!("{percent:.0}%")
}

/// The spread as a share of the mid, in basis points. A spread is only worth
/// reading against the price it sits on: two dollars is nothing on Bitcoin and
/// the whole market on a coin worth a cent, and the absolute figure makes you
/// do that division yourself for every market you look at.
pub fn fmt_bps(percent: f64) -> String {
    if !percent.is_finite() || percent <= 0.0 {
        return "—".to_owned();
    }
    format!("{:.1} bps", percent * 100.0)
}

/// Large money in a narrow column: "-$3.3M" rather than eleven digits.
pub fn fmt_compact_usd(value: f64) -> String {
    if value == 0.0 {
        return "$0".to_owned();
    }
    let sign = if value > 0.0 { "+" } else { "-" };
    format!("{sign}${}", format_volume(value.abs()))
}

/// A plain count for a section header.
pub fn fmt_count(value: i64) -> String {
    value.to_string()
}

/// A single fill's realized PnL: exact while it is small enough to read,
/// compact once it is not.
pub fn fmt_pnl(value: f64) -> String {
    if value.abs() < 10_000.0 {
        fmt_signed_usd(value)
    } else {
        fmt_compact_usd(value)
    }
}

/// How a position is levered, as one cell: "40x cross".
pub fn fmt_leverage_mode(value: f64, mode: String) -> String {
    if mode.is_empty() {
        return fmt_leverage(value);
    }
    format!("{} {mode}", fmt_leverage(value))
}

/// Substring match on the ticker, which is all a 200-row sidebar needs.
/// Filters the universe down to what the list shows, and marks the row that is
/// on screen. The mark rides along instead of being read from `coin` at render
/// time so each row stays a self-contained dependency for the view's cache.
pub fn filter_symbols(rows: Vec<SymbolRow>, query: String, coin: String) -> Vec<SymbolRow> {
    let query = query.trim().to_uppercase();
    rows.into_iter()
        .filter(|row| query.is_empty() || row.name.to_uppercase().contains(&query))
        .map(|row| SymbolRow {
            selected: row.name == coin,
            ..row
        })
        .collect()
}

pub fn symbol_row(rows: Vec<SymbolRow>, coin: String) -> Option<SymbolRow> {
    rows.into_iter().find(|row| row.name == coin)
}

/// Puts the fills the feed just pushed on top of the ones already listed,
/// newest first, ignoring any the opening snapshot had already shown, and
/// capped so the list builds a bounded number of rows however long the
/// account trades for.
pub fn push_fills(history: Vec<Fill>, incoming: Vec<Fill>, limit: i64) -> Vec<Fill> {
    let held: HashSet<i64> = history.iter().map(|fill| fill.tid).collect();
    let mut rows: Vec<Fill> = incoming
        .into_iter()
        .filter(|fill| !held.contains(&fill.tid))
        .chain(history)
        .collect();
    rows.sort_by_key(|fill| std::cmp::Reverse(fill.ts));
    rows.truncate(limit.max(0) as usize);
    rows
}

/// Puts a beat's prints on top of the tape, newest first, capped so the panel
/// builds a bounded number of rows however busy the market is. Arrival order
/// is the tape's order — prints inside one second are not re-sorted, because
/// the sequence they crossed in is the thing worth reading.
pub fn push_trades(tape: Vec<Trade>, tick: MarketTick, limit: i64) -> Vec<Trade> {
    let held: HashSet<i64> = tape.iter().map(|print| print.tid).collect();
    let mut rows: Vec<Trade> = tick
        .trades
        .into_iter()
        .rev()
        .filter(|print| !held.contains(&print.tid))
        .chain(tape)
        .collect();
    rows.truncate(limit.max(0) as usize);
    rows
}

/// How many resting orders one aggressor took, when it took more than one.
/// A single print needs no mark; a sweep is the thing worth seeing.
pub fn fmt_sweep(count: i64) -> String {
    if count <= 1 {
        return String::new();
    }
    format!("×{count}")
}

/// One step of the highlight decay on the newest fills.
pub fn cool_fills(rows: Vec<Fill>) -> Vec<Fill> {
    rows.into_iter()
        .map(|fill| Fill {
            heat: (fill.heat - 1).max(0),
            ..fill
        })
        .collect()
}

/// Whether anything in the list is still lit, which is the only reason to
/// run the decay timer at all.
pub fn any_hot(rows: Vec<Fill>) -> bool {
    rows.iter().any(|fill| fill.heat > 0)
}

/// An account address is `0x` and forty hexadecimal digits. Worth checking
/// before the request rather than after it: the exchange answers a malformed
/// address with a plain-text parser complaint rather than JSON, so a typo
/// surfaces as "Hyperliquid sent bad JSON" — the one error message that blames
/// the exchange for something the user just typed.
pub fn valid_address(address: String) -> bool {
    let address = address.trim();
    address.len() == 42
        && address.starts_with("0x")
        && address[2..].bytes().all(|digit| digit.is_ascii_hexdigit())
}

/// The lower panel's height, held between what still shows a row and what
/// still leaves a chart. Clamping rather than refusing matters on a drag: a
/// gesture that overshoots the limit should stop at it, not stop the panel
/// moving at all, which is what rejecting the whole delta does.
pub fn pane_height(wanted: f64) -> f64 {
    // Only a NaN needs a default; an infinity clamps to the right end on its
    // own, and `clamp` would hand a NaN straight back.
    if wanted.is_nan() {
        return LOWER_MIN;
    }
    wanted.clamp(LOWER_MIN, LOWER_MAX)
}

/// What a book row's button does, which is what a reader arriving on it needs
/// to hear. The price alone is a number with no side and no consequence, and
/// this row starts an order.
pub fn book_label(price: f64, buy: bool) -> String {
    let side = if buy { "Buy" } else { "Sell" };
    format!("{side} at {}", fmt_px(price))
}

/// A position row carries a side, a size, an entry, a liquidation price and a
/// PnL. Named by its coin alone, a reader arriving on it hears "BTC".
pub fn position_label(held: Position) -> String {
    let side = if held.size >= 0.0 { "long" } else { "short" };
    format!("{} {side} {}", held.coin, fmt_size(held.size))
}

/// A level somebody asked to be told about. Which side it is waiting on is
/// not asked for: a level above the mark can only be reached from below, and
/// one below it from above, so the direction is a fact about the price rather
/// than a question for the person setting it.
#[derive(Clone, PartialEq)]
pub struct Alert {
    pub coin: String,
    pub price: f64,
    pub above: bool,
    pub fired: bool,
}

pub fn add_alert(alerts: Vec<Alert>, coin: String, price: String, mark: f64) -> Vec<Alert> {
    let price = amount(&price);
    let mut alerts = alerts;
    // A level at the mark has already happened, and a duplicate is not a
    // second alert.
    if coin.is_empty() || price <= 0.0 || mark <= 0.0 || (price - mark).abs() < f64::EPSILON {
        return alerts;
    }
    if alerts
        .iter()
        .any(|held| held.coin == coin && (held.price - price).abs() < f64::EPSILON)
    {
        return alerts;
    }
    alerts.push(Alert {
        coin,
        price,
        above: price > mark,
        fired: false,
    });
    alerts
}

/// Marks every level the market has reached. Firing is one-way: a level that
/// was touched stays touched, so a price wobbling across it does not chime
/// twice and does not un-chime.
pub fn check_alerts(alerts: Vec<Alert>, tick: MarketTick) -> Vec<Alert> {
    alerts
        .into_iter()
        .map(|alert| {
            let Some(mark) = tick.mids.get(&alert.coin).copied() else {
                return alert;
            };
            let reached = if alert.above {
                mark >= alert.price
            } else {
                mark <= alert.price
            };
            Alert {
                fired: alert.fired || reached,
                ..alert
            }
        })
        .collect()
}

/// How many are still waiting, which is the only count worth a header.
pub fn waiting_alerts(alerts: Vec<Alert>) -> i64 {
    alerts.iter().filter(|alert| !alert.fired).count() as i64
}

pub fn drop_alert(alerts: Vec<Alert>, coin: String, price: f64) -> Vec<Alert> {
    alerts
        .into_iter()
        .filter(|alert| alert.coin != coin || (alert.price - price).abs() >= f64::EPSILON)
        .collect()
}

/// The market's price, or zero when it has not loaded — what an alert is set
/// relative to.
pub fn mark_price(market: Option<SymbolRow>) -> f64 {
    market.map_or(0.0, |row| row.price)
}

/// An alert names the level and which way the market has to go to reach it.
pub fn alert_label(alert: Alert) -> String {
    let state = if alert.fired {
        "reached"
    } else if alert.above {
        "waiting above"
    } else {
        "waiting below"
    };
    format!("{} {state} {}", alert.coin, fmt_px(alert.price))
}

pub fn alert_arrow(alert: Alert) -> String {
    if alert.above { "▲" } else { "▼" }.to_owned()
}

/// A size the account could actually put on: a share of what is free to
/// withdraw, levered, at the price in the ticket. Free margin rather than
/// equity, because equity already has positions standing on it.
/// `leverage` is the one the ticket was priced at, not the one that was
/// typed. Every other figure in the panel is quoted at the market's cap, and a
/// share button that levered at the typed number would fill in a size the
/// margin engine refuses — by exactly the factor the typed number overshot.
pub fn ticket_afford(account: Option<Account>, price: String, leverage: f64, share: f64) -> String {
    let free = account.map_or(0.0, |held| held.withdrawable);
    let price = amount(&price);
    if free <= 0.0 || price <= 0.0 || leverage <= 0.0 || share <= 0.0 {
        return String::new();
    }
    fmt_size(free * share.min(1.0) * leverage / price)
}

/// What an order would do to what you already hold. Opening and closing are
/// different acts on the same ticket, and the difference is the sign of a
/// number two panels apart — the size you typed here and the position sitting
/// in the panel below.
/// Where this order leaves the account, against the engine: the margin load
/// now and the load once the order is on. Reads "1% → 4%".
///
/// The panel already says what an order costs in margin. It did not say what
/// it costs in distance, which is the figure a cross account is actually
/// liquidated on — and the one a reader has to be able to see before sending,
/// not after.
///
/// Only cross positions are counted, because only they are held against the
/// account. An isolated order changes nothing here and says so.
pub fn order_load(
    account: Option<Account>,
    coin: String,
    size: String,
    buy: bool,
    market: Option<SymbolRow>,
) -> String {
    let Some(account) = account else {
        return String::new();
    };
    let size = amount(&size).abs();
    let Some(market) = market else {
        return String::new();
    };
    if size <= 0.0 || account.cross_value <= 0.0 || market.price <= 0.0 {
        return String::new();
    }
    // The venue reports what this account is held to, so the reading starts
    // from that rather than from a sum reassembled out of the positions.
    let now = account.maintenance;
    let held = account
        .positions
        .iter()
        .find(|position| position.margin_mode == "cross" && position.coin == coin)
        .map_or(0.0, |position| position.size);
    let after_size = held + if buy { size } else { -size };
    // The order replaces this market's contribution with what it leaves.
    let fraction = market.maintenance;
    let after =
        now - held.abs() * market.price * fraction + after_size.abs() * market.price * fraction;
    format!(
        "{} → {}",
        fmt_share(margin_load(account.cross_value, now) * 100.0),
        fmt_share(margin_load(account.cross_value, after.max(0.0)) * 100.0)
    )
}

/// What holding this order costs, or pays, in a day at the current funding
/// rate. Perpetuals have no expiry, so the position is rented rather than
/// bought, and the rent is the part of the cost that never appears on the
/// ticket — it arrives hourly, forever, and is the reason a carry that looks
/// free is not.
///
/// Longs pay a positive rate and shorts are paid it, so a long reads negative
/// when the rate is positive, and a short reads positive.
pub fn funding_day(market: Option<SymbolRow>, price: String, size: String, buy: bool) -> String {
    let Some(market) = market else {
        return String::new();
    };
    let notional = amount(&price).max(0.0) * amount(&size).abs();
    if notional <= 0.0 {
        return String::new();
    }
    // `funding_pct` is a percentage charged hourly.
    let hourly = market.funding_pct / 100.0 * notional;
    let daily = hourly * 24.0;
    let signed = if buy { -daily } else { daily };
    format!("{}/day", fmt_signed_usd(signed))
}

pub fn ticket_effect(positions: Vec<Position>, coin: String, size: String, buy: bool) -> String {
    let size = amount(&size).abs();
    if size <= 0.0 {
        return String::new();
    }
    let held = positions
        .into_iter()
        .find(|position| position.coin == coin)
        .map_or(0.0, |position| position.size);
    let side = if buy { "long" } else { "short" };
    // A buy against a short reduces it, and so does a sell against a long.
    if held == 0.0 || (held > 0.0) == buy {
        return format!("Opens {} {side}", fmt_size(size));
    }
    let open = held.abs();
    let holding = if held > 0.0 { "long" } else { "short" };
    if size < open {
        format!("Reduces your {holding} to {}", fmt_size(open - size))
    } else if size > open {
        format!(
            "Closes your {holding} and opens {} {side}",
            fmt_size(size - open)
        )
    } else {
        format!("Closes your {holding}")
    }
}

/// The signed size held in one market, or zero when none is. Signed, because
/// the ticket needs both how much to close and which way that trade goes.
pub fn position_held(positions: Vec<Position>, coin: String) -> f64 {
    positions
        .into_iter()
        .find(|position| position.coin == coin)
        .map_or(0.0, |position| position.size)
}

/// A resting order names its side, its size and the price it waits at.
pub fn order_label(order: Order) -> String {
    let side = if order.buy { "buy" } else { "sell" };
    format!(
        "{} {side} {} at {}",
        order.coin,
        fmt_size(order.size),
        fmt_px(order.price)
    )
}

/// A fill names what it did and where. Its realized PnL is the point when it
/// closed something, and its size when it opened.
pub fn fill_label(fill: Fill) -> String {
    let side = if fill.buy { "bought" } else { "sold" };
    let outcome = if fill.closed_pnl == 0.0 {
        fmt_size(fill.size)
    } else {
        fmt_signed_usd(fill.closed_pnl)
    };
    format!("{} {side} {} at {}", fill.coin, outcome, fmt_px(fill.price))
}

/// The share of the tape that lifted the offer, as a percentage. Which side
/// is crossing tells you something a price alone does not: the same price with
/// buyers taking it and with sellers hitting it are two different markets.
/// Reads 50 on an empty tape, which is the only honest reading of no trades.
pub fn tape_pressure(prints: Vec<Trade>) -> f64 {
    let (bought, total) = prints.iter().fold((0.0, 0.0), |(bought, total), print| {
        let size = print.size.abs();
        (bought + if print.buy { size } else { 0.0 }, total + size)
    });
    if total <= 0.0 {
        return 50.0;
    }
    bought / total * 100.0
}

/// How long a resting order has been waiting, in the coarsest unit that still
/// says something: an order placed four days ago and one placed four minutes
/// ago are different orders, and the seconds between them are not the point.
pub fn fmt_age(ts: i64) -> String {
    let seconds = now_ms() / 1_000 - ts;
    if ts <= 0 || seconds < 0 {
        return "—".to_owned();
    }
    match seconds {
        0..60 => "now".to_owned(),
        60..3_600 => format!("{}m", seconds / 60),
        3_600..86_400 => format!("{}h", seconds / 3_600),
        _ => format!("{}d", seconds / 86_400),
    }
}

/// One market and one position, so the panels that only exist when an account
/// does can be rendered and asserted without an exchange. The spec puts
/// deterministic test behaviour behind a named preset, and a preset needs its
/// state from somewhere; these are that somewhere. Two bugs in this panel were
/// only ever visible in a picture, and a picture needs data.
pub fn demo_symbols() -> Vec<SymbolRow> {
    demo_symbols_priced(64_000.0, 63_210.0)
}

/// The same markets with bitcoin marked where the account against the engine
/// is marked. A position's mark is the feed's price for its market — the app
/// sets one from the other every beat — so a fixture that priced them apart
/// would be a state no beat could produce.
pub fn demo_symbols_at_risk() -> Vec<SymbolRow> {
    demo_symbols_priced(58_000.0, 64_000.0)
}

fn demo_symbols_priced(btc: f64, btc_prev: f64) -> Vec<SymbolRow> {
    // More than one, because a list of one hides every question worth asking
    // of a list: which row is selected, what a search leaves, and whether a
    // price landed on the market it belongs to. Volume descending, as the
    // parser leaves them.
    vec![
        SymbolRow {
            name: "BTC".to_owned(),
            price: btc,
            change_pct: change_pct(btc, btc_prev),
            volume: 1_300_000_000.0,
            funding_pct: 0.00125,
            leverage: 40.0,
            open_interest: 35_000.0,
            prev: btc_prev,
            maintenance: 1.0 / 80.0,
            selected: true,
        },
        SymbolRow {
            name: "ETH".to_owned(),
            price: 3_540.0,
            change_pct: change_pct(3_540.0, 3_500.0),
            volume: 890_000_000.0,
            funding_pct: 0.0009,
            leverage: 25.0,
            open_interest: 410_000.0,
            prev: 3_500.0,
            maintenance: 1.0 / 50.0,
            selected: false,
        },
        SymbolRow {
            name: "SOL".to_owned(),
            price: 148.62,
            change_pct: change_pct(148.62, 152.0),
            volume: 410_000_000.0,
            funding_pct: -0.00042,
            leverage: 20.0,
            open_interest: 2_900_000.0,
            prev: 152.0,
            maintenance: 1.0 / 40.0,
            selected: false,
        },
        // A market priced in fractions of a cent, which most of a perp venue
        // is. Every column here was sized and formatted around bitcoin.
        SymbolRow {
            name: "kPEPE".to_owned(),
            price: 0.008421,
            change_pct: change_pct(0.008421, 0.007933),
            volume: 210_000_000.0,
            funding_pct: 0.0031,
            leverage: 10.0,
            open_interest: 4_100_000_000.0,
            prev: 0.007933,
            maintenance: 1.0 / 20.0,
            selected: false,
        },
    ]
}

pub fn demo_positions() -> Vec<Position> {
    vec![
        demo_position(
            "BTC",
            -30.0,
            81_461.5,
            64_000.0,
            40.0,
            Some(174_000.0),
            -3_309_304.0,
        ),
        // The state the risk rail exists for, and the one no capture had
        // ever drawn: an isolated long most of the way to its cliff.
        // Isolated, so the cross maintenance the equity bar reads is its own.
        demo_position("ETH", 40.0, 3_600.0, 3_540.0, 25.0, None, -142.0),
        // Small enough against the account that the venue reports no cliff
        // for it at all. The column says so rather than printing a zero, and
        // the rail beside it is empty because there is nothing to travel
        // toward — not because nothing has been travelled.
        demo_position("SOL", 12.0, 151.4, 148.62, 20.0, Some(0.0), -8.0),
    ]
}

/// A fixture position whose figures follow from the ones that are chosen,
/// through the same arithmetic the panel uses. Hand-typed money drifts from
/// the fields beside it and reads exactly as plausibly as the correct value.
fn demo_position(
    coin: &str,
    size: f64,
    entry: f64,
    mark: f64,
    leverage: f64,
    reported_liq: Option<f64>,
    funding: f64,
) -> Position {
    // A cross position is liquidated against the whole account, so its cliff
    // is the exchange's to report and arrives with it; an isolated one is
    // liquidated against its own margin and has the closed form. Which of the
    // two a position is, is that same fact, so it is not asked for twice.
    let mode = if reported_liq.is_some() {
        "cross"
    } else {
        "isolated"
    };
    let maintenance = 1.0 / (leverage * 2.0);
    let liq = reported_liq
        .unwrap_or_else(|| ticket_liquidation(entry, leverage, maintenance, size > 0.0));
    let pnl = (mark - entry) * size;
    let margin = entry * size.abs() / leverage;
    Position {
        coin: coin.to_owned(),
        size,
        entry,
        mark,
        liq,
        pnl,
        roe_pct: pnl / margin * 100.0,
        margin,
        risk: liquidation_travel(entry, mark, liq) * RISK_RAIL_WIDTH,
        leverage,
        margin_mode: mode.to_owned(),
        funding,
    }
}

/// A tape with candles already in it, focused on the market the other
/// fixtures describe. The chart is the largest panel here and the only one
/// that never appeared in a deterministic render, because its candles live
/// behind a lock the feed fills rather than in app state.
pub fn demo_candles() -> Tape {
    demo_candles_at(64_000.0)
}

/// Candles that end where the market is quoting. A chart drawn around another
/// price is a chart of another market, and it is the largest panel on screen.
pub fn demo_candles_at(last: f64) -> Tape {
    demo_candles_for("BTC".to_owned(), last)
}

/// Candles whose shape is a share of the price rather than a number of
/// dollars, so a market worth a fraction of a cent gets a chart and not a
/// flat line.
pub fn demo_candles_for(coin: String, last: f64) -> Tape {
    let tape = tape_focus(tape_new(), coin, "1m".to_owned());
    let mut candles = Vec::new();
    // The walk below lands its last close at `base + 519.5 * ...`; starting
    // from what that leaves puts the tip on the price the market quotes.
    let swing = last * 0.008125;
    let creep = last * 0.0000234;
    let wick = last * 0.0009375;
    let base = last - (119.0_f64 / 9.0).sin() * swing - 119.0 * creep;
    let mut close = base - wick;
    for step in 0..120 {
        // A shape rather than a straight line, so the moving averages have
        // something to say and the plot is not a diagonal.
        let drift = ((step as f64) / 9.0).sin() * swing;
        let open = close;
        close = base + drift + (step as f64) * creep;
        candles.push(Candle {
            ts: 1_786_110_000 + step * 60,
            open,
            high: open.max(close) + wick,
            low: open.min(close) - wick,
            close,
            volume: 40.0 + drift.abs() / last * 64_000.0,
        });
    }
    *lock(&tape.candles) = candles;
    tape
}

/// The account those positions belong to. Built the way a parsed one is, so
/// the rail and its percentage cannot disagree with the equity beside them.
pub fn demo_account() -> Account {
    demo_account_of(demo_positions(), demo_symbols(), 3_761_182.51, 2_200.0)
}

/// An account whose equity is nearly all spoken for: a long that has moved
/// against it, on collateral that cannot absorb much more. The equity bar is
/// the account's own distance to the margin engine, and until this fixture
/// existed no capture had drawn it anywhere but at rest.
/// A candle out of the fixture tape, for rendering the readout the crosshair
/// fills. Taken from the tape rather than invented, so what the row says is
/// what the chart is drawing under it.
/// A venue's worth of markets, so the list is longer than the panel that
/// holds it. Every capture so far held four, and a list that fits answers
/// nothing about a list that does not.
///
/// Generated rather than typed, so the ordering and the arithmetic the
/// fixture tests check hold by construction however many there are.
pub fn demo_symbols_many() -> Vec<SymbolRow> {
    const NAMES: [&str; 20] = [
        "AVAX", "LINK", "ARB", "OP", "SUI", "TIA", "SEI", "INJ", "APT", "DOT", "ATOM", "NEAR",
        "LDO", "AAVE", "CRV", "MKR", "RUNE", "FTM", "GALA", "IMX",
    ];
    let mut rows = demo_symbols();
    let floor = rows.last().map_or(1.0e8, |row| row.volume);
    for (step, name) in NAMES.iter().enumerate() {
        let step = step as f64;
        let price = 42.5 / (1.0 + step * 0.35);
        let prev = price * (1.0 - 0.004 * (step % 7.0 - 3.0));
        let leverage = 20.0 - (step % 3.0) * 5.0;
        rows.push(SymbolRow {
            name: (*name).to_owned(),
            price,
            change_pct: change_pct(price, prev),
            volume: floor * 0.94_f64.powf(step + 1.0),
            funding_pct: 0.0008 - 0.00021 * (step % 5.0),
            leverage,
            open_interest: 120_000.0 * (step + 1.0),
            prev,
            maintenance: maintenance_fraction(leverage),
            selected: false,
        });
    }
    rows
}

/// A tape at the depth the feed keeps it, rather than three prints.
pub fn demo_tape_full() -> Vec<Trade> {
    let mid = 64_000.0;
    (0..60)
        .map(|step| {
            let tid = step + 1;
            let up = step % 3 != 1;
            Trade {
                ts: 1_786_117_888 - tid,
                price: if up { mid + 1.0 } else { mid - 1.0 },
                size: 0.05 + (step % 9) as f64 * 0.17,
                buy: up,
                sweep: if step % 7 == 0 { 3 } else { 1 },
                tid,
            }
        })
        .collect()
}

/// One beat of the market feed, for driving the handler that folds a beat
/// into the app. Carries the mids the fixtures are priced at, so what a
/// dispatched beat leaves is what a real one would.
/// A failure the way the feed reports one, for driving the handler that
/// takes it.
pub fn demo_feed_error() -> HlError {
    HlError {
        message: "Hyperliquid unreachable".to_owned(),
    }
}

pub fn demo_tick() -> MarketTick {
    demo_tick_at(64_000.0)
}

/// A beat that moves bitcoin somewhere, for walking what a beat does: the
/// prices it applies, the positions it re-marks, the levels it fires.
pub fn demo_tick_at(btc: f64) -> MarketTick {
    MarketTick {
        mids: demo_symbols()
            .into_iter()
            .map(|row| {
                let price = if row.name == "BTC" { btc } else { row.price };
                (row.name, price)
            })
            .collect(),
        trades: Vec::new(),
        book: Some(demo_book()),
        latency: 42,
        context: None,
    }
}

pub fn demo_hover() -> Option<CandleHit> {
    let tape = demo_candles();
    let candles = lock(&tape.candles);
    let candle = candles.get(60)?;
    Some(CandleHit {
        index: 60,
        ts: candle.ts,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
    })
}

pub fn demo_positions_at_risk() -> Vec<Position> {
    demo_account_at_risk().positions
}

pub fn demo_account_at_risk() -> Account {
    // A cross position dies with the account, so its cliff is where the
    // account's equity meets its requirement:
    //
    //     collateral + (mark - entry) * size == mark * size * maintenance
    //
    // which for 5 BTC bought at 64,000 on 34,000 of collateral, held to a
    // fortieth of a fortieth, puts the account 76 dollars from the engine.
    let positions = vec![demo_position(
        "BTC",
        5.0,
        64_000.0,
        58_000.0,
        40.0,
        Some(57_924.05),
        -820.0,
    )];
    let equity = 34_000.0 + positions[0].pnl;
    demo_account_of(positions, demo_symbols_at_risk(), equity, 0.0)
}

/// The maintenance requirement an account is actually held to, summed from
/// the positions that are held against the whole account rather than against
/// their own margin. An isolated position dies alone and does not enter it.
/// What a set of cross positions is held to, at each market's own rate.
///
/// The rate is the asset's, not the position's: a market capped at 40x holds
/// every position in it to half of that, whether the trader opened at 40x or
/// at 2x. Reading it off the position's chosen leverage overstates a
/// conservative position by exactly the factor it was conservative by.
fn cross_maintenance(positions: &[Position], markets: &[SymbolRow]) -> f64 {
    positions
        .iter()
        .filter(|held| held.margin_mode == "cross")
        .map(|held| {
            let fraction = markets
                .iter()
                .find(|row| row.name == held.coin)
                .map_or(0.0, |row| row.maintenance);
            held.mark * held.size.abs() * fraction
        })
        .sum()
}

fn demo_account_of(
    positions: Vec<Position>,
    markets: Vec<SymbolRow>,
    value: f64,
    withdrawable: f64,
) -> Account {
    let maintenance = cross_maintenance(&positions, &markets);
    let isolated: f64 = positions
        .iter()
        .filter(|held| held.margin_mode == "isolated")
        .map(|held| held.margin)
        .sum();
    let cross_value = value - isolated;
    Account {
        value,
        cross_value,
        pnl: positions.iter().map(|position| position.pnl).sum(),
        withdrawable,
        notional: positions
            .iter()
            .map(|position| position.mark * position.size.abs())
            .sum(),
        maintenance,
        health: margin_load(cross_value, maintenance) * RISK_RAIL_WIDTH,
        margin_pct: margin_load(cross_value, maintenance) * 100.0,
        positions,
    }
}

/// Fills and resting orders, including one fill still lit, so the flash a
/// just-printed fill wears is drawn rather than only decayed in a unit test.
pub fn demo_fills() -> Vec<Fill> {
    let fill = |tid: i64, price: f64, size: f64, buy: bool, closed_pnl: f64, heat: i64| Fill {
        coin: "BTC".to_owned(),
        // Inside the candle window `demo_candles` builds, so each lands on
        // its own candle rather than piling onto the last one.
        ts: 1_786_110_000 + (7 - tid) * 900,
        price,
        size,
        buy,
        closed_pnl,
        heat,
        tid,
    };
    vec![
        fill(1, 64_010.0, 0.25, false, 1_240.0, FLASH_STEPS),
        fill(2, 63_940.0, 0.50, true, 0.0, 1),
        fill(3, 63_880.0, 0.75, true, 0.0, 0),
    ]
}

pub fn demo_orders() -> Vec<Order> {
    vec![
        Order {
            coin: "BTC".to_owned(),
            buy: true,
            price: 63_600.0,
            size: 1.5,
            ts: now_ms() / 1_000 - 7_200,
        },
        Order {
            coin: "BTC".to_owned(),
            buy: false,
            price: 64_440.0,
            size: 0.8,
            ts: now_ms() / 1_000 - 240,
        },
    ]
}

/// One level already reached, so the panel's two readings both draw.
pub fn demo_alerts() -> Vec<Alert> {
    vec![
        Alert {
            coin: "BTC".to_owned(),
            price: 64_100.0,
            above: true,
            fired: true,
        },
        // A level on a market the list is not showing. Alerts outlive the
        // market they were set from, so a row has to say which one it means.
        Alert {
            coin: "ETH".to_owned(),
            price: 3_400.0,
            above: false,
            fired: false,
        },
    ]
}

/// A book and a tape to go with them, so the whole terminal renders from
/// fixtures rather than from an exchange.
pub fn demo_book() -> Book {
    demo_book_at(64_000.0)
}

/// A book around the price its market is quoting. Depth that sat at another
/// price would be a book from another market: the panel walks it to say what
/// crossing costs, and the answer would be about a price nothing is trading
/// at.
pub fn demo_book_at(mid: f64) -> Book {
    demo_book_ticked(mid, 1.0)
}

/// A book whose levels are a tick apart. The tick is the market's, not
/// bitcoin's: a dollar between levels is the whole market on a coin worth a
/// fraction of a cent.
pub fn demo_book_ticked(mid: f64, tick: f64) -> Book {
    let level = |price: f64, size: f64, total: f64, deepest: f64| Level {
        price,
        size,
        total,
        bar: total / deepest * BOOK_BAR_WIDTH,
    };
    Book {
        bids: vec![
            level(mid - tick, 1.4, 1.4, 6.0),
            level(mid - tick * 2.0, 2.1, 3.5, 6.0),
            level(mid - tick * 3.0, 2.5, 6.0, 6.0),
        ],
        // Reversed, as the feed leaves them: the best ask sits last, against
        // the spread, so the panel walks both lists top to bottom.
        asks: vec![
            level(mid + tick * 3.0, 3.0, 7.0, 7.0),
            level(mid + tick * 2.0, 2.2, 4.0, 7.0),
            level(mid + tick, 1.8, 1.8, 7.0),
        ],
        spread: tick * 2.0,
        spread_pct: tick * 2.0 / mid * 100.0,
        mid,
    }
}

pub fn demo_tape() -> Vec<Trade> {
    demo_tape_at(64_000.0)
}

pub fn demo_tape_at(mid: f64) -> Vec<Trade> {
    demo_tape_ticked(mid, 1.0)
}

pub fn demo_tape_ticked(mid: f64, tick: f64) -> Vec<Trade> {
    let print = |tid: i64, price: f64, size: f64, buy: bool, sweep: i64| Trade {
        ts: 1_786_117_888 - tid,
        price,
        size,
        buy,
        sweep,
        tid,
    };
    vec![
        print(1, mid + tick, 0.53, true, 2),
        print(2, mid - tick, 1.20, false, 1),
        print(3, mid + tick, 0.08, true, 1),
    ]
}

/// Left gap the header keeps clear so its content never sits under the macOS
/// traffic lights, which float over the fullsize content view. The rightmost
/// button ends near 74pt; everywhere else the header owns its full width.
pub fn header_inset() -> f64 {
    if cfg!(target_os = "macos") { 78.0 } else { 0.0 }
}

/// This account's fills on one market, as chart glyphs: a buy points up out
/// of its price, a sell points down into it.
fn fill_markers(fills: &[Fill], coin: &str, palette: &theme::Palette) -> Vec<ChartMarker> {
    fills
        .iter()
        .filter(|fill| fill.coin == coin)
        .map(|fill| {
            let (shape, color) = if fill.buy {
                (MarkerShape::ArrowUp, palette.success)
            } else {
                (MarkerShape::ArrowDown, palette.destructive)
            };
            let label = if fill.closed_pnl == 0.0 {
                fmt_size(fill.size)
            } else {
                fmt_signed_usd(fill.closed_pnl)
            };
            ChartMarker::new(fill.ts, fill.price, shape, color).label(label)
        })
        .collect()
}

/// The levels an open position is read against: where it was entered, and
/// where it dies. A cross-margin position reports no liquidation price.
fn position_lines(positions: &[Position], coin: &str, palette: &theme::Palette) -> Vec<PriceLine> {
    positions
        .iter()
        .filter(|position| position.coin == coin)
        .flat_map(|position| {
            [
                Some(
                    PriceLine::new(position.entry, palette.foreground)
                        .label(fmt_px(position.entry)),
                ),
                (position.liq > 0.0).then(|| {
                    PriceLine::new(position.liq, palette.destructive).label(fmt_px(position.liq))
                }),
            ]
        })
        .flatten()
        .collect()
}

/// Resting orders as levels: the price you are still waiting to trade at.
fn order_lines(orders: &[Order], coin: &str, palette: &theme::Palette) -> Vec<PriceLine> {
    orders
        .iter()
        .filter(|order| order.coin == coin)
        .map(|order| {
            let color = if order.buy {
                palette.success
            } else {
                palette.destructive
            };
            PriceLine::new(order.price, color).label(fmt_size(order.size))
        })
        .collect()
}

/// What the chart reports back: the candle under the cursor, and whether the
/// view has been taken back to the oldest candle the tape holds.
#[derive(Clone, Copy, PartialEq)]
pub struct ChartSignal {
    pub hover: Option<CandleHit>,
    pub older: bool,
}

/// The chart for the selected market, with this account's fills marked on it
/// and its levels drawn across it. It repaints on its own beat, so candles
/// the feed merges into the tape show without an app message per tick.
pub fn chart(
    tape: &Tape,
    fills: &[Fill],
    positions: &[Position],
    orders: &[Order],
    coin: &str,
) -> Element<'static, ChartSignal> {
    // One theme for the frame. Reading `.palette` from a fresh one in each
    // helper rebuilt the whole thing, font resolution included, four times per
    // view — and the view rebuilds on every beat of the feed.
    let theme = chart_theme();
    let palette = &theme.palette;
    // The chart is told the instrument's scale rather than deriving one from
    // whatever range is on screen. Derived, it reads the gridline step, which
    // is enough to tell two gridlines apart and not enough to tell two ticks
    // apart: a market quoted to a millionth had its last price tagged
    // 0.00842 while every other panel said 0.008421.
    let scale = {
        let candles = lock(&tape.candles);
        candles
            .last()
            .map_or(2, |candle| price_decimals(candle.close))
    };
    let chart = candle_chart_shared(tape.candles.clone(), &theme)
        .precision(scale)
        .height(Length::Fill)
        .live(BEAT)
        .moving_averages([20, 60])
        .price_lines(position_lines(positions, coin, palette))
        .price_lines(order_lines(orders, coin, palette))
        .markers(fill_markers(fills, coin, palette))
        .on_hover(|hover| ChartSignal {
            hover,
            older: false,
        })
        .on_reach_start(|hover| ChartSignal { hover, older: true });
    accessible(chart, StableId::new("trading-chart"), Role::Image)
        .label("Hyperliquid candlestick chart with this account's fills marked")
        .logical_id("trading-chart")
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture is read as evidence, so it has to be a state the exchange
    /// could actually report. Five of this loop's bugs were impossible states
    /// drawn convincingly, and a wrong number in the right column is the one
    /// kind of wrong a render cannot show.
    #[test]
    fn the_fixture_positions_are_arithmetically_possible() {
        for held in demo_positions() {
            let coin = &held.coin;
            assert!(
                (held.pnl - (held.mark - held.entry) * held.size).abs() < 0.01,
                "{coin}: unrealized does not follow from entry, mark and size"
            );
            assert!(
                (held.margin - held.entry * held.size.abs() / held.leverage).abs() < 0.01,
                "{coin}: margin does not follow from the leverage beside it"
            );
            assert!(
                (held.roe_pct - held.pnl / held.margin * 100.0).abs() < 0.01,
                "{coin}: return on equity is not that return over that equity"
            );
            assert!(
                (held.risk - liquidation_travel(held.entry, held.mark, held.liq) * RISK_RAIL_WIDTH)
                    .abs()
                    < 1e-9,
                "{coin}: the risk rail is not how far the mark has travelled"
            );
            assert!(
                (0.0..=RISK_RAIL_WIDTH).contains(&held.risk),
                "{coin}: the rail is drawn {} wide of {RISK_RAIL_WIDTH}",
                held.risk
            );
            if held.liq <= 0.0 {
                // No cliff reported, so there is no distance to it: the rail
                // must read empty rather than full.
                assert_eq!(held.risk, 0.0, "{coin}: a rail toward nothing");
                continue;
            }
            // A long is liquidated below its entry and a short above it.
            assert_eq!(
                held.liq < held.entry,
                held.size > 0.0,
                "{coin}: the cliff is on the wrong side of the entry"
            );
        }
    }

    #[test]
    fn an_order_is_rented_by_the_hour_and_the_ticket_says_what_that_costs() {
        let btc = symbol_row(demo_symbols(), "BTC".to_owned());
        // 0.00125% an hour on 192,000 of notional is 2.40 an hour, 57.60 a
        // day. A long pays it and a short is paid it.
        assert_eq!(
            funding_day(btc.clone(), "64,000.00".to_owned(), "3".to_owned(), true),
            "-$57.60/day"
        );
        assert_eq!(
            funding_day(btc.clone(), "64,000.00".to_owned(), "3".to_owned(), false),
            "+$57.60/day"
        );

        // A negative rate turns it around: shorts pay and longs are paid.
        let sol = symbol_row(demo_symbols(), "SOL".to_owned());
        let paid = funding_day(sol, "148.62".to_owned(), "100".to_owned(), true);
        assert!(
            paid.starts_with('+'),
            "a long is paid a negative rate: {paid}"
        );

        // Nothing to say without a market or a size.
        assert!(funding_day(None, "1".to_owned(), "1".to_owned(), true).is_empty());
        assert!(funding_day(btc, "64,000.00".to_owned(), "".to_owned(), true).is_empty());
    }

    #[test]
    fn an_order_says_where_it_leaves_the_account_against_the_engine() {
        let account = demo_account_at_risk();
        let btc = symbol_row(demo_symbols_at_risk(), "BTC".to_owned());
        let load = |size: &str, buy: bool, market: Option<SymbolRow>| {
            order_load(
                Some(account.clone()),
                market
                    .as_ref()
                    .map_or_else(String::new, |row| row.name.clone()),
                size.to_owned(),
                buy,
                market,
            )
        };

        // Five bitcoin long already, ninety one percent of the way there.
        // Another five doubles what the account is held to, and it was
        // already past what its equity covers.
        assert_eq!(load("5", true, btc.clone()), "91% → 100%");
        // Selling into it is the way back: half the position, half the
        // requirement.
        assert_eq!(load("2.5", false, btc.clone()), "91% → 45%");
        // Closing it leaves nothing to be held to.
        assert_eq!(load("5", false, btc.clone()), "91% → 0%");

        // A different market adds its own requirement at its own rate, and
        // the isolated position already open there is not the account's to
        // answer for, so it is not what the order is measured against.
        let eth = symbol_row(demo_symbols_at_risk(), "ETH".to_owned());
        assert_eq!(load("1", true, eth), "91% → 92%");

        // Nothing to say without an account, a size, or a market.
        assert!(order_load(None, "BTC".to_owned(), "5".to_owned(), true, btc.clone()).is_empty());
        assert!(load("", true, btc).is_empty());
        assert!(load("5", true, None).is_empty());
    }

    /// The equity bar is the account's distance to the margin engine, and it
    /// has to be the distance these positions actually put it at. A hand-typed
    /// requirement read 38% loaded beside a cross position whose own rail read
    /// nothing travelled — two risk figures on one screen disagreeing.
    #[test]
    fn the_fixture_account_is_held_to_what_its_positions_require() {
        for (account, markets) in [
            (demo_account(), demo_symbols()),
            (demo_account_at_risk(), demo_symbols_at_risk()),
        ] {
            assert!(
                (account.pnl - account.positions.iter().map(|held| held.pnl).sum::<f64>()).abs()
                    < 0.01,
                "unrealized is not the sum of the positions under it"
            );
            assert!(
                (account.maintenance - cross_maintenance(&account.positions, &markets)).abs()
                    < 0.01,
                "the requirement is not what these positions are held to"
            );
            assert!(
                (account.margin_pct
                    - margin_load(account.cross_value, account.maintenance) * 100.0)
                    .abs()
                    < 1e-9,
                "the figure and the bar disagree"
            );
            assert!(
                (account.health - account.margin_pct / 100.0 * RISK_RAIL_WIDTH).abs() < 1e-9,
                "the bar is not that figure"
            );
        }

        // An isolated position is liquidated against its own margin, so it
        // asks nothing of the account's requirement.
        let isolated: Vec<Position> = demo_positions()
            .into_iter()
            .filter(|held| held.margin_mode == "isolated")
            .collect();
        assert!(!isolated.is_empty(), "the fixture holds one to check");
        assert_eq!(cross_maintenance(&isolated, &demo_symbols()), 0.0);

        // The rate is the asset's cap, not the leverage the trader chose. A
        // conservative position held to its own leverage reads as many times
        // more dangerous as it was conservative.
        let careful = vec![demo_position(
            "BTC",
            1.0,
            64_000.0,
            64_000.0,
            5.0,
            Some(52_000.0),
            0.0,
        )];
        assert!(
            (cross_maintenance(&careful, &demo_symbols()) - 64_000.0 / 80.0).abs() < 0.01,
            "a 5x position on a 40x market is held to a fortieth of a fortieth"
        );

        // A position's mark is the feed's price for its market — every beat
        // sets one from the other — so a fixture that priced them apart is a
        // state no beat could produce. It read as an order halving a
        // requirement that had been summed at a different price.
        for (account, markets) in [
            (demo_account(), demo_symbols()),
            (demo_account_at_risk(), demo_symbols_at_risk()),
        ] {
            for held in &account.positions {
                let market = markets
                    .iter()
                    .find(|row| row.name == held.coin)
                    .unwrap_or_else(|| panic!("{} is held but not listed", held.coin));
                assert!(
                    (held.mark - market.price).abs() < 0.01,
                    "{}: marked at {} while the market says {}",
                    held.coin,
                    held.mark,
                    market.price
                );
            }
        }

        // The chart is the largest panel on screen, and one drawn around
        // another price is a chart of another market.
        for (price, tape) in [
            (64_000.0, demo_candles()),
            (58_000.0, demo_candles_at(58_000.0)),
        ] {
            let candles = lock(&tape.candles);
            let last = candles.last().expect("the fixture tape has candles");
            assert!(
                (last.close - price).abs() < 0.01,
                "the tape ends at {} beside a market at {price}",
                last.close
            );
        }

        // The book and the tape belong to the market they are shown beside.
        // Depth at another price answers what crossing costs with a number
        // about a price nothing is trading at.
        for (markets, book, prints) in [
            (demo_symbols(), demo_book(), demo_tape()),
            (
                demo_symbols_at_risk(),
                demo_book_at(58_000.0),
                demo_tape_at(58_000.0),
            ),
        ] {
            let price = markets[0].price;
            assert!(
                (book.mid - price).abs() <= book.spread,
                "the book sits at {} beside a market at {price}",
                book.mid
            );
            for print in &prints {
                assert!(
                    (print.price - price).abs() <= book.spread,
                    "a print at {} beside a market at {price}",
                    print.price
                );
            }
        }

        // A resting order is one the book has not reached: a buy above the
        // market or a sell below it would have filled, and a terminal showing
        // one is showing an order that does not exist.
        let market = demo_symbols()[0].price;
        for order in demo_orders() {
            assert_eq!(
                order.price < market,
                order.buy,
                "a resting {} at {} against a market at {market}",
                if order.buy { "buy" } else { "sell" },
                order.price
            );
        }

        // An alert that has not fired is one the mark has not reached. One
        // waiting above a mark already past it would have gone off.
        for alert in demo_alerts() {
            if alert.fired {
                continue;
            }
            let mark = demo_symbols()
                .into_iter()
                .find(|row| row.name == alert.coin)
                .map(|row| row.price)
                .unwrap_or_else(|| panic!("{} is watched but not listed", alert.coin));
            assert_eq!(
                alert.price > mark,
                alert.above,
                "{} waits {} {} with the mark at {mark}",
                alert.coin,
                if alert.above { "above" } else { "below" },
                alert.price
            );
        }

        // At rest and at risk are genuinely different readings, or the
        // fixture pair is only one fixture.
        assert!(demo_account().margin_pct < 5.0);
        assert!(demo_account_at_risk().margin_pct > 50.0);
    }

    /// The long list is generated, so it is checked the same way rather than
    /// trusted: a list that breaks the ordering the panel assumes is a list
    /// the panel draws in the wrong order, quietly.
    #[test]
    fn the_long_market_list_is_the_same_shape_as_the_short_one() {
        let rows = demo_symbols_many();
        assert!(rows.len() > 20, "longer than any panel that holds it");
        for pair in rows.windows(2) {
            assert!(
                pair[0].volume >= pair[1].volume,
                "{} outranks {} on volume",
                pair[1].name,
                pair[0].name
            );
        }
        for row in &rows {
            assert!(row.price > 0.0 && row.leverage > 0.0, "{}", row.name);
            assert!(
                (row.maintenance - maintenance_fraction(row.leverage)).abs() < 1e-12,
                "{}: maintenance is half the margin at the cap",
                row.name
            );
            assert!(
                (row.change_pct - change_pct(row.price, row.prev)).abs() < 1e-9,
                "{}: the change is not this price against that close",
                row.name
            );
        }
        assert_eq!(
            rows.iter().filter(|row| row.selected).count(),
            1,
            "one market is the one being watched"
        );
    }

    /// The fixture markets have to be the shape the parser leaves: volume
    /// descending, one selection, and a maintenance that matches the cap.
    #[test]
    fn the_fixture_markets_are_the_shape_the_parser_leaves() {
        let rows = demo_symbols();
        assert!(rows.len() > 1, "a list of one hides what a list does");
        for pair in rows.windows(2) {
            assert!(
                pair[0].volume >= pair[1].volume,
                "{} outranks {} on volume",
                pair[1].name,
                pair[0].name
            );
        }
        for row in &rows {
            assert!(
                (row.maintenance - 1.0 / (row.leverage * 2.0)).abs() < 1e-12,
                "{}: maintenance is half the margin at the cap",
                row.name
            );
            assert!(
                (row.change_pct - change_pct(row.price, row.prev)).abs() < 1e-9,
                "{}: the change is not this price against that close",
                row.name
            );
        }
    }

    /// The book the fixture holds: bids 63,999/998/997 at 1.4/2.1/2.5 and
    /// asks 64,001/002/003 at 1.8/2.2/3.0, around a 64,000 mid.
    #[test]
    fn crossing_the_spread_is_priced_at_the_levels_that_are_there() {
        let book = || Some(demo_book());

        // Inside the best ask: one level, one price, and the slippage is the
        // half spread and nothing else.
        let small = book_impact(book(), "1.0".to_owned(), true);
        assert!(small.ready && !small.short);
        assert!((small.paid - 64_001.0).abs() < 1e-9);
        assert!((small.filled - 1.0).abs() < 1e-9);
        assert!((small.slippage_pct - (1.0 / 64_000.0 * 100.0)).abs() < 1e-9);

        // Through two levels: 1.8 at 64,001 and 1.2 at 64,002.
        let deeper = book_impact(book(), "3.0".to_owned(), true);
        let expected = (1.8 * 64_001.0 + 1.2 * 64_002.0) / 3.0;
        assert!((deeper.paid - expected).abs() < 1e-9, "{}", deeper.paid);
        assert!(deeper.paid > small.paid, "depth costs more, never less");

        // Selling walks the bids down, and the slippage still reads positive:
        // it is what the crossing costs, not which way the price went.
        let sold = book_impact(book(), "2.0".to_owned(), false);
        let expected = (1.4 * 63_999.0 + 0.6 * 63_998.0) / 2.0;
        assert!((sold.paid - expected).abs() < 1e-9);
        assert!(sold.slippage_pct > 0.0);
    }

    /// The walk must start at the best price. The asks are stored reversed for
    /// the panel to draw downward, so a walk that trusted the order would
    /// quote the worst level in the book as the first one filled.
    #[test]
    fn a_sweep_starts_at_the_best_price_not_the_first_row() {
        let impact = book_impact(Some(demo_book()), "0.5".to_owned(), true);
        assert!(
            (impact.paid - 64_001.0).abs() < 1e-9,
            "a small buy pays the best ask, not {}",
            impact.paid
        );
    }

    #[test]
    fn a_size_the_book_cannot_fill_says_so_rather_than_inventing_depth() {
        let impact = book_impact(Some(demo_book()), "100".to_owned(), true);
        assert!(impact.short, "7.0 of asks cannot fill 100");
        assert!((impact.filled - 7.0).abs() < 1e-9);
        assert!(
            (impact.paid - (1.8 * 64_001.0 + 2.2 * 64_002.0 + 3.0 * 64_003.0) / 7.0).abs() < 1e-9,
            "and it is priced over what is actually there"
        );

        assert!(!book_impact(Some(demo_book()), "0".to_owned(), true).ready);
        assert!(!book_impact(None, "1".to_owned(), true).ready);
    }

    /// The address the app opens on, which is a real account with real
    /// positions to check the valuation arithmetic against.
    const WATCHED: &str = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a";

    #[test]
    fn symbols_drop_delisted_and_sort_by_volume() {
        let response = json!([
            {
                "universe": [
                    { "name": "BTC", "maxLeverage": 40 },
                    { "name": "OLD", "maxLeverage": 5, "isDelisted": true },
                    { "name": "SOL", "maxLeverage": 20 },
                ]
            },
            [
                { "markPx": "64581.0", "prevDayPx": "64848.0", "dayNtlVlm": "1117558936.7", "funding": "0.0000042378" },
                { "markPx": "1.0", "prevDayPx": "1.0", "dayNtlVlm": "9999999999.0", "funding": "0.0" },
                { "markPx": "150.0", "prevDayPx": "120.0", "dayNtlVlm": "2000000000.0", "funding": "-0.00001" },
            ]
        ]);

        let rows = parse_symbols(&response);
        assert_eq!(rows.len(), 2, "delisted markets are not tradeable");
        assert_eq!(rows[0].name, "SOL", "highest volume leads");
        assert!((rows[0].change_pct - 25.0).abs() < 1e-9);
        assert!(rows[0].funding_pct < 0.0);
        assert_eq!(rows[1].name, "BTC");
        assert!(rows[1].change_pct < 0.0, "BTC is below its previous close");
    }

    #[test]
    fn positions_read_out_side_mark_and_pnl() {
        let response = json!({
            "marginSummary": {
                "accountValue": "10000.0",
                "totalMarginUsed": "477.86"
            },
            "withdrawable": "9000.0",
            "assetPositions": [
                { "type": "oneWay", "position": {
                    "coin": "BTC", "szi": "0.5", "entryPx": "60000.0",
                    "positionValue": "32000.0", "unrealizedPnl": "2000.0",
                    "returnOnEquity": "0.25", "liquidationPx": "45000.0",
                    "marginUsed": "8000.0", "cumFunding": { "sinceOpen": "-1.25" }
                }},
                { "type": "oneWay", "position": {
                    "coin": "ETH", "szi": "-2.0", "entryPx": "3000.0",
                    "positionValue": "5800.0", "unrealizedPnl": "200.0",
                    "returnOnEquity": "0.1", "liquidationPx": null,
                    "marginUsed": "1000.0", "cumFunding": { "sinceOpen": "0.5" }
                }},
            ]
        });

        let account = parse_account(&response);
        assert_eq!(account.value, 10_000.0);
        assert_eq!(account.pnl, 2_200.0, "summary PnL is the open positions");

        // A signed size is the side: the panel reads long or short off it.
        let long = &account.positions[0];
        assert!(long.size > 0.0);
        assert_eq!(long.mark, 64_000.0, "mark comes from notional over size");
        assert!((long.roe_pct - 25.0).abs() < 1e-9);

        let short = &account.positions[1];
        assert!(short.size < 0.0);
        assert_eq!(short.mark, 2_900.0);
        assert_eq!(short.liq, 0.0, "a null liquidation price reads as none");
    }

    #[test]
    fn fills_carry_a_side_and_second_precision_time() {
        let fills = parse_fills(
            &json!([
                { "coin": "BTC", "px": "64000.0", "sz": "0.1", "side": "B", "time": 1_786_092_480_123i64, "closedPnl": "0.0", "dir": "Open Long", "tid": 1 },
                { "coin": "BTC", "px": "64500.0", "sz": "0.1", "side": "A", "time": 1_786_092_540_000i64, "closedPnl": "50.0", "dir": "Close Long", "tid": 2 },
            ]),
            FLASH_STEPS,
        );

        assert!(fills[0].buy);
        assert_eq!(fills[0].ts, 1_786_092_480, "chart timestamps are seconds");
        assert!(!fills[1].buy);
        assert_eq!(fills[1].closed_pnl, 50.0);
        assert_eq!(fills[1].heat, FLASH_STEPS, "a pushed fill arrives lit");
    }

    #[test]
    fn candles_merge_in_place_and_stale_responses_are_dropped() {
        let mut tape = vec![
            Candle {
                ts: 60,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 10.0,
            },
            Candle {
                ts: 120,
                open: 1.5,
                high: 1.8,
                low: 1.4,
                close: 1.6,
                volume: 5.0,
            },
        ];
        merge(
            &mut tape,
            parse_candles(&json!([
                { "t": 120_000, "o": "1.5", "h": "2.2", "l": "1.4", "c": "2.1", "v": "9.0" },
                { "t": 180_000, "o": "2.1", "h": "2.3", "l": "2.0", "c": "2.2", "v": "1.0" },
            ])),
        );
        assert_eq!(tape.len(), 3, "the live candle is replaced, not appended");
        assert_eq!(tape[1].close, 2.1);
        assert_eq!(tape[2].ts, 180);

        // Focus guards the tape against a response for the previous market.
        let held = tape_focus(tape_new(), "BTC".into(), "1m".into());
        let switched = tape_focus(held.clone(), "ETH".into(), "5m".into());
        assert_eq!(*lock(&switched.focus), "ETH:5m");
        assert!(lock(&switched.candles).is_empty());
    }

    #[test]
    fn the_chart_marks_this_market_only() {
        let fills = vec![
            Fill {
                coin: "BTC".into(),
                ts: 100,
                price: 64_000.0,
                size: 0.5,
                buy: true,
                closed_pnl: 0.0,
                heat: 0,
                tid: 1,
            },
            Fill {
                coin: "ETH".into(),
                ts: 110,
                price: 3_000.0,
                size: 2.0,
                buy: true,
                closed_pnl: 0.0,
                heat: 0,
                tid: 2,
            },
            Fill {
                coin: "BTC".into(),
                ts: 120,
                price: 64_500.0,
                size: 0.5,
                buy: false,
                closed_pnl: 250.0,
                heat: 0,
                tid: 3,
            },
        ];

        let markers = fill_markers(&fills, "BTC", &chart_theme().palette);
        assert_eq!(
            markers.len(),
            2,
            "another market's fills stay off the chart"
        );
        assert_eq!(markers[0].shape, MarkerShape::ArrowUp, "a buy points up");
        assert_eq!(markers[0].ts, 100);
        assert_eq!(markers[0].label.as_deref(), Some("0.500"));
        assert_eq!(
            markers[1].shape,
            MarkerShape::ArrowDown,
            "a sell points down"
        );
        assert_ne!(markers[0].color, markers[1].color, "sides read apart");

        let positions = vec![
            Position {
                coin: "BTC".into(),
                size: 0.5,
                entry: 60_000.0,
                mark: 64_000.0,
                liq: 45_000.0,
                pnl: 2_000.0,
                roe_pct: 25.0,
                margin: 8_000.0,
                risk: 0.0,
                leverage: 20.0,
                margin_mode: "cross".into(),
                funding: 0.0,
            },
            Position {
                coin: "ETH".into(),
                size: -2.0,
                entry: 3_000.0,
                mark: 2_900.0,
                liq: 0.0,
                pnl: 200.0,
                roe_pct: 10.0,
                margin: 1_000.0,
                risk: 0.0,
                leverage: 20.0,
                margin_mode: "cross".into(),
                funding: 0.0,
            },
        ];
        let lines = position_lines(&positions, "BTC", &chart_theme().palette);
        assert_eq!(lines.len(), 2, "entry and liquidation");
        assert_eq!(lines[0].price, 60_000.0);
        assert_eq!(lines[1].price, 45_000.0);
        assert_eq!(
            position_lines(&positions, "ETH", &chart_theme().palette).len(),
            1,
            "no liquidation price means no line"
        );
    }

    fn tick_at<const MARKETS: usize>(prices: [(&str, f64); MARKETS]) -> MarketTick {
        MarketTick {
            mids: prices
                .into_iter()
                .map(|(coin, price)| (coin.to_owned(), price))
                .collect(),
            ..MarketTick::default()
        }
    }

    /// Between the polls that restate them, positions are worth whatever the
    /// feed's last price says they are.
    #[test]
    fn the_feed_revalues_positions_and_the_equity_holding_them() {
        let account = parse_account(&json!({
            "marginSummary": { "accountValue": "12200.0", "totalMarginUsed": "3500.0" },
            "withdrawable": "8700.0",
            "assetPositions": [
                { "position": {
                    "coin": "BTC", "szi": "0.5", "entryPx": "60000.0",
                    "positionValue": "32000.0", "unrealizedPnl": "2000.0",
                    "returnOnEquity": "1.0", "liquidationPx": "45000.0",
                    "marginUsed": "2133.33", "leverage": { "type": "cross", "value": 15 }
                }},
                { "position": {
                    "coin": "ETH", "szi": "-2.0", "entryPx": "3000.0",
                    "positionValue": "5800.0", "unrealizedPnl": "200.0",
                    "returnOnEquity": "0.1666666667", "liquidationPx": "3600.0",
                    "marginUsed": "1160.0", "leverage": { "type": "cross", "value": 5 }
                }},
            ]
        }));
        assert_eq!(account.pnl, 2_200.0);

        // BTC runs another $2k; the short is unchanged.
        let marked = mark_positions(
            account.positions.clone(),
            tick_at([("BTC", 66_000.0), ("ETH", 2_900.0)]),
        );

        let long = &marked[0];
        assert_eq!(long.mark, 66_000.0);
        assert_eq!(long.pnl, 3_000.0, "half a coin, six thousand up");
        // Return is on the margin the position opened with, 60000 * 0.5 / 15:
        // the reported 100% plus the 50% the last $2,000 added.
        assert!((long.roe_pct - 150.0).abs() < 1e-9);
        assert_eq!(
            long.margin, account.positions[0].margin,
            "what the position ties up is the margin engine's to say"
        );
        assert_eq!(
            long.risk, 0.0,
            "a long in profit has travelled nowhere toward its cliff"
        );

        let short = &marked[1];
        assert_eq!(short.pnl, 200.0, "an unmoved price restates the same PnL");
        assert!((short.roe_pct - (200.0 / 1_200.0 * 100.0)).abs() < 1e-6);

        // The rail measures the ground between the entry and the cliff, so a
        // mark halfway down is half a rail.
        let falling = mark_positions(
            account.positions.clone(),
            tick_at([("BTC", 52_500.0), ("ETH", 2_900.0)]),
        );
        assert!(falling[0].pnl < 0.0);
        assert!((falling[0].risk - RISK_RAIL_WIDTH / 2.0).abs() < 1e-9);

        // Equity carries unrealized PnL, so it moves by what the positions
        // just made, and re-running it is not compounding.
        let revalued = mark_account(Some(account.clone()), marked.clone()).expect("an account");
        assert_eq!(revalued.pnl, 3_200.0);
        assert_eq!(revalued.value, 13_200.0, "equity moved by the $1k gained");
        assert!(
            mark_account(Some(revalued.clone()), marked).as_ref() == Some(&revalued),
            "re-valuing the same prices twice changes nothing"
        );
        assert_eq!(revalued.notional, 66_000.0 * 0.5 + 2_900.0 * 2.0);
        assert_eq!(
            revalued.withdrawable, account.withdrawable,
            "what the margin engine will release waits for the next poll"
        );

        // A market the feed said nothing about keeps what it had.
        let quiet = mark_positions(account.positions.clone(), MarketTick::default());
        assert!(quiet == account.positions, "a silent beat restates nothing");
        assert!(
            mark_account(None, quiet).is_none(),
            "no account, nothing to re-total"
        );
    }

    #[test]
    fn the_health_rail_measures_equity_against_the_maintenance_call() {
        assert_eq!(margin_load(10_000.0, 0.0), 0.0, "nothing open, no cliff");
        assert_eq!(margin_load(10_000.0, 2_500.0), 0.25, "a quarter claimed");
        assert_eq!(margin_load(10_000.0, 10_000.0), 1.0, "the call lands here");
        assert_eq!(margin_load(8_000.0, 10_000.0), 1.0, "past it, clamped");
        assert_eq!(margin_load(0.0, 10_000.0), 1.0, "wiped out is not empty");
        assert_eq!(margin_load(-500.0, 10_000.0), 1.0, "nor is owing money");
        // An account with no requirement is not at risk however small it is.
        assert_eq!(margin_load(0.0, 0.0), 0.0);

        let account = parse_account(&json!({
            "marginSummary": { "accountValue": "10000.0", "totalMarginUsed": "3000.0" },
            "crossMarginSummary": { "accountValue": "10000.0", "totalMarginUsed": "3000.0" },
            "crossMaintenanceMarginUsed": "2000.0",
            "withdrawable": "7000.0",
            "assetPositions": [
                { "position": {
                    "coin": "BTC", "szi": "1.0", "entryPx": "60000.0",
                    "positionValue": "60000.0", "unrealizedPnl": "0.0",
                    "returnOnEquity": "0.0", "liquidationPx": "45000.0",
                    "marginUsed": "3000.0", "leverage": { "type": "cross", "value": 20 }
                }},
            ]
        }));
        assert_eq!(account.maintenance, 2_000.0);
        assert_eq!(
            account.health,
            RISK_RAIL_WIDTH / 5.0,
            "a fifth of the equity is spoken for"
        );
        // The rail is a length, and a length is not readable aloud. The same
        // reading has to exist as a number for the accessibility tree.
        assert_eq!(account.margin_pct, 20.0);

        // The requirement is the cross one, so the equity beside it has to be
        // the cross one. An account holding most of its money as margin behind
        // isolated positions has far less to meet the call with than its total
        // says, and dividing by the total reads it as comfortable while the
        // engine is one tick away.
        let locked = parse_account(&json!({
            "marginSummary": { "accountValue": "10000.0", "totalMarginUsed": "9900.0" },
            "crossMarginSummary": { "accountValue": "1000.0", "totalMarginUsed": "900.0" },
            "crossMaintenanceMarginUsed": "900.0",
            "withdrawable": "100.0",
            "assetPositions": []
        }));
        assert_eq!(locked.value, 10_000.0, "the equity readout is the total");
        assert_eq!(locked.cross_value, 1_000.0);
        assert_eq!(
            locked.margin_pct, 90.0,
            "ninety percent gone, not the nine the total would say"
        );
        assert_eq!(fmt_share(account.margin_pct), "20%");

        // The requirement waits for the next poll, but the equity it is
        // measured against does not: losing half the account doubles the rail.
        let sinking = mark_account(
            Some(account.clone()),
            mark_positions(account.positions.clone(), tick_at([("BTC", 55_000.0)])),
        )
        .expect("an account");
        assert_eq!(sinking.value, 5_000.0, "the position gave back $5k");
        assert_eq!(sinking.health, RISK_RAIL_WIDTH * 2.0 / 5.0);
        assert_eq!(
            sinking.maintenance, account.maintenance,
            "the requirement is the margin engine's to restate"
        );

        // A read-only session has no account and so no rail to draw.
        let browsing = parse_account(&json!({ "marginSummary": {} }));
        assert_eq!(browsing.health, 0.0);
    }

    #[test]
    fn one_aggressor_is_one_print_however_many_orders_it_ate() {
        // Two messages, one hash: a market order that took two resting orders
        // at the same price. That is one trade to watch, not two.
        let tape = parse_trades(
            &json!([
                { "coin": "BTC", "hash": "0xaa", "px": "64986.0", "sz": "0.2", "side": "A",
                  "time": 1_786_117_888_774i64, "tid": 1 },
                { "coin": "BTC", "hash": "0xaa", "px": "64986.0", "sz": "0.3", "side": "A",
                  "time": 1_786_117_888_774i64, "tid": 2 },
                { "coin": "BTC", "hash": "0xbb", "px": "64990.0", "sz": "1.0", "side": "B",
                  "time": 1_786_117_889_000i64, "tid": 3 },
                { "coin": "ETH", "hash": "0xdd", "px": "3000.0", "sz": "5.0", "side": "B",
                  "time": 1_786_117_889_100i64, "tid": 99 },
            ]),
            "BTC",
        );
        assert_eq!(
            tape.len(),
            2,
            "one row per aggressing order, and none from another market"
        );
        assert_eq!(tape[0].size, 0.5, "the sweep is what it took in total");
        assert_eq!(tape[0].sweep, 2);
        assert!(!tape[0].buy, "an ask print hit the bid");
        assert_eq!(tape[0].ts, 1_786_117_888, "seconds, like everything else");
        assert_eq!(tape[1].sweep, 1, "a single print wears no mark");
        assert!(tape[1].buy);
        assert_eq!(fmt_sweep(tape[1].sweep), "", "and no marker either");
        assert_eq!(fmt_sweep(tape[0].sweep), "×2");

        // A sweep across levels is priced at what the aggressor actually paid.
        let across = parse_trades(
            &json!([
                { "coin": "BTC", "hash": "0xcc", "px": "100.0", "sz": "1.0", "side": "B", "time": 0, "tid": 4 },
                { "coin": "BTC", "hash": "0xcc", "px": "102.0", "sz": "3.0", "side": "B", "time": 0, "tid": 5 },
            ]),
            "BTC",
        );
        assert_eq!(across[0].price, 101.5, "size-weighted, not the last level");

        // A missing hash is not an identity and must never merge two orders.
        let anonymous = parse_trades(
            &json!([
                { "coin": "BTC", "px": "1.0", "sz": "1.0", "side": "B", "time": 0, "tid": 6 },
                { "coin": "BTC", "px": "2.0", "sz": "1.0", "side": "B", "time": 0, "tid": 7 },
            ]),
            "BTC",
        );
        assert_eq!(anonymous.len(), 2, "no hash, no grouping");

        // The market switch this guards: a print already in flight for the
        // market just left must not be folded onto the one now on screen. The
        // sweep merge makes it worse than one stray row — a BTC print landing
        // beside an ETH one at the same hash would average their prices.
        assert!(
            parse_trades(
                &json!([{ "coin": "BTC", "hash": "0xee", "px": "64000.0", "sz": "1.0",
                          "side": "B", "time": 0, "tid": 8 }]),
                "ETH"
            )
            .is_empty(),
            "another market's print is not this market's tape"
        );
    }

    /// The chart is drawn by Rust and the panels around it by Ice, so the
    /// palette exists twice: once as tokens in `theme.ice` and once as the
    /// literals below. Nothing makes them agree, and the chart is half the
    /// screen — a drift would be obvious at runtime and invisible until then.
    /// Everything the extern block declares, the view has to read. A field
    /// Rust needs and Ice does not belongs in the struct and not in the
    /// declaration — `Fill.tid` is the pattern. A `sync` nothing calls is
    /// worse: it means an edit that was supposed to wire it up matched
    /// nothing, which has happened here four times, twice without a single
    /// test noticing, because a test can cover a function the screen never
    /// reaches.
    #[test]
    fn the_boundary_declares_only_what_the_view_reads() {
        const EXTERN: &str = include_str!("ui/extern/hyperliquid.ice");
        const APP: &str = include_str!("ui/app.ice");

        let mut dead: Vec<String> = Vec::new();
        for line in EXTERN.lines() {
            let line = line.trim_end();
            let Some(body) = line.strip_prefix("  ") else {
                continue;
            };
            if let Some(rest) = body
                .strip_prefix("sync ")
                .or(body.strip_prefix("component "))
            {
                let name = rest.split('(').next().unwrap_or(rest);
                if !APP.contains(&format!("{name}(")) {
                    dead.push(format!("`sync {name}` is declared and never called"));
                }
                continue;
            }
            // A struct: `Name(field:type, ...)`.
            let Some((name, fields)) = body.split_once('(') else {
                continue;
            };
            if !name.chars().next().is_some_and(char::is_uppercase) {
                continue;
            }
            for field in fields.trim_end_matches(')').split(", ") {
                let Some((field, _)) = field.split_once(':') else {
                    continue;
                };
                if !APP.contains(&format!(".{field}")) {
                    dead.push(format!("`{name}.{field}` is declared and never read"));
                }
            }
        }
        assert!(
            dead.is_empty(),
            "the boundary carries what nothing reads:\n  {}",
            dead.join("\n  ")
        );

        // A handler the app never routes to is dead however many tests
        // dispatch it. `dispatch` reaches anything by name, so a handler can
        // outlive the control that called it and still look exercised.
        let routed: Vec<&str> = APP
            .lines()
            .flat_map(|line| {
                // `| name _` is the error route of a `run` or a `stream`.
                [
                    "-> ", "| ", "change=", "submit=", "drag=", "dismiss=", "paste=",
                ]
                .iter()
                .filter_map(move |form| {
                    let at = line.find(form)? + form.len();
                    Some(line[at..].split([' ', '(']).next().unwrap_or_default())
                })
            })
            .collect();
        let orphans: Vec<&str> = APP
            .lines()
            .filter_map(|line| line.strip_prefix("on "))
            .map(|rest| rest.split('(').next().unwrap_or(rest).trim())
            .filter(|handler| *handler != "mount" && !routed.contains(handler))
            .collect();
        assert!(
            orphans.is_empty(),
            "handlers nothing routes to: {}",
            orphans.join(", ")
        );
    }

    #[test]
    fn the_chart_wears_the_same_palette_as_the_panels() {
        const THEME: &str = include_str!("ui/theme.ice");

        fn token(name: &str) -> Color {
            let line = THEME
                .lines()
                .map(str::trim)
                .find(|line| line.starts_with(&format!("{name} #")))
                .unwrap_or_else(|| panic!("theme.ice declares no `{name}`"));
            let hex = line.rsplit('#').next().expect("a colour");
            assert_eq!(hex.len(), 6, "`{name}` is not an opaque #RRGGBB");
            rgb(u32::from_str_radix(hex, 16).expect("hexadecimal"))
        }

        let chart = chart_theme().palette;
        assert_eq!(chart.background, token("bg"));
        assert_eq!(chart.foreground, token("fg"));
        assert_eq!(chart.muted_foreground, token("muted"));
        assert_eq!(chart.border, token("edge"));
        assert_eq!(chart.success, token("up"));
        assert_eq!(chart.destructive, token("down"));
        assert_eq!(chart.warning, token("faint"));

        // `accent` is the only slot with no counterpart, and deliberately so:
        // the moving averages are ink rather than colour, because the fills
        // and the position levels are the only long/short marks on the plot.
        assert_ne!(chart.accent, chart.success);
        assert_ne!(chart.accent, chart.destructive);
    }

    #[test]
    fn an_interactive_row_is_named_by_what_it_does() {
        // A book row starts an order, so it is named by the order and not by
        // the resting one it crosses: the ask is where you buy.
        assert_eq!(book_label(64_850.0, true), "Buy at 64,850.00");
        assert_eq!(book_label(64_849.0, false), "Sell at 64,849.00");

        let held = |size: f64| Position {
            coin: "BTC".into(),
            size,
            entry: 60_000.0,
            mark: 64_000.0,
            liq: 45_000.0,
            pnl: 0.0,
            roe_pct: 0.0,
            margin: 0.0,
            risk: 0.0,
            leverage: 20.0,
            margin_mode: "cross".into(),
            funding: 0.0,
        };
        // A row carrying a side, a size, an entry, a cliff and a PnL is worth
        // more than its ticker to somebody who cannot see the rest of it.
        assert_eq!(position_label(held(30.0)), "BTC long 30.00");

        let order = Order {
            coin: "BTC".into(),
            buy: true,
            price: 60_000.0,
            size: 0.5,
            ts: 0,
        };
        assert_eq!(order_label(order.clone()), "BTC buy 0.500 at 60,000.00");
        assert_eq!(
            order_label(Order {
                buy: false,
                ..order
            }),
            "BTC sell 0.500 at 60,000.00"
        );

        let fill = |closed_pnl: f64, buy: bool| Fill {
            coin: "BTC".into(),
            ts: 0,
            price: 64_000.0,
            size: 0.5,
            buy,
            closed_pnl,
            heat: 0,
            tid: 0,
        };
        // A fill that closed something is named by what it made; one that
        // opened, by what it took on. The row reads the same way.
        assert_eq!(
            fill_label(fill(250.0, false)),
            "BTC sold +$250.00 at 64,000.00"
        );
        assert_eq!(fill_label(fill(0.0, true)), "BTC bought 0.500 at 64,000.00");
        assert_eq!(
            position_label(held(-30.0)),
            "BTC short 30.00",
            "the size reads unsigned; the word carries the side"
        );
    }

    #[test]
    fn the_tape_reads_which_side_is_crossing() {
        let print = |size: f64, buy: bool| Trade {
            ts: 0,
            price: 1.0,
            size,
            buy,
            sweep: 1,
            tid: 0,
        };
        // Weighted by size, not by row count: one large seller against three
        // small buyers is a selling tape however the rows look.
        assert_eq!(
            tape_pressure(vec![
                print(1.0, true),
                print(1.0, true),
                print(1.0, true),
                print(9.0, false)
            ]),
            25.0
        );
        assert_eq!(tape_pressure(vec![print(4.0, true)]), 100.0);
        assert_eq!(tape_pressure(vec![print(4.0, false)]), 0.0);
        assert_eq!(
            tape_pressure(vec![print(1.0, true), print(1.0, false)]),
            50.0
        );
        // No trades is not a one-sided market, and must not divide by zero.
        assert_eq!(tape_pressure(Vec::new()), 50.0);
        assert_eq!(tape_pressure(vec![print(0.0, true)]), 50.0);
        assert_eq!(
            fmt_share(tape_pressure(demo_tape())),
            "34%",
            "the fixture tape"
        );
    }

    #[test]
    fn the_tape_stacks_newest_first_and_never_repeats_a_print() {
        let beat = |prints: Vec<Trade>| MarketTick {
            trades: prints,
            ..MarketTick::default()
        };
        let print = |tid: i64, price: f64| Trade {
            ts: tid,
            price,
            size: 1.0,
            buy: true,
            sweep: 1,
            tid,
        };

        // The feed hands them over oldest first; the panel reads down from now.
        let tape = push_trades(Vec::new(), beat(vec![print(1, 10.0), print(2, 11.0)]), 60);
        assert_eq!(tape[0].tid, 2, "newest on top");
        assert_eq!(tape[1].tid, 1);

        // A reconnect replays what we already have.
        let again = push_trades(tape.clone(), beat(vec![print(2, 11.0), print(3, 12.0)]), 60);
        assert_eq!(again.len(), 3, "only the print we had not seen is added");
        assert_eq!(again[0].tid, 3);

        let capped = push_trades(again, beat(vec![print(4, 13.0)]), 2);
        assert_eq!(capped.len(), 2, "the panel builds a bounded number of rows");
        assert_eq!(capped[0].tid, 4, "and drops the oldest, not the newest");
    }

    #[test]
    fn a_beat_repices_a_row_without_forgetting_what_the_asset_is() {
        let held = SymbolRow {
            name: "BTC".into(),
            price: 100.0,
            change_pct: 0.0,
            volume: 1.0,
            funding_pct: 0.0,
            leverage: 40.0,
            open_interest: 0.0,
            prev: 100.0,
            maintenance: 1.0 / 80.0,
            selected: false,
        };
        // `activeAssetCtx` restates the day's figures and not the asset's, so
        // it arrives with no maximum leverage and no maintenance rule. Taking
        // it wholesale would leave the ticket pricing against a zero.
        let context = parse_context(
            "BTC".into(),
            &json!({ "markPx": "110.0", "prevDayPx": "100.0", "dayNtlVlm": "9.0",
                     "funding": "0.0000125", "openInterest": "5.0" }),
        );
        assert_eq!(context.maintenance, 0.0, "the stream does not carry it");

        let beat = MarketTick {
            context: Some(context),
            mids: [("BTC".to_owned(), 120.0)].into_iter().collect(),
            ..MarketTick::default()
        };
        let rows = apply_feed(vec![held], beat);

        assert_eq!(rows[0].leverage, 40.0, "the asset's maximum survives");
        assert_eq!(rows[0].maintenance, 1.0 / 80.0, "and so does its rule");
        assert_eq!(rows[0].volume, 9.0, "the day's figures are the beat's");
        assert_eq!(rows[0].price, 120.0, "a mid is fresher than a context");
        assert_eq!(rows[0].change_pct, 20.0, "re-derived from the close");

        // A beat that names nothing leaves the row exactly as it was.
        let quiet = apply_feed(rows.clone(), MarketTick::default());
        assert!(quiet == rows, "a silent beat restates nothing");
    }

    #[test]
    fn the_ticket_prices_an_order_it_will_never_send() {
        let market = |max_leverage: f64| SymbolRow {
            name: "BTC".into(),
            price: 100.0,
            change_pct: 0.0,
            volume: 0.0,
            funding_pct: 0.0,
            leverage: max_leverage,
            maintenance: 1.0 / (2.0 * max_leverage),
            open_interest: 0.0,
            prev: 100.0,
            selected: false,
        };
        let quoted = |price: &str, size: &str, leverage: &str, buy: bool| {
            price_ticket(
                price.into(),
                size.into(),
                leverage.into(),
                Some(market(40.0)),
                buy,
                0.0,
            )
        };

        // A 10x long on 2 BTC at 100: 200 of notional on 20 of margin. The
        // maintenance requirement on a 40x market is 1/80th of the position,
        // so the cliff is where 20 + (X - 100)*2 meets X*2/80.
        let long = quoted("100", "2", "10", true);
        assert!(long.ready);
        assert_eq!(long.notional, 200.0);
        assert_eq!(long.margin, 20.0);
        assert!(
            (long.liquidation - 100.0 * 0.9 / (1.0 - 1.0 / 80.0)).abs() < 1e-9,
            "got {}",
            long.liquidation
        );
        assert!(long.liquidation < 100.0, "a long dies below its entry");

        let short = quoted("100", "2", "10", false);
        assert_eq!(short.margin, long.margin, "the side does not change margin");
        assert!(short.liquidation > 100.0, "a short dies above its entry");

        // More leverage moves the cliff toward the entry, which is the whole
        // reason the ticket shows it.
        assert!(quoted("100", "2", "20", true).liquidation > long.liquidation);
        assert!(
            quoted("100", "2", "1", true).liquidation == 0.0,
            "unlevered, there is no cliff to quote"
        );

        // Leverage past what the market allows is held at the ceiling rather
        // than quoting a liquidation the exchange would never have opened —
        // and the ticket reports the leverage it priced at, not the one that
        // was typed, so the figures and the number beside them agree.
        let over = quoted("100", "2", "400", true);
        assert_eq!(over.leverage, 40.0, "held at the market's maximum");
        assert_eq!(over.margin, quoted("100", "2", "40", true).margin);
        assert_eq!(over.liquidation, quoted("100", "2", "40", true).liquidation);
        assert_eq!(
            quoted("100", "2", "10", true).leverage,
            10.0,
            "and reports the typed one when it is allowed"
        );

        // Half-typed input is not an order, and must not read as one.
        for (price, size, leverage) in [("", "2", "10"), ("100", "", "10"), ("100", "2", "")] {
            let partial = quoted(price, size, leverage, true);
            assert!(!partial.ready, "{price:?}/{size:?}/{leverage:?} priced");
            assert_eq!(partial.margin, 0.0);
            assert_eq!(partial.liquidation, 0.0);
        }
        assert!(!quoted("not a price", "2", "10", true).ready);
        assert_eq!(
            quoted("1,250.5", "2", "10", true).notional,
            2_501.0,
            "a price read back off the screen carries its separators"
        );

        // A market the app has not loaded yet still prices what an order is
        // worth and what it ties up — that is multiplication — but it must
        // not quote a cliff. Treating an unknown requirement as zero puts the
        // liquidation further from the entry than it really is, which is the
        // one direction a risk number must never be wrong in.
        let unknown = price_ticket("100".into(), "2".into(), "10".into(), None, true, 0.0);
        assert!(unknown.ready, "the order is still describable");
        assert_eq!(unknown.notional, 200.0);
        assert_eq!(unknown.margin, 20.0);
        assert!(!unknown.known, "and the panel says it does not know");
        assert_eq!(unknown.liquidation, 0.0, "rather than an optimistic one");
        // What it would have said: 100 * (1 - 1/10) / (1 - 0) = 90, a cliff
        // ten percent away when the real one is nearer.
        assert!(
            quoted("100", "2", "10", true).liquidation > 90.0,
            "the real cliff sits closer to the entry than a zero requirement implies"
        );

        // The requirement is the venue's to publish, not this arithmetic's to
        // know. A market that maintains at twice the rate liquidates sooner,
        // and nothing here had to learn whose rule produced either number.
        let strict = SymbolRow {
            maintenance: 1.0 / 40.0,
            ..market(40.0)
        };
        let quoted_strict = price_ticket(
            "100".into(),
            "2".into(),
            "10".into(),
            Some(strict),
            true,
            0.0,
        );
        assert!(
            quoted_strict.liquidation > quoted("100", "2", "10", true).liquidation,
            "a heavier requirement moves the cliff toward the entry"
        );
    }

    #[test]
    fn an_alert_knows_which_way_the_market_has_to_go() {
        let beat = |price: f64| MarketTick {
            mids: [("BTC".to_owned(), price)].into_iter().collect(),
            ..MarketTick::default()
        };
        let set = |price: &str, mark: f64| add_alert(Vec::new(), "BTC".into(), price.into(), mark);

        // Nobody is asked which side to wait on: a level above the mark can
        // only be reached from below, and one below it from above.
        assert!(set("65000", 64_000.0)[0].above);
        assert!(!set("63000", 64_000.0)[0].above);

        // It fires when the market gets there, and not before.
        let above = set("65000", 64_000.0);
        assert!(!check_alerts(above.clone(), beat(64_900.0))[0].fired);
        assert!(
            check_alerts(above.clone(), beat(65_000.0))[0].fired,
            "at the level"
        );
        // And stays fired, so a price wobbling across it chimes once.
        let hit = check_alerts(above, beat(65_100.0));
        assert!(
            check_alerts(hit, beat(64_000.0))[0].fired,
            "firing is one-way"
        );

        // A market the beat says nothing about is left alone.
        let quiet = check_alerts(set("65000", 64_000.0), MarketTick::default());
        assert!(!quiet[0].fired);

        // A level at the mark has already happened; a duplicate is not a
        // second alert; nothing to price against is nothing to watch.
        assert!(set("64000", 64_000.0).is_empty(), "already there");
        assert_eq!(
            add_alert(
                set("65000", 64_000.0),
                "BTC".into(),
                "65000".into(),
                64_000.0
            )
            .len(),
            1
        );
        assert!(set("", 64_000.0).is_empty());
        assert!(set("65000", 0.0).is_empty(), "no mark to compare against");

        let watched = set("65000", 64_000.0);
        assert_eq!(waiting_alerts(watched.clone()), 1);
        assert_eq!(
            waiting_alerts(check_alerts(watched.clone(), beat(65_000.0))),
            0
        );
        assert_eq!(
            alert_label(watched[0].clone()),
            "BTC waiting above 65,000.00"
        );
        assert!(drop_alert(watched, "BTC".into(), 65_000.0).is_empty());
    }

    #[test]
    fn a_share_button_sizes_against_free_margin_not_equity() {
        let held = |withdrawable: f64| {
            Some(Account {
                value: 100_000.0,
                cross_value: 100_000.0,
                pnl: 0.0,
                withdrawable,
                notional: 0.0,
                maintenance: 0.0,
                health: 0.0,
                margin_pct: 0.0,
                positions: Vec::new(),
            })
        };
        let size = |share: f64| ticket_afford(held(10_000.0), "100".into(), 5.0, share);

        // 10,000 free at 5x is 50,000 of notional; a quarter of it at 100 is
        // 125. Equity is 100,000 and would say ten times that, which is the
        // point: equity already has positions standing on it.
        assert_eq!(size(0.25), "125.00");
        assert_eq!(size(1.0), "500.00");
        assert_eq!(size(2.0), "500.00", "past everything is everything");

        // Nothing to deploy, nothing to price against, nothing levered.
        assert_eq!(size(0.0), "");
        assert_eq!(ticket_afford(held(0.0), "100".into(), 5.0, 0.5), "");
        assert_eq!(ticket_afford(held(10_000.0), "".into(), 5.0, 0.5), "");
        assert_eq!(ticket_afford(held(10_000.0), "100".into(), 0.0, 0.5), "");
        assert_eq!(
            ticket_afford(None, "100".into(), 5.0, 0.5),
            "",
            "no account"
        );

        // The leverage handed in is the one the ticket was priced at, which is
        // the market's cap when the reader typed past it. Sizing at the typed
        // number would fill in an order the margin engine refuses, by exactly
        // the factor it overshot.
        let capped = price_ticket(
            "100".into(),
            "".into(),
            "40".into(),
            symbol_row(demo_symbols(), "kPEPE".into()),
            true,
            0.0,
        );
        assert_eq!(capped.leverage, 10.0, "the market caps it at ten");
        assert_eq!(
            ticket_afford(held(10_000.0), "100".into(), capped.leverage, 1.0),
            "1,000.00"
        );
    }

    #[test]
    fn the_ticket_says_whether_it_opens_or_closes() {
        let short = |size: f64| Position {
            coin: "BTC".into(),
            size,
            entry: 60_000.0,
            mark: 60_000.0,
            liq: 0.0,
            pnl: 0.0,
            roe_pct: 0.0,
            margin: 0.0,
            risk: 0.0,
            leverage: 20.0,
            margin_mode: "cross".into(),
            funding: 0.0,
        };
        let held = vec![short(-30.0)];
        let effect =
            |size: &str, buy: bool| ticket_effect(held.clone(), "BTC".into(), size.into(), buy);

        // A buy against a short is the interesting case: the same ticket that
        // opens a position on one side closes one on the other, and the only
        // thing that says which is a sign two panels apart.
        assert_eq!(effect("10", true), "Reduces your short to 20.00");
        assert_eq!(effect("30", true), "Closes your short");
        assert_eq!(effect("50", true), "Closes your short and opens 20.00 long");
        assert_eq!(effect("10", false), "Opens 10.00 short", "same side, adds");

        // A market you hold nothing in, and a market that is not this one.
        assert_eq!(
            ticket_effect(held.clone(), "ETH".into(), "1".into(), true),
            "Opens 1.00 long"
        );
        assert_eq!(
            ticket_effect(Vec::new(), "BTC".into(), "1".into(), true),
            "Opens 1.00 long"
        );
        // Nothing typed is not an order, and says nothing.
        assert_eq!(effect("", true), "");
        assert_eq!(effect("0", true), "");
    }

    #[test]
    fn a_closing_order_ties_up_nothing_and_has_no_cliff() {
        let quote = |size: &str, buy: bool, held: f64| {
            price_ticket(
                "100".into(),
                size.into(),
                "10".into(),
                Some(SymbolRow {
                    name: "BTC".into(),
                    price: 100.0,
                    change_pct: 0.0,
                    volume: 0.0,
                    funding_pct: 0.0,
                    leverage: 40.0,
                    open_interest: 0.0,
                    prev: 100.0,
                    maintenance: 1.0 / 80.0,
                    selected: false,
                }),
                buy,
                held,
            )
        };

        // Buying against a 30 short: the trade is worth its notional, but it
        // opens nothing, so it requires no margin and has no cliff. Quoting
        // one describes a position that would not exist.
        let closing = quote("30", true, -30.0);
        assert_eq!(closing.notional, 3_000.0, "the trade still has a value");
        assert_eq!(closing.margin, 0.0, "closing releases margin, not ties it");
        assert_eq!(closing.liquidation, 0.0, "and leaves nothing to liquidate");

        // A partial close is the same: still no new exposure.
        assert_eq!(quote("10", true, -30.0).margin, 0.0);

        // Past the position, only the excess opens.
        let flipped = quote("50", true, -30.0);
        assert_eq!(flipped.notional, 5_000.0, "all of it trades");
        assert_eq!(flipped.margin, 200.0, "20 of it opens, at 10x on 100");
        assert!(flipped.liquidation > 0.0, "and that part can be liquidated");

        // Adding to the same side opens all of it, as before.
        let adding = quote("30", false, -30.0);
        assert_eq!(adding.margin, 300.0);
        assert!(adding.liquidation > 0.0);
        assert_eq!(quote("30", true, 0.0).margin, 300.0, "nothing held");
    }

    #[test]
    fn closing_a_position_is_its_size_and_the_other_side() {
        let at = |coin: &str, size: f64| Position {
            coin: coin.into(),
            size,
            entry: 60_000.0,
            mark: 60_000.0,
            liq: 0.0,
            pnl: 0.0,
            roe_pct: 0.0,
            margin: 0.0,
            risk: 0.0,
            leverage: 20.0,
            margin_mode: "cross".into(),
            funding: 0.0,
        };
        let book = vec![at("BTC", -30.0), at("ETH", 5.0)];

        // Signed, because the ticket needs both the size to fill and the side
        // that closing takes — and they come from the same number.
        assert_eq!(position_held(book.clone(), "BTC".into()), -30.0);
        assert_eq!(position_held(book.clone(), "ETH".into()), 5.0);
        assert_eq!(position_held(book.clone(), "SOL".into()), 0.0, "none held");
        assert_eq!(position_held(Vec::new(), "BTC".into()), 0.0);

        // What the panel then fills: an unsigned size, and the opposite side.
        let short = position_held(book.clone(), "BTC".into());
        assert_eq!(fmt_size(short), "30.00", "the field takes no sign");
        assert!(short < 0.0, "so closing a short buys");
        // And the effect line agrees with what the button just did.
        assert_eq!(
            ticket_effect(book, "BTC".into(), fmt_size(short), short < 0.0),
            "Closes your short"
        );
    }

    #[test]
    fn the_ticket_opens_on_a_price_you_are_already_looking_at() {
        let book = Book {
            bids: Vec::new(),
            asks: Vec::new(),
            spread: 0.0,
            spread_pct: 0.0,
            mid: 64_849.0,
        };
        let row = SymbolRow {
            name: "BTC".into(),
            price: 64_500.0,
            change_pct: 0.0,
            volume: 0.0,
            funding_pct: 0.0,
            leverage: 40.0,
            maintenance: 1.0 / (2.0 * 40.0),
            open_interest: 0.0,
            prev: 0.0,
            selected: false,
        };
        assert_eq!(
            ticket_seed(Some(book.clone()), Some(row.clone())),
            "64,849.00",
            "the book is the freshest price on screen"
        );
        assert_eq!(
            ticket_seed(None, Some(row.clone())),
            "64,500.00",
            "without a book, the market's last"
        );
        assert_eq!(
            ticket_seed(Some(Book { mid: 0.0, ..book }), Some(row)),
            "64,500.00",
            "an empty book is not a price of zero"
        );
        assert_eq!(ticket_seed(None, None), "", "nothing to seed it with");
    }

    #[test]
    fn the_lower_panel_stops_at_its_limits_instead_of_at_the_gesture() {
        assert_eq!(pane_height(232.0), 232.0, "an ordinary drag is itself");
        assert_eq!(pane_height(LOWER_MIN), LOWER_MIN);
        assert_eq!(pane_height(LOWER_MAX), LOWER_MAX);
        // The reason this clamps rather than refusing: a drag that overshoots
        // has to stop at the limit. Rejecting the whole delta stops the panel
        // moving at all, so a fast gesture reads as a stuck divider.
        assert_eq!(pane_height(20.0), LOWER_MIN, "past the floor, pinned to it");
        assert_eq!(pane_height(9_000.0), LOWER_MAX, "and past the ceiling");
        assert_eq!(pane_height(-400.0), LOWER_MIN, "a drag through zero");
        assert_eq!(pane_height(f64::NAN), LOWER_MIN, "never a NaN-tall panel");
        assert_eq!(pane_height(f64::INFINITY), LOWER_MAX);
    }

    #[test]
    fn the_risk_rail_measures_entry_to_liquidation() {
        // A long: entry above the cliff, so travel grows as the mark falls.
        assert_eq!(liquidation_travel(100.0, 100.0, 80.0), 0.0, "at entry");
        assert_eq!(liquidation_travel(100.0, 90.0, 80.0), 0.5, "halfway down");
        assert_eq!(liquidation_travel(100.0, 80.0, 80.0), 1.0, "at the cliff");
        assert_eq!(
            liquidation_travel(100.0, 110.0, 80.0),
            0.0,
            "in profit, clamped"
        );
        assert_eq!(
            liquidation_travel(100.0, 70.0, 80.0),
            1.0,
            "past it, clamped"
        );

        // A short flips both endpoints, so the same ratio holds.
        assert_eq!(liquidation_travel(100.0, 110.0, 120.0), 0.5);

        // Cross positions report no liquidation price at all.
        assert_eq!(liquidation_travel(100.0, 90.0, 0.0), 0.0);
        assert_eq!(
            liquidation_travel(100.0, 90.0, 100.0),
            0.0,
            "no span to travel"
        );
    }

    #[test]
    fn the_book_stacks_depth_and_measures_the_spread() {
        let book = parse_book(&json!({
            "coin": "BTC",
            "levels": [
                [{ "px": "64848.0", "sz": "1.0", "n": 1 }, { "px": "64847.0", "sz": "3.0", "n": 2 }],
                [{ "px": "64850.0", "sz": "2.0", "n": 1 }, { "px": "64851.0", "sz": "2.0", "n": 1 }],
            ]
        }));

        assert_eq!(book.bids[0].total, 1.0, "depth accumulates from the top");
        assert_eq!(book.bids[1].total, 4.0);
        assert_eq!(
            book.bids[1].bar, BOOK_BAR_WIDTH,
            "the deepest level is a full bar"
        );
        assert!(book.bids[0].bar < book.bids[1].bar);
        assert_eq!(book.spread, 2.0, "64850 ask against a 64848 bid");
        assert_eq!(book.mid, 64_849.0);
        assert!((book.spread_pct - 2.0 / 64_849.0 * 100.0).abs() < 1e-12);

        // An empty side must not divide by its own zero depth.
        let empty = parse_book(&json!({ "levels": [[], []] }));
        assert_eq!(empty.spread, 0.0);
        assert_eq!(empty.mid, 0.0);
    }

    #[test]
    fn resting_orders_carry_a_side_and_price() {
        let orders = parse_orders(&json!([
            { "coin": "HYPE", "side": "A", "limitPx": "56.473", "sz": "23.24", "timestamp": 1_786_096_201_409i64 },
            { "coin": "BTC", "side": "B", "limitPx": "60000.0", "sz": "0.5", "timestamp": 1_786_096_201_409i64 },
        ]));
        assert!(!orders[0].buy, "an ask is a sell order");
        assert!(orders[1].buy);
        assert_eq!(orders[0].ts, 1_786_096_201, "seconds, like the chart");

        let lines = order_lines(&orders, "BTC", &chart_theme().palette);
        assert_eq!(lines.len(), 1, "only the charted market is drawn");
        assert_eq!(lines[0].price, 60_000.0);
    }

    fn fill(ts: i64, tid: i64, heat: i64) -> Fill {
        Fill {
            coin: "BTC".into(),
            ts,
            price: 1.0,
            size: 1.0,
            buy: true,
            closed_pnl: 0.0,
            heat,
            tid,
        }
    }

    #[test]
    fn pushed_fills_stack_newest_first_without_repeating_the_snapshot() {
        let snapshot = push_fills(Vec::new(), vec![fill(10, 1, 0), fill(30, 3, 0)], 4);
        assert_eq!(snapshot[0].ts, 30, "newest first");

        // The feed re-sends a fill the snapshot already showed alongside a
        // new one; only the new one lands, and only it is lit.
        let pushed = push_fills(
            snapshot,
            vec![fill(30, 3, FLASH_STEPS), fill(40, 4, FLASH_STEPS)],
            4,
        );
        assert_eq!(pushed.len(), 3, "the repeat is dropped");
        assert_eq!(pushed[0].ts, 40);
        assert_eq!(pushed[0].heat, FLASH_STEPS);
        assert_eq!(pushed[1].heat, 0, "the fill it already held stays cold");
        assert!(any_hot(pushed.clone()), "the list is flashing");

        let capped = push_fills(pushed, vec![fill(50, 5, FLASH_STEPS)], 2);
        assert_eq!(capped.len(), 2, "the list is capped");
        assert_eq!(capped[0].ts, 50);

        // Two beats of decay put the highlight out.
        let cooled = cool_fills(cool_fills(capped));
        assert!(!any_hot(cooled), "nothing stays lit");
    }

    #[test]
    fn formatting_shows_direction_and_instrument_precision() {
        assert_eq!(fmt_px(64_581.0), "64,581.00");
        assert_eq!(fmt_px(0.0004321), "0.000432");
        assert_eq!(fmt_signed_usd(-1_234.5), "-$1,234.50");
        assert_eq!(fmt_signed_usd(12.0), "+$12.00");
        assert_eq!(fmt_pct(-3.456), "-3.46%");
        assert_eq!(
            fmt_compact_usd(0.0),
            "$0",
            "a zero is neither a gain nor a loss"
        );
        assert_eq!(fmt_compact_usd(-3_309_352.0), "-$3.3M");
        assert_eq!(fmt_pct(3.4), "+3.40%");

        // A spread reads against the price it sits on: $2 on a $64,849 mid is
        // the tightest market on the exchange, and the same $2 on a $3 coin is
        // not a market at all. Both are "2.00" until you divide.
        assert_eq!(fmt_bps(2.0 / 64_849.0 * 100.0), "0.3 bps");
        assert_eq!(fmt_bps(2.0 / 3.0 * 100.0), "6666.7 bps");
        // A book with one side empty has no spread to quote.
        assert_eq!(fmt_bps(0.0), "—");
        assert_eq!(fmt_bps(f64::NAN), "—", "never render a NaN into the panel");

        // The feed reads its own round trip, and zero is not a fast socket —
        // it is a socket that has not answered a ping yet, or one that just
        // dropped. The header has to say so rather than hold the last good
        // number and look live.
        assert_eq!(fmt_latency(42), "42ms");
        assert_eq!(fmt_latency(0), "—", "unmeasured is not instant");
        assert_eq!(fmt_latency(-1), "—");

        // Order age reads in the coarsest unit that still says something.
        let now = now_ms() / 1_000;
        assert_eq!(fmt_age(now), "now");
        assert_eq!(fmt_age(now - 59), "now");
        assert_eq!(fmt_age(now - 60), "1m");
        assert_eq!(fmt_age(now - 3_599), "59m");
        assert_eq!(fmt_age(now - 3_600), "1h");
        assert_eq!(fmt_age(now - 86_400 * 4), "4d");
        // An order the exchange gave no timestamp, and one stamped in the
        // future by a clock that disagrees with ours, both read as unknown
        // rather than as "now" or as a negative age.
        assert_eq!(fmt_age(0), "—");
        assert_eq!(fmt_age(now + 600), "—");

        // Hourly funding is a hundredth of a percent on most of the exchange,
        // so the two decimals every other figure here uses would render 166 of
        // the 177 funded markets as the same "+0.00%".
        assert_eq!(fmt_pct(0.00125), "+0.00%", "what two decimals can say");
        assert_eq!(fmt_funding(0.00125), "+0.0013%");
        assert_eq!(fmt_funding(-0.232341), "-0.2323%", "the widest real rate");
        assert_eq!(fmt_funding(0.0), "+0.0000%");
    }

    /// The fixtures above encode what the exchange documents. This one asks
    /// the exchange. It talks to the network, so it stays opt-in:
    /// `cargo test -p trading-example -- --ignored`.
    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn live_api_matches_the_shapes_parsed_here() {
        smol::block_on(async {
            let symbols = hl_symbols().await.expect("symbol list");
            assert!(symbols.len() > 20, "got {} markets", symbols.len());
            let btc = symbols.iter().find(|row| row.name == "BTC").expect("BTC");
            assert!(btc.price > 0.0 && btc.volume > 0.0, "BTC context is empty");

            // A fresh tape adopts the first market loaded into it.
            let tape = tape_new();
            let bars = hl_candles(tape.clone(), "BTC".into(), "1m".into())
                .await
                .expect("candle backfill");
            assert!(bars > 100, "expected a backfill, got {bars} candles");
            assert_eq!(*lock(&tape.focus), "BTC:1m");
            {
                let candles = lock(&tape.candles);
                assert!(
                    candles.windows(2).all(|pair| pair[0].ts < pair[1].ts),
                    "candles must be sorted for the chart"
                );
                assert!(
                    candles
                        .iter()
                        .all(|candle| candle.high >= candle.low && candle.close > 0.0)
                );
            }

            // A vault address: reachable, and its summary parses.
            let account = hl_account("0xdfc24b077bc1425ad1dea75bcb6f8158e10df303".into())
                .await
                .expect("clearinghouse state");
            assert!(account.value > 0.0, "the HLP vault holds a balance");

            // The exchange reports a mark, a PnL, and a return; the feed only
            // sends a price. Hand its own marks back to the arithmetic that
            // turns one into the others, and it has to land where the
            // exchange did — position by position, on a live book.
            let watched = hl_account(WATCHED.to_owned())
                .await
                .expect("clearinghouse state");
            assert!(
                !watched.positions.is_empty(),
                "the address this app opens on holds nothing to re-value"
            );
            // A live leveraged account owes a maintenance requirement, and it
            // is under its equity or the exchange would have closed it.
            assert!(watched.maintenance > 0.0, "open positions, no requirement");
            assert!(
                watched.health > 0.0 && watched.health < RISK_RAIL_WIDTH,
                "a solvent account draws a partial rail, got {}",
                watched.health
            );
            let marked = mark_positions(
                watched.positions.clone(),
                MarketTick {
                    mids: watched
                        .positions
                        .iter()
                        .map(|position| (position.coin.clone(), position.mark))
                        .collect(),
                    ..MarketTick::default()
                },
            );
            // Feeding the exchange's own marks back in has to leave every
            // figure exactly where the exchange put it: a valuation that
            // moves on unchanged prices is a valuation that disagrees with
            // the account it is describing.
            for (reported, valued) in watched.positions.iter().zip(&marked) {
                let coin = &reported.coin;
                assert!(
                    (valued.pnl - reported.pnl).abs() <= reported.pnl.abs() * 1e-9,
                    "{coin}: valued {} against a reported {}",
                    valued.pnl,
                    reported.pnl
                );
                assert!(
                    (valued.roe_pct - reported.roe_pct).abs() <= reported.roe_pct.abs() * 1e-9,
                    "{coin}: returned {}% against a reported {}%",
                    valued.roe_pct,
                    reported.roe_pct
                );
            }
            let revalued = mark_account(Some(watched.clone()), marked).expect("an account");
            assert!(
                (revalued.value - watched.value).abs() <= watched.value.abs() * 1e-9,
                "the same prices cannot move the equity"
            );
            assert!(
                (revalued.notional - watched.notional).abs() <= watched.notional.abs() * 1e-6,
                "nor what the book says the positions are worth"
            );
        });
    }

    /// The other half of the boundary: the websocket. Also opt-in, and also
    /// the only place the subscription names, the payload shapes, and the
    /// ping round trip are checked against the exchange rather than against
    /// a recording.
    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn the_live_feed_fills_the_tape_and_the_book() {
        let tape = tape_focus(tape_new(), "BTC".into(), "1m".into());
        let feed = hl_market_feed(tape.clone());
        let deadline = Instant::now() + Duration::from_secs(45);
        let mut ticks = Vec::new();

        smol::block_on(async {
            // Long enough for a book, a context, and one ping round trip.
            while Instant::now() < deadline && ticks.len() < 60 {
                match feed.recv().await {
                    Ok(Ok(tick)) => ticks.push(tick),
                    Ok(Err(error)) => panic!("feed reported {}", error.message),
                    Err(_) => break,
                }
            }
        });

        assert!(!ticks.is_empty(), "the feed said nothing in 45s");
        let book = ticks
            .iter()
            .rev()
            .find_map(|tick| tick.book.clone())
            .expect("a book arrived");
        assert!(!book.bids.is_empty() && !book.asks.is_empty());
        assert!(book.spread > 0.0, "a live book has a spread");
        assert!(
            book.asks
                .last()
                .is_some_and(|best| best.price > book.bids[0].price),
            "the asks are reversed so the best ask sits against the spread"
        );
        assert!(
            ticks.iter().any(|tick| tick.mids.contains_key("BTC")),
            "allMids carries the market this app lists"
        );
        assert!(
            ticks.iter().any(|tick| tick
                .context
                .as_ref()
                .is_some_and(|context| context.name == "BTC" && context.prev > 0.0)),
            "activeAssetCtx carries the header's figures"
        );
        assert!(
            ticks.iter().any(|tick| tick.latency > 0),
            "the ping round trip is the latency readout"
        );

        // The tape. Bitcoin prints many times a minute, so silence here is a
        // subscription that never took rather than a quiet market.
        let prints: Vec<Trade> = ticks
            .iter()
            .flat_map(|tick| tick.trades.iter().cloned())
            .collect();
        assert!(!prints.is_empty(), "no trade arrived in 45s on BTC");
        assert!(
            prints
                .iter()
                .all(|print| print.price > 0.0 && print.size > 0.0),
            "a print with no price or no size is a parse that missed"
        );
        assert!(
            prints.iter().all(|print| print.sweep >= 1),
            "every row stands for at least one resting order"
        );
        // Both sides trade in any 45s window of a liquid market. All-one-side
        // is how a reversed or ignored `side` field would look.
        assert!(
            prints.iter().any(|print| print.buy) && prints.iter().any(|print| !print.buy),
            "every print read as the same side: {} of {} were buys",
            prints.iter().filter(|print| print.buy).count(),
            prints.len()
        );
        // Prints cross the spread, so they land on the book rather than
        // somewhere else entirely. A wide band, because the book moved while
        // these were arriving; it is checking the order of magnitude.
        let mid = book.mid;
        assert!(
            prints
                .iter()
                .all(|print| (print.price - mid).abs() < mid * 0.05),
            "a print landed 5% off the book, so the tape is not this market"
        );

        let folded = push_trades(Vec::new(), ticks[0].clone(), 60);
        assert!(
            folded.len() <= 60 && folded.windows(2).all(|pair| pair[0].ts >= pair[1].ts),
            "the panel reads down from now"
        );
        assert!(
            !lock(&tape.candles).is_empty(),
            "candles are merged into the tape rather than sent through Ice"
        );
    }

    #[test]
    fn an_address_is_checked_before_it_reaches_the_exchange() {
        // The address the app opens on, and the vault the live test reads.
        assert!(valid_address(WATCHED.to_owned()));
        assert!(valid_address(
            "  0xdfc24b077bc1425ad1dea75bcb6f8158e10df303  ".to_owned()
        ));
        assert!(
            valid_address("0xDFC24B077BC1425AD1DEA75BCB6F8158E10DF303".to_owned()),
            "checksummed addresses are the same address"
        );

        assert!(!valid_address(String::new()), "the prompt starts empty");
        assert!(!valid_address("0x".to_owned()));
        assert!(
            !valid_address("dfc24b077bc1425ad1dea75bcb6f8158e10df303".to_owned()),
            "forty digits without the prefix is not an address"
        );
        assert!(
            !valid_address("0xdfc24b077bc1425ad1dea75bcb6f8158e10df3033".to_owned()),
            "one digit too many"
        );
        assert!(
            !valid_address("0xzzc24b077bc1425ad1dea75bcb6f8158e10df303".to_owned()),
            "the right length, but not hexadecimal"
        );
        // Forty-two bytes of multibyte text must not be sliced mid-character.
        assert!(!valid_address("0x".to_owned() + &"é".repeat(20)));
    }

    #[test]
    fn search_matches_tickers_case_insensitively() {
        let rows = vec![
            SymbolRow {
                name: "BTC".into(),
                price: 1.0,
                change_pct: 0.0,
                volume: 0.0,
                funding_pct: 0.0,
                leverage: 40.0,
                maintenance: 1.0 / (2.0 * 40.0),
                open_interest: 0.0,
                prev: 1.0,
                selected: false,
            },
            SymbolRow {
                name: "ETH".into(),
                price: 1.0,
                change_pct: 0.0,
                volume: 0.0,
                funding_pct: 0.0,
                leverage: 25.0,
                maintenance: 1.0 / (2.0 * 25.0),
                open_interest: 0.0,
                prev: 1.0,
                selected: false,
            },
        ];
        assert_eq!(
            filter_symbols(rows.clone(), " et ".into(), "BTC".into()).len(),
            1
        );
        assert_eq!(
            filter_symbols(rows.clone(), "".into(), "BTC".into()).len(),
            2
        );
        assert_eq!(
            filter_symbols(rows.clone(), "doge".into(), "BTC".into()).len(),
            0
        );

        // The mark rides on the row, so the view can cache a row without also
        // reading the selected coin beside it.
        let marked = filter_symbols(rows, "".into(), "ETH".into());
        assert_eq!(
            marked
                .iter()
                .filter(|row| row.selected)
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            ["ETH"]
        );
    }
}
