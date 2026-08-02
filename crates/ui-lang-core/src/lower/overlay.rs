// Stable view IDs, route contracts, and origins remain part of the normalized
// contract even when the current backend does not inspect every field.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedOverlayAlignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedOverlay {
    pub(crate) id: ViewId,
    pub(crate) visible: CheckedExprUseId,
    pub(crate) padding: CheckedExprUseId,
    pub(crate) backdrop: ResolvedThemeColor,
    pub(crate) dismiss: Option<ResolvedInteractionRoute>,
    pub(crate) align_x: ResolvedOverlayAlignment,
    pub(crate) align_y: ResolvedOverlayAlignment,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn lower_overlay(
        &mut self,
        options: &OverlayOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::Overlay,
            crate::ast::overlay_semantic_key(options),
            span,
            outer_component,
        )?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;

        let [visible, padding] = checked.option_expressions.as_slice() else {
            return Err(self.invariant(span, "overlay expression cardinality diverged"));
        };
        for (expression, expected, label) in [
            (*visible, Type::Bool, "visibility"),
            (*padding, Type::F64, "padding"),
        ] {
            let retained = self.facts.try_expression_use(expression).ok_or_else(|| {
                self.invariant(span, format!("overlay {label} expression is invalid"))
            })?;
            if retained.source != expected || retained.destination != expected {
                return Err(
                    self.invariant(span, format!("overlay {label} expression changed type"))
                );
            }
        }

        let routes = crate::ast::overlay_routes(options);
        let mut route = 0usize;
        let dismiss = self.lower_optional_interaction_route(
            &options.dismiss,
            &checked,
            &routes,
            &mut route,
            id,
            scope,
        )?;
        if route != checked.routes.len() {
            return Err(self.invariant(span, "overlay left checked routes unconsumed"));
        }

        let alignment = |value| match value {
            FlexAlignment::Start => ResolvedOverlayAlignment::Start,
            FlexAlignment::Center => ResolvedOverlayAlignment::Center,
            FlexAlignment::End => ResolvedOverlayAlignment::End,
        };
        let resolved = ResolvedOverlay {
            id,
            visible: *visible,
            padding: *padding,
            backdrop: self.resolve_theme_color(&options.backdrop, span)?,
            dismiss,
            align_x: alignment(options.align_x),
            align_y: alignment(options.align_y),
            origin,
        };
        if self.overlays.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "overlay was lowered more than once"));
        }
        Ok(())
    }
}
