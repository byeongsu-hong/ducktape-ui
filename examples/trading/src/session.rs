//! Custody for a trading app that must never hold the wallet key.
//!
//! Two secrets, and only one of them can move money out. The wallet key stays
//! wherever the user keeps it. What this process can reach is an *agent key*:
//! a separate keypair the main wallet approved on the exchange, which places
//! and cancels orders, cannot withdraw, and stops working on a date the
//! exchange chose. Losing it costs an approval, not a balance.
//!
//! The secret that unlocks the app at all is held by the platform keychain, so
//! entry is a Touch ID or passkey prompt rather than a password this process
//! stores. Everything here except that one platform call is pure, which is
//! what turns "may this key sign?" into a question with a tested answer.
//!
//! What the public API confirms about the approval, probed against
//! `api.hyperliquid.xyz` while writing this:
//!
//! - The `approveAgent` action carries `hyperliquidChain` (`Mainnet` or
//!   `Testnet`), `signatureChainId`, `agentAddress`, and a `nonce`, plus an
//!   optional `agentName`. Dropping any of the four required ones is a 422;
//!   sending all of them reaches signature recovery and fails there, which is
//!   as far as an unsigned probe can get.
//! - **The action has no validity window.** Adding `validUntil` to it is
//!   rejected outright, so the expiry below is the exchange's to assign and
//!   ours to read back — never ours to ask for. The brief's "validity window"
//!   is real, but it is an output of the approval, not an input.
//! - **The agent address is mandatory and must be a whole address.** Both `""`
//!   and `"0x"` are 422s, so a key with no address is one the exchange would
//!   never have approved — which is why `admit` refuses to hold one.
//! - `{"type":"extraAgents","user":…}` lists live approvals as
//!   `{name, address, validUntil}`, `validUntil` in milliseconds. Read across
//!   1,899 approvals on 47 leaderboard accounts: not one was already lapsed at
//!   the moment of reading, no address was empty or repeated within an account,
//!   38 accounts held more than one key (up to 103), and the longest window ran
//!   ~179 days out, consistent with a 180-day cap. So a listing is a set of
//!   *live* keys, and an account having several is normal rather than odd.
//! - `{"type":"userRole","user":<agent>}` answers
//!   `{"role":"agent","data":{"user":<master>}}` — the exchange's own copy of
//!   which account a key acts for, and the fact `step` refuses a mismatch on.
//!
//! Not verified: the EIP-712 payload the wallet signs, the window a fresh
//! approval is actually granted, and whether approving an address that is
//! already listed extends that entry or replaces it. All three need a funded
//! wallet to produce, so this file never assumes a window — it reads whatever
//! the venue last reported, and treats the newest report as the truth.

#![allow(dead_code)]

use std::fmt;

/// Bytes that must not turn up in a log line. `Debug` prints the length and
/// nothing else, and the type is deliberately not `Clone`: a secret with two
/// owners has two chances to outlive its use.
pub struct Secret(Vec<u8>);

impl Secret {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Named to be uncomfortable at the call site, which is the only place the
    /// bytes have any business being.
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Secret({} bytes)", self.0.len())
    }
}

impl Drop for Secret {
    /// **Nothing in this file can show that this runs.** It is a write to
    /// memory nothing reads again, which the optimiser is entitled to delete,
    /// and no test can look at the freed buffer to check: replace this body
    /// with `{}` and every test below still passes, today and after any test
    /// anyone adds. Read it as hygiene against a heap that later gets reused or
    /// dumped, not as a guarantee — the guarantee costs the `zeroize`
    /// dependency, which is the upgrade if that day comes.
    fn drop(&mut self) {
        // ponytail: unverifiable best-effort wipe; `zeroize` is the upgrade.
        self.0.fill(0);
    }
}

/// What asking the platform to let us in produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unlock {
    /// The prompt was declined or dismissed. Nothing was released, and trying
    /// again is a reasonable thing for the user to do.
    Locked,
    /// There is nothing stored for this account yet, so there was never a
    /// prompt to decline — a first run. Kept apart from `Locked` because the
    /// two need opposite things on screen: asking again fetches this same
    /// answer forever, and `store` is the only thing that changes it.
    Unenrolled,
    /// Touch ID or the passkey answered and the keychain handed back what it
    /// held. Carries nothing: the secret goes to the caller that asked for it,
    /// never into the state machine.
    Platform,
    /// There is no keychain this session can use: either the build has none, or
    /// the one this machine has failed for a reason that is not the user's to
    /// answer. Retrying cannot fix either, which is exactly why it does not
    /// read as `Locked`.
    Unavailable(String),
}

/// One agent wallet the main wallet has approved.
///
/// No key material lives here. This is the public half plus its window, which
/// is everything the session needs to decide whether an order may be signed;
/// the private half belongs to whatever actually signs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentKey {
    /// The agent wallet's address — what `approveAgent` names and what
    /// `extraAgents` lists back.
    pub address: String,
    /// The master account it acts for. `userRole` on the agent address answers
    /// with this same address, so the venue can be asked to confirm it.
    pub account: String,
    /// When this app approved the key, in milliseconds since the epoch. The
    /// venue does not report an approval time, so a key discovered from
    /// `extraAgents` rather than approved here has nothing honest to put in
    /// this field.
    pub approved_at: i64,
    /// `validUntil` exactly as `extraAgents` reports it, in milliseconds.
    pub expires_at: i64,
}

impl AgentKey {
    /// Treated as exclusive: a key at its own `validUntil` is already gone.
    /// Which side of that millisecond the exchange sits on is not something an
    /// unsigned probe can settle, and the cheap error is refusing to sign one
    /// millisecond early rather than sending an order that comes back rejected.
    pub fn live(&self, now_ms: i64) -> bool {
        now_ms < self.expires_at
    }

    /// How much of the window is left. Never negative: a lapsed key has no
    /// remaining time, it has no key.
    ///
    /// Saturating, because the wound-back clock this file is built to survive —
    /// a manual change, an NTP correction, a machine waking with a stale RTC —
    /// is exactly what makes `expires_at - now_ms` overflow, and a countdown
    /// that panics is a worse answer than a countdown that pegs.
    pub fn remaining_ms(&self, now_ms: i64) -> i64 {
        self.expires_at.saturating_sub(now_ms).max(0)
    }
}

/// Where custody currently stands. The app reads this to decide what it is
/// allowed to draw, and only one variant may sign.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Session {
    /// Holding nothing.
    #[default]
    Locked,
    /// Nothing has ever been stored for this account, so there is no prompt to
    /// put up. Not the user's to retry the way `Locked` is: enrolling — a
    /// `store`, which happens outside this model — is what leaves it, and a
    /// `Prompt` after that works exactly as it does from `Locked`.
    Unenrolled,
    /// The platform's prompt is up; waiting on the user's finger.
    Unlocking,
    /// The keychain let us in, so the account can be read. Nothing has been
    /// approved to sign with, so nothing can be traded.
    Unlocked { account: String },
    /// A live approval. The only state in which an order may be signed.
    Ready { key: AgentKey },
    /// The approval outlived its window. Keeps the key on purpose, so the
    /// panel can name what lapsed and offer to approve again.
    Expired { key: AgentKey },
    /// No keychain on this platform. Which keystore a build has is decided at
    /// compile time, so a second prompt would fetch the same refusal: no event
    /// but `Lock` leaves this state, and none can carry it to `Ready`. That is
    /// the whole point of giving it a state of its own.
    Unavailable { reason: String },
}

/// Everything that can move custody. Times are milliseconds since the epoch,
/// the unit the exchange reports `validUntil` in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The user asked to come in; the platform prompt goes up.
    Prompt,
    /// The platform answered the prompt. `account` is the wallet address the
    /// released secret belongs to, and is empty unless the outcome is
    /// `Platform`.
    Unlocked { outcome: Unlock, account: String },
    /// An `approveAgent` landed, or `extraAgents` listed a key we already had.
    Approved { key: AgentKey, now: i64 },
    /// The clock moved.
    Tick(i64),
    /// The user left, the machine slept, or anything else that must forget the
    /// key.
    Lock,
}

/// The whole custody model: current state plus one event gives the next state.
/// Nothing here reads a clock, a keychain, or a network — every input arrives
/// in the event — which is what makes the rules below testable rather than
/// hopeful.
pub fn step(state: Session, event: Event) -> Session {
    match (state, event) {
        // Locking is the one transition with no conditions on it. Whatever was
        // held is dropped, from wherever we were.
        (_, Event::Lock) => Session::Locked,

        // Only an app that is not already in prompts. A sheet over a session
        // that is mid-trade is a way to lose an order, not to gain security.
        // `Unavailable` is left out on purpose: the keystore is chosen at
        // compile time, so a second prompt would only fetch the same refusal.
        // `Unenrolled` is in, because the thing that fixes it — storing a
        // secret — happens outside this model, and the prompt after it must
        // work.
        (Session::Locked | Session::Unenrolled, Event::Prompt) => Session::Unlocking,

        // An answer is only an answer to a prompt that is still up. A slow
        // Touch ID landing after the user locked the app must not re-open it.
        (Session::Unlocking, Event::Unlocked { outcome, account }) => match outcome {
            // An unlock that cannot say whose account it opened has opened
            // nothing there is anything to do with.
            Unlock::Platform if !account.is_empty() => Session::Unlocked { account },
            Unlock::Platform | Unlock::Locked => Session::Locked,
            Unlock::Unenrolled => Session::Unenrolled,
            Unlock::Unavailable(reason) => Session::Unavailable { reason },
        },

        // Every approval takes the same road, from whichever state can hold
        // one. `Unlocked` is the first approval, `Expired` is the normal place
        // to be re-approved from, and `Ready` is a renewal: the venue lists
        // only live keys and lets one account hold many, so an approval
        // arriving here is its latest word on what this account may sign with,
        // and the newest word wins. Renewing early therefore extends the
        // window instead of being dropped on the floor, and a key that lapsed
        // between the listing and this event lands in `Expired` — refusing to
        // sign is the cheap error, signing on a dead key is not.
        (state, Event::Approved { key, now }) if approvable(&state, &key) => admit(key, now),

        (Session::Ready { key }, Event::Tick(now)) => admit(key, now),

        // Expiry is one-way. A clock that steps backwards — a manual change, an
        // NTP correction, a machine waking with a stale RTC — must not hand
        // back a key the exchange has already stopped honouring. Only an
        // `Approved` above, carrying a fresh window from the venue, gets out.
        (state @ Session::Expired { .. }, Event::Tick(_)) => state,

        // Everything else changes nothing: a prompt answered that nobody
        // raised, a tick in a session holding no key, and — the one worth
        // naming — an approval that fails `approvable`, which is somebody
        // else's key or a key with no address to sign with.
        (state, _) => state,
    }
}

