//! What a venue owes the terminal, so one screen can show two exchanges.
//!
//! The panels, the folds, the formatters, the ticket's arithmetic and the
//! chart adapter never learn whose exchange they are drawing. What does is
//! short: three reads, and the handful of figures the two venues state in
//! different units. Those live here rather than in either adapter, because a
//! conversion that lives with one venue is a rule the other one has to know
//! about.
//!
//! Nothing here signs or sends. The boundary is the same one the ticket stops
//! at: everything up to the signature is arithmetic worth having.

// `Reads::LIGHTER` is real wiring, but the session that switches between
// venues has not landed to read it.
#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;

use crate::hyperliquid::{Account, Book, HlError, SymbolRow};
use crate::lighter::{lighter_account, lighter_book, lighter_symbols};

/// The exchanges this terminal can be pointed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Venue {
    Hyperliquid,
    Lighter,
}

impl Venue {
    /// Every venue the app can show, in the order a switcher lists them.
    pub const ALL: [Venue; 2] = [Venue::Hyperliquid, Venue::Lighter];

    /// What the screen calls it.
    pub fn name(self) -> &'static str {
        match self {
            Venue::Hyperliquid => "Hyperliquid",
            Venue::Lighter => "Lighter",
        }
    }

    /// What state, keys and messages call it: lowercase, stable, and never
    /// shown. Renaming what the tab says must not invalidate what was stored
    /// under it, so the two are separate strings rather than one lowercased.
    pub fn id(self) -> &'static str {
        match self {
            Venue::Hyperliquid => "hyperliquid",
            Venue::Lighter => "lighter",
        }
    }
}

/// One read in flight.
///
/// A fn pointer cannot name the anonymous future an `async fn` returns, and
/// the app holds whichever venue is on screen at runtime, so the future is
/// boxed at the seam — one allocation against a round trip to an exchange.
/// A trait would force the same box at every call site instead of naming it
/// once here, because a trait of `async fn`s is not dyn-compatible.
///
/// The error is `HlError` rather than a second one-field type: a failure is a
/// message, and which venue produced it is on the `Reads` the caller asked
/// through. It is misnamed until the module split moves it out of
/// `hyperliquid.rs`, along with the rest of the neutral shapes parked there.
pub type Fetch<T> = Pin<Box<dyn Future<Output = Result<T, HlError>> + Send>>;

/// Everything the terminal asks a venue for. Read-only, and each answer is
/// already in the shape the panels read rather than the shape the exchange
/// returned — the field map is the adapter's whole job.
///
/// Three reads rather than a rounder number, because these are the three both
/// adapters can actually answer with a function. A field no venue can fill is
/// a guess about the seam rather than a description of it; see the note under
/// `LIGHTER` for the two that are not here yet and why.
///
/// Markets are named by their ticker, which is what every panel holds.
/// Hyperliquid keys its requests by that ticker too; Lighter keys the book by
/// a numeric `market_id`, so its adapter carries the ticker-to-id table it
/// already read out of the universe. That is a lookup, not a second
/// identifier for the app to carry.
pub struct Reads {
    /// Which exchange answers these.
    pub venue: Venue,
    /// The tradeable universe: one row per market, with the day's figures and
    /// the margin rule that market holds a position to.
    pub markets: fn() -> Fetch<Vec<SymbolRow>>,
    /// The top of one market's book, by ticker.
    pub book: fn(String) -> Fetch<Book>,
    /// One account, by the address that owns it — equity, what the margin
    /// engine holds against it, and every open position.
    pub account: fn(String) -> Fetch<Account>,
}

impl Reads {
    /// Lighter, wired to the three functions its adapter publishes. The
    /// closures are here rather than in `lighter.rs` because the box is this
    /// seam's cost, not the adapter's: an adapter that returned a boxed future
    /// would pay for it even when called directly.
    pub const LIGHTER: Reads = Reads {
        venue: Venue::Lighter,
        markets: || Box::pin(lighter_symbols()),
        book: |coin| Box::pin(lighter_book(coin)),
        account: |address| Box::pin(lighter_account(address)),
    };
}

