use super::*;

pub(in crate::codegen) fn length_code(
    length: &LengthValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match length {
        LengthValue::Fill => "::iced::Fill".into(),
        LengthValue::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        LengthValue::Shrink => "::iced::Shrink".into(),
        LengthValue::Fixed(value) => {
            let code = expr_code(value, env, document, ValueMode::Owned)?;
            if expr_type(value, &env_types(env), document, &Span::line(1))? == Type::Length {
                code
            } else {
                format!("{code} as f32")
            }
        }
    })
}

pub(in crate::codegen) fn append_dimensions(
    code: &mut String,
    dimensions: [&Option<LengthValue>; 2],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    for (method, length) in ["width", "height"].into_iter().zip(dimensions) {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    Ok(())
}

pub(in crate::codegen) fn typed_padding_code(
    padding: &PaddingOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let code = |value: Option<&Expr>| {
        value
            .map(|value| expr_code(value, env, document, ValueMode::Owned))
            .transpose()
    };
    let all = code(padding.all.as_ref())?.unwrap_or_else(|| "0.0".into());
    let x = code(padding.x.as_ref())?.unwrap_or_else(|| all.clone());
    let y = code(padding.y.as_ref())?.unwrap_or_else(|| all.clone());
    let top = code(padding.top.as_ref())?.unwrap_or_else(|| y.clone());
    let right = code(padding.right.as_ref())?.unwrap_or_else(|| x.clone());
    let bottom = code(padding.bottom.as_ref())?.unwrap_or(y);
    let left = code(padding.left.as_ref())?.unwrap_or(x);
    Ok(Some(format!(
        "::ui_lang_runtime::bounded_padding({top}, {right}, {bottom}, {left})"
    )))
}

pub(in crate::codegen) fn radius_code(
    uniform: Option<&Expr>,
    corners: [Option<&Expr>; 4],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if uniform.is_none() && corners.iter().all(Option::is_none) {
        return Ok(None);
    }
    let base = uniform
        .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", env, document))
        .transpose()?
        .unwrap_or_else(|| "0.0".to_owned());
    let mut values = Vec::with_capacity(4);
    for corner in corners {
        values.push(
            corner
                .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", env, document))
                .transpose()?
                .unwrap_or_else(|| base.clone()),
        );
    }
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {}, top_right: {}, bottom_right: {}, bottom_left: {} }}",
        values[0], values[1], values[2], values[3]
    )))
}

pub(in crate::codegen) fn append_float_style(
    code: &mut String,
    style: &FloatStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let radius = radius_code(
        style.radius.as_ref(),
        [
            style.radius_top_left.as_ref(),
            style.radius_top_right.as_ref(),
            style.radius_bottom_right.as_ref(),
            style.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )?;
    if style.shadow_color.is_none()
        && style.shadow_x.is_none()
        && style.shadow_y.is_none()
        && style.shadow_blur.is_none()
        && radius.is_none()
    {
        return Ok(());
    }
    code.push_str(".style(move |_| { let mut __style = ::iced::widget::float::Style::default();");
    if let Some(color) = &style.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    append_f32_fields(
        code,
        [
            (&style.shadow_x, "__style.shadow.offset.x"),
            (&style.shadow_y, "__style.shadow.offset.y"),
            (&style.shadow_blur, "__style.shadow.blur_radius"),
        ],
        env,
        document,
    )?;
    if let Some(radius) = radius {
        write!(code, " __style.shadow_border_radius = {radius};").unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn background_code(
    background: &BackgroundValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    match background {
        BackgroundValue::Color(color) => Ok(format!(
            "::iced::Background::Color({})",
            theme_color(document, color)
        )),
        BackgroundValue::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                expr_code(angle, env, document, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    expr_code(&stop.offset, env, document, ValueMode::Owned)?,
                    theme_color(document, &stop.color)
                )
                .unwrap();
            }
            code.push(')');
            Ok(code)
        }
    }
}

pub(in crate::codegen) fn container_surface_style_value(
    utilities: &ResolvedStyle,
    options: &ContainerStyleOptions,
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    let has_typed_style = options.background.is_some()
        || options.text_color.is_some()
        || options.border_color.is_some()
        || options.border_width.is_some()
        || options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some()
        || options.shadow_color.is_some()
        || options.shadow_x.is_some()
        || options.shadow_y.is_some()
        || options.shadow_blur.is_some()
        || options.pixel_snap.is_some();
    let utility_style = container_style_value(utilities);
    let custom_style = custom
        .map(|style| {
            custom_style_call_code(style, ExternKind::ContainerStyle, "__theme", env, document)
        })
        .transpose()?;
    if !has_typed_style && custom_style.is_none() {
        return Ok(utility_style);
    }
    if !has_typed_style && utility_style.is_none() {
        return Ok(custom_style);
    }

    let has_custom_style = custom_style.is_some();
    let base = custom_style
        .or_else(|| utility_style.clone())
        .unwrap_or_else(|| "::iced::widget::container::Style::default()".into());
    let mut code = format!("{{ let mut __style = {base};");
    if has_custom_style {
        append_container_utility_overrides(&mut code, utilities);
    }
    append_surface_style_overrides(&mut code, options, env, document)?;
    if let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, color)
        )
        .unwrap();
    }
    code.push_str(" __style }");
    Ok(Some(code))
}

pub(in crate::codegen) fn append_container_utility_overrides(
    code: &mut String,
    style: &ResolvedStyle,
) {
    if let Some(background) = &style.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({}.into());",
            resolved_theme_color(background)
        )
        .unwrap();
    }
    if let Some(text) = &style.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(text)
        )
        .unwrap();
    }
    if let Some(border) = &style.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(border)
        )
        .unwrap();
    }
    if style.border_width != 0 {
        write!(code, " __style.border.width = {}.0;", style.border_width).unwrap();
    }
    if style.radius != 0 {
        write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
    }
}

pub(in crate::codegen) fn append_surface_style_overrides(
    code: &mut String,
    options: &ContainerStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({});",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        options.radius.as_ref(),
        [
            options.radius_top_left.as_ref(),
            options.radius_top_right.as_ref(),
            options.radius_bottom_right.as_ref(),
            options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(color) = &options.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(x) = &options.shadow_x {
        write!(
            code,
            " __style.shadow.offset.x = {} as f32;",
            expr_code(x, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(y) = &options.shadow_y {
        write!(
            code,
            " __style.shadow.offset.y = {} as f32;",
            expr_code(y, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(blur) = &options.shadow_blur {
        write!(
            code,
            " __style.shadow.blur_radius = {} as f32;",
            expr_code(blur, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(snap) = &options.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(())
}
