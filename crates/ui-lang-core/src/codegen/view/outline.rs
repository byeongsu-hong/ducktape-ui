//! Outlining of component instantiations into per-use methods.
//!
//! Component calls expand inline, so an app's whole tree lands in one
//! `__view` body — measured at ~100k generated lines in the ducktape app,
//! where rustc's type check and borrow check are superlinear in function
//! size (~M^1.7: 800 inlined uses = 86s check, the same uses outlined = 17s).
//! When a component use captures nothing from its render site beyond `self`,
//! the palette, and its reconciliation scope, the body is moved to a
//! standalone method and the call site shrinks to
//! `self.__ice_component_use_N(__ice_palette, scope)` — the scope expression
//! stays at the call site, so scopes chained through `for` keys still work.
//!
//! Uses that resolve any local binding (loop variables, lazy dependencies,
//! an enclosing component's context) render inline exactly as before — as
//! does a use whose slot content reaches back to such a binding at the call
//! site, since the content is emitted inside the method the body moved to.
//! So does everything inside a `lazy` closure: its content must be
//! `'static` and an outlined method borrows `self`. Outlining is only active
//! while `__view` is generated — test mounts re-expand inline under
//! `#[cfg(test)]`, which keeps every outlined method reachable from the
//! non-test build.

use std::cell::RefCell;

#[derive(Default)]
struct OutlineState {
    enabled: bool,
    /// Methods emitted while generating test mounts re-render the same view
    /// nodes under `#[cfg(test)]`, so they must carry the attribute — a
    /// method only reachable from a test mount would otherwise typecheck
    /// (and warn as dead code) in non-test builds.
    test_mode: bool,
    lazy_depth: usize,
    counter: usize,
    /// Outlined items paired with the fragment slug of the component (or
    /// lazy block) they were generated from. The slug groups methods into
    /// per-fragment `mod` wrappers that ui-lang-build splits into separate
    /// files: rustc hashes spans into incremental fingerprints, so an edit
    /// in one fragment must not shift the spans of every other fragment's
    /// methods (measured on the ducktape app: a 1-character edit re-checked
    /// everything positioned after it — 12.6 s instead of ~2 s).
    methods: Vec<(String, String)>,
    /// Body-identical methods fold into one definition: the key is the
    /// normalized signature+body text (per-use identifiers rewritten to
    /// positional names), the value is the method that already carries it.
    /// Kept across the view → test-mount passes so a mount can reuse a
    /// non-test method (the reverse cannot happen — the view pass runs
    /// first).
    dedup: std::collections::HashMap<String, String>,
}

thread_local! {
    static OUTLINE: RefCell<OutlineState> = RefCell::new(OutlineState::default());
}

/// Enables outlining for the duration of the guard — held around `__view`
/// generation and (in test mode, continuing the same counter) around test
/// mount generation. Dropping it clears all state, so an errored generation
/// cannot leak methods into the next one.
pub(in crate::codegen) struct OutlineViewGuard;

pub(in crate::codegen) fn enable_for_view() -> OutlineViewGuard {
    OUTLINE.with_borrow_mut(|state| {
        *state = OutlineState {
            enabled: true,
            ..OutlineState::default()
        };
    });
    OutlineViewGuard
}

/// Continues outlining for test mounts: methods gain `#[cfg(test)]`, and the
/// use counter keeps advancing so names never collide with the view's.
pub(in crate::codegen) fn enable_for_test_mounts() -> OutlineViewGuard {
    OUTLINE.with_borrow_mut(|state| {
        let counter = state.counter;
        let dedup = std::mem::take(&mut state.dedup);
        *state = OutlineState {
            enabled: true,
            test_mode: true,
            counter,
            dedup,
            ..OutlineState::default()
        };
    });
    OutlineViewGuard
}

impl Drop for OutlineViewGuard {
    fn drop(&mut self) {
        OUTLINE.with_borrow_mut(|state| {
            let counter = state.counter;
            *state = OutlineState {
                counter,
                ..OutlineState::default()
            };
        });
    }
}

/// Marks rendering inside a `lazy` closure: its content is `'static`, so an
/// outlined `&self` method must not be called from it.
pub(in crate::codegen) struct LazyRenderGuard;

pub(in crate::codegen) fn enter_lazy_render() -> LazyRenderGuard {
    OUTLINE.with_borrow_mut(|state| state.lazy_depth += 1);
    LazyRenderGuard
}

impl Drop for LazyRenderGuard {
    fn drop(&mut self) {
        OUTLINE.with_borrow_mut(|state| {
            state.lazy_depth = state.lazy_depth.saturating_sub(1);
        });
    }
}

pub(in crate::codegen) fn outlining_active() -> bool {
    OUTLINE.with_borrow(|state| state.enabled && state.lazy_depth == 0)
}

