use super::*;

pub(in crate::parser) fn parse_combo_box(
    parts: &[&str],
    styles: Vec<String>,
    route_source: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E088",
            line,
            "combo uses typed properties instead of `@` utilities",
        ));
    }
    if parts.len() < 4 {
        return Err(error(
            "E088",
            line,
            "combo expects `combo state selected \"Placeholder\" -> handler _`",
        ));
    }
    let route = route_source.ok_or_else(|| error("E088", line, "combo requires `-> handler _`"))?;
    let mut id = None;
    let mut options = ComboBoxOptions::default();
    for part in &parts[4..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E088", "combo")?;
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("menu-h=") {
            options.menu_height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("p=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text-size=") {
            options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-h=") {
            options.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shape=") {
            options.shaping = Some(parse_text_shaping(value, line, "E088")?);
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("input=") {
            options.input = Some(parse_payload_route(value, line, 1)?);
        } else if let Some(value) = part.strip_prefix("hover=") {
            options.hover = Some(parse_payload_route(value, line, 1)?);
        } else if let Some(value) = part.strip_prefix("open=") {
            options.open = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("close=") {
            options.close = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = Some(parse_extern_call(
                value,
                line,
                "E088",
                "combo style must be a declared style call",
            )?);
        } else if let Some(value) = part.strip_prefix("menu-style=") {
            options.custom_menu_style = Some(parse_extern_call(
                value,
                line,
                "E088",
                "combo menu style must be a declared style call",
            )?);
        } else {
            return Err(error(
                "E088",
                line,
                format!("unknown combo property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_combo_box_child(child, &mut options)?;
    }
    Ok(ViewNode::ComboBox {
        state: identifier(parts[1], line)?,
        id,
        selected: parse_expr(parts[2], line)?,
        placeholder: parse_expr(strip_wrapping_parens(parts[3]), line)?,
        options,
        route: parse_route(route.trim(), line)?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_combo_box_child(
    line: &Line,
    options: &mut ComboBoxOptions,
) -> Result<(), Error> {
    let parts = split_words(&line.text);
    match parts.first().copied() {
        Some("active" | "hovered" | "focused" | "focused-hovered" | "disabled") => {
            ensure_leaf(line)?;
            parse_text_input_status(&parts, line, &mut options.style, "E088", "combo", true)
        }
        Some("menu") => {
            ensure_leaf(line)?;
            if options.menu_style.is_some() {
                return Err(error("E088", line, "duplicate combo menu style"));
            }
            options.menu_style = Some(Box::new(parse_menu_style(&parts, line, "E088", "combo")?));
            Ok(())
        }
        Some("icon") => {
            ensure_leaf(line)?;
            if options.icon.is_some() {
                return Err(error("E088", line, "duplicate combo icon"));
            }
            options.icon = Some(parse_text_input_icon(&parts[1..], line, "E088", "combo")?);
            Ok(())
        }
        _ => Err(error(
            "E088",
            line,
            "combo blocks use active, hovered, focused, focused-hovered, disabled, menu, or icon",
        )),
    }
}

pub(in crate::parser) fn parse_text_input_status(
    parts: &[&str],
    line: &Line,
    styles: &mut TextInputStyleSet,
    code: &'static str,
    widget: &str,
    supports_icon: bool,
) -> Result<(), Error> {
    let status = parts.first().expect("text input status line");
    let slot = match *status {
        "active" => &mut styles.active,
        "hovered" => &mut styles.hovered,
        "focused" => &mut styles.focused,
        "focused-hovered" => &mut styles.focused_hovered,
        "disabled" => &mut styles.disabled,
        _ => unreachable!("text input status dispatch validates the status"),
    };
    if slot.is_some() {
        return Err(error(
            code,
            line,
            format!("duplicate {widget} {status} style"),
        ));
    }
    let mut style = TextInputStatusStyle {
        span: Some(Span::line(line.number)),
        ..TextInputStatusStyle::default()
    };
    for part in &parts[1..] {
        if supports_icon && let Some(value) = part.strip_prefix("icon=") {
            style.icon_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("placeholder=") {
            style.placeholder_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("value=") {
            style.value_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("selection=") {
            style.selection_color = Some(value.to_owned());
        } else if parse_container_style_option(part, &mut style.options, line)? {
            if style.options.text_color.is_some()
                || style.options.shadow_color.is_some()
                || style.options.shadow_x.is_some()
                || style.options.shadow_y.is_some()
                || style.options.shadow_blur.is_some()
                || style.options.pixel_snap.is_some()
            {
                return Err(error(
                    code,
                    line,
                    format!("unknown {widget} style property `{part}`"),
                ));
            }
        } else {
            return Err(error(
                code,
                line,
                format!("unknown {widget} style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

pub(in crate::parser) fn parse_text_input_icon(
    parts: &[&str],
    line: &Line,
    code: &'static str,
    widget: &str,
) -> Result<TextInputIcon, Error> {
    let mut code_point = None;
    let mut font = None;
    let mut size = None;
    let mut spacing = None;
    let mut side = IconSide::Left;
    for part in parts {
        if let Some(value) = part.strip_prefix("code=") {
            code_point = Some(parse_char_literal(
                value,
                line,
                code,
                format!("{widget} icon code must contain one character"),
            )?);
        } else if let Some(value) = part.strip_prefix("font=") {
            font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("gap=") {
            spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("side=") {
            side = match value {
                "left" => IconSide::Left,
                "right" => IconSide::Right,
                _ => {
                    return Err(error(
                        code,
                        line,
                        format!("{widget} icon side must be left or right"),
                    ));
                }
            };
        } else {
            return Err(error(
                code,
                line,
                format!("unknown {widget} icon property `{part}`"),
            ));
        }
    }
    Ok(TextInputIcon {
        code_point: code_point
            .ok_or_else(|| error(code, line, format!("{widget} icon requires code=\"…\"")))?,
        font,
        size,
        spacing,
        side,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pick_list(
    parts: &[&str],
    styles: Vec<String>,
    route_source: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E087",
            line,
            "pick uses typed properties instead of `@` utilities",
        ));
    }
    if parts.len() < 3 {
        return Err(error(
            "E087",
            line,
            "pick expects `pick options selected -> handler _`",
        ));
    }
    let route = route_source.ok_or_else(|| error("E087", line, "pick requires `-> handler _`"))?;
    let mut id = None;
    let mut config = PickListOptions::default();
    for part in &parts[3..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E087", "pick")?;
        } else if let Some(value) = part.strip_prefix("hint=") {
            config.placeholder = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("w=") {
            config.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("menu-h=") {
            config.menu_height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("p=") {
            config.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text-size=") {
            config.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-h=") {
            config.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shape=") {
            config.shaping = Some(parse_text_shaping(value, line, "E087")?);
        } else if let Some(value) = part.strip_prefix("font=") {
            config.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("open=") {
            config.open = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("close=") {
            config.close = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            config.custom_style = Some(parse_extern_call(
                value,
                line,
                "E087",
                "pick style must be a declared style call",
            )?);
        } else if let Some(value) = part.strip_prefix("menu-style=") {
            config.custom_menu_style = Some(parse_extern_call(
                value,
                line,
                "E087",
                "pick menu style must be a declared style call",
            )?);
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown pick property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_pick_list_child(child, &mut config)?;
    }
    Ok(ViewNode::PickList {
        options: parse_expr(parts[1], line)?,
        id,
        selected: parse_expr(parts[2], line)?,
        options_config: config,
        route: parse_route(route.trim(), line)?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pick_list_child(
    line: &Line,
    options: &mut PickListOptions,
) -> Result<(), Error> {
    let parts = split_words(&line.text);
    match parts.first().copied() {
        Some("active" | "hovered" | "opened" | "opened-hovered") => {
            ensure_leaf(line)?;
            parse_pick_list_status(&parts, line, &mut options.style)
        }
        Some("menu") => {
            ensure_leaf(line)?;
            if options.menu_style.is_some() {
                return Err(error("E087", line, "duplicate pick menu style"));
            }
            options.menu_style = Some(Box::new(parse_menu_style(&parts, line, "E087", "pick")?));
            Ok(())
        }
        Some("handle") => {
            if options.handle.is_some() {
                return Err(error("E087", line, "duplicate pick handle"));
            }
            options.handle = Some(parse_pick_list_handle(&parts, line)?);
            Ok(())
        }
        _ => Err(error(
            "E087",
            line,
            "pick blocks use active, hovered, opened, opened-hovered, menu, or handle",
        )),
    }
}

pub(in crate::parser) fn parse_pick_list_status(
    parts: &[&str],
    line: &Line,
    styles: &mut PickListStyleSet,
) -> Result<(), Error> {
    let status = parts.first().expect("pick status line");
    let slot = match *status {
        "active" => &mut styles.active,
        "hovered" => &mut styles.hovered,
        "opened" => &mut styles.opened,
        "opened-hovered" => &mut styles.opened_hovered,
        _ => unreachable!("pick status dispatch validates the status"),
    };
    if slot.is_some() {
        return Err(error(
            "E087",
            line,
            format!("duplicate pick {status} style"),
        ));
    }
    let mut style = PickListStatusStyle {
        span: Some(Span::line(line.number)),
        ..PickListStatusStyle::default()
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("placeholder=") {
            style.placeholder_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("handle=") {
            style.handle_color = Some(value.to_owned());
        } else if parse_container_style_option(part, &mut style.options, line)? {
            if style.options.shadow_color.is_some()
                || style.options.shadow_x.is_some()
                || style.options.shadow_y.is_some()
                || style.options.shadow_blur.is_some()
                || style.options.pixel_snap.is_some()
            {
                return Err(error(
                    "E087",
                    line,
                    format!("unknown pick status property `{part}`"),
                ));
            }
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown pick status property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

pub(in crate::parser) fn parse_menu_style(
    parts: &[&str],
    line: &Line,
    code: &'static str,
    widget: &str,
) -> Result<MenuStyleOptions, Error> {
    let mut style = MenuStyleOptions {
        span: Some(Span::line(line.number)),
        ..MenuStyleOptions::default()
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("selected-text=") {
            style.selected_text_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("selected-bg=") {
            style.selected_background = Some(parse_background_value(value, line)?);
        } else if parse_container_style_option(part, &mut style.options, line)? {
            if style.options.pixel_snap.is_some() {
                return Err(error(
                    code,
                    line,
                    format!("{widget} menu does not support px-snap"),
                ));
            }
        } else {
            return Err(error(
                code,
                line,
                format!("unknown {widget} menu property `{part}`"),
            ));
        }
    }
    Ok(style)
}

pub(in crate::parser) fn parse_pick_list_handle(
    parts: &[&str],
    line: &Line,
) -> Result<PickListHandle, Error> {
    let kind = parts.get(1).copied().ok_or_else(|| {
        error(
            "E087",
            line,
            "pick handle uses arrow, static, dynamic, or none",
        )
    })?;
    match kind {
        "arrow" => {
            ensure_leaf(line)?;
            let mut size = None;
            for part in &parts[2..] {
                if let Some(value) = part.strip_prefix("size=") {
                    size = Some(parse_expr(strip_wrapping_parens(value), line)?);
                } else {
                    return Err(error(
                        "E087",
                        line,
                        format!("unknown arrow handle property `{part}`"),
                    ));
                }
            }
            Ok(PickListHandle::Arrow { size })
        }
        "static" => {
            ensure_leaf(line)?;
            Ok(PickListHandle::Static(parse_pick_list_icon(
                &parts[2..],
                line,
            )?))
        }
        "dynamic" => {
            if parts.len() != 2
                || line.children.len() != 2
                || line.children[0].text.split_ascii_whitespace().next() != Some("closed")
                || line.children[1].text.split_ascii_whitespace().next() != Some("open")
            {
                return Err(error(
                    "E087",
                    line,
                    "dynamic pick handle requires closed then open icon lines",
                ));
            }
            let closed = split_words(&line.children[0].text);
            let open = split_words(&line.children[1].text);
            ensure_leaf(&line.children[0])?;
            ensure_leaf(&line.children[1])?;
            Ok(PickListHandle::Dynamic {
                closed: parse_pick_list_icon(&closed[1..], &line.children[0])?,
                open: parse_pick_list_icon(&open[1..], &line.children[1])?,
            })
        }
        "none" => {
            ensure_leaf(line)?;
            if parts.len() != 2 {
                return Err(error("E087", line, "none handle has no properties"));
            }
            Ok(PickListHandle::None)
        }
        _ => Err(error(
            "E087",
            line,
            "pick handle uses arrow, static, dynamic, or none",
        )),
    }
}

pub(in crate::parser) fn parse_pick_list_icon(
    parts: &[&str],
    line: &Line,
) -> Result<PickListIcon, Error> {
    let mut code_point = None;
    let mut font = None;
    let mut size = None;
    let mut line_height = None;
    let mut shaping = None;
    for part in parts {
        if let Some(value) = part.strip_prefix("code=") {
            code_point = Some(parse_char_literal(
                value,
                line,
                "E087",
                "pick handle code must contain one character",
            )?);
        } else if let Some(value) = part.strip_prefix("font=") {
            font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-h=") {
            line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shape=") {
            shaping = Some(parse_text_shaping(value, line, "E087")?);
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown pick handle icon property `{part}`"),
            ));
        }
    }
    Ok(PickListIcon {
        code_point: code_point.ok_or_else(|| {
            error(
                "E087",
                line,
                "static and dynamic pick handles require code=\"…\"",
            )
        })?,
        font,
        size,
        line_height,
        shaping,
        span: Span::line(line.number),
    })
}
