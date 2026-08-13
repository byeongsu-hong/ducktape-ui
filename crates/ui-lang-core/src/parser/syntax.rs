use super::*;

pub(in crate::parser) fn line_tree(
    source: &str,
    namespaces: &[Option<String>],
    symbols: std::rc::Rc<std::cell::RefCell<Vec<ParsedSymbol>>>,
) -> Result<Vec<Line>, Error> {
    if namespaces.len() != source.lines().count() {
        return Err(Error::new(
            "E000",
            &Span::line(1),
            "internal source namespace map does not match the document",
        ));
    }
    let mut flat = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        if raw.contains('\t') {
            return Err(Error::new(
                "E009",
                &Span::line(index + 1),
                "tabs are not allowed; use spaces",
            ));
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let indent_bytes = raw.len() - raw.trim_start().len();
        let indent = raw[..indent_bytes].chars().count();
        flat.push(Line {
            number: index + 1,
            indent,
            text: trimmed.into(),
            original_text: trimmed.into(),
            metadata: Vec::new(),
            children: Vec::new(),
            namespace: namespaces[index].clone(),
            symbols: std::rc::Rc::clone(&symbols),
            track_symbols: true,
        });
    }
    if flat.is_empty() {
        return Err(Error::new("E000", &Span::line(1), "empty .ice file"));
    }
    if flat[0].indent != 0 {
        return Err(error(
            "E009",
            &flat[0],
            "the first declaration must not be indented",
        ));
    }
    let mut index = 0;
    parse_block(&flat, &mut index, 0)
}

pub(in crate::parser) fn parse_block(
    flat: &[Line],
    index: &mut usize,
    indent: usize,
) -> Result<Vec<Line>, Error> {
    let mut output = Vec::new();
    while *index < flat.len() {
        if flat[*index].indent < indent {
            break;
        }
        if flat[*index].indent > indent {
            return Err(error("E009", &flat[*index], "unexpected indentation"));
        }
        let mut line = flat[*index].clone();
        *index += 1;
        if *index < flat.len() && flat[*index].indent > indent {
            let child_indent = flat[*index].indent;
            line.children = parse_block(flat, index, child_indent)?;
        }
        output.push(line);
    }
    Ok(output)
}

pub(in crate::parser) fn parse_signature(
    source: &str,
    line: &Line,
) -> Result<(String, String), Error> {
    let (name, args) = signature_parts(source, line)?;
    Ok((line.qualify(&qualified_identifier(name, line)?), args))
}

pub(in crate::parser) fn parse_local_signature(
    source: &str,
    line: &Line,
) -> Result<(String, String), Error> {
    let (name, args) = signature_parts(source, line)?;
    Ok((identifier(name, line)?, args))
}

pub(in crate::parser) fn parse_component_signature(
    source: &str,
    line: &Line,
) -> Result<(String, String), Error> {
    let (name, args) = signature_parts(source, line)?;
    Ok((line.qualify(&component_identifier(name, line)?), args))
}

pub(in crate::parser) fn signature_parts<'a>(
    source: &'a str,
    line: &Line,
) -> Result<(&'a str, String), Error> {
    let open = source
        .find('(')
        .ok_or_else(|| error("E024", line, "expected `(`"))?;
    let close = matching_paren(source, line)?;
    if !source[close + 1..].trim().is_empty() {
        return Err(error("E024", line, "unexpected text after `)`"));
    }
    Ok((source[..open].trim(), source[open + 1..close].into()))
}

fn advance_string(ch: char, string: &mut bool, escaped: &mut bool) {
    if !*string {
        *string = ch == '"';
    } else if *escaped {
        *escaped = false;
    } else if ch == '\\' {
        *escaped = true;
    } else if ch == '"' {
        *string = false;
    }
}

fn closing_paren(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    let mut string = false;
    let mut escaped = false;
    for (index, ch) in source.char_indices().skip_while(|(index, _)| *index < open) {
        advance_string(ch, &mut string, &mut escaped);
        if !string {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
        }
    }
    None
}

pub(in crate::parser) fn matching_paren(source: &str, line: &Line) -> Result<usize, Error> {
    let open = source
        .find('(')
        .ok_or_else(|| error("E024", line, "expected `(`"))?;
    closing_paren(source, open).ok_or_else(|| error("E024", line, "missing closing `)`"))
}

