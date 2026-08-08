//! What a venue owes the terminal, so one screen can show two exchanges.
//!
//! The panels, the folds, the formatters, the ticket's arithmetic and the
//! chart adapter never learn whose exchange they are drawing. What does is
//! short: the reads listed on `Reads`, the sentences a venue owes when it
//! cannot answer one of them, and the handful of figures the two venues state
//! in different units. Those live here rather than in either adapter, because
//! a conversion that lives with one venue is a rule the other one has to know
//! about.
//!
//! Nothing here signs or sends. The boundary is the same one the ticket stops
//! at: everything up to the signature is arithmetic worth having.

// The neutral readings below — the margin rule, the aggressor, the signed
// size, yesterday's close — are the vocabulary this seam publishes, and each
// adapter still carries its own copy of the one it needs. They are read by
// this module's tests and by `lighter_buy`'s caller, and the rest go live with
// the module split that moves the shapes out of `hyperliquid.rs`.
#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;

use smol::channel::Receiver;

use crate::Venue;
use crate::hyperliquid::{
    Account, Fill, HlError, MarketTick, Order, SymbolRow, Tape, hl_account, hl_candles,
    hl_fill_feed, hl_history, hl_market_feed, hl_orders, hl_symbols,
};
use crate::lighter::{lighter_account, lighter_market_feed, lighter_symbols};

/// What the screen calls a venue.
///
/// `Venue` itself is declared in Ice, because the app holds which one is on
/// screen as state and Ice owns state. That leaves the enum with no methods to
/// hang this on, which is the right way round: what a venue is called is a
/// sentence for a reader, and every other sentence here is a free function too.
pub fn venue_name(venue: Venue) -> String {
    match venue {
        Venue::Hyperliquid => "Hyperliquid".to_owned(),
        Venue::Lighter => "Lighter".to_owned(),
    }
}

