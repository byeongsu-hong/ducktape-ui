//! The versioned interaction-trace artifact shared by Inspector and the runtime.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ARTIFACT_KIND: &str = "ice_interaction_trace";
pub const SCHEMA_VERSION: u64 = 1;
pub const GENERATOR_VERSION: u64 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Authored,
    Fuzz,
    Replay,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Action,
    View,
    UiBuildLayout,
    EventDispatch,
    ProgramUpdate,
    WidgetOperation,
    TaskSettle,
    Draw,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    Panic,
    Timeout,
    Assertion,
    Latency,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceLocation {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub statement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub index: usize,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub parameters: Value,
    pub source: SourceLocation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_source: Option<SourceLocation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub preset: Option<String>,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub theme: Option<String>,
    pub system_theme: String,
    pub scale_factor: f32,
    pub locale: Option<String>,
    pub platform: String,
    pub reduced_motion: Option<bool>,
    pub build_profile: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub mode: Mode,
    pub test: Option<String>,
    pub warmup: usize,
    pub repeat: usize,
    pub steps: Option<usize>,
    pub confirmations: usize,
    pub deadline_ms: Option<f64>,
    pub max_to_median_ratio: Option<f64>,
    pub generator_version: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub run: usize,
    pub action_index: usize,
    pub phase: Phase,
    pub duration_ns: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub action_index: usize,
    pub phase: Phase,
    pub samples: usize,
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub max_ns: u64,
    pub deadline_misses_60hz: usize,
    pub deadline_misses_120hz: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub kind: FindingKind,
    pub fingerprint: String,
    pub action_index: usize,
    pub phase: Option<Phase>,
    pub message: String,
    pub confirmed_runs: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorstState {
    pub action_index: usize,
    pub phase: Phase,
    pub duration_ns: u64,
    pub png: String,
    pub manifest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReductionAttempt {
    pub candidate_len: usize,
    pub preserved: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reduction {
    pub original_len: usize,
    pub minimized_actions: Vec<Action>,
    pub attempts: Vec<ReductionAttempt>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub artifact_kind: String,
    pub schema_version: u64,
    pub app_root: String,
    pub package: String,
    pub environment: Environment,
    pub configuration: Configuration,
    pub seed: Option<u64>,
    pub actions: Vec<Action>,
    pub samples: Vec<Sample>,
    pub summaries: Vec<Summary>,
    pub unavailable_phases: Vec<Phase>,
    pub finding: Option<Finding>,
    pub worst_states: Vec<WorstState>,
    pub reduction: Option<Reduction>,
}

impl Artifact {
    pub fn validate(&self) -> Result<(), String> {
        if self.artifact_kind != ARTIFACT_KIND {
            return Err(format!(
                "unsupported trace artifact kind {:?}; expected {ARTIFACT_KIND:?}",
                self.artifact_kind
            ));
        }
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported trace schema version {}; expected {SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.app_root.is_empty() || self.package.is_empty() {
            return Err("trace app root and package must be non-empty".into());
        }
        if self.environment.system_theme.is_empty()
            || self.environment.platform.is_empty()
            || self.environment.build_profile.is_empty()
        {
            return Err("trace environment labels must be non-empty".into());
        }
        if !self.environment.viewport_width.is_finite()
            || self.environment.viewport_width <= 0.0
            || !self.environment.viewport_height.is_finite()
            || self.environment.viewport_height <= 0.0
            || !self.environment.scale_factor.is_finite()
            || self.environment.scale_factor <= 0.0
        {
            return Err("trace viewport and scale factor must be finite and positive".into());
        }
        if self.configuration.repeat == 0 || self.configuration.confirmations == 0 {
            return Err("trace repeat and confirmation counts must be positive".into());
        }
        if self
            .configuration
            .deadline_ms
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
            || self
                .configuration
                .max_to_median_ratio
                .is_some_and(|value| !value.is_finite() || value <= 1.0)
        {
            return Err(
                "trace deadline must be positive and max-to-median ratio must exceed one".into(),
            );
        }
        for (index, action) in self.actions.iter().enumerate() {
            if action.index != index || action.kind.is_empty() {
                return Err(format!(
                    "trace action {index} must have matching index and non-empty kind"
                ));
            }
            validate_source(&action.source, format_args!("action {index} source"))?;
            if let Some(source) = &action.target_source {
                validate_source(source, format_args!("action {index} target source"))?;
            }
        }
        for sample in &self.samples {
            if sample.action_index >= self.actions.len() {
                return Err(format!(
                    "trace sample references missing action {}",
                    sample.action_index
                ));
            }
        }
        for summary in &self.summaries {
            if summary.action_index >= self.actions.len() || summary.samples == 0 {
                return Err("trace summary is empty or references a missing action".into());
            }
            if summary.p50_ns > summary.p95_ns
                || summary.p95_ns > summary.p99_ns
                || summary.p99_ns > summary.max_ns
            {
                return Err("trace summary percentiles must be monotonically ordered".into());
            }
        }
        if let Some(finding) = &self.finding
            && (finding.action_index >= self.actions.len()
                || finding.fingerprint.is_empty()
                || finding.confirmed_runs == 0)
        {
            return Err("trace finding is incomplete or references a missing action".into());
        }
        for state in &self.worst_states {
            if state.action_index >= self.actions.len()
                || state.png.is_empty()
                || state.manifest.is_empty()
            {
                return Err("trace worst-state evidence is incomplete".into());
            }
        }
        if let Some(reduction) = &self.reduction {
            if reduction.original_len != self.actions.len()
                || reduction.minimized_actions.len() >= reduction.original_len
            {
                return Err(
                    "trace reduction must retain the original length and a strictly smaller sequence"
                        .into(),
                );
            }
            for (index, action) in reduction.minimized_actions.iter().enumerate() {
                if action.index != index {
                    return Err(format!(
                        "minimized trace action {index} has mismatched index {}",
                        action.index
                    ));
                }
                validate_source(
                    &action.source,
                    format_args!("minimized action {index} source"),
                )?;
                if let Some(source) = &action.target_source {
                    validate_source(
                        source,
                        format_args!("minimized action {index} target source"),
                    )?;
                }
            }
            if reduction.attempts.iter().any(|attempt| {
                attempt.candidate_len == 0 || attempt.candidate_len >= reduction.original_len
            }) {
                return Err("trace reduction attempts must be non-empty strict subsets".into());
            }
        }
        Ok(())
    }
}

fn validate_source(source: &SourceLocation, label: std::fmt::Arguments<'_>) -> Result<(), String> {
    if source.path.is_empty()
        || source.line == 0
        || source.column == 0
        || source.statement.is_empty()
    {
        Err(format!(
            "{label} must contain a path, statement, line, and column"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
    use std::alloc::System;

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

    #[test]
    fn trace_schema_round_trips_and_rejects_unknown_fields() {
        let artifact = artifact();
        let encoded = serde_json::to_string(&artifact).unwrap();
        let decoded: Artifact = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, artifact);
        decoded.validate().unwrap();

        let malformed = encoded.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"timings_in_capture_v2\":true",
            1,
        );
        assert!(serde_json::from_str::<Artifact>(&malformed).is_err());
    }

    #[test]
    fn trace_schema_requires_a_strictly_smaller_reduction() {
        let mut artifact = artifact();
        artifact.reduction = Some(Reduction {
            original_len: 1,
            minimized_actions: artifact.actions.clone(),
            attempts: Vec::new(),
        });
        assert!(
            artifact
                .validate()
                .unwrap_err()
                .contains("strictly smaller")
        );
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn valid_action_sources_do_not_allocate_during_validation() {
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
}
