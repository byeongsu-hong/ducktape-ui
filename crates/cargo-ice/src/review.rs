use crate::inspection::{
    DiffThresholds, compare_capture_manifests, containing_package, source_output_name,
};
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
    let analysis = match ui_lang_core::analyze_file_graph(&source) {
        Ok(analysis) => analysis,
        Err(error) => return analysis_failure(root, &source, &output, error),
    };
    let selected = selected_tests(&analysis.document.tests, &options.tests)?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let package = options
        .package
        .clone()
        .map(Ok)
        .unwrap_or_else(|| containing_package(root, &source, &cargo))?;
    let run_id = run_id()?;
    let artifact_dir = output.join("artifacts").join(&run_id);
    let log_dir = output.join("logs").join(&run_id);
    let diff_dir = output.join("diffs").join(&run_id);
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
        let stem = safe_component(test);
        let stdout_path = log_dir.join(format!("{stem}.stdout.log"));
        let stderr_path = log_dir.join(format!("{stem}.stderr.log"));
        fs::write(&stdout_path, &output_result.stdout).map_err(|error| error.to_string())?;
        fs::write(&stderr_path, &output_result.stderr).map_err(|error| error.to_string())?;
        test_results.push(json!({
            "name": test,
            "passed": output_result.status.success(),
            "status_code": output_result.status.code(),
            "elapsed_ms": elapsed_ms,
            "stdout": relative_path(&output, &stdout_path),
            "stderr": relative_path(&output, &stderr_path),
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
                    || relative_path(root, &source),
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
        &json!({ "schema_version": 1, "diagnostics": diagnostics }),
    )?;

    let current = capture_index(&artifact_dir)?;
    let baseline = options
        .baseline
        .as_ref()
        .map(|path| baseline_index(&root.join(path)))
        .transpose()?;
    let mut captures = Vec::new();
    let mut source_changes = Vec::new();
    let mut compared_baselines = BTreeSet::new();
    for (key, current_path) in &current {
        let manifest = read_json(current_path)?;
        let png = manifest_png(current_path, &manifest)?;
        let mut capture = json!({
            "key": key,
            "name": manifest["name"],
            "manifest": relative_path(&output, current_path),
            "png": relative_path(&output, &png),
            "comparison": { "status": "not_requested" },
        });
        if let Some(baseline) = &baseline {
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
                        for difference in report["manifest"]["differences"]
                            .as_array()
                            .into_iter()
                            .flatten()
                        {
                            source_changes.push(source_change(key, difference, &manifest));
                        }
                        capture["comparison"] = json!({
                            "status": if report["matches"] == true { "match" } else { "changed" },
                            "matches": report["matches"],
                            "baseline_manifest": baseline_path,
                            "report": relative_path(&output, &destination.join("report.json")),
                            "diff_png": relative_path(&output, &destination.join("diff.png")),
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
    }
    if let Some(baseline) = &baseline {
        for key in baseline.keys() {
            if !compared_baselines.contains(key) && !current.contains_key(key) {
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

    let accessibility = accessibility_summary(current.values())?;
    let tests_passed = test_results.iter().all(|test| test["passed"] == true);
    let comparisons_passed = captures.iter().all(|capture| {
        matches!(
            capture["comparison"]["status"].as_str(),
            Some("not_requested" | "match")
        )
    });
    let success = tests_passed && comparisons_passed;
    let report_path = output.join("report.json");
    let html_path = output.join("report.html");
    let report = json!({
        "schema_version": 1,
        "success": success,
        "source": relative_path(root, &source),
        "package": package,
        "run_id": run_id,
        "tests": {
            "selected": selected,
            "passed": test_results.iter().filter(|test| test["passed"] == true).count(),
            "failed": test_results.iter().filter(|test| test["passed"] != true).count(),
            "results": test_results,
        },
        "diagnostics": {
            "path": relative_path(&output, &diagnostics_path),
            "warning_count": diagnostics.len(),
            "error_count": 0,
            "items": diagnostics,
        },
        "captures": captures,
        "source_mapped_changes": source_changes,
        "accessibility": accessibility,
        "baseline": options.baseline,
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
        &json!({ "schema_version": 1, "diagnostics": [&diagnostic] }),
    )?;
    let report_path = output.join("report.json");
    let html_path = output.join("report.html");
    let report = json!({
        "schema_version": 1,
        "success": false,
        "source": relative_path(root, source),
        "package": null,
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

fn baseline_index(path: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let report_path = if path.is_dir() {
        path.join("report.json")
    } else {
        path.to_owned()
    };
    if report_path.is_file() {
        let report = read_json(&report_path)?;
        let directory = report_path.parent().unwrap_or_else(|| Path::new("."));
        let mut index = BTreeMap::new();
        for capture in report["captures"].as_array().into_iter().flatten() {
            let Some(key) = capture["key"].as_str() else {
                continue;
            };
            let Some(manifest) = capture["manifest"].as_str() else {
                continue;
            };
            index.insert(key.to_owned(), directory.join(manifest));
        }
        return Ok(index);
    }
    capture_index(path)
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

fn accessibility_summary<'a>(
    manifests: impl Iterator<Item = &'a PathBuf>,
) -> Result<Value, String> {
    let mut targets = 0_u64;
    let mut semantic = 0_u64;
    let mut named = 0_u64;
    let mut actionable = 0_u64;
    let mut actionable_without_name = Vec::new();
    let mut roles = BTreeMap::<String, u64>::new();
    for path in manifests {
        let manifest = read_json(path)?;
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
    Ok(json!({
        "target_count": targets,
        "semantic_target_count": semantic,
        "named_semantic_target_count": named,
        "actionable_target_count": actionable,
        "actionable_without_name_count": actionable_without_name.len(),
        "actionable_without_name": actionable_without_name,
        "roles": roles,
    }))
}

fn source_change(capture: &str, difference: &Value, manifest: &Value) -> Value {
    let path = difference["path"].as_str().unwrap_or_default();
    let source = path
        .strip_prefix("/targets/")
        .and_then(|path| path.split('/').next())
        .and_then(|index| index.parse::<usize>().ok())
        .and_then(|index| manifest["targets"].get(index))
        .and_then(|target| target.get("source"))
        .filter(|source| !source.is_null())
        .unwrap_or(&manifest["capture_source"]);
    json!({
        "capture": capture,
        "json_path": path,
        "source": source,
        "baseline": difference["baseline"],
        "current": difference["current"],
    })
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
        "<h2>Tests</h2><table><tr><th>Name</th><th>Result</th><th>Time</th><th>Logs</th></tr>",
    );
    for test in report["tests"]["results"].as_array().into_iter().flatten() {
        let passed = test["passed"] == true;
        let _ = write!(
            html,
            "<tr><td><code>{}</code></td><td class=\"{}\">{}</td><td>{} ms</td><td><a href=\"{}\">stdout</a> · <a href=\"{}\">stderr</a></td></tr>",
            html_escape(test["name"].as_str().unwrap_or_default()),
            if passed { "ok" } else { "bad" },
            if passed { "pass" } else { "fail" },
            test["elapsed_ms"],
            html_escape(test["stdout"].as_str().unwrap_or_default()),
            html_escape(test["stderr"].as_str().unwrap_or_default()),
        );
    }
    html.push_str("</table><h2>Captures</h2><div class=\"captures\">");
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

fn manifest_png(path: &Path, manifest: &Value) -> Result<PathBuf, String> {
    let png = manifest["png"]
        .as_str()
        .ok_or_else(|| format!("{} omits string field `png`", path.display()))?;
    let png = path.parent().unwrap_or_else(|| Path::new(".")).join(png);
    if png.is_file() {
        Ok(png)
    } else {
        Err(format!("capture PNG is missing at {}", png.display()))
    }
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
        let paths = [manifest_path.clone()];
        let summary = accessibility_summary(paths.iter()).unwrap();
        assert_eq!(summary["semantic_target_count"], 1);
        assert_eq!(summary["actionable_without_name_count"], 1);
        assert_eq!(summary["roles"]["button"], 1);
        let change = source_change(
            "test/capture.json",
            &json!({ "path": "/targets/0/geometry/x", "baseline": 1, "current": 2 }),
            &manifest,
        );
        assert_eq!(change["source"]["line"], 8);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn previous_review_reports_resolve_stable_capture_keys() {
        let fixture = fixture("baseline");
        fs::create_dir_all(fixture.join("artifacts/run/test")).unwrap();
        fs::write(fixture.join("artifacts/run/test/wide.json"), "{}").unwrap();
        write_json(
            &fixture.join("report.json"),
            &json!({
                "captures": [{
                    "key": "test/wide.json",
                    "manifest": "artifacts/run/test/wide.json"
                }]
            }),
        )
        .unwrap();
        let index = baseline_index(&fixture).unwrap();
        assert_eq!(
            index["test/wide.json"],
            fixture.join("artifacts/run/test/wide.json")
        );
        fs::remove_dir_all(fixture).unwrap();
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
        let failure = analysis_failure(&fixture, &source, &output, error).unwrap_err();
        assert!(failure.contains("review evidence failed"));
        let report = read_json(&output.join("report.json")).unwrap();
        assert_eq!(report["success"], false);
        assert_eq!(report["diagnostics"]["error_count"], 1);
        assert!(output.join("report.html").is_file());
        assert!(output.join("diagnostics.json").is_file());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn html_report_escapes_evidence() {
        let report = json!({
            "success": false,
            "source": "<app>",
            "tests": { "results": [] },
            "captures": [],
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
    }
}
