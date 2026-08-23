//! Portfolio-only data and drawing.
//!
//! This module deliberately does not route through the terminal's position
//! rows, stat cells, or chart package. The dashboard is a separate reading of
//! the account: gross exposure allocation now, and account value over time.

use iced::{Element, Length};
use serde_json::{Value, json};
use ui_lang_components::ui::candle_chart::format_ts;
use ui_lang_components::ui::chart::{
    AxisDomain, BarLayout, CartesianCurve, CartesianKind, ChartColor, ChartConfig, ChartData,
    ChartDatum, ChartHit, DomainSpec, SeriesConfig, cartesian_chart,
};

use crate::Venue;
use crate::hyperliquid::{Account, Fill, HlError, Position, chart_theme, fmt_usd, info};
use crate::signing::Chain;
use crate::venue::Network;

const BAR_WIDTH: f64 = 248.0;
/// The rail under the LONG / SHORT tile's two figures; `view.ice` draws it at
/// this width.
const EXPOSURE_RAIL_WIDTH: f64 = 200.0;

/// What this account's fill history says about it.
///
/// Every figure is a fold over fills the venue actually served. A venue that
/// serves none has no flow to report rather than a zeroed one, which is why
/// nothing here is drawn until the caller has asked `venue_account_gap`
/// whether the venue answers fills at all: a realized PnL of `$0.00` and a
/// realized PnL nobody can read are the same pixels and opposite facts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PortfolioFlow {
    /// Every fill, opening and closing alike.
    pub trades: i64,
    /// What those fills moved, at the price each printed at.
    pub volume: f64,
    /// Realized PnL: what the closing side of a round trip actually booked.
    pub realized: f64,
    /// Fills that closed something, split by which way it went. An opening
    /// fill books no PnL and is neither, so these do not sum to `trades`.
    pub wins: i64,
    pub losses: i64,
    /// The two above, so a caller can tell "no closes yet" from "all losses"
    /// rather than reading both out of a 0% win rate.
    pub closed: i64,
    pub win_pct: f64,
}

/// Funding as the reader's cash flow, split into the two directions that a
/// single net figure hides: an account paying 900 and receiving 880 is not
/// the same book as one that paid 20 and received nothing.
///
/// `Position.funding` is what the position has been CHARGED — the sign both
/// adapters normalize to — so a charge is money out and a credit is money in.
/// Both venues publish it, so unlike `PortfolioFlow` this is available
/// wherever there are positions to read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PortfolioFunding {
    /// Money out, as a positive figure.
    pub paid: f64,
    /// Money in, as a positive figure.
    pub received: f64,
    /// `received - paid`: positive when the book was paid to hold itself.
    pub net: f64,
}

pub fn portfolio_flow(fills: &[Fill]) -> PortfolioFlow {
    let wins = fills.iter().filter(|fill| fill.closed_pnl > 0.0).count() as i64;
    let losses = fills.iter().filter(|fill| fill.closed_pnl < 0.0).count() as i64;
    let closed = wins + losses;
    PortfolioFlow {
        trades: fills.len() as i64,
        volume: fills
            .iter()
            .map(|fill| fill.price * fill.size.abs())
            .sum::<f64>(),
        realized: fills.iter().map(|fill| fill.closed_pnl).sum(),
        wins,
        losses,
        closed,
        win_pct: if closed > 0 {
            wins as f64 / closed as f64 * 100.0
        } else {
            0.0
        },
    }
}

pub fn portfolio_funding(positions: &[Position]) -> PortfolioFunding {
    let paid = positions
        .iter()
        .map(|position| position.funding.max(0.0))
        .sum();
    let received = positions
        .iter()
        .map(|position| (-position.funding).max(0.0))
        .sum::<f64>();
    PortfolioFunding {
        paid,
        received,
        net: received - paid,
    }
}

/// Equity the account has posted behind its open positions, summed from the
/// positions themselves rather than taken as `value - withdrawable`: the
/// latter also carries whatever else the venue is holding, and this panel is
/// about the positions.
pub fn portfolio_margin_posted(positions: &[Position]) -> f64 {
    positions.iter().map(|position| position.margin).sum()
}

