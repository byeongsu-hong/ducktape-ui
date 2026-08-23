//! Derived values cached on the app struct.
//!
//! Every derived value owns one `OnceCell` in `__IceDerivedCache`, filled by
//! the first read after a write and handed out by reference until the next
//! write to an app-state field the expression reads. Caching is sound because
//! a derived expression is pure by construction: the checker already rejects
//! `sync` externs and recomputation-unsafe built-ins there.
//!
//! The invariant that makes the cache correct is that every emitter of an
//! app-state write goes through [`state_write`], which follows the write with
//! the cells it can have stale. `codegen/tests/derived_cache.rs` pins that
//! invariant against the generated output of every write form.

use super::*;

pub(in crate::codegen) const DERIVED_CACHE_FIELD: &str = "__ice_derived";
pub(in crate::codegen) const DERIVED_CACHE_TYPE: &str = "__IceDerivedCache";

pub(in crate::codegen) fn generate_derived_cache_type(out: &mut String, program: &LoweredProgram) {
    if program.derived().is_empty() {
        return;
    }
    writeln!(out, "#[derive(Default)]\nstruct {DERIVED_CACHE_TYPE} {{").unwrap();
    for derived in program.derived() {
        writeln!(
            out,
            "{}: ::std::cell::OnceCell<{}>,",
            derived.name,
            rust_type_code(program, &derived.ty)
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
}

pub(in crate::codegen) fn generate_derived(
    out: &mut String,
    program: &LoweredProgram,
) -> Result<(), Error> {
    let env = checked_state_env(program, "self");
    for derived in program.derived() {
        let value = resolved_expr_use_code(program, derived.initializer, &env, ValueMode::Owned)?;
        writeln!(out, "{}", source_marker(&derived.span)).unwrap();
        writeln!(
            out,
            "fn {}(&self) -> &{} {{ self.{DERIVED_CACHE_FIELD}.{}.get_or_init(|| {value}) }}",
            derived_method(&derived.name),
            rust_type_code(program, &derived.ty),
            derived.name,
        )
        .unwrap();
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    Ok(())
}

/// The one way generated code writes an app value. Emits `write` followed by
/// the cache clears for every derived value whose expression transitively
/// reads `target`, so the next read recomputes. A target no derived reads —
/// component-local state, a secret, a field nothing derives from — emits the
/// write alone.
pub(in crate::codegen) fn state_write(
    program: &LoweredProgram,
    receiver: &str,
    target: ResolvedValueRef,
    write: impl std::fmt::Display,
) -> String {
    let mut code = write.to_string();
    for derived in program
        .derived()
        .iter()
        .filter(|derived| derived.reads.contains(&target))
    {
        write!(
            code,
            " {receiver}.{DERIVED_CACHE_FIELD}.{}.take();",
            derived.name
        )
        .unwrap();
    }
    code
}
