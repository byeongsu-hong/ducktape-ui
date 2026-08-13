use atomicwrites::{AllowOverwrite, AtomicFile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const GENERATED_DIRECTORY: &str = "ui-lang-generated";
const GENERATED_MANIFEST: &str = "manifest.json";
const GENERATED_MANIFEST_SCHEMA: u32 = 3;
const GENERATION_LOCK: &str = ".generation.lock";
const TRANSACTION_DIRECTORY_PREFIX: &str = ".ui-lang-transaction-";
const ATOMIC_WRITE_DIRECTORY_PREFIX: &str = ".atomicwrite";
const COMPILER_STACK_SIZE: usize = 8 * 1024 * 1024;
const DEV_BUILD_FINGERPRINT_ENV: &str = "ICE_DEV_BUILD_FINGERPRINT";

static TRANSACTION_ID: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
std::thread_local! {
    static GENERATED_WRITES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CONTENT_COMPARISON_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

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
    outputs: BTreeMap<String, GeneratedManifestEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedManifestEntry {
    source: String,
    content_sha256: String,
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
    fn record(&mut self, relative: &str, contents: &str) -> Result<(String, bool), Error> {
        let output = generated_file_name(relative);
        let unchanged = self.insert(
            output.clone(),
            GeneratedManifestEntry {
                source: relative.to_owned(),
                content_sha256: content_digest(contents.as_bytes()),
            },
        )?;
        Ok((output, unchanged))
    }

    fn record_group(
        &mut self,
        relative: &str,
        slug: &str,
        contents: &str,
    ) -> Result<(String, bool), Error> {
        let output = generated_group_file_name(relative, slug);
        let unchanged = self.insert(
            output.clone(),
            GeneratedManifestEntry {
                source: relative.to_owned(),
                content_sha256: content_digest(contents.as_bytes()),
            },
        )?;
        Ok((output, unchanged))
    }

    /// Drops this source's entries that the current compilation no longer
    /// produced — a renamed or deleted fragment must take its group file
    /// with it (commit removes files absent from the manifest).
    fn retain_source_outputs(&mut self, source: &str, produced: &BTreeSet<String>) {
        self.outputs
            .retain(|output, entry| entry.source != source || produced.contains(output));
    }

    fn insert(&mut self, output: String, entry: GeneratedManifestEntry) -> Result<bool, Error> {
        if let Some(existing) = self.outputs.get(&output)
            && existing.source != entry.source
        {
            return Err(Error(format!(
                "ui-lang-build: generated output collision: {output} maps to both {} and {}",
                existing.source, entry.source
            )));
        }
        let unchanged = self.outputs.get(&output) == Some(&entry);
        self.outputs.insert(output, entry);
        Ok(unchanged)
    }

    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != GENERATED_MANIFEST_SCHEMA {
            return Err(Error(format!(
                "ui-lang-build: unsupported generated manifest schema {}; expected {GENERATED_MANIFEST_SCHEMA}",
                self.schema_version
            )));
        }
        for (output, entry) in &self.outputs {
            let normalized = normalized_relative(Path::new(&entry.source))?;
            let expected = generated_file_name(&normalized);
            let is_root = output == &expected;
            let is_group = is_group_output_of(output, &normalized);
            if entry.source != normalized || (!is_root && !is_group) {
                return Err(Error(format!(
                    "ui-lang-build: invalid generated manifest mapping {output} -> {}; expected {expected} (or a {expected}-prefixed group) -> {normalized}",
                    entry.source
                )));
            }
            if entry.content_sha256.len() != 64
                || !entry
                    .content_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(Error(format!(
                    "ui-lang-build: invalid generated content digest for {output}: {}",
                    entry.content_sha256
                )));
            }
        }
        Ok(())
    }
}

struct GenerationLock {
    _file: File,
}

struct GenerationTransaction {
    directory: PathBuf,
    staging_directory: PathBuf,
    manifest: GeneratedManifest,
    staged_outputs: BTreeSet<String>,
    committed: bool,
    _lock: GenerationLock,
}

