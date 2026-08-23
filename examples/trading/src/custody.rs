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
//! and this wiring adds seven more that only exist now that something calls it:
//!
//! 1. **UNLOCK raises one sheet, not two.** `load` is the only guarded read
//!    this path makes, so it should be one. Two would mean something else is
//!    touching the item, and the fix is a shared `LAContext` — a dependency
//!    this change would not add on a guess.
//! 2. **Cancelling the sheet leaves the panel offering to try again**, with
//!    "Touch ID was cancelled" beside a live button — never the dead button and
//!    red platform sentence a fault gets. This is the distinction the whole
//!    seam is shaped around and the one place it is visible.
//! 3. **The round trip.** ENROL ALL on a first run, then UNLOCK reaching
//!    `Ready` with a window the exchange assigned, for every network the plan
//!    named.
//! 4. **ENROL ALL over an existing enrolment costs a sheet and keeps the old
//!    secret when the add fails.** `session.rs` reads the old bytes back before
//!    it deletes them precisely so a failed replace is survivable, and that
//!    read is a guarded read — so expect a sheet on re-enrolment, none on a
//!    first run, and the previous key still loading afterwards. The other half
//!    of that survivability is `enrol_one`'s ordering — the venue is asked
//!    before anything is filed — and it needs a Mac for the same reason: point
//!    a build at an unreachable endpoint, press ENROL ALL over a live
//!    enrolment, and the previous key should still unlock.
//! 5. **THIS IS MINE stores the account key and says so.** One sheet, an item
//!    at `wallet:0x…`, and the panel's sentence naming the address it kept —
//!    which is the arm a build with no keychain answers with the platform's
//!    refusal instead, and the only arm a Linux runner ever sees.
//! 6. **ENROL ALL costs one sheet for four networks, not four.** The account
//!    key is read once and used for each network in turn, so the sheet count is
//!    the assertion — and it is the whole of what makes the owner's rule about
//!    naming rather than granularity hold up in practice.
//! 7. **A cancelled enrolment sheet registers nothing.** `Held::Declined`
//!    returns before the first network, so the failure mode to rule out is a
//!    partial enrolment nobody agreed to.
//!
//! Until a person on a Mac reports those, the honest claim is that this seam's
//! logic is tested and its platform half is compiled, reviewed and unrun.
//!
//! # What the first real Mac reported, 2026-08-10
//!
//! None of the sixteen. The owner pressed THIS IS MINE on a `cargo run` build
//! and the app answered with `-34018` — `errSecMissingEntitlement` — from the
//! Enclave, before a sheet ever had cause to appear. **That is the first yield
//! of this list, and it is a deployment answer rather than a defect**: the
//! Enclave and the data-protection keychain serve signed code, an unsigned
//! binary is not signed code, and no arrangement of this file changes that.
//!
//! Three things came of it, and each is somewhere:
//!
//! - **The sentence.** `described` in `session.rs` turns that one status into
//!   what a reader can act on, and every keychain and Enclave call in this app
//!   composes its message there — the entitlement is not specific to the
//!   wrapping key, so neither is the fix.
//! - **The build.** `scripts/sign-dev.sh` wraps the binary in a `.app` with an
//!   embedded provisioning profile and signs it. Apple documents why nothing
//!   cheaper works: `keychain-access-groups` is a restricted entitlement, and
//!   a restricted entitlement is honoured only when a profile authorizes it,
//!   which an ad-hoc signature cannot arrange.
//! - **The refusal.** An import on a build that cannot seal stores nothing —
//!   `store_sealed` seals before it files, so there is no path that writes an
//!   unsealed wallet — and says so with the command that fixes it. Held by
//!   `a_build_that_cannot_seal_files_nothing_and_names_the_fix`.
//!
//! So the sixteen above are not merely unrun; they are owed a signed build to
//! be run against, and every one of them is still open.

#[cfg(test)]
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use crate::hyperliquid::{
    HlError, Order, SymbolRow, fmt_px, fmt_size, hl_agents, hl_cancel, hl_place, order_label,
};
use crate::lighter::{lighter_account_index, lighter_api_keys, lighter_cancel, lighter_place};
use crate::lighter_sign::{PrivateKey, Resting};
/// Ice names one namespace per Rust module, and custody is the one it talks
/// to — so the type it holds is published from here. `session.rs` stays where
/// the rules are and stays out of the app's vocabulary.
pub use crate::session::Session;
use crate::session::{
    AgentKey, Cause, Event, Guard, Held, Keystore, KeystoreError, PlatformKeystore, PlatformWrap,
    Secret, Unlock, account, agent, can_trade, load_sealed, step, store_sealed,
};
use crate::signing::{self, MasterKey, Wallet};
use crate::vault;
use crate::venue::{Act, Draft, Network, Signing, Sweep, venue_list, venue_name};
use crate::{OrderKind, Tif, Venue};

/// What one act of custody produced: where the session now stands, and the
/// sentence the panel owes about it.
///
/// The note is not an error channel. It carries the outcomes that are answers
/// rather than faults — a declined sheet, an account whose key nobody has
/// approved yet — which the state alone cannot distinguish: `step` maps a
/// decline back to `Session::Locked`, which is also where the app sits before
/// anybody has asked for anything. Without a sentence beside it, cancelling
/// Touch ID and never pressing the button look identical on screen.
/// `wants_passphrase` is the one thing here that is a *question* rather than an
/// outcome. A build the Secure Enclave will not serve keeps its keys in a file
/// this app encrypts, and the screen has to put a box on it — see `vault`. The
/// flag rather than the sentence, because a screen deciding what to draw off
/// the wording of a note is a screen one rephrasing away from drawing nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub session: Session,
    pub note: String,
    pub wants_passphrase: bool,
}

impl Entry {
    fn plain(session: Session) -> Self {
        Self {
            session: agreeing(session),
            note: String::new(),
            wants_passphrase: false,
        }
    }

    fn saying(session: Session, note: &str) -> Self {
        Self {
            session: agreeing(session),
            note: note.to_owned(),
            wants_passphrase: false,
        }
    }

    /// The act could not happen because this build has no passphrase yet, and
    /// nothing was spent finding that out. The session is untouched: wanting a
    /// passphrase is not a refusal to unlock, it is a question about how this
    /// machine keeps things.
    fn asking(session: Session, note: &str) -> Self {
        Self {
            session: agreeing(session),
            note: note.to_owned(),
            wants_passphrase: true,
        }
    }
}

/// The one place the platform's refusal becomes the file vault's turn.
///
/// `-34018` is a fact about the binary rather than a failure of the call — see
/// `session::UNSIGNED` — so it is the one status with somewhere else to go.
/// Every keystore act in this file goes through here, because a site that
/// branched for itself would be a key this app can write and never read back.
fn or_vault<T>(
    platform: Result<T, KeystoreError>,
    otherwise: impl FnOnce() -> Result<T, KeystoreError>,
) -> Result<T, KeystoreError> {
    match platform {
        Err(failure) if failure.cause == Cause::Unsigned => otherwise(),
        other => other,
    }
}

/// Whether a keystore failure is a question for the reader rather than an
/// answer about the machine. Both of the file's own refusals are: one wants a
/// passphrase and the other wants a different one, and every act routes them to
/// the same box.
fn asking(failure: &KeystoreError) -> bool {
    matches!(failure.cause, Cause::WantsPassphrase | Cause::Refused)
}

/// Whether the file should answer a read instead of the platform.
///
/// The whole judgement, with no keychain and no filesystem in it, because both
/// of its arms have to be decidable on a machine that has neither. Two roads,
/// and only two:
///
/// - the platform refusing `-34018`, which is the ordinary one; and
/// - a keychain answering `Missing` over a file that is not — a machine signed
///   *after* it wrote one, reading back what it stored before it was. "Nothing
///   here", said by the place that was not looking, is not an answer about the
///   key.
///
/// Everything else is the platform's answer and stays it, a decline included: a
/// cancelled sheet is not a reason to go looking somewhere else.
fn file_reads(platform: &Result<Held, KeystoreError>, in_file: bool) -> bool {
    match platform {
        Err(failure) => failure.cause == Cause::Unsigned,
        Ok(Held::Missing) => in_file,
        Ok(_) => false,
    }
}

/// Read an item from wherever this machine actually keeps it.
fn read_item(name: &str, phrase: &str) -> Result<Held, KeystoreError> {
    let answered = PlatformKeystore.load(name);
    if file_reads(&answered, vault::holds(name)) {
        return vault::take(name, phrase);
    }
    answered
}

/// The account's own key, which takes the envelope road on the platform and the
/// same file road off it. Separate from `read_item` because only this one is
/// sealed twice on macOS — see `load_sealed` for the migration it also carries.
fn read_wallet_item(address: &str, phrase: &str) -> Result<Held, KeystoreError> {
    let name = wallet_item(address);
    let answered = load_sealed(&PlatformWrap, &PlatformKeystore, &name);
    if file_reads(&answered, vault::holds(&name)) {
        return vault::take(&name, phrase);
    }
    answered
}

/// Whether this machine has a key file at all, which is what a boot asks so the
/// passphrase box is already on the panel for a reader who set one last time.
pub fn vault_occupied() -> bool {
    vault::occupied()
}

