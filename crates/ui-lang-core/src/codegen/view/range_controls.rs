use super::*;

pub(in crate::codegen) fn render_slider(
    slider: &ResolvedSlider,
    identity: Option<&ResolvedViewIdentity>,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    let value = resolved_expr_use_code(program, slider.value, env, ValueMode::Borrowed)?;
    let min = resolved_expr_use_code(program, slider.min, env, ValueMode::Borrowed)?;
    let max = resolved_expr_use_code(program, slider.max, env, ValueMode::Borrowed)?;
    let step = resolved_expr_use_code(program, slider.step, env, ValueMode::Borrowed)?;
    let callback = resolved_interaction_route_callback_code(
        &slider.change,
        "__value",
        &["__value"],
        env,
        program,
        message,
    )?;
    let helper = match slider.axis {
        ResolvedRangeAxis::Horizontal => "slider",
        ResolvedRangeAxis::Vertical => "vertical_slider",
    };
    let mut widget = format!(
        "::iced::widget::{helper}(({min})..=({max}), __slider_value, {callback}).step({step})"
    );
    if let Some(default) = slider.default {
        write!(
            widget,
            ".default({})",
            resolved_expr_use_code(program, default, env, ValueMode::Borrowed)?
        )
        .unwrap();
    }
    if let Some(shift_step) = slider.shift_step {
        write!(
            widget,
            ".shift_step({})",
            resolved_expr_use_code(program, shift_step, env, ValueMode::Borrowed)?
        )
        .unwrap();
    }
    for (method, length) in [
        ("width", slider.width.as_ref()),
        ("height", slider.height.as_ref()),
    ] {
        if let Some(length) = length {
            write!(
                widget,
                ".{method}({})",
                resolved_text_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    widget.push_str(&resolved_slider_style_code(slider, program, env)?);
    if let Some(release) = &slider.release {
        write!(
            widget,
            ".on_release({})",
            resolved_interaction_route_code(release, &[], env, program, message)?
        )
        .unwrap();
    }
    let accessibility_key =
        resolved_accessibility_key_code(identity, "slider", slider.origin, scope, env, document)?;
    Ok(format!(
        "{{ let __a11y_key = {accessibility_key}; let __slider_value = {value}; let __slider = {widget}; ::ui_lang_runtime::accessible(__slider, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::Slider).logical_id(__a11y_key.clone()).label(\"Slider\").value(format!(\"{{}}\", __slider_value)).into() }}"
    ))
}

pub(in crate::codegen) fn render_progress(
    progress: &ResolvedProgress,
    identity: Option<&ResolvedViewIdentity>,
    document: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    let value = resolved_expr_use_code(program, progress.value, env, ValueMode::Owned)?;
    let min = resolved_expr_use_code(program, progress.min, env, ValueMode::Owned)?;
    let max = resolved_expr_use_code(program, progress.max, env, ValueMode::Owned)?;
    let mut widget = "::iced::widget::progress_bar(__progress_range, __progress_value)".to_owned();
    for (method, length) in [
        ("length", progress.length.as_ref()),
        ("girth", progress.girth.as_ref()),
    ] {
        if let Some(length) = length {
            write!(
                widget,
                ".{method}({})",
                resolved_text_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    if progress.axis == ResolvedRangeAxis::Vertical {
        widget.push_str(".vertical()");
    }
    widget.push_str(&resolved_progress_style_code(progress, program, env)?);
    let accessibility_key = resolved_accessibility_key_code(
        identity,
        "progress",
        progress.origin,
        scope,
        env,
        document,
    )?;
    Ok(format!(
        "{{ let __a11y_key = {accessibility_key}; let __progress_input = {value}; let __progress = {{ let (__progress_range, __progress_value) = ::ui_lang_runtime::progress_range({min}, {max}, __progress_input); {widget} }}; ::ui_lang_runtime::accessible(__progress, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::ProgressIndicator).logical_id(__a11y_key.clone()).label(\"Progress\").value(format!(\"{{}}\", __progress_input)).into() }}"
    ))
}

fn resolved_range_custom_style_call(
    style: &ResolvedRangeCustomStyle,
    leading: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let arguments = style
        .arguments
        .iter()
        .map(|argument| {
            resolved_expr_use_code(program, *argument, env, ValueMode::Owned)
                .map(|argument| format!(", {argument}"))
        })
        .collect::<Result<String, _>>()?;
    Ok(format!(
        "{}({leading}{arguments})",
        program.extern_function(style.function).rust_path
    ))
}

fn resolved_slider_style_code(
    slider: &ResolvedSlider,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let custom = slider
        .custom_style
        .as_ref()
        .map(|style| resolved_range_custom_style_call(style, "__theme, __status", program, env))
        .transpose()?;
    let has_overrides = [
        &slider.styles.active,
        &slider.styles.hovered,
        &slider.styles.dragged,
    ]
    .into_iter()
    .any(Option::is_some);
    if !has_overrides {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::slider::default(__theme, __status)".into());
    let mut code = format!(".style(move |__theme, __status| {{ let mut __style = {base};");
    if let Some(active) = &slider.styles.active {
        append_resolved_slider_status(&mut code, active, program, env)?;
    }
    if slider.styles.hovered.is_some() || slider.styles.dragged.is_some() {
        code.push_str(" match __status {");
        for (status, style) in [
            ("Hovered", &slider.styles.hovered),
            ("Dragged", &slider.styles.dragged),
        ] {
            if let Some(style) = style {
                write!(code, " ::iced::widget::slider::Status::{status} => {{").unwrap();
                append_resolved_slider_status(&mut code, style, program, env)?;
                code.push_str(" }");
            }
        }
        code.push_str(" _ => {} }");
    }
    code.push_str(" __style })");
    Ok(code)
}

fn append_resolved_slider_status(
    code: &mut String,
    style: &ResolvedSliderStatusStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    for (background, field) in [
        (&style.rail_start, "__style.rail.backgrounds.0"),
        (&style.rail_end, "__style.rail.backgrounds.1"),
        (&style.handle_color, "__style.handle.background"),
    ] {
        if let Some(background) = background {
            write!(
                code,
                " {field} = {};",
                resolved_text_background_code(background, program, env)?
            )
            .unwrap();
        }
    }
    for (color, field) in [
        (&style.rail_border_color, "__style.rail.border.color"),
        (&style.handle_border_color, "__style.handle.border_color"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {}.into();", resolved_theme_color(color)).unwrap();
        }
    }
    for (value, field) in [
        (style.rail_width, "__style.rail.width"),
        (style.rail_border_width, "__style.rail.border.width"),
        (style.handle_border_width, "__style.handle.border_width"),
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
    if let Some(radius) = resolved_text_radius_code(&style.rail_radius, program, env)? {
        write!(code, " __style.rail.border.radius = {radius};").unwrap();
    }
    if let Some(shape) = &style.handle_shape {
        let shape = match shape {
            ResolvedSliderHandleShape::Circle(radius) => format!(
                "::iced::widget::slider::HandleShape::Circle {{ radius: {} as f32 }}",
                resolved_expr_use_code(program, *radius, env, ValueMode::Owned)?
            ),
            ResolvedSliderHandleShape::Rectangle { width, radius } => {
                let radius = resolved_text_radius_code(radius, program, env)?
                    .unwrap_or_else(|| "::iced::border::Radius::default()".into());
                format!(
                    "::iced::widget::slider::HandleShape::Rectangle {{ width: {width}, border_radius: {radius} }}"
                )
            }
        };
        write!(code, " __style.handle.shape = {shape};").unwrap();
    }
    Ok(())
}

fn resolved_progress_style_code(
    progress: &ResolvedProgress,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let radius = resolved_text_radius_code(&progress.radius, program, env)?;
    let has_style = progress.style.is_some()
        || progress.custom_style.is_some()
        || progress.background.is_some()
        || progress.bar.is_some()
        || progress.border_color.is_some()
        || progress.border_width.is_some()
        || radius.is_some();
    if !has_style {
        return Ok(String::new());
    }
    let base = if let Some(custom) = &progress.custom_style {
        resolved_range_custom_style_call(custom, "__theme", program, env)?
    } else {
        let preset = match progress.style.unwrap_or(ResolvedProgressStyle::Primary) {
            ResolvedProgressStyle::Primary => "primary",
            ResolvedProgressStyle::Secondary => "secondary",
            ResolvedProgressStyle::Success => "success",
            ResolvedProgressStyle::Warning => "warning",
            ResolvedProgressStyle::Danger => "danger",
        };
        format!("::iced::widget::progress_bar::{preset}(__theme)")
    };
    let mut code = format!(".style(move |__theme| {{ let mut __style = {base};");
    if let Some(background) = &progress.background {
        write!(
            code,
            " __style.background = {};",
            resolved_text_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(bar) = &progress.bar {
        write!(
            code,
            " __style.bar = {};",
            resolved_text_background_code(bar, program, env)?
        )
        .unwrap();
    }
    if let Some(color) = &progress.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(width) = progress.border_width {
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
    code.push_str(" __style })");
    Ok(code)
}
