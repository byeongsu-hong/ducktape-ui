//! Derived values cached on the app struct.
//!
//! Every derived value owns one `OnceCell` in `__IceDerivedCache`, filled by
//! the first read after a write and handed out by reference until the next
//! write to an app-state field the expression reads. Caching is sound because
//! a derived expression is pure by construction: the checker already rejects
//! `sync` externs and recomputation-unsafe built-ins there.
//!
//! The invariant that makes the cache correct is that every emitter of an
//! app-state write goes through `state_write::state_write_code`, which follows
//! the write with the cells it can have stale ([`derived_clears_code`]).
//! `codegen/tests/derived_cache.rs` pins that invariant against the generated
//! output of every write form.

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

/// The cache clears a write to `target` must be followed by: one `take()`
/// per derived value whose expression transitively reads the field, so the
/// next read recomputes. Empty for a target no derived reads — component
/// state, a secret, a field nothing derives from. Emitted only by
/// `state_write::state_write_code`, the one write path.
pub(in crate::codegen) fn derived_clears_code(
    program: &LoweredProgram,
    receiver: &str,
    target: ResolvedValueRef,
) -> String {
    program
        .derived()
        .iter()
        .filter(|derived| derived.reads.contains(&target))
        .map(|derived| format!(" {receiver}.{DERIVED_CACHE_FIELD}.{}.take();", derived.name))
        .collect()
}