/// Compiles one manifest-relative Ice root into Cargo's `OUT_DIR`.
pub fn compile(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref().to_owned();
    compiler_thread(move || {
        let manifest = cargo_path("CARGO_MANIFEST_DIR")?;
        let out_dir = cargo_path("OUT_DIR")?;
        compile_many_at(&manifest, &out_dir, std::slice::from_ref(&path))
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
    let mut encoded = content_digest(relative.as_bytes());
    encoded.push_str(".rs");
    encoded
}

/// File name of one fenced fragment group split out of a root's generated
/// Rust: the root's digest plus the fragment slug. Same directory as the
/// root, which `include!`s it by this bare name (include! resolves relative
/// to the including file).
fn generated_group_file_name(relative: &str, slug: &str) -> String {
    format!("{}__{slug}.rs", content_digest(relative.as_bytes()))
}

/// Recognizes `<digest of source>__<slug>.rs` for manifest validation.
fn is_group_output_of(output: &str, source: &str) -> bool {
    let digest = content_digest(source.as_bytes());
    output
        .strip_prefix(digest.as_str())
        .and_then(|rest| rest.strip_prefix("__"))
        .and_then(|rest| rest.strip_suffix(".rs"))
        .is_some_and(|slug| {
            !slug.is_empty()
                && slug
                    .bytes()
                    .all(|byte| byte == b'_' || byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn content_digest(contents: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(contents);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
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

fn compile_many_at(manifest: &Path, out_dir: &Path, paths: &[PathBuf]) -> Result<(), Error> {
    let mut analysis_db = ui_lang_core::AnalysisDb::default();
    let mut transaction = GenerationTransaction::begin(out_dir)?;
    for path in paths {
        compile_one(&mut analysis_db, manifest, path, &mut transaction)?;
    }
    transaction.commit()
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
    let mut analysis_db = ui_lang_core::AnalysisDb::default();
    let mut transaction = GenerationTransaction::begin(out_dir)?;
    let mut generated = HashSet::new();
    for root in &roots {
        generated.extend(compile_one(
            &mut analysis_db,
            manifest,
            root,
            &mut transaction,
        )?);
    }
    prune_generated(&relative, &generated, &mut transaction.manifest);
    transaction.commit()
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
    analysis_db: &mut ui_lang_core::AnalysisDb,
    manifest: &Path,
    relative: &Path,
    transaction: &mut GenerationTransaction,
) -> Result<BTreeSet<String>, Error> {
    let relative = normalized_relative(relative)?;
    let source = manifest.join(Path::new(&relative));
    let compilation = analysis_db
        .compile_root(&source)
        .map_err(|error| Error(error.render(&source.display().to_string())))?;
    for directive in rerun_directives(&compilation.dependencies, &compilation.asset_dependencies) {
        println!("{directive}");
    }
    // Split each fenced fragment group into its own file: spans are
    // per-file, so an edit to one fragment leaves every other group's
    // rustc incremental fingerprints byte-identical. The root keeps an
    // `include!` where the region stood.
    let (root, groups) = split_generated_groups(&relative, &compilation.rust)?;
    let mut produced = BTreeSet::new();
    for (slug, contents) in &groups {
        produced.insert(transaction.stage_group_output(&relative, slug, contents)?);
    }
    produced.insert(transaction.stage_output(&relative, &root)?);
    transaction
        .manifest
        .retain_source_outputs(&relative, &produced);
    Ok(produced)
}

/// Cuts the `GROUP_MARKER` fenced regions out of generated Rust. Returns the
/// root text and the `(slug, contents)` regions in order of appearance. The
/// `include!` lines for the regions go at the END of the root — after the
/// lint-boundary macro invocation the regions were fenced inside, whose
/// per-item `#[allow]` would be an unused attribute on an `include!` item
/// (attributes do not reach included items; each group mod carries its own).
fn split_generated_groups(
    relative: &str,
    rust: &str,
) -> Result<(String, Vec<(String, String)>), Error> {
    let mut root = String::with_capacity(rust.len());
    let mut groups: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in rust.lines() {
        if let Some(slug) = line.strip_prefix(ui_lang_core::GROUP_MARKER_BEGIN) {
            if current.is_some() {
                return Err(Error(format!(
                    "ui-lang-build: nested generated group fence for {relative}"
                )));
            }
            current = Some((slug.trim().to_owned(), String::new()));
            continue;
        }
        if line.trim_end() == ui_lang_core::GROUP_MARKER_END {
            let (slug, contents) = current.take().ok_or_else(|| {
                Error(format!(
                    "ui-lang-build: unopened generated group fence for {relative}"
                ))
            })?;
            groups.push((slug, contents));
            continue;
        }
        match &mut current {
            Some((_, contents)) => {
                contents.push_str(line);
                contents.push('\n');
            }
            None => {
                root.push_str(line);
                root.push('\n');
            }
        }
    }
    if current.is_some() {
        return Err(Error(format!(
            "ui-lang-build: unclosed generated group fence for {relative}"
        )));
    }
    for (slug, _) in &groups {
        root.push_str(&format!(
            "include!({:?});\n",
            generated_group_file_name(relative, slug)
        ));
    }
    Ok((root, groups))
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

impl GenerationLock {
    fn acquire(directory: &Path) -> Result<Self, Error> {
        fs::create_dir_all(directory).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot create {}: {error}",
                directory.display()
            ))
        })?;
        let path = directory.join(GENERATION_LOCK);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|error| {
                Error(format!(
                    "ui-lang-build: cannot open generation lock {}: {error}",
                    path.display()
                ))
            })?;
        fs2::FileExt::lock_exclusive(&file).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot lock generation directory {}: {error}",
                directory.display()
            ))
        })?;
        Ok(Self { _file: file })
    }
}

