mod compat;
mod lsp;
mod schema;

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "ice") {
        args.remove(0);
    }
    let command = args.first().map(String::as_str).unwrap_or("check");
    let trailing = args.get(1..).unwrap_or_default();
    if !valid_command_args(command, trailing) {
        return Err(format!(
            "invalid arguments for `cargo ice {command}`; run `cargo ice help`"
        ));
    }
    let check_only = trailing == ["--check"];

    match command {
        "schema" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schema::document())
                    .map_err(|error| error.to_string())?
            );
            return Ok(());
        }
        "lsp" => return lsp::run_stdio(),
        "help" | "--help" | "-h" => {
            println!(
                "cargo ice <fmt [--check] | check | test [cargo-test args...] | clippy | compat | expand <file.ice> | schema | lsp>"
            );
            return Ok(());
        }
        _ => {}
    }

    let root = env::current_dir().map_err(|error| error.to_string())?;
    match command {
        "expand" => {
            let requested = args
                .get(1)
                .ok_or_else(|| "cargo ice expand <file.ice>".to_owned())?;
            let path = root.join(requested);
            let generated = ui_lang_core::compile_file(&path)
                .map_err(|error| error.render(&path.display().to_string()))?;
            print!("{}", generated.rust);
            return Ok(());
        }
        "fmt" | "check" | "test" | "clippy" | "compat" => {}
        other => return Err(format!("unknown cargo ice command `{other}`")),
    }
    let files = ice_files(&root)?;

    match command {
        "fmt" => {
            let roots = root_files(&files)?;
            if check_only {
                cargo(&["fmt", "--all", "--", "--check"])?;
            } else {
                cargo(&["fmt", "--all"])?;
            }
            let mut changed = Vec::new();
            for path in &files {
                let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
                let formatted = ui_lang_core::format_fragment(&source);
                if source != formatted {
                    changed.push(path.display().to_string());
                    if !check_only {
                        fs::write(path, formatted).map_err(|error| error.to_string())?;
                    }
                }
            }
            if check_only && !changed.is_empty() {
                return Err(format!("unformatted .ice files:\n{}", changed.join("\n")));
            }
            analyze(&roots)?;
            if check_only {
                println!("formatting is clean for {} .ice file(s)", files.len());
            } else {
                println!("formatted {} .ice file(s)", files.len());
            }
        }
        "check" => {
            let roots = root_files(&files)?;
            analyze(&roots)?;
            cargo(&["check", "--workspace"])?;
        }
        "test" => {
            let roots = root_files(&files)?;
            analyze(&roots)?;
            cargo(&["check", "--workspace", "--tests"])?;
            let mut cargo_args = vec!["test", "--workspace"];
            cargo_args.extend(trailing.iter().map(String::as_str));
            cargo(&cargo_args)?;
        }
        "clippy" => {
            let roots = root_files(&files)?;
            analyze(&roots)?;
            cargo(&["clippy", "--workspace", "--all-targets", "--no-deps"])?;
        }
        "compat" => {
            let roots = root_files(&files)?;
            analyze(&roots)?;
            compat::verify(&root)?;
            cargo(&["check", "-p", "iced-app", "--tests"])?;
            cargo(&["test", "-p", "iced-app"])?;
        }
        _ => unreachable!("commands were validated before scanning the workspace"),
    }
    Ok(())
}

fn valid_command_args(command: &str, trailing: &[String]) -> bool {
    match command {
        "fmt" => trailing.is_empty() || trailing == ["--check"],
        "expand" => trailing.len() == 1,
        "test" => true,
        "schema" | "lsp" | "help" | "--help" | "-h" | "check" | "clippy" | "compat" => {
            trailing.is_empty()
        }
        _ => true,
    }
}

fn analyze(files: &[PathBuf]) -> Result<(), String> {
    let mut documents = Vec::new();
    for path in files {
        let document = ui_lang_core::analyze_file(path)
            .map_err(|error| error.render(&path.display().to_string()))?;
        documents.push((path, document));
    }
    let reachable = documents
        .iter()
        .flat_map(|(_, document)| document.reachable_component_definitions())
        .filter_map(|range| Some((range.path.clone()?, range.line)))
        .collect::<std::collections::BTreeSet<_>>();
    let mut warnings = std::collections::BTreeMap::new();
    for (path, document) in &documents {
        for warning in document.warnings().iter().filter(|warning| {
            warning.code != "W001"
                || !warning
                    .path
                    .as_deref()
                    .is_some_and(|path| reachable.contains(&(PathBuf::from(path), warning.line)))
        }) {
            warnings
                .entry((
                    warning.code,
                    warning
                        .path
                        .clone()
                        .unwrap_or_else(|| path.display().to_string()),
                    warning.line,
                ))
                .or_insert_with(|| warning.render(&path.display().to_string()));
        }
    }
    for warning in warnings.into_values() {
        eprintln!("{warning}");
    }
    println!("checked {} .ice root graph(s)", files.len());
    Ok(())
}

