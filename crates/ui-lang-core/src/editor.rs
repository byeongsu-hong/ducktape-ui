use crate::Document;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CursorContext {
    TopLevel,
    HandlerBody,
    ViewNode,
    NodeMetadata { node: Option<String> },
    ComponentCall { component: String },
    ComponentEvents { component: String, forwarding: bool },
    MatchArms { match_line: usize },
    PaletteValue { contract: String },
    StyleStatus { target: Option<String> },
    ThemeContract,
    TestBody,
}

pub const STYLE_STATUS_NAMES: &[&str] = &[
    "active",
    "hovered",
    "pressed",
    "disabled",
    "focused",
    "focused-hovered",
    "opened",
    "opened-hovered",
    "dragged",
];

pub fn editor_indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

pub fn editor_first_word(line: &str) -> Option<&str> {
    line.trim().split_ascii_whitespace().next()
}

pub fn editor_ancestor_lines<'a>(lines: &'a [&str], line: usize) -> Vec<(usize, &'a str)> {
    let indent = lines.get(line).map_or(0, |line| editor_indentation(line));
    let mut limit = indent;
    let mut ancestors = Vec::new();
    for (index, candidate) in lines[..line.min(lines.len())].iter().enumerate().rev() {
        if candidate.trim().is_empty() || editor_indentation(candidate) >= limit {
            continue;
        }
        limit = editor_indentation(candidate);
        ancestors.push((index, *candidate));
        if limit == 0 {
            break;
        }
    }
    ancestors
}

pub fn editor_block_end(lines: &[&str], line: usize, indent: usize) -> usize {
    for (index, candidate) in lines.iter().enumerate().skip(line + 1) {
        if !candidate.trim().is_empty() && editor_indentation(candidate) <= indent {
            return index;
        }
    }
    lines.len()
}

fn declared_component(name: &str, document: Option<&Document>) -> bool {
    document.map_or_else(
        || {
            name.rsplit("::")
                .next()
                .and_then(|name| name.split('.').next())
                .and_then(|name| name.chars().next())
                .is_some_and(char::is_uppercase)
        },
        |document| {
            document
                .components
                .iter()
                .any(|component| component.name == name)
        },
    )
}

pub fn editor_component_name(line: &str, document: Option<&Document>) -> Option<String> {
    let name = editor_first_word(line)?;
    declared_component(name, document).then(|| name.to_owned())
}

pub fn cursor_context(
    source: &str,
    position: SourcePosition,
    document: Option<&Document>,
) -> CursorContext {
    let lines = source.split('\n').collect::<Vec<_>>();
    let current = lines.get(position.line).copied().unwrap_or("");
    let indent = editor_indentation(current);
    let trimmed = current.trim();
    let declared_palette = trimmed
        .split_once('=')
        .and_then(|(left, _)| left.split_once(":palette[").map(|(_, ty)| ty))
        .and_then(|ty| ty.strip_suffix(']'))
        .map(str::to_owned);
    let state_palette = trimmed
        .split_once('=')
        .map(|(left, _)| left.split(':').next().unwrap_or(left).trim())
        .and_then(|name| {
            document?.states.iter().find_map(|state| {
                (state.name == name)
                    .then_some(&state.ty)
                    .and_then(|ty| match ty {
                        crate::Type::Palette(contract) => Some(contract.clone()),
                        _ => None,
                    })
            })
        });
    if let Some(contract) = declared_palette.or(state_palette) {
        return CursorContext::PaletteValue { contract };
    }
    if indent > 0
        && trimmed.starts_with("palette ")
        && let Some(contract) = document
            .and_then(|document| document.theme_contract.as_ref())
            .map(|contract| contract.name.clone())
    {
        return CursorContext::PaletteValue { contract };
    }
    if indent == 0 && !current.trim().is_empty() {
        return CursorContext::TopLevel;
    }
    let ancestors = editor_ancestor_lines(&lines, position.line);
    if ancestors
        .iter()
        .any(|(_, line)| editor_first_word(line) == Some("on"))
    {
        return CursorContext::HandlerBody;
    }
    if ancestors
        .iter()
        .any(|(_, line)| editor_first_word(line) == Some("test"))
    {
        return CursorContext::TestBody;
    }
    if ancestors
        .iter()
        .any(|(_, line)| line.trim_start().starts_with("theme contract "))
    {
        return CursorContext::ThemeContract;
    }
    if editor_first_word(current).is_some_and(|word| STYLE_STATUS_NAMES.contains(&word)) {
        return CursorContext::StyleStatus {
            target: ancestors
                .first()
                .and_then(|(_, line)| editor_first_word(line))
                .map(str::to_owned),
        };
    }
    if ancestors
        .first()
        .and_then(|(_, line)| editor_first_word(line))
        .is_some_and(|word| STYLE_STATUS_NAMES.contains(&word))
    {
        return CursorContext::StyleStatus {
            target: ancestors
                .get(1)
                .and_then(|(_, line)| editor_first_word(line))
                .map(str::to_owned),
        };
    }
    if let Some((match_line, parent)) = ancestors
        .iter()
        .find(|(_, candidate)| editor_first_word(candidate) == Some("match"))
        && indent == editor_indentation(parent) + 2
    {
        return CursorContext::MatchArms {
            match_line: *match_line,
        };
    }
    if let Some(name) = editor_component_name(current, document) {
        return CursorContext::ComponentCall { component: name };
    }
    if let Some((_, metadata)) = ancestors.first()
        && matches!(metadata.trim(), "events" | "forward")
        && let Some(component) = ancestors
            .get(1)
            .and_then(|(_, line)| editor_component_name(line, document))
    {
        return CursorContext::ComponentEvents {
            component,
            forwarding: metadata.trim() == "forward",
        };
    }
    if let Some((_, metadata)) = ancestors.first()
        && metadata.trim() == "with"
    {
        return CursorContext::NodeMetadata {
            node: ancestors
                .get(1)
                .and_then(|(_, line)| editor_first_word(line))
                .map(str::to_owned),
        };
    }
    for (_, ancestor) in &ancestors {
        if let Some(component) = editor_component_name(ancestor, document) {
            return CursorContext::ComponentCall { component };
        }
        if !matches!(ancestor.trim(), "events" | "forward" | "with") {
            break;
        }
    }
    if indent == 0 {
        CursorContext::TopLevel
    } else {
        CursorContext::ViewNode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_incomplete_nested_source_without_parsing_it() {
        let source = "component Shell()\n  ui::Dialog\n    with\n      title=\n    events\n      \nview\n  match request\n    \n";
        assert_eq!(
            cursor_context(
                source,
                SourcePosition {
                    line: 3,
                    column: 12
                },
                None
            ),
            CursorContext::NodeMetadata {
                node: Some("ui::Dialog".into())
            }
        );
        assert_eq!(
            cursor_context(source, SourcePosition { line: 5, column: 6 }, None),
            CursorContext::ComponentEvents {
                component: "ui::Dialog".into(),
                forwarding: false,
            }
        );
        assert_eq!(
            cursor_context(source, SourcePosition { line: 8, column: 4 }, None),
            CursorContext::MatchArms { match_line: 7 }
        );
    }

    #[test]
    fn recognizes_component_local_handler_bodies() {
        let source = "component Search()\n  on submit\n    run \n  text \"Search\"\n";
        assert_eq!(
            cursor_context(source, SourcePosition { line: 2, column: 8 }, None),
            CursorContext::HandlerBody
        );
    }
}