/// The store, brought into agreement with the session about to be drawn.
///
/// `advance` drops every key whenever a transition lands anywhere but `Ready`,
/// and this is that same rule for the acts that are not transitions. Importing
/// a wallet and enrolling a network have no `Event` — they answer a session
/// directly — and a screen drawing READ ONLY over a store that still holds
/// keys is exactly the disagreement `advance` exists to make impossible.
///
/// It sits on the constructors rather than on their eleven call sites so that
/// the next act added here cannot forget it.
fn agreeing(session: Session) -> Session {
    if !matches!(session, Session::Ready { .. }) {
        drop(lock(vault()).take());
    }
    session
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

#[cfg(not(test))]
fn vault() -> &'static Mutex<Option<Vault>> {
    static HELD: OnceLock<Mutex<Option<Vault>>> = OnceLock::new();
    HELD.get_or_init(Mutex::default)
}

#[cfg(test)]
fn vault() -> &'static Mutex<Option<Vault>> {
    static HELD: InstanceStores<Option<Vault>> = OnceLock::new();
    per_instance(&HELD)
}

/// The stores one process is holding, by the application each belongs to.
#[cfg(test)]
type InstanceStores<T> = OnceLock<Mutex<HashMap<u64, &'static Mutex<T>>>>;

/// One store per driven application, for the two stores above.
///
/// A machine has one keychain, so a shipped process has one of these and a
/// `static` is the honest shape. A test binary runs one whole application per
/// test thread against that same `static`, and they overwrite each other: every
/// `lock_agent` takes every key with it, and it is one press away on every
/// screen; an import waiting in `pending` is read back by whichever test asks
/// first. This module's own tests used to take turns on a mutex for that
/// reason. Keying beats taking turns: the tests that drive a whole application
/// are the slow ones and there are hundreds of them.
///
/// The per-instance `Mutex` is leaked rather than dropped with its test. It is
/// one pointer-sized allocation per instance in a binary that exits when the
/// suite does, and it is what keeps the returned `&'static` honest without an
/// `Arc` on every call site.
#[cfg(test)]
fn per_instance<T: Default + Send + 'static>(
    stores: &'static InstanceStores<T>,
) -> &'static Mutex<T> {
    lock(stores.get_or_init(Mutex::default))
        .entry(ui_lang_runtime::testing::app_instance())
        .or_insert_with(|| Box::leak(Box::<Mutex<T>>::default()))
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

/// What the keychain files an account's **own** key under.
///
/// The address alone, with no venue in it, and that is the difference between
/// this and `item`. A venue key is approved on one deployment of one exchange
/// and is unknown on the others; the account's key is the account — the same
/// twelve words, the same address, on every network there is. One item, so an
/// owner types a phrase once rather than once per venue, and so there is one
/// thing to delete when they are done with this machine.
fn wallet_item(address: &str) -> String {
    format!("wallet:{}", address.trim().to_lowercase())
}

/// The account key an import derived, waiting for the owner to say it is theirs.
///
/// It sits here between the two halves of an import rather than in Ice state,
/// for the reason the vault does: state is cloned, captured and printed. Only
/// the *address* crosses back to the screen, which is the whole point of the
/// confirmation — the owner reads an address they recognise before anything is
/// written to a keychain.
#[cfg(not(test))]
fn pending() -> &'static Mutex<Option<MasterKey>> {
    static PENDING: OnceLock<Mutex<Option<MasterKey>>> = OnceLock::new();
    PENDING.get_or_init(Mutex::default)
}

#[cfg(test)]
fn pending() -> &'static Mutex<Option<MasterKey>> {
    static PENDING: InstanceStores<Option<MasterKey>> = OnceLock::new();
    per_instance(&PENDING)
}

/// A phrase this app just made, and the words it will ask for back.
///
/// The phrase crosses to the screen because the owner has to write it down —
/// that is the whole of this step, and the ceiling `state.ice` records covers
/// it. `positions` is what makes the confirmation a check rather than a
/// formality: they are chosen here, from the same generator the phrase came
/// from, so neither the view nor the owner picks which words are asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Minted {
    pub phrase: String,
    pub positions: Vec<i64>,
    pub error: String,
}

/// How many words the backup check asks for.
///
/// Three. Zero is negligent — a phrase nobody copied is an account nobody can
/// recover, and the app cannot tell the difference until it is too late to fix.
/// Twenty-four is hostile: re-typing the whole phrase is a five-minute chore
/// that trains people to paste it from wherever they saved it, which is exactly
/// the habit this step exists to discourage. Three sampled positions cannot be
/// answered by somebody who did not copy the phrase and can be answered in
/// seconds by somebody who did, which is the only distinction a backup check is
/// trying to draw.
const BACKUP_WORDS: usize = 3;

/// Make one, and choose what to ask about it.
///
/// `sync` rather than a task: this is a read from the OS generator and some
/// arithmetic, with nothing to await, and putting a round trip between the
/// press and the words would be inventing latency.
pub fn mint_wallet() -> Minted {
    let refused = |error: String| Minted {
        phrase: String::new(),
        positions: Vec::new(),
        error,
    };
    let phrase = match crate::seed::new_phrase() {
        Ok(phrase) => phrase,
        Err(error) => return refused(error.message()),
    };
    let words = phrase.split_whitespace().count();
    let Ok(count) = u8::try_from(words) else {
        return refused("that phrase is too long to check".to_owned());
    };
    let mut positions: Vec<i64> = Vec::new();
    // Rejection rather than a modulo, because a byte does not divide evenly by
    // twenty-four. The bias would not matter — this only picks which words are
    // asked about — but the unbiased version is one comparison, and a biased
    // draw in a file about key material is a thing somebody has to read twice
    // to be sure of.
    let ceiling = (u8::MAX / count) * count;
    for _ in 0..256 {
        if positions.len() == BACKUP_WORDS.min(words) {
            break;
        }
        let mut byte = [0u8; 1];
        if getrandom::fill(&mut byte).is_err() {
            return refused(crate::seed::SeedError::Entropy.message());
        }
        if byte[0] >= ceiling {
            continue;
        }
        let at = i64::from(byte[0] % count) + 1;
        if !positions.contains(&at) {
            positions.push(at);
        }
    }
    if positions.len() < BACKUP_WORDS.min(words) {
        return refused("could not choose the words to check".to_owned());
    }
    positions.sort_unstable();
    Minted {
        phrase,
        positions,
        error: String::new(),
    }
}

