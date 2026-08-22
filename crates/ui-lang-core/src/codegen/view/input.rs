use super::*;

pub(in crate::codegen) fn render_input(
    input: &ResolvedInput,
    identity: Option<&ResolvedViewIdentity>,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    // A secret input reads and writes the runtime store instead of app state.
    // `value_code` is what the widget draws from; there is no second copy, and
    // no expression anywhere else in the program can reach it.
    let (value_code, binding_constructor) = if let Some(slot) = input.binding.secret() {
        // The store's receiver comes from the render scope rather than a
        // hard-coded `self`, the same way ordinary state's does.
        let store = env.get(slot).ok_or_else(|| {
            program
                .invariant_at_origin(input.origin, "secret store is absent from its render scope")
        })?;
        (
            format!("{}.text({})", store.code, rust_string(slot)),
            format!(
                "{message}::{}(::std::string::String::from({}), __text)",
                SECRET_TYPED_VARIANT,
                rust_string(slot)
            ),
        )
    } else {
        let state = resolved_input_state(input, env, program)?;
        let constructor = match &state.state {
            Some(StateBinding::App(name)) => {
                let variant = binding_variant(name);
                format!("{message}::{variant} as fn(::std::string::String) -> {message}")
            }
            Some(StateBinding::Component {
                component,
                name,
                scope,
            }) => {
                let variant = component_binding_variant(component, name);
                format!(
                    "{{ let __scope = ({}).clone(); move |__value| {message}::{variant}(__scope.clone(), __value) }}",
                    borrowed_scope(scope)
                )
            }
            None => {
                return Err(program.invariant_at_origin(
                    input.origin,
                    "normalized input binding is absent from the state environment",
                ));
            }
        };
        (state.code.clone(), constructor)
    };
    let secret_slot = input.binding.secret();
    let binding_constructor = if secret_slot.is_some() {
        format!("move |__text| {binding_constructor}")
    } else {
        binding_constructor
    };
    let constructor = input
        .change
        .as_ref()
        .map(|route| {
            resolved_interaction_route_callback_code(
                route,
                "__value",
                &["__value"],
                env,
                program,
                message,
            )
        })
        .transpose()?
        .unwrap_or(binding_constructor);
    let accessibility_key =
        resolved_accessibility_key_code(identity, "input", input.origin, scope, env, document)?;
    let accessibility_label = input
        .accessibility_label
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| rust_string(&input.label));
    let accessibility_description = input
        .accessibility_description
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .map(|value| format!(".description({value})"))
        .unwrap_or_default();
    let disabled = input
        .disabled
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    // A secret input is masked permanently; the checker refuses `secure=` on
    // one, so there is no expression that could ever unmask it.
    let secure = if secret_slot.is_some() {
        "true".to_owned()
    } else {
        input
            .secure
            .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
            .transpose()?
            .unwrap_or_else(|| "false".into())
    };

    let mut widget = format!(
        "::iced::widget::text_input({}, &{value_code})",
        rust_string(&input.hint),
    );
    widget.push_str(".id(::iced::widget::Id::from(__a11y_key.clone()))");
    if let Some(padding) = input.utility_style.padding_code() {
        write!(widget, ".padding({padding})").unwrap();
    }
    if input.utility_style.width_fill {
        widget.push_str(".width(::iced::Fill)");
    }
    widget.push_str(".secure(__secure)");
    if let Some(width) = &input.width {
        write!(
            widget,
            ".width({})",
            resolved_length_code(width, program, env)?
        )
        .unwrap();
    }
    if let Some(padding) = input.padding {
        write!(
            widget,
            ".padding({} as f32)",
            resolved_expr_use_code(program, padding, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(size) = input.text_size {
        write!(
            widget,
            ".size({})",
            clamped_f32_code(size, "f32::EPSILON", "f32::MAX", program, env)?
        )
        .unwrap();
    }
    if let Some(line_height) = input.line_height {
        write!(
            widget,
            ".line_height(::iced::widget::text::LineHeight::Relative({}))",
            clamped_f32_code(line_height, "f32::EPSILON", "f32::MAX", program, env)?
        )
        .unwrap();
    }
    if let Some(align) = input.align {
        let align = match align {
            ResolvedInputAlignment::Left => "Left",
            ResolvedInputAlignment::Center => "Center",
            ResolvedInputAlignment::Right => "Right",
        };
        write!(widget, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
    }
    if let Some(font) = &input.font {
        write!(widget, ".font({})", resolved_input_font_code(font)).unwrap();
    }
    if let Some(icon) = &input.icon {
        write!(
            widget,
            ".icon({})",
            resolved_input_icon_code(icon, program, env)?
        )
        .unwrap();
    }
    write!(
        widget,
        ".on_input_maybe(if __disabled {{ None }} else {{ Some({constructor}) }})"
    )
    .unwrap();
    if let Some(route) = &input.submit {
        let submit = resolved_interaction_route_code(route, &[], env, program, message)?;
        write!(
            widget,
            ".on_submit_maybe(if __disabled {{ None }} else {{ Some({submit}) }})"
        )
        .unwrap();
    }
    if let Some(route) = &input.paste {
        let paste = resolved_interaction_route_callback_code(
            route,
            "__value",
            &["__value"],
            env,
            program,
            message,
        )?;
        write!(
            widget,
            ".on_paste_maybe(if __disabled {{ None }} else {{ Some({paste}) }})"
        )
        .unwrap();
    }
    widget.push_str(&resolved_input_style_code(input, program, env)?);
    let a11y_value = if secret_slot.is_some() {
        // Not "the branch is false" — there is no expression here that could
        // produce the text, so no later edit can make one true.
        ".value_maybe(::std::option::Option::None)".to_owned()
    } else {
        format!(".value_maybe((!__secure).then(|| ({value_code}).to_owned()))")
    };
    let view = if input.label.is_empty() {
        "__input.into()".to_owned()
    } else {
        format!(
            "::iced::widget::column![::iced::widget::text({}), __input].spacing(6).into()",
            rust_string(&input.label)
        )
    };
    Ok(format!(
        "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __disabled = {disabled}; let __secure = {secure}; let __role = if __secure {{ ::ui_lang_runtime::Role::PasswordInput }} else {{ ::ui_lang_runtime::Role::TextInput }}; let __input = ::ui_lang_runtime::accessible({widget}, __a11y_id, __role).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}){a11y_value}.disabled(__disabled){accessibility_description}; {view} }}"
    ))
}

fn resolved_input_state<'a>(
    input: &ResolvedInput,
    env: &'a dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<&'a Binding, Error> {
    let ResolvedInputBinding::State(binding) = &input.binding else {
        return Err(
            program.invariant_at_origin(input.origin, "secret input asked for a state binding")
        );
    };
    let state = env.get(binding.name()).ok_or_else(|| {
        program.invariant_at_origin(input.origin, "input state is absent from its render scope")
    })?;
    if !binding.accepts_type(&state.ty)
        || state.owner != Some(BindingOwner::Value(binding.checked_ref()))
    {
        return Err(program.invariant_at_origin(
            input.origin,
            "input render binding does not match its normalized state ID",
        ));
    }
    match (binding, &state.state) {
        (WritableStateRef::App { name, .. }, Some(StateBinding::App(actual))) if name == actual => {
        }
        (WritableStateRef::ComponentParam { .. }, Some(_)) => {}
        (
            WritableStateRef::ComponentState { id, name },
            Some(StateBinding::Component {
                component,
                name: actual,
                ..
            }),
        ) if component == &program.component(id.component).name && name == actual => {}
        _ => {
            return Err(program.invariant_at_origin(
                input.origin,
                "input render state capability diverged from normalized binding",
            ));
        }
    }
    Ok(state)
}

pub(super) fn resolved_input_font_code(font: &ResolvedTextFont) -> String {
    match font {
        ResolvedTextFont::Default => "::iced::Font::DEFAULT".into(),
        ResolvedTextFont::Monospace => "::iced::Font::MONOSPACE".into(),
        ResolvedTextFont::Named(font) => resolved_default_font_code(font),
    }
}

pub(super) fn resolved_input_icon_code(
    icon: &ResolvedInputIcon,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let font = icon
        .font
        .as_ref()
        .map(resolved_input_font_code)
        .unwrap_or_else(|| "::iced::Font::DEFAULT".into());
    let size = icon.size.map_or_else(
        || Ok("::std::option::Option::None".into()),
        |size| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some({}.into())",
                clamped_f32_code(size, "f32::EPSILON", "f32::MAX", program, env)?
            ))
        },
    )?;
    let spacing = icon
        .spacing
        .map(|spacing| resolved_expr_use_code(program, spacing, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "0.0".into());
    let side = match icon.side {
        ResolvedInputIconSide::Left => "Left",
        ResolvedInputIconSide::Right => "Right",
    };
    Ok(format!(
        "::iced::widget::text_input::Icon {{ font: {font}, code_point: {:?}, size: {size}, spacing: {spacing} as f32, side: ::iced::widget::text_input::Side::{side} }}",
        icon.code_point
    ))
}

fn resolved_input_style_code(
    input: &ResolvedInput,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let custom = input
        .custom_style
        .as_ref()
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
            Ok::<_, Error>(format!(
                "{}(__theme, __status{suffix})",
                program.extern_function(style.function).rust_path
            ))
        })
        .transpose()?;
    let utilities = &input.utility_style;
    let has_utilities = utilities.background.is_some()
        || utilities.border_color.is_some()
        || utilities.border_width != 0
        || utilities.radius != 0
        || utilities.focus_border_color.is_some();
    let has_overrides = [
        &input.styles.active,
        &input.styles.hovered,
        &input.styles.focused,
        &input.styles.focused_hovered,
        &input.styles.disabled,
    ]
    .into_iter()
    .any(Option::is_some);
    if !has_overrides && !has_utilities {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::text_input::default(__theme, __status)".into());
    let mut code = format!(".style(move |__theme, __status| {{ let mut __style = {base};");
    if let Some(background) = &utilities.background {
        write!(
            code,
            " __style.background = {}.into();",
            resolved_theme_color(background)
        )
        .unwrap();
    }
    if let Some(border) = &utilities.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(border)
        )
        .unwrap();
    }
    if utilities.border_width != 0 {
        write!(
            code,
            " __style.border.width = {}.0;",
            utilities.border_width
        )
        .unwrap();
    }
    if utilities.radius != 0 {
        write!(
            code,
            " __style.border.radius = {}.0.into();",
            utilities.radius
        )
        .unwrap();
    }
    if let Some(active) = &input.styles.active {
        append_resolved_input_status(&mut code, active, program, env)?;
    }
    // THE FOCUS PASS RUNS AFTER THE `active` BASE. `active` is the base for
    // every status, not just `Status::Active`, so writing it after the
    // `focus:border-*` utility restored the base border color while focused:
    // an input that declared any `active border=` silently killed the recipe
    // ring. An explicit `focused` block still wins, because the status match
    // below runs last. `button.rs` handles the identical hazard for
    // `disabled:opacity`.
    if let Some(focus) = &utilities.focus_border_color {
        write!(
            code,
            " if matches!(__status, ::iced::widget::text_input::Status::Focused {{ .. }}) {{ __style.border.color = {}; }}",
            resolved_theme_color(focus)
        )
        .unwrap();
    }
    let overrides = [
        ("Hovered", None, input.styles.hovered.as_ref()),
        (
            "Focused { is_hovered: false }",
            None,
            input.styles.focused.as_ref(),
        ),
        (
            "Focused { is_hovered: true }",
            input.styles.focused.as_ref(),
            input.styles.focused_hovered.as_ref(),
        ),
        ("Disabled", None, input.styles.disabled.as_ref()),
    ];
    if overrides
        .iter()
        .any(|(_, inherited, style)| inherited.is_some() || style.is_some())
    {
        code.push_str(" match __status {");
        for (status, inherited, style) in overrides {
            if inherited.is_none() && style.is_none() {
                continue;
            }
            write!(code, " ::iced::widget::text_input::Status::{status} => {{").unwrap();
            if let Some(inherited) = inherited {
                append_resolved_input_status(&mut code, inherited, program, env)?;
            }
            if let Some(style) = style {
                append_resolved_input_status(&mut code, style, program, env)?;
            }
            code.push_str(" }");
        }
        code.push_str(" _ => {} }");
    }
    code.push_str(" __style })");
    Ok(code)
}

pub(super) fn append_resolved_input_status(
    code: &mut String,
    status: &ResolvedInputStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(background) = &status.surface.background {
        write!(
            code,
            " __style.background = {};",
            resolved_text_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(color) = &status.surface.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(width) = status.surface.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            resolved_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = resolved_text_radius_code(&status.surface.radius, program, env)? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    for (color, field) in [
        (&status.icon_color, "__style.icon"),
        (&status.placeholder_color, "__style.placeholder"),
        (&status.value_color, "__style.value"),
        (&status.selection_color, "__style.selection"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {};", resolved_theme_color(color)).unwrap();
        }
    }
    Ok(())
}
