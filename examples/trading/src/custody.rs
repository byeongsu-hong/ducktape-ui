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
//! # Why the keychain item is keyed by deployment
//!
//! An agent key is approved on one deployment. The same address on mainnet and
//! on testnet holds two different accounts, approves two different keys, and a
//! secret read back under the wrong one is a key the venue has never heard of —
//! which surfaces as an order refused for a signer nobody recognises, long
//! after the mistake. So the keychain account is the chain and the address,
//! never the address alone.
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

use crate::Venue;
use crate::hyperliquid::hl_agents;
/// Ice names one namespace per Rust module, and custody is the one it talks
/// to — so the type it holds is published from here. `session.rs` stays where
/// the rules are and stays out of the app's vocabulary.
pub use crate::session::Session;
use crate::session::{
    AgentKey, Event, Held, Keystore, PlatformKeystore, Secret, Unlock, account, agent, can_trade,
    step,
};
use crate::signing::{Chain, Wallet};
use crate::venue::{Network, venue_name};

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

/// What the keychain files this account's secret under.
///
/// The deployment is in the key because a key approved on one is unknown on the
/// other; see the module header.
fn item(chain: Chain, address: &str) -> String {
    format!("{}:{}", chain.key(), address.trim().to_lowercase())
}

/// The deployment this network signs for, or the sentence saying it has none.
///
/// Lighter is the second case and it is not a shortcoming of this module:
/// `lighter_sign.rs` signs the token that venue's gated *reads* want and states
/// in its own header that nothing in it can place an order or move funds. There
/// is no write path to hold a key for, so offering to hold one would be the app
/// claiming a capability it does not have.
fn deployment(venue: Venue) -> Result<Chain, Entry> {
    Network::of(venue).chain.ok_or_else(|| {
        Entry::saying(
            Session::Locked,
            &format!(
                "This app has no way to sign for {}: its orders want an API key registered \
                 with the exchange, which an address alone cannot get. Reading it needs no \
                 key and is unaffected.",
                venue_name(venue),
            ),
        )
    })
}

/// Generate an agent key, hand its secret to the platform keychain, and report
/// the address the user now has to approve.
///
/// The key is generated here and never leaves: what goes to the keychain is the
/// secret, what goes to the screen is the address, and the account that owns it
/// approves the address elsewhere. Replacing an existing enrolment is the
/// keystore's problem and it solves it by preserving what it replaces — see
/// `session.rs`, which reads the old bytes back before it deletes them.
pub async fn enrol_agent(venue: Venue, address: String) -> Result<Entry, CustodyFault> {
    let chain = match deployment(venue) {
        Ok(chain) => chain,
        Err(refused) => return Ok(refused),
    };
    let address = address.trim().to_owned();
    if address.is_empty() {
        return Ok(Entry::saying(
            Session::Locked,
            "A key is approved for one account, so connect an address before making one.",
        ));
    }

    smol::unblock(move || {
        // The secret is made here rather than read back off a `Wallet`,
        // because the only two places it has any business being are the
        // keychain and the signer — and `signing.rs` deliberately publishes no
        // way to get it out again. What crosses back to the screen is the
        // address.
        let mut bytes = [0u8; 32];
        if getrandom::fill(&mut bytes).is_err() {
            return Ok(Entry::plain(Session::Unavailable {
                reason: "this machine would not produce randomness for a key".to_owned(),
            }));
        }
        let wallet = match Wallet::from_secret(&bytes) {
            Ok(wallet) => wallet,
            Err(failure) => {
                return Ok(Entry::plain(Session::Unavailable {
                    reason: failure.message,
                }));
            }
        };
        let secret = Secret::new(bytes.to_vec());
        match PlatformKeystore.store(&item(chain, &address), &secret) {
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
                &format!(
                    "Approve {} as an API wallet on {} from the wallet that owns this account, \
                     then unlock. This app cannot approve it: that signature is the account's \
                     own key, which is the one key it will never hold.",
                    wallet.address(),
                    venue_name(venue),
                ),
            )),
        }
    })
    .await
}

