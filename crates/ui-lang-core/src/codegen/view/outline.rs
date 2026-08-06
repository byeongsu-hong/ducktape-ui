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
//! slot content, an enclosing component's context) render inline exactly as
//! before, as does everything inside a `lazy` closure: its content must be
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
    methods: Vec<String>,
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
        *state = OutlineState {
            enabled: true,
            test_mode: true,
            counter,
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
/// `String` parameter under its original name, cloned at the call site.
pub(in crate::codegen) fn push_outlined_method(
    message: &str,
    scope_binding: &str,
    scope_locals: &std::collections::BTreeSet<String>,
    body: &str,
) -> String {
    OUTLINE.with_borrow_mut(|state| {
        let name = format!("__ice_component_use_{}", state.counter);
        state.counter += 1;
        let locals = scope_locals
            .iter()
            .map(|local| format!(", {local}: ::std::string::String"))
            .collect::<String>();
        let cfg = if state.test_mode {
            "#[cfg(test)]\n"
        } else {
            ""
        };
        // `#[inline(always)]` folds the frames back together at LLVM time
        // (the always-inliner runs even at opt-level 0): outlining exists for
        // rustc's per-item front-end costs, which inlining does not undo, but
        // stacked debug-build call frames tripled the view's runtime stack
        // peak and blew the app's 4 MiB full-view stack contract.
        state.methods.push(format!(
            "{cfg}#[inline(always)]\nfn {name}(&self, __ice_palette: __IcePalette, {scope_binding}: ::std::string::String{locals}) -> __IceElement<'_, {message}> {{ {body} }}"
        ));
        name
    })
}

/// Drains the methods collected while generating `__view`.
pub(in crate::codegen) fn drain_outlined_methods() -> Vec<String> {
    OUTLINE.with_borrow_mut(|state| std::mem::take(&mut state.methods))
}
