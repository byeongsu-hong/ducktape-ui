use super::*;

pub(in crate::codegen) fn render_controls(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
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
            styles,
            route,
            span,
        } => {
            let style = Style::parse(styles, document);
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
                append_text_options(&mut label, &TextOptions::default(), &style, env, document)?;
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
            code.push_str(&button_style_code(&style, &options.style, env, document)?);
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
        ViewNode::PickList {
            id,
            options,
            selected,
            options_config,
            route,
            span,
            ..
        } => {
            let options = expr_code(options, env, document, ValueMode::Owned)?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let mut code = format!(
                "{{ let __pick_options = {options}; let __pick_option_count = __pick_options.len(); ::iced::widget::pick_list(__pick_options, __pick_selected.clone(), {callback})"
            );
            if let Some(placeholder) = &options_config.placeholder {
                write!(
                    code,
                    ".placeholder({})",
                    expr_code(placeholder, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(width) = &options_config.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options_config.menu_height {
                write!(
                    code,
                    ".menu_height({})",
                    length_code(height, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = &options_config.padding {
                write!(
                    code,
                    ".padding(::ui_lang_runtime::bounded_table_metric({}, __pick_option_count))",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options_config.text_size {
                write!(
                    code,
                    ".text_size({})",
                    clamped_f32_code(size, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(height) = &options_config.line_height {
                write!(
                    code,
                    ".text_line_height(::iced::widget::text::LineHeight::Relative({}))",
                    clamped_f32_code(height, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(shaping) = options_config.shaping {
                write!(
                    code,
                    ".text_shaping(::iced::widget::text::Shaping::{})",
                    text_shaping_code(shaping)
                )
                .unwrap();
            }
            if let Some(font) = &options_config.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(handle) = &options_config.handle {
                write!(
                    code,
                    ".handle({})",
                    pick_list_handle_code(handle, env, document)?
                )
                .unwrap();
            }
            if let Some(route) = &options_config.open {
                write!(
                    code,
                    ".on_open({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options_config.close {
                write!(
                    code,
                    ".on_close({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            code.push_str(&pick_list_style_code(options_config, env, document)?);
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "pick-list", span, scope, env, document)?;
            let accessibility_label = options_config
                .placeholder
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "\"Select\"".to_owned());
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __pick_selected = {selected}; let __pick = {code} }}; ::ui_lang_runtime::accessible(__pick, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::ComboBox).logical_id(__a11y_key.clone()).label({accessibility_label}).value_maybe(__pick_selected.map(|__value| __value.to_string())).into() }}"
            ))
        }
        ViewNode::ComboBox {
            state,
            id,
            selected,
            placeholder,
            options,
            route,
            span,
            ..
        } => {
            let state = env.get(state).ok_or_else(|| {
                Error::new("E150", span, format!("unknown combo state `{state}`"))
            })?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let mut code = format!(
                "{{ let __combo_option_count = {}.options().len(); ::iced::widget::combo_box(&{}, {}, __combo_selection.as_ref(), {callback})",
                state.code,
                state.code,
                rust_string(placeholder)
            );
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.menu_height {
                write!(
                    code,
                    ".menu_height({})",
                    length_code(height, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    code,
                    ".padding(::ui_lang_runtime::bounded_table_metric({}, __combo_option_count))",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options.text_size {
                write!(
                    code,
                    ".size({})",
                    clamped_f32_code(size, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(height) = &options.line_height {
                write!(
                    code,
                    ".line_height(::iced::widget::text::LineHeight::Relative({}))",
                    clamped_f32_code(height, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(shaping) = options.shaping {
                write!(
                    code,
                    ".text_shaping(::iced::widget::text::Shaping::{})",
                    text_shaping_code(shaping)
                )
                .unwrap();
            }
            if let Some(font) = &options.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(icon) = &options.icon {
                write!(
                    code,
                    ".icon({})",
                    text_input_icon_code(icon, env, document)?
                )
                .unwrap();
            }
            if let Some(route) = &options.input {
                let callback =
                    route_callback_code(route, "__value", "__value", env, document, message)?;
                write!(code, ".on_input({callback})").unwrap();
            }
            if let Some(route) = &options.hover {
                let callback =
                    route_callback_code(route, "__value", "__value", env, document, message)?;
                write!(code, ".on_option_hovered({callback})").unwrap();
            }
            if let Some(route) = &options.open {
                write!(
                    code,
                    ".on_open({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.close {
                write!(
                    code,
                    ".on_close({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            code.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                None,
                env,
                document,
                "input_style",
                "text_input",
            )?);
            code.push_str(&menu_style_code(
                options.menu_style.as_deref(),
                options.custom_menu_style.as_ref(),
                env,
                document,
            )?);
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "combo-box", span, scope, env, document)?;
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __combo_selection = {selected}; let __combo = {code} }}; ::ui_lang_runtime::accessible(__combo, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::ComboBox).logical_id(__a11y_key.clone()).label({}).value_maybe(__combo_selection).into() }}",
                rust_string(placeholder)
            ))
        }
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}
