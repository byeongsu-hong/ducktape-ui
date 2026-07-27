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
}
