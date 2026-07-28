use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const GENERATED_DIRECTORY: &str = "ui-lang-generated";

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Compiles one manifest-relative Ice root into Cargo's `OUT_DIR`.
pub fn compile(path: impl AsRef<Path>) -> Result<(), Error> {
    compile_many([path])
}

/// Compiles manifest-relative Ice roots into Cargo's `OUT_DIR`.
pub fn compile_many<I, P>(paths: I) -> Result<(), Error>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let manifest = cargo_path("CARGO_MANIFEST_DIR")?;
    let out_dir = cargo_path("OUT_DIR")?;
    for path in paths {
        compile_one(&manifest, &out_dir, path.as_ref())?;
    }
    Ok(())
}

/// Compiles every app or daemon root below a manifest-relative directory.
pub fn compile_dir(path: impl AsRef<Path>) -> Result<(), Error> {
    let manifest = cargo_path("CARGO_MANIFEST_DIR")?;
    let out_dir = cargo_path("OUT_DIR")?;
    compile_dir_at(&manifest, &out_dir, path.as_ref())
}

/// Returns the generated Rust path used by both the build script and proc macro.
pub fn generated_path(out_dir: impl AsRef<Path>, relative: &str) -> Result<PathBuf, Error> {
    let relative = normalized_relative(Path::new(relative))?;
    let encoded = relative
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(out_dir
        .as_ref()
        .join(GENERATED_DIRECTORY)
        .join(format!("{encoded}.rs")))
}

fn cargo_path(name: &str) -> Result<PathBuf, Error> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| Error(format!("ui-lang-build: Cargo did not provide {name}")))
}

fn compile_dir_at(manifest: &Path, out_dir: &Path, relative: &Path) -> Result<(), Error> {
    let relative = normalized_relative(relative)?;
    let directory = manifest.join(Path::new(&relative));
    println!("cargo::rerun-if-changed={}", directory.display());
    let mut sources = Vec::new();
    collect_ice_sources(&directory, &mut sources)?;
    sources.sort();
    let mut roots = Vec::new();
    for source in &sources {
        println!("cargo::rerun-if-changed={}", source.display());
        let contents = fs::read_to_string(source).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot read {}: {error}",
                source.display()
            ))
        })?;
        if ui_lang_core::source_is_app(&contents) {
            let relative = source.strip_prefix(manifest).map_err(|_| {
                Error(format!(
                    "ui-lang-build: {} is outside {}",
                    source.display(),
                    manifest.display()
                ))
            })?;
            roots.push(relative.to_owned());
        }
    }
    if roots.is_empty() {
        return Err(Error(format!(
            "ui-lang-build: no app or daemon roots found below {}",
            directory.display()
        )));
    }
    let generated = roots
        .iter()
        .map(|root| compile_one(manifest, out_dir, root))
        .collect::<Result<HashSet<_>, _>>()?;
    prune_generated(out_dir, &relative, &generated)?;
    Ok(())
}

fn collect_ice_sources(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<(), Error> {
    let entries = fs::read_dir(directory).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot read directory {}: {error}",
            directory.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot read an entry in {}: {error}",
                directory.display()
            ))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot inspect {}: {error}",
                entry.path().display()
            ))
        })?;
        if file_type.is_dir() {
            collect_ice_sources(&entry.path(), sources)?;
        } else if file_type.is_file() && entry.path().extension().is_some_and(|ext| ext == "ice") {
            sources.push(entry.path());
        }
    }
    Ok(())
}

fn compile_one(manifest: &Path, out_dir: &Path, relative: &Path) -> Result<PathBuf, Error> {
    let relative = normalized_relative(relative)?;
    let source = manifest.join(Path::new(&relative));
    let compilation = ui_lang_core::compile_file(&source)
        .map_err(|error| Error(error.render(&source.display().to_string())))?;
    for dependency in &compilation.dependencies {
        println!("cargo::rerun-if-changed={}", dependency.display());
    }
    let destination = generated_path(out_dir, &relative)?;
    let directory = destination
        .parent()
        .expect("a generated path always has a parent");
    fs::create_dir_all(directory).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot create {}: {error}",
            directory.display()
        ))
    })?;
    if fs::read_to_string(&destination).ok().as_deref() != Some(&compilation.rust) {
        fs::write(&destination, compilation.rust).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot write {}: {error}",
                destination.display()
            ))
        })?;
    }
    Ok(destination)
}

