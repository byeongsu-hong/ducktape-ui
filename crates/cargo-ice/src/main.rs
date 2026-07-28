mod compat;
mod lsp;
mod schema;

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;
use ui_lang_core::{
    LivePlan, LiveReloadDecision, evaluate_live_reload, live_plan, live_program_contract,
};

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
                "cargo ice <fmt [--check] | check | test [cargo-test args...] | clippy | compat | expand <file.ice> | dev <file.ice> [-- cargo-build-args... [-- app-args...]] | schema | lsp>"
            );
            return Ok(());
        }
        _ => {}
    }

    let root = env::current_dir().map_err(|error| error.to_string())?;
    match command {
        "dev" => {
            let requested = trailing.first().ok_or_else(|| {
                "cargo ice dev <file.ice> [-- cargo-build-args... [-- app-args...]]".to_owned()
            })?;
            let cargo_args = trailing
                .get(2..)
                .filter(|_| trailing.get(1).is_some_and(|arg| arg == "--"))
                .unwrap_or_default();
            return run_dev(&root, &root.join(requested), cargo_args);
        }
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
            analyze(&roots, &files)?;
            if check_only {
                println!("formatting is clean for {} .ice file(s)", files.len());
            } else {
                println!("formatted {} .ice file(s)", files.len());
            }
        }
        "check" => {
            let roots = root_files(&files)?;
            analyze(&roots, &files)?;
            cargo(&["check", "--workspace"])?;
        }
        "test" => {
            let roots = root_files(&files)?;
            analyze(&roots, &files)?;
            cargo(&["check", "--workspace", "--tests"])?;
            let mut cargo_args = vec!["test", "--workspace"];
            cargo_args.extend(trailing.iter().map(String::as_str));
            cargo(&cargo_args)?;
        }
        "clippy" => {
            let roots = root_files(&files)?;
            analyze(&roots, &files)?;
            cargo(&["clippy", "--workspace", "--all-targets", "--no-deps"])?;
        }
        "compat" => {
            let roots = root_files(&files)?;
            analyze(&roots, &files)?;
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
        "dev" => {
            trailing.len() == 1
                || trailing.len() >= 2 && trailing.get(1).is_some_and(|arg| arg == "--")
        }
        "test" => true,
        "schema" | "lsp" | "help" | "--help" | "-h" | "check" | "clippy" | "compat" => {
            trailing.is_empty()
        }
        _ => true,
    }
}

struct DevCompilation {
    plan: LivePlan,
    dependencies: Vec<PathBuf>,
    lowering_error: Option<String>,
}

