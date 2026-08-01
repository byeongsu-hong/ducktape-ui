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

pub(in crate::codegen) fn button_style_code(
    style: &ResolvedStyle,
    typed: &ButtonStyleSet,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let has_utilities = style.background.is_some()
        || style.hover_background.is_some()
        || style.pressed_background.is_some()
        || style.disabled_background.is_some()
        || style.disabled_text_color.is_some()
        || style.text_color.is_some()
        || style.border_width != 0
        || style.border_color.is_some()
        || style.radius != 0
        || style.disabled_opacity.is_some();
    let has_typed = typed.active.is_some()
        || typed.hovered.is_some()
        || typed.pressed.is_some()
        || typed.disabled.is_some();
    let custom = typed
        .custom
        .as_ref()
        .map(|style| {
            custom_style_call_code(
                style,
                ExternKind::ButtonStyle,
                "__theme, __status",
                env,
                document,
            )
        })
        .transpose()?;
    let preset = match typed.preset {
        ButtonStylePreset::Primary => "primary",
        ButtonStylePreset::Secondary => "secondary",
        ButtonStylePreset::Success => "success",
        ButtonStylePreset::Warning => "warning",
        ButtonStylePreset::Danger => "danger",
        ButtonStylePreset::Text => "text",
        ButtonStylePreset::Background => "background",
        ButtonStylePreset::Subtle => "subtle",
    };
    if !has_utilities && !has_typed {
        return Ok(if let Some(custom) = custom {
            format!(".style(move |__theme, __status| {custom})")
        } else if typed.preset == ButtonStylePreset::Primary {
            String::new()
        } else {
            format!(".style(::iced::widget::button::{preset})")
        });
    }

    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::button::{preset}(__theme, __status)"));
    let mut code = format!(".style(move |__theme, __status| {{ let mut __style = {base};");
    if has_utilities {
        let normal = style.background.as_ref().map(resolved_theme_color);
        let hover = style
            .hover_background
            .as_ref()
            .map(resolved_theme_color)
            .or_else(|| normal.clone());
        let pressed = style
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
        if let Some(text) = &style.text_color {
            write!(
                code,
                " __style.text_color = {};",
                resolved_theme_color(text)
            )
            .unwrap();
        }
        if style.border_width > 0 {
            write!(code, " __style.border.width = {}.0;", style.border_width).unwrap();
        }
        if let Some(border) = &style.border_color {
            write!(
                code,
                " __style.border.color = {};",
                resolved_theme_color(border)
            )
            .unwrap();
        }
        if style.radius > 0 {
            write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
        }
        if style.background.is_some()
            || style.text_color.is_some()
            || style.disabled_opacity.is_some()
            || style.disabled_background.is_some()
            || style.disabled_text_color.is_some()
        {
            let disabled = style.disabled_opacity.unwrap_or(0.5);
            code.push_str(" if matches!(__status, ::iced::widget::button::Status::Disabled) {");
            if let Some(background) = &style.disabled_background {
                write!(
                    code,
                    " __style.background = Some({}.into());",
                    resolved_theme_color(background)
                )
                .unwrap();
            } else if style.background.is_some() || style.disabled_opacity.is_some() {
                write!(code, " if let Some(::iced::Background::Color(mut __color)) = __style.background {{ __color.a *= {disabled}; __style.background = Some(::iced::Background::Color(__color)); }}").unwrap();
            }
            if let Some(text) = &style.disabled_text_color {
                write!(
                    code,
                    " __style.text_color = {};",
                    resolved_theme_color(text)
                )
                .unwrap();
            } else if style.text_color.is_some() || style.disabled_opacity.is_some() {
                write!(code, " __style.text_color.a *= {disabled};").unwrap();
            }
            code.push_str(" }");
        }
    }
    if has_typed {
        if let Some(active) = &typed.active {
            append_button_status_style(&mut code, active, env, document)?;
        }
        let overrides = [
            ("Hovered", &typed.hovered),
            ("Pressed", &typed.pressed),
            ("Disabled", &typed.disabled),
        ];
        if overrides.iter().any(|(_, status)| status.is_some()) {
            code.push_str(" match __status {");
            for (variant, status) in overrides {
                let Some(status) = status else { continue };
                write!(code, " ::iced::widget::button::Status::{variant} => {{").unwrap();
                append_button_status_style(&mut code, status, env, document)?;
                code.push_str(" }");
            }
            code.push_str(" _ => {} }");
        }
    }
    code.push_str(" __style })");
    Ok(code)
}

