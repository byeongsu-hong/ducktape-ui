use crate::parser::{
    split_style_utilities_for_format as split_style_utilities,
    split_top_marker_for_format as split_top_marker, split_words_for_format as split_words,
};
use crate::{Error, parse};

const MAX_INLINE_NODE: usize = 100;

pub fn format_source(source: &str) -> Result<String, Error> {
    parse(source)?;
    Ok(format_fragment(source))
}

pub fn format_fragment(source: &str) -> String {
    let mut output = String::new();
    let mut indents = vec![0usize];
    let mut blank = false;

    for raw in source.lines() {
        let text = raw.trim();
        if text.is_empty() {
            blank = !output.is_empty();
            continue;
        }
        if blank && !output.ends_with("\n\n") {
            output.push('\n');
        }
        blank = false;

        let indent_bytes = raw.len() - raw.trim_start().len();
        let indent = raw[..indent_bytes].chars().count();
        while indents.last().is_some_and(|current| indent < *current) {
            indents.pop();
        }
        if indent > *indents.last().unwrap_or(&0) {
            indents.push(indent);
        }
        for _ in 1..indents.len() {
            output.push_str("  ");
        }
        output.push_str(text);
        output.push('\n');
    }
    rewrap_node_metadata(&reorder_component_metadata(&output))
}

fn reorder_component_metadata(source: &str) -> String {
    let mut lines = source.lines().collect::<Vec<_>>();
    for index in (0..lines.len()).rev() {
        let indent = lines[index].len() - lines[index].trim_start().len();
        let Some(head) = lines[index].split_ascii_whitespace().next() else {
            continue;
        };
        let component = head.rsplit("::").next().is_some_and(|name| {
            name.split('.')
                .all(|part| part.chars().next().is_some_and(char::is_uppercase))
        });
        if !component {
            continue;
        }
        let end = format_block_end(&lines, index, indent);
        let mut ranges = Vec::new();
        let mut child = index + 1;
        while child < end {
            let child_indent = lines[child].len() - lines[child].trim_start().len();
            if lines[child].trim().is_empty() || child_indent != indent + 2 {
                child += 1;
                continue;
            }
            let child_end = format_block_end(&lines, child, child_indent).min(end);
            ranges.push((child, child_end));
            child = child_end;
        }
        if ranges.is_empty() {
            continue;
        }
        let mut ordered = ranges.clone();
        ordered.sort_by_key(|(start, _)| match lines[*start].trim() {
            "with" => 0,
            "events" => 1,
            "forward" => 2,
            _ => 3,
        });
        if ordered == ranges {
            continue;
        }
        let children = ordered
            .into_iter()
            .flat_map(|(start, end)| lines[start..end].iter().copied())
            .collect::<Vec<_>>();
        lines.splice(index + 1..end, children);
    }
    let mut output = lines.join("\n");
    if source.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn format_block_end(lines: &[&str], line: usize, indent: usize) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(line + 1)
        .find(|(_, candidate)| {
            !candidate.trim().is_empty() && candidate.len() - candidate.trim_start().len() <= indent
        })
        .map_or(lines.len(), |(index, _)| index)
}

