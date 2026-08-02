use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedIterationBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    #[cfg(test)]
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedIteration {
    pub(crate) id: ViewId,
    pub(crate) items: CheckedExprUseId,
    pub(crate) item: ResolvedIterationBinding,
    pub(crate) reconciliation_line: usize,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_iteration(
        &mut self,
        _item: &str,
        _items: &Expr,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "for view has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::For { items, item: local } = checked_view.flow else {
            return Err(self.invariant(span, "for view has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component {
            return Err(self.invariant(span, "for view scope diverged after semantic checking"));
        }
        let checked_item = self
            .facts
            .try_local(local)
            .ok_or_else(|| self.invariant(span, "for item local ID is outside its arena"))?
            .clone();
        if checked_item.owner
            != (CheckedLocalOwner::View {
                view: id,
                role: CheckedViewLocalRole::ForItem,
            })
        {
            return Err(self.invariant(span, "for item binding contract diverged"));
        }
        let owner = CheckedExprOwner::View {
            view: id,
            role: CheckedViewExprRole::ForItems,
        };
        if self.facts.expression_use_by_owner(owner) != Some(items) {
            return Err(self.invariant(span, "for items owner mapping diverged"));
        }
        let expression = self.facts.try_expression_use(items).ok_or_else(|| {
            self.invariant(span, "for items expression-use ID is outside its arena")
        })?;
        let Type::List(inner) = &expression.source else {
            return Err(self.invariant(span, "for items type is not a list"));
        };
        if expression.owner != owner
            || expression.destination != expression.source
            || expression.coercion != CheckedInitializerCoercion::None
            || **inner != checked_item.ty
        {
            return Err(self.invariant(span, "for items type contract diverged"));
        }
        let policy = ViewWidgetExpressionPolicy {
            lowerer: self,
            view: id,
            scope: checked_view.scope,
            use_id: items,
            span,
            canvas_locals: false,
            own_view_locals: false,
            allowed_own_view_locals: None,
            family: "for view",
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
            return Err(self.invariant(span, "for items expression root type diverged"));
        }

        let resolved = ResolvedIteration {
            id,
            items,
            item: ResolvedIterationBinding {
                local,
                name: checked_item.name,
                #[cfg(test)]
                ty: checked_item.ty,
            },
            reconciliation_line: span.line,
            origin: checked_view.origin,
        };
        if self.iterations.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "for view was lowered more than once"));
        }
        Ok(())
    }
}