fn run_dev(root: &Path, source: &Path, cargo_args: &[String]) -> Result<(), String> {
    let source = source
        .canonicalize()
        .map_err(|error| format!("cannot open {}: {error}", source.display()))?;
    let plan_dir = root.join("target/ice-live");
    fs::create_dir_all(&plan_dir).map_err(|error| error.to_string())?;
    let plan_path = plan_dir.join(format!(
        "{}-{}.json",
        source
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("app"),
        std::process::id()
    ));
    let mut revision = 1;
    let mut current = compile_dev(&source, revision)?;
    if let Some(error) = &current.lowering_error {
        eprintln!("ice live: {error}; using build-and-restart fallback");
    }
    write_plan(&plan_path, &current.plan)?;
    let executable = cargo_build(root, cargo_args)?
        .ok_or_else(|| "ice live: initial app build failed".to_owned())?;
    let mut app = ChildGuard::spawn(root, &executable, runtime_args(cargo_args), &plan_path)?;
    let mut ice_stamp = ice_source_stamp(root, &current.dependencies)?;
    let mut rust_stamp = rust_source_stamp(root)?;
    println!(
        "ice live: watching {} Ice file(s); plan {}",
        ice_stamp.len(),
        plan_path.display()
    );

    loop {
        if let Some(status) = app.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("ice live: app exited with {status}"))
            };
        }
        thread::sleep(Duration::from_millis(100));
        let next_ice_stamp = ice_source_stamp(root, &current.dependencies)?;
        let next_rust_stamp = rust_source_stamp(root)?;
        let rust_changed = next_rust_stamp != rust_stamp;
        if next_ice_stamp == ice_stamp && !rust_changed {
            continue;
        }
        thread::sleep(Duration::from_millis(50));
        revision = revision.wrapping_add(1);
        let next = match compile_dev(&source, revision) {
            Ok(next) => next,
            Err(error) => {
                eprintln!("{error}");
                ice_stamp = ice_source_stamp(root, &current.dependencies)?;
                rust_stamp = rust_source_stamp(root)?;
                continue;
            }
        };
        let decision = evaluate_live_reload(&current.plan.contract, &next.plan.contract);
        let can_reload = !rust_changed
            && next.lowering_error.is_none()
            && matches!(decision, LiveReloadDecision::Reload { .. });
        if can_reload {
            write_plan(&plan_path, &next.plan)?;
            println!("ice live: published revision {revision}");
            current = next;
            ice_stamp = ice_source_stamp(root, &current.dependencies)?;
            rust_stamp = next_rust_stamp;
            continue;
        }

        if rust_changed {
            eprintln!("ice live: Rust or Cargo input changed; rebuilding with the app open");
        } else if let Some(error) = &next.lowering_error {
            eprintln!("ice live: {error}; rebuilding with the app open");
        } else if let LiveReloadDecision::RestartRequired { reasons } = &decision {
            eprintln!(
                "ice live: revision {revision} requires restart: {}",
                reasons
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if let Some(executable) = cargo_build(root, cargo_args)? {
            app.restart(
                root,
                &executable,
                runtime_args(cargo_args),
                &plan_path,
                &next.plan,
            )?;
            println!("ice live: restarted on revision {revision}");
            current = next;
        } else {
            eprintln!("ice live: build failed; keeping the current app open");
        }
        ice_stamp = ice_source_stamp(root, &current.dependencies)?;
        rust_stamp = rust_source_stamp(root)?;
    }
}

fn compile_dev(source: &Path, revision: u64) -> Result<DevCompilation, String> {
    let analysis = ui_lang_core::analyze_file_graph(source)
        .map_err(|error| error.render(&source.display().to_string()))?;
    let contract = live_program_contract(&analysis.document);
    match live_plan(&analysis.document, revision) {
        Ok(plan) => Ok(DevCompilation {
            plan,
            dependencies: analysis.dependencies,
            lowering_error: None,
        }),
        Err(error) => Ok(DevCompilation {
            plan: LivePlan {
                revision,
                contract,
                view: None,
            },
            dependencies: analysis.dependencies,
            lowering_error: Some(error.to_string()),
        }),
    }
}

