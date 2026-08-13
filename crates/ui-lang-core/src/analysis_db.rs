use crate::source::{
    LoadedSource, Origin, analyze_loaded_without_assets, asset_dependencies, check_assets,
    file_error, parse_use,
};
use crate::{Error, FileAnalysis, FileCompilation, codegen, lower, source_is_app};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

const DEFAULT_VALIDATION_INTERVAL: Duration = Duration::from_millis(750);

/// The Ice language contract used by the current compiler.
pub const LANGUAGE_REVISION: &str = "2.0";

/// A stable SHA-256 digest of one source buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn of(source: &str) -> Self {
        Self::of_bytes(source.as_bytes())
    }

    pub fn of_bytes(source: &[u8]) -> Self {
        Self(Sha256::digest(source).into())
    }

    pub fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Cargo/compiler features that can change Ice analysis semantics.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CompilerFeatureSet(BTreeSet<String>);

impl CompilerFeatureSet {
    pub fn new(features: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(features.into_iter().map(Into::into).collect())
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

/// The complete identity of one cached parsed source file.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileKey {
    canonical_path: PathBuf,
    content_hash: ContentHash,
    language_revision: String,
    compiler_features: CompilerFeatureSet,
}

impl FileKey {
    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub fn content_hash(&self) -> ContentHash {
        self.content_hash
    }

    pub fn language_revision(&self) -> &str {
        &self.language_revision
    }

    pub fn compiler_features(&self) -> &CompilerFeatureSet {
        &self.compiler_features
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisConfig {
    language_revision: String,
    compiler_features: CompilerFeatureSet,
}

impl AnalysisConfig {
    pub fn new(
        language_revision: impl Into<String>,
        compiler_features: CompilerFeatureSet,
    ) -> Self {
        Self {
            language_revision: language_revision.into(),
            compiler_features,
        }
    }

    pub fn language_revision(&self) -> &str {
        &self.language_revision
    }

    pub fn compiler_features(&self) -> &CompilerFeatureSet {
        &self.compiler_features
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self::new(LANGUAGE_REVISION, CompilerFeatureSet::default())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnalysisTimings {
    pub load: Duration,
    pub check: Duration,
    pub codegen: Duration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnalysisMetrics {
    pub files_loaded: usize,
    pub bytes_loaded: usize,
    pub files_hashed: usize,
    pub bytes_hashed: usize,
    pub files_scanned: usize,
    /// Metadata probes used to prove that retained disk sources are fresh.
    pub source_stamps_checked: usize,
    /// Metadata probes used to prove that retained host assets are fresh.
    pub asset_stamps_checked: usize,
    pub roots_checked: usize,
    pub roots_reused: usize,
    pub root_cache_hits: usize,
    pub speculative_runs: usize,
    pub symbols_indexed: usize,
    pub codegen_roots: usize,
    pub elapsed: AnalysisTimings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnalysisInvalidation {
    pub changed: bool,
    pub affected_roots: BTreeSet<PathBuf>,
}

/// One source snapshot already stabilized by an external file watcher.
///
/// A watcher-backed caller must include every notified Ice source that belongs
/// to the queried root. Files omitted from the batch are treated as unchanged
/// and reuse their retained parsed representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedSource {
    path: PathBuf,
    contents: Option<Vec<u8>>,
}

impl ValidatedSource {
    pub fn new(path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            contents: Some(contents.into()),
        }
    }

    pub fn missing(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            contents: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contents(&self) -> Option<&[u8]> {
        self.contents.as_deref()
    }
}

/// Bounds how long a retained result may skip disk validation.
///
/// Metadata checks catch ordinary edits cheaply. Content checks are the
/// authoritative fallback for timestamp-preserving writes and replacements.
/// The default gives consumers without file watching bounded freshness without
/// probing every file on every query. Watcher-backed consumers may use longer
/// intervals together with [`AnalysisDb::refresh_input`], while operations that
/// require an immediately current disk snapshot use
/// [`AnalysisDb::query_root_fresh`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationPolicy {
    pub metadata_interval: Duration,
    pub content_interval: Duration,
}

impl ValidationPolicy {
    pub const fn new(metadata_interval: Duration, content_interval: Duration) -> Self {
        Self {
            metadata_interval,
            content_interval,
        }
    }

    pub const fn always() -> Self {
        Self::new(Duration::ZERO, Duration::ZERO)
    }
}

impl Default for ValidationPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_VALIDATION_INTERVAL, DEFAULT_VALIDATION_INTERVAL)
    }
}

#[derive(Clone, Debug)]
struct OverlaySource {
    canonical_path: PathBuf,
    source: Arc<str>,
    revision: u64,
}

#[derive(Clone, Debug, Default)]
struct OverlayStore {
    sources_by_path: HashMap<PathBuf, Arc<str>>,
    aliases: HashMap<PathBuf, PathBuf>,
    sources_by_alias: HashMap<PathBuf, OverlaySource>,
    revision: u64,
}

#[derive(Clone, Debug)]
enum ParsedLine {
    Import {
        path: String,
        alias: Option<String>,
        line: usize,
    },
    Source {
        range: Range<usize>,
        line: usize,
    },
}

#[derive(Clone, Debug)]
struct ParsedFile {
    source: Arc<str>,
    lines: Vec<ParsedLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RootFingerprint(Vec<(FileKey, Option<String>)>);

#[derive(Clone, Debug)]
struct CheckedRoot {
    fingerprint: RootFingerprint,
    analysis: Arc<FileAnalysis>,
    source_stamps: Vec<(PathBuf, DiskStamp)>,
    asset_stamps: Vec<(PathBuf, DiskStamp)>,
    metadata_validated_at: Instant,
    content_validated_at: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DiskStamp {
    link_target: Option<PathBuf>,
    resolved_path: Option<PathBuf>,
    len: u64,
    modified: Option<SystemTime>,
    is_file: bool,
    content_hash: Option<ContentHash>,
}

type DiskStamps = Vec<(PathBuf, DiskStamp)>;

impl DiskStamp {
    fn read(path: &Path, content_hash: Option<ContentHash>) -> Self {
        let link_target = fs::symlink_metadata(path)
            .ok()
            .filter(|metadata| metadata.file_type().is_symlink())
            .and_then(|_| fs::read_link(path).ok());
        let resolved_path = path.canonicalize().ok();
        match fs::metadata(path) {
            Err(_) => Self {
                link_target,
                resolved_path,
                len: 0,
                modified: None,
                is_file: false,
                content_hash,
            },
            Ok(metadata) => Self {
                link_target,
                resolved_path,
                len: metadata.len(),
                modified: metadata.modified().ok(),
                is_file: metadata.is_file(),
                content_hash,
            },
        }
    }

    fn same_identity(&self, other: &Self) -> bool {
        self.link_target == other.link_target
            && self.resolved_path == other.resolved_path
            && self.len == other.len
            && self.modified == other.modified
            && self.is_file == other.is_file
    }

    fn same_resolved_input(&self, other: &Self) -> bool {
        self.link_target == other.link_target && self.resolved_path == other.resolved_path
    }

    fn matches_path(&self, path: &Path) -> bool {
        self.resolved_path.as_deref() == Some(path)
    }
}

#[derive(Debug)]
struct LoadedGraph {
    loaded: LoadedSource,
    fingerprint: RootFingerprint,
    dependency_paths: HashSet<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceLoad {
    Disk,
    Retained,
    RetainedRoot,
}

struct GraphLoader {
    graph: LoadedGraph,
    included: HashSet<(PathBuf, Option<String>)>,
    stack: Vec<PathBuf>,
    source_load: SourceLoad,
}

/// Process-local incremental storage for Ice source graphs.
///
/// The owner controls its lifetime. The DB never writes a cache to disk and it
/// does not use global state, so LSP/dev processes can retain it while one-shot
/// callers can create it for a single command.
#[derive(Debug, Default)]
pub struct AnalysisDb {
    config: AnalysisConfig,
    overlay_store: Arc<OverlayStore>,
    inherited_overlay_store: Option<Arc<OverlayStore>>,
    parsed_files: HashMap<FileKey, Arc<ParsedFile>>,
    current_files: HashMap<PathBuf, FileKey>,
    dependencies: HashMap<PathBuf, BTreeSet<PathBuf>>,
    reverse_dependencies: HashMap<PathBuf, BTreeSet<PathBuf>>,
    root_assets: HashMap<PathBuf, BTreeSet<PathBuf>>,
    asset_roots: HashMap<PathBuf, BTreeSet<PathBuf>>,
    known_roots: BTreeSet<PathBuf>,
    dirty_roots: BTreeSet<PathBuf>,
    checked_roots: HashMap<PathBuf, CheckedRoot>,
    metrics: AnalysisMetrics,
    validation_policy: ValidationPolicy,
}

impl AnalysisDb {
    pub fn new(config: AnalysisConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    pub fn config(&self) -> &AnalysisConfig {
        &self.config
    }

    pub fn metrics(&self) -> AnalysisMetrics {
        self.metrics
    }

    pub fn take_metrics(&mut self) -> AnalysisMetrics {
        std::mem::take(&mut self.metrics)
    }

    pub fn set_validation_policy(&mut self, policy: ValidationPolicy) {
        self.validation_policy = policy;
    }

    pub fn validation_policy(&self) -> ValidationPolicy {
        self.validation_policy
    }

    /// Analyze one candidate root overlay against a snapshot of retained state.
    ///
    /// Parsed and checked state is limited to the root's current source closure,
    /// while every open overlay is shared by `Arc` so a candidate may add an
    /// import of an unsaved file. The retained workspace DB is never mutated.
    pub fn analyze_overlay_candidate(
        &mut self,
        path: impl AsRef<Path>,
        source: impl Into<String>,
    ) -> Result<Arc<FileAnalysis>, Error> {
        let root = self.normalize_db_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve candidate path: {error}"),
            )
        })?;
        let mut candidate = self.root_snapshot(&root);
        candidate.set_overlay(&root, source)?;
        let result = candidate.query_root(&root);
        self.metrics.files_loaded += candidate.metrics.files_loaded;
        self.metrics.bytes_loaded += candidate.metrics.bytes_loaded;
        self.metrics.files_hashed += candidate.metrics.files_hashed;
        self.metrics.bytes_hashed += candidate.metrics.bytes_hashed;
        self.metrics.files_scanned += candidate.metrics.files_scanned;
        self.metrics.source_stamps_checked += candidate.metrics.source_stamps_checked;
        self.metrics.asset_stamps_checked += candidate.metrics.asset_stamps_checked;
        self.metrics.roots_checked += candidate.metrics.roots_checked;
        self.metrics.roots_reused += candidate.metrics.roots_reused;
        self.metrics.root_cache_hits += candidate.metrics.root_cache_hits;
        self.metrics.symbols_indexed += candidate.metrics.symbols_indexed;
        self.metrics.codegen_roots += candidate.metrics.codegen_roots;
        self.metrics.elapsed.load += candidate.metrics.elapsed.load;
        self.metrics.elapsed.check += candidate.metrics.elapsed.check;
        self.metrics.elapsed.codegen += candidate.metrics.elapsed.codegen;
        self.metrics.speculative_runs += 1 + candidate.metrics.speculative_runs;
        result
    }

