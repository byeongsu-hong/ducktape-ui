use crate::ast::*;
use crate::{CheckedDocument, Error};
use std::collections::{HashMap, HashSet};

pub fn analyze(mut document: Document) -> Result<CheckedDocument, Error> {
    check(&mut document)?;
    Ok(CheckedDocument::new(document))
}

fn check(document: &mut Document) -> Result<(), Error> {
    check_unique(document)?;
    check_fonts(document)?;
    check_slots(document)?;
    check_declared_types(document)?;
    check_theme(document)?;
    check_recipes(document)?;
    check_qr_data(document)?;
    if let Some(span) = repeated_pane_grid_span(&document.view) {
        return Err(Error::new(
            "E187",
            span,
            "panes cannot be repeated because each static ID owns one persistent layout state",
        ));
    }

    let states: HashMap<String, Type> = document
        .states
        .iter()
        .map(|state| (state.name.clone(), state.ty.clone()))
        .collect();
    let preset_handlers = document
        .presets
        .iter()
        .map(|preset| Handler {
            name: format!("preset_{}", preset.name),
            params: Vec::new(),
            statements: preset.statements.clone(),
            span: preset.span.clone(),
        })
        .collect::<Vec<_>>();
    for state in &document.states {
        let actual = expr_type(&state.initial, &HashMap::new(), document, &state.span)?;
        if state.ty == Type::Option(Box::new(Type::DebugSpan))
            && !matches!(state.initial, Expr::None)
        {
            return Err(Error::new(
                "E103",
                &state.span,
                "debug span state must start as `none`",
            ));
        } else if let Type::Combo(expected) = &state.ty {
            let Type::List(actual) = actual else {
                return Err(Error::new(
                    "E104",
                    &state.span,
                    "combo state must be initialized with a list",
                ));
            };
            require_type(&actual, expected, &state.span)?;
        } else if let Type::Animation(expected) = &state.ty {
            require_type(&actual, expected, &state.span)?;
            if **expected == Type::F64 {
                require_f32_literal_range(
                    &state.initial,
                    f64::NEG_INFINITY,
                    None,
                    "animation value",
                    &state.span,
                )?;
            }
            if let Some(easing) = state
                .animation
                .as_ref()
                .and_then(|options| options.easing.as_deref())
                && !ANIMATION_EASINGS.contains(&easing)
            {
                let function = extern_function(document, easing, ExternKind::Sync, &state.span)?;
                if function.params.len() != 1
                    || function.params[0].1 != Type::F64
                    || function.output != Type::F64
                    || function.error.is_some()
                {
                    return Err(Error::new(
                        "E103",
                        &state.span,
                        format!(
                            "animation easing `{easing}` must be `sync {easing}(value:f64) -> f64`"
                        ),
                    ));
                }
            }
        } else {
            let text_initial =
                matches!(state.ty, Type::Markdown | Type::Editor) && actual == Type::Str;
            if actual != Type::Unknown && !text_initial && !compatible(&state.ty, &actual) {
                return Err(type_error(&state.span, &state.ty, &actual));
            }
        }
    }
    for component in &document.components {
        for state in &component.states {
            let actual = expr_type(&state.initial, &HashMap::new(), document, &state.span)?;
            if actual != Type::Unknown && !compatible(&state.ty, &actual) {
                return Err(type_error(&state.span, &state.ty, &actual));
            }
        }
    }
    check_app_settings(document, &states)?;
    for handler in document.handlers.iter().chain(&preset_handlers) {
        if let Some(span) = latest_run_span(&handler.statements) {
            return Err(Error::new(
                "E140",
                span,
                "`run latest` is only valid in component handlers",
            ));
        }
        check_structured_tasks(handler)?;
    }
    for component in &document.components {
        for handler in &component.handlers {
            if handler.statements.iter().any(|statement| {
                !matches!(
                    statement,
                    Statement::Assign { .. }
                        | Statement::ReturnIf { .. }
                        | Statement::WidgetOperation {
                            operation: WidgetOperation::Focus { .. }
                                | WidgetOperation::Focused { .. }
                                | WidgetOperation::CursorFront { .. }
                                | WidgetOperation::CursorEnd { .. }
                                | WidgetOperation::Cursor { .. }
                                | WidgetOperation::SelectAll { .. }
                                | WidgetOperation::Select { .. }
                                | WidgetOperation::Snap { .. }
                                | WidgetOperation::SnapEnd { .. }
                                | WidgetOperation::ScrollTo { .. }
                                | WidgetOperation::ScrollBy { .. }
                                | WidgetOperation::Find {
                                    selector: WidgetSelector::Id(_),
                                    ..
                                },
                            ..
                        }
                        | Statement::Run {
                            kind: EffectKind::Future,
                            ..
                        }
                )
            }) {
                return Err(Error::new(
                    "E140",
                    &handler.span,
                    "component handlers support state assignments, scoped widget operations, and `run` futures only",
                ));
            }
        }
    }

    let mut signatures: HashMap<String, Vec<Option<Type>>> = document
        .handlers
        .iter()
        .map(|handler| (handler.name.clone(), vec![None; handler.params.len()]))
        .collect();
    for component in &document.components {
        for handler in &component.handlers {
            signatures.insert(
                component_handler_key(&component.name, &handler.name),
                vec![None; handler.params.len()],
            );
        }
    }

    let mut ids = HashSet::new();
    let mut view_states = states.clone();
    if document.daemon {
        view_states.insert("window".into(), Type::WindowId);
    }
    infer_view(
        &document.view,
        &view_states,
        document,
        &mut signatures,
        &mut ids,
    )?;
    for component in &document.components {
        if let Some(span) = pane_grid_span(&component.root) {
            return Err(Error::new(
                "E187",
                span,
                "panes must live in the app view because it owns persistent layout state",
            ));
        }
        let mut env: HashMap<String, Type> = component
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect();
        env.extend(
            component
                .states
                .iter()
                .map(|state| (state.name.clone(), state.ty.clone())),
        );
        env.insert(component_context_key(&component.name), Type::Unit);
        env.insert(
            component_output_key(&component.name),
            component.output.clone(),
        );
        let mut ids = HashSet::new();
        infer_view(&component.root, &env, document, &mut signatures, &mut ids)?;
    }
    infer_tests(document, &view_states, &mut signatures)?;
    let mut pane_grids = static_pane_grids(&document.view, &view_states, document)?;
    let mut operation_ids = widget_operation_ids(&document.view, &view_states, document)?;
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        pane_grids.extend(static_pane_grids(mount, &view_states, document)?);
        operation_ids.extend(widget_operation_ids(mount, &view_states, document)?);
    }
    controlled_state_bindings(document, false)?;
    controlled_state_bindings(document, true)?;
    infer_subscriptions(document, &states, &mut signatures)?;
    let empty_env = HashMap::new();
    for handler in document.handlers.iter().chain(&preset_handlers) {
        infer_runs(handler, document, &mut signatures, &empty_env)?;
    }
    for component in &document.components {
        let mut env = HashMap::new();
        env.insert(component_context_key(&component.name), Type::Unit);
        for handler in &component.handlers {
            infer_runs(handler, document, &mut signatures, &env)?;
        }
    }

    for handler in &mut document.handlers {
        let inferred = signatures.get(&handler.name).expect("handler signature");
        for (param, inferred) in handler.params.iter_mut().zip(inferred) {
            param.ty = inferred.clone().ok_or_else(|| {
                Error::new(
                    "E102",
                    &handler.span,
                    format!(
                        "cannot infer type of `{}` in handler `{}`",
                        param.name, handler.name
                    ),
                )
                .hint("route a typed widget or action payload to this parameter")
            })?;
        }
    }
    for component in &mut document.components {
        for handler in &mut component.handlers {
            let key = component_handler_key(&component.name, &handler.name);
            let inferred = signatures.get(&key).expect("component handler signature");
            for (param, inferred) in handler.params.iter_mut().zip(inferred) {
                param.ty = inferred.clone().ok_or_else(|| {
                    Error::new(
                        "E102",
                        &handler.span,
                        format!(
                            "cannot infer type of `{}` in component handler `{}.{}`",
                            param.name, component.name, handler.name
                        ),
                    )
                })?;
            }
        }
    }

    for handler in document.handlers.iter().chain(&preset_handlers) {
        check_handler(handler, &states, document, &operation_ids, &pane_grids)?;
    }
    for component in &document.components {
        let mut operation_env: HashMap<String, Type> = component
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect();
        operation_env.extend(
            component
                .states
                .iter()
                .map(|state| (state.name.clone(), state.ty.clone())),
        );
        let operation_ids = widget_operation_ids(&component.root, &operation_env, document)?;
        let states = component
            .states
            .iter()
            .map(|state| (state.name.clone(), state.ty.clone()))
            .collect();
        for handler in &component.handlers {
            check_handler(handler, &states, document, &operation_ids, &HashMap::new())?;
        }
    }
    check_tests(document, &view_states)?;
    Ok(())
}

