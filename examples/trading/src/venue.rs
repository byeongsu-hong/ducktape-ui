//! What a network owes the terminal, so one screen can show several of them.
//!
//! The panels, the folds, the formatters, the ticket's arithmetic and the
//! chart adapter never learn whose exchange they are drawing. What does is
//! `NETWORKS`: one entry per network, carrying its name, whether it is a test
//! deployment, the reads it answers, the sentence it owes when it cannot
//! answer one of them, and the handful of figures the exchanges state in
//! different units. Those live here rather than in either adapter, because a
//! conversion that lives with one venue is a rule the other one has to know
//! about.
//!
//! A *network* rather than an exchange is the identity because one exchange
//! can have more than one deployment. Hyperliquid's testnet is the same
//! protocol, the same parser and a different pair of endpoints, and the app
//! must never hold "which exchange" and "which deployment" as two values that
//! can disagree — a mainnet book pricing a testnet order looks entirely
//! right on both halves. So a network is one value, every read on it is
//! pinned to a `Chain`, and a signature is pinned to the same `Chain`.
//!
//! Adding one is an entry in `NETWORKS`, the `Venue` variant it names, and the
//! arm `Network::of` will not compile without.

// The neutral readings below — the margin rule, the aggressor, the signed
// size, yesterday's close — are the vocabulary this seam publishes, and each
// adapter still carries its own copy of the one it needs. They are read by
// this module's tests and by `lighter_buy`'s caller, and the rest go live with
// the module split that moves the shapes out of `hyperliquid.rs`.
#![allow(dead_code)]

use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use smol::channel::Receiver;

use crate::hyperliquid::{
    Account, Fill, HL_CANONICAL, HlError, MarketTick, Order, Position, SymbolRow, Tape, Ticket,
    amount, fmt_px, fmt_size, hl_account, hl_candles, hl_fill_feed, hl_history, hl_market_feed,
    hl_orders, hl_symbols, order_label,
};
use crate::lighter::{
    Zone, lighter_account, lighter_candles, lighter_history, lighter_market_feed, lighter_symbols,
};
use crate::portfolio::{PortfolioHistory, hl_portfolio, portfolio_unavailable};
use crate::signing::Chain;
use crate::{Tif, Venue};

/// What the screen calls a venue.
///
/// `Venue` itself is declared in Ice, because the app holds which one is on
/// screen as state and Ice owns state. That leaves the enum with no methods to
/// hang this on, which is the right way round: what a venue is called is a
/// sentence for a reader, and every other sentence here is a free function too.
pub fn venue_name(venue: Venue) -> String {
    Network::of(venue).name.to_owned()
}

/// Whether the network on screen trades money that is worth something.
///
/// The one fact on this seam that is not decoration. Everything else a network
/// carries changes what is drawn; this changes whether a mistake costs
/// anything, so it is read by the badge beside the venue, by the ticket, and
/// by the confirmation an order passes through — never inferred from the name.
pub fn venue_testnet(venue: Venue) -> bool {
    Network::of(venue).testnet
}

/// What kind of network this is, in the two words a reader has to be sure of
/// before they press anything.
///
/// Both are stated. A badge that appears only on testnet is a badge whose
/// absence has to be noticed, and nobody notices an absence — so the network
/// that can lose money says so in the same place, in the same shape, and the
/// reader learns where to look once rather than learning it the day it matters.
pub fn venue_kind(venue: Venue) -> String {
    if Network::of(venue).testnet {
        "TESTNET".to_owned()
    } else {
        "REAL MONEY".to_owned()
    }
}

/// Every network the app can point at, in the order the picker lists them.
///
/// The picker reads this rather than naming its entries, so a network added to
/// `NETWORKS` appears in the header without the view being touched. Real money
/// first, because that is the order a reader expects to find them in and the
/// order that puts the test networks where a deliberate choice reaches them.
pub fn venue_list() -> Vec<Venue> {
    NETWORKS.iter().map(|network| network.venue).collect()
}

/// The whole registry, and the only place a network is enumerated.
///
/// Adding one is this array, the `Venue` variant it names, and the arm
/// `Network::of` then refuses to compile without. The exhaustive match is the
/// point rather than an inconvenience: a network whose reads were wired and
/// whose capability sentence was forgotten is a screen that silently claims
/// the wrong exchange answers a panel it will leave empty.
const NETWORKS: [Network; 4] = [
    Network::HYPERLIQUID,
    Network::HYPERLIQUID_TESTNET,
    Network::LIGHTER,
    Network::LIGHTER_TESTNET,
];

/// What a reader hears on the switch, by the rule the page and interval tabs
/// already follow: the button is named for the act, and the one already taken
/// says so in its name rather than in its colour.
///
/// The kind is in here rather than only in the badge beside it. A labelled
/// button's name replaces its contents, so the box reading REAL MONEY inside
/// the row is painted and never spoken — which left the one reader who cannot
/// check the colour hearing four names and no deployments. This is the row a
/// finger is travelling towards; it has to answer "real money or not" before
/// it is pressed, and to every reader.
pub fn venue_label(venue: Venue, shown: bool) -> String {
    let state = if shown { ", already reading" } else { "" };
    format!("Read {}, {}{state}", venue_name(venue), spoken_kind(venue))
}

/// What the control that opens the picker says it is and what it is showing.
///
/// A trigger that only names the network is a label for a readout, and this
/// one is a button: without the act in its name, nothing tells a reader who
/// cannot see the panel drop that the header's venue block is a way to change
/// venue at all.
pub fn venue_switch_label(venue: Venue) -> String {
    format!(
        "{}, {} — switch network",
        venue_name(venue),
        spoken_kind(venue)
    )
}

/// `venue_kind` said aloud. The badge is set in capitals because it is read as
/// a shape at a glance; a screen reader spelling out R-E-A-L is not that.
fn spoken_kind(venue: Venue) -> String {
    venue_kind(venue).to_lowercase()
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
/// message, and which network produced it is on the `Network` the caller asked
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

/// How this app signs for a network, and which deployment that signature is
/// pinned to.
///
/// Two schemes rather than one, because the two exchanges share no part of a
/// signature. Hyperliquid signs EIP-712 typed data with an Ethereum agent key
/// the master wallet approved; Lighter signs L2 transactions with an API key
/// the account registered, Schnorr over a different curve entirely, and its
/// deployment is a number stamped into the digest rather than a domain. What
/// they do share is the one fact this type exists to carry: a signature belongs
/// to exactly one deployment, and it is the same deployment the reads on that
/// network are addressed to.
///
/// So the field on `Network` is this rather than a `Chain`. A `Chain` bent to
/// stand for both would be an EIP-712 domain claiming to describe a scheme that
/// has none, and the entry for a Lighter network would name a Hyperliquid
/// deployment — which is the exact confusion the whole seam is built to
/// prevent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signing {
    /// `signing.rs`, pinned by the chain the phantom agent's `source` names.
    Eip712(Chain),
    /// `lighter_sign.rs`, pinned by the chain id this zone's sequencer stamps
    /// into every transaction digest.
    ApiKey(Zone),
}

impl Signing {
    /// Whether the deployment this signs for trades money that is worth
    /// something. Read beside `Network.testnet`, which has to agree with it.
    pub fn testnet(self) -> bool {
        match self {
            Signing::Eip712(chain) => chain.testnet(),
            Signing::ApiKey(zone) => zone.testnet(),
        }
    }

    /// A short, stable name for what a key held here is for.
    ///
    /// The *scheme* is in the name and not only the deployment, which is the
    /// half a single-venue app never needs. One address is read at both
    /// exchanges and holds a different key at each; an item named for the
    /// deployment alone would file both under "mainnet", so enrolling the
    /// second would overwrite the first and each venue would afterwards be
    /// handed the other's secret. That failure has no symptom until an order is
    /// refused for a signer nobody recognises.
    ///
    /// Deliberately not the wire spelling of anything, for `Chain::key`'s
    /// reason: renaming a keychain item must never be able to change what a
    /// signature says.
    pub fn key(self) -> String {
        match self {
            Signing::Eip712(chain) => format!("hyperliquid-{}", chain.key()),
            Signing::ApiKey(zone) => format!("lighter-{}", zone.key()),
        }
    }
}

/// One network: what it is called, what it costs to be wrong on it, and
/// everything the terminal asks it for. Read-only, and each answer is already
/// in the shape the panels read rather than the shape the exchange returned —
/// the field map is the adapter's whole job.
///
/// This is the list the app actually asks, one field per `venue_*` extern, so
/// a network that cannot answer one of them has to say so here rather than
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
pub struct Network {
    /// Which network answers these. The identity the app holds as state, and
    /// an exchange *and* a deployment rather than an exchange: Hyperliquid's
    /// testnet is the same protocol and the same parser behind a different
    /// pair of endpoints, so a screen that held only the exchange would have
    /// to hold the deployment somewhere else and let the two disagree.
    pub venue: Venue,
    /// What the reader is told they are looking at. A test network says so in
    /// its own name, because the name is what a picker shows and a picker is
    /// where the choice is actually made.
    pub name: &'static str,
    /// Whether an order here costs anything to get wrong.
    pub testnet: bool,
    /// How a write to this network is signed, and the deployment that
    /// signature is pinned to.
    ///
    /// Declared here rather than left inside the read closures because it is
    /// the one fact about a network that two halves of the app have to agree
    /// on: the deployment named here is what the reads are addressed to *and*
    /// what a signature carries, so an entry whose badge says testnet while its
    /// signing says mainnet is a screen that prices an order on one deployment
    /// and sends it to the other. The test below holds the pair together.
    ///
    /// Not an `Option`. Every network in the registry can be written to, so a
    /// "no write path" case would be a state nothing is in — and the day one is
    /// added, a missing scheme should be a compile error here rather than a
    /// silent read-only network that the panels never explain.
    ///
    /// What it cannot hold: each read closure still names its own deployment
    /// literal, because a closure that captured this field would no longer be
    /// the `fn` pointer the seam is built from. A closure naming the wrong one
    /// compiles, and only review and the endpoint in the failure message catch
    /// it.
    pub signing: Signing,
    /// What this network will not tell the app about the account it is
    /// watching, or nothing when it answers everything asked of it. Stated
    /// once here so the panels that go empty read the same reason.
    ///
    /// Strictly a *refusal*: it is drawn where rows would have been, so a
    /// sentence here is a claim that those rows are never coming. Anything
    /// else a reader ought to know about a network is `note`, which is drawn
    /// on settings and nowhere near an empty panel.
    pub gap: &'static str,
    /// What a reader has to know about this network beyond its name, or
    /// nothing when the name says it all.
    ///
    /// Kept apart from `gap` because they are drawn in different places and
    /// only one of them explains a missing row. A test deployment answers
    /// every read this app makes — its panels fill exactly like the live one's
    /// — and putting "this is a test deployment" where "these rows are not
    /// coming" belongs made an empty order book read as a venue that refuses.
    pub note: &'static str,
    /// Whether a resting order here rests until it is cancelled.
    ///
    /// Hyperliquid's `Gtc` does. Lighter has no such thing: its order carries
    /// a deadline it was signed with (`ORDER_TIME_IN_FORCE_GOOD_TILL_TIME`)
    /// and expires there, so a button reading GTC over it would be this app
    /// inventing a guarantee the network never made. On the registry rather
    /// than in a match on the venue, because it is a fact about the exchange
    /// this network deploys and a new entry has to state it.
    pub rests_forever: bool,
    /// Whether a take-profit and a stop-loss can ride on the order that opens
    /// the position.
    ///
    /// Hyperliquid takes them on the same action, as a `trigger` order grouped
    /// with the entry. Lighter's public API has none: its SDK exposes
    /// `create_tp_order` and `create_sl_order` as whole independent orders and
    /// nothing anywhere in it names a parent, so offering the two fields there
    /// would be a promise the app cannot keep — the entry would go and the
    /// protection would not.
    pub attaches_levels: bool,
    /// Whether this network states the time it last charged funding.
    ///
    /// Both exchanges fund hourly, so both boundaries can be counted down to;
    /// what differs is whether the app is told or has to work it out. Lighter
    /// publishes `funding_timestamp` on its `market_stats` channel and the
    /// countdown anchors on it. Hyperliquid publishes none in an asset context,
    /// and the `nextFundingTime` its separate `predictedFundings` request
    /// carries is the boundary already gone by — read at 23:49:06Z on
    /// 2026-08-09 it answered 23:00:00Z — so that network's countdown is
    /// derived from the clock's own hour instead.
    ///
    /// On the registry rather than in a match, because it is a fact about the
    /// exchange this network deploys and a new entry has to state it.
    pub stamps_funding: bool,
    /// This account's realised history over a window, or the sentence saying
    /// why the network will not serve one.
    pub portfolio: fn(String) -> Fetch<PortfolioHistory>,
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

impl Network {
    /// The network on screen. Returned by value because every field is a
    /// pointer or a `&'static str`: choosing one costs a copy of a dozen of
    /// them and no allocation.
    pub fn of(venue: Venue) -> Network {
        match venue {
            Venue::Hyperliquid => Network::HYPERLIQUID,
            Venue::HyperliquidTestnet => Network::HYPERLIQUID_TESTNET,
            Venue::Lighter => Network::LIGHTER,
            Venue::LighterTestnet => Network::LIGHTER_TESTNET,
        }
    }

