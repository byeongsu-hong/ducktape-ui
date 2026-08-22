use super::*;

pub(in crate::parser) fn parse_text(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let value = parts
        .get(1)
        .ok_or_else(|| error("E063", line, "text expects one expression before `@`"))?;
    let mut id = None;
    let mut options = TextOptions::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E063", "text")?;
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if parse_line_height_option(part, &mut options.line_height, line)? {
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            options.align_x = Some(
                value
                    .parse()
                    .map_err(|()| error("E063", line, "unknown horizontal text alignment"))?,
            );
        } else if let Some(value) = part.strip_prefix("align-y=") {
            options.align_y = Some(
                value
                    .parse()
                    .map_err(|()| error("E063", line, "unknown vertical text alignment"))?,
            );
        } else if let Some(value) = part.strip_prefix("shape=") {
            options.shaping = Some(parse_text_shaping(value, line, "E063")?);
        } else if let Some(value) = part.strip_prefix("wrap=") {
            options.wrapping = Some(parse_text_wrapping(value, line, "E063")?);
        } else if let Some(value) = part.strip_prefix("tracking=") {
            options.tracking = Some(parse_text_tracking(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = Some(parse_extern_call(
                value,
                line,
                "E063",
                "text style must be a declared style call",
            )?);
        } else if part == "underline" {
            options.underline = Some(Expr::Bool(true));
        } else if let Some(value) = part.strip_prefix("underline=") {
            options.underline = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "strike" {
            options.strikethrough = Some(Expr::Bool(true));
        } else if let Some(value) = part.strip_prefix("strike=") {
            options.strikethrough = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E063",
                line,
                format!("unknown text property `{part}`"),
            ));
        }
    }
    ensure_leaf(line)?;
    Ok(ViewNode::Text {
        value: parse_expr(value, line)?,
        id,
        options,
        styles,
        span: Span::line(line.number),
    })
}

/// Tracking is a plain literal, never an expression: the lowering has to know
/// at compile time whether any tracking is asked for at all, because a text
/// without it must stay one text widget.
fn parse_text_tracking(source: &str, line: &Line) -> Result<f64, Error> {
    let invalid = || {
        error(
            "E063",
            line,
            "text tracking must be a non-negative number literal",
        )
    };
    let Expr::F64(value) = parse_expr(strip_wrapping_parens(source), line)? else {
        return Err(invalid());
    };
    if !value.is_finite() || value < 0.0 {
        return Err(invalid());
    }
    Ok(value)
}

pub(in crate::parser) fn parse_rich_text(
    parts: &[String],
    styles: Vec<String>,
    route_source: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let mut id = None;
    let mut options = TextOptions::default();
    let mut color = None;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E186", "rich-text")?;
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if parse_line_height_option(part, &mut options.line_height, line)? {
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            options.align_x = Some(
                value
                    .parse()
                    .map_err(|()| error("E186", line, "unknown rich text alignment"))?,
            );
        } else if let Some(value) = part.strip_prefix("align-y=") {
            options.align_y = Some(
                value
                    .parse()
                    .map_err(|()| error("E186", line, "unknown rich text alignment"))?,
            );
        } else if let Some(value) = part.strip_prefix("wrap=") {
            options.wrapping = Some(parse_text_wrapping(value, line, "E186")?);
        } else if let Some(value) = part.strip_prefix("color=") {
            color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = Some(parse_extern_call(
                value,
                line,
                "E186",
                "rich-text style must be a declared style call",
            )?);
        } else {
            return Err(error(
                "E186",
                line,
                format!("unknown rich-text property `{part}`"),
            ));
        }
    }
    let children = line
        .children
        .iter()
        .map(parse_rich_child)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ViewNode::RichText {
        id,
        options,
        color,
        children,
        styles,
        route: route_source
            .map(|route| parse_route(route, line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

/// A `rich-text` child is a literal `span` line or a `for` line whose
/// children are all `span` lines; anything else keeps the E186 rejection.
pub(in crate::parser) fn parse_rich_child(line: &Line) -> Result<RichTextChild, Error> {
    let Some(loop_source) = line.text.trim().strip_prefix("for ") else {
        return parse_rich_span(line).map(|span| RichTextChild::Span(Box::new(span)));
    };
    let Some((item, items)) = loop_source.split_once(" in ") else {
        return Err(error("E186", line, "loops use `for item in items`"));
    };
    let spans = line
        .children
        .iter()
        .map(parse_rich_span)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RichTextChild::For(RichTextFor {
        item: identifier(item.trim(), line)?,
        items: parse_expr(items.trim(), line)?,
        spans,
        span: Span::line(line.number),
    }))
}

pub(in crate::parser) fn parse_rich_span(line: &Line) -> Result<RichSpan, Error> {
    ensure_leaf(line)?;
    let (core, styles) = split_top_marker(&line.text, "@").map_or_else(
        || (line.text.trim(), Vec::new()),
        |(core, styles)| {
            (
                core.trim(),
                styles.split_whitespace().map(str::to_owned).collect(),
            )
        },
    );
    let parts = split_words(core);
    if parts.first().map(String::as_str) != Some("span") {
        return Err(error(
            "E186",
            line,
            "rich-text children must be `span` lines",
        ));
    }
    let value = parts
        .get(1)
        .ok_or_else(|| error("E186", line, "span expects one text expression"))?;
    let mut options = RichSpanOptions::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if parse_line_height_option(part, &mut options.line_height, line)? {
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("color=") {
            options.color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("link=") {
            options.link = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("bg=") {
            options.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("border=") {
            options.border = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-w=") {
            options.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if parse_radius_option(part, &mut options.radius, "", line)?
            || parse_padding_option(part, &mut options.padding, line)?
        {
        } else if part == "underline" {
            options.underline = Some(Expr::Bool(true));
        } else if let Some(value) = part.strip_prefix("underline=") {
            options.underline = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "strike" {
            options.strikethrough = Some(Expr::Bool(true));
        } else if let Some(value) = part.strip_prefix("strike=") {
            options.strikethrough = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E186",
                line,
                format!("unknown span property `{part}`"),
            ));
        }
    }
    Ok(RichSpan {
        value: parse_expr(value, line)?,
        options,
        styles,
        span: Span::line(line.number),
    })
}
