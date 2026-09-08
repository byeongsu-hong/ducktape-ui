//! Run without libtest: its coordinator may allocate on another thread while
//! a process-global allocation region is being measured. Keep the zero budget.
use serde_json::Value;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;
use ui_lang_template::trace::*;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn artifact() -> Artifact {
    Artifact {
        artifact_kind: ARTIFACT_KIND.into(),
        schema_version: SCHEMA_VERSION,
        app_root: "src/ui/app.ice".into(),
        package: "demo".into(),
        environment: Environment {
            preset: Some("busy".into()),
            viewport_width: 800.0,
            viewport_height: 600.0,
            theme: Some("dark".into()),
            system_theme: "none".into(),
            scale_factor: 1.0,
            locale: Some("en-US".into()),
            platform: "linux".into(),
            reduced_motion: Some(true),
            build_profile: "release".into(),
        },
        configuration: Configuration {
            mode: Mode::Fuzz,
            test: None,
            warmup: 0,
            repeat: 1,
            steps: Some(10),
            confirmations: 2,
            deadline_ms: Some(16.0),
            max_to_median_ratio: Some(4.0),
            generator_version: Some(GENERATOR_VERSION),
        },
        seed: Some(42),
        actions: vec![Action {
            index: 0,
            kind: "redraw".into(),
            target: None,
            parameters: Value::Null,
            source: SourceLocation {
                path: "src/ui/app.ice".into(),
                line: 1,
                column: 1,
                statement: "fuzz action 0".into(),
            },
            target_source: None,
        }],
        samples: vec![Sample {
            run: 0,
            action_index: 0,
            phase: Phase::Action,
            duration_ns: 1,
        }],
        summaries: vec![Summary {
            action_index: 0,
            phase: Phase::Action,
            samples: 1,
            p50_ns: 1,
            p95_ns: 1,
            p99_ns: 1,
            max_ns: 1,
            deadline_misses_60hz: 0,
            deadline_misses_120hz: 0,
        }],
        unavailable_phases: vec![Phase::Draw],
        finding: None,
        worst_states: Vec::new(),
        reduction: None,
    }
}

fn main() {
    const ACTIONS: usize = 4_000;
    let mut artifact = artifact();
    artifact.actions = (0..ACTIONS)
        .map(|index| Action {
            index,
            kind: "redraw".into(),
            target: None,
            parameters: Value::Null,
            source: SourceLocation {
                path: "src/ui/app.ice".into(),
                line: index + 1,
                column: 1,
                statement: "redraw".into(),
            },
            target_source: None,
        })
        .collect();

    let region = Region::new(GLOBAL);
    std::hint::black_box(&artifact).validate().unwrap();
    let stats = region.change();

    eprintln!(
        "{ACTIONS} valid trace actions: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, 0, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 0, "{stats:?}");
}