/// Gross marked notional against the equity standing behind it — the leverage
/// the account is actually running, as opposed to the per-market leverage each
/// position was opened at.
///
/// Zero equity answers zero rather than an infinity the column would have to
/// render: an account with no value has no position to be levered against it.
pub fn portfolio_leverage(account: Option<Account>) -> f64 {
    account
        .filter(|account| account.value > 0.0)
        .map_or(0.0, |account| account.notional / account.value)
}

/// One position as the allocation table reads it: what it is worth, what share
/// of the book that is, and how it is doing.
///
/// The last of those is the position's own `roe_pct` rather than a second
/// derivation of it. This column used to divide the PnL by notional while the
/// terminal's position row divided it by margin, and both rendered as a bare
/// percentage under a header spelled UNREALIZED — so the fixture's 40x BTC leg
/// read `+21.44%` on the dashboard and `+857.41%` in the terminal, same
/// account, same position, nothing on either screen saying they were different
/// questions. ROE is the one the venue itself answers: the Hyperliquid adapter
/// takes `roe_pct` straight from `returnOnEquity`, so a dashboard computing its
/// own return would be contradicting the exchange's reported figure for the
/// position beside it.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioAsset {
    pub coin: String,
    pub side: String,
    pub size: f64,
    pub entry: f64,
    pub mark: f64,
    pub liq: f64,
    pub leverage: f64,
    pub margin_mode: String,
    pub margin: f64,
    pub funding: f64,
    pub value: f64,
    pub share: f64,
    pub bar: f64,
    pub pnl: f64,
    pub roe_pct: f64,
}

/// One window of the venue's portfolio answer: the account's value at each
/// point, and the PnL it had booked by then, cumulative over the window.
/// The venue reports the two as separate series and they are kept separate
/// here, because value moves on deposits and withdrawals as well as on
/// trading and only the second is performance.
#[derive(Clone, Debug, Default, PartialEq)]
struct Series {
    values: Vec<(i64, f64)>,
    pnl: Vec<(i64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PortfolioHistory {
    day: Series,
    week: Series,
    month: Series,
    all: Series,
    pub note: String,
}

/// A point the reader is holding the pointer over on one of the portfolio
/// charts. Opaque to Ice the way `Draft` is: the view reads it through the
/// accessors below and passes it back to the chart unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioHover {
    hit: ChartHit,
    label: String,
    value: f64,
}

fn notional(position: &Position) -> f64 {
    (position.mark * position.size.abs()).max(0.0)
}

pub fn portfolio_assets(positions: &[Position]) -> Vec<PortfolioAsset> {
    let total: f64 = positions.iter().map(notional).sum();
    let mut assets: Vec<_> = positions
        .iter()
        .map(|position| {
            let value = notional(position);
            let share = if total > 0.0 {
                value / total * 100.0
            } else {
                0.0
            };
            PortfolioAsset {
                coin: position.coin.clone(),
                side: if position.size >= 0.0 {
                    "LONG"
                } else {
                    "SHORT"
                }
                .to_owned(),
                size: position.size,
                entry: position.entry,
                mark: position.mark,
                liq: position.liq,
                leverage: position.leverage,
                margin_mode: position.margin_mode.clone(),
                margin: position.margin,
                funding: position.funding,
                value,
                share,
                bar: share / 100.0 * BAR_WIDTH,
                pnl: position.pnl,
                roe_pct: position.roe_pct,
            }
        })
        .collect();
    assets.sort_by(|left, right| right.value.total_cmp(&left.value));
    assets
}

/// What pressing an asset row does. The same sentence the terminal's
/// position row announces, because it is the same act.
pub fn asset_label(asset: PortfolioAsset) -> String {
    format!("Open the {} market", asset.coin)
}

pub fn portfolio_exposure(positions: &[Position]) -> f64 {
    positions.iter().map(notional).sum()
}

pub fn portfolio_long_exposure(positions: &[Position]) -> f64 {
    positions
        .iter()
        .filter(|position| position.size > 0.0)
        .map(notional)
        .sum()
}

/// The long side's share of gross exposure as the width of a rail
/// [`EXPOSURE_RAIL_WIDTH`] wide, so the LONG / SHORT tile can draw the split
/// rather than leave the reader to compare two figures.
pub fn portfolio_long_rail(positions: &[Position]) -> f64 {
    let long = portfolio_long_exposure(positions);
    let total = portfolio_exposure(positions);
    if total <= 0.0 {
        return 0.0;
    }
    long / total * EXPOSURE_RAIL_WIDTH
}

pub fn portfolio_short_exposure(positions: &[Position]) -> f64 {
    positions
        .iter()
        .filter(|position| position.size < 0.0)
        .map(notional)
        .sum()
}

fn number(value: &Value) -> f64 {
    match value {
        Value::String(text) => text.parse().unwrap_or_default(),
        Value::Number(number) => number.as_f64().unwrap_or_default(),
        _ => 0.0,
    }
}

fn parse_points(value: &Value, key: &str) -> Vec<(i64, f64)> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|point| {
            let point = point.as_array()?;
            Some((point.first()?.as_i64()?, number(point.get(1)?)))
        })
        .collect()
}