/// Stores an outlined method body and returns the method name to call.
/// `scope_locals` are enclosing scope-local identifiers the body references
/// (an enclosing component's context or state scopes); each becomes an owned
/// `String` parameter, cloned at the call site in sorted order.
///
/// Per-use identifiers — the component scope binding (which embeds the call
/// line) and the enclosing scope-local names — are rewritten to positional
/// names first, so uses of the same component whose bodies differ only by
/// those spellings fold into ONE method definition: a component used N times
/// with parameterized arguments costs one typecheck, not N.
#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn push_outlined_method(
    message: &str,
    group: &str,
    scope_binding: &str,
    scope_locals: &std::collections::BTreeSet<String>,
    callback_params: &[(String, String, String)],
    value_params: &[(String, String, String)],
    body: &str,
) -> String {
    // Longest identifier first: one scope name must never rewrite inside a
    // longer one that merely contains it.
    let mut renames: Vec<(String, String)> = scope_locals
        .iter()
        .enumerate()
        .map(|(index, local)| (local.clone(), format!("__ice_ctx_{index}")))
        .collect();
    renames.push((scope_binding.to_owned(), "__ice_use_scope".into()));
    renames.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));
    let mut normalized = body.to_owned();
    for (from, to) in &renames {
        normalized = replace_ident(&normalized, from, to);
    }
    let locals = (0..scope_locals.len())
        .map(|index| format!(", __ice_ctx_{index}: ::std::string::String"))
        .collect::<String>();
    let callbacks = callback_params
        .iter()
        .map(|(ident, sig, _)| format!(", {ident}: {sig}"))
        .collect::<String>();
    let values = value_params
        .iter()
        .map(|(ident, ty, _)| format!(", {ident}: {ty}"))
        .collect::<String>();
    let key = format!(
        "(&self, __ice_palette: __IcePalette, __ice_use_scope: ::std::string::String{locals}{callbacks}{values}) -> __IceElement<'_, {message}> {{ {normalized} }}"
    );
    OUTLINE.with_borrow_mut(|state| {
        if let Some(existing) = state.dedup.get(&key) {
            return existing.clone();
        }
        let name = format!("__ice_component_use_{}", state.counter);
        state.counter += 1;
        let cfg = if state.test_mode {
            "#[cfg(test)]\n"
        } else {
            ""
        };
        // pub(super): the method lives inside a per-fragment `mod` and is
        // called from the include-site module (`__view`) and from sibling
        // fragment mods (nested outlined uses).
        state
            .methods
            .push((group.to_owned(), format!("{cfg}pub(super) fn {name}{key}")));
        state.dedup.insert(key, name.clone());
        name
    })
}

/// Stores an outlined lazy BODY as an associated fn over the memoized
/// dependency tuple plus the hoisted routing context, and returns the fn
/// name. An eager body takes no `self` (the lazy closure is `'static`); a
/// revision-keyed body borrows `self` to read the state it materializes
/// inside the builder — the built element is still `'static`. Bodies fold
/// through the same dedup map as component methods after per-site hoist
/// locals are rewritten to positional names.
pub(in crate::codegen) fn push_lazy_body(
    message: &str,
    group: &str,
    dependency_tuple: &str,
    context_params: &[(String, String)],
    borrows_self: bool,
    body: &str,
) -> String {
    let mut renames: Vec<(String, String)> = context_params
        .iter()
        .enumerate()
        .map(|(index, (local, _))| (local.clone(), format!("__ice_lazy_p{index}")))
        .collect();
    renames.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));
    let mut normalized = body.to_owned();
    for (from, to) in &renames {
        normalized = replace_ident(&normalized, from, to);
    }
    let params = context_params
        .iter()
        .enumerate()
        .map(|(index, (_, ty))| format!(", __ice_lazy_p{index}: {ty}"))
        .collect::<String>();
    let receiver = if borrows_self { "&self, " } else { "" };
    let key = format!(
        "({receiver}__ice_palette: __IcePalette, __dependency: &{dependency_tuple}{params}) -> __IceElement<'static, {message}> {{ {normalized} }}"
    );
    OUTLINE.with_borrow_mut(|state| {
        if let Some(existing) = state.dedup.get(&key) {
            return existing.clone();
        }
        let name = format!("__ice_lazy_body_{}", state.counter);
        state.counter += 1;
        let cfg = if state.test_mode {
            "#[cfg(test)]\n"
        } else {
            ""
        };
        state
            .methods
            .push((group.to_owned(), format!("{cfg}pub(super) fn {name}{key}")));
        state.dedup.insert(key, name.clone());
        name
    })
}

/// Replaces whole-identifier occurrences of `from` with `to`: a scope name
/// must never rewrite inside a longer identifier that merely contains it
/// (`..._scope_2` inside `..._scope_21`).
fn replace_ident(text: &str, from: &str, to: &str) -> String {
    let is_ident = |byte: u8| byte == b'_' || byte.is_ascii_alphanumeric();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find(from) {
        let end = position + from.len();
        let before_ok = position == 0 || !is_ident(rest.as_bytes()[position - 1]);
        let after_ok = end >= rest.len() || !is_ident(rest.as_bytes()[end]);
        out.push_str(&rest[..position]);
        if before_ok && after_ok {
            out.push_str(to);
        } else {
            out.push_str(&rest[position..end]);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Drains the `(fragment slug, item text)` pairs collected while generating
/// `__view` (or test mounts), in emission order.
pub(in crate::codegen) fn drain_outlined_methods() -> Vec<(String, String)> {
    OUTLINE.with_borrow_mut(|state| std::mem::take(&mut state.methods))
}
