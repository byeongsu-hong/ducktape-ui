use std::alloc::System;

use iced::advanced::layout;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Element, Font, Pixels, Size, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::{FlexItem, flex, flex_item};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type Renderer = iced_test::renderer::Renderer;

fn items(count: usize) -> Vec<FlexItem<'static, (), Theme, Renderer>> {
    (0..count)
        .map(|_| {
            let child: Element<'static, (), Theme, Renderer> = iced::widget::Space::new().into();
            flex_item(child)
        })
        .collect()
}

#[test]
fn default_order_layout_skips_the_ordering_scratch_vector() {
    const ITEMS: usize = 1_000;
    const FRAMES: usize = 32;
    const MAX_ALLOCATIONS: usize = FRAMES * 3 + 1;
    const MAX_ALLOCATED_BYTES: usize = FRAMES * 104_024 + 4_096;

    let renderer = iced_test::futures::futures::executor::block_on(<Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        None,
    ))
    .expect("headless renderer");
    let mut element: Element<'static, (), Theme, Renderer> = flex(items(ITEMS)).into();
    let mut tree = Tree::new(&element);
    let limits = layout::Limits::new(Size::ZERO, Size::new(1_000.0, 1_000.0));

    drop(
        element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits),
    );

    let region = Region::new(GLOBAL);
    for _ in 0..FRAMES {
        drop(
            element
                .as_widget_mut()
                .layout(std::hint::black_box(&mut tree), &renderer, &limits),
        );
    }
    let stats = region.change();

    assert!(
        stats.allocations <= MAX_ALLOCATIONS,
        "expected at most {MAX_ALLOCATIONS} allocations, got {stats:?}"
    );
    assert!(
        stats.bytes_allocated <= MAX_ALLOCATED_BYTES,
        "expected at most {MAX_ALLOCATED_BYTES} allocated bytes, got {stats:?}"
    );
}