fn parse_series(value: &Value) -> Series {
    Series {
        values: parse_points(value, "accountValueHistory"),
        pnl: parse_points(value, "pnlHistory"),
    }
}

fn parse_history(value: &Value) -> PortfolioHistory {
    let mut history = PortfolioHistory::default();
    for entry in value.as_array().map(Vec::as_slice).unwrap_or_default() {
        let Some(pair) = entry.as_array() else {
            continue;
        };
        let Some(name) = pair.first().and_then(Value::as_str) else {
            continue;
        };
        let Some(body) = pair.get(1) else { continue };
        match name {
            "perpDay" => history.day = parse_series(body),
            "perpWeek" => history.week = parse_series(body),
            "perpMonth" => history.month = parse_series(body),
            "perpAllTime" => history.all = parse_series(body),
            "day" if history.day.values.is_empty() => history.day = parse_series(body),
            "week" if history.week.values.is_empty() => history.week = parse_series(body),
            "month" if history.month.values.is_empty() => history.month = parse_series(body),
            "allTime" if history.all.values.is_empty() => history.all = parse_series(body),
            _ => {}
        }
    }
    history
}

/// This account's realised history on whichever Hyperliquid deployment is
/// named. One function per operation on the registry rather than one per
/// network, so a deployment added there is wired by the entry rather than by a
/// second arm here.
pub async fn hl_portfolio(chain: Chain, address: String) -> Result<PortfolioHistory, HlError> {
    info(chain, json!({ "type": "portfolio", "user": address }))
        .await
        .map(|value| parse_history(&value))
}

pub async fn venue_portfolio(venue: Venue, address: String) -> Result<PortfolioHistory, HlError> {
    if address.trim().is_empty() {
        return Ok(portfolio_empty());
    }
    (Network::of(venue).portfolio)(address).await
}

/// What a range button announces.
///
/// These were labelled with `page_label`, which made each one say "Show the 1d
/// page" — naming navigation that does not happen and a page that does not
/// exist. A range is how far back the account-value line is drawn, so it says
/// that, while the selected state is exposed directly through the button's
/// accessibility state.
pub fn range_label(range: String) -> String {
    let span = match range.as_str() {
        "day" => "the last day",
        "week" => "the last week",
        "month" => "the last month",
        _ => "its whole history",
    };
    format!("Show account value over {span}")
}

/// The window as a heading over the figures read off it.
pub fn range_heading(locale: crate::Locale, range: &str) -> String {
    crate::i18n::t(
        locale,
        match range {
            "day" => "LAST DAY",
            "week" => "LAST WEEK",
            "month" => "LAST MONTH",
            _ => "ALL TIME",
        }
        .to_owned(),
    )
}

pub fn portfolio_empty() -> PortfolioHistory {
    PortfolioHistory {
        note: "Connect an address to load portfolio performance.".to_owned(),
        ..PortfolioHistory::default()
    }
}

pub fn portfolio_unavailable(message: String) -> PortfolioHistory {
    PortfolioHistory {
        note: message,
        ..PortfolioHistory::default()
    }
}

