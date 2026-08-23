use std::collections::BTreeSet;

use super::*;
use crate::Warning;
use crate::ast::expr_source;
use crate::check::facts::{CheckedExprOwner, CheckedFacts, CheckedViewExprRole};
use crate::check::usage::path_roots;
use crate::hir::{DeclarationIndex, view_children};

/// iced rebuilds the whole view on every message, so anything a view
/// expression computes or clones is paid per view pass unless a `lazy`
/// boundary memoizes it. These warnings name the sites where that cost is
/// proportional to content: a native widget rebuilt from a string or list,
/// and a plain `lazy` that hashes a record or list clone once per loop item.
pub(in crate::check) fn performance_warnings(
    document: &Document,
    reachable_components: &HashSet<String>,
    declarations: &DeclarationIndex,
    facts: &CheckedFacts,
) -> Vec<Warning> {
    let mut warnings = unmemoized_content_warnings(document, reachable_components, |span| {
        lazy_dependency_type(span, declarations, facts)
    });
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
            keys,
            binding,
            span,
            ..
        } => {
            if keys.is_empty()
                && scope.in_loop
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
