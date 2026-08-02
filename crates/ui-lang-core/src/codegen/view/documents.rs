use super::*;

pub(in crate::codegen) fn render_documents(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let view = document.resolved_view(node)?;
    let identity = match view.kind {
        ResolvedViewKind::Markdown | ResolvedViewKind::Table { .. } => view.identity.as_ref(),
        _ => None,
    };
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    let rendered = match &view.kind {
        ResolvedViewKind::Markdown => {
            let markdown = document.resolved_markdown(node)?;
            render_markdown(markdown, document, message, env)
        }
        ResolvedViewKind::TextEditor => {
            let editor = document.resolved_text_editor(node)?;
            render_text_editor(
                editor,
                view.identity.as_ref(),
                document,
                message,
                env,
                scope,
            )
        }
        ResolvedViewKind::Table { columns } => {
            let table = document.resolved_table(node)?;
            render_table(table, columns, document, message, env, &child_scope, slot)
        }
        ResolvedViewKind::If { .. }
        | ResolvedViewKind::For { .. }
        | ResolvedViewKind::Match { .. } => Err(document.invariant_at_origin(
            view.origin,
            "if, for, and match must be children of a layout node",
        )),
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, identity, message, env, document, scope)?;
    Ok(Some(rendered))
}
