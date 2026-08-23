//! What one accessible node costs per frame, in allocations.
//!
//! Every semantic node carries a focus id, but only `operate` ever reads one,
//! and only a handful of nodes ever override the id derived from their own
//! `StableId`. So the derived id is resolved where it is read rather than
//! formatted for every node the moment it is built and cloned again by every
//! `diff` — a per-node, per-frame `String` for a value most nodes hand
//! straight back to the same `custom` reader.
//!
//! Both halves live in one test because `stats_alloc` counts the whole
//! process: a second `#[test]` in this binary would run on another thread
//! inside this one's measured window.

use std::alloc::System;

use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Layout, layout};
use iced::{Element, Font, Pixels, Size, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::{Role, StableId, accessible};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type Renderer = iced_test::renderer::Renderer;
type TestElement = Element<'static, (), Theme, Renderer>;

const NODES: usize = 1_000;
const FRAMES: usize = 8;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and that lands
/// inside whichever region is open. So each batch runs in its own window, up
/// to [`WINDOWS`] times, and the contract asks for one clean window rather
/// than a clean process.
const WINDOWS: usize = 4;

/// Runs `batch` in a fresh allocator window, up to [`WINDOWS`] times, and
/// returns the first window reporting `expected` allocations — or the last
/// window's stats, when none did.
fn clean_window(expected: usize, mut batch: impl FnMut()) -> stats_alloc::Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if stats.allocations == expected {
            break;
        }
    }
    stats
}

/// A walk that visits every node and reads nothing, so the allocations it
/// measures are the tree's own rather than the operation's.
struct Walk;

impl Operation for Walk {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }
}

fn renderer() -> Renderer {
    iced_test::futures::futures::executor::block_on(<Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        None,
    ))
    .expect("headless renderer")
}

/// `Role::Button` focuses the wrapper itself, so these nodes exercise both
/// focus-id readers in `operate`, not just the ungated one.
fn nodes() -> Vec<TestElement> {
    (0..NODES)
        .map(|index| {
            accessible(
                iced::widget::space(),
                StableId::new(format!("App/row({index})")),
                Role::Button,
            )
            .into()
        })
        .collect()
}

#[test]
fn an_accessible_node_costs_one_focus_id_per_operate_and_none_per_diff() {
    let renderer = renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(1_000.0, 1_000.0));
    let mut nodes = nodes();
    let mut trees = nodes.iter().map(Tree::new).collect::<Vec<_>>();
    let layouts = nodes
        .iter_mut()
        .zip(&mut trees)
        .map(|(element, tree)| element.as_widget_mut().layout(tree, &renderer, &limits))
        .collect::<Vec<_>>();
    for ((element, tree), node) in nodes.iter_mut().zip(&mut trees).zip(&layouts) {
        element.as_widget().diff(tree);
        element
            .as_widget_mut()
            .operate(tree, Layout::new(node), &renderer, &mut Walk);
    }

    let rebuilt = clean_window(0, || {
        for _ in 0..FRAMES {
            for (element, tree) in nodes.iter().zip(&mut trees) {
                element.as_widget().diff(std::hint::black_box(tree));
            }
        }
    });
    assert_eq!(rebuilt.allocations, 0, "{rebuilt:?}");

    let walked = clean_window(NODES * FRAMES, || {
        for _ in 0..FRAMES {
            for ((element, tree), node) in nodes.iter_mut().zip(&mut trees).zip(&layouts) {
                element.as_widget_mut().operate(
                    std::hint::black_box(tree),
                    Layout::new(node),
                    &renderer,
                    &mut Walk,
                );
            }
        }
    });
    assert_eq!(walked.allocations, NODES * FRAMES, "{walked:?}");
}
