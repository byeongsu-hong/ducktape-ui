use super::*;

pub(in crate::parser) fn parse_flexbox_options(
    parts: &[String],
    line: &Line,
) -> Result<LayoutOptions, Error> {
    let mut flexbox = FlexboxOptions::default();
    let mut native = Vec::new();
    let mut direction_seen = false;
    let mut wrap_seen = false;
    let mut align_items_seen = false;
    let mut align_content_seen = false;
    let mut max_width = None;
    let mut max_height = None;

    for part in parts {
        if let Some(value) = part.strip_prefix("dir=") {
            if direction_seen {
                return Err(error("E074", line, "flex direction may only be set once"));
            }
            direction_seen = true;
            flexbox.direction = parse_flex_direction(value, line)?;
        } else if let Some(value) = part.strip_prefix("flow=") {
            if direction_seen || wrap_seen {
                return Err(error(
                    "E074",
                    line,
                    "flow cannot be combined with dir or wrap",
                ));
            }
            let values = split_top(value, ',');
            if values.len() != 2 {
                return Err(error("E074", line, "flow expects dir,wrap"));
            }
            direction_seen = true;
            wrap_seen = true;
            flexbox.direction = parse_flex_direction(values[0], line)?;
            flexbox.wrap = parse_flex_wrap(values[1], line)?;
        } else if let Some(value) = part.strip_prefix("wrap=") {
            if wrap_seen {
                return Err(error("E074", line, "flex wrap may only be set once"));
            }
            wrap_seen = true;
            flexbox.wrap = parse_flex_wrap(value, line)?;
        } else if let Some(value) = part.strip_prefix("justify=") {
            if flexbox.justify_content.is_some() {
                return Err(error("E074", line, "justify may only be set once"));
            }
            flexbox.justify_content = Some(parse_flex_content_alignment(value, line)?);
        } else if let Some(value) = part.strip_prefix("items=") {
            if align_items_seen {
                return Err(error("E074", line, "items may only be set once"));
            }
            align_items_seen = true;
            flexbox.align_items = Some(parse_flex_item_alignment(value, line)?);
        } else if let Some(value) = part.strip_prefix("content=") {
            if align_content_seen {
                return Err(error("E074", line, "content may only be set once"));
            }
            align_content_seen = true;
            flexbox.align_content = Some(parse_flex_content_alignment(value, line)?);
        } else if let Some(value) = part.strip_prefix("gap=") {
            native.push(format!("gap={value}"));
        } else if let Some(value) = part.strip_prefix("gap-y=") {
            if flexbox.row_gap.is_some() {
                return Err(error("E074", line, "gap-y may only be set once"));
            }
            flexbox.row_gap = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("gap-x=") {
            if flexbox.column_gap.is_some() {
                return Err(error("E074", line, "gap-x may only be set once"));
            }
            flexbox.column_gap = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max-w=") {
            max_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max-h=") {
            max_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "wrap"
            || part.starts_with("align=")
            || part.starts_with("wrap-gap=")
            || part.starts_with("wrap-align=")
        {
            return Err(error(
                "E074",
                line,
                format!("unknown flex property `{part}`"),
            ));
        } else {
            native.push(part.clone());
        }
    }

    let native_kind = match flexbox.direction {
        FlexDirectionValue::Row | FlexDirectionValue::RowReverse => "row",
        FlexDirectionValue::Column | FlexDirectionValue::ColumnReverse => "col",
    };
    let mut options = parse_layout_options(native_kind, &native, line)?;
    options.max_width = max_width;
    options.max_height = max_height;
    options.flexbox = Some(flexbox);
    Ok(options)
}

pub(in crate::parser) fn parse_layout_options(
    kind: &str,
    parts: &[String],
    line: &Line,
) -> Result<LayoutOptions, Error> {
    let mut options = LayoutOptions::default();
    let is_flex = matches!(kind, "row" | "col");
    if kind == "scroll" {
        options.scroll = Some(ScrollOptions::default());
    }
    for part in parts {
        if kind == "hover"
            && let Some(value) = part.strip_prefix("tint=")
        {
            if options.hover_tint.is_some() {
                return Err(error("E062", line, "duplicate hover tint"));
            }
            options.hover_tint = Some(value.to_owned());
        } else if kind == "hover"
            && let Some(value) = part.strip_prefix("r=")
        {
            let Expr::F64(radius) = parse_expr(value, line)? else {
                return Err(error("E062", line, "hover r= takes a number literal"));
            };
            options.hover_radius = Some(radius);
        } else if let Some(value) = part.strip_prefix("cols=") {
            if kind != "grid" || options.columns.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.columns = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("clip=") {
            if !(is_flex || kind == "stack") || options.clip.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.clip = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if (is_flex || kind == "stack")
            && let Some(value) = part.strip_prefix("w=")
        {
            if options.width.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.width = Some(parse_length(value, line)?);
        } else if (is_flex || kind == "stack")
            && let Some(value) = part.strip_prefix("h=")
        {
            if options.height.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.height = Some(parse_length(value, line)?);
        } else if kind == "grid"
            && let Some(value) = part.strip_prefix("w=")
        {
            if options.width.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.width = Some(LengthValue::Fixed(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if kind == "grid"
            && let Some(value) = part.strip_prefix("h=")
        {
            if options.grid_height.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.grid_height = Some(parse_grid_sizing(value, line)?);
        } else if kind == "stack"
            && let Some(value) = part.strip_prefix("under=")
        {
            options.under = value.parse().map_err(|_| {
                error(
                    "E074",
                    line,
                    "stack under must be an integer from 0 to 65535",
                )
            })?;
        } else if (is_flex || kind == "grid")
            && let Some(value) = part.strip_prefix("gap=")
        {
            if options.spacing.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if kind == "grid"
            && let Some(value) = part.strip_prefix("min-cell=")
        {
            if options.min_cell.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.min_cell = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if kind == "grid"
            && let Some(value) = part.strip_prefix("max-cell=")
        {
            if options.max_cell.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.max_cell = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && parse_padding_option(part, &mut options.padding, line)? {
        } else if kind == "col"
            && let Some(value) = part.strip_prefix("max-w=")
        {
            options.max_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if kind == "col"
            && let Some(value) = part.strip_prefix("virtual-row=")
        {
            options.virtual_row = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("align=") {
            options.align = Some(parse_flex_alignment(value, line)?);
        } else if is_flex && part == "wrap" {
            options.wrap = true;
        } else if is_flex && let Some(value) = part.strip_prefix("wrap-gap=") {
            options.wrap_spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("wrap-align=") {
            options.wrap_align = Some(parse_flex_alignment(value, line)?);
        } else if kind == "scroll" {
            let scroll = options.scroll.as_mut().expect("scroll options");
            if let Some(value) = part.strip_prefix("dir=") {
                scroll.direction = match value {
                    "vertical" => ScrollDirection::Vertical,
                    "horizontal" => ScrollDirection::Horizontal,
                    "both" => ScrollDirection::Both,
                    _ => {
                        return Err(error(
                            "E074",
                            line,
                            "scroll direction must be vertical, horizontal, or both",
                        ));
                    }
                };
            } else if let Some(value) = part.strip_prefix("w=") {
                scroll.width = Some(parse_length(value, line)?);
            } else if let Some(value) = part.strip_prefix("h=") {
                scroll.height = Some(parse_length(value, line)?);
            } else if let Some(value) = part.strip_prefix("bar=") {
                scroll.hidden_bar = match value {
                    "visible" => false,
                    "hidden" => true,
                    _ => return Err(error("E074", line, "scroll bar must be visible or hidden")),
                };
            } else if let Some(value) = part.strip_prefix("bar-w=") {
                scroll.bar_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("bar-m=") {
                scroll.bar_margin = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("scroller-w=") {
                scroll.scroller_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("bar-gap=") {
                scroll.bar_spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("anchor-x=") {
                scroll.anchor_x = parse_scroll_anchor(value, line)?;
            } else if let Some(value) = part.strip_prefix("anchor-y=") {
                scroll.anchor_y = parse_scroll_anchor(value, line)?;
            } else if let Some(value) = part.strip_prefix("auto=") {
                scroll.auto_scroll = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("scroll=") {
                scroll.route = Some(parse_payload_route(value, line, 4)?);
            } else if let Some(value) = part.strip_prefix("viewport=") {
                scroll.viewport_route = Some(parse_payload_route(value, line, 14)?);
            } else if let Some(value) = part.strip_prefix("style=") {
                scroll.custom_style = Some(parse_extern_call(
                    value,
                    line,
                    "E074",
                    "scroll style must be a declared style call",
                )?);
            } else {
                return Err(error(
                    "E074",
                    line,
                    format!("unknown scroll property `{part}`"),
                ));
            }
        } else {
            return Err(error(
                "E074",
                line,
                format!("unknown layout property `{part}`"),
            ));
        }
    }
    if !options.wrap && (options.wrap_spacing.is_some() || options.wrap_align.is_some()) {
        return Err(error(
            "E074",
            line,
            "wrap-gap and wrap-align require `wrap`",
        ));
    }
    if [
        options.columns.is_some(),
        options.min_cell.is_some(),
        options.max_cell.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count()
        > 1
    {
        return Err(error(
            "E074",
            line,
            "grid cols, min-cell, and max-cell are mutually exclusive",
        ));
    }
    if options.min_cell.is_some() && options.grid_height.is_some() {
        return Err(error(
            "E074",
            line,
            "grid min-cell uses natural row height and cannot combine with h=",
        ));
    }
    if let Some(scroll) = &options.scroll
        && scroll.route.is_some()
        && scroll.viewport_route.is_some()
    {
        return Err(error(
            "E074",
            line,
            "scroll accepts either scroll= or viewport=, not both",
        ));
    }
    Ok(options)
}

pub(in crate::parser) fn parse_grid_sizing(source: &str, line: &Line) -> Result<GridSizing, Error> {
    if let Some(values) = source
        .strip_prefix("aspect(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let values = parse_expr_list(values, line)?;
        return match values.as_slice() {
            [width, height] => Ok(GridSizing::AspectRatio {
                width: width.clone(),
                height: height.clone(),
            }),
            _ => Err(error("E074", line, "grid aspect expects width and height")),
        };
    }
    Ok(GridSizing::EvenlyDistribute(parse_length(source, line)?))
}

pub(in crate::parser) fn parse_scroll_status_style(
    parts: &[String],
    line: &Line,
) -> Result<ScrollStatusStyle, Error> {
    let status = match parts.first().map(String::as_str) {
        Some("active") => ScrollStatus::Active,
        Some("hovered") => ScrollStatus::Hovered,
        Some("dragged") => ScrollStatus::Dragged,
        _ => unreachable!("scroll style dispatch validates the status"),
    };
    let mut style = ScrollStatusStyle {
        status,
        horizontal_interaction: None,
        vertical_interaction: None,
        horizontal_disabled: None,
        vertical_disabled: None,
        container: ContainerStyleOptions::default(),
        horizontal_rail: ScrollRailStyle::default(),
        vertical_rail: ScrollRailStyle::default(),
        gap: None,
        auto_scroll: ContainerStyleOptions::default(),
        auto_scroll_icon: None,
        span: Span::line(line.number),
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("x-disabled=") {
            style.horizontal_disabled = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("y-disabled=") {
            style.vertical_disabled = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("x-hovered=") {
            if status != ScrollStatus::Hovered {
                return Err(error("E074", line, "x-hovered requires hovered"));
            }
            style.horizontal_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("y-hovered=") {
            if status != ScrollStatus::Hovered {
                return Err(error("E074", line, "y-hovered requires hovered"));
            }
            style.vertical_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("x-dragged=") {
            if status != ScrollStatus::Dragged {
                return Err(error("E074", line, "x-dragged requires dragged"));
            }
            style.horizontal_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("y-dragged=") {
            if status != ScrollStatus::Dragged {
                return Err(error("E074", line, "y-dragged requires dragged"));
            }
            style.vertical_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else {
            return Err(error(
                "E074",
                line,
                format!("unknown scroll selector `{part}`; put styles in nested sections"),
            ));
        }
    }
    for child in &line.children {
        ensure_leaf(child)?;
        let parts = split_words(&child.text);
        let Some(kind) = parts.first().map(String::as_str) else {
            return Err(error("E074", child, "empty scroll style section"));
        };
        if kind == "gap" {
            let [property] = &parts[1..] else {
                return Err(error("E074", child, "scroll gap uses `gap bg=…`"));
            };
            let Some(value) = property.strip_prefix("bg=") else {
                return Err(error("E074", child, "scroll gap uses `gap bg=…`"));
            };
            parse_scroll_style_property(&mut style, &format!("gap={value}"), child)?;
            continue;
        }
        let prefix = match kind {
            "box" => "container-",
            "x-rail" => "x-rail-",
            "x-scroller" => "x-scroller-",
            "y-rail" => "y-rail-",
            "y-scroller" => "y-scroller-",
            "auto" => "auto-",
            _ => {
                return Err(error(
                    "E074",
                    child,
                    format!("unknown scroll style section `{kind}`"),
                ));
            }
        };
        for property in &parts[1..] {
            parse_scroll_style_property(&mut style, &format!("{prefix}{property}"), child)?;
        }
    }
    Ok(style)
}

pub(in crate::parser) fn parse_scroll_style_property(
    style: &mut ScrollStatusStyle,
    part: &str,
    line: &Line,
) -> Result<(), Error> {
    if let Some(property) = part.strip_prefix("container-") {
        if !parse_container_style_option(property, &mut style.container, line)? {
            return Err(error(
                "E074",
                line,
                format!("unknown scroll style property `{part}`"),
            ));
        }
    } else if parse_scroll_surface_property(
        part,
        "x-scroller-",
        &mut style.horizontal_rail.scroller,
        false,
        line,
    )? || parse_scroll_surface_property(
        part,
        "x-rail-",
        &mut style.horizontal_rail.rail,
        false,
        line,
    )? || parse_scroll_surface_property(
        part,
        "y-scroller-",
        &mut style.vertical_rail.scroller,
        false,
        line,
    )? || parse_scroll_surface_property(
        part,
        "y-rail-",
        &mut style.vertical_rail.rail,
        false,
        line,
    )? {
    } else if let Some(value) = part.strip_prefix("gap=") {
        style.gap = Some(parse_background_value(value, line)?);
    } else if let Some(value) = part.strip_prefix("auto-icon=") {
        style.auto_scroll_icon = Some(value.to_owned());
    } else if parse_scroll_surface_property(part, "auto-", &mut style.auto_scroll, true, line)? {
    } else {
        return Err(error(
            "E074",
            line,
            format!("unknown scroll style property `{part}`"),
        ));
    }
    Ok(())
}

pub(in crate::parser) fn parse_scroll_surface_property(
    part: &str,
    prefix: &str,
    style: &mut ContainerStyleOptions,
    allow_shadow: bool,
    line: &Line,
) -> Result<bool, Error> {
    let Some(property) = part.strip_prefix(prefix) else {
        return Ok(false);
    };
    if !parse_container_style_option(property, style, line)? {
        return Ok(false);
    }
    if style.text_color.is_some()
        || style.pixel_snap.is_some()
        || (!allow_shadow
            && (style.shadow_color.is_some()
                || style.shadow_x.is_some()
                || style.shadow_y.is_some()
                || style.shadow_blur.is_some()))
    {
        return Err(error(
            "E074",
            line,
            format!("unsupported scroll style property `{part}`"),
        ));
    }
    Ok(true)
}

pub(in crate::parser) fn parse_scroll_style_bool(source: &str, line: &Line) -> Result<bool, Error> {
    match source {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error(
            "E074",
            line,
            "scroll status selectors must be true or false",
        )),
    }
}

pub(in crate::parser) fn parse_flex_alignment(
    source: &str,
    line: &Line,
) -> Result<FlexAlignment, Error> {
    match source {
        "start" => Ok(FlexAlignment::Start),
        "center" => Ok(FlexAlignment::Center),
        "end" => Ok(FlexAlignment::End),
        _ => Err(error(
            "E074",
            line,
            "layout alignment must be start, center, or end",
        )),
    }
}

pub(in crate::parser) fn parse_flex_direction(
    source: &str,
    line: &Line,
) -> Result<FlexDirectionValue, Error> {
    match source {
        "row" => Ok(FlexDirectionValue::Row),
        "row-reverse" => Ok(FlexDirectionValue::RowReverse),
        "column" => Ok(FlexDirectionValue::Column),
        "column-reverse" => Ok(FlexDirectionValue::ColumnReverse),
        _ => Err(error(
            "E074",
            line,
            "flex direction must be row, row-reverse, column, or column-reverse",
        )),
    }
}

pub(in crate::parser) fn parse_flex_wrap(
    source: &str,
    line: &Line,
) -> Result<FlexWrapValue, Error> {
    match source {
        "nowrap" => Ok(FlexWrapValue::NoWrap),
        "wrap" => Ok(FlexWrapValue::Wrap),
        "wrap-reverse" => Ok(FlexWrapValue::WrapReverse),
        _ => Err(error(
            "E074",
            line,
            "flex-wrap must be nowrap, wrap, or wrap-reverse",
        )),
    }
}

pub(in crate::parser) fn parse_flex_content_alignment(
    source: &str,
    line: &Line,
) -> Result<FlexContentAlignment, Error> {
    match source {
        "flex-start" => Ok(FlexContentAlignment::FlexStart),
        "flex-end" => Ok(FlexContentAlignment::FlexEnd),
        "start" => Ok(FlexContentAlignment::Start),
        "end" => Ok(FlexContentAlignment::End),
        "center" => Ok(FlexContentAlignment::Center),
        "stretch" => Ok(FlexContentAlignment::Stretch),
        "space-between" => Ok(FlexContentAlignment::SpaceBetween),
        "space-around" => Ok(FlexContentAlignment::SpaceAround),
        "space-evenly" => Ok(FlexContentAlignment::SpaceEvenly),
        _ => Err(error(
            "E074",
            line,
            "flex content alignment must be start, end, flex-start, flex-end, center, stretch, space-between, space-around, or space-evenly",
        )),
    }
}

pub(in crate::parser) fn parse_flex_item_alignment(
    source: &str,
    line: &Line,
) -> Result<FlexItemAlignment, Error> {
    match source {
        "stretch" => Ok(FlexItemAlignment::Stretch),
        "start" => Ok(FlexItemAlignment::Start),
        "end" => Ok(FlexItemAlignment::End),
        "flex-start" => Ok(FlexItemAlignment::FlexStart),
        "flex-end" => Ok(FlexItemAlignment::FlexEnd),
        "center" => Ok(FlexItemAlignment::Center),
        "baseline" => Ok(FlexItemAlignment::Baseline),
        _ => Err(error(
            "E074",
            line,
            "flex item alignment must be start, end, flex-start, flex-end, center, baseline, or stretch",
        )),
    }
}

pub(in crate::parser) fn parse_scroll_anchor(
    source: &str,
    line: &Line,
) -> Result<ScrollAnchor, Error> {
    match source {
        "start" => Ok(ScrollAnchor::Start),
        "end" => Ok(ScrollAnchor::End),
        _ => Err(error("E074", line, "scroll anchor must be start or end")),
    }
}
