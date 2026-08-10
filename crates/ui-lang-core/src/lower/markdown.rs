use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMarkdownState {
    pub(crate) id: CheckedValueRef,
    pub(crate) name: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMarkdownViewer {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
    pub(crate) borrowed: Vec<bool>,
    pub(crate) output: Type,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedMarkdownStyle {
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) inline_code_background: Option<ResolvedContainerBackground>,
    pub(crate) inline_code_color: Option<ResolvedThemeColor>,
    pub(crate) inline_code_font: Option<ResolvedTextFont>,
    pub(crate) code_block_font: Option<ResolvedTextFont>,
    pub(crate) link_color: Option<ResolvedThemeColor>,
    pub(crate) inline_code_padding: ResolvedContainerPadding,
    pub(crate) inline_code_border_color: Option<ResolvedThemeColor>,
    pub(crate) inline_code_border_width: Option<CheckedExprUseId>,
    pub(crate) inline_code_radius: ResolvedContainerRadius,
    #[cfg(test)]
    pub(crate) origin: Option<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMarkdown {
    pub(crate) id: ViewId,
    pub(crate) content: ResolvedMarkdownState,
    pub(crate) text_size: Option<CheckedExprUseId>,
    pub(crate) h1_size: Option<CheckedExprUseId>,
    pub(crate) h2_size: Option<CheckedExprUseId>,
    pub(crate) h3_size: Option<CheckedExprUseId>,
    pub(crate) h4_size: Option<CheckedExprUseId>,
    pub(crate) h5_size: Option<CheckedExprUseId>,
    pub(crate) h6_size: Option<CheckedExprUseId>,
    pub(crate) code_size: Option<CheckedExprUseId>,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) viewer: Option<ResolvedMarkdownViewer>,
    pub(crate) style: ResolvedMarkdownStyle,
    pub(crate) link: ResolvedInteractionRoute,
    pub(crate) origin: OriginId,
}

struct MarkdownOperands<'a> {
    lowerer: &'a Lowerer,
    widget: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    parent: OriginId,
    style_origin: Option<OriginId>,
    span: &'a Span,
}

