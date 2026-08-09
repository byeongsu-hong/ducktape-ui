use super::*;

pub(in crate::codegen) fn render_pick_list(
    pick: &ResolvedPickList,
    identity: Option<&ResolvedViewIdentity>,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    let options = resolved_expr_use_code(program, pick.options, env, ValueMode::Owned)?;
    let selected = resolved_expr_use_code(program, pick.selected, env, ValueMode::Owned)?;
    let callback = resolved_interaction_route_callback_code(
        &pick.selection,
        "__value",
        &["__value"],
        env,
        program,
        message,
    )?;
    let mut widget =
        format!("::iced::widget::pick_list(__pick_options, __pick_selected.clone(), {callback})");
    if let Some(placeholder) = pick.placeholder {
        write!(
            widget,
            ".placeholder({})",
            resolved_expr_use_code(program, placeholder, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    append_selection_length(&mut widget, "width", pick.width.as_ref(), program, env)?;
    append_selection_length(
        &mut widget,
        "menu_height",
        pick.menu_height.as_ref(),
        program,
        env,
    )?;
    if let Some(padding) = pick.padding {
        write!(
            widget,
            ".padding(::ui_lang_runtime::bounded_table_metric({}, __pick_option_count))",
            resolved_expr_use_code(program, padding, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(size) = pick.text_size {
        write!(
            widget,
            ".text_size((({}) as f32).max(f32::EPSILON).min(f32::MAX))",
            resolved_expr_use_code(program, size, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(line_height) = pick.line_height {
        write!(
            widget,
            ".text_line_height(::iced::widget::text::LineHeight::Relative((({}) as f32).max(f32::EPSILON).min(f32::MAX)))",
            resolved_expr_use_code(program, line_height, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(shaping) = pick.shaping {
        write!(
            widget,
            ".text_shaping(::iced::widget::text::Shaping::{})",
            resolved_selection_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(font) = &pick.font {
        write!(widget, ".font({})", resolved_input_font_code(font)).unwrap();
    }
    if let Some(handle) = &pick.handle {
        write!(
            widget,
            ".handle({})",
            resolved_pick_handle_code(handle, program, env)?
        )
        .unwrap();
    }
    if let Some(route) = &pick.open {
        write!(
            widget,
            ".on_open({})",
            resolved_interaction_route_code(route, &[], env, program, message)?
        )
        .unwrap();
    }
    if let Some(route) = &pick.close {
        write!(
            widget,
            ".on_close({})",
            resolved_interaction_route_code(route, &[], env, program, message)?
        )
        .unwrap();
    }
    widget.push_str(&resolved_pick_style_code(pick, program, env)?);
    let accessibility_key =
        resolved_accessibility_key_code(identity, "pick-list", pick.origin, scope, env, document)?;
    let accessibility_label = pick
        .placeholder
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "\"Select\"".to_owned());
    Ok(format!(
        "{{ let __a11y_key = {accessibility_key}; let __pick_selected = {selected}; let __pick_options = {options}; let __pick_option_count = __pick_options.len(); let __pick = {widget}; ::ui_lang_runtime::accessible(__pick, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::ComboBox).logical_id(__a11y_key.clone()).label({accessibility_label}).value_maybe(__pick_selected.map(|__value| __value.to_string())).into() }}"
    ))
}

pub(in crate::codegen) fn render_combo_box(
    combo: &ResolvedComboBox,
    identity: Option<&ResolvedViewIdentity>,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    let state = resolved_combo_state(combo, env, program)?;
    let selected = resolved_expr_use_code(program, combo.selected, env, ValueMode::Owned)?;
    let callback = resolved_interaction_route_callback_code(
        &combo.selection,
        "__value",
        &["__value"],
        env,
        program,
        message,
    )?;
    let mut widget = format!(
        "::iced::widget::combo_box(&{}, {}, __combo_selection.as_ref(), {callback})",
        state.code,
        rust_string(&combo.placeholder),
    );
    append_selection_length(&mut widget, "width", combo.width.as_ref(), program, env)?;
    append_selection_length(
        &mut widget,
        "menu_height",
        combo.menu_height.as_ref(),
        program,
        env,
    )?;
    if let Some(padding) = combo.padding {
        write!(
            widget,
            ".padding(::ui_lang_runtime::bounded_table_metric({}, __combo_option_count))",
            resolved_expr_use_code(program, padding, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(size) = combo.text_size {
        write!(
            widget,
            ".size((({}) as f32).max(f32::EPSILON).min(f32::MAX))",
            resolved_expr_use_code(program, size, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(line_height) = combo.line_height {
        write!(
            widget,
            ".line_height(::iced::widget::text::LineHeight::Relative((({}) as f32).max(f32::EPSILON).min(f32::MAX)))",
            resolved_expr_use_code(program, line_height, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(shaping) = combo.shaping {
        write!(
            widget,
            ".text_shaping(::iced::widget::text::Shaping::{})",
            resolved_selection_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(font) = &combo.font {
        write!(widget, ".font({})", resolved_input_font_code(font)).unwrap();
    }
    if let Some(icon) = &combo.icon {
        write!(
            widget,
            ".icon({})",
            resolved_input_icon_code(icon, program, env)?
        )
        .unwrap();
    }
    if let Some(route) = &combo.input {
        let callback = resolved_interaction_route_callback_code(
            route,
            "__value",
            &["__value"],
            env,
            program,
            message,
        )?;
        write!(widget, ".on_input({callback})").unwrap();
    }
    if let Some(route) = &combo.hover {
        let callback = resolved_interaction_route_callback_code(
            route,
            "__value",
            &["__value"],
            env,
            program,
            message,
        )?;
        write!(widget, ".on_option_hovered({callback})").unwrap();
    }
    if let Some(route) = &combo.open {
        write!(
            widget,
            ".on_open({})",
            resolved_interaction_route_code(route, &[], env, program, message)?
        )
        .unwrap();
    }
    if let Some(route) = &combo.close {
        write!(
            widget,
            ".on_close({})",
            resolved_interaction_route_code(route, &[], env, program, message)?
        )
        .unwrap();
    }
    widget.push_str(&resolved_combo_style_code(combo, program, env)?);
    widget.push_str(&resolved_menu_style_code(&combo.menu, program, env)?);
    let accessibility_key =
        resolved_accessibility_key_code(identity, "combo-box", combo.origin, scope, env, document)?;
    Ok(format!(
        "{{ let __a11y_key = {accessibility_key}; let __combo_selection = {selected}; let __combo_option_count = {}.options().len(); let __combo = {widget}; ::ui_lang_runtime::accessible(__combo, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::ComboBox).logical_id(__a11y_key.clone()).label({}).value_maybe(__combo_selection).into() }}",
        state.code,
        rust_string(&combo.placeholder),
    ))
}

fn resolved_combo_state<'a>(
    combo: &ResolvedComboBox,
    env: &'a dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<&'a Binding, Error> {
    let state = env.get(&combo.state.name).ok_or_else(|| {
        program.invariant_at_origin(combo.origin, "combo state is absent from its render scope")
    })?;
    if state.owner != Some(BindingOwner::Value(combo.state.id))
        || state.ty != Type::Combo(Box::new(combo.state.option_type.clone()))
    {
        return Err(program.invariant_at_origin(
            combo.origin,
            "combo render binding does not match its normalized state ID and type",
        ));
    }
    Ok(state)
}

fn append_selection_length(
    code: &mut String,
    method: &str,
    length: Option<&ResolvedContainerLength>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(length) = length {
        write!(
            code,
            ".{method}({})",
            resolved_text_length_code(length, program, env)?
        )
        .unwrap();
    }
    Ok(())
}

fn resolved_selection_shaping_code(shaping: ResolvedSelectionShaping) -> &'static str {
    match shaping {
        ResolvedSelectionShaping::Auto => "Auto",
        ResolvedSelectionShaping::Basic => "Basic",
        ResolvedSelectionShaping::Advanced => "Advanced",
    }
}

fn resolved_pick_handle_code(
    handle: &ResolvedPickListHandle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match handle {
        ResolvedPickListHandle::Arrow { size } => {
            let size = size.map_or_else(
                || Ok("::std::option::Option::None".to_owned()),
                |value| {
                    Ok::<_, Error>(format!(
                        "::std::option::Option::Some((({}) as f32).max(f32::EPSILON).min(f32::MAX).into())",
                        resolved_expr_use_code(program, value, env, ValueMode::Owned)?
                    ))
                },
            )?;
            format!("::iced::widget::pick_list::Handle::Arrow {{ size: {size} }}")
        }
        ResolvedPickListHandle::Static(icon) => format!(
            "::iced::widget::pick_list::Handle::Static({})",
            resolved_pick_icon_code(icon, program, env)?
        ),
        ResolvedPickListHandle::Dynamic { closed, open } => format!(
            "::iced::widget::pick_list::Handle::Dynamic {{ closed: {}, open: {} }}",
            resolved_pick_icon_code(closed, program, env)?,
            resolved_pick_icon_code(open, program, env)?,
        ),
        ResolvedPickListHandle::None => "::iced::widget::pick_list::Handle::None".to_owned(),
    })
}

fn resolved_pick_icon_code(
    icon: &ResolvedPickListIcon,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let font = icon
        .font
        .as_ref()
        .map(resolved_input_font_code)
        .unwrap_or_else(|| "::iced::Font::DEFAULT".into());
    let size = icon.size.map_or_else(
        || Ok("::std::option::Option::None".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some((({}) as f32).max(f32::EPSILON).min(f32::MAX).into())",
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
    let shaping = icon.shaping.map_or_else(
        || "::iced::widget::text::Shaping::default()".to_owned(),
        |shaping| {
            format!(
                "::iced::widget::text::Shaping::{}",
                resolved_selection_shaping_code(shaping)
            )
        },
    );
    Ok(format!(
        "::iced::widget::pick_list::Icon {{ font: {font}, code_point: {:?}, size: {size}, line_height: {line_height}, shaping: {shaping} }}",
        icon.code_point
    ))
}

fn resolved_pick_style_code(
    pick: &ResolvedPickList,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let custom = resolved_status_custom_call(pick.custom_style.as_ref(), program, env)?;
    let has_overrides = [
        &pick.styles.active,
        &pick.styles.hovered,
        &pick.styles.opened,
        &pick.styles.opened_hovered,
    ]
    .into_iter()
    .any(Option::is_some);
    let mut code = if has_overrides {
        let base = custom
            .unwrap_or_else(|| "::iced::widget::pick_list::default(__theme, __status)".into());
        format!(".style(move |__theme, __status| {{ let mut __style = {base};")
    } else if let Some(custom) = custom {
        format!(".style(move |__theme, __status| {custom})")
    } else {
        String::new()
    };
    if has_overrides {
        if let Some(active) = &pick.styles.active {
            append_resolved_pick_status(&mut code, active, program, env)?;
        }
        let overrides = [
            ("Hovered", None, pick.styles.hovered.as_ref()),
            (
                "Opened { is_hovered: false }",
                None,
                pick.styles.opened.as_ref(),
            ),
            (
                "Opened { is_hovered: true }",
                pick.styles.opened.as_ref(),
                pick.styles.opened_hovered.as_ref(),
            ),
        ];
        if overrides
            .iter()
            .any(|(_, inherited, status)| inherited.is_some() || status.is_some())
        {
            code.push_str(" match __status {");
            for (variant, inherited, status) in overrides {
                if inherited.is_none() && status.is_none() {
                    continue;
                }
                write!(code, " ::iced::widget::pick_list::Status::{variant} => {{").unwrap();
                if let Some(inherited) = inherited {
                    append_resolved_pick_status(&mut code, inherited, program, env)?;
                }
                if let Some(status) = status {
                    append_resolved_pick_status(&mut code, status, program, env)?;
                }
                code.push_str(" }");
            }
            code.push_str(" _ => {} }");
        }
        code.push_str(" __style })");
    }
    code.push_str(&resolved_menu_style_code(&pick.menu, program, env)?);
    Ok(code)
}

fn resolved_combo_style_code(
    combo: &ResolvedComboBox,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let custom = resolved_status_custom_call(combo.custom_style.as_ref(), program, env)?;
    let has_overrides = [
        &combo.styles.active,
        &combo.styles.hovered,
        &combo.styles.focused,
        &combo.styles.focused_hovered,
        &combo.styles.disabled,
    ]
    .into_iter()
    .any(Option::is_some);
    if !has_overrides {
        return Ok(custom
            .map(|custom| format!(".input_style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::text_input::default(__theme, __status)".into());
    let mut code = format!(".input_style(move |__theme, __status| {{ let mut __style = {base};");
    if let Some(active) = &combo.styles.active {
        append_resolved_input_status(&mut code, active, program, env)?;
    }
    let overrides = [
        ("Hovered", None, combo.styles.hovered.as_ref()),
        (
            "Focused { is_hovered: false }",
            None,
            combo.styles.focused.as_ref(),
        ),
        (
            "Focused { is_hovered: true }",
            combo.styles.focused.as_ref(),
            combo.styles.focused_hovered.as_ref(),
        ),
        ("Disabled", None, combo.styles.disabled.as_ref()),
    ];
    if overrides
        .iter()
        .any(|(_, inherited, status)| inherited.is_some() || status.is_some())
    {
        code.push_str(" match __status {");
        for (variant, inherited, status) in overrides {
            if inherited.is_none() && status.is_none() {
                continue;
            }
            write!(code, " ::iced::widget::text_input::Status::{variant} => {{").unwrap();
            if let Some(inherited) = inherited {
                append_resolved_input_status(&mut code, inherited, program, env)?;
            }
            if let Some(status) = status {
                append_resolved_input_status(&mut code, status, program, env)?;
            }
            code.push_str(" }");
        }
        code.push_str(" _ => {} }");
    }
    code.push_str(" __style })");
    Ok(code)
}

fn resolved_status_custom_call(
    custom: Option<&ResolvedSelectionCustomStyle>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    custom
        .map(|custom| {
            let arguments = custom
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
                program.extern_function(custom.function).rust_path
            ))
        })
        .transpose()
}

fn append_resolved_pick_status(
    code: &mut String,
    status: &ResolvedPickListStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    append_resolved_selection_surface(code, &status.surface, false, program, env)?;
    if let Some(color) = &status.placeholder_color {
        write!(
            code,
            " __style.placeholder_color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(color) = &status.handle_color {
        write!(
            code,
            " __style.handle_color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    Ok(())
}

fn resolved_menu_style_code(
    menu: &ResolvedMenuStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let custom = menu
        .custom
        .as_ref()
        .map(|custom| {
            let arguments = custom
                .arguments
                .iter()
                .map(|argument| resolved_expr_use_code(program, *argument, env, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            let suffix = arguments
                .into_iter()
                .map(|argument| format!(", {argument}"))
                .collect::<String>();
            Ok::<_, Error>(format!(
                "{}(__theme{suffix})",
                program.extern_function(custom.function).rust_path
            ))
        })
        .transpose()?;
    let Some(surface) = &menu.surface else {
        return Ok(custom
            .map(|custom| format!(".menu_style(move |__theme| {custom})"))
            .unwrap_or_default());
    };
    let base = custom.unwrap_or_else(|| "::iced::overlay::menu::default(__theme)".into());
    let mut code = format!(".menu_style(move |__theme| {{ let mut __style = {base};");
    append_resolved_selection_surface(&mut code, surface, true, program, env)?;
    if let Some(color) = &menu.selected_text_color {
        write!(
            code,
            " __style.selected_text_color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(background) = &menu.selected_background {
        write!(
            code,
            " __style.selected_background = {};",
            resolved_text_background_code(background, program, env)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(code)
}

fn append_resolved_selection_surface(
    code: &mut String,
    surface: &ResolvedContainerSurface,
    shadow: bool,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(background) = &surface.background {
        write!(
            code,
            " __style.background = {};",
            resolved_text_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(color) = &surface.text_color {
        write!(
            code,
            " __style.text_color = {};",
            resolved_theme_color(color)
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
    if let Some(radius) = resolved_text_radius_code(&surface.radius, program, env)? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if shadow {
        if let Some(color) = &surface.shadow_color {
            write!(
                code,
                " __style.shadow.color = {};",
                resolved_theme_color(color)
            )
            .unwrap();
        }
        for (value, field) in [
            (surface.shadow_x, "__style.shadow.offset.x"),
            (surface.shadow_y, "__style.shadow.offset.y"),
            (surface.shadow_blur, "__style.shadow.blur_radius"),
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
    }
    Ok(())
}