/// The account every other panel reads, given a past.
///
/// Anchored on `demo_account().value` rather than invented beside it: the
/// tile at the top of the page and the headline over this chart both say
/// ACCOUNT VALUE, and a fixture that ended anywhere else had them disagree
/// by 27x on one screen. Each range is its own walk at its own sample rate —
/// a day of hours, a week of quarter-days, a month of half-days, a year of
/// days — so the four buttons draw four different lines, and `all` is not
/// `month` under another name.
pub fn demo_portfolio_history() -> PortfolioHistory {
    const NOW_MS: i64 = 1_786_100_000_000;
    let end = crate::hyperliquid::demo_account().value;
    let walk = |points: i64, step_ms: i64, swing: f64, drift: f64, phase: f64| {
        let values: Vec<(i64, f64)> = (0..points)
            .map(|index| {
                let t = index as f64 / (points - 1) as f64;
                let wave = (t * 9.0 + phase).sin() * swing + (t * 23.0 + phase).sin() * swing * 0.3;
                // Pinned to the account at the last point, whatever the wave
                // did on the way: the fixture's one promise is where it ends.
                let value = end - drift * (1.0 - t) - wave * (1.0 - t);
                (NOW_MS - (points - 1 - index) * step_ms, value)
            })
            .collect();
        let first = values[0].1;
        let pnl = values
            .iter()
            .map(|(ts, value)| (*ts, value - first))
            .collect();
        Series { values, pnl }
    };
    PortfolioHistory {
        day: walk(25, 3_600_000, 2_400.0, -1_150.0, 0.4),
        week: walk(29, 6 * 3_600_000, 9_800.0, 21_549.22, 1.3),
        month: walk(61, 12 * 3_600_000, 38_000.0, 176_400.0, 2.1),
        all: walk(181, 24 * 3_600_000, 120_000.0, 611_000.0, 0.9),
        note: String::new(),
    }
}

fn selected<'a>(history: &'a PortfolioHistory, range: &str) -> &'a Series {
    match range {
        "day" => &history.day,
        "week" => &history.week,
        "all" => &history.all,
        _ => &history.month,
    }
}

pub fn portfolio_history_note(history: &PortfolioHistory) -> String {
    history.note.clone()
}

pub fn portfolio_history_ready(history: &PortfolioHistory, range: &str) -> bool {
    selected(history, range).values.len() > 1
}

pub fn portfolio_history_start(history: &PortfolioHistory, range: &str) -> f64 {
    selected(history, range)
        .values
        .first()
        .map_or(0.0, |(_, value)| *value)
}

pub fn portfolio_history_end(history: &PortfolioHistory, range: &str) -> f64 {
    selected(history, range)
        .values
        .last()
        .map_or(0.0, |(_, value)| *value)
}

pub fn portfolio_history_change(history: &PortfolioHistory, range: &str) -> f64 {
    portfolio_history_end(history, range) - portfolio_history_start(history, range)
}

pub fn portfolio_history_change_pct(history: &PortfolioHistory, range: &str) -> f64 {
    let start = portfolio_history_start(history, range);
    if start == 0.0 {
        0.0
    } else {
        portfolio_history_change(history, range) / start * 100.0
    }
}

/// The highest the account stood over the window.
pub fn portfolio_history_peak(history: &PortfolioHistory, range: &str) -> f64 {
    selected(history, range)
        .values
        .iter()
        .map(|(_, value)| *value)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0)
}

/// How far under its peak the account stands now, as a share of that peak.
/// Zero at a new high; never negative.
pub fn portfolio_history_drawdown(history: &PortfolioHistory, range: &str) -> f64 {
    let peak = portfolio_history_peak(history, range);
    let end = portfolio_history_end(history, range);
    if peak <= 0.0 {
        0.0
    } else {
        ((peak - end) / peak * 100.0).max(0.0)
    }
}

/// The deepest peak-to-trough fall anywhere in the window, as a share of the
/// peak it fell from — what a reader asks about a curve before anything else.
pub fn portfolio_history_max_drawdown(history: &PortfolioHistory, range: &str) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut worst: f64 = 0.0;
    for (_, value) in &selected(history, range).values {
        peak = peak.max(*value);
        if peak > 0.0 {
            worst = worst.max((peak - value) / peak * 100.0);
        }
    }
    worst
}

/// PnL booked over the window: the cumulative series' last point less its
/// first, which is also what the bars under it sum to. The two are one figure
/// whether or not the venue starts the window's series at zero. Distinct from
/// the change in value: a deposit moves one and not the other.
pub fn portfolio_history_pnl(history: &PortfolioHistory, range: &str) -> f64 {
    let pnl = &selected(history, range).pnl;
    match (pnl.first(), pnl.last()) {
        (Some((_, first)), Some((_, last))) => last - first,
        _ => 0.0,
    }
}

