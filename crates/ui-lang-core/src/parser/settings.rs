use super::*;

pub(in crate::parser) fn parse_preset(name: &str, line: &Line) -> Result<Preset, Error> {
    let name = identifier(name, line)?;
    let mut statements = Vec::new();
    let mut state = false;
    let mut boot = false;
    for section in &line.children {
        match section.text.as_str() {
            "state" if !state && !boot => {
                state = true;
                for item in &section.children {
                    match parse_statement(item)? {
                        statement @ Statement::Assign { .. } => statements.push(statement),
                        _ => {
                            return Err(error(
                                "E016",
                                item,
                                "preset state only accepts `name = value` overrides",
                            ));
                        }
                    }
                }
            }
            "boot" if !boot => {
                boot = true;
                statements.extend(
                    section
                        .children
                        .iter()
                        .map(parse_statement)
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            "state" if boot => {
                return Err(error("E016", section, "preset state must precede boot"));
            }
            "state" | "boot" => {
                return Err(error(
                    "E016",
                    section,
                    format!("duplicate preset section `{}`", section.text),
                ));
            }
            _ => {
                return Err(error(
                    "E016",
                    section,
                    "preset accepts `state` and `boot` sections",
                ));
            }
        }
    }
    Ok(Preset {
        name,
        statements,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_app_settings(line: &Line) -> Result<AppSettings, Error> {
    let mut settings = AppSettings::default();
    for item in &line.children {
        if item.text == "window" {
            if settings.window.is_some() {
                return Err(error("E014", item, "duplicate app setting `window`"));
            }
            settings.window = Some(parse_window_settings(item)?);
            continue;
        }
        if let Some(name) = item.text.strip_prefix("window ") {
            let name = identifier(name.trim(), item)?;
            if settings.windows.iter().any(|window| window.name == name) {
                return Err(error(
                    "E014",
                    item,
                    format!("duplicate app window `{name}`"),
                ));
            }
            settings.windows.push(NamedWindow {
                name,
                settings: parse_window_settings(item)?,
            });
            continue;
        }
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error(
                "E014",
                item,
                format!("unknown app setting `{}`", item.text),
            ));
        };
        let value = value.trim();
        macro_rules! set {
            ($field:ident, $value:expr) => {
                set_setting(&mut settings.$field, $value, name, item)?
            };
        }
        match name {
            "title" => set!(title, app_expression(value, item)?),
            "theme" => set!(theme, app_expression(value, item)?),
            "palette" => set!(palette, app_expression(value, item)?),
            "bg" => set!(background, app_expression(value, item)?),
            "fg" => set!(text_color, app_expression(value, item)?),
            "id" => set!(id, string_literal(value, item)?),
            "executor" => set!(executor, rust_path(value, item)?),
            "renderer" => set!(renderer, rust_path(value, item)?),
            "font" => {
                let path = string_literal(value, item)?;
                if path.is_empty()
                    || path.contains('\\')
                    || crate::has_windows_drive_prefix(&path)
                    || std::path::Path::new(&path).is_absolute()
                {
                    return Err(error(
                        "E015",
                        item,
                        "font paths must be non-empty relative `/` paths",
                    ));
                }
                if settings.fonts.iter().any(|font| font.path == path) {
                    return Err(error("E014", item, format!("duplicate app font `{path}`")));
                }
                settings.fonts.push(FontAsset {
                    path,
                    span: Span::line(item.number),
                });
            }
            "text-size" => set!(default_text_size, config_positive_number(value, item)?),
            "antialiasing" => set!(antialiasing, config_bool(value, item)?),
            "vsync" => set!(vsync, config_bool(value, item)?),
            "scale" => set!(scale_factor, app_number_expression(value, item)?),
            _ => {
                return Err(error("E014", item, format!("unknown app setting `{name}`")));
            }
        }
    }
    Ok(settings)
}

pub(in crate::parser) fn app_expression(source: &str, line: &Line) -> Result<AppExpression, Error> {
    Ok(AppExpression {
        value: parse_expr(source, line)?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn app_number_expression(
    source: &str,
    line: &Line,
) -> Result<AppExpression, Error> {
    let mut expression = app_expression(source, line)?;
    if let Expr::I64(value) = &expression.value {
        expression.value = Expr::F64(*value as f64);
    }
    Ok(expression)
}

pub(in crate::parser) fn parse_window_settings(line: &Line) -> Result<WindowSettings, Error> {
    let mut settings = WindowSettings::default();
    for item in &line.children {
        if let Some(platform) = item.text.strip_prefix("platform ") {
            match platform.trim() {
                "linux" => set_setting(
                    &mut settings.linux,
                    parse_linux_window_settings(item)?,
                    "platform linux",
                    item,
                )?,
                "windows" => set_setting(
                    &mut settings.windows,
                    parse_windows_window_settings(item)?,
                    "platform windows",
                    item,
                )?,
                "macos" => set_setting(
                    &mut settings.macos,
                    parse_macos_window_settings(item)?,
                    "platform macos",
                    item,
                )?,
                "wasm" => set_setting(
                    &mut settings.wasm,
                    parse_wasm_window_settings(item)?,
                    "platform wasm",
                    item,
                )?,
                _ => {
                    return Err(error(
                        "E015",
                        item,
                        "window platform must be linux, windows, macos, or wasm",
                    ));
                }
            }
            continue;
        }
        ensure_leaf(item)?;
        let (name, value) = item
            .text
            .split_once(char::is_whitespace)
            .map_or((item.text.as_str(), ""), |(name, value)| {
                (name, value.trim())
            });
        macro_rules! set {
            ($field:ident, $value:expr) => {
                set_setting(&mut settings.$field, $value, name, item)?
            };
        }
        match name {
            "size" => set!(size, config_size(value, item)?),
            "maximized" => set!(maximized, config_bool(value, item)?),
            "fullscreen" => set!(fullscreen, config_bool(value, item)?),
            "position" => set!(position, config_position(value, item)?),
            "min-size" => set!(min_size, config_size(value, item)?),
            "max-size" => set!(max_size, config_size(value, item)?),
            "visible" => set!(visible, config_bool(value, item)?),
            "resizable" => set!(resizable, config_bool(value, item)?),
            "closeable" => set!(closeable, config_bool(value, item)?),
            "minimizable" => set!(minimizable, config_bool(value, item)?),
            "decorations" => set!(decorations, config_bool(value, item)?),
            "transparent" => set!(transparent, config_bool(value, item)?),
            "blur" => set!(blur, config_bool(value, item)?),
            "level" => set!(
                level,
                match value {
                    "normal" => WindowLevel::Normal,
                    "always-on-bottom" => WindowLevel::AlwaysOnBottom,
                    "always-on-top" => WindowLevel::AlwaysOnTop,
                    _ => {
                        return Err(error(
                            "E015",
                            item,
                            "window level must be normal, always-on-bottom, or always-on-top",
                        ));
                    }
                }
            ),
            "icon-rgba" => set!(icon, config_window_icon(value, item)?),
            "exit-on-close" => {
                set!(exit_on_close_request, config_bool(value, item)?)
            }
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown window setting `{name}`"),
                ));
            }
        }
    }
    if let (Some((min_width, min_height)), Some((max_width, max_height))) =
        (settings.min_size, settings.max_size)
        && (min_width > max_width || min_height > max_height)
    {
        return Err(error(
            "E015",
            line,
            "window min-size cannot exceed max-size",
        ));
    }
    Ok(settings)
}

pub(in crate::parser) fn parse_linux_window_settings(
    line: &Line,
) -> Result<LinuxWindowSettings, Error> {
    let mut settings = LinuxWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error("E015", item, "Linux window setting requires a value"));
        };
        let value = value.trim();
        match name {
            "app-id" => set_setting(
                &mut settings.application_id,
                string_literal(value, item)?,
                name,
                item,
            )?,
            "override-redirect" => set_setting(
                &mut settings.override_redirect,
                config_bool(value, item)?,
                name,
                item,
            )?,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown Linux window setting `{name}`"),
                ));
            }
        }
    }
    Ok(settings)
}

