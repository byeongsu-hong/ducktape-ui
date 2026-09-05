//! What an accessible node's `diff` costs per frame once it carries strings.
//!
//! A node's label, value and logical id are `String`s in its semantic
//! snapshot, and the tree keeps a copy of that snapshot. A pass whose element
//! carries the same strings the tree already holds compares them instead of
//! cloning them and dropping the old copy: on a dense screen that was a
//! few `String`s per node per frame for nodes that never changed.

use iced::advanced::widget::Tree;
use iced::{Element, Theme};
use ui_lang_runtime::{Role, StableId, accessible};

mod common;
use common::clean_window_allocations;

type Renderer = iced_test::renderer::Renderer;
type TestElement = Element<'static, (), Theme, Renderer>;

const NODES: usize = 1_000;
const FRAMES: usize = 8;

/// Nodes the way generated code builds them: a logical id, a label and a
/// value, each its own `String`.
fn nodes() -> Vec<TestElement> {
    (0..NODES)
        .map(|index| {
            let key = format!("App/row({index})");
            accessible(iced::widget::space(), StableId::new(&key), Role::Label)
                .logical_id(key)
                .label(format!("Row {index}"))
                .value(format!("{index}"))
                .into()
        })
        .collect()
}

#[test]
fn an_unchanged_node_diffs_without_cloning_its_strings() {
    let nodes = nodes();
    let mut trees = nodes.iter().map(Tree::new).collect::<Vec<_>>();
    for (element, tree) in nodes.iter().zip(&mut trees) {
        element.as_widget().diff(tree);
    }

    let held = clean_window_allocations(0, || {
        for _ in 0..FRAMES {
            for (element, tree) in nodes.iter().zip(&mut trees) {
                element.as_widget().diff(std::hint::black_box(tree));
            }
        }
    });
    assert_eq!(held.allocations, 0, "{held:?}");

    // A changed snapshot still lands in the tree, at the cost of its strings:
    // one for the moved node's value, three to put the original back.
    let moved = (0..NODES)
        .map(|_| -> TestElement {
            accessible(
                iced::widget::space(),
                StableId::new("App/moved"),
                Role::Label,
            )
            .value("moved")
            .into()
        })
        .collect::<Vec<_>>();
    let changed = clean_window_allocations(NODES * 4, || {
        for (element, tree) in moved.iter().zip(&mut trees) {
            element.as_widget().diff(std::hint::black_box(tree));
        }
        for (element, tree) in nodes.iter().zip(&mut trees) {
            element.as_widget().diff(std::hint::black_box(tree));
        }
    });
    assert_eq!(changed.allocations, NODES * 4, "{changed:?}");
}