/// Whether an approval is this session's to hold at all, asked before its
/// window is even looked at.
///
/// The account must be the one already open, because however valid somebody
/// else's approval is, it is not this session's to sign with — and `userRole`
/// on the agent address is the venue's own copy of that pairing. Both halves
/// must also be non-empty. The exchange 422s an `approveAgent` carrying `""`
/// or `"0x"` as the agent address, so a key without one was never approved by
/// anybody, and holding it would let `Ready` mean "may sign, with what?"; the
/// account is checked here rather than trusted from the unlock because
/// `Session` is a public enum with public fields, so "unreachable" only holds
/// for states this module built.
fn approvable(state: &Session, key: &AgentKey) -> bool {
    !key.address.is_empty()
        && !key.account.is_empty()
        && account(state) == Some(key.account.as_str())
}

/// A key that belongs to this account is admitted on its window alone, so an
/// approval that arrives already lapsed reads as expired rather than as
/// permission to trade.
fn admit(key: AgentKey, now: i64) -> Session {
    if key.live(now) {
        Session::Ready { key }
    } else {
        Session::Expired { key }
    }
}

/// The one question the ticket asks before it enables its buttons.
///
/// Takes the clock because the alternative cannot be right: a window closes on
/// a schedule the exchange set, not when an event happens to arrive, so a
/// `Ready` that answered on its variant alone would keep saying yes through
/// every millisecond between expiry and the next `Tick` — and would say yes
/// forever if the ticks stop, which is precisely when a laptop that slept
/// through the expiry starts asking. The state is a claim about the past; only
/// `now_ms` makes it a claim about now.
pub fn can_trade(state: &Session, now_ms: i64) -> bool {
    matches!(state, Session::Ready { key } if key.live(now_ms))
}

/// The agent key the session holds, live or lapsed, so the panel can show the
/// window running down and say which key ran out.
pub fn agent(state: &Session) -> Option<&AgentKey> {
    match state {
        Session::Ready { key } | Session::Expired { key } => Some(key),
        _ => None,
    }
}

/// Which account this session is for, in every state that knows one.
pub fn account(state: &Session) -> Option<&str> {
    match state {
        Session::Unlocked { account } => Some(account),
        Session::Ready { key } | Session::Expired { key } => Some(&key.account),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeystoreError {
    pub message: String,
}

impl KeystoreError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

/// What reading the keychain for an account produced.
///
/// Three answers rather than an `Option`, because reading is *also* what raises
/// the Touch ID sheet — the bytes cannot come back until the guard is
/// satisfied. So a decline reaches `load` as a status code just as it reaches
/// `prompt`, and it is neither a secret nor an empty keychain. `Option` had
/// nowhere to put it, which is how it ended up reported as a broken keychain.
#[derive(Debug)]
pub enum Held {
    /// The keychain released the bytes.
    Secret(Secret),
    /// Nothing stored for this account yet — a first run. `store` is the fix.
    Missing,
    /// The user declined the sheet the read raised, or could not prove it was
    /// them. Nothing is broken, and asking again is reasonable.
    Declined,
}

/// What an item needs guarding with, which is not the same question for every
/// item this app files.
///
/// **The owner's decision, 2026-08-10.** A bare secret is protected by the
/// item's own access control and by nothing else, so it carries one. A sealed
/// blob is protected by the Secure Enclave key that opens it — which already
/// demands biometry *per use* — so an access control on the item as well is a
/// second lock on an empty box, and it is not free: on macOS the read that
/// satisfies it is a sheet, and putting one in front of ciphertext costs a
/// prompt to protect nothing an attacker wants.
///
/// A parameter rather than sniffing the payload for the sealed marker. The
/// sniff would be a smaller diff and would tie *storage policy* to *payload
/// content*, which is the kind of implicit coupling that is right until the day
/// something else starts with the same bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guard {
    /// The item is the secret. Its access control is the whole protection.
    UserPresence,
    /// The item is ciphertext for a key that guards itself. See above.
    Sealed,
}

/// The three things a platform has to be able to do for the app to have an
/// unlock at all. Nothing else in this file touches the platform, which is why
/// everything else in this file is decidable on any machine.
///
/// The return types *are* the seam. A keystore says no for three different
/// reasons and only one of them is a fault: the user declined (ask again),
/// nothing has been stored for this account yet (the state every install starts
/// in), and the keychain itself failed. Both readers say all three —
/// `Unlock::Locked` / `Unlock::Unenrolled` / `Unlock::Unavailable` for the door,
/// `Held::Declined` / `Held::Missing` / `Err` for the bytes. Collapsing any two
/// of them leaves the app either nagging about a fault that is not one, or
/// swallowing one that is.
pub trait Keystore {
    /// Put the platform's own prompt on screen — Touch ID, or the passkey
    /// sheet — and report what came of it.
    ///
    /// Takes the account because macOS has no free-standing "prove it is you"
    /// dialog to raise: the sheet appears *because* something touched a guarded
    /// item, so which item is part of the asking. `reason` names what the app
    /// was trying to do, for the message a failure carries.
    fn prompt(&self, account: &str, reason: &str) -> Unlock;
    /// Replace whatever is stored for this account. Must not lose the secret it
    /// was asked to replace when it fails — see the macOS impl for the price of
    /// that.
    fn store(&self, account: &str, secret: &Secret, guard: Guard) -> Result<(), KeystoreError>;
    /// Read the secret, raising whatever prompt guards it. `Err` is the
    /// keychain itself failing and only that; a first run and a decline are
    /// `Held::Missing` and `Held::Declined`, because they are answers, not
    /// faults, and the app acts differently on each.
    fn load(&self, account: &str) -> Result<Held, KeystoreError>;
}

/// Whichever keystore this build actually has. The app names this one type;
/// which implementation answers is the platform's business.
pub struct PlatformKeystore;

/// What turns a secret into the bytes that are allowed to sit in a keychain,
/// and back again.
///
/// **The owner's requirement, 2026-08-10: the stored form is ciphertext.** The
/// keychain already encrypts what it holds — a data-protection item is sealed
/// at rest under a class key the Secure Enclave protects — so this is not the
/// first layer of encryption over the account's key. It is the second, and it
/// exists because the first one is the *platform's* judgement about who may
/// read the item. Anything that gets past that judgement — an entitlement that
/// widens an access group, an access control that did not take, a future path
/// in this app that reads without the guard — comes away with bytes that still
/// need a separate assertion against a key that cannot leave the chip.
///
/// Sealing takes only the public half, so writing costs no prompt. Opening is
/// the private half, which is the assertion.
pub trait Wrap {
    /// Seal a secret for this account. No prompt: the public half is enough.
    fn seal(&self, account: &str, plain: &Secret) -> Result<Vec<u8>, KeystoreError>;
    /// Open one, which is what raises the biometric assertion. Answers `Held`
    /// for the reason `load` does — a decline is not a fault.
    fn open(&self, account: &str, sealed: &[u8]) -> Result<Held, KeystoreError>;
}

/// Whichever wrapper this build has, on the pattern `PlatformKeystore` already
/// sets: one name, and the platform decides what answers.
pub struct PlatformWrap;

/// What marks a keychain item as *this* scheme's ciphertext.
///
/// Present, and the rest is a sealed blob. Absent, and the item predates the
/// envelope and holds a bare secret — see `load_sealed`. The version digit is
/// here so a second scheme can be told from this one by looking rather than by
/// guessing from length.
const SEALED: &[u8] = b"ducktape-sealed-1:";

/// Store a secret as ciphertext.
///
/// Two steps and no cleverness: seal, then hand the sealed bytes to the same
/// keychain path an unsealed secret took. Everything the keychain does about
/// replacement, preservation and access control is unchanged and still its own
/// — this only changes what it is handed.
///
/// The sheet arithmetic, since this is where it is decided. A wallet read is
/// **one** assertion: the item carries no access control of its own (see
/// `Guard`), and the Enclave key that opens the blob demands biometry once, per
/// use. It was briefly two, and that was a lock on an empty box.
///
/// ponytail: a *session* unlock still costs one assertion per enrolled network,
/// because each trading key is its own guarded item and macOS raises a sheet
/// per guarded read. Collapsing those needs one `LAContext` shared across the
/// reads via `kSecUseAuthenticationContext`. `ItemSearchOptions` exposes that
/// setter safely, but its bound is core-foundation's `TCFType` and an
/// `LAContext` is an Objective-C object — `Retained<LAContext>` does not
/// implement it, and every bridge across is `unsafe`, which this workspace
/// forbids in `Cargo.toml` rather than by habit. The upgrade path is upstream:
/// an overload on `security-framework` taking objc2's type, or the `TCFType`
/// impl. Filed as its own task. Until then an owner answers once per network,
/// once per session, and the common path — reading the wallet — is one.
pub fn store_sealed(
    wrap: &impl Wrap,
    store: &impl Keystore,
    account: &str,
    secret: &Secret,
) -> Result<(), KeystoreError> {
    let mut blob = SEALED.to_vec();
    blob.extend_from_slice(&wrap.seal(account, secret)?);
    store.store(account, &Secret::new(blob), Guard::Sealed)
}

/// Read one back, and bring an item from before the envelope forward.
///
/// **The migration, and it is one moment rather than a fallback.** #523 stored
/// the account key as itself. An item without the marker is one of those, so it
/// is re-sealed on the spot and the plaintext form is gone the moment the
/// replacement lands — `store` replaces rather than adds, so there is nothing
/// left to delete afterwards and nothing left for a later read to find.
///
/// The arm can be deleted once no machine holds a #523 item. It is dated rather
/// than permanent on purpose: a branch that reads keychain bytes as a bare key
/// is exactly the thing this change exists to stop, and leaving it forever
/// would mean the scheme's own escape hatch never closes.
pub fn load_sealed(
    wrap: &impl Wrap,
    store: &impl Keystore,
    account: &str,
) -> Result<Held, KeystoreError> {
    let stored = match store.load(account)? {
        Held::Secret(stored) => stored,
        // A first run and a decline are answers rather than ciphertext, and
        // they travel unchanged.
        other => return Ok(other),
    };
    let Some(sealed) = stored.expose().strip_prefix(SEALED) else {
        // Pre-envelope, 2026-08-10. Re-sealed here, handed back once, and the
        // bare form replaced in the same breath.
        let plain = Secret::new(stored.expose().to_vec());
        store_sealed(wrap, store, account, &plain)?;
        return Ok(Held::Secret(plain));
    };
    wrap.open(account, sealed)
}

/// Why a keychain call came back with nothing, in the only three flavours the
/// app can act on differently. Sorting a status code into one of these is the
/// entire judgement this file makes about the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// No item for this account.
    Missing,
    /// The user said no, could not prove it was them, or there was no way to
    /// ask. All three are "not now", and all three are worth asking again.
    Declined,
    /// The keychain itself is unhappy. Nothing the user does at a sheet changes
    /// it, so re-prompting only burns their patience.
    Failed,
}

