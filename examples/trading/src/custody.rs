//! Custody as the app performs it: the platform prompt, the venue's word on
//! the approval, and the pure model in `session.rs` folding the two into one
//! state the screen can draw.
//!
//! `session.rs` is pure on purpose — it reads no clock, no keychain and no
//! network, so "may this key sign?" is a question with a tested answer. That
//! makes it a model and not a seam. This module is the seam: it performs the
//! two effects the model consumes as events, in the order the model's rules
//! expect, and hands back the state that came out. Nothing here decides
//! anything the model could have decided.
//!
//! # What the user actually does
//!
//! Two acts, and the app cannot perform the one in the middle.
//!
//! 1. **Enrol.** The app generates an agent key and puts its secret in the
//!    platform keychain under this account and this deployment. It shows the
//!    agent's address.
//! 2. **Approve.** The user approves that address from the wallet that owns
//!    the account — in the exchange's own interface, or anywhere else that
//!    can sign. **This app never does this step**, and could not: an
//!    `approveAgent` is signed by the master wallet, which is the one key this
//!    whole design exists to avoid holding. `signing.rs` builds the action and
//!    says the same thing in its own header.
//! 3. **Unlock.** Touch ID releases the agent secret, the app asks the venue
//!    which of this account's keys are live, and a listing naming ours is what
//!    `Session::Ready` is made of.
//!
//! The middle step being somebody else's is not a gap in the app. It is the
//! property: what this process can reach places and cancels orders, cannot
//! withdraw, and stops working on a date the exchange chose.
//!
//! # Why the keychain item is keyed by network
//!
//! An agent key is approved on one deployment. The same address on mainnet and
//! on testnet holds two different accounts, approves two different keys, and a
//! secret read back under the wrong one is a key the venue has never heard of —
//! which surfaces as an order refused for a signer nobody recognises, long
//! after the mistake. So the keychain account is the deployment and the
//! address, never the address alone.
//!
//! And the exchange too, which is the half a single-venue app never needs. Both
//! venues have a mainnet, one address is read at both, and their keys are
//! unrelated — different curves, even — so an item named for the deployment
//! alone would have the second enrolment overwrite the first and afterwards
//! hand each venue the other's secret. `Signing::key` in `venue.rs` is what
//! spells the whole name, and `item` below is the address glued to it.
//!
//! # What CI holds, and what needs a Mac
//!
//! Everything decidable without a Keychain is decided in CI. `session.rs` runs
//! its state machine exhaustively — no run of events reaches `Ready` on a dead
//! key or an unopened account — the tests below hold this seam's projections
//! and its two refusals, and `tests/custody.ice` drives the panel through every
//! state a preset can put it in. A Linux runner reaches all of that, because
//! none of it touches a keychain: `PlatformKeystore` on a build without one
//! answers `Unavailable`, which is a state with a test rather than a gap.
//!
//! What no runner reaches is the sheet. `security-framework` is a
//! `cfg(target_os = "macos")` dependency, so the macOS jobs *compile* this
//! path and nothing executes it — a CI runner has no window server to raise a
//! Touch ID sheet in front of, and no enrolled finger to answer it with. The
//! nine experiments `session.rs` lists on its `impl Keystore` are still owed,
//! and this wiring adds four more that only exist now that something calls it:
//!
//! 1. **UNLOCK raises one sheet, not two.** `load` is the only guarded read
//!    this path makes, so it should be one. Two would mean something else is
//!    touching the item, and the fix is a shared `LAContext` — a dependency
//!    this change would not add on a guess.
//! 2. **Cancelling the sheet leaves the panel offering to try again**, with
//!    "Touch ID was cancelled" beside a live button — never the dead button and
//!    red platform sentence a fault gets. This is the distinction the whole
//!    seam is shaped around and the one place it is visible.
//! 3. **The round trip.** NEW KEY on a first run, the address it prints
//!    approved from the account's own wallet at the exchange, then UNLOCK
//!    reaching `Ready` with a window the exchange assigned. Every step but the
//!    middle one is this app's; the middle one proves the middle one is not.
//! 4. **NEW KEY over an existing enrolment costs a sheet and keeps the old
//!    secret when the add fails.** `session.rs` reads the old bytes back before
//!    it deletes them precisely so a failed replace is survivable, and that
//!    read is a guarded read — so expect a sheet on re-enrolment, none on a
//!    first run, and the previous key still loading afterwards.
//!
//! Until a person on a Mac reports those, the honest claim is that this seam's
//! logic is tested and its platform half is compiled, reviewed and unrun.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use crate::hyperliquid::{HlError, Order, SymbolRow, fmt_size, hl_agents, hl_cancel, hl_place};
use crate::lighter::{lighter_account_index, lighter_api_keys, lighter_cancel, lighter_place};
use crate::lighter_sign::{PrivateKey, Resting};
/// Ice names one namespace per Rust module, and custody is the one it talks
/// to — so the type it holds is published from here. `session.rs` stays where
/// the rules are and stays out of the app's vocabulary.
pub use crate::session::Session;
use crate::session::{
    AgentKey, Event, Held, Keystore, PlatformKeystore, Secret, Unlock, account, agent, can_trade,
    step,
};
use crate::signing::{self, Wallet};
use crate::venue::{Draft, Network, Signing, venue_list, venue_name};
use crate::{Tif, Venue};

/// What one act of custody produced: where the session now stands, and the
/// sentence the panel owes about it.
///
/// The note is not an error channel. It carries the outcomes that are answers
/// rather than faults — a declined sheet, an account whose key nobody has
/// approved yet — which the state alone cannot distinguish: `step` maps a
/// decline back to `Session::Locked`, which is also where the app sits before
/// anybody has asked for anything. Without a sentence beside it, cancelling
/// Touch ID and never pressing the button look identical on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub session: Session,
    pub note: String,
}

impl Entry {
    fn plain(session: Session) -> Self {
        Self {
            session,
            note: String::new(),
        }
    }

    fn saying(session: Session, note: &str) -> Self {
        Self {
            session,
            note: note.to_owned(),
        }
    }
}

/// A custody act that could not be completed for a reason that is neither the
/// user's answer nor a state the model has: the venue would not say which keys
/// are live.
///
/// Kept as a failure route rather than folded into `Entry.note` because it is
/// the one outcome here that is worth retrying unchanged and that belongs in
/// the app's own alarm line beside every other read that could not reach an
/// exchange. Every *other* refusal is a state, because `session.rs` has one
/// for each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustodyFault {
    pub message: String,
}

impl CustodyFault {
    fn new(message: String) -> Self {
        Self { message }
    }
}

// ---------------------------------------------------------------------------
// What the session may sign with, while it may sign.
//
// **The owner's decision, 2026-08-09, implemented here.** The key is held in
// memory for the session's `Ready` window — one platform prompt per unlock, not
// one per order. What that bought convenience with is stated where it was
// spent: a sheet per order would have made every single order carry its own
// proof of presence, and the confirmation step in front of send is now the
// per-order safety and the whole of it. Weakening that confirm — a "don't ask
// again", a confirm that does not restate the priced figures, a path that sends
// without one — spends a guarantee this decision already spent once, and
// nothing is left underneath.
//
// Three rules hold the retention, and each is here rather than in a comment
// somewhere else:
//
// 1. **Outside Ice state.** Ice state is cloned, captured into fixtures and
//    printed by tests; a key that could reach any of those has already leaked.
//    It lives in this module, behind this seam, and crosses no extern.
// 2. **Exactly the `Ready` window.** `step` is the one thing that decides
//    whether a session may sign, so the drop hangs off `step`: every real
//    transition goes through `advance`, which drops the keys on the way past
//    whenever what comes back is not `Ready`. Lock and expiry need no rule of
//    their own — they are already transitions — and a change of address is held
//    by `signing_key`, which reaches nothing for a session that is not the one
//    the vault was unlocked for. Switching *network* is deliberately not on
//    that list any more: see `Vault`.
// 3. **`can_trade` is the only gate, held by the compiler.** The key is
//    reachable only through `signing_key`, which returns `None` unless a
//    `Ready` session and a clock said yes. A path that reaches it without
//    asking does not typecheck, because nothing else in this file exposes it.
// ---------------------------------------------------------------------------

