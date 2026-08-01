// Stable IDs and origins are retained for validation even when the emitter
// does not inspect every field directly.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLazyBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLazy {
    pub(crate) id: ViewId,
    pub(crate) dependency: CheckedExprUseId,
    pub(crate) binding: ResolvedLazyBinding,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_lazy(
        &mut self,
        _dependency: &Expr,
        binding: &str,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "lazy has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::Lazy {
            dependency,
            binding: local,
        } = checked_view.flow
        else {
            return Err(self.invariant(span, "lazy has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component {
            return Err(self.invariant(span, "lazy scope diverged after semantic checking"));
        }
        let checked_local = self
            .facts
            .try_local(local)
            .ok_or_else(|| self.invariant(span, "lazy binding local ID is outside its arena"))?;
        if checked_local.name != binding
            || checked_local.owner
                != (CheckedLocalOwner::View {
                    view: id,
                    role: CheckedViewLocalRole::LazyDependency,
                })
        {
            return Err(self.invariant(span, "lazy binding contract diverged"));
        }
        let owner = CheckedExprOwner::View {
            view: id,
            role: CheckedViewExprRole::LazyDependency,
        };
        if self.facts.expression_use_by_owner(owner) != Some(dependency) {
            return Err(self.invariant(span, "lazy dependency owner mapping diverged"));
        }
        let expression = self.facts.try_expression_use(dependency).ok_or_else(|| {
            self.invariant(
                span,
                "lazy dependency expression-use ID is outside its arena",
            )
        })?;
        if expression.owner != owner
            || expression.source != checked_local.ty
            || expression.destination != checked_local.ty
            || expression.coercion != CheckedInitializerCoercion::None
        {
            return Err(self.invariant(span, "lazy dependency type contract diverged"));
        }
        let policy = ViewWidgetExpressionPolicy {
            lowerer: self,
            view: id,
            scope: checked_view.scope,
            use_id: dependency,
            span,
            canvas_locals: false,
            own_view_locals: false,
            family: "lazy",
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
            return Err(self.invariant(span, "lazy dependency expression root type diverged"));
        }

        let resolved = ResolvedLazy {
            id,
            dependency,
            binding: ResolvedLazyBinding {
                local,
                name: checked_local.name.clone(),
                ty: checked_local.ty.clone(),
            },
            origin: checked_view.origin,
        };
        if self.lazy_views.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "lazy was lowered more than once"));
        }
        Ok(())
    }
}
