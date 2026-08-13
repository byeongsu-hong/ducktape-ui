use super::*;

pub(in crate::parser) fn parse_ui_enum(source: &str, line: &Line) -> Result<UiEnum, Error> {
    let name = identifier(source, line)?;
    if !name.chars().next().is_some_and(char::is_uppercase) {
        return Err(error(
            "E024",
            line,
            "enum names must start with an uppercase letter",
        ));
    }
    let name = line.qualify(&name);
    if line.children.is_empty() {
        return Err(error("E024", line, "enum requires at least one variant"));
    }
    let mut variants = Vec::new();
    for variant in &line.children {
        ensure_leaf(variant)?;
        let (name, payload) = if variant.text.contains('(') {
            let close = matching_paren(&variant.text, variant)?;
            if close + 1 != variant.text.len() {
                return Err(error(
                    "E024",
                    variant,
                    "enum variants use `name` or `name(Type)`",
                ));
            }
            let open = variant
                .text
                .find('(')
                .expect("variant has an opening parenthesis");
            let payload = variant.text[open + 1..close].trim();
            if payload.is_empty() {
                return Err(error("E024", variant, "payload variants require one type"));
            }
            (
                identifier(variant.text[..open].trim(), variant)?,
                Some(parse_type(payload, variant)?),
            )
        } else {
            (identifier(&variant.text, variant)?, None)
        };
        if name.split('_').any(str::is_empty)
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(error("E024", variant, "enum variants use snake_case"));
        }
        variants.push(UiEnumVariant {
            name,
            payload,
            span: Span::line(variant.number),
        });
    }
    Ok(UiEnum {
        name,
        variants,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_extern_struct(
    line: &Line,
    namespace: &str,
) -> Result<ExternStruct, Error> {
    ensure_leaf(line)?;
    let (source_name, fields) = parse_local_signature(&line.text, line)?;
    let name = line.qualify(&source_name);
    let mut parsed_fields = Vec::new();
    if !fields.trim().is_empty() {
        for field in split_top(&fields, ',') {
            let Some((name, ty)) = field.split_once(':') else {
                return Err(error("E020", line, "struct fields use `name:type`"));
            };
            parsed_fields.push((identifier(name.trim(), line)?, parse_type(ty.trim(), line)?));
        }
    }
    Ok(ExternStruct {
        rust_path: format!("{namespace}::{source_name}"),
        name,
        fields: parsed_fields,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_extern_fn(
    source: &str,
    line: &Line,
    namespace: &str,
    kind: ExternKind,
) -> Result<ExternFn, Error> {
    ensure_leaf(line)?;
    let close = matching_paren(source, line)?;
    let source_name = identifier(source[..source.find('(').unwrap_or(0)].trim(), line)?;
    let name = line.qualify(&source_name);
    if matches!(kind, ExternKind::Pure | ExternKind::Sync) && source_name == "bytes" {
        return Err(error(
            "E021",
            line,
            "`pure` or `sync` function name `bytes` conflicts with byte literal syntax",
        ));
    }
    let params_source = &source[source.find('(').unwrap_or(0) + 1..close];
    let mut params = Vec::new();
    let mut borrowed = Vec::new();
    if !params_source.trim().is_empty() {
        for param in split_top(params_source, ',') {
            let Some((name, ty)) = param.split_once(':') else {
                return Err(error("E021", line, "function parameters use `name:type`"));
            };
            let ty = ty.trim();
            let (is_borrowed, ty) = ty
                .strip_prefix('&')
                .map_or((false, ty), |ty| (true, ty.trim_start()));
            if is_borrowed && kind != ExternKind::Component {
                return Err(error(
                    "E021",
                    line,
                    "only extern component parameters may borrow with `&type`",
                ));
            }
            params.push((identifier(name.trim(), line)?, parse_type(ty, line)?));
            borrowed.push(is_borrowed);
        }
    }
    let rest = source[close + 1..].trim();
    let (progress, rest) = if kind == ExternKind::Sip {
        let Some(rest) = rest.strip_prefix("progress=") else {
            return Err(error(
                "E022",
                line,
                "extern sips require `progress=ProgressType -> ReturnType`",
            ));
        };
        let Some((progress, rest)) = split_top_marker(rest, "->") else {
            return Err(error(
                "E022",
                line,
                "extern sips require `progress=ProgressType -> ReturnType`",
            ));
        };
        (Some(parse_type(progress.trim(), line)?), rest)
    } else {
        let Some(rest) = rest.strip_prefix("->") else {
            return Err(error(
                "E022",
                line,
                "extern functions require `-> ReturnType`",
            ));
        };
        (None, rest)
    };
    let (output, error_ty) = match split_top_once(rest.trim(), '!') {
        Some((output, error_ty)) => (
            parse_type(output.trim(), line)?,
            Some(parse_type(error_ty.trim(), line)?),
        ),
        None => (parse_type(rest.trim(), line)?, None),
    };
    if error_ty.is_some()
        && matches!(
            kind,
            ExternKind::Component
                | ExternKind::Shader
                | ExternKind::Recipe
                | ExternKind::Selector
                | ExternKind::EventFilter
                | ExternKind::Pure
                | ExternKind::Sync
                | ExternKind::Subscription
                | ExternKind::Themer
                | ExternKind::Window
                | ExternKind::MarkdownViewer
                | ExternKind::EditorBinding
                | ExternKind::EditorAction
                | ExternKind::EditorHighlighter
                | ExternKind::EditorStyle
                | ExternKind::TextStyle
                | ExternKind::SliderStyle
                | ExternKind::ProgressStyle
                | ExternKind::ButtonStyle
                | ExternKind::CheckboxStyle
                | ExternKind::TogglerStyle
                | ExternKind::RadioStyle
                | ExternKind::ContainerStyle
                | ExternKind::SvgStyle
                | ExternKind::InputStyle
                | ExternKind::ScrollStyle
                | ExternKind::PickListStyle
                | ExternKind::MenuStyle
                | ExternKind::PaneGridStyle
        )
    {
        return Err(error(
            "E023",
            line,
            "extern components, shaders, recipes, event filters, pure/sync functions, subscriptions, themers, window callbacks, markdown viewers, editor bindings/highlighters, and widget styles cannot declare an error type",
        ));
    }
    Ok(ExternFn {
        kind,
        rust_path: format!("{namespace}::{source_name}"),
        name,
        params,
        borrowed,
        progress,
        output,
        error: error_ty,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_subscription(line: &Line) -> Result<Subscription, Error> {
    ensure_leaf(line)?;
    let Some((call, route)) = split_top_marker(&line.text, "->") else {
        return Err(error(
            "E084",
            line,
            "subscription uses `name(args)`, `every duration`, `repeat name() every duration`, `run name(args)`, `recipe name(args)`, `events id using=filter`, `event [raw] [with-id]`, `input-method event`, `keyboard event`, `mouse event`, `touch event`, `window event`, or `system theme` before `-> handler _`",
        ));
    };
    let call = call.trim();
    let (call, condition) = split_top_marker(call, " when ")
        .map_or((call, None), |(call, condition)| {
            (call.trim(), Some(condition.trim()))
        });
    let (call, status) =
        split_top_marker(call, " status=").map_or(Ok((call, None)), |(call, status)| {
            let status = match status.trim() {
                "any" => EventStatus::Any,
                "captured" => EventStatus::Captured,
                "ignored" => EventStatus::Ignored,
                _ => {
                    return Err(error(
                        "E084",
                        line,
                        "subscription status must be any, captured, or ignored",
                    ));
                }
            };
            Ok((call.trim(), Some(status)))
        })?;
    let (call, filter) = split_top_marker(call, " filter=")
        .map_or(Ok((call, None)), |(call, filter)| {
            Ok((call.trim(), Some(identifier(filter.trim(), line)?)))
        })?;
    let (call, context) = split_top_marker(call, " with=")
        .map_or((call, None), |(call, context)| {
            (call.trim(), Some(context.trim()))
        });
    let mut window_id = false;
    let source = if call == "system theme" {
        SubscriptionSource::SystemTheme
    } else if let Some(source) = call.strip_prefix("repeat ") {
        let Some((call, duration)) = split_top_marker(source, " every ") else {
            return Err(error(
                "E084",
                line,
                "repeat uses `repeat name() every duration`",
            ));
        };
        let (function, args) = parse_signature(call.trim(), line)?;
        if !args.trim().is_empty() {
            return Err(error(
                "E084",
                line,
                "repeated async functions cannot take arguments",
            ));
        }
        SubscriptionSource::Repeat {
            function,
            milliseconds: parse_duration(duration.trim(), line)?,
        }
    } else if let Some(duration) = call.strip_prefix("every ") {
        SubscriptionSource::Every {
            milliseconds: parse_duration(duration.trim(), line)?,
        }
    } else if let Some(call) = call.strip_prefix("run ") {
        let (function, args) = parse_signature(call.trim(), line)?;
        SubscriptionSource::Run {
            function,
            args: parse_expr_list(&args, line)?,
        }
    } else if let Some(call) = call.strip_prefix("recipe ") {
        let (function, args) = parse_signature(call.trim(), line)?;
        SubscriptionSource::Recipe {
            function,
            args: parse_expr_list(&args, line)?,
        }
    } else if let Some(source) = call.strip_prefix("events ") {
        let Some((id, filter)) = split_top_marker(source, " using=") else {
            return Err(error(
                "E084",
                line,
                "raw events use `events identity using=event_filter`",
            ));
        };
        SubscriptionSource::Events {
            id: parse_expr(id.trim(), line)?,
            filter: identifier(filter.trim(), line)?,
        }
    } else if matches!(
        call,
        "event" | "event with-id" | "event raw" | "event raw with-id"
    ) {
        window_id = call.ends_with("with-id");
        SubscriptionSource::Event {
            raw: call.starts_with("event raw"),
        }
    } else if call.starts_with("event ") {
        return Err(error(
            "E084",
            line,
            "generic event source uses `event [raw] [with-id]`",
        ));
    } else if let Some(event) = call.strip_prefix("input-method ") {
        SubscriptionSource::InputMethod(match event.trim() {
            "opened" => InputMethodEvent::Opened,
            "preedit" => InputMethodEvent::Preedit,
            "commit" => InputMethodEvent::Commit,
            "closed" => InputMethodEvent::Closed,
            _ => {
                return Err(error(
                    "E084",
                    line,
                    "input-method event must be opened, preedit, commit, or closed",
                ));
            }
        })
    } else if let Some(event) = call.strip_prefix("window ") {
        let event = event.trim();
        let event = event.strip_suffix(" with-id").map_or(event, |event| {
            window_id = true;
            event.trim()
        });
        SubscriptionSource::Window(match event {
            "frame" => WindowEvent::Frame,
            "opened" => WindowEvent::Opened,
            "closed" => WindowEvent::Closed,
            "moved" => WindowEvent::Moved,
            "resized" => WindowEvent::Resized,
            "rescaled" => WindowEvent::Rescaled,
            "close-request" => WindowEvent::CloseRequested,
            "focused" => WindowEvent::Focused,
            "unfocused" => WindowEvent::Unfocused,
            "file-hovered" => WindowEvent::FileHovered,
            "file-dropped" => WindowEvent::FileDropped,
            "files-hovered-left" => WindowEvent::FilesHoveredLeft,
            _ => return Err(error("E084", line, "unknown window event")),
        })
    } else if let Some(event) = call.strip_prefix("keyboard ") {
        SubscriptionSource::Keyboard(parse_keyboard_event(event, line)?)
    } else if let Some(event) = call.strip_prefix("mouse ") {
        SubscriptionSource::Mouse(match event.trim() {
            "entered" => MouseEvent::Entered,
            "left" => MouseEvent::Left,
            "moved" => MouseEvent::Moved,
            "pressed" => MouseEvent::Pressed,
            "released" => MouseEvent::Released,
            "wheel" => MouseEvent::Wheel,
            _ => {
                return Err(error(
                    "E084",
                    line,
                    "mouse event must be entered, left, moved, pressed, released, or wheel",
                ));
            }
        })
    } else if let Some(event) = call.strip_prefix("touch ") {
        SubscriptionSource::Touch(match event.trim() {
            "pressed" => TouchEvent::Pressed,
            "moved" => TouchEvent::Moved,
            "lifted" => TouchEvent::Lifted,
            "lost" => TouchEvent::Lost,
            _ => {
                return Err(error(
                    "E084",
                    line,
                    "touch event must be pressed, moved, lifted, or lost",
                ));
            }
        })
    } else {
        let (function, args) = parse_signature(call, line)?;
        SubscriptionSource::Extern {
            function,
            args: parse_expr_list(&args, line)?,
        }
    };
    if window_id && matches!(&source, SubscriptionSource::Window(WindowEvent::Frame)) {
        return Err(error(
            "E084",
            line,
            "window frame does not expose a window ID",
        ));
    }
    let supports_status = matches!(
        &source,
        SubscriptionSource::Event { .. }
            | SubscriptionSource::InputMethod(_)
            | SubscriptionSource::Keyboard(_)
            | SubscriptionSource::Mouse(_)
            | SubscriptionSource::Touch(_)
            | SubscriptionSource::Window(
                WindowEvent::Opened
                    | WindowEvent::Closed
                    | WindowEvent::Moved
                    | WindowEvent::Resized
                    | WindowEvent::Rescaled
                    | WindowEvent::CloseRequested
                    | WindowEvent::Focused
                    | WindowEvent::Unfocused
                    | WindowEvent::FileHovered
                    | WindowEvent::FileDropped
                    | WindowEvent::FilesHoveredLeft
            )
    );
    if status.is_some() && !supports_status {
        return Err(error(
            "E084",
            line,
            "status filtering is only available on non-frame runtime events",
        ));
    }
    Ok(Subscription {
        source,
        window_id,
        context: context
            .map(|context| parse_expr(context, line))
            .transpose()?,
        filter,
        condition: condition
            .map(|condition| parse_expr(condition, line))
            .transpose()?,
        status,
        route: parse_route(route.trim(), line)?,
        span: Span::line(line.number),
    })
}

fn parse_keyboard_event(event: &str, line: &Line) -> Result<KeyboardEvent, Error> {
    let mut words = event.split_whitespace();
    let kind = words.next().unwrap_or_default();
    let key = match words.next() {
        None => None,
        Some(option) => {
            let Some(name) = option.strip_prefix("key=") else {
                return Err(error(
                    "E084",
                    line,
                    "keyboard press/release takes only a `key=` option",
                ));
            };
            Some(named_key_variant(name, line)?)
        }
    };
    if words.next().is_some() {
        return Err(error(
            "E084",
            line,
            "keyboard press/release takes only a `key=` option",
        ));
    }
    match (kind, key) {
        ("press", key) => Ok(KeyboardEvent::Press { key }),
        ("release", key) => Ok(KeyboardEvent::Release { key }),
        ("modifiers", None) => Ok(KeyboardEvent::Modifiers),
        ("modifiers", Some(_)) => Err(error(
            "E084",
            line,
            "keyboard modifiers does not take a key filter",
        )),
        _ => Err(error(
            "E084",
            line,
            "keyboard event must be press, release, or modifiers",
        )),
    }
}

fn named_key_variant(name: &str, line: &Line) -> Result<String, Error> {
    let (exact, ergonomic) = testing::test_key_name_shape(name);
    if !exact && !ergonomic {
        return Err(error(
            "E084",
            line,
            "key= names a key with a lowercase name like escape or arrow-left, or an exact iced variant like TVInputHDMI1",
        ));
    }
    let canonical = if exact {
        name.to_owned()
    } else {
        name.replace('_', "-")
    };
    Ok(crate::semantic::test_keyboard_variant_name(&canonical))
}

pub(in crate::parser) fn parse_duration(source: &str, line: &Line) -> Result<u64, Error> {
    let (number, multiplier) = source
        .strip_suffix("ms")
        .map(|number| (number, 1))
        .or_else(|| source.strip_suffix('s').map(|number| (number, 1_000)))
        .ok_or_else(|| {
            error(
                "E084",
                line,
                "duration must use `ms` or `s`, like `500ms` or `2s`",
            )
        })?;
    let value = number
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(multiplier))
        .filter(|value| *value > 0)
        .ok_or_else(|| error("E084", line, "duration must be a positive whole number"))?;
    Ok(value)
}

pub(in crate::parser) fn parse_state(line: &Line) -> Result<State, Error> {
    let Some((left, right)) = split_top_once(&line.text, '=') else {
        return Err(error(
            "E030",
            line,
            "state entries use `name[:type] = value`",
        ));
    };
    let (name, declared) = match left.split_once(':') {
        Some((name, ty)) => (
            identifier(name.trim(), line)?,
            Some(parse_type(ty.trim(), line)?),
        ),
        None => (identifier(left.trim(), line)?, None),
    };
    let initial = parse_expr(right.trim(), line)?;
    let inferred = literal_type(&initial);
    let ty = declared.or(inferred).ok_or_else(|| {
        error("E031", line, "state type cannot be inferred")
            .hint("write an explicit type, for example `items:[Item] = []`")
    })?;
    let animation = if matches!(ty, Type::Animation(_)) {
        Some(parse_animation_options(&line.children)?)
    } else {
        ensure_leaf(line)?;
        None
    };
    Ok(State {
        name,
        ty,
        initial,
        animation,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_animation_options(
    lines: &[Line],
) -> Result<AnimationOptions, Error> {
    let mut options = AnimationOptions::default();
    let mut seen = BTreeMap::new();
    for line in lines {
        ensure_leaf(line)?;
        let Some((name, value)) = line.text.split_once(char::is_whitespace) else {
            return Err(error("E030", line, "animation settings use `name value`"));
        };
        if seen.insert(name, line.number).is_some() {
            return Err(error(
                "E030",
                line,
                format!("duplicate animation setting `{name}`"),
            ));
        }
        let value = value.trim();
        match name {
            "easing" if !value.is_empty() && !value.contains(char::is_whitespace) => {
                options.easing = Some(value.into());
            }
            "duration" => {
                options.duration = Some(match value {
                    "very-quick" => AnimationDuration::VeryQuick,
                    "quick" => AnimationDuration::Quick,
                    "slow" => AnimationDuration::Slow,
                    "very-slow" => AnimationDuration::VerySlow,
                    value => {
                        AnimationDuration::Milliseconds(parse_animation_duration(value, line)?)
                    }
                });
            }
            "delay" => options.delay_ms = Some(parse_animation_duration(value, line)?),
            "repeat" if value == "forever" => options.repeat_forever = true,
            "repeat" => {
                options.repeat = Some(value.parse::<u32>().map_err(|_| {
                    error(
                        "E030",
                        line,
                        "animation repeat expects a whole number or `forever`",
                    )
                })?);
            }
            "auto-reverse" => options.auto_reverse = config_bool(value, line)?,
            "from" => {
                options.from = Some(match value {
                    "true" => AnimationStart::Bool(true),
                    "false" => AnimationStart::Bool(false),
                    value => AnimationStart::Number(value.parse::<f64>().map_err(|_| {
                        error(
                            "E030",
                            line,
                            "animation `from` expects `true`, `false`, or a number",
                        )
                    })?),
                });
            }
            "easing" => {
                return Err(error("E030", line, "animation easing expects one name"));
            }
            _ => {
                return Err(error(
                    "E030",
                    line,
                    format!("unknown animation setting `{name}`"),
                ));
            }
        }
    }
    Ok(options)
}

pub(in crate::parser) fn parse_animation_duration(source: &str, line: &Line) -> Result<u64, Error> {
    let (number, multiplier) = source
        .strip_suffix("ms")
        .map(|number| (number, 1))
        .or_else(|| source.strip_suffix('s').map(|number| (number, 1_000)))
        .ok_or_else(|| error("E030", line, "animation time uses `ms` or `s`"))?;
    number
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(multiplier))
        .ok_or_else(|| error("E030", line, "animation time must be a whole number"))
}

pub(in crate::parser) fn parse_component(header: &str, line: &Line) -> Result<Component, Error> {
    let close = matching_paren(header, line)?;
    let (name, params_source) = parse_component_signature(&header[..=close], line)?;
    let output = match header[close + 1..].trim() {
        "" => Type::Unit,
        rest => {
            let output = rest.strip_prefix("->").ok_or_else(|| {
                error(
                    "E043",
                    line,
                    "component output uses `component Name(...) -> Type`",
                )
            })?;
            parse_type(output.trim(), line)?
        }
    };
    line.record_symbol(SymbolKind::Component, &name, true, header);
    let symbol_start = line.symbols.borrow().len();
    let mut params = Vec::new();
    if !params_source.trim().is_empty() {
        for param in split_top(&params_source, ',') {
            let Some((name, declaration)) = param.split_once(':') else {
                return Err(error(
                    "E043",
                    line,
                    "component parameters require `[bind] name:type`",
                ));
            };
            let name = name.trim();
            let (bind, name) = name
                .strip_prefix("bind ")
                .map_or((false, name), |name| (true, name.trim_start()));
            let (ty, default) = split_top_once(declaration.trim(), '=')
                .map_or((declaration.trim(), None), |(ty, default)| {
                    (ty.trim(), Some(default.trim()))
                });
            if default == Some("") {
                return Err(error("E043", line, "component prop default needs a value"));
            }
            params.push(ComponentParam {
                name: identifier(name, line)?,
                ty: parse_type(ty, line)?,
                bind,
                default: default.map(|value| parse_expr(value, line)).transpose()?,
            });
        }
    }
    let mut states = Vec::new();
    let mut events = Vec::new();
    let mut handlers = Vec::new();
    let mut roots = Vec::new();
    let mut state_block = false;
    let mut emits_block = false;
    let mut lifetime = None;
    for child in &line.children {
        if child.text == "state" {
            if state_block {
                return Err(error("E040", child, "component has duplicate state blocks"));
            }
            if child.children.is_empty() {
                return Err(error("E040", child, "component state cannot be empty"));
            }
            state_block = true;
            states.extend(
                child
                    .children
                    .iter()
                    .map(parse_state)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else if child.text == "emits" {
            if emits_block {
                return Err(error("E040", child, "component has duplicate emits blocks"));
            }
            if child.children.is_empty() {
                return Err(error("E040", child, "component emits cannot be empty"));
            }
            emits_block = true;
            for event in &child.children {
                ensure_leaf(event)?;
                let (name, payloads) = if event.text.contains('(') {
                    let (name, payloads) = parse_local_signature(&event.text, event)?;
                    let payloads = if payloads.trim().is_empty() {
                        Vec::new()
                    } else {
                        split_top(&payloads, ',')
                            .into_iter()
                            .map(|payload| parse_type(payload.trim(), event))
                            .collect::<Result<Vec<_>, _>>()?
                    };
                    (name, payloads)
                } else {
                    (identifier(&event.text, event)?, Vec::new())
                };
                events.push(ComponentEvent {
                    name,
                    payloads,
                    span: Span::line(event.number),
                });
            }
        } else if let Some(value) = child.text.strip_prefix("lifetime ") {
            ensure_leaf(child)?;
            if lifetime.is_some() {
                return Err(error(
                    "E040",
                    child,
                    "component has duplicate lifetime declarations",
                ));
            }
            lifetime = Some(match value {
                "retained" => ComponentLifetime::Retained,
                "mounted" => ComponentLifetime::Mounted,
                _ => {
                    return Err(error(
                        "E040",
                        child,
                        "component lifetime must be `retained` or `mounted`",
                    ));
                }
            });
        } else if child.text == "lifetime" {
            return Err(error(
                "E040",
                child,
                "component lifetime must be `retained` or `mounted`",
            ));
        } else if let Some(header) = child.text.strip_prefix("on ") {
            handlers.push(parse_handler(header, child)?);
        } else {
            roots.push(child);
        }
    }
    let [root] = roots.as_slice() else {
        return Err(error(
            "E040",
            line,
            "component must have exactly one root node",
        ));
    };
    let root = parse_view(root)?;
    let local_handler = |name: &str| handlers.iter().any(|handler| handler.name == name);
    let mut symbols = line.symbols.borrow_mut();
    let retained = symbols
        .drain(symbol_start..)
        .filter(|symbol| {
            symbol.kind != SymbolKind::Handler
                || !symbol.definition && !local_handler(&symbol.name) && symbol.name != "emit"
        })
        .collect::<Vec<_>>();
    symbols.extend(retained);
    drop(symbols);
    Ok(Component {
        name,
        params,
        output,
        events,
        lifetime: lifetime.unwrap_or_default(),
        states,
        handlers,
        root,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_handler(header: &str, line: &Line) -> Result<Handler, Error> {
    let header = header.trim();
    let (name, params) = if header.contains('(') {
        let (name, params) = parse_local_signature(header, line)?;
        let params = if params.trim().is_empty() {
            Vec::new()
        } else {
            split_top(&params, ',')
                .into_iter()
                .map(|value| {
                    Ok(HandlerParam {
                        name: identifier(value.trim(), line)?,
                        ty: Type::Unknown,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?
        };
        (name, params)
    } else {
        (identifier(header, line)?, Vec::new())
    };
    line.record_symbol(SymbolKind::Handler, &name, true, header);
    let statements = line
        .children
        .iter()
        .map(parse_statement)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Handler {
        name,
        params,
        statements,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_route(source: &str, line: &Line) -> Result<Route, Error> {
    let source = source.trim();
    let line = line.origin_for(source);
    if source == "emit" || source.starts_with("emit ") {
        return Err(error(
            "E052",
            line,
            "emit uses function syntax such as `emit(event, value)`",
        ));
    }
    if let Some(open) = source.find('(') {
        let close = matching_paren(source, line)?;
        if !source[close + 1..].trim().is_empty() {
            return Err(error("E052", line, "unexpected text after route"));
        }
        let handler = identifier(source[..open].trim(), line)?;
        line.record_symbol(SymbolKind::Handler, &handler, false, source);
        let args_source = &source[open + 1..close];
        if handler == "emit" && args_source.trim().is_empty() {
            return Err(error("E052", line, "emit requires an output or event"));
        }
        let args = if args_source.trim().is_empty() {
            Vec::new()
        } else {
            split_top(args_source, ',')
                .into_iter()
                .map(|part| {
                    if part.trim() == "_" {
                        Ok(RouteArg::Payload)
                    } else {
                        Ok(RouteArg::Expr(parse_expr(part.trim(), line)?))
                    }
                })
                .collect::<Result<_, Error>>()?
        };
        return Ok(Route {
            handler,
            args,
            span: Span::line(line.number),
        });
    }
    let mut words = source.split_whitespace();
    let handler = identifier(
        words
            .next()
            .ok_or_else(|| error("E052", line, "empty route"))?,
        line,
    )?;
    line.record_symbol(SymbolKind::Handler, &handler, false, source);
    let args = words
        .map(|word| {
            if word == "_" {
                Ok(RouteArg::Payload)
            } else {
                Ok(RouteArg::Expr(parse_expr(word, line)?))
            }
        })
        .collect::<Result<_, Error>>()?;
    Ok(Route {
        handler,
        args,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_id(source: &str, line: &Line) -> Result<Id, Error> {
    let source = source.strip_prefix('#').unwrap_or(source);
    if let Some(open) = source.find('(') {
        let close = matching_paren(source, line)?;
        if close + 1 != source.len() {
            return Err(error("E068", line, "unexpected text after dynamic id"));
        }
        Ok(Id {
            name: kebab_identifier(&source[..open], line)?,
            key: Some(parse_expr(&source[open + 1..close], line)?),
        })
    } else {
        Ok(Id {
            name: kebab_identifier(source, line)?,
            key: None,
        })
    }
}