pub(in crate::parser) fn parse_windows_window_settings(
    line: &Line,
) -> Result<WindowsWindowSettings, Error> {
    let mut settings = WindowsWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error(
                "E015",
                item,
                "Windows window setting requires a value",
            ));
        };
        let value = value.trim();
        match name {
            "drag-and-drop" => set_setting(
                &mut settings.drag_and_drop,
                config_bool(value, item)?,
                name,
                item,
            )?,
            "skip-taskbar" => set_setting(
                &mut settings.skip_taskbar,
                config_bool(value, item)?,
                name,
                item,
            )?,
            "undecorated-shadow" => set_setting(
                &mut settings.undecorated_shadow,
                config_bool(value, item)?,
                name,
                item,
            )?,
            "corner" => set_setting(
                &mut settings.corner,
                match value {
                    "default" => WindowCorner::Default,
                    "do-not-round" => WindowCorner::DoNotRound,
                    "round" => WindowCorner::Round,
                    "round-small" => WindowCorner::RoundSmall,
                    _ => {
                        return Err(error(
                            "E015",
                            item,
                            "Windows window corner must be default, do-not-round, round, or round-small",
                        ));
                    }
                },
                name,
                item,
            )?,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown Windows window setting `{name}`"),
                ));
            }
        }
    }
    Ok(settings)
}

