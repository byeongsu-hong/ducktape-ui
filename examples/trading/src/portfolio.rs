//! Portfolio-only data and drawing.
//!
//! This module deliberately does not route through the terminal's position
//! rows, stat cells, or chart package. The dashboard is a separate reading of
//! the account: gross exposure allocation now, and account value over time.

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};
use serde_json::{Value, json};

use crate::Venue;
use crate::hyperliquid::{Account, Fill, HlError, Position, info};
use crate::signing::Chain;
use crate::venue::Network;

const BAR_WIDTH: f64 = 248.0;

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

pub fn portfolio_flow(fills: Vec<Fill>) -> PortfolioFlow {
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

pub fn portfolio_funding(positions: Vec<Position>) -> PortfolioFunding {
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
pub fn portfolio_margin_posted(positions: Vec<Position>) -> f64 {
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

#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioAsset {
    pub coin: String,
    pub side: String,
    pub size: f64,
    pub mark: f64,
    pub value: f64,
    pub share: f64,
    pub bar: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Series {
    values: Vec<(i64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PortfolioHistory {
    day: Series,
    week: Series,
    month: Series,
    all: Series,
    pub note: String,
}

fn notional(position: &Position) -> f64 {
    (position.mark * position.size.abs()).max(0.0)
}

pub fn portfolio_assets(positions: Vec<Position>) -> Vec<PortfolioAsset> {
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
            let cost = position.entry * position.size.abs();
            PortfolioAsset {
                coin: position.coin.clone(),
                side: if position.size >= 0.0 {
                    "LONG"
                } else {
                    "SHORT"
                }
                .to_owned(),
                size: position.size,
                mark: position.mark,
                value,
                share,
                bar: share / 100.0 * BAR_WIDTH,
                pnl: position.pnl,
                pnl_pct: if cost > 0.0 {
                    position.pnl / cost * 100.0
                } else {
                    0.0
                },
            }
        })
        .collect();
    assets.sort_by(|left, right| right.value.total_cmp(&left.value));
    assets
}

pub fn portfolio_exposure(positions: Vec<Position>) -> f64 {
    positions.iter().map(notional).sum()
}

pub fn portfolio_long_exposure(positions: Vec<Position>) -> f64 {
    positions
        .iter()
        .filter(|position| position.size > 0.0)
        .map(notional)
        .sum()
}

pub fn portfolio_short_exposure(positions: Vec<Position>) -> f64 {
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

fn parse_series(value: &Value) -> Series {
    let values = value
        .get("accountValueHistory")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|point| {
            let point = point.as_array()?;
            Some((point.first()?.as_i64()?, number(point.get(1)?)))
        })
        .collect();
    Series { values }
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
/// that, and the one already drawn appends rather than renaming itself, the
/// way every other control here does.
pub fn range_label(range: String, shown: bool) -> String {
    let state = if shown { ", already showing" } else { "" };
    let span = match range.as_str() {
        "day" => "the last day",
        "week" => "the last week",
        "month" => "the last month",
        _ => "its whole history",
    };
    format!("Show account value over {span}{state}")
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

pub fn demo_portfolio_history() -> PortfolioHistory {
    let month: Vec<_> = (0..48)
        .map(|index: i64| {
            let wave = ((index as f64) * 0.43).sin() * 1_850.0;
            (
                1_780_000_000_000 + index * 43_200_000,
                118_000.0 + index as f64 * 420.0 + wave,
            )
        })
        .collect();
    PortfolioHistory {
        day: Series {
            values: month[44..].to_vec(),
        },
        week: Series {
            values: month[34..].to_vec(),
        },
        month: Series {
            values: month.clone(),
        },
        all: Series { values: month },
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

pub fn portfolio_history_note(history: PortfolioHistory) -> String {
    history.note.clone()
}

pub fn portfolio_history_ready(history: PortfolioHistory, range: String) -> bool {
    selected(&history, &range).values.len() > 1
}

pub fn portfolio_history_start(history: PortfolioHistory, range: String) -> f64 {
    selected(&history, &range)
        .values
        .first()
        .map_or(0.0, |(_, value)| *value)
}

pub fn portfolio_history_end(history: PortfolioHistory, range: String) -> f64 {
    selected(&history, &range)
        .values
        .last()
        .map_or(0.0, |(_, value)| *value)
}

pub fn portfolio_history_change(history: PortfolioHistory, range: String) -> f64 {
    portfolio_history_end(history.clone(), range.clone()) - portfolio_history_start(history, range)
}

pub fn portfolio_history_change_pct(history: PortfolioHistory, range: String) -> f64 {
    let start = portfolio_history_start(history.clone(), range.clone());
    if start == 0.0 {
        0.0
    } else {
        portfolio_history_change(history, range) / start * 100.0
    }
}

struct PerformanceProgram {
    values: Vec<f64>,
}

impl canvas::Program<()> for PerformanceProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let edge = Color::from_rgb8(0x2f, 0x28, 0x23);
        let faint = Color::from_rgb8(0x6b, 0x61, 0x57);
        let up = Color::from_rgb8(0x5f, 0xae, 0x7e);
        let down = Color::from_rgb8(0xd0, 0x64, 0x5a);
        let inset = 12.0;
        let width = (bounds.width - inset * 2.0).max(1.0);
        let height = (bounds.height - inset * 2.0).max(1.0);

        for step in 0..=4 {
            let y = inset + height * step as f32 / 4.0;
            frame.stroke(
                &Path::line(Point::new(inset, y), Point::new(inset + width, y)),
                Stroke::default().with_color(edge).with_width(1.0),
            );
        }
        if self.values.len() < 2 {
            frame.stroke(
                &Path::line(
                    Point::new(inset, inset + height / 2.0),
                    Point::new(inset + width, inset + height / 2.0),
                ),
                Stroke::default().with_color(faint).with_width(1.0),
            );
            return vec![frame.into_geometry()];
        }

        let low = self.values.iter().copied().fold(f64::INFINITY, f64::min);
        let high = self
            .values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let span = (high - low).max(f64::EPSILON);
        let line = Path::new(|path| {
            for (index, value) in self.values.iter().enumerate() {
                let x = inset + width * index as f32 / (self.values.len() - 1) as f32;
                let y = inset + height - ((*value - low) / span) as f32 * height;
                if index == 0 {
                    path.move_to(Point::new(x, y));
                } else {
                    path.line_to(Point::new(x, y));
                }
            }
        });
        let color = if self.values.last() >= self.values.first() {
            up
        } else {
            down
        };
        frame.stroke(&line, Stroke::default().with_color(color).with_width(2.0));
        vec![frame.into_geometry()]
    }
}

pub fn portfolio_performance(history: &PortfolioHistory, range: String) -> Element<'static, ()> {
    iced::widget::canvas(PerformanceProgram {
        values: selected(history, &range)
            .values
            .iter()
            .map(|(_, value)| *value)
            .collect(),
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_are_sorted_and_allocate_the_whole_exposure() {
        let positions = crate::hyperliquid::demo_positions();
        let assets = portfolio_assets(positions);
        assert!(assets.windows(2).all(|pair| pair[0].value >= pair[1].value));
        let share: f64 = assets.iter().map(|asset| asset.share).sum();
        assert!((share - 100.0).abs() < 1e-9, "{share}");
    }

    #[test]
    fn portfolio_payload_prefers_perp_history() {
        let parsed = parse_history(&json!([
            ["month", { "accountValueHistory": [[1, "1"]] }],
            ["perpMonth", { "accountValueHistory": [[1, "10"], [2, "12"]] }]
        ]));
        assert_eq!(
            portfolio_history_start(parsed.clone(), "month".to_owned()),
            10.0
        );
        assert_eq!(portfolio_history_change(parsed, "month".to_owned()), 2.0);
    }

    /// Every expectation below is derived from the fixture rather than typed
    /// beside it. A hand-written total is right until somebody edits a
    /// position, and then it is a test asserting last month's arithmetic.
    #[test]
    fn a_flow_folds_exactly_the_fills_it_was_given() {
        let fills = crate::hyperliquid::demo_fills();
        let flow = portfolio_flow(fills.clone());

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
        let flow = portfolio_flow(opening.clone());
        assert_eq!(flow.trades, opening.len() as i64);
        assert_eq!(flow.closed, 0);
        assert_eq!(flow.realized, 0.0);
    }

    #[test]
    fn funding_splits_the_charge_from_the_credit() {
        let positions = crate::hyperliquid::demo_positions();
        let funding = portfolio_funding(positions.clone());

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
        let funding = portfolio_funding(positions);
        assert_eq!(funding.net, 0.0);
        assert_eq!(funding.paid, 900.0);
        assert_eq!(funding.received, 900.0);
    }

    #[test]
    fn margin_and_leverage_come_off_the_account_they_describe() {
        let positions = crate::hyperliquid::demo_positions();
        let account = crate::hyperliquid::demo_account();

        assert_eq!(
            portfolio_margin_posted(positions.clone()),
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
        assert_eq!(portfolio_exposure(positions.clone()), account.notional);
        assert_eq!(
            portfolio_exposure(positions.clone()),
            portfolio_long_exposure(positions.clone()) + portfolio_short_exposure(positions)
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
