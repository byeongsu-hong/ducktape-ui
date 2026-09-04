//! A view module's frame is taken in on the window thread, so what the
//! host spends per frame is what every window sharing that thread waits
//! for. The tree here is the widest one the wire allows — a screen of
//! inputs at `MAX_NODES` — and the contract is that taking it in stays
//! flat in the number of inputs rather than growing with their square.
//!
//! The budget is generous against a shared runner; a quadratic `adopt` of
//! this tree measured 69 ms, two orders over what is asserted here.

#![cfg(not(debug_assertions))]

mod common;

use std::time::Instant;

use common::assert_wall_clock_budgets;
use ui_lang_runtime::view_tree::Inputs;
use ui_lang_wire as wire;

fn input(nth: usize) -> wire::Node {
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

fn screen_of_inputs() -> wire::Node {
    wire::Node::Linear {
        key: "App/form".into(),
        axis: wire::Axis::Column,
        spacing: None,
        padding: None,
        width: None,
        height: None,
        align: None,
        children: (0..wire::MAX_NODES - 1).map(input).collect(),
    }
}

/// One `adopt` of a tree the host has already seen: the steady state of a
/// running app, and the one a guest can force on every frame.
fn adopt_again(root: &wire::Node, samples: usize) -> Vec<u128> {
    let mut inputs = Inputs::default();
    inputs.adopt(root);
    (0..samples)
        .map(|_| {
            let started = Instant::now();
            inputs.adopt(root);
            started.elapsed().as_micros()
        })
        .collect()
}

#[test]
fn taking_in_a_full_screen_of_inputs_stays_flat() {
    // Wide of the 1.3 ms this measures locally, because a shared runner is
    // slower than a desk and the regression it guards is not subtle: the
    // list this replaced took 69 ms, four times the p95 below.
    const P50_BUDGET_US: u128 = 15_000;
    const P95_BUDGET_US: u128 = 30_000;

    let root = screen_of_inputs();
    let (p50, p95) = assert_wall_clock_budgets(
        "view_tree adopt",
        adopt_again(&root, 32),
        P50_BUDGET_US,
        P95_BUDGET_US,
        || adopt_again(&root, 32),
    );
    eprintln!("full screen of inputs adopted: p50={p50}us p95={p95}us");
}
