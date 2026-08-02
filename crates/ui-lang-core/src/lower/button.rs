use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedButtonContent {
    Label(String),
    Child(ViewId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedButtonPreset {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Text,
    Background,
    Subtle,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedButtonCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedButtonStatusStyle {
    pub(crate) surface: ResolvedContainerSurface,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedButtonStyleSet {
    pub(crate) active: Option<ResolvedButtonStatusStyle>,
    pub(crate) hovered: Option<ResolvedButtonStatusStyle>,
    pub(crate) pressed: Option<ResolvedButtonStatusStyle>,
    pub(crate) disabled: Option<ResolvedButtonStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedButton {
    pub(crate) id: ViewId,
    pub(crate) content: ResolvedButtonContent,
    pub(crate) disabled: Option<CheckedExprUseId>,
    pub(crate) accessibility_label: Option<CheckedExprUseId>,
    pub(crate) accessibility_description: Option<CheckedExprUseId>,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) clip: Option<CheckedExprUseId>,
    pub(crate) route: ResolvedInteractionRoute,
    pub(crate) preset: ResolvedButtonPreset,
    pub(crate) custom_style: Option<ResolvedButtonCustomStyle>,
    pub(crate) styles: ResolvedButtonStyleSet,
    pub(crate) utility_style: ResolvedStyle,
    pub(crate) origin: OriginId,
}

struct ButtonOperands<'a> {
    lowerer: &'a Lowerer,
    button: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    span: &'a Span,
}

impl ButtonOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer
                .invariant(self.span, format!("button {label} expression disappeared"))
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.button,
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
                    format!("button {label} expression ID is invalid"),
                )
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("button {label} expression contract diverged"),
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
        source: Option<&T>,
        expected: &Type,
        label: &str,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        source.map(|_| self.take(expected, label)).transpose()
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.expressions.next().is_some() {
            return Err(self.lowerer.invariant(
                self.span,
                "button left checked option expressions unconsumed",
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_button(
        &mut self,
        label: &Option<String>,
        content: &Option<Box<ViewNode>>,
        disabled: &Option<Expr>,
        options: &ButtonOptions,
        route: &Route,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let semantic_key =
            crate::ast::button_semantic_key(label, content, disabled, options, route);
        let roots = crate::ast::button_expression_roots(disabled, options);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Button,
            semantic_key,
            span,
            outer_component,
        )?;
        let checked_button = self
            .facts
            .button(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "button has no checked HIR facts"))?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        if checked.option_expressions.len() != roots.len() {
            return Err(self.invariant(span, "button expression cardinality diverged"));
        }
        let checked_children = &self.facts.view(id).children;
        let resolved_content = match (label, content) {
            (Some(label), None) if checked_children.is_empty() => {
                ResolvedButtonContent::Label(label.clone())
            }
            (None, Some(child)) if checked_children.len() == 1 => {
                let child_id = self.declarations.view_id(child.span()).ok_or_else(|| {
                    self.invariant(child.span(), "button child has no checked view ID")
                })?;
                if checked_children[0] != child_id {
                    return Err(self.invariant(span, "button child identity diverged"));
                }
                ResolvedButtonContent::Child(child_id)
            }
            _ => return Err(self.invariant(span, "button label/child topology diverged")),
        };
        let mut values = ButtonOperands {
            lowerer: self,
            button: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };
        let disabled = values.optional(disabled.as_ref(), &Type::Bool, "disabled")?;
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
        let width = Self::resolve_button_length(&mut values, &options.width, "width")?;
        let height = Self::resolve_button_length(&mut values, &options.height, "height")?;
        let padding = values.optional(options.padding.as_ref(), &Type::F64, "padding")?;
        let clip = values.optional(options.clip.as_ref(), &Type::Bool, "clip")?;
        let custom_style = options
            .style
            .custom
            .as_ref()
            .map(|style| {
                let function = checked_button.style.ok_or_else(|| {
                    self.invariant(span, "button custom style lost its checked extern ID")
                })?;
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .ok_or_else(|| self.invariant(span, "button style extern disappeared"))?;
                if declaration.name != style.function
                    || declaration.kind != ExternKind::ButtonStyle
                    || declaration.params.len() != style.args.len()
                {
                    return Err(self.invariant(span, "button style extern contract diverged"));
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "style argument"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedButtonCustomStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        if custom_style.is_none() != checked_button.style.is_none() {
            return Err(self.invariant(span, "button custom style presence diverged"));
        }
        let styles = self.resolve_button_styles(
            &mut values,
            &options.style,
            &checked_button.status_origins,
            origin,
            span,
        )?;
        values.finish()?;
        if checked.routes.len() != 1 {
            return Err(self.invariant(span, "button checked route cardinality diverged"));
        }
        let route = self.lower_interaction_route(route, &checked, 0, id, scope)?;
        let utility_style = self
            .styles
            .style_use(span)
            .map(|style| style.style.clone())
            .map_err(|_| self.invariant(span, "button utility style site is not normalized"))?;
        let preset = match options.style.preset {
            ButtonStylePreset::Primary => ResolvedButtonPreset::Primary,
            ButtonStylePreset::Secondary => ResolvedButtonPreset::Secondary,
            ButtonStylePreset::Success => ResolvedButtonPreset::Success,
            ButtonStylePreset::Warning => ResolvedButtonPreset::Warning,
            ButtonStylePreset::Danger => ResolvedButtonPreset::Danger,
            ButtonStylePreset::Text => ResolvedButtonPreset::Text,
            ButtonStylePreset::Background => ResolvedButtonPreset::Background,
            ButtonStylePreset::Subtle => ResolvedButtonPreset::Subtle,
        };
        if self
            .buttons
            .insert(
                id,
                ResolvedButton {
                    id,
                    content: resolved_content,
                    disabled,
                    accessibility_label,
                    accessibility_description,
                    width,
                    height,
                    padding,
                    clip,
                    route,
                    preset,
                    custom_style,
                    styles,
                    utility_style,
                    origin,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "button was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_button_length(
        values: &mut ButtonOperands<'_>,
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
                    _ => unreachable!("validated button length type"),
                })
            }
        })
    }

    fn resolve_button_styles(
        &self,
        values: &mut ButtonOperands<'_>,
        styles: &ButtonStyleSet,
        origins: &[OriginId],
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedButtonStyleSet, Error> {
        let sources = [
            &styles.active,
            &styles.hovered,
            &styles.pressed,
            &styles.disabled,
        ];
        if origins.len()
            != sources
                .into_iter()
                .filter(|source| source.is_some())
                .count()
        {
            return Err(self.invariant(span, "button status origin count diverged"));
        }
        let mut origins = origins.iter().copied();
        let mut resolve = |source: &Option<ButtonStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let origin = origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "button status origin disappeared"))?;
                    if self
                        .origins
                        .try_get(origin)
                        .is_none_or(|origin| origin.parent != Some(parent))
                    {
                        return Err(self.invariant(span, "button status origin parent diverged"));
                    }
                    Ok(ResolvedButtonStatusStyle {
                        surface: self.resolve_button_surface(values, &source.options, span)?,
                        #[cfg(test)]
                        origin,
                    })
                })
                .transpose()
        };
        let resolved = ResolvedButtonStyleSet {
            active: resolve(&styles.active)?,
            hovered: resolve(&styles.hovered)?,
            pressed: resolve(&styles.pressed)?,
            disabled: resolve(&styles.disabled)?,
        };
        if origins.next().is_some() {
            return Err(self.invariant(span, "button left status origins unconsumed"));
        }
        Ok(resolved)
    }

    fn resolve_button_surface(
        &self,
        values: &mut ButtonOperands<'_>,
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
        Ok(ResolvedContainerSurface {
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
            border_width: values.optional(
                surface.border_width.as_ref(),
                &Type::F64,
                "status border width",
            )?,
            radius: ResolvedContainerRadius {
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
            },
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
}
