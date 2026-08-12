//! The Hyperliquid API behind two shapes: the `info` endpoint as one blocking
//! POST moved off the UI thread with `smol::unblock`, and the websocket as a
//! thread per feed pushing into a channel Ice reads as a stream. Every
//! response is read as a `Value` and mapped by hand: the exchange sends all
//! numbers as JSON strings, so a derive would need a custom deserializer per
//! field anyway.

use std::cmp::Ordering;
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

use crate::Venue;
use crate::signing::{self, Action, Chain, Wallet};
use crate::venue::venue_name;

pub use ducktape_ui::ui::candle_chart::{Candle, CandleHit};

const TIMEOUT: Duration = Duration::from_secs(15);
/// Items waiting between a websocket thread and Ice.
///
/// The handoff is lossless: a full buffer stops the socket thread instead of
/// dropping a tick or fill delta. That can delay reads and heartbeats while
/// the UI is stalled; if the venue closes the quiet socket, the existing retry
/// loop reconnects after the UI drains the backlog. Dropping the receiver
/// wakes a blocked send and ends the thread.
pub(crate) const FEED_BUFFER_CAPACITY: usize = 16;

pub(crate) fn feed_channel<T>() -> (Sender<T>, Receiver<T>) {
    smol::channel::bounded(FEED_BUFFER_CAPACITY)
}

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
/// Book levels shown per side, and the pixel width of a full depth bar.
const BOOK_DEPTH: usize = 10;
const BOOK_BAR_WIDTH: f64 = 196.0;
/// What the lower panel can be dragged between: enough for a position row,
/// and never so tall that the chart is gone.
const LOWER_MIN: f64 = 120.0;
const LOWER_MAX: f64 = 560.0;
/// Pixel width of the risk rail drawn under a position's liquidation price.
const RISK_RAIL_WIDTH: f64 = 80.0;
/// The finest step any Hyperliquid market quotes a size to. A size is never
/// rounded past this, because below it the digits are the exchange's own; and
/// never quoted beyond it, because past it there is nothing but the noise of
/// subtracting two of them.
const SIZE_DECIMALS: usize = 8;

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

/// Whether this build may reach the exchange.
///
/// Closed under test, and opened only by the tests whose whole subject is the
/// live API. A test drives the real program, subscriptions included, so the
/// five-second account poll fires inside whichever test the suite's own load
/// stretches past five seconds, and that test then asserts against whatever
/// the exchange happened to be holding. The tests own their fixtures; the wire
/// is not one of them.
///
/// The opt-in is one flag for the whole process, so it is only ever right in a
/// run whose tests are all live ones. `--ignored` is that run: it selects the
/// ignored tests and nothing else, so nothing that is not about the exchange
/// can be reading this while they flip it. `--include-ignored` is not — it puts
/// the ordinary tests in the same process, where one flip hands them the
/// exchange for the rest of the run. So `open_the_wire` refuses under that flag
/// rather than the comment claiming a boundary the flag walks through.
#[cfg(test)]
static WIRE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
pub(crate) fn wire_is_open() -> bool {
    WIRE.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(not(test))]
pub(crate) fn wire_is_open() -> bool {
    true
}

