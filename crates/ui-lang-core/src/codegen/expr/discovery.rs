use super::*;

pub(in crate::codegen) fn initial_code(state: &State, document: &Document) -> String {
    let Type::Animation(inner) = &state.ty else {
        return initial_value_code(&state.initial, &state.ty, document);
    };
    let mut code = initial_value_code(&state.initial, inner, document);
    if **inner == Type::F64 {
        code = format!("({code}) as f32");
    }
    code = format!("::iced::Animation::new({code})");
    let options = state.animation.as_ref().expect("parsed animation options");
    if let Some(easing) = &options.easing {
        let easing = if ANIMATION_EASINGS.contains(&easing.as_str()) {
            format!("::iced::animation::Easing::{}", pascal(easing))
        } else {
            let function = find_extern_function(document, easing, ExternKind::Sync)
                .expect("checked custom animation easing");
            format!(
                "::iced::animation::Easing::Custom(|__value: f32| {}(__value as f64) as f32)",
                function.rust_path
            )
        };
        code.push_str(&format!(".easing({easing})"));
    }
    if let Some(duration) = options.duration {
        code.push_str(match duration {
            AnimationDuration::VeryQuick => ".very_quick()",
            AnimationDuration::Quick => ".quick()",
            AnimationDuration::Slow => ".slow()",
            AnimationDuration::VerySlow => ".very_slow()",
            AnimationDuration::Milliseconds(milliseconds) => {
                return format!(
                    "{code}.duration(::std::time::Duration::from_millis({milliseconds})){}",
                    animation_tail(options)
                );
            }
        });
    }
    format!("{code}{}", animation_tail(options))
}

pub(in crate::codegen) fn animation_tail(options: &AnimationOptions) -> String {
    let mut code = String::new();
    if let Some(milliseconds) = options.delay_ms {
        code.push_str(&format!(
            ".delay(::std::time::Duration::from_millis({milliseconds}))"
        ));
    }
    if options.repeat_forever {
        code.push_str(".repeat_forever()");
    } else if let Some(repeat) = options.repeat {
        code.push_str(&format!(".repeat({repeat})"));
    }
    if options.auto_reverse {
        code.push_str(".auto_reverse()");
    }
    code
}

