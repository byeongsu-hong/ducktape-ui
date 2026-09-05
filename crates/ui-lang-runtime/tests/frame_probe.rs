//! A composite frame-cost probe shaped like the ducktape chat screen: a
//! scrollable stream of `memo_lazy` rows over a text-input composer. Not a
//! contract — it prints per-phase costs (build, event walk, draw, keystroke
//! rebuild, one-row edit, screen switch) so the perf loop can see which phase
//! dominates a real frame before optimizing it.
#![cfg(not(debug_assertions))]

mod common;
use common::GLOBAL;

use common::{percentile, percentile_usize};

use iced::advanced::renderer;
use iced::advanced::{clipboard, mouse};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Event, Font, Length, Pixels, Point, Size, Theme};
use iced_test::runtime::UserInterface;
use iced_test::runtime::user_interface;
use stats_alloc::Region;
use std::sync::Arc;
use ui_lang_runtime::memo_lazy;
use ui_lang_runtime::{
    VirtualListConfig, VirtualListEvent, VirtualListId, VirtualListState, virtual_list,
};

const ROWS: usize = 150;
const WINDOW: Size = Size::new(1280.0, 800.0);
const WARMUP_FRAMES: usize = 8;
const FRAMES: usize = 60;

#[derive(Debug, Clone)]
enum TimelineMessage {
    List(VirtualListEvent<u64>),
}

#[derive(Debug, Clone)]
enum Message {
    #[allow(dead_code)]
    Composer(String),
    #[allow(dead_code)]
    React(usize),
}

#[derive(Clone)]
struct Model {
    bodies: Arc<Vec<String>>,
    versions: Vec<u64>,
    composer: String,
}

fn model() -> Model {
    Model {
        bodies: Arc::new(
            (0..ROWS)
                .map(|index| {
                    format!(
                        "message {index}: the quick brown fox jumps over the lazy dog \
                         while the review bot files another finding about wrapping \
                         behavior in long chat lines that span two rendered rows"
                    )
                })
                .collect(),
        ),
        versions: vec![0; ROWS],
        composer: String::new(),
    }
}

fn view(model: &Model) -> Element<'static, Message, Theme, iced_test::renderer::Renderer> {
    let stream = column(model.versions.iter().enumerate().map(|(index, version)| {
        let bodies = Arc::clone(&model.bodies);
        memo_lazy(
            (index, *version),
            move |&(index, _): &(usize, u64)| -> Element<
                'static,
                Message,
                Theme,
                iced_test::renderer::Renderer,
            > {
                row![
                    text(format!("user-{}", index % 7)).width(120.0),
                    column![
                        text(bodies[index].clone()).size(14),
                        row![
                            button(text("react")).on_press(Message::React(index)),
                            text(format!("{} replies", index % 3)).size(12),
                        ]
                        .spacing(8.0),
                    ]
                    .width(Length::Fill),
                ]
                .spacing(12.0)
                .into()
            },
            index as u64,
            index,
        )
        .into()
    }))
    .spacing(6.0);

    let composer = text_input("Message #general", &model.composer)
        .on_input(Message::Composer)
        .padding(10.0);

    container(column![scrollable(stream).height(Length::Fill), composer,])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

struct Phase {
    label: &'static str,
    elapsed_us: Vec<u128>,
    allocations: Vec<usize>,
    allocated_bytes: Vec<usize>,
}

impl Phase {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            elapsed_us: Vec::with_capacity(FRAMES),
            allocations: Vec::with_capacity(FRAMES),
            allocated_bytes: Vec::with_capacity(FRAMES),
        }
    }

    fn sample<T>(&mut self, work: impl FnOnce() -> T) -> T {
        let region = Region::new(GLOBAL);
        let started = std::time::Instant::now();
        let value = work();
        self.elapsed_us.push(started.elapsed().as_micros());
        let stats = region.change();
        self.allocations.push(stats.allocations);
        self.allocated_bytes.push(stats.bytes_allocated);
        value
    }

    fn p50(&self) -> u128 {
        percentile(&self.elapsed_us, 50).max(1)
    }

    fn report(&self) {
        let p50 = percentile(&self.elapsed_us, 50);
        let p95 = percentile(&self.elapsed_us, 95);
        let allocations = percentile_usize(&self.allocations, 95);
        let bytes = percentile_usize(&self.allocated_bytes, 95);
        eprintln!(
            "{:<28} p50={p50:>6}us p95={p95:>6}us allocs(p95)={allocations:>6} bytes(p95)={bytes:>9}",
            self.label
        );
    }
}

