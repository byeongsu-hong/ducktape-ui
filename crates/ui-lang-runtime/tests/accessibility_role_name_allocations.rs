use std::alloc::System;

use iced::{Element, Task, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::testing::{Config, Driver, Location};
use ui_lang_runtime::{Role, StableId, accessible};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const HERE: Location = Location::new(
    "accessibility-role-name.ice",
    1,
    1,
    "read the semantic role name",
);

#[test]
fn repeated_public_role_name_reads_do_not_collect_characters() {
    const READS: usize = 32;

    let mut driver = Driver::new(
        iced::application::<(), (), Theme, iced::Renderer>(boot, update, view),
        Config::new("accessibility_role_name_allocations").viewport(160.0, 80.0),
    );
    let target = driver.target("App/item", HERE);
    assert_eq!(target.accessibility_role_name(), "tree-item");

    let region = Region::new(GLOBAL);
    for _ in 0..READS {
        assert_eq!(
            std::hint::black_box(target.accessibility_role_name()),
            "tree-item"
        );
    }
    let stats = region.change();

    eprintln!(
        "{READS} role name reads: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, READS * 2, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}

fn boot() {}

fn update(_state: &mut (), _message: ()) -> Task<()> {
    Task::none()
}

fn view(_state: &()) -> Element<'_, ()> {
    accessible(
        iced::widget::text("item"),
        StableId::new("App/item"),
        Role::TreeItem,
    )
    .logical_id("App/item")
    .into()
}