fn append_button_status_style(
    code: &mut String,
    style: &ButtonStatusStyle,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    append_surface_style_overrides(code, &style.options, env, document)?;
    if let Some(color) = &style.options.text_color {
        write!(
            code,
            " __style.text_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn theme_color(document: &Document, token: &str) -> String {
    let (name, opacity) = token
        .split_once('/')
        .map_or((token, None), |(name, opacity)| {
            (name, opacity.parse::<u8>().ok())
        });
    let color = match name {
        "white" => color_code("#ffffff", None),
        "black" => color_code("#000000", None),
        "transparent" => color_code("#00000000", None),
        name => {
            let index = document
                .theme_contract
                .as_ref()
                .and_then(|contract| contract.tokens.iter().position(|token| token == name))
                .expect("checker validates theme tokens");
            format!("__ice_palette.colors[{index}]")
        }
    };
    opacity.map_or(color.clone(), |opacity| {
        format!(
            "{{ let mut __color = {color}; __color.a = {:.6}; __color }}",
            opacity as f32 / 100.0
        )
    })
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
    let function = program.extern_function(factory.function);
    let args = expr_list_code(&factory.arguments, env, program.document())?;
    Ok(format!("{}({args})", function.rust_path))
}

pub(in crate::codegen) fn resolved_background_code(
    background: &ResolvedBackground,
    env: &dyn BindingEnvironment,
    document: &RenderDocument<'_>,
) -> Result<String, Error> {
    Ok(match background {
        ResolvedBackground::Color(color) => {
            format!("::iced::Background::Color({})", resolved_theme_color(color))
        }
        ResolvedBackground::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                expr_code(angle, env, document, ValueMode::Owned)?
            );
            for (color, offset) in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    expr_code(offset, env, document, ValueMode::Owned)?,
                    resolved_theme_color(color)
                )
                .unwrap();
            }
            code.push(')');
            code
        }
    })
}

/// Encodes a QR payload where it is rendered, never once in application state:
/// the payload is an expression, so a matrix built at startup would be stale
/// the moment the expression changes.
pub(in crate::codegen) fn qr_data_code(
    payload: &Expr,
    correction: Option<QrCorrection>,
    version: Option<QrVersion>,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let module = "::iced::widget::qr_code";
    let data = format!(
        "&({})",
        expr_code(payload, env, document, ValueMode::Borrowed)?
    );
    let correction_code = |value| match value {
        QrCorrection::Low => format!("{module}::ErrorCorrection::Low"),
        QrCorrection::Medium => format!("{module}::ErrorCorrection::Medium"),
        QrCorrection::Quartile => format!("{module}::ErrorCorrection::Quartile"),
        QrCorrection::High => format!("{module}::ErrorCorrection::High"),
    };
    Ok(if let Some(version) = version {
        let version = match version {
            QrVersion::Normal(value) => format!("{module}::Version::Normal({value})"),
            QrVersion::Micro(value) => format!("{module}::Version::Micro({value})"),
        };
        let correction = correction_code(correction.unwrap_or(QrCorrection::Medium));
        format!("{module}::Data::with_version({data}, {version}, {correction})")
    } else if let Some(value) = correction {
        format!(
            "{module}::Data::with_error_correction({data}, {})",
            correction_code(value)
        )
    } else {
        format!("{module}::Data::new({data})")
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
