//! `canvas` calls `Program::draw` once per frame, so a dashed border that
//! rebuilds its `Frame` there re-tessellates the same stroke forever. This
//! contract holds the geometry to one build per parameter change, and — the
//! half that protects correctness — proves a parameter change still rebuilds
//! it, so a cache that simply never invalidated could not pass.
//!
//! Rebuilding every frame measured 7 allocations and 6 reallocations per frame
//! (448 + 768 over 64 frames, 100_864 bytes); the cached stroke measures 1 and
//! 0 (64 over 64 frames, 6_144 bytes). The one that remains is the
//! `Vec<Geometry>` `Program::draw` returns by signature, so it is the floor.
use std::hint::black_box;

use iced::advanced::renderer::Headless;
use iced::border::Radius;
use iced::widget::canvas::Program;
use iced::{Color, Font, Pixels, Rectangle, Size, Theme, mouse};
use stats_alloc::Region;
use ui_lang_runtime::{DashedBorder, DashedBorderCache};

mod common;
use common::{GLOBAL, clean_window_allocations};

/// The frame budget of one dashed border that only ever redraws: the
/// `Vec<Geometry>` `Program::draw` returns by signature, and nothing else.
const PER_CACHED_FRAME: usize = 1;

fn border(color: Color) -> DashedBorder {
    DashedBorder::new(color, 1.5, Radius::new(6.0), vec![4.0, 3.0])
}

fn draw(program: &DashedBorder, state: &DashedBorderCache, renderer: &iced::Renderer) -> String {
    let geometry = Program::<()>::draw(
        black_box(program),
        black_box(state),
        black_box(renderer),
        &Theme::Light,
        black_box(Rectangle::with_size(Size::new(320.0, 200.0))),
        mouse::Cursor::Unavailable,
    );
    format!("{geometry:?}")
}

#[test]
fn performance_contract_dashed_border_tessellates_once_per_parameter_change() {
    const FRAMES: usize = 64;

    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let red = border(Color::from_rgb(1.0, 0.0, 0.0));
    let blue = border(Color::from_rgb(0.0, 0.0, 1.0));
    let state = DashedBorderCache::default();

    // The stroke is real geometry, and a colour change is a different stroke:
    // if these ever matched, the "unchanged" window below would be measuring
    // an empty draw.
    let first = draw(&red, &state, &renderer);
    assert!(first.contains("Stroke"), "{first}");
    assert_ne!(first, draw(&blue, &state, &renderer));

    // A repeat of the *same* parameters must return the same geometry without
    // rebuilding it.
    let warm = draw(&red, &state, &renderer);
    assert_eq!(warm, first);
    let unchanged = clean_window_allocations(FRAMES * PER_CACHED_FRAME, || {
        for _ in 0..FRAMES {
            drop(black_box(Program::<()>::draw(
                black_box(&red),
                black_box(&state),
                black_box(&renderer),
                &Theme::Light,
                black_box(Rectangle::with_size(Size::new(320.0, 200.0))),
                mouse::Cursor::Unavailable,
            )));
        }
    });
    assert_eq!(
        unchanged.allocations,
        FRAMES * PER_CACHED_FRAME,
        "{unchanged:?}"
    );

    // And a parameter change must still rebuild: without this the cache could
    // pass the window above by never invalidating at all.
    let alternating = Region::new(GLOBAL);
    for frame in 0..FRAMES {
        let program = if frame % 2 == 0 { &red } else { &blue };
        drop(black_box(Program::<()>::draw(
            black_box(program),
            black_box(&state),
            black_box(&renderer),
            &Theme::Light,
            black_box(Rectangle::with_size(Size::new(320.0, 200.0))),
            mouse::Cursor::Unavailable,
        )));
    }
    let alternating = alternating.change();
    assert!(
        alternating.allocations > 8 * FRAMES * PER_CACHED_FRAME,
        "a changed colour must re-tessellate: {alternating:?}"
    );
    assert_eq!(draw(&red, &state, &renderer), first);
}
