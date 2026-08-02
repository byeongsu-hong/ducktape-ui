// Stable IDs and origins are retained for validation even when the emitter
// does not inspect every field directly.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedEditorWrapping {
    None,
    Glyph,
    Word,
    WordOrGlyph,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedHighlightTheme {
    SolarizedDark,
    Base16Mocha,
    Base16Ocean,
    Base16Eighties,
    InspiredGithub,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedEditorExternCall {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedEditorKeyBinding {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
    pub(crate) route: ResolvedInteractionRoute,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTextEditor {
    pub(crate) id: ViewId,
    pub(crate) binding: WritableStateRef,
    pub(crate) placeholder: Option<String>,
    pub(crate) disabled: Option<CheckedExprUseId>,
    pub(crate) width: Option<CheckedExprUseId>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) min_height: Option<CheckedExprUseId>,
    pub(crate) max_height: Option<CheckedExprUseId>,
    pub(crate) size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<ResolvedTextLineHeight>,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) wrapping: Option<ResolvedEditorWrapping>,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) highlight: Option<String>,
    pub(crate) highlight_theme: Option<ResolvedHighlightTheme>,
    pub(crate) highlighter: Option<ResolvedEditorExternCall>,
    pub(crate) key_binding: Option<ResolvedEditorKeyBinding>,
    pub(crate) action: Option<ResolvedEditorExternCall>,
    pub(crate) custom_style: Option<ResolvedEditorExternCall>,
    pub(crate) styles: ResolvedInputStyleSet,
    pub(crate) origin: OriginId,
}

struct EditorOperands<'a> {
    lowerer: &'a Lowerer,
    editor: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    span: &'a Span,
}

