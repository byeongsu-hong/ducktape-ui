use super::*;

pub(in crate::codegen) fn render_controls(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let view = document.resolved_view(node)?;
    let identity = match view.kind {
        ResolvedViewKind::Slider
        | ResolvedViewKind::Progress
        | ResolvedViewKind::PickList
        | ResolvedViewKind::ComboBox => view.identity.as_ref(),
        _ => None,
    };
    let rendered = match &view.kind {
        ResolvedViewKind::Button { content } => render_button(
            document.resolved_button(node)?,
            view.identity.as_ref(),
            *content,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ResolvedViewKind::Checkbox | ResolvedViewKind::Toggler | ResolvedViewKind::Radio => {
            render_boolean_control(
                document.resolved_boolean_control(node)?,
                view.identity.as_ref(),
                document,
                message,
                env,
                scope,
            )
        }
        ResolvedViewKind::Slider => render_slider(
            document.resolved_slider(node)?,
            view.identity.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        ResolvedViewKind::Progress => render_progress(
            document.resolved_progress(node)?,
            view.identity.as_ref(),
            document,
            env,
            scope,
        ),
        ResolvedViewKind::PickList => render_pick_list(
            document.resolved_pick_list(node)?,
            view.identity.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        ResolvedViewKind::ComboBox => render_combo_box(
            document.resolved_combo_box(node)?,
            view.identity.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, identity, message)?;
    Ok(Some(rendered))
}
