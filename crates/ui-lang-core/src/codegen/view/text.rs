use super::*;

pub(in crate::codegen) fn render_text(
    text: &ResolvedText,
    identity: Option<&ResolvedViewIdentity>,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    match &text.content {
        ResolvedTextContent::Plain { value } => {
            let value = resolved_expr_use_code(program, *value, env, ValueMode::Borrowed)?;
            let accessibility_key = resolved_accessibility_key_code(
                identity,
                "text",
                text.origin,
                scope,
                env,
                document,
            )?;
            let code = resolved_plain_text_code(text, message, env, program)?;
            // A tracked run is a row, and a ruled text is a paragraph span —
            // neither is the Text widget the selectable wrapper adapts.
            let plain_widget = text
                .options
                .tracking
                .filter(|tracking| *tracking > 0.0)
                .is_none()
                && text.options.underline.is_none()
                && text.options.strikethrough.is_none();
            let selection = if plain_widget {
                "let __text = ::ui_lang_runtime::selectable_text(__text);"
            } else {
                ""
            };
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __text_value = ({value}).to_string(); let __text = {code}; {selection} ::ui_lang_runtime::accessible(__text, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::Label).logical_id(__a11y_key).value(__text_value).into() }}"
            ))
        }
        ResolvedTextContent::Rich {
            color,
            children,
            route,
        } => render_resolved_rich_text(
            text,
            identity,
            color.as_ref(),
            children,
            route.as_ref(),
            document,
            message,
            env,
            scope,
        ),
    }
}

fn resolved_plain_text_code(
    text: &ResolvedText,
    message: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let options = &text.options;
    let style = &text.utility_style;
    if options.underline.is_some() || options.strikethrough.is_some() {
        return resolved_ruled_text_code(text, env, program);
    }
    let mut glyph = String::new();
    let Some(tracking) = options.tracking.filter(|tracking| *tracking > 0.0) else {
        glyph.push_str("::iced::widget::text(__text_value.clone())");
        append_resolved_glyph_options(&mut glyph, options, style, env, program)?;
        return Ok(glyph);
    };
    let mut run = options.clone();
    run.width = None;
    run.height = None;
    run.align_x = None;
    run.align_y = None;
    glyph.push_str("::iced::widget::text(__grapheme.to_owned())");
    append_resolved_glyph_options(&mut glyph, &run, style, env, program)?;
    let mut code = format!(
        "{{ let mut __tracked: ::std::vec::Vec<__IceElement<'_, {message}>> = ::std::vec::Vec::new(); for __grapheme in ::ui_lang_runtime::graphemes(&__text_value) {{ __tracked.push({glyph}.into()); }} let __spacing = ::ui_lang_runtime::bounded_spacing({}, __tracked.len()); let __run = ::iced::widget::row(__tracked).spacing(__spacing);",
        rust_f64(tracking)
    );
    let bounded = options.width.is_some()
        || options.height.is_some()
        || options.align_x.is_some()
        || options.align_y.is_some();
    if !bounded {
        code.push_str(" __run }");
        return Ok(code);
    }
    let mut wrapper = String::from("::iced::widget::container(__run)");
    append_resolved_text_dimensions(
        &mut wrapper,
        [&options.width, &options.height],
        program,
        env,
    )?;
    if let Some(alignment) = options.align_x {
        let alignment = match alignment {
            ResolvedTextAlignment::Default | ResolvedTextAlignment::Left => "Left",
            ResolvedTextAlignment::Center => "Center",
            ResolvedTextAlignment::Right => "Right",
            ResolvedTextAlignment::Justified => {
                return Err(program.invariant_at_origin(
                    text.origin,
                    "tracked text retained a justified alignment",
                ));
            }
        };
        write!(
            wrapper,
            ".align_x(::iced::alignment::Horizontal::{alignment})"
        )
        .unwrap();
    }
    if let Some(alignment) = options.align_y {
        let alignment = resolved_text_vertical_alignment_code(alignment);
        write!(
            wrapper,
            ".align_y(::iced::alignment::Vertical::{alignment})"
        )
        .unwrap();
    }
    write!(code, " {wrapper} }}").unwrap();
    Ok(code)
}

