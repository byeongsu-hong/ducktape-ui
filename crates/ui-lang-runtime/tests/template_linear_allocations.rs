use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::template::{
    A11y, Axis, Node, SlotCounts, Slots, SubtreeSlot, Template, render,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn ordinary_linear_children_do_not_allocate_temporary_vectors() {
    const CHILDREN: usize = 64;
    // The direct loop measures 71 allocations. The old `flat_map` path
    // measured 135: one additional singleton `Vec` for every ordinary child.
    const ALLOCATION_BUDGET: usize = 80;

    let template = Template {
        root: Node::Linear {
            a11y: A11y {
                segment: "content".into(),
                named: true,
                source: None,
            },
            axis: Axis::Column,
            spacing: None,
            padding: None,
            width: None,
            height: None,
            align_x: None,
            align_y: None,
            children: (0..CHILDREN)
                .map(|slot| Node::Subtree {
                    slot: SubtreeSlot(slot),
                })
                .collect(),
        },
        slots: SlotCounts {
            subtrees: CHILDREN,
            ..SlotCounts::default()
        },
    };
    let mut slots = Slots::<()>::with_capacity(template.slots);
    for _ in 0..CHILDREN {
        slots.push_subtree(iced::widget::Space::new());
    }

    let region = Region::new(GLOBAL);
    let rendered = render(&template, &slots, &[], "app", &[]);
    let stats = region.change();
    drop(rendered);

    eprintln!(
        "{CHILDREN} ordinary linear children: {} allocations, {} bytes",
        stats.allocations, stats.bytes_allocated
    );
    assert!(
        stats.allocations <= ALLOCATION_BUDGET,
        "linear render allocated {} times ({} bytes)",
        stats.allocations,
        stats.bytes_allocated
    );
}
