use super::*;

#[derive(Clone)]
enum ControlledBinding {
    App(String),
    Writable,
    ReadOnly(Type),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ControlledEditorBinding {
    pub name: String,
    pub action: Option<String>,
}

pub(crate) fn controlled_state_bindings(
    document: &Document,
    editors: bool,
) -> Result<Vec<String>, Error> {
    Ok(controlled_bindings(document, editors)?
        .into_iter()
        .map(|binding| binding.name)
        .collect())
}

pub(crate) fn controlled_editor_bindings(
    document: &Document,
) -> Result<Vec<ControlledEditorBinding>, Error> {
    controlled_bindings(document, true)
}

fn controlled_bindings(
    document: &Document,
    editors: bool,
) -> Result<Vec<ControlledEditorBinding>, Error> {
    fn collect(
        node: &ViewNode,
        document: &Document,
        editors: bool,
        env: &HashMap<String, ControlledBinding>,
        components: &mut HashSet<String>,
        output: &mut Vec<ControlledEditorBinding>,
    ) -> Result<(), Error> {
        let binding = match node {
            ViewNode::Input { binding, span, .. } if !editors => {
                Some((binding, "input", span, None))
            }
            ViewNode::TextEditor {
                binding,
                options,
                span,
                ..
            } if editors => Some((
                binding,
                "editor",
                span,
                options
                    .action
                    .as_ref()
                    .map(|action| action.function.clone()),
            )),
            _ => None,
        };
        if let Some((binding, widget, span, action)) = binding {
            match env.get(binding) {
                Some(ControlledBinding::App(state)) => {
                    if let Some(existing) = output.iter().find(|binding| binding.name == *state) {
                        if existing.action != action {
                            return Err(Error::new(
                                "E139",
                                span,
                                format!(
                                    "editor state `{state}` must use the same action adapter everywhere"
                                ),
                            ));
                        }
                    } else {
                        output.push(ControlledEditorBinding {
                            name: state.clone(),
                            action,
                        });
                    }
                }
                Some(ControlledBinding::Writable) => {}
                Some(ControlledBinding::ReadOnly(ty)) => {
                    return Err(Error::new(
                        "E139",
                        span,
                        format!("component prop `{binding}` is read-only and cannot be bound"),
                    )
                    .hint(format!("declare it as `bind {binding}:{}`", ty.display())));
                }
                None => {
                    return Err(Error::new(
                        "E139",
                        span,
                        format!("{widget} binding must resolve to writable state"),
                    ));
                }
            }
            return Ok(());
        }

        match node {
            ViewNode::Layout { children, .. } | ViewNode::If { children, .. } => {
                for child in children {
                    collect(child, document, editors, env, components, output)?;
                }
            }
            ViewNode::For { item, children, .. } => {
                let mut child_env = env.clone();
                child_env.remove(item);
                for child in children {
                    collect(child, document, editors, &child_env, components, output)?;
                }
            }
            ViewNode::Match { arms, .. } => {
                for arm in arms {
                    let mut child_env = env.clone();
                    if let Some(binding) = arm.pattern.binding() {
                        child_env.remove(binding);
                    }
                    for child in &arm.children {
                        collect(child, document, editors, &child_env, components, output)?;
                    }
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            }
            | ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => {
                collect(content, document, editors, env, components, output)?;
            }
            ViewNode::KeyedColumn { item, child, .. } => {
                let mut child_env = env.clone();
                child_env.remove(item);
                collect(child, document, editors, &child_env, components, output)?;
            }
            ViewNode::Lazy { binding, child, .. } => {
                let mut child_env = env.clone();
                child_env.remove(binding);
                collect(child, document, editors, &child_env, components, output)?;
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, document, editors, env, components, output)?;
                collect(tip, document, editors, env, components, output)?;
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, document, editors, env, components, output)?;
                collect(layer, document, editors, env, components, output)?;
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for pane in panes {
                    let mut child_env = env.clone();
                    if let Some(binding) = &pane.maximized {
                        child_env.remove(binding);
                    }
                    for child in pane.nodes() {
                        collect(child, document, editors, &child_env, components, output)?;
                    }
                }
                for template in templates {
                    let mut child_env = env.clone();
                    child_env.remove(&template.item);
                    if let Some(binding) = &template.pane.maximized {
                        child_env.remove(binding);
                    }
                    for child in template.pane.nodes() {
                        collect(child, document, editors, &child_env, components, output)?;
                    }
                }
            }
            ViewNode::Table { item, columns, .. } => {
                let mut cell_env = env.clone();
                cell_env.remove(item);
                for column in columns {
                    collect(&column.header, document, editors, env, components, output)?;
                    collect(
                        &column.cell,
                        document,
                        editors,
                        &cell_env,
                        components,
                        output,
                    )?;
                }
            }
            ViewNode::Component {
                name,
                args,
                slots,
                span,
                ..
            } => {
                for slot in slots {
                    collect(&slot.content, document, editors, env, components, output)?;
                }
                if !components.insert(name.clone()) {
                    return Err(Error::new(
                        "E122",
                        span,
                        format!("recursive component `{name}` cannot contain controlled state"),
                    ));
                }
                let component = document
                    .components
                    .iter()
                    .find(|item| item.name == *name)
                    .expect("checker validates component names");
                let mut component_env = HashMap::new();
                for param in &component.params {
                    if !param.bind {
                        component_env.insert(
                            param.name.clone(),
                            ControlledBinding::ReadOnly(param.ty.clone()),
                        );
                        continue;
                    }
                    let arg = args
                        .iter()
                        .find(|arg| arg.name == param.name)
                        .expect("checker requires every bind prop");
                    let Expr::Path(path) = &arg.value else {
                        return Err(Error::new(
                            "E139",
                            span,
                            format!(
                                "component `{name}` bind prop `{}` requires a direct writable state",
                                param.name
                            ),
                        )
                        .hint(format!("pass `{}<->state`", param.name)));
                    };
                    let [binding] = path.as_slice() else {
                        return Err(Error::new(
                            "E139",
                            span,
                            format!(
                                "component `{name}` bind prop `{}` requires a direct writable state",
                                param.name
                            ),
                        )
                        .hint(format!("pass `{}<->state`", param.name)));
                    };
                    let source = env.get(binding).ok_or_else(|| {
                        Error::new(
                            "E139",
                            span,
                            format!(
                                "component `{name}` bind prop `{}` requires a direct writable state",
                                param.name
                            ),
                        )
                        .hint("pass app state, component state, or another bind prop")
                    })?;
                    if let ControlledBinding::ReadOnly(ty) = source {
                        return Err(Error::new(
                            "E139",
                            span,
                            format!(
                                "component prop `{binding}` is read-only and cannot be forwarded"
                            ),
                        )
                        .hint(format!("declare it as `bind {binding}:{}`", ty.display())));
                    }
                    component_env.insert(param.name.clone(), source.clone());
                }
                component_env.extend(
                    component
                        .states
                        .iter()
                        .map(|state| (state.name.clone(), ControlledBinding::Writable)),
                );
                collect(
                    &component.root,
                    document,
                    editors,
                    &component_env,
                    components,
                    output,
                )?;
                components.remove(name);
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, document, editors, env, components, output)?;
                    collect(wide, document, editors, env, components, output)?;
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.remove(width);
                    child_env.remove(height);
                    collect(content, document, editors, &child_env, components, output)?;
                }
            },
            _ => {}
        }
        Ok(())
    }

    let env = document
        .states
        .iter()
        .map(|state| {
            (
                state.name.clone(),
                ControlledBinding::App(state.name.clone()),
            )
        })
        .collect();
    let mut output = Vec::new();
    collect(
        &document.view,
        document,
        editors,
        &env,
        &mut HashSet::new(),
        &mut output,
    )?;
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        collect(
            mount,
            document,
            editors,
            &env,
            &mut HashSet::new(),
            &mut output,
        )?;
    }
    for component in &document.components {
        let env = component
            .params
            .iter()
            .map(|param| {
                (
                    param.name.clone(),
                    if param.bind {
                        ControlledBinding::Writable
                    } else {
                        ControlledBinding::ReadOnly(param.ty.clone())
                    },
                )
            })
            .chain(
                component
                    .states
                    .iter()
                    .map(|state| (state.name.clone(), ControlledBinding::Writable)),
            )
            .collect();
        collect(
            &component.root,
            document,
            editors,
            &env,
            &mut HashSet::new(),
            &mut output,
        )?;
    }
    Ok(output)
}

