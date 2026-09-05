use super::*;

struct TestOperands<'a> {
    lowerer: &'a Lowerer,
    step: TestStepId,
    span: &'a Span,
    values: Vec<ResolvedExpressionId>,
    next: usize,
}

impl<'a> TestOperands<'a> {
    fn new(
        lowerer: &'a Lowerer,
        test: TestId,
        step: TestStepId,
        source: &'a TestStep,
    ) -> Result<Self, Error> {
        let values = crate::ast::test_step_expression_roots(source)
            .into_iter()
            .enumerate()
            .map(|(operand, _)| {
                lowerer.validate_test_expression_use(
                    CheckedExprOwner::TestStepOperand {
                        step,
                        operand: operand as u32,
                    },
                    test,
                    &source.span,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            lowerer,
            step,
            span: &source.span,
            values,
            next: 0,
        })
    }

    fn take(&mut self) -> Result<ResolvedExpressionId, Error> {
        let value = self.values.get(self.next).copied().ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                format!(
                    "test step {:?} consumed more checked operands than retained",
                    self.step
                ),
            )
        })?;
        self.next += 1;
        Ok(value)
    }

    fn take_type(&mut self, expected: &Type, label: &str) -> Result<ResolvedExpressionId, Error> {
        let value = self.take()?;
        let actual = &self.lowerer.facts.expression_use(value).source;
        if !crate::check::compatible(actual, expected) {
            return Err(self.lowerer.invariant(
                self.span,
                format!(
                    "{label} retained type changed from `{}` to `{}`",
                    expected.display(),
                    actual.display()
                ),
            ));
        }
        Ok(value)
    }

    fn take_number(&mut self, label: &str, positive: bool) -> Result<ResolvedExpressionId, Error> {
        let value = self.take()?;
        let expression = self.lowerer.facts.expression_use(value);
        if !matches!(expression.source, Type::I64 | Type::F64) {
            return Err(self
                .lowerer
                .invariant(self.span, format!("{label} retained a non-numeric type")));
        }
        let root = self.lowerer.facts.expression(expression.root);
        let literal = match root.kind {
            CheckedExprKind::I64(value) => Some(value as f64),
            CheckedExprKind::F64(value) => Some(value),
            _ => None,
        };
        if let Some(value) = literal
            && (!value.is_finite() || value.abs() > f32::MAX as f64 || positive && value <= 0.0)
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("{label} retained an invalid numeric literal"),
            ));
        }
        Ok(value)
    }

    fn take_index(&mut self, label: &str, positive: bool) -> Result<ResolvedExpressionId, Error> {
        let value = self.take_type(&Type::I64, label)?;
        let expression = self.lowerer.facts.expression_use(value);
        if let CheckedExprKind::I64(literal) = self.lowerer.facts.expression(expression.root).kind
            && if positive { literal <= 0 } else { literal < 0 }
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("{label} retained an invalid integer literal"),
            ));
        }
        Ok(value)
    }

    fn finish(self) -> Result<(), Error> {
        if self.next != self.values.len() {
            return Err(self.lowerer.invariant(
                self.span,
                format!(
                    "test step {:?} left {} checked operands unconsumed",
                    self.step,
                    self.values.len() - self.next
                ),
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_tests(&self) -> Result<Vec<ResolvedTest>, Error> {
        self.document
            .tests
            .iter()
            .enumerate()
            .map(|(index, source)| self.lower_test(index, source))
            .collect()
    }

    fn lower_test(&self, index: usize, source: &TestDecl) -> Result<ResolvedTest, Error> {
        let declaration = self.declarations.test(index);
        let id = declaration.declaration.id;
        if id != TestId(index as u32)
            || declaration.name != source.name
            || declaration.semantic_key != crate::ast::test_declaration_semantic_key(source)
        {
            return Err(self.invariant(
                &source.span,
                "test declaration diverged from its checked semantic contract",
            ));
        }
        if source.viewport.is_some_and(|(width, height)| {
            !valid_positive_f32(width) || !valid_positive_f32(height)
        }) || source
            .scale_factor
            .is_some_and(|value| !valid_positive_f32(value))
            || source.timeout_ms == Some(0)
        {
            return Err(self.invariant(
                &source.span,
                "test configuration contains an invalid retained numeric contract",
            ));
        }
        if let Some(preset) = &source.preset
            && !self
                .document
                .presets
                .iter()
                .any(|value| value.name == *preset)
        {
            return Err(self.invariant(
                &source.span,
                "test configuration references an unknown retained preset",
            ));
        }

        let mut aliases = HashMap::new();
        let targets = source
            .targets
            .iter()
            .enumerate()
            .map(|(target_index, target)| {
                let id = TestTargetId {
                    test: id,
                    index: target_index as u32,
                };
                let retained = self.declarations.test_target(id).ok_or_else(|| {
                    self.invariant(&target.span, "test target has no stable declaration")
                })?;
                let local_id = self
                    .facts
                    .local_by_owner(CheckedLocalOwner::TestTarget(id))
                    .ok_or_else(|| self.invariant(&target.span, "test target has no local ID"))?;
                let local = self.facts.try_local(local_id).ok_or_else(|| {
                    self.invariant(&target.span, "test target local ID is outside its arena")
                })?;
                if retained.name != target.name
                    || local.name != retained.name
                    || local.ty != Type::TestTarget
                    || local.origin != retained.declaration.origin
                {
                    return Err(self.invariant(
                        &target.span,
                        "test target diverged from its checked declaration",
                    ));
                }
                let path =
                    self.lower_declared_test_target_path(id, &target.target, &target.span)?;
                if aliases.insert(retained.name.clone(), id).is_some() {
                    return Err(
                        self.invariant(&target.span, "duplicate resolved test target alias")
                    );
                }
                Ok(ResolvedTestTarget {
                    id,
                    name: retained.name.clone(),
                    path,
                    local: local_id,
                    origin: retained.declaration.origin,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let steps = source
            .steps
            .iter()
            .enumerate()
            .map(|(step_index, step)| {
                let step_id = TestStepId {
                    test: id,
                    index: step_index as u32,
                };
                let retained = self.declarations.test_step(step_id).ok_or_else(|| {
                    self.invariant(&step.span, "test step has no stable declaration")
                })?;
                if retained.semantic_key != crate::ast::test_step_semantic_key(step) {
                    return Err(self.invariant(
                        &step.span,
                        "test step diverged from its checked semantic contract",
                    ));
                }
                let mut operands = TestOperands::new(self, id, step_id, step)?;
                let kind = self.lower_test_step_kind(step, &aliases, &mut operands)?;
                operands.finish()?;
                Ok(ResolvedTestStep {
                    id: step_id,
                    kind,
                    source: retained.source.clone(),
                    origin: retained.declaration.origin,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let mount = source
            .mount
            .as_ref()
            .map(|mount| {
                self.declarations.view_id(mount.span()).ok_or_else(|| {
                    self.invariant(&source.span, "test mount has no stable root view identity")
                })
            })
            .transpose()?;
        Ok(ResolvedTest {
            id,
            name: declaration.name.clone(),
            config: ResolvedTestConfig {
                viewport: source.viewport,
                timeout_ms: source.timeout_ms,
                theme: source.theme.map(resolve_theme),
                scale_factor: source.scale_factor,
                locale: source.locale.clone(),
                platform: source.platform.map(resolve_platform),
                reduced_motion: source.reduced_motion,
                preset: source.preset.clone(),
            },
            window_local: self.facts.daemon_window_local(),
            mount,
            targets,
            steps,
            origin: declaration.declaration.origin,
        })
    }

    fn lower_declared_test_target_path(
        &self,
        id: TestTargetId,
        source: &WidgetTarget,
        span: &Span,
    ) -> Result<ResolvedTestTargetPath, Error> {
        let retained = self
            .declarations
            .test_target(id)
            .ok_or_else(|| self.invariant(span, "test target has no declaration"))?;
        if retained.segments.len() != source.segments.len() {
            return Err(self.invariant(span, "test target path topology changed"));
        }
        let segments = source
            .segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                if retained.segments[index] != (segment.name.clone(), segment.key.is_some()) {
                    return Err(self.invariant(span, "test target path semantic contract changed"));
                }
                let key = segment
                    .key
                    .as_ref()
                    .map(|_| {
                        self.validate_test_expression_use(
                            CheckedExprOwner::TestTargetKey {
                                target: id,
                                segment: index as u32,
                            },
                            id.test,
                            span,
                        )
                    })
                    .transpose()?;
                Ok(ResolvedTestTargetSegment {
                    name: retained.segments[index].0.clone(),
                    key,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(ResolvedTestTargetPath { segments })
    }

    fn lower_test_target_ref(
        &self,
        source: &TestTargetRef,
        aliases: &HashMap<String, TestTargetId>,
        operands: &mut TestOperands<'_>,
    ) -> Result<ResolvedTestTargetRef, Error> {
        match source {
            TestTargetRef::Alias(name) => aliases
                .get(name)
                .copied()
                .map(ResolvedTestTargetRef::Alias)
                .ok_or_else(|| {
                    self.invariant(
                        operands.span,
                        "test step references an unknown target alias",
                    )
                }),
            TestTargetRef::Id(target) => {
                let segments = target
                    .segments
                    .iter()
                    .map(|segment| {
                        Ok(ResolvedTestTargetSegment {
                            name: segment.name.clone(),
                            key: segment.key.as_ref().map(|_| operands.take()).transpose()?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok(ResolvedTestTargetRef::Id(ResolvedTestTargetPath {
                    segments,
                }))
            }
        }
    }

    fn lower_test_step_kind(
        &self,
        step: &TestStep,
        aliases: &HashMap<String, TestTargetId>,
        values: &mut TestOperands<'_>,
    ) -> Result<ResolvedTestStepKind, Error> {
        let target = |source, values: &mut TestOperands<'_>| {
            self.lower_test_target_ref(source, aliases, values)
        };
        Ok(match &step.kind {
            TestStepKind::Click {
                target: source,
                button,
                count,
            } => ResolvedTestStepKind::Click {
                target: target(source, values)?,
                button: resolve_button(*button),
                count: *count,
            },
            TestStepKind::ClickAt {
                x,
                y,
                button,
                count,
            } => {
                let _ = (x, y);
                ResolvedTestStepKind::ClickAt {
                    x: values.take_number("test x coordinate", false)?,
                    y: values.take_number("test y coordinate", false)?,
                    button: resolve_button(*button),
                    count: *count,
                }
            }
            TestStepKind::Leave => ResolvedTestStepKind::Leave,
            TestStepKind::Move(TestPointerPosition::Target(source)) => {
                ResolvedTestStepKind::MoveTarget(target(source, values)?)
            }
            TestStepKind::Move(TestPointerPosition::Point(..)) => ResolvedTestStepKind::MovePoint(
                values.take_number("test x coordinate", false)?,
                values.take_number("test y coordinate", false)?,
            ),
            TestStepKind::Press {
                target: source,
                button,
            } => ResolvedTestStepKind::Press {
                target: target(source, values)?,
                button: resolve_button(*button),
            },
            TestStepKind::Release(button) => ResolvedTestStepKind::Release(resolve_button(*button)),
            TestStepKind::Wheel { unit, .. } => ResolvedTestStepKind::Wheel {
                unit: resolve_wheel_unit(*unit),
                x: values.take_number("horizontal wheel delta", false)?,
                y: values.take_number("vertical wheel delta", false)?,
            },
            TestStepKind::Scroll {
                mode,
                target: source,
                ..
            } => ResolvedTestStepKind::Scroll {
                mode: resolve_scroll_mode(*mode),
                target: target(source, values)?,
                x: values.take_number("horizontal scroll delta", false)?,
                y: values.take_number("vertical scroll delta", false)?,
            },
            TestStepKind::Snap { target: source, .. } => ResolvedTestStepKind::Snap {
                target: target(source, values)?,
                x: values.take_number("horizontal snap offset", false)?,
                y: values.take_number("vertical snap offset", false)?,
            },
            TestStepKind::SnapEnd(source) => ResolvedTestStepKind::SnapEnd(target(source, values)?),
            TestStepKind::Drag { from, to } => ResolvedTestStepKind::Drag {
                from: target(from, values)?,
                to: target(to, values)?,
            },
            TestStepKind::Drop(source) => ResolvedTestStepKind::Drop(target(source, values)?),
            TestStepKind::Focus(source) => ResolvedTestStepKind::Focus(target(source, values)?),
            TestStepKind::FocusNext => ResolvedTestStepKind::FocusNext,
            TestStepKind::FocusPrevious => ResolvedTestStepKind::FocusPrevious,
            TestStepKind::Blur => ResolvedTestStepKind::Blur,
            TestStepKind::WindowFocus(value) => ResolvedTestStepKind::WindowFocus(*value),
            TestStepKind::Type(_) => {
                ResolvedTestStepKind::Type(values.take_type(&Type::Str, "typed text")?)
            }
            TestStepKind::Clear => ResolvedTestStepKind::Clear,
            TestStepKind::Replace(_) => {
                ResolvedTestStepKind::Replace(values.take_type(&Type::Str, "replacement text")?)
            }
            TestStepKind::Select(..) => ResolvedTestStepKind::Select(
                values.take_index("selection start", false)?,
                values.take_index("selection end", false)?,
            ),
            TestStepKind::SelectAll => ResolvedTestStepKind::SelectAll,
            TestStepKind::Cursor(_) => {
                ResolvedTestStepKind::Cursor(values.take_index("cursor index", false)?)
            }
            TestStepKind::CursorFront => ResolvedTestStepKind::CursorFront,
            TestStepKind::CursorEnd => ResolvedTestStepKind::CursorEnd,
            TestStepKind::Composition(composition) => {
                ResolvedTestStepKind::Composition(match composition {
                    TestComposition::Start => ResolvedTestComposition::Start,
                    TestComposition::Update { selection, .. } => ResolvedTestComposition::Update {
                        value: values.take_type(&Type::Str, "composition value")?,
                        selection: selection
                            .as_ref()
                            .map(|_| {
                                Ok((
                                    values.take_index("composition selection start", false)?,
                                    values.take_index("composition selection end", false)?,
                                ))
                            })
                            .transpose()?,
                    },
                    TestComposition::Commit(_) => ResolvedTestComposition::Commit(
                        values.take_type(&Type::Str, "composition value")?,
                    ),
                    TestComposition::Cancel => ResolvedTestComposition::Cancel,
                })
            }
            TestStepKind::Key(key) => ResolvedTestStepKind::Key(resolve_key(key)),
            TestStepKind::KeyDown(event) => ResolvedTestStepKind::KeyDown(resolve_key_event(event)),
            TestStepKind::KeyUp(event) => ResolvedTestStepKind::KeyUp(resolve_key_event(event)),
            TestStepKind::Modifiers(modifiers) => {
                ResolvedTestStepKind::Modifiers(resolve_modifiers(*modifiers))
            }
            TestStepKind::Chord { modifiers, key } => ResolvedTestStepKind::Chord {
                modifiers: resolve_modifiers(*modifiers),
                key: resolve_key(key),
            },
            TestStepKind::Repeat { key, .. } => ResolvedTestStepKind::Repeat {
                key: resolve_key(key),
                count: values.take_index("repeat count", true)?,
            },
            TestStepKind::Tap {
                target: source,
                count,
            } => ResolvedTestStepKind::Tap {
                target: target(source, values)?,
                count: *count,
            },
            TestStepKind::Touch { phase, .. } => ResolvedTestStepKind::Touch {
                phase: resolve_touch_phase(*phase),
                id: values.take_index("touch id", false)?,
                x: values.take_number("touch x coordinate", false)?,
                y: values.take_number("touch y coordinate", false)?,
            },
            TestStepKind::WindowMove(..) => ResolvedTestStepKind::WindowMove(
                values.take_number("window x coordinate", false)?,
                values.take_number("window y coordinate", false)?,
            ),
            TestStepKind::Resize(..) => ResolvedTestStepKind::Resize(
                values.take_number("test viewport width", true)?,
                values.take_number("test viewport height", true)?,
            ),
            TestStepKind::Rescale(_) => {
                ResolvedTestStepKind::Rescale(values.take_number("window scale factor", true)?)
            }
            TestStepKind::WindowClose => ResolvedTestStepKind::WindowClose,
            TestStepKind::WindowOpened => ResolvedTestStepKind::WindowOpened,
            TestStepKind::WindowClosed => ResolvedTestStepKind::WindowClosed,
            TestStepKind::Redraw => ResolvedTestStepKind::Redraw,
            TestStepKind::SystemTheme(theme) => {
                ResolvedTestStepKind::SystemTheme(resolve_theme(*theme))
            }
            TestStepKind::FileHover(_) => {
                ResolvedTestStepKind::FileHover(values.take_type(&Type::Str, "hovered file path")?)
            }
            TestStepKind::FileDrop(_) => {
                ResolvedTestStepKind::FileDrop(values.take_type(&Type::Str, "dropped file path")?)
            }
            TestStepKind::FileLeave => ResolvedTestStepKind::FileLeave,
            TestStepKind::Wait(duration) => ResolvedTestStepKind::Wait(*duration),
            TestStepKind::Advance(duration) => ResolvedTestStepKind::Advance(*duration),
            TestStepKind::Idle => ResolvedTestStepKind::Idle,
            TestStepKind::Capture(name) => ResolvedTestStepKind::Capture(name.clone()),
            TestStepKind::Accessibility {
                action,
                target: source,
            } => ResolvedTestStepKind::Accessibility {
                action: resolve_accessibility_action(*action),
                target: target(source, values)?,
            },
            TestStepKind::Dispatch { handler, args } => {
                let handler_id = self
                    .declarations
                    .handler_id(HandlerOwner::App, handler)
                    .ok_or_else(|| {
                        self.invariant(&step.span, "test dispatch handler disappeared")
                    })?;
                let declaration = self.declarations.try_handler(handler_id).ok_or_else(|| {
                    self.invariant(&step.span, "test dispatch handler ID is outside its arena")
                })?;
                if declaration.owner != HandlerOwner::App
                    || declaration.name != *handler
                    || declaration.payloads.len() != args.len()
                {
                    return Err(self.invariant(
                        &step.span,
                        "test dispatch handler contract changed before lowering",
                    ));
                }
                let args = declaration
                    .payloads
                    .iter()
                    .enumerate()
                    .map(|(index, expected)| {
                        values.take_type(expected, &format!("dispatch argument {index}"))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                ResolvedTestStepKind::Dispatch {
                    handler: handler_id,
                    handler_name: declaration.name.clone(),
                    args,
                }
            }
            TestStepKind::TrayChoose(_) => {
                ResolvedTestStepKind::TrayChoose(values.take_type(&Type::Str, "tray choose")?)
            }
            TestStepKind::Expect(expectation) => ResolvedTestStepKind::Expect(
                self.lower_test_expectation(expectation, aliases, values)?,
            ),
        })
    }

    fn lower_test_expectation(
        &self,
        expectation: &TestExpectation,
        aliases: &HashMap<String, TestTargetId>,
        values: &mut TestOperands<'_>,
    ) -> Result<ResolvedTestExpectation, Error> {
        let target = |source, values: &mut TestOperands<'_>| {
            self.lower_test_target_ref(source, aliases, values)
        };
        Ok(match expectation {
            TestExpectation::Expr(_) => {
                let expression = values.take_type(&Type::Bool, "test expectation")?;
                let root = self.facts.expression_use(expression).root;
                match &self.facts.expression(root).kind {
                    CheckedExprKind::Binary {
                        left,
                        operator: CheckedBinaryOperator::Equality { op, .. },
                        right,
                    } => ResolvedTestExpectation::Equality {
                        left: *left,
                        right: *right,
                        negated: *op == BinaryOp::NotEq,
                        expression,
                    },
                    _ => ResolvedTestExpectation::Expr {
                        expression,
                        unwrap_operator: matches!(
                            self.facts.expression(root).kind,
                            CheckedExprKind::Unary { .. } | CheckedExprKind::Binary { .. }
                        ),
                    },
                }
            }
            TestExpectation::Approx { .. } => ResolvedTestExpectation::Approx {
                left: values.take_number("approximate value", false)?,
                right: values.take_number("approximate value", false)?,
            },
            TestExpectation::Exists(source) => {
                ResolvedTestExpectation::Exists(target(source, values)?)
            }
            TestExpectation::Missing(source) => {
                ResolvedTestExpectation::Missing(target(source, values)?)
            }
            TestExpectation::Text {
                within, negated, ..
            } => ResolvedTestExpectation::Text {
                value: values.take_type(&Type::Str, "text expectation")?,
                within: within
                    .as_ref()
                    .map(|source| target(source, values))
                    .transpose()?,
                negated: *negated,
            },
            TestExpectation::Tray { field, negated, .. } => ResolvedTestExpectation::Tray {
                field: match field {
                    TrayField::Label => ResolvedTrayField::Label,
                    TrayField::Icon => ResolvedTrayField::Icon,
                    TrayField::Item => ResolvedTrayField::Item,
                    TrayField::Command => ResolvedTrayField::Command,
                },
                value: values.take_type(&Type::Str, "tray expectation")?,
                negated: *negated,
            },
            TestExpectation::ComponentState {
                target: source,
                negated,
                ..
            } => {
                let target = target(source, values)?;
                let read = self
                    .test_component_reads
                    .iter()
                    .find(|read| read.step == values.step)
                    .ok_or_else(|| {
                        self.invariant(
                            values.span,
                            "component state expectation has no checked component read",
                        )
                    })?;
                let state = self
                    .components
                    .iter()
                    .find(|component| component.id == read.component)
                    .and_then(|component| {
                        component.states.iter().find(|state| state.id == read.state)
                    })
                    .ok_or_else(|| {
                        self.invariant(
                            values.span,
                            "component state expectation names a state outside the arena",
                        )
                    })?;
                ResolvedTestExpectation::ComponentState {
                    target,
                    component: read.component,
                    state: read.state,
                    value: values.take_type(&state.ty, "component state expectation")?,
                    negated: *negated,
                }
            }
            TestExpectation::Accessibility {
                target: source,
                property,
            } => {
                let target = target(source, values)?;
                let property = match property {
                    TestAccessibilityProperty::Role(_) => ResolvedTestAccessibilityProperty::Role(
                        values.take_type(&Type::Str, "accessibility role")?,
                    ),
                    TestAccessibilityProperty::Name(_) => ResolvedTestAccessibilityProperty::Name(
                        values.take_type(&Type::Str, "accessibility name")?,
                    ),
                    TestAccessibilityProperty::Value(_) => {
                        ResolvedTestAccessibilityProperty::Value(
                            values.take_type(&Type::Str, "accessibility value")?,
                        )
                    }
                    TestAccessibilityProperty::Checked(_) => {
                        ResolvedTestAccessibilityProperty::Checked(
                            values.take_type(&Type::Bool, "accessibility checked state")?,
                        )
                    }
                    TestAccessibilityProperty::Expanded(_) => {
                        ResolvedTestAccessibilityProperty::Expanded(
                            values.take_type(&Type::Bool, "accessibility expanded state")?,
                        )
                    }
                    TestAccessibilityProperty::Disabled(_) => {
                        ResolvedTestAccessibilityProperty::Disabled(
                            values.take_type(&Type::Bool, "accessibility disabled state")?,
                        )
                    }
                    TestAccessibilityProperty::Focused(_) => {
                        ResolvedTestAccessibilityProperty::Focused(
                            values.take_type(&Type::Bool, "accessibility focus state")?,
                        )
                    }
                    TestAccessibilityProperty::Action { name, .. } => {
                        ResolvedTestAccessibilityProperty::Action {
                            name: name.clone(),
                            expected: values
                                .take_type(&Type::Bool, "accessibility action support")?,
                        }
                    }
                };
                ResolvedTestExpectation::Accessibility { target, property }
            }
        })
    }
}

fn resolve_theme(value: TestTheme) -> ResolvedTestTheme {
    match value {
        TestTheme::Light => ResolvedTestTheme::Light,
        TestTheme::Dark => ResolvedTestTheme::Dark,
        TestTheme::None => ResolvedTestTheme::None,
    }
}

fn resolve_platform(value: TestPlatform) -> ResolvedTestPlatform {
    match value {
        TestPlatform::Linux => ResolvedTestPlatform::Linux,
        TestPlatform::Windows => ResolvedTestPlatform::Windows,
        TestPlatform::Macos => ResolvedTestPlatform::Macos,
        TestPlatform::Wasm => ResolvedTestPlatform::Wasm,
    }
}

fn resolve_button(value: TestMouseButton) -> ResolvedTestMouseButton {
    match value {
        TestMouseButton::Left => ResolvedTestMouseButton::Left,
        TestMouseButton::Right => ResolvedTestMouseButton::Right,
        TestMouseButton::Middle => ResolvedTestMouseButton::Middle,
        TestMouseButton::Back => ResolvedTestMouseButton::Back,
        TestMouseButton::Forward => ResolvedTestMouseButton::Forward,
    }
}

fn resolve_wheel_unit(value: TestWheelUnit) -> ResolvedTestWheelUnit {
    match value {
        TestWheelUnit::Pixels => ResolvedTestWheelUnit::Pixels,
        TestWheelUnit::Lines => ResolvedTestWheelUnit::Lines,
    }
}

fn resolve_scroll_mode(value: TestScrollMode) -> ResolvedTestScrollMode {
    match value {
        TestScrollMode::To => ResolvedTestScrollMode::To,
        TestScrollMode::By => ResolvedTestScrollMode::By,
    }
}

fn resolve_touch_phase(value: TestTouchPhase) -> ResolvedTestTouchPhase {
    match value {
        TestTouchPhase::Down => ResolvedTestTouchPhase::Down,
        TestTouchPhase::Move => ResolvedTestTouchPhase::Move,
        TestTouchPhase::Up => ResolvedTestTouchPhase::Up,
        TestTouchPhase::Cancel => ResolvedTestTouchPhase::Cancel,
    }
}

fn resolve_accessibility_action(value: TestAccessibilityAction) -> ResolvedTestAccessibilityAction {
    match value {
        TestAccessibilityAction::Activate => ResolvedTestAccessibilityAction::Activate,
        TestAccessibilityAction::Focus => ResolvedTestAccessibilityAction::Focus,
        TestAccessibilityAction::Increment => ResolvedTestAccessibilityAction::Increment,
        TestAccessibilityAction::Decrement => ResolvedTestAccessibilityAction::Decrement,
    }
}

fn resolve_key(value: &TestKey) -> ResolvedTestKey {
    match value {
        TestKey::Named(name) => ResolvedTestKey::Named(name.clone()),
        TestKey::Character(value) => ResolvedTestKey::Character(value.clone()),
    }
}

fn resolve_key_location(value: TestKeyLocation) -> ResolvedTestKeyLocation {
    match value {
        TestKeyLocation::Standard => ResolvedTestKeyLocation::Standard,
        TestKeyLocation::Left => ResolvedTestKeyLocation::Left,
        TestKeyLocation::Right => ResolvedTestKeyLocation::Right,
        TestKeyLocation::Numpad => ResolvedTestKeyLocation::Numpad,
    }
}

fn resolve_modifiers(value: TestModifiers) -> ResolvedTestModifiers {
    ResolvedTestModifiers {
        shift: value.shift,
        control: value.control,
        alt: value.alt,
        logo: value.logo,
    }
}

fn resolve_key_event(value: &TestKeyEvent) -> ResolvedTestKeyEvent {
    ResolvedTestKeyEvent {
        key: resolve_key(&value.key),
        modified_key: value.modified_key.as_ref().map(resolve_key),
        location: resolve_key_location(value.location),
        physical: value.physical.clone(),
        text: value.text.clone(),
        repeat: value.repeat,
    }
}