    /// Hyperliquid's mainnet, which answers all eight.
    pub const HYPERLIQUID: Network = Network {
        venue: Venue::Hyperliquid,
        name: "Hyperliquid",
        rests_forever: true,
        attaches_levels: false,
        stamps_funding: false,
        testnet: false,
        signing: Signing::Eip712(Chain::Mainnet),
        gap: "",
        note: "",
        portfolio: |address| Box::pin(hl_portfolio(Chain::Mainnet, address)),
        // The canonical group is headed with this network's own name, so the
        // rail never says a word the header contradicts.
        markets: || Box::pin(hl_symbols(Chain::Mainnet, HL_CANONICAL)),
        candles: |tape, coin, interval| Box::pin(hl_candles(Chain::Mainnet, tape, coin, interval)),
        history: |tape, coin, interval| Box::pin(hl_history(Chain::Mainnet, tape, coin, interval)),
        // Always an account: an address that has never traded here reads back
        // a zeroed `clearinghouseState` rather than a refusal, so the venue
        // draws no line between an empty account and an absent one and neither
        // does this.
        account: |address| {
            Box::pin(async move { hl_account(Chain::Mainnet, address).await.map(Some) })
        },
        orders: |address| Box::pin(hl_orders(Chain::Mainnet, address)),
        market_feed: |tape| hl_market_feed(Chain::Mainnet, tape),
        fill_feed: |address| hl_fill_feed(Chain::Mainnet, address),
    };

    /// The same exchange's test deployment: the same protocol, the same
    /// parser, and a different pair of endpoints.
    ///
    /// It is a separate entry rather than a flag on the one above because the
    /// two must never be one value the app toggles. Everything on this seam is
    /// pinned to `Chain`, which is also what a signature is pinned to — so a
    /// screen drawn from these endpoints can only be traded by an order
    /// carrying this chain, and the mistake where a mainnet book prices a
    /// testnet order has no way to happen.
    ///
    /// What is different here is real and is not hidden: the deployment holds
    /// its own universe, its own accounts and its own books, so an address
    /// with a mainnet position has nothing here until it is funded on testnet,
    /// and prices are whatever this deployment's traders last agreed on.
    pub const HYPERLIQUID_TESTNET: Network = Network {
        venue: Venue::HyperliquidTestnet,
        rests_forever: true,
        attaches_levels: false,
        stamps_funding: false,
        name: "Hyperliquid Testnet",
        testnet: true,
        signing: Signing::Eip712(Chain::Testnet),
        // It answers every read this app makes, so it refuses nothing and this
        // is empty. What is different about it is a `note`.
        gap: "",
        note: "This is Hyperliquid's test deployment. It answers every read the \
               live one does, and it answers them about its own universe, its own \
               books and its own accounts — so an address funded on mainnet has \
               nothing here until it is funded again here, and nothing traded here \
               is worth anything.",
        portfolio: |address| Box::pin(hl_portfolio(Chain::Testnet, address)),
        markets: || Box::pin(hl_symbols(Chain::Testnet, "Hyperliquid Testnet")),
        candles: |tape, coin, interval| Box::pin(hl_candles(Chain::Testnet, tape, coin, interval)),
        history: |tape, coin, interval| Box::pin(hl_history(Chain::Testnet, tape, coin, interval)),
        account: |address| {
            Box::pin(async move { hl_account(Chain::Testnet, address).await.map(Some) })
        },
        orders: |address| Box::pin(hl_orders(Chain::Testnet, address)),
        market_feed: |tape| hl_market_feed(Chain::Testnet, tape),
        fill_feed: |address| hl_fill_feed(Chain::Testnet, address),
    };

    /// Lighter, wired to what its adapter publishes and stating the two it
    /// cannot serve. The closures are here rather than in `lighter.rs` because
    /// the box is this seam's cost, not the adapter's: an adapter that
    /// returned a boxed future would pay for it even when called directly.
    ///
    /// A gap answers empty rather than failing, because a venue that does not
    /// carry a channel has not broken: an error here would raise the app's
    /// alarm line over something that is working exactly as documented. What
    /// tells the reader is `venue_orders_note` and `venue_fills_note`, in the
    /// panel that would otherwise be blank.
    pub const LIGHTER: Network = Network {
        venue: Venue::Lighter,
        rests_forever: false,
        attaches_levels: false,
        stamps_funding: true,
        name: "Lighter",
        testnet: false,
        // An L2 transaction signed by an API key the account registers, which
        // is the same key and the same `api_key_index` the read token
        // `lighter_sign.rs` mints. The zone rather than a chain: what pins one
        // of these to a deployment is the chain id its sequencer stamps into
        // the digest, not an EIP-712 domain.
        signing: Signing::ApiKey(Zone::Mainnet),
        gap: "Lighter serves resting orders and this account's fills only to an \
              API-key-signed token, which an address alone cannot get and this app \
              does not hold.",
        note: "",
        portfolio: |_address| {
            Box::pin(async {
                Ok(portfolio_unavailable(
                    "Historical performance on Lighter needs a read-only API token; \
                     this address-only session still shows current exposure."
                        .to_owned(),
                ))
            })
        },
        markets: || Box::pin(lighter_symbols(Zone::Mainnet)),
        candles: |tape, coin, interval| {
            Box::pin(lighter_candles(Zone::Mainnet, tape, coin, interval))
        },
        history: |tape, coin, interval| {
            Box::pin(lighter_history(Zone::Mainnet, tape, coin, interval))
        },
        account: |address| Box::pin(lighter_account(Zone::Mainnet, address)),
        // `account_all/<index>` and the order channels are keyed by account
        // index rather than L1 address, and want an API-key-signed token an
        // address alone cannot get (`code 20001`). An address is all this app
        // asks a reader for, so there are no orders and no fills here.
        orders: |_address| Box::pin(async { Ok(Vec::new()) }),
        market_feed: |tape| lighter_market_feed(Zone::Mainnet, tape),
        fill_feed: |_address| no_stream(),
    };

