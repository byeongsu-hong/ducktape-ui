use crate::evidence::{
    CAPTURE_DIFF_ARTIFACT_KIND, REVIEW_SCHEMA_VERSION, validate_capture_manifest,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use ui_lang_template::trace::{
    ARTIFACT_KIND as TRACE_ARTIFACT_KIND, Action as TraceAction, Artifact as TraceArtifact,
    Configuration as TraceConfiguration, Finding as TraceFinding, FindingKind, Mode as TraceMode,
    Phase as TracePhase, Sample as TraceSample, Summary as TraceSummary,
    WorstState as TraceWorstState,
};

const INSPECT_TEST: &str = "__ice_agent_inspect";
const MAX_DIFF_PIXELS: usize = 16_777_216;
/// Manifest subtrees a visual diff never reports: artifact identity, and the
/// frame timings `--frames` records, which differ on every run by design.
const IGNORED_MANIFEST_PATHS: &[&str] = &["/name", "/png", "/capture_source/statement", "/frames"];

#[derive(Debug, Default, PartialEq)]
struct InspectOptions {
    source: PathBuf,
    package: Option<String>,
    output: Option<PathBuf>,
    name: Option<String>,
    viewport: Option<(f32, f32)>,
    preset: Option<String>,
    theme: Option<String>,
    system_theme: Option<String>,
    scale: Option<f32>,
    locale: Option<String>,
    platform: Option<String>,
    reduced_motion: bool,
    test: Option<String>,
    frames: Option<usize>,
    release: bool,
    trace: bool,
    warmup: Option<usize>,
    repeat: Option<usize>,
    fuzz: Option<String>,
    seed: Option<u64>,
    steps: Option<usize>,
    replay: Option<PathBuf>,
    confirmations: Option<usize>,
    deadline_ms: Option<f64>,
    max_to_median_ratio: Option<f64>,
}

pub(super) fn inspect(root: &Path, args: &[String]) -> Result<(), String> {
    let options = parse_inspect(args)?;
    let source = root
        .join(&options.source)
        .canonicalize()
        .map_err(|error| format!("cannot open {}: {error}", options.source.display()))?;
    let source_text = fs::read_to_string(&source).map_err(|error| error.to_string())?;
    if !ui_lang_core::source_is_app(&source_text) {
        return Err(format!(
            "{} is not an Ice root; inspect the file containing its top-level `app` or `daemon`",
            source.display()
        ));
    }
    ui_lang_core::compile_file(&source)
        .map_err(|error| error.render(&source.display().to_string()))?;

    let name = options.name.as_deref().unwrap_or("inspection");
    validate_capture_name(name)?;
    let artifact_dir = options
        .output
        .as_ref()
        .map(|path| root.join(path))
        .unwrap_or_else(|| {
            root.join("target/ice-inspect")
                .join(source_output_name(root, &source))
        });
    fs::create_dir_all(&artifact_dir).map_err(|error| {
        format!(
            "cannot create inspection output {}: {error}",
            artifact_dir.display()
        )
    })?;
    let artifact_dir = artifact_dir
        .canonicalize()
        .unwrap_or_else(|_| artifact_dir.clone());
    let request_dir = root.join("target/ice-inspect");
    fs::create_dir_all(&request_dir).map_err(|error| error.to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let result_path = request_dir.join(format!(".request-{}-{nonce}.json", std::process::id()));

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let package = options
        .package
        .clone()
        .map(Ok)
        .unwrap_or_else(|| containing_package(root, &source, &cargo))?;
    if let Some(mode) = trace_mode(&options) {
        return inspect_trace(
            root,
            &source,
            &package,
            &artifact_dir,
            &result_path,
            &cargo,
            &options,
            mode,
        );
    }
    let mut command = inspect_command(
        &cargo,
        root,
        &package,
        &source,
        name,
        &artifact_dir,
        &result_path,
        &options,
    );

    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        let _ = fs::remove_file(&result_path);
        return Err(format!(
            "headless inspection failed for {}\nstdout:\n{}\nstderr:\n{}",
            source.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let result_bytes = fs::read(&result_path).map_err(|error| {
        format!(
            "no generated test matched {}; ensure the root is included with `ui_lang::include_app!` ({error})",
            source.display()
        )
    })?;
    let _ = fs::remove_file(&result_path);
    let result: Value = serde_json::from_slice(&result_bytes).map_err(|error| error.to_string())?;
    let png = result["png"]
        .as_str()
        .ok_or_else(|| "inspection result omitted `png`".to_owned())?;
    let manifest = result["manifest"]
        .as_str()
        .ok_or_else(|| "inspection result omitted `manifest`".to_owned())?;
    if !Path::new(png).is_file() || !Path::new(manifest).is_file() {
        return Err("inspection completed without both PNG and JSON artifacts".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "source": source,
            "png": png,
            "manifest": manifest,
        }))
        .expect("inspection output is serializable")
    );
    if let Some(frames) = result.get("frames") {
        println!("{}", frames_summary(frames));
    }
    Ok(())
}

/// The `cargo test` invocation a plain inspection runs.
#[allow(clippy::too_many_arguments)]
fn inspect_command(
    cargo: &str,
    root: &Path,
    package: &str,
    source: &Path,
    name: &str,
    artifact_dir: &Path,
    result_path: &Path,
    options: &InspectOptions,
) -> Command {
    let mut command = Command::new(cargo);
    command
        .current_dir(root)
        .args(["test", "--package", package]);
    if options.release {
        command.arg("--release");
    }
    command
        .args([INSPECT_TEST, "--", "--nocapture"])
        .env("ICE_AGENT_INSPECT_SOURCE", source)
        .env("ICE_AGENT_INSPECT_ROOT", root)
        .env("ICE_AGENT_INSPECT_NAME", name)
        .env("ICE_AGENT_INSPECT_ARTIFACT_DIR", artifact_dir)
        .env("ICE_AGENT_INSPECT_RESULT", result_path);
    set_inspect_environment(&mut command, options);
    command
}

/// The one stdout line `--frames` adds after the inspection result.
fn frames_summary(frames: &Value) -> String {
    let phase = |name: &str| {
        let percentiles = &frames[format!("{name}_us")];
        format!(
            "{name} p50 {}us p95 {}us",
            percentiles["p50"], percentiles["p95"]
        )
    };
    let memo = |name: &str| {
        let counts = &frames[name];
        format!("{name} {}/{}", counts["hits"], counts["misses"])
    };
    format!(
        "frames: {} @ {} | {} | {} | {} | {} | {}",
        frames["count"],
        frames["build_profile"].as_str().unwrap_or_default(),
        phase("view"),
        phase("layout"),
        phase("update"),
        memo("rev_memo"),
        memo("memo_lazy"),
    )
}

#[allow(clippy::too_many_arguments)]
fn inspect_trace(
    root: &Path,
    source: &Path,
    package: &str,
    artifact_dir: &Path,
    result_path: &Path,
    cargo: &str,
    options: &InspectOptions,
    mode: TraceMode,
) -> Result<(), String> {
    match mode {
        TraceMode::Authored => inspect_authored_trace(
            root,
            source,
            package,
            artifact_dir,
            result_path,
            cargo,
            options,
        ),
        TraceMode::Fuzz | TraceMode::Replay => inspect_campaign_trace(
            root,
            source,
            package,
            artifact_dir,
            result_path,
            cargo,
            options,
            mode,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect_authored_trace(
    root: &Path,
    source: &Path,
    package: &str,
    artifact_dir: &Path,
    result_path: &Path,
    cargo: &str,
    options: &InspectOptions,
) -> Result<(), String> {
    let test = options
        .test
        .as_ref()
        .expect("authored trace test validated");
    let analysis = ui_lang_core::analyze_file_graph(source)
        .map_err(|error| error.render(&source.display().to_string()))?;
    crate::review::selected_tests(
        &analysis.document.source_document().tests,
        std::slice::from_ref(test),
    )?;
    let exact = format!("__ice_tests::{test}");
    for _ in 0..options.warmup.unwrap_or(0) {
        let output = run_ice_test(root, source, package, cargo, &exact, options, None, &[])?;
        if !output.status.success() {
            break;
        }
    }

    let requested_repeat = options.repeat.unwrap_or(1);
    let mut traces = Vec::with_capacity(requested_repeat.max(2));
    let mut failures = Vec::new();
    let mut any_failed = false;
    let mut run = 0;
    while run < requested_repeat || (any_failed && run < 2) {
        let path = result_path.with_extension(format!("trace-{run}.json"));
        let output = run_ice_test(
            root,
            source,
            package,
            cargo,
            &exact,
            options,
            Some((&path, "authored")),
            &[],
        )?;
        let trace = read_trace(&path)?;
        let _ = fs::remove_file(&path);
        if !output.status.success() {
            any_failed = true;
            if let Some(failure) = authored_failure(&trace, &output) {
                failures.push(failure);
            }
        }
        traces.push(trace);
        run += 1;
    }
    let repeat = traces.len();
    let mut artifact = aggregate_authored(
        traces,
        test,
        options.warmup.unwrap_or(0),
        repeat,
        options.deadline_ms,
        options.max_to_median_ratio,
    )?;
    if let Some(candidate) = failures.first() {
        let confirmed_runs = failures
            .iter()
            .filter(|failure| failure.fingerprint == candidate.fingerprint)
            .count();
        if confirmed_runs >= 2 {
            artifact.finding = Some(TraceFinding {
                confirmed_runs,
                ..candidate.clone()
            });
        }
    }
    let capture_target = artifact.finding.as_ref().map_or_else(
        || {
            artifact
                .summaries
                .iter()
                .filter(|summary| summary.phase == TracePhase::Action)
                .max_by_key(|summary| summary.max_ns)
                .map(|summary| (summary.action_index, summary.phase, summary.max_ns))
        },
        |finding| {
            Some((
                finding.action_index,
                finding.phase.unwrap_or(TracePhase::Action),
                artifact
                    .summaries
                    .iter()
                    .find(|summary| {
                        summary.action_index == finding.action_index
                            && Some(summary.phase) == finding.phase
                    })
                    .map_or(0, |summary| summary.max_ns),
            ))
        },
    );
    if let Some((action_index, phase, duration_ns)) = capture_target
        && !artifact.actions.is_empty()
    {
        let capture_result = result_path.with_extension("capture.json");
        let capture_flag = if artifact.finding.as_ref().is_some_and(|finding| {
            matches!(finding.kind, FindingKind::Panic | FindingKind::Timeout)
        }) {
            "ICE_TRACE_CAPTURE_BEFORE_ACTION"
        } else {
            "ICE_TRACE_CAPTURE_ACTION"
        };
        let output = run_ice_test(
            root,
            source,
            package,
            cargo,
            &exact,
            options,
            None,
            &[
                (capture_flag, action_index.to_string()),
                (
                    "ICE_TRACE_CAPTURE_RESULT",
                    capture_result.to_string_lossy().into_owned(),
                ),
            ],
        )?;
        if !any_failed {
            require_test_success(source, test, &output)?;
        }
        let result: Value =
            serde_json::from_slice(&fs::read(&capture_result).map_err(|error| {
                format!(
                    "authored trace replay omitted worst-state capture {}: {error}",
                    capture_result.display()
                )
            })?)
            .map_err(|error| format!("invalid worst-state capture result: {error}"))?;
        let _ = fs::remove_file(&capture_result);
        artifact.worst_states.push(TraceWorstState {
            action_index,
            phase,
            duration_ns,
            png: result["png"]
                .as_str()
                .ok_or_else(|| "worst-state capture omitted PNG path".to_owned())?
                .to_owned(),
            manifest: result["manifest"]
                .as_str()
                .ok_or_else(|| "worst-state capture omitted manifest path".to_owned())?
                .to_owned(),
        });
    }
    write_trace(artifact_dir, &artifact)?;
    print_trace_summary(artifact_dir, &artifact);
    if !any_failed {
        Ok(())
    } else {
        Err(format!(
            "release trace of Ice test `{test}` failed for {}; evidence: {}",
            source.display(),
            artifact_dir.join("trace.json").display()
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect_campaign_trace(
    root: &Path,
    source: &Path,
    package: &str,
    artifact_dir: &Path,
    result_path: &Path,
    cargo: &str,
    options: &InspectOptions,
    mode: TraceMode,
) -> Result<(), String> {
    let replay = options
        .replay
        .as_ref()
        .map(|path| root.join(path))
        .map(|path| read_trace(&path).map(|trace| (path, trace)))
        .transpose()?;
    if let Some((_, trace)) = &replay {
        let relative_source = source
            .strip_prefix(root)
            .unwrap_or(source)
            .to_string_lossy()
            .replace('\\', "/");
        if trace.app_root != relative_source || trace.package != package {
            return Err(format!(
                "replay artifact targets {} in package {}, not {} in package {package}",
                trace.app_root,
                trace.package,
                source.display()
            ));
        }
    }
    let mut extra = vec![(
        "ICE_TRACE_CONFIRMATIONS",
        options.confirmations.unwrap_or(2).to_string(),
    )];
    if let Some(deadline) = options.deadline_ms {
        extra.push(("ICE_TRACE_DEADLINE_MS", deadline.to_string()));
    }
    if let Some(ratio) = options.max_to_median_ratio {
        extra.push(("ICE_TRACE_MAX_TO_MEDIAN", ratio.to_string()));
    }
    match mode {
        TraceMode::Fuzz => {
            extra.extend([
                (
                    "ICE_TRACE_SEED",
                    options.seed.expect("seed validated").to_string(),
                ),
                (
                    "ICE_TRACE_STEPS",
                    options.steps.expect("steps validated").to_string(),
                ),
            ]);
        }
        TraceMode::Replay => {
            extra.push((
                "ICE_TRACE_REPLAY",
                replay
                    .as_ref()
                    .expect("replay artifact loaded")
                    .0
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        TraceMode::Authored => unreachable!(),
    }
    let mode_name = match mode {
        TraceMode::Fuzz => "fuzz",
        TraceMode::Replay => "replay",
        TraceMode::Authored => unreachable!(),
    };
    let output = run_inspect_test(
        root,
        source,
        package,
        cargo,
        options,
        Some((result_path, mode_name)),
        &extra,
        replay.as_ref().map(|(_, trace)| trace),
    )?;
    if !output.status.success() {
        let _ = fs::remove_file(result_path);
        return Err(format!(
            "headless {mode_name} trace failed for {}\nstdout:\n{}\nstderr:\n{}",
            source.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let result: Value = serde_json::from_slice(
        &fs::read(result_path)
            .map_err(|error| format!("headless {mode_name} trace omitted its result: {error}"))?,
    )
    .map_err(|error| format!("invalid headless {mode_name} result: {error}"))?;
    let _ = fs::remove_file(result_path);
    let trace_path = result["trace"]
        .as_str()
        .ok_or_else(|| format!("headless {mode_name} result omitted `trace`"))?;
    let artifact = read_trace(Path::new(trace_path))?;
    print_trace_summary(artifact_dir, &artifact);
    Ok(())
}

fn aggregate_authored(
    traces: Vec<TraceArtifact>,
    test: &str,
    warmup: usize,
    repeat: usize,
    deadline_ms: Option<f64>,
    max_to_median_ratio: Option<f64>,
) -> Result<TraceArtifact, String> {
    let mut traces = traces.into_iter();
    let mut artifact = traces
        .next()
        .ok_or_else(|| "authored trace produced no runs".to_owned())?;
    for (run, trace) in traces.enumerate() {
        if trace.actions != artifact.actions || trace.environment != artifact.environment {
            return Err(format!(
                "authored trace run {} changed its action sequence or environment",
                run + 2
            ));
        }
        artifact
            .samples
            .extend(trace.samples.into_iter().map(|mut sample| {
                sample.run = run + 1;
                sample
            }));
    }
    artifact.configuration = TraceConfiguration {
        mode: TraceMode::Authored,
        test: Some(test.to_owned()),
        warmup,
        repeat,
        steps: Some(artifact.actions.len()),
        confirmations: 1,
        deadline_ms,
        max_to_median_ratio,
        generator_version: None,
    };
    artifact.summaries = trace_summaries(&artifact.samples);
    artifact.finding = authored_latency_finding(&artifact);
    artifact.worst_states.clear();
    artifact.validate()?;
    Ok(artifact)
}

fn trace_summaries(samples: &[TraceSample]) -> Vec<TraceSummary> {
    let mut groups = BTreeMap::<(usize, TracePhase), Vec<u64>>::new();
    for sample in samples {
        groups
            .entry((sample.action_index, sample.phase))
            .or_default()
            .push(sample.duration_ns);
    }
    groups
        .into_iter()
        .map(|((action_index, phase), mut values)| {
            values.sort_unstable();
            let percentile =
                |rank: usize| values[(values.len() * rank).div_ceil(100).saturating_sub(1)];
            TraceSummary {
                action_index,
                phase,
                samples: values.len(),
                p50_ns: percentile(50),
                p95_ns: percentile(95),
                p99_ns: percentile(99),
                max_ns: *values.last().expect("sample group is non-empty"),
                deadline_misses_60hz: values
                    .iter()
                    .filter(|value| **value > 1_000_000_000 / 60)
                    .count(),
                deadline_misses_120hz: values
                    .iter()
                    .filter(|value| **value > 1_000_000_000 / 120)
                    .count(),
            }
        })
        .collect()
}

fn authored_latency_finding(artifact: &TraceArtifact) -> Option<TraceFinding> {
    let deadline = artifact
        .configuration
        .deadline_ms
        .map(|value| (value * 1_000_000.0) as u64);
    let ratio = artifact.configuration.max_to_median_ratio;
    artifact
        .summaries
        .iter()
        .filter(|summary| summary.phase == TracePhase::Action && summary.samples >= 2)
        .filter(|summary| {
            deadline.is_some_and(|deadline| summary.max_ns > deadline)
                || ratio.is_some_and(|ratio| {
                    summary.samples >= 3 && summary.max_ns as f64 > summary.p50_ns as f64 * ratio
                })
        })
        .max_by_key(|summary| summary.max_ns)
        .map(|summary| TraceFinding {
            kind: FindingKind::Latency,
            fingerprint: trace_fingerprint(
                FindingKind::Latency,
                &artifact.actions[summary.action_index],
                summary.phase,
            ),
            action_index: summary.action_index,
            phase: Some(summary.phase),
            message: format!("authored action latency outlier: {}ns", summary.max_ns),
            confirmed_runs: summary.samples,
        })
}

fn trace_fingerprint(kind: FindingKind, action: &TraceAction, phase: TracePhase) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in format!(
        "{kind:?}\0{}\0{}\0{phase:?}",
        action.kind,
        action.target.as_deref().unwrap_or("")
    )
    .bytes()
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn write_trace(directory: &Path, artifact: &TraceArtifact) -> Result<PathBuf, String> {
    artifact.validate()?;
    let path = directory.join("trace.json");
    let mut bytes = serde_json::to_vec_pretty(artifact).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&path, bytes)
        .map_err(|error| format!("cannot write trace artifact {}: {error}", path.display()))?;
    Ok(path)
}

pub(super) fn read_trace(path: &Path) -> Result<TraceArtifact, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read trace artifact {}: {error}", path.display()))?;
    let artifact: TraceArtifact = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not a strict trace artifact: {error}", path.display()))?;
    artifact
        .validate()
        .map_err(|error| format!("invalid trace artifact {}: {error}", path.display()))?;
    Ok(artifact)
}

fn print_trace_summary(directory: &Path, artifact: &TraceArtifact) {
    let worst = artifact
        .summaries
        .iter()
        .max_by_key(|summary| summary.max_ns);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "artifact_kind": TRACE_ARTIFACT_KIND,
            "trace": directory.join("trace.json"),
            "actions": artifact.actions.len(),
            "raw_samples": artifact.samples.len(),
            "worst": worst.map(|summary| json!({
                "action_index": summary.action_index,
                "action": artifact.actions[summary.action_index].kind,
                "target": artifact.actions[summary.action_index].target,
                "phase": summary.phase,
                "max_ns": summary.max_ns,
                "source": artifact.actions[summary.action_index].source,
            })),
            "finding": artifact.finding,
            "minimized_actions": artifact.reduction.as_ref().map(|reduction| reduction.minimized_actions.len()),
            "worst_states": artifact.worst_states,
        }))
        .expect("trace summary is serializable")
    );
}

#[allow(clippy::too_many_arguments)]
fn run_ice_test(
    root: &Path,
    source: &Path,
    package: &str,
    cargo: &str,
    exact: &str,
    options: &InspectOptions,
    trace: Option<(&Path, &str)>,
    extra: &[(&str, String)],
) -> Result<std::process::Output, String> {
    let mut command = Command::new(cargo);
    command.current_dir(root).args([
        "test",
        "--release",
        "--package",
        package,
        exact,
        "--",
        "--exact",
        "--nocapture",
    ]);
    apply_trace_environment(
        &mut command,
        root,
        source,
        package,
        options,
        trace,
        extra,
        None,
    );
    command.output().map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn run_inspect_test(
    root: &Path,
    source: &Path,
    package: &str,
    cargo: &str,
    options: &InspectOptions,
    trace: Option<(&Path, &str)>,
    extra: &[(&str, String)],
    replay: Option<&TraceArtifact>,
) -> Result<std::process::Output, String> {
    let mut command = Command::new(cargo);
    command.current_dir(root).args([
        "test",
        "--release",
        "--package",
        package,
        INSPECT_TEST,
        "--",
        "--nocapture",
    ]);
    apply_trace_environment(
        &mut command,
        root,
        source,
        package,
        options,
        trace,
        extra,
        replay,
    );
    command.output().map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn apply_trace_environment(
    command: &mut Command,
    root: &Path,
    source: &Path,
    package: &str,
    options: &InspectOptions,
    trace: Option<(&Path, &str)>,
    extra: &[(&str, String)],
    replay: Option<&TraceArtifact>,
) {
    let artifact_dir = options.output.as_ref().map_or_else(
        || {
            root.join("target/ice-inspect")
                .join(source_output_name(root, source))
        },
        |path| root.join(path),
    );
    command
        .env("ICE_AGENT_INSPECT_SOURCE", source)
        .env("ICE_AGENT_INSPECT_ROOT", root)
        .env(
            "ICE_AGENT_INSPECT_NAME",
            options.name.as_deref().unwrap_or("inspection"),
        )
        .env("ICE_AGENT_INSPECT_ARTIFACT_DIR", &artifact_dir)
        .env("ICE_TEST_ARTIFACT_DIR", &artifact_dir)
        .env("ICE_TRACE_APP_ROOT", source)
        .env("ICE_TRACE_PACKAGE", package);
    if let Some((path, mode)) = trace {
        command.env("ICE_TRACE_MODE", mode).env(
            if mode == "authored" {
                "ICE_TRACE_RESULT"
            } else {
                "ICE_AGENT_INSPECT_RESULT"
            },
            path,
        );
    }
    if let Some(replay) = replay {
        set_replay_environment(command, replay);
    } else {
        set_inspect_environment(command, options);
    }
    for (name, value) in extra {
        command.env(name, value);
    }
}

fn set_inspect_environment(command: &mut Command, options: &InspectOptions) {
    set_optional_env(command, "ICE_AGENT_INSPECT_PRESET", &options.preset);
    set_optional_env(command, "ICE_AGENT_INSPECT_THEME", &options.theme);
    set_optional_env(
        command,
        "ICE_AGENT_INSPECT_SYSTEM_THEME",
        &options.system_theme,
    );
    set_optional_env(command, "ICE_AGENT_INSPECT_LOCALE", &options.locale);
    set_optional_env(command, "ICE_AGENT_INSPECT_PLATFORM", &options.platform);
    if let Some((width, height)) = options.viewport {
        command
            .env("ICE_AGENT_INSPECT_WIDTH", width.to_string())
            .env("ICE_AGENT_INSPECT_HEIGHT", height.to_string());
    }
    if let Some(scale) = options.scale {
        command.env("ICE_AGENT_INSPECT_SCALE", scale.to_string());
    }
    if options.reduced_motion {
        command.env("ICE_AGENT_INSPECT_REDUCED_MOTION", "true");
    }
    if let Some(frames) = options.frames {
        command.env("ICE_AGENT_INSPECT_FRAMES", frames.to_string());
    }
}

fn set_replay_environment(command: &mut Command, replay: &TraceArtifact) {
    let environment = &replay.environment;
    command
        .env(
            "ICE_AGENT_INSPECT_WIDTH",
            environment.viewport_width.to_string(),
        )
        .env(
            "ICE_AGENT_INSPECT_HEIGHT",
            environment.viewport_height.to_string(),
        )
        .env(
            "ICE_AGENT_INSPECT_SCALE",
            environment.scale_factor.to_string(),
        )
        .env("ICE_AGENT_INSPECT_SYSTEM_THEME", &environment.system_theme)
        .env("ICE_AGENT_INSPECT_PLATFORM", &environment.platform);
    if let Some(value) = &environment.preset {
        command.env("ICE_AGENT_INSPECT_PRESET", value);
    }
    if let Some(value) = &environment.theme {
        command.env("ICE_AGENT_INSPECT_THEME", value);
    }
    if let Some(value) = &environment.locale {
        command.env("ICE_AGENT_INSPECT_LOCALE", value);
    }
    if let Some(value) = environment.reduced_motion {
        command.env("ICE_AGENT_INSPECT_REDUCED_MOTION", value.to_string());
    }
}

fn require_test_success(
    source: &Path,
    test: &str,
    output: &std::process::Output,
) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "release trace of Ice test `{test}` failed for {}\nstdout:\n{}\nstderr:\n{}",
            source.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

fn authored_failure(trace: &TraceArtifact, output: &std::process::Output) -> Option<TraceFinding> {
    let action_index = trace.actions.len().checked_sub(1)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = format!("{stdout}\n{stderr}");
    let kind = if message.contains("expectation failed") {
        FindingKind::Assertion
    } else if message.contains("quiescence within") {
        FindingKind::Timeout
    } else {
        FindingKind::Panic
    };
    Some(TraceFinding {
        kind,
        fingerprint: trace_fingerprint(kind, &trace.actions[action_index], TracePhase::Action),
        action_index,
        phase: Some(TracePhase::Action),
        message: message
            .lines()
            .find(|line| {
                line.contains("expectation failed")
                    || line.contains("quiescence within")
                    || line.contains("panicked at")
            })
            .or_else(|| message.lines().rev().find(|line| !line.trim().is_empty()))
            .unwrap_or("authored Ice test failed")
            .to_owned(),
        confirmed_runs: 1,
    })
}

fn set_optional_env(command: &mut Command, name: &str, value: &Option<String>) {
    if let Some(value) = value {
        command.env(name, value);
    }
}

fn parse_inspect(args: &[String]) -> Result<InspectOptions, String> {
    let Some(source) = args.first() else {
        return Err("cargo ice inspect <file.ice> [options]".into());
    };
    if source.starts_with('-') {
        return Err("cargo ice inspect requires a root .ice file first".into());
    }
    let mut options = InspectOptions {
        source: source.into(),
        ..InspectOptions::default()
    };
    let mut index = 1;
    while index < args.len() {
        let flag = args[index].as_str();
        if let Some(slot) = match flag {
            "--reduced-motion" => Some(&mut options.reduced_motion),
            "--release" => Some(&mut options.release),
            "--trace" => Some(&mut options.trace),
            _ => None,
        } {
            if *slot {
                return Err(format!("duplicate `{flag}`"));
            }
            *slot = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?
            .clone();
        match flag {
            "--output" => set_once(&mut options.output, PathBuf::from(value), flag)?,
            "--package" => set_once(&mut options.package, value, flag)?,
            "--name" => set_once(&mut options.name, value, flag)?,
            "--preset" => set_once(&mut options.preset, value, flag)?,
            "--test" => set_once(&mut options.test, value, flag)?,
            "--fuzz" => {
                validate_choice(flag, &value, &["interactions"])?;
                set_once(&mut options.fuzz, value, flag)?;
            }
            "--seed" => {
                let value = value
                    .parse::<u64>()
                    .map_err(|error| format!("invalid {flag} value {value:?}: {error}"))?;
                set_once(&mut options.seed, value, flag)?;
            }
            "--steps" => {
                let value = positive_usize(flag, &value)?;
                set_once(&mut options.steps, value, flag)?;
            }
            "--frames" => {
                let value = positive_usize(flag, &value)?;
                set_once(&mut options.frames, value, flag)?;
            }
            "--warmup" => {
                let value = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid {flag} value {value:?}: {error}"))?;
                set_once(&mut options.warmup, value, flag)?;
            }
            "--repeat" => {
                let value = positive_usize(flag, &value)?;
                set_once(&mut options.repeat, value, flag)?;
            }
            "--replay" => set_once(&mut options.replay, PathBuf::from(value), flag)?,
            "--confirm" => {
                let value = positive_usize(flag, &value)?;
                set_once(&mut options.confirmations, value, flag)?;
            }
            "--deadline-ms" => {
                let value = positive_f64(flag, &value)?;
                set_once(&mut options.deadline_ms, value, flag)?;
            }
            "--max-to-median" => {
                let value = positive_f64(flag, &value)?;
                if value <= 1.0 {
                    return Err("--max-to-median requires a value greater than 1".into());
                }
                set_once(&mut options.max_to_median_ratio, value, flag)?;
            }
            "--theme" => {
                validate_choice(flag, &value, &["none", "light", "dark"])?;
                set_once(&mut options.theme, value, flag)?;
            }
            "--system-theme" => {
                validate_choice(flag, &value, &["none", "light", "dark"])?;
                set_once(&mut options.system_theme, value, flag)?;
            }
            "--locale" => set_once(&mut options.locale, value, flag)?,
            "--platform" => {
                validate_choice(flag, &value, &["linux", "windows", "macos", "wasm"])?;
                set_once(&mut options.platform, value, flag)?;
            }
            "--scale" => {
                let value = positive_f32(flag, &value)?;
                set_once(&mut options.scale, value, flag)?;
            }
            "--viewport" => {
                let (width, height) = value
                    .split_once('x')
                    .ok_or_else(|| "--viewport expects WIDTHxHEIGHT".to_owned())?;
                let value = (positive_f32(flag, width)?, positive_f32(flag, height)?);
                set_once(&mut options.viewport, value, flag)?;
            }
            _ => return Err(format!("unknown inspect option `{flag}`")),
        }
        index += 2;
    }
    validate_inspect_options(&options)?;
    Ok(options)
}

fn trace_mode(options: &InspectOptions) -> Option<TraceMode> {
    if options.replay.is_some() {
        Some(TraceMode::Replay)
    } else if options.fuzz.is_some() {
        Some(TraceMode::Fuzz)
    } else {
        options.trace.then_some(TraceMode::Authored)
    }
}

fn validate_inspect_options(options: &InspectOptions) -> Result<(), String> {
    let requested_modes = usize::from(options.trace)
        + usize::from(options.fuzz.is_some())
        + usize::from(options.replay.is_some());
    if requested_modes > 1 {
        return Err("choose exactly one of `--trace`, `--fuzz`, or `--replay`".into());
    }
    if options.frames.is_some() && requested_modes > 0 {
        return Err(
            "--frames measures the plain inspection; use --trace for interaction timings".into(),
        );
    }
    if options.release && options.frames.is_none() {
        return Err("--release requires --frames".into());
    }
    match trace_mode(options) {
        None => {
            if options.test.is_some()
                || options.warmup.is_some()
                || options.repeat.is_some()
                || options.seed.is_some()
                || options.steps.is_some()
                || options.confirmations.is_some()
                || options.deadline_ms.is_some()
                || options.max_to_median_ratio.is_some()
            {
                return Err("trace options require `--trace`, `--fuzz`, or `--replay`".into());
            }
        }
        Some(TraceMode::Authored) => {
            if options.test.is_none() {
                return Err("--trace requires `--test NAME`".into());
            }
            if options.seed.is_some() || options.steps.is_some() || options.confirmations.is_some()
            {
                return Err(
                    "authored tracing does not accept fuzz seed, step, or confirmation options"
                        .into(),
                );
            }
        }
        Some(TraceMode::Fuzz) => {
            if options.seed.is_none() || options.steps.is_none() {
                return Err("--fuzz interactions requires both `--seed` and `--steps`".into());
            }
            if options.test.is_some() || options.warmup.is_some() || options.repeat.is_some() {
                return Err(
                    "fuzzing does not accept authored test, warmup, or repeat options".into(),
                );
            }
        }
        Some(TraceMode::Replay) => {
            if options.test.is_some()
                || options.warmup.is_some()
                || options.repeat.is_some()
                || options.seed.is_some()
                || options.steps.is_some()
            {
                return Err(
                    "replay takes its test, environment, seed, and steps from the artifact".into(),
                );
            }
        }
    }
    Ok(())
}

pub(super) fn containing_package(
    root: &Path,
    source: &Path,
    cargo: &str,
) -> Result<String, String> {
    let output = Command::new(cargo)
        .current_dir(root)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("cargo metadata failed while locating the Ice app package".into());
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata omitted packages".to_owned())?;
    packages
        .iter()
        .filter_map(|package| {
            let name = package["name"].as_str()?;
            let manifest = Path::new(package["manifest_path"].as_str()?);
            let directory = manifest.parent()?.canonicalize().ok()?;
            source
                .starts_with(&directory)
                .then_some((directory.components().count(), name.to_owned()))
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, name)| name)
        .ok_or_else(|| {
            format!(
                "no Cargo package contains {}; pass `--package <name>` for an external include",
                source.display()
            )
        })
}

pub(super) fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("duplicate `{flag}`"))
    } else {
        Ok(())
    }
}

fn positive_f32(flag: &str, value: &str) -> Result<f32, String> {
    let value = value
        .parse::<f32>()
        .map_err(|error| format!("invalid {flag} value {value:?}: {error}"))?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(format!("{flag} requires a finite positive number"))
    }
}

fn positive_f64(flag: &str, value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|error| format!("invalid {flag} value {value:?}: {error}"))?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(format!("{flag} requires a finite positive number"))
    }
}

fn positive_usize(flag: &str, value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|error| format!("invalid {flag} value {value:?}: {error}"))?;
    if value == 0 {
        Err(format!("{flag} requires a positive integer"))
    } else {
        Ok(value)
    }
}

fn validate_choice(flag: &str, value: &str, choices: &[&str]) -> Result<(), String> {
    if choices.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "{flag} expects one of {}; got {value:?}",
            choices.join(", ")
        ))
    }
}

fn validate_capture_name(name: &str) -> Result<(), String> {
    if !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        Ok(())
    } else {
        Err("--name must be non-empty snake_case".into())
    }
}

pub(super) fn source_output_name(root: &Path, source: &Path) -> String {
    let source = source.strip_prefix(root).unwrap_or(source);
    let mut name = source
        .with_extension("")
        .to_string_lossy()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if name.is_empty() {
        name.push_str("app");
    }
    name
}

#[derive(Debug)]
struct DiffOptions {
    baseline: PathBuf,
    current: PathBuf,
    output: Option<PathBuf>,
    pixel_threshold: u8,
    max_changed_ratio: f64,
    value_tolerance: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct DiffThresholds {
    pub(super) pixel: u8,
    pub(super) max_changed_ratio: f64,
    pub(super) value: f64,
}

pub(super) fn diff(root: &Path, args: &[String]) -> Result<(), String> {
    let options = parse_diff(args)?;
    let baseline_path = root.join(&options.baseline);
    let current_path = root.join(&options.current);
    let output = options.output.as_ref().map_or_else(
        || {
            root.join("target/ice-diff").join(format!(
                "{}-vs-{}",
                file_stem(&baseline_path),
                file_stem(&current_path)
            ))
        },
        |path| root.join(path),
    );
    let report = compare_capture_manifests(
        &baseline_path,
        &current_path,
        &output,
        DiffThresholds {
            pixel: options.pixel_threshold,
            max_changed_ratio: options.max_changed_ratio,
            value: options.value_tolerance,
        },
    )?;
    let matches = report["matches"]
        .as_bool()
        .expect("capture diff report has a boolean result");
    let report_path = output.join("report.json");
    let diff_png = output.join("diff.png");
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "matches": matches,
            "report": report_path,
            "diff_png": diff_png,
            "manifest_differences": report["manifest"]["difference_count"],
            "changed_ratio": report["pixels"]["changed_ratio"],
        }))
        .expect("diff output is serializable")
    );
    if matches {
        Ok(())
    } else {
        Err(format!("inspection differs; see {}", report_path.display()))
    }
}

