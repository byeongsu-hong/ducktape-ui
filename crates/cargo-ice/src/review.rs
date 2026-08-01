use crate::evidence::{REVIEW_ARTIFACT_KIND, REVIEW_SCHEMA_VERSION, validate_capture_manifest};
use crate::inspection::{
    DiffThresholds, compare_capture_manifests, containing_package, source_output_name,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, PartialEq)]
struct ReviewOptions {
    source: PathBuf,
    package: Option<String>,
    output: Option<PathBuf>,
    baseline: Option<PathBuf>,
    tests: Vec<String>,
    thresholds: DiffThresholds,
}

#[derive(Debug, PartialEq)]
enum BaselineScope {
    Full,
    Selected(BTreeSet<String>),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReviewArtifactKind {
    IceReviewBundle,
}

#[derive(Deserialize)]
struct RequiredNullable<T>(Option<T>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewReportV1 {
    #[serde(rename = "artifact_kind")]
    _artifact_kind: ReviewArtifactKind,
    schema_version: u64,
    success: bool,
    #[serde(rename = "source")]
    _source: String,
    #[serde(rename = "package")]
    _package: RequiredNullable<String>,
    #[serde(rename = "run_id")]
    _run_id: String,
    #[serde(rename = "tests")]
    _tests: BTreeMap<String, Value>,
    #[serde(rename = "diagnostics")]
    _diagnostics: BTreeMap<String, Value>,
    captures: Vec<ReviewCaptureV1>,
    #[serde(rename = "source_mapped_changes")]
    _source_mapped_changes: Vec<Value>,
    #[serde(rename = "accessibility")]
    _accessibility: BTreeMap<String, Value>,
    #[serde(rename = "baseline")]
    _baseline: RequiredNullable<String>,
    #[serde(rename = "baseline_error")]
    _baseline_error: RequiredNullable<String>,
    #[serde(rename = "thresholds")]
    _thresholds: BTreeMap<String, Value>,
    #[serde(rename = "failure")]
    _failure: Option<BTreeMap<String, Value>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewCaptureV1 {
    key: String,
    manifest: String,
    #[serde(rename = "name")]
    _name: Option<String>,
    #[serde(rename = "png")]
    _png: Option<String>,
    #[serde(rename = "comparison")]
    _comparison: BTreeMap<String, Value>,
}

impl BaselineScope {
    fn from_selection(selected: &[String], explicitly_selected: bool) -> Self {
        if explicitly_selected {
            Self::Selected(selected.iter().map(|test| safe_component(test)).collect())
        } else {
            Self::Full
        }
    }

    fn contains_capture(&self, key: &str) -> bool {
        match self {
            Self::Full => true,
            Self::Selected(tests) => key
                .split_once('/')
                .is_some_and(|(test, _)| tests.contains(test)),
        }
    }
}

pub(super) fn review(root: &Path, args: &[String]) -> Result<(), String> {
    let options = parse_review(args)?;
    let source = root
        .join(&options.source)
        .canonicalize()
        .map_err(|error| format!("cannot open {}: {error}", options.source.display()))?;
    let source_text = fs::read_to_string(&source).map_err(|error| error.to_string())?;
    if !ui_lang_core::source_is_app(&source_text) {
        return Err(format!(
            "{} is not an Ice root; review the file containing its top-level `app` or `daemon`",
            source.display()
        ));
    }

    let run_id = run_id()?;

    let output = options.output.as_ref().map_or_else(
        || {
            root.join("target/ice-review")
                .join(source_output_name(root, &source))
        },
        |path| root.join(path),
    );
    fs::create_dir_all(&output)
        .map_err(|error| format!("cannot create review output {}: {error}", output.display()))?;
    let output = output.canonicalize().unwrap_or(output);
    match review_opened(root, &source, &output, &options, &run_id) {
        Ok(()) => Ok(()),
        Err(error) => {
            match ensure_current_failure_bundle(root, &source, &output, &run_id, &error) {
                Ok(()) => Err(error),
                Err(publish_error) => Err(format!(
                    "{error}\nfailed to publish current review failure bundle: {publish_error}"
                )),
            }
        }
    }
}

fn review_opened(
    root: &Path,
    source: &Path,
    output: &Path,
    options: &ReviewOptions,
    run_id: &str,
) -> Result<(), String> {
    let analysis = match ui_lang_core::analyze_file_graph(source) {
        Ok(analysis) => analysis,
        Err(error) => return analysis_failure(root, source, output, run_id, error),
    };
    let selected = selected_tests(&analysis.document.tests, &options.tests)?;
    let baseline_scope = BaselineScope::from_selection(&selected, !options.tests.is_empty());
    let expected_captures = expected_capture_keys(&analysis.document.tests, &selected)?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let package = options
        .package
        .clone()
        .map(Ok)
        .unwrap_or_else(|| containing_package(root, source, &cargo))?;
    let artifact_dir = output.join("artifacts").join(run_id);
    let log_dir = output.join("logs").join(run_id);
    let diff_dir = output.join("diffs").join(run_id);
    fs::create_dir_all(&artifact_dir).map_err(|error| error.to_string())?;
    fs::create_dir_all(&log_dir).map_err(|error| error.to_string())?;

    let mut test_results = Vec::new();
    for test in &selected {
        let started = Instant::now();
        let output_result = Command::new(&cargo)
            .current_dir(root)
            .args([
                "test",
                "--package",
                &package,
                &format!("__ice_tests::{test}"),
                "--",
                "--exact",
                "--nocapture",
            ])
            .env("ICE_TEST_ARTIFACT_DIR", &artifact_dir)
            .env("ICE_AGENT_INSPECT_ROOT", root)
            .output()
            .map_err(|error| format!("cannot run Ice test `{test}`: {error}"))?;
        let elapsed_ms = started.elapsed().as_millis();
        let executions =
            exact_test_executions(&output_result.stdout, &format!("__ice_tests::{test}"));
        let execution_error = if output_result.status.success() {
            match executions {
                1 => None,
                0 => Some(format!(
                    "cargo reported success without executing exact Ice test `{test}`; ensure the root is included with `ui_lang::include_app!`"
                )),
                count => Some(format!(
                    "cargo executed exact Ice test `{test}` {count} times; select a package target with one generated test"
                )),
            }
        } else {
            None
        };
        let stem = safe_component(test);
        let stdout_path = log_dir.join(format!("{stem}.stdout.log"));
        let stderr_path = log_dir.join(format!("{stem}.stderr.log"));
        fs::write(&stdout_path, &output_result.stdout).map_err(|error| error.to_string())?;
        fs::write(&stderr_path, &output_result.stderr).map_err(|error| error.to_string())?;
        test_results.push(json!({
            "name": test,
            "passed": output_result.status.success() && execution_error.is_none(),
            "executions": executions,
            "error": execution_error,
            "status_code": output_result.status.code(),
            "elapsed_ms": elapsed_ms,
            "stdout": relative_path(output, &stdout_path),
            "stderr": relative_path(output, &stderr_path),
        }));
    }

    let diagnostics = analysis
        .document
        .warnings()
        .iter()
        .map(|warning| {
            json!({
                "severity": "warning",
                "code": warning.code,
                "path": warning.path.as_deref().map_or_else(
                    || relative_path(root, source),
                    |path| relative_path(root, Path::new(path)),
                ),
                "line": warning.line,
                "column": warning.column,
                "message": warning.message,
                "hint": warning.hint,
                "rendered": warning.render(&source.display().to_string()),
            })
        })
        .collect::<Vec<_>>();
    let diagnostics_path = output.join("diagnostics.json");
    write_json(
        &diagnostics_path,
        &json!({ "schema_version": REVIEW_SCHEMA_VERSION, "diagnostics": diagnostics }),
    )?;

    let current = capture_index(&artifact_dir)?;
    let (baseline, baseline_error) = match options
        .baseline
        .as_ref()
        .map(|path| baseline_index(&root.join(path), &baseline_scope))
        .transpose()
    {
        Ok(baseline) => (baseline, None),
        Err(error) => (None, Some(error)),
    };
    let mut captures = Vec::new();
    let mut source_changes = Vec::new();
    let mut compared_baselines = BTreeSet::new();
    let mut valid_manifests = Vec::new();
    for (key, current_path) in &current {
        let manifest = match read_json(current_path) {
            Ok(manifest) => manifest,
            Err(error) => {
                captures.push(json!({
                    "key": key,
                    "manifest": relative_path(output, current_path),
                    "comparison": { "status": "error", "error": error },
                }));
                continue;
            }
        };
        let png = match validate_capture_manifest(current_path, &manifest) {
            Ok(manifest) => manifest.png,
            Err(error) => {
                captures.push(json!({
                    "key": key,
                    "name": manifest["name"],
                    "manifest": relative_path(output, current_path),
                    "comparison": { "status": "error", "error": error },
                }));
                continue;
            }
        };
        let mut capture = json!({
            "key": key,
            "name": manifest["name"],
            "manifest": relative_path(output, current_path),
            "png": relative_path(output, &png),
            "comparison": { "status": "not_requested" },
        });
        if !expected_captures.contains(key) {
            capture["comparison"] = json!({
                "status": "unexpected_current",
                "error": "capture was not declared by a selected Ice test",
            });
        } else if let Some(error) = &baseline_error {
            capture["comparison"] = json!({ "status": "error", "error": error });
        } else if let Some(baseline) = &baseline {
            if let Some(baseline_path) = baseline.get(key) {
                compared_baselines.insert(key.clone());
                let destination = diff_dir.join(key.trim_end_matches(".json"));
                match compare_capture_manifests(
                    baseline_path,
                    current_path,
                    &destination,
                    options.thresholds,
                ) {
                    Ok(report) => {
                        let baseline_manifest = read_json(baseline_path).ok();
                        for difference in report["manifest"]["differences"]
                            .as_array()
                            .into_iter()
                            .flatten()
                        {
                            source_changes.push(source_change(
                                key,
                                difference,
                                baseline_manifest.as_ref(),
                                &manifest,
                            ));
                        }
                        capture["comparison"] = json!({
                            "status": if report["matches"] == true { "match" } else { "changed" },
                            "matches": report["matches"],
                            "baseline_manifest": baseline_path,
                            "report": relative_path(output, &destination.join("report.json")),
                            "diff_png": relative_path(output, &destination.join("diff.png")),
                            "manifest_differences": report["manifest"]["difference_count"],
                            "changed_ratio": report["pixels"]["changed_ratio"],
                        });
                    }
                    Err(error) => {
                        capture["comparison"] = json!({
                            "status": "error",
                            "baseline_manifest": baseline_path,
                            "error": error,
                        });
                    }
                }
            } else {
                capture["comparison"] = json!({ "status": "missing_baseline" });
            }
        }
        captures.push(capture);
        valid_manifests.push(manifest);
    }
    for key in &expected_captures {
        if !current.contains_key(key) {
            if let Some(baseline_path) = baseline.as_ref().and_then(|baseline| baseline.get(key)) {
                compared_baselines.insert(key.clone());
                captures.push(json!({
                    "key": key,
                    "comparison": {
                        "status": "removed",
                        "baseline_manifest": baseline_path,
                        "error": "selected Ice test did not publish its declared capture",
                    },
                }));
            } else {
                captures.push(json!({
                    "key": key,
                    "comparison": {
                        "status": "missing_current",
                        "error": "selected Ice test did not publish its declared capture",
                    },
                }));
            }
        }
    }
    if let Some(baseline) = &baseline {
        for key in baseline.keys() {
            if baseline_scope.contains_capture(key)
                && !compared_baselines.contains(key)
                && !current.contains_key(key)
            {
                captures.push(json!({
                    "key": key,
                    "comparison": {
                        "status": "removed",
                        "baseline_manifest": baseline[key],
                    }
                }));
            }
        }
    }
    captures.sort_by(|left, right| left["key"].as_str().cmp(&right["key"].as_str()));

    let accessibility = accessibility_summary(valid_manifests.iter());
    let tests_passed = test_results.iter().all(|test| test["passed"] == true);
    let comparisons_passed = captures.iter().all(|capture| {
        matches!(
            capture["comparison"]["status"].as_str(),
            Some("not_requested" | "match")
        )
    });
    let success = tests_passed && comparisons_passed && baseline_error.is_none();
    let report_path = output.join("report.json");
    let html_path = output.join("report.html");
    let report = json!({
        "artifact_kind": REVIEW_ARTIFACT_KIND,
        "schema_version": REVIEW_SCHEMA_VERSION,
        "success": success,
        "source": relative_path(root, source),
        "package": package,
        "run_id": run_id,
        "tests": {
            "selected": selected,
            "passed": test_results.iter().filter(|test| test["passed"] == true).count(),
            "failed": test_results.iter().filter(|test| test["passed"] != true).count(),
            "results": test_results,
        },
        "diagnostics": {
            "path": relative_path(output, &diagnostics_path),
            "warning_count": diagnostics.len(),
            "error_count": 0,
            "items": diagnostics,
        },
        "captures": captures,
        "source_mapped_changes": source_changes,
        "accessibility": accessibility,
        "baseline": options.baseline,
        "baseline_error": baseline_error,
        "thresholds": {
            "pixel": options.thresholds.pixel,
            "max_changed_ratio": options.thresholds.max_changed_ratio,
            "value": options.thresholds.value,
        },
    });
    write_json(&report_path, &report)?;
    fs::write(&html_path, render_html(&report)).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "success": success,
            "report": report_path,
            "html": html_path,
            "tests": report["tests"],
            "captures": report["captures"].as_array().map_or(0, Vec::len),
        }))
        .expect("review result is serializable")
    );
    if success {
        Ok(())
    } else {
        Err(format!(
            "review evidence failed; see {}",
            report_path.display()
        ))
    }
}