impl GenerationTransaction {
    fn begin(out_dir: &Path) -> Result<Self, Error> {
        let directory = out_dir.join(GENERATED_DIRECTORY);
        let generation_lock = GenerationLock::acquire(&directory)?;
        cleanup_stale_temporary_entries(&directory)?;
        let manifest = load_or_recover_generated_manifest(&directory)?;
        let staging_directory = create_staging_directory(&directory)?;
        Ok(Self {
            directory,
            staging_directory,
            manifest,
            staged_outputs: BTreeSet::new(),
            committed: false,
            _lock: generation_lock,
        })
    }

    fn stage_output(&mut self, relative: &str, contents: &str) -> Result<String, Error> {
        let (output, unchanged) = self.manifest.record(relative, contents)?;
        self.stage_named(output, contents, unchanged)
    }

    fn stage_group_output(
        &mut self,
        relative: &str,
        slug: &str,
        contents: &str,
    ) -> Result<String, Error> {
        let (output, unchanged) = self.manifest.record_group(relative, slug, contents)?;
        self.stage_named(output, contents, unchanged)
    }

    fn stage_named(
        &mut self,
        output: String,
        contents: &str,
        unchanged: bool,
    ) -> Result<String, Error> {
        // Existing manifest entries were content-validated when this
        // transaction began; entries added here already have staged output.
        if unchanged {
            return Ok(output);
        }
        let staged = self.staging_directory.join(&output);
        write_synced(&staged, contents.as_bytes(), "staged generated output")?;
        self.staged_outputs.insert(output.clone());
        Ok(output)
    }

    fn commit(mut self) -> Result<(), Error> {
        self.manifest.validate()?;
        let obsolete = obsolete_generated_outputs(&self.directory, &self.manifest)?;
        let manifest_contents = serialize_generated_manifest(&self.directory, &self.manifest)?;
        let manifest_destination = self.directory.join(GENERATED_MANIFEST);
        let manifest_changed = !file_contents_equal(&manifest_destination, &manifest_contents);

        if self.staged_outputs.is_empty() && obsolete.is_empty() && !manifest_changed {
            self.cleanup_staging()?;
            return Ok(());
        }

        let staged_manifest = self.staging_directory.join(GENERATED_MANIFEST);
        write_synced(
            &staged_manifest,
            &manifest_contents,
            "staged generated manifest",
        )?;

        for output in &self.staged_outputs {
            replace_atomically(
                &self.staging_directory.join(output),
                &self.directory.join(output),
                "generated output",
            )?;
            record_generated_write();
        }
        for output in obsolete {
            let path = self.directory.join(output);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(Error(format!(
                        "ui-lang-build: cannot remove stale output {}: {error}",
                        path.display()
                    )));
                }
            }
        }

        replace_atomically(
            &staged_manifest,
            &manifest_destination,
            "generated manifest",
        )?;
        record_generated_write();
        self.cleanup_staging()
    }

    fn cleanup_staging(&mut self) -> Result<(), Error> {
        fs::remove_dir_all(&self.staging_directory).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot remove transaction directory {}: {error}",
                self.staging_directory.display()
            ))
        })?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for GenerationTransaction {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.staging_directory);
        }
    }
}

