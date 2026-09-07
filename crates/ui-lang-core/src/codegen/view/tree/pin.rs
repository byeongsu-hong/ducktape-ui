use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    node: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let pin = program.resolved_pin(node)?;
    let key = key_code(identity, "pin", pin.origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, env, &child_scope, slot)?;
    let x = resolved_expr_use_code(program, pin.x, env, ValueMode::Owned)?;
    let y = resolved_expr_use_code(program, pin.y, env, ValueMode::Owned)?;
    let width = dimension_code(pin.width.as_ref(), false, program, env, pin.origin)?;
    let height = dimension_code(pin.height.as_ref(), false, program, env, pin.origin)?;
    Ok(format!(
        "{WIRE}::Node::Pin {{ key: {key}, x: ({x}) as f32, y: ({y}) as f32, width: {width}, height: {height}, content: Box::new({content}) }}"
    ))
}