/// `OSStatus` values from `<Security/SecBase.h>`, which `security-framework`
/// does not re-export.
///
/// Written out here, outside any `cfg`, rather than taken from
/// `security-framework-sys`: the classification below is the part of this file
/// a decline or a first run turns on, and keeping it platform-free is what lets
/// it be tested on a machine that has no Keychain to ask.
const ITEM_NOT_FOUND: i32 = -25300;
const USER_CANCELED: i32 = -128;
const AUTH_FAILED: i32 = -25293;
const INTERACTION_NOT_ALLOWED: i32 = -25308;
/// `errSecMissingEntitlement`. Reported by the owner's Mac, 2026-08-10, on the
/// first real run of the envelope — see `described`.
const MISSING_ENTITLEMENT: i32 = -34018;

fn refusal(status: i32) -> Refusal {
    match status {
        ITEM_NOT_FOUND => Refusal::Missing,
        USER_CANCELED | AUTH_FAILED | INTERACTION_NOT_ALLOWED => Refusal::Declined,
        _ => Refusal::Failed,
    }
}

/// A read that released nothing, as the unlock the session model consumes.
///
/// The one rule this whole seam exists for: a user declining the sheet is
/// `Locked`, which the app can offer to retry, and never `Unavailable`, which
/// it cannot. A first run is its own answer again — the door is shut either
/// way, but "try again" is wrong advice for an account that has nothing behind
/// the door yet, and `store` rather than a second sheet is what fixes it.
fn unlock(refusal: Refusal, failure: String) -> Unlock {
    match refusal {
        Refusal::Missing => Unlock::Unenrolled,
        Refusal::Declined => Unlock::Locked,
        Refusal::Failed => Unlock::Unavailable(failure),
    }
}

/// The same sorting for the caller that wanted the bytes rather than the door.
///
/// It lives here, beside `unlock`, rather than inline in the macOS `load`,
/// because these two are the only places a status code becomes an outcome and
/// both must be checkable on a machine with no Keychain. Inline, the arm that
/// turned a decline into an error was invisible to every test on this box.
fn held(refusal: Refusal, failure: String) -> Result<Held, KeystoreError> {
    match refusal {
        Refusal::Missing => Ok(Held::Missing),
        Refusal::Declined => Ok(Held::Declined),
        Refusal::Failed => Err(KeystoreError::new(failure)),
    }
}

/// What `-34018` is actually about, which is not the call that got it.
///
/// **Reported from the owner's Mac, 2026-08-10, and the first thing a real Mac
/// taught this seam.** The Secure Enclave and the data-protection keychain
/// serve code that is signed and carries the keychain entitlements; a binary
/// `cargo run` built and nobody signed has neither, so the Enclave declines to
/// make it a key. The system's own words for that status are
/// "missing entitlement", and `SecKey`'s failure prints the key it was asked
/// for — a page of curve parameters and coordinates in front of a reader whose
/// actual problem is one build step.
///
/// So this status alone gets a sentence about the *binary* rather than about
/// the call. Everything else keeps the platform's own wording, because for
/// everything else the platform's wording is the true one.
const UNSIGNED: &str = "This build is not code-signed, and the Secure Enclave will not make a key \
                        for an unsigned binary. Nothing has been stored. Build, sign and run in \
                        one step with `scripts/sign-dev.sh -p trading-example`.";

/// The sentence a failed platform call is owed.
///
/// It sits out here beside `refusal`, `unlock` and `held` for their reason: it
/// is a judgement about a status code, it is what a person reads when the
/// platform says no, and every arm of it has to be checkable on a machine with
/// no Keychain to produce one. The system's own words arrive as a parameter
/// rather than being looked up here, which is the whole of what keeps this
/// function portable.
fn described(status: i32, doing: &str, system: &str) -> String {
    if status == MISSING_ENTITLEMENT {
        return UNSIGNED.to_owned();
    }
    format!("{doing}: {system} ({status})")
}

/// macOS: one generic-password item per account, in the data-protection
/// keychain, guarded so that reading it costs a Touch ID and so that the item
/// never leaves this Mac.
///
/// Three choices, none of them decoration:
///
/// - `use_protected_keychain` asks for the data-protection keychain. On the
///   legacy file keychain macOS ignores `kSecAttrAccessible*` outright, so
///   without it the "this device only" half below would be silently dropped and
///   the secret would be free to ride a Migration Assistant transfer onto the
///   next Mac. That call is what the dependency's `OSX_10_15` feature is for.
/// - `AccessibleWhenUnlockedThisDeviceOnly` keeps the item out of iCloud
///   Keychain and out of every backup and transfer, and unreadable while the
///   screen is locked.
/// - `USER_PRESENCE` is `kSecAccessControlUserPresence`: biometry, falling back
///   to the passcode. Deliberately not `BIOMETRY_CURRENT_SET`, which destroys
///   the item the moment a finger is enrolled or removed. The agent key behind
///   this is cheap to re-approve, but silently losing the app's own unlock
///   because somebody added a thumb is a support ticket, not a security win.
///
/// `reason` reaches the user only inside a failure message. The sheet's own
/// wording is the system's, because choosing it means setting
/// `kSecUseOperationPrompt` in a query dictionary built by hand — raw FFI, and
/// this crate forbids `unsafe`.
#[cfg(target_os = "macos")]
mod keychain {
    use super::{
        Held, Keystore, KeystoreError, PlatformKeystore, Refusal, Secret, Unlock, described, held,
        refusal, unlock,
    };

    use super::{Guard, PlatformWrap, Wrap};

    use security_framework::access_control::{ProtectionMode, SecAccessControl};
    use security_framework::base::Error;
    use security_framework::item::{
        ItemClass, ItemSearchOptions, KeyClass, Location, Reference, SearchResult,
    };
    use security_framework::key::{Algorithm, GenerateKeyOptions, KeyType, SecKey, Token};
    use security_framework::passwords::{
        AccessControlOptions, PasswordOptions, delete_generic_password_options, generic_password,
        set_generic_password_options,
    };

    /// What the item is filed under, and what a user poking around Keychain
    /// Access sees.
    const SERVICE: &str = "dev.ducktape.trading";
    const LABEL: &str = "Ducktape trading unlock";

    /// Service, account, and the two attributes that decide *which* keychain is
    /// being addressed. Every call goes through here, because a read that
    /// forgot `use_protected_keychain` would look in the legacy keychain and
    /// report "nothing stored" for an item that is sitting right there.
    fn query(account: &str) -> PasswordOptions {
        let mut options = PasswordOptions::new_generic_password(SERVICE, account);
        options.use_protected_keychain();
        options.set_access_synchronized(Some(false));
        options
    }

    /// Reading is what raises the prompt: the bytes cannot come back until the
    /// access control is satisfied. `Err` carries the raw `OSStatus` and
    /// nothing else, so deciding what one *means* stays in `refusal`, where a
    /// machine with no Keychain can still test it.
    fn read(account: &str) -> Result<Vec<u8>, i32> {
        generic_password(query(account)).map_err(Error::code)
    }

    /// The system's own wording for a status, next to what we were doing when
    /// it came back. A bare `-25308` in a log is a puzzle; the pair is a cause.
    ///
    /// The pairing itself is `described`, one level up and outside this `cfg`,
    /// because one status is owed a different sentence and that judgement has
    /// to be testable on a machine with no Keychain. This is the half only a
    /// Mac can supply: the system's own words for the code.
    fn describe(status: i32, doing: &str) -> String {
        described(status, doing, &Error::from_code(status).to_string())
    }

    /// One item, built the way *this* code says it should be: the guard is made
    /// fresh each call because `set_access_control` consumes one, and putting a
    /// replaced secret back needs its own.
    fn add(account: &str, secret: &Secret, guard: Guard) -> Result<(), KeystoreError> {
        let mut options = query(account);
        options.set_label(LABEL);
        // A sealed blob gets no access control of its own. What opens it is a
        // Secure Enclave key with `BIOMETRY_CURRENT_SET` on every use, so the
        // assertion still happens — once, over the thing that matters, instead
        // of twice with the first one standing over ciphertext.
        if guard == Guard::UserPresence {
            let control = SecAccessControl::create_with_protection(
                Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
                AccessControlOptions::USER_PRESENCE.bits(),
            )
            .map_err(|error| {
                KeystoreError::new(describe(error.code(), "building the Touch ID guard"))
            })?;
            options.set_access_control(control);
        }
        set_generic_password_options(secret.expose(), options)
            .map_err(|error| KeystoreError::new(describe(error.code(), "storing the secret")))
    }

