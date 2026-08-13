use crate::ast::*;
use crate::semantic::*;
use crate::{CheckedDocument, Error};
#[cfg(test)]
use std::cell::Cell;
use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(test)]
thread_local! {
    static HANDLER_SIGNATURE_WORKLIST_VISITS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_handler_signature_worklist_visits() {
    HANDLER_SIGNATURE_WORKLIST_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn handler_signature_worklist_visits() -> usize {
    HANDLER_SIGNATURE_WORKLIST_VISITS.get()
}

struct CheckOutput {
    analyses: facts::CheckedAnalyses,
    controlled_inputs: Vec<crate::hir::AppStateId>,
    controlled_editors: Vec<crate::CheckedControlledEditor>,
}

pub fn analyze(mut document: Document) -> Result<CheckedDocument, Error> {
    let reachable = reachable_components(&document);
    let reachable_handlers = reachable_handlers(&document, &reachable);
    let usage = UsageSession::start(&document, &reachable, &reachable_handlers);
    let mut origins = crate::hir::OriginArena::default();
    let mut declarations = crate::hir::DeclarationIndex::build(&document, &mut origins);
    let checked = check(
        &mut document,
        &reachable,
        &reachable_handlers,
        &declarations,
    )?;
    declarations.finalize_checked_handlers(&document)?;
    let facts =
        without_usage(|| facts::build(&document, &declarations, &mut origins, checked.analyses))?;
    let mut warnings = unreachable_component_warnings(&document, &reachable);
    warnings.extend(unreachable_handler_warnings(
        &document,
        &reachable,
        &reachable_handlers,
    ));
    warnings.extend(usage.finish());
    warnings.extend(immediate_handler_cycle_warnings(
        &document,
        &reachable_handlers,
    ));
    warnings.extend(routed_task_cycle_warnings(&document, &reachable_handlers));
    warnings.extend(raw_event_feedback_warnings(&document));
    warnings.extend(component_identity_warnings(&document));
    warnings.extend(unscoped_component_widget_warnings(&document, &reachable));
    warnings.extend(semantic_smell_warnings(
        &document,
        &reachable,
        &reachable_handlers,
    ));
    warnings.sort_by_key(|warning| warning.line);
    Ok(CheckedDocument::new(
        document,
        facts,
        declarations,
        origins,
        warnings,
        reachable,
        reachable_handlers.app,
        checked.controlled_inputs,
        checked.controlled_editors,
    ))
}

pub(crate) fn component_slots(node: &ViewNode) -> Vec<(&str, bool, &Span)> {
    declarations::slots(node)
}

fn check(
    document: &mut Document,
    reachable: &HashSet<String>,
    reachable_handlers: &HandlerReachability,
    declarations: &crate::hir::DeclarationIndex,
) -> Result<CheckOutput, Error> {
    check_unique(document)?;
    check_fonts(document)?;
    check_slots(document)?;
    check_declared_types(document)?;
    check_theme(document)?;
    check_recipes(document)?;
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
    let mut initializer_analyses = facts::CheckedAnalyses::default();
    let derived = without_usage(|| {
        check_derived(document, &states, declarations, &mut initializer_analyses)
    })?;
    let mut app_values = states.clone();
    app_values.extend(derived);
    // Secrets join the expression environment but never `states`: the checked
    // state list is what presets construct and what a test's `expect` reads a
    // typed field from, and a secret has no field there to read.
    app_values.extend(
        document
            .secrets
            .iter()
            .map(|secret| (secret.name.clone(), Type::Secret)),
    );
    let preset_handlers = document
        .presets
        .iter()
        .map(preset_handler)
        .collect::<Vec<_>>();
    check_run_lanes(document)?;
    let empty_initializer_env = HashMap::new();
    let initializer_env = SyncTypeEnv::new(&empty_initializer_env);
    for (index, state) in document.states.iter().enumerate() {
        let analysis =
            expr::analyze_expr_types(&state.initial, &initializer_env, document, &state.span)?;
        let actual = analysis
            .type_of(&state.initial)
            .cloned()
            .ok_or_else(|| Error::new("E196", &state.span, "missing checked state type"))?;
        initializer_analyses.insert(
            facts::CheckedValueRef::AppState(declarations.app_state(index).id),
            analysis,
        )?;
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
            check_animation_state(state, expected, document)?;
        } else {
            let text_initial =
                matches!(state.ty, Type::Markdown | Type::Editor) && actual == Type::Str;
            if actual != Type::Unknown && !text_initial && !compatible(&state.ty, &actual) {
                return Err(type_error(&state.span, &state.ty, &actual));
            }
        }
    }
    for (component_index, component) in document.components.iter().enumerate() {
        let component_id = declarations.component(component_index).id;
        let mut saw_default = false;
        for (param_index, param) in component.params.iter().enumerate() {
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
                if let Some(function) = recomputation_unsafe_builtin_call(default, document) {
                    return Err(Error::new(
                        "E103",
                        &component.span,
                        format!(
                            "component prop `{}` default cannot call recomputation-unsafe builtin `{function}`",
                            param.name
                        ),
                    )
                    .hint("capture the runtime value in app state and pass it as an explicit prop"));
                }
                let analysis =
                    expr::analyze_expr_types(default, &HashMap::new(), document, &component.span)?;
                let actual = analysis.type_of(default).cloned().ok_or_else(|| {
                    Error::new("E196", &component.span, "missing checked default type")
                })?;
                require_type(&actual, &param.ty, &component.span)?;
                initializer_analyses.insert(
                    facts::CheckedValueRef::ComponentParam(
                        declarations.component_param(component_id, param_index).id,
                    ),
                    analysis,
                )?;
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
        for (state_index, state) in component.states.iter().enumerate() {
            if let Some(function) = recomputation_unsafe_builtin_call(&state.initial, document) {
                return Err(Error::new(
                    "E103",
                    &state.span,
                    format!(
                        "component state `{}` initializer cannot call recomputation-unsafe builtin `{function}`",
                        state.name
                    ),
                )
                .hint("capture the runtime value in app state or assign component state from a handler"));
            }
            let analysis =
                expr::analyze_expr_types(&state.initial, &HashMap::new(), document, &state.span)?;
            let actual = analysis.type_of(&state.initial).cloned().ok_or_else(|| {
                Error::new("E196", &state.span, "missing checked component state type")
            })?;
            if let Type::Animation(expected) = &state.ty {
                require_type(&actual, expected, &state.span)?;
                check_animation_state(state, expected, document)?;
            } else if actual != Type::Unknown && !compatible(&state.ty, &actual) {
                return Err(type_error(&state.span, &state.ty, &actual));
            }
            initializer_analyses.insert(
                facts::CheckedValueRef::ComponentState(
                    declarations.component_state(component_id, state_index).id,
                ),
                analysis,
            )?;
        }
    }
    check_app_settings(document, &app_values, &mut initializer_analyses)?;
    for handler in document.handlers.iter().chain(&preset_handlers) {
        check_structured_tasks(handler)?;
    }
    for component in &document.components {
        for handler in &component.handlers {
            check_structured_tasks(handler)?;
            if let Some(span) = handler.statements.iter().find_map(component_stream_every) {
                return Err(Error::new(
                    "E140",
                    span,
                    "component handlers cannot use `stream every`",
                )
                .hint(
                    "use `stream replace lane=name ...` so the component owns one replaceable stream",
                ));
            }
            if handler
                .statements
                .iter()
                .any(|statement| !component_handler_statement_supported(statement))
            {
                return Err(Error::new(
                    "E140",
                    &handler.span,
                    "component handlers support state assignments, scoped widget operations, `run` futures, `stream replace`, and task groups composed from those task-producing statements only",
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

    let view_analysis_guard = view::ViewAnalysisGuard::start(document, declarations);
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
            COMPONENT_CONTEXT_INDEX.into(),
            Type::Named(component.name.clone()),
        );
        env.insert(
            component_output_key(&component.name),
            component.output.clone(),
        );
        let mut ids = HashSet::new();
        with_component_scope(&component.name, reachable.contains(&component.name), || {
            infer_view(&component.root, &env, document, &mut signatures, &mut ids)
        })?;
    }
    check_lazy_delivered_routes(document)?;
    infer_tests(document, &view_states, &mut signatures)?;
    let mut pane_grids = static_pane_grids(&document.view, &view_states, document)?;
    let mut operation_ids = widget_operation_ids(&document.view, &view_states, document)?;
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        pane_grids.extend(static_pane_grids(mount, &view_states, document)?);
        operation_ids.extend(widget_operation_ids(mount, &view_states, document)?);
    }
    let controlled_input_names = controlled_state_bindings(document, false)?;
    for binding in &controlled_input_names {
        if let Some(state) = document
            .states
            .iter()
            .find(|state| state.name == binding.as_str())
        {
            record_write(binding, &state.span);
        }
    }
    let controlled_editor_contracts = controlled_editor_bindings(document)?;
    for binding in &controlled_editor_contracts {
        if let Some(state) = document
            .states
            .iter()
            .find(|state| state.name == binding.name)
        {
            record_write(&binding.name, &state.span);
        }
    }
    infer_subscriptions(
        document,
        &states,
        &mut signatures,
        declarations,
        &mut initializer_analyses,
    )?;
    #[derive(Clone, Copy)]
    enum InferenceSource {
        App(usize),
        Preset(usize),
        Component { component: usize, handler: usize },
    }

    let mut sources = Vec::new();
    let mut source_by_signature = HashMap::new();
    for (index, handler) in document.handlers.iter().enumerate() {
        source_by_signature.insert(handler.name.clone(), sources.len());
        sources.push(InferenceSource::App(index));
    }
    for index in 0..preset_handlers.len() {
        sources.push(InferenceSource::Preset(index));
    }
    for (component_index, component) in document.components.iter().enumerate() {
        for (handler_index, handler) in component.handlers.iter().enumerate() {
            source_by_signature.insert(
                component_handler_key(&component.name, &handler.name),
                sources.len(),
            );
            sources.push(InferenceSource::Component {
                component: component_index,
                handler: handler_index,
            });
        }
    }
    let targets = sources
        .iter()
        .map(|source| {
            let (handler, component) = match *source {
                InferenceSource::App(index) => (&document.handlers[index], None),
                InferenceSource::Preset(index) => (&preset_handlers[index], None),
                InferenceSource::Component { component, handler } => (
                    &document.components[component].handlers[handler],
                    Some(document.components[component].name.as_str()),
                ),
            };
            let mut routes = VecDeque::new();
            collect_statement_routes(&handler.statements, &mut routes);
            let mut keys = routes
                .into_iter()
                .map(|handler| {
                    component.map_or_else(
                        || handler.to_owned(),
                        |component| component_handler_key(component, handler),
                    )
                })
                .filter(|key| signatures.contains_key(key))
                .collect::<Vec<_>>();
            keys.sort_unstable();
            keys.dedup();
            keys
        })
        .collect::<Vec<_>>();

    let empty_env = HashMap::new();
    let mut queue = (0..sources.len()).collect::<VecDeque<_>>();
    let mut queued = vec![true; sources.len()];
    let mut errors = (0..sources.len())
        .map(|_| None)
        .collect::<Vec<Option<Error>>>();
    while let Some(source_index) = queue.pop_front() {
        queued[source_index] = false;
        #[cfg(test)]
        HANDLER_SIGNATURE_WORKLIST_VISITS.with(|visits| visits.set(visits.get() + 1));
        let previous_targets = targets[source_index]
            .iter()
            .map(|key| signatures.get(key).cloned())
            .collect::<Vec<_>>();
        let result = match sources[source_index] {
            InferenceSource::App(index) => {
                let handler = &document.handlers[index];
                with_app_handler_scope(reachable_handlers.app_contains(&handler.name), || {
                    infer_runs(handler, document, &mut signatures, &app_values, &empty_env)
                })
            }
            InferenceSource::Preset(index) => infer_runs(
                &preset_handlers[index],
                document,
                &mut signatures,
                &app_values,
                &empty_env,
            ),
            InferenceSource::Component { component, handler } => {
                let component = &document.components[component];
                let handler = &component.handlers[handler];
                let values: HashMap<String, Type> = component
                    .states
                    .iter()
                    .map(|state| (state.name.clone(), state.ty.clone()))
                    .collect();
                let env = HashMap::from([
                    (component_context_key(&component.name), Type::Unit),
                    (
                        COMPONENT_CONTEXT_INDEX.into(),
                        Type::Named(component.name.clone()),
                    ),
                ]);
                with_component_scope(
                    &component.name,
                    reachable.contains(&component.name)
                        && reachable_handlers.component_contains(&component.name, &handler.name),
                    || infer_runs(handler, document, &mut signatures, &values, &env),
                )
            }
        };
        errors[source_index] = result.err();
        for (key, previous) in targets[source_index].iter().zip(previous_targets) {
            if previous.as_ref() == signatures.get(key) {
                continue;
            }
            let Some(&target_source) = source_by_signature.get(key) else {
                continue;
            };
            if !queued[target_source] {
                queued[target_source] = true;
                queue.push_back(target_source);
            }
        }
    }
    if let Some(error) = errors.into_iter().flatten().next() {
        return Err(error);
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

    let handler_analysis_guard = expr::HandlerAnalysisGuard::start();
    for handler in &document.handlers {
        with_app_handler_scope(reachable_handlers.app_contains(&handler.name), || {
            with_handler_usage(None, &handler.name, || {
                infer_runs(handler, document, &mut signatures, &app_values, &empty_env)?;
                check_handler(
                    handler,
                    &states,
                    &app_values,
                    document,
                    &operation_ids,
                    &pane_grids,
                    true,
                )
            })
        })?;
    }
    for handler in &preset_handlers {
        with_handler_usage(None, &handler.name, || {
            infer_runs(handler, document, &mut signatures, &app_values, &empty_env)?;
            check_handler(
                handler,
                &states,
                &app_values,
                document,
                &operation_ids,
                &pane_grids,
                true,
            )
        })?;
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
            with_component_scope(
                &component.name,
                reachable.contains(&component.name)
                    && reachable_handlers.component_contains(&component.name, &handler.name),
                || {
                    with_handler_usage(Some(&component.name), &handler.name, || {
                        let route_env = HashMap::from([
                            (component_context_key(&component.name), Type::Unit),
                            (
                                COMPONENT_CONTEXT_INDEX.into(),
                                Type::Named(component.name.clone()),
                            ),
                        ]);
                        infer_runs(handler, document, &mut signatures, &states, &route_env)?;
                        check_handler(
                            handler,
                            &states,
                            &states,
                            document,
                            &operation_ids,
                            &HashMap::new(),
                            false,
                        )
                    })?;
                    Ok::<_, Error>(())
                },
            )?;
        }
    }
    initializer_analyses.retain_handlers(handler_analysis_guard.finish(), preset_handlers);
    let test_analysis_guard = expr::HandlerAnalysisGuard::start();
    check_tests(document, &view_states)?;
    let mut test_analyses = test_analysis_guard.finish();
    let test_expression_keys = document
        .tests
        .iter()
        .flat_map(|test| {
            test.targets
                .iter()
                .flat_map(|target| crate::ast::widget_target_expression_roots(&target.target))
                .chain(
                    test.steps
                        .iter()
                        .flat_map(crate::ast::test_step_expression_roots),
                )
        })
        .map(expr::expr_key)
        .collect::<HashSet<_>>();
    test_analyses
        .expressions
        .retain(|key, _| test_expression_keys.contains(key));
    initializer_analyses.retain_tests(test_analyses)?;
    initializer_analyses.extend(view_analysis_guard.finish())?;
    let controlled_inputs = controlled_input_names
        .into_iter()
        .map(|name| {
            let index = document
                .states
                .iter()
                .position(|state| state.name == name)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        document.view.span(),
                        "checked input binding is not an app state",
                    )
                })?;
            Ok(declarations.app_state(index).id)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let controlled_editors = controlled_editor_contracts
        .into_iter()
        .map(|binding| {
            let index = document
                .states
                .iter()
                .position(|state| state.name == binding.name)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        document.view.span(),
                        "checked editor binding is not an app state",
                    )
                })?;
            let action = binding
                .action
                .map(|name| {
                    declarations
                        .extern_decl_by_name(&name)
                        .filter(|function| function.kind == ExternKind::EditorAction)
                        .map(|function| function.declaration.id)
                        .ok_or_else(|| {
                            Error::new(
                                "E196",
                                document.view.span(),
                                "checked editor action extern disappeared",
                            )
                        })
                })
                .transpose()?;
            Ok(crate::CheckedControlledEditor {
                state: declarations.app_state(index).id,
                action,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(CheckOutput {
        analyses: initializer_analyses,
        controlled_inputs,
        controlled_editors,
    })
}

/// Checks the settings block an animation state carries, wherever it is
/// declared. `inner` is the animated type, already matched against the
/// initializer.
fn check_animation_state(state: &State, inner: &Type, document: &Document) -> Result<(), Error> {
    if *inner == Type::F64 {
        require_f32_literal_range(
            &state.initial,
            f64::NEG_INFINITY,
            None,
            "animation value",
            &state.span,
        )?;
    }
    let Some(options) = &state.animation else {
        return Ok(());
    };
    if let Some(easing) = options.easing.as_deref()
        && !ANIMATION_EASINGS.contains(&easing)
    {
        let function = extern_function(document, easing, ExternKind::Pure, &state.span)?;
        if function.params.len() != 1
            || function.params[0].1 != Type::F64
            || function.output != Type::F64
            || function.error.is_some()
        {
            return Err(Error::new(
                "E103",
                &state.span,
                format!("animation easing `{easing}` must be `pure {easing}(value:f64) -> f64`"),
            ));
        }
    }
    if let Some(from) = options.from
        && from.ty() != *inner
    {
        return Err(Error::new(
            "E103",
            &state.span,
            format!(
                "animation `from` must be a `{}` value, matching the animated type",
                inner.display()
            ),
        ));
    }
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

fn recomputation_unsafe_builtin_call<'a>(expr: &'a Expr, document: &Document) -> Option<&'a str> {
    match expr {
        Expr::Call { name, args } => {
            let extern_shadows_builtin = document.functions.iter().any(|function| {
                function.name == *name
                    && matches!(function.kind, ExternKind::Pure | ExternKind::Sync)
            });
            let builtin = crate::unqualified_name(name);
            let implicit_animation_clock = matches!(
                (builtin, args.len()),
                ("animation.animating" | "animation.remaining", 1)
                    | ("animation.interpolate" | "animation.project", 3)
            );
            (!extern_shadows_builtin
                && (matches!(
                    builtin,
                    "window_id.unique"
                        | "aborted"
                        | "debug.time_with"
                        | "image.upgrade"
                        | "encoded"
                        | "rgba"
                ) || implicit_animation_clock))
                .then_some(name.as_str())
                .or_else(|| {
                    args.iter()
                        .find_map(|argument| recomputation_unsafe_builtin_call(argument, document))
                })
        }
        Expr::List(values) => values
            .iter()
            .find_map(|value| recomputation_unsafe_builtin_call(value, document)),
        Expr::Unary { value, .. } => recomputation_unsafe_builtin_call(value, document),
        Expr::Binary { left, right, .. } => recomputation_unsafe_builtin_call(left, document)
            .or_else(|| recomputation_unsafe_builtin_call(right, document)),
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
    declarations: &crate::hir::DeclarationIndex,
    analyses: &mut facts::CheckedAnalyses,
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

    struct DerivedVisitor<'a> {
        document: &'a Document,
        states: &'a HashMap<String, Type>,
        names: &'a HashMap<String, usize>,
        declarations: &'a crate::hir::DeclarationIndex,
        analyses: &'a mut facts::CheckedAnalyses,
        marks: Vec<u8>,
        types: Vec<Option<Type>>,
    }

    impl DerivedVisitor<'_> {
        fn visit(&mut self, index: usize) -> Result<Type, Error> {
            if self.marks[index] == 1 {
                return Err(Error::new(
                    "E103",
                    &self.document.derived[index].span,
                    format!(
                        "derived value `{}` has a dependency cycle",
                        self.document.derived[index].name
                    ),
                ));
            }
            if let Some(ty) = &self.types[index] {
                return Ok(ty.clone());
            }
            self.marks[index] = 1;
            if let Some(function) =
                sync_extern_call(&self.document.derived[index].value, self.document)
            {
                let derived = &self.document.derived[index];
                return Err(Error::new(
                    "E103",
                    &derived.span,
                    format!(
                        "derived value `{}` cannot call sync extern `{function}`",
                        derived.name
                    ),
                )
                .hint("declare a deterministic, side-effect-free Rust function as `pure`"));
            }
            if let Some(function) = recomputation_unsafe_builtin_call(
                &self.document.derived[index].value,
                self.document,
            ) {
                let derived = &self.document.derived[index];
                return Err(Error::new(
                    "E103",
                    &derived.span,
                    format!(
                        "derived value `{}` cannot call recomputation-unsafe builtin `{function}`",
                        derived.name
                    ),
                )
                .hint("capture the runtime value in state from an initializer or handler, then derive from that state"));
            }
            let mut env = ScopedTypeEnv::new(self.states);
            let mut deps = Vec::new();
            dependencies(&self.document.derived[index].value, self.names, &mut deps);
            for dependency in deps {
                let ty = self.visit(dependency)?;
                env.insert(self.document.derived[dependency].name.clone(), ty);
            }
            let derived = &self.document.derived[index];
            let analysis =
                expr::analyze_expr_types(&derived.value, &env, self.document, &derived.span)?;
            let ty = analysis
                .type_of(&derived.value)
                .cloned()
                .ok_or_else(|| Error::new("E196", &derived.span, "missing checked derived type"))?;
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
            self.marks[index] = 2;
            self.types[index] = Some(ty.clone());
            self.analyses.insert(
                facts::CheckedValueRef::Derived(self.declarations.derived(index).id),
                analysis,
            )?;
            Ok(ty)
        }
    }

    let names = document
        .derived
        .iter()
        .enumerate()
        .map(|(index, derived)| (derived.name.clone(), index))
        .collect::<HashMap<_, _>>();
    let types = {
        let mut visitor = DerivedVisitor {
            document,
            states,
            names: &names,
            declarations,
            analyses,
            marks: vec![0; document.derived.len()],
            types: vec![None; document.derived.len()],
        };
        for index in 0..document.derived.len() {
            visitor.visit(index)?;
        }
        visitor.types
    };
    let mut env = HashMap::new();
    for (derived, ty) in document.derived.iter_mut().zip(types) {
        let ty = ty.expect("every derived value was visited");
        derived.ty = ty.clone();
        env.insert(derived.name.clone(), ty);
    }
    Ok(env)
}

const COMPONENT_CONTEXT_PREFIX: &str = "\0component:";
const COMPONENT_CONTEXT_INDEX: &str = "\0component-context";
const COMPONENT_OUTPUT_PREFIX: &str = "\0component-output:";

fn component_context_key(component: &str) -> String {
    format!("{COMPONENT_CONTEXT_PREFIX}{component}")
}

fn component_output_key(component: &str) -> String {
    format!("{COMPONENT_OUTPUT_PREFIX}{component}")
}

fn component_context(env: &dyn ExprTypeEnv) -> Option<&str> {
    match env.get_type(COMPONENT_CONTEXT_INDEX) {
        Some(Type::Named(component)) => Some(component),
        _ => None,
    }
}

fn component_output(env: &dyn ExprTypeEnv) -> Option<&Type> {
    env.type_with_prefix(COMPONENT_OUTPUT_PREFIX)
}

fn component_handler_key(component: &str, handler: &str) -> String {
    format!("{component}.{handler}")
}

fn preset_handler(preset: &Preset) -> Handler {
    Handler {
        name: format!("preset {}", preset.name),
        params: Vec::new(),
        statements: preset.statements.clone(),
        span: preset.span.clone(),
    }
}

fn component_handler_statement_supported(statement: &Statement) -> bool {
    match statement {
        Statement::Let { .. }
        | Statement::Assign { .. }
        | Statement::ReturnIf { .. }
        | Statement::WidgetOperation {
            operation:
                WidgetOperation::Focus { .. }
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
        | Statement::InvalidateLane { .. }
        | Statement::Run {
            kind: EffectKind::Future,
            ..
        }
        | Statement::Run {
            kind: EffectKind::Stream,
            mode: DeliveryMode::Replace,
            ..
        } => true,
        Statement::TaskGroup { statements, .. } => {
            statements.iter().all(component_handler_statement_supported)
        }
        _ => false,
    }
}

fn component_stream_every(statement: &Statement) -> Option<&Span> {
    match statement {
        Statement::Run {
            kind: EffectKind::Stream,
            mode: DeliveryMode::Every,
            span,
            ..
        } => Some(span),
        Statement::TaskGroup { statements, .. } => {
            statements.iter().find_map(component_stream_every)
        }
        Statement::Abortable { task, .. } => component_stream_every(task),
        _ => None,
    }
}

fn check_run_lanes(document: &Document) -> Result<(), Error> {
    let mut contracts = HashMap::new();
    for handler in &document.handlers {
        check_handler_run_lanes(&handler.statements, None, &mut contracts)?;
    }
    for preset in &document.presets {
        check_handler_run_lanes(&preset.statements, None, &mut contracts)?;
    }
    for component in &document.components {
        for handler in &component.handlers {
            check_handler_run_lanes(
                &handler.statements,
                Some(component.name.as_str()),
                &mut contracts,
            )?;
        }
    }
    for handler in &document.handlers {
        check_handler_lane_invalidations(&handler.statements, None, &contracts)?;
    }
    for preset in &document.presets {
        check_handler_lane_invalidations(&preset.statements, None, &contracts)?;
    }
    for component in &document.components {
        for handler in &component.handlers {
            check_handler_lane_invalidations(
                &handler.statements,
                Some(component.name.as_str()),
                &contracts,
            )?;
        }
    }
    Ok(())
}

fn check_handler_run_lanes<'a>(
    statements: &'a [Statement],
    owner: Option<&'a str>,
    contracts: &mut HashMap<(Option<&'a str>, &'a str), (EffectKind, DeliveryMode, &'a Span)>,
) -> Result<(), Error> {
    fn visit<'a>(
        statements: &'a [Statement],
        owner: Option<&'a str>,
        contracts: &mut HashMap<(Option<&'a str>, &'a str), (EffectKind, DeliveryMode, &'a Span)>,
        seen: &mut HashSet<&'a str>,
    ) -> Result<(), Error> {
        for statement in statements {
            match statement {
                Statement::Run {
                    kind: EffectKind::Stream,
                    mode: DeliveryMode::Latest,
                    span,
                    ..
                } => {
                    return Err(Error::new(
                        "E140",
                        span,
                        "`stream latest` is not supported",
                    )
                    .hint(
                        "use `stream replace lane=name ...` to abort and suppress the prior stream",
                    ));
                }
                Statement::Run {
                    kind,
                    mode,
                    lane: Some(lane),
                    span,
                    ..
                } => {
                    if !seen.insert(lane) {
                        return Err(Error::new(
                            "E140",
                            span,
                            format!(
                                "delivery lane `{lane}` cannot be started more than once in the same handler"
                            ),
                        ));
                    }
                    let key = (owner, lane.as_str());
                    if let Some((expected_kind, expected_mode, first)) = contracts.get(&key) {
                        if expected_kind != kind || expected_mode != mode {
                            return Err(Error::new(
                                "E140",
                                span,
                                format!(
                                    "delivery lane `{lane}` uses both `{}` and `{}` for the same owner",
                                    delivery_statement_name(*expected_kind, *expected_mode),
                                    delivery_statement_name(*kind, *mode),
                                ),
                            )
                            .hint(format!(
                                "use `{}` for this lane; it was first declared on line {}",
                                delivery_statement_name(*expected_kind, *expected_mode),
                                first.line,
                            )));
                        }
                    } else {
                        contracts.insert(key, (*kind, *mode, span));
                    }
                }
                Statement::TaskGroup { statements, .. } => {
                    visit(statements, owner, contracts, seen)?;
                }
                Statement::Abortable { task, .. } => {
                    visit(
                        ::std::slice::from_ref(task.as_ref()),
                        owner,
                        contracts,
                        seen,
                    )?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    visit(statements, owner, contracts, &mut HashSet::new())
}

fn check_handler_lane_invalidations<'a>(
    statements: &'a [Statement],
    owner: Option<&'a str>,
    contracts: &HashMap<(Option<&'a str>, &'a str), (EffectKind, DeliveryMode, &'a Span)>,
) -> Result<(), Error> {
    for statement in statements {
        if let Statement::InvalidateLane { lane, span } = statement
            && !contracts.contains_key(&(owner, lane.as_str()))
        {
            return Err(Error::new(
                "E140",
                span,
                format!("delivery lane `{lane}` is not declared for this state owner"),
            )
            .hint(
                "declare it with a named `run latest`, `run replace`, or `stream replace` lane for the same state owner",
            ));
        }
    }
    Ok(())
}

fn delivery_statement_name(kind: EffectKind, mode: DeliveryMode) -> &'static str {
    match (kind, mode) {
        (EffectKind::Future, DeliveryMode::Every) => "run every",
        (EffectKind::Future, DeliveryMode::Latest) => "run latest",
        (EffectKind::Future, DeliveryMode::Replace) => "run replace",
        (EffectKind::Stream, DeliveryMode::Every) => "stream every",
        (EffectKind::Stream, DeliveryMode::Latest) => "stream latest",
        (EffectKind::Stream, DeliveryMode::Replace) => "stream replace",
        (EffectKind::Task, _) => "task",
    }
}

mod application;
mod canvas;
mod cycles;
mod declarations;
mod expr;
mod facts;
mod handler;
mod lazy_delivery;
mod lifecycle;
mod options;
mod reachability;
mod smells;
mod state;
mod style;
mod subscription;
mod testing;
mod usage;
mod view;
mod widgets;

use application::*;
use canvas::*;
use cycles::*;
use declarations::*;
use handler::*;
use lazy_delivery::*;
use lifecycle::*;
use options::*;
use reachability::*;
use smells::*;
use state::{check_qr_payload, check_theme, pane_grid_span, repeated_pane_grid_span};
pub(crate) use state::{controlled_editor_bindings, controlled_state_bindings};
use style::*;
pub(crate) use subscription::native_subscription_payloads;
use subscription::*;
use testing::*;
use usage::*;
pub(crate) use view::lazy_hashable;
use view::*;
use widgets::*;

pub(crate) use expr::fields::field_type;
pub(crate) use expr::signature::{BuiltinArgumentContext, ContextualBuiltin, unify_type_evidence};
pub(crate) use expr::{ExprTypeEnv, ScopedTypeEnv, SyncTypeEnv, canonical_builtin_type, expr_type};
use expr::{check_length_value, contains_ui_enum};
#[cfg(test)]
pub(crate) use facts::CheckedFactMetrics;
pub(crate) use facts::{
    CheckedAppSettings, CheckedBinaryOperator, CheckedBooleanControl, CheckedCallArgument,
    CheckedCallTarget, CheckedCanvas, CheckedCanvasRouteArg, CheckedCanvasRouteTarget,
    CheckedComboBox, CheckedComponentArgumentSource, CheckedComponentEventDelivery,
    CheckedEffectTarget, CheckedExprId, CheckedExprKind, CheckedExprOwner, CheckedExprUse,
    CheckedExprUseId, CheckedExternViewAdapter, CheckedFacts, CheckedInitializerCoercion,
    CheckedInput, CheckedInteraction, CheckedInteractionKind, CheckedInteractionRoute,
    CheckedKeyedLength, CheckedLayout, CheckedLocalId, CheckedLocalOwner, CheckedMarkdown,
    CheckedMatchArm, CheckedMatchPattern, CheckedMedia, CheckedPaneAxis, CheckedPaneBackground,
    CheckedPaneConfiguration, CheckedPaneCustomStyle, CheckedPaneGrid, CheckedPaneGridStyle,
    CheckedPaneLength, CheckedPanePadding, CheckedPaneRadius, CheckedPaneStyleSite,
    CheckedPaneSurface, CheckedPaneTemplate, CheckedPaneTitle, CheckedPaneView, CheckedPathRoot,
    CheckedPickList, CheckedProjectionKind, CheckedResponsiveLength, CheckedRouteArgKind,
    CheckedStatement, CheckedSubscription, CheckedSubscriptionExprRole, CheckedSubscriptionSource,
    CheckedTableLength, CheckedText, CheckedTooltip, CheckedUnaryOperator, CheckedValueRef,
    CheckedView, CheckedViewExprRole, CheckedViewFlow, CheckedViewLocalRole, CheckedViewScope,
};
pub(crate) use handler::task_flow_type;

pub(in crate::check) type WidgetIdPath = Vec<(String, Option<Type>)>;

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
