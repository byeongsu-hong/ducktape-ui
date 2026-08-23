use std::collections::BTreeSet;

use super::*;
use crate::Warning;
use crate::ast::expr_source;
use crate::check::facts::{
    CheckedCallArgument, CheckedCallTarget, CheckedExprKind, CheckedExprOwner, CheckedFacts,
    CheckedPathRoot, CheckedSubscriptionExprRole, CheckedValueRef, CheckedViewExprRole,
    CheckedViewScope,
};
use crate::check::usage::path_roots;
use crate::hir::{DeclarationIndex, OriginArena, ViewId, view_children};

/// iced rebuilds the whole view on every message, so anything a view
/// expression computes or clones is paid per view pass unless a `lazy`
/// boundary memoizes it. These warnings name the sites where that cost is
/// proportional to content: a native widget rebuilt from a string or list,
/// a plain `lazy` that hashes a record or list clone once per loop item, and
/// a string, bytes, list, or editor state cloned into a by-value extern parameter.
pub(in crate::check) fn performance_warnings(
    document: &Document,
    reachable_components: &HashSet<String>,
    declarations: &DeclarationIndex,
    facts: &CheckedFacts,
    origins: &OriginArena,
) -> Vec<Warning> {
    let mut warnings = unmemoized_content_warnings(document, reachable_components, |span| {
        lazy_dependency_type(span, declarations, facts)
    });
    by_value_state_clone_warnings(
        document,
        reachable_components,
        declarations,
        facts,
        origins,
        &mut warnings,
    );
    warnings.sort_by_key(|warning| (warning.line, warning.column));
    warnings
}

fn lazy_dependency_type(
    span: &Span,
    declarations: &DeclarationIndex,
    facts: &CheckedFacts,
) -> Option<Type> {
    let view = declarations.view_id(span)?;
    let use_id = facts.expression_use_by_owner(CheckedExprOwner::View {
        view,
        role: CheckedViewExprRole::LazyDependency,
    })?;
    Some(facts.try_expression_use(use_id)?.source.clone())
}

/// Where a view-time read ultimately comes from, after following loop items,
/// match payloads, and table rows back to what they iterate.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Root {
    AppState(String),
    Derived(String),
    ComponentState(String),
    Param(String),
}

struct ViewScope<'a> {
    host: Option<&'a Component>,
    /// Loop items, match payloads, and table rows bound above this node,
    /// each already resolved to the roots of the expression it iterates.
    bindings: Vec<(String, Vec<Root>)>,
    in_loop: bool,
}

impl ViewScope<'_> {
    fn roots(&self, expr: &Expr, document: &Document) -> Vec<Root> {
        let mut names = Vec::new();
        path_roots(expr, &mut names);
        let mut roots = Vec::new();
        for name in names {
            if let Some((_, bound)) = self.bindings.iter().rev().find(|(bound, _)| *bound == name) {
                roots.extend(bound.iter().cloned());
                continue;
            }
            let root = match self.host {
                Some(host) if host.params.iter().any(|param| param.name == name) => {
                    Root::Param(name)
                }
                Some(host) if host.states.iter().any(|state| state.name == name) => {
                    Root::ComponentState(name)
                }
                Some(_) => continue,
                None if document.states.iter().any(|state| state.name == name) => {
                    Root::AppState(name)
                }
                None if document.derived.iter().any(|derived| derived.name == name) => {
                    Root::Derived(name)
                }
                None => continue,
            };
            roots.push(root);
        }
        roots
    }
}

/// A plain `lazy` clones and hashes its dependency on every pass. For a
/// record of scalars and strings that is the whole-row memo idiom (the
/// repository's own examples pin it with frame contracts); it is a waste
/// only when the clone walks a collection.
fn owns_collection(ty: &Type, document: &Document) -> bool {
    match ty {
        Type::List(_) => true,
        Type::Option(inner) => owns_collection(inner, document),
        Type::Named(name) => document
            .structs
            .iter()
            .find(|item| item.name == *name)
            .is_some_and(|item| {
                item.fields
                    .iter()
                    .any(|(_, field)| matches!(field, Type::List(_)))
            }),
        _ => false,
    }
}

/// Component params that feed an unmemoized extern component's content,
/// keyed by component name, then param name, valued by the names of the
/// externs that consume it.
type ContentParams = HashMap<String, HashMap<String, BTreeSet<String>>>;

fn unmemoized_content_warnings(
    document: &Document,
    reachable_components: &HashSet<String>,
    lazy_dependency_type: impl Fn(&Span) -> Option<Type>,
) -> Vec<Warning> {
    let mut content_params = ContentParams::new();
    // A component's content params are only known once every component it
    // mounts has been walked; iterate until no call site adds a new one.
    loop {
        let mut warnings = Vec::new();
        let mut changed = false;
        let mut walk = |host: Option<&Component>, root: &ViewNode| {
            let mut scope = ViewScope {
                host,
                bindings: Vec::new(),
                in_loop: false,
            };
            content_walk(
                root,
                &mut scope,
                document,
                &mut content_params,
                &lazy_dependency_type,
                &mut warnings,
                &mut changed,
            );
        };
        walk(None, &document.view);
        for component in document
            .components
            .iter()
            .filter(|component| reachable_components.contains(&component.name))
        {
            walk(Some(component), &component.root);
        }
        if !changed {
            return warnings;
        }
    }
}

