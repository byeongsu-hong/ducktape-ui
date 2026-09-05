use iced::advanced::Widget;
use iced::advanced::widget::Tree;
use iced::{Element, Theme};
use ui_lang_runtime::{flex, flex_item};

mod common;
use common::clean_window;

type Renderer = iced_test::renderer::Renderer;

fn items(count: usize) -> Vec<ui_lang_runtime::FlexItem<'static, (), Theme, Renderer>> {
    (0..count)
        .map(|_| {
            let child: Element<'static, (), Theme, Renderer> = iced::widget::Space::new().into();
            flex_item(child)
        })
        .collect()
}

#[test]
fn repeated_flex_diff_does_not_allocate_a_child_reference_vector() {
    const ITEMS: usize = 1_000;
    const FRAMES: usize = 32;

    let initial = flex(items(ITEMS));
    let mut tree = Tree::new(&initial as &dyn Widget<(), Theme, Renderer>);
    let unchanged = flex(items(ITEMS));
    unchanged.diff(std::hint::black_box(&mut tree));

    let stats = clean_window((0, 0), || {
        for _ in 0..FRAMES {
            unchanged.diff(std::hint::black_box(&mut tree));
        }
    });

    assert_eq!(stats.allocations, 0, "{stats:?}");
}
