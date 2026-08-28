//! The list operations the view calls; Ice expressions are closed, so a list
//! is rewritten by a pure Rust function rather than mutated in place. The
//! list itself lives in the host's storage as one line per item.

use app_store_sdk::host;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Item {
    pub id: i64,
    pub text: String,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StorageError {
    pub message: String,
}

impl From<String> for StorageError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

const KEY: &str = "items";

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

pub fn remaining(items: &[Item]) -> String {
    let left = items.iter().filter(|item| !item.done).count();
    format!("{left} left")
}

pub fn next_after(items: Vec<Item>) -> i64 {
    items.iter().map(|item| item.id).max().unwrap_or(0) + 1
}

/// What a fresh install shows before anything was ever saved.
fn seed() -> Vec<Item> {
    add_item(
        add_item(Vec::new(), 1, "Ship the recording renderer".into()),
        2,
        "Draw it from the host".into(),
    )
}

pub async fn load_items() -> Result<Vec<Item>, StorageError> {
    let bytes = host::request("storage.get", KEY.as_bytes()).await?;
    if bytes.is_empty() {
        return Ok(seed());
    }
    Ok(decode(&bytes))
}

/// Writes the list, then tells the bus. Two requests, in order.
pub async fn save_items(items: Vec<Item>) -> Result<String, StorageError> {
    let mut payload = format!("{KEY}\n").into_bytes();
    payload.extend(encode(&items));
    host::request("storage.set", &payload).await?;
    let left = items.iter().filter(|item| !item.done).count();
    let news = format!("todo\n{} items, {left} left", items.len());
    host::request("bus.publish", news.as_bytes()).await?;
    Ok(format!("saved {} items", items.len()))
}

/// `id\tdone\ttext` per line; tabs and newlines in a text are folded to spaces.
pub fn encode(items: &[Item]) -> Vec<u8> {
    items
        .iter()
        .map(|item| {
            let text = item.text.replace(['\t', '\n'], " ");
            format!("{}\t{}\t{text}\n", item.id, u8::from(item.done))
        })
        .collect::<String>()
        .into_bytes()
}

pub fn decode(bytes: &[u8]) -> Vec<Item> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let id = fields.next()?.parse().ok()?;
            let done = fields.next()? == "1";
            let text = fields.next()?.to_string();
            Some(Item { id, text, done })
        })
        .collect()
}
