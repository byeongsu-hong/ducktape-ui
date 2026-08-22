use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTextAlignment {
    Default,
    Left,
    Center,
    Right,
    Justified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTextVerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTextShaping {
    Auto,
    Basic,
    Advanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTextWrapping {
    None,
    Word,
    Glyph,
    WordOrGlyph,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTextLineHeight {
    Relative(CheckedExprUseId),
    Absolute(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTextFont {
    Default,
    Monospace,
    Named(ResolvedDefaultFont),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTextCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTextOptions {
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<ResolvedTextLineHeight>,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) align_x: Option<ResolvedTextAlignment>,
    pub(crate) align_y: Option<ResolvedTextVerticalAlignment>,
    pub(crate) shaping: Option<ResolvedTextShaping>,
    pub(crate) wrapping: Option<ResolvedTextWrapping>,
    pub(crate) tracking: Option<f64>,
    pub(crate) custom_style: Option<ResolvedTextCustomStyle>,
    pub(crate) underline: Option<CheckedExprUseId>,
    pub(crate) strikethrough: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedRichSpan {
    pub(crate) value: CheckedExprUseId,
    pub(crate) size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<ResolvedTextLineHeight>,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) color: Option<ResolvedThemeColor>,
    pub(crate) link: Option<CheckedExprUseId>,
    pub(crate) background: Option<ResolvedContainerBackground>,
    pub(crate) border_color: Option<ResolvedThemeColor>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedContainerRadius,
    pub(crate) padding: ResolvedContainerPadding,
    pub(crate) underline: Option<CheckedExprUseId>,
    pub(crate) strikethrough: Option<CheckedExprUseId>,
    pub(crate) utility_style: ResolvedStyle,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

/// One resolved `rich-text` child: a literal span, or a `for` whose span
/// templates expand against the iterated items at render time — still inside
/// the same single paragraph widget.
#[derive(Clone, Debug)]
pub(crate) enum ResolvedRichChild {
    Span(Box<ResolvedRichSpan>),
    For(ResolvedRichFor),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedRichFor {
    pub(crate) items: CheckedExprUseId,
    pub(crate) item: ResolvedIterationBinding,
    pub(crate) spans: Vec<ResolvedRichSpan>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTextContent {
    Plain {
        value: CheckedExprUseId,
    },
    Rich {
        color: Option<ResolvedThemeColor>,
        children: Vec<ResolvedRichChild>,
        route: Option<ResolvedInteractionRoute>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedText {
    pub(crate) id: ViewId,
    pub(crate) options: ResolvedTextOptions,
    pub(crate) utility_style: ResolvedStyle,
    pub(crate) content: ResolvedTextContent,
    pub(crate) origin: OriginId,
}

struct TextOperands<'a> {
    lowerer: &'a Lowerer,
    text: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    span: &'a Span,
}

impl TextOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer
                .invariant(self.span, format!("text {label} expression disappeared"))
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.text,
            index: self.next,
        });
        self.next += 1;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, format!("text {label} expression ID is invalid"))
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("text {label} expression contract diverged"),
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

    fn finish(&mut self) -> Result<(), Error> {
        if self.expressions.next().is_some() {
            return Err(self
                .lowerer
                .invariant(self.span, "text left checked option expressions unconsumed"));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_text(
        &mut self,
        node: &ViewNode,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let no_route = None;
        let (kind, semantic_key, roots, options, route) = match node {
            ViewNode::Text { value, options, .. } => (
                CheckedInteractionKind::Text,
                crate::ast::text_semantic_key(options),
                crate::ast::text_expression_roots(value, options),
                options,
                &no_route,
            ),
            ViewNode::RichText {
                options,
                color,
                children,
                route,
                ..
            } => (
                CheckedInteractionKind::RichText,
                crate::ast::rich_text_semantic_key(options, color, children, route),
                crate::ast::rich_text_expression_roots(options, children),
                options,
                route,
            ),
            _ => return Err(self.invariant(span, "non-text node reached text lowering")),
        };
        let (id, checked, scope, origin) =
            self.interaction_contract(kind, semantic_key, span, outer_component)?;
        let checked_text = self
            .facts
            .text(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "text has no checked HIR facts"))?;
        let has_scoped_locals = checked_text
            .rich_children
            .iter()
            .any(|child| matches!(child, CheckedRichChild::For { .. }));
        if has_scoped_locals {
            let contracts =
                self.rich_text_local_contracts(&checked_text, checked.expression_count, span)?;
            self.validate_interaction_expression_graphs_with_local_contracts(
                id,
                scope,
                checked.expression_count,
                &contracts,
                span,
            )?;
        } else {
            self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        }
        if checked.option_expressions.len() != roots.len() {
            return Err(self.invariant(span, "text expression cardinality diverged"));
        }
        let mut values = TextOperands {
            lowerer: self,
            text: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };

        let plain_value = match node {
            ViewNode::Text { .. } => Some(
                values
                    .take_where("value", |actual| {
                        matches!(actual, Type::Str | Type::I64 | Type::F64)
                    })?
                    .0,
            ),
            ViewNode::RichText { .. } => None,
            _ => unreachable!(),
        };
        let resolved_options =
            self.resolve_text_options(&mut values, options, &checked_text, origin, span)?;
        let content = match node {
            ViewNode::Text { .. } => ResolvedTextContent::Plain {
                value: plain_value.expect("plain text value"),
            },
            ViewNode::RichText {
                color, children, ..
            } => {
                if checked_text.rich_children.len() != children.len() {
                    return Err(self.invariant(span, "rich-text child count diverged"));
                }
                let mut resolved_children = Vec::with_capacity(children.len());
                for (index, (child, checked_child)) in
                    children.iter().zip(&checked_text.rich_children).enumerate()
                {
                    resolved_children.push(self.resolve_rich_child(
                        &mut values,
                        child,
                        checked_child,
                        index,
                        id,
                        origin,
                        span,
                    )?);
                }
                ResolvedTextContent::Rich {
                    color: color
                        .as_deref()
                        .map(|color| self.resolve_theme_color(color, span))
                        .transpose()?,
                    children: resolved_children,
                    route: None,
                }
            }
            _ => unreachable!(),
        };
        values.finish()?;

        let routes = route.iter().collect::<Vec<_>>();
        let mut route_index = 0usize;
        let resolved_route = self.lower_optional_interaction_route(
            route,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != checked.routes.len() {
            return Err(self.invariant(span, "text left checked routes unconsumed"));
        }
        let content = match content {
            ResolvedTextContent::Rich {
                color, children, ..
            } => ResolvedTextContent::Rich {
                color,
                children,
                route: resolved_route,
            },
            content => {
                if resolved_route.is_some() {
                    return Err(self.invariant(span, "plain text unexpectedly retained a route"));
                }
                content
            }
        };
        let utility_style = self
            .styles
            .style_use(span)
            .map(|style| style.style.clone())
            .map_err(|_| self.invariant(span, "text utility style site is not normalized"))?;
        let resolved = ResolvedText {
            id,
            options: resolved_options,
            utility_style,
            content,
            origin,
        };
        if self.texts.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "text was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_text_options(
        &self,
        values: &mut TextOperands<'_>,
        options: &TextOptions,
        checked: &CheckedText,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedTextOptions, Error> {
        let width = Self::resolve_text_length(values, &options.width, "width")?;
        let height = Self::resolve_text_length(values, &options.height, "height")?;
        let size = values.optional(options.size.as_ref(), &Type::F64, "size")?;
        let line_height = Self::resolve_text_line_height(values, &options.line_height)?;
        let custom_style = options
            .custom_style
            .as_ref()
            .map(|style| {
                let function = checked.style.ok_or_else(|| {
                    self.invariant(span, "text custom style lost its checked extern ID")
                })?;
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .ok_or_else(|| self.invariant(span, "text style extern disappeared"))?;
                if declaration.name != style.function
                    || declaration.kind != ExternKind::TextStyle
                    || declaration.params.len() != style.args.len()
                {
                    return Err(self.invariant(span, "text style extern contract diverged"));
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "style argument"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedTextCustomStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        if custom_style.is_none() != checked.style.is_none() {
            return Err(self.invariant(span, "text custom style presence diverged"));
        }
        let underline = values.optional(options.underline.as_ref(), &Type::Bool, "underline")?;
        let strikethrough =
            values.optional(options.strikethrough.as_ref(), &Type::Bool, "strike")?;
        Ok(ResolvedTextOptions {
            width,
            height,
            size,
            line_height,
            font: self.resolve_text_font(options.font.as_ref(), origin, span)?,
            align_x: options.align_x.map(Self::resolve_text_alignment),
            align_y: options.align_y.map(|alignment| match alignment {
                VerticalAlignment::Top => ResolvedTextVerticalAlignment::Top,
                VerticalAlignment::Center => ResolvedTextVerticalAlignment::Center,
                VerticalAlignment::Bottom => ResolvedTextVerticalAlignment::Bottom,
            }),
            shaping: options.shaping.map(|shaping| match shaping {
                TextShaping::Auto => ResolvedTextShaping::Auto,
                TextShaping::Basic => ResolvedTextShaping::Basic,
                TextShaping::Advanced => ResolvedTextShaping::Advanced,
            }),
            wrapping: options.wrapping.map(|wrapping| match wrapping {
                TextWrapping::None => ResolvedTextWrapping::None,
                TextWrapping::Word => ResolvedTextWrapping::Word,
                TextWrapping::Glyph => ResolvedTextWrapping::Glyph,
                TextWrapping::WordOrGlyph => ResolvedTextWrapping::WordOrGlyph,
            }),
            tracking: options.tracking,
            custom_style,
            underline,
            strikethrough,
        })
    }

    /// Every rich-text expression may read no own-view local, except the
    /// expressions a `for` child checked under its loop scope, which may read
    /// exactly that child's item local.
    fn rich_text_local_contracts(
        &self,
        checked_text: &CheckedText,
        expression_count: u32,
        span: &Span,
    ) -> Result<HashMap<CheckedExprUseId, HashSet<CheckedLocalId>>, Error> {
        let text = checked_text.id;
        let expression_use = |index: u32| {
            let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: text,
                index,
            });
            self.facts.expression_use_by_owner(owner).ok_or_else(|| {
                self.invariant(span, "rich-text expression has no checked owner mapping")
            })
        };
        let mut contracts = HashMap::new();
        for index in 0..expression_count {
            contracts.insert(expression_use(index)?, HashSet::new());
        }
        for child in &checked_text.rich_children {
            let CheckedRichChild::For {
                item,
                scoped_expressions,
                ..
            } = child
            else {
                continue;
            };
            for index in scoped_expressions.clone() {
                contracts
                    .get_mut(&expression_use(index)?)
                    .ok_or_else(|| {
                        self.invariant(span, "rich-text scoped expression is outside its widget")
                    })?
                    .insert(*item);
            }
        }
        Ok(contracts)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_rich_child(
        &self,
        values: &mut TextOperands<'_>,
        child: &RichTextChild,
        checked_child: &CheckedRichChild,
        index: usize,
        text: ViewId,
        text_origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedRichChild, Error> {
        let require_parent = |origin: OriginId, parent: OriginId, span: &Span| {
            let retained = self
                .origins
                .try_get(origin)
                .ok_or_else(|| self.invariant(span, "rich-text origin is outside its arena"))?;
            if retained.parent != Some(parent) {
                return Err(self.invariant(span, "rich-text origin parent diverged"));
            }
            Ok(())
        };
        match (child, checked_child) {
            (RichTextChild::Span(item), CheckedRichChild::Span { origin }) => {
                require_parent(*origin, text_origin, &item.span)?;
                Ok(ResolvedRichChild::Span(Box::new(
                    self.resolve_rich_span(values, item, *origin)?,
                )))
            }
            (
                RichTextChild::For(iteration),
                CheckedRichChild::For {
                    items,
                    item,
                    origin,
                    spans,
                    ..
                },
            ) => {
                require_parent(*origin, text_origin, &iteration.span)?;
                let (items_use, source) =
                    values.take_where("for items", |ty| matches!(ty, Type::List(_)))?;
                if items_use != *items {
                    return Err(
                        self.invariant(&iteration.span, "rich-text for items contract diverged")
                    );
                }
                let checked_item = self
                    .facts
                    .try_local(*item)
                    .ok_or_else(|| {
                        self.invariant(
                            &iteration.span,
                            "rich-text for item local ID is outside its arena",
                        )
                    })?
                    .clone();
                let expected_owner = CheckedLocalOwner::View {
                    view: text,
                    role: CheckedViewLocalRole::RichForItem(index as u32),
                };
                let Type::List(inner) = &source else {
                    return Err(self.invariant(span, "rich-text for items type is not a list"));
                };
                if checked_item.owner != expected_owner || **inner != checked_item.ty {
                    return Err(
                        self.invariant(&iteration.span, "rich-text for item binding diverged")
                    );
                }
                if spans.len() != iteration.spans.len() {
                    return Err(
                        self.invariant(&iteration.span, "rich-text for span count diverged")
                    );
                }
                let spans = iteration
                    .spans
                    .iter()
                    .zip(spans)
                    .map(|(span_item, span_origin)| {
                        require_parent(*span_origin, *origin, &span_item.span)?;
                        self.resolve_rich_span(values, span_item, *span_origin)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedRichChild::For(ResolvedRichFor {
                    items: items_use,
                    item: ResolvedIterationBinding {
                        local: *item,
                        name: checked_item.name,
                        #[cfg(test)]
                        ty: checked_item.ty,
                    },
                    spans,
                }))
            }
            _ => Err(self.invariant(span, "rich-text child shape diverged")),
        }
    }

    fn resolve_rich_span(
        &self,
        values: &mut TextOperands<'_>,
        span: &RichSpan,
        origin: OriginId,
    ) -> Result<ResolvedRichSpan, Error> {
        let value = values
            .take_where("span value", |actual| {
                matches!(actual, Type::Str | Type::I64 | Type::F64 | Type::Bool)
            })?
            .0;
        let size = values.optional(span.options.size.as_ref(), &Type::F64, "span size")?;
        let line_height = Self::resolve_text_line_height(values, &span.options.line_height)?;
        let link = values.optional(span.options.link.as_ref(), &Type::Str, "span link")?;
        let background = span
            .options
            .background
            .as_ref()
            .map(|background| self.resolve_text_background(values, background, &span.span))
            .transpose()?;
        let border_width = values.optional(
            span.options.border_width.as_ref(),
            &Type::F64,
            "span border width",
        )?;
        let radius = ResolvedContainerRadius {
            all: values.optional(span.options.radius.all.as_ref(), &Type::F64, "span radius")?,
            top_left: values.optional(
                span.options.radius.top_left.as_ref(),
                &Type::F64,
                "span top-left radius",
            )?,
            top_right: values.optional(
                span.options.radius.top_right.as_ref(),
                &Type::F64,
                "span top-right radius",
            )?,
            bottom_right: values.optional(
                span.options.radius.bottom_right.as_ref(),
                &Type::F64,
                "span bottom-right radius",
            )?,
            bottom_left: values.optional(
                span.options.radius.bottom_left.as_ref(),
                &Type::F64,
                "span bottom-left radius",
            )?,
        };
        let padding = ResolvedContainerPadding {
            all: values.optional(
                span.options.padding.all.as_ref(),
                &Type::F64,
                "span padding",
            )?,
            x: values.optional(
                span.options.padding.x.as_ref(),
                &Type::F64,
                "span padding-x",
            )?,
            y: values.optional(
                span.options.padding.y.as_ref(),
                &Type::F64,
                "span padding-y",
            )?,
            top: values.optional(
                span.options.padding.top.as_ref(),
                &Type::F64,
                "span padding-top",
            )?,
            right: values.optional(
                span.options.padding.right.as_ref(),
                &Type::F64,
                "span padding-right",
            )?,
            bottom: values.optional(
                span.options.padding.bottom.as_ref(),
                &Type::F64,
                "span padding-bottom",
            )?,
            left: values.optional(
                span.options.padding.left.as_ref(),
                &Type::F64,
                "span padding-left",
            )?,
        };
        let underline = values.optional(
            span.options.underline.as_ref(),
            &Type::Bool,
            "span underline",
        )?;
        let strikethrough = values.optional(
            span.options.strikethrough.as_ref(),
            &Type::Bool,
            "span strikethrough",
        )?;
        let utility_style = self
            .styles
            .style_use(&span.span)
            .map(|style| style.style.clone())
            .map_err(|_| {
                self.invariant(&span.span, "rich-text span utility site is not normalized")
            })?;
        Ok(ResolvedRichSpan {
            value,
            size,
            line_height,
            font: self.resolve_text_font(span.options.font.as_ref(), origin, &span.span)?,
            color: span
                .options
                .color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, &span.span))
                .transpose()?,
            link,
            background,
            border_color: span
                .options
                .border
                .as_deref()
                .map(|color| self.resolve_theme_color(color, &span.span))
                .transpose()?,
            border_width,
            radius,
            padding,
            underline,
            strikethrough,
            utility_style,
            #[cfg(test)]
            origin,
        })
    }

    fn resolve_text_length(
        values: &mut TextOperands<'_>,
        length: &Option<LengthValue>,
        label: &str,
    ) -> Result<Option<ResolvedContainerLength>, Error> {
        Ok(match length {
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
                    _ => unreachable!("validated text length type"),
                })
            }
        })
    }

    fn resolve_text_line_height(
        values: &mut TextOperands<'_>,
        line_height: &Option<TextLineHeight>,
    ) -> Result<Option<ResolvedTextLineHeight>, Error> {
        line_height
            .as_ref()
            .map(|line_height| {
                let expression = values.take(&Type::F64, "line height")?;
                Ok(match line_height {
                    TextLineHeight::Relative(_) => ResolvedTextLineHeight::Relative(expression),
                    TextLineHeight::Absolute(_) => ResolvedTextLineHeight::Absolute(expression),
                })
            })
            .transpose()
    }

    pub(super) fn resolve_text_font(
        &self,
        font: Option<&FontPreset>,
        origin: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedTextFont>, Error> {
        font.map(|font| match font {
            FontPreset::Default => Ok(ResolvedTextFont::Default),
            FontPreset::Monospace => Ok(ResolvedTextFont::Monospace),
            FontPreset::Named(name) => {
                let font = self
                    .document
                    .fonts
                    .iter()
                    .find(|font| font.name == *name)
                    .ok_or_else(|| self.invariant(span, "named text font disappeared"))?;
                Ok(ResolvedTextFont::Named(ResolvedDefaultFont {
                    family: font.family.clone(),
                    weight: font.weight,
                    stretch: font.stretch,
                    style: font.style,
                    origin,
                }))
            }
        })
        .transpose()
    }

    fn resolve_text_background(
        &self,
        values: &mut TextOperands<'_>,
        background: &BackgroundValue,
        span: &Span,
    ) -> Result<ResolvedContainerBackground, Error> {
        Ok(match background {
            BackgroundValue::Color(color) => {
                ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
            }
            BackgroundValue::Linear { stops, .. } => {
                let angle = values.take(&Type::F64, "span background angle")?;
                let stops = stops
                    .iter()
                    .map(|stop| {
                        Ok(ResolvedContainerGradientStop {
                            color: self.resolve_theme_color(&stop.color, span)?,
                            offset: values.take(&Type::F64, "span background stop")?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                ResolvedContainerBackground::Linear { angle, stops }
            }
        })
    }

    fn resolve_text_alignment(alignment: TextAlignment) -> ResolvedTextAlignment {
        match alignment {
            TextAlignment::Default => ResolvedTextAlignment::Default,
            TextAlignment::Left => ResolvedTextAlignment::Left,
            TextAlignment::Center => ResolvedTextAlignment::Center,
            TextAlignment::Right => ResolvedTextAlignment::Right,
            TextAlignment::Justified => ResolvedTextAlignment::Justified,
        }
    }
}