fn root_files(files: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    for path in files {
        let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
        if ui_lang_core::source_is_app(&source) {
            roots.push(path.clone());
        }
    }
    if roots.is_empty() {
        return Err("no .ice file contains a top-level `app` or `daemon` declaration".into());
    }
    Ok(roots)
}

fn ice_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if !ignored_dir(&path) {
                    visit(&path, output)?;
                }
            } else if file_type.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("ice")
            {
                output.push(path);
            }
        }
        Ok(())
    }

    let mut output = Vec::new();
    visit(root, &mut output)?;
    output.sort();
    Ok(output)
}

fn ignored_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | ".worktree" | "target")
    ) || (path.file_name().and_then(|name| name.to_str()) == Some("cases")
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("tests"))
}

fn cargo(args: &[&str]) -> Result<(), String> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    if matches!(args.first(), Some(&"fmt" | &"test")) {
        let status = Command::new(cargo)
            .args(args)
            .status()
            .map_err(|error| error.to_string())?;
        return if status.success() {
            Ok(())
        } else {
            Err(format!("cargo {} failed", args.join(" ")))
        };
    }
    let mut child = Command::new(cargo)
        .args(args)
        .arg("--message-format=json-diagnostic-rendered-ansi")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let stdout = child.stdout.take().expect("piped cargo stdout");
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| error.to_string())?;
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            println!("{line}");
            continue;
        };
        if message["reason"] != "compiler-message" {
            continue;
        }
        let diagnostic = &message["message"];
        if let Some(rendered) = remap_compiler_diagnostic(diagnostic) {
            eprint!("{rendered}");
        } else if let Some(rendered) = diagnostic["rendered"].as_str() {
            eprint!("{rendered}");
        }
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed", args.join(" ")))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IceLocation {
    path: PathBuf,
    line: usize,
    column: usize,
}

fn remap_compiler_diagnostic(diagnostic: &serde_json::Value) -> Option<String> {
    let spans = diagnostic["spans"].as_array()?;
    let mapped = spans
        .iter()
        .filter_map(|span| {
            let file = span["file_name"].as_str()?;
            let line = span["line_start"].as_u64()? as usize;
            let generated_column = span["column_start"].as_u64()? as usize;
            let location = mapped_ice_location(Path::new(file), line)?;
            Some((span, location, file, line, generated_column))
        })
        .collect::<Vec<_>>();
    let primary = mapped
        .iter()
        .find(|(span, ..)| span["is_primary"].as_bool() == Some(true))?;
    let level = diagnostic["level"].as_str().unwrap_or("error");
    let code = diagnostic["code"]["code"]
        .as_str()
        .map(|code| format!("[{code}]"))
        .unwrap_or_default();
    let message = diagnostic["message"]
        .as_str()
        .unwrap_or("generated Rust error");
    let location = &primary.1;
    let mut output = format!(
        "{level}{code} {}:{}:{}: {message}\n",
        location.path.display(),
        location.line,
        location.column
    );
    push_source_excerpt(&mut output, location);
    let mut related = std::collections::BTreeSet::new();
    for (span, location, ..) in &mapped {
        if location == &primary.1 {
            continue;
        }
        let label = span["label"]
            .as_str()
            .unwrap_or("related generated expression");
        related.insert(format!(
            "related: {}:{}:{}: {label}",
            location.path.display(),
            location.line,
            location.column
        ));
    }
    for line in related {
        output.push_str(&line);
        output.push('\n');
    }
    if let Some(children) = diagnostic["children"].as_array() {
        for child in children {
            let level = child["level"].as_str().unwrap_or("note");
            if let Some(message) = child["message"].as_str() {
                output.push_str(&format!("{level}: {message}\n"));
            }
        }
    }
    output.push_str(&format!(
        "note: generated Rust location: {}:{}:{}\n",
        primary.2, primary.3, primary.4
    ));
    Some(output)
}

fn push_source_excerpt(output: &mut String, location: &IceLocation) {
    let Ok(source) = fs::read_to_string(&location.path) else {
        return;
    };
    let Some(line) = source.lines().nth(location.line.saturating_sub(1)) else {
        return;
    };
    let gutter = location.line.to_string();
    let column = location.column.saturating_sub(1).min(line.chars().count());
    output.push_str(&format!(
        "{gutter} | {line}\n{} | {}^\n",
        " ".repeat(gutter.len()),
        " ".repeat(column)
    ));
}

fn mapped_ice_location(path: &Path, generated_line: usize) -> Option<IceLocation> {
    let generated = fs::read_to_string(path).ok()?;
    mapped_ice_location_in(&generated, generated_line)
}

