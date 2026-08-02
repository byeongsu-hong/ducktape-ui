use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedPinLength {
    Fill,
    FillPortion(u16),
    Shrink,
    FixedF64(CheckedExprUseId),
    FixedLength(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPin {
    pub(crate) id: ViewId,
    pub(crate) x: CheckedExprUseId,
    pub(crate) y: CheckedExprUseId,
    pub(crate) width: Option<ResolvedPinLength>,
    pub(crate) height: Option<ResolvedPinLength>,
    pub(crate) origin: OriginId,
}

struct PinOperands<'a> {
    lowerer: &'a Lowerer,
    pin: ViewId,
    next: u32,
    span: &'a Span,
}

impl PinOperands<'_> {
    fn take(&mut self) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Pin(PinExpressionId {
            pin: self.pin,
            index: self.next,
        });
        self.next += 1;
        let expression = self
            .lowerer
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, "pin expression has no owner")
            })?;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, "pin expression-use ID is outside its arena")
            })?;
        if retained.owner != owner
            || retained.source != retained.destination
            || retained.coercion != CheckedInitializerCoercion::None
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self
                .lowerer
                .invariant(self.span, "pin expression contract diverged"));
        }
        Ok(expression)
    }

    fn finish(&self, expected: u32) -> Result<(), Error> {
        if self.next != expected {
            return Err(self
                .lowerer
                .invariant(self.span, "pin left checked expressions unconsumed"));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_pin(
        &mut self,
        width: &Option<LengthValue>,
        height: &Option<LengthValue>,
        x: &Expr,
        y: &Expr,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "pin has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::Pin {
            semantic_key,
            expression_count,
        } = &checked_view.flow
        else {
            return Err(self.invariant(span, "pin has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component
            || semantic_key != &crate::ast::pin_semantic_key(width, height)
            || *expression_count as usize
                != crate::ast::pin_expression_roots(width, height, x, y).len()
        {
            return Err(self.invariant(span, "pin topology diverged after semantic checking"));
        }
        self.validate_pin_expression_graphs(id, checked_view.scope, *expression_count, span)?;

        let mut values = PinOperands {
            lowerer: self,
            pin: id,
            next: 0,
            span,
        };
        let x = values.take()?;
        let y = values.take()?;
        for position in [x, y] {
            let expression = self.facts.expression_use(position);
            if expression.source != Type::F64 || expression.destination != Type::F64 {
                return Err(self.invariant(span, "pin position type diverged after checking"));
            }
        }
        let width = self.lower_pin_length(width, &mut values)?;
        let height = self.lower_pin_length(height, &mut values)?;
        values.finish(*expression_count)?;

        let resolved = ResolvedPin {
            id,
            x,
            y,
            width,
            height,
            origin: checked_view.origin,
        };
        if self.pins.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "pin was lowered more than once"));
        }
        Ok(())
    }

    fn lower_pin_length(
        &self,
        value: &Option<LengthValue>,
        expressions: &mut PinOperands<'_>,
    ) -> Result<Option<ResolvedPinLength>, Error> {
        value
            .as_ref()
            .map(|value| {
                Ok(match value {
                    LengthValue::Fill => ResolvedPinLength::Fill,
                    LengthValue::FillPortion(value) => ResolvedPinLength::FillPortion(*value),
                    LengthValue::Shrink => ResolvedPinLength::Shrink,
                    LengthValue::Fixed(_) => {
                        let expression = expressions.take()?;
                        let source = self.facts.expression_use(expression).source.clone();
                        match source {
                            Type::F64 => ResolvedPinLength::FixedF64(expression),
                            Type::Length => ResolvedPinLength::FixedLength(expression),
                            _ => {
                                return Err(self.invariant(
                                    expressions.span,
                                    "pin length type diverged after checking",
                                ));
                            }
                        }
                    }
                })
            })
            .transpose()
    }

    fn validate_pin_expression_graphs(
        &self,
        pin: ViewId,
        scope: CheckedViewScope,
        count: u32,
        span: &Span,
    ) -> Result<(), Error> {
        let mut graph = CheckedExpressionGraph::default();
        for index in 0..count {
            let owner = CheckedExprOwner::Pin(PinExpressionId { pin, index });
            let use_id = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "pin expression has no checked owner mapping")
            })?;
            let expression = self.facts.try_expression_use(use_id).ok_or_else(|| {
                self.invariant(span, "pin expression-use ID is outside its arena")
            })?;
            if expression.owner != owner {
                return Err(self.invariant(span, "pin expression owner mapping diverged"));
            }
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: pin,
                scope,
                use_id,
                span,
                canvas_locals: false,
                own_view_locals: false,
                allowed_own_view_locals: None,
                family: "pin",
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
                    self.invariant(span, "pin expression type or coercion contract diverged")
                );
            }
        }
        Ok(())
    }
}
