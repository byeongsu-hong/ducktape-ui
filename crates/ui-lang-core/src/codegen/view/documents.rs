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
        ViewNode::Markdown { .. } => {
            let markdown = document.hir().resolved_markdown_for(node)?;
            render_markdown(markdown, document, message, env)
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
