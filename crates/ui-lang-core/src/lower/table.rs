use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTableBinding {
    pub(crate) local: CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTableLength {
    Fill,
    FillPortion(u16),
    Shrink,
    FixedF64(CheckedExprUseId),
    FixedLength(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTableColumn {
    pub(crate) width: Option<ResolvedTableLength>,
    pub(crate) align_x: Option<InputAlignment>,
    pub(crate) align_y: Option<VerticalAlignment>,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTable {
    pub(crate) id: ViewId,
    pub(crate) rows: CheckedExprUseId,
    pub(crate) row: ResolvedTableBinding,
    pub(crate) width: Option<ResolvedTableLength>,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) padding_x: Option<CheckedExprUseId>,
    pub(crate) padding_y: Option<CheckedExprUseId>,
    pub(crate) separator: Option<CheckedExprUseId>,
    pub(crate) separator_x: Option<CheckedExprUseId>,
    pub(crate) separator_y: Option<CheckedExprUseId>,
    pub(crate) columns: Vec<ResolvedTableColumn>,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_table(
        &mut self,
        _item: &str,
        _rows: &Expr,
        options: &TableOptions,
        raw_columns: &[TableColumn],
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "table has no shared view ID"))?;
        let checked_view = self.facts.view(id).clone();
        let CheckedViewFlow::Table {
            rows,
            item: local,
            layout,
        } = checked_view.flow
        else {
            return Err(self.invariant(span, "table has no checked HIR facts"));
        };
        let expected_scope = match checked_view.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component
            || layout.semantic_key != crate::ast::table_semantic_key(options, raw_columns)
            || layout.columns.len() != raw_columns.len()
        {
            return Err(self.invariant(span, "table topology diverged after semantic checking"));
        }

        let checked_row = self
            .facts
            .try_local(local)
            .ok_or_else(|| self.invariant(span, "table row local ID is outside its arena"))?;
        if checked_row.owner
            != (CheckedLocalOwner::View {
                view: id,
                role: CheckedViewLocalRole::TableRow,
            })
        {
            return Err(self.invariant(span, "table row binding contract diverged"));
        }
        let rows_ty = self.validate_table_expression(
            id,
            checked_view.scope,
            rows,
            CheckedViewExprRole::TableRows,
            span,
        )?;
        let Type::List(inner) = rows_ty else {
            return Err(self.invariant(span, "table rows type is not a list"));
        };
        if *inner != checked_row.ty {
            return Err(self.invariant(span, "table row type diverged from its list"));
        }

        let width = self.resolve_table_length(
            id,
            checked_view.scope,
            layout.width,
            CheckedViewExprRole::TableWidth,
            span,
        )?;
        for (value, role) in [
            (layout.padding, CheckedViewExprRole::TablePadding),
            (layout.padding_x, CheckedViewExprRole::TablePaddingX),
            (layout.padding_y, CheckedViewExprRole::TablePaddingY),
            (layout.separator, CheckedViewExprRole::TableSeparator),
            (layout.separator_x, CheckedViewExprRole::TableSeparatorX),
            (layout.separator_y, CheckedViewExprRole::TableSeparatorY),
        ] {
            if let Some(value) = value {
                let ty =
                    self.validate_table_expression(id, checked_view.scope, value, role, span)?;
                if ty != Type::F64 {
                    return Err(self.invariant(span, "table metric type is not f64"));
                }
            }
        }

        let mut columns = Vec::with_capacity(layout.columns.len());
        for (index, column) in layout.columns.into_iter().enumerate() {
            let origin = self.origins.try_get(column.origin).ok_or_else(|| {
                self.invariant(span, "table column origin ID is outside its arena")
            })?;
            if origin.parent != Some(checked_view.origin) {
                return Err(
                    self.invariant_at_origin(column.origin, "table column origin parent diverged")
                );
            }
            let width = self.resolve_table_length(
                id,
                checked_view.scope,
                column.width,
                CheckedViewExprRole::TableColumnWidth(index as u32),
                span,
            )?;
            columns.push(ResolvedTableColumn {
                width,
                align_x: column.align_x,
                align_y: column.align_y,
                #[cfg(test)]
                origin: column.origin,
            });
        }

        let resolved = ResolvedTable {
            id,
            rows,
            row: ResolvedTableBinding {
                local,
                name: checked_row.name.clone(),
                ty: checked_row.ty.clone(),
            },
            width,
            padding: layout.padding,
            padding_x: layout.padding_x,
            padding_y: layout.padding_y,
            separator: layout.separator,
            separator_x: layout.separator_x,
            separator_y: layout.separator_y,
            columns,
            origin: checked_view.origin,
        };
        if self.tables.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "table was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_table_length(
        &self,
        view: ViewId,
        scope: CheckedViewScope,
        length: CheckedLength,
        role: CheckedViewExprRole,
        span: &Span,
    ) -> Result<Option<ResolvedTableLength>, Error> {
        Ok(match length {
            CheckedLength::None => None,
            CheckedLength::Fill => Some(ResolvedTableLength::Fill),
            CheckedLength::FillPortion(portion) => Some(ResolvedTableLength::FillPortion(portion)),
            CheckedLength::Shrink => Some(ResolvedTableLength::Shrink),
            CheckedLength::Fixed { expression, source } => {
                let actual = self.validate_table_expression(view, scope, expression, role, span)?;
                if actual != source {
                    return Err(self.invariant(span, "table length type contract diverged"));
                }
                match source {
                    Type::F64 => Some(ResolvedTableLength::FixedF64(expression)),
                    Type::Length => Some(ResolvedTableLength::FixedLength(expression)),
                    _ => return Err(self.invariant(span, "table length has invalid type")),
                }
            }
        })
    }

    fn validate_table_expression(
        &self,
        view: ViewId,
        scope: CheckedViewScope,
        use_id: CheckedExprUseId,
        role: CheckedViewExprRole,
        span: &Span,
    ) -> Result<Type, Error> {
        let owner = CheckedExprOwner::View { view, role };
        if self.facts.expression_use_by_owner(owner) != Some(use_id) {
            return Err(self.invariant(span, "table expression owner mapping diverged"));
        }
        let expression = self
            .facts
            .try_expression_use(use_id)
            .ok_or_else(|| self.invariant(span, "table expression-use ID is outside its arena"))?;
        if expression.owner != owner
            || !checked_expression_coercion_is_valid(
                &expression.source,
                &expression.destination,
                &expression.coercion,
            )
        {
            return Err(self.invariant(span, "table expression contract diverged"));
        }
        let policy = ViewWidgetExpressionPolicy {
            lowerer: self,
            view,
            scope,
            use_id,
            span,
            canvas_locals: false,
            own_view_locals: false,
            allowed_own_view_locals: None,
            family: "table",
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
            return Err(self.invariant(span, "table expression root type diverged"));
        }
        Ok(source)
    }
}
