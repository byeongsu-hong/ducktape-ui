use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedRuleAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedRulePreset {
    Default,
    Weak,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedRuleFill {
    Full,
    Percent(CheckedExprUseId),
    Padded(u16),
    AsymmetricPadding(u16, u16),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedRule {
    pub(crate) id: ViewId,
    pub(crate) axis: ResolvedRuleAxis,
    pub(crate) thickness: CheckedExprUseId,
    pub(crate) preset: ResolvedRulePreset,
    pub(crate) fill: Option<ResolvedRuleFill>,
    pub(crate) color: Option<ResolvedThemeColor>,
    pub(crate) radius: ResolvedContainerRadius,
    pub(crate) snap: Option<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedQrPayloadKind {
    Text,
    Bytes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedQrCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedQrVersion {
    Normal(u8),
    Micro(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedQrEncoding {
    Auto {
        correction: Option<ResolvedQrCorrection>,
    },
    Versioned {
        version: ResolvedQrVersion,
        correction: ResolvedQrCorrection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedQrSize {
    Default,
    Cell(CheckedExprUseId),
    Total(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedQrCode {
    pub(crate) id: ViewId,
    pub(crate) payload: CheckedExprUseId,
    #[cfg(test)]
    pub(crate) payload_kind: ResolvedQrPayloadKind,
    pub(crate) encoding: ResolvedQrEncoding,
    pub(crate) size: ResolvedQrSize,
    pub(crate) cell: Option<ResolvedThemeColor>,
    pub(crate) background: Option<ResolvedThemeColor>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSpace {
    pub(crate) id: ViewId,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) height: Option<ResolvedContainerLength>,
    pub(crate) origin: OriginId,
}

struct PrimitiveOperands<'a> {
    lowerer: &'a Lowerer,
    widget: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    parent: OriginId,
    span: &'a Span,
    family: &'static str,
}

impl PrimitiveOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
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
        let parent_origin = self.lowerer.origins.try_get(self.parent).ok_or_else(|| {
            self.lowerer
                .invariant(self.span, format!("{} origin is invalid", self.family))
        })?;
        if retained.owner != owner
            || self.lowerer.facts.expression_use_by_owner(owner) != Some(expression)
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
            || retained_origin.parent != Some(self.parent)
            || retained_origin.path != parent_origin.path
            || retained_origin.line != parent_origin.line
            || retained_origin.column != parent_origin.column
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("{} {label} expression contract diverged", self.family),
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
                format!("{} left checked expressions unconsumed", self.family),
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_rule(
        &mut self,
        axis: Axis,
        thickness: &Expr,
        options: &RuleOptions,
        styles: &[String],
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let roots = crate::ast::rule_expression_roots(thickness, options);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Rule,
            crate::ast::rule_semantic_key(axis, options, styles),
            span,
            outer_component,
        )?;
        self.facts
            .rule(id)
            .ok_or_else(|| self.invariant(span, "rule has no checked HIR facts"))?;
        if checked.option_expressions.len() != roots.len() || !checked.routes.is_empty() {
            return Err(self.invariant(span, "rule expression or route cardinality diverged"));
        }
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        let mut values = PrimitiveOperands {
            lowerer: self,
            widget: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            parent: origin,
            span,
            family: "rule",
        };
        let thickness = values.take(&Type::F64, "thickness")?;
        let fill = match &options.fill {
            None => None,
            Some(RuleFill::Full) => Some(ResolvedRuleFill::Full),
            Some(RuleFill::Percent(_)) => Some(ResolvedRuleFill::Percent(
                values.take(&Type::F64, "fill percent")?,
            )),
            Some(RuleFill::Padded(value)) => Some(ResolvedRuleFill::Padded(*value)),
            Some(RuleFill::AsymmetricPadding(first, second)) => {
                Some(ResolvedRuleFill::AsymmetricPadding(*first, *second))
            }
        };
        let radius = Self::resolve_primitive_radius(
            &mut values,
            &options.radius.all,
            [
                &options.radius.top_left,
                &options.radius.top_right,
                &options.radius.bottom_right,
                &options.radius.bottom_left,
            ],
            "radius",
        )?;
        let snap = values.optional(options.snap.as_ref(), &Type::Bool, "snap")?;
        values.finish()?;
        let rule = ResolvedRule {
            id,
            axis: match axis {
                Axis::Horizontal => ResolvedRuleAxis::Horizontal,
                Axis::Vertical => ResolvedRuleAxis::Vertical,
            },
            thickness,
            preset: match options.style.unwrap_or(RuleStyle::Default) {
                RuleStyle::Default => ResolvedRulePreset::Default,
                RuleStyle::Weak => ResolvedRulePreset::Weak,
            },
            fill,
            color: options
                .color
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            radius,
            snap,
            origin,
        };
        if self.rules.insert(id, rule).is_some() {
            return Err(self.invariant(span, "rule was lowered more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_qr_code(
        &mut self,
        payload: &Expr,
        correction: Option<QrCorrection>,
        version: Option<QrVersion>,
        cell_size: &Option<Expr>,
        total_size: &Option<Expr>,
        cell: &Option<String>,
        background: &Option<String>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let roots = crate::ast::qr_code_expression_roots(payload, cell_size, total_size);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::QrCode,
            crate::ast::qr_code_semantic_key(
                correction, version, cell_size, total_size, cell, background,
            ),
            span,
            outer_component,
        )?;
        let facts = self
            .facts
            .qr_code(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "qr has no checked HIR facts"))?;
        if checked.option_expressions.len() != roots.len() || !checked.routes.is_empty() {
            return Err(self.invariant(span, "qr expression or route cardinality diverged"));
        }
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        let mut values = PrimitiveOperands {
            lowerer: self,
            widget: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            parent: origin,
            span,
            family: "qr",
        };
        let _payload_kind = match &facts.payload_type {
            Type::Str => ResolvedQrPayloadKind::Text,
            Type::Bytes => ResolvedQrPayloadKind::Bytes,
            _ => return Err(self.invariant(span, "qr retained an invalid payload type")),
        };
        let payload = values.take(&facts.payload_type, "payload")?;
        let cell_size = values.optional(cell_size.as_ref(), &Type::F64, "cell size")?;
        let total_size = values.optional(total_size.as_ref(), &Type::F64, "total size")?;
        let size = match (cell_size, total_size) {
            (None, None) => ResolvedQrSize::Default,
            (Some(value), None) => ResolvedQrSize::Cell(value),
            (None, Some(value)) => ResolvedQrSize::Total(value),
            (Some(_), Some(_)) => {
                return Err(self.invariant(span, "qr retained conflicting size modes"));
            }
        };
        values.finish()?;
        let correction = correction.map(Self::resolve_qr_correction);
        let encoding = match version {
            None => ResolvedQrEncoding::Auto { correction },
            Some(version) => ResolvedQrEncoding::Versioned {
                version: self.resolve_qr_version(version, span)?,
                correction: correction.unwrap_or(ResolvedQrCorrection::Medium),
            },
        };
        let qr = ResolvedQrCode {
            id,
            payload,
            #[cfg(test)]
            payload_kind: _payload_kind,
            encoding,
            size,
            cell: cell
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            background: background
                .as_deref()
                .map(|color| self.resolve_theme_color(color, span))
                .transpose()?,
            origin,
        };
        if self.qr_codes.insert(id, qr).is_some() {
            return Err(self.invariant(span, "qr was lowered more than once"));
        }
        Ok(())
    }

    pub(super) fn lower_space(
        &mut self,
        width: &Option<LengthValue>,
        height: &Option<LengthValue>,
        styles: &[String],
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let roots = crate::ast::space_expression_roots(width, height);
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Space,
            crate::ast::space_semantic_key(width, height, styles),
            span,
            outer_component,
        )?;
        self.facts
            .space(id)
            .ok_or_else(|| self.invariant(span, "space has no checked HIR facts"))?;
        if checked.option_expressions.len() != roots.len() || !checked.routes.is_empty() {
            return Err(self.invariant(span, "space expression or route cardinality diverged"));
        }
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        let mut values = PrimitiveOperands {
            lowerer: self,
            widget: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            parent: origin,
            span,
            family: "space",
        };
        let width = Self::resolve_primitive_length(&mut values, width, "width")?;
        let height = Self::resolve_primitive_length(&mut values, height, "height")?;
        values.finish()?;
        if self
            .spaces
            .insert(
                id,
                ResolvedSpace {
                    id,
                    width,
                    height,
                    origin,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "space was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_primitive_length(
        values: &mut PrimitiveOperands<'_>,
        source: &Option<LengthValue>,
        label: &str,
    ) -> Result<Option<ResolvedContainerLength>, Error> {
        Ok(match source {
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
                    _ => unreachable!("validated primitive length type"),
                })
            }
        })
    }

    fn resolve_primitive_radius(
        values: &mut PrimitiveOperands<'_>,
        all: &Option<Expr>,
        corners: [&Option<Expr>; 4],
        label: &str,
    ) -> Result<ResolvedContainerRadius, Error> {
        Ok(ResolvedContainerRadius {
            all: values.optional(all.as_ref(), &Type::F64, label)?,
            top_left: values.optional(corners[0].as_ref(), &Type::F64, label)?,
            top_right: values.optional(corners[1].as_ref(), &Type::F64, label)?,
            bottom_right: values.optional(corners[2].as_ref(), &Type::F64, label)?,
            bottom_left: values.optional(corners[3].as_ref(), &Type::F64, label)?,
        })
    }

    fn resolve_qr_correction(value: QrCorrection) -> ResolvedQrCorrection {
        match value {
            QrCorrection::Low => ResolvedQrCorrection::Low,
            QrCorrection::Medium => ResolvedQrCorrection::Medium,
            QrCorrection::Quartile => ResolvedQrCorrection::Quartile,
            QrCorrection::High => ResolvedQrCorrection::High,
        }
    }

    fn resolve_qr_version(
        &self,
        value: QrVersion,
        span: &Span,
    ) -> Result<ResolvedQrVersion, Error> {
        match value {
            QrVersion::Normal(value @ 1..=40) => Ok(ResolvedQrVersion::Normal(value)),
            QrVersion::Micro(value @ 1..=4) => Ok(ResolvedQrVersion::Micro(value)),
            _ => Err(self.invariant(span, "qr retained an invalid version")),
        }
    }
}