// There is no `Reads::HYPERLIQUID`, and the gap is in `hyperliquid.rs` rather
// than here: `hl_symbols` and `hl_account` are public, but the book is read off
// the websocket and both `info` and `parse_book` are private to that module.
// The request is not the problem — `{"type":"l2Book","coin":"BTC"}` answers 200
// with a `levels` pair, checked live — so this is one `pub` away, and it is on
// the same list the module split owes.
//
// The public tape is not a read on this seam at all. `Trade` cannot be built
// outside `hyperliquid.rs`, because its `tid` is private, so neither adapter
// could return one; Hyperliquid's own prints arrive on the websocket rather
// than through a function to point at. Whoever lands it inherits an ordering
// question that is now settled and was previously documented backwards: both
// endpoints answer newest first. `{"type":"recentTrades","coin":"BTC"}` came
// back with `time` non-increasing across every print, and Lighter's
// `recentTrades?market_id=1` with `timestamp` and `trade_id` both descending.
// That is already the order the app holds a tape in — `push_trades` reverses a
// websocket beat to put its newest print on top — so the read wants no reverse.

/// What a market's margin engine holds a position to.
///
/// The one figure in the ticket's arithmetic that is a venue's rule rather
/// than arithmetic, and the two venues state it in different units.
/// Hyperliquid publishes a maximum leverage and holds half the margin at it;
/// Lighter publishes the fractions themselves, in basis points. The panels
/// read a fraction, so each venue converts into one here and the shared math
/// never learns either rule.
///
/// Zero in either figure means the venue did not state it, not that the venue
/// requires nothing. Both constructors agree on that and agree on how it is
/// reached: each figure is zeroed on its own, from the input that was missing,
/// and neither invents the other from what was stated. A requirement read as a
/// real zero would put the cliff further from the entry than it is, which is
/// why the ticket refuses to quote a liquidation for a market it has not read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginRule {
    /// The share of a position's value held against it while it is open —
    /// `SymbolRow.maintenance`, which is what the ticket's liquidation is
    /// priced against.
    pub maintenance: f64,
    /// The largest leverage the venue will open a position at, which is what
    /// the ticket holds a typed leverage to.
    pub leverage: f64,
}

impl MarginRule {
    /// Half the initial margin at the cap in force, so a 40x market maintains
    /// at 1/80th of a position's value.
    ///
    /// Settled rather than quoted: on the 127-position cross book of
    /// 0x8cc94dc8…3512b6a, each position's `positionValue` over twice its
    /// market's `maxLeverage`, summed, is 1423198.709379 against a published
    /// `crossMaintenanceMarginUsed` of 1423198.709367. The residual is 1.2e-05
    /// on 1.4 million, which is the venue publishing each value to six decimals.
    ///
    /// The cap is not one number per market, which is why this takes the cap
    /// rather than the market. `meta` publishes `marginTables`, and 36 of the
    /// 232 markets step their cap down above a notional bound — BTC is 40x
    /// under $150M and 20x over, ETH 25x under $100M and 15x over. An asset's
    /// `maxLeverage` is its first tier's cap, and that is the figure
    /// `SymbolRow` carries, so what this returns is the rule for a position
    /// that fits in that tier. Whether maintenance follows the cap down a tier
    /// is *not* settled here: the largest position on the account checked was
    /// a tenth of its market's bound, so the sum above agrees with both
    /// readings and chose between them for neither.
    pub fn hyperliquid(max_leverage: f64) -> Self {
        Self {
            maintenance: if max_leverage > 0.0 {
                1.0 / (2.0 * max_leverage)
            } else {
                0.0
            },
            // Not the input: a market that states a nonsense cap has stated no
            // cap, and this is the figure a typed leverage is held to.
            leverage: max_leverage.max(0.0),
        }
    }

