use super::*;

#[derive(Default)]
pub(in crate::codegen) struct Style {
    pub(in crate::codegen) width_fill: bool,
    pub(in crate::codegen) height_fill: bool,
    pub(in crate::codegen) max_width: Option<u16>,
    pub(in crate::codegen) padding: [u16; 4],
    pub(in crate::codegen) gap: Option<u16>,
    pub(in crate::codegen) items_center: bool,
    pub(in crate::codegen) self_center: bool,
    pub(in crate::codegen) clip: bool,
    pub(in crate::codegen) text_size: Option<f32>,
    pub(in crate::codegen) text_line_height: Option<f32>,
    pub(in crate::codegen) font_monospace: bool,
    pub(in crate::codegen) font_weight: Option<StyleFontWeight>,
    pub(in crate::codegen) text_color: Option<String>,
    pub(in crate::codegen) background: Option<String>,
    pub(in crate::codegen) hover_background: Option<String>,
    pub(in crate::codegen) pressed_background: Option<String>,
    pub(in crate::codegen) border_color: Option<String>,
    pub(in crate::codegen) focus_border_color: Option<String>,
    pub(in crate::codegen) border_width: u16,
    pub(in crate::codegen) radius: u16,
    pub(in crate::codegen) disabled_opacity: Option<f32>,
}

#[derive(Clone, Copy)]
pub(in crate::codegen) enum StyleFontWeight {
    Medium,
    Semibold,
    Bold,
}

impl StyleFontWeight {
    pub(in crate::codegen) fn code(self) -> &'static str {
        match self {
            Self::Medium => "Medium",
            Self::Semibold => "Semibold",
            Self::Bold => "Bold",
        }
    }
}

impl Style {
    pub(in crate::codegen) fn parse(tokens: &[String], document: &Document) -> Self {
        let mut style = Self::default();
        for token in document.expand_styles(tokens) {
            let (variant, utility) = token
                .split_once(':')
                .map_or((None, token.as_str()), |(a, b)| (Some(a), b));
            if variant == Some("hover") && utility.starts_with("bg-") {
                style.hover_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("pressed") && utility.starts_with("bg-") {
                style.pressed_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("focus") && utility.starts_with("border-") {
                style.focus_border_color = Some(utility[7..].into());
                continue;
            }
            if variant == Some("disabled") && utility.starts_with("opacity-") {
                style.disabled_opacity =
                    utility[8..].parse::<f32>().ok().map(|value| value / 100.0);
                continue;
            }
            if variant.is_some() {
                continue;
            }
            match utility {
                "w-full" => style.width_fill = true,
                "h-full" => style.height_fill = true,
                "max-w-sm" => style.max_width = Some(384),
                "max-w-md" => style.max_width = Some(448),
                "max-w-lg" => style.max_width = Some(512),
                "max-w-xl" => style.max_width = Some(576),
                "max-w-2xl" => style.max_width = Some(672),
                "items-center" => style.items_center = true,
                "self-center" => style.self_center = true,
                "overflow-hidden" => style.clip = true,
                "text-xs" => style.text_size = Some(12.0),
                "text-sm" => style.text_size = Some(14.0),
                "text-base" => style.text_size = Some(16.0),
                "text-lg" => style.text_size = Some(18.0),
                "text-xl" => style.text_size = Some(20.0),
                "text-2xl" => style.text_size = Some(24.0),
                "leading-tight" => style.text_line_height = Some(1.2),
                "leading-snug" => style.text_line_height = Some(1.35),
                "leading-normal" => style.text_line_height = Some(1.5),
                "leading-relaxed" => style.text_line_height = Some(1.65),
                "font-mono" => style.font_monospace = true,
                "font-medium" => style.font_weight = Some(StyleFontWeight::Medium),
                "font-semibold" => style.font_weight = Some(StyleFontWeight::Semibold),
                "font-bold" => style.font_weight = Some(StyleFontWeight::Bold),
                "border" => style.border_width = 1,
                "border-2" => style.border_width = 2,
                "rounded-sm" => style.radius = 2,
                "rounded" | "rounded-md" => style.radius = 6,
                "rounded-lg" => style.radius = 10,
                "rounded-full" => style.radius = 999,
                _ if utility.starts_with("text-") && exact_text_size(&utility[5..]).is_some() => {
                    style.text_size = exact_text_size(&utility[5..]);
                }
                _ if utility.starts_with("rounded-") => {
                    style.radius = exact_radius(&utility[8..]).unwrap_or_default()
                }
                _ if utility.starts_with("gap-") => style.gap = spacing(&utility[4..]),
                _ if utility.starts_with("p-") => {
                    if let Some(value) = spacing(&utility[2..]) {
                        style.padding = [value; 4];
                    }
                }
                _ if utility.starts_with("px-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[1] = value;
                        style.padding[3] = value;
                    }
                }
                _ if utility.starts_with("py-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[0] = value;
                        style.padding[2] = value;
                    }
                }
                _ if utility.starts_with("bg-") => style.background = Some(utility[3..].into()),
                _ if utility.starts_with("text-") => style.text_color = Some(utility[5..].into()),
                _ if utility.starts_with("border-") => {
                    style.border_color = Some(utility[7..].into())
                }
                _ => {}
            }
        }
        style
    }