    /// Written on Linux, type-checked for `aarch64-apple-darwin`, never once
    /// run. Everything below is a claim about a machine this was not built on,
    /// so it lives here rather than in a pull request nobody reads twice.
    ///
    /// **None of it is reachable from `cargo run`, which is what the first real
    /// Mac reported.** Item 8 was the guess, and it was right: an unsigned
    /// binary gets `-34018` from the data-protection keychain and from the
    /// Enclave alike, so the list below is owed a *signed* build to run
    /// against. `scripts/sign-dev.sh` is what makes one.
    ///
    /// A macOS machine still has to demonstrate all of this:
    ///
    /// 1. The sheet appears at all, and it is Touch ID with a passcode fallback
    ///    — not the password dialog the legacy login keychain puts up, which
    ///    would mean `use_protected_keychain` did not take.
    /// 2. Declining it yields `Unlock::Locked` from `prompt` and
    ///    `Held::Declined` from `load` — never an error from either — and a
    ///    later `prompt` still succeeds. Which status a declined sheet actually
    ///    returns — `-128`, `-25293`, or something this file does not name — is
    ///    the assumption `refusal` rests on, and a wrong guess shows up only as
    ///    a user saying no that the app reports as a broken keychain.
    /// 3. For an account that was never stored, `load` is `Ok(Held::Missing)`
    ///    and `prompt` is `Unlock::Unenrolled` — neither is an error, and
    ///    neither is the same answer a decline gives.
    /// 4. Whether `prompt` immediately followed by `load` raises one sheet or
    ///    two. Two is a real UX bug, and the fix is a shared `LAContext` — a
    ///    second dependency, and one this change would not add on a guess.
    /// 5. `store` twice for one account leaves exactly one item, and the
    ///    survivor carries the access control *this* code asked for rather than
    ///    the one the first item was born with. That is what the delete-then-add
    ///    below is for.
    /// 6. What that replace costs in sheets. `store` over an existing item
    ///    reads it first so a failing add cannot destroy it, and that read is a
    ///    guarded read: expect a sheet on re-enrolment and none on a first run,
    ///    and confirm the delete itself raises no second one.
    /// 7. That the preserve actually preserves. Force the add to fail — store a
    ///    secret, then store again with the keychain made to refuse — and check
    ///    the previous secret still loads and the error says it was put back.
    ///    This is the one claim in `store` that no Linux test can reach.
    /// 8. **Answered, 2026-08-10, by the owner's Mac.** An unsigned `cargo run`
    ///    gets `-34018`, `errSecMissingEntitlement`, at the first Enclave key
    ///    it asks for. The fix was never in this file: Apple's TN3137 makes
    ///    Enclave access a data-protection keychain feature, and TN3125 makes
    ///    `keychain-access-groups` a restricted entitlement no ad-hoc signature
    ///    can carry — so what is missing is a `.app`, a provisioning profile
    ///    and a real identity. `scripts/sign-dev.sh` assembles the three. What
    ///    this file owed was the sentence, and `described` is it. What is still
    ///    unknown: whether a *free* Apple ID's Personal Team may authorize the
    ///    entitlement at all, which Apple does not document either way.
    /// 9. The item does not appear on a second Mac signed into the same iCloud
    ///    account, and does not survive Migration Assistant.
    impl Keystore for PlatformKeystore {
        fn prompt(&self, account: &str, reason: &str) -> Unlock {
            match read(account) {
                // Straight into a `Secret` on its way to being dropped, so the
                // copy the Keychain handed back is overwritten before this
                // returns. `prompt` answers whether the user is there, never
                // with what they unlocked.
                Ok(bytes) => {
                    drop(Secret::new(bytes));
                    Unlock::Platform
                }
                Err(status) => unlock(refusal(status), describe(status, reason)),
            }
        }

        /// Replace rather than update, and hold the old secret across the gap.
        ///
        /// The delete is deliberate: `set_generic_password_options` falls back
        /// to `SecItemUpdate` on a duplicate, which keeps whatever access
        /// control the *existing* item was born with, so a build that tightened
        /// the guard would silently not apply it to anyone who already had a
        /// secret. There is no atomic way to do that — the item is keyed by
        /// service and account, so the old one has to be gone before the new
        /// one can land — which leaves a window where a failing add would have
        /// left the account holding nothing at all.
        ///
        /// So the old bytes are read out first and put back if the add fails.
        /// Reading them raises the same sheet a `load` does, and that is the
        /// honest price: an item guarded by user presence cannot be preserved
        /// without presence. It buys the two outcomes worth having — a decline
        /// aborts the replace with the old secret untouched, and a failed add
        /// is followed by a restore.
        fn store(&self, account: &str, secret: &Secret, guard: Guard) -> Result<(), KeystoreError> {
            let previous = match read(account) {
                Ok(bytes) => Some(Secret::new(bytes)),
                Err(status) if refusal(status) == Refusal::Missing => None,
                // ponytail: an item this process cannot read is an item it will
                // not overwrite either, so a keychain that fails or declines
                // here wedges `store` until the user retries or deletes the item
                // in Keychain Access. Refusing beats deleting a secret to find
                // out whether the add would have worked.
                Err(status) => {
                    return Err(KeystoreError::new(describe(
                        status,
                        "reading the secret being replaced",
                    )));
                }
            };

            // A delete that fails is not worth reporting on its own: there was
            // either nothing there, or the add below is about to fail with the
            // real reason.
            let _ = delete_generic_password_options(query(account));

            let Err(failure) = add(account, secret, guard) else {
                return Ok(());
            };
            let Some(previous) = previous else {
                return Err(failure);
            };
            // Put back exactly what was replaced. If even that fails the secret
            // really is gone, and the only thing left to do about it is say so
            // in words a caller can act on.
            match add(account, &previous, guard) {
                Ok(()) => Err(KeystoreError::new(format!(
                    "{}; the secret it replaced is still there",
                    failure.message
                ))),
                Err(restore) => Err(KeystoreError::new(format!(
                    "{}; and {} — the secret it replaced is gone and has to be stored again",
                    failure.message, restore.message
                ))),
            }
        }

        fn load(&self, account: &str) -> Result<Held, KeystoreError> {
            match read(account) {
                Ok(bytes) => Ok(Held::Secret(Secret::new(bytes))),
                // Reading is what raised the sheet, so `held` — not an `Err` —
                // is what a decline comes back through.
                Err(status) => held(refusal(status), describe(status, "reading the secret")),
            }
        }
    }

    /// What the wrapping key is filed under. One key per account, so forgetting
    /// an account is deleting its key and its item and nothing else.
    fn wrap_label(account: &str) -> String {
        format!("{SERVICE}.wrap:{}", account.trim().to_lowercase())
    }

    /// The guard on the wrapping key, which is the whole of what protects the
    /// account's own key on this machine.
    ///
    /// `PRIVATE_KEY_USAGE` is what makes the flags apply to *using* the key
    /// rather than to reading an item. `BIOMETRY_CURRENT_SET` is deliberately
    /// stricter than the item's `USER_PRESENCE`: enrolling a new finger
    /// invalidates the key, so somebody who learns the passcode and adds their
    /// own biometry cannot then unwrap the account. That is a real trade — a
    /// user who re-enrols their own finger loses the wrapping key and has to
    /// import the phrase again — and it is the right way round for a key that
    /// signs enrolments. The phrase is written down; the convenience is not
    /// worth the other outcome.
    fn wrap_guard() -> Result<SecAccessControl, KeystoreError> {
        SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenPasscodeSetThisDeviceOnly),
            (AccessControlOptions::PRIVATE_KEY_USAGE | AccessControlOptions::BIOMETRY_CURRENT_SET)
                .bits(),
        )
        .map_err(|error| {
            KeystoreError::new(describe(error.code(), "building the wrapping key's guard"))
        })
    }

    /// The wrapping key for this account, or nothing when none has been made.
    ///
    /// A search rather than a handle kept somewhere: the key outlives the
    /// process, and a reference cached across an unlock is a reference that
    /// survives a lock.
    fn wrap_key(account: &str) -> Result<Option<SecKey>, KeystoreError> {
        let found = ItemSearchOptions::new()
            .class(ItemClass::key())
            .key_class(KeyClass::private())
            .label(&wrap_label(account))
            .load_refs(true)
            .limit(1)
            .search();
        match found {
            Ok(results) => Ok(results.into_iter().find_map(|result| match result {
                SearchResult::Ref(Reference::Key(key)) => Some(key),
                _ => None,
            })),
            Err(error) if refusal(error.code()) == Refusal::Missing => Ok(None),
            Err(error) => Err(KeystoreError::new(describe(
                error.code(),
                "looking for the wrapping key",
            ))),
        }
    }

    /// Mint one, in the Secure Enclave, permanently.
    ///
    /// P-256 because it is the only curve the Enclave generates, and the
    /// data-protection keychain because a Secure Enclave key cannot live in the
    /// other one. `is_permanent` is not a setter: `security-framework` derives
    /// it from the location, which is why the location is set even though
    /// nothing here reads it back by hand.
    fn make_wrap_key(account: &str) -> Result<SecKey, KeystoreError> {
        let mut options = GenerateKeyOptions::default();
        options
            .set_key_type(KeyType::ec_sec_prime_random())
            .set_size_in_bits(256)
            .set_token(Token::SecureEnclave)
            .set_location(Location::DataProtectionKeychain)
            .set_label(wrap_label(account))
            .set_access_control(wrap_guard()?);
        // Through `describe` rather than printing the error: `SecKey`'s own
        // `Display` renders the key it was asked to make — curve, coordinates
        // and all — which is a page of noise in front of the one status that
        // matters. This is the call the owner's Mac refused with `-34018`.
        SecKey::new(&options).map_err(|error| {
            KeystoreError::new(describe(
                error.code() as i32,
                "making the wrapping key in the Secure Enclave",
            ))
        })
    }

    /// ECIES over the Enclave's P-256 key: an ephemeral key agreement, X9.63
    /// KDF, AES-GCM with its IV drawn from the KDF output — the variant
    /// `SecKey.h` directs new code to; the fixed-IV one is documented legacy.
    /// The sender's half is generated per call and thrown away, so the same
    /// secret sealed twice gives different bytes and neither can be opened by
    /// anything but the key in the chip.
    const ENVELOPE: Algorithm = Algorithm::ECIESEncryptionCofactorVariableIVX963SHA256AESGCM;

    /// Written on Linux, type-checked for `aarch64-apple-darwin`, never once
    /// run — the same standing as everything else in this module. What a Mac
    /// still has to demonstrate is in `custody.rs`'s list.
    impl Wrap for PlatformWrap {
        fn seal(&self, account: &str, plain: &Secret) -> Result<Vec<u8>, KeystoreError> {
            let key = match wrap_key(account)? {
                Some(key) => key,
                None => make_wrap_key(account)?,
            };
            // The public half, which is not guarded: sealing raises no sheet,
            // and an owner storing a wallet is already past the one prompt an
            // import costs.
            let public = key.public_key().ok_or_else(|| {
                KeystoreError::new("the wrapping key has no public half".to_owned())
            })?;
            public
                .encrypt_data(ENVELOPE, plain.expose())
                .map_err(|error| {
                    KeystoreError::new(describe(error.code() as i32, "sealing the secret"))
                })
        }

        fn open(&self, account: &str, sealed: &[u8]) -> Result<Held, KeystoreError> {
            // No key means nothing this app sealed, which is the same answer as
            // an empty keychain rather than a fault: the fix is to store one.
            let Some(key) = wrap_key(account)? else {
                return Ok(Held::Missing);
            };
            match key.decrypt_data(ENVELOPE, sealed) {
                Ok(plain) => Ok(Held::Secret(Secret::new(plain))),
                // Using the key is what raised the sheet, so a decline arrives
                // here and is an answer. `CFError` carries the same `OSStatus`
                // space the keychain does, so `refusal` sorts it.
                Err(error) => held(
                    refusal(error.code() as i32),
                    describe(error.code() as i32, "opening the sealed secret"),
                ),
            }
        }
    }
}

