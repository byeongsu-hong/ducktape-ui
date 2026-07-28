use crate::schema;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use ui_lang_core::{
    CursorContext, STYLE_STATUS_NAMES as STATUS_NAMES, SourcePosition,
    editor_ancestor_lines as ancestor_lines, editor_block_end as child_block_end,
    editor_component_name as component_name_on_line, editor_first_word as first_word,
    editor_indentation as indentation,
};

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
        let namespace = self
            .symbol
            .name
            .rsplit_once("::")
            .map(|(namespace, _)| namespace);
        let new_name = namespace.map_or_else(
            || new_name.to_owned(),
            |namespace| format!("{namespace}::{new_name}"),
        );
        if self.symbol.kind == ui_lang_core::SymbolKind::Component
            && let Some(suffix) = name.strip_prefix(&self.symbol.name)
        {
            return format!("{new_name}{suffix}");
        }
        new_name
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
                            "hoverProvider": true,
                            "signatureHelpProvider": { "triggerCharacters": [" ", "=", "<"] },
                            "codeActionProvider": true,
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
                        let items = completion_items_at(&documents, &message["params"])
                            .unwrap_or_else(schema::completion_items);
                        respond(writer, id, Value::Array(items))?;
                    } else {
                        invalid_params(writer, id, "invalid text document position")?;
                    }
                }
            }
            "textDocument/hover" => {
                if let Some(id) = id {
                    if valid_text_document_position_params(&message["params"]) {
                        let result = hover_at(&documents, &message["params"]);
                        respond(writer, id, result.unwrap_or(Value::Null))?;
                    } else {
                        invalid_params(writer, id, "invalid text document position")?;
                    }
                }
            }
            "textDocument/signatureHelp" => {
                if let Some(id) = id {
                    if valid_text_document_position_params(&message["params"]) {
                        let result = signature_help_at(&documents, &message["params"]);
                        respond(writer, id, result.unwrap_or(Value::Null))?;
                    } else {
                        invalid_params(writer, id, "invalid text document position")?;
                    }
                }
            }
            "textDocument/codeAction" => {
                if let Some(id) = id {
                    if valid_code_action_params(&message["params"]) {
                        let actions =
                            code_actions_at(&documents, &message["params"]).unwrap_or_default();
                        respond(writer, id, Value::Array(actions))?;
                    } else {
                        invalid_params(writer, id, "invalid code action params")?;
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
                                            "placeholder": navigation
                                                .symbol
                                                .name
                                                .rsplit("::")
                                                .next()
                                                .unwrap_or(&navigation.symbol.name),
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

fn valid_code_action_params(params: &Value) -> bool {
    let range = &params["range"];
    params.is_object()
        && params["textDocument"]["uri"].is_string()
        && range["start"]["line"].as_u64().is_some()
        && range["start"]["character"].as_u64().is_some()
        && range["end"]["line"].as_u64().is_some()
        && range["end"]["character"].as_u64().is_some()
}

fn checked_document(
    documents: &HashMap<String, String>,
    uri: &str,
) -> Option<ui_lang_core::CheckedDocument> {
    let source = documents.get(uri)?;
    let overlays = source_overlays(documents);
    file_uri_path(uri).map_or_else(
        || ui_lang_core::analyze(source).ok(),
        |path| ui_lang_core::analyze_file_with_overlays(path, &overlays).ok(),
    )
}

fn completion_items_at(documents: &HashMap<String, String>, params: &Value) -> Option<Vec<Value>> {
    let uri = params["textDocument"]["uri"].as_str()?;
    let source = documents.get(uri)?;
    let line = usize::try_from(params["position"]["line"].as_u64()?).ok()?;
    let checked = checked_document(documents, uri);
    let parsed = checked
        .is_none()
        .then(|| ui_lang_core::parse(source).ok())
        .flatten();
    let document = checked.as_deref().or(parsed.as_ref());
    let context = ui_lang_core::cursor_context(
        source,
        SourcePosition {
            line,
            column: usize::try_from(params["position"]["character"].as_u64()?).ok()?,
        },
        document,
    );
    let items = match context {
        CursorContext::TopLevel => schema::completion_items_for(&["declaration"]),
        CursorContext::HandlerBody => {
            let mut items = schema::completion_items_for(&["statement"]);
            if let Some(document) = document {
                items.extend(effect_completions(document));
            }
            items
        }
        CursorContext::StyleStatus { target } => status_completions(target.as_deref()),
        CursorContext::ViewNode | CursorContext::NodeMetadata { .. } => {
            let mut items = schema::completion_items_for(&["layout", "widget", "control"]);
            if let Some(document) = document {
                items.extend(document.components.iter().map(component_node_completion));
            }
            items
        }
        CursorContext::MatchArms { match_line } => document
            .map(|document| match_arm_completions(document, match_line))
            .unwrap_or_default(),
        CursorContext::ComponentCall { component: name } => document
            .and_then(|document| {
                document
                    .components
                    .iter()
                    .find(|component| component.name == name)
            })
            .map(component_contract_completions)
            .unwrap_or_default(),
        CursorContext::PaletteValue { contract } => document
            .map(|document| palette_value_completions(document, &contract))
            .unwrap_or_default(),
        CursorContext::ComponentEvents {
            component: name,
            forwarding,
        } => document
            .and_then(|document| {
                document
                    .components
                    .iter()
                    .find(|component| component.name == name)
            })
            .map(|component| component_event_completions(component, forwarding))
            .unwrap_or_default(),
        CursorContext::ThemeContract => Vec::new(),
        CursorContext::TestBody => schema::completion_items_for(&[
            "test configuration",
            "test statement",
            "test interaction",
            "test assertion",
            "operator",
        ]),
    };
    Some(items)
}

fn component_node_completion(component: &ui_lang_core::Component) -> Value {
    json!({
        "label": component.name,
        "kind": 7,
        "detail": "Ice component",
        "insertText": component.name,
    })
}

fn palette_value_completions(document: &ui_lang_core::Document, contract: &str) -> Vec<Value> {
    document
        .palettes
        .iter()
        .map(|palette| {
            let value = format!("{contract}.{}", palette.name);
            json!({
                "label": value,
                "kind": 20,
                "detail": format!("{} palette", contract),
                "insertText": value,
            })
        })
        .collect()
}

fn component_contract_completions(component: &ui_lang_core::Component) -> Vec<Value> {
    let mut items = component
        .params
        .iter()
        .map(|param| {
            let capability = if param.bind { "bind" } else { "read" };
            let default = if param.default.is_some() {
                " (default)"
            } else {
                ""
            };
            let operator = if param.bind { "<->" } else { "=" };
            json!({
                "label": format!("{}{operator}", param.name),
                "kind": 10,
                "detail": format!("{capability} {}{default}", param.ty.display()),
                "insertText": format!("{}{operator}${{1:value}}", param.name),
                "insertTextFormat": 2,
            })
        })
        .collect::<Vec<_>>();
    items.extend(component_slots(&component.root).into_iter().map(|slot| {
        let optional = if slot.optional { "optional " } else { "" };
        json!({
            "label": format!("{}:", slot.name),
            "kind": 10,
            "detail": format!("{optional}component slot"),
            "insertText": format!("{}:\n  $0", slot.name),
            "insertTextFormat": 2,
        })
    }));
    items.extend(component_event_completions(component, false));
    items
}

fn component_event_completions(
    component: &ui_lang_core::Component,
    forwarding: bool,
) -> Vec<Value> {
    component
        .events
        .iter()
        .map(|event| {
            let insert = if forwarding {
                event.name.clone()
            } else {
                format!("{} -> ${{1:handler}}", event.name)
            };
            json!({
                "label": event.name,
                "kind": 23,
                "detail": component_event_signature(event),
                "insertText": insert,
                "insertTextFormat": 2,
            })
        })
        .collect()
}

fn status_completions(parent: Option<&str>) -> Vec<Value> {
    let statuses: &[&str] = match parent {
        Some("button") => &["active", "hovered", "pressed", "disabled"],
        Some("input" | "editor" | "combo") => &[
            "active",
            "hovered",
            "focused",
            "focused-hovered",
            "disabled",
        ],
        Some("slider" | "scroll") => &["active", "hovered", "dragged"],
        Some("pick") => &["active", "hovered", "opened", "opened-hovered"],
        Some("checkbox" | "toggler") => &["active", "hovered", "disabled"],
        Some("radio") => &["active", "hovered"],
        _ => STATUS_NAMES,
    };
    statuses
        .iter()
        .map(|status| {
            json!({
                "label": status,
                "kind": 14,
                "detail": "Ice widget status",
                "insertText": format!("{status} ${{1:property}}=${{2:value}}"),
                "insertTextFormat": 2,
            })
        })
        .collect()
}

fn effect_completions(document: &ui_lang_core::Document) -> Vec<Value> {
    document
        .functions
        .iter()
        .filter_map(|function| {
            let keyword = match function.kind {
                ui_lang_core::ExternKind::Future => "run",
                ui_lang_core::ExternKind::Task => "task",
                ui_lang_core::ExternKind::Stream => "stream",
                _ => return None,
            };
            let error = function.error.as_ref().map_or(String::new(), |_| {
                format!(" | ${{4:{}_failed}} _", function.name)
            });
            Some(json!({
                "label": function.name,
                "kind": 3,
                "detail": format!("Ice {keyword} extern -> {}", function.output.display()),
                "insertText": format!(
                    "{keyword} {}(${{1}}) -> ${{2:{}_completed}} ${{3:_}}{error}",
                    function.name, function.name
                ),
                "insertTextFormat": 2,
            }))
        })
        .collect()
}

fn visit_view<'a>(
    node: &'a ui_lang_core::ViewNode,
    visitor: &mut impl FnMut(&'a ui_lang_core::ViewNode),
) {
    use ui_lang_core::{ResponsiveContent, ViewNode};
    visitor(node);
    match node {
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => {
            for child in children {
                visit_view(child, visitor);
            }
        }
        ViewNode::Match { arms, .. } => {
            for child in arms.iter().flat_map(|arm| &arm.children) {
                visit_view(child, visitor);
            }
        }
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::ResizeHandle { content, .. }
        | ViewNode::Container { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => visit_view(content, visitor),
        ViewNode::Tooltip { content, tip, .. }
        | ViewNode::Overlay {
            content,
            layer: tip,
            ..
        } => {
            visit_view(content, visitor);
            visit_view(tip, visitor);
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => {
            for child in panes
                .iter()
                .flat_map(ui_lang_core::PaneView::nodes)
                .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            {
                visit_view(child, visitor);
            }
        }
        ViewNode::Table { columns, .. } => {
            for column in columns {
                visit_view(&column.header, visitor);
                visit_view(&column.cell, visitor);
            }
        }
        ViewNode::Component { slots, .. } => {
            for slot in slots {
                visit_view(&slot.content, visitor);
            }
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                visit_view(narrow, visitor);
                visit_view(wide, visitor);
            }
            ResponsiveContent::Size { content, .. } => visit_view(content, visitor),
        },
        _ => {}
    }
}

#[derive(Clone, Copy)]
struct ComponentSlotInfo<'a> {
    name: &'a str,
    optional: bool,
}

fn component_slots(node: &ui_lang_core::ViewNode) -> Vec<ComponentSlotInfo<'_>> {
    let mut output = Vec::new();
    visit_view(node, &mut |node| {
        if let ui_lang_core::ViewNode::Slot { name, optional, .. } = node {
            output.push(ComponentSlotInfo {
                name,
                optional: *optional,
            });
        }
    });
    output
}

fn match_node_at_line(
    root: &ui_lang_core::ViewNode,
    line: usize,
) -> Option<(&ui_lang_core::Expr, &[ui_lang_core::MatchArm])> {
    let mut found = None;
    visit_view(root, &mut |node| {
        if let ui_lang_core::ViewNode::Match { value, arms, span } = node
            && span.line == line + 1
        {
            found = Some((value, arms.as_slice()));
        }
    });
    found
}

fn match_at_line(
    document: &ui_lang_core::Document,
    line: usize,
) -> Option<(
    &ui_lang_core::Expr,
    &[ui_lang_core::MatchArm],
    Option<&ui_lang_core::Component>,
)> {
    if let Some((value, arms)) = match_node_at_line(&document.view, line) {
        return Some((value, arms, None));
    }
    document.components.iter().find_map(|component| {
        match_node_at_line(&component.root, line)
            .map(|(value, arms)| (value, arms, Some(component)))
    })
}

fn match_value_type<'a>(
    document: &'a ui_lang_core::Document,
    component: Option<&'a ui_lang_core::Component>,
    value: &ui_lang_core::Expr,
) -> Option<&'a ui_lang_core::Type> {
    let ui_lang_core::Expr::Path(path) = value else {
        return None;
    };
    let [name] = path.as_slice() else {
        return None;
    };
    component
        .and_then(|component| {
            component
                .params
                .iter()
                .find(|param| param.name == *name)
                .map(|param| &param.ty)
                .or_else(|| {
                    component
                        .states
                        .iter()
                        .find(|state| state.name == *name)
                        .map(|state| &state.ty)
                })
        })
        .or_else(|| {
            document
                .states
                .iter()
                .find(|state| state.name == *name)
                .map(|state| &state.ty)
        })
        .or_else(|| {
            document
                .derived
                .iter()
                .find(|derived| derived.name == *name)
                .map(|derived| &derived.ty)
        })
}

