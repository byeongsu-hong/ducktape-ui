use iced::advanced::renderer;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, Widget, layout, mouse};
use iced::{Element, Font, Length, Pixels, Rectangle, Size, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;
use ui_lang_runtime::virtual_keyed_children;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const ROWS: usize = 1_000;

/// How many allocator windows a batch may be measured in before the contract
/// gives up on finding one that reads clean.
///
/// `StatsAlloc` is this binary's global allocator, so its counters cover every
/// thread, and this binary holds a single test. While that test runs, libtest's
/// main thread is doing its one-time setup — `Receiver::recv()` allocates the
/// `mpmc` context and waker entry it needs, plus the harness's bookkeeping
/// around it — and those blocks land in whichever window happens to be open.
/// That is what failed `Rust and Ice` on main with `32 exact-key frames
/// allocated 4 times (900 bytes)`, where a rerun of the same commit passed: 4
/// is not a multiple of 32, so it was never one allocation per frame. A frame
/// that allocated would dirty *every* window; a one-time foreign block dirties
/// at most one. So each batch is measured in its own window and the contract
/// asks for one clean window rather than a clean process.
const WINDOWS: usize = 4;

/// Runs `batch` in its own allocator window, up to [`WINDOWS`] times, and
/// returns the first `(allocations, bytes)` that match `expected` — or the last
/// window's, when none did.
fn measure(expected: (usize, usize), mut batch: impl FnMut()) -> (usize, usize) {
    let mut measured = (0, 0);
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        let stats = region.change();
        measured = (stats.allocations, stats.bytes_allocated);
        if measured == expected {
            break;
        }
    }
    measured
}

struct FixedRow;

impl Widget<(), Theme, iced_test::renderer::Renderer> for FixedRow {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(20.0))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced_test::renderer::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(limits.max().width, 20.0))
    }

    fn draw(
        &self,
        _tree: &Tree,
        _renderer: &mut iced_test::renderer::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }
}

fn rows() -> Vec<(
    u64,
    Element<'static, (), Theme, iced_test::renderer::Renderer>,
)> {
    (0..ROWS)
        .map(|key| (key as u64, Element::new(FixedRow)))
        .collect()
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
fn repeated_diff_and_layout_skip_temporary_buffers() {
    const DIFF_FRAMES: usize = 32;
    const LAYOUT_FRAMES: usize = 16;
    const REORDER_FRAMES: usize = 32;

    let renderer = renderer();
    let mut initial = virtual_keyed_children(rows(), 20.0);
    let mut tree = Tree::new(&initial as &dyn Widget<(), Theme, iced_test::renderer::Renderer>);
    let limits = layout::Limits::new(Size::ZERO, Size::new(240.0, 100_000.0));
    let _ = initial.layout(&mut tree, &renderer, &limits);

    let unchanged = virtual_keyed_children(rows(), 20.0);
    unchanged.diff(std::hint::black_box(&mut tree));

    let diffed = measure((0, 0), || {
        for _ in 0..DIFF_FRAMES {
            unchanged.diff(std::hint::black_box(&mut tree));
        }
    });

    assert_eq!(
        diffed,
        (0, 0),
        "{DIFF_FRAMES} exact-key frames allocated {} times ({} bytes)",
        diffed.0,
        diffed.1
    );

    // One `Vec` of child nodes per frame, and nothing else.
    let expected = (
        LAYOUT_FRAMES,
        LAYOUT_FRAMES * ROWS * std::mem::size_of::<layout::Node>(),
    );
    let laid_out = measure(expected, || {
        for _ in 0..LAYOUT_FRAMES {
            std::hint::black_box(initial.layout(&mut tree, &renderer, &limits));
        }
    });

    assert_eq!(
        laid_out, expected,
        "{LAYOUT_FRAMES} layout frames allocated {} times ({} bytes)",
        laid_out.0, laid_out.1
    );

    let mut reordered_rows = rows();
    reordered_rows.rotate_left(1);
    let reordered = virtual_keyed_children(reordered_rows, 20.0);
    reordered.diff(std::hint::black_box(&mut tree));
    unchanged.diff(std::hint::black_box(&mut tree));

    let region = Region::new(GLOBAL);
    for _ in 0..REORDER_FRAMES {
        reordered.diff(std::hint::black_box(&mut tree));
        unchanged.diff(std::hint::black_box(&mut tree));
    }
    let stats = region.change();

    assert_eq!(
        stats.allocations,
        REORDER_FRAMES * 22,
        "{REORDER_FRAMES} reordered-key frame pairs allocated {} times ({} bytes)",
        stats.allocations,
        stats.bytes_allocated
    );
    assert_eq!(
        stats.bytes_allocated,
        REORDER_FRAMES * 155_448,
        "{REORDER_FRAMES} reordered-key frame pairs allocated {} bytes",
        stats.bytes_allocated
    );
}
