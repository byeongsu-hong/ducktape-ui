use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let handle = program.resolved_resize_handle(id)?;
    let key = key_code(identity, "resize", handle.origin, scope, env, program)?;
    let content = render_node(
        content,
        program,
        message,
        env,
        &rendered_child_scope(identity, scope)?,
        slot,
    )?;
    let discrete = |route: &Option<ResolvedInteractionRoute>| -> Result<String, Error> {
        Ok(option_code(
            route
                .as_ref()
                .map(|route| {
                    resolved_interaction_route_code(route, &[], env, program, message)
                        .map(|code| format!("{SLOTS}::message({code})"))
                })
                .transpose()?,
        ))
    };
    let drag = handle
        .drag
        .as_ref()
        .map(|route| -> Result<String, Error> {
            Ok(handler_code(
                "(f64, f64)",
                message,
                &snapshot_callback(
                    route,
                    "__delta: (f64, f64)",
                    &["__delta.0", "__delta.1"],
                    env,
                    program,
                    message,
                )?,
                "move |__sent: (f64, f64)| ::std::option::Option::Some(__route(__sent))",
            ))
        })
        .transpose()?;
    let cursor = option_code(
        handle
            .interaction
            .map(|cursor| format!("{WIRE}::mouse::Cursor::{}", mouse_interaction_code(cursor))),
    );
    Ok(format!(
        "{WIRE}::Node::ResizeHandle {{ key: {key}, on_press: {}, on_release: {}, on_drag: {}, cursor: {cursor}, content: ::std::boxed::Box::new({content}) }}",
        discrete(&handle.press)?,
        discrete(&handle.release)?,
        option_code(drag)
    ))
}