/// Let this test reach the exchange. Only a test whose subject is the live API
/// may call it, and it says so by calling it.
#[cfg(test)]
pub(crate) fn open_the_wire() {
    assert!(
        only_the_live_tests_are_running(std::env::args()),
        "the wire is one flag for the whole process: run the live tests with \
         --ignored, which runs them and nothing else, rather than with \
         --include-ignored, which would leave this open under every ordinary \
         test that outlives the flip"
    );
    WIRE.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether this process was started to run the ignored tests and nothing else,
/// which is the only run a process-wide wire is sound in. Takes the arguments
/// rather than reading them, so the rule can be held against a run that is not
/// the one the test itself is in.
#[cfg(test)]
fn only_the_live_tests_are_running(mut args: impl Iterator<Item = String>) -> bool {
    !args.any(|arg| arg == "--include-ignored")
}

/// Everything the exchange can tell us goes through this one endpoint, on the
/// deployment the caller names.
///
/// The chain is a parameter rather than a constant because the app reads two
/// Hyperliquid deployments and one of them is the one where an order costs
/// nothing to get wrong. A default here would be a network chosen by whoever
/// forgot to pass one.
pub(crate) async fn info(chain: Chain, body: Value) -> Result<Value, HlError> {
    smol::unblock(move || info_blocking(chain, &body)).await
}

/// The same request without the executor around it, for the two callers that
/// are already off the UI thread: the batch that reads the whole universe on
/// one pool of threads, and the feed thread deciding what to subscribe to.
pub(crate) fn info_blocking(chain: Chain, body: &Value) -> Result<Value, HlError> {
    // A test drives the real program, subscriptions included, so the 5s account
    // poll fires inside any test the suite's own load stretches past five
    // seconds and this endpoint answers it from the live exchange. Whichever
    // test that lands in then asserts against whatever the exchange happened to
    // hold. The tests own their fixtures; the wire is not one of them.
    //
    // Held here rather than on the async wrapper because this is what every
    // caller reaches the wire through. Guarding only the wrapper left the
    // universe batch and the feed's dex read going out to the live exchange
    // from inside the suite, and a test asserting on an empty market list then
    // failed against whatever Hyperliquid happened to be listing.
    if !wire_is_open() {
        return Err(HlError::new(
            "Hyperliquid unreachable: no wire under test".to_owned(),
        ));
    }
    let mut response = agent()
        .post(chain.info_url())
        .send_json(body)
        .map_err(|error| HlError::new(format!("Hyperliquid unreachable: {error}")))?;
    response
        .body_mut()
        .read_json::<Value>()
        .map_err(|error| HlError::new(format!("Hyperliquid sent bad JSON: {error}")))
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
    pub(crate) fn new(message: String) -> Self {
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
///
/// `Default` is derived so a fixture states the figures its assertion is about
/// and nothing else. The parsers do not use it: a market read off the wire
/// states every field, because a field silently left at zero there is a price
/// or a margin requirement the screen would show as real.
#[derive(Clone, Default, PartialEq)]
pub struct SymbolRow {
    pub name: String,
    /// Which list this market belongs to, as the rail heads it and a reader
    /// hears it: the venue's own perps, or the name of the builder that
    /// deployed the market. Empty when the venue lists one flat universe,
    /// which is the whole of what says whether this list is grouped at all.
    pub category: String,
    /// The token this market's margin is posted and settled in. Canonical
    /// Hyperliquid is USDC; a builder dex names its own, and several of the
    /// live ones are not USDC. The ticket quotes a margin requirement, and a
    /// requirement without its unit is a number pretending to be dollars.
    pub collateral: String,
    /// Whether this row opens its category in the list as filtered. Not a
    /// property of the market — the same market is a heading or not depending
    /// on what the search left above it — so it is written by the filter that
    /// decides the order, and read by the one `if` that draws a header.
    pub heading: bool,
    pub price: f64,
    pub change_pct: f64,
    pub volume: f64,
    pub funding_pct: f64,
    /// When this venue last charged funding on this market, in epoch seconds,
    /// or zero when the venue does not say.
    ///
    /// A rate says what funding costs and never when it lands, which is the
    /// half a holder of a position actually has to plan around. This is the
    /// anchor the countdown is measured from, and it is the venue's own stamp
    /// rather than a boundary this app assumed: see `funding_countdown` in
    /// `venue.rs` for which network states one and which is derived from the
    /// clock.
    pub funding_at: i64,
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
    /// This market's index in its own `meta.universe`, which is what an order
    /// carries on the wire — the name never reaches the exchange.
    ///
    /// Read from the universe *before* the volume sort below, because the sort
    /// is the app's presentation order and the index is the exchange's
    /// identity. Taking it from the sorted position would name a different
    /// market on every poll, which is an order for whatever happened to be as
    /// busy as the one you meant.
    ///
    /// Canonical markets only. A builder dex numbers its own universe and the
    /// wire offsets it, and this app does not place orders there anyway: those
    /// markets are margined against a clearinghouse it cannot read, and
    /// `own_clearinghouse` already declines them on the ticket. The order path
    /// refuses them again rather than trusting that.
    pub asset: u32,
    /// How finely this market quotes a size. It is the instrument's, not the
    /// size's: the venue accepts the same step whether you trade a thousandth
    /// of a coin or a thousand of them, so a size the app works out for itself
    /// is rounded to this rather than to how large it came out.
    pub size_decimals: usize,
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
        self.asset.hash(state);
        self.category.hash(state);
        self.collateral.hash(state);
        self.heading.hash(state);
        self.selected.hash(state);
        self.size_decimals.hash(state);
        self.funding_at.hash(state);
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
///
/// `Hash` is hand-written for the same reason `Fill`'s is: the tape is a
/// `lazy` boundary keyed on the print it draws, and the money on a print is
/// `f64`. A row's cache is invalidated by any change to that print, which is
/// exactly what the row renders.
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
    pub(crate) tid: i64,
}

impl Hash for Trade {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ts.hash(state);
        self.buy.hash(state);
        self.sweep.hash(state);
        self.tid.hash(state);
        for value in [self.price, self.size] {
            hash_f64(value, state);
        }
    }
}

/// One executed trade, which is what the chart marks.
///
/// `Hash` is hand-written for the same reason `SymbolRow`'s is: the fills list
/// is a `lazy` boundary keyed on the fill it draws, and the money on a fill is
/// `f64`. A row's cache is invalidated by any change to that fill, which is
/// exactly what the row renders.
#[derive(Clone, PartialEq)]
pub struct Fill {
    pub coin: String,
    pub ts: i64,
    pub price: f64,
    pub size: f64,
    pub buy: bool,
    pub closed_pnl: f64,
    /// Whether this fill arrived on the feed rather than in the opening
    /// snapshot. Only an arrival flashes; the books a trader already had do
    /// not announce themselves on first paint. It never changes afterwards —
    /// the fade is the row's own animation, not a countdown in the data.
    pub hot: bool,
    /// The exchange's trade id, which is how a fill pushed by the feed is
    /// recognised as one the snapshot already listed, and — because a `lazy`
    /// subtree cannot see which iteration built it — the identity its row is
    /// listed under.
    pub tid: i64,
}

impl Hash for Fill {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.coin.hash(state);
        self.ts.hash(state);
        self.buy.hash(state);
        self.hot.hash(state);
        self.tid.hash(state);
        for value in [self.price, self.size, self.closed_pnl] {
            hash_f64(value, state);
        }
    }
}

/// One resting order, listed and drawn on the chart as a level.
#[derive(Clone, PartialEq)]
pub struct Order {
    /// The exchange's own name for this resting order, and the only thing a
    /// cancel can name it by: a coin and a price identify a level, not an
    /// order, and an account may rest several at one price.
    ///
    /// Signed, because it is the neutral shape's field and the two venues name
    /// an order differently: Hyperliquid answers an unsigned `oid`, and Lighter
    /// has no order id at all in its acknowledgement — an order there is named
    /// by the `ClientOrderIndex` its placer chose, which is an `i64` by the
    /// venue's own reckoning. One signed field holds both, and it is what
    /// crosses to Ice, which has no unsigned integer.
    pub oid: i64,
    pub coin: String,
    pub buy: bool,
    pub price: f64,
    pub size: f64,
    pub ts: i64,
}

/// One price level of the book, with the cumulative depth behind it already
/// resolved to a bar width so the view does no arithmetic.
#[derive(Clone, Debug, PartialEq)]
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
    pub(crate) candles: SharedCandles,
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
    /// Split from the right, because a market's name may itself carry a colon.
    /// A builder-deployed market is named `dex:SYMBOL` on the wire — that is
    /// the whole of its identity, and every book, tape and candle request
    /// takes it verbatim — so `xyz:NVDA:1h` splitting from the left yields the
    /// dex as the coin and `NVDA:1h` as the interval, and the feed then holds
    /// a subscription to nothing.
    pub(crate) fn focus(&self) -> Option<(String, String)> {
        let focus = lock(&self.focus);
        let (coin, interval) = focus.rsplit_once(':')?;
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
pub(crate) fn merge(tape: &mut Vec<Candle>, fresh: Vec<Candle>) {
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
pub async fn hl_candles(
    chain: Chain,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    let key = focus_key(&coin, &interval);
    let bars = if lock(&tape.candles).is_empty() {
        BACKFILL_BARS
    } else {
        REFRESH_BARS
    };
    let end = now_ms();
    let start = end - bars * interval_secs(&interval) * 1_000;
    let response = info(
        chain,
        json!({
            "type": "candleSnapshot",
            "req": { "coin": coin, "interval": interval, "startTime": start, "endTime": end },
        }),
    )
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

/// How many of the tape's bars are older than the bar it began at, which is
/// the whole of what a history read achieved. Counted against a timestamp
/// rather than against a length, because the live feed appends at the other
/// end of the same tape: a length that grew by one says nothing about whether
/// the exchange had anything older to give.
pub(crate) fn older_than(candles: &[Candle], oldest: i64) -> i64 {
    candles.partition_point(|candle| candle.ts < oldest) as i64
}

/// Loads the window of candles that ends where the tape currently begins, so
/// a chart panned back to its oldest bar can keep going.
///
/// Answers how many older bars it added. Zero means the venue had nothing
/// before the bar the tape starts at, and the caller must stop asking: the
/// window is derived from that same first bar, so an identical request would
/// come back identically for as long as the chart sits at its left edge. A
/// read that lands after the reader has moved on adds nothing either, and
/// answers zero for the same reason a read of an empty tape does — it moved
/// no left edge.
pub async fn hl_history(
    chain: Chain,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    let key = focus_key(&coin, &interval);
    let oldest = { lock(&tape.candles).first().map(|candle| candle.ts) };
    let Some(oldest) = oldest else {
        return Ok(0);
    };
    let end = oldest * 1_000;
    let start = end - BACKFILL_BARS * interval_secs(&interval) * 1_000;
    let response = info(
        chain,
        json!({
            "type": "candleSnapshot",
            "req": { "coin": coin, "interval": interval, "startTime": start, "endTime": end },
        }),
    )
    .await?;

    let mut candles = lock(&tape.candles);
    if *lock(&tape.focus) != key {
        // The user moved on while this was in flight.
        return Ok(0);
    }
    merge(&mut candles, parse_candles(&response));
    Ok(older_than(&candles, oldest))
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

fn parse_symbols(value: &Value, category: &str, collateral: &str) -> Vec<SymbolRow> {
    let Some(pair) = value.as_array() else {
        return Vec::new();
    };
    let (Some(meta), Some(contexts)) = (pair.first(), pair.get(1)) else {
        return Vec::new();
    };
    let universe = list(meta, "universe");
    let contexts = contexts.as_array().map(Vec::as_slice).unwrap_or_default();
    // Enumerated before the delisting filter as well as before the sort: the
    // index is a position in the venue's own list, and a delisted market still
    // occupies one. Counting only the survivors would shift every market after
    // the first delisting by one.
    let mut rows: Vec<SymbolRow> = universe
        .iter()
        .enumerate()
        .zip(contexts)
        .filter(|((_, asset), _)| {
            !asset
                .get("isDelisted")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .map(|((index, asset), context)| SymbolRow {
            category: category.to_owned(),
            collateral: collateral.to_owned(),
            asset: index as u32,
            leverage: max_leverage(asset),
            maintenance: maintenance_fraction(max_leverage(asset)),
            size_decimals: size_decimals(asset),
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
        // Hyperliquid states no funding time anywhere in an asset context, and
        // the one place its API does publish `nextFundingTime` — the separate
        // `predictedFundings` request — reports a boundary that has already
        // gone by: read at 23:49:06Z on 2026-08-09 it answered 23:00:00Z for
        // every market. A countdown drawn from that would run negative for
        // most of every hour, so this stays zero and `funding_countdown` takes
        // the venue's documented hourly boundary off the clock instead.
        funding_at: 0,
        // The streamed context restates neither the asset's index nor its
        // maximum nor its size step, so the universe's reading of all of them
        // is kept by the caller.
        asset: 0,
        leverage: 0.0,
        maintenance: 0.0,
        size_decimals: 0,
        // Nor which list the market is in, what it settles in, or where the
        // filter put it. Those come from the universe request that named the
        // dex and from the filter that ordered the rows, and the caller keeps
        // both readings for the same reason it keeps the three above.
        category: String::new(),
        collateral: String::new(),
        heading: false,
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

/// The market's own size step. A market that does not publish one is quoted at
/// the finest step there is, because rounding a size the venue would have
/// accepted is the failure this reading exists to prevent.
fn size_decimals(asset: &Value) -> usize {
    asset
        .get("szDecimals")
        .and_then(Value::as_u64)
        .map_or(SIZE_DECIMALS, |decimals| decimals as usize)
        .min(SIZE_DECIMALS)
}

/// What the live exchange's own perp list is headed with. Hyperliquid's own
/// universe is one list among several now, and a group of markets with no name
/// over it reads as "the rest" rather than as the exchange's own book.
///
/// The heading is passed in rather than read from here, because it is the
/// network's own name and the test deployment's name is not this one. This is
/// the mainnet entry's, named once so the registry and the tests below cannot
/// drift apart on the exchange's own spelling.
pub(crate) const HL_CANONICAL: &str = "Hyperliquid";
/// What canonical Hyperliquid margins in, which is also `collateralToken: 0`.
///
/// A constant rather than a network fact, because it is one: both deployments
/// settle their own canonical perps in their own USDC. What *does* differ
/// between them is which builder dexs are listed beside those perps — testnet
/// answers `perpDexs` with `test dex` and `unit dex` where mainnet answers
/// `xyz` — and that arrives in the response rather than from here.
const HL_COLLATERAL: &str = "USDC";

/// The builder-deployed markets a HIP-3 dex lists, as the universe request
/// needs to ask for them and the rail needs to head them.
struct PerpDex {
    /// The wire name, which is also the prefix every one of its markets
    /// carries: `xyz` deploys `xyz:NVDA`.
    name: String,
    /// What the deployer calls it. Several dexs give no full name, and the
    /// short one is then what the rail heads the group with.
    label: String,
}

/// Every dex whose markets this exchange lists, canonical first.
///
/// `perpDexs` answers with `null` in the first slot for the exchange's own
/// perps and one object per builder deployment after it. The `null` is the
/// canonical list and is not asked for by name, so it is dropped here and the
/// caller reads it with no `dex` at all.
fn parse_perp_dexs(value: &Value) -> Vec<PerpDex> {
    value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|entry| !entry.is_null())
        .filter_map(|entry| {
            let name = text(entry, "name");
            if name.is_empty() {
                return None;
            }
            let label = match text(entry, "fullName") {
                empty if empty.is_empty() => name.to_uppercase(),
                full => full,
            };
            Some(PerpDex { name, label })
        })
        .collect()
}

/// The name of every spot token by index, which is how a dex names what it
/// settles in. `collateralToken` is an index and nothing else, so without
/// this the ticket can say how much margin a market wants and not in what.
fn parse_token_names(value: &Value) -> HashMap<usize, String> {
    list(value, "tokens")
        .iter()
        .filter_map(|token| {
            let index = token.get("index").and_then(Value::as_u64)? as usize;
            Some((index, text(token, "name")))
        })
        .collect()
}

/// The whole tradeable universe: the exchange's own perps and every
/// builder-deployed dex listed beside them, each group named.
///
/// This is `1 + n` requests where it used to be one, so they go out together
/// on a pool of blocking threads rather than in a queue — ten round trips end
/// to end is a rail that arrives a second and a half late. A dex whose
/// request fails contributes no rows and does not fail the universe: one
/// builder being unreachable is not the exchange being unreachable.
/// `canonical` is what the exchange's own group is headed with, and it is the
/// network's own name: a rail on the test deployment headed "Hyperliquid" over
/// markets that are not the live exchange's would be the one place on screen
/// that contradicts the header. It is the caller's because which deployment
/// this is is the caller's.
pub async fn hl_symbols(chain: Chain, canonical: &str) -> Result<Vec<SymbolRow>, HlError> {
    // Only the canonical list is required. Without `perpDexs` there is one
    // group, and one group is an uncategorized list — which is what the app
    // drew before any of this, and it stays honest.
    // Neither of these needs the other, and the universe cannot be asked for
    // until the dex list is back, so they go out together.
    let (dexs, tokens) = smol::future::zip(
        info(chain, json!({ "type": "perpDexs" })),
        info(chain, json!({ "type": "spotMeta" })),
    )
    .await;
    let dexs = dexs.as_ref().map(parse_perp_dexs).unwrap_or_default();
    let tokens = tokens.as_ref().map(parse_token_names).unwrap_or_default();
    let requests: Vec<Value> = std::iter::once(json!({ "type": "metaAndAssetCtxs" }))
        .chain(
            dexs.iter()
                .map(|dex| json!({ "type": "metaAndAssetCtxs", "dex": dex.name })),
        )
        .collect();
    let mut answers = smol::unblock(move || batch(chain, &requests))
        .await
        .into_iter();
    // The exchange's own list is the one whose failure is the exchange's.
    let canonical_answer = answers.next().unwrap_or_else(|| Err(panicked_thread()))?;
    let mut groups = vec![parse_symbols(&canonical_answer, canonical, HL_COLLATERAL)];
    for (dex, answer) in dexs.iter().zip(answers) {
        let Ok(answer) = answer else {
            continue;
        };
        let collateral = answer
            .get(0)
            .and_then(|meta| meta.get("collateralToken"))
            .and_then(Value::as_u64)
            .and_then(|index| tokens.get(&(index as usize)))
            .map_or(HL_COLLATERAL, String::as_str);
        let rows = parse_symbols(&answer, &dex.label, collateral);
        // A dex with nothing live to trade is not a group. Most of the
        // deployed ones are in that state at any moment, and a header over no
        // rows is a list of headers.
        if !rows.is_empty() {
            groups.push(rows);
        }
    }
    let categorized = groups.len() > 1;
    Ok(groups
        .into_iter()
        .flatten()
        .map(|row| SymbolRow {
            // One group is not a categorization. Left named, every row of a
            // flat list would be headed and read out as "Hyperliquid", which
            // says nothing a reader did not already know from the window.
            category: if categorized {
                row.category
            } else {
                String::new()
            },
            ..row
        })
        .collect())
}

/// Several `info` requests at once, answered in the order they were asked.
/// Already off the UI thread, so the requests block a pool of scoped threads
/// rather than the executor.
fn batch(chain: Chain, requests: &[Value]) -> Vec<Result<Value, HlError>> {
    std::thread::scope(|scope| {
        let running: Vec<_> = requests
            .iter()
            .map(|body| scope.spawn(move || info_blocking(chain, body)))
            .collect();
        running
            .into_iter()
            .map(|handle| handle.join().unwrap_or_else(|_| Err(panicked_thread())))
            .collect()
    })
}

fn panicked_thread() -> HlError {
    HlError::new("Hyperliquid universe read failed".to_owned())
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

/// Why the ticket is not quoting a cliff, in the words that are true of the
/// state it is actually in.
///
/// `Ticket.known` is one bit over four different situations and the panel
/// used to read all of them out as "market not loaded". That is true of
/// exactly one. A universe that has arrived and does not carry the market on
/// screen is a market this venue does not list — nothing is loading, and
/// waiting will not change it. A market that is listed but states no
/// maintenance requirement is loaded in full, and what is missing is the one
/// figure a cliff is priced against.
///
/// `loaded` is whether the universe itself has arrived, which is the only
/// thing that separates the first two: both have no row for the market, and
/// one of them is going to get one.
///
/// The fourth belongs to the margin mode rather than to the market. A cross
/// position dies against the account's equity, so with no account read there
/// is nothing to measure the fall against — and the market being perfectly
/// well loaded is exactly why this cannot be said in the market's words.
pub fn liquidation_gap(
    market: Option<SymbolRow>,
    loaded: bool,
    cross: bool,
    banked: bool,
) -> String {
    if cross && own_clearinghouse(market.as_ref()) {
        return "separate margin account".to_owned();
    }
    if cross && !banked {
        return "needs the account it is held against".to_owned();
    }
    match market {
        Some(_) => "no requirement stated".to_owned(),
        None if loaded => "not listed here".to_owned(),
        None => "market not loaded".to_owned(),
    }
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
/// against the whole account's equity — so this is the isolated case and
/// `cross_liquidation` is the other one.
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

/// The menu bar mini status: the focused market's coin and last price, or
/// just the coin while the market list is still loading.
///
/// A dead feed does not make the last price wrong — it is still the last thing
/// the exchange said — but it stops it being the price, and the menu bar is
/// the one surface read without the header beside it to say so. The header
/// greys the figure and stamps NOT LIVE next to it; a status item is one
/// string with no ink to spend, so it says the same thing in the same words.
/// With no row to price there is no figure to qualify, and the coin alone
/// claims nothing.
///
/// TESTNET is stamped here and REAL MONEY is not, which is the one place this
/// app marks only one side of that distinction. The header states both because
/// it has room and because a badge whose absence must be noticed is a badge
/// nobody notices. This string is the label, and the label is what a glance
/// gets before any click: the danger a glance can carry is reading a test
/// network as the real one, and the menu below states both in `tray_venue` one
/// click later. `menu_kind` is the same word the header's badge uses.
pub fn tray_status(coin: String, focus: Option<SymbolRow>, live: bool, venue: Venue) -> String {
    let price = match focus.filter(|row| row.price > 0.0) {
        Some(row) if live => format!("{coin} {}", fmt_px(row.price)),
        Some(row) => format!("{coin} {} NOT LIVE", fmt_px(row.price)),
        None => coin,
    };
    if crate::venue::venue_testnet(venue) {
        return format!("{price} TESTNET");
    }
    price
}

/// The alert row, and the one event on this menu worth opening it for.
///
/// A level being hit is the only thing the terminal knows that a reader wants
/// pushed at them rather than looked up, so it takes the top of the menu and
/// says HIT in the word the rail already uses. Waiting alerts are a smaller
/// fact and read as one; no alerts says so rather than leaving the row blank,
/// because a blank row is indistinguishable from a row that failed to fill.
pub fn tray_alerts(alerts: Vec<Alert>) -> String {
    let hit = alerts.iter().filter(|alert| alert.fired).count();
    let waiting = alerts.len() - hit;
    match (hit, waiting) {
        (0, 0) => "No alerts".to_owned(),
        (0, waiting) => format!("{waiting} alert{} waiting", plural(waiting)),
        (hit, 0) => format!("{hit} ALERT{} HIT", plural(hit).to_uppercase()),
        (hit, waiting) => format!(
            "{hit} ALERT{} HIT · {waiting} waiting",
            plural(hit).to_uppercase()
        ),
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// The account submenu's title, which is where its rows get their liveness.
///
/// The header does not qualify equity and PnL by `live`, and it is right not
/// to: `mark_account` re-marks them from the same feed, so the NOT LIVE badge
/// sitting on the same strip covers the whole reading. A menu row inherits no
/// strip. Putting the word on the title rather than on both figures is what
/// keeps that true without stamping the same qualifier twice — a reader cannot
/// reach the rows without opening the row that carries it.
pub fn tray_account(account: Option<Account>, live: bool) -> String {
    match account {
        None => "Account — no address".to_owned(),
        Some(_) if live => "Account".to_owned(),
        Some(_) => "Account — NOT LIVE".to_owned(),
    }
}

/// Equity, in the header's own words and formatter.
///
/// The dash is the header's too: a missing figure is a dash in the slot it
/// will come back to, never a differently shaped row.
pub fn tray_equity(account: Option<Account>) -> String {
    match account {
        Some(held) => format!("EQUITY  {}", fmt_usd(held.value)),
        None => "EQUITY  —".to_owned(),
    }
}

/// Unrealized PnL, through `fmt_pnl` like every other PnL in this app.
///
/// `Account::pnl` rather than a sum over positions: `mark_account` writes it
/// as exactly that sum on every tick, and a second computation of one number
/// is how two figures that must agree stop agreeing.
pub fn tray_pnl(account: Option<Account>) -> String {
    match account {
        Some(held) => format!("PNL  {}", fmt_pnl(held.pnl)),
        None => "PNL  —".to_owned(),
    }
}

/// What the account is in, as one row: how many positions and which coins.
///
/// A menu's row count is fixed when the app compiles, so there is no row per
/// position to give. This says the two things a fixed row honestly can — the
/// count, and the names — and states neither a size nor a PnL per coin, which
/// would need the rows this surface does not have. The window is where a
/// position is read; the menu bar is where you learn there are three.
pub fn tray_positions(positions: Vec<Position>) -> String {
    if positions.is_empty() {
        return "No open positions".to_owned();
    }
    const SHOWN: usize = 4;
    let mut coins: Vec<&str> = positions
        .iter()
        .take(SHOWN)
        .map(|held| held.coin.as_str())
        .collect();
    let more = positions.len().saturating_sub(SHOWN);
    let extra;
    if more > 0 {
        extra = format!("+{more} more");
        coins.push(&extra);
    }
    format!("{} open: {}", positions.len(), coins.join(", "))
}

/// The feed row, named rather than left as a bare figure.
///
/// `fmt_latency` alone puts a lone `—` in the menu when there is no feed,
/// which says nothing at all: a menu row is read without the header's column
/// heading beside it, so it carries its own. A dead feed says so here because
/// this is the row the connection is about — the label says it for the price
/// and the account title says it for the figures, and each of the three is
/// read on its own.
pub fn tray_feed(millis: i64, live: bool) -> String {
    if live {
        format!("FEED  {}", fmt_latency(millis))
    } else {
        "FEED  NOT LIVE".to_owned()
    }
}

/// The venue submenu's title: which exchange, and whether being wrong on it
/// costs anything.
///
/// Both kinds are stated, the same way and in the same place as the header's
/// badge, because this is the surface with room for both. The label above has
/// none and marks only TESTNET; between them a reader never has to notice an
/// absence to know which network they are on.
pub fn tray_venue(venue: Venue) -> String {
    format!(
        "{} — {}",
        crate::venue::venue_name(venue),
        crate::venue::venue_kind(venue)
    )
}

/// A number as typed into a ticket field. Anything that is not one reads as
/// zero, which every figure downstream already treats as "not yet".
pub(crate) fn amount(typed: &str) -> f64 {
    typed.trim().replace(',', "").parse().map_or(
        0.0,
        |value: f64| if value.is_finite() { value } else { 0.0 },
    )
}

/// Where a cross position dies, which is nowhere near where an isolated one
/// does.
///
/// An isolated position stands on the margin posted behind it and on nothing
/// else, so its cliff falls out of its own entry and leverage. A cross
/// position stands on the whole account: it is closed when the account's
/// equity falls to the account's maintenance requirement, and every other
/// cross position it stands beside has already moved that line. Quoting the
/// isolated formula for a cross order puts the cliff further away than it is
/// by however much the account is already carrying — which is precisely the
/// account whose reader needs the figure.
///
/// Holding the other markets still, because they do not move because this one
/// does, equity and requirement as this market travels to `p` are
///
/// ```text
/// equity(p)      = E + (p - mark)·held + (p - entry)·order
/// requirement(p) = M - |held|·mark·m + |held + order|·p·m
/// ```
///
/// `E` and `M` are the account's now, so `M` already counts what this market
/// holds and the term removing it is what stops it being counted twice. They
/// meet at
///
/// ```text
/// p = (M - |held|·mark·m - E + mark·held + entry·order) / (after - |after|·m)
/// ```
///
/// which reduces to the isolated formula for an account holding nothing else:
/// `E` is then the posted margin `entry·order/L` and `M` and `held` are zero,
/// leaving `entry(1 - 1/L)/(1 - m)`.
fn cross_liquidation(
    entry: f64,
    order: f64,
    held: f64,
    mark: f64,
    maintenance: f64,
    equity: f64,
    requirement: f64,
) -> f64 {
    let after = held + order;
    let denominator = after - after.abs() * maintenance;
    if !(denominator.is_finite() && denominator.abs() > f64::EPSILON) {
        return 0.0;
    }
    let numerator =
        requirement - held.abs() * mark * maintenance - equity + mark * held + entry * order;
    let price = numerator / denominator;
    if price.is_finite() && price > 0.0 {
        price
    } else {
        0.0
    }
}

/// Prices a ticket from what the panel has typed into it. Leverage is held
/// inside what the market allows, so a ticket cannot quote a liquidation the
/// exchange would never have opened.
///
/// `entry` is the price the order actually transacts at rather than the one in
/// the field: a market order has nothing in the field, and is quoted at what
/// walking the book would pay. `size` is already in the instrument and already
/// capped by whatever reduce-only promised, because `order_size` does both
/// before anything here is asked — so this function never learns there was a
/// unit toggle or a reduce-only box, and cannot disagree with the panel about
/// which order is being priced.
///
/// The margin requirement is the same figure under either mode — the venue
/// takes `notional/leverage` to open, whichever pocket it comes out of — and
/// the cliff is not, so `cross` chooses which cliff and `account` carries what
/// a cross one is measured against.
#[allow(clippy::too_many_arguments)]
pub fn price_ticket(
    entry: f64,
    size: String,
    leverage: String,
    market: Option<SymbolRow>,
    buy: bool,
    held: f64,
    cross: bool,
    account: Option<Account>,
) -> Ticket {
    let (max_leverage, maintenance) = market
        .as_ref()
        .map_or((0.0, 0.0), |row| (row.leverage, row.maintenance));
    let price = entry.max(0.0);
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
    // A cross cliff is measured against an account, so an unread one leaves
    // nothing to measure — and saying so is the whole reason the panel prints
    // a sentence there instead of a number.
    //
    // A builder-deployed market is not held against the account on screen at
    // all, so the account being read changes nothing: it is the wrong equity
    // and the wrong requirement, and the cliff it produces would be the most
    // confident wrong number in the panel. The isolated cliff is still quoted
    // there, because that one is the market's own arithmetic.
    let backing = account
        .filter(|held| held.cross_value > 0.0)
        .filter(|_| !own_clearinghouse(market.as_ref()));
    let known = maintenance > 0.0 && (!cross || backing.is_some());
    let liquidation = match (ready && known && opening > 0.0, cross, backing) {
        (false, _, _) => 0.0,
        (true, false, _) => ticket_liquidation(price, leverage, maintenance, buy),
        (true, true, None) => 0.0,
        (true, true, Some(account)) => {
            // Only what is held cross in this market moves with it; an
            // isolated position in the same market is standing on its own
            // margin and is not part of this account's fall.
            let held_cross = account
                .positions
                .iter()
                .find(|position| {
                    position.margin_mode == "cross"
                        && market.as_ref().is_some_and(|row| row.name == position.coin)
                })
                .map_or(0.0, |position| position.size);
            let mark = market.as_ref().map_or(price, |row| row.price);
            cross_liquidation(
                price,
                if buy { size } else { -size },
                held_cross,
                if mark > 0.0 { mark } else { price },
                maintenance,
                account.cross_value,
                account.maintenance,
            )
        }
    };
    Ticket {
        notional,
        margin: if ready {
            price * opening / leverage
        } else {
            0.0
        },
        liquidation,
        leverage,
        ready,
        known,
    }
}

/// The size the order is actually for, in the instrument, whatever unit it was
/// typed in and whatever the venue would let it be.
///
/// Two normalizations, and both belong here rather than downstream. The unit
/// toggle is a wording — `$10,000 of BTC` and `0.156 BTC` are one order — so
/// USD is converted to the instrument once and nothing below this learns there
/// was a toggle. And reduce-only is a cap, because the venue trims a
/// reduce-only order to the position rather than filling past it: an order
/// typed larger than the position is quoted at what it would actually do, not
/// at what was typed.
///
/// This is the one figure the panel prints and the one a payload is built
/// from, so the two cannot come apart.
#[allow(clippy::too_many_arguments)]
pub fn order_size(
    size: String,
    usd: bool,
    price: f64,
    market: Option<SymbolRow>,
    reduce: bool,
    held: f64,
    buy: bool,
) -> String {
    let step = size_step(market.as_ref());
    let typed = amount(&size).abs();
    let coins = if usd {
        if price <= 0.0 {
            return String::new();
        }
        // Down onto the step: a size rounded up asks the venue to fill past
        // the dollars that were typed, by however much the step is worth.
        (typed / price * step).floor() / step
    } else {
        typed
    };
    // Reduce-only against the side you hold is refused rather than trimmed, so
    // there is nothing to cap and the refusal beside the box is the answer.
    let capped = if reduce && held != 0.0 && (held > 0.0) != buy {
        coins.min(held.abs())
    } else {
        coins
    };
    if capped <= 0.0 {
        return String::new();
    }
    fmt_size(capped)
}

/// The instrument's own size step, as a multiplier. A market that publishes no
/// step is quoted at the finest there is, because rounding a size the venue
/// would have accepted is the failure the reading exists to prevent.
fn size_step(market: Option<&SymbolRow>) -> f64 {
    10_f64.powi(market.map_or(SIZE_DECIMALS, |row| row.size_decimals) as i32)
}

/// The price a size typed in dollars is converted at, and the price the label
/// beside the field has to name — a conversion whose rate is not on screen is
/// a number the reader cannot check.
///
/// A limit order converts at the price in the field, because `$10,000 of BTC`
/// typed over a limit means at that limit. A market order has no such price
/// and converts at the book's mid.
///
/// Deliberately not what crossing would pay: that price is a function of the
/// size, and the size would then be a function of it.
pub fn size_price(
    market: bool,
    price: String,
    book: Option<Book>,
    focus: Option<SymbolRow>,
) -> f64 {
    if !market {
        let typed = amount(&price);
        if typed > 0.0 {
            return typed;
        }
    }
    book.map(|depth| depth.mid)
        .filter(|mid| *mid > 0.0)
        .or_else(|| focus.map(|row| row.price))
        .unwrap_or(0.0)
}

/// The price every figure in the ticket is quoted at.
///
/// A limit order is quoted at the price in the field. A market order has no
/// price in the field — it has no field — and is quoted at what walking the
/// book on screen would actually pay, which is the figure the panel already
/// prints one row further down. The same arithmetic read once and spent on the
/// value, the requirement and the cliff, rather than printed beside figures
/// that contradict it.
///
/// A market order with no book to walk falls back to the seed the ticket would
/// have opened at, which is the last price the venue stated.
pub fn order_price(
    market: bool,
    price: String,
    book: Option<Book>,
    size: String,
    buy: bool,
    focus: Option<SymbolRow>,
) -> f64 {
    if !market {
        return amount(&price).max(0.0);
    }
    let impact = book_impact(book.clone(), size, buy);
    if impact.ready {
        return impact.paid;
    }
    book.map(|depth| depth.mid)
        .filter(|mid| *mid > 0.0)
        .or_else(|| focus.map(|row| row.price))
        .unwrap_or(0.0)
}

/// The same quantity said in the other unit, which is what pressing the unit
/// toggle asks for. A reader who typed 3 BTC and pressed USD wants to see what
/// 3 BTC costs; leaving the 3 there would turn the order into three dollars of
/// it, and the field looks identical either way.
pub fn retype_size(size: String, usd: bool, price: f64, market: Option<SymbolRow>) -> String {
    let typed = amount(&size).abs();
    if typed <= 0.0 || price <= 0.0 {
        return size;
    }
    if usd {
        format_price(typed * price, 2)
    } else {
        let step = size_step(market.as_ref());
        let coins = (typed / price * step).floor() / step;
        if coins <= 0.0 {
            String::new()
        } else {
            fmt_size(coins)
        }
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

pub async fn hl_account(chain: Chain, address: String) -> Result<Account, HlError> {
    Ok(parse_account(
        &info(
            chain,
            json!({ "type": "clearinghouseState", "user": address }),
        )
        .await?,
    ))
}

/// The account's fills. `hot` is what the list flashes on, so the snapshot the
/// feed opens with reads cold and everything pushed after it arrives lit.
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

/// A fill the exchange did not give a trade id to is dropped rather than
/// listed. `tid` is this row's identity — the key `push_fills` dedupes on and
/// the key its `lazy` row is cached and parked under — and a missing one reads
/// as `0`, which every such fill would then share. Listing them would collapse
/// them into each other; not listing them loses a row the exchange never
/// identified. `userFills` always carries one, so this is a malformed payload
/// either way.
fn parse_fills(value: &Value, hot: bool) -> Vec<Fill> {
    value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|fill| {
            Some(Fill {
                coin: text(fill, "coin"),
                ts: value_i64(fill, "time") / 1_000,
                price: num(fill, "px"),
                size: num(fill, "sz"),
                // "B" is a buy, "A" hits the ask side and is a sell.
                buy: text(fill, "side") == "B",
                closed_pnl: num(fill, "closedPnl"),
                hot,
                tid: fill.get("tid").and_then(Value::as_i64)?,
            })
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
            oid: value_i64(order, "oid").max(0),
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

/// Every agent key the venue currently lists as live for this account, by
/// address, with the millisecond `validUntil` it assigned.
///
/// `extraAgents` lists *live* approvals only — read across 1,899 of them on 47
/// accounts while `session.rs` was written, not one was already lapsed at the
/// moment of reading — so an address missing from this answer is a key that is
/// either unapproved or finished, and either way not one to sign with. The
/// window is the exchange's to assign and ours to read back; the approval
/// action has no field to ask for one.
///
/// An entry with no address is dropped rather than carried: the exchange 422s
/// an approval naming `""`, so a listing containing one is a row this app has
/// nothing to do with, and `session.rs` refuses to hold it anyway.
pub async fn hl_agents(chain: Chain, address: String) -> Result<Vec<(String, i64)>, HlError> {
    let listed = info(chain, json!({ "type": "extraAgents", "user": address })).await?;
    Ok(listed
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|agent| (text(agent, "address"), value_i64(agent, "validUntil")))
        .filter(|(address, _)| !address.is_empty())
        .collect())
}

// The order path: complete, tested against the venue's own answers, and
// pointed at by nothing until the ticket is wired to it. The same shape
// `lighter_sign.rs` states for its own signer — built and held to its evidence
// before a button can reach it, rather than appearing in the same change as the
// button that spends money with it.
#[allow(dead_code)]
/// The one endpoint that changes anything, on the deployment the caller names.
///
/// Behind the same `wire_is_open` gate every read passes, and that is not
/// symmetry for its own sake: it is the reason no test in this suite can place
/// an order. A test drives the real program, subscriptions and handlers
/// included, so a gate on the reads alone would leave the one path that spends
/// money as the only one a test could reach.
pub(crate) async fn exchange(chain: Chain, body: Value) -> Result<Value, HlError> {
    if !wire_is_open() {
        return Err(HlError::new(
            "Hyperliquid unreachable: no wire under test".to_owned(),
        ));
    }
    smol::unblock(move || {
        let mut response = agent()
            .post(chain.exchange_url())
            // A rejection arrives as a 200 carrying `status: "err"`, and a 4xx
            // is a malformed action rather than a transport failure. Both are
            // the venue's answer and both have to reach the reader in its own
            // words, so neither is turned into a thrown transport error here.
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(&body)
            .map_err(|error| HlError::new(format!("Hyperliquid unreachable: {error}")))?;
        let status = response.status();
        response
            .body_mut()
            .read_json::<Value>()
            .map_err(|error| HlError::new(format!("Hyperliquid answered {status} with {error}")))
    })
    .await
}

#[allow(dead_code)]
/// What the exchange said about one submitted action.
///
/// Hyperliquid answers a refusal with HTTP 200 and `status: "err"`, and answers
/// a *partial* refusal with `status: "ok"` and an `error` inside one of the
/// per-order statuses — so "the request succeeded" and "the order rested" are
/// two different questions and only the second one matters. Reading the outer
/// status alone reports a rejected order as placed.
/// What the exchange actually did with the order, in its own numbers.
///
/// Three outcomes and they are not one word: an order can rest whole, fill
/// whole, or fill part and rest the remainder — and an immediate-or-cancel one
/// fills part and cancels the rest. Reporting any of those at the size that was
/// *typed* tells a trader they hold something they do not.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placed {
    /// The id the remainder rests under, or zero when nothing rested.
    pub resting: i64,
    /// How much filled on arrival, as the venue counted it.
    pub filled: f64,
    /// The average price it filled at, and zero when nothing filled.
    pub at: f64,
}

fn placed(answer: &Value) -> Result<Placed, HlError> {
    if let Some(message) = answer.get("response").and_then(Value::as_str) {
        return Err(HlError::new(message.to_owned()));
    }
    if text(answer, "status") == "err" {
        // The venue puts its sentence where the payload would be.
        let said = answer
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("Hyperliquid refused the action and said nothing");
        return Err(HlError::new(said.to_owned()));
    }
    let statuses = answer
        .get("response")
        .and_then(|response| response.get("data"))
        .map(|data| list(data, "statuses"))
        .unwrap_or_default();
    if statuses.is_empty() {
        return Err(HlError::new(
            "Hyperliquid accepted the request and reported nothing about the order".to_owned(),
        ));
    }
    let mut done = Placed::default();
    // ponytail: one `Placed` for the whole batch, which reads a bracket's three
    // statuses as one order — `resting` ends up whichever leg answered last,
    // and `receipt` would name a stop's id as the order that rested. Correct
    // for every batch this app can currently send, because `attaches_levels` is
    // false everywhere and `wire_orders` therefore always builds exactly one
    // leg. Upgrade path: answer a `Placed` per leg and let `receipt` say what
    // happened to the entry and what is now guarding it. Deliberately not built
    // here — what the venue actually returns for each leg of a `normalTpsl`
    // group is the thing no offline test can establish, and guessing it is how
    // a receipt starts lying about protection that never rested.
    for status in statuses {
        // One refusal among several is still a refusal, and the venue's own
        // sentence is the only useful thing to say about it.
        if let Some(said) = status.get("error").and_then(Value::as_str) {
            return Err(HlError::new(said.to_owned()));
        }
        // A resting order answers with its id. One that crossed answers how
        // much of it crossed and at what — read rather than assumed, because
        // the amount the venue filled is the only honest thing to report and it
        // is not always the amount that was asked for.
        if let Some(resting) = status.get("resting") {
            done.resting = value_i64(resting, "oid");
        }
        if let Some(filled) = status.get("filled") {
            done.filled += num(filled, "totalSz");
            done.at = num(filled, "avgPx");
        }
    }
    Ok(done)
}

#[allow(dead_code)]
/// Send a signed action and read what the venue made of it.
async fn acted(
    chain: Chain,
    wallet: &Wallet,
    action: &Action<signing::Trading>,
) -> Result<Placed, HlError> {
    placed(&exchange(chain, action.request(wallet)).await?)
}

#[allow(dead_code)]
/// Place one order, or a batch the exchange is to read as one thing, and answer
/// the id it rests under.
///
/// The market is named by its index, which is what the wire carries — see
/// `SymbolRow::asset`. A market whose margin lives in a clearinghouse this app
/// cannot read is refused here as well as on the ticket, because an order is
/// the one place where being wrong about which account backs it costs money.
///
/// A slice and a grouping rather than one order, because the two are the same
/// argument: legs sent under `Grouping::Na` are unrelated orders however they
/// were meant, and a grouping over one leg groups nothing. Sending them apart
/// would let a caller pair an entry with a stop that was never attached to it,
/// which is exactly the failure the whole gate exists to prevent.
///
/// **An empty batch is refused rather than sent.** The exchange takes an empty
/// `orders` array happily and does nothing with it, which would come back
/// through `placed` as "accepted and reported nothing" — a sentence the reader
/// would have to read as a venue problem rather than as this app having
/// dropped every leg it was given.
pub async fn hl_place(
    chain: Chain,
    wallet: &Wallet,
    market: &SymbolRow,
    wires: &[signing::Order],
    grouping: signing::Grouping,
) -> Result<Placed, HlError> {
    if market.name.contains(':') {
        return Err(HlError::new(format!(
            "{} is margined against a clearinghouse this app cannot read, so it will not send \
             an order there.",
            market.name,
        )));
    }
    if wires.is_empty() {
        return Err(HlError::new(
            "There is nothing in this order to send.".to_owned(),
        ));
    }
    let action = signing::order(chain, wires, grouping, now_ms() as u64)?;
    acted(chain, wallet, &action).await
}

#[allow(dead_code)]
/// Pull one resting order by the id the exchange gave it.
pub async fn hl_cancel(
    chain: Chain,
    wallet: &Wallet,
    market: &SymbolRow,
    oid: i64,
) -> Result<(), HlError> {
    let action = signing::cancel(
        chain,
        &[signing::Cancel {
            asset: market.asset,
            // The wire is unsigned; the neutral row is not, and an id below
            // zero is one no exchange issued.
            oid: oid.max(0) as u64,
        }],
        now_ms() as u64,
    );
    acted(chain, wallet, &action).await.map(|_| ())
}

pub async fn hl_orders(chain: Chain, address: String) -> Result<Vec<Order>, HlError> {
    Ok(parse_orders(
        &info(chain, json!({ "type": "openOrders", "user": address })).await?,
    ))
}

/// One websocket connection, plus the set of subscriptions it is currently
/// holding open.
struct Socket {
    ws: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    open: Vec<Value>,
}

impl Socket {
    fn connect(chain: Chain) -> Result<Self, HlError> {
        // The same gate `info` passes, and for the same reason: a test drives
        // the real program, so a handler that starts a feed would open a
        // socket to the exchange and make the suite depend on it being up.
        // Nothing dispatched a feed until the venue switch landed, which is
        // the only reason this was not here already.
        if !wire_is_open() {
            return Err(HlError::new(
                "Hyperliquid feed unreachable: no wire under test".to_owned(),
            ));
        }
        let (ws, _) = tungstenite::connect(chain.ws_url())
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
///
/// Shared with the Lighter adapter, whose reader answers the same three
/// events, so a test that walks either one walks the same sequence. The socket
/// that produces them is not shared: only these three moments are common to
/// the two venues, and everything either says on the wire differs.
pub(crate) enum Event<'a> {
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
fn feed<T, S, R>(chain: Chain, mut subscribe: S, mut read: R) -> Receiver<Result<T, HlError>>
where
    T: Send + 'static,
    S: FnMut() -> Vec<Value> + Send + 'static,
    R: FnMut(Event<'_>) -> Option<T> + Send + 'static,
{
    let (sender, receiver) = feed_channel();
    std::thread::spawn(move || {
        while !sender.is_closed() {
            let Err(error) = pump(chain, &mut subscribe, &mut read, &sender) else {
                return;
            };
            if sender.send_blocking(Err(error)).is_err() {
                return;
            }
            // Retrying is for a connection that dropped. Under test there is no
            // wire to reconnect to, and a feed that kept trying would never let
            // the app settle — a handler that starts one could not be dispatched
            // by a test at all.
            if !wire_is_open() {
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
    chain: Chain,
    subscribe: &mut S,
    read: &mut R,
    sender: &Sender<Result<T, HlError>>,
) -> Result<(), HlError>
where
    S: FnMut() -> Vec<Value>,
    R: FnMut(Event<'_>) -> Option<T>,
{
    let mut socket = Socket::connect(chain)?;
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
    pub(crate) mids: HashMap<String, f64>,
    /// Prints on the charted market since the last beat, oldest first.
    pub(crate) trades: Vec<Trade>,
    /// A republished market context, when one arrived. It names the market it
    /// describes, and `apply_feed` writes it to the row of that name.
    pub(crate) context: Option<SymbolRow>,
}

/// The market data feed: every mid price on the exchange, and the book,
/// candles, and context of whatever market the tape is pointed at. Candles
/// are merged into the tape in place, so the chart follows them on its own
/// repaint beat without an app message per tick.
pub fn hl_market_feed(chain: Chain, tape: Tape) -> Receiver<Result<MarketTick, HlError>> {
    let subscriptions = tape.clone();
    // `allMids` answers for one dex at a time and the canonical request does
    // not carry the builder ones, so the rail's prices for a hundred-odd
    // builder-deployed markets would sit at whatever the universe read once
    // and never move again. Read on the feed's own thread, once per
    // connection: a frozen price column that looks live is the failure this
    // list exists to avoid.
    let mut dexs: Option<Vec<String>> = None;
    feed(
        chain,
        move || {
            let named = dexs.get_or_insert_with(|| {
                info_blocking(chain, &json!({ "type": "perpDexs" }))
                    .as_ref()
                    .map(parse_perp_dexs)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|dex| dex.name)
                    .collect()
            });
            let mut wanted = vec![json!({ "type": "allMids" })];
            wanted.extend(
                named
                    .iter()
                    .map(|dex| json!({ "type": "allMids", "dex": dex })),
            );
            if let Some((coin, interval)) = subscriptions.focus() {
                wanted.push(json!({ "type": "l2Book", "coin": coin }));
                wanted.push(json!({ "type": "activeAssetCtx", "coin": coin }));
                wanted.push(json!({ "type": "candle", "coin": coin, "interval": interval }));
                wanted.push(json!({ "type": "trades", "coin": coin }));
            }
            wanted
        },
        market_reader(tape),
    )
}

/// What the market feed makes of one connection's traffic. It is apart from
/// the socket so that a test can walk it through the sequence a market switch
/// produces, which is the only way to reach these arms without the exchange.
fn market_reader(tape: Tape) -> impl FnMut(Event<'_>) -> Option<MarketTick> + Send + 'static {
    let mut tick = MarketTick::default();
    let mut changed = false;
    // Which market the per-market fields belong to. The book is the focused
    // market's and carries nothing that says so once parsed, and the app can
    // move that focus between two beats: without this the next beat
    // republishes the book of the market the reader just left, over the top
    // of the one they are looking at. The context is the exception and needs
    // no coin guard: it keeps the coin it names and lands on that row.
    let mut held: Option<(String, String)> = None;
    move |event| match event {
        Event::Payload("allMids", data) => {
            // Merged rather than assigned: one of these arrives per dex, each
            // carrying only its own markets. Assigned, the last message of a
            // beat would be the only prices the rail saw, and every other
            // group's column would go blank and back on alternate beats.
            tick.mids.extend(
                data.get("mids")
                    .and_then(Value::as_object)?
                    .iter()
                    .filter_map(|(coin, price)| {
                        Some((coin.clone(), price.as_str()?.parse().ok()?))
                    }),
            );
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
            // A book for the market the app just left would be read as
            // this one's. Clearing the held book on the switch is not
            // enough on its own: the socket keeps serving the old
            // subscription until the unsubscribe takes effect, so a book
            // that arrives after the switch has to be turned away by the
            // coin it names.
            let (coin, _) = tape.focus()?;
            if text(data, "coin") != coin {
                return None;
            }
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
    }
}

/// This account's fills as they print. The exchange opens with a snapshot of
/// the recent ones and pushes each new one after that; only the pushed ones
/// are lit, so the list flashes what just happened rather than its history.
pub fn hl_fill_feed(chain: Chain, address: String) -> Receiver<Result<Vec<Fill>, HlError>> {
    feed(
        chain,
        move || vec![json!({ "type": "userFills", "user": address })],
        move |event| {
            let Event::Payload("userFills", data) = event else {
                return None;
            };
            let snapshot = data
                .get("isSnapshot")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let fills = parse_fills(data.get("fills")?, !snapshot);
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
            // The index is the one field here an order is actually sent with,
            // and a beat that dropped it would leave every market pointing at
            // whatever `parse_context` zeroes to — which is a real market.
            asset: row.asset,
            // All three belong to the asset rather than to the day, and the
            // streamed context does not carry them.
            leverage: row.leverage,
            maintenance: row.maintenance,
            size_decimals: row.size_decimals,
            // Nor does it carry which list the market is in or what it
            // settles in. A beat that dropped those would move a row out of
            // its group and quote its margin in the wrong token, on nothing
            // more than a price having ticked.
            category: std::mem::take(&mut row.category),
            collateral: std::mem::take(&mut row.collateral),
            heading: row.heading,
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

/// Whether an account was read at all, which the panels under it need as a
/// plain bool because the empty list means two different things. "No open
/// positions on this account" is a claim about an account, and with none it
/// names one that does not exist.
pub fn account_read(account: Option<Account>) -> bool {
    account.is_some()
}

/// What an account read came back holding, or nothing when there was no
/// account to read. The positions panel draws this list rather than reaching
/// into the account, so that "no address" and "an account with no positions"
/// stay two different states of the same handler.
pub fn held_positions(account: Option<Account>) -> Vec<Position> {
    account.map(|held| held.positions).unwrap_or_default()
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

/// The smallest price step this market is actually quoting, read off the book
/// rather than assumed.
///
/// Both venues publish a book already grouped to the tick they accept, so the
/// gap between two adjacent levels is the tick — and one tick is what a trader
/// means by "one up". Taken as the smallest gap across both sides merged,
/// because the one gap in that list that is *not* a tick is the spread, and it
/// is never the smallest.
///
/// With no book to read, one unit of the precision the price is printed at:
/// the smallest step that changes what the field says.
pub(crate) fn book_tick(book: Option<&Book>, price: f64) -> f64 {
    let mut levels: Vec<f64> = book
        .into_iter()
        .flat_map(|depth| depth.bids.iter().chain(depth.asks.iter()))
        .map(|level| level.price)
        .collect();
    levels.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    levels
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|gap| *gap > 0.0)
        .fold(None, |best: Option<f64>, gap| {
            Some(best.map_or(gap, |best| best.min(gap)))
        })
        .unwrap_or_else(|| 10_f64.powi(-(price_decimals(price) as i32)))
}

pub fn fmt_px(value: f64) -> String {
    format_price(value, price_decimals(value))
}

pub fn fmt_usd(value: f64) -> String {
    format!("${}", format_price(value, 2))
}

/// Whether this market's margin is held somewhere other than the account the
/// screen is reading.
///
/// A builder-deployed market is named `dex:SYMBOL` on the wire, and that
/// prefix is the whole of the difference: `clearinghouseState` for the same
/// address answers with a different equity, different positions and a
/// different maintenance requirement depending on whether it carries that
/// dex. Nothing else about the market says so — the book, the tape and the
/// candles are read exactly as any other market's — so this one reading is
/// what every account-relative figure on the ticket is gated on.
fn own_clearinghouse(market: Option<&SymbolRow>) -> bool {
    market.is_some_and(|row| row.name.contains(':'))
}

/// What a group's header says beside its name, which is the collateral when
/// that is not the venue's own and nothing at all when it is.
///
/// Empty rather than "USDC" on the canonical list: the default is what every
/// reader already assumes, and printed on every header it would be the one
/// word that appears most often in the rail while carrying the least. Printed
/// on a group that settles in USDe it is the difference between two lists that
/// look interchangeable and two that are not.
pub fn group_note(market: SymbolRow) -> String {
    if market.collateral == HL_COLLATERAL {
        return String::new();
    }
    market.collateral
}

/// A margin requirement in the token it is actually posted in.
///
/// Canonical Hyperliquid margins in USDC and the figure is dollars. A builder
/// dex names its own collateral, and the live ones are not all USDC: read
/// live, `flx`, `vntl` and `km` settle in USDH, `hyna` in USDe and `cash` in
/// USDT0. The arithmetic is the same either way — price times size over
/// leverage — but a dollar sign in front of a USDe figure is the panel
/// claiming a peg it has not checked.
pub fn fmt_margin(value: f64, market: Option<SymbolRow>) -> String {
    match market {
        Some(row) if !row.collateral.is_empty() && row.collateral != HL_COLLATERAL => {
            format!("{} {}", format_price(value, 2), row.collateral)
        }
        _ => fmt_usd(value),
    }
}

pub fn fmt_signed_usd(value: f64) -> String {
    let sign = if value >= 0.0 { "+" } else { "-" };
    format!("{sign}${}", format_price(value.abs(), 2))
}

pub fn fmt_pct(value: f64) -> String {
    let sign = if value >= 0.0 { "+" } else { "" };
    format!("{sign}{value:.2}%")
}

/// A size, quoted at the precision it actually carries. The exchange quantizes
/// a size to the instrument's step before it sends it, so the digits a size
/// arrives with are the market's and none of them may be dropped: a quote that
/// coarsened as the size grew seeded CLOSE POSITION with a size that no longer
/// closes the position, leaving a residual open or flipping into the other
/// side. Trailing zeros go because a size of thirty is thirty, not thirty
/// dollars and no cents.
pub fn fmt_size(value: f64) -> String {
    format_price(value.abs(), SIZE_DECIMALS)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
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

/// Leverage as it was actually used, to the hundredth. Rounding it to a whole
/// number printed a figure the margin and the liquidation beside it were never
/// computed from: a ticket levered at 2.5 said 3x while pricing at 2.5. The
/// digits still have to stop somewhere: the field is free text, the cell is a
/// fixed width, and a fraction typed out to fifteen places renders as a string
/// as long as it was typed.
pub fn fmt_leverage(value: f64) -> String {
    let value = format!("{value:.2}");
    format!("{}x", value.trim_end_matches('0').trim_end_matches('.'))
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
/// Funding as the reader's cash flow rather than as the venue's charge.
///
/// `Position.funding` is what the position has been CHARGED, which is the
/// sign both venues report and the sign the arithmetic wants. It is the
/// opposite of what a column headed FUNDING means to the person reading it:
/// money that left the account is negative there, the way it is in every
/// other money column on this screen. Shown as the charge, a position that
/// had been PAID funding read as a loss, in the colour a loss is drawn in.
pub fn fmt_funding_flow(charged: f64) -> String {
    fmt_compact_usd(-charged)
}

/// Whether that flow was into the account, for the colour beside it. A charge
/// is a cost and reads the way costs read.
pub fn funding_received(charged: f64) -> bool {
    charged <= 0.0
}

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

/// One rule for every PnL on screen — the account header, the position rows
/// and the fills: exact while it is small enough to read, compact once it is
/// not. Two formatters printed the same number twice on one screen, so an
/// account down thirty thousand read "-$30,000.00" above "-$30.0K".
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
///
/// The search reaches every category, because the list it walks is every
/// category: the universe arrives already ordered group by group, so filtering
/// it flat leaves the groups intact and narrower. Typing `NVDA` therefore finds
/// the builder-deployed market of that name even though no canonical row
/// matches, and the group it belongs to still says which list it came out of.
///
/// Which rows head their group is decided here rather than at parse time for
/// the same reason: a search that removes the first row of a group makes the
/// next one the heading, and only the filter knows what is left.
pub fn filter_symbols(rows: Vec<SymbolRow>, query: String, coin: String) -> Vec<SymbolRow> {
    let query = query.trim().to_uppercase();
    let mut headed = String::new();
    rows.into_iter()
        .filter(|row| query.is_empty() || row.name.to_uppercase().contains(&query))
        .map(|row| {
            let heading = !row.category.is_empty() && row.category != headed;
            if heading {
                headed = row.category.clone();
            }
            SymbolRow {
                selected: row.name == coin,
                heading,
                ..row
            }
        })
        .collect()
}

pub fn symbol_row(rows: Vec<SymbolRow>, coin: String) -> Option<SymbolRow> {
    rows.into_iter().find(|row| row.name == coin)
}

/// Which market the terminal is left pointed at once a universe arrives: the
/// one it is already on when this universe lists it, and otherwise the venue's
/// own busiest market.
///
/// A ticker is not portable. Read live on the day this was written,
/// Hyperliquid listed 232 markets and Lighter 205 active ones with 93 names in
/// common, and the near-misses are spelled apart rather than shared —
/// Hyperliquid's kPEPE, kSHIB and kBONK are Lighter's 1000PEPE, 1000SHIB and
/// 1000BONK, while Lighter alone lists perps on shares, metals and currencies.
/// So a ticker carried across a switch names a market the venue being opened
/// may never have heard of, and every panel then draws nothing while the
/// header goes on naming it. The same thing happens without a switch, more
/// slowly: a market delisted under a terminal that is watching it stops being
/// in the next universe the 60-second poll reads.
///
/// The fallback is the first row rather than a ticker named here, because both
/// adapters sort a universe by the day's notional volume before returning it.
/// The first row is therefore whatever this venue trades most — the one market
/// it is certain to list, the one whose book and tape have something in them,
/// and the one a reader who has expressed no preference is best pointed at. It
/// also stays true when the busiest market changes, which a name typed here
/// would not.
///
/// An empty universe is a read that answered nothing rather than a venue that
/// lists nothing, so the market on screen is kept: landing on the first of no
/// rows would throw the reader's market away every time a request came back
/// empty.
pub fn listed_coin(rows: Vec<SymbolRow>, coin: String) -> String {
    if rows.iter().any(|row| row.name == coin) {
        return coin;
    }
    rows.first().map_or(coin, |row| row.name.clone())
}

/// Puts the fills the feed just pushed on top of the ones already listed,
/// newest first, ignoring any the opening snapshot had already shown, and
/// capped so the list builds a bounded number of rows however long the
/// account trades for.
///
/// Every fill the app lists comes through here, so this is where the list's
/// one invariant is enforced: **no two listed fills share a `tid`**. The rows
/// are `lazy`, keyed and parked by that id, and a repeat is not merely a
/// duplicate on screen — the second row displaces the first in the memo lot.
/// `a_fill_without_a_trade_id_is_not_listed` and
/// `push_fills_lists_each_trade_id_once` hold both halves down.
pub fn push_fills(history: Vec<Fill>, incoming: Vec<Fill>, limit: i64) -> Vec<Fill> {
    // Seeded from the history — which is a previous result of this function,
    // and unique by induction — then grown as each incoming fill is admitted,
    // so a batch that repeats a trade id lists it once.
    let mut seen: HashSet<i64> = history.iter().map(|fill| fill.tid).collect();
    let mut rows: Vec<Fill> = incoming
        .into_iter()
        .filter(|fill| seen.insert(fill.tid))
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

/// A position row carries a side, a size, an entry, a liquidation price, a
/// funding flow and a PnL. The row is one button, and a button's label replaces
/// every cell inside it rather than introducing them, so a name that stopped at
/// the side and the size asked a reader who cannot see the other five columns
/// whether this position is making money or about to be closed for them. It
/// names one figure per column the panel gives a header to.
pub fn position_label(held: Position) -> String {
    let side = if held.size >= 0.0 { "long" } else { "short" };
    // The LIQ column draws "none" for a position the venue reports no cliff
    // for, and a label may not read out a price the panel does not show.
    let liq = if held.liq > 0.0 {
        format!("liquidation {}", fmt_px(held.liq))
    } else {
        "no liquidation price".to_owned()
    };
    format!(
        "{} {side} {}, entry {}, {liq}, funding {}, unrealized {} at {}",
        held.coin,
        fmt_size(held.size),
        fmt_px(held.entry),
        fmt_funding_flow(held.funding),
        fmt_pnl(held.pnl),
        fmt_pct(held.roe_pct)
    )
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

/// Why a level cannot be watched, or empty when it can. Every refusal
/// `add_alert` makes is one of these, so the button that calls it reads the
/// same answer and says which one it is instead of swallowing the press.
fn alert_refusal(alerts: &[Alert], coin: &str, price: f64, mark: f64) -> &'static str {
    if coin.is_empty() || mark <= 0.0 {
        return "This market has no price yet to watch a level against.";
    }
    if price <= 0.0 {
        return "A level is a price above zero.";
    }
    // A level at the mark has already happened, and a duplicate is not a
    // second alert.
    if (price - mark).abs() < f64::EPSILON {
        return "That level is where this market is trading now.";
    }
    if alerts
        .iter()
        .any(|held| held.coin == coin && (held.price - price).abs() < f64::EPSILON)
    {
        return "That level is already being watched.";
    }
    ""
}

pub fn alert_refused(alerts: Vec<Alert>, coin: String, price: String, mark: f64) -> String {
    alert_refusal(&alerts, &coin, amount(&price), mark).to_owned()
}

pub fn add_alert(alerts: Vec<Alert>, coin: String, price: String, mark: f64) -> Vec<Alert> {
    let price = amount(&price);
    let mut alerts = alerts;
    if !alert_refusal(&alerts, &coin, price, mark).is_empty() {
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

/// The whole row is the button that drops the alert, so its label has to say
/// that. A status line — "BTC waiting above 64,400" — reads as something to
/// look at, and this one deletes the level it names when it is pressed.
pub fn alert_label(alert: Alert) -> String {
    // A level that has already been reached has no side left to wait on.
    let side = if alert.fired {
        "at"
    } else if alert.above {
        "above"
    } else {
        "below"
    };
    format!(
        "Stop watching {} {side} {}",
        alert.coin,
        fmt_px(alert.price)
    )
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
/// This is the only size the app works out rather than reads, so it is the
/// only one that has to be put back on the instrument's step. Downward: a size
/// rounded up asks the margin engine for more than the account has free, by
/// however much the step is worth. On a market that trades in whole units that
/// floor can land on nothing, and a MAX that fills in "0" offers to send an
/// order for none of the instrument: there is no size to offer, and this says
/// so the same way it says the account has nothing free.
///
/// It fills the field, so it answers in whatever unit the field is being typed
/// in. A share of an account is a dollar figure to begin with — the conversion
/// runs to reach a size, not away from one — so the USD case is the shorter
/// arithmetic rather than a second one.
///
/// `price` is the one the *field* is read at rather than the one the order
/// transacts at, and the difference only exists for a market order. A dollar
/// figure filled in here is turned back into a size by `order_size` at that
/// same price, so handing this the crossing price would put a size in the box
/// that the box's own arithmetic disagreed with. What that costs is the
/// spread, on the one press that is already floored to the instrument's step
/// for the same reason — and the panel prints what crossing pays a row below.
pub fn ticket_afford(
    account: Option<Account>,
    price: f64,
    market: Option<SymbolRow>,
    leverage: f64,
    share: f64,
    usd: bool,
) -> String {
    // Sizing to a share of the balance needs the balance this market is
    // actually margined against, and for a builder-deployed market that is
    // not the account on screen. Read live, one address held $127,575 on the
    // canonical clearinghouse and $5,235,542 on the `xyz` dex at the same
    // moment: "50% of your account" priced off the wrong one of those is not
    // a rounding error, so the buttons decline rather than answer.
    if own_clearinghouse(market.as_ref()) {
        return String::new();
    }
    let free = account.map_or(0.0, |held| held.withdrawable);
    if free <= 0.0 || price <= 0.0 || leverage <= 0.0 || share <= 0.0 {
        return String::new();
    }
    let notional = free * share.min(1.0) * leverage;
    if usd {
        return format_price(notional, 2);
    }
    let step = size_step(market.as_ref());
    let size = (notional / price * step).floor() / step;
    if size <= 0.0 {
        return String::new();
    }
    fmt_size(size)
}

/// What a share button fills in, which is a share of two different things.
///
/// Opening, it is a slice of what the account could put on. Once CLOSE
/// POSITION has set reduce-only, the only quantity on the screen is the
/// position itself, and "50%" has to be half of what is held: sized off free
/// margin it is a number with no relation to the thing being closed, and it
/// looks exactly like a size. `order_size` already caps a reduce-only order at
/// the position, so the old behaviour was not dangerous — it was silently
/// wrong, which is worse to read.
///
/// MAX closes the position rather than the position floored to the
/// instrument's step. Every other size this app works out is floored, because
/// rounding an *opening* order up asks the margin engine for margin that is not
/// there; a close asks for none, and the floor would leave a few thousandths of
/// dust the venue still carries as an open position. The figure came off the
/// venue on the step it quotes, so there is nothing to round.
///
/// The account and the leverage are still taken and are deliberately unread on
/// the reduce path: closing asks nothing of either.
#[allow(clippy::too_many_arguments)]
pub fn share_size(
    account: Option<Account>,
    price: f64,
    market: Option<SymbolRow>,
    leverage: f64,
    share: f64,
    usd: bool,
    reduce: bool,
    held: f64,
) -> String {
    if !reduce {
        return ticket_afford(account, price, market, leverage, share, usd);
    }
    let position = held.abs();
    if position <= 0.0 || share <= 0.0 {
        return String::new();
    }
    let coins = if share >= 1.0 {
        position
    } else {
        let step = size_step(market.as_ref());
        (position * share * step).floor() / step
    };
    if coins <= 0.0 {
        return String::new();
    }
    // The field answers in the unit it is being typed in, the way the
    // buying-power path does. A share of a position starts as a size, so here
    // it is the dollar case that is the extra conversion.
    if usd {
        if price <= 0.0 {
            return String::new();
        }
        return format_price(coins * price, 2);
    }
    fmt_size(coins)
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
    // The whole reading is this account's maintenance requirement before and
    // after, and a builder-deployed market is not held against this account
    // at all — it has its own clearinghouse, its own equity and its own
    // collateral token. Quoting the canonical account's load for an order
    // that would never touch it is the most confident wrong number on the
    // screen, so the row says which account it cannot see instead.
    if own_clearinghouse(market.as_ref()) {
        return "separate margin account".to_owned();
    }
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
pub fn funding_day(market: Option<SymbolRow>, price: f64, size: String, buy: bool) -> String {
    let Some(market) = market else {
        return String::new();
    };
    let notional = price.max(0.0) * amount(&size).abs();
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
    let held = position_held(positions, coin);
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

/// Why reduce-only cannot be sent with the order as typed, or empty when it
/// can.
///
/// Reduce-only is a promise to the venue that the order only ever moves the
/// position towards zero, and the venue keeps that promise by refusing the
/// order outright rather than by shrinking it. So an order that would add to
/// what is held is not sent smaller — it is not sent, and a box that quietly
/// guaranteed nothing would have been the reader's only warning.
///
/// CLOSE POSITION is this same promise with the size and the side filled in,
/// which is why it sets the box rather than carrying a second path of its own.
pub fn reduce_refused(positions: Vec<Position>, coin: String, buy: bool) -> String {
    let held = position_held(positions, coin);
    if held == 0.0 {
        return "Reduce-only needs a position to reduce, and there is none in this market."
            .to_owned();
    }
    if (held > 0.0) == buy {
        let side = if held > 0.0 { "long" } else { "short" };
        return format!(
            "This order adds to the {side} you hold. Reduce-only sends nothing rather than a smaller order."
        );
    }
    String::new()
}

/// What an exit at this level would realize on the order in the ticket: the
/// same reading the positions table shows against a mark, pointed instead at a
/// price that has not happened yet. Entry to exit, times the size, signed by
/// the side the order opens.
pub fn level_pnl(entry: f64, exit: String, size: String, buy: bool) -> f64 {
    let exit = amount(&exit);
    let size = amount(&size).abs();
    if entry <= 0.0 || exit <= 0.0 || size <= 0.0 {
        return 0.0;
    }
    if buy {
        (exit - entry) * size
    } else {
        (entry - exit) * size
    }
}

/// Why a take-profit cannot be attached at that level, or empty when it can.
///
/// A target on the wrong side of the entry is a stop wearing the wrong name,
/// and the venue sends it as one: the trigger is already true when the order
/// fills, so the position closes at a loss immediately. That is one press away
/// from the opposite of what was asked for, so it is refused with the reason
/// rather than accepted quietly.
///
/// An empty field is not a refusal. Take-profit is optional, and a blank one
/// is the order without it.
pub fn tp_refused(entry: f64, price: String, buy: bool) -> String {
    if price.trim().is_empty() {
        return String::new();
    }
    if entry <= 0.0 {
        return "There is no entry price yet to set a target against.".to_owned();
    }
    let level = amount(&price);
    if level <= 0.0 {
        return "A take-profit is a price above zero.".to_owned();
    }
    if (buy && level <= entry) || (!buy && level >= entry) {
        let side = if buy { "long" } else { "short" };
        let direction = if buy { "above" } else { "below" };
        return format!(
            "A take-profit on a {side} sits {direction} the {} it opens at.",
            fmt_px(entry)
        );
    }
    String::new()
}

/// Why a stop-loss cannot be attached at that level, or empty when it can.
///
/// Two refusals rather than one. The wrong side of the entry is a target
/// wearing the wrong name — the take-profit's mistake, mirrored, and refused
/// in the same words. Past the liquidation is worse than wrong: it is a stop
/// that reads as protection and is not there, because the engine closes the
/// position before the trigger is ever reached, at the engine's price and not
/// at the chosen one.
pub fn sl_refused(entry: f64, price: String, buy: bool, liquidation: f64) -> String {
    if price.trim().is_empty() {
        return String::new();
    }
    if entry <= 0.0 {
        return "There is no entry price yet to set a stop against.".to_owned();
    }
    let level = amount(&price);
    if level <= 0.0 {
        return "A stop-loss is a price above zero.".to_owned();
    }
    let side = if buy { "long" } else { "short" };
    if (buy && level >= entry) || (!buy && level <= entry) {
        let direction = if buy { "below" } else { "above" };
        return format!(
            "A stop-loss on a {side} sits {direction} the {} it opens at.",
            fmt_px(entry)
        );
    }
    if liquidation > 0.0 && ((buy && level <= liquidation) || (!buy && level >= liquidation)) {
        return format!(
            "The engine closes this {side} at {}, before that stop is reached.",
            fmt_px(liquidation)
        );
    }
    String::new()
}

/// A level field's name, carrying the figure drawn beside it. The number is
/// the whole reason to choose one level over another, and it is painted in the
/// label row where a reader who cannot see it hears nothing — so the field
/// says what it is worth as well as what it is.
pub fn level_label(name: String, pnl: f64) -> String {
    if pnl == 0.0 {
        return format!("{name} price");
    }
    format!("{name} price, {} at that level", fmt_pnl(pnl))
}

/// A segmented choice's name, by the rule every tab in this app already
/// follows: the button is named for the act it performs, and the one already
/// taken says so in its own name rather than only in its colour. accesskit
/// carries a toggled state for a checkbox and a switch but not for a button,
/// so the highlight is the whole answer for everyone who can see it and no
/// answer at all for anyone who cannot.
/// What a share button fills in, said in full for a reader who cannot see
/// which ticket it sits in.
///
/// The percentage on the face is the same number whether the ticket is opening
/// or closing and means two different things, so the name carries which. Left
/// at "25%" it named the fraction and never the thing it was a fraction of,
/// which was already thin and became wrong the moment there were two.
pub fn share_act(share: f64, reduce: bool) -> String {
    let of = if reduce {
        "this position"
    } else {
        "your buying power"
    };
    if share >= 1.0 {
        return format!("Set the size to all of {of}");
    }
    format!("Set the size to {}% of {of}", (share * 100.0).round())
}

/// What the requirement above it is standing on, which is the whole of the
/// difference between the two margin modes. The figure is the same either way
/// — the venue takes notional over leverage to open, whichever pocket it comes
/// out of — so a panel that printed the number and left the mode unsaid was
/// showing the identical requirement for two orders that die in different
/// places.
pub fn margin_note(cross: bool) -> String {
    if cross {
        "Cross margin: this order is backed by the whole account and goes when the account does, at the requirement drawn under the equity figure. Everything else held cross moves that line."
    } else {
        "Isolated margin: this order stands on the requirement above and on nothing else, at the maintenance this market holds. The rest of the account is untouched by it."
    }
    .to_owned()
}

/// What a market order is quoted at, said where the limit price would have
/// been typed.
///
/// A market order has no price to type, and the field's worth of space is
/// better spent on the price it is actually being quoted at than on saying
/// there is none. That price is the book's, and it is the same walk the row
/// below prints — so the two cannot disagree, and a book too thin to price the
/// size says so there rather than leaving the figure to look firm.
pub fn market_note(
    book: Option<Book>,
    size: String,
    buy: bool,
    focus: Option<SymbolRow>,
) -> String {
    let impact = book_impact(book.clone(), size.clone(), buy);
    if impact.ready {
        return format!("Crosses the spread now, at {}.", fmt_px(impact.paid));
    }
    let seed = order_price(true, String::new(), book, size, buy, focus);
    if seed > 0.0 {
        return format!(
            "Crosses the spread. No book on screen to walk, so it is quoted at the venue's last, {}.",
            fmt_px(seed)
        );
    }
    "Crosses the spread. Nothing on screen prices it yet.".to_owned()
}

/// Which price the dollars in the field are being turned into a size at.
///
/// A conversion whose rate is off screen is a number a reader has no way to
/// check, and the two rates this can be are not interchangeable: a limit that
/// has not traded, and a mid that is moving while the field is being typed.
pub fn size_note(
    usd: bool,
    market: bool,
    price: String,
    book: Option<Book>,
    focus: Option<SymbolRow>,
) -> String {
    if !usd {
        return String::new();
    }
    let at = size_price(market, price.clone(), book, focus);
    if at <= 0.0 {
        return "Nothing on screen prices this market yet, so dollars cannot be sized.".to_owned();
    }
    let source = if !market && amount(&price) > 0.0 {
        "the limit price"
    } else {
        "the market's mid"
    };
    format!("Sized at {}, {source}.", fmt_px(at))
}

/// A tab is named by what pressing it does, like every other control here, so
/// all six say `Show`; the selected state is exposed separately on the button.
pub fn interval_label(interval: String) -> String {
    format!("Show {interval} candles")
}

/// The widths the chart offers, coarsest first, which is the order it opens
/// them in. All six are quoted by both venues: Hyperliquid's `interval_secs`
/// takes each one and Lighter's `RESOLUTIONS` lists all six among its eight,
/// so one ladder serves the terminal whichever exchange it is reading.
const WIDTHS: [&str; 6] = ["1d", "4h", "1h", "15m", "5m", "1m"];

/// What a width has to carry before the chart is worth opening on it.
///
/// This is the chart's own window rather than a number chosen here: it opens
/// showing its last `DEFAULT_BARS` candles and draws a 20- and 60-period
/// average across them, so a width holding fewer than that opens on a plot
/// that is mostly empty with a long average that never begins. A market listed
/// last week has four daily bars and four hundred hourly ones, and the hourly
/// chart is the one that shows it.
const ENOUGH_BARS: i64 = ducktape_ui::ui::candle_chart::DEFAULT_BARS as i64;

/// The next width down when this one is too thin to open on, and the width
/// itself when it is not — which is also the answer at `1m`, where there is
/// nothing finer to step to. A market with three bars at every width settles
/// there and draws its three.
///
/// Only an automatic open walks this ladder. A width the reader pressed is
/// theirs, and a chart that answered the press by showing a different width
/// would be refusing it.
pub fn finer_interval(interval: String, bars: i64) -> String {
    if bars >= ENOUGH_BARS {
        return interval;
    }
    let next = WIDTHS
        .iter()
        .position(|width| *width == interval)
        .map(|at| at + 1)
        .and_then(|at| WIDTHS.get(at));
    next.map_or(interval, |width| (*width).to_owned())
}

/// A folded-away pane's toggle, by the same rule as the interval tabs: the name
/// a reader hears is the act the button performs. It says "hide" while the pane
/// is open because that is what pressing it does — a control that announced the
/// pane's current state would leave a reader guessing at the verb.
pub fn pane_label(pane: String, open: bool) -> String {
    let act = if open { "Hide" } else { "Show" };
    format!("{act} the {} pane", pane.to_lowercase())
}

/// A page tab by the same rule. The tab draws its page's name in capitals
/// because it is a heading for the surface it opens; the selected state is
/// exposed separately on the button.
pub fn page_label(page: String) -> String {
    format!("Show the {} page", page.to_lowercase())
}

/// A hovered candle's figures, one per cell of the crosshair readout. The demo
/// tape walks a sine, so a test can only name the candle under the crosshair by
/// asking the fixture for it; transcribed numbers would check the arithmetic
/// against a copy of itself. They take the candle rather than a hover, because
/// the readout is drawn inside `some(hit)`: a figure answered for no hover
/// would be answering a test and nothing else.
pub fn hit_open(hit: CandleHit) -> f64 {
    hit.open
}

pub fn hit_high(hit: CandleHit) -> f64 {
    hit.high
}

pub fn hit_low(hit: CandleHit) -> f64 {
    hit.low
}

pub fn hit_close(hit: CandleHit) -> f64 {
    hit.close
}

pub fn hit_volume(hit: CandleHit) -> f64 {
    hit.volume
}

/// A resting order names its side, its size and the price it waits at.
/// A market row names its ticker and what the day has done to it. The rail
/// draws both beside the name, and a reader who cannot see those two columns
/// was being asked to choose a market from its ticker alone.
///
/// Called once per `MarketRow` and nowhere else, which is what makes it the
/// market list's row counter — the arrangement `fill_label` already has.
/// `markets_stay_memoized_performance_contract` asserts the cold count is a
/// whole multiple of the rows on screen, so a second caller appearing fails
/// the contract rather than quietly skewing it.
/// A market row read out. The group is named on every row of a grouped list,
/// not only on the one under the header: a header is a heading, and a reader
/// moving row by row does not carry it down the list with them. On a venue
/// that lists one flat universe there is no group to name and the label says
/// what it always said.
pub fn market_label(market: SymbolRow) -> String {
    #[cfg(test)]
    count(&MARKET_ROWS);
    let name = &market.name;
    let price = fmt_px(market.price);
    let change = fmt_pct(market.change_pct);
    if market.category.is_empty() {
        return format!("{name} at {price}, {change} today");
    }
    format!(
        "{name} at {price}, {change} today, {} market settled in {}",
        market.category, market.collateral
    )
}

pub fn order_label(order: Order) -> String {
    let side = if order.buy { "buy" } else { "sell" };
    format!(
        "{} {side} {} at {}",
        order.coin,
        fmt_size(order.size),
        fmt_px(order.price)
    )
}

/// What pressing the row itself would do, which is no longer only "go there".
///
/// The row loads the order back into the ticket, so the name says that rather
/// than naming the order and leaving the act to be guessed at — and it says it
/// beside a CANCEL on the same row whose name is the same order with a
/// different verb, which is the pair a reader has to tell apart.
pub fn order_pick_label(order: Order) -> String {
    format!("Load this {} into the ticket", order_label(order))
}

/// What CANCEL on this row would do, for whoever cannot see which row it is on.
///
/// It names the order rather than the act, because "cancel" is what every one
/// of these buttons says and the row is the whole of what tells them apart.
pub fn order_cancel_label(order: Order) -> String {
    format!("Cancel this {}", order_label(order))
}

// One count per row built, on the thread that built it — which is what the
// `lazy` boundaries in `frame_probe` are held down with: a redraw that rebuilds
// a memoized row shows up in these counters and nowhere else.
//
// Per THREAD, not per process. libtest runs the probes concurrently and every
// one of them builds the same 200-row screen, so a global counter reads its
// neighbours' cold builds as its own — which is how the memo contract came to
// report 85 rows rebuilt for a fill whose row rebuilt exactly once. The memo
// parking lot in `ui-lang-runtime` is thread-local for the same reason.
#[cfg(test)]
thread_local! {
    /// Fill rows built: `fill_label` is called once per `FillRow` and nowhere
    /// else.
    pub(crate) static FILL_LABELS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Market rows built: `market_label` is called once per `MarketRow` and
    /// nowhere else.
    pub(crate) static MARKET_ROWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Adds one to a per-thread row counter. Reading one is `Cell::take`.
#[cfg(test)]
pub(crate) fn count(counter: &'static std::thread::LocalKey<std::cell::Cell<usize>>) {
    counter.with(|rows| rows.set(rows.get() + 1));
}

/// A fill names what it did, how much of it, and where. Choosing between the
/// size and the realized PnL was choosing between two cells the row draws side
/// by side: a closing fill announced what it made and left a reader unable to
/// hear whether it closed the position or a quarter of it. The PnL is said only
/// when there is one, because the row draws an em dash where there is not.
pub fn fill_label(fill: Fill) -> String {
    #[cfg(test)]
    count(&FILL_LABELS);
    let side = if fill.buy { "bought" } else { "sold" };
    let closed = if fill.closed_pnl == 0.0 {
        String::new()
    } else {
        format!(", realized {}", fmt_pnl(fill.closed_pnl))
    };
    format!(
        "{} {side} {} at {}{closed}",
        fill.coin,
        fmt_size(fill.size),
        fmt_px(fill.price)
    )
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
pub fn now_seconds() -> i64 {
    now_ms() / 1_000
}

pub fn fmt_age(ts: i64, now: i64) -> String {
    let seconds = now - ts;
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
            size_decimals: 5,
            selected: true,
            ..Default::default()
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
            size_decimals: 4,
            selected: false,
            ..Default::default()
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
            size_decimals: 2,
            selected: false,
            ..Default::default()
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
            // A coin worth a fraction of a cent trades in whole units.
            size_decimals: 0,
            selected: false,
            ..Default::default()
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
            size_decimals: 2,
            selected: false,
            ..Default::default()
        });
    }
    rows
}

/// A universe with more than one dex in it, which is what Hyperliquid's now
/// is: the exchange's own perps, and builder-deployed markets under the name
/// of whoever deployed them.
///
/// The shape is the live one rather than an invented one. Read on the day this
/// was written, `perpDexs` answered with `null` in the first slot and nine
/// builder deployments after it, `xyz` ("XYZ") listing 94 live markets against
/// USDC collateral and `hyna` ("HyENA") 18 against USDe. Those two are the two
/// here, because the pair is what the rail has to get right: a second group at
/// all, and a group that does not settle in dollars.
///
/// The tickers are the live ones too. A builder market is named `dex:SYMBOL`
/// on the wire and that string is its whole identity — the book, the tape and
/// the candle requests take it verbatim — so a fixture that shortened it to
/// `NVDA` would be testing a market Hyperliquid does not list.
pub fn demo_symbols_categorized() -> Vec<SymbolRow> {
    let canonical = demo_symbols().into_iter().map(|row| SymbolRow {
        category: HL_CANONICAL.to_owned(),
        collateral: HL_COLLATERAL.to_owned(),
        ..row
    });
    let builder = |name: &str, category: &str, collateral: &str, price: f64, prev: f64| SymbolRow {
        name: name.to_owned(),
        category: category.to_owned(),
        collateral: collateral.to_owned(),
        price,
        change_pct: change_pct(price, prev),
        prev,
        volume: 26_000_000.0,
        funding_pct: 0.00053,
        leverage: 20.0,
        open_interest: 6_252.0,
        maintenance: maintenance_fraction(20.0),
        size_decimals: 3,
        ..Default::default()
    };
    canonical
        .chain([
            builder("xyz:NVDA", "XYZ", HL_COLLATERAL, 224.51, 223.86),
            builder("xyz:SP500", "XYZ", HL_COLLATERAL, 6_970.51, 6_961.02),
            builder("hyna:HYPE", "HyENA", "USDe", 38.42, 37.90),
        ])
        .collect()
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

/// A beat that carries a print, which `demo_tick_at` does not.
///
/// The tape is the one panel a beat *prepends* to, so every row on it draws a
/// different print afterwards than it drew before — and that is the only way a
/// memoized row can go stale. Driving that needs a tick with a trade on it.
pub fn demo_tick_printing(price: f64, size: f64) -> MarketTick {
    MarketTick {
        trades: vec![Trade {
            ts: 1_786_117_900,
            price,
            size,
            buy: true,
            sweep: 1,
            tid: 900,
        }],
        ..demo_tick_at(price)
    }
}

/// The chart reporting that the view has been taken back to the oldest bar it
/// holds, which is the one signal that asks for history. For driving the
/// handler that answers it.
pub fn demo_chart_older() -> ChartSignal {
    ChartSignal {
        hover: None,
        older: true,
    }
}

pub fn demo_hover() -> CandleHit {
    let tape = demo_candles();
    let candles = lock(&tape.candles);
    let candle = &candles[60];
    CandleHit {
        index: 60,
        ts: candle.ts,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
    }
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

/// Fills and resting orders, including fills that arrived lit, so the wash a
/// just-printed fill wears is drawn rather than only asserted in a unit test.
pub fn demo_fills() -> Vec<Fill> {
    let fill = |tid: i64, price: f64, size: f64, buy: bool, closed_pnl: f64, hot: bool| Fill {
        coin: "BTC".to_owned(),
        // Inside the candle window `demo_candles` builds, so each lands on
        // its own candle rather than piling onto the last one.
        ts: 1_786_110_000 + (7 - tid) * 900,
        price,
        size,
        buy,
        closed_pnl,
        hot,
        tid,
    };
    vec![
        fill(1, 64_010.0, 0.25, false, 1_240.0, true),
        fill(2, 63_940.0, 0.50, true, 0.0, true),
        fill(3, 63_880.0, 0.75, true, 0.0, false),
    ]
}

/// A trading history filling the list to its cap, every fill different from
/// every other one.
///
/// The rows are `lazy`, cached and parked under `tid`, so a fixture that
/// repeats a fill measures a list of repeats: it shares one cache entry
/// between every row that repeats it, and parks one subtree where a real list
/// parks that many. Nothing here repeats — not the id, not the timestamp, not
/// the money, and not the coin, whose `String` the row's hash walks.
pub fn demo_fills_many(count: i64) -> Vec<Fill> {
    const COINS: [&str; 8] = ["BTC", "ETH", "SOL", "HYPE", "AVAX", "LINK", "ARB", "SUI"];
    (0..count.max(0))
        .map(|step| {
            let coin = COINS[step as usize % COINS.len()];
            // Prices that belong to their market, so a row formats the way the
            // real one does — six figures for BTC, cents for the small caps.
            let base = 64_000.0 / (1.0 + (step % COINS.len() as i64) as f64 * 3.7);
            Fill {
                coin: coin.to_owned(),
                ts: 1_786_110_000 - step * 37,
                price: base + step as f64 * 0.31,
                size: 0.05 + step as f64 * 0.017,
                buy: step % 3 != 0,
                closed_pnl: if step % 4 == 0 {
                    0.0
                } else {
                    (step as f64 * 13.7) % 900.0 - 400.0
                },
                hot: step == 0,
                tid: 4_000_000 + step,
            }
        })
        .collect()
}

/// Fills that opened a position and closed nothing — the ordinary state of an
/// account that has not round-tripped yet, and the one a win rate cannot be
/// computed for. Derived from the fixture above rather than typed again, so
/// the two cannot drift into disagreeing about what an opening fill is.
pub fn demo_fills_opening() -> Vec<Fill> {
    demo_fills()
        .into_iter()
        .filter(|fill| fill.closed_pnl == 0.0)
        .collect()
}

pub fn demo_orders() -> Vec<Order> {
    vec![
        Order {
            oid: 71_234_567_890,
            coin: "BTC".to_owned(),
            buy: true,
            price: 63_600.0,
            size: 1.5,
            ts: now_ms() / 1_000 - 7_200,
        },
        Order {
            oid: 71_234_567_891,
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
///
/// `BOOK_DEPTH` levels a side, because that is what both venues publish and a
/// fixture shallower than the feed is a fixture that cannot draw the case the
/// panel has to survive. It was three a side for a long time, and three fit in
/// any column ever drawn: the book took the height of every list under it at
/// the window's own minimum and no test could see it, and the wide terminal
/// drew empty panel below the bids.
///
/// The touch is unchanged from that fixture — 1.4 on the best bid, 1.8 on the
/// best ask — so what crossing a small order costs is what it always cost. Each
/// side thickens by its own step away from the touch, and the two steps differ
/// so that a walk which took the wrong side gets a different answer rather than
/// the right one by symmetry.
pub fn demo_book_ticked(mid: f64, tick: f64) -> Book {
    let side = |away: f64, first: f64, step: f64| {
        let mut total = 0.0;
        let mut levels = (1..=BOOK_DEPTH)
            .map(|rank| {
                let size = first + step * (rank - 1) as f64;
                total += size;
                Level {
                    price: mid + away * tick * rank as f64,
                    size,
                    total,
                    bar: total,
                }
            })
            .collect::<Vec<Level>>();
        // The widest bar is the deepest level's running total, which is only
        // known once the whole side is built.
        for level in &mut levels {
            level.bar = level.bar / total * BOOK_BAR_WIDTH;
        }
        levels
    };
    let mut asks = side(1.0, 1.8, 0.4);
    // Reversed, as the feed leaves them: the best ask sits last, against the
    // spread, so the panel walks both lists top to bottom.
    asks.reverse();
    Book {
        bids: side(-1.0, 1.4, 0.7),
        asks,
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

/// What a reader who cannot see the chart is told it is.
///
/// A canvas has no text to read, so this sentence is the whole of the chart
/// for that reader, and both halves of it are claims that can be false. The
/// app draws two exchanges through one chart, so a fixed venue name describes
/// the other one half the time; and "with this account's fills marked" over a
/// chart carrying no marks sends someone hunting it for arrows that are not
/// there — which is every Lighter chart, since that venue serves no fills to
/// an address, and every chart on either venue before the first fill lands.
///
/// So both halves are read off what the chart is actually given: the venue it
/// was told to draw, and whether any markers survived the filter.
fn chart_label(venue: Venue, marked: bool) -> String {
    let name = venue_name(venue);
    if marked {
        format!("{name} candlestick chart with this account's fills marked")
    } else {
        format!("{name} candlestick chart")
    }
}

/// The chart for the selected market, with this account's levels drawn across
/// it and its fills marked where there are any. It repaints on its own beat,
/// so candles the feed merges into the tape show without an app message per
/// tick.
///
/// It draws whichever venue is on screen, so it is told which one — see
/// `chart_label`.
pub fn chart(
    venue: Venue,
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
    let marks = fill_markers(fills, coin, palette);
    let label = chart_label(venue, !marks.is_empty());
    let chart = candle_chart_shared(tape.candles.clone(), &theme)
        .precision(scale)
        .height(Length::Fill)
        .live(BEAT)
        .moving_averages([20, 60])
        .price_lines(position_lines(positions, coin, palette))
        .price_lines(order_lines(orders, coin, palette))
        .markers(marks)
        .on_hover(|hover| ChartSignal {
            hover,
            older: false,
        })
        .on_reach_start(|hover| ChartSignal { hover, older: true });
    accessible(chart, StableId::new("trading-chart"), Role::Image)
        .label(label)
        .logical_id("trading-chart")
        .into()
}

/// Traffic in the shape one connection carries, for probes that need a beat
/// larger than the fixtures. A `MarketTick` holds private fields on purpose —
/// the app folds one rather than building one — so the way to a real beat is
/// to drive the real reader over real-shaped JSON, which is also the only way
/// to price the reader itself.
#[cfg(all(test, not(debug_assertions)))]
pub(crate) mod probe {
    use super::*;

    pub(crate) fn market(index: usize) -> String {
        if index == 0 {
            "BTC".to_owned()
        } else {
            format!("SYM{index}")
        }
    }

    /// Every mid on the exchange, as decimal strings. This is the whole
    /// `allMids` payload, and it arrives on every beat whatever moved.
    pub(crate) fn all_mids(markets: usize, mid: f64) -> Value {
        let mids: serde_json::Map<String, Value> = (0..markets)
            .map(|index| {
                (
                    market(index),
                    Value::String(format!("{:.4}", mid + index as f64)),
                )
            })
            .collect();
        json!({ "mids": mids })
    }

    pub(crate) fn l2_book(coin: &str, depth: usize, mid: f64) -> Value {
        let side = |sign: f64| -> Vec<Value> {
            (1..=depth)
                .map(|step| {
                    json!({
                        "px": format!("{:.1}", mid + sign * step as f64),
                        "sz": "1.7",
                        "n": 3,
                    })
                })
                .collect()
        };
        json!({
            "coin": coin,
            "time": 1_786_117_888_000_i64,
            "levels": [side(-1.0), side(1.0)],
        })
    }

    pub(crate) fn prints(coin: &str, count: usize, mid: f64) -> Value {
        Value::Array(
            (0..count)
                .map(|step| {
                    json!({
                        "coin": coin,
                        "side": if step % 3 == 1 { "A" } else { "B" },
                        "px": format!("{:.1}", mid + (step % 3) as f64),
                        "sz": "0.42",
                        "time": 1_786_117_888_000_i64 + step as i64,
                        "hash": format!("0x{step:064x}"),
                        "tid": 900_000 + step as i64,
                    })
                })
                .collect(),
        )
    }

    pub(crate) fn context(coin: &str, mid: f64) -> Value {
        json!({
            "coin": coin,
            "ctx": {
                "markPx": format!("{mid:.1}"),
                "prevDayPx": format!("{:.1}", mid - 400.0),
                "dayNtlVlm": "1284000000.0",
                "funding": "0.0000125",
                "openInterest": "24000.0",
            },
        })
    }

    /// Drives the real reader through one beat's traffic and returns the beat
    /// it publishes.
    pub(crate) fn beat(markets: usize, depth: usize, count: usize, mid: f64) -> MarketTick {
        let tape = tape_focus(tape_new(), "BTC".to_owned(), "1m".to_owned());
        let mut read = market_reader(tape);
        let (mids, book) = (all_mids(markets, mid), l2_book("BTC", depth, mid));
        let (tape_prints, ctx) = (prints("BTC", count, mid), context("BTC", mid));
        read(Event::Beat);
        read(Event::Payload("allMids", &mids));
        read(Event::Payload("l2Book", &book));
        read(Event::Payload("activeAssetCtx", &ctx));
        read(Event::Payload("trades", &tape_prints));
        read(Event::Pong(42));
        read(Event::Beat).expect("a beat that carried traffic publishes a tick")
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn hyperliquid_market_feed_has_a_finite_handoff() {
        let receiver = hl_market_feed(Chain::Mainnet, tape_new());
        assert_eq!(receiver.capacity(), Some(FEED_BUFFER_CAPACITY));
    }

    #[test]
    fn hyperliquid_fill_feed_has_a_finite_handoff() {
        let receiver = hl_fill_feed(Chain::Mainnet, String::new());
        assert_eq!(receiver.capacity(), Some(FEED_BUFFER_CAPACITY));
    }

    #[test]
    fn a_full_feed_buffer_blocks_until_the_receiver_is_dropped() {
        let (sender, receiver) = feed_channel();
        for item in 0..FEED_BUFFER_CAPACITY {
            sender.send_blocking(item).expect("receiver is open");
        }
        assert!(sender.is_full());

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let worker_barrier = barrier.clone();
        let (done, blocked) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            done.send(sender.send_blocking(FEED_BUFFER_CAPACITY).is_err())
                .expect("test is listening");
        });
        barrier.wait();

        assert_eq!(
            blocked.recv_timeout(Duration::from_millis(100)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout),
            "the full buffer must backpressure its producer"
        );
        drop(receiver);
        assert_eq!(
            blocked.recv_timeout(Duration::from_secs(1)),
            Ok(true),
            "dropping the UI receiver must wake the blocked producer"
        );
        worker.join().expect("producer exits");
    }

    /// The whole order path against the live test deployment: place, see it
    /// resting, cancel it, see it gone.
    ///
    /// Testnet only, and structurally so — `Chain::Testnet` is the only chain
    /// this builds, so there is no argument to get wrong. The account and its
    /// approved agent key come from the environment because they are the
    /// owner's: enrolment needs a signature from the wallet that owns the
    /// account, which this app will never hold. Until those are set this test
    /// says what is missing and stops, rather than passing quietly on nothing.
    ///
    /// The order rests far from the market on purpose — a bid at a fraction of
    /// the mark cannot fill, so the round trip ends with the book exactly as it
    /// started.
    #[test]
    #[ignore = "hits Hyperliquid testnet with a real order; needs ICE_HL_TESTNET_ACCOUNT and ICE_HL_TESTNET_AGENT_KEY"]
    fn the_order_path_places_rests_and_cancels_on_the_test_deployment() {
        let (Ok(account), Ok(secret)) = (
            std::env::var("ICE_HL_TESTNET_ACCOUNT"),
            std::env::var("ICE_HL_TESTNET_AGENT_KEY"),
        ) else {
            panic!(
                "set ICE_HL_TESTNET_ACCOUNT to the funded testnet address and \
                 ICE_HL_TESTNET_AGENT_KEY to the hex secret of an agent key that \
                 address has approved — see the enrolment checklist in README.md"
            );
        };
        open_the_wire();
        let bytes: [u8; 32] = hex::decode(secret.trim_start_matches("0x"))
            .expect("a hex agent key")
            .try_into()
            .expect("32 bytes");
        let wallet = Wallet::from_secret(&bytes).expect("an agent key");

        smol::block_on(async {
            let markets = hl_symbols(Chain::Testnet, HL_CANONICAL)
                .await
                .expect("the testnet universe");
            let market = markets
                .iter()
                .find(|row| row.name == "BTC")
                .expect("testnet lists BTC")
                .clone();

            // A tenth of the mark: a bid nothing will cross.
            let done = hl_place(
                Chain::Testnet,
                &wallet,
                &market,
                &[signing::Order {
                    asset: market.asset,
                    buy: true,
                    price: (market.price / 10.0).round(),
                    size: 0.001,
                    reduce_only: false,
                    kind: signing::Kind::Limit(signing::Tif::Gtc),
                }],
                signing::Grouping::Na,
            )
            .await
            .expect("the order is accepted");
            let oid = done.resting;
            assert_ne!(
                oid, 0,
                "a bid this far under the book rests, it does not fill"
            );
            assert_eq!(done.filled, 0.0, "and nothing of it crossed");

            let open = hl_orders(Chain::Testnet, account.clone())
                .await
                .expect("open orders");
            assert!(
                open.iter().any(|order| order.oid == oid),
                "the order the venue said it rested is the one it lists back"
            );

            hl_cancel(Chain::Testnet, &wallet, &market, oid)
                .await
                .expect("the cancel is accepted");

            let after = hl_orders(Chain::Testnet, account)
                .await
                .expect("open orders");
            assert!(
                !after.iter().any(|order| order.oid == oid),
                "a cancel the venue accepted is an order it stops listing"
            );
        });
    }

    /// The venue answers a refusal with HTTP 200, so "the request worked" and
    /// "the order rested" are different questions. Reading only the outer
    /// status reports a rejected order as placed, which is the one failure on
    /// this path that costs money by being quiet.
    #[test]
    fn a_refusal_is_read_as_a_refusal_however_the_venue_spells_it() {
        // Rested: the id comes back and is what a cancel later names.
        let resting = json!({
            "status": "ok",
            "response": { "type": "order", "data": { "statuses": [
                { "resting": { "oid": 77_665_544 } }
            ]}},
        });
        assert_eq!(
            placed(&resting).expect("a resting order"),
            Placed {
                resting: 77_665_544,
                filled: 0.0,
                at: 0.0
            },
        );

        // Refused outright, at the top level.
        let refused = json!({ "status": "err", "response": "Insufficient margin to place order." });
        let said = placed(&refused).expect_err("a refusal is not a placement");
        assert_eq!(
            said.message, "Insufficient margin to place order.",
            "the venue's own sentence, not a sentence about the venue"
        );

        // Refused *inside* an otherwise ok response, which is the shape that
        // reads as success to anything looking at `status` alone.
        let partial = json!({
            "status": "ok",
            "response": { "type": "order", "data": { "statuses": [
                { "error": "Order price cannot be more than 95% away from the reference price" }
            ]}},
        });
        let said = placed(&partial).expect_err("an error inside an ok is still an error");
        assert!(said.message.contains("95%"), "{}", said.message);

        // Accepted and said nothing about the order: not a success to report.
        let silent =
            json!({ "status": "ok", "response": { "type": "order", "data": { "statuses": [] }}});
        assert!(placed(&silent).is_err(), "silence is not a placement");

        // Filled immediately: no resting id, and no error either.
        let filled = json!({
            "status": "ok",
            "response": { "type": "order", "data": { "statuses": [
                { "filled": { "totalSz": "1.0", "avgPx": "64000.0", "oid": 5 } }
            ]}},
        });
        assert_eq!(
            placed(&filled).expect("a fill is not a failure"),
            Placed {
                resting: 0,
                filled: 1.0,
                at: 64_000.0
            },
            "a filled order rests under no id, and the amount it filled is the \
             venue's own number rather than the size that was asked for",
        );

        // The shape this app used to report at the size that was typed: an
        // immediate-or-cancel order for ten that crossed two. The venue says
        // two, and two is what a reader has to be told.
        let partial_fill = json!({
            "status": "ok",
            "response": { "type": "order", "data": { "statuses": [
                { "filled": { "totalSz": "2.0", "avgPx": "63999.5", "oid": 9 } }
            ]}},
        });
        assert_eq!(
            placed(&partial_fill).expect("a partial fill is not a failure"),
            Placed {
                resting: 0,
                filled: 2.0,
                at: 63_999.5
            },
        );

        // And the shape where both happen at once: part crossed, the rest
        // rests, and the answer carries an id *and* an amount.
        let both = json!({
            "status": "ok",
            "response": { "type": "order", "data": { "statuses": [
                { "filled": { "totalSz": "2.0", "avgPx": "63999.5", "oid": 9 } },
                { "resting": { "oid": 77_665_545 } }
            ]}},
        });
        assert_eq!(
            placed(&both).expect("a part-filled order is not a failure"),
            Placed {
                resting: 77_665_545,
                filled: 2.0,
                at: 63_999.5
            },
        );
    }

    /// The index an order is sent with is the venue's, and the row order on
    /// screen is the app's. Reading the index off the sorted list names
    /// whichever market happened to be as busy as the one you meant — an order
    /// for the wrong asset, with every figure on screen still correct.
    #[test]
    fn the_asset_index_survives_the_sort_and_the_delisting_filter() {
        let response = json!([
            { "universe": [
                { "name": "AAA", "szDecimals": 2, "maxLeverage": 10 },
                { "name": "GONE", "szDecimals": 2, "maxLeverage": 10, "isDelisted": true },
                { "name": "BBB", "szDecimals": 2, "maxLeverage": 10 }
            ]},
            [
                { "markPx": "1.0", "prevDayPx": "1.0", "dayNtlVlm": "5" },
                { "markPx": "1.0", "prevDayPx": "1.0", "dayNtlVlm": "9" },
                { "markPx": "1.0", "prevDayPx": "1.0", "dayNtlVlm": "100" }
            ],
        ]);
        let rows = parse_symbols(&response, HL_CANONICAL, HL_COLLATERAL);
        let index = |name: &str| {
            rows.iter()
                .find(|row| row.name == name)
                .unwrap_or_else(|| panic!("{name} is listed"))
                .asset
        };
        // BBB sorts first on volume and is the venue's third asset.
        assert_eq!(rows[0].name, "BBB", "the rows are sorted by volume");
        assert_eq!(
            index("BBB"),
            2,
            "and carry the venue's index, not the row's"
        );
        assert_eq!(index("AAA"), 0);
        assert_eq!(
            rows.len(),
            2,
            "the delisted market is dropped but still occupied its slot"
        );
    }

    /// A beat carries a price, not an identity. Dropping the index on one
    /// would leave every market pointing at asset zero, which is a real market.
    #[test]
    fn a_streamed_beat_does_not_move_which_asset_a_row_is() {
        let held = SymbolRow {
            name: "ETH".into(),
            price: 3_500.0,
            asset: 4,
            ..Default::default()
        };
        // A context off the socket, which carries no index — `parse_context`
        // zeroes it, and zero is a real market.
        let beat = MarketTick {
            context: Some(parse_context(
                "ETH".into(),
                &json!({ "markPx": "3600.0", "prevDayPx": "3500.0" }),
            )),
            ..MarketTick::default()
        };
        let after = apply_feed(vec![held], beat);
        assert_eq!(after[0].price, 3_600.0, "the price is the beat's");
        assert_eq!(after[0].asset, 4, "the index is not");
    }
    use super::*;

    /// A builder-deployed market's name carries a colon, and the tape holds
    /// its market and interval in one colon-joined string. Split from the
    /// left, `xyz:NVDA:1m` reads as the coin `xyz` at the interval `NVDA:1m`,
    /// and every subscription the feed then holds is for a market that does
    /// not exist.
    ///
    /// Verified against the live exchange: `l2Book` and `candleSnapshot`
    /// answer for the coin `xyz:NVDA` with no `dex` parameter at all, and
    /// answer `null` for a bare `NVDA` — the qualified string is the whole of
    /// the identity, so nothing downstream may take it apart.
    #[test]
    fn a_dex_qualified_market_survives_the_tape_focus_round_trip() {
        let tape = tape_focus(tape_new(), "xyz:NVDA".to_owned(), "1m".to_owned());
        assert_eq!(
            tape.focus(),
            Some(("xyz:NVDA".to_owned(), "1m".to_owned())),
            "the dex prefix belongs to the market, not to the interval"
        );
        let plain = tape_focus(tape_new(), "BTC".to_owned(), "15m".to_owned());
        assert_eq!(plain.focus(), Some(("BTC".to_owned(), "15m".to_owned())));
    }

    /// The same claim where it is actually felt. The book, the tape and the
    /// context are each turned away unless they name the market the tape is
    /// pointed at, so a focus that lost the dex prefix does not fail loudly —
    /// it publishes a beat with no book, no prints and no context, and the
    /// panels sit empty over a market the rail says is selected.
    #[test]
    fn a_dex_qualified_market_is_fed_its_own_book_and_prints() {
        let tape = tape_focus(tape_new(), "xyz:NVDA".to_owned(), "1m".to_owned());
        let mut read = market_reader(tape);
        let book = json!({
            "coin": "xyz:NVDA",
            "levels": [
                [{ "px": "224.49", "sz": "21.9", "n": 4 }],
                [{ "px": "224.51", "sz": "0.2", "n": 1 }],
            ]
        });
        let tape_prints = json!([
            { "coin": "xyz:NVDA", "px": "224.50", "sz": "1.0", "side": "B", "time": 1_786_265_690_665_i64, "tid": 1, "hash": "a" },
            { "coin": "xyz:NVDA", "px": "224.51", "sz": "2.0", "side": "A", "time": 1_786_265_690_777_i64, "tid": 2, "hash": "b" },
        ]);
        let ctx = json!({
            "coin": "xyz:NVDA",
            "ctx": { "markPx": "224.51", "prevDayPx": "223.86", "dayNtlVlm": "26553442.8", "funding": "0.0000053" }
        });
        read(Event::Beat);
        read(Event::Payload("l2Book", &book));
        read(Event::Payload("activeAssetCtx", &ctx));
        read(Event::Payload("trades", &tape_prints));
        let tick = read(Event::Beat).expect("a beat that carried traffic publishes a tick");
        assert!(
            tick.book.is_some(),
            "the book names the market it was asked for"
        );
        assert_eq!(tick.trades.len(), 2, "so do the prints");
        assert_eq!(
            tick.context.map(|row| row.name),
            Some("xyz:NVDA".to_owned()),
            "and so does the context that re-prices the row"
        );
    }

    /// `allMids` answers for one dex at a time, so a rail listing several of
    /// them is fed one message per dex per beat. Assigned rather than merged,
    /// the last message would be the only prices the beat carried and every
    /// other group's price column would blink between a price and nothing.
    #[test]
    fn mids_from_several_dexs_land_in_one_beat() {
        let tape = tape_focus(tape_new(), "BTC".to_owned(), "1m".to_owned());
        let mut read = market_reader(tape);
        let canonical = json!({ "mids": { "BTC": "64000.0" } });
        let builder = json!({ "mids": { "xyz:NVDA": "224.51" } });
        read(Event::Beat);
        read(Event::Payload("allMids", &canonical));
        read(Event::Payload("allMids", &builder));
        let tick = read(Event::Beat).expect("a beat that carried traffic publishes a tick");
        assert_eq!(tick.mids.get("BTC"), Some(&64_000.0));
        assert_eq!(
            tick.mids.get("xyz:NVDA"),
            Some(&224.51),
            "a second dex's prices do not replace the first's"
        );
    }

    /// `perpDexs` answers with `null` in the first slot for the exchange's own
    /// perps and one object per builder deployment after it. The `null` is not
    /// a dex to ask for by name — it is the list read with no `dex` at all —
    /// and a dex that states no full name is headed with its short one rather
    /// than with a blank.
    #[test]
    fn the_canonical_list_is_not_one_of_the_builder_dexs() {
        let response = json!([
            null,
            { "name": "xyz", "fullName": "XYZ" },
            { "name": "hyna", "fullName": "HyENA" },
            { "name": "abcd" },
        ]);
        let dexs = parse_perp_dexs(&response);
        let named: Vec<(&str, &str)> = dexs
            .iter()
            .map(|dex| (dex.name.as_str(), dex.label.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![("xyz", "XYZ"), ("hyna", "HyENA"), ("abcd", "ABCD")],
            "the canonical slot is not asked for by name"
        );
    }

    /// A search that empties a group must not leave its header behind, and one
    /// that removes a group's first row must move the header onto what is
    /// left. Both are why the heading is written by the filter rather than
    /// stamped when the universe is read.
    #[test]
    fn a_filtered_list_re_heads_the_groups_it_leaves() {
        let rows = demo_symbols_categorized();
        let all = filter_symbols(rows.clone(), String::new(), "BTC".to_owned());
        let headed: Vec<&str> = all
            .iter()
            .filter(|row| row.heading)
            .map(|row| row.category.as_str())
            .collect();
        assert_eq!(headed, vec![HL_CANONICAL, "XYZ", "HyENA"]);

        // `xyz:SP500` is the second row of its group, so the header has to
        // move onto it once the first is filtered out.
        let narrowed = filter_symbols(rows, "SP500".to_owned(), "BTC".to_owned());
        assert_eq!(narrowed.len(), 1);
        assert!(
            narrowed[0].heading,
            "what is left of a group still heads it"
        );
        assert_eq!(narrowed[0].category, "XYZ");
    }

    /// One group is not a categorization. A venue that lists one flat universe
    /// names no group at all, and the rail then draws the list it always drew.
    #[test]
    fn a_flat_universe_is_not_headed() {
        let rows = filter_symbols(demo_symbols(), String::new(), "BTC".to_owned());
        assert!(
            rows.iter()
                .all(|row| !row.heading && row.category.is_empty()),
            "an uncategorized list has nothing to head"
        );
    }
    // The other venue's fixture universe, which lives with the parsers that
    // built it out of the payloads that venue answered.
    use crate::lighter::demo_symbols_lighter;

    /// Median nanoseconds per call over `rounds` batches. Batched because the
    /// cheap end of this boundary is faster than the clock reads.
    #[cfg(not(debug_assertions))]
    fn per_call(batch: usize, rounds: usize, mut call: impl FnMut()) -> f64 {
        for _ in 0..8 {
            call();
        }
        let mut samples: Vec<u128> = (0..rounds)
            .map(|_| {
                let started = std::time::Instant::now();
                for _ in 0..batch {
                    call();
                }
                started.elapsed().as_nanos()
            })
            .collect();
        samples.sort_unstable();
        samples[rounds / 2] as f64 / batch as f64
    }

    /// What one connection's traffic costs before the app ever sees it.
    ///
    ///     cargo test --release -p trading-example -- --ignored --nocapture feed_cost
    #[test]
    #[ignore = "feed-cost probe, run explicitly: prints per-message costs, asserts nothing"]
    #[cfg(not(debug_assertions))]
    fn feed_cost() {
        const MARKETS: usize = 200;
        const ROUNDS: usize = 40;
        let mid = 64_000.0;

        let mids = probe::all_mids(MARKETS, mid);
        let book = probe::l2_book("BTC", BOOK_DEPTH, mid);
        let trades = probe::prints("BTC", 8, mid);
        let ctx = probe::context("BTC", mid);
        let mids_text = mids.to_string();
        let book_text = book.to_string();

        eprintln!("\nhyperliquid feed, {MARKETS} markets, book depth {BOOK_DEPTH}");
        eprintln!(
            "{:<34} {:>7} bytes",
            "allMids payload on the wire",
            mids_text.len()
        );
        eprintln!(
            "{:<34} {:>9.0}ns  serde_json::from_str",
            "allMids parse to Value",
            per_call(200, ROUNDS, || {
                std::hint::black_box(serde_json::from_str::<Value>(&mids_text).unwrap());
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  serde_json::from_str",
            "l2Book parse to Value",
            per_call(2_000, ROUNDS, || {
                std::hint::black_box(serde_json::from_str::<Value>(&book_text).unwrap());
            })
        );

        // What the reader makes of each message, given the Value.
        let tape = tape_focus(tape_new(), "BTC".to_owned(), "1m".to_owned());
        let mut read = market_reader(tape.clone());
        read(Event::Beat);
        eprintln!(
            "{:<34} {:>9.0}ns  reader, allMids",
            "allMids fold to mids map",
            per_call(200, ROUNDS, || {
                std::hint::black_box(read(Event::Payload("allMids", &mids)));
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  reader, l2Book",
            "l2Book fold to a Book",
            per_call(2_000, ROUNDS, || {
                std::hint::black_box(read(Event::Payload("l2Book", &book)));
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  reader, trades",
            "trades fold to prints",
            per_call(2_000, ROUNDS, || {
                std::hint::black_box(read(Event::Payload("trades", &trades)));
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  reader, activeAssetCtx",
            "activeAssetCtx fold",
            per_call(2_000, ROUNDS, || {
                std::hint::black_box(read(Event::Payload("activeAssetCtx", &ctx)));
            })
        );

        eprintln!(
            "{:<34} {:>9.0}ns  reader, every message + publish",
            "one whole beat, folded",
            per_call(200, ROUNDS, || {
                read(Event::Payload("allMids", &mids));
                read(Event::Payload("l2Book", &book));
                read(Event::Payload("activeAssetCtx", &ctx));
                read(Event::Payload("trades", &trades));
                read(Event::Pong(42));
                std::hint::black_box(read(Event::Beat));
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  from_str on both payloads",
            "one whole beat, parsed",
            per_call(200, ROUNDS, || {
                std::hint::black_box(serde_json::from_str::<Value>(&mids_text).unwrap());
                std::hint::black_box(serde_json::from_str::<Value>(&book_text).unwrap());
            })
        );
    }

    /// The chart is a canvas with no text in it, so its accessibility name is
    /// the entire chart for a reader who cannot see it — and both halves of
    /// that sentence are claims that can be false.
    ///
    /// Which venue: the name was fixed at "Hyperliquid" while the chart drew
    /// whichever venue was on screen, so a Lighter chart announced the other
    /// exchange's. Both venues are asserted, because a name that is right on
    /// one and fixed is indistinguishable from one that is right on both.
    ///
    /// Whether marks: "with this account's fills marked" over a chart with no
    /// marks sends someone hunting for arrows that are not there. Lighter
    /// serves no fills to an address, so that is its every chart — and it is
    /// Hyperliquid's too until this account's first fill in this market. Both
    /// states are asserted for the same reason.
    #[test]
    fn the_chart_is_named_for_the_venue_it_draws_and_the_marks_it_has() {
        assert_eq!(
            chart_label(Venue::Hyperliquid, true),
            "Hyperliquid candlestick chart with this account's fills marked"
        );
        assert_eq!(
            chart_label(Venue::Lighter, true),
            "Lighter candlestick chart with this account's fills marked"
        );
        // No marks, so no promise of any. The venue is still named.
        assert_eq!(
            chart_label(Venue::Hyperliquid, false),
            "Hyperliquid candlestick chart"
        );
        assert_eq!(
            chart_label(Venue::Lighter, false),
            "Lighter candlestick chart"
        );

        // The name is the venue's own, rather than a second spelling of it
        // that could drift from the one the switch and the header say.
        for venue in [Venue::Hyperliquid, Venue::Lighter] {
            for marked in [false, true] {
                assert!(
                    chart_label(venue, marked).starts_with(&venue_name(venue)),
                    "the chart has to open with the name the rest of the app uses"
                );
            }
        }
    }

    #[test]
    fn lazy_row_hashes_cover_every_rendered_field() {
        type FieldMutation<T> = (&'static str, fn(&mut T));

        fn fingerprint(value: &impl Hash) -> u64 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        let market = SymbolRow {
            name: "BTC".into(),
            price: 64_000.0,
            change_pct: 2.5,
            volume: 1_000_000.0,
            funding_pct: 0.001,
            leverage: 40.0,
            open_interest: 500_000.0,
            prev: 62_000.0,
            maintenance: 1.0 / 80.0,
            size_decimals: 5,
            selected: false,
            ..Default::default()
        };
        let market_moves: &[FieldMutation<SymbolRow>] = &[
            ("name", |row| row.name.push('X')),
            ("price", |row| row.price += 0.5),
            ("change_pct", |row| row.change_pct += 0.5),
            ("volume", |row| row.volume += 0.5),
            ("funding_pct", |row| row.funding_pct += 0.5),
            ("leverage", |row| row.leverage += 0.5),
            ("open_interest", |row| row.open_interest += 0.5),
            ("prev", |row| row.prev += 0.5),
            ("maintenance", |row| row.maintenance += 0.5),
            ("size_decimals", |row| row.size_decimals += 1),
            ("selected", |row| row.selected = !row.selected),
        ];
        for (field, move_it) in market_moves {
            let mut changed = market.clone();
            move_it(&mut changed);
            assert_ne!(
                fingerprint(&market),
                fingerprint(&changed),
                "changing SymbolRow.{field} must invalidate its lazy subtree"
            );
        }

        let fill = Fill {
            coin: "BTC".into(),
            ts: 1,
            price: 64_000.0,
            size: 0.5,
            buy: true,
            closed_pnl: 10.0,
            hot: true,
            tid: 3,
        };
        let fill_moves: &[FieldMutation<Fill>] = &[
            ("coin", |fill| fill.coin.push('X')),
            ("ts", |fill| fill.ts += 1),
            ("price", |fill| fill.price += 0.5),
            ("size", |fill| fill.size += 0.5),
            ("buy", |fill| fill.buy = !fill.buy),
            ("closed_pnl", |fill| fill.closed_pnl += 0.5),
            ("hot", |fill| fill.hot = !fill.hot),
            ("tid", |fill| fill.tid += 1),
        ];
        for (field, move_it) in fill_moves {
            let mut changed = fill.clone();
            move_it(&mut changed);
            assert_ne!(
                fingerprint(&fill),
                fingerprint(&changed),
                "changing Fill.{field} must invalidate its lazy subtree"
            );
        }
    }

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
            funding_day(btc.clone(), 64_000.0, "3".to_owned(), true),
            "-$57.60/day"
        );
        assert_eq!(
            funding_day(btc.clone(), 64_000.0, "3".to_owned(), false),
            "+$57.60/day"
        );

        // A negative rate turns it around: shorts pay and longs are paid.
        let sol = symbol_row(demo_symbols(), "SOL".to_owned());
        let paid = funding_day(sol, 148.62, "100".to_owned(), true);
        assert!(
            paid.starts_with('+'),
            "a long is paid a negative rate: {paid}"
        );

        // Nothing to say without a market or a size.
        assert!(funding_day(None, 1.0, "1".to_owned(), true).is_empty());
        assert!(funding_day(btc, 64_000.0, "".to_owned(), true).is_empty());
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

    /// The best level of a side, and the one behind it. The asks arrive
    /// reversed, so "best" is a different end of each list and reading it off
    /// the fixture is what keeps these expectations true when the fixture
    /// changes shape.
    fn touch(levels: &[Level], reversed: bool) -> (&Level, &Level) {
        match reversed {
            true => (&levels[levels.len() - 1], &levels[levels.len() - 2]),
            false => (&levels[0], &levels[1]),
        }
    }

    #[test]
    fn crossing_the_spread_is_priced_at_the_levels_that_are_there() {
        let fixture = demo_book();
        let book = || Some(demo_book());
        let (best_ask, next_ask) = touch(&fixture.asks, true);
        let (best_bid, next_bid) = touch(&fixture.bids, false);

        // Inside the best ask: one level, one price, and the slippage is the
        // half spread and nothing else.
        let small = book_impact(book(), "1.0".to_owned(), true);
        assert!(small.ready && !small.short);
        assert!(1.0 < best_ask.size, "a 1.0 buy has to rest on one level");
        assert!((small.paid - best_ask.price).abs() < 1e-9);
        assert!((small.filled - 1.0).abs() < 1e-9);
        assert!(
            (small.slippage_pct - ((best_ask.price - fixture.mid) / fixture.mid * 100.0)).abs()
                < 1e-9
        );

        // Through two levels: the whole best ask, and the rest behind it.
        let size = best_ask.size + 1.2;
        let deeper = book_impact(book(), format!("{size}"), true);
        let expected = (best_ask.size * best_ask.price + 1.2 * next_ask.price) / size;
        assert!((deeper.paid - expected).abs() < 1e-9, "{}", deeper.paid);
        assert!(deeper.paid > small.paid, "depth costs more, never less");

        // Selling walks the bids down, and the slippage still reads positive:
        // it is what the crossing costs, not which way the price went.
        let size = best_bid.size + 0.6;
        let sold = book_impact(book(), format!("{size}"), false);
        let expected = (best_bid.size * best_bid.price + 0.6 * next_bid.price) / size;
        assert!((sold.paid - expected).abs() < 1e-9);
        assert!(sold.slippage_pct > 0.0);
    }

    /// The walk must start at the best price. The asks are stored reversed for
    /// the panel to draw downward, so a walk that trusted the order would
    /// quote the worst level in the book as the first one filled.
    #[test]
    fn a_sweep_starts_at_the_best_price_not_the_first_row() {
        let fixture = demo_book();
        let (best_ask, _) = touch(&fixture.asks, true);
        let impact = book_impact(Some(demo_book()), "0.5".to_owned(), true);
        assert!(
            (impact.paid - best_ask.price).abs() < 1e-9,
            "a small buy pays the best ask, not {}",
            impact.paid
        );
        let worst = fixture.asks[0].price;
        assert!(best_ask.price < worst, "the fixture stores the worst first");
    }

    #[test]
    fn a_size_the_book_cannot_fill_says_so_rather_than_inventing_depth() {
        let fixture = demo_book();
        let depth = fixture.asks.iter().map(|level| level.size).sum::<f64>();
        let spent = fixture
            .asks
            .iter()
            .map(|level| level.size * level.price)
            .sum::<f64>();
        let impact = book_impact(Some(demo_book()), "100".to_owned(), true);
        assert!(depth < 100.0, "the fixture has to be short of the ask");
        assert!(impact.short, "{depth} of asks cannot fill 100");
        assert!((impact.filled - depth).abs() < 1e-9);
        assert!(
            (impact.paid - spent / depth).abs() < 1e-9,
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

        let rows = parse_symbols(&response, HL_CANONICAL, HL_COLLATERAL);
        assert_eq!(rows.len(), 2, "delisted markets are not tradeable");
        assert!(
            rows.iter()
                .all(|row| row.category == HL_CANONICAL && row.collateral == HL_COLLATERAL),
            "every market carries the list it was read out of"
        );
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
            true,
        );

        assert!(fills[0].buy);
        assert_eq!(fills[0].ts, 1_786_092_480, "chart timestamps are seconds");
        assert!(!fills[1].buy);
        assert_eq!(fills[1].closed_pnl, 50.0);
        assert!(fills[1].hot, "a pushed fill arrives lit");
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

    /// What a history read answers, and it is the figure the chart stops on: a
    /// page that carried nothing before the bar the tape began at is the venue
    /// saying there is no more, and asking again would send the same request.
    ///
    /// Counted against that first timestamp rather than against a length,
    /// because the live feed is merging into the same tape from the other end.
    /// A length comparison reads a new live bar as a page of history and asks
    /// again, which is the loop this figure exists to end.
    #[test]
    fn a_history_page_is_measured_by_what_it_added_before_the_first_bar() {
        let bar = |ts: i64| Candle {
            ts,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 1.0,
        };
        let mut tape = vec![bar(120), bar(180)];
        let oldest = tape[0].ts;

        // The venue answers the same window it already gave, and the live bar
        // lands while it does.
        merge(&mut tape, vec![bar(120), bar(180), bar(240)]);
        assert_eq!(tape.len(), 3, "the feed's bar is on the tape");
        assert_eq!(older_than(&tape, oldest), 0, "nothing older arrived");

        merge(&mut tape, vec![bar(0), bar(60)]);
        assert_eq!(older_than(&tape, oldest), 2, "two bars before the first");
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
                hot: false,
                tid: 1,
            },
            Fill {
                coin: "ETH".into(),
                ts: 110,
                price: 3_000.0,
                size: 2.0,
                buy: true,
                closed_pnl: 0.0,
                hot: false,
                tid: 2,
            },
            Fill {
                coin: "BTC".into(),
                ts: 120,
                price: 64_500.0,
                size: 0.5,
                buy: false,
                closed_pnl: 250.0,
                hot: false,
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
        assert_eq!(markers[0].label.as_deref(), Some("0.5"));
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

    /// The trading app as one string. `app.ice` is a shell of `use` lines, so
    /// the view, the handlers and the tests it pulls in are where the boundary
    /// is actually read; scanning the root alone would call the whole boundary
    /// dead. The directory is walked rather than listed so a fragment added
    /// later is covered without anyone remembering this test.
    fn trading_ice() -> String {
        use std::path::Path;

        fn walk(directory: &Path, source: &mut String) {
            let mut entries: Vec<_> = std::fs::read_dir(directory)
                .expect("read the Ice source directory")
                .map(|entry| entry.expect("a directory entry").path())
                .collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    walk(&path, source);
                    continue;
                }
                if path.extension().and_then(std::ffi::OsStr::to_str) != Some("ice") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).expect("read an Ice source");
                source.push_str(&text);
                source.push('\n');
            }
        }

        let mut source = String::new();
        walk(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"),
            &mut source,
        );
        assert!(
            source.contains("daemon Trading\n"),
            "the walk did not reach src/ui/app.ice, so everything below reads as dead"
        );
        source
    }

    /// The chart is drawn by Rust and the panels around it by Ice, so the
    /// palette exists twice: once as tokens in `theme.ice` and once as the
    /// literals below. Nothing makes them agree, and the chart is half the
    /// screen — a drift would be obvious at runtime and invisible until then.
    /// Everything the extern block declares, the view has to read. A field
    /// Rust needs and Ice does not belongs in the struct and not in the
    /// declaration — `Fill.tid` is the pattern. An extern nothing calls is
    /// worse: it means an edit that was supposed to wire it up matched
    /// nothing, which has happened here four times, twice without a single
    /// test noticing, because a test can cover a function the screen never
    /// reaches.
    #[test]
    fn the_boundary_declares_only_what_the_view_reads() {
        const EXTERN: &str = include_str!("ui/extern/hyperliquid.ice");
        let graph = trading_ice();
        let app = graph.as_str();

        let mut dead: Vec<String> = Vec::new();
        for line in EXTERN.lines() {
            let line = line.trim_end();
            let Some(body) = line.strip_prefix("  ") else {
                continue;
            };
            if let Some(rest) = body
                .strip_prefix("sync ")
                .or(body.strip_prefix("pure "))
                .or(body.strip_prefix("component "))
            {
                let name = rest.split('(').next().unwrap_or(rest);
                if !app.contains(&format!("{name}(")) {
                    dead.push(format!("extern `{name}` is declared and never called"));
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
                if !app.contains(&format!(".{field}")) {
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
        let routed: Vec<&str> = app
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
        let orphans: Vec<&str> = app
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
        // A row carrying a side, a size, an entry, a cliff, a funding flow and
        // a PnL is worth more than its ticker to somebody who cannot see the
        // rest of it — and the button it is drawn as replaces all six.
        assert_eq!(
            position_label(held(30.0)),
            "BTC long 30, entry 60,000.00, liquidation 45,000.00, \
             funding $0, unrealized +$0.00 at +0.00%"
        );
        // A venue that reports no cliff for a position leaves the column
        // reading "none", and the name may not invent a price for it.
        assert_eq!(
            position_label(Position {
                liq: 0.0,
                ..held(30.0)
            }),
            "BTC long 30, entry 60,000.00, no liquidation price, \
             funding $0, unrealized +$0.00 at +0.00%"
        );

        let order = Order {
            oid: 1,
            coin: "BTC".into(),
            buy: true,
            price: 60_000.0,
            size: 0.5,
            ts: 0,
        };
        assert_eq!(order_label(order.clone()), "BTC buy 0.5 at 60,000.00");
        assert_eq!(
            order_label(Order {
                buy: false,
                ..order
            }),
            "BTC sell 0.5 at 60,000.00"
        );

        let fill = |closed_pnl: f64, buy: bool| Fill {
            coin: "BTC".into(),
            ts: 0,
            price: 64_000.0,
            size: 0.5,
            buy,
            closed_pnl,
            hot: false,
            tid: 0,
        };
        // A fill that closed something is named by what it took and what it
        // made, because the row draws both: what it made alone cannot tell a
        // full close from a quarter of one. A fill that opened has no realized
        // PnL and the row draws an em dash, so the name says nothing there.
        assert_eq!(
            fill_label(fill(250.0, false)),
            "BTC sold 0.5 at 64,000.00, realized +$250.00"
        );
        assert_eq!(fill_label(fill(0.0, true)), "BTC bought 0.5 at 64,000.00");
        assert!(
            position_label(held(-30.0)).starts_with("BTC short 30,"),
            "the size reads unsigned; the word carries the side"
        );

        // A page tab draws a heading and is heard as the act, and the page
        // already on screen is a button like the other three: which one that
        // is has to be in the name, because nothing else about a button is.
        assert_eq!(page_label("PORTFOLIO".into()), "Show the portfolio page");
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
            size_decimals: 5,
            selected: false,
            ..Default::default()
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
            size_decimals: 5,
            open_interest: 0.0,
            prev: 100.0,
            selected: false,
            ..Default::default()
        };
        let quoted = |price: &str, size: &str, leverage: &str, buy: bool| {
            price_ticket(
                amount(price),
                size.into(),
                leverage.into(),
                Some(market(40.0)),
                buy,
                0.0,
                false,
                None,
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
        let unknown = price_ticket(100.0, "2".into(), "10".into(), None, true, 0.0, false, None);
        assert!(unknown.ready, "the order is still describable");
        assert_eq!(unknown.notional, 200.0);
        assert_eq!(unknown.margin, 20.0);
        assert!(!unknown.known, "and the panel says it does not know");
        assert_eq!(unknown.liquidation, 0.0, "rather than an optimistic one");
        // Not knowing has three causes and they are three different sentences.
        // Only one of them is a load in progress; the other two are finished
        // reads, and "market not loaded" over either is the panel describing a
        // wait that is not happening.
        assert_eq!(
            liquidation_gap(None, false, false, false),
            "market not loaded"
        );
        assert_eq!(liquidation_gap(None, true, false, false), "not listed here");
        assert_eq!(
            liquidation_gap(Some(market(40.0)), true, false, false),
            "no requirement stated"
        );
        // A row is a row whether or not the universe around it arrived, so the
        // market's own answer does not depend on the list.
        assert_eq!(
            liquidation_gap(Some(market(40.0)), false, false, false),
            liquidation_gap(Some(market(40.0)), true, false, false)
        );
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
            100.0,
            "2".into(),
            "10".into(),
            Some(strict),
            true,
            0.0,
            false,
            None,
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
        // The row deletes the level it names, so its label says so rather than
        // reading as a status line.
        assert_eq!(
            alert_label(watched[0].clone()),
            "Stop watching BTC above 65,000.00"
        );
        assert_eq!(
            alert_label(check_alerts(watched.clone(), beat(65_000.0))[0].clone()),
            "Stop watching BTC at 65,000.00",
            "a level already reached has no side left to wait on"
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
        let btc = symbol_row(demo_symbols(), "BTC".into());
        let size =
            |share: f64| ticket_afford(held(10_000.0), 100.0, btc.clone(), 5.0, share, false);

        // 10,000 free at 5x is 50,000 of notional; a quarter of it at 100 is
        // 125. Equity is 100,000 and would say ten times that, which is the
        // point: equity already has positions standing on it.
        assert_eq!(size(0.25), "125");
        assert_eq!(size(1.0), "500");
        assert_eq!(size(2.0), "500", "past everything is everything");

        // Nothing to deploy, nothing to price against, nothing levered.
        assert_eq!(size(0.0), "");
        assert_eq!(
            ticket_afford(held(0.0), 100.0, btc.clone(), 5.0, 0.5, false),
            ""
        );
        assert_eq!(
            ticket_afford(held(10_000.0), 0.0, btc.clone(), 5.0, 0.5, false),
            ""
        );
        assert_eq!(
            ticket_afford(held(10_000.0), 100.0, btc.clone(), 0.0, 0.5, false),
            ""
        );
        assert_eq!(
            ticket_afford(None, 100.0, btc, 5.0, 0.5, false),
            "",
            "no account"
        );

        // A size that does not divide evenly lands on the market's own step,
        // and lands below what the account can afford rather than above it: a
        // MAX rounded up is an order the margin engine refuses.
        let sol = symbol_row(demo_symbols(), "SOL".into()).expect("a market");
        let exact = 10_000.0 * 5.0 / 148.62;
        let filled = ticket_afford(held(10_000.0), 148.62, Some(sol.clone()), 5.0, 1.0, false);
        assert_eq!(sol.size_decimals, 2, "the venue quotes SOL to a hundredth");
        assert_eq!(filled, "336.42");
        assert!(
            amount(&filled) <= exact,
            "MAX filled {filled} of a {exact} the account can afford"
        );

        // The leverage handed in is the one the ticket was priced at, which is
        // the market's cap when the reader typed past it. Sizing at the typed
        // number would fill in an order the margin engine refuses, by exactly
        // the factor it overshot.
        let capped = price_ticket(
            100.0,
            "".into(),
            "40".into(),
            symbol_row(demo_symbols(), "kPEPE".into()),
            true,
            0.0,
            false,
            None,
        );
        assert_eq!(capped.leverage, 10.0, "the market caps it at ten");
        assert_eq!(
            ticket_afford(
                held(10_000.0),
                100.0,
                symbol_row(demo_symbols(), "kPEPE".into()),
                capped.leverage,
                1.0,
                false
            ),
            "1,000"
        );

        // A market that trades in whole units, priced above what the account
        // can put up: the floor lands on nothing. "0" is a size, and a MAX
        // that fills one in offers to send an order for none of the
        // instrument. There is nothing to offer, and it says so the same way
        // it says the account has nothing free.
        let whole = SymbolRow {
            size_decimals: 0,
            ..symbol_row(demo_symbols(), "BTC".into()).expect("a market")
        };
        assert_eq!(
            ticket_afford(
                held(10_000.0),
                64_000.0,
                Some(whole.clone()),
                5.0,
                1.0,
                false
            ),
            "",
            "10,000 free at 5x buys 0.78 of a coin that only trades whole"
        );
        // Enough for one, and there is a size to offer again.
        assert_eq!(
            ticket_afford(held(13_000.0), 64_000.0, Some(whole), 5.0, 1.0, false),
            "1"
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
        assert_eq!(effect("10", true), "Reduces your short to 20");
        assert_eq!(effect("30", true), "Closes your short");
        assert_eq!(effect("50", true), "Closes your short and opens 20 long");
        assert_eq!(effect("10", false), "Opens 10 short", "same side, adds");

        // A market you hold nothing in, and a market that is not this one.
        assert_eq!(
            ticket_effect(held.clone(), "ETH".into(), "1".into(), true),
            "Opens 1 long"
        );
        assert_eq!(
            ticket_effect(Vec::new(), "BTC".into(), "1".into(), true),
            "Opens 1 long"
        );
        // Nothing typed is not an order, and says nothing.
        assert_eq!(effect("", true), "");
        assert_eq!(effect("0", true), "");
    }

    #[test]
    fn a_closing_order_ties_up_nothing_and_has_no_cliff() {
        let quote = |size: &str, buy: bool, held: f64| {
            price_ticket(
                100.0,
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
                    size_decimals: 5,
                    selected: false,
                    ..Default::default()
                }),
                buy,
                held,
                false,
                None,
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

    /// An isolated position stands on its own margin and a cross one stands on
    /// the account. The two arithmetics have to agree where they describe the
    /// same thing — an account holding nothing else is exactly the isolated
    /// case — and diverge everywhere else, because everything else the account
    /// carries has already moved the line the fall is measured to.
    #[test]
    fn a_cross_cliff_is_the_accounts_fall_and_an_isolated_one_is_its_own() {
        let maintenance = 1.0 / 80.0;
        let market = SymbolRow {
            name: "BTC".into(),
            price: 100.0,
            leverage: 40.0,
            maintenance,
            size_decimals: 5,
            ..Default::default()
        };
        let lone = |equity: f64, requirement: f64, positions: Vec<Position>| {
            Some(Account {
                value: equity,
                cross_value: equity,
                pnl: 0.0,
                withdrawable: equity,
                notional: 0.0,
                maintenance: requirement,
                health: 0.0,
                margin_pct: 0.0,
                positions,
            })
        };

        let isolated = price_ticket(
            100.0,
            "2".into(),
            "10".into(),
            Some(market.clone()),
            true,
            0.0,
            false,
            None,
        );
        assert!(isolated.known);
        assert!((isolated.liquidation - 100.0 * 0.9 / (1.0 - maintenance)).abs() < 1e-9);

        // The same order held cross against an account whose whole equity is
        // the margin this position would post, and which owes nothing else.
        // That is the isolated case written the other way round, so the two
        // have to land on the same price.
        let alone = price_ticket(
            100.0,
            "2".into(),
            "10".into(),
            Some(market.clone()),
            true,
            0.0,
            true,
            lone(20.0, 0.0, Vec::new()),
        );
        assert!(
            (alone.liquidation - isolated.liquidation).abs() < 1e-6,
            "cross {} vs isolated {}",
            alone.liquidation,
            isolated.liquidation
        );

        // Give the account more equity than the position needs and the cliff
        // moves away; make it owe maintenance on something else and it moves
        // back. Neither is visible to the isolated formula, which is the whole
        // reason the mode chooses between them.
        let rich = price_ticket(
            100.0,
            "2".into(),
            "10".into(),
            Some(market.clone()),
            true,
            0.0,
            true,
            lone(200.0, 0.0, Vec::new()),
        );
        assert!(
            rich.liquidation < alone.liquidation,
            "more equity is a longer fall: {} vs {}",
            rich.liquidation,
            alone.liquidation
        );
        let owing = price_ticket(
            100.0,
            "2".into(),
            "10".into(),
            Some(market.clone()),
            true,
            0.0,
            true,
            lone(200.0, 100.0, Vec::new()),
        );
        assert!(
            owing.liquidation > rich.liquidation,
            "a requirement elsewhere raises the floor: {} vs {}",
            owing.liquidation,
            rich.liquidation
        );

        // No account is no cliff, and the panel says so rather than quoting
        // the isolated one under a cross label.
        let unbanked = price_ticket(
            100.0,
            "2".into(),
            "10".into(),
            Some(market.clone()),
            true,
            0.0,
            true,
            None,
        );
        assert!(!unbanked.known);
        assert_eq!(unbanked.liquidation, 0.0);
        assert_eq!(
            liquidation_gap(Some(market.clone()), true, true, false),
            "needs the account it is held against"
        );
        // And a builder market is never held against this account at all, so
        // reading one changes nothing.
        let builder = SymbolRow {
            name: "xyz:NVDA".into(),
            ..market.clone()
        };
        let elsewhere = price_ticket(
            100.0,
            "2".into(),
            "10".into(),
            Some(builder.clone()),
            true,
            0.0,
            true,
            lone(200.0, 0.0, Vec::new()),
        );
        assert!(!elsewhere.known, "the account on screen is the wrong one");
        assert_eq!(elsewhere.liquidation, 0.0);
        assert_eq!(
            liquidation_gap(Some(builder.clone()), true, true, true),
            "separate margin account"
        );
        // Isolated is the market's own arithmetic and is not gated by any of
        // that, which is the point of not gating the whole panel.
        assert!(
            price_ticket(
                100.0,
                "2".into(),
                "10".into(),
                Some(builder),
                true,
                0.0,
                false,
                None
            )
            .liquidation
                > 0.0
        );
    }

    /// The size the venue would be sent, which is neither what was typed nor
    /// what was typed rounded: dollars become the instrument at a stated price,
    /// and reduce-only trims an order to the position it promised not to pass.
    #[test]
    fn an_order_is_sized_in_the_instrument_whatever_unit_it_was_typed_in() {
        let sol = symbol_row(demo_symbols(), "SOL".into()).expect("a market");
        assert_eq!(sol.size_decimals, 2, "the venue quotes SOL to a hundredth");
        let sized = |typed: &str, usd: bool, price: f64| {
            order_size(
                typed.into(),
                usd,
                price,
                Some(sol.clone()),
                false,
                0.0,
                true,
            )
        };

        // Coins pass through; dollars divide, and land on the venue's step
        // downward so the order never asks for more than was typed.
        assert_eq!(sized("3", false, 100.0), "3");
        assert_eq!(sized("300", true, 100.0), "3");
        assert_eq!(sized("1,000", true, 148.62), "6.72");
        assert!(
            amount(&sized("1,000", true, 148.62)) <= 1_000.0 / 148.62,
            "a size rounded up buys past the dollars that were typed"
        );
        // Dollars with nothing to price them against is not a size.
        assert_eq!(sized("1,000", true, 0.0), "");
        assert_eq!(sized("0", false, 100.0), "");

        // Reduce-only against the other side is a cap, because the venue fills
        // to the position and no further.
        let capped = |typed: &str, held: f64, buy: bool| {
            order_size(
                typed.into(),
                false,
                100.0,
                Some(sol.clone()),
                true,
                held,
                buy,
            )
        };
        assert_eq!(capped("50", -30.0, true), "30", "trimmed to the short");
        assert_eq!(capped("10", -30.0, true), "10", "under it, untouched");
        assert_eq!(capped("50", 30.0, false), "30", "and the long mirrors it");
        // On the side already held there is nothing to trim to: the venue
        // refuses the order outright and the sentence beside the box says so,
        // so the size is left as typed rather than quietly becoming another.
        assert_eq!(capped("50", -30.0, false), "50");
        assert_eq!(capped("50", 0.0, true), "50");

        // The same quantity said the other way, which is what the toggle asks
        // for. Three at 148.62 is 445.86 of them and back again.
        assert_eq!(
            retype_size("3".into(), true, 148.62, Some(sol.clone())),
            "445.86"
        );
        assert_eq!(
            retype_size("445.86".into(), false, 148.62, Some(sol.clone())),
            "3"
        );
        // Nothing typed and nothing to price it at leave the field alone
        // rather than emptying it under the reader.
        assert_eq!(retype_size("".into(), true, 148.62, Some(sol.clone())), "");
        assert_eq!(retype_size("3".into(), true, 0.0, Some(sol)), "3");
    }

    /// A level on the wrong side of the entry is the other kind of order, and
    /// the venue sends it as one. A stop past the cliff is not an order at all
    /// by the time it would fire.
    #[test]
    fn a_level_on_the_wrong_side_of_the_entry_is_refused_with_the_reason() {
        // Two bitcoin long from 64,000: a thousand either way is two thousand.
        assert_eq!(
            level_pnl(64_000.0, "65,000".into(), "2".into(), true),
            2_000.0
        );
        assert_eq!(
            level_pnl(64_000.0, "63,000".into(), "2".into(), true),
            -2_000.0
        );
        // The short is the mirror, which is what makes the sign the side's
        // rather than the level's.
        assert_eq!(
            level_pnl(64_000.0, "63,000".into(), "2".into(), false),
            2_000.0
        );
        // Nothing to measure between is nothing.
        assert_eq!(level_pnl(0.0, "63,000".into(), "2".into(), true), 0.0);
        assert_eq!(level_pnl(64_000.0, "".into(), "2".into(), true), 0.0);

        // An empty field is the order without the level, not a refusal.
        assert!(tp_refused(64_000.0, "".into(), true).is_empty());
        assert!(sl_refused(64_000.0, "  ".into(), true, 0.0).is_empty());

        assert!(tp_refused(64_000.0, "65,000".into(), true).is_empty());
        assert_eq!(
            tp_refused(64_000.0, "63,000".into(), true),
            "A take-profit on a long sits above the 64,000.00 it opens at."
        );
        assert!(tp_refused(64_000.0, "63,000".into(), false).is_empty());
        assert_eq!(
            tp_refused(64_000.0, "65,000".into(), false),
            "A take-profit on a short sits below the 64,000.00 it opens at."
        );
        assert_eq!(
            tp_refused(0.0, "65,000".into(), true),
            "There is no entry price yet to set a target against."
        );

        assert!(sl_refused(64_000.0, "63,000".into(), true, 0.0).is_empty());
        assert_eq!(
            sl_refused(64_000.0, "65,000".into(), true, 0.0),
            "A stop-loss on a long sits below the 64,000.00 it opens at."
        );
        // On the right side, and still not there: the engine closes the
        // position at 60,000 and the stop never fires.
        assert_eq!(
            sl_refused(64_000.0, "59,000".into(), true, 60_000.0),
            "The engine closes this long at 60,000.00, before that stop is reached."
        );
        assert!(
            sl_refused(64_000.0, "61,000".into(), true, 60_000.0).is_empty(),
            "inside the cliff is where a stop belongs"
        );
        // Without a cliff there is no second refusal to make.
        assert!(sl_refused(64_000.0, "59,000".into(), true, 0.0).is_empty());
    }

    /// Reduce-only is refused rather than shrunk, so the box has to say which
    /// of the two ways it is wrong.
    #[test]
    fn reduce_only_says_why_the_venue_would_drop_the_order() {
        let held = demo_positions();
        // Short 30 bitcoin: a buy reduces it and a sell adds to it.
        assert!(reduce_refused(held.clone(), "BTC".into(), true).is_empty());
        assert_eq!(
            reduce_refused(held.clone(), "BTC".into(), false),
            "This order adds to the short you hold. Reduce-only sends nothing rather than a smaller order."
        );
        // Long 40 ether is the mirror.
        assert!(reduce_refused(held.clone(), "ETH".into(), false).is_empty());
        assert!(
            reduce_refused(held.clone(), "ETH".into(), true)
                .starts_with("This order adds to the long")
        );
        // And a market nothing is held in has nothing to reduce either way.
        assert_eq!(
            reduce_refused(held, "kPEPE".into(), true),
            "Reduce-only needs a position to reduce, and there is none in this market."
        );
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
        assert_eq!(fmt_size(short), "30", "the field takes no sign");
        assert!(short < 0.0, "so closing a short buys");
        // And the effect line agrees with what the button just did.
        assert_eq!(
            ticket_effect(book, "BTC".into(), fmt_size(short), short < 0.0),
            "Closes your short"
        );
    }

    /// A size is the instrument's number and the panel may not round it. The
    /// quote used to follow the magnitude instead — two decimals above 1, three
    /// above a thousandth — so a position of 30.12345 seeded CLOSE POSITION
    /// with 30.12, and the order that button fills in left a residual open. The
    /// same rule quoted a small size more finely than a large one on the very
    /// same market, which no venue does.
    #[test]
    fn a_size_keeps_every_digit_it_carries_however_large_it_is() {
        // Bitcoin trades to a hundred-thousandth, at any magnitude.
        assert_eq!(fmt_size(0.00001), "0.00001");
        assert_eq!(fmt_size(30_000.000_01), "30,000.00001");
        // And a size with nothing after the point is a whole number of coins.
        assert_eq!(fmt_size(30.0), "30");
        assert_eq!(fmt_size(-0.5), "0.5", "the field takes no sign");
        // Subtracting two of the venue's own sizes leaves float noise that is
        // not a size, and the quote stops where the venue's digits do.
        assert_eq!(fmt_size(0.3 - 0.1), "0.2");

        let held = -30.12345;
        let seeded = fmt_size(held);
        assert_eq!(seeded, "30.12345");
        assert_eq!(
            amount(&seeded),
            held.abs(),
            "the seeded order is the position, to the digit"
        );
        let position = Position {
            coin: "BTC".into(),
            size: held,
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
        // What CLOSE POSITION fills in has to close the position: a rounded
        // seed reads back as a partial close, or past the position as a flip.
        assert_eq!(
            ticket_effect(vec![position], "BTC".into(), seeded, held < 0.0),
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
            size_decimals: 5,
            open_interest: 0.0,
            prev: 0.0,
            selected: false,
            ..Default::default()
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

    /// A one-level book, so the market it names is the only thing under test.
    fn book_payload(coin: &str, bid: f64, ask: f64) -> Value {
        json!({
            "coin": coin,
            "levels": [
                [{ "px": bid.to_string(), "sz": "1.0", "n": 1 }],
                [{ "px": ask.to_string(), "sz": "1.0", "n": 1 }],
            ]
        })
    }

    /// Switching markets re-subscribes on the socket that is already open, so
    /// the exchange goes on serving the market being left until it acts on
    /// the unsubscribe. Everything that arrives in that window names the old
    /// coin, and the app has no way to tell what it is looking at apart from
    /// what the feed publishes.
    #[test]
    fn a_book_from_the_market_the_reader_left_is_never_published_as_this_ones() {
        let tape = tape_focus(tape_new(), "ETH".into(), "1m".into());
        let mut read = market_reader(tape);
        let (bid, ask) = (64_848.0, 64_850.0);

        // The beat that takes up the focus, so what follows is the steady
        // state rather than the reader's first pass.
        assert!(read(Event::Beat).is_none());

        assert!(read(Event::Payload("l2Book", &book_payload("BTC", bid, ask))).is_none());
        assert!(
            read(Event::Beat).is_none(),
            "a book from another market is not news about this one"
        );
        // A pong moves the beat on its own, so the beat below publishes for a
        // reason that has nothing to do with the book it carries.
        read(Event::Pong(7));
        let tick = read(Event::Beat).expect("a fresh round trip is worth a beat");
        assert!(
            tick.book.is_none(),
            "the beat published a book for a market the reader is not on"
        );

        read(Event::Payload("l2Book", &book_payload("ETH", bid, ask)));
        let tick = read(Event::Beat).expect("the focused market's book is worth a beat");
        assert_eq!(
            tick.book.expect("the focused market's book").mid,
            (bid + ask) / 2.0
        );
    }

    /// A book is stored as *the* book, so one from the market being left has
    /// to be turned away by the coin it names. `activeAssetCtx` needs no such
    /// guard, and this is the whole reason why: a context keeps the coin it
    /// describes and is folded into the row of that name, so one that arrives
    /// in the switch window restates its own market rather than the one on
    /// screen.
    #[test]
    fn a_context_is_folded_into_the_market_it_names_not_the_one_on_screen() {
        let row = |name: &str, price: f64| SymbolRow {
            name: name.into(),
            price,
            change_pct: 0.0,
            volume: 0.0,
            funding_pct: 0.0,
            leverage: 40.0,
            open_interest: 0.0,
            prev: price,
            maintenance: 1.0 / 80.0,
            size_decimals: 5,
            selected: false,
            ..Default::default()
        };
        // The reader has moved to BTC and the socket serves one more context
        // for the market it just left.
        let beat = MarketTick {
            context: Some(parse_context(
                "ETH".into(),
                &json!({ "markPx": "3540.0", "prevDayPx": "3500.0", "dayNtlVlm": "9.0" }),
            )),
            ..MarketTick::default()
        };
        let rows = apply_feed(vec![row("BTC", 64_000.0), row("ETH", 3_500.0)], beat);

        assert_eq!(rows[0].price, 64_000.0, "the market on screen is untouched");
        assert_eq!(
            rows[1].price, 3_540.0,
            "the context restated its own market"
        );
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

    fn fill(ts: i64, tid: i64, hot: bool) -> Fill {
        Fill {
            coin: "BTC".into(),
            ts,
            price: 1.0,
            size: 1.0,
            buy: true,
            closed_pnl: 0.0,
            hot,
            tid,
        }
    }

    #[test]
    fn pushed_fills_stack_newest_first_without_repeating_the_snapshot() {
        let snapshot = push_fills(Vec::new(), vec![fill(10, 1, false), fill(30, 3, false)], 4);
        assert_eq!(snapshot[0].ts, 30, "newest first");

        // The feed re-sends a fill the snapshot already showed alongside a
        // new one; only the new one lands, and only it is lit.
        let pushed = push_fills(snapshot, vec![fill(30, 3, true), fill(40, 4, true)], 4);
        assert_eq!(pushed.len(), 3, "the repeat is dropped");
        assert_eq!(pushed[0].ts, 40);
        assert!(pushed[0].hot, "a fill the feed pushed arrives lit");
        assert!(!pushed[1].hot, "the fill it already held stays cold");

        let capped = push_fills(pushed, vec![fill(50, 5, true)], 2);
        assert_eq!(capped.len(), 2, "the list is capped");
        assert_eq!(capped[0].ts, 50);
    }

    /// The listed fills' one invariant: a trade id appears once. The rows are
    /// `lazy`, keyed on that id, so a repeat is not a cosmetic duplicate — the
    /// two rows share a cache entry and a parking slot.
    #[test]
    fn push_fills_lists_each_trade_id_once() {
        let listed = push_fills(
            vec![fill(10, 1, false)],
            // One repeat of the history, and one repeat inside the batch.
            vec![fill(10, 1, false), fill(20, 2, false), fill(21, 2, false)],
            10,
        );
        let mut ids: Vec<i64> = listed.iter().map(|fill| fill.tid).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2], "two trade ids, two rows");
    }

    /// A payload with no trade id has no identity to list the row under, and
    /// `unwrap_or_default` would give every such fill the id `0` — one shared
    /// row identity for unrelated trades. It is dropped instead.
    #[test]
    fn a_fill_without_a_trade_id_is_not_listed() {
        let fills = parse_fills(
            &json!([
                { "coin": "BTC", "px": "64000.0", "sz": "0.1", "side": "B", "time": 1_786_092_480_123i64, "closedPnl": "0.0", "tid": 7 },
                { "coin": "BTC", "px": "64500.0", "sz": "0.1", "side": "A", "time": 1_786_092_540_000i64, "closedPnl": "50.0" },
                { "coin": "ETH", "px": "3100.0", "sz": "2.0", "side": "B", "time": 1_786_092_600_000i64, "closedPnl": "0.0" },
            ]),
            false,
        );
        assert_eq!(
            fills.iter().map(|fill| fill.tid).collect::<Vec<_>>(),
            vec![7],
            "the two without an id would have shared the id 0"
        );
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
        let now = now_seconds();
        assert_eq!(fmt_age(now, now), "now");
        assert_eq!(fmt_age(now - 59, now), "now");
        assert_eq!(fmt_age(now - 60, now), "1m");
        assert_eq!(fmt_age(now - 3_599, now), "59m");
        assert_eq!(fmt_age(now - 3_600, now), "1h");
        assert_eq!(fmt_age(now - 86_400 * 4, now), "4d");
        // An order the exchange gave no timestamp, and one stamped in the
        // future by a clock that disagrees with ours, both read as unknown
        // rather than as "now" or as a negative age.
        assert_eq!(fmt_age(0, now), "—");
        assert_eq!(fmt_age(now + 600, now), "—");

        // Hourly funding is a hundredth of a percent on most of the exchange,
        // so the two decimals every other figure here uses would render 166 of
        // the 177 funded markets as the same "+0.00%".
        assert_eq!(fmt_pct(0.00125), "+0.00%", "what two decimals can say");
        assert_eq!(fmt_funding(0.00125), "+0.0013%");
        assert_eq!(fmt_funding(-0.232341), "-0.2323%", "the widest real rate");
        assert_eq!(fmt_funding(0.0), "+0.0000%");
    }

    /// The wire the live tests open is one flag for the whole process, so the
    /// run it is opened in has to be one where every test is a live test.
    /// `--ignored` is that run. `--include-ignored` is the one that is not, and
    /// it is the whole reason the opt-in checks: under it every ordinary test
    /// that outlives the flip reads the exchange instead of its fixtures.
    #[test]
    fn the_wire_opt_in_refuses_the_run_that_mixes_the_two_sets() {
        let run = |flag: &str| ["trading-example", flag].map(str::to_owned).into_iter();
        assert!(only_the_live_tests_are_running(run("--ignored")));
        assert!(!only_the_live_tests_are_running(run("--include-ignored")));
    }

    /// The fixtures above encode what the exchange documents. This one asks
    /// the exchange. It talks to the network, so it stays opt-in:
    /// `cargo test -p trading-example -- --ignored`.
    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn live_api_matches_the_shapes_parsed_here() {
        open_the_wire();
        smol::block_on(async {
            let symbols = hl_symbols(Chain::Mainnet, HL_CANONICAL)
                .await
                .expect("symbol list");
            assert!(symbols.len() > 20, "got {} markets", symbols.len());
            let btc = symbols.iter().find(|row| row.name == "BTC").expect("BTC");
            assert!(btc.price > 0.0 && btc.volume > 0.0, "BTC context is empty");
            assert_eq!(btc.category, HL_CANONICAL, "the exchange's own list");
            assert_eq!(btc.collateral, HL_COLLATERAL);

            // The universe is more than one dex since HIP-3, and every one of
            // its markets carries the list it came out of. Asserted against
            // whatever is deployed today rather than against a named dex: the
            // deployments are third-party and come and go, so the claim is the
            // shape — a second group exists, its markets are `dex:SYMBOL`, and
            // each states what it settles in.
            let builder: Vec<&SymbolRow> = symbols
                .iter()
                .filter(|row| row.category != HL_CANONICAL)
                .collect();
            assert!(
                !builder.is_empty(),
                "perpDexs lists builder deployments and none of them reached the rail"
            );
            for row in &builder {
                assert!(
                    row.name.contains(':'),
                    "a builder market is named dex:SYMBOL, got {}",
                    row.name
                );
                assert!(!row.category.is_empty() && !row.collateral.is_empty());
            }

            // And the qualified name is the whole identity: the book request
            // takes it verbatim, with no `dex` parameter anywhere.
            let listed = builder
                .iter()
                .find(|row| row.price > 0.0)
                .expect("a builder market with a price");
            let book = info(
                Chain::Mainnet,
                json!({ "type": "l2Book", "coin": listed.name }),
            )
            .await;
            assert_eq!(
                text(&book.expect("book for a builder market"), "coin"),
                listed.name,
                "the exchange answers a dex-qualified coin as itself"
            );

            // A fresh tape adopts the first market loaded into it.
            let tape = tape_new();
            let bars = hl_candles(Chain::Mainnet, tape.clone(), "BTC".into(), "1m".into())
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
            let account = hl_account(
                Chain::Mainnet,
                "0xdfc24b077bc1425ad1dea75bcb6f8158e10df303".into(),
            )
            .await
            .expect("clearinghouse state");
            assert!(account.value > 0.0, "the HLP vault holds a balance");

            // The exchange reports a mark, a PnL, and a return; the feed only
            // sends a price. Hand its own marks back to the arithmetic that
            // turns one into the others, and it has to land where the
            // exchange did — position by position, on a live book.
            let watched = hl_account(Chain::Mainnet, WATCHED.to_owned())
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
        open_the_wire();
        let tape = tape_focus(tape_new(), "BTC".into(), "1m".into());
        let feed = hl_market_feed(Chain::Mainnet, tape.clone());
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
                size_decimals: 5,
                open_interest: 0.0,
                prev: 1.0,
                selected: false,
                ..Default::default()
            },
            SymbolRow {
                name: "ETH".into(),
                price: 1.0,
                change_pct: 0.0,
                volume: 0.0,
                funding_pct: 0.0,
                leverage: 25.0,
                maintenance: 1.0 / (2.0 * 25.0),
                size_decimals: 4,
                open_interest: 0.0,
                prev: 1.0,
                selected: false,
                ..Default::default()
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

    /// The two fixtures have to be two universes, or every test that switches
    /// between them is switching between the same exchange twice.
    #[test]
    fn the_two_fixture_universes_are_not_the_same_universe() {
        let names = |rows: Vec<SymbolRow>| -> Vec<String> {
            rows.into_iter().map(|row| row.name).collect()
        };
        let here = names(demo_symbols());
        let there = names(demo_symbols_lighter());
        assert!(here.contains(&"kPEPE".to_owned()));
        assert!(!there.contains(&"kPEPE".to_owned()));
        assert!(there.contains(&"1000PEPE".to_owned()));
        // And a market only one of them lists at all, which a rename does not
        // cover.
        assert!(there.contains(&"AAPL".to_owned()));
        assert!(!here.contains(&"AAPL".to_owned()));
        // Shared names too: a universe with nothing in common would make
        // "keeps a listed ticker" untestable.
        assert!(here.contains(&"BTC".to_owned()) && there.contains(&"BTC".to_owned()));
        // A shared ticker is still not a shared row. Bitcoin is capped at 40x
        // on one fixture and 50x on the other because that is what the two
        // exchanges publish, and it is the figure the ticket prices a cliff
        // against — so a Lighter fixture derived from this one would quote
        // Hyperliquid's risk under Lighter's name.
        let cap = |rows: Vec<SymbolRow>| symbol_row(rows, "BTC".into()).expect("bitcoin").leverage;
        assert_eq!(cap(demo_symbols()), 40.0);
        assert_eq!(cap(demo_symbols_lighter()), 50.0);

        // Volume descending, as both parsers leave a universe — which is what
        // makes the first row the venue's busiest market.
        let mut sorted = demo_symbols_lighter();
        sorted.sort_by(|left, right| right.volume.total_cmp(&left.volume));
        assert_eq!(names(sorted), there);
    }

    /// A ticker is not portable, and a terminal pointed at a market its venue
    /// does not list draws nothing under a header that still names it.
    #[test]
    fn a_market_the_venue_does_not_list_lands_on_the_one_it_trades_most() {
        let there = demo_symbols_lighter();
        // Not typed here: the busiest market is whatever sorts first, and the
        // fallback has to be that row rather than a name this test agrees with.
        let busiest = there.first().expect("a universe").name.clone();

        // kPEPE is Hyperliquid's spelling. Carried across, it names nothing.
        assert!(symbol_row(there.clone(), "kPEPE".into()).is_none());
        assert_eq!(listed_coin(there.clone(), "kPEPE".into()), busiest);
        // And what it lands on is a market that is actually there.
        assert!(symbol_row(there.clone(), busiest.clone()).is_some());

        // A ticker both venues list is kept — including one that is not the
        // fallback, so keeping is distinguishable from always landing home.
        assert_ne!("SOL", busiest);
        assert_eq!(listed_coin(there.clone(), "SOL".into()), "SOL");
        assert_eq!(listed_coin(there, busiest.clone()), busiest);

        // The same rule the other way round, so neither venue is the one with
        // the special case.
        assert!(symbol_row(demo_symbols(), "1000PEPE".into()).is_none());
        assert_eq!(
            listed_coin(demo_symbols(), "1000PEPE".into()),
            demo_symbols().first().expect("a universe").name
        );

        // An empty universe is a read that answered nothing, not a venue that
        // lists nothing, so the market on screen survives it.
        assert_eq!(listed_coin(Vec::new(), "kPEPE".into()), "kPEPE");
    }
}
