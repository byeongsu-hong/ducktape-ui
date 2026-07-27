use crate::schema;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

struct DiagnosticReport {
    target: String,
    diagnostic: Option<Value>,
}

enum Incoming {
    Message(Value),
    ParseError,
}

struct Navigation {
    symbol: ui_lang_core::CheckedSymbol,
    family: Vec<ui_lang_core::CheckedSymbol>,
    occurrence: ui_lang_core::SourceRange,
    declarations: Vec<(ui_lang_core::SymbolKind, Option<String>, String)>,
    root_uri: String,
}

impl Navigation {
    fn renameable(&self) -> bool {
        self.symbol.renameable
            && !(self.symbol.kind == ui_lang_core::SymbolKind::Component
                && self.symbol.name.contains('.'))
            && self.family.iter().all(|symbol| symbol.renameable)
    }

    fn family_name(&self, name: &str, new_name: &str) -> String {
        if self.symbol.kind == ui_lang_core::SymbolKind::Component
            && let Some(suffix) = name.strip_prefix(&self.symbol.name)
        {
            return format!("{new_name}{suffix}");
        }
        new_name.to_owned()
    }

    fn collides(&self, new_name: &str) -> bool {
        let family = self
            .family
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>();
        self.family.iter().any(|symbol| {
            let renamed = self.family_name(&symbol.name, new_name);
            renamed != symbol.name
                && self.declarations.iter().any(|(kind, scope, name)| {
                    *kind == self.symbol.kind
                        && scope == &self.symbol.scope
                        && name == &renamed
                        && !family.contains(&name.as_str())
                })
        })
    }
}

fn same_component_family(root: &str, name: &str) -> bool {
    name == root
        || name
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn same_navigation_family(
    symbol: &ui_lang_core::CheckedSymbol,
    candidate: &ui_lang_core::CheckedSymbol,
) -> bool {
    candidate.kind == symbol.kind
        && candidate.scope == symbol.scope
        && (candidate.name == symbol.name
            || symbol.kind == ui_lang_core::SymbolKind::Component
                && same_component_family(&symbol.name, &candidate.name))
        && (symbol.kind != ui_lang_core::SymbolKind::TestTarget
            || candidate.definition == symbol.definition)
}

pub fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve(&mut BufReader::new(stdin.lock()), &mut stdout.lock()).map_err(|error| error.to_string())
}

