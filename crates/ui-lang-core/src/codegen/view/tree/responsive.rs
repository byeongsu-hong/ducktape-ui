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
    let responsive = program.resolved_responsive(node)?;
    let key = key_code(
        identity,
        "responsive",
        responsive.origin,
        scope,
        env,
        program,
    )?;
    let local_key = format!("__ice_container_{}", node.0);
    let mut child_env = ScopedBindingEnv::new(env);
    for (local, axis) in [
        (&responsive.measured_width, ContainerAxis::Width),
        (&responsive.measured_height, ContainerAxis::Height),
    ] {
        child_env.insert(
            local.name.clone(),
            Binding {
                code: format!("{local_key}.clone()"),
                ty: Type::F64,
                local: false,
                state: None,
                owner: Some(BindingOwner::ContainerSize(local.local, axis)),
            },
        );
    }
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, &child_env, &child_scope, slot)?;
    let width = dimension_code(
        responsive.width.as_ref(),
        false,
        program,
        env,
        responsive.origin,
    )?;
    let height = dimension_code(
        responsive.height.as_ref(),
        false,
        program,
        env,
        responsive.origin,
    )?;
    Ok(format!(
        "{{ let {local_key}: String = {key}; {WIRE}::Node::Responsive {{ key: {local_key}.clone(), width: {width}, height: {height}, content: Box::new({content}) }} }}"
    ))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_container_condition(
    out: &mut String,
    node: ViewId,
    children: &[ViewId],
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<bool, Error> {
    if program.target() != Target::Tree {
        return Ok(false);
    }
    let conditional = program.resolved_conditional(node)?;
    let root = program
        .expressions()
        .expression_use(conditional.condition)
        .root;
    if !measured(root, program, env) {
        return Ok(false);
    }
    let view = program.resolved_view(node)?;
    let mut pending = vec![root];
    let mut count = 0;
    while let Some(id) = pending.pop() {
        count += 1;
        if count > 64 {
            return Err(refused(
                program,
                view.origin,
                "a container condition with more than 64 operations",
            ));
        }
        if measured(id, program, env) {
            match &program.expressions().expression(id).kind {
                ResolvedExpressionKind::Unary { value, .. } => pending.push(*value),
                ResolvedExpressionKind::Binary { left, right, .. } => {
                    pending.extend([*left, *right])
                }
                _ => {}
            }
        }
    }
    let ops = operations(root, program, env, view.origin)?;
    let key = key_code(
        view.identity.as_ref(),
        "when",
        view.origin,
        scope,
        env,
        program,
    )?;
    let mut body = String::new();
    let _host_condition = super::super::outline::enter_host_condition();
    render_children(&mut body, children, program, message, env, scope, slot)?;
    write!(out, " __children.push({{ let mut __children = Vec::new(); {body} {WIRE}::Node::When {{ key: {key}, condition: {WIRE}::ContainerQuery {{ ops: vec![{ops}] }}, children: __children }} }});").unwrap();
    Ok(true)
}

fn dimension_binding<'a>(
    id: ResolvedLocalId,
    program: &LoweredProgram,
    env: &'a dyn BindingEnvironment,
) -> Option<(&'a Binding, ContainerAxis)> {
    let binding = env.get(&program.expressions().local(id).name)?;
    match binding.owner {
        Some(BindingOwner::ContainerSize(local, axis)) if id == local => Some((binding, axis)),
        _ => None,
    }
}
fn measured(
    root: ResolvedExpressionNodeId,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> bool {
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        match &program.expressions().expression(id).kind {
            ResolvedExpressionKind::Path {
                root: ResolvedPathRoot::Local(local),
                ..
            } if dimension_binding(*local, program, env).is_some() => return true,
            ResolvedExpressionKind::Unary { value, .. } => pending.push(*value),
            ResolvedExpressionKind::Binary { left, right, .. } => pending.extend([*left, *right]),
            ResolvedExpressionKind::List(values) => pending.extend(values),
            ResolvedExpressionKind::Call { arguments, .. } => {
                for arg in arguments {
                    match arg {
                        ResolvedCallArgument::Value(value) => pending.push(*value),
                        ResolvedCallArgument::Binding(local)
                            if dimension_binding(*local, program, env).is_some() =>
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
fn operations(
    id: ResolvedExpressionNodeId,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    origin: OriginId,
) -> Result<String, Error> {
    let expression = program.expressions().expression(id);
    if !measured(id, program, env) {
        validate_snapshot(id, program, origin)?;
        let value = resolved_expr_node_code(program, expression.owner, id, env, ValueMode::Owned)?;
        return Ok(match expression.ty {
            Type::Bool => format!("{WIRE}::QueryOp::Bool({value})"),
            Type::I64 | Type::F64 => format!("{WIRE}::QueryOp::Number(({value}) as f64)"),
            _ => {
                return Err(program.error_at_origin(
                    "E190",
                    origin,
                    "container conditions require numeric or Boolean data",
                ));
            }
        });
    }
    let op = |name: &str| format!("{WIRE}::QueryOp::{name}");
    match &expression.kind {
        ResolvedExpressionKind::Path {
            root: ResolvedPathRoot::Local(local),
            projections,
        } if projections.is_empty() => {
            let (binding, axis) = dimension_binding(*local, program, env).expect("measured local");
            Ok(format!(
                "{}({})",
                op(match axis {
                    ContainerAxis::Width => "Width",
                    ContainerAxis::Height => "Height",
                }),
                binding.code
            ))
        }
        ResolvedExpressionKind::Unary { operator, value } => Ok(format!(
            "{},{}",
            operations(*value, program, env, origin)?,
            op(match operator {
                ResolvedUnaryOperator::BooleanNot => "Not",
                ResolvedUnaryOperator::NumericNegation => "Negate",
            })
        )),
        ResolvedExpressionKind::Binary {
            operator,
            left,
            right,
        } => Ok(format!(
            "{},{},{}",
            operations(*left, program, env, origin)?,
            operations(*right, program, env, origin)?,
            op(match operator {
                BinaryOp::Add => "Add",
                BinaryOp::Sub => "Subtract",
                BinaryOp::Mul => "Multiply",
                BinaryOp::Div => "Divide",
                BinaryOp::Rem => "Remainder",
                BinaryOp::Eq => "Equal",
                BinaryOp::NotEq => "NotEqual",
                BinaryOp::Lt => "Less",
                BinaryOp::LtEq => "LessEqual",
                BinaryOp::Gt => "Greater",
                BinaryOp::GtEq => "GreaterEqual",
                BinaryOp::And => "And",
                BinaryOp::Or => "Or",
            })
        )),
        _ => Err(program.error_at_origin(
            "E190",
            origin,
            "a native function of responsive measurements",
        )),
    }
}

// Copied operands are evaluated before layout. Refuse calls and arithmetic that
// native &&/|| might skip; users can deliberately precompute thresholds in state.
fn validate_snapshot(
    root: ResolvedExpressionNodeId,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<(), Error> {
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        match &program.expressions().expression(id).kind {
            ResolvedExpressionKind::Bool(_)
            | ResolvedExpressionKind::I64(_)
            | ResolvedExpressionKind::F64(_)
            | ResolvedExpressionKind::Str(_) => {}
            ResolvedExpressionKind::Path { root, projections }
                if !matches!(root, ResolvedPathRoot::Value(ResolvedValueRef::Derived(_)))
                    && projections.iter().all(|projection| matches!(projection.kind, crate::lower::ResolvedProjectionKind::Struct(_))) => {}
            ResolvedExpressionKind::Unary {
                operator: ResolvedUnaryOperator::BooleanNot, value,
            } => pending.push(*value),
            ResolvedExpressionKind::Binary {
                operator: BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt
                    | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq | BinaryOp::And | BinaryOp::Or,
                left, right,
            } =>
            {
                pending.extend([*left, *right]);
            }
            _ => return Err(program.error_at_origin("E190", origin,
                "container conditions cannot eagerly copy calls or arithmetic (including lazy derived reads); precompute the threshold in guest state")),
        }
    }
    Ok(())
}