pub(super) fn compare_capture_manifests(
    baseline_path: &Path,
    current_path: &Path,
    output: &Path,
    thresholds: DiffThresholds,
) -> Result<Value, String> {
    let baseline = read_json(baseline_path)?;
    let current = read_json(current_path)?;
    let baseline_png = validate_capture_manifest(baseline_path, &baseline)?.png;
    let current_png = validate_capture_manifest(current_path, &current)?.png;
    let baseline_image = read_png(&baseline_png)?;
    let current_image = read_png(&current_png)?;
    fs::create_dir_all(output).map_err(|error| error.to_string())?;
    let output = output.canonicalize().unwrap_or_else(|_| output.to_owned());
    let diff_png = output.join("diff.png");
    let pixels = compare_pixels(&baseline_image, &current_image, thresholds.pixel)?;
    write_png(&diff_png, pixels.width, pixels.height, &pixels.rgba)?;

    let mut differences = Vec::new();
    compare_json("", &baseline, &current, thresholds.value, &mut differences);
    differences.retain(|difference| {
        difference["path"]
            .as_str()
            .is_none_or(|path| !ignored_manifest_path(path))
    });
    let changed_ratio = if pixels.total == 0 {
        0.0
    } else {
        pixels.changed as f64 / pixels.total as f64
    };
    let matches = differences.is_empty() && changed_ratio <= thresholds.max_changed_ratio;
    let report_path = output.join("report.json");
    let report = json!({
        "artifact_kind": CAPTURE_DIFF_ARTIFACT_KIND,
        "schema_version": REVIEW_SCHEMA_VERSION,
        "matches": matches,
        "baseline": { "manifest": baseline_path, "png": baseline_png },
        "current": { "manifest": current_path, "png": current_png },
        "manifest": {
            "value_tolerance": thresholds.value,
            "ignored_paths": IGNORED_MANIFEST_PATHS,
            "difference_count": differences.len(),
            "differences": differences,
        },
        "pixels": {
            "threshold": thresholds.pixel,
            "max_changed_ratio": thresholds.max_changed_ratio,
            "changed": pixels.changed,
            "total": pixels.total,
            "changed_ratio": changed_ratio,
            "max_channel_delta": pixels.max_delta,
            "baseline_size": { "width": baseline_image.width, "height": baseline_image.height },
            "current_size": { "width": current_image.width, "height": current_image.height },
            "diff_png": diff_png,
        },
    });
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report).expect("diff report is serializable"),
    )
    .map_err(|error| error.to_string())?;
    Ok(report)
}

