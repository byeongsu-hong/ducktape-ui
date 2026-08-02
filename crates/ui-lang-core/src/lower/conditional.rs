// Stable IDs and origins are retained for validation even when the emitter
// does not inspect every field directly.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedConditional {
    pub(crate) id: ViewId,
    pub(crate) condition: CheckedExprUseId,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_conditional(
        &mut self,
        _condition: &Expr,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "if view has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::If { condition } = checked_view.flow else {
            return Err(self.invariant(span, "if view has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component {
            return Err(self.invariant(span, "if view scope diverged after semantic checking"));
        }
        let owner = CheckedExprOwner::View {
            view: id,
            role: CheckedViewExprRole::IfCondition,
        };
        if self.facts.expression_use_by_owner(owner) != Some(condition) {
            return Err(self.invariant(span, "if condition owner mapping diverged"));
        }
        let expression = self.facts.try_expression_use(condition).ok_or_else(|| {
            self.invariant(span, "if condition expression-use ID is outside its arena")
        })?;
        if expression.owner != owner
            || expression.source != Type::Bool
            || expression.destination != Type::Bool
            || expression.coercion != CheckedInitializerCoercion::None
        {
            return Err(self.invariant(span, "if condition type contract diverged"));
        }
        let policy = ViewWidgetExpressionPolicy {
            lowerer: self,
            view: id,
            scope: checked_view.scope,
            use_id: condition,
            span,
            canvas_locals: false,
            own_view_locals: false,
            allowed_own_view_locals: None,
            family: "if view",
        };
        let mut graph = CheckedExpressionGraph::default();
        let root_scope = graph.root_scope();
        let source = self.validate_checked_expression_node(
            expression.root,
            &policy,
            &mut graph,
            root_scope,
        )?;
        if source != expression.source {
            return Err(self.invariant(span, "if condition expression root type diverged"));
        }

        let resolved = ResolvedConditional {
            id,
            condition,
            origin: checked_view.origin,
        };
        if self.conditionals.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "if view was lowered more than once"));
        }
        Ok(())
    }
}