fn analysis_failure(
    root: &Path,
    source: &Path,
    output: &Path,
    run_id: &str,
    error: ui_lang_core::Error,
) -> Result<(), String> {
    let rendered = error.render(&source.display().to_string());
    let path = error.path.as_deref().map_or_else(
        || relative_path(root, source),
        |path| relative_path(root, Path::new(path)),
    );
    let diagnostic = json!({
        "severity": "error",
        "code": error.code,
        "path": path,
        "line": error.line,
        "column": error.column,
        "message": error.message,
        "hint": error.hint,
        "rendered": rendered,
    });
    let diagnostics_path = output.join("diagnostics.json");
    write_json(
        &diagnostics_path,
        &json!({
            "schema_version": REVIEW_SCHEMA_VERSION,
            "diagnostics": [&diagnostic]
        }),
    )?;
    let report_path = output.join("report.json");
    let html_path = output.join("report.html");
    let report = json!({
        "artifact_kind": REVIEW_ARTIFACT_KIND,
        "schema_version": REVIEW_SCHEMA_VERSION,
        "success": false,
        "source": relative_path(root, source),
        "package": null,
        "run_id": run_id,
        "tests": { "selected": [], "passed": 0, "failed": 0, "results": [] },
        "diagnostics": {
            "path": relative_path(output, &diagnostics_path),
            "warning_count": 0,
            "error_count": 1,
            "items": [diagnostic],
        },
        "captures": [],
        "source_mapped_changes": [],
        "accessibility": {
            "target_count": 0,
            "semantic_target_count": 0,
            "named_semantic_target_count": 0,
            "actionable_target_count": 0,
            "actionable_without_name_count": 0,
            "actionable_without_name": [],
            "roles": {},
        },
    });
    write_json(&report_path, &report)?;
    fs::write(&html_path, render_html(&report)).map_err(|error| error.to_string())?;
    eprintln!(
        "review diagnostics written to {}",
        diagnostics_path.display()
    );
    Err(format!(
        "{rendered}\nreview evidence failed; see {}",
        report_path.display()
    ))
}