/// The private half this session is holding, for whichever scheme its network
/// signs with.
///
/// One enum rather than two stores: the retention rules above are about *a
/// key*, not about a venue, and two parallel holders would be two lifetimes to
/// keep in agreement. Neither variant is `Clone` — `Wallet` and `PrivateKey`
/// both refuse it — so the `Arc` below is genuinely the only copy of the bytes.
enum HeldKey {
    /// Hyperliquid's agent wallet, approved by the account's master key.
    Agent(Wallet),
    /// Lighter's registered API key, with the two indices that name it on the
    /// wire. The address never appears in a Lighter transaction; these do.
    ApiKey {
        key: PrivateKey,
        account: i64,
        index: u8,
    },
}

impl fmt::Debug for HeldKey {
    /// Neither variant may print itself, and this exists so that a future
    /// `#[derive(Debug)]` on something holding one cannot leak by accident.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HeldKey(<redacted>)")
    }
}

/// Every key this address has enrolled, by the network it signs for.
///
/// **Decided by the repository owner, 2026-08-10: one unlock activates every
/// enrolled network of the active address.** Switching network is no longer an
/// authentication boundary — the key set survives it, and only locking, the
/// window closing, and changing address take it away.
///
/// What that spends is stated so it cannot be forgotten. A switch used to be a
/// second gate: moving from a test deployment to a live one re-asked for a
/// finger, and now it does not. **What remains between a reader and an order on
/// the wrong network is the confirmation panel and the REAL MONEY / TESTNET
/// kind stated inside it.** That panel is now carrying two decisions' worth of
/// safety; anyone loosening it is loosening both.
///
/// What it does *not* change: the keychain still files one item per exchange,
/// deployment and address, because a key approved on one deployment is unknown
/// on the others and a secret read back under the wrong name is a key the venue
/// has never heard of. This is the in-memory set only.
///
/// ponytail: on macOS a *read* is what raises the sheet, so an address enrolled
/// on several networks may cost more than one prompt during the single unlock
/// that fills this. The decided behaviour — no prompt on a switch — holds
/// either way, because every sheet happens at the unlock. Collapsing them to
/// one needs a shared `LAContext` passed as `kSecUseAuthenticationContext`,
/// which is the open question `session.rs` already records and not a dependency
/// to add on a guess. The whole macOS path is compiled and unrun anyway.
struct Vault {
    /// The account these keys belong to. Held so a change of address is a
    /// change of vault rather than a rule somebody has to remember.
    address: String,
    /// One entry per network this address has a usable key for. A list rather
    /// than a map because there are four networks in the whole registry and
    /// `Venue` is an Ice-declared enum with no `Hash`; a linear scan of four is
    /// not a thing to buy a derive for.
    keys: Vec<(Venue, Arc<HeldKey>)>,
}

fn vault() -> &'static Mutex<Option<Vault>> {
    static HELD: OnceLock<Mutex<Option<Vault>>> = OnceLock::new();
    HELD.get_or_init(Mutex::default)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The one place a session transition happens, and therefore the one place a
/// key is dropped.
///
/// Every event the app raises goes through here. `step` decides, and whatever
/// it decides that is not `Ready` takes the key with it — so the key's lifetime
/// is the `Ready` window by construction rather than by four call sites
/// remembering. Dropping is the wipe: `Wallet` holds a `k256::SigningKey` which
/// zeroizes its own scalar, and `PrivateKey` overwrites its scalar in `Drop`.
///
/// The preset builders below deliberately do *not* come through here. They
/// drive `step` directly to make fixtures, and a fixture must never be able to
/// put a key in the hand of a screenshot — a `Ready` built that way finds this
/// store empty and can sign nothing, which is exactly right.
fn advance(state: Session, event: Event) -> Session {
    let next = step(state, event);
    if !matches!(next, Session::Ready { .. }) {
        // Taken out of the store before it is dropped, so the store is empty
        // for the whole of the destructor's run rather than after it. Every
        // network's key goes together: they were released by one prompt, so
        // they are held and forgotten as one thing.
        drop(lock(vault()).take());
    }
    next
}

/// The key this session may sign with, or nothing.
///
/// The gate is inside rather than beside: `can_trade` is asked here, with the
/// clock, so no caller can reach the key by forgetting to ask. It is asked
/// again on every send rather than once at unlock because a window closes on
/// the exchange's schedule — a laptop that slept through an expiry has a
/// `Ready` in state and no right to sign.
///
/// An `Arc` rather than a borrow because a send is asynchronous: the transition
/// that ends the session can land while an order is in flight, and an order
/// already on the wire cannot be un-sent. The clone keeps that one send's key
/// alive until it finishes and no longer; the store's own copy is gone the
/// moment `advance` says so, so nothing new can be signed with it.
fn signing_key(venue: Venue, session: &Session, now_s: i64) -> Option<Arc<HeldKey>> {
    if !can_trade(session, millis(now_s)) {
        return None;
    }
    let vault = lock(vault());
    let vault = vault.as_ref()?;
    // The address is the third thing that ends a key's life, and it is checked
    // here rather than left to whichever handler changes one. A vault belongs
    // to the account it was unlocked for; a session for any other account
    // reaches nothing, so "changing address forgets the keys" is a property of
    // this accessor rather than a rule four call sites have to remember.
    if account(session) != Some(vault.address.as_str()) {
        return None;
    }
    // Per network even though the prompt was not: a session may sign, and this
    // network is one it holds a key for or it is not. A network the address has
    // never enrolled reads as needing enrolment rather than as a session that
    // cannot trade, which is a different sentence and a different fix.
    vault
        .keys
        .iter()
        .find(|(held, _)| *held == venue)
        .map(|(_, key)| Arc::clone(key))
}

/// Milliseconds, which is the unit the exchange reports `validUntil` in and
/// therefore the unit the whole model works in.
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64)
}

/// The view holds seconds — one `clock`, ticked with the universe — so every
/// projection below takes seconds and converts here rather than each caller
/// remembering which unit it is holding.
fn millis(now_s: i64) -> i64 {
    now_s.saturating_mul(1_000)
}

/// What the keychain files this network's secret under.
///
/// The network rather than the address alone, because a key belongs to one
/// deployment *of one exchange*: see the module header for the deployment half,
/// and `Signing::key` for why the exchange is in the name too.
fn item(venue: Venue, address: &str) -> String {
    format!(
        "{}:{}",
        Network::of(venue).signing.key(),
        address.trim().to_lowercase()
    )
}

/// How long this app holds a Lighter key before asking for it again.
///
/// Hyperliquid's window is the exchange's: `extraAgents` reports a
/// `validUntil` and the key stops working there whatever this app thinks.
/// Lighter's is not — a registered API key has no expiry, and the venue would
/// let this process sign with one until somebody deregisters it. So this window
/// is the app's own, and is named as such on the panel: eight hours, the same
/// ceiling `lighter_sign.rs` already refuses to mint a read token past, after
/// which the key is dropped and the reader unlocks again.
///
/// The shorter of "what the venue allows" and "what this app will hold" is the
/// one that should govern, and on this venue the venue allows everything.
const APP_WINDOW_MS: i64 = 8 * 60 * 60 * 1_000;

/// Generate the private half this network's scheme signs with, and answer the
/// public half the account's owner has to register.
///
/// Both halves of the app's job in one place, because they are one act: what
/// goes to the keychain is the secret, what goes to the screen is the public
/// name of it, and nothing else ever holds either. The two schemes differ only
/// in the curve and in what "the public half" is called — an Ethereum address
/// on one, a compressed ECgFp5 point on the other.
///
/// The retry is `Wallet::generate`'s and for its reason: both constructors
/// reject a scalar that is zero or out of range, and a generator that returned
/// "sometimes" would be worse than one that loops on astronomical odds.
fn generate(scheme: Signing) -> Result<(Vec<u8>, String), String> {
    for _ in 0..64 {
        let mut bytes = vec![
            0u8;
            if matches!(scheme, Signing::Eip712(_)) {
                32
            } else {
                40
            }
        ];
        if getrandom::fill(&mut bytes).is_err() {
            return Err("this machine would not produce randomness for a key".to_owned());
        }
        match scheme {
            Signing::Eip712(_) => {
                let fixed: [u8; 32] = bytes[..].try_into().expect("32 bytes");
                if let Ok(wallet) = Wallet::from_secret(&fixed) {
                    return Ok((bytes, wallet.address().to_string()));
                }
            }
            Signing::ApiKey(_) => {
                if let Ok(key) = PrivateKey::from_hex(&hex::encode(&bytes)) {
                    return Ok((bytes, hex::encode(key.public_key())));
                }
            }
        }
    }
    Err("could not generate a usable key".to_owned())
}