fn prune_generated(
    out_dir: &Path,
    relative_directory: &str,
    expected: &HashSet<PathBuf>,
) -> Result<(), Error> {
    let directory = out_dir.join(GENERATED_DIRECTORY);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Error(format!(
                "ui-lang-build: cannot read {}: {error}",
                directory.display()
            )));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot read an entry in {}: {error}",
                directory.display()
            ))
        })?;
        let path = entry.path();
        if generated_relative(&path).is_some_and(|relative| {
            relative
                .strip_prefix(relative_directory)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) && !expected.contains(&path)
        {
            fs::remove_file(&path).map_err(|error| {
                Error(format!(
                    "ui-lang-build: cannot remove stale output {}: {error}",
                    path.display()
                ))
            })?;
        }
    }
    Ok(())
}

fn generated_relative(path: &Path) -> Option<String> {
    let encoded = path.file_name()?.to_str()?.strip_suffix(".rs")?;
    if encoded.len() % 2 != 0 {
        return None;
    }
    let bytes = encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

fn normalized_relative(path: &Path) -> Result<String, Error> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir if parts.last().is_some_and(|part| part != "..") => {
                parts.pop();
            }
            Component::ParentDir => parts.push("..".to_owned()),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir => {
                return Err(Error(format!(
                    "ui-lang-build: paths must be manifest-relative: {}",
                    path.display()
                )));
            }
        }
    }
    let relative = parts.join("/");
    validate_relative(&relative)?;
    Ok(relative)
}

fn validate_relative(relative: &str) -> Result<(), Error> {
    if relative.is_empty()
        || relative.contains('\\')
        || Path::new(relative).is_absolute()
        || relative.as_bytes().get(1) == Some(&b':') && relative.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err(Error(format!(
            "ui-lang-build: paths must be non-empty manifest-relative `/` paths: {relative}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compile_dir_at, generated_path};
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn generates_roots_below_cargo_out_dir() {
        let fixture = std::env::temp_dir().join(format!(
            "ui-lang-build-{}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let manifest = fixture.join("manifest");
        let out_dir = fixture.join("target/out");
        fs::create_dir_all(manifest.join("src/ui/fragments")).unwrap();
        fs::write(
            manifest.join("src/ui/app.ice"),
            concat!(
                "app Example\n",
                "theme contract AppTheme\n",
                "  bg\n",
                "  fg\n",
                "  primary\n",
                "  danger\n",
                "palette app for AppTheme\n",
                "  bg #000000\n",
                "  fg #ffffff\n",
                "  primary #333333\n",
                "  danger #ff0000\n",
                "view\n",
                "  text \"Hello\"\n",
            ),
        )
        .unwrap();
        fs::write(
            manifest.join("src/ui/fragments/text.ice"),
            "component Label(value:str)\n  text value\n",
        )
        .unwrap();
        let stale = generated_path(&out_dir, "src/ui/fragments/text.ice").unwrap();
        fs::create_dir_all(stale.parent().unwrap()).unwrap();
        fs::write(&stale, "stale").unwrap();
        let marker = stale.parent().unwrap().join("preserve.txt");
        fs::write(&marker, "not generated Rust").unwrap();
        let outside = generated_path(&out_dir, "other/app.ice").unwrap();
        fs::write(&outside, "another compile call owns this output").unwrap();

        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();

        let generated = generated_path(&out_dir, "src/ui/app.ice").unwrap();
        assert!(generated.starts_with(&out_dir));
        assert!(generated.is_file());
        assert_eq!(
            generated,
            generated_path(&out_dir, "src/ui/../ui/app.ice").unwrap()
        );
        assert!(generated_path(&out_dir, "/tmp/app.ice").is_err());
        assert!(!stale.exists());
        assert!(marker.exists());
        assert!(outside.exists());
        fs::remove_dir_all(fixture).unwrap();
    }
}
