use super::*;

pub(in crate::codegen) fn render_controls(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Slider { id, .. }
        | ViewNode::Progress { id, .. }
        | ViewNode::PickList { id, .. }
        | ViewNode::ComboBox { id, .. } => id.as_ref(),
        _ => None,
    };
    let rendered = match node {
        ViewNode::Button { content, id, .. } => render_button(
            document.program().resolved_button_for(node)?,
            id,
            content.as_deref(),
            document,
            message,
            env,
            scope,
            slot,
        ),
        ViewNode::Checkbox { .. } | ViewNode::Toggler { .. } | ViewNode::Radio { .. } => {
            render_boolean_control(
                document.hir().resolved_boolean_control_for(node)?,
                document.hir(),
                message,
                env,
                scope,
            )
        }
        ViewNode::Slider { id, .. } => render_slider(
            document.program().resolved_slider_for(node)?,
            id.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        ViewNode::Progress { id, .. } => render_progress(
            document.program().resolved_progress_for(node)?,
            id.as_ref(),
            document,
            env,
            scope,
        ),
        ViewNode::PickList { id, .. } => render_pick_list(
            document.hir().resolved_pick_list_for(node)?,
            id.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        ViewNode::ComboBox { id, .. } => render_combo_box(
            document.hir().resolved_combo_box_for(node)?,
            id.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}