fn create_staging_directory(directory: &Path) -> Result<PathBuf, Error> {
    loop {
        let id = TRANSACTION_ID.fetch_add(1, Ordering::Relaxed);
        let staging = directory.join(format!(
            "{TRANSACTION_DIRECTORY_PREFIX}{}-{id}",
            std::process::id()
        ));
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(Error(format!(
                    "ui-lang-build: cannot create transaction directory {}: {error}",
                    staging.display()
                )));
            }
        }
    }
}

fn cleanup_stale_temporary_entries(directory: &Path) -> Result<(), Error> {
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
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(TRANSACTION_DIRECTORY_PREFIX)
            && !name.starts_with(ATOMIC_WRITE_DIRECTORY_PREFIX)
        {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot inspect stale temporary entry {}: {error}",
                path.display()
            ))
        })?;
        let result = if file_type.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        result.map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot remove stale temporary entry {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn load_or_recover_generated_manifest(directory: &Path) -> Result<GeneratedManifest, Error> {
    match read_generated_manifest(directory) {
        Ok(Some(manifest)) => {
            remove_untracked_generated(directory, &manifest)?;
            Ok(manifest)
        }
        Ok(None) | Err(_) => {
            reset_generated_cache(directory)?;
            Ok(GeneratedManifest::default())
        }
    }
}

fn read_generated_manifest(directory: &Path) -> Result<Option<GeneratedManifest>, Error> {
    let path = directory.join(GENERATED_MANIFEST);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
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
    for (output, entry) in &manifest.outputs {
        let output_path = directory.join(output);
        let contents = fs::read(&output_path).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot validate generated output {}: {error}",
                output_path.display()
            ))
        })?;
        let actual = content_digest(&contents);
        if actual != entry.content_sha256 {
            return Err(Error(format!(
                "ui-lang-build: generated output digest mismatch for {}: expected {}, found {actual}",
                output_path.display(),
                entry.content_sha256
            )));
        }
    }
    Ok(Some(manifest))
}

fn serialize_generated_manifest(
    directory: &Path,
    manifest: &GeneratedManifest,
) -> Result<Vec<u8>, Error> {
    let path = directory.join(GENERATED_MANIFEST);
    let mut contents = serde_json::to_string_pretty(manifest).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot serialize generated manifest {}: {error}",
            path.display()
        ))
    })?;
    contents.push('\n');
    Ok(contents.into_bytes())
}

fn write_synced(path: &Path, contents: &[u8], kind: &str) -> Result<(), Error> {
    let mut file = File::create(path).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot create {kind} {}: {error}",
            path.display()
        ))
    })?;
    file.write_all(contents).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot write {kind} {}: {error}",
            path.display()
        ))
    })?;
    file.flush().map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot flush {kind} {}: {error}",
            path.display()
        ))
    })?;
    file.sync_all().map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot sync {kind} {}: {error}",
            path.display()
        ))
    })
}

fn replace_atomically(staged: &Path, destination: &Path, kind: &str) -> Result<(), Error> {
    AtomicFile::new(destination, AllowOverwrite)
        .write(|file| -> io::Result<()> {
            let mut source = File::open(staged)?;
            io::copy(&mut source, file)?;
            file.flush()
        })
        .map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot atomically replace {kind} {}: {error}",
                destination.display()
            ))
        })
}

