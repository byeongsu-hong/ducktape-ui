use super::*;

pub(in crate::codegen) fn custom_style_call_code(
    style: &ExternCall,
    kind: ExternKind,
    leading_args: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let function = find_extern_function(document, &style.function, kind)
        .expect("checker validates custom style");
    let args = expr_args_suffix_code(&style.args, env, document)?;
    Ok(format!("{}({leading_args}{args})", function.rust_path))
}

pub(in crate::codegen) fn append_f32_fields<'a>(
    code: &mut String,
    fields: impl IntoIterator<Item = (&'a Option<Expr>, &'a str)>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    for (value, field) in fields {
        if let Some(value) = value {
            write!(
                code,
                " {field} = {} as f32;",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(())
}

pub(in crate::codegen) fn text_shaping_code(shaping: TextShaping) -> &'static str {
    match shaping {
        TextShaping::Auto => "Auto",
        TextShaping::Basic => "Basic",
        TextShaping::Advanced => "Advanced",
    }
}

pub(in crate::codegen) fn text_wrapping_code(wrapping: TextWrapping) -> &'static str {
    match wrapping {
        TextWrapping::None => "None",
        TextWrapping::Word => "Word",
        TextWrapping::Glyph => "Glyph",
        TextWrapping::WordOrGlyph => "WordOrGlyph",
    }
}

pub(in crate::codegen) fn text_line_height_code(
    line_height: &TextLineHeight,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    match line_height {
        TextLineHeight::Relative(value) => Ok(format!(
            "::iced::widget::text::LineHeight::Relative({})",
            clamped_f32_code(value, "f32::EPSILON", "f32::MAX", env, document)?
        )),
        TextLineHeight::Absolute(value) => Ok(format!(
            "::iced::widget::text::LineHeight::Absolute({}.into())",
            clamped_f32_code(value, "f32::EPSILON", "f32::MAX", env, document)?
        )),
    }
}

pub(in crate::codegen) fn font_preset_code(
    font: &FontPreset,
    document: &Document,
) -> Result<String, Error> {
    match font {
        FontPreset::Default => Ok("::iced::Font::DEFAULT".into()),
        FontPreset::Monospace => Ok("::iced::Font::MONOSPACE".into()),
        FontPreset::Named(name) => document
            .fonts
            .iter()
            .find(|font| font.name == *name)
            .map(font_decl_code)
            .ok_or_else(|| Error::new("E171", &Span::line(1), format!("unknown font `{name}`"))),
    }
}

pub(in crate::codegen) fn font_decl_code(font: &FontDecl) -> String {
    let family = match &font.family {
        FontFamily::Named(name) => format!("::iced::font::Family::Name({})", rust_string(name)),
        FontFamily::Serif => "::iced::font::Family::Serif".into(),
        FontFamily::SansSerif => "::iced::font::Family::SansSerif".into(),
        FontFamily::Cursive => "::iced::font::Family::Cursive".into(),
        FontFamily::Fantasy => "::iced::font::Family::Fantasy".into(),
        FontFamily::Monospace => "::iced::font::Family::Monospace".into(),
    };
    let weight = match font.weight {
        FontWeight::Thin => "Thin",
        FontWeight::ExtraLight => "ExtraLight",
        FontWeight::Light => "Light",
        FontWeight::Normal => "Normal",
        FontWeight::Medium => "Medium",
        FontWeight::Semibold => "Semibold",
        FontWeight::Bold => "Bold",
        FontWeight::ExtraBold => "ExtraBold",
        FontWeight::Black => "Black",
    };
    let stretch = match font.stretch {
        FontStretch::UltraCondensed => "UltraCondensed",
        FontStretch::ExtraCondensed => "ExtraCondensed",
        FontStretch::Condensed => "Condensed",
        FontStretch::SemiCondensed => "SemiCondensed",
        FontStretch::Normal => "Normal",
        FontStretch::SemiExpanded => "SemiExpanded",
        FontStretch::Expanded => "Expanded",
        FontStretch::ExtraExpanded => "ExtraExpanded",
        FontStretch::UltraExpanded => "UltraExpanded",
    };
    let style = match font.style {
        FontStyle::Normal => "Normal",
        FontStyle::Italic => "Italic",
        FontStyle::Oblique => "Oblique",
    };
    format!(
        "::iced::Font {{ family: {family}, weight: ::iced::font::Weight::{weight}, stretch: ::iced::font::Stretch::{stretch}, style: ::iced::font::Style::{style} }}"
    )
}

