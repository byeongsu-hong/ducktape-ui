use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_button(
    button: &ResolvedButton,
    id: &Option<Id>,
    raw_child: Option<&ViewNode>,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document.hir();
    let source_span = Span::line(program.origin(button.origin).line);
    let accessibility_key =
        accessibility_key_code(id.as_ref(), "button", &source_span, scope, env, document)?;
    let fallback_label = match &button.content {
        ResolvedButtonContent::Label(label) => rust_string(label),
        ResolvedButtonContent::Child(_) => "::std::string::String::new()".into(),
    };
    let accessibility_label = button
        .accessibility_label
        .map(|value| checked_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or(fallback_label);
    let accessibility_description = button
        .accessibility_description
        .map(|value| checked_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .map(|value| format!(".description({value})"))
        .unwrap_or_default();
    let disabled = button
        .disabled
        .map(|value| checked_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let activate = resolved_interaction_route_code(&button.route, &[], env, program, message)?;
    let mut content = match (&button.content, raw_child) {
        (ResolvedButtonContent::Label(label), None) => {
            let mut text = format!("::iced::widget::text({})", rust_string(label));
            append_resolved_button_label_style(&mut text, &button.utility_style);
            format!("{text}.into()")
        }
        (ResolvedButtonContent::Child(expected), Some(child)) => {
            let actual = program.checked_view(child.span())?.id;
            if actual != *expected {
                return Err(program.invariant_at_origin(
                    button.origin,
                    "button child diverged from normalized HIR",
                ));
            }
            let child_scope = id.as_ref().map_or_else(
                || Ok(scope.to_owned()),
                |id| id_code(id, scope, env, document),
            )?;
            render_node(child, document, message, env, &child_scope, slot)?
        }
        _ => {
            return Err(program.invariant_at_origin(
                button.origin,
                "button raw content topology diverged from normalized HIR",
            ));
        }
    };
    let center_x = matches!(
        button.width,
        Some(ResolvedContainerLength::FixedF64(_) | ResolvedContainerLength::FixedLength(_))
    );
    let center_y = matches!(
        button.height,
        Some(ResolvedContainerLength::FixedF64(_) | ResolvedContainerLength::FixedLength(_))
    );
    if center_x || center_y {
        let mut centered = format!(
            "{{ let __button_inner: __IceElement<'_, {message}> = {content}; ::iced::widget::container(__button_inner)"
        );
        if center_x {
            centered
                .push_str(".width(::iced::Fill).align_x(::iced::alignment::Horizontal::Center)");
        }
        if center_y {
            centered.push_str(".height(::iced::Fill).align_y(::iced::alignment::Vertical::Center)");
        }
        content = format!("{centered}.into() }}");
    }
    let mut code = format!(
        "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __disabled = {disabled}; let __activate = {activate}; let __button_content: __IceElement<'_, {message}> = {content}; let __button = ::iced::widget::button(__button_content)"
    );
    if let Some(padding) = button.utility_style.padding_code() {
        write!(code, ".padding({padding})").unwrap();
    }
    for (method, length) in [("width", &button.width), ("height", &button.height)] {
        if let Some(length) = length {
            write!(
                code,
                ".{method}({})",
                resolved_text_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    if let Some(padding) = button.padding {
        write!(
            code,
            ".padding({} as f32)",
            checked_expr_use_code(program, padding, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(clip) = button.clip {
        write!(
            code,
            ".clip({})",
            checked_expr_use_code(program, clip, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(".on_press_maybe(if __disabled { None } else { Some(__activate.clone()) })");
    code.push_str(&resolved_button_style_code(button, program, env)?);
    Ok(format!(
        "{code}; ::ui_lang_runtime::accessible(__button, __a11y_id, ::ui_lang_runtime::Role::Button).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
    ))
}

fn append_resolved_button_label_style(code: &mut String, style: &ResolvedStyle) {
    if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    if let Some(line_height) = style.text_line_height {
        write!(
            code,
            ".line_height(::iced::widget::text::LineHeight::Relative({line_height}))"
        )
        .unwrap();
    }
    let font = match (style.font_monospace, style.font_weight) {
        (false, None) => None,
        (true, None) => Some("::iced::Font::MONOSPACE".into()),
        (monospace, Some(weight)) => {
            let base = if monospace {
                "::iced::Font::MONOSPACE"
            } else {
                "Self::default_font()"
            };
            Some(format!(
                "::iced::Font {{ weight: ::iced::font::Weight::{}, ..{base} }}",
                weight.code()
            ))
        }
    };
    if let Some(font) = font {
        write!(code, ".font({font})").unwrap();
    }
}

fn resolved_button_style_code(
    button: &ResolvedButton,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let utilities = &button.utility_style;
    let has_utilities = utilities.background.is_some()
        || utilities.hover_background.is_some()
        || utilities.pressed_background.is_some()
        || utilities.disabled_background.is_some()
        || utilities.disabled_text_color.is_some()
        || utilities.text_color.is_some()
        || utilities.border_width != 0
        || utilities.border_color.is_some()
        || utilities.radius != 0
        || utilities.disabled_opacity.is_some();
    let has_typed = [
        &button.styles.active,
        &button.styles.hovered,
        &button.styles.pressed,
        &button.styles.disabled,
    ]
    .into_iter()
    .any(Option::is_some);
    let custom = button
        .custom_style
        .as_ref()
        .map(|style| {
            let arguments = style
                .arguments
                .iter()
                .map(|argument| checked_expr_use_code(program, *argument, env, ValueMode::Owned))
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
    let preset = match button.preset {
        ResolvedButtonPreset::Primary => "primary",
        ResolvedButtonPreset::Secondary => "secondary",
        ResolvedButtonPreset::Success => "success",
        ResolvedButtonPreset::Warning => "warning",
        ResolvedButtonPreset::Danger => "danger",
        ResolvedButtonPreset::Text => "text",
        ResolvedButtonPreset::Background => "background",
        ResolvedButtonPreset::Subtle => "subtle",
    };
    if !has_utilities && !has_typed {
        return Ok(if let Some(custom) = custom {
            format!(".style(move |__theme, __status| {custom})")
        } else if button.preset == ResolvedButtonPreset::Primary {
            String::new()
        } else {
            format!(".style(::iced::widget::button::{preset})")
        });
    }
    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::button::{preset}(__theme, __status)"));
    let mut code = format!(".style(move |__theme, __status| {{ let mut __style = {base};");
    if has_utilities {
        let normal = utilities.background.as_ref().map(resolved_theme_color);
        let hover = utilities
            .hover_background
            .as_ref()
            .map(resolved_theme_color)
            .or_else(|| normal.clone());
        let pressed = utilities
            .pressed_background
            .as_ref()
            .map(resolved_theme_color)
            .or_else(|| hover.clone())
            .or_else(|| normal.clone());
        let option = |color: Option<String>| {
            color.map_or_else(|| "None".into(), |color| format!("Some({color})"))
        };
        write!(
            code,
            " let __background: Option<::iced::Color> = match __status {{ ::iced::widget::button::Status::Hovered => {}, ::iced::widget::button::Status::Pressed => {}, ::iced::widget::button::Status::Disabled => {}, _ => {} }}; if let Some(__background) = __background {{ __style.background = Some(::iced::Background::Color(__background)); }}",
            option(hover),
            option(pressed),
            option(normal.clone()),
            option(normal),
        )
        .unwrap();
        if let Some(text) = &utilities.text_color {
            write!(
                code,
                " __style.text_color = {};",
                resolved_theme_color(text)
            )
            .unwrap();
        }
        if utilities.border_width > 0 {
            write!(
                code,
                " __style.border.width = {}.0;",
                utilities.border_width
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
        if utilities.radius > 0 {
            write!(
                code,
                " __style.border.radius = {}.0.into();",
                utilities.radius
            )
            .unwrap();
        }
        if utilities.background.is_some()
            || utilities.text_color.is_some()
            || utilities.disabled_opacity.is_some()
            || utilities.disabled_background.is_some()
            || utilities.disabled_text_color.is_some()
        {
            let disabled = utilities.disabled_opacity.unwrap_or(0.5);
            code.push_str(" if matches!(__status, ::iced::widget::button::Status::Disabled) {");
            if let Some(background) = &utilities.disabled_background {
                write!(
                    code,
                    " __style.background = Some({}.into());",
                    resolved_theme_color(background)
                )
                .unwrap();
            } else if utilities.background.is_some() || utilities.disabled_opacity.is_some() {
                write!(code, " if let Some(::iced::Background::Color(mut __color)) = __style.background {{ __color.a *= {disabled}; __style.background = Some(::iced::Background::Color(__color)); }}").unwrap();
            }
            if let Some(text) = &utilities.disabled_text_color {
                write!(
                    code,
                    " __style.text_color = {};",
                    resolved_theme_color(text)
                )
                .unwrap();
            } else if utilities.text_color.is_some() || utilities.disabled_opacity.is_some() {
                write!(code, " __style.text_color.a *= {disabled};").unwrap();
            }
            code.push_str(" }");
        }
    }
    if let Some(active) = &button.styles.active {
        append_resolved_button_status(&mut code, active, program, env)?;
    }
    let overrides = [
        ("Hovered", &button.styles.hovered),
        ("Pressed", &button.styles.pressed),
        ("Disabled", &button.styles.disabled),
    ];
    if overrides.iter().any(|(_, status)| status.is_some()) {
        code.push_str(" match __status {");
        for (status, style) in overrides {
            let Some(style) = style else { continue };
            write!(code, " ::iced::widget::button::Status::{status} => {{").unwrap();
            append_resolved_button_status(&mut code, style, program, env)?;
            code.push_str(" }");
        }
        code.push_str(" _ => {} }");
    }
    code.push_str(" __style })");
    Ok(code)
}

fn append_resolved_button_status(
    code: &mut String,
    status: &ResolvedButtonStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    let surface = &status.surface;
    if let Some(background) = &surface.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({});",
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
            checked_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = resolved_text_radius_code(&surface.radius, program, env)? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
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
                checked_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(snap) = surface.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            checked_expr_use_code(program, snap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(())
}
