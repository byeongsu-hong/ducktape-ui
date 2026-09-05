use iced::advanced::layout;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Element, Font, Length, Pixels, Size, Theme};
use ui_lang_runtime::{FlexBasis, flex, flex_item};

mod common;
use common::clean_window;

type Renderer = iced_test::renderer::Renderer;

#[test]
fn shrinking_flex_layout_reuses_item_state_for_clamping() {
    const ITEMS: usize = 64;
    const FRAMES: usize = 256;
    const ALLOCATIONS: usize = 768;
    const ALLOCATED_BYTES: usize = 1_710_080;

    let renderer = iced_test::futures::futures::executor::block_on(<Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        None,
    ))
    .expect("headless renderer");
    let items = (0..ITEMS)
        .map(|index| {
            let child: Element<'static, (), Theme, Renderer> = iced::widget::Space::new().into();
            flex_item(child)
                .basis(FlexBasis::Fixed(if index == 0 { 1.0 } else { 10.0 }))
                .shrink(if index == 0 { 100.0 } else { 1.0 })
        })
        .collect();
    let mut element: Element<'static, (), Theme, Renderer> = flex(items)
        .width(Length::Fixed(500.0))
        .height(Length::Fixed(100.0))
        .into();
    let mut tree = Tree::new(&element);
    let limits = layout::Limits::new(Size::ZERO, Size::new(500.0, 100.0));

    drop(
        element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits),
    );

    let stats = clean_window((ALLOCATIONS, ALLOCATED_BYTES), || {
        for _ in 0..FRAMES {
            let node =
                element
                    .as_widget_mut()
                    .layout(std::hint::black_box(&mut tree), &renderer, &limits);
            assert_eq!(node.size(), Size::new(500.0, 100.0));
        }
    });

    eprintln!(
        "{FRAMES} shrinking flex layouts: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, ALLOCATIONS);
    assert_eq!(stats.bytes_allocated, ALLOCATED_BYTES);
}
