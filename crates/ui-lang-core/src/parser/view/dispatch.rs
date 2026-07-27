use super::*;

pub(in crate::parser) fn parse_view(line: &Line) -> Result<ViewNode, Error> {
    if let Some(value) = line.text.strip_prefix("match ") {
        if line.children.is_empty() {
            return Err(error("E060", line, "match requires at least one arm"));
        }
        // ponytail: lower through the existing If IR; add a Match node only if
        // single evaluation of large or expensive match expressions matters.
        let value = parse_expr(value, line)?;
        let mut matched = None;
        let mut branches = Vec::new();
        for (index, arm) in line.children.iter().enumerate() {
            if arm.children.is_empty() {
                return Err(error("E060", arm, "match arms require view content"));
            }
            let condition = if arm.text == "_" {
                if index + 1 != line.children.len() {
                    return Err(error("E060", arm, "the `_` match arm must be last"));
                }
                matched
                    .take()
                    .map_or(Expr::Bool(true), |value| Expr::Unary {
                        op: UnaryOp::Not,
                        value: Box::new(value),
                    })
            } else {
                let current = Expr::Binary {
                    left: Box::new(value.clone()),
                    op: BinaryOp::Eq,
                    right: Box::new(parse_expr(&arm.text, arm)?),
                };
                let condition = matched.as_ref().map_or_else(
                    || current.clone(),
                    |previous| Expr::Binary {
                        left: Box::new(Expr::Unary {
                            op: UnaryOp::Not,
                            value: Box::new(previous.clone()),
                        }),
                        op: BinaryOp::And,
                        right: Box::new(current.clone()),
                    },
                );
                matched = Some(matched.map_or(current.clone(), |previous| Expr::Binary {
                    left: Box::new(previous),
                    op: BinaryOp::Or,
                    right: Box::new(current),
                }));
                condition
            };
            branches.push(ViewNode::If {
                condition,
                children: arm
                    .children
                    .iter()
                    .map(parse_view)
                    .collect::<Result<_, _>>()?,
                span: Span::line(arm.number),
            });
        }
        return Ok(ViewNode::If {
            condition: Expr::Bool(true),
            children: branches,
            span: Span::line(line.number),
        });
    }
    if let Some(condition) = line.text.strip_prefix("if ") {
        return Ok(ViewNode::If {
            condition: parse_expr(condition, line)?,
            children: line
                .children
                .iter()
                .map(parse_view)
                .collect::<Result<_, _>>()?,
            span: Span::line(line.number),
        });
    }
    if let Some(loop_source) = line.text.strip_prefix("for ") {
        let Some((item, items)) = loop_source.split_once(" in ") else {
            return Err(error("E060", line, "loops use `for item in items`"));
        };
        return Ok(ViewNode::For {
            item: identifier(item.trim(), line)?,
            items: parse_expr(items.trim(), line)?,
            children: line
                .children
                .iter()
                .map(parse_view)
                .collect::<Result<_, _>>()?,
            span: Span::line(line.number),
        });
    }

    let (without_route, route_source) = split_top_marker(&line.text, "->")
        .map_or((line.text.as_str(), None), |(left, right)| {
            (left, Some(right))
        });
    let (core, styles) = split_style_utilities(without_route);
    for style in &styles {
        line.record_symbol(SymbolKind::Recipe, style, false, style);
    }
    let parts = split_words(core);
    let Some(kind) = parts.first().map(String::as_str) else {
        return Err(error("E061", line, "empty view node"));
    };
    if route_source.is_some()
        && !matches!(
            kind,
            "button"
                | "checkbox"
                | "toggler"
                | "slider"
                | "radio"
                | "pick"
                | "combo"
                | "markdown"
                | "rich-text"
                | "editor"
                | "extern"
                | "themer"
                | "shader"
        )
        && !kind.chars().next().is_some_and(char::is_uppercase)
    {
        return Err(error(
            "E081",
            line,
            format!("`{kind}` does not emit a route payload"),
        ));
    }
    let span = Span::line(line.number);

    match kind {
        "col" | "row" | "flex" | "scroll" | "grid" | "stack" => {
            let id = parts
                .get(1)
                .filter(|part| part.starts_with('#'))
                .map(|part| parse_id(part, line))
                .transpose()?;
            let option_start = usize::from(id.is_some()) + 1;
            let option_parts = parts[option_start..].to_vec();
            let mut options = if kind == "flex" {
                parse_flexbox_options(&option_parts, line)?
            } else {
                parse_layout_options(kind, &option_parts, line)?
            };
            let layout_kind = match options.flexbox.as_ref().map(|flex| flex.direction) {
                Some(FlexDirectionValue::Row | FlexDirectionValue::RowReverse) => "row",
                Some(FlexDirectionValue::Column | FlexDirectionValue::ColumnReverse) => "col",
                None => kind,
            };
            let children = if layout_kind == "scroll" {
                let scroll = options.scroll.as_mut().expect("scroll options");
                let mut content = Vec::new();
                for child in &line.children {
                    let parts = split_words(&child.text);
                    if matches!(
                        parts.first().map(String::as_str),
                        Some("active" | "hovered" | "dragged")
                    ) {
                        scroll
                            .styles
                            .push(parse_scroll_status_style(&parts, child)?);
                    } else {
                        content.push(parse_view(child)?);
                    }
                }
                if content.len() != 1 {
                    return Err(error(
                        "E062",
                        line,
                        "scroll must have exactly one content child beside status styles",
                    ));
                }
                content
            } else {
                line.children
                    .iter()
                    .map(parse_view)
                    .collect::<Result<_, _>>()?
            };
            Ok(ViewNode::Layout {
                kind: match layout_kind {
                    "col" => Layout::Column,
                    "row" => Layout::Row,
                    "scroll" => Layout::Scroll,
                    "grid" => Layout::Grid,
                    _ => Layout::Stack,
                },
                options: Box::new(options),
                id,
                styles,
                children,
                span,
            })
        }
        "text" => parse_text(&parts, styles, line),
        "rich-text" => parse_rich_text(&parts, styles, route_source, line),
        "box" => parse_container(&parts, styles, line),
        "overlay" => parse_overlay(&parts, styles, line),
        "panes" => parse_pane_grid(&parts, styles, line),
        "input" => parse_input(&parts, styles, line),
        "button" => parse_button(&parts, styles, route_source, line),
        "checkbox" => parse_checkbox(&parts, styles, route_source, line),
        "toggler" => parse_toggler(&parts, styles, route_source, line),
        "slider" => parse_slider(&parts, styles, route_source, line),
        "progress" => parse_progress(&parts, styles, line),
        "radio" => parse_radio(&parts, styles, route_source, line),
        "pick" => parse_pick_list(&parts, styles, route_source, line),
        "combo" => parse_combo_box(&parts, styles, route_source, line),
        "rule" => parse_rule(&parts, styles, line),
        "qr" => parse_qr_code(&parts, styles, line),
        "space" => parse_space(&parts, styles, line),
        "extern" => parse_extern_component(&parts, styles, route_source, line),
        "themer" => parse_themer(&parts, styles, route_source, line),
        "shader" => parse_shader(&parts, styles, route_source, line),
        "image" | "svg" | "viewer" => parse_media(kind, &parts, styles, line),
        "tooltip" => parse_tooltip(&parts, styles, line),
        "mouse" => parse_mouse_area(&parts, styles, line),
        "resize-handle" => parse_resize_handle(&parts, styles, line),
        "canvas" => parse_canvas(&parts, styles, line),
        "theme" => parse_theme(&parts, styles, line),
        "slot" => parse_slot(&parts, styles, line),
        "keyed" => parse_keyed_column(&parts, styles, line),
        "lazy" => parse_lazy(&parts, styles, line),
        "markdown" => parse_markdown(&parts, styles, route_source, line),
        "editor" => parse_text_editor(&parts, styles, route_source, line),
        "table" => parse_table(&parts, styles, line),
        "float" => parse_float(&parts, styles, line),
        "pin" => parse_pin(&parts, styles, line),
        "sensor" => parse_sensor(&parts, styles, line),
        "responsive" => parse_responsive(&parts, styles, line),
        _ if kind.chars().next().is_some_and(char::is_uppercase) => {
            if !styles.is_empty() {
                return Err(error(
                    "E040",
                    line,
                    "component calls do not accept `@` utilities; style the component root",
                ));
            }
            let (name, args, id) = parse_component_call(&parts, line)?;
            let (slots, events) = parse_component_children(&name, line)?;
            let route = route_source
                .map(|route| parse_route(route.trim(), line))
                .transpose()?;
            Ok(ViewNode::Component {
                name,
                args,
                id,
                slots,
                events,
                route,
                span,
            })
        }
        _ => Err(error("E064", line, format!("unknown view node `{kind}`"))),
    }
}
