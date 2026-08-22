use super::expr::analyze_expr_types;
use super::facts::CheckedAnalyses;
use super::*;

pub(in crate::check) fn check_app_settings(
    document: &Document,
    states: &dyn ExprTypeEnv,
    analyses: &mut CheckedAnalyses,
) -> Result<(), Error> {
    let mut callback_states = ScopedTypeEnv::new(states);
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
    if let Some(tray) = &document.settings.tray {
        check_tray(tray, document, states, analyses)?;
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

/// Type-checks every tray expression and enforces the rules that make the
/// icon fold total and the menu addressable.
fn check_tray(
    tray: &TraySettings,
    document: &Document,
    states: &dyn ExprTypeEnv,
    analyses: &mut CheckedAnalyses,
) -> Result<(), Error> {
    let mut typed = |id, setting: &AppExpression, expected: &Type, what: &str| {
        let analysis = analyze_expr_types(&setting.value, states, document, &setting.span)?;
        require_type(
            analysis
                .type_of(&setting.value)
                .ok_or_else(|| Error::new("E196", &setting.span, format!("missing {what}")))?,
            expected,
            &setting.span,
        )?;
        analyses.insert_expression(CheckedExprOwner::AppSetting(id), analysis)
    };
    // A literal never enters the checked-expression world: it is applied once
    // at startup, so there is nothing for the rest of the pipeline to resolve.
    for (id, setting) in [
        (crate::hir::AppSettingExprId::TrayLabel, &tray.label),
        (crate::hir::AppSettingExprId::TrayTooltip, &tray.tooltip),
    ]
    .into_iter()
    .filter_map(|(id, setting)| setting.as_ref().map(|setting| (id, setting)))
    .filter(|(_, setting)| tray_text_is_reactive(setting))
    {
        typed(id, setting, &Type::Str, "checked tray text type")?;
    }
    for (index, icon) in tray.icons.iter().enumerate() {
        if let Some(guard) = &icon.when {
            typed(
                crate::hir::AppSettingExprId::TrayIconGuard(index as u32),
                guard,
                &Type::Bool,
                "checked tray icon guard type",
            )?;
        }
    }
    for (index, row) in tray.menu.iter().enumerate() {
        let TrayRow::Item { text, when, .. } = row else {
            continue;
        };
        if tray_text_is_reactive(text) {
            typed(
                crate::hir::AppSettingExprId::TrayMenuRow(index as u32),
                text,
                &Type::Str,
                "checked tray menu row type",
            )?;
        }
        if let Some(guard) = when {
            typed(
                crate::hir::AppSettingExprId::TrayRowGuard(index as u32),
                guard,
                &Type::Bool,
                "checked tray row guard type",
            )?;
        }
    }
    // Guards are tried in declaration order and the first match wins, so the
    // last line is what applies when none does. Requiring it to be unguarded
    // is what makes the fold total: codegen never emits a fallible selection
    // and no author can write a tray with no icon to show.
    let last = tray.icons.len().saturating_sub(1);
    for (index, icon) in tray.icons.iter().enumerate() {
        if index == last && icon.when.is_some() {
            return Err(Error::new(
                "E015",
                &icon.icon.span,
                format!(
                    "tray icon-rgba `{}` is guarded but is the last one",
                    icon.icon.path
                ),
            )
            .hint(
                "the last `icon-rgba` selects when no guard matches, so it cannot carry `when`",
            ));
        }
        if index != last && icon.when.is_none() {
            return Err(Error::new(
                "E015",
                &icon.icon.span,
                format!(
                    "tray icon-rgba `{}` before the last one needs `when`",
                    icon.icon.path
                ),
            )
            .hint("guards are tried in order and the first match wins"));
        }
        if tray.icons[..index]
            .iter()
            .any(|earlier| earlier.icon.path == icon.icon.path)
        {
            return Err(Error::new(
                "E014",
                &icon.icon.span,
                format!("duplicate tray icon `{}`", icon.icon.path),
            )
            .hint("the path names the icon in `expect tray icon`"));
        }
    }
    for row in &tray.menu {
        let TrayRow::Item {
            route: Some(route),
            nested,
            span,
            ..
        } = row
        else {
            continue;
        };
        // A submenu is opened, not chosen. The platform gives its row no
        // activation to deliver, so a route on one would name a handler
        // nothing could ever reach.
        if *nested > 0 {
            return Err(
                Error::new("E015", span, "a tray submenu row cannot name a route").hint(
                    "the platform opens a submenu instead of activating it; route its rows instead",
                ),
            );
        }
        if !document
            .handlers
            .iter()
            .any(|handler| handler.name == *route && handler.params.is_empty())
        {
            return Err(
                Error::new("E173", span, format!("unknown handler `{route}`"))
                    .hint("a menu row calls a handler that takes no parameters"),
            );
        }
    }
    Ok(())
}

pub(in crate::check) fn valid_app_color(value: &str) -> bool {
    let hex = value.strip_prefix('#').unwrap_or(value);
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|value| value.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fmt::Write as _;

    struct CountingTypeEnv {
        types: HashMap<String, Type>,
        plain_setting_lookups: Cell<u8>,
        window_lookups: Cell<usize>,
        visited_entries: Cell<usize>,
    }

    impl ExprTypeEnv for CountingTypeEnv {
        fn get_type(&self, name: &str) -> Option<&Type> {
            let plain_setting = match name {
                "background" => 1,
                "foreground" => 2,
                _ => 0,
            };
            self.plain_setting_lookups
                .set(self.plain_setting_lookups.get() | plain_setting);
            if name == "window" {
                self.window_lookups.set(self.window_lookups.get() + 1);
            }
            self.types.get(name)
        }

        fn visit_types(&self, visitor: &mut dyn FnMut(&str, &Type)) {
            self.visited_entries
                .set(self.visited_entries.get() + self.types.len());
            for (name, ty) in &self.types {
                visitor(name, ty);
            }
        }

        fn type_with_prefix(&self, prefix: &str) -> Option<&Type> {
            self.types
                .iter()
                .find_map(|(name, ty)| name.starts_with(prefix).then_some(ty))
        }
    }

    #[test]
    fn daemon_callback_settings_borrow_and_shadow_the_state_environment() {
        const FILLER_STATES: usize = 256;
        let mut source = String::from(
            r#"daemon ScopedSettings
  title describe(window, label)
  theme native_theme(window, dark)
  bg background
  fg foreground
  scale scale_for(window, zoom)
extern crate::backend
  pure describe(id:window-id, label:str) -> str
  pure scale_for(id:window-id, zoom:f64) -> f64
  theme native_theme(id:window-id, dark:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  label = "Scoped"
  dark = false
  background = "000000"
  foreground = "ffffff"
  zoom = 1.25
"#,
        );
        for index in 0..FILLER_STATES {
            writeln!(source, "  filler_{index} = {index}").unwrap();
        }
        source.push_str("secret window\nview\n  text label\n");

        let document = crate::parse(&source).unwrap();
        let mut types = document
            .states
            .iter()
            .map(|state| (state.name.clone(), state.ty.clone()))
            .collect::<HashMap<_, _>>();
        types.extend(
            document
                .secrets
                .iter()
                .map(|secret| (secret.name.clone(), Type::Secret)),
        );
        let env = CountingTypeEnv {
            types,
            plain_setting_lookups: Cell::new(0),
            window_lookups: Cell::new(0),
            visited_entries: Cell::new(0),
        };
        assert_eq!(env.types.len(), FILLER_STATES + 6);
        assert_eq!(env.types.get("window"), Some(&Type::Secret));

        check_app_settings(&document, &env, &mut CheckedAnalyses::default()).unwrap();

        assert_eq!(
            env.plain_setting_lookups.get(),
            3,
            "background and foreground must use the base environment"
        );
        assert_eq!(
            env.window_lookups.get(),
            0,
            "the daemon callback-local `window` must shadow the secret"
        );
        assert_eq!(
            env.visited_entries.get(),
            0,
            "checking callback settings copied the {}-entry environment",
            env.types.len()
        );
    }
}
