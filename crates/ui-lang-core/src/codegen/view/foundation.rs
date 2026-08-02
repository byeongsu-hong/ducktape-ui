use super::*;

pub(in crate::codegen) fn render_foundation(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let view = document.resolved_view(node)?;
    let rendered = match &view.kind {
        ResolvedViewKind::Layout { children } => render_layout(
            document.resolved_layout(node)?,
            view.identity.as_ref(),
            children,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ResolvedViewKind::Container { content } => render_container(
            document.resolved_container(node)?,
            view.identity.as_ref(),
            *content,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ResolvedViewKind::Overlay { content, layer } => {
            let overlay = document.resolved_overlay(node)?;
            render_overlay(
                view.identity.as_ref(),
                overlay,
                *content,
                *layer,
                document,
                message,
                env,
                scope,
                slot,
            )
        }
        ResolvedViewKind::PaneGrid { panes, templates } => render_pane_grid(
            document.resolved_pane_grid(node)?,
            panes,
            templates,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ResolvedViewKind::Text | ResolvedViewKind::RichText => render_text(
            document.resolved_text(node)?,
            view.identity.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        ResolvedViewKind::Input => render_input(
            document.resolved_input(node)?,
            view.identity.as_ref(),
            document,
            message,
            env,
            scope,
        ),
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
