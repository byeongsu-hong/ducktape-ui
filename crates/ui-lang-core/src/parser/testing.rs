use super::*;

const TEST_ERROR: &str = "E194";

pub(in crate::parser) fn parse_test_decl(source: &str, line: &Line) -> Result<TestDecl, Error> {
    let name = identifier(source, line)?;
    if !test_name(&name) {
        return Err(error(TEST_ERROR, line, "test names use snake_case"));
    }

    let mut preset = None;
    let mut viewport = None;
    let mut timeout_ms = None;
    let mut theme = None;
    let mut scale_factor = None;
    let mut locale = None;
    let mut platform = None;
    let mut reduced_motion = None;
    let mut mount = None;
    let mut targets = Vec::new();
    let mut steps = Vec::new();
    let mut executable = false;

    for child in &line.children {
        let declaration = child.text.starts_with("preset ")
            || child.text.starts_with("viewport ")
            || child.text.starts_with("timeout ")
            || child.text.starts_with("theme ")
            || child.text.starts_with("scale ")
            || child.text.starts_with("locale ")
            || child.text.starts_with("platform ")
            || child.text.starts_with("reduced-motion ")
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
        } else if let Some(value) = child.text.strip_prefix("theme ") {
            ensure_leaf(child)?;
            if theme.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares theme more than once",
                ));
            }
            theme = Some(parse_test_theme(value.trim(), child)?);
        } else if let Some(value) = child.text.strip_prefix("scale ") {
            ensure_leaf(child)?;
            if scale_factor.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares scale more than once",
                ));
            }
            scale_factor = Some(parse_viewport_dimension(value.trim(), child).map_err(|_| {
                error(
                    TEST_ERROR,
                    child,
                    "test scale must be a positive finite number in the f32 range",
                )
            })?);
        } else if let Some(value) = child.text.strip_prefix("locale ") {
            ensure_leaf(child)?;
            if locale.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares locale more than once",
                ));
            }
            let value = string_literal(value.trim(), child)?;
            if value.is_empty() {
                return Err(error(TEST_ERROR, child, "test locale must not be empty"));
            }
            locale = Some(value);
        } else if let Some(value) = child.text.strip_prefix("platform ") {
            ensure_leaf(child)?;
            if platform.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares platform more than once",
                ));
            }
            platform = Some(match value.trim() {
                "linux" => TestPlatform::Linux,
                "windows" => TestPlatform::Windows,
                "macos" => TestPlatform::Macos,
                "wasm" => TestPlatform::Wasm,
                _ => {
                    return Err(error(
                        TEST_ERROR,
                        child,
                        "test platform must be linux, windows, macos, or wasm",
                    ));
                }
            });
        } else if let Some(value) = child.text.strip_prefix("reduced-motion ") {
            ensure_leaf(child)?;
            if reduced_motion.is_some() {
                return Err(error(
                    TEST_ERROR,
                    child,
                    "test declares reduced-motion more than once",
                ));
            }
            reduced_motion = Some(match value.trim() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(error(
                        TEST_ERROR,
                        child,
                        "test reduced-motion must be true or false",
                    ));
                }
            });
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
            let target = parse_test_target_decl(target_source, child, &name, &targets)?;
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
        theme,
        scale_factor,
        locale,
        platform,
        reduced_motion,
        mount,
        targets,
        steps,
        span: Span::line(line.number),
    })
}

