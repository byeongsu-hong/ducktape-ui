use super::*;

pub(in crate::codegen) fn render_media(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
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
        ViewNode::Media {
            kind,
            source,
            options,
            span,
            ..
        } => {
            let source_type = expr_type(source, &env_types(env), document, span)?;
            let source_mode =
                if *kind == MediaKind::Svg && options.svg_memory && source_type == Type::Str {
                    ValueMode::Borrowed
                } else {
                    ValueMode::Owned
                };
            let source = expr_code(source, env, document, source_mode)?;
            let mut code = match kind {
                MediaKind::Image => format!("::iced::widget::image({source})"),
                MediaKind::Viewer if source_type == Type::Str => format!(
                    "::iced::widget::image::viewer(::iced::widget::image::Handle::from_path({source}))"
                ),
                MediaKind::Viewer => format!("::iced::widget::image::viewer({source})"),
                MediaKind::Svg if options.svg_memory && source_type == Type::Bytes => format!(
                    "::iced::widget::svg(::iced::widget::svg::Handle::from_memory({source}))"
                ),
                MediaKind::Svg if options.svg_memory => format!(
                    "::iced::widget::svg(::iced::widget::svg::Handle::from_memory(({source}).as_bytes().to_vec()))"
                ),
                MediaKind::Svg => format!("::iced::widget::svg({source})"),
            };
            append_dimensions(&mut code, [&options.width, &options.height], env, document)?;
            if let Some(fit) = &options.fit {
                write!(
                    code,
                    ".content_fit({})",
                    expr_code(fit, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(rotation) = &options.rotation {
                write!(
                    code,
                    ".rotation({})",
                    expr_code(rotation, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(opacity) = &options.opacity {
                write!(
                    code,
                    ".opacity({})",
                    clamped_f32_code(opacity, "0.0", "1.0", env, document)?
                )
                .unwrap();
            }
            if *kind == MediaKind::Svg {
                let custom = options
                    .svg_style
                    .as_ref()
                    .map(|style| {
                        custom_style_call_code(
                            style,
                            ExternKind::SvgStyle,
                            "__theme, __status",
                            env,
                            document,
                        )
                    })
                    .transpose()?;
                let has_colors = options.svg_color.is_some() || options.svg_hover_color.is_some();
                if !has_colors {
                    if let Some(custom) = custom {
                        write!(code, ".style(move |__theme, __status| {custom})").unwrap();
                    }
                } else {
                    let base = custom
                        .unwrap_or_else(|| "::iced::widget::svg::Style::default()".to_owned());
                    let idle = options
                        .svg_color
                        .as_ref()
                        .map(|color| format!("Some({})", theme_color(document, color)));
                    let hovered = match &options.svg_hover_color {
                        Some(Some(color)) => {
                            Some(format!("Some({})", theme_color(document, color)))
                        }
                        Some(None) => Some("None".to_owned()),
                        None => idle.clone(),
                    };
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
                }
            }
            if let Some(filter) = options.filter {
                let filter = match filter {
                    ImageFilter::Linear => "Linear",
                    ImageFilter::Nearest => "Nearest",
                };
                write!(
                    code,
                    ".filter_method(::iced::widget::image::FilterMethod::{filter})"
                )
                .unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    code,
                    ".padding({})",
                    clamped_f32_code(padding, "0.0", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if *kind == MediaKind::Viewer
                && (options.min_scale.is_some() || options.max_scale.is_some())
            {
                let min = options.min_scale.as_ref().map_or_else(
                    || Ok("0.25".into()),
                    |value| expr_code(value, env, document, ValueMode::Owned),
                )?;
                let max = options.max_scale.as_ref().map_or_else(
                    || Ok("10.0".into()),
                    |value| expr_code(value, env, document, ValueMode::Owned),
                )?;
                code = format!(
                    "{{ let (__viewer_min_scale, __viewer_max_scale) = ::ui_lang_runtime::viewer_scale_bounds({min}, {max}); {code}.min_scale(__viewer_min_scale).max_scale(__viewer_max_scale) }}"
                );
            }
            if let Some(step) = &options.scale_step {
                write!(
                    code,
                    ".scale_step({})",
                    clamped_f32_code(step, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(scale) = &options.scale {
                write!(
                    code,
                    ".scale({})",
                    clamped_f32_code(scale, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(expand) = &options.expand {
                write!(
                    code,
                    ".expand({})",
                    expr_code(expand, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(radius) = radius_code(
                options.radius.as_ref(),
                [
                    options.radius_top_left.as_ref(),
                    options.radius_top_right.as_ref(),
                    options.radius_bottom_right.as_ref(),
                    options.radius_bottom_left.as_ref(),
                ],
                env,
                document,
            )? {
                write!(code, ".border_radius({radius})").unwrap();
            }
            if let Some([x, y, width, height]) = &options.crop {
                write!(
                    code,
                    ".crop(::iced::Rectangle {{ x: {}, y: {}, width: {}, height: {} }})",
                    u32_code(x, env, document)?,
                    u32_code(y, env, document)?,
                    u32_code(width, env, document)?,
                    u32_code(height, env, document)?,
                )
                .unwrap();
            }
            if options.accessibility.label.is_some() {
                let accessibility_key =
                    accessibility_key_code(None, "media", span, scope, env, document)?;
                let (label, description) =
                    accessibility_code(&options.accessibility, String::new, env, document)?;
                Ok(format!(
                    "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); ::ui_lang_runtime::accessible({code}, __a11y_id, ::ui_lang_runtime::Role::Image).logical_id(__a11y_key.clone()).label({label}){description}.into() }}"
                ))
            } else {
                Ok(format!("{code}.into()"))
            }
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
        ViewNode::Canvas {
            options,
            locals,
            commands,
            events,
            ..
        } => render_canvas(options, locals, commands, events, document, message, env),
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}
