use super::*;
use crate::hir::view_children;

#[derive(Clone)]
enum ControlledBinding {
    App(String),
    /// A component's own state entry, named by its owning component — an
    /// editor bound to one (directly, or through a chain of bind props)
    /// contracts with THAT component, not the app.
    Component {
        component: String,
        state: String,
    },
    Writable,
    ReadOnly(Type),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ControlledEditorBinding {
    pub name: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentControlledEditorBinding {
    pub component: String,
    pub state: String,
    pub action: Option<String>,
}

#[derive(Default)]
pub(crate) struct ControlledEditors {
    pub(crate) app: Vec<ControlledEditorBinding>,
    pub(crate) component: Vec<ComponentControlledEditorBinding>,
}

#[derive(Default)]
struct ControlledOutput {
    bindings: Vec<ControlledEditorBinding>,
    by_name: HashMap<String, usize>,
    component_bindings: Vec<ComponentControlledEditorBinding>,
    component_by_key: HashMap<(String, String), usize>,
}

pub(crate) fn controlled_state_bindings(
    document: &Document,
    editors: bool,
) -> Result<Vec<String>, Error> {
    Ok(controlled_bindings(document, editors)?
        .bindings
        .into_iter()
        .map(|binding| binding.name)
        .collect())
}

pub(crate) fn controlled_editor_bindings(document: &Document) -> Result<ControlledEditors, Error> {
    let output = controlled_bindings(document, true)?;
    Ok(ControlledEditors {
        app: output.bindings,
        component: output.component_bindings,
    })
}

fn controlled_bindings(document: &Document, editors: bool) -> Result<ControlledOutput, Error> {
    fn collect(
        node: &ViewNode,
        document: &Document,
        editors: bool,
        env: &HashMap<String, ControlledBinding>,
        components: &mut HashSet<String>,
        output: &mut ControlledOutput,
    ) -> Result<(), Error> {
        let binding = match node {
            ViewNode::Input { binding, span, .. }
                if !editors
                    && !document
                        .secrets
                        .iter()
                        .any(|secret| secret.name == *binding) =>
            {
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
                    if let Some(existing) = output
                        .by_name
                        .get(state)
                        .and_then(|index| output.bindings.get(*index))
                    {
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
                        let index = output.bindings.len();
                        output.by_name.insert(state.clone(), index);
                        output.bindings.push(ControlledEditorBinding {
                            name: state.clone(),
                            action,
                        });
                    }
                }
                Some(ControlledBinding::Component { component, state }) => {
                    // Inputs over component `str` state route through the
                    // per-state binding variant and need no contract here;
                    // editors contract with their owning component so codegen
                    // knows the one action adapter to apply.
                    if editors {
                        let key = (component.clone(), state.clone());
                        if let Some(existing) = output
                            .component_by_key
                            .get(&key)
                            .and_then(|index| output.component_bindings.get(*index))
                        {
                            if existing.action != action {
                                return Err(Error::new(
                                    "E139",
                                    span,
                                    format!(
                                        "editor state `{component}.{state}` must use the same action adapter everywhere"
                                    ),
                                ));
                            }
                        } else {
                            let index = output.component_bindings.len();
                            output.component_by_key.insert(key, index);
                            output
                                .component_bindings
                                .push(ComponentControlledEditorBinding {
                                    component: component.clone(),
                                    state: state.clone(),
                                    action,
                                });
                        }
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
            ViewNode::Lazy {
                keys,
                binding,
                child,
                ..
            } => {
                let mut child_env = env.clone();
                child_env.remove(binding);
                for key in keys {
                    if let Expr::Path(segments) = key
                        && let [name] = segments.as_slice()
                    {
                        child_env.remove(name);
                    }
                }
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
                component_env.extend(component.states.iter().map(|state| {
                    (
                        state.name.clone(),
                        ControlledBinding::Component {
                            component: name.clone(),
                            state: state.name.clone(),
                        },
                    )
                }));
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
    let mut output = ControlledOutput::default();
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
            .chain(component.states.iter().map(|state| {
                (
                    state.name.clone(),
                    ControlledBinding::Component {
                        component: component.name.clone(),
                        state: state.name.clone(),
                    },
                )
            }))
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
    if let ViewNode::PaneGrid { span, .. } = node {
        return Some(span);
    }
    view_children(node).into_iter().find_map(pane_grid_span)
}

pub(in crate::check) fn repeated_pane_grid_span(node: &ViewNode) -> Option<&Span> {
    match node {
        // Below a repeating scope any pane grid counts, not just one repeated
        // directly, so the search widens here and never narrows again.
        ViewNode::For { children, .. } => children.iter().find_map(pane_grid_span),
        ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => pane_grid_span(child),
        ViewNode::Table { columns, .. } => columns.iter().find_map(|column| {
            pane_grid_span(&column.header).or_else(|| pane_grid_span(&column.cell))
        }),
        _ => view_children(node)
            .into_iter()
            .find_map(repeated_pane_grid_span),
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