fn reset_generated_cache(directory: &Path) -> Result<(), Error> {
    let entries = fs::read_dir(directory).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot read {} while resetting generated cache: {error}",
            directory.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot read an entry in {} while resetting generated cache: {error}",
                directory.display()
            ))
        })?;
        let path = entry.path();
        let is_manifest = entry.file_name() == GENERATED_MANIFEST;
        let is_generated_rust = path.extension().is_some_and(|extension| extension == "rs");
        if is_manifest || is_generated_rust {
            fs::remove_file(&path).map_err(|error| {
                Error(format!(
                    "ui-lang-build: cannot remove invalid cache entry {}: {error}",
                    path.display()
                ))
            })?;
        }
    }
    Ok(())
}

fn remove_untracked_generated(directory: &Path, manifest: &GeneratedManifest) -> Result<(), Error> {
    for output in obsolete_generated_outputs(directory, manifest)? {
        let path = directory.join(output);
        fs::remove_file(&path).map_err(|error| {
            Error(format!(
                "ui-lang-build: cannot remove untracked output {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn obsolete_generated_outputs(
    directory: &Path,
    manifest: &GeneratedManifest,
) -> Result<Vec<String>, Error> {
    let entries = fs::read_dir(directory).map_err(|error| {
        Error(format!(
            "ui-lang-build: cannot read {}: {error}",
            directory.display()
        ))
    })?;
    let mut obsolete = Vec::new();
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
            && let Some(name) = path.file_name().and_then(|name| name.to_str())
            && !manifest.outputs.contains_key(name)
        {
            obsolete.push(name.to_owned());
        }
    }
    obsolete.sort();
    Ok(obsolete)
}

fn prune_generated(
    relative_directory: &str,
    expected: &HashSet<String>,
    manifest: &mut GeneratedManifest,
) {
    manifest.outputs.retain(|output, entry| {
        !entry
            .source
            .strip_prefix(relative_directory)
            .is_some_and(|suffix| suffix.starts_with('/'))
            || expected.contains(output)
    });
}

fn file_contents_equal(path: &Path, contents: &[u8]) -> bool {
    let Ok(existing) = fs::read(path) else {
        return false;
    };
    #[cfg(test)]
    CONTENT_COMPARISON_READS.with(|reads| reads.set(reads.get() + 1));
    existing == contents
}

fn record_generated_write() {
    #[cfg(test)]
    GENERATED_WRITES.with(|writes| writes.set(writes.get() + 1));
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
        GENERATED_DIRECTORY, GENERATED_MANIFEST, GeneratedManifest, GeneratedManifestEntry,
        GenerationLock, compile_dir_at, compile_many_at, content_digest, generated_file_name,
        generated_path, read_generated_manifest, rerun_directives, serialize_generated_manifest,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Barrier, mpsc};
    use std::thread;
    use std::time::{Duration, Instant};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    fn reset_generated_writes() {
        super::GENERATED_WRITES.with(|writes| writes.set(0));
    }

    fn generated_writes() -> usize {
        super::GENERATED_WRITES.with(std::cell::Cell::get)
    }

    fn reset_content_comparison_reads() {
        super::CONTENT_COMPARISON_READS.with(|reads| reads.set(0));
    }

    fn content_comparison_reads() -> usize {
        super::CONTENT_COMPARISON_READS.with(std::cell::Cell::get)
    }

    fn fixture(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ui-lang-build-{name}-{}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// Files one root produces: the root itself plus one per fenced group.
    /// The fixture apps have no fragments, so the groups are exactly the two
    /// the generator always fences out — `__update` and `__view`.
    const FILES_PER_ROOT: usize = 3;

    fn app_source(name: &str, text: &str) -> String {
        format!(
            concat!(
                "app {}\n",
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
                "  text \"{}\"\n",
            ),
            name, text
        )
    }

    fn write_manifest_fixture(out_dir: &Path, manifest: &GeneratedManifest) {
        let directory = out_dir.join(GENERATED_DIRECTORY);
        fs::create_dir_all(&directory).unwrap();
        let contents = serialize_generated_manifest(&directory, manifest).unwrap();
        fs::write(directory.join(GENERATED_MANIFEST), contents).unwrap();
    }

    fn read_manifest(out_dir: &Path) -> GeneratedManifest {
        read_generated_manifest(&out_dir.join(GENERATED_DIRECTORY))
            .unwrap()
            .unwrap()
    }

    fn assert_no_temporary_entries(out_dir: &Path) {
        let directory = out_dir.join(GENERATED_DIRECTORY);
        if !directory.exists() {
            return;
        }
        for entry in fs::read_dir(directory).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            assert!(
                !name.starts_with(super::TRANSACTION_DIRECTORY_PREFIX)
                    && !name.starts_with(super::ATOMIC_WRITE_DIRECTORY_PREFIX),
                "stale transaction entry remained: {name}"
            );
        }
    }

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
        let (output, unchanged) = manifest.record("src/ui/app.ice", "first").unwrap();
        assert!(!unchanged);
        assert!(manifest.record("src/ui/app.ice", "first").unwrap().1);
        assert!(!manifest.record("src/ui/app.ice", "changed").unwrap().1);
        let error = manifest
            .insert(
                output,
                GeneratedManifestEntry {
                    source: "src/ui/other.ice".to_owned(),
                    content_sha256: content_digest(b"second"),
                },
            )
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
            GeneratedManifestEntry {
                source: "src/ui/app.ice".to_owned(),
                content_sha256: content_digest(b"generated"),
            },
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
        let fixture = fixture("prune");
        let manifest = fixture.join("manifest");
        let out_dir = fixture.join("target/out");
        fs::create_dir_all(manifest.join("src/ui/fragments")).unwrap();
        fs::write(
            manifest.join("src/ui/app.ice"),
            app_source("Example", "Hello"),
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
            .record("src/ui/fragments/text.ice", "stale")
            .unwrap();
        previous_manifest
            .record("other/app.ice", "another compile call owns this output")
            .unwrap();
        write_manifest_fixture(&out_dir, &previous_manifest);
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
        let generated_manifest = read_manifest(&out_dir);
        assert_eq!(
            generated_manifest
                .outputs
                .get(&generated_file_name("src/ui/app.ice"))
                .map(|entry| entry.source.as_str()),
            Some("src/ui/app.ice")
        );
        assert_eq!(
            generated_manifest
                .outputs
                .get(&generated_file_name("other/app.ice"))
                .map(|entry| entry.source.as_str()),
            Some("other/app.ice")
        );
        assert!(
            !generated_manifest
                .outputs
                .contains_key(&generated_file_name("src/ui/fragments/text.ice"))
        );
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn failed_batch_publishes_nothing() {
        let fixture = fixture("failed-batch");
        let manifest = fixture.join("manifest");
        let out_dir = fixture.join("target/out");
        fs::create_dir_all(manifest.join("src/ui")).unwrap();
        fs::write(
            manifest.join("src/ui/valid.ice"),
            app_source("Valid", "ready"),
        )
        .unwrap();
        fs::write(
            manifest.join("src/ui/invalid.ice"),
            "app Invalid\nview\n  wat\n",
        )
        .unwrap();

        let error = compile_many_at(
            &manifest,
            &out_dir,
            &[
                PathBuf::from("src/ui/valid.ice"),
                PathBuf::from("src/ui/invalid.ice"),
            ],
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown view node `wat`"));
        assert!(
            !generated_path(&out_dir, "src/ui/valid.ice")
                .unwrap()
                .exists()
        );
        assert!(
            !out_dir
                .join(GENERATED_DIRECTORY)
                .join(GENERATED_MANIFEST)
                .exists()
        );
        assert_no_temporary_entries(&out_dir);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn failed_batch_preserves_previous_generation() {
        let fixture = fixture("failed-update");
        let manifest = fixture.join("manifest");
        let source_dir = manifest.join("src/ui");
        let out_dir = fixture.join("target/out");
        fs::create_dir_all(&source_dir).unwrap();
        let valid = source_dir.join("valid.ice");
        fs::write(&valid, app_source("Valid", "before")).unwrap();
        compile_many_at(&manifest, &out_dir, &[PathBuf::from("src/ui/valid.ice")]).unwrap();
        let output = generated_path(&out_dir, "src/ui/valid.ice").unwrap();
        let manifest_path = out_dir.join(GENERATED_DIRECTORY).join(GENERATED_MANIFEST);
        let previous_output = fs::read(&output).unwrap();
        let previous_manifest = fs::read(&manifest_path).unwrap();

        fs::write(&valid, app_source("Valid", "after")).unwrap();
        fs::write(source_dir.join("invalid.ice"), "app Invalid\nview\n  wat\n").unwrap();
        compile_many_at(
            &manifest,
            &out_dir,
            &[
                PathBuf::from("src/ui/valid.ice"),
                PathBuf::from("src/ui/invalid.ice"),
            ],
        )
        .unwrap_err();

        assert_eq!(fs::read(output).unwrap(), previous_output);
        assert_eq!(fs::read(manifest_path).unwrap(), previous_manifest);
        assert_no_temporary_entries(&out_dir);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn corrupt_cache_is_discarded_and_fully_regenerated() {
        let fixture = fixture("corrupt-cache");
        let manifest = fixture.join("manifest");
        let source_dir = manifest.join("src/ui");
        let out_dir = fixture.join("target/out");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("first.ice"), app_source("First", "one")).unwrap();
        fs::write(source_dir.join("second.ice"), app_source("Second", "two")).unwrap();

        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();
        let directory = out_dir.join(GENERATED_DIRECTORY);
        let manifest_path = directory.join(GENERATED_MANIFEST);
        fs::write(&manifest_path, "{not-json").unwrap();

        reset_generated_writes();
        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();
        assert_eq!(
            generated_writes(),
            2 * FILES_PER_ROOT + 1,
            "a malformed manifest must regenerate every root and the manifest"
        );
        assert_eq!(read_manifest(&out_dir).outputs.len(), 2 * FILES_PER_ROOT);

        let interrupted = generated_path(&out_dir, "src/ui/first.ice").unwrap();
        fs::write(&interrupted, "partially replaced output").unwrap();
        let stale_transaction = directory.join(format!(
            "{}dead-process",
            super::TRANSACTION_DIRECTORY_PREFIX
        ));
        let stale_atomic = directory.join(format!(
            "{}dead-process",
            super::ATOMIC_WRITE_DIRECTORY_PREFIX
        ));
        fs::create_dir_all(&stale_transaction).unwrap();
        fs::write(stale_transaction.join("output.tmp"), "partial").unwrap();
        fs::create_dir_all(&stale_atomic).unwrap();
        fs::write(stale_atomic.join("tmpfile.tmp"), "partial").unwrap();

        reset_generated_writes();
        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();
        assert_eq!(
            generated_writes(),
            2 * FILES_PER_ROOT + 1,
            "an interrupted output replacement must invalidate the complete cache"
        );
        assert!(!stale_transaction.exists());
        assert!(!stale_atomic.exists());
        assert_no_temporary_entries(&out_dir);
        assert_eq!(read_manifest(&out_dir).outputs.len(), 2 * FILES_PER_ROOT);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn unchanged_generation_preserves_output_and_manifest_mtime() {
        let fixture = fixture("mtime");
        let manifest = fixture.join("manifest");
        let source_dir = manifest.join("src/ui");
        let out_dir = fixture.join("target/out");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("app.ice"), app_source("Stable", "same")).unwrap();

        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();
        let output = generated_path(&out_dir, "src/ui/app.ice").unwrap();
        let manifest_path = out_dir.join(GENERATED_DIRECTORY).join(GENERATED_MANIFEST);
        let output_mtime = fs::metadata(&output).unwrap().modified().unwrap();
        let manifest_mtime = fs::metadata(&manifest_path).unwrap().modified().unwrap();
        thread::sleep(Duration::from_millis(50));

        reset_generated_writes();
        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();

        assert_eq!(generated_writes(), 0);
        assert_eq!(
            fs::metadata(output).unwrap().modified().unwrap(),
            output_mtime
        );
        assert_eq!(
            fs::metadata(manifest_path).unwrap().modified().unwrap(),
            manifest_mtime
        );
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn generation_lock_serializes_concurrent_publishers() {
        let fixture = fixture("concurrent");
        let manifest = fixture.join("manifest");
        let source_dir = manifest.join("src/ui");
        let out_dir = fixture.join("target/out");
        let generated_directory = out_dir.join(GENERATED_DIRECTORY);
        fs::create_dir_all(&source_dir).unwrap();

        let first_lock = GenerationLock::acquire(&generated_directory).unwrap();
        let (acquired_sender, acquired_receiver) = mpsc::channel();
        let blocked_directory = generated_directory.clone();
        let blocked = thread::spawn(move || {
            let second_lock = GenerationLock::acquire(&blocked_directory).unwrap();
            acquired_sender.send(()).unwrap();
            drop(second_lock);
        });
        assert!(
            acquired_receiver
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "a second publisher acquired the generation directory while it was locked"
        );
        drop(first_lock);
        acquired_receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        blocked.join().unwrap();

        fs::write(source_dir.join("first.ice"), app_source("First", "one")).unwrap();
        fs::write(source_dir.join("second.ice"), app_source("Second", "two")).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let first = {
            let barrier = Arc::clone(&barrier);
            let manifest = manifest.clone();
            let out_dir = out_dir.clone();
            thread::spawn(move || {
                barrier.wait();
                compile_many_at(&manifest, &out_dir, &[PathBuf::from("src/ui/first.ice")]).unwrap();
            })
        };
        let second = {
            let barrier = Arc::clone(&barrier);
            let manifest = manifest.clone();
            let out_dir = out_dir.clone();
            thread::spawn(move || {
                barrier.wait();
                compile_many_at(&manifest, &out_dir, &[PathBuf::from("src/ui/second.ice")])
                    .unwrap();
            })
        };
        barrier.wait();
        first.join().unwrap();
        second.join().unwrap();

        let generated_manifest = read_manifest(&out_dir);
        assert_eq!(generated_manifest.outputs.len(), 2 * FILES_PER_ROOT);
        assert!(
            generated_path(&out_dir, "src/ui/first.ice")
                .unwrap()
                .is_file()
        );
        assert!(
            generated_path(&out_dir, "src/ui/second.ice")
                .unwrap()
                .is_file()
        );
        assert_no_temporary_entries(&out_dir);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    #[ignore = "CI performance contract; run explicitly"]
    fn performance_contract_compiles_one_hundred_roots_incrementally() {
        const ROOTS: usize = 100;
        const COLD_BUDGET: Duration = Duration::from_secs(10);
        const INCREMENTAL_BUDGET: Duration = Duration::from_secs(5);

        let fixture = std::env::temp_dir().join(format!(
            "ui-lang-build-performance-{}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let manifest = fixture.join("manifest");
        let source_dir = manifest.join("src/ui");
        let out_dir = fixture.join("target/out");
        fs::create_dir_all(&source_dir).unwrap();
        for index in 0..ROOTS {
            fs::write(
                source_dir.join(format!("app-{index:03}.ice")),
                format!(
                    concat!(
                        "app Performance{}\n",
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
                        "  text \"ready\"\n",
                    ),
                    index
                ),
            )
            .unwrap();
        }

        reset_generated_writes();
        let started = Instant::now();
        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();
        let cold = started.elapsed();
        assert_eq!(generated_writes(), ROOTS * FILES_PER_ROOT + 1);
        assert!(
            cold <= COLD_BUDGET,
            "cold AOT codegen for {ROOTS} roots took {cold:?}; budget is {COLD_BUDGET:?}"
        );

        reset_generated_writes();
        reset_content_comparison_reads();
        let started = Instant::now();
        compile_dir_at(&manifest, &out_dir, Path::new("src/ui")).unwrap();
        let incremental = started.elapsed();
        assert_eq!(
            generated_writes(),
            0,
            "unchanged incremental codegen must not rewrite generated outputs"
        );
        assert_eq!(
            content_comparison_reads(),
            1,
            "unchanged outputs were reread after manifest validation"
        );
        assert!(
            incremental <= INCREMENTAL_BUDGET,
            "incremental AOT codegen for {ROOTS} unchanged roots took {incremental:?}; budget is {INCREMENTAL_BUDGET:?}"
        );
        eprintln!(
            "{ROOTS} roots: cold {cold:?}, unchanged incremental {incremental:?}, incremental writes 0"
        );

        fs::remove_dir_all(fixture).unwrap();
    }
}
