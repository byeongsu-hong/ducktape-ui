use super::*;

pub(in crate::codegen) fn render_rule(
    rule: &ResolvedRule,
    document: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let program = document.hir();
    let thickness = resolved_expr_use_code(program, rule.thickness, env, ValueMode::Owned)?;
    let axis = match rule.axis {
        ResolvedRuleAxis::Horizontal => "horizontal",
        ResolvedRuleAxis::Vertical => "vertical",
    };
    let mut code = format!("::iced::widget::rule::{axis}({thickness} as f32)");
    code.push_str(&resolved_rule_style_code(rule, program, env)?);
    Ok(format!("{code}.into()"))
}

pub(in crate::codegen) fn render_qr_code(
    qr: &ResolvedQrCode,
    document: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let program = document.hir();
    let payload = resolved_expr_use_code(program, qr.payload, env, ValueMode::Borrowed)?;
    let data = resolved_qr_data_code(qr, &payload);
    let mut code = format!("::ui_lang_runtime::qr_code({data}.ok())");
    match qr.size {
        ResolvedQrSize::Default => {}
        // QR version 40 is at most 177 modules wide; iced adds its quiet zone,
        // so 182 is the safe native cell-size divisor used by the runtime.
        ResolvedQrSize::Cell(value) => write!(
            code,
            ".cell_size(::ui_lang_runtime::bounded_spacing({}, 182))",
            resolved_expr_use_code(program, value, env, ValueMode::Owned)?
        )
        .unwrap(),
        ResolvedQrSize::Total(value) => write!(
            code,
            ".total_size(::ui_lang_runtime::bounded_spacing({}, 3))",
            resolved_expr_use_code(program, value, env, ValueMode::Owned)?
        )
        .unwrap(),
    }
    if qr.cell.is_some() || qr.background.is_some() {
        let cell = qr.cell.as_ref().map(resolved_theme_color);
        let background = qr.background.as_ref().map(resolved_theme_color);
        let (theme, default) = if cell.is_none() || background.is_none() {
            (
                "theme",
                "let default = ::iced::widget::qr_code::default(theme); ",
            )
        } else {
            ("_theme", "")
        };
        write!(
            code,
            ".style(move |{theme}| {{ {default}::iced::widget::qr_code::Style {{ cell: {}, background: {} }} }})",
            cell.unwrap_or_else(|| "default.cell".into()),
            background.unwrap_or_else(|| "default.background".into())
        )
        .unwrap();
    }
    Ok(format!("{code}.into()"))
}

pub(in crate::codegen) fn render_space(
    space: &ResolvedSpace,
    document: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let program = document.hir();
    let mut code = String::from("::iced::widget::space()");
    for (method, length) in [
        ("width", space.width.as_ref()),
        ("height", space.height.as_ref()),
    ] {
        if let Some(length) = length {
            write!(
                code,
                ".{method}({})",
                resolved_text_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into()"))
}

fn resolved_rule_style_code(
    rule: &ResolvedRule,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let radius = resolved_text_radius_code(&rule.radius, program, env)?;
    if rule.preset == ResolvedRulePreset::Default
        && rule.fill.is_none()
        && rule.color.is_none()
        && radius.is_none()
        && rule.snap.is_none()
    {
        return Ok(String::new());
    }
    let preset = match rule.preset {
        ResolvedRulePreset::Default => "default",
        ResolvedRulePreset::Weak => "weak",
    };
    let mut code = format!(
        ".style(move |__theme| {{ let mut __style = ::iced::widget::rule::{preset}(__theme);"
    );
    if let Some(fill) = &rule.fill {
        let fill = match fill {
            ResolvedRuleFill::Full => "::iced::widget::rule::FillMode::Full".into(),
            ResolvedRuleFill::Percent(value) => {
                let value = resolved_expr_use_code(program, *value, env, ValueMode::Owned)?;
                format!(
                    "::iced::widget::rule::FillMode::Percent((({value}) as f32).max(0.0).min(100.0))"
                )
            }
            ResolvedRuleFill::Padded(value) => {
                format!("::iced::widget::rule::FillMode::Padded({value})")
            }
            ResolvedRuleFill::AsymmetricPadding(first, second) => {
                format!("::iced::widget::rule::FillMode::AsymmetricPadding({first}, {second})")
            }
        };
        write!(code, " __style.fill_mode = {fill};").unwrap();
    }
    if let Some(color) = &rule.color {
        write!(code, " __style.color = {};", resolved_theme_color(color)).unwrap();
    }
    if let Some(radius) = radius {
        write!(code, " __style.radius = {radius};").unwrap();
    }
    if let Some(snap) = rule.snap {
        write!(
            code,
            " __style.snap = {};",
            resolved_expr_use_code(program, snap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(code)
}

fn resolved_qr_data_code(qr: &ResolvedQrCode, payload: &str) -> String {
    let module = "::iced::widget::qr_code";
    let data = format!("&({payload})");
    match qr.encoding {
        ResolvedQrEncoding::Auto { correction: None } => {
            format!("{module}::Data::new({data})")
        }
        ResolvedQrEncoding::Auto {
            correction: Some(correction),
        } => format!(
            "{module}::Data::with_error_correction({data}, {})",
            resolved_qr_correction_code(correction)
        ),
        ResolvedQrEncoding::Versioned {
            version,
            correction,
        } => {
            let version = match version {
                ResolvedQrVersion::Normal(value) => {
                    format!("{module}::Version::Normal({value})")
                }
                ResolvedQrVersion::Micro(value) => {
                    format!("{module}::Version::Micro({value})")
                }
            };
            format!(
                "{module}::Data::with_version({data}, {version}, {})",
                resolved_qr_correction_code(correction)
            )
        }
    }
}

fn resolved_qr_correction_code(correction: ResolvedQrCorrection) -> &'static str {
    match correction {
        ResolvedQrCorrection::Low => "::iced::widget::qr_code::ErrorCorrection::Low",
        ResolvedQrCorrection::Medium => "::iced::widget::qr_code::ErrorCorrection::Medium",
        ResolvedQrCorrection::Quartile => "::iced::widget::qr_code::ErrorCorrection::Quartile",
        ResolvedQrCorrection::High => "::iced::widget::qr_code::ErrorCorrection::High",
    }
}
