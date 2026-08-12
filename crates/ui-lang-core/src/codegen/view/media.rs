use super::*;

pub(in crate::codegen) fn render_media(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let view = document.resolved_view(node)?;
    let identity = view.identity.as_ref();
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    let rendered = match &view.kind {
        ResolvedViewKind::Media => {
            let resolved = document.resolved_media(node)?;
            render_resolved_media(resolved, identity, document, env, scope)
        }
        ResolvedViewKind::Tooltip { content, tip } => {
            let content = render_node(*content, document, message, env, &child_scope, slot)?;
            let tip = render_node(*tip, document, message, env, &child_scope, slot)?;
            let resolved = document.resolved_tooltip(node)?;
            render_resolved_tooltip(resolved, document, message, env, content, tip)
        }
        ResolvedViewKind::MouseArea { content } => {
            let content = render_node(*content, document, message, env, &child_scope, slot)?;
            let resolved = document.resolved_mouse_area(node)?;
            render_resolved_mouse_area(resolved, document, message, env, content)
        }
        ResolvedViewKind::ResizeHandle { content } => {
            let content = render_node(*content, document, message, env, &child_scope, slot)?;
            let resolved = document.resolved_resize_handle(node)?;
            render_resolved_resize_handle(resolved, document, message, env, content)
        }
        ResolvedViewKind::Canvas => {
            let resolved = document.resolved_canvas(node)?;
            render_canvas(resolved, document, message, env)
        }
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, identity, message, env, document, scope)?;
    Ok(Some(rendered))
}

