use super::*;

const TEST_ERROR: &str = "E194";

pub(in crate::parser) fn parse_test_decl(source: &str, line: &Line) -> Result<TestDecl, Error> {
    let name = identifier(source, line)?;
    if !crate::canonical_snake(&name) {
        return Err(error(TEST_ERROR, line, "test names use snake_case"));
    }

    let mut preset = None;
    let mut viewport = None;
    let mut timeout_ms = None;
    let mut mount = None;
    let mut targets = Vec::new();
    let mut steps = Vec::new();
    let mut executable = false;

    for child in &line.children {
        let declaration = child.text.starts_with("preset ")
            || child.text.starts_with("viewport ")
            || child.text.starts_with("timeout ")
            || child.text == "mount"
            || child.text.starts_with("target ");
        if declaration && executable {
            return Err(error(
                TEST_ERROR,
                child,
                "test configuration and targets must precede executable steps",
            ));
        }

        if let Some(value) = child.text.strip_prefix("preset ") {
            ensure_leaf(child)?;
            if preset.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares preset more than once",
                ));
            }
            preset = Some(identifier(value.trim(), child)?);
        } else if let Some(value) = child.text.strip_prefix("viewport ") {
            ensure_leaf(child)?;
            if viewport.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares viewport more than once",
                ));
            }
            let values = split_words(value);
            let [width, height] = values.as_slice() else {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "viewport uses `viewport width height`",
                ));
            };
            viewport = Some((
                parse_viewport_dimension(width, child)?,
                parse_viewport_dimension(height, child)?,
            ));
        } else if let Some(value) = child.text.strip_prefix("timeout ") {
            ensure_leaf(child)?;
            if timeout_ms.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares timeout more than once",
                ));
            }
            timeout_ms = Some(parse_duration(value.trim(), child).map_err(|_| {
                error(
                    TEST_ERROR,
                    child,
                    "timeout must be a positive duration such as `500ms` or `2s`",
                )
            })?);
        } else if child.text == "mount" {
            if mount.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares mount more than once",
                ));
            }
            let [root] = child.children.as_slice() else {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "mount must contain exactly one root node",
                ));
            };
            mount = Some(parse_view(root)?);
        } else if let Some(value) = child.text.strip_prefix("target ") {
            ensure_leaf(child)?;
            let Some((alias, target)) = split_top_once(value, '=') else {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "target aliases use `target name = #scoped/id`",
                ));
            };
            let alias_source = alias.trim();
            let alias = identifier(alias_source, child)?;
            child.record_scoped_symbol(
                SymbolKind::TestTarget,
                Some(&name),
                &alias,
                true,
                alias_source,
            );
            let target_source = target.trim();
            let target = parse_widget_target(target_source, child)?;
            record_test_target_alias_references(target_source, child, &name, &targets)?;
            targets.push(TestTargetDecl {
                name: alias,
                target,
                span: line_span(child),
            });
        } else {
            executable = true;
            steps.push(parse_test_step(child, &name, &targets)?);
        }
    }

    Ok(TestDecl {
        name,
        preset,
        viewport,
        timeout_ms,
        mount,
        targets,
        steps,
        span: Span::line(line.number),
    })
}

fn parse_viewport_dimension(source: &str, line: &Line) -> Result<f64, Error> {
    source
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= f32::MAX as f64)
        .ok_or_else(|| {
            error(
                TEST_ERROR,
                line,
                "viewport dimensions must be positive finite numbers in the f32 range",
            )
        })
}

fn parse_test_step(
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestStep, Error> {
    ensure_leaf(line)?;
    let kind = if let Some(target) = line.text.strip_prefix("click ") {
        TestStepKind::Click(parse_test_target_ref(target, line, scope, targets)?)
    } else if let Some(target) = line.text.strip_prefix("hover ") {
        TestStepKind::Hover(parse_test_target_ref(target, line, scope, targets)?)
    } else if let Some(target) = line.text.strip_prefix("press ") {
        TestStepKind::Press(parse_test_target_ref(target, line, scope, targets)?)
    } else if line.text == "release" {
        TestStepKind::Release
    } else if let Some(value) = line.text.strip_prefix("type ") {
        TestStepKind::Type(parse_test_expr(value.trim(), line, scope, targets)?)
    } else if let Some(value) = line.text.strip_prefix("key ") {
        TestStepKind::Key(match value.trim() {
            "enter" => TestKey::Enter,
            "escape" => TestKey::Escape,
            "tab" => TestKey::Tab,
            "backspace" => TestKey::Backspace,
            _ => {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "key must be enter, escape, tab, or backspace",
                ));
            }
        })
    } else if let Some(values) = line.text.strip_prefix("resize ") {
        record_test_alias_references(values, line, scope, targets);
        let values = split_words(values);
        let [width, height] = values.as_slice() else {
            return Err(error(
                TEST_ERROR,
                line,
                "resize uses `resize width height`; wrap compound expressions in parentheses",
            ));
        };
        TestStepKind::Resize(
            parse_expr(strip_wrapping_parens(width), line)?,
            parse_expr(strip_wrapping_parens(height), line)?,
        )
    } else if let Some(call) = line.text.strip_prefix("dispatch ") {
        let call = call.trim();
        let (handler, args) = if call.contains('(') {
            let (handler, args) = parse_signature(call, line)?;
            let parsed = parse_expr_list(&args, line)?;
            let open = call.find('(').expect("signature parser requires `(`");
            let close = matching_paren(call, line)?;
            record_test_alias_references(&call[open + 1..close], line, scope, targets);
            (handler, parsed)
        } else {
            (identifier(call, line)?, Vec::new())
        };
        line.record_symbol(SymbolKind::Handler, &handler, false, call);
        TestStepKind::Dispatch { handler, args }
    } else if let Some(expectation) = line.text.strip_prefix("expect ") {
        TestStepKind::Expect(parse_test_expectation(
            expectation.trim(),
            line,
            scope,
            targets,
        )?)
    } else {
        return Err(error(
            TEST_ERROR,
            line,
            format!("unknown test step `{}`", line.text),
        ));
    };
    Ok(TestStep {
        kind,
        span: line_span(line),
    })
}

