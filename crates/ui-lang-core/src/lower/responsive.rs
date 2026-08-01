// Stable IDs and origins are retained for validation even when the emitter
// does not inspect every field directly.
#![allow(dead_code)]

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
        let mut graph = CheckedExpressionGraph::default();
        let (semantic_key, dimensions, kind) = match (&checked_view.flow, content) {
            (
                CheckedViewFlow::ResponsiveBreakpoint {
                    semantic_key,
                    breakpoint,
                    dimensions,
                },
                ResponsiveContent::Breakpoint { .. },
            ) => {
                self.validate_responsive_expression(
                    id,
                    checked_view.scope,
                    *breakpoint,
                    CheckedViewExprRole::ResponsiveBreakpoint,
                    &Type::F64,
                    span,
                    &mut graph,
                )?;
                (
                    semantic_key,
                    dimensions,
                    ResolvedResponsiveKind::Breakpoint {
                        breakpoint: *breakpoint,
                    },
                )
            }
            (
                CheckedViewFlow::ResponsiveSize {
                    semantic_key,
                    width,
                    height,
                    dimensions,
                },
                ResponsiveContent::Size { .. },
            ) => (
                semantic_key,
                dimensions,
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
        if semantic_key != &expected_key {
            return Err(
                self.invariant(span, "responsive topology diverged after semantic checking")
            );
        }
        let width = self.resolve_responsive_length(
            id,
            checked_view.scope,
            &dimensions[0],
            CheckedViewExprRole::ResponsiveWidthDimension,
            span,
            &mut graph,
        )?;
        let height = self.resolve_responsive_length(
            id,
            checked_view.scope,
            &dimensions[1],
            CheckedViewExprRole::ResponsiveHeightDimension,
            span,
            &mut graph,
        )?;

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
        length: &CheckedResponsiveLength,
        role: CheckedViewExprRole,
        span: &Span,
        graph: &mut CheckedExpressionGraph,
    ) -> Result<Option<ResolvedResponsiveLength>, Error> {
        Ok(match length {
            CheckedResponsiveLength::None => None,
            CheckedResponsiveLength::Fill => Some(ResolvedResponsiveLength::Fill),
            CheckedResponsiveLength::FillPortion(portion) => {
                Some(ResolvedResponsiveLength::FillPortion(*portion))
            }
            CheckedResponsiveLength::Shrink => Some(ResolvedResponsiveLength::Shrink),
            CheckedResponsiveLength::Fixed { expression, source } => {
                self.validate_responsive_expression(
                    responsive,
                    scope,
                    *expression,
                    role,
                    source,
                    span,
                    graph,
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
        graph: &mut CheckedExpressionGraph,
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
            family: "responsive",
        };
        let root_scope = graph.root_scope();
        let source =
            self.validate_checked_expression_node(expression.root, &policy, graph, root_scope)?;
        if source != expression.source {
            return Err(self.invariant(span, "responsive expression root type diverged"));
        }
        Ok(())
    }
}