    pub fn parsed_file_count(&self) -> usize {
        self.parsed_files.len()
    }

    pub fn checked_root_count(&self) -> usize {
        self.checked_roots.len()
    }

    /// Roots whose checked result is absent or invalid after an input failure.
    pub fn dirty_roots(&self) -> &BTreeSet<PathBuf> {
        &self.dirty_roots
    }

    /// Whether a root must be analyzed before its previous result can be used.
    pub fn needs_analysis(&self, path: impl AsRef<Path>) -> bool {
        let Ok(path) = self.normalize_db_path(path.as_ref()) else {
            return true;
        };
        !self.known_roots.contains(&path)
            || self.dirty_roots.contains(&path)
            || !self.checked_roots.contains_key(&path)
    }

    /// Stop retaining a checked root while keeping reusable parsed imports.
    pub fn forget_root(&mut self, path: impl AsRef<Path>) -> bool {
        let Ok(path) = self.normalize_db_path(path.as_ref()) else {
            return false;
        };
        self.known_roots.remove(&path);
        self.dirty_roots.remove(&path);
        self.replace_root_assets(path.clone(), BTreeSet::new());
        let removed = self.checked_roots.remove(&path).is_some();
        self.prune_unreachable_files();
        removed
    }

    pub fn dependencies(&self, path: impl AsRef<Path>) -> BTreeSet<PathBuf> {
        self.normalize_db_path(path.as_ref())
            .ok()
            .and_then(|path| self.dependencies.get(&path).cloned())
            .unwrap_or_default()
    }

    pub fn reverse_dependencies(&self, path: impl AsRef<Path>) -> BTreeSet<PathBuf> {
        self.normalize_db_path(path.as_ref())
            .ok()
            .and_then(|path| self.reverse_dependencies.get(&path).cloned())
            .unwrap_or_default()
    }

    /// Install or replace an editor buffer and invalidate only roots reachable
    /// through the reverse import graph.
    pub fn set_overlay(
        &mut self,
        path: impl AsRef<Path>,
        source: impl Into<String>,
    ) -> Result<AnalysisInvalidation, Error> {
        let source = source.into();
        let lexical_path = absolute_lexical_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve overlay path: {error}"),
            )
        })?;
        let path = self
            .overlay_alias(&lexical_path)
            .cloned()
            .map_or_else(|| normalize_path(&lexical_path), Ok)
            .map_err(|error| {
                file_error(
                    "E181",
                    &lexical_path,
                    1,
                    format!("cannot resolve overlay path: {error}"),
                )
            })?;
        let source: Arc<str> = Arc::from(source);
        let previous = self.overlay_source(&path).cloned();
        let revision = self
            .overlay_store
            .revision
            .max(
                self.inherited_overlay_store
                    .as_deref()
                    .map_or(0, |store| store.revision),
            )
            .checked_add(1)
            .expect("overlay revision exhausted");
        let store = Arc::make_mut(&mut self.overlay_store);
        store.revision = revision;
        store.aliases.insert(lexical_path.clone(), path.clone());
        store.sources_by_alias.insert(
            lexical_path,
            OverlaySource {
                canonical_path: path.clone(),
                source: Arc::clone(&source),
                revision,
            },
        );
        if previous
            .as_ref()
            .is_some_and(|current| current.as_ref() == source.as_ref())
        {
            return Ok(AnalysisInvalidation {
                changed: false,
                affected_roots: self.dirty_roots.clone(),
            });
        }
        let key = self.file_key(path.clone(), &source);
        let changed = self.current_files.get(&path) != Some(&key);
        Arc::make_mut(&mut self.overlay_store)
            .sources_by_path
            .insert(path.clone(), source);
        if !changed {
            return Ok(AnalysisInvalidation {
                changed: false,
                affected_roots: self.dirty_roots.clone(),
            });
        }
        let affected_roots = self.replace_current_key(path, key);
        Ok(AnalysisInvalidation {
            changed: true,
            affected_roots,
        })
    }