/// What the panel asks for, in the owner's numbering.
pub fn backup_asks(positions: &[i64]) -> String {
    let named: Vec<String> = positions.iter().map(|at| format!("word {at}")).collect();
    match named.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// What one of the asked-for positions is called, for the field that takes it.
///
/// One field per position, so the label *is* the question: nobody has to work
/// out which of three boxes the ninth word goes in.
pub fn backup_label(positions: &[i64], at: i64) -> String {
    usize::try_from(at)
        .ok()
        .and_then(|at| positions.get(at))
        .map_or_else(String::new, |at| format!("Word {at}"))
}

/// Why the backup check has not passed, or nothing when it has.
///
/// It names the **fields** that do not match and never the words that would.
/// Which positions are being asked for is on screen already — each field
/// carries its own label — so saying which of them is wrong tells an attacker
/// nothing they cannot read, and tells the owner the one thing they need in
/// order to fix it. Naming the expected word would be the leak: it turns a
/// check into a prompt, and a phrase nobody wrote down would pass on the second
/// attempt.
///
/// This is a change from the single blind field it replaces, where hiding the
/// position was right precisely because the positions were not each their own
/// visible question.
pub fn backup_refused(phrase: String, positions: Vec<i64>, given: Vec<String>) -> String {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    if words.is_empty() || positions.is_empty() {
        return "There is no phrase waiting to be confirmed.".to_owned();
    }
    if given.len() != positions.len() {
        return format!("Fill in {}.", backup_asks(&positions));
    }
    let wrong: Vec<i64> = positions
        .iter()
        .zip(&given)
        .filter(|(at, given)| {
            // One word per field. Two words in a box is not the word that was
            // asked for, however the first of them reads.
            let mut typed = given.split_whitespace();
            let (Some(word), None) = (typed.next(), typed.next()) else {
                return true;
            };
            !usize::try_from(**at)
                .ok()
                .and_then(|at| at.checked_sub(1))
                .and_then(|at| words.get(at))
                .is_some_and(|expected| expected.eq_ignore_ascii_case(word))
        })
        .map(|(at, _)| *at)
        .collect();
    if wrong.is_empty() {
        String::new()
    } else {
        format!(
            "{} does not match what you wrote down. Check your copy — nothing has been stored.",
            backup_asks(&wrong),
        )
    }
}

/// Derive the account a typed phrase names, and hold it for confirmation.
///
/// Both arguments arrive as `ui_lang_runtime::Secret`, which is the only
/// reading of those buffers that exists anywhere in the program: not clonable,
/// redacted when printed, borrowed once here, and wiped when this function
/// returns.
pub async fn read_wallet(
    phrase: ui_lang_runtime::Secret,
    passphrase: ui_lang_runtime::Secret,
) -> Result<Entry, CustodyFault> {
    read_phrase(phrase.expose(), passphrase.expose()).await
}

/// The same derivation over a phrase this app generated and put on the screen.
///
/// It takes a `String` because that is what a phrase on screen is. Keeping it
/// separate rather than widening `read_wallet` is the point: `secret` is the
/// type no value can be turned into, and a second entry point is what that
/// costs. A made phrase has no passphrase — the owner never chose one.
pub async fn read_made_wallet(phrase: String) -> Result<Entry, CustodyFault> {
    read_phrase(&phrase, "").await
}

/// Nothing is stored and no sheet is raised: this answers an address and
/// waits. A phrase with a bad checksum, an unknown word or a passphrase this
/// app will not normalise is refused here, where the owner can still see what
/// they typed.
///
/// The alternate input is a raw private key, because both venues' SDKs take one
/// and not every owner holds a phrase — the same 32 bytes reached by a shorter
/// road, and stored and wiped identically. Which of the two it is stays a
/// question for this side: `looks_like_a_key` reads the borrow, so no fact
/// about the text's shape ever needs a name in Ice, and the box stays one box.
async fn read_phrase(phrase: &str, passphrase: &str) -> Result<Entry, CustodyFault> {
    let derived = if looks_like_a_key(phrase) {
        from_raw_key(phrase.trim())
    } else {
        crate::seed::seed_from_phrase(phrase, passphrase)
            .and_then(|seed| crate::seed::ethereum_key(&seed))
            .map_err(|error| error.message())
            .and_then(|mut key| {
                let made = MasterKey::from_secret(&key).map_err(|failure| failure.message);
                key.fill(0);
                std::hint::black_box(&mut key);
                made
            })
    };
    Ok(match derived {
        Err(reason) => Entry::saying(Session::Locked, &reason),
        Ok(master) => {
            let address = master.address().to_string();
            *lock(pending()) = Some(master);
            Entry::saying(
                Session::Locked,
                &format!(
                    "This phrase is the account {address}. If that is not the address you expect, \
                     nothing has been stored — go back and check the words.",
                ),
            )
        }
    })
}

/// 64 hex characters, with or without the prefix, is somebody pasting a private
/// key rather than a phrase. Distinguished by shape rather than by a second
/// field: a recovery phrase is words and a key is hex, and no phrase is either.
fn looks_like_a_key(typed: &str) -> bool {
    let text = typed.trim().strip_prefix("0x").unwrap_or(typed.trim());
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn from_raw_key(typed: &str) -> Result<MasterKey, String> {
    let text = typed.strip_prefix("0x").unwrap_or(typed);
    let mut bytes: [u8; 32] = hex::decode(text)
        .ok()
        .and_then(|raw| raw.try_into().ok())
        .ok_or_else(|| "That is not 32 bytes of hex.".to_owned())?;
    let made = MasterKey::from_secret(&bytes).map_err(|failure| failure.message);
    bytes.fill(0);
    std::hint::black_box(&mut bytes);
    made
}

/// The address the phrase just derived, or nothing when none is waiting.
pub fn pending_wallet() -> String {
    lock(pending())
        .as_ref()
        .map_or_else(String::new, |master| master.address().to_string())
}

/// Forget it unstored. What every exit from the import step does.
///
/// Answers a session so a handler can assign it, the shape `lock_agent` already
/// has: closing the import step is also a moment nothing may be signed from.
pub fn forget_wallet() -> Session {
    drop(lock(pending()).take());
    lock_agent()
}

/// Store the account key the owner just confirmed.
///
/// The one sheet an import costs. Past here the phrase is not needed again on
/// this machine: what is kept is the 32 bytes it derives, which is all an
/// enrolment signature needs and rather less than the phrase can do.
pub async fn keep_wallet(passphrase: ui_lang_runtime::Secret) -> Result<Entry, CustodyFault> {
    // Peeked rather than taken. A store that cannot happen *yet*, because this
    // build has no passphrase for its file, must leave the key exactly where it
    // was: the owner is about to type one and press again, and a key spent on
    // being asked a question is an import they have to start over.
    let waiting = {
        let held = lock(pending());
        held.as_ref()
            .map(|master| (master.address().to_string(), master.secret()))
    };
    let Some((address, bytes)) = waiting else {
        return Ok(Entry::saying(
            Session::Locked,
            "There is no wallet waiting to be stored.",
        ));
    };
    let phrase = passphrase.expose().to_owned();
    let filed = {
        let address = address.clone();
        smol::unblock(move || {
            // Sealed on the way in. What lands in the keychain is ciphertext
            // only this machine's Secure Enclave can open, and the item's own
            // guard sits over that — see `Wrap` in `session.rs` for why both.
            // Where there is no Enclave to reach, the same secret is sealed
            // under the passphrase and lands in a file instead. Never in the
            // clear, on either road.
            let name = wallet_item(&address);
            let secret = Secret::new(bytes);
            or_vault(
                store_sealed(&PlatformWrap, &PlatformKeystore, &name, &secret),
                || vault::keep(&name, &secret, &phrase),
            )
        })
        .await
    };
    // Spent now, and only on an answer that settles something. A question
    // leaves the key waiting; everything else spends it, because the road that
    // answered had already been handed the bytes.
    if !matches!(&filed, Err(failure) if asking(failure)) {
        drop(lock(pending()).take());
    }
    Ok(match filed {
        // The file has no passphrase yet, or the one it was given does not open
        // it. Both are questions rather than refusals: the session is
        // untouched, the key is still waiting, and what the screen owes is the
        // box with a sentence beside it.
        Err(failure) if asking(&failure) => Entry::asking(Session::Locked, &failure.message),
        // Said on the step as well as held in the session, because the step
        // is the only thing the owner is looking at: the address leaves the
        // panel either way, and without a sentence a build with nowhere to
        // put a key answers THIS IS MINE by emptying the screen. The rule
        // `session_refusal` already states — the platform's own words rather
        // than a press that answers with nothing — applies to this press too.
        Err(failure) => Entry::saying(
            Session::Unavailable {
                reason: failure.message.clone(),
            },
            &failure.message,
        ),
        Ok(()) => wallet_stored(&address),
    })
}

/// What a kept wallet answers with: the address, where it went, and the act it
/// unblocks.
///
/// Split out of `keep_wallet` so `demo_wallet_kept` is this sentence rather
/// than a copy of it that can drift away from it. Where it went is read off the
/// file rather than passed in, because the road was chosen by a platform status
/// several frames down and a caller repeating that judgement is a caller that
/// can get it wrong.
fn wallet_stored(address: &str) -> Entry {
    let kept_by = if vault::holds(&wallet_item(address)) {
        "in a file only that passphrase opens"
    } else {
        "behind Touch ID"
    };
    Entry::saying(
        Session::Locked,
        &format!(
            "{address} is on this Mac now, {kept_by}. Enrol the networks you want to trade and \
             this app can sign for itself.",
        ),
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
                if PrivateKey::from_hex(&hex::encode(&bytes)).is_ok() {
                    // The *secret* in hex, because the Lighter arm has to sign
                    // the registration with the key it is registering. The
                    // Hyperliquid arm answers an address, which is public.
                    let secret = hex::encode(&bytes);
                    return Ok((bytes, secret));
                }
            }
        }
    }
    Err("could not generate a usable key".to_owned())
}

/// What one enrolment sheet is about to authorise, under the `#enrol-row`
/// rows that name each network it is for with its kind beside it.
///
/// **The owner's rule, 2026-08-10: a master signature never happens without a
/// sheet the user just answered, and the sheet — or the confirmation in front
/// of it — names everything that signature authorises.** The application of it,
/// mine: explicitness is about *naming*, not about counting sheets, so one
/// prompt may authorise four enrolments as long as the panel lists all four
/// above the sentence. Four sheets that each said "approve a key" would be less explicit than
/// one that says which four networks, and on which of them being wrong costs
/// money.
///
/// Sits beside the retention decision above because they are the same kind of
/// thing: a rule about what a reader has agreed to before a key is used. The
/// same rule governs an eviction — one incident names every network it covers,
/// and never rides along on another action's sheet.
pub fn enrolment_plan(address: &str) -> String {
    let address = address.trim().to_lowercase();
    if address.is_empty() {
        return String::new();
    }
    format!(
        "One Touch ID, and this app registers a key of its own on each network above for \
         {address}. That signature is your account's. It approves trading keys and cannot \
         withdraw."
    )
}