fn serve(reader: &mut impl BufRead, writer: &mut impl Write) -> io::Result<()> {
    let mut documents = HashMap::<String, String>::new();
    let mut diagnostic_reports = HashMap::<String, DiagnosticReport>::new();
    let mut workspace_roots = Vec::<PathBuf>::new();
    let mut initialized = false;
    let mut shutdown = false;

    while let Some(incoming) = read_message(reader)? {
        let Incoming::Message(message) = incoming else {
            request_error(writer, Value::Null, -32700, "parse error")?;
            continue;
        };
        let id = message.get("id").cloned();
        let valid_id = id
            .as_ref()
            .is_none_or(|id| id.is_null() || id.is_number() || id.is_string());
        if !message.is_object()
            || message.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || !message.get("method").is_some_and(Value::is_string)
            || !message
                .get("params")
                .is_none_or(|params| params.is_array() || params.is_object())
            || !valid_id
        {
            let id = id
                .filter(|id| id.is_null() || id.is_number() || id.is_string())
                .unwrap_or(Value::Null);
            request_error(writer, id, -32600, "invalid request")?;
            continue;
        }
        let method = message["method"].as_str().expect("validated method");
        if let Some(id) = &id
            && matches!(
                method,
                "initialized"
                    | "$/cancelRequest"
                    | "exit"
                    | "textDocument/didOpen"
                    | "textDocument/didChange"
                    | "textDocument/didClose"
            )
        {
            request_error(
                writer,
                id.clone(),
                -32600,
                "notification method sent as request",
            )?;
            continue;
        }
        if !initialized && method != "initialize" && method != "exit" {
            if let Some(id) = id {
                request_error(writer, id, -32002, "server is not initialized")?;
            }
            continue;
        }
        if shutdown && method != "exit" {
            if let Some(id) = id {
                request_error(writer, id, -32600, "server is shutting down")?;
            }
            continue;
        }
        match method {
            "initialize" if initialized => {
                if let Some(id) = id {
                    request_error(writer, id, -32600, "initialize may only be sent once")?;
                }
            }
            "initialize" => {
                let Some(id) = id else {
                    continue;
                };
                let Some(params) = message.get("params").filter(|params| params.is_object()) else {
                    invalid_params(writer, id, "initialize params must be an object")?;
                    continue;
                };
                workspace_roots = initialize_roots(params);
                respond(
                    writer,
                    id,
                    json!({
                        "capabilities": {
                            "positionEncoding": "utf-16",
                            "textDocumentSync": { "openClose": true, "change": 1 },
                            "documentFormattingProvider": true,
                            "completionProvider": { "resolveProvider": false },
                            "definitionProvider": true,
                            "renameProvider": { "prepareProvider": true },
                        },
                        "serverInfo": {
                            "name": "ice-lsp",
                            "version": env!("CARGO_PKG_VERSION"),
                        },
                    }),
                )?;
                initialized = true;
            }
            "shutdown" => {
                if let Some(id) = id {
                    shutdown = true;
                    respond(writer, id, Value::Null)?;
                }
            }
            "exit" => {
                if shutdown {
                    break;
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "LSP exit received before shutdown",
                ));
            }
            "textDocument/didOpen" => {
                let params = &message["params"]["textDocument"];
                if let (Some(uri), Some(text)) = (params["uri"].as_str(), params["text"].as_str()) {
                    documents.insert(uri.to_owned(), text.to_owned());
                    reanalyze_open_roots(writer, &documents, &mut diagnostic_reports)?;
                }
            }
            "textDocument/didChange" => {
                let uri = message["params"]["textDocument"]["uri"].as_str();
                let text = message["params"]["contentChanges"]
                    .as_array()
                    .and_then(|changes| changes.last())
                    .and_then(|change| change["text"].as_str());
                if let (Some(uri), Some(text)) = (uri, text) {
                    documents.insert(uri.to_owned(), text.to_owned());
                    reanalyze_open_roots(writer, &documents, &mut diagnostic_reports)?;
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = message["params"]["textDocument"]["uri"].as_str() {
                    documents.remove(uri);
                    reanalyze_open_roots(writer, &documents, &mut diagnostic_reports)?;
                }
            }
            "textDocument/formatting" => {
                if let Some(id) = id {
                    if valid_document_formatting_params(&message["params"]) {
                        let uri = message["params"]["textDocument"]["uri"].as_str();
                        match uri.and_then(|uri| documents.get(uri)) {
                            Some(source) => {
                                let formatted = ui_lang_core::format_fragment(source);
                                let edits = if formatted == *source {
                                    Vec::new()
                                } else {
                                    vec![json!({
                                        "range": whole_document_range(source),
                                        "newText": formatted,
                                    })]
                                };
                                respond(writer, id, Value::Array(edits))?;
                            }
                            None => invalid_params(writer, id, "document is not open")?,
                        }
                    } else {
                        invalid_params(writer, id, "invalid document formatting params")?;
                    }
                }
            }
            "textDocument/completion" => {
                if let Some(id) = id {
                    if valid_text_document_position_params(&message["params"]) {
                        respond(writer, id, Value::Array(schema::completion_items()))?;
                    } else {
                        invalid_params(writer, id, "invalid text document position")?;
                    }
                }
            }
            "textDocument/definition" => {
                if let Some(id) = id {
                    if valid_text_document_position_params(&message["params"]) {
                        let result =
                            navigation_at(&documents, &workspace_roots, &message["params"])
                                .and_then(|navigation| {
                                    location(
                                        &documents,
                                        &navigation.symbol.definition,
                                        &navigation.root_uri,
                                    )
                                });
                        respond(writer, id, result.unwrap_or(Value::Null))?;
                    } else {
                        invalid_params(writer, id, "invalid text document position")?;
                    }
                }
            }
            "textDocument/prepareRename" => {
                if let Some(id) = id {
                    if valid_text_document_position_params(&message["params"]) {
                        let result =
                            navigation_at(&documents, &workspace_roots, &message["params"])
                                .filter(Navigation::renameable)
                                .and_then(|navigation| {
                                    let (_, source) = range_document(
                                        &documents,
                                        &navigation.occurrence,
                                        &navigation.root_uri,
                                    )?;
                                    source_range(&source, &navigation.occurrence).map(|range| {
                                        json!({
                                            "range": range,
                                            "placeholder": navigation.symbol.name,
                                        })
                                    })
                                });
                        respond(writer, id, result.unwrap_or(Value::Null))?;
                    } else {
                        invalid_params(writer, id, "invalid text document position")?;
                    }
                }
            }
            "textDocument/rename" => {
                if let Some(id) = id {
                    let new_name = message["params"]["newName"].as_str();
                    match (
                        navigation_at(&documents, &workspace_roots, &message["params"]),
                        new_name,
                    ) {
                        (Some(navigation), Some(new_name))
                            if navigation.renameable()
                                && navigation.symbol.kind.accepts(new_name)
                                && !(navigation.symbol.kind
                                    == ui_lang_core::SymbolKind::Component
                                    && new_name.contains('.'))
                                && !navigation.collides(new_name) =>
                        {
                            match workspace_edit(&documents, &navigation, new_name) {
                                Some(edit) => respond(writer, id, edit)?,
                                None => invalid_params(
                                    writer,
                                    id,
                                    "cannot read every file required for rename",
                                )?,
                            }
                        }
                        (Some(_), Some(_)) => invalid_params(
                            writer,
                            id,
                            "rename is incomplete, invalid, or collides with a declaration",
                        )?,
                        _ => invalid_params(writer, id, "no renameable symbol at position")?,
                    }
                }
            }
            "initialized" | "$/cancelRequest" => {}
            _ if id.is_some() => {
                request_error(writer, id.unwrap(), -32601, "method not found")?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn reanalyze_open_roots(
    writer: &mut impl Write,
    documents: &HashMap<String, String>,
    reports: &mut HashMap<String, DiagnosticReport>,
) -> io::Result<()> {
    let overlays = source_overlays(documents);
    // ponytail: Add a dependency graph only if profiling shows open-root scale matters.
    let next = documents
        .iter()
        .filter(|(_, source)| ui_lang_core::source_is_app(source))
        .map(|(uri, source)| (uri.clone(), analyze_diagnostics(uri, source, &overlays)))
        .collect::<HashMap<_, _>>();
    let targets = reports
        .values()
        .chain(next.values())
        .map(|report| report.target.clone())
        .collect::<BTreeSet<_>>();
    *reports = next;
    for target in targets {
        publish_aggregated(writer, reports, &target)?;
    }
    Ok(())
}

fn source_overlays(documents: &HashMap<String, String>) -> HashMap<PathBuf, String> {
    documents
        .iter()
        .filter_map(|(uri, source)| {
            file_uri_path(uri).map(|path| {
                let path = canonical_path(&path).unwrap_or(path);
                (path, source.clone())
            })
        })
        .collect()
}

fn analyze_diagnostics(
    uri: &str,
    source: &str,
    overlays: &HashMap<PathBuf, String>,
) -> DiagnosticReport {
    let analysis = file_uri_path(uri).map_or_else(
        || ui_lang_core::analyze(source),
        |path| ui_lang_core::analyze_file_with_overlays(path, overlays),
    );
    match analysis {
        Ok(_) => DiagnosticReport {
            target: uri.to_owned(),
            diagnostic: None,
        },
        Err(error) => {
            let (target, target_source) = diagnostic_target(uri, source, overlays, &error);
            let mut message = error.message;
            if let Some(hint) = error.hint {
                message.push_str("\nhint: ");
                message.push_str(&hint);
            }
            DiagnosticReport {
                target,
                diagnostic: Some(json!({
                    "range": diagnostic_range(&target_source, error.line, error.column),
                    "severity": 1,
                    "code": error.code,
                    "source": "ice",
                    "message": message,
                })),
            }
        }
    }
}

fn initialize_roots(params: &Value) -> Vec<PathBuf> {
    let mut roots = params["workspaceFolders"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|folder| folder["uri"].as_str())
        .filter_map(file_uri_path)
        .collect::<Vec<_>>();
    if roots.is_empty()
        && let Some(root) = params["rootUri"].as_str().and_then(file_uri_path)
    {
        roots.push(root);
    }
    if roots.is_empty()
        && let Some(root) = params["rootPath"].as_str()
    {
        roots.push(PathBuf::from(root));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn valid_text_document_position_params(params: &Value) -> bool {
    let position = &params["position"];
    params.is_object()
        && params["textDocument"]["uri"].is_string()
        && position["line"]
            .as_u64()
            .is_some_and(|value| value <= i32::MAX as u64)
        && position["character"]
            .as_u64()
            .is_some_and(|value| value <= i32::MAX as u64)
}

fn valid_document_formatting_params(params: &Value) -> bool {
    let options = &params["options"];
    params.is_object()
        && params["textDocument"]["uri"].is_string()
        && options.is_object()
        && options["tabSize"]
            .as_u64()
            .is_some_and(|value| value <= i32::MAX as u64)
        && options["insertSpaces"].is_boolean()
}

fn navigation_at(
    documents: &HashMap<String, String>,
    workspace_roots: &[PathBuf],
    params: &Value,
) -> Option<Navigation> {
    let uri = params["textDocument"]["uri"].as_str()?;
    let source = documents.get(uri)?;
    let overlays = source_overlays(documents);
    let line = usize::try_from(params["position"]["line"].as_u64()?).ok()?;
    let character = usize::try_from(params["position"]["character"].as_u64()?).ok()?;
    let column = utf16_column(source_line(source, line)?, character)?;
    let query_path = file_uri_path(uri).map(|path| canonical_path(&path).unwrap_or(path));

    let mut roots = Vec::<(String, String)>::new();
    let mut workspace_complete = !workspace_roots.is_empty();
    for workspace_root in workspace_roots {
        let files = match crate::ice_files(workspace_root) {
            Ok(files) => files,
            Err(_) => {
                workspace_complete = false;
                continue;
            }
        };
        for path in files {
            let open = documents.iter().find(|(open_uri, _)| {
                file_uri_path(open_uri).is_some_and(|open| same_file(&open, &path))
            });
            let (root_uri, root_source) = match open {
                Some((open_uri, source)) => ((*open_uri).clone(), (*source).clone()),
                None => match fs::read_to_string(&path) {
                    Ok(source) => (file_path_uri(&path), source),
                    Err(_) => {
                        workspace_complete = false;
                        continue;
                    }
                },
            };
            if ui_lang_core::source_is_app(&root_source) {
                roots.push((root_uri, root_source));
            }
        }
    }
    for (root_uri, root_source) in documents {
        if ui_lang_core::source_is_app(root_source)
            && !roots.iter().any(|(candidate, _)| {
                match (file_uri_path(candidate), file_uri_path(root_uri)) {
                    (Some(candidate), Some(root)) => same_file(&candidate, &root),
                    _ => candidate == root_uri,
                }
            })
        {
            roots.push((root_uri.clone(), root_source.clone()));
        }
    }
    roots.sort_by(|(left, _), (right, _)| {
        (left != uri)
            .cmp(&(right != uri))
            .then_with(|| left.cmp(right))
    });
    let mut analyzed = Vec::new();
    let mut incomplete = false;
    for (root_uri, root_source) in &roots {
        let checked = match file_uri_path(root_uri) {
            Some(path) => match ui_lang_core::analyze_file_with_overlays(path, &overlays) {
                Ok(checked) => checked,
                Err(_) => {
                    incomplete = true;
                    continue;
                }
            },
            None if root_uri == uri => match ui_lang_core::analyze(root_source) {
                Ok(checked) => checked,
                Err(_) => {
                    incomplete = true;
                    continue;
                }
            },
            None => {
                incomplete = true;
                continue;
            }
        };
        analyzed.push((root_uri, checked));
    }

    let mut navigation = analyzed.iter().find_map(|(root_uri, checked)| {
        let path = query_path.as_deref();
        let (symbol, occurrence) = checked.symbol_at(path, line + 1, column)?;
        let family = checked
            .symbols()
            .iter()
            .filter(|candidate| same_navigation_family(symbol, candidate))
            .cloned()
            .collect();
        Some(Navigation {
            symbol: symbol.clone(),
            family,
            occurrence: occurrence.clone(),
            declarations: checked
                .symbols()
                .iter()
                .map(|symbol| (symbol.kind, symbol.scope.clone(), symbol.name.clone()))
                .collect(),
            root_uri: (*root_uri).clone(),
        })
    })?;

    let selected_root_in_workspace = file_uri_path(&navigation.root_uri)
        .and_then(|root| canonical_path(&root))
        .is_some_and(|root| {
            workspace_roots.iter().any(|workspace| {
                workspace
                    .canonicalize()
                    .is_ok_and(|workspace| root.starts_with(workspace))
            })
        });
    if navigation
        .symbol
        .definition
        .path
        .as_deref()
        .is_some_and(|definition| {
            !file_uri_path(&navigation.root_uri).is_some_and(|root| same_file(&root, definition))
        })
        && (!workspace_complete || !selected_root_in_workspace)
    {
        incomplete = true;
    }

    for (root_uri, checked) in analyzed {
        if navigation.symbol.definition.path.is_none() && *root_uri != navigation.root_uri {
            continue;
        }
        let Some(symbol) = checked.symbols().iter().find(|symbol| {
            symbol.kind == navigation.symbol.kind
                && symbol.scope == navigation.symbol.scope
                && symbol.name == navigation.symbol.name
                && symbol.definition == navigation.symbol.definition
        }) else {
            continue;
        };
        navigation.symbol.renameable &= symbol.renameable;
        for reference in &symbol.references {
            if !navigation.symbol.references.contains(reference) {
                navigation.symbol.references.push(reference.clone());
            }
        }
        for candidate in checked
            .symbols()
            .iter()
            .filter(|candidate| same_navigation_family(&navigation.symbol, candidate))
        {
            if let Some(existing) = navigation.family.iter_mut().find(|existing| {
                existing.name == candidate.name && existing.definition == candidate.definition
            }) {
                existing.renameable &= candidate.renameable;
                for reference in &candidate.references {
                    if !existing.references.contains(reference) {
                        existing.references.push(reference.clone());
                    }
                }
            } else {
                navigation.family.push(candidate.clone());
            }
        }
        if navigation.symbol.kind != ui_lang_core::SymbolKind::TestTarget {
            for declaration in checked
                .symbols()
                .iter()
                .map(|symbol| (symbol.kind, symbol.scope.clone(), symbol.name.clone()))
            {
                if !navigation.declarations.contains(&declaration) {
                    navigation.declarations.push(declaration);
                }
            }
        }
    }
    if incomplete {
        navigation.symbol.renameable = false;
        for symbol in &mut navigation.family {
            symbol.renameable = false;
        }
    }
    if navigation.family.iter().any(|symbol| {
        std::iter::once(&symbol.definition)
            .chain(&symbol.references)
            .any(|range| range_document(documents, range, &navigation.root_uri).is_none())
    }) {
        navigation.symbol.renameable = false;
    }
    Some(navigation)
}

fn utf16_column(line: &str, target: usize) -> Option<usize> {
    let mut utf16 = 0;
    let mut column = 1;
    for ch in line.chars() {
        if utf16 == target {
            return Some(column);
        }
        utf16 += ch.len_utf16();
        if utf16 > target {
            return None;
        }
        column += 1;
    }
    (utf16 == target).then_some(column)
}

fn source_line(source: &str, line: usize) -> Option<&str> {
    source
        .split('\n')
        .nth(line)
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
}

fn source_range(source: &str, range: &ui_lang_core::SourceRange) -> Option<Value> {
    let line = source_line(source, range.line.checked_sub(1)?)?;
    let start_column = range.start_column.checked_sub(1)?;
    let end_column = range.end_column.checked_sub(1)?;
    if start_column > end_column || end_column > line.chars().count() {
        return None;
    }
    let start = line
        .chars()
        .take(start_column)
        .map(char::len_utf16)
        .sum::<usize>();
    let end = line
        .chars()
        .take(end_column)
        .map(char::len_utf16)
        .sum::<usize>();
    Some(json!({
        "start": { "line": range.line - 1, "character": start },
        "end": { "line": range.line - 1, "character": end },
    }))
}

fn range_document(
    documents: &HashMap<String, String>,
    range: &ui_lang_core::SourceRange,
    fallback_uri: &str,
) -> Option<(String, String)> {
    let Some(path) = range.path.as_deref() else {
        return documents
            .get(fallback_uri)
            .map(|source| (fallback_uri.to_owned(), source.clone()));
    };
    let open_uri = documents
        .keys()
        .find(|uri| file_uri_path(uri).is_some_and(|open| same_file(&open, path)))
        .cloned();
    if let Some(uri) = open_uri {
        let source = documents.get(&uri)?.clone();
        return Some((uri, source));
    }
    Some((file_path_uri(path), fs::read_to_string(path).ok()?))
}

fn location(
    documents: &HashMap<String, String>,
    range: &ui_lang_core::SourceRange,
    fallback_uri: &str,
) -> Option<Value> {
    let (uri, source) = range_document(documents, range, fallback_uri)?;
    Some(json!({
        "uri": uri,
        "range": source_range(&source, range)?,
    }))
}

fn workspace_edit(
    documents: &HashMap<String, String>,
    navigation: &Navigation,
    new_name: &str,
) -> Option<Value> {
    let mut changes = BTreeMap::<String, Vec<Value>>::new();
    for symbol in &navigation.family {
        let renamed = navigation.family_name(&symbol.name, new_name);
        for range in std::iter::once(&symbol.definition).chain(&symbol.references) {
            let (uri, source) = range_document(documents, range, &navigation.root_uri)?;
            let start = range.start_column.checked_sub(1)?;
            let length = range.end_column.checked_sub(range.start_column)?;
            let line = source_line(&source, range.line.checked_sub(1)?)?;
            if !line
                .chars()
                .skip(start)
                .take(length)
                .eq(symbol.name.chars())
            {
                return None;
            }
            changes.entry(uri).or_default().push(json!({
                "range": source_range(&source, range)?,
                "newText": renamed,
            }));
        }
    }
    Some(json!({ "changes": changes }))
}

fn publish_aggregated(
    writer: &mut impl Write,
    reports: &HashMap<String, DiagnosticReport>,
    uri: &str,
) -> io::Result<()> {
    let diagnostics = reports
        .values()
        .filter(|report| report.target == uri)
        .filter_map(|report| report.diagnostic.clone())
        .collect::<Vec<_>>();
    write_message(
        writer,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "diagnostics": diagnostics },
        }),
    )
}

fn diagnostic_target(
    root_uri: &str,
    root_source: &str,
    overlays: &HashMap<PathBuf, String>,
    error: &ui_lang_core::Error,
) -> (String, String) {
    let Some(error_path) = error.path.as_deref().map(Path::new) else {
        return (root_uri.to_owned(), root_source.to_owned());
    };
    if file_uri_path(root_uri).is_some_and(|root_path| same_file(&root_path, error_path)) {
        return (root_uri.to_owned(), root_source.to_owned());
    }
    if let Some(source) = overlays.get(error_path) {
        return (file_path_uri(error_path), source.clone());
    }
    match fs::read_to_string(error_path) {
        Ok(source) => (file_path_uri(error_path), source),
        Err(_) => (root_uri.to_owned(), root_source.to_owned()),
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn canonical_path(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok().or_else(|| {
        let parent = path.parent()?.canonicalize().ok()?;
        Some(parent.join(path.file_name()?))
    })
}

fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    let local_path = if path.eq_ignore_ascii_case("localhost") {
        Some("/".to_owned())
    } else {
        path.split_once('/').and_then(|(authority, path)| {
            authority
                .eq_ignore_ascii_case("localhost")
                .then(|| format!("/{path}"))
        })
    };
    let path = local_path.as_deref().unwrap_or(path);
    #[cfg(windows)]
    let path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("//{path}")
    };
    #[cfg(not(windows))]
    if !path.starts_with('/') {
        return None;
    }
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex(*bytes.get(index + 1)?)?;
            let low = hex(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        Some(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
    }
    #[cfg(not(unix))]
    {
        let decoded = String::from_utf8(decoded).ok()?;
        #[cfg(windows)]
        let decoded = decoded
            .strip_prefix('/')
            .filter(|path| path.as_bytes().get(1) == Some(&b':'))
            .unwrap_or(&decoded);
        Some(PathBuf::from(decoded))
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn file_path_uri(path: &Path) -> String {
    #[cfg(windows)]
    let path = {
        let path = path.to_string_lossy().replace('\\', "/");
        if path
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("//?/UNC/"))
        {
            format!("//{}", &path[8..])
        } else if let Some(path) = path.strip_prefix("//?/") {
            path.to_owned()
        } else {
            path
        }
    };
    #[cfg(windows)]
    let mut uri = if path.starts_with("//") {
        String::from("file:")
    } else if path.starts_with('/') {
        String::from("file://")
    } else {
        String::from("file:///")
    };
    #[cfg(not(windows))]
    let mut uri = String::from("file://");
    #[cfg(windows)]
    let bytes = path.as_bytes();
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;

        path.as_os_str().as_bytes()
    };
    #[cfg(all(not(windows), not(unix)))]
    let path = path.to_string_lossy();
    #[cfg(all(not(windows), not(unix)))]
    let bytes = path.as_bytes();
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'.' | b'-' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

fn diagnostic_range(source: &str, one_based_line: usize, one_based_column: usize) -> Value {
    let line = one_based_line
        .saturating_sub(1)
        .min(source.split('\n').count().saturating_sub(1));
    let text = source_line(source, line).unwrap_or("");
    let character = one_based_column.saturating_sub(1).min(text.chars().count());
    let start = text
        .chars()
        .take(character)
        .map(char::len_utf16)
        .sum::<usize>();
    let end = start
        + text
            .chars()
            .nth(character)
            .map(char::len_utf16)
            .unwrap_or(0);
    json!({
        "start": { "line": line, "character": start },
        "end": { "line": line, "character": end },
    })
}

fn whole_document_range(source: &str) -> Value {
    let line = source.bytes().filter(|byte| *byte == b'\n').count();
    let character = source
        .rsplit_once('\n')
        .map_or(source, |(_, tail)| tail)
        .encode_utf16()
        .count();
    json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": line, "character": character },
    })
}

fn respond(writer: &mut impl Write, id: Value, result: Value) -> io::Result<()> {
    write_message(
        writer,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

fn invalid_params(writer: &mut impl Write, id: Value, message: &str) -> io::Result<()> {
    request_error(writer, id, -32602, message)
}

fn request_error(writer: &mut impl Write, id: Value, code: i64, message: &str) -> io::Result<()> {
    write_message(
        writer,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message },
        }),
    )
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Incoming>> {
    let mut length = None;
    let mut started = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return if started {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete LSP headers",
                ))
            } else {
                Ok(None)
            };
        }
        started = true;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            length = Some(value.trim().parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
            })?);
        }
    }

    let length = length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = Vec::new();
    body.try_reserve_exact(length)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Content-Length is too large"))?;
    body.resize(length, 0);
    reader.read_exact(&mut body)?;
    Ok(Some(
        serde_json::from_slice(&body).map_or(Incoming::ParseError, Incoming::Message),
    ))
}

