mod analysis_db;
mod api;
mod ast;
mod check;
mod codegen;
mod editor;
mod format;
mod hir;
mod lower;
mod parser;
mod semantic;
mod source;
#[cfg(test)]
mod test_support;

pub use analysis_db::{
    AnalysisConfig, AnalysisDb, AnalysisInvalidation, AnalysisMetrics, AnalysisTimings,
    CompilerFeatureSet, ContentHash, FileKey, LANGUAGE_REVISION, ValidatedSource, ValidationPolicy,
};
pub use api::*;
pub use ast::*;
pub use editor::{
    CursorContext, STYLE_STATUS_NAMES, SourcePosition, cursor_context, editor_ancestor_lines,
    editor_block_end, editor_component_name, editor_first_word, editor_indentation,
};
pub use format::{format_fragment, format_source};
pub use semantic::*;
pub use source::{
    FileAnalysis, FileCompilation, analyze_file, analyze_file_graph, analyze_file_with_overlays,
    analyze_file_with_source, compile_file, discover_file_asset_dependencies,
    discover_file_dependencies, source_is_app,
};

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct CheckedDocument {
    document: Document,
    facts: check::CheckedFacts,
    declarations: hir::DeclarationIndex,
    origins: hir::OriginArena,
    symbols: Vec<CheckedSymbol>,
    warnings: Vec<Warning>,
    reachable_components: HashSet<String>,
    reachable_handlers: HashSet<String>,
    controlled_inputs: Vec<hir::AppStateId>,
    controlled_editors: Vec<CheckedControlledEditor>,
}

#[derive(Clone, Debug)]
struct CheckedControlledEditor {
    state: hir::AppStateId,
    action: Option<hir::ExternFnId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymbolKind {
    Component,
    Handler,
    Recipe,
    TestTarget,
}

impl SymbolKind {
    pub fn accepts(self, name: &str) -> bool {
        match self {
            Self::Component => {
                let parts = name.split("::").collect::<Vec<_>>();
                let Some((component, modules)) = parts.split_last() else {
                    return false;
                };
                modules.iter().all(|module| valid_identifier(module))
                    && component.split('.').all(|part| {
                        valid_identifier(part)
                            && part
                                .chars()
                                .next()
                                .is_some_and(|ch| ch.is_ascii_uppercase())
                    })
            }
            Self::Handler => name != "mount" && valid_identifier(name),
            Self::Recipe | Self::TestTarget => valid_identifier(name),
        }
    }
}

const RUST_KEYWORDS: &str = concat!(
    "abstract as async await become box break const continue crate do dyn else enum extern false ",
    "final fn for gen if impl in let loop macro match mod move mut override priv pub ref return ",
    "self Self static struct super trait true try type typeof unsafe unsized use virtual where while yield",
);

fn valid_rust_identifier(name: &str) -> bool {
    !name.is_empty()
        && name != "_"
        && RUST_KEYWORDS
            .split_ascii_whitespace()
            .all(|keyword| keyword != name)
        && name.chars().enumerate().all(|(index, ch)| {
            ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
        })
}

fn valid_identifier(name: &str) -> bool {
    name != "none" && !name.starts_with("__") && valid_rust_identifier(name)
}

fn canonical_snake(name: &str) -> bool {
    name.split('_')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_lowercase()))
}

