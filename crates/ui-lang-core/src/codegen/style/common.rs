use super::*;

pub(in crate::codegen) fn append_container_utility_overrides(
    code: &mut String,
    style: &ResolvedStyle,
) {
    if let Some(background) = &style.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({}.into());",
            resolved_theme_color(background)
        )
        .unwrap();
    }
    if let Some(text) = &style.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(text)
        )
        .unwrap();
    }
    if let Some(border) = &style.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(border)
        )
        .unwrap();
    }
    if style.border_width != 0 {
        write!(code, " __style.border.width = {}.0;", style.border_width).unwrap();
    }
    if style.radius != 0 {
        write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
    }
}