fn renderer() -> iced_test::renderer::Renderer {
    iced_test::futures::futures::executor::block_on(
        <iced_test::renderer::Renderer as renderer::Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            None,
        ),
    )
    .expect("headless renderer")
}

#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn chat_frame_phase_costs() {
    let mut model = model();
    let mut renderer = renderer();
    let mut clipboard = clipboard::Null;
    let mut messages: Vec<Message> = Vec::new();
    let mut cache = user_interface::Cache::default();

    for _ in 0..WARMUP_FRAMES {
        let ui = UserInterface::build(view(&model), WINDOW, cache, &mut renderer);
        cache = ui.into_cache();
    }

    let mut build_phase = Phase::new("unchanged build+layout");
    let mut cursor_phase = Phase::new("cursor-move update walk");
    let mut draw_phase = Phase::new("draw");
    let mut keystroke_phase = Phase::new("composer keystroke rebuild");
    let mut edit_phase = Phase::new("one-row edit rebuild");
    let mut switch_phase = Phase::new("screen switch (park+reclaim)");

    for frame in 0..FRAMES {
        let mut ui =
            build_phase.sample(|| UserInterface::build(view(&model), WINDOW, cache, &mut renderer));

        let position = Point::new(400.0, 100.0 + (frame % 400) as f32);
        let cursor = mouse::Cursor::Available(position);
        cursor_phase.sample(|| {
            ui.update(
                &[Event::Mouse(mouse::Event::CursorMoved { position })],
                cursor,
                &mut renderer,
                &mut clipboard,
                &mut messages,
            )
        });
        messages.clear();

        draw_phase.sample(|| {
            ui.draw(
                &mut renderer,
                &Theme::Light,
                &renderer::Style::default(),
                cursor,
            );
        });
        cache = ui.into_cache();

        model.composer.push('x');
        let ui = keystroke_phase
            .sample(|| UserInterface::build(view(&model), WINDOW, cache, &mut renderer));
        cache = ui.into_cache();

        model.versions[frame % ROWS] += 1;
        let ui =
            edit_phase.sample(|| UserInterface::build(view(&model), WINDOW, cache, &mut renderer));
        cache = ui.into_cache();

        // A screen switch drops the whole tree (parking every lazy row) and
        // rebuilds it from a fresh cache (reclaiming them).
        cache = switch_phase.sample(|| {
            drop(std::mem::take(&mut cache));
            UserInterface::build(view(&model), WINDOW, cache, &mut renderer).into_cache()
        });
    }

    // Every dependency changes while the tree stays mounted (a select-mode
    // toggle flipping every row): rebuilds go through `diff`, which preserves
    // matching text states, so unchanged bodies must not re-shape.
    let mut all_deps_phase = Phase::new("all deps changed (mounted)");
    for _ in 0..8 {
        for version in &mut model.versions {
            *version += 1;
        }
        cache = all_deps_phase.sample(|| {
            UserInterface::build(view(&model), WINDOW, cache, &mut renderer).into_cache()
        });
    }

    // A cold channel switch: every row's dependency is new, so nothing in the
    // parking lot matches — every row view-builds and re-shapes from nothing.
    let mut cold_phase = Phase::new("cold switch (all rows new)");
    for _ in 0..8 {
        for version in &mut model.versions {
            *version += 1;
        }
        drop(std::mem::take(&mut cache));
        cache = cold_phase.sample(|| {
            UserInterface::build(view(&model), WINDOW, cache, &mut renderer).into_cache()
        });
    }

    // The measured virtual list mounting the same content shape at 1000
    // rows: a cold channel open only builds and shapes the viewport window,
    // not the whole stream. One sample = the full settle — build, measure,
    // apply reported events, rebuild.
    let timeline_config = VirtualListConfig::measured(48.0).unwrap();
    let mut timeline_phases = Vec::new();
    for (label, timeline_rows) in [
        ("virtual timeline @150 cold", ROWS),
        ("virtual timeline @1000 cold", 1_000),
    ] {
        let timeline_bodies: Vec<String> = (0..timeline_rows)
            .map(|index| {
                format!(
                    "message {index}: the quick brown fox jumps over the lazy dog \
                     while the review bot files another finding about wrapping \
                     behavior in long chat lines that span two rendered rows"
                )
            })
            .collect();
        let mut timeline_phase = Phase::new(label);
        for round in 0..8 {
            timeline_phase.sample(|| {
                let mut state: VirtualListState<u64> =
                    VirtualListState::new(VirtualListId::new(format!("probe-timeline-{round}")));
                let items: Vec<u64> = (0..timeline_rows as u64).collect();
                state
                    .reconcile(&items, |key| *key, timeline_config)
                    .unwrap();
                state.apply(
                    VirtualListEvent::ViewportChanged {
                        height: WINDOW.height,
                    },
                    &items,
                    |key| *key,
                    timeline_config,
                );
                state.scroll_to_end(items.len(), timeline_config);
                let mut timeline_cache = user_interface::Cache::default();
                let mut messages: Vec<TimelineMessage> = Vec::new();
                for _ in 0..2 {
                    let element: Element<
                        '_,
                        TimelineMessage,
                        Theme,
                        iced_test::renderer::Renderer,
                    > = virtual_list(
                        &state,
                        &items,
                        timeline_config,
                        "Probe timeline",
                        |key| *key,
                        |key| format!("Item {key}"),
                        |index, _, _| {
                            column![
                                text(format!("user-{}", index % 7)),
                                text(timeline_bodies[index].clone()).size(14),
                            ]
                            .into()
                        },
                        TimelineMessage::List,
                    );
                    let mut ui =
                        UserInterface::build(element, WINDOW, timeline_cache, &mut renderer);
                    ui.update(
                        &[Event::Window(iced::window::Event::RedrawRequested(
                            iced::time::Instant::now(),
                        ))],
                        mouse::Cursor::Unavailable,
                        &mut renderer,
                        &mut clipboard,
                        &mut messages,
                    );
                    timeline_cache = ui.into_cache();
                    for message in messages.drain(..) {
                        let TimelineMessage::List(event) = message;
                        state.apply(event, &items, |key| *key, timeline_config);
                    }
                }
                std::hint::black_box(&state);
            });
        }
        timeline_phases.push(timeline_phase);
    }

    // Constructing row elements WITHOUT laying them out: the cost a
    // "build every child, lay out only the visible ones" virtualization
    // would still pay. Shaping happens in layout, so this isolates it.
    let mut construct_phase = Phase::new("construct 1000 rows, no layout");
    for _ in 0..8 {
        construct_phase.sample(|| {
            let built: Vec<Element<'_, Message, Theme, iced_test::renderer::Renderer>> = (0..1000)
                .map(|index| {
                    column![
                        text(format!("user-{}", index % 7)),
                        text(model.bodies[index % ROWS].clone()).size(14),
                    ]
                    .into()
                })
                .collect();
            std::hint::black_box(&built);
        });
    }

    eprintln!(
        "chat frame probe: {ROWS} lazy rows, {}x{}",
        WINDOW.width, WINDOW.height
    );
    build_phase.report();
    cursor_phase.report();
    draw_phase.report();
    keystroke_phase.report();
    edit_phase.report();
    switch_phase.report();
    all_deps_phase.report();
    cold_phase.report();
    construct_phase.report();
    for phase in &timeline_phases {
        phase.report();
    }

    // Ratio contracts, not absolute budgets: both sides are measured in this
    // same run on this same machine, so a busy box slows them together and
    // the comparison still holds. Absolute microsecond budgets would only
    // flake here.
    let [timeline_small, timeline_large] = &timeline_phases[..] else {
        panic!("expected exactly two timeline phases");
    };
    let column_cold = cold_phase.p50();
    let small = timeline_small.p50();
    let large = timeline_large.p50();

    // 1. Virtualizing the same row count must be decisively cheaper than
    //    building every row: the plain lazy column shapes offscreen text,
    //    the virtual list shapes only the viewport.
    assert!(
        small * 5 < column_cold,
        "virtual timeline @{ROWS} ({small}us) must beat the plain column @{ROWS} \
         ({column_cold}us) by more than 5x (measured 13x)"
    );

    // 2. The whole point: cost tracks the viewport, not the stream. Measured,
    //    6.7x the rows costs +0.3% and the exact same allocation count, so a
    //    2.5x bound is loose enough never to flake and tight enough to catch
    //    any new per-row work leaking into a frame.
    assert!(
        large * 2 < small * 5,
        "virtual timeline must stay viewport-proportional: @1000 ({large}us) \
         vs @{ROWS} ({small}us) grew more than 2.5x for 6.7x the rows"
    );
}
