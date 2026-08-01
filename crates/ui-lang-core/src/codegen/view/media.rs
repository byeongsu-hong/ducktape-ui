use super::*;

pub(in crate::codegen) fn render_media(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Media { id, .. }
        | ViewNode::Tooltip { id, .. }
        | ViewNode::MouseArea { id, .. }
        | ViewNode::ResizeHandle { id, .. }
        | ViewNode::Canvas { id, .. } => id.as_ref(),
        _ => None,
    };
    let child_scope = rendered_child_scope(id, scope, env, document)?;
    let rendered = match node {
        ViewNode::Media { .. } => {
            let hir = document.hir();
            let resolved = hir.resolved_media_for(node)?;
            render_resolved_media(resolved, hir, env, scope)
        }
        ViewNode::Tooltip {
            options,
            content,
            tip,
            ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let tip = render_node(tip, document, message, env, &child_scope, slot)?;
            let position = match options.position {
                TooltipPosition::Top => "Top",
                TooltipPosition::Bottom => "Bottom",
                TooltipPosition::Left => "Left",
                TooltipPosition::Right => "Right",
                TooltipPosition::FollowCursor => "FollowCursor",
            };
            let gap = expr_code(&options.gap, env, document, ValueMode::Owned)?;
            let padding = expr_code(&options.padding, env, document, ValueMode::Owned)?;
            let delay = expr_code(&options.delay_ms, env, document, ValueMode::Owned)?;
            let snap = expr_code(&options.snap, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __tooltip_content: __IceElement<'_, {message}> = {content}; let __tooltip_tip: __IceElement<'_, {message}> = {tip}; ::iced::widget::tooltip(__tooltip_content, __tooltip_tip, ::iced::widget::tooltip::Position::{position}).gap(::ui_lang_runtime::bounded_table_metric({gap}, 1)).padding(::ui_lang_runtime::bounded_table_metric({padding}, 1)).delay(::std::time::Duration::from_millis(u64::try_from({delay}).unwrap_or(0))).snap_within_viewport({snap})"
            );
            append_tooltip_style(&mut code, options, env, document)?;
            code.push_str(".into() }");
            Ok(code)
        }
        ViewNode::MouseArea {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let mut code = format!(
                "{{ let __mouse_content: __IceElement<'_, {message}> = {content}; ::iced::widget::mouse_area(__mouse_content)"
            );
            for (route, method) in [
                (&options.press, "on_press"),
                (&options.release, "on_release"),
                (&options.double_click, "on_double_click"),
                (&options.right_press, "on_right_press"),
                (&options.right_release, "on_right_release"),
                (&options.middle_press, "on_middle_press"),
                (&options.middle_release, "on_middle_release"),
                (&options.enter, "on_enter"),
                (&options.exit, "on_exit"),
            ] {
                if let Some(route) = route {
                    write!(
                        code,
                        ".{method}({})",
                        route_code(route, "", env, document, message)?
                    )
                    .unwrap();
                }
            }
            if let Some(route) = &options.move_route {
                let callback = ordered_route_callback_code(
                    route,
                    "__point",
                    &["__point.x as f64", "__point.y as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, ".on_move({callback})").unwrap();
            }
            if let Some(route) = &options.scroll {
                let callback = route_callback_with_code(
                    route,
                    "__delta",
                    env,
                    document,
                    |callback_env| {
                        let lines = ordered_route_code(
                            route,
                            &["__x as f64", "__y as f64", "false"],
                            callback_env,
                            document,
                            message,
                        )?;
                        let pixels = ordered_route_code(
                            route,
                            &["__x as f64", "__y as f64", "true"],
                            callback_env,
                            document,
                            message,
                        )?;
                        Ok(format!(
                            "match __delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {lines}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {pixels} }}"
                        ))
                    },
                )?;
                write!(code, ".on_scroll({callback})").unwrap();
            }
            if let Some(interaction) = options.interaction {
                write!(
                    code,
                    ".interaction(::iced::mouse::Interaction::{})",
                    mouse_interaction_code(interaction)
                )
                .unwrap();
            } else if let Some(interaction) = &options.interaction_expr {
                write!(
                    code,
                    ".interaction({})",
                    expr_code(interaction, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::ResizeHandle {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let mut code = format!(
                "{{ let __resize_content: __IceElement<'_, {message}> = {content}; ::ui_lang_runtime::resize_handle(__resize_content)"
            );
            if let Some(route) = &options.drag {
                let callback = ordered_route_callback_code(
                    route,
                    "__dx, __dy",
                    &["__dx", "__dy"],
                    env,
                    document,
                    message,
                )?;
                write!(code, ".on_drag({callback})").unwrap();
            }
            for (route, method) in [
                (&options.press, "on_press"),
                (&options.release, "on_release"),
            ] {
                if let Some(route) = route {
                    write!(
                        code,
                        ".{method}({})",
                        route_code(route, "", env, document, message)?
                    )
                    .unwrap();
                }
            }
            if let Some(interaction) = options.interaction {
                write!(
                    code,
                    ".interaction(::iced::mouse::Interaction::{})",
                    mouse_interaction_code(interaction)
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Canvas { .. } => {
            let hir = document.hir();
            let resolved = hir.resolved_canvas_for(node)?;
            render_canvas(resolved, hir, message, env)
        }
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}

fn render_resolved_media(
    media: &ResolvedMedia,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let options = &media.options;
    let source_mode = if media.kind == ResolvedMediaKind::Svg
        && options.svg_memory
        && media.source_type == Type::Str
    {
        ValueMode::Borrowed
    } else {
        ValueMode::Owned
    };
    let source = checked_expr_use_code(program, media.source, env, source_mode)?;
    let mut code = match media.kind {
        ResolvedMediaKind::Image => format!("::iced::widget::image({source})"),
        ResolvedMediaKind::Viewer if media.source_type == Type::Str => format!(
            "::iced::widget::image::viewer(::iced::widget::image::Handle::from_path({source}))"
        ),
        ResolvedMediaKind::Viewer => format!("::iced::widget::image::viewer({source})"),
        ResolvedMediaKind::Svg if options.svg_memory && media.source_type == Type::Bytes => {
            format!("::iced::widget::svg(::iced::widget::svg::Handle::from_memory({source}))")
        }
        ResolvedMediaKind::Svg if options.svg_memory => format!(
            "::iced::widget::svg(::iced::widget::svg::Handle::from_memory(({source}).as_bytes().to_vec()))"
        ),
        ResolvedMediaKind::Svg => format!("::iced::widget::svg({source})"),
    };
    append_resolved_media_dimensions(&mut code, [&options.width, &options.height], program, env)?;
    if let Some(fit) = options.fit {
        write!(
            code,
            ".content_fit({})",
            checked_expr_use_code(program, fit, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(rotation) = options.rotation {
        write!(
            code,
            ".rotation({})",
            checked_expr_use_code(program, rotation, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(opacity) = options.opacity {
        write!(
            code,
            ".opacity({})",
            resolved_media_clamped_f32(opacity, "0.0", "1.0", program, env)?
        )
        .unwrap();
    }
    if media.kind == ResolvedMediaKind::Svg {
        let custom = options
            .svg_style
            .as_ref()
            .map(|style| resolved_media_svg_style(style, program, env))
            .transpose()?;
        if let Some(colors) = &options.svg_colors {
            let base = custom.unwrap_or_else(|| "::iced::widget::svg::Style::default()".into());
            let idle = colors
                .idle
                .as_ref()
                .map(|color| format!("Some({})", resolved_theme_color(color)));
            let hovered = colors.hovered.as_ref().map(|color| {
                color.as_ref().map_or_else(
                    || "None".into(),
                    |color| format!("Some({})", resolved_theme_color(color)),
                )
            });
            let exhaustive = idle.is_some() && hovered.is_some();
            write!(
                code,
                ".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{"
            )
            .unwrap();
            if let Some(idle) = idle {
                write!(
                    code,
                    " ::iced::widget::svg::Status::Idle => __style.color = {idle},"
                )
                .unwrap();
            }
            if let Some(hovered) = hovered {
                write!(
                    code,
                    " ::iced::widget::svg::Status::Hovered => __style.color = {hovered},"
                )
                .unwrap();
            }
            if !exhaustive {
                code.push_str(" _ => {}");
            }
            code.push_str(" } __style })");
        } else if let Some(custom) = custom {
            write!(code, ".style(move |__theme, __status| {custom})").unwrap();
        }
    }
    if let Some(filter) = options.filter {
        let filter = match filter {
            ResolvedMediaFilter::Linear => "Linear",
            ResolvedMediaFilter::Nearest => "Nearest",
        };
        write!(
            code,
            ".filter_method(::iced::widget::image::FilterMethod::{filter})"
        )
        .unwrap();
    }
    if let Some(padding) = options.padding {
        write!(
            code,
            ".padding({})",
            resolved_media_clamped_f32(padding, "0.0", "f32::MAX", program, env)?
        )
        .unwrap();
    }
    if let Some(bounds) = &options.scale_bounds {
        let minimum = resolved_media_scale_bound(&bounds.minimum, program, env)?;
        let maximum = resolved_media_scale_bound(&bounds.maximum, program, env)?;
        code = format!(
            "{{ let (__viewer_min_scale, __viewer_max_scale) = ::ui_lang_runtime::viewer_scale_bounds({minimum}, {maximum}); {code}.min_scale(__viewer_min_scale).max_scale(__viewer_max_scale) }}"
        );
    }
    if let Some(step) = options.scale_step {
        write!(
            code,
            ".scale_step({})",
            resolved_media_clamped_f32(step, "f32::EPSILON", "f32::MAX", program, env)?
        )
        .unwrap();
    }
    if let Some(scale) = options.scale {
        write!(
            code,
            ".scale({})",
            resolved_media_clamped_f32(scale, "f32::EPSILON", "f32::MAX", program, env)?
        )
        .unwrap();
    }
    if let Some(expand) = options.expand {
        write!(
            code,
            ".expand({})",
            checked_expr_use_code(program, expand, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = resolved_media_radius(&options.radius, program, env)? {
        write!(code, ".border_radius({radius})").unwrap();
    }
    if let Some([x, y, width, height]) = options.crop {
        write!(
            code,
            ".crop(::iced::Rectangle {{ x: {}, y: {}, width: {}, height: {} }})",
            resolved_media_u32(x, program, env)?,
            resolved_media_u32(y, program, env)?,
            resolved_media_u32(width, program, env)?,
            resolved_media_u32(height, program, env)?,
        )
        .unwrap();
    }
    if let Some(label) = options.accessibility_label {
        let origin = program.origin(media.origin);
        let span = Span {
            line: media.reconciliation_line,
            column: origin.column,
        };
        let reconciliation_scope = reconciliation_scope(scope, env);
        let accessibility_key = format!(
            "format!(\"{{}}/@media:{}\", {reconciliation_scope})",
            span.line
        );
        let label = checked_expr_use_code(program, label, env, ValueMode::Owned)?;
        let description = options
            .accessibility_description
            .map(|description| {
                checked_expr_use_code(program, description, env, ValueMode::Owned)
                    .map(|value| format!(".description({value})"))
            })
            .transpose()?
            .unwrap_or_default();
        Ok(format!(
            "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); ::ui_lang_runtime::accessible({code}, __a11y_id, ::ui_lang_runtime::Role::Image).logical_id(__a11y_key.clone()).label({label}){description}.into() }}"
        ))
    } else {
        Ok(format!("{code}.into()"))
    }
}

fn append_resolved_media_dimensions(
    code: &mut String,
    dimensions: [&Option<ResolvedMediaLength>; 2],
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    for (method, length) in ["width", "height"].into_iter().zip(dimensions) {
        let Some(length) = length else { continue };
        let value = match length {
            ResolvedMediaLength::Fill => "::iced::Fill".into(),
            ResolvedMediaLength::FillPortion(portion) => {
                format!("::iced::Length::FillPortion({portion})")
            }
            ResolvedMediaLength::Shrink => "::iced::Shrink".into(),
            ResolvedMediaLength::Fixed { expression, source } => {
                let value = checked_expr_use_code(program, *expression, env, ValueMode::Owned)?;
                if *source == Type::Length {
                    value
                } else {
                    format!("{value} as f32")
                }
            }
        };
        write!(code, ".{method}({value})").unwrap();
    }
    Ok(())
}

fn resolved_media_svg_style(
    style: &ResolvedMediaSvgStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let function = program.extern_function(style.function);
    let arguments = style
        .arguments
        .iter()
        .map(|argument| checked_expr_use_code(program, *argument, env, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?;
    let suffix = if arguments.is_empty() {
        String::new()
    } else {
        format!(", {}", arguments.join(", "))
    };
    Ok(format!("{}(__theme, __status{suffix})", function.rust_path))
}

fn resolved_media_clamped_f32(
    expression: ResolvedExpressionId,
    minimum: &str,
    maximum: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let code = checked_expr_use_code(program, expression, env, ValueMode::Owned)?;
    Ok(format!("(({code}) as f32).max({minimum}).min({maximum})"))
}

fn resolved_media_scale_bound(
    bound: &ResolvedMediaScaleBound,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    match bound {
        ResolvedMediaScaleBound::Default(value) => Ok(format!("{value:?}")),
        ResolvedMediaScaleBound::Expression(expression) => {
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)
        }
    }
}

fn resolved_media_radius(
    radius: &ResolvedMediaRadius,
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
    let all = radius
        .all
        .map(|value| resolved_media_clamped_f32(value, "0.0", "f32::MAX", program, env))
        .transpose()?
        .unwrap_or_else(|| "0.0".into());
    let corner = |value: Option<ResolvedExpressionId>| {
        value
            .map(|value| resolved_media_clamped_f32(value, "0.0", "f32::MAX", program, env))
            .transpose()
    };
    let top_left = corner(radius.top_left)?.unwrap_or_else(|| all.clone());
    let top_right = corner(radius.top_right)?.unwrap_or_else(|| all.clone());
    let bottom_right = corner(radius.bottom_right)?.unwrap_or_else(|| all.clone());
    let bottom_left = corner(radius.bottom_left)?.unwrap_or(all);
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {top_left}, top_right: {top_right}, bottom_right: {bottom_right}, bottom_left: {bottom_left} }}"
    )))
}

fn resolved_media_u32(
    expression: ResolvedExpressionId,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(format!(
        "({}).clamp(0, u32::MAX as i64) as u32",
        checked_expr_use_code(program, expression, env, ValueMode::Owned)?
    ))
}