/// An ignored path covers its own value and everything below it.
fn ignored_manifest_path(path: &str) -> bool {
    IGNORED_MANIFEST_PATHS.iter().any(|ignored| {
        path.strip_prefix(ignored)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

fn parse_diff(args: &[String]) -> Result<DiffOptions, String> {
    if args.len() < 2 {
        return Err("cargo ice diff <baseline.json> <current.json> [options]".into());
    }
    let mut options = DiffOptions {
        baseline: (&args[0]).into(),
        current: (&args[1]).into(),
        output: None,
        pixel_threshold: 0,
        max_changed_ratio: 0.0,
        value_tolerance: 0.0,
    };
    let mut seen_pixel_threshold = false;
    let mut seen_ratio = false;
    let mut seen_tolerance = false;
    let mut index = 2;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag {
            "--output" => set_once(&mut options.output, value.into(), flag)?,
            "--pixel-threshold" if !seen_pixel_threshold => {
                options.pixel_threshold = value
                    .parse()
                    .map_err(|error| format!("invalid {flag} value: {error}"))?;
                seen_pixel_threshold = true;
            }
            "--max-changed-ratio" if !seen_ratio => {
                options.max_changed_ratio = unit_f64(flag, value)?;
                seen_ratio = true;
            }
            "--value-tolerance" if !seen_tolerance => {
                options.value_tolerance = nonnegative_f64(flag, value)?;
                seen_tolerance = true;
            }
            "--pixel-threshold" | "--max-changed-ratio" | "--value-tolerance" => {
                return Err(format!("duplicate `{flag}`"));
            }
            _ => return Err(format!("unknown diff option `{flag}`")),
        }
        index += 2;
    }
    Ok(options)
}

pub(super) fn unit_f64(flag: &str, value: &str) -> Result<f64, String> {
    let value = nonnegative_f64(flag, value)?;
    if value <= 1.0 {
        Ok(value)
    } else {
        Err(format!("{flag} must be between 0 and 1"))
    }
}

pub(super) fn nonnegative_f64(flag: &str, value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|error| format!("invalid {flag} value: {error}"))?;
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(format!("{flag} requires a finite non-negative number"))
    }
}

