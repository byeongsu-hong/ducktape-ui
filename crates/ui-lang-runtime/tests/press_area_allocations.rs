use iced::{Element, Theme};
use ui_lang_runtime::press_area;

mod common;
use common::{WINDOWS, clean_window};

type Renderer = iced_test::renderer::Renderer;

#[test]
fn press_callbacks_use_the_widget_allocation() {
    const FRAMES: usize = 64;

    // One batch's worth of contents per window, built before any window opens
    // so the measurement only ever sees the press areas.
    let mut contents: Vec<Element<'static, u64, Theme, Renderer>> = (0..FRAMES * WINDOWS)
        .map(|_| iced::widget::Space::new().into())
        .collect();

    let stats = clean_window((FRAMES, 2_048), || {
        for (index, content) in contents.drain(..FRAMES).enumerate() {
            let area: Element<'static, u64, Theme, Renderer> = press_area(content)
                .on_press_at(move |_| index as u64)
                .into();
            drop(std::hint::black_box(area));
        }
    });

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
