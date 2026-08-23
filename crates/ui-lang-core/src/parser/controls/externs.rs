use super::*;

pub(in crate::parser) fn parse_extern_component(
    parts: &[&str],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error(
            "E083",
            line,
            "extern components own their styling and do not accept `@` utilities",
        ));
    }
    let mut id = None;
    let mut signature = None;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E083", "extern component")?;
        } else if signature.replace(part).is_some() {
            return Err(error(
                "E083",
                line,
                "extern component uses `extern name(args) #id -> handler _`",
            ));
        }
    }
    let signature = signature.ok_or_else(|| {
        error(
            "E083",
            line,
            "extern component uses `extern name(args) #id -> handler _`",
        )
    })?;
    let (function, args) = parse_signature(signature, line)?;
    Ok(ViewNode::ExternComponent {
        function,
        id,
        args: parse_expr_list(&args, line)?,
        route: route.map(|route| parse_route(route, line)).transpose()?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_themer(
    parts: &[&str],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error(
            "E094",
            line,
            "themer uses `themer name(args) #id -> handler _` and owns its styling",
        ));
    }
    let mut id = None;
    let mut signature = None;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E094", "themer")?;
        } else if signature.replace(part).is_some() {
            return Err(error(
                "E094",
                line,
                "themer uses `themer name(args) #id -> handler _` and owns its styling",
            ));
        }
    }
    let signature = signature.ok_or_else(|| {
        error(
            "E094",
            line,
            "themer uses `themer name(args) #id -> handler _` and owns its styling",
        )
    })?;
    let (function, args) = parse_signature(signature, line)?;
    Ok(ViewNode::Themer {
        function,
        id,
        args: parse_expr_list(&args, line)?,
        route: route.map(|route| parse_route(route, line)).transpose()?,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_shader(
    parts: &[&str],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error("E191", line, "shader does not accept `@` utilities"));
    }
    if parts.len() < 2 {
        return Err(error(
            "E191",
            line,
            "shader uses `shader name(args) w=fill h=120.0 -> handler _`",
        ));
    }
    let (function, args) = parse_signature(parts[1], line)?;
    let mut id = None;
    let mut width = None;
    let mut height = None;
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E191", "shader")?;
        } else if let Some(value) = part.strip_prefix("w=") {
            if width.is_some() {
                return Err(error("E191", line, "duplicate shader width"));
            }
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            if height.is_some() {
                return Err(error("E191", line, "duplicate shader height"));
            }
            height = Some(parse_length(value, line)?);
        } else {
            return Err(error(
                "E191",
                line,
                format!("unknown shader property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Shader {
        function,
        id,
        args: parse_expr_list(&args, line)?,
        width,
        height,
        route: route.map(|route| parse_route(route, line)).transpose()?,
        span: Span::line(line.number),
    })
}
