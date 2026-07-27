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
    let derived = check_derived(document, &states)?;
    let mut app_values = states.clone();
    app_values.extend(derived);
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
        let mut saw_default = false;
        for param in &component.params {
            if let Some(default) = &param.default {
                saw_default = true;
                if param.bind {
                    return Err(Error::new(
                        "E103",
                        &component.span,
                        format!("bind prop `{}` cannot declare a default", param.name),
                    ));
                }
                if !component_value_is_cloneable(&param.ty) {
                    return Err(Error::new(
                        "E103",
                        &component.span,
                        format!(
                            "component prop `{}` cannot default a mutable value of type `{}`",
                            param.name,
                            param.ty.display()
                        ),
                    ));
                }
                if let Some(function) = sync_extern_call(default, document) {
                    return Err(Error::new(
                        "E103",
                        &component.span,
                        format!(
                            "component prop `{}` default cannot call extern function `{function}`",
                            param.name
                        ),
                    ));
                }
                let actual = expr_type(default, &HashMap::new(), document, &component.span)?;
                require_type(&actual, &param.ty, &component.span)?;
            } else if saw_default {
                return Err(Error::new(
                    "E103",
                    &component.span,
                    format!(
                        "required prop `{}` cannot follow a prop with a default",
                        param.name
                    ),
                ));
            }
        }
        for state in &component.states {
            let actual = expr_type(&state.initial, &HashMap::new(), document, &state.span)?;
            if actual != Type::Unknown && !compatible(&state.ty, &actual) {
                return Err(type_error(&state.span, &state.ty, &actual));
            }
        }
    }
    check_app_settings(document, &states)?;
    for handler in document.handlers.iter().chain(&preset_handlers) {
        if let Some((mode, span)) = scoped_run(&handler.statements) {
            let keyword = match mode {
                FutureMode::Latest => "latest",
                FutureMode::Replace => "replace",
                FutureMode::Every => unreachable!(),
            };
            return Err(Error::new(
                "E140",
                span,
                format!("`run {keyword}` is only valid in component handlers"),
            ));
        }
        check_structured_tasks(handler)?;
    }
    for component in &document.components {
        for handler in &component.handlers {
            if handler.statements.iter().any(|statement| {
                !matches!(
                    statement,
                    Statement::Let { .. }
                        | Statement::Assign { .. }
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
    let mut view_states = app_values.clone();
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
        env.extend(
            slots(&component.root)
                .into_iter()
                .map(|(name, _, _)| (format!("\0slot-provided:{name}"), Type::Bool)),
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
        infer_runs(handler, document, &mut signatures, &app_values, &empty_env)?;
    }
    for component in &document.components {
        let values = component
            .states
            .iter()
            .map(|state| (state.name.clone(), state.ty.clone()))
            .collect();
        let env = HashMap::from([(component_context_key(&component.name), Type::Unit)]);
        for handler in &component.handlers {
            infer_runs(handler, document, &mut signatures, &values, &env)?;
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
        check_handler(
            handler,
            &states,
            &app_values,
            document,
            &operation_ids,
            &pane_grids,
        )?;
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
            check_handler(
                handler,
                &states,
                &states,
                document,
                &operation_ids,
                &HashMap::new(),
            )?;
        }
    }
    check_tests(document, &view_states)?;
    Ok(())
}

fn sync_extern_call<'a>(expr: &'a Expr, document: &Document) -> Option<&'a str> {
    match expr {
        Expr::Call { name, args } => document
            .functions
            .iter()
            .any(|function| function.name == *name && function.kind == ExternKind::Sync)
            .then_some(name.as_str())
            .or_else(|| {
                args.iter()
                    .find_map(|argument| sync_extern_call(argument, document))
            }),
        Expr::List(values) => values
            .iter()
            .find_map(|value| sync_extern_call(value, document)),
        Expr::Unary { value, .. } => sync_extern_call(value, document),
        Expr::Binary { left, right, .. } => {
            sync_extern_call(left, document).or_else(|| sync_extern_call(right, document))
        }
        Expr::Bool(_)
        | Expr::I64(_)
        | Expr::F64(_)
        | Expr::Str(_)
        | Expr::Bytes(_)
        | Expr::EmptyList
        | Expr::None
        | Expr::Path(_) => None,
    }
}

fn check_derived(
    document: &mut Document,
    states: &HashMap<String, Type>,
) -> Result<HashMap<String, Type>, Error> {
    fn dependencies(expr: &Expr, names: &HashMap<String, usize>, output: &mut Vec<usize>) {
        match expr {
            Expr::Path(path) => {
                if let Some(index) = path.first().and_then(|name| names.get(name)) {
                    output.push(*index);
                }
            }
            Expr::List(values) | Expr::Call { args: values, .. } => {
                for value in values {
                    dependencies(value, names, output);
                }
            }
            Expr::Unary { value, .. } => dependencies(value, names, output),
            Expr::Binary { left, right, .. } => {
                dependencies(left, names, output);
                dependencies(right, names, output);
            }
            Expr::Bool(_)
            | Expr::I64(_)
            | Expr::F64(_)
            | Expr::Str(_)
            | Expr::Bytes(_)
            | Expr::EmptyList
            | Expr::None => {}
        }
    }

    fn contains_unknown(ty: &Type) -> bool {
        match ty {
            Type::Unknown => true,
            Type::List(inner)
            | Type::Option(inner)
            | Type::Combo(inner)
            | Type::Animation(inner) => contains_unknown(inner),
            Type::Result(output, error) => contains_unknown(output) || contains_unknown(error),
            _ => false,
        }
    }

    fn visit(
        index: usize,
        document: &Document,
        states: &HashMap<String, Type>,
        names: &HashMap<String, usize>,
        marks: &mut [u8],
        types: &mut [Option<Type>],
    ) -> Result<Type, Error> {
        if marks[index] == 1 {
            return Err(Error::new(
                "E103",
                &document.derived[index].span,
                format!(
                    "derived value `{}` has a dependency cycle",
                    document.derived[index].name
                ),
            ));
        }
        if let Some(ty) = &types[index] {
            return Ok(ty.clone());
        }
        marks[index] = 1;
        let derived = &document.derived[index];
        if sync_extern_call(&derived.value, document).is_some() {
            return Err(Error::new(
                "E103",
                &derived.span,
                format!(
                    "derived value `{}` must use a pure Ice expression",
                    derived.name
                ),
            ));
        }
        let mut env = states.clone();
        let mut deps = Vec::new();
        dependencies(&derived.value, names, &mut deps);
        for dependency in deps {
            let ty = visit(dependency, document, states, names, marks, types)?;
            env.insert(document.derived[dependency].name.clone(), ty);
        }
        let ty = expr_type(&derived.value, &env, document, &derived.span)?;
        if contains_unknown(&ty) {
            return Err(Error::new(
                "E103",
                &derived.span,
                format!("cannot infer type of derived value `{}`", derived.name),
            ));
        }
        if !component_value_is_cloneable(&ty) {
            return Err(Error::new(
                "E103",
                &derived.span,
                format!(
                    "derived value `{}` must produce an ordinary cloneable value",
                    derived.name
                ),
            ));
        }
        marks[index] = 2;
        types[index] = Some(ty.clone());
        Ok(ty)
    }

    let names = document
        .derived
        .iter()
        .enumerate()
        .map(|(index, derived)| (derived.name.clone(), index))
        .collect::<HashMap<_, _>>();
    let mut marks = vec![0; document.derived.len()];
    let mut types = vec![None; document.derived.len()];
    for index in 0..document.derived.len() {
        visit(index, document, states, &names, &mut marks, &mut types)?;
    }
    let mut env = HashMap::new();
    for (derived, ty) in document.derived.iter_mut().zip(types) {
        let ty = ty.expect("every derived value was visited");
        derived.ty = ty.clone();
        env.insert(derived.name.clone(), ty);
    }
    Ok(env)
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

fn scoped_run(statements: &[Statement]) -> Option<(FutureMode, &Span)> {
    statements.iter().find_map(|statement| match statement {
        Statement::Run { mode, span, .. } if *mode != FutureMode::Every => Some((*mode, span)),
        Statement::TaskGroup { statements, .. } => scoped_run(statements),
        Statement::Abortable { task, .. } => scoped_run(::std::slice::from_ref(task.as_ref())),
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

pub(crate) use expr::expr_type;
use expr::{check_length_value, contains_ui_enum};
pub(crate) use handler::task_flow_type;

pub(in crate::check) type WidgetIdPath = Vec<(String, Option<Type>)>;

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
