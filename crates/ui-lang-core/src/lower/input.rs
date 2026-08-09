use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedInputAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedInputIconSide {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInputIcon {
    pub(crate) code_point: char,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) size: Option<CheckedExprUseId>,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) side: ResolvedInputIconSide,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInputCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInputStatusStyle {
    pub(crate) surface: ResolvedContainerSurface,
    pub(crate) icon_color: Option<ResolvedThemeColor>,
    pub(crate) placeholder_color: Option<ResolvedThemeColor>,
    pub(crate) value_color: Option<ResolvedThemeColor>,
    pub(crate) selection_color: Option<ResolvedThemeColor>,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedInputStyleSet {
    pub(crate) active: Option<ResolvedInputStatusStyle>,
    pub(crate) hovered: Option<ResolvedInputStatusStyle>,
    pub(crate) focused: Option<ResolvedInputStatusStyle>,
    pub(crate) focused_hovered: Option<ResolvedInputStatusStyle>,
    pub(crate) disabled: Option<ResolvedInputStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInput {
    pub(crate) id: ViewId,
    pub(crate) label: String,
    pub(crate) hint: String,
    pub(crate) binding: WritableStateRef,
    pub(crate) disabled: Option<CheckedExprUseId>,
    pub(crate) accessibility_label: Option<CheckedExprUseId>,
    pub(crate) accessibility_description: Option<CheckedExprUseId>,
    pub(crate) secure: Option<CheckedExprUseId>,
    pub(crate) change: Option<ResolvedInteractionRoute>,
    pub(crate) submit: Option<ResolvedInteractionRoute>,
    pub(crate) paste: Option<ResolvedInteractionRoute>,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) text_size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<CheckedExprUseId>,
    pub(crate) align: Option<ResolvedInputAlignment>,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) icon: Option<ResolvedInputIcon>,
    pub(crate) custom_style: Option<ResolvedInputCustomStyle>,
    pub(crate) styles: ResolvedInputStyleSet,
    pub(crate) utility_style: ResolvedStyle,
    pub(crate) origin: OriginId,
}

struct InputOperands<'a> {
    lowerer: &'a Lowerer,
    input: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    span: &'a Span,
}

