use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::template::{
    A11y, Axis, GroupSlot, Node, SlotCounts, Slots, SubtreeSlot, Template, render,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn linear_children_use_bounded_temporary_storage() {
    const CHILDREN: usize = 64;
    // The direct loop measures 70 allocations. The old `flat_map` path
    // measured 135: one additional singleton `Vec` for every ordinary child.
    // The headroom between the two is what makes this a shape guard rather
    // than a pin, so it moves only by what a change actually earns.
    const ALLOCATION_BUDGET: usize = 79;

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

    // Key formatting may make one 8-byte growth request; expanding this group
    // from the template's one placeholder used to add 1,008 bytes of its own.
    const REALLOCATION_BUDGET: usize = 1;
    const REALLOCATED_BYTES_BUDGET: isize = 8;

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
            children: vec![Node::Group { slot: GroupSlot(0) }],
        },
        slots: SlotCounts {
            groups: 1,
            ..SlotCounts::default()
        },
    };
    let mut slots = Slots::<()>::with_capacity(template.slots);
    slots.push_group(
        (0..CHILDREN)
            .map(|_| iced::widget::Space::new().into())
            .collect(),
    );

    let region = Region::new(GLOBAL);
    let rendered = render(&template, &slots, &[], "app", &[]);
    let stats = region.change();
    drop(rendered);

    eprintln!(
        "{CHILDREN} dynamic group children: {} allocations, {} reallocations, {} bytes reallocated",
        stats.allocations, stats.reallocations, stats.bytes_reallocated
    );
    assert!(
        stats.reallocations <= REALLOCATION_BUDGET
            && stats.bytes_reallocated <= REALLOCATED_BYTES_BUDGET,
        "group expansion grew storage {} times by {} bytes",
        stats.reallocations,
        stats.bytes_reallocated
    );
}