pub(crate) fn split_words(source: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut string = false;
    let mut escaped = false;
    for (byte, ch) in source.char_indices() {
        advance_string(ch, &mut string, &mut escaped);
        match ch {
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            ch if ch.is_whitespace() && !string && depth == 0 => {
                if start < byte {
                    output.push(source[start..byte].into());
                }
                start = byte + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start < source.len() {
        output.push(source[start..].into());
    }
    output
}

pub(in crate::parser) fn split_top(source: &str, delimiter: char) -> Vec<&str> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut string = false;
    let mut escaped = false;
    for (index, ch) in source.char_indices() {
        advance_string(ch, &mut string, &mut escaped);
        match ch {
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            ch if ch == delimiter && !string && depth == 0 => {
                output.push(source[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    output.push(source[start..].trim());
    output
}

pub(in crate::parser) fn split_top_once(source: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut depth = 0;
    let mut string = false;
    let mut escaped = false;
    for (index, ch) in source.char_indices() {
        advance_string(ch, &mut string, &mut escaped);
        match ch {
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            ch if ch == delimiter && !string && depth == 0 => {
                return Some((&source[..index], &source[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn split_top_marker<'a>(source: &'a str, marker: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0;
    let mut string = false;
    let mut escaped = false;
    let bytes = source.as_bytes();
    let mut index = 0;
    while index + marker.len() <= bytes.len() {
        let ch = source[index..].chars().next()?;
        advance_string(ch, &mut string, &mut escaped);
        match ch {
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            _ => {}
        }
        let part_of_binding = marker == "->" && index > 0 && bytes[index - 1] == b'<';
        if !string && depth == 0 && !part_of_binding && source[index..].starts_with(marker) {
            return Some((&source[..index], &source[index + marker.len()..]));
        }
        index += ch.len_utf8();
    }
    None
}

pub(in crate::parser) fn strip_wrapping_parens(source: &str) -> &str {
    let source = source.trim();
    if source.starts_with('(') && closing_paren(source, 0) == Some(source.len() - 1) {
        &source[1..source.len() - 1]
    } else {
        source
    }
}

pub(in crate::parser) fn string_literal(source: &str, line: &Line) -> Result<String, Error> {
    match parse_expr(source, line)? {
        Expr::Str(value) => Ok(value),
        _ => Err(error("E071", line, "expected string literal")),
    }
}

pub(in crate::parser) fn literal_type(expr: &Expr) -> Option<Type> {
    Some(match expr {
        Expr::Bool(_) => Type::Bool,
        Expr::I64(_) => Type::I64,
        Expr::F64(_) => Type::F64,
        Expr::Str(_) => Type::Str,
        Expr::Bytes(_) => Type::Bytes,
        Expr::Call { name, .. } if matches!(crate::unqualified_name(name), "encoded" | "rgba") => {
            Type::Image
        }
        Expr::EmptyList => return None,
        Expr::List(values) => {
            let first = values.first().and_then(literal_type)?;
            if values
                .iter()
                .skip(1)
                .all(|value| literal_type(value).as_ref() == Some(&first))
            {
                Type::List(Box::new(first))
            } else {
                return None;
            }
        }
        Expr::None => return None,
        _ => return None,
    })
}

pub(in crate::parser) fn valid_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

pub(in crate::parser) fn identifier(source: &str, line: &Line) -> Result<String, Error> {
    if crate::valid_identifier(source) {
        Ok(source.into())
    } else {
        Err(error(
            "E072",
            line,
            format!("invalid identifier `{source}`"),
        ))
    }
}

pub(in crate::parser) fn qualified_identifier(source: &str, line: &Line) -> Result<String, Error> {
    if !source.is_empty() && source.split("::").all(crate::valid_identifier) {
        Ok(source.into())
    } else {
        Err(error(
            "E072",
            line,
            format!("invalid qualified identifier `{source}`"),
        ))
    }
}

pub(in crate::parser) fn component_identifier(source: &str, line: &Line) -> Result<String, Error> {
    if SymbolKind::Component.accepts(source) {
        Ok(source.into())
    } else {
        Err(error(
            "E072",
            line,
            format!("invalid component name `{source}`"),
        ))
    }
}

pub(in crate::parser) fn kebab_identifier(source: &str, line: &Line) -> Result<String, Error> {
    if !source.is_empty()
        && source
            .chars()
            .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
    {
        Ok(source.into())
    } else {
        Err(error("E072", line, format!("invalid id `{source}`")))
    }
}

pub(in crate::parser) fn rust_path(source: &str, line: &Line) -> Result<String, Error> {
    if source
        .split("::")
        .enumerate()
        .all(|(index, part)| index == 0 && part == "crate" || crate::valid_rust_identifier(part))
    {
        Ok(source.into())
    } else {
        Err(error("E073", line, format!("invalid Rust path `{source}`")))
    }
}

pub(in crate::parser) fn ensure_leaf(line: &Line) -> Result<(), Error> {
    if line.children.is_empty() {
        Ok(())
    } else {
        Err(error(
            "E009",
            line,
            "this line cannot have an indented block",
        ))
    }
}

pub(in crate::parser) fn error(
    code: &'static str,
    line: &Line,
    message: impl Into<String>,
) -> Error {
    let message = message.into();
    let line = line.diagnostic_origin(&message);
    Error::new(code, &Span::line(line.number), message)
}
