use super::*;

pub(super) fn render(
    children: &[ResolvedRichChild],
    route: Option<&ResolvedInteractionRoute>,
    fields: &str,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let spans = super::super::text::render_rich_children(children, program, env, |span, env| {
        span_code(span, program, env)
    })?;
    let on_link = route
        .map(|route| {
            Ok::<_, Error>(handler_code(
                "::std::string::String",
                message,
                &snapshot_callback(
                    route,
                    "__link: ::std::string::String",
                    &["__link"],
                    env,
                    program,
                    message,
                )?,
                "move |__sent: ::std::string::String| ::std::option::Option::Some(__route(__sent))",
            ))
        })
        .transpose()?;
    Ok(format!(
        "{{ let mut __rich_spans: Vec<{WIRE}::RichSpan> = Vec::new(); {spans} {WIRE}::Node::RichText {{ {fields}, spans: __rich_spans, on_link: {} }} }}",
        option_code(on_link)
    ))
}

fn span_code(
    span: &ResolvedRichSpan,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let style = &span.utility_style;
    let value = |id| resolved_expr_use_code(program, id, env, ValueMode::Owned);
    let size = match span.size {
        Some(id) => Some(clamped_f32_code(
            id,
            "f32::EPSILON",
            "f32::MAX",
            program,
            env,
        )?),
        None => style.text_size.map(|value| format!("{value:?}f32")),
    };
    let line_height = match &span.line_height {
        Some(line) => {
            let (kind, id) = match line {
                ResolvedTextLineHeight::Relative(id) => ("Relative", *id),
                ResolvedTextLineHeight::Absolute(id) => ("Absolute", *id),
            };
            Some(format!(
                "{WIRE}::LineHeight::{kind}({})",
                clamped_f32_code(id, "f32::EPSILON", "f32::MAX", program, env)?
            ))
        }
        None => style
            .text_line_height
            .map(|v| format!("{WIRE}::LineHeight::Relative({v:?}f32)")),
    };
    let font = match &span.font {
        Some(ResolvedTextFont::Named(font)) => Some(text::named_font(font, style.font_weight)),
        None if !style.font_monospace && style.font_weight.is_none() => None,
        None if !style.font_monospace && program.settings().default_font.is_some() => {
            Some(text::named_font(
                program.settings().default_font.as_ref().unwrap(),
                style.font_weight,
            ))
        }
        font => {
            let family = if matches!(font, Some(ResolvedTextFont::Monospace))
                || (font.is_none() && style.font_monospace)
            {
                "Monospace"
            } else {
                "SansSerif"
            };
            let weight = style.font_weight.map_or("Normal", |weight| weight.code());
            Some(format!(
                "{WIRE}::NamedFont {{ family: {WIRE}::FontFamily::{family}, weight: {WIRE}::Weight::{weight}, stretch: {WIRE}::FontStretch::Normal, style: {WIRE}::FontStyle::Normal }}"
            ))
        }
    };
    let border = border_code(
        &ResolvedContainerSurface {
            border_color: span.border_color.clone(),
            border_width: span.border_width,
            radius: span.radius.clone(),
            ..Default::default()
        },
        &ResolvedStyle::default(),
        program,
        env,
    )?;
    let padding = edges_code(&span.padding, [0; 4], program, env)?;
    Ok(format!(
        "{WIRE}::RichSpan {{ content: ({}).to_string(), size: {}, line_height: {}, font: {}, color: {}, link: {}, background: {}, border: {border}, padding: {padding}, underline: {}, strikethrough: {} }}",
        value(span.value)?,
        option_code(size),
        option_code(line_height),
        option_code(font),
        option_code(
            span.color
                .as_ref()
                .or(style.text_color.as_ref())
                .map(rgba_code)
        ),
        option_code(span.link.map(value).transpose()?),
        plain_background_code(span.background.as_ref(), program, span.origin)?,
        span.underline
            .map(value)
            .transpose()?
            .unwrap_or_else(|| "false".into()),
        span.strikethrough
            .map(value)
            .transpose()?
            .unwrap_or_else(|| "false".into()),
    ))
}