fn ensure_current_failure_bundle(
    root: &Path,
    source: &Path,
    output: &Path,
    run_id: &str,
    error: &str,
) -> Result<(), String> {
    let report_path = output.join("report.json");
    let html_path = output.join("report.html");
    if let Ok(report) = read_json(&report_path)
        && report["artifact_kind"] == REVIEW_ARTIFACT_KIND
        && report["schema_version"] == REVIEW_SCHEMA_VERSION
        && report["run_id"] == run_id
        && report["success"] == false
    {
        fs::write(&html_path, render_html(&report)).map_err(|write_error| {
            format!(
                "cannot finish detailed failure HTML {}: {write_error}",
                html_path.display()
            )
        })?;
        return Ok(());
    }

    let diagnostic = json!({
        "severity": "error",
        "code": "E_REVIEW",
        "path": relative_path(root, source),
        "line": 1,
        "column": 1,
        "message": error,
        "hint": null,
        "rendered": error,
    });
    let diagnostics_path = output.join("diagnostics.json");
    write_json(
        &diagnostics_path,
        &json!({
            "schema_version": REVIEW_SCHEMA_VERSION,
            "diagnostics": [&diagnostic],
        }),
    )?;
    let report = json!({
        "artifact_kind": REVIEW_ARTIFACT_KIND,
        "schema_version": REVIEW_SCHEMA_VERSION,
        "success": false,
        "source": relative_path(root, source),
        "package": null,
        "run_id": run_id,
        "tests": { "selected": [], "passed": 0, "failed": 0, "results": [] },
        "diagnostics": {
            "path": relative_path(output, &diagnostics_path),
            "warning_count": 0,
            "error_count": 1,
            "items": [diagnostic],
        },
        "captures": [],
        "source_mapped_changes": [],
        "accessibility": empty_accessibility_summary(),
        "baseline": null,
        "baseline_error": error,
        "failure": {
            "kind": "tooling",
            "message": error,
        },
    });
    write_json(&report_path, &report)?;
    fs::write(&html_path, render_html(&report)).map_err(|write_error| {
        format!(
            "cannot write generic failure HTML {}: {write_error}",
            html_path.display()
        )
    })
}

fn empty_accessibility_summary() -> Value {
    json!({
        "target_count": 0,
        "semantic_target_count": 0,
        "named_semantic_target_count": 0,
        "actionable_target_count": 0,
        "actionable_without_name_count": 0,
        "actionable_without_name": [],
        "roles": {},
    })
}

fn parse_review(args: &[String]) -> Result<ReviewOptions, String> {
    let Some(source) = args.first() else {
        return Err("cargo ice review <file.ice> [options]".into());
    };
    if source.starts_with('-') {
        return Err("cargo ice review requires a root .ice file first".into());
    }
    let mut options = ReviewOptions {
        source: source.into(),
        ..ReviewOptions::default()
    };
    let mut index = 1;
    let mut seen_pixel = false;
    let mut seen_ratio = false;
    let mut seen_value = false;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?
            .clone();
        match flag {
            "--package" => set_once(&mut options.package, value, flag)?,
            "--output" => set_once(&mut options.output, value.into(), flag)?,
            "--baseline" => set_once(&mut options.baseline, value.into(), flag)?,
            "--test" => {
                if options.tests.contains(&value) {
                    return Err(format!("duplicate `--test {value}`"));
                }
                options.tests.push(value);
            }
            "--pixel-threshold" if !seen_pixel => {
                options.thresholds.pixel = value
                    .parse()
                    .map_err(|error| format!("invalid {flag} value: {error}"))?;
                seen_pixel = true;
            }
            "--max-changed-ratio" if !seen_ratio => {
                options.thresholds.max_changed_ratio = unit_f64(flag, &value)?;
                seen_ratio = true;
            }
            "--value-tolerance" if !seen_value => {
                options.thresholds.value = nonnegative_f64(flag, &value)?;
                seen_value = true;
            }
            "--pixel-threshold" | "--max-changed-ratio" | "--value-tolerance" => {
                return Err(format!("duplicate `{flag}`"));
            }
            _ => return Err(format!("unknown review option `{flag}`")),
        }
        index += 2;
    }
    Ok(options)
}