impl EditorOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer
                .invariant(self.span, format!("editor {label} expression disappeared"))
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.editor,
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
                    format!("editor {label} expression ID is invalid"),
                )
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("editor {label} expression contract diverged"),
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
                "editor left checked option expressions unconsumed",
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_text_editor(
        &mut self,
        binding: &str,
        disabled: &Option<Expr>,
        options: &TextEditorOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let semantic_key = crate::ast::text_editor_semantic_key(binding, disabled, options);
        let roots = crate::ast::text_editor_expression_roots(disabled, options);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::TextEditor,
            semantic_key,
            span,
            outer_component,
        )?;
        let checked_editor = self
            .facts
            .text_editor(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "text editor has no checked HIR facts"))?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        if checked.option_expressions.len() != roots.len() {
            return Err(self.invariant(span, "editor expression cardinality diverged"));
        }
        let mut values = EditorOperands {
            lowerer: self,
            editor: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };
        let disabled = values.optional(disabled.as_ref(), &Type::Bool, "disabled")?;
        let width = values.optional(options.width.as_ref(), &Type::F64, "width")?;
        let height = Self::resolve_editor_length(&mut values, &options.height)?;
        let min_height = values.optional(options.min_height.as_ref(), &Type::F64, "min height")?;
        let max_height = values.optional(options.max_height.as_ref(), &Type::F64, "max height")?;
        let size = values.optional(options.size.as_ref(), &Type::F64, "text size")?;
        let line_height = options
            .line_height
            .as_ref()
            .map(|height| {
                let expression = values.take(&Type::F64, "line height")?;
                Ok(match height {
                    TextLineHeight::Relative(_) => ResolvedTextLineHeight::Relative(expression),
                    TextLineHeight::Absolute(_) => ResolvedTextLineHeight::Absolute(expression),
                })
            })
            .transpose()?;
        let padding = values.optional(options.padding.as_ref(), &Type::F64, "padding")?;
        let highlighter = self.resolve_editor_call(
            &mut values,
            options.highlighter.as_ref(),
            checked_editor.highlighter,
            ExternKind::EditorHighlighter,
            origin,
            span,
        )?;
        let key_call = self.resolve_editor_call(
            &mut values,
            options.key_binding.as_ref(),
            checked_editor.key_binding,
            ExternKind::EditorBinding,
            origin,
            span,
        )?;
        let action = self.resolve_editor_call(
            &mut values,
            options.action.as_ref(),
            checked_editor.action,
            ExternKind::EditorAction,
            origin,
            span,
        )?;
        let custom_style = self.resolve_editor_call(
            &mut values,
            options.custom_style.as_ref(),
            checked_editor.style,
            ExternKind::EditorStyle,
            origin,
            span,
        )?;
        let styles = self.resolve_editor_styles(
            &mut values,
            &options.style,
            &checked_editor.status_origins,
            origin,
            span,
        )?;
        values.finish()?;

        let routes = options.key_binding_route.iter().collect::<Vec<_>>();
        let mut route_index = 0usize;
        let key_route = self.lower_optional_interaction_route(
            &options.key_binding_route,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != checked.routes.len() {
            return Err(self.invariant(span, "editor left checked routes unconsumed"));
        }
        let key_binding = match (key_call, key_route) {
            (Some(call), Some(route)) => Some(ResolvedEditorKeyBinding {
                function: call.function,
                arguments: call.arguments,
                route,
                origin: call.origin,
            }),
            (None, None) => None,
            _ => return Err(self.invariant(span, "editor key binding and route diverged")),
        };
        let binding =
            self.resolve_editor_binding(checked_editor.binding, binding, outer_component, span)?;
        let resolved = ResolvedTextEditor {
            id,
            binding,
            placeholder: options.placeholder.clone(),
            disabled,
            width,
            height,
            min_height,
            max_height,
            size,
            line_height,
            padding,
            wrapping: options.wrapping.map(|wrapping| match wrapping {
                TextWrapping::None => ResolvedEditorWrapping::None,
                TextWrapping::Glyph => ResolvedEditorWrapping::Glyph,
                TextWrapping::Word => ResolvedEditorWrapping::Word,
                TextWrapping::WordOrGlyph => ResolvedEditorWrapping::WordOrGlyph,
            }),
            font: self.resolve_text_font(options.font.as_ref(), origin, span)?,
            highlight: options.highlight.clone(),
            highlight_theme: options.highlight_theme.map(|theme| match theme {
                HighlightTheme::SolarizedDark => ResolvedHighlightTheme::SolarizedDark,
                HighlightTheme::Base16Mocha => ResolvedHighlightTheme::Base16Mocha,
                HighlightTheme::Base16Ocean => ResolvedHighlightTheme::Base16Ocean,
                HighlightTheme::Base16Eighties => ResolvedHighlightTheme::Base16Eighties,
                HighlightTheme::InspiredGithub => ResolvedHighlightTheme::InspiredGithub,
            }),
            highlighter,
            key_binding,
            action,
            custom_style,
            styles,
            origin,
        };
        if self.text_editors.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "text editor was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_editor_binding(
        &self,
        binding: CheckedValueRef,
        expected_name: &str,
        outer_component: Option<ComponentId>,
        span: &Span,
    ) -> Result<WritableStateRef, Error> {
        let value = self
            .facts
            .try_value_by_ref(binding)
            .ok_or_else(|| self.invariant(span, "editor binding value ID is invalid"))?;
        if value.ty != Type::Editor || value.name != expected_name {
            return Err(self.invariant(span, "editor binding identity diverged"));
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
            _ => Err(self.invariant(span, "editor binding is not writable in this scope")),
        }
    }

    fn resolve_editor_call(
        &self,
        values: &mut EditorOperands<'_>,
        source: Option<&ExternCall>,
        function: Option<ExternFnId>,
        kind: ExternKind,
        origin: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedEditorExternCall>, Error> {
        match (source, function) {
            (None, None) => Ok(None),
            (Some(source), Some(function)) => {
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .filter(|declaration| {
                        declaration.kind == kind
                            && declaration.name == source.function
                            && declaration.params.len() == source.args.len()
                    })
                    .ok_or_else(|| self.invariant(span, "editor extern contract diverged"))?;
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "extern argument"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(ResolvedEditorExternCall {
                    function,
                    arguments,
                    origin,
                }))
            }
            _ => Err(self.invariant(span, "editor extern presence diverged")),
        }
    }

    fn resolve_editor_styles(
        &self,
        values: &mut EditorOperands<'_>,
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
            return Err(self.invariant(span, "editor status origin count diverged"));
        }
        let mut origins = origins.iter().copied();
        let mut resolve = |source: &Option<TextInputStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let origin = origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "editor status origin disappeared"))?;
                    if self
                        .origins
                        .try_get(origin)
                        .is_none_or(|origin| origin.parent != Some(parent))
                    {
                        return Err(self.invariant(span, "editor status origin parent diverged"));
                    }
                    self.resolve_editor_status(values, source, origin, span)
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
            return Err(self.invariant(span, "editor left status origins unconsumed"));
        }
        Ok(resolved)
    }

    fn resolve_editor_status(
        &self,
        values: &mut EditorOperands<'_>,
        status: &TextInputStatusStyle,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedInputStatusStyle, Error> {
        Ok(ResolvedInputStatusStyle {
            surface: self.resolve_editor_surface(values, &status.options, span)?,
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
            origin,
        })
    }

    fn resolve_editor_surface(
        &self,
        values: &mut EditorOperands<'_>,
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

    fn resolve_editor_length(
        values: &mut EditorOperands<'_>,
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
                let (expression, source) = values.take_where("height", |actual| {
                    matches!(actual, Type::F64 | Type::Length)
                })?;
                Some(match source {
                    Type::F64 => ResolvedContainerLength::FixedF64(expression),
                    Type::Length => ResolvedContainerLength::FixedLength(expression),
                    _ => unreachable!("validated editor height type"),
                })
            }
        })
    }
}
