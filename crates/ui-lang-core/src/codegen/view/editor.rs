use super::*;

pub(in crate::codegen) fn render_text_editor(
    editor: &ResolvedTextEditor,
    identity: Option<&ResolvedViewIdentity>,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    let state = resolved_editor_state(editor, env, program)?;
    let (action_constructor, controlled_action, content_ref) = match &state.state {
        Some(StateBinding::App(name)) => {
            let controlled = program.controlled_editor_binding(name)?;
            let variant = editor_variant(name);
            (
                format!(
                    "{message}::{variant} as fn(::iced::widget::text_editor::Action) -> {message}"
                ),
                controlled.action,
                format!("&{}", state.code),
            )
        }
        Some(StateBinding::Component {
            component,
            name,
            scope,
        }) => {
            let controlled = program.component_controlled_editor_binding(component, name)?;
            let variant = component_editor_variant(component, name);
            let field = component_state_field(component);
            let initial = component_editor_initial_field(component, name);
            let scope_code = borrowed_scope(scope);
            (
                format!(
                    "{{ let __ice_scope = ({scope_code}).clone(); move |__ice_action| {message}::{variant}(__ice_scope.clone(), __ice_action) }}"
                ),
                controlled.action,
                // The retained map hands the view a plain borrow; an
                // instance with no materialized entry renders the shared
                // initial content, which only an update pass can replace.
                format!(
                    "self.{field}.get(&{scope_code}).map_or(&self.{initial}, |__ice_local| &__ice_local.{name})"
                ),
            )
        }
        None => {
            return Err(program.invariant_at_origin(
                editor.origin,
                "normalized editor binding does not resolve to editor state",
            ));
        }
    };
    if controlled_action != editor.action.as_ref().map(|action| action.function) {
        return Err(program.invariant_at_origin(
            editor.origin,
            "normalized editor action diverges from its controlled binding",
        ));
    }
    let accessibility_key =
        resolved_accessibility_key_code(identity, "editor", editor.origin, scope, env, document)?;
    let accessibility_label = editor
        .placeholder
        .as_deref()
        .map(rust_string)
        .unwrap_or_else(|| "\"Editor\"".to_owned());
    let disabled = editor
        .disabled
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?;
    let mut code = "::iced::widget::text_editor(__ice_editor_content)".to_owned();
    if let Some(identity) = identity {
        write!(
            code,
            ".id(::iced::widget::Id::from({}))",
            resolved_view_identity_code(identity, scope, env, document)?
        )
        .unwrap();
    }
    if let Some(placeholder) = &editor.placeholder {
        write!(code, ".placeholder({})", rust_string(placeholder)).unwrap();
    }
    if let Some(width) = editor.width {
        write!(
            code,
            ".width((({}) as f32).max(0.0).min(f32::MAX))",
            resolved_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(height) = &editor.height {
        write!(
            code,
            ".height({})",
            resolved_text_length_code(height, program, env)?
        )
        .unwrap();
    }
    for (value, method, min) in [
        (editor.min_height, "min_height", "0.0"),
        (editor.max_height, "max_height", "0.0"),
        (editor.size, "size", "f32::EPSILON"),
        (editor.padding, "padding", "0.0"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}((({}) as f32).max({min}).min(f32::MAX))",
                resolved_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(line_height) = &editor.line_height {
        let line_height = match line_height {
            ResolvedTextLineHeight::Relative(value) => format!(
                "::iced::widget::text::LineHeight::Relative((({}) as f32).max(f32::EPSILON).min(f32::MAX))",
                resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
            ),
            ResolvedTextLineHeight::Absolute(value) => format!(
                "::iced::widget::text::LineHeight::Absolute((({}) as f32).max(f32::EPSILON).min(f32::MAX).into())",
                resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
            ),
        };
        write!(code, ".line_height({line_height})").unwrap();
    }
    if let Some(wrapping) = editor.wrapping {
        let wrapping = match wrapping {
            ResolvedEditorWrapping::None => "None",
            ResolvedEditorWrapping::Glyph => "Glyph",
            ResolvedEditorWrapping::Word => "Word",
            ResolvedEditorWrapping::WordOrGlyph => "WordOrGlyph",
        };
        write!(
            code,
            ".wrapping(::iced::widget::text::Wrapping::{wrapping})"
        )
        .unwrap();
    }
    if let Some(font) = &editor.font {
        write!(code, ".font({})", resolved_input_font_code(font)).unwrap();
    }
    if let Some(syntax) = &editor.highlight {
        let theme = match editor
            .highlight_theme
            .unwrap_or(ResolvedHighlightTheme::Base16Ocean)
        {
            ResolvedHighlightTheme::SolarizedDark => "SolarizedDark",
            ResolvedHighlightTheme::Base16Mocha => "Base16Mocha",
            ResolvedHighlightTheme::Base16Ocean => "Base16Ocean",
            ResolvedHighlightTheme::Base16Eighties => "Base16Eighties",
            ResolvedHighlightTheme::InspiredGithub => "InspiredGitHub",
        };
        write!(
            code,
            ".highlight({}, ::iced::highlighter::Theme::{theme})",
            rust_string(syntax)
        )
        .unwrap();
    }
    if let Some(binding) = &editor.key_binding {
        let callback = resolved_interaction_route_callback_with_code(
            &binding.route,
            "__key_press",
            env,
            program,
            |callback_env| {
                let suffix = checked_editor_args_suffix(&binding.arguments, program, callback_env)?;
                let route = resolved_interaction_route_code(
                    &binding.route,
                    &["__value"],
                    callback_env,
                    program,
                    message,
                )?;
                Ok(format!(
                    "{}(__key_press{suffix}).map(|__binding| __ice_map_editor_binding(__binding, &|__value| {route}))",
                    program.extern_function(binding.function).rust_path
                ))
            },
        )?;
        write!(code, ".key_binding({callback})").unwrap();
    }
    code.push_str(&resolved_editor_style_code(editor, program, env)?);
    let finish = |editor_code: String| -> Result<String, Error> {
        if let Some(highlighter) = &editor.highlighter {
            let suffix = checked_editor_args_suffix(&highlighter.arguments, program, env)?;
            Ok(format!(
                "{}({editor_code}{suffix})",
                program.extern_function(highlighter.function).rust_path
            ))
        } else {
            Ok(editor_code)
        }
    };
    let enabled = format!("{code}.on_action({action_constructor})");
    if let Some(disabled) = disabled {
        let disabled_editor = finish(code)?;
        let enabled_editor = finish(enabled)?;
        let editor_code = format!(
            "if __disabled {{ {disabled_editor}.into() }} else {{ {enabled_editor}.into() }}"
        );
        Ok(format!(
            "{{ let __a11y_key = {accessibility_key}; let __ice_editor_content = {content_ref}; let __disabled = {disabled}; let __editor_value = __ice_editor_content.text(); let __editor: __IceElement<'_, {message}> = {editor_code}; ::ui_lang_runtime::accessible(__editor, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::MultilineTextInput).logical_id(__a11y_key.clone()).label({accessibility_label}).value(__editor_value).disabled(__disabled).into() }}"
        ))
    } else {
        let editor_code = format!("{}.into()", finish(enabled)?);
        Ok(format!(
            "{{ let __a11y_key = {accessibility_key}; let __ice_editor_content = {content_ref}; let __editor_value = __ice_editor_content.text(); let __editor: __IceElement<'_, {message}> = {editor_code}; ::ui_lang_runtime::accessible(__editor, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::MultilineTextInput).logical_id(__a11y_key.clone()).label({accessibility_label}).value(__editor_value).into() }}"
        ))
    }
}

fn resolved_editor_state<'a>(
    editor: &ResolvedTextEditor,
    env: &'a dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<&'a Binding, Error> {
    let state = env.get(editor.binding.name()).ok_or_else(|| {
        program.invariant_at_origin(
            editor.origin,
            "editor state is absent from its render scope",
        )
    })?;
    if state.ty != Type::Editor
        || state.owner != Some(BindingOwner::Value(editor.binding.checked_ref()))
    {
        return Err(program.invariant_at_origin(
            editor.origin,
            "editor render binding does not match its normalized state ID",
        ));
    }
    match (&editor.binding, &state.state) {
        (WritableStateRef::App { name, .. }, Some(StateBinding::App(actual))) if name == actual => {
        }
        (
            WritableStateRef::ComponentParam { .. },
            Some(StateBinding::App(_) | StateBinding::Component { .. }),
        ) => {}
        (
            WritableStateRef::ComponentState { name, .. },
            Some(StateBinding::Component { name: actual, .. }),
        ) if name == actual => {}
        _ => {
            return Err(program.invariant_at_origin(
                editor.origin,
                "editor render state capability diverged from normalized binding",
            ));
        }
    }
    Ok(state)
}

fn checked_editor_args_suffix(
    arguments: &[ResolvedExpressionId],
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    arguments
        .iter()
        .map(|argument| {
            resolved_expr_use_code(program, *argument, env, ValueMode::Owned)
                .map(|argument| format!(", {argument}"))
        })
        .collect()
}

fn resolved_editor_style_code(
    editor: &ResolvedTextEditor,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let custom = editor
        .custom_style
        .as_ref()
        .map(|style| {
            let suffix = checked_editor_args_suffix(&style.arguments, program, env)?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{suffix})",
                program.extern_function(style.function).rust_path
            ))
        })
        .transpose()?;
    let has_overrides = [
        &editor.styles.active,
        &editor.styles.hovered,
        &editor.styles.focused,
        &editor.styles.focused_hovered,
        &editor.styles.disabled,
    ]
    .into_iter()
    .any(Option::is_some);
    if !has_overrides {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::text_editor::default(__theme, __status)".into());
    let mut code = format!(".style(move |__theme, __status| {{ let mut __style = {base};");
    if let Some(active) = &editor.styles.active {
        append_resolved_input_status(&mut code, active, program, env)?;
    }
    let overrides = [
        ("Hovered", None, editor.styles.hovered.as_ref()),
        (
            "Focused { is_hovered: false }",
            None,
            editor.styles.focused.as_ref(),
        ),
        (
            "Focused { is_hovered: true }",
            editor.styles.focused.as_ref(),
            editor.styles.focused_hovered.as_ref(),
        ),
        ("Disabled", None, editor.styles.disabled.as_ref()),
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
            write!(code, " ::iced::widget::text_editor::Status::{status} => {{").unwrap();
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
