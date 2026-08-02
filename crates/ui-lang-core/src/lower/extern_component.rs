// Extern component emission consumes only this normalized contract. Raw
// function names, argument syntax, borrow decisions, and routes are retained
// solely for topology validation while lowering.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedExternComponentArgumentMode {
    Owned,
    BorrowedAsRef,
    Borrowed,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedExternComponentArgument {
    pub(crate) expression: CheckedExprUseId,
    pub(crate) ty: Type,
    pub(crate) mode: ResolvedExternComponentArgumentMode,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedExternComponentFunction {
    pub(crate) id: ExternFnId,
    pub(crate) name: String,
    pub(crate) rust_path: String,
    pub(crate) declaration_origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedExternComponent {
    pub(crate) id: ViewId,
    pub(crate) function: ResolvedExternComponentFunction,
    pub(crate) arguments: Vec<ResolvedExternComponentArgument>,
    pub(crate) output: Type,
    pub(crate) route: Option<ResolvedInteractionRoute>,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_extern_component(
        &mut self,
        function: &str,
        args: &[Expr],
        route: &Option<Route>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, interaction, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::ExternComponent,
            crate::ast::extern_component_semantic_key(function, args, route),
            span,
            outer_component,
        )?;
        let checked = self
            .facts
            .extern_component(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "extern component has no checked HIR facts"))?;
        if checked.id != id {
            return Err(self.invariant(span, "extern component checked identity diverged"));
        }
        let declaration = self
            .declarations
            .try_extern_decl(checked.function)
            .filter(|declaration| {
                declaration.kind == ExternKind::Component
                    && declaration.name == function
                    && declaration.output == checked.output
                    && declaration.params.len() == args.len()
                    && declaration.borrowed.len() == args.len()
            })
            .ok_or_else(|| {
                self.invariant(span, "extern component declaration contract diverged")
            })?;
        self.origins
            .try_get(declaration.declaration.origin)
            .ok_or_else(|| {
                self.invariant(span, "extern component declaration origin is invalid")
            })?;
        if interaction.option_expressions.len() != args.len()
            || interaction.routes.len() != usize::from(route.is_some())
        {
            return Err(self.invariant(
                span,
                "extern component expression or route cardinality diverged",
            ));
        }
        self.validate_interaction_expression_graphs(id, scope, interaction.expression_count, span)?;

        let parent_origin = self
            .origins
            .try_get(origin)
            .ok_or_else(|| self.invariant(span, "extern component origin is invalid"))?;
        let mut arguments = Vec::with_capacity(args.len());
        for (index, ((expression, (_, expected)), borrowed)) in interaction
            .option_expressions
            .iter()
            .zip(&declaration.params)
            .zip(&declaration.borrowed)
            .enumerate()
        {
            let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: id,
                index: index as u32,
            });
            let retained = self
                .facts
                .try_expression_use(*expression)
                .ok_or_else(|| self.invariant(span, "extern component argument ID is invalid"))?;
            let argument_origin = self.origins.try_get(retained.origin).ok_or_else(|| {
                self.invariant(span, "extern component argument origin is invalid")
            })?;
            if retained.owner != owner
                || self.facts.expression_use_by_owner(owner) != Some(*expression)
                || retained.source != *expected
                || retained.destination != *expected
                || self.facts.try_expression(retained.root).is_none()
                || argument_origin.parent != Some(origin)
                || argument_origin.path != parent_origin.path
                || argument_origin.line != parent_origin.line
                || argument_origin.column != parent_origin.column
            {
                return Err(self.invariant(
                    span,
                    format!("extern component argument {index} contract diverged"),
                ));
            }
            let mode = match (borrowed, expected) {
                (false, _) => ResolvedExternComponentArgumentMode::Owned,
                (true, Type::Str | Type::Bytes | Type::List(_)) => {
                    ResolvedExternComponentArgumentMode::BorrowedAsRef
                }
                (true, _) => ResolvedExternComponentArgumentMode::Borrowed,
            };
            arguments.push(ResolvedExternComponentArgument {
                expression: *expression,
                ty: expected.clone(),
                mode,
                origin: retained.origin,
            });
        }

        let routes = route.iter().collect::<Vec<_>>();
        let mut route_index = 0;
        let resolved_route = self.lower_optional_interaction_route(
            route,
            &interaction,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != interaction.routes.len()
            || resolved_route
                .as_ref()
                .is_some_and(|route| route.source_payloads != vec![checked.output.clone()])
            || (resolved_route.is_none() && checked.output != Type::Unit)
        {
            return Err(self.invariant(span, "extern component output route contract diverged"));
        }

        let resolved = ResolvedExternComponent {
            id,
            function: ResolvedExternComponentFunction {
                id: checked.function,
                name: declaration.name.clone(),
                rust_path: declaration.rust_path.clone(),
                declaration_origin: declaration.declaration.origin,
            },
            arguments,
            output: checked.output,
            route: resolved_route,
            origin,
        };
        if self.extern_components.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "extern component was lowered more than once"));
        }
        Ok(())
    }
}
