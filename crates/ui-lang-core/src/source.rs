use crate::{CheckedDocument, Document, Error, Span, check, codegen, parser};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub struct FileCompilation {
    pub rust: String,
    pub dependencies: Vec<PathBuf>,
    pub asset_dependencies: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct FileAnalysis {
    pub document: CheckedDocument,
    pub dependencies: Vec<PathBuf>,
    pub asset_dependencies: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
struct Origin {
    path: PathBuf,
    line: usize,
    namespace: Option<String>,
}

#[derive(Debug)]
struct LoadedSource {
    source: String,
    origins: Vec<Origin>,
    dependencies: Vec<PathBuf>,
}

pub fn source_is_app(source: &str) -> bool {
    source.lines().any(|line| {
        line.len() == line.trim_start().len()
            && (line.starts_with("app ") || line.starts_with("daemon "))
    })
}

pub fn analyze_file(path: impl AsRef<Path>) -> Result<CheckedDocument, Error> {
    Ok(analyze_file_graph(path)?.document)
}

/// Analyzes one root and returns every source and asset input used by it.
pub fn analyze_file_graph(path: impl AsRef<Path>) -> Result<FileAnalysis, Error> {
    let loaded = load(path.as_ref())?;
    let document = analyze_loaded(&loaded)?;
    let mut dependencies = discover_file_dependencies(path.as_ref())?;
    dependencies.extend(loaded.dependencies.iter().cloned());
    dependencies.sort();
    dependencies.dedup();
    let asset_dependencies = asset_dependencies(&document, &loaded);
    Ok(FileAnalysis {
        document,
        dependencies,
        asset_dependencies,
    })
}

/// Returns readable and missing Ice paths referenced by a root.
///
/// This best-effort traversal does not parse or check the merged document, so
/// a development watcher can observe a missing import and retry when it is
/// created.
pub fn discover_file_dependencies(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, Error> {
    let path = absolute_lexical_path(path.as_ref()).map_err(|error| {
        file_error(
            "E181",
            path.as_ref(),
            1,
            format!("cannot resolve source path: {error}"),
        )
    })?;
    let mut dependencies = Vec::new();
    let mut visited = HashSet::new();
    discover_dependencies_into(&path, true, &mut dependencies, &mut visited)?;
    dependencies.sort();
    dependencies.dedup();
    Ok(dependencies)
}

/// Returns host assets referenced by an otherwise valid Ice source graph.
///
/// Asset existence is deliberately not checked, allowing a watcher to recover
/// when a missing font or window icon is created.
pub fn discover_file_asset_dependencies(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, Error> {
    let loaded = load(path.as_ref())?;
    let document = analyze_loaded_without_assets(&loaded)?;
    Ok(asset_dependencies(&document, &loaded))
}

/// Analyze an unsaved root buffer while resolving its `use` graph from disk.
pub fn analyze_file_with_source(
    path: impl AsRef<Path>,
    source: &str,
) -> Result<CheckedDocument, Error> {
    let path = path.as_ref();
    let overlays = HashMap::from([(path.to_owned(), source)]);
    let loaded = load_with_overlays(path, &overlays)?;
    analyze_loaded(&loaded)
}

/// Analyze a file graph with in-memory sources replacing matching disk files.
pub fn analyze_file_with_overlays(
    path: impl AsRef<Path>,
    overlays: &HashMap<PathBuf, String>,
) -> Result<CheckedDocument, Error> {
    let loaded = load_with_overlays(path.as_ref(), overlays)?;
    analyze_loaded(&loaded)
}

pub fn compile_file(path: impl AsRef<Path>) -> Result<FileCompilation, Error> {
    let loaded = load(path.as_ref())?;
    let document = analyze_loaded(&loaded)?;
    let asset_dependencies = asset_dependencies(&document, &loaded);
    let root = loaded
        .dependencies
        .first()
        .expect("a loaded source always has a root");
    let mut rust = codegen::generate(&document, &root.display().to_string())
        .map_err(|error| remap_error(error, &loaded))?;
    for dependency in loaded.dependencies.iter().skip(1) {
        rust.push_str(&format!(
            "const _: &str = include_str!({:?});\n",
            dependency.display().to_string()
        ));
    }
    Ok(FileCompilation {
        rust,
        dependencies: loaded.dependencies,
        asset_dependencies,
    })
}

fn analyze_loaded(loaded: &LoadedSource) -> Result<CheckedDocument, Error> {
    let document = analyze_loaded_without_assets(loaded)?;
    check_assets(&document, loaded).map_err(|error| remap_error(error, loaded))?;
    Ok(document)
}

fn analyze_loaded_without_assets(loaded: &LoadedSource) -> Result<CheckedDocument, Error> {
    let namespaces = loaded
        .origins
        .iter()
        .map(|origin| origin.namespace.clone())
        .collect::<Vec<_>>();
    let (document, symbols) =
        parser::parse_with_symbols_and_namespaces(&loaded.source, &namespaces)
            .map_err(|error| remap_error(error, loaded))?;
    let document = check::analyze(document).map_err(|error| remap_error(error, loaded))?;
    Ok(document
        .with_parsed_symbols(remap_symbols(symbols, loaded))
        .with_source_origins(
            loaded
                .origins
                .iter()
                .map(|origin| (origin.path.clone(), origin.line))
                .collect(),
        ))
}

fn asset_dependencies(document: &Document, loaded: &LoadedSource) -> Vec<PathBuf> {
    let root = loaded
        .dependencies
        .first()
        .expect("a loaded source always has a root");
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    let mut dependencies = document
        .settings
        .fonts
        .iter()
        .map(|font| parent.join(&font.path))
        .chain(
            document
                .settings
                .window
                .iter()
                .chain(
                    document
                        .settings
                        .windows
                        .iter()
                        .map(|window| &window.settings),
                )
                .filter_map(|window| window.icon.as_ref())
                .map(|icon| parent.join(&icon.path)),
        )
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

fn remap_symbols(
    mut symbols: Vec<parser::ParsedSymbol>,
    loaded: &LoadedSource,
) -> Vec<parser::ParsedSymbol> {
    for symbol in &mut symbols {
        let Some(range) = &mut symbol.range else {
            continue;
        };
        let Some(origin) = range
            .line
            .checked_sub(1)
            .and_then(|index| loaded.origins.get(index))
        else {
            symbol.range = None;
            continue;
        };
        range.path = Some(origin.path.clone());
        range.line = origin.line;
    }
    symbols
}

fn check_assets(document: &Document, loaded: &LoadedSource) -> Result<(), Error> {
    let root = loaded
        .dependencies
        .first()
        .expect("a loaded source always has a root");
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    for font in &document.settings.fonts {
        let path = parent.join(&font.path);
        if !path.is_file() {
            return Err(Error::new(
                "E192",
                &font.span,
                format!("cannot read font file `{}`", path.display()),
            ));
        }
    }
    for icon in document
        .settings
        .window
        .iter()
        .chain(
            document
                .settings
                .windows
                .iter()
                .map(|window| &window.settings),
        )
        .filter_map(|window| window.icon.as_ref())
    {
        let path = parent.join(&icon.path);
        if !path.is_file() {
            return Err(Error::new(
                "E192",
                &icon.span,
                format!("cannot read window icon file `{}`", path.display()),
            ));
        }
        let actual = fs::metadata(&path)
            .map_err(|error| {
                Error::new(
                    "E192",
                    &icon.span,
                    format!(
                        "cannot inspect window icon file `{}`: {error}",
                        path.display()
                    ),
                )
            })?
            .len();
        if actual != icon.byte_len as u64 {
            return Err(Error::new(
                "E193",
                &icon.span,
                format!(
                    "window icon `{}` has {actual} RGBA bytes; expected {} for {} × {}",
                    path.display(),
                    icon.byte_len,
                    icon.width,
                    icon.height
                ),
            ));
        }
    }
    Ok(())
}

fn load(path: &Path) -> Result<LoadedSource, Error> {
    load_with_overlays(path, &HashMap::<PathBuf, String>::new())
}

#[cfg(test)]
pub(crate) fn load_test_source(path: &Path) -> Result<String, Error> {
    Ok(load(path)?.source)
}

fn load_with_overlays<S: AsRef<str>>(
    path: &Path,
    overlays: &HashMap<PathBuf, S>,
) -> Result<LoadedSource, Error> {
    let mut normalized_overlays = HashMap::new();
    for (overlay_path, source) in overlays {
        let normalized = normalize_overlay_path(overlay_path).map_err(|error| {
            file_error(
                "E181",
                overlay_path,
                1,
                format!("cannot resolve overlay path: {error}"),
            )
        })?;
        normalized_overlays.insert(normalized, source.as_ref());
    }
    let root = resolve(path, path, 1, &normalized_overlays)?;
    let mut overlays = normalized_overlays;
    let disk_source = if overlays.contains_key(&root) {
        None
    } else {
        Some(fs::read_to_string(&root).map_err(|error| {
            file_error("E181", &root, 1, format!("cannot read .ice file: {error}"))
        })?)
    };
    let root_source = overlays
        .get(&root)
        .copied()
        .or(disk_source.as_deref())
        .expect("a root source is loaded from an overlay or disk");
    if !source_is_app(root_source) {
        return Err(file_error(
            "E183",
            &root,
            1,
            "a root must declare `app Name` or `daemon Name`; import this fragment from an app or daemon instead",
        ));
    }
    if let Some(source) = disk_source.as_deref() {
        overlays.insert(root.clone(), source);
    }
    let mut loaded = LoadedSource {
        source: String::new(),
        origins: Vec::new(),
        dependencies: Vec::new(),
    };
    let mut included = HashSet::new();
    let mut stack = Vec::new();
    load_into(
        &root,
        (&root, 1),
        None,
        &overlays,
        &mut loaded,
        &mut included,
        &mut stack,
    )?;
    Ok(loaded)
}

fn load_into(
    path: &Path,
    imported_from: (&Path, usize),
    namespace: Option<String>,
    overlays: &HashMap<PathBuf, &str>,
    loaded: &mut LoadedSource,
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
    if !loaded.dependencies.contains(&path.to_owned()) {
        loaded.dependencies.push(path.to_owned());
    }
    let disk_source;
    let source = if let Some(source) = overlays.get(path) {
        *source
    } else {
        disk_source = fs::read_to_string(path).map_err(|error| {
            file_error("E181", path, 1, format!("cannot read .ice file: {error}"))
        })?;
        &disk_source
    };
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        if raw.len() == raw.trim_start().len() && raw.starts_with("use ") {
            let import = parse_use(raw, path, line)?;
            let target = resolve(
                &path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(import.path),
                path,
                line,
                overlays,
            )?;
            let child_namespace =
                import.alias.map_or_else(
                    || namespace.clone(),
                    |alias| {
                        Some(namespace.as_ref().map_or_else(
                            || alias.to_owned(),
                            |parent| format!("{parent}::{alias}"),
                        ))
                    },
                );
            load_into(
                &target,
                (path, line),
                child_namespace,
                overlays,
                loaded,
                included,
                stack,
            )?;
        } else {
            loaded.source.push_str(raw);
            loaded.source.push('\n');
            loaded.origins.push(Origin {
                path: path.to_owned(),
                line,
                namespace: namespace.clone(),
            });
        }
    }
    stack.pop();
    Ok(())
}

#[derive(Debug)]
struct Import<'a> {
    path: &'a str,
    alias: Option<&'a str>,
}

fn parse_use<'a>(source: &'a str, path: &Path, line: usize) -> Result<Import<'a>, Error> {
    let value = source[4..].trim();
    let Some(value) = value.strip_prefix('"') else {
        return Err(file_error(
            "E180",
            path,
            line,
            "imports use `use \"relative/file.ice\"` or `use \"relative/file.ice\" as name`",
        ));
    };
    let Some(end) = value.find('"') else {
        return Err(file_error(
            "E180",
            path,
            line,
            "imports use `use \"relative/file.ice\"` or `use \"relative/file.ice\" as name`",
        ));
    };
    let import_path = &value[..end];
    let rest = value[end + 1..].trim();
    let alias = if rest.is_empty() {
        None
    } else if let Some(alias) = rest.strip_prefix("as ").map(str::trim)
        && crate::valid_identifier(alias)
    {
        Some(alias)
    } else {
        return Err(file_error(
            "E180",
            path,
            line,
            "an import alias uses `use \"relative/file.ice\" as name`",
        ));
    };
    let import = Path::new(import_path);
    if import_path.contains('\\')
        || crate::has_windows_drive_prefix(import_path)
        || import.is_absolute()
        || import.extension().and_then(|ext| ext.to_str()) != Some("ice")
    {
        return Err(file_error(
            "E180",
            path,
            line,
            "import paths must be relative `/` paths ending in `.ice`",
        ));
    }
    Ok(Import {
        path: import_path,
        alias,
    })
}

fn discover_dependencies_into(
    path: &Path,
    required: bool,
    dependencies: &mut Vec<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), Error> {
    dependencies.push(path.to_owned());
    let target = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    dependencies.push(target.clone());
    if !visited.insert(target.clone()) {
        return Ok(());
    }
    let source = match fs::read_to_string(&target) {
        Ok(source) => source,
        Err(_) if !required => return Ok(()),
        Err(error) => {
            return Err(file_error(
                "E181",
                &target,
                1,
                format!("cannot read .ice file: {error}"),
            ));
        }
    };
    for (index, raw) in source.lines().enumerate() {
        if raw.len() != raw.trim_start().len() || !raw.starts_with("use ") {
            continue;
        }
        let Ok(import) = parse_use(raw, &target, index + 1) else {
            continue;
        };
        let candidate = normalize_path_lexically(
            &target
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(import.path),
        );
        discover_dependencies_into(&candidate, false, dependencies, visited)?;
    }
    Ok(())
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
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
    normalized
}

fn absolute_lexical_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(normalize_path_lexically(path))
    } else {
        std::env::current_dir().map(|current| normalize_path_lexically(&current.join(path)))
    }
}