    /// The same exchange's test deployment, verified live rather than assumed:
    /// it answers the same reads over the same shapes, streams the same
    /// `order_book:N` channels over the same `/stream`, and its faucet creates
    /// and funds an account from one request.
    ///
    /// A separate entry rather than a flag, for the reason the Hyperliquid pair
    /// is: the market ids are the deployment's own — BTC is 1 here against a
    /// mainnet universe of 222 — so nothing derived from one deployment means
    /// anything against the other, and the adapter reads the ticker-to-id table
    /// out of whichever universe it was handed.
    pub const LIGHTER_TESTNET: Network = Network {
        venue: Venue::LighterTestnet,
        rests_forever: false,
        attaches_levels: false,
        stamps_funding: true,
        name: "Lighter Testnet",
        testnet: true,
        signing: Signing::ApiKey(Zone::Testnet),
        gap: "Lighter serves resting orders and this account's fills only to an \
              API-key-signed token, which an address alone cannot get and this app \
              does not hold.",
        note: "This is Lighter's test deployment. It is a separate book with its \
               own accounts and its own market ids — BTC is market 1 here and the live \
               exchange lists two hundred more — so an account funded on mainnet has \
               nothing here, and nothing traded here is worth anything. \
               `GET /api/v1/faucet?l1_address=…` creates and funds one.",
        portfolio: |_address| {
            Box::pin(async {
                Ok(portfolio_unavailable(
                    "Historical performance on Lighter needs a read-only API token; \
                     this address-only session still shows current exposure."
                        .to_owned(),
                ))
            })
        },
        markets: || Box::pin(lighter_symbols(Zone::Testnet)),
        candles: |tape, coin, interval| {
            Box::pin(lighter_candles(Zone::Testnet, tape, coin, interval))
        },
        history: |tape, coin, interval| {
            Box::pin(lighter_history(Zone::Testnet, tape, coin, interval))
        },
        account: |address| Box::pin(lighter_account(Zone::Testnet, address)),
        // `account_all/<index>` and the order channels are keyed by account
        // index rather than L1 address, and want an API-key-signed token an
        // address alone cannot get (`code 20001`). An address is all this app
        // asks a reader for, so there are no orders and no fills here.
        orders: |_address| Box::pin(async { Ok(Vec::new()) }),
        market_feed: |tape| lighter_market_feed(Zone::Testnet, tape),
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
    (Network::of(venue).markets)().await
}

pub async fn venue_candles(
    venue: Venue,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    (Network::of(venue).candles)(tape, coin, interval).await
}

pub async fn venue_history(
    venue: Venue,
    tape: Tape,
    coin: String,
    interval: String,
) -> Result<i64, HlError> {
    (Network::of(venue).history)(tape, coin, interval).await
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
    (Network::of(venue).account)(address).await
}

pub async fn venue_orders(venue: Venue, address: String) -> Result<Vec<Order>, HlError> {
    if address.trim().is_empty() {
        return Ok(Vec::new());
    }
    (Network::of(venue).orders)(address).await
}

pub fn venue_market_feed(venue: Venue, tape: Tape) -> Receiver<Result<MarketTick, HlError>> {
    (Network::of(venue).market_feed)(tape)
}

/// The same rule on the socket. A fills subscription names the account it is
/// for, and Hyperliquid answers an empty user with a rejected subscription —
/// which the feed reports as a dropped connection and retries forever.
pub fn venue_fill_feed(venue: Venue, address: String) -> Receiver<Result<Vec<Fill>, HlError>> {
    if address.trim().is_empty() {
        return no_stream();
    }
    (Network::of(venue).fill_feed)(address)
}

/// What a venue will not tell this app about the account it is watching, or
/// nothing when it answers everything asked of it.
///
/// One fact, stated once: `orders` and `fill_feed` are empty on Lighter for the
/// same reason, so the two panels below read their emptiness out of this rather
/// than each carrying its own opinion of why.
pub fn venue_account_gap(venue: Venue) -> String {
    Network::of(venue).gap.to_owned()
}

/// What a reader has to know about this network that its name does not say,
/// or nothing. Drawn on settings among the network's other facts, never where
/// rows would be: a sentence under an empty panel is read as the reason the
/// panel is empty.
pub fn venue_note(venue: Venue) -> String {
    Network::of(venue).note.to_owned()
}

/// What the account strip says when there is no account to draw.
///
/// Four facts share that empty state and only one of them is about this app:
/// with no address there is nothing to read; with an address the read can
/// still be in flight, can have failed, or can have come back saying there is
/// nothing at this venue to find. "Settings takes an address" is the first,
/// and said over any of the others it sends a reader to re-enter the address
/// that is already there. One address is typed once and read at whichever
/// venue is on screen, so holding a book at one exchange and none at the other
/// is ordinary — and it is the venue rather than the address that makes it so,
/// which is why the sentence names the venue.
///
/// `missing` is the venue's own answer rather than the absence of one: an
/// account read that has not landed yet leaves it false, and drawing "no
/// account here" over it reports a slow venue as an empty one. A failure wins
/// over both, because a read that broke says nothing about what is there.
pub fn venue_account_note(venue: Venue, watching: bool, missing: bool, failure: String) -> String {
    if !watching {
        return "No account is being read. Settings takes an address.".to_owned();
    }
    if !failure.is_empty() {
        return format!("This account could not be read: {failure}");
    }
    if missing {
        return format!("No {} account for this address.", venue_name(venue));
    }
    format!("Reading this account on {}.", venue_name(venue))
}

/// What the resting-orders panel says when it has no rows to draw.
///
/// An empty list reads as "nothing has happened", which on a venue that will
/// not answer is a lie: nothing can happen. The sentence is the panel's empty
/// state rather than a banner so that it lands where the reader is already
/// looking for the rows.
///
/// A read that failed empties the panel the same way and means the third
/// thing: the venue serves this, and the app does not know what it holds.
/// `failure` is the message of the read that broke, and it outranks the empty
/// list because the list is not evidence of anything once the read behind it
/// is gone. It cannot outrank the gap: a venue that does not carry the channel
/// is never asked, so it never fails.
pub fn venue_orders_note(venue: Venue, watching: bool, failure: String) -> String {
    match venue_account_gap(venue) {
        gap if !gap.is_empty() => gap,
        _ if !failure.is_empty() => format!("Resting orders could not be read: {failure}"),
        _ if watching => "No resting orders.".to_owned(),
        _ => "Orders need an address.".to_owned(),
    }
}

/// The same for fills. An address is what separates the two things a venue
/// that does answer can say; a venue that does not answer says so either way,
/// because connecting an address would not change it.
pub fn venue_fills_note(venue: Venue, watching: bool, failure: String) -> String {
    match venue_account_gap(venue) {
        gap if !gap.is_empty() => gap,
        _ if !failure.is_empty() => format!("Fills could not be read: {failure}"),
        _ if watching => "No fills on this account yet.".to_owned(),
        _ => "Fills need an address.".to_owned(),
    }
}

/// Where a written export landed, or why it did not.
///
/// Two fields rather than one string, for the reason the three read failures in
/// `state.ice` are three fields: a path and a refusal are drawn in different
/// places — the app's own status line and its alarm line — and a single string
/// would have to be parsed by the view to tell them apart.
#[derive(Clone, Default, PartialEq)]
pub struct Export {
    /// What the app says it did, naming the file. Empty when nothing was
    /// written.
    pub note: String,
    /// Why nothing was written. Empty when something was.
    pub error: String,
}

/// The fills on screen as a spreadsheet reads them.
///
/// Every column a fill row draws plus the two it does not: which network the
/// account was read from, and whether that network trades money worth anything.
/// A testnet fill exported without saying so is a mainnet record the moment it
/// is opened somewhere else, and neither the venue nor the deployment is
/// recoverable from a coin and a price.
///
/// Every field is quoted, including the header and the numbers. Nothing this
/// app holds carries a comma today, and quoting is four characters against a
/// symbol that carries one tomorrow.
///
/// Figures are written at full precision rather than through the panel's
/// formatters: a thousands separator is a reader's comma inside a field, and a
/// price rounded to what a 72-pixel column had room for is not the fill.
pub fn fills_csv(venue: Venue, fills: &[Fill]) -> String {
    let network = Network::of(venue);
    let deployment = if network.testnet {
        "testnet"
    } else {
        "mainnet"
    };
    let mut out = String::new();
    out.push_str(
        "\"time\",\"coin\",\"side\",\"size\",\"price\",\"closed_pnl\",\"trade_id\",\"venue\",\
         \"network\"\n",
    );
    for fill in fills {
        let row = [
            iso_utc(fill.ts),
            fill.coin.clone(),
            if fill.buy { "buy" } else { "sell" }.to_owned(),
            fill.size.to_string(),
            fill.price.to_string(),
            fill.closed_pnl.to_string(),
            fill.tid.to_string(),
            network.name.to_owned(),
            deployment.to_owned(),
        ];
        for (index, field) in row.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&field.replace('"', "\"\""));
            out.push('"');
        }
        out.push('\n');
    }
    out
}

/// Writes those fills where a reader can find them, and answers with the path.
///
/// There is no file chooser here. Iced has no such widget and Ice has no
/// built-in that opens one; the repository's one precedent adds `rfd` and an
/// async extern for it, which is a modal over a screen holding open positions
/// and a dependency for a button. So the file goes to a place the reader
/// already has — their downloads folder, or their home, or the process temp
/// directory — and the app says the whole path rather than leaving them to
/// guess it.
///
/// The name is the newest fill's own hour and not the wall clock, so exporting
/// the same fills twice writes the same file rather than a second copy under a
/// new name.
pub fn write_fills_csv(venue: Venue, fills: Vec<Fill>) -> Export {
    if fills.is_empty() {
        return Export {
            note: String::new(),
            error: "No fills to export.".to_owned(),
        };
    }
    let newest = fills.iter().map(|fill| fill.ts).max().unwrap_or_default();
    let slug = venue_name(venue).to_lowercase().replace(' ', "-");
    let stamp = iso_utc(newest).replace(['-', ':'], "").replace('Z', "");
    let path = export_dir().join(format!("fills-{slug}-{stamp}.csv"));
    match std::fs::write(&path, fills_csv(venue, &fills)) {
        Ok(()) => Export {
            note: format!("Wrote {} fills to {}", fills.len(), path.display()),
            error: String::new(),
        },
        Err(cause) => Export {
            note: String::new(),
            error: format!("Could not write {}: {cause}", path.display()),
        },
    }
}