pub(in crate::parser) fn parse_macos_window_settings(
    line: &Line,
) -> Result<MacosWindowSettings, Error> {
    let mut settings = MacosWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error("E015", item, "macOS window setting requires a value"));
        };
        let value = value.trim();
        let slot = match name {
            "title-hidden" => &mut settings.title_hidden,
            "titlebar-transparent" => &mut settings.titlebar_transparent,
            "fullsize-content-view" => &mut settings.fullsize_content_view,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown macOS window setting `{name}`"),
                ));
            }
        };
        set_setting(slot, config_bool(value, item)?, name, item)?;
    }
    Ok(settings)
}

pub(in crate::parser) fn parse_wasm_window_settings(
    line: &Line,
) -> Result<WasmWindowSettings, Error> {
    let mut settings = WasmWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error("E015", item, "Wasm window setting requires a value"));
        };
        let value = value.trim();
        match name {
            "target" => set_setting(
                &mut settings.target,
                if value == "none" {
                    None
                } else {
                    Some(string_literal(value, item)?)
                },
                name,
                item,
            )?,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown Wasm window setting `{name}`"),
                ));
            }
        }
    }
    Ok(settings)
}

pub(in crate::parser) fn config_window_icon(
    source: &str,
    line: &Line,
) -> Result<WindowIcon, Error> {
    let parts = split_words(source);
    if parts.len() != 3 {
        return Err(error(
            "E015",
            line,
            "window icon-rgba expects `\"relative/path.rgba\" width height`",
        ));
    }
    let path = string_literal(&parts[0], line)?;
    if path.is_empty()
        || path.contains('\\')
        || crate::has_windows_drive_prefix(&path)
        || std::path::Path::new(&path).is_absolute()
    {
        return Err(error(
            "E015",
            line,
            "window icon paths must be non-empty relative `/` paths",
        ));
    }
    let dimension = |value: &str| {
        value
            .parse::<u32>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                error(
                    "E015",
                    line,
                    "window icon dimensions must be positive integers",
                )
            })
    };
    let width = dimension(&parts[1])?;
    let height = dimension(&parts[2])?;
    let byte_len = width
        .checked_mul(height)
        .and_then(|pixels| usize::try_from(pixels).ok())
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| error("E015", line, "window icon dimensions are too large"))?;
    Ok(WindowIcon {
        path,
        width,
        height,
        byte_len,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn set_setting<T>(
    slot: &mut Option<T>,
    value: T,
    name: &str,
    line: &Line,
) -> Result<(), Error> {
    if slot.replace(value).is_some() {
        Err(error("E014", line, format!("duplicate setting `{name}`")))
    } else {
        Ok(())
    }
}

pub(in crate::parser) fn config_bool(source: &str, line: &Line) -> Result<bool, Error> {
    match source {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error("E015", line, "setting expects true or false")),
    }
}

pub(in crate::parser) fn config_number(source: &str, line: &Line) -> Result<f64, Error> {
    source
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && (*value as f32).is_finite())
        .ok_or_else(|| {
            error(
                "E015",
                line,
                "setting expects a finite number within the f32 range",
            )
        })
}

pub(in crate::parser) fn config_positive_number(source: &str, line: &Line) -> Result<f64, Error> {
    let value = config_number(source, line)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(error("E015", line, "setting must be greater than zero"))
    }
}

