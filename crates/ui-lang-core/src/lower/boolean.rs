use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedBooleanKind {
    Checkbox,
    Toggler,
    Radio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedCheckboxPreset {
    Primary,
    Secondary,
    Success,
    Danger,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedBooleanColor {
    pub(crate) value: ResolvedThemeColor,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedBooleanBackground {
    pub(crate) value: ResolvedContainerBackground,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedBooleanCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedBooleanIcon {
    pub(crate) code_point: char,
    pub(crate) size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<CheckedExprUseId>,
    pub(crate) shaping: Option<ResolvedTextShaping>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedBooleanOptions {
    pub(crate) accessibility_label: Option<CheckedExprUseId>,
    pub(crate) accessibility_description: Option<CheckedExprUseId>,
    pub(crate) size: Option<CheckedExprUseId>,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) text_size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<CheckedExprUseId>,
    pub(crate) shaping: Option<ResolvedTextShaping>,
    pub(crate) wrapping: Option<ResolvedTextWrapping>,
    pub(crate) font: Option<ResolvedTextFont>,
    #[cfg(test)]
    pub(crate) font_origin: Option<OriginId>,
    pub(crate) alignment: Option<ResolvedTextAlignment>,
    pub(crate) icon: Option<ResolvedBooleanIcon>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedCheckboxStatusStyle {
    pub(crate) background: Option<ResolvedBooleanBackground>,
    pub(crate) icon_color: Option<ResolvedBooleanColor>,
    pub(crate) text_color: Option<ResolvedBooleanColor>,
    pub(crate) border_color: Option<ResolvedBooleanColor>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) radius: ResolvedContainerRadius,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedCheckboxStyleSet {
    pub(crate) preset: Option<ResolvedCheckboxPreset>,
    pub(crate) custom: Option<ResolvedBooleanCustomStyle>,
    pub(crate) active_checked: Option<ResolvedCheckboxStatusStyle>,
    pub(crate) active_unchecked: Option<ResolvedCheckboxStatusStyle>,
    pub(crate) hovered_checked: Option<ResolvedCheckboxStatusStyle>,
    pub(crate) hovered_unchecked: Option<ResolvedCheckboxStatusStyle>,
    pub(crate) disabled_checked: Option<ResolvedCheckboxStatusStyle>,
    pub(crate) disabled_unchecked: Option<ResolvedCheckboxStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTogglerStatusStyle {
    pub(crate) background: Option<ResolvedBooleanBackground>,
    pub(crate) background_border_color: Option<ResolvedBooleanColor>,
    pub(crate) background_border_width: Option<CheckedExprUseId>,
    pub(crate) foreground: Option<ResolvedBooleanBackground>,
    pub(crate) foreground_border_color: Option<ResolvedBooleanColor>,
    pub(crate) foreground_border_width: Option<CheckedExprUseId>,
    pub(crate) text_color: Option<ResolvedBooleanColor>,
    pub(crate) radius: ResolvedContainerRadius,
    pub(crate) padding_ratio: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedTogglerStyleSet {
    pub(crate) custom: Option<ResolvedBooleanCustomStyle>,
    pub(crate) active_checked: Option<ResolvedTogglerStatusStyle>,
    pub(crate) active_unchecked: Option<ResolvedTogglerStatusStyle>,
    pub(crate) hovered_checked: Option<ResolvedTogglerStatusStyle>,
    pub(crate) hovered_unchecked: Option<ResolvedTogglerStatusStyle>,
    pub(crate) disabled_checked: Option<ResolvedTogglerStatusStyle>,
    pub(crate) disabled_unchecked: Option<ResolvedTogglerStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedRadioStatusStyle {
    pub(crate) background: Option<ResolvedBooleanBackground>,
    pub(crate) dot_color: Option<ResolvedBooleanColor>,
    pub(crate) border_color: Option<ResolvedBooleanColor>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) text_color: Option<ResolvedBooleanColor>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedRadioStyleSet {
    pub(crate) custom: Option<ResolvedBooleanCustomStyle>,
    pub(crate) active_selected: Option<ResolvedRadioStatusStyle>,
    pub(crate) active_unselected: Option<ResolvedRadioStatusStyle>,
    pub(crate) hovered_selected: Option<ResolvedRadioStatusStyle>,
    pub(crate) hovered_unselected: Option<ResolvedRadioStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedBooleanStyle {
    Checkbox(Box<ResolvedCheckboxStyleSet>),
    Toggler(Box<ResolvedTogglerStyleSet>),
    Radio(Box<ResolvedRadioStyleSet>),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedBooleanControl {
    pub(crate) id: ViewId,
    pub(crate) kind: ResolvedBooleanKind,
    pub(crate) label: CheckedExprUseId,
    pub(crate) checked: CheckedExprUseId,
    pub(crate) value: Option<CheckedExprUseId>,
    pub(crate) disabled: Option<CheckedExprUseId>,
    pub(crate) options: ResolvedBooleanOptions,
    pub(crate) route: ResolvedInteractionRoute,
    pub(crate) style: ResolvedBooleanStyle,
    pub(crate) source_line: usize,
    pub(crate) origin: OriginId,
}

pub(super) struct CheckboxSource<'a> {
    pub(super) id: &'a Option<Id>,
    pub(super) label: &'a Expr,
    pub(super) checked: &'a Expr,
    pub(super) disabled: &'a Option<Expr>,
    pub(super) options: &'a BoolControlOptions,
    pub(super) style: &'a CheckboxStyleSet,
    pub(super) route: &'a Route,
    pub(super) span: &'a Span,
}

pub(super) struct TogglerSource<'a> {
    pub(super) id: &'a Option<Id>,
    pub(super) label: &'a Expr,
    pub(super) checked: &'a Expr,
    pub(super) disabled: &'a Option<Expr>,
    pub(super) options: &'a BoolControlOptions,
    pub(super) style: &'a TogglerStyleSet,
    pub(super) route: &'a Route,
    pub(super) span: &'a Span,
}

pub(super) struct RadioSource<'a> {
    pub(super) id: &'a Option<Id>,
    pub(super) label: &'a Expr,
    pub(super) value: &'a Expr,
    pub(super) selected: &'a Expr,
    pub(super) options: &'a BoolControlOptions,
    pub(super) style: &'a RadioStyleSet,
    pub(super) route: &'a Route,
    pub(super) span: &'a Span,
}

struct BooleanCustomStyleSource<'a> {
    call: Option<&'a ExternCall>,
    function: Option<ExternFnId>,
    origin: Option<OriginId>,
    kind: ExternKind,
    parent: OriginId,
    span: &'a Span,
}

struct BooleanOperands<'a> {
    lowerer: &'a Lowerer,
    control: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    span: &'a Span,
}

impl BooleanOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        parent: OriginId,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer
                .invariant(self.span, format!("boolean {label} expression disappeared"))
        })?;
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: self.control,
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
                    format!("boolean {label} expression ID is invalid"),
                )
            })?;
        let origin = self
            .lowerer
            .origins
            .try_get(retained.origin)
            .ok_or_else(|| {
                self.lowerer
                    .invariant(self.span, format!("boolean {label} origin is invalid"))
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
            || origin.parent != Some(parent)
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("boolean {label} expression contract diverged"),
            ));
        }
        Ok((expression, retained.source.clone()))
    }

    fn take(
        &mut self,
        expected: &Type,
        label: &str,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        self.take_where(label, parent, |actual| actual == expected)
            .map(|(expression, _)| expression)
    }

    fn optional<T>(
        &mut self,
        source: Option<&T>,
        expected: &Type,
        label: &str,
        parent: OriginId,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        source
            .map(|_| self.take(expected, label, parent))
            .transpose()
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.expressions.next().is_some() {
            return Err(self.lowerer.invariant(
                self.span,
                "boolean control left checked expressions unconsumed",
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    pub(super) fn lower_checkbox(
        &mut self,
        source: CheckboxSource<'_>,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let CheckboxSource {
            id: raw_id,
            label,
            checked: checked_value,
            disabled,
            options,
            style,
            route,
            span,
        } = source;
        let roots =
            crate::ast::checkbox_expression_roots(label, checked_value, disabled, options, style);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Checkbox,
            crate::ast::checkbox_semantic_key(
                raw_id,
                label,
                checked_value,
                disabled,
                options,
                style,
                route,
            ),
            span,
            outer_component,
        )?;
        let facts = self.checked_boolean_facts(id, span)?;
        self.validate_boolean_header(&checked, &facts, roots.len(), scope, span)?;
        let mut values = BooleanOperands {
            lowerer: self,
            control: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };
        let label = values.take(&Type::Str, "label", origin)?;
        let checked_value = values.take(&Type::Bool, "checked", origin)?;
        let disabled = values.optional(disabled.as_ref(), &Type::Bool, "disabled", origin)?;
        let options =
            self.resolve_boolean_options(&mut values, options, true, &facts, origin, span)?;
        let custom = self.resolve_boolean_custom_style(
            &mut values,
            BooleanCustomStyleSource {
                call: style.custom.as_ref(),
                function: facts.style,
                origin: facts.style_origin,
                kind: ExternKind::CheckboxStyle,
                parent: origin,
                span,
            },
        )?;
        let mut status_origins = facts.status_origins.iter().copied();
        let mut take_status = |source: &Option<CheckboxStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let status_origin = status_origins.next().ok_or_else(|| {
                        self.invariant(span, "checkbox status origin disappeared")
                    })?;
                    self.validate_boolean_origin(
                        status_origin,
                        source.span.as_ref().unwrap_or(span),
                        origin,
                        "checkbox status",
                    )?;
                    self.resolve_checkbox_status(&mut values, source, status_origin, span)
                })
                .transpose()
        };
        let styles = ResolvedCheckboxStyleSet {
            preset: Some(match style.preset {
                CheckboxStylePreset::Primary => ResolvedCheckboxPreset::Primary,
                CheckboxStylePreset::Secondary => ResolvedCheckboxPreset::Secondary,
                CheckboxStylePreset::Success => ResolvedCheckboxPreset::Success,
                CheckboxStylePreset::Danger => ResolvedCheckboxPreset::Danger,
            }),
            custom,
            active_checked: take_status(&style.active_checked)?,
            active_unchecked: take_status(&style.active_unchecked)?,
            hovered_checked: take_status(&style.hovered_checked)?,
            hovered_unchecked: take_status(&style.hovered_unchecked)?,
            disabled_checked: take_status(&style.disabled_checked)?,
            disabled_unchecked: take_status(&style.disabled_unchecked)?,
        };
        if status_origins.next().is_some() {
            return Err(self.invariant(span, "checkbox left status origins unconsumed"));
        }
        values.finish()?;
        let route = self.lower_boolean_route(route, &checked, id, scope, span)?;
        self.insert_boolean_control(
            ResolvedBooleanControl {
                id,
                kind: ResolvedBooleanKind::Checkbox,
                label,
                checked: checked_value,
                value: None,
                disabled,
                options,
                route,
                style: ResolvedBooleanStyle::Checkbox(Box::new(styles)),
                source_line: span.line,
                origin,
            },
            span,
        )
    }

    pub(super) fn lower_toggler(
        &mut self,
        source: TogglerSource<'_>,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let TogglerSource {
            id: raw_id,
            label,
            checked: checked_value,
            disabled,
            options,
            style,
            route,
            span,
        } = source;
        let roots =
            crate::ast::toggler_expression_roots(label, checked_value, disabled, options, style);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Toggler,
            crate::ast::toggler_semantic_key(
                raw_id,
                label,
                checked_value,
                disabled,
                options,
                style,
                route,
            ),
            span,
            outer_component,
        )?;
        let facts = self.checked_boolean_facts(id, span)?;
        self.validate_boolean_header(&checked, &facts, roots.len(), scope, span)?;
        let mut values = BooleanOperands {
            lowerer: self,
            control: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };
        let label = values.take(&Type::Str, "label", origin)?;
        let checked_value = values.take(&Type::Bool, "checked", origin)?;
        let disabled = values.optional(disabled.as_ref(), &Type::Bool, "disabled", origin)?;
        let options =
            self.resolve_boolean_options(&mut values, options, true, &facts, origin, span)?;
        let custom = self.resolve_boolean_custom_style(
            &mut values,
            BooleanCustomStyleSource {
                call: style.custom.as_ref(),
                function: facts.style,
                origin: facts.style_origin,
                kind: ExternKind::TogglerStyle,
                parent: origin,
                span,
            },
        )?;
        let mut status_origins = facts.status_origins.iter().copied();
        let mut take_status = |source: &Option<TogglerStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let status_origin = status_origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "toggler status origin disappeared"))?;
                    self.validate_boolean_origin(
                        status_origin,
                        source.span.as_ref().unwrap_or(span),
                        origin,
                        "toggler status",
                    )?;
                    self.resolve_toggler_status(&mut values, source, status_origin, span)
                })
                .transpose()
        };
        let styles = ResolvedTogglerStyleSet {
            custom,
            active_checked: take_status(&style.active_checked)?,
            active_unchecked: take_status(&style.active_unchecked)?,
            hovered_checked: take_status(&style.hovered_checked)?,
            hovered_unchecked: take_status(&style.hovered_unchecked)?,
            disabled_checked: take_status(&style.disabled_checked)?,
            disabled_unchecked: take_status(&style.disabled_unchecked)?,
        };
        if status_origins.next().is_some() {
            return Err(self.invariant(span, "toggler left status origins unconsumed"));
        }
        values.finish()?;
        let route = self.lower_boolean_route(route, &checked, id, scope, span)?;
        self.insert_boolean_control(
            ResolvedBooleanControl {
                id,
                kind: ResolvedBooleanKind::Toggler,
                label,
                checked: checked_value,
                value: None,
                disabled,
                options,
                route,
                style: ResolvedBooleanStyle::Toggler(Box::new(styles)),
                source_line: span.line,
                origin,
            },
            span,
        )
    }

    pub(super) fn lower_radio(
        &mut self,
        source: RadioSource<'_>,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let RadioSource {
            id: raw_id,
            label,
            value,
            selected,
            options,
            style,
            route,
            span,
        } = source;
        let roots = crate::ast::radio_expression_roots(label, value, selected, options, style);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Radio,
            crate::ast::radio_semantic_key(raw_id, label, value, selected, options, style, route),
            span,
            outer_component,
        )?;
        let facts = self.checked_boolean_facts(id, span)?;
        self.validate_boolean_header(&checked, &facts, roots.len(), scope, span)?;
        let mut values = BooleanOperands {
            lowerer: self,
            control: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };
        let label = values.take(&Type::Str, "label", origin)?;
        let (value, value_type) = values.take_where("radio value", origin, |actual| {
            matches!(
                actual,
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            )
        })?;
        let selected = values.take(&Type::Bool, "selected", origin)?;
        let options =
            self.resolve_boolean_options(&mut values, options, false, &facts, origin, span)?;
        let custom = self.resolve_boolean_custom_style(
            &mut values,
            BooleanCustomStyleSource {
                call: style.custom.as_ref(),
                function: facts.style,
                origin: facts.style_origin,
                kind: ExternKind::RadioStyle,
                parent: origin,
                span,
            },
        )?;
        let mut status_origins = facts.status_origins.iter().copied();
        let mut take_status = |source: &Option<RadioStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let status_origin = status_origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "radio status origin disappeared"))?;
                    self.validate_boolean_origin(
                        status_origin,
                        source.span.as_ref().unwrap_or(span),
                        origin,
                        "radio status",
                    )?;
                    self.resolve_radio_status(&mut values, source, status_origin, span)
                })
                .transpose()
        };
        let styles = ResolvedRadioStyleSet {
            custom,
            active_selected: take_status(&style.active_selected)?,
            active_unselected: take_status(&style.active_unselected)?,
            hovered_selected: take_status(&style.hovered_selected)?,
            hovered_unselected: take_status(&style.hovered_unselected)?,
        };
        if status_origins.next().is_some() {
            return Err(self.invariant(span, "radio left status origins unconsumed"));
        }
        values.finish()?;
        let route = self.lower_boolean_route(route, &checked, id, scope, span)?;
        if route.source_payloads != vec![value_type] {
            return Err(self.invariant(span, "radio route payload type diverged from its value"));
        }
        self.insert_boolean_control(
            ResolvedBooleanControl {
                id,
                kind: ResolvedBooleanKind::Radio,
                label,
                checked: selected,
                value: Some(value),
                disabled: None,
                options,
                route,
                style: ResolvedBooleanStyle::Radio(Box::new(styles)),
                source_line: span.line,
                origin,
            },
            span,
        )
    }

    fn checked_boolean_facts(
        &self,
        id: ViewId,
        span: &Span,
    ) -> Result<CheckedBooleanControl, Error> {
        self.facts
            .boolean_control(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "boolean control has no checked HIR facts"))
    }

    fn validate_boolean_header(
        &self,
        checked: &CheckedInteraction,
        facts: &CheckedBooleanControl,
        roots: usize,
        scope: CheckedViewScope,
        span: &Span,
    ) -> Result<(), Error> {
        if facts.id != checked.id || checked.option_expressions.len() != roots {
            return Err(self.invariant(
                span,
                "boolean control fact identity or cardinality diverged",
            ));
        }
        self.validate_interaction_expression_graphs(
            checked.id,
            scope,
            checked.expression_count,
            span,
        )
    }

    fn validate_boolean_origin(
        &self,
        id: OriginId,
        span: &Span,
        parent: OriginId,
        label: &str,
    ) -> Result<(), Error> {
        let origin = self
            .origins
            .try_get(id)
            .ok_or_else(|| self.invariant(span, format!("{label} origin is invalid")))?;
        let (expected_path, expected_line) = self
            .origins
            .source_origin(span.line)
            .map_or((None, span.line), |(path, line)| (Some(path), line));
        if origin.parent != Some(parent)
            || origin.path.as_deref() != expected_path
            || origin.line != expected_line
            || origin.column != span.column
        {
            return Err(self.invariant(span, format!("{label} origin diverged")));
        }
        Ok(())
    }

    fn resolve_boolean_options(
        &self,
        values: &mut BooleanOperands<'_>,
        options: &BoolControlOptions,
        include_accessibility: bool,
        facts: &CheckedBooleanControl,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedBooleanOptions, Error> {
        let accessibility_label = include_accessibility
            .then(|| {
                values.optional(
                    options.accessibility.label.as_ref(),
                    &Type::Str,
                    "accessibility label",
                    origin,
                )
            })
            .transpose()?
            .flatten();
        let accessibility_description = include_accessibility
            .then(|| {
                values.optional(
                    options.accessibility.description.as_ref(),
                    &Type::Str,
                    "accessibility description",
                    origin,
                )
            })
            .transpose()?
            .flatten();
        let size = values.optional(options.size.as_ref(), &Type::F64, "size", origin)?;
        let width = Self::resolve_boolean_length(values, &options.width, origin)?;
        let spacing = values.optional(options.spacing.as_ref(), &Type::F64, "spacing", origin)?;
        let text_size =
            values.optional(options.text_size.as_ref(), &Type::F64, "text size", origin)?;
        let line_height = values.optional(
            options.line_height.as_ref(),
            &Type::F64,
            "line height",
            origin,
        )?;
        let icon_size =
            values.optional(options.icon_size.as_ref(), &Type::F64, "icon size", origin)?;
        let icon_line_height = values.optional(
            options.icon_line_height.as_ref(),
            &Type::F64,
            "icon line height",
            origin,
        )?;
        let font_origin = match (options.font.as_ref(), facts.font_origin) {
            (Some(_), Some(font_origin)) => {
                self.validate_boolean_origin(font_origin, span, origin, "boolean font")?;
                Some(font_origin)
            }
            (None, None) => None,
            _ => return Err(self.invariant(span, "boolean font origin presence diverged")),
        };
        let font =
            self.resolve_text_font(options.font.as_ref(), font_origin.unwrap_or(origin), span)?;
        let icon = options.icon.map(|code_point| ResolvedBooleanIcon {
            code_point,
            size: icon_size,
            line_height: icon_line_height,
            shaping: options.icon_shaping.map(Self::resolve_boolean_shaping),
        });
        if icon.is_none() && (icon_size.is_some() || icon_line_height.is_some()) {
            return Err(self.invariant(span, "boolean icon operands survived without an icon"));
        }
        Ok(ResolvedBooleanOptions {
            accessibility_label,
            accessibility_description,
            size,
            width,
            spacing,
            text_size,
            line_height,
            shaping: options.shaping.map(Self::resolve_boolean_shaping),
            wrapping: options.wrapping.map(Self::resolve_boolean_wrapping),
            font,
            #[cfg(test)]
            font_origin,
            alignment: options.alignment.map(Self::resolve_boolean_alignment),
            icon,
        })
    }

    fn resolve_boolean_length(
        values: &mut BooleanOperands<'_>,
        length: &Option<LengthValue>,
        origin: OriginId,
    ) -> Result<Option<ResolvedContainerLength>, Error> {
        Ok(match length {
            None => None,
            Some(LengthValue::Fill) => Some(ResolvedContainerLength::Fill),
            Some(LengthValue::FillPortion(portion)) => {
                Some(ResolvedContainerLength::FillPortion(*portion))
            }
            Some(LengthValue::Shrink) => Some(ResolvedContainerLength::Shrink),
            Some(LengthValue::Fixed(_)) => {
                let (expression, source) = values.take_where("width", origin, |actual| {
                    matches!(actual, Type::F64 | Type::Length)
                })?;
                Some(match source {
                    Type::F64 => ResolvedContainerLength::FixedF64(expression),
                    Type::Length => ResolvedContainerLength::FixedLength(expression),
                    _ => unreachable!("checked boolean width type"),
                })
            }
        })
    }

    fn resolve_boolean_custom_style(
        &self,
        values: &mut BooleanOperands<'_>,
        source: BooleanCustomStyleSource<'_>,
    ) -> Result<Option<ResolvedBooleanCustomStyle>, Error> {
        let BooleanCustomStyleSource {
            call,
            function,
            origin: style_origin,
            kind,
            parent,
            span,
        } = source;
        match (call, function, style_origin) {
            (None, None, None) => Ok(None),
            (Some(source), Some(function), Some(style_origin)) => {
                self.validate_boolean_origin(style_origin, span, parent, "boolean extern")?;
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .ok_or_else(|| self.invariant(span, "boolean style extern ID is invalid"))?;
                if declaration.name != source.function
                    || declaration.kind != kind
                    || declaration.params.len() != source.args.len()
                {
                    return Err(self.invariant(span, "boolean style extern contract diverged"));
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "style argument", parent))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(ResolvedBooleanCustomStyle {
                    function,
                    arguments,
                    #[cfg(test)]
                    origin: style_origin,
                }))
            }
            _ => Err(self.invariant(span, "boolean style extern presence diverged")),
        }
    }

    fn resolve_checkbox_status(
        &self,
        values: &mut BooleanOperands<'_>,
        source: &CheckboxStatusStyle,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedCheckboxStatusStyle, Error> {
        Ok(ResolvedCheckboxStatusStyle {
            background: self.resolve_boolean_background(
                values,
                &source.background,
                origin,
                span,
            )?,
            icon_color: self.resolve_boolean_color(source.icon_color.as_deref(), origin, span)?,
            text_color: self.resolve_boolean_color(source.text_color.as_deref(), origin, span)?,
            border_color: self.resolve_boolean_color(
                source.border_color.as_deref(),
                origin,
                span,
            )?,
            border_width: values.optional(
                source.border_width.as_ref(),
                &Type::F64,
                "checkbox border width",
                origin,
            )?,
            radius: self.resolve_boolean_radius(
                values,
                &source.radius.all,
                [
                    &source.radius.top_left,
                    &source.radius.top_right,
                    &source.radius.bottom_right,
                    &source.radius.bottom_left,
                ],
                origin,
            )?,
            #[cfg(test)]
            origin,
        })
    }

    fn resolve_toggler_status(
        &self,
        values: &mut BooleanOperands<'_>,
        source: &TogglerStatusStyle,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedTogglerStatusStyle, Error> {
        Ok(ResolvedTogglerStatusStyle {
            background: self.resolve_boolean_background(
                values,
                &source.background,
                origin,
                span,
            )?,
            background_border_color: self.resolve_boolean_color(
                source.background_border_color.as_deref(),
                origin,
                span,
            )?,
            background_border_width: values.optional(
                source.background_border_width.as_ref(),
                &Type::F64,
                "toggler background border width",
                origin,
            )?,
            foreground: self.resolve_boolean_background(
                values,
                &source.foreground,
                origin,
                span,
            )?,
            foreground_border_color: self.resolve_boolean_color(
                source.foreground_border_color.as_deref(),
                origin,
                span,
            )?,
            foreground_border_width: values.optional(
                source.foreground_border_width.as_ref(),
                &Type::F64,
                "toggler foreground border width",
                origin,
            )?,
            text_color: self.resolve_boolean_color(source.text_color.as_deref(), origin, span)?,
            radius: self.resolve_boolean_radius(
                values,
                &source.radius.all,
                [
                    &source.radius.top_left,
                    &source.radius.top_right,
                    &source.radius.bottom_right,
                    &source.radius.bottom_left,
                ],
                origin,
            )?,
            padding_ratio: values.optional(
                source.padding_ratio.as_ref(),
                &Type::F64,
                "toggler padding ratio",
                origin,
            )?,
        })
    }

    fn resolve_radio_status(
        &self,
        values: &mut BooleanOperands<'_>,
        source: &RadioStatusStyle,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedRadioStatusStyle, Error> {
        Ok(ResolvedRadioStatusStyle {
            background: self.resolve_boolean_background(
                values,
                &source.background,
                origin,
                span,
            )?,
            dot_color: self.resolve_boolean_color(source.dot_color.as_deref(), origin, span)?,
            border_color: self.resolve_boolean_color(
                source.border_color.as_deref(),
                origin,
                span,
            )?,
            border_width: values.optional(
                source.border_width.as_ref(),
                &Type::F64,
                "radio border width",
                origin,
            )?,
            text_color: self.resolve_boolean_color(source.text_color.as_deref(), origin, span)?,
        })
    }

    fn resolve_boolean_background(
        &self,
        values: &mut BooleanOperands<'_>,
        source: &Option<BackgroundValue>,
        origin: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedBooleanBackground>, Error> {
        source
            .as_ref()
            .map(|source| {
                let value = match source {
                    BackgroundValue::Color(color) => {
                        ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
                    }
                    BackgroundValue::Linear { stops, .. } => {
                        let angle = values.take(&Type::F64, "gradient angle", origin)?;
                        let stops = stops
                            .iter()
                            .map(|stop| {
                                Ok(ResolvedContainerGradientStop {
                                    color: self.resolve_theme_color(&stop.color, span)?,
                                    offset: values.take(&Type::F64, "gradient stop", origin)?,
                                })
                            })
                            .collect::<Result<Vec<_>, Error>>()?;
                        ResolvedContainerBackground::Linear { angle, stops }
                    }
                };
                Ok(ResolvedBooleanBackground {
                    value,
                    #[cfg(test)]
                    origin,
                })
            })
            .transpose()
    }

    fn resolve_boolean_color(
        &self,
        source: Option<&str>,
        _origin: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedBooleanColor>, Error> {
        source
            .map(|source| {
                Ok(ResolvedBooleanColor {
                    value: self.resolve_theme_color(source, span)?,
                    #[cfg(test)]
                    origin: _origin,
                })
            })
            .transpose()
    }

    fn resolve_boolean_radius(
        &self,
        values: &mut BooleanOperands<'_>,
        all: &Option<Expr>,
        corners: [&Option<Expr>; 4],
        origin: OriginId,
    ) -> Result<ResolvedContainerRadius, Error> {
        Ok(ResolvedContainerRadius {
            all: values.optional(all.as_ref(), &Type::F64, "radius", origin)?,
            top_left: values.optional(
                corners[0].as_ref(),
                &Type::F64,
                "top-left radius",
                origin,
            )?,
            top_right: values.optional(
                corners[1].as_ref(),
                &Type::F64,
                "top-right radius",
                origin,
            )?,
            bottom_right: values.optional(
                corners[2].as_ref(),
                &Type::F64,
                "bottom-right radius",
                origin,
            )?,
            bottom_left: values.optional(
                corners[3].as_ref(),
                &Type::F64,
                "bottom-left radius",
                origin,
            )?,
        })
    }

    fn lower_boolean_route(
        &self,
        route: &Route,
        checked: &CheckedInteraction,
        id: ViewId,
        scope: CheckedViewScope,
        span: &Span,
    ) -> Result<ResolvedInteractionRoute, Error> {
        let source = Some(route.clone());
        let routes = source.iter().collect::<Vec<_>>();
        let mut index = 0usize;
        let resolved = self
            .lower_optional_interaction_route(&source, checked, &routes, &mut index, id, scope)?
            .ok_or_else(|| self.invariant(span, "boolean route disappeared"))?;
        if index != checked.routes.len() {
            return Err(self.invariant(span, "boolean left checked routes unconsumed"));
        }
        Ok(resolved)
    }

    fn insert_boolean_control(
        &mut self,
        control: ResolvedBooleanControl,
        span: &Span,
    ) -> Result<(), Error> {
        if self.boolean_controls.insert(control.id, control).is_some() {
            return Err(self.invariant(span, "boolean control was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_boolean_shaping(value: TextShaping) -> ResolvedTextShaping {
        match value {
            TextShaping::Auto => ResolvedTextShaping::Auto,
            TextShaping::Basic => ResolvedTextShaping::Basic,
            TextShaping::Advanced => ResolvedTextShaping::Advanced,
        }
    }

    fn resolve_boolean_wrapping(value: TextWrapping) -> ResolvedTextWrapping {
        match value {
            TextWrapping::None => ResolvedTextWrapping::None,
            TextWrapping::Word => ResolvedTextWrapping::Word,
            TextWrapping::Glyph => ResolvedTextWrapping::Glyph,
            TextWrapping::WordOrGlyph => ResolvedTextWrapping::WordOrGlyph,
        }
    }

    fn resolve_boolean_alignment(value: TextAlignment) -> ResolvedTextAlignment {
        match value {
            TextAlignment::Default => ResolvedTextAlignment::Default,
            TextAlignment::Left => ResolvedTextAlignment::Left,
            TextAlignment::Center => ResolvedTextAlignment::Center,
            TextAlignment::Right => ResolvedTextAlignment::Right,
            TextAlignment::Justified => ResolvedTextAlignment::Justified,
        }
    }
}
