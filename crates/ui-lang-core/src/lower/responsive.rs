use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedResponsiveLength {
    Fill,
    FillPortion(u16),
    Shrink,
    FixedF64(CheckedExprUseId),
    FixedLength(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedResponsiveLocal {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedResponsiveKind {
    Breakpoint {
        breakpoint: CheckedExprUseId,
    },
    Size {
        width: ResolvedResponsiveLocal,
        height: ResolvedResponsiveLocal,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedResponsive {
    pub(crate) id: ViewId,
    pub(crate) kind: ResolvedResponsiveKind,
    pub(crate) width: Option<ResolvedResponsiveLength>,
    pub(crate) height: Option<ResolvedResponsiveLength>,
    pub(crate) origin: OriginId,
}

#[derive(Default)]
struct ResponsiveExpressionValidation {
    graph: CheckedExpressionGraph,
    consumed: HashSet<CheckedExprUseId>,
}

impl Lowerer {
    pub(super) fn lower_responsive(
        &mut self,
        content: &ResponsiveContent,
        width: &Option<LengthValue>,
        height: &Option<LengthValue>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "responsive has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component {
            return Err(self.invariant(span, "responsive scope diverged after semantic checking"));
        }
        let expected_key = crate::ast::responsive_semantic_key(content, width, height);
        let expected_expression_count =
            crate::ast::responsive_expression_count(content, width, height);
        let mut expressions = ResponsiveExpressionValidation::default();
        let (semantic_key, expression_count, dimensions, breakpoint, kind) =
            match (&checked_view.flow, content) {
                (
                    CheckedViewFlow::ResponsiveBreakpoint {
                        semantic_key,
                        expression_count,
                        breakpoint,
                        dimensions,
                    },
                    ResponsiveContent::Breakpoint { .. },
                ) => (
                    semantic_key,
                    expression_count,
                    dimensions,
                    Some(*breakpoint),
                    ResolvedResponsiveKind::Breakpoint {
                        breakpoint: *breakpoint,
                    },
                ),
                (
                    CheckedViewFlow::ResponsiveSize {
                        semantic_key,
                        expression_count,
                        width,
                        height,
                        dimensions,
                    },
                    ResponsiveContent::Size { .. },
                ) => (
                    semantic_key,
                    expression_count,
                    dimensions,
                    None,
                    ResolvedResponsiveKind::Size {
                        width: self.resolve_responsive_local(
                            id,
                            *width,
                            CheckedViewLocalRole::ResponsiveWidth,
                            span,
                        )?,
                        height: self.resolve_responsive_local(
                            id,
                            *height,
                            CheckedViewLocalRole::ResponsiveHeight,
                            span,
                        )?,
                    },
                ),
                _ => {
                    return Err(
                        self.invariant(span, "responsive content diverged after semantic checking")
                    );
                }
            };
        if semantic_key != &expected_key
            || !responsive_length_topology_matches(width, &dimensions[0])
            || !responsive_length_topology_matches(height, &dimensions[1])
        {
            return Err(
                self.invariant(span, "responsive topology diverged after semantic checking")
            );
        }
        let checked_expression_count = u32::from(breakpoint.is_some())
            + dimensions
                .iter()
                .filter(|dimension| matches!(dimension, CheckedLength::Fixed { .. }))
                .count() as u32;
        if *expression_count != expected_expression_count
            || checked_expression_count != *expression_count
        {
            return Err(self.invariant(
                span,
                "responsive expression cardinality diverged after semantic checking",
            ));
        }
        self.validate_responsive_expression_owners(id, breakpoint, dimensions, span)?;
        if let Some(breakpoint) = breakpoint {
            self.validate_responsive_expression(
                id,
                checked_view.scope,
                breakpoint,
                CheckedViewExprRole::ResponsiveBreakpoint,
                &Type::F64,
                span,
                &mut expressions,
            )?;
        }
        let width = self.resolve_responsive_length(
            id,
            checked_view.scope,
            &dimensions[0],
            CheckedViewExprRole::ResponsiveWidthDimension,
            span,
            &mut expressions,
        )?;
        let height = self.resolve_responsive_length(
            id,
            checked_view.scope,
            &dimensions[1],
            CheckedViewExprRole::ResponsiveHeightDimension,
            span,
            &mut expressions,
        )?;
        if expressions.consumed.len() != *expression_count as usize {
            return Err(self.invariant(span, "responsive left checked expressions unconsumed"));
        }

        let resolved = ResolvedResponsive {
            id,
            kind,
            width,
            height,
            origin: checked_view.origin,
        };
        if self.responsives.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "responsive was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_responsive_local(
        &self,
        responsive: ViewId,
        local: CheckedLocalId,
        role: CheckedViewLocalRole,
        span: &Span,
    ) -> Result<ResolvedResponsiveLocal, Error> {
        let checked = self
            .facts
            .try_local(local)
            .ok_or_else(|| self.invariant(span, "responsive local ID is outside its arena"))?;
        if checked.ty != Type::F64
            || checked.owner
                != (CheckedLocalOwner::View {
                    view: responsive,
                    role,
                })
        {
            return Err(self.invariant(span, "responsive local contract diverged"));
        }
        Ok(ResolvedResponsiveLocal {
            local,
            name: checked.name.clone(),
        })
    }

    fn resolve_responsive_length(
        &self,
        responsive: ViewId,
        scope: CheckedViewScope,
        length: &CheckedLength,
        role: CheckedViewExprRole,
        span: &Span,
        expressions: &mut ResponsiveExpressionValidation,
    ) -> Result<Option<ResolvedResponsiveLength>, Error> {
        Ok(match length {
            CheckedLength::None => None,
            CheckedLength::Fill => Some(ResolvedResponsiveLength::Fill),
            CheckedLength::FillPortion(portion) => {
                Some(ResolvedResponsiveLength::FillPortion(*portion))
            }
            CheckedLength::Shrink => Some(ResolvedResponsiveLength::Shrink),
            CheckedLength::Fixed { expression, source } => {
                self.validate_responsive_expression(
                    responsive,
                    scope,
                    *expression,
                    role,
                    source,
                    span,
                    expressions,
                )?;
                Some(match source {
                    Type::F64 => ResolvedResponsiveLength::FixedF64(*expression),
                    Type::Length => ResolvedResponsiveLength::FixedLength(*expression),
                    _ => {
                        return Err(self.invariant(
                            span,
                            "responsive dimension type diverged after semantic checking",
                        ));
                    }
                })
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_responsive_expression(
        &self,
        responsive: ViewId,
        scope: CheckedViewScope,
        use_id: CheckedExprUseId,
        role: CheckedViewExprRole,
        expected: &Type,
        span: &Span,
        expressions: &mut ResponsiveExpressionValidation,
    ) -> Result<(), Error> {
        let owner = CheckedExprOwner::View {
            view: responsive,
            role,
        };
        let mapped = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
            self.invariant(span, "responsive expression has no checked owner mapping")
        })?;
        if mapped != use_id {
            return Err(self.invariant(span, "responsive expression owner mapping diverged"));
        }
        let expression = self.facts.try_expression_use(use_id).ok_or_else(|| {
            self.invariant(span, "responsive expression-use ID is outside its arena")
        })?;
        if expression.owner != owner
            || expression.source != *expected
            || expression.destination != *expected
            || expression.coercion != CheckedInitializerCoercion::None
        {
            return Err(self.invariant(span, "responsive expression contract diverged"));
        }
        let policy = ViewWidgetExpressionPolicy {
            lowerer: self,
            view: responsive,
            scope,
            use_id,
            span,
            canvas_locals: false,
            own_view_locals: false,
            allowed_own_view_locals: None,
            family: "responsive",
        };
        let root_scope = expressions.graph.root_scope();
        let source = self.validate_checked_expression_node(
            expression.root,
            &policy,
            &mut expressions.graph,
            root_scope,
        )?;
        if source != expression.source {
            return Err(self.invariant(span, "responsive expression root type diverged"));
        }
        if !expressions.consumed.insert(use_id) {
            return Err(self.invariant(span, "responsive expression was consumed more than once"));
        }
        Ok(())
    }

    fn validate_responsive_expression_owners(
        &self,
        responsive: ViewId,
        breakpoint: Option<CheckedExprUseId>,
        dimensions: &[CheckedLength; 2],
        span: &Span,
    ) -> Result<(), Error> {
        let expected = [
            (CheckedViewExprRole::ResponsiveBreakpoint, breakpoint),
            (
                CheckedViewExprRole::ResponsiveWidthDimension,
                responsive_dimension_expression(&dimensions[0]),
            ),
            (
                CheckedViewExprRole::ResponsiveHeightDimension,
                responsive_dimension_expression(&dimensions[1]),
            ),
        ];
        for (role, expected) in expected {
            let actual = self.facts.expression_use_by_owner(CheckedExprOwner::View {
                view: responsive,
                role,
            });
            if actual != expected {
                return Err(
                    self.invariant(span, "responsive expression owner cardinality diverged")
                );
            }
        }
        Ok(())
    }
}

fn responsive_dimension_expression(length: &CheckedLength) -> Option<CheckedExprUseId> {
    match length {
        CheckedLength::Fixed { expression, .. } => Some(*expression),
        _ => None,
    }
}

fn responsive_length_topology_matches(raw: &Option<LengthValue>, checked: &CheckedLength) -> bool {
    matches!(
        (raw, checked),
        (None, CheckedLength::None)
            | (Some(LengthValue::Fill), CheckedLength::Fill)
            | (Some(LengthValue::Shrink), CheckedLength::Shrink)
            | (Some(LengthValue::Fixed(_)), CheckedLength::Fixed { .. })
    ) || matches!(
        (raw, checked),
        (
            Some(LengthValue::FillPortion(raw)),
            CheckedLength::FillPortion(checked)
        ) if raw == checked
    )
}