/// Enrol every network at once, on one sheet.
///
/// The account key is read exactly once and then used for each network in turn:
/// a fresh trading key generated and filed under that network's own item, and
/// the enrolment that registers it signed by the account. The alternative — a
/// sheet per network — is not more explicit, it is the same act asked about
/// four times, and the rows over `enrolment_plan` are what make the one act
/// explicit.
///
/// A network that fails is reported and the rest continue: an exchange being
/// unreachable is not a reason to leave the other three unenrolled, and the
/// sentence says which of them landed.
pub async fn enrol_all(
    address: String,
    passphrase: ui_lang_runtime::Secret,
) -> Result<Entry, CustodyFault> {
    let address = address.trim().to_owned();
    if address.is_empty() {
        return Ok(Entry::saying(
            Session::Locked,
            "A key belongs to one account, so connect an address before enrolling.",
        ));
    }

    // The one sheet. Everything after it is arithmetic and requests.
    let phrase = passphrase.expose().to_owned();
    let opened = {
        let address = address.clone();
        let phrase = phrase.clone();
        // The one sheet, and the assertion that opens the envelope behind it.
        // An item written before the envelope existed is re-sealed here rather
        // than refused, once, on the read that finds it. On a build with no
        // Enclave the same read is the passphrase opening the file, and it is
        // the same one prompt: typed once, and every network below shares it.
        smol::unblock(move || read_wallet_item(&address, &phrase)).await
    };
    let master = match opened {
        Ok(Held::Missing) => {
            return Ok(Entry::saying(
                Session::Locked,
                "No wallet on this Mac for this address yet. Import one first.",
            ));
        }
        Ok(Held::Declined) => {
            return Ok(Entry::saying(
                Session::Locked,
                "Touch ID was cancelled, so nothing was signed and nothing was registered.",
            ));
        }
        Err(failure) if asking(&failure) => {
            return Ok(Entry::asking(Session::Locked, &failure.message));
        }
        Err(failure) => {
            return Ok(Entry::plain(Session::Unavailable {
                reason: failure.message,
            }));
        }
        Ok(Held::Secret(secret)) => {
            let bytes: Result<[u8; 32], _> = secret.expose().try_into();
            let Ok(made) = bytes.map(|bytes| MasterKey::from_secret(&bytes)) else {
                return Ok(Entry::plain(Session::Unavailable {
                    reason: "the stored wallet is not an account key; import one again".to_owned(),
                }));
            };
            match made {
                Ok(master) => master,
                Err(failure) => {
                    return Ok(Entry::plain(Session::Unavailable {
                        reason: failure.message,
                    }));
                }
            }
        }
    };

    let mut landed: Vec<String> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    for venue in venue_list() {
        match enrol_one(venue, &address, &master, &phrase).await {
            Ok(()) => landed.push(venue_name(venue)),
            Err(reason) => refused.push(format!("{}: {reason}", venue_name(venue))),
        }
    }
    Ok(Entry::saying(
        Session::Locked,
        &enrolment_outcome(&landed, &refused),
    ))
}

/// What came of the one act, in the order a reader needs it: what did not work
/// first, because that is the part with something to do about it.
fn enrolment_outcome(landed: &[String], refused: &[String]) -> String {
    match (landed.is_empty(), refused.is_empty()) {
        (true, false) => format!("Nothing was registered. {}", refused.join("; ")),
        (false, false) => format!(
            "{} did not take: {}. Registered on {}. Unlock to trade there.",
            if refused.len() == 1 {
                "One network"
            } else {
                "Some networks"
            },
            refused.join("; "),
            landed.join(", "),
        ),
        (true, true) => "There are no networks to enrol on.".to_owned(),
        (false, true) => format!(
            "Registered on {}. Unlock and this app can sign for itself.",
            landed.join(", "),
        ),
    }
}

/// One network's half of the act: a trading key made and filed, and the
/// enrolment that registers it signed by the account and sent.
///
/// Both venues take the same shape and differ only in what a registration *is*.
/// Hyperliquid approves an address as an API wallet, signed as typed data;
/// Lighter registers a public key at a slot, authorised by a signature over a
/// sentence. Neither can be signed by the key being registered, which is the
/// property the whole design exists for and the reason `MasterKey` is a
/// separate type.
async fn enrol_one(
    venue: Venue,
    address: &str,
    master: &MasterKey,
    phrase: &str,
) -> Result<(), String> {
    let scheme = Network::of(venue).signing;
    let (bytes, public) = generate(scheme)?;
    // Registered first and filed second, which is the order that survives a
    // venue saying no. `store` replaces whatever is under this network's item
    // and puts the old bytes back only when the *add* fails — it cannot know a
    // request is still to come. Filed first, an exchange that was unreachable,
    // or an address with no account on that deployment yet, would have taken a
    // working key with it and left an item that unlocks into "approve this
    // address from your wallet" when what the owner actually has to do is press
    // this button again. Filed last, a refused registration leaves the previous
    // key exactly where it was.
    //
    // What this order costs instead is the other way round: a keychain that
    // refuses after the venue agreed leaves a key registered and unusable, and
    // the next unlock reads `Missing` and says to enrol — which is the true
    // sentence, and the same press fixes it.
    registration(scheme, address, &public, master).await?;
    let name = item(venue, address);
    let secret = Secret::new(bytes);
    // A trading key is the secret, so the item's own guard is the whole of what
    // protects it — unlike the sealed wallet blob, which is guarded by the key
    // that opens it.
    let phrase = phrase.to_owned();
    smol::unblock(move || {
        or_vault(
            PlatformKeystore.store(&name, &secret, Guard::UserPresence),
            || vault::keep(&name, &secret, &phrase),
        )
    })
    .await
    .map_err(|failure| failure.message)
}

/// The half of an enrolment the venue performs, which is the half that can be
/// refused. Split out so the store above is unambiguously after it.
async fn registration(
    scheme: Signing,
    address: &str,
    public: &str,
    master: &MasterKey,
) -> Result<(), String> {
    match scheme {
        Signing::Eip712(chain) => {
            let agent =
                crate::signing::Address::parse(public).map_err(|failure| failure.message)?;
            let action = crate::signing::approve_agent(chain, agent, "ducktape", now_ms() as u64);
            crate::hyperliquid::exchange(chain, action.request(master))
                .await
                .map_err(|failure| failure.message)
                .and_then(|answer| {
                    // The venue answers a refusal with HTTP 200 and a sentence
                    // where the payload would be, which is the trap the order
                    // path already holds: reading the status alone reports a
                    // refused approval as a registration.
                    match answer.get("status").and_then(serde_json::Value::as_str) {
                        Some("ok") => Ok(()),
                        _ => Err(format!("{answer}")),
                    }
                })
        }
        Signing::ApiKey(zone) => {
            let key = crate::lighter_sign::PrivateKey::from_hex(public)
                .map_err(|error| error.to_string())?;
            let account = crate::lighter::lighter_account_index(zone, address.to_owned())
                .await
                .map_err(|failure| failure.message)?
                .ok_or_else(|| "this address has no account here yet".to_owned())?;
            let registration = crate::lighter_sign::Registration {
                account,
                api_key: LIGHTER_SLOT,
                public_key: key.public_key(),
                deadline_ms: now_ms() + 10 * 60 * 1_000,
                nonce: crate::lighter::lighter_nonce(zone, account, LIGHTER_SLOT)
                    .await
                    .map_err(|failure| failure.message)?,
            };
            let l1 = master
                .personal_sign(&crate::lighter_sign::registration_body(&registration))
                .hex();
            let built = crate::lighter_sign::change_pub_key(zone, &registration, &l1)
                .map_err(|error| error.to_string())?;
            crate::lighter::send_tx(zone, &built, &key)
                .await
                .map(|_| ())
                .map_err(|failure| failure.message)
        }
    }
}

