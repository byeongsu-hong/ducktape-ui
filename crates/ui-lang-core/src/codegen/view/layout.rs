use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_layout(
    layout: &ResolvedLayout,
    identity: Option<&ResolvedViewIdentity>,
    children: &[ViewId],
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    match &layout.mode {
        ResolvedLayoutMode::Scroll(scroll) => render_resolved_scroll(
            layout,
            scroll,
            identity,
            children[0],
            document,
            message,
            env,
            scope,
            slot,
        ),
        ResolvedLayoutMode::Flex(flex) => render_resolved_flexbox(
            layout, flex, identity, children, document, message, env, scope, slot,
        ),
        ResolvedLayoutMode::Linear(_)
        | ResolvedLayoutMode::Grid(_)
        | ResolvedLayoutMode::Stack(_)
        | ResolvedLayoutMode::Hover(_) => render_resolved_regular_layout(
            layout, identity, children, document, message, env, scope, slot,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_resolved_regular_layout(
    layout: &ResolvedLayout,
    identity: Option<&ResolvedViewIdentity>,
    children: &[ViewId],
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document;
    let style = &layout.utility_style;
    let accessibility_key =
        resolved_accessibility_key_code(identity, "layout", layout.origin, scope, env, document)?;
    let mut body = String::from("{ let mut __children: ::std::vec::Vec<__IceElement<'_, ");
    write!(body, "{message}>> = ::std::vec::Vec::new();").unwrap();
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    render_children(
        &mut body,
        children,
        document,
        message,
        env,
        &child_scope,
        slot,
    )?;

    let needs_child_count = match &layout.mode {
        ResolvedLayoutMode::Linear(linear) => {
            !linear.wrap || linear.spacing.is_some() || linear.wrap_spacing.is_some()
        }
        ResolvedLayoutMode::Grid(grid) => grid.spacing.is_some(),
        ResolvedLayoutMode::Stack(_) | ResolvedLayoutMode::Hover(_) => false,
        ResolvedLayoutMode::Flex(_) | ResolvedLayoutMode::Scroll(_) => unreachable!(),
    };
    if needs_child_count {
        body.push_str(" let __child_count = __children.len();");
    }
    if let ResolvedLayoutMode::Grid(grid) = &layout.mode
        && let Some(columns) = grid.columns
    {
        write!(
            body,
            " let __grid_columns = usize::try_from({}).unwrap_or(0).max(1);",
            resolved_expr_use_code(program, columns, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let ResolvedLayoutMode::Linear(linear) = &layout.mode
        && !linear.wrap
    {
        let horizontal = linear.axis == ResolvedLinearAxis::Row;
        write!(
            body,
            " let __children = __children.into_iter().map(|__child| ::ui_lang_runtime::bounded_fill_element(__child, __child_count, {horizontal})).collect::<::std::vec::Vec<_>>();"
        )
        .unwrap();
    }

    match &layout.mode {
        ResolvedLayoutMode::Stack(stack) if stack.under > 0 => {
            write!(
                body,
                " let __under = ({} as usize).min(__children.len()); let __above = __children.split_off(__under); let __layout = __above.into_iter().fold(::iced::widget::Stack::new(), |__stack, __child| __stack.push(__child)); let __layout = __children.into_iter().rev().fold(__layout, |__stack, __child| __stack.push_under(__child))",
                stack.under
            )
            .unwrap();
        }
        ResolvedLayoutMode::Stack(_) => {
            body.push_str(" let __layout = ::ui_lang_runtime::zstack(__children)");
        }
        ResolvedLayoutMode::Hover(hover) => {
            // Exactly two children (the parser enforces it): base, reveal.
            body.push_str(" let __reveal = __children.pop().expect(\"hover reveal\"); let __base = __children.pop().expect(\"hover base\"); let __layout = ::ui_lang_runtime::hover_reveal(__base, __reveal)");
            if let Some(tint) = &hover.tint {
                write!(body, ".tint({})", resolved_theme_color(tint)).unwrap();
            }
            if hover.radius > 0.0 {
                write!(body, ".radius({} as f32)", hover.radius).unwrap();
            }
            if let Some(open) = hover.open {
                write!(
                    body,
                    ".open({})",
                    resolved_expr_use_code(program, open, env, ValueMode::Owned)?
                )
                .unwrap();
            }
        }
        ResolvedLayoutMode::Linear(linear) => {
            let constructor = if linear.axis == ResolvedLinearAxis::Column {
                "column"
            } else {
                "row"
            };
            if let Some(virtual_row) = linear.virtual_row {
                // Keep the ordinary column so padding, dimensions, and
                // max-width behave exactly as they do elsewhere; only per-child
                // layout moves inside, where the rows the viewport cannot see
                // are never laid out and so never shape their text. Spacing
                // goes in with them — the outer column has one child, so its
                // own spacing would have nothing to sit between.
                let estimate = resolved_expr_use_code(program, virtual_row, env, ValueMode::Owned)?;
                let spacing = match linear.spacing {
                    Some(spacing) => format!(
                        ".spacing(({}) as f32)",
                        resolved_expr_use_code(program, spacing, env, ValueMode::Owned)?
                    ),
                    None => String::new(),
                };
                write!(
                    body,
                    " let __layout = ::iced::widget::{constructor}(::std::vec![::iced::Element::from(::ui_lang_runtime::virtual_children(__children, ({estimate}) as f32){spacing})])"
                )
                .unwrap();
            } else {
                write!(
                    body,
                    " let __layout = ::iced::widget::{constructor}(__children)"
                )
                .unwrap();
            }
        }
        ResolvedLayoutMode::Grid(_) => {
            body.push_str(" let __layout = ::iced::widget::grid(__children)");
        }
        ResolvedLayoutMode::Flex(_) | ResolvedLayoutMode::Scroll(_) => unreachable!(),
    }

    if let Some(gap) = style.gap {
        write!(body, ".spacing({gap})").unwrap();
    }
    if let ResolvedLayoutMode::Linear(linear) = &layout.mode {
        if let Some(padding) = style.padding_code() {
            write!(body, ".padding({padding})").unwrap();
        }
        if style.items_center {
            let method = if linear.axis == ResolvedLinearAxis::Column {
                "align_x"
            } else {
                "align_y"
            };
            write!(body, ".{method}(::iced::Center)").unwrap();
        }
        // A virtualized column has exactly one child — the spacing went in
        // with the rows, and repeating it here would only read as if it
        // applied twice.
        if let Some(spacing) = linear.spacing.filter(|_| linear.virtual_row.is_none()) {
            write!(
                body,
                ".spacing(::ui_lang_runtime::bounded_spacing({}, __child_count))",
                resolved_expr_use_code(program, spacing, env, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(padding) = resolved_layout_padding_code(&linear.padding, program, env)? {
            write!(body, ".padding({padding})").unwrap();
        }
        append_resolved_layout_dimensions(
            &mut body,
            [&linear.width, &linear.height],
            program,
            env,
        )?;
        if let Some(max_width) = linear.max_width {
            write!(
                body,
                ".max_width({} as f32)",
                resolved_expr_use_code(program, max_width, env, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(align) = linear.align {
            let (method, alignment) = match (linear.axis, align) {
                (ResolvedLinearAxis::Column, ResolvedContainerAlignment::Start) => {
                    ("align_x", "::iced::alignment::Horizontal::Left")
                }
                (ResolvedLinearAxis::Column, ResolvedContainerAlignment::Center) => {
                    ("align_x", "::iced::alignment::Horizontal::Center")
                }
                (ResolvedLinearAxis::Column, ResolvedContainerAlignment::End) => {
                    ("align_x", "::iced::alignment::Horizontal::Right")
                }
                (ResolvedLinearAxis::Row, ResolvedContainerAlignment::Start) => {
                    ("align_y", "::iced::alignment::Vertical::Top")
                }
                (ResolvedLinearAxis::Row, ResolvedContainerAlignment::Center) => {
                    ("align_y", "::iced::alignment::Vertical::Center")
                }
                (ResolvedLinearAxis::Row, ResolvedContainerAlignment::End) => {
                    ("align_y", "::iced::alignment::Vertical::Bottom")
                }
            };
            write!(body, ".{method}({alignment})").unwrap();
        }
        if let Some(clip) = linear.clip {
            write!(
                body,
                ".clip({})",
                resolved_expr_use_code(program, clip, env, ValueMode::Owned)?
            )
            .unwrap();
        } else if style.clip {
            body.push_str(".clip(true)");
        }
        if linear.wrap {
            body.push_str(".wrap()");
            if let Some(spacing) = linear.wrap_spacing {
                let method = if linear.axis == ResolvedLinearAxis::Column {
                    "horizontal_spacing"
                } else {
                    "vertical_spacing"
                };
                write!(
                    body,
                    ".{method}(::ui_lang_runtime::bounded_spacing({}, __child_count))",
                    resolved_expr_use_code(program, spacing, env, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(align) = linear.wrap_align {
                let alignment = match (linear.axis, align) {
                    (ResolvedLinearAxis::Column, ResolvedContainerAlignment::Start) => {
                        "::iced::alignment::Vertical::Top"
                    }
                    (ResolvedLinearAxis::Column, ResolvedContainerAlignment::Center) => {
                        "::iced::alignment::Vertical::Center"
                    }
                    (ResolvedLinearAxis::Column, ResolvedContainerAlignment::End) => {
                        "::iced::alignment::Vertical::Bottom"
                    }
                    (ResolvedLinearAxis::Row, ResolvedContainerAlignment::Start) => {
                        "::iced::alignment::Horizontal::Left"
                    }
                    (ResolvedLinearAxis::Row, ResolvedContainerAlignment::Center) => {
                        "::iced::alignment::Horizontal::Center"
                    }
                    (ResolvedLinearAxis::Row, ResolvedContainerAlignment::End) => {
                        "::iced::alignment::Horizontal::Right"
                    }
                };
                write!(body, ".align_x({alignment})").unwrap();
            }
        }
    }

    if let ResolvedLayoutMode::Grid(grid) = &layout.mode {
        if let Some(spacing) = grid.spacing {
            let entries = if grid.columns.is_some() {
                "__child_count.max(__grid_columns)"
            } else {
                "__child_count"
            };
            write!(
                body,
                ".spacing(::ui_lang_runtime::bounded_spacing({}, {entries}))",
                resolved_expr_use_code(program, spacing, env, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(width) = grid.width {
            write!(
                body,
                ".width({})",
                clamped_f32_code(width, "0.0", "f32::MAX", program, env)?
            )
            .unwrap();
        }
        if let Some(height) = &grid.height {
            match height {
                ResolvedGridHeight::AspectRatio { width, height } => {
                    let width = resolved_expr_use_code(program, *width, env, ValueMode::Owned)?;
                    let height = resolved_expr_use_code(program, *height, env, ValueMode::Owned)?;
                    let ratio = clamped_f32(
                        &format!("({width}) / ({height})"),
                        "f32::EPSILON",
                        "f32::MAX",
                    );
                    write!(
                        body,
                        ".height(::iced::widget::grid::Sizing::AspectRatio({ratio}))"
                    )
                    .unwrap();
                }
                ResolvedGridHeight::EvenlyDistribute(length) => {
                    write!(
                        body,
                        ".height({})",
                        resolved_length_code(length, program, env)?
                    )
                    .unwrap();
                }
            }
        }
        if let Some(max_cell) = grid.max_cell {
            write!(
                body,
                ".fluid({})",
                clamped_f32_code(max_cell, "f32::EPSILON", "f32::MAX", program, env,)?
            )
            .unwrap();
        } else if grid.columns.is_some() {
            body.push_str(".columns(__grid_columns)");
        }
    }

    if let ResolvedLayoutMode::Stack(stack) = &layout.mode {
        if let Some(clip) = stack.clip {
            write!(
                body,
                ".clip({})",
                resolved_expr_use_code(program, clip, env, ValueMode::Owned)?
            )
            .unwrap();
        } else if style.clip {
            body.push_str(".clip(true)");
        }
        append_resolved_layout_dimensions(&mut body, [&stack.width, &stack.height], program, env)?;
        append_size(&mut body, style);
    }

    body.push(';');
    body.push_str(" let __content = ::iced::widget::container(__layout)");
    if matches!(layout.mode, ResolvedLayoutMode::Grid(_)) && style.clip {
        body.push_str(".clip(true)");
    }
    if matches!(
        layout.mode,
        ResolvedLayoutMode::Grid(_) | ResolvedLayoutMode::Stack(_)
    ) && let Some(padding) = style.padding_code()
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
        write!(
            body,
            " let __layout_content: __IceElement<'_, {message}> = ::iced::widget::container(__content).width(::iced::Fill).center_x(::iced::Fill).into();"
        )
        .unwrap();
    } else {
        write!(
            body,
            " let __layout_content: __IceElement<'_, {message}> = __content.into();"
        )
        .unwrap();
    }
    write!(
        body,
        " let __a11y_key = {accessibility_key}; ::ui_lang_runtime::accessible(__layout_content, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}"
    )
    .unwrap();
    Ok(body)
}

#[allow(clippy::too_many_arguments)]
fn render_resolved_flexbox(
    layout: &ResolvedLayout,
    flex: &ResolvedFlexLayout,
    identity: Option<&ResolvedViewIdentity>,
    children: &[ViewId],
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document;
    let style = &layout.utility_style;
    let accessibility_key =
        resolved_accessibility_key_code(identity, "layout", layout.origin, scope, env, document)?;
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    let mut body = String::from("{ let mut __items = ::std::vec::Vec::new();");
    render_flex_children(
        &mut body,
        children,
        document,
        message,
        env,
        &child_scope,
        slot,
        flex.min_cell,
    )?;
    write!(
        body,
        " let __layout = ::ui_lang_runtime::flex(__items).direction(::ui_lang_runtime::FlexDirection::{})",
        resolved_flex_direction_name(flex.direction)
    )
    .unwrap();
    if flex.wrap != ResolvedFlexWrap::NoWrap {
        write!(
            body,
            ".wrap(::ui_lang_runtime::FlexWrap::{})",
            match flex.wrap {
                ResolvedFlexWrap::NoWrap => unreachable!(),
                ResolvedFlexWrap::Wrap => "Wrap",
                ResolvedFlexWrap::WrapReverse => "WrapReverse",
            }
        )
        .unwrap();
    }
    if let Some(justify) = flex.justify_content {
        write!(
            body,
            ".justify_content(::ui_lang_runtime::JustifyContent::{})",
            resolved_flex_content_alignment_name(justify)
        )
        .unwrap();
    }
    if let Some(align) = flex.align_items {
        write!(
            body,
            ".align_items(::ui_lang_runtime::AlignItems::{})",
            resolved_flex_item_alignment_name(align)
        )
        .unwrap();
    } else if style.items_center {
        body.push_str(".align_items(::ui_lang_runtime::AlignItems::Center)");
    }
    if let Some(align) = flex.align_content {
        write!(
            body,
            ".align_content(::ui_lang_runtime::AlignContent::{})",
            resolved_flex_content_alignment_name(align)
        )
        .unwrap();
    }
    if let Some(gap) = style.gap {
        write!(body, ".gap({gap}.0)").unwrap();
    }
    if let Some(gap) = flex.spacing {
        write!(
            body,
            ".gap({} as f32)",
            resolved_expr_use_code(program, gap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(gap) = flex.wrap_spacing {
        let method = match flex.direction {
            ResolvedFlexDirection::Row | ResolvedFlexDirection::RowReverse => "row_gap",
            ResolvedFlexDirection::Column | ResolvedFlexDirection::ColumnReverse => "column_gap",
        };
        write!(
            body,
            ".{method}({} as f32)",
            resolved_expr_use_code(program, gap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    for (gap, method) in [(flex.row_gap, "row_gap"), (flex.column_gap, "column_gap")] {
        if let Some(gap) = gap {
            write!(
                body,
                ".{method}({} as f32)",
                resolved_expr_use_code(program, gap, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(padding) = style.padding_code() {
        write!(body, ".padding({padding})").unwrap();
    }
    if let Some(padding) = resolved_layout_padding_code(&flex.padding, program, env)? {
        write!(body, ".padding({padding})").unwrap();
    }
    append_size(&mut body, style);
    append_resolved_layout_dimensions(&mut body, [&flex.width, &flex.height], program, env)?;
    if let Some(max_width) = flex.max_width {
        write!(
            body,
            ".max_width({} as f32)",
            resolved_expr_use_code(program, max_width, env, ValueMode::Owned)?
        )
        .unwrap();
    } else if let Some(max_width) = style.max_width {
        write!(body, ".max_width({max_width}.0)").unwrap();
    }
    if let Some(max_height) = flex.max_height {
        write!(
            body,
            ".max_height({} as f32)",
            resolved_expr_use_code(program, max_height, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(clip) = flex.clip {
        write!(
            body,
            ".clip({})",
            resolved_expr_use_code(program, clip, env, ValueMode::Owned)?
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
    write!(
        body,
        "; let __layout_content: __IceElement<'_, {message}> = __content.into(); let __a11y_key = {accessibility_key}; ::ui_lang_runtime::accessible(__layout_content, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}"
    )
    .unwrap();
    Ok(body)
}

#[allow(clippy::too_many_arguments)]
fn render_resolved_scroll(
    layout: &ResolvedLayout,
    scroll: &ResolvedScrollLayout,
    identity: Option<&ResolvedViewIdentity>,
    child: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document;
    let accessibility_key =
        resolved_accessibility_key_code(identity, "layout", layout.origin, scope, env, document)?;
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    let has_virtual_rows = contains_virtual_rows(child, document, slot)?;
    let child = render_node(child, document, message, env, &child_scope, slot)?;
    let mut code = String::from("::iced::widget::scrollable(__scroll_content)");
    let bar = resolved_scroll_bar_code(scroll, program, env)?;
    let direction = match scroll.direction {
        ResolvedScrollDirection::Vertical => {
            format!("::iced::widget::scrollable::Direction::Vertical({bar})")
        }
        ResolvedScrollDirection::Horizontal => {
            format!("::iced::widget::scrollable::Direction::Horizontal({bar})")
        }
        ResolvedScrollDirection::Both => format!(
            "::iced::widget::scrollable::Direction::Both {{ vertical: {bar}, horizontal: {bar} }}"
        ),
    };
    write!(code, ".direction({direction})").unwrap();
    if let Some(identity) = identity {
        write!(
            code,
            ".id(::iced::widget::Id::from({}))",
            resolved_view_identity_code(identity, scope, env, document)?
        )
        .unwrap();
    }
    // `keep` is iced's `Start` anchor — the resting place is the same — with
    // the correction supplied by the wrapper applied below.
    let anchor = |anchor| match anchor {
        ResolvedScrollAnchor::Start | ResolvedScrollAnchor::Keep => "Start",
        ResolvedScrollAnchor::End => "End",
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
    if let Some(auto_scroll) = scroll.auto_scroll {
        write!(
            code,
            ".auto_scroll({})",
            resolved_expr_use_code(program, auto_scroll, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(route) = &scroll.route {
        let callback = resolved_interaction_route_callback_with_code(
            route,
            "__viewport",
            env,
            program,
            |callback_env| {
                let message_code = resolved_interaction_route_code(
                    route,
                    &[
                        "__absolute.x as f64",
                        "__absolute.y as f64",
                        "__relative.x as f64",
                        "__relative.y as f64",
                    ],
                    callback_env,
                    program,
                    message,
                )?;
                Ok(format!(
                    "{{ let __absolute = __viewport.absolute_offset(); let __relative = __viewport.relative_offset(); {message_code} }}"
                ))
            },
        )?;
        write!(code, ".on_scroll({callback})").unwrap();
    } else if let Some(route) = &scroll.viewport_route {
        let callback = resolved_interaction_route_callback_with_code(
            route,
            "__viewport",
            env,
            program,
            |callback_env| {
                let message_code = resolved_interaction_route_code(
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
                    program,
                    message,
                )?;
                Ok(format!(
                    "{{ let __absolute = __viewport.absolute_offset(); let __reversed = __viewport.absolute_offset_reversed(); let __relative = __viewport.relative_offset(); let __bounds = __viewport.bounds(); let __content_bounds = __viewport.content_bounds(); {message_code} }}"
                ))
            },
        )?;
        write!(code, ".on_scroll({callback})").unwrap();
    }
    code.push_str(&resolved_scroll_style_code(scroll, program, env)?);
    append_size(&mut code, &layout.utility_style);
    append_resolved_layout_dimensions(&mut code, [&scroll.width, &scroll.height], program, env)?;
    if has_virtual_rows {
        code = format!("::ui_lang_runtime::virtual_scroll({code})");
    }
    // `anchor-y=keep` wraps the scrollable — and only the scrollable, so the
    // wrapper's operation walk reaches it first and a nested list keeps its
    // own offset. The accessibility wrapper stays outside, where the id is.
    if scroll.anchor_y == ResolvedScrollAnchor::Keep {
        code = format!("::ui_lang_runtime::scroll_anchor({code})");
    }
    Ok(format!(
        "{{ let __a11y_key = {accessibility_key}; let __scroll_content: __IceElement<'_, {message}> = {child}; let __layout = {code}; ::ui_lang_runtime::accessible(__layout, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}"
    ))
}

/// Whether this scroll's own content contains a virtual row. Nested scrolls
/// stop the walk because they own their wheel transaction and get their own
/// wrapper when rendered.
///
/// `slots` is the content the caller passed for the component body this scroll
/// sits in, exactly as `render_node` receives it: a slot is written at the call
/// site and rendered inline here, so a component whose body is `scroll { slot }`
/// only learns about virtual rows by following it.
fn contains_virtual_rows(
    node: ViewId,
    document: &LoweredProgram,
    slots: Option<&SlotContext>,
) -> Result<bool, Error> {
    fn any(
        children: impl IntoIterator<Item = ViewId>,
        document: &LoweredProgram,
        slots: Option<&SlotContext>,
    ) -> Result<bool, Error> {
        for child in children {
            if contains_virtual_rows(child, document, slots)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    let view = document.resolved_view(node)?;
    match &view.kind {
        ResolvedViewKind::Layout { children } => match &document.resolved_layout(node)?.mode {
            ResolvedLayoutMode::Scroll(_) => Ok(false),
            ResolvedLayoutMode::Linear(linear) if linear.virtual_row.is_some() => Ok(true),
            _ => any(children.iter().copied(), document, slots),
        },
        ResolvedViewKind::KeyedColumn { child } => {
            if document.resolved_keyed_column(node)?.virtual_row.is_some() {
                Ok(true)
            } else {
                contains_virtual_rows(*child, document, slots)
            }
        }
        ResolvedViewKind::Container { content }
        | ResolvedViewKind::MouseArea { content }
        | ResolvedViewKind::ResizeHandle { content }
        | ResolvedViewKind::Theme { content }
        | ResolvedViewKind::Float { content }
        | ResolvedViewKind::Pin { content }
        | ResolvedViewKind::Sensor { content }
        | ResolvedViewKind::ResponsiveSize { content }
        | ResolvedViewKind::Lazy { child: content } => {
            contains_virtual_rows(*content, document, slots)
        }
        ResolvedViewKind::Button {
            content: Some(content),
        } => contains_virtual_rows(*content, document, slots),
        ResolvedViewKind::Overlay { content, layer }
        | ResolvedViewKind::Tooltip {
            content,
            tip: layer,
        } => any([*content, *layer], document, slots),
        ResolvedViewKind::If { children } | ResolvedViewKind::For { children } => {
            any(children.iter().copied(), document, slots)
        }
        ResolvedViewKind::Match { arms } => any(arms.iter().flatten().copied(), document, slots),
        ResolvedViewKind::Table { columns } => any(
            columns
                .iter()
                .flat_map(|column| [column.header, column.cell]),
            document,
            slots,
        ),
        ResolvedViewKind::PaneGrid { panes, templates } => {
            let mut children = Vec::new();
            for pane in panes.iter().chain(templates) {
                children.push(pane.content);
                if let Some(title) = &pane.title {
                    children.push(title.content);
                    children.extend(title.controls);
                    children.extend(title.compact_controls);
                }
            }
            any(children, document, slots)
        }
        ResolvedViewKind::Component { call } => {
            let call = document.component_call_by_id(*call)?;
            let component = document.component(call.component);
            // The body's own slots bind at THIS call, not in the scope the
            // walk is standing in, so they are covered by the call's contents
            // below rather than followed from inside the body.
            if contains_virtual_rows(component.root, document, None)? {
                return Ok(true);
            }
            any(
                call.slots.iter().filter_map(|slot| slot.content),
                document,
                slots,
            )
        }
        // Content the caller wrote, rendered here. Its own slots resolve
        // against the caller's context, which is the parent this hands down —
        // the same handoff `render_node` makes for the rendered content.
        ResolvedViewKind::Slot { slot, .. } => {
            let Some(content) =
                slots.and_then(|slots| slots.entries.iter().find(|entry| entry.slot == *slot))
            else {
                return Ok(false);
            };
            contains_virtual_rows(
                content.view,
                document,
                slots.and_then(|slots| slots.parent.as_deref()),
            )
        }
        ResolvedViewKind::Text
        | ResolvedViewKind::RichText
        | ResolvedViewKind::Input
        | ResolvedViewKind::Button { content: None }
        | ResolvedViewKind::Checkbox
        | ResolvedViewKind::Toggler
        | ResolvedViewKind::Slider
        | ResolvedViewKind::Progress
        | ResolvedViewKind::Radio
        | ResolvedViewKind::PickList
        | ResolvedViewKind::ComboBox
        | ResolvedViewKind::Rule
        | ResolvedViewKind::QrCode
        | ResolvedViewKind::Space
        | ResolvedViewKind::Markdown
        | ResolvedViewKind::TextEditor
        | ResolvedViewKind::ExternComponent
        | ResolvedViewKind::Themer
        | ResolvedViewKind::Shader
        | ResolvedViewKind::Media
        | ResolvedViewKind::Canvas => Ok(false),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_flex_children(
    out: &mut String,
    children: &[ViewId],
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
    min_cell: Option<ResolvedExpressionId>,
) -> Result<(), Error> {
    for child in children {
        let view = document.resolved_view(*child)?;
        match &view.kind {
            ResolvedViewKind::If { children } => {
                let program = document;
                let conditional = program.resolved_conditional(*child)?;
                let condition =
                    resolved_expr_use_code(program, conditional.condition, env, ValueMode::Owned)?;
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
            ResolvedViewKind::For { children } => {
                let program = document;
                let iteration = program.resolved_iteration(*child)?;
                let item_name = &iteration.item.name;
                let items = resolved_expr_use_code(
                    program,
                    iteration.items,
                    env,
                    ValueMode::TransientBorrowed,
                )?;
                // Copy rows are free to copy; anything else iterates by
                // reference, exactly as a `for` outside a flex layout does.
                let element_ty = &program.expressions().local(iteration.item.local).ty;
                let iterate = if copy_expression_type(element_ty) {
                    ".iter().cloned().enumerate()"
                } else {
                    ".iter().enumerate()"
                };
                let reconciliation_scope = borrowed_scope(reconciliation_scope(scope, env));
                write!(
                    out,
                    " for (__ice_index, {item_name}) in {items}{iterate} {{ let __for_scope = format!(\"{{}}/@for:{}({{}})\", {reconciliation_scope}, __ice_index);",
                    iteration.reconciliation_line
                )
                .unwrap();
                let mut child_env = ScopedBindingEnv::new(env);
                child_env.insert(
                    item_name.clone(),
                    resolved_local_binding(
                        LocalBindingTypeSource::Resolved(program),
                        iteration.item.local,
                        item_name.clone(),
                        false,
                    ),
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
            ResolvedViewKind::Match { arms } => {
                let program = document;
                let resolved = program.resolved_match(*child)?;
                if arms.len() != resolved.arms.len() {
                    return Err(program
                        .invariant_at_origin(view.origin, "flex match HIR arm length diverged"));
                }
                let value =
                    resolved_expr_use_code(program, resolved.value, env, ValueMode::Borrowed)?;
                write!(out, " match &({value}) {{").unwrap();
                for (arm_children, resolved_arm) in arms.iter().zip(&resolved.arms) {
                    write!(
                        out,
                        " {} => {{",
                        resolved_match_pattern_code(resolved_arm, program)?
                    )
                    .unwrap();
                    let mut child_env = ScopedBindingEnv::new(env);
                    if let Some(payload) = &resolved_arm.binding {
                        let name = payload.name.clone();
                        child_env.insert(
                            name.clone(),
                            resolved_local_binding(
                                LocalBindingTypeSource::Hir(payload),
                                payload.local,
                                name,
                                false,
                            ),
                        );
                    }
                    render_flex_children(
                        out,
                        arm_children,
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
                    render_node_if_present(*child, document, message, env, scope, slot)?
                else {
                    continue;
                };
                let item = if let Some(min_cell) = min_cell {
                    format!(
                        "::ui_lang_runtime::flex_item(__flex_child).grow(1.0).shrink(0.0).basis(::ui_lang_runtime::FlexBasis::Fixed({}))",
                        clamped_f32_code(min_cell, "f32::EPSILON", "f32::MAX", document, env,)?
                    )
                } else {
                    let options = match view.kind {
                        ResolvedViewKind::Container { .. } => {
                            Some(&document.resolved_container(*child)?.flex_item)
                        }
                        _ => None,
                    };
                    resolved_flex_item_code("__flex_child", options, document, env)?
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

fn resolved_flex_item_code(
    child: &str,
    options: Option<&ResolvedContainerFlexItem>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let mut code = format!("::ui_lang_runtime::flex_item({child})");
    let Some(options) = options else {
        return Ok(code);
    };
    if let Some(order) = options.order {
        write!(
            code,
            ".order({} as i64)",
            resolved_expr_use_code(program, order, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    for (value, method) in [(options.grow, "grow"), (options.shrink, "shrink")] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                resolved_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(basis) = &options.basis {
        let basis = match basis {
            ResolvedContainerFlexBasis::Auto => "::ui_lang_runtime::FlexBasis::Auto".to_owned(),
            ResolvedContainerFlexBasis::Content => {
                "::ui_lang_runtime::FlexBasis::Content".to_owned()
            }
            ResolvedContainerFlexBasis::Fixed(value) => format!(
                "::ui_lang_runtime::FlexBasis::Fixed({} as f32)",
                resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
            ),
            ResolvedContainerFlexBasis::Percent(value) => format!(
                "::ui_lang_runtime::FlexBasis::Percent(({} as f32) / 100.0)",
                resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
            ),
        };
        write!(code, ".basis({basis})").unwrap();
    }
    if let Some(align) = options.align_self {
        write!(
            code,
            ".align_self(::ui_lang_runtime::AlignItems::{})",
            resolved_flex_item_alignment_name(align)
        )
        .unwrap();
    }
    if let Some(margins) = &options.margins {
        write!(
            code,
            ".margins(::ui_lang_runtime::FlexMargins {{ top: {}, right: {}, bottom: {}, left: {} }})",
            resolved_flex_margin_code(&margins.top, program, env)?,
            resolved_flex_margin_code(&margins.right, program, env)?,
            resolved_flex_margin_code(&margins.bottom, program, env)?,
            resolved_flex_margin_code(&margins.left, program, env)?,
        )
        .unwrap();
    }
    Ok(code)
}

fn resolved_flex_margin_code(
    margin: &ResolvedContainerFlexMargin,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match margin {
        ResolvedContainerFlexMargin::Zero => "::ui_lang_runtime::FlexMargin::Zero".to_owned(),
        ResolvedContainerFlexMargin::Auto => "::ui_lang_runtime::FlexMargin::Auto".to_owned(),
        ResolvedContainerFlexMargin::Fixed(value) => format!(
            "::ui_lang_runtime::FlexMargin::Fixed({} as f32)",
            resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
        ),
        ResolvedContainerFlexMargin::Percent(value) => format!(
            "::ui_lang_runtime::FlexMargin::Percent(({} as f32) / 100.0)",
            resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
        ),
    })
}

fn resolved_flex_item_alignment_name(align: ResolvedContainerFlexAlignment) -> &'static str {
    match align {
        ResolvedContainerFlexAlignment::Start => "Start",
        ResolvedContainerFlexAlignment::End => "End",
        ResolvedContainerFlexAlignment::FlexStart => "FlexStart",
        ResolvedContainerFlexAlignment::FlexEnd => "FlexEnd",
        ResolvedContainerFlexAlignment::Center => "Center",
        ResolvedContainerFlexAlignment::Baseline => "Baseline",
        ResolvedContainerFlexAlignment::Stretch => "Stretch",
    }
}

fn resolved_flex_direction_name(direction: ResolvedFlexDirection) -> &'static str {
    match direction {
        ResolvedFlexDirection::Row => "Row",
        ResolvedFlexDirection::RowReverse => "RowReverse",
        ResolvedFlexDirection::Column => "Column",
        ResolvedFlexDirection::ColumnReverse => "ColumnReverse",
    }
}

fn resolved_flex_content_alignment_name(align: ResolvedFlexContentAlignment) -> &'static str {
    match align {
        ResolvedFlexContentAlignment::Start => "Start",
        ResolvedFlexContentAlignment::End => "End",
        ResolvedFlexContentAlignment::FlexStart => "FlexStart",
        ResolvedFlexContentAlignment::FlexEnd => "FlexEnd",
        ResolvedFlexContentAlignment::Center => "Center",
        ResolvedFlexContentAlignment::Stretch => "Stretch",
        ResolvedFlexContentAlignment::SpaceBetween => "SpaceBetween",
        ResolvedFlexContentAlignment::SpaceAround => "SpaceAround",
        ResolvedFlexContentAlignment::SpaceEvenly => "SpaceEvenly",
    }
}

fn resolved_layout_padding_code(
    padding: &ResolvedContainerPadding,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let value = |expression: Option<ResolvedExpressionId>| {
        expression
            .map(|expression| resolved_expr_use_code(program, expression, env, ValueMode::Owned))
            .transpose()
    };
    let all = value(padding.all)?.unwrap_or_else(|| "0.0".into());
    let x = value(padding.x)?.unwrap_or_else(|| all.clone());
    let y = value(padding.y)?.unwrap_or_else(|| all.clone());
    let top = value(padding.top)?.unwrap_or_else(|| y.clone());
    let right = value(padding.right)?.unwrap_or_else(|| x.clone());
    let bottom = value(padding.bottom)?.unwrap_or(y);
    let left = value(padding.left)?.unwrap_or(x);
    Ok(Some(format!(
        "::ui_lang_runtime::bounded_padding({top}, {right}, {bottom}, {left})"
    )))
}

fn append_resolved_layout_dimensions(
    code: &mut String,
    dimensions: [&Option<ResolvedContainerLength>; 2],
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    for (method, length) in ["width", "height"].into_iter().zip(dimensions) {
        let Some(length) = length else { continue };
        write!(
            code,
            ".{method}({})",
            resolved_length_code(length, program, env)?
        )
        .unwrap();
    }
    Ok(())
}

pub(super) fn resolved_length_code(
    length: &ResolvedContainerLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedContainerLength::Fill => "::iced::Fill".into(),
        ResolvedContainerLength::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        ResolvedContainerLength::Shrink => "::iced::Shrink".into(),
        ResolvedContainerLength::FixedF64(expression) => format!(
            "{} as f32",
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedContainerLength::FixedLength(expression) => {
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        }
    })
}

fn resolved_scroll_bar_code(
    scroll: &ResolvedScrollLayout,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let constructor = if scroll.hidden_bar { "hidden" } else { "new" };
    let mut code = format!("::iced::widget::scrollable::Scrollbar::{constructor}()");
    for (value, method) in [
        (scroll.bar_width, "width"),
        (scroll.bar_margin, "margin"),
        (scroll.scroller_width, "scroller_width"),
        (scroll.bar_spacing, "spacing"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}(::ui_lang_runtime::bounded_table_metric({}, 2))",
                resolved_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(code)
}

fn resolved_scroll_style_code(
    scroll: &ResolvedScrollLayout,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let custom = scroll
        .custom_style
        .as_ref()
        .map(|style| resolved_scroll_custom_style_code(style, program, env))
        .transpose()?;
    if scroll.styles.is_empty() {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::scrollable::default(__theme, __status)".into());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, pattern) in [
        (
            ResolvedScrollStatus::Active,
            "Active { is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ResolvedScrollStatus::Hovered,
            "Hovered { is_horizontal_scrollbar_hovered: __horizontal_interaction, is_vertical_scrollbar_hovered: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ResolvedScrollStatus::Dragged,
            "Dragged { is_horizontal_scrollbar_dragged: __horizontal_interaction, is_vertical_scrollbar_dragged: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
    ] {
        write!(code, " ::iced::widget::scrollable::Status::{pattern} => {{").unwrap();
        for style in scroll
            .styles
            .iter()
            .filter(|style| {
                status != ResolvedScrollStatus::Active
                    && style.status == ResolvedScrollStatus::Active
            })
            .chain(scroll.styles.iter().filter(|style| style.status == status))
        {
            write!(
                code,
                " if {} {{",
                resolved_scroll_selector_code(&style.selector)
            )
            .unwrap();
            append_resolved_scroll_status_style(&mut code, style, program, env)?;
            code.push_str(" }");
        }
        code.push_str(" }");
    }
    code.push_str(" } __style })");
    Ok(code)
}

fn resolved_scroll_custom_style_code(
    style: &ResolvedScrollCustomStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let arguments = style
        .arguments
        .iter()
        .map(|argument| resolved_expr_use_code(program, *argument, env, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?;
    let suffix = arguments
        .into_iter()
        .map(|argument| format!(", {argument}"))
        .collect::<String>();
    Ok(format!(
        "{}(__theme, __status{suffix})",
        program.extern_function(style.function).rust_path
    ))
}

fn resolved_scroll_selector_code(selector: &ResolvedScrollSelector) -> String {
    let mut conditions = Vec::new();
    for (value, binding) in [
        (selector.horizontal_disabled, "__horizontal_disabled"),
        (selector.vertical_disabled, "__vertical_disabled"),
        (selector.horizontal_interaction, "__horizontal_interaction"),
        (selector.vertical_interaction, "__vertical_interaction"),
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

fn append_resolved_scroll_status_style(
    code: &mut String,
    style: &ResolvedScrollStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    append_resolved_scroll_surface(
        code,
        &style.container,
        "__style.container",
        true,
        true,
        program,
        env,
    )?;
    for (rail, scroller, target) in [
        (
            &style.horizontal_rail,
            &style.horizontal_scroller,
            "__style.horizontal_rail",
        ),
        (
            &style.vertical_rail,
            &style.vertical_scroller,
            "__style.vertical_rail",
        ),
    ] {
        append_resolved_scroll_surface(code, rail, target, true, false, program, env)?;
        append_resolved_scroll_surface(
            code,
            scroller,
            &format!("{target}.scroller"),
            false,
            false,
            program,
            env,
        )?;
    }
    if let Some(gap) = &style.gap {
        write!(
            code,
            " __style.gap = ::std::option::Option::Some({});",
            resolved_layout_background_code(gap, program, env)?
        )
        .unwrap();
    }
    append_resolved_scroll_surface(
        code,
        &style.auto_scroll,
        "__style.auto_scroll",
        false,
        false,
        program,
        env,
    )?;
    if let Some(color) = &style.auto_scroll_icon {
        write!(
            code,
            " __style.auto_scroll.icon = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_resolved_scroll_surface(
    code: &mut String,
    surface: &ResolvedContainerSurface,
    target: &str,
    optional_background: bool,
    text: bool,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if !optional_background && let Some(background) = &surface.background {
        write!(
            code,
            " {target}.background = {};",
            resolved_layout_background_code(background, program, env)?
        )
        .unwrap();
    }
    write!(code, " {{ let __style = &mut {target};").unwrap();
    if optional_background && let Some(background) = &surface.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({});",
            resolved_layout_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(color) = &surface.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(width) = surface.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            resolved_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = resolved_layout_radius_code(&surface.radius, program, env)? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(color) = &surface.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    for (expression, field) in [
        (surface.shadow_x, "offset.x"),
        (surface.shadow_y, "offset.y"),
        (surface.shadow_blur, "blur_radius"),
    ] {
        if let Some(expression) = expression {
            write!(
                code,
                " __style.shadow.{field} = {} as f32;",
                resolved_expr_use_code(program, expression, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(snap) = surface.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            resolved_expr_use_code(program, snap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if text && let Some(color) = &surface.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    code.push_str(" }");
    Ok(())
}

fn resolved_layout_background_code(
    background: &ResolvedContainerBackground,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match background {
        ResolvedContainerBackground::Color(color) => {
            format!("::iced::Background::Color({})", resolved_theme_color(color))
        }
        ResolvedContainerBackground::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                resolved_expr_use_code(program, *angle, env, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    resolved_expr_use_code(program, stop.offset, env, ValueMode::Owned)?,
                    resolved_theme_color(&stop.color)
                )
                .unwrap();
            }
            code.push(')');
            code
        }
    })
}

fn resolved_layout_radius_code(
    radius: &ResolvedContainerRadius,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
    {
        return Ok(None);
    }
    let base = radius
        .all
        .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
        .transpose()?
        .unwrap_or_else(|| "0.0".into());
    let corner = |value: Option<ResolvedExpressionId>| {
        value
            .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
            .transpose()
    };
    let top_left = corner(radius.top_left)?.unwrap_or_else(|| base.clone());
    let top_right = corner(radius.top_right)?.unwrap_or_else(|| base.clone());
    let bottom_right = corner(radius.bottom_right)?.unwrap_or_else(|| base.clone());
    let bottom_left = corner(radius.bottom_left)?.unwrap_or(base);
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {top_left}, top_right: {top_right}, bottom_right: {bottom_right}, bottom_left: {bottom_left} }}"
    )))
}
