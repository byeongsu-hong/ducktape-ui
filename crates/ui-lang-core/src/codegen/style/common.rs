use super::*;

pub(in crate::codegen) fn length_code(
    length: &LengthValue,
    env: &dyn BindingEnvironment,
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
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    for (method, length) in ["width", "height"].into_iter().zip(dimensions) {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    Ok(())
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
