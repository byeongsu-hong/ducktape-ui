use super::*;

pub(in crate::parser) fn parse_input(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if parts.len() < 4 {
        return Err(error(
            "E065",
            line,
            "input uses `input \"Label\" #id <-> state`",
        ));
    }
    let label = string_literal(parts[1], line)?;
    let mut id = None;
    let mut binding = None;
    let mut hint = String::new();
    let mut disabled = None;
    let mut options = InputOptions::default();
    let mut index = 2;
    while index < parts.len() {
        let part = &parts[index];
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E065", "input")?;
        } else if *part == "<->" {
            index += 1;
            let value = parts
                .get(index)
                .ok_or_else(|| error("E065", line, "missing binding after `<->`"))?;
            if binding.replace(identifier(value, line)?).is_some() {
                return Err(error("E065", line, "input has more than one binding"));
            }
        } else if let Some(value) = part.strip_prefix("hint=") {
            hint = string_literal(value, line)?;
        } else if parse_accessibility_option(part, &mut options.accessibility, line)? {
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("secure=") {
            options.secure = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("change=") {
            options.change = Some(parse_payload_route(value, line, 1)?);
        } else if let Some(value) = part.strip_prefix("submit=") {
            options.submit = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("paste=") {
            options.paste = Some(parse_payload_route(value, line, 1)?);
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("p=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text-size=") {
            options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-h=") {
            options.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("align=") {
            options.align =
                Some(value.parse().map_err(|()| {
                    error("E065", line, "input align must be left, center, or right")
                })?);
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = Some(parse_extern_call(
                value,
                line,
                "E065",
                "input style must be a declared style call",
            )?);
        } else {
            return Err(error(
                "E065",
                line,
                format!("unknown input property `{part}`"),
            ));
        }
        index += 1;
    }
    for child in &line.children {
        let parts = split_words(&child.text);
        match parts.first().copied() {
            Some("active" | "hovered" | "focused" | "focused-hovered" | "disabled") => {
                ensure_leaf(child)?;
                parse_text_input_status(&parts, child, &mut options.style, "E065", "input", true)?;
            }
            Some("icon") => {
                ensure_leaf(child)?;
                if options.icon.is_some() {
                    return Err(error("E065", child, "duplicate input icon"));
                }
                options.icon = Some(parse_text_input_icon(&parts[1..], child, "E065", "input")?);
            }
            _ => {
                return Err(error(
                    "E065",
                    child,
                    "input blocks use active, hovered, focused, focused-hovered, disabled, or icon",
                ));
            }
        }
    }
    Ok(ViewNode::Input {
        label,
        id,
        binding: binding.ok_or_else(|| error("E065", line, "input requires `<-> state`"))?,
        hint,
        disabled,
        options,
        styles,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_button(
    parts: &[&str],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .filter(|part| part.starts_with('"'))
        .map(|part| string_literal(part, line))
        .transpose()?;
    let mut id = None;
    let mut disabled = None;
    let mut options = ButtonOptions::default();
    let option_start = if label.is_some() { 2 } else { 1 };
    for part in &parts[option_start..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E066", "button")?;
        } else if parse_accessibility_option(part, &mut options.accessibility, line)? {
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("checked=") {
            options.checked = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("expanded=") {
            options.expanded = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("p=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("clip=") {
            options.clip = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            if let Some(preset) = match value {
                "primary" => Some(ButtonStylePreset::Primary),
                "secondary" => Some(ButtonStylePreset::Secondary),
                "success" => Some(ButtonStylePreset::Success),
                "warning" => Some(ButtonStylePreset::Warning),
                "danger" => Some(ButtonStylePreset::Danger),
                "text" => Some(ButtonStylePreset::Text),
                "bg" => Some(ButtonStylePreset::Background),
                "subtle" => Some(ButtonStylePreset::Subtle),
                _ => None,
            } {
                options.style.preset = preset;
                options.style.custom = None;
            } else {
                options.style.custom = Some(parse_extern_call(
                    value,
                    line,
                    "E066",
                    "button style must be a preset or declared style call",
                )?);
            }
        } else {
            return Err(error(
                "E066",
                line,
                format!("unknown button property `{part}`"),
            ));
        }
    }
    let mut content = None;
    for child in &line.children {
        let parts = split_words(&child.text);
        if parts
            .first()
            .is_some_and(|part| matches!(*part, "active" | "hovered" | "pressed" | "disabled"))
        {
            parse_button_status_style(child, &mut options.style)?;
        } else {
            if content.is_some() {
                return Err(error("E066", line, "button accepts at most one child"));
            }
            content = Some(parse_view(child)?);
        }
    }
    if label.is_some() && content.is_some() {
        return Err(error(
            "E066",
            line,
            "button uses either a string label or one child, not both",
        ));
    }
    if label.is_none() && content.is_none() {
        return Err(error("E066", line, "button needs a label or one child"));
    }
    Ok(ViewNode::Button {
        label,
        content: content.map(Box::new),
        id,
        disabled,
        options,
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E066", line, "button requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_button_status_style(
    line: &Line,
    styles: &mut ButtonStyleSet,
) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let (slot, status) = match parts.first().copied() {
        Some("active") => (&mut styles.active, "active"),
        Some("hovered") => (&mut styles.hovered, "hovered"),
        Some("pressed") => (&mut styles.pressed, "pressed"),
        Some("disabled") => (&mut styles.disabled, "disabled"),
        _ => unreachable!("button status was classified before parsing"),
    };
    if slot.is_some() {
        return Err(error(
            "E066",
            line,
            format!("duplicate button {status} style"),
        ));
    }
    let mut options = ContainerStyleOptions::default();
    for part in &parts[1..] {
        if !parse_container_style_option(part, &mut options, line)? {
            return Err(error(
                "E066",
                line,
                format!("unknown button style property `{part}`"),
            ));
        }
    }
    *slot = Some(ButtonStatusStyle {
        options,
        span: Span::line(line.number),
    });
    Ok(())
}

pub(in crate::parser) fn parse_checkbox(
    parts: &[&str],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .ok_or_else(|| error("E067", line, "checkbox needs a label expression"))?;
    let mut id = None;
    let mut checked = None;
    let mut disabled = None;
    let mut options = BoolControlOptions::default();
    let mut style = CheckboxStyleSet::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E067", "checkbox")?;
        } else if parse_accessibility_option(part, &mut options.accessibility, line)? {
        } else if let Some(value) = part.strip_prefix("checked=") {
            checked = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            if let Some(preset) = match value {
                "primary" => Some(CheckboxStylePreset::Primary),
                "secondary" => Some(CheckboxStylePreset::Secondary),
                "success" => Some(CheckboxStylePreset::Success),
                "danger" => Some(CheckboxStylePreset::Danger),
                _ => None,
            } {
                style.preset = preset;
                style.custom = None;
            } else {
                style.custom = Some(parse_extern_call(
                    value,
                    line,
                    "E067",
                    "checkbox style must be a preset or declared style call",
                )?);
            }
        } else if parse_bool_control_option(part, &mut options, false, true, line)? {
        } else {
            return Err(error(
                "E067",
                line,
                format!("unknown checkbox property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_checkbox_status_style(child, &mut style)?;
    }
    Ok(ViewNode::Checkbox {
        label: parse_expr(label, line)?,
        id,
        checked: checked.ok_or_else(|| error("E067", line, "checkbox requires `checked=value`"))?,
        disabled,
        options,
        style: Box::new(style),
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E067", line, "checkbox requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_checkbox_status_style(
    line: &Line,
    styles: &mut CheckboxStyleSet,
) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let status = parts.first().copied();
    let checked = parts.get(1).copied();
    let slot = match (status, checked) {
        (Some("active"), Some("checked")) => &mut styles.active_checked,
        (Some("active"), Some("unchecked")) => &mut styles.active_unchecked,
        (Some("hovered"), Some("checked")) => &mut styles.hovered_checked,
        (Some("hovered"), Some("unchecked")) => &mut styles.hovered_unchecked,
        (Some("disabled"), Some("checked")) => &mut styles.disabled_checked,
        (Some("disabled"), Some("unchecked")) => &mut styles.disabled_unchecked,
        _ => {
            return Err(error(
                "E067",
                line,
                "checkbox style lines use `<active|hovered|disabled> <checked|unchecked>`",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E067",
            line,
            format!(
                "duplicate checkbox {} {} style",
                status.unwrap(),
                checked.unwrap()
            ),
        ));
    }
    let mut style = CheckboxStatusStyle {
        span: Some(Span::line(line.number)),
        ..CheckboxStatusStyle::default()
    };
    for part in &parts[2..] {
        let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
        if let Some(value) = part.strip_prefix("bg=") {
            style.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("icon=") {
            style.icon_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("text=") {
            style.text_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border=") {
            style.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-w=") {
            style.border_width = Some(parse(value)?);
        } else if parse_radius_option(part, &mut style.radius, "", line)? {
        } else {
            return Err(error(
                "E067",
                line,
                format!("unknown checkbox style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

pub(in crate::parser) fn parse_toggler(
    parts: &[&str],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .ok_or_else(|| error("E075", line, "toggler needs a label expression"))?;
    let mut id = None;
    let mut checked = None;
    let mut disabled = None;
    let mut options = BoolControlOptions::default();
    let mut style = TogglerStyleSet::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E075", "toggler")?;
        } else if parse_accessibility_option(part, &mut options.accessibility, line)? {
        } else if let Some(value) = part.strip_prefix("checked=") {
            checked = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            style.custom = Some(parse_extern_call(
                value,
                line,
                "E075",
                "toggler style must be a declared style call",
            )?);
        } else if parse_bool_control_option(part, &mut options, true, false, line)? {
        } else {
            return Err(error(
                "E075",
                line,
                format!("unknown toggler property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_toggler_status_style(child, &mut style)?;
    }
    Ok(ViewNode::Toggler {
        label: parse_expr(label, line)?,
        id,
        checked: checked.ok_or_else(|| error("E075", line, "toggler requires `checked=value`"))?,
        disabled,
        options,
        style: Box::new(style),
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E075", line, "toggler requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_toggler_status_style(
    line: &Line,
    styles: &mut TogglerStyleSet,
) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let status = parts.first().copied();
    let checked = parts.get(1).copied();
    let slot = match (status, checked) {
        (Some("active"), Some("checked")) => &mut styles.active_checked,
        (Some("active"), Some("unchecked")) => &mut styles.active_unchecked,
        (Some("hovered"), Some("checked")) => &mut styles.hovered_checked,
        (Some("hovered"), Some("unchecked")) => &mut styles.hovered_unchecked,
        (Some("disabled"), Some("checked")) => &mut styles.disabled_checked,
        (Some("disabled"), Some("unchecked")) => &mut styles.disabled_unchecked,
        _ => {
            return Err(error(
                "E075",
                line,
                "toggler style lines use `<active|hovered|disabled> <checked|unchecked>`",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E075",
            line,
            format!(
                "duplicate toggler {} {} style",
                status.unwrap(),
                checked.unwrap()
            ),
        ));
    }
    let mut style = TogglerStatusStyle {
        span: Some(Span::line(line.number)),
        ..TogglerStatusStyle::default()
    };
    for part in &parts[2..] {
        let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
        if let Some(value) = part.strip_prefix("bg=") {
            style.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("bg-border=") {
            style.background_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("bg-border-w=") {
            style.background_border_width = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("fg=") {
            style.foreground = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("fg-border=") {
            style.foreground_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("fg-border-w=") {
            style.foreground_border_width = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("text=") {
            style.text_color = Some(value.to_owned());
        } else if parse_radius_option(part, &mut style.radius, "", line)? {
        } else if let Some(value) = part.strip_prefix("p-ratio=") {
            style.padding_ratio = Some(parse(value)?);
        } else {
            return Err(error(
                "E075",
                line,
                format!("unknown toggler style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

pub(in crate::parser) fn parse_bool_control_option(
    part: &str,
    options: &mut BoolControlOptions,
    allow_alignment: bool,
    allow_icon: bool,
    line: &Line,
) -> Result<bool, Error> {
    if let Some(value) = part.strip_prefix("size=") {
        options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("w=") {
        options.width = Some(parse_length(value, line)?);
    } else if let Some(value) = part.strip_prefix("gap=") {
        options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("text-size=") {
        options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("line-h=") {
        options.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("shape=") {
        options.shaping = Some(parse_text_shaping(value, line, "E075")?);
    } else if let Some(value) = part.strip_prefix("wrap=") {
        options.wrapping = Some(parse_text_wrapping(value, line, "E075")?);
    } else if let Some(value) = part.strip_prefix("font=") {
        options.font = Some(parse_font_preset(value, line)?);
    } else if allow_alignment && let Some(value) = part.strip_prefix("align=") {
        options.alignment = Some(
            value
                .parse()
                .map_err(|()| error("E075", line, "unknown text alignment"))?,
        );
    } else if allow_icon && let Some(value) = part.strip_prefix("icon=") {
        options.icon = Some(parse_char_literal(
            value,
            line,
            "E067",
            "checkbox icon must contain one character",
        )?);
    } else if allow_icon && let Some(value) = part.strip_prefix("icon-size=") {
        options.icon_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if allow_icon && let Some(value) = part.strip_prefix("icon-line-h=") {
        options.icon_line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if allow_icon && let Some(value) = part.strip_prefix("icon-shape=") {
        options.icon_shaping = Some(parse_text_shaping(value, line, "E075")?);
    } else {
        return Ok(false);
    }
    Ok(true)
}

pub(in crate::parser) fn parse_text_shaping(
    source: &str,
    line: &Line,
    code: &'static str,
) -> Result<TextShaping, Error> {
    match source {
        "auto" => Ok(TextShaping::Auto),
        "basic" => Ok(TextShaping::Basic),
        "advanced" => Ok(TextShaping::Advanced),
        _ => Err(error(
            code,
            line,
            "shaping must be auto, basic, or advanced",
        )),
    }
}

pub(in crate::parser) fn parse_text_wrapping(
    source: &str,
    line: &Line,
    code: &'static str,
) -> Result<TextWrapping, Error> {
    match source {
        "none" => Ok(TextWrapping::None),
        "word" => Ok(TextWrapping::Word),
        "glyph" => Ok(TextWrapping::Glyph),
        "word-or-glyph" => Ok(TextWrapping::WordOrGlyph),
        _ => Err(error(
            code,
            line,
            "wrapping must be none, word, glyph, or word-or-glyph",
        )),
    }
}

pub(in crate::parser) fn parse_font_preset(source: &str, line: &Line) -> Result<FontPreset, Error> {
    Ok(match source {
        "default" => FontPreset::Default,
        "mono" => FontPreset::Monospace,
        name => FontPreset::Named(line.qualify(&qualified_identifier(name, line)?)),
    })
}
