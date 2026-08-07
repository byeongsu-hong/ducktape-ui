//! The Hyperliquid `info` API behind one blocking POST, moved off the UI
//! thread with `smol::unblock`. Every response is read as a `Value` and
//! mapped by hand: the exchange sends all numbers as JSON strings, so a
//! derive would need a custom deserializer per field anyway.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ducktape_ui::ui::candle_chart::{
    ChartMarker, MarkerShape, PriceLine, SharedCandles, candle_chart_shared, format_price,
    format_volume,
};
use ducktape_ui::ui::theme;
use iced::{Color, Element, Font, Length};
use serde_json::{Value, json};
use ui_lang_runtime::{Role, StableId, accessible};

pub use ducktape_ui::ui::candle_chart::{Candle, CandleHit};

const INFO_URL: &str = "https://api.hyperliquid.xyz/info";
const TIMEOUT: Duration = Duration::from_secs(15);
/// Candles fetched when a market is opened, and on every poll after that.
const BACKFILL_BARS: i64 = 500;
const REFRESH_BARS: i64 = 3;
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
    /// Yesterday's close, kept so a mid-price tick can re-derive the 24h
    /// change without re-fetching the whole universe.
    pub prev_day: f64,
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

fn parse_candles(value: &Value) -> Vec<Candle> {
    value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|candle| Candle {
            // Hyperliquid timestamps are milliseconds; the chart wants seconds.
            ts: value_i64(candle, "t") / 1_000,
            open: num(candle, "o"),
            high: num(candle, "h"),
            low: num(candle, "l"),
            close: num(candle, "c"),
            volume: num(candle, "v"),
        })
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
        .map(|(asset, context)| {
            let price = num(context, "markPx");
            let prev_day = num(context, "prevDayPx");
            SymbolRow {
                name: text(asset, "name"),
                price,
                change_pct: change_pct(price, prev_day),
                volume: num(context, "dayNtlVlm"),
                funding_pct: num(context, "funding") * 100.0,
                leverage: asset
                    .get("maxLeverage")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                open_interest: num(context, "openInterest"),
                prev_day,
            }
        })
        .collect();
    rows.sort_by(|left, right| right.volume.total_cmp(&left.volume));
    rows
}

fn change_pct(price: f64, prev_day: f64) -> f64 {
    if prev_day > 0.0 {
        (price - prev_day) / prev_day * 100.0
    } else {
        0.0
    }
}

pub async fn hl_symbols() -> Result<Vec<SymbolRow>, HlError> {
    Ok(parse_symbols(
        &info(json!({ "type": "metaAndAssetCtxs" })).await?,
    ))
}

/// Re-prices the market list without re-reading the universe. `allMids` is one
/// flat `coin -> price` map and costs a tenth of `metaAndAssetCtxs`, which is
/// what lets prices tick every few seconds while the 24h figures behind them
/// refresh on a slow timer. Row order is left alone, so the sidebar does not
/// reshuffle under the cursor between volume refreshes.
///
/// An empty list means nothing has loaded yet, so this reads the universe
/// instead: the caller polls one function and the list heals itself after a
/// failed first load.
pub async fn hl_mids(rows: Vec<SymbolRow>) -> Result<Vec<SymbolRow>, HlError> {
    if rows.is_empty() {
        return hl_symbols().await;
    }
    let mids = info(json!({ "type": "allMids" })).await?;
    Ok(apply_mids(rows, &mids))
}

fn apply_mids(rows: Vec<SymbolRow>, mids: &Value) -> Vec<SymbolRow> {
    rows.into_iter()
        .map(|row| {
            // A market the map does not mention, or quotes unreadably, keeps
            // the price it already had rather than dropping to zero.
            let mid = num(mids, &row.name);
            let price = if mid > 0.0 { mid } else { row.price };
            SymbolRow {
                change_pct: change_pct(price, row.prev_day),
                price,
                ..row
            }
        })
        .collect()
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
    Account {
        value: num(&summary, "accountValue"),
        pnl: positions.iter().map(|position| position.pnl).sum(),
        margin_used: num(&summary, "totalMarginUsed"),
        withdrawable: num(value, "withdrawable"),
        notional: num(&summary, "totalNtlPos"),
        maintenance: num(value, "crossMaintenanceMarginUsed"),
        positions,
    }
}

pub async fn hl_account(address: String) -> Result<Account, HlError> {
    Ok(parse_account(
        &info(json!({ "type": "clearinghouseState", "user": address })).await?,
    ))
}

