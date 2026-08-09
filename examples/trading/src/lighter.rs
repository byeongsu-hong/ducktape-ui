//! Lighter read into the model `hyperliquid` defines, so a row from either
//! venue means the same thing to the panel that draws it. The reads are a REST
//! API with query strings rather than one POST endpoint, and the live feed is
//! a websocket of its own shape; both mix JSON strings with JSON numbers in
//! the same object — sometimes for the same quantity — so every field goes
//! through the same tolerant readers `hyperliquid` uses rather than a derive.
//!
//! Every unit that could be read two ways was settled against live responses
//! rather than guessed, and each one carries the identity that proved it.

// Read-only and complete, but nothing points at it until the venue switch
// lands.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use smol::channel::{Receiver, Sender};
use tungstenite::stream::MaybeTlsStream;

use crate::hyperliquid::{
    Account, Book, Candle, Event, HlError, Level, MarketTick, Order, Position, SymbolRow, Tape,
    Trade, merge, older_than, wire_is_open,
};
use crate::lighter_sign::{self, PrivateKey, Resting, Transaction};
#[cfg(test)]
use crate::signing::Wallet;
use crate::venue::lighter_buy;

/// Which Lighter deployment a read is addressed to.
///
/// The same reason Hyperliquid's reads carry a `Chain`: the app reads two
/// deployments and one of them is the one where an order costs nothing to get
/// wrong. They are separate books with separate accounts and separate market
/// ids — read live, testnet lists BTC as market 1 and mainnet lists 222
/// markets — so nothing derived from one is meaningful against the other, and
/// a default here would be a deployment chosen by whoever forgot to pass one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Mainnet,
    Testnet,
}

impl Zone {
    pub fn api_url(self) -> &'static str {
        match self {
            Zone::Mainnet => "https://mainnet.zklighter.elliot.ai/api/v1",
            Zone::Testnet => "https://testnet.zklighter.elliot.ai/api/v1",
        }
    }

    pub fn stream_url(self) -> &'static str {
        match self {
            Zone::Mainnet => "wss://mainnet.zklighter.elliot.ai/stream",
            Zone::Testnet => "wss://testnet.zklighter.elliot.ai/stream",
        }
    }

    pub fn testnet(self) -> bool {
        matches!(self, Zone::Testnet)
    }

    /// The number the sequencer stamps into every transaction digest.
    ///
    /// It is what makes a signature belong to one deployment: the chain id
    /// leads the hashed elements, so a transaction signed for the test
    /// sequencer recovers nobody on the live one. That is the same job
    /// `Chain`'s phantom-agent `source` does on the other venue, and it is why
    /// the transaction builders take a `Zone` rather than a bare number — a
    /// caller cannot post to one deployment while signing for the other.
    pub fn chain_id(self) -> u32 {
        match self {
            Zone::Mainnet => 304,
            Zone::Testnet => 300,
        }
    }

    /// A short, stable name for this deployment, for anything that has to file
    /// something under it. The same rule `Chain::key` follows: not for a
    /// reader, and deliberately not the wire spelling, so renaming a keychain
    /// item can never change what a signature says.
    pub fn key(self) -> &'static str {
        match self {
            Zone::Mainnet => "mainnet",
            Zone::Testnet => "testnet",
        }
    }
}
const TIMEOUT: Duration = Duration::from_secs(15);

/// Margin fractions arrive as integers out of 10_000.
///
/// Established, not assumed: for account 702384 the six position values times
/// their markets' `maintenance_margin_fraction` over 10_000 sum to
/// 3084.930401, which is `cross_maintenance_margin_requirement` to the last
/// published digit. Reading them as percent or as a raw fraction misses by
/// two orders of magnitude either way, so the identity fixes the scale with no
/// room left over. `min_initial_margin_fraction` shares it: BTC's 200 is 2%,
/// which is the 50x the venue advertises.
const BASIS: f64 = 10_000.0;

/// What every Lighter perp margins in. One token across the whole universe:
/// `quote_asset_id` is 0 on all 222 of them, and the venue has no second
/// clearinghouse to settle anything anywhere else.
const LIGHTER_COLLATERAL: &str = "USDC";

/// `/funding-rates` publishes every venue's rate over the same eight-hour
/// window, and `SymbolRow.funding_pct` is the hourly figure `fmt_funding`
/// labels.
///
/// Re-established rather than inherited: comparing the payload's own
/// `hyperliquid` rows against Hyperliquid's `metaAndAssetCtxs` in the same
/// minute, across the 93 markets both quote with a non-zero rate the ratio has
/// a median of exactly 8.0 with both quartiles on 8.0, and 62 of the 93 land
/// on 8.0 to the last digit. The rest sit between 7.41 and 8.91, which is the
/// two venues having repriced between the snapshots rather than a second
/// window. So Lighter restates an hourly rate over eight hours, and its own
/// row is undone the same way.
const FUNDING_HOURS: f64 = 8.0;

/// The account an L1 address means: `account_type` 0 is the book the address
/// itself trades, and `account?by=l1_address` returns it beside every
/// sub-account the address has opened.
///
/// Established over 85 live addresses drawn from resting orders: 45 of them
/// answer with more than one account, every one of the 85 has exactly one
/// `account_type` 0, and the rest are `account_type` 1 at an index past 2^48.
/// The sub-accounts are not this account's spare change — one sampled address
/// holds 1.81M in its main book and 3.27M in a sub — so they are neither
/// summed nor preferred: each book posts its own collateral and meets its own
/// maintenance requirement, and `Account` carries one equity against one
/// requirement. Merging them would draw a health rail for a portfolio no
/// margin engine margins, since either book can liquidate while the other is
/// comfortable.
const MAIN_ACCOUNT: i64 = 0;

/// Mirrors of `hyperliquid`'s private layout constants — that module publishes
/// no `pub const`, so these cannot be imported. The rails and depth bars are
/// the view's geometry rather than either venue's, so a row from here has to
/// be drawn against the same widths or the two venues render at different
/// scales. `the_view_geometry_matches_what_hyperliquid_draws` pins two of the
/// three to numbers that module itself produces.
const RISK_RAIL_WIDTH: f64 = 80.0;
const BOOK_DEPTH: usize = 10;
const BOOK_BAR_WIDTH: f64 = 196.0;

/// `orderBookOrders` returns individual resting orders, not price levels, so
/// the depth the book shows costs more rows than it draws. Live BTC collapses
/// roughly two orders into every level; this leaves room for far worse.
const ORDER_FETCH: usize = BOOK_DEPTH * 20;

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            // ureq turns any 4xx or 5xx into `Error::StatusCode` and drops the
            // body with it, but the body is where this venue puts its reason:
            // a malformed query answers HTTP 400 carrying
            // `{"code":20001,"message":"invalid param "}`, and the default
            // would report that as the bare number 400. Reading the status
            // here instead is what lets `get` quote the venue.
            .http_status_as_error(false)
            .build()
            .into()
    })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn fail(message: String) -> HlError {
    HlError { message }
}

/// Lighter's verdict on a read that did answer, which is a different thing
/// from the read having failed.
///
/// `21100 account not found` is the whole reason this is a number rather than
/// a boolean: an address with no account here is a fact about the address, not
/// a broken request, and the account read is the one call this app makes that
/// the venue can truthfully answer "nothing". Live, `account?by=l1_address&
/// value=0x8cc94dc843e1ea7a19805e0cca43001123512b6a` — an address with a real
/// Hyperliquid account — comes back HTTP 400 with
/// `{"code":21100,"message":"account not found"}`, while a malformed address
/// comes back `21103 invalid account l1 address`, which is a failure.
const OK: i64 = 200;
const NO_ACCOUNT: i64 = 21100;

/// One GET off the UI thread, answering the venue's own verdict beside the
/// body it arrived with. Two shapes of refusal were seen live and both are
/// worth quoting: a path the REST surface does not serve answers 403 with an
/// empty body, so the status is all there is, and a bad parameter answers 400
/// with a `code` and a `message`. A body that parsed is still not a body that
/// succeeded, since the venue also carries its own `code` on a 200 — so the
/// code is what this answers, and a status that failed with nothing to say
/// stands in as one.
async fn read(zone: Zone, path: String) -> Result<(i64, Value), HlError> {
    // The same gate the socket passes, and the same reason: a test drives the
    // real program, so a handler that reads the universe would read it from the
    // live venue and assert against whatever it happened to hold.
    if !wire_is_open() {
        return Err(fail("Lighter unreachable: no wire under test".to_owned()));
    }
    smol::unblock(move || {
        let url = format!("{}/{path}", zone.api_url());
        let mut response = agent()
            .get(&url)
            .call()
            .map_err(|error| fail(format!("Lighter unreachable: {error}")))?;
        let status = response.status();
        let body = response
            .body_mut()
            .read_json::<Value>()
            .map_err(|error| fail(format!("Lighter answered {path} with {status}: {error}")))?;
        let stated = body.get("code").and_then(Value::as_i64);
        Ok((
            stated.unwrap_or(if status.is_success() {
                OK
            } else {
                i64::from(status.as_u16())
            }),
            body,
        ))
    })
    .await
}

/// The same read with every verdict but success as a failure, which is what
/// all but the account read wants: nothing else this app asks Lighter for has
/// an answer that means "there is nothing here".
async fn get(zone: Zone, path: String) -> Result<Value, HlError> {
    match read(zone, path.clone()).await? {
        (OK, body) => Ok(body),
        (code, body) => Err(refused(&path, code, &body)),
    }
}

fn refused(path: &str, code: i64, body: &Value) -> HlError {
    fail(format!(
        "Lighter refused {path}: code {code} {}",
        text(body, "message")
    ))
}

/// Lighter sends prices and sizes as strings and volumes and changes as
/// numbers, and which is which varies by endpoint. Same tolerance as the other
/// venue: unreadable is zero rather than a failed response.
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

fn value_i64(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default()
}

/// Yesterday's close, backed out of the move the venue publishes.
///
/// `daily_price_change` is already a percentage. Read as a fraction instead,
/// the implied previous price falls outside the market's own 24h high and low
/// on all but 3 of the 205 active markets, and inside it on 163 when read as
/// percent — and the price 24 hours ago is inside the last 24 hours' range by
/// construction. BTC settles it alone: 0.6118 against a mark of 64975.3 and a
/// range of 64270.5 to 65349.4 only lands as a percentage.
fn previous_price(price: f64, change_pct: f64) -> f64 {
    let scale = 1.0 + change_pct / 100.0;
    if price > 0.0 && scale > 0.0 {
        price / scale
    } else {
        0.0
    }
}

/// Maximum leverage is the reciprocal of the smallest initial margin the venue
/// will open a position at, which is the same statement Hyperliquid publishes
/// directly as `maxLeverage`.
fn max_leverage(min_initial_bp: f64) -> f64 {
    if min_initial_bp > 0.0 {
        BASIS / min_initial_bp
    } else {
        0.0
    }
}

fn parse_symbol(detail: &Value, funding: &HashMap<i64, f64>) -> SymbolRow {
    let price = num(detail, "mark_price");
    let change_pct = num(detail, "daily_price_change");
    SymbolRow {
        name: text(detail, "symbol"),
        // Lighter keys a book by `market_id`, which this deployment's own
        // universe supplies; the Hyperliquid universe index this field carries
        // is not a thing Lighter has. The order path does not read it either:
        // it resolves the id and the two step counts together out of
        // `orderBookDetails`, because a price is sent as a count of the
        // market's own steps and the two must come from one row.
        asset: 0,
        price,
        change_pct,
        // The quote-denominated leg, which is what Hyperliquid's `dayNtlVlm`
        // is; the base leg is published beside it and is a different number.
        volume: num(detail, "daily_quote_token_volume"),
        // Positive means longs pay shorts on both venues, so no sign work is
        // needed to match `hyperliquid`'s `funding * 100`. Settled live rather
        // than assumed: `/fundings` carries the payer in a `direction` field
        // beside an unsigned rate, and over 45 markets the sign here agreed
        // with that direction on 44. The one that disagreed had flipped since
        // its last settlement, which is what a rate quoted for the next one
        // does.
        funding_pct: funding
            .get(&value_i64(detail, "market_id"))
            .map_or(0.0, |rate| rate / FUNDING_HOURS * 100.0),
        leverage: max_leverage(num(detail, "min_initial_margin_fraction")),
        open_interest: num(detail, "open_interest"),
        prev: previous_price(price, change_pct),
        maintenance: num(detail, "maintenance_margin_fraction") / BASIS,
        // The venue's own step for this market, which is the same statement
        // Hyperliquid makes as `szDecimals`. Live: BTC 5, ETH 4, SOL 3, and a
        // market that publishes none quotes whole units, so a size worked out
        // by the app floors onto whole units rather than onto a step the venue
        // would refuse.
        size_decimals: value_i64(detail, "size_decimals").max(0) as usize,
        // Lighter lists one flat perp universe and says so: read live,
        // `/orderBookDetails` answers with 222 markets whose `market_type` is
        // `perp` and nothing else, and not one of its forty-four fields names
        // a deployer, a builder, a sub-exchange or a second collateral. There
        // is no Lighter equivalent of a HIP-3 dex to reflect, so the rail
        // draws this venue's list with no headings over it rather than
        // inventing one group to sit under a header for symmetry with the
        // other venue.
        category: String::new(),
        collateral: LIGHTER_COLLATERAL.to_owned(),
        heading: false,
        selected: false,
    }
}

/// The tradeable universe, busiest first. Markets keep their row on Lighter
/// with `status` moved off "active", and they are dropped here the way
/// `isDelisted` rows are dropped on the other venue: a market that cannot be
/// traded is worse than absent, because it looks tradeable.
///
/// The status is the whole test, and it has to be. Of the 17 inactive markets
/// live today only 5 are zeroed out; the other 12 still publish a mark price,
/// an index price, open interest and margin fractions — BIRB reads 0.07548
/// against 181620.3 of open interest — so nothing about the numbers tells a
/// retired market from a quiet one.
fn parse_symbols(details: &Value, rates: &Value) -> Vec<SymbolRow> {
    let funding = parse_funding(rates);
    let mut rows: Vec<SymbolRow> = list(details, "order_book_details")
        .iter()
        .filter(|detail| text(detail, "status") == "active")
        .map(|detail| parse_symbol(detail, &funding))
        .collect();
    rows.sort_by(|left, right| right.volume.total_cmp(&left.volume));
    rows
}

/// Lighter republishes the funding rates of the venues it indexes alongside
/// its own, so the rows are filtered down to the one that is actually charged
/// here.
fn parse_funding(rates: &Value) -> HashMap<i64, f64> {
    list(rates, "funding_rates")
        .iter()
        .filter(|rate| text(rate, "exchange") == "lighter")
        .map(|rate| (value_i64(rate, "market_id"), num(rate, "rate")))
        .collect()
}

/// Symbol to market id, which is what every per-market request is keyed by.
/// The mapping is a fact about the venue rather than about the day, so it is
/// resolved once and held: a book request would otherwise pay for the whole
/// 222-market universe every time it ran.
///
/// It is the one piece of venue data that outlives the venue being on screen —
/// nothing in `switch_venue` clears it — and that is safe rather than
/// overlooked. It only ever holds Lighter's own tickers against Lighter's own
/// ids; the other adapter keys its requests by ticker and never reads or
/// writes this. Both writers replace the whole map rather than merging into
/// it, so it cannot come to hold two snapshots at once, and a switch away and
/// back rewrites it from `lighter_symbols` before anything reads it. What is
/// left is one stale entry for a ticker delisted while the terminal was
/// reading the other exchange, and the read that would use it is a book for a
/// market `listed_coin` has already moved the terminal off.
fn ids() -> &'static Mutex<HashMap<String, Market>> {
    static IDS: OnceLock<Mutex<HashMap<String, Market>>> = OnceLock::new();
    IDS.get_or_init(Mutex::default)
}

/// What a request needs to know about a market that its ticker does not say.
///
/// The two step counts ride along with the id because they come out of the
/// same `orderBookDetails` row and are needed at the same moment: an order
/// carries its price and size as integers counted in the market's own steps,
/// so a price without its decimals is a number off by a power of ten. Reading
/// them separately would be a second request for a fact already in hand — and,
/// worse, a chance for the id and the decimals to come from different reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Market {
    pub id: i64,
    /// How finely this market quotes a price. Live on the test deployment,
    /// BTC quotes to one decimal and sizes to five.
    pub price_decimals: u32,
    pub size_decimals: u32,
}

fn parse_ids(details: &Value) -> HashMap<String, Market> {
    list(details, "order_book_details")
        .iter()
        .map(|detail| {
            (
                text(detail, "symbol"),
                Market {
                    id: value_i64(detail, "market_id"),
                    price_decimals: value_i64(detail, "price_decimals").clamp(0, 18) as u32,
                    size_decimals: value_i64(detail, "size_decimals").clamp(0, 18) as u32,
                },
            )
        })
        .collect()
}

pub async fn lighter_symbols(zone: Zone) -> Result<Vec<SymbolRow>, HlError> {
    let details = get(zone, "orderBookDetails".to_owned()).await?;
    // Free, since the universe is already in hand.
    *lock(ids()) = parse_ids(&details);
    let rates = get(zone, "funding-rates".to_owned()).await?;
    Ok(parse_symbols(&details, &rates))
}

/// The market a ticker trades as, fetching the universe only if nothing has
/// yet.
async fn market_of(zone: Zone, coin: &str) -> Result<Market, HlError> {
    if let Some(market) = lock(ids()).get(coin) {
        return Ok(*market);
    }
    let details = get(zone, "orderBookDetails".to_owned()).await?;
    let fresh = parse_ids(&details);
    let found = fresh.get(coin).copied();
    *lock(ids()) = fresh;
    found.ok_or_else(|| fail(format!("Lighter does not list {coin}")))
}

/// One side of the book. Lighter serves resting orders rather than price
/// levels, so orders sharing a price are one level here — three makers queued
/// at the same tick are one thing to trade against and three rows on the wire.
/// Both sides arrive sorted best-first, so the fold only has to look at the
/// row it just wrote.
fn parse_levels(side: &[Value]) -> Vec<Level> {
    let mut rows: Vec<Level> = Vec::new();
    let mut total = 0.0;
    for order in side {
        // What is left of the order, not what it was placed for: the filled
        // part is not depth anybody can trade against.
        let size = num(order, "remaining_base_amount");
        let price = num(order, "price");
        if size <= 0.0 {
            continue;
        }
        match rows.last_mut() {
            Some(level) if level.price == price => {
                level.size += size;
                total += size;
                level.total = total;
            }
            _ => {
                if rows.len() == BOOK_DEPTH {
                    break;
                }
                total += size;
                rows.push(Level {
                    price,
                    size,
                    total,
                    bar: 0.0,
                });
            }
        }
    }
    scale_bars(&mut rows);
    rows
}

/// The depth bar behind each row, as a share of the deepest level shown. The
/// REST book and the streamed one both draw it, so the arithmetic is here once
/// rather than in each.
fn scale_bars(rows: &mut [Level]) {
    let deepest = rows.last().map_or(0.0, |level| level.total);
    if deepest > 0.0 {
        for level in rows {
            level.bar = level.total / deepest * BOOK_BAR_WIDTH;
        }
    }
}

