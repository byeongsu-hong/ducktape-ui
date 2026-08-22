use super::*;

/// The clamp every emitted f32 dimension goes through, as text. Callers that
/// already hold assembled code — a composed ratio, say — use this; the rest go
/// through `clamped_f32_code`.
pub(in crate::codegen) fn clamped_f32(code: &str, minimum: &str, maximum: &str) -> String {
    format!("(({code}) as f32).max({minimum}).min({maximum})")
}

pub(in crate::codegen) fn clamped_f32_code(
    expression: ResolvedExpressionId,
    minimum: &str,
    maximum: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(clamped_f32(
        &resolved_expr_use_code(program, expression, env, ValueMode::Owned)?,
        minimum,
        maximum,
    ))
}

pub(in crate::codegen) fn text_shaping_code(shaping: TextShaping) -> &'static str {
    match shaping {
        TextShaping::Auto => "Auto",
        TextShaping::Basic => "Basic",
        TextShaping::Advanced => "Advanced",
    }
}

pub(in crate::codegen) fn resolved_default_font_code(font: &ResolvedDefaultFont) -> String {
    font_value_code(&font.family, font.weight, font.stretch, font.style)
}

fn font_value_code(
    family: &FontFamily,
    weight: FontWeight,
    stretch: FontStretch,
    style: FontStyle,
) -> String {
    let family = match family {
        FontFamily::Named(name) => format!("::iced::font::Family::Name({})", rust_string(name)),
        FontFamily::Serif => "::iced::font::Family::Serif".into(),
        FontFamily::SansSerif => "::iced::font::Family::SansSerif".into(),
        FontFamily::Cursive => "::iced::font::Family::Cursive".into(),
        FontFamily::Fantasy => "::iced::font::Family::Fantasy".into(),
        FontFamily::Monospace => "::iced::font::Family::Monospace".into(),
    };
    let weight = match weight {
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
    let stretch = match stretch {
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
    let style = match style {
        FontStyle::Normal => "Normal",
        FontStyle::Italic => "Italic",
        FontStyle::Oblique => "Oblique",
    };
    format!(
        "::iced::Font {{ family: {family}, weight: ::iced::font::Weight::{weight}, stretch: ::iced::font::Stretch::{stretch}, style: ::iced::font::Style::{style} }}"
    )
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

/// The application field holding every `secret` buffer. Private to the
/// generated module and absent from the declared state list, so nothing in Ice
/// — a preset, an `expect`, a snapshot — has a name for it.
pub(in crate::codegen) const SECRET_STORE_FIELD: &str = "__ice_secrets";

/// One message for every secret input, carrying the slot it belongs to. iced's
/// text input hands back an owned `String`, so the typed text does cross a
/// message on its way to the buffer; `update` moves it in and drops it there.
pub(in crate::codegen) const SECRET_TYPED_VARIANT: &str = "__SecretTyped";

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
