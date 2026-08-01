use std::fs;
use std::path::{Path, PathBuf};

const BACKDOORS: &[(&str, &str)] = &[
    ("source AST dependency", "use crate::ast::*;"),
    ("checked-document escape", "program.document()"),
    ("checker dependency", "crate::check"),
    ("checked-facts escape", "checked_facts()"),
    ("declaration-index escape", "declarations()"),
    ("type re-analysis", "expr_type("),
    ("extern re-resolution", "find_extern_function("),
    ("raw expression fallback", "ExprNode::Ast"),
    ("Document input", "&Document"),
    ("Expr input", "&Expr"),
    ("Route input", "&Route"),
    ("Statement slice input", "&[Statement]"),
];

const EXPECTED_INVENTORY: &str = r#"[source AST dependency]
src/codegen.rs 1
[checked-document escape]
src/codegen/application.rs 3
src/codegen/expr/discovery.rs 2
src/codegen/expr.rs 1
src/codegen/statement/view_fn.rs 1
src/codegen/style/model.rs 1
src/codegen/testing.rs 3
src/codegen.rs 3
[checker dependency]
src/codegen/expr.rs 4
src/codegen/statement/task.rs 3
src/codegen/statement.rs 1
src/codegen.rs 1
[checked-facts escape]
src/codegen/expr/children.rs 3
src/codegen/expr/discovery.rs 1
src/codegen/expr.rs 7
src/codegen/statement/view_fn.rs 1
src/codegen/testing.rs 1
src/codegen/view/layout.rs 3
src/codegen/view/pane.rs 3
src/codegen/view/structure.rs 3
src/codegen/view/table.rs 2
src/codegen.rs 2
[declaration-index escape]
src/codegen/expr.rs 4
src/codegen.rs 3
[type re-analysis]
src/codegen/canvas/commands.rs 4
src/codegen/canvas.rs 1
src/codegen/expr.rs 1
src/codegen/statement.rs 1
src/codegen/style/common.rs 1
src/codegen/view/media.rs 1
[extern re-resolution]
src/codegen/application.rs 1
src/codegen/expr.rs 2
src/codegen/statement/task.rs 1
src/codegen/statement.rs 4
src/codegen/style/helpers.rs 2
src/codegen/subscription.rs 6
src/codegen/view/content.rs 3
src/codegen/view/documents.rs 3
[raw expression fallback]
src/codegen/expr.rs 4
[Document input]
src/codegen/application.rs 2
src/codegen/canvas/commands.rs 1
src/codegen/canvas/events.rs 2
src/codegen/canvas/path.rs 1
src/codegen/canvas/style.rs 8
src/codegen/canvas.rs 2
src/codegen/expr/binding.rs 1
src/codegen/expr/discovery.rs 8
src/codegen/expr/routes.rs 8
src/codegen/expr.rs 4
src/codegen/probes.rs 2
src/codegen/runtime.rs 7
src/codegen/settings.rs 2
src/codegen/statement/task.rs 2
src/codegen/statement.rs 2
src/codegen/style/boolean.rs 8
src/codegen/style/common.rs 8
src/codegen/style/controls.rs 5
src/codegen/style/helpers.rs 12
src/codegen/style/model.rs 4
src/codegen/style/selection.rs 9
src/codegen/subscription.rs 1
src/codegen/testing.rs 5
src/codegen/view/container.rs 1
src/codegen/view/foundation.rs 2
src/codegen/view/layout.rs 6
src/codegen/view/pane.rs 1
src/codegen/view.rs 2
[Expr input]
src/codegen/canvas/style.rs 5
src/codegen/expr/routes.rs 1
src/codegen/expr.rs 26
src/codegen/statement.rs 7
src/codegen/style/common.rs 3
src/codegen/style/model.rs 1
src/codegen/testing.rs 1
src/codegen/view/layout.rs 1
[Route input]
src/codegen/canvas/events.rs 1
src/codegen/expr/routes.rs 7
src/codegen/statement.rs 1
[Statement slice input]
src/codegen/application.rs 1
src/codegen/expr/discovery.rs 1
src/codegen/runtime.rs 3
src/codegen/statement.rs 1"#;

#[test]
fn codegen_semantic_backdoors_are_an_explicit_shrinking_inventory() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest.join("src/codegen");
    let mut files = Vec::new();
    files.push(manifest.join("src/codegen.rs"));
    collect_rust_files(&source_root, &mut files);
    files.sort();

    let mut inventory = String::new();
    for (label, needle) in BACKDOORS {
        inventory.push_str(&format!("[{label}]\n"));
        for path in &files {
            let relative = path.strip_prefix(&manifest).expect("codegen source path");
            if relative
                .components()
                .any(|part| part.as_os_str() == "tests")
            {
                continue;
            }
            let source = fs::read_to_string(path).expect("read codegen source");
            let count = source.matches(needle).count();
            if count != 0 {
                let relative = relative
                    .iter()
                    .map(|part| part.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                inventory.push_str(&format!("{relative} {count}\n"));
            }
        }
    }

    assert_eq!(
        inventory.trim_end(),
        EXPECTED_INVENTORY,
        "the codegen AST/checker backdoor inventory changed; semantic migrations must remove the old path and shrink this exact ledger, while new or restored entries are forbidden",
    );
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read codegen directory") {
        let path = entry.expect("read codegen entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}