/// Raise the platform's prompt, and on the far side of it ask the venue which
/// of this account's keys are live.
///
/// The order is the model's, not a preference. `Prompt` puts the session in
/// `Unlocking`, which is the only state an answer is accepted in — a slow Touch
/// ID landing after the user locked the app must not re-open it — and only an
/// `Approved` carrying a window the venue reported can reach `Ready`. Nothing
/// here shortcuts to `Ready`, and `session.rs` would refuse it if it tried:
/// a key is admitted on its own window, so an approval that already lapsed
/// lands in `Expired` rather than in permission to trade.
pub async fn unlock_agent(venue: Venue, address: String) -> Result<Entry, CustodyFault> {
    let chain = match deployment(venue) {
        Ok(chain) => chain,
        Err(refused) => return Ok(refused),
    };
    let address = address.trim().to_owned();
    if address.is_empty() {
        return Ok(Entry::saying(
            Session::Locked,
            "A key is approved for one account, so connect an address before unlocking.",
        ));
    }

    let opened = {
        let address = address.clone();
        // Reading is what raises the sheet, and the sheet blocks — so it is off
        // the executor's thread rather than on it.
        smol::unblock(move || read_key(chain, &address)).await
    };
    let (held, wallet) = match opened {
        Opened::Refused(entry) => return Ok(entry),
        Opened::Held(session, wallet) => (session, wallet),
    };

    // Past here the app is `Unlocked`: it knows whose account this is and can
    // sign nothing. What turns that into `Ready` is the venue's own listing.
    let listed = hl_agents(chain, address.clone())
        .await
        .map_err(|failure| CustodyFault::new(failure.message))?;

    let ours = wallet.address().to_string();
    let Some(&(_, expires_at)) = listed
        .iter()
        .find(|(listed, _)| listed.eq_ignore_ascii_case(&ours))
    else {
        return Ok(Entry::saying(
            held,
            &format!(
                "{ours} is not an approved API wallet for this account on {}. Approve it from \
                 the wallet that owns the account, then unlock again. The account can be read \
                 either way.",
                venue_name(venue),
            ),
        ));
    };

    let now = now_ms();
    let key = AgentKey {
        address: ours,
        account: address,
        // The venue reports no approval time. What this app knows is when it
        // read one, and nothing is inferred from that.
        approved_at: now,
        expires_at,
    };
    // The `Wallet` is dropped here, and the session that comes back holds the
    // approval's public half and no key material — which is `session.rs`'s
    // rule, not an oversight: "the secret goes to the caller that asked for
    // it, never into the state machine".
    //
    // That leaves the next thing to be built with a decision to make rather
    // than a default to fall into. Signing an order needs this key again, and
    // there are only two honest answers: hold the live `Wallet` somewhere
    // outside the Ice state for as long as the session is `Ready` and drop it
    // on `Lock`, or read the keychain again per order and raise a sheet every
    // time. The first is what "unlocked" means to a reader and what every
    // other client does; the second is defensible only if a sheet per order is
    // wanted. Whichever it is, it is a choice with a reason attached and not
    // something to discover halfway through wiring the ticket.
    Ok(Entry::plain(step(held, Event::Approved { key, now })))
}

/// What reading the keychain produced: either a session holding this account
/// and the wallet behind it, or a finished answer to hand straight back.
enum Opened {
    Held(Session, Wallet),
    Refused(Entry),
}