fn parse_fills(value: &Value) -> Vec<Fill> {
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

/// The book renders newest-first from the top, so the asks are reversed here
/// and the view just walks both lists.
pub async fn hl_book(coin: String) -> Result<Book, HlError> {
    let mut book = parse_book(&info(json!({ "type": "l2Book", "coin": coin })).await?);
    book.asks.reverse();
    Ok(book)
}

pub async fn hl_fills(address: String) -> Result<Vec<Fill>, HlError> {
    Ok(parse_fills(
        &info(json!({ "type": "userFills", "user": address })).await?,
    ))
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

/// The newest fills across every market, capped so the list builds a bounded
/// number of rows however long the account's history is.
pub fn recent_fills(rows: Vec<Fill>, limit: i64) -> Vec<Fill> {
    let mut rows = rows;
    rows.sort_by_key(|fill| std::cmp::Reverse(fill.ts));
    rows.truncate(limit.max(0) as usize);
    rows
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

/// The chart for the selected market, with this account's fills marked on it
/// and its levels drawn across it.
pub fn chart(
    tape: &Tape,
    fills: &[Fill],
    positions: &[Position],
    orders: &[Order],
    coin: &str,
) -> Element<'static, Option<CandleHit>> {
    let chart = candle_chart_shared(tape.candles.clone(), &chart_theme())
        .height(Length::Fill)
        .moving_averages([20, 60])
        .price_lines(position_lines(positions, coin))
        .price_lines(order_lines(orders, coin))
        .markers(fill_markers(fills, coin))
        .on_hover(|hit| hit);
    accessible(chart, StableId::new("trading-chart"), Role::Image)
        .label("Hyperliquid candlestick chart with this account's fills marked")
        .logical_id("trading-chart")
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let fills = parse_fills(&json!([
            { "coin": "BTC", "px": "64000.0", "sz": "0.1", "side": "B", "time": 1_786_092_480_123i64, "closedPnl": "0.0", "dir": "Open Long" },
            { "coin": "BTC", "px": "64500.0", "sz": "0.1", "side": "A", "time": 1_786_092_540_000i64, "closedPnl": "50.0", "dir": "Close Long" },
        ]));

        assert!(fills[0].buy);
        assert_eq!(fills[0].ts, 1_786_092_480, "chart timestamps are seconds");
        assert!(!fills[1].buy);
        assert_eq!(fills[1].closed_pnl, 50.0);
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

    #[test]
    fn the_fills_list_is_newest_first_and_bounded() {
        let fill = |ts: i64| Fill {
            coin: "BTC".into(),
            ts,
            price: 1.0,
            size: 1.0,
            buy: true,
            closed_pnl: 0.0,
            action: String::new(),
            fee: 0.0,
        };
        let rows = recent_fills(vec![fill(10), fill(30), fill(20)], 2);
        assert_eq!(rows.len(), 2, "the list is capped");
        assert_eq!(rows[0].ts, 30, "newest first");
        assert_eq!(rows[1].ts, 20);
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
            assert!(btc.prev_day > 0.0, "the 24h change needs yesterday's close");

            // The cheap poll re-prices those same rows in place.
            let repriced = hl_mids(symbols.clone()).await.expect("mid prices");
            assert_eq!(repriced.len(), symbols.len(), "no row is dropped");
            let quoted = repriced
                .iter()
                .zip(&symbols)
                .filter(|(fresh, stale)| fresh.price != stale.price)
                .count();
            assert!(quoted > 0, "allMids named none of the markets we hold");
            let btc = repriced.iter().find(|row| row.name == "BTC").expect("BTC");
            assert!(btc.price > 0.0 && btc.volume > 0.0, "context survived");

            // An empty list falls back to the universe, so the sidebar heals.
            assert!(!hl_mids(Vec::new()).await.expect("fallback").is_empty());

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

            let book = hl_book("BTC".into()).await.expect("order book");
            assert!(!book.bids.is_empty() && !book.asks.is_empty());
            assert!(book.spread > 0.0, "a live book has a spread");
            assert!(
                book.asks
                    .last()
                    .is_some_and(|best| best.price > book.bids[0].price),
                "the asks are reversed so the best ask sits against the spread"
            );

            // A vault address: reachable, and its summary parses.
            let account = hl_account("0xdfc24b077bc1425ad1dea75bcb6f8158e10df303".into())
                .await
                .expect("clearinghouse state");
            assert!(account.value > 0.0, "the HLP vault holds a balance");
        });
    }

    #[test]
    fn a_mid_reprices_a_row_without_disturbing_its_context() {
        let row = |name: &str, price: f64| SymbolRow {
            name: name.into(),
            price,
            change_pct: 0.0,
            volume: 1.0,
            funding_pct: 0.0,
            leverage: 40.0,
            open_interest: 0.0,
            prev_day: 100.0,
        };
        // `allMids` quotes every venue in one map: perps by ticker, spot as
        // "@n", prediction markets as "#n". Only the tickers we hold matter.
        let mids = json!({ "BTC": "110.0", "@1": "16.2", "SOL": "not a number" });
        let rows = apply_mids(vec![row("BTC", 100.0), row("SOL", 50.0)], &mids);

        assert_eq!(rows[0].price, 110.0);
        assert_eq!(
            rows[0].change_pct, 10.0,
            "re-derived from yesterday's close"
        );
        assert_eq!(rows[0].volume, 1.0, "24h context is left for the slow poll");
        assert_eq!(
            rows[1].price, 50.0,
            "an unreadable quote keeps the old price"
        );
        assert_eq!(
            apply_mids(vec![row("DOGE", 7.0)], &mids)[0].price,
            7.0,
            "a market the map omits keeps the price it had"
        );
    }

    /// Hyperliquid meters `info` by request weight, not request count: 1200
    /// per minute per IP. The poll cadence lives in `app.ice`, so this reads
    /// it back out and prices it, charging the documented worst case for
    /// every paged response. Adding a request to a timer, or speeding one up,
    /// fails here rather than in production with a 429.
    #[test]
    fn polling_stays_inside_the_rate_limit() {
        const APP: &str = include_str!("ui/app.ice");
        const BUDGET: u32 = 1_200;

        /// `l2Book`, `allMids`, and `clearinghouseState` cost 2; every other
        /// documented request costs 20; paged responses add one per page.
        fn weight(call: &str) -> u32 {
            match call {
                "hl_book" | "hl_mids" | "hl_account" => 2,
                "hl_symbols" | "hl_orders" => 20,
                // One extra per 60 candles, and the poll right after a market
                // switch is a full backfill.
                "hl_candles" => 20 + BACKFILL_BARS as u32 / 60,
                // One extra per 20 fills; the endpoint returns at most 2000.
                "hl_fills" => 20 + 2_000 / 20,
                other => panic!("{other} has no published weight"),
            }
        }

        // Which fetches each handler fires, straight from the handler bodies.
        let mut owner = "";
        let mut calls: Vec<(&str, &str)> = Vec::new();
        for line in APP.lines() {
            if let Some(name) = line.strip_prefix("on ") {
                owner = name.split('(').next().unwrap_or(name).trim();
            } else if let Some(call) = line.trim().strip_prefix("run ") {
                calls.push((owner, call.split('(').next().unwrap_or(call)));
            }
        }

        let timers: Vec<(u32, &str)> = APP
            .lines()
            .filter_map(|line| line.trim().strip_prefix("every "))
            .map(|timer| {
                let (period, route) = timer.split_once("s ").expect("a seconds cadence");
                let handler = route.rsplit("-> ").next().expect("a route").trim();
                (period.parse().expect("whole seconds"), handler)
            })
            .collect();
        assert!(!timers.is_empty(), "the subscribe block did not parse");

        let mut total = 0;
        for (period, handler) in timers {
            let per_poll: u32 = calls
                .iter()
                .filter(|(fired_by, _)| *fired_by == handler)
                .map(|(_, call)| weight(call))
                .sum();
            assert!(per_poll > 0, "timer for {handler} fetches nothing");
            // Round the rate up, so a cadence that does not divide the minute
            // is charged the busier of the two rates it alternates between.
            total += per_poll * 60u32.div_ceil(period);
        }
        assert!(
            total <= BUDGET,
            "polling costs {total} weight/minute against a {BUDGET} budget"
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
                prev_day: 1.0,
            },
            SymbolRow {
                name: "ETH".into(),
                price: 1.0,
                change_pct: 0.0,
                volume: 0.0,
                funding_pct: 0.0,
                leverage: 25.0,
                open_interest: 0.0,
                prev_day: 1.0,
            },
        ];
        assert_eq!(filter_symbols(rows.clone(), " et ".into()).len(), 1);
        assert_eq!(filter_symbols(rows.clone(), "".into()).len(), 2);
        assert_eq!(filter_symbols(rows, "doge".into()).len(), 0);
    }
}
