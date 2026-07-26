use super::*;

pub(crate) fn controlled_state_bindings(
    document: &Document,
    editors: bool,
) -> Result<Vec<String>, Error> {
    fn collect(
        node: &ViewNode,
        document: &Document,
        editors: bool,
        env: &HashMap<String, Option<String>>,
        components: &mut HashSet<String>,
        output: &mut Vec<String>,
    ) -> Result<(), Error> {
        let binding = match node {
            ViewNode::Input { binding, span, .. } if !editors => Some((binding, "input", span)),
            ViewNode::TextEditor { binding, span, .. } if editors => {
                Some((binding, "editor", span))
            }
            _ => None,
        };
        if let Some((binding, widget, span)) = binding {
            let state = env.get(binding).ok_or_else(|| {
                Error::new(
                    "E139",
                    span,
                    format!("{widget} binding must resolve to an app state"),
                )
            })?;
            if let Some(state) = state
                && !output.contains(state)
            {
                output.push(state.clone());
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
                for (param, _) in &component.params {
                    let arg = args
                        .iter()
                        .find(|arg| &arg.name == param)
                        .expect("checker validates named component arguments");
                    if let Expr::Path(path) = &arg.value
                        && path.len() == 1
                        && let Some(state) = env.get(&path[0])
                    {
                        component_env.insert(param.clone(), state.clone());
                    }
                }
                component_env.extend(
                    component
                        .states
                        .iter()
                        .map(|state| (state.name.clone(), None)),
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
        .map(|state| (state.name.clone(), Some(state.name.clone())))
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
    Ok(output)
}

pub(in crate::check) fn pane_grid_span(node: &ViewNode) -> Option<&Span> {
    match node {
        ViewNode::PaneGrid { span, .. } => Some(span),
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => children.iter().find_map(pane_grid_span),
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

pub(in crate::check) fn check_qr_data(document: &Document) -> Result<(), Error> {
    for qr in &document.qr_codes {
        let valid = match qr.version {
            None | Some(QrVersion::Normal(1..=40)) | Some(QrVersion::Micro(1..=4)) => true,
            Some(QrVersion::Normal(_) | QrVersion::Micro(_)) => false,
        };
        if !valid {
            return Err(Error::new(
                "E136",
                &qr.span,
                "qr version must be normal(1..40) or micro(1..4)",
            ));
        }
        let data = match &qr.data {
            QrPayload::Text(value) => value.as_bytes(),
            QrPayload::Bytes(value) => value.as_slice(),
        };
        let correction = match qr.correction.unwrap_or(QrCorrection::Medium) {
            QrCorrection::Low => qrcode::EcLevel::L,
            QrCorrection::Medium => qrcode::EcLevel::M,
            QrCorrection::Quartile => qrcode::EcLevel::Q,
            QrCorrection::High => qrcode::EcLevel::H,
        };
        let encoded = match qr.version {
            Some(QrVersion::Normal(version)) => qrcode::QrCode::with_version(
                data,
                qrcode::Version::Normal(i16::from(version)),
                correction,
            ),
            Some(QrVersion::Micro(version)) => qrcode::QrCode::with_version(
                data,
                qrcode::Version::Micro(i16::from(version)),
                correction,
            ),
            None if qr.correction.is_some() => {
                qrcode::QrCode::with_error_correction_level(data, correction)
            }
            None => qrcode::QrCode::new(data),
        };
        if let Err(error) = encoded {
            return Err(Error::new(
                "E136",
                &qr.span,
                format!("cannot encode qr data `{}`: {error}", qr.name),
            ));
        }
    }
    Ok(())
}

pub(in crate::check) fn check_theme(document: &Document) -> Result<(), Error> {
    for required in ["bg", "fg", "primary", "danger"] {
        if !document.theme.contains_key(required) {
            return Err(Error::new(
                "E110",
                &Span::line(1),
                format!("theme is missing `{required}`"),
            ));
        }
    }
    Ok(())
}