fn read_key(chain: Chain, address: &str) -> Opened {
    let asked = step(Session::Locked, Event::Prompt);
    let answered = |outcome: Unlock| {
        step(
            asked.clone(),
            Event::Unlocked {
                outcome,
                account: address.to_owned(),
            },
        )
    };

    match PlatformKeystore.load(&item(chain, address)) {
        // Not a fault and not a decline: nothing has ever been stored for this
        // account here, and a second sheet would fetch the same answer forever.
        // Enrolling is what changes it.
        Ok(Held::Missing) => Opened::Refused(Entry::saying(
            answered(Unlock::Unenrolled),
            "No agent key on this Mac for this account and this network yet. Make one, approve \
             it from the wallet that owns the account, then unlock.",
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
        Ok(Held::Secret(secret)) => {
            let bytes: Result<[u8; 32], _> = secret.expose().try_into();
            // The item is there and is not a key. No sheet answers that, so it
            // reads as the keystore being unusable rather than as a refusal.
            let Ok(bytes) = bytes else {
                return Opened::Refused(Entry::plain(answered(Unlock::Unavailable(
                    "the stored secret is not an agent key; enrol again to replace it".to_owned(),
                ))));
            };
            match Wallet::from_secret(&bytes) {
                Err(failure) => {
                    Opened::Refused(Entry::plain(answered(Unlock::Unavailable(failure.message))))
                }
                Ok(wallet) => Opened::Held(answered(Unlock::Platform), wallet),
            }
        }
    }
}

/// Forget the key. The one transition with no conditions on it.
pub fn lock_agent() -> Session {
    step(Session::Locked, Event::Lock)
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
    step(session, Event::Tick(millis(now_s)))
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
pub fn session_refusal(venue: Venue, session: Session) -> String {
    if Network::of(venue).chain.is_none() {
        return format!(
            "There is no key to hold for {}: this app cannot sign its orders at all.",
            venue_name(venue),
        );
    }
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
    approved(now_s, 88 * 24 * 3_600)
}

/// The same approval, a day after the exchange stopped honouring it.
pub fn demo_session_expired(now_s: i64) -> Session {
    approved(now_s, -24 * 3_600)
}

fn approved(now_s: i64, left_s: i64) -> Session {
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

    const ACCOUNT: &str = "0x1025d5c2057058ffd8acf57109c5f649c11bdc11";
    const AGENT: &str = "0x13070cb3597c75100928720060c7acff4d22bc09";
    const HOUR: i64 = 3_600;

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
            session_refusal(Venue::Hyperliquid, declined).is_empty(),
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
            session_refusal(Venue::Hyperliquid, faulted.clone()),
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
    /// keychain problem.
    #[test]
    fn a_network_with_no_write_path_is_refused_before_any_sheet() {
        let refused = session_refusal(Venue::Lighter, Session::Locked);
        assert!(
            refused.contains("Lighter"),
            "the refusal names the network it is about: {refused}"
        );
        assert!(
            session_refusal(Venue::Hyperliquid, Session::Locked).is_empty(),
            "and says nothing where there is a write path"
        );

        // The seam refuses without touching the keychain at all, so the same
        // answer arrives on a machine that has one and a machine that does not.
        let entry = smol::block_on(unlock_agent(Venue::Lighter, ACCOUNT.to_owned()))
            .expect("a network without a write path is an answer, not a fault");
        assert_eq!(entry.session, Session::Locked);
        assert!(entry.note.contains("Lighter"));
    }

    /// The keychain item names the deployment, because the same address holds
    /// two different accounts on the two of them and a secret read back under
    /// the wrong one is a key the venue has never heard of.
    #[test]
    fn one_address_on_two_deployments_is_two_keychain_items() {
        assert_ne!(item(Chain::Mainnet, ACCOUNT), item(Chain::Testnet, ACCOUNT));
        assert!(item(Chain::Testnet, ACCOUNT).contains(ACCOUNT));
        // And the same address typed either way is the same item, so a capital
        // letter is not a second enrolment.
        assert_eq!(
            item(Chain::Mainnet, ACCOUNT),
            item(Chain::Mainnet, &ACCOUNT.to_uppercase().replace("0X", "0x")),
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
