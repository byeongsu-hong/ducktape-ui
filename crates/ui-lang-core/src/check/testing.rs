use super::*;

const TEST_ERROR: &str = "E194";

pub(in crate::check) fn infer_tests(
    document: &Document,
    states: &HashMap<String, Type>,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    let mut pane_names = static_pane_grids(&document.view, states, document)?
        .into_keys()
        .collect::<HashSet<_>>();
    for test in &document.tests {
        if let Some(mount) = &test.mount {
            if let Some(span) = repeated_pane_grid_span(mount) {
                return Err(Error::new(
                    "E187",
                    span,
                    "panes cannot be repeated because each static ID owns one persistent layout state",
                ));
            }
            for name in static_pane_grids(mount, states, document)?.into_keys() {
                if !pane_names.insert(name.clone()) {
                    return Err(Error::new(
                        "E187",
                        &test.span,
                        format!("duplicate persistent panes `#{name}` across app and test mounts"),
                    )
                    .hint("give every persistent pane grid in the Ice source graph a unique #id"));
                }
            }
            let mut ids = HashSet::new();
            infer_view(mount, states, document, signatures, &mut ids)?;
        }

        let env = test_env(test, states);
        for step in &test.steps {
            if let TestStepKind::Dispatch { handler, args } = &step.kind {
                infer_route(
                    &Route {
                        handler: handler.clone(),
                        args: args.iter().cloned().map(RouteArg::Expr).collect(),
                        span: step.span.clone(),
                    },
                    None,
                    &env,
                    document,
                    signatures,
                )?;
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn check_tests(
    document: &Document,
    states: &HashMap<String, Type>,
) -> Result<(), Error> {
    for test in &document.tests {
        if let Some(renderer) = &document.settings.renderer
            && let Some(span) = test_paint_span(test)
        {
            return Err(Error::new(
                TEST_ERROR,
                span,
                format!("paint assertions do not support custom renderer `{renderer}`"),
            )
            .hint(
                "assert layout and interactions in Ice, or inspect the custom renderer from Rust",
            ));
        }
        if let Some(preset) = &test.preset
            && !document.presets.iter().any(|item| item.name == *preset)
        {
            return Err(Error::new(
                TEST_ERROR,
                &test.span,
                format!("unknown test preset `{preset}`"),
            )
            .hint(format!("declare `preset {preset}` before using it")));
        }

        let root = test.mount.as_ref().unwrap_or(&document.view);
        let ids = test_widget_ids(root, states, document)?;
        let mut target_env = states.clone();
        for target in &test.targets {
            check_test_widget_target(&target.target, &target_env, document, &ids, &target.span)?;
            target_env.insert(target.name.clone(), Type::TestTarget);
        }

        let env = test_env(test, states);
        for step in &test.steps {
            check_test_step(step, &env, test, document, &ids)?;
        }
    }
    Ok(())
}

fn test_paint_span(test: &TestDecl) -> Option<&Span> {
    test.steps
        .iter()
        .find_map(|step| {
            let uses_paint = match &step.kind {
                TestStepKind::Type(value) => expr_uses_test_paint(value, test),
                TestStepKind::Resize(width, height) => {
                    expr_uses_test_paint(width, test) || expr_uses_test_paint(height, test)
                }
                TestStepKind::Dispatch { args, .. } => {
                    args.iter().any(|value| expr_uses_test_paint(value, test))
                }
                TestStepKind::Expect(expectation) => match expectation {
                    TestExpectation::Expr(value) => expr_uses_test_paint(value, test),
                    TestExpectation::Approx { left, right } => {
                        expr_uses_test_paint(left, test) || expr_uses_test_paint(right, test)
                    }
                    TestExpectation::Text { value, within, .. } => {
                        expr_uses_test_paint(value, test)
                            || within
                                .as_ref()
                                .is_some_and(|target| target_ref_uses_test_paint(target, test))
                    }
                    TestExpectation::Exists(target) | TestExpectation::Missing(target) => {
                        target_ref_uses_test_paint(target, test)
                    }
                },
                TestStepKind::Click(target)
                | TestStepKind::Hover(target)
                | TestStepKind::Press(target) => target_ref_uses_test_paint(target, test),
                TestStepKind::Release | TestStepKind::Key(_) => false,
            };
            uses_paint.then_some(&step.span)
        })
        .or_else(|| {
            test.targets.iter().find_map(|target| {
                target_uses_test_paint(&target.target, test, &mut HashSet::new())
                    .then_some(&target.span)
            })
        })
}

fn target_ref_uses_test_paint(target: &TestTargetRef, test: &TestDecl) -> bool {
    let mut visited = HashSet::new();
    match target {
        TestTargetRef::Alias(name) => alias_uses_test_paint(name, test, &mut visited),
        TestTargetRef::Id(target) => target_uses_test_paint(target, test, &mut visited),
    }
}

fn target_uses_test_paint(
    target: &WidgetTarget,
    test: &TestDecl,
    visited: &mut HashSet<String>,
) -> bool {
    target.segments.iter().any(|segment| {
        segment
            .key
            .as_ref()
            .is_some_and(|key| expr_uses_test_paint_inner(key, test, visited))
    })
}

fn alias_uses_test_paint(name: &str, test: &TestDecl, visited: &mut HashSet<String>) -> bool {
    let Some(target) = test.targets.iter().find(|target| target.name == name) else {
        return false;
    };
    visited.insert(name.to_owned()) && target_uses_test_paint(&target.target, test, visited)
}

fn expr_uses_test_paint(expr: &Expr, test: &TestDecl) -> bool {
    expr_uses_test_paint_inner(expr, test, &mut HashSet::new())
}

fn expr_uses_test_paint_inner(expr: &Expr, test: &TestDecl, visited: &mut HashSet<String>) -> bool {
    match expr {
        Expr::Path(path) => {
            let Some(name) = path.first() else {
                return false;
            };
            test.targets.iter().any(|target| target.name == *name)
                && (path.get(1).is_some_and(|field| {
                    matches!(
                        field.as_str(),
                        "background"
                            | "border"
                            | "shadow"
                            | "text_color"
                            | "text_size"
                            | "font"
                            | "line_height"
                    )
                }) || alias_uses_test_paint(name, test, visited))
        }
        Expr::List(values) | Expr::Call { args: values, .. } => values
            .iter()
            .any(|value| expr_uses_test_paint_inner(value, test, visited)),
        Expr::Unary { value, .. } => expr_uses_test_paint_inner(value, test, visited),
        Expr::Binary { left, right, .. } => {
            expr_uses_test_paint_inner(left, test, visited)
                || expr_uses_test_paint_inner(right, test, visited)
        }
        Expr::Bool(_)
        | Expr::I64(_)
        | Expr::F64(_)
        | Expr::Str(_)
        | Expr::Bytes(_)
        | Expr::EmptyList
        | Expr::None => false,
    }
}

fn test_env(test: &TestDecl, states: &HashMap<String, Type>) -> HashMap<String, Type> {
    let mut env = states.clone();
    env.extend(
        test.targets
            .iter()
            .map(|target| (target.name.clone(), Type::TestTarget)),
    );
    env
}

fn check_test_step(
    step: &TestStep,
    env: &HashMap<String, Type>,
    test: &TestDecl,
    document: &Document,
    ids: &TestWidgetIds,
) -> Result<(), Error> {
    match &step.kind {
        TestStepKind::Click(target) | TestStepKind::Hover(target) | TestStepKind::Press(target) => {
            check_test_target_ref(target, env, test, document, ids, &step.span)?;
        }
        TestStepKind::Release | TestStepKind::Key(_) => {}
        TestStepKind::Type(value) => {
            require_type(
                &expr_type(value, env, document, &step.span)?,
                &Type::Str,
                &step.span,
            )?;
        }
        TestStepKind::Resize(width, height) => {
            for (value, label) in [
                (width, "test viewport width"),
                (height, "test viewport height"),
            ] {
                require_test_number(value, env, document, &step.span, label, true)?;
            }
        }
        TestStepKind::Dispatch { handler, args } => {
            if handler == "mount" {
                return Err(Error::new(
                    TEST_ERROR,
                    &step.span,
                    "`mount` is initialization-only and cannot be dispatched",
                ));
            }
            let handler = document
                .handlers
                .iter()
                .find(|item| item.name == *handler)
                .ok_or_else(|| {
                    Error::new(
                        TEST_ERROR,
                        &step.span,
                        format!("unknown handler `{handler}`"),
                    )
                })?;
            if args.len() != handler.params.len() {
                return Err(Error::new(
                    TEST_ERROR,
                    &step.span,
                    format!(
                        "handler `{}` expects {} arguments, got {}",
                        handler.name,
                        handler.params.len(),
                        args.len()
                    ),
                ));
            }
            for (arg, param) in args.iter().zip(&handler.params) {
                require_type(
                    &expr_type(arg, env, document, &step.span)?,
                    &param.ty,
                    &step.span,
                )?;
            }
        }
        TestStepKind::Expect(expectation) => match expectation {
            TestExpectation::Expr(value) => {
                require_type(
                    &expr_type(value, env, document, &step.span)?,
                    &Type::Bool,
                    &step.span,
                )?;
            }
            TestExpectation::Approx { left, right } => {
                require_test_number(left, env, document, &step.span, "approximate value", false)?;
                require_test_number(right, env, document, &step.span, "approximate value", false)?;
            }
            TestExpectation::Exists(target) | TestExpectation::Missing(target) => {
                check_test_target_ref(target, env, test, document, ids, &step.span)?;
            }
            TestExpectation::Text { value, within, .. } => {
                require_type(
                    &expr_type(value, env, document, &step.span)?,
                    &Type::Str,
                    &step.span,
                )?;
                if let Some(target) = within {
                    check_test_target_ref(target, env, test, document, ids, &step.span)?;
                }
            }
        },
    }
    Ok(())
}

fn require_test_number(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    label: &str,
    positive: bool,
) -> Result<(), Error> {
    let ty = expr_type(value, env, document, span)?;
    if !matches!(ty, Type::I64 | Type::F64) {
        return Err(Error::new(
            TEST_ERROR,
            span,
            format!("{label} must be numeric, got `{}`", ty.display()),
        ));
    }
    let literal = match value {
        Expr::I64(value) => Some(*value as f64),
        Expr::F64(value) => Some(*value),
        _ => None,
    };
    if let Some(value) = literal
        && (!value.is_finite() || value.abs() > f32::MAX as f64 || positive && value <= 0.0)
    {
        return Err(Error::new(
            TEST_ERROR,
            span,
            format!(
                "{label} must be {}in the f32 range",
                if positive { "positive and " } else { "" }
            ),
        ));
    }
    Ok(())
}

fn check_test_target_ref(
    target: &TestTargetRef,
    env: &HashMap<String, Type>,
    test: &TestDecl,
    document: &Document,
    ids: &TestWidgetIds,
    span: &Span,
) -> Result<(), Error> {
    match target {
        TestTargetRef::Alias(name) => {
            if test.targets.iter().any(|target| target.name == *name) {
                Ok(())
            } else {
                Err(Error::new(
                    TEST_ERROR,
                    span,
                    format!("unknown test target alias `{name}`"),
                ))
            }
        }
        TestTargetRef::Id(target) => check_test_widget_target(target, env, document, ids, span),
    }
}

fn check_test_widget_target(
    target: &WidgetTarget,
    env: &HashMap<String, Type>,
    document: &Document,
    ids: &TestWidgetIds,
    span: &Span,
) -> Result<(), Error> {
    match check_widget_target(target, env, document, &ids.targets, span) {
        Ok(()) => Ok(()),
        Err(mut failure) => {
            failure.code = TEST_ERROR;
            if failure.message.starts_with("unknown app widget target") {
                let actual = typed_target_path(target, env, document, span)?;
                let label = target_label(target);
                if ids
                    .component_scopes
                    .iter()
                    .any(|scope| widget_paths_match(scope, &actual))
                {
                    return Err(Error::new(
                        TEST_ERROR,
                        span,
                        format!("{label} identifies a component scope, not a rendered widget"),
                    )
                    .hint("target an explicit #id rendered inside the component"));
                }
                failure.message = format!("unknown rendered widget target `{label}`");
                failure.hint = Some(
                    "use the full component, layout, keyed, table, or pane identity path from the tested view"
                        .into(),
                );
            }
            Err(failure)
        }
    }
}

fn typed_target_path(
    target: &WidgetTarget,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<WidgetIdPath, Error> {
    target
        .segments
        .iter()
        .map(|segment| {
            Ok((
                segment.name.clone(),
                segment
                    .key
                    .as_ref()
                    .map(|key| expr_type(key, env, document, span))
                    .transpose()?,
            ))
        })
        .collect()
}

fn widget_paths_match(expected: &WidgetIdPath, actual: &WidgetIdPath) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|((expected_name, expected_key), (name, key))| {
                expected_name == name
                    && match (expected_key, key) {
                        (None, None) => true,
                        (Some(expected), Some(actual)) => compatible(expected, actual),
                        _ => false,
                    }
            })
}

fn target_label(target: &WidgetTarget) -> String {
    format!(
        "#{}",
        target
            .segments
            .iter()
            .map(|segment| if segment.key.is_some() {
                format!("{}(key)", segment.name)
            } else {
                segment.name.clone()
            })
            .collect::<Vec<_>>()
            .join("/")
    )
}