/// What the reader has to do with the public half, which is the one step of
/// enrolment this app cannot perform.
///
/// Different words per scheme because they are different acts at different
/// places: Hyperliquid approves an *address* as an API wallet, Lighter
/// registers a *public key* at an api-key slot. Naming the act wrongly sends
/// somebody to the wrong screen with the right string.
fn enrolment_note(venue: Venue, scheme: Signing, public: &str) -> String {
    let network = venue_name(venue);
    match scheme {
        Signing::Eip712(_) => format!(
            "Approve {public} as an API wallet on {network} from the wallet that owns this \
             account, then unlock. This app cannot approve it: that signature is the account's \
             own key, which is the one key it will never hold.",
        ),
        Signing::ApiKey(_) => format!(
            "Register this public key as an API key for this account on {network}, from the \
             wallet that owns it, then unlock — the app finds which slot you used. This app \
             cannot register it: that signature is the account's own key. {public}",
        ),
    }
}

/// Generate a key, hand its secret to the platform keychain, and report the
/// public half the account's owner now has to register.
///
/// The key is generated here and never leaves: what goes to the keychain is the
/// secret, what goes to the screen is the public half, and the account that
/// owns it registers that elsewhere. Replacing an existing enrolment is the
/// keystore's problem and it solves it by preserving what it replaces — see
/// `session.rs`, which reads the old bytes back before it deletes them.
pub async fn enrol_agent(venue: Venue, address: String) -> Result<Entry, CustodyFault> {
    let scheme = Network::of(venue).signing;
    let address = address.trim().to_owned();
    if address.is_empty() {
        return Ok(Entry::saying(
            Session::Locked,
            "A key belongs to one account, so connect an address before making one.",
        ));
    }

    smol::unblock(move || {
        // Neither signer publishes a way to read its scalar back out, so the
        // only two places these bytes are ever seen are the keychain and this
        // frame. What crosses back to the screen is the public half.
        let (bytes, public) = match generate(scheme) {
            Ok(made) => made,
            Err(reason) => return Ok(Entry::plain(Session::Unavailable { reason })),
        };
        match PlatformKeystore.store(&item(venue, &address), &Secret::new(bytes)) {
            // A keystore that cannot hold a secret is not a thing to ask again:
            // either this build has none, or the one this machine has failed
            // for a reason no sheet the user answers will change. `session.rs`
            // has a state that says exactly that and that no event but `Lock`
            // leaves.
            Err(failure) => Ok(Entry::plain(Session::Unavailable {
                reason: failure.message,
            })),
            Ok(()) => Ok(Entry::saying(
                Session::Locked,
                &enrolment_note(venue, scheme, &public),
            )),
        }
    })
    .await
}

/// Raise the platform's prompt, and on the far side of it ask the venue whether
/// the key it released may sign for this account.
///
/// The order is the model's, not a preference. `Prompt` puts the session in
/// `Unlocking`, which is the only state an answer is accepted in — a slow Touch
/// ID landing after the user locked the app must not re-open it — and only an
/// `Approved` carrying a window can reach `Ready`. Nothing here shortcuts to
/// `Ready`, and `session.rs` would refuse it if it tried.
///
/// Both venues take the same road and answer the same question with different
/// reads: Hyperliquid lists the addresses approved as API wallets for the
/// account, Lighter lists the public keys registered against it. Finding ours
/// in that listing is what `Ready` is made of, either way — and on Lighter the
/// listing also answers *which slot*, so the reader is never asked for an index
/// the venue already knows.
pub async fn unlock_agent(venue: Venue, address: String) -> Result<Entry, CustodyFault> {
    let address = address.trim().to_owned();
    if address.is_empty() {
        return Ok(Entry::saying(
            Session::Locked,
            "A key belongs to one account, so connect an address before unlocking.",
        ));
    }

    let opened = {
        let address = address.clone();
        // Reading is what raises the sheet, and the sheet blocks — so it is off
        // the executor's thread rather than on it.
        smol::unblock(move || read_key(venue, &address)).await
    };
    let (held, loaded) = match opened {
        Opened::Refused(entry) => return Ok(entry),
        Opened::Held(session, loaded) => (session, loaded),
    };

    // Past here the app is `Unlocked`: it knows whose account this is and can
    // sign nothing. What turns that into `Ready` is the venue's own listing.
    let now = now_ms();
    let (key, signer) = match approved(venue, &address, loaded, now).await {
        Ok(pair) => pair,
        Err(Denied::Refused(note)) => return Ok(Entry::saying(held, &note)),
        Err(Denied::Fault(message)) => return Err(CustodyFault::new(message)),
    };

    // And every other network this address has enrolled, released by the same
    // prompt. A network that is not enrolled, whose key the venue no longer
    // lists, or whose exchange will not answer right now is simply not held —
    // it reads as needing enrolment when the reader reaches it, which is what
    // it would have read as before this unlock too. None of those is a reason
    // to refuse an unlock the asked network already answered.
    let mut keys = vec![(venue, Arc::new(signer))];
    for other in venue_list() {
        if other == venue {
            continue;
        }
        let secret = {
            let address = address.clone();
            smol::unblock(move || secret_for(other, &address)).await
        };
        let Some(secret) = secret else {
            continue;
        };
        if let Ok((_, signer)) = approved(other, &address, secret, now).await {
            keys.push((other, Arc::new(signer)));
        }
    }

    // The app's own ceiling, where the set includes a network that has none of
    // the venue's. Hyperliquid stops honouring a key on a date it chose;
    // Lighter never does, so `APP_WINDOW_MS` is this app's own limit on holding
    // one — and a session that reached `Ready` through Hyperliquid's much
    // longer window must not quietly extend it over a Lighter key it is also
    // holding. The shorter bound governs, which is the whole of the rule.
    let key = match keys
        .iter()
        .any(|(held, _)| matches!(Network::of(*held).signing, Signing::ApiKey(_)))
    {
        true => AgentKey {
            expires_at: key.expires_at.min(now.saturating_add(APP_WINDOW_MS)),
            ..key
        },
        false => key,
    };

    // The keys go into the store *before* the transition, and `advance` takes
    // them straight back out if what comes back is not `Ready` — so there is no
    // ordering of these two lines that leaves a key held by a session which may
    // not sign.
    *lock(vault()) = Some(Vault {
        address: address.clone(),
        keys,
    });
    Ok(Entry::plain(advance(held, Event::Approved { key, now })))
}

/// Why a network's key was not admitted, and where the answer belongs.
///
/// Two routes because they are two different events. A venue that says this key
/// is not approved has *answered*, and the answer is a sentence for the panel;
/// a venue that would not answer at all is a read that failed like any other
/// and belongs in the app's alarm line beside the rest. Collapsing them would
/// put "not approved" on screen for an exchange that was merely unreachable.
enum Denied {
    Refused(String),
    Fault(String),
}

