use std::alloc::System;
use std::ops::Range;

use iced::advanced::renderer::Headless;
use iced::advanced::{Widget, layout, text, widget};
use iced::widget::text_editor::Content;
use iced::{Font, Length, Pixels, Size, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::rich_text_editor::Format;
use ui_lang_runtime::{ContentVersion, RichTextEditor};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// loaded runner that lands inside the region as +4 allocations. Code under
/// test that allocated would dirty *every* window; a one-time foreign block
/// dirties at most one. So the batch runs in its own window, up to
/// [`WINDOWS`] times, and the contract asks for one clean window rather than a
/// clean process.
const WINDOWS: usize = 4;

/// Runs `batch` in a fresh allocator window, up to [`WINDOWS`] times, and
/// returns the first window reporting `expected` allocations — or the last
/// window's stats, when none did.
fn clean_window(expected: usize, mut batch: impl FnMut()) -> stats_alloc::Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if stats.allocations == expected {
            break;
        }
    }
    stats
}

struct StringSettings;

impl text::Highlighter for StringSettings {
    type Settings = String;
    type Highlight = ();
    type Iterator<'a> = std::iter::Empty<(Range<usize>, ())>;

    fn new(_settings: &Self::Settings) -> Self {
        Self
    }

    fn update(&mut self, _settings: &Self::Settings) {}

    fn change_line(&mut self, _line: usize) {}

    fn highlight_line(&mut self, _line: &str) -> Self::Iterator<'_> {
        std::iter::empty()
    }

    fn current_line(&self) -> usize {
        usize::MAX
    }
}

#[test]
fn changed_highlighter_settings_reuse_owned_storage() {
    const FRAMES: usize = 32;

    let content = Content::with_text("token-a token-b");
    let renderer = iced_test::futures::futures::executor::block_on(
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
    )
    .expect("headless renderer");
    let limits = layout::Limits::new(Size::ZERO, Size::new(320.0, 80.0));
    let version = ContentVersion::new(1, 0);
    let mut initial = editor(&content, version, String::from("token-a"));
    let mut tree = widget::Tree::new(&initial as &dyn Widget<(), Theme, iced::Renderer>);
    std::hint::black_box(initial.layout(&mut tree, &renderer, &limits));

    let mut warmup = editor(&content, version, String::from("token-b"));
    warmup.diff(&mut tree);
    std::hint::black_box(warmup.layout(&mut tree, &renderer, &limits));

    let stats = clean_window(FRAMES, || {
        for frame in 0..FRAMES {
            let settings = String::from(if frame % 2 == 0 { "token-a" } else { "token-b" });
            let mut editor = editor(&content, version, settings);
            editor.diff(&mut tree);
            std::hint::black_box(editor.layout(&mut tree, &renderer, &limits));
        }
    });

    eprintln!(
        "{FRAMES} settings updates: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, FRAMES, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}

fn editor(
    content: &Content,
    version: ContentVersion,
    settings: String,
) -> RichTextEditor<'_, StringSettings, ()> {
    RichTextEditor::new(content, version)
        .width(Length::Fixed(320.0))
        .height(Length::Fixed(80.0))
        .wrapping(text::Wrapping::None)
        .highlight_with::<StringSettings>(settings, 0, |_| Format::default())
}
