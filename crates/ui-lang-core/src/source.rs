use crate::{CheckedDocument, Document, Error, Span, check, codegen, parser};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileCompilation {
    pub rust: String,
    pub dependencies: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
struct Origin {
    path: PathBuf,
    line: usize,
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
    let loaded = load(path.as_ref())?;
    analyze_loaded(&loaded)
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
    })
}

fn analyze_loaded(loaded: &LoadedSource) -> Result<CheckedDocument, Error> {
    let (document, symbols) =
        parser::parse_with_symbols(&loaded.source).map_err(|error| remap_error(error, loaded))?;
    let document = check::analyze(document).map_err(|error| remap_error(error, loaded))?;
    check_assets(&document, loaded).map_err(|error| remap_error(error, loaded))?;
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
        &root,
        1,
        &overlays,
        &mut loaded,
        &mut included,
        &mut stack,
    )?;
    Ok(loaded)
}

fn load_into(
    path: &Path,
    imported_from: &Path,
    import_line: usize,
    overlays: &HashMap<PathBuf, &str>,
    loaded: &mut LoadedSource,
    included: &mut HashSet<PathBuf>,
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
            imported_from,
            import_line,
            format!("cyclic `use`: {}", cycle.join(" -> ")),
        ));
    }
    if !included.insert(path.to_owned()) {
        return Ok(());
    }

    stack.push(path.to_owned());
    loaded.dependencies.push(path.to_owned());
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
            let relative = parse_use(raw, path, line)?;
            let target = resolve(
                &path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(relative),
                path,
                line,
                overlays,
            )?;
            load_into(&target, path, line, overlays, loaded, included, stack)?;
        } else {
            loaded.source.push_str(raw);
            loaded.source.push('\n');
            loaded.origins.push(Origin {
                path: path.to_owned(),
                line,
            });
        }
    }
    stack.pop();
    Ok(())
}

