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

use std::future::Future;
use std::pin::Pin;

use smol::channel::Receiver;

use crate::hyperliquid::{
    Account, Fill, HL_CANONICAL, HlError, MarketTick, Order, SymbolRow, Tape, hl_account,
    hl_candles, hl_fill_feed, hl_history, hl_market_feed, hl_orders, hl_symbols,
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
        attaches_levels: true,
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
        attaches_levels: true,
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
    format!(
        "{} attaches no levels to an entry. Its API takes them as separate orders once the \
         position exists, which this app does not place.",
        venue_name(venue)
    )
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
    use crate::hyperliquid::{hl_symbols, tape_new};
    // The book is not on `Reads` — the app never asks a venue for one — so the
    // live seam test reaches the adapter's own read directly.
    use crate::lighter::lighter_book;

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
}
