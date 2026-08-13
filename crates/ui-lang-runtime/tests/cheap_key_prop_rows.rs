//! The thread-rail shape the Ice compiler emits when a keyed lazy's row list
//! arrives as a component prop: the component body outlines to a `&self`
//! method and the prop bakes to the caller's state place
//! (`self.thread_messages`), so rows reach the loop by reference exactly as
//! in the state-rooted form, and the memo dependency tuple carries only the
//! cheap keys. This pins the runtime half of that contract for the
//! prop-rooted form: no builder runs — and therefore no row is cloned —
//! while the dependency hash is unchanged, and a key change rebuilds exactly
//! the row it names.

use iced::advanced::renderer;
use iced::widget::{column, text};
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

/// The app the generated component method renders from: the prop's argument
/// is a state field, so inside the outlined method the prop reads
/// `self.thread_messages`.
struct Screen {
    thread_messages: Vec<Row>,
}

impl Screen {
    fn seeded() -> Self {
        Self {
            thread_messages: (0..ROWS as i64)
                .map(|seq| Row {
                    rev: 0,
                    seq,
                    author: format!("user-{}", seq % 7),
                    body: format!("message {seq}: the quick brown fox jumps over the lazy dog"),
                })
                .collect(),
        }
    }

    /// Mirrors the outlined component method: a `&self` renderer whose loop
    /// iterates the baked prop place by reference, cloning a row only inside
    /// the `memo_lazy` builder.
    fn view(&self) -> Element<'_, (), Theme, iced_test::renderer::Renderer> {
        column(self.thread_messages.iter().map(|message| {
            memo_lazy(
                (message.rev, message.seq, ("Thread/root").to_owned(), "light"),
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
                9u64,
                message.seq,
            )
            .into()
        }))
        .into()
    }
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
fn unchanged_frames_clone_no_prop_rows_and_a_key_change_clones_one() {
    let mut screen = Screen::seeded();
    ROW_CLONES.store(0, Ordering::Relaxed);
    let mut renderer = renderer();

    // The first mount builds every row once: one clone per row, into the
    // cached subtree.
    let ui = UserInterface::build(
        screen.view(),
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
        let ui = UserInterface::build(screen.view(), WINDOW, cache, &mut renderer);
        cache = ui.into_cache();
    }
    assert_eq!(
        ROW_CLONES.swap(0, Ordering::Relaxed),
        0,
        "unchanged frames must deep-clone no prop row"
    );

    // A key change rebuilds — and therefore clones — exactly the row it
    // names; every other row stays cached.
    screen.thread_messages[7].rev += 1;
    screen.thread_messages[7].body.push_str(" (edited)");
    let ui = UserInterface::build(screen.view(), WINDOW, cache, &mut renderer);
    let _cache = ui.into_cache();
    assert_eq!(
        ROW_CLONES.swap(0, Ordering::Relaxed),
        1,
        "a single key change must clone exactly the changed row"
    );
}