/// The folder an export lands in.
///
/// Downloads is where a browser puts a file a reader asked for and the first
/// place they look for one; home is where a machine without it still has
/// somewhere they own; the temp directory is the last resort rather than the
/// habit, because a file there is one the system may delete.
fn export_dir() -> PathBuf {
    // A suite run must not leave files in the reader's own folders, and the
    // spec puts deterministic extern behaviour behind `cfg(test)`.
    if cfg!(test) {
        return std::env::temp_dir();
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let Some(home) = home else {
        return std::env::temp_dir();
    };
    let downloads = home.join("Downloads");
    if downloads.is_dir() { downloads } else { home }
}

/// Epoch seconds as a calendar instant in UTC.
///
/// A spreadsheet column of `64970`-shaped integers is a column nobody reads,
/// and the panel's own `fmt_time` is a wall clock with no date on it — fine
/// against a list printed today and useless in a file opened next quarter.
fn iso_utc(ts: i64) -> String {
    let (year, month, day) = civil_from_days(ts.div_euclid(86_400));
    let seconds = ts.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

/// Howard Hinnant's `civil_from_days`, which is the whole of what turns a day
/// count into a date and is exact over every year this app can be handed.
/// Vendored as arithmetic rather than as a dependency: a calendar crate is a
/// tree of them for nine lines.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// What a venue calls the three ways an order can rest, in the four
/// characters the segmented row has for it.
///
/// Two of the three are the same word at both exchanges. The third is not, and
/// the difference is not cosmetic: Hyperliquid's `Gtc` rests until it is
/// cancelled, and Lighter has no such thing — its order carries a deadline it
/// was signed with (`ORDER_TIME_IN_FORCE_GOOD_TILL_TIME`), so an order left
/// alone expires rather than waiting. A button reading GTC over that would be
/// this app inventing a guarantee the venue never made.
///
/// Read from the exchange endpoint's `limit.tif` (`Alo` | `Ioc` | `Gtc`) and
/// from Lighter's own SDK enum (`IMMEDIATE_OR_CANCEL` | `GOOD_TILL_TIME` |
/// `POST_ONLY`).
pub fn tif_name(venue: Venue, tif: Tif) -> String {
    match tif {
        Tif::Gtc if Network::of(venue).rests_forever => "GTC",
        Tif::Gtc => "GTT",
        Tif::Ioc => "IOC",
        Tif::Alo => "ALO",
    }
    .to_owned()
}

/// The same three, as the act a reader hears rather than as the abbreviation
/// they are painted in. Four letters is what the column has and no help at all
/// to anyone hearing them one at a time.
pub fn tif_act(venue: Venue, tif: Tif) -> String {
    match tif {
        Tif::Gtc if Network::of(venue).rests_forever => "Rest until cancelled",
        Tif::Gtc => "Rest until its deadline",
        Tif::Ioc => "Fill now or cancel the rest",
        Tif::Alo => "Rest only; cancel if it would cross",
    }
    .to_owned()
}

/// What the chosen resting rule does not mean at this venue, or nothing when
/// the name is the whole of it.
///
/// One sentence under the row rather than a renamed button alone, because the
/// name is four letters and the difference is a deadline the reader is about
/// to sign.
pub fn venue_tif_note(venue: Venue, tif: Tif) -> String {
    if tif != Tif::Gtc || Network::of(venue).rests_forever {
        return String::new();
    }
    format!(
        "{} has no rest-until-cancelled: the order carries a deadline it is signed with and \
         expires there.",
        venue_name(venue)
    )
}

/// Whether this venue takes a take-profit and a stop-loss attached to the
/// order that opens the position, and what it does instead when it does not.
///
/// Hyperliquid takes them on the same action: a `trigger` order carrying
/// `triggerPx` and `tpsl`, grouped with the entry. Lighter's public API has no
/// such grouping — its SDK exposes `create_tp_order` and `create_sl_order` as
/// whole independent orders, and nothing anywhere in it names a parent. So on
/// Lighter these two fields would be a promise the app cannot keep: the entry
/// would go, and the protection would be a second order nobody placed.
pub fn venue_attaches_levels(venue: Venue) -> bool {
    Network::of(venue).attaches_levels
}

pub fn venue_levels_note(venue: Venue) -> String {
    if venue_attaches_levels(venue) {
        return String::new();
    }
    // Two different reasons, and a reader deserves the one true of the exchange
    // in front of them: one venue has nothing to attach, the other has it and
    // this app has not built it.
    match Network::of(venue).signing {
        Signing::ApiKey(_) => format!(
            "{} attaches no levels to an entry. Its API takes them as separate orders once the \
             position exists, which this app does not place.",
            venue_name(venue)
        ),
        Signing::Eip712(_) => format!(
            "{} does take a target and a stop on the entry, and this app does not send them \
             yet. They are offered nowhere rather than offered here: a field promising a \
             position is protected, over an order that carries no protection, is the one \
             mistake this panel must never make.",
            venue_name(venue)
        ),
    }
}

/// The order the ticket is describing, frozen.
///
/// This is the payload seam and the confirmation's script at once, which is the
/// point: the panel's readouts and the bytes that reach an exchange are the
/// same handful of numbers, projected once. A confirmation built from a second
/// walk of the state would be a screen agreeing with itself about an order the
/// wire never saw.
///
/// It carries figures rather than sentences. Every one of these is already on
/// screen above the button, formatted by the same `fmt_*` the confirmation
/// formats it with, so the confirmation restates the order in the words the
/// reader has been reading — it makes no claim of its own and computes nothing.
///
/// **Frozen** matters as much as projected. The book moves; a confirmation that
/// re-derived itself between the press and the send would show one price and
/// send another, and the reader would have agreed to neither. So the handler
/// snapshots this on the press and the send spends the snapshot.
#[derive(Clone, PartialEq)]
pub struct Draft {
    pub venue: Venue,
    pub coin: String,
    /// The market row this order was priced against, which is also what names
    /// the market on the wire: Hyperliquid carries an index out of its own
    /// universe and never a ticker. Carried in the snapshot rather than looked
    /// up at send time so the order reaches the market the ticket was quoting.
    pub market: Option<SymbolRow>,
    pub buy: bool,
    /// The size in the instrument, as `order_size` normalised it — the unit
    /// toggle converted and reduce-only already capped.
    pub size: f64,
    /// What the order transacts at, as `order_price` resolved it.
    pub price: f64,
    /// Whether that price came from walking the book rather than from the
    /// field. The confirmation says which, because a walk is an estimate and a
    /// typed limit is a promise.
    pub walked: bool,
    pub reduce_only: bool,
    pub cross: bool,
    pub tif: Tif,
    pub leverage: f64,
    pub notional: f64,
    pub margin: f64,
    /// Zero when the ticket could not price one, which the confirmation draws
    /// as the same "not known" the panel above it draws.
    pub liquidation: f64,
    pub tp: f64,
    pub sl: f64,
    /// Why this order cannot be sent as typed, or nothing when it can.
    ///
    /// Folded here rather than left to the view because sendability is one
    /// decision: the view disables one button and prints one sentence, and a
    /// condition the view forgot to `&&` is a live button over an order the
    /// venue will refuse. The per-control refusals are still drawn beside their
    /// own controls — this is what the *send* reads.
    pub refusal: String,
}

/// Prints the order and never the market row behind it. `SymbolRow` carries
/// forty fields of live market data that say nothing about what is being sent,
/// and a `Debug` that dumped them would bury the eight figures that matter in a
/// test failure.
impl fmt::Debug for Draft {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Draft")
            .field("coin", &self.coin)
            .field("buy", &self.buy)
            .field("size", &self.size)
            .field("price", &self.price)
            .field("walked", &self.walked)
            .field("reduce_only", &self.reduce_only)
            .field("cross", &self.cross)
            .field("listed", &self.market.is_some())
            .field("refusal", &self.refusal)
            .finish()
    }
}

/// Project the order the ticket is describing.
///
/// Every argument is a value the panel is already showing. Nothing is
/// recomputed: `size` and `price` arrive as `order_size` and `order_price`
/// resolved them, the risk figures as `price_ticket` priced them, and the three
/// refusals as their own controls already worked them out.
#[allow(clippy::too_many_arguments)]
pub fn order_draft(
    venue: Venue,
    coin: String,
    market: Option<SymbolRow>,
    buy: bool,
    size: String,
    price: f64,
    walked: bool,
    reduce_only: bool,
    cross: bool,
    tif: Tif,
    quote: Ticket,
    tp: String,
    sl: String,
    reduce_refusal: String,
    tp_refusal: String,
    sl_refusal: String,
) -> Draft {
    let size = amount(&size).abs();
    let (tp, sl) = (amount(&tp), amount(&sl));
    Draft {
        venue,
        coin: coin.clone(),
        market: market.clone(),
        buy,
        size,
        price,
        walked,
        reduce_only,
        cross,
        tif,
        leverage: quote.leverage,
        notional: quote.notional,
        margin: quote.margin,
        liquidation: quote.liquidation,
        tp,
        sl,
        refusal: draft_refusal(
            &coin,
            market.as_ref(),
            size,
            price,
            tp,
            sl,
            [
                // Only when the promise is actually being made. The refusal is
                // computed whether or not the box is ticked — the ticket draws
                // it under `if ticket_reduce` for the same reason — and folding
                // it unconditionally would refuse every order in a market this
                // account holds no position in.
                if reduce_only {
                    reduce_refusal
                } else {
                    String::new()
                },
                tp_refusal,
                sl_refusal,
            ],
        ),
    }
}

/// Why an order as typed is not sendable, in the order a reader fixes them.
///
/// A market with no name is the ticket before a market is chosen; the two
/// figures are what an order is made of, and either at zero is not an order
/// that has been described yet rather than one worth refusing loudly. A
/// per-control refusal outranks both, because a control that is already saying
/// what is wrong with it should not be contradicted by a button saying
/// something vaguer.
#[allow(clippy::too_many_arguments)]
fn draft_refusal(
    coin: &str,
    market: Option<&SymbolRow>,
    size: f64,
    price: f64,
    tp: f64,
    sl: f64,
    refusals: [String; 3],
) -> String {
    if let Some(said) = refusals.into_iter().find(|said| !said.is_empty()) {
        return said;
    }
    if coin.trim().is_empty() {
        return "Choose a market first.".to_owned();
    }
    // A market deployed by somebody else is margined against a clearinghouse
    // this app cannot read, so it can quote neither the requirement nor the
    // cliff for one — and an order is the one place being wrong about which
    // account backs it costs money. `hl_place` refuses these again on the wire
    // rather than trusting this.
    if coin.contains(':') {
        return format!(
            "{coin} is margined against a clearinghouse this app cannot read, so it will not \
             send an order there."
        );
    }
    if market.is_none() {
        return "This market is not loaded here.".to_owned();
    }
    // Belt beside the venue fact. `attaches_levels` is what stops the two
    // fields being offered; this is what stops an order carrying them if they
    // are ever set by some other path, because the wire has nowhere to put
    // them and the confirmation would be promising protection that never left.
    if tp > 0.0 || sl > 0.0 {
        return "This app does not attach a target or a stop to an order yet, so it will not \
                send one that has them."
            .to_owned();
    }
    // `<=` rather than `!(_ > _)`: a size that is NaN is not a size either,
    // and both spellings refuse it — this one just says so readably.
    if size.is_nan() || size <= 0.0 {
        return "This order has no size yet.".to_owned();
    }
    if price.is_nan() || price <= 0.0 {
        return "This order has no price yet.".to_owned();
    }
    String::new()
}

/// What pressing send would do, said in one line.
///
/// The button's accessible name, so somebody who cannot see the ticket hears
/// the order rather than the word "send" — and hears which network it is going
/// to, because that is the fact this whole screen exists to keep in front of
/// the reader.
pub fn order_act(draft: Draft) -> String {
    let side = if draft.buy { "buy" } else { "sell" };
    let network = venue_name(draft.venue);
    let kind = venue_kind(draft.venue);
    format!(
        "Send this {side} of {} {} on {network}, {kind}",
        fmt_size(draft.size),
        draft.coin,
    )
}

/// Which margin engine holds this position: the word the ticket's own toggle
/// uses, so the confirmation names the mode in the reader's vocabulary rather
/// than in a second one.
pub fn margin_mode(cross: bool) -> String {
    if cross { "cross" } else { "isolated" }.to_owned()
}

/// What the review button says, which is the side it would review.
///
/// The side is in the word rather than only in the button's colour, for the
/// reason every other control on this screen states its own state in its name:
/// a reader who cannot see two inks still has to know which way this order runs
/// before they open the panel that spends money.
pub fn review_label(buy: bool) -> String {
    if buy { "REVIEW BUY" } else { "REVIEW SELL" }.to_owned()
}

/// The frozen order's own figures, for the tests that hold it against the
/// ticket it was projected from. Ice sees `Draft` through the fields its extern
/// declares; these are the ones an assertion needs and the view does not.
pub fn confirm_price(draft: Option<Draft>) -> f64 {
    draft.map_or(0.0, |draft| draft.price)
}

pub fn confirm_size(draft: Option<Draft>) -> f64 {
    draft.map_or(0.0, |draft| draft.size)
}

pub fn confirm_notional(draft: Option<Draft>) -> f64 {
    draft.map_or(0.0, |draft| draft.notional)
}

pub fn confirm_liquidation(draft: Option<Draft>) -> f64 {
    draft.map_or(0.0, |draft| draft.liquidation)
}

pub fn confirm_walked(draft: Option<Draft>) -> bool {
    draft.is_some_and(|draft| draft.walked)
}

/// What the margin figures on a confirmation are, and are not.
///
/// They are arithmetic done here, for the mode and the leverage the ticket is
/// holding. **Neither is sent.** Both exchanges keep a margin mode and a
/// leverage per market on the account itself, and a position opens at whatever
/// that setting says — so a confirmation that stated "isolated, 5x" as though
/// it had arranged anything would be describing an order the venue never
/// receives.
///
/// Not implemented rather than not noticed. Hyperliquid has an `updateLeverage`
/// action, and sending it before the order would make the figures true — but it
/// sets the leverage for the *market*, not for the order, so it would silently
/// re-lever any position already open there, and a pair where the first half
/// lands and the second does not leaves an account changed with nothing bought.
/// That is a second promise the panel would be making and not keeping, so the
/// honest thing is the sentence rather than the action, until the pair can be
/// sent and seen to take.
pub fn margin_estimate_note() -> String {
    "These margin figures are worked out here, for the mode and leverage above. Neither is sent \
     with the order: both are settings the exchange keeps per market on your account, and the \
     position opens at whatever they say."
        .to_owned()
}

/// Whether a confirmation is standing over an order.
pub fn order_pending(draft: Option<Draft>) -> bool {
    draft.is_some()
}

/// How far through the mark a flatten is allowed to pay.
///
/// A close-everything is a market order and a market order needs a price the
/// book will actually take. There is no book on screen for the markets a
/// reader is not watching, so the crossing price is the position's own mark
/// moved this far in the direction that fills — which is what Hyperliquid's own
/// SDK does for `market_close`, at this same figure. It is stated on the
/// confirmation rather than left as a constant nobody sees, because a limit
/// five per cent through the mark is a real amount of money on a wide book.
const FLATTEN_SLIPPAGE: f64 = 0.05;

/// One act over every row of a panel, frozen on the press.
///
/// The panel-wide controls are loops over the single paths and nothing else:
/// CANCEL ALL is the row's own CANCEL run down the list, FLATTEN ALL is CLOSE
/// POSITION run down the list. What makes them worth a type is the same thing
/// that makes `Draft` worth one — the confirmation and the wire have to be
/// describing the same act, and a list re-read between the press and the send
/// is a list that changed.
///
/// Frozen at list granularity for exactly the reason `Draft` is frozen at order
/// granularity: an order can fill and a position can move while the reader is
/// reading the panel. What is agreed to is what is sent, and the rows below are
/// the rows that go.
#[derive(Clone, PartialEq)]
pub struct Sweep {
    pub venue: Venue,
    /// Which act this is: pull every resting order, or flatten every position.
    /// One field rather than two lists that could both be full.
    pub cancel: bool,
    /// The resting orders, when this is a cancel. Empty otherwise.
    pub orders: Vec<Order>,
    /// One closing order per position, when this is a flatten. Each is an
    /// ordinary `Draft` and goes down the ordinary send path, so nothing here
    /// reaches an exchange by a route a single order does not.
    pub drafts: Vec<Draft>,
    /// What the confirmation's heading says, in the count it froze.
    pub act: String,
    /// What this costs to be wrong about, under the heading.
    pub note: String,
    /// One line per row, for the panel to list what is about to go. The lists
    /// above are the payload; this is the only part the view reads.
    pub rows: Vec<String>,
}

/// Prints the act and the count rather than the payload: a failure that dumped
/// forty orders buries the one fact worth reading.
impl fmt::Debug for Sweep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Sweep")
            .field("cancel", &self.cancel)
            .field("rows", &self.rows.len())
            .field("act", &self.act)
            .finish()
    }
}