fn test_name(name: &str) -> bool {
    name.split('_').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
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

fn parse_test_target_decl(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<WidgetTarget, Error> {
    if source.starts_with('#') {
        let target = parse_widget_target(source, line)?;
        record_test_target_alias_references(source, line, scope, targets)?;
        return Ok(target);
    }

    let Some((base_source, descendant_source)) = split_top_once(source, '/') else {
        return Err(error(
            TEST_ERROR,
            line,
            "target aliases use `target name = #scoped/id` or `target name = base/descendant`",
        ));
    };
    let base_source = base_source.trim();
    let base = identifier(base_source, line)?;
    let Some(target) = targets.iter().find(|target| target.name == base) else {
        return Err(error(
            TEST_ERROR,
            line,
            format!("relative target base `{base}` must name an earlier target alias"),
        ));
    };
    line.record_scoped_symbol(
        SymbolKind::TestTarget,
        Some(scope),
        &base,
        false,
        base_source,
    );

    let descendant_source = descendant_source.trim();
    let descendant = parse_widget_target(&format!("#{descendant_source}"), line)?;
    record_test_target_alias_references(descendant_source, line, scope, targets)?;
    let mut segments = target.target.segments.clone();
    segments.extend(descendant.segments);
    Ok(WidgetTarget {
        segments,
        window: None,
    })
}

fn parse_test_step(
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestStep, Error> {
    ensure_leaf(line)?;
    let kind = if let Some(value) = line.text.strip_prefix("double-click ") {
        let (target, button) = parse_target_and_button(value, line, scope, targets)?;
        TestStepKind::Click {
            target,
            button,
            count: 2,
        }
    } else if let Some(value) = line.text.strip_prefix("click-at ") {
        let values = split_words(value);
        let (x, y, button) = match values.as_slice() {
            [x, y] => (x.as_str(), y.as_str(), TestMouseButton::Left),
            [x, y, button] => (x.as_str(), y.as_str(), parse_mouse_button(button, line)?),
            _ => {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "click-at uses `click-at x y [left|right|middle|back|forward]`",
                ));
            }
        };
        TestStepKind::ClickAt {
            x: parse_test_expr(strip_wrapping_parens(x), line, scope, targets)?,
            y: parse_test_expr(strip_wrapping_parens(y), line, scope, targets)?,
            button,
            count: 1,
        }
    } else if let Some(value) = line.text.strip_prefix("click ") {
        let (target, button) = parse_target_and_button(value, line, scope, targets)?;
        TestStepKind::Click {
            target,
            button,
            count: 1,
        }
    } else if let Some(target) = line.text.strip_prefix("hover ") {
        TestStepKind::Hover(parse_test_target_ref(target, line, scope, targets)?)
    } else if let Some(target) = line.text.strip_prefix("enter ") {
        TestStepKind::Enter(parse_test_target_ref(target, line, scope, targets)?)
    } else if line.text == "leave" {
        TestStepKind::Leave
    } else if let Some(value) = line.text.strip_prefix("move ") {
        let values = split_words(value);
        match values.as_slice() {
            [target] => TestStepKind::Move(TestPointerPosition::Target(parse_test_target_ref(
                target, line, scope, targets,
            )?)),
            [x, y] => TestStepKind::Move(TestPointerPosition::Point(
                parse_test_expr(strip_wrapping_parens(x), line, scope, targets)?,
                parse_test_expr(strip_wrapping_parens(y), line, scope, targets)?,
            )),
            _ => {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "move uses `move target` or `move x y`",
                ));
            }
        }
    } else if let Some(value) = line.text.strip_prefix("press ") {
        let (target, button) = parse_target_and_button(value, line, scope, targets)?;
        TestStepKind::Press { target, button }
    } else if line.text == "release" {
        TestStepKind::Release(TestMouseButton::Left)
    } else if let Some(button) = line.text.strip_prefix("release ") {
        TestStepKind::Release(parse_mouse_button(button.trim(), line)?)
    } else if let Some(values) = line.text.strip_prefix("wheel ") {
        let values = split_words(values);
        let (unit, x, y) = match values.as_slice() {
            [x, y] => (TestWheelUnit::Pixels, x.as_str(), y.as_str()),
            [unit, x, y] if unit == "pixels" => (TestWheelUnit::Pixels, x.as_str(), y.as_str()),
            [unit, x, y] if unit == "lines" => (TestWheelUnit::Lines, x.as_str(), y.as_str()),
            _ => {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "wheel uses `wheel [pixels|lines] x y`",
                ));
            }
        };
        TestStepKind::Wheel {
            unit,
            x: parse_test_expr(strip_wrapping_parens(x), line, scope, targets)?,
            y: parse_test_expr(strip_wrapping_parens(y), line, scope, targets)?,
        }
    } else if let Some(values) = line
        .text
        .strip_prefix("scroll-to ")
        .or_else(|| line.text.strip_prefix("scroll-by "))
    {
        let values = split_words(values);
        let [target, x, y] = values.as_slice() else {
            return Err(error(
                TEST_ERROR,
                line,
                "scroll uses `scroll-to target x y` or `scroll-by target x y`",
            ));
        };
        TestStepKind::Scroll {
            mode: if line.text.starts_with("scroll-to ") {
                TestScrollMode::To
            } else {
                TestScrollMode::By
            },
            target: parse_test_target_ref(target, line, scope, targets)?,
            x: parse_test_expr(strip_wrapping_parens(x), line, scope, targets)?,
            y: parse_test_expr(strip_wrapping_parens(y), line, scope, targets)?,
        }
    } else if let Some(values) = line.text.strip_prefix("snap ") {
        let values = split_words(values);
        let [target, x, y] = values.as_slice() else {
            return Err(error(TEST_ERROR, line, "snap uses `snap target x y`"));
        };
        TestStepKind::Snap {
            target: parse_test_target_ref(target, line, scope, targets)?,
            x: parse_test_expr(strip_wrapping_parens(x), line, scope, targets)?,
            y: parse_test_expr(strip_wrapping_parens(y), line, scope, targets)?,
        }
    } else if let Some(target) = line.text.strip_prefix("snap-end ") {
        TestStepKind::SnapEnd(parse_test_target_ref(target, line, scope, targets)?)
    } else if let Some(values) = line.text.strip_prefix("drag ") {
        let values = split_words(values);
        let [from, to] = values.as_slice() else {
            return Err(error(
                TEST_ERROR,
                line,
                "drag uses `drag from-target to-target`",
            ));
        };
        TestStepKind::Drag {
            from: parse_test_target_ref(from, line, scope, targets)?,
            to: parse_test_target_ref(to, line, scope, targets)?,
        }
    } else if let Some(target) = line.text.strip_prefix("drop ") {
        TestStepKind::Drop(parse_test_target_ref(target, line, scope, targets)?)
    } else if let Some(target) = line.text.strip_prefix("focus ") {
        TestStepKind::Focus(parse_test_target_ref(target, line, scope, targets)?)
    } else if line.text == "focus-next" {
        TestStepKind::FocusNext
    } else if line.text == "focus-previous" {
        TestStepKind::FocusPrevious
    } else if line.text == "blur" {
        TestStepKind::Blur
    } else if let Some(value) = line.text.strip_prefix("window ") {
        parse_window_step(value, line, scope, targets)?
    } else if let Some(value) = line.text.strip_prefix("type ") {
        TestStepKind::Type(parse_test_expr(value.trim(), line, scope, targets)?)
    } else if line.text == "clear" {
        TestStepKind::Clear
    } else if let Some(value) = line.text.strip_prefix("replace ") {
        TestStepKind::Replace(parse_test_expr(value.trim(), line, scope, targets)?)
    } else if let Some(values) = line.text.strip_prefix("select ") {
        let (start, end) = parse_test_pair(values, "select", line, scope, targets)?;
        TestStepKind::Select(start, end)
    } else if line.text == "select-all" {
        TestStepKind::SelectAll
    } else if line.text == "cursor front" {
        TestStepKind::CursorFront
    } else if line.text == "cursor end" {
        TestStepKind::CursorEnd
    } else if let Some(value) = line.text.strip_prefix("cursor ") {
        TestStepKind::Cursor(parse_test_expr(value.trim(), line, scope, targets)?)
    } else if let Some(value) = line.text.strip_prefix("composition ") {
        TestStepKind::Composition(parse_composition(value, line, scope, targets)?)
    } else if let Some(value) = line.text.strip_prefix("key-down ") {
        TestStepKind::KeyDown(parse_test_key_event(value.trim(), line, true)?)
    } else if let Some(value) = line.text.strip_prefix("key-up ") {
        TestStepKind::KeyUp(parse_test_key_event(value.trim(), line, false)?)
    } else if let Some(value) = line.text.strip_prefix("key ") {
        TestStepKind::Key(parse_test_key(value.trim(), line)?)
    } else if line.text == "modifiers" {
        TestStepKind::Modifiers(TestModifiers::default())
    } else if let Some(value) = line.text.strip_prefix("modifiers ") {
        TestStepKind::Modifiers(parse_test_modifiers(&split_words(value), line)?)
    } else if let Some(value) = line.text.strip_prefix("chord ") {
        let values = split_words(value);
        let Some((key, modifiers)) = values.split_last() else {
            return Err(error(TEST_ERROR, line, "chord requires a key"));
        };
        TestStepKind::Chord {
            modifiers: parse_test_modifiers(modifiers, line)?,
            key: parse_test_key(key, line)?,
        }
    } else if let Some(value) = line.text.strip_prefix("repeat ") {
        let values = split_words(value);
        let [key, count] = values.as_slice() else {
            return Err(error(TEST_ERROR, line, "repeat uses `repeat key count`"));
        };
        TestStepKind::Repeat {
            key: parse_test_key(key, line)?,
            count: parse_test_expr(strip_wrapping_parens(count), line, scope, targets)?,
        }
    } else if let Some(value) = line.text.strip_prefix("tap ") {
        let values = split_words(value);
        let (target, count) = match values.as_slice() {
            [target] => (target.as_str(), 1),
            [target, count] => (
                target.as_str(),
                parse_static_count(count, "tap count", line)?,
            ),
            _ => return Err(error(TEST_ERROR, line, "tap uses `tap target [count]`")),
        };
        TestStepKind::Tap {
            target: parse_test_target_ref(target, line, scope, targets)?,
            count,
        }
    } else if let Some(value) = line.text.strip_prefix("touch ") {
        parse_touch_step(value, line, scope, targets)?
    } else if let Some(values) = line.text.strip_prefix("resize ") {
        let (width, height) = parse_test_pair(values, "resize", line, scope, targets)?;
        TestStepKind::Resize(width, height)
    } else if let Some(value) = line.text.strip_prefix("system-theme ") {
        TestStepKind::SystemTheme(parse_test_theme(value.trim(), line)?)
    } else if let Some(value) = line.text.strip_prefix("file-hover ") {
        TestStepKind::FileHover(parse_test_expr(value.trim(), line, scope, targets)?)
    } else if let Some(value) = line.text.strip_prefix("file-drop ") {
        TestStepKind::FileDrop(parse_test_expr(value.trim(), line, scope, targets)?)
    } else if line.text == "file-leave" {
        TestStepKind::FileLeave
    } else if let Some(value) = line.text.strip_prefix("wait ") {
        TestStepKind::Wait(parse_test_duration(value, line, "wait")?)
    } else if let Some(value) = line.text.strip_prefix("advance ") {
        TestStepKind::Advance(parse_test_duration(value, line, "advance")?)
    } else if line.text == "idle" {
        TestStepKind::Idle
    } else if let Some(value) = line.text.strip_prefix("capture ") {
        let name = identifier(value.trim(), line)?;
        if !test_name(&name) {
            return Err(error(TEST_ERROR, line, "capture names use snake_case"));
        }
        TestStepKind::Capture(name)
    } else if let Some(value) = line.text.strip_prefix("a11y ") {
        parse_accessibility_action(value, line, scope, targets)?
    } else if let Some(value) = line.text.strip_prefix("tray choose ") {
        TestStepKind::TrayChoose(parse_test_expr(value.trim(), line, scope, targets)?)
    } else if let Some(call) = line.text.strip_prefix("dispatch ") {
        let call = call.trim();
        let (handler, args) = if call.contains('(') {
            let (handler, args) = parse_local_signature(call, line)?;
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

fn parse_test_theme(source: &str, line: &Line) -> Result<TestTheme, Error> {
    match source {
        "light" => Ok(TestTheme::Light),
        "dark" => Ok(TestTheme::Dark),
        "none" => Ok(TestTheme::None),
        _ => Err(error(
            TEST_ERROR,
            line,
            "test theme must be light, dark, or none",
        )),
    }
}

fn parse_mouse_button(source: &str, line: &Line) -> Result<TestMouseButton, Error> {
    match source {
        "left" => Ok(TestMouseButton::Left),
        "right" => Ok(TestMouseButton::Right),
        "middle" => Ok(TestMouseButton::Middle),
        "back" => Ok(TestMouseButton::Back),
        "forward" => Ok(TestMouseButton::Forward),
        _ => Err(error(
            TEST_ERROR,
            line,
            "mouse button must be left, right, middle, back, or forward",
        )),
    }
}

fn parse_target_and_button(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<(TestTargetRef, TestMouseButton), Error> {
    let values = split_words(source);
    let (target, button) = match values.as_slice() {
        [target] => (target.as_str(), TestMouseButton::Left),
        [target, button] => (target.as_str(), parse_mouse_button(button, line)?),
        _ => {
            return Err(error(
                TEST_ERROR,
                line,
                "pointer action uses `action target [left|right|middle|back|forward]`",
            ));
        }
    };
    Ok((parse_test_target_ref(target, line, scope, targets)?, button))
}

fn parse_test_pair(
    source: &str,
    action: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<(Expr, Expr), Error> {
    let values = split_words(source);
    let [first, second] = values.as_slice() else {
        return Err(error(
            TEST_ERROR,
            line,
            format!(
                "{action} uses `{action} first second`; wrap compound expressions in parentheses"
            ),
        ));
    };
    Ok((
        parse_test_expr(strip_wrapping_parens(first), line, scope, targets)?,
        parse_test_expr(strip_wrapping_parens(second), line, scope, targets)?,
    ))
}

fn parse_test_key(source: &str, line: &Line) -> Result<TestKey, Error> {
    if source.starts_with('"') {
        let value = string_literal(source, line)?;
        if value.is_empty() {
            return Err(error(TEST_ERROR, line, "character key must not be empty"));
        }
        return Ok(TestKey::Character(value));
    }
    let (exact, ergonomic) = test_key_name_shape(source);
    if !exact && !ergonomic {
        return Err(error(
            TEST_ERROR,
            line,
            "named keys use lowercase names like arrow-left or exact iced variants like TVInputHDMI1; characters use strings",
        ));
    }
    Ok(TestKey::Named(if exact {
        source.to_owned()
    } else {
        source.replace('_', "-")
    }))
}

fn parse_test_key_event(source: &str, line: &Line, pressed: bool) -> Result<TestKeyEvent, Error> {
    let values = split_words(source);
    let Some((key, options)) = values.split_first() else {
        return Err(error(TEST_ERROR, line, "keyboard event requires a key"));
    };
    let mut event = TestKeyEvent {
        key: parse_test_key(key, line)?,
        modified_key: None,
        location: TestKeyLocation::Standard,
        physical: None,
        text: None,
        repeat: false,
    };
    let mut location = false;
    let mut repeat = false;
    for option in options {
        if let Some(value) = option.strip_prefix("location=") {
            if location {
                return Err(error(TEST_ERROR, line, "duplicate keyboard location"));
            }
            location = true;
            event.location = match value {
                "standard" => TestKeyLocation::Standard,
                "left" => TestKeyLocation::Left,
                "right" => TestKeyLocation::Right,
                "numpad" => TestKeyLocation::Numpad,
                _ => {
                    return Err(error(
                        TEST_ERROR,
                        line,
                        "keyboard location must be standard, left, right, or numpad",
                    ));
                }
            };
        } else if let Some(value) = option.strip_prefix("modified=") {
            if event.modified_key.is_some() {
                return Err(error(TEST_ERROR, line, "duplicate modified key"));
            }
            event.modified_key = Some(parse_test_key(value, line)?);
        } else if let Some(value) = option.strip_prefix("physical=") {
            if event.physical.is_some() {
                return Err(error(TEST_ERROR, line, "duplicate physical key"));
            }
            let (exact, ergonomic) = test_key_name_shape(value);
            if !exact && !ergonomic {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "physical keys use lowercase code names like key-a or exact iced variants like IntlBackslash",
                ));
            }
            event.physical = Some(if exact {
                value.to_owned()
            } else {
                value.replace('_', "-")
            });
        } else if let Some(value) = option.strip_prefix("text=") {
            if !pressed {
                return Err(error(TEST_ERROR, line, "key-up does not carry text"));
            }
            if event.text.is_some() {
                return Err(error(TEST_ERROR, line, "duplicate keyboard text"));
            }
            let value = string_literal(value, line)?;
            if value.is_empty() {
                return Err(error(TEST_ERROR, line, "keyboard text must not be empty"));
            }
            event.text = Some(value);
        } else if let Some(value) = option.strip_prefix("repeat=") {
            if !pressed {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "key-up does not carry repeat metadata",
                ));
            }
            if repeat {
                return Err(error(TEST_ERROR, line, "duplicate keyboard repeat flag"));
            }
            repeat = true;
            event.repeat = match value {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(error(
                        TEST_ERROR,
                        line,
                        "keyboard repeat must be true or false",
                    ));
                }
            };
        } else {
            return Err(error(
                TEST_ERROR,
                line,
                "keyboard options use modified=..., location=..., physical=..., text=\"...\", or repeat=true|false",
            ));
        }
    }
    Ok(event)
}

