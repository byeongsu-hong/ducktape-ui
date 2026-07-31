use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const GENERATED_DIRECTORY: &str = "ui-lang-generated";
const GENERATED_MANIFEST: &str = "manifest.json";
const GENERATED_MANIFEST_SCHEMA: u32 = 1;
const COMPILER_STACK_SIZE: usize = 8 * 1024 * 1024;
const DEV_BUILD_FINGERPRINT_ENV: &str = "ICE_DEV_BUILD_FINGERPRINT";

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedManifest {
    schema_version: u32,
    outputs: BTreeMap<String, String>,
}

impl Default for GeneratedManifest {
    fn default() -> Self {
        Self {
            schema_version: GENERATED_MANIFEST_SCHEMA,
            outputs: BTreeMap::new(),
        }
    }
}

impl GeneratedManifest {
    fn record(&mut self, relative: &str) -> Result<String, Error> {
        let output = generated_file_name(relative);
        self.insert(output.clone(), relative.to_owned())?;
        Ok(output)
    }

    fn insert(&mut self, output: String, relative: String) -> Result<(), Error> {
        if let Some(existing) = self.outputs.get(&output)
            && existing != &relative
        {
            return Err(Error(format!(
                "ui-lang-build: generated output collision: {output} maps to both {existing} and {relative}"
            )));
        }
        self.outputs.insert(output, relative);
        Ok(())
    }

    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != GENERATED_MANIFEST_SCHEMA {
            return Err(Error(format!(
                "ui-lang-build: unsupported generated manifest schema {}; expected {GENERATED_MANIFEST_SCHEMA}",
                self.schema_version
            )));
        }
        for (output, relative) in &self.outputs {
            let normalized = normalized_relative(Path::new(relative))?;
            let expected = generated_file_name(&normalized);
            if relative != &normalized || output != &expected {
                return Err(Error(format!(
                    "ui-lang-build: invalid generated manifest mapping {output} -> {relative}; expected {expected} -> {normalized}"
                )));
            }
        }
        Ok(())
    }
}

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
    let paths = paths
        .into_iter()
        .map(|path| path.as_ref().to_owned())
        .collect::<Vec<_>>();
    compiler_thread(move || {
        let manifest = cargo_path("CARGO_MANIFEST_DIR")?;
        let out_dir = cargo_path("OUT_DIR")?;
        let mut generated_manifest = load_generated_manifest(&out_dir)?;
        for path in paths {
            compile_one(&manifest, &out_dir, &path, &mut generated_manifest)?;
        }
        write_generated_manifest(&out_dir, &generated_manifest)?;
        Ok(())
    })
}

/// Compiles every app or daemon root below a manifest-relative directory.
pub fn compile_dir(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref().to_owned();
    compiler_thread(move || {
        let manifest = cargo_path("CARGO_MANIFEST_DIR")?;
        let out_dir = cargo_path("OUT_DIR")?;
        compile_dir_at(&manifest, &out_dir, &path)
    })
}

/// Returns the generated Rust path used by both the build script and proc macro.
pub fn generated_path(out_dir: impl AsRef<Path>, relative: &str) -> Result<PathBuf, Error> {
    let relative = normalized_relative(Path::new(relative))?;
    Ok(out_dir
        .as_ref()
        .join(GENERATED_DIRECTORY)
        .join(generated_file_name(&relative)))
}

fn generated_file_name(relative: &str) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(relative.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2 + 3);
    for byte in digest {
        write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded.push_str(".rs");
    encoded
}

fn cargo_path(name: &str) -> Result<PathBuf, Error> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| Error(format!("ui-lang-build: Cargo did not provide {name}")))
}

fn compiler_thread<T>(
    operation: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> Result<T, Error>
where
    T: Send + 'static,
{
    std::thread::Builder::new()
        .name("ui-lang-build".to_owned())
        .stack_size(COMPILER_STACK_SIZE)
        .spawn(operation)
        .map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot start compiler thread: {error}"
            ))
        })?
        .join()
        .map_err(|_| Error("ui-lang-build: compiler thread panicked".to_owned()))?
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
    let mut generated_manifest = load_generated_manifest(out_dir)?;
    let generated = roots
        .iter()
        .map(|root| compile_one(manifest, out_dir, root, &mut generated_manifest))
        .collect::<Result<HashSet<_>, _>>()?;
    prune_generated(out_dir, &relative, &generated, &mut generated_manifest)?;
    write_generated_manifest(out_dir, &generated_manifest)?;
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

