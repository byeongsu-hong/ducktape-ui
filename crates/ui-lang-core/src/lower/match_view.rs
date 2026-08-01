// Stable IDs and origins are retained for validation even when the emitter
// does not inspect every field directly.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMatchBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedMatchPattern {
    Some,
    None,
    Ok,
    Err,
    Enum { owner: String, variant: String },
    Palette { contract: String, palette: String },
    Wildcard,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMatchArm {
    pub(crate) pattern: ResolvedMatchPattern,
    pub(crate) binding: Option<ResolvedMatchBinding>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMatch {
    pub(crate) id: ViewId,
    pub(crate) value: CheckedExprUseId,
    pub(crate) value_ty: Type,
    pub(crate) arms: Vec<ResolvedMatchArm>,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_match_view(
        &mut self,
        _value: &Expr,
        raw_arms: &[MatchArm],
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "match view has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::Match { value, arms } = checked_view.flow else {
            return Err(self.invariant(span, "match view has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component || raw_arms.len() != arms.len() {
            return Err(
                self.invariant(span, "match view topology diverged after semantic checking")
            );
        }
        let owner = CheckedExprOwner::View {
            view: id,
            role: CheckedViewExprRole::MatchValue,
        };
        if self.facts.expression_use_by_owner(owner) != Some(value) {
            return Err(self.invariant(span, "match value owner mapping diverged"));
        }
        let expression = self.facts.try_expression_use(value).ok_or_else(|| {
            self.invariant(span, "match value expression-use ID is outside its arena")
        })?;
        if expression.owner != owner
            || expression.destination != expression.source
            || expression.coercion != CheckedInitializerCoercion::None
        {
            return Err(self.invariant(span, "match value type contract diverged"));
        }
        let policy = ViewWidgetExpressionPolicy {
            lowerer: self,
            view: id,
            scope: checked_view.scope,
            use_id: value,
            span,
            canvas_locals: false,
            own_view_locals: false,
            family: "match view",
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
            return Err(self.invariant(span, "match value expression root type diverged"));
        }
        let value_ty = expression.source.clone();
        let mut resolved_arms = Vec::with_capacity(arms.len());
        for (index, arm) in arms.into_iter().enumerate() {
            if self.origins.try_get(arm.origin).is_none() {
                return Err(self.invariant(span, "match arm origin ID is outside its arena"));
            }
            let (pattern, expected_binding) =
                self.resolve_checked_match_pattern(&value_ty, &arm.pattern, arm.origin)?;
            let binding = match (arm.binding, expected_binding) {
                (Some(local), Some(expected_ty)) => {
                    let checked = self.facts.try_local(local).ok_or_else(|| {
                        self.invariant_at_origin(
                            arm.origin,
                            "match payload local ID is outside its arena",
                        )
                    })?;
                    let local_origin = self.origins.try_get(checked.origin).ok_or_else(|| {
                        self.invariant_at_origin(
                            arm.origin,
                            "match payload origin ID is outside its arena",
                        )
                    })?;
                    if checked.ty != expected_ty
                        || checked.owner
                            != (CheckedLocalOwner::View {
                                view: id,
                                role: CheckedViewLocalRole::MatchPayload(index as u32),
                            })
                        || local_origin.parent != Some(arm.origin)
                    {
                        return Err(self.invariant_at_origin(
                            arm.origin,
                            "match payload local contract diverged",
                        ));
                    }
                    Some(ResolvedMatchBinding {
                        local,
                        name: checked.name.clone(),
                        ty: checked.ty.clone(),
                    })
                }
                (None, None) => None,
                _ => {
                    return Err(
                        self.invariant_at_origin(arm.origin, "match payload presence diverged")
                    );
                }
            };
            resolved_arms.push(ResolvedMatchArm {
                pattern,
                binding,
                origin: arm.origin,
            });
        }

        let resolved = ResolvedMatch {
            id,
            value,
            value_ty,
            arms: resolved_arms,
            origin: checked_view.origin,
        };
        if self.match_views.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "match view was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_checked_match_pattern(
        &self,
        value_ty: &Type,
        pattern: &CheckedMatchPattern,
        origin: OriginId,
    ) -> Result<(ResolvedMatchPattern, Option<Type>), Error> {
        Ok(match (value_ty, pattern) {
            (Type::Option(inner), CheckedMatchPattern::Some) => {
                (ResolvedMatchPattern::Some, Some(inner.as_ref().clone()))
            }
            (Type::Option(_), CheckedMatchPattern::None) => (ResolvedMatchPattern::None, None),
            (Type::Result(output, _), CheckedMatchPattern::Ok) => {
                (ResolvedMatchPattern::Ok, Some(output.as_ref().clone()))
            }
            (Type::Result(_, error), CheckedMatchPattern::Err) => {
                (ResolvedMatchPattern::Err, Some(error.as_ref().clone()))
            }
            (Type::Named(name), CheckedMatchPattern::Enum(variant_id)) => {
                let owner = self
                    .declarations
                    .try_enum_decl(variant_id.owner)
                    .ok_or_else(|| {
                        self.invariant_at_origin(origin, "match enum ID is outside its arena")
                    })?;
                let variant = self
                    .declarations
                    .try_enum_variant_decl(*variant_id)
                    .ok_or_else(|| {
                        self.invariant_at_origin(origin, "match variant ID is outside its arena")
                    })?;
                if owner.name != *name {
                    return Err(self.invariant_at_origin(origin, "match enum owner type diverged"));
                }
                (
                    ResolvedMatchPattern::Enum {
                        owner: owner.rust_name.clone(),
                        variant: variant.name.clone(),
                    },
                    variant.payload.clone(),
                )
            }
            (Type::Palette(contract), CheckedMatchPattern::Palette(palette_id)) => {
                let palette = self.declarations.palette_name(*palette_id).ok_or_else(|| {
                    self.invariant_at_origin(origin, "match palette ID is outside its arena")
                })?;
                (
                    ResolvedMatchPattern::Palette {
                        contract: contract.clone(),
                        palette: palette.to_owned(),
                    },
                    None,
                )
            }
            (_, CheckedMatchPattern::Wildcard) => (ResolvedMatchPattern::Wildcard, None),
            _ => {
                return Err(
                    self.invariant_at_origin(origin, "match pattern type contract diverged")
                );
            }
        })
    }
}