pub(super) fn read_json(path: &Path) -> Result<Value, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("capture")
        .to_owned()
}

fn compare_json(
    path: &str,
    baseline: &Value,
    current: &Value,
    tolerance: f64,
    out: &mut Vec<Value>,
) {
    compare_json_values(path, Some(baseline), Some(current), tolerance, out);
}

fn compare_json_values(
    path: &str,
    baseline: Option<&Value>,
    current: Option<&Value>,
    tolerance: f64,
    out: &mut Vec<Value>,
) {
    match (baseline, current) {
        (Some(baseline), Some(current)) if baseline == current => {}
        (Some(Value::Object(baseline)), Some(Value::Object(current))) => {
            let mut keys = baseline.keys().chain(current.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                compare_json_values(
                    &format!("{path}/{}", escape_pointer(key)),
                    baseline.get(key),
                    current.get(key),
                    tolerance,
                    out,
                );
            }
        }
        (Some(Value::Array(baseline)), Some(Value::Array(current))) => {
            for index in 0..baseline.len().max(current.len()) {
                compare_json_values(
                    &format!("{path}/{index}"),
                    baseline.get(index),
                    current.get(index),
                    tolerance,
                    out,
                );
            }
        }
        (Some(Value::Number(baseline)), Some(Value::Number(current))) => {
            let equal = baseline == current
                || baseline
                    .as_f64()
                    .zip(current.as_f64())
                    .is_some_and(|(baseline, current)| (baseline - current).abs() <= tolerance);
            if !equal {
                out.push(json!({ "path": path, "baseline": baseline, "current": current }));
            }
        }
        _ => out.push(json!({
            "path": path,
            "baseline": difference_value(baseline),
            "current": difference_value(current),
        })),
    }
}

