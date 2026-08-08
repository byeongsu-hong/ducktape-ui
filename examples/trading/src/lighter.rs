//! Lighter read into the model `hyperliquid` defines, so a row from either
//! venue means the same thing to the panel that draws it. Lighter is a REST
//! API with query strings rather than one POST endpoint, and it mixes JSON
//! strings with JSON numbers in the same object — sometimes for the same
//! quantity — so every field goes through the same tolerant readers
//! `hyperliquid` uses rather than a derive.
//!
//! The three units that could each be read two ways were settled against live
//! responses rather than guessed; each one carries the identity that proved it.

// Read-only and complete, but nothing points at it until the venue switch
// lands.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Duration;

use serde_json::Value;

use crate::hyperliquid::{Account, Book, HlError, Level, Position, SymbolRow};

const BASE: &str = "https://mainnet.zklighter.elliot.ai/api/v1";
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

/// One GET off the UI thread, reporting whatever the venue said about a
/// failure. Two shapes of refusal were seen live and both are worth quoting:
/// a path the REST surface does not serve answers 403 with an empty body, so
/// the status is all there is, and a bad parameter answers 400 with a `code`
/// and a `message`. A body that parsed is still not a body that succeeded,
/// since the venue also carries its own `code` on a 200.
async fn get(path: String) -> Result<Value, HlError> {
    smol::unblock(move || {
        let url = format!("{BASE}/{path}");
        let mut response = agent()
            .get(&url)
            .call()
            .map_err(|error| fail(format!("Lighter unreachable: {error}")))?;
        let status = response.status();
        let body = response
            .body_mut()
            .read_json::<Value>()
            .map_err(|error| fail(format!("Lighter answered {path} with {status}: {error}")))?;
        let code = body.get("code").and_then(Value::as_i64).unwrap_or(200);
        if status.is_success() && code == 200 {
            Ok(body)
        } else {
            Err(fail(format!(
                "Lighter refused {path}: {status}, code {code} {}",
                text(&body, "message")
            )))
        }
    })
    .await
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
fn ids() -> &'static Mutex<HashMap<String, i64>> {
    static IDS: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    IDS.get_or_init(Mutex::default)
}

fn parse_ids(details: &Value) -> HashMap<String, i64> {
    list(details, "order_book_details")
        .iter()
        .map(|detail| (text(detail, "symbol"), value_i64(detail, "market_id")))
        .collect()
}

pub async fn lighter_symbols() -> Result<Vec<SymbolRow>, HlError> {
    let details = get("orderBookDetails".to_owned()).await?;
    // Free, since the universe is already in hand.
    *lock(ids()) = parse_ids(&details);
    let rates = get("funding-rates".to_owned()).await?;
    Ok(parse_symbols(&details, &rates))
}

/// The market id a ticker trades under, fetching the universe only if nothing
/// has yet.
async fn market_id(coin: &str) -> Result<i64, HlError> {
    if let Some(id) = lock(ids()).get(coin) {
        return Ok(*id);
    }
    let details = get("orderBookDetails".to_owned()).await?;
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
    let deepest = rows.last().map_or(0.0, |level| level.total);
    if deepest > 0.0 {
        for level in &mut rows {
            level.bar = level.total / deepest * BOOK_BAR_WIDTH;
        }
    }
    rows
}