fn parse_use<'a>(source: &'a str, path: &Path, line: usize) -> Result<&'a str, Error> {
    let value = source[4..].trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(file_error(
            "E180",
            path,
            line,
            "imports use `use \"relative/file.ice\"`",
        ));
    }
    let value = &value[1..value.len() - 1];
    let import = Path::new(value);
    if value.contains('\\')
        || crate::has_windows_drive_prefix(value)
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
    Ok(value)
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
        analyze_file_with_overlays, analyze_file_with_source, compile_file, parse_use,
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
            "theme\n  fg #ffffff\n  bg #000000\n  primary #333333\n  danger #ff0000\n",
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
            "app Saved\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Saved\"\n",
        );
        fixture.write("part.ice", "component Broken()\n  text \"Saved\"\n");
        let root = fixture.path("app.ice");
        let part = fixture.path("part.ice");
        let mut overlays = HashMap::from([
            (
                root.clone(),
                "app Overlay\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Broken\n"
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
                "app New\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n"
                    .to_owned(),
            ),
            (part, "component Card()\n  text \"Unsaved\"\n".to_owned()),
        ]);

        analyze_file_with_overlays(root, &overlays).unwrap();
    }

    #[test]
    fn retains_checked_component_and_handler_locations_across_imports() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        fixture.write("app.ice", root);
        fixture.write(
            "part.ice",
            "component Card()\n  button \"Go\" -> clicked\non clicked\n",
        );

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let part = fixture.path("part.ice").canonicalize().unwrap();
        let (component, _) = checked.symbol_at(Some(&app), 9, 3).unwrap();
        let (handler, _) = checked.symbol_at(Some(&part), 2, 20).unwrap();

        assert_eq!(component.name, "Card");
        assert_eq!(component.definition.path.as_deref(), Some(part.as_path()));
        assert_eq!(component.definition.line, 1);
        assert_eq!(component.references.len(), 1);
        assert!(component.renameable);
        assert_eq!(handler.name, "clicked");
        assert_eq!(handler.definition.line, 3);
        assert_eq!(handler.references.len(), 1);
        assert!(handler.renameable);
    }

    #[test]
    fn keeps_reused_test_target_aliases_scoped_to_their_test() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non clicked\nview\n  col #root\n    button \"Go\" #action -> clicked\ntest first\n  target root = #root\n  target action = root/action\n  expect root.width == root.height\n  click action\ntest second\n  target root = #root\n  expect root.visible\n";
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
        assert_eq!(first_root.definition.line, 12);
        assert_eq!(first_root.references.len(), 3);
        assert_eq!(second_root.definition.line, 17);
        assert_eq!(second_root.references.len(), 1);
        assert!(first_root.renameable);
        assert!(second_root.renameable);
    }

    #[test]
    fn tracks_dynamic_test_target_key_aliases_in_every_target_context() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  col #root\n    text \"Key\" #key\n    text \"Item\" #item(\"Key\")\ntest dynamic_targets\n  target key = #root/key\n  target item = #root/item(key.value)\n  expect exists #root/item(key.value)\n  expect text \"Item\" within #root/item(key.value)\n  click #root/item(key.value)\n";
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

        assert_eq!(key.definition.line, 12);
        assert_eq!(
            key.references
                .iter()
                .map(|reference| reference.line)
                .collect::<Vec<_>>(),
            [13, 14, 15, 16]
        );
    }

    #[test]
    fn generated_test_locations_retain_imported_source_origins() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"tests.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  col #root\n",
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
    fn keeps_component_local_handlers_out_of_global_symbol_navigation() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Toggle()\n  state\n    enabled = false\n  on changed(next)\n    enabled = next\n  checkbox \"Enabled\" checked=enabled -> changed _\non changed\nview\n  Toggle #toggle\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let changed = checked
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == SymbolKind::Handler && symbol.name == "changed")
            .unwrap();

        assert_eq!(changed.definition.line, 13);
        assert!(changed.references.is_empty());
        assert!(checked.symbol_at(Some(&app), 10, 6).is_none());
        let route_column = root.lines().nth(11).unwrap().rfind("changed").unwrap() + 1;
        assert!(checked.symbol_at(Some(&app), 12, route_column).is_none());
    }

    #[test]
    fn keeps_component_outputs_out_of_global_handler_navigation() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Choice() -> bool\n  checkbox \"Choice\" checked=false -> emit _\non emit(value)\non changed(value)\nview\n  col\n    Choice -> changed _\n    checkbox \"Global\" checked=false -> emit _\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let emit = checked
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == SymbolKind::Handler && symbol.name == "emit")
            .unwrap();
        let output_route = root.lines().nth(7).unwrap();
        let output_column = output_route.rfind("emit").unwrap() + 1;

        assert_eq!(emit.references.len(), 1);
        assert!(checked.symbol_at(Some(&app), 8, output_column).is_none());
    }

    #[test]
    fn retains_global_handler_routes_from_component_handlers() {
        let fixture = Fixture::new();
        let root = "app Demo\nextern crate::backend\n  fetch() -> str\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Search()\n  on search\n    run fetch() -> loaded _\n  button \"Search\" -> search\non loaded(value)\nview\n  Search\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let route = root.lines().nth(10).unwrap();
        let route_column = route.rfind("loaded").unwrap() + 1;
        let (loaded, reference) = checked.symbol_at(Some(&app), 11, route_column).unwrap();

        assert_eq!(loaded.name, "loaded");
        assert_eq!(
            loaded.references.as_slice(),
            std::slice::from_ref(reference)
        );
        assert!(
            !checked
                .symbols()
                .iter()
                .any(|symbol| symbol.name == "search")
        );
    }

    #[test]
    fn retains_handler_locations_in_named_route_properties() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  draft:str = \"\"\non submit\nview\n  input \"Draft\" <-> draft submit=submit\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let route = root.lines().nth(10).unwrap();
        let column = route.rfind("submit").unwrap() + 1;
        let (handler, reference) = checked.symbol_at(Some(&app), 11, column).unwrap();

        assert_eq!(handler.name, "submit");
        assert_eq!(reference.start_column, column);
        assert!(handler.renameable);
    }

    #[test]
    fn retains_recipe_locations_across_imports() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"recipes.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n  surface #111111\nview\n  box @panel\n    text \"Panel\"\n";
        fixture.write("app.ice", root);
        fixture.write(
            "recipes.ice",
            "recipe panel for box\n  @p-4 bg-surface rounded-md\n",
        );

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let recipes = fixture.path("recipes.ice").canonicalize().unwrap();
        let line = root.lines().nth(9).unwrap();
        let column = line.find("panel").unwrap() + 1;
        let (panel, reference) = checked.symbol_at(Some(&app), 10, column).unwrap();

        assert_eq!(panel.kind, SymbolKind::Recipe);
        assert_eq!(panel.name, "panel");
        assert_eq!(panel.definition.path.as_deref(), Some(recipes.as_path()));
        assert_eq!(panel.references.as_slice(), std::slice::from_ref(reference));
        assert!(panel.renameable);
    }

    #[test]
    fn keeps_the_implicit_mount_hook_out_of_rename() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non mount\nview\n  text \"Ready\"\n";
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
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Toggle() -> bool\n  checkbox \"Toggle\" checked=false -> emit _\non changed(value)\nview\n  Toggle -> changed _\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let route = root.lines().nth(7).unwrap();
        let column = route.find("emit").unwrap() + 1;

        assert!(checked.symbol_at(Some(&app), 8, column).is_none());
    }

    #[test]
    fn counts_unicode_indentation_in_source_columns() {
        let fixture = Fixture::new();
        let indent = "\u{a0}\u{a0}";
        let root = format!(
            "app Demo\ntheme\n{indent}bg #000000\n{indent}fg #ffffff\n{indent}primary #333333\n{indent}danger #ff0000\ncomponent Card()\n{indent}text \"Card\"\nview\n{indent}Card\n"
        );
        fixture.write("app.ice", &root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), &root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let (card, reference) = checked.symbol_at(Some(&app), 10, 3).unwrap();

        assert_eq!(card.name, "Card");
        assert_eq!(reference.start_column, 3);
        assert_eq!(reference.end_column, 7);
    }

    #[test]
    fn retains_compact_canvas_routes_without_synthetic_references() {
        let fixture = Fixture::new();
        let root = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non pressed(button)\non canvas_event\nview\n  canvas\n    event mouse pressed -> pressed _\n    capture touch lost\n";
        fixture.write("app.ice", root);

        let checked = analyze_file_with_source(fixture.path("app.ice"), root).unwrap();
        let app = fixture.path("app.ice").canonicalize().unwrap();
        let route = root.lines().nth(10).unwrap();
        let column = route.rfind("pressed").unwrap() + 1;
        let (pressed, reference) = checked.symbol_at(Some(&app), 11, column).unwrap();
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
            "app Demo\n  font \"assets/Brand.ttf\"\n  window\n    icon-rgba \"assets/app.rgba\" 2 1\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
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
            "app Demo\n  font \"assets/Missing.ttf\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );

        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E192");
        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("app.ice"));
        assert!(error.message.contains("assets/Missing.ttf"));

        fixture.write(
            "app.ice",
            "app Demo\n  window child\n    icon-rgba \"assets/missing.rgba\" 2 1\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E192");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("assets/missing.rgba"));

        fixture.write("assets/wrong.rgba", "RGBA");
        fixture.write(
            "app.ice",
            "app Demo\n  window\n    icon-rgba \"assets/wrong.rgba\" 2 1\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
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