    /// Close an editor buffer and restore the corresponding disk source.
    pub fn remove_overlay(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<AnalysisInvalidation, Error> {
        let lexical_path = absolute_lexical_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve overlay path: {error}"),
            )
        })?;
        let (opened, opened_path) = {
            let store = Arc::make_mut(&mut self.overlay_store);
            (
                store.sources_by_alias.remove(&lexical_path),
                store.aliases.remove(&lexical_path),
            )
        };
        let path = opened_path
            .or_else(|| opened.as_ref().map(|source| source.canonical_path.clone()))
            .map_or_else(|| normalize_path(&lexical_path), Ok)
            .map_err(|error| {
                file_error(
                    "E181",
                    &lexical_path,
                    1,
                    format!("cannot resolve overlay path: {error}"),
                )
            })?;
        if let Some(replacement) = self
            .overlay_store
            .sources_by_alias
            .values()
            .filter(|source| source.canonical_path == path)
            .max_by_key(|source| source.revision)
            .map(|source| Arc::clone(&source.source))
        {
            let unchanged = self
                .overlay_source(&path)
                .is_some_and(|current| current.as_ref() == replacement.as_ref());
            Arc::make_mut(&mut self.overlay_store)
                .sources_by_path
                .insert(path.clone(), Arc::clone(&replacement));
            if unchanged {
                return Ok(AnalysisInvalidation::default());
            }
            let key = self.file_key(path.clone(), &replacement);
            let affected_roots = self.replace_current_key(path, key);
            return Ok(AnalysisInvalidation {
                changed: true,
                affected_roots,
            });
        }
        let Some(previous) = Arc::make_mut(&mut self.overlay_store)
            .sources_by_path
            .remove(&path)
        else {
            return Ok(AnalysisInvalidation::default());
        };
        let disk = match fs::read_to_string(&path) {
            Ok(disk) => {
                self.metrics.files_loaded += 1;
                self.metrics.bytes_loaded += disk.len();
                disk
            }
            Err(error) => {
                self.remove_current_file(&path);
                return Err(file_error(
                    "E181",
                    &path,
                    1,
                    format!("cannot read .ice file: {error}"),
                ));
            }
        };
        if previous.as_ref() == disk {
            let key = self.file_key(path.clone(), &disk);
            self.replace_current_key(path, key);
            return Ok(AnalysisInvalidation {
                changed: false,
                affected_roots: self.dirty_roots.clone(),
            });
        }
        let key = self.file_key(path.clone(), &disk);
        let affected_roots = self.replace_current_key(path, key);
        Ok(AnalysisInvalidation {
            changed: true,
            affected_roots,
        })
    }

    /// Refresh one disk input. Equal content keeps every checked root reusable.
    pub fn refresh_file(&mut self, path: impl AsRef<Path>) -> Result<AnalysisInvalidation, Error> {
        let path = self.normalize_db_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        let source = match self.read_source(&path) {
            Ok(source) => source,
            Err(error) => {
                self.remove_current_file(&path);
                return Err(error);
            }
        };
        let key = self.file_key(path.clone(), &source);
        if self.current_files.get(&path) == Some(&key) {
            return Ok(AnalysisInvalidation {
                changed: false,
                affected_roots: self.dirty_roots.clone(),
            });
        }
        let affected_roots = self.replace_current_key(path, key);
        Ok(AnalysisInvalidation {
            changed: true,
            affected_roots,
        })
    }

    pub fn analyze_root(&mut self, path: impl AsRef<Path>) -> Result<FileAnalysis, Error> {
        self.analyze_root_shared(path, SourceLoad::Disk)
            .map(|analysis| analysis.as_ref().clone())
    }

    /// Analyze a root from a watcher-validated change batch.
    ///
    /// The supplied contents replace only their matching retained disk inputs.
    /// Every other file in the previously loaded source closure is reused
    /// without another read, hash, or import scan. A newly imported file still
    /// falls back to disk because it is not part of that retained closure.
    pub fn analyze_root_with_validated_sources(
        &mut self,
        path: impl AsRef<Path>,
        sources: impl IntoIterator<Item = ValidatedSource>,
    ) -> Result<FileAnalysis, Error> {
        for source in sources {
            self.install_validated_source(source)?;
        }
        self.analyze_root_shared(path, SourceLoad::Retained)
            .map(|analysis| analysis.as_ref().clone())
    }

    fn analyze_root_shared(
        &mut self,
        path: impl AsRef<Path>,
        source_load: SourceLoad,
    ) -> Result<Arc<FileAnalysis>, Error> {
        let requested = path.as_ref().to_owned();
        let requested_path = absolute_lexical_path(&requested).map_err(|error| {
            file_error(
                "E181",
                &requested,
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        let root = self.normalize_db_path(&requested).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        self.known_roots.insert(root.clone());
        self.dirty_roots.insert(root.clone());
        let started = Instant::now();
        let graph = self.load_graph(&root, source_load);
        self.metrics.elapsed.load += started.elapsed();
        let graph = graph?;
        if let Some(cached) = self.checked_roots.get(&root)
            && cached.fingerprint == graph.fingerprint
        {
            let cached_analysis = Arc::clone(&cached.analysis);
            check_assets(&cached_analysis.document, &graph.loaded)
                .map_err(|error| crate::source::remap_error(error, &graph.loaded))?;
            let (source_stamps, asset_stamps) = self.analysis_stamps(&cached_analysis);
            let now = Instant::now();
            if let Some(cached) = self.checked_roots.get_mut(&root) {
                cached.source_stamps = source_stamps;
                cached.asset_stamps = asset_stamps;
                cached.metadata_validated_at = now;
                cached.content_validated_at = now;
            }
            self.metrics.roots_reused += 1;
            self.dirty_roots.remove(&root);
            if cached_analysis.dependencies.contains(&requested_path) {
                return Ok(cached_analysis);
            }
            let mut analysis = cached_analysis.as_ref().clone();
            analysis.dependencies.push(requested_path);
            analysis.dependencies.sort();
            analysis.dependencies.dedup();
            return Ok(Arc::new(analysis));
        }

        let started = Instant::now();
        self.metrics.roots_checked += 1;
        let document = analyze_loaded_without_assets(&graph.loaded);
        self.metrics.elapsed.check += started.elapsed();
        let document = document?;
        let asset_dependencies = asset_dependencies(&document, &graph.loaded);
        self.replace_root_assets(root.clone(), asset_dependencies.iter().cloned().collect());
        check_assets(&document, &graph.loaded)
            .map_err(|error| crate::source::remap_error(error, &graph.loaded))?;
        self.metrics.symbols_indexed += document.symbols().len();
        let mut dependencies = graph.loaded.dependencies.clone();
        dependencies.push(requested_path);
        dependencies.sort();
        dependencies.dedup();
        let analysis = Arc::new(FileAnalysis {
            document,
            dependencies,
            asset_dependencies,
        });
        let (source_stamps, asset_stamps) = self.analysis_stamps(&analysis);
        let now = Instant::now();
        self.checked_roots.insert(
            root.clone(),
            CheckedRoot {
                fingerprint: graph.fingerprint,
                analysis: Arc::clone(&analysis),
                source_stamps,
                asset_stamps,
                metadata_validated_at: now,
                content_validated_at: now,
            },
        );
        self.dirty_roots.remove(&root);
        Ok(analysis)
    }

    /// Return a retained checked root after validating disk inputs on the
    /// configured bounded metadata/content epochs. Watch notifications are only
    /// an eager invalidation hint; correctness does not depend on clients
    /// supporting or delivering them.
    pub fn query_root(&mut self, path: impl AsRef<Path>) -> Result<Arc<FileAnalysis>, Error> {
        self.query_root_with_validation(path.as_ref(), false)
    }

    /// Return a retained checked root only after validating every closed disk
    /// source and asset by content, regardless of the configured epochs.
    ///
    /// Use this for correctness-sensitive operations, such as workspace rename,
    /// whose edits must be computed from an immediately current snapshot.
    pub fn query_root_fresh(&mut self, path: impl AsRef<Path>) -> Result<Arc<FileAnalysis>, Error> {
        self.query_root_with_validation(path.as_ref(), true)
    }

    fn query_root_with_validation(
        &mut self,
        path: &Path,
        force_content_validation: bool,
    ) -> Result<Arc<FileAnalysis>, Error> {
        let requested = path.to_owned();
        let root = self.normalize_db_path(&requested).map_err(|error| {
            file_error(
                "E181",
                &requested,
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        self.refresh_root_inputs(&root, force_content_validation)?;
        if !self.dirty_roots.contains(&root)
            && let Some(cached) = self.checked_roots.get(&root)
        {
            self.metrics.root_cache_hits += 1;
            return Ok(Arc::clone(&cached.analysis));
        }

        self.analyze_root_shared(&requested, SourceLoad::Disk)
    }

    /// Refresh a watched path only when it belongs to a retained source graph
    /// or asset set. Unrelated workspace notifications do no source I/O.
    pub fn refresh_input(&mut self, path: impl AsRef<Path>) -> Result<AnalysisInvalidation, Error> {
        let lexical_path = absolute_lexical_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve input path: {error}"),
            )
        })?;
        let resolved_path = normalize_path(&lexical_path).unwrap_or_else(|_| lexical_path.clone());
        let mut affected_roots = self.roots_for_input(&lexical_path, &resolved_path);
        let resolution_changed = self.input_resolution_changed(&lexical_path);
        let source_input = self.current_files.contains_key(&resolved_path)
            || self.reverse_dependencies.contains_key(&resolved_path)
            || self.reverse_dependencies.contains_key(&lexical_path);
        if source_input {
            match self.refresh_file(&lexical_path) {
                Ok(invalidation) if !invalidation.changed && !resolution_changed => {
                    return Ok(invalidation);
                }
                Ok(invalidation) => affected_roots.extend(invalidation.affected_roots),
                Err(error) => {
                    for root in &affected_roots {
                        self.checked_roots.remove(root);
                        self.dirty_roots.insert(root.clone());
                    }
                    return Err(error);
                }
            }
        }
        for root in &affected_roots {
            self.checked_roots.remove(root);
            self.dirty_roots.insert(root.clone());
        }
        Ok(AnalysisInvalidation {
            changed: !affected_roots.is_empty(),
            affected_roots,
        })
    }

    pub fn analyze_roots(
        &mut self,
        roots: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Vec<Result<FileAnalysis, Error>> {
        roots
            .into_iter()
            .map(|root| self.analyze_root(root))
            .collect()
    }

    /// Publishes a root's view as data without generating any Rust.
    ///
    /// This is the reload path: parse, check, and lower still run — so an edit
    /// is still diagnosed — but nothing reaches the compiler. `None` means the
    /// view uses constructs only compiled Rust expresses, and the caller must
    /// fall back to rebuilding.
    pub fn view_template(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<Option<codegen::ViewTemplate>, Error> {
        let path = normalize_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        let analysis = self.analyze_root(&path)?;
        let source_origins = analysis.document.source_origins().to_vec();
        let program = lower::lower(analysis.document)
            .map_err(|error| remap_origin(error, &source_origins))?;
        codegen::view_template(&program, &path.display().to_string()).map_err(|mut error| {
            if let Some((origin, line)) = program.source_origin(error.line) {
                error.path = Some(origin.display().to_string());
                error.line = line;
            }
            error
        })
    }

    pub fn compile_root(&mut self, path: impl AsRef<Path>) -> Result<FileCompilation, Error> {
        let path = normalize_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        let analysis = self.analyze_root(&path)?;
        self.compile_analysis(path, analysis)
    }

    /// Compile a root from a source snapshot already stabilized by the caller.
    ///
    /// Imports are still loaded from disk, including files retained while
    /// compiling another root.
    pub fn compile_root_with_validated_source(
        &mut self,
        path: impl AsRef<Path>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<FileCompilation, Error> {
        let path = normalize_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        self.install_validated_source(ValidatedSource::new(&path, contents))?;
        let analysis = self
            .analyze_root_shared(&path, SourceLoad::RetainedRoot)?
            .as_ref()
            .clone();
        self.compile_analysis(path, analysis)
    }

    fn compile_analysis(
        &mut self,
        path: PathBuf,
        analysis: FileAnalysis,
    ) -> Result<FileCompilation, Error> {
        let started = Instant::now();
        let source_origins = analysis.document.source_origins().to_vec();
        let program = lower::lower(analysis.document)
            .map_err(|error| remap_origin(error, &source_origins))?;
        let mut rust =
            codegen::generate(&program, &path.display().to_string()).map_err(|mut error| {
                if let Some((origin, line)) = program.source_origin(error.line) {
                    error.path = Some(origin.display().to_string());
                    error.line = line;
                }
                error
            })?;
        for dependency in analysis.dependencies.iter().filter(|entry| *entry != &path) {
            rust.push_str(&format!(
                "const _: &str = include_str!({:?});\n",
                dependency.display().to_string()
            ));
        }
        self.metrics.elapsed.codegen += started.elapsed();
        self.metrics.codegen_roots += 1;
        Ok(FileCompilation {
            rust,
            dependencies: analysis.dependencies,
            asset_dependencies: analysis.asset_dependencies,
        })
    }

    fn file_key(&mut self, canonical_path: PathBuf, source: &str) -> FileKey {
        self.metrics.files_hashed += 1;
        self.metrics.bytes_hashed += source.len();
        FileKey {
            canonical_path,
            content_hash: ContentHash::of(source),
            language_revision: self.config.language_revision.clone(),
            compiler_features: self.config.compiler_features.clone(),
        }
    }

    fn refresh_root_inputs(
        &mut self,
        root: &Path,
        force_content_validation: bool,
    ) -> Result<(), Error> {
        let Some(checked) = self.checked_roots.get(root) else {
            return Ok(());
        };
        let now = Instant::now();
        let metadata_due = force_content_validation
            || now.duration_since(checked.metadata_validated_at)
                >= self.validation_policy.metadata_interval;
        let content_due = force_content_validation
            || now.duration_since(checked.content_validated_at)
                >= self.validation_policy.content_interval;
        if !metadata_due && !content_due {
            return Ok(());
        }
        let source_stamps = checked.source_stamps.clone();
        let asset_stamps = checked.asset_stamps.clone();
        let mut next_sources = Vec::with_capacity(source_stamps.len());
        let mut next_assets = Vec::with_capacity(asset_stamps.len());

        for (path, previous) in source_stamps {
            self.metrics.source_stamps_checked += 1;
            let mut metadata = DiskStamp::read(&path, None);
            metadata.content_hash = previous.content_hash;
            let current = if content_due || !metadata.same_identity(&previous) {
                self.disk_stamp_with_content(&path)
            } else {
                metadata
            };
            if !current.same_resolved_input(&previous)
                || !current.is_file
                || current.content_hash != previous.content_hash
            {
                self.checked_roots.remove(root);
                self.dirty_roots.insert(root.to_owned());
                return Ok(());
            }
            next_sources.push((path, current));
        }
        for (path, previous) in asset_stamps {
            self.metrics.asset_stamps_checked += 1;
            let mut metadata = DiskStamp::read(&path, None);
            metadata.content_hash = previous.content_hash;
            let current = if content_due || !metadata.same_identity(&previous) {
                self.disk_stamp_with_content(&path)
            } else {
                metadata
            };
            if !current.same_resolved_input(&previous)
                || !current.is_file
                || current.content_hash != previous.content_hash
            {
                self.checked_roots.remove(root);
                self.dirty_roots.insert(root.to_owned());
                return Ok(());
            }
            next_assets.push((path, current));
        }

        if let Some(checked) = self.checked_roots.get_mut(root) {
            checked.source_stamps = next_sources;
            checked.asset_stamps = next_assets;
            checked.metadata_validated_at = now;
            if content_due {
                checked.content_validated_at = now;
            }
        }
        Ok(())
    }

    fn disk_stamp_with_content(&mut self, path: &Path) -> DiskStamp {
        let content_hash = fs::read(path).ok().map(|bytes| {
            self.metrics.files_loaded += 1;
            self.metrics.bytes_loaded += bytes.len();
            self.metrics.files_hashed += 1;
            self.metrics.bytes_hashed += bytes.len();
            ContentHash::of_bytes(&bytes)
        });
        DiskStamp::read(path, content_hash)
    }

    fn analysis_stamps(&mut self, analysis: &FileAnalysis) -> (DiskStamps, DiskStamps) {
        let source_paths = analysis
            .dependencies
            .iter()
            .filter(|path| !self.is_overlay_path(path))
            .cloned()
            .collect::<Vec<_>>();
        let source_stamps = source_paths
            .into_iter()
            .map(|path| {
                let content_hash = normalize_path(&path)
                    .ok()
                    .and_then(|resolved| self.current_files.get(&resolved))
                    .map(FileKey::content_hash)
                    .or_else(|| self.disk_stamp_with_content(&path).content_hash);
                let stamp = DiskStamp::read(&path, content_hash);
                (path, stamp)
            })
            .collect();
        let asset_paths = analysis.asset_dependencies.clone();
        let asset_stamps = asset_paths
            .into_iter()
            .map(|path| {
                let stamp = self.disk_stamp_with_content(&path);
                (path, stamp)
            })
            .collect();
        (source_stamps, asset_stamps)
    }

    fn read_source(&mut self, path: &Path) -> Result<Arc<str>, Error> {
        let source = if let Some(source) = self.overlay_source(path) {
            Arc::clone(source)
        } else {
            fs::read_to_string(path).map(Arc::from).map_err(|error| {
                file_error("E181", path, 1, format!("cannot read .ice file: {error}"))
            })?
        };
        self.metrics.files_loaded += 1;
        self.metrics.bytes_loaded += source.len();
        Ok(source)
    }

    fn install_validated_source(&mut self, source: ValidatedSource) -> Result<(), Error> {
        let lexical_path = absolute_lexical_path(&source.path).map_err(|error| {
            file_error(
                "E181",
                &source.path,
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        if self.is_overlay_path(&lexical_path) {
            return Ok(());
        }
        let path = normalize_path(&lexical_path).map_err(|error| {
            file_error(
                "E181",
                &source.path,
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        let Some(contents) = source.contents else {
            self.remove_current_file(&path);
            return Ok(());
        };
        self.metrics.files_loaded += 1;
        self.metrics.bytes_loaded += contents.len();
        let source = String::from_utf8(contents).map_err(|error| {
            self.remove_current_file(&path);
            file_error("E181", &path, 1, format!("cannot read .ice file: {error}"))
        })?;
        let source: Arc<str> = Arc::from(source);
        let key = self.file_key(path.clone(), &source);
        if self.current_files.get(&path) == Some(&key) && self.parsed_files.contains_key(&key) {
            return Ok(());
        }
        self.replace_current_key(path.clone(), key.clone());
        let parsed = self.scan_source(&path, Arc::clone(&source))?;
        self.parsed_files.insert(key, parsed);
        Ok(())
    }

    fn parsed_file(
        &mut self,
        path: &Path,
        source_load: SourceLoad,
    ) -> Result<(FileKey, Arc<ParsedFile>), Error> {
        if source_load == SourceLoad::Retained
            && let Some(key) = self.current_files.get(path)
            && let Some(parsed) = self.parsed_files.get(key)
        {
            return Ok((key.clone(), Arc::clone(parsed)));
        }
        let source = match self.read_source(path) {
            Ok(source) => source,
            Err(error) => {
                self.remove_current_file(path);
                return Err(error);
            }
        };
        let key = self.file_key(path.to_owned(), &source);
        if self.current_files.get(path) != Some(&key) {
            self.replace_current_key(path.to_owned(), key.clone());
        }
        if let Some(parsed) = self.parsed_files.get(&key) {
            return Ok((key, Arc::clone(parsed)));
        }

        let parsed = self.scan_source(path, source)?;
        self.parsed_files.insert(key.clone(), Arc::clone(&parsed));
        Ok((key, parsed))
    }

    fn scan_source(&mut self, path: &Path, source: Arc<str>) -> Result<Arc<ParsedFile>, Error> {
        let mut lines = Vec::new();
        let mut offset = 0;
        for (index, chunk) in source.split_inclusive('\n').enumerate() {
            let raw = chunk.strip_suffix('\n').unwrap_or(chunk);
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let line = index + 1;
            if raw.len() == raw.trim_start().len() && raw.starts_with("use ") {
                let import = parse_use(raw, path, line)?;
                lines.push(ParsedLine::Import {
                    path: import.path.to_owned(),
                    alias: import.alias.map(str::to_owned),
                    line,
                });
            } else {
                lines.push(ParsedLine::Source {
                    range: offset..offset + raw.len(),
                    line,
                });
            }
            offset += chunk.len();
        }
        let parsed = Arc::new(ParsedFile { source, lines });
        self.metrics.files_scanned += 1;
        Ok(parsed)
    }

    fn load_graph(&mut self, root: &Path, source_load: SourceLoad) -> Result<LoadedGraph, Error> {
        let root_source_load = match source_load {
            SourceLoad::RetainedRoot => SourceLoad::Retained,
            source_load => source_load,
        };
        let (_, parsed) = self.parsed_file(root, root_source_load)?;
        if !source_is_app(&parsed.source) {
            return Err(file_error(
                "E183",
                root,
                1,
                "a root must declare `app Name` or `daemon Name`; import this fragment from an app or daemon instead",
            ));
        }
        let mut loader = GraphLoader {
            graph: LoadedGraph {
                loaded: LoadedSource {
                    source: String::new(),
                    origins: Vec::new(),
                    dependencies: Vec::new(),
                },
                fingerprint: RootFingerprint(Vec::new()),
                dependency_paths: HashSet::new(),
            },
            included: HashSet::new(),
            stack: Vec::new(),
            source_load,
        };
        self.load_into(root, (root, 1), None, &mut loader)?;
        Ok(loader.graph)
    }

    fn load_into(
        &mut self,
        path: &Path,
        imported_from: (&Path, usize),
        namespace: Option<String>,
        loader: &mut GraphLoader,
    ) -> Result<(), Error> {
        if let Some(start) = loader.stack.iter().position(|entry| entry == path) {
            let mut cycle = loader.stack[start..]
                .iter()
                .map(|entry| entry.display().to_string())
                .collect::<Vec<_>>();
            cycle.push(path.display().to_string());
            return Err(file_error(
                "E182",
                imported_from.0,
                imported_from.1,
                format!("cyclic `use`: {}", cycle.join(" -> ")),
            ));
        }
        if !loader.included.insert((path.to_owned(), namespace.clone())) {
            return Ok(());
        }
        let source_load = match loader.source_load {
            SourceLoad::RetainedRoot if loader.stack.is_empty() => SourceLoad::Retained,
            SourceLoad::RetainedRoot => SourceLoad::Disk,
            source_load => source_load,
        };
        loader.stack.push(path.to_owned());
        if loader.graph.dependency_paths.insert(path.to_owned()) {
            loader.graph.loaded.dependencies.push(path.to_owned());
        }
        let (key, parsed) = self.parsed_file(path, source_load)?;
        loader.graph.fingerprint.0.push((key, namespace.clone()));
        let mut direct = BTreeSet::new();
        let mut resolved_imports = Vec::new();
        let mut first_import_error = None;
        for line in &parsed.lines {
            let ParsedLine::Import {
                path: relative,
                alias,
                line,
            } = line
            else {
                continue;
            };
            let candidate = path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(relative);
            if let Ok(candidate_path) = absolute_lexical_path(&candidate)
                && loader.graph.dependency_paths.insert(candidate_path.clone())
            {
                loader.graph.loaded.dependencies.push(candidate_path);
            }
            match self.resolve_import(&candidate, path, *line) {
                Ok(target) => {
                    direct.insert(target.clone());
                    let child_namespace = alias.as_ref().map_or_else(
                        || namespace.clone(),
                        |alias| {
                            Some(namespace.as_ref().map_or_else(
                                || alias.clone(),
                                |parent| format!("{parent}::{alias}"),
                            ))
                        },
                    );
                    resolved_imports.push((target, child_namespace, *line));
                }
                Err(error) => {
                    if let Ok(target) = normalize_path(&candidate) {
                        direct.insert(target);
                    }
                    if first_import_error.is_none() {
                        first_import_error = Some(error);
                    }
                }
            }
        }
        self.replace_dependencies(path.to_owned(), direct);
        if let Some(error) = first_import_error {
            return Err(error);
        }

        let mut imports = resolved_imports.into_iter();
        for line in &parsed.lines {
            match line {
                ParsedLine::Import { .. } => {
                    let (target, child_namespace, line) = imports
                        .next()
                        .expect("every resolved import retains its source position");
                    self.load_into(&target, (path, line), child_namespace, loader)?;
                }
                ParsedLine::Source { range, line } => {
                    loader
                        .graph
                        .loaded
                        .source
                        .push_str(&parsed.source[range.clone()]);
                    loader.graph.loaded.source.push('\n');
                    loader.graph.loaded.origins.push(Origin {
                        path: path.to_owned(),
                        line: *line,
                        namespace: namespace.clone(),
                    });
                }
            }
        }
        loader.stack.pop();
        Ok(())
    }

    fn replace_dependencies(&mut self, source: PathBuf, next: BTreeSet<PathBuf>) {
        let previous = if next.is_empty() {
            self.dependencies.remove(&source)
        } else {
            self.dependencies.insert(source.clone(), next.clone())
        };
        if let Some(previous) = previous {
            for dependency in previous.difference(&next) {
                if let Some(reverse) = self.reverse_dependencies.get_mut(dependency) {
                    reverse.remove(&source);
                    if reverse.is_empty() {
                        self.reverse_dependencies.remove(dependency);
                    }
                }
            }
        }
        for dependency in next {
            self.reverse_dependencies
                .entry(dependency)
                .or_default()
                .insert(source.clone());
        }
    }

    fn replace_root_assets(&mut self, root: PathBuf, next: BTreeSet<PathBuf>) {
        let previous = if next.is_empty() {
            self.root_assets.remove(&root)
        } else {
            self.root_assets.insert(root.clone(), next.clone())
        };
        if let Some(previous) = previous {
            for asset in previous.difference(&next) {
                if let Some(roots) = self.asset_roots.get_mut(asset) {
                    roots.remove(&root);
                    if roots.is_empty() {
                        self.asset_roots.remove(asset);
                    }
                }
            }
        }
        for asset in next {
            self.asset_roots
                .entry(asset)
                .or_default()
                .insert(root.clone());
        }
    }

    fn resolve_import(
        &self,
        candidate: &Path,
        source: &Path,
        line: usize,
    ) -> Result<PathBuf, Error> {
        if let Ok(lexical) = absolute_lexical_path(candidate)
            && let Some(path) = self.overlay_alias(&lexical)
        {
            return Ok(path.clone());
        }
        match candidate.canonicalize() {
            Ok(path) => Ok(path),
            Err(error) => {
                let normalized = normalize_path(candidate).map_err(|_| {
                    file_error(
                        "E181",
                        source,
                        line,
                        format!("cannot read `{}`: {error}", candidate.display()),
                    )
                })?;
                if self.contains_overlay(&normalized) {
                    Ok(normalized)
                } else {
                    Err(file_error(
                        "E181",
                        source,
                        line,
                        format!("cannot read `{}`: {error}", candidate.display()),
                    ))
                }
            }
        }
    }

    fn invalidate_dependents(&mut self, path: &Path) -> BTreeSet<PathBuf> {
        let mut affected = BTreeSet::new();
        let mut pending = vec![path.to_owned()];
        let mut visited = HashSet::new();
        while let Some(path) = pending.pop() {
            if !visited.insert(path.clone()) {
                continue;
            }
            if self.known_roots.contains(&path) {
                self.checked_roots.remove(&path);
                self.dirty_roots.insert(path.clone());
                affected.insert(path.clone());
            }
            pending.extend(
                self.reverse_dependencies
                    .get(&path)
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
        }
        affected
    }

    fn replace_current_key(&mut self, path: PathBuf, key: FileKey) -> BTreeSet<PathBuf> {
        if self.current_files.get(&path) == Some(&key) {
            return self.dirty_roots.clone();
        }
        let mut affected = self.invalidate_dependents(&path);
        self.replace_dependencies(path.clone(), BTreeSet::new());
        if let Some(previous) = self.current_files.insert(path, key) {
            self.parsed_files.remove(&previous);
        }
        affected.extend(self.dirty_roots.iter().cloned());
        affected
    }

    fn remove_current_file(&mut self, path: &Path) -> BTreeSet<PathBuf> {
        let mut affected = self.invalidate_dependents(path);
        self.replace_dependencies(path.to_owned(), BTreeSet::new());
        if let Some(previous) = self.current_files.remove(path) {
            self.parsed_files.remove(&previous);
        }
        affected.extend(self.dirty_roots.iter().cloned());
        affected
    }

    fn prune_unreachable_files(&mut self) {
        let mut reachable = self.known_roots.iter().cloned().collect::<HashSet<_>>();
        reachable.extend(self.overlay_store.sources_by_path.keys().cloned());
        if let Some(inherited) = &self.inherited_overlay_store {
            reachable.extend(inherited.sources_by_path.keys().cloned());
        }
        let mut pending = reachable.iter().cloned().collect::<Vec<_>>();
        while let Some(path) = pending.pop() {
            for dependency in self.dependencies.get(&path).into_iter().flatten() {
                if reachable.insert(dependency.clone()) {
                    pending.push(dependency.clone());
                }
            }
        }

        self.dependencies
            .retain(|source, _| reachable.contains(source));
        self.reverse_dependencies.retain(|dependency, sources| {
            sources.retain(|source| reachable.contains(source));
            reachable.contains(dependency) && !sources.is_empty()
        });
        self.current_files
            .retain(|path, _| reachable.contains(path));
        self.parsed_files
            .retain(|key, _| reachable.contains(key.canonical_path()));
    }

    fn overlay_source(&self, path: &Path) -> Option<&Arc<str>> {
        self.overlay_store.sources_by_path.get(path).or_else(|| {
            self.inherited_overlay_store
                .as_deref()
                .and_then(|store| store.sources_by_path.get(path))
        })
    }

    fn overlay_alias(&self, lexical: &Path) -> Option<&PathBuf> {
        self.overlay_store.aliases.get(lexical).or_else(|| {
            self.inherited_overlay_store
                .as_deref()
                .and_then(|store| store.aliases.get(lexical))
        })
    }

    fn contains_overlay(&self, path: &Path) -> bool {
        self.overlay_source(path).is_some()
    }

    fn normalize_db_path(&self, path: &Path) -> std::io::Result<PathBuf> {
        let lexical = absolute_lexical_path(path)?;
        self.overlay_alias(&lexical)
            .cloned()
            .map_or_else(|| normalize_path(&lexical), Ok)
    }

    fn is_overlay_path(&self, path: &Path) -> bool {
        self.contains_overlay(path)
            || absolute_lexical_path(path)
                .ok()
                .and_then(|lexical| self.overlay_alias(&lexical))
                .is_some_and(|key| self.contains_overlay(key))
            || normalize_path(path)
                .ok()
                .is_some_and(|resolved| self.contains_overlay(&resolved))
    }

    fn roots_for_input(&self, lexical: &Path, resolved: &Path) -> BTreeSet<PathBuf> {
        let mut roots = self
            .asset_roots
            .get(lexical)
            .into_iter()
            .chain(self.asset_roots.get(resolved))
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>();
        for (root, checked) in &self.checked_roots {
            if checked
                .source_stamps
                .iter()
                .chain(&checked.asset_stamps)
                .any(|(path, stamp)| {
                    path == lexical
                        || path == resolved
                        || stamp.matches_path(lexical)
                        || stamp.matches_path(resolved)
                })
            {
                roots.insert(root.clone());
            }
        }
        roots.extend(self.invalidate_targets_for_path(lexical));
        roots.extend(self.invalidate_targets_for_path(resolved));
        roots
    }

    fn input_resolution_changed(&self, lexical: &Path) -> bool {
        let current = DiskStamp::read(lexical, None);
        self.checked_roots.values().any(|checked| {
            checked
                .source_stamps
                .iter()
                .chain(&checked.asset_stamps)
                .any(|(path, previous)| path == lexical && !current.same_resolved_input(previous))
        })
    }

    fn invalidate_targets_for_path(&self, path: &Path) -> BTreeSet<PathBuf> {
        let mut affected = BTreeSet::new();
        let mut pending = vec![path.to_owned()];
        let mut visited = HashSet::new();
        while let Some(path) = pending.pop() {
            if !visited.insert(path.clone()) {
                continue;
            }
            if self.known_roots.contains(&path) {
                affected.insert(path.clone());
            }
            pending.extend(
                self.reverse_dependencies
                    .get(&path)
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
        }
        affected
    }

    fn root_snapshot(&self, root: &Path) -> Self {
        let mut reachable = HashSet::from([root.to_owned()]);
        let mut pending = vec![root.to_owned()];
        while let Some(path) = pending.pop() {
            for dependency in self.dependencies.get(&path).into_iter().flatten() {
                if reachable.insert(dependency.clone()) {
                    pending.push(dependency.clone());
                }
            }
        }
        debug_assert!(self.inherited_overlay_store.is_none());
        let current_files = self
            .current_files
            .iter()
            .filter(|(path, _)| reachable.contains(*path))
            .map(|(path, key)| (path.clone(), key.clone()))
            .collect::<HashMap<_, _>>();
        let parsed_files = self
            .parsed_files
            .iter()
            .filter(|(key, _)| reachable.contains(key.canonical_path()))
            .map(|(key, parsed)| (key.clone(), Arc::clone(parsed)))
            .collect();
        let dependencies = self
            .dependencies
            .iter()
            .filter(|(path, _)| reachable.contains(*path))
            .map(|(path, dependencies)| (path.clone(), dependencies.clone()))
            .collect::<HashMap<_, _>>();
        let mut reverse_dependencies = HashMap::<PathBuf, BTreeSet<PathBuf>>::new();
        for (source, targets) in &dependencies {
            for target in targets {
                reverse_dependencies
                    .entry(target.clone())
                    .or_default()
                    .insert(source.clone());
            }
        }
        let checked_roots = self
            .checked_roots
            .get(root)
            .cloned()
            .map(|checked| HashMap::from([(root.to_owned(), checked)]))
            .unwrap_or_default();
        Self {
            config: self.config.clone(),
            overlay_store: Arc::default(),
            inherited_overlay_store: Some(Arc::clone(&self.overlay_store)),
            parsed_files,
            current_files,
            dependencies,
            reverse_dependencies,
            root_assets: HashMap::new(),
            asset_roots: HashMap::new(),
            known_roots: if self.known_roots.contains(root) {
                BTreeSet::from([root.to_owned()])
            } else {
                BTreeSet::new()
            },
            dirty_roots: if self.dirty_roots.contains(root) {
                BTreeSet::from([root.to_owned()])
            } else {
                BTreeSet::new()
            },
            checked_roots,
            metrics: AnalysisMetrics::default(),
            validation_policy: self.validation_policy,
        }
    }
}

fn remap_origin(mut error: Error, origins: &[(PathBuf, usize)]) -> Error {
    if let Some((origin, line)) = error
        .line
        .checked_sub(1)
        .and_then(|index| origins.get(index))
    {
        error.path = Some(origin.display().to_string());
        error.line = *line;
    }
    error
}

fn normalize_path(path: &Path) -> std::io::Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(original_error) => {
            let normalized = absolute_lexical_path(path)?;
            let mut ancestor = normalized.as_path();
            let mut missing = Vec::new();
            loop {
                match ancestor.canonicalize() {
                    Ok(mut canonical) => {
                        for component in missing.iter().rev() {
                            canonical.push(component);
                        }
                        return Ok(canonical);
                    }
                    Err(_) => {
                        let Some(name) = ancestor.file_name() else {
                            return Err(original_error);
                        };
                        missing.push(name.to_owned());
                        let Some(parent) = ancestor.parent() else {
                            return Err(original_error);
                        };
                        ancestor = parent;
                    }
                }
            }
        }
    }
}

fn absolute_lexical_path(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{
        AnalysisConfig, AnalysisDb, CompilerFeatureSet, LANGUAGE_REVISION, ValidationPolicy,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("ui-lang-analysis-db-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, source: &str) {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, source).unwrap();
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.0.join(relative)
        }

        fn remove(&self, relative: &str) {
            fs::remove_file(self.path(relative)).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn app(name: &str, import: &str, component: &str) -> String {
        format!(
            concat!(
                "app {}\n",
                "use \"{}\"\n",
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
                "  {}\n",
            ),
            name, import, component
        )
    }

    fn component(name: &str, text: &str) -> String {
        format!("component {name}()\n  text \"{text}\"\n")
    }

    fn inline_app(name: &str, text: &str) -> String {
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
                "component Part()\n",
                "  text \"{}\"\n",
                "view\n",
                "  Part\n",
            ),
            name, text
        )
    }

    #[test]
    fn file_key_includes_path_hash_revision_and_features() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "one"));
        let mut db = AnalysisDb::new(AnalysisConfig::new(
            LANGUAGE_REVISION,
            CompilerFeatureSet::new(["native-dialog"]),
        ));

        db.analyze_root(fixture.path("app.ice")).unwrap();
        let key = db
            .current_files
            .get(&fixture.path("part.ice").canonicalize().unwrap())
            .unwrap();
        assert_eq!(key.canonical_path(), fixture.path("part.ice"));
        assert_ne!(key.content_hash().bytes(), [0; 32]);
        assert_eq!(key.language_revision(), LANGUAGE_REVISION);
        assert_eq!(
            key.compiler_features().iter().collect::<Vec<_>>(),
            ["native-dialog"]
        );
    }

    #[test]
    fn retained_root_query_performs_zero_source_or_semantic_work() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "one"));
        let mut db = AnalysisDb::default();

        let first = db.query_root(fixture.path("app.ice")).unwrap();
        db.take_metrics();
        let second = db.query_root(fixture.path("app.ice")).unwrap();
        let metrics = db.take_metrics();

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(metrics.root_cache_hits, 1);
        assert_eq!(metrics.files_loaded, 0);
        assert_eq!(metrics.bytes_loaded, 0);
        assert_eq!(metrics.files_hashed, 0);
        assert_eq!(metrics.bytes_hashed, 0);
        assert_eq!(metrics.files_scanned, 0);
        assert_eq!(metrics.roots_checked, 0);
        assert_eq!(metrics.roots_reused, 0);
        assert_eq!(metrics.symbols_indexed, 0);
        assert_eq!(metrics.source_stamps_checked, 0);
    }

    #[test]
    fn retained_query_detects_an_import_change_without_a_watcher_notification() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "one"));
        let mut db = AnalysisDb::default();
        let root = fixture.path("app.ice");

        db.query_root(&root).unwrap();
        db.take_metrics();
        fixture.write("part.ice", &component("Renamed", "two"));
        let checked = db
            .checked_roots
            .get_mut(&root.canonicalize().unwrap())
            .unwrap();
        checked.metadata_validated_at = Instant::now() - Duration::from_secs(1);
        checked.content_validated_at = Instant::now() - Duration::from_secs(1);

        let error = db.query_root(&root).unwrap_err();
        let metrics = db.take_metrics();
        assert!(
            error.message.contains("unknown component `Part`"),
            "{error:?}"
        );
        assert!(metrics.source_stamps_checked >= 2, "{metrics:?}");
        assert!(metrics.files_loaded >= 2, "{metrics:?}");
        assert!(metrics.roots_checked >= 1, "{metrics:?}");
    }

    #[test]
    fn content_epoch_detects_same_length_source_with_restored_mtime() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "same"));
        let root = fixture.path("app.ice");
        let part = fixture.path("part.ice");
        let mut db = AnalysisDb::default();
        db.set_validation_policy(ValidationPolicy::new(
            Duration::from_millis(5),
            Duration::from_millis(5),
        ));
        db.query_root(&root).unwrap();
        let metadata = fs::metadata(&part).unwrap();

        fixture.write("part.ice", &component("Tile", "same"));
        std::fs::OpenOptions::new()
            .write(true)
            .open(&part)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(metadata.accessed().unwrap())
                    .set_modified(metadata.modified().unwrap()),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(10));

        let error = db.query_root(root).unwrap_err();
        assert!(
            error.message.contains("unknown component `Part`"),
            "{error:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn content_epoch_detects_timestamp_preserving_atomic_replacement() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "same"));
        let root = fixture.path("app.ice");
        let part = fixture.path("part.ice");
        let replacement = fixture.path("replacement.ice");
        let mut db = AnalysisDb::default();
        db.set_validation_policy(ValidationPolicy::always());
        db.query_root(&root).unwrap();
        let metadata = fs::metadata(&part).unwrap();
        fixture.write("replacement.ice", &component("Tile", "same"));
        std::fs::OpenOptions::new()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(metadata.accessed().unwrap())
                    .set_modified(metadata.modified().unwrap()),
            )
            .unwrap();

        fs::rename(replacement, part).unwrap();

        assert!(
            db.query_root(root)
                .unwrap_err()
                .message
                .contains("unknown component `Part`")
        );
    }

    #[cfg(unix)]
    #[test]
    fn import_symlink_retarget_changes_the_retained_source_closure() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("a.ice", &component("Part", "same"));
        fixture.write("b.ice", &component("Part", "same"));
        let a = fixture.path("a.ice");
        let b = fixture.path("b.ice");
        let link = fixture.path("part.ice");
        let metadata = fs::metadata(&a).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&b)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(metadata.accessed().unwrap())
                    .set_modified(metadata.modified().unwrap()),
            )
            .unwrap();
        symlink("a.ice", &link).unwrap();
        let mut db = AnalysisDb::default();
        db.set_validation_policy(ValidationPolicy::always());
        let first = db.query_root(fixture.path("app.ice")).unwrap();

        fs::remove_file(&link).unwrap();
        symlink("b.ice", &link).unwrap();
        let second = db.query_root(fixture.path("app.ice")).unwrap();

        assert!(!Arc::ptr_eq(&first, &second));
        assert!(second.dependencies.contains(&b));
        assert!(!second.dependencies.contains(&a));
    }

    #[cfg(unix)]
    #[test]
    fn asset_symlink_retarget_rechecks_the_resolved_target() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  window\n    icon-rgba \"icon.rgba\" 1 1\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        fixture.write("a.rgba", "RGBA");
        fixture.write("b.rgba", "RGBAFAIL");
        let link = fixture.path("icon.rgba");
        symlink("a.rgba", &link).unwrap();
        let mut db = AnalysisDb::default();
        db.set_validation_policy(ValidationPolicy::always());
        db.query_root(fixture.path("app.ice")).unwrap();

        fs::remove_file(&link).unwrap();
        symlink("b.rgba", &link).unwrap();

        assert_eq!(
            db.query_root(fixture.path("app.ice")).unwrap_err().code,
            "E193"
        );
    }

    #[cfg(unix)]
    #[test]
    fn root_symlink_overlay_closes_against_its_opened_target() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        let a = fixture.path("a.ice");
        let link = fixture.path("app.ice");
        fixture.write("a.ice", &inline_app("A", "disk a"));
        fixture.write("b.ice", &inline_app("B", "disk b"));
        symlink("a.ice", &link).unwrap();
        let mut db = AnalysisDb::default();
        db.set_overlay(&link, inline_app("Unsaved", "overlay"))
            .unwrap();
        db.query_root(&link).unwrap();

        fs::remove_file(&link).unwrap();
        symlink("b.ice", &link).unwrap();
        db.remove_overlay(&link).unwrap();
        let analysis = db.query_root(&a).unwrap();

        assert_eq!(analysis.document.source_document().app, "A");
    }

    #[cfg(unix)]
    #[test]
    fn closing_the_latest_alias_overlay_restores_the_remaining_open_source() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Real"));
        fixture.write("part.ice", &component("Disk", "disk"));
        let root = fixture.path("app.ice");
        let real = fixture.path("part.ice");
        let link = fixture.path("part-link.ice");
        symlink("part.ice", &link).unwrap();
        let mut db = AnalysisDb::default();

        db.set_overlay(&real, component("Real", "real buffer"))
            .unwrap();
        db.set_overlay(&link, component("Link", "link buffer"))
            .unwrap();
        assert!(db.query_root(&root).is_err());

        let invalidation = db.remove_overlay(&link).unwrap();
        let analysis = db.query_root(&root).unwrap();

        assert!(invalidation.changed);
        assert_eq!(analysis.document.source_document().app, "Demo");
        assert_eq!(
            db.overlay_store
                .sources_by_path
                .get(&real.canonicalize().unwrap())
                .map(AsRef::as_ref),
            Some(component("Real", "real buffer").as_str())
        );
    }

    #[test]
    fn metadata_only_source_change_does_not_recheck_semantics() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        let part_source = component("Part", "same");
        fixture.write("part.ice", &part_source);
        let root = fixture.path("app.ice");
        let mut db = AnalysisDb::default();
        db.set_validation_policy(ValidationPolicy::new(
            Duration::ZERO,
            Duration::from_secs(60),
        ));

        let first = db.query_root(&root).unwrap();
        db.take_metrics();
        std::thread::sleep(Duration::from_millis(2));
        fixture.write("part.ice", &part_source);
        let second = db.query_root(&root).unwrap();
        let metrics = db.take_metrics();

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(metrics.roots_checked, 0, "{metrics:?}");
        assert_eq!(metrics.files_loaded, 1, "{metrics:?}");
        assert_eq!(metrics.files_hashed, 1, "{metrics:?}");
    }

    #[test]
    fn metadata_only_watcher_refresh_keeps_the_retained_root() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        let part_source = component("Part", "same");
        fixture.write("part.ice", &part_source);
        let root = fixture.path("app.ice");
        let part = fixture.path("part.ice");
        let mut db = AnalysisDb::default();
        let first = db.query_root(&root).unwrap();
        db.take_metrics();
        fixture.write("part.ice", &part_source);

        let invalidation = db.refresh_input(&part).unwrap();
        let second = db.query_root(&root).unwrap();
        let metrics = db.take_metrics();

        assert!(!invalidation.changed);
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(metrics.roots_checked, 0, "{metrics:?}");
    }

    #[test]
    fn retained_query_detects_asset_deletion_and_recovers_without_a_watcher() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  font \"Brand.ttf\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        fixture.write("Brand.ttf", "font bytes");
        let root = fixture.path("app.ice");
        let font = fixture.path("Brand.ttf");
        let mut db = AnalysisDb::default();

        db.query_root(&root).unwrap();
        db.take_metrics();
        fs::remove_file(&font).unwrap();

        let error = db.query_root_fresh(&root).unwrap_err();
        assert_eq!(error.code, "E192");
        assert_eq!(db.metrics().asset_stamps_checked, 1);

        fixture.write("Brand.ttf", "replacement font bytes");
        db.query_root(&root).unwrap();
    }

    #[test]
    fn media_asset_edit_invalidates_the_root_that_embeds_it() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  image \"photo.ppm\"\n",
        );
        fixture.write("photo.ppm", "P6 1 1 255 pixels");
        let root = fixture.path("app.ice");
        let asset = fixture.path("photo.ppm");
        let mut db = AnalysisDb::default();

        let first = db.query_root(&root).unwrap();
        assert_eq!(first.asset_dependencies, std::slice::from_ref(&asset));

        fixture.write("photo.ppm", "P6 1 1 255 repainted");
        let invalidation = db.refresh_input(&asset).unwrap();
        let second = db.query_root(&root).unwrap();

        assert!(invalidation.changed);
        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn retained_query_detects_icon_content_change_without_a_watcher() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  window\n    icon-rgba \"app.rgba\" 1 1\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        fixture.write("app.rgba", "RGBA");
        let root = fixture.path("app.ice");
        let mut db = AnalysisDb::default();

        db.query_root(&root).unwrap();
        fixture.write("app.rgba", "RGBAFAIL");

        let error = db.query_root_fresh(root).unwrap_err();
        assert_eq!(error.code, "E193");
    }

    #[test]
    fn content_epoch_detects_same_length_asset_with_restored_mtime() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  window\n    icon-rgba \"app.rgba\" 1 1\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        fixture.write("app.rgba", "RGBA");
        let root = fixture.path("app.ice");
        let asset = fixture.path("app.rgba");
        let mut db = AnalysisDb::default();
        db.set_validation_policy(ValidationPolicy::always());
        let first = db.query_root(&root).unwrap();
        let metadata = fs::metadata(&asset).unwrap();

        fixture.write("app.rgba", "DIFF");
        std::fs::OpenOptions::new()
            .write(true)
            .open(&asset)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_accessed(metadata.accessed().unwrap())
                    .set_modified(metadata.modified().unwrap()),
            )
            .unwrap();
        let second = db.query_root(root).unwrap();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn speculative_analysis_preserves_the_retained_root() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "one"));
        let mut db = AnalysisDb::default();
        let retained = db.query_root(fixture.path("app.ice")).unwrap();
        db.take_metrics();

        let candidate = app("Candidate", "part.ice", "Part");
        assert!(
            db.analyze_overlay_candidate(fixture.path("app.ice"), candidate)
                .is_ok()
        );
        let current = db.query_root(fixture.path("app.ice")).unwrap();
        assert!(Arc::ptr_eq(&retained, &current));

        let metrics = db.take_metrics();
        assert_eq!(metrics.speculative_runs, 1);
        assert_eq!(metrics.roots_checked, 1);
        assert_eq!(metrics.root_cache_hits, 1);
    }

    #[test]
    fn speculative_candidate_can_import_an_unsaved_overlay_outside_the_old_closure() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &inline_app("Retained", "disk"));
        let root = fixture.path("app.ice");
        let pending = fixture.path("pending.ice");
        let mut db = AnalysisDb::default();
        let retained = db.query_root(&root).unwrap();
        db.set_overlay(&pending, component("Pending", "unsaved"))
            .unwrap();

        let candidate = app("Candidate", "pending.ice", "Pending");
        let analyzed = db.analyze_overlay_candidate(&root, candidate).unwrap();
        let current = db.query_root(&root).unwrap();

        assert_eq!(analyzed.document.source_document().app, "Candidate");
        assert!(analyzed.dependencies.contains(&pending));
        assert!(Arc::ptr_eq(&retained, &current));
    }

    #[test]
    fn default_validation_epoch_bounds_large_disk_closure_probes() {
        const IMPORTS: usize = 1_000;
        const REQUESTS: usize = 10;

        let fixture = Fixture::new();
        let mut source = String::from("app Scale\n");
        for index in 0..IMPORTS {
            let name = format!("part-{index}.ice");
            fixture.write(&name, &format!("// fragment {index}\n"));
            source.push_str(&format!("use \"{name}\"\n"));
        }
        source.push_str(
            "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"ready\"\n",
        );
        fixture.write("app.ice", &source);
        let root = fixture.path("app.ice");
        let mut db = AnalysisDb::default();
        assert_eq!(
            db.validation_policy(),
            ValidationPolicy::new(Duration::from_millis(750), Duration::from_millis(750))
        );
        db.query_root(&root).unwrap();
        db.take_metrics();

        let started = Instant::now();
        for _ in 0..REQUESTS {
            db.query_root(&root).unwrap();
        }
        let elapsed = started.elapsed();
        let metrics = db.take_metrics();

        assert_eq!(metrics.source_stamps_checked, 0, "{metrics:?}");
        assert_eq!(metrics.files_loaded, 0, "{metrics:?}");
        assert_eq!(metrics.files_hashed, 0, "{metrics:?}");
        assert_eq!(metrics.root_cache_hits, REQUESTS, "{metrics:?}");
        assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
    }

    #[test]
    fn independent_root_reuses_checked_result_after_other_graph_changes() {
        let fixture = Fixture::new();
        fixture.write("a.ice", &app("A", "a_part.ice", "AView"));
        fixture.write("a_part.ice", &component("AView", "one"));
        fixture.write("b.ice", &app("B", "b_part.ice", "BView"));
        fixture.write("b_part.ice", &component("BView", "two"));
        let mut db = AnalysisDb::default();
        db.analyze_root(fixture.path("a.ice")).unwrap();
        db.analyze_root(fixture.path("b.ice")).unwrap();
        db.take_metrics();

        db.set_overlay(fixture.path("a_part.ice"), component("AView", "changed"))
            .unwrap();
        db.analyze_root(fixture.path("a.ice")).unwrap();
        db.analyze_root(fixture.path("b.ice")).unwrap();

        let metrics = db.take_metrics();
        assert_eq!(metrics.files_scanned, 1);
        assert_eq!(metrics.roots_checked, 1);
        assert_eq!(metrics.roots_reused, 1);
    }

    #[test]
    fn validated_compile_root_reloads_a_shared_import_between_roots() {
        let fixture = Fixture::new();
        let a_source = app("A", "shared.ice", "Shared");
        let b_source = app("B", "shared.ice", "Shared");
        let a = fixture.path("a.ice");
        let b = fixture.path("b.ice");
        fixture.write("a.ice", &a_source);
        fixture.write("b.ice", &b_source);
        fixture.write("shared.ice", &component("Shared", "one"));
        let mut db = AnalysisDb::default();
        db.compile_root_with_validated_source(&a, a_source.as_bytes())
            .unwrap();
        db.take_metrics();
        let changed = component("Renamed", "two");
        fixture.write("shared.ice", &changed);

        let error = db
            .compile_root_with_validated_source(&b, b_source.as_bytes())
            .unwrap_err();
        let metrics = db.take_metrics();

        assert!(
            error.message.contains("unknown component `Shared`"),
            "{error:?}"
        );
        assert_eq!(metrics.files_loaded, 2, "{metrics:?}");
        assert_eq!(
            metrics.bytes_loaded,
            b_source.len() + changed.len(),
            "{metrics:?}"
        );
    }

    #[test]
    fn validated_leaf_source_reuses_every_other_file_in_the_retained_closure() {
        const IMPORTS: usize = 128;

        let fixture = Fixture::new();
        let mut source = String::from("app Large\n");
        for index in 0..IMPORTS {
            let name = format!("part-{index:03}.ice");
            fixture.write(&name, &format!("// fragment {index}\n"));
            source.push_str(&format!("use \"{name}\"\n"));
        }
        source.push_str(
            "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"ready\"\n",
        );
        fixture.write("large.ice", &source);
        fixture.write("other.ice", &inline_app("Other", "untouched"));
        let root = fixture.path("large.ice");
        let other = fixture.path("other.ice");
        let changed = fixture.path("part-064.ice");
        let changed_source = b"// changed fragment\n".to_vec();
        let mut db = AnalysisDb::default();
        db.query_root(&root).unwrap();
        db.query_root(&other).unwrap();
        db.take_metrics();
        fs::write(&changed, &changed_source).unwrap();

        db.analyze_root_with_validated_sources(
            &root,
            [super::ValidatedSource::new(
                changed.clone(),
                changed_source.clone(),
            )],
        )
        .unwrap();
        db.query_root(&other).unwrap();
        let metrics = db.take_metrics();

        assert_eq!(metrics.files_loaded, 1, "{metrics:?}");
        assert_eq!(metrics.bytes_loaded, changed_source.len(), "{metrics:?}");
        assert_eq!(metrics.files_hashed, 1, "{metrics:?}");
        assert_eq!(metrics.bytes_hashed, changed_source.len(), "{metrics:?}");
        assert_eq!(metrics.files_scanned, 1, "{metrics:?}");
        assert_eq!(metrics.roots_checked, 1, "{metrics:?}");
        assert_eq!(metrics.root_cache_hits, 1, "{metrics:?}");
    }

    #[test]
    fn shared_fragment_invalidates_every_reverse_dependent_root() {
        let fixture = Fixture::new();
        fixture.write("a.ice", &app("A", "shared.ice", "Shared"));
        fixture.write("b.ice", &app("B", "shared.ice", "Shared"));
        fixture.write("shared.ice", &component("Shared", "one"));
        let mut db = AnalysisDb::default();
        db.analyze_root(fixture.path("a.ice")).unwrap();
        db.analyze_root(fixture.path("b.ice")).unwrap();
        db.take_metrics();

        let invalidation = db
            .set_overlay(fixture.path("shared.ice"), component("Shared", "two"))
            .unwrap();

        assert_eq!(invalidation.affected_roots.len(), 2);
        assert!(db.reverse_dependencies(fixture.path("shared.ice")).len() == 2);
        db.analyze_root(fixture.path("a.ice")).unwrap();
        db.analyze_root(fixture.path("b.ice")).unwrap();
        assert_eq!(db.take_metrics().roots_checked, 2);
    }

    #[test]
    fn forgetting_a_root_prunes_only_its_unreachable_file_state() {
        let fixture = Fixture::new();
        fixture.write("a.ice", &app("A", "a_part.ice", "AView"));
        fixture.write("a_part.ice", &component("AView", "one"));
        fixture.write("b.ice", &app("B", "b_part.ice", "BView"));
        fixture.write("b_part.ice", &component("BView", "two"));
        let a = fixture.path("a.ice").canonicalize().unwrap();
        let a_part = fixture.path("a_part.ice").canonicalize().unwrap();
        let b = fixture.path("b.ice").canonicalize().unwrap();
        let b_part = fixture.path("b_part.ice").canonicalize().unwrap();
        let mut db = AnalysisDb::default();
        db.analyze_root(&a).unwrap();
        db.analyze_root(&b).unwrap();

        assert!(db.forget_root(&a));
        assert!(!db.current_files.contains_key(&a));
        assert!(!db.current_files.contains_key(&a_part));
        assert!(db.current_files.contains_key(&b));
        assert!(db.current_files.contains_key(&b_part));
        assert_eq!(db.checked_root_count(), 1);
        assert_eq!(db.parsed_file_count(), 2);
    }

    #[test]
    fn closing_overlay_falls_back_to_disk_and_unchanged_content_reuses() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "disk"));
        let mut db = AnalysisDb::default();
        db.analyze_root(fixture.path("app.ice")).unwrap();
        db.take_metrics();

        let unchanged = db
            .set_overlay(fixture.path("part.ice"), component("Part", "disk"))
            .unwrap();
        assert!(!unchanged.changed);
        db.analyze_root(fixture.path("app.ice")).unwrap();
        assert_eq!(db.take_metrics().roots_checked, 0);

        db.set_overlay(fixture.path("part.ice"), component("Part", "overlay"))
            .unwrap();
        db.analyze_root(fixture.path("app.ice")).unwrap();
        db.take_metrics();
        let closed = db.remove_overlay(fixture.path("part.ice")).unwrap();
        assert!(closed.changed);
        db.analyze_root(fixture.path("app.ice")).unwrap();
        assert_eq!(db.take_metrics().roots_checked, 1);
    }

    #[test]
    fn disk_refresh_invalidates_only_when_content_changes() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "one"));
        let mut db = AnalysisDb::default();
        db.analyze_root(fixture.path("app.ice")).unwrap();
        db.take_metrics();

        assert!(!db.refresh_file(fixture.path("part.ice")).unwrap().changed);
        fixture.write("part.ice", &component("Part", "two"));
        let changed = db.refresh_file(fixture.path("part.ice")).unwrap();
        assert!(changed.changed);
        assert_eq!(changed.affected_roots.len(), 1);
        db.analyze_root(fixture.path("app.ice")).unwrap();
        assert_eq!(db.take_metrics().roots_checked, 1);
    }

    #[test]
    fn dependency_graph_tracks_transitive_imports() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "middle.ice", "Part"));
        fixture.write("middle.ice", "use \"part.ice\"\n");
        fixture.write("part.ice", &component("Part", "one"));
        let mut db = AnalysisDb::default();
        db.analyze_root(fixture.path("app.ice")).unwrap();

        assert_eq!(db.dependencies(fixture.path("app.ice")).len(), 1);
        assert_eq!(db.dependencies(fixture.path("middle.ice")).len(), 1);
        assert_eq!(db.reverse_dependencies(fixture.path("part.ice")).len(), 1);
    }

    #[test]
    fn missing_import_records_a_recovery_edge_before_the_file_exists() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "missing.ice", "Part"));
        let root = fixture.path("app.ice").canonicalize().unwrap();
        let missing = fixture.path("missing.ice");
        let mut db = AnalysisDb::default();

        assert!(db.analyze_root(&root).is_err());
        assert!(db.dirty_roots().contains(&root));
        assert_eq!(
            db.reverse_dependencies(&missing),
            BTreeSet::from([root.clone()])
        );

        let invalidation = db
            .set_overlay(&missing, component("Part", "created"))
            .unwrap();
        assert!(invalidation.affected_roots.contains(&root));
        db.analyze_root(&root).unwrap();
        assert!(!db.dirty_roots().contains(&root));
    }

    #[test]
    fn malformed_import_keeps_its_parent_root_dirty_until_fixed() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "middle.ice", "Part"));
        fixture.write("middle.ice", "use part.ice\n");
        fixture.write("part.ice", &component("Part", "fixed"));
        let root = fixture.path("app.ice").canonicalize().unwrap();
        let middle = fixture.path("middle.ice").canonicalize().unwrap();
        let mut db = AnalysisDb::default();

        assert!(db.analyze_root(&root).is_err());
        assert_eq!(
            db.reverse_dependencies(&middle),
            BTreeSet::from([root.clone()])
        );
        let invalidation = db.set_overlay(&middle, "use \"part.ice\"\n").unwrap();
        assert!(invalidation.affected_roots.contains(&root));
        db.analyze_root(&root).unwrap();
    }

    #[test]
    fn semantic_check_failure_keeps_the_root_dirty_until_its_fragment_is_fixed() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Expected"));
        fixture.write("part.ice", &component("Wrong", "broken"));
        let root = fixture.path("app.ice").canonicalize().unwrap();
        let part = fixture.path("part.ice").canonicalize().unwrap();
        let mut db = AnalysisDb::default();

        assert!(db.analyze_root(&root).is_err());
        assert!(db.dirty_roots().contains(&root));
        let invalidation = db
            .set_overlay(&part, component("Expected", "fixed"))
            .unwrap();
        assert!(invalidation.affected_roots.contains(&root));
        db.analyze_root(&root).unwrap();
        assert!(!db.dirty_roots().contains(&root));
    }

    #[test]
    fn import_add_rename_and_remove_replace_reverse_edges_immediately() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &inline_app("Demo", "inline"));
        fixture.write("a.ice", &component("Part", "a"));
        fixture.write("b.ice", &component("Part", "b"));
        let root = fixture.path("app.ice").canonicalize().unwrap();
        let a = fixture.path("a.ice").canonicalize().unwrap();
        let b = fixture.path("b.ice").canonicalize().unwrap();
        let mut db = AnalysisDb::default();
        db.analyze_root(&root).unwrap();

        db.set_overlay(&root, app("Demo", "a.ice", "Part")).unwrap();
        db.analyze_root(&root).unwrap();
        assert_eq!(db.reverse_dependencies(&a), BTreeSet::from([root.clone()]));

        db.set_overlay(&root, app("Demo", "b.ice", "Part")).unwrap();
        assert!(db.reverse_dependencies(&a).is_empty());
        db.analyze_root(&root).unwrap();
        assert_eq!(db.reverse_dependencies(&b), BTreeSet::from([root.clone()]));

        db.set_overlay(&root, inline_app("Demo", "inline again"))
            .unwrap();
        assert!(db.reverse_dependencies(&b).is_empty());
        db.analyze_root(&root).unwrap();
    }

    #[test]
    fn deleted_file_invalidates_cached_roots_and_recovers_after_recreation() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        fixture.write("part.ice", &component("Part", "one"));
        let root = fixture.path("app.ice").canonicalize().unwrap();
        let part = fixture.path("part.ice").canonicalize().unwrap();
        let mut db = AnalysisDb::default();
        db.analyze_root(&root).unwrap();

        fixture.remove("part.ice");
        assert!(db.refresh_file(&part).is_err());
        assert!(db.needs_analysis(&root));
        assert!(!db.current_files.contains_key(&part));
        assert!(db.analyze_root(&root).is_err());

        fixture.write("part.ice", &component("Part", "recreated"));
        let invalidation = db.refresh_file(&part).unwrap();
        assert!(invalidation.affected_roots.contains(&root));
        db.analyze_root(&root).unwrap();
    }

    #[test]
    fn import_cycle_records_edges_and_recovers_when_the_cycle_is_removed() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "a.ice", "Part"));
        fixture.write("a.ice", "use \"b.ice\"\n");
        fixture.write("b.ice", "use \"a.ice\"\n");
        let root = fixture.path("app.ice").canonicalize().unwrap();
        let b = fixture.path("b.ice").canonicalize().unwrap();
        let mut db = AnalysisDb::default();

        assert!(db.analyze_root(&root).is_err());
        let invalidation = db
            .set_overlay(&b, component("Part", "cycle removed"))
            .unwrap();
        assert!(invalidation.affected_roots.contains(&root));
        db.analyze_root(&root).unwrap();
    }

    #[test]
    fn missing_disk_after_overlay_close_marks_the_root_dirty() {
        let fixture = Fixture::new();
        fixture.write("app.ice", &app("Demo", "part.ice", "Part"));
        let root = fixture.path("app.ice").canonicalize().unwrap();
        let part = fixture.path("part.ice");
        let mut db = AnalysisDb::default();
        db.set_overlay(&part, component("Part", "overlay")).unwrap();
        db.analyze_root(&root).unwrap();

        assert!(db.remove_overlay(&part).is_err());
        assert!(db.needs_analysis(&root));
        assert!(db.analyze_root(&root).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn missing_overlay_under_a_symlink_uses_the_imports_canonical_parent() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        fixture.write("real/app.ice", &app("Demo", "pending.ice", "Part"));
        symlink(fixture.path("real"), fixture.path("link")).unwrap();
        let root = fixture.path("real/app.ice");
        let overlay = fixture.path("link/pending.ice");
        let canonical_overlay = fixture.path("real/pending.ice");
        let mut db = AnalysisDb::default();

        db.set_overlay(&overlay, component("Part", "unsaved"))
            .unwrap();
        db.analyze_root(&root).unwrap();
        assert_eq!(
            db.reverse_dependencies(&canonical_overlay),
            BTreeSet::from([root.canonicalize().unwrap()])
        );
    }
}
