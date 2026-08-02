use super::*;

pub(in crate::codegen) fn render_documents(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
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
        ViewNode::TextEditor { id, .. } => {
            let editor = document.hir().resolved_text_editor_for(node)?;
            render_text_editor(editor, id.as_ref(), document, message, env, scope)
        }
        ViewNode::Table { columns, .. } => {
            let table = document.hir().resolved_table_for(node)?;
            render_table(table, columns, document, message, env, &child_scope, slot)
        }
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
