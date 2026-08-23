use super::*;

pub(in crate::parser) fn parse_media(
    kind: &str,
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error(
            "E085",
            line,
            "media uses typed properties instead of `@` utilities",
        ));
    }
    let source = parts
        .get(1)
        .ok_or_else(|| error("E085", line, format!("{kind} requires a source expression")))?;
    let media_kind = match kind {
        "image" => MediaKind::Image,
        "svg" => MediaKind::Svg,
        "viewer" => MediaKind::Viewer,
        _ => unreachable!(),
    };
    let mut id = None;
    let mut options = MediaOptions::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E085", kind)?;
        } else if parse_accessibility_option(part, &mut options.accessibility, line)? {
        } else if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("fit=") {
            options.fit = Some(match value {
                "contain" | "cover" | "fill" | "none" | "scale-down" => Expr::Call {
                    name: format!("fit.{}", value.replace('-', "_")),
                    args: Vec::new(),
                },
                value => parse_expr(strip_wrapping_parens(value), line)?,
            });
        } else if let Some(value) = part.strip_prefix("rotate=") {
            if media_kind == MediaKind::Viewer {
                return Err(error("E085", line, "rotate is not available on viewer"));
            }
            options.rotation = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("opacity=") {
            if media_kind == MediaKind::Viewer {
                return Err(error("E085", line, "opacity is not available on viewer"));
            }
            options.opacity = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if *part == "memory" {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "memory is only available on svg"));
            }
            options.svg_memory = true;
        } else if let Some(value) = part.strip_prefix("color=") {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "color is only available on svg"));
            }
            options.svg_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("hover=") {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "hover is only available on svg"));
            }
            options.svg_hover_color = Some((value != "none").then(|| value.to_owned()));
        } else if let Some(value) = part.strip_prefix("style=") {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "style is only available on svg"));
            }
            options.svg_style = Some(parse_extern_call(
                value,
                line,
                "E085",
                "svg style must be a declared style call",
            )?);
        } else if let Some(value) = part.strip_prefix("filter=") {
            if media_kind == MediaKind::Svg {
                return Err(error(
                    "E085",
                    line,
                    "filter is only available on image and viewer",
                ));
            }
            options.filter = Some(match value {
                "linear" => ImageFilter::Linear,
                "nearest" => ImageFilter::Nearest,
                _ => return Err(error("E085", line, "filter must be linear or nearest")),
            });
        } else if let Some(value) = part.strip_prefix("scale=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "scale is only available on image"));
            }
            options.scale = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("expand=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "expand is only available on image"));
            }
            options.expand = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "radius is only available on image"));
            }
            options.radius.all = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some((field, value)) = [
            ("r-tl=", &mut options.radius.top_left),
            ("r-tr=", &mut options.radius.top_right),
            ("r-br=", &mut options.radius.bottom_right),
            ("r-bl=", &mut options.radius.bottom_left),
        ]
        .into_iter()
        .find_map(|(prefix, field)| part.strip_prefix(prefix).map(|value| (field, value)))
        {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "radius is only available on image"));
            }
            *field = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("crop=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "crop is only available on image"));
            }
            options.crop = Some(
                parse_expr_list(strip_wrapping_parens(value), line)?
                    .try_into()
                    .map_err(|_| error("E085", line, "crop requires x, y, width, and height"))?,
            );
        } else if let Some((property, field, value)) = [
            ("p=", &mut options.padding),
            ("min-scale=", &mut options.min_scale),
            ("max-scale=", &mut options.max_scale),
            ("scale-step=", &mut options.scale_step),
        ]
        .into_iter()
        .find_map(|(property, field)| {
            part.strip_prefix(property)
                .map(|value| (property, field, value))
        }) {
            if media_kind != MediaKind::Viewer {
                return Err(error(
                    "E085",
                    line,
                    format!(
                        "{} is only available on viewer",
                        property.trim_end_matches('=')
                    ),
                ));
            }
            *field = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E085",
                line,
                format!("unknown {kind} property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Media {
        kind: media_kind,
        id,
        source: parse_expr(source, line)?,
        options,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_length(source: &str, line: &Line) -> Result<LengthValue, Error> {
    Ok(match source {
        "fill" => LengthValue::Fill,
        "shrink" => LengthValue::Shrink,
        source => {
            if let Some(value) = source
                .strip_prefix("fill(")
                .and_then(|value| value.strip_suffix(')'))
            {
                LengthValue::FillPortion(value.parse().map_err(|_| {
                    error(
                        "E074",
                        line,
                        "fill portion must be an integer from 0 to 65535",
                    )
                })?)
            } else {
                LengthValue::Fixed(parse_expr(strip_wrapping_parens(source), line)?)
            }
        }
    })
}

pub(in crate::parser) fn parse_tooltip(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E086",
            line,
            "tooltip content owns its styling; the wrapper does not accept `@`",
        ));
    }
    if line.children.len() != 2 {
        return Err(error(
            "E086",
            line,
            "tooltip requires exactly two children: content, then tip",
        ));
    }
    let mut id = None;
    let mut options = TooltipOptions {
        position: TooltipPosition::Top,
        gap: Expr::F64(0.0),
        padding: Expr::F64(5.0),
        delay_ms: Expr::I64(0),
        snap: Expr::Bool(true),
        style: None,
        custom_style: None,
        background: None,
        text_color: None,
        border_color: None,
        border_width: None,
        radius: RadiusOptions::default(),
        shadow_color: None,
        shadow_x: None,
        shadow_y: None,
        shadow_blur: None,
        pixel_snap: None,
    };
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E086", "tooltip")?;
        } else if let Some(value) = part.strip_prefix("position=") {
            options.position = match value {
                "top" => TooltipPosition::Top,
                "bottom" => TooltipPosition::Bottom,
                "left" => TooltipPosition::Left,
                "right" => TooltipPosition::Right,
                "cursor" => TooltipPosition::FollowCursor,
                _ => {
                    return Err(error(
                        "E086",
                        line,
                        "tooltip position must be top, bottom, left, right, or cursor",
                    ));
                }
            };
        } else if let Some(value) = part.strip_prefix("gap=") {
            options.gap = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("p=") {
            options.padding = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("delay=") {
            options.delay_ms = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("snap=") {
            options.snap = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = None;
            options.style = match value {
                "transparent" => Some(TooltipStyle::Transparent),
                "rounded" => Some(TooltipStyle::Rounded),
                "bordered" => Some(TooltipStyle::Bordered),
                "dark" => Some(TooltipStyle::Dark),
                "primary" => Some(TooltipStyle::Primary),
                "secondary" => Some(TooltipStyle::Secondary),
                "success" => Some(TooltipStyle::Success),
                "warning" => Some(TooltipStyle::Warning),
                "danger" => Some(TooltipStyle::Danger),
                _ => {
                    options.custom_style = Some(parse_extern_call(
                        value,
                        line,
                        "E086",
                        "tooltip style must be a preset or declared box style call",
                    )?);
                    None
                }
            };
        } else if let Some(value) = part.strip_prefix("bg=") {
            options.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("text=") {
            options.text_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border=") {
            options.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-w=") {
            options.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if parse_radius_option(part, &mut options.radius, "", line)? {
        } else if let Some(value) = part.strip_prefix("shadow=") {
            options.shadow_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("shadow-x=") {
            options.shadow_x = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shadow-y=") {
            options.shadow_y = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shadow-blur=") {
            options.shadow_blur = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("px-snap=") {
            options.pixel_snap = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E086",
                line,
                format!("unknown tooltip property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Tooltip {
        id,
        options,
        content: Box::new(parse_view(&line.children[0])?),
        tip: Box::new(parse_view(&line.children[1])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_mouse_area(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E087", line, "mouse does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E087", line, "mouse requires exactly one child"));
    }
    let mut id = None;
    let mut options = MouseAreaOptions::default();
    for part in &parts[1..] {
        let route = |value: &str| parse_route(value, line);
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E087", "mouse")?;
        } else if let Some(value) = part.strip_prefix("press=") {
            options.press = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("press-at=") {
            options.press_at = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("release=") {
            options.release = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("double=") {
            options.double_click = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("right-press=") {
            options.right_press = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("right-release=") {
            options.right_release = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("middle-press=") {
            options.middle_press = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("middle-release=") {
            options.middle_release = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("enter=") {
            options.enter = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("move=") {
            options.move_route = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("scroll=") {
            options.scroll = Some(parse_payload_route(value, line, 3)?);
        } else if let Some(value) = part.strip_prefix("exit=") {
            options.exit = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("cursor=") {
            if options.interaction.is_some() || options.interaction_expr.is_some() {
                return Err(error("E087", line, "duplicate mouse cursor property"));
            }
            if value.starts_with('(') {
                options.interaction_expr = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else {
                options.interaction = Some(parse_mouse_interaction(value, line)?);
            }
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown mouse property `{part}`"),
            ));
        }
    }
    if options.press.is_none()
        && options.press_at.is_none()
        && options.release.is_none()
        && options.double_click.is_none()
        && options.right_press.is_none()
        && options.right_release.is_none()
        && options.middle_press.is_none()
        && options.middle_release.is_none()
        && options.enter.is_none()
        && options.move_route.is_none()
        && options.scroll.is_none()
        && options.exit.is_none()
        && options.interaction.is_none()
        && options.interaction_expr.is_none()
    {
        return Err(error(
            "E087",
            line,
            "mouse needs an event route or cursor property",
        ));
    }
    Ok(ViewNode::MouseArea {
        id,
        options,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_resize_handle(
    parts: &[&str],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E087",
            line,
            "resize-handle does not accept `@` utilities",
        ));
    }
    if line.children.len() != 1 {
        return Err(error(
            "E087",
            line,
            "resize-handle requires exactly one child",
        ));
    }
    let mut id = None;
    let mut options = ResizeHandleOptions::default();
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E087", "resize-handle")?;
        } else if let Some(value) = part.strip_prefix("drag=") {
            options.drag = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("press=") {
            options.press = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("release=") {
            options.release = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("cursor=") {
            options.interaction = Some(parse_mouse_interaction(value, line)?);
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown resize-handle property `{part}`"),
            ));
        }
    }
    if options.drag.is_none() {
        return Err(error(
            "E087",
            line,
            "resize-handle requires `drag=handler` to report the drag delta",
        ));
    }
    Ok(ViewNode::ResizeHandle {
        id,
        options,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_mouse_interaction(
    source: &str,
    line: &Line,
) -> Result<MouseInteraction, Error> {
    Ok(match source {
        "none" => MouseInteraction::None,
        "hidden" => MouseInteraction::Hidden,
        "idle" => MouseInteraction::Idle,
        "context-menu" => MouseInteraction::ContextMenu,
        "help" => MouseInteraction::Help,
        "pointer" => MouseInteraction::Pointer,
        "progress" => MouseInteraction::Progress,
        "wait" => MouseInteraction::Wait,
        "cell" => MouseInteraction::Cell,
        "crosshair" => MouseInteraction::Crosshair,
        "text" => MouseInteraction::Text,
        "alias" => MouseInteraction::Alias,
        "copy" => MouseInteraction::Copy,
        "move" => MouseInteraction::Move,
        "no-drop" => MouseInteraction::NoDrop,
        "not-allowed" => MouseInteraction::NotAllowed,
        "grab" => MouseInteraction::Grab,
        "grabbing" => MouseInteraction::Grabbing,
        "resize-horizontal" => MouseInteraction::ResizingHorizontally,
        "resize-vertical" => MouseInteraction::ResizingVertically,
        "resize-diagonal-up" => MouseInteraction::ResizingDiagonallyUp,
        "resize-diagonal-down" => MouseInteraction::ResizingDiagonallyDown,
        "resize-column" => MouseInteraction::ResizingColumn,
        "resize-row" => MouseInteraction::ResizingRow,
        "all-scroll" => MouseInteraction::AllScroll,
        "zoom-in" => MouseInteraction::ZoomIn,
        "zoom-out" => MouseInteraction::ZoomOut,
        _ => return Err(error("E087", line, format!("unknown cursor `{source}`"))),
    })
}