fn render_resolved_mouse_area(
    mouse: &ResolvedMouseArea,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    content: String,
) -> Result<String, Error> {
    let press_at = match &mouse.press_at {
        Some(route) => {
            let callback = resolved_interaction_route_callback_code(
                route,
                "__point",
                &["__point.x as f64", "__point.y as f64"],
                env,
                program,
                message,
            )?;
            Some(format!(
                "::ui_lang_runtime::press_area({{inner}}).on_press_at({callback})"
            ))
        }
        None => None,
    };
    // `press-at=` alone needs no stock mouse area at all: the press observer
    // wraps the content directly.
    if let Some(press_at) = &press_at
        && mouse_area_is_press_at_only(mouse)
    {
        let wrapped = press_at.replace("{inner}", "__mouse_content");
        return Ok(format!(
            "{{ let __mouse_content: __IceElement<'_, {message}> = {content}; {wrapped}.into() }}"
        ));
    }
    let mut code = format!(
        "{{ let __mouse_content: __IceElement<'_, {message}> = {content}; ::iced::widget::mouse_area(__mouse_content)"
    );
    for (route, method) in [
        (&mouse.press, "on_press"),
        (&mouse.release, "on_release"),
        (&mouse.double_click, "on_double_click"),
        (&mouse.right_press, "on_right_press"),
        (&mouse.right_release, "on_right_release"),
        (&mouse.middle_press, "on_middle_press"),
        (&mouse.middle_release, "on_middle_release"),
        (&mouse.enter, "on_enter"),
        (&mouse.exit, "on_exit"),
    ] {
        if let Some(route) = route {
            write!(
                code,
                ".{method}({})",
                resolved_interaction_route_code(route, &[], env, program, message)?
            )
            .unwrap();
        }
    }
    if let Some(route) = &mouse.move_route {
        let callback = resolved_interaction_route_callback_code(
            route,
            "__point",
            &["__point.x as f64", "__point.y as f64"],
            env,
            program,
            message,
        )?;
        write!(code, ".on_move({callback})").unwrap();
    }
    if let Some(route) = &mouse.scroll {
        let callback = resolved_interaction_route_callback_with_code(
            route,
            "__delta",
            env,
            program,
            |callback_env| {
                let lines = resolved_interaction_route_code(
                    route,
                    &["__x as f64", "__y as f64", "false"],
                    callback_env,
                    program,
                    message,
                )?;
                let pixels = resolved_interaction_route_code(
                    route,
                    &["__x as f64", "__y as f64", "true"],
                    callback_env,
                    program,
                    message,
                )?;
                Ok(format!(
                    "match __delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {lines}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {pixels} }}"
                ))
            },
        )?;
        write!(code, ".on_scroll({callback})").unwrap();
    }
    if let Some(interaction) = mouse.interaction {
        write!(
            code,
            ".interaction(::iced::mouse::Interaction::{})",
            mouse_interaction_code(interaction)
        )
        .unwrap();
    } else if let Some(interaction) = mouse.interaction_expression {
        write!(
            code,
            ".interaction({})",
            resolved_expr_use_code(program, interaction, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    // The press observer wraps the finished mouse area, so its callback fires
    // after the content — captured or not — has processed the press.
    if let Some(press_at) = press_at {
        let wrapped = press_at.replace("{inner}", "__press_content");
        return Ok(format!(
            "{{ let __press_content: __IceElement<'_, {message}> = {code}.into() }}; {wrapped}.into() }}"
        ));
    }
    Ok(format!("{code}.into() }}"))
}

fn mouse_area_is_press_at_only(mouse: &ResolvedMouseArea) -> bool {
    mouse.press.is_none()
        && mouse.release.is_none()
        && mouse.double_click.is_none()
        && mouse.right_press.is_none()
        && mouse.right_release.is_none()
        && mouse.middle_press.is_none()
        && mouse.middle_release.is_none()
        && mouse.enter.is_none()
        && mouse.exit.is_none()
        && mouse.move_route.is_none()
        && mouse.scroll.is_none()
        && mouse.interaction.is_none()
        && mouse.interaction_expression.is_none()
}

fn render_resolved_resize_handle(
    handle: &ResolvedResizeHandle,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    content: String,
) -> Result<String, Error> {
    let mut code = format!(
        "{{ let __resize_content: __IceElement<'_, {message}> = {content}; ::ui_lang_runtime::resize_handle(__resize_content)"
    );
    if let Some(route) = &handle.drag {
        let callback = resolved_interaction_route_callback_code(
            route,
            "__dx, __dy",
            &["__dx", "__dy"],
            env,
            program,
            message,
        )?;
        write!(code, ".on_drag({callback})").unwrap();
    }
    for (route, method) in [(&handle.press, "on_press"), (&handle.release, "on_release")] {
        if let Some(route) = route {
            write!(
                code,
                ".{method}({})",
                resolved_interaction_route_code(route, &[], env, program, message)?
            )
            .unwrap();
        }
    }
    if let Some(interaction) = handle.interaction {
        write!(
            code,
            ".interaction(::iced::mouse::Interaction::{})",
            mouse_interaction_code(interaction)
        )
        .unwrap();
    }
    Ok(format!("{code}.into() }}"))
}

fn render_resolved_tooltip(
    tooltip: &ResolvedTooltip,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    content: String,
    tip: String,
) -> Result<String, Error> {
    let position = match tooltip.position {
        ResolvedTooltipPosition::Top => "Top",
        ResolvedTooltipPosition::Bottom => "Bottom",
        ResolvedTooltipPosition::Left => "Left",
        ResolvedTooltipPosition::Right => "Right",
        ResolvedTooltipPosition::FollowCursor => "FollowCursor",
    };
    let gap = resolved_expr_use_code(program, tooltip.gap, env, ValueMode::Owned)?;
    let padding = resolved_expr_use_code(program, tooltip.padding, env, ValueMode::Owned)?;
    let delay = resolved_expr_use_code(program, tooltip.delay_ms, env, ValueMode::Owned)?;
    let snap = resolved_expr_use_code(program, tooltip.snap, env, ValueMode::Owned)?;
    let mut code = format!(
        "{{ let __tooltip_content: __IceElement<'_, {message}> = {content}; let __tooltip_tip: __IceElement<'_, {message}> = {tip}; ::iced::widget::tooltip(__tooltip_content, __tooltip_tip, ::iced::widget::tooltip::Position::{position}).gap(::ui_lang_runtime::bounded_table_metric({gap}, 1)).padding(::ui_lang_runtime::bounded_table_metric({padding}, 1)).delay(::std::time::Duration::from_millis(u64::try_from({delay}).unwrap_or(0))).snap_within_viewport({snap})"
    );
    append_resolved_tooltip_style(&mut code, tooltip, program, env)?;
    code.push_str(".into() }");
    Ok(code)
}

fn append_resolved_tooltip_style(
    code: &mut String,
    tooltip: &ResolvedTooltip,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let radius = resolved_tooltip_radius(&tooltip.radius, program, env)?;
    if tooltip.base_style.is_none()
        && tooltip.background.is_none()
        && tooltip.text_color.is_none()
        && tooltip.border_color.is_none()
        && tooltip.border_width.is_none()
        && radius.is_none()
        && tooltip.shadow_color.is_none()
        && tooltip.shadow_x.is_none()
        && tooltip.shadow_y.is_none()
        && tooltip.shadow_blur.is_none()
        && tooltip.pixel_snap.is_none()
    {
        return Ok(());
    }
    match &tooltip.base_style {
        Some(ResolvedTooltipBaseStyle::Custom(style)) => {
            let function = program.extern_function(style.function);
            let arguments = style
                .arguments
                .iter()
                .map(|argument| resolved_expr_use_code(program, *argument, env, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            let suffix = if arguments.is_empty() {
                String::new()
            } else {
                format!(", {}", arguments.join(", "))
            };
            write!(
                code,
                ".style(move |__theme| {{ let mut __style = {}(__theme{suffix});",
                function.rust_path
            )
            .unwrap();
        }
        preset => {
            let preset = match preset {
                None
                | Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Transparent)) => {
                    "transparent"
                }
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Rounded)) => {
                    "rounded_box"
                }
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Bordered)) => {
                    "bordered_box"
                }
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Dark)) => "dark",
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Primary)) => "primary",
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Secondary)) => {
                    "secondary"
                }
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Success)) => "success",
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Warning)) => "warning",
                Some(ResolvedTooltipBaseStyle::Preset(ResolvedTooltipPreset::Danger)) => "danger",
                Some(ResolvedTooltipBaseStyle::Custom(_)) => unreachable!(),
            };
            write!(
                code,
                ".style(move |__theme| {{ let mut __style = ::iced::widget::container::{preset}(__theme);"
            )
            .unwrap();
        }
    }
    if let Some(background) = &tooltip.background {
        write!(
            code,
            " __style.background = Some({});",
            resolved_tooltip_background(background, program, env)?
        )
        .unwrap();
    }
    if let Some(text) = &tooltip.text_color {
        write!(
            code,
            " __style.text_color = Some({});",
            resolved_theme_color(text)
        )
        .unwrap();
    }
    if let Some(border) = &tooltip.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(border)
        )
        .unwrap();
    }
    if let Some(width) = tooltip.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            resolved_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(shadow) = &tooltip.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            resolved_theme_color(shadow)
        )
        .unwrap();
    }
    for (value, field) in [
        (tooltip.shadow_x, "__style.shadow.offset.x"),
        (tooltip.shadow_y, "__style.shadow.offset.y"),
        (tooltip.shadow_blur, "__style.shadow.blur_radius"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                " {field} = {} as f32;",
                resolved_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(pixel_snap) = tooltip.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            resolved_expr_use_code(program, pixel_snap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn resolved_tooltip_background(
    background: &ResolvedTooltipBackground,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match background {
        ResolvedTooltipBackground::Color(color) => {
            format!("::iced::Background::Color({})", resolved_theme_color(color))
        }
        ResolvedTooltipBackground::Linear { angle, stops } => {
            let angle = resolved_expr_use_code(program, *angle, env, ValueMode::Owned)?;
            let mut code =
                format!("::iced::Background::from(::iced::gradient::Linear::new({angle} as f32)");
            for stop in stops {
                let offset = resolved_expr_use_code(program, stop.offset, env, ValueMode::Owned)?;
                write!(
                    code,
                    ".add_stop({offset} as f32, {})",
                    resolved_theme_color(&stop.color)
                )
                .unwrap();
            }
            code.push(')');
            code
        }
    })
}

fn resolved_tooltip_radius(
    radius: &ResolvedTooltipRadius,
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

fn render_resolved_media(
    media: &ResolvedMedia,
    identity: Option<&ResolvedViewIdentity>,
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
    let embedded = if media.source_type == Type::Str && matches!(source_mode, ValueMode::Owned) {
        embedded_asset_bytes_code(program, media.source)
    } else {
        None
    };
    let is_embedded = embedded.is_some();
    let source = match embedded {
        Some(bytes) => bytes,
        None => resolved_expr_use_code(program, media.source, env, source_mode)?,
    };
    let mut code = match media.kind {
        ResolvedMediaKind::Image if is_embedded => {
            format!("::iced::widget::image(::iced::widget::image::Handle::from_bytes({source}))")
        }
        ResolvedMediaKind::Image => format!("::iced::widget::image({source})"),
        ResolvedMediaKind::Viewer if is_embedded => format!(
            "::iced::widget::image::viewer(::iced::widget::image::Handle::from_bytes({source}))"
        ),
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
        ResolvedMediaKind::Svg if is_embedded => {
            format!("::iced::widget::svg(::iced::widget::svg::Handle::from_memory({source}))")
        }
        ResolvedMediaKind::Svg => format!("::iced::widget::svg({source})"),
    };
    append_resolved_media_dimensions(&mut code, [&options.width, &options.height], program, env)?;
    if let Some(fit) = options.fit {
        write!(
            code,
            ".content_fit({})",
            resolved_expr_use_code(program, fit, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(rotation) = options.rotation {
        write!(
            code,
            ".rotation({})",
            resolved_expr_use_code(program, rotation, env, ValueMode::Owned)?
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
            resolved_expr_use_code(program, expand, env, ValueMode::Owned)?
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
        let accessibility_key =
            resolved_accessibility_key_code(identity, "media", media.origin, scope, env, program)?;
        let label = resolved_expr_use_code(program, label, env, ValueMode::Owned)?;
        let description = options
            .accessibility_description
            .map(|description| {
                resolved_expr_use_code(program, description, env, ValueMode::Owned)
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
                let value = resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?;
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
    let _derived_guard = enter_escaping_derived_reads();
    let function = program.extern_function(style.function);
    let arguments = style
        .arguments
        .iter()
        .map(|argument| resolved_expr_use_code(program, *argument, env, ValueMode::Owned))
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
    let code = resolved_expr_use_code(program, expression, env, ValueMode::Owned)?;
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
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)
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
        resolved_expr_use_code(program, expression, env, ValueMode::Owned)?
    ))
}
