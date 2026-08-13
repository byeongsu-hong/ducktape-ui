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

fn editor_ignored_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with("//")
}

pub fn editor_indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

pub fn editor_first_word(line: &str) -> Option<&str> {
    if editor_ignored_line(line) {
        return None;
    }
    line.trim().split_ascii_whitespace().next()
}

pub fn editor_ancestor_lines<'a>(lines: &'a [&str], line: usize) -> Vec<(usize, &'a str)> {
    let indent = lines.get(line).map_or(0, |line| editor_indentation(line));
    let mut limit = indent;
    let mut ancestors = Vec::new();
    for (index, candidate) in lines[..line.min(lines.len())].iter().enumerate().rev() {
        if editor_ignored_line(candidate) || editor_indentation(candidate) >= limit {
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
        if !editor_ignored_line(candidate) && editor_indentation(candidate) <= indent {
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
    let lines = source
        .split('\n')
        .take(position.line.saturating_add(1))
        .collect::<Vec<_>>();
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

    fn source_line(source: &str, needle: &str) -> usize {
        source
            .lines()
            .position(|line| line.trim() == needle)
            .unwrap_or_else(|| panic!("missing source line `{needle}`"))
    }

    fn context_signature(source: &str, context: CursorContext) -> String {
        match context {
            CursorContext::TopLevel => "top-level".into(),
            CursorContext::HandlerBody => "handler-body".into(),
            CursorContext::ViewNode => "view-node".into(),
            CursorContext::NodeMetadata { node } => format!("metadata:{node:?}"),
            CursorContext::ComponentCall { component } => format!("component:{component}"),
            CursorContext::ComponentEvents {
                component,
                forwarding,
            } => format!("events:{component}:{forwarding}"),
            CursorContext::MatchArms { match_line } => format!(
                "match:{}",
                source.lines().nth(match_line).unwrap_or_default().trim()
            ),
            CursorContext::PaletteValue { contract } => format!("palette:{contract}"),
            CursorContext::StyleStatus { target } => format!("status:{target:?}"),
            CursorContext::ThemeContract => "theme-contract".into(),
            CursorContext::TestBody => "test-body".into(),
        }
    }

    fn context_on(source: &str, needle: &str, document: Option<&Document>) -> CursorContext {
        let line = source_line(source, needle);
        cursor_context(
            source,
            SourcePosition {
                line,
                column: source.lines().nth(line).unwrap_or_default().chars().count(),
            },
            document,
        )
    }

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

    #[test]
    fn ignores_comments_at_any_indentation_like_the_parser() {
        let source = "app Demo\ncomponent Dialog(title:str)\n  emits\n    close\n  text title\nview\n  Dialog\n// parser ignores this line\n    with\n// regardless of its indentation\n      title=\"Demo\"\n// and this one\n    events\n// too\n      close -> closed\non closed\n  exit\n";
        assert!(crate::parse(source).is_ok());
        assert_eq!(
            context_on(source, "title=\"Demo\"", None),
            CursorContext::NodeMetadata {
                node: Some("Dialog".into())
            }
        );
        assert_eq!(
            context_on(source, "close -> closed", None),
            CursorContext::ComponentEvents {
                component: "Dialog".into(),
                forwarding: false,
            }
        );
        assert_eq!(context_on(source, "exit", None), CursorContext::HandlerBody);

        let lines = source.lines().collect::<Vec<_>>();
        let view = source_line(source, "view");
        let handler = source_line(source, "on closed");
        assert_eq!(editor_block_end(&lines, view, 0), handler);
        assert_eq!(editor_first_word("// comment"), None);
    }

    #[test]
    fn parsed_spans_and_editor_context_agree_before_and_after_formatting() {
        fn assert_contexts(source: &str) {
            let document = crate::parse(source).unwrap();
            let handler_statement = document.handlers[0].statements[0].span().line - 1;
            let test_step = document.tests[0].steps[0].span.line - 1;
            let component_root = document.components[0].root.span().line - 1;
            let view_root = document.view.span().line - 1;

            assert_eq!(
                cursor_context(
                    source,
                    SourcePosition {
                        line: handler_statement,
                        column: 0,
                    },
                    Some(&document),
                ),
                CursorContext::HandlerBody
            );
            assert_eq!(
                cursor_context(
                    source,
                    SourcePosition {
                        line: test_step,
                        column: 0,
                    },
                    Some(&document),
                ),
                CursorContext::TestBody
            );
            assert_eq!(
                cursor_context(
                    source,
                    SourcePosition {
                        line: component_root,
                        column: 0,
                    },
                    Some(&document),
                ),
                CursorContext::ViewNode
            );
            assert_eq!(
                cursor_context(
                    source,
                    SourcePosition {
                        line: view_root,
                        column: 0,
                    },
                    Some(&document),
                ),
                CursorContext::ComponentCall {
                    component: "Card".into(),
                }
            );
        }

        let source = "app ContextFixture\nstate\n    count = 0\ncomponent Card(title:str)\n    text title\non increment\n    count = count + 1\nview\n    Card title=\"Ready\"\ntest increments\n    dispatch increment\n";
        assert_contexts(source);
        assert_contexts(&crate::format_source(source).unwrap());
    }

    #[test]
    fn comment_insertion_preserves_incomplete_source_contexts() {
        let source = "component Shell()\n  on submit\n    run \n  ui::Dialog\n    with\n      title=\n    events\n      close\nview\n  match request\n    pa\ntest interaction\n  click #submit\n";
        let anchors = [
            "run",
            "title=",
            "close",
            "ui::Dialog",
            "pa",
            "click #submit",
        ];
        let expected = anchors
            .iter()
            .map(|anchor| {
                let context = context_on(source, anchor, None);
                (*anchor, context_signature(source, context))
            })
            .collect::<Vec<_>>();
        let lines = source.lines().collect::<Vec<_>>();

        for insertion in 0..=lines.len() {
            let mut mutated = lines.clone();
            mutated.insert(insertion, "// mutation");
            let mutated = format!("{}\n", mutated.join("\n"));
            for (anchor, expected) in &expected {
                let context = context_on(&mutated, anchor, None);
                assert_eq!(
                    context_signature(&mutated, context),
                    *expected,
                    "context changed after inserting a comment at line {insertion}"
                );
            }
        }
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_cursor_context_collects_only_the_source_prefix() {
        use stats_alloc::{INSTRUMENTED_SYSTEM, Region};

        const TRAILING_LINES: usize = 4_000;
        let mut source = String::from("component Shell()\n  ui::Dialog\n    with\n      title=\n");
        for _ in 0..TRAILING_LINES {
            source.push_str("view\n");
        }
        let region = Region::new(&INSTRUMENTED_SYSTEM);

        let context = cursor_context(
            std::hint::black_box(&source),
            SourcePosition {
                line: 3,
                column: 12,
            },
            None,
        );
        let stats = region.change();

        assert_eq!(
            context,
            CursorContext::NodeMetadata {
                node: Some("ui::Dialog".into())
            }
        );
        eprintln!(
            "cursor context before {TRAILING_LINES} trailing lines: {} allocations / {} reallocations / {} bytes",
            stats.allocations, stats.reallocations, stats.bytes_allocated
        );
        assert_eq!(stats.reallocations, 0, "{stats:?}");
    }

    #[test]
    fn incomplete_source_mutations_never_panic() {
        let source = "component Shell()\n  on submit\n    run \n  ui::Dialog\n    with\n      title=\n    events\n      close\nview\n  match request\n    \ntest interaction\n  click #submit\n";
        let fragments = [
            "",
            "// mutation",
            "  // mutation",
            "  ui::",
            "    with",
            "      title=",
            "  on",
            "    match",
        ];
        let base = source.lines().collect::<Vec<_>>();
        let mut state = 0x4d59_5df4_usize;

        for _ in 0..512 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let mut lines = base.clone();
            let index = state % lines.len();
            if state & 1 == 0 {
                lines.remove(index);
            } else {
                let fragment = fragments[(state >> 8) % fragments.len()];
                lines.insert(index, fragment);
            }
            let mutated = lines.join("\n");
            for line in (0..=lines.len()).chain([usize::MAX]) {
                for column in [0, lines.get(line).map_or(0, |line| line.len()), usize::MAX] {
                    let _ = cursor_context(&mutated, SourcePosition { line, column }, None);
                }
            }
        }
    }
}