fn mapped_ice_location_in(generated: &str, generated_line: usize) -> Option<IceLocation> {
    let mut stack = Vec::new();
    for (index, line) in generated.lines().enumerate() {
        if index + 1 > generated_line {
            break;
        }
        if line == "// __ICE_SOURCE_END" {
            stack.pop();
            continue;
        }
        if let Some(location) = parse_source_marker(line) {
            stack.push(location);
        }
    }
    stack.pop()
}

fn parse_source_marker(line: &str) -> Option<IceLocation> {
    let location = line.strip_prefix("// __ICE_SOURCE ")?;
    let mut parts = location.split_ascii_whitespace();
    let line = parts.next()?.parse().ok()?;
    let column = parts.next()?.parse().ok()?;
    let path = decode_hex(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(IceLocation {
        path: PathBuf::from(path),
        line,
        column,
    })
}

fn decode_hex(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|digits| {
            let digits = std::str::from_utf8(digits).ok()?;
            u8::from_str_radix(digits, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::{
        IceLocation, ice_files, ignored_dir, mapped_ice_location_in, remap_compiler_diagnostic,
        root_files, valid_command_args,
    };
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ignores_build_and_fixture_directories() {
        assert!(ignored_dir(Path::new("target")));
        assert!(ignored_dir(Path::new(".worktree")));
        assert!(ignored_dir(Path::new("tests/cases")));
        assert!(!ignored_dir(Path::new("src/cases")));
    }

    #[test]
    fn rejects_unknown_command_arguments() {
        assert!(valid_command_args("fmt", &[]));
        assert!(valid_command_args("fmt", &["--check".into()]));
        assert!(!valid_command_args("fmt", &["--chek".into()]));
        assert!(!valid_command_args("check", &["extra".into()]));
        assert!(valid_command_args("test", &[]));
        assert!(valid_command_args("test", &["render_contract".into()]));
        assert!(valid_command_args(
            "test",
            &["render_contract".into(), "--".into(), "--nocapture".into()]
        ));
        assert!(valid_command_args("expand", &["app.ice".into()]));
        assert!(!valid_command_args("expand", &[]));
    }

    #[test]
    fn missing_root_names_both_root_kinds() {
        assert!(root_files(&[]).unwrap_err().contains("`app` or `daemon`"));
    }

    #[test]
    fn maps_nested_generated_regions_to_the_innermost_ice_source() {
        let generated = "boilerplate\n// __ICE_SOURCE 4 1 6170702e696365\nouter\n// __ICE_SOURCE 9 3 667261676d656e742e696365\ninner\n// __ICE_SOURCE_END\nouter again\n// __ICE_SOURCE_END\nboilerplate\n";
        assert_eq!(
            mapped_ice_location_in(generated, 5),
            Some(IceLocation {
                path: "fragment.ice".into(),
                line: 9,
                column: 3,
            })
        );
        assert_eq!(
            mapped_ice_location_in(generated, 7),
            Some(IceLocation {
                path: "app.ice".into(),
                line: 4,
                column: 1,
            })
        );
        assert_eq!(mapped_ice_location_in(generated, 9), None);
    }

    #[test]
    fn renders_rustc_diagnostics_against_ice_syntax() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "cargo-ice-source-map-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let source = directory.join("app.ice");
        let generated = directory.join("generated.rs");
        std::fs::write(&source, "app Demo\nview\n  text moved_value\n").unwrap();
        let encoded = source
            .display()
            .to_string()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        std::fs::write(
            &generated,
            format!(
                "fn generated() {{\n// __ICE_SOURCE 3 1 {encoded}\nlet moved_value = String::new();\nlet consumed = moved_value;\nuse_again(moved_value);\n// __ICE_SOURCE_END\n}}\n"
            ),
        )
        .unwrap();
        let diagnostic = serde_json::json!({
            "message": "use of moved value: `moved_value`",
            "code": { "code": "E0382" },
            "level": "error",
            "spans": [{
                "file_name": generated.display().to_string(),
                "line_start": 5,
                "column_start": 11,
                "is_primary": true,
                "label": "value used here after move"
            }],
            "children": [{ "level": "note", "message": "move occurs because the value is not Copy" }],
        });
        let rendered = remap_compiler_diagnostic(&diagnostic).unwrap();
        assert!(rendered.contains(&format!("{}:3:1", source.display())));
        assert!(rendered.contains("3 |   text moved_value"));
        assert!(rendered.contains("error[E0382]"));
        assert!(rendered.contains("generated Rust location"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_symlinks() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cargo-ice-files-{nonce}"));
        std::fs::create_dir(&root).unwrap();
        let app = root.join("app.ice");
        std::fs::write(&app, "app Example").unwrap();
        std::os::unix::fs::symlink(&root, root.join("loop")).unwrap();
        std::os::unix::fs::symlink(&app, root.join("linked.ice")).unwrap();

        assert_eq!(ice_files(&root).unwrap(), [app]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