pub(in crate::codegen) fn initial_value_code(
    expr: &Expr,
    ty: &Type,
    document: &Document,
) -> String {
    match (expr, ty) {
        (Expr::Str(value), Type::Str) => format!("{}.to_owned()", rust_string(value)),
        (Expr::Str(value), Type::Markdown) => format!(
            "::iced::widget::markdown::Content::parse({})",
            rust_string(value)
        ),
        (Expr::Str(value), Type::Editor) => format!(
            "::iced::widget::text_editor::Content::with_text({})",
            rust_string(value)
        ),
        (Expr::EmptyList, Type::List(_)) => "::std::vec::Vec::new()".into(),
        (Expr::EmptyList, Type::Combo(_)) => {
            "::iced::widget::combo_box::State::new(::std::vec::Vec::new())".into()
        }
        (Expr::List(values), Type::Combo(_)) => format!(
            "::iced::widget::combo_box::State::new(::std::vec![{}])",
            values
                .iter()
                .map(|value| {
                    expr_code(value, &HashMap::new(), document, ValueMode::Owned)
                        .unwrap_or_else(|_| "::core::default::Default::default()".into())
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        (Expr::None, Type::Option(_)) => "::std::option::Option::None".into(),
        (Expr::Bool(value), _) => value.to_string(),
        (Expr::I64(value), _) => value.to_string(),
        (Expr::F64(value), _) => rust_f64(*value),
        _ => expr_code(expr, &HashMap::new(), document, ValueMode::Owned)
            .unwrap_or_else(|_| "::core::default::Default::default()".into()),
    }
}

pub(in crate::codegen) fn pane_field(name: &str) -> String {
    if canonical_snake(name) && !name.ends_with("_splits") {
        format!("__pane_{name}")
    } else {
        format!("__pane_0{}", rust_identifier_hex(name))
    }
}

pub(in crate::codegen) fn pane_splits_field(name: &str) -> String {
    format!("{}_splits", pane_field(name))
}

pub(in crate::codegen) fn pane_type(name: &str) -> String {
    if canonical_snake(name) {
        format!("__IcePane{}", pascal(name))
    } else {
        format!("__Ice0P{}", rust_identifier_hex(name))
    }
}

pub(in crate::codegen) fn pane_template_variant(name: &str) -> String {
    if canonical_snake(name) {
        pascal(name)
    } else {
        format!("__0T{}", rust_identifier_hex(name))
    }
}

pub(in crate::codegen) fn pane_template_types(
    template: &PaneTemplate,
    document: &Document,
) -> Result<(Type, Type), Error> {
    let state = document
        .states
        .iter()
        .find(|state| state.name == template.items)
        .expect("checker validates dynamic pane state");
    let Type::List(item_type) = &state.ty else {
        unreachable!("checker validates dynamic pane lists")
    };
    let mut env = document
        .states
        .iter()
        .map(|state| (state.name.clone(), state.ty.clone()))
        .collect::<HashMap<_, _>>();
    env.insert(template.item.clone(), (**item_type).clone());
    Ok((
        (**item_type).clone(),
        expr_type(&template.key, &env, document, &template.span)?,
    ))
}

pub(in crate::codegen) fn generate_pane_types(
    out: &mut String,
    document: &Document,
) -> Result<(), Error> {
    for (node, test_only) in document_pane_grids(document) {
        let ViewNode::PaneGrid {
            name, templates, ..
        } = node
        else {
            unreachable!()
        };
        if templates.is_empty() {
            continue;
        }
        let pane_type = pane_type(name);
        let cfg = if test_only { "#[cfg(test)]\n" } else { "" };
        writeln!(
            out,
            "{cfg}#[derive(Debug, Clone, PartialEq)]\nenum {pane_type} {{\n__Static(&'static str),"
        )
        .unwrap();
        for template in templates {
            let (_, key_type) = pane_template_types(template, document)?;
            writeln!(
                out,
                "{}({}),",
                pane_template_variant(&template.item),
                key_type.rust(&document.structs)
            )
            .unwrap();
        }
        writeln!(out, "}}\n{cfg}impl {pane_type} {{\nfn __name(&self) -> ::std::string::String {{\nmatch self {{\nSelf::__Static(__name) => (*__name).to_owned(),").unwrap();
        for template in templates {
            writeln!(
                out,
                "Self::{}(__key) => ::std::format!({}, __key),",
                pane_template_variant(&template.item),
                rust_string(&format!("{}({{}})", template.item))
            )
            .unwrap();
        }
        writeln!(out, "}}\n}}\n}}").unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn pane_reference_find_code(
    reference: &PaneReference,
    grid: &str,
    state: &str,
    dynamic: bool,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let field = pane_field(grid);
    if dynamic {
        let value = pane_reference_value_code(reference, grid, true, env, document)?;
        return Ok(format!(
            "{{ let __value = {value}; {state}.{field}.iter().find_map(|(__pane, __pane_value)| (__pane_value == &__value).then_some(*__pane)) }}"
        ));
    }
    Ok(match reference {
        PaneReference::Static(name) => format!(
            "{state}.{field}.iter().find_map(|(__pane, __name)| (*__name == {}).then_some(*__pane))",
            rust_string(name)
        ),
        PaneReference::Dynamic { .. } => unreachable!("dynamic pane requires a template"),
    })
}

pub(in crate::codegen) fn pane_reference_value_code(
    reference: &PaneReference,
    grid: &str,
    dynamic: bool,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match reference {
        PaneReference::Static(name) if dynamic => {
            format!("{}::__Static({})", pane_type(grid), rust_string(name))
        }
        PaneReference::Static(name) => rust_string(name),
        PaneReference::Dynamic { template, key } => format!(
            "{}::{}({})",
            pane_type(grid),
            pane_template_variant(template),
            expr_code(key, env, document, ValueMode::Owned)?
        ),
    })
}

pub(in crate::codegen) fn pane_split_slots(configuration: &PaneConfiguration) -> Vec<Option<&str>> {
    fn collect<'a>(configuration: &'a PaneConfiguration, output: &mut Vec<Option<&'a str>>) {
        if let PaneConfiguration::Split { name, a, b, .. } = configuration {
            output.push(name.as_deref());
            collect(b, output);
            collect(a, output);
        }
    }

    let mut output = Vec::new();
    collect(configuration, &mut output);
    output
}

pub(in crate::codegen) fn pane_configuration_code(
    configuration: &PaneConfiguration,
    pane_type: Option<&str>,
) -> String {
    match configuration {
        PaneConfiguration::Pane(name) => {
            let value = pane_type.map_or_else(
                || rust_string(name),
                |pane_type| format!("{pane_type}::__Static({})", rust_string(name)),
            );
            format!("::iced::widget::pane_grid::Configuration::Pane({value})")
        }
        PaneConfiguration::Split {
            axis, ratio, a, b, ..
        } => {
            let axis = match axis {
                PaneAxis::Horizontal => "Horizontal",
                PaneAxis::Vertical => "Vertical",
            };
            format!(
                "::iced::widget::pane_grid::Configuration::Split {{ axis: ::iced::widget::pane_grid::Axis::{axis}, ratio: {ratio:?}, a: ::std::boxed::Box::new({}), b: ::std::boxed::Box::new({}) }}",
                pane_configuration_code(a, pane_type),
                pane_configuration_code(b, pane_type)
            )
        }
    }
}

pub(in crate::codegen) fn pane_resize_variant(name: &str) -> String {
    if canonical_snake(name) {
        format!("__Pane{}Resize", pascal(name))
    } else {
        format!("__0P{}R", rust_identifier_hex(name))
    }
}

pub(in crate::codegen) fn pane_drag_variant(name: &str) -> String {
    if canonical_snake(name) {
        format!("__Pane{}Drag", pascal(name))
    } else {
        format!("__0P{}D", rust_identifier_hex(name))
    }
}

pub(in crate::codegen) fn pane_grids(root: &ViewNode) -> Vec<&ViewNode> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<&'a ViewNode>) {
        match node {
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                output.push(node);
                for pane in panes {
                    for node in pane.nodes() {
                        collect(node, output);
                    }
                }
                for template in templates {
                    for node in template.pane.nodes() {
                        collect(node, output);
                    }
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Match { arms, .. } => {
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(root, &mut output);
    output
}

pub(in crate::codegen) fn document_pane_grids(document: &Document) -> Vec<(&ViewNode, bool)> {
    fn statements_reference_grid(statements: &[Statement], name: &str) -> bool {
        statements.iter().any(|statement| match statement {
            Statement::PaneOperation { grid, .. } => grid == name,
            Statement::TaskGroup { statements, .. } => statements_reference_grid(statements, name),
            Statement::Abortable { task, .. } => {
                statements_reference_grid(::std::slice::from_ref(task), name)
            }
            _ => false,
        })
    }

    let referenced = |name: &str| {
        document
            .handlers
            .iter()
            .any(|handler| statements_reference_grid(&handler.statements, name))
            || document
                .presets
                .iter()
                .any(|preset| statements_reference_grid(&preset.statements, name))
    };
    pane_grids(&document.view)
        .into_iter()
        .map(|node| (node, false))
        .chain(
            document
                .tests
                .iter()
                .filter_map(|test| test.mount.as_ref())
                .flat_map(pane_grids)
                .map(|node| {
                    let ViewNode::PaneGrid { name, .. } = node else {
                        unreachable!()
                    };
                    (node, !referenced(name))
                }),
        )
        .collect()
}

pub(in crate::codegen) fn uses_canvas(document: &Document) -> bool {
    !canvases(document).is_empty()
}

pub(in crate::codegen) fn canvases(
    document: &Document,
) -> Vec<(&CanvasOptions, &[State], &[CanvasEvent])> {
    fn collect<'a>(
        node: &'a ViewNode,
        output: &mut Vec<(&'a CanvasOptions, &'a [State], &'a [CanvasEvent])>,
    ) {
        match node {
            ViewNode::Canvas {
                options,
                locals,
                events,
                ..
            } => output.push((options, locals, events)),
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Match { arms, .. } => {
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for node in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    collect(node, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(&document.view, &mut output);
    for component in &document.components {
        collect(&component.root, &mut output);
    }
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        collect(mount, &mut output);
    }
    output
}

pub(in crate::codegen) fn canvas_cache_groups(document: &Document) -> Vec<&str> {
    let mut groups = Vec::new();
    for group in canvases(document)
        .into_iter()
        .filter_map(|(options, _, _)| options.cache_group.as_deref())
    {
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    groups
}

pub(in crate::codegen) fn canvas_events(document: &Document) -> Vec<&CanvasEvent> {
    canvases(document)
        .into_iter()
        .flat_map(|(_, _, events)| events)
        .collect()
}

pub(in crate::codegen) fn canvas_group_symbol(group: &str) -> String {
    if canonical_snake(group) {
        format!("__ICE_CANVAS_GROUP_{}", group.to_ascii_uppercase())
    } else {
        format!("__ICE_CANVAS_GROUP_0{}", rust_identifier_hex(group))
    }
}

pub(in crate::codegen) fn needs_extern_noop(document: &Document) -> bool {
    fn contains(node: &ViewNode) -> bool {
        match node {
            ViewNode::ExternComponent { route: None, .. }
            | ViewNode::Themer { route: None, .. }
            | ViewNode::Shader { route: None, .. } => true,
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => children.iter().any(contains),
            ViewNode::Match { arms, .. } => arms.iter().flat_map(|arm| &arm.children).any(contains),
            ViewNode::Tooltip { content, tip, .. } => contains(content) || contains(tip),
            ViewNode::Overlay { .. } => true,
            ViewNode::PaneGrid {
                panes, templates, ..
            } => panes
                .iter()
                .flat_map(PaneView::nodes)
                .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                .any(contains),
            ViewNode::Table { columns, .. } => columns
                .iter()
                .any(|column| contains(&column.header) || contains(&column.cell)),
            ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. } => contains(content),
            ViewNode::Component { slots, .. } => slots.iter().any(|slot| contains(&slot.content)),
            ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => contains(child),
            ViewNode::Button {
                content: Some(content),
                ..
            } => contains(content),
            ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => contains(content),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    contains(narrow) || contains(wide)
                }
                ResponsiveContent::Size { content, .. } => contains(content),
            },
            _ => false,
        }
    }
    contains(&document.view)
        || document.components.iter().any(|item| contains(&item.root))
        || document
            .tests
            .iter()
            .filter_map(|test| test.mount.as_ref())
            .any(contains)
}
