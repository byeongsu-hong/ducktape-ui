//! What the terminal answers on the keyboard.
//!
//! **No key sends.** That is the whole safety rule and it is structural rather
//! than careful: the keys below reach the confirmation and stop there, and the
//! subscription that carries them is off whenever a confirmation is standing —
//! so SEND IT and DO IT are pressed by hand, every time, by somebody who has
//! read the panel. There is no chord, no repeat and no modifier that spends
//! money.
//!
//! The second rule is that a focused field owns what is typed into it. That one
//! is structural too, and it is not enforced here: the subscription asks for
//! `status=ignored`, so a keystroke a widget consumed never arrives. Every key
//! in this scheme is one a focused text input consumes — `b`, `s` and the
//! digits are characters it inserts — so a reader typing "b" into the search
//! box types a b and nothing else happens. REVIEW is not in this file at all:
//! it is `submit=` on the ticket's own fields, which is the strongest form of
//! the same rule, because a widget's own submit cannot fire from a widget the
//! reader is not in.
//!
//! Each function answers the one key it owns and hands back what it was given
//! for every other key. That shape is not decoration: an Ice handler cannot
//! branch, so a scheme built as branches would have to be one handler and one
//! subscription per key. Built as mappings it is one of each, and one press
//! moves exactly one thing.

use iced::keyboard::{Key, key::Named};

use crate::Locale;
use crate::hyperliquid::{Book, amount, book_tick, fmt_px};
use crate::i18n::t;

/// One line of the scheme, for the panel that documents it.
///
/// A scheme nobody can read is a scheme that surprises somebody, and these are
/// the app's own strings rather than a second list kept beside the code: what
/// Settings prints and what the handler answers come from one place.
#[derive(Clone, PartialEq, Debug)]
pub struct Hotkey {
    pub keys: String,
    pub act: String,
}

fn hotkey(locale: Locale, keys: &str, act: &str) -> Hotkey {
    Hotkey {
        keys: keys.to_owned(),
        act: t(locale, act),
    }
}

/// The scheme, in the order a reader meets it: which side, how big, what price,
/// and then the one that opens the confirmation.
pub fn hotkey_list(locale: Locale) -> Vec<Hotkey> {
    vec![
        hotkey(locale, "B", "Buy / long"),
        hotkey(locale, "S", "Sell / short"),
        hotkey(locale, "1 2 3 4", "Size to 25%, 50%, 75%, all"),
        hotkey(locale, "↑ ↓", "Move the limit price one tick"),
        hotkey(locale, "Enter", "Review the order — in a ticket field"),
        hotkey(locale, "Esc", "Close an open picker, then the search"),
    ]
}

/// What the scheme will not do, said where the scheme is read.
pub fn hotkey_note(locale: Locale) -> String {
    t(
        locale,
        "No key sends an order. The keys above reach the confirmation and stop \
         there, and they are off entirely while one is open — SEND IT is pressed by \
         hand. A field you are typing in keeps its own keystrokes, so these do \
         nothing while the search box or a ticket field has the cursor.",
    )
}

fn character(pressed: &Key) -> Option<&str> {
    match pressed {
        Key::Character(typed) => Some(typed.as_str()),
        _ => None,
    }
}

fn named(pressed: &Key) -> Option<Named> {
    match pressed {
        Key::Named(name) => Some(*name),
        _ => None,
    }
}

/// `b` buys, `s` sells, and every other key leaves the side alone.
pub fn hotkey_side(pressed: Key, current: bool) -> bool {
    match character(&pressed) {
        Some("b") | Some("B") => true,
        Some("s") | Some("S") => false,
        _ => current,
    }
}

/// `1` `2` `3` `4` are the four buttons under the size field, in the order they
/// are drawn.
///
/// Zero for every other key, which is what `share_size` already reads as
/// "there is no size to fill in" — so the handler needs no branch of its own
/// and an unrelated key cannot empty the field.
pub fn hotkey_share(pressed: Key) -> f64 {
    match character(&pressed) {
        Some("1") => 0.25,
        Some("2") => 0.5,
        Some("3") => 0.75,
        Some("4") => 1.0,
        _ => 0.0,
    }
}

/// The arrows move the limit one tick, and the tick is the market's own.
///
/// An empty field is not nudged: an arrow is an adjustment, and adjusting
/// nothing into one tick puts a price in the box that the reader never chose
/// and that the panel below would immediately quote an order against.
pub fn hotkey_price(pressed: Key, typed: String, book: Option<Book>) -> String {
    let step = match named(&pressed) {
        Some(Named::ArrowUp) => 1.0,
        Some(Named::ArrowDown) => -1.0,
        _ => return typed,
    };
    let price = amount(&typed);
    if price <= 0.0 {
        return typed;
    }
    let tick = book_tick(book.as_ref(), price);
    fmt_px((price + step * tick).max(tick))
}
