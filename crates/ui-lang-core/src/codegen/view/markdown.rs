use super::*;

pub(in crate::codegen) fn render_markdown(
    markdown: &ResolvedMarkdown,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let program = document.hir();
    let content = resolved_markdown_content(markdown, env, program)?;
    let mut settings = String::from(
        "let mut __markdown_settings = ::iced::widget::markdown::Settings::from(self.__theme());",
    );
    for (value, field, minimum) in [
        (markdown.text_size, "text_size", "f32::EPSILON"),
        (markdown.h1_size, "h1_size", "f32::EPSILON"),
        (markdown.h2_size, "h2_size", "f32::EPSILON"),
        (markdown.h3_size, "h3_size", "f32::EPSILON"),
        (markdown.h4_size, "h4_size", "f32::EPSILON"),
        (markdown.h5_size, "h5_size", "f32::EPSILON"),
        (markdown.h6_size, "h6_size", "f32::EPSILON"),
        (markdown.code_size, "code_size", "f32::EPSILON"),
        (markdown.spacing, "spacing", "0.0"),
    ] {
        if let Some(value) = value {
            write!(
                settings,
                " __markdown_settings.{field} = {}.into();",
                resolved_markdown_f32(value, minimum, program, env)?
            )
            .unwrap();
        }
    }
    append_resolved_markdown_style(&mut settings, &markdown.style, program, env)?;
    let callback = resolved_interaction_route_callback_code(
        &markdown.link,
        "__event",
        &["__event"],
        env,
        program,
        message,
    )?;
    let view = if let Some(viewer) = &markdown.viewer {
        let function = program
            .try_extern_function(viewer.function)
            .ok_or_else(|| {
                program.invariant_at_origin(markdown.origin, "markdown viewer extern ID is invalid")
            })?;
        if function.kind != ExternKind::MarkdownViewer
            || function.output != viewer.output
            || function.borrowed != viewer.borrowed
            || function.params.len() != viewer.arguments.len()
            || viewer.borrowed.len() != viewer.arguments.len()
        {
            return Err(program.invariant_at_origin(
                markdown.origin,
                "markdown viewer extern contract diverged before emission",
            ));
        }
        let arguments = viewer
            .arguments
            .iter()
            .zip(&viewer.borrowed)
            .map(|(argument, borrowed)| {
                checked_expr_use_code(
                    program,
                    *argument,
                    env,
                    if *borrowed {
                        ValueMode::Borrowed
                    } else {
                        ValueMode::Owned
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        format!(
            "let __markdown_viewer = {}({arguments}); ::iced::widget::markdown::view_with({}.items(), __markdown_settings, &__markdown_viewer)",
            function.rust_path, content.code
        )
    } else {
        format!(
            "::iced::widget::markdown::view({}.items(), __markdown_settings)",
            content.code
        )
    };
    Ok(format!("{{ {settings} {view}.map({callback}) }}"))
}

fn resolved_markdown_content<'a>(
    markdown: &ResolvedMarkdown,
    env: &'a dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<&'a Binding, Error> {
    let content = env.get(&markdown.content.name).ok_or_else(|| {
        program.invariant_at_origin(
            markdown.origin,
            "markdown content is absent from its render scope",
        )
    })?;
    if content.owner != Some(BindingOwner::Value(markdown.content.id))
        || content.ty != Type::Markdown
    {
        return Err(program.invariant_at_origin(
            markdown.origin,
            "markdown render binding does not match its normalized state ID and type",
        ));
    }
    Ok(content)
}

fn resolved_markdown_f32(
    expression: CheckedExprUseId,
    minimum: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let code = checked_expr_use_code(program, expression, env, ValueMode::Owned)?;
    Ok(format!("(({code}) as f32).max({minimum}).min(f32::MAX)"))
}

fn append_resolved_markdown_style(
    code: &mut String,
    style: &ResolvedMarkdownStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    if let Some(font) = &style.font {
        write!(
            code,
            " __markdown_settings.style.font = {};",
            resolved_input_font_code(font)
        )
        .unwrap();
    }
    if let Some(background) = &style.inline_code_background {
        write!(
            code,
            " __markdown_settings.style.inline_code_highlight.background = {};",
            resolved_text_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(color) = &style.inline_code_color {
        write!(
            code,
            " __markdown_settings.style.inline_code_color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(font) = &style.inline_code_font {
        write!(
            code,
            " __markdown_settings.style.inline_code_font = {};",
            resolved_input_font_code(font)
        )
        .unwrap();
    }
    if let Some(font) = &style.code_block_font {
        write!(
            code,
            " __markdown_settings.style.code_block_font = {};",
            resolved_input_font_code(font)
        )
        .unwrap();
    }
    if let Some(color) = &style.link_color {
        write!(
            code,
            " __markdown_settings.style.link_color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(padding) = resolved_markdown_padding(&style.inline_code_padding, program, env)? {
        write!(
            code,
            " __markdown_settings.style.inline_code_padding = {padding};"
        )
        .unwrap();
    }
    if let Some(color) = &style.inline_code_border_color {
        write!(
            code,
            " __markdown_settings.style.inline_code_highlight.border.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(width) = style.inline_code_border_width {
        write!(
            code,
            " __markdown_settings.style.inline_code_highlight.border.width = {} as f32;",
            checked_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = resolved_text_radius_code(&style.inline_code_radius, program, env)? {
        write!(
            code,
            " __markdown_settings.style.inline_code_highlight.border.radius = {radius};"
        )
        .unwrap();
    }
    Ok(())
}

fn resolved_markdown_padding(
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
    let value = |expression: Option<CheckedExprUseId>| {
        expression
            .map(|expression| checked_expr_use_code(program, expression, env, ValueMode::Owned))
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
