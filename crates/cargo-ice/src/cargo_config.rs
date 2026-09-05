use std::env;
use std::path::{Component, Path, PathBuf};

pub(crate) fn files(invocation_cwd: &Path, cargo_args: &[String]) -> Vec<PathBuf> {
    let invocation_cwd = if invocation_cwd.is_absolute() {
        invocation_cwd.to_owned()
    } else {
        env::current_dir()
            .map(|current| current.join(invocation_cwd))
            .unwrap_or_else(|_| invocation_cwd.to_owned())
    };
    files_with_home(
        &invocation_cwd,
        cargo_args,
        cargo_home(&invocation_cwd).as_deref(),
    )
}

fn files_with_home(
    invocation_cwd: &Path,
    cargo_args: &[String],
    cargo_home: Option<&Path>,
) -> Vec<PathBuf> {
    let invocation_cwd = normalize_path(invocation_cwd);
    let mut files = discovered_files(&invocation_cwd);
    files.extend(explicit_files(&invocation_cwd, cargo_args));
    if let Some(cargo_home) = cargo_home {
        let cargo_home = absolute_path(&invocation_cwd, cargo_home);
        files.push(cargo_home.join("config.toml"));
        files.push(cargo_home.join("config"));
    }
    files.sort();
    files.dedup();
    files
}

fn discovered_files(invocation_cwd: &Path) -> Vec<PathBuf> {
    invocation_cwd
        .ancestors()
        .flat_map(|ancestor| {
            let cargo = ancestor.join(".cargo");
            [cargo.join("config.toml"), cargo.join("config")]
        })
        .collect()
}

fn explicit_files(invocation_cwd: &Path, cargo_args: &[String]) -> Vec<PathBuf> {
    let mut args = cargo_args
        .iter()
        .take_while(|argument| argument.as_str() != "--");
    let mut files = Vec::new();
    while let Some(argument) = args.next() {
        let value = if argument.as_str() == "--config" {
            args.next().map(|value| value.as_str())
        } else {
            argument.strip_prefix("--config=")
        };
        if let Some(value) = value {
            let path = absolute_path(invocation_cwd, Path::new(value));
            if !value.contains('=') || path.exists() {
                files.push(path);
            }
        }
    }
    files
}

fn cargo_home(invocation_cwd: &Path) -> Option<PathBuf> {
    if let Some(configured) = env::var_os("CARGO_HOME") {
        return Some(absolute_path(invocation_cwd, Path::new(&configured)));
    }
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| absolute_path(invocation_cwd, &home).join(".cargo"))
}

fn absolute_path(base: &Path, path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    normalize_path(&path)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{explicit_files, files_with_home};
    use std::path::{Path, PathBuf};

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_explicit_arguments_stream_without_scratch() {
        const REQUESTS: u64 = 100;

        let cwd = Path::new("/workspace/app");
        let arguments = ["--release".to_owned()];
        assert!(explicit_files(cwd, &arguments).is_empty());

        let measured = crate::allocation::clean_window(0, || {
            for _ in 0..REQUESTS {
                assert!(
                    std::hint::black_box(explicit_files(cwd, std::hint::black_box(&arguments)))
                        .is_empty()
                );
            }
        });

        assert_eq!(measured, (0, 0), "argument scan allocations: {measured:?}");
        eprintln!(
            "{REQUESTS} explicit argument scans: {} heap blocks / {} bytes",
            measured.0, measured.1
        );
    }

    #[test]
    fn explicit_argument_scan_preserves_separator_and_value_boundaries() {
        let cwd = Path::new("/workspace/app");

        assert_eq!(
            explicit_files(
                cwd,
                &[
                    "--config=before.toml".to_owned(),
                    "--".to_owned(),
                    "--config=after.toml".to_owned(),
                ],
            ),
            [cwd.join("before.toml")]
        );
        assert!(
            explicit_files(
                cwd,
                &[
                    "--config".to_owned(),
                    "--".to_owned(),
                    "--config=after.toml".to_owned(),
                ],
            )
            .is_empty()
        );
        assert_eq!(
            explicit_files(cwd, &["--config".to_owned(), "--release".to_owned()]),
            [cwd.join("--release")]
        );
    }

    #[test]
    fn collects_both_explicit_config_path_forms_relative_to_the_invocation_cwd() {
        let cwd = Path::new("/workspace/app/nested");
        let arguments = [
            "--config".to_owned(),
            "../first.toml".to_owned(),
            "--config=./second.toml".to_owned(),
        ];
        assert_eq!(
            explicit_files(cwd, &arguments),
            [
                PathBuf::from("/workspace/app/first.toml"),
                PathBuf::from("/workspace/app/nested/second.toml"),
            ]
        );
    }

    #[test]
    fn inline_config_overrides_are_not_treated_as_paths() {
        let cwd = Path::new("/workspace/app");
        let arguments = [
            "--config".to_owned(),
            "build.rustflags = [\"--cfg\", \"ice\"]".to_owned(),
            "--config=net.git-fetch-with-cli=true".to_owned(),
        ];
        assert!(explicit_files(cwd, &arguments).is_empty());
    }

    #[test]
    fn an_existing_config_path_with_an_equals_sign_matches_cargo_file_detection() {
        let fixture = tempfile::tempdir().unwrap();
        let cwd = fixture.path();
        let config = cwd.join("extra=config.toml");
        std::fs::write(&config, "[net]\noffline = true\n").unwrap();

        assert_eq!(
            explicit_files(cwd, &["--config=extra=config.toml".to_owned()]),
            [config]
        );
    }

    #[test]
    fn discovery_includes_ancestor_and_cargo_home_candidates_only() {
        let files = files_with_home(
            Path::new("/workspace/app/nested"),
            &[],
            Some(Path::new("/cargo-home")),
        );
        for expected in [
            "/workspace/app/nested/.cargo/config.toml",
            "/workspace/app/nested/.cargo/config",
            "/workspace/app/.cargo/config.toml",
            "/workspace/app/.cargo/config",
            "/cargo-home/config.toml",
            "/cargo-home/config",
        ] {
            assert!(
                files.contains(&PathBuf::from(expected)),
                "missing {expected}"
            );
        }
        assert!(!files.contains(&PathBuf::from("/cargo-home/credentials.toml")));
        assert!(!files.contains(&PathBuf::from("/cargo-home/registry")));
        assert!(!files.contains(&PathBuf::from("/cargo-home/git")));
    }
}
