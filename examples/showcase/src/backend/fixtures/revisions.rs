//! The counted-clone row behind `lazy_state_revisions.ice`: every deep clone
//! of an `Entry` is tallied, so a frame contract can say exactly when a
//! `lazy` over a state list materialized its value. The tally is per thread
//! — the driver renders on the test's own thread, and the generated `.ice`
//! tests of the same app run beside the contract and clone rows of their
//! own.

use std::cell::Cell;

thread_local! {
    static ENTRY_CLONES: Cell<usize> = const { Cell::new(0) };
}

/// Takes this thread's tally since the last call.
pub fn entry_clones() -> usize {
    ENTRY_CLONES.replace(0)
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub id: i64,
    pub title: String,
}

impl Clone for Entry {
    fn clone(&self) -> Self {
        ENTRY_CLONES.set(ENTRY_CLONES.get() + 1);
        Self {
            id: self.id,
            title: self.title.clone(),
        }
    }
}

pub fn seeded_entries() -> Vec<Entry> {
    (1..=3)
        .map(|id| Entry {
            id,
            title: format!("Entry {id}"),
        })
        .collect()
}

pub fn appended(mut entries: Vec<Entry>, title: String) -> Vec<Entry> {
    let id = entries.iter().map(|entry| entry.id).max().unwrap_or(0) + 1;
    entries.push(Entry { id, title });
    entries
}