fn content_walk(
    node: &ViewNode,
    scope: &mut ViewScope<'_>,
    document: &Document,
    content_params: &mut ContentParams,
    lazy_dependency_type: &impl Fn(&Span) -> Option<Type>,
    warnings: &mut Vec<Warning>,
    changed: &mut bool,
) {
    let mut pushed = 0;
    let mut bind = |scope: &mut ViewScope<'_>, name: &str, source: &Expr| {
        let roots = scope.roots(source, document);
        scope.bindings.push((name.to_owned(), roots));
        pushed += 1;
    };
    let was_in_loop = scope.in_loop;
    match node {
        ViewNode::Lazy {
            dependency,
            keyed,
            binding,
            span,
            ..
        } => {
            // A value rooted in state keys the memo off state revisions and
            // is cloned only on a miss; the per-pass clone is the row-local
            // value — the loop item, a match payload, a table row.
            let mut names = Vec::new();
            path_roots(dependency, &mut names);
            let reads_row_local = names
                .iter()
                .any(|name| scope.bindings.iter().any(|(bound, _)| bound == name));
            if !keyed
                && scope.in_loop
                && reads_row_local
                && lazy_dependency_type(span).is_some_and(|ty| owns_collection(&ty, document))
            {
                let dependency = expr_source(dependency);
                warnings.push(
                    Warning::new(
                        "W017",
                        span,
                        format!(
                            "lazy hashes a clone of `{dependency}` on every view pass; key it with `lazy {dependency} by <cheap keys> as {binding}`"
                        ),
                    )
                    .hint("the keyed form captures the value by reference and hashes only the keys"),
                );
            }
            // A lazy child is memoized: nothing inside it runs per view pass.
            return;
        }
        ViewNode::For { item, items, .. } => {
            bind(scope, item, items);
            scope.in_loop = true;
        }
        ViewNode::KeyedColumn { item, items, .. } => {
            bind(scope, item, items);
            scope.in_loop = true;
        }
        ViewNode::Table { item, rows, .. } => {
            bind(scope, item, rows);
            scope.in_loop = true;
        }
        ViewNode::Match { value, arms, .. } => {
            for binding in arms.iter().filter_map(|arm| arm.pattern.binding()) {
                bind(scope, binding, value);
            }
        }
        ViewNode::ExternComponent {
            function,
            args,
            span,
            ..
        } => {
            let declaration = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Component);
            // A component that borrows any parameter reads app state in
            // place by design: it is a live control, not parsed content.
            if let Some(declaration) = declaration
                && !declaration.borrowed.iter().any(|borrowed| *borrowed)
            {
                for ((_, ty), arg) in declaration.params.iter().zip(args) {
                    if !matches!(ty, Type::Str | Type::List(_)) {
                        continue;
                    }
                    content_site(
                        function,
                        None,
                        arg,
                        span,
                        scope,
                        document,
                        content_params,
                        warnings,
                        changed,
                    );
                }
            }
        }
        ViewNode::Component {
            name, args, span, ..
        } => {
            let params = content_params.get(name).cloned().unwrap_or_default();
            for arg in args {
                for function in params.get(&arg.name).into_iter().flatten() {
                    content_site(
                        function,
                        Some(format!("{name}.{}", arg.name)),
                        &arg.value,
                        span,
                        scope,
                        document,
                        content_params,
                        warnings,
                        changed,
                    );
                }
            }
        }
        _ => {}
    }
    for child in view_children(node) {
        content_walk(
            child,
            scope,
            document,
            content_params,
            lazy_dependency_type,
            warnings,
            changed,
        );
    }
    scope.bindings.truncate(scope.bindings.len() - pushed);
    scope.in_loop = was_in_loop;
}

#[allow(clippy::too_many_arguments)]
fn content_site(
    function: &str,
    through: Option<String>,
    arg: &Expr,
    span: &Span,
    scope: &ViewScope<'_>,
    document: &Document,
    content_params: &mut ContentParams,
    warnings: &mut Vec<Warning>,
    changed: &mut bool,
) {
    let mut reads_state = false;
    for root in scope.roots(arg, document) {
        match root {
            Root::AppState(_) | Root::Derived(_) | Root::ComponentState(_) => reads_state = true,
            Root::Param(param) => {
                let host = scope
                    .host
                    .expect("a component parameter resolves inside its component");
                *changed |= content_params
                    .entry(host.name.clone())
                    .or_default()
                    .entry(param)
                    .or_default()
                    .insert(function.to_owned());
            }
        }
    }
    if !reads_state {
        return;
    }
    let arg = expr_source(arg);
    let through = through
        .map(|param| format!(" (through `{param}`)"))
        .unwrap_or_default();
    warnings.push(
        Warning::new(
            "W016",
            span,
            format!(
                "`{function}`{through} rebuilds its native widget from `{arg}` on every view pass; wrap it in `lazy {arg} as <alias>` so it is rebuilt only when that content changes"
            ),
        )
        .hint(format!(
            "a list can be keyed with `lazy {arg} by <cheap keys> as <alias>` so only the keys are hashed per pass"
        )),
    );
}