fn parse_book(value: &Value) -> Book {
    let bids = parse_levels(list(value, "bids"));
    let asks = parse_levels(list(value, "asks"));
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

/// The book, keyed by ticker rather than by market id so it reads like the
/// other venue's. The asks come back best-first; the panel reverses them
/// itself, the same way it does for the feed it already has.
pub async fn lighter_book(coin: String) -> Result<Book, HlError> {
    let id = market_id(&coin).await?;
    Ok(parse_book(
        &get(format!(
            "orderBookOrders?market_id={id}&limit={ORDER_FETCH}"
        ))
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

/// The account behind an L1 address. Lighter also keys accounts by its own
/// index, but an address is what the app already asks the reader for and what
/// the other venue takes.
pub async fn lighter_account(address: String) -> Result<Account, HlError> {
    Ok(parse_account(
        &get(format!("account?by=l1_address&value={address}")).await?,
    ))
}

// Candles are not reachable. `GET /candlesticks` answers 403 from CloudFront
// for every parameter set tried, with and without browser `Origin`, `Referer`,
// and `User-Agent` headers, so the refusal is at the edge rather than in the
// API. Nothing else on the REST surface carries OHLC: `orderBookDetails` has
// a `daily_chart` key, but it is `{}` on all 222 markets, and the only other
// history endpoints are trades and funding.
//
// So there is no `lighter_candles`. A chart pointed at this venue has one
// route left — folding `recentTrades` into bars locally — which is a different
// thing from a candle the venue agrees with, and is not written here rather
// than written and quietly wrong.

// `recentTrades` parses cleanly and its side is settled: `is_maker_ask` true
// means the resting order was the sell, so the aggressor bought. Confirmed on
// consecutive live prints by watching `maker_position_size_before` move
// against the trade size — 43.08300 to 43.08280 on a 0.00020 print with
// `is_maker_ask` true is a maker who sold, and -0.14077 to -0.14071 on a
// 0.00006 print with it false is a maker who bought.
//
// The parser is still missing, because `hyperliquid::Trade` has a private
// `tid` field and a struct outside that module cannot name it. One `pub` there
// is the whole gap.

#[cfg(test)]
mod tests {

    /// The step a size is quoted to belongs to the market, and each venue
    /// publishes its own. Reading it off the wrong field, or defaulting it,
    /// would let the app work out a size the venue then refuses.
    #[test]
    fn a_market_carries_the_step_the_venue_quotes_it_at() {
        let rows = parse_symbols(&details(), &rates());
        let step = |name: &str| {
            rows.iter()
                .find(|row| row.name == name)
                .unwrap_or_else(|| panic!("{name} is not in the sample"))
                .size_decimals
        };
        // ASTER quotes to a tenth of a coin and MKR to a ten-thousandth,
        // which are the venue's own published steps for those two markets.
        assert_eq!(step("ASTER"), 1);
    }
    use super::*;
    use serde_json::json;

    /// Trimmed from a live `GET /orderBookDetails`: the busiest market, a
    /// mid-table one, and a delisted one, with the fields the parser reads.
    fn details() -> Value {
        json!({
            "code": 200,
            "order_book_details": [
                {
                    "symbol": "MKR", "market_id": 28, "market_type": "perp",
                    "status": "inactive", "size_decimals": 4,
                    "maintenance_margin_fraction": 0,
                    "min_initial_margin_fraction": 0,
                    "mark_price": "0.00", "index_price": "0.00",
                    "daily_price_change": 0, "daily_quote_token_volume": 0,
                    "daily_base_token_volume": 0, "open_interest": 0
                },
                {
                    "symbol": "ASTER", "market_id": 83, "market_type": "perp",
                    "status": "active", "size_decimals": 1,
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
                    "status": "active",
                    "maintenance_margin_fraction": 120,
                    "min_initial_margin_fraction": 200,
                    "default_initial_margin_fraction": 500,
                    "mark_price": "64975.3", "index_price": "64980.1",
                    "daily_price_change": 0.6118286879673691,
                    "daily_quote_token_volume": 618847551.336845,
                    "daily_base_token_volume": 9520.35,
                    "daily_price_low": 64270.5, "daily_price_high": 65349.4,
                    "open_interest": 1836.69392
                }
            ]
        })
    }

    /// Trimmed from a live `GET /funding-rates`, keeping one venue Lighter
    /// only republishes so the filter has something to reject.
    fn rates() -> Value {
        json!({
            "code": 200,
            "funding_rates": [
                { "market_id": 1, "exchange": "binance", "symbol": "BTC", "rate": 4.183e-05 },
                { "market_id": 1, "exchange": "hyperliquid", "symbol": "BTC", "rate": 3.20592e-05 },
                { "market_id": 1, "exchange": "lighter", "symbol": "BTC", "rate": 6.4e-05 },
                { "market_id": 83, "exchange": "lighter", "symbol": "ASTER", "rate": 0.0001 }
            ]
        })
    }

    #[test]
    fn markets_sort_by_volume_and_drop_the_delisted() {
        let rows = parse_symbols(&details(), &rates());
        assert_eq!(
            rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
            ["BTC", "ASTER"],
            "busiest first, and MKR is not tradeable"
        );
    }

    #[test]
    fn margin_fractions_read_as_basis_points() {
        let rows = parse_symbols(&details(), &rates());
        // 120 bp of a position's value is what the engine holds against it.
        assert!((rows[0].maintenance - 0.012).abs() < 1e-12);
        // 200 bp is the smallest initial margin, so 50x is the ceiling.
        assert!((rows[0].leverage - 50.0).abs() < 1e-12);
        assert!((rows[1].maintenance - 0.12).abs() < 1e-12);
        assert!((rows[1].leverage - 5.0).abs() < 1e-12);
    }

    /// The check that would catch the fraction/percent slip: a previous close
    /// outside the day's own range is the arithmetic saying it read the change
    /// in the wrong unit.
    #[test]
    fn yesterdays_close_lands_inside_the_days_range() {
        let rows = parse_symbols(&details(), &rates());
        assert!(
            (64_270.5..=65_349.4).contains(&rows[0].prev),
            "BTC previous close {} is outside 64270.5..65349.4",
            rows[0].prev
        );
        assert!((0.59635..=0.60212).contains(&rows[1].prev));
        // And it still reconstructs the mark it was backed out of.
        let round_trip = rows[0].prev * (1.0 + rows[0].change_pct / 100.0);
        assert!((round_trip - rows[0].price).abs() < 1e-6);
    }

    #[test]
    fn funding_is_the_hourly_share_of_the_published_eight_hour_rate() {
        let rows = parse_symbols(&details(), &rates());
        // 6.4e-05 over eight hours is 8.0e-06 an hour, which is 0.0008%.
        assert!((rows[0].funding_pct - 6.4e-05 / 8.0 * 100.0).abs() < 1e-15);
        assert!((rows[0].funding_pct - 0.000_8).abs() < 1e-12);
        assert_eq!(rows[0].volume, 618_847_551.336_845);
    }

    /// A live `GET /account?by=index&value=702384`, trimmed in fields but not
    /// in rows: all six positions are here because the account's totals are
    /// the sum over all six, and a subset of them proves nothing.
    fn account() -> Value {
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

    #[test]
    fn sign_and_size_become_one_signed_size() {
        let held = parse_account(&account()).positions;
        assert!((held[0].size - 3.87333).abs() < 1e-12, "sign 1 is a long");
        assert!((held[1].size + 1382.1).abs() < 1e-12, "sign -1 is a short");
        // Neither field alone is the size, and the short is the one that
        // proves it: 1382.1 unsigned is the same string a long would send.
        assert!(held[1].size < 0.0);
    }

    #[test]
    fn a_position_is_marked_at_what_the_venue_says_it_is_worth() {
        let held = parse_account(&account()).positions;
        assert!((held[0].mark - 251_677.750_743 / 3.87333).abs() < 1e-9);
        // A short is worth a positive amount, so the mark stays positive.
        assert!((held[1].mark - 942.882_441 / 1382.1).abs() < 1e-12);
        assert!(held[1].mark > 0.0);
    }

    #[test]
    fn cross_positions_carry_their_share_of_the_accounts_requirement() {
        let parsed = parse_account(&account());
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
        let held = parse_account(&account()).positions;
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
        let raw = account();
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
        let parsed = parse_account(&account());
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
        // `maintenance_margin_fraction` for these six markets, from the same
        // live `/orderBookDetails` the account snapshot was taken beside.
        let published = [
            ("BTC", 120.0),
            ("SUI", 600.0),
            ("ENA", 600.0),
            ("OP", 399.0),
            ("NMR", 2000.0),
            ("STRK", 1200.0),
        ];
        let parsed = parse_account(&account());
        let held: f64 = parsed
            .positions
            .iter()
            .map(|position| {
                let (_, basis_points) = published
                    .iter()
                    .find(|(name, _)| *name == position.coin)
                    .expect("every position's market is listed");
                position.mark * position.size.abs() * basis_points / BASIS
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
        let parsed = parse_account(&account());
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
        let flat = parse_account(&account()).positions;
        assert_eq!(flat[0].liq, 0.0);
        assert_eq!(flat[0].risk, 0.0);
    }

    /// Trimmed from a live `GET /orderBookOrders?market_id=1`, keeping the
    /// two bids that rest at the same price.
    fn orders() -> Value {
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

    #[test]
    fn orders_at_one_price_are_one_level() {
        let book = parse_book(&orders());
        assert_eq!(book.bids.len(), 3, "four orders rest at three prices");
        assert!((book.bids[2].price - 64_973.1).abs() < 1e-9);
        assert!((book.bids[2].size - 0.00616).abs() < 1e-12, "0.00308 twice");
    }

    #[test]
    fn depth_accumulates_down_the_side() {
        let book = parse_book(&orders());
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
        let book = parse_book(&orders());
        assert!((book.asks[2].size - 0.64335).abs() < 1e-12);
        assert!((book.asks[1].size - 0.00020).abs() < 1e-12);
    }

    #[test]
    fn the_spread_is_between_the_two_best_prices() {
        let book = parse_book(&orders());
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
        smol::block_on(async {
            let rows = lighter_symbols().await.expect("markets");
            assert!(rows.len() > 100, "the venue lists a couple hundred markets");
            assert!(rows[0].price > 0.0 && rows[0].maintenance > 0.0);
            // Sorted, so the busiest market is the one the id table is asked
            // for, and it has to resolve without a second universe fetch.
            let book = lighter_book(rows[0].name.clone()).await.expect("book");
            assert!(book.mid > 0.0 && !book.bids.is_empty() && !book.asks.is_empty());
            assert!(book.asks[0].price > book.bids[0].price, "crossed book");
            let account = lighter_account("0x3f4ec7684F679F83c782e485b358A2D43045d6A2".to_owned())
                .await
                .expect("account");
            assert!(account.value > 0.0);
            // And a refusal quotes the venue instead of a bare number. This is
            // the half that cannot be checked from a captured payload: ureq
            // makes every 4xx an error and drops the body, and the body is the
            // only place Lighter says what was wrong.
            // `Account` has no `Debug`, so the error comes out of a match
            // rather than `expect_err`.
            let Err(refused) = lighter_account("0xdead".to_owned()).await else {
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
}