fn missing_match_patterns(
    document: &ui_lang_core::Document,
    value_ty: &ui_lang_core::Type,
    arms: &[ui_lang_core::MatchArm],
) -> Vec<String> {
    use ui_lang_core::{MatchPattern, Type};
    let covered = arms
        .iter()
        .filter_map(|arm| match &arm.pattern {
            MatchPattern::Some(_) => Some("some".to_owned()),
            MatchPattern::None => Some("none".to_owned()),
            MatchPattern::Ok(_) => Some("ok".to_owned()),
            MatchPattern::Err(_) => Some("err".to_owned()),
            MatchPattern::Enum { variant, .. } => Some(variant.clone()),
            MatchPattern::Wildcard => None,
        })
        .collect::<BTreeSet<_>>();
    match value_ty {
        Type::Option(_) => [("some", "some(value)"), ("none", "none")]
            .into_iter()
            .filter(|(key, _)| !covered.contains(*key))
            .map(|(_, pattern)| pattern.to_owned())
            .collect(),
        Type::Result(_, _) => [("ok", "ok(value)"), ("err", "err(error)")]
            .into_iter()
            .filter(|(key, _)| !covered.contains(*key))
            .map(|(_, pattern)| pattern.to_owned())
            .collect(),
        Type::Named(name) => document
            .enums
            .iter()
            .find(|item| item.name == *name)
            .map(|item| {
                item.variants
                    .iter()
                    .filter(|variant| !covered.contains(&variant.name))
                    .map(|variant| {
                        if variant.payload.is_some() {
                            format!("{name}.{}(value)", variant.name)
                        } else {
                            format!("{name}.{}", variant.name)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        Type::Palette(contract) => document
            .palettes
            .iter()
            .filter(|palette| !covered.contains(&palette.name))
            .map(|palette| format!("{contract}.{}", palette.name))
            .collect(),
        _ => Vec::new(),
    }
}

fn match_patterns_fit_type(
    document: &ui_lang_core::Document,
    value_ty: &ui_lang_core::Type,
    arms: &[ui_lang_core::MatchArm],
) -> bool {
    use ui_lang_core::{MatchPattern, Type};
    arms.iter().all(|arm| match (value_ty, &arm.pattern) {
        (Type::Option(_), MatchPattern::Some(_) | MatchPattern::None)
        | (Type::Result(_, _), MatchPattern::Ok(_) | MatchPattern::Err(_))
        | (_, MatchPattern::Wildcard) => true,
        (
            Type::Named(name),
            MatchPattern::Enum {
                enum_name,
                variant,
                binding,
            },
        ) if name == enum_name => document
            .enums
            .iter()
            .find(|item| item.name == *name)
            .and_then(|item| item.variants.iter().find(|item| item.name == *variant))
            .is_some_and(|variant| variant.payload.is_some() == binding.is_some()),
        (
            Type::Palette(contract),
            MatchPattern::Enum {
                enum_name,
                variant,
                binding,
            },
        ) if contract == enum_name && binding.is_none() => document
            .palettes
            .iter()
            .any(|palette| palette.name == *variant),
        _ => false,
    })
}

fn missing_match_patterns_at(
    document: &ui_lang_core::Document,
    line: usize,
) -> Option<Vec<String>> {
    let (value, arms, component) = match_at_line(document, line)?;
    let value_ty = match_value_type(document, component, value)?;
    match_patterns_fit_type(document, value_ty, arms)
        .then(|| missing_match_patterns(document, value_ty, arms))
}

fn match_arm_completions(document: &ui_lang_core::Document, line: usize) -> Vec<Value> {
    missing_match_patterns_at(document, line)
        .unwrap_or_default()
        .into_iter()
        .map(|pattern| {
            json!({
                "label": pattern,
                "kind": 14,
                "detail": "missing typed match arm",
                "insertText": format!("{pattern}\n  $0"),
                "insertTextFormat": 2,
            })
        })
        .collect()
}

fn component_event_signature(event: &ui_lang_core::ComponentEvent) -> String {
    let payloads = event
        .payloads
        .iter()
        .map(ui_lang_core::Type::display)
        .collect::<Vec<_>>()
        .join(", ");
    format!("event {}({payloads})", event.name)
}

fn hover_at(documents: &HashMap<String, String>, params: &Value) -> Option<Value> {
    let uri = params["textDocument"]["uri"].as_str()?;
    let source = documents.get(uri)?;
    let line = usize::try_from(params["position"]["line"].as_u64()?).ok()?;
    let character = usize::try_from(params["position"]["character"].as_u64()?).ok()?;
    let word = word_at(source_line(source, line)?, character)?;
    let checked = checked_document(documents, uri);
    let parsed = checked
        .is_none()
        .then(|| ui_lang_core::parse(source).ok())
        .flatten();
    let document = checked.as_deref().or(parsed.as_ref())?;
    let value = if let Some(component) = document
        .components
        .iter()
        .find(|component| component.name == word)
    {
        component_hover(component)
    } else {
        let recipe_name = word.strip_prefix('@').unwrap_or(word);
        let recipe = document
            .recipes
            .iter()
            .find(|recipe| recipe.name == recipe_name)?;
        recipe_hover(document, recipe)
    };
    Some(json!({
        "contents": { "kind": "markdown", "value": value },
    }))
}

fn signature_help_at(documents: &HashMap<String, String>, params: &Value) -> Option<Value> {
    let uri = params["textDocument"]["uri"].as_str()?;
    let source = documents.get(uri)?;
    let line = usize::try_from(params["position"]["line"].as_u64()?).ok()?;
    let character = usize::try_from(params["position"]["character"].as_u64()?).ok()?;
    let checked = checked_document(documents, uri);
    let parsed = checked
        .is_none()
        .then(|| ui_lang_core::parse(source).ok())
        .flatten();
    let document = checked.as_deref().or(parsed.as_ref())?;
    let name = component_name_on_line(source_line(source, line)?, Some(document))?;
    let component = document
        .components
        .iter()
        .find(|component| component.name == name)?;
    let parameters = component
        .params
        .iter()
        .map(|param| {
            let capability = if param.bind { "bind " } else { "" };
            let default = if param.default.is_some() {
                "=<default>"
            } else {
                ""
            };
            format!("{capability}{}:{}{default}", param.name, param.ty.display())
        })
        .collect::<Vec<_>>();
    let output = if component.output == ui_lang_core::Type::Unit {
        String::new()
    } else {
        format!(" -> {}", component.output.display())
    };
    let prefix = source_line(source, line)?
        .encode_utf16()
        .take(character)
        .collect::<Vec<_>>();
    let prefix = String::from_utf16(&prefix).ok()?;
    let active = component
        .params
        .iter()
        .position(|param| {
            prefix
                .split_ascii_whitespace()
                .last()
                .is_some_and(|word| word.starts_with(&param.name))
        })
        .unwrap_or(0);
    Some(json!({
        "signatures": [{
            "label": format!("{}({}){output}", component.name, parameters.join(", ")),
            "documentation": { "kind": "markdown", "value": component_hover(component) },
            "parameters": parameters
                .iter()
                .map(|label| json!({ "label": label }))
                .collect::<Vec<_>>(),
        }],
        "activeSignature": 0,
        "activeParameter": active,
    }))
}

fn word_range_at(line: &str, utf16_character: usize) -> Option<(usize, usize)> {
    let column = utf16_column(line, utf16_character)?.saturating_sub(1);
    let column = line
        .char_indices()
        .nth(column)
        .map_or(line.len(), |(byte, _)| byte);
    let bytes = line.as_bytes();
    let is_word =
        |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'@');
    let mut start = column.min(bytes.len());
    while start > 0 && is_word(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = column.min(bytes.len());
    while end < bytes.len() && is_word(bytes[end]) {
        end += 1;
    }
    (start < end).then_some((start, end))
}

fn word_at(line: &str, utf16_character: usize) -> Option<&str> {
    let (start, end) = word_range_at(line, utf16_character)?;
    Some(&line[start..end])
}

fn component_hover(component: &ui_lang_core::Component) -> String {
    let mut lines = vec![format!("```ice\ncomponent {}", component.name)];
    for param in &component.params {
        let capability = if param.bind { "bind" } else { "read" };
        let default = if param.default.is_some() {
            " = <default>"
        } else {
            ""
        };
        lines.push(format!(
            "  {}: {capability} {}{default}",
            param.name,
            param.ty.display()
        ));
    }
    if component.output != ui_lang_core::Type::Unit {
        lines.push(format!("  output: {}", component.output.display()));
    }
    if !component.events.is_empty() {
        lines.push("  emits:".into());
        lines.extend(
            component
                .events
                .iter()
                .map(|event| format!("    {}", component_event_signature(event))),
        );
    }
    let slots = component_slots(&component.root);
    if !slots.is_empty() {
        lines.push(format!(
            "  slots: {}",
            slots
                .iter()
                .map(|slot| format!("{}{}", slot.name, if slot.optional { "?" } else { "" }))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    lines.push("```".into());
    lines.join("\n")
}

fn recipe_hover(document: &ui_lang_core::Document, recipe: &ui_lang_core::StyleRecipe) -> String {
    fn flatten(
        document: &ui_lang_core::Document,
        recipe: &ui_lang_core::StyleRecipe,
        utilities: &mut Vec<String>,
    ) {
        if let Some(base) = recipe.base.as_deref().and_then(|name| {
            document
                .recipes
                .iter()
                .find(|candidate| candidate.name == name)
        }) {
            flatten(document, base, utilities);
        }
        utilities.extend(recipe.utilities.iter().cloned());
    }

    let mut utilities = Vec::new();
    flatten(document, recipe, &mut utilities);
    let base = recipe
        .base
        .as_deref()
        .map(|base| format!(" extends @{base}"))
        .unwrap_or_default();
    format!(
        "```ice\n@{} for {}{base}\n  {}\n```",
        recipe.name,
        recipe.target.source_name(),
        utilities
            .iter()
            .map(|utility| format!("@{utility}"))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn code_actions_at(documents: &HashMap<String, String>, params: &Value) -> Option<Vec<Value>> {
    let uri = params["textDocument"]["uri"].as_str()?;
    let source = documents.get(uri)?;
    let line = usize::try_from(params["range"]["start"]["line"].as_u64()?).ok()?;
    let character = usize::try_from(params["range"]["start"]["character"].as_u64()?).ok()?;
    let lines = source.split('\n').collect::<Vec<_>>();
    let current = lines.get(line).copied()?;
    let parsed = ui_lang_core::parse(source).ok();
    let checked = checked_document(documents, uri);
    let document = checked.as_deref().or(parsed.as_ref());
    let mut actions = Vec::new();

    if let Some(document) = document {
        bind_declaration_action(source, line, character, document, uri, &mut actions);
        component_call_actions(source, &lines, line, document, uri, &mut actions);
        exhaustive_match_action(source, &lines, line, document, uri, &mut actions);
        fallible_route_action(source, line, current, document, uri, &mut actions);
        handler_skeleton_action(source, line, current, document, uri, &mut actions);
        recipe_extraction_action(source, &lines, current, document, uri, &mut actions);
        component_handler_event_action(source, &lines, line, current, document, uri, &mut actions);
    }
    qualification_action(documents, source, line, character, uri, &mut actions);
    button_label_action(source, &lines, line, uri, &mut actions);
    with_block_action(source, line, current, uri, &mut actions);
    Some(actions)
}

fn code_action(title: &str, kind: &str, uri: &str, edits: Vec<Value>) -> Value {
    json!({
        "title": title,
        "kind": kind,
        "edit": { "changes": { uri: edits } },
    })
}

fn utf16_offset(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].encode_utf16().count()
}

fn line_range(source: &str, line: usize, start: usize, end: usize) -> Value {
    let text = source_line(source, line).unwrap_or("");
    json!({
        "start": { "line": line, "character": utf16_offset(text, start) },
        "end": { "line": line, "character": utf16_offset(text, end) },
    })
}

fn insertion_at_line(line: usize) -> Value {
    json!({
        "start": { "line": line, "character": 0 },
        "end": { "line": line, "character": 0 },
    })
}

fn line_block_range(source: &str, start: usize, end: usize) -> Value {
    let line_count = source.split('\n').count();
    let end = if end < line_count {
        json!({ "line": end, "character": 0 })
    } else {
        whole_document_range(source)["end"].clone()
    };
    json!({
        "start": { "line": start, "character": 0 },
        "end": end,
    })
}

fn edit(range: Value, new_text: impl Into<String>) -> Value {
    json!({ "range": range, "newText": new_text.into() })
}

fn enclosing_match_line(lines: &[&str], line: usize) -> Option<usize> {
    lines
        .get(line)
        .is_some_and(|line| line.trim_start().starts_with("match "))
        .then_some(line)
        .or_else(|| {
            ancestor_lines(lines, line)
                .into_iter()
                .find(|(_, candidate)| candidate.trim_start().starts_with("match "))
                .map(|(line, _)| line)
        })
}

fn exhaustive_match_action(
    source: &str,
    lines: &[&str],
    line: usize,
    document: &ui_lang_core::Document,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some(match_line) = enclosing_match_line(lines, line) else {
        return;
    };
    let Some(patterns) = missing_match_patterns_at(document, match_line) else {
        return;
    };
    if patterns.is_empty() {
        return;
    }
    let Some((_, arms, _)) = match_at_line(document, match_line) else {
        return;
    };
    let match_indent = indentation(lines[match_line]);
    let wildcard = arms
        .iter()
        .find(|arm| matches!(arm.pattern, ui_lang_core::MatchPattern::Wildcard));
    let fallback = wildcard.and_then(|arm| {
        let wildcard_line = arm.span.line.checked_sub(1)?;
        let end = child_block_end(lines, wildcard_line, match_indent + 2);
        Some((wildcard_line, end, &lines[wildcard_line + 1..end]))
    });
    let replacement = patterns
        .iter()
        .map(|pattern| {
            let mut arm = format!("{}{pattern}\n", " ".repeat(match_indent + 2));
            if let Some((_, _, children)) = fallback {
                for child in children {
                    arm.push_str(child);
                    arm.push('\n');
                }
            } else {
                arm.push_str(&format!("{}space\n", " ".repeat(match_indent + 4)));
            }
            arm
        })
        .collect::<String>();
    let (title, range) = match fallback {
        Some((wildcard_line, end, _)) if line == wildcard_line => (
            "Replace wildcard with all missing typed match arms",
            line_block_range(source, wildcard_line, end),
        ),
        Some((wildcard_line, _, _)) => (
            "Add all missing typed match arms before wildcard",
            insertion_at_line(wildcard_line),
        ),
        None => (
            "Add all missing typed match arms",
            insertion_at_line(child_block_end(lines, match_line, match_indent)),
        ),
    };
    actions.push(code_action(
        title,
        "quickfix",
        uri,
        vec![edit(range, replacement)],
    ));
}

fn import_aliases(source: &str) -> BTreeSet<&str> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix("use ")?;
            let (_, alias) = line.rsplit_once(" as ")?;
            (!alias.is_empty()
                && alias
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
            .then_some(alias)
        })
        .collect()
}

fn qualification_action(
    documents: &HashMap<String, String>,
    source: &str,
    line: usize,
    character: usize,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some(path) = file_uri_path(uri) else {
        return;
    };
    let path = canonical_path(&path).unwrap_or(path);
    let Some(text) = source_line(source, line) else {
        return;
    };
    let Some((start, end)) = word_range_at(text, character) else {
        return;
    };
    if text[..start].ends_with("::") || text[end..].starts_with("::") {
        return;
    }
    let word = &text[start..end];
    let name = word.strip_prefix('@').unwrap_or(word);
    if name.is_empty() || name.contains(['.', '-']) {
        return;
    }
    let aliases = import_aliases(source);
    if aliases.is_empty() {
        return;
    }
    let mut overlays = source_overlays(documents);
    if ui_lang_core::analyze_file_with_overlays(&path, &overlays).is_ok() {
        return;
    }
    let offset = source
        .split_inclusive('\n')
        .take(line)
        .map(str::len)
        .sum::<usize>();
    let mut candidates = Vec::new();
    for alias in aliases {
        let qualified = if word.starts_with('@') {
            format!("@{alias}::{name}")
        } else {
            format!("{alias}::{name}")
        };
        let mut candidate = source.to_owned();
        candidate.replace_range(offset + start..offset + end, &qualified);
        overlays.insert(path.clone(), candidate);
        if ui_lang_core::analyze_file_with_overlays(&path, &overlays).is_ok() {
            candidates.push(qualified);
        }
    }
    let [qualified] = candidates.as_slice() else {
        return;
    };
    actions.push(code_action(
        &format!("Qualify `{word}` as `{qualified}`"),
        "quickfix",
        uri,
        vec![edit(
            line_range(source, line, start, end),
            qualified.clone(),
        )],
    ));
}

fn recipe_extraction_action(
    source: &str,
    lines: &[&str],
    current: &str,
    document: &ui_lang_core::Document,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some(target) = first_word(current).filter(|target| recipe_target(target)) else {
        return;
    };
    let Some(style_start) = current.rfind(" @") else {
        return;
    };
    let style_end = current[style_start + 1..]
        .find(" ->")
        .map_or(current.len(), |end| style_start + 1 + end);
    let utilities = current[style_start + 2..style_end]
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if utilities.len() < 2 || utilities.iter().any(|utility| utility.contains(['"', '='])) {
        return;
    }
    let occurrences = lines
        .iter()
        .enumerate()
        .filter_map(|(line, candidate)| {
            if first_word(candidate) != Some(target) {
                return None;
            }
            let start = candidate.rfind(" @")?;
            let end = candidate[start + 1..]
                .find(" ->")
                .map_or(candidate.len(), |end| start + 1 + end);
            (candidate[start + 2..end]
                .split_ascii_whitespace()
                .eq(utilities.iter().copied()))
            .then_some((line, start, end))
        })
        .collect::<Vec<_>>();
    if occurrences.len() < 2 {
        return;
    }
    let base = format!("{target}_recipe");
    let name = (1..)
        .map(|index| {
            if index == 1 {
                base.clone()
            } else {
                format!("{base}_{index}")
            }
        })
        .find(|name| document.recipes.iter().all(|recipe| recipe.name != *name))
        .expect("an unused recipe suffix exists");
    let Some(app_line) = lines
        .iter()
        .position(|line| line.starts_with("app ") || line.starts_with("daemon "))
    else {
        return;
    };
    let declaration = format!("recipe {name} for {target}\n  @{}\n", utilities.join(" "));
    actions.push(code_action(
        &format!("Extract utilities to `@{name}`"),
        "refactor.extract",
        uri,
        occurrences
            .into_iter()
            .map(|(line, start, end)| {
                edit(line_range(source, line, start, end), format!(" @{name}"))
            })
            .chain(std::iter::once(block_insertion(
                source,
                lines,
                child_block_end(lines, app_line, 0),
                declaration,
            )))
            .collect(),
    ));
}

fn recipe_target(target: &str) -> bool {
    matches!(
        target,
        "col" | "row" | "flex" | "grid" | "stack" | "box" | "text" | "input" | "button"
    )
}

fn component_handler_event_action(
    source: &str,
    lines: &[&str],
    line: usize,
    current: &str,
    document: &ui_lang_core::Document,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some((component_line, component)) = document.components.iter().find_map(|component| {
        let declaration = component.span.line.checked_sub(1)?;
        (declaration < line && line < child_block_end(lines, declaration, 0))
            .then_some((declaration, component))
    }) else {
        return;
    };
    let in_root = lines[component_line + 1..=line]
        .iter()
        .rev()
        .find(|candidate| !candidate.trim().is_empty() && indentation(candidate) == 2)
        .is_some_and(|ancestor| {
            ancestor.trim() != "state"
                && ancestor.trim() != "emits"
                && !ancestor.trim_start().starts_with("on ")
        });
    if !in_root {
        return;
    }
    let Some(arrow) = current.rfind("->") else {
        return;
    };
    let route = split_words(current[arrow + 2..].trim());
    let Some((handler_name, args)) = route.split_first() else {
        return;
    };
    if handler_name == "emit"
        || component
            .handlers
            .iter()
            .any(|handler| handler.name == *handler_name)
    {
        return;
    }
    let Some(handler) = document
        .handlers
        .iter()
        .find(|handler| handler.name == *handler_name && handler.params.len() == args.len())
    else {
        return;
    };
    let Some(types) = args
        .iter()
        .map(|arg| component_value_type(arg, first_word(current), component))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let calls = lines
        .iter()
        .enumerate()
        .filter(|(_, candidate)| first_word(candidate) == Some(component.name.as_str()))
        .collect::<Vec<_>>();
    let [(call_line, call)] = calls.as_slice() else {
        return;
    };
    if let Some(event) = component
        .events
        .iter()
        .find(|event| event.name == *handler_name)
        && event.payloads != types
    {
        return;
    }

    let call_indent = indentation(call);
    let call_end = child_block_end(lines, *call_line, call_indent);
    let event_block = (*call_line + 1..call_end).find(|index| {
        indentation(lines[*index]) == call_indent + 2 && lines[*index].trim() == "events"
    });
    let existing_route = event_block.and_then(|events| {
        let end = child_block_end(lines, events, call_indent + 2);
        lines[events + 1..end]
            .iter()
            .find(|route| first_word(route) == Some(handler_name))
            .map(|route| route.trim())
    });
    let placeholders = " _".repeat(types.len());
    let expected_route = format!("{handler_name} -> {handler_name}{placeholders}");
    if existing_route.is_some_and(|route| route != expected_route) {
        return;
    }

    let route_start = arrow + 2;
    let handler_start =
        route_start + current[route_start..].len() - current[route_start..].trim_start().len();
    let emitted = std::iter::once(handler_name.as_str())
        .chain(args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(", ");
    let mut edits = vec![edit(
        line_range(source, line, handler_start, current.len()),
        format!("emit({emitted})"),
    )];
    if component
        .events
        .iter()
        .all(|event| event.name != *handler_name)
    {
        let signature = if types.is_empty() {
            handler_name.clone()
        } else {
            format!(
                "{}({})",
                handler_name,
                types
                    .iter()
                    .map(ui_lang_core::Type::display)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let component_end = child_block_end(lines, component_line, 0);
        if let Some(emits) = (component_line + 1..component_end)
            .find(|index| indentation(lines[*index]) == 2 && lines[*index].trim() == "emits")
        {
            edits.push(block_insertion(
                source,
                lines,
                child_block_end(lines, emits, 2),
                format!("    {signature}\n"),
            ));
        } else {
            edits.push(edit(
                insertion_at_line(component_line + 1),
                format!("  emits\n    {signature}\n"),
            ));
        }
    }
    if existing_route.is_none() {
        let (at, route) = if let Some(events) = event_block {
            (
                child_block_end(lines, events, call_indent + 2),
                format!("{}{expected_route}\n", " ".repeat(call_indent + 4)),
            )
        } else {
            (
                call_end,
                format!(
                    "{}events\n{}{expected_route}\n",
                    " ".repeat(call_indent + 2),
                    " ".repeat(call_indent + 4)
                ),
            )
        };
        edits.push(block_insertion(source, lines, at, route));
    }
    actions.push(code_action(
        &format!(
            "Route app handler `{}` through a component event",
            handler.name
        ),
        "refactor.rewrite",
        uri,
        edits,
    ));
}

fn component_value_type(
    source: &str,
    node: Option<&str>,
    component: &ui_lang_core::Component,
) -> Option<ui_lang_core::Type> {
    if source == "_" {
        return match node {
            Some("checkbox" | "toggler") => Some(ui_lang_core::Type::Bool),
            Some("markdown" | "rich-text") => Some(ui_lang_core::Type::Str),
            _ => None,
        };
    }
    component
        .params
        .iter()
        .find(|param| param.name == source)
        .map(|param| param.ty.clone())
        .or_else(|| {
            component
                .states
                .iter()
                .find(|state| state.name == source)
                .map(|state| state.ty.clone())
        })
        .or_else(|| match source {
            "true" | "false" => Some(ui_lang_core::Type::Bool),
            source if quoted_prefix(source) == Some(source) => Some(ui_lang_core::Type::Str),
            source if source.parse::<i64>().is_ok() => Some(ui_lang_core::Type::I64),
            source if source.parse::<f64>().is_ok() => Some(ui_lang_core::Type::F64),
            _ => None,
        })
}

fn bind_declaration_action(
    source: &str,
    line: usize,
    character: usize,
    document: &ui_lang_core::Document,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some(component) = document
        .components
        .iter()
        .find(|component| component.span.line == line + 1)
    else {
        return;
    };
    let Some(selected) = source_line(source, line).and_then(|line| word_at(line, character)) else {
        return;
    };
    let Some(param) = component
        .params
        .iter()
        .find(|param| param.name == selected && !param.bind)
    else {
        return;
    };
    let text = source_line(source, line).unwrap_or("");
    let Some(start) = text.find(&format!("{}:", param.name)) else {
        return;
    };
    actions.push(code_action(
        &format!("Declare `{}` as a bind prop", param.name),
        "quickfix",
        uri,
        vec![edit(line_range(source, line, start, start), "bind ")],
    ));
}

fn component_call_actions(
    source: &str,
    lines: &[&str],
    line: usize,
    document: &ui_lang_core::Document,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some(name) = first_word(lines[line]) else {
        return;
    };
    let Some(component) = document
        .components
        .iter()
        .find(|component| component.name == name)
    else {
        return;
    };
    let text = lines[line];
    for param in &component.params {
        let (wrong, right) = if param.bind {
            ("=", "<->")
        } else {
            ("<->", "=")
        };
        let pattern = format!("{}{wrong}", param.name);
        let Some(start) = text.find(&pattern).map(|start| start + param.name.len()) else {
            continue;
        };
        actions.push(code_action(
            &format!(
                "Use `{}` for {} prop `{}`",
                right,
                if param.bind { "bind" } else { "read" },
                param.name
            ),
            "quickfix",
            uri,
            vec![edit(
                line_range(source, line, start, start + wrong.len()),
                right,
            )],
        ));
    }

    if component.events.is_empty() {
        return;
    }
    let indent = indentation(text);
    let end = child_block_end(lines, line, indent);
    let event_block = (line + 1..end)
        .find(|index| indentation(lines[*index]) == indent + 2 && lines[*index].trim() == "events");
    let existing = event_block
        .map(|events| {
            let event_end = child_block_end(lines, events, indent + 2);
            lines[events + 1..event_end]
                .iter()
                .filter_map(|line| first_word(line))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let missing = component
        .events
        .iter()
        .filter(|event| !existing.contains(event.name.as_str()))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return;
    }
    let (at, text) = if let Some(events) = event_block {
        let at = child_block_end(lines, events, indent + 2);
        let text = missing
            .iter()
            .map(|event| {
                format!(
                    "{}{} -> {}\n",
                    " ".repeat(indent + 4),
                    event.name,
                    event.name
                )
            })
            .collect::<String>();
        (at, text)
    } else {
        let text = format!(
            "{}events\n{}",
            " ".repeat(indent + 2),
            missing
                .iter()
                .map(|event| format!(
                    "{}{} -> {}\n",
                    " ".repeat(indent + 4),
                    event.name,
                    event.name
                ))
                .collect::<String>()
        );
        (end, text)
    };
    actions.push(code_action(
        "Add missing component event routes",
        "quickfix",
        uri,
        vec![block_insertion(source, lines, at, text)],
    ));
}

fn block_insertion(source: &str, lines: &[&str], at: usize, text: String) -> Value {
    if at < lines.len() {
        edit(insertion_at_line(at), text)
    } else {
        let prefix = if source.ends_with('\n') { "" } else { "\n" };
        edit(whole_document_end(source), format!("{prefix}{text}"))
    }
}

fn route_handler(line: &str) -> Option<(&str, usize)> {
    let route = line.split_once("->")?.1.trim();
    let success = route.split('|').next()?.trim();
    let mut words = success.split_ascii_whitespace();
    let handler = words.next()?;
    if handler == "_" {
        return None;
    }
    let payloads = words.filter(|word| *word == "_").count();
    Some((handler, payloads))
}

fn handler_skeleton_action(
    source: &str,
    _line: usize,
    current: &str,
    document: &ui_lang_core::Document,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some((handler, payloads)) = route_handler(current) else {
        return;
    };
    if document.handlers.iter().any(|item| item.name == handler) {
        return;
    }
    let skeleton = handler_skeleton(source, handler, payloads);
    actions.push(code_action(
        &format!("Create handler `{handler}`"),
        "quickfix",
        uri,
        vec![edit(whole_document_end(source), skeleton)],
    ));
}

fn handler_skeleton(source: &str, handler: &str, payloads: usize) -> String {
    let parameters = (0..payloads)
        .map(|index| {
            if index == 0 {
                "value".into()
            } else {
                format!("value{}", index + 1)
            }
        })
        .collect::<Vec<String>>()
        .join(", ");
    let prefix = if source.ends_with('\n') { "\n" } else { "\n\n" };
    if parameters.is_empty() {
        format!("{prefix}on {handler}\n  return if true\n")
    } else {
        format!("{prefix}on {handler}({parameters})\n  return if true\n")
    }
}

fn whole_document_end(source: &str) -> Value {
    let range = whole_document_range(source);
    json!({ "start": range["end"], "end": range["end"] })
}

fn fallible_route_action(
    source: &str,
    line: usize,
    current: &str,
    document: &ui_lang_core::Document,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    if current.contains('|') {
        return;
    }
    let trimmed = current.trim();
    let Some(call) = ["run ", "task ", "stream "]
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
    else {
        return;
    };
    let call = call.strip_prefix("latest ").unwrap_or(call);
    let Some((function_name, _)) = call.split_once('(') else {
        return;
    };
    let function_name = function_name.trim();
    let Some(function) = document
        .functions
        .iter()
        .find(|function| function.name == function_name && function.error.is_some())
    else {
        return;
    };
    let handler = format!("{}_failed", function.name);
    let mut edits = vec![edit(
        line_range(source, line, current.len(), current.len()),
        format!(" | {handler} _"),
    )];
    if !document.handlers.iter().any(|item| item.name == handler) {
        edits.push(edit(
            whole_document_end(source),
            handler_skeleton(source, &handler, 1),
        ));
    }
    actions.push(code_action(
        &format!("Add error route for `{function_name}`"),
        "quickfix",
        uri,
        edits,
    ));
}

fn button_label_action(
    source: &str,
    lines: &[&str],
    line: usize,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let current = lines[line];
    let trimmed = current.trim();
    let Some(after_button) = trimmed.strip_prefix("button") else {
        return;
    };
    if after_button.trim_start().starts_with('"') || current.contains(" label=") {
        return;
    }
    let indent = indentation(current);
    let Some(child) = lines[line + 1..]
        .iter()
        .find(|candidate| !candidate.trim().is_empty())
    else {
        return;
    };
    if indentation(child) <= indent {
        return;
    }
    let label = child
        .trim()
        .strip_prefix("text ")
        .filter(|text| text.starts_with('"'))
        .and_then(quoted_prefix)
        .unwrap_or("\"Button\"");
    let metadata_end = [current.find(" @"), current.find(" ->")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(current.len());
    actions.push(code_action(
        "Add an accessible label to child-content button",
        "quickfix",
        uri,
        vec![edit(
            line_range(source, line, metadata_end, metadata_end),
            format!(" label={label}"),
        )],
    ));
}

fn quoted_prefix(source: &str) -> Option<&str> {
    let mut escaped = false;
    for (index, ch) in source.char_indices().skip(1) {
        if ch == '"' && !escaped {
            return Some(&source[..=index]);
        }
        escaped = ch == '\\' && !escaped;
        if ch != '\\' {
            escaped = false;
        }
    }
    None
}

fn with_block_action(
    source: &str,
    line: usize,
    current: &str,
    uri: &str,
    actions: &mut Vec<Value>,
) {
    let Some(replacement) = with_block(current) else {
        return;
    };
    actions.push(code_action(
        "Convert long node metadata to a `with` block",
        "refactor.rewrite",
        uri,
        vec![edit(
            line_range(source, line, 0, current.len()),
            replacement,
        )],
    ));
}

fn with_block(line: &str) -> Option<String> {
    let indent = indentation(line);
    let trimmed = line.trim();
    let (node, route) = trimmed
        .split_once(" -> ")
        .map_or((trimmed, None), |(node, route)| (node, Some(route)));
    let words = split_words(node);
    let mut head = Vec::new();
    let mut metadata = Vec::new();
    let mut utilities = false;
    for word in words {
        if word.starts_with('@') {
            utilities = true;
            metadata.push(word);
        } else if utilities {
            metadata.push(format!("@{word}"));
        } else if word.contains('=') || word.contains("<->") {
            metadata.push(word);
        } else {
            head.push(word);
        }
    }
    if metadata.len() < 3 && line.chars().count() <= 100 {
        return None;
    }
    if metadata.is_empty() || !first_word(trimmed).is_some_and(is_view_node) {
        return None;
    }
    let mut output = format!("{}{}", " ".repeat(indent), head.join(" "));
    if let Some(route) = route {
        output.push_str(" -> ");
        output.push_str(route);
    }
    output.push('\n');
    output.push_str(&" ".repeat(indent + 2));
    output.push_str("with\n");
    for item in metadata {
        output.push_str(&" ".repeat(indent + 4));
        output.push_str(&item);
        output.push('\n');
    }
    Some(output.trim_end_matches('\n').to_owned())
}

fn split_words(source: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut start = 0;
    let mut quote = false;
    let mut depth = 0usize;
    for (index, ch) in source.char_indices() {
        match ch {
            '"' => quote = !quote,
            '(' | '[' if !quote => depth += 1,
            ')' | ']' if !quote => depth = depth.saturating_sub(1),
            _ if ch.is_whitespace() && !quote && depth == 0 => {
                if start < index {
                    words.push(source[start..index].to_owned());
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start < source.len() {
        words.push(source[start..].to_owned());
    }
    words
}

fn is_view_node(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
        || matches!(
            name,
            "row"
                | "col"
                | "flex"
                | "grid"
                | "stack"
                | "scroll"
                | "box"
                | "text"
                | "input"
                | "button"
                | "checkbox"
                | "toggler"
                | "slider"
                | "progress"
                | "radio"
                | "pick"
                | "combo"
                | "rule"
                | "qr"
                | "space"
                | "markdown"
                | "editor"
                | "image"
                | "svg"
                | "viewer"
                | "tooltip"
                | "mouse"
                | "resize-handle"
                | "theme"
                | "float"
                | "pin"
                | "sensor"
                | "responsive"
        )
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
        let source_name = symbol.name.rsplit("::").next()?;
        let source_renamed = renamed.rsplit("::").next()?;
        for range in std::iter::once(&symbol.definition).chain(&symbol.references) {
            let (uri, source) = range_document(documents, range, &navigation.root_uri)?;
            let start = range.start_column.checked_sub(1)?;
            let length = range.end_column.checked_sub(range.start_column)?;
            let line = source_line(&source, range.line.checked_sub(1)?)?;
            if !line
                .chars()
                .skip(start)
                .take(length)
                .eq(source_name.chars())
            {
                return None;
            }
            changes.entry(uri).or_default().push(json!({
                "range": source_range(&source, range)?,
                "newText": source_renamed,
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
        Navigation, code_actions_at, completion_items_at, diagnostic_range, file_path_uri,
        file_uri_path, hover_at, navigation_at, read_message, serve, signature_help_at,
        source_range, whole_document_range, workspace_edit,
    };
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use std::fs;
    use std::io::{BufReader, Cursor};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const APP_WITH_PART: &str = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Broken\n";

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

    fn apply_action(source: &str, action: &Value, uri: &str) -> String {
        let offset = |position: &Value| {
            let line = usize::try_from(position["line"].as_u64().unwrap()).unwrap();
            let character = usize::try_from(position["character"].as_u64().unwrap()).unwrap();
            source
                .split_inclusive('\n')
                .take(line)
                .map(str::len)
                .sum::<usize>()
                + character
        };
        let mut edits = action["edit"]["changes"][uri]
            .as_array()
            .unwrap()
            .iter()
            .map(|edit| {
                (
                    offset(&edit["range"]["start"]),
                    offset(&edit["range"]["end"]),
                    edit["newText"].as_str().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        let mut output = source.to_owned();
        for (start, end, text) in edits {
            output.replace_range(start..end, text);
        }
        output
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
        assert_eq!(capabilities["hoverProvider"], true);
        assert_eq!(
            capabilities["signatureHelpProvider"]["triggerCharacters"],
            json!([" ", "=", "<"])
        );
        assert_eq!(capabilities["codeActionProvider"], true);
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
                "params": { "textDocument": { "uri": uri, "text": "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  wat\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri },
                    "contentChanges": [{ "text": "app Demo\ntheme contract AppTheme\n    bg\n    fg\n    primary\n    danger\npalette app for AppTheme\n    bg #000000\n    fg #ffffff\n    primary #333333\n    danger #ff0000\nview\n    text \"Hi\"\n" }],
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
                "start": { "line": 12, "character": 0 },
                "end": { "line": 12, "character": 1 },
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
        let source = "app Demo\ntheme contract AppTheme\n    bg\n    fg\n    primary\n    danger\npalette app for AppTheme\n    bg #000000\n    fg #ffffff\n    primary #333333\n    danger #ff0000\nview\n    text \"😀\"";
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
                "params": { "textDocument": { "uri": uri }, "position": { "line": 12, "character": 0 } },
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
                "end": { "line": 12, "character": 13 },
            })
        );
        let completions = response(&messages, 3)["result"].as_array().unwrap();
        assert!(completions.iter().any(|item| item["label"] == "text"));
        assert!(completions.iter().any(|item| item["label"] == "button"));
        assert!(!completions.iter().any(|item| item["label"] == "state"));
        assert!(!completions.iter().any(|item| item["label"] == "run"));
    }

    #[test]
    fn completion_uses_cursor_and_checked_contract_context() {
        let uri = "file:///tmp/context.ice";
        let source = "app Demo\nextern crate::backend\n  load(query:str) -> str ! str\n  task save(value:str) -> bool\n  stream changes() -> str\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  title = \"Draft\"\non submit\n  return if true\ncomponent Card(bind title:str, tone:str=\"quiet\")\n  emits\n    select(str)\n  col\n    slot Header\n    button label=title -> emit(select, title)\n      text title\n      active bg=primary\nview\n  Card title<->title\n    events\n      select -> selected _\n";
        assert!(
            ui_lang_core::parse(source).is_ok(),
            "{:?}",
            ui_lang_core::parse(source)
        );
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let complete = |line| {
            completion_items_at(
                &documents,
                &json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": 2 },
                }),
            )
            .unwrap()
        };

        let handler = complete(18);
        assert!(handler.iter().any(|item| {
            item["label"] == "load"
                && item["insertText"]
                    .as_str()
                    .unwrap()
                    .starts_with("run load(")
        }));
        assert!(handler.iter().any(|item| {
            item["label"] == "save"
                && item["insertText"]
                    .as_str()
                    .unwrap()
                    .starts_with("task save(")
        }));
        assert!(handler.iter().any(|item| {
            item["label"] == "changes"
                && item["insertText"]
                    .as_str()
                    .unwrap()
                    .starts_with("stream changes(")
        }));
        assert!(!handler.iter().any(|item| item["label"] == "button"));

        let top_level = complete(0);
        assert!(top_level.iter().any(|item| item["label"] == "component"));
        assert!(top_level.iter().any(|item| item["label"] == "state"));
        assert!(!top_level.iter().any(|item| item["label"] == "button"));
        assert!(!top_level.iter().any(|item| item["label"] == "run"));

        let status = complete(26);
        for label in ["active", "hovered", "pressed", "disabled"] {
            assert!(status.iter().any(|item| item["label"] == label));
        }
        assert!(!status.iter().any(|item| item["label"] == "opened"));
        assert!(!status.iter().any(|item| item["label"] == "button"));

        let component = complete(28);
        let item = |label| {
            component
                .iter()
                .find(|item| item["label"] == label)
                .unwrap_or_else(|| panic!("missing {label}"))
        };
        assert_eq!(item("title<->")["detail"], "bind str");
        assert_eq!(item("tone=")["detail"], "read str (default)");
        assert_eq!(item("Header:")["detail"], "component slot");
        assert!(item("select")["detail"].as_str().unwrap().contains("str"));
        assert!(!component.iter().any(|item| item["label"] == "button"));

        let signature = signature_help_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "position": { "line": 28, "character": 22 },
            }),
        )
        .unwrap();
        assert_eq!(
            signature["signatures"][0]["label"],
            "Card(bind title:str, tone:str=<default>)"
        );
        assert_eq!(signature["activeParameter"], 0);
        assert!(
            signature["signatures"][0]["documentation"]["value"]
                .as_str()
                .unwrap()
                .contains("event select(str)")
        );
    }

    #[test]
    fn completion_tracks_match_arms_optional_slots_and_theme_contracts() {
        let uri = "file:///tmp/new-contexts.ice";
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  choice:str? = none\ncomponent Card()\n  col\n    slot Header\n    slot Footer?\nview\n  col\n    Card\n    match choice\n      some(value)\n        text value\n";
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let complete = |line, character| {
            completion_items_at(
                &documents,
                &json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                }),
            )
            .unwrap()
        };

        assert!(complete(2, 2).is_empty());
        let component = complete(19, 6);
        assert_eq!(
            component
                .iter()
                .find(|item| item["label"] == "Footer:")
                .unwrap()["detail"],
            "optional component slot"
        );
        let hover = hover_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "position": { "line": 13, "character": 12 },
            }),
        )
        .unwrap();
        assert!(
            hover["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("slots: Header, Footer?")
        );
        let patterns = complete(21, 8);
        assert!(patterns.iter().any(|item| item["label"] == "none"));
        assert!(!patterns.iter().any(|item| item["label"] == "some(value)"));
        assert!(!patterns.iter().any(|item| item["label"] == "text"));
    }

    #[test]
    fn completes_nominal_palette_values_and_match_arms() {
        let uri = "file:///tmp/palette-context.ice";
        let source = "app Demo\n  palette active_palette\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette light for AppTheme\n  bg #ffffff\n  fg #111111\n  primary #3366ff\n  danger #cc3344\npalette dark for AppTheme\n  bg #111111\n  fg #ffffff\n  primary #88aaff\n  danger #ff6677\nstate\n  active_palette:palette[AppTheme] = AppTheme.light\nview\n  match active_palette\n    AppTheme.light\n      text \"Light\"\n";
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let complete = |line| {
            completion_items_at(
                &documents,
                &json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": 8 },
                }),
            )
            .unwrap()
        };
        let state_line = source
            .lines()
            .position(|line| line.contains("active_palette:palette"))
            .unwrap();
        let values = complete(state_line);
        assert!(values.iter().any(|item| item["label"] == "AppTheme.light"));
        assert!(values.iter().any(|item| item["label"] == "AppTheme.dark"));

        let arm_line = source.lines().count();
        let source = format!("{source}    \n");
        let documents = HashMap::from([(uri.to_owned(), source)]);
        let patterns = completion_items_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "position": { "line": arm_line, "character": 4 },
            }),
        )
        .unwrap();
        assert!(patterns.iter().any(|item| item["label"] == "AppTheme.dark"));
        assert!(
            !patterns
                .iter()
                .any(|item| item["label"] == "AppTheme.light")
        );
    }

    #[test]
    fn hover_shows_component_contract_and_flattened_recipe() {
        let uri = "file:///tmp/hover.ice";
        let source = "app Demo\nrecipe action for button\n  @px-4 rounded-md\nrecipe danger for button extends action\n  @bg-danger\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Card(bind title:str, tone:str=\"quiet\") -> bool\n  emits\n    select(str)\n  col\n    slot Header\n    button label=title -> emit(select, title)\n      text title\nview\n  text \"Demo\"\n";
        assert!(
            ui_lang_core::analyze(source).is_ok(),
            "{:?}",
            ui_lang_core::analyze(source)
        );
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let hover = |line, character| {
            hover_at(
                &documents,
                &json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                }),
            )
            .unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .to_owned()
        };

        let component = hover(15, 12);
        assert!(component.contains("title: bind str"));
        assert!(component.contains("tone: read str = <default>"));
        assert!(component.contains("output: bool"));
        assert!(component.contains("event select(str)"));
        assert!(component.contains("slots: Header"));

        let recipe = hover(3, 10);
        assert!(recipe.contains("@danger for button extends @action"));
        assert!(recipe.contains("@px-4 @rounded-md @bg-danger"));
    }

    #[test]
    fn code_actions_return_workspace_edits_for_component_contracts() {
        let uri = "file:///tmp/actions.ice";
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  title = \"Draft\"\n  tone = \"quiet\"\ncomponent Card(bind title:str, tone:str)\n  emits\n    select(str)\n  text title\nview\n  Card title=title tone<->tone\n";
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let actions = |line, character| {
            code_actions_at(
                &documents,
                &json!({
                    "textDocument": { "uri": uri },
                    "range": {
                        "start": { "line": line, "character": character },
                        "end": { "line": line, "character": character },
                    },
                    "context": { "diagnostics": [] },
                }),
            )
            .unwrap()
        };

        let declaration = actions(14, 35);
        let bind = declaration
            .iter()
            .find(|action| action["title"] == "Declare `tone` as a bind prop")
            .unwrap();
        assert_eq!(bind["edit"]["changes"][uri][0]["newText"], "bind ");

        let call = actions(19, 8);
        let titles = call
            .iter()
            .filter_map(|action| action["title"].as_str())
            .collect::<Vec<_>>();
        assert!(titles.contains(&"Use `<->` for bind prop `title`"));
        assert!(titles.contains(&"Use `=` for read prop `tone`"));
        assert!(titles.contains(&"Add missing component event routes"));
        let events = call
            .iter()
            .find(|action| action["title"] == "Add missing component event routes")
            .unwrap();
        assert!(
            events["edit"]["changes"][uri][0]["newText"]
                .as_str()
                .unwrap()
                .contains("select -> select")
        );
    }

    #[test]
    fn code_action_adds_all_missing_typed_match_arms() {
        let prefix = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";
        for (declaration, expected) in [
            (
                "state\n  value:str? = none\nview\n  match value\n    some(value)\n      text value\n",
                "none\n      space",
            ),
            (
                "state\n  value:result[str,str] = ok(\"yes\")\nview\n  match value\n    ok(value)\n      text value\n",
                "err(error)\n      space",
            ),
            (
                "enum Screen\n  home\n  settings(str)\nstate\n  value:Screen = Screen.home\nview\n  match value\n    Screen.home\n      text \"home\"\n",
                "Screen.settings(value)\n      space",
            ),
        ] {
            let source = format!("{prefix}{declaration}");
            let uri = "file:///tmp/exhaustive.ice";
            let line = source
                .lines()
                .position(|line| line.trim_start().starts_with("match "))
                .unwrap();
            let documents = HashMap::from([(uri.to_owned(), source.clone())]);
            let actions = code_actions_at(
                &documents,
                &json!({
                    "textDocument": { "uri": uri },
                    "range": {
                        "start": { "line": line, "character": 4 },
                        "end": { "line": line, "character": 4 },
                    },
                    "context": { "diagnostics": [] },
                }),
            )
            .unwrap();
            let action = actions
                .iter()
                .find(|action| action["title"] == "Add all missing typed match arms")
                .unwrap();
            let output = apply_action(&source, action, uri);
            assert!(output.contains(expected), "{output}");
            ui_lang_core::analyze(&output).unwrap();
        }
    }

    #[test]
    fn code_action_can_replace_a_selected_match_wildcard() {
        let uri = "file:///tmp/wildcard.ice";
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nenum Screen\n  home\n  settings(str)\nstate\n  value:Screen = Screen.home\nview\n  match value\n    Screen.home\n      text \"home\"\n    _\n      text \"fallback\"\n";
        let line = source.lines().position(|line| line.trim() == "_").unwrap();
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": line, "character": 4 },
                    "end": { "line": line, "character": 4 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        let action = actions
            .iter()
            .find(|action| action["title"] == "Replace wildcard with all missing typed match arms")
            .unwrap();
        let output = apply_action(source, action, uri);
        assert!(!output.contains("\n    _\n"));
        assert!(output.contains("Screen.settings(value)\n      text \"fallback\""));
        assert!(output.contains("Screen.home\n      text \"home\""));
        ui_lang_core::analyze(&output).unwrap();
    }

    #[test]
    fn code_action_qualifies_only_the_one_import_alias_that_checks() {
        let fixture = Fixture::new();
        fixture.write(
            "ui.ice",
            "extern crate::backend\n  sync label(value:str) -> str\nenum Mode\n  idle\nrecipe panel for text\n  @text-fg\ncomponent Card()\n  text \"Card\"\n",
        );
        let prefix = "app Demo\nuse \"ui.ice\" as ui\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";
        for (name, suffix, selected) in [
            ("component", "view\n  Card\n", "Card"),
            ("recipe", "view\n  text \"Demo\" @panel\n", "@panel"),
            ("extern", "view\n  text label(\"Demo\")\n", "label"),
            (
                "type",
                "state\n  mode:Mode = ui::Mode.idle\nview\n  text \"Demo\"\n",
                "Mode",
            ),
        ] {
            let source = format!("{prefix}{suffix}");
            let path = fixture.path(&format!("{name}.ice"));
            fixture.write(&format!("{name}.ice"), &source);
            let uri = file_path_uri(&path);
            let (line, column) = source
                .lines()
                .enumerate()
                .find_map(|(line, text)| {
                    text.find(selected)
                        .map(|column| (line, column + selected.len()))
                })
                .unwrap();
            let documents = HashMap::from([(uri.clone(), source.clone())]);
            let actions = code_actions_at(
                &documents,
                &json!({
                    "textDocument": { "uri": uri },
                    "range": {
                        "start": { "line": line, "character": column },
                        "end": { "line": line, "character": column },
                    },
                    "context": { "diagnostics": [] },
                }),
            )
            .unwrap();
            let action = actions
                .iter()
                .find(|action| {
                    action["title"]
                        .as_str()
                        .is_some_and(|title| title.starts_with(&format!("Qualify `{selected}`")))
                })
                .unwrap_or_else(|| panic!("missing qualification for {name}: {actions:?}"));
            let output = apply_action(&source, action, &uri);
            let qualified = if selected.starts_with('@') {
                format!("@ui::{}", selected.trim_start_matches('@'))
            } else {
                format!("ui::{selected}")
            };
            assert!(output.contains(&qualified), "{output}");
            let overlays = HashMap::from([(path, output)]);
            ui_lang_core::analyze_file_with_overlays(
                fixture.path(&format!("{name}.ice")),
                &overlays,
            )
            .unwrap();
        }
    }

    #[test]
    fn code_action_uses_an_unsaved_import_fragment() {
        let fixture = Fixture::new();
        let source = "app Demo\nuse \"ui.ice\" as ui\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        fixture.write("app.ice", source);
        fixture.write("ui.ice", "component Saved()\n  text \"Saved\"\n");
        let root = fixture.path("app.ice");
        let part = fixture.path("ui.ice");
        let uri = file_path_uri(&root);
        let part_uri = file_path_uri(&part);
        let documents = HashMap::from([
            (uri.clone(), source.to_owned()),
            (
                part_uri,
                "component Card()\n  text \"Unsaved\"\n".to_owned(),
            ),
        ]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 13, "character": 6 },
                    "end": { "line": 13, "character": 6 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        let action = actions
            .iter()
            .find(|action| action["title"] == "Qualify `Card` as `ui::Card`")
            .unwrap_or_else(|| panic!("missing qualification from unsaved import: {actions:?}"));
        let output = apply_action(source, action, &uri);
        let overlays = HashMap::from([
            (root.clone(), output),
            (part, "component Card()\n  text \"Unsaved\"\n".to_owned()),
        ]);
        ui_lang_core::analyze_file_with_overlays(root, &overlays).unwrap();
    }

    #[test]
    fn code_action_omits_ambiguous_import_qualification() {
        let fixture = Fixture::new();
        fixture.write("ui.ice", "component Card()\n  text \"Card\"\n");
        let source = "app Demo\nuse \"ui.ice\" as first\nuse \"ui.ice\" as second\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
        let path = fixture.path("app.ice");
        fixture.write("app.ice", source);
        let uri = file_path_uri(&path);
        let line = source
            .lines()
            .position(|line| line.trim() == "Card")
            .unwrap();
        let documents = HashMap::from([(uri.clone(), source.to_owned())]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": line, "character": 6 },
                    "end": { "line": line, "character": 6 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        assert!(!actions.iter().any(|action| {
            action["title"]
                .as_str()
                .is_some_and(|title| title.starts_with("Qualify `Card`"))
        }));
    }

    #[test]
    fn code_actions_cover_handlers_errors_accessibility_and_long_nodes() {
        let uri = "file:///tmp/more-actions.ice";
        let source = "app Demo\nextern crate::backend\n  load(query:str) -> str ! str\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non loaded(value)\n  return if true\non submit\n  run load(\"x\") -> loaded _\nview\n  col\n    button #go w=fill h=40.0 p=8.0 disabled=false @w-full px-4 rounded-2 -> missing _\n      text \"Go\"\n";
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 16, "character": 4 },
                    "end": { "line": 16, "character": 4 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        let error = actions
            .iter()
            .find(|action| action["title"] == "Add error route for `load`")
            .unwrap();
        assert_eq!(
            error["edit"]["changes"][uri][0]["newText"],
            " | load_failed _"
        );

        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 19, "character": 8 },
                    "end": { "line": 19, "character": 8 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        let titles = actions
            .iter()
            .filter_map(|action| action["title"].as_str())
            .collect::<Vec<_>>();
        assert!(titles.contains(&"Create handler `missing`"));
        assert!(titles.contains(&"Add an accessible label to child-content button"));
        assert!(titles.contains(&"Convert long node metadata to a `with` block"));
        let label = actions
            .iter()
            .find(|action| action["title"] == "Add an accessible label to child-content button")
            .unwrap();
        assert_eq!(label["edit"]["changes"][uri][0]["newText"], " label=\"Go\"");
    }

    #[test]
    fn code_action_extracts_inline_utilities_into_a_recipe() {
        let uri = "file:///tmp/extract-recipe.ice";
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non save\n  return if true\nview\n  col\n    button \"Save\" @px-4 py-3 -> save\n    button \"Save again\" @px-4 py-3 -> save\n";
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 15, "character": 4 },
                    "end": { "line": 15, "character": 4 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        let action = actions
            .iter()
            .find(|action| action["title"] == "Extract utilities to `@button_recipe`")
            .unwrap();

        let output = apply_action(source, action, uri);
        assert!(output.contains("recipe button_recipe for button\n  @px-4 py-3\n"));
        assert_eq!(output.matches("@button_recipe -> save").count(), 2);
        assert_eq!(output.matches("@px-4 py-3").count(), 1);
        ui_lang_core::analyze(&output).unwrap();

        let single = source.replace("    button \"Save again\" @px-4 py-3 -> save\n", "");
        let documents = HashMap::from([(uri.to_owned(), single)]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 15, "character": 4 },
                    "end": { "line": 15, "character": 4 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        assert!(
            !actions
                .iter()
                .any(|action| action["title"] == "Extract utilities to `@button_recipe`")
        );
    }

    #[test]
    fn code_action_closes_a_component_over_one_unambiguous_call_site() {
        let uri = "file:///tmp/close-component.ice";
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\ncomponent Nav(page:str)\n  button \"Open\" -> navigate page\non navigate(page)\n  return if true\nview\n  Nav page=\"home\"\n";
        let documents = HashMap::from([(uri.to_owned(), source.to_owned())]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 12, "character": 20 },
                    "end": { "line": 12, "character": 20 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        let action = actions
            .iter()
            .find(|action| {
                action["title"] == "Route app handler `navigate` through a component event"
            })
            .unwrap();

        let output = apply_action(source, action, uri);
        assert!(output.contains("component Nav(page:str)\n  emits\n    navigate(str)"));
        assert!(output.contains("button \"Open\" -> emit(navigate, page)"));
        assert!(output.contains("events\n      navigate -> navigate _"));
        ui_lang_core::analyze(&output).unwrap();

        let ambiguous = source.replace(
            "view\n  Nav page=\"home\"",
            "view\n  col\n    Nav page=\"home\"\n    Nav page=\"other\"",
        );
        let documents = HashMap::from([(uri.to_owned(), ambiguous)]);
        let actions = code_actions_at(
            &documents,
            &json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 12, "character": 20 },
                    "end": { "line": 12, "character": 20 },
                },
                "context": { "diagnostics": [] },
            }),
        )
        .unwrap();
        assert!(
            !actions.iter().any(|action| action["title"]
                == "Route app handler `navigate` through a component event")
        );
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
        assert!(
            completion("theme contract")["insertText"]
                .as_str()
                .unwrap()
                .starts_with("theme contract ${1:Name}")
        );
        assert!(
            completion("palette")["insertText"]
                .as_str()
                .unwrap()
                .starts_with("palette ${1:light} for ${2:Name}")
        );
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
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nstate\n  count = 0\non increment\n  count = count + 1\nview\n  text count\ntest dispatches\n  dispatch increment\n";
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
                "params": { "textDocument": { "uri": uri }, "position": { "line": 18, "character": 12 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 18, "character": 12 }, "newName": "bump" },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["result"]["uri"], uri);
        assert_eq!(
            response(&messages, 2)["result"]["range"]["start"],
            json!({ "line": 13, "character": 3 })
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
            [13, 18]
        );
    }

    #[test]
    fn test_target_rename_stays_inside_one_test_scope() {
        let uri = "file:///tmp/test-target-navigation.ice";
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\non clicked\nview\n  col #root\n    button \"Go\" #action -> clicked\ntest first\n  target root = #root\n  target action = root/action\n  expect root.width == root.height\n  click action\ntest second\n  target root = #root\n  expect root.visible\n";
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
                "params": { "textDocument": { "uri": uri }, "position": { "line": 18, "character": 10 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 18, "character": 10 }, "newName": "surface" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 18, "character": 10 }, "newName": "action" },
            }),
            json!({ "jsonrpc": "2.0", "id": 5, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(
            response(&messages, 2)["result"]["range"]["start"],
            json!({ "line": 16, "character": 9 })
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
            [16, 17, 18, 18]
        );
        assert_eq!(response(&messages, 4)["error"]["code"], -32602);
    }

    #[test]
    fn defines_and_renames_test_aliases_inside_dynamic_target_keys() {
        let uri = "file:///tmp/test-target-key-navigation.ice";
        let source = "app Demo\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  col #root\n    text \"Key\" #key\n    text \"Item\" #item(\"Key\")\ntest dynamic_targets\n  target key = #root/key\n  target item = #root/item(key.value)\n  expect exists #root/item(key.value)\n  expect text \"Item\" within #root/item(key.value)\n  click #root/item(key.value)\n";
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
                "params": { "textDocument": { "uri": uri }, "position": { "line": 17, "character": 28 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 17, "character": 28 }, "newName": "lookup" },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(
            response(&messages, 2)["result"]["range"]["start"],
            json!({ "line": 16, "character": 9 })
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
            [16, 17, 18, 19, 20]
        );
    }

    #[test]
    fn defines_and_safely_renames_checked_symbols_across_imports() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n    events\n      click -> clicked\n";
        let other = "app Other\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n    events\n      click -> clicked\n";
        let part = "component Card()\n  emits\n    click\n  button \"😀\" -> emit(click)\ncomponent Panel()\n  text \"Other\"\non clicked\non mount\n";
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
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 15, "character": 18 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "Tile" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "Panel" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 15, "character": 18 }, "newName": "activated" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "bad-name" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "tile" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 15, "character": 18 }, "newName": "mount" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": part_uri }, "position": { "line": 7, "character": 4 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": part_uri }, "position": { "line": 7, "character": 4 }, "newName": "launched" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 12,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "Tile.Header" },
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
                "start": { "line": 15, "character": 15 },
                "end": { "line": 15, "character": 22 },
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
            1
        );
        assert_eq!(
            response(&messages, 6)["result"]["changes"][&root_uri]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            response(&messages, 6)["result"]["changes"][&other_uri]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(response(&messages, 7)["error"]["code"], -32602);
        assert_eq!(response(&messages, 8)["error"]["code"], -32602);
        assert_eq!(response(&messages, 9)["error"]["code"], -32602);
        assert_eq!(response(&messages, 10)["result"], Value::Null);
        assert_eq!(response(&messages, 11)["error"]["code"], -32602);
        assert_eq!(response(&messages, 12)["error"]["code"], -32602);
    }

    #[test]
    fn renames_the_source_name_of_an_aliased_component() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\" as ui\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  ui::Card\n";
        let part = "component Card()\n  text \"Card\"\n";
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
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 6 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 6 }, "newName": "Tile" },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, 2)["result"]["placeholder"], "Card");
        assert_eq!(
            response(&messages, 3)["result"]["changes"][&root_uri][0]["newText"],
            "Tile"
        );
        assert_eq!(
            response(&messages, 3)["result"]["changes"][&part_uri][0]["newText"],
            "Tile"
        );
    }

    #[test]
    fn defines_and_renames_imported_style_recipes() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"recipes.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\n  surface\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n  surface #111111\nview\n  box @panel\n    text \"Panel\"\n";
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
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 15, "character": 8 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 15, "character": 8 }, "newName": "surface_panel" },
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
        let root = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
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
            "position": { "line": 13, "character": 3 },
        });

        let navigation = navigation_at(&documents, &[], &params).unwrap();

        assert!(navigation.symbol.definition.path.is_some());
        assert!(!navigation.renameable());
    }

    #[test]
    fn imported_rename_accepts_a_new_root_inside_the_workspace() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
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
            "position": { "line": 13, "character": 3 },
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
        let workspace_app = "app Workspace\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Workspace\"\n";
        let outside_app = "app Outside\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
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
            "position": { "line": 13, "character": 3 },
        });

        let navigation = navigation_at(&documents, &[workspace], &params).unwrap();

        assert_eq!(navigation.symbol.name, "Card");
        assert!(!navigation.renameable());
    }

    #[test]
    fn open_fragment_new_facts_participate_in_rename_checks() {
        let fixture = Fixture::new();
        let root = "app Demo\nuse \"part.ice\"\nuse \"extra.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
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
            "position": { "line": 14, "character": 3 },
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
        let root = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Dialog\n    Dialog.Header\n";
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
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "Modal" },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 14, "character": 8 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 14, "character": 8 }, "newName": "Modal.Header" },
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
        let root = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
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
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "Tile" },
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
        let root = "app Demo\nuse \"part.ice\"\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Card\n";
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
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/prepareRename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "textDocument/rename",
                "params": { "textDocument": { "uri": root_uri }, "position": { "line": 13, "character": 3 }, "newName": "Tile" },
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
