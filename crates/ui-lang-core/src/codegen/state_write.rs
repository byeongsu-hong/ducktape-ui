//! State revisions: the one write path for app and component state, and the
//! revision reads `lazy` keys its memo off.
//!
//! Every app-state field and every component-local state field has a
//! compiler-owned `u64` revision in a `__ice_rev` array beside the fields —
//! index = declaration position. Every emitter of a state write goes through
//! [`state_write_code`], which stores the value, ticks the field's revision,
//! and clears the derived cells that read the field (`derived.rs`): compared
//! first when the type can compare (autoref specialization in
//! `ui_lang_runtime::rev`, so an extern Rust type need not implement
//! `PartialEq`), unconditionally for in-place mutations and for
//! self-assignments that already took the old value out of the field.
//!
//! A `lazy` whose value reads state puts those fields' revisions in its memo
//! tuple instead of the value: [`revision_reads`] collects them through
//! derived values and through component params, whose call sites leave their
//! own read set beside the baked argument ([`param_revisions_key`]).

use super::*;
use std::collections::BTreeSet;

/// The revision array on the app struct and on each component state struct.
pub(in crate::codegen) const REVISIONS_FIELD: &str = "__ice_rev";

/// How a write reaches a state field.
pub(in crate::codegen) enum StateWrite {
    /// `field = value`, ticking the revision only when the stored value
    /// differs (or cannot be compared).
    Assign(String),
    /// `field = value` with the revision ticked unconditionally: the old
    /// value is gone (a `mem::take` self-assignment) or is a fresh widget
    /// state no comparison could see through.
    Replace(String),
    /// A statement that mutates the field in place (`push`, `perform`,
    /// `go_mut`, ...), followed by an unconditional tick.
    Mutate(String),
}

/// The field a writable value lives in and its revision index, or `None` for
/// a store that has neither (a secret).
fn revision_slot(program: &LoweredProgram, target: ResolvedValueRef) -> Option<(&str, usize)> {
    match target {
        ResolvedValueRef::AppState(id) => {
            Some((&program.app_states()[id.0 as usize].name, id.0 as usize))
        }
        ResolvedValueRef::ComponentState(id) => Some((
            &program.component(id.component).states[id.index as usize].name,
            id.index as usize,
        )),
        ResolvedValueRef::Secret(_) => None,
        ResolvedValueRef::Derived(_) | ResolvedValueRef::ComponentParam(_) => {
            unreachable!("derived values and component params are never written")
        }
    }
}

/// The one way generated code writes a state value: `receiver` is the struct
/// holding the field (`self`, `__local`), `target` the value written.
pub(in crate::codegen) fn state_write_code(
    program: &LoweredProgram,
    receiver: &str,
    target: ResolvedValueRef,
    write: StateWrite,
) -> String {
    let Some((field, index)) = revision_slot(program, target) else {
        return match write {
            StateWrite::Assign(_) | StateWrite::Replace(_) => {
                unreachable!("a secret is only ever wiped in place")
            }
            StateWrite::Mutate(statement) => format!("{statement};"),
        };
    };
    // The tick and the derived-cache clears are one event: a derived value
    // over the field goes stale exactly when the field's revision moves, so
    // an equal-value assignment leaves both alone.
    let tick = format!(
        "{receiver}.{REVISIONS_FIELD}[{index}] += 1;{}",
        derived_clears_code(program, receiver, target)
    );
    match write {
        StateWrite::Assign(value) => format!(
            "{{ let __ice_next = {value}; if ::ui_lang_runtime::state_changed!({receiver}.{field}, __ice_next) {{ {receiver}.{field} = __ice_next; {tick} }} }}"
        ),
        StateWrite::Replace(value) => format!("{receiver}.{field} = {value}; {tick}"),
        StateWrite::Mutate(statement) => format!("{statement}; {tick}"),
    }
}

/// The `__ice_rev: [u64; N]` field declaration for `count` revisions.
pub(in crate::codegen) fn revisions_field_code(count: usize) -> String {
    format!("{REVISIONS_FIELD}: [u64; {count}],")
}

