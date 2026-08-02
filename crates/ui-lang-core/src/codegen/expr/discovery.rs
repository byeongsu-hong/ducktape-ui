use super::*;
use crate::lower::{ResolvedAnimation, ResolvedAnimationEasing, ResolvedInitializer};

pub(in crate::codegen) fn resolved_initializer_code(
    initializer: &ResolvedInitializer,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let mut code = checked_expr_use_code(
        program,
        initializer.expression,
        &HashMap::new(),
        ValueMode::Owned,
    )?;
    let Some(options) = &initializer.animation else {
        return Ok(code);
    };
    if let Some(easing) = &options.easing {
        let easing = match easing {
            ResolvedAnimationEasing::Builtin(easing) => {
                format!("::iced::animation::Easing::{}", pascal(easing))
            }
            ResolvedAnimationEasing::Custom(function) => format!(
                "::iced::animation::Easing::Custom(|__value: f32| {}(__value as f64) as f32)",
                program.extern_function(*function).rust_path
            ),
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
                return Ok(format!(
                    "{code}.duration(::std::time::Duration::from_millis({milliseconds})){}",
                    animation_tail(options)
                ));
            }
        });
    }
    Ok(format!("{code}{}", animation_tail(options)))
}

fn animation_tail(options: &ResolvedAnimation) -> String {
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

pub(in crate::codegen) fn generate_pane_types(
    out: &mut String,
    program: &LoweredProgram,
) -> Result<(), Error> {
    for (pane, test_only) in document_pane_grids(program) {
        if pane.templates.is_empty() {
            continue;
        }
        let pane_type = pane_type(&pane.name);
        let cfg = if test_only { "#[cfg(test)]\n" } else { "" };
        writeln!(
            out,
            "{cfg}#[derive(Debug, Clone, PartialEq)]\npub(crate) enum {pane_type} {{\n__Static(&'static str),"
        )
        .unwrap();
        for template in &pane.templates {
            writeln!(
                out,
                "{}({}),",
                pane_template_variant(&template.item.name),
                template.key_type.rust(&program.document().structs)
            )
            .unwrap();
        }
        writeln!(out, "}}\n{cfg}impl {pane_type} {{\nfn __name(&self) -> ::std::string::String {{\nmatch self {{\nSelf::__Static(__name) => (*__name).to_owned(),").unwrap();
        for template in &pane.templates {
            writeln!(
                out,
                "Self::{}(__key) => ::std::format!({}, __key),",
                pane_template_variant(&template.item.name),
                rust_string(&format!("{}({{}})", template.item.name))
            )
            .unwrap();
        }
        writeln!(out, "}}\n}}\n}}").unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn pane_split_slots(
    configuration: &ResolvedPaneConfiguration,
) -> Vec<Option<&str>> {
    fn collect<'a>(
        configuration: &'a ResolvedPaneConfiguration,
        output: &mut Vec<Option<&'a str>>,
    ) {
        if let ResolvedPaneConfiguration::Split { name, a, b, .. } = configuration {
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
    configuration: &ResolvedPaneConfiguration,
    pane_type: Option<&str>,
) -> String {
    match configuration {
        ResolvedPaneConfiguration::Pane(name) => {
            let value = pane_type.map_or_else(
                || rust_string(name),
                |pane_type| format!("{pane_type}::__Static({})", rust_string(name)),
            );
            format!("::iced::widget::pane_grid::Configuration::Pane({value})")
        }
        ResolvedPaneConfiguration::Split {
            axis, ratio, a, b, ..
        } => {
            let axis = match axis {
                ResolvedPaneAxis::Horizontal => "Horizontal",
                ResolvedPaneAxis::Vertical => "Vertical",
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

pub(in crate::codegen) fn document_pane_grids(
    program: &LoweredProgram,
) -> Vec<(&ResolvedPaneGrid, bool)> {
    fn statements_reference_grid(statements: &[ResolvedStatement], name: &str) -> bool {
        statements.iter().any(|statement| match statement {
            ResolvedStatement {
                kind: ResolvedStatementKind::PaneOperation { grid, .. },
                ..
            } => grid == name,
            ResolvedStatement {
                kind: ResolvedStatementKind::TaskGroup { statements, .. },
                ..
            } => statements_reference_grid(statements, name),
            ResolvedStatement {
                kind: ResolvedStatementKind::Abortable { task, .. },
                ..
            } => statements_reference_grid(::std::slice::from_ref(task), name),
            _ => false,
        })
    }

    let referenced = |name: &str| {
        program
            .app_handlers()
            .any(|handler| statements_reference_grid(&handler.statements, name))
            || program
                .preset_handlers()
                .any(|handler| statements_reference_grid(&handler.statements, name))
    };
    program
        .pane_grids()
        .into_iter()
        .map(|pane| (pane, pane.test_scope && !referenced(&pane.name)))
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

pub(in crate::codegen) fn needs_extern_noop(program: &LoweredProgram, document: &Document) -> bool {
    fn contains(node: &ViewNode) -> bool {
        match node {
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
    program
        .extern_components()
        .any(|component| component.route.is_none())
        || program
            .themers()
            .any(|themer| themer.adapter.route.is_none())
        || program
            .shaders()
            .any(|shader| shader.adapter.route.is_none())
        || contains(&document.view)
        || document.components.iter().any(|item| contains(&item.root))
        || document
            .tests
            .iter()
            .filter_map(|test| test.mount.as_ref())
            .any(contains)
}
