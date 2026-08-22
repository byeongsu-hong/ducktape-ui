use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedFloatRadius {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) top_left: Option<CheckedExprUseId>,
    pub(crate) top_right: Option<CheckedExprUseId>,
    pub(crate) bottom_right: Option<CheckedExprUseId>,
    pub(crate) bottom_left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedFloat {
    pub(crate) id: ViewId,
    pub(crate) scale: CheckedExprUseId,
    pub(crate) x: CheckedExprUseId,
    pub(crate) y: CheckedExprUseId,
    pub(crate) geometry: [ResolvedFloatGeometry; 8],
    pub(crate) shadow_color: Option<ResolvedThemeColor>,
    pub(crate) shadow_x: Option<CheckedExprUseId>,
    pub(crate) shadow_y: Option<CheckedExprUseId>,
    pub(crate) shadow_blur: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedFloatRadius,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedFloatGeometry {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
}

struct FloatOperands<'a> {
    lowerer: &'a Lowerer,
    float: ViewId,
    next: u32,
    span: &'a Span,
}

impl FloatOperands<'_> {
    fn take(&mut self) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Float(FloatExpressionId {
            float: self.float,
            index: self.next,
        });
        self.next += 1;
        let expression = self
            .lowerer
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, "float expression has no owner")
            })?;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, "float expression-use ID is outside its arena")
            })?;
        if retained.owner != owner
            || retained.source != Type::F64
            || retained.destination != Type::F64
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self
                .lowerer
                .invariant(self.span, "float expression contract diverged"));
        }
        Ok(expression)
    }

    fn optional<T>(&mut self, value: Option<&T>) -> Result<Option<CheckedExprUseId>, Error> {
        value.map(|_| self.take()).transpose()
    }

    fn finish(&self, expected: u32) -> Result<(), Error> {
        if self.next != expected {
            return Err(self
                .lowerer
                .invariant(self.span, "float left checked expressions unconsumed"));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_float(
        &mut self,
        scale: &Expr,
        x: &Expr,
        y: &Expr,
        style: &FloatStyleOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "float has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::Float {
            semantic_key,
            expression_count,
            geometry,
        } = &checked_view.flow
        else {
            return Err(self.invariant(span, "float has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component
            || semantic_key != &crate::ast::float_semantic_key(style)
            || *expression_count as usize
                != crate::ast::float_expression_roots(scale, x, y, style).len()
        {
            return Err(self.invariant(span, "float topology diverged after semantic checking"));
        }
        let geometry = self.validate_float_geometry(id, geometry, span)?;
        self.validate_float_expression_graphs(id, checked_view.scope, *expression_count, span)?;

        let mut values = FloatOperands {
            lowerer: self,
            float: id,
            next: 0,
            span,
        };
        let scale = values.take()?;
        let x = values.take()?;
        let y = values.take()?;
        let shadow_x = values.optional(style.shadow_x.as_ref())?;
        let shadow_y = values.optional(style.shadow_y.as_ref())?;
        let shadow_blur = values.optional(style.shadow_blur.as_ref())?;
        let radius = ResolvedFloatRadius {
            all: values.optional(style.radius.all.as_ref())?,
            top_left: values.optional(style.radius.top_left.as_ref())?,
            top_right: values.optional(style.radius.top_right.as_ref())?,
            bottom_right: values.optional(style.radius.bottom_right.as_ref())?,
            bottom_left: values.optional(style.radius.bottom_left.as_ref())?,
        };
        values.finish(*expression_count)?;

        let resolved = ResolvedFloat {
            id,
            scale,
            x,
            y,
            geometry,
            shadow_color: style
                .shadow_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            shadow_x,
            shadow_y,
            shadow_blur,
            radius,
            origin: checked_view.origin,
        };
        if self.floats.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "float was lowered more than once"));
        }
        Ok(())
    }

    fn validate_float_geometry(
        &self,
        float: ViewId,
        geometry: &[CheckedLocalId; 8],
        span: &Span,
    ) -> Result<[ResolvedFloatGeometry; 8], Error> {
        let expected = [
            ("original_x", CheckedViewLocalRole::FloatOriginalX),
            ("original_y", CheckedViewLocalRole::FloatOriginalY),
            ("original_width", CheckedViewLocalRole::FloatOriginalWidth),
            ("original_height", CheckedViewLocalRole::FloatOriginalHeight),
            ("viewport_x", CheckedViewLocalRole::FloatViewportX),
            ("viewport_y", CheckedViewLocalRole::FloatViewportY),
            ("viewport_width", CheckedViewLocalRole::FloatViewportWidth),
            ("viewport_height", CheckedViewLocalRole::FloatViewportHeight),
        ];
        let mut resolved = Vec::with_capacity(geometry.len());
        for (local, (name, role)) in geometry.iter().zip(expected) {
            let checked = self
                .facts
                .try_local(*local)
                .ok_or_else(|| self.invariant(span, "float geometry local ID is invalid"))?;
            if checked.name != name
                || checked.ty != Type::F64
                || checked.owner != (CheckedLocalOwner::View { view: float, role })
            {
                return Err(self.invariant(span, "float geometry local contract diverged"));
            }
            resolved.push(ResolvedFloatGeometry {
                local: *local,
                name: checked.name.clone(),
            });
        }
        resolved
            .try_into()
            .map_err(|_| self.invariant(span, "float geometry local count diverged"))
    }

    fn validate_float_expression_graphs(
        &self,
        float: ViewId,
        scope: CheckedViewScope,
        count: u32,
        span: &Span,
    ) -> Result<(), Error> {
        let mut graph = CheckedExpressionGraph::default();
        for index in 0..count {
            let owner = CheckedExprOwner::Float(FloatExpressionId { float, index });
            let use_id = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "float expression has no checked owner mapping")
            })?;
            let expression = self.facts.try_expression_use(use_id).ok_or_else(|| {
                self.invariant(span, "float expression-use ID is outside its arena")
            })?;
            if expression.owner != owner {
                return Err(self.invariant(span, "float expression owner mapping diverged"));
            }
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: float,
                scope,
                use_id,
                span,
                canvas_locals: false,
                own_view_locals: matches!(index, 1 | 2),
                allowed_own_view_locals: None,
                family: "float",
            };
            let root_scope = graph.root_scope();
            let source = self.validate_checked_expression_node(
                expression.root,
                &policy,
                &mut graph,
                root_scope,
            )?;
            if source != expression.source
                || !checked_expression_coercion_is_valid(
                    &expression.source,
                    &expression.destination,
                    &expression.coercion,
                )
            {
                return Err(
                    self.invariant(span, "float expression type or coercion contract diverged")
                );
            }
        }
        Ok(())
    }
}
