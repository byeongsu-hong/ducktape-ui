use super::*;

pub(in crate::check) fn infer_layout_group(
    node: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::Layout {
            kind,
            options,
            id,
            styles,
            children,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            if let Some(columns) = &options.columns {
                require_type(&expr_type(columns, env, document, span)?, &Type::I64, span)?;
                if matches!(columns, Expr::I64(value) if *value <= 0) {
                    return Err(Error::new("E124", span, "grid cols must be positive"));
                }
            }
            for (value, label) in [
                (&options.min_cell, "grid minimum cell width"),
                (&options.max_cell, "grid maximum cell width"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_f32_literal_range(value, f64::EPSILON, None, label, span)?;
                }
            }
            if let Some(height) = &options.grid_height {
                match height {
                    GridSizing::AspectRatio { width, height } => {
                        for (value, label) in
                            [(width, "grid aspect width"), (height, "grid aspect height")]
                        {
                            require_type(
                                &expr_type(value, env, document, span)?,
                                &Type::F64,
                                span,
                            )?;
                            require_f32_literal_range(value, f64::EPSILON, None, label, span)?;
                        }
                    }
                    GridSizing::EvenlyDistribute(length) => {
                        check_length_value(length, env, document, span, "grid height")?;
                    }
                }
            }
            if let Some(clip) = &options.clip {
                require_type(&expr_type(clip, env, document, span)?, &Type::Bool, span)?;
            }
            let layout_metric = match kind {
                Layout::Column => "column metric",
                Layout::Row => "row metric",
                Layout::Stack => "stack size",
                Layout::Scroll => "scroll metric",
                Layout::Grid => "grid metric",
            };
            if let Some(width) = &options.width {
                if *kind == Layout::Grid {
                    let LengthValue::Fixed(value) = width else {
                        unreachable!("parser keeps grid widths fixed")
                    };
                    require_nonnegative_f64(value, env, document, layout_metric, span)?;
                } else {
                    check_length_value(width, env, document, span, layout_metric)?;
                }
            }
            if let Some(height) = &options.height {
                check_length_value(height, env, document, span, layout_metric)?;
            }
            for value in [
                &options.spacing,
                &options.padding.all,
                &options.padding.x,
                &options.padding.y,
                &options.padding.top,
                &options.padding.right,
                &options.padding.bottom,
                &options.padding.left,
                &options.max_width,
                &options.max_height,
                &options.wrap_spacing,
            ]
            .into_iter()
            .flatten()
            {
                require_nonnegative_f64(value, env, document, layout_metric, span)?;
            }
            if let Some(flexbox) = &options.flexbox {
                for value in [&flexbox.row_gap, &flexbox.column_gap]
                    .into_iter()
                    .flatten()
                {
                    require_nonnegative_f64(value, env, document, "flex gap", span)?;
                }
            }
            if let Some(scroll) = &options.scroll {
                for length in [&scroll.width, &scroll.height].into_iter().flatten() {
                    check_length_value(length, env, document, span, "scroll size")?;
                }
                for (value, label) in [
                    (&scroll.bar_width, "scroll bar width"),
                    (&scroll.bar_margin, "scroll bar margin"),
                    (&scroll.scroller_width, "scroll scroller width"),
                    (&scroll.bar_spacing, "scroll bar spacing"),
                ] {
                    if let Some(value) = value {
                        require_nonnegative_f64(value, env, document, label, span)?;
                    }
                }
                if let Some(auto_scroll) = &scroll.auto_scroll {
                    require_type(
                        &expr_type(auto_scroll, env, document, span)?,
                        &Type::Bool,
                        span,
                    )?;
                }
                if let Some(route) = &scroll.route {
                    infer_ordered_payload_route(
                        route,
                        &[Type::F64, Type::F64, Type::F64, Type::F64],
                        env,
                        document,
                        signatures,
                        "scroll viewport",
                    )?;
                }
                if let Some(route) = &scroll.viewport_route {
                    infer_ordered_payload_route(
                        route,
                        &[const { Type::F64 }; 14],
                        env,
                        document,
                        signatures,
                        "complete scroll viewport",
                    )?;
                }
                if let Some(style) = &scroll.custom_style {
                    let function =
                        extern_function(document, &style.function, ExternKind::ScrollStyle, span)?;
                    check_call_args(function, &style.args, env, document, span)?;
                }
                check_scroll_styles(&scroll.styles, env, document)?;
            }
            check_styles(styles, document, span, StyleTarget::Layout(*kind, options))?;
            for child in children {
                infer_view(child, env, document, signatures, ids)?;
            }
        }
        ViewNode::Container {
            options,
            id,
            styles,
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            for length in [&options.width, &options.height].into_iter().flatten() {
                check_length_value(length, env, document, span, "box size")?;
            }
            for value in [
                &options.padding.all,
                &options.padding.x,
                &options.padding.y,
                &options.padding.top,
                &options.padding.right,
                &options.padding.bottom,
                &options.padding.left,
                &options.max_width,
                &options.max_height,
            ]
            .into_iter()
            .flatten()
            {
                require_nonnegative_f64(value, env, document, "box metric", span)?;
            }
            if let Some(clip) = &options.clip {
                require_type(&expr_type(clip, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(order) = &options.flex_item.order {
                require_type(&expr_type(order, env, document, span)?, &Type::I64, span)?;
            }
            for (value, label) in [
                (&options.flex_item.grow, "grow"),
                (&options.flex_item.shrink, "shrink"),
            ] {
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, label, span)?;
                }
            }
            if let Some(basis) = &options.flex_item.basis {
                let value = match basis {
                    FlexBasisValue::Fixed(value) | FlexBasisValue::Percent(value) => Some(value),
                    FlexBasisValue::Auto | FlexBasisValue::Content => None,
                };
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, "basis", span)?;
                }
            }
            for margin in [
                &options.flex_item.margin.all,
                &options.flex_item.margin.x,
                &options.flex_item.margin.y,
                &options.flex_item.margin.top,
                &options.flex_item.margin.right,
                &options.flex_item.margin.bottom,
                &options.flex_item.margin.left,
            ]
            .into_iter()
            .flatten()
            {
                let value = match margin {
                    FlexMarginValue::Fixed(value) | FlexMarginValue::Percent(value) => Some(value),
                    FlexMarginValue::Auto => None,
                };
                if let Some(value) = value {
                    require_f32_value(value, env, document, "flex margin", span)?;
                }
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::ContainerStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_container_style_options(&options.style, env, document, span, "E184")?;
            check_styles(styles, document, span, StyleTarget::Container(options))?;
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Overlay {
            options,
            content,
            layer,
            span,
        } => {
            require_type(
                &expr_type(&options.visible, env, document, span)?,
                &Type::Bool,
                span,
            )?;
            require_nonnegative_f64(&options.padding, env, document, "overlay padding", span)?;
            require_theme_color(
                &options.backdrop,
                document,
                span,
                "E185",
                "overlay backdrop",
            )?;
            if let Some(dismiss) = &options.dismiss {
                infer_route(dismiss, None, env, document, signatures)?;
            }
            infer_view(content, env, document, signatures, ids)?;
            infer_view(layer, env, document, signatures, ids)?;
        }
        ViewNode::PaneGrid {
            name,
            options,
            panes,
            templates,
            span,
            ..
        } => {
            if !ids.insert(name.clone()) {
                return Err(Error::new(
                    "E161",
                    span,
                    format!("duplicate local id `#{name}`"),
                ));
            }
            for length in [&options.width, &options.height].into_iter().flatten() {
                check_length_value(length, env, document, span, "panes bounds")?;
            }
            for (value, label) in [
                (&options.spacing, "panes spacing"),
                (&options.min_size, "panes minimum size"),
                (&options.resize_leeway, "panes resize leeway"),
            ] {
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, label, span)?;
                }
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::PaneGridStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            if let Some(background) = &options.style.region_background {
                check_background_value(
                    background,
                    env,
                    document,
                    span,
                    "E187",
                    "panes background",
                )?;
            }
            for color in [
                &options.style.region_border,
                &options.style.hovered_split,
                &options.style.picked_split,
            ]
            .into_iter()
            .flatten()
            {
                require_theme_color(color, document, span, "E187", "panes style")?;
            }
            for value in [
                &options.style.region_border_width,
                &options.style.region_radius,
                &options.style.region_radius_top_left,
                &options.style.region_radius_top_right,
                &options.style.region_radius_bottom_right,
                &options.style.region_radius_bottom_left,
                &options.style.hovered_split_width,
                &options.style.picked_split_width,
            ]
            .into_iter()
            .flatten()
            {
                require_nonnegative_f64(value, env, document, "panes style metric", span)?;
            }
            if let Some(click) = &options.click {
                infer_route(click, Some(Type::Str), env, document, signatures)?;
            }
            for pane in panes {
                infer_pane_view(pane, env, document, signatures, ids)?;
            }
            for template in templates {
                let Some(Type::List(item_type)) = env.get(&template.items) else {
                    return Err(Error::new(
                        "E187",
                        &template.span,
                        format!(
                            "dynamic pane template `{}` requires list state `{}`",
                            template.item, template.items
                        ),
                    ));
                };
                let mut template_env = env.clone();
                template_env.insert(template.item.clone(), (**item_type).clone());
                let key_type = expr_type(&template.key, &template_env, document, &template.span)?;
                if !matches!(key_type, Type::Bool | Type::I64 | Type::F64 | Type::Str) {
                    return Err(Error::new(
                        "E187",
                        &template.span,
                        "dynamic pane keys must be bool, i64, f64, or str values",
                    ));
                }
                infer_pane_view(&template.pane, &template_env, document, signatures, ids)?;
            }
        }
        _ => return Ok(false),
    };
    Ok(true)
}