pub(in crate::check) fn pane_grid_span(node: &ViewNode) -> Option<&Span> {
    match node {
        ViewNode::PaneGrid { span, .. } => Some(span),
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => children.iter().find_map(pane_grid_span),
        ViewNode::Match { arms, .. } => arms
            .iter()
            .flat_map(|arm| &arm.children)
            .find_map(pane_grid_span),
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::ResizeHandle { content, .. }
        | ViewNode::Container { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => pane_grid_span(content),
        ViewNode::Tooltip { content, tip, .. } => {
            pane_grid_span(content).or_else(|| pane_grid_span(tip))
        }
        ViewNode::Overlay { content, layer, .. } => {
            pane_grid_span(content).or_else(|| pane_grid_span(layer))
        }
        ViewNode::Table { columns, .. } => columns.iter().find_map(|column| {
            pane_grid_span(&column.header).or_else(|| pane_grid_span(&column.cell))
        }),
        ViewNode::Component { slots, .. } => {
            slots.iter().find_map(|slot| pane_grid_span(&slot.content))
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                pane_grid_span(narrow).or_else(|| pane_grid_span(wide))
            }
            ResponsiveContent::Size { content, .. } => pane_grid_span(content),
        },
        _ => None,
    }
}

pub(in crate::check) fn repeated_pane_grid_span(node: &ViewNode) -> Option<&Span> {
    match node {
        ViewNode::For { children, .. } => children.iter().find_map(pane_grid_span),
        ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => pane_grid_span(child),
        ViewNode::Table { columns, .. } => columns.iter().find_map(|column| {
            pane_grid_span(&column.header).or_else(|| pane_grid_span(&column.cell))
        }),
        ViewNode::Layout { children, .. } | ViewNode::If { children, .. } => {
            children.iter().find_map(repeated_pane_grid_span)
        }
        ViewNode::Match { arms, .. } => arms
            .iter()
            .flat_map(|arm| &arm.children)
            .find_map(repeated_pane_grid_span),
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::ResizeHandle { content, .. }
        | ViewNode::Container { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. } => repeated_pane_grid_span(content),
        ViewNode::Tooltip { content, tip, .. } => {
            repeated_pane_grid_span(content).or_else(|| repeated_pane_grid_span(tip))
        }
        ViewNode::Overlay { content, layer, .. } => {
            repeated_pane_grid_span(content).or_else(|| repeated_pane_grid_span(layer))
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => panes
            .iter()
            .flat_map(PaneView::nodes)
            .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            .find_map(repeated_pane_grid_span),
        ViewNode::Component { slots, .. } => slots
            .iter()
            .find_map(|slot| repeated_pane_grid_span(&slot.content)),
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                repeated_pane_grid_span(narrow).or_else(|| repeated_pane_grid_span(wide))
            }
            ResponsiveContent::Size { content, .. } => repeated_pane_grid_span(content),
        },
        _ => None,
    }
}

