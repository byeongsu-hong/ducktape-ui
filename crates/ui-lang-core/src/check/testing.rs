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
    declarations: &crate::hir::DeclarationIndex,
) -> Result<Vec<crate::CheckedTestComponentRead>, Error> {
    let mut reads = Vec::new();
    for (test_index, test) in document.tests.iter().enumerate() {
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
        // An alias may name a component scope — `expect component` reads the
        // instance behind it — but every rendered-widget use of that alias is
        // rejected where it happens, the way the bare path is.
        let mut scope_aliases = HashSet::new();
        for target in &test.targets {
            // A path that fails to type is left to the widget check, which
            // already words that failure.
            if let Ok(Some(_)) =
                component_scope_name(&target.target, &target_env, document, &ids, &target.span)
            {
                scope_aliases.insert(target.name.as_str());
            } else {
                check_test_widget_target(
                    &target.target,
                    &target_env,
                    document,
                    &ids,
                    &target.span,
                )?;
            }
            target_env.insert(target.name.clone(), Type::TestTarget);
        }

        let env = test_env(test, states);
        for (step_index, step) in test.steps.iter().enumerate() {
            if let Some(alias) = crate::ast::test_step_expression_roots(step)
                .into_iter()
                .find_map(|expr| expr_scope_alias(expr, &scope_aliases))
            {
                return Err(component_scope_error(&format!("`{alias}`"), &step.span));
            }
            if let Some((component, state)) = check_test_step(step, &env, test, document, &ids)? {
                let component = declarations.component(component).id;
                reads.push(crate::CheckedTestComponentRead {
                    step: crate::hir::TestStepId {
                        test: declarations.test(test_index).declaration.id,
                        index: step_index as u32,
                    },
                    component,
                    state: declarations.component_state(component, state).id,
                });
            }
        }
    }
    Ok(reads)
}

/// The alias of a component scope that `expr` reads a field of, if any:
/// a scope has no bounds or paint, so `alias.width` is as wrong as `click alias`.
fn expr_scope_alias<'a>(expr: &Expr, scope_aliases: &HashSet<&'a str>) -> Option<&'a str> {
    match expr {
        Expr::Path(path) => path
            .first()
            .and_then(|name| scope_aliases.get(name.as_str()).copied()),
        Expr::List(values) | Expr::Call { args: values, .. } => values
            .iter()
            .find_map(|value| expr_scope_alias(value, scope_aliases)),
        Expr::Unary { value, .. } => expr_scope_alias(value, scope_aliases),
        Expr::Binary { left, right, .. } => {
            expr_scope_alias(left, scope_aliases).or_else(|| expr_scope_alias(right, scope_aliases))
        }
        _ => None,
    }
}

fn component_scope_error(label: &str, span: &Span) -> Error {
    Error::new(
        TEST_ERROR,
        span,
        format!("{label} identifies a component scope, not a rendered widget"),
    )
    .hint(
        "target an explicit #id rendered inside the component, or read its state with `expect component`",
    )
}

