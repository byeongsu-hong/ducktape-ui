use super::*;

pub(in crate::codegen) fn render_controls(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Toggler { id, .. }
        | ViewNode::Slider { id, .. }
        | ViewNode::Progress { id, .. }
        | ViewNode::Radio { id, .. }
        | ViewNode::PickList { id, .. }
        | ViewNode::ComboBox { id, .. } => id.as_ref(),
        _ => None,
    };
    let rendered = match node {
        ViewNode::Button {
            label,
            content,
            id,
            disabled,
            options,
            route,
            span,
            ..
        } => {
            let style = &document.program().style_use(span)?.style;
            let message_code = route_code(route, "", env, document, message)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "button", span, scope, env, document)?;
            let (accessibility_label, accessibility_description) = accessibility_code(
                &options.accessibility,
                || rust_string(label.as_ref().expect("checked button accessibility label")),
                env,
                document,
            )?;
            let disabled_value = disabled
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
            let mut content = if let Some(content) = content {
                let child_scope = id.as_ref().map_or_else(
                    || Ok(scope.to_owned()),
                    |id| id_code(id, scope, env, document),
                )?;
                render_node(content, document, message, env, &child_scope, slot)?
            } else {
                let label = rust_string(label.as_ref().expect("button label"));
                let mut label = format!("::iced::widget::text({label})");
                append_text_options(&mut label, &TextOptions::default(), style, env, document)?;
                format!("{label}.into()")
            };
            let center_x = matches!(options.width.as_ref(), Some(LengthValue::Fixed(_)));
            let center_y = matches!(options.height.as_ref(), Some(LengthValue::Fixed(_)));
            if center_x || center_y {
                let mut centered = format!(
                    "{{ let __button_inner: __IceElement<'_, {message}> = {content}; ::iced::widget::container(__button_inner)"
                );
                if center_x {
                    centered.push_str(
                        ".width(::iced::Fill).align_x(::iced::alignment::Horizontal::Center)",
                    );
                }
                if center_y {
                    centered.push_str(
                        ".height(::iced::Fill).align_y(::iced::alignment::Vertical::Center)",
                    );
                }
                content = format!("{centered}.into() }}");
            }
            let mut code = format!(
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __disabled = {disabled_value}; let __activate = {message_code}; let __button_content: __IceElement<'_, {message}> = {content}; let __button = ::iced::widget::button(__button_content)"
            );
            if let Some(padding) = style.padding_code() {
                write!(code, ".padding({padding})").unwrap();
            }
            append_dimensions(&mut code, [&options.width, &options.height], env, document)?;
            if let Some(padding) = &options.padding {
                write!(
                    code,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(clip) = &options.clip {
                write!(
                    code,
                    ".clip({})",
                    expr_code(clip, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            code.push_str(
                ".on_press_maybe(if __disabled { None } else { Some(__activate.clone()) })",
            );
            code.push_str(&button_style_code(style, &options.style, env, document)?);
            Ok(format!(
                "{code}; ::ui_lang_runtime::accessible(__button, __a11y_id, ::ui_lang_runtime::Role::Button).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
            ))
        }
        ViewNode::Checkbox {
            label,
            id,
            checked,
            disabled,
            options,
            style,
            route,
            span,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "checkbox", span, scope, env, document)?;
            let (accessibility_label, accessibility_description) = accessibility_code(
                &options.accessibility,
                || "__label.clone()".into(),
                env,
                document,
            )?;
            let disabled_value = disabled
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
            let mut code = format!(
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __label = {label}; let __checked = {checked}; let __disabled = {disabled_value}; let __activate = {{ let __value = !__checked; {message_code} }}; let __checkbox = ::iced::widget::checkbox(__checked).label(__label.clone())"
            );
            append_bool_control_options(&mut code, options, env, document, false)?;
            write!(
                code,
                ".on_toggle_maybe(if __disabled {{ None }} else {{ Some({callback}) }})"
            )
            .unwrap();
            code.push_str(&checkbox_style_code(style, env, document)?);
            Ok(format!(
                "{code}; ::ui_lang_runtime::accessible(__checkbox, __a11y_id, ::ui_lang_runtime::Role::CheckBox).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).checked(__checked).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
            ))
        }
        ViewNode::Toggler {
            label,
            id,
            checked,
            disabled,
            options,
            style,
            route,
            span,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let activation = route_code(route, "!__checked", env, document, message)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "toggler", span, scope, env, document)?;
            let (accessibility_label, accessibility_description) = accessibility_code(
                &options.accessibility,
                || "__label.clone()".into(),
                env,
                document,
            )?;
            let disabled_value = disabled
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
            let mut code = format!(
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __label = {label}; let __checked = {checked}; let __disabled = {disabled_value}; let __activate = {activation}; let __toggler = ::iced::widget::toggler(__checked).label(__label.clone())"
            );
            append_bool_control_options(&mut code, options, env, document, true)?;
            write!(
                code,
                ".on_toggle_maybe(if __disabled {{ None }} else {{ Some({callback}) }})"
            )
            .unwrap();
            code.push_str(&toggler_style_code(style, env, document)?);
            Ok(format!(
                "{code}; ::ui_lang_runtime::accessible(__toggler, __a11y_id, ::ui_lang_runtime::Role::Switch).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).checked(__checked).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
            ))
        }
        ViewNode::Slider {
            value,
            id,
            min,
            max,
            step,
            options,
            vertical,
            route,
            release,
            span,
            ..
        } => {
            let value = expr_code(value, env, document, ValueMode::Borrowed)?;
            let min = expr_code(min, env, document, ValueMode::Borrowed)?;
            let max = expr_code(max, env, document, ValueMode::Borrowed)?;
            let step = expr_code(step, env, document, ValueMode::Borrowed)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let helper = if *vertical {
                "vertical_slider"
            } else {
                "slider"
            };
            let mut code = format!(
                "::iced::widget::{helper}(({min})..=({max}), __slider_value, {callback}).step({step})"
            );
            if let Some(default) = &options.default {
                write!(
                    code,
                    ".default({})",
                    expr_code(default, env, document, ValueMode::Borrowed)?
                )
                .unwrap();
            }
            if let Some(shift_step) = &options.shift_step {
                write!(
                    code,
                    ".shift_step({})",
                    expr_code(shift_step, env, document, ValueMode::Borrowed)?
                )
                .unwrap();
            }
            for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
                if let Some(length) = length {
                    write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
                }
            }
            append_slider_styles(&mut code, &options.style, env, document)?;
            if let Some(release) = release {
                write!(
                    code,
                    ".on_release({})",
                    route_code(release, "", env, document, message)?
                )
                .unwrap();
            }
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "slider", span, scope, env, document)?;
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __slider_value = {value}; let __slider = {code}; ::ui_lang_runtime::accessible(__slider, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::Slider).logical_id(__a11y_key.clone()).label(\"Slider\").value(format!(\"{{}}\", __slider_value)).into() }}"
            ))
        }
        ViewNode::Progress {
            value,
            id,
            min,
            max,
            options,
            vertical,
            span,
            ..
        } => {
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let min = expr_code(min, env, document, ValueMode::Owned)?;
            let max = expr_code(max, env, document, ValueMode::Owned)?;
            let mut code =
                "::iced::widget::progress_bar(__progress_range, __progress_value)".to_owned();
            if let Some(length) = &options.length {
                write!(code, ".length({})", length_code(length, env, document)?).unwrap();
            }
            if let Some(girth) = &options.girth {
                write!(code, ".girth({})", length_code(girth, env, document)?).unwrap();
            }
            if *vertical {
                code.push_str(".vertical()");
            }
            append_progress_options(&mut code, options, env, document)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "progress", span, scope, env, document)?;
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __progress_input = {value}; let __progress = {{ let (__progress_range, __progress_value) = ::ui_lang_runtime::progress_range({min}, {max}, __progress_input); {code} }}; ::ui_lang_runtime::accessible(__progress, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::ProgressIndicator).logical_id(__a11y_key.clone()).label(\"Progress\").value(format!(\"{{}}\", __progress_input)).into() }}"
            ))
        }
        ViewNode::Radio {
            label,
            value,
            selected,
            options,
            style,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let callback = route_callback_code(route, "_", &value, env, document, message)?;
            let mut code = format!(
                "::iced::widget::radio({label}, true, if {selected} {{ Some(true) }} else {{ None }}, {callback})"
            );
            append_bool_control_options(&mut code, options, env, document, false)?;
            code.push_str(&radio_style_code(style, env, document)?);
            Ok(format!("{code}.into()"))
        }
        ViewNode::PickList { id, .. } => render_pick_list(
            document.hir().resolved_pick_list_for(node)?,
            id.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        ViewNode::ComboBox { id, .. } => render_combo_box(
            document.hir().resolved_combo_box_for(node)?,
            id.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}