/// What a reader hears on the switch, by the rule the page and interval tabs
/// already follow: the button is named for the act, and the one already taken
/// says so in its name rather than in its colour.
pub fn venue_label(venue: Venue, shown: bool) -> String {
    let state = if shown { ", already reading" } else { "" };
    format!("Read {}{state}", venue_name(venue))
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

/// A stream that ended before it started: what a venue answers when it will
/// not carry a channel at all. The sender is dropped here, so the receiver is
/// closed on its first read and the app's `stream` finishes without a message
/// — which is not an error, and must not be drawn as one. What the panel
/// draws instead is the sentence beside it.
fn no_stream<T>() -> Receiver<Result<T, HlError>> {
    let (_sender, receiver) = smol::channel::bounded(1);
    receiver
}

/// Everything the terminal asks a venue for. Read-only, and each answer is
/// already in the shape the panels read rather than the shape the exchange
/// returned — the field map is the adapter's whole job.
///
/// This is the list the app actually asks, one field per `venue_*` extern, so
/// a venue that cannot answer one of them has to say so here rather than
/// somewhere a handler would have to know about. There is no `book`: both
/// venues publish theirs on the socket, and a REST book would be a read the
/// app never makes.
///
/// Markets are named by their ticker, which is what every panel holds.
/// Hyperliquid keys its requests by that ticker too; Lighter keys the book by
/// a numeric `market_id`, so its adapter carries the ticker-to-id table it
/// already read out of the universe. That is a lookup, not a second
/// identifier for the app to carry.
#[derive(Clone, Copy)]
pub struct Reads {
    /// Which exchange answers these.
    pub venue: Venue,
    /// The tradeable universe: one row per market, with the day's figures and
    /// the margin rule that market holds a position to.
    pub markets: fn() -> Fetch<Vec<SymbolRow>>,
    /// The window of candles a chart opens on, merged into the tape in place.
    pub candles: fn(Tape, String, String) -> Fetch<i64>,
    /// The window before the tape's oldest bar, for a chart panned back that
    /// far. Answers the tape's length, unchanged when there is no more.
    pub history: fn(Tape, String, String) -> Fetch<i64>,
    /// One account, by the address that owns it — equity, what the margin
    /// engine holds against it, and every open position — or nothing when the
    /// address has no account at this venue.
    ///
    /// Nothing is an answer, and it is the answer this seam exists to keep
    /// separate from a failure: an address is typed once and read at whichever
    /// venue is on screen, so having an account at one exchange and none at
    /// the other is the ordinary case rather than a broken read.
    pub account: fn(String) -> Fetch<Option<Account>>,
    /// That account's resting orders.
    pub orders: fn(String) -> Fetch<Vec<Order>>,
    /// Mids, book, prints and candles for whatever market the tape is pointed
    /// at, coalesced to one beat.
    pub market_feed: fn(Tape) -> Receiver<Result<MarketTick, HlError>>,
    /// That account's fills as they print.
    pub fill_feed: fn(String) -> Receiver<Result<Vec<Fill>, HlError>>,
}

impl Reads {
    /// The venue on screen. Returned by value because every field is a
    /// pointer: choosing a venue costs a copy of seven of them and no
    /// allocation.
    pub fn of(venue: Venue) -> Reads {
        match venue {
            Venue::Hyperliquid => Reads::HYPERLIQUID,
            Venue::Lighter => Reads::LIGHTER,
        }
    }

    /// Hyperliquid, which answers all seven.
    pub const HYPERLIQUID: Reads = Reads {
        venue: Venue::Hyperliquid,
        markets: || Box::pin(hl_symbols()),
        candles: |tape, coin, interval| Box::pin(hl_candles(tape, coin, interval)),
        history: |tape, coin, interval| Box::pin(hl_history(tape, coin, interval)),
        // Always an account: an address that has never traded here reads back
        // a zeroed `clearinghouseState` rather than a refusal, so the venue
        // draws no line between an empty account and an absent one and neither
        // does this.
        account: |address| Box::pin(async move { hl_account(address).await.map(Some) }),
        orders: |address| Box::pin(hl_orders(address)),
        market_feed: hl_market_feed,
        fill_feed: hl_fill_feed,
    };

    /// Lighter, wired to what its adapter publishes and stating the four it
    /// cannot serve. The closures are here rather than in `lighter.rs` because
    /// the box is this seam's cost, not the adapter's: an adapter that
    /// returned a boxed future would pay for it even when called directly.
    ///
    /// A gap answers empty rather than failing, because a venue that does not
    /// carry a channel has not broken: an error here would raise the app's
    /// alarm line over something that is working exactly as documented. What
    /// tells the reader is `venue_orders_note`, `venue_fills_note` and
    /// `venue_chart_note`, in the panel that would otherwise be blank.
    pub const LIGHTER: Reads = Reads {
        venue: Venue::Lighter,
        markets: || Box::pin(lighter_symbols()),
        // No candle history exists to ask for: `/candlesticks` answers 403
        // from CloudFront, and `candle/<id>/<res>` answers a subscription with
        // the one bar now forming on all eight resolutions it quotes. So the
        // tape fills forward off the feed alone, from nothing.
        candles: |_tape, _coin, _interval| Box::pin(async { Ok(0) }),
        history: |_tape, _coin, _interval| Box::pin(async { Ok(0) }),
        account: |address| Box::pin(lighter_account(address)),
        // `account_all/<index>` and the order channels are keyed by account
        // index rather than L1 address, and want an API-key-signed token an
        // address alone cannot get (`code 20001`). An address is all this app
        // asks a reader for, so there are no orders and no fills here.
        orders: |_address| Box::pin(async { Ok(Vec::new()) }),
        market_feed: lighter_market_feed,
        fill_feed: |_address| no_stream(),
    };
}

// The public tape is not a read on this seam at all: both venues' prints
// arrive on their websocket rather than through a function to point at, and
// both feeds already fold them into the same `Trade`. Whoever lands a REST
// tape inherits an ordering question that is settled and was once documented
// backwards: both
// endpoints answer newest first. `{"type":"recentTrades","coin":"BTC"}` came
// back with `time` non-increasing across every print, and Lighter's
// `recentTrades?market_id=1` with `timestamp` and `trade_id` both descending.
// That is already the order the app holds a tape in — `push_trades` reverses a
// websocket beat to put its newest print on top — so the read wants no reverse.

/// Every read the app makes, with the venue it is making it of. One function
/// per operation rather than one per venue: Ice cannot pick a function at the
/// call site, so the choice is made here, and a handler names the operation
/// and hands over the venue it is holding.
pub async fn venue_symbols(venue: Venue) -> Result<Vec<SymbolRow>, HlError> {
    (Reads::of(venue).markets)().await
}

pub async fn venue_candles(
    venue: Venue,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    (Reads::of(venue).candles)(tape, coin, interval).await
}

pub async fn venue_history(
    venue: Venue,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    (Reads::of(venue).history)(tape, coin, interval).await
}

/// The three account reads share one rule, and it is here rather than in the
/// handlers because a handler cannot hold it: a task group has to be a
/// handler's last statement, so a read guarded in Ice would have to be a second
/// copy of the whole group. No address is not a failure and not an empty
/// account — it is a read the app did not make, and each of these says so in
/// the shape its own answer already has.
pub async fn venue_account(venue: Venue, address: String) -> Result<Option<Account>, HlError> {
    if address.trim().is_empty() {
        return Ok(None);
    }
    (Reads::of(venue).account)(address).await
}

pub async fn venue_orders(venue: Venue, address: String) -> Result<Vec<Order>, HlError> {
    if address.trim().is_empty() {
        return Ok(Vec::new());
    }
    (Reads::of(venue).orders)(address).await
}

pub fn venue_market_feed(venue: Venue, tape: Tape) -> Receiver<Result<MarketTick, HlError>> {
    (Reads::of(venue).market_feed)(tape)
}

/// The same rule on the socket. A fills subscription names the account it is
/// for, and Hyperliquid answers an empty user with a rejected subscription —
/// which the feed reports as a dropped connection and retries forever.
pub fn venue_fill_feed(venue: Venue, address: String) -> Receiver<Result<Vec<Fill>, HlError>> {
    if address.trim().is_empty() {
        return no_stream();
    }
    (Reads::of(venue).fill_feed)(address)
}

/// What a venue will not tell this app about the account it is watching, or
/// nothing when it answers everything asked of it.
///
/// One fact, stated once: `orders` and `fill_feed` are empty on Lighter for the
/// same reason, so the two panels below read their emptiness out of this rather
/// than each carrying its own opinion of why.
pub fn venue_account_gap(venue: Venue) -> String {
    match venue {
        Venue::Hyperliquid => String::new(),
        Venue::Lighter => {
            "Lighter serves resting orders and this account's fills only to an API-key-signed \
             token, which an address alone cannot get and this app does not hold."
                .to_owned()
        }
    }
}

/// What the account strip says when there is no account to draw.
///
/// Two facts share that empty state and only one of them is about this app:
/// with no address there is nothing to read, and with an address there is
/// nothing at this venue to find. "Settings takes an address" is the first,
/// and said over the second it sends a reader to re-enter the address that is
/// already there. One address is typed once and read at whichever venue is on
/// screen, so holding a book at one exchange and none at the other is ordinary
/// — and it is the venue rather than the address that makes it so, which is
/// why the sentence names the venue.
pub fn venue_account_note(venue: Venue, watching: bool) -> String {
    if watching {
        format!("No {} account for this address.", venue_name(venue))
    } else {
        "No account is being read. Settings takes an address.".to_owned()
    }
}

/// What the resting-orders panel says when it has no rows to draw.
///
/// An empty list reads as "nothing has happened", which on a venue that will
/// not answer is a lie: nothing can happen. The sentence is the panel's empty
/// state rather than a banner so that it lands where the reader is already
/// looking for the rows.
pub fn venue_orders_note(venue: Venue, watching: bool) -> String {
    match venue_account_gap(venue) {
        gap if !gap.is_empty() => gap,
        _ if watching => "No resting orders.".to_owned(),
        _ => "Orders need an address.".to_owned(),
    }
}

/// The same for fills. An address is what separates the two things a venue
/// that does answer can say; a venue that does not answer says so either way,
/// because connecting an address would not change it.
pub fn venue_fills_note(venue: Venue, watching: bool) -> String {
    match venue_account_gap(venue) {
        gap if !gap.is_empty() => gap,
        _ if watching => "No fills on this account yet.".to_owned(),
        _ => "Fills need an address.".to_owned(),
    }
}

/// What the chart owes the reader about the bars it is drawing, or nothing
/// when the venue backfills like a chart is expected to.
///
/// Lighter's chart opens empty and gains one bar per interval. Left unsaid,
/// a one-bar chart reads as a market that has not traded.
pub fn venue_chart_note(venue: Venue) -> String {
    match venue {
        Venue::Hyperliquid => String::new(),
        Venue::Lighter => {
            "Lighter publishes no candle history — this chart starts empty and gains a bar per \
             interval."
                .to_owned()
        }
    }
}

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
    // The book is not on `Reads` — the app never asks a venue for one — so the
    // live seam test reaches the adapter's own read directly.
    use crate::lighter::lighter_book;

    /// The account `lighter.rs` established its own units against, so a
    /// failure here is the seam rather than the address.
    const LIGHTER_ACCOUNT: &str = "0x3f4ec7684F679F83c782e485b358A2D43045d6A2";

    /// The two venues the switch offers, so a loop below can assert something
    /// of both without naming them twice.
    const BOTH: [Venue; 2] = [Venue::Hyperliquid, Venue::Lighter];

    /// A switch that says which venue is being read only in its highlight
    /// colour says it to whoever can see two inks. The name a reader hears
    /// carries the state, and it is a different sentence for each venue and
    /// for each of the two states.
    #[test]
    fn the_switch_names_the_venue_and_says_which_one_is_being_read() {
        assert_eq!(venue_name(Venue::Hyperliquid), "Hyperliquid");
        assert_eq!(venue_name(Venue::Lighter), "Lighter");

        assert_eq!(venue_label(Venue::Lighter, false), "Read Lighter");
        assert_eq!(
            venue_label(Venue::Lighter, true),
            "Read Lighter, already reading"
        );

        let mut heard: Vec<String> = BOTH
            .iter()
            .flat_map(|venue| [venue_label(*venue, false), venue_label(*venue, true)])
            .collect();
        assert!(
            heard.iter().all(
                |label| label.contains(&venue_name(if label.contains("Lighter") {
                    Venue::Lighter
                } else {
                    Venue::Hyperliquid
                }))
            ),
            "a label has to name the venue it switches to"
        );
        heard.sort();
        heard.dedup();
        assert_eq!(heard.len(), 4, "two tabs sounding alike say nothing");
    }

    /// A venue that cannot answer a read owes the panel a sentence, and the
    /// sentence has to name the venue and the reason: "no resting orders" and
    /// "this venue will not tell you" are different facts and one of them is
    /// the empty list's own default.
    #[test]
    fn a_venue_that_cannot_answer_says_so_where_the_rows_would_be() {
        // Connecting an address would not change what Lighter will answer, so
        // both panels say the same thing whether or not one is connected.
        let gap = venue_account_gap(Venue::Lighter);
        assert!(gap.contains("Lighter") && gap.contains("API-key"), "{gap}");
        for watching in [false, true] {
            assert_eq!(venue_orders_note(Venue::Lighter, watching), gap);
            assert_eq!(venue_fills_note(Venue::Lighter, watching), gap);
        }
        assert!(venue_account_gap(Venue::Hyperliquid).is_empty());
        // Hyperliquid answers both, so its panels say only what the account
        // holds — and an address is what separates the two things it can say.
        assert_eq!(
            venue_orders_note(Venue::Hyperliquid, true),
            "No resting orders."
        );
        assert_eq!(
            venue_orders_note(Venue::Hyperliquid, false),
            "Orders need an address."
        );
        assert_eq!(
            venue_fills_note(Venue::Hyperliquid, true),
            "No fills on this account yet."
        );
        assert_eq!(
            venue_fills_note(Venue::Hyperliquid, false),
            "Fills need an address."
        );

        // The chart's gap is a venue's fact rather than an account's, so it
        // takes no `watching` and it is empty on the venue that backfills.
        assert!(venue_chart_note(Venue::Hyperliquid).is_empty());
        assert!(venue_chart_note(Venue::Lighter).contains("no candle history"));
    }

    /// No account is two different facts and the strip has one line to say
    /// which. Sending a reader who has connected an address to Settings to
    /// enter the address that is already there is the app blaming them for the
    /// venue's answer.
    #[test]
    fn no_account_says_whether_there_is_an_address_or_only_no_account_here() {
        for venue in BOTH {
            let missing = venue_account_note(venue, true);
            assert!(
                missing.contains(&venue_name(venue)),
                "the absence is this venue's, so it has to name it: {missing}"
            );
            assert!(
                !missing.contains("Settings"),
                "the address is already connected: {missing}"
            );
            assert_eq!(
                venue_account_note(venue, false),
                "No account is being read. Settings takes an address.",
                "with no address it is the app that has nothing, not the venue"
            );
        }
        // And the two venues do not sound alike, because which one answered
        // nothing is the useful half.
        assert_ne!(
            venue_account_note(Venue::Hyperliquid, true),
            venue_account_note(Venue::Lighter, true)
        );
    }

    /// A gap is not a failure. Both are drawn, and they are drawn in different
    /// places by different rules: an error raises the app's alarm line, a gap
    /// is the empty state of one panel. A venue answering `Err` for something
    /// it was never going to carry would put "Lighter unreachable" over a
    /// working screen.
    #[test]
    fn a_gap_answers_empty_rather_than_failing() {
        let lighter = Reads::of(Venue::Lighter);
        smol::block_on(async {
            let tape = crate::hyperliquid::tape_new();
            let bars = (lighter.candles)(tape.clone(), "BTC".to_owned(), "1m".to_owned())
                .await
                .expect("a gap is not a failure");
            assert_eq!(bars, 0);
            let older = (lighter.history)(tape, "BTC".to_owned(), "1m".to_owned())
                .await
                .expect("a gap is not a failure");
            assert_eq!(older, 0);
            let resting = (lighter.orders)(LIGHTER_ACCOUNT.to_owned())
                .await
                .expect("a gap is not a failure");
            assert!(resting.is_empty());
        });
        // And a stream it will not carry ends rather than staying open with
        // nothing on it, so the app is not left waiting on a live feed.
        assert!(
            venue_fill_feed(Venue::Lighter, LIGHTER_ACCOUNT.to_owned())
                .recv_blocking()
                .is_err(),
            "a refused feed must close, not hang"
        );
    }

    /// Browsing has no address, and a fills subscription with an empty user is
    /// rejected by the exchange — which the feed reads as a dropped connection
    /// and retries forever. Refused before it reaches either venue.
    #[test]
    fn a_fill_feed_without_an_address_is_refused_before_the_wire() {
        for venue in BOTH {
            for address in ["", "   "] {
                assert!(
                    venue_fill_feed(venue, address.to_owned())
                        .recv_blocking()
                        .is_err(),
                    "an addressless fill feed must not open a socket"
                );
            }
        }
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
        crate::hyperliquid::open_the_wire();
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
    /// reads are ones a venue can actually answer. Lighter's own parsers are
    /// private to its module, so there is no offline half of this: what the
    /// compiler checks is that the adapter coerces into `Reads`, and what this
    /// checks is that asking through it returns a drawable answer.
    ///
    /// The book is read directly rather than through `Reads`, because the app
    /// never asks a venue for one — both publish theirs on the socket — and a
    /// field the app does not read would be a claim about the seam that the
    /// seam does not make.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the Lighter seam end to end"]
    fn the_lighter_reads_answer_through_the_seam() {
        crate::hyperliquid::open_the_wire();
        let reads = Reads::of(Venue::Lighter);
        assert_eq!(venue_name(reads.venue), "Lighter");
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
            let book = lighter_book(top.name.clone()).await.expect("book");
            assert!(!book.bids.is_empty() && !book.asks.is_empty());
            assert!(book.asks[0].price > book.bids[0].price, "crossed book");

            let account = (reads.account)(LIGHTER_ACCOUNT.to_owned())
                .await
                .expect("account")
                .expect("an account this venue holds");
            assert!(account.value > 0.0);
        });
    }

    /// One address, two venues, and it does not have to exist at both. The
    /// seam has to carry that back as an answer, because the alternative it
    /// used to carry — a failure — raises the app's alarm line over a screen
    /// that is working exactly as it should.
    ///
    /// Live, because the whole claim is about what the venues answer. Both
    /// directions are run: an address with a Hyperliquid account and no
    /// Lighter one, and the Lighter account itself, so "always nothing" cannot
    /// pass.
    #[test]
    #[ignore = "hits both live venues, run explicitly: one address at two venues"]
    fn an_address_with_no_account_at_a_venue_is_absent_rather_than_broken() {
        crate::hyperliquid::open_the_wire();
        smol::block_on(async {
            let held = venue_account(Venue::Lighter, LIGHTER_ACCOUNT.to_owned())
                .await
                .expect("a read that answered");
            assert!(held.is_some_and(|account| account.value > 0.0));

            // The other venue's demo address, which this app's own fixtures
            // are drawn for there: Lighter answers `21100 account not found`.
            let elsewhere = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a".to_owned();
            assert!(
                venue_account(Venue::Lighter, elsewhere.clone())
                    .await
                    .expect("a refusal that is an answer is not an error")
                    .is_none(),
                "no account at this venue is not a failed read"
            );
            // And it is that venue's answer rather than the address being
            // unreadable: the same address is an account on Hyperliquid.
            assert!(
                venue_account(Venue::Hyperliquid, elsewhere)
                    .await
                    .expect("the account")
                    .is_some_and(|account| account.value > 0.0)
            );
        });
    }
}
