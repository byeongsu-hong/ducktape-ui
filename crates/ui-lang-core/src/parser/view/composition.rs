use super::*;

pub(in crate::parser) fn split_style_utilities<'a>(
    source: &'a str,
    line: &Line,
) -> (&'a str, Vec<String>) {
    split_top_marker(source, "@").map_or_else(
        || (source.trim(), Vec::new()),
        |(core, styles)| {
            (
                core.trim(),
                styles
                    .split_whitespace()
                    .map(|style| line.qualify(style))
                    .collect(),
            )
        },
    )
}

pub(crate) fn split_style_utilities_for_format(source: &str) -> (&str, Vec<String>) {
    split_top_marker(source, "@").map_or_else(
        || (source.trim(), Vec::new()),
        |(core, styles)| {
            (
                core.trim(),
                styles.split_whitespace().map(str::to_owned).collect(),
            )
        },
    )
}

pub(in crate::parser) fn parse_component_children(
    component: &str,
    line: &Line,
) -> Result<(Vec<ComponentSlot>, Vec<ComponentEventRoute>), Error> {
    let mut event_blocks = line.children.iter().filter(|child| child.text == "events");
    let events = event_blocks.next();
    if event_blocks.next().is_some() {
        return Err(error(
            "E040",
            line,
            "component call has duplicate events blocks",
        ));
    }
    let event_routes = events
        .map(|events| {
            events
                .children
                .iter()
                .map(|event| {
                    ensure_leaf(event)?;
                    let Some((name, route)) = split_top_marker(&event.text, "->") else {
                        return Err(error(
                            "E040",
                            event,
                            "component event routes use `name -> handler`",
                        ));
                    };
                    Ok(ComponentEventRoute {
                        name: identifier(name.trim(), event)?,
                        route: parse_route(route.trim(), event)?,
                        span: Span::line(event.number),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()
        })
        .transpose()?
        .unwrap_or_default();
    let children = line
        .children
        .iter()
        .filter(|child| child.text != "events")
        .collect::<Vec<_>>();
    if children.is_empty() {
        return Ok((Vec::new(), event_routes));
    }
    let named = children.iter().any(|child| child.text.ends_with(':'));
    if !named {
        let compound = children
            .iter()
            .map(|child| compound_slot_name(component, child))
            .collect::<Vec<_>>();
        if compound.iter().all(Option::is_some) {
            let slots = children
                .iter()
                .zip(compound)
                .map(|(child, name)| {
                    Ok(ComponentSlot {
                        name: name.expect("all compound slots are present"),
                        content: Box::new(parse_view(child)?),
                        span: Span::line(child.number),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            return Ok((slots, event_routes));
        }
        if compound.iter().any(Option::is_some) {
            return Err(error(
                "E040",
                line,
                "cannot mix compound components with direct component children",
            )
            .hint(format!(
                "use only `{component}.Name` children, or wrap direct children in one layout"
            )));
        }
        let slots = match children.as_slice() {
            [content] => Ok(vec![ComponentSlot {
                name: "children".into(),
                content: Box::new(parse_view(content)?),
                span: Span::line(content.number),
            }]),
            _ => Err(error(
                "E040",
                line,
                "component children need one root or named `slot:` blocks",
            )
            .hint("wrap siblings in row or col, or write `header:` and `body:` blocks")),
        }?;
        return Ok((slots, event_routes));
    }

    let slots = children
        .iter()
        .map(|section| {
            let Some(name) = section.text.strip_suffix(':') else {
                return Err(error(
                    "E040",
                    section,
                    "cannot mix a direct child with named component slots",
                ));
            };
            if section.children.len() != 1 {
                return Err(error(
                    "E040",
                    section,
                    format!("component slot `{}` needs exactly one root", name.trim()),
                ));
            }
            Ok(ComponentSlot {
                name: identifier(name.trim(), section)?,
                content: Box::new(parse_view(&section.children[0])?),
                span: Span::line(section.number),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok((slots, event_routes))
}

pub(in crate::parser) fn compound_slot_name(component: &str, line: &Line) -> Option<String> {
    let head = line.text.split_ascii_whitespace().next()?;
    let name = head.split_once('(').map_or(head, |(name, _)| name);
    let slot = name.strip_prefix(component)?.strip_prefix('.')?;
    (!slot.contains('.'))
        .then(|| identifier(slot, line).ok())
        .flatten()
}