    pub(in crate::codegen) fn padding_code(&self) -> Option<String> {
        (self.padding != [0; 4]).then(|| {
            format!(
                "::iced::Padding {{ top: {}.0, right: {}.0, bottom: {}.0, left: {}.0 }}",
                self.padding[0], self.padding[1], self.padding[2], self.padding[3]
            )
        })
    }
}

pub(in crate::codegen) fn append_size(code: &mut String, style: &Style) {
    if style.width_fill {
        code.push_str(".width(::iced::Fill)");
    }
    if style.height_fill {
        code.push_str(".height(::iced::Fill)");
    }
}

pub(in crate::codegen) fn container_style_code(style: &Style, document: &Document) -> String {
    container_style_value(style, document)
        .map(|style| format!(".style(move |_| {style})"))
        .unwrap_or_default()
}

pub(in crate::codegen) fn container_style_value(
    style: &Style,
    document: &Document,
) -> Option<String> {
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
        .map(|color| format!("Some({}.into())", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let text = style
        .text_color
        .as_ref()
        .map(|color| format!("Some({})", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let border = style
        .border_color
        .as_ref()
        .map(|color| theme_color(document, color))
        .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
    Some(format!(
        "::iced::widget::container::Style {{ background: {background}, text_color: {text}, border: ::iced::Border {{ color: {border}, width: {}.0, radius: {}.0.into() }}, ..::iced::widget::container::Style::default() }}",
        style.border_width, style.radius
    ))
}

pub(in crate::codegen) fn button_style_code(
    style: &Style,
    typed: &ButtonStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let has_utilities = style.background.is_some()
        || style.hover_background.is_some()
        || style.pressed_background.is_some()
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
        let normal = style
            .background
            .as_ref()
            .map(|color| theme_color(document, color));
        let hover = style
            .hover_background
            .as_ref()
            .map(|color| theme_color(document, color))
            .or_else(|| normal.clone());
        let pressed = style
            .pressed_background
            .as_ref()
            .map(|color| theme_color(document, color))
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
                theme_color(document, text)
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
                theme_color(document, border)
            )
            .unwrap();
        }
        if style.radius > 0 {
            write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
        }
        if style.background.is_some()
            || style.text_color.is_some()
            || style.disabled_opacity.is_some()
        {
            let disabled = style.disabled_opacity.unwrap_or(0.5);
            write!(code, " if matches!(__status, ::iced::widget::button::Status::Disabled) {{ __style.text_color.a *= {disabled}; if let Some(::iced::Background::Color(mut __color)) = __style.background {{ __color.a *= {disabled}; __style.background = Some(::iced::Background::Color(__color)); }} }}").unwrap();
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
    env: &HashMap<String, Binding>,
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

pub(in crate::codegen) fn theme_preset_code(
    preset: &ThemePreset,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match preset {
        ThemePreset::Default => "::std::option::Option::None".into(),
        ThemePreset::App => "::std::option::Option::Some(__ice_app_theme.clone())".into(),
        ThemePreset::BuiltIn(name) => format!(
            "::std::option::Option::Some(::iced::Theme::{})",
            pascal(name)
        ),
        ThemePreset::Factory(factory) => format!(
            "::std::option::Option::Some({})",
            theme_factory_code(&factory.function, &factory.args, env, document)?
        ),
    })
}

pub(in crate::codegen) fn theme_factory_code(
    name: &str,
    args: &[Expr],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let function = find_extern_function(document, name, ExternKind::Theme)
        .expect("checker validates theme factories");
    let args = expr_list_code(args, env, document)?;
    Ok(format!("{}({args})", function.rust_path))
}

/// Encodes a QR payload where it is rendered, never once in application state:
/// the payload is an expression, so a matrix built at startup would be stale
/// the moment the expression changes.
pub(in crate::codegen) fn qr_data_code(
    payload: &Expr,
    correction: Option<QrCorrection>,
    version: Option<QrVersion>,
    env: &HashMap<String, Binding>,
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

pub(in crate::codegen) fn spacing(value: &str) -> Option<u16> {
    exact_pixels(value).or_else(|| {
        value
            .parse::<u16>()
            .ok()
            .and_then(|value| value.checked_mul(4))
    })
}

fn exact_pixels(value: &str) -> Option<u16> {
    value.strip_suffix("px")?.parse().ok()
}

fn exact_text_size(value: &str) -> Option<f32> {
    let value = value.strip_suffix("px")?.parse::<f32>().ok()?;
    (value.is_finite() && value > 0.0).then_some(value)
}

fn exact_radius(value: &str) -> Option<u16> {
    exact_pixels(value).filter(|value| *value > 0)
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
