use super::*;
use crate::Warning;
use crate::hir::view_children;

pub(in crate::check) fn semantic_smell_warnings(
    document: &Document,
    reachable_components: &HashSet<String>,
    reachable_handlers: &HandlerReachability,
) -> Vec<Warning> {
    let mut warnings = Vec::new();
    for preset in &document.presets {
        statement_smells(&preset.statements, &mut warnings);
    }
    for handler in document
        .handlers
        .iter()
        .filter(|handler| reachable_handlers.app_contains(&handler.name))
    {
        statement_smells(&handler.statements, &mut warnings);
    }
    for component in document
        .components
        .iter()
        .filter(|component| reachable_components.contains(&component.name))
    {
        for handler in component
            .handlers
            .iter()
            .filter(|handler| reachable_handlers.component_contains(&component.name, &handler.name))
        {
            statement_smells(&handler.statements, &mut warnings);
        }
    }

    view_smells(&document.view, &mut warnings);
    for component in document
        .components
        .iter()
        .filter(|component| reachable_components.contains(&component.name))
    {
        view_smells(&component.root, &mut warnings);
    }
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        view_smells(mount, &mut warnings);
    }
    duplicate_subscription_warnings(document, &mut warnings);
    warnings
}

fn statement_smells(statements: &[Statement], warnings: &mut Vec<Warning>) {
    let mut unconditional_return = None;
    for statement in statements {
        if let Some(return_span) = unconditional_return {
            warnings.push(
                Warning::new(
                    "W013",
                    statement.span(),
                    format!(
                        "statement is unreachable because `return if true` at line {} always exits the handler",
                        return_span
                    ),
                )
                .hint("remove the unreachable statements or replace the constant guard"),
            );
            break;
        }
        match statement {
            Statement::Assign {
                target,
                value: Expr::Path(path),
                at: None,
                span,
            } if path.as_slice() == [target.as_str()] => warnings.push(
                Warning::new(
                    "W012",
                    span,
                    format!("assignment `{target} = {target}` has no effect"),
                )
                .hint("remove the assignment or write the intended new value"),
            ),
            Statement::ReturnIf {
                condition: Expr::Bool(false),
                span,
            } => warnings.push(
                Warning::new("W012", span, "`return if false` never exits the handler")
                    .hint("remove the constant guard or use the intended condition"),
            ),
            Statement::ReturnIf {
                condition: Expr::Bool(true),
                span,
            } => unconditional_return = Some(span.line),
            Statement::TaskGroup { statements, .. } => statement_smells(statements, warnings),
            Statement::Match { arms, .. } => {
                for arm in arms {
                    statement_smells(&arm.statements, warnings);
                }
            }
            Statement::Abortable { task, .. } => {
                statement_smells(std::slice::from_ref(task), warnings);
            }
            _ => {}
        }
    }
}

fn duplicate_subscription_warnings(document: &Document, warnings: &mut Vec<Warning>) {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for subscription in &document.subscriptions {
        if matches!(subscription.condition, Some(Expr::Bool(false))) {
            continue;
        }
        let key = format!(
            "{:?}",
            (
                &subscription.source,
                subscription.window_id,
                &subscription.context,
                &subscription.filter,
                &subscription.condition,
                subscription.status,
                &subscription.route.handler,
                &subscription.route.args,
            )
        );
        if let Some(first_line) = seen.get(&key) {
            warnings.push(
                Warning::new(
                    "W014",
                    &subscription.span,
                    format!(
                        "subscription duplicates the same source, gates, and route declared at line {first_line}"
                    ),
                )
                .hint("remove one subscription so each external event is delivered once"),
            );
        } else {
            seen.insert(key, subscription.span.line);
        }
    }
}

fn view_smells(node: &ViewNode, warnings: &mut Vec<Warning>) {
    match node {
        ViewNode::If {
            condition: Expr::Bool(value),
            span,
            ..
        } => {
            let (message, hint) = if *value {
                (
                    "`if true` is a redundant view gate",
                    "remove the gate and keep its children",
                )
            } else {
                (
                    "`if false` makes its entire view subtree unreachable",
                    "remove the dead subtree or use the intended condition",
                )
            };
            warnings.push(Warning::new("W012", span, message).hint(hint));
            // A dead subtree is reported once, at its gate. Walking into it
            // would report every smell inside the code the user is being told
            // to delete.
            if !*value {
                return;
            }
        }
        ViewNode::For { items, span, .. }
            if matches!(items, Expr::EmptyList)
                || matches!(items, Expr::List(values) if values.is_empty()) =>
        {
            warnings.push(
                Warning::new(
                    "W012",
                    span,
                    "repetition over a constant empty list never renders its subtree",
                )
                .hint("remove the dead subtree or provide a non-empty list"),
            );
            return;
        }
        _ => {}
    }

    for child in view_children(node) {
        view_smells(child, warnings);
    }
}