pub(in crate::codegen) fn app_default_font_code(document: &Document) -> String {
    document
        .fonts
        .iter()
        .find(|font| font.default)
        .map(font_decl_code)
        .unwrap_or_else(|| "::iced::Font::DEFAULT".into())
}

pub(in crate::codegen) fn text_alignment_code(alignment: TextAlignment) -> &'static str {
    match alignment {
        TextAlignment::Default => "Default",
        TextAlignment::Left => "Left",
        TextAlignment::Center => "Center",
        TextAlignment::Right => "Right",
        TextAlignment::Justified => "Justified",
    }
}

pub(in crate::codegen) fn mouse_interaction_code(interaction: MouseInteraction) -> &'static str {
    match interaction {
        MouseInteraction::None => "None",
        MouseInteraction::Hidden => "Hidden",
        MouseInteraction::Idle => "Idle",
        MouseInteraction::ContextMenu => "ContextMenu",
        MouseInteraction::Help => "Help",
        MouseInteraction::Pointer => "Pointer",
        MouseInteraction::Progress => "Progress",
        MouseInteraction::Wait => "Wait",
        MouseInteraction::Cell => "Cell",
        MouseInteraction::Crosshair => "Crosshair",
        MouseInteraction::Text => "Text",
        MouseInteraction::Alias => "Alias",
        MouseInteraction::Copy => "Copy",
        MouseInteraction::Move => "Move",
        MouseInteraction::NoDrop => "NoDrop",
        MouseInteraction::NotAllowed => "NotAllowed",
        MouseInteraction::Grab => "Grab",
        MouseInteraction::Grabbing => "Grabbing",
        MouseInteraction::ResizingHorizontally => "ResizingHorizontally",
        MouseInteraction::ResizingVertically => "ResizingVertically",
        MouseInteraction::ResizingDiagonallyUp => "ResizingDiagonallyUp",
        MouseInteraction::ResizingDiagonallyDown => "ResizingDiagonallyDown",
        MouseInteraction::ResizingColumn => "ResizingColumn",
        MouseInteraction::ResizingRow => "ResizingRow",
        MouseInteraction::AllScroll => "AllScroll",
        MouseInteraction::ZoomIn => "ZoomIn",
        MouseInteraction::ZoomOut => "ZoomOut",
    }
}

pub(in crate::codegen) fn first_class_mouse_interaction_code(name: &str) -> String {
    let name = name
        .strip_prefix("interaction.")
        .expect("checked interaction builtin");
    match name {
        "resize_horizontal" => "ResizingHorizontally".into(),
        "resize_vertical" => "ResizingVertically".into(),
        "resize_diagonal_up" => "ResizingDiagonallyUp".into(),
        "resize_diagonal_down" => "ResizingDiagonallyDown".into(),
        "resize_column" => "ResizingColumn".into(),
        "resize_row" => "ResizingRow".into(),
        _ => pascal(name),
    }
}

pub(in crate::codegen) fn binding_variant(binding: &str) -> String {
    if canonical_snake(binding) {
        format!("__Bind{}", pascal(binding))
    } else {
        format!("__0B{}", rust_identifier_hex(binding))
    }
}

pub(in crate::codegen) fn editor_variant(binding: &str) -> String {
    if canonical_snake(binding) {
        format!("__Edit{}", pascal(binding))
    } else {
        format!("__0E{}", rust_identifier_hex(binding))
    }
}

