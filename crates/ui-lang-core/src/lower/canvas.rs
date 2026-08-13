use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvas {
    pub(crate) id: ViewId,
    pub(crate) options: ResolvedCanvasOptions,
    pub(crate) states: Vec<ResolvedCanvasState>,
    pub(crate) commands: Vec<ResolvedCanvasCommand>,
    pub(crate) events: Vec<ResolvedCanvasEvent>,
    pub(crate) width_local: CheckedLocalId,
    pub(crate) height_local: CheckedLocalId,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasState {
    #[cfg(test)]
    pub(crate) id: CanvasLocalId,
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) resolved_ty: ResolvedType,
    pub(crate) initializer: ResolvedInitializer,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasLength {
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed {
        expression: CheckedExprUseId,
        source: Type,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasOptions {
    pub(crate) width: Option<ResolvedCanvasLength>,
    pub(crate) height: Option<ResolvedCanvasLength>,
    pub(crate) cache: Option<CheckedExprUseId>,
    pub(crate) cache_group: Option<String>,
    pub(crate) capture: Option<CheckedExprUseId>,
    pub(crate) press: Option<ResolvedCanvasRoute>,
    pub(crate) release: Option<ResolvedCanvasRoute>,
    pub(crate) right_press: Option<ResolvedCanvasRoute>,
    pub(crate) right_release: Option<ResolvedCanvasRoute>,
    pub(crate) middle_press: Option<ResolvedCanvasRoute>,
    pub(crate) middle_release: Option<ResolvedCanvasRoute>,
    pub(crate) enter: Option<ResolvedCanvasRoute>,
    pub(crate) move_route: Option<ResolvedCanvasRoute>,
    pub(crate) scroll: Option<ResolvedCanvasRoute>,
    pub(crate) exit: Option<ResolvedCanvasRoute>,
    pub(crate) interaction: Option<MouseInteraction>,
    pub(crate) interaction_expr: Option<ResolvedCanvasInteraction>,
    pub(crate) interaction_outside: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasInteraction {
    pub(crate) expression: CheckedExprUseId,
    pub(crate) source: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasRoute {
    #[cfg(test)]
    pub(crate) id: CanvasRouteId,
    pub(crate) target: ResolvedCanvasRouteTarget,
    pub(crate) args: Vec<ResolvedCanvasRouteArg>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasRouteTarget {
    Handler(HandlerId),
    ComponentOutput { component: ComponentId },
    ComponentEvent { name: String, payloads: Vec<Type> },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasRouteArg {
    Expression(CheckedExprUseId),
    Payload { index: u32 },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasEvent {
    #[cfg(test)]
    pub(crate) id: CanvasEventId,
    pub(crate) source: ResolvedCanvasEventSource,
    pub(crate) bindings: Vec<ResolvedCanvasEventBinding>,
    pub(crate) updates: Vec<ResolvedCanvasStateUpdate>,
    pub(crate) action: Option<ResolvedCanvasEventAction>,
    pub(crate) capture: bool,
    pub(crate) route_payload: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedCanvasEventSource {
    InputMethod(InputMethodEvent),
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    Touch(TouchEvent),
    Window(WindowEvent),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasEventBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasStateUpdate {
    pub(crate) name: String,
    pub(crate) value: CheckedExprUseId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasEventAction {
    Route(ResolvedCanvasRoute),
    Redraw { after_ms: Option<u64> },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasCommand {
    Rectangle {
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        width: CheckedExprUseId,
        height: CheckedExprUseId,
        radius: ResolvedCanvasRadius,
        paint: ResolvedCanvasPaint,
    },
    Circle {
        #[cfg(test)]
        id: CanvasCommandId,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        radius: CheckedExprUseId,
        paint: ResolvedCanvasPaint,
    },
    Line {
        x1: CheckedExprUseId,
        y1: CheckedExprUseId,
        x2: CheckedExprUseId,
        y2: CheckedExprUseId,
        stroke: ResolvedCanvasStroke,
    },
    Text {
        #[cfg(test)]
        id: CanvasCommandId,
        value: CheckedExprUseId,
        value_type: Type,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        max_width: Option<CheckedExprUseId>,
        color: ResolvedThemeColor,
        size: Option<CheckedExprUseId>,
        line_height: Option<ResolvedCanvasLineHeight>,
        font: Option<ResolvedCanvasFont>,
        align_x: Option<TextAlignment>,
        align_y: Option<VerticalAlignment>,
        shaping: Option<TextShaping>,
    },
    Image {
        source: CheckedExprUseId,
        source_type: Type,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        width: CheckedExprUseId,
        height: CheckedExprUseId,
        filter: ImageFilter,
        rotation: CheckedExprUseId,
        opacity: CheckedExprUseId,
        snap: CheckedExprUseId,
        radius: ResolvedCanvasRadius,
    },
    Svg {
        source: CheckedExprUseId,
        source_type: Type,
        memory: bool,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        width: CheckedExprUseId,
        height: CheckedExprUseId,
        color: Option<ResolvedThemeColor>,
        rotation: CheckedExprUseId,
        opacity: CheckedExprUseId,
    },
    Path {
        segments: Vec<ResolvedCanvasPathSegment>,
        paint: ResolvedCanvasPaint,
    },
    Group {
        transform: ResolvedCanvasTransform,
        commands: Vec<ResolvedCanvasCommand>,
    },
    If {
        condition: CheckedExprUseId,
        commands: Vec<ResolvedCanvasCommand>,
    },
    For {
        #[cfg(test)]
        id: CanvasCommandId,
        item: ResolvedCanvasCommandItem,
        items: CheckedExprUseId,
        commands: Vec<ResolvedCanvasCommand>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasCommandItem {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedCanvasRadius {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) top_left: Option<CheckedExprUseId>,
    pub(crate) top_right: Option<CheckedExprUseId>,
    pub(crate) bottom_right: Option<CheckedExprUseId>,
    pub(crate) bottom_left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasPaint {
    pub(crate) fill: Option<ResolvedCanvasBackground>,
    pub(crate) fill_rule: CanvasFillRule,
    pub(crate) stroke: Option<ResolvedCanvasStroke>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasBackground {
    Color(ResolvedThemeColor),
    Linear {
        angle: CheckedExprUseId,
        stops: Vec<ResolvedCanvasGradientStop>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasGradientStop {
    pub(crate) color: ResolvedThemeColor,
    pub(crate) offset: CheckedExprUseId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCanvasStroke {
    pub(crate) style: ResolvedCanvasBackground,
    pub(crate) width: CheckedExprUseId,
    pub(crate) cap: CanvasLineCap,
    pub(crate) join: CanvasLineJoin,
    pub(crate) dash: Vec<CheckedExprUseId>,
    pub(crate) dash_offset: CheckedExprUseId,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedCanvasTransform {
    pub(crate) x: Option<CheckedExprUseId>,
    pub(crate) y: Option<CheckedExprUseId>,
    pub(crate) rotate: Option<CheckedExprUseId>,
    pub(crate) scale: Option<CheckedExprUseId>,
    pub(crate) scale_x: Option<CheckedExprUseId>,
    pub(crate) scale_y: Option<CheckedExprUseId>,
    pub(crate) clip: Option<[CheckedExprUseId; 4]>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasLineHeight {
    Relative(CheckedExprUseId),
    Absolute(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasFont {
    Default,
    Monospace,
    Custom(ResolvedDefaultFont),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedCanvasPathSegment {
    Move(CheckedExprUseId, CheckedExprUseId),
    Line(CheckedExprUseId, CheckedExprUseId),
    Arc {
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        radius: CheckedExprUseId,
        start: CheckedExprUseId,
        end: CheckedExprUseId,
    },
    ArcTo {
        ax: CheckedExprUseId,
        ay: CheckedExprUseId,
        bx: CheckedExprUseId,
        by: CheckedExprUseId,
        radius: CheckedExprUseId,
    },
    Ellipse {
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        radius_x: CheckedExprUseId,
        radius_y: CheckedExprUseId,
        rotation: CheckedExprUseId,
        start: CheckedExprUseId,
        end: CheckedExprUseId,
    },
    Bezier {
        control_ax: CheckedExprUseId,
        control_ay: CheckedExprUseId,
        control_bx: CheckedExprUseId,
        control_by: CheckedExprUseId,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
    },
    Quadratic {
        control_x: CheckedExprUseId,
        control_y: CheckedExprUseId,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
    },
    Rectangle {
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        width: CheckedExprUseId,
        height: CheckedExprUseId,
    },
    RoundedRectangle {
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        width: CheckedExprUseId,
        height: CheckedExprUseId,
        radius: ResolvedCanvasRadius,
    },
    Circle {
        x: CheckedExprUseId,
        y: CheckedExprUseId,
        radius: CheckedExprUseId,
    },
    Close,
}

struct CanvasOperands {
    values: std::vec::IntoIter<CheckedExprUseId>,
}

impl CanvasOperands {
    fn new(values: Vec<CheckedExprUseId>) -> Self {
        Self {
            values: values.into_iter(),
        }
    }

    fn take(&mut self, lowerer: &Lowerer, span: &Span) -> Result<CheckedExprUseId, Error> {
        self.values
            .next()
            .ok_or_else(|| lowerer.invariant(span, "canvas command exhausted its checked operands"))
    }

    fn optional<T>(
        &mut self,
        value: &Option<T>,
        lowerer: &Lowerer,
        span: &Span,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        value.as_ref().map(|_| self.take(lowerer, span)).transpose()
    }

    fn finish(mut self, lowerer: &Lowerer, span: &Span) -> Result<(), Error> {
        if self.values.next().is_some() {
            return Err(lowerer.invariant(span, "canvas command left checked operands unconsumed"));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_canvas(
        &mut self,
        options: &CanvasOptions,
        locals: &[State],
        commands: &[CanvasCommand],
        events: &[CanvasEvent],
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "canvas has no shared view ID"))?;
        let declaration = self
            .declarations
            .canvas(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "canvas has no stable declaration"))?;
        let checked = self
            .facts
            .canvas(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "canvas has no checked HIR facts"))?;
        let checked_view = self.facts.view(id);
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if checked.id != id
            || declaration.declaration.id != id
            || expected_scope != outer_component
            || declaration.locals.len() != locals.len()
            || declaration.commands.len() != crate::ast::canvas_command_spans(commands).len()
            || declaration.events.len() != events.len()
            || declaration.routes.len() != crate::ast::canvas_routes(options, events).len()
            || declaration.options_semantic_key != crate::ast::canvas_options_semantic_key(options)
            || checked.expression_count as usize
                != crate::ast::canvas_expression_roots(options, locals, commands, events).len()
        {
            return Err(self.invariant(span, "canvas topology diverged after semantic checking"));
        }
        self.validate_canvas_expression_graphs(
            id,
            checked_view.scope,
            checked.expression_count,
            span,
        )?;

        let mut expression = 0u32;
        let mut route = 0usize;
        let width = self.lower_canvas_length(&options.width, id, &mut expression, span)?;
        let height = self.lower_canvas_length(&options.height, id, &mut expression, span)?;
        let cache = options
            .cache
            .as_ref()
            .map(|_| self.take_canvas_expression(id, &mut expression, span))
            .transpose()?;
        let capture = options
            .capture
            .as_ref()
            .map(|_| self.take_canvas_expression(id, &mut expression, span))
            .transpose()?;

        let press = self.lower_optional_canvas_route(
            &options.press,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let release = self.lower_optional_canvas_route(
            &options.release,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let right_press = self.lower_optional_canvas_route(
            &options.right_press,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let right_release = self.lower_optional_canvas_route(
            &options.right_release,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let middle_press = self.lower_optional_canvas_route(
            &options.middle_press,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let middle_release = self.lower_optional_canvas_route(
            &options.middle_release,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let enter = self.lower_optional_canvas_route(
            &options.enter,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let move_route = self.lower_optional_canvas_route(
            &options.move_route,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let scroll = self.lower_optional_canvas_route(
            &options.scroll,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;
        let exit = self.lower_optional_canvas_route(
            &options.exit,
            &checked,
            &mut route,
            id,
            &mut expression,
            span,
        )?;

        let mut states = Vec::with_capacity(locals.len());
        for (index, source) in locals.iter().enumerate() {
            let retained = declaration
                .locals
                .get(index)
                .ok_or_else(|| self.invariant(&source.span, "canvas state has no declaration"))?;
            let state_id = CanvasLocalId {
                canvas: id,
                index: index as u32,
            };
            let local =
                checked.states.get(index).copied().ok_or_else(|| {
                    self.invariant(&source.span, "canvas state has no checked local")
                })?;
            let fact = self.facts.try_local(local).ok_or_else(|| {
                self.invariant(&source.span, "canvas state local is outside its arena")
            })?;
            if retained.declaration.id != state_id
                || retained.name != source.name
                || retained.ty != source.ty
                || fact.owner != CheckedLocalOwner::CanvasState(state_id)
                || fact.name != retained.name
                || fact.ty != retained.ty
            {
                return Err(self.invariant(&source.span, "canvas state contract diverged"));
            }
            states.push(ResolvedCanvasState {
                #[cfg(test)]
                id: state_id,
                local,
                name: retained.name.clone(),
                ty: retained.ty.clone(),
                resolved_ty: self.resolve_type(&retained.ty, &source.span)?,
                initializer: ResolvedInitializer {
                    expression: self.take_canvas_expression(id, &mut expression, &source.span)?,
                    animation: None,
                },
                origin: retained.declaration.origin,
            });
        }
        self.validate_canvas_builtin_local(
            checked.width,
            CheckedLocalOwner::CanvasWidth(id),
            span,
        )?;
        self.validate_canvas_builtin_local(
            checked.height,
            CheckedLocalOwner::CanvasHeight(id),
            span,
        )?;

        let interaction_expr = options
            .interaction_expr
            .as_ref()
            .map(|_| {
                let expression = self.take_canvas_expression(id, &mut expression, span)?;
                let source = self.facts.expression_use(expression).source.clone();
                Ok(ResolvedCanvasInteraction { expression, source })
            })
            .transpose()?;
        let interaction_outside = options
            .interaction_outside
            .as_ref()
            .map(|_| self.take_canvas_expression(id, &mut expression, span))
            .transpose()?;

        let mut command = 0u32;
        let commands =
            self.lower_canvas_commands(commands, id, &declaration, &mut command, &mut expression)?;
        let events = events
            .iter()
            .enumerate()
            .map(|(index, event)| {
                self.lower_canvas_event(
                    event,
                    index,
                    id,
                    &declaration,
                    &checked,
                    &states,
                    &mut route,
                    &mut expression,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        if expression != checked.expression_count
            || command as usize != declaration.commands.len()
            || route != checked.routes.len()
        {
            return Err(self.invariant(span, "canvas lowering did not consume its checked arenas"));
        }
        let resolved = ResolvedCanvas {
            id,
            options: ResolvedCanvasOptions {
                width,
                height,
                cache,
                cache_group: options.cache_group.clone(),
                capture,
                press,
                release,
                right_press,
                right_release,
                middle_press,
                middle_release,
                enter,
                move_route,
                scroll,
                exit,
                interaction: options.interaction,
                interaction_expr,
                interaction_outside,
            },
            states,
            commands,
            events,
            width_local: checked.width,
            height_local: checked.height,
            origin: declaration.declaration.origin,
        };
        if self.canvases.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "canvas was lowered more than once"));
        }
        Ok(())
    }

    fn lower_canvas_length(
        &self,
        value: &Option<LengthValue>,
        canvas: ViewId,
        expression: &mut u32,
        span: &Span,
    ) -> Result<Option<ResolvedCanvasLength>, Error> {
        value
            .as_ref()
            .map(|value| {
                Ok(match value {
                    LengthValue::Fill => ResolvedCanvasLength::Fill,
                    LengthValue::FillPortion(value) => ResolvedCanvasLength::FillPortion(*value),
                    LengthValue::Shrink => ResolvedCanvasLength::Shrink,
                    LengthValue::Fixed(_) => {
                        let expression = self.take_canvas_expression(canvas, expression, span)?;
                        ResolvedCanvasLength::Fixed {
                            expression,
                            source: self.facts.expression_use(expression).source.clone(),
                        }
                    }
                })
            })
            .transpose()
    }

    fn validate_canvas_expression_graphs(
        &self,
        canvas: ViewId,
        scope: CheckedViewScope,
        count: u32,
        span: &Span,
    ) -> Result<(), Error> {
        let mut graph = CheckedExpressionGraph::default();
        for index in 0..count {
            let owner = CheckedExprOwner::Canvas(CanvasExpressionId { canvas, index });
            let use_id = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "canvas expression has no checked owner mapping")
            })?;
            let expression = self.facts.try_expression_use(use_id).ok_or_else(|| {
                self.invariant(span, "canvas expression-use ID is outside its arena")
            })?;
            if expression.owner != owner {
                return Err(self.invariant(span, "canvas expression owner mapping diverged"));
            }
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: canvas,
                scope,
                use_id,
                span,
                canvas_locals: true,
                own_view_locals: false,
                allowed_own_view_locals: None,
                family: "canvas",
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
                    self.invariant(span, "canvas expression type or coercion contract diverged")
                );
            }
        }
        Ok(())
    }

    fn take_canvas_expression(
        &self,
        canvas: ViewId,
        index: &mut u32,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Canvas(CanvasExpressionId {
            canvas,
            index: *index,
        });
        *index += 1;
        let expression = self
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| self.invariant(span, "canvas expression has no checked owner"))?;
        let retained = self
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| self.invariant(span, "canvas expression-use ID is outside its arena"))?;
        if retained.owner != owner || self.facts.try_expression(retained.root).is_none() {
            return Err(self.invariant(span, "canvas expression graph has an invalid owner"));
        }
        Ok(expression)
    }

    fn validate_canvas_builtin_local(
        &self,
        local: CheckedLocalId,
        owner: CheckedLocalOwner,
        span: &Span,
    ) -> Result<(), Error> {
        let retained = self
            .facts
            .try_local(local)
            .ok_or_else(|| self.invariant(span, "canvas built-in local is outside its arena"))?;
        if retained.owner != owner || retained.ty != Type::F64 {
            return Err(self.invariant(span, "canvas built-in local contract diverged"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_optional_canvas_route(
        &self,
        source: &Option<Route>,
        canvas: &CheckedCanvas,
        route: &mut usize,
        canvas_id: ViewId,
        expression: &mut u32,
        span: &Span,
    ) -> Result<Option<ResolvedCanvasRoute>, Error> {
        source
            .as_ref()
            .map(|source| {
                self.lower_canvas_route(source, canvas, route, canvas_id, expression, span)
            })
            .transpose()
    }

    fn lower_canvas_route(
        &self,
        source: &Route,
        canvas: &CheckedCanvas,
        route_index: &mut usize,
        canvas_id: ViewId,
        expression: &mut u32,
        span: &Span,
    ) -> Result<ResolvedCanvasRoute, Error> {
        let checked = canvas
            .routes
            .get(*route_index)
            .ok_or_else(|| self.invariant(&source.span, "canvas route has no checked contract"))?;
        *route_index += 1;
        let source_args = if matches!(
            checked.target,
            CheckedCanvasRouteTarget::ComponentEvent { .. }
        ) {
            source.args.get(1..).unwrap_or_default()
        } else {
            source.args.as_slice()
        };
        if checked.id.canvas != canvas_id || checked.args.len() != source_args.len() {
            return Err(self.invariant(&source.span, "canvas route topology diverged"));
        }
        let mut payload = 0u32;
        let mut args = Vec::with_capacity(checked.args.len());
        for (raw, retained) in source_args.iter().zip(&checked.args) {
            match (raw, retained) {
                (RouteArg::Expr(_), CheckedCanvasRouteArg::Expression(expected)) => {
                    let actual = self.take_canvas_expression(canvas_id, expression, span)?;
                    if actual != *expected {
                        return Err(self.invariant(
                            &source.span,
                            "canvas route expression owner order diverged",
                        ));
                    }
                    args.push(ResolvedCanvasRouteArg::Expression(actual));
                }
                (RouteArg::Payload, CheckedCanvasRouteArg::Payload) => {
                    let index = if checked.ordered_payloads { payload } else { 0 };
                    checked.source_payloads.get(index as usize).ok_or_else(|| {
                        self.invariant(&source.span, "canvas route payload is out of range")
                    })?;
                    payload += 1;
                    args.push(ResolvedCanvasRouteArg::Payload { index });
                }
                _ => {
                    return Err(self.invariant(&source.span, "canvas route argument kind diverged"));
                }
            }
        }
        let target = match &checked.target {
            CheckedCanvasRouteTarget::Handler(handler) => {
                if self.declarations.try_handler(*handler).is_none() {
                    return Err(self.invariant(&source.span, "canvas route handler is invalid"));
                }
                ResolvedCanvasRouteTarget::Handler(*handler)
            }
            CheckedCanvasRouteTarget::ComponentOutput { component, .. } => {
                ResolvedCanvasRouteTarget::ComponentOutput {
                    component: *component,
                }
            }
            CheckedCanvasRouteTarget::ComponentEvent { name, payloads, .. } => {
                ResolvedCanvasRouteTarget::ComponentEvent {
                    name: name.clone(),
                    payloads: payloads.clone(),
                }
            }
        };
        Ok(ResolvedCanvasRoute {
            #[cfg(test)]
            id: checked.id,
            target,
            args,
            origin: checked.origin,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_canvas_event(
        &self,
        event: &CanvasEvent,
        index: usize,
        canvas: ViewId,
        declaration: &CanvasDeclaration,
        checked: &CheckedCanvas,
        states: &[ResolvedCanvasState],
        route: &mut usize,
        expression: &mut u32,
    ) -> Result<ResolvedCanvasEvent, Error> {
        let id = CanvasEventId {
            canvas,
            index: index as u32,
        };
        let retained = declaration
            .events
            .get(index)
            .ok_or_else(|| self.invariant(&event.span, "canvas event has no declaration"))?;
        if retained.declaration.id != id
            || retained.semantic_key != crate::ast::canvas_event_semantic_key(event)
        {
            return Err(self.invariant(&event.span, "canvas event identity diverged"));
        }
        let payloads = crate::check::native_subscription_payloads(&event.source, false)
            .ok_or_else(|| self.invariant(&event.span, "canvas event source is not native"))?;
        let source = match &event.source {
            SubscriptionSource::InputMethod(event) => {
                ResolvedCanvasEventSource::InputMethod(*event)
            }
            SubscriptionSource::Keyboard(event) => {
                ResolvedCanvasEventSource::Keyboard(event.clone())
            }
            SubscriptionSource::Mouse(event) => ResolvedCanvasEventSource::Mouse(*event),
            SubscriptionSource::Touch(event) => ResolvedCanvasEventSource::Touch(*event),
            SubscriptionSource::Window(event) => ResolvedCanvasEventSource::Window(*event),
            _ => return Err(self.invariant(&event.span, "canvas event source is not native")),
        };
        if event.bindings.len() > payloads.len() {
            return Err(self.invariant(&event.span, "canvas event binding arity diverged"));
        }
        let bindings = event
            .bindings
            .iter()
            .zip(payloads)
            .enumerate()
            .map(|(binding, (name, ty))| {
                let owner = CheckedLocalOwner::CanvasEventBinding {
                    event: id,
                    index: binding as u32,
                };
                let local = self.facts.local_by_owner(owner).ok_or_else(|| {
                    self.invariant(&event.span, "canvas event binding has no checked local")
                })?;
                let fact = self.facts.try_local(local).ok_or_else(|| {
                    self.invariant(&event.span, "canvas event binding local is invalid")
                })?;
                if fact.name != *name || fact.ty != ty {
                    return Err(
                        self.invariant(&event.span, "canvas event binding contract diverged")
                    );
                }
                Ok(ResolvedCanvasEventBinding {
                    local,
                    name: name.clone(),
                    ty,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let updates = event
            .updates
            .iter()
            .map(|update| {
                let target = states
                    .iter()
                    .find(|state| state.name == update.name)
                    .ok_or_else(|| {
                        self.invariant(&update.span, "canvas update target disappeared")
                    })?;
                Ok(ResolvedCanvasStateUpdate {
                    name: target.name.clone(),
                    value: self.take_canvas_expression(canvas, expression, &update.span)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let action = match &event.action {
            Some(CanvasEventAction::Route(source)) => Some(ResolvedCanvasEventAction::Route(
                self.lower_canvas_route(source, checked, route, canvas, expression, &event.span)?,
            )),
            Some(CanvasEventAction::Redraw { after_ms }) => {
                Some(ResolvedCanvasEventAction::Redraw {
                    after_ms: *after_ms,
                })
            }
            None => None,
        };
        Ok(ResolvedCanvasEvent {
            #[cfg(test)]
            id,
            source,
            bindings,
            updates,
            action,
            capture: event.capture,
            route_payload: event.route_payload,
        })
    }

    fn lower_canvas_commands(
        &self,
        commands: &[CanvasCommand],
        canvas: ViewId,
        declaration: &CanvasDeclaration,
        command: &mut u32,
        expression: &mut u32,
    ) -> Result<Vec<ResolvedCanvasCommand>, Error> {
        commands
            .iter()
            .map(|source| {
                let id = CanvasCommandId {
                    canvas,
                    index: *command,
                };
                *command += 1;
                let retained = declaration
                    .commands
                    .get(id.index as usize)
                    .filter(|retained| {
                        retained.declaration.id == id
                            && retained.semantic_key
                                == crate::ast::canvas_command_semantic_key(source)
                    })
                    .ok_or_else(|| {
                        self.invariant(
                            crate::ast::canvas_command_span(source),
                            "canvas command has no declaration",
                        )
                    })?;
                let span = crate::ast::canvas_command_span(source);
                let values = crate::ast::canvas_command_direct_expression_roots(source)
                    .into_iter()
                    .map(|_| self.take_canvas_expression(canvas, expression, span))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut operands = CanvasOperands::new(values);
                let resolved = self.lower_canvas_command(
                    source,
                    id,
                    retained.declaration.origin,
                    canvas,
                    declaration,
                    command,
                    expression,
                    &mut operands,
                )?;
                operands.finish(self, span)?;
                Ok(resolved)
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_canvas_command(
        &self,
        source: &CanvasCommand,
        id: CanvasCommandId,
        origin: OriginId,
        canvas: ViewId,
        declaration: &CanvasDeclaration,
        command: &mut u32,
        expression: &mut u32,
        operands: &mut CanvasOperands,
    ) -> Result<ResolvedCanvasCommand, Error> {
        let span = crate::ast::canvas_command_span(source);
        let take = |operands: &mut CanvasOperands| operands.take(self, span);
        Ok(match source {
            CanvasCommand::Rectangle { radius, paint, .. } => ResolvedCanvasCommand::Rectangle {
                x: take(operands)?,
                y: take(operands)?,
                width: take(operands)?,
                height: take(operands)?,
                radius: self.lower_canvas_radius(radius, operands, span)?,
                paint: self.lower_canvas_paint(paint, operands, span)?,
            },
            CanvasCommand::Circle { paint, .. } => ResolvedCanvasCommand::Circle {
                #[cfg(test)]
                id,
                x: take(operands)?,
                y: take(operands)?,
                radius: take(operands)?,
                paint: self.lower_canvas_paint(paint, operands, span)?,
            },
            CanvasCommand::Line { stroke, .. } => ResolvedCanvasCommand::Line {
                x1: take(operands)?,
                y1: take(operands)?,
                x2: take(operands)?,
                y2: take(operands)?,
                stroke: self.lower_canvas_stroke(stroke, operands, span)?,
            },
            CanvasCommand::Text {
                max_width,
                color,
                size,
                line_height,
                font,
                align_x,
                align_y,
                shaping,
                ..
            } => {
                let value = take(operands)?;
                ResolvedCanvasCommand::Text {
                    #[cfg(test)]
                    id,
                    value,
                    value_type: self.facts.expression_use(value).source.clone(),
                    x: take(operands)?,
                    y: take(operands)?,
                    max_width: operands.optional(max_width, self, span)?,
                    color: self.resolve_theme_color(color.as_deref().unwrap_or("fg"), span)?,
                    size: operands.optional(size, self, span)?,
                    line_height: line_height
                        .as_ref()
                        .map(|height| {
                            Ok(match height {
                                TextLineHeight::Relative(_) => {
                                    ResolvedCanvasLineHeight::Relative(take(operands)?)
                                }
                                TextLineHeight::Absolute(_) => {
                                    ResolvedCanvasLineHeight::Absolute(take(operands)?)
                                }
                            })
                        })
                        .transpose()?,
                    font: font
                        .as_ref()
                        .map(|font| self.lower_canvas_font(font, origin, span))
                        .transpose()?,
                    align_x: *align_x,
                    align_y: *align_y,
                    shaping: *shaping,
                }
            }
            CanvasCommand::Image { filter, radius, .. } => {
                let source = take(operands)?;
                ResolvedCanvasCommand::Image {
                    source,
                    source_type: self.facts.expression_use(source).source.clone(),
                    x: take(operands)?,
                    y: take(operands)?,
                    width: take(operands)?,
                    height: take(operands)?,
                    filter: *filter,
                    rotation: take(operands)?,
                    opacity: take(operands)?,
                    snap: take(operands)?,
                    radius: self.lower_canvas_radius(radius, operands, span)?,
                }
            }
            CanvasCommand::Svg { memory, color, .. } => {
                let source = take(operands)?;
                ResolvedCanvasCommand::Svg {
                    source,
                    source_type: self.facts.expression_use(source).source.clone(),
                    memory: *memory,
                    x: take(operands)?,
                    y: take(operands)?,
                    width: take(operands)?,
                    height: take(operands)?,
                    color: color
                        .as_deref()
                        .map(|color| self.resolve_theme_color(color, span))
                        .transpose()?,
                    rotation: take(operands)?,
                    opacity: take(operands)?,
                }
            }
            CanvasCommand::Path {
                segments, paint, ..
            } => ResolvedCanvasCommand::Path {
                segments: self.lower_canvas_path(segments, operands, span)?,
                paint: self.lower_canvas_paint(paint, operands, span)?,
            },
            CanvasCommand::Group {
                transform,
                commands,
                ..
            } => ResolvedCanvasCommand::Group {
                transform: self.lower_canvas_transform(transform, operands, span)?,
                commands: self.lower_canvas_commands(
                    commands,
                    canvas,
                    declaration,
                    command,
                    expression,
                )?,
            },
            CanvasCommand::If { commands, .. } => ResolvedCanvasCommand::If {
                condition: take(operands)?,
                commands: self.lower_canvas_commands(
                    commands,
                    canvas,
                    declaration,
                    command,
                    expression,
                )?,
            },
            CanvasCommand::For { item, commands, .. } => {
                let items = take(operands)?;
                let local = self
                    .facts
                    .local_by_owner(CheckedLocalOwner::CanvasCommandItem(id))
                    .ok_or_else(|| self.invariant(span, "canvas for item has no checked local"))?;
                let fact = self.facts.try_local(local).ok_or_else(|| {
                    self.invariant(span, "canvas for item local is outside its arena")
                })?;
                let Type::List(inner) = &self.facts.expression_use(items).source else {
                    return Err(self.invariant(span, "canvas for items are not a checked list"));
                };
                if fact.name != *item || fact.ty != **inner {
                    return Err(self.invariant(span, "canvas for item contract diverged"));
                }
                ResolvedCanvasCommand::For {
                    #[cfg(test)]
                    id,
                    item: ResolvedCanvasCommandItem {
                        local,
                        name: item.clone(),
                        ty: fact.ty.clone(),
                    },
                    items,
                    commands: self.lower_canvas_commands(
                        commands,
                        canvas,
                        declaration,
                        command,
                        expression,
                    )?,
                }
            }
        })
    }

    fn lower_canvas_radius(
        &self,
        source: &CanvasRadius,
        operands: &mut CanvasOperands,
        span: &Span,
    ) -> Result<ResolvedCanvasRadius, Error> {
        Ok(ResolvedCanvasRadius {
            all: operands.optional(&source.all, self, span)?,
            top_left: operands.optional(&source.top_left, self, span)?,
            top_right: operands.optional(&source.top_right, self, span)?,
            bottom_right: operands.optional(&source.bottom_right, self, span)?,
            bottom_left: operands.optional(&source.bottom_left, self, span)?,
        })
    }

    fn lower_canvas_paint(
        &self,
        source: &CanvasPaint,
        operands: &mut CanvasOperands,
        span: &Span,
    ) -> Result<ResolvedCanvasPaint, Error> {
        Ok(ResolvedCanvasPaint {
            fill: source
                .fill
                .as_ref()
                .map(|fill| self.lower_canvas_background(fill, operands, span))
                .transpose()?,
            fill_rule: source.fill_rule,
            stroke: source
                .stroke
                .as_ref()
                .map(|stroke| self.lower_canvas_stroke(stroke, operands, span))
                .transpose()?,
        })
    }

    fn lower_canvas_background(
        &self,
        source: &BackgroundValue,
        operands: &mut CanvasOperands,
        span: &Span,
    ) -> Result<ResolvedCanvasBackground, Error> {
        Ok(match source {
            BackgroundValue::Color(color) => {
                ResolvedCanvasBackground::Color(self.resolve_theme_color(color, span)?)
            }
            BackgroundValue::Linear { stops, .. } => ResolvedCanvasBackground::Linear {
                angle: operands.take(self, span)?,
                stops: stops
                    .iter()
                    .map(|stop| {
                        Ok(ResolvedCanvasGradientStop {
                            color: self.resolve_theme_color(&stop.color, span)?,
                            offset: operands.take(self, span)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?,
            },
        })
    }

    fn lower_canvas_font(
        &self,
        source: &FontPreset,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedCanvasFont, Error> {
        Ok(match source {
            FontPreset::Default => ResolvedCanvasFont::Default,
            FontPreset::Monospace => ResolvedCanvasFont::Monospace,
            FontPreset::Named(name) => {
                let font = self
                    .document
                    .fonts
                    .iter()
                    .find(|font| font.name == *name)
                    .ok_or_else(|| {
                        self.invariant(span, format!("unknown checked font preset `{name}`"))
                    })?;
                ResolvedCanvasFont::Custom(ResolvedDefaultFont {
                    family: font.family.clone(),
                    weight: font.weight,
                    stretch: font.stretch,
                    style: font.style,
                    origin,
                })
            }
        })
    }

    fn lower_canvas_stroke(
        &self,
        source: &CanvasStroke,
        operands: &mut CanvasOperands,
        span: &Span,
    ) -> Result<ResolvedCanvasStroke, Error> {
        Ok(ResolvedCanvasStroke {
            style: self.lower_canvas_background(&source.style, operands, span)?,
            width: operands.take(self, span)?,
            cap: source.cap,
            join: source.join,
            dash: source
                .dash
                .iter()
                .map(|_| operands.take(self, span))
                .collect::<Result<Vec<_>, _>>()?,
            dash_offset: operands.take(self, span)?,
        })
    }

    fn lower_canvas_transform(
        &self,
        source: &CanvasTransform,
        operands: &mut CanvasOperands,
        span: &Span,
    ) -> Result<ResolvedCanvasTransform, Error> {
        let x = operands.optional(&source.x, self, span)?;
        let y = operands.optional(&source.y, self, span)?;
        let rotate = operands.optional(&source.rotate, self, span)?;
        let scale = operands.optional(&source.scale, self, span)?;
        let scale_x = operands.optional(&source.scale_x, self, span)?;
        let scale_y = operands.optional(&source.scale_y, self, span)?;
        let clip = source
            .clip
            .as_ref()
            .map(|_| {
                Ok([
                    operands.take(self, span)?,
                    operands.take(self, span)?,
                    operands.take(self, span)?,
                    operands.take(self, span)?,
                ])
            })
            .transpose()?;
        Ok(ResolvedCanvasTransform {
            x,
            y,
            rotate,
            scale,
            scale_x,
            scale_y,
            clip,
        })
    }

    fn lower_canvas_path(
        &self,
        segments: &[CanvasPathSegment],
        operands: &mut CanvasOperands,
        span: &Span,
    ) -> Result<Vec<ResolvedCanvasPathSegment>, Error> {
        let take = |operands: &mut CanvasOperands| operands.take(self, span);
        segments
            .iter()
            .map(|segment| {
                Ok(match segment {
                    CanvasPathSegment::Move(_, _) => {
                        ResolvedCanvasPathSegment::Move(take(operands)?, take(operands)?)
                    }
                    CanvasPathSegment::Line(_, _) => {
                        ResolvedCanvasPathSegment::Line(take(operands)?, take(operands)?)
                    }
                    CanvasPathSegment::Arc { .. } => ResolvedCanvasPathSegment::Arc {
                        x: take(operands)?,
                        y: take(operands)?,
                        radius: take(operands)?,
                        start: take(operands)?,
                        end: take(operands)?,
                    },
                    CanvasPathSegment::ArcTo { .. } => ResolvedCanvasPathSegment::ArcTo {
                        ax: take(operands)?,
                        ay: take(operands)?,
                        bx: take(operands)?,
                        by: take(operands)?,
                        radius: take(operands)?,
                    },
                    CanvasPathSegment::Ellipse { .. } => ResolvedCanvasPathSegment::Ellipse {
                        x: take(operands)?,
                        y: take(operands)?,
                        radius_x: take(operands)?,
                        radius_y: take(operands)?,
                        rotation: take(operands)?,
                        start: take(operands)?,
                        end: take(operands)?,
                    },
                    CanvasPathSegment::Bezier { .. } => ResolvedCanvasPathSegment::Bezier {
                        control_ax: take(operands)?,
                        control_ay: take(operands)?,
                        control_bx: take(operands)?,
                        control_by: take(operands)?,
                        x: take(operands)?,
                        y: take(operands)?,
                    },
                    CanvasPathSegment::Quadratic { .. } => ResolvedCanvasPathSegment::Quadratic {
                        control_x: take(operands)?,
                        control_y: take(operands)?,
                        x: take(operands)?,
                        y: take(operands)?,
                    },
                    CanvasPathSegment::Rectangle { .. } => ResolvedCanvasPathSegment::Rectangle {
                        x: take(operands)?,
                        y: take(operands)?,
                        width: take(operands)?,
                        height: take(operands)?,
                    },
                    CanvasPathSegment::RoundedRectangle { radius, .. } => {
                        ResolvedCanvasPathSegment::RoundedRectangle {
                            x: take(operands)?,
                            y: take(operands)?,
                            width: take(operands)?,
                            height: take(operands)?,
                            radius: self.lower_canvas_radius(radius, operands, span)?,
                        }
                    }
                    CanvasPathSegment::Circle { .. } => ResolvedCanvasPathSegment::Circle {
                        x: take(operands)?,
                        y: take(operands)?,
                        radius: take(operands)?,
                    },
                    CanvasPathSegment::Close => ResolvedCanvasPathSegment::Close,
                })
            })
            .collect()
    }
}
