mod compat;
mod lsp;
mod schema;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

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
    for path in files {
        ui_lang_core::analyze_file(path)
            .map_err(|error| error.render(&path.display().to_string()))?;
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
    let status = Command::new(cargo)
        .args(args)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed", args.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::{ice_files, ignored_dir, root_files, valid_command_args};
    use std::path::Path;

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
