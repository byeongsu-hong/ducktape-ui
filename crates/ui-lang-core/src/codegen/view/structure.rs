use super::*;

pub(in crate::codegen) fn render_structure(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Theme { id, .. }
        | ViewNode::Float { id, .. }
        | ViewNode::Pin { id, .. }
        | ViewNode::Sensor { id, .. }
        | ViewNode::Responsive { id, .. }
        | ViewNode::KeyedColumn { id, .. }
        | ViewNode::Lazy { id, .. } => id.as_ref(),
        _ => None,
    };
    let child_scope = rendered_child_scope(id, scope, env, document)?;
    let rendered = match node {
        ViewNode::Theme { content, span, .. } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let theme = document.program().nested_theme(span)?;
            let preset = resolved_theme_preset_code(&theme.preset, env, document.program())?;
            let text = theme.text.as_ref().map(resolved_theme_color).map_or_else(
                || "::std::option::Option::None".into(),
                |color| format!("::std::option::Option::Some({color})"),
            );
            let background = theme
                .background
                .as_ref()
                .map(|background| resolved_background_code(background, env, document))
                .transpose()?
                .map_or_else(
                    || "::std::option::Option::None".into(),
                    |background| format!("::std::option::Option::Some({background})"),
                );
            Ok(format!(
                "{{ let __theme_content: __IceElement<'_, {message}> = {content}; ::ui_lang_runtime::dynamic_themer({preset}, __theme_content, {text}, {background}).into() }}"
            ))
        }
        ViewNode::Float {
            scale,
            x,
            y,
            style,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let scale = clamped_f32_code(scale, "f32::EPSILON", "f32::MAX", env, document)?;
            let mut translate_env = env.clone();
            for (name, code) in [
                ("original_x", "(__original.x as f64)"),
                ("original_y", "(__original.y as f64)"),
                ("original_width", "(__original.width as f64)"),
                ("original_height", "(__original.height as f64)"),
                ("viewport_x", "(__viewport.x as f64)"),
                ("viewport_y", "(__viewport.y as f64)"),
                ("viewport_width", "(__viewport.width as f64)"),
                ("viewport_height", "(__viewport.height as f64)"),
            ] {
                translate_env.insert(
                    name.to_owned(),
                    Binding {
                        code: code.to_owned(),
                        ty: Type::F64,
                        local: true,
                        state: None,
                    },
                );
            }
            let x = expr_code(x, &translate_env, document, ValueMode::Owned)?;
            let y = expr_code(y, &translate_env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __float_content: __IceElement<'_, {message}> = {content}; let __float = ::iced::widget::float(__float_content).scale({scale}).translate(move |__original, __viewport| ::iced::Vector::new({x} as f32, {y} as f32))"
            );
            append_float_style(&mut code, style, env, document)?;
            Ok(format!("{code}; __float.into() }}"))
        }
        ViewNode::Pin {
            width,
            height,
            x,
            y,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let x = expr_code(x, env, document, ValueMode::Owned)?;
            let y = expr_code(y, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __pin_content: __IceElement<'_, {message}> = {content}; ::iced::widget::pin(__pin_content).x({x} as f32).y({y} as f32)"
            );
            append_dimensions(&mut code, [width, height], env, document)?;
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Sensor {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, &child_scope, slot)?;
            let mut code = format!(
                "{{ let __sensor_content: __IceElement<'_, {message}> = {content}; ::iced::widget::sensor(__sensor_content)"
            );
            if let Some(route) = &options.show {
                let callback = ordered_route_callback_code(
                    route,
                    "__size",
                    &["__size.width as f64", "__size.height as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, ".on_show({callback})").unwrap();
            }
            if let Some(route) = &options.resize {
                let callback = ordered_route_callback_code(
                    route,
                    "__size",
                    &["__size.width as f64", "__size.height as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, ".on_resize({callback})").unwrap();
            }
            if let Some(route) = &options.hide {
                write!(
                    code,
                    ".on_hide({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(key) = &options.key {
                write!(
                    code,
                    ".key({})",
                    expr_code(key, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(distance) = &options.anticipate {
                write!(
                    code,
                    ".anticipate({})",
                    clamped_f32_code(distance, "0.0", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(delay) = &options.delay_ms {
                write!(
                    code,
                    ".delay(::std::time::Duration::from_millis(u64::try_from({}).unwrap_or(0)))",
                    expr_code(delay, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Responsive {
            content,
            width,
            height,
            ..
        } => {
            let builder = match content {
                ResponsiveContent::Breakpoint {
                    breakpoint,
                    narrow,
                    wide,
                } => {
                    let breakpoint =
                        clamped_f32_code(breakpoint, "f32::EPSILON", "f32::MAX", env, document)?;
                    let narrow = render_node(narrow, document, message, env, &child_scope, slot)?;
                    let wide = render_node(wide, document, message, env, &child_scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = if __size.width < {breakpoint} {{ {narrow} }} else {{ {wide} }}; __responsive }}"
                    )
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.insert(
                        width.clone(),
                        Binding {
                            code: "(__size.width as f64)".into(),
                            ty: Type::F64,
                            local: true,
                            state: None,
                        },
                    );
                    child_env.insert(
                        height.clone(),
                        Binding {
                            code: "(__size.height as f64)".into(),
                            ty: Type::F64,
                            local: true,
                            state: None,
                        },
                    );
                    let content =
                        render_node(content, document, message, &child_env, &child_scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = {content}; __responsive }}"
                    )
                }
            };
            let mut code = format!("::iced::widget::responsive({builder})");
            append_dimensions(&mut code, [width, height], env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::KeyedColumn {
            item,
            items,
            key,
            options,
            child,
            span,
            ..
        } => render_keyed_column(
            item,
            items,
            key,
            options,
            child,
            span,
            document,
            message,
            env,
            &child_scope,
            slot,
        ),
        ViewNode::Lazy {
            dependency,
            binding,
            child,
            span,
            ..
        } => {
            let dependency_type = expr_type(dependency, &env_types(env), document, span)?;
            let dependency = expr_code(dependency, env, document, ValueMode::Owned)?;
            let mut child_env = HashMap::new();
            child_env.insert(
                binding.clone(),
                Binding {
                    code: binding.clone(),
                    ty: dependency_type.clone(),
                    local: false,
                    state: None,
                },
            );
            let child = render_node(
                child,
                document,
                message,
                &child_env,
                "__lazy_scope.clone()",
                None,
            )?;
            let dependency_rust = dependency_type.rust(&document.structs);
            Ok(format!(
                "::iced::widget::lazy(({dependency}, ({child_scope}).to_owned(), __ice_palette.name), move |__dependency| {{ let {binding}: {dependency_rust} = __dependency.0.clone(); let __lazy_scope = __dependency.1.clone(); let __lazy_content: __IceElement<'static, {message}> = {child}; __lazy_content }}).into()"
            ))
        }
        _ => return Ok(None),
    }?;
    Ok(Some(identify_rendered(
        rendered, id, message, env, document, scope,
    )?))
}