fn selected_tests(
    tests: &[ui_lang_core::TestDecl],
    requested: &[String],
) -> Result<Vec<String>, String> {
    let available = tests
        .iter()
        .map(|test| test.name.as_str())
        .collect::<BTreeSet<_>>();
    if available.is_empty() {
        return Err("review root declares no Ice tests".into());
    }
    if let Some(missing) = requested
        .iter()
        .find(|name| !available.contains(name.as_str()))
    {
        return Err(format!("review root does not declare Ice test `{missing}`"));
    }
    let mut selected = if requested.is_empty() {
        available.into_iter().map(str::to_owned).collect()
    } else {
        requested.to_vec()
    };
    selected.sort();
    Ok(selected)
}

fn expected_capture_keys(
    tests: &[ui_lang_core::TestDecl],
    selected: &[String],
) -> Result<BTreeSet<String>, String> {
    let selected = selected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut captures = BTreeSet::new();
    for test in tests
        .iter()
        .filter(|test| selected.contains(test.name.as_str()))
    {
        for step in &test.steps {
            if let ui_lang_core::TestStepKind::Capture(name) = &step.kind {
                let key = format!("{}/{}.json", safe_component(&test.name), name);
                if !captures.insert(key.clone()) {
                    return Err(format!(
                        "selected Ice tests declare duplicate capture key `{key}`"
                    ));
                }
            }
        }
    }
    Ok(captures)
}

fn exact_test_executions(stdout: &[u8], test: &str) -> usize {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("test "))
        .filter_map(|line| line.split_once(" ... "))
        .filter(|(name, _)| *name == test)
        .count()
}

fn capture_index(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut paths = Vec::new();
    collect_json(root, &mut paths)?;
    let mut index = BTreeMap::new();
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if index.insert(relative.clone(), path).is_some() {
            return Err(format!("duplicate review capture key `{relative}`"));
        }
    }
    Ok(index)
}

fn baseline_index(path: &Path, scope: &BaselineScope) -> Result<BTreeMap<String, PathBuf>, String> {
    let report_path = if path.is_dir() {
        path.join("report.json")
    } else {
        path.to_owned()
    };
    if report_path.is_file() {
        let value = read_json(&report_path)?;
        let report = serde_json::from_value::<ReviewReportV1>(value).map_err(|error| {
            format!(
                "{} is not a strict review report v{REVIEW_SCHEMA_VERSION}: {error}",
                report_path.display()
            )
        })?;
        if report.schema_version != REVIEW_SCHEMA_VERSION {
            return Err(format!(
                "{} uses unsupported review report schema version {}; expected {}",
                report_path.display(),
                report.schema_version,
                REVIEW_SCHEMA_VERSION
            ));
        }
        if !report.success {
            return Err(format!(
                "{} is not a successful review baseline",
                report_path.display()
            ));
        }
        if report._baseline_error.0.is_some() {
            return Err(format!(
                "{} claims success while retaining `baseline_error`",
                report_path.display()
            ));
        }
        if report._source.is_empty()
            || report._package.0.as_deref().is_none_or(str::is_empty)
            || report._run_id.is_empty()
            || report._failure.is_some()
        {
            return Err(format!(
                "{} has an inconsistent successful review envelope",
                report_path.display()
            ));
        }
        let directory = report_path.parent().unwrap_or_else(|| Path::new("."));
        let mut index = BTreeMap::new();
        let mut keys = BTreeSet::new();
        for capture in &report.captures {
            validate_report_capture_key(&report_path, &capture.key)?;
            if !matches!(
                capture._comparison.get("status").and_then(Value::as_str),
                Some("not_requested" | "match")
            ) {
                return Err(format!(
                    "{} claims success with a non-passing comparison for capture `{}`",
                    report_path.display(),
                    capture.key
                ));
            }
            if !keys.insert(capture.key.clone()) {
                return Err(format!(
                    "{} contains duplicate capture key `{}`",
                    report_path.display(),
                    capture.key
                ));
            }
        }
        for capture in report.captures {
            if !scope.contains_capture(&capture.key) {
                continue;
            }
            let manifest = resolve_report_path(directory, &capture.manifest)?;
            validate_baseline_manifest(&manifest)?;
            if index.insert(capture.key.clone(), manifest).is_some() {
                return Err(format!(
                    "{} contains duplicate capture key `{}`",
                    report_path.display(),
                    capture.key
                ));
            }
        }
        return Ok(index);
    }
    let mut index = capture_index(path)?;
    index.retain(|key, _| scope.contains_capture(key));
    for manifest in index.values() {
        validate_baseline_manifest(manifest)?;
    }
    Ok(index)
}

fn validate_baseline_manifest(path: &Path) -> Result<(), String> {
    let manifest = read_json(path)?;
    validate_capture_manifest(path, &manifest).map(|_| ())
}

fn validate_report_capture_key(report_path: &Path, key: &str) -> Result<(), String> {
    let Some((test, capture)) = key.split_once('/') else {
        return Err(format!(
            "{} contains invalid capture key {key:?}; expected test/capture.json",
            report_path.display()
        ));
    };
    if test.is_empty()
        || capture.is_empty()
        || capture.contains('/')
        || !capture.ends_with(".json")
        || ![test, capture.trim_end_matches(".json")]
            .into_iter()
            .all(|component| {
                !component.is_empty()
                    && component.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                    })
            })
    {
        return Err(format!(
            "{} contains invalid capture key {key:?}; expected safe test/capture.json",
            report_path.display()
        ));
    }
    Ok(())
}