fn rewrap_node_metadata(source: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let indent = line.len() - line.trim_start().len();
        let restricted_context = parent_is_status(&lines, index, indent)
            || inside_block(&lines, index, indent, "canvas")
            || (line.split_ascii_whitespace().next() == Some("col")
                && parent_is(&lines, index, indent, "table"));
        if lines.get(index + 1).is_some_and(|next| {
            next.len() - next.trim_start().len() == indent + 2 && next.trim() == "with"
        }) {
            let mut end = index + 2;
            while lines.get(end).is_some_and(|entry| {
                !entry.trim().is_empty() && entry.len() - entry.trim_start().len() == indent + 4
            }) {
                end += 1;
            }
            let metadata = lines[index + 2..end]
                .iter()
                .flat_map(|entry| {
                    let entry = entry.trim();
                    entry.strip_prefix('@').map_or_else(
                        || vec![entry.to_owned()],
                        |styles| {
                            styles
                                .split_ascii_whitespace()
                                .map(|style| format!("@{}", style.trim_start_matches('@')))
                                .collect()
                        },
                    )
                })
                .collect::<Vec<_>>();
            let metadata = metadata.iter().map(String::as_str).collect::<Vec<_>>();
            let merged = merge_metadata(line.trim(), &metadata);
            if interleaved_metadata(&merged) {
                for original in &lines[index..end] {
                    output.push_str(original);
                    output.push('\n');
                }
                index = end;
                continue;
            }
            if let Some((parent, metadata)) = split_metadata(&merged)
                && indent > 0
                && !restricted_context
                && (metadata.len() > 2 || indent + merged.chars().count() > MAX_INLINE_NODE)
            {
                output.push_str(&" ".repeat(indent));
                output.push_str(&parent);
                output.push('\n');
                output.push_str(&" ".repeat(indent + 2));
                output.push_str("with\n");
                for entry in metadata {
                    output.push_str(&" ".repeat(indent + 4));
                    output.push_str(&entry);
                    output.push('\n');
                }
            } else {
                output.push_str(&" ".repeat(indent));
                output.push_str(&merged);
                output.push('\n');
            }
            index = end;
            continue;
        }
        if indent > 0
            && !restricted_context
            && let Some((parent, metadata)) = split_metadata(line.trim())
            && (metadata.len() > 2 || indent + line.trim().chars().count() > MAX_INLINE_NODE)
        {
            output.push_str(&" ".repeat(indent));
            output.push_str(&parent);
            output.push('\n');
            output.push_str(&" ".repeat(indent + 2));
            output.push_str("with\n");
            for entry in metadata {
                output.push_str(&" ".repeat(indent + 4));
                output.push_str(&entry);
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
        index += 1;
    }
    output
}

fn merge_metadata(parent: &str, metadata: &[&str]) -> String {
    let (node, route) = split_top_marker(parent, "->").map_or((parent, None), |(node, route)| {
        (node.trim(), Some(route.trim()))
    });
    let (core, existing) = split_style_utilities(node);
    let mut merged = core.to_owned();
    let mut utilities = existing;
    for entry in metadata {
        if let Some(styles) = entry.strip_prefix('@') {
            utilities.extend(
                styles
                    .split_ascii_whitespace()
                    .map(|style| style.trim_start_matches('@').to_owned()),
            );
        } else {
            merged.push(' ');
            merged.push_str(entry);
        }
    }
    if !utilities.is_empty() {
        merged.push_str(" @");
        merged.push_str(&utilities.join(" "));
    }
    if let Some(route) = route {
        merged.push_str(" -> ");
        merged.push_str(route);
    }
    merged
}

fn split_metadata(source: &str) -> Option<(String, Vec<String>)> {
    let (node, route) = split_top_marker(source, "->").map_or((source, None), |(node, route)| {
        (node.trim(), Some(route.trim()))
    });
    let (core, utilities) = split_style_utilities(node);
    let parts = split_words(core);
    let head = parts.first()?;
    if !wrappable_node(source) || interleaved_metadata(source) {
        return None;
    }
    let component = head.chars().next().is_some_and(char::is_uppercase);
    let mut inline = vec![head.clone()];
    let mut metadata = Vec::new();
    for part in &parts[1..] {
        if metadata_part(head, component, part) {
            metadata.push(part.to_string());
        } else {
            inline.push(part.to_string());
        }
    }
    metadata.extend(utilities.into_iter().map(|utility| format!("@{utility}")));
    if metadata.is_empty() {
        return None;
    }
    let mut parent = inline.join(" ");
    if let Some(route) = route {
        parent.push_str(" -> ");
        parent.push_str(route);
    }
    Some((parent, metadata))
}

fn metadata_part(head: &str, component: bool, part: &str) -> bool {
    (property(part) && !(head == "keyed" && part.starts_with("by=")))
        || (component && crate::valid_identifier(part))
}

/// A line whose metadata-shaped words interleave with other words — such as a
/// parser-rejected `disabled=a || b` — cannot be rewrapped without tearing the
/// words apart, so the formatter must preserve the region as written. Binding
/// words like `draft<->input_draft` stay inline by design and can never
/// continue a spaced expression, so metadata may hop over them.
fn interleaved_metadata(source: &str) -> bool {
    let node = split_top_marker(source, "->").map_or(source, |(node, _)| node);
    let (core, _) = split_style_utilities(node);
    let parts = split_words(core);
    let Some(head) = parts.first() else {
        return false;
    };
    let component = head.chars().next().is_some_and(char::is_uppercase);
    let mut seen_metadata = false;
    for part in &parts[1..] {
        if metadata_part(head, component, part) {
            seen_metadata = true;
        } else if seen_metadata && !part.contains("<->") {
            return true;
        }
    }
    false
}

fn wrappable_node(source: &str) -> bool {
    source
        .split_ascii_whitespace()
        .next()
        .is_some_and(|head| head.chars().next().is_some_and(char::is_uppercase) || is_node(head))
}

fn property(part: &str) -> bool {
    part.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    })
}

