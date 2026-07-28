use super::*;

pub(in crate::parser) fn parse_component_call(
    parts: &[String],
    line: &Line,
) -> Result<(String, Vec<ComponentArg>, Option<Id>), Error> {
    let head = &parts[0];
    let name = line.qualify(&component_identifier(head, line)?);
    line.record_component_reference(&name);
    let mut args = Vec::new();
    let mut id = None;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E040", "component call")?;
            continue;
        }
        let (prop, value, bind) = split_top_marker(part, "<->")
            .map(|(prop, value)| (prop, value, true))
            .or_else(|| split_top_once(part, '=').map(|(prop, value)| (prop, value, false)))
            .ok_or_else(|| {
                error(
                    "E040",
                    line,
                    "component props use `name=value` or `name<->state`",
                )
            })?;
        args.push(ComponentArg {
            name: identifier(prop.trim(), line)?,
            value: parse_expr(strip_wrapping_parens(value.trim()), line)?,
            bind,
        });
    }
    Ok((name, args, id))
}

pub(in crate::parser) fn parse_text_editor(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E099", line, "editor does not accept `@` utilities"));
    }
    let mut binding = None;
    let mut id = None;
    let mut disabled = None;
    let mut options = TextEditorOptions::default();
    let mut index = 1;
    while index < parts.len() {
        let part = &parts[index];
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E099", "editor")?;
        } else if part == "<->" {
            index += 1;
            let value = identifier(
                parts
                    .get(index)
                    .ok_or_else(|| error("E099", line, "missing editor binding"))?,
                line,
            )?;
            if binding.replace(value).is_some() {
                return Err(error("E099", line, "editor has more than one binding"));
            }
        } else if let Some(value) = part.strip_prefix("hint=") {
            options.placeholder = Some(string_literal(value, line)?);
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("min-h=") {
            options.min_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max-h=") {
            options.max_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if parse_line_height_option(part, &mut options.line_height, line)? {
        } else if let Some(value) = part.strip_prefix("p=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("wrap=") {
            options.wrapping = Some(parse_text_wrapping(value, line, "E099")?);
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("highlight=") {
            options.highlight = Some(string_literal(value, line)?);
        } else if let Some(value) = part.strip_prefix("highlight-theme=") {
            options.highlight_theme = Some(match value {
                "solarized-dark" => HighlightTheme::SolarizedDark,
                "base16-mocha" => HighlightTheme::Base16Mocha,
                "base16-ocean" => HighlightTheme::Base16Ocean,
                "base16-eighties" => HighlightTheme::Base16Eighties,
                "inspired-github" => HighlightTheme::InspiredGithub,
                _ => return Err(error("E099", line, "unknown editor highlight theme")),
            });
        } else if let Some(value) = part.strip_prefix("highlighter=") {
            let (function, args) = parse_signature(value, line)?;
            options.highlighter = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("key-binding=") {
            let (function, args) = parse_signature(value, line)?;
            options.key_binding = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = Some(parse_extern_call(
                value,
                line,
                "E099",
                "editor style must be a declared style call",
            )?);
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E099",
                line,
                format!("unknown editor property `{part}`"),
            ));
        }
        index += 1;
    }
    if options.highlight.is_none() && options.highlight_theme.is_some() {
        return Err(error("E099", line, "highlight-theme requires highlight"));
    }
    if options.highlight.is_some() && options.highlighter.is_some() {
        return Err(error(
            "E099",
            line,
            "editor accepts either highlight or highlighter, not both",
        ));
    }
    options.key_binding_route = match (&options.key_binding, route) {
        (Some(_), Some(route)) => Some(parse_route(route.trim(), line)?),
        (Some(_), None) => {
            return Err(error(
                "E099",
                line,
                "key-binding requires `-> handler _` for custom bindings",
            ));
        }
        (None, Some(_)) => {
            return Err(error(
                "E099",
                line,
                "an editor route requires key-binding=name(args)",
            ));
        }
        (None, None) => None,
    };
    for child in &line.children {
        let parts = split_words(&child.text);
        match parts.first().map(String::as_str) {
            Some("active" | "hovered" | "focused" | "focused-hovered" | "disabled") => {
                ensure_leaf(child)?;
                parse_text_input_status(
                    &parts,
                    child,
                    &mut options.style,
                    "E099",
                    "editor",
                    false,
                )?;
            }
            _ => {
                return Err(error(
                    "E099",
                    child,
                    "editor blocks use active, hovered, focused, focused-hovered, or disabled",
                ));
            }
        }
    }
    Ok(ViewNode::TextEditor {
        binding: binding.ok_or_else(|| error("E099", line, "editor requires `<-> state`"))?,
        id,
        disabled,
        options,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_table(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E098", line, "table does not accept `@` utilities"));
    }
    if parts.len() < 4 || parts.get(2).map(String::as_str) != Some("in") {
        return Err(error("E098", line, "table uses `table item in rows`"));
    }
    if line.children.is_empty() {
        return Err(error("E098", line, "table requires at least one column"));
    }
    let mut id = None;
    let mut options = TableOptions::default();
    for part in &parts[4..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E098", "table")?;
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else {
            let (name, value) = part
                .split_once('=')
                .ok_or_else(|| error("E098", line, format!("unknown table property `{part}`")))?;
            let value = parse_expr(strip_wrapping_parens(value), line)?;
            match name {
                "p" => options.padding = Some(value),
                "px" => options.padding_x = Some(value),
                "py" => options.padding_y = Some(value),
                "sep" => options.separator = Some(value),
                "sep-x" => options.separator_x = Some(value),
                "sep-y" => options.separator_y = Some(value),
                _ => {
                    return Err(error(
                        "E098",
                        line,
                        format!("unknown table property `{name}`"),
                    ));
                }
            }
        }
    }
    Ok(ViewNode::Table {
        item: identifier(&parts[1], line)?,
        rows: parse_expr(strip_wrapping_parens(&parts[3]), line)?,
        id,
        options,
        columns: line
            .children
            .iter()
            .map(parse_table_column)
            .collect::<Result<_, _>>()?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_table_column(line: &Line) -> Result<TableColumn, Error> {
    let parts = split_words(&line.text);
    if parts.first().map(String::as_str) != Some("col") {
        return Err(error("E098", line, "table children must be `col` nodes"));
    }
    let mut width = None;
    let mut align_x = None;
    let mut align_y = None;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("w=") {
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            align_x = Some(value.parse().map_err(|()| {
                error(
                    "E098",
                    line,
                    "column align-x must be left, center, or right",
                )
            })?);
        } else if let Some(value) = part.strip_prefix("align-y=") {
            align_y = Some(value.parse().map_err(|()| {
                error(
                    "E098",
                    line,
                    "column align-y must be top, center, or bottom",
                )
            })?);
        } else {
            return Err(error(
                "E098",
                line,
                format!("unknown column property `{part}`"),
            ));
        }
    }
    if line.children.len() != 2 {
        return Err(error(
            "E098",
            line,
            "column requires one header and one cell",
        ));
    }
    let parse_part = |part: &Line, expected: &str| {
        if part.text != expected || part.children.len() != 1 {
            return Err(error(
                "E098",
                part,
                format!("column `{expected}` requires exactly one child"),
            ));
        }
        parse_view(&part.children[0])
    };
    Ok(TableColumn {
        width,
        align_x,
        align_y,
        header: parse_part(&line.children[0], "header")?,
        cell: parse_part(&line.children[1], "cell")?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_markdown(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E097",
            line,
            "markdown does not accept `@` utilities",
        ));
    }
    let content = parts
        .get(1)
        .ok_or_else(|| error("E097", line, "markdown requires a content state"))?;
    let route = route.ok_or_else(|| {
        error(
            "E097",
            line,
            "markdown requires a link route such as `-> open_link _`",
        )
    })?;
    let mut id = None;
    let mut options = MarkdownOptions::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E097", "markdown")?;
            continue;
        }
        let (name, value) = part
            .split_once('=')
            .ok_or_else(|| error("E097", line, format!("unknown markdown property `{part}`")))?;
        match name {
            "text-size" => {
                options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?)
            }
            "h1-size" => options.h1_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h2-size" => options.h2_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h3-size" => options.h3_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h4-size" => options.h4_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h5-size" => options.h5_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h6-size" => options.h6_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "code-size" => {
                options.code_size = Some(parse_expr(strip_wrapping_parens(value), line)?)
            }
            "gap" => options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "viewer" => {
                let (function, args) = parse_signature(value, line)?;
                options.viewer = Some(ExternCall {
                    function,
                    args: parse_expr_list(&args, line)?,
                });
            }
            _ => {
                return Err(error(
                    "E097",
                    line,
                    format!("unknown markdown property `{name}`"),
                ));
            }
        }
    }
    options.style = match line.children.as_slice() {
        [] => MarkdownStyleOptions::default(),
        [style] => parse_markdown_style(style)?,
        _ => {
            return Err(error(
                "E097",
                line,
                "markdown accepts at most one `style` child",
            ));
        }
    };
    Ok(ViewNode::Markdown {
        content: identifier(content, line)?,
        id,
        options: Box::new(options),
        route: parse_route(route, line)?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_markdown_style(line: &Line) -> Result<MarkdownStyleOptions, Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    if parts.first().map(String::as_str) != Some("style") {
        return Err(error("E097", line, "markdown child must be `style`"));
    }
    let mut style = MarkdownStyleOptions::default();
    let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
    for part in &parts[1..] {
        let Some((name, value)) = part.split_once('=') else {
            return Err(error(
                "E097",
                line,
                format!("unknown markdown style property `{part}`"),
            ));
        };
        match name {
            "font" => style.font = Some(parse_font_preset(value, line)?),
            "inline-code-bg" => {
                style.inline_code_background = Some(parse_background_value(value, line)?)
            }
            "inline-code-fg" => style.inline_code_color = Some(value.to_owned()),
            "inline-code-font" => style.inline_code_font = Some(parse_font_preset(value, line)?),
            "code-block-font" => style.code_block_font = Some(parse_font_preset(value, line)?),
            "link" => style.link_color = Some(value.to_owned()),
            "inline-code-p" => style.inline_code_padding.all = Some(parse(value)?),
            "inline-code-px" => style.inline_code_padding.x = Some(parse(value)?),
            "inline-code-py" => style.inline_code_padding.y = Some(parse(value)?),
            "inline-code-pt" => style.inline_code_padding.top = Some(parse(value)?),
            "inline-code-pr" => style.inline_code_padding.right = Some(parse(value)?),
            "inline-code-pb" => style.inline_code_padding.bottom = Some(parse(value)?),
            "inline-code-pl" => style.inline_code_padding.left = Some(parse(value)?),
            "inline-code-border" => style.inline_code_border_color = Some(value.to_owned()),
            "inline-code-border-w" => style.inline_code_border_width = Some(parse(value)?),
            "inline-code-r" => style.inline_code_radius = Some(parse(value)?),
            "inline-code-r-tl" => style.inline_code_radius_top_left = Some(parse(value)?),
            "inline-code-r-tr" => style.inline_code_radius_top_right = Some(parse(value)?),
            "inline-code-r-br" => style.inline_code_radius_bottom_right = Some(parse(value)?),
            "inline-code-r-bl" => style.inline_code_radius_bottom_left = Some(parse(value)?),
            _ => {
                return Err(error(
                    "E097",
                    line,
                    format!("unknown markdown style property `{name}`"),
                ));
            }
        }
    }
    Ok(style)
}

pub(in crate::parser) fn parse_lazy(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E096", line, "lazy does not accept `@` utilities"));
    }
    if !(4..=5).contains(&parts.len()) || parts[2] != "as" {
        return Err(error("E096", line, "lazy uses `lazy dependency as name`"));
    }
    if line.children.len() != 1 {
        return Err(error(
            "E096",
            line,
            "lazy requires exactly one child subtree",
        ));
    }
    let mut id = None;
    for part in &parts[4..] {
        parse_unique_id(part, &mut id, line, "E096", "lazy")?;
    }
    Ok(ViewNode::Lazy {
        dependency: parse_expr(strip_wrapping_parens(&parts[1]), line)?,
        binding: identifier(&parts[3], line)?,
        id,
        child: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_keyed_column(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E095", line, "keyed does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error(
            "E095",
            line,
            "keyed requires exactly one child template",
        ));
    }
    if parts.len() < 5 || parts.get(2).map(String::as_str) != Some("in") {
        return Err(error(
            "E095",
            line,
            "keyed uses `keyed item in items by=item.id`",
        ));
    }
    let key = parts[4]
        .strip_prefix("by=")
        .ok_or_else(|| error("E095", line, "keyed uses `keyed item in items by=item.id`"))?;
    let mut id = None;
    let mut option_parts = Vec::new();
    for part in &parts[5..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E095", "keyed")?;
        } else {
            option_parts.push(part.clone());
        }
    }
    let options = parse_layout_options("col", &option_parts, line)?;
    if options.clip.is_some() || options.wrap {
        return Err(error(
            "E095",
            line,
            "keyed columns do not support clip or wrap",
        ));
    }
    Ok(ViewNode::KeyedColumn {
        item: identifier(&parts[1], line)?,
        items: parse_expr(strip_wrapping_parens(&parts[3]), line)?,
        key: parse_expr(strip_wrapping_parens(key), line)?,
        id,
        options: Box::new(options),
        child: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_slot(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !(1..=2).contains(&parts.len()) || !styles.is_empty() {
        return Err(error(
            "E040",
            line,
            "slot accepts an optional name and no properties or styles",
        ));
    }
    let (name, optional) = parts.get(1).map_or_else(
        || Ok(("children".into(), false)),
        |name| {
            let (name, optional) = name
                .strip_suffix('?')
                .map_or((name.as_str(), false), |name| (name, true));
            Ok::<_, Error>((identifier(name, line)?, optional))
        },
    )?;
    Ok(ViewNode::Slot {
        name,
        optional,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_theme(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E094", line, "theme does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E094", line, "theme requires exactly one child"));
    }
    let mut id = None;
    let mut preset = ThemePreset::Default;
    let mut text = None;
    let mut background = None;
    let mut preset_seen = false;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E094", "theme")?;
        } else if !preset_seen && !part.contains('=') {
            preset = parse_theme_preset(part, line)?;
            preset_seen = true;
        } else if let Some(value) = part.strip_prefix("fg=") {
            text = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("bg=") {
            background = Some(parse_background_value(value, line)?);
        } else {
            return Err(error(
                "E094",
                line,
                format!("unknown theme property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Theme {
        id,
        preset,
        text,
        background,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_theme_preset(
    value: &str,
    line: &Line,
) -> Result<ThemePreset, Error> {
    if value.contains('(') {
        return Ok(ThemePreset::Factory(parse_extern_call(
            value,
            line,
            "E094",
            "theme factory must be a declared call",
        )?));
    }
    match value {
        "default" => Ok(ThemePreset::Default),
        "app" => Ok(ThemePreset::App),
        value if BUILT_IN_THEMES.contains(&value) => Ok(ThemePreset::BuiltIn(value.into())),
        _ => Err(error("E094", line, format!("unknown iced theme `{value}`"))),
    }
}

pub(in crate::parser) fn parse_qr_code(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error("E093", line, "qr does not accept `@` utilities"));
    }
    let payload = parts
        .get(1)
        .ok_or_else(|| error("E093", line, "qr needs one payload expression"))?;
    let mut id = None;
    let mut correction = None;
    let mut version = None;
    let mut cell_size = None;
    let mut total_size = None;
    let mut cell = None;
    let mut background = None;
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E093", "qr")?;
        } else if let Some(value) = part.strip_prefix("correction=") {
            correction = Some(match value {
                "low" => QrCorrection::Low,
                "medium" => QrCorrection::Medium,
                "quartile" => QrCorrection::Quartile,
                "high" => QrCorrection::High,
                _ => {
                    return Err(error(
                        "E093",
                        line,
                        "qr correction must be low, medium, quartile, or high",
                    ));
                }
            });
        } else if let Some(value) = part.strip_prefix("version=") {
            version = Some(parse_qr_version(value, line)?);
        } else if let Some(value) = part.strip_prefix("cell-size=") {
            cell_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            total_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("cell=") {
            cell = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("bg=") {
            background = Some(value.to_owned());
        } else {
            return Err(error("E093", line, format!("unknown qr property `{part}`")));
        }
    }
    if cell_size.is_some() && total_size.is_some() {
        return Err(error(
            "E093",
            line,
            "qr accepts either cell-size or size, not both",
        ));
    }
    Ok(ViewNode::QrCode {
        payload: parse_expr(payload, line)?,
        id,
        correction,
        version,
        cell_size,
        total_size,
        cell,
        background,
        span: Span::line(line.number),
    })
}

fn parse_qr_version(source: &str, line: &Line) -> Result<QrVersion, Error> {
    let invalid = || error("E093", line, "qr version uses normal(1..40) or micro(1..4)");
    let (kind, number) = source
        .split_once('(')
        .and_then(|(kind, number)| number.strip_suffix(')').map(|number| (kind, number)))
        .ok_or_else(invalid)?;
    let number = number.parse::<u8>().map_err(|_| invalid())?;
    match kind {
        "normal" => Ok(QrVersion::Normal(number)),
        "micro" => Ok(QrVersion::Micro(number)),
        _ => Err(invalid()),
    }
}
