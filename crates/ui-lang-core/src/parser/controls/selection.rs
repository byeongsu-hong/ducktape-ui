use super::*;

pub(in crate::parser) fn parse_slider(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let value = parts
        .get(1)
        .ok_or_else(|| error("E076", line, "slider needs a value expression"))?;
    let mut id = None;
    let mut min = None;
    let mut max = None;
    let mut step = Expr::F64(1.0);
    let mut options = SliderOptions::default();
    let mut vertical = false;
    let mut release = None;
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E076", "slider")?;
        } else if let Some(value) = part.strip_prefix("min=") {
            min = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max=") {
            max = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("step=") {
            step = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("default=") {
            options.default = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shift-step=") {
            options.shift_step = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            options.style.custom = Some(parse_extern_call(
                value,
                line,
                "E076",
                "slider style must be a declared style call",
            )?);
        } else if part == "vertical" {
            vertical = true;
        } else if let Some(value) = part.strip_prefix("release=") {
            release = Some(parse_route(value, line)?);
        } else {
            return Err(error(
                "E076",
                line,
                format!("unknown slider property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_slider_style(child, &mut options.style)?;
    }
    Ok(ViewNode::Slider {
        value: parse_expr(value, line)?,
        id,
        min: min.ok_or_else(|| error("E076", line, "slider requires `min=value`"))?,
        max: max.ok_or_else(|| error("E076", line, "slider requires `max=value`"))?,
        step,
        options: Box::new(options),
        vertical,
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E076", line, "slider requires `-> handler`"))?,
            line,
        )?,
        release,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_slider_style(
    line: &Line,
    styles: &mut SliderStyleSet,
) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let (slot, status) = match parts.first().map(String::as_str) {
        Some("active") => (&mut styles.active, "active"),
        Some("hovered") => (&mut styles.hovered, "hovered"),
        Some("dragged") => (&mut styles.dragged, "dragged"),
        _ => {
            return Err(error(
                "E076",
                line,
                "slider style block must be active, hovered, or dragged",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E076",
            line,
            format!("duplicate slider {status} style"),
        ));
    }
    let mut style = SliderStyle {
        span: Some(Span::line(line.number)),
        ..SliderStyle::default()
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("rail-start=") {
            style.rail_start = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("rail-end=") {
            style.rail_end = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("rail-w=") {
            style.rail_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-border=") {
            style.rail_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("rail-border-w=") {
            style.rail_border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-r=") {
            style.rail_radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-r-tl=") {
            style.rail_radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-r-tr=") {
            style.rail_radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-r-br=") {
            style.rail_radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-r-bl=") {
            style.rail_radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle=") {
            style.handle_shape = Some(parse_slider_handle(value, line)?);
        } else if let Some(value) = part.strip_prefix("handle-color=") {
            style.handle_color = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("handle-border=") {
            style.handle_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("handle-border-w=") {
            style.handle_border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-r=") {
            style.handle_radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-r-tl=") {
            style.handle_radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-r-tr=") {
            style.handle_radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-r-br=") {
            style.handle_radius_bottom_right =
                Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-r-bl=") {
            style.handle_radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E076",
                line,
                format!("unknown slider style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

pub(in crate::parser) fn parse_slider_handle(
    source: &str,
    line: &Line,
) -> Result<SliderHandleShape, Error> {
    if let Some(value) = source
        .strip_prefix("circle(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(SliderHandleShape::Circle(parse_expr(value, line)?));
    }
    if let Some(value) = source
        .strip_prefix("rect(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(SliderHandleShape::Rectangle {
            width: value
                .parse()
                .map_err(|_| error("E076", line, "slider rectangle width must be a u16"))?,
        });
    }
    Err(error(
        "E076",
        line,
        "slider handle must be circle(N) or rect(N)",
    ))
}

pub(in crate::parser) fn parse_progress(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    let value = parts
        .get(1)
        .ok_or_else(|| error("E077", line, "progress needs a value expression"))?;
    let mut id = None;
    let mut min = Expr::F64(0.0);
    let mut max = Expr::F64(100.0);
    let mut options = ProgressOptions::default();
    let mut vertical = false;
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E077", "progress")?;
        } else if let Some(value) = part.strip_prefix("min=") {
            min = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("max=") {
            max = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("length=") {
            options.length = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("girth=") {
            options.girth = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            if let Some(style) = match value {
                "primary" => Some(ProgressStyle::Primary),
                "secondary" => Some(ProgressStyle::Secondary),
                "success" => Some(ProgressStyle::Success),
                "warning" => Some(ProgressStyle::Warning),
                "danger" => Some(ProgressStyle::Danger),
                _ => None,
            } {
                options.style = Some(style);
                options.custom_style = None;
            } else {
                options.custom_style = Some(parse_extern_call(
                    value,
                    line,
                    "E077",
                    "progress style must be a preset or declared style call",
                )?);
                options.style = None;
            }
        } else if let Some(value) = part.strip_prefix("bg=") {
            options.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("bar=") {
            options.bar = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("border=") {
            options.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-w=") {
            options.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r=") {
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-tl=") {
            options.radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-tr=") {
            options.radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-br=") {
            options.radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-bl=") {
            options.radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "vertical" {
            vertical = true;
        } else {
            return Err(error(
                "E077",
                line,
                format!("unknown progress property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Progress {
        value: parse_expr(value, line)?,
        id,
        min,
        max,
        options,
        vertical,
        styles,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_radio(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .ok_or_else(|| error("E078", line, "radio needs a label expression"))?;
    let mut id = None;
    let mut value = None;
    let mut selected = None;
    let mut options = BoolControlOptions::default();
    let mut style = RadioStyleSet::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E078", "radio")?;
        } else if let Some(source) = part.strip_prefix("value=") {
            value = Some(parse_expr(strip_wrapping_parens(source), line)?);
        } else if let Some(source) = part.strip_prefix("selected=") {
            selected = Some(parse_expr(strip_wrapping_parens(source), line)?);
        } else if let Some(source) = part.strip_prefix("style=") {
            style.custom = Some(parse_extern_call(
                source,
                line,
                "E078",
                "radio style must be a declared style call",
            )?);
        } else if parse_bool_control_option(part, &mut options, false, false, line)? {
        } else {
            return Err(error(
                "E078",
                line,
                format!("unknown radio property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_radio_status_style(child, &mut style)?;
    }
    Ok(ViewNode::Radio {
        label: parse_expr(label, line)?,
        id,
        value: value.ok_or_else(|| error("E078", line, "radio requires `value=value`"))?,
        selected: selected
            .ok_or_else(|| error("E078", line, "radio requires `selected=condition`"))?,
        options,
        style: Box::new(style),
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E078", line, "radio requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_radio_status_style(
    line: &Line,
    styles: &mut RadioStyleSet,
) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let status = parts.first().map(String::as_str);
    let selected = parts.get(1).map(String::as_str);
    let slot = match (status, selected) {
        (Some("active"), Some("selected")) => &mut styles.active_selected,
        (Some("active"), Some("unselected")) => &mut styles.active_unselected,
        (Some("hovered"), Some("selected")) => &mut styles.hovered_selected,
        (Some("hovered"), Some("unselected")) => &mut styles.hovered_unselected,
        _ => {
            return Err(error(
                "E078",
                line,
                "radio style lines use `<active|hovered> <selected|unselected>`",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E078",
            line,
            format!(
                "duplicate radio {} {} style",
                status.unwrap(),
                selected.unwrap()
            ),
        ));
    }
    let mut style = RadioStatusStyle {
        span: Some(Span::line(line.number)),
        ..RadioStatusStyle::default()
    };
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("bg=") {
            style.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("dot=") {
            style.dot_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border=") {
            style.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-w=") {
            style.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text=") {
            style.text_color = Some(value.to_owned());
        } else {
            return Err(error(
                "E078",
                line,
                format!("unknown radio style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

pub(in crate::parser) fn parse_rule(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    let axis = match parts.get(1).map(String::as_str) {
        Some("horizontal") => Axis::Horizontal,
        Some("vertical") => Axis::Vertical,
        _ => return Err(error("E079", line, "rule uses `rule horizontal|vertical`")),
    };
    let mut id = None;
    let mut thickness = Expr::F64(1.0);
    let mut options = RuleOptions::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E079", "rule")?;
        } else if let Some(value) = part.strip_prefix("thickness=") {
            thickness = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("style=") {
            options.style = Some(match value {
                "default" => RuleStyle::Default,
                "weak" => RuleStyle::Weak,
                _ => return Err(error("E079", line, "rule style must be default or weak")),
            });
        } else if let Some(value) = part.strip_prefix("fill=") {
            options.fill = Some(parse_rule_fill(value, line)?);
        } else if let Some(value) = part.strip_prefix("color=") {
            options.color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("r=") {
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-tl=") {
            options.radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-tr=") {
            options.radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-br=") {
            options.radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-bl=") {
            options.radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("snap=") {
            options.snap = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E079",
                line,
                format!("unknown rule property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Rule {
        axis,
        id,
        thickness,
        options,
        styles,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_rule_fill(source: &str, line: &Line) -> Result<RuleFill, Error> {
    if source == "full" {
        return Ok(RuleFill::Full);
    }
    if let Some(value) = source
        .strip_prefix("percent(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(RuleFill::Percent(parse_expr(value, line)?));
    }
    if let Some(value) = source
        .strip_prefix("pad(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let values = split_top(value, ',');
        let parse = |value: &str| {
            value
                .trim()
                .parse::<u16>()
                .map_err(|_| error("E079", line, "rule padding must be a u16"))
        };
        return match values.as_slice() {
            [value] => Ok(RuleFill::Padded(parse(value)?)),
            [first, second] => Ok(RuleFill::AsymmetricPadding(parse(first)?, parse(second)?)),
            _ => Err(error("E079", line, "rule pad expects one or two values")),
        };
    }
    Err(error(
        "E079",
        line,
        "rule fill must be full, percent(N), pad(N), or pad(A,B)",
    ))
}

pub(in crate::parser) fn parse_space(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    let mut id = None;
    let mut width = None;
    let mut height = None;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E080", "space")?;
        } else if let Some(value) = part.strip_prefix("w=") {
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            height = Some(parse_length(value, line)?);
        } else {
            return Err(error(
                "E080",
                line,
                format!("unknown space property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Space {
        id,
        width,
        height,
        styles,
        span: Span::line(line.number),
    })
}
