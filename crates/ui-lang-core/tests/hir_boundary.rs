use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "hir_boundary/scanner.rs"]
mod scanner;

use scanner::{SourceFile, exported_ast_types, inventory, is_production_codegen_path};

const EXPECTED_INVENTORY: &str = include_str!("hir_boundary/expected.txt");

#[test]
fn codegen_semantic_backdoors_are_an_explicit_reviewed_inventory() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let files = production_codegen_sources(&manifest);
    let ast_types = exported_ast_types(&rust_sources(&manifest.join("src/ast")));
    let actual = inventory(&files, &ast_types).expect("scan codegen boundary");
    assert_eq!(
        actual,
        EXPECTED_INVENTORY.trim_end(),
        "the selected codegen AST/checker lexical boundary inventory changed; occurrence growth is forbidden and every fingerprint change requires review"
    );
}

#[test]
fn lexical_scanner_detects_lifetime_mut_by_value_container_and_qualified_ast_types() {
    let ast_types = probe_ast_types();
    let actual = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "use crate::ast::*; fn emit<'a>(document: &'a crate::Document, expr: &mut Expr, route: Option<Route>, statements: Vec<Document>, qualified: crate::ast::Statement) {}",
        )],
        &ast_types,
    )
    .unwrap();
    for category in [
        "source AST import",
        "source AST semantic reference",
        "Document reference",
        "Expr reference",
        "Route reference",
        "Statement reference",
    ] {
        assert!(section(&actual, category).contains("src/codegen/probe.rs"));
    }
}

#[test]
fn lexical_scanner_does_not_treat_identifier_prefixes_as_ast_types() {
    let ast_types = probe_ast_types();
    let clean = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "fn 이름(인자: &ExprArguments, 문맥: &ExprEmission<'_>) {}",
        )],
        &ast_types,
    )
    .unwrap();
    assert!(!section(&clean, "Expr reference").contains("probe.rs"));
    assert!(!section(&clean, "source AST semantic reference").contains("probe.rs"));
}

#[test]
fn lexical_scanner_handles_non_ascii_rust_identifiers_without_byte_boundary_panics() {
    let actual = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "fn 경계_검사(값: 문자열) { let 결과 = 값; drop(결과); }",
        )],
        &probe_ast_types(),
    )
    .unwrap();
    assert!(!actual.contains("src/codegen/probe.rs"));
}

#[test]
fn lexical_scanner_ignores_comments_and_string_literals() {
    let ast_types = probe_ast_types();
    let clean = inventory(
        &[probe(
            "src/codegen/probe.rs",
            r###"fn emit() {
                // program.document(); Expr Document
                /* nested /* RenderDocument crate::check */ comment */
                let _ = "program.document(); &Expr";
                let _ = r#"crate::ast::Document"#;
            }"###,
        )],
        &ast_types,
    )
    .unwrap();
    for category in [
        "source AST import",
        "source AST semantic reference",
        "checked-document escape",
        "raw document wrapper",
        "checker semantic reference",
        "Document reference",
        "Expr reference",
    ] {
        assert!(!section(&clean, category).contains("probe.rs"));
    }
}

#[test]
fn lexical_scanner_exports_only_top_level_public_ast_declarations() {
    let exported = exported_ast_types(&[
        "pub struct Document { pub span: usize } impl Document { pub fn identity(&self) {} } mod nested { pub struct Hidden; } pub(crate) enum Expr {}"
            .into(),
    ]);
    assert_eq!(
        exported,
        BTreeSet::from(["Document".to_owned(), "Expr".to_owned()])
    );
}

#[test]
fn lexical_scanner_ignores_same_named_local_items_without_an_ast_import() {
    let ast_types = exported_ast_types(&[
        "pub struct Component; pub enum TestStepKind {} pub struct Expr;".into(),
    ]);
    let actual = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "struct Component; enum TestStepKind {} struct Expr; fn emit(_: Component, _: TestStepKind, _: Expr) {}",
        )],
        &ast_types,
    )
    .unwrap();
    assert_eq!(
        section_count(
            &actual,
            "source AST semantic reference",
            "src/codegen/probe.rs"
        ),
        0
    );
    assert_eq!(
        section_count(&actual, "Expr reference", "src/codegen/probe.rs"),
        0
    );
}

#[test]
fn lexical_scanner_ignores_non_ast_crate_globs() {
    let ast_types = exported_ast_types(&["pub enum Type {} pub struct Component;".into()]);
    let actual = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "use crate::lower::*; use crate::semantic::*; fn emit(_: Type, _: Component) {}",
        )],
        &ast_types,
    )
    .unwrap();
    assert_eq!(
        section_count(&actual, "source AST import", "src/codegen/probe.rs"),
        0
    );
    assert_eq!(
        section_count(
            &actual,
            "source AST semantic reference",
            "src/codegen/probe.rs"
        ),
        0
    );
}

#[test]
fn lexical_scanner_resolves_ast_module_aliases_and_alias_chains() {
    let actual = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "use crate::{ast as syntax}; use syntax::Expr as Expression; fn emit(_: syntax::Document, _: Expression) {}",
        )],
        &probe_ast_types(),
    )
    .unwrap();
    assert_eq!(
        section_count(
            &actual,
            "source AST semantic reference",
            "src/codegen/probe.rs"
        ),
        2
    );
    assert!(section(&actual, "source AST import").contains("src/codegen/probe.rs"));
}

