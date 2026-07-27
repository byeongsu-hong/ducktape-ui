use super::*;

pub(in crate::codegen) fn render_content(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let id = match node {
        ViewNode::Rule { id, .. }
        | ViewNode::QrCode { id, .. }
        | ViewNode::Space { id, .. }
        | ViewNode::ExternComponent { id, .. }
        | ViewNode::Themer { id, .. }
        | ViewNode::Shader { id, .. } => id.as_ref(),
        _ => None,
    };
    let rendered = match node {
        ViewNode::Rule {
            axis,
            thickness,
            options,
            ..
        } => {
            let thickness = expr_code(thickness, env, document, ValueMode::Owned)?;
            let axis = match axis {
                Axis::Horizontal => "horizontal",
                Axis::Vertical => "vertical",
            };
            let mut code = format!("::iced::widget::rule::{axis}({thickness} as f32)");
            append_rule_options(&mut code, options, env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::QrCode {
            data,
            cell_size,
            total_size,
            cell,
            background,
            ..
        } => {
            let mut code = format!("::iced::widget::qr_code(&self.{data})");
            // QR v40 has 177 cells plus four quiet-zone cells; spacing counts gaps.
            if let Some(value) = cell_size {
                write!(
                    code,
                    ".cell_size(::ui_lang_runtime::bounded_spacing({}, 182))",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(value) = total_size {
                write!(
                    code,
                    ".total_size(::ui_lang_runtime::bounded_spacing({}, 3))",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if cell.is_some() || background.is_some() {
                let cell = cell.as_deref().map(|value| theme_color(document, value));
                let background = background
                    .as_deref()
                    .map(|value| theme_color(document, value));
                write!(
                    code,
                    ".style(|theme| {{ let default = ::iced::widget::qr_code::default(theme); ::iced::widget::qr_code::Style {{ cell: {}, background: {} }} }})",
                    cell.unwrap_or_else(|| "default.cell".into()),
                    background.unwrap_or_else(|| "default.background".into())
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Space { width, height, .. } => {
            let mut code = String::from("::iced::widget::space()");
            append_dimensions(&mut code, [width, height], env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::Component {
            name,
            args,
            id,
            slots,
            route,
            span,
        } => {
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            let mut component_env = HashMap::new();
            for (param, ty) in &component.params {
                let arg = args
                    .iter()
                    .find(|arg| &arg.name == param)
                    .expect("checker requires every component prop");
                let state = match &arg.value {
                    Expr::Path(path) if path.len() == 1 => {
                        env.get(&path[0]).and_then(|binding| binding.state.clone())
                    }
                    _ => None,
                };
                component_env.insert(
                    param.clone(),
                    Binding {
                        code: expr_code(&arg.value, env, document, ValueMode::Borrowed)?,
                        ty: ty.clone(),
                        local: false,
                        state,
                    },
                );
            }
            if let Some(route) = route {
                component_env.insert(
                    component_output_key(name),
                    Binding {
                        code: route_callback_code(
                            route, "__value", "__value", env, document, message,
                        )?,
                        ty: component.output.clone(),
                        local: true,
                        state: None,
                    },
                );
            }
            let component_scope = id.as_ref().map_or_else(
                || {
                    let scope = reconciliation_scope(scope, env);
                    format!("format!(\"{{}}/{}@{}\", {scope})", name, span.line)
                },
                |id| id_code(id, scope, env, document).unwrap_or_else(|_| scope.into()),
            );
            set_reconciliation_scope(&mut component_env, component_scope.clone());
            let scope_binding = component_scope_binding(name, span.line);
            if !component.states.is_empty() || !component.handlers.is_empty() {
                let field = component_state_field(name);
                for state in &component.states {
                    component_env.insert(
                        state.name.clone(),
                        Binding {
                            code: format!(
                                "self.{field}.get(&{scope_binding}).map_or_else(|| {}, |__state| __state.{}.clone())",
                                initial_code(state, document),
                                state.name
                            ),
                            ty: state.ty.clone(),
                            local: true,
                            state: Some(StateBinding::Component {
                                component: name.clone(),
                                name: state.name.clone(),
                                scope: scope_binding.clone(),
                            }),
                        },
                    );
                }
                component_env.insert(
                    component_context_key(name),
                    Binding {
                        code: scope_binding.clone(),
                        ty: Type::Unit,
                        local: true,
                        state: None,
                    },
                );
            }
            let component_slots = (!slots.is_empty()).then(|| SlotContext {
                entries: slots
                    .iter()
                    .map(|component_slot| SlotContent {
                        name: component_slot.name.clone(),
                        node: (*component_slot.content).clone(),
                        env: env.clone(),
                    })
                    .collect(),
                parent: slot.cloned().map(Box::new),
            });
            let render_scope = if component.states.is_empty() && component.handlers.is_empty() {
                component_scope.clone()
            } else {
                format!("{scope_binding}.clone()")
            };
            let rendered = render_node(
                &component.root,
                document,
                message,
                &component_env,
                &render_scope,
                component_slots.as_ref(),
            )?;
            let rendered = if component.states.is_empty() && component.handlers.is_empty() {
                rendered
            } else {
                format!("{{ let {scope_binding} = {component_scope}; {rendered} }}")
            };
            Ok(format!(
                "(|| {{ let __component_content: __IceElement<'_, {message}> = {rendered}; __component_content }})()"
            ))
        }
        ViewNode::Slot { name, span } => {
            let slot = slot.ok_or_else(|| {
                Error::new(
                    "E170",
                    span,
                    "slot reached codegen without component content",
                )
            })?;
            let content = slot
                .entries
                .iter()
                .find(|entry| entry.name == *name)
                .ok_or_else(|| {
                    Error::new(
                        "E170",
                        span,
                        format!("slot `{name}` reached codegen without component content"),
                    )
                })?;
            let mut content_env = content.env.clone();
            set_reconciliation_scope(&mut content_env, scope.to_owned());
            let rendered = render_node(
                &content.node,
                document,
                message,
                &content_env,
                scope,
                slot.parent.as_deref(),
            )?;
            Ok(format!(
                "(|| {{ let __slot_content: __IceElement<'_, {message}> = {rendered}; __slot_content }})()"
            ))
        }
        ViewNode::ExternComponent {
            function,
            args,
            route,
            span,
            ..
        } => {
            let component = find_extern_function(document, function, ExternKind::Component)
                .ok_or_else(|| {
                    Error::new(
                        "E130",
                        span,
                        format!("unknown extern component `{function}`"),
                    )
                })?;
            let args = args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    if component.borrowed[index] {
                        let arg = expr_code(arg, env, document, ValueMode::Borrowed)?;
                        let borrow = if matches!(
                            component.params[index].1,
                            Type::Str | Type::Bytes | Type::List(_)
                        ) {
                            "::std::convert::AsRef::as_ref"
                        } else {
                            "::std::borrow::Borrow::borrow"
                        };
                        Ok(format!("{borrow}(&({arg}))"))
                    } else {
                        expr_code(arg, env, document, ValueMode::Owned)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mapped = if let Some(route) = route {
                route_callback_code(route, "__value", "__value", env, document, message)?
            } else {
                format!("move |__value| {message}::__ExternNoop")
            };
            Ok(format!(
                "{}({args}).map({mapped}).into()",
                component.rust_path
            ))
        }
        ViewNode::Themer {
            function,
            args,
            route,
            span,
            ..
        } => {
            let themer =
                find_extern_function(document, function, ExternKind::Themer).ok_or_else(|| {
                    Error::new("E130", span, format!("unknown extern themer `{function}`"))
                })?;
            let args = expr_list_code(args, env, document)?;
            let mapped = if let Some(route) = route {
                route_callback_code(route, "__value", "__value", env, document, message)?
            } else {
                format!("move |__value| {message}::__ExternNoop")
            };
            Ok(format!(
                "{{ let (__theme, __content, __text_color, __background) = {}({args}); let mut __themer = ::iced::widget::themer(__theme, __content); if let ::std::option::Option::Some(__text_color) = __text_color {{ __themer = __themer.text_color(__text_color); }} if let ::std::option::Option::Some(__background) = __background {{ __themer = __themer.background(__background); }} let __themed: __IceElement<'_, {}> = __themer.into(); __themed.map({mapped}).into() }}",
                themer.rust_path,
                themer.output.rust(&document.structs)
            ))
        }
        ViewNode::Shader {
            function,
            args,
            width,
            height,
            route,
            span,
            ..
        } => {
            let shader = find_extern_function(document, function, ExternKind::Shader)
                .ok_or_else(|| Error::new("E191", span, format!("unknown shader `{function}`")))?;
            let args = expr_list_code(args, env, document)?;
            let mut code = format!("::iced::widget::Shader::new({}({args}))", shader.rust_path);
            append_dimensions(&mut code, [width, height], env, document)?;
            let output = shader.output.rust(&document.structs);
            let mapped = if let Some(route) = route {
                route_callback_code(route, "__value", "__value", env, document, message)?
            } else {
                format!("move |__value| {message}::__ExternNoop")
            };
            Ok(format!(
                "{{ let __shader: __IceElement<'_, {output}> = {code}.into(); __shader.map({mapped}).into() }}"
            ))
        }
        _ => return Ok(None),
    }?;
    let rendered = identify_rendered(rendered, id, message, env, document, scope)?;
    Ok(Some(rendered))
}