fn resolve_report_path(directory: &Path, value: &str) -> Result<PathBuf, String> {
    let relative = Path::new(value);
    if relative.as_os_str().is_empty()
        || !relative
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "review report path {value:?} must stay below {}",
            directory.display()
        ));
    }
    let directory = directory.canonicalize().map_err(|error| {
        format!(
            "cannot resolve review directory {}: {error}",
            directory.display()
        )
    })?;
    let path = directory.join(relative).canonicalize().map_err(|error| {
        format!(
            "cannot resolve review evidence {}: {error}",
            directory.join(relative).display()
        )
    })?;
    if !path.starts_with(&directory) || !path.is_file() {
        return Err(format!(
            "review report path {value:?} escapes {}",
            directory.display()
        ));
    }
    Ok(path)
}

fn collect_json(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(path)
        .map_err(|error| format!("cannot read capture directory {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_json(&entry.path(), output)?;
        } else if file_type.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
        {
            output.push(entry.path());
        }
    }
    output.sort();
    Ok(())
}

fn accessibility_summary<'a>(manifests: impl Iterator<Item = &'a Value>) -> Value {
    let mut targets = 0_u64;
    let mut semantic = 0_u64;
    let mut named = 0_u64;
    let mut actionable = 0_u64;
    let mut actionable_without_name = Vec::new();
    let mut roles = BTreeMap::<String, u64>::new();
    for manifest in manifests {
        for target in manifest["targets"].as_array().into_iter().flatten() {
            targets += 1;
            let Some(accessibility) = target["accessibility"].as_object() else {
                continue;
            };
            semantic += 1;
            if let Some(role) = accessibility["role"].as_str() {
                *roles.entry(role.to_owned()).or_default() += 1;
            }
            let has_name = accessibility["name"]
                .as_str()
                .is_some_and(|name| !name.trim().is_empty());
            named += u64::from(has_name);
            let has_action = accessibility["actions"]["click"] == true
                || accessibility["actions"]["focus"] == true;
            actionable += u64::from(has_action);
            if has_action && !has_name {
                actionable_without_name.push(json!({
                    "id": target["id"],
                    "role": accessibility["role"],
                    "source": target["source"],
                }));
            }
        }
    }
    json!({
        "target_count": targets,
        "semantic_target_count": semantic,
        "named_semantic_target_count": named,
        "actionable_target_count": actionable,
        "actionable_without_name_count": actionable_without_name.len(),
        "actionable_without_name": actionable_without_name,
        "roles": roles,
    })
}

fn source_change(
    capture: &str,
    difference: &Value,
    baseline_manifest: Option<&Value>,
    current_manifest: &Value,
) -> Value {
    let path = difference["path"].as_str().unwrap_or_default();
    let baseline_source = baseline_manifest.and_then(|manifest| change_source(path, manifest));
    let current_source = change_source(path, current_manifest);
    let source = if difference["current"]["$missing"] == true {
        baseline_source.or(current_source)
    } else {
        current_source.or(baseline_source)
    }
    .cloned()
    .unwrap_or(Value::Null);
    json!({
        "capture": capture,
        "json_path": path,
        "source": source,
        "baseline_source": baseline_source,
        "current_source": current_source,
        "baseline": difference["baseline"],
        "current": difference["current"],
    })
}

fn change_source<'a>(path: &str, manifest: &'a Value) -> Option<&'a Value> {
    path.strip_prefix("/targets/")
        .and_then(|path| path.split('/').next())
        .and_then(|index| index.parse::<usize>().ok())
        .and_then(|index| manifest["targets"].get(index))
        .and_then(|target| target.get("source"))
        .filter(|source| !source.is_null())
        .or_else(|| manifest.get("capture_source"))
        .filter(|source| !source.is_null())
}

fn render_html(report: &Value) -> String {
    let status = if report["success"] == true {
        "PASS"
    } else {
        "FAIL"
    };
    let mut html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Ice review {status}</title><style>{}</style><main><h1>Ice review: {status}</h1><p><code>{}</code></p>",
        "body{font:14px system-ui;max-width:1200px;margin:40px auto;padding:0 20px;color:#18202a}code{background:#eef1f4;padding:2px 5px}table{border-collapse:collapse;width:100%}th,td{border:1px solid #ccd3da;padding:8px;text-align:left;vertical-align:top}.ok{color:#087a37}.bad{color:#b42318}.captures{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:16px}.card{border:1px solid #ccd3da;border-radius:8px;padding:12px}.card img{max-width:100%;background:#eee}.muted{color:#5d6875}pre{white-space:pre-wrap}",
        html_escape(report["source"].as_str().unwrap_or("<source>"))
    );
    html.push_str(
        "<h2>Tests</h2><table><tr><th>Name</th><th>Result</th><th>Time</th><th>Logs</th><th>Error</th></tr>",
    );
    for test in report["tests"]["results"].as_array().into_iter().flatten() {
        let passed = test["passed"] == true;
        let _ = write!(
            html,
            "<tr><td><code>{}</code></td><td class=\"{}\">{}</td><td>{} ms</td><td><a href=\"{}\">stdout</a> · <a href=\"{}\">stderr</a></td><td>{}</td></tr>",
            html_escape(test["name"].as_str().unwrap_or_default()),
            if passed { "ok" } else { "bad" },
            if passed { "pass" } else { "fail" },
            test["elapsed_ms"],
            html_escape(test["stdout"].as_str().unwrap_or_default()),
            html_escape(test["stderr"].as_str().unwrap_or_default()),
            html_escape(test["error"].as_str().unwrap_or_default()),
        );
    }
    html.push_str("</table><h2>Captures</h2><div class=\"captures\">");
    if let Some(error) = report["baseline_error"].as_str() {
        let _ = write!(html, "<pre class=\"bad\">{}</pre>", html_escape(error));
    }
    for capture in report["captures"].as_array().into_iter().flatten() {
        let key = html_escape(capture["key"].as_str().unwrap_or_default());
        let comparison = html_escape(
            capture["comparison"]["status"]
                .as_str()
                .unwrap_or("unknown"),
        );
        let _ = write!(
            html,
            "<section class=\"card\"><h3>{key}</h3><p>comparison: <code>{comparison}</code></p>"
        );
        if let Some(error) = capture["comparison"]["error"].as_str() {
            let _ = write!(html, "<pre class=\"bad\">{}</pre>", html_escape(error));
        }
        if let Some(png) = capture["png"].as_str() {
            let png = html_escape(png);
            let _ = write!(
                html,
                "<a href=\"{png}\"><img src=\"{png}\" alt=\"{key}\"></a>"
            );
        }
        if let Some(diff) = capture["comparison"]["diff_png"].as_str() {
            let diff = html_escape(diff);
            let _ = write!(
                html,
                "<p>Diff</p><a href=\"{diff}\"><img src=\"{diff}\" alt=\"diff {key}\"></a>"
            );
        }
        html.push_str("</section>");
    }
    let a11y = &report["accessibility"];
    let _ = write!(
        html,
        "</div><h2>Accessibility</h2><p>{} semantic targets; {} named; {} actionable without a name.</p>",
        a11y["semantic_target_count"],
        a11y["named_semantic_target_count"],
        a11y["actionable_without_name_count"],
    );
    html.push_str("<h2>Source-mapped changes</h2><table><tr><th>Capture</th><th>Path</th><th>Source</th></tr>");
    for change in report["source_mapped_changes"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let source = serde_json::to_string(&change["source"]).unwrap_or_default();
        let _ = write!(
            html,
            "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td></tr>",
            html_escape(change["capture"].as_str().unwrap_or_default()),
            html_escape(change["json_path"].as_str().unwrap_or_default()),
            html_escape(&source),
        );
    }
    html.push_str("</table><h2>Diagnostics</h2><pre>");
    for diagnostic in report["diagnostics"]["items"]
        .as_array()
        .into_iter()
        .flatten()
    {
        html.push_str(&html_escape(
            diagnostic["rendered"].as_str().unwrap_or_default(),
        ));
        html.push('\n');
    }
    html.push_str("</pre></main>");
    html
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn safe_component(value: &str) -> String {
    let value = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if value.is_empty() {
        "test".into()
    } else {
        value
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn run_id() -> Result<String, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    Ok(format!("{}-{nanos}", std::process::id()))
}

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("duplicate `{flag}`"))
    } else {
        Ok(())
    }
}

