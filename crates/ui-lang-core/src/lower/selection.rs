use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedSelectionShaping {
    Auto,
    Basic,
    Advanced,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSelectionCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedMenuStyle {
    pub(crate) custom: Option<ResolvedSelectionCustomStyle>,
    pub(crate) surface: Option<ResolvedContainerSurface>,
    pub(crate) selected_text_color: Option<ResolvedThemeColor>,
    pub(crate) selected_background: Option<ResolvedContainerBackground>,
    #[cfg(test)]
    pub(crate) origin: Option<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPickListStatusStyle {
    pub(crate) surface: ResolvedContainerSurface,
    pub(crate) placeholder_color: Option<ResolvedThemeColor>,
    pub(crate) handle_color: Option<ResolvedThemeColor>,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedPickListStyleSet {
    pub(crate) active: Option<ResolvedPickListStatusStyle>,
    pub(crate) hovered: Option<ResolvedPickListStatusStyle>,
    pub(crate) opened: Option<ResolvedPickListStatusStyle>,
    pub(crate) opened_hovered: Option<ResolvedPickListStatusStyle>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPickListIcon {
    pub(crate) code_point: char,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<CheckedExprUseId>,
    pub(crate) shaping: Option<ResolvedSelectionShaping>,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedPickListHandle {
    Arrow {
        size: Option<CheckedExprUseId>,
    },
    Static(ResolvedPickListIcon),
    Dynamic {
        closed: ResolvedPickListIcon,
        open: ResolvedPickListIcon,
    },
    None,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPickList {
    pub(crate) id: ViewId,
    pub(crate) options: CheckedExprUseId,
    pub(crate) selected: CheckedExprUseId,
    #[cfg(test)]
    pub(crate) option_type: Type,
    pub(crate) placeholder: Option<CheckedExprUseId>,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) menu_height: Option<ResolvedContainerLength>,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) text_size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<CheckedExprUseId>,
    pub(crate) shaping: Option<ResolvedSelectionShaping>,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) handle: Option<ResolvedPickListHandle>,
    pub(crate) selection: ResolvedInteractionRoute,
    pub(crate) open: Option<ResolvedInteractionRoute>,
    pub(crate) close: Option<ResolvedInteractionRoute>,
    pub(crate) custom_style: Option<ResolvedSelectionCustomStyle>,
    pub(crate) styles: ResolvedPickListStyleSet,
    pub(crate) menu: ResolvedMenuStyle,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedComboState {
    pub(crate) id: CheckedValueRef,
    pub(crate) name: String,
    pub(crate) option_type: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedComboBox {
    pub(crate) id: ViewId,
    pub(crate) state: ResolvedComboState,
    pub(crate) selected: CheckedExprUseId,
    pub(crate) placeholder: String,
    pub(crate) width: Option<ResolvedContainerLength>,
    pub(crate) menu_height: Option<ResolvedContainerLength>,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) text_size: Option<CheckedExprUseId>,
    pub(crate) line_height: Option<CheckedExprUseId>,
    pub(crate) shaping: Option<ResolvedSelectionShaping>,
    pub(crate) font: Option<ResolvedTextFont>,
    pub(crate) icon: Option<ResolvedInputIcon>,
    pub(crate) selection: ResolvedInteractionRoute,
    pub(crate) input: Option<ResolvedInteractionRoute>,
    pub(crate) hover: Option<ResolvedInteractionRoute>,
    pub(crate) open: Option<ResolvedInteractionRoute>,
    pub(crate) close: Option<ResolvedInteractionRoute>,
    pub(crate) custom_style: Option<ResolvedSelectionCustomStyle>,
    pub(crate) styles: ResolvedInputStyleSet,
    pub(crate) menu: ResolvedMenuStyle,
    pub(crate) origin: OriginId,
}

struct SelectionOperands<'a> {
    lowerer: &'a Lowerer,
    widget: ViewId,
    expressions: std::slice::Iter<'a, CheckedExprUseId>,
    next: u32,
    span: &'a Span,
}

impl SelectionOperands<'_> {
    fn take_where(
        &mut self,
        label: &str,
        expected: impl FnOnce(&Type) -> bool,
    ) -> Result<(CheckedExprUseId, Type), Error> {
        let expression = *self.expressions.next().ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                format!("selection {label} expression disappeared"),
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
                    format!("selection {label} expression ID is invalid"),
                )
            })?;
        if retained.owner != owner
            || retained.destination != retained.source
            || !expected(&retained.source)
            || self.lowerer.facts.try_expression(retained.root).is_none()
        {
            return Err(self.lowerer.invariant(
                self.span,
                format!("selection {label} expression contract diverged"),
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
                "selection left checked option expressions unconsumed",
            ));
        }
        Ok(())
    }
}