    /// Lighter states both fractions directly, in basis points of a
    /// position's value: bitcoin is 200 initial — a 50x cap — and 120
    /// maintenance.
    ///
    /// The two are read separately rather than one derived from the other,
    /// because Lighter's maintenance is not half its initial: 120 against
    /// 200. Hyperliquid's rule applied here would read 1/(2·50) = 0.0100
    /// against a real 0.0120, understating what the engine holds — and a
    /// requirement read low puts the cliff further away than it is, the one
    /// direction a risk number must never be wrong in.
    pub fn lighter(maintenance_bps: f64, initial_bps: f64) -> Self {
        Self {
            maintenance: if maintenance_bps > 0.0 {
                maintenance_bps / 10_000.0
            } else {
                0.0
            },
            leverage: if initial_bps > 0.0 {
                10_000.0 / initial_bps
            } else {
                0.0
            },
        }
    }
}

/// Which side crossed the spread, from Hyperliquid's encoding: `B` took the
/// offer and `A` hit the bid, the same two letters for a public print and for
/// this account's own fill.
pub fn hyperliquid_buy(side: &str) -> bool {
    side == "B"
}

/// The same reading from Lighter, which names the two orders a trade is made
/// of rather than the side that crossed. A trade has one bid and one ask and
/// one of them was resting, so the maker being the ask makes the taker the
/// bid: the aggressor bought.
pub fn lighter_buy(is_maker_ask: bool) -> bool {
    is_maker_ask
}

/// A position's signed size, which is the number every risk figure on the row
/// reads its direction from. Hyperliquid signs it (`szi`); Lighter reports
/// the magnitude beside a `sign` of +1 or -1, so a short dropped through
/// unsigned reads as a long and puts the cliff on the wrong side of the entry.
pub fn lighter_size(sign: i64, size: f64) -> f64 {
    if sign < 0 { -size.abs() } else { size.abs() }
}