/// The `__ice_rev` initializer for `count` revisions: every field starts at
/// the instance's process-unique seed, so a memo built from another
/// instance's state never hashes alike (`ui_lang_runtime::rev::seed`).
pub(in crate::codegen) fn revisions_init_code(count: usize) -> String {
    format!("{REVISIONS_FIELD}: [::ui_lang_runtime::rev::seed(); {count}],")
}

/// The view-time read of one revision: an app field's from `self`, a
/// component field's through the instance map keyed by the call-site scope,
/// `0` while the instance has no entry — the same lookup the state value
/// itself makes.
fn revision_read_code(
    program: &LoweredProgram,
    value: ResolvedValueRef,
    env: &dyn BindingEnvironment,
) -> Option<String> {
    match value {
        ResolvedValueRef::AppState(id) => Some(format!("self.{REVISIONS_FIELD}[{}]", id.0)),
        ResolvedValueRef::ComponentState(id) => {
            let component = program.component(id.component);
            let state = component.states.iter().find(|state| state.id == id)?;
            let binding = env.get(&state.name)?;
            let Some(StateBinding::Component { scope, .. }) = &binding.state else {
                return None;
            };
            let field = component_state_field(&component.name);
            let states = match component.storage {
                ComponentStorage::Retained => format!("self.{field}"),
                ComponentStorage::Mounted => format!("self.{field}.values()"),
                ComponentStorage::Stateless => return None,
            };
            Some(format!(
                "{states}.get(&{scope}).map_or(0, |__state| __state.{REVISIONS_FIELD}[{}])",
                id.index
            ))
        }
        ResolvedValueRef::Derived(_)
        | ResolvedValueRef::Secret(_)
        | ResolvedValueRef::ComponentParam(_) => None,
    }
}

/// The marker a component call site leaves beside a baked argument: its code
/// is the argument expression's revision reads, [`REVISION_SEPARATOR`]-joined
/// (empty when the expression reads no state). Absent when the argument
/// cannot move into a `'static` builder — it reads a render-site local, an
/// enclosing scope binding, or a per-frame derived snapshot.
pub(in crate::codegen) fn param_revisions_key(name: &str) -> String {
    format!("\0param-revisions:{name}")
}

pub(in crate::codegen) fn is_param_revisions_key(name: &str) -> bool {
    name.starts_with("\0param-revisions:")
}

pub(in crate::codegen) const REVISION_SEPARATOR: char = '\u{1f}';

/// The revision reads of everything `expression_use` reads from state,
/// transitively through derived values and component params, in a stable
/// order — or `None` when the expression also reads something no revision
/// tracks: a render-site local (a loop row, a match payload, a window id) or
/// a param no call site could root in state, which cannot be materialized
/// inside a `'static` builder; a secret, whose store has no revision; a
/// recomputation-unsafe builtin (an implicit animation clock, a minted
/// window id), whose result moves with no write behind it.
pub(in crate::codegen) fn revision_reads(
    program: &LoweredProgram,
    expression_use: ResolvedExpressionId,
    env: &dyn BindingEnvironment,
) -> Option<BTreeSet<String>> {
    let mut reads = BTreeSet::new();
    let mut walk = RevisionWalk {
        program,
        env,
        bound: HashSet::new(),
    };
    let root = program.expressions().expression_use(expression_use).root;
    walk.node(root, &mut reads).then_some(reads)
}

/// The state field a bare key expression reads, as its revision read, when
/// the key is exactly an app or component state field: that revision
/// subsumes the key in the memo tuple.
pub(in crate::codegen) fn bare_state_revision(
    program: &LoweredProgram,
    expression_use: ResolvedExpressionId,
    env: &dyn BindingEnvironment,
) -> Option<String> {
    let expressions = program.expressions();
    let root = expressions.expression_use(expression_use).root;
    let ResolvedExpressionKind::Path {
        root: ResolvedPathRoot::Value(value),
        projections,
    } = &expressions.expression(root).kind
    else {
        return None;
    };
    if !projections.is_empty() {
        return None;
    }
    match value {
        ResolvedValueRef::AppState(_) | ResolvedValueRef::ComponentState(_) => {
            revision_read_code(program, *value, env)
        }
        _ => None,
    }
}

