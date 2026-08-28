//! The list operations the view calls; Ice expressions are closed, so a list
//! is rewritten by a pure Rust function rather than mutated in place.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Item {
    pub id: i64,
    pub text: String,
    pub done: bool,
}

/// Handlers have no conditionals, so an empty draft is refused here.
pub fn add_item(mut items: Vec<Item>, id: i64, text: String) -> Vec<Item> {
    if !text.trim().is_empty() {
        items.push(Item {
            id,
            text,
            done: false,
        });
    }
    items
}

pub fn toggle_item(mut items: Vec<Item>, id: i64) -> Vec<Item> {
    for item in &mut items {
        if item.id == id {
            item.done = !item.done;
        }
    }
    items
}

pub fn remove_item(mut items: Vec<Item>, id: i64) -> Vec<Item> {
    items.retain(|item| item.id != id);
    items
}

pub fn item_mark(done: bool) -> String {
    if done { "✓".into() } else { "○".into() }
}

pub fn remaining(items: Vec<Item>) -> i64 {
    items.iter().filter(|item| !item.done).count() as i64
}