fn write_message(writer: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::{
        Navigation, diagnostic_range, file_path_uri, file_uri_path, navigation_at, read_message,
        serve, source_range, whole_document_range, workspace_edit,
    };
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use std::fs;
    use std::io::{BufReader, Cursor};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const APP_WITH_PART: &str = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Broken\n";

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("cargo-ice-lsp-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, source: &str) {
            fs::write(self.0.join(relative), source).unwrap();
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

    fn frame(message: &Value, output: &mut Vec<u8>) {
        let body = serde_json::to_vec(message).unwrap();
        output.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
        output.extend_from_slice(&body);
    }

    fn run(messages: &[Value]) -> std::io::Result<Vec<Value>> {
        let mut input = Vec::new();
        for message in messages {
            frame(message, &mut input);
        }

        let mut output = Vec::new();
        serve(&mut BufReader::new(Cursor::new(input)), &mut output)?;

        let mut reader = BufReader::new(Cursor::new(output));
        let mut messages = Vec::new();
        while let Some(incoming) = read_message(&mut reader)? {
            match incoming {
                super::Incoming::Message(message) => messages.push(message),
                super::Incoming::ParseError => unreachable!("server emitted invalid JSON"),
            }
        }
        Ok(messages)
    }

    fn response(messages: &[Value], id: impl Into<Value>) -> &Value {
        let id = id.into();
        messages.iter().find(|message| message["id"] == id).unwrap()
    }

    #[test]
    fn initializes_and_shuts_down_with_honest_capabilities() {
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let capabilities = &response(&messages, 1)["result"]["capabilities"];
        assert_eq!(capabilities["positionEncoding"], "utf-16");
        assert_eq!(capabilities["textDocumentSync"]["change"], 1);
        assert_eq!(capabilities["documentFormattingProvider"], true);
        assert_eq!(capabilities["completionProvider"]["resolveProvider"], false);
        assert_eq!(capabilities["definitionProvider"], true);
        assert_eq!(capabilities["renameProvider"]["prepareProvider"], true);
        assert_eq!(response(&messages, 2)["result"], Value::Null);
    }

    #[test]
    fn enforces_the_initialize_boundary() {
        let uri = "file:///tmp/early.ice";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": "early", "method": "textDocument/completion" }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": "app Early\nview\n  wat\n" } },
            }),
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "id": "again", "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "method": "shutdown" }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/formatting",
                "params": {
                    "textDocument": { "uri": uri },
                    "options": { "tabSize": 2, "insertSpaces": true },
                },
            }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, "early")["error"]["code"], -32002);
        assert_eq!(response(&messages, "again")["error"]["code"], -32600);
        assert_eq!(response(&messages, 2)["error"]["code"], -32602);
        assert!(
            !messages
                .iter()
                .any(|message| message["method"] == "textDocument/publishDiagnostics")
        );
    }

    #[test]
    fn reports_malformed_json_and_continues() {
        let mut input = Vec::new();
        frame(
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            &mut input,
        );
        input.extend_from_slice(b"Content-Length: 1\r\n\r\n{");
        frame(
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            &mut input,
        );
        frame(&json!({ "jsonrpc": "2.0", "method": "exit" }), &mut input);

        let mut output = Vec::new();
        serve(&mut BufReader::new(Cursor::new(input)), &mut output).unwrap();

        let mut reader = BufReader::new(Cursor::new(output));
        let mut messages = Vec::new();
        while let Some(incoming) = read_message(&mut reader).unwrap() {
            match incoming {
                super::Incoming::Message(message) => messages.push(message),
                super::Incoming::ParseError => unreachable!("server emitted invalid JSON"),
            }
        }
        assert_eq!(
            response(&messages, 1)["result"]["serverInfo"]["name"],
            "ice-lsp"
        );
        assert_eq!(response(&messages, 2)["result"], Value::Null);
        assert!(
            messages.iter().any(|message| {
                message["id"] == Value::Null && message["error"]["code"] == -32700
            })
        );
    }

    #[test]
    fn rejects_unallocatable_content_lengths() {
        let input = format!("Content-Length: {}\r\n\r\n", usize::MAX);
        let result = read_message(&mut BufReader::new(Cursor::new(input)));

        let Err(error) = result else {
            panic!("unallocatable content length must fail");
        };
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_invalid_json_rpc_messages_and_continues() {
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": "missing-method" }),
            json!([]),
            json!({ "jsonrpc": "1.0", "id": "wrong-version", "method": "initialize" }),
            json!({ "jsonrpc": "2.0", "id": "missing-init-params", "method": "initialize" }),
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "id": "bad-params",
                "method": "textDocument/completion",
                "params": true,
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        for id in ["missing-method", "wrong-version", "bad-params"] {
            assert_eq!(response(&messages, id)["error"]["code"], -32600);
        }
        assert_eq!(
            response(&messages, "missing-init-params")["error"]["code"],
            -32602
        );
        assert!(
            messages.iter().any(|message| {
                message["id"] == Value::Null && message["error"]["code"] == -32600
            })
        );
        assert_eq!(
            response(&messages, 1)["result"]["serverInfo"]["name"],
            "ice-lsp"
        );
        assert_eq!(response(&messages, 2)["result"], Value::Null);
    }

    #[test]
    fn rejects_notification_methods_sent_as_requests() {
        let uri = "file:///tmp/request.ice";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "id": "open-request",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": "app Demo\nview\n  wat\n" } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "id": "exit-request", "method": "exit" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        for id in ["open-request", "exit-request"] {
            assert_eq!(response(&messages, id)["error"]["code"], -32600);
        }
        assert!(
            !messages
                .iter()
                .any(|message| message["method"] == "textDocument/publishDiagnostics")
        );
        assert_eq!(response(&messages, 2)["result"], Value::Null);
    }

    #[test]
    fn rejects_invalid_text_document_position_params() {
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": "file:///tmp/demo.ice" } },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        for id in [2, 3] {
            assert_eq!(response(&messages, id)["error"]["code"], -32602);
        }
        assert_eq!(response(&messages, 4)["result"], Value::Null);
    }

    #[test]
    fn rejects_invalid_document_formatting_params() {
        let uri = "file:///tmp/demo.ice";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": "app Demo\nview\n  text \"Hi\"\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/formatting",
                "params": { "textDocument": { "uri": uri } },
            }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["error"]["code"], -32602);
        assert_eq!(response(&messages, 3)["result"], Value::Null);
    }

    #[test]
    fn publishes_diagnostics_for_open_and_change() {
        let uri = "file:///tmp/demo.ice";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  wat\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri },
                    "contentChanges": [{ "text": "app Demo\ntheme\n    bg #000000\n    fg #ffffff\n    primary #333333\n    danger #ff0000\nview\n    text \"Hi\"\n" }],
                },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let diagnostics = messages
            .iter()
            .filter(|message| message["method"] == "textDocument/publishDiagnostics")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0]["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            diagnostics[0]["params"]["diagnostics"][0]["range"],
            json!({
                "start": { "line": 7, "character": 0 },
                "end": { "line": 7, "character": 1 },
            })
        );
        assert!(
            diagnostics[1]["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn publishes_imported_errors_at_the_imported_file() {
        let fixture = Fixture::new();
        fixture.write("app.ice", "app Saved\nview\n  text \"Saved\"\n");
        fixture.write("part.ice", "component Broken()\n  wat\n");
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let overlay = "app Overlay\nuse \"part.ice\"\nview\n  Broken\n";

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": overlay } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .find(|message| message["method"] == "textDocument/publishDiagnostics")
            .unwrap();
        assert_eq!(published["params"]["uri"], part_uri);
        assert_eq!(published["params"]["diagnostics"][0]["code"], "E064");
        assert_eq!(
            published["params"]["diagnostics"][0]["range"],
            json!({
                "start": { "line": 1, "character": 0 },
                "end": { "line": 1, "character": 1 },
            })
        );
    }

    #[test]
    fn analyzes_new_unsaved_root_and_import_files() {
        let fixture = Fixture::new();
        let root_uri = file_path_uri(&fixture.path("new.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": "component Broken()\n  wat\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": "app New\nuse \"part.ice\"\nview\n  Broken\n" } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .find(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == part_uri
            })
            .unwrap();
        assert_eq!(published["params"]["diagnostics"][0]["code"], "E064");
    }

    #[test]
    fn unsaved_import_errors_recover_on_edit() {
        let fixture = Fixture::new();
        fixture.write("app.ice", APP_WITH_PART);
        fixture.write("part.ice", "component Broken()\n  text \"Saved\"\n");
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": APP_WITH_PART } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": "component Broken()\n  wat\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": part_uri },
                    "contentChanges": [{ "text": "component Broken()\n  text \"Unsaved\"\n" }],
                },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .filter(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == part_uri
            })
            .collect::<Vec<_>>();
        assert_eq!(published.len(), 2);
        assert_eq!(published[0]["params"]["diagnostics"][0]["code"], "E064");
        assert!(
            published[1]["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn closing_an_import_overlay_falls_back_to_disk() {
        let fixture = Fixture::new();
        fixture.write("app.ice", APP_WITH_PART);
        fixture.write("part.ice", "component Broken()\n  wat\n");
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": APP_WITH_PART } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": "component Broken()\n  text \"Unsaved\"\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": part_uri } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .filter(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == part_uri
            })
            .collect::<Vec<_>>();
        let counts = published
            .iter()
            .map(|message| message["params"]["diagnostics"].as_array().unwrap().len())
            .collect::<Vec<_>>();
        assert_eq!(counts, [1, 0, 1]);
        assert_eq!(published[0]["params"]["diagnostics"][0]["code"], "E064");
        assert_eq!(published[2]["params"]["diagnostics"][0]["code"], "E064");
    }

    #[test]
    fn fragments_keep_root_owned_diagnostics_aggregated() {
        let fixture = Fixture::new();
        fixture.write("one.ice", "app One\nview\n  text \"Saved\"\n");
        fixture.write("two.ice", "app Two\nview\n  text \"Saved\"\n");
        fixture.write("part.ice", "component Broken()\n  wat\n");
        let one_uri = file_path_uri(&fixture.path("one.ice"));
        let two_uri = file_path_uri(&fixture.path("two.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let one = "app One\nuse \"part.ice\"\nview\n  Broken\n";
        let two = "app Two\nuse \"part.ice\"\nview\n  Broken\n";
        let fragment = "component Broken()\n  text \"Open buffer\"\n";

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": one_uri, "text": one } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": two_uri, "text": two } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": fragment } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": part_uri } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": one_uri } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": two_uri } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .filter(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == part_uri
            })
            .collect::<Vec<_>>();
        let counts = published
            .iter()
            .map(|message| message["params"]["diagnostics"].as_array().unwrap().len())
            .collect::<Vec<_>>();
        assert_eq!(counts, [1, 2, 0, 2, 1, 0]);
        assert!(published.iter().all(|message| {
            message["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .all(|diagnostic| diagnostic["code"] == "E064")
        }));
    }

    #[test]
    fn formats_open_documents_and_completes_from_the_schema() {
        let uri = "file:///tmp/demo.ice";
        let source = "app Demo\ntheme\n    bg #000000\n    fg #ffffff\n    primary #333333\n    danger #ff0000\nview\n    text \"😀\"";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": source } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/formatting",
                "params": {
                    "textDocument": { "uri": uri },
                    "options": { "tabSize": 2, "insertSpaces": true },
                },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 7, "character": 0 } },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert!(
            response(&messages, 2)["result"][0]["newText"]
                .as_str()
                .unwrap()
                .contains("\n  text \"😀\"\n")
        );
        assert_eq!(
            response(&messages, 2)["result"][0]["range"],
            json!({
                "start": { "line": 0, "character": 0 },
                "end": { "line": 7, "character": 13 },
            })
        );
        let component = response(&messages, 3)["result"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["label"] == "component")
            .unwrap();
        assert_eq!(component["insertText"], "component ${1:Name}(${2})\n  $0");
        assert_eq!(component["insertTextFormat"], 2);
    }

    #[test]
    fn completes_first_class_test_mode_from_the_schema() {
        let uri = "file:///tmp/tests.ice";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 0, "character": 0 } },
            }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let items = response(&messages, 2)["result"].as_array().unwrap();
        let completion = |label| {
            items
                .iter()
                .find(|item| item["label"] == label)
                .unwrap_or_else(|| panic!("missing `{label}` completion"))
        };
        assert_eq!(completion("test")["insertText"], "test ${1:name}\n  $0");
        assert_eq!(
            completion("target")["insertText"],
            "target ${1:name} = #${2:id}"
        );
        assert_eq!(completion("expect")["insertText"], "expect ${1:condition}");
        for label in [
            "preset", "viewport", "timeout", "mount", "click", "hover", "press", "release", "type",
            "key", "resize", "dispatch", "~=",
        ] {
            assert_eq!(completion(label)["insertTextFormat"], 2, "{label}");
        }
        for label in crate::schema::document()["core"]["testMode"]["targets"]["directIdNodes"]
            .as_array()
            .unwrap()
        {
            assert_eq!(
                completion(label.as_str().unwrap())["insertTextFormat"],
                2,
                "{label}"
            );
        }
    }

    #[test]
    fn defines_and_renames_handlers_referenced_by_tests() {
        let uri = "file:///tmp/test-navigation.ice";
        let source = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  count = 0\non increment\n  count = count + 1\nview\n  text count\ntest dispatches\n  dispatch increment\n";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": source } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 13, "character": 12 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 13, "character": 12 }, "newName": "bump" },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["result"]["uri"], uri);
        assert_eq!(
            response(&messages, 2)["result"]["range"]["start"],
            json!({ "line": 8, "character": 3 })
        );
        let edits = response(&messages, 3)["result"]["changes"][uri]
            .as_array()
            .unwrap();
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|edit| edit["newText"] == "bump"));
        assert_eq!(
            edits
                .iter()
                .map(|edit| edit["range"]["start"]["line"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [8, 13]
        );
    }

    #[test]
    fn test_target_rename_stays_inside_one_test_scope() {
        let uri = "file:///tmp/test-target-navigation.ice";
        let source = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non clicked\nview\n  col #root\n    button \"Go\" #action -> clicked\ntest first\n  target root = #root\n  target action = root/action\n  expect root.width == root.height\n  click action\ntest second\n  target root = #root\n  expect root.visible\n";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": source } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 13, "character": 10 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 13, "character": 10 }, "newName": "surface" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 13, "character": 10 }, "newName": "action" },
            }),
            json!({ "jsonrpc": "2.0", "id": 5, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(
            response(&messages, 2)["result"]["range"]["start"],
            json!({ "line": 11, "character": 9 })
        );
        let edits = response(&messages, 3)["result"]["changes"][uri]
            .as_array()
            .unwrap();
        assert_eq!(edits.len(), 4);
        assert!(edits.iter().all(|edit| edit["newText"] == "surface"));
        assert_eq!(
            edits
                .iter()
                .map(|edit| edit["range"]["start"]["line"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [11, 12, 13, 13]
        );
        assert_eq!(response(&messages, 4)["error"]["code"], -32602);
    }

    #[test]
    fn defines_and_renames_test_aliases_inside_dynamic_target_keys() {
        let uri = "file:///tmp/test-target-key-navigation.ice";
        let source = "app Demo\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  col #root\n    text \"Key\" #key\n    text \"Item\" #item(\"Key\")\ntest dynamic_targets\n  target key = #root/key\n  target item = #root/item(key.value)\n  expect exists #root/item(key.value)\n  expect text \"Item\" within #root/item(key.value)\n  click #root/item(key.value)\n";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": source } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 12, "character": 28 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 12, "character": 28 }, "newName": "lookup" },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(
            response(&messages, 2)["result"]["range"]["start"],
            json!({ "line": 11, "character": 9 })
        );
        let edits = response(&messages, 3)["result"]["changes"][uri]
            .as_array()
            .unwrap();
        assert_eq!(edits.len(), 5);
        assert!(edits.iter().all(|edit| edit["newText"] == "lookup"));
        assert_eq!(
            edits
                .iter()
                .map(|edit| edit["range"]["start"]["line"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn defines_and_safely_renames_checked_symbols_across_imports() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let other = "app Other\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let part = "component Card()\n  button \"😀\" -> clicked\ncomponent Panel()\n  text \"Other\"\non clicked\non mount\n";
        fixture.write("app.ice", root);
        fixture.write("other.ice", other);
        fixture.write("part.ice", part);
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let other_uri = file_path_uri(&fixture.path("other.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let workspace_uri = file_path_uri(&fixture.0);

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "rootUri": workspace_uri } }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": root } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": part } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": part_uri }, "position": { "line": 1, "character": 20 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "Tile" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "Panel" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": part_uri }, "position": { "line": 1, "character": 20 }, "newName": "activated" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "bad-name" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "tile" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": part_uri }, "position": { "line": 1, "character": 20 }, "newName": "mount" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": part_uri }, "position": { "line": 5, "character": 4 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": part_uri }, "position": { "line": 5, "character": 4 }, "newName": "launched" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 12,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "Tile.Header" },
            }),
            json!({ "jsonrpc": "2.0", "id": 13, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["result"]["uri"], part_uri);
        assert_eq!(
            response(&messages, 2)["result"]["range"],
            json!({
                "start": { "line": 0, "character": 10 },
                "end": { "line": 0, "character": 14 },
            })
        );
        assert_eq!(response(&messages, 3)["result"]["placeholder"], "clicked");
        assert_eq!(
            response(&messages, 3)["result"]["range"],
            json!({
                "start": { "line": 1, "character": 17 },
                "end": { "line": 1, "character": 24 },
            })
        );
        assert_eq!(
            response(&messages, 4)["result"]["changes"][&root_uri]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            response(&messages, 4)["result"]["changes"][&part_uri]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            response(&messages, 4)["result"]["changes"][&other_uri]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(response(&messages, 5)["error"]["code"], -32602);
        assert_eq!(
            response(&messages, 6)["result"]["changes"][&part_uri]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(response(&messages, 7)["error"]["code"], -32602);
        assert_eq!(response(&messages, 8)["error"]["code"], -32602);
        assert_eq!(response(&messages, 9)["error"]["code"], -32602);
        assert_eq!(response(&messages, 10)["result"], Value::Null);
        assert_eq!(response(&messages, 11)["error"]["code"], -32602);
        assert_eq!(response(&messages, 12)["error"]["code"], -32602);
    }

    #[test]
    fn defines_and_renames_imported_style_recipes() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"recipes.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n  surface #111111\nview\n  box @panel\n    text \"Panel\"\n";
        let recipes = "recipe panel for box\n  @p-4 bg-surface rounded-md\n";
        fixture.write("app.ice", root);
        fixture.write("recipes.ice", recipes);
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let recipe_uri = file_path_uri(&fixture.path("recipes.ice"));
        let workspace_uri = file_path_uri(&fixture.0);

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "rootUri": workspace_uri } }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": root } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": recipe_uri, "text": recipes } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 9, "character": 8 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 9, "character": 8 }, "newName": "surface_panel" },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["result"]["uri"], recipe_uri);
        assert_eq!(
            response(&messages, 3)["result"]["changes"][&root_uri][0]["newText"],
            "surface_panel"
        );
        assert_eq!(
            response(&messages, 3)["result"]["changes"][&recipe_uri][0]["newText"],
            "surface_panel"
        );
    }

    #[test]
    fn imported_rename_requires_an_initialized_workspace() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let part = "component Card()\n  text \"Card\"\n";
        fixture.write("app.ice", root);
        fixture.write("part.ice", part);
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let documents = HashMap::from([
            (root_uri.clone(), root.to_owned()),
            (part_uri, part.to_owned()),
        ]);
        let params = json!({
            "textDocument": { "uri": root_uri },
            "position": { "line": 8, "character": 3 },
        });

        let navigation = navigation_at(&documents, &[], &params).unwrap();

        assert!(navigation.symbol.definition.path.is_some());
        assert!(!navigation.renameable());
    }

    #[test]
    fn imported_rename_accepts_a_new_root_inside_the_workspace() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let part = "component Card()\n  text \"Card\"\n";
        fixture.write("part.ice", part);
        let root_uri = file_path_uri(&fixture.path("new.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let documents = HashMap::from([
            (root_uri.clone(), root.to_owned()),
            (part_uri, part.to_owned()),
        ]);
        let params = json!({
            "textDocument": { "uri": root_uri },
            "position": { "line": 8, "character": 3 },
        });

        let navigation =
            navigation_at(&documents, std::slice::from_ref(&fixture.0), &params).unwrap();

        assert!(navigation.renameable());
    }

    #[test]
    fn imported_rename_stays_inside_the_initialized_workspace() {
        let fixture = Fixture::new();
        let workspace = fixture.path("workspace");
        let outside = fixture.path("outside");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let workspace_app = "app Workspace\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Workspace\"\n";
        let outside_app = "app Outside\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let part = "component Card()\n  text \"Card\"\n";
        fs::write(workspace.join("app.ice"), workspace_app).unwrap();
        fs::write(outside.join("app.ice"), outside_app).unwrap();
        fs::write(outside.join("part.ice"), part).unwrap();
        let root_uri = file_path_uri(&outside.join("app.ice"));
        let part_uri = file_path_uri(&outside.join("part.ice"));
        let documents = HashMap::from([
            (root_uri.clone(), outside_app.to_owned()),
            (part_uri, part.to_owned()),
        ]);
        let params = json!({
            "textDocument": { "uri": root_uri },
            "position": { "line": 8, "character": 3 },
        });

        let navigation = navigation_at(&documents, &[workspace], &params).unwrap();

        assert_eq!(navigation.symbol.name, "Card");
        assert!(!navigation.renameable());
    }

    #[test]
    fn open_fragment_new_facts_participate_in_rename_checks() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\nuse \"extra.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let part = "component Card()\n  text \"Card\"\n";
        fixture.write("app.ice", root);
        fixture.write("part.ice", part);
        fixture.write("extra.ice", "// saved fragment\n");
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let extra_uri = file_path_uri(&fixture.path("extra.ice"));
        let documents = HashMap::from([
            (root_uri.clone(), root.to_owned()),
            (extra_uri, "component Tile()\n  text \"New\"\n".to_owned()),
        ]);
        let params = json!({
            "textDocument": { "uri": root_uri },
            "position": { "line": 9, "character": 3 },
        });

        let navigation =
            navigation_at(&documents, std::slice::from_ref(&fixture.0), &params).unwrap();

        assert_eq!(navigation.symbol.name, "Card");
        assert!(navigation.renameable());
        assert!(navigation.collides("Tile"));
    }

    #[test]
    fn renames_a_compound_component_family_together() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Dialog\n    Dialog.Header\n";
        let part =
            "component Dialog()\n  slot Header\ncomponent Dialog.Header()\n  text \"Header\"\n";
        fixture.write("app.ice", root);
        fixture.write("part.ice", part);
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let workspace_uri = file_path_uri(&fixture.0);

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "rootUri": workspace_uri } }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": root } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": part } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "Modal" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 9, "character": 8 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 9, "character": 8 }, "newName": "Modal.Header" },
            }),
            json!({ "jsonrpc": "2.0", "id": 5, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let root_edits = response(&messages, 2)["result"]["changes"][&root_uri]
            .as_array()
            .unwrap();
        let part_edits = response(&messages, 2)["result"]["changes"][&part_uri]
            .as_array()
            .unwrap();
        assert_eq!(root_edits.len(), 2);
        assert_eq!(part_edits.len(), 2);
        assert!(root_edits.iter().any(|edit| edit["newText"] == "Modal"));
        assert!(
            root_edits
                .iter()
                .any(|edit| edit["newText"] == "Modal.Header")
        );
        assert!(part_edits.iter().any(|edit| edit["newText"] == "Modal"));
        assert!(
            part_edits
                .iter()
                .any(|edit| edit["newText"] == "Modal.Header")
        );
        assert_eq!(response(&messages, 3)["result"], Value::Null);
        assert_eq!(response(&messages, 4)["error"]["code"], -32602);
    }

    #[test]
    fn rename_waits_until_every_workspace_app_root_checks() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let part = "component Card()\n  text \"Card\"\n";
        let broken = "app Broken\nview\n  wat\n";
        fixture.write("app.ice", root);
        fixture.write("part.ice", part);
        fixture.write("broken.ice", broken);
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let workspace_uri = file_path_uri(&fixture.0);

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "rootUri": workspace_uri } }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": root } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": part } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "Tile" },
            }),
            json!({ "jsonrpc": "2.0", "id": 5, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["result"]["uri"], part_uri);
        assert_eq!(response(&messages, 3)["result"], Value::Null);
        assert_eq!(response(&messages, 4)["error"]["code"], -32602);
    }

    #[test]
    fn uses_unsaved_import_ranges_for_navigation() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let part = "component Card()\n  text \"Card\"\n";
        let dirty_part = format!("// unsaved line\n{part}");
        fixture.write("app.ice", root);
        fixture.write("part.ice", part);
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let workspace_uri = file_path_uri(&fixture.0);

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "rootUri": workspace_uri } }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": root } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": dirty_part } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/definition",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 8, "character": 3 }, "newName": "Tile" },
            }),
            json!({ "jsonrpc": "2.0", "id": 5, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["result"]["uri"], part_uri);
        assert_eq!(
            response(&messages, 2)["result"]["range"],
            json!({
                "start": { "line": 1, "character": 10 },
                "end": { "line": 1, "character": 14 },
            })
        );
        assert_eq!(response(&messages, 3)["result"]["placeholder"], "Card");
        assert_eq!(
            response(&messages, 4)["result"]["changes"][&part_uri][0]["range"],
            json!({
                "start": { "line": 1, "character": 10 },
                "end": { "line": 1, "character": 14 },
            })
        );
    }

    #[test]
    fn returns_json_rpc_errors_and_rejects_requests_after_shutdown() {
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "id": "unknown", "method": "ice/unknown" }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/formatting",
                "params": {
                    "textDocument": { "uri": "file:///not-open.ice" },
                    "options": { "tabSize": 2, "insertSpaces": true },
                },
            }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "textDocument/completion" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, "unknown")["error"]["code"], -32601);
        assert_eq!(response(&messages, 2)["error"]["code"], -32602);
        assert_eq!(response(&messages, 3)["result"], Value::Null);
        assert_eq!(response(&messages, 4)["error"]["code"], -32600);
    }

    #[test]
    fn exit_before_shutdown_is_an_error() {
        let mut input = Vec::new();
        frame(&json!({ "jsonrpc": "2.0", "method": "exit" }), &mut input);
        let error = serve(&mut BufReader::new(Cursor::new(input)), &mut Vec::new()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "LSP exit received before shutdown");
    }

    #[test]
    fn ranges_use_utf16_and_clamp_to_the_document() {
        assert_eq!(
            diagnostic_range("a😀b\n", 1, 2),
            json!({
                "start": { "line": 0, "character": 1 },
                "end": { "line": 0, "character": 3 },
            })
        );
        assert_eq!(
            diagnostic_range("a😀b\n", 99, 99),
            json!({
                "start": { "line": 1, "character": 0 },
                "end": { "line": 1, "character": 0 },
            })
        );
        assert_eq!(
            diagnostic_range("abc\r\n", 1, 99),
            json!({
                "start": { "line": 0, "character": 3 },
                "end": { "line": 0, "character": 3 },
            })
        );
        assert_eq!(
            whole_document_range("first\n😀x"),
            json!({
                "start": { "line": 0, "character": 0 },
                "end": { "line": 1, "character": 3 },
            })
        );
        let path = Path::new("/tmp/Ice Demo/😀.ice");
        assert_eq!(file_uri_path(&file_path_uri(path)).as_deref(), Some(path));

        let stale = ui_lang_core::SourceRange {
            path: None,
            line: 1,
            start_column: 2,
            end_column: 99,
        };
        assert!(source_range("short", &stale).is_none());
        assert!(source_range("abc\r\n", &stale).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn file_uris_round_trip_non_utf8_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(OsString::from_vec(b"/tmp/Ice \xFF.ice".to_vec()));
        let uri = file_path_uri(&path);

        assert_eq!(uri, "file:///tmp/Ice%20%FF.ice");
        assert_eq!(file_uri_path(&uri).as_deref(), Some(path.as_path()));
    }

    #[test]
    fn rename_rejects_stale_same_length_text() {
        let uri = "file:///tmp/app.ice".to_owned();
        let definition = ui_lang_core::SourceRange {
            path: None,
            line: 1,
            start_column: 1,
            end_column: 5,
        };
        let symbol = ui_lang_core::CheckedSymbol {
            kind: ui_lang_core::SymbolKind::Component,
            scope: None,
            name: "Card".to_owned(),
            definition: definition.clone(),
            references: Vec::new(),
            renameable: true,
        };
        let navigation = Navigation {
            symbol: symbol.clone(),
            family: vec![symbol],
            occurrence: definition,
            declarations: Vec::new(),
            root_uri: uri.clone(),
        };

        assert!(
            workspace_edit(
                &HashMap::from([(uri, "Tile\n".to_owned())]),
                &navigation,
                "Panel"
            )
            .is_none()
        );
    }

    #[cfg(windows)]
    #[test]
    fn file_uris_round_trip_windows_drive_paths() {
        let path = Path::new(r"C:\Ice Demo\😀.ice");
        let uri = file_path_uri(path);

        assert_eq!(uri, "file:///C:/Ice%20Demo/%F0%9F%98%80.ice");
        assert_eq!(file_uri_path(&uri).as_deref(), Some(path));
        assert_eq!(
            file_uri_path("file://LOCALHOST/C:/Ice%20Demo/app.ice").as_deref(),
            Some(Path::new(r"C:\Ice Demo\app.ice"))
        );

        let unc = Path::new(r"\\localhost-server\share\app.ice");
        assert_eq!(file_path_uri(unc), "file://localhost-server/share/app.ice");
        assert_eq!(
            file_uri_path("file://localhost-server/share/app.ice").as_deref(),
            Some(unc)
        );

        assert_eq!(
            file_path_uri(Path::new(r"\\?\C:\Ice Demo\app.ice")),
            "file:///C:/Ice%20Demo/app.ice"
        );
        assert_eq!(
            file_path_uri(Path::new(r"\\?\UNC\localhost-server\share\app.ice")),
            "file://localhost-server/share/app.ice"
        );
    }
}