#[test]
fn lexical_scanner_resolves_local_use_aliases_only_inside_their_item() {
    let actual = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "fn imported() { use crate::{ast::{Expr as LocalExpr}}; let _: LocalExpr; } fn unrelated(_: LocalExpr) {}",
        )],
        &probe_ast_types(),
    )
    .unwrap();
    assert_eq!(
        section_count(
            &actual,
            "source AST semantic reference",
            "src/codegen/probe.rs"
        ),
        1
    );
    assert!(section(&actual, "source AST import").contains("src/codegen/probe.rs"));
}

#[test]
fn occurrence_fingerprints_reject_same_file_delete_and_add_swaps() {
    let ast_types = probe_ast_types();
    let before = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "fn old(program: &LoweredProgram) { let _ = program.document(); } fn new(_: &LoweredProgram) {}",
        )],
        &ast_types,
    )
    .unwrap();
    let after = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "fn old(_: &LoweredProgram) {} fn new(program: &LoweredProgram) { let _ = program.document(); }",
        )],
        &ast_types,
    )
    .unwrap();
    assert_eq!(
        section_count(&before, "checked-document escape", "src/codegen/probe.rs"),
        section_count(&after, "checked-document escape", "src/codegen/probe.rs")
    );
    assert_ne!(
        before, after,
        "relocating one occurrence must change its fingerprint"
    );
}

#[test]
fn root_imports_do_not_hide_new_ast_or_checker_uses() {
    let ast_types = exported_ast_types(&[
        "pub enum Expr {} pub fn ast_helper() {} pub const AST_REVISION: u32 = 1; pub static mut AST_ROOT: u32 = 1;"
            .into(),
    ]);
    for exported in ["Expr", "ast_helper", "AST_REVISION", "AST_ROOT"] {
        assert!(ast_types.contains(exported));
    }
    let before = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "use crate::ast::*; use crate::check::{CheckedExprId}; fn emit(_: CheckedExprId) {}",
        )],
        &ast_types,
    )
    .unwrap();
    let after = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "use crate::ast::*; use crate::check::{CheckedExprId, expr_type}; fn emit(_: CheckedExprId, _: Expr) { expr_type(); ast_helper(); let _ = AST_REVISION; let _ = AST_ROOT; }",
        )],
        &ast_types,
    )
    .unwrap();
    assert_ne!(before, after);
    assert!(section(&after, "Expr reference").contains("probe.rs"));
    assert!(section(&after, "checker semantic reference").contains("probe.rs"));
}

#[test]
fn grouped_and_nested_ast_alias_uses_change_the_inventory() {
    let ast_types = probe_ast_types();
    let before = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "use crate::{ast::{Document as Doc, expr::{Expr as Expression}}}; fn emit() {}",
        )],
        &ast_types,
    )
    .unwrap();
    let after = inventory(
        &[probe(
            "src/codegen/probe.rs",
            "use crate::{ast::{Document as Doc, expr::{Expr as Expression}}}; fn emit(_: Doc, _: Option<Expression>) {}",
        )],
        &ast_types,
    )
    .unwrap();
    let before_count = section_count(
        &before,
        "source AST semantic reference",
        "src/codegen/probe.rs",
    );
    let after_count = section_count(
        &after,
        "source AST semantic reference",
        "src/codegen/probe.rs",
    );
    assert_eq!(after_count, before_count + 2);
    assert_ne!(before, after);
    assert!(section(&after, "source AST import").contains("probe.rs"));
}

#[test]
fn production_filter_excludes_both_test_module_shapes() {
    assert!(!is_production_codegen_path(Path::new(
        "src/codegen/tests.rs"
    )));
    assert!(!is_production_codegen_path(Path::new(
        "src/codegen/tests/application.rs"
    )));
    assert!(is_production_codegen_path(Path::new(
        "src/codegen/application.rs"
    )));
}

fn production_codegen_sources(manifest: &Path) -> Vec<SourceFile> {
    let mut paths = vec![manifest.join("src/codegen.rs")];
    collect_rust_files(&manifest.join("src/codegen"), &mut paths);
    paths.retain(|path| {
        path.strip_prefix(manifest)
            .is_ok_and(is_production_codegen_path)
    });
    paths.sort();
    paths
        .into_iter()
        .map(|path| SourceFile {
            relative: relative_path(manifest, &path),
            source: fs::read_to_string(&path).expect("read codegen source"),
        })
        .collect()
}

fn rust_sources(directory: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    collect_rust_files(directory, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read_to_string(path).expect("read Rust source"))
        .collect()
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read Rust source directory") {
        let entry = entry.expect("read Rust source entry");
        let file_type = entry.file_type().expect("read Rust source file type");
        assert!(
            !file_type.is_symlink(),
            "source inventory does not follow symlinks"
        );
        let path = entry.path();
        if file_type.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("source path below crate root")
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn probe(relative: &str, source: &str) -> SourceFile {
    SourceFile {
        relative: relative.to_owned(),
        source: source.to_owned(),
    }
}

fn probe_ast_types() -> BTreeSet<String> {
    exported_ast_types(&[
        "pub struct Document; pub enum Expr {} pub struct Route; pub enum Statement {}".into(),
    ])
}

fn section<'a>(inventory: &'a str, category: &str) -> &'a str {
    let header = format!("[{category}]\n");
    let body = inventory
        .strip_prefix(&header)
        .or_else(|| inventory.split_once(&header).map(|(_, body)| body))
        .expect("inventory category");
    body.split("\n[").next().unwrap_or(body)
}

fn section_count(inventory: &str, category: &str, path: &str) -> usize {
    section(inventory, category)
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next() == Some(path)).then(|| fields.next().unwrap().parse().unwrap())
        })
        .unwrap_or(0)
}
