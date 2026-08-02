use super::*;

pub(in crate::codegen) fn render_boolean_control(
    control: &ResolvedBooleanControl,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let label = resolved_expr_use_code(program, control.label, env, ValueMode::Owned)?;
    let checked = resolved_expr_use_code(program, control.checked, env, ValueMode::Owned)?;
    let rendered = match (&control.kind, &control.style) {
        (ResolvedBooleanKind::Checkbox, ResolvedBooleanStyle::Checkbox(style)) => {
            let disabled = resolved_boolean_disabled(control, program, env)?;
            let callback = resolved_interaction_route_callback_code(
                &control.route,
                "__value",
                &["__value"],
                env,
                program,
                message,
            )?;
            let activation = resolved_interaction_route_code(
                &control.route,
                &["__value"],
                env,
                program,
                message,
            )?;
            let accessibility_key =
                resolved_boolean_identity_code(control, identity, "checkbox", scope, program, env)?;
            let (accessibility_label, accessibility_description) =
                resolved_boolean_accessibility(control, "__label.clone()", program, env)?;
            let mut widget = "::iced::widget::checkbox(__checked).label(__label.clone())".into();
            append_resolved_boolean_options(&mut widget, &control.options, false, program, env)?;
            write!(
                widget,
                ".on_toggle_maybe(if __disabled {{ None }} else {{ Some({callback}) }})"
            )
            .unwrap();
            widget.push_str(&resolved_checkbox_style_code(style, program, env)?);
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __label = {label}; let __checked = {checked}; let __disabled = {disabled}; let __activate = {{ let __value = !__checked; {activation} }}; let __checkbox = {widget}; ::ui_lang_runtime::accessible(__checkbox, __a11y_id, ::ui_lang_runtime::Role::CheckBox).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).checked(__checked).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
            ))
        }
        (ResolvedBooleanKind::Toggler, ResolvedBooleanStyle::Toggler(style)) => {
            let disabled = resolved_boolean_disabled(control, program, env)?;
            let callback = resolved_interaction_route_callback_code(
                &control.route,
                "__value",
                &["__value"],
                env,
                program,
                message,
            )?;
            let activation = resolved_interaction_route_code(
                &control.route,
                &["!__checked"],
                env,
                program,
                message,
            )?;
            let accessibility_key =
                resolved_boolean_identity_code(control, identity, "toggler", scope, program, env)?;
            let (accessibility_label, accessibility_description) =
                resolved_boolean_accessibility(control, "__label.clone()", program, env)?;
            let mut widget = "::iced::widget::toggler(__checked).label(__label.clone())".into();
            append_resolved_boolean_options(&mut widget, &control.options, true, program, env)?;
            write!(
                widget,
                ".on_toggle_maybe(if __disabled {{ None }} else {{ Some({callback}) }})"
            )
            .unwrap();
            widget.push_str(&resolved_toggler_style_code(style, program, env)?);
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __label = {label}; let __checked = {checked}; let __disabled = {disabled}; let __activate = {activation}; let __toggler = {widget}; ::ui_lang_runtime::accessible(__toggler, __a11y_id, ::ui_lang_runtime::Role::Switch).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).checked(__checked).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
            ))
        }
        (ResolvedBooleanKind::Radio, ResolvedBooleanStyle::Radio(style)) => {
            let value = control.value.ok_or_else(|| {
                program.invariant_at_origin(control.origin, "normalized radio value disappeared")
            })?;
            let value = resolved_expr_use_code(program, value, env, ValueMode::Owned)?;
            // Iced's native callback provides its internal bool selection marker. Ice radios
            // deliberately discard it and route the declared `value: T` instead.
            let callback = resolved_interaction_route_callback_code(
                &control.route,
                "_",
                &[&value],
                env,
                program,
                message,
            )?;
            let mut widget = format!(
                "::iced::widget::radio({label}, true, if {checked} {{ Some(true) }} else {{ None }}, {callback})"
            );
            append_resolved_boolean_options(&mut widget, &control.options, false, program, env)?;
            widget.push_str(&resolved_radio_style_code(style, program, env)?);
            Ok(format!("{widget}.into()"))
        }
        _ => Err(program.invariant_at_origin(
            control.origin,
            "boolean control kind and style HIR diverged",
        )),
    }?;
    if matches!(
        control.kind,
        ResolvedBooleanKind::Toggler | ResolvedBooleanKind::Radio
    ) {
        identify_resolved_boolean(rendered, control, identity, message, scope, program, env)
    } else {
        Ok(rendered)
    }
}