/// An underlined or struck text renders as a one-span paragraph: iced's
/// plain `Text` cannot draw a rule, and `Span` can. The container carries the
/// same options the plain widget would; the check layer keeps `tracking=` and
/// `shape=` — the two options a paragraph cannot express — out of this path.
fn resolved_ruled_text_code(
    text: &ResolvedText,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let options = &text.options;
    let mut span = String::from("::iced::widget::span(__text_value.clone())");
    if let Some(underline) = options.underline {
        write!(
            span,
            ".underline({})",
            resolved_expr_use_code(program, underline, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(strikethrough) = options.strikethrough {
        write!(
            span,
            ".strikethrough({})",
            resolved_expr_use_code(program, strikethrough, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    let mut code = format!(
        "{{ let __rich_spans: ::std::vec::Vec<::iced::widget::text::Span<'_, ::std::string::String>> = ::std::vec![{span}]; ::iced::widget::rich_text(__rich_spans)"
    );
    append_resolved_glyph_options(&mut code, options, &text.utility_style, env, program)?;
    code.push_str(" }");
    Ok(code)
}

fn append_resolved_glyph_options(
    code: &mut String,
    options: &ResolvedTextOptions,
    style: &ResolvedStyle,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<(), Error> {
    append_resolved_text_options(code, options, style, env, program)?;
    // `disabled:text-*` exists only on text inside a button's content (the
    // checker enforces it), where the enclosing button's generated block
    // binds `__disabled` — the same predicate that makes the button
    // `Status::Disabled`. Branching on it here is how an explicitly-colored
    // child follows the button's disabled ramp.
    match (&style.text_color, &style.disabled_text_color) {
        (Some(color), Some(disabled)) => {
            write!(
                code,
                ".color(if __disabled {{ {} }} else {{ {} }})",
                resolved_theme_color(disabled),
                resolved_theme_color(color)
            )
            .unwrap();
        }
        (None, Some(disabled)) => {
            write!(
                code,
                ".color_maybe(if __disabled {{ ::std::option::Option::Some({}) }} else {{ ::std::option::Option::None }})",
                resolved_theme_color(disabled)
            )
            .unwrap();
        }
        (Some(color), None) => {
            write!(code, ".color({})", resolved_theme_color(color)).unwrap();
        }
        (None, None) => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_resolved_rich_text(
    text: &ResolvedText,
    identity: Option<&ResolvedViewIdentity>,
    color: Option<&ResolvedThemeColor>,
    children: &[ResolvedRichChild],
    route: Option<&ResolvedInteractionRoute>,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let program = document;
    // Every child appends to the same span vector, so literal spans and
    // `for`-generated spans land in one paragraph widget.
    let mut spans = String::new();
    for child in children {
        match child {
            ResolvedRichChild::Span(rich_span) => {
                write!(
                    spans,
                    " __rich_spans.push({});",
                    render_resolved_rich_span(rich_span, program, env)?
                )
                .unwrap();
            }
            ResolvedRichChild::For(iteration) => {
                let item_name = &iteration.item.name;
                let items =
                    resolved_expr_use_code(program, iteration.items, env, ValueMode::Borrowed)?;
                let mut child_env = ScopedBindingEnv::new(env);
                child_env.insert(
                    item_name.clone(),
                    resolved_local_binding(
                        LocalBindingTypeSource::Resolved(program),
                        iteration.item.local,
                        item_name.clone(),
                        false,
                    ),
                );
                write!(spans, " for {item_name} in {items}.iter().cloned() {{").unwrap();
                for rich_span in &iteration.spans {
                    write!(
                        spans,
                        " __rich_spans.push({});",
                        render_resolved_rich_span(rich_span, program, &child_env)?
                    )
                    .unwrap();
                }
                spans.push_str(" }");
            }
        }
    }
    let mut code = String::from("::iced::widget::rich_text(__rich_spans)");
    append_resolved_text_options(&mut code, &text.options, &text.utility_style, env, program)?;
    if let Some(color) = color {
        write!(code, ".color({})", resolved_theme_color(color)).unwrap();
    } else if let Some(color) = &text.utility_style.text_color {
        write!(code, ".color({})", resolved_theme_color(color)).unwrap();
    }
    if let Some(route) = route {
        let callback = resolved_interaction_route_callback_code(
            route,
            "__link",
            &["__link"],
            env,
            program,
            message,
        )?;
        write!(code, ".on_link_click({callback})").unwrap();
    }
    let rendered = format!(
        "{{ let mut __rich_spans: ::std::vec::Vec<::iced::widget::text::Span<'_, ::std::string::String>> = ::std::vec::Vec::new();{spans} {code}.into() }}"
    );
    let Some(identity) = identity else {
        return Ok(rendered);
    };
    let id = resolved_view_identity_code(identity, scope, env, document)?;
    Ok(format!(
        "{{ let __a11y_key = {id}; let __identified_text: __IceElement<'_, {message}> = {rendered}; ::ui_lang_runtime::accessible(__identified_text, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::Label).logical_id(__a11y_key).into() }}"
    ))
}

fn render_resolved_rich_span(
    rich_span: &ResolvedRichSpan,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let value = resolved_expr_use_code(program, rich_span.value, env, ValueMode::Owned)?;
    let mut code = format!("::iced::widget::span({value})");
    if let Some(size) = rich_span.size {
        write!(
            code,
            ".size({})",
            clamped_f32_code(size, "f32::EPSILON", "f32::MAX", program, env)?
        )
        .unwrap();
    } else if let Some(size) = rich_span.utility_style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    if let Some(line_height) = &rich_span.line_height {
        write!(
            code,
            ".line_height({})",
            resolved_text_line_height_code(line_height, program, env)?
        )
        .unwrap();
    } else if let Some(line_height) = rich_span.utility_style.text_line_height {
        write!(
            code,
            ".line_height(::iced::widget::text::LineHeight::Relative({line_height}))"
        )
        .unwrap();
    }
    if let Some(font) =
        resolved_styled_text_font_code(rich_span.font.as_ref(), &rich_span.utility_style)
    {
        write!(code, ".font({font})").unwrap();
    }
    if let Some(color) = &rich_span.color {
        write!(code, ".color({})", resolved_theme_color(color)).unwrap();
    } else if let Some(color) = &rich_span.utility_style.text_color {
        write!(code, ".color({})", resolved_theme_color(color)).unwrap();
    }
    if let Some(link) = rich_span.link {
        write!(
            code,
            ".link({})",
            resolved_expr_use_code(program, link, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(background) = &rich_span.background {
        write!(
            code,
            ".background({})",
            resolved_text_background_code(background, program, env)?
        )
        .unwrap();
    }
    let has_border = rich_span.border_color.is_some()
        || rich_span.border_width.is_some()
        || rich_span.radius.all.is_some()
        || rich_span.radius.top_left.is_some()
        || rich_span.radius.top_right.is_some()
        || rich_span.radius.bottom_right.is_some()
        || rich_span.radius.bottom_left.is_some();
    if has_border {
        let color = rich_span
            .border_color
            .as_ref()
            .map(resolved_theme_color)
            .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
        let width = rich_span.border_width.map_or_else(
            || Ok("0.0".to_owned()),
            |width| resolved_expr_use_code(program, width, env, ValueMode::Owned),
        )?;
        let radius = resolved_text_radius_code(&rich_span.radius, program, env)?
            .unwrap_or_else(|| "::iced::border::Radius::default()".into());
        write!(
            code,
            ".border(::iced::Border {{ color: {color}, width: {width} as f32, radius: {radius} }})"
        )
        .unwrap();
    }
    if let Some(padding) = resolved_text_padding_code(&rich_span.padding, program, env)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(underline) = rich_span.underline {
        write!(
            code,
            ".underline({})",
            resolved_expr_use_code(program, underline, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(strikethrough) = rich_span.strikethrough {
        write!(
            code,
            ".strikethrough({})",
            resolved_expr_use_code(program, strikethrough, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(code)
}

fn append_resolved_text_options(
    code: &mut String,
    options: &ResolvedTextOptions,
    style: &ResolvedStyle,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<(), Error> {
    if let Some(size) = options.size {
        write!(
            code,
            ".size({})",
            clamped_f32_code(size, "f32::EPSILON", "f32::MAX", program, env)?
        )
        .unwrap();
    } else if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    append_resolved_text_dimensions(code, [&options.width, &options.height], program, env)?;
    if let Some(line_height) = &options.line_height {
        write!(
            code,
            ".line_height({})",
            resolved_text_line_height_code(line_height, program, env)?
        )
        .unwrap();
    } else if let Some(line_height) = style.text_line_height {
        write!(
            code,
            ".line_height(::iced::widget::text::LineHeight::Relative({line_height}))"
        )
        .unwrap();
    }
    if let Some(alignment) = options.align_x {
        write!(
            code,
            ".align_x(::iced::widget::text::Alignment::{})",
            resolved_text_alignment_code(alignment)
        )
        .unwrap();
    }
    if let Some(alignment) = options.align_y {
        write!(
            code,
            ".align_y(::iced::alignment::Vertical::{})",
            resolved_text_vertical_alignment_code(alignment)
        )
        .unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".shaping(::iced::widget::text::Shaping::{})",
            resolved_text_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".wrapping(::iced::widget::text::Wrapping::{})",
            resolved_text_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = resolved_styled_text_font_code(options.font.as_ref(), style) {
        write!(code, ".font({font})").unwrap();
    }
    if let Some(custom) = &options.custom_style {
        let arguments = custom
            .arguments
            .iter()
            .map(|argument| resolved_expr_use_code(program, *argument, env, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?;
        let suffix = arguments
            .into_iter()
            .map(|argument| format!(", {argument}"))
            .collect::<String>();
        write!(
            code,
            ".style(move |__theme| {}(__theme{suffix}))",
            program.extern_function(custom.function).rust_path
        )
        .unwrap();
    }
    Ok(())
}

pub(super) fn resolved_styled_text_font_code(
    font: Option<&ResolvedTextFont>,
    style: &ResolvedStyle,
) -> Option<String> {
    let base = match font {
        Some(ResolvedTextFont::Default) => Some("::iced::Font::DEFAULT".into()),
        Some(ResolvedTextFont::Monospace) => Some("::iced::Font::MONOSPACE".into()),
        Some(ResolvedTextFont::Named(font)) => Some(resolved_default_font_code(font)),
        None if style.font_monospace => Some("::iced::Font::MONOSPACE".into()),
        None if style.font_weight.is_some() => Some("Self::default_font()".into()),
        None => None,
    };
    base.map(|font| match style.font_weight {
        Some(weight) => format!(
            "::iced::Font {{ weight: ::iced::font::Weight::{}, ..{font} }}",
            weight.code()
        ),
        None => font,
    })
}

fn append_resolved_text_dimensions(
    code: &mut String,
    dimensions: [&Option<ResolvedContainerLength>; 2],
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    for (method, length) in ["width", "height"].into_iter().zip(dimensions) {
        if let Some(length) = length {
            write!(
                code,
                ".{method}({})",
                resolved_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    Ok(())
}

fn resolved_text_line_height_code(
    line_height: &ResolvedTextLineHeight,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    match line_height {
        ResolvedTextLineHeight::Relative(expression) => Ok(format!(
            "::iced::widget::text::LineHeight::Relative({})",
            clamped_f32_code(*expression, "f32::EPSILON", "f32::MAX", program, env)?
        )),
        ResolvedTextLineHeight::Absolute(expression) => Ok(format!(
            "::iced::widget::text::LineHeight::Absolute({}.into())",
            clamped_f32_code(*expression, "f32::EPSILON", "f32::MAX", program, env)?
        )),
    }
}

pub(super) fn resolved_text_background_code(
    background: &ResolvedContainerBackground,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match background {
        ResolvedContainerBackground::Color(color) => {
            format!("::iced::Background::Color({})", resolved_theme_color(color))
        }
        ResolvedContainerBackground::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                resolved_expr_use_code(program, *angle, env, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    resolved_expr_use_code(program, stop.offset, env, ValueMode::Owned)?,
                    resolved_theme_color(&stop.color)
                )
                .unwrap();
            }
            code.push(')');
            code
        }
    })
}

fn resolved_text_padding_code(
    padding: &ResolvedContainerPadding,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let value = |expression: Option<ResolvedExpressionId>| {
        expression
            .map(|expression| resolved_expr_use_code(program, expression, env, ValueMode::Owned))
            .transpose()
    };
    let all = value(padding.all)?.unwrap_or_else(|| "0.0".into());
    let x = value(padding.x)?.unwrap_or_else(|| all.clone());
    let y = value(padding.y)?.unwrap_or_else(|| all.clone());
    let top = value(padding.top)?.unwrap_or_else(|| y.clone());
    let right = value(padding.right)?.unwrap_or_else(|| x.clone());
    let bottom = value(padding.bottom)?.unwrap_or(y);
    let left = value(padding.left)?.unwrap_or(x);
    Ok(Some(format!(
        "::ui_lang_runtime::bounded_padding({top}, {right}, {bottom}, {left})"
    )))
}

pub(super) fn resolved_text_radius_code(
    radius: &ResolvedContainerRadius,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
    {
        return Ok(None);
    }
    let base = radius
        .all
        .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
        .transpose()?
        .unwrap_or_else(|| "0.0".into());
    let corner = |value: Option<ResolvedExpressionId>| {
        value
            .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
            .transpose()
    };
    let top_left = corner(radius.top_left)?.unwrap_or_else(|| base.clone());
    let top_right = corner(radius.top_right)?.unwrap_or_else(|| base.clone());
    let bottom_right = corner(radius.bottom_right)?.unwrap_or_else(|| base.clone());
    let bottom_left = corner(radius.bottom_left)?.unwrap_or(base);
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {top_left}, top_right: {top_right}, bottom_right: {bottom_right}, bottom_left: {bottom_left} }}"
    )))
}

fn resolved_text_alignment_code(alignment: ResolvedTextAlignment) -> &'static str {
    match alignment {
        ResolvedTextAlignment::Default => "Default",
        ResolvedTextAlignment::Left => "Left",
        ResolvedTextAlignment::Center => "Center",
        ResolvedTextAlignment::Right => "Right",
        ResolvedTextAlignment::Justified => "Justified",
    }
}

fn resolved_text_vertical_alignment_code(alignment: ResolvedTextVerticalAlignment) -> &'static str {
    match alignment {
        ResolvedTextVerticalAlignment::Top => "Top",
        ResolvedTextVerticalAlignment::Center => "Center",
        ResolvedTextVerticalAlignment::Bottom => "Bottom",
    }
}

fn resolved_text_shaping_code(shaping: ResolvedTextShaping) -> &'static str {
    match shaping {
        ResolvedTextShaping::Auto => "Auto",
        ResolvedTextShaping::Basic => "Basic",
        ResolvedTextShaping::Advanced => "Advanced",
    }
}

fn resolved_text_wrapping_code(wrapping: ResolvedTextWrapping) -> &'static str {
    match wrapping {
        ResolvedTextWrapping::None => "None",
        ResolvedTextWrapping::Word => "Word",
        ResolvedTextWrapping::Glyph => "Glyph",
        ResolvedTextWrapping::WordOrGlyph => "WordOrGlyph",
    }
}
