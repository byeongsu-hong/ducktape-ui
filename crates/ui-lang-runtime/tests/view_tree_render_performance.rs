//! What rendering a view module's tree costs the window thread, per redraw.
//!
//! `view_tree::render` runs on EVERY frame — the guest's `frame_rev` only
//! decides whether to publish a wake, not whether the host rebuilds the
//! element — so a tree the guest chose to fill (up to `wire::MAX_NODES`) is
//! a per-frame cost every other window on the same thread waits behind.
//!
//! Both trees are measured in one test because `stats_alloc` counts the
//! whole process: a second `#[test]` in this binary would run on another
//! thread inside this one's measured window.

#![cfg(not(debug_assertions))]

mod common;
use common::clean_window_allocations;

use std::time::Instant;

use common::assert_wall_clock_budgets;
use ui_lang_runtime::view_tree::{Inputs, render};
use ui_lang_wire as wire;

fn text_node(nth: usize) -> wire::Node {
    wire::Node::Text {
        key: format!("App/row/{nth}"),
        content: format!("row {nth}"),
        size: None,
        color: None,
        font: wire::Font::default(),
        width: None,
        align_x: None,
    }
}

fn input_node(nth: usize) -> wire::Node {
    wire::Node::Input {
        key: format!("App/field/{nth}"),
        placeholder: String::new(),
        value: String::new(),
        on_input: 0,
        on_submit: None,
        width: None,
        secure: false,
        style: wire::InputStyle::default(),
    }
}

fn column_of(children: Vec<wire::Node>) -> wire::Node {
    wire::Node::Linear {
        key: "App/column".into(),
        axis: wire::Axis::Column,
        spacing: None,
        padding: None,
        width: None,
        height: None,
        align: None,
        children,
    }
}

/// One render of a tree the host has already seen: the steady state of a
/// running app, redrawn every frame regardless of whether the guest changed
/// anything.
fn render_again(root: &wire::Node, inputs: &Inputs, samples: usize) -> Vec<u128> {
    (0..samples)
        .map(|_| {
            let started = Instant::now();
            drop(std::hint::black_box(render(
                std::hint::black_box(root),
                std::hint::black_box(inputs),
            )));
            started.elapsed().as_micros()
        })
        .collect()
}

/// The wire's node cap, filled with one kind of node, rendered once and
/// timed: the shape every other `MAX_NODES`-sized screen shares.
fn render_within_budget(
    label: &str,
    root: &wire::Node,
    inputs: &Inputs,
    expected_allocations: usize,
    p50_budget_us: u128,
    p95_budget_us: u128,
) {
    drop(render(root, inputs));
    let stats = clean_window_allocations(expected_allocations, || {
        drop(render(std::hint::black_box(root), inputs));
    });
    assert!(
        stats.allocations <= expected_allocations,
        "{label}: expected at most {expected_allocations} allocations, got {stats:?}"
    );

    let (p50, p95) = assert_wall_clock_budgets(
        label,
        render_again(root, inputs, 16),
        p50_budget_us,
        p95_budget_us,
        || render_again(root, inputs, 16),
    );
    eprintln!(
        "{label}: p50={p50}us p95={p95}us allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
}

#[test]
fn a_column_at_the_wire_cap_renders_within_budget() {
    const NODES: usize = wire::MAX_NODES - 1;

    // Measured after the clone cuts in `view_tree::render_node`: a column's
    // own wrapping (the `Vec` collecting rendered children, its container)
    // adds a fixed handful of allocations on top of a flat per-node cost —
    // 5 per text node, plus 5 once for the column. A regression that adds
    // an allocation back per node fails immediately, rather than waiting to
    // be noticed on a slower screen.
    const TEXT_ALLOCATIONS_PER_NODE: usize = 5;
    const TEXT_FIXED_OVERHEAD: usize = 5;
    const TEXT_EXPECTED_ALLOCATIONS: usize =
        NODES * TEXT_ALLOCATIONS_PER_NODE + TEXT_FIXED_OVERHEAD;
    const TEXT_P50_BUDGET_US: u128 = 20_000;
    const TEXT_P95_BUDGET_US: u128 = 35_000;

    let text_root = column_of((0..NODES).map(text_node).collect());
    let text_inputs = Inputs::default();
    render_within_budget(
        "view_tree render (text)",
        &text_root,
        &text_inputs,
        TEXT_EXPECTED_ALLOCATIONS,
        TEXT_P50_BUDGET_US,
        TEXT_P95_BUDGET_US,
    );

    // An input node carries an `Id` for the widget itself and another for
    // the accessible wrapper's focus id, on top of the text node's shape —
    // that pair of `widget::Id` allocations is the whole gap.
    const INPUT_ALLOCATIONS_PER_NODE: usize = 8;
    const INPUT_FIXED_OVERHEAD: usize = 5;
    const INPUT_EXPECTED_ALLOCATIONS: usize =
        NODES * INPUT_ALLOCATIONS_PER_NODE + INPUT_FIXED_OVERHEAD;
    const INPUT_P50_BUDGET_US: u128 = 130_000;
    const INPUT_P95_BUDGET_US: u128 = 200_000;

    let input_root = column_of((0..NODES).map(input_node).collect());
    let mut input_inputs = Inputs::default();
    input_inputs.adopt(&input_root);
    render_within_budget(
        "view_tree render (input)",
        &input_root,
        &input_inputs,
        INPUT_EXPECTED_ALLOCATIONS,
        INPUT_P50_BUDGET_US,
        INPUT_P95_BUDGET_US,
    );
}
