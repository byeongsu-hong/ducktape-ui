//! `cargo ice bundle --target wasm32-unknown-unknown`: builds Ice apps as
//! core modules and wraps each as an `ice:view` component in a catalog
//! directory an app-store host scans.
//!
//! The core module still links wasm-bindgen's placeholder imports — iced's
//! wasm target pulls `web-time` and `web-sys` in, and their `#[wasm_bindgen]`
//! glue is exported, so the linker keeps it — and a component may not import
//! what its world does not name. Each of those import modules is satisfied by
//! a stub adapter whose every function traps: nothing on a guest's frame path
//! calls them, and a call is a bug worth a trap. The stubs are generated from
//! the module's own import list, so a new wasm-bindgen hash needs no edit.

use super::{Request, capture, create_dir, path, tool, write};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) const TARGET: &str = "wasm32-unknown-unknown";
const CATALOG_DIR: &str = "app-store-catalog";
/// The world's own import list; the host provides those.
const WORLD_IMPORTS: &str = "$root";
/// wasm-bindgen's metadata is a third of the file and nothing reads it.
const BINDGEN_SECTION: &str = "__wasm_bindgen_unstable";

pub(super) fn run(root: &Path, request: &Request) -> Result<(), String> {
    let metadata = crate::dev::metadata_for(root, &request.manifest_arguments())?;
    let target_directory: PathBuf = metadata["target_directory"]
        .as_str()
        .ok_or_else(|| "cargo metadata output has no target directory".to_owned())?
        .into();
    let release = target_directory.join(TARGET).join("release");
    let modules = request
        .packages
        .iter()
        .map(|package| Ok(release.join(format!("{}.wasm", library_name(&metadata, package)?))))
        .collect::<Result<Vec<_>, String>>()?;
    wasm_tools(&["--version".to_owned()])?;

    let build = super::build_arguments(request);
    crate::cargo(&build.iter().map(String::as_str).collect::<Vec<_>>())?;

    let catalog = request
        .out
        .clone()
        .unwrap_or_else(|| target_directory.join(CATALOG_DIR));
    create_dir(&catalog)?;
    let scratch = target_directory.join("ice-bundle").join(TARGET);
    create_dir(&scratch)?;
    let optimizer = wasm_opt();
    if optimizer.is_none() {
        eprintln!("wasm-opt is not on PATH; components are written unoptimized");
    }
    for module in &modules {
        let component = componentize(module, &catalog, &scratch, optimizer)?;
        println!("{}", component.display());
    }
    Ok(())
}

/// The `cdylib` a package builds for wasm32, by the file name cargo gives it.
fn library_name(metadata: &Value, package: &str) -> Result<String, String> {
    let found = metadata["packages"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .find(|entry| entry["source"].is_null() && entry["name"].as_str() == Some(package))
        .ok_or_else(|| format!("ice bundle: package `{package}` was not found"))?;
    found["targets"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .find(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "cdylib"))
        })
        .and_then(|target| target["name"].as_str())
        .map(|name| name.replace('-', "_"))
        .ok_or_else(|| {
            format!(
                "ice bundle: package `{package}` has no `cdylib` target; an `ice:view` component is built from one (`crate-type = [\"cdylib\", \"rlib\"]`)"
            )
        })
}

fn componentize(
    module: &Path,
    catalog: &Path,
    scratch: &Path,
    optimizer: Option<&'static str>,
) -> Result<PathBuf, String> {
    let name = module
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("`{}` is not a module path", module.display()))?;
    let module = match optimizer {
        Some(optimizer) => {
            let optimized = scratch.join(format!("{name}.opt.wasm"));
            tool(
                optimizer,
                &[
                    "-Os".to_owned(),
                    "--all-features".to_owned(),
                    path(module),
                    "-o".to_owned(),
                    path(&optimized),
                ],
            )?;
            optimized
        }
        None => module.to_path_buf(),
    };
    let wat = wasm_tools(&["print".to_owned(), path(&module)])?;
    let mut arguments = vec!["component".to_owned(), "new".to_owned(), path(&module)];
    for (import, stub) in stubs(&wat)? {
        let text = scratch.join(format!("{name}.{import}.wat"));
        let binary = text.with_extension("wasm");
        write(&text, stub.as_bytes())?;
        wasm_tools(&[
            "parse".to_owned(),
            path(&text),
            "-o".to_owned(),
            path(&binary),
        ])?;
        arguments.push("--adapt".to_owned());
        arguments.push(format!("{import}={}", binary.display()));
    }
    let component = catalog.join(format!("{name}.wasm"));
    arguments.push("-o".to_owned());
    arguments.push(path(&component));
    wasm_tools(&arguments)?;
    wasm_tools(&[
        "strip".to_owned(),
        "--delete".to_owned(),
        BINDGEN_SECTION.to_owned(),
        path(&component),
        "-o".to_owned(),
        path(&component),
    ])?;
    Ok(component)
}

