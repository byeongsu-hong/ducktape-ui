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
        ViewNode::Button { content, id, .. } => render_button(
            document.program().resolved_button_for(node)?,
            id,
            content.as_deref(),
            document,
            message,
            env,
            scope,
            slot,
        ),
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
        ViewNode::Slider { id, .. } => render_slider(
            document.program().resolved_slider_for(node)?,
            id.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        ViewNode::Progress { id, .. } => render_progress(
            document.program().resolved_progress_for(node)?,
            id.as_ref(),
            document,
            env,
            scope,
        ),
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