struct RevisionWalk<'a> {
    program: &'a LoweredProgram,
    env: &'a dyn BindingEnvironment,
    /// Locals a builtin body binds inside the expression itself.
    bound: HashSet<ResolvedLocalId>,
}

impl RevisionWalk<'_> {
    /// `false` when the subtree reads something a builder cannot own.
    fn node(&mut self, node: ResolvedExpressionNodeId, reads: &mut BTreeSet<String>) -> bool {
        let expressions = self.program.expressions();
        match &expressions.expression(node).kind {
            ResolvedExpressionKind::Bool(_)
            | ResolvedExpressionKind::I64(_)
            | ResolvedExpressionKind::F64(_)
            | ResolvedExpressionKind::Str(_)
            | ResolvedExpressionKind::Bytes(_)
            | ResolvedExpressionKind::None
            | ResolvedExpressionKind::SlotProvided(_) => true,
            ResolvedExpressionKind::List(items) => items.iter().all(|item| self.node(*item, reads)),
            ResolvedExpressionKind::Path { root, .. } => match root {
                ResolvedPathRoot::Value(value) => self.value(*value, reads),
                ResolvedPathRoot::Local(local) => self.bound.contains(local),
                ResolvedPathRoot::EnumVariant { .. } | ResolvedPathRoot::Palette(_) => true,
            },
            ResolvedExpressionKind::Call { target, arguments } => {
                // A clock read or a minted identity changes with no write
                // behind it, so no revision set can stand in for the value.
                if let ResolvedCallTarget::Builtin(name) = target
                    && crate::hir::recomputation_unsafe_builtin(name, arguments.len())
                {
                    return false;
                }
                for argument in arguments {
                    if let ResolvedCallArgument::Binding(local) = argument {
                        self.bound.insert(*local);
                    }
                }
                arguments.iter().all(|argument| match argument {
                    ResolvedCallArgument::Value(value) => self.node(*value, reads),
                    ResolvedCallArgument::Binding(_) => true,
                })
            }
            ResolvedExpressionKind::Unary { value, .. } => self.node(*value, reads),
            ResolvedExpressionKind::Binary { left, right, .. } => {
                self.node(*left, reads) && self.node(*right, reads)
            }
        }
    }

    fn value(&mut self, value: ResolvedValueRef, reads: &mut BTreeSet<String>) -> bool {
        match value {
            ResolvedValueRef::AppState(_) | ResolvedValueRef::ComponentState(_) => {
                match revision_read_code(self.program, value, self.env) {
                    Some(read) => {
                        reads.insert(read);
                        true
                    }
                    None => false,
                }
            }
            // A secret has no revision: typed input writes its store
            // directly and the wipe ticks nothing, so `empty`/`len` over it
            // must stay a hashed value.
            ResolvedValueRef::Secret(_) => false,
            // A derived value's app-state reads are decided by the checker
            // (`DerivedContract::reads`, the set its cache clears on); its
            // expression is pure by construction, so those revisions are
            // exactly what its value depends on.
            ResolvedValueRef::Derived(id) => {
                for read in &self.program.derived()[id.0 as usize].reads {
                    match revision_read_code(self.program, *read, self.env) {
                        Some(read) => {
                            reads.insert(read);
                        }
                        None => return false,
                    }
                }
                true
            }
            ResolvedValueRef::ComponentParam(_) => {
                let name = &self.program.expressions().value(value).name;
                match self.env.get(&param_revisions_key(name)) {
                    Some(marker) => {
                        reads.extend(
                            marker
                                .code
                                .split(REVISION_SEPARATOR)
                                .filter(|read| !read.is_empty())
                                .map(str::to_owned),
                        );
                        true
                    }
                    None => false,
                }
            }
        }
    }
}
