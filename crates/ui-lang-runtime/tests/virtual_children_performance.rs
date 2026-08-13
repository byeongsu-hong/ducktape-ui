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
fn repeated_exact_key_diff_allocates_nothing() {
    const FRAMES: usize = 32;

    let renderer = renderer();
    let mut initial = virtual_keyed_children(rows(), 20.0);
    let mut tree = Tree::new(&initial as &dyn Widget<(), Theme, iced_test::renderer::Renderer>);
    let _ = initial.layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(240.0, 100_000.0)),
    );

    let unchanged = virtual_keyed_children(rows(), 20.0);
    unchanged.diff(std::hint::black_box(&mut tree));

    let region = Region::new(GLOBAL);
    for _ in 0..FRAMES {
        unchanged.diff(std::hint::black_box(&mut tree));
    }
    let stats = region.change();

    assert_eq!(
        stats.allocations, 0,
        "{FRAMES} exact-key frames allocated {} times ({} bytes)",
        stats.allocations, stats.bytes_allocated
    );
}
