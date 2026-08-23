use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLazyBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLazyKeyBinding {
    pub(crate) index: usize,
    pub(crate) binding: ResolvedLazyBinding,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLazy {
    pub(crate) id: ViewId,
    pub(crate) dependency: CheckedExprUseId,
    /// The cheap dependencies beside the value: `by` projections that stand
    /// in for it in the memo dependency tuple when `keyed`, extras hashed
    /// alongside it otherwise.
    pub(crate) keys: Vec<CheckedExprUseId>,
    pub(crate) keyed: bool,
    /// Bare-identifier keys are immutable snapshots inside the lazy body.
    /// They already live in the memo dependency tuple, so exposing them costs
    /// no capture and lets the cached subtree render from the exact revision
    /// inputs that selected it.
    pub(crate) key_bindings: Vec<ResolvedLazyKeyBinding>,
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
            keys,
            keyed,
            key_bindings: checked_key_bindings,
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
            allowed_own_view_locals: None,
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
        for (index, key) in keys.iter().enumerate() {
            let owner = CheckedExprOwner::View {
                view: id,
                role: CheckedViewExprRole::LazyKey(index as u32),
            };
            if self.facts.expression_use_by_owner(owner) != Some(*key) {
                return Err(self.invariant(span, "lazy key owner mapping diverged"));
            }
            let expression = self.facts.try_expression_use(*key).ok_or_else(|| {
                self.invariant(span, "lazy key expression-use ID is outside its arena")
            })?;
            if expression.owner != owner || expression.coercion != CheckedInitializerCoercion::None
            {
                return Err(self.invariant(span, "lazy key contract diverged"));
            }
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: id,
                scope: checked_view.scope,
                use_id: *key,
                span,
                canvas_locals: false,
                own_view_locals: false,
                allowed_own_view_locals: None,
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
                return Err(self.invariant(span, "lazy key expression root type diverged"));
            }
        }

        if checked_key_bindings.len() != keys.len() {
            return Err(self.invariant(span, "lazy key binding count diverged"));
        }
        let mut key_bindings = Vec::new();
        for (index, (key, binding)) in keys.iter().zip(checked_key_bindings).enumerate() {
            let (local, name) = match (binding.local, binding.name) {
                (None, None) => continue,
                (Some(local), Some(name)) => (local, name),
                _ => return Err(self.invariant(span, "lazy key binding shape diverged")),
            };
            let checked = self.facts.try_local(local).ok_or_else(|| {
                self.invariant(span, "lazy key binding local ID is outside its arena")
            })?;
            let expression = self.facts.try_expression_use(*key).ok_or_else(|| {
                self.invariant(span, "lazy key expression-use ID is outside its arena")
            })?;
            let expected_owner = CheckedLocalOwner::View {
                view: id,
                role: CheckedViewLocalRole::LazyKey(index as u32),
            };
            if checked.owner != expected_owner
                || checked.name != name
                || checked.ty != expression.source
            {
                return Err(self.invariant(span, "lazy key binding contract diverged"));
            }
            key_bindings.push(ResolvedLazyKeyBinding {
                index,
                binding: ResolvedLazyBinding {
                    local,
                    name: checked.name.clone(),
                    ty: checked.ty.clone(),
                },
            });
        }

        let resolved = ResolvedLazy {
            id,
            dependency,
            keys,
            keyed,
            key_bindings,
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
