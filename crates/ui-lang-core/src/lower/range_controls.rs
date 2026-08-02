// Stable IDs, checked types, and physical origins are retained even when the
// emitter does not inspect every field directly.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedRangeAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedRangeCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedSliderHandleShape {
    Circle(CheckedExprUseId),
    Rectangle {
        width: u16,
        radius: ResolvedContainerRadius,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSliderStatusStyle {
    pub(crate) rail_start: Option<ResolvedContainerBackground>,
    pub(crate) rail_end: Option<ResolvedContainerBackground>,
    pub(crate) rail_width: Option<CheckedExprUseId>,
    pub(crate) rail_border_color: Option<ResolvedThemeColor>,
    pub(crate) rail_border_width: Option<CheckedExprUseId>,
    pub(crate) rail_radius: ResolvedContainerRadius,
    pub(crate) handle_shape: Option<ResolvedSliderHandleShape>,
    pub(crate) handle_color: Option<ResolvedContainerBackground>,
    pub(crate) handle_border_color: Option<ResolvedThemeColor>,
    pub(crate) handle_border_width: Option<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedSliderStyleSet {
    pub(crate) active: Option<ResolvedSliderStatusStyle>,
    pub(crate) hovered: Option<ResolvedSliderStatusStyle>,
    pub(crate) dragged: Option<ResolvedSliderStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSlider {
    pub(crate) id: ViewId,
    pub(crate) value_type: Type,
    pub(crate) value: CheckedExprUseId,
    pub(crate) min: CheckedExprUseId,
    pub(crate) max: CheckedExprUseId,
    pub(crate) step: CheckedExprUseId,
    pub(crate) default: Option<CheckedExprUseId>,
    pub(crate) shift_step: Option<CheckedExprUseId>,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) axis: ResolvedRangeAxis,
    pub(crate) change: ResolvedInteractionRoute,
    pub(crate) release: Option<ResolvedInteractionRoute>,
    pub(crate) custom_style: Option<ResolvedRangeCustomStyle>,
    pub(crate) styles: ResolvedSliderStyleSet,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedProgressStyle {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedProgress {
    pub(crate) id: ViewId,
    pub(crate) value: CheckedExprUseId,
    pub(crate) min: CheckedExprUseId,
    pub(crate) max: CheckedExprUseId,
    pub(crate) length: Option<ResolvedContainerLength>,
    pub(crate) girth: Option<ResolvedContainerLength>,
    pub(crate) axis: ResolvedRangeAxis,
    pub(crate) style: Option<ResolvedProgressStyle>,
    pub(crate) custom_style: Option<ResolvedRangeCustomStyle>,
    pub(crate) background: Option<ResolvedContainerBackground>,
    pub(crate) bar: Option<ResolvedContainerBackground>,
    pub(crate) border_color: Option<ResolvedThemeColor>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedContainerRadius,
    pub(crate) origin: OriginId,
}

struct RangeOperands<'a> {
    lowerer: &'a Lowerer,
    widget: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    parent: OriginId,
    span: &'a Span,
    family: &'static str,
}

impl RangeOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
        physical_origin: OriginId,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                format!("{} {label} expression disappeared", self.family),
            )
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.widget,
            index: self.next,
        });
        self.next += 1;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!("{} {label} expression ID is invalid", self.family),
                )
            })?;
        let retained_origin = self
            .lowerer
            .origins
            .try_get(retained.origin)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!("{} {label} expression origin is invalid", self.family),
                )
            })?;
        let expected_origin = self
            .lowerer
            .origins
            .try_get(physical_origin)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!("{} {label} physical origin is invalid", self.family),
                )
            })?;
        if retained.owner != owner
            || self.lowerer.facts.expression_use_by_owner(owner) != Some(expression)
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
            || retained_origin.parent != Some(self.parent)
            || retained_origin.path != expected_origin.path
            || retained_origin.line != expected_origin.line
            || retained_origin.column != expected_origin.column
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("{} {label} expression contract diverged", self.family),
            ));
        }
        Ok((expression, retained.source.clone()))
    }

    fn take(
        &mut self,
        expected: &Type,
        label: &str,
        origin: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        self.take_where(label, |actual| actual == expected, origin)
            .map(|(expression, _)| expression)
    }

    fn optional<T>(
        &mut self,
        source: Option<&T>,
        expected: &Type,
        label: &str,
        origin: OriginId,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        source
            .map(|_| self.take(expected, label, origin))
            .transpose()
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.expressions.next().is_some() {
            return Err(self.lowerer.invariant(
                self.span,
                format!("{} left checked option expressions unconsumed", self.family),
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_slider(
        &mut self,
        value: &Expr,
        min: &Expr,
        max: &Expr,
        step: &Expr,
        options: &SliderOptions,
        vertical: bool,
        styles: &[String],
        route: &Route,
        release: &Option<Route>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let semantic_key =
            crate::ast::slider_semantic_key(options, vertical, styles, route, release);
        let roots = crate::ast::slider_expression_roots(value, min, max, step, options, span);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Slider,
            semantic_key,
            span,
            outer_component,
        )?;
        let slider = self
            .facts
            .slider(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "slider has no checked HIR facts"))?;
        if checked.option_expressions.len() != roots.len() {
            return Err(self.invariant(span, "slider expression cardinality diverged"));
        }
        if !matches!(slider.value_type, Type::F64 | Type::Named(_)) {
            return Err(self.invariant(span, "slider retained an invalid value type"));
        }
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        let mut values = RangeOperands {
            lowerer: self,
            widget: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            parent: origin,
            span,
            family: "slider",
        };
        let value = values.take(&slider.value_type, "value", origin)?;
        let min = values.take(&slider.value_type, "minimum", origin)?;
        let max = values.take(&slider.value_type, "maximum", origin)?;
        let step = values.take(&slider.value_type, "step", origin)?;
        let default = values.optional(
            options.default.as_ref(),
            &slider.value_type,
            "default",
            origin,
        )?;
        let shift_step = values.optional(
            options.shift_step.as_ref(),
            &slider.value_type,
            "shift step",
            origin,
        )?;
        let width = Self::resolve_range_length(&mut values, &options.width, "width", origin)?;
        let height = Self::resolve_range_length(&mut values, &options.height, "height", origin)?;
        let custom_style = self.resolve_range_custom_style(
            &mut values,
            options.style.custom.as_ref(),
            slider.style,
            slider.style_origin,
            ExternKind::SliderStyle,
            "slider",
            origin,
            span,
        )?;
        let resolved_styles = self.resolve_slider_styles(
            &mut values,
            &options.style,
            &slider.status_origins,
            origin,
            span,
        )?;
        values.finish()?;

        let routes = std::iter::once(route)
            .chain(release.iter())
            .collect::<Vec<_>>();
        let mut route_index = 0usize;
        let change = self.lower_required_interaction_route(
            route,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        let release = self.lower_optional_interaction_route(
            release,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != checked.routes.len()
            || change.source_payloads != [slider.value_type.clone()]
            || release
                .as_ref()
                .is_some_and(|route| !route.source_payloads.is_empty())
        {
            return Err(self.invariant(span, "slider route contract diverged"));
        }
        let resolved = ResolvedSlider {
            id,
            value_type: slider.value_type,
            value,
            min,
            max,
            step,
            default,
            shift_step,
            width,
            height,
            axis: if vertical {
                ResolvedRangeAxis::Vertical
            } else {
                ResolvedRangeAxis::Horizontal
            },
            change,
            release,
            custom_style,
            styles: resolved_styles,
            origin,
        };
        if self.sliders.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "slider was lowered more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_progress(
        &mut self,
        value: &Expr,
        min: &Expr,
        max: &Expr,
        options: &ProgressOptions,
        vertical: bool,
        styles: &[String],
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let semantic_key = crate::ast::progress_semantic_key(options, vertical, styles);
        let roots = crate::ast::progress_expression_roots(value, min, max, options, span);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Progress,
            semantic_key,
            span,
            outer_component,
        )?;
        let progress = self
            .facts
            .progress(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "progress has no checked HIR facts"))?;
        if checked.option_expressions.len() != roots.len() || !checked.routes.is_empty() {
            return Err(self.invariant(span, "progress expression or route cardinality diverged"));
        }
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        let mut values = RangeOperands {
            lowerer: self,
            widget: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            parent: origin,
            span,
            family: "progress",
        };
        let value = values.take(&Type::F64, "value", origin)?;
        let min = values.take(&Type::F64, "minimum", origin)?;
        let max = values.take(&Type::F64, "maximum", origin)?;
        let length = Self::resolve_range_length(&mut values, &options.length, "length", origin)?;
        let girth = Self::resolve_range_length(&mut values, &options.girth, "girth", origin)?;
        let custom_style = self.resolve_range_custom_style(
            &mut values,
            options.custom_style.as_ref(),
            progress.style,
            progress.style_origin,
            ExternKind::ProgressStyle,
            "progress",
            origin,
            span,
        )?;
        let background = self.resolve_range_background(
            &mut values,
            &options.background,
            "background",
            origin,
            span,
        )?;
        let bar = self.resolve_range_background(&mut values, &options.bar, "bar", origin, span)?;
        let border_width = values.optional(
            options.border_width.as_ref(),
            &Type::F64,
            "border width",
            origin,
        )?;
        let radius = Self::resolve_range_radius(
            &mut values,
            &options.radius,
            [
                &options.radius_top_left,
                &options.radius_top_right,
                &options.radius_bottom_right,
                &options.radius_bottom_left,
            ],
            "radius",
            origin,
        )?;
        values.finish()?;
        let resolved = ResolvedProgress {
            id,
            value,
            min,
            max,
            length,
            girth,
            axis: if vertical {
                ResolvedRangeAxis::Vertical
            } else {
                ResolvedRangeAxis::Horizontal
            },
            style: options.style.map(|style| match style {
                ProgressStyle::Primary => ResolvedProgressStyle::Primary,
                ProgressStyle::Secondary => ResolvedProgressStyle::Secondary,
                ProgressStyle::Success => ResolvedProgressStyle::Success,
                ProgressStyle::Warning => ResolvedProgressStyle::Warning,
                ProgressStyle::Danger => ResolvedProgressStyle::Danger,
            }),
            custom_style,
            background,
            bar,
            border_color: options
                .border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_width,
            radius,
            origin,
        };
        if self.progresses.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "progress was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_range_length(
        values: &mut RangeOperands<'_>,
        source: &Option<LengthValue>,
        label: &str,
        origin: OriginId,
    ) -> Result<Option<ResolvedContainerLength>, Error> {
        Ok(match source {
            None => None,
            Some(LengthValue::Fill) => Some(ResolvedContainerLength::Fill),
            Some(LengthValue::FillPortion(portion)) => {
                Some(ResolvedContainerLength::FillPortion(*portion))
            }
            Some(LengthValue::Shrink) => Some(ResolvedContainerLength::Shrink),
            Some(LengthValue::Fixed(_)) => {
                let (expression, source) = values.take_where(
                    label,
                    |actual| matches!(actual, Type::F64 | Type::Length),
                    origin,
                )?;
                Some(match source {
                    Type::F64 => ResolvedContainerLength::FixedF64(expression),
                    Type::Length => ResolvedContainerLength::FixedLength(expression),
                    _ => unreachable!("validated range length type"),
                })
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_range_custom_style(
        &self,
        values: &mut RangeOperands<'_>,
        source: Option<&ExternCall>,
        checked: Option<ExternFnId>,
        checked_origin: Option<OriginId>,
        kind: ExternKind,
        family: &str,
        parent: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedRangeCustomStyle>, Error> {
        match (source, checked, checked_origin) {
            (None, None, None) => Ok(None),
            (Some(source), Some(function), Some(origin)) => {
                self.require_range_origin(origin, parent, span, family)?;
                let declaration = self.declarations.try_extern_decl(function).ok_or_else(|| {
                    self.invariant(span, format!("{family} style extern is invalid"))
                })?;
                if declaration.kind != kind
                    || declaration.name != source.function
                    || declaration.params.len() != source.args.len()
                {
                    return Err(
                        self.invariant(span, format!("{family} style extern contract diverged"))
                    );
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "style argument", origin))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(ResolvedRangeCustomStyle {
                    function,
                    arguments,
                    origin,
                }))
            }
            _ => Err(self.invariant(span, format!("{family} style extern presence diverged"))),
        }
    }

    fn resolve_slider_styles(
        &self,
        values: &mut RangeOperands<'_>,
        source: &SliderStyleSet,
        origins: &[OriginId],
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedSliderStyleSet, Error> {
        let sources = [&source.active, &source.hovered, &source.dragged];
        if origins.len() != sources.into_iter().filter(|style| style.is_some()).count() {
            return Err(self.invariant(span, "slider status origin count diverged"));
        }
        let mut origins = origins.iter().copied();
        let mut resolve = |source: &Option<SliderStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let origin = origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "slider status origin disappeared"))?;
                    self.require_range_origin(origin, parent, span, "slider status")?;
                    self.resolve_slider_status(values, source, origin, span)
                })
                .transpose()
        };
        let styles = ResolvedSliderStyleSet {
            active: resolve(&source.active)?,
            hovered: resolve(&source.hovered)?,
            dragged: resolve(&source.dragged)?,
        };
        if origins.next().is_some() {
            return Err(self.invariant(span, "slider left status origins unconsumed"));
        }
        Ok(styles)
    }

    fn resolve_slider_status(
        &self,
        values: &mut RangeOperands<'_>,
        source: &SliderStyle,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedSliderStatusStyle, Error> {
        let rail_start =
            self.resolve_range_background(values, &source.rail_start, "rail start", origin, span)?;
        let rail_end =
            self.resolve_range_background(values, &source.rail_end, "rail end", origin, span)?;
        let handle_color = self.resolve_range_background(
            values,
            &source.handle_color,
            "handle color",
            origin,
            span,
        )?;
        let rail_width =
            values.optional(source.rail_width.as_ref(), &Type::F64, "rail width", origin)?;
        let rail_border_width = values.optional(
            source.rail_border_width.as_ref(),
            &Type::F64,
            "rail border width",
            origin,
        )?;
        let rail_radius = Self::resolve_range_radius(
            values,
            &source.rail_radius,
            [
                &source.rail_radius_top_left,
                &source.rail_radius_top_right,
                &source.rail_radius_bottom_right,
                &source.rail_radius_bottom_left,
            ],
            "rail radius",
            origin,
        )?;
        let handle_border_width = values.optional(
            source.handle_border_width.as_ref(),
            &Type::F64,
            "handle border width",
            origin,
        )?;
        let handle_radius = Self::resolve_range_radius(
            values,
            &source.handle_radius,
            [
                &source.handle_radius_top_left,
                &source.handle_radius_top_right,
                &source.handle_radius_bottom_right,
                &source.handle_radius_bottom_left,
            ],
            "handle radius",
            origin,
        )?;
        let handle_shape = match &source.handle_shape {
            None => None,
            Some(SliderHandleShape::Circle(_)) => Some(ResolvedSliderHandleShape::Circle(
                values.take(&Type::F64, "handle circle radius", origin)?,
            )),
            Some(SliderHandleShape::Rectangle { width }) => {
                Some(ResolvedSliderHandleShape::Rectangle {
                    width: *width,
                    radius: handle_radius,
                })
            }
        };
        Ok(ResolvedSliderStatusStyle {
            rail_start,
            rail_end,
            rail_width,
            rail_border_color: source
                .rail_border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            rail_border_width,
            rail_radius,
            handle_shape,
            handle_color,
            handle_border_color: source
                .handle_border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            handle_border_width,
            origin,
        })
    }

    fn resolve_range_background(
        &self,
        values: &mut RangeOperands<'_>,
        source: &Option<BackgroundValue>,
        label: &str,
        origin: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedContainerBackground>, Error> {
        source
            .as_ref()
            .map(|source| {
                Ok(match source {
                    BackgroundValue::Color(color) => {
                        ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
                    }
                    BackgroundValue::Linear { stops, .. } => {
                        let angle = values.take(&Type::F64, &format!("{label} angle"), origin)?;
                        let stops = stops
                            .iter()
                            .map(|stop| {
                                Ok(ResolvedContainerGradientStop {
                                    color: self.resolve_theme_color(&stop.color, span)?,
                                    offset: values.take(
                                        &Type::F64,
                                        &format!("{label} stop"),
                                        origin,
                                    )?,
                                })
                            })
                            .collect::<Result<Vec<_>, Error>>()?;
                        ResolvedContainerBackground::Linear { angle, stops }
                    }
                })
            })
            .transpose()
    }

    fn resolve_range_radius(
        values: &mut RangeOperands<'_>,
        all: &Option<Expr>,
        corners: [&Option<Expr>; 4],
        label: &str,
        origin: OriginId,
    ) -> Result<ResolvedContainerRadius, Error> {
        Ok(ResolvedContainerRadius {
            all: values.optional(all.as_ref(), &Type::F64, label, origin)?,
            top_left: values.optional(corners[0].as_ref(), &Type::F64, label, origin)?,
            top_right: values.optional(corners[1].as_ref(), &Type::F64, label, origin)?,
            bottom_right: values.optional(corners[2].as_ref(), &Type::F64, label, origin)?,
            bottom_left: values.optional(corners[3].as_ref(), &Type::F64, label, origin)?,
        })
    }

    fn require_range_origin(
        &self,
        origin: OriginId,
        parent: OriginId,
        span: &Span,
        label: &str,
    ) -> Result<(), Error> {
        if self
            .origins
            .try_get(origin)
            .is_none_or(|origin| origin.parent != Some(parent))
        {
            return Err(self.invariant(span, format!("{label} origin parent diverged")));
        }
        Ok(())
    }
}