/// Pull every resting order, frozen.
pub fn sweep_orders(venue: Venue, orders: Vec<Order>) -> Sweep {
    let rows = orders.iter().cloned().map(order_label).collect();
    Sweep {
        venue,
        cancel: true,
        act: sweep_act(orders.len(), true),
        note: "Every order below stops resting. Nothing is bought or sold: a cancelled order \
               that had not filled leaves no position behind, and one that had filled has \
               already left one."
            .to_owned(),
        orders,
        drafts: Vec::new(),
        rows,
    }
}

/// Close every position, frozen: one ordinary closing order per row.
///
/// Each draft is built the way CLOSE POSITION builds one — reduce-only, the
/// side that moves the position towards zero, the whole size — and priced to
/// cross rather than to rest, because a flatten that quietly rested would be a
/// panic button that did nothing.
pub fn sweep_positions(venue: Venue, positions: Vec<Position>, markets: Vec<SymbolRow>) -> Sweep {
    let drafts: Vec<Draft> = positions
        .iter()
        .map(|held| flatten_draft(venue, held, &markets))
        .collect();
    let rows = drafts.iter().map(flatten_line).collect();
    Sweep {
        venue,
        cancel: false,
        act: sweep_act(positions.len(), false),
        note: format!(
            "Each goes as a reduce-only order priced up to {}% through its own mark, so it \
             crosses rather than rests. This spends money and it is not reversible.",
            (FLATTEN_SLIPPAGE * 100.0).round()
        ),
        orders: Vec::new(),
        drafts,
        rows,
    }
}

/// One line of a flatten's list: the position being closed and the limit the
/// order that closes it carries.
///
/// The same shape a cancel's line has, because both lists are lists of orders
/// that are about to go. The side named is the *position's* — a buy closes a
/// short — because that is what the reader is looking at in the panel behind.
fn flatten_line(draft: &Draft) -> String {
    let side = if draft.buy { "short" } else { "long" };
    format!(
        "Close {} {side} {} at up to {}",
        draft.coin,
        fmt_size(draft.size),
        fmt_px(draft.price)
    )
}

/// The closing order for one position, which is CLOSE POSITION without a ticket
/// to seed.
///
/// Neither figure the ticket would have quoted is invented here: the margin a
/// close asks for is none and the cliff it moves towards is none, which is what
/// `price_ticket` already answers for a reduce-only order and what the ticket's
/// own readouts already print. `draft_refusal` runs over it exactly as it runs
/// over a typed one, so a market this app cannot price is refused here and not
/// at the exchange.
fn flatten_draft(venue: Venue, held: &Position, markets: &[SymbolRow]) -> Draft {
    let buy = held.size < 0.0;
    let size = held.size.abs();
    let market = markets
        .iter()
        .find(|row| row.name == held.coin && !row.heading)
        .cloned();
    let cross = held.margin_mode == "cross";
    let slip = if buy {
        1.0 + FLATTEN_SLIPPAGE
    } else {
        1.0 - FLATTEN_SLIPPAGE
    };
    let price = (held.mark * slip).max(0.0);
    Draft {
        venue,
        coin: held.coin.clone(),
        market: market.clone(),
        buy,
        size,
        price,
        walked: true,
        reduce_only: true,
        cross,
        tif: Tif::Ioc,
        leverage: held.leverage,
        notional: size * held.mark,
        margin: 0.0,
        liquidation: 0.0,
        tp: 0.0,
        sl: 0.0,
        refusal: draft_refusal(
            &held.coin,
            market.as_ref(),
            size,
            price,
            0.0,
            0.0,
            [String::new(), String::new(), String::new()],
        ),
    }
}

/// The heading a sweep's confirmation carries, in the count it froze.
fn sweep_act(count: usize, cancel: bool) -> String {
    let thing = if cancel { "resting order" } else { "position" };
    let plural = if count == 1 { "" } else { "s" };
    let verb = if cancel { "Cancel" } else { "Close" };
    format!("{verb} {count} {thing}{plural}")
}

/// Why a panel-wide control is dead, or nothing when it is live.
///
/// The session's own refusal outranks the panel's: a locked session cannot
/// cancel one order or seven, and saying "no orders to cancel" over a list that
/// has seven in it would be a second, wrong reason.
pub fn sweep_refused(locked: String, count: i64, cancel: bool) -> String {
    if !locked.is_empty() {
        return locked;
    }
    if count > 0 {
        return String::new();
    }
    if cancel {
        "No resting orders to cancel.".to_owned()
    } else {
        "No open positions to close.".to_owned()
    }
}

/// What a panel-wide control does, said in full — including why it will not.
///
/// The reason travels in the name because these two sit in header rows a
/// sentence does not fit in, and a control that is dead for a reason nobody can
/// read is the thing this app refuses to ship elsewhere.
pub fn sweep_label(count: i64, cancel: bool, refusal: String) -> String {
    let count = usize::try_from(count).unwrap_or(0);
    if refusal.is_empty() {
        return format!("{}, one confirmation", sweep_act(count, cancel));
    }
    format!("{} — {refusal}", sweep_act(count, cancel))
}

/// Whether a sweep's confirmation is standing over the terminal.
pub fn sweep_pending(sweep: Option<Sweep>) -> bool {
    sweep.is_some()
}

/// The frozen sweep's own words, read the way the order confirmation reads a
/// frozen draft's own figures: through an accessor per line rather than by
/// projecting the option in the view. Nothing here computes; each is the string
/// the press already built.
pub fn sweep_heading(sweep: Option<Sweep>) -> String {
    sweep.map(|act| act.act).unwrap_or_default()
}

pub fn sweep_note(sweep: Option<Sweep>) -> String {
    sweep.map(|act| act.note).unwrap_or_default()
}

pub fn sweep_rows(sweep: Option<Sweep>) -> Vec<String> {
    sweep.map(|act| act.rows).unwrap_or_default()
}
/// How long until this market is funded again, as the positions panel
/// prints it.
///
/// The app has always shown funding as a rate and never as a schedule, which
/// answers what a position costs to hold and not when the next bill lands. Both
/// exchanges charge hourly, so the boundary is an interval past an anchor: the
/// network's own stamp of the charge it last took where it publishes one, and
/// the clock's own hour where it does not. The stamp is rolled forward by whole
/// intervals rather than simply incremented, so a socket that has been quiet
/// for three hours still names a boundary in the future instead of counting
/// down into the past.
///
/// A dash where it is not known, which on Lighter is every market until the
/// stats channel has spoken: an invented hour on a screen a position is held
/// against is worse than a screen that says it does not know.
pub fn funding_countdown(venue: Venue, market: Option<SymbolRow>, now: i64) -> String {
    let unknown = || "—".to_owned();
    let (Some(market), true) = (market, now > 0) else {
        return unknown();
    };
    let anchor = if Network::of(venue).stamps_funding {
        if market.funding_at <= 0 {
            return unknown();
        }
        market.funding_at
    } else {
        0
    };
    let next = anchor + FUNDING_INTERVAL * ((now - anchor).div_euclid(FUNDING_INTERVAL) + 1);
    match next - now {
        remaining if remaining < 60 => "now".to_owned(),
        remaining => format!("{}m", remaining / 60),
    }
}