/// Ask one network whether the key this address holds for it may sign, and
/// build what the session and the store each need from the answer.
///
/// One path for both schemes and for every network, asked or not: the question
/// is the same — is this key listed for this account — and only the read that
/// answers it differs. A second, more forgiving copy for the networks nobody
/// asked about is how the two would come to disagree about what "approved"
/// means.
async fn approved(
    venue: Venue,
    address: &str,
    loaded: Loaded,
    now: i64,
) -> Result<(AgentKey, HeldKey), Denied> {
    let fault = |failure: HlError| Denied::Fault(failure.message);
    match (Network::of(venue).signing, loaded) {
        (Signing::Eip712(chain), Loaded::Agent(wallet)) => {
            let listed = hl_agents(chain, address.to_owned()).await.map_err(fault)?;
            let ours = wallet.address().to_string();
            let Some(&(_, expires_at)) = listed
                .iter()
                .find(|(listed, _)| listed.eq_ignore_ascii_case(&ours))
            else {
                return Err(Denied::Refused(format!(
                    "{ours} is not an approved API wallet for this account on {}. Approve it \
                     from the wallet that owns the account, then unlock again. The account can \
                     be read either way.",
                    venue_name(venue),
                )));
            };
            Ok((
                AgentKey {
                    address: ours,
                    account: address.to_owned(),
                    // The venue reports no approval time. What this app knows
                    // is when it read one, and nothing is inferred from that.
                    approved_at: now,
                    expires_at,
                },
                HeldKey::Agent(wallet),
            ))
        }
        (Signing::ApiKey(zone), Loaded::ApiKey(key)) => {
            let Some(account) = lighter_account_index(zone, address.to_owned())
                .await
                .map_err(fault)?
            else {
                return Err(Denied::Refused(format!(
                    "This address has no account on {} yet, so there is nothing for a key to \
                     sign for. Fund one there first — the app reads whatever it finds either \
                     way.",
                    venue_name(venue),
                )));
            };
            let ours = hex::encode(key.public_key());
            let listed = lighter_api_keys(zone, account).await.map_err(fault)?;
            let Some((index, _)) = listed
                .into_iter()
                .find(|(_, public)| public.eq_ignore_ascii_case(&ours))
            else {
                return Err(Denied::Refused(format!(
                    "This key is not registered against account {account} on {}. Register its \
                     public key from the wallet that owns the account, then unlock again — the \
                     app finds which slot you used. The account can be read either way. {ours}",
                    venue_name(venue),
                )));
            };
            Ok((
                AgentKey {
                    // The public key is what the venue lists and what a reader
                    // can compare against the panel, so it is what the session
                    // names its key by: the analogue of the agent address.
                    address: ours,
                    account: address.to_owned(),
                    approved_at: now,
                    // The venue puts no expiry on a registered key, so this
                    // window is the app's own. See `APP_WINDOW_MS`.
                    expires_at: now.saturating_add(APP_WINDOW_MS),
                },
                HeldKey::ApiKey {
                    key,
                    account,
                    index,
                },
            ))
        }
        // A secret stored for one scheme and read back under another, which is
        // a keychain item left over from a previous shape of this app.
        // `load_key` parses by the network's own scheme so the pair always
        // agrees; this exists because the compiler cannot know that.
        _ => Err(Denied::Refused(
            "the stored secret is not a key for this network; make a new one to replace it"
                .to_owned(),
        )),
    }
}

/// A secret stored for one scheme and read back under another, which is a
/// keychain item left over from a previous shape of this app. `read_key` parses
/// by the network's own scheme so the pair always agrees — this exists because
/// the compiler cannot know that and must not be told otherwise.
#[allow(dead_code)]
fn mismatched() -> Entry {
    Entry::plain(Session::Unavailable {
        reason: "the stored secret is not a key for this network; make a new one to replace it"
            .to_owned(),
    })
}

/// The private half the keychain released, before the venue has said whether it
/// may sign. Not a `HeldKey` yet: Lighter's needs the two indices only the
/// venue's own listing can supply.
enum Loaded {
    Agent(Wallet),
    ApiKey(PrivateKey),
}

/// What reading the keychain produced: either a session holding this account
/// and the key behind it, or a finished answer to hand straight back.
enum Opened {
    Held(Session, Loaded),
    Refused(Entry),
}

fn read_key(venue: Venue, address: &str) -> Opened {
    let asked = advance(Session::Locked, Event::Prompt);
    let answered = |outcome: Unlock| {
        advance(
            asked.clone(),
            Event::Unlocked {
                outcome,
                account: address.to_owned(),
            },
        )
    };

    match PlatformKeystore.load(&item(venue, address)) {
        // Not a fault and not a decline: nothing has ever been stored for this
        // account here, and a second sheet would fetch the same answer forever.
        // Enrolling is what changes it.
        Ok(Held::Missing) => Opened::Refused(Entry::saying(
            answered(Unlock::Unenrolled),
            "No key on this Mac for this account and this network yet. Make one, register it \
             with the account's own wallet, then unlock.",
        )),
        // The user said no, or could not prove it was them. Asking again is a
        // reasonable thing for them to do, which is the whole reason this is
        // not `Unavailable`.
        Ok(Held::Declined) => Opened::Refused(Entry::saying(
            answered(Unlock::Locked),
            "Touch ID was cancelled, so nothing was released. Unlock again when you are ready.",
        )),
        Err(failure) => {
            Opened::Refused(Entry::plain(answered(Unlock::Unavailable(failure.message))))
        }
        // The item is there and is not a key for this scheme. No sheet answers
        // that, so it reads as the keystore being unusable rather than as a
        // refusal — and the scheme is what says which shape to expect, so a
        // Lighter item can never be read as a Hyperliquid one.
        Ok(Held::Secret(secret)) => match load_key(Network::of(venue).signing, secret.expose()) {
            Err(reason) => Opened::Refused(Entry::plain(answered(Unlock::Unavailable(reason)))),
            Ok(loaded) => Opened::Held(answered(Unlock::Platform), loaded),
        },
    }
}

/// The same read for a network nobody asked about: the secret, or nothing.
///
/// No session events and no sentences. Whether this address has enrolled this
/// network at all is not a question the unlock is answering, so a missing item,
/// a declined sheet and an unusable secret are all the same answer here — this
/// network is not one the vault will hold.
fn secret_for(venue: Venue, address: &str) -> Option<Loaded> {
    match PlatformKeystore.load(&item(venue, address)) {
        Ok(Held::Secret(secret)) => load_key(Network::of(venue).signing, secret.expose()).ok(),
        _ => None,
    }
}

