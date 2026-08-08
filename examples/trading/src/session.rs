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
    fn drop(&mut self) {
        // ponytail: a plain overwrite the optimiser is free to elide; `zeroize`
        // is the upgrade if a dependency is ever worth it here.
        self.0.fill(0);
    }
}

/// What asking the platform to let us in produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unlock {
    /// The prompt was declined or dismissed. Nothing was released, and trying
    /// again is a reasonable thing for the user to do.
    Locked,
    /// Touch ID or the passkey answered and the keychain handed back what it
    /// held. Carries nothing: the secret goes to the caller that asked for it,
    /// never into the state machine.
    Platform,
    /// There is no platform keychain in this build. Retrying cannot fix that,
    /// which is exactly why it does not read as `Locked`.
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
        (Session::Locked, Event::Prompt) => Session::Unlocking,

        // An answer is only an answer to a prompt that is still up. A slow
        // Touch ID landing after the user locked the app must not re-open it.
        (Session::Unlocking, Event::Unlocked { outcome, account }) => match outcome {
            // An unlock that cannot say whose account it opened has opened
            // nothing there is anything to do with.
            Unlock::Platform if !account.is_empty() => Session::Unlocked { account },
            Unlock::Platform | Unlock::Locked => Session::Locked,
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

/// The three things a platform has to be able to do for the app to have an
/// unlock at all. Nothing else in this file touches the platform, which is why
/// everything else in this file is decidable on any machine.
///
/// `prompt` returns an `Unlock` rather than a `Result` on purpose: a declined
/// prompt and a missing keychain are both already variants of it, and two ways
/// to say no is one too many for a caller to get right.
pub trait Keystore {
    /// Put the platform's own prompt on screen — Touch ID, or the passkey
    /// sheet — and report what came of it. `reason` is the line the OS shows.
    fn prompt(&self, reason: &str) -> Unlock;
    fn store(&self, account: &str, secret: &Secret) -> Result<(), KeystoreError>;
    fn load(&self, account: &str) -> Result<Secret, KeystoreError>;
}

/// Whichever keystore this build actually has. The app names this one type;
/// which implementation answers is the platform's business.
pub struct PlatformKeystore;

/// macOS: the Keychain, guarded by the Secure Enclave, which is where this
/// belongs.
///
/// Not wired. Reaching the Keychain needs the `security-framework` crate, and
/// that crate is not in this workspace's lockfile — adding a dependency is not
/// a decision this file gets to make. Until it is added every operation says so
/// plainly: an unavailable keystore is a state the session model already
/// handles and tests, whereas a `todo!()` here would take the app down on its
/// first launch on the one platform this was written for.
#[cfg(target_os = "macos")]
const UNAVAILABLE: &str = "macOS Keychain not wired: needs the security-framework crate, which this workspace does not depend on";

/// Everywhere else: there is no keychain to ask, and pretending otherwise
/// would mean inventing somewhere to keep a secret. The session model has a
/// state for this, so the app can say why it cannot trade instead of failing
/// at the moment somebody clicks buy.
#[cfg(not(target_os = "macos"))]
const UNAVAILABLE: &str = "no platform keychain on this build";

/// One impl, because until a keychain is actually wired both platforms do the
/// same thing — refuse, and say which reason it is. The tests read
/// `UNAVAILABLE` rather than repeating its text, so changing the wording of a
/// refusal cannot leave a test asserting a string nothing produces any more.
impl Keystore for PlatformKeystore {
    fn prompt(&self, _reason: &str) -> Unlock {
        Unlock::Unavailable(UNAVAILABLE.to_owned())
    }

    fn store(&self, _account: &str, _secret: &Secret) -> Result<(), KeystoreError> {
        Err(KeystoreError::new(UNAVAILABLE.to_owned()))
    }

    fn load(&self, _account: &str) -> Result<Secret, KeystoreError> {
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
    }

    impl Memory {
        fn new(answer: Unlock) -> Self {
            Self {
                answer,
                held: RefCell::new(HashMap::new()),
            }
        }
    }

    impl Keystore for Memory {
        fn prompt(&self, _reason: &str) -> Unlock {
            self.answer.clone()
        }

        fn store(&self, account: &str, secret: &Secret) -> Result<(), KeystoreError> {
            self.held
                .borrow_mut()
                .insert(account.to_owned(), secret.expose().to_vec());
            Ok(())
        }

        fn load(&self, account: &str) -> Result<Secret, KeystoreError> {
            self.held
                .borrow()
                .get(account)
                .map(|bytes| Secret::new(bytes.clone()))
                .ok_or_else(|| KeystoreError::new(format!("nothing stored for {account}")))
        }
    }

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
            Session::Unlocking,
            Session::Unlocked {
                account: ACCOUNT.to_owned(),
            },
            ready,
            Session::Expired { key: lapsed() },
            Session::Unavailable {
                reason: UNAVAILABLE.to_owned(),
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
                outcome: Unlock::Unavailable(UNAVAILABLE.to_owned()),
                account: String::new(),
            },
        ]);
        assert_eq!(
            unavailable,
            Session::Unavailable {
                reason: UNAVAILABLE.to_owned()
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

    /// Whatever platform this suite runs on, the answer has to be
    /// "unavailable" rather than a stub that quietly succeeds, because a
    /// keystore that lies is worse than one that is missing. The reason is
    /// compared against the constant the impl uses, not against a copy of its
    /// text: a copy would keep passing after somebody reworded the refusal, or
    /// wired a real Keychain, which is exactly when this should stop.
    #[test]
    fn this_build_has_no_keychain_and_says_so() {
        let keystore = PlatformKeystore;
        let Unlock::Unavailable(reason) = keystore.prompt("Unlock trading") else {
            panic!("a build with no keychain claimed one");
        };
        assert_eq!(reason, UNAVAILABLE, "unavailable for an unexpected reason");
        // All three operations must refuse for the same stated reason. Reading
        // the constant rather than a copy of its text is what lets this notice
        // one of them drifting onto its own wording — or a real Keychain being
        // wired under only some of them, which is when this should stop.
        assert_eq!(
            keystore
                .store(ACCOUNT, &Secret::new(vec![1, 2, 3]))
                .expect_err("a build with no keychain stored a secret")
                .message,
            UNAVAILABLE
        );
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
                outcome: keystore.prompt("Unlock trading"),
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
        keystore
            .store(ACCOUNT, &Secret::new(bytes.clone()))
            .expect("the in-memory keychain stores");
        assert_eq!(
            keystore
                .load(ACCOUNT)
                .expect("stored secrets load")
                .expose(),
            bytes
        );
        assert!(
            keystore.load(OTHER).is_err(),
            "one account's secret must not answer for another's"
        );
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
            Session::Unlocking,
            Session::Unlocked {
                account: ACCOUNT.to_owned(),
            },
            Session::Ready { key: key() },
            Session::Expired { key: lapsed() },
            Session::Unavailable {
                reason: UNAVAILABLE.to_owned(),
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
                outcome: Unlock::Unavailable(UNAVAILABLE.to_owned()),
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
}
