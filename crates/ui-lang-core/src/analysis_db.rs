use crate::source::{
    LoadedSource, Origin, analyze_loaded, asset_dependencies, check_assets, file_error, parse_use,
};
use crate::{Error, FileAnalysis, FileCompilation, codegen, lower, source_is_app};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The Ice language contract used by the current compiler.
pub const LANGUAGE_REVISION: &str = "2.0";

/// A stable SHA-256 digest of one source buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn of(source: &str) -> Self {
        Self(Sha256::digest(source.as_bytes()).into())
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
    pub roots_checked: usize,
    pub roots_reused: usize,
    pub symbols_indexed: usize,
    pub codegen_roots: usize,
    pub elapsed: AnalysisTimings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnalysisInvalidation {
    pub changed: bool,
    pub affected_roots: BTreeSet<PathBuf>,
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
    analysis: FileAnalysis,
}

#[derive(Debug)]
struct LoadedGraph {
    loaded: LoadedSource,
    fingerprint: RootFingerprint,
}

/// Process-local incremental storage for Ice source graphs.
///
/// The owner controls its lifetime. The DB never writes a cache to disk and it
/// does not use global state, so LSP/dev processes can retain it while one-shot
/// callers can create it for a single command.
#[derive(Debug, Default)]
pub struct AnalysisDb {
    config: AnalysisConfig,
    overlays: HashMap<PathBuf, Arc<str>>,
    parsed_files: HashMap<FileKey, Arc<ParsedFile>>,
    current_files: HashMap<PathBuf, FileKey>,
    dependencies: HashMap<PathBuf, BTreeSet<PathBuf>>,
    reverse_dependencies: HashMap<PathBuf, BTreeSet<PathBuf>>,
    known_roots: BTreeSet<PathBuf>,
    dirty_roots: BTreeSet<PathBuf>,
    checked_roots: HashMap<PathBuf, CheckedRoot>,
    metrics: AnalysisMetrics,
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
        let Ok(path) = normalize_path(path.as_ref()) else {
            return true;
        };
        !self.known_roots.contains(&path)
            || self.dirty_roots.contains(&path)
            || !self.checked_roots.contains_key(&path)
    }

    /// Stop retaining a checked root while keeping reusable parsed imports.
    pub fn forget_root(&mut self, path: impl AsRef<Path>) -> bool {
        let Ok(path) = normalize_path(path.as_ref()) else {
            return false;
        };
        self.known_roots.remove(&path);
        self.dirty_roots.remove(&path);
        let removed = self.checked_roots.remove(&path).is_some();
        self.prune_unreachable_files();
        removed
    }

    pub fn dependencies(&self, path: impl AsRef<Path>) -> BTreeSet<PathBuf> {
        normalize_path(path.as_ref())
            .ok()
            .and_then(|path| self.dependencies.get(&path).cloned())
            .unwrap_or_default()
    }

    pub fn reverse_dependencies(&self, path: impl AsRef<Path>) -> BTreeSet<PathBuf> {
        normalize_path(path.as_ref())
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
        let path = normalize_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve overlay path: {error}"),
            )
        })?;
        let source: Arc<str> = Arc::from(source.into());
        let key = self.file_key(path.clone(), &source);
        let changed = self.current_files.get(&path) != Some(&key);
        self.overlays.insert(path.clone(), source);
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
        let path = normalize_path(path.as_ref()).map_err(|error| {
            file_error(
                "E181",
                path.as_ref(),
                1,
                format!("cannot resolve overlay path: {error}"),
            )
        })?;
        let Some(previous) = self.overlays.remove(&path) else {
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
        let path = normalize_path(path.as_ref()).map_err(|error| {
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
        let requested = path.as_ref().to_owned();
        let requested_path = absolute_lexical_path(&requested).map_err(|error| {
            file_error(
                "E181",
                &requested,
                1,
                format!("cannot resolve source path: {error}"),
            )
        })?;
        let root = normalize_path(&requested).map_err(|error| {
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
        let graph = self.load_graph(&root);
        self.metrics.elapsed.load += started.elapsed();
        let graph = graph?;
        if let Some(cached) = self.checked_roots.get(&root)
            && cached.fingerprint == graph.fingerprint
        {
            check_assets(&cached.analysis.document, &graph.loaded)
                .map_err(|error| crate::source::remap_error(error, &graph.loaded))?;
            self.metrics.roots_reused += 1;
            self.dirty_roots.remove(&root);
            let mut analysis = cached.analysis.clone();
            analysis.dependencies.push(requested_path);
            analysis.dependencies.sort();
            analysis.dependencies.dedup();
            return Ok(analysis);
        }

        let started = Instant::now();
        self.metrics.roots_checked += 1;
        let document = analyze_loaded(&graph.loaded);
        self.metrics.elapsed.check += started.elapsed();
        let document = document?;
        let asset_dependencies = asset_dependencies(&document, &graph.loaded);
        self.metrics.symbols_indexed += document.symbols().len();
        let mut dependencies = graph.loaded.dependencies.clone();
        dependencies.push(requested_path);
        dependencies.sort();
        dependencies.dedup();
        let analysis = FileAnalysis {
            document,
            dependencies,
            asset_dependencies,
        };
        self.checked_roots.insert(
            root.clone(),
            CheckedRoot {
                fingerprint: graph.fingerprint,
                analysis: analysis.clone(),
            },
        );
        self.dirty_roots.remove(&root);
        Ok(analysis)
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
        let started = Instant::now();
        let source_origins = analysis.document.source_origins().to_vec();
        let program = lower::lower(analysis.document)
            .map_err(|error| remap_origin(error, &source_origins))?;
        let mut rust = codegen::generate(&program, &path.display().to_string()).map_err(
            |mut error| {
                if let Some((origin, line)) = program.source_origin(error.line) {
                    error.path = Some(origin.display().to_string());
                    error.line = line;
                }
                error
            },
        )?;
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

    fn read_source(&mut self, path: &Path) -> Result<Arc<str>, Error> {
        let source = if let Some(source) = self.overlays.get(path) {
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

    fn parsed_file(&mut self, path: &Path) -> Result<(FileKey, Arc<ParsedFile>), Error> {
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
        self.parsed_files.insert(key.clone(), Arc::clone(&parsed));
        self.metrics.files_scanned += 1;
        Ok((key, parsed))
    }

    fn load_graph(&mut self, root: &Path) -> Result<LoadedGraph, Error> {
        let (_, parsed) = self.parsed_file(root)?;
        if !source_is_app(&parsed.source) {
            return Err(file_error(
                "E183",
                root,
                1,
                "a root must declare `app Name` or `daemon Name`; import this fragment from an app or daemon instead",
            ));
        }
        let mut graph = LoadedGraph {
            loaded: LoadedSource {
                source: String::new(),
                origins: Vec::new(),
                dependencies: Vec::new(),
            },
            fingerprint: RootFingerprint(Vec::new()),
        };
        let mut included = HashSet::new();
        let mut stack = Vec::new();
        self.load_into(root, (root, 1), None, &mut graph, &mut included, &mut stack)?;
        Ok(graph)
    }

    fn load_into(
        &mut self,
        path: &Path,
        imported_from: (&Path, usize),
        namespace: Option<String>,
        graph: &mut LoadedGraph,
        included: &mut HashSet<(PathBuf, Option<String>)>,
        stack: &mut Vec<PathBuf>,
    ) -> Result<(), Error> {
        if let Some(start) = stack.iter().position(|entry| entry == path) {
            let mut cycle = stack[start..]
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
        if !included.insert((path.to_owned(), namespace.clone())) {
            return Ok(());
        }
        stack.push(path.to_owned());
        if !graph.loaded.dependencies.contains(&path.to_owned()) {
            graph.loaded.dependencies.push(path.to_owned());
        }
        let (key, parsed) = self.parsed_file(path)?;
        graph.fingerprint.0.push((key, namespace.clone()));
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
                && !graph.loaded.dependencies.contains(&candidate_path)
            {
                graph.loaded.dependencies.push(candidate_path);
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
                    self.load_into(
                        &target,
                        (path, line),
                        child_namespace,
                        graph,
                        included,
                        stack,
                    )?;
                }
                ParsedLine::Source { range, line } => {
                    graph.loaded.source.push_str(&parsed.source[range.clone()]);
                    graph.loaded.source.push('\n');
                    graph.loaded.origins.push(Origin {
                        path: path.to_owned(),
                        line: *line,
                        namespace: namespace.clone(),
                    });
                }
            }
        }
        stack.pop();
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

    fn resolve_import(
        &self,
        candidate: &Path,
        source: &Path,
        line: usize,
    ) -> Result<PathBuf, Error> {
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
                if self.overlays.contains_key(&normalized) {
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
        let mut reachable = self
            .known_roots
            .iter()
            .chain(self.overlays.keys())
            .cloned()
            .collect::<HashSet<_>>();
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
    use super::{AnalysisConfig, AnalysisDb, CompilerFeatureSet, LANGUAGE_REVISION};
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

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