const COMPONENT_CONTEXT_PREFIX: &str = "\0component:";
const COMPONENT_OUTPUT_PREFIX: &str = "\0component-output:";

fn component_context_key(component: &str) -> String {
    format!("{COMPONENT_CONTEXT_PREFIX}{component}")
}

fn component_output_key(component: &str) -> String {
    format!("{COMPONENT_OUTPUT_PREFIX}{component}")
}

fn component_context(env: &HashMap<String, Type>) -> Option<&str> {
    env.keys()
        .find_map(|name| name.strip_prefix(COMPONENT_CONTEXT_PREFIX))
}

fn component_output(env: &HashMap<String, Type>) -> Option<&Type> {
    env.iter()
        .find_map(|(name, output)| name.starts_with(COMPONENT_OUTPUT_PREFIX).then_some(output))
}

fn component_handler_key(component: &str, handler: &str) -> String {
    format!("{component}.{handler}")
}

fn latest_run_span(statements: &[Statement]) -> Option<&Span> {
    statements.iter().find_map(|statement| match statement {
        Statement::Run {
            latest: true, span, ..
        } => Some(span),
        Statement::TaskGroup { statements, .. } => latest_run_span(statements),
        Statement::Abortable { task, .. } => latest_run_span(::std::slice::from_ref(task.as_ref())),
        _ => None,
    })
}

mod application;
mod canvas;
mod declarations;
mod expr;
mod handler;
mod options;
mod state;
mod style;
mod subscription;
mod testing;
mod view;
mod widgets;

use application::*;
use canvas::*;
use declarations::*;
use handler::*;
use options::*;
pub(crate) use state::controlled_state_bindings;
use state::{check_qr_data, check_theme, pane_grid_span, repeated_pane_grid_span};
use style::*;
use subscription::*;
use testing::*;
use view::*;
use widgets::*;

use expr::check_length_value;
pub(crate) use expr::expr_type;
pub(crate) use handler::task_flow_type;

pub(in crate::check) type WidgetIdPath = Vec<(String, Option<Type>)>;

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
