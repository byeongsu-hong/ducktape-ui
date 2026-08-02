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
    pub(crate) children: Vec<ViewId>,
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
        self.validate_checked_match_coverage(&value_ty, &arms, checked_view.origin)?;
        let checked_children = arms
            .iter()
            .flat_map(|arm| arm.children.iter().copied())
            .collect::<Vec<_>>();
        if checked_children != checked_view.children {
            return Err(self.invariant_at_origin(
                checked_view.origin,
                "match checked arm children diverged from the checked view topology",
            ));
        }
        for (raw_arm, arm) in raw_arms.iter().zip(&arms) {
            let Some(origin) = self.origins.try_get(arm.origin) else {
                return Err(self.match_arm_invariant(
                    &raw_arm.span,
                    "match arm origin ID is outside its arena",
                ));
            };
            let (expected_path, expected_line) = self
                .origins
                .source_origin(raw_arm.span.line)
                .map_or((None, raw_arm.span.line), |(path, line)| (Some(path), line));
            if origin.parent != Some(checked_view.origin)
                || origin.path.as_deref() != expected_path
                || origin.line != expected_line
                || origin.column != raw_arm.span.column
            {
                return Err(self.match_arm_invariant(
                    &raw_arm.span,
                    "match arm origin diverged from its checked parent or source",
                ));
            }
            let raw_children = raw_arm
                .children
                .iter()
                .map(|child| {
                    self.declarations.view_id(child.span()).ok_or_else(|| {
                        self.invariant_at_origin(
                            arm.origin,
                            "match arm child has no shared view ID",
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if raw_children != arm.children {
                return Err(self.invariant_at_origin(
                    arm.origin,
                    "match raw arm children diverged from checked arm topology",
                ));
            }
            for child in &arm.children {
                let valid_parent = self
                    .facts
                    .views()
                    .get(child.0 as usize)
                    .is_some_and(|checked| checked.id == *child && checked.parent == Some(id));
                if !valid_parent {
                    return Err(self.invariant_at_origin(
                        arm.origin,
                        "match checked arm child has an invalid parent or view ID",
                    ));
                }
            }
        }
        let mut resolved_arms = Vec::with_capacity(arms.len());
        for (index, arm) in arms.into_iter().enumerate() {
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
                children: arm.children,
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

    fn validate_checked_match_coverage(
        &self,
        value_ty: &Type,
        arms: &[CheckedMatchArm],
        match_origin: OriginId,
    ) -> Result<(), Error> {
        let mut wildcard = false;
        let mut option_cases = HashSet::new();
        let mut result_cases = HashSet::new();
        let mut enum_variants = HashSet::new();
        let mut palettes = HashSet::new();

        for (index, arm) in arms.iter().enumerate() {
            if matches!(arm.pattern, CheckedMatchPattern::Wildcard) {
                if wildcard || index + 1 != arms.len() {
                    return Err(self.invariant_at_origin(
                        arm.origin,
                        "match checked wildcard topology diverged",
                    ));
                }
                wildcard = true;
                continue;
            }
            let inserted = match (value_ty, &arm.pattern) {
                (Type::Option(_), CheckedMatchPattern::Some) => option_cases.insert(0_u8),
                (Type::Option(_), CheckedMatchPattern::None) => option_cases.insert(1_u8),
                (Type::Result(_, _), CheckedMatchPattern::Ok) => result_cases.insert(0_u8),
                (Type::Result(_, _), CheckedMatchPattern::Err) => result_cases.insert(1_u8),
                (Type::Named(_), CheckedMatchPattern::Enum(id)) => {
                    enum_variants.insert(id.to_owned())
                }
                (Type::Palette(_), CheckedMatchPattern::Palette(id)) => {
                    palettes.insert(id.to_owned())
                }
                _ => {
                    return Err(self.invariant_at_origin(
                        arm.origin,
                        "match checked pattern type contract diverged",
                    ));
                }
            };
            if !inserted {
                return Err(self.invariant_at_origin(
                    arm.origin,
                    "match checked patterns contain a duplicate case",
                ));
            }
        }

        if wildcard {
            return Ok(());
        }
        let exhaustive = match value_ty {
            Type::Option(_) => option_cases.len() == 2,
            Type::Result(_, _) => result_cases.len() == 2,
            Type::Named(name) => self
                .declarations
                .enum_decl_by_name(name)
                .is_some_and(|owner| enum_variants.len() == owner.variants.len()),
            Type::Palette(_) => palettes.len() == self.declarations.palette_count(),
            _ => false,
        };
        if !exhaustive {
            return Err(
                self.invariant_at_origin(match_origin, "match checked patterns are not exhaustive")
            );
        }
        Ok(())
    }

    fn match_arm_invariant(&self, span: &Span, message: impl Into<String>) -> Error {
        let message = format!("lowering invariant failed: {}", message.into());
        let (path, line) = self
            .origins
            .source_origin(span.line)
            .map_or((None, span.line), |(path, line)| (Some(path), line));
        let mut error = Error::new(
            "E196",
            &Span {
                line,
                column: span.column,
            },
            message,
        );
        if let Some(path) = path {
            error = error.at_path(path.display().to_string());
        }
        error
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

impl LoweredProgram {
    pub(crate) fn validate_match_arm_children(
        &self,
        raw: &MatchArm,
        resolved: &ResolvedMatchArm,
    ) -> Result<(), Error> {
        let children = raw
            .children
            .iter()
            .map(|child| {
                self.declarations.view_id(child.span()).ok_or_else(|| {
                    self.invariant_at_origin(
                        resolved.origin,
                        "match raw arm child has no shared view ID",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if children != resolved.children {
            return Err(self.invariant_at_origin(
                resolved.origin,
                "match raw arm children diverged from normalized HIR topology",
            ));
        }
        Ok(())
    }
}
