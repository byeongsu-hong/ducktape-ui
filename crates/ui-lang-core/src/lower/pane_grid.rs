use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedPaneAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedPaneConfiguration {
    Pane(String),
    Split {
        name: Option<String>,
        axis: ResolvedPaneAxis,
        ratio: f32,
        a: Box<ResolvedPaneConfiguration>,
        b: Box<ResolvedPaneConfiguration>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedPaneLength {
    Fill,
    FillPortion(u16),
    Shrink,
    FixedF64(CheckedExprUseId),
    FixedLength(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneGradientStop {
    pub(crate) color: ResolvedThemeColor,
    pub(crate) offset: CheckedExprUseId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedPaneBackground {
    Color(ResolvedThemeColor),
    Linear {
        angle: CheckedExprUseId,
        stops: Vec<ResolvedPaneGradientStop>,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedPaneRadius {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) top_left: Option<CheckedExprUseId>,
    pub(crate) top_right: Option<CheckedExprUseId>,
    pub(crate) bottom_right: Option<CheckedExprUseId>,
    pub(crate) bottom_left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedPaneSurface {
    pub(crate) background: Option<ResolvedPaneBackground>,
    pub(crate) text_color: Option<ResolvedThemeColor>,
    pub(crate) border_color: Option<ResolvedThemeColor>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedPaneRadius,
    pub(crate) shadow_color: Option<ResolvedThemeColor>,
    pub(crate) shadow_x: Option<CheckedExprUseId>,
    pub(crate) shadow_y: Option<CheckedExprUseId>,
    pub(crate) shadow_blur: Option<CheckedExprUseId>,
    pub(crate) pixel_snap: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedPanePadding {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) x: Option<CheckedExprUseId>,
    pub(crate) y: Option<CheckedExprUseId>,
    pub(crate) top: Option<CheckedExprUseId>,
    pub(crate) right: Option<CheckedExprUseId>,
    pub(crate) bottom: Option<CheckedExprUseId>,
    pub(crate) left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneTitle {
    pub(crate) padding: ResolvedPanePadding,
    pub(crate) always_show_controls: bool,
    pub(crate) has_controls: bool,
    pub(crate) has_compact_controls: bool,
    pub(crate) surface: ResolvedPaneSurface,
    pub(crate) utility_style: ResolvedStyle,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneView {
    pub(crate) name: String,
    pub(crate) maximized: Option<ResolvedPaneBinding>,
    pub(crate) surface: ResolvedPaneSurface,
    pub(crate) utility_style: ResolvedStyle,
    pub(crate) title: Option<ResolvedPaneTitle>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedPaneItems {
    Value(CheckedValueRef),
    Local(CheckedLocalId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneTemplate {
    pub(crate) items: ResolvedPaneItems,
    pub(crate) item: ResolvedPaneBinding,
    pub(crate) key: CheckedExprUseId,
    pub(crate) key_type: Type,
    pub(crate) pane: ResolvedPaneView,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneGridStyle {
    pub(crate) region_background: Option<ResolvedPaneBackground>,
    pub(crate) region_border: Option<ResolvedThemeColor>,
    pub(crate) region_border_width: Option<CheckedExprUseId>,
    pub(crate) region_radius: ResolvedPaneRadius,
    pub(crate) hovered_split: Option<ResolvedThemeColor>,
    pub(crate) hovered_split_width: Option<CheckedExprUseId>,
    pub(crate) picked_split: Option<ResolvedThemeColor>,
    pub(crate) picked_split_width: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneGrid {
    pub(crate) id: ViewId,
    pub(crate) name: String,
    pub(crate) configuration: ResolvedPaneConfiguration,
    pub(crate) width: Option<ResolvedPaneLength>,
    pub(crate) height: Option<ResolvedPaneLength>,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) min_size: Option<CheckedExprUseId>,
    pub(crate) resize_leeway: Option<CheckedExprUseId>,
    pub(crate) draggable: bool,
    pub(crate) click: Option<ResolvedInteractionRoute>,
    pub(crate) custom_style: Option<ResolvedPaneCustomStyle>,
    pub(crate) style: ResolvedPaneGridStyle,
    pub(crate) panes: Vec<ResolvedPaneView>,
    pub(crate) templates: Vec<ResolvedPaneTemplate>,
    pub(crate) test_scope: bool,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_pane_grid(
        &mut self,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "pane grid has no shared view ID"))?;
        let checked = self
            .facts
            .pane_grid(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "pane grid has no checked HIR facts"))?;
        let checked_view = self.facts.view(id);
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if checked.id != id
            || checked.origin != checked_view.origin
            || expected_scope != outer_component
        {
            return Err(self.invariant(span, "pane grid checked topology diverged"));
        }

        let local_contracts = self.pane_local_contracts(&checked, span)?;
        self.validate_interaction_expression_graphs_with_local_contracts(
            id,
            checked_view.scope,
            checked.expression_count,
            &local_contracts,
            span,
        )?;

        let width = self.resolve_pane_length(&checked.width, span)?;
        let height = self.resolve_pane_length(&checked.height, span)?;
        for expression in [checked.spacing, checked.min_size, checked.resize_leeway]
            .into_iter()
            .flatten()
        {
            self.require_pane_expression_type(expression, &Type::F64, span)?;
        }
        let custom_style = checked
            .custom_style
            .as_ref()
            .map(|style| self.resolve_pane_custom_style(style, span))
            .transpose()?;
        let style = self.resolve_pane_grid_style(&checked.style, checked.origin, span)?;
        let click = checked
            .click
            .as_ref()
            .map(|route| self.resolve_pane_route(route, id, checked_view.scope, span))
            .transpose()?;
        let panes = checked
            .panes
            .iter()
            .enumerate()
            .map(|(index, pane)| {
                self.resolve_pane_view(
                    pane,
                    id,
                    CheckedViewLocalRole::PaneMaximized(index as u32),
                    span,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let templates = checked
            .templates
            .iter()
            .enumerate()
            .map(|(index, template)| {
                self.resolve_pane_template(template, id, index as u32, checked_view.scope, span)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let resolved = ResolvedPaneGrid {
            id,
            name: checked.name,
            configuration: Self::resolve_pane_configuration(checked.configuration),
            width,
            height,
            spacing: checked.spacing,
            min_size: checked.min_size,
            resize_leeway: checked.resize_leeway,
            draggable: checked.draggable,
            click,
            custom_style,
            style,
            panes,
            templates,
            test_scope: matches!(checked_view.scope, CheckedViewScope::Test(_)),
            origin: checked.origin,
        };
        if self.pane_grids.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "pane grid was lowered more than once"));
        }
        Ok(())
    }

    fn pane_local_contracts(
        &self,
        checked: &CheckedPaneGrid,
        span: &Span,
    ) -> Result<HashMap<CheckedExprUseId, HashSet<CheckedLocalId>>, Error> {
        let mut contracts = HashMap::new();
        for index in 0..checked.expression_count {
            let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: checked.id,
                index,
            });
            let expression = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "pane expression has no checked owner mapping")
            })?;
            contracts.insert(expression, HashSet::new());
        }
        for pane in &checked.panes {
            let allowed = pane.maximized.into_iter().collect::<HashSet<_>>();
            add_pane_view_local_contracts(&mut contracts, pane, &allowed, span)?;
        }
        for template in &checked.templates {
            let mut allowed = HashSet::from([template.item]);
            add_allowed(&mut contracts, template.key, &allowed, span)?;
            if let Some(maximized) = template.pane.maximized {
                allowed.insert(maximized);
            }
            add_pane_view_local_contracts(&mut contracts, &template.pane, &allowed, span)?;
        }
        if contracts.len() != checked.expression_count as usize {
            return Err(self.invariant(span, "pane expression local contracts diverged"));
        }
        Ok(contracts)
    }

    fn resolve_pane_length(
        &self,
        length: &CheckedLength,
        span: &Span,
    ) -> Result<Option<ResolvedPaneLength>, Error> {
        Ok(match length {
            CheckedLength::None => None,
            CheckedLength::Fill => Some(ResolvedPaneLength::Fill),
            CheckedLength::FillPortion(portion) => Some(ResolvedPaneLength::FillPortion(*portion)),
            CheckedLength::Shrink => Some(ResolvedPaneLength::Shrink),
            CheckedLength::Fixed { expression, source } => {
                self.require_pane_expression_type(*expression, source, span)?;
                match source {
                    Type::F64 => Some(ResolvedPaneLength::FixedF64(*expression)),
                    Type::Length => Some(ResolvedPaneLength::FixedLength(*expression)),
                    _ => return Err(self.invariant(span, "pane length has an invalid type")),
                }
            }
        })
    }

    fn resolve_pane_custom_style(
        &self,
        style: &crate::check::CheckedPaneCustomStyle,
        span: &Span,
    ) -> Result<ResolvedPaneCustomStyle, Error> {
        let function = self
            .declarations
            .try_extern_decl(style.function)
            .ok_or_else(|| self.invariant(span, "pane style extern ID is outside its arena"))?;
        if function.kind != ExternKind::PaneGridStyle
            || function.params.len() != style.arguments.len()
        {
            return Err(self.invariant(span, "pane style extern contract diverged"));
        }
        for (argument, (_, expected)) in style.arguments.iter().zip(&function.params) {
            self.require_pane_expression_type(*argument, expected, span)?;
        }
        Ok(ResolvedPaneCustomStyle {
            function: style.function,
            arguments: style.arguments.clone(),
        })
    }

    fn resolve_pane_grid_style(
        &self,
        style: &CheckedPaneGridStyle,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedPaneGridStyle, Error> {
        for expression in [
            style.region_border_width,
            style.region_radius.all,
            style.region_radius.top_left,
            style.region_radius.top_right,
            style.region_radius.bottom_right,
            style.region_radius.bottom_left,
            style.hovered_split_width,
            style.picked_split_width,
        ]
        .into_iter()
        .flatten()
        {
            self.require_pane_expression_type(expression, &Type::F64, span)?;
        }
        Ok(ResolvedPaneGridStyle {
            region_background: style
                .region_background
                .as_ref()
                .map(|background| self.resolve_pane_background(background, origin, span))
                .transpose()?,
            region_border: self.resolve_optional_pane_color(
                style.region_border.as_deref(),
                origin,
                span,
            )?,
            region_border_width: style.region_border_width,
            region_radius: resolve_pane_radius(&style.region_radius),
            hovered_split: self.resolve_optional_pane_color(
                style.hovered_split.as_deref(),
                origin,
                span,
            )?,
            hovered_split_width: style.hovered_split_width,
            picked_split: self.resolve_optional_pane_color(
                style.picked_split.as_deref(),
                origin,
                span,
            )?,
            picked_split_width: style.picked_split_width,
        })
    }

    fn resolve_pane_view(
        &self,
        pane: &CheckedPaneView,
        grid: ViewId,
        role: CheckedViewLocalRole,
        span: &Span,
    ) -> Result<ResolvedPaneView, Error> {
        self.require_origin_parent(pane.origin, self.facts.view(grid).origin, span)?;
        let maximized = pane
            .maximized
            .map(|local| {
                self.resolve_pane_binding(local, grid, role, Type::Bool, pane.origin, span)
            })
            .transpose()?;
        Ok(ResolvedPaneView {
            name: pane.name.clone(),
            maximized,
            surface: self.resolve_pane_surface(&pane.surface, pane.origin, span)?,
            utility_style: self.resolve_pane_utility_style(pane.style_site, span)?,
            title: pane
                .title
                .as_ref()
                .map(|title| self.resolve_pane_title(title, pane.origin, span))
                .transpose()?,
            origin: pane.origin,
        })
    }

    fn resolve_pane_template(
        &self,
        template: &CheckedPaneTemplate,
        grid: ViewId,
        index: u32,
        scope: CheckedViewScope,
        span: &Span,
    ) -> Result<ResolvedPaneTemplate, Error> {
        self.require_origin_parent(template.origin, self.facts.view(grid).origin, span)?;
        let item = self.resolve_pane_binding(
            template.item,
            grid,
            CheckedViewLocalRole::PaneTemplateItem(index),
            self.facts
                .try_local(template.item)
                .ok_or_else(|| self.invariant(span, "pane template item local is invalid"))?
                .ty
                .clone(),
            template.origin,
            span,
        )?;
        let items = self.resolve_pane_items(&template.items, &item.ty, grid, scope, span)?;
        if template.key_type
            != self
                .facts
                .try_expression_use(template.key)
                .ok_or_else(|| self.invariant(span, "pane template key expression is invalid"))?
                .source
            || !matches!(
                template.key_type,
                Type::Bool | Type::I64 | Type::F64 | Type::Str
            )
        {
            return Err(self.invariant(span, "pane template key type diverged"));
        }
        self.require_origin_parent(template.pane.origin, template.origin, span)?;
        let pane = self.resolve_pane_view_with_parent(
            &template.pane,
            grid,
            CheckedViewLocalRole::PaneTemplateMaximized(index),
            template.origin,
            span,
        )?;
        Ok(ResolvedPaneTemplate {
            items,
            item,
            key: template.key,
            key_type: template.key_type.clone(),
            pane,
            origin: template.origin,
        })
    }

    fn resolve_pane_view_with_parent(
        &self,
        pane: &CheckedPaneView,
        grid: ViewId,
        role: CheckedViewLocalRole,
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedPaneView, Error> {
        self.require_origin_parent(pane.origin, parent, span)?;
        let maximized = pane
            .maximized
            .map(|local| {
                self.resolve_pane_binding(local, grid, role, Type::Bool, pane.origin, span)
            })
            .transpose()?;
        Ok(ResolvedPaneView {
            name: pane.name.clone(),
            maximized,
            surface: self.resolve_pane_surface(&pane.surface, pane.origin, span)?,
            utility_style: self.resolve_pane_utility_style(pane.style_site, span)?,
            title: pane
                .title
                .as_ref()
                .map(|title| self.resolve_pane_title(title, pane.origin, span))
                .transpose()?,
            origin: pane.origin,
        })
    }

    fn resolve_pane_binding(
        &self,
        local: CheckedLocalId,
        grid: ViewId,
        role: CheckedViewLocalRole,
        ty: Type,
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedPaneBinding, Error> {
        let checked = self
            .facts
            .try_local(local)
            .ok_or_else(|| self.invariant(span, "pane local ID is outside its arena"))?;
        if checked.owner != (CheckedLocalOwner::View { view: grid, role }) || checked.ty != ty {
            return Err(self.invariant(span, "pane local binding contract diverged"));
        }
        self.require_origin_parent(checked.origin, parent, span)?;
        Ok(ResolvedPaneBinding {
            local,
            name: checked.name.clone(),
            ty: checked.ty.clone(),
        })
    }

    fn resolve_pane_items(
        &self,
        root: &CheckedPathRoot,
        item_ty: &Type,
        grid: ViewId,
        scope: CheckedViewScope,
        span: &Span,
    ) -> Result<ResolvedPaneItems, Error> {
        let list_ty = Type::List(Box::new(item_ty.clone()));
        match root {
            CheckedPathRoot::Value(value) => {
                let checked = self.facts.try_value_by_ref(*value).ok_or_else(|| {
                    self.invariant(span, "pane items value ID is outside its arena")
                })?;
                let allowed = match (scope, *value) {
                    (
                        CheckedViewScope::App | CheckedViewScope::Test(_),
                        CheckedValueRef::AppState(_) | CheckedValueRef::Derived(_),
                    ) => true,
                    (
                        CheckedViewScope::Component(component),
                        CheckedValueRef::ComponentParam(id),
                    ) => id.component == component,
                    (
                        CheckedViewScope::Component(component),
                        CheckedValueRef::ComponentState(id),
                    ) => id.component == component,
                    _ => false,
                };
                if !allowed || checked.ty != list_ty {
                    return Err(self.invariant(span, "pane items value contract diverged"));
                }
                Ok(ResolvedPaneItems::Value(*value))
            }
            CheckedPathRoot::Local(local) => {
                let checked = self.facts.try_local(*local).ok_or_else(|| {
                    self.invariant(span, "pane items local ID is outside its arena")
                })?;
                let CheckedLocalOwner::View { view, .. } = checked.owner else {
                    return Err(self.invariant(span, "pane items local has an invalid owner"));
                };
                let mut parent = self.facts.view(grid).parent;
                let mut allowed = false;
                while let Some(current) = parent {
                    if current == view {
                        allowed = true;
                        break;
                    }
                    parent = self.facts.view(current).parent;
                }
                if !allowed || checked.ty != list_ty {
                    return Err(self.invariant(span, "pane items local contract diverged"));
                }
                Ok(ResolvedPaneItems::Local(*local))
            }
            CheckedPathRoot::EnumVariant(_) | CheckedPathRoot::Palette(_) => {
                Err(self.invariant(span, "pane items path is not a value or local"))
            }
        }
    }

    fn resolve_pane_title(
        &self,
        title: &CheckedPaneTitle,
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedPaneTitle, Error> {
        self.require_origin_parent(title.origin, parent, span)?;
        for expression in padding_ids(&title.padding) {
            self.require_pane_expression_type(expression, &Type::F64, span)?;
        }
        Ok(ResolvedPaneTitle {
            padding: ResolvedPanePadding {
                all: title.padding.all,
                x: title.padding.x,
                y: title.padding.y,
                top: title.padding.top,
                right: title.padding.right,
                bottom: title.padding.bottom,
                left: title.padding.left,
            },
            always_show_controls: title.always_show_controls,
            has_controls: title.has_controls,
            has_compact_controls: title.has_compact_controls,
            surface: self.resolve_pane_surface(&title.surface, title.origin, span)?,
            utility_style: self.resolve_pane_utility_style(title.style_site, span)?,
            origin: title.origin,
        })
    }

    fn resolve_pane_utility_style(
        &self,
        site: CheckedPaneStyleSite,
        span: &Span,
    ) -> Result<ResolvedStyle, Error> {
        self.styles
            .style_use(&Span {
                line: site.line,
                column: site.column,
            })
            .map(|style| style.style.clone())
            .map_err(|_| self.invariant(span, "pane utility style site is not normalized"))
    }

    fn resolve_pane_surface(
        &self,
        surface: &CheckedPaneSurface,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedPaneSurface, Error> {
        for expression in pane_surface_ids(surface) {
            let expected = if Some(expression) == surface.pixel_snap {
                &Type::Bool
            } else {
                &Type::F64
            };
            self.require_pane_expression_type(expression, expected, span)?;
        }
        Ok(ResolvedPaneSurface {
            background: surface
                .background
                .as_ref()
                .map(|background| self.resolve_pane_background(background, origin, span))
                .transpose()?,
            text_color: self.resolve_optional_pane_color(
                surface.text_color.as_deref(),
                origin,
                span,
            )?,
            border_color: self.resolve_optional_pane_color(
                surface.border_color.as_deref(),
                origin,
                span,
            )?,
            border_width: surface.border_width,
            radius: resolve_pane_radius(&surface.radius),
            shadow_color: self.resolve_optional_pane_color(
                surface.shadow_color.as_deref(),
                origin,
                span,
            )?,
            shadow_x: surface.shadow_x,
            shadow_y: surface.shadow_y,
            shadow_blur: surface.shadow_blur,
            pixel_snap: surface.pixel_snap,
        })
    }

    fn resolve_pane_background(
        &self,
        background: &CheckedPaneBackground,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedPaneBackground, Error> {
        Ok(match background {
            CheckedPaneBackground::Color(color) => {
                ResolvedPaneBackground::Color(self.resolve_pane_color(color, origin, span)?)
            }
            CheckedPaneBackground::Linear { angle, stops } => {
                self.require_pane_expression_type(*angle, &Type::F64, span)?;
                let stops = stops
                    .iter()
                    .map(|stop| {
                        self.require_pane_expression_type(stop.offset, &Type::F64, span)?;
                        Ok(ResolvedPaneGradientStop {
                            color: self.resolve_pane_color(&stop.color, origin, span)?,
                            offset: stop.offset,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                ResolvedPaneBackground::Linear {
                    angle: *angle,
                    stops,
                }
            }
        })
    }

    fn resolve_optional_pane_color(
        &self,
        color: Option<&str>,
        origin: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedThemeColor>, Error> {
        color
            .map(|color| self.resolve_pane_color(color, origin, span))
            .transpose()
    }

    fn resolve_pane_color(
        &self,
        color: &str,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedThemeColor, Error> {
        let source = self
            .origins
            .try_get(origin)
            .ok_or_else(|| self.invariant(span, "pane color origin ID is outside its arena"))?;
        self.resolve_theme_color(
            color,
            &Span {
                line: source.line,
                column: source.column,
            },
        )
    }

    fn require_pane_expression_type(
        &self,
        expression: CheckedExprUseId,
        expected: &Type,
        span: &Span,
    ) -> Result<(), Error> {
        let checked = self
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| self.invariant(span, "pane expression-use ID is outside its arena"))?;
        if &checked.source != expected || &checked.destination != expected {
            return Err(self.invariant(span, "pane expression type contract diverged"));
        }
        Ok(())
    }

    fn resolve_pane_route(
        &self,
        checked: &crate::check::CheckedInteractionRoute,
        grid: ViewId,
        scope: CheckedViewScope,
        span: &Span,
    ) -> Result<ResolvedInteractionRoute, Error> {
        if checked.id
            != (InteractionRouteId {
                widget: grid,
                index: 0,
            })
        {
            return Err(self.invariant(span, "pane click route ID diverged"));
        }
        let mut payload = 0u32;
        let mut args = Vec::with_capacity(checked.args.len());
        for argument in &checked.args {
            match argument {
                CheckedCanvasRouteArg::Expression(expression) => {
                    let retained = self.facts.try_expression_use(*expression).ok_or_else(|| {
                        self.invariant(span, "pane route expression ID is invalid")
                    })?;
                    if !matches!(
                        retained.owner,
                        CheckedExprOwner::Interaction(InteractionExpressionId { widget, .. })
                            if widget == grid
                    ) {
                        return Err(self.invariant(span, "pane route expression owner diverged"));
                    }
                    args.push(ResolvedInteractionRouteArg::Expression(*expression));
                }
                CheckedCanvasRouteArg::Payload => {
                    let index = if checked.ordered_payloads { payload } else { 0 };
                    let ty = checked
                        .source_payloads
                        .get(index as usize)
                        .cloned()
                        .ok_or_else(|| {
                            self.invariant(span, "pane route payload is out of range")
                        })?;
                    payload += 1;
                    args.push(ResolvedInteractionRouteArg::Payload { index, ty });
                }
            }
        }
        let target = match &checked.target {
            CheckedCanvasRouteTarget::Handler(handler) => {
                let declaration = self.declarations.try_handler(*handler).ok_or_else(|| {
                    self.invariant(span, "pane route handler ID is outside its arena")
                })?;
                if !route_handler_owner_is_reachable(declaration.owner, scope) {
                    return Err(self.invariant(span, "pane route handler scope diverged"));
                }
                ResolvedInteractionRouteTarget::TargetHandler(*handler)
            }
            CheckedCanvasRouteTarget::ComponentOutput { component, output } => {
                if !matches!(scope, CheckedViewScope::Component(owner) if owner == *component)
                    || self.declarations.component_output(*component) != Some(output)
                {
                    return Err(self.invariant(span, "pane route component output diverged"));
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
                    self.invariant(span, "pane route event ID is outside its arena")
                })?;
                if !matches!(scope, CheckedViewScope::Component(component) if component == event.component)
                    || declaration.name != *name
                    || declaration.payloads != *payloads
                {
                    return Err(self.invariant(span, "pane route event contract diverged"));
                }
                ResolvedInteractionRouteTarget::NamedEvent {
                    event: *event,
                    name: name.clone(),
                    payloads: payloads.clone(),
                }
            }
        };
        Ok(ResolvedInteractionRoute {
            id: checked.id,
            target,
            args,
            source_payloads: checked.source_payloads.clone(),
            ordered_payloads: checked.ordered_payloads,
            origin: checked.origin,
        })
    }

    fn require_origin_parent(
        &self,
        origin: OriginId,
        parent: OriginId,
        span: &Span,
    ) -> Result<(), Error> {
        let checked = self
            .origins
            .try_get(origin)
            .ok_or_else(|| self.invariant(span, "pane origin ID is outside its arena"))?;
        if checked.parent != Some(parent) {
            return Err(self.invariant_at_origin(origin, "pane origin parent diverged"));
        }
        Ok(())
    }

    fn resolve_pane_configuration(checked: CheckedPaneConfiguration) -> ResolvedPaneConfiguration {
        match checked {
            CheckedPaneConfiguration::Pane(name) => ResolvedPaneConfiguration::Pane(name),
            CheckedPaneConfiguration::Split {
                name,
                axis,
                ratio,
                a,
                b,
            } => ResolvedPaneConfiguration::Split {
                name,
                axis: match axis {
                    CheckedPaneAxis::Horizontal => ResolvedPaneAxis::Horizontal,
                    CheckedPaneAxis::Vertical => ResolvedPaneAxis::Vertical,
                },
                ratio,
                a: Box::new(Self::resolve_pane_configuration(*a)),
                b: Box::new(Self::resolve_pane_configuration(*b)),
            },
        }
    }
}

fn resolve_pane_radius(radius: &CheckedPaneRadius) -> ResolvedPaneRadius {
    ResolvedPaneRadius {
        all: radius.all,
        top_left: radius.top_left,
        top_right: radius.top_right,
        bottom_right: radius.bottom_right,
        bottom_left: radius.bottom_left,
    }
}

fn pane_background_ids(background: &CheckedPaneBackground) -> Vec<CheckedExprUseId> {
    match background {
        CheckedPaneBackground::Color(_) => Vec::new(),
        CheckedPaneBackground::Linear { angle, stops } => std::iter::once(*angle)
            .chain(stops.iter().map(|stop| stop.offset))
            .collect(),
    }
}

fn pane_radius_ids(radius: &CheckedPaneRadius) -> Vec<CheckedExprUseId> {
    [
        radius.all,
        radius.top_left,
        radius.top_right,
        radius.bottom_right,
        radius.bottom_left,
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn pane_surface_ids(surface: &CheckedPaneSurface) -> Vec<CheckedExprUseId> {
    surface
        .background
        .as_ref()
        .into_iter()
        .flat_map(pane_background_ids)
        .chain([surface.border_width].into_iter().flatten())
        .chain(pane_radius_ids(&surface.radius))
        .chain(
            [
                surface.shadow_x,
                surface.shadow_y,
                surface.shadow_blur,
                surface.pixel_snap,
            ]
            .into_iter()
            .flatten(),
        )
        .collect()
}

fn padding_ids(padding: &CheckedPadding) -> Vec<CheckedExprUseId> {
    [
        padding.all,
        padding.x,
        padding.y,
        padding.top,
        padding.right,
        padding.bottom,
        padding.left,
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn add_allowed(
    contracts: &mut HashMap<CheckedExprUseId, HashSet<CheckedLocalId>>,
    expression: CheckedExprUseId,
    allowed: &HashSet<CheckedLocalId>,
    span: &Span,
) -> Result<(), Error> {
    let contract = contracts.get_mut(&expression).ok_or_else(|| {
        Error::new(
            "E196",
            span,
            "lowering invariant violated: pane expression contract was not indexed",
        )
    })?;
    contract.extend(allowed);
    Ok(())
}

fn add_pane_view_local_contracts(
    contracts: &mut HashMap<CheckedExprUseId, HashSet<CheckedLocalId>>,
    pane: &CheckedPaneView,
    allowed: &HashSet<CheckedLocalId>,
    span: &Span,
) -> Result<(), Error> {
    for expression in pane_surface_ids(&pane.surface) {
        add_allowed(contracts, expression, allowed, span)?;
    }
    if let Some(title) = &pane.title {
        for expression in padding_ids(&title.padding)
            .into_iter()
            .chain(pane_surface_ids(&title.surface))
        {
            add_allowed(contracts, expression, allowed, span)?;
        }
    }
    Ok(())
}