fn nonnegative_f64(flag: &str, value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|error| format!("invalid {flag} value: {error}"))?;
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(format!("{flag} requires a finite non-negative number"))
    }
}

fn unit_f64(flag: &str, value: &str) -> Result<f64, String> {
    let value = nonnegative_f64(flag, value)?;
    if value <= 1.0 {
        Ok(value)
    } else {
        Err(format!("{flag} must be between 0 and 1"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::CAPTURE_SCHEMA_VERSION;
    use ui_lang_core::{Span, TestDecl};

    fn fixture(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "cargo-ice-review-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn test_decl(name: &str) -> TestDecl {
        TestDecl {
            name: name.into(),
            preset: None,
            viewport: None,
            timeout_ms: None,
            theme: None,
            scale_factor: None,
            locale: None,
            platform: None,
            reduced_motion: None,
            mount: None,
            targets: Vec::new(),
            steps: Vec::new(),
            span: Span::line(1),
        }
    }

    fn review_report(captures: Value) -> Value {
        json!({
            "artifact_kind": REVIEW_ARTIFACT_KIND,
            "schema_version": REVIEW_SCHEMA_VERSION,
            "success": true,
            "source": "app.ice",
            "package": "fixture",
            "run_id": "fixture-run",
            "tests": {},
            "diagnostics": {},
            "captures": captures,
            "source_mapped_changes": [],
            "accessibility": {},
            "baseline": null,
            "baseline_error": null,
            "thresholds": {},
        })
    }

    fn capture_entry(key: &str, manifest: &str) -> Value {
        json!({
            "key": key,
            "manifest": manifest,
            "name": "ready",
            "png": "ready.png",
            "comparison": { "status": "not_requested" },
        })
    }

    fn capture_manifest(name: &str) -> Value {
        json!({
            "schema_version": CAPTURE_SCHEMA_VERSION,
            "name": name,
            "png": format!("{name}.png"),
            "capture_source": {
                "path": "app.ice", "line": 1, "column": 1, "statement": format!("capture {name}")
            },
            "viewport": { "width": 320.0, "height": 240.0 },
            "physical_size": { "width": 320, "height": 240 },
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
    fn parses_review_inputs_and_selects_declared_tests() {
        let options = parse_review(&[
            "src/ui/app.ice".into(),
            "--test".into(),
            "wide".into(),
            "--baseline".into(),
            "baseline".into(),
            "--max-changed-ratio".into(),
            "0.01".into(),
        ])
        .unwrap();
        assert_eq!(options.tests, ["wide"]);
        assert_eq!(options.thresholds.max_changed_ratio, 0.01);
        assert!(
            parse_review(&[
                "app.ice".into(),
                "--test".into(),
                "x".into(),
                "--test".into(),
                "x".into()
            ])
            .is_err()
        );
        let tests = [test_decl("narrow"), test_decl("wide")];
        assert_eq!(selected_tests(&tests, &[]).unwrap(), ["narrow", "wide"]);
        assert!(selected_tests(&tests, &["missing".into()]).is_err());
    }

    #[test]
    fn selected_baseline_scope_excludes_unselected_capture_removals() {
        let selected = BaselineScope::from_selection(&["wide".into()], true);
        assert!(selected.contains_capture("wide/current.json"));
        assert!(!selected.contains_capture("narrow/removed.json"));

        let full = BaselineScope::from_selection(&["wide".into()], false);
        assert!(full.contains_capture("narrow/removed.json"));
    }

    #[test]
    fn summarizes_accessibility_and_maps_changes_to_sources() {
        let fixture = fixture("summary");
        fs::create_dir_all(&fixture).unwrap();
        let manifest_path = fixture.join("capture.json");
        let manifest = json!({
            "capture_source": { "path": "app.ice", "line": 20, "column": 3 },
            "targets": [{
                "id": "App/save",
                "source": { "path": "app.ice", "line": 8, "column": 5 },
                "accessibility": {
                    "role": "button", "name": null,
                    "actions": { "click": true, "focus": true }
                }
            }]
        });
        write_json(&manifest_path, &manifest).unwrap();
        let summary = accessibility_summary(std::iter::once(&manifest));
        assert_eq!(summary["semantic_target_count"], 1);
        assert_eq!(summary["actionable_without_name_count"], 1);
        assert_eq!(summary["roles"]["button"], 1);
        let change = source_change(
            "test/capture.json",
            &json!({ "path": "/targets/0/geometry/x", "baseline": 1, "current": 2 }),
            None,
            &manifest,
        );
        assert_eq!(change["source"]["line"], 8);

        let baseline = json!({
            "capture_source": { "path": "app.ice", "line": 20, "column": 3 },
            "targets": [{
                "source": { "path": "old.ice", "line": 4, "column": 2 }
            }]
        });
        let removed = source_change(
            "test/capture.json",
            &json!({
                "path": "/targets/0",
                "baseline": { "id": "gone" },
                "current": { "$missing": true }
            }),
            Some(&baseline),
            &json!({ "capture_source": null, "targets": [] }),
        );
        assert_eq!(removed["source"]["path"], "old.ice");
        assert_eq!(removed["baseline_source"]["line"], 4);
        assert!(removed["current_source"].is_null());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn previous_review_reports_resolve_stable_capture_keys() {
        let fixture = fixture("baseline");
        fs::create_dir_all(fixture.join("artifacts/run/test")).unwrap();
        write_json(
            &fixture.join("artifacts/run/test/wide.json"),
            &capture_manifest("wide"),
        )
        .unwrap();
        fs::write(fixture.join("artifacts/run/test/wide.png"), []).unwrap();
        write_json(
            &fixture.join("report.json"),
            &review_report(json!([capture_entry(
                "test/wide.json",
                "artifacts/run/test/wide.json"
            )])),
        )
        .unwrap();
        let index = baseline_index(&fixture, &BaselineScope::Full).unwrap();
        assert_eq!(
            index["test/wide.json"],
            fixture.join("artifacts/run/test/wide.json")
        );
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn previous_review_reports_reject_escaping_and_duplicate_evidence() {
        let fixture = fixture("unsafe-baseline");
        fs::create_dir_all(&fixture).unwrap();
        write_json(
            &fixture.join("report.json"),
            &review_report(json!([capture_entry("test/wide.json", "../outside.json")])),
        )
        .unwrap();
        assert!(baseline_index(&fixture, &BaselineScope::Full).is_err());

        fs::create_dir_all(fixture.join("artifacts")).unwrap();
        fs::write(fixture.join("artifacts/wide.json"), "{}").unwrap();
        write_json(
            &fixture.join("report.json"),
            &review_report(json!([
                capture_entry("test/wide.json", "artifacts/wide.json"),
                capture_entry("test/wide.json", "artifacts/wide.json")
            ])),
        )
        .unwrap();
        assert!(baseline_index(&fixture, &BaselineScope::Full).is_err());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn selected_reports_do_not_resolve_unselected_manifest_paths() {
        let fixture = fixture("selected-path-scope");
        fs::create_dir_all(fixture.join("artifacts")).unwrap();
        write_json(
            &fixture.join("artifacts/wide.json"),
            &capture_manifest("wide"),
        )
        .unwrap();
        fs::write(fixture.join("artifacts/wide.png"), []).unwrap();
        write_json(
            &fixture.join("report.json"),
            &review_report(json!([
                capture_entry("wide/ready.json", "artifacts/wide.json"),
                capture_entry("narrow/ready.json", "missing/unselected.json")
            ])),
        )
        .unwrap();

        let selected = BaselineScope::from_selection(&["wide".into()], true);
        let index = baseline_index(&fixture, &selected).unwrap();
        assert_eq!(
            index.keys().cloned().collect::<Vec<_>>(),
            ["wide/ready.json"]
        );
        assert!(baseline_index(&fixture, &BaselineScope::Full).is_err());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn previous_review_reports_require_the_supported_schema() {
        let fixture = fixture("baseline-schema");
        fs::create_dir_all(&fixture).unwrap();
        for schema in [Value::Null, json!("1"), json!(REVIEW_SCHEMA_VERSION + 1)] {
            let mut report = review_report(json!([]));
            if !schema.is_null() {
                report["schema_version"] = schema;
            } else {
                report.as_object_mut().unwrap().remove("schema_version");
            }
            write_json(&fixture.join("report.json"), &report).unwrap();
            assert!(baseline_index(&fixture, &BaselineScope::Full).is_err());
        }
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn baselines_require_review_kind_success_and_typed_captures() {
        let fixture = fixture("baseline-envelope");
        fs::create_dir_all(&fixture).unwrap();
        for mutate in [
            "diff-kind",
            "failed",
            "missing-captures",
            "missing-manifest",
            "null-package",
            "failure-marker",
            "mistyped-tests",
            "missing-baseline-error",
            "non-passing-comparison",
            "unknown-field",
        ] {
            let mut report = review_report(json!([]));
            match mutate {
                "diff-kind" => report["artifact_kind"] = json!("ice_capture_diff"),
                "failed" => report["success"] = json!(false),
                "missing-captures" => {
                    report.as_object_mut().unwrap().remove("captures");
                }
                "missing-manifest" => {
                    report["captures"] = json!([{
                        "key": "wide/ready.json",
                        "comparison": {}
                    }]);
                }
                "null-package" => report["package"] = Value::Null,
                "failure-marker" => report["failure"] = json!({ "kind": "tooling" }),
                "mistyped-tests" => report["tests"] = json!([]),
                "missing-baseline-error" => {
                    report.as_object_mut().unwrap().remove("baseline_error");
                }
                "non-passing-comparison" => {
                    report["captures"] =
                        json!([capture_entry("wide/ready.json", "missing/unselected.json")]);
                    report["captures"][0]["comparison"] = json!({ "status": "changed" });
                }
                "unknown-field" => report["extra"] = json!(true),
                _ => unreachable!(),
            }
            write_json(&fixture.join("report.json"), &report).unwrap();
            assert!(
                baseline_index(&fixture, &BaselineScope::Full).is_err(),
                "{mutate}"
            );
        }
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn current_capture_manifest_requires_complete_named_evidence() {
        let fixture = fixture("capture-contract");
        fs::create_dir_all(&fixture).unwrap();
        let path = fixture.join("ready.json");
        let png = fixture.join("ready.png");
        fs::write(&path, "{}").unwrap();
        fs::write(&png, []).unwrap();
        let manifest = capture_manifest("ready");
        assert_eq!(
            validate_capture_manifest(&path, &manifest).unwrap().png,
            png.canonicalize().unwrap()
        );

        let mut incomplete = manifest.clone();
        incomplete.as_object_mut().unwrap().remove("targets");
        assert!(validate_capture_manifest(&path, &incomplete).is_err());
        incomplete["targets"] = json!([]);
        incomplete["name"] = json!("renamed");
        assert!(validate_capture_manifest(&path, &incomplete).is_err());
        for schema in [Value::Null, json!("2"), json!(CAPTURE_SCHEMA_VERSION + 1)] {
            let mut invalid = manifest.clone();
            if schema.is_null() {
                invalid.as_object_mut().unwrap().remove("schema_version");
            } else {
                invalid["schema_version"] = schema;
            }
            assert!(validate_capture_manifest(&path, &invalid).is_err());
        }
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn exact_test_execution_count_rejects_zero_or_ambiguous_matches() {
        let output = b"running 1 test\ntest __ice_tests::wide ... ok\n";
        assert_eq!(exact_test_executions(output, "__ice_tests::wide"), 1);
        assert_eq!(exact_test_executions(output, "__ice_tests::narrow"), 0);

        let duplicated = b"test __ice_tests::wide ... ok\ntest __ice_tests::wide ... ok\n";
        assert_eq!(exact_test_executions(duplicated, "__ice_tests::wide"), 2);
    }

    #[test]
    fn declared_capture_keys_are_stable_and_duplicates_fail() {
        let mut wide = test_decl("wide");
        wide.steps.push(ui_lang_core::TestStep {
            kind: ui_lang_core::TestStepKind::Capture("ready".into()),
            span: Span::line(2),
        });
        assert_eq!(
            expected_capture_keys(&[wide.clone()], &["wide".into()]).unwrap(),
            BTreeSet::from(["wide/ready.json".into()])
        );
        wide.steps.push(ui_lang_core::TestStep {
            kind: ui_lang_core::TestStepKind::Capture("ready".into()),
            span: Span::line(3),
        });
        assert!(expected_capture_keys(&[wide], &["wide".into()]).is_err());
    }

    #[test]
    fn analysis_failures_still_publish_diagnostics_and_html() {
        let fixture = fixture("diagnostic");
        let output = fixture.join("review");
        fs::create_dir_all(&output).unwrap();
        let source = fixture.join("app.ice");
        fs::write(&source, "app Broken\nview\n  wat\n").unwrap();
        let error = ui_lang_core::Error {
            code: "E121",
            path: None,
            line: 3,
            column: 3,
            message: "unknown view node `wat`".into(),
            hint: None,
        };
        let failure =
            analysis_failure(&fixture, &source, &output, "analysis-run", error).unwrap_err();
        assert!(failure.contains("review evidence failed"));
        let report = read_json(&output.join("report.json")).unwrap();
        assert_eq!(report["artifact_kind"], REVIEW_ARTIFACT_KIND);
        assert_eq!(report["run_id"], "analysis-run");
        assert_eq!(report["success"], false);
        assert_eq!(report["diagnostics"]["error_count"], 1);
        assert!(output.join("report.html").is_file());
        assert!(output.join("diagnostics.json").is_file());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn generic_failure_replaces_a_stale_success_for_the_new_run() {
        let fixture = fixture("stale-success");
        let output = fixture.join("review");
        fs::create_dir_all(&output).unwrap();
        let source = fixture.join("app.ice");
        fs::write(&source, "app Fixture\n").unwrap();
        write_json(&output.join("report.json"), &review_report(json!([]))).unwrap();

        ensure_current_failure_bundle(
            &fixture,
            &source,
            &output,
            "new-run",
            "package lookup failed",
        )
        .unwrap();
        let report = read_json(&output.join("report.json")).unwrap();
        assert_eq!(report["artifact_kind"], REVIEW_ARTIFACT_KIND);
        assert_eq!(report["run_id"], "new-run");
        assert_eq!(report["success"], false);
        assert_eq!(report["failure"]["message"], "package lookup failed");
        assert!(output.join("report.html").is_file());
        assert!(output.join("diagnostics.json").is_file());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn opened_review_errors_publish_a_new_failure_instead_of_stale_success() {
        let fixture = fixture("opened-review-failure");
        let output = fixture.join("review");
        fs::create_dir_all(&output).unwrap();
        fs::write(
            fixture.join("app.ice"),
            concat!(
                "app Fixture\n",
                "theme contract AppTheme\n",
                "  bg\n  fg\n  primary\n  danger\n",
                "palette app for AppTheme\n",
                "  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n",
                "test only\n  expect true\n",
                "view\n  text \"fixture\"\n",
            ),
        )
        .unwrap();
        write_json(&output.join("report.json"), &review_report(json!([]))).unwrap();

        let error = review(
            &fixture,
            &[
                "app.ice".into(),
                "--test".into(),
                "missing".into(),
                "--output".into(),
                "review".into(),
            ],
        )
        .unwrap_err();
        assert!(error.contains("does not declare Ice test `missing`"));
        let report = read_json(&output.join("report.json")).unwrap();
        assert_eq!(report["artifact_kind"], REVIEW_ARTIFACT_KIND);
        assert_eq!(report["success"], false);
        assert_ne!(report["run_id"], "fixture-run");
        assert!(
            report["failure"]["message"]
                .as_str()
                .unwrap()
                .contains("missing")
        );
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn generic_failure_does_not_replace_current_detailed_failure() {
        let fixture = fixture("detailed-failure");
        let output = fixture.join("review");
        fs::create_dir_all(&output).unwrap();
        let source = fixture.join("app.ice");
        fs::write(&source, "app Broken\n").unwrap();
        let error = ui_lang_core::Error {
            code: "E121",
            path: None,
            line: 1,
            column: 1,
            message: "detailed diagnostic".into(),
            hint: None,
        };
        analysis_failure(&fixture, &source, &output, "same-run", error).unwrap_err();
        let before = fs::read(output.join("report.json")).unwrap();

        ensure_current_failure_bundle(&fixture, &source, &output, "same-run", "generic fallback")
            .unwrap();
        assert_eq!(fs::read(output.join("report.json")).unwrap(), before);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn html_report_escapes_evidence() {
        let report = json!({
            "success": false,
            "source": "<app>",
            "tests": { "results": [{
                "name": "bad<test>",
                "passed": false,
                "elapsed_ms": 0,
                "stdout": "stdout.log",
                "stderr": "stderr.log",
                "error": "<script>test()</script>"
            }] },
            "baseline_error": "<script>baseline()</script>",
            "captures": [{
                "key": "bad<capture>",
                "comparison": {
                    "status": "error",
                    "error": "<script>capture()</script>"
                }
            }],
            "accessibility": {
                "semantic_target_count": 0,
                "named_semantic_target_count": 0,
                "actionable_without_name_count": 0
            },
            "source_mapped_changes": [],
            "diagnostics": { "items": [{ "rendered": "a < b & c" }] }
        });
        let html = render_html(&report);
        assert!(html.contains("&lt;app&gt;"));
        assert!(html.contains("a &lt; b &amp; c"));
        assert!(!html.contains("a < b"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;capture()&lt;/script&gt;"));
    }
}