fn resolved_boolean_identity_code(
    control: &ResolvedBooleanControl,
    identity: Option<&ResolvedViewIdentity>,
    kind: &str,
    scope: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let Some(identity) = identity else {
        let scope = reconciliation_scope(scope, env);
        return Ok(format!(
            "format!(\"{{}}/@{kind}:{}\", {scope})",
            control.source_line
        ));
    };
    if let Some(key) = identity.key {
        Ok(format!(
            "format!(\"{{}}/{}({{}})\", {scope}, {})",
            identity.name,
            resolved_expr_use_code(program, key, env, ValueMode::Borrowed)?
        ))
    } else {
        Ok(format!("format!(\"{{}}/{}\", {scope})", identity.name))
    }
}

fn identify_resolved_boolean(
    rendered: String,
    control: &ResolvedBooleanControl,
    identity: Option<&ResolvedViewIdentity>,
    message: &str,
    scope: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    if identity.is_none() {
        return Ok(rendered);
    }
    let id = resolved_boolean_identity_code(control, identity, "boolean", scope, program, env)?;
    Ok(format!(
        "{{ let __identified: __IceElement<'_, {message}> = {rendered}; let __ice_id = {id}; #[cfg(test)] ::ui_lang_runtime::testing::register_render_source(&__ice_id); ::iced::widget::container(__identified).id(::iced::widget::Id::from(__ice_id)).into() }}"
    ))
}

fn resolved_boolean_disabled(
    control: &ResolvedBooleanControl,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    control
        .disabled
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()
        .map(|value| value.unwrap_or_else(|| "false".into()))
}

fn resolved_boolean_accessibility(
    control: &ResolvedBooleanControl,
    fallback: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(String, String), Error> {
    let label = control
        .options
        .accessibility_label
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| fallback.into());
    let description = control
        .options
        .accessibility_description
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .map(|value| format!(".description({value})"))
        .unwrap_or_default();
    Ok((label, description))
}