/// Two sides, each nearest price first, into the book the panel reads. Shared
/// for the same reason `scale_bars` is: the spread and the mid are the panel's
/// reading of a book rather than either source's, so a book read over REST and
/// one folded from the stream cannot answer them differently.
fn book_of(bids: Vec<Level>, asks: Vec<Level>) -> Book {
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

fn parse_book(value: &Value) -> Book {
    book_of(
        parse_levels(list(value, "bids")),
        parse_levels(list(value, "asks")),
    )
}

/// The book, keyed by ticker rather than by market id so it reads like the
/// other venue's. The asks come back best-first; the panel reverses them
/// itself, the same way it does for the feed it already has.
pub async fn lighter_book(zone: Zone, coin: String) -> Result<Book, HlError> {
    let id = market_of(zone, &coin).await?.id;
    Ok(parse_book(
        &get(
            zone,
            format!("orderBookOrders?market_id={id}&limit={ORDER_FETCH}"),
        )
        .await?,
    ))
}

/// Copies of `hyperliquid`'s private rail arithmetic. Both are the reading the
/// view draws rather than anything a venue reports, so they have to agree
/// exactly or the same risk renders two lengths.
fn liquidation_travel(entry: f64, mark: f64, liquidation: f64) -> f64 {
    let span = liquidation - entry;
    if !(span.is_finite() && span.abs() > f64::EPSILON) || liquidation <= 0.0 {
        return 0.0;
    }
    ((mark - entry) / span).clamp(0.0, 1.0)
}

fn margin_load(equity: f64, maintenance: f64) -> f64 {
    if maintenance <= 0.0 {
        return 0.0;
    }
    if equity <= 0.0 {
        return 1.0;
    }
    (maintenance / equity).clamp(0.0, 1.0)
}

/// A position's `initial_margin_fraction` is a percentage, and it is a
/// different unit from the market's identically named basis-point field.
///
/// Established the same way: for account 702384 the position values times this
/// number over 100 sum to 84253.253098, which is the reported
/// `cross_initial_margin_requirement` exactly. Every position on that account
/// reads "33.33", which is 3x — as basis points it would be 300x, which the
/// venue does not offer on any market.
fn position_leverage(initial_pct: f64) -> f64 {
    if initial_pct > 0.0 {
        100.0 / initial_pct
    } else {
        0.0
    }
}

fn parse_position(value: &Value) -> Position {
    // `position` is unsigned and `sign` carries the direction, so neither
    // field alone is the size the panel means.
    let size = num(value, "position") * num(value, "sign");
    let notional = num(value, "position_value");
    let entry = num(value, "avg_entry_price");
    let liq = num(value, "liquidation_price");
    let initial_pct = num(value, "initial_margin_fraction");
    let leverage = position_leverage(initial_pct);
    // The venue reports what the position is worth, not what it is marked at.
    let mark = if size == 0.0 {
        0.0
    } else {
        notional / size.abs()
    };
    let pnl = num(value, "unrealized_pnl");
    // What the position was opened against, which is what a return on equity
    // divides by — and the same quantity `mark_positions` recomputes from
    // entry, size, and leverage between polls, so the two stay consistent.
    let opening = entry * size.abs() * initial_pct / 100.0;
    // An isolated position posts its own collateral and the venue names it;
    // a cross one ties up its share of the account's requirement instead, and
    // that share is exactly what the account's total is the sum of.
    let allocated = num(value, "allocated_margin");
    // `margin_mode` 0 is cross and 1 is isolated, which only one of the two
    // partitions can be. Re-checked live on the two sampled accounts that hold
    // both: summing position value times each market's maintenance fraction
    // over the mode-0 positions alone reproduces the reported
    // `cross_maintenance_margin_requirement` to the last published digit —
    // 1712.597478 on account 270812 and 92911.691340 on 102394 — while summing
    // over every open position overshoots to 2255.06 and 94072.94.
    let cross = value_i64(value, "margin_mode") == 0;
    Position {
        coin: text(value, "symbol"),
        size,
        entry,
        mark,
        liq,
        pnl,
        roe_pct: if opening > 0.0 {
            pnl / opening * 100.0
        } else {
            0.0
        },
        margin: if cross {
            notional * initial_pct / 100.0
        } else {
            allocated
        },
        risk: liquidation_travel(entry, mark, liq) * RISK_RAIL_WIDTH,
        leverage,
        // The strings the shared arithmetic already branches on: `mark_account`
        // sums the PnL of everything reading "cross".
        margin_mode: if cross { "cross" } else { "isolated" }.to_owned(),
        // Negated, because the two venues report opposite quantities under one
        // field, and left alone the same funding bill would read as income
        // here and as a cost on the other venue.
        //
        // `hyperliquid` fills this from `cumFunding.sinceOpen`, which is
        // funding *charged*: across 13 positions on 6 live addresses that
        // field equalled the negated sum of the account's own `userFunding`
        // events, and in 500 of 500 of those events a negative `usdc` was the
        // account paying. Lighter's `total_funding_paid_out` is the cash flow
        // itself, so paying reads negative. Watched across one settlement
        // rather than argued: over the 08:00 hour, 52 of 52 open positions
        // whose size did not change moved this field the way that reading
        // predicts — every long in a market charging longs more negative,
        // every short more positive — with none moving the other way.
        funding: -num(value, "total_funding_paid_out"),
    }
}

/// The address's own trading book, picked out of however many accounts the
/// address has. Selected by `account_type` rather than by position: the main
/// account came first in all 85 responses sampled, but the venue documents no
/// ordering, and reading position 0 would hand the panel a sub-account's
/// equity the first time that held.
fn parse_account(value: &Value) -> Account {
    parse_account_body(
        list(value, "accounts")
            .iter()
            .find(|account| value_i64(account, "account_type") == MAIN_ACCOUNT)
            .unwrap_or(&Value::Null),
    )
}

fn parse_account_body(account: &Value) -> Account {
    // Lighter keeps a row for every market the account has ever traded, so a
    // closed position stays in the list at size zero rather than leaving it.
    // These are the majority of what the endpoint returns: 660 of the 1213
    // rows across 25 live accounts, with one account carrying 56 of them and
    // no open position at all. Size is the whole test because it is the only
    // thing that separates them — all 660 also read zero for entry price and
    // position value — and a row that is nothing to hold and nothing to close
    // would only bury the positions that are real.
    let positions: Vec<Position> = list(account, "positions")
        .iter()
        .map(parse_position)
        .filter(|position| position.size != 0.0)
        .collect();
    // Equity the cross engine measures, which excludes what is posted behind
    // isolated positions. Verified as the venue's own split rather than
    // derived here: on account 702384, with nothing isolated, `total_asset_value`
    // and `cross_asset_value` agree, and both equal `collateral` plus the
    // summed unrealized PnL.
    let cross_value = num(account, "cross_asset_value");
    let maintenance = num(account, "cross_maintenance_margin_requirement");
    Account {
        value: num(account, "total_asset_value"),
        cross_value,
        pnl: positions.iter().map(|position| position.pnl).sum(),
        // Taken as published, because it is not reconstructable here. It is
        // `total_asset_value` less `cross_initial_margin_requirement` to the
        // cent on all 8 sampled accounts that hold nothing isolated, and on
        // neither of the 2 that do — account 270812 is 903.81 under that
        // figure and 102394 is 7965.86 under it, against 1012.27 and 8141.64
        // of allocated margin. Isolated collateral clearly moves it, but not
        // by an amount these fields pin down, so the venue's own number stands.
        withdrawable: num(account, "available_balance"),
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

/// The account behind an L1 address, or nothing when the address has none
/// here. Lighter also keys accounts by its own index, but an address is what
/// the app already asks the reader for and what the other venue takes.
///
/// The absence is an answer rather than a failure, and this is the only read
/// that can tell the difference. One address is typed once and read at
/// whichever venue is on screen, so an address that trades on the other
/// exchange and has never opened an account here is the ordinary case — drawn
/// as an error it would put "Lighter refused" over a screen that is working,
/// and hide the true and useful thing, which is that there is nothing here to
/// draw.
pub async fn lighter_account(zone: Zone, address: String) -> Result<Option<Account>, HlError> {
    let path = format!("account?by=l1_address&value={address}");
    match read(zone, path.clone()).await? {
        (OK, body) => Ok(Some(parse_account(&body))),
        (NO_ACCOUNT, _) => Ok(None),
        (code, body) => Err(refused(&path, code, &body)),
    }
}

// The order path: complete, held to the venue's own answers, and pointed at by
// nothing until the ticket is wired to it — the shape `hyperliquid.rs` states
// for its own writes, and for the same reason: built and proven before a button
// can spend money with it.

/// How long a signed transaction stays submittable. Ten minutes, which is what
/// the venue's own SDK gives one.
///
/// It bounds a *transaction*, not an order: a signed body somebody copied off
/// the wire stops being worth replaying after this, and the order it placed
/// keeps resting regardless. The two deadlines are separate fields for that
/// reason and are never one value.
const TX_LIFETIME_MS: i64 = 10 * 60 * 1_000;

/// How long a placed order rests before the venue drops it. 28 days, the
/// venue's SDK default, and required to be *something*: a good-till-time order
/// with no expiry is refused, and this app places no other kind.
const ORDER_LIFETIME_MS: i64 = 28 * 24 * 60 * 60 * 1_000;

/// The verdicts `/sendTx` answers with, each read live off the test deployment
/// and each meaning something different to a reader.
///
/// The ladder matters more than any one rung. The venue checks the body's shape
/// first, then the account, then the key, then the signature — so a refusal
/// names how far a submission got, and `KEY_UNKNOWN` in particular means every
/// field was read and accepted and only the enrolment was missing. That is what
/// makes an unregistered key useful evidence: it is the venue confirming it
/// parses what this module writes.
const BAD_TX_INFO: i64 = 21501;
const NO_ACCOUNT_HERE: i64 = 21100;
const KEY_UNKNOWN: i64 = 21109;
const BAD_SIGNATURE: i64 = 21120;

/// The next transaction number for one API key.
///
/// A nonce is per key rather than per account, which is why both indices are in
/// the query: two keys on one account count separately, and reusing a number is
/// how a replay is refused.
///
/// It is not evidence the account exists. Read live, `nextNonce` answers
/// `{"code":200,"nonce":0}` for an account index nobody has ever opened — so
/// nothing may infer an account from a nonce, and the account read is still the
/// only thing that answers that question.
pub async fn lighter_nonce(zone: Zone, account: i64, api_key: u8) -> Result<i64, HlError> {
    let body = get(
        zone,
        format!("nextNonce?account_index={account}&api_key_index={api_key}"),
    )
    .await?;
    body.get("nonce")
        .and_then(Value::as_i64)
        .ok_or_else(|| fail("Lighter answered the nonce request without a nonce in it".to_owned()))
}

/// Post one signed transaction, and answer the hash the sequencer filed it
/// under.
///
/// Behind the same `wire_is_open` gate every read passes, and for the reason
/// that gate exists at all: a test drives the real program, so a path that
/// spends money must not be the one path a test can reach.
async fn send_tx(zone: Zone, tx: &Transaction, key: &PrivateKey) -> Result<String, HlError> {
    if !wire_is_open() {
        return Err(fail("Lighter unreachable: no wire under test".to_owned()));
    }
    let url = format!("{}/sendTx", zone.api_url());
    let tx_type = tx.tx_type().to_string();
    // Signed here rather than inside `unblock`, so the key never crosses onto
    // another thread — what does is 80 bytes of signature and a JSON body.
    let tx_info = tx.signed(key);
    let (code, body) = smol::unblock(move || {
        let mut response = agent()
            // The venue reads a form, not a JSON body, and the signature is
            // base64 — whose `+` and `/` are exactly the characters a naive
            // body would corrupt. `send_form` percent-encodes both fields.
            .post(&url)
            .send_form([("tx_type", tx_type.as_str()), ("tx_info", tx_info.as_str())])
            .map_err(|error| fail(format!("Lighter unreachable: {error}")))?;
        let status = response.status();
        let body = response
            .body_mut()
            .read_json::<Value>()
            .map_err(|error| fail(format!("Lighter answered sendTx {status}: {error}")))?;
        let stated = body.get("code").and_then(Value::as_i64);
        Ok::<_, HlError>((
            stated.unwrap_or(if status.is_success() {
                OK
            } else {
                i64::from(status.as_u16())
            }),
            body,
        ))
    })
    .await?;
    submitted(code, &body)
}

/// What the venue made of a submitted transaction.
///
/// The `code` is the verdict, not the HTTP status: every refusal seen live
/// arrives as HTTP 400 carrying its own number and sentence, and the venue puts
/// a `code` on its successes too. Reading the status alone would report a
/// refused order as sent.
///
/// What an acceptance is, exactly: the sequencer took the transaction. It is
/// **not** the order resting. The answer carries a
/// `predicted_execution_time_ms` beside the hash, which is the venue saying so
/// itself — the book is where an order appears, and this is a receipt for a
/// submission. A caller that draws "placed" off this is drawing a claim the
/// venue did not make.
///
/// A `code` of 200 with no hash under it is refused rather than accepted: an
/// acceptance with nothing to point at is not an outcome anybody can act on,
/// and it is the shape a changed response would take.
fn submitted(code: i64, body: &Value) -> Result<String, HlError> {
    if code != OK {
        let said = text(body, "message");
        return Err(fail(if said.is_empty() {
            format!("Lighter refused the transaction: code {code}")
        } else {
            format!("Lighter refused the transaction: code {code} {said}")
        }));
    }
    let hash = text(body, "tx_hash");
    if hash.is_empty() {
        return Err(fail(
            "Lighter accepted the transaction and named no hash for it".to_owned(),
        ));
    }
    Ok(hash)
}

/// A figure on screen as the integer count of the market's own steps.
///
/// Refused rather than rounded, which is the same stance `signing.rs` takes on
/// the other venue's decimal strings: a price quietly rounded onto the tick is
/// a fill nobody asked for, and it is invisible on every screen afterwards.
/// The caller rounds to the market's step before it prices anything; this is
/// the check that it did.
fn stepped(what: &str, value: f64, decimals: u32) -> Result<i64, HlError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(fail(format!("{value} is not a {what} to send")));
    }
    let scale = 10f64.powi(decimals as i32);
    let scaled = value * scale;
    let rounded = scaled.round();
    if (scaled - rounded).abs() > 1e-6 {
        return Err(fail(format!(
            "this market counts its {what} in steps of {}, and {value} is not a whole number of \
             them",
            1.0 / scale,
        )));
    }
    Ok(rounded as i64)
}

/// Place one limit order, and answer the index it was placed under.
///
/// That index is the app's own and is chosen here, because the venue's answer
/// has no order id in it: a submission is acknowledged with a transaction hash,
/// and the only name a later cancel can use is the `ClientOrderIndex` its
/// placer picked. The clock supplies it, which is the same source the other
/// venue's nonce comes from and gives an index that rises and sits well inside
/// the venue's range.
///
/// ponytail: two orders placed inside one millisecond would share an index.
/// A person pressing a button cannot, and the sequencer's nonce is what stops a
/// transaction being replayed regardless — so the counter this would need waits
/// until something places orders in a loop.
#[allow(clippy::too_many_arguments)]
pub async fn lighter_place(
    zone: Zone,
    key: &PrivateKey,
    account: i64,
    api_key: u8,
    coin: &str,
    order: Order,
    reduce_only: bool,
    resting: Resting,
) -> Result<i64, HlError> {
    let market = market_of(zone, coin).await?;
    let now = now_ms();
    let client_index = now;
    let built = lighter_sign::create_order(
        zone,
        &lighter_sign::NewOrder {
            account,
            api_key,
            market: i16::try_from(market.id).map_err(|_| {
                fail(format!(
                    "Lighter numbers {coin} past what an order can name"
                ))
            })?,
            client_index,
            base_amount: stepped("size", order.size, market.size_decimals)?,
            price: u32::try_from(stepped("price", order.price, market.price_decimals)?).map_err(
                |_| {
                    fail(format!(
                        "{} is past the highest price this venue takes",
                        order.price
                    ))
                },
            )?,
            ask: !order.buy,
            reduce_only,
            resting,
            // The venue validates the pairing rather than ignoring it: an
            // order that does not rest must carry no expiry, and one that
            // rests must carry one.
            expiry_ms: if resting.expires() {
                now + ORDER_LIFETIME_MS
            } else {
                0
            },
            deadline_ms: now + TX_LIFETIME_MS,
            nonce: lighter_nonce(zone, account, api_key).await?,
        },
    )
    .map_err(|error| fail(error.to_string()))?;
    send_tx(zone, &built, key).await?;
    Ok(client_index)
}

/// Pull one resting order, by the index it was placed under.
pub async fn lighter_cancel(
    zone: Zone,
    key: &PrivateKey,
    account: i64,
    api_key: u8,
    coin: &str,
    index: i64,
) -> Result<(), HlError> {
    let market = market_of(zone, coin).await?;
    let now = now_ms();
    let built = lighter_sign::cancel_order(
        zone,
        &lighter_sign::Cancel {
            account,
            api_key,
            market: i16::try_from(market.id).map_err(|_| {
                fail(format!(
                    "Lighter numbers {coin} past what an order can name"
                ))
            })?,
            index,
            deadline_ms: now + TX_LIFETIME_MS,
            nonce: lighter_nonce(zone, account, api_key).await?,
        },
    )
    .map_err(|error| fail(error.to_string()))?;
    send_tx(zone, &built, key).await.map(|_| ())
}

/// The account index an L1 address trades under here, or nothing when the
/// address has no account on this deployment.
///
/// Every write is keyed by this rather than by the address — a transaction
/// carries an `AccountIndex` and never an `0x…` — so custody has to resolve it
/// once at unlock and hold it beside the key. It is the venue's answer rather
/// than a derivation: an address opens its account by being funded, and the
/// index it is given is the sequencer's to assign.
pub async fn lighter_account_index(zone: Zone, address: String) -> Result<Option<i64>, HlError> {
    let path = format!("account?by=l1_address&value={address}");
    match read(zone, path.clone()).await? {
        (OK, body) => Ok(list(&body, "accounts")
            .iter()
            .find(|account| value_i64(account, "account_type") == MAIN_ACCOUNT)
            .map(|account| value_i64(account, "account_index"))),
        (NO_ACCOUNT, _) => Ok(None),
        (code, body) => Err(refused(&path, code, &body)),
    }
}

/// Every API key this account has registered, as the index it sits under and
/// the public key registered there.
///
/// This is Lighter's `extraAgents`: the venue's own word on which keys may
/// sign for an account, and the only thing that can turn a key this app
/// generated into one the app may trade with. The index comes back *from* the
/// listing rather than being asked of the reader — the owner registers a public
/// key at whichever slot they like, and the venue then says which.
pub async fn lighter_api_keys(zone: Zone, account: i64) -> Result<Vec<(u8, String)>, HlError> {
    let body = get(zone, format!("apikeys?account_index={account}")).await?;
    Ok(list(&body, "api_keys")
        .iter()
        .filter_map(|key| {
            let index = u8::try_from(value_i64(key, "api_key_index")).ok()?;
            let public = text(key, "public_key");
            (!public.is_empty()).then_some((index, public))
        })
        .collect())
}

/// The most bars one `/candles` call will serve, which is also the window a
/// chart opens on. Enforced by the venue rather than chosen here:
/// `count_back=1000` over a 1000-bar window answers with 500.
const CANDLE_PAGE: i64 = 500;

/// What a loaded tape asks for on a refresh — the bar still forming and enough
/// either side of it that a beat missed while the app was busy is not a hole.
const CANDLE_REFRESH: i64 = 3;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// The venue omits a zero field rather than sending it, which the tolerant
/// readers already do the right thing with: a bar that traded nothing arrives
/// without its `v` and reads zero volume.
fn parse_candle(value: &Value) -> Candle {
    Candle {
        // Milliseconds on the wire, seconds on the chart — the same conversion
        // `hyperliquid::parse_candle` makes.
        ts: value_i64(value, "t") / 1_000,
        open: num(value, "o"),
        high: num(value, "h"),
        low: num(value, "l"),
        close: num(value, "c"),
        // The base leg. `V` sits beside it and is the quote leg — 451.955 of
        // bitcoin against 29,366,566 of quote on one live hourly bar — and
        // `hyperliquid`'s `v` is the base one, so the two venues put the same
        // quantity on the same axis.
        volume: num(value, "v"),
    }
}

/// The request for `bars` candles of one market ending at `end_ms`.
///
/// The window and `count_back` are both required and the answer is the wider
/// of the two, capped at `CANDLE_PAGE` — so asking for the same count on both
/// sides is the one request whose length is the length that was asked for.
/// Live, that returns exactly 3 and exactly 500 bars at 1m, 15m, 1h, 4h and
/// 1d, whereas `count_back` alone under a wide window answers the window.
fn candles_path(id: i64, width: &str, secs: i64, end_ms: i64, bars: i64) -> String {
    let start = end_ms - bars * secs * 1_000;
    format!(
        "candles?market_id={id}&resolution={width}&start_timestamp={start}\
         &end_timestamp={end_ms}&count_back={bars}"
    )
}

/// Reads a window and folds it into the tape, answering the tape's length.
///
/// The focus is re-read after the await rather than before, because that is
/// the whole of what the guard is for: the reader can switch market or width
/// while an exchange is answering, and a window folded in after that would be
/// the previous coin's bars drawn as this one's.
async fn fill(
    zone: Zone,
    tape: &Tape,
    coin: &str,
    interval: &str,
    end_ms: i64,
    bars: i64,
) -> Result<i64, HlError> {
    let (width, secs) = resolution(interval).ok_or_else(|| {
        fail(format!(
            "Lighter quotes no {interval} candle: it has {}",
            widths()
        ))
    })?;
    let id = market_of(zone, coin).await?.id;
    let body = get(zone, candles_path(id, width, secs, end_ms, bars)).await?;
    let fresh: Vec<Candle> = list(&body, "c").iter().map(parse_candle).collect();

    let mut candles = lock(&tape.candles);
    if tape
        .focus()
        .is_none_or(|(held, held_width)| held != coin || held_width != interval)
    {
        // The reader moved on while this was in flight.
        return Ok(candles.len() as i64);
    }
    merge(&mut candles, fresh);
    Ok(candles.len() as i64)
}

/// Brings the chart's tape up to date, backfilling a full window into an empty
/// one and refreshing only what can still have changed in a loaded one.
///
/// The route is `GET /api/v1/candles`, which is the name in Lighter's own
/// OpenAPI document. It serves real history: bitcoin's hourly bars reach back
/// past 2025-07-13 and its daily ones past 2025-03-27.
///
/// `/candlesticks` is the trap, and it is worth naming because it looks like
/// an answer. It is not a route on this venue, and the 403 it draws says
/// nothing about candles: every path the edge does not recognise answers the
/// same way — `/info`, `/openapi.json` and `/nonsense` all 403 while
/// `/candles` and `/orderBookDetails` beside them answer 200. A 403 there is
/// the edge failing to route, not the venue withholding history.
pub async fn lighter_candles(
    zone: Zone,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    let bars = if lock(&tape.candles).is_empty() {
        CANDLE_PAGE
    } else {
        CANDLE_REFRESH
    };
    fill(zone, &tape, &coin, &interval, now_ms(), bars).await
}

/// The window ending where the tape currently begins, so a chart panned back
/// to its oldest bar can keep going.
///
/// Answers how many older bars it added, on `hl_history`'s contract and for
/// its reasons: zero is the venue saying it has nothing before the bar the
/// tape starts at, and the caller stops asking rather than sending the same
/// request again.
pub async fn lighter_history(
    zone: Zone,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    let Some(oldest) = lock(&tape.candles).first().map(|candle| candle.ts) else {
        return Ok(0);
    };
    fill(zone, &tape, &coin, &interval, oldest * 1_000, CANDLE_PAGE).await?;
    Ok(older_than(&lock(&tape.candles), oldest))
}

// ---------------------------------------------------------------------------
// The live feed.
//
// Same shape as `hl_market_feed`: one socket on its own thread, folding
// whatever arrives into a `MarketTick` and emitting at most one per beat.
// The transport is written again here rather than shared, because only the
// loop is common — Lighter spells a subscription, a ping, an error and a
// payload differently at every one of those four points, and a socket that
// took all four as parameters would be a protocol description with two
// implementations rather than a shared socket.

/// Mirrors of `hyperliquid`'s feed timings, for the same reason the geometry
/// above is mirrored: that module publishes no `pub const`. A venue switch
/// must not change how often the screen is asked to redraw, so the beat in
/// particular is the app's rate rather than either venue's.
const POLL: Duration = Duration::from_millis(200);
const BEAT: Duration = Duration::from_millis(100);
const PING: Duration = Duration::from_secs(15);
const RETRY: Duration = Duration::from_secs(2);

/// Every market's day, which is where the mid prices for the market list come
/// from. The per-market `market_stats/<id>` channel carries the same object,
/// so the focused market's context is read out of this one stream too.
const ALL_STATS: &str = "market_stats/all";
const ALL_STATS_ECHO: &str = "market_stats:all";

/// The candle widths the venue quotes, and how wide each one is in seconds.
///
/// Checked by subscribing to all thirteen a chart might plausibly offer: the
/// eight here answered with a candle and `2h`, `6h`, `8h`, `3d` and `1w` each
/// answered `{"error":{"code":30005,"message":"Invalid Channel:  (invalid
/// resolution)"}}`. The refusal does not name the channel it refused, which is
/// why an unsupported interval is turned away here rather than on the wire.
/// `/candles` takes the same set and answers anything else `20001 "invalid
/// param "`, so one table serves the socket and the history read both — the
/// seconds are the REST side's, which has to state a window as well as a
/// width.
const RESOLUTIONS: [(&str, i64); 8] = [
    ("1m", 60),
    ("5m", 300),
    ("15m", 900),
    ("30m", 1_800),
    ("1h", 3_600),
    ("4h", 14_400),
    ("12h", 43_200),
    ("1d", 86_400),
];

/// The venue's spelling of an interval the app asked for and its width, or
/// nothing if it does not quote one that wide. The two vocabularies happen to
/// agree on every tab this app offers, but this is a lookup against the
/// venue's list, so a tab the venue dropped becomes a refusal instead of a
/// chart drawn at the wrong width.
fn resolution(interval: &str) -> Option<(&'static str, i64)> {
    RESOLUTIONS
        .iter()
        .copied()
        .find(|(known, _)| *known == interval)
}

/// The widths the venue quotes, for a refusal that tells the reader what it
/// could have asked for instead.
fn widths() -> String {
    RESOLUTIONS.map(|(width, _)| width).join(", ")
}

/// The channels the feed wants open for the market the tape is pointed at.
///
/// The resolution is checked before the market id, so an interval the venue
/// does not quote is refused whether or not the ticker table has been filled.
/// A ticker with no id yet is not an error: the table is filled by the
/// universe read, and until it lands the feed holds the exchange-wide stats
/// and nothing per-market.
fn channels(tape: &Tape) -> Result<Vec<String>, HlError> {
    let mut wanted = vec![ALL_STATS.to_owned()];
    let Some((coin, interval)) = tape.focus() else {
        return Ok(wanted);
    };
    let (width, _) = resolution(&interval).ok_or_else(|| {
        fail(format!(
            "Lighter quotes no {interval} candle: it has {}",
            widths()
        ))
    })?;
    let Some(id) = lock(ids()).get(&coin).map(|market| market.id) else {
        return Ok(wanted);
    };
    wanted.push(format!("order_book/{id}"));
    wanted.push(format!("trade/{id}"));
    wanted.push(format!("candle/{id}/{width}"));
    Ok(wanted)
}

/// The three per-market channels the reader will fold, in the spelling the
/// replies carry: a subscription is sent with slashes and echoed back with
/// colons, so `order_book/1` is answered on `order_book:1`.
///
/// Empty until the market is known, and empty strings never match a channel a
/// message arrived on — which is the guard that stops the book of a market the
/// app just left from being drawn as this one's.
#[derive(Clone, Debug, Default, PartialEq)]
struct Focused {
    coin: String,
    book: String,
    prints: String,
    bars: String,
}

fn focused(tape: &Tape) -> Focused {
    let Some((coin, interval)) = tape.focus() else {
        return Focused::default();
    };
    let (Some(id), Some((width, _))) = (
        lock(ids()).get(&coin).map(|market| market.id),
        resolution(&interval),
    ) else {
        // The market list has not landed yet, or the app is asking for a width
        // this venue does not quote. Either way there is nothing per-market to
        // fold, but the coin is still the one the context belongs to.
        return Focused {
            coin,
            ..Focused::default()
        };
    };
    Focused {
        coin,
        book: format!("order_book:{id}"),
        prints: format!("trade:{id}"),
        bars: format!("candle:{id}:{width}"),
    }
}

/// The levels a book is holding between deltas.
///
/// Keyed by the price's bit pattern rather than by the price itself: a
/// positive `f64` orders the same as its bits read as an integer, so the map
/// sorts by price without an `Ord` on `f64` and without knowing the market's
/// tick size. Iterating gives the whole side in price order, and the panel
/// only ever wants its first ten.
#[derive(Clone, Debug, Default, PartialEq)]
struct Depth {
    bids: BTreeMap<u64, f64>,
    asks: BTreeMap<u64, f64>,
    /// The update the held levels are at. Zero until a snapshot lands, which
    /// is what makes a delta arriving before one a gap rather than a book.
    nonce: i64,
}

impl Depth {
    /// The top of the book, nearest price first on both sides.
    fn book(&self) -> Book {
        book_of(top(self.bids.iter().rev()), top(self.asks.iter()))
    }
}

/// One side's best `BOOK_DEPTH` levels with the depth behind each resolved,
/// which is the same shape `parse_levels` builds out of REST orders.
fn top<'a>(side: impl Iterator<Item = (&'a u64, &'a f64)>) -> Vec<Level> {
    let mut total = 0.0;
    let mut rows: Vec<Level> = side
        .take(BOOK_DEPTH)
        .map(|(price, size)| {
            total += size;
            Level {
                price: f64::from_bits(*price),
                size: *size,
                total,
                bar: 0.0,
            }
        })
        .collect();
    scale_bars(&mut rows);
    rows
}

