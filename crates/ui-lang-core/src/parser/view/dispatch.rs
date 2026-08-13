use super::*;

pub(in crate::parser) fn parse_view(line: &Line) -> Result<ViewNode, Error> {
    if line.children.iter().any(|child| child.text == "with") {
        if matches!(
            line.text.split_ascii_whitespace().next(),
            Some("match" | "if" | "for")
        ) {
            return Err(error(
                "E040",
                line,
                "with is metadata for a widget or component call",
            ));
        }
        let mut metadata_blocks = line
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.text == "with");
        let (metadata_index, metadata) = metadata_blocks
            .next()
            .expect("a with block was found above");
        if metadata_blocks.next().is_some() {
            return Err(error("E040", line, "node has duplicate with blocks"));
        }
        if metadata.children.is_empty() {
            return Err(error("E040", metadata, "with cannot be empty"));
        }
        let mut options = Vec::new();
        let mut utilities = Vec::new();
        for item in &metadata.children {
            ensure_leaf(item)?;
            if let Some(styles) = item.text.strip_prefix('@') {
                utilities.extend(
                    styles
                        .split_ascii_whitespace()
                        .map(|style| style.trim_start_matches('@')),
                );
            } else {
                options.push(item.text.as_str());
            }
        }
        let (without_route, route) = split_top_marker(&line.text, "->")
            .map_or((line.text.as_str(), None), |(node, route)| {
                (node.trim(), Some(route.trim()))
            });
        let (core, existing_utilities) = split_style_utilities(without_route, line);
        let mut expanded = line.clone();
        expanded.metadata = metadata.children.clone();
        expanded.text = core.to_owned();
        for option in options {
            expanded.text.push(' ');
            expanded.text.push_str(option);
        }
        let all_utilities = existing_utilities
            .iter()
            .map(String::as_str)
            .chain(utilities)
            .collect::<Vec<_>>();
        if !all_utilities.is_empty() {
            expanded.text.push_str(" @");
            expanded.text.push_str(&all_utilities.join(" "));
        }
        if let Some(route) = route {
            expanded.text.push_str(" -> ");
            expanded.text.push_str(route);
        }
        expanded.children = line
            .children
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != metadata_index)
            .map(|(_, child)| child.clone())
            .collect();
        return parse_view(&expanded);
    }
    if let Some(value) = line.text.strip_prefix("match ") {
        if line.children.is_empty() {
            return Err(error("E060", line, "match requires at least one arm"));
        }
        let value = parse_expr(value, line)?;
        let patterns = line
            .children
            .iter()
            .map(parse_match_pattern)
            .collect::<Result<Vec<_>, _>>()?;
        if patterns.iter().any(|pattern| {
            pattern
                .as_ref()
                .is_some_and(|pattern| !matches!(pattern, MatchPattern::Wildcard))
        }) {
            if patterns.iter().any(Option::is_none) {
                return Err(error(
                    "E060",
                    line,
                    "typed match patterns cannot be mixed with literal arms",
                ));
            }
            let mut arms = Vec::new();
            for (index, (arm, pattern)) in line.children.iter().zip(patterns).enumerate() {
                if arm.children.is_empty() {
                    return Err(error("E060", arm, "match arms require view content"));
                }
                let pattern = pattern.expect("all typed match arms have a pattern");
                if matches!(pattern, MatchPattern::Wildcard) && index + 1 != line.children.len() {
                    return Err(error("E060", arm, "the `_` match arm must be last"));
                }
                arms.push(MatchArm {
                    pattern,
                    children: arm
                        .children
                        .iter()
                        .map(parse_view)
                        .collect::<Result<_, _>>()?,
                    span: Span::line(arm.number),
                });
            }
            return Ok(ViewNode::Match {
                value,
                arms,
                span: Span::line(line.number),
            });
        }
        // Literal matches keep their established first-match lowering.
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
    let (core, styles) = split_style_utilities(without_route, line);
    for style in &styles {
        line.record_symbol(
            SymbolKind::Recipe,
            style,
            false,
            crate::unqualified_name(style),
        );
    }
    let parts = split_words(core);
    let Some(kind) = parts.first().map(String::as_str) else {
        return Err(error("E061", line, "empty view node"));
    };
    let component_like = kind
        .rsplit("::")
        .next()
        .and_then(|name| name.chars().next())
        .is_some_and(char::is_uppercase);
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
        && !component_like
    {
        return Err(error(
            "E081",
            line,
            format!("`{kind}` does not emit a route payload"),
        ));
    }
    let span = Span::line(line.number);

    match kind {
        "col" | "row" | "flex" | "scroll" | "grid" | "stack" | "hover" => {
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
            if layout_kind == "hover" && children.len() != 2 {
                return Err(error(
                    "E062",
                    line,
                    "hover takes exactly two children: the base, then the reveal",
                ));
            }
            Ok(ViewNode::Layout {
                kind: match layout_kind {
                    "col" => Layout::Column,
                    "row" => Layout::Row,
                    "scroll" => Layout::Scroll,
                    "grid" => Layout::Grid,
                    "hover" => Layout::Hover,
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
        _ if component_like => {
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

fn parse_match_pattern(line: &Line) -> Result<Option<MatchPattern>, Error> {
    if line.text == "_" {
        return Ok(Some(MatchPattern::Wildcard));
    }
    if line.text == "none" {
        return Ok(Some(MatchPattern::None));
    }
    for (name, constructor) in [
        ("some", MatchPattern::Some as fn(String) -> MatchPattern),
        ("ok", MatchPattern::Ok as fn(String) -> MatchPattern),
        ("err", MatchPattern::Err as fn(String) -> MatchPattern),
    ] {
        if line
            .text
            .strip_prefix(name)
            .is_some_and(|rest| rest.starts_with('('))
        {
            let (actual, binding) = parse_local_signature(&line.text, line)?;
            if actual != name || binding.contains(',') || binding.trim().is_empty() {
                return Err(error(
                    "E060",
                    line,
                    format!("{name} pattern binds one value"),
                ));
            }
            return Ok(Some(constructor(identifier(binding.trim(), line)?)));
        }
    }
    let Some((enum_name, rest)) = line.text.rsplit_once('.') else {
        return Ok(None);
    };
    if !enum_name
        .rsplit("::")
        .next()
        .and_then(|name| name.chars().next())
        .is_some_and(char::is_uppercase)
    {
        return Ok(None);
    }
    let enum_name = line.qualify(&qualified_identifier(enum_name, line)?);
    let (variant, binding) = if rest.contains('(') {
        let (variant, binding) = parse_local_signature(rest, line)?;
        if binding.contains(',') || binding.trim().is_empty() {
            return Err(error("E060", line, "enum payload pattern binds one value"));
        }
        (variant, Some(identifier(binding.trim(), line)?))
    } else {
        (identifier(rest, line)?, None)
    };
    Ok(Some(MatchPattern::Enum {
        enum_name,
        variant,
        binding,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use stats_alloc::{INSTRUMENTED_SYSTEM, Region};

    fn line(text: &str) -> Line {
        line_tree(text, &[None], Rc::default())
            .unwrap()
            .pop()
            .unwrap()
    }

    #[test]
    fn typed_match_pattern_requires_the_opening_delimiter() {
        assert!(matches!(
            parse_match_pattern(&line("some(value)")).unwrap(),
            Some(MatchPattern::Some(binding)) if binding == "value"
        ));
        assert!(
            parse_match_pattern(&line("somewhere(value)"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn literal_match_pattern_scan_does_not_allocate() {
        const CALLS: usize = 4_000;
        let line = line("literal");
        let region = Region::new(&INSTRUMENTED_SYSTEM);

        for _ in 0..CALLS {
            assert!(
                std::hint::black_box(parse_match_pattern(std::hint::black_box(&line)))
                    .unwrap()
                    .is_none()
            );
        }
        let stats = region.change();

        eprintln!(
            "{CALLS} literal pattern scans: {} allocations / {} reallocations / {} allocated bytes",
            stats.allocations, stats.reallocations, stats.bytes_allocated
        );
        assert_eq!(stats.allocations, 0, "{stats:?}");
        assert_eq!(stats.reallocations, 0, "{stats:?}");
        assert_eq!(stats.bytes_allocated, 0, "{stats:?}");
    }
}
