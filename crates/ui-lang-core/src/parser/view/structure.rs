use super::*;

pub(in crate::parser) fn parse_float(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E089", line, "float does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E089", line, "float requires exactly one child"));
    }
    let mut id = None;
    let mut scale = Expr::F64(1.0);
    let mut x = Expr::F64(0.0);
    let mut y = Expr::F64(0.0);
    let mut style = FloatStyleOptions::default();
    let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E089", "float")?;
        } else if let Some(value) = part.strip_prefix("scale=") {
            scale = parse(value)?;
        } else if let Some(value) = part.strip_prefix("x=") {
            x = parse(value)?;
        } else if let Some(value) = part.strip_prefix("y=") {
            y = parse(value)?;
        } else if let Some(value) = part.strip_prefix("shadow=") {
            style.shadow_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("shadow-x=") {
            style.shadow_x = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("shadow-y=") {
            style.shadow_y = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("shadow-blur=") {
            style.shadow_blur = Some(parse(value)?);
        } else if parse_radius_option(part, &mut style.radius, "", line)? {
        } else {
            return Err(error(
                "E089",
                line,
                format!("unknown float property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Float {
        id,
        scale,
        x,
        y,
        style,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pin(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E090", line, "pin does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E090", line, "pin requires exactly one child"));
    }
    let mut id = None;
    let mut width = None;
    let mut height = None;
    let mut x = Expr::F64(0.0);
    let mut y = Expr::F64(0.0);
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E090", "pin")?;
        } else if let Some(value) = part.strip_prefix("w=") {
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("x=") {
            x = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("y=") {
            y = parse_expr(strip_wrapping_parens(value), line)?;
        } else {
            return Err(error(
                "E090",
                line,
                format!("unknown pin property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Pin {
        id,
        width,
        height,
        x,
        y,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_sensor(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E091", line, "sensor does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E091", line, "sensor requires exactly one child"));
    }
    let mut id = None;
    let mut options = SensorOptions::default();
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E091", "sensor")?;
        } else if let Some(value) = part.strip_prefix("show=") {
            options.show = Some(parse_size_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("resize=") {
            options.resize = Some(parse_size_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("hide=") {
            options.hide = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("key=") {
            options.key = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("anticipate=") {
            options.anticipate = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("delay=") {
            options.delay_ms = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E091",
                line,
                format!("unknown sensor property `{part}`"),
            ));
        }
    }
    if options.show.is_none() && options.resize.is_none() && options.hide.is_none() {
        return Err(error("E091", line, "sensor requires show, resize, or hide"));
    }
    Ok(ViewNode::Sensor {
        id,
        options,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_responsive(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E092",
            line,
            "responsive does not accept `@` utilities",
        ));
    }
    let mut id = None;
    let mut size = None;
    let mut width = None;
    let mut height = None;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E092", "responsive")?;
        } else if let Some(value) = part.strip_prefix("size=") {
            if size.is_some() {
                return Err(error("E092", line, "responsive repeats `size=`"));
            }
            let Some(value) = value
                .strip_prefix('(')
                .and_then(|value| value.strip_suffix(')'))
            else {
                return Err(error(
                    "E092",
                    line,
                    "responsive size bindings use `size=(width, height)`",
                ));
            };
            let names = split_top(value, ',');
            let [width, height] = names.as_slice() else {
                return Err(error(
                    "E092",
                    line,
                    "responsive size expects width and height bindings",
                ));
            };
            let width = identifier(width, line)?;
            let height = identifier(height, line)?;
            if width == height {
                return Err(error(
                    "E092",
                    line,
                    "responsive size bindings must have different names",
                ));
            }
            size = Some((width, height));
        } else if let Some(value) = part.strip_prefix("w=") {
            if width.is_some() {
                return Err(error("E092", line, "responsive repeats `w=`"));
            }
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            if height.is_some() {
                return Err(error("E092", line, "responsive repeats `h=`"));
            }
            height = Some(parse_length(value, line)?);
        } else {
            return Err(error(
                "E092",
                line,
                format!("unknown responsive property `{part}`"),
            ));
        }
    }
    let Some((width_name, height_name)) = size else {
        return Err(error(
            "E092",
            line,
            "responsive requires `size=(width, height)`",
        ));
    };
    if line.children.len() != 1 {
        return Err(error("E092", line, "responsive requires exactly one child"));
    }
    let content = ResponsiveContent::Size {
        width: width_name,
        height: height_name,
        content: Box::new(parse_view(&line.children[0])?),
    };
    Ok(ViewNode::Responsive {
        id,
        content,
        width,
        height,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_size_route(source: &str, line: &Line) -> Result<Route, Error> {
    parse_payload_route(source, line, 2)
}

pub(in crate::parser) fn parse_payload_route(
    source: &str,
    line: &Line,
    count: usize,
) -> Result<Route, Error> {
    let mut route = parse_route(source, line)?;
    if route.args.is_empty() {
        route.args = vec![RouteArg::Payload; count];
    }
    Ok(route)
}
