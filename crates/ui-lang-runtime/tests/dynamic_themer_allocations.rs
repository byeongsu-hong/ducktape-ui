// iced implements `Renderer` for `()` only under `debug_assertions`; the
// same gate keeps `cargo build --release --tests` compiling this crate.
#![cfg(debug_assertions)]

use std::alloc::System;

use iced::advanced::{Layout, Widget, layout, renderer, widget::Tree};
use iced::theme;
use iced::{Color, Element, Rectangle, Size};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::{DynamicThemer, dynamic_themer};

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
/// returns the first window whose `(allocations, bytes_allocated)` equal
/// `expected` — or the last window's stats, when none did.
fn clean_window(expected: (usize, usize), mut batch: impl FnMut()) -> stats_alloc::Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if (stats.allocations, stats.bytes_allocated) == expected {
            break;
        }
    }
    stats
}

#[derive(Debug)]
struct AllocatingTheme(String);

impl theme::Base for AllocatingTheme {
    fn default(_preference: theme::Mode) -> Self {
        Self(String::from("unused fallback theme"))
    }

    fn mode(&self) -> theme::Mode {
        theme::Mode::Light
    }

    fn base(&self) -> theme::Style {
        theme::Style {
            background_color: Color::BLACK,
            text_color: Color::WHITE,
        }
    }

    fn palette(&self) -> Option<theme::Palette> {
        None
    }

    fn name(&self) -> &str {
        &self.0
    }
}

type Themer<'a> = DynamicThemer<'a, (), AllocatingTheme, ()>;

#[test]
fn explicit_theme_skips_fallback_allocations() {
    const FRAMES: usize = 256;

    let content: Element<'_, (), AllocatingTheme, ()> = iced::widget::space().into();
    let mut themer: Themer<'_> = dynamic_themer(
        Some(AllocatingTheme(String::from("selected theme"))),
        content,
        None,
        None,
    );
    let mut tree = Tree::new(&themer as &dyn Widget<(), AllocatingTheme, ()>);
    let limits = layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0));
    let node = <Themer<'_> as Widget<(), AllocatingTheme, ()>>::layout(
        &mut themer,
        &mut tree,
        &(),
        &limits,
    );
    let viewport = Rectangle::with_size(node.size());
    let outer_theme = AllocatingTheme(String::from("outer theme"));
    let mut renderer = ();

    let stats = clean_window((0, 0), || {
        for _ in 0..FRAMES {
            <Themer<'_> as Widget<(), AllocatingTheme, ()>>::draw(
                &themer,
                &tree,
                &mut renderer,
                &outer_theme,
                &renderer::Style::default(),
                Layout::new(&node),
                iced::mouse::Cursor::Unavailable,
                &viewport,
            );
        }
    });

    eprintln!(
        "{FRAMES} explicit-theme draws: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.bytes_allocated, 0);
}