/// Encodes a literal QR payload at compile time, so a payload that cannot fit
/// the requested version and error correction fails the build instead of the
/// frame. A payload minted at runtime cannot be encoded here; the widget
/// renders nothing when it turns out not to fit.
pub(in crate::check) fn check_qr_payload(
    payload: &Expr,
    correction: Option<QrCorrection>,
    version: Option<QrVersion>,
    span: &Span,
) -> Result<(), Error> {
    let valid = match version {
        None | Some(QrVersion::Normal(1..=40)) | Some(QrVersion::Micro(1..=4)) => true,
        Some(QrVersion::Normal(_) | QrVersion::Micro(_)) => false,
    };
    if !valid {
        return Err(Error::new(
            "E136",
            span,
            "qr version must be normal(1..40) or micro(1..4)",
        ));
    }
    let data = match payload {
        Expr::Str(value) => value.as_bytes(),
        Expr::Bytes(value) => value.as_slice(),
        _ => return Ok(()),
    };
    let level = match correction.unwrap_or(QrCorrection::Medium) {
        QrCorrection::Low => qrcode::EcLevel::L,
        QrCorrection::Medium => qrcode::EcLevel::M,
        QrCorrection::Quartile => qrcode::EcLevel::Q,
        QrCorrection::High => qrcode::EcLevel::H,
    };
    let encoded = match version {
        Some(QrVersion::Normal(version)) => {
            qrcode::QrCode::with_version(data, qrcode::Version::Normal(i16::from(version)), level)
        }
        Some(QrVersion::Micro(version)) => {
            qrcode::QrCode::with_version(data, qrcode::Version::Micro(i16::from(version)), level)
        }
        None if correction.is_some() => qrcode::QrCode::with_error_correction_level(data, level),
        None => qrcode::QrCode::new(data),
    };
    if let Err(error) = encoded {
        return Err(Error::new(
            "E136",
            span,
            format!("cannot encode qr payload: {error}"),
        ));
    }
    Ok(())
}

