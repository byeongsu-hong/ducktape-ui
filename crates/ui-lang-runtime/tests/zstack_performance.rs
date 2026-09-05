use iced::advanced::Widget;
use iced::advanced::renderer::Headless;
use iced::advanced::{layout, widget};
use iced::{Element, Size, Theme};
use ui_lang_runtime::zstack;

mod common;
use common::clean_window_allocations;

type Renderer = iced_test::renderer::Renderer;

#[test]
fn zstack_layout_allocates_only_its_retained_children() {
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

    let mut measured = None;
    let stats = clean_window_allocations(1, || {
        measured = Some(std::hint::black_box(&mut stack).layout(&mut tree, &renderer, &limits));
    });
    let node = measured.expect("a laid out bounded stack");

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

    let children = std::iter::once(Element::<'static, (), Theme, Renderer>::from(
        iced::widget::Space::new().width(64.0).height(10.0),
    ))
    .chain((1..LAYERS).map(|_| {
        Element::from(
            iced::widget::Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
    }))
    .collect::<Vec<_>>();
    let mut stack = zstack(children);
    let mut tree = widget::Tree::new(&stack as &dyn Widget<(), Theme, Renderer>);
    let limits = layout::Limits::new(Size::ZERO, Size::new(1_000.0, f32::INFINITY));

    let mut measured = None;
    let stats = clean_window_allocations(1, || {
        measured = Some(std::hint::black_box(&mut stack).layout(&mut tree, &renderer, &limits));
    });
    let node = measured.expect("a laid out unbounded stack");

    assert_eq!(node.children().len(), LAYERS);
    assert_eq!(node.size(), Size::new(64.0, 10.0));
    for child in node.children() {
        assert_eq!(child.size(), Size::new(64.0, 10.0));
    }
    eprintln!(
        "{LAYERS} unbounded layers: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
