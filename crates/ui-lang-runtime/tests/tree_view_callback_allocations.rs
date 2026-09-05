use iced::{Element, Theme};
use ui_lang_runtime::{TreeViewConfig, TreeViewId, TreeViewNode, TreeViewState, tree_view};

mod common;
use common::clean_window;

type Renderer = iced_test::renderer::Renderer;

#[test]
fn tree_callbacks_share_one_allocation() {
    const FRAMES: usize = 256;
    const ALLOCATIONS: usize = 5_120;
    const ALLOCATED_BYTES: usize = 356_096;

    let config = TreeViewConfig::new(20.0).unwrap();
    let items = [1_u64];
    let mut state = TreeViewState::new(TreeViewId::new("callback-allocation-contract"));
    state
        .reconcile(&items, |key| TreeViewNode::leaf(*key, None), config)
        .unwrap();

    let stats = clean_window((ALLOCATIONS, ALLOCATED_BYTES), || {
        for _ in 0..FRAMES {
            let element: Element<'_, (), Theme, Renderer> = tree_view(
                &state,
                &items,
                config,
                "Tree",
                |_| String::new(),
                |_, _, _| iced::widget::space().into(),
                |_| (),
            );
            drop(std::hint::black_box(element));
        }
    });

    eprintln!(
        "{FRAMES} tree renders: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, ALLOCATIONS);
    assert_eq!(stats.bytes_allocated, ALLOCATED_BYTES);
}
