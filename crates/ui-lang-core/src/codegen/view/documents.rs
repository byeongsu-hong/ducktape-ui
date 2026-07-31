use super::*;

pub(in crate::codegen) fn render_documents(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Markdown { id, .. } | ViewNode::Table { id, .. } => id.as_ref(),
        _ => None,
    };
    let child_scope = rendered_child_scope(id, scope, env, document)?;
    let rendered = match node {
        ViewNode::Markdown {
            content,
            options,
            route,
            ..
        } => {
            let mut settings = String::from(
                "let mut __markdown_settings = ::iced::widget::markdown::Settings::from(self.__theme());",
            );
            for (value, field, min) in [
                (&options.text_size, "text_size", "f32::EPSILON"),
                (&options.h1_size, "h1_size", "f32::EPSILON"),
                (&options.h2_size, "h2_size", "f32::EPSILON"),
                (&options.h3_size, "h3_size", "f32::EPSILON"),
                (&options.h4_size, "h4_size", "f32::EPSILON"),
                (&options.h5_size, "h5_size", "f32::EPSILON"),
                (&options.h6_size, "h6_size", "f32::EPSILON"),
                (&options.code_size, "code_size", "f32::EPSILON"),
                (&options.spacing, "spacing", "0.0"),
            ] {
                if let Some(value) = value {
                    write!(
                        settings,
                        " __markdown_settings.{field} = {}.into();",
                        clamped_f32_code(value, min, "f32::MAX", env, document)?
                    )
                    .unwrap();
                }
            }
            let style = &options.style;
            if let Some(font) = &style.font {
                write!(
                    settings,
                    " __markdown_settings.style.font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(background) = &style.inline_code_background {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.background = {};",
                    background_code(background, env, document)?
                )
                .unwrap();
            }
            if let Some(color) = &style.inline_code_color {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(font) = &style.inline_code_font {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(font) = &style.code_block_font {
                write!(
                    settings,
                    " __markdown_settings.style.code_block_font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(color) = &style.link_color {
                write!(
                    settings,
                    " __markdown_settings.style.link_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(padding) = typed_padding_code(&style.inline_code_padding, env, document)? {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_padding = {padding};"
                )
                .unwrap();
            }
            if let Some(color) = &style.inline_code_border_color {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(width) = &style.inline_code_border_width {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.width = {} as f32;",
                    expr_code(width, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(radius) = radius_code(
                style.inline_code_radius.as_ref(),
                [
                    style.inline_code_radius_top_left.as_ref(),
                    style.inline_code_radius_top_right.as_ref(),
                    style.inline_code_radius_bottom_right.as_ref(),
                    style.inline_code_radius_bottom_left.as_ref(),
                ],
                env,
                document,
            )? {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.radius = {radius};"
                )
                .unwrap();
            }
            let callback =
                route_callback_code(route, "__event", "__event", env, document, message)?;
            let view = if let Some(viewer) = &options.viewer {
                let function =
                    find_extern_function(document, &viewer.function, ExternKind::MarkdownViewer)
                        .expect("checker validates markdown viewer");
                let args = expr_list_code(&viewer.args, env, document)?;
                format!(
                    "let __markdown_viewer = {}({args}); ::iced::widget::markdown::view_with(self.{content}.items(), __markdown_settings, &__markdown_viewer)",
                    function.rust_path
                )
            } else {
                format!(
                    "::iced::widget::markdown::view(self.{content}.items(), __markdown_settings)"
                )
            };
            Ok(format!("{{ {settings} {view}.map({callback}) }}"))
        }
        ViewNode::TextEditor {
            binding,
            id,
            disabled,
            options,
            span,
        } => {
            let state = env.get(binding).ok_or_else(|| {
                Error::new("E150", span, format!("unknown editor state `{binding}`"))
            })?;
            let state_name = controlled_state_name(&state.code, "editor", span)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "editor", span, scope, env, document)?;
            let accessibility_label = options
                .placeholder
                .as_deref()
                .map(rust_string)
                .unwrap_or_else(|| "\"Editor\"".to_owned());
            let mut code = format!("::iced::widget::text_editor(&{})", state.code);
            if let Some(id) = id {
                write!(
                    code,
                    ".id(::iced::widget::Id::from({}))",
                    id_code(id, scope, env, document)?
                )
                .unwrap();
            }
            if let Some(placeholder) = &options.placeholder {
                write!(code, ".placeholder({})", rust_string(placeholder)).unwrap();
            }
            if let Some(width) = &options.width {
                write!(
                    code,
                    ".width({})",
                    clamped_f32_code(width, "0.0", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            for (value, method, min) in [
                (&options.min_height, "min_height", "0.0"),
                (&options.max_height, "max_height", "0.0"),
                (&options.size, "size", "f32::EPSILON"),
                (&options.padding, "padding", "0.0"),
            ] {
                if let Some(value) = value {
                    write!(
                        code,
                        ".{method}({})",
                        clamped_f32_code(value, min, "f32::MAX", env, document)?
                    )
                    .unwrap();
                }
            }
            if let Some(line_height) = &options.line_height {
                let line_height = text_line_height_code(line_height, env, document)?;
                write!(code, ".line_height({line_height})").unwrap();
            }
            if let Some(wrapping) = options.wrapping {
                write!(
                    code,
                    ".wrapping(::iced::widget::text::Wrapping::{})",
                    text_wrapping_code(wrapping)
                )
                .unwrap();
            }
            if let Some(font) = &options.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(syntax) = &options.highlight {
                let theme = match options
                    .highlight_theme
                    .unwrap_or(HighlightTheme::Base16Ocean)
                {
                    HighlightTheme::SolarizedDark => "SolarizedDark",
                    HighlightTheme::Base16Mocha => "Base16Mocha",
                    HighlightTheme::Base16Ocean => "Base16Ocean",
                    HighlightTheme::Base16Eighties => "Base16Eighties",
                    HighlightTheme::InspiredGithub => "InspiredGitHub",
                };
                write!(
                    code,
                    ".highlight({}, ::iced::highlighter::Theme::{theme})",
                    rust_string(syntax)
                )
                .unwrap();
            }
            if let Some(binding) = &options.key_binding {
                let function =
                    find_extern_function(document, &binding.function, ExternKind::EditorBinding)
                        .expect("checker validates editor binding");
                let route = options
                    .key_binding_route
                    .as_ref()
                    .expect("parser requires a key-binding route");
                let callback = route_callback_with_code(
                    route,
                    "__key_press",
                    env,
                    document,
                    |callback_env| {
                        let args = expr_args_suffix_code(&binding.args, callback_env, document)?;
                        let route = route_code(route, "__value", callback_env, document, message)?;
                        Ok(format!(
                            "{}(__key_press{args}).map(|__binding| __ice_map_editor_binding(__binding, &|__value| {route}))",
                            function.rust_path
                        ))
                    },
                )?;
                write!(code, ".key_binding({callback})").unwrap();
            }
            code.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                None,
                env,
                document,
                "style",
                "text_editor",
            )?);
            let finish = |editor: String| -> Result<String, Error> {
                if let Some(highlighter) = &options.highlighter {
                    let function = find_extern_function(
                        document,
                        &highlighter.function,
                        ExternKind::EditorHighlighter,
                    )
                    .expect("checker validates editor highlighter");
                    let args = expr_args_suffix_code(&highlighter.args, env, document)?;
                    Ok(format!("{}({editor}{args})", function.rust_path))
                } else {
                    Ok(editor)
                }
            };
            let variant = editor_variant(&state_name);
            let enabled = format!(
                "{code}.on_action({message}::{variant} as fn(::iced::widget::text_editor::Action) -> {message})"
            );
            if let Some(disabled) = disabled {
                let disabled_value = expr_code(disabled, env, document, ValueMode::Owned)?;
                let disabled_editor = finish(code)?;
                let enabled_editor = finish(enabled)?;
                let editor = format!(
                    "if __disabled {{ {disabled_editor}.into() }} else {{ {enabled_editor}.into() }}"
                );
                Ok(format!(
                    "{{ let __a11y_key = {accessibility_key}; let __disabled = {disabled_value}; let __editor_value = (&{}).text(); let __editor: __IceElement<'_, {message}> = {editor}; ::ui_lang_runtime::accessible(__editor, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::MultilineTextInput).logical_id(__a11y_key.clone()).label({accessibility_label}).value(__editor_value).disabled(__disabled).into() }}",
                    state.code
                ))
            } else {
                let editor = format!("{}.into()", finish(enabled)?);
                Ok(format!(
                    "{{ let __a11y_key = {accessibility_key}; let __editor_value = (&{}).text(); let __editor: __IceElement<'_, {message}> = {editor}; ::ui_lang_runtime::accessible(__editor, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::MultilineTextInput).logical_id(__a11y_key.clone()).label({accessibility_label}).value(__editor_value).into() }}",
                    state.code
                ))
            }
        }
        ViewNode::Table {
            item,
            rows,
            options,
            columns,
            span,
            ..
        } => render_table(
            item,
            rows,
            options,
            columns,
            span,
            document,
            message,
            env,
            &child_scope,
            slot,
        ),
        ViewNode::If { span, .. } | ViewNode::For { span, .. } | ViewNode::Match { span, .. } => {
            Err(Error::new(
                "E170",
                span,
                "if, for, and match must be children of a layout node",
            ))
        }
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}
