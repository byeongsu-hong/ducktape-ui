use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedMediaKind {
    Image,
    Svg,
    Viewer,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedMediaLength {
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed {
        expression: CheckedExprUseId,
        source: Type,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedMediaFilter {
    Linear,
    Nearest,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMediaSvgColors {
    pub(crate) idle: Option<ResolvedThemeColor>,
    pub(crate) hovered: Option<Option<ResolvedThemeColor>>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMediaSvgStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedMediaRadius {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) top_left: Option<CheckedExprUseId>,
    pub(crate) top_right: Option<CheckedExprUseId>,
    pub(crate) bottom_right: Option<CheckedExprUseId>,
    pub(crate) bottom_left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedMediaScaleBound {
    Default(f64),
    Expression(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMediaScaleBounds {
    pub(crate) minimum: ResolvedMediaScaleBound,
    pub(crate) maximum: ResolvedMediaScaleBound,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMediaOptions {
    pub(crate) accessibility_label: Option<CheckedExprUseId>,
    pub(crate) accessibility_description: Option<CheckedExprUseId>,
    pub(crate) width: Option<ResolvedMediaLength>,
    pub(crate) height: Option<ResolvedMediaLength>,
    pub(crate) fit: Option<CheckedExprUseId>,
    pub(crate) rotation: Option<CheckedExprUseId>,
    pub(crate) opacity: Option<CheckedExprUseId>,
    pub(crate) svg_memory: bool,
    pub(crate) svg_colors: Option<ResolvedMediaSvgColors>,
    pub(crate) svg_style: Option<ResolvedMediaSvgStyle>,
    pub(crate) filter: Option<ResolvedMediaFilter>,
    pub(crate) scale: Option<CheckedExprUseId>,
    pub(crate) expand: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedMediaRadius,
    pub(crate) crop: Option<[CheckedExprUseId; 4]>,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) scale_bounds: Option<ResolvedMediaScaleBounds>,
    pub(crate) scale_step: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMedia {
    pub(crate) id: ViewId,
    pub(crate) kind: ResolvedMediaKind,
    pub(crate) source: CheckedExprUseId,
    pub(crate) source_type: Type,
    pub(crate) options: ResolvedMediaOptions,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_media(
        &mut self,
        kind: MediaKind,
        source: &Expr,
        options: &MediaOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "media has no shared view ID"))?;
        let checked = self
            .facts
            .media(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "media has no checked HIR facts"))?;
        let checked_view = self.facts.view(id);
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        let roots = crate::ast::media_expression_roots(source, options);
        if checked.id != id
            || expected_scope != outer_component
            || checked.semantic_key != crate::ast::media_semantic_key(kind, options)
            || checked.expression_count as usize != roots.len()
        {
            return Err(self.invariant(span, "media topology diverged after semantic checking"));
        }
        self.validate_media_style(&checked, options, span)?;
        self.validate_media_expression_graphs(
            id,
            checked_view.scope,
            checked.expression_count,
            span,
        )?;

        let mut expression = 0u32;
        let source = self.take_media_expression(id, &mut expression, span)?;
        let source_type = self.facts.expression_use(source).source.clone();
        let valid_source = match kind {
            MediaKind::Image | MediaKind::Viewer => {
                matches!(&source_type, Type::Str | Type::Image)
            }
            MediaKind::Svg if options.svg_memory => {
                matches!(&source_type, Type::Str | Type::Bytes)
            }
            MediaKind::Svg => source_type == Type::Str,
        };
        if !valid_source {
            return Err(self.invariant(span, "media source type diverged after semantic checking"));
        }
        let accessibility_label = self.take_optional_media_expression(
            options.accessibility.label.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        let accessibility_description = self.take_optional_media_expression(
            options.accessibility.description.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        self.require_media_optional_type(accessibility_label, &Type::Str, span)?;
        self.require_media_optional_type(accessibility_description, &Type::Str, span)?;
        let width = self.lower_media_length(&options.width, id, &mut expression, span)?;
        let height = self.lower_media_length(&options.height, id, &mut expression, span)?;
        let fit =
            self.take_optional_media_expression(options.fit.as_ref(), id, &mut expression, span)?;
        let rotation = self.take_optional_media_expression(
            options.rotation.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        let opacity = self.take_optional_media_expression(
            options.opacity.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        self.require_media_optional_type(fit, &Type::ContentFit, span)?;
        self.require_media_optional_type(rotation, &Type::Rotation, span)?;
        self.require_media_optional_type(opacity, &Type::F64, span)?;
        let svg_style = options
            .svg_style
            .as_ref()
            .map(|style| {
                let function = checked.style.ok_or_else(|| {
                    self.invariant(span, "media style lost its checked extern ID")
                })?;
                let arguments = style
                    .args
                    .iter()
                    .map(|_| self.take_media_expression(id, &mut expression, span))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedMediaSvgStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        let scale =
            self.take_optional_media_expression(options.scale.as_ref(), id, &mut expression, span)?;
        let expand = self.take_optional_media_expression(
            options.expand.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        self.require_media_optional_type(scale, &Type::F64, span)?;
        self.require_media_optional_type(expand, &Type::Bool, span)?;
        let radius = ResolvedMediaRadius {
            all: self.take_optional_media_expression(
                options.radius.as_ref(),
                id,
                &mut expression,
                span,
            )?,
            top_left: self.take_optional_media_expression(
                options.radius_top_left.as_ref(),
                id,
                &mut expression,
                span,
            )?,
            top_right: self.take_optional_media_expression(
                options.radius_top_right.as_ref(),
                id,
                &mut expression,
                span,
            )?,
            bottom_right: self.take_optional_media_expression(
                options.radius_bottom_right.as_ref(),
                id,
                &mut expression,
                span,
            )?,
            bottom_left: self.take_optional_media_expression(
                options.radius_bottom_left.as_ref(),
                id,
                &mut expression,
                span,
            )?,
        };
        let crop = options
            .crop
            .as_ref()
            .map(|_| {
                Ok([
                    self.take_media_expression(id, &mut expression, span)?,
                    self.take_media_expression(id, &mut expression, span)?,
                    self.take_media_expression(id, &mut expression, span)?,
                    self.take_media_expression(id, &mut expression, span)?,
                ])
            })
            .transpose()?;
        for value in crop.iter().flatten() {
            self.require_media_type(*value, &Type::I64, span)?;
        }
        let padding = self.take_optional_media_expression(
            options.padding.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        let minimum = self.take_optional_media_expression(
            options.min_scale.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        let maximum = self.take_optional_media_expression(
            options.max_scale.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        let scale_bounds = if minimum.is_some() || maximum.is_some() {
            Some(ResolvedMediaScaleBounds {
                minimum: minimum.map_or(
                    ResolvedMediaScaleBound::Default(0.25),
                    ResolvedMediaScaleBound::Expression,
                ),
                maximum: maximum.map_or(
                    ResolvedMediaScaleBound::Default(10.0),
                    ResolvedMediaScaleBound::Expression,
                ),
            })
        } else {
            None
        };
        let scale_step = self.take_optional_media_expression(
            options.scale_step.as_ref(),
            id,
            &mut expression,
            span,
        )?;
        for value in [
            radius.all,
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
            padding,
            minimum,
            maximum,
            scale_step,
        ]
        .into_iter()
        .flatten()
        {
            self.require_media_type(value, &Type::F64, span)?;
        }
        if let Some(style) = &svg_style {
            let declaration = self
                .declarations
                .try_extern_decl(style.function)
                .ok_or_else(|| self.invariant(span, "media style extern disappeared"))?;
            for (argument, (_, expected)) in style.arguments.iter().zip(&declaration.params) {
                self.require_media_type(*argument, expected, span)?;
            }
        }
        if expression != checked.expression_count {
            return Err(self.invariant(span, "media left checked expressions unconsumed"));
        }

        let idle = options
            .svg_color
            .as_deref()
            .map(|color| self.resolve_theme_color(color, span))
            .transpose()?;
        let hovered = match &options.svg_hover_color {
            None => idle.clone().map(Some),
            Some(None) => Some(None),
            Some(Some(color)) => Some(Some(self.resolve_theme_color(color, span)?)),
        };
        let svg_colors = (options.svg_color.is_some() || options.svg_hover_color.is_some())
            .then_some(ResolvedMediaSvgColors { idle, hovered });
        let resolved = ResolvedMedia {
            id,
            kind: match kind {
                MediaKind::Image => ResolvedMediaKind::Image,
                MediaKind::Svg => ResolvedMediaKind::Svg,
                MediaKind::Viewer => ResolvedMediaKind::Viewer,
            },
            source,
            source_type,
            options: ResolvedMediaOptions {
                accessibility_label,
                accessibility_description,
                width,
                height,
                fit,
                rotation,
                opacity,
                svg_memory: options.svg_memory,
                svg_colors,
                svg_style,
                filter: options.filter.map(|filter| match filter {
                    ImageFilter::Linear => ResolvedMediaFilter::Linear,
                    ImageFilter::Nearest => ResolvedMediaFilter::Nearest,
                }),
                scale,
                expand,
                radius,
                crop,
                padding,
                scale_bounds,
                scale_step,
            },
            origin: checked_view.origin,
        };
        if self.media.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "media was lowered more than once"));
        }
        Ok(())
    }

    fn validate_media_style(
        &self,
        checked: &CheckedMedia,
        options: &MediaOptions,
        span: &Span,
    ) -> Result<(), Error> {
        match (&options.svg_style, checked.style) {
            (None, None) => Ok(()),
            (Some(style), Some(id)) => {
                let declaration = self.declarations.try_extern_decl(id).ok_or_else(|| {
                    self.invariant(span, "media style references an invalid extern ID")
                })?;
                if declaration.kind != ExternKind::SvgStyle
                    || declaration.name != style.function
                    || declaration.params.len() != style.args.len()
                {
                    return Err(
                        self.invariant(span, "media style contract diverged after checking")
                    );
                }
                Ok(())
            }
            _ => Err(self.invariant(span, "media style presence diverged after checking")),
        }
    }

    fn validate_media_expression_graphs(
        &self,
        media: ViewId,
        scope: CheckedViewScope,
        count: u32,
        span: &Span,
    ) -> Result<(), Error> {
        let mut graph = CheckedExpressionGraph::default();
        for index in 0..count {
            let owner = CheckedExprOwner::Media(MediaExpressionId { media, index });
            let use_id = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "media expression has no checked owner mapping")
            })?;
            let expression = self.facts.try_expression_use(use_id).ok_or_else(|| {
                self.invariant(span, "media expression-use ID is outside its arena")
            })?;
            if expression.owner != owner {
                return Err(self.invariant(span, "media expression owner mapping diverged"));
            }
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: media,
                scope,
                use_id,
                span,
                canvas_locals: false,
                own_view_locals: false,
                allowed_own_view_locals: None,
                family: "media",
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
                    self.invariant(span, "media expression type or coercion contract diverged")
                );
            }
        }
        Ok(())
    }

    fn lower_media_length(
        &self,
        value: &Option<LengthValue>,
        media: ViewId,
        expression: &mut u32,
        span: &Span,
    ) -> Result<Option<ResolvedMediaLength>, Error> {
        value
            .as_ref()
            .map(|value| {
                Ok(match value {
                    LengthValue::Fill => ResolvedMediaLength::Fill,
                    LengthValue::FillPortion(value) => ResolvedMediaLength::FillPortion(*value),
                    LengthValue::Shrink => ResolvedMediaLength::Shrink,
                    LengthValue::Fixed(_) => {
                        let expression = self.take_media_expression(media, expression, span)?;
                        let source = self.facts.expression_use(expression).source.clone();
                        if !matches!(source, Type::F64 | Type::Length) {
                            return Err(
                                self.invariant(span, "media length type diverged after checking")
                            );
                        }
                        ResolvedMediaLength::Fixed { expression, source }
                    }
                })
            })
            .transpose()
    }

    fn take_optional_media_expression<T>(
        &self,
        value: Option<&T>,
        media: ViewId,
        expression: &mut u32,
        span: &Span,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        value
            .map(|_| self.take_media_expression(media, expression, span))
            .transpose()
    }

    fn require_media_optional_type(
        &self,
        expression: Option<CheckedExprUseId>,
        expected: &Type,
        span: &Span,
    ) -> Result<(), Error> {
        expression
            .map(|expression| self.require_media_type(expression, expected, span))
            .transpose()
            .map(|_| ())
    }

    fn require_media_type(
        &self,
        expression: CheckedExprUseId,
        expected: &Type,
        span: &Span,
    ) -> Result<(), Error> {
        let retained = self.facts.expression_use(expression);
        if &retained.source != expected || &retained.destination != expected {
            return Err(self.invariant(span, "media option type diverged after semantic checking"));
        }
        Ok(())
    }

    fn take_media_expression(
        &self,
        media: ViewId,
        index: &mut u32,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Media(MediaExpressionId {
            media,
            index: *index,
        });
        *index += 1;
        let expression = self
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| self.invariant(span, "media expression has no checked owner"))?;
        let retained = self
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| self.invariant(span, "media expression-use ID is outside its arena"))?;
        if retained.owner != owner || self.facts.try_expression(retained.root).is_none() {
            return Err(self.invariant(span, "media expression graph has an invalid owner"));
        }
        Ok(expression)
    }
}
