use super::*;
use crate::lower::{ResolvedAnimation, ResolvedAnimationEasing, ResolvedInitializer};

pub(in crate::codegen) fn resolved_initializer_code(
    initializer: &ResolvedInitializer,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let mut code = resolved_expr_use_code(
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
                rust_type_code(program, &template.key_type)
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

pub(in crate::codegen) fn uses_canvas(program: &LoweredProgram) -> bool {
    !program.resolved_canvases().is_empty()
}

pub(in crate::codegen) fn canvases(program: &LoweredProgram) -> Vec<&ResolvedCanvas> {
    program.resolved_canvases()
}

pub(in crate::codegen) fn canvas_cache_groups(program: &LoweredProgram) -> Vec<&str> {
    let mut groups = Vec::new();
    for group in canvases(program)
        .into_iter()
        .filter_map(|canvas| canvas.options.cache_group.as_deref())
    {
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    groups
}

pub(in crate::codegen) fn canvas_events(program: &LoweredProgram) -> Vec<&ResolvedCanvasEvent> {
    canvases(program)
        .into_iter()
        .flat_map(|canvas| &canvas.events)
        .collect()
}

pub(in crate::codegen) fn canvas_group_symbol(group: &str) -> String {
    if canonical_snake(group) {
        format!("__ICE_CANVAS_GROUP_{}", group.to_ascii_uppercase())
    } else {
        format!("__ICE_CANVAS_GROUP_0{}", rust_identifier_hex(group))
    }
}

pub(in crate::codegen) fn needs_extern_noop(program: &LoweredProgram) -> bool {
    program
        .extern_components()
        .any(|component| component.route.is_none())
        || program
            .themers()
            .any(|themer| themer.adapter.route.is_none())
        || program
            .shaders()
            .any(|shader| shader.adapter.route.is_none())
        || program
            .resolved_views()
            .any(|view| matches!(view.kind, ResolvedViewKind::Overlay { .. }))
}