fn line_span(line: &Line) -> Span {
    Span {
        line: line.number,
        column: line.indent + 1,
    }
}

fn parse_test_expectation(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestExpectation, Error> {
    if let Some(target) = source.strip_prefix("exists ") {
        return Ok(TestExpectation::Exists(parse_test_target_ref(
            target, line, scope, targets,
        )?));
    }
    if let Some(target) = source.strip_prefix("missing ") {
        return Ok(TestExpectation::Missing(parse_test_target_ref(
            target, line, scope, targets,
        )?));
    }
    if let Some(value) = source.strip_prefix("no text ") {
        return parse_text_expectation(value, true, line, scope, targets);
    }
    if let Some(value) = source.strip_prefix("text ") {
        return parse_text_expectation(value, false, line, scope, targets);
    }
    if let Some((left, right)) = split_top_marker(source, "~=") {
        return Ok(TestExpectation::Approx {
            left: parse_test_expr(left.trim(), line, scope, targets)?,
            right: parse_test_expr(right.trim(), line, scope, targets)?,
        });
    }
    Ok(TestExpectation::Expr(parse_test_expr(
        source, line, scope, targets,
    )?))
}

fn parse_text_expectation(
    source: &str,
    negated: bool,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestExpectation, Error> {
    let (value, within) = split_top_marker(source, " within ")
        .map_or((source, None), |(value, target)| (value, Some(target)));
    Ok(TestExpectation::Text {
        value: parse_test_expr(value.trim(), line, scope, targets)?,
        within: within
            .map(|target| parse_test_target_ref(target, line, scope, targets))
            .transpose()?,
        negated,
    })
}

fn parse_test_target_ref(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestTargetRef, Error> {
    let source = source.trim();
    if source.starts_with('#') {
        let target = parse_widget_target(source, line)?;
        record_test_target_alias_references(source, line, scope, targets)?;
        Ok(TestTargetRef::Id(target))
    } else {
        let alias = identifier(source, line)?;
        line.record_scoped_symbol(SymbolKind::TestTarget, Some(scope), &alias, false, source);
        Ok(TestTargetRef::Alias(alias))
    }
}

fn record_test_target_alias_references(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<(), Error> {
    let source = source.strip_prefix('#').unwrap_or(source);
    for segment in split_top(source, '/') {
        let segment = segment.strip_prefix('#').unwrap_or(segment);
        if let Some(open) = segment.find('(') {
            let close = matching_paren(segment, line)?;
            record_test_alias_references(&segment[open + 1..close], line, scope, targets);
        }
    }
    Ok(())
}

fn parse_test_expr(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<Expr, Error> {
    let value = parse_expr(source, line)?;
    record_test_alias_references(source, line, scope, targets);
    Ok(value)
}

fn record_test_alias_references(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut string = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            string = true;
            index += 1;
            continue;
        }
        if byte == b'_' || byte.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index] == b'_' || bytes[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            let name = &source[start..index];
            let field = source[..start]
                .bytes()
                .rev()
                .find(|byte| !byte.is_ascii_whitespace())
                == Some(b'.');
            if !field
                && !test_path_is_call(bytes, index)
                && targets.iter().any(|target| target.name == name)
            {
                line.record_scoped_symbol(
                    SymbolKind::TestTarget,
                    Some(scope),
                    name,
                    false,
                    &source[start..index],
                );
            }
            continue;
        }
        index += 1;
    }
}

fn test_path_is_call(bytes: &[u8], mut index: usize) -> bool {
    loop {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if bytes.get(index) != Some(&b'.') {
            break;
        }
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if !bytes
            .get(index)
            .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphabetic())
        {
            return false;
        }
        index += 1;
        while bytes
            .get(index)
            .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
        {
            index += 1;
        }
    }
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    bytes.get(index) == Some(&b'(')
}
