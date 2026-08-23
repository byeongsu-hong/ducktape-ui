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
    component_controlled_editors: Vec<crate::CheckedComponentControlledEditor>,
    test_component_reads: Vec<crate::CheckedTestComponentRead>,
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
    warnings.extend(performance_warnings(
        &document,
        &reachable,
        &declarations,
        &facts,
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
        checked.component_controlled_editors,
        checked.test_component_reads,
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
            } else {
                let text_initial = state.ty == Type::Editor && actual == Type::Str;
                if actual != Type::Unknown && !text_initial && !compatible(&state.ty, &actual) {
                    return Err(type_error(&state.span, &state.ty, &actual));
                }
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
    handler_emit_targets(document)?;
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
            if let Some(span) = handler
                .statements
                .iter()
                .find_map(|statement| match statement {
                    Statement::Slice { span, .. } => Some(span),
                    _ => None,
                })
            {
                return Err(Error::new(
                    "E140",
                    span,
                    "a slice hands an APP handler's payload down; a component handler already has its own state",
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
    for binding in &controlled_editor_contracts.app {
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
                // A synthesized `boot` param carries its prop's concrete
                // type; no route targets boot, so there is nothing to infer
                // from — a pre-typed param is authoritative.
                if param.ty != Type::Unknown {
                    continue;
                }
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
                    None,
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
                None,
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
                            Some(&component.events),
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
    let test_component_reads = check_tests(document, &view_states, declarations)?;
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
    let editor_action = |action: Option<String>| {
        action
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
            .transpose()
    };
    let controlled_editors = controlled_editor_contracts
        .app
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
            Ok(crate::CheckedControlledEditor {
                state: declarations.app_state(index).id,
                action: editor_action(binding.action)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let component_controlled_editors = controlled_editor_contracts
        .component
        .into_iter()
        .map(|binding| {
            let (component_index, component) = document
                .components
                .iter()
                .enumerate()
                .find(|(_, component)| component.name == binding.component)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        document.view.span(),
                        "checked editor binding is not a component",
                    )
                })?;
            let state_index = component
                .states
                .iter()
                .position(|state| state.name == binding.state)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        document.view.span(),
                        "checked editor binding is not a component state",
                    )
                })?;
            let component_id = declarations.component(component_index).id;
            Ok(crate::CheckedComponentControlledEditor {
                state: declarations.component_state(component_id, state_index).id,
                action: editor_action(binding.action)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(CheckOutput {
        analyses: initializer_analyses,
        controlled_inputs,
        controlled_editors,
        component_controlled_editors,
        test_component_reads,
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

/// Resolves every handler-emitted component event to the ONE app handler it
/// delivers to. A component handler firing `emit(event, ...)` cannot see any
/// call site's environment, so the delivery must be recoverable statically:
/// every call site routes the event either straight to one app handler with
/// only `_` payloads, or chains it upward with `emit(outer, _...)` from
/// inside another component, where the same rule applies to the outer event.
pub(crate) fn handler_emit_targets(
    document: &Document,
) -> Result<HashMap<(String, String), String>, Error> {
    fn collect_emits(statements: &[Statement], output: &mut Vec<(String, Span)>) {
        for statement in statements {
            match statement {
                Statement::Emit { event, span, .. } => output.push((event.clone(), span.clone())),
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        collect_emits(&arm.statements, output);
                    }
                }
                Statement::TaskGroup { statements, .. } => collect_emits(statements, output),
                Statement::Abortable { task, .. } => {
                    collect_emits(std::slice::from_ref(task.as_ref()), output);
                }
                _ => {}
            }
        }
    }

    fn walk_calls<'a>(
        node: &'a ViewNode,
        scope: Option<&'a str>,
        output: &mut Vec<(Option<&'a str>, &'a ViewNode)>,
    ) {
        match node {
            ViewNode::Component { slots, .. } => {
                output.push((scope, node));
                // Slot content is authored at the call site, so nested calls
                // inside it belong to the CURRENT scope, not the callee's.
                for slot in slots {
                    walk_calls(&slot.content, scope, output);
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    walk_calls(child, scope, output);
                }
            }
            ViewNode::Match { arms, .. } => {
                for arm in arms {
                    for child in &arm.children {
                        walk_calls(child, scope, output);
                    }
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            }
            | ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => walk_calls(content, scope, output),
            ViewNode::Tooltip { content, tip, .. } => {
                walk_calls(content, scope, output);
                walk_calls(tip, scope, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                walk_calls(content, scope, output);
                walk_calls(layer, scope, output);
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    walk_calls(child, scope, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    walk_calls(&column.header, scope, output);
                    walk_calls(&column.cell, scope, output);
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Size { content, .. } => walk_calls(content, scope, output),
            },
            _ => {}
        }
    }

    fn resolve(
        document: &Document,
        sites: &[(Option<&str>, &ViewNode)],
        component: &str,
        event: &str,
        span: &Span,
        path: &mut Vec<(String, String)>,
    ) -> Result<String, Error> {
        let key = (component.to_owned(), event.to_owned());
        if path.contains(&key) {
            return Err(Error::new(
                "E140",
                span,
                format!("handler-emitted event `{component}.{event}` chains into a cycle"),
            ));
        }
        path.push(key);
        let payloads = document
            .components
            .iter()
            .find(|entry| entry.name == component)
            .and_then(|entry| {
                entry
                    .events
                    .iter()
                    .find(|declared| declared.name == event)
                    .map(|declared| declared.payloads.len())
            })
            .ok_or_else(|| {
                Error::new(
                    "E140",
                    span,
                    format!("component `{component}` does not declare event `{event}`"),
                )
            })?;
        let mut target: Option<String> = None;
        for (scope, node) in sites {
            let ViewNode::Component { name, events, .. } = node else {
                continue;
            };
            if name != component {
                continue;
            }
            let Some(route) = events
                .iter()
                .find(|entry| entry.name == event)
                .and_then(|entry| entry.route.as_ref())
            else {
                continue;
            };
            let resolved = if route.handler == "emit" {
                let Some(outer_scope) = scope else {
                    return Err(Error::new(
                        "E140",
                        &route.span,
                        format!(
                            "handler-emitted event `{component}.{event}` chains with `emit(...)` outside any component"
                        ),
                    ));
                };
                let mut args = route.args.iter();
                let Some(RouteArg::Expr(Expr::Path(segments))) = args.next() else {
                    return Err(Error::new(
                        "E140",
                        &route.span,
                        format!(
                            "handler-emitted event `{component}.{event}` chains to an unnamed event"
                        ),
                    ));
                };
                let [outer] = segments.as_slice() else {
                    return Err(Error::new(
                        "E140",
                        &route.span,
                        format!(
                            "handler-emitted event `{component}.{event}` chains to an unnamed event"
                        ),
                    ));
                };
                if args.len() != payloads || args.any(|arg| !matches!(arg, RouteArg::Payload)) {
                    return Err(Error::new(
                        "E140",
                        &route.span,
                        format!(
                            "handler-emitted event `{component}.{event}` must chain every payload with `_` — the emitting handler cannot see this call site's values"
                        ),
                    ));
                }
                resolve(document, sites, outer_scope, outer, span, path)?
            } else {
                if scope.is_some() {
                    return Err(Error::new(
                        "E140",
                        &route.span,
                        format!(
                            "handler-emitted event `{component}.{event}` routes to a caller-local handler; chain it upward with `emit(outer_event, _)` instead — the emitting handler cannot name the caller's instance"
                        ),
                    ));
                }
                if route.args.len() != payloads
                    || route
                        .args
                        .iter()
                        .any(|arg| !matches!(arg, RouteArg::Payload))
                {
                    return Err(Error::new(
                        "E140",
                        &route.span,
                        format!(
                            "handler-emitted event `{component}.{event}` must route every payload with `_` — the emitting handler cannot see this call site's values"
                        ),
                    ));
                }
                route.handler.clone()
            };
            match &target {
                Some(existing) if *existing != resolved => {
                    return Err(Error::new(
                        "E140",
                        &route.span,
                        format!(
                            "handler-emitted event `{component}.{event}` must deliver to one app handler at every call site (`{existing}` vs `{resolved}`)"
                        ),
                    ));
                }
                _ => target = Some(resolved),
            }
        }
        path.pop();
        target.ok_or_else(|| {
            Error::new(
                "E140",
                span,
                format!(
                    "handler-emitted event `{component}.{event}` has no routed call site to deliver through"
                ),
            )
        })
    }

    let mut sites = Vec::new();
    walk_calls(&document.view, None, &mut sites);
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        walk_calls(mount, None, &mut sites);
    }
    for component in &document.components {
        walk_calls(&component.root, Some(&component.name), &mut sites);
    }
    let mut targets = HashMap::new();
    for component in &document.components {
        let mut emits = Vec::new();
        for handler in &component.handlers {
            collect_emits(&handler.statements, &mut emits);
        }
        for (event, span) in emits {
            let key = (component.name.clone(), event.clone());
            if targets.contains_key(&key) {
                continue;
            }
            let target = resolve(
                document,
                &sites,
                &component.name,
                &event,
                &span,
                &mut Vec::new(),
            )?;
            targets.insert(key, target);
        }
    }
    Ok(targets)
}

fn component_handler_statement_supported(statement: &Statement) -> bool {
    match statement {
        Statement::Let { .. }
        | Statement::Assign { .. }
        | Statement::ReturnIf { .. }
        | Statement::Emit { .. }
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
                | WidgetOperation::ScrollToKey { .. }
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
        Statement::Match { arms, .. } => arms
            .iter()
            .flat_map(|arm| &arm.statements)
            .all(component_handler_statement_supported),
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
        Statement::Match { arms, .. } => arms
            .iter()
            .flat_map(|arm| &arm.statements)
            .find_map(component_stream_every),
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
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        let mut branch_seen = seen.clone();
                        visit(&arm.statements, owner, contracts, &mut branch_seen)?;
                    }
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
        if let Statement::Match { arms, .. } = statement {
            for arm in arms {
                check_handler_lane_invalidations(&arm.statements, owner, contracts)?;
            }
            continue;
        }
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
pub(crate) mod declarations;
mod expr;
mod facts;
mod handler;
mod lazy_delivery;
mod lifecycle;
mod options;
mod perf;
mod reachability;
mod smells;
mod state;
mod style;
mod subscription;
mod testing;

pub(crate) use style::compatible;
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
use perf::*;
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
    CheckedLayout, CheckedLength, CheckedLocalId, CheckedLocalOwner, CheckedMarkdown,
    CheckedMatchArm, CheckedMatchPattern, CheckedMedia, CheckedPadding, CheckedPaneAxis,
    CheckedPaneBackground, CheckedPaneConfiguration, CheckedPaneCustomStyle, CheckedPaneGrid,
    CheckedPaneGridStyle, CheckedPaneRadius, CheckedPaneStyleSite, CheckedPaneSurface,
    CheckedPaneTemplate, CheckedPaneTitle, CheckedPaneView, CheckedPathRoot, CheckedPickList,
    CheckedProjectionKind, CheckedRichChild, CheckedRouteArgKind, CheckedStatement,
    CheckedSubscription, CheckedSubscriptionExprRole, CheckedSubscriptionSource, CheckedText,
    CheckedTooltip, CheckedUnaryOperator, CheckedValueRef, CheckedView, CheckedViewExprRole,
    CheckedViewFlow, CheckedViewLocalRole, CheckedViewScope,
};
pub(crate) use handler::task_flow_type;

pub(in crate::check) type WidgetIdPath = Vec<(String, Option<Type>)>;

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