impl MarkdownOperands<'_> {
    fn take(
        &mut self,
        expected: &Type,
        label: &str,
        in_style: bool,
    ) -> Result<CheckedExprUseId, Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                format!("markdown {label} expression disappeared"),
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
                    format!("markdown {label} expression ID is invalid"),
                )
            })?;
        let expression_origin = self
            .lowerer
            .origins
            .try_get(retained.origin)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, format!("markdown {label} origin is invalid"))
            })?;
        let expected_origin = self
            .lowerer
            .origins
            .try_get(if in_style {
                self.style_origin.ok_or_else(|| {
                    self.lowerer
                        .invariant(self.span, "markdown style expression has no style origin")
                })?
            } else {
                self.parent
            })
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, "markdown origin is invalid")
            })?;
        if retained.owner != owner
            || self.lowerer.facts.expression_use_by_owner(owner) != Some(expression)
            || retained.source != *expected
            || retained.destination != *expected
            || self.lowerer.facts.try_expression(retained.root).is_none()
            || expression_origin.parent != Some(self.parent)
            || expression_origin.path != expected_origin.path
            || expression_origin.line != expected_origin.line
            || expression_origin.column != expected_origin.column
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("markdown {label} expression contract diverged"),
            ));
        }
        Ok(expression)
    }

    fn optional<T>(
        &mut self,
        source: Option<&T>,
        expected: &Type,
        label: &str,
        in_style: bool,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        source
            .map(|_| self.take(expected, label, in_style))
            .transpose()
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.expressions.next().is_some() {
            return Err(self
                .lowerer
                .invariant(self.span, "markdown left checked expressions unconsumed"));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_markdown(
        &mut self,
        content: &str,
        options: &MarkdownOptions,
        route: &Route,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, interaction, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Markdown,
            crate::ast::markdown_semantic_key(options),
            span,
            outer_component,
        )?;
        let checked = self
            .facts
            .markdown(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "markdown has no checked HIR facts"))?;
        if checked.id != id || interaction.routes.len() != 1 {
            return Err(self.invariant(span, "markdown checked identity or route count diverged"));
        }
        self.validate_interaction_expression_graphs(id, scope, interaction.expression_count, span)?;
        let content =
            self.resolve_markdown_state(checked.content, content, outer_component, span)?;
        let style_origin = self.validate_markdown_style_origin(&checked, options, origin, span)?;
        let mut values = MarkdownOperands {
            lowerer: self,
            widget: id,
            expressions: interaction.option_expressions.iter(),
            next: 0,
            parent: origin,
            style_origin,
            span,
        };
        let text_size =
            values.optional(options.text_size.as_ref(), &Type::F64, "text size", false)?;
        let h1_size = values.optional(options.h1_size.as_ref(), &Type::F64, "h1 size", false)?;
        let h2_size = values.optional(options.h2_size.as_ref(), &Type::F64, "h2 size", false)?;
        let h3_size = values.optional(options.h3_size.as_ref(), &Type::F64, "h3 size", false)?;
        let h4_size = values.optional(options.h4_size.as_ref(), &Type::F64, "h4 size", false)?;
        let h5_size = values.optional(options.h5_size.as_ref(), &Type::F64, "h5 size", false)?;
        let h6_size = values.optional(options.h6_size.as_ref(), &Type::F64, "h6 size", false)?;
        let code_size =
            values.optional(options.code_size.as_ref(), &Type::F64, "code size", false)?;
        let spacing = values.optional(options.spacing.as_ref(), &Type::F64, "spacing", false)?;
        let style =
            self.resolve_markdown_style(&mut values, &options.style, style_origin, origin, span)?;
        let viewer =
            self.resolve_markdown_viewer(&mut values, options.viewer.as_ref(), &checked, span)?;
        values.finish()?;
        let mut route_index = 0;
        let link = self.lower_required_interaction_route(
            route,
            &interaction,
            &[route],
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != interaction.routes.len()
            || link.source_payloads != vec![checked.viewer_output.clone()]
        {
            return Err(self.invariant(span, "markdown link payload contract diverged"));
        }
        let resolved = ResolvedMarkdown {
            id,
            content,
            text_size,
            h1_size,
            h2_size,
            h3_size,
            h4_size,
            h5_size,
            h6_size,
            code_size,
            spacing,
            viewer,
            style,
            link,
            origin,
        };
        if self.markdowns.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "markdown was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_markdown_state(
        &self,
        content: CheckedValueRef,
        expected_name: &str,
        outer_component: Option<ComponentId>,
        span: &Span,
    ) -> Result<ResolvedMarkdownState, Error> {
        let value = self
            .facts
            .try_value_by_ref(content)
            .ok_or_else(|| self.invariant(span, "markdown content value ID is invalid"))?;
        if value.name != expected_name || value.ty != Type::Markdown {
            return Err(self.invariant(span, "markdown content identity or type diverged"));
        }
        let valid_scope = match content {
            CheckedValueRef::Secret(_) => false,
            CheckedValueRef::AppState(_) | CheckedValueRef::Derived(_) => outer_component.is_none(),
            CheckedValueRef::ComponentParam(id) => outer_component == Some(id.component),
            CheckedValueRef::ComponentState(id) => outer_component == Some(id.component),
        };
        if !valid_scope {
            return Err(self.invariant(span, "markdown content scope diverged"));
        }
        Ok(ResolvedMarkdownState {
            id: content,
            name: value.name.clone(),
        })
    }

    fn validate_markdown_style_origin(
        &self,
        checked: &CheckedMarkdown,
        options: &MarkdownOptions,
        parent: OriginId,
        span: &Span,
    ) -> Result<Option<OriginId>, Error> {
        match (checked.style_origin, options.style.span.as_ref()) {
            (None, None) => Ok(None),
            (Some(origin), Some(_)) => {
                let retained = self.origins.try_get(origin).ok_or_else(|| {
                    self.invariant(span, "markdown style origin is outside its arena")
                })?;
                if retained.parent != Some(parent) {
                    return Err(self.invariant(span, "markdown style origin parent diverged"));
                }
                Ok(Some(origin))
            }
            _ => Err(self.invariant(span, "markdown style origin presence diverged")),
        }
    }

    fn resolve_markdown_style(
        &self,
        values: &mut MarkdownOperands<'_>,
        style: &MarkdownStyleOptions,
        style_origin: Option<OriginId>,
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedMarkdownStyle, Error> {
        let origin = style_origin.unwrap_or(parent);
        let inline_code_background = style
            .inline_code_background
            .as_ref()
            .map(|background| self.resolve_markdown_background(values, background, span))
            .transpose()?;
        Ok(ResolvedMarkdownStyle {
            font: self.resolve_text_font(style.font.as_ref(), origin, span)?,
            inline_code_background,
            inline_code_color: style
                .inline_code_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            inline_code_font: self.resolve_text_font(
                style.inline_code_font.as_ref(),
                origin,
                span,
            )?,
            code_block_font: self.resolve_text_font(
                style.code_block_font.as_ref(),
                origin,
                span,
            )?,
            link_color: style
                .link_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            inline_code_padding: ResolvedContainerPadding {
                all: values.optional(
                    style.inline_code_padding.all.as_ref(),
                    &Type::F64,
                    "inline code padding",
                    true,
                )?,
                x: values.optional(
                    style.inline_code_padding.x.as_ref(),
                    &Type::F64,
                    "inline code horizontal padding",
                    true,
                )?,
                y: values.optional(
                    style.inline_code_padding.y.as_ref(),
                    &Type::F64,
                    "inline code vertical padding",
                    true,
                )?,
                top: values.optional(
                    style.inline_code_padding.top.as_ref(),
                    &Type::F64,
                    "inline code top padding",
                    true,
                )?,
                right: values.optional(
                    style.inline_code_padding.right.as_ref(),
                    &Type::F64,
                    "inline code right padding",
                    true,
                )?,
                bottom: values.optional(
                    style.inline_code_padding.bottom.as_ref(),
                    &Type::F64,
                    "inline code bottom padding",
                    true,
                )?,
                left: values.optional(
                    style.inline_code_padding.left.as_ref(),
                    &Type::F64,
                    "inline code left padding",
                    true,
                )?,
            },
            inline_code_border_color: style
                .inline_code_border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            inline_code_border_width: values.optional(
                style.inline_code_border_width.as_ref(),
                &Type::F64,
                "inline code border width",
                true,
            )?,
            inline_code_radius: ResolvedContainerRadius {
                all: values.optional(
                    style.inline_code_radius.as_ref(),
                    &Type::F64,
                    "inline code radius",
                    true,
                )?,
                top_left: values.optional(
                    style.inline_code_radius_top_left.as_ref(),
                    &Type::F64,
                    "inline code top-left radius",
                    true,
                )?,
                top_right: values.optional(
                    style.inline_code_radius_top_right.as_ref(),
                    &Type::F64,
                    "inline code top-right radius",
                    true,
                )?,
                bottom_right: values.optional(
                    style.inline_code_radius_bottom_right.as_ref(),
                    &Type::F64,
                    "inline code bottom-right radius",
                    true,
                )?,
                bottom_left: values.optional(
                    style.inline_code_radius_bottom_left.as_ref(),
                    &Type::F64,
                    "inline code bottom-left radius",
                    true,
                )?,
            },
            #[cfg(test)]
            origin: style_origin,
        })
    }

    fn resolve_markdown_background(
        &self,
        values: &mut MarkdownOperands<'_>,
        background: &BackgroundValue,
        span: &Span,
    ) -> Result<ResolvedContainerBackground, Error> {
        Ok(match background {
            BackgroundValue::Color(color) => {
                ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
            }
            BackgroundValue::Linear { stops, .. } => {
                let angle = values.take(&Type::F64, "inline code background angle", true)?;
                let stops = stops
                    .iter()
                    .map(|stop| {
                        Ok(ResolvedContainerGradientStop {
                            color: self.resolve_theme_color(&stop.color, span)?,
                            offset: values.take(&Type::F64, "inline code background stop", true)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                ResolvedContainerBackground::Linear { angle, stops }
            }
        })
    }

    fn resolve_markdown_viewer(
        &self,
        values: &mut MarkdownOperands<'_>,
        source: Option<&ExternCall>,
        checked: &CheckedMarkdown,
        span: &Span,
    ) -> Result<Option<ResolvedMarkdownViewer>, Error> {
        match (source, checked.viewer) {
            (None, None) => {
                if checked.viewer_output != Type::Str {
                    return Err(self.invariant(span, "default markdown viewer output diverged"));
                }
                Ok(None)
            }
            (Some(source), Some(function_id)) => {
                let function = self
                    .declarations
                    .try_extern_decl(function_id)
                    .filter(|function| {
                        function.kind == ExternKind::MarkdownViewer
                            && function.name == source.function
                            && function.output == checked.viewer_output
                            && function.params.len() == source.args.len()
                            && function.borrowed.len() == source.args.len()
                    })
                    .ok_or_else(|| self.invariant(span, "markdown viewer contract diverged"))?;
                let arguments = function
                    .params
                    .iter()
                    .map(|(_, ty)| values.take(ty, "viewer argument", false))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(ResolvedMarkdownViewer {
                    function: function_id,
                    arguments,
                    borrowed: function.borrowed.clone(),
                    output: function.output.clone(),
                }))
            }
            _ => Err(self.invariant(span, "markdown viewer presence diverged")),
        }
    }
}