/// Everywhere else: there is no keychain to ask, and pretending otherwise would
/// mean inventing somewhere to keep a secret. The session model has a state for
/// this, so the app can say why it cannot trade instead of failing at the
/// moment somebody clicks buy.
///
/// `load` refuses rather than answering `Held::Missing`: "nothing stored yet" is
/// an invitation to store one, and there is nowhere here to put it.
#[cfg(not(target_os = "macos"))]
const UNAVAILABLE: &str = "no platform keychain on this build";

/// No Secure Enclave here either, and the same honest refusal: a build with
/// nowhere to keep a key has nowhere to keep the key that would wrap it.
#[cfg(not(target_os = "macos"))]
impl Wrap for PlatformWrap {
    fn seal(&self, _account: &str, _plain: &Secret) -> Result<Vec<u8>, KeystoreError> {
        Err(KeystoreError::new(UNAVAILABLE.to_owned()))
    }

    fn open(&self, _account: &str, _sealed: &[u8]) -> Result<Held, KeystoreError> {
        Err(KeystoreError::new(UNAVAILABLE.to_owned()))
    }
}

#[cfg(not(target_os = "macos"))]
impl Keystore for PlatformKeystore {
    fn prompt(&self, _account: &str, _reason: &str) -> Unlock {
        Unlock::Unavailable(UNAVAILABLE.to_owned())
    }

    fn store(&self, _account: &str, _secret: &Secret, _guard: Guard) -> Result<(), KeystoreError> {
        Err(KeystoreError::new(UNAVAILABLE.to_owned()))
    }