fn compile_one(
    manifest: &Path,
    out_dir: &Path,
    relative: &Path,
    generated_manifest: &mut GeneratedManifest,
) -> Result<PathBuf, Error> {
    let relative = normalized_relative(relative)?;
    let source = manifest.join(Path::new(&relative));
    let compilation = ui_lang_core::compile_file(&source)
        .map_err(|error| Error(error.render(&source.display().to_string())))?;
    for directive in rerun_directives(&compilation.dependencies, &compilation.asset_dependencies) {
        println!("{directive}");
    }
    let output = generated_manifest.record(&relative)?;
    let directory = out_dir.join(GENERATED_DIRECTORY);
    let destination = directory.join(output);
    fs::create_dir_all(&directory).map_err(|error| {
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

fn rerun_directives(sources: &[PathBuf], assets: &[PathBuf]) -> Vec<String> {
    std::iter::once(format!(
        "cargo::rerun-if-env-changed={DEV_BUILD_FINGERPRINT_ENV}"
    ))
    .chain(
        sources
            .iter()
            .chain(assets)
            .map(|path| format!("cargo::rerun-if-changed={}", path.display())),
    )
    .collect()
}

fn load_generated_manifest(out_dir: &Path) -> Result<GeneratedManifest, Error> {
    let path = out_dir.join(GENERATED_DIRECTORY).join(GENERATED_MANIFEST);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GeneratedManifest::default());
        }
        Err(error) => {
            return Err(Error(format!(
                "ui-lang-build: cannot read generated manifest {}: {error}",
                path.display()
            )));
        }
    };
    let manifest = serde_json::from_str::<GeneratedManifest>(&contents).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot parse generated manifest {}: {error}",
            path.display()
        ))
    })?;
    manifest.validate()?;
    Ok(manifest)
}

fn write_generated_manifest(
    out_dir: &Path,
    generated_manifest: &GeneratedManifest,
) -> Result<(), Error> {
    generated_manifest.validate()?;
    let directory = out_dir.join(GENERATED_DIRECTORY);
    fs::create_dir_all(&directory).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot create {}: {error}",
            directory.display()
        ))
    })?;
    prune_untracked_generated(&directory, generated_manifest)?;
    let path = directory.join(GENERATED_MANIFEST);
    let mut contents = serde_json::to_string_pretty(generated_manifest).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot serialize generated manifest {}: {error}",
            path.display()
        ))
    })?;
    contents.push('\n');
    if fs::read_to_string(&path).ok().as_deref() != Some(&contents) {
        fs::write(&path, contents).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot write generated manifest {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn prune_untracked_generated(
    directory: &Path,
    generated_manifest: &GeneratedManifest,
) -> Result<(), Error> {
    let entries = fs::read_dir(directory).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot read {}: {error}",
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
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|error| {
                Error(format!(
                    "ui-lang-build: cannot inspect {}: {error}",
                    path.display()
                ))
            })?
            .is_file()
            && path.extension().is_some_and(|extension| extension == "rs")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| !generated_manifest.outputs.contains_key(name))
        {
            fs::remove_file(&path).map_err(|error| {
                Error(format!(
                    "ui-lang-build: cannot remove untracked output {}: {error}",
                    path.display()
                ))
            })?;
        }
    }
    Ok(())
}