pub(in crate::check) fn check_theme(document: &Document) -> Result<(), Error> {
    let contract = document.theme_contract.as_ref().ok_or_else(|| {
        Error::new(
            "E110",
            &Span::line(1),
            "missing `theme contract Name` declaration",
        )
    })?;
    for required in ["bg", "fg", "primary", "danger"] {
        if !contract.tokens.iter().any(|token| token == required) {
            return Err(Error::new(
                "E110",
                &contract.span,
                format!("theme contract `{}` is missing `{required}`", contract.name),
            ));
        }
    }
    if document.palettes.is_empty() {
        return Err(Error::new(
            "E110",
            &contract.span,
            format!(
                "theme contract `{}` requires at least one palette",
                contract.name
            ),
        ));
    }
    if contract.name == document.app
        || document
            .structs
            .iter()
            .any(|item| item.name == contract.name)
        || document.enums.iter().any(|item| item.name == contract.name)
    {
        return Err(Error::new(
            "E110",
            &contract.span,
            format!(
                "theme contract `{}` conflicts with another generated type",
                contract.name
            ),
        ));
    }
    let mut rust_variants = HashSet::new();
    for palette in &document.palettes {
        if palette.contract != contract.name {
            return Err(Error::new(
                "E110",
                &palette.span,
                format!(
                    "palette `{}` targets theme contract `{}`, not `{}`",
                    palette.name, palette.contract, contract.name
                ),
            ));
        }
        let rust_variant = palette
            .name
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            })
            .collect::<String>();
        if !rust_variants.insert(rust_variant) {
            return Err(Error::new(
                "E110",
                &palette.span,
                format!(
                    "palette `{}` conflicts with another generated palette variant",
                    palette.name
                ),
            ));
        }
        if let Some(token) = palette
            .colors
            .keys()
            .find(|token| !contract.tokens.contains(token))
        {
            return Err(Error::new(
                "E110",
                &palette.span,
                format!("palette `{}` has unknown token `{token}`", palette.name),
            ));
        }
        if let Some(token) = contract
            .tokens
            .iter()
            .find(|token| !palette.colors.contains_key(*token))
        {
            return Err(Error::new(
                "E110",
                &palette.span,
                format!("palette `{}` is missing token `{token}`", palette.name),
            ));
        }
    }
    Ok(())
}
