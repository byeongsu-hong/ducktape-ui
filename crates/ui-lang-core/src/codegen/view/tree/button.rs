use super::*;

pub(super) fn recipe(
    button: &ResolvedButton,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let style = &button.utility_style;
    refuse_when(
        program,
        button.origin,
        style.has_non_button_properties(),
        "this utility style on a button",
    )?;
    if style.is_empty()
        && program.settings().default_font.is_none()
        && program.settings().default_text_size.is_none()
    {
        return Ok("None".into());
    }
    let color = |value: Option<&ResolvedThemeColor>| option_code(value.map(rgba_code));
    let font = if style.font_monospace
        || style.font_weight.is_some()
        || program.settings().default_font.is_some()
    {
        let fallback = ResolvedDefaultFont {
            family: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            stretch: FontStretch::Normal,
            style: FontStyle::Normal,
            origin: button.origin,
        };
        let face = if style.font_monospace {
            ResolvedDefaultFont {
                family: FontFamily::Monospace,
                ..fallback
            }
        } else {
            program.settings().default_font.clone().unwrap_or(fallback)
        };
        format!("Some({})", text::named_font(&face, style.font_weight))
    } else {
        "None".into()
    };
    Ok(format!(
        "Some({WIRE}::ButtonRecipe {{ base: {WIRE}::Face {{ background: {}, text: {}, border: {} }}, hover_background: {}, pressed_background: {}, disabled_background: {}, disabled_text: {}, disabled_opacity: {}, focus_ring: {}, text_size: {}, line_height: {}, font: {font} }})",
        color(style.background.as_ref()),
        color(style.text_color.as_ref()),
        border_code(&ResolvedContainerSurface::default(), style, program, env)?,
        color(style.hover_background.as_ref()),
        color(style.pressed_background.as_ref()),
        color(style.disabled_background.as_ref()),
        color(style.disabled_text_color.as_ref()),
        option_code(style.disabled_opacity.map(|v| format!("{v:?}f32"))),
        color(style.focus_visible_border_color.as_ref()),
        option_code(
            style
                .text_size
                .map(f64::from)
                .or(program.settings().default_text_size)
                .map(|v| format!("{:?}f32", v.min(f64::from(f32::MAX))))
        ),
        option_code(style.text_line_height.map(|v| format!("{v:?}f32"))),
    ))
}