fn prune_generated(
    out_dir: &Path,
    relative_directory: &str,
    expected: &HashSet<PathBuf>,
    generated_manifest: &mut GeneratedManifest,
) -> Result<(), Error> {
    let directory = out_dir.join(GENERATED_DIRECTORY);
    let stale = generated_manifest
        .outputs
        .iter()
        .filter(|(output, relative)| {
            relative
                .strip_prefix(relative_directory)
                .is_some_and(|suffix| suffix.starts_with('/'))
                && !expected.contains(&directory.join(output))
        })
        .map(|(output, _)| output.clone())
        .collect::<Vec<_>>();
    for output in stale {
        let path = directory.join(&output);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error(format!(
                    "ui-lang-build: cannot remove stale output {}: {error}",
                    path.display()
                )));
            }
        }
        generated_manifest.outputs.remove(&output);
    }
    Ok(())
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
    use super::{
        GeneratedManifest, compile_dir_at, generated_file_name, generated_path,
        load_generated_manifest, rerun_directives, write_generated_manifest,
    };
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn cargo_tracks_dev_fingerprint_sources_and_assets() {
        assert_eq!(
            rerun_directives(
                &["src/ui/app.ice".into(), "src/ui/content.ice".into()],
                &["assets/font.ttf".into(), "assets/icon.rgba".into()],
            ),
            [
                "cargo::rerun-if-env-changed=ICE_DEV_BUILD_FINGERPRINT",
                "cargo::rerun-if-changed=src/ui/app.ice",
                "cargo::rerun-if-changed=src/ui/content.ice",
                "cargo::rerun-if-changed=assets/font.ttf",
                "cargo::rerun-if-changed=assets/icon.rgba",
            ]
        );
    }

    #[test]
    fn generated_paths_use_fixed_length_stable_hashes() {
        let out_dir = Path::new("/target/out");
        let generated = generated_path(out_dir, "src/ui/app.ice").unwrap();
        assert_eq!(
            generated.file_name().unwrap(),
            "b9aad780c706fd9ac71260fc38ade7ed3328145af1306eb3683d056e3d3b7c92.rs"
        );
        assert_eq!(generated.file_name().unwrap().len(), 67);
        assert_eq!(
            generated,
            generated_path(out_dir, "src/ui/../ui/app.ice").unwrap()
        );

        let deep = format!("src/{}/화면.ice", vec!["nested-component"; 64].join("/"));
        let deep_generated = generated_path(out_dir, &deep).unwrap();
        assert_eq!(deep_generated.file_name().unwrap().len(), 67);
        assert_ne!(generated, deep_generated);
        assert!(generated_path(out_dir, "/tmp/app.ice").is_err());
    }

    #[test]
    fn manifest_rejects_output_collisions() {
        let mut manifest = GeneratedManifest::default();
        let output = manifest.record("src/ui/app.ice").unwrap();
        let error = manifest
            .insert(output, "src/ui/other.ice".to_owned())
            .unwrap_err();
        assert!(error.to_string().contains("generated output collision"));
    }

    #[test]
    fn manifest_rejects_unknown_schema_and_invalid_mappings() {
        let mut manifest = GeneratedManifest::default();
        manifest.schema_version += 1;
        assert!(
            manifest
                .validate()
                .unwrap_err()
                .to_string()
                .contains("unsupported generated manifest schema")
        );

        let mut manifest = GeneratedManifest::default();
        manifest.outputs.insert(
            "not-the-source-hash.rs".to_owned(),
            "src/ui/app.ice".to_owned(),
        );
        assert!(
            manifest
                .validate()
                .unwrap_err()
                .to_string()
                .contains("invalid generated manifest mapping")
        );
    }

    #[test]
    fn generates_and_prunes_roots_below_cargo_out_dir() {
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
        let mut previous_manifest = GeneratedManifest::default();
        previous_manifest
            .record("src/ui/fragments/text.ice")
            .unwrap();
        previous_manifest.record("other/app.ice").unwrap();
        write_generated_manifest(&out_dir, &previous_manifest).unwrap();
        let untracked = stale.parent().unwrap().join("untracked.rs");
        fs::write(&untracked, "not in the canonical manifest").unwrap();

        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();

        let generated = generated_path(&out_dir, "src/ui/app.ice").unwrap();
        assert!(generated.starts_with(&out_dir));
        assert!(generated.is_file());
        assert!(!stale.exists());
        assert!(!untracked.exists());
        assert!(marker.exists());
        assert!(outside.exists());
        let generated_manifest = load_generated_manifest(&out_dir).unwrap();
        assert_eq!(
            generated_manifest
                .outputs
                .get(&generated_file_name("src/ui/app.ice"))
                .map(String::as_str),
            Some("src/ui/app.ice")
        );
        assert_eq!(
            generated_manifest
                .outputs
                .get(&generated_file_name("other/app.ice"))
                .map(String::as_str),
            Some("other/app.ice")
        );
        assert!(
            !generated_manifest
                .outputs
                .contains_key(&generated_file_name("src/ui/fragments/text.ice"))
        );
        fs::remove_dir_all(fixture).unwrap();
    }
}