fn resolve(
    path: &Path,
    source: &Path,
    line: usize,
    overlays: &HashMap<PathBuf, &str>,
) -> Result<PathBuf, Error> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) => match normalize_overlay_path(path) {
            Ok(path) if overlays.contains_key(&path) => Ok(path),
            _ => Err(file_error(
                "E181",
                source,
                line,
                format!("cannot read `{}`: {error}", path.display()),
            )),
        },
    }
}

fn normalize_overlay_path(path: &Path) -> std::io::Result<PathBuf> {
    path.canonicalize().or_else(|_| {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let name = path.file_name().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "overlay path has no file name",
            )
        })?;
        parent.canonicalize().map(|parent| parent.join(name))
    })
}

fn remap_error(mut error: Error, loaded: &LoadedSource) -> Error {
    if let Some(origin) = error
        .line
        .checked_sub(1)
        .and_then(|index| loaded.origins.get(index))
    {
        error.line = origin.line;
        error.path = Some(origin.path.display().to_string());
    }
    error
}

fn file_error(code: &'static str, path: &Path, line: usize, message: impl Into<String>) -> Error {
    Error::new(code, &Span::line(line), message).at_path(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_file, analyze_file_graph, analyze_file_with_overlays, analyze_file_with_source,
        compile_file, discover_file_asset_dependencies, discover_file_dependencies, parse_use,
        source_is_app,
    };
    use crate::SymbolKind;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("ui-lang-source-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, source: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, source).unwrap();
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.0.join(relative)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn compiles_relative_and_nested_imports_once() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"shared/theme.ice\"\nuse \"parts/body.ice\"\nview\n  Card\n",
        );
        fixture.write(
            "shared/theme.ice",
            "theme contract AppTheme\n  fg\n  bg\n  primary\n  danger\npalette app for AppTheme\n  fg #ffffff\n  bg #000000\n  primary #333333\n  danger #ff0000\n",
        );
        fixture.write(
            "parts/body.ice",
            "use \"../shared/theme.ice\"\ncomponent Card()\n  text \"Hello\" @text-fg\n",
        );

        let compiled = compile_file(fixture.path("app.ice")).unwrap();

        assert_eq!(compiled.dependencies.len(), 3);
        assert!(compiled.rust.contains("struct Demo"));
        assert_eq!(compiled.rust.matches("include_str!").count(), 3);
    }

    #[test]
    fn analysis_reports_source_and_asset_inputs() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            concat!(
                "app Demo\n",
                "  font \"font.ttf\"\n",
                "use \"part.ice\"\n",
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
                "  Card\n",
            ),
        );
        fixture.write("part.ice", "component Card()\n  text \"Card\"\n");
        fixture.write("font.ttf", "font bytes");

        let analysis = analyze_file_graph(fixture.path("app.ice")).unwrap();
        let compilation = compile_file(fixture.path("app.ice")).unwrap();

        assert!(analysis.dependencies.contains(&fixture.path("app.ice")));
        assert!(analysis.dependencies.contains(&fixture.path("part.ice")));
        assert_eq!(analysis.asset_dependencies, [fixture.path("font.ttf")]);
        assert_eq!(compilation.asset_dependencies, [fixture.path("font.ttf")]);
    }

    #[test]
    fn dependency_discovery_keeps_missing_imports_watchable() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"missing.ice\"\nview\n  Missing\n",
        );

        let dependencies = discover_file_dependencies(fixture.path("app.ice")).unwrap();

        assert!(dependencies.contains(&fixture.path("missing.ice")));
    }

    #[test]
    fn asset_discovery_keeps_missing_assets_watchable() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            concat!(
                "app Demo\n",
                "  font \"missing.ttf\"\n",
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
                "  text \"Ready\"\n",
            ),
        );

        assert_eq!(
            discover_file_asset_dependencies(fixture.path("app.ice")).unwrap(),
            [fixture.path("missing.ttf")]
        );
    }

    #[test]
    fn aliases_the_same_module_into_distinct_checked_namespaces() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"ui.ice\" as first\nuse \"ui.ice\" as second\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  first_data:first::Data = first::data(\"A\")\n  second_data:second::Data = second::data(\"B\")\n  first_mode:first::Mode = first::Mode.idle\n  second_mode:second::Mode = second::Mode.ready(\"B\")\nview\n  row\n    first::Badge value=first_data mode=first_mode\n    second::Badge value=second_data mode=second_mode\n    match second_mode\n      second::Mode.idle\n        text \"Idle\"\n      second::Mode.ready(label)\n        text label\n",
        );
        fixture.write(
            "ui.ice",
            "extern crate::helpers\n  Data(value:str)\n  sync data(value:str) -> Data\n  sync label(value:str) -> str\nenum Mode\n  idle\n  ready(str)\nrecipe panel for box\n  @p-4\nfont body family=\"Inter\"\ncomponent Badge(value:Data, mode:Mode)\n  box @panel\n    col\n      text label(trim(value.value)) font=body\n      match mode\n        Mode.idle\n          text \"Idle\"\n        Mode.ready(label)\n          text label\n",
        );

        let document = analyze_file(fixture.path("app.ice")).unwrap();
        assert_eq!(
            document
                .components
                .iter()
                .map(|component| component.name.as_str())
                .collect::<Vec<_>>(),
            ["first::Badge", "second::Badge"]
        );
        assert!(
            document
                .recipes
                .iter()
                .any(|recipe| recipe.name == "first::panel")
        );
        assert!(
            document
                .fonts
                .iter()
                .any(|font| font.name == "second::body")
        );
        assert!(
            document
                .structs
                .iter()
                .any(|item| item.name == "first::Data")
        );
        assert!(
            document
                .functions
                .iter()
                .any(|function| function.name == "second::label")
        );
        assert!(document.enums.iter().any(|item| item.name == "first::Mode"));

        let compiled = compile_file(fixture.path("app.ice")).unwrap();
        assert!(compiled.rust.contains("crate::helpers::label"));
        assert!(!compiled.rust.contains("enum first::Mode"));
        assert_eq!(compiled.dependencies.len(), 2);
    }

    #[test]
    fn checks_bind_props_and_named_events_across_an_alias() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"ui.ice\" as ui\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  draft = \"Draft\"\non changed(value)\n  draft = value\nview\n  ui::Field value<->draft\n    events\n      change -> changed _\n",
        );
        fixture.write(
            "ui.ice",
            "component Field(bind value:str)\n  emits\n    change(str)\n  col\n    input \"Value\" <-> value\n    button \"Apply\" -> emit(change, value)\n",
        );

        let compiled = compile_file(fixture.path("app.ice")).unwrap();
        assert!(compiled.rust.contains("__BindDraft"));
        assert!(
            compiled
                .rust
                .contains("move |__event_0| __DemoMessage::Changed(__event_0)")
        );
    }

    #[test]
    fn nests_import_aliases() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"ui.ice\" as ui\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  ui::parts::Badge\n",
        );
        fixture.write("ui.ice", "use \"parts.ice\" as parts\n");
        fixture.write("parts.ice", "component Badge()\n  text \"Badge\"\n");

        let document = analyze_file(fixture.path("app.ice")).unwrap();

        assert_eq!(document.components[0].name, "ui::parts::Badge");
    }

    #[test]
    fn aliased_imports_keep_the_theme_contract_global() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"ui.ice\" as ui\nview\n  ui::Badge\n",
        );
        fixture.write(
            "ui.ice",
            "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette light for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Badge()\n  text \"Badge\"\n",
        );

        let document = analyze_file(fixture.path("app.ice")).unwrap();
        assert_eq!(document.theme_contract.as_ref().unwrap().name, "AppTheme");
        assert_eq!(document.palettes[0].name, "light");
        assert_eq!(document.components[0].name, "ui::Badge");
    }

    #[test]
    fn compiles_provided_for_an_aliased_optional_slot_component() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"ui.ice\" as ui\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  col\n    ui::Card\n    ui::Card\n      ui::Card.Footer\n        text \"Footer\"\n",
        );
        fixture.write(
            "ui.ice",
            "component Card()\n  col\n    text \"Body\"\n    if provided(Footer)\n      slot Footer?\ncomponent Card.Footer()\n  box\n    slot\n",
        );

        let compiled = compile_file(fixture.path("app.ice")).unwrap();
        assert!(compiled.rust.contains("Footer"));
    }

    #[test]
    fn resolves_recipe_inheritance_inside_an_aliased_import() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"ui.ice\" as ui\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non save\nview\n  button \"Save\" @ui::primary_action -> save\n",
        );
        fixture.write(
            "ui.ice",
            "recipe action for button\n  @p-4\nrecipe primary_action for button extends action\n  @bg-primary\n",
        );

        let document = analyze_file(fixture.path("app.ice")).unwrap();
        assert_eq!(document.recipes[1].base.as_deref(), Some("ui::action"));
    }

    #[test]
    fn rejects_host_independent_absolute_imports() {
        let error = parse_use(r#"use "C:/tmp/part.ice""#, Path::new("app.ice"), 1).unwrap_err();

        assert_eq!(error.code, "E180");
        assert!(error.message.contains("relative"));
    }

    #[test]
    fn analyzes_an_unsaved_root_with_disk_imports_and_source_mapping() {
        let fixture = Fixture::new();
        fixture.write("app.ice", "app Saved\nview\n  text \"Saved\"\n");
        fixture.write("part.ice", "component Broken()\n  wat\n");
        let overlay = "app Overlay\nuse \"part.ice\"\nview\n  Broken\n";

        let error = analyze_file_with_source(fixture.path("app.ice"), overlay).unwrap_err();

        assert_eq!(error.code, "E064");
        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("part.ice"));
    }

    #[test]
    fn analyzes_and_recovers_an_unsaved_import_overlay() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Saved\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Saved\"\n",
        );
        fixture.write("part.ice", "component Broken()\n  text \"Saved\"\n");
        let root = fixture.path("app.ice");
        let part = fixture.path("part.ice");
        let mut overlays = HashMap::from([
            (
                root.clone(),
                "app Overlay\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Broken\n"
                    .into(),
            ),
            (part.clone(), "component Broken()\n  wat\n".into()),
        ]);

        let error = analyze_file_with_overlays(&root, &overlays).unwrap_err();

        assert_eq!(error.code, "E064");
        assert_eq!(error.line, 2);
        assert_eq!(error.path.as_deref(), Some(part.to_string_lossy().as_ref()));

        overlays.insert(part, "component Broken()\n  text \"Unsaved\"\n".into());
        analyze_file_with_overlays(root, &overlays).unwrap();
    }

    #[test]
    fn analyzes_new_unsaved_root_and_import_files() {
        let fixture = Fixture::new();
        let root = fixture.path("new.ice");
        let part = fixture.path("part.ice");
        let overlays = HashMap::from([
            (
                root.clone(),
                "app New\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n"
                    .to_owned(),
            ),
            (part, "component Card()\n  text \"Unsaved\"\n".to_owned()),
        ]);

        analyze_file_with_overlays(root, &overlays).unwrap();
    }

    #[test]
    fn retains_checked_component_and_handler_locations_across_imports() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non clicked\nview\n  Card\n    events\n      click -> clicked\n";
        fixture.write("app.ice", root);
        fixture.write(
            "part.ice",
            "component Card()\n  emits\n    click\n  button \"Go\" -> emit(click)\n",
        );

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let part = fixture.path("part.ice").canonicalize().unwrap();
        let (component, _) = checked.symbol_at(Some(&app), 15, 3).unwrap();
        let (handler, _) = checked.symbol_at(Some(&app), 17, 16).unwrap();

        assert_eq!(component.name, "Card");
        assert_eq!(component.definition.path.as_deref(), Some(part.as_path()));
        assert_eq!(component.definition.line, 1);
        assert_eq!(component.references.len(), 1);
        assert!(component.renameable);
        assert_eq!(handler.name, "clicked");
        assert_eq!(handler.definition.line, 13);
        assert_eq!(handler.references.len(), 1);
        assert!(handler.renameable);
    }

    #[test]
    fn keeps_reused_test_target_aliases_scoped_to_their_test() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non clicked\nview\n  col #root\n    button \"Go\" #action -> clicked\ntest first\n  target root = #root\n  target action = root/action\n  expect root.width == root.height\n  click action\ntest second\n  target root = #root\n  expect root.visible\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let aliases = checked
            .symbols()
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::TestTarget)
            .collect::<Vec<_>>();

        assert_eq!(aliases.len(), 3);
        let first_root = aliases
            .iter()
            .find(|symbol| symbol.scope.as_deref() == Some("first") && symbol.name == "root")
            .unwrap();
        let second_root = aliases
            .iter()
            .find(|symbol| symbol.scope.as_deref() == Some("second") && symbol.name == "root")
            .unwrap();
        assert_eq!(first_root.definition.line, 17);
        assert_eq!(first_root.references.len(), 3);
        assert_eq!(second_root.definition.line, 22);
        assert_eq!(second_root.references.len(), 1);
        assert!(first_root.renameable);
        assert!(second_root.renameable);
    }

    #[test]
    fn tracks_dynamic_test_target_key_aliases_in_every_target_context() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  col #root\n    text \"Key\" #key\n    text \"Item\" #item(\"Key\")\ntest dynamic_targets\n  target key = #root/key\n  target item = #root/item(key.value)\n  expect exists #root/item(key.value)\n  expect text \"Item\" within #root/item(key.value)\n  click #root/item(key.value)\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let key = checked
            .symbols()
            .iter()
            .find(|symbol| {
                symbol.kind == SymbolKind::TestTarget
                    && symbol.scope.as_deref() == Some("dynamic_targets")
                    && symbol.name == "key"
            })
            .unwrap();

        assert_eq!(key.definition.line, 17);
        assert_eq!(
            key.references
                .iter()
                .map(|reference| reference.line)
                .collect::<Vec<_>>(),
            [18, 19, 20, 21]
        );
    }

    #[test]
    fn generated_test_locations_retain_imported_source_origins() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"tests.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  col #root\n",
        );
        fixture.write(
            "tests.ice",
            "test imported\n  target root = #root\n  release\n  expect root.visible\n",
        );
        let root = fixture.path("app.ice");
        let part = fixture.path("tests.ice").canonicalize().unwrap();

        let checked = analyze_file_with_source(&root, &fs::read_to_string(&root).unwrap()).unwrap();
        let release = &checked.tests[0].steps[0].span;
        let (origin, line) = checked.source_origin(release.line).unwrap();
        assert_eq!(origin, part);
        assert_eq!(line, 3);

        let generated = compile_file(root).unwrap().rust;
        assert!(
            generated.contains(&format!(
                "Location::new({:?}, 3, 3, \"release\")",
                part.display().to_string()
            )),
            "generated test did not retain the imported location:\n{generated}"
        );
        assert!(
            generated.contains(&format!(
                "Config::new(\"imported\").source(::ui_lang_runtime::testing::Location::new({:?}, 1, 1, \"test imported\"))",
                part.display().to_string()
            )),
            "generated test config did not retain the imported declaration location:\n{generated}"
        );
    }

    #[test]
    fn generated_rust_markers_retain_imported_view_origins() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"components.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n",
        );
        fixture.write("components.ice", "component Card()\n  text \"Imported\"\n");
        let component = fixture.path("components.ice").canonicalize().unwrap();
        let encoded_path = component
            .display()
            .to_string()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let generated = compile_file(fixture.path("app.ice")).unwrap().rust;
        assert!(
            generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_path}")),
            "generated Rust did not retain the imported view location:\n{generated}"
        );
        assert!(
            generated.contains(&format!(
                "Location::new({:?}, 2, 1, \"rendered view node\")",
                component.display().to_string()
            )),
            "generated render provenance did not retain the imported view location:\n{generated}"
        );
    }

    #[test]
    fn keeps_component_local_handlers_out_of_global_symbol_navigation() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Toggle()\n  state\n    enabled = false\n  on changed(next)\n    enabled = next\n  checkbox \"Enabled\" checked=enabled -> changed _\non changed\nview\n  Toggle #toggle\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let changed = checked
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == SymbolKind::Handler && symbol.name == "changed")
            .unwrap();

        assert_eq!(changed.definition.line, 18);
        assert!(changed.references.is_empty());
        assert!(checked.symbol_at(Some(&app), 15, 6).is_none());
        let route_column = root.lines().nth(16).unwrap().rfind("changed").unwrap() + 1;
        assert!(checked.symbol_at(Some(&app), 17, route_column).is_none());
    }

    #[test]
    fn keeps_component_outputs_out_of_global_handler_navigation() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Choice() -> bool\n  checkbox \"Choice\" checked=false -> emit(_)\non emit(value)\non changed(value)\nview\n  col\n    Choice -> changed _\n    checkbox \"Global\" checked=false -> emit(_)\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let emit = checked
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == SymbolKind::Handler && symbol.name == "emit")
            .unwrap();
        let output_route = root.lines().nth(12).unwrap();
        let output_column = output_route.rfind("emit").unwrap() + 1;

        assert_eq!(emit.references.len(), 1);
        assert!(checked.symbol_at(Some(&app), 13, output_column).is_none());
    }

    #[test]
    fn rejects_global_handler_routes_from_component_handlers() {
        let fixture = Fixture::new();
        let root = "app Demo\nextern crate::backend\n  fetch() -> str\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Search()\n  on search\n    run fetch() -> loaded _\n  button \"Search\" -> search\non loaded(value)\nview\n  Search\n";
        fixture.write("app.ice", root);

        let error = analyze_file_with_source(fixture.path("app.ice"), root).unwrap_err();
        assert_eq!(error.code, "E132");
        assert!(
            error
                .message
                .contains("cannot reference app handler `loaded`")
        );
    }

    #[test]
    fn retains_handler_locations_in_named_route_properties() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  draft:str = \"\"\non submit\nview\n  input \"Draft\" <-> draft submit=submit\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let route = root.lines().nth(15).unwrap();
        let column = route.rfind("submit").unwrap() + 1;
        let (handler, reference) = checked.symbol_at(Some(&app), 16, column).unwrap();

        assert_eq!(handler.name, "submit");
        assert_eq!(reference.start_column, column);
        assert!(handler.renameable);
    }

    #[test]
    fn retains_recipe_locations_across_imports() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"recipes.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\n  surface\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n  surface #111111\nview\n  box @panel\n    text \"Panel\"\n";
        fixture.write("app.ice", root);
        fixture.write(
            "recipes.ice",
            "recipe surface for box\n  @bg-surface\nrecipe panel for box extends surface\n  @p-4 rounded-md\n",
        );

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let recipes = fixture.path("recipes.ice").canonicalize().unwrap();
        let line = root.lines().nth(15).unwrap();
        let column = line.find("panel").unwrap() + 1;
        let (panel, reference) = checked.symbol_at(Some(&app), 16, column).unwrap();

        assert_eq!(panel.kind, SymbolKind::Recipe);
        assert_eq!(panel.name, "panel");
        assert_eq!(panel.definition.path.as_deref(), Some(recipes.as_path()));
        assert_eq!(panel.references.as_slice(), std::slice::from_ref(reference));
        assert!(panel.renameable);

        let surface = checked
            .symbols()
            .iter()
            .find(|symbol| symbol.name == "surface")
            .unwrap();
        assert_eq!(surface.kind, SymbolKind::Recipe);
        assert_eq!(surface.references.len(), 1);
        assert_eq!(
            surface.references[0].path.as_deref(),
            Some(recipes.as_path())
        );
    }

    #[test]
    fn preserves_with_metadata_source_lines_across_imports() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"recipes.ice\"\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n",
        );
        fixture.write("recipes.ice", "recipe panel for box\n  @p-4\n");
        let part =
            "component Card()\n  box\n    with\n      w=fill\n      @panel\n    text \"Card\"\n";
        fixture.write("part.ice", part);

        let checked = analyze_file_with_overlays(fixture.path("app.ice"), &HashMap::new()).unwrap();
        let part_path = fixture.path("part.ice").canonicalize().unwrap();
        let (panel, reference) = checked.symbol_at(Some(&part_path), 5, 8).unwrap();
        assert_eq!(panel.kind, SymbolKind::Recipe);
        assert_eq!(reference.line, 5);
        assert_eq!(reference.start_column, 8);

        let overlays = HashMap::from([(
            part_path,
            part.replace("      @panel", "      unknown=true\n      @panel"),
        )]);
        let error = analyze_file_with_overlays(fixture.path("app.ice"), &overlays).unwrap_err();
        assert_eq!(error.line, 5);
        assert!(error.path.as_deref().unwrap().ends_with("part.ice"));
    }

    #[test]
    fn keeps_the_implicit_mount_hook_out_of_rename() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non mount\nview\n  text \"Ready\"\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let mount = checked
            .symbols()
            .iter()
            .find(|symbol| symbol.name == "mount")
            .unwrap();

        assert_eq!(mount.kind, SymbolKind::Handler);
        assert!(!mount.renameable);
    }

    #[test]
    fn keeps_component_emit_out_of_navigation() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Toggle() -> bool\n  checkbox \"Toggle\" checked=false -> emit(_)\non changed(value)\nview\n  Toggle -> changed _\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let route = root.lines().nth(12).unwrap();
        let column = route.find("emit").unwrap() + 1;

        assert!(checked.symbol_at(Some(&app), 13, column).is_none());
    }

    #[test]
    fn counts_unicode_indentation_in_source_columns() {
        let fixture = Fixture::new();
        let indent = "\u{a0}\u{a0}";
        let root = format!(
            "app Demo\ntheme contract AppTheme\n{indent}bg\n{indent}fg\n{indent}primary\n{indent}danger\npalette app for AppTheme\n{indent}bg #000000\n{indent}fg #ffffff\n{indent}primary #333333\n{indent}danger #ff0000\ncomponent Card()\n{indent}text \"Card\"\nview\n{indent}Card\n"
        );
        fixture.write("app.ice", &root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), &root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let (card, reference) = checked.symbol_at(Some(&app), 15, 3).unwrap();

        assert_eq!(card.name, "Card");
        assert_eq!(reference.start_column, 3);
        assert_eq!(reference.end_column, 7);
    }

    #[test]
    fn retains_compact_canvas_routes_without_synthetic_references() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non pressed(button)\non canvas_event\nview\n  canvas\n    event mouse pressed -> pressed _\n    capture touch lost\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let route = root.lines().nth(15).unwrap();
        let column = route.rfind("pressed").unwrap() + 1;
        let (pressed, reference) = checked.symbol_at(Some(&app), 16, column).unwrap();
        let synthetic = checked
            .symbols()
            .iter()
            .find(|symbol| symbol.name == "canvas_event")
            .unwrap();

        assert_eq!(pressed.name, "pressed");
        assert_eq!(reference.start_column, column);
        assert_eq!(synthetic.references.len(), 0);
    }

    #[test]
    fn reports_an_import_cycle_at_the_imported_file() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"part.ice\"\nview\n  text \"Hi\"\n",
        );
        fixture.write("part.ice", "use \"app.ice\"\n");

        let error = compile_file(fixture.path("app.ice")).unwrap_err();

        assert_eq!(error.code, "E182");
        assert!(error.path.as_deref().unwrap().ends_with("part.ice"));
        assert_eq!(error.line, 1);
        assert!(error.message.contains("cyclic `use`"));
    }

    #[test]
    fn reports_a_missing_import_at_the_use_site() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"missing.ice\"\nview\n  text \"Hi\"\n",
        );

        let error = compile_file(fixture.path("app.ice")).unwrap_err();

        assert_eq!(error.code, "E181");
        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("app.ice"));
        assert!(error.message.contains("missing.ice"));
    }

    #[test]
    fn checks_and_embeds_app_font_files_relative_to_the_root() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  font \"assets/Brand.ttf\"\n  window\n    icon-rgba \"assets/app.rgba\" 2 1\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        fixture.write("assets/Brand.ttf", "font bytes");
        fixture.write("assets/app.rgba", "RGBAABC\n");

        let compiled = compile_file(fixture.path("app.ice")).unwrap();
        let font = fixture.path("assets/Brand.ttf");
        assert!(compiled.rust.contains(&format!(
            ".font(include_bytes!({:?}).as_slice())",
            font.display().to_string()
        )));
        let icon = fixture.path("assets/app.rgba");
        assert!(
            compiled
                .rust
                .contains(&format!("include_bytes!({:?})", icon.display().to_string()))
        );
    }

    #[test]
    fn reports_a_missing_app_font_at_its_setting() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  font \"assets/Missing.ttf\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );

        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E192");
        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("app.ice"));
        assert!(error.message.contains("assets/Missing.ttf"));

        fixture.write(
            "app.ice",
            "app Demo\n  window child\n    icon-rgba \"assets/missing.rgba\" 2 1\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E192");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("assets/missing.rgba"));

        fixture.write("assets/wrong.rgba", "RGBA");
        fixture.write(
            "app.ice",
            "app Demo\n  window\n    icon-rgba \"assets/wrong.rgba\" 2 1\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E193");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("has 4 RGBA bytes; expected 8"));
    }

    #[test]
    fn remaps_language_errors_to_the_fragment() {
        let fixture = Fixture::new();
        fixture.write("app.ice", "app Demo\nuse \"part.ice\"\nview\n  Broken\n");
        fixture.write("part.ice", "component Broken()\n  wat\n");

        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        let rendered = error.render(&fixture.path("app.ice").display().to_string());

        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("part.ice"));
        assert!(rendered.contains("part.ice:2:1:"));
        assert!(rendered.contains("2 |   wat\n  | ^"));
    }

    #[test]
    fn only_top_level_app_declarations_make_roots() {
        assert!(source_is_app("app Demo\nview\n  text \"Hi\"\n"));
        assert!(source_is_app("daemon Agent\nview\n  text \"Hi\"\n"));
        assert!(!source_is_app("component Card()\n  text \"app Demo\"\n"));
    }

    #[test]
    fn rejects_a_fragment_as_a_root() {
        let fixture = Fixture::new();
        fixture.write("part.ice", "component Card()\n  text \"Hi\"\n");

        let error = compile_file(fixture.path("part.ice")).unwrap_err();

        assert_eq!(error.code, "E183");
        assert!(error.message.contains("`app Name` or `daemon Name`"));
    }
}
