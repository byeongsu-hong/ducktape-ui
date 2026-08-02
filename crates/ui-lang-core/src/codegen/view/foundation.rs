use super::*;

pub(in crate::codegen) fn render_foundation(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Layout { id, children, .. } => render_layout(
            document.hir().resolved_layout_for(node)?,
            id,
            children,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ViewNode::Container { id, content, .. } => render_container(
            document.program().resolved_container_for(node)?,
            id,
            content,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ViewNode::Overlay {
            id, content, layer, ..
        } => {
            let overlay = document.program().resolved_overlay_for(node)?;
            render_overlay(
                id, overlay, content, layer, document, message, env, scope, slot,
            )
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => render_pane_grid(
            document.program().resolved_pane_grid_for(node)?,
            panes,
            templates,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ViewNode::Text { id, .. } | ViewNode::RichText { id, .. } => render_text(
            document.hir().resolved_text_for(node)?,
            id,
            document,
            message,
            env,
            scope,
        ),
        ViewNode::Input { id, .. } => render_input(
            document.hir().resolved_input_for(node)?,
            id,
            document,
            message,
            env,
            scope,
        ),
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