pub(super) fn test_key_name_shape(source: &str) -> (bool, bool) {
    let exact = source
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase())
        && source.bytes().all(|byte| byte.is_ascii_alphanumeric())
        && crate::valid_identifier(source);
    let ergonomic = source
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase())
        && source.split(['-', '_']).all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
        && crate::valid_identifier(&test_keyboard_variant_name(source));
    (exact, ergonomic)
}

fn parse_test_modifiers(values: &[String], line: &Line) -> Result<TestModifiers, Error> {
    let mut modifiers = TestModifiers::default();
    for value in values {
        let slot = match value.as_str() {
            "shift" => &mut modifiers.shift,
            "control" => &mut modifiers.control,
            "alt" => &mut modifiers.alt,
            "logo" => &mut modifiers.logo,
            _ => {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "modifiers must be shift, control, alt, or logo",
                ));
            }
        };
        if *slot {
            return Err(error(
                TEST_ERROR,
                line,
                format!("duplicate modifier `{value}`"),
            ));
        }
        *slot = true;
    }
    Ok(modifiers)
}

fn parse_static_count(source: &str, label: &str, line: &Line) -> Result<u8, Error> {
    source
        .parse::<u8>()
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| error(TEST_ERROR, line, format!("{label} must be in 1..=255")))
}

