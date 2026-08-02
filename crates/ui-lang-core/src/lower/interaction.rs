// Stable route IDs, payload contracts, and origins are retained for backend
// validation even when today's emitter does not inspect every field.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Debug)]
pub(crate) enum ResolvedInteractionWidget {
    MouseArea(Box<ResolvedMouseArea>),
    ResizeHandle(Box<ResolvedResizeHandle>),
    Sensor(Box<ResolvedSensor>),
}

impl ResolvedInteractionWidget {
    pub(crate) fn id(&self) -> ViewId {
        match self {
            Self::MouseArea(widget) => widget.id,
            Self::ResizeHandle(widget) => widget.id,
            Self::Sensor(widget) => widget.id,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMouseArea {
    pub(crate) id: ViewId,
    pub(crate) press: Option<ResolvedInteractionRoute>,
    pub(crate) release: Option<ResolvedInteractionRoute>,
    pub(crate) double_click: Option<ResolvedInteractionRoute>,
    pub(crate) right_press: Option<ResolvedInteractionRoute>,
    pub(crate) right_release: Option<ResolvedInteractionRoute>,
    pub(crate) middle_press: Option<ResolvedInteractionRoute>,
    pub(crate) middle_release: Option<ResolvedInteractionRoute>,
    pub(crate) enter: Option<ResolvedInteractionRoute>,
    pub(crate) exit: Option<ResolvedInteractionRoute>,
    pub(crate) move_route: Option<ResolvedInteractionRoute>,
    pub(crate) scroll: Option<ResolvedInteractionRoute>,
    pub(crate) interaction: Option<MouseInteraction>,
    pub(crate) interaction_expression: Option<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedResizeHandle {
    pub(crate) id: ViewId,
    pub(crate) drag: Option<ResolvedInteractionRoute>,
    pub(crate) press: Option<ResolvedInteractionRoute>,
    pub(crate) release: Option<ResolvedInteractionRoute>,
    pub(crate) interaction: Option<MouseInteraction>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSensor {
    pub(crate) id: ViewId,
    pub(crate) show: Option<ResolvedInteractionRoute>,
    pub(crate) resize: Option<ResolvedInteractionRoute>,
    pub(crate) hide: Option<ResolvedInteractionRoute>,
    pub(crate) key: Option<CheckedExprUseId>,
    pub(crate) anticipate: Option<CheckedExprUseId>,
    pub(crate) delay_ms: Option<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInteractionRoute {
    pub(crate) id: InteractionRouteId,
    pub(crate) target: ResolvedInteractionRouteTarget,
    pub(crate) args: Vec<ResolvedInteractionRouteArg>,
    pub(crate) source_payloads: Vec<Type>,
    pub(crate) ordered_payloads: bool,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedInteractionRouteTarget {
    TargetHandler(HandlerId),
    OutputCallback {
        component: ComponentId,
        output: Type,
    },
    NamedEvent {
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedInteractionRouteArg {
    Expression(CheckedExprUseId),
    Payload { index: u32, ty: Type },
}

impl Lowerer {
    pub(super) fn lower_mouse_area(
        &mut self,
        options: &MouseAreaOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::MouseArea,
            crate::ast::mouse_area_semantic_key(options),
            span,
            outer_component,
        )?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        let expected_option_expressions = usize::from(options.interaction_expr.is_some());
        if checked.option_expressions.len() != expected_option_expressions {
            return Err(self.invariant(span, "mouse-area interaction expression presence diverged"));
        }
        if let Some(expression) = checked.option_expressions.first().copied() {
            let retained = self.facts.try_expression_use(expression).ok_or_else(|| {
                self.invariant(span, "mouse-area interaction expression is invalid")
            })?;
            if retained.destination != Type::MouseInteraction {
                return Err(self.invariant(span, "mouse-area interaction expression changed type"));
            }
        }
        let routes = crate::ast::mouse_area_routes(options);
        let mut route = 0usize;
        let mut take = |source: &Option<Route>| {
            self.lower_optional_interaction_route(source, &checked, &routes, &mut route, id, scope)
        };
        let resolved = ResolvedMouseArea {
            id,
            press: take(&options.press)?,
            release: take(&options.release)?,
            double_click: take(&options.double_click)?,
            right_press: take(&options.right_press)?,
            right_release: take(&options.right_release)?,
            middle_press: take(&options.middle_press)?,
            middle_release: take(&options.middle_release)?,
            enter: take(&options.enter)?,
            exit: take(&options.exit)?,
            move_route: take(&options.move_route)?,
            scroll: take(&options.scroll)?,
            interaction: options.interaction,
            interaction_expression: checked.option_expressions.first().copied(),
            origin,
        };
        if route != checked.routes.len() {
            return Err(self.invariant(span, "mouse-area left checked routes unconsumed"));
        }
        self.insert_interaction(
            id,
            ResolvedInteractionWidget::MouseArea(Box::new(resolved)),
            span,
        )
    }

    pub(super) fn lower_resize_handle(
        &mut self,
        options: &ResizeHandleOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::ResizeHandle,
            crate::ast::resize_handle_semantic_key(options),
            span,
            outer_component,
        )?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        if !checked.option_expressions.is_empty() {
            return Err(self.invariant(
                span,
                "resize-handle unexpectedly retained an interaction expression",
            ));
        }
        let routes = crate::ast::resize_handle_routes(options);
        let mut route = 0usize;
        let mut take = |source: &Option<Route>| {
            self.lower_optional_interaction_route(source, &checked, &routes, &mut route, id, scope)
        };
        let resolved = ResolvedResizeHandle {
            id,
            drag: take(&options.drag)?,
            press: take(&options.press)?,
            release: take(&options.release)?,
            interaction: options.interaction,
            origin,
        };
        if route != checked.routes.len() {
            return Err(self.invariant(span, "resize-handle left checked routes unconsumed"));
        }
        self.insert_interaction(
            id,
            ResolvedInteractionWidget::ResizeHandle(Box::new(resolved)),
            span,
        )
    }

    pub(super) fn lower_sensor(
        &mut self,
        options: &SensorOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Sensor,
            crate::ast::sensor_semantic_key(options),
            span,
            outer_component,
        )?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;

        let mut expressions = checked.option_expressions.iter().copied();
        let mut take_expression = |present: bool,
                                   expected: Option<&Type>,
                                   label: &str|
         -> Result<_, Error> {
            if !present {
                return Ok(None);
            }
            let expression = expressions.next().ok_or_else(|| {
                self.invariant(span, format!("sensor {label} expression disappeared"))
            })?;
            let retained = self.facts.try_expression_use(expression).ok_or_else(|| {
                self.invariant(span, format!("sensor {label} expression is invalid"))
            })?;
            if retained.destination != expected.unwrap_or(&retained.source).clone() {
                return Err(self.invariant(span, format!("sensor {label} expression changed type")));
            }
            Ok(Some(expression))
        };
        let key = take_expression(options.key.is_some(), None, "key")?;
        let anticipate =
            take_expression(options.anticipate.is_some(), Some(&Type::F64), "anticipate")?;
        let delay_ms = take_expression(options.delay_ms.is_some(), Some(&Type::I64), "delay")?;
        if let Some(key) = key {
            let ty = &self.facts.expression_use(key).source;
            if !matches!(
                ty,
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            ) {
                return Err(self.invariant(span, "sensor key retained an invalid checked type"));
            }
        }
        if expressions.next().is_some() {
            return Err(self.invariant(span, "sensor left checked option expressions unconsumed"));
        }

        let routes = crate::ast::sensor_routes(options);
        let mut route = 0usize;
        let mut take_route = |source: &Option<Route>| {
            self.lower_optional_interaction_route(source, &checked, &routes, &mut route, id, scope)
        };
        let resolved = ResolvedSensor {
            id,
            show: take_route(&options.show)?,
            resize: take_route(&options.resize)?,
            hide: take_route(&options.hide)?,
            key,
            anticipate,
            delay_ms,
            origin,
        };
        if route != checked.routes.len() {
            return Err(self.invariant(span, "sensor left checked routes unconsumed"));
        }
        self.insert_interaction(
            id,
            ResolvedInteractionWidget::Sensor(Box::new(resolved)),
            span,
        )
    }

    pub(super) fn interaction_contract(
        &self,
        kind: CheckedInteractionKind,
        semantic_key: String,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(ViewId, CheckedInteraction, CheckedViewScope, OriginId), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "interaction widget has no shared view ID"))?;
        let checked =
            self.facts.interaction(id).cloned().ok_or_else(|| {
                self.invariant(span, "interaction widget has no checked HIR facts")
            })?;
        let checked_view = self.facts.view(id);
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if checked.id != id
            || checked.kind != kind
            || checked.semantic_key != semantic_key
            || expected_scope != outer_component
        {
            return Err(self.invariant(
                span,
                "interaction widget topology diverged after semantic checking",
            ));
        }
        Ok((id, checked, checked_view.scope, checked_view.origin))
    }

    pub(super) fn lower_optional_interaction_route(
        &self,
        source: &Option<Route>,
        checked: &CheckedInteraction,
        routes: &[&Route],
        route: &mut usize,
        widget: ViewId,
        scope: CheckedViewScope,
    ) -> Result<Option<ResolvedInteractionRoute>, Error> {
        source
            .as_ref()
            .map(|source| {
                let expected = routes.get(*route).copied().ok_or_else(|| {
                    self.invariant(&source.span, "interaction route order is out of range")
                })?;
                if !std::ptr::eq(source, expected) {
                    return Err(self
                        .invariant(&source.span, "interaction route presence or order diverged"));
                }
                let result =
                    self.lower_interaction_route(source, checked, *route, widget, scope)?;
                *route += 1;
                Ok(result)
            })
            .transpose()
    }

    pub(super) fn lower_required_interaction_route(
        &self,
        source: &Route,
        checked: &CheckedInteraction,
        routes: &[&Route],
        route: &mut usize,
        widget: ViewId,
        scope: CheckedViewScope,
    ) -> Result<ResolvedInteractionRoute, Error> {
        let expected = routes.get(*route).copied().ok_or_else(|| {
            self.invariant(&source.span, "interaction route order is out of range")
        })?;
        if !std::ptr::eq(source, expected) {
            return Err(self.invariant(&source.span, "interaction route order diverged"));
        }
        let result = self.lower_interaction_route(source, checked, *route, widget, scope)?;
        *route += 1;
        Ok(result)
    }

    pub(super) fn lower_interaction_route(
        &self,
        source: &Route,
        interaction: &CheckedInteraction,
        route_index: usize,
        widget: ViewId,
        scope: CheckedViewScope,
    ) -> Result<ResolvedInteractionRoute, Error> {
        let checked = interaction.routes.get(route_index).ok_or_else(|| {
            self.invariant(&source.span, "interaction route has no checked contract")
        })?;
        if checked.id
            != (InteractionRouteId {
                widget,
                index: route_index as u32,
            })
        {
            return Err(self.invariant(&source.span, "interaction route ID diverged"));
        }
        let route_origin = self.origins.try_get(checked.origin).ok_or_else(|| {
            self.invariant(
                &source.span,
                "interaction route origin is outside its arena",
            )
        })?;
        if route_origin.parent != Some(self.facts.view(widget).origin) {
            return Err(self.invariant(
                &source.span,
                "interaction route origin has the wrong view parent",
            ));
        }
        let source_args = match &checked.target {
            CheckedCanvasRouteTarget::ComponentEvent { name, .. } => {
                let Some(RouteArg::Expr(Expr::Path(path))) = source.args.first() else {
                    return Err(self
                        .invariant(&source.span, "interaction named event selector disappeared"));
                };
                if path.len() != 1 || path[0] != *name {
                    return Err(
                        self.invariant(&source.span, "interaction named event selector diverged")
                    );
                }
                &source.args[1..]
            }
            _ => source.args.as_slice(),
        };
        if source_args.len() != checked.args.len() {
            return Err(self.invariant(&source.span, "interaction route arity diverged"));
        }
        let mut expected_expression_index = interaction.option_expressions.len() as u32
            + interaction.routes[..route_index]
                .iter()
                .flat_map(|route| &route.args)
                .filter(|argument| matches!(argument, CheckedCanvasRouteArg::Expression(_)))
                .count() as u32;
        let mut payload = 0u32;
        let mut args = Vec::with_capacity(checked.args.len());
        for (raw, retained) in source_args.iter().zip(&checked.args) {
            match (raw, retained) {
                (RouteArg::Expr(_), CheckedCanvasRouteArg::Expression(expression)) => {
                    let checked_expression =
                        self.facts.try_expression_use(*expression).ok_or_else(|| {
                            self.invariant(
                                &source.span,
                                "interaction route expression ID is invalid",
                            )
                        })?;
                    let expected_owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                        widget,
                        index: expected_expression_index,
                    });
                    expected_expression_index += 1;
                    if checked_expression.owner != expected_owner
                        || self.facts.expression_use_by_owner(expected_owner) != Some(*expression)
                    {
                        return Err(self.invariant(
                            &source.span,
                            "interaction route expression slot identity diverged",
                        ));
                    }
                    args.push(ResolvedInteractionRouteArg::Expression(*expression));
                }
                (RouteArg::Payload, CheckedCanvasRouteArg::Payload) => {
                    let index = if checked.ordered_payloads { payload } else { 0 };
                    let ty = checked
                        .source_payloads
                        .get(index as usize)
                        .cloned()
                        .ok_or_else(|| {
                            self.invariant(
                                &source.span,
                                "interaction route payload is out of range",
                            )
                        })?;
                    payload += 1;
                    args.push(ResolvedInteractionRouteArg::Payload { index, ty });
                }
                _ => {
                    return Err(
                        self.invariant(&source.span, "interaction route argument kind diverged")
                    );
                }
            }
        }
        let target = self.resolve_interaction_route_target(source, &checked.target, scope)?;
        Ok(ResolvedInteractionRoute {
            id: checked.id,
            target,
            args,
            source_payloads: checked.source_payloads.clone(),
            ordered_payloads: checked.ordered_payloads,
            origin: checked.origin,
        })
    }

    fn resolve_interaction_route_target(
        &self,
        source: &Route,
        checked: &CheckedCanvasRouteTarget,
        scope: CheckedViewScope,
    ) -> Result<ResolvedInteractionRouteTarget, Error> {
        Ok(match checked {
            CheckedCanvasRouteTarget::Handler(handler) => {
                let declaration = self.declarations.try_handler(*handler).ok_or_else(|| {
                    self.invariant(&source.span, "interaction route handler is invalid")
                })?;
                let expected_owner = match scope {
                    CheckedViewScope::Component(component) => HandlerOwner::Component(component),
                    CheckedViewScope::App | CheckedViewScope::Test(_) => HandlerOwner::App,
                };
                if declaration.owner != expected_owner || declaration.name != source.handler {
                    return Err(
                        self.invariant(&source.span, "interaction route handler contract diverged")
                    );
                }
                ResolvedInteractionRouteTarget::TargetHandler(*handler)
            }
            CheckedCanvasRouteTarget::ComponentOutput { component, output } => {
                if source.handler != "emit"
                    || !matches!(scope, CheckedViewScope::Component(owner) if owner == *component)
                    || self.declarations.component_output(*component) != Some(output)
                {
                    return Err(self.invariant(
                        &source.span,
                        "interaction component output contract diverged",
                    ));
                }
                ResolvedInteractionRouteTarget::OutputCallback {
                    component: *component,
                    output: output.clone(),
                }
            }
            CheckedCanvasRouteTarget::ComponentEvent {
                event,
                name,
                payloads,
            } => {
                let declaration = self.declarations.component_event(*event).ok_or_else(|| {
                    self.invariant(&source.span, "interaction component event is invalid")
                })?;
                if source.handler != "emit"
                    || !matches!(scope, CheckedViewScope::Component(component) if component == event.component)
                    || declaration.name != *name
                    || declaration.payloads != *payloads
                {
                    return Err(self.invariant(
                        &source.span,
                        "interaction component event contract diverged",
                    ));
                }
                ResolvedInteractionRouteTarget::NamedEvent {
                    event: *event,
                    name: name.clone(),
                    payloads: payloads.clone(),
                }
            }
        })
    }

    pub(super) fn validate_interaction_expression_graphs(
        &self,
        widget: ViewId,
        scope: CheckedViewScope,
        count: u32,
        span: &Span,
    ) -> Result<(), Error> {
        self.validate_interaction_expression_graphs_with_locals(widget, scope, count, None, span)
    }

    pub(super) fn validate_interaction_expression_graphs_with_local_contracts(
        &self,
        widget: ViewId,
        scope: CheckedViewScope,
        count: u32,
        allowed_locals: &HashMap<CheckedExprUseId, HashSet<CheckedLocalId>>,
        span: &Span,
    ) -> Result<(), Error> {
        self.validate_interaction_expression_graphs_with_locals(
            widget,
            scope,
            count,
            Some(allowed_locals),
            span,
        )
    }

    fn validate_interaction_expression_graphs_with_locals(
        &self,
        widget: ViewId,
        scope: CheckedViewScope,
        count: u32,
        allowed_locals: Option<&HashMap<CheckedExprUseId, HashSet<CheckedLocalId>>>,
        span: &Span,
    ) -> Result<(), Error> {
        let mut graph = CheckedExpressionGraph::default();
        for index in 0..count {
            let owner = CheckedExprOwner::Interaction(InteractionExpressionId { widget, index });
            let use_id = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "interaction expression has no checked owner mapping")
            })?;
            let expression = self.facts.try_expression_use(use_id).ok_or_else(|| {
                self.invariant(span, "interaction expression-use ID is outside its arena")
            })?;
            if expression.owner != owner {
                return Err(self.invariant(span, "interaction expression owner mapping diverged"));
            }
            let expression_allowed_locals = allowed_locals
                .map(|allowed| {
                    allowed.get(&use_id).ok_or_else(|| {
                        self.invariant(span, "interaction expression has no local-scope contract")
                    })
                })
                .transpose()?;
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: widget,
                scope,
                use_id,
                span,
                canvas_locals: false,
                own_view_locals: allowed_locals.is_some(),
                allowed_own_view_locals: expression_allowed_locals,
                family: "interaction",
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
                return Err(self.invariant(
                    span,
                    "interaction expression type or coercion contract diverged",
                ));
            }
        }
        Ok(())
    }

    fn insert_interaction(
        &mut self,
        id: ViewId,
        interaction: ResolvedInteractionWidget,
        span: &Span,
    ) -> Result<(), Error> {
        if interaction.id() != id || self.interaction_widgets.insert(id, interaction).is_some() {
            return Err(self.invariant(span, "interaction widget was lowered more than once"));
        }
        Ok(())
    }
}
