// Themer and Shader emission consume only these normalized contracts. Raw
// extern names, argument syntax, borrow choices, dimensions, and routes remain
// available solely for topology validation while lowering.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedExternViewArgumentMode {
    Owned,
    BorrowedAsRef,
    Borrowed,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedExternViewArgument {
    pub(crate) expression: CheckedExprUseId,
    pub(crate) ty: Type,
    pub(crate) mode: ResolvedExternViewArgumentMode,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedExternViewFunction {
    pub(crate) id: ExternFnId,
    pub(crate) name: String,
    pub(crate) rust_path: String,
    pub(crate) declaration_origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedExternViewAdapter {
    pub(crate) function: ResolvedExternViewFunction,
    pub(crate) arguments: Vec<ResolvedExternViewArgument>,
    pub(crate) output: Type,
    pub(crate) route: Option<ResolvedInteractionRoute>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedThemer {
    pub(crate) id: ViewId,
    pub(crate) adapter: ResolvedExternViewAdapter,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedShader {
    pub(crate) id: ViewId,
    pub(crate) adapter: ResolvedExternViewAdapter,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_themer(
        &mut self,
        function: &str,
        args: &[Expr],
        route: &Option<Route>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, interaction, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Themer,
            crate::ast::themer_semantic_key(function, args, route),
            span,
            outer_component,
        )?;
        let checked =
            self.facts.themer(id).cloned().ok_or_else(|| {
                self.invariant_at_origin(origin, "themer has no checked HIR facts")
            })?;
        let declaration = self.extern_view_declaration(
            &checked,
            id,
            function,
            args.len(),
            ExternKind::Themer,
            origin,
        )?;
        if interaction.option_expressions.len() != args.len()
            || interaction.routes.len() != usize::from(route.is_some())
        {
            return Err(
                self.invariant_at_origin(origin, "themer expression or route cardinality diverged")
            );
        }
        self.validate_interaction_expression_graphs(id, scope, interaction.expression_count, span)?;
        let mut expression_index = 0;
        let arguments = self.lower_extern_view_arguments(
            id,
            origin,
            &interaction,
            &declaration,
            &mut expression_index,
        )?;
        if expression_index != interaction.option_expressions.len() {
            return Err(
                self.invariant_at_origin(origin, "themer left checked expressions unconsumed")
            );
        }
        let resolved_route =
            self.lower_extern_view_route(id, origin, scope, route, &interaction, &checked.output)?;
        let resolved = ResolvedThemer {
            id,
            adapter: ResolvedExternViewAdapter {
                function: self.resolved_extern_view_function(&declaration),
                arguments,
                output: checked.output,
                route: resolved_route,
            },
            origin,
        };
        if self.themers.insert(id, resolved).is_some() {
            return Err(self.invariant_at_origin(origin, "themer was lowered more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_shader(
        &mut self,
        function: &str,
        args: &[Expr],
        width: &Option<LengthValue>,
        height: &Option<LengthValue>,
        route: &Option<Route>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, interaction, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Shader,
            crate::ast::shader_semantic_key(function, args, width, height, route),
            span,
            outer_component,
        )?;
        let checked =
            self.facts.shader(id).cloned().ok_or_else(|| {
                self.invariant_at_origin(origin, "shader has no checked HIR facts")
            })?;
        let declaration = self.extern_view_declaration(
            &checked,
            id,
            function,
            args.len(),
            ExternKind::Shader,
            origin,
        )?;
        let fixed_dimensions = [width, height]
            .into_iter()
            .filter(|length| matches!(length, Some(LengthValue::Fixed(_))))
            .count();
        if interaction.option_expressions.len() != args.len() + fixed_dimensions
            || interaction.routes.len() != usize::from(route.is_some())
        {
            return Err(
                self.invariant_at_origin(origin, "shader expression or route cardinality diverged")
            );
        }
        self.validate_interaction_expression_graphs(id, scope, interaction.expression_count, span)?;
        let mut expression_index = 0;
        let arguments = self.lower_extern_view_arguments(
            id,
            origin,
            &interaction,
            &declaration,
            &mut expression_index,
        )?;
        let width =
            self.lower_extern_view_length(width, id, origin, &interaction, &mut expression_index)?;
        let height =
            self.lower_extern_view_length(height, id, origin, &interaction, &mut expression_index)?;
        if expression_index != interaction.option_expressions.len() {
            return Err(
                self.invariant_at_origin(origin, "shader left checked expressions unconsumed")
            );
        }
        let resolved_route =
            self.lower_extern_view_route(id, origin, scope, route, &interaction, &checked.output)?;
        let resolved = ResolvedShader {
            id,
            adapter: ResolvedExternViewAdapter {
                function: self.resolved_extern_view_function(&declaration),
                arguments,
                output: checked.output,
                route: resolved_route,
            },
            width,
            height,
            origin,
        };
        if self.shaders.insert(id, resolved).is_some() {
            return Err(self.invariant_at_origin(origin, "shader was lowered more than once"));
        }
        Ok(())
    }

    fn extern_view_declaration(
        &self,
        checked: &CheckedExternViewAdapter,
        id: ViewId,
        function: &str,
        argument_count: usize,
        kind: ExternKind,
        origin: OriginId,
    ) -> Result<crate::hir::ExternDeclaration, Error> {
        if checked.id != id {
            return Err(self.invariant_at_origin(origin, "extern view checked identity diverged"));
        }
        let declaration = self
            .declarations
            .try_extern_decl(checked.function)
            .filter(|declaration| {
                declaration.kind == kind
                    && declaration.name == function
                    && declaration.output == checked.output
                    && declaration.params.len() == argument_count
                    && declaration.borrowed.len() == argument_count
                    && !declaration.borrowed.iter().any(|borrowed| *borrowed)
            })
            .cloned()
            .ok_or_else(|| {
                self.invariant_at_origin(origin, "extern view declaration contract diverged")
            })?;
        self.origins
            .try_get(declaration.declaration.origin)
            .ok_or_else(|| {
                self.invariant_at_origin(origin, "extern view declaration origin is invalid")
            })?;
        Ok(declaration)
    }

    pub(super) fn resolved_extern_view_function(
        &self,
        declaration: &crate::hir::ExternDeclaration,
    ) -> ResolvedExternViewFunction {
        ResolvedExternViewFunction {
            id: declaration.declaration.id,
            name: declaration.name.clone(),
            rust_path: declaration.rust_path.clone(),
            declaration_origin: declaration.declaration.origin,
        }
    }

    pub(super) fn lower_extern_view_arguments(
        &self,
        id: ViewId,
        origin: OriginId,
        interaction: &CheckedInteraction,
        declaration: &crate::hir::ExternDeclaration,
        expression_index: &mut usize,
    ) -> Result<Vec<ResolvedExternViewArgument>, Error> {
        let mut arguments = Vec::with_capacity(declaration.params.len());
        for (index, ((_, expected), borrowed)) in declaration
            .params
            .iter()
            .zip(&declaration.borrowed)
            .enumerate()
        {
            let retained = self.extern_view_expression(
                id,
                origin,
                interaction,
                *expression_index,
                Some(expected),
                &format!("extern view argument {index}"),
            )?;
            *expression_index += 1;
            let mode = match (borrowed, expected) {
                (false, _) => ResolvedExternViewArgumentMode::Owned,
                (true, Type::Str | Type::Bytes | Type::List(_)) => {
                    ResolvedExternViewArgumentMode::BorrowedAsRef
                }
                (true, _) => ResolvedExternViewArgumentMode::Borrowed,
            };
            arguments.push(ResolvedExternViewArgument {
                expression: interaction.option_expressions[index],
                ty: expected.clone(),
                mode,
                origin: retained.origin,
            });
        }
        Ok(arguments)
    }

    fn lower_extern_view_length(
        &self,
        length: &Option<LengthValue>,
        id: ViewId,
        origin: OriginId,
        interaction: &CheckedInteraction,
        expression_index: &mut usize,
    ) -> Result<Option<ResolvedContainerLength>, Error> {
        length
            .as_ref()
            .map(|length| {
                Ok(match length {
                    LengthValue::Fill => ResolvedContainerLength::Fill,
                    LengthValue::FillPortion(portion) => {
                        ResolvedContainerLength::FillPortion(*portion)
                    }
                    LengthValue::Shrink => ResolvedContainerLength::Shrink,
                    LengthValue::Fixed(_) => {
                        let retained = self.extern_view_expression(
                            id,
                            origin,
                            interaction,
                            *expression_index,
                            None,
                            "shader dimension",
                        )?;
                        let expression = interaction.option_expressions[*expression_index];
                        *expression_index += 1;
                        match retained.source {
                            Type::F64 => ResolvedContainerLength::FixedF64(expression),
                            Type::Length => ResolvedContainerLength::FixedLength(expression),
                            _ => {
                                return Err(self.invariant_at_origin(
                                    origin,
                                    "shader dimension type diverged",
                                ));
                            }
                        }
                    }
                })
            })
            .transpose()
    }

    pub(super) fn extern_view_expression(
        &self,
        id: ViewId,
        origin: OriginId,
        interaction: &CheckedInteraction,
        index: usize,
        expected: Option<&Type>,
        label: &str,
    ) -> Result<&CheckedExprUse, Error> {
        let expression = interaction.option_expressions.get(index).ok_or_else(|| {
            self.invariant_at_origin(origin, format!("{label} is outside its partition"))
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: id,
            index: index as u32,
        });
        let retained = self
            .facts
            .try_expression_use(*expression)
            .ok_or_else(|| self.invariant_at_origin(origin, format!("{label} ID is invalid")))?;
        let expression_origin = self.origins.try_get(retained.origin).ok_or_else(|| {
            self.invariant_at_origin(origin, format!("{label} origin is invalid"))
        })?;
        let parent_origin = self.origins.get(origin);
        let expected_types_match = expected.is_none_or(|expected| {
            retained.source == *expected && retained.destination == *expected
        });
        if retained.owner != owner
            || self.facts.expression_use_by_owner(owner) != Some(*expression)
            || !expected_types_match
            || expected.is_none() && retained.source != retained.destination
            || self.facts.try_expression(retained.root).is_none()
            || expression_origin.parent != Some(origin)
            || expression_origin.path != parent_origin.path
            || expression_origin.line != parent_origin.line
            || expression_origin.column != parent_origin.column
        {
            return Err(self.invariant_at_origin(origin, format!("{label} contract diverged")));
        }
        Ok(retained)
    }

    fn lower_extern_view_route(
        &self,
        id: ViewId,
        origin: OriginId,
        scope: CheckedViewScope,
        route: &Option<Route>,
        interaction: &CheckedInteraction,
        output: &Type,
    ) -> Result<Option<ResolvedInteractionRoute>, Error> {
        let routes = route.iter().collect::<Vec<_>>();
        let mut route_index = 0;
        let resolved = self.lower_optional_interaction_route(
            route,
            interaction,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != interaction.routes.len()
            || resolved
                .as_ref()
                .is_some_and(|route| route.source_payloads != vec![output.clone()])
            || (resolved.is_none() && *output != Type::Unit)
        {
            return Err(
                self.invariant_at_origin(origin, "extern view output route contract diverged")
            );
        }
        Ok(resolved)
    }
}
