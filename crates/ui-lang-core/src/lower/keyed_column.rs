use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedKeyedBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedKeyedLength {
    Fill,
    FillPortion(u16),
    Shrink,
    FixedF64(CheckedExprUseId),
    FixedLength(CheckedExprUseId),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedKeyedPadding {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) x: Option<CheckedExprUseId>,
    pub(crate) y: Option<CheckedExprUseId>,
    pub(crate) top: Option<CheckedExprUseId>,
    pub(crate) right: Option<CheckedExprUseId>,
    pub(crate) bottom: Option<CheckedExprUseId>,
    pub(crate) left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedKeyedColumn {
    pub(crate) id: ViewId,
    pub(crate) items: CheckedExprUseId,
    pub(crate) key: CheckedExprUseId,
    pub(crate) item: ResolvedKeyedBinding,
    pub(crate) width: Option<ResolvedKeyedLength>,
    pub(crate) height: Option<ResolvedKeyedLength>,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) padding: ResolvedKeyedPadding,
    pub(crate) max_width: Option<CheckedExprUseId>,
    pub(crate) virtual_row: Option<CheckedExprUseId>,
    pub(crate) align: Option<FlexAlignment>,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_keyed_column(
        &mut self,
        item: &str,
        _items: &Expr,
        _key: &Expr,
        options: &LayoutOptions,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "keyed column has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::Keyed {
            items,
            key,
            item: local,
            layout,
        } = checked_view.flow
        else {
            return Err(self.invariant(span, "keyed column has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component
            || layout.semantic_key != crate::ast::keyed_column_semantic_key(options)
        {
            return Err(self.invariant(
                span,
                "keyed column topology diverged after semantic checking",
            ));
        }

        let checked_item = self
            .facts
            .try_local(local)
            .ok_or_else(|| self.invariant(span, "keyed item local ID is outside its arena"))?;
        if checked_item.name != item
            || checked_item.owner
                != (CheckedLocalOwner::View {
                    view: id,
                    role: CheckedViewLocalRole::KeyedItem,
                })
        {
            return Err(self.invariant(span, "keyed item binding contract diverged"));
        }
        let items_ty = self.validate_keyed_expression(
            id,
            checked_view.scope,
            items,
            CheckedViewExprRole::KeyedItems,
            false,
            span,
        )?;
        let Type::List(inner) = items_ty else {
            return Err(self.invariant(span, "keyed items type is not a list"));
        };
        if *inner != checked_item.ty {
            return Err(self.invariant(span, "keyed item type diverged from its list"));
        }
        let key_ty = self.validate_keyed_expression(
            id,
            checked_view.scope,
            key,
            CheckedViewExprRole::KeyedKey,
            true,
            span,
        )?;
        if !matches!(key_ty, Type::Bool | Type::I64 | Type::F64) {
            return Err(self.invariant(span, "keyed key type is not copyable"));
        }

        let width = self.resolve_keyed_length(
            id,
            checked_view.scope,
            layout.width,
            CheckedViewExprRole::KeyedWidth,
            span,
        )?;
        let height = self.resolve_keyed_length(
            id,
            checked_view.scope,
            layout.height,
            CheckedViewExprRole::KeyedHeight,
            span,
        )?;
        for (value, role) in [
            (layout.spacing, CheckedViewExprRole::KeyedSpacing),
            (layout.padding.all, CheckedViewExprRole::KeyedPaddingAll),
            (layout.padding.x, CheckedViewExprRole::KeyedPaddingX),
            (layout.padding.y, CheckedViewExprRole::KeyedPaddingY),
            (layout.padding.top, CheckedViewExprRole::KeyedPaddingTop),
            (layout.padding.right, CheckedViewExprRole::KeyedPaddingRight),
            (
                layout.padding.bottom,
                CheckedViewExprRole::KeyedPaddingBottom,
            ),
            (layout.padding.left, CheckedViewExprRole::KeyedPaddingLeft),
            (layout.max_width, CheckedViewExprRole::KeyedMaxWidth),
            (layout.virtual_row, CheckedViewExprRole::KeyedVirtualRow),
        ] {
            if let Some(value) = value {
                let ty = self.validate_keyed_expression(
                    id,
                    checked_view.scope,
                    value,
                    role,
                    false,
                    span,
                )?;
                if ty != Type::F64 {
                    return Err(self.invariant(span, "keyed metric type is not f64"));
                }
            }
        }

        let resolved = ResolvedKeyedColumn {
            id,
            items,
            key,
            item: ResolvedKeyedBinding {
                local,
                name: checked_item.name.clone(),
            },
            width,
            height,
            spacing: layout.spacing,
            padding: ResolvedKeyedPadding {
                all: layout.padding.all,
                x: layout.padding.x,
                y: layout.padding.y,
                top: layout.padding.top,
                right: layout.padding.right,
                bottom: layout.padding.bottom,
                left: layout.padding.left,
            },
            max_width: layout.max_width,
            virtual_row: layout.virtual_row,
            align: layout.align,
            origin: checked_view.origin,
        };
        if self.keyed_columns.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "keyed column was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_keyed_length(
        &self,
        view: ViewId,
        scope: CheckedViewScope,
        length: CheckedKeyedLength,
        role: CheckedViewExprRole,
        span: &Span,
    ) -> Result<Option<ResolvedKeyedLength>, Error> {
        Ok(match length {
            CheckedKeyedLength::None => None,
            CheckedKeyedLength::Fill => Some(ResolvedKeyedLength::Fill),
            CheckedKeyedLength::FillPortion(portion) => {
                Some(ResolvedKeyedLength::FillPortion(portion))
            }
            CheckedKeyedLength::Shrink => Some(ResolvedKeyedLength::Shrink),
            CheckedKeyedLength::Fixed { expression, source } => {
                let actual =
                    self.validate_keyed_expression(view, scope, expression, role, false, span)?;
                if actual != source {
                    return Err(self.invariant(span, "keyed length type contract diverged"));
                }
                match source {
                    Type::F64 => Some(ResolvedKeyedLength::FixedF64(expression)),
                    Type::Length => Some(ResolvedKeyedLength::FixedLength(expression)),
                    _ => return Err(self.invariant(span, "keyed length has invalid type")),
                }
            }
        })
    }

    fn validate_keyed_expression(
        &self,
        view: ViewId,
        scope: CheckedViewScope,
        use_id: CheckedExprUseId,
        role: CheckedViewExprRole,
        own_view_locals: bool,
        span: &Span,
    ) -> Result<Type, Error> {
        let owner = CheckedExprOwner::View { view, role };
        if self.facts.expression_use_by_owner(owner) != Some(use_id) {
            return Err(self.invariant(span, "keyed expression owner mapping diverged"));
        }
        let expression = self
            .facts
            .try_expression_use(use_id)
            .ok_or_else(|| self.invariant(span, "keyed expression-use ID is outside its arena"))?;
        if expression.owner != owner
            || !checked_expression_coercion_is_valid(
                &expression.source,
                &expression.destination,
                &expression.coercion,
            )
        {
            return Err(self.invariant(span, "keyed expression contract diverged"));
        }
        let policy = ViewWidgetExpressionPolicy {
            lowerer: self,
            view,
            scope,
            use_id,
            span,
            canvas_locals: false,
            own_view_locals,
            allowed_own_view_locals: None,
            family: "keyed column",
        };
        let mut graph = CheckedExpressionGraph::default();
        let root_scope = graph.root_scope();
        let source = self.validate_checked_expression_node(
            expression.root,
            &policy,
            &mut graph,
            root_scope,
        )?;
        if source != expression.source {
            return Err(self.invariant(span, "keyed expression root type diverged"));
        }
        Ok(source)
    }
}
