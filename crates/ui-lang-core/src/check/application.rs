use super::expr::analyze_expr_types;
use super::facts::CheckedAnalyses;
use super::*;

pub(in crate::check) fn check_app_settings(
    document: &Document,
    states: &HashMap<String, Type>,
    analyses: &mut CheckedAnalyses,
) -> Result<(), Error> {
    let mut callback_states = states.clone();
    if document.daemon {
        callback_states.insert("window".into(), Type::WindowId);
    }
    for (id, setting) in [
        (
            crate::hir::AppSettingExprId::Background,
            &document.settings.background,
        ),
        (
            crate::hir::AppSettingExprId::TextColor,
            &document.settings.text_color,
        ),
    ]
    .into_iter()
    .filter_map(|(id, setting)| setting.as_ref().map(|setting| (id, setting)))
    {
        let analysis = analyze_expr_types(&setting.value, states, document, &setting.span)?;
        require_type(
            analysis.type_of(&setting.value).ok_or_else(|| {
                Error::new("E196", &setting.span, "missing checked app color type")
            })?,
            &Type::Str,
            &setting.span,
        )?;
        analyses.insert_expression(CheckedExprOwner::AppSetting(id), analysis)?;
    }
    if let Some(setting) = &document.settings.title {
        let analysis =
            analyze_expr_types(&setting.value, &callback_states, document, &setting.span)?;
        require_type(
            analysis.type_of(&setting.value).ok_or_else(|| {
                Error::new("E196", &setting.span, "missing checked app title type")
            })?,
            &Type::Str,
            &setting.span,
        )?;
        analyses.insert_expression(
            CheckedExprOwner::AppSetting(crate::hir::AppSettingExprId::Title),
            analysis,
        )?;
    }
    if let Some(setting) = &document.settings.theme {
        if let Expr::Call { name, args } = &setting.value
            && let Some(factory) = document
                .functions
                .iter()
                .find(|function| function.name == *name && function.kind == ExternKind::Theme)
        {
            if args.len() != factory.params.len() {
                return Err(Error::new(
                    "E142",
                    &setting.span,
                    format!(
                        "extern `{}` expects {} arguments, got {}",
                        factory.name,
                        factory.params.len(),
                        args.len()
                    ),
                ));
            }
            for (index, (arg, (_, expected))) in args.iter().zip(&factory.params).enumerate() {
                let analysis = analyze_expr_types(arg, &callback_states, document, &setting.span)?;
                require_type(
                    analysis.type_of(arg).ok_or_else(|| {
                        Error::new(
                            "E196",
                            &setting.span,
                            "missing checked app theme factory argument type",
                        )
                    })?,
                    expected,
                    &setting.span,
                )?;
                analyses.insert_expression(
                    CheckedExprOwner::AppSetting(
                        crate::hir::AppSettingExprId::ThemeFactoryArgument(index as u32),
                    ),
                    analysis,
                )?;
            }
        } else {
            let analysis =
                analyze_expr_types(&setting.value, &callback_states, document, &setting.span)?;
            require_type(
                analysis.type_of(&setting.value).ok_or_else(|| {
                    Error::new("E196", &setting.span, "missing checked app theme type")
                })?,
                &Type::Str,
                &setting.span,
            )?;
            analyses.insert_expression(
                CheckedExprOwner::AppSetting(crate::hir::AppSettingExprId::Theme),
                analysis,
            )?;
        }
    }
    if let Some(setting) = &document.settings.palette {
        let contract = document
            .theme_contract
            .as_ref()
            .expect("theme contract is checked before app settings");
        let analysis =
            analyze_expr_types(&setting.value, &callback_states, document, &setting.span)?;
        require_type(
            analysis.type_of(&setting.value).ok_or_else(|| {
                Error::new("E196", &setting.span, "missing checked app palette type")
            })?,
            &Type::Palette(contract.name.clone()),
            &setting.span,
        )?;
        analyses.insert_expression(
            CheckedExprOwner::AppSetting(crate::hir::AppSettingExprId::Palette),
            analysis,
        )?;
    }
    if let Some(setting) = &document.settings.scale_factor {
        let analysis =
            analyze_expr_types(&setting.value, &callback_states, document, &setting.span)?;
        require_type(
            analysis.type_of(&setting.value).ok_or_else(|| {
                Error::new("E196", &setting.span, "missing checked app scale type")
            })?,
            &Type::F64,
            &setting.span,
        )?;
        if f64_literal(&setting.value).is_some_and(|value| value <= 0.0) {
            return Err(Error::new(
                "E015",
                &setting.span,
                "scale must be greater than zero",
            ));
        }
        require_f32_literal_range(&setting.value, 0.0, None, "scale", &setting.span)?;
        analyses.insert_expression(
            CheckedExprOwner::AppSetting(crate::hir::AppSettingExprId::ScaleFactor),
            analysis,
        )?;
    }
    if let Some(AppExpression {
        value: Expr::Str(value),
        span,
    }) = &document.settings.theme
        && value != "app"
        && value != "default"
        && !BUILT_IN_THEMES.contains(&value.as_str())
    {
        return Err(Error::new(
            "E015",
            span,
            format!("unknown iced theme `{value}`"),
        ));
    }
    for setting in [&document.settings.background, &document.settings.text_color]
        .into_iter()
        .flatten()
    {
        if let Expr::Str(value) = &setting.value
            && !valid_app_color(value)
        {
            return Err(Error::new(
                "E015",
                &setting.span,
                "application colors must be 3, 4, 6, or 8 digit hexadecimal strings",
            ));
        }
    }
    Ok(())
}

pub(in crate::check) fn valid_app_color(value: &str) -> bool {
    let hex = value.strip_prefix('#').unwrap_or(value);
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|value| value.is_ascii_hexdigit())
}