fn test_paint_span(test: &TestDecl) -> Option<&Span> {
    test.steps
        .iter()
        .find_map(|step| {
            let uses_paint = match &step.kind {
                TestStepKind::Type(value)
                | TestStepKind::Replace(value)
                | TestStepKind::Cursor(value)
                | TestStepKind::Rescale(value)
                | TestStepKind::FileHover(value)
                | TestStepKind::FileDrop(value) => expr_uses_test_paint(value, test),
                TestStepKind::Select(width, height)
                | TestStepKind::WindowMove(width, height)
                | TestStepKind::Resize(width, height) => {
                    expr_uses_test_paint(width, test) || expr_uses_test_paint(height, test)
                }
                TestStepKind::ClickAt { x, y, .. } | TestStepKind::Wheel { x, y, .. } => {
                    expr_uses_test_paint(x, test) || expr_uses_test_paint(y, test)
                }
                TestStepKind::Scroll { target, x, y, .. } => {
                    target_ref_uses_test_paint(target, test)
                        || expr_uses_test_paint(x, test)
                        || expr_uses_test_paint(y, test)
                }
                TestStepKind::Snap { target, x, y } => {
                    target_ref_uses_test_paint(target, test)
                        || expr_uses_test_paint(x, test)
                        || expr_uses_test_paint(y, test)
                }
                TestStepKind::Move(TestPointerPosition::Point(x, y)) => {
                    expr_uses_test_paint(x, test) || expr_uses_test_paint(y, test)
                }
                TestStepKind::Repeat { count, .. } => expr_uses_test_paint(count, test),
                TestStepKind::Touch { id, x, y, .. } => {
                    expr_uses_test_paint(id, test)
                        || expr_uses_test_paint(x, test)
                        || expr_uses_test_paint(y, test)
                }
                TestStepKind::Composition(TestComposition::Update { value, selection }) => {
                    expr_uses_test_paint(value, test)
                        || selection.as_ref().is_some_and(|(start, end)| {
                            expr_uses_test_paint(start, test) || expr_uses_test_paint(end, test)
                        })
                }
                TestStepKind::Composition(TestComposition::Commit(value)) => {
                    expr_uses_test_paint(value, test)
                }
                TestStepKind::Dispatch { args, .. } => {
                    args.iter().any(|value| expr_uses_test_paint(value, test))
                }
                TestStepKind::TrayChoose(value) => expr_uses_test_paint(value, test),
                TestStepKind::Expect(expectation) => match expectation {
                    TestExpectation::Expr(value) => expr_uses_test_paint(value, test),
                    TestExpectation::Approx { left, right } => {
                        expr_uses_test_paint(left, test) || expr_uses_test_paint(right, test)
                    }
                    TestExpectation::Text { .. } | TestExpectation::Tray { .. } => true,
                    TestExpectation::Exists(target) | TestExpectation::Missing(target) => {
                        target_ref_uses_test_paint(target, test)
                    }
                    TestExpectation::ComponentState { target, value, .. } => {
                        target_ref_uses_test_paint(target, test)
                            || expr_uses_test_paint(value, test)
                    }
                    TestExpectation::Accessibility { target, property } => {
                        target_ref_uses_test_paint(target, test)
                            || accessibility_property_expr(property)
                                .is_some_and(|value| expr_uses_test_paint(value, test))
                    }
                },
                TestStepKind::Click { target, .. }
                | TestStepKind::Move(TestPointerPosition::Target(target))
                | TestStepKind::Press { target, .. }
                | TestStepKind::Drop(target)
                | TestStepKind::Focus(target)
                | TestStepKind::SnapEnd(target)
                | TestStepKind::Tap { target, .. }
                | TestStepKind::Accessibility { target, .. } => {
                    target_ref_uses_test_paint(target, test)
                }
                TestStepKind::Drag { from, to } => {
                    target_ref_uses_test_paint(from, test) || target_ref_uses_test_paint(to, test)
                }
                TestStepKind::Release(_)
                | TestStepKind::Leave
                | TestStepKind::Blur
                | TestStepKind::FocusNext
                | TestStepKind::FocusPrevious
                | TestStepKind::WindowFocus(_)
                | TestStepKind::Clear
                | TestStepKind::SelectAll
                | TestStepKind::CursorFront
                | TestStepKind::CursorEnd
                | TestStepKind::Composition(TestComposition::Start | TestComposition::Cancel)
                | TestStepKind::Key(_)
                | TestStepKind::KeyDown(_)
                | TestStepKind::KeyUp(_)
                | TestStepKind::Modifiers(_)
                | TestStepKind::Chord { .. }
                | TestStepKind::WindowClose
                | TestStepKind::WindowOpened
                | TestStepKind::WindowClosed
                | TestStepKind::Redraw
                | TestStepKind::SystemTheme(_)
                | TestStepKind::FileLeave
                | TestStepKind::Wait(_)
                | TestStepKind::Advance(_)
                | TestStepKind::Idle
                | TestStepKind::Capture(_) => false,
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

fn accessibility_property_expr(property: &TestAccessibilityProperty) -> Option<&Expr> {
    match property {
        TestAccessibilityProperty::Role(value)
        | TestAccessibilityProperty::Name(value)
        | TestAccessibilityProperty::Value(value)
        | TestAccessibilityProperty::Checked(value)
        | TestAccessibilityProperty::Expanded(value)
        | TestAccessibilityProperty::Disabled(value)
        | TestAccessibilityProperty::Focused(value)
        | TestAccessibilityProperty::Action {
            expected: value, ..
        } => Some(value),
    }
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
                            | "surface_count"
                            | "text_count"
                            | "image_count"
                            | "text_x"
                            | "text_y"
                            | "text_width"
                            | "text_height"
                            | "text_baseline"
                            | "image_x"
                            | "image_y"
                            | "image_width"
                            | "image_height"
                            | "image_color"
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

/// Checks one step; a component state read answers the component and state
/// indexes it resolved to, which the lowering cannot rediscover on its own.
fn check_test_step(
    step: &TestStep,
    env: &HashMap<String, Type>,
    test: &TestDecl,
    document: &Document,
    ids: &TestWidgetIds,
) -> Result<Option<(usize, usize)>, Error> {
    match &step.kind {
        TestStepKind::Click { target, .. }
        | TestStepKind::Move(TestPointerPosition::Target(target))
        | TestStepKind::Press { target, .. }
        | TestStepKind::Drop(target)
        | TestStepKind::Focus(target)
        | TestStepKind::Tap { target, .. }
        | TestStepKind::Accessibility { target, .. } => {
            check_test_target_ref(target, env, test, document, ids, &step.span)?;
        }
        TestStepKind::Drag { from, to } => {
            check_test_target_ref(from, env, test, document, ids, &step.span)?;
            check_test_target_ref(to, env, test, document, ids, &step.span)?;
        }
        TestStepKind::Scroll { target, x, y, .. } => {
            check_test_target_ref(target, env, test, document, ids, &step.span)?;
            require_test_number(
                x,
                env,
                document,
                &step.span,
                "horizontal scroll delta",
                false,
            )?;
            require_test_number(y, env, document, &step.span, "vertical scroll delta", false)?;
        }
        TestStepKind::Snap { target, x, y } => {
            check_test_target_ref(target, env, test, document, ids, &step.span)?;
            require_test_number(
                x,
                env,
                document,
                &step.span,
                "horizontal snap offset",
                false,
            )?;
            require_test_number(y, env, document, &step.span, "vertical snap offset", false)?;
        }
        TestStepKind::SnapEnd(target) => {
            check_test_target_ref(target, env, test, document, ids, &step.span)?;
        }
        TestStepKind::ClickAt { x, y, .. }
        | TestStepKind::Wheel { x, y, .. }
        | TestStepKind::Move(TestPointerPosition::Point(x, y))
        | TestStepKind::WindowMove(x, y) => {
            require_test_number(x, env, document, &step.span, "test x coordinate", false)?;
            require_test_number(y, env, document, &step.span, "test y coordinate", false)?;
        }
        TestStepKind::Release(_)
        | TestStepKind::Leave
        | TestStepKind::Blur
        | TestStepKind::FocusNext
        | TestStepKind::FocusPrevious
        | TestStepKind::WindowFocus(_)
        | TestStepKind::Clear
        | TestStepKind::SelectAll
        | TestStepKind::CursorFront
        | TestStepKind::CursorEnd
        | TestStepKind::Composition(TestComposition::Start | TestComposition::Cancel)
        | TestStepKind::Key(_)
        | TestStepKind::KeyDown(_)
        | TestStepKind::KeyUp(_)
        | TestStepKind::Modifiers(_)
        | TestStepKind::Chord { .. }
        | TestStepKind::WindowClose
        | TestStepKind::WindowOpened
        | TestStepKind::WindowClosed
        | TestStepKind::Redraw
        | TestStepKind::SystemTheme(_)
        | TestStepKind::FileLeave
        | TestStepKind::Wait(_)
        | TestStepKind::Advance(_)
        | TestStepKind::Idle
        | TestStepKind::Capture(_) => {}
        TestStepKind::Type(value)
        | TestStepKind::Replace(value)
        | TestStepKind::Composition(TestComposition::Commit(value))
        | TestStepKind::FileHover(value)
        | TestStepKind::FileDrop(value) => {
            require_type(
                &expr_type(value, env, document, &step.span)?,
                &Type::Str,
                &step.span,
            )?;
        }
        TestStepKind::Composition(TestComposition::Update { value, selection }) => {
            require_type(
                &expr_type(value, env, document, &step.span)?,
                &Type::Str,
                &step.span,
            )?;
            if let Some((start, end)) = selection {
                require_test_index(
                    start,
                    env,
                    document,
                    &step.span,
                    "composition selection start",
                )?;
                require_test_index(end, env, document, &step.span, "composition selection end")?;
            }
        }
        TestStepKind::Select(start, end) => {
            require_test_index(start, env, document, &step.span, "selection start")?;
            require_test_index(end, env, document, &step.span, "selection end")?;
        }
        TestStepKind::Cursor(index) => {
            require_test_index(index, env, document, &step.span, "cursor index")?;
        }
        TestStepKind::Repeat { count, .. } => {
            require_test_positive_integer(count, env, document, &step.span, "repeat count")?;
        }
        TestStepKind::Touch { id, x, y, .. } => {
            require_test_index(id, env, document, &step.span, "touch id")?;
            require_test_number(x, env, document, &step.span, "touch x coordinate", false)?;
            require_test_number(y, env, document, &step.span, "touch y coordinate", false)?;
        }
        TestStepKind::Resize(width, height) => {
            for (value, label) in [
                (width, "test viewport width"),
                (height, "test viewport height"),
            ] {
                require_test_number(value, env, document, &step.span, label, true)?;
            }
        }
        TestStepKind::Rescale(value) => {
            require_test_number(
                value,
                env,
                document,
                &step.span,
                "window scale factor",
                true,
            )?;
        }
        TestStepKind::TrayChoose(value) => {
            require_type(
                &expr_type(value, env, document, &step.span)?,
                &Type::Str,
                &step.span,
            )?;
            // Nothing to choose without a menu, and the row a test names is a
            // row the author wrote: a program with no tray menu can never
            // satisfy this step, so it is a mistake at check time.
            if !document.settings.tray.as_ref().is_some_and(|tray| {
                tray.menu
                    .iter()
                    .any(|row| matches!(row, TrayRow::Item { .. }))
            }) {
                return Err(Error::new(
                    TEST_ERROR,
                    &step.span,
                    "`tray choose` needs a `tray` block with a `menu`",
                ));
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
            TestExpectation::Tray { value, .. } => {
                require_type(
                    &expr_type(value, env, document, &step.span)?,
                    &Type::Str,
                    &step.span,
                )?;
            }
            TestExpectation::ComponentState {
                target,
                field,
                value,
                ..
            } => {
                let (path, label) = match target {
                    TestTargetRef::Alias(name) => (
                        &test_target_alias(test, name, &step.span)?.target,
                        format!("`{name}`"),
                    ),
                    TestTargetRef::Id(path) => (path, target_label(path)),
                };
                let Some(name) = component_scope_name(path, env, document, ids, &step.span)? else {
                    return Err(Error::new(
                        TEST_ERROR,
                        &step.span,
                        format!("{label} does not identify a component instance"),
                    )
                    .hint(
                        "`expect component` reads the state behind a component call's explicit #id",
                    ));
                };
                let (component_index, component) = document
                    .components
                    .iter()
                    .enumerate()
                    .find(|(_, component)| component.name == name)
                    .expect("widget ids name checked components");
                let Some((state_index, state)) = component
                    .states
                    .iter()
                    .enumerate()
                    .find(|(_, state)| state.name == *field)
                else {
                    let declared = component
                        .states
                        .iter()
                        .map(|state| format!("`{}`", state.name))
                        .collect::<Vec<_>>();
                    return Err(Error::new(
                        TEST_ERROR,
                        &step.span,
                        format!("component `{name}` has no state `{field}`"),
                    )
                    .hint(if declared.is_empty() {
                        format!("`{name}` declares no state")
                    } else {
                        format!("declared state: {}", declared.join(", "))
                    }));
                };
                if matches!(state.ty, Type::Animation(_)) {
                    return Err(Error::new(
                        TEST_ERROR,
                        &step.span,
                        format!(
                            "state `{field}` of `{name}` is an animation with no comparable value"
                        ),
                    ));
                }
                require_type(
                    &expr_type(value, env, document, &step.span)?,
                    &state.ty,
                    &step.span,
                )?;
                return Ok(Some((component_index, state_index)));
            }
            TestExpectation::Accessibility { target, property } => {
                check_test_target_ref(target, env, test, document, ids, &step.span)?;
                let (value, ty) = match property {
                    TestAccessibilityProperty::Role(value)
                    | TestAccessibilityProperty::Name(value)
                    | TestAccessibilityProperty::Value(value) => (value, Type::Str),
                    TestAccessibilityProperty::Checked(value)
                    | TestAccessibilityProperty::Expanded(value)
                    | TestAccessibilityProperty::Disabled(value)
                    | TestAccessibilityProperty::Focused(value)
                    | TestAccessibilityProperty::Action {
                        expected: value, ..
                    } => (value, Type::Bool),
                };
                require_type(
                    &expr_type(value, env, document, &step.span)?,
                    &ty,
                    &step.span,
                )?;
            }
        },
    }
    Ok(None)
}

fn require_test_index(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    label: &str,
) -> Result<(), Error> {
    require_type(&expr_type(value, env, document, span)?, &Type::I64, span)?;
    if let Expr::I64(value) = value
        && *value < 0
    {
        return Err(Error::new(
            TEST_ERROR,
            span,
            format!("{label} must be non-negative"),
        ));
    }
    Ok(())
}

fn require_test_positive_integer(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    label: &str,
) -> Result<(), Error> {
    require_type(&expr_type(value, env, document, span)?, &Type::I64, span)?;
    if let Expr::I64(value) = value
        && *value <= 0
    {
        return Err(Error::new(
            TEST_ERROR,
            span,
            format!("{label} must be positive"),
        ));
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
            let declared = test_target_alias(test, name, span)?;
            if component_scope_name(&declared.target, env, document, ids, span)?.is_some() {
                return Err(component_scope_error(&format!("`{name}`"), span));
            }
            Ok(())
        }
        TestTargetRef::Id(target) => check_test_widget_target(target, env, document, ids, span),
    }
}

fn test_target_alias<'a>(
    test: &'a TestDecl,
    name: &str,
    span: &Span,
) -> Result<&'a TestTargetDecl, Error> {
    test.targets
        .iter()
        .find(|target| target.name == *name)
        .ok_or_else(|| {
            Error::new(
                TEST_ERROR,
                span,
                format!("unknown test target alias `{name}`"),
            )
        })
}

/// The component whose instance scope `target` names, when it names one. A
/// path under which two different components mount in exclusive branches is
/// rejected: a read there would not know whose state it compares.
fn component_scope_name<'a>(
    target: &WidgetTarget,
    env: &HashMap<String, Type>,
    document: &Document,
    ids: &'a TestWidgetIds,
    span: &Span,
) -> Result<Option<&'a str>, Error> {
    let actual = typed_target_path(target, env, document, span)?;
    let mut names = ids
        .component_scopes
        .iter()
        .filter(|(scope, _)| widget_paths_match(scope, &actual))
        .map(|(_, name)| name.as_str());
    let Some(first) = names.next() else {
        return Ok(None);
    };
    if let Some(other) = names.find(|name| *name != first) {
        return Err(Error::new(
            TEST_ERROR,
            span,
            format!(
                "{} identifies instances of both `{first}` and `{other}`",
                target_label(target)
            ),
        )
        .hint("give each component call its own #id"));
    }
    Ok(Some(first))
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
                let label = target_label(target);
                if component_scope_name(target, env, document, ids, span)?.is_some() {
                    return Err(component_scope_error(&label, span));
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
