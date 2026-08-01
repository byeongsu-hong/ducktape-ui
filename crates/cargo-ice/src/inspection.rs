use crate::evidence::{
    CAPTURE_DIFF_ARTIFACT_KIND, REVIEW_SCHEMA_VERSION, validate_capture_manifest,
};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const INSPECT_TEST: &str = "__ice_agent_inspect";
const MAX_DIFF_PIXELS: usize = 16_777_216;
const IGNORED_MANIFEST_PATHS: &[&str] = &["/name", "/png", "/capture_source/statement"];

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
    let mut command = Command::new(&cargo);
    command
        .current_dir(root)
        .args([
            "test",
            "--package",
            &package,
            INSPECT_TEST,
            "--",
            "--nocapture",
        ])
        .env("ICE_AGENT_INSPECT_SOURCE", &source)
        .env("ICE_AGENT_INSPECT_ROOT", root)
        .env("ICE_AGENT_INSPECT_NAME", name)
        .env("ICE_AGENT_INSPECT_ARTIFACT_DIR", &artifact_dir)
        .env("ICE_AGENT_INSPECT_RESULT", &result_path);
    set_optional_env(&mut command, "ICE_AGENT_INSPECT_PRESET", &options.preset);
    set_optional_env(&mut command, "ICE_AGENT_INSPECT_THEME", &options.theme);
    set_optional_env(
        &mut command,
        "ICE_AGENT_INSPECT_SYSTEM_THEME",
        &options.system_theme,
    );
    set_optional_env(&mut command, "ICE_AGENT_INSPECT_LOCALE", &options.locale);
    set_optional_env(
        &mut command,
        "ICE_AGENT_INSPECT_PLATFORM",
        &options.platform,
    );
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
    Ok(())
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
        if flag == "--reduced-motion" {
            if options.reduced_motion {
                return Err("duplicate `--reduced-motion`".into());
            }
            options.reduced_motion = true;
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
    Ok(options)
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

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
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
            .is_none_or(|path| !IGNORED_MANIFEST_PATHS.contains(&path))
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

fn unit_f64(flag: &str, value: &str) -> Result<f64, String> {
    let value = nonnegative_f64(flag, value)?;
    if value <= 1.0 {
        Ok(value)
    } else {
        Err(format!("{flag} must be between 0 and 1"))
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

fn read_json(path: &Path) -> Result<Value, String> {
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
        (Some(baseline), Some(current)) if baseline == current => {}
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
    fn reports_structured_and_pixel_differences() {
        let baseline = json!({ "targets": [{ "geometry": { "x": 1.0 } }] });
        let current = json!({ "targets": [{ "geometry": { "x": 1.1 } }] });
        let mut differences = Vec::new();
        compare_json("", &baseline, &current, 0.01, &mut differences);
        assert_eq!(differences[0]["path"], "/targets/0/geometry/x");
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