    fn load(&self, _account: &str) -> Result<Held, KeystoreError> {
        Err(KeystoreError::new(UNAVAILABLE.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;
    use std::collections::HashMap;

    /// The platform, minus the platform: a fixed answer to the prompt and a map
    /// standing in for the keychain.
    struct Memory {
        answer: Unlock,
        held: RefCell<HashMap<String, Vec<u8>>>,
        /// What each item was asked to be guarded with, so a test can hold the
        /// one thing the macOS `add` does with that answer.
        guards: RefCell<HashMap<String, Guard>>,
    }

    impl Memory {
        fn new(answer: Unlock) -> Self {
            Self {
                answer,
                held: RefCell::new(HashMap::new()),
                guards: RefCell::new(HashMap::new()),
            }
        }
    }

    impl Keystore for Memory {
        fn prompt(&self, _account: &str, _reason: &str) -> Unlock {
            self.answer.clone()
        }

        fn store(&self, account: &str, secret: &Secret, guard: Guard) -> Result<(), KeystoreError> {
            self.held
                .borrow_mut()
                .insert(account.to_owned(), secret.expose().to_vec());
            self.guards.borrow_mut().insert(account.to_owned(), guard);
            Ok(())
        }

        fn load(&self, account: &str) -> Result<Held, KeystoreError> {
            Ok(match self.held.borrow().get(account) {
                Some(bytes) => Held::Secret(Secret::new(bytes.clone())),
                None => Held::Missing,
            })
        }
    }

    /// Any old reason string, for the states that only need to hold one. The
    /// text a real refusal carries is the platform's and is checked where the
    /// platform is.
    const NO_KEYCHAIN: &str = "no platform keychain on this build";

    const HOUR: i64 = 3_600_000;
    const DAY: i64 = 24 * HOUR;

    /// A real account and one of its real approvals, read off `extraAgents`
    /// rather than invented, so the shape of an address and the size of a
    /// window are the exchange's rather than mine.
    const ACCOUNT: &str = "0x1025d5c2057058ffd8acf57109c5f649c11bdc11";
    const AGENT: &str = "0x13070cb3597c75100928720060c7acff4d22bc09";
    const OTHER: &str = "0x27c9fa86c91b84ddfa15de58c482ff662498d65d";
    /// The wall clock when that listing was read. The venue reports no approval
    /// time, so this stands in for the moment the app would have recorded one.
    const APPROVED_AT: i64 = 1_786_172_634_169;
    /// `validUntil` as the venue reported it for `AGENT`.
    const VALID_UNTIL: i64 = 1_793_781_655_468;
    /// The window the venue actually granted, so a renewal in these tests is as
    /// long as a real approval rather than a round number I picked.
    const WINDOW: i64 = VALID_UNTIL - APPROVED_AT;
    /// Somewhere inside that window, so `live` is a fact and not a coincidence.
    const NOW: i64 = APPROVED_AT + DAY;

    fn key() -> AgentKey {
        AgentKey {
            address: AGENT.to_owned(),
            account: ACCOUNT.to_owned(),
            approved_at: APPROVED_AT,
            expires_at: VALID_UNTIL,
        }
    }

    /// The same approval with a window that closed before `NOW`.
    fn lapsed() -> AgentKey {
        AgentKey {
            expires_at: APPROVED_AT + HOUR,
            ..key()
        }
    }

    fn entered() -> Vec<Event> {
        vec![
            Event::Prompt,
            Event::Unlocked {
                outcome: Unlock::Platform,
                account: ACCOUNT.to_owned(),
            },
        ]
    }

    fn run(events: impl IntoIterator<Item = Event>) -> Session {
        events.into_iter().fold(Session::default(), step)
    }

    #[test]
    fn the_window_is_read_off_the_venue_rather_than_guessed() {
        let key = key();
        assert_eq!(
            key.remaining_ms(APPROVED_AT),
            VALID_UNTIL - APPROVED_AT,
            "the whole window is what is left at the moment of approval"
        );
        assert!(key.live(VALID_UNTIL - 1));
        assert!(!key.live(VALID_UNTIL), "validUntil is the first dead ms");
        assert_eq!(key.remaining_ms(VALID_UNTIL), 0);
        assert_eq!(
            key.remaining_ms(VALID_UNTIL + DAY),
            0,
            "a lapsed key has no remaining time, it has no key"
        );
    }

    /// The countdown has to survive the same wound-back clock the state machine
    /// does. `expires_at - i64::MIN` is not representable, so the subtraction
    /// this used to do panicked in debug on the one input the file advertises
    /// it handles — and a panicking countdown takes the app down from a screen
    /// that was only trying to draw a timer.
    #[test]
    fn a_clock_wound_past_the_end_of_time_pegs_instead_of_panicking() {
        let key = key();
        assert_eq!(
            key.remaining_ms(i64::MIN),
            i64::MAX,
            "a clock before the epoch pegs the countdown, it does not overflow"
        );
        assert_eq!(key.remaining_ms(i64::MAX), 0);
        // `live` only knows the far end of the window, so a clock wound back
        // reads as inside it. What stops a wound-back clock reviving a dead
        // key is `step` keeping expiry one-way, tested below — not this.
        assert!(key.live(i64::MIN));
        assert!(!key.live(i64::MAX));
    }

    #[test]
    fn the_happy_path_walks_locked_to_ready() {
        assert_eq!(run([]), Session::Locked);
        assert_eq!(run([Event::Prompt]), Session::Unlocking);
        assert_eq!(
            run(entered()),
            Session::Unlocked {
                account: ACCOUNT.to_owned()
            }
        );

        let ready = run(entered().into_iter().chain([Event::Approved {
            key: key(),
            now: NOW,
        }]));
        assert_eq!(ready, Session::Ready { key: key() });
        assert!(can_trade(&ready, NOW));
        assert_eq!(account(&ready), Some(ACCOUNT));
    }

    #[test]
    fn an_unlocked_session_without_an_agent_cannot_trade() {
        let unlocked = run(entered());
        assert!(!can_trade(&unlocked, NOW), "reading is not signing");
        assert_eq!(agent(&unlocked), None);
        // A clock tick is not an approval.
        assert!(!can_trade(&step(unlocked, Event::Tick(NOW)), NOW));
    }

    #[test]
    fn an_expired_key_is_not_ready() {
        let after = run(entered().into_iter().chain([Event::Approved {
            key: lapsed(),
            now: NOW,
        }]));
        assert_eq!(after, Session::Expired { key: lapsed() });
        assert!(
            !can_trade(&after, NOW),
            "an approval that arrived already lapsed is not permission"
        );
    }

    #[test]
    fn a_live_key_expires_the_moment_the_clock_reaches_its_window() {
        let ready = run(entered().into_iter().chain([Event::Approved {
            key: key(),
            now: NOW,
        }]));
        assert!(can_trade(
            &step(ready.clone(), Event::Tick(VALID_UNTIL - 1)),
            VALID_UNTIL - 1
        ));
        let after = step(ready, Event::Tick(VALID_UNTIL));
        assert_eq!(after, Session::Expired { key: key() });
        assert!(!can_trade(&after, VALID_UNTIL));
    }

    /// The one the whole file exists for. Ticks are a courtesy: they arrive
    /// while a machine is awake and stop while it sleeps, and the exchange
    /// closes the window on its own schedule either way. So a `Ready` that was
    /// never told the time must still refuse the moment its window shuts —
    /// asking is what dates the answer, not the last event that landed.
    #[test]
    fn a_window_closes_on_a_ready_session_that_was_never_ticked() {
        let ready = run(entered().into_iter().chain([Event::Approved {
            key: key(),
            now: NOW,
        }]));
        assert!(can_trade(&ready, NOW));
        assert!(
            can_trade(&ready, VALID_UNTIL - 1),
            "the last millisecond the venue honours is still tradeable"
        );
        assert!(
            !can_trade(&ready, VALID_UNTIL),
            "no tick arrived, and the window shut anyway"
        );
        assert!(!can_trade(&ready, VALID_UNTIL + DAY));
        assert!(!can_trade(&ready, i64::MAX));
        assert_eq!(
            ready,
            Session::Ready { key: key() },
            "asking the time must not be what changes the state"
        );
    }

    #[test]
    fn time_going_backwards_does_not_resurrect_an_expired_key() {
        let expired = step(Session::Ready { key: key() }, Event::Tick(VALID_UNTIL));
        assert_eq!(expired, Session::Expired { key: key() });
        for wound_back in [VALID_UNTIL - 1, NOW, APPROVED_AT, 0, i64::MIN] {
            let after = step(expired.clone(), Event::Tick(wound_back));
            assert_eq!(
                after,
                Session::Expired { key: key() },
                "a clock at {wound_back} handed a dead key back"
            );
            // Nor is there a moment you can ask about that gets a yes, which
            // matters because `live` alone would call a wound-back clock
            // "inside the window".
            for now in every_moment() {
                assert!(
                    !can_trade(&after, now),
                    "traded at {now} after {wound_back}"
                );
            }
        }
    }

    #[test]
    fn a_key_for_a_different_account_is_refused() {
        let unlocked = run(entered());
        let theirs = AgentKey {
            account: OTHER.to_owned(),
            ..key()
        };
        assert_eq!(
            step(
                unlocked.clone(),
                Event::Approved {
                    key: theirs.clone(),
                    now: NOW,
                },
            ),
            unlocked,
            "a live key belonging to somebody else must change nothing"
        );

        // And the same refusal from every other state that holds a key: a
        // lapsed one, where a re-approval normally arrives, and a live one,
        // where letting somebody else's key in would swap the account a
        // session is already signing for.
        for holding in [
            Session::Expired { key: lapsed() },
            Session::Ready { key: key() },
        ] {
            assert_eq!(
                step(
                    holding.clone(),
                    Event::Approved {
                        key: theirs.clone(),
                        now: NOW,
                    },
                ),
                holding,
                "{holding:?} took a key belonging to another account"
            );
        }
    }

    /// A key renewed before it lapsed used to fall down the catch-all and be
    /// dropped, so the session kept counting down to an expiry the venue had
    /// already moved. The venue lists only live keys and lets one account hold
    /// several, so an approval landing in `Ready` is its latest word: take it.
    #[test]
    fn a_renewal_arriving_before_expiry_extends_the_window() {
        let ready = run(entered().into_iter().chain([Event::Approved {
            key: key(),
            now: NOW,
        }]));
        let renewed = AgentKey {
            approved_at: NOW,
            expires_at: VALID_UNTIL + WINDOW,
            ..key()
        };
        let after = step(
            ready,
            Event::Approved {
                key: renewed.clone(),
                now: NOW,
            },
        );
        assert_eq!(after, Session::Ready { key: renewed });
        assert!(
            can_trade(&after, VALID_UNTIL),
            "the renewal is what makes the old window stop mattering"
        );
    }

    /// The other half of the same rule. The venue never listed a lapsed key in
    /// 1,899 real approvals, so this is a key that ran out between the listing
    /// and the event — and the cheap error there is refusing to sign, not
    /// keeping a window the venue has moved on from.
    #[test]
    fn a_renewal_that_already_lapsed_stops_the_session_rather_than_being_ignored() {
        let after = step(
            Session::Ready { key: key() },
            Event::Approved {
                key: lapsed(),
                now: NOW,
            },
        );
        assert_eq!(after, Session::Expired { key: lapsed() });
        assert!(!can_trade(&after, NOW));
    }

    /// `approveAgent` is a 422 at the venue for `""` and for `"0x"`, so a key
    /// with no address was never approved by anyone. Admitting one would make
    /// `Ready` mean "may sign, with what?".
    #[test]
    fn an_approval_naming_no_agent_address_is_refused() {
        let nameless = AgentKey {
            address: String::new(),
            ..key()
        };
        for holding in [
            run(entered()),
            Session::Expired { key: lapsed() },
            Session::Ready { key: key() },
        ] {
            assert_eq!(
                step(
                    holding.clone(),
                    Event::Approved {
                        key: nameless.clone(),
                        now: NOW,
                    },
                ),
                holding,
                "{holding:?} admitted a key with no address"
            );
        }
    }

    /// `Session` is a public enum with public fields, so the unlock guard that
    /// makes an empty account unreachable only speaks for states this module
    /// built. A caller can hand `step` one anyway, and two empty strings
    /// matching each other must not read as "this is my account".
    #[test]
    fn an_empty_account_never_matches_an_empty_account() {
        let unnamed = AgentKey {
            account: String::new(),
            ..key()
        };
        let nobody = Session::Unlocked {
            account: String::new(),
        };
        let after = step(
            nobody.clone(),
            Event::Approved {
                key: unnamed,
                now: NOW,
            },
        );
        assert_eq!(
            after, nobody,
            "an anonymous key opened an anonymous session"
        );
        assert!(!can_trade(&after, NOW));
    }

    #[test]
    fn a_lapsed_session_is_re_approved_for_its_own_account() {
        let renewed = AgentKey {
            approved_at: NOW,
            expires_at: NOW + WINDOW,
            ..key()
        };
        let after = step(
            Session::Expired { key: lapsed() },
            Event::Approved {
                key: renewed.clone(),
                now: NOW,
            },
        );
        assert_eq!(after, Session::Ready { key: renewed });
        assert!(can_trade(&after, NOW));
    }

    #[test]
    fn re_locking_discards_the_key() {
        let ready = Session::Ready { key: key() };
        for state in [
            Session::Unenrolled,
            Session::Unlocking,
            Session::Unlocked {
                account: ACCOUNT.to_owned(),
            },
            ready,
            Session::Expired { key: lapsed() },
            Session::Unavailable {
                reason: NO_KEYCHAIN.to_owned(),
            },
        ] {
            let after = step(state.clone(), Event::Lock);
            assert_eq!(after, Session::Locked, "{state:?} survived a lock");
            assert_eq!(agent(&after), None);
            assert_eq!(account(&after), None);
            assert!(!can_trade(&after, NOW));
        }
    }

    #[test]
    fn a_declined_prompt_leaves_the_app_locked() {
        assert_eq!(
            run(vec![
                Event::Prompt,
                Event::Unlocked {
                    outcome: Unlock::Locked,
                    account: String::new(),
                },
            ]),
            Session::Locked
        );
    }

    #[test]
    fn an_unlock_that_names_no_account_unlocks_nothing() {
        assert_eq!(
            run(vec![
                Event::Prompt,
                Event::Unlocked {
                    outcome: Unlock::Platform,
                    account: String::new(),
                },
            ]),
            Session::Locked,
            "a secret with no account attached cannot be traded for anyone"
        );
    }

    #[test]
    fn a_late_platform_answer_cannot_reopen_a_locked_app() {
        let after = run(vec![
            Event::Prompt,
            Event::Lock,
            Event::Unlocked {
                outcome: Unlock::Platform,
                account: ACCOUNT.to_owned(),
            },
        ]);
        assert_eq!(
            after,
            Session::Locked,
            "a prompt answered after the user left must not let them back in"
        );
    }

    #[test]
    fn a_platform_without_a_keychain_can_never_trade() {
        let unavailable = run(vec![
            Event::Prompt,
            Event::Unlocked {
                outcome: Unlock::Unavailable(NO_KEYCHAIN.to_owned()),
                account: String::new(),
            },
        ]);
        assert_eq!(
            unavailable,
            Session::Unavailable {
                reason: NO_KEYCHAIN.to_owned()
            }
        );
        // Nothing but a lock leaves. Not a perfectly good approval — there is
        // nowhere to keep the secret that would make it mean anything — and
        // not a second prompt: which keystore a build has was settled at
        // compile time, so asking again fetches the same refusal. This used to
        // walk back to `Unlocking` and make "permanently" in the doc a lie.
        for event in every_event() {
            let after = step(unavailable.clone(), event.clone());
            let expected = if matches!(event, Event::Lock) {
                Session::Locked
            } else {
                unavailable.clone()
            };
            assert_eq!(after, expected, "{event:?} moved an unavailable build");
            assert!(!can_trade(&after, NOW));
        }
    }

    /// On a platform with no keychain the answer has to be "unavailable" rather
    /// than a stub that quietly succeeds, because a keystore that lies is worse
    /// than one that is missing. The reason is compared against the constant
    /// the impl uses, not against a copy of its text: a copy would keep passing
    /// after somebody reworded the refusal.
    ///
    /// macOS is excluded because macOS now has a real Keychain, and what it
    /// does can only be found out on one — see the list on the impl.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn this_build_has_no_keychain_and_says_so() {
        let keystore = PlatformKeystore;
        let Unlock::Unavailable(reason) = keystore.prompt(ACCOUNT, "Unlock trading") else {
            panic!("a build with no keychain claimed one");
        };
        assert_eq!(reason, UNAVAILABLE, "unavailable for an unexpected reason");
        // All three operations must refuse for the same stated reason. Reading
        // the constant rather than a copy of its text is what lets this notice
        // one of them drifting onto its own wording — or a real Keychain being
        // wired under only some of them, which is when this should stop.
        assert_eq!(
            keystore
                .store(ACCOUNT, &Secret::new(vec![1, 2, 3]), Guard::UserPresence)
                .expect_err("a build with no keychain stored a secret")
                .message,
            UNAVAILABLE
        );
        // Not `Ok(None)`: "nothing stored yet" invites the app to store one,
        // and there is nowhere here to put it.
        assert_eq!(
            keystore
                .load(ACCOUNT)
                .expect_err("a build with no keychain loaded a secret")
                .message,
            UNAVAILABLE
        );

        // And the answer drives the session to the one state that cannot trade.
        let after = run(vec![
            Event::Prompt,
            Event::Unlocked {
                outcome: keystore.prompt(ACCOUNT, "Unlock trading"),
                account: String::new(),
            },
        ]);
        assert_eq!(after, Session::Unavailable { reason });
        assert!(!can_trade(&after, NOW));
    }

    #[test]
    fn the_keystore_round_trips_a_secret_under_its_account() {
        let keystore = Memory::new(Unlock::Platform);
        let bytes = b"agent-key-seed".to_vec();
        assert!(
            matches!(
                keystore
                    .load(ACCOUNT)
                    .expect("an empty keychain is not a broken one"),
                Held::Missing
            ),
            "a first run must read as nothing stored yet, not as a failure"
        );
        keystore
            .store(ACCOUNT, &Secret::new(bytes.clone()), Guard::UserPresence)
            .expect("the in-memory keychain stores");
        let Held::Secret(stored) = keystore.load(ACCOUNT).expect("stored secrets load") else {
            panic!("a stored secret is there");
        };
        assert_eq!(stored.expose(), bytes);
        assert!(
            matches!(
                keystore
                    .load(OTHER)
                    .expect("another account missing is not a failure either"),
                Held::Missing
            ),
            "one account's secret must not answer for another's"
        );
    }

    /// The distinction the platform half of this file exists to make, checked
    /// at *both* consumers. Every status here is a keychain saying no; only one
    /// flavour of no is the app's problem, and getting that backwards is either
    /// an app that nags about a fault the user cannot fix or one that shrugs
    /// off a real one.
    ///
    /// `load` is in here because reading the item is what raises the sheet, so
    /// a decline arrives there exactly as it arrives at `prompt`. That arm used
    /// to be written inline inside the `cfg(target_os = "macos")` block, where
    /// no test on this box could see it — and it read a declined sheet as a
    /// broken keychain, the one mistake the whole seam exists to prevent.
    /// Inverting any single arm of `refusal`, `unlock`, or `held` now fails
    /// here.
    ///
    /// The codes are `OSStatus` values, so only macOS can produce them — but
    /// sorting them is arithmetic, which is why all three functions live
    /// outside the `cfg` and can be checked here.
    #[test]
    fn a_decline_and_a_first_run_are_not_a_broken_keychain() {
        for status in [USER_CANCELED, AUTH_FAILED, INTERACTION_NOT_ALLOWED] {
            assert_eq!(refusal(status), Refusal::Declined, "status {status}");
            assert_eq!(
                unlock(refusal(status), NO_KEYCHAIN.to_owned()),
                Unlock::Locked,
                "status {status} turned a user saying no into a broken keychain"
            );
            assert!(
                matches!(
                    held(refusal(status), NO_KEYCHAIN.to_owned()),
                    Ok(Held::Declined)
                ),
                "status {status} turned a declined read into a broken keychain"
            );
        }

        assert_eq!(refusal(ITEM_NOT_FOUND), Refusal::Missing);
        assert_eq!(
            unlock(refusal(ITEM_NOT_FOUND), NO_KEYCHAIN.to_owned()),
            Unlock::Unenrolled,
            "a first run is an install to finish, not a sheet to try again"
        );
        assert!(
            matches!(
                held(refusal(ITEM_NOT_FOUND), NO_KEYCHAIN.to_owned()),
                Ok(Held::Missing)
            ),
            "an empty keychain read as something other than empty"
        );

        // errSecNotAvailable, errSecMissingEntitlement, errSecIO, and a code
        // from nowhere: none of them are anything a user can answer at a sheet.
        for status in [-25291, -34018, -36, 1] {
            assert_eq!(refusal(status), Refusal::Failed, "status {status}");
            assert_eq!(
                unlock(refusal(status), NO_KEYCHAIN.to_owned()),
                Unlock::Unavailable(NO_KEYCHAIN.to_owned()),
                "status {status} was written off as the user's doing"
            );
            assert_eq!(
                held(refusal(status), NO_KEYCHAIN.to_owned())
                    .expect_err("a broken keychain answered a read")
                    .message,
                NO_KEYCHAIN,
                "status {status} was written off as the user's doing on a read"
            );
        }
    }

    /// The three refusals stay three all the way through. Each one lands the
    /// session somewhere with a different next move: enrol, ask again, or give
    /// up — and no two of them are the same state, which is the property that
    /// broke when `Missing` and `Declined` shared an answer.
    #[test]
    fn a_first_run_a_decline_and_a_fault_are_three_different_sessions() {
        let after = |status| {
            run(vec![
                Event::Prompt,
                Event::Unlocked {
                    outcome: unlock(refusal(status), NO_KEYCHAIN.to_owned()),
                    account: String::new(),
                },
            ])
        };

        let fresh = after(ITEM_NOT_FOUND);
        let declined = after(USER_CANCELED);
        let faulted = after(-36);
        assert_eq!(fresh, Session::Unenrolled);
        assert_eq!(declined, Session::Locked);
        assert_eq!(
            faulted,
            Session::Unavailable {
                reason: NO_KEYCHAIN.to_owned()
            }
        );
        assert_ne!(
            fresh, declined,
            "a first install and a user saying no are not the same fact"
        );

        for state in [&fresh, &declined, &faulted] {
            assert!(!can_trade(state, NOW));
            assert_eq!(account(state), None);
        }

        // And the next move differs. Enrolling happens outside this model — a
        // `store` — so the prompt after it has to work, exactly as it does for
        // a decline; a fault stays put however often it is asked.
        assert_eq!(step(fresh, Event::Prompt), Session::Unlocking);
        assert_eq!(step(declined, Event::Prompt), Session::Unlocking);
        assert_eq!(step(faulted.clone(), Event::Prompt), faulted);
    }

    /// And why the distinction is worth the enum: the two answers land the
    /// session in states with opposite futures. A decline can be asked again;
    /// a fault is the end of the road until something outside the app changes.
    #[test]
    fn a_declined_session_can_be_asked_again_and_a_faulted_one_cannot() {
        let declined = run(vec![
            Event::Prompt,
            Event::Unlocked {
                outcome: unlock(refusal(USER_CANCELED), NO_KEYCHAIN.to_owned()),
                account: String::new(),
            },
        ]);
        assert_eq!(declined, Session::Locked);
        assert_eq!(
            step(declined, Event::Prompt),
            Session::Unlocking,
            "declining once must not lock the user out of asking twice"
        );

        let faulted = run(vec![
            Event::Prompt,
            Event::Unlocked {
                outcome: unlock(refusal(-36), NO_KEYCHAIN.to_owned()),
                account: String::new(),
            },
        ]);
        assert_eq!(
            faulted,
            Session::Unavailable {
                reason: NO_KEYCHAIN.to_owned()
            }
        );
        assert_eq!(
            step(faulted.clone(), Event::Prompt),
            faulted,
            "re-prompting a broken keychain only burns the user's patience"
        );
        assert!(!can_trade(&faulted, NOW));
    }

    #[test]
    fn a_secret_never_prints_itself() {
        let secret = Secret::new(b"correct horse battery staple".to_vec());
        let printed = format!("{secret:?}");
        assert_eq!(printed, "Secret(28 bytes)");
        assert!(
            !printed.contains("horse"),
            "a secret reached a format string: {printed}"
        );
    }

    fn every_state() -> Vec<Session> {
        vec![
            Session::Locked,
            Session::Unenrolled,
            Session::Unlocking,
            Session::Unlocked {
                account: ACCOUNT.to_owned(),
            },
            Session::Ready { key: key() },
            Session::Expired { key: lapsed() },
            Session::Unavailable {
                reason: NO_KEYCHAIN.to_owned(),
            },
        ]
    }

    fn every_event() -> Vec<Event> {
        vec![
            Event::Prompt,
            Event::Unlocked {
                outcome: Unlock::Platform,
                account: ACCOUNT.to_owned(),
            },
            Event::Unlocked {
                outcome: Unlock::Platform,
                account: OTHER.to_owned(),
            },
            Event::Unlocked {
                outcome: Unlock::Platform,
                account: String::new(),
            },
            Event::Unlocked {
                outcome: Unlock::Locked,
                account: String::new(),
            },
            Event::Unlocked {
                outcome: Unlock::Unenrolled,
                account: String::new(),
            },
            Event::Unlocked {
                outcome: Unlock::Unavailable(NO_KEYCHAIN.to_owned()),
                account: String::new(),
            },
            Event::Approved {
                key: key(),
                now: NOW,
            },
            Event::Approved {
                key: lapsed(),
                now: NOW,
            },
            Event::Approved {
                key: AgentKey {
                    account: OTHER.to_owned(),
                    ..key()
                },
                now: NOW,
            },
            Event::Approved {
                key: AgentKey {
                    address: String::new(),
                    ..key()
                },
                now: NOW,
            },
            // A renewal: same account, a window past the one it holds.
            Event::Approved {
                key: AgentKey {
                    approved_at: NOW,
                    expires_at: VALID_UNTIL + WINDOW,
                    ..key()
                },
                now: NOW,
            },
            // An approval whose own clock is already past the window it
            // carries, which is what a report that raced the expiry looks like.
            Event::Approved {
                key: key(),
                now: VALID_UNTIL,
            },
            Event::Tick(NOW),
            Event::Tick(VALID_UNTIL),
            Event::Tick(0),
            Event::Tick(i64::MIN),
            Event::Tick(i64::MAX),
            Event::Lock,
        ]
    }

    /// Every moment worth asking `can_trade` at: both ends of the number line,
    /// both sides of the window the venue granted, and the ordinary middle.
    fn every_moment() -> [i64; 8] {
        [
            i64::MIN,
            0,
            APPROVED_AT,
            NOW,
            VALID_UNTIL - 1,
            VALID_UNTIL,
            VALID_UNTIL + WINDOW,
            i64::MAX,
        ]
    }

    /// Every state against every event, checked against the one rule the rest
    /// of the app trusts: a session that may sign holds a key that is live now
    /// and belongs to the account that was already unlocked. No transition gets
    /// to invent either half of that.
    #[test]
    fn no_transition_can_sign_with_a_dead_key_or_for_an_unopened_account() {
        for before in every_state() {
            for event in every_event() {
                let after = step(before.clone(), event.clone());
                let Session::Ready { key } = &after else {
                    continue;
                };
                assert!(
                    key.live(NOW),
                    "{before:?} + {event:?} would sign with a dead key"
                );
                assert_eq!(
                    account(&before),
                    Some(key.account.as_str()),
                    "{before:?} + {event:?} would sign for an account it never unlocked"
                );
            }
        }
    }

    /// The safety property this whole file exists for, over runs rather than
    /// single steps: no sequence of events reaches a `true` answer while the
    /// key the session is holding is dead at the moment of asking. Three deep
    /// over the whole alphabet is enough — `step` has no memory beyond the
    /// state it is handed, so a longer run only revisits these.
    #[test]
    fn no_run_of_events_can_trade_on_an_expired_key() {
        let events = every_event();
        let mut yes = 0;
        for first in &events {
            for second in &events {
                for third in &events {
                    let after = run([first.clone(), second.clone(), third.clone()]);
                    for now in every_moment() {
                        if !can_trade(&after, now) {
                            continue;
                        }
                        yes += 1;
                        let key = agent(&after).expect("a session that may sign holds a key");
                        assert!(
                            key.live(now),
                            "{first:?} → {second:?} → {third:?} would sign at {now} \
                             with a key the venue stopped honouring at {}",
                            key.expires_at
                        );
                    }
                }
            }
        }
        assert!(
            yes > 0,
            "the sweep never reached a tradeable session, so it proved nothing"
        );
    }

    /// The other half of the same sweep: nothing but an approval may ever add
    /// a key, so no clock tick or unlock can conjure one into a session that
    /// did not already hold it.
    #[test]
    fn only_an_approval_ever_introduces_a_key() {
        for before in every_state() {
            for event in every_event() {
                if matches!(event, Event::Approved { .. }) {
                    continue;
                }
                let after = step(before.clone(), event.clone());
                let Some(key) = agent(&after) else { continue };
                assert_eq!(
                    agent(&before),
                    Some(key),
                    "{before:?} + {event:?} produced a key out of nowhere"
                );
            }
        }
    }

    /// A stand-in for the Secure Enclave, and emphatically not a cipher.
    ///
    /// It exists to answer one question the Enclave cannot be asked on this
    /// machine: does the envelope *plumbing* seal what it stores and open what
    /// it reads, or does a bare secret reach the keychain? A reversible byte
    /// flip is enough to decide that and is obviously not encryption — the real
    /// wrapper is ECIES over a P-256 key that never leaves the chip, and its
    /// strength is the platform's rather than this file's.
    ///
    /// It also records what it was asked to open, so a test can tell "opened
    /// the ciphertext" from "never called".
    struct Flip {
        opened: RefCell<Vec<Vec<u8>>>,
    }

    impl Flip {
        fn new() -> Self {
            Self {
                opened: RefCell::new(Vec::new()),
            }
        }
    }

    impl Wrap for Flip {
        fn seal(&self, _account: &str, plain: &Secret) -> Result<Vec<u8>, KeystoreError> {
            Ok(plain.expose().iter().map(|byte| byte ^ 0xa5).collect())
        }

        fn open(&self, _account: &str, sealed: &[u8]) -> Result<Held, KeystoreError> {
            self.opened.borrow_mut().push(sealed.to_vec());
            Ok(Held::Secret(Secret::new(
                sealed.iter().map(|byte| byte ^ 0xa5).collect::<Vec<u8>>(),
            )))
        }
    }

    /// What reaches the keychain is not the secret, and what comes back is.
    ///
    /// The first half is the claim the owner asked for and the one a keychain
    /// dump would test: the bytes filed under the account contain the secret
    /// nowhere, in whole or in part. The second half is what keeps the first
    /// from being satisfied by simply losing the key.
    #[test]
    fn a_stored_secret_is_ciphertext_and_still_comes_back() {
        let store = Memory::new(Unlock::Platform);
        let wrap = Flip::new();
        let secret = Secret::new(b"the account's own thirty-two byte".to_vec());

        store_sealed(&wrap, &store, ACCOUNT, &secret).expect("sealed and filed");

        let filed = store.held.borrow().get(ACCOUNT).cloned().expect("an item");
        assert_ne!(filed, secret.expose(), "the plaintext reached the keychain");
        assert!(
            !filed
                .windows(secret.expose().len())
                .any(|window| window == secret.expose()),
            "the plaintext is in the item somewhere",
        );
        // And it is marked as this scheme's, which is what tells a later read
        // it is looking at ciphertext rather than at a #523 item.
        assert!(filed.starts_with(SEALED), "no marker: {filed:?}");

        let Held::Secret(back) = load_sealed(&wrap, &store, ACCOUNT).expect("read back") else {
            panic!("a sealed item reads back as a secret");
        };
        assert_eq!(back.expose(), secret.expose());
        // Opened rather than passed through: the marker was stripped and the
        // wrapper saw only the sealed half.
        assert_eq!(wrap.opened.borrow().len(), 1);
        assert_eq!(wrap.opened.borrow()[0], filed[SEALED.len()..]);
    }

    /// An item from before the envelope is brought forward on the read that
    /// finds it, and the bare form does not survive that read.
    ///
    /// The plant is deliberate: this is exactly the shape #523 wrote — the
    /// secret, as itself, under the account — and the only way to be sure the
    /// migration works is to write one and then read it.
    #[test]
    fn a_secret_from_before_the_envelope_is_resealed_on_the_way_out() {
        let store = Memory::new(Unlock::Platform);
        let wrap = Flip::new();
        let secret = Secret::new(b"a #523 key, stored as itself....".to_vec());

        // The old storage form, planted by hand.
        store
            .store(ACCOUNT, &secret, Guard::UserPresence)
            .expect("the legacy item");
        assert_eq!(store.held.borrow()[ACCOUNT], secret.expose());

        let Held::Secret(back) = load_sealed(&wrap, &store, ACCOUNT).expect("read back") else {
            panic!("a legacy item still answers with its secret");
        };
        assert_eq!(
            back.expose(),
            secret.expose(),
            "migrating must not change what the account is",
        );

        // And the bare form is gone: what sits there now is sealed, and a
        // second read goes through the wrapper rather than the migration.
        let filed = store.held.borrow().get(ACCOUNT).cloned().expect("an item");
        assert!(filed.starts_with(SEALED), "not re-sealed: {filed:?}");
        assert_ne!(filed, secret.expose());
        assert!(
            wrap.opened.borrow().is_empty(),
            "the migration did not unwrap"
        );

        let Held::Secret(again) = load_sealed(&wrap, &store, ACCOUNT).expect("read back twice")
        else {
            panic!("still a secret");
        };
        assert_eq!(again.expose(), secret.expose());
        assert_eq!(
            wrap.opened.borrow().len(),
            1,
            "the second read is an ordinary sealed read",
        );
    }

    /// A first run and a decline are answers, and they travel through the
    /// envelope unchanged rather than being mistaken for ciphertext.
    #[test]
    fn nothing_stored_stays_nothing_stored_through_the_envelope() {
        let store = Memory::new(Unlock::Platform);
        let wrap = Flip::new();
        assert!(matches!(
            load_sealed(&wrap, &store, ACCOUNT).expect("an answer"),
            Held::Missing
        ));
        assert!(wrap.opened.borrow().is_empty());
    }

    /// A sealed blob asks for no guard of its own; a bare secret still does.
    ///
    /// Both halves, because the change is a *removal* and a removal that went
    /// one step too far would look exactly like this one working. What the
    /// macOS `add` does with the answer — build an access control, or not — is
    /// on the list of things only a Mac can show; what is decided here is which
    /// answer each kind of item gets, and that is decidable anywhere.
    #[test]
    fn only_a_bare_secret_is_guarded_by_its_own_item() {
        let store = Memory::new(Unlock::Platform);
        let wrap = Flip::new();
        let secret = Secret::new(b"the account's own thirty-two byte".to_vec());

        // The wallet: ciphertext, and the Enclave key that opens it carries the
        // biometry. A second sheet here would stand over bytes an attacker
        // cannot use.
        store_sealed(&wrap, &store, ACCOUNT, &secret).expect("sealed and filed");
        assert_eq!(store.guards.borrow()[ACCOUNT], Guard::Sealed);

        // A trading key, through the same keychain: the item *is* the secret,
        // so the item is what has to be guarded. This half pins the seam's
        // shape — two distinct answers, carried to the keystore rather than
        // inferred there — and not the call site: `enrol_one` naming
        // `Guard::UserPresence` is a constant in `custody.rs` that no Linux
        // test can reach, because that path holds `PlatformKeystore` directly.
        // What the macOS `add` then *does* with either answer is on the list
        // only a Mac can settle.
        const AGENT_ITEM: &str = "hyperliquid-mainnet:0x1025d5c2";
        store
            .store(AGENT_ITEM, &secret, Guard::UserPresence)
            .expect("filed");
        assert_eq!(store.guards.borrow()[AGENT_ITEM], Guard::UserPresence);
        assert_ne!(
            store.guards.borrow()[ACCOUNT],
            store.guards.borrow()[AGENT_ITEM],
            "the two kinds of item are not guarded the same way",
        );
    }

    /// The system's own words for `-34018` are true and useless, so this one
    /// status gets a sentence about the binary instead.
    ///
    /// **This is the first thing a real Mac taught this file**, 2026-08-10: the
    /// owner pressed THIS IS MINE and read `OSStatus error -34018` followed by
    /// a `SecKeyRef` dump — curve name, x and y coordinates — over a problem
    /// whose entire fix is a build step. The dump is what the assertions rule
    /// out; the command is what makes the sentence actionable rather than
    /// merely honest.
    ///
    /// Every other status keeps the platform's wording, and the loop is the
    /// half that keeps this fix from becoming a habit of rewriting the
    /// platform: for everything else, the platform's words are the true ones.
    #[test]
    fn the_one_status_a_reader_can_fix_says_what_to_do_instead_of_naming_a_key() {
        // The system's own wording for `-34018`, from `SecBase.h`.
        const APPLES_WORDS: &str = "A required entitlement isn't present.";
        let said = described(
            MISSING_ENTITLEMENT,
            "making the wrapping key in the Secure Enclave",
            APPLES_WORDS,
        );
        assert!(
            said.contains("not code-signed"),
            "the reason the Enclave refused is not in the sentence: {said}",
        );
        assert!(
            said.contains("scripts/sign-dev.sh -p trading-example"),
            "the one command that fixes it is not in the sentence: {said}",
        );
        assert!(
            said.contains("Nothing has been stored"),
            "a refusal that does not say what became of the secret: {said}",
        );
        assert!(
            !said.contains("-34018") && !said.contains(APPLES_WORDS),
            "the raw status is back, which is the surfacing this replaced: {said}",
        );

        for status in [
            ITEM_NOT_FOUND,
            USER_CANCELED,
            AUTH_FAILED,
            INTERACTION_NOT_ALLOWED,
            -25291,
            -36,
        ] {
            assert_eq!(
                described(status, "reading the secret", "the system's own words"),
                format!("reading the secret: the system's own words ({status})"),
                "status {status} lost the platform's wording, which was the true one",
            );
        }
    }

    /// A build that cannot seal files nothing at all.
    ///
    /// **The decision, and it is the whole point of the envelope surviving a
    /// deployment gap**: an unsigned build has no Enclave key, and the answer
    /// to that is a refusal naming the fix — never a wallet stored in the clear
    /// because the machine could not manage the ciphertext. `store_sealed`
    /// seals before it files, so the refusal is structural rather than a rule
    /// somebody remembers; this pins it, because the shape that would undo it
    /// is a one-line "seal if you can" and it would look like a kindness.
    #[test]
    fn a_build_that_cannot_seal_files_nothing_and_names_the_fix() {
        /// The owner's Mac, 2026-08-10: `cargo run`, no signature, and an
        /// Enclave that will not make a key for it.
        struct Unsigned;

        impl Wrap for Unsigned {
            fn seal(&self, _account: &str, _plain: &Secret) -> Result<Vec<u8>, KeystoreError> {
                Err(KeystoreError::new(described(
                    MISSING_ENTITLEMENT,
                    "making the wrapping key in the Secure Enclave",
                    "A required entitlement isn't present.",
                )))
            }

            fn open(&self, _account: &str, _sealed: &[u8]) -> Result<Held, KeystoreError> {
                unreachable!("nothing was ever sealed, so nothing can be opened")
            }
        }

        let store = Memory::new(Unlock::Platform);
        let secret = Secret::new(b"the account's own thirty-two byte".to_vec());

        let refused = store_sealed(&Unsigned, &store, ACCOUNT, &secret)
            .expect_err("an unsealable build must not report a stored wallet");

        assert!(
            store.held.borrow().is_empty(),
            "the wallet reached the keychain unsealed: {:?}",
            store.held.borrow(),
        );
        assert!(
            refused
                .message
                .contains("scripts/sign-dev.sh -p trading-example"),
            "the refusal does not name the fix: {}",
            refused.message,
        );
    }
}