/// One trapping stub module per import module the world does not name, from
/// the module's printed text: `(type (;N;) (func …))` lines give the
/// signatures, `(import "module" "name" (func … (type N)))` lines the names.
fn stubs(wat: &str) -> Result<Vec<(String, String)>, String> {
    let mut signatures = BTreeMap::new();
    let mut imports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in wat.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("(type (;") {
            let (id, rest) = rest
                .split_once(";) (func")
                .ok_or_else(|| format!("unexpected type line in module text: `{trimmed}`"))?;
            let signature = rest
                .strip_suffix("))")
                .ok_or_else(|| format!("unexpected type line in module text: `{trimmed}`"))?;
            signatures.insert(id.to_owned(), signature.to_owned());
        } else if let Some(rest) = trimmed.strip_prefix("(import \"") {
            let mut names = rest.split('"');
            let module = names.next().unwrap_or_default();
            let name = names.nth(1).unwrap_or_default();
            let descriptor = names.next().unwrap_or_default().trim();
            if module == WORLD_IMPORTS {
                continue;
            }
            let type_id = descriptor
                .strip_prefix("(func")
                .and_then(|func| func.rsplit_once("(type "))
                .and_then(|(_, id)| id.split(')').next())
                .ok_or_else(|| {
                    format!(
                        "`{module}` `{name}` is imported as something other than a function, which no stub adapter can satisfy: `{trimmed}`"
                    )
                })?;
            let signature = signatures.get(type_id).ok_or_else(|| {
                format!("import `{module}` `{name}` names type {type_id}, which the module text does not define")
            })?;
            imports.entry(module.to_owned()).or_default().push(format!(
                "  (func (export \"{name}\"){signature} unreachable)\n"
            ));
        }
    }
    Ok(imports
        .into_iter()
        .map(|(module, functions)| (module, format!("(module\n{})\n", functions.concat())))
        .collect())
}

fn wasm_tools(arguments: &[String]) -> Result<String, String> {
    capture("wasm-tools", arguments, None).map_err(|error| {
        if error.contains("cannot run wasm-tools") {
            "`wasm-tools` is not on PATH; a wasm bundle needs it to wrap the module as a component (`cargo install wasm-tools`, or a release from github.com/bytecodealliance/wasm-tools)".to_owned()
        } else {
            error
        }
    })
}

/// `wasm-opt` when the host has it; a component ships unoptimized otherwise.
fn wasm_opt() -> Option<&'static str> {
    Command::new("wasm-opt")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "wasm-opt")
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAT: &str = r#"(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func (param i32) (result i32)))
  (type (;2;) (func))
  (import "$root" "host-request" (func (;0;) (type 1)))
  (import "wbg" "__wbindgen_throw" (func $throw (;1;) (type 0)))
  (import "__wbindgen_placeholder__" "__wbg_now_abc123" (func (;2;) (type 2)))
  (import "wbg" "__wbindgen_describe" (func (;3;) (type 2)))
  (func (;4;) (type 2))
)
"#;

    #[test]
    fn every_import_outside_the_world_gets_a_trapping_stub() {
        let stubs = stubs(WAT).expect("stubs from module text");
        assert_eq!(
            stubs,
            vec![
                (
                    "__wbindgen_placeholder__".to_owned(),
                    "(module\n  (func (export \"__wbg_now_abc123\") unreachable)\n)\n".to_owned()
                ),
                (
                    "wbg".to_owned(),
                    "(module\n  (func (export \"__wbindgen_throw\") (param i32 i32) unreachable)\n  (func (export \"__wbindgen_describe\") unreachable)\n)\n".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn a_non_function_import_is_refused() {
        let wat = "(module\n  (import \"wbg\" \"memory\" (memory (;0;) 1))\n)\n";
        let error = stubs(wat).expect_err("a memory import has no stub");
        assert!(error.contains("something other than a function"), "{error}");
    }
}