/// Whether the venue sent a PnL series at all for this window. Hyperliquid
/// does; a venue read through an address alone does not, and the bars then
/// say so rather than drawing a flat zero.
pub fn portfolio_pnl_ready(history: &PortfolioHistory, range: &str) -> bool {
    selected(history, range).pnl.len() > 1
}

pub fn hover_label(hover: PortfolioHover) -> String {
    hover.label
}

pub fn hover_value(hover: PortfolioHover) -> f64 {
    hover.value
}

/// The time-axis label for one point, at the granularity the window's
/// spacing calls for: a day of hours reads as clock time, a month of
/// half-days as dates.
fn point_label(points: &[(i64, f64)], index: usize) -> String {
    let step = match points {
        [(first, _), (second, _), ..] => (second - first) / 1_000,
        _ => 86_400,
    };
    // Steps of a quarter-day and up are read as dates: a month of half-days
    // labelled by the clock is a row of `10:53`s saying nothing about which
    // day any of them is.
    let step = if step >= 6 * 3_600 {
        step.max(86_400)
    } else {
        step
    };
    format_ts(points[index].0 / 1_000, step)
}

/// The value axis of a curve, held to the curve: from zero, a month that
/// moved 6% on a seven-figure account is a flat line under the ceiling.
fn value_domain(points: &[(i64, f64)]) -> DomainSpec {
    let (low, high) = points.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(low, high), (_, value)| (low.min(*value), high.max(*value)),
    );
    if !low.is_finite() || !high.is_finite() {
        return DomainSpec::default();
    }
    let pad = ((high - low) * 0.08).max(high.abs() * 0.001).max(1.0);
    DomainSpec {
        x: None,
        y: Some(AxisDomain::new((low - pad) as f32, (high + pad) as f32)),
    }
}

/// A money axis tick. Two more digits than the compact figures elsewhere
/// carry, because five ticks a few percent apart all read `$3.7M` at one.
fn money_tick(value: f32) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let magnitude = value.abs();
    if magnitude >= 1_000_000.0 {
        format!("{sign}${:.2}M", magnitude / 1_000_000.0)
    } else if magnitude >= 1_000.0 {
        format!("{sign}${:.1}K", magnitude / 1_000.0)
    } else {
        format!("{sign}${magnitude:.0}")
    }
}

/// One series of points as the chart kit reads them.
fn chart_data(points: &[(i64, f64)], key: &str) -> ChartData {
    ChartData::new(points.iter().enumerate().map(|(index, (_, value))| {
        ChartDatum::new(index as f32, point_label(points, index)).with_value(key, *value as f32)
    }))
}

fn hovered(points: &[(i64, f64)], hover: Option<PortfolioHover>) -> Option<ChartHit> {
    hover
        .filter(|hover| hover.hit.datum_index < points.len())
        .map(|hover| hover.hit)
}

fn hover_at(points: &[(i64, f64)], hit: Option<ChartHit>) -> Option<PortfolioHover> {
    let hit = hit?;
    let (_, value) = points.get(hit.datum_index)?;
    Some(PortfolioHover {
        label: point_label(points, hit.datum_index),
        value: *value,
        hit,
    })
}

/// Account value over the window: an area under a line, a money axis, and
/// the point under the pointer answered back so the panel can print it.
///
/// Drawn with the workspace's chart kit rather than a program of its own:
/// the kit owns the axes, the hover geometry and the cache, and a second
/// renderer for one line is a second place for a gridline to go wrong.
pub fn portfolio_performance(
    history: &PortfolioHistory,
    range: String,
    hover: Option<PortfolioHover>,
) -> Element<'static, Option<PortfolioHover>> {
    let points = selected(history, &range).values.clone();
    let up = points.last().map(|(_, v)| *v) >= points.first().map(|(_, v)| *v);
    let config = ChartConfig::new([SeriesConfig::new(
        "value",
        "Account value",
        if up {
            ChartColor::Success
        } else {
            ChartColor::Destructive
        },
    )]);
    let data = chart_data(&points, "value");
    let for_hover = points.clone();
    cartesian_chart(&config, &data, &chart_theme())
        .kind(CartesianKind::Area { points: false })
        .curve(CartesianCurve::Monotone)
        .domain(value_domain(&points))
        .tick_format(money_tick)
        .hovered(hovered(&points, hover))
        .on_hover(move |hit| hover_at(&for_hover, hit))
        .height(Length::Fill)
        .into()
}