fn is_node(name: &str) -> bool {
    matches!(
        name,
        "col"
            | "row"
            | "flex"
            | "scroll"
            | "grid"
            | "stack"
            | "box"
            | "text"
            | "input"
            | "button"
            | "checkbox"
            | "toggler"
            | "slider"
            | "progress"
            | "radio"
            | "pick"
            | "combo"
            | "rule"
            | "qr"
            | "space"
            | "float"
            | "pin"
            | "sensor"
            | "responsive"
            | "image"
            | "svg"
            | "viewer"
            | "tooltip"
            | "mouse"
            | "hover"
            | "resize-handle"
            | "canvas"
            | "theme"
            | "keyed"
            | "lazy"
            | "markdown"
            | "editor"
            | "table"
            | "overlay"
            | "rich-text"
    )
}

fn parent_is_status(lines: &[&str], index: usize, indent: usize) -> bool {
    lines[..index]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty() && line.len() - line.trim_start().len() < indent)
        .and_then(|line| line.split_ascii_whitespace().next())
        .is_some_and(|name| {
            matches!(
                name,
                "active"
                    | "hovered"
                    | "dragged"
                    | "focused"
                    | "focused-hovered"
                    | "pressed"
                    | "disabled"
                    | "opened"
                    | "opened-hovered"
                    | "selected"
                    | "rail"
                    | "scroller"
                    | "x-rail"
                    | "x-scroller"
                    | "y-rail"
                    | "y-scroller"
            )
        })
}

fn parent_is(lines: &[&str], index: usize, indent: usize, parent: &str) -> bool {
    lines[..index]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty() && line.len() - line.trim_start().len() < indent)
        .is_some_and(|line| line.split_ascii_whitespace().next() == Some(parent))
}

