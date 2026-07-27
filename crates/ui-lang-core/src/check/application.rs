use super::*;

pub(in crate::check) fn check_app_settings(
    document: &Document,
    states: &HashMap<String, Type>,
) -> Result<(), Error> {
    let mut callback_states = states.clone();
    if document.daemon {
        callback_states.insert("window".into(), Type::WindowId);
    }
    for setting in [&document.settings.background, &document.settings.text_color]
        .into_iter()
        .flatten()
    {
        require_type(
            &expr_type(&setting.value, states, document, &setting.span)?,
            &Type::Str,
            &setting.span,
        )?;
    }
    if let Some(setting) = &document.settings.title {
        require_type(
            &expr_type(&setting.value, &callback_states, document, &setting.span)?,
            &Type::Str,
            &setting.span,
        )?;
    }
    if let Some(setting) = &document.settings.theme {
        if let Expr::Call { name, args } = &setting.value
            && let Some(factory) = document
                .functions
                .iter()
                .find(|function| function.name == *name && function.kind == ExternKind::Theme)
        {
            check_call_args(factory, args, &callback_states, document, &setting.span)?;
        } else {
            require_type(
                &expr_type(&setting.value, &callback_states, document, &setting.span)?,
                &Type::Str,
                &setting.span,
            )?;
        }
    }
    if let Some(setting) = &document.settings.palette {
        require_type(
            &expr_type(&setting.value, &callback_states, document, &setting.span)?,
            &Type::Str,
            &setting.span,
        )?;
        if let Expr::Str(value) = &setting.value
            && !document
                .palettes
                .iter()
                .any(|palette| palette.name == *value)
        {
            return Err(Error::new(
                "E015",
                &setting.span,
                format!("unknown palette `{value}`"),
            ));
        }
    }
    if let Some(setting) = &document.settings.scale_factor {
        require_type(
            &expr_type(&setting.value, &callback_states, document, &setting.span)?,
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
