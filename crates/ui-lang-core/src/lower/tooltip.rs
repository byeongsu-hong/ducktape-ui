// Stable IDs and origin links are part of the normalized contract even when
// today's backend does not inspect every retained field.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
    FollowCursor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTooltipPreset {
    Transparent,
    Rounded,
    Bordered,
    Dark,
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTooltipCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTooltipBaseStyle {
    Preset(ResolvedTooltipPreset),
    Custom(ResolvedTooltipCustomStyle),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTooltipGradientStop {
    pub(crate) color: ResolvedThemeColor,
    pub(crate) offset: CheckedExprUseId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTooltipBackground {
    Color(ResolvedThemeColor),
    Linear {
        angle: CheckedExprUseId,
        stops: Vec<ResolvedTooltipGradientStop>,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedTooltipRadius {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) top_left: Option<CheckedExprUseId>,
    pub(crate) top_right: Option<CheckedExprUseId>,
    pub(crate) bottom_right: Option<CheckedExprUseId>,
    pub(crate) bottom_left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTooltip {
    pub(crate) id: ViewId,
    pub(crate) position: ResolvedTooltipPosition,
    pub(crate) gap: CheckedExprUseId,
    pub(crate) padding: CheckedExprUseId,
    pub(crate) delay_ms: CheckedExprUseId,
    pub(crate) snap: CheckedExprUseId,
    pub(crate) base_style: Option<ResolvedTooltipBaseStyle>,
    pub(crate) background: Option<ResolvedTooltipBackground>,
    pub(crate) text_color: Option<ResolvedThemeColor>,
    pub(crate) border_color: Option<ResolvedThemeColor>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedTooltipRadius,
    pub(crate) shadow_color: Option<ResolvedThemeColor>,
    pub(crate) shadow_x: Option<CheckedExprUseId>,
    pub(crate) shadow_y: Option<CheckedExprUseId>,
    pub(crate) shadow_blur: Option<CheckedExprUseId>,
    pub(crate) pixel_snap: Option<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

struct TooltipOperands<'a> {
    lowerer: &'a Lowerer,
    tooltip: ViewId,
    next: u32,
    span: &'a Span,
}

impl TooltipOperands<'_> {
    fn take(&mut self, expected: &Type) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Tooltip(TooltipExpressionId {
            tooltip: self.tooltip,
            index: self.next,
        });
        self.next += 1;
        let expression = self
            .lowerer
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, "tooltip expression has no owner")
            })?;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, "tooltip expression-use ID is outside its arena")
            })?;
        if retained.owner != owner
            || &retained.source != expected
            || &retained.destination != expected
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self
                .lowerer
                .invariant(self.span, "tooltip expression contract diverged"));
        }
        Ok(expression)
    }

    fn optional<T>(
        &mut self,
        value: Option<&T>,
        expected: &Type,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        value.map(|_| self.take(expected)).transpose()
    }

    fn finish(&self, expected: u32) -> Result<(), Error> {
        if self.next != expected {
            return Err(self
                .lowerer
                .invariant(self.span, "tooltip left checked expressions unconsumed"));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_tooltip(
        &mut self,
        options: &TooltipOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "tooltip has no shared view ID"))?;
        let checked = self
            .facts
            .tooltip(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "tooltip has no checked HIR facts"))?;
        let checked_view = self.facts.view(id);
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if checked.id != id
            || expected_scope != outer_component
            || checked.semantic_key != crate::ast::tooltip_semantic_key(options)
            || checked.expression_count as usize
                != crate::ast::tooltip_expression_roots(options).len()
        {
            return Err(self.invariant(span, "tooltip topology diverged after semantic checking"));
        }
        self.validate_tooltip_style(&checked, options, span)?;
        self.validate_tooltip_expression_graphs(
            id,
            checked_view.scope,
            checked.expression_count,
            span,
        )?;

        let mut values = TooltipOperands {
            lowerer: self,
            tooltip: id,
            next: 0,
            span,
        };
        let gap = values.take(&Type::F64)?;
        let padding = values.take(&Type::F64)?;
        let delay_ms = values.take(&Type::I64)?;
        let snap = values.take(&Type::Bool)?;
        let base_style = if let Some(style) = &options.custom_style {
            let function = checked
                .style
                .ok_or_else(|| self.invariant(span, "tooltip style lost its checked extern ID"))?;
            let arguments = style
                .args
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    let declaration = self
                        .declarations
                        .try_extern_decl(function)
                        .ok_or_else(|| self.invariant(span, "tooltip style extern disappeared"))?;
                    let expected = &declaration.params[index].1;
                    values.take(expected)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Some(ResolvedTooltipBaseStyle::Custom(
                ResolvedTooltipCustomStyle {
                    function,
                    arguments,
                    origin: checked_view.origin,
                },
            ))
        } else {
            options.style.map(|style| {
                ResolvedTooltipBaseStyle::Preset(match style {
                    TooltipStyle::Transparent => ResolvedTooltipPreset::Transparent,
                    TooltipStyle::Rounded => ResolvedTooltipPreset::Rounded,
                    TooltipStyle::Bordered => ResolvedTooltipPreset::Bordered,
                    TooltipStyle::Dark => ResolvedTooltipPreset::Dark,
                    TooltipStyle::Primary => ResolvedTooltipPreset::Primary,
                    TooltipStyle::Secondary => ResolvedTooltipPreset::Secondary,
                    TooltipStyle::Success => ResolvedTooltipPreset::Success,
                    TooltipStyle::Warning => ResolvedTooltipPreset::Warning,
                    TooltipStyle::Danger => ResolvedTooltipPreset::Danger,
                })
            })
        };
        let background = options
            .background
            .as_ref()
            .map(|background| {
                Ok(match background {
                    BackgroundValue::Color(color) => {
                        ResolvedTooltipBackground::Color(self.resolve_theme_color(color, span)?)
                    }
                    BackgroundValue::Linear { stops, .. } => {
                        let angle = values.take(&Type::F64)?;
                        let stops = stops
                            .iter()
                            .map(|stop| {
                                Ok(ResolvedTooltipGradientStop {
                                    color: self.resolve_theme_color(&stop.color, span)?,
                                    offset: values.take(&Type::F64)?,
                                })
                            })
                            .collect::<Result<Vec<_>, Error>>()?;
                        ResolvedTooltipBackground::Linear { angle, stops }
                    }
                })
            })
            .transpose()?;
        let border_width = values.optional(options.border_width.as_ref(), &Type::F64)?;
        let radius = ResolvedTooltipRadius {
            all: values.optional(options.radius.as_ref(), &Type::F64)?,
            top_left: values.optional(options.radius_top_left.as_ref(), &Type::F64)?,
            top_right: values.optional(options.radius_top_right.as_ref(), &Type::F64)?,
            bottom_right: values.optional(options.radius_bottom_right.as_ref(), &Type::F64)?,
            bottom_left: values.optional(options.radius_bottom_left.as_ref(), &Type::F64)?,
        };
        let shadow_x = values.optional(options.shadow_x.as_ref(), &Type::F64)?;
        let shadow_y = values.optional(options.shadow_y.as_ref(), &Type::F64)?;
        let shadow_blur = values.optional(options.shadow_blur.as_ref(), &Type::F64)?;
        let pixel_snap = values.optional(options.pixel_snap.as_ref(), &Type::Bool)?;
        values.finish(checked.expression_count)?;

        let resolved = ResolvedTooltip {
            id,
            position: match options.position {
                TooltipPosition::Top => ResolvedTooltipPosition::Top,
                TooltipPosition::Bottom => ResolvedTooltipPosition::Bottom,
                TooltipPosition::Left => ResolvedTooltipPosition::Left,
                TooltipPosition::Right => ResolvedTooltipPosition::Right,
                TooltipPosition::FollowCursor => ResolvedTooltipPosition::FollowCursor,
            },
            gap,
            padding,
            delay_ms,
            snap,
            base_style,
            background,
            text_color: options
                .text_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_color: options
                .border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_width,
            radius,
            shadow_color: options
                .shadow_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            shadow_x,
            shadow_y,
            shadow_blur,
            pixel_snap,
            origin: checked_view.origin,
        };
        if self.tooltips.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "tooltip was lowered more than once"));
        }
        Ok(())
    }

    fn validate_tooltip_style(
        &self,
        checked: &CheckedTooltip,
        options: &TooltipOptions,
        span: &Span,
    ) -> Result<(), Error> {
        match (&options.custom_style, checked.style) {
            (None, None) => Ok(()),
            (Some(style), Some(id)) => {
                let declaration = self.declarations.try_extern_decl(id).ok_or_else(|| {
                    self.invariant(span, "tooltip style references an invalid extern ID")
                })?;
                if declaration.kind != ExternKind::ContainerStyle
                    || declaration.name != style.function
                    || declaration.params.len() != style.args.len()
                {
                    return Err(
                        self.invariant(span, "tooltip style contract diverged after checking")
                    );
                }
                Ok(())
            }
            _ => Err(self.invariant(span, "tooltip style presence diverged after checking")),
        }
    }

    fn validate_tooltip_expression_graphs(
        &self,
        tooltip: ViewId,
        scope: CheckedViewScope,
        count: u32,
        span: &Span,
    ) -> Result<(), Error> {
        let mut graph = CheckedExpressionGraph::default();
        for index in 0..count {
            let owner = CheckedExprOwner::Tooltip(TooltipExpressionId { tooltip, index });
            let use_id = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "tooltip expression has no checked owner mapping")
            })?;
            let expression = self.facts.try_expression_use(use_id).ok_or_else(|| {
                self.invariant(span, "tooltip expression-use ID is outside its arena")
            })?;
            if expression.owner != owner {
                return Err(self.invariant(span, "tooltip expression owner mapping diverged"));
            }
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: tooltip,
                scope,
                use_id,
                span,
                canvas_locals: false,
                family: "tooltip",
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
                    "tooltip expression type or coercion contract diverged",
                ));
            }
        }
        Ok(())
    }
}
