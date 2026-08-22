use std::alloc::System;

use iced::{Element, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::press_area;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type Renderer = iced_test::renderer::Renderer;

#[test]
fn press_callbacks_use_the_widget_allocation() {
    const FRAMES: usize = 64;

    let mut contents: Vec<Element<'static, u64, Theme, Renderer>> = (0..FRAMES)
        .map(|_| iced::widget::Space::new().into())
        .collect();

    let region = Region::new(GLOBAL);
    for (index, content) in contents.drain(..).enumerate() {
        let area: Element<'static, u64, Theme, Renderer> = press_area(content)
            .on_press_at(move |_| index as u64)
            .into();
        drop(std::hint::black_box(area));
    }
    let stats = region.change();

    assert_eq!(
        stats.allocations, FRAMES,
        "{FRAMES} press areas allocated {} times ({} bytes)",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(
        stats.bytes_allocated, 2_048,
        "{FRAMES} press areas allocated {} bytes",
        stats.bytes_allocated
    );
}
