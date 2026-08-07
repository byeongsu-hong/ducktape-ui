//! The Hyperliquid API behind two shapes: the `info` endpoint as one blocking
//! POST moved off the UI thread with `smol::unblock`, and the websocket as a
//! thread per feed pushing into a channel Ice reads as a stream. Every
//! response is read as a `Value` and mapped by hand: the exchange sends all
//! numbers as JSON strings, so a derive would need a custom deserializer per
//! field anyway.

use std::collections::{HashMap, HashSet};
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
#[derive(Clone, PartialEq)]
pub struct SymbolRow {
    pub name: String,
    pub price: f64,
    pub change_pct: f64,
    pub volume: f64,
    pub funding_pct: f64,
    pub leverage: f64,
    pub open_interest: f64,
    pub oracle: f64,
    /// Yesterday's close, kept so a streamed mid price can be turned back
    /// into a 24h change without another request.
    pub prev: f64,
}

/// One open position, shaped the way the official app reads it out.
#[derive(Clone, PartialEq)]
pub struct Position {
    pub coin: String,
    pub side: String,
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
    pub pnl: f64,
    pub margin_used: f64,
    pub withdrawable: f64,
    pub notional: f64,
    pub maintenance: f64,
    /// How far the account's equity has fallen toward its maintenance
    /// requirement, already scaled to the rail's pixel width.
    pub health: f64,
    pub positions: Vec<Position>,
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
    pub action: String,
    pub fee: f64,
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
            leverage: asset
                .get("maxLeverage")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
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
        leverage: 0.0,
        open_interest: num(context, "openInterest"),
        oracle: num(context, "oraclePx"),
        prev: previous,
    }
}

pub async fn hl_symbols() -> Result<Vec<SymbolRow>, HlError> {
    Ok(parse_symbols(
        &info(json!({ "type": "metaAndAssetCtxs" })).await?,
    ))
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
                side: if size >= 0.0 { "Long" } else { "Short" }.to_owned(),
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
    let maintenance = num(value, "crossMaintenanceMarginUsed");
    Account {
        value: equity,
        pnl: positions.iter().map(|position| position.pnl).sum(),
        margin_used: num(&summary, "totalMarginUsed"),
        withdrawable: num(value, "withdrawable"),
        notional: num(&summary, "totalNtlPos"),
        maintenance,
        health: margin_load(equity, maintenance) * RISK_RAIL_WIDTH,
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
            action: text(fill, "dir"),
            fee: num(fill, "fee"),
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
    feed(
        move || {
            let mut wanted = vec![json!({ "type": "allMids" })];
            if let Some((coin, interval)) = subscriptions.focus() {
                wanted.push(json!({ "type": "l2Book", "coin": coin }));
                wanted.push(json!({ "type": "activeAssetCtx", "coin": coin }));
                wanted.push(json!({ "type": "candle", "coin": coin, "interval": interval }));
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
                    mids: std::mem::take(&mut tick.mids),
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
            leverage: row.leverage,
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
    let value = account.value - account.pnl + pnl;
    Some(Account {
        value,
        pnl,
        notional: positions
            .iter()
            .map(|position| position.mark * position.size.abs())
            .sum(),
        // The requirement itself waits for the poll, but the equity it is
        // measured against does not, so the rail closes as the account loses.
        health: margin_load(value, account.maintenance) * RISK_RAIL_WIDTH,
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
pub fn filter_symbols(rows: Vec<SymbolRow>, query: String) -> Vec<SymbolRow> {
    let query = query.trim().to_uppercase();
    if query.is_empty() {
        return rows;
    }
    rows.into_iter()
        .filter(|row| row.name.to_uppercase().contains(&query))
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

/// Left gap the header keeps clear so its content never sits under the macOS
/// traffic lights, which float over the fullsize content view. The rightmost
/// button ends near 74pt; everywhere else the header owns its full width.
pub fn header_inset() -> f64 {
    if cfg!(target_os = "macos") { 78.0 } else { 0.0 }
}

/// This account's fills on one market, as chart glyphs: a buy points up out
/// of its price, a sell points down into it.
fn fill_markers(fills: &[Fill], coin: &str) -> Vec<ChartMarker> {
    let palette = chart_theme().palette;
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
fn position_lines(positions: &[Position], coin: &str) -> Vec<PriceLine> {
    let palette = chart_theme().palette;
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
fn order_lines(orders: &[Order], coin: &str) -> Vec<PriceLine> {
    let palette = chart_theme().palette;
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
    let chart = candle_chart_shared(tape.candles.clone(), &chart_theme())
        .height(Length::Fill)
        .live(BEAT)
        .moving_averages([20, 60])
        .price_lines(position_lines(positions, coin))
        .price_lines(order_lines(orders, coin))
        .markers(fill_markers(fills, coin))
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

        let long = &account.positions[0];
        assert_eq!(long.side, "Long");
        assert_eq!(long.mark, 64_000.0, "mark comes from notional over size");
        assert!((long.roe_pct - 25.0).abs() < 1e-9);

        let short = &account.positions[1];
        assert_eq!(short.side, "Short");
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
                action: "Open Long".into(),
                fee: 0.0,
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
                action: "Open Long".into(),
                fee: 0.0,
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
                action: "Close Long".into(),
                fee: 1.0,
                heat: 0,
                tid: 3,
            },
        ];

        let markers = fill_markers(&fills, "BTC");
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
                side: "Long".into(),
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
                side: "Short".into(),
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
        let lines = position_lines(&positions, "BTC");
        assert_eq!(lines.len(), 2, "entry and liquidation");
        assert_eq!(lines[0].price, 60_000.0);
        assert_eq!(lines[1].price, 45_000.0);
        assert_eq!(
            position_lines(&positions, "ETH").len(),
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

        let lines = order_lines(&orders, "BTC");
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
            action: String::new(),
            fee: 0.0,
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
        assert!(
            !lock(&tape.candles).is_empty(),
            "candles are merged into the tape rather than sent through Ice"
        );
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
                open_interest: 0.0,
                oracle: 1.0,
                prev: 1.0,
            },
            SymbolRow {
                name: "ETH".into(),
                price: 1.0,
                change_pct: 0.0,
                volume: 0.0,
                funding_pct: 0.0,
                leverage: 25.0,
                open_interest: 0.0,
                oracle: 1.0,
                prev: 1.0,
            },
        ];
        assert_eq!(filter_symbols(rows.clone(), " et ".into()).len(), 1);
        assert_eq!(filter_symbols(rows.clone(), "".into()).len(), 2);
        assert_eq!(filter_symbols(rows, "doge".into()).len(), 0);
    }
}
