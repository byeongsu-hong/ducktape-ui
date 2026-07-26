use super::*;

pub(in crate::parser) fn parse_container(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let kind = parts.first().map_or("box", String::as_str);
    if line.children.len() != 1 {
        return Err(error(
            "E184",
            line,
            format!("{kind} requires exactly one child"),
        ));
    }
    let id = parts
        .get(1)
        .filter(|part| part.starts_with('#'))
        .map(|part| parse_id(part, line))
        .transpose()?;
    let mut options = ContainerOptions::default();
    let option_start = usize::from(id.is_some()) + 1;
    for part in &parts[option_start..] {
        if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("max-w=") {
            options.max_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max-h=") {
            options.max_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            options.align_x = Some(parse_flex_alignment(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-y=") {
            options.align_y = Some(parse_flex_alignment(value, line)?);
        } else if let Some(value) = part.strip_prefix("clip=") {
            options.clip = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if parse_padding_option(part, &mut options.padding, line)? {
        } else if let Some(value) = part.strip_prefix("order=") {
            options.flex_item.order = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("grow=") {
            options.flex_item.grow = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shrink=") {
            options.flex_item.shrink = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("basis=") {
            options.flex_item.basis = Some(parse_flex_basis(value, line)?);
        } else if let Some(value) = part.strip_prefix("self=") {
            options.flex_item.align_self = match value {
                "auto" => None,
                _ => Some(parse_flex_item_alignment(value, line)?),
            };
        } else if let Some(value) = part.strip_prefix("flex=") {
            parse_flex_shorthand(value, &mut options.flex_item, line)?;
        } else if let Some(value) = part.strip_prefix("m=") {
            options.flex_item.margin.all = Some(parse_flex_margin(value, line)?);
        } else if let Some(value) = part.strip_prefix("mx=") {
            options.flex_item.margin.x = Some(parse_flex_margin(value, line)?);
        } else if let Some(value) = part.strip_prefix("my=") {
            options.flex_item.margin.y = Some(parse_flex_margin(value, line)?);
        } else if let Some(value) = part.strip_prefix("mt=") {
            options.flex_item.margin.top = Some(parse_flex_margin(value, line)?);
        } else if let Some(value) = part.strip_prefix("mr=") {
            options.flex_item.margin.right = Some(parse_flex_margin(value, line)?);
        } else if let Some(value) = part.strip_prefix("mb=") {
            options.flex_item.margin.bottom = Some(parse_flex_margin(value, line)?);
        } else if let Some(value) = part.strip_prefix("ml=") {
            options.flex_item.margin.left = Some(parse_flex_margin(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = Some(parse_extern_call(
                value,
                line,
                "E184",
                format!("{kind} style must be a declared style call"),
            )?);
        } else if parse_container_style_option(part, &mut options.style, line)? {
        } else {
            return Err(error(
                "E184",
                line,
                format!("unknown {kind} property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Container {
        options: Box::new(options),
        id,
        styles,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_flex_basis(source: &str, line: &Line) -> Result<FlexBasisValue, Error> {
    match source {
        "auto" => Ok(FlexBasisValue::Auto),
        "content" => Ok(FlexBasisValue::Content),
        _ => parse_flex_percentage(source, line)?.map_or_else(
            || {
                Ok(FlexBasisValue::Fixed(parse_expr(
                    strip_wrapping_parens(source),
                    line,
                )?))
            },
            |value| Ok(FlexBasisValue::Percent(value)),
        ),
    }
}

fn parse_flex_margin(source: &str, line: &Line) -> Result<FlexMarginValue, Error> {
    if source == "auto" {
        return Ok(FlexMarginValue::Auto);
    }
    parse_flex_percentage(source, line)?.map_or_else(
        || {
            Ok(FlexMarginValue::Fixed(parse_expr(
                strip_wrapping_parens(source),
                line,
            )?))
        },
        |value| Ok(FlexMarginValue::Percent(value)),
    )
}

fn parse_flex_percentage(source: &str, line: &Line) -> Result<Option<Expr>, Error> {
    source
        .strip_prefix("percent(")
        .and_then(|value| value.strip_suffix(')'))
        .map(|value| parse_expr(strip_wrapping_parens(value), line))
        .transpose()
}

fn parse_flex_shorthand(
    source: &str,
    options: &mut FlexItemOptions,
    line: &Line,
) -> Result<(), Error> {
    match source {
        "none" => {
            options.grow = Some(Expr::F64(0.0));
            options.shrink = Some(Expr::F64(0.0));
            options.basis = Some(FlexBasisValue::Auto);
        }
        "auto" => {
            options.grow = Some(Expr::F64(1.0));
            options.shrink = Some(Expr::F64(1.0));
            options.basis = Some(FlexBasisValue::Auto);
        }
        "initial" => {
            options.grow = Some(Expr::F64(0.0));
            options.shrink = Some(Expr::F64(1.0));
            options.basis = Some(FlexBasisValue::Auto);
        }
        _ => {
            let values = split_top(source, ',');
            if values.is_empty() || values.len() > 3 {
                return Err(error(
                    "E184",
                    line,
                    "flex expects grow[,shrink[,basis]], auto, initial, or none",
                ));
            }
            options.grow = Some(parse_expr(strip_wrapping_parens(values[0]), line)?);
            options.shrink = Some(if values.len() > 1 {
                parse_expr(strip_wrapping_parens(values[1]), line)?
            } else {
                Expr::F64(1.0)
            });
            options.basis = Some(if values.len() > 2 {
                parse_flex_basis(values[2], line)?
            } else {
                FlexBasisValue::Fixed(Expr::F64(0.0))
            });
        }
    }
    Ok(())
}

pub(in crate::parser) fn parse_overlay(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E185",
            line,
            "overlay uses typed properties and does not accept `@` utilities",
        ));
    }
    if line.children.len() != 2
        || line.children[0].text != "content"
        || line.children[1].text != "layer"
    {
        return Err(error(
            "E185",
            line,
            "overlay requires `content` then `layer` sections",
        ));
    }
    for section in &line.children {
        if section.children.len() != 1 {
            return Err(error(
                "E185",
                section,
                format!(
                    "overlay `{}` requires exactly one child; wrap siblings in row, col, grid, or stack",
                    section.text
                ),
            ));
        }
    }

    let mut id = None;
    let mut visible = None;
    let mut dismiss = None;
    let mut backdrop = "black/50".to_owned();
    let mut padding = Expr::F64(24.0);
    let mut align_x = FlexAlignment::Center;
    let mut align_y = FlexAlignment::Center;
    for part in &parts[1..] {
        if part.starts_with('#') {
            parse_unique_id(part, &mut id, line, "E185", "overlay")?;
        } else if let Some(value) = part.strip_prefix("when=") {
            visible = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("dismiss=") {
            dismiss = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("backdrop=") {
            backdrop = value.to_owned();
        } else if let Some(value) = part.strip_prefix("p=") {
            padding = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("align-x=") {
            align_x = parse_flex_alignment(value, line)?;
        } else if let Some(value) = part.strip_prefix("align-y=") {
            align_y = parse_flex_alignment(value, line)?;
        } else {
            return Err(error(
                "E185",
                line,
                format!("unknown overlay property `{part}`"),
            ));
        }
    }
    let visible = visible.ok_or_else(|| error("E185", line, "overlay requires `when=`"))?;
    Ok(ViewNode::Overlay {
        id,
        options: OverlayOptions {
            visible,
            dismiss,
            backdrop,
            padding,
            align_x,
            align_y,
        },
        content: Box::new(parse_view(&line.children[0].children[0])?),
        layer: Box::new(parse_view(&line.children[1].children[0])?),
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pane_ratio(value: &str, line: &Line) -> Result<f32, Error> {
    let ratio = value.parse::<f32>().map_err(|_| {
        error(
            "E187",
            line,
            "pane split ratio must be a number from 0 to 1",
        )
    })?;
    if !(0.0..=1.0).contains(&ratio) {
        return Err(error(
            "E187",
            line,
            "pane split ratio must be a number from 0 to 1",
        ));
    }
    Ok(ratio)
}

pub(in crate::parser) fn parse_background_value(
    source: &str,
    line: &Line,
) -> Result<BackgroundValue, Error> {
    let Some(inner) = source
        .strip_prefix("linear(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        if source.starts_with("linear(") {
            return Err(error("E189", line, "linear background is missing `)`"));
        }
        return Ok(BackgroundValue::Color(source.to_owned()));
    };
    let parts = split_top(inner, ',');
    let angle = parse_expr(
        parts
            .first()
            .copied()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| error("E189", line, "linear background requires an angle"))?,
        line,
    )?;
    if parts.len() > 9 {
        return Err(error(
            "E189",
            line,
            "linear background accepts at most 8 color stops",
        ));
    }
    let stops = parts[1..]
        .iter()
        .map(|stop| {
            let (color, offset) = split_top_once(stop, '@')
                .ok_or_else(|| error("E189", line, "linear color stops use `color@offset`"))?;
            if color.is_empty() || offset.is_empty() {
                return Err(error("E189", line, "linear color stops use `color@offset`"));
            }
            Ok(GradientStop {
                color: color.to_owned(),
                offset: parse_expr(offset, line)?,
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(BackgroundValue::Linear { angle, stops })
}

pub(in crate::parser) fn parse_container_style_option(
    part: &str,
    style: &mut ContainerStyleOptions,
    line: &Line,
) -> Result<bool, Error> {
    let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
    if let Some(value) = part.strip_prefix("bg=") {
        style.background = Some(parse_background_value(value, line)?);
    } else if let Some(value) = part.strip_prefix("text=") {
        style.text_color = Some(value.to_owned());
    } else if let Some(value) = part.strip_prefix("border=") {
        style.border_color = Some(value.to_owned());
    } else if let Some(value) = part.strip_prefix("border-w=") {
        style.border_width = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("r=") {
        style.radius = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("r-tl=") {
        style.radius_top_left = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("r-tr=") {
        style.radius_top_right = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("r-br=") {
        style.radius_bottom_right = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("r-bl=") {
        style.radius_bottom_left = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("shadow=") {
        style.shadow_color = Some(value.to_owned());
    } else if let Some(value) = part.strip_prefix("shadow-x=") {
        style.shadow_x = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("shadow-y=") {
        style.shadow_y = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("shadow-blur=") {
        style.shadow_blur = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("px-snap=") {
        style.pixel_snap = Some(parse(value)?);
    } else {
        return Ok(false);
    }
    Ok(true)
}

pub(in crate::parser) fn parse_pane_view(
    name: &str,
    style_parts: &[String],
    styles: Vec<String>,
    line: &Line,
    names: &mut std::collections::HashSet<String>,
    panes: &mut Vec<PaneView>,
) -> Result<String, Error> {
    let name = identifier(name, line)?;
    if !names.insert(name.clone()) {
        return Err(error("E187", line, format!("duplicate pane `{name}`")));
    }
    let mut maximized = None;
    let mut style = ContainerStyleOptions::default();
    for part in style_parts {
        if let Some(value) = part.strip_prefix("maximized=") {
            if maximized.replace(identifier(value, line)?).is_some() {
                return Err(error("E187", line, "duplicate pane `maximized` binding"));
            }
        } else if !parse_container_style_option(part, &mut style, line)? {
            return Err(error(
                "E187",
                line,
                format!("unknown pane style property `{part}`"),
            ));
        }
    }
    let (content, title) = parse_pane_contents(line)?;
    panes.push(PaneView {
        name: name.clone(),
        maximized,
        content,
        title,
        styles,
        style,
        span: Span::line(line.number),
    });
    Ok(name)
}

pub(in crate::parser) fn parse_pane_contents(
    line: &Line,
) -> Result<(Box<ViewNode>, Option<PaneTitle>), Error> {
    let mut content = None;
    let mut title = None;
    let mut controls = None;
    let mut compact_controls = None;
    for section in &line.children {
        let (core, styles) = split_style_utilities(&section.text);
        let parts = split_words(core);
        let kind = parts.first().map(String::as_str).unwrap_or("");
        match kind {
            "title" => {
                if title.is_some() {
                    return Err(error("E187", section, "duplicate pane `title` section"));
                }
                if section.children.len() != 1 {
                    return Err(error(
                        "E187",
                        section,
                        "pane `title` section requires exactly one child",
                    ));
                }
                title = Some(parse_pane_title(&parts[1..], styles, section)?);
            }
            "controls" if parts.len() == 1 && styles.is_empty() => {
                if controls.is_some() {
                    return Err(error("E187", section, "duplicate pane `controls` section"));
                }
                if section.children.len() != 1 {
                    return Err(error(
                        "E187",
                        section,
                        "pane `controls` section requires exactly one child",
                    ));
                }
                controls = Some(Box::new(parse_view(&section.children[0])?));
            }
            "compact" if parts.len() == 1 && styles.is_empty() => {
                if compact_controls.is_some() {
                    return Err(error("E187", section, "duplicate pane `compact` section"));
                }
                if section.children.len() != 1 {
                    return Err(error(
                        "E187",
                        section,
                        "pane `compact` section requires exactly one child",
                    ));
                }
                compact_controls = Some(Box::new(parse_view(&section.children[0])?));
            }
            "controls" | "compact" => {
                return Err(error(
                    "E187",
                    section,
                    format!("pane `{kind}` section does not accept properties or styles"),
                ));
            }
            _ => {
                if content.is_some() {
                    return Err(error(
                        "E187",
                        section,
                        "pane requires one content node; wrap siblings in row or col",
                    ));
                }
                content = Some(Box::new(parse_view(section)?));
            }
        }
    }
    let content = content.ok_or_else(|| error("E187", line, "pane requires one content node"))?;
    if controls.is_some() && title.is_none() {
        return Err(error(
            "E187",
            line,
            "pane controls require a `title` section",
        ));
    }
    if compact_controls.is_some() && controls.is_none() {
        return Err(error(
            "E187",
            line,
            "pane compact controls require a `controls` section",
        ));
    }
    if title
        .as_ref()
        .is_some_and(|title| title.always_show_controls)
        && controls.is_none()
    {
        return Err(error(
            "E187",
            line,
            "pane title `always-controls` requires a `controls` section",
        ));
    }
    if let Some(title) = &mut title {
        title.controls = controls;
        title.compact_controls = compact_controls;
    }
    Ok((content, title))
}

pub(in crate::parser) fn parse_pane_title(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<PaneTitle, Error> {
    let mut padding = PaddingOptions::default();
    let mut always_show_controls = false;
    let mut style = ContainerStyleOptions::default();
    for part in parts {
        if parse_padding_option(part, &mut padding, line)? {
        } else if part == "always-controls" {
            always_show_controls = true;
        } else if parse_container_style_option(part, &mut style, line)? {
        } else {
            return Err(error(
                "E187",
                line,
                format!("unknown pane title property `{part}`"),
            ));
        }
    }
    Ok(PaneTitle {
        content: Box::new(parse_view(&line.children[0])?),
        controls: None,
        compact_controls: None,
        padding,
        always_show_controls,
        styles,
        style,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pane_configuration(
    line: &Line,
    names: &mut std::collections::HashSet<String>,
    splits: &mut std::collections::HashSet<String>,
    panes: &mut Vec<PaneView>,
) -> Result<PaneConfiguration, Error> {
    let (core, styles) = split_style_utilities(&line.text);
    let parts = split_words(core);
    match parts.first().map(String::as_str) {
        Some("pane") if parts.len() >= 2 => Ok(PaneConfiguration::Pane(parse_pane_view(
            &parts[1],
            &parts[2..],
            styles,
            line,
            names,
            panes,
        )?)),
        Some("split") if (2..=4).contains(&parts.len()) => {
            if !styles.is_empty() {
                return Err(error("E187", line, "nested pane split does not accept `@`"));
            }
            let (name, axis_index) = if matches!(parts[1].as_str(), "horizontal" | "vertical") {
                (None, 1)
            } else {
                let name = identifier(&parts[1], line)?;
                if !splits.insert(name.clone()) {
                    return Err(error(
                        "E187",
                        line,
                        format!("duplicate pane split `{name}`"),
                    ));
                }
                (Some(name), 2)
            };
            let axis = match parts.get(axis_index).map(String::as_str) {
                Some("horizontal") => PaneAxis::Horizontal,
                Some("vertical") => PaneAxis::Vertical,
                _ => {
                    return Err(error(
                        "E187",
                        line,
                        "nested pane split uses `split [name] horizontal|vertical ratio=value`",
                    ));
                }
            };
            if parts.len() > axis_index + 2 {
                return Err(error(
                    "E187",
                    line,
                    "nested pane split uses `split [name] horizontal|vertical ratio=value`",
                ));
            }
            let ratio = parts.get(axis_index + 1).map_or(Ok(0.5), |part| {
                parse_pane_ratio(
                    part.strip_prefix("ratio=").ok_or_else(|| {
                        error("E187", line, "nested pane split ratio uses `ratio=value`")
                    })?,
                    line,
                )
            })?;
            if line.children.len() != 2 {
                return Err(error(
                    "E187",
                    line,
                    "nested pane split requires exactly two pane or split children",
                ));
            }
            Ok(PaneConfiguration::Split {
                name,
                axis,
                ratio,
                a: Box::new(parse_pane_configuration(
                    &line.children[0],
                    names,
                    splits,
                    panes,
                )?),
                b: Box::new(parse_pane_configuration(
                    &line.children[1],
                    names,
                    splits,
                    panes,
                )?),
            })
        }
        _ => Err(error(
            "E187",
            line,
            "pane configuration uses `pane name` or `split [name] axis ratio=value`",
        )),
    }
}

pub(in crate::parser) fn parse_closed_pane(
    line: &Line,
    names: &mut std::collections::HashSet<String>,
    panes: &mut Vec<PaneView>,
) -> Result<(), Error> {
    let (core, styles) = split_style_utilities(&line.text);
    let parts = split_words(core);
    if parts.len() < 3 || parts[0] != "pane" || parts[2] != "closed" {
        return Err(error(
            "E187",
            line,
            "extra pane templates use `pane name closed`",
        ));
    }
    parse_pane_view(&parts[1], &parts[3..], styles, line, names, panes)?;
    Ok(())
}

pub(in crate::parser) fn parse_pane_template(
    line: &Line,
    names: &mut std::collections::HashSet<String>,
) -> Result<PaneTemplate, Error> {
    let (core, styles) = split_style_utilities(&line.text);
    let parts = split_words(core);
    if parts.len() < 5 || parts[0] != "pane" || parts[2] != "in" {
        return Err(error(
            "E187",
            line,
            "dynamic pane templates use `pane item in state by=item.id`",
        ));
    }
    let key = parts[4].strip_prefix("by=").ok_or_else(|| {
        error(
            "E187",
            line,
            "dynamic pane templates use `pane item in state by=item.id`",
        )
    })?;
    let item = identifier(&parts[1], line)?;
    let items = identifier(&parts[3], line)?;
    let mut panes = Vec::new();
    parse_pane_view(&item, &parts[5..], styles, line, names, &mut panes)?;
    let pane = panes.pop().expect("pane template was parsed");
    if pane.maximized.as_deref() == Some(item.as_str()) {
        return Err(error(
            "E187",
            line,
            "pane `maximized` binding must differ from its template item",
        ));
    }
    Ok(PaneTemplate {
        item,
        items,
        key: parse_expr(strip_wrapping_parens(key), line)?,
        pane,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn is_pane_template(line: &Line) -> bool {
    let (core, _) = split_style_utilities(&line.text);
    split_words(core).get(2).map(String::as_str) == Some("in")
}

pub(in crate::parser) fn parse_pane_grid(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E187", line, "panes does not accept `@` utilities"));
    }
    let name = parts
        .get(1)
        .filter(|part| part.starts_with('#'))
        .ok_or_else(|| error("E187", line, "panes requires a static `#id`"))?;
    let name = identifier(name.trim_start_matches('#'), line)?;
    let mut options = PaneGridOptions::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("w=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("h=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("gap=") {
            options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("min-size=") {
            options.min_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("resize=") {
            options.resize_leeway = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "drag" {
            options.draggable = true;
        } else if let Some(value) = part.strip_prefix("click=") {
            options.click = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = Some(parse_extern_call(
                value,
                line,
                "E187",
                "panes style must be a declared panes style call",
            )?);
        } else {
            return Err(error(
                "E187",
                line,
                format!("unknown panes property `{part}`"),
            ));
        }
    }
    let children = if line
        .children
        .first()
        .is_some_and(|child| child.text == "style")
    {
        options.style = parse_pane_grid_style(&line.children[0])?;
        &line.children[1..]
    } else {
        if line
            .children
            .iter()
            .skip(1)
            .any(|child| child.text == "style")
        {
            return Err(error("E187", line, "panes `style` must be its first child"));
        }
        line.children.as_slice()
    };
    let mut names = std::collections::HashSet::new();
    let mut splits = std::collections::HashSet::new();
    let mut panes = Vec::new();
    let mut templates = Vec::new();
    let (configuration, closed) = children.split_first().ok_or_else(|| {
        error(
            "E187",
            line,
            "panes requires an initial pane or split configuration",
        )
    })?;
    let configuration =
        parse_pane_configuration(configuration, &mut names, &mut splits, &mut panes)?;
    for pane in closed {
        if is_pane_template(pane) {
            templates.push(parse_pane_template(pane, &mut names)?);
        } else {
            parse_closed_pane(pane, &mut names, &mut panes)?;
        }
    }
    Ok(ViewNode::PaneGrid {
        name,
        configuration,
        options,
        panes,
        templates,
        span: Span::line(line.number),
    })
}

pub(in crate::parser) fn parse_pane_grid_style(line: &Line) -> Result<PaneGridStyle, Error> {
    if line.children.is_empty() {
        return Err(error(
            "E187",
            line,
            "panes style requires at least one status",
        ));
    }
    let mut style = PaneGridStyle::default();
    let mut statuses = std::collections::HashSet::new();
    for status in &line.children {
        if !status.children.is_empty() {
            return Err(error("E187", status, "panes style statuses are leaves"));
        }
        let parts = split_words(&status.text);
        let kind = parts.first().map(String::as_str).unwrap_or("");
        if !statuses.insert(kind.to_owned()) {
            return Err(error(
                "E187",
                status,
                format!("duplicate panes style status `{kind}`"),
            ));
        }
        if parts.len() == 1 {
            return Err(error(
                "E187",
                status,
                format!("panes style status `{kind}` requires properties"),
            ));
        }
        let parse = |value: &str| parse_expr(strip_wrapping_parens(value), status);
        for part in &parts[1..] {
            match kind {
                "hovered-region" => {
                    if let Some(value) = part.strip_prefix("bg=") {
                        style.region_background = Some(parse_background_value(value, status)?);
                    } else if let Some(value) = part.strip_prefix("border=") {
                        style.region_border = Some(value.to_owned());
                    } else if let Some(value) = part.strip_prefix("border-w=") {
                        style.region_border_width = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("r=") {
                        style.region_radius = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("r-tl=") {
                        style.region_radius_top_left = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("r-tr=") {
                        style.region_radius_top_right = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("r-br=") {
                        style.region_radius_bottom_right = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("r-bl=") {
                        style.region_radius_bottom_left = Some(parse(value)?);
                    } else {
                        return Err(error(
                            "E187",
                            status,
                            format!("unknown hovered-region style property `{part}`"),
                        ));
                    }
                }
                "hovered-split" | "picked-split" => {
                    let (color, width) = if kind == "hovered-split" {
                        (&mut style.hovered_split, &mut style.hovered_split_width)
                    } else {
                        (&mut style.picked_split, &mut style.picked_split_width)
                    };
                    if let Some(value) = part.strip_prefix("color=") {
                        *color = Some(value.to_owned());
                    } else if let Some(value) = part.strip_prefix("w=") {
                        *width = Some(parse(value)?);
                    } else {
                        return Err(error(
                            "E187",
                            status,
                            format!("unknown {kind} style property `{part}`"),
                        ));
                    }
                }
                _ => {
                    return Err(error(
                        "E187",
                        status,
                        "panes style status must be hovered-region, hovered-split, or picked-split",
                    ));
                }
            }
        }
    }
    Ok(style)
}