/// Turn the bytes the keychain released back into the key this network signs
/// with, refusing anything that is not one.
///
/// Strict on both length and validity, which is a trust boundary rather than
/// fussiness: a truncated item that reduced into *some* scalar would be a key
/// the venue has never heard of, and the first thing to say so would be a
/// rejected order.
fn load_key(scheme: Signing, bytes: &[u8]) -> Result<Loaded, String> {
    let wrong = || "the stored secret is not a key for this network; make a new one".to_owned();
    match scheme {
        Signing::Eip712(_) => {
            let bytes: [u8; 32] = bytes.try_into().map_err(|_| wrong())?;
            Wallet::from_secret(&bytes)
                .map(Loaded::Agent)
                .map_err(|failure| failure.message)
        }
        Signing::ApiKey(_) => {
            if bytes.len() != 40 {
                return Err(wrong());
            }
            PrivateKey::from_hex(&hex::encode(bytes))
                .map(Loaded::ApiKey)
                .map_err(|failure| failure.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Sending, which is the only thing in this app that spends money.
// ---------------------------------------------------------------------------

/// Why this order cannot be sent, or nothing when it can.
///
/// One sentence and one decision, read by the button that disables itself and
/// printed under it. Composed here rather than in the view because the view
/// would have to `&&` the conditions together, and a condition somebody forgot
/// to add is a live button over an order that should never have been offered.
///
/// The session comes first because it is the precondition rather than the
/// order: no key means no order at all, whatever is typed. What the ticket is
/// describing comes second, because it changes with every keystroke and its
/// sentences are already beside the controls that caused them.
pub fn order_gate(venue: Venue, session: Session, now_s: i64, draft: Draft) -> String {
    match trade_refusal(venue, session, now_s) {
        locked if !locked.is_empty() => locked,
        _ => draft.refusal,
    }
}

/// Why this session may not sign anything right now, or nothing when it may.
///
/// The session half of the send's refusal, on its own, because pulling a
/// resting order asks nothing of the ticket: a half-typed size must never be a
/// reason a resting order cannot be cancelled. Both controls read the same
/// sentence about custody and only the send reads the order's own.
pub fn trade_refusal(venue: Venue, session: Session, now_s: i64) -> String {
    if can_trade(&session, millis(now_s)) {
        return String::new();
    }
    locked_out(venue, session)
}

/// Why a session that cannot trade cannot trade, said for the send button
/// rather than for the unlock one.
///
/// A different question from `session_refusal`, which answers "why is UNLOCK
/// dead". This answers "why can this order not go", and the two differ exactly
/// where it matters: a `Locked` session leaves UNLOCK live and says nothing
/// there, while here it is the whole reason the send is dead.
fn locked_out(venue: Venue, session: Session) -> String {
    match session {
        Session::Unavailable { reason } => reason,
        Session::Unlocking => "Waiting for the platform's prompt.".to_owned(),
        Session::Expired { .. } => {
            "This key's window has closed. Unlock again before sending an order.".to_owned()
        }
        Session::Unenrolled => format!(
            "No key on this Mac for this account on {}. Make one on Settings, register it with \
             the account's own wallet, then unlock.",
            venue_name(venue),
        ),
        Session::Unlocked { .. } => format!(
            "This account has no key registered on {} yet. Settings says what to register and \
             where.",
            venue_name(venue),
        ),
        Session::Locked | Session::Ready { .. } => {
            "Unlock on Settings before sending an order.".to_owned()
        }
    }
}

/// Send the order the reader confirmed.
///
/// Takes the draft rather than the ticket's fields, and that is the whole
/// design: the draft is what the confirmation restated, so what goes to the
/// exchange is what was agreed to and not a second reading of a screen that has
/// moved since. Nothing here re-derives a price, a size or a side.
///
/// The gate is asked again here rather than trusted from the button. A press
/// and a send are two moments, and a window that closed between them is exactly
/// the case a screen-level check cannot see.
pub async fn submit_order(
    venue: Venue,
    session: Session,
    now_s: i64,
    draft: Option<Draft>,
) -> Result<String, HlError> {
    // A send with no confirmation behind it is not a send. The handler cannot
    // reach here without one, and this is the arm that keeps that true rather
    // than assuming it.
    let Some(draft) = draft else {
        return Err(HlError::new(
            "There is no confirmed order to send.".to_owned(),
        ));
    };
    let refused = order_gate(venue, session.clone(), now_s, draft.clone());
    if !refused.is_empty() {
        return Err(HlError::new(refused));
    }
    let key = signing_key(venue, &session, now_s)
        .ok_or_else(|| HlError::new(locked_out(venue, session.clone())))?;
    let order = Order {
        oid: 0,
        coin: draft.coin.clone(),
        buy: draft.buy,
        price: draft.price,
        size: draft.size,
        ts: 0,
    };
    match (Network::of(venue).signing, &*key) {
        (Signing::Eip712(chain), HeldKey::Agent(wallet)) => {
            let market = draft
                .market
                .clone()
                .ok_or_else(|| HlError::new("This market is not loaded here.".to_owned()))?;
            let resting = hl_place(
                chain,
                wallet,
                &market,
                order,
                draft.reduce_only,
                hl_tif(draft.tif),
            )
            .await?;
            // The venue answers a resting order with the id it rests under, and
            // an order that filled on arrival with no id at all — so this says
            // which happened rather than one word for both.
            Ok(match resting.first() {
                Some(oid) => format!("{} is resting as order {oid}.", acted(&draft)),
                None => format!("{} filled on arrival.", acted(&draft)),
            })
        }
        (
            Signing::ApiKey(zone),
            HeldKey::ApiKey {
                key,
                account,
                index,
            },
        ) => {
            let placed = lighter_place(
                zone,
                key,
                *account,
                *index,
                &draft.coin,
                order,
                draft.reduce_only,
                lighter_resting(draft.tif),
            )
            .await?;
            // Deliberately *not* "resting". This venue answers a submission
            // with a transaction hash and a predicted execution time — a
            // receipt that the sequencer took it, which is not the book having
            // taken it. Only the orders read can say that, so this says exactly
            // what the venue said and no more.
            Ok(format!(
                "{} was submitted as order {placed}. It rests once the sequencer takes it.",
                acted(&draft),
            ))
        }
        // Unreachable while the store is only written by `unlock_agent`, which
        // pairs the key with the network's own scheme. Refused rather than
        // asserted, because the one thing worse than not sending an order is
        // sending it signed by a key for another exchange.
        _ => Err(HlError::new(
            "The key this session holds is not for this network.".to_owned(),
        )),
    }
}

/// Pull one resting order, by the id the row carries.
///
/// One id and one path for both venues, because the row already holds whichever
/// name its venue gave the order: Hyperliquid's own `oid`, and on Lighter the
/// client order index the placer chose — which is the only handle that venue
/// offers, since a submission is answered with a transaction hash.
pub async fn cancel_resting(
    venue: Venue,
    session: Session,
    now_s: i64,
    coin: String,
    oid: i64,
) -> Result<String, HlError> {
    let key = signing_key(venue, &session, now_s)
        .ok_or_else(|| HlError::new(locked_out(venue, session.clone())))?;
    match (Network::of(venue).signing, &*key) {
        (Signing::Eip712(chain), HeldKey::Agent(wallet)) => {
            // The wire names a market by its index, which is what the universe
            // supplies; the row carries the ticker.
            let market = SymbolRow {
                name: coin.clone(),
                ..SymbolRow::default()
            };
            hl_cancel(chain, wallet, &market, oid).await?;
            Ok(format!("Order {oid} on {coin} is cancelled."))
        }
        (
            Signing::ApiKey(zone),
            HeldKey::ApiKey {
                key,
                account,
                index,
            },
        ) => {
            lighter_cancel(zone, key, *account, *index, &coin, oid).await?;
            Ok(format!("Order {oid} on {coin} was sent for cancellation."))
        }
        _ => Err(HlError::new(
            "The key this session holds is not for this network.".to_owned(),
        )),
    }
}

/// What the order was, in the words the receipt opens with.
fn acted(draft: &Draft) -> String {
    format!(
        "{} {} {}",
        if draft.buy { "Buy" } else { "Sell" },
        fmt_size(draft.size),
        draft.coin,
    )
}

/// The ticket's resting rule as each venue numbers it. Two small maps rather
/// than one shared enum, because the two venues do not agree on what the
/// longest-lived order is — `venue_tif_note` says so on the ticket.
fn hl_tif(tif: Tif) -> signing::Tif {
    match tif {
        Tif::Gtc => signing::Tif::Gtc,
        Tif::Ioc => signing::Tif::Ioc,
        Tif::Alo => signing::Tif::Alo,
    }
}

fn lighter_resting(tif: Tif) -> Resting {
    match tif {
        Tif::Gtc => Resting::Deadline,
        Tif::Ioc => Resting::Immediate,
        Tif::Alo => Resting::PostOnly,
    }
}

/// Forget the key. The one transition with no conditions on it.
pub fn lock_agent() -> Session {
    advance(Session::Locked, Event::Lock)
}

/// The clock moving, which is the only thing that ends a window.
///
/// Ticked rather than merely read, because a session that expired between two
/// ticks has to *become* `Expired` — the panel names the key that lapsed and
/// offers to approve again, and it can only do that while it still holds one.
/// `can_trade` is the belt beside this brace: it takes the clock too, so a
/// laptop that slept through an expiry cannot trade on the strength of a tick
/// that never arrived.
pub fn tick_agent(session: Session, now_s: i64) -> Session {
    advance(session, Event::Tick(millis(now_s)))
}

/// The one question the ticket asks before it enables anything.
pub fn session_can_trade(session: Session, now_s: i64) -> bool {
    can_trade(&session, millis(now_s))
}

/// The word in the header, which answers what the app may do rather than which
/// state machine arm it is in.
///
/// Four words for seven states, because the header has room for one and the
/// question it answers is nearly binary. The two that are not `READ ONLY` are
/// the two worth a different word: a window that ran out is not the same as
/// never having had one, and a prompt that is up is a thing to wait for.
pub fn session_badge(session: Session, now_s: i64) -> String {
    match session {
        _ if can_trade(&session, millis(now_s)) => "UNLOCKED",
        Session::Unlocking => "UNLOCKING",
        Session::Expired { .. } => "KEY EXPIRED",
        // Everything else can only read, and says the same true thing about
        // itself that this badge has always said.
        _ => "READ ONLY",
    }
    .to_owned()
}

/// What the custody panel says about where the session stands, when the state
/// itself carries the sentence.
///
/// Only `Unavailable` does: its reason is the platform's own words about a
/// keychain that will not serve this build or this machine, and inventing a
/// second sentence for it here would be this module guessing at something
/// `session.rs` was careful to carry.
pub fn session_reason(session: Session) -> String {
    match session {
        Session::Unavailable { reason } => reason,
        _ => String::new(),
    }
}

/// The agent address this session holds, live or lapsed, or nothing.
pub fn session_agent(session: Session) -> String {
    agent(&session).map_or_else(String::new, |key| key.address.clone())
}

/// The account it acts for, in every state that knows one.
pub fn session_account(session: Session) -> String {
    account(&session).unwrap_or_default().to_owned()
}

/// How much of the approval's window is left, as a sentence, or nothing when
/// there is no key to say it about.
///
/// Written from `remaining_ms`, which saturates, so a clock wound past the end
/// of time pegs at "expired" rather than printing a negative countdown.
pub fn session_window(session: Session, now_s: i64) -> String {
    let now = millis(now_s);
    let Some(key) = agent(&session) else {
        return String::new();
    };
    let left = key.remaining_ms(now);
    if left == 0 {
        return "This key's window has closed. Approve it again to keep trading.".to_owned();
    }
    let hours = left / 3_600_000;
    let span = if hours >= 48 {
        format!("{} days", hours / 24)
    } else if hours >= 1 {
        format!("{hours} hours")
    } else {
        format!("{} minutes", (left / 60_000).max(1))
    };
    format!("The exchange stops honouring this key in {span}.")
}

/// Whether unlocking is a thing this session can be asked to do.
///
/// `Unavailable` is out because the keystore is chosen at compile time and a
/// second prompt fetches the same refusal — which is exactly why `session.rs`
/// gives it a state of its own. `Unlocking` is out because a sheet is already
/// up. Everything else can be asked, including `Ready`: renewing early extends
/// the window rather than being dropped on the floor.
pub fn session_unlockable(session: Session) -> bool {
    !matches!(session, Session::Unavailable { .. } | Session::Unlocking)
}

/// Why the unlock button is dead, or nothing when it is not.
///
/// The same shape the gate refuses an address in: the control goes dead and
/// says which refusal it is, rather than answering a press with nothing.
///
/// It no longer takes a venue. It used to refuse Lighter outright, because this
/// panel could only make Ethereum agent keys; it now makes whichever key the
/// network's scheme signs with, so every network is unlockable and the only
/// reasons left are about the platform rather than the exchange.
pub fn session_refusal(session: Session) -> String {
    match session {
        Session::Unavailable { reason } => reason,
        Session::Unlocking => "Waiting for the platform's prompt.".to_owned(),
        _ => String::new(),
    }
}

/// A session that has never been asked for anything, which is where the app
/// boots and where locking returns it to.
pub fn session_start() -> Session {
    Session::default()
}

/// A real account and one of its real approvals, read off `extraAgents` rather
/// than invented, so a drawn panel carries the shape of an address and the size
/// of a window the exchange actually issues.
const DEMO_ACCOUNT: &str = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a";
const DEMO_AGENT: &str = "0x13070cb3597c75100928720060c7acff4d22bc09";

/// The states a preset draws, each built by driving the machine rather than by
/// naming a variant.
///
/// Naming one would let a fixture be a state the rules cannot reach — a
/// `Ready` for an account nobody unlocked, an `Expired` holding a key with no
/// address — and a panel drawn from an impossible state is a panel nobody can
/// trust the picture of. Every one of these goes through `step`, so the
/// machine's own refusals apply to fixtures too.
fn entered(account: &str) -> Session {
    step(
        step(Session::Locked, Event::Prompt),
        Event::Unlocked {
            outcome: Unlock::Platform,
            account: account.to_owned(),
        },
    )
}

pub fn demo_session_unenrolled() -> Session {
    step(
        step(Session::Locked, Event::Prompt),
        Event::Unlocked {
            outcome: Unlock::Unenrolled,
            account: String::new(),
        },
    )
}

pub fn demo_session_unavailable() -> Session {
    step(
        step(Session::Locked, Event::Prompt),
        Event::Unlocked {
            outcome: Unlock::Unavailable("no platform keychain on this build".to_owned()),
            account: String::new(),
        },
    )
}

/// Unlocked, with nothing approved to sign with — the state a first unlock
/// lands in before the account's wallet has approved anything.
pub fn demo_session_unapproved() -> Session {
    entered(DEMO_ACCOUNT)
}

/// A live approval, with the window the venue actually granted the key this
/// address holds: a shade under 180 days.
pub fn demo_session_ready(now_s: i64) -> Session {
    demo_approved(now_s, 88 * 24 * 3_600)
}

/// The same approval, a day after the exchange stopped honouring it.
pub fn demo_session_expired(now_s: i64) -> Session {
    demo_approved(now_s, -24 * 3_600)
}

fn demo_approved(now_s: i64, left_s: i64) -> Session {
    let now = millis(now_s);
    step(
        entered(DEMO_ACCOUNT),
        Event::Approved {
            key: AgentKey {
                address: DEMO_AGENT.to_owned(),
                account: DEMO_ACCOUNT.to_owned(),
                approved_at: now,
                expires_at: now.saturating_add(millis(left_s)),
            },
            now,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::Chain;

    const ACCOUNT: &str = "0x1025d5c2057058ffd8acf57109c5f649c11bdc11";
    const AGENT: &str = "0x13070cb3597c75100928720060c7acff4d22bc09";
    const HOUR: i64 = 3_600;

    /// The key store is one global, and these tests write to it — so they take
    /// a turn each rather than racing. Without this the suite would be a test
    /// of thread scheduling: one test's `lock_agent` clears the key another has
    /// just seeded, and which one fails moves with the machine.
    fn one_at_a_time() -> MutexGuard<'static, ()> {
        static TURN: Mutex<()> = Mutex::new(());
        TURN.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Put keys in the store the way an unlock would, so the retention can be
    /// tested without a keychain. Nothing here can read the bytes back, because
    /// nothing in the module can — what a test may see is which networks are
    /// held, and for whom.
    fn seed_keys(venues: &[Venue]) {
        *lock(vault()) = Some(Vault {
            address: ACCOUNT.to_owned(),
            keys: venues
                .iter()
                .map(|venue| (*venue, Arc::new(HeldKey::Agent(Wallet::generate()))))
                .collect(),
        });
    }

    fn seed_key() {
        seed_keys(&[Venue::Hyperliquid]);
    }

    fn holding_a_key() -> bool {
        lock(vault()).is_some()
    }

    /// A priced ticket, as `price_ticket` would answer for the draft below.
    fn quote() -> crate::hyperliquid::Ticket {
        crate::hyperliquid::price_ticket(
            64_000.0,
            "1".to_owned(),
            "5".to_owned(),
            Some(SymbolRow {
                name: "BTC".to_owned(),
                leverage: 40.0,
                maintenance: 0.0125,
                ..SymbolRow::default()
            }),
            true,
            0.0,
            false,
            None,
        )
    }

    fn ready_at(now_s: i64, days: i64) -> Session {
        step(
            step(
                step(Session::Locked, Event::Prompt),
                Event::Unlocked {
                    outcome: Unlock::Platform,
                    account: ACCOUNT.to_owned(),
                },
            ),
            Event::Approved {
                key: AgentKey {
                    address: AGENT.to_owned(),
                    account: ACCOUNT.to_owned(),
                    approved_at: millis(now_s),
                    expires_at: millis(now_s + days * 24 * HOUR),
                },
                now: millis(now_s),
            },
        )
    }

    /// The badge answers what the app may do, so the only state that may say
    /// `UNLOCKED` is the one the ticket may trade from — and it has to stop
    /// saying it the moment the window closes, whether or not a tick arrived
    /// to notice. A badge read off the variant alone would say it forever.
    #[test]
    fn only_a_session_that_may_trade_reads_as_unlocked() {
        let now = 1_786_172_634;
        let ready = ready_at(now, 30);
        assert!(session_can_trade(ready.clone(), now));
        assert_eq!(session_badge(ready.clone(), now), "UNLOCKED");

        // The same session, past its window, with nothing else changed.
        let later = now + 31 * 24 * HOUR;
        assert!(!session_can_trade(ready.clone(), later));
        assert_ne!(
            session_badge(ready.clone(), later),
            "UNLOCKED",
            "a lapsed window still reading UNLOCKED is the badge lying about \
             the one thing it is for"
        );

        for locked in [
            Session::Locked,
            Session::Unenrolled,
            Session::Unlocked {
                account: ACCOUNT.to_owned(),
            },
            Session::Unavailable {
                reason: "no keychain".to_owned(),
            },
        ] {
            assert!(!session_can_trade(locked.clone(), now));
            assert_ne!(session_badge(locked, now), "UNLOCKED");
        }
    }

    /// A refusal and a fault are different answers and the panel must not blur
    /// them: one is worth pressing again and the other never will be.
    #[test]
    fn a_declined_prompt_can_be_asked_again_and_a_broken_keystore_cannot() {
        let declined = step(
            step(Session::Locked, Event::Prompt),
            Event::Unlocked {
                outcome: Unlock::Locked,
                account: String::new(),
            },
        );
        assert!(session_unlockable(declined.clone()));
        assert!(
            session_refusal(declined).is_empty(),
            "a cancelled sheet leaves the button live, because asking again is \
             a reasonable thing for the user to do"
        );

        let faulted = step(
            step(Session::Locked, Event::Prompt),
            Event::Unlocked {
                outcome: Unlock::Unavailable("no platform keychain on this build".to_owned()),
                account: String::new(),
            },
        );
        assert!(!session_unlockable(faulted.clone()));
        assert_eq!(
            session_refusal(faulted.clone()),
            "no platform keychain on this build",
            "the reason is the platform's own words, not a sentence invented here"
        );
        assert_eq!(
            session_reason(faulted),
            "no platform keychain on this build"
        );
    }

    /// A network this app cannot sign for is not a network to offer a key on,
    /// and the refusal says which fact it is rather than looking like a
    /// Every network is unlockable now, which is the claim that replaced a
    /// refusal.
    ///
    /// This panel used to mint Ethereum agent keys only, so Lighter was refused
    /// before any sheet. It now mints whichever key the network's scheme signs
    /// with, so the only reasons left for a dead UNLOCK are about the platform:
    /// a keychain this build does not have, and a prompt already up. A venue is
    /// no longer one of them, and asserting that on all four is what stops the
    /// old refusal creeping back under a new name.
    #[test]
    fn every_network_can_be_unlocked_and_only_the_platform_refuses() {
        for venue in [
            Venue::Hyperliquid,
            Venue::HyperliquidTestnet,
            Venue::Lighter,
            Venue::LighterTestnet,
        ] {
            assert!(
                session_refusal(Session::Locked).is_empty(),
                "{}: nothing about a network makes UNLOCK dead any more",
                venue_name(venue),
            );
            assert!(session_unlockable(Session::Locked));
        }

        // The two that do refuse, and they are the platform's rather than the
        // venue's.
        assert_eq!(
            session_refusal(Session::Unavailable {
                reason: "no platform keychain on this build".to_owned(),
            }),
            "no platform keychain on this build",
        );
        assert!(!session_refusal(Session::Unlocking).is_empty());
    }

    /// The keychain item names the deployment, because the same address holds
    /// two different accounts on the two of them and a secret read back under
    /// the wrong one is a key the venue has never heard of.
    #[test]
    fn one_address_on_two_deployments_is_two_keychain_items() {
        assert_ne!(
            item(Venue::Hyperliquid, ACCOUNT),
            item(Venue::HyperliquidTestnet, ACCOUNT)
        );
        assert!(item(Venue::HyperliquidTestnet, ACCOUNT).contains(ACCOUNT));
        // And the exchange too, for the same reason one step out: both venues
        // have a mainnet, one address is read at both, and the keys are
        // unrelated — so an item named for the deployment alone would have the
        // second enrolment overwrite the first.
        assert_ne!(
            item(Venue::Hyperliquid, ACCOUNT),
            item(Venue::Lighter, ACCOUNT)
        );
        let mut filed: Vec<String> = [
            Venue::Hyperliquid,
            Venue::HyperliquidTestnet,
            Venue::Lighter,
            Venue::LighterTestnet,
        ]
        .iter()
        .map(|venue| item(*venue, ACCOUNT))
        .collect();
        filed.sort();
        let held = filed.len();
        filed.dedup();
        assert_eq!(filed.len(), held, "two networks share a keychain item");
        // And the same address typed either way is the same item, so a capital
        // letter is not a second enrolment.
        assert_eq!(
            item(Venue::Hyperliquid, ACCOUNT),
            item(
                Venue::Hyperliquid,
                &ACCOUNT.to_uppercase().replace("0X", "0x")
            ),
        );
    }

    /// The countdown is what tells a reader the key is running out while it
    /// still works, so it has to shorten as the clock moves and stop rather
    /// than go negative when the window closes.
    #[test]
    fn the_window_counts_down_and_pegs_instead_of_going_negative() {
        let now = 1_786_172_634;
        let ready = ready_at(now, 30);
        let opened = session_window(ready.clone(), now);
        assert!(opened.contains("30 days"), "{opened}");

        let nearly = session_window(ready.clone(), now + 29 * 24 * HOUR);
        assert!(nearly.contains("24 hours"), "{nearly}");

        let past = session_window(ready.clone(), now + 40 * 24 * HOUR);
        assert!(
            past.contains("closed"),
            "a window that has run out has no time left to print: {past}"
        );
        assert!(!past.contains('-'), "and never a negative one: {past}");

        assert!(
            session_window(Session::Locked, now).is_empty(),
            "a session holding no key has no window to count"
        );
    }

    /// Locking forgets the key from wherever the session was, which is the one
    /// transition with no conditions on it.
    #[test]
    fn locking_forgets_the_key() {
        let now = 1_786_172_634;
        let ready = ready_at(now, 30);
        assert_eq!(session_agent(ready.clone()), AGENT);
        assert_eq!(session_account(ready), ACCOUNT);

        let locked = lock_agent();
        assert!(session_agent(locked.clone()).is_empty());
        assert!(session_account(locked.clone()).is_empty());
        assert!(!session_can_trade(locked, now));
    }

    /// The one read this seam makes, held against what the venue actually
    /// answers rather than against a fixture. A fixture agreeing with the
    /// parser that built it proves only that the fixture was typed carefully.
    ///
    /// Both deployments, because the unlock has to work on the one where being
    /// wrong is free — and `extraAgents` is a different request type from the
    /// four the terminal already makes, so the deployment answering those is
    /// not evidence that it answers this. The mainnet address holds a real
    /// approval, so a parser that returned nothing would pass on testnet alone;
    /// the testnet answer for the same address is empty, which is correct and
    /// is what an account with no key here looks like.
    ///
    /// Nothing is signed and nothing is sent: this is one GET-shaped POST to a
    /// public read.
    #[test]
    #[ignore = "hits both live deployments, run explicitly: the approval read"]
    fn the_venue_lists_the_approvals_this_seam_reads() {
        crate::hyperliquid::open_the_wire();
        smol::block_on(async {
            let live = hl_agents(Chain::Mainnet, ACCOUNT.to_owned())
                .await
                .expect("mainnet answers which keys are live");
            let held = live
                .iter()
                .find(|(address, _)| address.eq_ignore_ascii_case(AGENT))
                .expect("this account's approval, which is what the fixture was read off");
            assert!(
                held.1 > millis(now_ms() / 1_000),
                "the venue lists live approvals only, so a listed key has a window ahead of it"
            );
            assert!(
                live.iter().all(|(address, _)| !address.is_empty()),
                "a key with no address is one the exchange would never have approved"
            );

            // The same read on the other deployment. An address funded on
            // mainnet has approved nothing here, and an empty answer is that
            // fact rather than a failure — which is exactly the distinction the
            // unlock has to draw when it says a key is not approved yet.
            let test = hl_agents(Chain::Testnet, ACCOUNT.to_owned())
                .await
                .expect("testnet answers the same read");
            assert!(
                test.iter()
                    .all(|(address, _)| !address.eq_ignore_ascii_case(AGENT)),
                "the mainnet key is not approved on testnet, which is the whole \
                 reason the keychain item names the deployment"
            );
        });
    }

    /// The key's lifetime is the `Ready` window, by construction.
    ///
    /// `advance` is the one place a transition happens, and every transition
    /// that does not land on `Ready` takes the key with it. That is what makes
    /// lock, expiry, and the network and address switches need no rule of their
    /// own — they are already transitions.
    ///
    /// Asserted one event at a time, because the failure this guards is a
    /// single arm that forgets: a `Lock` that drops the key and a `Tick` past
    /// the window that does not is a session drawn READ ONLY holding a live
    /// key.
    #[test]
    fn a_transition_out_of_ready_drops_the_key() {
        let _turn = one_at_a_time();
        let now = 1_786_172_634;
        let ready = ready_at(now, 30);

        // Locking, which is the transition with no conditions on it.
        seed_key();
        assert!(
            holding_a_key(),
            "the seed is the premise of every case here"
        );
        lock_agent();
        assert!(!holding_a_key(), "locking must forget the key");

        // A window that ran out while nobody was looking.
        seed_key();
        let lapsed = tick_agent(ready.clone(), now + 31 * 24 * HOUR);
        assert!(matches!(lapsed, Session::Expired { .. }));
        assert!(!holding_a_key(), "an expired window keeps no key");

        // And the case that must *not* drop it: a tick inside the window is a
        // session that goes on trading, so a rule phrased as "any tick drops
        // the key" would lock the app out every second.
        seed_key();
        let still = tick_agent(ready, now + HOUR);
        assert!(matches!(still, Session::Ready { .. }));
        assert!(holding_a_key(), "a live window keeps its key");
        lock_agent();
    }

    /// `can_trade` is the only gate, and it is inside the accessor rather than
    /// beside it.
    ///
    /// Two halves. A session that may not trade cannot reach the key however
    /// many keys are held — including a `Ready` whose window closed while the
    /// app was asleep, which is the case a check on the variant alone gets
    /// wrong. And a session that may trade still reaches nothing when the store
    /// is empty, which is what a preset, a capture or a fixture is.
    #[test]
    fn only_a_session_that_may_trade_reaches_the_key() {
        let _turn = one_at_a_time();
        let now = 1_786_172_634;
        let ready = ready_at(now, 30);

        seed_key();
        assert!(
            signing_key(Venue::Hyperliquid, &ready, now).is_some(),
            "a live window signs"
        );

        // The same session, one tick past its window and never ticked.
        assert!(
            signing_key(Venue::Hyperliquid, &ready, now + 31 * 24 * HOUR).is_none(),
            "a lapsed window must not sign on the strength of a tick that never came",
        );
        for locked in [
            Session::Locked,
            Session::Unenrolled,
            Session::Unlocked {
                account: ACCOUNT.to_owned(),
            },
            Session::Expired {
                key: AgentKey {
                    address: AGENT.to_owned(),
                    account: ACCOUNT.to_owned(),
                    approved_at: millis(now),
                    expires_at: millis(now),
                },
            },
        ] {
            assert!(
                signing_key(Venue::Hyperliquid, &locked, now).is_none(),
                "{locked:?} may not sign"
            );
        }

        // And the half a fixture is: `Ready` reached through the real machine,
        // with nothing behind it.
        lock_agent();
        assert!(!holding_a_key());
        assert!(
            signing_key(Venue::Hyperliquid, &ready_at(now, 30), now).is_none(),
            "a session built without an unlock holds no key, which is what makes \
             a preset safe to screenshot",
        );
    }

    /// One unlock, every enrolled network — and nothing else.
    ///
    /// The owner's 2026-08-10 scope, from the accessor's side. A key is reached
    /// per network even though the prompt was not, so a network the address
    /// never enrolled reaches nothing and reads as needing enrolment; and the
    /// vault belongs to the account it was unlocked for, so a session for any
    /// other address reaches nothing either. That last one is what "changing
    /// address forgets the keys" is made of, and it is here rather than in
    /// whichever handler changes an address.
    #[test]
    fn one_unlock_reaches_every_enrolled_network_and_no_others() {
        let _turn = one_at_a_time();
        let now = 1_786_172_634;
        let ready = ready_at(now, 30);

        // Two of the four enrolled, which is the ordinary case: an address that
        // trades one exchange's live deployment and the other's testnet.
        seed_keys(&[Venue::Hyperliquid, Venue::LighterTestnet]);
        for held in [Venue::Hyperliquid, Venue::LighterTestnet] {
            assert!(
                signing_key(held, &ready, now).is_some(),
                "{} was enrolled and released by the same prompt",
                venue_name(held),
            );
        }
        for absent in [Venue::HyperliquidTestnet, Venue::Lighter] {
            assert!(
                signing_key(absent, &ready, now).is_none(),
                "{} was never enrolled, so it has nothing to sign with",
                venue_name(absent),
            );
        }

        // The same vault, a session for somebody else. Nothing is reachable,
        // whatever the session says about itself.
        let elsewhere = step(
            step(
                step(Session::Locked, Event::Prompt),
                Event::Unlocked {
                    outcome: Unlock::Platform,
                    account: "0x0000000000000000000000000000000000000001".to_owned(),
                },
            ),
            Event::Approved {
                key: AgentKey {
                    address: AGENT.to_owned(),
                    account: "0x0000000000000000000000000000000000000001".to_owned(),
                    approved_at: millis(now),
                    expires_at: millis(now + 30 * 24 * HOUR),
                },
                now: millis(now),
            },
        );
        assert!(can_trade(&elsewhere, millis(now)), "premise: it may sign");
        assert!(
            signing_key(Venue::Hyperliquid, &elsewhere, now).is_none(),
            "a vault belongs to the account it was unlocked for",
        );

        // And locking still takes every network's key at once: they were
        // released by one prompt, so they are forgotten as one thing.
        lock_agent();
        for venue in [Venue::Hyperliquid, Venue::LighterTestnet] {
            assert!(signing_key(venue, &ready, now).is_none());
        }
    }

    /// The send refuses before it reaches a venue, and says which half of the
    /// gate refused it.
    #[test]
    fn a_send_without_a_key_is_refused_in_the_app() {
        let _turn = one_at_a_time();
        let now = 1_786_172_634;
        lock_agent();
        let draft = crate::venue::order_draft(
            Venue::Hyperliquid,
            "BTC".to_owned(),
            Some(SymbolRow {
                name: "BTC".to_owned(),
                ..SymbolRow::default()
            }),
            true,
            "1".to_owned(),
            64_000.0,
            false,
            false,
            false,
            Tif::Gtc,
            quote(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert!(draft.refusal.is_empty(), "{}", draft.refusal);

        // A session that may not sign at all.
        let refused = smol::block_on(submit_order(
            Venue::Hyperliquid,
            Session::Locked,
            now,
            Some(draft.clone()),
        ))
        .expect_err("a locked session cannot send");
        assert!(refused.message.contains("Unlock"), "{}", refused.message);

        // And one that may, with nothing behind it — the fixture case again,
        // this time through the send rather than through the accessor.
        let empty_handed = smol::block_on(submit_order(
            Venue::Hyperliquid,
            ready_at(now, 30),
            now,
            Some(draft),
        ))
        .expect_err("a session holding no key cannot send");
        assert!(
            empty_handed.message.contains("Unlock"),
            "{}",
            empty_handed.message
        );

        // And the gate's *own* half, which the key check would otherwise cover
        // for it: a session holding a real key, sending an order the ticket
        // never finished describing. Without the gate this reaches the venue
        // and fails at the wire, which is a different sentence and a request
        // that should never have left.
        seed_key();
        let unfinished = crate::venue::order_draft(
            Venue::Hyperliquid,
            "BTC".to_owned(),
            Some(SymbolRow {
                name: "BTC".to_owned(),
                ..SymbolRow::default()
            }),
            true,
            String::new(),
            64_000.0,
            false,
            false,
            false,
            Tif::Gtc,
            quote(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert_eq!(unfinished.refusal, "This order has no size yet.");
        let refused_order = smol::block_on(submit_order(
            Venue::Hyperliquid,
            ready_at(now, 30),
            now,
            Some(unfinished),
        ))
        .expect_err("an order with no size is not an order");
        assert_eq!(
            refused_order.message, "This order has no size yet.",
            "the order's own refusal has to reach the reader rather than a \
             transport failure from a request that should never have been made",
        );
        lock_agent();

        // A confirmation that is not there is not an order.
        let nothing = smol::block_on(submit_order(
            Venue::Hyperliquid,
            ready_at(now, 30),
            now,
            None,
        ))
        .expect_err("there is nothing to send");
        assert!(
            nothing.message.contains("no confirmed order"),
            "{}",
            nothing.message
        );
    }

    /// A tick is what turns a window that has run out into a session that says
    /// so, and it is one-way: a clock wound backwards must not hand back a key
    /// the exchange has already stopped honouring.
    #[test]
    fn a_tick_expires_a_window_and_never_reopens_one() {
        let now = 1_786_172_634;
        let ready = ready_at(now, 30);
        assert!(matches!(ready, Session::Ready { .. }));

        let lapsed = tick_agent(ready, now + 31 * 24 * HOUR);
        assert!(matches!(lapsed, Session::Expired { .. }));
        assert_eq!(session_badge(lapsed.clone(), now), "KEY EXPIRED");
        assert!(
            !session_agent(lapsed.clone()).is_empty(),
            "the panel names the key that lapsed, so the key is kept"
        );

        let wound_back = tick_agent(lapsed, now);
        assert!(matches!(wound_back, Session::Expired { .. }));
        assert!(!session_can_trade(wound_back, now));
    }
}