fn unqualified_name(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.get(1) == Some(&b':') && bytes[0].is_ascii_alphabetic()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRange {
    pub path: Option<PathBuf>,
    pub line: usize,
    pub start_column: usize,
    pub end_column: usize,
}

impl SourceRange {
    pub fn contains(&self, path: Option<&Path>, line: usize, column: usize) -> bool {
        self.path.as_deref() == path
            && self.line == line
            && (self.start_column..self.end_column).contains(&column)
    }
}

#[derive(Clone, Debug)]
pub struct CheckedSymbol {
    pub kind: SymbolKind,
    pub scope: Option<String>,
    pub name: String,
    pub definition: SourceRange,
    pub references: Vec<SourceRange>,
    pub renameable: bool,
}

impl CheckedDocument {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        document: Document,
        facts: check::CheckedFacts,
        declarations: hir::DeclarationIndex,
        origins: hir::OriginArena,
        warnings: Vec<Warning>,
        reachable_components: HashSet<String>,
        reachable_handlers: HashSet<String>,
        controlled_inputs: Vec<hir::AppStateId>,
        controlled_editors: Vec<CheckedControlledEditor>,
    ) -> Self {
        Self {
            document,
            facts,
            declarations,
            origins,
            symbols: Vec::new(),
            warnings,
            reachable_components,
            reachable_handlers,
            controlled_inputs,
            controlled_editors,
        }
    }

    /// Returns the parsed source model for syntax-oriented tooling.
    ///
    /// Semantic consumers should use checked facts or lower to HIR instead.
    pub fn source_document(&self) -> &Document {
        &self.document
    }

    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    pub fn reachable_component_definitions(&self) -> Vec<&SourceRange> {
        self.symbols
            .iter()
            .filter(|symbol| {
                symbol.kind == SymbolKind::Component
                    && self.reachable_components.contains(&symbol.name)
            })
            .map(|symbol| &symbol.definition)
            .collect()
    }

    pub fn reachable_handler_definitions(&self) -> Vec<&SourceRange> {
        self.symbols
            .iter()
            .filter(|symbol| {
                symbol.kind == SymbolKind::Handler && self.reachable_handlers.contains(&symbol.name)
            })
            .map(|symbol| &symbol.definition)
            .collect()
    }

    pub fn symbols(&self) -> &[CheckedSymbol] {
        &self.symbols
    }

    pub fn symbol_at(
        &self,
        path: Option<&Path>,
        line: usize,
        column: usize,
    ) -> Option<(&CheckedSymbol, &SourceRange)> {
        self.symbols.iter().find_map(|symbol| {
            std::iter::once(&symbol.definition)
                .chain(&symbol.references)
                .find(|range| range.contains(path, line, column))
                .map(|range| (symbol, range))
        })
    }

    pub(crate) fn with_parsed_symbols(mut self, parsed: Vec<parser::ParsedSymbol>) -> Self {
        struct Builder {
            kind: SymbolKind,
            scope: Option<String>,
            name: String,
            definition: Option<SourceRange>,
            references: Vec<SourceRange>,
            complete: bool,
        }

        let mut symbols = BTreeMap::<(SymbolKind, Option<String>, String), Builder>::new();
        for parsed in parsed {
            let key = (parsed.kind, parsed.scope.clone(), parsed.name.clone());
            let symbol = symbols.entry(key).or_insert_with(|| Builder {
                kind: parsed.kind,
                scope: parsed.scope,
                name: parsed.name,
                definition: None,
                references: Vec::new(),
                complete: true,
            });
            let Some(range) = parsed.range else {
                symbol.complete = false;
                continue;
            };
            if parsed.definition {
                if symbol.definition.replace(range).is_some() {
                    symbol.complete = false;
                }
            } else {
                symbol.references.push(range);
            }
        }
        self.symbols = symbols
            .into_values()
            .filter_map(|symbol| {
                let definition = symbol.definition?;
                Some(CheckedSymbol {
                    kind: symbol.kind,
                    scope: symbol.scope,
                    renameable: symbol.complete
                        && !(symbol.kind == SymbolKind::Handler && symbol.name == "mount"),
                    name: symbol.name,
                    definition,
                    references: symbol.references,
                })
            })
            .collect();
        self
    }

    pub(crate) fn with_source_origins(mut self, origins: Vec<(PathBuf, usize)>) -> Self {
        for warning in &mut self.warnings {
            let Some((path, line)) = warning
                .line
                .checked_sub(1)
                .and_then(|index| origins.get(index))
            else {
                continue;
            };
            warning.path = Some(path.display().to_string());
            warning.line = *line;
        }
        self.origins.set_source_origins(origins);
        self
    }

    pub(crate) fn source_origins(&self) -> &[(PathBuf, usize)] {
        self.origins.source_origins()
    }

    #[cfg(test)]
    pub(crate) fn source_origin(&self, merged_line: usize) -> Option<(&Path, usize)> {
        self.origins.source_origin(merged_line)
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    pub code: &'static str,
    pub path: Option<String>,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: Option<String>,
}

impl Error {
    pub(crate) fn new(code: &'static str, span: &Span, message: impl Into<String>) -> Self {
        Self {
            code,
            path: None,
            line: span.line,
            column: span.column,
            message: message.into(),
            hint: None,
        }
    }

    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub(crate) fn at_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn render(&self, path: &str) -> String {
        let path = self.path.as_deref().unwrap_or(path);
        let mut rendered = format!(
            "{} {}:{}:{}: {}",
            self.code, path, self.line, self.column, self.message
        );
        if let Ok(source) = std::fs::read_to_string(path)
            && let Some(index) = self.line.checked_sub(1)
            && let Some(line) = source.lines().nth(index)
        {
            let gutter = self.line.to_string();
            let column = self.column.saturating_sub(1).min(line.chars().count());
            rendered.push_str(&format!(
                "\n{gutter} | {line}\n{} | {}^",
                " ".repeat(gutter.len()),
                " ".repeat(column)
            ));
        }
        if let Some(hint) = &self.hint {
            rendered.push_str("\nhint: ");
            rendered.push_str(hint);
        }
        rendered
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self
            .path
            .as_deref()
            .map(|path| format!(" in {path}"))
            .unwrap_or_default();
        write!(
            f,
            "{} at {}:{}{}: {}",
            self.code, self.line, self.column, path, self.message
        )
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub code: &'static str,
    pub path: Option<String>,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: Option<String>,
}

impl Warning {
    pub(crate) fn new(code: &'static str, span: &Span, message: impl Into<String>) -> Self {
        Self {
            code,
            path: None,
            line: span.line,
            column: span.column,
            message: message.into(),
            hint: None,
        }
    }

    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render(&self, path: &str) -> String {
        let path = self.path.as_deref().unwrap_or(path);
        let mut rendered = format!(
            "warning[{}] {}:{}:{}: {}",
            self.code, path, self.line, self.column, self.message
        );
        if let Ok(source) = std::fs::read_to_string(path)
            && let Some(index) = self.line.checked_sub(1)
            && let Some(line) = source.lines().nth(index)
        {
            let gutter = self.line.to_string();
            let column = self.column.saturating_sub(1).min(line.chars().count());
            rendered.push_str(&format!(
                "\n{gutter} | {line}\n{} | {}^",
                " ".repeat(gutter.len()),
                " ".repeat(column)
            ));
        }
        if let Some(hint) = &self.hint {
            rendered.push_str("\nhint: ");
            rendered.push_str(hint);
        }
        rendered
    }
}

pub fn parse(source: &str) -> Result<Document, Error> {
    parser::parse(source)
}

pub fn analyze(source: &str) -> Result<CheckedDocument, Error> {
    let (document, symbols) = parser::parse_with_symbols(source)?;
    Ok(check::analyze(document)?.with_parsed_symbols(symbols))
}

pub fn compile(source: &str, source_path: &str) -> Result<String, Error> {
    let document = analyze(source)?;
    let program = lower::lower(document)?;
    codegen::generate(&program, source_path)
}
