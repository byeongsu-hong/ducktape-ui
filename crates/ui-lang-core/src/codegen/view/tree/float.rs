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
    let float = program.resolved_float(node)?;
    let key = key_code(identity, "float", float.origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, env, &child_scope, slot)?;
    let x = expression(float.x, float, program, env)?;
    let y = expression(float.y, float, program, env)?;
    let scale = clamped_f32_code(float.scale, "f32::EPSILON", "f32::MAX", program, env)?;
    let shadow = shadow_code(
        float.shadow_color.as_ref(),
        float.shadow_x,
        float.shadow_y,
        float.shadow_blur,
        program,
        env,
    )?;
    let radius = option_code(radius_code(
        &ResolvedContainerRadius {
            all: float.radius.all,
            top_left: float.radius.top_left,
            top_right: float.radius.top_right,
            bottom_right: float.radius.bottom_right,
            bottom_left: float.radius.bottom_left,
        },
        0,
        program,
        env,
    )?);
    Ok(format!(
        "{WIRE}::Node::Float {{ key: {key}, x: {x}, y: {y}, scale: {scale}, shadow: {shadow}, radius: {radius}, content: Box::new({content}) }}"
    ))
}

fn geometry_index(local: ResolvedLocalId, float: &ResolvedFloat) -> Option<usize> {
    float
        .geometry
        .iter()
        .position(|geometry| geometry.local == local)
}

// Inspect call arguments and lists as well as arithmetic: geometry hidden in a
// Rust call cannot be evaluated by the guest before the host performs layout.
fn measured(
    root: ResolvedExpressionNodeId,
    float: &ResolvedFloat,
    program: &LoweredProgram,
) -> bool {
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        match &program.expressions().expression(id).kind {
            ResolvedExpressionKind::Path {
                root: ResolvedPathRoot::Local(local),
                ..
            } if geometry_index(*local, float).is_some() => return true,
            ResolvedExpressionKind::Unary { value, .. } => pending.push(*value),
            ResolvedExpressionKind::Binary { left, right, .. } => pending.extend([*left, *right]),
            ResolvedExpressionKind::List(values) => pending.extend(values),
            ResolvedExpressionKind::Call { arguments, .. } => {
                for argument in arguments {
                    match argument {
                        ResolvedCallArgument::Value(value) => pending.push(*value),
                        ResolvedCallArgument::Binding(local)
                            if geometry_index(*local, float).is_some() =>
                        {
                            return true;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn expression(
    expression: ResolvedExpressionId,
    float: &ResolvedFloat,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    enum Pending {
        Value(ResolvedExpressionNodeId),
        Operator(&'static str),
    }
    let root = program.expressions().expression_use(expression).root;
    let mut pending = vec![Pending::Value(root)];
    let mut ops = Vec::new();
    while let Some(next) = pending.pop() {
        // Each pending item contributes at least one emitted operation.
        if ops.len() + pending.len() + 1 > 64 {
            return Err(refused(
                program,
                float.origin,
                "a float expression with more than 64 operations",
            ));
        }
        let id = match next {
            Pending::Operator(operator) => {
                ops.push(format!("{WIRE}::FloatOp::{operator}"));
                continue;
            }
            Pending::Value(id) => id,
        };
        let node = program.expressions().expression(id);
        if !measured(id, float, program) {
            let value = resolved_expr_node_code(program, node.owner, id, env, ValueMode::Owned)?;
            ops.push(format!("{WIRE}::FloatOp::Number(({value}) as f64)"));
            continue;
        }
        match &node.kind {
            ResolvedExpressionKind::Path {
                root: ResolvedPathRoot::Local(local),
                projections,
            } if projections.is_empty() => {
                let index = geometry_index(*local, float).expect("measured float local");
                ops.push(format!("{WIRE}::FloatOp::Geometry({index})"));
            }
            ResolvedExpressionKind::Unary {
                operator: ResolvedUnaryOperator::NumericNegation,
                value,
            } => pending.extend([Pending::Operator("Negate"), Pending::Value(*value)]),
            ResolvedExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                let operator = match operator {
                    BinaryOp::Add => "Add",
                    BinaryOp::Sub => "Subtract",
                    BinaryOp::Mul => "Multiply",
                    BinaryOp::Div => "Divide",
                    BinaryOp::Rem => "Remainder",
                    _ => {
                        return Err(refused(
                            program,
                            float.origin,
                            "non-arithmetic float geometry",
                        ));
                    }
                };
                pending.extend([
                    Pending::Operator(operator),
                    Pending::Value(*right),
                    Pending::Value(*left),
                ]);
            }
            _ => {
                return Err(program.error_at_origin(
                    "E190",
                    float.origin,
                    "a native function of float geometry is not available in a view module",
                ));
            }
        }
    }
    Ok(format!(
        "{WIRE}::FloatExpression {{ ops: vec![{}] }}",
        ops.join(",")
    ))
}