/// What one `order_book` message did to the levels it was applied to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fold {
    /// The message was the next one, and the levels now include it.
    Applied,
    /// The message is one the held levels have already had. Applying it again
    /// would write an old size over a newer one, because a delta states what a
    /// level *is* rather than how it changed.
    Stale,
    /// At least one message between the held levels and this one was missed.
    /// What those messages said is unrecoverable — an absolute size for a
    /// level nobody mentioned again is gone — so the caller throws the book
    /// away and asks for a fresh snapshot rather than drifting.
    Gap,
}

/// Applies one `order_book` message to the levels held for that market.
///
/// Pure, and the whole reason the book folds correctly: everything about
/// keeping a delta stream honest is decided here, where a test can drive it
/// through an insert, an update, a removal, a replayed message and a gap
/// without a socket.
///
/// The nonces are the venue's own sequencing: each update states the
/// `begin_nonce` it expects the reader to be holding, which is the previous
/// update's `nonce`. Anything else is a message out of order, and the two
/// directions are not the same failure — one is already applied and one is
/// missing.
fn fold_book(held: Depth, message: &Value) -> (Depth, Fold) {
    let body = message.get("order_book").unwrap_or(&Value::Null);
    let nonce = value_i64(body, "nonce");
    // The venue answers a subscription with `subscribed/order_book` carrying
    // the whole book, and pushes `update/order_book` after it. Only the first
    // may replace what is held.
    if text(message, "type").starts_with("subscribed") {
        let mut fresh = Depth {
            nonce,
            ..Depth::default()
        };
        apply(&mut fresh.bids, list(body, "bids"));
        apply(&mut fresh.asks, list(body, "asks"));
        return (fresh, Fold::Applied);
    }
    if held.nonce == 0 {
        // Nothing to apply a delta to. The venue opens with a snapshot, so
        // this is a subscription that has to be restarted rather than a
        // message to fold.
        return (held, Fold::Gap);
    }
    if value_i64(body, "begin_nonce") != held.nonce {
        // Behind what is held is a message already applied; ahead of it is one
        // or more that never arrived.
        let fold = if nonce <= held.nonce {
            Fold::Stale
        } else {
            Fold::Gap
        };
        return (held, fold);
    }
    let mut folded = Depth { nonce, ..held };
    apply(&mut folded.bids, list(body, "bids"));
    apply(&mut folded.asks, list(body, "asks"));
    (folded, Fold::Applied)
}

/// One side of a delta. A row states what the level now holds, and a size of
/// zero states that it holds nothing — live `order_book` messages carry
/// `"0.00000"` for a level that has just emptied, and a reader that kept it
/// would show depth nobody can trade against and eventually a crossed book.
fn apply(side: &mut BTreeMap<u64, f64>, rows: &[Value]) {
    for row in rows {
        let price = num(row, "price");
        if !(price > 0.0 && price.is_finite()) {
            continue;
        }
        let size = num(row, "size");
        if size > 0.0 {
            side.insert(price.to_bits(), size);
        } else {
            side.remove(&price.to_bits());
        }
    }
}

/// The public tape, folded the way it was traded rather than the way it was
/// messaged — the same fold `hyperliquid` does over its `hash`. One aggressing
/// order that takes five resting orders arrives as five prints sharing a taker
/// order id, and five rows at one price is the wire's bookkeeping rather than
/// the market's.
///
/// Which id is the taker's follows from `is_maker_ask`: the maker being the
/// ask makes the bid the aggressor, so the taker is `bid_id`, and the other
/// way round when the maker was the bid.
///
/// The venue lists its prints newest first — checked across a live
/// subscription, where every message's `trade_id` descended — and `MarketTick`
/// carries a beat's prints oldest first for `push_trades` to reverse onto the
/// panel, so the array is walked backwards.
///
/// `liquidation_trades` rides in the same message and is deliberately not on
/// this tape: it is a separate recent-liquidations list rather than this
/// stream's prints, and the copy that arrives with the subscription was hours
/// old when it was read.
fn parse_prints(message: &Value) -> Vec<Trade> {
    let mut tape: Vec<Trade> = Vec::new();
    let mut aggressor = 0;
    for print in list(message, "trades").iter().rev() {
        let maker_ask = print
            .get("is_maker_ask")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let taker = value_i64(print, if maker_ask { "bid_id" } else { "ask_id" });
        let price = num(print, "price");
        let size = num(print, "size");
        match tape.last_mut() {
            // A missing id is not an identity, so it never merges.
            Some(held) if taker != 0 && taker == aggressor => {
                let total = held.size + size;
                if total > 0.0 {
                    held.price = (held.price * held.size + price * size) / total;
                }
                held.size = total;
                held.sweep += 1;
            }
            _ => {
                aggressor = taker;
                tape.push(Trade {
                    // The venue's prints are stamped in milliseconds; the tape
                    // reads seconds.
                    ts: value_i64(print, "timestamp") / 1_000,
                    price,
                    size,
                    buy: lighter_buy(maker_ask),
                    sweep: 1,
                    tid: value_i64(print, "trade_id"),
                });
            }
        }
    }
    tape
}

/// One market's day out of `market_stats`, in the shape `apply_feed` writes
/// onto the row of that name. Leverage, maintenance and the size step belong
/// to the asset rather than to the day and are not restated here, so they read
/// as zero and the caller keeps whatever the universe said — the same contract
/// `hyperliquid`'s streamed context has.
fn parse_stats(stats: &Value) -> SymbolRow {
    let price = num(stats, "mark_price");
    let change_pct = num(stats, "daily_price_change");
    SymbolRow {
        name: text(stats, "symbol"),
        // Lighter keys a book by `market_id`, which this deployment's own
        // universe supplies; the Hyperliquid universe index this field carries
        // is not a thing Lighter has. The order path does not read it either:
        // it resolves the id and the two step counts together out of
        // `orderBookDetails`, because a price is sent as a count of the
        // market's own steps and the two must come from one row.
        asset: 0,
        price,
        change_pct,
        volume: num(stats, "daily_quote_token_volume"),
        // Already the hourly rate as a percentage, which is exactly what
        // `SymbolRow.funding_pct` is and is *not* the unit `/funding-rates`
        // publishes. Settled against that endpoint in the same minute: across
        // all 196 markets both quote with a non-zero rate, this field over the
        // REST rate is 12.5 with every quartile on 12.5 — which is the 100/8
        // that turns an eight-hour fraction into an hourly percentage.
        funding_pct: num(stats, "current_funding_rate"),
        leverage: 0.0,
        maintenance: 0.0,
        size_decimals: 0,
        // Nor does the stream restate the group or the collateral, so these
        // read as empty and the caller keeps the universe's reading — which
        // on this venue is one flat list settled in one token.
        category: String::new(),
        collateral: String::new(),
        heading: false,
        open_interest: open_interest(num(stats, "open_interest"), price),
        prev: previous_price(price, change_pct),
        selected: false,
    }
}

/// Open interest in coins, which is the unit the market list holds and the
/// unit `/orderBookDetails` publishes. The stream states it in dollars
/// instead, so it is divided back out by the mark it was valued at.
///
/// Settled rather than inferred from the magnitude: against the universe read
/// in the same minute, this quotient over the published figure is 1.0 across
/// all 207 markets that carry any, spanning 0.99991 to 1.00004 — while the two
/// figures taken as the same unit differ by a median factor of 1.885, which is
/// just the price of a coin.
fn open_interest(quote: f64, price: f64) -> f64 {
    if price > 0.0 { quote / price } else { 0.0 }
}

