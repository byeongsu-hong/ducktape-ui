use super::*;

impl ResolvedStyleFontWeight {
    pub(in crate::codegen) fn code(self) -> &'static str {
        match self {
            Self::Medium => "Medium",
            Self::Semibold => "Semibold",
            Self::Bold => "Bold",
        }
    }
}

impl ResolvedStyle {
    pub(in crate::codegen) fn padding_code(&self) -> Option<String> {
        (self.padding != [0; 4]).then(|| {
            format!(
                "::iced::Padding {{ top: {}.0, right: {}.0, bottom: {}.0, left: {}.0 }}",
                self.padding[0], self.padding[1], self.padding[2], self.padding[3]
            )
        })
    }
}

pub(in crate::codegen) fn append_size(code: &mut String, style: &ResolvedStyle) {
    if style.width_fill {
        code.push_str(".width(::iced::Fill)");
    }
    if style.height_fill {
        code.push_str(".height(::iced::Fill)");
    }
}

pub(in crate::codegen) fn container_style_code(style: &ResolvedStyle) -> String {
    container_style_value(style)
        .map(|style| format!(".style(move |_| {style})"))
        .unwrap_or_default()
}

pub(in crate::codegen) fn container_style_value(style: &ResolvedStyle) -> Option<String> {
    if style.background.is_none()
        && style.border_width == 0
        && style.border_color.is_none()
        && style.radius == 0
        && style.text_color.is_none()
    {
        return None;
    }
    let background = style
        .background
        .as_ref()
        .map(|color| format!("Some({}.into())", resolved_theme_color(color)))
        .unwrap_or_else(|| "None".into());
    let text = style
        .text_color
        .as_ref()
        .map(|color| format!("Some({})", resolved_theme_color(color)))
        .unwrap_or_else(|| "None".into());
    let border = style
        .border_color
        .as_ref()
        .map(resolved_theme_color)
        .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
    Some(format!(
        "::iced::widget::container::Style {{ background: {background}, text_color: {text}, border: ::iced::Border {{ color: {border}, width: {}.0, radius: {}.0.into() }}, ..::iced::widget::container::Style::default() }}",
        style.border_width, style.radius
    ))
}

pub(in crate::codegen) fn resolved_theme_color(color: &ResolvedThemeColor) -> String {
    let value = match color.base {
        ResolvedThemeColorBase::White => color_code("#ffffff", None),
        ResolvedThemeColorBase::Black => color_code("#000000", None),
        ResolvedThemeColorBase::Transparent => color_code("#00000000", None),
        ResolvedThemeColorBase::Token(token) => {
            format!("__ice_palette.colors[{}]", token.index)
        }
    };
    color.opacity.map_or(value.clone(), |opacity| {
        format!(
            "{{ let mut __color = {value}; __color.a = {:.6}; __color }}",
            opacity as f32 / 100.0
        )
    })
}

pub(in crate::codegen) fn resolved_theme_preset_code(
    preset: &ResolvedThemePreset,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    Ok(match preset {
        ResolvedThemePreset::Default => "::std::option::Option::None".into(),
        ResolvedThemePreset::App => "::std::option::Option::Some(__ice_app_theme.clone())".into(),
        ResolvedThemePreset::BuiltIn(name) => format!(
            "::std::option::Option::Some(::iced::Theme::{})",
            pascal(name)
        ),
        ResolvedThemePreset::Factory(factory) => format!(
            "::std::option::Option::Some({})",
            resolved_theme_factory_code(factory, env, program)?
        ),
    })
}

pub(in crate::codegen) fn resolved_theme_factory_code(
    factory: &ResolvedThemeFactory,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let args = factory
        .arguments
        .iter()
        .map(|argument| {
            let mode = match argument.mode {
                ResolvedExternViewArgumentMode::Owned => ValueMode::Owned,
                ResolvedExternViewArgumentMode::BorrowedAsRef
                | ResolvedExternViewArgumentMode::Borrowed => ValueMode::Borrowed,
            };
            let code = resolved_expr_use_code(program, argument.expression, env, mode)?;
            Ok(match argument.mode {
                ResolvedExternViewArgumentMode::Owned => code,
                ResolvedExternViewArgumentMode::BorrowedAsRef => {
                    format!("::std::convert::AsRef::as_ref(&({code}))")
                }
                ResolvedExternViewArgumentMode::Borrowed => {
                    format!("::std::borrow::Borrow::borrow(&({code}))")
                }
            })
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{}({args})", factory.function.rust_path))
}

pub(in crate::codegen) fn resolved_app_theme_factory_code(
    factory: &ResolvedAppThemeFactory,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let function = program.extern_function(factory.function);
    let args = factory
        .arguments
        .iter()
        .map(|argument| resolved_expr_use_code(program, argument.expression, env, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!("{}({args})", function.rust_path))
}

pub(in crate::codegen) fn resolved_background_code(
    background: &ResolvedBackground,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    Ok(match background {
        ResolvedBackground::Color(color) => {
            format!("::iced::Background::Color({})", resolved_theme_color(color))
        }
        ResolvedBackground::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                resolved_expr_use_code(program, *angle, env, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    resolved_expr_use_code(program, stop.offset, env, ValueMode::Owned)?,
                    resolved_theme_color(&stop.color)
                )
                .unwrap();
            }
            code.push(')');
            code
        }
    })
}

pub(in crate::codegen) fn color_code(value: &str, opacity: Option<u8>) -> String {
    let hex = value.trim_start_matches('#');
    let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).unwrap_or(0);
    let alpha = opacity
        .map(|value| value as f32 / 100.0)
        .or_else(|| (hex.len() == 8).then(|| byte(6..8) as f32 / 255.0))
        .unwrap_or(1.0);
    format!(
        "::iced::Color::from_rgba8({}, {}, {}, {alpha:.6})",
        byte(0..2),
        byte(2..4),
        byte(4..6)
    )
}

pub(in crate::codegen) fn rust_string(value: &str) -> String {
    format!("{value:?}")
}

pub(in crate::codegen) fn rust_f64(value: f64) -> String {
    format!("{value:?}")
}

pub(in crate::codegen) fn pascal(value: &str) -> String {
    value
        .split(['_', '-', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect()
}
