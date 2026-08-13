//! The chat-stream shape the Ice compiler emits for a keyed column whose row
//! template is a keyed lazy:
//! `keyed message in messages by=message.seq` + `lazy message by message.rev,
//! message.seq as row`. The keyed loop iterates by reference — the same
//! borrow-aware treatment `for` rows get — and the memo dependency tuple
//! carries only the cheap keys, so the ONLY deep clone of a row happens
//! inside the `memo_lazy` builder. This pins the runtime half of that
//! contract for the keyed-column form: no builder runs — and therefore no
//! row is cloned — while the dependency hash is unchanged, and a key change
//! rebuilds exactly the row it names.

use iced::advanced::renderer;
use iced::widget::{column, keyed_column, text};
use iced::{Element, Font, Pixels, Size, Theme};
use iced_test::runtime::{UserInterface, user_interface};
use std::sync::atomic::{AtomicUsize, Ordering};
use ui_lang_runtime::memo_lazy;

const ROWS: usize = 50;
const WINDOW: Size = Size::new(1280.0, 800.0);

static ROW_CLONES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Row {
    rev: i64,
    seq: i64,
    author: String,
    body: String,
}

impl Clone for Row {
    fn clone(&self) -> Self {
        ROW_CLONES.fetch_add(1, Ordering::Relaxed);
        Self {
            rev: self.rev,
            seq: self.seq,
            author: self.author.clone(),
            body: self.body.clone(),
        }
    }
}

fn rows() -> Vec<Row> {
    (0..ROWS as i64)
        .map(|seq| Row {
            rev: 0,
            seq,
            author: format!("user-{}", seq % 7),
            body: format!("message {seq}: the quick brown fox jumps over the lazy dog"),
        })
        .collect()
}

/// Mirrors the generated keyed column: rows reach the loop by reference (no
/// clone), each child is keyed by the row's own key expression, the memo
/// dependency tuple carries `(rev, seq, scope, palette)`, and the row is
/// cloned into the owned binding only inside the builder.
fn view(messages: &[Row]) -> Element<'_, (), Theme, iced_test::renderer::Renderer> {
    let mut children: Vec<(i64, Element<'_, (), Theme, iced_test::renderer::Renderer>)> =
        Vec::new();
    for message in messages.iter() {
        let key = message.seq;
        let child: Element<'_, (), Theme, iced_test::renderer::Renderer> = memo_lazy(
            (
                message.rev,
                message.seq,
                format!("KeyedRows/key({key})"),
                "light",
            ),
            move |__dependency: &(i64, i64, String, &'static str)| -> Element<
                'static,
                (),
                Theme,
                iced_test::renderer::Renderer,
            > {
                let _lazy_scope = __dependency.2.clone();
                let row: Row = message.clone();
                column![text(row.author).width(120.0), text(row.body).size(14)].into()
            },
            17u64,
            key,
        )
        .into();
        children.push((key, child));
    }
    keyed_column(children).into()
}

fn renderer() -> iced_test::renderer::Renderer {
    iced_test::futures::futures::executor::block_on(
        <iced_test::renderer::Renderer as renderer::Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            None,
        ),
    )
    .expect("headless renderer")
}

#[test]
fn unchanged_frames_clone_no_keyed_rows_and_a_key_change_clones_one() {
    let mut messages = rows();
    let mut renderer = renderer();

    // The first mount builds every row once: one clone per row, into the
    // cached subtree.
    let ui = UserInterface::build(
        view(&messages),
        WINDOW,
        user_interface::Cache::default(),
        &mut renderer,
    );
    let mut cache = ui.into_cache();
    assert_eq!(
        ROW_CLONES.swap(0, Ordering::Relaxed),
        ROWS,
        "the first mount clones each row exactly once"
    );

    // Unchanged frames: every dependency hash is unchanged, so no builder
    // runs and no row is cloned — the contract the cheap-key form exists for.
    for _ in 0..5 {
        let ui = UserInterface::build(view(&messages), WINDOW, cache, &mut renderer);
        cache = ui.into_cache();
    }
    assert_eq!(
        ROW_CLONES.swap(0, Ordering::Relaxed),
        0,
        "unchanged frames must deep-clone no keyed row"
    );

    // A key change rebuilds — and therefore clones — exactly the row it
    // names; every other row stays cached.
    messages[7].rev += 1;
    messages[7].body.push_str(" (edited)");
    let ui = UserInterface::build(view(&messages), WINDOW, cache, &mut renderer);
    let _cache = ui.into_cache();
    assert_eq!(
        ROW_CLONES.swap(0, Ordering::Relaxed),
        1,
        "a single key change must clone exactly the changed row"
    );
}