pub(in crate::codegen) fn controlled_state_name(
    code: &str,
    widget: &str,
    span: &Span,
) -> Result<String, Error> {
    let Some(name) = code.strip_prefix("self.") else {
        return Err(Error::new(
            "E139",
            span,
            format!("{widget} binding must resolve to an app state"),
        ));
    };
    if name.contains('.') {
        return Err(Error::new(
            "E139",
            span,
            format!("{widget} binding must resolve to one app state"),
        ));
    }
    Ok(name.to_owned())
}

pub(in crate::codegen) fn id_code(
    id: &Id,
    scope: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    if let Some(key) = &id.key {
        Ok(format!(
            "format!(\"{{}}/{}({{}})\", {scope}, {})",
            id.name,
            expr_code(key, env, document, ValueMode::Borrowed)?
        ))
    } else {
        Ok(format!("format!(\"{{}}/{}\", {scope})", id.name))
    }
}

pub(in crate::codegen) fn accessibility_key_code(
    id: Option<&Id>,
    kind: &str,
    span: &Span,
    scope: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    id.map_or_else(
        || {
            let scope = reconciliation_scope(scope, env);
            Ok(format!("format!(\"{{}}/@{kind}:{}\", {scope})", span.line))
        },
        |id| id_code(id, scope, env, document),
    )
}

pub(in crate::codegen) fn accessibility_code(
    options: &AccessibilityOptions,
    default_label: impl FnOnce() -> String,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(String, String), Error> {
    let label = options
        .label
        .as_ref()
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(default_label);
    let description = options
        .description
        .as_ref()
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .transpose()?
        .map(|value| format!(".description({value})"))
        .unwrap_or_default();
    Ok((label, description))
}

pub(in crate::codegen) fn widget_target_code(
    target: &WidgetTarget,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let constructor = if component_context(env).is_none()
        && target.segments.iter().all(|segment| segment.key.is_none())
    {
        "new"
    } else {
        "from"
    };
    Ok(format!(
        "::iced::widget::Id::{constructor}({})",
        widget_target_path_code(target, env, document)?
    ))
}

pub(in crate::codegen) fn widget_target_path_code(
    target: &WidgetTarget,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    if let Some((_, context)) = component_context(env) {
        let mut scope = context.code.clone();
        for segment in &target.segments {
            scope = id_code(segment, &scope, env, document)?;
        }
        return Ok(scope);
    }
    if target.segments.iter().all(|segment| segment.key.is_none()) {
        return Ok(rust_string(&format!(
            "{}/{}",
            document.app,
            target
                .segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("/")
        )));
    }
    let mut scope = rust_string(&document.app);
    for segment in &target.segments {
        scope = id_code(segment, &scope, env, document)?;
    }
    Ok(scope)
}

pub(in crate::codegen) fn widget_selector_code(
    selector: &WidgetSelector,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(String, Option<&'static str>), Error> {
    match selector {
        WidgetSelector::Id(target) => Ok((
            format!(
                "::iced::widget::selector::id({})",
                widget_target_code(target, env, document)?
            ),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Text(value) => Ok((
            expr_code(value, env, document, ValueMode::Owned)?,
            Some("__ice_widget_target_from_text"),
        )),
        WidgetSelector::Point { x, y } => Ok((
            format!(
                "::iced::Point::new(({}) as f32, ({}) as f32)",
                expr_code(x, env, document, ValueMode::Owned)?,
                expr_code(y, env, document, ValueMode::Owned)?
            ),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Focused => Ok((
            "::iced::widget::selector::is_focused()".into(),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Extern { function, args } => {
            let function = find_extern_function(document, function, ExternKind::Selector)
                .expect("checker validates selectors");
            Ok((
                format!(
                    "{}({})",
                    function.rust_path,
                    expr_list_code(args, env, document)?
                ),
                None,
            ))
        }
    }
}
