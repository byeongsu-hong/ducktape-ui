use super::*;

pub(in crate::codegen) fn render_children(
    out: &mut String,
    children: &[ViewNode],
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<(), Error> {
    for child in children {
        match child {
            ViewNode::If { children, .. } => {
                let program = document.hir();
                let conditional = program.resolved_conditional_for(child)?;
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
            ViewNode::For { children, span, .. } => {
                let CheckedViewFlow::For { items, item } =
                    &document.program().checked_view(span)?.flow
                else {
                    return Err(Error::new("E196", span, "for view has no checked flow"));
                };
                let local = document.program().checked_facts().local(*item);
                let item_name = &local.name;
                let items =
                    checked_expr_use_code(document.program(), *items, env, ValueMode::Borrowed)?;
                let reconciliation_scope = reconciliation_scope(scope, env);
                write!(
                    out,
                    " for (__ice_index, {item_name}) in {items}.iter().cloned().enumerate() {{ let __for_scope = format!(\"{{}}/@for:{}({{}})\", {reconciliation_scope}, __ice_index);",
                    span.line
                )
                .unwrap();
                let mut child_env = ScopedBindingEnv::new(env);
                child_env.insert(
                    item_name.clone(),
                    checked_local_binding(document.program(), *item, item_name.clone(), false),
                );
                child_env.insert(
                    RECONCILIATION_SCOPE_BINDING.into(),
                    reconciliation_scope_binding("__for_scope.clone()".into()),
                );
                render_children(out, children, document, message, &child_env, scope, slot)?;
                out.push_str(" }");
            }
            ViewNode::Match { arms, span, .. } => {
                let CheckedViewFlow::Match {
                    value,
                    arms: checked_arms,
                } = &document.program().checked_view(span)?.flow
                else {
                    return Err(Error::new("E196", span, "match view has no checked flow"));
                };
                if arms.len() != checked_arms.len() {
                    return Err(Error::new("E196", span, "match arm arena length diverged"));
                }
                let value_ty = document
                    .program()
                    .checked_facts()
                    .expression_use(*value)
                    .source
                    .clone();
                let value =
                    checked_expr_use_code(document.program(), *value, env, ValueMode::Borrowed)?;
                write!(out, " match &({value}) {{").unwrap();
                for (arm, checked_arm) in arms.iter().zip(checked_arms) {
                    write!(
                        out,
                        " {} => {{",
                        checked_match_pattern_code(
                            document.program(),
                            &checked_arm.pattern,
                            checked_arm.binding,
                            &value_ty,
                            &arm.span,
                        )?
                    )
                    .unwrap();
                    let mut child_env = ScopedBindingEnv::new(env);
                    if let Some(local) = checked_arm.binding {
                        let name = document.program().checked_facts().local(local).name.clone();
                        child_env.insert(
                            name.clone(),
                            checked_local_binding(document.program(), local, name, false),
                        );
                    }
                    render_children(
                        out,
                        &arm.children,
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
                    render_node_if_present(child, document, message, env, scope, slot)?
                {
                    write!(out, " __children.push({child});").unwrap();
                }
            }
        }
    }
    Ok(())
}