fn difference_value(value: Option<&Value>) -> Value {
    value
        .cloned()
        .unwrap_or_else(|| json!({ "$missing": true }))
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

struct PngImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn read_png(path: &Path) -> Result<PngImage, String> {
    let file =
        fs::File::open(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let pixels = (reader.info().width as usize)
        .checked_mul(reader.info().height as usize)
        .ok_or_else(|| format!("{} has overflowing dimensions", path.display()))?;
    if pixels > MAX_DIFF_PIXELS {
        return Err(format!(
            "{} exceeds the {MAX_DIFF_PIXELS}-pixel comparison limit",
            path.display()
        ));
    }
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| format!("{} has an unsupported PNG buffer size", path.display()))?;
    let mut rgba = vec![0; size];
    let info = reader
        .next_frame(&mut rgba)
        .map_err(|error| error.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{} must be an 8-bit RGBA PNG", path.display()));
    }
    rgba.truncate(info.buffer_size());
    Ok(PngImage {
        width: info.width,
        height: info.height,
        rgba,
    })
}

struct PixelDiff {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    changed: usize,
    total: usize,
    max_delta: u8,
}

fn compare_pixels(
    baseline: &PngImage,
    current: &PngImage,
    threshold: u8,
) -> Result<PixelDiff, String> {
    let width = baseline.width.max(current.width);
    let height = baseline.height.max(current.height);
    let total = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "comparison dimensions overflow".to_owned())?;
    if total > MAX_DIFF_PIXELS {
        return Err(format!(
            "combined image bounds exceed the {MAX_DIFF_PIXELS}-pixel comparison limit"
        ));
    }
    let capacity = total
        .checked_mul(4)
        .ok_or_else(|| "comparison buffer size overflows".to_owned())?;
    let mut rgba = Vec::with_capacity(capacity);
    let mut changed = 0;
    let mut max_delta = 0;
    for y in 0..height {
        for x in 0..width {
            let baseline = pixel(baseline, x, y);
            let current = pixel(current, x, y);
            let deltas = match (baseline, current) {
                (Some(baseline), Some(current)) => [
                    baseline[0].abs_diff(current[0]),
                    baseline[1].abs_diff(current[1]),
                    baseline[2].abs_diff(current[2]),
                    baseline[3].abs_diff(current[3]),
                ],
                _ => [u8::MAX; 4],
            };
            let pixel_delta = *deltas.iter().max().expect("four channels");
            max_delta = max_delta.max(pixel_delta);
            if pixel_delta > threshold {
                changed += 1;
                rgba.extend_from_slice(&[pixel_delta.max(64), deltas[1] / 2, deltas[2] / 2, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Ok(PixelDiff {
        width,
        height,
        rgba,
        changed,
        total,
        max_delta,
    })
}

fn pixel(image: &PngImage, x: u32, y: u32) -> Option<&[u8]> {
    if x >= image.width || y >= image.height {
        return None;
    }
    let start = (y as usize * image.width as usize + x as usize) * 4;
    image.rgba.get(start..start + 4)
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = fs::File::create(path).map_err(|error| error.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    writer
        .write_image_data(rgba)
        .map_err(|error| error.to_string())?;
    writer.finish().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::CAPTURE_SCHEMA_VERSION;
    use ui_lang_template::trace::{
        Environment as TraceEnvironment, SCHEMA_VERSION as TRACE_SCHEMA_VERSION,
        SourceLocation as TraceSource,
    };

    fn capture_manifest(name: &str) -> Value {
        json!({
            "schema_version": CAPTURE_SCHEMA_VERSION,
            "name": name,
            "png": format!("{name}.png"),
            "capture_source": {
                "path": "app.ice", "line": 1, "column": 1, "statement": format!("capture {name}")
            },
            "viewport": { "width": 1.0, "height": 1.0 },
            "physical_size": { "width": 1, "height": 1 },
            "scale_factor": 1.0,
            "configured_theme": null,
            "resolved_theme": { "mode": "light", "name": "Light" },
            "system_theme": "none",
            "locale": null,
            "platform": "linux",
            "reduced_motion": null,
            "window": { "position": null, "focused": true },
            "clock": {
                "supports_virtual_redraw_advance": true,
                "iced_timer_futures_are_virtual": false
            },
            "targets": [],
        })
    }

    #[test]
    fn parses_deterministic_inspection_inputs() {
        let options = parse_inspect(&[
            "src/ui/app.ice".into(),
            "--viewport".into(),
            "390x844".into(),
            "--theme".into(),
            "dark".into(),
            "--scale".into(),
            "2".into(),
            "--reduced-motion".into(),
        ])
        .unwrap();
        assert_eq!(options.source, PathBuf::from("src/ui/app.ice"));
        assert_eq!(options.viewport, Some((390.0, 844.0)));
        assert_eq!(options.theme.as_deref(), Some("dark"));
        assert_eq!(options.scale, Some(2.0));
        assert!(options.reduced_motion);
        assert!(parse_inspect(&["app.ice".into(), "--theme".into(), "blue".into()]).is_err());
    }

    #[test]
    fn parses_frame_measurement_only_for_the_plain_inspection() {
        let options = parse_inspect(&[
            "src/ui/app.ice".into(),
            "--frames".into(),
            "60".into(),
            "--release".into(),
        ])
        .unwrap();
        assert_eq!(options.frames, Some(60));
        assert!(options.release);

        for (arguments, expected) in [
            (vec!["app.ice", "--frames", "0"], "positive integer"),
            (
                vec!["app.ice", "--frames", "5", "--trace"],
                "use --trace for interaction timings",
            ),
            (vec!["app.ice", "--release"], "--release requires --frames"),
            (
                vec!["app.ice", "--frames", "5", "--frames", "6"],
                "duplicate `--frames`",
            ),
        ] {
            let arguments = arguments.into_iter().map(String::from).collect::<Vec<_>>();
            let error = parse_inspect(&arguments).unwrap_err();
            assert!(error.contains(expected), "{arguments:?} reported {error:?}");
        }
    }

    #[test]
    fn frame_measurement_reaches_the_inspection_command() {
        fn command(frames: Option<usize>, release: bool) -> Command {
            let options = InspectOptions {
                frames,
                release,
                ..InspectOptions::default()
            };
            inspect_command(
                "cargo",
                Path::new("/root"),
                "example",
                Path::new("/root/src/ui/app.ice"),
                "inspection",
                Path::new("/root/target/ice-inspect"),
                Path::new("/root/target/ice-inspect/.request.json"),
                &options,
            )
        }
        fn release_arguments(command: &Command) -> usize {
            command
                .get_args()
                .filter(|argument| *argument == "--release")
                .count()
        }
        fn frames_environment(command: &Command) -> Option<String> {
            let (_, value) = command
                .get_envs()
                .find(|(name, _)| *name == "ICE_AGENT_INSPECT_FRAMES")?;
            Some(value?.to_string_lossy().into_owned())
        }

        let measured = command(Some(60), true);
        assert_eq!(
            release_arguments(&measured),
            1,
            "--release once: {:?}",
            measured.get_args().collect::<Vec<_>>()
        );
        assert_eq!(frames_environment(&measured).as_deref(), Some("60"));

        let plain = command(None, false);
        assert_eq!(
            release_arguments(&plain),
            0,
            "no --release: {:?}",
            plain.get_args().collect::<Vec<_>>()
        );
        assert_eq!(frames_environment(&plain), None);
    }

    #[test]
    fn frames_summary_reports_the_contract_line() {
        let frames = json!({
            "count": 60,
            "warmup": 8,
            "build_profile": "debug",
            "view_us": { "p50": 650, "p95": 720 },
            "layout_us": { "p50": 1100, "p95": 1300 },
            "update_us": { "p50": 100, "p95": 130 },
            "rev_memo": { "hits": 81, "misses": 0 },
            "memo_lazy": { "hits": 12, "misses": 0 },
        });
        assert_eq!(
            frames_summary(&frames),
            "frames: 60 @ debug | view p50 650us p95 720us | layout p50 1100us p95 1300us \
             | update p50 100us p95 130us | rev_memo 81/0 | memo_lazy 12/0"
        );
    }

    #[test]
    fn diff_ignores_measured_frame_timings() {
        fn manifest_with_frames(name: &str, view_p50: u64) -> Value {
            let mut manifest = capture_manifest(name);
            manifest["frames"] = json!({
                "count": 30,
                "warmup": 8,
                "build_profile": "debug",
                "view_us": { "p50": view_p50, "p95": view_p50 + 70 },
                "layout_us": { "p50": 1_100, "p95": 1_300 },
                "update_us": { "p50": 100, "p95": 130 },
                "rev_memo": { "hits": 81, "misses": 0 },
                "memo_lazy": { "hits": 12, "misses": 0 },
            });
            manifest
        }

        let fixture = tempfile::tempdir().unwrap();
        let baseline = fixture.path().join("baseline.json");
        let current = fixture.path().join("current.json");
        for (path, manifest) in [
            (&baseline, manifest_with_frames("baseline", 650)),
            (&current, manifest_with_frames("current", 940)),
        ] {
            fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
            write_png(&path.with_extension("png"), 1, 1, &[0, 0, 0, 255]).unwrap();
        }

        let report = compare_capture_manifests(
            &baseline,
            &current,
            &fixture.path().join("diff"),
            DiffThresholds::default(),
        )
        .unwrap();

        assert_eq!(
            report["manifest"]["difference_count"], 0,
            "frame timings are not a visual delta: {}",
            report["manifest"]["differences"]
        );
        assert_eq!(report["matches"], true);
        assert!(
            report["manifest"]["ignored_paths"]
                .as_array()
                .is_some_and(|paths| paths.iter().any(|path| path == "/frames")),
            "the diff report must name the rule it applied"
        );
    }

    #[test]
    fn parses_trace_fuzz_and_replay_modes_without_ambiguous_combinations() {
        let authored = parse_inspect(&[
            "app.ice".into(),
            "--test".into(),
            "rapid_scroll".into(),
            "--trace".into(),
            "--warmup".into(),
            "4".into(),
            "--repeat".into(),
            "60".into(),
        ])
        .unwrap();
        assert_eq!(trace_mode(&authored), Some(TraceMode::Authored));
        assert_eq!(authored.warmup, Some(4));
        assert_eq!(authored.repeat, Some(60));

        let fuzz = parse_inspect(&[
            "app.ice".into(),
            "--fuzz".into(),
            "interactions".into(),
            "--seed".into(),
            "18421".into(),
            "--steps".into(),
            "500".into(),
            "--confirm".into(),
            "3".into(),
            "--deadline-ms".into(),
            "16.667".into(),
            "--max-to-median".into(),
            "5".into(),
        ])
        .unwrap();
        assert_eq!(trace_mode(&fuzz), Some(TraceMode::Fuzz));
        assert_eq!(fuzz.seed, Some(18_421));
        assert_eq!(fuzz.steps, Some(500));
        assert_eq!(fuzz.confirmations, Some(3));

        let replay =
            parse_inspect(&["app.ice".into(), "--replay".into(), "failure.json".into()]).unwrap();
        assert_eq!(trace_mode(&replay), Some(TraceMode::Replay));
        assert!(
            parse_inspect(&[
                "app.ice".into(),
                "--trace".into(),
                "--test".into(),
                "test".into(),
                "--fuzz".into(),
                "interactions".into(),
                "--seed".into(),
                "1".into(),
                "--steps".into(),
                "1".into(),
            ])
            .unwrap_err()
            .contains("exactly one")
        );
        assert!(
            parse_inspect(&[
                "app.ice".into(),
                "--fuzz".into(),
                "interactions".into(),
                "--seed".into(),
                "1".into(),
            ])
            .unwrap_err()
            .contains("both `--seed` and `--steps`")
        );
    }

    fn trace_artifact() -> TraceArtifact {
        TraceArtifact {
            artifact_kind: TRACE_ARTIFACT_KIND.into(),
            schema_version: TRACE_SCHEMA_VERSION,
            app_root: "app.ice".into(),
            package: "fixture".into(),
            environment: TraceEnvironment {
                preset: None,
                viewport_width: 800.0,
                viewport_height: 600.0,
                theme: None,
                system_theme: "none".into(),
                scale_factor: 1.0,
                locale: None,
                platform: "linux".into(),
                reduced_motion: None,
                build_profile: "release".into(),
            },
            configuration: TraceConfiguration {
                mode: TraceMode::Authored,
                test: Some("scenario".into()),
                warmup: 0,
                repeat: 1,
                steps: Some(1),
                confirmations: 1,
                deadline_ms: None,
                max_to_median_ratio: None,
                generator_version: None,
            },
            seed: None,
            actions: vec![TraceAction {
                index: 0,
                kind: "redraw".into(),
                target: None,
                parameters: Value::Null,
                source: TraceSource {
                    path: "app.ice".into(),
                    line: 1,
                    column: 1,
                    statement: "window redraw".into(),
                },
                target_source: None,
            }],
            samples: vec![TraceSample {
                run: 0,
                action_index: 0,
                phase: TracePhase::Action,
                duration_ns: 10,
            }],
            summaries: vec![TraceSummary {
                action_index: 0,
                phase: TracePhase::Action,
                samples: 1,
                p50_ns: 10,
                p95_ns: 10,
                p99_ns: 10,
                max_ns: 10,
                deadline_misses_60hz: 0,
                deadline_misses_120hz: 0,
            }],
            unavailable_phases: vec![TracePhase::Draw],
            finding: None,
            worst_states: Vec::new(),
            reduction: None,
        }
    }

    #[test]
    fn trace_reader_rejects_unknown_fields_and_unsupported_versions() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("trace.json");
        let mut value = serde_json::to_value(trace_artifact()).unwrap();
        value["timings_in_capture_v2"] = json!(true);
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            read_trace(&path)
                .unwrap_err()
                .contains("strict trace artifact")
        );

        let mut value = serde_json::to_value(trace_artifact()).unwrap();
        value["schema_version"] = json!(TRACE_SCHEMA_VERSION + 1);
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            read_trace(&path)
                .unwrap_err()
                .contains("unsupported trace schema")
        );
    }

    #[test]
    fn reports_structured_and_pixel_differences() {
        let baseline = json!({ "targets": [{ "geometry": { "x": 10, "y": 20 } }] });
        let current = json!({ "targets": [{ "geometry": { "x": 11, "y": 22 } }] });
        let mut differences = Vec::new();
        compare_json("", &baseline, &current, 1.0, &mut differences);
        assert_eq!(
            differences,
            [json!({
                "path": "/targets/0/geometry/y",
                "baseline": 20,
                "current": 22,
            })]
        );
        let image = PngImage {
            width: 1,
            height: 1,
            rgba: vec![0, 10, 20, 255],
        };
        let changed = PngImage {
            width: 1,
            height: 1,
            rgba: vec![0, 12, 20, 255],
        };
        assert_eq!(compare_pixels(&image, &changed, 1).unwrap().changed, 1);
        assert_eq!(compare_pixels(&image, &changed, 2).unwrap().changed, 0);

        let baseline = json!({ "source": null });
        let current = json!({});
        let mut differences = Vec::new();
        compare_json("", &baseline, &current, 0.0, &mut differences);
        assert_eq!(differences[0]["path"], "/source");
        assert_eq!(differences[0]["current"]["$missing"], true);
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_equal_json_skips_path_building() {
        const FIELDS: usize = 4_096;
        let baseline = Value::Object(
            (0..FIELDS)
                .map(|index| (format!("field_{index:04}"), Value::from(index)))
                .collect(),
        );
        let current = baseline.clone();
        let mut differences = Vec::new();

        let measured = crate::allocation::clean_window(0, || {
            compare_json("", &baseline, &current, 0.0, &mut differences);
        });

        assert!(differences.is_empty());
        assert_eq!(
            measured,
            (0, 0),
            "equal JSON diff allocations: {measured:?}"
        );
        eprintln!(
            "unchanged {FIELDS}-field JSON diff: {} heap blocks / {} bytes",
            measured.0, measured.1
        );
    }

    #[test]
    fn direct_diff_rejects_missing_string_and_unsupported_capture_schemas() {
        let fixture = tempfile::tempdir().unwrap();
        let baseline = fixture.path().join("baseline.json");
        let current = fixture.path().join("current.json");
        let output = fixture.path().join("diff");
        fs::write(
            &current,
            serde_json::to_vec(&capture_manifest("current")).unwrap(),
        )
        .unwrap();
        fs::write(fixture.path().join("current.png"), []).unwrap();

        for invalid in [
            json!({}),
            json!({ "schema_version": "2" }),
            json!({
                "schema_version": CAPTURE_SCHEMA_VERSION + 1
            }),
        ] {
            fs::write(&baseline, serde_json::to_vec(&invalid).unwrap()).unwrap();
            assert!(
                compare_capture_manifests(&baseline, &current, &output, DiffThresholds::default(),)
                    .is_err()
            );
        }

        fs::write(
            &baseline,
            serde_json::to_vec(&capture_manifest("baseline")).unwrap(),
        )
        .unwrap();
        fs::write(fixture.path().join("baseline.png"), []).unwrap();
        fs::write(
            &current,
            serde_json::to_vec(&json!({ "schema_version": "2" })).unwrap(),
        )
        .unwrap();
        assert!(
            compare_capture_manifests(&baseline, &current, &output, DiffThresholds::default())
                .unwrap_err()
                .contains(&current.display().to_string())
        );

        let partial = json!({
            "schema_version": CAPTURE_SCHEMA_VERSION,
            "png": "missing.png"
        });
        fs::write(&baseline, serde_json::to_vec(&partial).unwrap()).unwrap();
        fs::write(&current, serde_json::to_vec(&partial).unwrap()).unwrap();
        assert!(
            compare_capture_manifests(&baseline, &current, &output, DiffThresholds::default())
                .is_err()
        );
    }

    #[test]
    #[ignore = "CI performance contract; run explicitly"]
    fn performance_contract_compares_four_megapixels_in_one_pass() {
        const WIDTH: u32 = 2048;
        const HEIGHT: u32 = 2048;
        const PIXELS: usize = WIDTH as usize * HEIGHT as usize;
        const BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

        let baseline = PngImage {
            width: WIDTH,
            height: HEIGHT,
            rgba: vec![0; PIXELS * 4],
        };
        let current = PngImage {
            width: WIDTH,
            height: HEIGHT,
            rgba: vec![0; PIXELS * 4],
        };
        let started = std::time::Instant::now();
        let difference = compare_pixels(&baseline, &current, 0).unwrap();
        let elapsed = started.elapsed();

        assert_eq!(difference.total, PIXELS);
        assert_eq!(difference.changed, 0);
        assert_eq!(difference.rgba.len(), PIXELS * 4);
        assert_eq!(difference.rgba.capacity(), PIXELS * 4);
        assert!(
            elapsed <= BUDGET,
            "four-megapixel inspection diff took {elapsed:?}; budget is {BUDGET:?}"
        );
        eprintln!("{PIXELS} pixels compared in {elapsed:?} with an exact-size output buffer");
    }
}