/// Yesterday's close, which `SymbolRow` carries so a streamed mid can be
/// turned back into a 24h change without another request.
///
/// Hyperliquid publishes it (`prevDayPx`) and Lighter publishes the move
/// instead (`daily_price_change`, already a percentage), so it is recovered
/// from the price that move ended at. Without it Lighter's whole change
/// column would read `+0.00%` the moment the feed replaced the day's figures:
/// the fold divides by this, and a zero divisor reads as no move at all.
///
/// A market that has lost all of its value has no close left to divide by.
pub fn previous_close(price: f64, change_pct: f64) -> f64 {
    let factor = 1.0 + change_pct / 100.0;
    if factor > 0.0 { price / factor } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperliquid::hl_symbols;

    /// The account `lighter.rs` established its own units against, so a
    /// failure here is the seam rather than the address.
    const LIGHTER_ACCOUNT: &str = "0x3f4ec7684F679F83c782e485b358A2D43045d6A2";

    /// A venue is two strings and they are not the same string: one is read
    /// and one is stored.
    #[test]
    fn a_venue_is_named_for_the_screen_and_identified_for_everything_else() {
        assert_eq!(Venue::Hyperliquid.name(), "Hyperliquid");
        assert_eq!(Venue::Hyperliquid.id(), "hyperliquid");
        assert_eq!(Venue::Lighter.name(), "Lighter");
        assert_eq!(Venue::Lighter.id(), "lighter");

        let mut ids: Vec<&str> = Venue::ALL.iter().map(|venue| venue.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Venue::ALL.len(), "two venues sharing one id");
    }

    /// Half the margin at the cap the market states, which is the whole of the
    /// rule for a position inside the market's first tier.
    #[test]
    fn hyperliquid_maintains_at_half_the_margin_of_its_cap() {
        // A 40x market: margin is 1/40th of the position, half of that is
        // 1/80th.
        assert_eq!(MarginRule::hyperliquid(40.0).maintenance, 0.0125);
        assert_eq!(MarginRule::hyperliquid(40.0).leverage, 40.0);
        // A 10x market maintains at 1/20th, a 1x market at half.
        assert_eq!(MarginRule::hyperliquid(10.0).maintenance, 0.05);
        assert_eq!(MarginRule::hyperliquid(1.0).maintenance, 0.5);
    }

    /// Lighter states both fractions and they are not each other's half, so
    /// the maintenance is read rather than derived.
    #[test]
    fn lighter_states_its_fractions_in_basis_points() {
        // Bitcoin, read from the live universe: 200 initial, 120 maintenance.
        let btc = MarginRule::lighter(120.0, 200.0);
        assert_eq!(btc.maintenance, 0.012);
        assert_eq!(btc.leverage, 50.0);
        // A 3333 initial is a 3x market; 2000 maintenance is a fifth.
        assert_eq!(MarginRule::lighter(2_000.0, 3_333.0).maintenance, 0.2);

        // Hyperliquid's rule at the same cap would hold a bitcoin position to
        // 1.00% where Lighter holds it to 1.20% — low, which is the direction
        // that draws the cliff further off than it is.
        let borrowed = MarginRule::hyperliquid(btc.leverage);
        assert_eq!(borrowed.maintenance, 0.01);
        assert!(
            borrowed.maintenance < btc.maintenance,
            "one venue's rule must not be assumed for the other"
        );
    }

    /// The agreement the two constructors have to keep: a figure the venue did
    /// not state reads zero, on its own, and never lends its absence to the
    /// other figure or borrows a value from it.
    #[test]
    fn an_unstated_figure_reads_as_zero_on_both_venues() {
        let unknown = MarginRule {
            maintenance: 0.0,
            leverage: 0.0,
        };
        assert_eq!(MarginRule::hyperliquid(0.0), unknown);
        assert_eq!(MarginRule::lighter(0.0, 0.0), unknown);
        // A cap that cannot be one is no cap stated, not a short position's.
        assert_eq!(MarginRule::hyperliquid(-40.0), unknown);
        assert_eq!(MarginRule::lighter(-120.0, -200.0), unknown);

        // Lighter states the two separately, so one can arrive without the
        // other — and the missing one stays missing rather than being derived
        // from the one that came.
        let no_cap = MarginRule::lighter(120.0, 0.0);
        assert_eq!(no_cap.maintenance, 0.012);
        assert_eq!(no_cap.leverage, 0.0);
        let no_requirement = MarginRule::lighter(0.0, 200.0);
        assert_eq!(no_requirement.maintenance, 0.0);
        assert_eq!(no_requirement.leverage, 50.0);
    }

    /// The point of the fraction: two encodings, one number, and the panels
    /// cannot tell which venue it came from.
    #[test]
    fn one_requirement_reached_from_either_encoding() {
        // A 50x cap on Hyperliquid is 1/100th; 100 basis points on Lighter is
        // the same hundredth.
        assert_eq!(MarginRule::hyperliquid(50.0).maintenance, 0.01);
        assert_eq!(MarginRule::lighter(100.0, 200.0).maintenance, 0.01);
        assert_eq!(
            MarginRule::hyperliquid(50.0).maintenance,
            MarginRule::lighter(100.0, 200.0).maintenance
        );

        // And the cap survives the round trip: 200 basis points of initial
        // margin is the 50x that produced it.
        assert_eq!(MarginRule::lighter(100.0, 200.0).leverage, 50.0);
    }

    /// A tape read backwards is every row on it wrong, and the two venues
    /// encode the aggressor differently.
    #[test]
    fn the_side_that_crossed_reads_the_same_on_both() {
        assert!(hyperliquid_buy("B"));
        assert!(!hyperliquid_buy("A"));
        // Anything else is not a buy: a missing side must not read as one.
        assert!(!hyperliquid_buy(""));
        assert!(!hyperliquid_buy("b"));

        assert!(lighter_buy(true));
        assert!(!lighter_buy(false));
        assert_eq!(hyperliquid_buy("B"), lighter_buy(true));
        assert_eq!(hyperliquid_buy("A"), lighter_buy(false));
    }

    /// Both live readings, from one Lighter account: long bitcoin, short sui.
    #[test]
    fn a_short_is_signed_whichever_venue_reported_it() {
        assert_eq!(lighter_size(1, 3.22113), 3.22113);
        assert_eq!(lighter_size(-1, 1382.1), -1382.1);
        // The magnitude is a magnitude; the sign is the only thing that says
        // which way the position runs.
        assert_eq!(lighter_size(-1, -1382.1), -1382.1);
        assert_eq!(lighter_size(0, 3.22113), 3.22113);
    }

    /// The close is what the change was measured from, so measuring the
    /// change back out of it has to return what the venue published.
    #[test]
    fn yesterdays_close_is_recovered_from_the_move() {
        // Up five percent from 100 lands on 105.
        assert_eq!(previous_close(105.0, 5.0), 100.0);
        // Down four percent from 100 lands on 96.
        assert_eq!(previous_close(96.0, -4.0), 100.0);
        assert_eq!(previous_close(100.0, 0.0), 100.0);

        // Bitcoin as Lighter published it: 64,973.9 after +0.609815584…%.
        let close = previous_close(64_973.9, 0.609815584210453);
        let back = (64_973.9 - close) / close * 100.0;
        assert!(
            (back - 0.609815584210453).abs() < 1e-9,
            "the change does not come back out of the close: {back}"
        );

        // Nothing left to have moved from.
        assert_eq!(previous_close(100.0, -100.0), 0.0);
        assert_eq!(previous_close(100.0, -150.0), 0.0);
    }

    /// The rule stated here has to be the rule the venue's own parser already
    /// applies, or the neutral type is a second opinion. Held against what
    /// `parse_symbols` makes of a live `metaAndAssetCtxs` rather than against a
    /// fixture, because a fixture agreeing with the rule that built it proves
    /// only that the fixture was typed carefully.
    ///
    /// Live, so it fails on a train rather than on a bug.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the whole universe against the rule"]
    fn the_rule_agrees_with_what_hyperliquid_already_parses() {
        let rows = smol::block_on(hl_symbols()).expect("the universe");
        assert!(rows.len() > 100, "the venue lists a couple hundred markets");
        for row in rows {
            assert_eq!(
                MarginRule::hyperliquid(row.leverage),
                MarginRule {
                    maintenance: row.maintenance,
                    leverage: row.leverage,
                },
                "{}: the neutral rule disagrees with the venue's own row",
                row.name
            );
        }
    }

    /// The seam carrying real traffic, which is the only thing that shows the
    /// three reads are the three a venue can actually answer. Lighter's own
    /// parsers are private to its module, so there is no offline half of this:
    /// what the compiler checks is that the adapter coerces into `Reads`, and
    /// what this checks is that asking through it returns a drawable answer.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the Lighter seam end to end"]
    fn the_lighter_reads_answer_through_the_seam() {
        let reads = Reads::LIGHTER;
        assert_eq!(reads.venue.id(), "lighter");
        smol::block_on(async {
            let rows = (reads.markets)().await.expect("markets");
            assert!(rows.len() > 100, "the venue lists a couple hundred markets");
            let top = &rows[0];
            assert!(top.price > 0.0 && top.maintenance > 0.0 && top.leverage > 0.0);
            // The neutral rule has to survive the round trip through the venue
            // that states it in basis points: what the adapter parsed is the
            // maintenance a fraction of that cap would be reached from.
            assert_eq!(
                MarginRule::lighter(top.maintenance * 10_000.0, 10_000.0 / top.leverage),
                MarginRule {
                    maintenance: top.maintenance,
                    leverage: top.leverage,
                },
            );

            // Keyed by the ticker the markets read carries, which is the whole
            // claim the seam makes about naming a market.
            let book = (reads.book)(top.name.clone()).await.expect("book");
            assert!(!book.bids.is_empty() && !book.asks.is_empty());
            assert!(book.asks[0].price > book.bids[0].price, "crossed book");

            let account = (reads.account)(LIGHTER_ACCOUNT.to_owned())
                .await
                .expect("account");
            assert!(account.value > 0.0);
        });
    }
}
