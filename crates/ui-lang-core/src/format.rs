use crate::{Error, parse};

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
        output.push_str(&"  ".repeat(indents.len() - 1));
        output.push_str(text);
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn collapses_repeated_blank_lines_and_formats_indentation_idempotently() {
        let source = "app Demo\n\n\ntheme\n    bg #000000\nview\n    box w=fill p=8.0\n        text \"Hello\"\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\n\ntheme\n  bg #000000\nview\n  box w=fill p=8.0\n    text \"Hello\"\n"
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
        let source = "app Demo\nrecipe action for button\n    p-4 bg-primary\nrecipe danger for button extends action\n    bg-danger\ncomponent Badge(label:str=\"Untitled\", selected:bool=false)\n    text label\nview\n    Badge\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\nrecipe action for button\n  p-4 bg-primary\nrecipe danger for button extends action\n  bg-danger\ncomponent Badge(label:str=\"Untitled\", selected:bool=false)\n  text label\nview\n  Badge\n"
        );
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
    fn formats_named_component_event_blocks() {
        let source = "app Demo\ncomponent Dialog()\n    emits\n        confirm\n        select(str, bool)\n    button \"Confirm\" -> emit confirm\non confirmed\non selected(value, active)\nview\n    Dialog\n        events\n            confirm -> confirmed\n            select -> selected _ _\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\ncomponent Dialog()\n  emits\n    confirm\n    select(str, bool)\n  button \"Confirm\" -> emit confirm\non confirmed\non selected(value, active)\nview\n  Dialog\n    events\n      confirm -> confirmed\n      select -> selected _ _\n"
        );
    }

    #[test]
    fn formats_component_lifetime_and_replace() {
        let source = "app Demo\nextern crate::backend\n    fetch() -> str\ncomponent Search()\n    lifetime mounted\n    on search\n        run replace fetch() -> loaded _\n    button \"Search\" -> search\non loaded(value)\nview\n    Search\n";
        let formatted = format_source(source).unwrap();
        assert_eq!(
            formatted,
            "app Demo\nextern crate::backend\n  fetch() -> str\ncomponent Search()\n  lifetime mounted\n  on search\n    run replace fetch() -> loaded _\n  button \"Search\" -> search\non loaded(value)\nview\n  Search\n"
        );
    }
}
