use std::alloc::System;

use iced::advanced::Widget;
use iced::advanced::renderer::Headless;
use iced::advanced::{layout, widget};
use iced::{Element, Size, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::zstack;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type Renderer = iced_test::renderer::Renderer;

#[test]
fn bounded_zstack_layout_allocates_only_its_retained_children() {
    const LAYERS: usize = 64;

    let renderer = iced_test::futures::futures::executor::block_on(<Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(14.0),
        None,
    ))
    .expect("headless renderer");
    let children = (0..LAYERS)
        .map(|index| {
            Element::<'static, (), Theme, Renderer>::from(
                iced::widget::Space::new()
                    .width((index + 1) as f32)
                    .height(10.0),
            )
        })
        .collect::<Vec<_>>();
    let mut stack = zstack(children);
    let mut tree = widget::Tree::new(&stack as &dyn Widget<(), Theme, Renderer>);
    let limits = layout::Limits::new(Size::ZERO, Size::new(1_000.0, 1_000.0));

    let region = Region::new(GLOBAL);
    let node = std::hint::black_box(&mut stack).layout(&mut tree, &renderer, &limits);
    let stats = region.change();

    assert_eq!(node.children().len(), LAYERS);
    for (index, child) in node.children().iter().enumerate() {
        assert_eq!(child.size(), Size::new((index + 1) as f32, 10.0));
    }
    assert_eq!(node.size(), Size::new(LAYERS as f32, 10.0));
    eprintln!(
        "{LAYERS} bounded layers: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