impl InputOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer
                .invariant(self.span, format!("input {label} expression disappeared"))
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.input,
            index: self.next,
        });
        self.next += 1;
        let retained = self
            .lowerer
            .facts
            .try_expression_use(expression)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, format!("input {label} expression ID is invalid"))
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("input {label} expression contract diverged"),
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
            return Err(self.lowerer.invariant(
                self.span,
                "input left checked option expressions unconsumed",
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_input(
        &mut self,
        label: &str,
        binding: &str,
        hint: &str,
        disabled: &Option<Expr>,
        options: &InputOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let semantic_key = crate::ast::input_semantic_key(label, binding, hint, disabled, options);
        let roots = crate::ast::input_expression_roots(disabled, options);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Input,
            semantic_key,
            span,
            outer_component,
        )?;
        let checked_input = self
            .facts
            .input(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "input has no checked HIR facts"))?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        if checked.option_expressions.len() != roots.len() {
            return Err(self.invariant(span, "input expression cardinality diverged"));
        }
        let mut values = InputOperands {
            lowerer: self,
            input: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };

        let disabled_expression = values.optional(disabled.as_ref(), &Type::Bool, "disabled")?;
        let accessibility_label = values.optional(
            options.accessibility.label.as_ref(),
            &Type::Str,
            "accessibility label",
        )?;
        let accessibility_description = values.optional(
            options.accessibility.description.as_ref(),
            &Type::Str,
            "accessibility description",
        )?;
        let secure = values.optional(options.secure.as_ref(), &Type::Bool, "secure")?;
        let width = Self::resolve_input_length(&mut values, &options.width)?;
        let padding = values.optional(options.padding.as_ref(), &Type::F64, "padding")?;
        let text_size = values.optional(options.text_size.as_ref(), &Type::F64, "text size")?;
        let line_height =
            values.optional(options.line_height.as_ref(), &Type::F64, "line height")?;
        let icon = options
            .icon
            .as_ref()
            .map(|icon| self.resolve_input_icon(&mut values, icon, &checked_input))
            .transpose()?;
        if icon.is_none() != checked_input.icon_origin.is_none() {
            return Err(self.invariant(span, "input icon origin presence diverged"));
        }
        let custom_style = options
            .custom_style
            .as_ref()
            .map(|style| {
                let function = checked_input.style.ok_or_else(|| {
                    self.invariant(span, "input custom style lost its checked extern ID")
                })?;
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .ok_or_else(|| self.invariant(span, "input style extern disappeared"))?;
                if declaration.name != style.function
                    || declaration.kind != ExternKind::InputStyle
                    || declaration.params.len() != style.args.len()
                {
                    return Err(self.invariant(span, "input style extern contract diverged"));
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "style argument"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedInputCustomStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        if custom_style.is_none() != checked_input.style.is_none() {
            return Err(self.invariant(span, "input custom style presence diverged"));
        }
        let styles = self.resolve_input_styles(
            &mut values,
            &options.style,
            &checked_input.status_origins,
            origin,
            span,
        )?;
        values.finish()?;

        let sources = [&options.change, &options.submit, &options.paste];
        let routes = sources.into_iter().flatten().collect::<Vec<_>>();
        let mut route_index = 0usize;
        let mut take_route = |source: &Option<Route>| {
            self.lower_optional_interaction_route(
                source,
                &checked,
                &routes,
                &mut route_index,
                id,
                scope,
            )
        };
        let change = take_route(&options.change)?;
        let submit = take_route(&options.submit)?;
        let paste = take_route(&options.paste)?;
        if route_index != checked.routes.len() {
            return Err(self.invariant(span, "input left checked routes unconsumed"));
        }
        let utility_style = self
            .styles
            .style_use(span)
            .map(|style| style.style.clone())
            .map_err(|_| self.invariant(span, "input utility style site is not normalized"))?;
        let resolved = ResolvedInput {
            id,
            label: label.to_owned(),
            hint: hint.to_owned(),
            binding: self.resolve_input_binding(
                checked_input.binding,
                binding,
                outer_component,
                span,
            )?,
            disabled: disabled_expression,
            accessibility_label,
            accessibility_description,
            secure,
            change,
            submit,
            paste,
            width,
            padding,
            text_size,
            line_height,
            align: options.align.map(|align| match align {
                InputAlignment::Left => ResolvedInputAlignment::Left,
                InputAlignment::Center => ResolvedInputAlignment::Center,
                InputAlignment::Right => ResolvedInputAlignment::Right,
            }),
            font: self.resolve_text_font(options.font.as_ref(), origin, span)?,
            icon,
            custom_style,
            styles,
            utility_style,
            origin,
        };
        if self.inputs.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "input was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_input_binding(
        &self,
        binding: CheckedValueRef,
        expected_name: &str,
        outer_component: Option<ComponentId>,
        span: &Span,
    ) -> Result<WritableStateRef, Error> {
        let value = self
            .facts
            .try_value_by_ref(binding)
            .ok_or_else(|| self.invariant(span, "input binding value ID is invalid"))?;
        if value.ty != Type::Str || value.name != expected_name {
            return Err(self.invariant(span, "input binding identity diverged"));
        }
        match binding {
            CheckedValueRef::AppState(id) if outer_component.is_none() => {
                Ok(WritableStateRef::App {
                    id,
                    name: value.name.clone(),
                })
            }
            CheckedValueRef::ComponentParam(id)
                if outer_component == Some(id.component)
                    && self.components[id.component.0 as usize].params[id.index as usize]
                        .capability
                        == ParamCapability::Bind =>
            {
                Ok(WritableStateRef::ComponentParam {
                    id,
                    name: value.name.clone(),
                })
            }
            CheckedValueRef::ComponentState(id) if outer_component == Some(id.component) => {
                Ok(WritableStateRef::ComponentState {
                    id,
                    name: value.name.clone(),
                })
            }
            _ => Err(self.invariant(span, "input binding is not writable in this scope")),
        }
    }

    fn resolve_input_icon(
        &self,
        values: &mut InputOperands<'_>,
        icon: &TextInputIcon,
        checked: &CheckedInput,
    ) -> Result<ResolvedInputIcon, Error> {
        let origin = checked
            .icon_origin
            .ok_or_else(|| self.invariant(&icon.span, "input icon origin disappeared"))?;
        let retained = self
            .origins
            .try_get(origin)
            .ok_or_else(|| self.invariant(&icon.span, "input icon origin is outside its arena"))?;
        if retained.parent != Some(self.declarations.view(checked.id).origin) {
            return Err(self.invariant(&icon.span, "input icon origin parent diverged"));
        }
        Ok(ResolvedInputIcon {
            code_point: icon.code_point,
            font: self.resolve_text_font(icon.font.as_ref(), origin, &icon.span)?,
            size: values.optional(icon.size.as_ref(), &Type::F64, "icon size")?,
            spacing: values.optional(icon.spacing.as_ref(), &Type::F64, "icon spacing")?,
            side: match icon.side {
                IconSide::Left => ResolvedInputIconSide::Left,
                IconSide::Right => ResolvedInputIconSide::Right,
            },
            #[cfg(test)]
            origin,
        })
    }

    fn resolve_input_styles(
        &self,
        values: &mut InputOperands<'_>,
        styles: &TextInputStyleSet,
        origins: &[OriginId],
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedInputStyleSet, Error> {
        let sources = [
            &styles.active,
            &styles.hovered,
            &styles.focused,
            &styles.focused_hovered,
            &styles.disabled,
        ];
        if origins.len()
            != sources
                .into_iter()
                .filter(|source| source.is_some())
                .count()
        {
            return Err(self.invariant(span, "input status origin count diverged"));
        }
        let mut origins = origins.iter().copied();
        let mut resolve = |source: &Option<TextInputStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let origin = origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "input status origin disappeared"))?;
                    if self
                        .origins
                        .try_get(origin)
                        .is_none_or(|origin| origin.parent != Some(parent))
                    {
                        return Err(self.invariant(span, "input status origin parent diverged"));
                    }
                    self.resolve_input_status(values, source, origin, span)
                })
                .transpose()
        };
        let resolved = ResolvedInputStyleSet {
            active: resolve(&styles.active)?,
            hovered: resolve(&styles.hovered)?,
            focused: resolve(&styles.focused)?,
            focused_hovered: resolve(&styles.focused_hovered)?,
            disabled: resolve(&styles.disabled)?,
        };
        if origins.next().is_some() {
            return Err(self.invariant(span, "input left status origins unconsumed"));
        }
        Ok(resolved)
    }

    fn resolve_input_status(
        &self,
        values: &mut InputOperands<'_>,
        status: &TextInputStatusStyle,
        _origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedInputStatusStyle, Error> {
        Ok(ResolvedInputStatusStyle {
            surface: self.resolve_input_surface(values, &status.options, span)?,
            icon_color: status
                .icon_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            placeholder_color: status
                .placeholder_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            value_color: status
                .value_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            selection_color: status
                .selection_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            #[cfg(test)]
            origin: _origin,
        })
    }

    fn resolve_input_surface(
        &self,
        values: &mut InputOperands<'_>,
        surface: &ContainerStyleOptions,
        span: &Span,
    ) -> Result<ResolvedContainerSurface, Error> {
        let background = surface
            .background
            .as_ref()
            .map(|background| {
                Ok(match background {
                    BackgroundValue::Color(color) => {
                        ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
                    }
                    BackgroundValue::Linear { stops, .. } => {
                        let angle = values.take(&Type::F64, "status background angle")?;
                        let stops = stops
                            .iter()
                            .map(|stop| {
                                Ok(ResolvedContainerGradientStop {
                                    color: self.resolve_theme_color(&stop.color, span)?,
                                    offset: values.take(&Type::F64, "status background stop")?,
                                })
                            })
                            .collect::<Result<Vec<_>, Error>>()?;
                        ResolvedContainerBackground::Linear { angle, stops }
                    }
                })
            })
            .transpose()?;
        let border_width = values.optional(
            surface.border_width.as_ref(),
            &Type::F64,
            "status border width",
        )?;
        let radius = ResolvedContainerRadius {
            all: values.optional(surface.radius.as_ref(), &Type::F64, "status radius")?,
            top_left: values.optional(
                surface.radius_top_left.as_ref(),
                &Type::F64,
                "status top-left radius",
            )?,
            top_right: values.optional(
                surface.radius_top_right.as_ref(),
                &Type::F64,
                "status top-right radius",
            )?,
            bottom_right: values.optional(
                surface.radius_bottom_right.as_ref(),
                &Type::F64,
                "status bottom-right radius",
            )?,
            bottom_left: values.optional(
                surface.radius_bottom_left.as_ref(),
                &Type::F64,
                "status bottom-left radius",
            )?,
        };
        Ok(ResolvedContainerSurface {
            background_alpha: None,
            background,
            text_color: surface
                .text_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_color: surface
                .border_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            border_width,
            radius,
            shadow_color: surface
                .shadow_color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            shadow_x: values.optional(surface.shadow_x.as_ref(), &Type::F64, "status shadow x")?,
            shadow_y: values.optional(surface.shadow_y.as_ref(), &Type::F64, "status shadow y")?,
            shadow_blur: values.optional(
                surface.shadow_blur.as_ref(),
                &Type::F64,
                "status shadow blur",
            )?,
            pixel_snap: values.optional(
                surface.pixel_snap.as_ref(),
                &Type::Bool,
                "status pixel snap",
            )?,
        })
    }

    fn resolve_input_length(
        values: &mut InputOperands<'_>,
        length: &Option<LengthValue>,
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
                    .take_where("width", |actual| matches!(actual, Type::F64 | Type::Length))?;
                Some(match source {
                    Type::F64 => ResolvedContainerLength::FixedF64(expression),
                    Type::Length => ResolvedContainerLength::FixedLength(expression),
                    _ => unreachable!("validated input width type"),
                })
            }
        })
    }
}