/// A by-value `pure`/`sync` parameter clones its argument; when the argument
/// is a state field read straight from a per-pass owner, that clone is paid
/// once per frame (or once per subscription check) for nothing the borrowed
/// `&type` form would not give.
fn by_value_state_clone_warnings(
    document: &Document,
    reachable_components: &HashSet<String>,
    declarations: &DeclarationIndex,
    facts: &CheckedFacts,
    origins: &OriginArena,
    warnings: &mut Vec<Warning>,
) {
    let call_views = facts
        .views()
        .iter()
        .filter_map(|view| Some((declarations.component_call_id(view.id)?, view.id)))
        .collect::<HashMap<_, _>>();
    let reachable = reachable_components
        .iter()
        .filter_map(|name| declarations.component_id(name))
        .collect::<HashSet<_>>();
    // A test mount renders once, and an unreachable component never; every
    // other view expression is re-evaluated per view pass. A `lazy` subtree
    // needs no check here: only its alias and key locals are in scope there,
    // so no state field can be named under one.
    let per_pass_view = |view: ViewId| match facts.view(view).scope {
        CheckedViewScope::App => true,
        CheckedViewScope::Component(component) => reachable.contains(&component),
        CheckedViewScope::Test(_) => false,
    };
    for expr in facts.expressions() {
        let CheckedExprKind::Call {
            target: CheckedCallTarget::Extern(function),
            arguments,
        } = &expr.kind
        else {
            continue;
        };
        let Some(declaration) = declarations.try_extern_decl(function.id) else {
            continue;
        };
        if !matches!(declaration.kind, ExternKind::Pure | ExternKind::Sync) {
            continue;
        }
        // Handlers run once per message, and a `derived` value is about to be
        // cached across frames (feat/derived-cache), so only view-owned and
        // subscription-owned calls are per pass.
        let per_pass = |view| per_pass_view(view).then_some("view pass");
        let cadence = match facts.expression_use(expr.owner).owner {
            CheckedExprOwner::View { view, .. } => per_pass(view),
            CheckedExprOwner::Interaction(id) => per_pass(id.widget),
            CheckedExprOwner::Media(id) => per_pass(id.media),
            CheckedExprOwner::Tooltip(id) => per_pass(id.tooltip),
            CheckedExprOwner::Float(id) => per_pass(id.float),
            CheckedExprOwner::Pin(id) => per_pass(id.pin),
            CheckedExprOwner::ComponentArgument { call, .. } => {
                call_views.get(&call).and_then(|view| per_pass(*view))
            }
            CheckedExprOwner::Subscription {
                role: CheckedSubscriptionExprRole::Condition,
                ..
            } => Some("subscription check"),
            _ => None,
        };
        let Some(cadence) = cadence else {
            continue;
        };
        report_by_value_arguments(
            document,
            facts,
            origins,
            declaration,
            arguments,
            expr.origin,
            cadence,
            warnings,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn report_by_value_arguments(
    document: &Document,
    facts: &CheckedFacts,
    origins: &OriginArena,
    declaration: &crate::hir::ExternDeclaration,
    arguments: &[CheckedCallArgument],
    origin: crate::hir::OriginId,
    cadence: &str,
    warnings: &mut Vec<Warning>,
) {
    for (argument, borrowed) in arguments.iter().zip(&declaration.borrowed) {
        if *borrowed {
            continue;
        }
        let CheckedCallArgument::Value(argument) = argument else {
            continue;
        };
        // Only a bare state read is a whole-value clone the callee could
        // borrow; a projection already narrows to one field.
        let CheckedExprKind::Path {
            root:
                CheckedPathRoot::Value(
                    value @ (CheckedValueRef::AppState(_) | CheckedValueRef::ComponentState(_)),
                ),
            projections,
        } = &facts.expression(*argument).kind
        else {
            continue;
        };
        if !projections.is_empty() {
            continue;
        }
        let value = facts.value_by_ref(*value);
        if !matches!(value.ty, Type::Str | Type::Bytes | Type::Editor)
            && !owns_collection(&value.ty, document)
        {
            continue;
        }
        let origin = origins.get(origin);
        let span = Span {
            line: origin.line,
            column: origin.column,
        };
        let ty = value.ty.display();
        warnings.push(
            Warning::new(
                "W018",
                &span,
                format!(
                    "the {ty} state `{}` is cloned for `{}` on every {cadence}; declare the parameter borrowed (`&{ty}`) or move the result into state in the handler that writes `{}`",
                    value.name, declaration.name, value.name
                ),
            )
            .hint("a `pure` or `sync` parameter declared `&str`, `&bytes`, `&[T]`, `&T`, or `&editor` receives a reference; the call site is unchanged"),
        );
    }
}
