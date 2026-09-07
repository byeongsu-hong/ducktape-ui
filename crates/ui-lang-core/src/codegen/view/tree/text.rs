use super::*;

pub(super) fn options(
    text: &ResolvedText,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let options = &text.options;
    let style = &text.utility_style;
    let height = dimension_code(
        options.height.as_ref(),
        style.height_fill,
        program,
        env,
        text.origin,
    )?;
    let align_y = option_code(options.align_y.map(|align| {
        format!(
            "{WIRE}::AlignY::{}",
            match align {
                ResolvedTextVerticalAlignment::Top => "Top",
                ResolvedTextVerticalAlignment::Center => "Center",
                ResolvedTextVerticalAlignment::Bottom => "Bottom",
            }
        )
    }));
    let line_height = match &options.line_height {
        Some(line) => {
            let (kind, expression) = match line {
                ResolvedTextLineHeight::Relative(value) => ("Relative", *value),
                ResolvedTextLineHeight::Absolute(value) => ("Absolute", *value),
            };
            let value = clamped_f32_code(expression, "f32::EPSILON", "f32::MAX", program, env)?;
            Some(format!("{WIRE}::LineHeight::{kind}({value})"))
        }
        None => style
            .text_line_height
            .map(|value| format!("{WIRE}::LineHeight::Relative({value:?}f32)")),
    };
    let shaping = option_code(
        options
            .shaping
            .map(|value| format!("{WIRE}::Shaping::{value:?}")),
    );
    let wrapping = option_code(
        options
            .wrapping
            .map(|value| format!("{WIRE}::Wrapping::{value:?}")),
    );
    let face = match &options.font {
        Some(ResolvedTextFont::Named(font)) => Some(font),
        None if !style.font_monospace => program.settings().default_font.as_ref(),
        _ => None,
    };
    let font = face.map(|font| named_font(font, style.font_weight));
    Ok(format!(
        "{WIRE}::TextOptions {{ height: {height}, align_y: {align_y}, line_height: {}, shaping: {shaping}, wrapping: {wrapping}, tracking: {:?}f32, font: {} }}",
        option_code(line_height),
        options.tracking.unwrap_or(0.0).min(f64::from(f32::MAX)) as f32,
        option_code(font)
    ))
}

pub(super) fn named_font(
    font: &ResolvedDefaultFont,
    weight: Option<ResolvedStyleFontWeight>,
) -> String {
    let family = match &font.family {
        FontFamily::Named(name) => {
            format!("{WIRE}::FontFamily::Named({}.into())", rust_string(name))
        }
        family => format!("{WIRE}::FontFamily::{family:?}"),
    };
    let weight = weight.map_or_else(
        || format!("{:?}", font.weight),
        |weight| weight.code().to_owned(),
    );
    format!(
        "{WIRE}::NamedFont {{ family: {family}, weight: {WIRE}::Weight::{weight}, stretch: {WIRE}::FontStretch::{:?}, style: {WIRE}::FontStyle::{:?} }}",
        font.stretch, font.style
    )
}
