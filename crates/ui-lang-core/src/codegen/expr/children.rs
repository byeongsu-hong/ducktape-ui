use super::*;

pub(in crate::codegen) fn render_children(
    out: &mut String,
    children: &[ViewId],
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<(), Error> {
    for child in children {
        let view = document.resolved_view(*child)?;
        match &view.kind {
            ResolvedViewKind::If { children } => {
                let program = document;
                let conditional = program.resolved_conditional(*child)?;
                let condition =
                    checked_expr_use_code(program, conditional.condition, env, ValueMode::Owned)?;
                if condition == "false" {
                    continue;
                }
                if condition == "true" {
                    render_children(out, children, document, message, env, scope, slot)?;
                    continue;
                }
                write!(out, " if {condition} {{").unwrap();
                render_children(out, children, document, message, env, scope, slot)?;
                out.push_str(" }");
            }
            ResolvedViewKind::For { children } => {
                let program = document;
                let iteration = program.resolved_iteration(*child)?;
                let item_name = &iteration.item.name;
                let items =
                    checked_expr_use_code(program, iteration.items, env, ValueMode::Borrowed)?;
                let reconciliation_scope = reconciliation_scope(scope, env);
                write!(
                    out,
                    " for (__ice_index, {item_name}) in {items}.iter().cloned().enumerate() {{ let __for_scope = format!(\"{{}}/@for:{}({{}})\", {reconciliation_scope}, __ice_index);",
                    iteration.reconciliation_line
                )
                .unwrap();
                let mut child_env = ScopedBindingEnv::new(env);
                child_env.insert(
                    item_name.clone(),
                    checked_local_binding(
                        LocalBindingTypeSource::Checked(program),
                        iteration.item.local,
                        item_name.clone(),
                        false,
                    ),
                );
                child_env.insert(
                    RECONCILIATION_SCOPE_BINDING.into(),
                    reconciliation_scope_binding("__for_scope.clone()".into()),
                );
                render_children(out, children, document, message, &child_env, scope, slot)?;
                out.push_str(" }");
            }
            ResolvedViewKind::Match { arms } => {
                let program = document;
                let resolved = program.resolved_match(*child)?;
                if arms.len() != resolved.arms.len() {
                    return Err(
                        program.invariant_at_origin(view.origin, "match HIR arm length diverged")
                    );
                }
                let value =
                    checked_expr_use_code(program, resolved.value, env, ValueMode::Borrowed)?;
                write!(out, " match &({value}) {{").unwrap();
                for (arm_children, resolved_arm) in arms.iter().zip(&resolved.arms) {
                    write!(
                        out,
                        " {} => {{",
                        resolved_match_pattern_code(resolved_arm, program)?
                    )
                    .unwrap();
                    let mut child_env = ScopedBindingEnv::new(env);
                    if let Some(payload) = &resolved_arm.binding {
                        let name = payload.name.clone();
                        child_env.insert(
                            name.clone(),
                            checked_local_binding(
                                LocalBindingTypeSource::Hir(payload),
                                payload.local,
                                name,
                                false,
                            ),
                        );
                    }
                    render_children(
                        out,
                        arm_children,
                        document,
                        message,
                        &child_env,
                        scope,
                        slot,
                    )?;
                    out.push_str(" },");
                }
                out.push_str(" }");
            }
            _ => {
                if let Some(child) =
                    render_node_if_present(*child, document, message, env, scope, slot)?
                {
                    write!(out, " __children.push({child});").unwrap();
                }
            }
        }
    }
    Ok(())
}