/// PnL booked per step of the window, as bars: what each hour, quarter-day
/// or day actually made or lost, read off the venue's cumulative series by
/// differencing it. Gains and losses are two series so each carries its own
/// colour; the one that does not apply at a point is zero and draws nothing.
pub fn portfolio_pnl_bars(
    history: &PortfolioHistory,
    range: String,
    hover: Option<PortfolioHover>,
) -> Element<'static, Option<PortfolioHover>> {
    let cumulative = &selected(history, &range).pnl;
    let steps: Vec<(i64, f64)> = cumulative
        .windows(2)
        .map(|pair| (pair[1].0, pair[1].1 - pair[0].1))
        .collect();
    let config = ChartConfig::new([
        SeriesConfig::new("gain", "Gain", ChartColor::Success),
        SeriesConfig::new("loss", "Loss", ChartColor::Destructive),
    ]);
    let data = ChartData::new(steps.iter().enumerate().map(|(index, (_, delta))| {
        ChartDatum::new(index as f32, point_label(&steps, index))
            .with_value("gain", delta.max(0.0) as f32)
            .with_value("loss", delta.min(0.0) as f32)
    }));
    let for_hover = steps.clone();
    cartesian_chart(&config, &data, &chart_theme())
        .kind(CartesianKind::Bar(BarLayout::Stacked))
        .tick_format(money_tick)
        .hovered(hovered(&steps, hover))
        .on_hover(move |hit| hover_at(&for_hover, hit))
        .height(Length::Fill)
        .into()
}