fn inside_block(lines: &[&str], index: usize, indent: usize, block: &str) -> bool {
    let mut child_indent = indent;
    for line in lines[..index].iter().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let line_indent = line.len() - line.trim_start().len();
        if line_indent >= child_indent {
            continue;
        }
        if line.split_ascii_whitespace().next() == Some(block) {
            return true;
        }
        child_indent = line_indent;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{format_fragment, format_source};

    #[test]
    fn collapses_repeated_blank_lines_and_formats_indentation_idempotently() {
        let source = "app Demo\n\n\ntheme contract AppTheme\n    bg\npalette app for AppTheme\n    bg #000000\nview\n    box w=fill p=8.0\n        text \"Hello\"\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\n\ntheme contract AppTheme\n  bg\npalette app for AppTheme\n  bg #000000\nview\n  box w=fill p=8.0\n    text \"Hello\"\n"
        );
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn formats_first_class_test_blocks() {
        let source = "app Demo\nview\n    col #root\n        text \"ok\" #text\ntest layout\n    viewport 320 240\n    target root = #root\n    target text = root/text\n    expect root.width ~= 320.0\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\nview\n  col #root\n    text \"ok\" #text\ntest layout\n  viewport 320 240\n  target root = #root\n  target text = root/text\n  expect root.width ~= 320.0\n"
        );
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn preserves_explicit_component_bind_syntax() {
        let source = "app Demo\nstate\n    draft = \"\"\ncomponent Field(bind value:str)\n    input \"Value\" <-> value\nview\n    Field value<->draft\n";
        assert_eq!(
            format_source(source).unwrap(),
            "app Demo\nstate\n  draft = \"\"\ncomponent Field(bind value:str)\n  input \"Value\" <-> value\nview\n  Field value<->draft\n"
        );
    }

    #[test]
    fn formats_recipe_inheritance_and_component_defaults() {
        let source = "app Demo\nrecipe action for button\n    @p-4 bg-primary\nrecipe danger for button extends action\n    @bg-danger\ncomponent Badge(label:str=\"Untitled\", selected:bool=false)\n    text label\nview\n    Badge\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\nrecipe action for button\n  @p-4 bg-primary\nrecipe danger for button extends action\n  @bg-danger\ncomponent Badge(label:str=\"Untitled\", selected:bool=false)\n  text label\nview\n  Badge\n"
        );
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn formats_multiline_with_metadata() {
        let source = "app Demo\nstate\n    draft = \"\"\nview\n    input \"Draft\" <-> draft\n        with\n            hint=\"Write\"\n            disabled=false\n            @p-4 bg-surface\n        active border=primary\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\nstate\n  draft = \"\"\nview\n  input \"Draft\" <-> draft\n    with\n      hint=\"Write\"\n      disabled=false\n      @p-4\n      @bg-surface\n    active border=primary\n"
        );
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn keeps_two_short_metadata_entries_inline() {
        let source = "app Demo\non saved\nview\n    button \"Save\" -> saved\n        with\n            disabled=false\n            @px-4\n";
        assert_eq!(
            format_source(source).unwrap(),
            "app Demo\non saved\nview\n  button \"Save\" disabled=false @px-4 -> saved\n"
        );
    }

    #[test]
    fn keeps_bindings_on_the_node_line_and_orders_component_metadata() {
        let source = "app Demo\nstate\n  title = \"Draft\"\ncomponent Card(bind title:str)\n  emits\n    save\n  col\n    slot Body\n    button \"Save\" -> emit(save)\non saved\nview\n  Card title<->title tone=\"quiet\" elevated=false rounded=true\n    Body:\n      text title\n    events\n      save -> saved\n    with\n      shadowed=false\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains(
            "  Card title<->title\n    with\n      tone=\"quiet\"\n      elevated=false\n      rounded=true\n      shadowed=false\n    events\n      save -> saved\n    Body:"
        ));
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn formats_derived_values_and_handler_locals() {
        let source = "app Demo\nstate\n    draft = \"\"\nderived\n    normalized = trim(draft)\non submit\n    let title = normalized\n    draft = title\nview\n    text normalized\n";
        assert_eq!(
            format_source(source).unwrap(),
            "app Demo\nstate\n  draft = \"\"\nderived\n  normalized = trim(draft)\non submit\n  let title = normalized\n  draft = title\nview\n  text normalized\n"
        );
    }

    #[test]
    fn expands_long_node_metadata_into_with() {
        let source = "  Sidebar #sidebar selected_page=selected_page home_favorite=home_favorite roadmap_favorite=roadmap_favorite launch_favorite=launch_favorite\n";
        assert_eq!(
            format_fragment(source),
            "  Sidebar #sidebar\n    with\n      selected_page=selected_page\n      home_favorite=home_favorite\n      roadmap_favorite=roadmap_favorite\n      launch_favorite=launch_favorite\n"
        );
    }

    #[test]
    fn formats_named_component_event_blocks() {
        let source = "app Demo\ncomponent Dialog()\n    emits\n        confirm\n        select(str, bool)\n    button \"Confirm\" -> emit(confirm)\non confirmed\non selected(value, active)\nview\n    Dialog\n        events\n            confirm -> confirmed\n            select -> selected _ _\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\ncomponent Dialog()\n  emits\n    confirm\n    select(str, bool)\n  button \"Confirm\" -> emit(confirm)\non confirmed\non selected(value, active)\nview\n  Dialog\n    events\n      confirm -> confirmed\n      select -> selected _ _\n"
        );
    }

    #[test]
    fn formats_component_lifetime_replace_and_lane_invalidation() {
        let source = "app Demo\nextern crate::backend\n    fetch() -> str\ncomponent Search()\n    lifetime mounted\n    on cancel\n        invalidate lane=search\n    on search\n        run replace lane=search fetch() -> loaded _\n    col\n        button \"Search\" -> search\n        button \"Cancel\" -> cancel\non loaded(value)\nview\n    Search\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\nextern crate::backend\n  fetch() -> str\ncomponent Search()\n  lifetime mounted\n  on cancel\n    invalidate lane=search\n  on search\n    run replace lane=search fetch() -> loaded _\n  col\n    button \"Search\" -> search\n    button \"Cancel\" -> cancel\non loaded(value)\nview\n  Search\n"
        );
    }

    #[test]
    fn keeps_table_columns_and_dragged_scroll_styles_as_leaves() {
        let source = r#"app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  width_with_an_intentionally_very_long_but_valid_identifier_to_force_formatter_wrapping:f64 = 120.0
view
  col
    scroll
      col
        text "Content"
      dragged y-dragged=true
        box bg=bg text=fg border=primary border-w=1.0 r=8.0 shadow=black/25 shadow-y=2.0 shadow-blur=4.0 px-snap=true
    table item in []
      col w=width_with_an_intentionally_very_long_but_valid_identifier_to_force_formatter_wrapping align-x=center align-y=center
        header
          text "Header"
        cell
          text "Cell"
"#;
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("        box bg=bg text=fg border=primary"));
        assert!(formatted.contains("      col w=width_with_an_intentionally"));
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }
}
