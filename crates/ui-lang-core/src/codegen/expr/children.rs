use super::*;

pub(in crate::codegen) fn render_children(
    out: &mut String,
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<(), Error> {
    for child in children {
        match child {
            ViewNode::If {
                condition,
                children,
                ..
            } => {
                let condition = expr_code(condition, env, document, ValueMode::Owned)?;
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
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                let Type::List(inner) = expr_type(items, &env_types(env), document, span)? else {
                    return Err(Error::new("E121", span, "for expects a list"));
                };
                let items = expr_code(items, env, document, ValueMode::Borrowed)?;
                let reconciliation_scope = reconciliation_scope(scope, env);
                write!(
                    out,
                    " for (__ice_index, {item}) in {items}.iter().cloned().enumerate() {{ let __for_scope = format!(\"{{}}/@for:{}({{}})\", {reconciliation_scope}, __ice_index);",
                    span.line
                )
                .unwrap();
                let mut child_env = env.clone();
                child_env.insert(
                    item.clone(),
                    Binding {
                        code: item.clone(),
                        ty: *inner,
                        local: false,
                        state: None,
                    },
                );
                set_reconciliation_scope(&mut child_env, "__for_scope.clone()".into());
                render_children(out, children, document, message, &child_env, scope, slot)?;
                out.push_str(" }");
            }
            ViewNode::Match { value, arms, span } => {
                let value_ty = expr_type(value, &env_types(env), document, span)?;
                let value = expr_code(value, env, document, ValueMode::Borrowed)?;
                write!(out, " match &({value}) {{").unwrap();
                for arm in arms {
                    write!(out, " {} => {{", match_pattern_code(&arm.pattern)).unwrap();
                    let mut child_env = env.clone();
                    if let Some((name, ty)) =
                        match_pattern_binding(&arm.pattern, &value_ty, document)
                    {
                        child_env.insert(
                            name.clone(),
                            Binding {
                                code: name,
                                ty,
                                local: false,
                                state: None,
                            },
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