pub(in crate::parser) fn config_pair(source: &str, line: &Line) -> Result<(f64, f64), Error> {
    let parts = split_words(source);
    if parts.len() != 2 {
        return Err(error("E015", line, "window size expects `width height`"));
    }
    Ok((
        config_number(&parts[0], line)?,
        config_number(&parts[1], line)?,
    ))
}

pub(in crate::parser) fn config_size(source: &str, line: &Line) -> Result<(f64, f64), Error> {
    let (width, height) = config_pair(source, line)?;
    if width > 0.0 && height > 0.0 {
        Ok((width, height))
    } else {
        Err(error(
            "E015",
            line,
            "window dimensions must be greater than zero",
        ))
    }
}

pub(in crate::parser) fn config_position(
    source: &str,
    line: &Line,
) -> Result<WindowPosition, Error> {
    match source {
        "default" => Ok(WindowPosition::Default),
        "centered" => Ok(WindowPosition::Centered),
        _ => {
            let (x, y) = config_pair(source, line).map_err(|_| {
                error(
                    "E015",
                    line,
                    "window position expects default, centered, or `x y`",
                )
            })?;
            Ok(WindowPosition::Specific(x, y))
        }
    }
}

pub(in crate::parser) fn parse_font(source: &str, line: &Line) -> Result<FontDecl, Error> {
    ensure_leaf(line)?;
    let parts = split_words(source);
    let source_name = identifier(
        parts
            .first()
            .ok_or_else(|| error("E013", line, "font requires a name"))?,
        line,
    )?;
    let name = line.qualify(&source_name);
    if matches!(source_name.as_str(), "default" | "mono") {
        return Err(error(
            "E013",
            line,
            format!("font name `{source_name}` is built in and cannot be redeclared"),
        ));
    }
    let mut family = FontFamily::Named(source_name);
    let mut weight = FontWeight::Normal;
    let mut stretch = FontStretch::Normal;
    let mut style = FontStyle::Normal;
    let mut default = false;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("family=") {
            family = match value {
                "serif" => FontFamily::Serif,
                "sans" => FontFamily::SansSerif,
                "cursive" => FontFamily::Cursive,
                "fantasy" => FontFamily::Fantasy,
                "mono" => FontFamily::Monospace,
                value => FontFamily::Named(string_literal(value, line)?),
            };
        } else if let Some(value) = part.strip_prefix("weight=") {
            weight = match value {
                "thin" => FontWeight::Thin,
                "extra-light" => FontWeight::ExtraLight,
                "light" => FontWeight::Light,
                "normal" => FontWeight::Normal,
                "medium" => FontWeight::Medium,
                "semibold" => FontWeight::Semibold,
                "bold" => FontWeight::Bold,
                "extra-bold" => FontWeight::ExtraBold,
                "black" => FontWeight::Black,
                _ => return Err(error("E013", line, "unknown font weight")),
            };
        } else if let Some(value) = part.strip_prefix("stretch=") {
            stretch = match value {
                "ultra-condensed" => FontStretch::UltraCondensed,
                "extra-condensed" => FontStretch::ExtraCondensed,
                "condensed" => FontStretch::Condensed,
                "semi-condensed" => FontStretch::SemiCondensed,
                "normal" => FontStretch::Normal,
                "semi-expanded" => FontStretch::SemiExpanded,
                "expanded" => FontStretch::Expanded,
                "extra-expanded" => FontStretch::ExtraExpanded,
                "ultra-expanded" => FontStretch::UltraExpanded,
                _ => return Err(error("E013", line, "unknown font stretch")),
            };
        } else if let Some(value) = part.strip_prefix("style=") {
            style = match value {
                "normal" => FontStyle::Normal,
                "italic" => FontStyle::Italic,
                "oblique" => FontStyle::Oblique,
                _ => return Err(error("E013", line, "unknown font style")),
            };
        } else if let Some(value) = part.strip_prefix("default=") {
            default = match value {
                "true" => true,
                "false" => false,
                _ => return Err(error("E013", line, "font default must be true or false")),
            };
        } else {
            return Err(error(
                "E013",
                line,
                format!("unknown font property `{part}`"),
            ));
        }
    }
    Ok(FontDecl {
        name,
        family,
        weight,
        stretch,
        style,
        default,
        span: Span::line(line.number),
    })
}
