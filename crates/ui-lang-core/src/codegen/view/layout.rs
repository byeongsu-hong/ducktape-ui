use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_layout(
    kind: Layout,
    options: &LayoutOptions,
    id: &Option<Id>,
    children: &[ViewNode],
    span: &Span,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let style = &document.program().style_use(span)?.style;
    if kind == Layout::Grid && options.min_cell.is_some() {
        let mut flex_options = options.clone();
        flex_options.width.get_or_insert(LengthValue::Fill);
        flex_options.flexbox = Some(FlexboxOptions {
            direction: FlexDirectionValue::Row,
            wrap: FlexWrapValue::Wrap,
            ..FlexboxOptions::default()
        });
        return render_flexbox(
            &flex_options,
            id,
            children,
            span,
            document,
            message,
            env,
            scope,
            slot,
        );
    }
    let accessibility_key =
        accessibility_key_code(id.as_ref(), "layout", span, scope, env, document)?;
    if kind == Layout::Scroll {
        let child_scope = id.as_ref().map_or_else(
            || Ok(scope.to_owned()),
            |id| id_code(id, scope, env, document),
        )?;
        let child = render_node(&children[0], document, message, env, &child_scope, slot)?;
        let mut code = String::from("::iced::widget::scrollable(__scroll_content)");
        let scroll = options.scroll.as_ref().expect("scroll options");
        let bar = scroll_bar_code(scroll, env, document)?;
        let direction = match scroll.direction {
            ScrollDirection::Vertical => {
                format!("::iced::widget::scrollable::Direction::Vertical({bar})")
            }
            ScrollDirection::Horizontal => {
                format!("::iced::widget::scrollable::Direction::Horizontal({bar})")
            }
            ScrollDirection::Both => format!(
                "::iced::widget::scrollable::Direction::Both {{ vertical: {bar}, horizontal: {bar} }}"
            ),
        };
        write!(code, ".direction({direction})").unwrap();
        if let Some(id) = id {
            write!(
                code,
                ".id(::iced::widget::Id::from({}))",
                id_code(id, scope, env, document)?
            )
            .unwrap();
        }
        let anchor = |anchor| match anchor {
            ScrollAnchor::Start => "Start",
            ScrollAnchor::End => "End",
        };
        write!(
            code,
            ".anchor_x(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_x)
        )
        .unwrap();
        write!(
            code,
            ".anchor_y(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_y)
        )
        .unwrap();
        if let Some(auto_scroll) = &scroll.auto_scroll {
            write!(
                code,
                ".auto_scroll({})",
                expr_code(auto_scroll, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(route) = &scroll.route {
            let callback = route_callback_with_code(
                route,
                "__viewport",
                env,
                document,
                |callback_env| {
                    let message_code = ordered_route_code(
                        route,
                        &[
                            "__absolute.x as f64",
                            "__absolute.y as f64",
                            "__relative.x as f64",
                            "__relative.y as f64",
                        ],
                        callback_env,
                        document,
                        message,
                    )?;
                    Ok(format!(
                        "{{ let __absolute = __viewport.absolute_offset(); let __relative = __viewport.relative_offset(); {message_code} }}"
                    ))
                },
            )?;
            write!(code, ".on_scroll({callback})").unwrap();
        } else if let Some(route) = &scroll.viewport_route {
            let callback = route_callback_with_code(
                route,
                "__viewport",
                env,
                document,
                |callback_env| {
                    let message_code = ordered_route_code(
                        route,
                        &[
                            "__absolute.x as f64",
                            "__absolute.y as f64",
                            "__reversed.x as f64",
                            "__reversed.y as f64",
                            "__relative.x as f64",
                            "__relative.y as f64",
                            "__bounds.x as f64",
                            "__bounds.y as f64",
                            "__bounds.width as f64",
                            "__bounds.height as f64",
                            "__content_bounds.x as f64",
                            "__content_bounds.y as f64",
                            "__content_bounds.width as f64",
                            "__content_bounds.height as f64",
                        ],
                        callback_env,
                        document,
                        message,
                    )?;
                    Ok(format!(
                        "{{ let __absolute = __viewport.absolute_offset(); let __reversed = __viewport.absolute_offset_reversed(); let __relative = __viewport.relative_offset(); let __bounds = __viewport.bounds(); let __content_bounds = __viewport.content_bounds(); {message_code} }}"
                    ))
                },
            )?;
            write!(code, ".on_scroll({callback})").unwrap();
        }
        code.push_str(&scroll_style_code(
            &scroll.styles,
            scroll.custom_style.as_ref(),
            env,
            document,
        )?);
        append_size(&mut code, style);
        append_dimensions(&mut code, [&scroll.width, &scroll.height], env, document)?;
        return Ok(format!(
            "{{ let __a11y_key = {accessibility_key}; let __scroll_content: __IceElement<'_, {message}> = {child}; let __layout = {code}; ::ui_lang_runtime::accessible(__layout, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}"
        ));
    }

    if options.flexbox.is_some() {
        return render_flexbox(
            options, id, children, span, document, message, env, scope, slot,
        );
    }

    let mut body = String::from("{ let mut __children: ::std::vec::Vec<__IceElement<'_, ");
    write!(body, "{message}>> = ::std::vec::Vec::new();").unwrap();
    let child_scope = id.as_ref().map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )?;
    render_children(
        &mut body,
        children,
        document,
        message,
        env,
        &child_scope,
        slot,
    )?;
    let needs_child_count = matches!(kind, Layout::Column | Layout::Row)
        && (!options.wrap || options.spacing.is_some() || options.wrap_spacing.is_some())
        || kind == Layout::Grid && options.spacing.is_some();
    if needs_child_count {
        body.push_str(" let __child_count = __children.len();");
    }
    if kind == Layout::Grid
        && let Some(columns) = &options.columns
    {
        write!(
            body,
            " let __grid_columns = usize::try_from({}).unwrap_or(0).max(1);",
            expr_code(columns, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if matches!(kind, Layout::Column | Layout::Row) && !options.wrap {
        let horizontal = kind == Layout::Row;
        write!(
            body,
            " let __children = __children.into_iter().map(|__child| ::ui_lang_runtime::bounded_fill_element(__child, __child_count, {horizontal})).collect::<::std::vec::Vec<_>>();"
        )
        .unwrap();
    }
    let constructor = match kind {
        Layout::Column => "column",
        Layout::Row => "row",
        Layout::Grid => "grid",
        Layout::Stack => "stack",
        Layout::Scroll => unreachable!("scroll returned above"),
    };
    if kind == Layout::Stack && options.under > 0 {
        write!(
            body,
            " let __under = ({} as usize).min(__children.len()); let __above = __children.split_off(__under); let __layout = __above.into_iter().fold(::iced::widget::Stack::new(), |__stack, __child| __stack.push(__child)); let __layout = __children.into_iter().rev().fold(__layout, |__stack, __child| __stack.push_under(__child))",
            options.under
        )
        .unwrap();
    } else if kind == Layout::Stack {
        // A z-stack sizes to the bounding box of every layer (see
        // `ui_lang_runtime::zstack`); iced's native `Stack` would let the first
        // layer alone dictate the size and collapse a small-first-layer popover.
        body.push_str(" let __layout = ::ui_lang_runtime::zstack(__children)");
    } else {
        write!(
            body,
            " let __layout = ::iced::widget::{constructor}(__children)"
        )
        .unwrap();
    }
    if let Some(gap) = style.gap {
        write!(body, ".spacing({gap})").unwrap();
    }
    if matches!(kind, Layout::Column | Layout::Row)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    if style.items_center {
        if kind == Layout::Column {
            body.push_str(".align_x(::iced::Center)");
        } else {
            body.push_str(".align_y(::iced::Center)");
        }
    }
    if kind == Layout::Grid {
        if let Some(spacing) = &options.spacing {
            let entries = if options.columns.is_some() {
                "__child_count.max(__grid_columns)"
            } else {
                "__child_count"
            };
            write!(
                body,
                ".spacing(::ui_lang_runtime::bounded_spacing({}, {entries}))",
                expr_code(spacing, env, document, ValueMode::Owned)?,
            )
            .unwrap();
        }
        if let Some(width) = &options.width {
            let LengthValue::Fixed(width) = width else {
                unreachable!("grid widths are always fixed")
            };
            write!(
                body,
                ".width({})",
                clamped_f32_code(width, "0.0", "f32::MAX", env, document)?
            )
            .unwrap();
        }
        if let Some(height) = &options.grid_height {
            match height {
                GridSizing::AspectRatio { width, height } => {
                    let width = expr_code(width, env, document, ValueMode::Owned)?;
                    let height = expr_code(height, env, document, ValueMode::Owned)?;
                    write!(
                        body,
                        ".height(::iced::widget::grid::Sizing::AspectRatio(((({width}) / ({height})) as f32).max(f32::EPSILON).min(f32::MAX)))"
                    )
                    .unwrap();
                }
                GridSizing::EvenlyDistribute(length) => {
                    write!(body, ".height({})", length_code(length, env, document)?).unwrap();
                }
            }
        }
        if let Some(max_cell) = &options.max_cell {
            write!(
                body,
                ".fluid({})",
                clamped_f32_code(max_cell, "f32::EPSILON", "f32::MAX", env, document)?
            )
            .unwrap();
        } else if options.columns.is_some() {
            body.push_str(".columns(__grid_columns)");
        }
    }
    if matches!(kind, Layout::Column | Layout::Row) {
        if let Some(spacing) = &options.spacing {
            write!(
                body,
                ".spacing(::ui_lang_runtime::bounded_spacing({}, __child_count))",
                expr_code(spacing, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
            write!(body, ".padding({padding})").unwrap();
        }
        append_dimensions(&mut body, [&options.width, &options.height], env, document)?;
        if let Some(max_width) = &options.max_width {
            write!(
                body,
                ".max_width({} as f32)",
                expr_code(max_width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(align) = options.align {
            let alignment = match (kind, align) {
                (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                (Layout::Column, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                (Layout::Column, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Vertical::Center",
                (Layout::Row, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                _ => unreachable!("only row and column reach flex alignment"),
            };
            let method = if kind == Layout::Column {
                "align_x"
            } else {
                "align_y"
            };
            write!(body, ".{method}({alignment})").unwrap();
        }
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        } else if style.clip {
            body.push_str(".clip(true)");
        }
        if options.wrap {
            body.push_str(".wrap()");
            if let Some(spacing) = &options.wrap_spacing {
                let method = if kind == Layout::Column {
                    "horizontal_spacing"
                } else {
                    "vertical_spacing"
                };
                write!(
                    body,
                    ".{method}(::ui_lang_runtime::bounded_spacing({}, __child_count))",
                    expr_code(spacing, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(align) = options.wrap_align {
                let alignment = match (kind, align) {
                    (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                    (Layout::Column, FlexAlignment::Center) => {
                        "::iced::alignment::Vertical::Center"
                    }
                    (Layout::Column, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                    (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                    (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                    (Layout::Row, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                    _ => unreachable!("only row and column can wrap"),
                };
                write!(body, ".align_x({alignment})").unwrap();
            }
        }
    }
    if kind == Layout::Stack {
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        } else if style.clip {
            body.push_str(".clip(true)");
        }
        append_dimensions(&mut body, [&options.width, &options.height], env, document)?;
        append_size(&mut body, style);
    }
    body.push(';');
    body.push_str(" let __content = ::iced::widget::container(__layout)");
    if kind == Layout::Grid && style.clip {
        body.push_str(".clip(true)");
    }
    if matches!(kind, Layout::Grid | Layout::Stack)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    append_size(&mut body, style);
    if let Some(max_width) = style.max_width {
        write!(body, ".max_width({max_width})").unwrap();
    }
    body.push_str(&container_style_code(style));
    body.push(';');
    if style.self_center {
        write!(body, " let __layout_content: __IceElement<'_, {message}> = ::iced::widget::container(__content).width(::iced::Fill).center_x(::iced::Fill).into();").unwrap();
    } else {
        write!(
            body,
            " let __layout_content: __IceElement<'_, {message}> = __content.into();"
        )
        .unwrap();
    }
    write!(body, " let __a11y_key = {accessibility_key}; ::ui_lang_runtime::accessible(__layout_content, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}").unwrap();
    Ok(body)
}

#[allow(clippy::too_many_arguments)]
fn render_flexbox(
    options: &LayoutOptions,
    id: &Option<Id>,
    children: &[ViewNode],
    span: &Span,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let flexbox = options.flexbox.as_ref().expect("flexbox options");
    let style = &document.program().style_use(span)?.style;
    let accessibility_key =
        accessibility_key_code(id.as_ref(), "layout", span, scope, env, document)?;
    let child_scope = id.as_ref().map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )?;
    let mut body = String::from("{ let mut __items = ::std::vec::Vec::new();");
    render_flex_children(
        &mut body,
        children,
        document,
        message,
        env,
        &child_scope,
        slot,
        options.min_cell.as_ref(),
    )?;
    write!(
        body,
        " let __layout = ::ui_lang_runtime::flex(__items).direction(::ui_lang_runtime::FlexDirection::{})",
        flex_direction_name(flexbox.direction)
    )
    .unwrap();
    if flexbox.wrap != FlexWrapValue::NoWrap {
        write!(
            body,
            ".wrap(::ui_lang_runtime::FlexWrap::{})",
            match flexbox.wrap {
                FlexWrapValue::NoWrap => unreachable!(),
                FlexWrapValue::Wrap => "Wrap",
                FlexWrapValue::WrapReverse => "WrapReverse",
            }
        )
        .unwrap();
    }
    if let Some(justify) = flexbox.justify_content {
        write!(
            body,
            ".justify_content(::ui_lang_runtime::JustifyContent::{})",
            flex_content_alignment_name(justify)
        )
        .unwrap();
    }
    if let Some(align) = flexbox.align_items {
        write!(
            body,
            ".align_items(::ui_lang_runtime::AlignItems::{})",
            flex_item_alignment_name(align)
        )
        .unwrap();
    } else if style.items_center {
        body.push_str(".align_items(::ui_lang_runtime::AlignItems::Center)");
    }
    if let Some(align) = flexbox.align_content {
        write!(
            body,
            ".align_content(::ui_lang_runtime::AlignContent::{})",
            flex_content_alignment_name(align)
        )
        .unwrap();
    }
    if let Some(gap) = style.gap {
        write!(body, ".gap({gap}.0)").unwrap();
    }
    if let Some(gap) = &options.spacing {
        write!(
            body,
            ".gap({} as f32)",
            expr_code(gap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(gap) = &options.wrap_spacing {
        let method = match flexbox.direction {
            FlexDirectionValue::Row | FlexDirectionValue::RowReverse => "row_gap",
            FlexDirectionValue::Column | FlexDirectionValue::ColumnReverse => "column_gap",
        };
        write!(
            body,
            ".{method}({} as f32)",
            expr_code(gap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    for (gap, method) in [
        (&flexbox.row_gap, "row_gap"),
        (&flexbox.column_gap, "column_gap"),
    ] {
        if let Some(gap) = gap {
            write!(
                body,
                ".{method}({} as f32)",
                expr_code(gap, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(padding) = style.padding_code() {
        write!(body, ".padding({padding})").unwrap();
    }
    if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
        write!(body, ".padding({padding})").unwrap();
    }
    if style.width_fill {
        body.push_str(".width(::iced::Fill)");
    }
    if style.height_fill {
        body.push_str(".height(::iced::Fill)");
    }
    append_dimensions(&mut body, [&options.width, &options.height], env, document)?;
    if let Some(max_width) = &options.max_width {
        write!(
            body,
            ".max_width({} as f32)",
            expr_code(max_width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    } else if let Some(max_width) = style.max_width {
        write!(body, ".max_width({max_width}.0)").unwrap();
    }
    if let Some(max_height) = &options.max_height {
        write!(
            body,
            ".max_height({} as f32)",
            expr_code(max_height, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(clip) = &options.clip {
        write!(
            body,
            ".clip({})",
            expr_code(clip, env, document, ValueMode::Owned)?
        )
        .unwrap();
    } else if style.clip {
        body.push_str(".clip(true)");
    }
    body.push(';');
    body.push_str(" let __content = ::iced::widget::container(__layout)");
    append_size(&mut body, style);
    if let Some(max_width) = style.max_width {
        write!(body, ".max_width({max_width})").unwrap();
    }
    body.push_str(&container_style_code(style));
    body.push_str("; let __layout_content: __IceElement<'_, ");
    write!(body, "{message}> = __content.into();").unwrap();
    write!(body, " let __a11y_key = {accessibility_key}; ::ui_lang_runtime::accessible(__layout_content, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}").unwrap();
    Ok(body)
}

#[allow(clippy::too_many_arguments)]
fn render_flex_children(
    out: &mut String,
    children: &[ViewNode],
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
    min_cell: Option<&Expr>,
) -> Result<(), Error> {
    for child in children {
        match child {
            ViewNode::If { children, .. } => {
                let program = document.hir();
                let conditional = program.resolved_conditional_for(child)?;
                let condition =
                    checked_expr_use_code(program, conditional.condition, env, ValueMode::Owned)?;
                if condition == "false" {
                    continue;
                }
                if condition == "true" {
                    render_flex_children(
                        out, children, document, message, env, scope, slot, min_cell,
                    )?;
                    continue;
                }
                write!(out, " if {condition} {{").unwrap();
                render_flex_children(out, children, document, message, env, scope, slot, min_cell)?;
                out.push_str(" }");
            }
            ViewNode::For { children, span, .. } => {
                let CheckedViewFlow::For { items, item } =
                    &document.program().checked_view(span)?.flow
                else {
                    return Err(Error::new("E196", span, "flex for has no checked flow"));
                };
                let local = document.program().checked_facts().local(*item);
                let item_name = &local.name;
                let items =
                    checked_expr_use_code(document.program(), *items, env, ValueMode::Borrowed)?;
                let reconciliation_scope = reconciliation_scope(scope, env);
                write!(
                    out,
                    " for (__ice_index, {item_name}) in {items}.iter().cloned().enumerate() {{ let __for_scope = format!(\"{{}}/@for:{}({{}})\", {reconciliation_scope}, __ice_index);",
                    span.line
                )
                .unwrap();
                let mut child_env = ScopedBindingEnv::new(env);
                child_env.insert(
                    item_name.clone(),
                    checked_local_binding(document.program(), *item, item_name.clone(), false),
                );
                child_env.insert(
                    RECONCILIATION_SCOPE_BINDING.into(),
                    reconciliation_scope_binding("__for_scope.clone()".into()),
                );
                render_flex_children(
                    out, children, document, message, &child_env, scope, slot, min_cell,
                )?;
                out.push_str(" }");
            }
            ViewNode::Match { arms, span, .. } => {
                let CheckedViewFlow::Match {
                    value,
                    arms: checked_arms,
                } = &document.program().checked_view(span)?.flow
                else {
                    return Err(Error::new("E196", span, "flex match has no checked flow"));
                };
                if arms.len() != checked_arms.len() {
                    return Err(Error::new(
                        "E196",
                        span,
                        "flex match arm arena length diverged",
                    ));
                }
                let value_ty = document
                    .program()
                    .checked_facts()
                    .expression_use(*value)
                    .source
                    .clone();
                let value =
                    checked_expr_use_code(document.program(), *value, env, ValueMode::Borrowed)?;
                write!(out, " match &({value}) {{").unwrap();
                for (arm, checked_arm) in arms.iter().zip(checked_arms) {
                    write!(
                        out,
                        " {} => {{",
                        checked_match_pattern_code(
                            document.program(),
                            &checked_arm.pattern,
                            checked_arm.binding,
                            &value_ty,
                            &arm.span,
                        )?
                    )
                    .unwrap();
                    let mut child_env = ScopedBindingEnv::new(env);
                    if let Some(local) = checked_arm.binding {
                        let name = document.program().checked_facts().local(local).name.clone();
                        child_env.insert(
                            name.clone(),
                            checked_local_binding(document.program(), local, name, false),
                        );
                    }
                    render_flex_children(
                        out,
                        &arm.children,
                        document,
                        message,
                        &child_env,
                        scope,
                        slot,
                        min_cell,
                    )?;
                    out.push_str(" },");
                }
                out.push_str(" }");
            }
            _ => {
                let Some(rendered) =
                    render_node_if_present(child, document, message, env, scope, slot)?
                else {
                    continue;
                };
                let item = if let Some(min_cell) = min_cell {
                    format!(
                        "::ui_lang_runtime::flex_item(__flex_child).grow(1.0).shrink(0.0).basis(::ui_lang_runtime::FlexBasis::Fixed({}))",
                        clamped_f32_code(min_cell, "f32::EPSILON", "f32::MAX", env, document,)?
                    )
                } else {
                    let options = match child {
                        ViewNode::Container { options, .. } => Some(&options.flex_item),
                        _ => None,
                    };
                    flex_item_code("__flex_child", options, env, document)?
                };
                write!(
                    out,
                    " let __flex_child: __IceElement<'_, {message}> = {rendered}; __items.push({item});"
                )
                .unwrap();
            }
        }
    }
    Ok(())
}

fn flex_item_code(
    child: &str,
    options: Option<&FlexItemOptions>,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let mut code = format!("::ui_lang_runtime::flex_item({child})");
    let Some(options) = options else {
        return Ok(code);
    };
    if let Some(order) = &options.order {
        write!(
            code,
            ".order({} as i64)",
            expr_code(order, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    for (value, method) in [(&options.grow, "grow"), (&options.shrink, "shrink")] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(basis) = &options.basis {
        let basis = match basis {
            FlexBasisValue::Auto => "::ui_lang_runtime::FlexBasis::Auto".to_owned(),
            FlexBasisValue::Content => "::ui_lang_runtime::FlexBasis::Content".to_owned(),
            FlexBasisValue::Fixed(value) => format!(
                "::ui_lang_runtime::FlexBasis::Fixed({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            FlexBasisValue::Percent(value) => format!(
                "::ui_lang_runtime::FlexBasis::Percent(({} as f32) / 100.0)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
        };
        write!(code, ".basis({basis})").unwrap();
    }
    if let Some(align) = options.align_self {
        write!(
            code,
            ".align_self(::ui_lang_runtime::AlignItems::{})",
            flex_item_alignment_name(align)
        )
        .unwrap();
    }
    if flex_margin_present(&options.margin) {
        let top = flex_margin_side(
            options.margin.top.as_ref(),
            options.margin.y.as_ref(),
            options.margin.all.as_ref(),
        );
        let right = flex_margin_side(
            options.margin.right.as_ref(),
            options.margin.x.as_ref(),
            options.margin.all.as_ref(),
        );
        let bottom = flex_margin_side(
            options.margin.bottom.as_ref(),
            options.margin.y.as_ref(),
            options.margin.all.as_ref(),
        );
        let left = flex_margin_side(
            options.margin.left.as_ref(),
            options.margin.x.as_ref(),
            options.margin.all.as_ref(),
        );
        write!(
            code,
            ".margins(::ui_lang_runtime::FlexMargins {{ top: {}, right: {}, bottom: {}, left: {} }})",
            flex_margin_code(top, env, document)?,
            flex_margin_code(right, env, document)?,
            flex_margin_code(bottom, env, document)?,
            flex_margin_code(left, env, document)?,
        )
        .unwrap();
    }
    Ok(code)
}

fn flex_margin_present(margin: &FlexMarginOptions) -> bool {
    margin.all.is_some()
        || margin.x.is_some()
        || margin.y.is_some()
        || margin.top.is_some()
        || margin.right.is_some()
        || margin.bottom.is_some()
        || margin.left.is_some()
}

fn flex_margin_side<'a>(
    side: Option<&'a FlexMarginValue>,
    axis: Option<&'a FlexMarginValue>,
    all: Option<&'a FlexMarginValue>,
) -> Option<&'a FlexMarginValue> {
    side.or(axis).or(all)
}

fn flex_margin_code(
    margin: Option<&FlexMarginValue>,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    Ok(match margin {
        None => "::ui_lang_runtime::FlexMargin::Zero".to_owned(),
        Some(FlexMarginValue::Auto) => "::ui_lang_runtime::FlexMargin::Auto".to_owned(),
        Some(FlexMarginValue::Fixed(value)) => format!(
            "::ui_lang_runtime::FlexMargin::Fixed({} as f32)",
            expr_code(value, env, document, ValueMode::Owned)?
        ),
        Some(FlexMarginValue::Percent(value)) => format!(
            "::ui_lang_runtime::FlexMargin::Percent(({} as f32) / 100.0)",
            expr_code(value, env, document, ValueMode::Owned)?
        ),
    })
}

fn flex_direction_name(direction: FlexDirectionValue) -> &'static str {
    match direction {
        FlexDirectionValue::Row => "Row",
        FlexDirectionValue::RowReverse => "RowReverse",
        FlexDirectionValue::Column => "Column",
        FlexDirectionValue::ColumnReverse => "ColumnReverse",
    }
}

fn flex_item_alignment_name(align: FlexItemAlignment) -> &'static str {
    match align {
        FlexItemAlignment::Start => "Start",
        FlexItemAlignment::End => "End",
        FlexItemAlignment::FlexStart => "FlexStart",
        FlexItemAlignment::FlexEnd => "FlexEnd",
        FlexItemAlignment::Center => "Center",
        FlexItemAlignment::Baseline => "Baseline",
        FlexItemAlignment::Stretch => "Stretch",
    }
}

fn flex_content_alignment_name(align: FlexContentAlignment) -> &'static str {
    match align {
        FlexContentAlignment::Start => "Start",
        FlexContentAlignment::End => "End",
        FlexContentAlignment::FlexStart => "FlexStart",
        FlexContentAlignment::FlexEnd => "FlexEnd",
        FlexContentAlignment::Center => "Center",
        FlexContentAlignment::Stretch => "Stretch",
        FlexContentAlignment::SpaceBetween => "SpaceBetween",
        FlexContentAlignment::SpaceAround => "SpaceAround",
        FlexContentAlignment::SpaceEvenly => "SpaceEvenly",
    }
}

pub(in crate::codegen) fn scroll_bar_code(
    scroll: &ScrollOptions,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let constructor = if scroll.hidden_bar { "hidden" } else { "new" };
    let mut code = format!("::iced::widget::scrollable::Scrollbar::{constructor}()");
    for (value, method) in [
        (&scroll.bar_width, "width"),
        (&scroll.bar_margin, "margin"),
        (&scroll.scroller_width, "scroller_width"),
        (&scroll.bar_spacing, "spacing"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}(::ui_lang_runtime::bounded_table_metric({}, 2))",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(code)
}

pub(in crate::codegen) fn scroll_style_code(
    styles: &[ScrollStatusStyle],
    custom: Option<&ExternCall>,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let custom = custom
        .map(|style| {
            custom_style_call_code(
                style,
                ExternKind::ScrollStyle,
                "__theme, __status",
                env,
                document,
            )
        })
        .transpose()?;
    if styles.is_empty() {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base = custom
        .unwrap_or_else(|| "::iced::widget::scrollable::default(__theme, __status)".to_owned());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, pattern) in [
        (
            ScrollStatus::Active,
            "Active { is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ScrollStatus::Hovered,
            "Hovered { is_horizontal_scrollbar_hovered: __horizontal_interaction, is_vertical_scrollbar_hovered: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ScrollStatus::Dragged,
            "Dragged { is_horizontal_scrollbar_dragged: __horizontal_interaction, is_vertical_scrollbar_dragged: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
    ] {
        write!(code, " ::iced::widget::scrollable::Status::{pattern} => {{").unwrap();
        for style in styles
            .iter()
            .filter(|style| status != ScrollStatus::Active && style.status == ScrollStatus::Active)
            .chain(styles.iter().filter(|style| style.status == status))
        {
            write!(code, " if {} {{", scroll_selector_code(style)).unwrap();
            append_scroll_status_style(&mut code, style, env, document)?;
            code.push_str(" }");
        }
        code.push_str(" }");
    }
    code.push_str(" } __style })");
    Ok(code)
}

pub(in crate::codegen) fn scroll_selector_code(style: &ScrollStatusStyle) -> String {
    let mut conditions = Vec::new();
    for (value, binding) in [
        (style.horizontal_disabled, "__horizontal_disabled"),
        (style.vertical_disabled, "__vertical_disabled"),
        (style.horizontal_interaction, "__horizontal_interaction"),
        (style.vertical_interaction, "__vertical_interaction"),
    ] {
        if let Some(value) = value {
            conditions.push(format!("{binding} == {value}"));
        }
    }
    if conditions.is_empty() {
        "true".into()
    } else {
        conditions.join(" && ")
    }
}

pub(in crate::codegen) fn append_scroll_status_style(
    code: &mut String,
    style: &ScrollStatusStyle,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    append_scroll_surface_style(
        code,
        &style.container,
        "__style.container",
        true,
        true,
        env,
        document,
    )?;
    for (rail, target) in [
        (&style.horizontal_rail, "__style.horizontal_rail"),
        (&style.vertical_rail, "__style.vertical_rail"),
    ] {
        append_scroll_surface_style(code, &rail.rail, target, true, false, env, document)?;
        append_scroll_surface_style(
            code,
            &rail.scroller,
            &format!("{target}.scroller"),
            false,
            false,
            env,
            document,
        )?;
    }
    if let Some(gap) = &style.gap {
        write!(
            code,
            " __style.gap = ::std::option::Option::Some({});",
            background_code(gap, env, document)?
        )
        .unwrap();
    }
    append_scroll_surface_style(
        code,
        &style.auto_scroll,
        "__style.auto_scroll",
        false,
        false,
        env,
        document,
    )?;
    if let Some(color) = &style.auto_scroll_icon {
        write!(
            code,
            " __style.auto_scroll.icon = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn append_scroll_surface_style(
    code: &mut String,
    options: &ContainerStyleOptions,
    target: &str,
    optional_background: bool,
    text: bool,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    let mut options = options.clone();
    if !optional_background && let Some(background) = options.background.take() {
        write!(
            code,
            " {target}.background = {};",
            background_code(&background, env, document)?
        )
        .unwrap();
    }
    write!(code, " {{ let __style = &mut {target};").unwrap();
    append_surface_style_overrides(code, &options, env, document)?;
    if text && let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, color)
        )
        .unwrap();
    }
    code.push_str(" }");
    Ok(())
}