/// How often either exchange charges funding, in seconds.
///
/// Hourly at both, and settled against each rather than assumed: Hyperliquid's
/// `predictedFundings` states `fundingIntervalHours: 1` for its own perps, and
/// Lighter's `market_stats` stamps whole hours (23:00:00.002Z live on
/// 2026-08-09, 11:00:00.002Z in the captured fixture) while quoting a rate the
/// parser already reads as hourly.
const FUNDING_INTERVAL: i64 = 3_600;

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
    use crate::hyperliquid::{
        demo_fills, demo_orders, demo_positions, demo_symbols, hl_symbols, tape_new,
    };
    // The book is not on `Reads` — the app never asks a venue for one — so the
    // live seam test reaches the adapter's own read directly.
    use crate::lighter::{demo_symbols_lighter, lighter_book};

    /// The account `lighter.rs` established its own units against, so a
    /// failure here is the seam rather than the address.
    const LIGHTER_ACCOUNT: &str = "0x3f4ec7684F679F83c782e485b358A2D43045d6A2";

    /// The two venues the switch offers, so a loop below can assert something
    /// of both without naming them twice.
    const BOTH: [Venue; 2] = [Venue::Hyperliquid, Venue::Lighter];

    /// The registry is the one place a network is enumerated, so the picker
    /// drawing it and `Network::of` resolving it must both be that list and
    /// not a second copy of it.
    ///
    /// The round trip is the part worth having. Adding a network is a variant,
    /// an entry and an arm, and the arm is the one a copy-paste gets wrong:
    /// `Venue::HyperliquidTestnet => Network::HYPERLIQUID` compiles, draws the
    /// right name in the picker, and points every read on the testnet at
    /// mainnet. Nothing else in this file would notice.
    #[test]
    fn the_registry_is_the_only_list_of_networks() {
        assert_eq!(
            venue_list().len(),
            NETWORKS.len(),
            "the picker draws the registry rather than its own list"
        );
        for network in NETWORKS {
            assert_eq!(
                Network::of(network.venue).name,
                network.name,
                "{}: the arm resolves to some other entry",
                network.name,
            );
            assert!(
                !network.name.is_empty(),
                "a network a reader has to choose between needs a name"
            );
        }

        let mut names: Vec<&str> = NETWORKS.iter().map(|network| network.name).collect();
        names.sort_unstable();
        let listed = names.len();
        names.dedup();
        assert_eq!(
            names.len(),
            listed,
            "two networks under one name are two networks a reader cannot tell apart"
        );
    }

    /// The badge and the deployment a signature is pinned to are the same
    /// fact, and an entry where they disagree is the failure this whole seam
    /// exists to make impossible: a screen labelled TESTNET whose orders are
    /// signed for mainnet reads correctly on both halves and empties a real
    /// account.
    ///
    /// What this does not reach: each read closure names its own deployment
    /// literal, because capturing this field would stop it being the `fn`
    /// pointer the seam is built from. A closure naming the wrong deployment
    /// compiles and is caught by review, not by this.
    #[test]
    fn a_network_signs_for_the_deployment_it_says_it_is() {
        // Every network's signing deployment must be the one its badge names.
        // No entry is skipped: the field is not optional, so a network added
        // without a scheme is a compile error rather than a row this loop
        // walks past.
        for network in NETWORKS {
            assert_eq!(
                network.testnet,
                network.signing.testnet(),
                "{}: the badge and the signing deployment disagree",
                network.name,
            );
        }

        // Which networks this app can sign for, and by which scheme. Pinned
        // rather than derived, because changing it is never only a registry
        // edit: the refusal sentences, the enrolment a user has to perform and
        // the custody seam's branches all say which venues can be written to.
        let schemes: Vec<(&str, Signing)> = NETWORKS
            .iter()
            .map(|network| (network.name, network.signing))
            .collect();
        assert_eq!(
            schemes,
            vec![
                ("Hyperliquid", Signing::Eip712(Chain::Mainnet)),
                ("Hyperliquid Testnet", Signing::Eip712(Chain::Testnet)),
                ("Lighter", Signing::ApiKey(Zone::Mainnet)),
                ("Lighter Testnet", Signing::ApiKey(Zone::Testnet)),
            ],
            "the networks this app can sign for changed; the refusal sentences \
             and the enrolment checklist are part of that change"
        );

        assert_eq!(
            Chain::Mainnet.info_url(),
            "https://api.hyperliquid.xyz/info"
        );
        assert_eq!(
            Chain::Testnet.info_url(),
            "https://api.hyperliquid-testnet.xyz/info"
        );
        assert_ne!(
            Network::HYPERLIQUID.signing,
            Network::HYPERLIQUID_TESTNET.signing,
            "one exchange's two deployments have to be two signatures"
        );
        assert_ne!(
            Network::LIGHTER.signing,
            Network::LIGHTER_TESTNET.signing,
            "and the same on the other exchange"
        );
    }

    /// A key is filed under the network it signs for, and one address holds a
    /// different key at each of the four.
    ///
    /// The scheme has to be in that name and not only the deployment. Both
    /// exchanges have a "mainnet", the same address is read at both, and the
    /// two hold unrelated secrets — so names that collided would have the
    /// second enrolment overwrite the first and afterwards hand each venue the
    /// other's key, which surfaces only as an order refused for a signer
    /// nobody recognises.
    #[test]
    fn every_network_files_its_key_under_a_name_of_its_own() {
        let mut names: Vec<String> = NETWORKS
            .iter()
            .map(|network| network.signing.key())
            .collect();
        assert_eq!(names.len(), NETWORKS.len());
        names.sort();
        let filed = names.len();
        names.dedup();
        assert_eq!(
            names.len(),
            filed,
            "two networks file a key under one name: {names:?}"
        );

        // And each name says both halves, so a reader of the keychain can tell
        // which key they are looking at.
        for network in NETWORKS {
            let name = network.signing.key();
            let exchange = if name.starts_with("hyperliquid") {
                "hyperliquid"
            } else {
                "lighter"
            };
            assert!(name.starts_with(exchange), "{name}");
            assert_eq!(
                name.contains("testnet"),
                network.testnet,
                "{}: the item name and the badge disagree",
                network.name
            );
        }
    }

    /// A switch that says which venue is being read only in its highlight
    /// colour says it to whoever can see two inks. The name a reader hears
    /// carries the state, and it is a different sentence for each venue and
    /// for each of the two states.
    #[test]
    fn the_switch_names_the_venue_and_says_which_one_is_being_read() {
        assert_eq!(venue_name(Venue::Hyperliquid), "Hyperliquid");
        assert_eq!(venue_name(Venue::Lighter), "Lighter");

        assert_eq!(
            venue_label(Venue::Lighter, false),
            "Read Lighter, real money"
        );
        assert_eq!(
            venue_label(Venue::Lighter, true),
            "Read Lighter, real money, already reading"
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

    /// Which deployment a row is, spoken rather than only painted.
    ///
    /// The badge inside each row is drawn and never read out: a labelled
    /// button's name replaces its contents, so a reader on a screen reader was
    /// choosing between four names that differ by a word, in the one control
    /// where getting it wrong costs money. The trigger owes the same, plus the
    /// act — a control that only names the network is a readout.
    #[test]
    fn every_switch_says_which_deployment_it_is() {
        for network in NETWORKS {
            let spoken = venue_kind(network.venue).to_lowercase();
            for shown in [false, true] {
                let row = venue_label(network.venue, shown);
                assert!(
                    row.contains(&spoken),
                    "{row}: a row chosen without its kind is a row chosen blind"
                );
            }

            let trigger = venue_switch_label(network.venue);
            assert!(trigger.starts_with(network.name), "{trigger}");
            assert!(trigger.contains(&spoken), "{trigger}");
            assert!(trigger.contains("switch network"), "{trigger}");
        }

        assert_eq!(
            venue_switch_label(Venue::HyperliquidTestnet),
            "Hyperliquid Testnet, testnet — switch network"
        );
        assert_eq!(
            venue_switch_label(Venue::Hyperliquid),
            "Hyperliquid, real money — switch network"
        );
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
            assert_eq!(venue_orders_note(Venue::Lighter, watching, none()), gap);
            assert_eq!(venue_fills_note(Venue::Lighter, watching, none()), gap);
        }
        assert!(venue_account_gap(Venue::Hyperliquid).is_empty());
        // Hyperliquid answers both, so its panels say only what the account
        // holds — and an address is what separates the two things it can say.
        assert_eq!(
            venue_orders_note(Venue::Hyperliquid, true, none()),
            "No resting orders."
        );
        assert_eq!(
            venue_orders_note(Venue::Hyperliquid, false, none()),
            "Orders need an address."
        );
        assert_eq!(
            venue_fills_note(Venue::Hyperliquid, true, none()),
            "No fills on this account yet."
        );
        assert_eq!(
            venue_fills_note(Venue::Hyperliquid, false, none()),
            "Fills need an address."
        );
    }

    /// The third thing an empty panel can mean, and the one an empty list
    /// cannot say on its own: the venue serves this and the read for it broke.
    /// Drawn as "no resting orders" it is the app reporting an unread book as
    /// a flat one.
    #[test]
    fn a_read_that_failed_does_not_read_as_a_venue_with_nothing_to_say() {
        let broke = "Hyperliquid unreachable".to_owned();
        for watching in [false, true] {
            let orders = venue_orders_note(Venue::Hyperliquid, watching, broke.clone());
            let fills = venue_fills_note(Venue::Hyperliquid, watching, broke.clone());
            assert!(orders.contains(&broke), "{orders}");
            assert!(fills.contains(&broke), "{fills}");
            assert_ne!(
                orders,
                venue_orders_note(Venue::Hyperliquid, watching, none())
            );
            assert_ne!(
                fills,
                venue_fills_note(Venue::Hyperliquid, watching, none())
            );
        }
        // A venue that never carries the channel is never asked for it, so the
        // gap is what it says either way rather than a failure it cannot have.
        assert_eq!(
            venue_orders_note(Venue::Lighter, true, broke.clone()),
            venue_account_gap(Venue::Lighter)
        );
        assert_eq!(
            venue_fills_note(Venue::Lighter, true, broke),
            venue_account_gap(Venue::Lighter)
        );
    }

    /// No failure, spelled the way the app spells one.
    fn none() -> String {
        String::new()
    }

    /// The chart's read on the other venue is a read, not a zero.
    ///
    /// The failure this guards is a `candles` that answers `Ok(0)` for every
    /// market and every width, which leaves a chart opened on Lighter holding
    /// the single bar the feed is forming rather than the window it asked for.
    /// Nothing about an answer of that shape says so: it succeeds, and a chart
    /// with nothing in it is what a market that has not traded looks like too.
    ///
    /// So the oracle is the pair of answers only a real read can give. A width
    /// the venue does not quote is refused by name, before any request is
    /// made; a width it does quote goes to the wire, which is closed under
    /// test and says so. A stub gives neither, and neither does a read wired
    /// to the wrong venue.
    #[test]
    fn the_other_venues_chart_reads_its_candles_rather_than_answering_zero() {
        let ask = |interval: &str| {
            smol::block_on(venue_candles(
                Venue::Lighter,
                tape_new(),
                "BTC".to_owned(),
                interval.to_owned(),
            ))
        };

        let refused = ask("3h").expect_err("3h is not a width this venue quotes");
        assert!(refused.message.contains("3h"), "{}", refused.message);
        // And it says what could have been asked for instead, which is the
        // venue's own list rather than the app's tabs.
        assert!(refused.message.contains("30m, 1h"), "{}", refused.message);

        let reached = ask("1h").expect_err("the wire is closed under test");
        assert!(
            reached.message.contains("no wire under test"),
            "a quoted width should reach the venue, got: {}",
            reached.message
        );
    }

    /// No account is two different facts and the strip has one line to say
    /// which. Sending a reader who has connected an address to Settings to
    /// enter the address that is already there is the app blaming them for the
    /// venue's answer.
    #[test]
    fn no_account_says_whether_there_is_an_address_or_only_no_account_here() {
        for venue in BOTH {
            let missing = venue_account_note(venue, true, true, none());
            assert!(
                missing.contains(&venue_name(venue)),
                "the absence is this venue's, so it has to name it: {missing}"
            );
            assert!(
                !missing.contains("Settings"),
                "the address is already connected: {missing}"
            );
            assert_eq!(
                venue_account_note(venue, false, true, none()),
                "No account is being read. Settings takes an address.",
                "with no address it is the app that has nothing, not the venue"
            );
        }
        // And the two venues do not sound alike, because which one answered
        // nothing is the useful half.
        assert_ne!(
            venue_account_note(Venue::Hyperliquid, true, true, none()),
            venue_account_note(Venue::Lighter, true, true, none())
        );
    }

    /// The other two things an unread account can be, and neither of them is
    /// the venue answering "not here". A read still in flight drawn as an
    /// absence reports a slow venue as an empty one; a read that failed drawn
    /// as an absence reports a broken one as an empty one. Both look exactly
    /// like the settled answer, which is why each has to say which it is.
    #[test]
    fn an_unread_account_is_not_an_account_the_venue_says_is_not_there() {
        for venue in BOTH {
            let absent = venue_account_note(venue, true, true, none());
            let reading = venue_account_note(venue, true, false, none());
            let broke = venue_account_note(venue, true, false, "no wire".to_owned());
            assert_ne!(reading, absent, "a read in flight is not an answer");
            assert_ne!(broke, absent, "a read that failed is not an answer");
            assert_ne!(broke, reading);
            assert!(broke.contains("no wire"), "{broke}");
            assert!(
                reading.contains(&venue_name(venue)),
                "the read is of this venue: {reading}"
            );
            // A failure outranks the venue's last answer, because a read that
            // broke says nothing about what is there now.
            assert_eq!(
                venue_account_note(venue, true, true, "no wire".to_owned()),
                broke
            );
        }
    }

    /// A gap is not a failure. Both are drawn, and they are drawn in different
    /// places by different rules: an error raises the app's alarm line, a gap
    /// is the empty state of one panel. A venue answering `Err` for something
    /// it was never going to carry would put "Lighter unreachable" over a
    /// working screen.
    #[test]
    fn a_gap_answers_empty_rather_than_failing() {
        let lighter = Network::of(Venue::Lighter);
        smol::block_on(async {
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

    /// The projection, held against the values it was given.
    ///
    /// Every figure on a `Draft` is one the ticket already computed, so what
    /// this pins is that each arrives *unchanged* and in the right field — the
    /// failure it guards is a transposition, which is invisible on screen when
    /// two figures happen to look alike and is an order for the wrong size when
    /// they do not.
    #[test]
    fn the_draft_carries_the_ticket_s_own_figures() {
        let quote = Ticket {
            notional: 192_000.0,
            margin: 4_800.0,
            liquidation: 62_812.5,
            leverage: 40.0,
            ready: true,
            known: true,
        };
        let draft = order_draft(
            Venue::HyperliquidTestnet,
            "BTC".to_owned(),
            Some(market("BTC")),
            true,
            "3.00".to_owned(),
            64_000.0,
            false,
            false,
            true,
            Tif::Ioc,
            quote.clone(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert_eq!(
            draft.size, 3.0,
            "the size arrives normalised, not re-parsed"
        );
        assert_eq!(draft.price, 64_000.0);
        assert_eq!(draft.notional, quote.notional);
        assert_eq!(draft.margin, quote.margin);
        assert_eq!(draft.liquidation, quote.liquidation);
        assert_eq!(draft.leverage, quote.leverage);
        // No venue attaches levels, so a sendable draft carries none — and one
        // that did would be refused rather than sent without them. The pair
        // below is that refusal.
        assert_eq!((draft.tp, draft.sl), (0.0, 0.0));
        assert!(draft.buy && draft.cross && !draft.walked && !draft.reduce_only);
        assert_eq!(draft.tif, Tif::Ioc);
        assert!(draft.refusal.is_empty(), "{}", draft.refusal);

        let with_levels = order_draft(
            Venue::HyperliquidTestnet,
            "BTC".to_owned(),
            Some(market("BTC")),
            true,
            "3.00".to_owned(),
            64_000.0,
            false,
            false,
            true,
            Tif::Ioc,
            quote,
            "70,000".to_owned(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert_eq!(with_levels.tp, 70_000.0, "the figure is still read");
        assert!(
            with_levels
                .refusal
                .contains("does not attach a target or a stop"),
            "and refused rather than dropped: {}",
            with_levels.refusal,
        );

        // And the one line a reader hears before they press: the side, the
        // size, the network, and what it costs to be wrong on it.
        assert_eq!(
            order_act(draft),
            "Send this buy of 3 BTC on Hyperliquid Testnet, TESTNET",
        );
    }

    /// Why an order cannot be sent, in the order a reader fixes them.
    ///
    /// One sentence, because the button has one disabled state. A control that
    /// is already saying what is wrong with it outranks the button saying
    /// something vaguer, and the two figures an order is made of outrank
    /// nothing at all.
    #[test]
    fn the_draft_refuses_in_the_order_a_reader_fixes_things() {
        let sendable = |size: &str, price: f64| {
            order_draft(
                Venue::Hyperliquid,
                "BTC".to_owned(),
                Some(market("BTC")),
                true,
                size.to_owned(),
                price,
                false,
                false,
                false,
                Tif::Gtc,
                unpriced(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
            .refusal
        };
        assert!(sendable("3", 64_000.0).is_empty());
        assert!(sendable("", 64_000.0).contains("no size"));
        assert!(sendable("0", 64_000.0).contains("no size"));
        assert!(sendable("3", 0.0).contains("no price"));
        // A size is asked for before a price, because a ticket with neither is
        // one nobody has started typing into.
        assert!(sendable("", 0.0).contains("no size"));

        // A control that already said what is wrong outranks all of it. Only
        // when the promise is actually being made, though: the refusal is
        // computed whether or not the box is ticked.
        let with_reduce = |reduce: bool| {
            order_draft(
                Venue::Hyperliquid,
                "BTC".to_owned(),
                Some(market("BTC")),
                true,
                "3".to_owned(),
                64_000.0,
                false,
                reduce,
                false,
                Tif::Gtc,
                unpriced(),
                String::new(),
                String::new(),
                "nothing to reduce".to_owned(),
                String::new(),
                String::new(),
            )
            .refusal
        };
        assert_eq!(with_reduce(true), "nothing to reduce");
        assert!(
            with_reduce(false).is_empty(),
            "a promise nobody is making cannot refuse an order"
        );
    }

    /// A market margined against a clearinghouse this app cannot read is
    /// refused here as well as on the wire, and the sentence names it.
    #[test]
    fn a_builder_market_is_refused_before_anything_is_signed() {
        let refusal = |coin: &str| {
            order_draft(
                Venue::Hyperliquid,
                coin.to_owned(),
                Some(market(coin)),
                true,
                "3".to_owned(),
                224.0,
                false,
                false,
                false,
                Tif::Gtc,
                unpriced(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
            .refusal
        };
        let refused = refusal("xyz:NVDA");
        assert!(refused.contains("xyz:NVDA"), "{refused}");
        assert!(refused.contains("clearinghouse"), "{refused}");
        assert!(
            refusal("BTC").is_empty(),
            "and the venue's own markets are not"
        );
    }

    /// A market the app has not read cannot be ordered against: the wire names
    /// a Hyperliquid market by its index out of a universe this has not
    /// arrived, and an order priced against nothing is not an order.
    #[test]
    fn an_unlisted_market_is_refused() {
        let refusal = order_draft(
            Venue::Hyperliquid,
            "BTC".to_owned(),
            None,
            true,
            "3".to_owned(),
            64_000.0,
            false,
            false,
            false,
            Tif::Gtc,
            unpriced(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .refusal;
        assert!(refusal.contains("not loaded"), "{refusal}");
    }

    /// A ticket with nothing priced in it, which is what the panel holds until
    /// a size and a price are both typed.
    fn unpriced() -> Ticket {
        Ticket {
            notional: 0.0,
            margin: 0.0,
            liquidation: 0.0,
            leverage: 0.0,
            ready: false,
            known: false,
        }
    }

    /// One market row, named.
    fn market(name: &str) -> SymbolRow {
        SymbolRow {
            name: name.to_owned(),
            leverage: 40.0,
            maintenance: 0.0125,
            ..SymbolRow::default()
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
        let rows = smol::block_on(hl_symbols(Chain::Mainnet, HL_CANONICAL)).expect("the universe");
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
        let reads = Network::of(Venue::Lighter);
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
            let book = lighter_book(Zone::Mainnet, top.name.clone())
                .await
                .expect("book");
            assert!(!book.bids.is_empty() && !book.asks.is_empty());
            assert!(book.asks[0].price > book.bids[0].price, "crossed book");

            let account = (reads.account)(LIGHTER_ACCOUNT.to_owned())
                .await
                .expect("account")
                .expect("an account this venue holds");
            assert!(account.value > 0.0);
        });
    }

    /// The test deployment answering the same reads, and answering them
    /// *differently* from mainnet.
    ///
    /// Both halves matter. That the reads succeed says the endpoints and the
    /// parser are right; that the two universes disagree says the entry is
    /// actually pointed at its own deployment, which is the failure a copy-
    /// pasted `Chain` produces and which every offline test walks straight
    /// past. A testnet reading mainnet's markets passes "the reads work",
    /// draws a plausible screen, and prices orders against a book its own
    /// exchange has never seen.
    ///
    /// The disagreement is asserted on the shape of the universe rather than
    /// on any particular market: which markets a test deployment lists is its
    /// own business and changes, but it is a different, smaller universe than
    /// the live one and the maximum leverage its BTC carries is its own.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the testnet seam reads its own deployment"]
    fn the_test_deployment_answers_its_own_reads_rather_than_the_live_ones() {
        crate::hyperliquid::open_the_wire();
        let test = Network::of(Venue::HyperliquidTestnet);
        assert_eq!(test.name, "Hyperliquid Testnet");
        assert!(test.testnet, "the entry has to say what it is");

        smol::block_on(async {
            let rows = (test.markets)().await.expect("the testnet universe");
            assert!(
                rows.len() > 10,
                "the test deployment lists a real universe, not a stub"
            );
            let top = &rows[0];
            assert!(top.price > 0.0, "a market on it has a price");
            assert!(
                top.leverage > 0.0 && top.maintenance > 0.0,
                "and the margin rule the ticket prices against"
            );

            let live = (Network::HYPERLIQUID.markets)()
                .await
                .expect("the live universe");
            assert_ne!(
                rows.len(),
                live.len(),
                "two deployments listing the same number of markets is what \
                 reading one of them through the other's endpoints looks like"
            );

            // HIP-3 is per deployment, and the canonical group's heading is
            // what says which deployment a categorised rail belongs to. Not
            // the builder dexs: their names are third-party and *do* collide —
            // read live, both deployments list a `HyENA` and an `XYZ`, because
            // anyone may deploy a dex of any name on either. So what is held is
            // the one group neither a builder nor a rename can move, and it is
            // exactly what a copy-pasted `Chain` gets wrong.
            let groups = |rows: &[SymbolRow]| {
                let mut named: Vec<String> = rows
                    .iter()
                    .map(|row| row.category.clone())
                    .filter(|category| !category.is_empty())
                    .collect();
                named.sort_unstable();
                named.dedup();
                named
            };
            let here = groups(&rows);
            let there = groups(&live);
            assert!(
                here.contains(&"Hyperliquid Testnet".to_owned()),
                "the test deployment heads its own perps with its own name: {here:?}"
            );
            assert!(
                !here.contains(&"Hyperliquid".to_owned()),
                "and never with the live exchange's: {here:?}"
            );
            assert!(
                there.contains(&"Hyperliquid".to_owned())
                    && !there.contains(&"Hyperliquid Testnet".to_owned()),
                "and the live exchange heads its own the other way round: {there:?}"
            );
        });
    }

    /// Lighter's test deployment answering the same reads as its live one, and
    /// answering them about its own book.
    ///
    /// The ids are the point. Lighter keys a book by a numeric `market_id`
    /// rather than by ticker, and the two deployments number their markets
    /// independently — so a ticker-to-id table built from one and used against
    /// the other opens the wrong book under the right name, which is a screen
    /// that looks entirely correct. Reading BTC here and finding a BTC book is
    /// the whole claim, because it can only be true if the table came from this
    /// deployment's own universe.
    #[test]
    #[ignore = "hits the live venue, run explicitly: the Lighter testnet seam"]
    fn the_lighter_test_deployment_reads_its_own_book() {
        crate::hyperliquid::open_the_wire();
        let test = Network::of(Venue::LighterTestnet);
        assert_eq!(test.name, "Lighter Testnet");
        assert!(test.testnet);

        smol::block_on(async {
            let rows = (test.markets)().await.expect("the testnet universe");
            assert!(!rows.is_empty(), "the deployment lists markets");
            let live = (Network::LIGHTER.markets)()
                .await
                .expect("the live universe");
            assert!(
                live.len() > rows.len(),
                "the live exchange lists far more markets than the test one; \
                 equal counts is one read through the other's endpoints"
            );

            // Keyed by ticker through this deployment's own id table.
            let btc = rows
                .iter()
                .find(|row| row.name == "BTC")
                .expect("the test deployment lists BTC");
            assert!(btc.price > 0.0 && btc.maintenance > 0.0 && btc.leverage > 0.0);
            let book = lighter_book(Zone::Testnet, btc.name.clone())
                .await
                .expect("its book");
            assert!(!book.bids.is_empty() && !book.asks.is_empty());
            assert!(book.asks[0].price > book.bids[0].price, "crossed book");
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

    /// Half past the hour on 2026-08-09, which every countdown assertion below
    /// is measured from. A literal rather than the clock, because a countdown
    /// read against `now` asserts nothing: the answer would be right by
    /// construction whatever the arithmetic did.
    const HALF_PAST: i64 = 1_786_318_200;

    /// A market whose venue has stamped the funding it last took, at `at`.
    fn stamped(at: i64) -> Option<SymbolRow> {
        Some(SymbolRow {
            name: "BTC".to_owned(),
            funding_at: at,
            ..SymbolRow::default()
        })
    }

    /// Hyperliquid states no funding time anywhere the app reads, so its
    /// boundary is the clock's own hour — the interval the venue documents and
    /// its `predictedFundings` restates as `fundingIntervalHours: 1`.
    ///
    /// The oracle is the remainder rather than a shape: at 23:30 the answer is
    /// 30 minutes and at 23:59:50 it is the last one, so a countdown reading
    /// the hour that has passed rather than the one coming, or flooring where
    /// it should ceil, is a different string rather than a differently
    /// formatted one.
    #[test]
    fn hyperliquid_counts_down_to_the_hour_the_clock_is_in() {
        assert_eq!(
            funding_countdown(Venue::Hyperliquid, stamped(0), HALF_PAST),
            "30m"
        );
        assert_eq!(
            funding_countdown(Venue::HyperliquidTestnet, stamped(0), HALF_PAST),
            "30m"
        );
        // Under a minute is "now" rather than "0m", which would sit there for
        // a whole minute reading as an hour away.
        assert_eq!(
            funding_countdown(Venue::Hyperliquid, stamped(0), 1_786_319_990),
            "now"
        );
        // And the boundary itself is the next one, not this one at zero.
        assert_eq!(
            funding_countdown(Venue::Hyperliquid, stamped(0), 1_786_320_000),
            "60m"
        );
    }

    /// Lighter publishes the stamp, so the countdown is measured from what the
    /// venue said and not from the clock's hour.
    ///
    /// The fixture stamp is deliberately off the hour. Both exchanges happen to
    /// fund on it today, so a stamp landing on :00 would let a countdown that
    /// ignored the stamp entirely pass every assertion — the two answers would
    /// be the same number. Anchored at 22:35 they are five minutes and thirty
    /// minutes, and only one of them is the venue's.
    #[test]
    fn lighter_counts_down_from_the_stamp_it_published() {
        let stamp = 1_786_314_900;
        assert_eq!(
            funding_countdown(Venue::Lighter, stamped(stamp), HALF_PAST),
            "5m"
        );
        assert_eq!(
            funding_countdown(Venue::LighterTestnet, stamped(stamp), HALF_PAST),
            "5m"
        );
        assert_ne!(
            funding_countdown(Venue::Lighter, stamped(stamp), HALF_PAST),
            funding_countdown(Venue::Hyperliquid, stamped(stamp), HALF_PAST),
            "the stamp is the anchor on one venue and ignored on the other"
        );
        // A socket quiet for three hours leaves a stamp three intervals old,
        // and the boundary it names is still ahead: rolled forward by whole
        // intervals rather than incremented once into the past.
        assert_eq!(
            funding_countdown(Venue::Lighter, stamped(stamp - 3 * 3_600), HALF_PAST),
            "5m"
        );
    }

    /// The honest half. Lighter serves the stamp on one channel and nowhere
    /// else, so every market it lists has no boundary until that channel has
    /// spoken — and a screen a position is held against says it does not know
    /// rather than drawing the hour the other venue happens to be on.
    #[test]
    fn a_venue_that_has_not_said_shows_no_countdown() {
        assert_eq!(
            funding_countdown(Venue::Lighter, stamped(0), HALF_PAST),
            "—"
        );
        assert_eq!(funding_countdown(Venue::Lighter, None, HALF_PAST), "—");
        assert_eq!(
            funding_countdown(Venue::Hyperliquid, None, HALF_PAST),
            "—",
            "no market is no market on either venue"
        );
        // And a market read over REST is exactly that market: the universe
        // request carries no funding time at all.
        assert!(
            demo_symbols_lighter().iter().all(|row| row.funding_at == 0),
            "orderBookDetails states no funding time"
        );
    }

    /// A fill written for a spreadsheet, field for field.
    ///
    /// The two columns worth the test are the last two. A row that says BTC,
    /// a size and a price is the same row on either deployment, and a testnet
    /// fill filed without saying so becomes a mainnet record the moment it is
    /// opened anywhere else — so the file names the network it was read from
    /// and whether that network's money is worth anything.
    ///
    /// Pinned whole rather than by `contains`, because the failure this guards
    /// is a column quietly dropped and every remaining column still reading
    /// correctly.
    #[test]
    fn an_exported_fill_states_the_network_it_was_read_from() {
        assert_eq!(
            fills_csv(Venue::HyperliquidTestnet, &demo_fills()),
            concat!(
                "\"time\",\"coin\",\"side\",\"size\",\"price\",\"closed_pnl\",\"trade_id\",",
                "\"venue\",\"network\"\n",
                "\"2026-08-07T15:10:00Z\",\"BTC\",\"sell\",\"0.25\",\"64010\",\"1240\",\"1\",",
                "\"Hyperliquid Testnet\",\"testnet\"\n",
                "\"2026-08-07T14:55:00Z\",\"BTC\",\"buy\",\"0.5\",\"63940\",\"0\",\"2\",",
                "\"Hyperliquid Testnet\",\"testnet\"\n",
                "\"2026-08-07T14:40:00Z\",\"BTC\",\"buy\",\"0.75\",\"63880\",\"0\",\"3\",",
                "\"Hyperliquid Testnet\",\"testnet\"\n",
            )
        );
        // The same fills off the live deployment are the same rows with the
        // two columns that matter reading differently.
        let real = fills_csv(Venue::Hyperliquid, &demo_fills());
        assert!(real.contains("\"Hyperliquid\",\"mainnet\""), "{real}");
        assert!(!real.contains("testnet"), "{real}");
    }

    /// A comma in a field must not become a column. Nothing either venue lists
    /// carries one today, which is exactly why this is a test rather than a
    /// wait: the day a symbol does, a file that split on it would file every
    /// row afterwards under the wrong heading.
    #[test]
    fn a_symbol_carrying_a_comma_or_a_quote_stays_one_field() {
        let awkward = vec![Fill {
            coin: "A,B\"C".to_owned(),
            ts: 0,
            price: 1.0,
            size: 1.0,
            buy: true,
            closed_pnl: 0.0,
            hot: false,
            tid: 9,
        }];
        let written = fills_csv(Venue::Hyperliquid, &awkward);
        assert!(
            written.contains("\"1970-01-01T00:00:00Z\",\"A,B\"\"C\",\"buy\""),
            "{written}"
        );
        // Nine columns in the header and nine in the row, whatever the coin
        // spells.
        let row = written.lines().nth(1).expect("one row");
        assert_eq!(row.matches('"').count() % 2, 0, "{row}");
    }

    /// The write itself, read back. `export_dir` answers the process temp
    /// directory under test, so a suite run leaves nothing in the reader's own
    /// folders.
    ///
    /// The testnet rather than the live network, and not for symmetry: the name
    /// is the venue's and the newest fill's hour, so the live network over
    /// `demo_fills` is the exact path the Ice test's press writes — two threads
    /// of one suite truncating and reading one file. The deployment in the name
    /// is what keeps them apart.
    #[test]
    fn writing_fills_names_the_file_it_wrote() {
        let written = write_fills_csv(Venue::HyperliquidTestnet, demo_fills());
        assert!(written.error.is_empty(), "{}", written.error);
        let path = std::env::temp_dir().join("fills-hyperliquid-testnet-20260807T151000.csv");
        assert_eq!(
            written.note,
            format!("Wrote 3 fills to {}", path.display()),
            "the app has to say where it went"
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("the file the note names"),
            fills_csv(Venue::HyperliquidTestnet, &demo_fills())
        );
        std::fs::remove_file(&path).expect("the file this test wrote");
    }

    /// Nothing to write is a refusal rather than an empty file. A header-only
    /// CSV in a downloads folder is indistinguishable from an export that
    /// silently lost its rows.
    #[test]
    fn no_fills_writes_nothing() {
        let refused = write_fills_csv(Venue::Hyperliquid, Vec::new());
        assert!(refused.note.is_empty());
        assert_eq!(refused.error, "No fills to export.");
    }

    /// A flatten is one ordinary closing order per position, and every field of
    /// each one follows from the position rather than from anything typed.
    ///
    /// The side is the interesting half: a buy closes a short, and getting it
    /// backwards doubles a position instead of closing it — which the venue
    /// would happily do, because reduce-only refuses the order rather than
    /// flipping it.
    #[test]
    fn a_flatten_closes_each_position_with_its_own_order() {
        let markets = demo_symbols();
        let act = sweep_positions(Venue::Hyperliquid, demo_positions(), markets);
        assert!(!act.cancel);
        assert!(act.orders.is_empty(), "a flatten pulls nothing");
        assert_eq!(act.act, "Close 3 positions");
        assert_eq!(act.drafts.len(), 3, "one order per position, and no more");
        assert_eq!(act.rows.len(), 3);

        // The fixture is short 30 bitcoin at a mark of 64,000, so the order
        // that closes it buys 30 and is willing to pay five per cent through.
        let btc = &act.drafts[0];
        assert_eq!(btc.coin, "BTC");
        assert!(btc.buy, "a buy closes a short");
        assert_eq!(btc.size, 30.0);
        assert_eq!(btc.price, 64_000.0 * 1.05);
        assert!(btc.reduce_only, "a close only moves towards zero");
        assert_eq!(btc.tif, Tif::Ioc, "it crosses rather than rests");
        assert!(btc.market.is_some(), "the wire names a market by its index");
        assert!(btc.refusal.is_empty());
        assert_eq!(act.rows[0], "Close BTC short 30 at up to 67,200.00");

        // And the long the other way, at a price under its mark rather than
        // over it.
        let eth = &act.drafts[1];
        assert!(!eth.buy, "a sell closes a long");
        assert_eq!(eth.size, 40.0);
        assert_eq!(eth.price, 3_540.0 * 0.95);
        assert!(eth.reduce_only);

        // Neither figure the ticket would have quoted is invented: a close
        // asks for no margin and moves towards no cliff.
        assert!(act.drafts.iter().all(|draft| draft.margin == 0.0));
        assert!(act.drafts.iter().all(|draft| draft.liquidation == 0.0));
    }

    /// A market this app cannot price is refused in the list rather than at the
    /// exchange, by the same rule a typed order is refused by.
    #[test]
    fn a_flatten_refuses_a_market_it_cannot_margin() {
        let mut held = demo_positions();
        held.truncate(1);
        held[0].coin = "xyz:NVDA".to_owned();
        let act = sweep_positions(Venue::Hyperliquid, held, demo_symbols());
        assert_eq!(act.drafts.len(), 1);
        assert!(
            act.drafts[0]
                .refusal
                .contains("clearinghouse this app cannot read"),
            "got {}",
            act.drafts[0].refusal
        );
    }

    /// The cancel half: every resting order named once, in the order the panel
    /// listed them.
    #[test]
    fn a_cancel_all_names_every_resting_order() {
        let act = sweep_orders(Venue::Hyperliquid, demo_orders());
        assert!(act.cancel);
        assert!(act.drafts.is_empty(), "a cancel places nothing");
        assert_eq!(act.act, "Cancel 2 resting orders");
        assert_eq!(
            act.rows,
            vec![
                "BTC buy 1.5 at 63,600.00".to_owned(),
                "BTC sell 0.8 at 64,440.00".to_owned(),
            ]
        );
        assert_eq!(
            sweep_orders(Venue::Hyperliquid, Vec::new()).act,
            "Cancel 0 resting orders"
        );
    }

    /// Why a panel-wide control is dead, and in which order the two reasons are
    /// asked. Custody first: a locked session cannot cancel one order or seven,
    /// and "nothing to cancel" over a full list is a second and wrong reason.
    #[test]
    fn a_panel_wide_refusal_asks_custody_before_the_panel() {
        let locked = "Unlock on Settings before sending an order.".to_owned();
        assert_eq!(sweep_refused(locked.clone(), 7, true), locked);
        assert_eq!(sweep_refused(locked.clone(), 0, true), locked);
        assert_eq!(sweep_refused(String::new(), 7, true), "");
        assert_eq!(
            sweep_refused(String::new(), 0, true),
            "No resting orders to cancel."
        );
        assert_eq!(
            sweep_refused(String::new(), 0, false),
            "No open positions to close."
        );
        // The reason travels in the name, because a header row has no width
        // for a sentence.
        assert_eq!(
            sweep_label(7, true, locked.clone()),
            "Cancel 7 resting orders — Unlock on Settings before sending an order."
        );
        assert_eq!(
            sweep_label(1, false, String::new()),
            "Close 1 position, one confirmation"
        );
    }
}