/// What the readout over a chart says for the point under the pointer.
pub fn hover_readout(hover: Option<PortfolioHover>, signed: bool) -> String {
    let Some(hover) = hover else {
        return String::new();
    };
    let value = if signed {
        crate::hyperliquid::fmt_pnl(hover.value)
    } else {
        fmt_usd(hover.value)
    };
    format!("{}  {value}", hover.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_are_sorted_and_allocate_the_whole_exposure() {
        let positions = crate::hyperliquid::demo_positions();
        let assets = portfolio_assets(&positions);
        assert!(assets.windows(2).all(|pair| pair[0].value >= pair[1].value));
        let share: f64 = assets.iter().map(|asset| asset.share).sum();
        assert!((share - 100.0).abs() < 1e-9, "{share}");
    }

    /// The dashboard's percentage is the terminal's, because both answer the
    /// same question about the same position. Nothing is derived a second time
    /// here: a return over notional and a return over margin are the same
    /// figure only at 1x, and every expectation below comes off the fixture so
    /// that a column quietly divided by something else stops reading as
    /// agreement.
    #[test]
    fn an_asset_reports_the_same_return_the_terminal_does() {
        let positions = crate::hyperliquid::demo_positions();
        let assets = portfolio_assets(&positions);

        for position in &positions {
            let asset = assets
                .iter()
                .find(|asset| asset.coin == position.coin)
                .expect("every position is allocated a row");
            assert_eq!(asset.pnl, position.pnl, "{}", position.coin);
            assert_eq!(
                asset.roe_pct, position.roe_pct,
                "{}: the dashboard is not reading the position's own return",
                position.coin
            );
        }

        // And the assertion above is not vacuous: at 1x the two bases coincide,
        // so a fixture of unlevered positions would agree whichever one the
        // column divided by. This one holds a leg that tells them apart.
        let levered = positions
            .iter()
            .find(|position| {
                let over_notional = position.pnl / (position.entry * position.size.abs()) * 100.0;
                (position.roe_pct - over_notional).abs() > 1.0
            })
            .map(|position| position.coin.clone());
        assert!(
            levered.is_some(),
            "no fixture position separates a return over margin from one over notional"
        );
    }

    /// The fixture's one promise: it ends where the account the tiles read
    /// stands, on every range, so two ACCOUNT VALUE labels on one page cannot
    /// name two accounts.
    #[test]
    fn the_demo_history_ends_where_the_demo_account_stands() {
        let value = crate::hyperliquid::demo_account().value;
        for range in ["day", "week", "month", "all"] {
            let end = portfolio_history_end(&demo_portfolio_history(), range);
            assert!((end - value).abs() < 1e-6, "{range}: {end} vs {value}");
        }
    }

    /// Four buttons draw four lines: each range has its own span and its own
    /// start, and `all` is not `month` under another name.
    #[test]
    fn the_demo_ranges_are_four_different_windows() {
        let history = demo_portfolio_history();
        let starts: Vec<f64> = ["day", "week", "month", "all"]
            .into_iter()
            .map(|range| portfolio_history_start(&history, range))
            .collect();
        for pair in starts.windows(2) {
            assert!((pair[0] - pair[1]).abs() > 1.0, "{starts:?}");
        }
        let spans: Vec<i64> = [&history.day, &history.week, &history.month, &history.all]
            .into_iter()
            .map(|series| series.values.last().unwrap().0 - series.values.first().unwrap().0)
            .collect();
        assert!(spans.windows(2).all(|pair| pair[0] < pair[1]), "{spans:?}");
    }

    /// Drawdown is read off the curve: a series that only ever rose stands at
    /// its peak, and one that fell from a high and did not recover is under
    /// it by exactly that much.
    #[test]
    fn drawdown_is_the_fall_from_the_running_peak() {
        let series = |values: &[f64]| PortfolioHistory {
            month: Series {
                values: values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i as i64, *v))
                    .collect(),
                pnl: Vec::new(),
            },
            ..PortfolioHistory::default()
        };
        let rising = series(&[100.0, 110.0, 120.0]);
        assert_eq!(portfolio_history_drawdown(&rising, "month"), 0.0);
        assert_eq!(portfolio_history_max_drawdown(&rising, "month"), 0.0);
        let fallen = series(&[100.0, 200.0, 150.0, 160.0]);
        assert!((portfolio_history_drawdown(&fallen, "month") - 20.0).abs() < 1e-9);
        assert!((portfolio_history_max_drawdown(&fallen, "month") - 25.0).abs() < 1e-9);
    }

    /// The bars are the venue's cumulative PnL differenced, so they sum back
    /// to the window's PnL and a point between two equal readings is a zero
    /// bar rather than a missing one.
    #[test]
    fn pnl_bars_difference_the_cumulative_series() {
        let history = PortfolioHistory {
            month: Series {
                values: vec![(0, 100.0), (1, 100.0), (2, 100.0)],
                pnl: vec![(0, 0.0), (1, 5.0), (2, 2.0)],
            },
            ..PortfolioHistory::default()
        };
        assert!(portfolio_pnl_ready(&history, "month"));
        assert_eq!(portfolio_history_pnl(&history, "month"), 2.0);
        let cumulative = &selected(&history, "month").pnl;
        let steps: Vec<f64> = cumulative
            .windows(2)
            .map(|pair| pair[1].1 - pair[0].1)
            .collect();
        assert_eq!(steps, vec![5.0, -3.0]);
        assert_eq!(steps.iter().sum::<f64>(), 2.0);
    }

    #[test]
    fn portfolio_payload_reads_both_series() {
        let parsed = parse_history(&json!([
            ["perpMonth", { "accountValueHistory": [[1, "10"], [2, "12"]], "pnlHistory": [[1, "0"], [2, "2"]] }]
        ]));
        assert_eq!(portfolio_history_pnl(&parsed, "month"), 2.0);
        assert!(portfolio_pnl_ready(&parsed, "month"));
    }

    #[test]
    fn portfolio_payload_prefers_perp_history() {
        let parsed = parse_history(&json!([
            ["month", { "accountValueHistory": [[1, "1"]] }],
            ["perpMonth", { "accountValueHistory": [[1, "10"], [2, "12"]] }]
        ]));
        assert_eq!(portfolio_history_start(&parsed, "month"), 10.0);
        assert_eq!(portfolio_history_change(&parsed, "month"), 2.0);
    }

    /// Every expectation below is derived from the fixture rather than typed
    /// beside it. A hand-written total is right until somebody edits a
    /// position, and then it is a test asserting last month's arithmetic.
    #[test]
    fn a_flow_folds_exactly_the_fills_it_was_given() {
        let fills = crate::hyperliquid::demo_fills();
        let flow = portfolio_flow(&fills);

        assert_eq!(flow.trades, fills.len() as i64);
        assert_eq!(
            flow.realized,
            fills.iter().map(|fill| fill.closed_pnl).sum::<f64>()
        );
        assert_eq!(
            flow.volume,
            fills
                .iter()
                .map(|fill| fill.price * fill.size.abs())
                .sum::<f64>()
        );
        assert_eq!(flow.wins + flow.losses, flow.closed);
        // An opening fill books no PnL, so it is neither a win nor a loss and
        // the two do not have to reach `trades`.
        assert!(flow.closed <= flow.trades);
        assert_eq!(flow.win_pct, flow.wins as f64 / flow.closed as f64 * 100.0);
    }

    /// The distinction the panel exists to draw: no round trip has closed, so
    /// there is no win rate — which the struct reports as `closed == 0` rather
    /// than as a rate of zero, because the view renders those differently.
    #[test]
    fn nothing_closed_is_not_a_zero_percent_win_rate() {
        let opening: Vec<_> = crate::hyperliquid::demo_fills()
            .into_iter()
            .filter(|fill| fill.closed_pnl == 0.0)
            .collect();
        assert!(!opening.is_empty());
        let flow = portfolio_flow(&opening);
        assert_eq!(flow.trades, opening.len() as i64);
        assert_eq!(flow.closed, 0);
        assert_eq!(flow.realized, 0.0);
    }

    #[test]
    fn funding_splits_the_charge_from_the_credit() {
        let positions = crate::hyperliquid::demo_positions();
        let funding = portfolio_funding(&positions);

        assert_eq!(
            funding.paid,
            positions
                .iter()
                .filter(|position| position.funding > 0.0)
                .map(|position| position.funding)
                .sum::<f64>()
        );
        assert_eq!(
            funding.received,
            positions
                .iter()
                .filter(|position| position.funding < 0.0)
                .map(|position| -position.funding)
                .sum::<f64>()
        );
        // Both sides are quoted positive, so the net is their difference and
        // never the raw sum of a field whose sign means the opposite thing.
        assert_eq!(funding.net, funding.received - funding.paid);
        assert_eq!(
            funding.net,
            -positions
                .iter()
                .map(|position| position.funding)
                .sum::<f64>()
        );
    }

    /// A charge and a credit of the same size are not a book that paid
    /// nothing, and a net alone cannot tell them apart.
    #[test]
    fn funding_that_nets_to_nothing_still_states_both_sides() {
        let mut positions = crate::hyperliquid::demo_positions();
        positions[0].funding = 900.0;
        positions[1].funding = -900.0;
        positions[2].funding = 0.0;
        let funding = portfolio_funding(&positions);
        assert_eq!(funding.net, 0.0);
        assert_eq!(funding.paid, 900.0);
        assert_eq!(funding.received, 900.0);
    }

    #[test]
    fn margin_and_leverage_come_off_the_account_they_describe() {
        let positions = crate::hyperliquid::demo_positions();
        let account = crate::hyperliquid::demo_account();

        assert_eq!(
            portfolio_margin_posted(&positions),
            positions
                .iter()
                .map(|position| position.margin)
                .sum::<f64>()
        );
        assert_eq!(
            portfolio_leverage(Some(account.clone())),
            account.notional / account.value
        );
        // Gross exposure and the account's own notional are the same fold over
        // the same positions, so the two figures on the page agree.
        assert_eq!(portfolio_exposure(&positions), account.notional);
        assert_eq!(
            portfolio_exposure(&positions),
            portfolio_long_exposure(&positions) + portfolio_short_exposure(&positions)
        );
    }

    /// No account is not an account at zero leverage, and no equity is not
    /// infinite leverage. Both answer zero, which is the only figure the
    /// column can render without asserting something false.
    #[test]
    fn leverage_without_equity_is_not_an_infinity() {
        assert_eq!(portfolio_leverage(None), 0.0);
        let mut broke = crate::hyperliquid::demo_account();
        broke.value = 0.0;
        assert_eq!(portfolio_leverage(Some(broke)), 0.0);
    }
}