/// The api-key slot this app registers under.
///
/// One slot, named once. An owner may hold keys at other slots for other tools
/// and this app neither reads nor replaces them; unlock finds ours by its public
/// key rather than by where it sits, so this is only where a *new* one goes.
const LIGHTER_SLOT: u8 = 2;

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
pub async fn unlock_agent(
    venue: Venue,
    address: String,
    passphrase: ui_lang_runtime::Secret,
) -> Result<Entry, CustodyFault> {
    let address = address.trim().to_owned();
    if address.is_empty() {
        return Ok(Entry::saying(
            Session::Locked,
            "A key belongs to one account, so connect an address before unlocking.",
        ));
    }

    let phrase = passphrase.expose().to_owned();
    let opened = {
        let address = address.clone();
        let phrase = phrase.clone();
        // Reading is what raises the sheet, and the sheet blocks — so it is off
        // the executor's thread rather than on it. On a build with no keychain
        // to raise one, the passphrase already typed is what opens the file,
        // and the same rule holds: one answer releases every network below.
        smol::unblock(move || read_key(venue, &address, &phrase)).await
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
            let phrase = phrase.clone();
            smol::unblock(move || secret_for(other, &address, &phrase)).await
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

fn read_key(venue: Venue, address: &str, phrase: &str) -> Opened {
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

    match read_item(&item(venue, address), phrase) {
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
        // A file with no passphrase yet, or one that does not open it. Asking
        // is not the session moving: `Locked` with a box on the panel, rather
        // than an `Unavailable` saying this machine cannot do it at all.
        Err(failure) if asking(&failure) => {
            Opened::Refused(Entry::asking(Session::Locked, &failure.message))
        }
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
fn secret_for(venue: Venue, address: &str, phrase: &str) -> Option<Loaded> {
    match read_item(&item(venue, address), phrase) {
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
///
/// The kind chooses which "what the ticket is describing" that is, because a
/// scale ticket describes a list. Both refusals arrive already worked out — the
/// draft's own and the ladder's — so this stays the one decision it was rather
/// than growing a second opinion about either.
pub fn order_gate(
    venue: Venue,
    session: Session,
    now_s: i64,
    kind: OrderKind,
    draft: Draft,
    ladder: String,
) -> String {
    let describes = if kind == OrderKind::Scale {
        ladder
    } else {
        draft.refusal
    };
    gated(venue, session, now_s, describes)
}

/// The session first, then whatever is being sent says about itself.
///
/// One fold shared by the button and by the send, so the order the two reasons
/// come in cannot drift between the screen and the wire.
fn gated(venue: Venue, session: Session, now_s: i64, refusal: String) -> String {
    match trade_refusal(venue, session, now_s) {
        locked if !locked.is_empty() => locked,
        _ => refusal,
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
    // Asked of the order in hand rather than of the ticket's kind: everything
    // that reaches here is one order, a ladder's rungs included, and a rung is
    // refused for its own reasons or for none.
    let refused = gated(venue, session.clone(), now_s, draft.refusal.clone());
    if !refused.is_empty() {
        return Err(HlError::new(refused));
    }
    let key = signing_key(venue, &session, now_s)
        .ok_or_else(|| HlError::new(locked_out(venue, session.clone())))?;
    match (Network::of(venue).signing, &*key) {
        (Signing::Eip712(chain), HeldKey::Agent(wallet)) => {
            let market = draft
                .market
                .clone()
                .ok_or_else(|| HlError::new("This market is not loaded here.".to_owned()))?;
            let (wires, grouping) = wire_orders(&draft, market.asset);
            let done = hl_place(chain, wallet, &market, &wires, grouping)
                .await
                .map_err(|failure| re_enrol(venue, failure))?;
            Ok(receipt(&draft, done))
        }
        (
            Signing::ApiKey(zone),
            HeldKey::ApiKey {
                key,
                account,
                index,
            },
        ) => {
            let order = Order {
                oid: 0,
                coin: draft.coin.clone(),
                buy: draft.buy,
                price: draft.price,
                size: draft.size,
                ts: 0,
            };
            let placed = lighter_place(
                zone,
                key,
                *account,
                *index,
                &draft.coin,
                order,
                draft.reduce_only,
                lighter_resting(draft.tif),
                draft.minutes,
            )
            .await
            .map_err(|failure| re_enrol(venue, failure))?;
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

/// An eviction reaches the reader as the thing to do about it, and everything
/// else reaches them unchanged.
fn re_enrol(venue: Venue, failure: HlError) -> HlError {
    if evicted(&failure.message) {
        return HlError::new(eviction_note(venue));
    }
    failure
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
            hl_cancel(chain, wallet, &market, oid)
                .await
                .map_err(|failure| re_enrol(venue, failure))?;
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
            lighter_cancel(zone, key, *account, *index, &coin, oid)
                .await
                .map_err(|failure| re_enrol(venue, failure))?;
            Ok(format!("Order {oid} on {coin} was sent for cancellation."))
        }
        _ => Err(HlError::new(
            "The key this session holds is not for this network.".to_owned(),
        )),
    }
}

/// Send a whole panel's worth: every resting order pulled, or every position
/// closed.
///
/// A loop over the two paths above and nothing else. Neither venue is asked
/// anything a single row does not already ask it, and every draft goes through
/// `submit_order`, so the gate, the key, the network's own signing scheme and
/// the per-order refusals all apply row by row exactly as they do to a typed
/// order. There is no bulk endpoint and this does not invent one.
///
/// Sequential rather than concurrent. Both venues sign with one key and one
/// nonce sequence, and firing seven signatures at once is how a nonce race
/// turns six good orders into one — the visible cost is a few round trips on a
/// button nobody presses twice a minute.
///
/// A row that fails does not stop the ones after it. A flatten that gave up on
/// its second position would leave five open and read as done; each is an
/// independent act, so each is attempted and the answer says which did not go.
pub async fn submit_sweep(
    venue: Venue,
    session: Session,
    now_s: i64,
    sweep: Option<Sweep>,
) -> Result<String, HlError> {
    let Some(sweep) = sweep else {
        return Err(HlError::new(
            "There is no confirmed sweep to send.".to_owned(),
        ));
    };
    let refused = trade_refusal(venue, session.clone(), now_s);
    if !refused.is_empty() {
        return Err(HlError::new(refused));
    }
    let mut sent = 0_usize;
    let mut refusals: Vec<String> = Vec::new();
    if sweep.act == Act::Cancel {
        for order in &sweep.orders {
            match cancel_resting(venue, session.clone(), now_s, order.coin.clone(), order.oid).await
            {
                Ok(_) => sent += 1,
                Err(error) => {
                    refusals.push(format!("{}\n{}", order_label(order.clone()), error.message))
                }
            }
        }
    } else {
        for (at, draft) in sweep.drafts.iter().enumerate() {
            match submit_order(venue, session.clone(), now_s, Some(draft.clone())).await {
                Ok(_) => sent += 1,
                // Named by the line the panel listed rather than by a second
                // description of the row. A flatten's rows are one market each
                // and the ticker told them apart; a ladder's are all one
                // market, and only the price says which rung the venue turned
                // down. The frozen line is what the reader agreed to, so it is
                // what the refusal is reported against.
                Err(error) => refusals.push(format!(
                    "{}\n{}",
                    sweep
                        .rows
                        .get(at)
                        .cloned()
                        .unwrap_or_else(|| draft.coin.clone()),
                    error.message
                )),
            }
        }
    }
    let asked = if sweep.act == Act::Cancel {
        sweep.orders.len()
    } else {
        sweep.drafts.len()
    };
    let what = sweep.act.done();
    if refusals.is_empty() {
        return Ok(format!("{sent} of {asked} {what}."));
    }
    // Partly done is a failure and says so, because the half that went is
    // already money. The panel stays up over the list it froze and the account
    // poll behind it is what says which rows are still there. Each refused
    // row is a line of its own with the venue's reason under it: a line is
    // the one boundary a translated sentence keeps.
    Err(HlError::new(format!(
        "{sent} of {asked} {what}.\n{}",
        refusals.join("\n")
    )))
}

/// The orders the wire carries, from the order that was confirmed.
///
/// The whole of the mapping, in one place, so a test can hold what the exchange
/// receives against what the panel said — and so that adding a field to `Draft`
/// without deciding whether it reaches the wire is a thing somebody has to do
/// on purpose rather than by omission.
///
/// **A confirmation with levels on it is more than one order.** The entry is
/// the first leg; a target and a stop are legs of their own, each the opposite
/// side of the entry and reduce-only, and the grouping is what makes the
/// exchange hold them together instead of resting them immediately. So this
/// answers a batch and the grouping that batch is to be read under, and a
/// caller cannot send one without the other.
///
/// Levels are unreachable while `attaches_levels` is false on every network —
/// `draft_refusal` refuses a draft carrying one before a key is asked for — so
/// today this always answers one leg under `Na`. It is written whole anyway
/// because the alternative is a gate that hides untested code: the day the fact
/// flips, what is behind it has already been held against the exchange's own
/// signer.
fn wire_orders(draft: &Draft, asset: u32) -> (Vec<signing::Order>, signing::Grouping) {
    let entry = signing::Order {
        asset,
        buy: draft.buy,
        price: draft.price,
        size: draft.size,
        reduce_only: draft.reduce_only,
        kind: signing::Kind::Limit(hl_tif(draft.tif)),
    };
    let mut wires = vec![entry];
    for (level, tpsl) in [(draft.tp, signing::Tpsl::Tp), (draft.sl, signing::Tpsl::Sl)] {
        // Zero is the ticket's "no level typed", not a level at zero — the two
        // fields are empty strings until somebody fills them and `amount`
        // reads an empty field as nothing.
        if level > 0.0 {
            wires.push(signing::Order {
                asset,
                // A level closes the position the entry opens, so it runs the
                // other way and may only ever shrink what is held. Neither is
                // a setting: a level on the entry's own side would add to the
                // position it was supposed to protect.
                buy: !draft.buy,
                // A trigger order still carries a limit price, and for a
                // market trigger the exchange reads it as the worst fill it
                // may take. The level's own price is the honest bound: it is
                // the number on screen, and it is the number the reader agreed
                // to.
                price: level,
                size: draft.size,
                reduce_only: true,
                kind: signing::Kind::Trigger {
                    px: level,
                    // Market, and deliberately not offered as a choice. A
                    // stop-limit that does not fill is a stop that did not
                    // stop, and a panel with one price field on it has not
                    // asked the reader which of the two they meant. The field
                    // says "stop at 90"; a market trigger is the only reading
                    // of that which keeps the promise.
                    market: true,
                    tpsl,
                },
            });
        }
    }
    let grouping = if wires.len() > 1 {
        // The levels ride on an entry that is in this same batch, so they wait
        // for it to fill. `PositionTpsl` is the other case — levels onto a
        // position that already exists — and nothing builds that draft yet.
        signing::Grouping::NormalTpsl
    } else {
        signing::Grouping::Na
    };
    (wires, grouping)
}

/// Whether a venue's refusal is it saying this app's key is not one of its
/// keys any more.
///
/// The two sentences are the two venues' own, read live while the order paths
/// were written. They matter because they are the *only* way an eviction
/// surfaces between unlocks: a key can be revoked from the exchange's own
/// interface at any moment, and this app finds out when it next tries to use
/// it. Drawn as an ordinary failure it reads as the exchange being down, and
/// the reader waits for something that will never come back.
///
/// Matched on the venue's words rather than on a code, because Hyperliquid has
/// no code — it answers HTTP 200 with a sentence. `21109` is Lighter's, and
/// both are quoted here rather than paraphrased.
pub fn evicted(message: &str) -> bool {
    message.contains("does not exist") || message.contains("21109")
}

/// What to do about it, named for the account and every network at once.
///
/// The same rule the enrolment sheet follows: one incident, one sentence, every
/// network it covers named. An eviction is not per-order news and re-enrolling
/// is not per-order work, so folding it into the receipt of whichever order
/// happened to hit it would be the silent version of both.
pub fn eviction_note(venue: Venue) -> String {
    format!(
        "{} no longer recognises this app's trading key. Nothing was sent. Import is unaffected \
         — enrol again from Settings and the account's own key will register a fresh one.",
        venue_name(venue),
    )
}

/// What the venue did, in the venue's own numbers.
///
/// Three outcomes and they are not one word: an order can rest whole, fill
/// whole, or fill part and rest — or, immediate-or-cancel, fill part and have
/// the rest cancelled. The amount filled comes from the venue's answer; the
/// only thing taken from the draft is the size that was asked for, and it is
/// there to say what did *not* happen.
fn receipt(draft: &Draft, done: crate::hyperliquid::Placed) -> String {
    let what = acted(draft);
    let short = done.filled + 1e-12 < draft.size;
    match (done.filled > 0.0, done.resting, short) {
        (true, 0, true) => format!(
            "{what}: {} of {} filled at {}, and the rest was cancelled.",
            fmt_size(done.filled),
            fmt_size(draft.size),
            fmt_px(done.at),
        ),
        (true, 0, false) => format!("{what} filled at {}.", fmt_px(done.at)),
        (true, oid, _) => format!(
            "{what}: {} of {} filled at {}, and {} rests as order {oid}.",
            fmt_size(done.filled),
            fmt_size(draft.size),
            fmt_px(done.at),
            fmt_size(draft.size - done.filled),
        ),
        (false, 0, _) => {
            format!(
                "{what} was accepted, and the venue reported neither a fill nor a resting order."
            )
        }
        (false, oid, _) => format!("{what} is resting as order {oid}."),
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

/// What pressing UNLOCK does. A build the Enclave refused keeps its keys in
/// a passphrase file, and the button's name says which prompt is coming.
pub fn unlock_label(locale: crate::Locale, vault: bool) -> String {
    crate::i18n::t(
        locale,
        if vault {
            "Unlock with this machine's passphrase"
        } else {
            "Unlock with Touch ID"
        },
    )
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

/// What a Mac answers when the keychain kept the key.
///
/// The one custody answer no Linux runner can produce: a build with no keychain
/// refuses the store and answers `Unavailable`, so the screen on the far side
/// of a *successful* one has no other way to be driven. Built by the same
/// constructor the real store returns, so a fixture cannot say something the
/// app would not.
pub fn demo_wallet_kept(address: String) -> Entry {
    wallet_stored(&address)
}

/// What a build the Secure Enclave will not serve answers with: a question
/// rather than an outcome, and nothing spent asking it.
///
/// Here for the reason the sessions below are, and for one more: `-34018` is
/// raised by a Mac deciding a binary is unsigned, and no runner in this
/// repository is one. A build with no keychain at all takes a different road —
/// `Unavailable`, which is what the panel already says — so the asking arm has
/// no other way to be driven.
pub fn demo_vault_asks() -> Entry {
    Entry::asking(Session::Locked, crate::vault::NO_PASSPHRASE)
}

/// The same seam answering rather than asking, which is every act that got
/// somewhere. Paired with the one above so a test can hold the difference
/// between them, which is the whole of what decides the box's life.
pub fn demo_vault_answers() -> Entry {
    Entry::plain(Session::Locked)
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

    use crate::session::UNSIGNED;

    /// The rule the whole fallback hangs off, in the one place it is written.
    ///
    /// `-34018` is the only status with somewhere else to go. A build with no
    /// keychain at all — which is every runner in this repository — must NOT
    /// reach the file: it has a different answer already, `Unavailable`, and
    /// the panel says so. Widening this predicate would quietly turn every
    /// Linux CI job into one keeping account keys in a file.
    #[test]
    fn only_an_unsigned_binary_is_the_files_turn() {
        let vaulted = || Ok::<&str, KeystoreError>("the file");
        let refuse = || -> Result<&str, KeystoreError> { panic!("not the file's turn") };

        assert_eq!(
            or_vault(Err(KeystoreError::new(UNSIGNED.to_owned())), vaulted),
            Ok("the file"),
        );
        assert!(
            or_vault(
                Err(KeystoreError::plain(
                    "no platform keychain on this build".to_owned()
                )),
                refuse
            )
            .is_err()
        );
        assert!(
            or_vault(
                Err(KeystoreError::plain("the keychain is unhappy".to_owned())),
                refuse
            )
            .is_err()
        );
        // And a platform that worked is never second-guessed.
        assert_eq!(or_vault(Ok("the keychain"), refuse), Ok("the keychain"));
    }

    /// The read side of the same rule, and the arm no end-to-end test here can
    /// reach: a keychain answering `Missing` over a file that holds the key.
    /// That is a machine signed *after* it wrote one, and reading it as "no key
    /// on this Mac, enrol again" would leave the keys sitting in the file while
    /// the panel says there are none.
    #[test]
    fn the_file_answers_a_read_the_keychain_was_not_looking_for() {
        let unsigned = || Err(KeystoreError::new(UNSIGNED.to_owned()));
        let broken = || {
            Err(KeystoreError::plain(
                "no platform keychain on this build".to_owned(),
            ))
        };

        // Unsigned goes to the file whatever the file holds — including
        // nothing, where the file answers `Missing` on its own.
        assert!(file_reads(&unsigned(), true));
        assert!(file_reads(&unsigned(), false));
        // The signed-later arm, and its own negative: an empty keychain over an
        // empty file is still an empty answer, not a trip to the filesystem.
        assert!(file_reads(&Ok(Held::Missing), true));
        assert!(!file_reads(&Ok(Held::Missing), false));
        // Everything else is the platform's answer and stays it. A cancelled
        // sheet especially: going looking in a file because the owner said no
        // would be this app routing around its own guard.
        assert!(!file_reads(&Ok(Held::Declined), true));
        assert!(!file_reads(
            &Ok(Held::Secret(Secret::new(vec![1; 32]))),
            true
        ));
        assert!(!file_reads(&broken(), true));
    }

    /// An entry that asks is not an entry that refused. The screen branches on
    /// the flag, and the session must stay somewhere the buttons are alive —
    /// being asked for a passphrase is the opposite of being told this machine
    /// cannot do it.
    #[test]
    fn asking_for_a_passphrase_leaves_the_session_where_the_buttons_work() {
        let asked = demo_vault_asks();
        assert!(asked.wants_passphrase);
        assert!(!asked.note.is_empty());
        assert!(session_unlockable(asked.session));
        assert!(!demo_vault_answers().wants_passphrase);
    }

    use crate::signing::Chain;

    const ACCOUNT: &str = "0x1025d5c2057058ffd8acf57109c5f649c11bdc11";
    /// The BIP-39 zero phrase at 24 words — public, worthless, and the same
    /// vector `seed.rs` pins the generator against.
    const ZEROS_24: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
                            abandon abandon abandon abandon abandon abandon abandon abandon \
                            abandon abandon abandon abandon abandon abandon abandon art";
    const AGENT: &str = "0x13070cb3597c75100928720060c7acff4d22bc09";
    const HOUR: i64 = 3_600;

    /// Two applications in one process each keep their own keys.
    ///
    /// A machine has one keychain, so the store is a `static` and that is the
    /// honest shape for a shipped process. A test binary is not that process:
    /// it runs one whole application per test thread, and against a single
    /// `static` one of them clears what another has just unlocked — `lock_agent`
    /// is the transition that takes every key with it, and it is one press away
    /// on every screen. Whole-app tests then fail on whichever pair the machine
    /// happened to overlap, and the panel a failing test drew is one the code
    /// would never draw.
    ///
    /// The barriers pin the interleaving to that one rather than leaving it to
    /// the scheduler, so this fails every run against a shared store rather than
    /// most of them.
    /// Nothing is asserted on either thread: a failed assertion there would
    /// leave the other one waiting on a barrier nobody reaches, and a test that
    /// hangs where it should fail costs the whole suite its timeout.
    #[test]
    fn two_apps_in_one_process_each_keep_their_own_keys() {
        let unlocked = std::sync::Barrier::new(2);
        let locked = std::sync::Barrier::new(2);
        let (seeded, kept, borrowed) = std::thread::scope(|scope| {
            let mine = scope.spawn(|| {
                seed_key();
                let seeded = holding_a_key();
                unlocked.wait();
                locked.wait();
                (seeded, holding_a_key())
            });
            let theirs = scope.spawn(|| {
                unlocked.wait();
                let borrowed = holding_a_key();
                let _ = lock_agent();
                locked.wait();
                borrowed
            });
            let (seeded, kept) = mine.join().expect("the seeding app panicked");
            (
                seeded,
                kept,
                theirs.join().expect("the locking app panicked"),
            )
        });
        assert!(seeded, "the seeded key never reached this app's own store");
        assert!(
            !borrowed,
            "an application was handed a key it never unlocked"
        );
        assert!(
            kept,
            "another application's lock took the key this one was holding"
        );
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

    /// The receipt says what the venue did, in the venue's numbers.
    ///
    /// The failure it guards is the quiet one: an immediate-or-cancel order for
    /// ten that crossed two, reported as ten. A trader reads that and believes
    /// they hold five times what they hold, and every screen afterwards agrees
    /// with the venue rather than with the receipt — so nothing ever tells them.
    #[test]
    fn the_receipt_says_what_the_venue_filled() {
        let draft = sendable(10.0);
        let done = |resting: i64, filled: f64, at: f64| crate::hyperliquid::Placed {
            resting,
            filled,
            at,
        };

        // The whole of it crossed.
        let whole = receipt(&draft, done(0, 10.0, 64_000.0));
        assert_eq!(whole, "Buy 10 BTC filled at 64,000.00.");

        // Two of ten crossed and the rest was cancelled, which is what an
        // immediate-or-cancel order does with what it cannot fill.
        let part = receipt(&draft, done(0, 2.0, 63_999.5));
        assert!(part.contains("2 of 10 filled"), "{part}");
        assert!(part.contains("the rest was cancelled"), "{part}");
        assert!(
            !part.contains("Buy 10 BTC filled at"),
            "a partial fill must not read as a whole one: {part}"
        );

        // Two of ten crossed and eight rest, which is what a good-till-cancelled
        // order does — and the remainder is named so it can be cancelled.
        let both = receipt(&draft, done(77, 2.0, 63_999.5));
        assert!(both.contains("2 of 10 filled"), "{both}");
        assert!(both.contains("8 rests as order 77"), "{both}");

        // Nothing crossed at all.
        assert_eq!(
            receipt(&draft, done(77, 0.0, 0.0)),
            "Buy 10 BTC is resting as order 77."
        );

        // And the answer that says neither, which is not a placement to report
        // as one.
        let silent = receipt(&draft, done(0, 0.0, 0.0));
        assert!(
            silent.contains("neither a fill nor a resting order"),
            "{silent}"
        );
    }

    /// One sendable draft of a given size, for the receipts above.
    fn sendable(size: f64) -> Draft {
        crate::venue::order_draft(
            Venue::Hyperliquid,
            "BTC".to_owned(),
            Some(SymbolRow {
                name: "BTC".to_owned(),
                ..SymbolRow::default()
            }),
            true,
            size.to_string(),
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
            String::new(),
        )
    }

    /// **What the exchange receives is what the confirmation said.**
    ///
    /// The freeze design's whole claim, tested as one thing rather than
    /// inferred from the pieces. Three defects of exactly this class had to be
    /// found by review before it existed: a target and a stop the panel drew
    /// and the wire never carried, a margin mode it stated and never sent, and
    /// a fill it reported at the size that was typed instead of the size that
    /// crossed.
    ///
    /// The `let Draft { .. }` below is the part that kills the class rather
    /// than the three instances. It names every field, so a field added to the
    /// confirmation and not to the wire **does not compile** — whoever adds one
    /// has to say here which of the two it is, in front of a reviewer.
    #[test]
    fn the_wire_carries_what_the_confirmation_said() {
        let draft = Draft {
            venue: Venue::HyperliquidTestnet,
            coin: "BTC".to_owned(),
            market: Some(SymbolRow {
                name: "BTC".to_owned(),
                asset: 7,
                ..SymbolRow::default()
            }),
            buy: false,
            size: 2.5,
            price: 64_000.5,
            walked: false,
            reduce_only: true,
            cross: true,
            tif: Tif::Ioc,
            leverage: 20.0,
            notional: 160_001.25,
            margin: 8_000.0,
            liquidation: 61_000.0,
            tp: 0.0,
            sl: 0.0,
            minutes: 0.0,
            refusal: String::new(),
        };
        let (wires, grouping) = wire_orders(&draft, 7);

        // An order with no levels on it is one order, and grouping one order
        // with nothing is `Na`.
        assert_eq!(wires.len(), 1);
        assert_eq!(grouping, signing::Grouping::Na);

        // Every figure the panel showed, on the wire unchanged.
        let wire = wires[0];
        assert_eq!(wire.asset, 7);
        assert_eq!(wire.buy, draft.buy);
        assert_eq!(wire.price, draft.price);
        assert_eq!(wire.size, draft.size);
        assert_eq!(wire.reduce_only, draft.reduce_only);
        assert_eq!(wire.kind, signing::Kind::Limit(signing::Tif::Ioc));

        // And the accounting, field by field. Anything not on the wire is
        // named here with the reason it is not, so "the panel says it" and
        // "the order carries it" cannot drift apart unnoticed again.
        let Draft {
            // On the wire, asserted above.
            buy: _,
            size: _,
            price: _,
            reduce_only: _,
            tif: _,
            // Names the market, which reaches the wire as its index.
            market,
            // Not on the wire, and each for a stated reason.
            //
            // `venue` and `coin` choose *where* it goes rather than riding in
            // it; `walked`, `notional`, `margin`, `liquidation` and `refusal`
            // are readings of the order rather than parts of it.
            venue: _,
            coin: _,
            walked: _,
            notional: _,
            margin: _,
            liquidation: _,
            refusal: _,
            // Not on the wire and *said so on screen*: both exchanges keep
            // these per market on the account, so the confirmation prints
            // `margin_estimate_note` under the figures it worked out from them.
            cross: _,
            leverage: _,
            // On the wire, as a leg each — asserted below against a draft that
            // carries them, because this one does not. Still refused before a
            // key is asked for while no venue attaches levels, which is the
            // last assertion in this test.
            tp,
            sl,
            // **Not on this venue's wire at all, and refused rather than
            // dropped.** Hyperliquid works an order over a window with a
            // separate `twapOrder` action carrying a `twap` object, and the
            // SDK every Hyperliquid vector here is driven out of does not sign
            // one — so this app has nothing to hold such bytes against and
            // `places_twap` is false on both Hyperliquid networks. A draft
            // carrying a window is refused by `draft_refusal` before a key is
            // asked for, which is asserted below. On Lighter it *is* on the
            // wire, as the order's type and its expiry together, and
            // `lighter_sign.rs` pins that against the venue's own signer.
            minutes,
        } = draft.clone();
        assert_eq!(minutes, 0.0);
        assert_eq!(market.map(|row| row.asset), Some(wire.asset));
        assert_eq!((tp, sl), (0.0, 0.0));

        // The two the note covers are covered by a sentence that names them.
        let note = crate::venue::margin_estimate_note();
        assert!(note.contains("Neither is sent"), "{note}");
        assert!(note.contains("per market on your account"), "{note}");

        // **Every level the confirmation showed leaves as its own leg.**
        //
        // Derived from the draft rather than counted out by hand, because the
        // defect this is here for is a leg that is quietly *not* built: a
        // batch missing its stop is a position with no protection on it, it
        // signs and recovers perfectly, and the exchange has nothing to
        // complain about. So the expectation is "one leg per level that is
        // set", read off the same draft the panel drew.
        let levelled = Draft {
            tp: 70_000.0,
            sl: 61_500.0,
            ..draft.clone()
        };
        let (legs, grouping) = wire_orders(&levelled, 7);
        let levels = [
            (levelled.tp, signing::Tpsl::Tp),
            (levelled.sl, signing::Tpsl::Sl),
        ];
        assert_eq!(
            legs.len(),
            1 + levels.iter().filter(|(level, _)| *level > 0.0).count(),
            "one leg for the entry and one for each level the panel showed: {legs:?}",
        );
        // And the grouping that makes them wait for the entry rather than rest
        // on their own. Under `Na` these same three legs are three unrelated
        // orders and the stop is live before there is anything to stop.
        assert_eq!(grouping, signing::Grouping::NormalTpsl);

        for (level, tpsl) in levels {
            let leg = legs
                .iter()
                .find(|leg| {
                    leg.kind
                        == signing::Kind::Trigger {
                            px: level,
                            market: true,
                            tpsl,
                        }
                })
                .unwrap_or_else(|| panic!("{tpsl:?} at {level} never reached the wire: {legs:?}"));
            // It closes rather than adds: the other side of the entry, only
            // ever shrinking, and for the size the entry opened.
            assert_eq!(leg.buy, !levelled.buy, "{tpsl:?} runs the entry's way");
            assert!(leg.reduce_only, "{tpsl:?} could add to the position");
            assert_eq!(leg.size, levelled.size);
            assert_eq!(leg.asset, wire.asset);
        }

        // And what the reader is told today: no venue attaches levels, so a
        // draft carrying either is refused before a key is ever asked for.
        // The legs above are what the gate is holding back, not what it lets
        // through.
        let with_levels = crate::venue::order_draft(
            Venue::HyperliquidTestnet,
            "BTC".to_owned(),
            draft.market.clone(),
            true,
            "1".to_owned(),
            64_000.0,
            false,
            false,
            false,
            Tif::Gtc,
            quote(),
            "70000".to_owned(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert!(
            with_levels
                .refusal
                .contains("does not attach a target or a stop"),
            "{}",
            with_levels.refusal,
        );
        assert!(
            !crate::venue::venue_attaches_levels(Venue::HyperliquidTestnet),
            "the refusal above is the gate, and it is only the gate while the \
             venue fact says the app does not attach levels",
        );
    }

    /// The send refuses before it reaches a venue, and says which half of the
    /// gate refused it.
    #[test]
    fn a_send_without_a_key_is_refused_in_the_app() {
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

    /// A revoked key surfaces as the thing to do about it, and nothing else does.
    ///
    /// These two sentences are the venues' own, read live while the order paths
    /// were written, and they are the *only* way an eviction reaches a reader
    /// between unlocks: the key can be revoked from the exchange's own interface
    /// at any moment and this app finds out when it next tries to use it. Drawn
    /// as an ordinary failure it reads as the exchange being down, and the reader
    /// waits for something that is never coming back.
    ///
    /// Both sides are held. A test that only checked the eviction would pass for
    /// a `re_enrol` that rewrote every failure into this sentence, which would
    /// send somebody to re-enrol over a network timeout.
    #[test]
    fn an_evicted_key_reads_as_the_thing_to_do_about_it() {
        // Hyperliquid answers HTTP 200 with a sentence and no code; Lighter has
        // the code. Both are quoted rather than paraphrased.
        for (venue, refusal) in [
            (
                Venue::Hyperliquid,
                "User or API Wallet 0x13070cb3597c75100928720060c7acff4d22bc09 does not exist.",
            ),
            (
                Venue::LighterTestnet,
                "code 21109: api key not registered for this account",
            ),
        ] {
            assert!(evicted(refusal), "{refusal}");
            let said = re_enrol(venue, HlError::new(refusal.to_owned())).message;
            assert_eq!(said, eviction_note(venue), "{refusal}");
            assert!(
                said.starts_with(&venue_name(venue)),
                "the incident names its network: {said}"
            );
            assert!(
                said.contains("enrol again from Settings"),
                "the incident says what to do about it: {said}"
            );
            assert!(
                said.contains("Nothing was sent"),
                "the order did not happen, and the sentence says so: {said}"
            );
            // And the venue's raw words are gone, because they are what read as
            // an outage.
            assert!(!said.contains(refusal), "{said}");
        }

        // Everything else reaches the reader unchanged. An exchange that is
        // simply down is not a key that was taken away.
        for ordinary in [
            "code 21734: price too far from mark",
            "connection reset by peer",
            "code 21706: order notional below the minimum",
        ] {
            assert!(!evicted(ordinary), "{ordinary}");
            assert_eq!(
                re_enrol(Venue::Hyperliquid, HlError::new(ordinary.to_owned())).message,
                ordinary,
            );
        }
    }

    /// A custody act that answers anything but `Ready` leaves the store empty.
    ///
    /// `advance` holds this for every transition, and importing and enrolling
    /// are not transitions — they answer a session directly. Before `agreeing`
    /// sat on the constructors, pressing CHECK or ENROL ALL from an unlocked
    /// session drew READ ONLY over a vault that still held every network's key.
    ///
    /// The act driven here is the one that touches nothing else on its way to
    /// the answer: an enrolment with no address reaches neither the keychain
    /// nor the waiting wallet, so what it proves is the constructor's rule and
    /// not a side effect of the path.
    #[test]
    fn an_act_that_answers_locked_forgets_the_keys() {
        seed_keys(&[Venue::Hyperliquid, Venue::Lighter]);
        assert!(holding_a_key(), "the fixture put keys in the store");

        let entry = smol::block_on(enrol_all(String::new(), ui_lang_runtime::Secret::new("")))
            .expect("an answer, not a fault");
        assert!(
            matches!(entry.session, Session::Locked),
            "{:?}",
            entry.session
        );
        assert!(
            !entry.note.is_empty(),
            "an act that refuses says why: {entry:?}"
        );
        assert!(
            !holding_a_key(),
            "a session drawn as locked must not leave a signable key behind"
        );
    }

    /// The backup check, driven from both sides against a phrase this test
    /// chose — which is the half the Ice test cannot reach, because a made
    /// phrase is random and no driver step can compute a word of it.
    #[test]
    fn backup_refused_takes_the_words_that_were_shown_and_nothing_else() {
        let phrase: Vec<&str> = ZEROS_24.split_whitespace().collect();
        let positions = vec![2i64, 9, 24];
        let right = || {
            vec![
                phrase[1].to_owned(),
                phrase[8].to_owned(),
                phrase[23].to_owned(),
            ]
        };

        assert_eq!(
            backup_refused(ZEROS_24.to_owned(), positions.clone(), right()),
            "",
            "the words that were shown, each in its own box",
        );
        // Capitals are the same word: somebody reading off paper types how they
        // type, and surrounding space is the field's, not the answer's.
        assert_eq!(
            backup_refused(
                ZEROS_24.to_owned(),
                positions.clone(),
                right()
                    .into_iter()
                    .map(|word| format!("  {}  ", word.to_uppercase()))
                    .collect(),
            ),
            "",
            "case and stray space are not wrong answers",
        );

        // One wrong box, and the refusal says which box — never what belongs
        // in it.
        let mut wrong = right();
        wrong[2] = "zoo".to_owned();
        let said = backup_refused(ZEROS_24.to_owned(), positions.clone(), wrong);
        assert_eq!(
            said,
            "word 24 does not match what you wrote down. Check your copy — nothing has been stored."
        );

        // Right words, wrong boxes — a check that sorted would pass this.
        let swapped = vec![
            phrase[23].to_owned(),
            phrase[8].to_owned(),
            phrase[1].to_owned(),
        ];
        assert!(
            !backup_refused(ZEROS_24.to_owned(), positions.clone(), swapped).is_empty(),
            "which box a word went in is part of the answer",
        );

        // A box holding two words is not the word that was asked for, however
        // the first of them reads.
        let mut crowded = right();
        crowded[0] = format!("{} {}", phrase[1], phrase[1]);
        assert!(
            !backup_refused(ZEROS_24.to_owned(), positions.clone(), crowded).is_empty(),
            "one word per box",
        );

        // An empty box, and too few boxes.
        let mut blank = right();
        blank[1] = String::new();
        assert!(!backup_refused(ZEROS_24.to_owned(), positions.clone(), blank).is_empty());
        assert!(
            !backup_refused(
                ZEROS_24.to_owned(),
                positions.clone(),
                vec![phrase[1].to_owned()],
            )
            .is_empty(),
        );

        // And nothing to check is refused rather than passed, which is the arm
        // a skipped mint would land on.
        assert!(!backup_refused(String::new(), positions, right()).is_empty());
    }

    /// What the refusal says, and what it must not say.
    ///
    /// It names the box, because each box is already labelled with its own
    /// position on screen — saying which one is wrong tells an attacker nothing
    /// they cannot read. What it must never name is the word that belongs
    /// there: that would turn a check into a prompt.
    #[test]
    fn the_backup_refusal_names_the_box_and_never_the_word() {
        let phrase: Vec<&str> = ZEROS_24.split_whitespace().collect();
        let said = backup_refused(
            ZEROS_24.to_owned(),
            vec![2, 9, 24],
            vec![phrase[1].to_owned(), "zoo".to_owned(), "zoo".to_owned()],
        );
        assert!(
            said.contains("word 9") && said.contains("word 24"),
            "{said}"
        );
        assert!(
            !said.contains("word 2,") && !said.starts_with("word 2 "),
            "{said}"
        );
        for leak in [phrase[8], phrase[23]] {
            assert!(!said.contains(leak), "the refusal leaked {leak:?}: {said}");
        }
    }

    /// The label each box carries, which is the question it is asking.
    #[test]
    fn each_box_is_labelled_with_the_position_it_takes() {
        let positions = vec![2i64, 9, 24];
        assert_eq!(backup_label(&positions, 0), "Word 2");
        assert_eq!(backup_label(&positions, 1), "Word 9");
        assert_eq!(backup_label(&positions, 2), "Word 24");
        assert_eq!(backup_label(&positions, 3), "");
    }

    /// A minted wallet: the shape the panel and the check both depend on.
    ///
    /// Minted many times rather than once, because "three distinct positions"
    /// is a claim one draw cannot test: three positions taken from twenty-four
    /// collide about one time in eight, so a generator that had stopped
    /// rejecting repeats would pass a single mint seven times out of eight and
    /// fail this suite at random afterwards. Two hundred draws make the same
    /// generator fail here every time.
    #[test]
    fn minting_answers_a_phrase_and_the_positions_it_will_ask_for() {
        for _ in 0..200 {
            let made = mint_wallet();
            assert_eq!(made.error, "");
            assert_eq!(made.phrase.split_whitespace().count(), 24);
            assert_eq!(made.positions.len(), 3);

            let mut sorted = made.positions.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(
                sorted, made.positions,
                "positions are distinct and in order"
            );
            assert!(
                made.positions.iter().all(|at| (1..=24).contains(at)),
                "{:?}",
                made.positions,
            );
        }
        let made = mint_wallet();

        // The phrase it answered is one the check accepts back, which is the
        // seam between minting and confirming.
        let words: Vec<&str> = made.phrase.split_whitespace().collect();
        let right: Vec<&str> = made
            .positions
            .iter()
            .map(|at| words[usize::try_from(*at).expect("a position") - 1])
            .collect();
        assert_eq!(
            backup_refused(
                made.phrase.clone(),
                made.positions,
                right.into_iter().map(str::to_owned).collect(),
            ),
            "",
        );
    }

    /// What the panel asks for, in words rather than in a list.
    #[test]
    fn the_backup_asks_for_its_positions_in_a_sentence() {
        assert_eq!(backup_asks(&[2, 9, 24]), "word 2, word 9 and word 24");
        assert_eq!(backup_asks(&[7]), "word 7");
        assert_eq!(backup_asks(&[]), "");
    }
}
