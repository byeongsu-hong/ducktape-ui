use super::*;

#[derive(Clone, Debug)]
pub(crate) enum ResolvedContainerLength {
    Fill,
    FillPortion(u16),
    Shrink,
    FixedF64(CheckedExprUseId),
    FixedLength(CheckedExprUseId),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedContainerPadding {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) x: Option<CheckedExprUseId>,
    pub(crate) y: Option<CheckedExprUseId>,
    pub(crate) top: Option<CheckedExprUseId>,
    pub(crate) right: Option<CheckedExprUseId>,
    pub(crate) bottom: Option<CheckedExprUseId>,
    pub(crate) left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedContainerGradientStop {
    pub(crate) color: ResolvedThemeColor,
    pub(crate) offset: CheckedExprUseId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedContainerBackground {
    Color(ResolvedThemeColor),
    Linear {
        angle: CheckedExprUseId,
        stops: Vec<ResolvedContainerGradientStop>,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedContainerRadius {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) top_left: Option<CheckedExprUseId>,
    pub(crate) top_right: Option<CheckedExprUseId>,
    pub(crate) bottom_right: Option<CheckedExprUseId>,
    pub(crate) bottom_left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedContainerSurface {
    pub(crate) background: Option<ResolvedContainerBackground>,
    pub(crate) text_color: Option<ResolvedThemeColor>,
    pub(crate) border_color: Option<ResolvedThemeColor>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedContainerRadius,
    pub(crate) shadow_color: Option<ResolvedThemeColor>,
    pub(crate) shadow_x: Option<CheckedExprUseId>,
    pub(crate) shadow_y: Option<CheckedExprUseId>,
    pub(crate) shadow_blur: Option<CheckedExprUseId>,
    pub(crate) pixel_snap: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedContainerCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedContainerAlignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedContainerFlexAlignment {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedContainerFlexBasis {
    Auto,
    Content,
    Fixed(CheckedExprUseId),
    Percent(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedContainerFlexMargin {
    Zero,
    Auto,
    Fixed(CheckedExprUseId),
    Percent(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedContainerFlexMargins {
    pub(crate) top: ResolvedContainerFlexMargin,
    pub(crate) right: ResolvedContainerFlexMargin,
    pub(crate) bottom: ResolvedContainerFlexMargin,
    pub(crate) left: ResolvedContainerFlexMargin,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedContainerFlexItem {
    pub(crate) order: Option<CheckedExprUseId>,
    pub(crate) grow: Option<CheckedExprUseId>,
    pub(crate) shrink: Option<CheckedExprUseId>,
    pub(crate) basis: Option<ResolvedContainerFlexBasis>,
    pub(crate) align_self: Option<ResolvedContainerFlexAlignment>,
    pub(crate) margins: Option<ResolvedContainerFlexMargins>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedContainer {
    pub(crate) id: ViewId,
    pub(crate) padding: ResolvedContainerPadding,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) max_width: Option<CheckedExprUseId>,
    pub(crate) max_height: Option<CheckedExprUseId>,
    pub(crate) align_x: Option<ResolvedContainerAlignment>,
    pub(crate) align_y: Option<ResolvedContainerAlignment>,
    pub(crate) clip: Option<CheckedExprUseId>,
    pub(crate) custom_style: Option<ResolvedContainerCustomStyle>,
    pub(crate) surface: ResolvedContainerSurface,
    pub(crate) border_dash: Vec<CheckedExprUseId>,
    pub(crate) flex_item: ResolvedContainerFlexItem,
    pub(crate) utility_style: ResolvedStyle,
    pub(crate) origin: OriginId,
}

struct ContainerOperands<'a> {
    lowerer: &'a Lowerer,
    container: ViewId,
    next: u32,
    span: &'a Span,
}

impl ContainerOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.container,
            index: self.next,
        });
        self.next += 1;
        let expression = self
            .lowerer
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!("container {label} expression has no owner"),
                )
            })?;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!("container {label} expression ID is invalid"),
                )
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("container {label} expression contract diverged"),
            ));
        }
        Ok((expression, retained.source.clone()))
    }

    fn take(&mut self, expected: &Type, label: &str) -> Result<CheckedExprUseId, Error> {
        self.take_where(label, |actual| actual == expected)
            .map(|(expression, _)| expression)
    }

    fn optional<T>(
        &mut self,
        value: Option<&T>,
        expected: &Type,
        label: &str,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        value.map(|_| self.take(expected, label)).transpose()
    }

    fn finish(&self, expected: u32) -> Result<(), Error> {
        if self.next != expected {
            return Err(self
                .lowerer
                .invariant(self.span, "container left checked expressions unconsumed"));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_container(
        &mut self,
        options: &ContainerOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "container has no shared view ID"))?;
        let checked = self
            .facts
            .container(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "container has no checked HIR facts"))?;
        let checked_view = self.facts.view(id);
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if checked.id != id
            || expected_scope != outer_component
            || checked.semantic_key != crate::ast::container_semantic_key(options)
            || checked.expression_count as usize
                != crate::ast::container_expression_roots(options).len()
        {
            return Err(self.invariant(span, "container topology diverged after semantic checking"));
        }
        self.validate_interaction_expression_graphs(
            id,
            checked_view.scope,
            checked.expression_count,
            span,
        )?;
        let origin = checked_view.origin;
        let mut values = ContainerOperands {
            lowerer: self,
            container: id,
            next: 0,
            span,
        };

        let padding = ResolvedContainerPadding {
            all: values.optional(options.padding.all.as_ref(), &Type::F64, "padding")?,
            x: values.optional(options.padding.x.as_ref(), &Type::F64, "padding-x")?,
            y: values.optional(options.padding.y.as_ref(), &Type::F64, "padding-y")?,
            top: values.optional(options.padding.top.as_ref(), &Type::F64, "padding-top")?,
            right: values.optional(options.padding.right.as_ref(), &Type::F64, "padding-right")?,
            bottom: values.optional(
                options.padding.bottom.as_ref(),
                &Type::F64,
                "padding-bottom",
            )?,
            left: values.optional(options.padding.left.as_ref(), &Type::F64, "padding-left")?,
        };
        let width = Self::resolve_container_length(&mut values, &options.width, "width")?;
        let height = Self::resolve_container_length(&mut values, &options.height, "height")?;
        let max_width = values.optional(options.max_width.as_ref(), &Type::F64, "max-width")?;
        let max_height = values.optional(options.max_height.as_ref(), &Type::F64, "max-height")?;
        let clip = values.optional(options.clip.as_ref(), &Type::Bool, "clip")?;
        let custom_style = options
            .custom_style
            .as_ref()
            .map(|style| {
                let function = checked.style.ok_or_else(|| {
                    self.invariant(span, "container custom style lost its checked extern ID")
                })?;
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .ok_or_else(|| self.invariant(span, "container style extern disappeared"))?;
                if declaration.name != style.function
                    || declaration.kind != ExternKind::ContainerStyle
                    || declaration.params.len() != style.args.len()
                {
                    return Err(self.invariant(span, "container style extern contract diverged"));
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "style argument"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedContainerCustomStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        if custom_style.is_none() != checked.style.is_none() {
            return Err(self.invariant(span, "container custom style presence diverged"));
        }
        let surface = self.resolve_container_surface(&mut values, &options.style, span)?;
        let border_dash = options
            .border_dash
            .iter()
            .map(|_| values.take(&Type::F64, "border dash"))
            .collect::<Result<Vec<_>, _>>()?;
        let flex_item = Self::resolve_container_flex_item(&mut values, &options.flex_item)?;
        values.finish(checked.expression_count)?;

        let alignment = |value| match value {
            FlexAlignment::Start => ResolvedContainerAlignment::Start,
            FlexAlignment::Center => ResolvedContainerAlignment::Center,
            FlexAlignment::End => ResolvedContainerAlignment::End,
        };
        let utility_style = self
            .styles
            .style_use(span)
            .map(|style| style.style.clone())
            .map_err(|_| self.invariant(span, "container utility style site is not normalized"))?;
        let resolved = ResolvedContainer {
            id,
            padding,
            width,
            height,
            max_width,
            max_height,
            align_x: options.align_x.map(alignment),
            align_y: options.align_y.map(alignment),
            clip,
            custom_style,
            surface,
            border_dash,
            flex_item,
            utility_style,
            origin,
        };
        if self.containers.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "container was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_container_length(
        values: &mut ContainerOperands<'_>,
        value: &Option<LengthValue>,
        label: &str,
    ) -> Result<Option<ResolvedContainerLength>, Error> {
        Ok(match value {
            None => None,
            Some(LengthValue::Fill) => Some(ResolvedContainerLength::Fill),
            Some(LengthValue::FillPortion(portion)) => {
                Some(ResolvedContainerLength::FillPortion(*portion))
            }
            Some(LengthValue::Shrink) => Some(ResolvedContainerLength::Shrink),
            Some(LengthValue::Fixed(_)) => {
                let (expression, source) = values
                    .take_where(label, |actual| matches!(actual, Type::F64 | Type::Length))?;
                Some(match source {
                    Type::F64 => ResolvedContainerLength::FixedF64(expression),
                    Type::Length => ResolvedContainerLength::FixedLength(expression),
                    _ => unreachable!("validated container length type"),
                })
            }
        })
    }

    fn resolve_container_surface(
        &self,
        values: &mut ContainerOperands<'_>,
        style: &ContainerStyleOptions,
        span: &Span,
    ) -> Result<ResolvedContainerSurface, Error> {
        let background = style
            .background
            .as_ref()
            .map(|background| {
                Ok(match background {
                    BackgroundValue::Color(color) => {
                        ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
                    }
                    BackgroundValue::Linear { stops, .. } => {
                        let angle = values.take(&Type::F64, "background angle")?;
                        let stops = stops
                            .iter()
                            .map(|stop| {
                                Ok(ResolvedContainerGradientStop {
                                    color: self.resolve_theme_color(&stop.color, span)?,
                                    offset: values.take(&Type::F64, "background stop")?,
                                })
                            })
                            .collect::<Result<Vec<_>, Error>>()?;
                        ResolvedContainerBackground::Linear { angle, stops }
                    }
                })
            })
            .transpose()?;
        let border_width =
            values.optional(style.border_width.as_ref(), &Type::F64, "border width")?;
        let radius = ResolvedContainerRadius {
            all: values.optional(style.radius.as_ref(), &Type::F64, "radius")?,
            top_left: values.optional(
                style.radius_top_left.as_ref(),
                &Type::F64,
                "radius top-left",
            )?,
            top_right: values.optional(
                style.radius_top_right.as_ref(),
                &Type::F64,
                "radius top-right",
            )?,
            bottom_right: values.optional(
                style.radius_bottom_right.as_ref(),
                &Type::F64,
                "radius bottom-right",
            )?,
            bottom_left: values.optional(
                style.radius_bottom_left.as_ref(),
                &Type::F64,
                "radius bottom-left",
            )?,
        };
        Ok(ResolvedContainerSurface {
            background,
            text_color: style
                .text_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_color: style
                .border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_width,
            radius,
            shadow_color: style
                .shadow_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            shadow_x: values.optional(style.shadow_x.as_ref(), &Type::F64, "shadow x")?,
            shadow_y: values.optional(style.shadow_y.as_ref(), &Type::F64, "shadow y")?,
            shadow_blur: values.optional(style.shadow_blur.as_ref(), &Type::F64, "shadow blur")?,
            pixel_snap: values.optional(style.pixel_snap.as_ref(), &Type::Bool, "pixel snap")?,
        })
    }

    fn resolve_container_flex_item(
        values: &mut ContainerOperands<'_>,
        item: &FlexItemOptions,
    ) -> Result<ResolvedContainerFlexItem, Error> {
        let order = values.optional(item.order.as_ref(), &Type::I64, "flex order")?;
        let grow = values.optional(item.grow.as_ref(), &Type::F64, "flex grow")?;
        let shrink = values.optional(item.shrink.as_ref(), &Type::F64, "flex shrink")?;
        let basis = match &item.basis {
            None => None,
            Some(FlexBasisValue::Auto) => Some(ResolvedContainerFlexBasis::Auto),
            Some(FlexBasisValue::Content) => Some(ResolvedContainerFlexBasis::Content),
            Some(FlexBasisValue::Fixed(_)) => Some(ResolvedContainerFlexBasis::Fixed(
                values.take(&Type::F64, "flex basis")?,
            )),
            Some(FlexBasisValue::Percent(_)) => Some(ResolvedContainerFlexBasis::Percent(
                values.take(&Type::F64, "flex basis")?,
            )),
        };
        let mut margin = |value: &Option<FlexMarginValue>| {
            Ok(match value {
                None => None,
                Some(FlexMarginValue::Auto) => Some(ResolvedContainerFlexMargin::Auto),
                Some(FlexMarginValue::Fixed(_)) => Some(ResolvedContainerFlexMargin::Fixed(
                    values.take(&Type::F64, "flex margin")?,
                )),
                Some(FlexMarginValue::Percent(_)) => Some(ResolvedContainerFlexMargin::Percent(
                    values.take(&Type::F64, "flex margin")?,
                )),
            })
        };
        let all = margin(&item.margin.all)?;
        let x = margin(&item.margin.x)?;
        let y = margin(&item.margin.y)?;
        let top = margin(&item.margin.top)?;
        let right = margin(&item.margin.right)?;
        let bottom = margin(&item.margin.bottom)?;
        let left = margin(&item.margin.left)?;
        let margin_present = [
            &item.margin.all,
            &item.margin.x,
            &item.margin.y,
            &item.margin.top,
            &item.margin.right,
            &item.margin.bottom,
            &item.margin.left,
        ]
        .into_iter()
        .any(Option::is_some);
        let choose = |side: &Option<ResolvedContainerFlexMargin>,
                      axis: &Option<ResolvedContainerFlexMargin>| {
            side.clone()
                .or_else(|| axis.clone())
                .or_else(|| all.clone())
                .unwrap_or(ResolvedContainerFlexMargin::Zero)
        };
        let margins = margin_present.then(|| ResolvedContainerFlexMargins {
            top: choose(&top, &y),
            right: choose(&right, &x),
            bottom: choose(&bottom, &y),
            left: choose(&left, &x),
        });
        let align_self = item.align_self.map(|align| match align {
            FlexItemAlignment::Start => ResolvedContainerFlexAlignment::Start,
            FlexItemAlignment::End => ResolvedContainerFlexAlignment::End,
            FlexItemAlignment::FlexStart => ResolvedContainerFlexAlignment::FlexStart,
            FlexItemAlignment::FlexEnd => ResolvedContainerFlexAlignment::FlexEnd,
            FlexItemAlignment::Center => ResolvedContainerFlexAlignment::Center,
            FlexItemAlignment::Baseline => ResolvedContainerFlexAlignment::Baseline,
            FlexItemAlignment::Stretch => ResolvedContainerFlexAlignment::Stretch,
        });
        Ok(ResolvedContainerFlexItem {
            order,
            grow,
            shrink,
            basis,
            align_self,
            margins,
        })
    }
}