fn write_plan(path: &Path, plan: &LivePlan) -> Result<(), String> {
    let payload = serde_json::to_vec(plan).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("{}.next", std::process::id()));
    fs::write(&temporary, payload).map_err(|error| error.to_string())?;
    #[cfg(target_os = "windows")]
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn stamp_files(files: &[PathBuf]) -> Result<Vec<(PathBuf, u64)>, String> {
    let mut files = files.to_vec();
    files.sort();
    files.dedup();
    files
        .into_iter()
        .map(|path| {
            let hash = match fs::read(&path) {
                Ok(bytes) => stable_hash(&bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
                Err(error) => return Err(error.to_string()),
            };
            Ok((path, hash))
        })
        .collect()
}

fn ice_source_stamp(root: &Path, dependencies: &[PathBuf]) -> Result<Vec<(PathBuf, u64)>, String> {
    let mut files = dependencies.to_vec();
    files.extend(ice_files(root)?);
    stamp_files(&files)
}

fn rust_source_stamp(root: &Path) -> Result<Vec<(PathBuf, u64)>, String> {
    let mut files = Vec::new();
    visit_rust_sources(root, &mut files)?;
    stamp_files(&files)
}

fn visit_rust_sources(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if !ignored_dir(&path)
                && path.file_name().and_then(|name| name.to_str()) != Some("vendor")
            {
                visit_rust_sources(&path, output)?;
            }
        } else if file_type.is_file()
            && (path.extension().and_then(|extension| extension.to_str()) == Some("rs")
                || matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("Cargo.toml" | "Cargo.lock")
                ))
        {
            output.push(path);
        }
    }
    Ok(())
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn cargo_build(root: &Path, cargo_args: &[String]) -> Result<Option<PathBuf>, String> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let build_args = cargo_args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .collect::<Vec<_>>();
    let mut child = Command::new(cargo)
        .arg("build")
        .args(build_args)
        .arg("--message-format=json-render-diagnostics")
        .current_dir(root)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let stdout = child.stdout.take().expect("piped cargo stdout");
    let mut executables = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| error.to_string())?;
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            println!("{line}");
            continue;
        };
        if message["reason"] == "compiler-artifact"
            && message["target"]["kind"].as_array().is_some_and(|kinds| {
                kinds
                    .iter()
                    .any(|kind| matches!(kind.as_str(), Some("bin" | "example")))
            })
            && let Some(executable) = message["executable"].as_str()
        {
            executables.push(PathBuf::from(executable));
        }
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    if !status.success() {
        return Ok(None);
    }
    executables.sort();
    executables.dedup();
    match executables.as_slice() {
        [executable] => Ok(Some(executable.clone())),
        [] => Err("ice live: cargo build produced no runnable binary".into()),
        _ => Err(format!(
            "ice live: cargo build produced multiple binaries; select one with `--bin`: {}",
            executables
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn runtime_args(cargo_args: &[String]) -> &[String] {
    cargo_args
        .iter()
        .position(|arg| arg == "--")
        .and_then(|separator| cargo_args.get(separator + 1..))
        .unwrap_or_default()
}

struct ChildGuard(Child);

impl ChildGuard {
    fn spawn(root: &Path, executable: &Path, args: &[String], plan: &Path) -> Result<Self, String> {
        Command::new(executable)
            .args(args)
            .env("ICE_LIVE_PLAN", plan)
            .current_dir(root)
            .spawn()
            .map(Self)
            .map_err(|error| error.to_string())
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.0.try_wait().map_err(|error| error.to_string())
    }

    fn restart(
        &mut self,
        root: &Path,
        executable: &Path,
        args: &[String],
        plan_path: &Path,
        plan: &LivePlan,
    ) -> Result<(), String> {
        self.0.kill().map_err(|error| error.to_string())?;
        self.0.wait().map_err(|error| error.to_string())?;
        write_plan(plan_path, plan)?;
        *self = Self::spawn(root, executable, args, plan_path)?;
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn analyze(files: &[PathBuf], all_files: &[PathBuf]) -> Result<(), String> {
    let mut documents = Vec::new();
    let mut dependencies = std::collections::BTreeSet::new();
    for path in files {
        let analysis = ui_lang_core::analyze_file_graph(path)
            .map_err(|error| error.render(&path.display().to_string()))?;
        dependencies.extend(analysis.dependencies);
        documents.push((path, analysis.document));
    }
    let reachable = documents
        .iter()
        .flat_map(|(_, document)| document.reachable_component_definitions())
        .filter_map(|range| Some((range.path.clone()?, range.line)))
        .collect::<std::collections::BTreeSet<_>>();
    let reachable_handlers = documents
        .iter()
        .flat_map(|(_, document)| document.reachable_handler_definitions())
        .filter_map(|range| Some((range.path.clone()?, range.line)))
        .collect::<std::collections::BTreeSet<_>>();
    let mut warnings = std::collections::BTreeMap::new();
    for (path, document) in &documents {
        for warning in document
            .warnings()
            .iter()
            .filter(|warning| {
                warning.code != "W001"
                    || !warning.path.as_deref().is_some_and(|path| {
                        reachable.contains(&(PathBuf::from(path), warning.line))
                    })
            })
            .filter(|warning| {
                warning.code != "W005"
                    || !warning.path.as_deref().is_some_and(|path| {
                        reachable_handlers.contains(&(PathBuf::from(path), warning.line))
                    })
            })
        {
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
    for path in orphan_ice_files(all_files, &dependencies) {
        eprintln!(
            "warning[W010] {}:1:1: .ice source is outside every app, daemon, and test import graph\n  = help: import this file from a reachable root or remove it",
            path.display()
        );
    }
    println!("checked {} .ice root graph(s)", files.len());
    Ok(())
}

fn orphan_ice_files<'a>(
    files: &'a [PathBuf],
    dependencies: &std::collections::BTreeSet<PathBuf>,
) -> Vec<&'a PathBuf> {
    files
        .iter()
        .filter(|path| !dependencies.contains(*path))
        .collect()
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
    let mut source_maps = GeneratedSourceMaps::new();
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
        if let Some(rendered) = remap_compiler_diagnostic_with_maps(diagnostic, &mut source_maps) {
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

#[derive(Clone, Debug)]
struct GeneratedSourceMap(Vec<Option<IceLocation>>);

type GeneratedSourceMaps = std::collections::HashMap<PathBuf, GeneratedSourceMap>;

impl GeneratedSourceMap {
    fn parse(generated: &str) -> Self {
        let mut stack = Vec::new();
        let mut locations = Vec::new();
        for line in generated.lines() {
            if line == "// __ICE_SOURCE_END" {
                stack.pop();
            } else if let Some(location) = parse_source_marker(line) {
                stack.push(location);
            }
            locations.push(stack.last().cloned());
        }
        Self(locations)
    }

    fn location(&self, generated_line: usize) -> Option<IceLocation> {
        self.0.get(generated_line.checked_sub(1)?)?.clone()
    }
}

#[cfg(test)]
fn remap_compiler_diagnostic(diagnostic: &serde_json::Value) -> Option<String> {
    remap_compiler_diagnostic_with_maps(diagnostic, &mut GeneratedSourceMaps::new())
}

fn remap_compiler_diagnostic_with_maps(
    diagnostic: &serde_json::Value,
    source_maps: &mut GeneratedSourceMaps,
) -> Option<String> {
    let spans = diagnostic["spans"].as_array()?;
    let mapped = spans
        .iter()
        .filter_map(|span| {
            let file = span["file_name"].as_str()?;
            let line = span["line_start"].as_u64()? as usize;
            let generated_column = span["column_start"].as_u64()? as usize;
            let location = mapped_ice_location(source_maps, Path::new(file), line)?;
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

fn mapped_ice_location(
    source_maps: &mut GeneratedSourceMaps,
    path: &Path,
    generated_line: usize,
) -> Option<IceLocation> {
    if !source_maps.contains_key(path) {
        let generated = fs::read_to_string(path).ok()?;
        source_maps.insert(path.to_owned(), GeneratedSourceMap::parse(&generated));
    }
    source_maps.get(path)?.location(generated_line)
}

#[cfg(test)]
fn mapped_ice_location_in(generated: &str, generated_line: usize) -> Option<IceLocation> {
    GeneratedSourceMap::parse(generated).location(generated_line)
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
        IceLocation, ice_files, ignored_dir, mapped_ice_location_in, orphan_ice_files,
        remap_compiler_diagnostic, root_files, valid_command_args,
    };
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
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
        assert!(valid_command_args("dev", &["app.ice".into()]));
        assert!(valid_command_args(
            "dev",
            &["app.ice".into(), "--".into(), "-p".into(), "demo".into()]
        ));
        assert!(!valid_command_args(
            "dev",
            &["app.ice".into(), "-p".into(), "demo".into()]
        ));
    }

    #[test]
    fn missing_root_names_both_root_kinds() {
        assert!(root_files(&[]).unwrap_err().contains("`app` or `daemon`"));
    }

    #[test]
    fn reports_ice_files_outside_every_root_graph() {
        let files = ["app.ice", "used.ice", "orphan.ice"]
            .map(PathBuf::from)
            .to_vec();
        let dependencies = [PathBuf::from("app.ice"), PathBuf::from("used.ice")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            orphan_ice_files(&files, &dependencies),
            [&PathBuf::from("orphan.ice")]
        );
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