/// The market data feed: every market's mid price, and the book, prints,
/// candles and context of whatever market the tape is pointed at. Candles are
/// merged into the tape in place, so the chart follows them on its own repaint
/// beat without an app message per bar.
///
/// The same signature and the same behaviour as `hl_market_feed`, so the app
/// cannot tell which venue it is holding — with one difference it cannot hide:
/// the tape only fills forward. Lighter's `candle` channel answers a
/// subscription with exactly one candle, the one now forming, on every
/// resolution it quotes — checked on all eight — and no REST route serves
/// history. So a chart opened here starts with a single bar and gains one per
/// interval, where the other venue backfills five hundred. The candles that do
/// arrive are the venue's own; nothing is folded out of the tape to stand in
/// for the ones it will not send.
pub fn lighter_market_feed(zone: Zone, tape: Tape) -> Receiver<Result<MarketTick, HlError>> {
    let (sender, receiver) = smol::channel::unbounded();
    // What the reader hands the socket: the channels whose held state it had
    // to throw away, which only a fresh snapshot can restore.
    let refresh: Arc<Mutex<Vec<String>>> = Arc::default();
    let stale = refresh.clone();
    let subscriptions = tape.clone();
    std::thread::spawn(move || {
        let mut subscribe = move || channels(&subscriptions);
        let mut read = market_reader(tape, refresh);
        while !sender.is_closed() {
            let Err(error) = pump(zone, &mut subscribe, &mut read, &sender, &stale) else {
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

/// What the market feed makes of one connection's traffic. Apart from the
/// socket so that a test can walk it through the sequence a market switch
/// produces, which is the only way to reach these arms without the exchange.
fn market_reader(
    tape: Tape,
    refresh: Arc<Mutex<Vec<String>>>,
) -> impl FnMut(Event<'_>) -> Option<MarketTick> + Send + 'static {
    let mut tick = MarketTick::default();
    let mut changed = false;
    let mut depth = Depth::default();
    // Which market the per-market fields belong to. Recomputed every beat
    // rather than only when the app moves, because the ticker-to-id table can
    // fill in after the feed has already started and the channels are named
    // for the id.
    let mut held = Focused::default();
    move |event| match event {
        Event::Payload(channel, message) if channel == ALL_STATS_ECHO => {
            // Keyed by market id, one market per update and every market in
            // the snapshot. Folded into the mids the app already holds rather
            // than replacing them, because a beat carries whichever markets
            // moved during it rather than the whole exchange.
            for stats in message.get("market_stats")?.as_object()?.values() {
                let name = text(stats, "symbol");
                let price = num(stats, "mark_price");
                if name.is_empty() || price <= 0.0 {
                    continue;
                }
                if name == held.coin {
                    tick.context = Some(parse_stats(stats));
                }
                // The mark rather than the mid, because the mark is what the
                // universe read put on the row and what the venue values a
                // position at. Two quantities alternating in one column would
                // read as a market that never stops ticking.
                tick.mids.insert(name, price);
                changed = true;
            }
            None
        }
        Event::Payload(channel, message) if channel == held.book => {
            let (folded, fold) = fold_book(std::mem::take(&mut depth), message);
            depth = folded;
            match fold {
                // Already applied: the levels are ahead of this message.
                Fold::Stale => None,
                Fold::Gap => {
                    // The levels cannot be brought forward, so they are not
                    // drawn. Asking the socket for a fresh snapshot is the
                    // whole recovery; a book missing its deltas would go on
                    // rendering, slowly wrong, and eventually crossed.
                    depth = Depth::default();
                    lock(&refresh).push(channel.replace(':', "/"));
                    changed |= tick.book.is_some();
                    tick.book = None;
                    None
                }
                Fold::Applied => {
                    let mut book = depth.book();
                    // The book renders from the top down, so the asks are
                    // reversed here and the view just walks both lists — the
                    // same thing `hl_market_feed` does to its own.
                    book.asks.reverse();
                    changed |= tick.book.as_ref() != Some(&book);
                    tick.book = Some(book);
                    None
                }
            }
        }
        Event::Payload(channel, message) if channel == held.prints => {
            let fresh = parse_prints(message);
            changed |= !fresh.is_empty();
            tick.trades.extend(fresh);
            None
        }
        Event::Payload(channel, message) if channel == held.bars => {
            // Merged in place and deliberately not counted as a change: the
            // chart repaints off the shared tape on its own beat, so a bar
            // costs no app message at all.
            merge(&mut lock(&tape.candles), parse_candles(message));
            None
        }
        Event::Payload(..) => None,
        Event::Pong(round_trip) => {
            changed |= tick.latency != round_trip;
            tick.latency = round_trip;
            None
        }
        Event::Beat => {
            let fresh = focused(&tape);
            if fresh != held {
                // Everything per-market belongs to the market that was on
                // screen a moment ago. The socket keeps serving the old
                // subscriptions until the unsubscribe takes effect, and the
                // channel names above are what turns those away.
                held = fresh;
                depth = Depth::default();
                tick.book = None;
                tick.context = None;
                tick.trades.clear();
                changed = false;
                return None;
            }
            let ready = changed;
            changed = false;
            // Nothing moved: an unchanged message would rebuild the view for
            // no reason.
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

/// The bars one `candle` message carries. The venue sends OHLCV as JSON
/// floats here and prices as JSON strings nearly everywhere else, which costs
/// nothing to read because every field on this venue already goes through a
/// reader that takes either.
fn parse_candles(message: &Value) -> Vec<Candle> {
    // The same bar the history read parses, under the socket's own key for the
    // array: the feed spells it `candles` and `/candles` spells it `c`, and
    // everything inside is identical. One reader, so a bar cannot mean one
    // thing forming and another once it is history.
    list(message, "candles").iter().map(parse_candle).collect()
}

/// One websocket connection, plus the channels it is currently holding open.
struct Socket {
    ws: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    open: Vec<String>,
}

impl Socket {
    fn connect(zone: Zone) -> Result<Self, HlError> {
        // The same gate the REST reads pass: a test drives the real program,
        // subscriptions included, so an ungated socket would make the suite
        // depend on an exchange being up.
        if !wire_is_open() {
            return Err(fail(
                "Lighter feed unreachable: no wire under test".to_owned(),
            ));
        }
        let (ws, _) = tungstenite::connect(zone.stream_url())
            .map_err(|error| fail(format!("Lighter feed unreachable: {error}")))?;
        // Reads have to time out, or the loop could never look at the clock to
        // ping, or at the app to see that it changed markets.
        let stream = match ws.get_ref() {
            MaybeTlsStream::Plain(stream) => stream,
            MaybeTlsStream::Rustls(tls) => tls.get_ref(),
            _ => return Err(fail("Unknown Lighter transport".to_owned())),
        };
        stream
            .set_read_timeout(Some(POLL))
            .map_err(|error| fail(format!("Lighter feed unreadable: {error}")))?;
        Ok(Self {
            ws,
            open: Vec::new(),
        })
    }

    fn send(&mut self, request: &Value) -> Result<(), HlError> {
        self.ws
            .send(tungstenite::Message::Text(request.to_string().into()))
            .map_err(|error| fail(format!("Lighter feed refused: {error}")))
    }

    /// Holds exactly `wanted` open, sending only the difference. Switching
    /// markets therefore costs two frames rather than a new connection, and
    /// asking for what is already open costs nothing — which is not only
    /// thrift: subscribing twice to one channel is answered `{"code":30003,
    /// "message":"Already Subscribed to : order_book:1"}`, and this loop reads
    /// any error as a failed connection.
    fn want(&mut self, wanted: Vec<String>) -> Result<(), HlError> {
        let gone: Vec<String> = self
            .open
            .iter()
            .filter(|held| !wanted.contains(held))
            .cloned()
            .collect();
        let fresh: Vec<String> = wanted
            .iter()
            .filter(|want| !self.open.contains(want))
            .cloned()
            .collect();
        for channel in gone {
            self.send(&json!({ "type": "unsubscribe", "channel": channel }))?;
        }
        for channel in fresh {
            self.send(&json!({ "type": "subscribe", "channel": channel }))?;
        }
        self.open = wanted;
        Ok(())
    }

    /// Asks a channel for its snapshot again, which is how a book that missed
    /// a delta gets back to a state it can apply the next one to. Dropped
    /// before it is re-taken because a second subscribe to a channel already
    /// open is refused rather than answered.
    fn resubscribe(&mut self, channel: &str) -> Result<(), HlError> {
        self.send(&json!({ "type": "unsubscribe", "channel": channel }))?;
        self.send(&json!({ "type": "subscribe", "channel": channel }))
    }
}

/// One connection's lifetime. Returns `Ok` only when the app has stopped
/// listening; anything else is an error the caller reconnects through.
fn pump(
    zone: Zone,
    subscribe: &mut impl FnMut() -> Result<Vec<String>, HlError>,
    read: &mut impl FnMut(Event<'_>) -> Option<MarketTick>,
    sender: &Sender<Result<MarketTick, HlError>>,
    refresh: &Mutex<Vec<String>>,
) -> Result<(), HlError> {
    let mut socket = Socket::connect(zone)?;
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
            socket.want(subscribe()?)?;
            // Deduplicated because every delta arriving between the gap and
            // the fresh snapshot asks again, and each ask costs a whole book.
            let mut stale = std::mem::take(&mut *lock(refresh));
            stale.sort_unstable();
            stale.dedup();
            for channel in stale {
                socket.resubscribe(&channel)?;
            }
            if let Some(item) = read(Event::Beat)
                && sender.send_blocking(Ok(item)).is_err()
            {
                return Ok(());
            }
        }
        if now >= ping {
            ping = now + PING;
            sent = Some(now);
            socket.send(&json!({ "type": "ping" }))?;
        }
        match socket.ws.read() {
            Ok(tungstenite::Message::Text(body)) => {
                let message: Value = serde_json::from_str(&body)
                    .map_err(|error| fail(format!("Lighter feed sent bad JSON: {error}")))?;
                if let Some(refused) = message.get("error") {
                    // A rejected subscription is silence otherwise, and
                    // silence is indistinguishable from a quiet market.
                    return Err(fail(format!(
                        "Lighter feed rejected a request: code {} {}",
                        value_i64(refused, "code"),
                        text(refused, "message")
                    )));
                }
                let item = if text(&message, "type") == "pong" {
                    let round_trip = sent.take().map_or(0, |at| at.elapsed().as_millis() as i64);
                    read(Event::Pong(round_trip))
                } else {
                    // The venue nests nothing: a payload is the frame it
                    // arrived in, and `connected` carries no channel at all.
                    let channel = text(&message, "channel");
                    if channel.is_empty() {
                        None
                    } else {
                        read(Event::Payload(&channel, &message))
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
            Err(error) => return Err(fail(format!("Lighter feed dropped: {error}"))),
        }
    }
}

// ---------------------------------------------------------------------------
// What Lighter actually answered.
//
// Four responses off the live venue, trimmed in rows and in fields but never
// in figures. They are the fixtures this module's parsers are checked against
// and, run through those same parsers, they are also the Lighter the app draws
// when it is drawing a fixture rather than a feed. One copy serves both on
// purpose: a demo universe written by hand beside a captured one is a second
// opinion about what this venue lists, and the two drift the moment either is
// touched. Every number a Lighter screen shows therefore comes back to a
// number the venue published.
//
// The four were taken within the same market day and cross-check each other:
// the account's positions are markets the universe lists, and multiplying each
// position's value by its market's published maintenance fraction reproduces
// the requirement the account states — which is the check
// `maintenance_in_basis_points_reproduces_the_accounts_requirement` runs.

/// `GET /orderBookDetails`, trimmed to the markets the fixtures need out of
/// the 222 the venue lists.
///
/// The trim is not arbitrary: every market the captured account holds a
/// position in is here, so the two payloads describe one terminal; MKR is here
/// because a retired market keeps its row and has to be dropped by `status`
/// rather than by its numbers; and AAPL and 1000PEPE are here because they are
/// the two shapes of difference from the other venue's universe — a market
/// Hyperliquid does not list at all, and a coin it lists under another
/// spelling.
fn captured_universe() -> Value {
    json!({
        "code": 200,
        "order_book_details": [
            {
                "symbol": "MKR", "market_id": 28, "market_type": "perp",
                "status": "inactive", "price_decimals": 2, "size_decimals": 4,
                "maintenance_margin_fraction": 0,
                "min_initial_margin_fraction": 0,
                "mark_price": "0.00", "index_price": "0.00",
                "daily_price_change": 0, "daily_quote_token_volume": 0,
                "daily_base_token_volume": 0, "open_interest": 0
            },
            {
                "symbol": "ASTER", "market_id": 83, "market_type": "perp",
                "status": "active", "price_decimals": 5, "size_decimals": 1,
                "maintenance_margin_fraction": 1200,
                "min_initial_margin_fraction": 2000,
                "default_initial_margin_fraction": 2000,
                "mark_price": "0.60108", "index_price": "0.60140",
                "daily_price_change": -0.02161407242376883,
                "daily_quote_token_volume": 129411.264773,
                "daily_base_token_volume": 216005.6,
                "daily_price_low": 0.59635, "daily_price_high": 0.60212,
                "open_interest": 1463278.2
            },
            {
                "symbol": "BTC", "market_id": 1, "market_type": "perp",
                "status": "active", "price_decimals": 1, "size_decimals": 5,
                "maintenance_margin_fraction": 120,
                "min_initial_margin_fraction": 200,
                "default_initial_margin_fraction": 500,
                "mark_price": "64975.3", "index_price": "64980.1",
                "daily_price_change": 0.6118286879673691,
                "daily_quote_token_volume": 618847551.336845,
                "daily_base_token_volume": 9520.35,
                "daily_price_low": 64270.5, "daily_price_high": 65349.4,
                "open_interest": 1836.69392
            },
            {
                "symbol": "ETH", "market_id": 0, "market_type": "perp",
                "status": "active", "price_decimals": 2, "size_decimals": 4,
                "maintenance_margin_fraction": 120,
                "min_initial_margin_fraction": 200,
                "default_initial_margin_fraction": 500,
                "mark_price": "1918.42", "index_price": "1918.63",
                "daily_price_change": 0.27967421874183196,
                "daily_quote_token_volume": 111002380.342004,
                "daily_base_token_volume": 57891.4413,
                "daily_price_low": 1904.51, "daily_price_high": 1925.62,
                "open_interest": 42741.2792
            },
            {
                "symbol": "SOL", "market_id": 2, "market_type": "perp",
                "status": "active", "price_decimals": 3, "size_decimals": 3,
                "maintenance_margin_fraction": 240,
                "min_initial_margin_fraction": 400,
                "default_initial_margin_fraction": 1000,
                "mark_price": "75.460", "index_price": "75.472",
                "daily_price_change": 2.4113879389893067,
                "daily_quote_token_volume": 27263870.348171,
                "daily_base_token_volume": 366209.573,
                "daily_price_low": 73.152, "daily_price_high": 75.685,
                "open_interest": 150485.366
            },
            {
                "symbol": "ENA", "market_id": 29, "market_type": "perp",
                "status": "active", "price_decimals": 5, "size_decimals": 1,
                "maintenance_margin_fraction": 600,
                "min_initial_margin_fraction": 1000,
                "default_initial_margin_fraction": 2000,
                "mark_price": "0.09090", "index_price": "0.09093",
                "daily_price_change": -3.6327049353950436,
                "daily_quote_token_volume": 544490.761735,
                "daily_base_token_volume": 5904329.4,
                "daily_price_low": 0.08937, "daily_price_high": 0.09594,
                "open_interest": 25634609.1
            },
            {
                "symbol": "SUI", "market_id": 16, "market_type": "perp",
                "status": "active", "price_decimals": 5, "size_decimals": 1,
                "maintenance_margin_fraction": 600,
                "min_initial_margin_fraction": 1000,
                "default_initial_margin_fraction": 2000,
                "mark_price": "0.69232", "index_price": "0.69245",
                "daily_price_change": 3.845577211394303,
                "daily_quote_token_volume": 284005.289459,
                "daily_base_token_volume": 416781.4,
                "daily_price_low": 0.66409, "daily_price_high": 0.69587,
                "open_interest": 858813.5
            },
            {
                "symbol": "AAPL", "market_id": 113, "market_type": "perp",
                "status": "active", "price_decimals": 3, "size_decimals": 3,
                "maintenance_margin_fraction": 300,
                "min_initial_margin_fraction": 500,
                "default_initial_margin_fraction": 1000,
                "mark_price": "312.351", "index_price": "312.402",
                "daily_price_change": -0.49991084392592405,
                "daily_quote_token_volume": 202518.34768,
                "daily_base_token_volume": 645.821,
                "daily_price_low": 311.494, "daily_price_high": 314.873,
                "open_interest": 2200.689
            },
            {
                "symbol": "1000PEPE", "market_id": 4, "market_type": "perp",
                "status": "active", "price_decimals": 6, "size_decimals": 0,
                "maintenance_margin_fraction": 600,
                "min_initial_margin_fraction": 1000,
                "default_initial_margin_fraction": 2000,
                "mark_price": "0.002856", "index_price": "0.002857",
                "daily_price_change": 1.1693834160170091,
                "daily_quote_token_volume": 71666.242926,
                "daily_base_token_volume": 25373122,
                "daily_price_low": 0.002794, "daily_price_high": 0.002879,
                "open_interest": 135356889
            },
            {
                "symbol": "OP", "market_id": 55, "market_type": "perp",
                "status": "active", "price_decimals": 5, "size_decimals": 1,
                "maintenance_margin_fraction": 399,
                "min_initial_margin_fraction": 666,
                "default_initial_margin_fraction": 1332,
                "mark_price": "0.08849", "index_price": "0.08851",
                "daily_price_change": 2.458256029684601,
                "daily_quote_token_volume": 29502.738809,
                "daily_base_token_volume": 336881.5,
                "daily_price_low": 0.08576, "daily_price_high": 0.08916,
                "open_interest": 2103923.9
            },
            {
                "symbol": "STRK", "market_id": 104, "market_type": "perp",
                "status": "active", "size_decimals": 1,
                "maintenance_margin_fraction": 1200,
                "min_initial_margin_fraction": 2000,
                "default_initial_margin_fraction": 4000,
                "mark_price": "0.02518", "index_price": "0.02519",
                "daily_price_change": 1.2257809410834322,
                "daily_quote_token_volume": 4901.07704,
                "daily_base_token_volume": 194998.6,
                "daily_price_low": 0.0247, "daily_price_high": 0.02561,
                "open_interest": 1527130.8
            },
            {
                "symbol": "NMR", "market_id": 74, "market_type": "perp",
                "status": "active", "size_decimals": 2,
                "maintenance_margin_fraction": 2000,
                "min_initial_margin_fraction": 3333,
                "default_initial_margin_fraction": 6666,
                "mark_price": "8.4603", "index_price": "8.4611",
                "daily_price_change": -0.11517758503161507,
                "daily_quote_token_volume": 872.92694,
                "daily_base_token_volume": 103.19,
                "daily_price_low": 8.4606, "daily_price_high": 8.515,
                "open_interest": 6540.92
            }
        ]
    })
}

/// `GET /funding-rates`, keeping every market above and the two venues Lighter
/// only republishes for BTC, so the filter has something to reject.
fn captured_rates() -> Value {
    json!({
        "code": 200,
        "funding_rates": [
            { "market_id": 1, "exchange": "binance", "symbol": "BTC", "rate": 4.183e-05 },
            { "market_id": 1, "exchange": "hyperliquid", "symbol": "BTC", "rate": 3.20592e-05 },
            { "market_id": 1, "exchange": "lighter", "symbol": "BTC", "rate": 6.4e-05 },
            { "market_id": 0, "exchange": "lighter", "symbol": "ETH", "rate": 9.6e-05 },
            { "market_id": 2, "exchange": "lighter", "symbol": "SOL", "rate": 9.6e-05 },
            { "market_id": 4, "exchange": "lighter", "symbol": "1000PEPE", "rate": 9.6e-05 },
            { "market_id": 16, "exchange": "lighter", "symbol": "SUI", "rate": 9.6e-05 },
            { "market_id": 29, "exchange": "lighter", "symbol": "ENA", "rate": 9.6e-05 },
            { "market_id": 55, "exchange": "lighter", "symbol": "OP", "rate": 9.6e-05 },
            { "market_id": 74, "exchange": "lighter", "symbol": "NMR", "rate": 9.6e-05 },
            { "market_id": 83, "exchange": "lighter", "symbol": "ASTER", "rate": 0.0001 },
            { "market_id": 104, "exchange": "lighter", "symbol": "STRK", "rate": 9.6e-05 },
            { "market_id": 113, "exchange": "lighter", "symbol": "AAPL", "rate": 3.2e-05 }
        ]
    })
}

/// A live `GET /account?by=index&value=702384`, trimmed in fields but not
/// in rows: all six positions are here because the account's totals are
/// the sum over all six, and a subset of them proves nothing.
///
/// `l1_address` is the address the fixtures are drawn for, and it is this
/// account's rather than a plausible-looking one: an address is what the app
/// asks a reader for, and a Lighter screen showing an address with no account
/// on Lighter is the same fixture mistake as showing the wrong venue's
/// markets.
fn captured_account() -> Value {
    json!({
        "code": 200,
        "total": 1,
        "accounts": [{
            "account_index": 702384,
            "account_type": 0,
            "l1_address": "0x3f4ec7684F679F83c782e485b358A2D43045d6A2",
            "collateral": "1837526.962274",
            "available_balance": "1753357.208845",
            "total_asset_value": "1837610.4619429999",
            "cross_asset_value": "1837610.4619429999",
            "cross_initial_margin_requirement": "84253.253098",
            "cross_maintenance_margin_requirement": "3084.930401",
            "positions": [
                {
                    "market_id": 1, "symbol": "BTC",
                    "initial_margin_fraction": "33.33",
                    "sign": 1, "position": "3.87333",
                    "avg_entry_price": "64969.5",
                    "position_value": "251677.750743",
                    "unrealized_pnl": "29.306261",
                    "realized_pnl": "0.000000",
                    "liquidation_price": "0",
                    "total_funding_paid_out": "-17.314739",
                    "margin_mode": 0, "allocated_margin": "0.000000"
                },
                {
                    "market_id": 16, "symbol": "SUI",
                    "initial_margin_fraction": "33.33",
                    "sign": -1, "position": "1382.1",
                    "avg_entry_price": "0.75987",
                    "position_value": "942.882441",
                    "unrealized_pnl": "107.336382",
                    "realized_pnl": "0.000000",
                    "liquidation_price": "1252.8958441209281",
                    "total_funding_paid_out": "10.905219",
                    "margin_mode": 0, "allocated_margin": "0.000000"
                },
                {
                    "market_id": 29, "symbol": "ENA",
                    "initial_margin_fraction": "33.33",
                    "sign": 1, "position": "125.5",
                    "avg_entry_price": "0.11002",
                    "position_value": "11.374065",
                    "unrealized_pnl": "-2.433445",
                    "liquidation_price": "0",
                    "total_funding_paid_out": "-0.327232",
                    "margin_mode": 0, "allocated_margin": "0.000000"
                },
                {
                    "market_id": 55, "symbol": "OP",
                    "initial_margin_fraction": "33.33",
                    "sign": 1, "position": "1640.9",
                    "avg_entry_price": "0.11977",
                    "position_value": "142.266030",
                    "unrealized_pnl": "-54.264563",
                    "liquidation_price": "0",
                    "total_funding_paid_out": "-3.851005",
                    "margin_mode": 0, "allocated_margin": "0.000000"
                },
                {
                    "market_id": 74, "symbol": "NMR",
                    "initial_margin_fraction": "33.33",
                    "sign": -1, "position": "0.85",
                    "avg_entry_price": "8.3569",
                    "position_value": "7.173065",
                    "unrealized_pnl": "-0.069700",
                    "liquidation_price": "1798562.881587889",
                    "total_funding_paid_out": "0.167017",
                    "margin_mode": 0, "allocated_margin": "0.000000"
                },
                {
                    "market_id": 104, "symbol": "STRK",
                    "initial_margin_fraction": "33.33",
                    "sign": -1, "position": "144.7",
                    "avg_entry_price": "0.04987",
                    "position_value": "3.591454",
                    "unrealized_pnl": "3.624735",
                    "liquidation_price": "11319.784492361827",
                    "total_funding_paid_out": "0.047498",
                    "margin_mode": 0, "allocated_margin": "0.000000"
                }
            ]
        }]
    })
}

/// Trimmed from a live `GET /orderBookOrders?market_id=1`, keeping the
/// two bids that rest at the same price.
fn captured_orders() -> Value {
    json!({
        "code": 200,
        "total_asks": 3,
        "asks": [
            { "order_id": "562953070761608", "owner_account_index": 27927,
              "initial_base_amount": "0.00020", "remaining_base_amount": "0.00020",
              "price": "64973.7" },
            { "order_id": "562953070761603", "owner_account_index": 27927,
              "initial_base_amount": "0.00021", "remaining_base_amount": "0.00020",
              "price": "64974.3" },
            { "order_id": "562953070761589", "owner_account_index": 702384,
              "initial_base_amount": "0.64351", "remaining_base_amount": "0.64335",
              "price": "64978.1" }
        ],
        "total_bids": 4,
        "bids": [
            { "order_id": "844421764488015", "owner_account_index": 702384,
              "initial_base_amount": "0.06759", "remaining_base_amount": "0.06759",
              "price": "64973.3" },
            { "order_id": "844421764488027", "owner_account_index": 702384,
              "initial_base_amount": "0.49604", "remaining_base_amount": "0.49604",
              "price": "64973.2" },
            { "order_id": "844421764488011", "owner_account_index": 726982,
              "initial_base_amount": "0.00308", "remaining_base_amount": "0.00308",
              "price": "64973.1" },
            { "order_id": "844421764488009", "owner_account_index": 726941,
              "initial_base_amount": "0.00308", "remaining_base_amount": "0.00308",
              "price": "64973.1" }
        ]
    })
}

/// Trimmed from a live `update/trade` on market 1: one taker sweeping
/// three resting asks, then a second taker's single print. The venue lists
/// them newest first, which is the order these are in.
fn captured_prints() -> Value {
    json!({
        "channel": "trade:1",
        "type": "update/trade",
        "liquidation_trades": [],
        "trades": [
            { "trade_id": 27_074_589_733_i64, "timestamp": 1_786_189_598_959_i64,
              "is_maker_ask": false, "bid_id": 844_421_764_109_748_i64,
              "ask_id": 562_953_071_100_242_i64, "price": "64940.6", "size": "0.00006" },
            { "trade_id": 27_074_589_705_i64, "timestamp": 1_786_189_598_601_i64,
              "is_maker_ask": true, "bid_id": 844_421_764_109_750_i64,
              "ask_id": 562_953_071_096_525_i64, "price": "64941.0", "size": "0.00228" },
            { "trade_id": 27_074_589_704_i64, "timestamp": 1_786_189_598_601_i64,
              "is_maker_ask": true, "bid_id": 844_421_764_109_750_i64,
              "ask_id": 562_953_071_096_526_i64, "price": "64940.9", "size": "0.00148" },
            { "trade_id": 27_074_589_703_i64, "timestamp": 1_786_189_598_601_i64,
              "is_maker_ask": true, "bid_id": 844_421_764_109_750_i64,
              "ask_id": 562_953_071_096_527_i64, "price": "64940.8", "size": "0.00068" }
        ]
    })
}

/// The L1 address the Lighter fixtures are drawn for: the owner of the
/// captured account, which is an address that genuinely holds a book here.
///
/// Harvested the way the venue offers them — `orderBookOrders` names an
/// `owner_account_index` on every resting order, and `account?by=index`
/// answers that account's `l1_address` — so it is a real participant rather
/// than an address chosen for looking like one. 702384 rests two of the four
/// bids in `captured_orders`.
pub fn demo_address_lighter() -> String {
    text(
        list(&captured_account(), "accounts")
            .first()
            .unwrap_or(&Value::Null),
        "l1_address",
    )
}

/// Lighter's universe as Lighter published it: its own tickers, its own price
/// scales, its own size steps and its own leverage caps — 50x on bitcoin
/// against a 3x on NMR, which is a spread the other venue's fixture does not
/// have.
pub fn demo_symbols_lighter() -> Vec<SymbolRow> {
    parse_symbols(&captured_universe(), &captured_rates())
}

/// The captured account, through the parser the live read uses. Every figure
/// on the equity strip is the venue's own, including the ones the app cannot
/// rebuild — `withdrawable` above all, which no other field pins down.
pub fn demo_account_lighter() -> Account {
    parse_account(&captured_account())
}

/// Its open positions, taken from the account rather than written beside it,
/// so the rows and the totals over them cannot disagree.
pub fn demo_positions_lighter() -> Vec<Position> {
    demo_account_lighter().positions
}

/// The captured bitcoin book, at Lighter's tick and with its levels folded the
/// way the venue serves them — resting orders rather than price levels, two of
/// which share a price.
pub fn demo_book_lighter() -> Book {
    parse_book(&captured_orders())
}

/// The captured prints, newest first, which is the order the panel holds a
/// tape in and the order `push_trades` leaves the feed's own beat in.
pub fn demo_tape_lighter() -> Vec<Trade> {
    let mut prints = parse_prints(&captured_prints());
    prints.reverse();
    prints
}

#[cfg(test)]
mod tests {
    use crate::hyperliquid::{apply_feed, symbol_row, tape_focus, tape_new};

    use super::*;

    /// The row of one market out of the captured universe, by name — because
    /// the rows come back in volume order and an index is a claim about the
    /// day rather than about the market being asserted on.
    fn listed_market(name: &str) -> SymbolRow {
        symbol_row(demo_symbols_lighter(), name.to_owned())
            .unwrap_or_else(|| panic!("{name} is not in the sample"))
    }

    // -----------------------------------------------------------------------
    // The order path.
    // -----------------------------------------------------------------------

    /// Every verdict below is one this deployment actually answered, recorded
    /// while the order path was written by posting to `testnet.zklighter` and
    /// reading what came back. They are the ladder the venue checks in: the
    /// body's shape, then each field, then the account, then the key, then the
    /// signature.
    fn refusal(code: i64, message: &str) -> Value {
        json!({ "code": code, "message": message })
    }

    /// What an acceptance looks like: the venue's own success shape, with the
    /// two figures it puts beside the hash. `predicted_execution_time_ms` is
    /// the venue saying in its own answer that the order has not executed yet.
    fn accepted(hash: &str) -> Value {
        json!({
            "code": 200,
            "tx_hash": hash,
            "predicted_execution_time_ms": 12,
            "volume_quota_remaining": 1_000_000,
        })
    }

    /// The id an order names a market by and the steps its figures are counted
    /// in come out of one row of one read.
    ///
    /// They have to. A price is sent as an integer count of the market's price
    /// steps, so a price paired with another market's decimals is out by a
    /// power of ten — an order at a tenth of the intended price, or ten times
    /// it, sent to a market that will happily take either. Reading them
    /// together is what makes that impossible rather than merely unlikely.
    #[test]
    fn a_markets_id_and_its_steps_come_from_the_same_row() {
        let table = parse_ids(&captured_universe());
        // The venue's own published pairs, live: bitcoin is market 1, priced to
        // a tenth and sized to five decimals; ether is market 0, priced to a
        // cent and sized to four.
        assert_eq!(
            table.get("BTC").copied(),
            Some(Market {
                id: 1,
                price_decimals: 1,
                size_decimals: 5
            }),
        );
        assert_eq!(
            table.get("ETH").copied(),
            Some(Market {
                id: 0,
                price_decimals: 2,
                size_decimals: 4
            }),
        );
        // And the two are genuinely per market rather than one figure serving
        // both: a coin worth a fraction of a cent is priced to six decimals and
        // sized in whole units, which is the opposite way round from bitcoin.
        assert_eq!(
            table.get("1000PEPE").copied(),
            Some(Market {
                id: 4,
                price_decimals: 6,
                size_decimals: 0
            }),
        );

        // A market missing its steps counts in whole units rather than
        // borrowing another market's, which is a refused order rather than a
        // wrong one.
        let bare = parse_ids(&json!({
            "order_book_details": [{ "symbol": "ICEONE", "market_id": 4_240 }],
        }));
        assert_eq!(
            bare.get("ICEONE").copied(),
            Some(Market {
                id: 4_240,
                price_decimals: 0,
                size_decimals: 0
            }),
        );
    }

    /// A refused transaction must never read as a sent one, and the venue's own
    /// sentence is the only useful thing to say about it.
    ///
    /// The verdict is the `code` rather than the HTTP status: every refusal
    /// here arrived as HTTP 400, so a parser reading the status would be right
    /// by accident on all of them and wrong the day one arrives as a 200 — and
    /// this venue does put a `code` on its 200s, which is why the read side
    /// already works this way.
    #[test]
    fn a_refused_transaction_never_reads_as_a_sent_one() {
        for (code, message) in [
            (BAD_TX_INFO, "invalid tx info"),
            (21602, "invalid market index"),
            (21701, "invalid base amount"),
            // The rule a bid far below the mark trips: the venue asks for a
            // minimum notional as well as a minimum size.
            (21706, "invalid order base or quote amount"),
            // And the band a limit order has to sit inside, which is the rule
            // an order priced to be unfillable trips.
            (21734, "limit order price is too far from the mark price"),
            // A registration whose L1 sentence recovers somebody else. It is
            // the only refusal here about a signature this app makes with an
            // Ethereum key, and it is what proves `personal_sign` frames the
            // message the way every wallet does: the venue recovers an address
            // from that string, so a byte out of place recovers a stranger.
            (21504, "fail to l1 signature"),
            (21702, "invalid price"),
            (21705, "invalid OrderTimeInForce"),
            (21104, "invalid nonce"),
            (NO_ACCOUNT_HERE, "account not found"),
            (KEY_UNKNOWN, "api key not found"),
            (BAD_SIGNATURE, "invalid signature"),
        ] {
            let refused = submitted(code, &refusal(code, message))
                .expect_err("the venue refused this transaction");
            assert!(
                refused.message.contains(message),
                "the venue's own sentence is what a reader needs: {}",
                refused.message
            );
            assert!(
                refused.message.contains(&code.to_string()),
                "and the code, which is what a support request quotes: {}",
                refused.message
            );
        }

        // A refusal the venue did not put a sentence on still has to read as a
        // refusal rather than as an empty success.
        let silent = submitted(21999, &json!({ "code": 21999 })).expect_err("still a refusal");
        assert!(silent.message.contains("21999"), "{}", silent.message);

        // And the accepted shape is the one thing that is not a refusal.
        assert_eq!(
            submitted(OK, &accepted("0xabc")).expect("the venue took it"),
            "0xabc"
        );
    }

    /// An acceptance with nothing to point at is refused.
    ///
    /// A `code` of 200 and no hash under it is not an outcome anybody can act
    /// on: there is nothing to look the transaction up by and nothing to say
    /// happened. It is also the shape a changed response takes, and reading it
    /// as success would report every submission as sent forever after.
    #[test]
    fn an_acceptance_with_no_hash_is_not_an_acceptance() {
        for body in [
            json!({ "code": 200 }),
            json!({ "code": 200, "tx_hash": "" }),
            json!({ "code": 200, "tx_hash": 7 }),
        ] {
            let refused = submitted(OK, &body).expect_err("an acceptance names a transaction");
            assert!(refused.message.contains("no hash"), "{}", refused.message);
        }
        assert!(submitted(OK, &accepted("0x01")).is_ok());
    }

    /// A figure that is not a whole number of the market's own steps is refused
    /// rather than rounded onto one.
    ///
    /// Rounding here is the failure with no symptom: the order goes out at a
    /// price nobody typed, rests there, and every screen afterwards shows the
    /// price the venue holds rather than the one that was meant. The other
    /// venue's signer takes the same stance on its decimal strings.
    #[test]
    fn a_figure_off_the_markets_step_is_refused_rather_than_rounded() {
        // Bitcoin on this venue: prices to a tenth, sizes to five decimals.
        assert_eq!(stepped("price", 64_912.5, 1).expect("on the tick"), 649_125);
        assert_eq!(stepped("size", 0.00021, 5).expect("on the step"), 21);
        // A whole-unit market counts its size in whole units.
        assert_eq!(stepped("size", 3.0, 0).expect("a whole coin"), 3);

        let refused = stepped("price", 64_912.55, 1).expect_err("half a tick is not a price");
        assert!(
            refused.message.contains("64912.55") && refused.message.contains("steps"),
            "the refusal names the figure and the rule: {}",
            refused.message
        );
        assert!(stepped("size", 0.000215, 5).is_err(), "half a step");

        // Nothing that is not a figure at all.
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(stepped("size", bad, 5).is_err(), "{bad} is not a size");
        }
    }

    /// The nonce read, live, on the deployment orders would go to.
    ///
    /// Two claims, and the second is the one worth the network. A nonce comes
    /// back at all — so the endpoint, its two query parameters and the field
    /// this reads are right. And it is *not* evidence the account exists: the
    /// same read answers `nonce: 0` for an index nobody has opened, so nothing
    /// downstream may treat a nonce as an account.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the nonce read"]
    fn the_nonce_read_answers_and_says_nothing_about_the_account() {
        crate::hyperliquid::open_the_wire();
        smol::block_on(async {
            // The faucet account on the test deployment.
            let held = lighter_nonce(Zone::Testnet, 637, 0)
                .await
                .expect("the venue answers a nonce");
            assert!(held >= 0, "a nonce counts up from zero");

            // An account index far past anything opened. It answers, which is
            // exactly why a nonce is not an account.
            assert!(
                lighter_nonce(Zone::Testnet, 281_474_976_710_000, 0)
                    .await
                    .is_ok(),
                "the nonce read answers for an account that does not exist, so \
                 nothing may infer one from it"
            );
        });
    }

    /// The venue reads what this module writes, established the only way it can
    /// be without an enrolled key: by submitting a correctly built, correctly
    /// signed transaction and being refused for the enrolment rather than for
    /// the shape.
    ///
    /// What `21109 api key not found` discriminates, exactly: the venue parsed
    /// the form, parsed `tx_info` as JSON, read every field, accepted the market
    /// index, the base amount, the price, the time-in-force, the nonce and the
    /// deadline, found the account — and then found no key registered under the
    /// index named. A transaction this module spelled wrong lands earlier and
    /// says which field (`21501`, `21602`, `21701`, `21702`, `21705`, `21104`),
    /// and one for an account that does not exist lands on `21100`. So this is
    /// evidence about the transaction's *shape*, and that is all it is.
    ///
    /// It is not evidence the signature is any good; the key here is registered
    /// on nothing, so a correct signature and 80 random bytes both land on
    /// 21109. The cryptography is pinned offline against the venue's own signer
    /// in `lighter_sign.rs`, which is where that claim belongs.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the submission reaches the key check"]
    fn a_well_formed_transaction_reaches_the_venues_key_check() {
        crate::hyperliquid::open_the_wire();
        // The throwaway key `lighter_sign.rs` pins its vectors against,
        // registered on no account anywhere.
        let key = PrivateKey::from_hex(
            "0x64ca3ac2840332193cf362603055e0808e039bc143e965f0b0aa922a1a4d40d5af86c4cb0cd07370",
        )
        .expect("the oracle key");
        smol::block_on(async {
            // A bid at a fraction of the mark, on an account whose key is not
            // enrolled: refused twice over before anything could rest.
            let refused = lighter_place(
                Zone::Testnet,
                &key,
                637,
                0,
                "BTC",
                Order {
                    oid: 0,
                    coin: "BTC".to_owned(),
                    buy: true,
                    price: 1_000.0,
                    size: 0.001,
                    ts: 0,
                },
                false,
                Resting::Deadline,
            )
            .await
            .expect_err("an unenrolled key cannot place an order");
            assert!(
                refused.message.contains(&KEY_UNKNOWN.to_string()),
                "expected the key check, got: {}",
                refused.message
            );

            // And a cancel, which is the other transaction type through the
            // same gates.
            let pulled = lighter_cancel(Zone::Testnet, &key, 637, 0, "BTC", 1)
                .await
                .expect_err("an unenrolled key cannot cancel either");
            assert!(
                pulled.message.contains(&KEY_UNKNOWN.to_string()),
                "expected the key check, got: {}",
                pulled.message
            );
        });
    }

    // -----------------------------------------------------------------------
    // A disposable identity, minted per run.
    //
    // The custody design exists to keep this app away from an account owner's
    // real wallet. That is a statement about *value*, not about key material —
    // so on a test deployment whose faucet funds any address that asks, the
    // honest way to get live evidence is for the tooling to own an identity of
    // its own and register its own API key. Nobody is asked for anything, and
    // nothing this key can reach is worth anything.
    //
    // **Testnet by construction.** Every request below goes through
    // `disposable_zone`, which is the only zone this tooling names, and it
    // refuses to be anything but a test deployment. That is the property rather
    // than a convention: an edit pointing this at mainnet fails
    // `a_disposable_identity_can_only_ever_touch_a_test_deployment` in the
    // ordinary suite, long before anything reaches a wallet.
    // -----------------------------------------------------------------------

    /// The one deployment a key this process minted may touch.
    fn disposable_zone() -> Zone {
        let zone = Zone::Testnet;
        assert!(
            zone.testnet(),
            "a disposable L1 key may only ever reach a deployment where being \
             wrong costs nothing",
        );
        zone
    }

    /// The slot this tooling registers its key at. Any would do on an account
    /// nobody else has ever touched; naming one keeps the transcript readable.
    const DISPOSABLE_SLOT: u8 = 2;

    /// An account nobody owns, funded by the faucet, with an API key this
    /// process registered for it.
    struct Disposable {
        key: PrivateKey,
        account: i64,
        slot: u8,
    }

    /// Mint one, end to end.
    ///
    /// Four acts, and the third is the one the owner would otherwise have to
    /// perform: the registration is authorised by an L1 signature over the
    /// venue's own sentence, and the L1 wallet here is one this function made a
    /// moment ago.
    fn disposable_identity() -> Disposable {
        let zone = disposable_zone();
        let wallet = Wallet::generate();
        let address = wallet.address().to_string();

        // 1. An account, from a faucet that asks nothing of the address.
        eprintln!("minted a disposable L1 wallet: {address}");
        smol::block_on(get(zone, format!("faucet?l1_address={address}")))
            .expect("the faucet funds any address on this deployment");

        // 2. Which index it was given. The faucet answers before the account is
        //    readable, so this is a poll rather than a read.
        let account = settle("the faucet's account", || {
            smol::block_on(lighter_account_index(zone, address.clone())).expect("the account read")
        });
        eprintln!("the faucet opened account {account} for it");

        // 3. A key, and the registration that puts it in a slot. The digest is
        //    signed by the new key — proof it is held — and the sentence by the
        //    L1 wallet, which is what says the account agreed.
        let mut secret = [0u8; 40];
        let key = loop {
            getrandom::fill(&mut secret).expect("OS entropy");
            if let Ok(key) = crate::lighter_sign::PrivateKey::from_hex(&hex::encode(secret)) {
                break key;
            }
        };
        let registration = crate::lighter_sign::Registration {
            account,
            api_key: DISPOSABLE_SLOT,
            public_key: key.public_key(),
            deadline_ms: now_ms() + TX_LIFETIME_MS,
            nonce: smol::block_on(lighter_nonce(zone, account, DISPOSABLE_SLOT))
                .expect("the nonce for a fresh slot"),
        };
        let l1 = wallet
            .personal_sign(&crate::lighter_sign::registration_body(&registration))
            .hex();
        let built = crate::lighter_sign::change_pub_key(zone, &registration, &l1)
            .expect("a buildable registration");
        smol::block_on(send_tx(zone, &built, &key)).expect("the venue takes the registration");

        // 4. The venue's own word on it, which is what custody would ask for
        //    too: our public key, in a slot the listing names.
        let ours = hex::encode(key.public_key());
        let slot = settle("the registered key", || {
            smol::block_on(lighter_api_keys(zone, account))
                .expect("the key listing")
                .into_iter()
                .find(|(_, public)| public.eq_ignore_ascii_case(&ours))
                .map(|(slot, _)| slot)
        });
        assert_eq!(
            slot, DISPOSABLE_SLOT,
            "the venue put the key where it was asked"
        );
        eprintln!("registered {ours} as api key {slot} on account {account}");
        Disposable { key, account, slot }
    }

    /// Poll until the venue answers, or say what never arrived.
    ///
    /// The sequencer takes a transaction before it has applied it, so every
    /// read that follows one is a poll. Thirty seconds because a testnet
    /// sequencer is not a fast one, and a bare `sleep` long enough to always
    /// work would make every run pay for the worst one.
    fn settle<T>(what: &str, mut read: impl FnMut() -> Option<T>) -> T {
        for _ in 0..60 {
            if let Some(answer) = read() {
                return answer;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        panic!("{what} never arrived");
    }

    /// The disposable identity can only ever be pointed at a test deployment.
    ///
    /// Cheap, offline, and in the ordinary suite on purpose: it is the guard
    /// that makes the live test below safe to own an L1 key at all. An edit
    /// pointing this tooling at mainnet fails here rather than at a faucet.
    #[test]
    fn a_disposable_identity_can_only_ever_touch_a_test_deployment() {
        let zone = disposable_zone();
        assert!(zone.testnet());
        assert_eq!(zone, Zone::Testnet);
        assert!(
            zone.api_url().contains("testnet.") && zone.stream_url().contains("testnet."),
            "every endpoint this tooling reaches names the test deployment: {}",
            zone.api_url(),
        );
        assert_ne!(zone.chain_id(), Zone::Mainnet.chain_id());
    }

    /// The whole round trip on the test deployment, with nobody asked for
    /// anything: an identity minted here, an API key it registered for itself,
    /// an order placed, found resting under the index it was placed with, and
    /// pulled again.
    ///
    /// The order rests far from the market on purpose: a bid at a fraction of
    /// the mark cannot fill, so the round trip ends with the book exactly as it
    /// started.
    ///
    /// It reads the book back through `accountActiveOrders`, which is a gated
    /// read and wants the token `lighter_sign.rs` already mints. That read is
    /// built here rather than published, because the app's own orders panel
    /// does not make it and a function with one caller in a test is a seam
    /// nothing crosses.
    #[test]
    #[ignore = "mints a testnet identity and places a real order on Lighter testnet"]
    fn the_order_path_places_rests_and_cancels_on_the_test_deployment() {
        crate::hyperliquid::open_the_wire();
        let zone = disposable_zone();
        let held = disposable_identity();

        smol::block_on(async {
            let markets = lighter_symbols(zone).await.expect("the testnet universe");
            let btc = markets
                .iter()
                .find(|row| row.name == "BTC")
                .expect("the test deployment lists BTC");
            let step = market_of(zone, "BTC").await.expect("the market");

            // A tenth under the mark, rounded onto the market's own tick so
            // nothing is refused for a price it cannot spell. Two live rules
            // bound this from both sides and the order has to sit between them:
            // a bid far below the mark is refused outright (`21734`, the
            // venue's own price band), and one too near it would fill. Ten
            // percent clears the band and is nowhere near the book.
            let tick = 10f64.powi(step.price_decimals as i32);
            let price = (btc.price * 0.9 * tick).round() / tick;
            // And big enough to clear the market's minimum *notional*, which is
            // the other rule an order priced under the mark trips: the venue
            // asks for ten dollars of it, and answers `21706` below that.
            let size = 0.01;
            let index = lighter_place(
                zone,
                &held.key,
                held.account,
                held.slot,
                "BTC",
                Order {
                    oid: 0,
                    coin: "BTC".to_owned(),
                    buy: true,
                    price,
                    size,
                    ts: 0,
                },
                false,
                Resting::Deadline,
            )
            .await
            .expect("the venue takes the order");
            eprintln!("placed {size} BTC at {price} as client order {index}");

            // The submission is a receipt rather than a fill, so the book is
            // what says the order rested.
            settle("the resting order", || {
                resting(zone, &held, step.id).contains(&index).then_some(())
            });
            eprintln!("the book lists it resting");

            lighter_cancel(zone, &held.key, held.account, held.slot, "BTC", index)
                .await
                .expect("the venue takes the cancel");
            settle("the cancellation", || {
                (!resting(zone, &held, step.id).contains(&index)).then_some(())
            });
            eprintln!("cancelled, and the book stops listing it");
        });
    }

    /// The client order indices this account is resting in one market, read
    /// through the gated endpoint with a token minted for the same key.
    fn resting(zone: Zone, held: &Disposable, market: i64) -> Vec<i64> {
        let deadline = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_secs()
            + 600;
        let token = crate::lighter_sign::auth_token(&held.key, held.account, held.slot, deadline)
            .expect("a token inside the venue's window");
        let mut response = agent()
            .get(&format!(
                "{}/accountActiveOrders?account_index={}&market_id={market}",
                zone.api_url(),
                held.account,
            ))
            .header("Authorization", &token)
            .call()
            .expect("Lighter reachable");
        let body: Value = response.body_mut().read_json().expect("a JSON answer");
        assert_eq!(
            body.get("code").and_then(Value::as_i64),
            Some(OK),
            "the gated read refused this token: {body}"
        );
        list(&body, "orders")
            .iter()
            .map(|order| value_i64(order, "client_order_index"))
            .collect()
    }

    /// The step a size is quoted to belongs to the market, and each venue
    /// publishes its own. Reading it off the wrong field, or defaulting it,
    /// would let the app work out a size the venue then refuses.
    #[test]
    fn a_market_carries_the_step_the_venue_quotes_it_at() {
        // The venue's own published steps: bitcoin to five decimals, ether to
        // four, sol to three, aster to a tenth of a coin — and a coin worth a
        // fraction of a cent trades in whole units.
        assert_eq!(listed_market("BTC").size_decimals, 5);
        assert_eq!(listed_market("ETH").size_decimals, 4);
        assert_eq!(listed_market("SOL").size_decimals, 3);
        assert_eq!(listed_market("ASTER").size_decimals, 1);
        assert_eq!(listed_market("1000PEPE").size_decimals, 0);
    }

    #[test]
    fn markets_sort_by_volume_and_drop_the_delisted() {
        let rows = demo_symbols_lighter();
        assert_eq!(
            rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
            [
                "BTC", "ETH", "SOL", "ENA", "SUI", "AAPL", "ASTER", "1000PEPE", "OP", "STRK", "NMR"
            ],
            "busiest first, and MKR is not tradeable"
        );
    }

    #[test]
    fn margin_fractions_read_as_basis_points() {
        // 120 bp of a position's value is what the engine holds against
        // bitcoin, and 200 bp is the smallest initial margin, so 50x is the
        // ceiling.
        assert!((listed_market("BTC").maintenance - 0.012).abs() < 1e-12);
        assert!((listed_market("BTC").leverage - 50.0).abs() < 1e-12);
        // A market the venue holds a hundredfold tighter, so a fixture that
        // read the scale wrong could not pass both.
        assert!((listed_market("ASTER").maintenance - 0.12).abs() < 1e-12);
        assert!((listed_market("ASTER").leverage - 5.0).abs() < 1e-12);
        // And the tightest of them, whose 3333 bp is not a round cap.
        assert!((listed_market("NMR").maintenance - 0.2).abs() < 1e-12);
        assert!((listed_market("NMR").leverage - 10_000.0 / 3_333.0).abs() < 1e-12);
    }

    /// The check that would catch the fraction/percent slip: a previous close
    /// outside the day's own range is the arithmetic saying it read the change
    /// in the wrong unit. Every market in the sample, against the low and high
    /// that market itself published, because the price twenty-four hours ago
    /// is inside the last twenty-four hours' range by construction.
    #[test]
    fn yesterdays_close_lands_inside_the_days_range() {
        for detail in list(&captured_universe(), "order_book_details") {
            let name = text(detail, "symbol");
            if text(detail, "status") != "active" {
                continue;
            }
            let row = listed_market(&name);
            let range = num(detail, "daily_price_low")..=num(detail, "daily_price_high");
            assert!(
                range.contains(&row.prev),
                "{name} previous close {} is outside {range:?}",
                row.prev
            );
            // And it still reconstructs the mark it was backed out of.
            let round_trip = row.prev * (1.0 + row.change_pct / 100.0);
            assert!((round_trip - row.price).abs() < 1e-6);
        }
    }

    #[test]
    fn funding_is_the_hourly_share_of_the_published_eight_hour_rate() {
        let btc = listed_market("BTC");
        // 6.4e-05 over eight hours is 8.0e-06 an hour, which is 0.0008%.
        assert!((btc.funding_pct - 6.4e-05 / 8.0 * 100.0).abs() < 1e-15);
        assert!((btc.funding_pct - 0.000_8).abs() < 1e-12);
        assert_eq!(btc.volume, 618_847_551.336_845);
    }

    #[test]
    fn sign_and_size_become_one_signed_size() {
        let held = parse_account(&captured_account()).positions;
        assert!((held[0].size - 3.87333).abs() < 1e-12, "sign 1 is a long");
        assert!((held[1].size + 1382.1).abs() < 1e-12, "sign -1 is a short");
        // Neither field alone is the size, and the short is the one that
        // proves it: 1382.1 unsigned is the same string a long would send.
        assert!(held[1].size < 0.0);
    }

    #[test]
    fn a_position_is_marked_at_what_the_venue_says_it_is_worth() {
        let held = parse_account(&captured_account()).positions;
        assert!((held[0].mark - 251_677.750_743 / 3.87333).abs() < 1e-9);
        // A short is worth a positive amount, so the mark stays positive.
        assert!((held[1].mark - 942.882_441 / 1382.1).abs() < 1e-12);
        assert!(held[1].mark > 0.0);
    }

    #[test]
    fn cross_positions_carry_their_share_of_the_accounts_requirement() {
        let parsed = parse_account(&captured_account());
        let margin: f64 = parsed.positions.iter().map(|held| held.margin).sum();
        // The identity that fixed the unit: the per-position initial margin
        // sums to the total the venue reports for the cross book.
        assert!(
            (margin - 84_253.253_098).abs() < 1e-5,
            "summed margin {margin} should be the reported requirement"
        );
        assert_eq!(parsed.positions[0].margin_mode, "cross");
        // 33.33% initial margin is 3x, and nothing here rounds it to 3.
        assert!((parsed.positions[0].leverage - 100.0 / 33.33).abs() < 1e-12);
    }

    #[test]
    fn return_on_equity_is_the_pnl_over_what_was_posted_to_open() {
        let held = parse_account(&captured_account()).positions;
        let opening = 64_969.5 * 3.87333 * 0.3333;
        assert!((held[0].roe_pct - 29.306_261 / opening * 100.0).abs() < 1e-12);
        // Both are gains here, so both read positive whichever way they lean.
        assert!(held[0].roe_pct > 0.0 && held[1].roe_pct > 0.0);
    }

    /// `Position.funding` is funding charged, which is what `hyperliquid`
    /// puts there from `cumFunding.sinceOpen`. Lighter publishes the opposite
    /// quantity under `total_funding_paid_out`, so the parse has to turn it
    /// over; the expected values are the fixture's own figures negated rather
    /// than copies, so dropping the negation cannot pass.
    #[test]
    fn funding_is_the_cost_charged_rather_than_the_cash_paid_out() {
        let raw = captured_account();
        let published = |index: usize| {
            num(
                &raw["accounts"][0]["positions"][index],
                "total_funding_paid_out",
            )
        };
        let held = parse_account(&raw).positions;
        for index in [0, 1] {
            assert_eq!(held[index].funding, -published(index));
        }
        assert!(held[0].funding > 0.0, "the BTC long was charged funding");
        assert!(held[1].funding < 0.0, "the SUI short collected it");
    }

    #[test]
    fn the_account_totals_are_the_venues_own_and_agree_with_its_positions() {
        let parsed = parse_account(&captured_account());
        assert!((parsed.value - 1_837_610.461_943).abs() < 1e-6);
        assert!((parsed.withdrawable - 1_753_357.208_845).abs() < 1e-6);
        assert!((parsed.maintenance - 3_084.930_401).abs() < 1e-9);
        // Equity is collateral plus what the open positions have made. The
        // residual is 1e-6 on 1.8 million, which is the venue rounding the
        // figures it publishes to six decimals rather than anything here.
        assert!((parsed.value - (1_837_526.962_274 + parsed.pnl)).abs() < 1e-5);
        // Withdrawable is equity less what the cross book has to keep.
        assert!((parsed.value - parsed.withdrawable - 84_253.253_098).abs() < 1e-5);
        let notional =
            251_677.750_743 + 942.882_441 + 11.374_065 + 142.266_030 + 7.173_065 + 3.591_454;
        assert!((parsed.notional - notional).abs() < 1e-6);
    }

    /// The identity that established the basis-point reading, run rather than
    /// only described: every position's value times its own market's
    /// maintenance fraction, summed, is the requirement the venue publishes
    /// for the account. Read as a percentage or as a raw fraction it misses by
    /// a hundredfold either way, so nothing else can pass this.
    #[test]
    fn maintenance_in_basis_points_reproduces_the_accounts_requirement() {
        // The fraction is not restated here: it is read out of the captured
        // universe, which is the same `/orderBookDetails` the account snapshot
        // was taken beside. So this is also the check that the two payloads
        // describe one terminal rather than two moments.
        let parsed = parse_account(&captured_account());
        let held: f64 = parsed
            .positions
            .iter()
            .map(|position| {
                let market = listed_market(&position.coin);
                position.mark * position.size.abs() * market.maintenance
            })
            .sum();
        assert!(
            (held - 3_084.930_401).abs() < 1e-5,
            "rebuilt maintenance {held} should be the published requirement"
        );
        assert!((held - parsed.maintenance).abs() < 1e-5);
    }

    #[test]
    fn account_health_is_the_requirement_against_the_cross_equity() {
        let parsed = parse_account(&captured_account());
        let load = 3_084.930_401 / 1_837_610.461_943;
        assert!((parsed.margin_pct - load * 100.0).abs() < 1e-9);
        assert!((parsed.health - load * RISK_RAIL_WIDTH).abs() < 1e-9);
        // A quarter of a percent of equity at risk is not a rail worth drawing.
        assert!(parsed.margin_pct < 1.0);
    }

    /// A live `GET /account?by=l1_address&value=0xA1265E…F2813`, which answers
    /// with two accounts: the address's own book and a sub-account. Trimmed in
    /// positions — 6 of the 22 the main book carries — but not in accounts,
    /// and every figure is the venue's. Three of the six positions are closed
    /// rows the venue keeps at size zero.
    fn addressed() -> Value {
        json!({
            "code": 200,
            "total": 2,
            "accounts": [
                {
                    "account_index": 270812,
                    "account_type": 0,
                    "l1_address": "0xA1265E2554Cf9F2fBDdf0374d60d4cCB5D1F2813",
                    "collateral": "250101.646925",
                    "available_balance": "247488.977538",
                    "total_asset_value": "251220.50086300002",
                    "cross_asset_value": "250016.300132",
                    "cross_initial_margin_requirement": "2827.637933",
                    "cross_maintenance_margin_requirement": "1696.593798",
                    "positions": [
                        {
                            "market_id": 0, "symbol": "ETH",
                            "initial_margin_fraction": "2.00",
                            "sign": -1, "position": "2.1940",
                            "avg_entry_price": "1905.86",
                            "position_value": "4198.043480",
                            "unrealized_pnl": "-16.592368",
                            "liquidation_price": "113752.61958393085",
                            "total_funding_paid_out": "8.034270",
                            "margin_mode": 0, "allocated_margin": "0.000000"
                        },
                        {
                            "market_id": 45, "symbol": "PUMP",
                            "initial_margin_fraction": "10.00",
                            "sign": 1, "position": "0",
                            "avg_entry_price": "0.000000",
                            "position_value": "-0.000000",
                            "unrealized_pnl": "0.000000",
                            "liquidation_price": "0",
                            "margin_mode": 0, "allocated_margin": "0.000000"
                        },
                        {
                            "market_id": 90, "symbol": "ZEC",
                            "initial_margin_fraction": "10.00",
                            "sign": -1, "position": "1.917",
                            "avg_entry_price": "509.636",
                            "position_value": "968.368716",
                            "unrealized_pnl": "8.603496",
                            "liquidation_price": "122708.5250995315",
                            "total_funding_paid_out": "0.034989",
                            "margin_mode": 0, "allocated_margin": "0.000000"
                        },
                        {
                            "market_id": 93, "symbol": "XAG",
                            "initial_margin_fraction": "4.00",
                            "sign": 1, "position": "0.00",
                            "avg_entry_price": "0.0000",
                            "position_value": "-0.000000",
                            "unrealized_pnl": "0.000000",
                            "liquidation_price": "0",
                            "margin_mode": 1, "allocated_margin": "0.000000"
                        },
                        {
                            "market_id": 94, "symbol": "MEGA",
                            "initial_margin_fraction": "33.33",
                            "sign": 1, "position": "6443.9",
                            "avg_entry_price": "0.03488",
                            "position_value": "225.794256",
                            "unrealized_pnl": "1.058630",
                            "liquidation_price": "0.029280283485156502",
                            "total_funding_paid_out": "-0.131240",
                            "margin_mode": 1, "allocated_margin": "73.792252"
                        },
                        {
                            "market_id": 131, "symbol": "AXS",
                            "initial_margin_fraction": "20.00",
                            "sign": 1, "position": "0.00",
                            "avg_entry_price": "0.0000",
                            "position_value": "-0.000000",
                            "unrealized_pnl": "0.000000",
                            "liquidation_price": "0",
                            "margin_mode": 0, "allocated_margin": "0.000000"
                        }
                    ]
                },
                {
                    // Sub-account indices start past 2^48, so this one needs
                    // the wider literal `json!` will not infer.
                    "account_index": 281_474_976_627_905_i64,
                    "account_type": 1,
                    "l1_address": "0xA1265E2554Cf9F2fBDdf0374d60d4cCB5D1F2813",
                    "collateral": "0.000000",
                    "available_balance": "0.000000",
                    "total_asset_value": "0",
                    "cross_asset_value": "0",
                    "cross_initial_margin_requirement": "0.000000",
                    "cross_maintenance_margin_requirement": "0.000000",
                    "positions": [
                        {
                            "market_id": 110, "symbol": "NVDA",
                            "initial_margin_fraction": "10.00",
                            "sign": 1, "position": "0.000",
                            "avg_entry_price": "0.000",
                            "position_value": "-0.000000",
                            "unrealized_pnl": "0.000000",
                            "liquidation_price": "0",
                            "margin_mode": 1, "allocated_margin": "0.000000"
                        }
                    ]
                }
            ]
        })
    }

    /// The address's own book is the one the panel means, and it is found by
    /// `account_type` rather than by arriving first — so the same payload with
    /// its accounts the other way round has to read identically. The venue
    /// happens to send the main account first, which is exactly why position
    /// alone cannot be the test.
    #[test]
    fn an_address_with_sub_accounts_reads_its_own_book() {
        let payload = addressed();
        let parsed = parse_account(&payload);
        assert!((parsed.value - 251_220.500_863).abs() < 1e-6);
        assert!((parsed.maintenance - 1_696.593_798).abs() < 1e-9);

        let mut reversed = payload.clone();
        reversed["accounts"]
            .as_array_mut()
            .expect("two accounts")
            .reverse();
        let swapped = parse_account(&reversed);
        assert_eq!(
            swapped.value, parsed.value,
            "the sub-account is not the book"
        );
        assert_eq!(swapped.positions.len(), parsed.positions.len());
    }

    /// Lighter never drops a market it has traded, so the list carries closed
    /// rows at size zero. They are positions in name only: no size, no value,
    /// no PnL, and nothing to close.
    #[test]
    fn markets_the_account_has_left_are_not_open_positions() {
        let parsed = parse_account(&addressed());
        assert_eq!(
            parsed
                .positions
                .iter()
                .map(|held| held.coin.as_str())
                .collect::<Vec<_>>(),
            ["ETH", "ZEC", "MEGA"],
            "PUMP, XAG and AXS are closed rows"
        );
        assert!(parsed.positions.iter().all(|held| held.size != 0.0));
        // And the totals are taken over what is actually held.
        let notional: f64 = 4_198.043_480 + 968.368_716 + 225.794_256;
        assert!((parsed.notional - notional).abs() < 1e-6);
    }

    /// The view geometry above is copied because `hyperliquid` publishes no
    /// `pub const`, so these cannot be imported. This pins the copies to
    /// numbers that module itself produces rather than to a second copy of
    /// them. `BOOK_DEPTH` has no such anchor — nothing public there truncates
    /// a book — so drift in it is only visible by reading both files.
    #[test]
    fn the_view_geometry_matches_what_hyperliquid_draws() {
        // The deepest level of any book carries the whole bar by construction,
        // so `demo_book` renders that module's own width.
        let drawn = crate::hyperliquid::demo_book();
        assert_eq!(drawn.bids.last().expect("a bid side").bar, BOOK_BAR_WIDTH);

        // An account with no equity left against a requirement it still owes
        // is fully loaded, so its rail is the whole of that module's width.
        let spent = Account {
            cross_value: 0.0,
            maintenance: 1.0,
            ..parse_account_body(&Value::Null)
        };
        let marked =
            crate::hyperliquid::mark_account(Some(spent), Vec::new()).expect("an account back");
        assert_eq!(marked.margin_pct, 100.0, "nothing left is fully loaded");
        assert_eq!(marked.health, RISK_RAIL_WIDTH);
    }

    /// A real isolated position, from `GET /account?by=index&value=270812` —
    /// the account that proved which `margin_mode` is which.
    fn isolated() -> Value {
        json!({
            "market_id": 94, "symbol": "MEGA",
            "initial_margin_fraction": "33.33",
            "sign": 1, "position": "6443.9",
            "avg_entry_price": "0.03488",
            "position_value": "226.374207",
            "unrealized_pnl": "1.638581",
            "liquidation_price": "0.029280283485156502",
            "total_funding_paid_out": "-0.131240",
            "margin_mode": 1, "allocated_margin": "73.792252"
        })
    }

    #[test]
    fn an_isolated_position_posts_its_own_margin() {
        let held = parse_position(&isolated());
        assert_eq!(held.margin_mode, "isolated");
        // Its own collateral, not its share of the account's requirement —
        // which for this position would have been 75.46.
        assert!((held.margin - 73.792_252).abs() < 1e-12);
        assert!((held.margin - 226.374_207 * 0.3333).abs() > 1.0);
    }

    /// The rail on a position that has a cliff to run at. This one is long
    /// from 0.03488 with liquidation at 0.02928, and marked above its entry,
    /// so it has travelled nowhere toward it.
    #[test]
    fn the_risk_rail_reads_zero_when_the_mark_is_the_safe_side_of_entry() {
        let held = parse_position(&isolated());
        assert!(held.mark > held.entry);
        assert_eq!(held.risk, 0.0);
        // And a position the venue quotes no liquidation for has no rail.
        let flat = parse_account(&captured_account()).positions;
        assert_eq!(flat[0].liq, 0.0);
        assert_eq!(flat[0].risk, 0.0);
    }

    #[test]
    fn orders_at_one_price_are_one_level() {
        let book = parse_book(&captured_orders());
        assert_eq!(book.bids.len(), 3, "four orders rest at three prices");
        assert!((book.bids[2].price - 64_973.1).abs() < 1e-9);
        assert!((book.bids[2].size - 0.00616).abs() < 1e-12, "0.00308 twice");
    }

    #[test]
    fn depth_accumulates_down_the_side() {
        let book = parse_book(&captured_orders());
        assert!((book.bids[0].total - 0.06759).abs() < 1e-12);
        assert!((book.bids[1].total - (0.06759 + 0.49604)).abs() < 1e-12);
        let deepest = 0.06759 + 0.49604 + 0.00616;
        assert!((book.bids[2].total - deepest).abs() < 1e-12);
        // The bar is the level's share of the deepest, so the last is full.
        assert!((book.bids[2].bar - BOOK_BAR_WIDTH).abs() < 1e-9);
        assert!(book.bids[0].bar < book.bids[1].bar);
    }

    /// A partly filled order is only depth for what is left of it.
    #[test]
    fn a_level_counts_what_remains_rather_than_what_was_placed() {
        let book = parse_book(&captured_orders());
        assert!((book.asks[2].size - 0.64335).abs() < 1e-12);
        assert!((book.asks[1].size - 0.00020).abs() < 1e-12);
    }

    #[test]
    fn the_spread_is_between_the_two_best_prices() {
        let book = parse_book(&captured_orders());
        assert!((book.spread - (64_973.7 - 64_973.3)).abs() < 1e-9);
        assert!((book.mid - (64_973.7 + 64_973.3) / 2.0).abs() < 1e-9);
        assert!((book.spread_pct - book.spread / book.mid * 100.0).abs() < 1e-12);
    }

    /// The one thing the captured payloads cannot check: that the URLs are
    /// still the venue's, that the `code` guard lets a good body through, and
    /// that the ticker-to-id table resolves a book. Kept out of the default run
    /// because it fails on a train rather than on a bug.
    #[test]
    #[ignore = "hits the live venue, run explicitly: checks the URLs still answer"]
    fn the_requests_reach_the_venue() {
        crate::hyperliquid::open_the_wire();
        smol::block_on(async {
            let rows = lighter_symbols(Zone::Mainnet).await.expect("markets");
            assert!(rows.len() > 100, "the venue lists a couple hundred markets");
            assert!(rows[0].price > 0.0 && rows[0].maintenance > 0.0);
            // Sorted, so the busiest market is the one the id table is asked
            // for, and it has to resolve without a second universe fetch.
            let book = lighter_book(Zone::Mainnet, rows[0].name.clone())
                .await
                .expect("book");
            assert!(book.mid > 0.0 && !book.bids.is_empty() && !book.asks.is_empty());
            assert!(book.asks[0].price > book.bids[0].price, "crossed book");
            // The address the fixtures are drawn for, which has to be one the
            // venue actually holds a book for — a Lighter screen drawn for an
            // address with no Lighter account is a fixture pretending twice.
            let Ok(Some(account)) = lighter_account(Zone::Mainnet, demo_address_lighter()).await
            else {
                panic!("the fixture address has no account on Lighter");
            };
            assert!(account.value > 0.0);

            // The three answers this read has, and the two that are not
            // failures. An address with a real account at the *other* exchange
            // and none here is the ordinary case of one address read at two
            // venues: `21100 account not found` is Lighter answering that
            // there is nothing here, and drawn as a failure it would put an
            // alarm over a working screen.
            assert!(
                matches!(
                    lighter_account(
                        Zone::Mainnet,
                        "0x8cc94dc843e1ea7a19805e0cca43001123512b6a".to_owned()
                    )
                    .await,
                    Ok(None)
                ),
                "an address with no account here is an absence, not a failure"
            );

            // A refusal that *is* one quotes the venue instead of a bare
            // number. This is the half that cannot be checked from a captured
            // payload: ureq makes every 4xx an error and drops the body, and
            // the body is the only place Lighter says what was wrong — the
            // same 400 carries both `21100` and this.
            // `Account` has no `Debug`, so the error comes out of a match
            // rather than `expect_err`.
            let Err(refused) = lighter_account(Zone::Mainnet, "0xdead".to_owned()).await else {
                panic!("the venue should not accept 0xdead as an address");
            };
            assert!(
                refused.message.contains("21103")
                    && refused.message.contains("invalid account l1 address"),
                "the refusal should carry the venue's own words: {}",
                refused.message
            );
        });
    }

    /// Median nanoseconds per call over `rounds` batches.
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

    /// The universe at the size the venue actually publishes it, built from
    /// the same fields the trimmed fixture carries.
    #[cfg(not(debug_assertions))]
    fn wide_details(markets: usize) -> Value {
        let rows: Vec<Value> = (0..markets)
            .map(|index| {
                json!({
                    "symbol": format!("SYM{index}"), "market_id": index as i64,
                    "market_type": "perp", "status": "active", "size_decimals": 3,
                    "maintenance_margin_fraction": 120,
                    "min_initial_margin_fraction": 200,
                    "default_initial_margin_fraction": 500,
                    "mark_price": format!("{}.3", 64_000 + index),
                    "index_price": format!("{}.1", 64_000 + index),
                    "daily_price_change": 0.6118286879673691,
                    "daily_quote_token_volume": 618_847_551.336_845 - index as f64,
                    "daily_base_token_volume": 9520.35,
                    "daily_price_low": 64_270.5, "daily_price_high": 65_349.4,
                    "open_interest": 1836.69392
                })
            })
            .collect();
        json!({ "code": 200, "order_book_details": rows })
    }

    /// Lighter republishes the rates of the venues it indexes, so the payload
    /// is several times the size of the part that is kept.
    #[cfg(not(debug_assertions))]
    fn wide_rates(markets: usize) -> Value {
        let mut rows = Vec::with_capacity(markets * 3);
        for index in 0..markets {
            for exchange in ["binance", "hyperliquid", "lighter"] {
                rows.push(json!({
                    "market_id": index as i64, "exchange": exchange,
                    "symbol": format!("SYM{index}"), "rate": 6.4e-05
                }));
            }
        }
        json!({ "code": 200, "funding_rates": rows })
    }

    /// `orderBookOrders` serves resting orders rather than levels, and the
    /// adapter asks for `ORDER_FETCH` of them a side to fold into `BOOK_DEPTH`.
    #[cfg(not(debug_assertions))]
    fn wide_book(orders: usize) -> Value {
        let side = |sign: f64| -> Vec<Value> {
            (0..orders)
                .map(|step| {
                    json!({
                        "price": format!("{:.1}", 64_000.0 + sign * (step / 20) as f64),
                        "remaining_base_amount": "0.31",
                        "initial_base_amount": "0.50",
                    })
                })
                .collect()
        };
        json!({ "code": 200, "bids": side(-1.0), "asks": side(1.0) })
    }

    /// What Lighter costs the terminal per response. It has no websocket: the
    /// three reads are polled, so its "per message" is one HTTP body parsed
    /// and folded, and its "per beat" is whatever the poll interval asks for.
    ///
    ///     cargo test --release -p trading-example -- --ignored --nocapture feed_cost
    #[test]
    #[ignore = "feed-cost probe, run explicitly: prints per-message costs, asserts nothing"]
    #[cfg(not(debug_assertions))]
    fn feed_cost_lighter() {
        const MARKETS: usize = 222;
        const ROUNDS: usize = 40;

        let details = wide_details(MARKETS);
        let rates = wide_rates(MARKETS);
        let book = wide_book(ORDER_FETCH);
        let held = captured_account();
        let details_text = details.to_string();
        let rates_text = rates.to_string();
        let book_text = book.to_string();

        eprintln!("\nlighter reads, {MARKETS} markets, {ORDER_FETCH} orders a side");
        eprintln!(
            "{:<34} {:>7} bytes  orderBookDetails",
            "universe payload",
            details_text.len()
        );
        eprintln!(
            "{:<34} {:>7} bytes  funding-rates",
            "funding payload",
            rates_text.len()
        );
        eprintln!(
            "{:<34} {:>7} bytes  orderBookOrders",
            "book payload",
            book_text.len()
        );
        eprintln!(
            "{:<34} {:>9.0}ns  serde_json::from_str",
            "universe parse to Value",
            per_call(20, ROUNDS, || {
                std::hint::black_box(serde_json::from_str::<Value>(&details_text).unwrap());
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  serde_json::from_str",
            "funding parse to Value",
            per_call(20, ROUNDS, || {
                std::hint::black_box(serde_json::from_str::<Value>(&rates_text).unwrap());
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  serde_json::from_str",
            "book parse to Value",
            per_call(200, ROUNDS, || {
                std::hint::black_box(serde_json::from_str::<Value>(&book_text).unwrap());
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  parse_symbols",
            "universe fold to rows",
            per_call(20, ROUNDS, || {
                std::hint::black_box(parse_symbols(&details, &rates));
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  parse_ids",
            "universe fold to the id table",
            per_call(20, ROUNDS, || {
                std::hint::black_box(parse_ids(&details));
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  parse_book",
            "book fold to a Book",
            per_call(200, ROUNDS, || {
                std::hint::black_box(parse_book(&book));
            })
        );
        eprintln!(
            "{:<34} {:>9.0}ns  parse_account",
            "account fold to an Account",
            per_call(2_000, ROUNDS, || {
                std::hint::black_box(parse_account(&held));
            })
        );
    }

    /// Candle history, on the wire, against the venue this file once recorded
    /// as serving none. The length is the whole point: the reported bug is a
    /// chart holding one bar, so a page that is not a page is the failure.
    #[test]
    #[ignore = "hits the live venue, run explicitly: candle history is served, at /candles"]
    fn a_chart_opened_on_this_venue_fills_with_history() {
        crate::hyperliquid::open_the_wire();
        smol::block_on(async {
            let now = now_ms() / 1_000;
            let tape = tape_focus(tape_new(), "BTC".to_owned(), "1h".to_owned());
            let filled = lighter_candles(
                Zone::Mainnet,
                tape.clone(),
                "BTC".to_owned(),
                "1h".to_owned(),
            )
            .await
            .expect("candles");
            assert_eq!(filled, CANDLE_PAGE, "a chart opens on a full page");

            let bars = lock(&tape.candles).clone();
            assert_eq!(bars.len() as i64, CANDLE_PAGE);
            for pair in bars.windows(2) {
                assert_eq!(
                    pair[1].ts - pair[0].ts,
                    3_600,
                    "oldest first, an hour apart"
                );
            }
            for bar in &bars {
                assert!(bar.low > 0.0 && bar.low <= bar.high);
                assert!(bar.low <= bar.open && bar.open <= bar.high);
                assert!(bar.low <= bar.close && bar.close <= bar.high);
            }
            assert!(
                now - bars.last().expect("a bar").ts < 7_200,
                "the newest bar is the live one"
            );
            // 500 hours is three weeks, so this is history rather than a
            // window folded out of the public tape.
            assert!(now - bars[0].ts > 495 * 3_600);

            // A loaded tape refreshes rather than paging itself in again, and
            // the length it answers is the length it still holds.
            let oldest = bars[0].ts;
            let again = lighter_candles(
                Zone::Mainnet,
                tape.clone(),
                "BTC".to_owned(),
                "1h".to_owned(),
            )
            .await
            .expect("refresh");
            assert_eq!(again, CANDLE_PAGE, "a refresh adds at most the live bar");
            assert_eq!(lock(&tape.candles)[0].ts, oldest, "nothing was dropped");

            // And panning past the oldest bar pages further back. What comes
            // back is the count of bars older than the one the tape began at,
            // which is the figure that tells the chart there was more to page
            // to — zero is the venue saying there is not.
            let older = lighter_history(
                Zone::Mainnet,
                tape.clone(),
                "BTC".to_owned(),
                "1h".to_owned(),
            )
            .await
            .expect("history");
            assert!(older > 0, "a page back added no older bars");
            assert_eq!(lock(&tape.candles).len() as i64, CANDLE_PAGE + older);
            assert!(lock(&tape.candles)[0].ts < oldest);

            // A width the venue does not quote never reaches it.
            let Err(refused) =
                lighter_candles(Zone::Mainnet, tape, "BTC".to_owned(), "3h".to_owned()).await
            else {
                panic!("3h is not one of the venue's resolutions");
            };
            assert!(refused.message.contains("3h"), "{}", refused.message);
        });
    }

    /// The venue answers some failures with a body rather than a status, and
    /// an unread `code` would turn one into an empty market list.
    #[test]
    fn an_empty_response_parses_to_an_empty_account_rather_than_a_wrong_one() {
        let parsed = parse_account(&json!({ "code": 200, "accounts": [] }));
        assert!(parsed.positions.is_empty());
        assert_eq!(parsed.value, 0.0);
        assert_eq!(parsed.margin_pct, 0.0);
        assert!(parse_symbols(&json!({ "code": 200 }), &json!({})).is_empty());
    }

    // -----------------------------------------------------------------------
    // The live feed.

    fn rung(price: &str, size: &str) -> Value {
        json!({ "price": price, "size": size })
    }

    /// The shape a subscription is answered with: the whole book, and the
    /// nonce every delta after it will be measured against. Prices are the
    /// live ones from a `subscribed/order_book` on market 1.
    fn book_snapshot(nonce: i64) -> Value {
        json!({
            "channel": "order_book:1",
            "type": "subscribed/order_book",
            "order_book": {
                "code": 0,
                "asks": [rung("64941.8", "0.02772"), rung("64944.3", "1.23355")],
                "bids": [rung("64939.5", "0.70469"), rung("64931.6", "0.08625")],
                "nonce": nonce, "begin_nonce": 0, "offset": 9_865_330_i64
            }
        })
    }

    fn book_delta(begin: i64, nonce: i64, bids: Value, asks: Value) -> Value {
        json!({
            "channel": "order_book:1",
            "type": "update/order_book",
            "order_book": {
                "code": 0, "asks": asks, "bids": bids,
                "nonce": nonce, "begin_nonce": begin, "offset": 9_865_331_i64
            }
        })
    }

    /// The nonces are the venue's own: an update states the nonce it expects
    /// the reader to be holding, and carries the one it leaves behind.
    const FIRST: i64 = 18_744_130_787;
    const SECOND: i64 = 18_744_130_812;
    const THIRD: i64 = 18_744_130_849;

    fn seeded() -> Depth {
        let (depth, fold) = fold_book(Depth::default(), &book_snapshot(FIRST));
        assert_eq!(fold, Fold::Applied);
        depth
    }

    fn sizes(side: &BTreeMap<u64, f64>) -> Vec<(f64, f64)> {
        side.iter()
            .map(|(price, size)| (f64::from_bits(*price), *size))
            .collect()
    }

    #[test]
    fn a_snapshot_replaces_the_levels_and_sets_the_nonce() {
        let depth = seeded();
        assert_eq!(depth.nonce, FIRST);
        assert_eq!(
            sizes(&depth.bids),
            [(64_931.6, 0.08625), (64_939.5, 0.70469)]
        );
        assert_eq!(
            sizes(&depth.asks),
            [(64_941.8, 0.02772), (64_944.3, 1.23355)]
        );

        // And it replaces rather than merges: a second snapshot is the whole
        // book, so a level the first one had and the second does not is gone.
        let (again, fold) = fold_book(
            depth,
            &json!({
                "channel": "order_book:1",
                "type": "subscribed/order_book",
                "order_book": {
                    "asks": [rung("64941.8", "0.02772")],
                    "bids": [rung("64939.5", "0.70469")],
                    "nonce": SECOND, "begin_nonce": 0
                }
            }),
        );
        assert_eq!(fold, Fold::Applied);
        assert_eq!(again.nonce, SECOND);
        assert_eq!(sizes(&again.bids), [(64_939.5, 0.70469)]);
    }

    #[test]
    fn a_delta_inserts_a_level_the_book_did_not_have() {
        let (depth, fold) = fold_book(
            seeded(),
            &book_delta(
                FIRST,
                SECOND,
                json!([rung("64936.5", "0.01608")]),
                json!([]),
            ),
        );
        assert_eq!(fold, Fold::Applied);
        assert_eq!(depth.nonce, SECOND);
        // In price order, between the two the snapshot held.
        assert_eq!(
            sizes(&depth.bids),
            [
                (64_931.6, 0.08625),
                (64_936.5, 0.01608),
                (64_939.5, 0.70469)
            ]
        );
    }

    #[test]
    fn a_delta_restates_a_level_rather_than_adding_to_it() {
        let (depth, fold) = fold_book(
            seeded(),
            &book_delta(
                FIRST,
                SECOND,
                json!([rung("64931.6", "0.08317")]),
                json!([]),
            ),
        );
        assert_eq!(fold, Fold::Applied);
        // What the level now holds, not what it held plus what arrived: the
        // sum would be 0.16942 and the size behind the best bid would grow
        // every time a maker repriced.
        assert_eq!(
            sizes(&depth.bids),
            [(64_931.6, 0.08317), (64_939.5, 0.70469)]
        );
    }

    #[test]
    fn a_size_of_zero_removes_the_level() {
        let (depth, fold) = fold_book(
            seeded(),
            &book_delta(
                FIRST,
                SECOND,
                json!([rung("64939.5", "0.00000")]),
                json!([rung("64941.8", "0.00000")]),
            ),
        );
        assert_eq!(fold, Fold::Applied);
        // The best bid and the best ask both left, so the top of the book is
        // now the pair behind them.
        assert_eq!(sizes(&depth.bids), [(64_931.6, 0.08625)]);
        assert_eq!(sizes(&depth.asks), [(64_944.3, 1.23355)]);
        // A level that is gone is gone rather than resting at nothing: a zero
        // kept would be drawn as depth and would sit in front of a real bid.
        assert!(!depth.bids.contains_key(&64_939.5_f64.to_bits()));
    }

    #[test]
    fn a_message_the_book_already_has_is_not_applied_twice() {
        let held = seeded();
        let first = book_delta(
            FIRST,
            SECOND,
            json!([rung("64931.6", "0.08317")]),
            json!([]),
        );
        let (moved, fold) = fold_book(held, &first);
        assert_eq!(fold, Fold::Applied);

        // The same message again — a replay, or the tail of a resubscription
        // overlapping what is held.
        let (after, fold) = fold_book(moved.clone(), &first);
        assert_eq!(fold, Fold::Stale);
        assert_eq!(after, moved, "a replay must not move the book");

        // And one older still: it ends before what is held began.
        let (after, fold) = fold_book(
            moved.clone(),
            &book_delta(
                FIRST - 20,
                FIRST,
                json!([rung("64931.6", "0.99999")]),
                json!([]),
            ),
        );
        assert_eq!(fold, Fold::Stale);
        assert_eq!(after, moved, "an old size must not land on a newer one");
    }

    #[test]
    fn a_message_that_never_arrived_is_a_gap_rather_than_a_drift() {
        let held = seeded();
        // Begins where the reader is not: whatever the message in between
        // said about levels nobody mentions again is unrecoverable.
        let (after, fold) = fold_book(
            held.clone(),
            &book_delta(
                SECOND,
                THIRD,
                json!([rung("64939.5", "0.00000")]),
                json!([]),
            ),
        );
        assert_eq!(fold, Fold::Gap);
        assert_eq!(
            after, held,
            "a gap must leave the book alone rather than half-applying"
        );

        // The next one in sequence still applies, which is what makes the gap
        // a statement about the message rather than about the connection.
        let (after, fold) = fold_book(
            held,
            &book_delta(
                FIRST,
                SECOND,
                json!([rung("64939.5", "0.00000")]),
                json!([]),
            ),
        );
        assert_eq!(fold, Fold::Applied);
        assert_eq!(after.nonce, SECOND);
    }

    #[test]
    fn a_delta_before_any_snapshot_is_a_gap() {
        // Nothing to apply it to. Reading it as a book would draw two levels
        // and call them the market.
        let (after, fold) = fold_book(
            Depth::default(),
            &book_delta(0, FIRST, json!([rung("64939.5", "0.70469")]), json!([])),
        );
        assert_eq!(fold, Fold::Gap);
        assert_eq!(after, Depth::default());
    }

    #[test]
    fn the_top_of_the_book_reads_nearest_price_first_on_both_sides() {
        let mut depth = Depth::default();
        // Twelve levels a side, so the panel's ten are a choice rather than
        // everything there was.
        for step in 0..12 {
            let bid = 64_939.5 - f64::from(step);
            let ask = 64_941.8 + f64::from(step);
            depth.bids.insert(bid.to_bits(), 1.0 + f64::from(step));
            depth.asks.insert(ask.to_bits(), 2.0 + f64::from(step));
        }
        let book = depth.book();
        assert_eq!(book.bids.len(), BOOK_DEPTH);
        assert_eq!(book.asks.len(), BOOK_DEPTH);
        assert_eq!(book.bids[0].price, 64_939.5, "the best bid is the highest");
        assert_eq!(book.asks[0].price, 64_941.8, "the best ask is the lowest");
        assert!(
            book.bids
                .windows(2)
                .all(|pair| pair[0].price > pair[1].price)
        );
        assert!(
            book.asks
                .windows(2)
                .all(|pair| pair[0].price < pair[1].price)
        );
        // Depth accumulates away from the touch, and the bar is each level's
        // share of the deepest shown.
        assert_eq!(book.bids[0].total, book.bids[0].size);
        assert_eq!(book.bids[1].total, book.bids[0].size + book.bids[1].size);
        assert_eq!(book.bids[BOOK_DEPTH - 1].bar, BOOK_BAR_WIDTH);
        assert!(book.bids[0].bar < book.bids[1].bar);
        assert_eq!(book.spread, 64_941.8 - 64_939.5);
        assert_eq!(book.mid, (64_941.8 + 64_939.5) / 2.0);
    }

    /// The streamed book and the REST book are read by the same panel, so a
    /// venue switch that changed which one filled it must not change what it
    /// draws.
    #[test]
    fn a_folded_book_reads_the_same_as_one_read_over_rest() {
        let folded = seeded().book();
        let over_rest = parse_book(&json!({
            "bids": [
                { "remaining_base_amount": "0.70469", "price": "64939.5" },
                { "remaining_base_amount": "0.08625", "price": "64931.6" }
            ],
            "asks": [
                { "remaining_base_amount": "0.02772", "price": "64941.8" },
                { "remaining_base_amount": "1.23355", "price": "64944.3" }
            ]
        }));
        assert_eq!(folded.bids, over_rest.bids);
        assert_eq!(folded.asks, over_rest.asks);
        assert_eq!(folded.mid, over_rest.mid);
        assert_eq!(folded.spread_pct, over_rest.spread_pct);
    }

    #[test]
    fn one_aggressor_across_several_resting_orders_is_one_print() {
        let tape = parse_prints(&captured_prints());
        assert_eq!(tape.len(), 2, "four messages, two orders crossed");
        // Oldest first, which is the order `push_trades` reverses onto the
        // panel: the sweep happened before the single print that followed it.
        assert_eq!(tape[0].sweep, 3);
        assert_eq!(tape[1].sweep, 1);
        assert!(tape[0].ts <= tape[1].ts);

        // What that order paid on average, weighted by what it took at each
        // level — not the last price it reached.
        let (sizes, notional) = [
            (0.00068, 64_940.8),
            (0.00148, 64_940.9),
            (0.00228, 64_941.0),
        ]
        .into_iter()
        .fold((0.0, 0.0), |(sizes, notional), (size, price)| {
            (sizes + size, notional + size * price)
        });
        assert!((tape[0].size - sizes).abs() < 1e-12);
        assert!((tape[0].price - notional / sizes).abs() < 1e-9);
        assert!(
            tape[0].price < 64_941.0,
            "the average is under the last fill"
        );
    }

    #[test]
    fn the_side_that_crossed_is_the_one_that_was_not_resting() {
        let tape = parse_prints(&captured_prints());
        // The maker was the ask, so the aggressor bought.
        assert!(tape[0].buy);
        // And the other way round on the print that followed.
        assert!(!tape[1].buy);
        // Seconds, because that is what the tape holds; the venue stamps
        // milliseconds.
        assert_eq!(tape[1].ts, 1_786_189_598_959 / 1_000);
    }

    /// A live `subscribed/market_stats` object for market 1, whole.
    fn stats() -> Value {
        json!({
            "base_interest_rate": "0.0100",
            "best_ask_price": "64940.9", "best_bid_price": "64940.6",
            "current_funding_rate": "0.0005",
            "daily_base_token_volume": 8039.38822,
            "daily_price_change": -0.4717357053966074,
            "daily_price_high": 65349.4, "daily_price_low": 64489.1,
            "daily_quote_token_volume": 522580275.211415,
            "funding_rate": "0.0005", "funding_timestamp": 1_786_186_800_002_i64,
            "index_price": "64969.9", "last_trade_price": "64940.9",
            "mark_price": "64939.2", "market_id": 1, "mid_price": "64940.8",
            "open_interest": "119270958.327360", "premium": "-0.0456", "symbol": "BTC"
        })
    }

    /// The stream and `/funding-rates` state the same rate in different units,
    /// and the row can only carry one of them. The fixture's REST rate is the
    /// eight-hour fraction this hourly percentage would have been published
    /// as, so the two readings have to land on the same number.
    #[test]
    fn the_streamed_funding_rate_is_already_the_row_s_own_unit() {
        let streamed = parse_stats(&stats());
        let hourly_pct = 0.0005;
        let over_rest = parse_symbols(
            &captured_universe(),
            &json!({ "funding_rates": [
                { "market_id": 1, "exchange": "lighter", "symbol": "BTC",
                  "rate": hourly_pct / 100.0 * FUNDING_HOURS }
            ]}),
        );
        let btc = over_rest.iter().find(|row| row.name == "BTC").expect("BTC");
        assert!((streamed.funding_pct - btc.funding_pct).abs() < 1e-15);
        assert!((streamed.funding_pct - hourly_pct).abs() < 1e-15);
    }

    #[test]
    fn the_streamed_open_interest_is_turned_back_into_coins() {
        let streamed = parse_stats(&stats());
        // The venue's own two figures: dollars of open interest over the mark
        // they were valued at.
        assert_eq!(streamed.open_interest, 119_270_958.327_360 / 64_939.2);
        // And that is the quantity the universe publishes for the same
        // market, which the dollars themselves are nowhere near.
        let universe = parse_symbols(&captured_universe(), &captured_rates());
        let btc = universe.iter().find(|row| row.name == "BTC").expect("BTC");
        assert!(
            (streamed.open_interest - btc.open_interest).abs() < btc.open_interest * 0.01,
            "{} coins against the universe's {}",
            streamed.open_interest,
            btc.open_interest
        );
        // A market with no mark to divide by states no open interest rather
        // than a number of dollars pretending to be coins.
        assert_eq!(open_interest(119_270_958.0, 0.0), 0.0);
    }

    #[test]
    fn the_streamed_context_leaves_the_assets_own_figures_alone() {
        let streamed = parse_stats(&stats());
        assert_eq!(streamed.name, "BTC");
        assert_eq!(streamed.price, 64_939.2, "the mark, as the universe read");
        assert_eq!(streamed.volume, 522_580_275.211_415);
        // The day says nothing about the margin engine or the size step, so
        // these read zero and `apply_feed` keeps what the universe said.
        assert_eq!(streamed.leverage, 0.0);
        assert_eq!(streamed.maintenance, 0.0);
        assert_eq!(streamed.size_decimals, 0);
        // Yesterday's close, backed out of the move, lands inside the day.
        assert!((64_489.1..=65_349.4).contains(&streamed.prev));
    }

    /// A market by id alone, for the feed tests: what a channel name is built
    /// from is the id, and the two step counts belong to the order path.
    fn market_at(id: i64) -> Market {
        Market {
            id,
            price_decimals: 0,
            size_decimals: 0,
        }
    }

    /// A ticker only this test uses, so seeding the shared id table cannot
    /// disturb another one.
    fn charted(coin: &str, id: i64, interval: &str) -> Tape {
        lock(ids()).insert(coin.to_owned(), market_at(id));
        tape_focus(tape_new(), coin.to_owned(), interval.to_owned())
    }

    #[test]
    fn the_channels_are_the_focused_market_s_and_the_stats_of_everything() {
        // Nothing on screen yet: the market list still wants its prices.
        let idle = tape_new();
        assert_eq!(channels(&idle).expect("channels"), [ALL_STATS]);

        let tape = charted("ICEONE", 4_242, "15m");
        let wanted = channels(&tape).expect("channels");
        assert_eq!(
            wanted,
            [
                ALL_STATS,
                "order_book/4242",
                "trade/4242",
                "candle/4242/15m"
            ]
        );

        // The reader listens on the same channels, spelled the way the venue
        // echoes them back.
        let listening = focused(&tape);
        assert_eq!(listening.coin, "ICEONE");
        assert_eq!(listening.book, wanted[1].replace('/', ":"));
        assert_eq!(listening.prints, wanted[2].replace('/', ":"));
        assert_eq!(listening.bars, wanted[3].replace('/', ":"));
    }

    #[test]
    fn a_market_with_no_id_yet_asks_for_nothing_per_market() {
        // The universe read fills the ticker table; until it lands there is no
        // channel to name, and asking for `order_book/` would be refused.
        let tape = tape_focus(tape_new(), "ICEUNLISTED".to_owned(), "1m".to_owned());
        assert_eq!(channels(&tape).expect("channels"), [ALL_STATS]);
        assert_eq!(focused(&tape).book, "");
    }

    #[test]
    fn an_interval_the_venue_does_not_quote_is_refused_rather_than_charted() {
        assert_eq!(resolution("4h"), Some(("4h", 14_400)));
        // The venue answers `candle/1/2h` with `Invalid Channel: (invalid
        // resolution)` and does not say which channel it refused, so the
        // chart would simply stay empty with nothing said about why.
        assert_eq!(resolution("2h"), None);
        assert_eq!(resolution("1w"), None);

        let tape = charted("ICETWO", 4_243, "2h");
        let Err(refused) = channels(&tape) else {
            panic!("a 2h chart must not be drawn from a venue with no 2h candle");
        };
        assert!(
            refused.message.contains("2h") && refused.message.contains("12h"),
            "the refusal should name the interval and the ones on offer: {}",
            refused.message
        );
        // And the refusal does not depend on the market: it is the width the
        // venue does not quote, not the ticker.
        let unlisted = tape_focus(tape_new(), "ICEUNLISTED".to_owned(), "2h".to_owned());
        assert!(channels(&unlisted).is_err());
    }

    /// Every tab the chart draws has to be a width the venue quotes — and the
    /// width in seconds has to be that width, because the history read states
    /// its window in seconds and a wrong one asks for the wrong span of time.
    #[test]
    fn every_interval_the_app_offers_is_one_the_venue_quotes() {
        // The chart's own tabs, which is the vocabulary this map exists to
        // hold the venue against, each beside the seconds it is worth.
        for (interval, secs) in [
            ("1m", 60),
            ("5m", 300),
            ("15m", 900),
            ("1h", 3_600),
            ("4h", 14_400),
            ("1d", 86_400),
        ] {
            assert_eq!(resolution(interval), Some((interval, secs)), "{interval}");
        }
    }

    /// A chart opened on this venue asks for a window, and the request is the
    /// whole of why: the window and `count_back` are both required and the
    /// answer is the wider of the two, so a request that named only one of
    /// them would come back a length nobody chose.
    ///
    /// The reported bug is the answer being one bar, and the two ways to get
    /// there are both visible here — a `count_back` of 1, and a window that
    /// spans one width or none at all — so this is the arithmetic that decides
    /// it rather than a round trip.
    #[test]
    fn a_chart_opens_by_asking_for_a_full_page_of_bars() {
        // Bitcoin's hourly bars, ending on a round timestamp so the window is
        // readable: 500 hours before 1786242000 is 1784442000.
        let end_ms = 1_786_242_000_000;
        let path = candles_path(1, "1h", 3_600, end_ms, CANDLE_PAGE);
        assert_eq!(
            path,
            "candles?market_id=1&resolution=1h&start_timestamp=1784442000000\
             &end_timestamp=1786242000000&count_back=500"
        );
        // Both halves say 500, which is what makes the answer 500 rather than
        // whichever of the two happened to be wider.
        assert!(path.contains("count_back=500"));
        assert_eq!((end_ms - 1_784_442_000_000) / 1_000 / 3_600, CANDLE_PAGE);

        // A refresh is the same request over a shorter span, so the live bar
        // is re-read without paging the history in again.
        let refresh = candles_path(1, "1h", 3_600, end_ms, CANDLE_REFRESH);
        assert!(refresh.contains("count_back=3"));
        assert!(refresh.contains("start_timestamp=1786231200000"));

        // The width is the span's unit as well as the venue's parameter: a
        // day of bars covers a day per bar.
        let daily = candles_path(1, "1d", 86_400, end_ms, CANDLE_PAGE);
        assert!(daily.contains("resolution=1d"));
        assert!(
            daily.contains("start_timestamp=1743042000000"),
            "500 days back, not 500 hours: {daily}"
        );
    }

    /// The history read and the feed hand the chart the same bar. The venue
    /// sends milliseconds and two volume legs, and the chart holds seconds and
    /// the base one — so a bar read either way has to arrive converted and on
    /// the leg the other venue's chart is drawn in.
    #[test]
    fn a_bar_read_from_history_is_the_bar_the_feed_forms() {
        // Trimmed from a live `GET /candles?market_id=1&resolution=1h`, with
        // the keys exactly as the venue sends them — one letter each, and `V`
        // beside `v` for the leg that is not the chart's.
        let page = json!({
            "code": 200,
            "r": "1h",
            "c": [
                { "t": 1786190400000_i64, "o": 65086.5, "h": 65127.7, "l": 65040.2,
                  "c": 65098.4, "v": 141.03674, "V": 9180008.926911 },
                { "t": 1786194000000_i64, "o": 65098.4, "h": 65160.1, "l": 65071.9,
                  "c": 65133.0, "v": 98.41255, "V": 6408184.271336 },
                { "t": 1786197600000_i64, "o": 65133.0, "h": 65141.2, "l": 65098.5,
                  "c": 65125.8, "V": 0.0 }
            ]
        });
        let bars: Vec<Candle> = list(&page, "c").iter().map(parse_candle).collect();
        assert_eq!(bars.len(), 3, "a page is bars, not a bar");
        assert_eq!(bars[0].ts, 1_786_190_400, "seconds, like the chart");
        // An hour apart, which is the resolution that was asked for.
        assert_eq!(bars[1].ts - bars[0].ts, 3_600);
        assert_eq!(bars[0].open, 65_086.5);
        assert_eq!(bars[0].close, 65_098.4);
        assert!(bars[0].low <= bars[0].open && bars[0].open <= bars[0].high);
        // The base leg, not the quote leg beside it — which is four orders of
        // magnitude away, so reading the wrong key is not a rounding error.
        assert_eq!(bars[0].volume, 141.03674);
        assert!(
            bars[0].volume < bars[0].close,
            "the quote leg is not volume"
        );
        // A bar that traded nothing arrives without its `v` at all.
        assert_eq!(bars[2].volume, 0.0);
        // A close carries into the next bar's open, which is what makes a tape
        // out of a list.
        assert_eq!(bars[0].close, bars[1].open);

        // And the feed's spelling of the same bar reads back identically, so
        // history and the forming bar cannot disagree about a bar's units.
        let formed = parse_candles(&json!({
            "channel": "candle:1:1h",
            "candles": list(&page, "c"),
        }));
        assert_eq!(formed, bars);
    }

    #[test]
    fn candles_arrive_as_floats_and_land_on_the_tape_in_seconds() {
        let bars = parse_candles(&json!({
            "channel": "candle:1:1m", "type": "update/candle",
            "candles": [{
                "V": 29652.644232000035, "c": 64940.9, "h": 64944.2,
                "i": 27_074_544_215_i64, "l": 64935.4, "o": 64936.5,
                "t": 1_786_189_440_000_i64, "v": 0.4565999999999997
            }]
        }));
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].ts, 1_786_189_440_000 / 1_000);
        assert_eq!(bars[0].open, 64_936.5);
        assert_eq!(bars[0].close, 64_940.9);
        assert_eq!(bars[0].high, 64_944.2);
        assert_eq!(bars[0].low, 64_935.4);
        // The base leg. `V` beside it is the same volume in dollars, and it is
        // four orders of magnitude away.
        assert_eq!(bars[0].volume, 0.4565999999999997);
    }

    /// Walks the reader the way a connection would, which is the only way to
    /// reach these arms without the exchange.
    fn reader(
        tape: &Tape,
    ) -> (
        impl FnMut(Event<'_>) -> Option<MarketTick>,
        Arc<Mutex<Vec<String>>>,
    ) {
        let refresh: Arc<Mutex<Vec<String>>> = Arc::default();
        (market_reader(tape.clone(), refresh.clone()), refresh)
    }

    #[test]
    fn a_beat_carries_the_book_and_nothing_is_sent_when_nothing_moved() {
        let tape = charted("ICETHREE", 4_244, "1m");
        let (mut read, _) = reader(&tape);
        // The first beat adopts the market on screen.
        assert!(read(Event::Beat).is_none());

        assert!(read(Event::Payload("order_book:4244", &book_snapshot(FIRST))).is_none());
        let tick = read(Event::Beat).expect("a book is worth a beat");
        let book = tick.book.expect("the book");
        // Reversed, because the panel walks the asks from the top of the
        // screen down: the best ask is the last row.
        assert_eq!(book.asks.last().expect("an ask").price, 64_941.8);
        assert_eq!(book.bids[0].price, 64_939.5);

        // Nothing arrived since, so there is nothing to redraw.
        assert!(read(Event::Beat).is_none());
    }

    #[test]
    fn a_gap_drops_the_book_and_asks_for_a_fresh_snapshot() {
        let tape = charted("ICEFOUR", 4_245, "1m");
        let (mut read, refresh) = reader(&tape);
        assert!(read(Event::Beat).is_none());
        assert!(read(Event::Payload("order_book:4245", &book_snapshot(FIRST))).is_none());
        assert!(read(Event::Beat).expect("a book").book.is_some());

        // A message that begins after the one held: whatever was in between
        // is unrecoverable.
        let missed = book_delta(
            SECOND,
            THIRD,
            json!([rung("64939.5", "0.00000")]),
            json!([]),
        );
        assert!(read(Event::Payload("order_book:4245", &missed)).is_none());
        let tick = read(Event::Beat).expect("the dropped book is a change");
        assert!(
            tick.book.is_none(),
            "a book that missed a delta must not go on being drawn"
        );
        assert_eq!(
            *lock(&refresh),
            ["order_book/4245"],
            "the socket is asked for the snapshot, in the spelling it takes"
        );

        // And the fresh snapshot puts it back.
        assert!(read(Event::Payload("order_book:4245", &book_snapshot(THIRD))).is_none());
        assert!(read(Event::Beat).expect("the book is back").book.is_some());
    }

    #[test]
    fn the_book_of_the_market_just_left_is_not_drawn_as_this_one_s() {
        let tape = charted("ICEFIVE", 4_246, "1m");
        lock(ids()).insert("ICESIX".to_owned(), market_at(4_247));
        let (mut read, _) = reader(&tape);
        assert!(read(Event::Beat).is_none());
        assert!(read(Event::Payload("order_book:4246", &book_snapshot(FIRST))).is_none());
        assert!(read(Event::Beat).expect("a book").book.is_some());

        // The app moves. The socket keeps serving the old subscription until
        // the unsubscribe takes effect.
        tape_focus(tape.clone(), "ICESIX".to_owned(), "1m".to_owned());
        assert!(
            read(Event::Beat).is_none(),
            "the switch itself is not a tick"
        );
        assert!(read(Event::Payload("order_book:4246", &book_snapshot(SECOND))).is_none());

        // Something the whole exchange shares moves, so the next beat is sent
        // — and it must not carry the book of the market that was left. The
        // one the app is looking at has no book yet, and no book is the honest
        // answer to that.
        assert!(read(Event::Payload(ALL_STATS_ECHO, &all_stats())).is_none());
        let tick = read(Event::Beat).expect("the prices moved");
        assert!(
            tick.book.is_none(),
            "the market just left is still on screen"
        );
    }

    /// A row as the universe read it, before any beat has touched it.
    fn listed(name: &str, size_decimals: usize) -> SymbolRow {
        SymbolRow {
            name: name.to_owned(),
            price: 0.0,
            change_pct: 0.0,
            volume: 0.0,
            funding_pct: 0.0,
            leverage: 0.0,
            open_interest: 0.0,
            prev: 0.0,
            maintenance: 0.0,
            size_decimals,
            selected: false,
            ..Default::default()
        }
    }

    /// `market_stats/all` as it arrives: keyed by market id, two live markets.
    fn all_stats() -> Value {
        json!({
            "channel": ALL_STATS_ECHO, "type": "update/market_stats",
            "market_stats": {
                "0": {
                    "symbol": "ETH", "market_id": 0, "mark_price": "1918.51",
                    "index_price": "1919.57", "mid_price": "1918.52",
                    "daily_price_change": -0.6529618889809445,
                    "daily_quote_token_volume": 152445945.005175,
                    "current_funding_rate": "0.0009",
                    "open_interest": "82013802.489619"
                },
                "1": stats()
            }
        })
    }

    #[test]
    fn the_stats_of_every_market_become_mids_and_one_context() {
        // Charted on the market the stats object does not end with, so a
        // context taken off the wrong row is the wrong row rather than the
        // right one by coincidence.
        let tape = charted("ETH", 0, "1m");
        let (mut read, _) = reader(&tape);
        assert!(read(Event::Beat).is_none());
        assert!(read(Event::Payload(ALL_STATS_ECHO, &all_stats())).is_none());
        let tick = read(Event::Beat).expect("prices moved");
        assert_eq!(
            tick.context.as_ref().expect("a day").name,
            "ETH",
            "the day republished is the charted market's"
        );

        let rows = apply_feed(vec![listed("BTC", 5), listed("ETH", 4)], tick);
        // Every market's price, so the whole list re-marks.
        assert_eq!(rows[0].price, 64_939.2);
        assert_eq!(rows[1].price, 1_918.51);
        // The charted market's day lands whole, and the size step the universe
        // read survives it rather than being zeroed by the context.
        assert_eq!(rows[1].volume, 152_445_945.005_175);
        assert_eq!(rows[1].size_decimals, 4);
        // The rest are re-priced and nothing more.
        assert_eq!(
            rows[0].volume, 0.0,
            "the day of a market nobody charted is not republished"
        );
    }

    #[test]
    fn a_beat_with_no_market_still_carries_the_prices() {
        // The market list before anything is charted: no book, no candles,
        // and a list that still has to tick.
        let tape = tape_new();
        let (mut read, _) = reader(&tape);
        assert!(read(Event::Beat).is_none());
        assert!(read(Event::Payload(ALL_STATS_ECHO, &all_stats())).is_none());
        let tick = read(Event::Beat).expect("prices moved");
        assert!(tick.book.is_none());
        assert!(tick.context.is_none(), "no market, no market's day");
        let rows = apply_feed(vec![listed("BTC", 5), listed("ETH", 4)], tick);
        assert_eq!(rows[0].price, 64_939.2);
        assert_eq!(rows[1].price, 1_918.51);
    }

    /// The round trip the header shows. It is the only honest latency reading
    /// available — a round trip needs no agreement between our clock and the
    /// venue's — and an unchanged one is not worth a redraw.
    #[test]
    fn a_pong_moves_the_latency_and_only_when_it_changed() {
        let tape = tape_new();
        let (mut read, _) = reader(&tape);
        assert!(read(Event::Beat).is_none());
        assert!(read(Event::Pong(31)).is_none());
        assert_eq!(read(Event::Beat).expect("latency moved").latency, 31);
        assert!(read(Event::Pong(31)).is_none());
        assert!(read(Event::Beat).is_none(), "the same reading is no news");
    }

    /// Everything the captured payloads cannot check: that the channel names
    /// are still the venue's, that a delta stream folded over a live minute
    /// stays a book, and that the tape fills. Kept out of the default run
    /// because it fails on a train rather than on a bug.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the feed against the exchange"]
    fn the_live_feed_folds_a_book_a_tape_and_a_context() {
        crate::hyperliquid::open_the_wire();
        // Fills the ticker-to-id table, which is what names the channels.
        let universe = smol::block_on(lighter_symbols(Zone::Mainnet)).expect("markets");
        assert!(universe.iter().any(|row| row.name == "BTC"));

        let tape = tape_focus(tape_new(), "BTC".to_owned(), "1m".to_owned());
        let feed = lighter_market_feed(Zone::Mainnet, tape.clone());
        let deadline = Instant::now() + Duration::from_secs(60);
        let (mut books, mut prints, mut context, mut latency) = (0, 0, 0, 0);

        smol::block_on(async {
            while Instant::now() < deadline {
                let tick = match feed.recv().await {
                    Ok(Ok(tick)) => tick,
                    Ok(Err(error)) => panic!("the feed reported {}", error.message),
                    Err(_) => break,
                };
                assert!(!tick.mids.is_empty() || tick.book.is_some() || tick.latency > 0);
                if let Some(price) = tick.mids.get("BTC") {
                    assert!(*price > 0.0, "BTC at {price}");
                }
                if tick.latency > 0 {
                    latency = tick.latency;
                }
                if let Some(row) = &tick.context {
                    assert_eq!(row.name, "BTC");
                    assert!(row.price > 0.0 && row.volume > 0.0);
                    // Coins rather than dollars: bitcoin's open interest is a
                    // few thousand coins and a few hundred million dollars.
                    assert!(row.open_interest < 1e6, "{} coins", row.open_interest);
                    context += 1;
                }
                prints += tick.trades.len();
                let Some(book) = &tick.book else {
                    continue;
                };
                books += 1;
                // The whole claim the delta fold makes, held against a live
                // minute of it: a book that stopped deleting levels, or
                // applied a message twice, crosses within seconds.
                let best_bid = book.bids.first().expect("a bid side").price;
                let best_ask = book.asks.last().expect("an ask side").price;
                assert!(
                    best_bid < best_ask,
                    "crossed book after {books} folds: {best_bid} bid against {best_ask} ask"
                );
                assert!(book.bids.iter().all(|level| level.size > 0.0));
                assert!(book.asks.iter().all(|level| level.size > 0.0));
                // Both sides descend down the screen: the bids from the touch
                // and the asks toward it, which is the reversal the panel
                // draws from.
                assert!(
                    book.bids
                        .windows(2)
                        .all(|pair| pair[0].price > pair[1].price)
                );
                assert!(
                    book.asks
                        .windows(2)
                        .all(|pair| pair[0].price > pair[1].price)
                );
                assert_eq!(book.bids.len(), BOOK_DEPTH, "bitcoin is deeper than ten");
                if books > 100 && prints > 0 && context > 0 && latency > 0 {
                    break;
                }
            }
        });

        assert!(books > 100, "only {books} books folded in a minute");
        assert!(prints > 0, "no prints crossed in a minute of bitcoin");
        assert!(context > 0, "the market's own day never arrived");
        assert!(latency > 0, "no ping round trip came back");

        // One candle per interval is all this venue sends, and it sends the
        // one now forming — so a minute of feed is one bar, merged in place.
        let candles = lock(&tape.candles);
        assert!(!candles.is_empty(), "the tape stayed empty");
        assert!(candles.windows(2).all(|pair| pair[0].ts < pair[1].ts));
        assert!(
            candles
                .iter()
                .all(|bar| bar.high >= bar.low && bar.close > 0.0)
        );
    }

    /// The other half of the socket: that it follows the app off one market
    /// and onto another, and that a channel it has dropped can be taken back.
    ///
    /// The last leg is the recovery a gap asks for — `resubscribe` sends the
    /// same two frames a switch back does — and it is the one thing about the
    /// gap path no captured payload can check, because the venue refuses a
    /// second subscribe to a channel it is already serving.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the feed following a market switch"]
    fn the_live_feed_follows_a_switch_and_takes_a_dropped_channel_back() {
        crate::hyperliquid::open_the_wire();
        let universe = smol::block_on(lighter_symbols(Zone::Mainnet)).expect("markets");
        let published = |coin: &str| {
            universe
                .iter()
                .find(|row| row.name == coin)
                .unwrap_or_else(|| panic!("{coin} is listed"))
                .price
        };

        let tape = tape_focus(tape_new(), "BTC".to_owned(), "1m".to_owned());
        let feed = lighter_market_feed(Zone::Mainnet, tape.clone());
        smol::block_on(async {
            // Bitcoin, then ether, then bitcoin again: the third leg subscribes
            // to a channel this socket has already unsubscribed from.
            for coin in ["BTC", "ETH", "BTC"] {
                tape_focus(tape.clone(), coin.to_owned(), "1m".to_owned());
                let want = published(coin);
                let deadline = Instant::now() + Duration::from_secs(30);
                let mid = loop {
                    assert!(Instant::now() < deadline, "no {coin} book in 30 seconds");
                    let tick = match feed.recv().await {
                        Ok(Ok(tick)) => tick,
                        Ok(Err(error)) => panic!("the feed reported {}", error.message),
                        Err(_) => panic!("the feed hung up"),
                    };
                    let Some(book) = tick.book else {
                        continue;
                    };
                    // Which market a book belongs to is only visible in what
                    // it is priced at, which is the whole reason the reader
                    // turns away the channels it did not ask for.
                    if (book.mid - want).abs() < want * 0.1 {
                        break book.mid;
                    }
                };
                assert!(mid > 0.0, "{coin} at {mid} against a published {want}");
            }
        });
    }
}