fn append_resolved_boolean_options(
    code: &mut String,
    options: &ResolvedBooleanOptions,
    toggler: bool,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    for (value, method) in [(options.size, "size"), (options.spacing, "spacing")] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}(::ui_lang_runtime::bounded_table_metric({}, 1))",
                resolved_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(text_size) = options.text_size {
        write!(
            code,
            ".text_size((({}) as f32).max(f32::EPSILON).min(f32::MAX))",
            resolved_expr_use_code(program, text_size, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(width) = &options.width {
        write!(
            code,
            ".width({})",
            resolved_text_length_code(width, program, env)?
        )
        .unwrap();
    }
    if let Some(line_height) = options.line_height {
        write!(
            code,
            ".text_line_height(::iced::widget::text::LineHeight::Relative((({}) as f32).max(f32::EPSILON).min(f32::MAX)))",
            resolved_expr_use_code(program, line_height, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".text_shaping(::iced::widget::text::Shaping::{})",
            resolved_boolean_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".text_wrapping(::iced::widget::text::Wrapping::{})",
            resolved_boolean_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = &options.font {
        write!(code, ".font({})", resolved_input_font_code(font)).unwrap();
    }
    if toggler {
        if let Some(alignment) = options.alignment {
            write!(
                code,
                ".text_alignment(::iced::widget::text::Alignment::{})",
                resolved_boolean_alignment_code(alignment)
            )
            .unwrap();
        }
    } else if let Some(icon) = &options.icon {
        let size = icon.size.map_or_else(
            || Ok("None".to_owned()),
            |value| {
                Ok::<_, Error>(format!(
                    "Some((({}) as f32).max(f32::EPSILON).min(f32::MAX).into())",
                    resolved_expr_use_code(program, value, env, ValueMode::Owned)?
                ))
            },
        )?;
        let line_height = icon.line_height.map_or_else(
            || Ok("::iced::widget::text::LineHeight::default()".to_owned()),
            |value| {
                Ok::<_, Error>(format!(
                    "::iced::widget::text::LineHeight::Relative((({}) as f32).max(f32::EPSILON).min(f32::MAX))",
                    resolved_expr_use_code(program, value, env, ValueMode::Owned)?
                ))
            },
        )?;
        let shaping = icon.shaping.unwrap_or(ResolvedTextShaping::Auto);
        write!(
            code,
            ".icon(::iced::widget::checkbox::Icon {{ font: ::iced::Font::DEFAULT, code_point: {:?}, size: {size}, line_height: {line_height}, shaping: ::iced::widget::text::Shaping::{} }})",
            icon.code_point,
            resolved_boolean_shaping_code(shaping)
        )
        .unwrap();
    }
    Ok(())
}

fn resolved_checkbox_style_code(
    styles: &ResolvedCheckboxStyleSet,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let custom = resolved_boolean_custom_style(styles.custom.as_ref(), program, env)?;
    let preset = match styles.preset.unwrap_or(ResolvedCheckboxPreset::Primary) {
        ResolvedCheckboxPreset::Primary => "primary",
        ResolvedCheckboxPreset::Secondary => "secondary",
        ResolvedCheckboxPreset::Success => "success",
        ResolvedCheckboxPreset::Danger => "danger",
    };
    let overrides = [
        ("Active", true, &styles.active_checked),
        ("Active", false, &styles.active_unchecked),
        ("Hovered", true, &styles.hovered_checked),
        ("Hovered", false, &styles.hovered_unchecked),
        ("Disabled", true, &styles.disabled_checked),
        ("Disabled", false, &styles.disabled_unchecked),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(if let Some(custom) = custom {
            format!(".style(move |__theme, __status| {custom})")
        } else if matches!(styles.preset, Some(ResolvedCheckboxPreset::Primary)) {
            String::new()
        } else {
            format!(".style(::iced::widget::checkbox::{preset})")
        });
    }
    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::checkbox::{preset}(__theme, __status)"));
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    let mut rendered = 0usize;
    for (status, checked, style) in overrides {
        let inherited = match (status, checked) {
            ("Active", _) => None,
            (_, true) => styles.active_checked.as_ref(),
            (_, false) => styles.active_unchecked.as_ref(),
        };
        if inherited.is_none() && style.is_none() {
            continue;
        }
        rendered += 1;
        write!(
            code,
            " ::iced::widget::checkbox::Status::{status} {{ is_checked: {checked} }} => {{"
        )
        .unwrap();
        if let Some(inherited) = inherited {
            append_resolved_checkbox_status(&mut code, inherited, program, env)?;
        }
        if let Some(style) = style {
            append_resolved_checkbox_status(&mut code, style, program, env)?;
        }
        code.push_str(" }");
    }
    if rendered < overrides.len() {
        code.push_str(" _ => {}");
    }
    code.push_str(" } __style })");
    Ok(code)
}

fn append_resolved_checkbox_status(
    code: &mut String,
    style: &ResolvedCheckboxStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(background) = &style.background {
        write!(
            code,
            " __style.background = {};",
            resolved_text_background_code(&background.value, program, env)?
        )
        .unwrap();
    }
    append_resolved_boolean_color(code, "__style.icon_color", style.icon_color.as_ref());
    if let Some(color) = &style.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(&color.value)
        )
        .unwrap();
    }
    append_resolved_boolean_color(code, "__style.border.color", style.border_color.as_ref());
    append_resolved_boolean_metric(
        code,
        "__style.border.width",
        style.border_width,
        program,
        env,
    )?;
    if let Some(radius) = resolved_text_radius_code(&style.radius, program, env)? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    Ok(())
}

fn resolved_toggler_style_code(
    styles: &ResolvedTogglerStyleSet,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let custom = resolved_boolean_custom_style(styles.custom.as_ref(), program, env)?;
    let overrides = [
        ("Active", true, &styles.active_checked),
        ("Active", false, &styles.active_unchecked),
        ("Hovered", true, &styles.hovered_checked),
        ("Hovered", false, &styles.hovered_unchecked),
        ("Disabled", true, &styles.disabled_checked),
        ("Disabled", false, &styles.disabled_unchecked),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::toggler::default(__theme, __status)".into());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    let mut rendered = 0usize;
    for (status, checked, style) in overrides {
        let inherited = match (status, checked) {
            ("Active", _) => None,
            (_, true) => styles.active_checked.as_ref(),
            (_, false) => styles.active_unchecked.as_ref(),
        };
        if inherited.is_none() && style.is_none() {
            continue;
        }
        rendered += 1;
        write!(
            code,
            " ::iced::widget::toggler::Status::{status} {{ is_toggled: {checked} }} => {{"
        )
        .unwrap();
        if let Some(inherited) = inherited {
            append_resolved_toggler_status(&mut code, inherited, program, env)?;
        }
        if let Some(style) = style {
            append_resolved_toggler_status(&mut code, style, program, env)?;
        }
        code.push_str(" }");
    }
    if rendered < overrides.len() {
        code.push_str(" _ => {}");
    }
    code.push_str(" } __style })");
    Ok(code)
}

fn append_resolved_toggler_status(
    code: &mut String,
    style: &ResolvedTogglerStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(background) = &style.background {
        write!(
            code,
            " __style.background = {};",
            resolved_text_background_code(&background.value, program, env)?
        )
        .unwrap();
    }
    append_resolved_boolean_color(
        code,
        "__style.background_border_color",
        style.background_border_color.as_ref(),
    );
    append_resolved_boolean_metric(
        code,
        "__style.background_border_width",
        style.background_border_width,
        program,
        env,
    )?;
    if let Some(foreground) = &style.foreground {
        write!(
            code,
            " __style.foreground = {};",
            resolved_text_background_code(&foreground.value, program, env)?
        )
        .unwrap();
    }
    append_resolved_boolean_color(
        code,
        "__style.foreground_border_color",
        style.foreground_border_color.as_ref(),
    );
    append_resolved_boolean_metric(
        code,
        "__style.foreground_border_width",
        style.foreground_border_width,
        program,
        env,
    )?;
    if let Some(color) = &style.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(&color.value)
        )
        .unwrap();
    }
    if let Some(radius) = resolved_text_radius_code(&style.radius, program, env)? {
        write!(
            code,
            " __style.border_radius = ::std::option::Option::Some({radius});"
        )
        .unwrap();
    }
    if let Some(ratio) = style.padding_ratio {
        write!(
            code,
            " __style.padding_ratio = (({}) as f32).max(0.0).min(0.5);",
            resolved_expr_use_code(program, ratio, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(())
}

fn resolved_radio_style_code(
    styles: &ResolvedRadioStyleSet,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let custom = resolved_boolean_custom_style(styles.custom.as_ref(), program, env)?;
    let overrides = [
        ("Active", true, &styles.active_selected),
        ("Active", false, &styles.active_unselected),
        ("Hovered", true, &styles.hovered_selected),
        ("Hovered", false, &styles.hovered_unselected),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base = custom.unwrap_or_else(|| "::iced::widget::radio::default(__theme, __status)".into());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    let mut rendered = 0usize;
    for (status, selected, style) in overrides {
        let inherited = match (status, selected) {
            ("Active", _) => None,
            (_, true) => styles.active_selected.as_ref(),
            (_, false) => styles.active_unselected.as_ref(),
        };
        if inherited.is_none() && style.is_none() {
            continue;
        }
        rendered += 1;
        write!(
            code,
            " ::iced::widget::radio::Status::{status} {{ is_selected: {selected} }} => {{"
        )
        .unwrap();
        if let Some(inherited) = inherited {
            append_resolved_radio_status(&mut code, inherited, program, env)?;
        }
        if let Some(style) = style {
            append_resolved_radio_status(&mut code, style, program, env)?;
        }
        code.push_str(" }");
    }
    if rendered < overrides.len() {
        code.push_str(" _ => {}");
    }
    code.push_str(" } __style })");
    Ok(code)
}

fn append_resolved_radio_status(
    code: &mut String,
    style: &ResolvedRadioStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(background) = &style.background {
        write!(
            code,
            " __style.background = {};",
            resolved_text_background_code(&background.value, program, env)?
        )
        .unwrap();
    }
    append_resolved_boolean_color(code, "__style.dot_color", style.dot_color.as_ref());
    append_resolved_boolean_color(code, "__style.border_color", style.border_color.as_ref());
    append_resolved_boolean_metric(
        code,
        "__style.border_width",
        style.border_width,
        program,
        env,
    )?;
    if let Some(color) = &style.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(&color.value)
        )
        .unwrap();
    }
    Ok(())
}

fn resolved_boolean_custom_style(
    style: Option<&ResolvedBooleanCustomStyle>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    style
        .map(|style| {
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
        })
        .transpose()
}

fn append_resolved_boolean_color(
    code: &mut String,
    field: &str,
    color: Option<&ResolvedBooleanColor>,
) {
    if let Some(color) = color {
        write!(code, " {field} = {};", resolved_theme_color(&color.value)).unwrap();
    }
}

fn append_resolved_boolean_metric(
    code: &mut String,
    field: &str,
    value: Option<ResolvedExpressionId>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(value) = value {
        write!(
            code,
            " {field} = {} as f32;",
            resolved_expr_use_code(program, value, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(())
}

fn resolved_boolean_shaping_code(value: ResolvedTextShaping) -> &'static str {
    match value {
        ResolvedTextShaping::Auto => "Auto",
        ResolvedTextShaping::Basic => "Basic",
        ResolvedTextShaping::Advanced => "Advanced",
    }
}

fn resolved_boolean_wrapping_code(value: ResolvedTextWrapping) -> &'static str {
    match value {
        ResolvedTextWrapping::None => "None",
        ResolvedTextWrapping::Word => "Word",
        ResolvedTextWrapping::Glyph => "Glyph",
        ResolvedTextWrapping::WordOrGlyph => "WordOrGlyph",
    }
}

fn resolved_boolean_alignment_code(value: ResolvedTextAlignment) -> &'static str {
    match value {
        ResolvedTextAlignment::Default => "Default",
        ResolvedTextAlignment::Left => "Left",
        ResolvedTextAlignment::Center => "Center",
        ResolvedTextAlignment::Right => "Right",
        ResolvedTextAlignment::Justified => "Justified",
    }
}