impl Lowerer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_pick_list(
        &mut self,
        options: &Expr,
        selected: &Expr,
        config: &PickListOptions,
        route: &Route,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::PickList,
            crate::ast::pick_list_semantic_key(config, route),
            span,
            outer_component,
        )?;
        let facts = self
            .facts
            .pick_list(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "pick list has no checked HIR facts"))?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        if checked.option_expressions.len()
            != crate::ast::pick_list_expression_roots(options, selected, config).len()
        {
            return Err(self.invariant(span, "pick list expression cardinality diverged"));
        }
        let mut values = SelectionOperands {
            lowerer: self,
            widget: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };
        let (options, options_type) =
            values.take_where("options", |ty| matches!(ty, Type::List(_)))?;
        let Type::List(option_type) = options_type else {
            unreachable!()
        };
        let selected = values.take(&Type::Option(option_type.clone()), "selected value")?;
        let placeholder =
            values.optional(config.placeholder.as_ref(), &Type::Str, "placeholder")?;
        let width = Self::resolve_selection_length(&mut values, &config.width, "width")?;
        let menu_height =
            Self::resolve_selection_length(&mut values, &config.menu_height, "menu height")?;
        let padding = values.optional(config.padding.as_ref(), &Type::F64, "padding")?;
        let text_size = values.optional(config.text_size.as_ref(), &Type::F64, "text size")?;
        let line_height =
            values.optional(config.line_height.as_ref(), &Type::F64, "line height")?;
        let handle = self.resolve_pick_handle(&mut values, &config.handle, &facts, origin, span)?;
        let custom_style = self.resolve_selection_custom_style(
            &mut values,
            config.custom_style.as_ref(),
            facts.style,
            ExternKind::PickListStyle,
            "pick style",
            span,
        )?;
        let custom_menu_style = self.resolve_selection_custom_style(
            &mut values,
            config.custom_menu_style.as_ref(),
            facts.menu_style,
            ExternKind::MenuStyle,
            "pick menu style",
            span,
        )?;
        let styles = self.resolve_pick_styles(
            &mut values,
            &config.style,
            &facts.status_origins,
            origin,
            span,
        )?;
        let menu = self.resolve_selection_menu(
            &mut values,
            config.menu_style.as_deref(),
            custom_menu_style,
            facts.menu_origin,
            origin,
            span,
        )?;
        values.finish()?;

        let routes = crate::ast::pick_list_routes(config, route);
        if routes
            .first()
            .is_none_or(|candidate| !std::ptr::eq(*candidate, route))
        {
            return Err(self.invariant(span, "pick selection route order diverged"));
        }
        let selection = self.lower_interaction_route(route, &checked, 0, id, scope)?;
        let mut route_index = 1usize;
        let open = self.lower_optional_interaction_route(
            &config.open,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        let close = self.lower_optional_interaction_route(
            &config.close,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != checked.routes.len() {
            return Err(self.invariant(span, "pick list left checked routes unconsumed"));
        }
        let resolved = ResolvedPickList {
            id,
            options,
            selected,
            #[cfg(test)]
            option_type: *option_type,
            placeholder,
            width,
            menu_height,
            padding,
            text_size,
            line_height,
            shaping: config.shaping.map(Self::resolve_selection_shaping),
            font: self.resolve_text_font(config.font.as_ref(), origin, span)?,
            handle,
            selection,
            open,
            close,
            custom_style,
            styles,
            menu,
            origin,
        };
        if self.pick_lists.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "pick list was lowered more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_combo_box(
        &mut self,
        state: &str,
        selected: &Expr,
        placeholder: &str,
        options: &ComboBoxOptions,
        route: &Route,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let (id, checked, scope, origin) = self.interaction_contract(
            CheckedInteractionKind::ComboBox,
            crate::ast::combo_box_semantic_key(state, placeholder, options, route),
            span,
            outer_component,
        )?;
        let facts = self
            .facts
            .combo_box(id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "combo box has no checked HIR facts"))?;
        self.validate_interaction_expression_graphs(id, scope, checked.expression_count, span)?;
        if checked.option_expressions.len()
            != crate::ast::combo_box_expression_roots(selected, options).len()
        {
            return Err(self.invariant(span, "combo expression cardinality diverged"));
        }
        let binding = self.resolve_combo_state(facts.binding, state, outer_component, span)?;
        let mut values = SelectionOperands {
            lowerer: self,
            widget: id,
            expressions: checked.option_expressions.iter(),
            next: 0,
            span,
        };
        let selected = values.take(
            &Type::Option(Box::new(binding.option_type.clone())),
            "selected value",
        )?;
        let width = Self::resolve_selection_length(&mut values, &options.width, "width")?;
        let menu_height =
            Self::resolve_selection_length(&mut values, &options.menu_height, "menu height")?;
        let padding = values.optional(options.padding.as_ref(), &Type::F64, "padding")?;
        let text_size = values.optional(options.text_size.as_ref(), &Type::F64, "text size")?;
        let line_height =
            values.optional(options.line_height.as_ref(), &Type::F64, "line height")?;
        let icon = options
            .icon
            .as_ref()
            .map(|icon| self.resolve_combo_icon(&mut values, icon, &facts, origin))
            .transpose()?;
        if icon.is_none() != facts.icon_origin.is_none() {
            return Err(self.invariant(span, "combo icon origin presence diverged"));
        }
        let custom_style = self.resolve_selection_custom_style(
            &mut values,
            options.custom_style.as_ref(),
            facts.style,
            ExternKind::InputStyle,
            "combo style",
            span,
        )?;
        let custom_menu_style = self.resolve_selection_custom_style(
            &mut values,
            options.custom_menu_style.as_ref(),
            facts.menu_style,
            ExternKind::MenuStyle,
            "combo menu style",
            span,
        )?;
        let styles = self.resolve_combo_styles(
            &mut values,
            &options.style,
            &facts.status_origins,
            origin,
            span,
        )?;
        let menu = self.resolve_selection_menu(
            &mut values,
            options.menu_style.as_deref(),
            custom_menu_style,
            facts.menu_origin,
            origin,
            span,
        )?;
        values.finish()?;

        let routes = crate::ast::combo_box_routes(options, route);
        if routes
            .first()
            .is_none_or(|candidate| !std::ptr::eq(*candidate, route))
        {
            return Err(self.invariant(span, "combo selection route order diverged"));
        }
        let selection = self.lower_interaction_route(route, &checked, 0, id, scope)?;
        let mut route_index = 1usize;
        let input = self.lower_optional_interaction_route(
            &options.input,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        let hover = self.lower_optional_interaction_route(
            &options.hover,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        let open = self.lower_optional_interaction_route(
            &options.open,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        let close = self.lower_optional_interaction_route(
            &options.close,
            &checked,
            &routes,
            &mut route_index,
            id,
            scope,
        )?;
        if route_index != checked.routes.len() {
            return Err(self.invariant(span, "combo left checked routes unconsumed"));
        }
        let resolved = ResolvedComboBox {
            id,
            state: binding,
            selected,
            placeholder: placeholder.to_owned(),
            width,
            menu_height,
            padding,
            text_size,
            line_height,
            shaping: options.shaping.map(Self::resolve_selection_shaping),
            font: self.resolve_text_font(options.font.as_ref(), origin, span)?,
            icon,
            selection,
            input,
            hover,
            open,
            close,
            custom_style,
            styles,
            menu,
            origin,
        };
        if self.combo_boxes.insert(id, resolved).is_some() {
            return Err(self.invariant(span, "combo box was lowered more than once"));
        }
        Ok(())
    }

    fn resolve_combo_state(
        &self,
        binding: CheckedValueRef,
        expected_name: &str,
        outer_component: Option<ComponentId>,
        span: &Span,
    ) -> Result<ResolvedComboState, Error> {
        let value = self
            .facts
            .try_value_by_ref(binding)
            .ok_or_else(|| self.invariant(span, "combo binding value ID is invalid"))?;
        let Type::Combo(option_type) = &value.ty else {
            return Err(self.invariant(span, "combo binding type diverged"));
        };
        if value.name != expected_name {
            return Err(self.invariant(span, "combo binding identity diverged"));
        }
        let valid_scope = match binding {
            CheckedValueRef::Secret(_) => false,
            CheckedValueRef::AppState(_) => outer_component.is_none(),
            CheckedValueRef::ComponentParam(id) => outer_component == Some(id.component),
            CheckedValueRef::ComponentState(id) => outer_component == Some(id.component),
            CheckedValueRef::Derived(_) => false,
        };
        if !valid_scope {
            return Err(self.invariant(span, "combo binding scope diverged"));
        }
        Ok(ResolvedComboState {
            id: binding,
            name: value.name.clone(),
            option_type: (**option_type).clone(),
        })
    }

    fn resolve_combo_icon(
        &self,
        values: &mut SelectionOperands<'_>,
        icon: &TextInputIcon,
        checked: &CheckedComboBox,
        parent: OriginId,
    ) -> Result<ResolvedInputIcon, Error> {
        let origin = checked
            .icon_origin
            .ok_or_else(|| self.invariant(&icon.span, "combo icon origin disappeared"))?;
        self.validate_selection_origin(origin, parent, &icon.span, "combo icon")?;
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

    fn resolve_pick_handle(
        &self,
        values: &mut SelectionOperands<'_>,
        handle: &Option<PickListHandle>,
        checked: &CheckedPickList,
        parent: OriginId,
        span: &Span,
    ) -> Result<Option<ResolvedPickListHandle>, Error> {
        let mut origins = checked.handle_origins.iter().copied();
        let mut icon = |source: &PickListIcon| {
            let origin = origins
                .next()
                .ok_or_else(|| self.invariant(&source.span, "pick icon origin disappeared"))?;
            self.validate_selection_origin(origin, parent, &source.span, "pick icon")?;
            Ok(ResolvedPickListIcon {
                code_point: source.code_point,
                font: self.resolve_text_font(source.font.as_ref(), origin, &source.span)?,
                size: values.optional(source.size.as_ref(), &Type::F64, "handle icon size")?,
                line_height: values.optional(
                    source.line_height.as_ref(),
                    &Type::F64,
                    "handle icon line height",
                )?,
                shaping: source.shaping.map(Self::resolve_selection_shaping),
                #[cfg(test)]
                origin,
            })
        };
        let resolved = match handle {
            None => None,
            Some(PickListHandle::Arrow { size }) => Some(ResolvedPickListHandle::Arrow {
                size: values.optional(size.as_ref(), &Type::F64, "handle arrow size")?,
            }),
            Some(PickListHandle::Static(source)) => {
                Some(ResolvedPickListHandle::Static(icon(source)?))
            }
            Some(PickListHandle::Dynamic { closed, open }) => {
                Some(ResolvedPickListHandle::Dynamic {
                    closed: icon(closed)?,
                    open: icon(open)?,
                })
            }
            Some(PickListHandle::None) => Some(ResolvedPickListHandle::None),
        };
        if origins.next().is_some() {
            return Err(self.invariant(span, "pick left handle origins unconsumed"));
        }
        Ok(resolved)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_selection_custom_style(
        &self,
        values: &mut SelectionOperands<'_>,
        source: Option<&ExternCall>,
        checked: Option<ExternFnId>,
        kind: ExternKind,
        label: &str,
        span: &Span,
    ) -> Result<Option<ResolvedSelectionCustomStyle>, Error> {
        let resolved = source
            .map(|source| {
                let function = checked.ok_or_else(|| {
                    self.invariant(span, format!("{label} lost its checked extern ID"))
                })?;
                let declaration = self
                    .declarations
                    .try_extern_decl(function)
                    .ok_or_else(|| self.invariant(span, format!("{label} extern disappeared")))?;
                if declaration.name != source.function
                    || declaration.kind != kind
                    || declaration.params.len() != source.args.len()
                {
                    return Err(self.invariant(span, format!("{label} extern contract diverged")));
                }
                let arguments = declaration
                    .params
                    .iter()
                    .map(|(_, expected)| values.take(expected, "style argument"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedSelectionCustomStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        if resolved.is_none() != checked.is_none() {
            return Err(self.invariant(span, format!("{label} presence diverged")));
        }
        Ok(resolved)
    }

    fn resolve_pick_styles(
        &self,
        values: &mut SelectionOperands<'_>,
        styles: &PickListStyleSet,
        origins: &[OriginId],
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedPickListStyleSet, Error> {
        let mut origins = origins.iter().copied();
        let mut resolve = |source: &Option<PickListStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let origin = origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "pick status origin disappeared"))?;
                    let source_span = source.span.as_ref().unwrap_or(span);
                    self.validate_selection_origin(origin, parent, source_span, "pick status")?;
                    Ok(ResolvedPickListStatusStyle {
                        surface: self.resolve_selection_surface(values, &source.options, span)?,
                        placeholder_color: source
                            .placeholder_color
                            .as_deref()
                            .map(|color| self.resolve_theme_color(color, source_span))
                            .transpose()?,
                        handle_color: source
                            .handle_color
                            .as_deref()
                            .map(|color| self.resolve_theme_color(color, source_span))
                            .transpose()?,
                        #[cfg(test)]
                        origin,
                    })
                })
                .transpose()
        };
        let resolved = ResolvedPickListStyleSet {
            active: resolve(&styles.active)?,
            hovered: resolve(&styles.hovered)?,
            opened: resolve(&styles.opened)?,
            opened_hovered: resolve(&styles.opened_hovered)?,
        };
        if origins.next().is_some() {
            return Err(self.invariant(span, "pick left status origins unconsumed"));
        }
        Ok(resolved)
    }

    fn resolve_combo_styles(
        &self,
        values: &mut SelectionOperands<'_>,
        styles: &TextInputStyleSet,
        origins: &[OriginId],
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedInputStyleSet, Error> {
        let mut origins = origins.iter().copied();
        let mut resolve = |source: &Option<TextInputStatusStyle>| {
            source
                .as_ref()
                .map(|source| {
                    let origin = origins
                        .next()
                        .ok_or_else(|| self.invariant(span, "combo status origin disappeared"))?;
                    let source_span = source.span.as_ref().unwrap_or(span);
                    self.validate_selection_origin(origin, parent, source_span, "combo status")?;
                    Ok(ResolvedInputStatusStyle {
                        surface: self.resolve_selection_surface(values, &source.options, span)?,
                        icon_color: source
                            .icon_color
                            .as_deref()
                            .map(|color| self.resolve_theme_color(color, source_span))
                            .transpose()?,
                        placeholder_color: source
                            .placeholder_color
                            .as_deref()
                            .map(|color| self.resolve_theme_color(color, source_span))
                            .transpose()?,
                        value_color: source
                            .value_color
                            .as_deref()
                            .map(|color| self.resolve_theme_color(color, source_span))
                            .transpose()?,
                        selection_color: source
                            .selection_color
                            .as_deref()
                            .map(|color| self.resolve_theme_color(color, source_span))
                            .transpose()?,
                        #[cfg(test)]
                        origin,
                    })
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
            return Err(self.invariant(span, "combo left status origins unconsumed"));
        }
        Ok(resolved)
    }

    fn resolve_selection_menu(
        &self,
        values: &mut SelectionOperands<'_>,
        source: Option<&MenuStyleOptions>,
        custom: Option<ResolvedSelectionCustomStyle>,
        origin: Option<OriginId>,
        parent: OriginId,
        span: &Span,
    ) -> Result<ResolvedMenuStyle, Error> {
        let (surface, selected_text_color, selected_background) = source.map_or_else(
            || Ok((None, None, None)),
            |source| {
                let retained =
                    origin.ok_or_else(|| self.invariant(span, "menu style origin disappeared"))?;
                let source_span = source.span.as_ref().unwrap_or(span);
                self.validate_selection_origin(retained, parent, source_span, "menu style")?;
                Ok((
                    Some(self.resolve_selection_surface(values, &source.options, source_span)?),
                    source
                        .selected_text_color
                        .as_deref()
                        .map(|color| self.resolve_theme_color(color, source_span))
                        .transpose()?,
                    source
                        .selected_background
                        .as_ref()
                        .map(|background| {
                            self.resolve_selection_background(values, background, source_span)
                        })
                        .transpose()?,
                ))
            },
        )?;
        if source.is_none() != origin.is_none() {
            return Err(self.invariant(span, "menu style origin presence diverged"));
        }
        Ok(ResolvedMenuStyle {
            custom,
            surface,
            selected_text_color,
            selected_background,
            #[cfg(test)]
            origin,
        })
    }

    fn resolve_selection_surface(
        &self,
        values: &mut SelectionOperands<'_>,
        surface: &ContainerStyleOptions,
        span: &Span,
    ) -> Result<ResolvedContainerSurface, Error> {
        Ok(ResolvedContainerSurface {
            background: surface
                .background
                .as_ref()
                .map(|background| self.resolve_selection_background(values, background, span))
                .transpose()?,
            background_alpha: None,
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

    fn resolve_selection_background(
        &self,
        values: &mut SelectionOperands<'_>,
        background: &BackgroundValue,
        span: &Span,
    ) -> Result<ResolvedContainerBackground, Error> {
        Ok(match background {
            BackgroundValue::Color(color) => {
                ResolvedContainerBackground::Color(self.resolve_theme_color(color, span)?)
            }
            BackgroundValue::Linear { stops, .. } => {
                let angle = values.take(&Type::F64, "background angle")?;
                let stops = stops
                    .iter()
                    .map(|stop| {
                        Ok(ResolvedContainerGradientStop {
                            color: self.resolve_theme_color(&stop.color, span)?,
                            offset: values.take(&Type::F64, "background stop")?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                ResolvedContainerBackground::Linear { angle, stops }
            }
        })
    }

    fn resolve_selection_length(
        values: &mut SelectionOperands<'_>,
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
                    _ => unreachable!("validated selection length type"),
                })
            }
        })
    }

    fn resolve_selection_shaping(shaping: TextShaping) -> ResolvedSelectionShaping {
        match shaping {
            TextShaping::Auto => ResolvedSelectionShaping::Auto,
            TextShaping::Basic => ResolvedSelectionShaping::Basic,
            TextShaping::Advanced => ResolvedSelectionShaping::Advanced,
        }
    }

    fn validate_selection_origin(
        &self,
        origin: OriginId,
        parent: OriginId,
        source: &Span,
        label: &str,
    ) -> Result<(), Error> {
        let retained = self.origins.try_get(origin).ok_or_else(|| {
            self.invariant(source, format!("{label} origin is outside its arena"))
        })?;
        let (expected_path, expected_line) = self
            .origins
            .source_origin(source.line)
            .map_or((None, source.line), |(path, line)| (Some(path), line));
        if retained.parent != Some(parent)
            || retained.path.as_deref() != expected_path
            || retained.line != expected_line
            || retained.column != source.column
        {
            return Err(self.invariant(source, format!("{label} origin diverged")));
        }
        Ok(())
    }
}