fn parse_composition(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestComposition, Error> {
    if source == "start" {
        Ok(TestComposition::Start)
    } else if source == "cancel" {
        Ok(TestComposition::Cancel)
    } else if let Some(value) = source.strip_prefix("update ") {
        let values = split_words(value);
        let (value, selection) = match values.as_slice() {
            [value] => (value, None),
            [value, start, end] => (
                value,
                Some((
                    parse_test_expr(strip_wrapping_parens(start), line, scope, targets)?,
                    parse_test_expr(strip_wrapping_parens(end), line, scope, targets)?,
                )),
            ),
            _ => {
                return Err(error(
                    TEST_ERROR,
                    line,
                    "composition update uses `composition update text [selection-start selection-end]`",
                ));
            }
        };
        Ok(TestComposition::Update {
            value: parse_test_expr(strip_wrapping_parens(value), line, scope, targets)?,
            selection,
        })
    } else if let Some(value) = source.strip_prefix("commit ") {
        Ok(TestComposition::Commit(parse_test_expr(
            value.trim(),
            line,
            scope,
            targets,
        )?))
    } else {
        Err(error(
            TEST_ERROR,
            line,
            "composition uses start, update text, commit text, or cancel",
        ))
    }
}

fn parse_touch_step(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestStepKind, Error> {
    let values = split_words(source);
    let [phase, id, x, y] = values.as_slice() else {
        return Err(error(
            TEST_ERROR,
            line,
            "touch uses `touch down|move|up|cancel id x y`",
        ));
    };
    let phase = match phase.as_str() {
        "down" => TestTouchPhase::Down,
        "move" => TestTouchPhase::Move,
        "up" => TestTouchPhase::Up,
        "cancel" => TestTouchPhase::Cancel,
        _ => {
            return Err(error(
                TEST_ERROR,
                line,
                "touch phase must be down, move, up, or cancel",
            ));
        }
    };
    Ok(TestStepKind::Touch {
        phase,
        id: parse_test_expr(strip_wrapping_parens(id), line, scope, targets)?,
        x: parse_test_expr(strip_wrapping_parens(x), line, scope, targets)?,
        y: parse_test_expr(strip_wrapping_parens(y), line, scope, targets)?,
    })
}

fn parse_window_step(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestStepKind, Error> {
    match source {
        "focus" => Ok(TestStepKind::WindowFocus(true)),
        "blur" => Ok(TestStepKind::WindowFocus(false)),
        "close-request" => Ok(TestStepKind::WindowClose),
        "opened" => Ok(TestStepKind::WindowOpened),
        "closed" => Ok(TestStepKind::WindowClosed),
        "redraw" => Ok(TestStepKind::Redraw),
        _ if source.starts_with("move ") => {
            let (x, y) = parse_test_pair(&source[5..], "window move", line, scope, targets)?;
            Ok(TestStepKind::WindowMove(x, y))
        }
        _ if source.starts_with("resize ") => {
            let (width, height) =
                parse_test_pair(&source[7..], "window resize", line, scope, targets)?;
            Ok(TestStepKind::Resize(width, height))
        }
        _ if source.starts_with("rescale ") => Ok(TestStepKind::Rescale(parse_test_expr(
            source[8..].trim(),
            line,
            scope,
            targets,
        )?)),
        _ => Err(error(
            TEST_ERROR,
            line,
            "window uses focus, blur, move x y, resize width height, rescale factor, close-request, opened, closed, or redraw",
        )),
    }
}

fn parse_test_duration(source: &str, line: &Line, action: &str) -> Result<u64, Error> {
    parse_duration(source.trim(), line).map_err(|_| {
        error(
            TEST_ERROR,
            line,
            format!("{action} requires a positive duration such as `16ms` or `1s`"),
        )
    })
}

fn parse_accessibility_action(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestStepKind, Error> {
    let values = split_words(source);
    let [action, target] = values.as_slice() else {
        return Err(error(
            TEST_ERROR,
            line,
            "a11y actions use `a11y activate|focus target`",
        ));
    };
    let action = match action.as_str() {
        "activate" => TestAccessibilityAction::Activate,
        "focus" => TestAccessibilityAction::Focus,
        _ => {
            return Err(error(
                TEST_ERROR,
                line,
                "a11y action must be activate or focus",
            ));
        }
    };
    Ok(TestStepKind::Accessibility {
        action,
        target: parse_test_target_ref(target, line, scope, targets)?,
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
    if let Some(value) = source.strip_prefix("a11y ") {
        return parse_accessibility_expectation(value, line, scope, targets);
    }
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
    if let Some(value) = source.strip_prefix("no tray ") {
        return parse_tray_expectation(value, true, line, scope, targets);
    }
    if let Some(value) = source.strip_prefix("tray ") {
        return parse_tray_expectation(value, false, line, scope, targets);
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

fn parse_accessibility_expectation(
    source: &str,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestExpectation, Error> {
    let values = split_words(source);
    let [target, property, rest @ ..] = values.as_slice() else {
        return Err(error(
            TEST_ERROR,
            line,
            "a11y expectation requires a target, property, and expected value",
        ));
    };
    let target = parse_test_target_ref(target, line, scope, targets)?;
    let expression =
        |source: &str| parse_test_expr(strip_wrapping_parens(source), line, scope, targets);
    let property = match (property.as_str(), rest) {
        ("role", [value]) => TestAccessibilityProperty::Role(expression(value)?),
        ("name", [value]) => TestAccessibilityProperty::Name(expression(value)?),
        ("value", [value]) => TestAccessibilityProperty::Value(expression(value)?),
        ("checked", [value]) => TestAccessibilityProperty::Checked(expression(value)?),
        ("expanded", [value]) => TestAccessibilityProperty::Expanded(expression(value)?),
        ("disabled", [value]) => TestAccessibilityProperty::Disabled(expression(value)?),
        ("focused", [value]) => TestAccessibilityProperty::Focused(expression(value)?),
        ("action", [name]) if name == "true" => {
            return Err(error(
                TEST_ERROR,
                line,
                "a11y action requires an action name",
            ));
        }
        ("action", [name]) => {
            validate_accessibility_action_name(name, line)?;
            TestAccessibilityProperty::Action {
                name: name.to_owned(),
                expected: Expr::Bool(true),
            }
        }
        ("action", [name, expected]) if expected == "true" => {
            validate_accessibility_action_name(name, line)?;
            TestAccessibilityProperty::Action {
                name: name.to_owned(),
                expected: Expr::Bool(true),
            }
        }
        ("action", [name, expected]) => {
            validate_accessibility_action_name(name, line)?;
            TestAccessibilityProperty::Action {
                name: name.to_owned(),
                expected: expression(expected)?,
            }
        }
        _ => {
            return Err(error(
                TEST_ERROR,
                line,
                "a11y properties use role|name|value string, checked|expanded|disabled|focused bool, or action name [bool]",
            ));
        }
    };
    Ok(TestExpectation::Accessibility { target, property })
}

fn validate_accessibility_action_name(source: &str, line: &Line) -> Result<(), Error> {
    if matches!(source, "click" | "focus") {
        Ok(())
    } else {
        Err(error(
            TEST_ERROR,
            line,
            format!("unsupported accessibility action `{source}`; tests support click and focus"),
        ))
    }
}

fn parse_tray_expectation(
    source: &str,
    negated: bool,
    line: &Line,
    scope: &str,
    targets: &[TestTargetDecl],
) -> Result<TestExpectation, Error> {
    let Some((field, value)) = source.split_once(char::is_whitespace) else {
        return Err(error(TEST_ERROR, line, "tray expectations take a value"));
    };
    let field = match field {
        "label" => TrayField::Label,
        "icon" => TrayField::Icon,
        "item" => TrayField::Item,
        "command" => TrayField::Command,
        _ => {
            return Err(error(
                TEST_ERROR,
                line,
                format!("unknown tray expectation `{field}`"),
            )
            .hint("tray expectations are `label`, `icon`, `item`, and `command`"));
        }
    };
    Ok(TestExpectation::Tray {
        field,
        value: parse_test_expr(value.trim(), line, scope, targets)?,
        negated,
    })
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
