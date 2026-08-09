//! AST-facing construction of the checked declaration index.
//!
//! This is the only part of the `hir` module tree that reads the source AST;
//! `hir.rs` itself owns HIR types exclusively.

use crate::ast::*;
use std::collections::{HashMap, HashSet};

use super::*;

impl DeclarationIndex {
    pub(crate) fn build(document: &Document, origins: &mut OriginArena) -> Self {
        let app_states = document
            .states
            .iter()
            .enumerate()
            .map(|(index, state)| Declaration {
                id: AppStateId(index as u32),
                origin: origins.push(&state.span, None),
            })
            .collect::<Vec<_>>();
        let derived = document
            .derived
            .iter()
            .enumerate()
            .map(|(index, value)| Declaration {
                id: DerivedId(index as u32),
                origin: origins.push(&value.span, None),
            })
            .collect::<Vec<_>>();
        let mut components = document
            .components
            .iter()
            .enumerate()
            .map(|(component_index, component)| {
                let id = ComponentId(component_index as u32);
                let origin = origins.push(&component.span, None);
                let params = component
                    .params
                    .iter()
                    .enumerate()
                    .map(|(index, _)| Declaration {
                        id: ComponentParamId {
                            component: id,
                            index: index as u32,
                        },
                        origin: origins.push(&component.span, Some(origin)),
                    })
                    .collect();
                let slots = declared_slots(&component.root)
                    .into_iter()
                    .enumerate()
                    .map(|(index, span)| Declaration {
                        id: ComponentSlotId {
                            component: id,
                            index: index as u32,
                        },
                        origin: origins.push(span, Some(origin)),
                    })
                    .collect();
                let events = component
                    .events
                    .iter()
                    .enumerate()
                    .map(|(index, event)| ComponentEventDeclaration {
                        declaration: Declaration {
                            id: ComponentEventId {
                                component: id,
                                index: index as u32,
                            },
                            origin: origins.push(&event.span, Some(origin)),
                        },
                        name: event.name.clone(),
                        payloads: event.payloads.clone(),
                    })
                    .collect();
                let states = component
                    .states
                    .iter()
                    .enumerate()
                    .map(|(index, state)| Declaration {
                        id: ComponentStateId {
                            component: id,
                            index: index as u32,
                        },
                        origin: origins.push(&state.span, Some(origin)),
                    })
                    .collect();
                ComponentDeclarations {
                    declaration: Declaration { id, origin },
                    output: component.output.clone(),
                    params,
                    events,
                    slots,
                    slot_views: Vec::new(),
                    states,
                }
            })
            .collect::<Vec<_>>();
        let components_by_name = document
            .components
            .iter()
            .zip(&components)
            .map(|(component, declarations)| (component.name.clone(), declarations.declaration.id))
            .collect();

        let structs = document
            .structs
            .iter()
            .enumerate()
            .map(|(struct_index, item)| {
                let id = StructId(struct_index as u32);
                let origin = origins.push(&item.span, None);
                let fields = item
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, (name, ty))| StructFieldDeclaration {
                        declaration: Declaration {
                            id: StructFieldId {
                                owner: id,
                                index: index as u32,
                            },
                            origin: origins.push(&item.span, Some(origin)),
                        },
                        name: name.clone(),
                        ty: ty.clone(),
                    })
                    .collect();
                StructDeclaration {
                    declaration: Declaration { id, origin },
                    name: item.name.clone(),
                    rust_path: item.rust_path.clone(),
                    fields,
                }
            })
            .collect::<Vec<_>>();
        let structs_by_name = structs
            .iter()
            .map(|item| (item.name.clone(), item.declaration.id))
            .collect();
        let struct_fields_by_owner = structs
            .iter()
            .map(|item| {
                (
                    item.declaration.id,
                    item.fields
                        .iter()
                        .map(|field| (field.name.clone(), field.declaration.id))
                        .collect(),
                )
            })
            .collect();

        let enums = document
            .enums
            .iter()
            .enumerate()
            .map(|(enum_index, item)| {
                let id = EnumId(enum_index as u32);
                let origin = origins.push(&item.span, None);
                let variants = item
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(index, variant)| EnumVariantDeclaration {
                        declaration: Declaration {
                            id: EnumVariantId {
                                owner: id,
                                index: index as u32,
                            },
                            origin: origins.push(&variant.span, Some(origin)),
                        },
                        name: variant.name.clone(),
                        payload: variant.payload.clone(),
                    })
                    .collect();
                EnumDeclaration {
                    declaration: Declaration { id, origin },
                    name: item.name.clone(),
                    rust_name: canonical_rust_type_name(&item.name),
                    variants,
                }
            })
            .collect::<Vec<_>>();
        let enums_by_name = enums
            .iter()
            .map(|item| (item.name.clone(), item.declaration.id))
            .collect();
        let enum_variants_by_owner = enums
            .iter()
            .map(|item| {
                (
                    item.declaration.id,
                    item.variants
                        .iter()
                        .map(|variant| (variant.name.clone(), variant.declaration.id))
                        .collect(),
                )
            })
            .collect();

        let palettes = document
            .palettes
            .iter()
            .enumerate()
            .map(|(index, palette)| Declaration {
                id: PaletteId(index as u32),
                origin: origins.push(&palette.span, None),
            })
            .collect::<Vec<_>>();
        let palettes_by_name = document
            .palettes
            .iter()
            .zip(&palettes)
            .map(|(palette, declaration)| (palette.name.clone(), declaration.id))
            .collect();
        let palette_names = document
            .palettes
            .iter()
            .map(|palette| palette.name.clone())
            .collect();
        let externs = document
            .functions
            .iter()
            .enumerate()
            .map(|(index, function)| ExternDeclaration {
                declaration: Declaration {
                    id: ExternFnId(index as u32),
                    origin: origins.push(&function.span, None),
                },
                kind: function.kind,
                name: function.name.clone(),
                rust_path: function.rust_path.clone(),
                params: function.params.clone(),
                borrowed: function.borrowed.clone(),
                progress: function.progress.clone(),
                output: function.output.clone(),
                error: function.error.clone(),
            })
            .collect::<Vec<_>>();
        let externs_by_name = externs
            .iter()
            .map(|function| (function.name.clone(), function.declaration.id))
            .collect();

        let subscriptions = document
            .subscriptions
            .iter()
            .enumerate()
            .map(|(index, subscription)| Declaration {
                id: SubscriptionId(index as u32),
                origin: origins.push(&subscription.span, None),
            })
            .collect();

        let tests = document
            .tests
            .iter()
            .enumerate()
            .map(|(test_index, test)| {
                let id = TestId(test_index as u32);
                let origin = origins.push(&test.span, None);
                let targets = test
                    .targets
                    .iter()
                    .enumerate()
                    .map(|(index, target)| TestTargetDeclaration {
                        declaration: Declaration {
                            id: TestTargetId {
                                test: id,
                                index: index as u32,
                            },
                            origin: origins.push(&target.span, Some(origin)),
                        },
                        name: target.name.clone(),
                        segments: target
                            .target
                            .segments
                            .iter()
                            .map(|segment| (segment.name.clone(), segment.key.is_some()))
                            .collect(),
                    })
                    .collect();
                let steps = test
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(index, step)| TestStepDeclaration {
                        declaration: Declaration {
                            id: TestStepId {
                                test: id,
                                index: index as u32,
                            },
                            origin: origins.push(&step.span, Some(origin)),
                        },
                        semantic_key: crate::ast::test_step_semantic_key(step),
                        source: crate::ast::test_step_source(step),
                    })
                    .collect();
                TestDeclaration {
                    declaration: Declaration { id, origin },
                    name: test.name.clone(),
                    semantic_key: crate::ast::test_declaration_semantic_key(test),
                    targets,
                    steps,
                }
            })
            .collect();

        let mut views = Vec::new();
        let mut views_by_site = HashMap::new();
        let mut canvases = HashMap::new();
        let mut component_calls_by_view = HashMap::new();
        for (index, component) in document.components.iter().enumerate() {
            index_view_declarations(
                &component.root,
                Some(components[index].declaration.origin),
                origins,
                &mut views,
                &mut views_by_site,
                &mut canvases,
                &mut component_calls_by_view,
            );
        }
        index_view_declarations(
            &document.view,
            None,
            origins,
            &mut views,
            &mut views_by_site,
            &mut canvases,
            &mut component_calls_by_view,
        );
        for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
            index_view_declarations(
                mount,
                None,
                origins,
                &mut views,
                &mut views_by_site,
                &mut canvases,
                &mut component_calls_by_view,
            );
        }
        for (component, declarations) in document.components.iter().zip(&mut components) {
            declarations.slot_views = declared_slots(&component.root)
                .into_iter()
                .map(|span| {
                    views_by_site[&SourceSite {
                        line: span.line,
                        column: span.column,
                    }]
                })
                .collect();
        }

        let mut handlers = Vec::new();
        let mut statements = Vec::new();
        let mut tasks = Vec::new();
        let mut routes = Vec::new();
        let mut run_lanes = Vec::new();
        let mut run_lanes_by_owner_name = HashMap::new();
        for handler in &document.handlers {
            index_handler_declaration(
                handler,
                HandlerOwner::App,
                None,
                origins,
                &mut handlers,
                &mut statements,
                &mut tasks,
                &mut routes,
                &mut run_lanes,
                &mut run_lanes_by_owner_name,
            );
        }
        for (component_index, component) in document.components.iter().enumerate() {
            let component_id = components[component_index].declaration.id;
            let parent = Some(components[component_index].declaration.origin);
            for handler in &component.handlers {
                index_handler_declaration(
                    handler,
                    HandlerOwner::Component(component_id),
                    parent,
                    origins,
                    &mut handlers,
                    &mut statements,
                    &mut tasks,
                    &mut routes,
                    &mut run_lanes,
                    &mut run_lanes_by_owner_name,
                );
            }
        }
        for (preset_index, preset) in document.presets.iter().enumerate() {
            let handler = Handler {
                name: format!("preset {}", preset.name),
                params: Vec::new(),
                statements: preset.statements.clone(),
                span: preset.span.clone(),
            };
            index_handler_declaration(
                &handler,
                HandlerOwner::Preset(preset_index as u32),
                None,
                origins,
                &mut handlers,
                &mut statements,
                &mut tasks,
                &mut routes,
                &mut run_lanes,
                &mut run_lanes_by_owner_name,
            );
        }

        let handlers_by_owner_name = handlers
            .iter()
            .map(|handler| {
                let route_owner = match handler.owner {
                    HandlerOwner::Preset(_) => HandlerOwner::App,
                    owner => owner,
                };
                ((route_owner, handler.name.clone()), handler.declaration.id)
            })
            .collect();

        let app_settings = Declaration {
            id: AppSettingsId,
            origin: origins.push(&document.settings.span, None),
        };
        let mut app_setting_expressions = HashMap::new();
        let mut push_app_expression = |id: AppSettingExprId, expression: &AppExpression| {
            app_setting_expressions.insert(
                id,
                Declaration {
                    id,
                    origin: origins.push(&expression.span, Some(app_settings.origin)),
                },
            );
        };
        if let Some(expression) = &document.settings.title {
            push_app_expression(AppSettingExprId::Title, expression);
        }
        if let Some(expression) = &document.settings.theme {
            push_app_expression(AppSettingExprId::Theme, expression);
            if let Expr::Call { name, args } = &expression.value
                && document
                    .functions
                    .iter()
                    .any(|function| function.name == *name && function.kind == ExternKind::Theme)
            {
                for (index, _) in args.iter().enumerate() {
                    push_app_expression(
                        AppSettingExprId::ThemeFactoryArgument(index as u32),
                        expression,
                    );
                }
            }
        }
        for (id, expression) in [
            (AppSettingExprId::Palette, &document.settings.palette),
            (AppSettingExprId::Background, &document.settings.background),
            (AppSettingExprId::TextColor, &document.settings.text_color),
            (
                AppSettingExprId::ScaleFactor,
                &document.settings.scale_factor,
            ),
        ] {
            if let Some(expression) = expression {
                push_app_expression(id, expression);
            }
        }
        if let Some(tray) = &document.settings.tray {
            for (id, expression) in [
                (AppSettingExprId::TrayLabel, &tray.label),
                (AppSettingExprId::TrayTooltip, &tray.tooltip),
            ] {
                if let Some(expression) = expression
                    .as_ref()
                    .filter(|setting| crate::ast::tray_text_is_reactive(setting))
                {
                    push_app_expression(id, expression);
                }
            }
            for (index, icon) in tray.icons.iter().enumerate() {
                if let Some(guard) = &icon.when {
                    push_app_expression(AppSettingExprId::TrayIconGuard(index as u32), guard);
                }
            }
            for (index, row) in tray.menu.iter().enumerate() {
                if let crate::ast::TrayRow::Item { text, .. } = row
                    && crate::ast::tray_text_is_reactive(text)
                {
                    push_app_expression(AppSettingExprId::TrayMenuRow(index as u32), text);
                }
            }
        }

        Self {
            app_settings,
            app_setting_expressions,
            app_states,
            derived,
            components,
            components_by_name,
            structs,
            structs_by_name,
            struct_fields_by_owner,
            enums,
            enums_by_name,
            enum_variants_by_owner,
            palettes,
            palette_names,
            palettes_by_name,
            externs,
            externs_by_name,
            #[cfg(test)]
            extern_name_lookups: TestLookupCount::default(),
            subscriptions,
            tests,
            views,
            views_by_site,
            canvases,
            component_calls_by_view,
            handlers,
            handlers_by_owner_name,
            statements,
            tasks,
            routes,
            run_lanes,
        }
    }

    pub(crate) fn finalize_checked_handlers(
        &mut self,
        document: &Document,
    ) -> Result<(), crate::Error> {
        let mut expected = Vec::new();
        expected.extend(document.handlers.iter().map(|handler| {
            (
                HandlerOwner::App,
                handler.name.clone(),
                handler
                    .params
                    .iter()
                    .map(|param| param.ty.clone())
                    .collect::<Vec<_>>(),
                handler.span.clone(),
            )
        }));
        for (index, component) in document.components.iter().enumerate() {
            expected.extend(component.handlers.iter().map(|handler| {
                (
                    HandlerOwner::Component(ComponentId(index as u32)),
                    handler.name.clone(),
                    handler
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect::<Vec<_>>(),
                    handler.span.clone(),
                )
            }));
        }
        expected.extend(document.presets.iter().enumerate().map(|(index, preset)| {
            (
                HandlerOwner::Preset(index as u32),
                format!("preset {}", preset.name),
                Vec::new(),
                preset.span.clone(),
            )
        }));
        if self.handlers.len() != expected.len() {
            return Err(crate::Error::new(
                "E196",
                &Span::line(1),
                "checked handler declarations changed during semantic analysis",
            ));
        }
        for (declaration, (owner, name, payloads, span)) in self.handlers.iter_mut().zip(expected) {
            if declaration.owner != owner || declaration.name != name {
                return Err(crate::Error::new(
                    "E196",
                    &span,
                    "checked handler identity changed during semantic analysis",
                ));
            }
            declaration.payloads = payloads;
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn index_handler_declaration(
    handler: &Handler,
    owner: HandlerOwner,
    parent: Option<OriginId>,
    origins: &mut OriginArena,
    handlers: &mut Vec<HandlerDeclaration>,
    statements: &mut Vec<StatementDeclaration>,
    tasks: &mut Vec<TaskDeclaration>,
    routes: &mut Vec<RouteDeclaration>,
    run_lanes: &mut Vec<RunLaneDeclaration>,
    run_lanes_by_owner_name: &mut HashMap<(HandlerOwner, String), RunLaneId>,
) {
    let id = HandlerId(handlers.len() as u32);
    let origin = origins.push(&handler.span, parent);
    let statement_roots = handler
        .statements
        .iter()
        .enumerate()
        .map(|(index, statement)| {
            index_statement_declaration(
                statement,
                id,
                owner,
                None,
                index + 1 == handler.statements.len(),
                origin,
                origins,
                statements,
                tasks,
                routes,
                run_lanes,
                run_lanes_by_owner_name,
            )
        })
        .collect();
    handlers.push(HandlerDeclaration {
        declaration: Declaration { id, origin },
        owner,
        name: handler.name.clone(),
        statement_roots,
        payloads: handler
            .params
            .iter()
            .map(|param| param.ty.clone())
            .collect(),
    });
}

#[allow(clippy::too_many_arguments)]
fn index_statement_declaration(
    statement: &Statement,
    handler: HandlerId,
    owner: HandlerOwner,
    parent: Option<StatementId>,
    is_final: bool,
    parent_origin: OriginId,
    origins: &mut OriginArena,
    statements: &mut Vec<StatementDeclaration>,
    tasks: &mut Vec<TaskDeclaration>,
    routes: &mut Vec<RouteDeclaration>,
    run_lanes: &mut Vec<RunLaneDeclaration>,
    run_lanes_by_owner_name: &mut HashMap<(HandlerOwner, String), RunLaneId>,
) -> StatementId {
    let id = StatementId(statements.len() as u32);
    let origin = origins.push(statement.span(), Some(parent_origin));
    statements.push(StatementDeclaration {
        declaration: Declaration { id, origin },
        handler,
        parent,
        task: None,
        source_tasks: Vec::new(),
        routes: Vec::new(),
        run_lane: None,
        children: Vec::new(),
        is_final,
    });

    let task = statement.immediate_task().map(|_| {
        let task = TaskId(tasks.len() as u32);
        let task_origin = origins.push(statement.span(), Some(origin));
        tasks.push(TaskDeclaration {
            declaration: Declaration {
                id: task,
                origin: task_origin,
            },
            statement: id,
            parent: None,
        });
        task
    });
    statements[id.0 as usize].task = task;

    if let Statement::TaskFlow {
        source, transforms, ..
    } = statement
    {
        let parent_task = task;
        let source_task = index_task_source(source, id, parent_task, origins, tasks);
        statements[id.0 as usize].source_tasks.push(source_task);
        for transform in transforms {
            let transform_task = index_task_transform(transform, id, parent_task, origins, tasks);
            statements[id.0 as usize].source_tasks.push(transform_task);
        }
    }

    let route_values: Vec<&Route> = match statement {
        Statement::Run { success, error, .. } => {
            std::iter::once(success).chain(error.iter()).collect()
        }
        Statement::Sip {
            progress,
            success,
            error,
            ..
        } => std::iter::once(progress)
            .chain(std::iter::once(success))
            .chain(error.iter())
            .collect(),
        Statement::TaskFlow {
            success,
            error,
            units,
            ..
        } => success
            .iter()
            .chain(error.iter())
            .chain(units.iter())
            .collect(),
        Statement::WidgetOperation { route, .. }
        | Statement::PaneOperation { route, .. }
        | Statement::WindowOperation { route, .. } => route.iter().collect(),
        _ => Vec::new(),
    };
    for route in route_values {
        let route_id = RouteId(routes.len() as u32);
        let route_origin = origins.push(&route.span, Some(origin));
        routes.push(RouteDeclaration {
            declaration: Declaration {
                id: route_id,
                origin: route_origin,
            },
            statement: id,
            task,
        });
        statements[id.0 as usize].routes.push(route_id);
    }

    if let Statement::Run {
        mode,
        lane: Some(name),
        ..
    } = statement
    {
        let owner = match owner {
            HandlerOwner::Preset(_) => HandlerOwner::App,
            owner => owner,
        };
        let key = (owner, name.clone());
        let run_lane = if let Some(run_lane) = run_lanes_by_owner_name.get(&key).copied() {
            run_lanes[run_lane.0 as usize].statements.push(id);
            run_lane
        } else {
            let run_lane = RunLaneId(run_lanes.len() as u32);
            run_lanes.push(RunLaneDeclaration {
                declaration: Declaration {
                    id: run_lane,
                    origin,
                },
                owner,
                name: name.clone(),
                mode: *mode,
                statements: vec![id],
            });
            run_lanes_by_owner_name.insert(key, run_lane);
            run_lane
        };
        statements[id.0 as usize].run_lane = Some(run_lane);
    }

    let children: Vec<&Statement> = match statement {
        Statement::TaskGroup { statements, .. } => statements.iter().collect(),
        Statement::Abortable { task, .. } => vec![task.as_ref()],
        _ => Vec::new(),
    };
    let child_ids = children
        .into_iter()
        .map(|child| {
            index_statement_declaration(
                child,
                handler,
                owner,
                Some(id),
                true,
                origin,
                origins,
                statements,
                tasks,
                routes,
                run_lanes,
                run_lanes_by_owner_name,
            )
        })
        .collect();
    statements[id.0 as usize].children = child_ids;
    id
}

fn index_task_source(
    source: &TaskSource,
    statement: StatementId,
    parent: Option<TaskId>,
    origins: &mut OriginArena,
    tasks: &mut Vec<TaskDeclaration>,
) -> TaskId {
    let span = match source {
        TaskSource::Effect { span, .. }
        | TaskSource::Done { span, .. }
        | TaskSource::None { span, .. } => span,
    };
    let id = TaskId(tasks.len() as u32);
    let parent_origin = parent.map(|parent| tasks[parent.0 as usize].declaration.origin);
    let origin = origins.push(span, parent_origin);
    tasks.push(TaskDeclaration {
        declaration: Declaration { id, origin },
        statement,
        parent,
    });
    id
}

fn index_task_transform(
    transform: &TaskTransform,
    statement: StatementId,
    parent: Option<TaskId>,
    origins: &mut OriginArena,
    tasks: &mut Vec<TaskDeclaration>,
) -> TaskId {
    let span = match transform {
        TaskTransform::Map { span, .. }
        | TaskTransform::Then { span, .. }
        | TaskTransform::AndThen { span, .. }
        | TaskTransform::MapError { span, .. }
        | TaskTransform::Collect { span }
        | TaskTransform::Discard { span } => span,
    };
    let id = TaskId(tasks.len() as u32);
    let parent_origin = parent.map(|parent| tasks[parent.0 as usize].declaration.origin);
    let origin = origins.push(span, parent_origin);
    tasks.push(TaskDeclaration {
        declaration: Declaration { id, origin },
        statement,
        parent,
    });
    id
}

fn declared_slots(node: &ViewNode) -> Vec<&Span> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<&'a Span>) {
        if let ViewNode::Slot { span, .. } = node {
            output.push(span);
        }
        for child in view_children(node) {
            collect(child, output);
        }
    }

    let mut output = Vec::new();
    collect(node, &mut output);
    output
}

fn index_view_declarations(
    node: &ViewNode,
    parent: Option<OriginId>,
    origins: &mut OriginArena,
    views: &mut Vec<Declaration<ViewId>>,
    views_by_site: &mut HashMap<SourceSite, ViewId>,
    canvases: &mut HashMap<ViewId, CanvasDeclaration>,
    component_calls_by_view: &mut HashMap<ViewId, ComponentCallId>,
) {
    let id = ViewId(views.len() as u32);
    let origin = origins.push(node.span(), parent);
    views.push(Declaration { id, origin });
    views_by_site.insert(
        SourceSite {
            line: node.span().line,
            column: node.span().column,
        },
        id,
    );
    if matches!(node, ViewNode::Component { .. }) {
        let call = ComponentCallId(component_calls_by_view.len() as u32);
        component_calls_by_view.insert(id, call);
    }
    if let ViewNode::Canvas {
        options,
        locals,
        commands,
        events,
        ..
    } = node
    {
        let local_declarations = locals
            .iter()
            .enumerate()
            .map(|(index, local)| CanvasLocalDeclaration {
                declaration: Declaration {
                    id: CanvasLocalId {
                        canvas: id,
                        index: index as u32,
                    },
                    origin: origins.push(&local.span, Some(origin)),
                },
                name: local.name.clone(),
                ty: local.ty.clone(),
            })
            .collect();
        let command_declarations = crate::ast::canvas_command_spans(commands)
            .into_iter()
            .zip(crate::ast::canvas_command_semantic_keys(commands))
            .enumerate()
            .map(|(index, (span, semantic_key))| CanvasCommandDeclaration {
                declaration: Declaration {
                    id: CanvasCommandId {
                        canvas: id,
                        index: index as u32,
                    },
                    origin: origins.push(span, Some(origin)),
                },
                semantic_key,
            })
            .collect();
        let event_declarations = events
            .iter()
            .enumerate()
            .map(|(index, event)| CanvasEventDeclaration {
                declaration: Declaration {
                    id: CanvasEventId {
                        canvas: id,
                        index: index as u32,
                    },
                    origin: origins.push(&event.span, Some(origin)),
                },
                semantic_key: crate::ast::canvas_event_semantic_key(event),
            })
            .collect();
        let route_declarations = crate::ast::canvas_routes(options, events)
            .into_iter()
            .enumerate()
            .map(|(index, route)| Declaration {
                id: CanvasRouteId {
                    canvas: id,
                    index: index as u32,
                },
                origin: origins.push(&route.span, Some(origin)),
            })
            .collect();
        canvases.insert(
            id,
            CanvasDeclaration {
                declaration: Declaration { id, origin },
                options_semantic_key: crate::ast::canvas_options_semantic_key(options),
                locals: local_declarations,
                commands: command_declarations,
                events: event_declarations,
                routes: route_declarations,
            },
        );
    }
    for child in view_children(node) {
        index_view_declarations(
            child,
            Some(origin),
            origins,
            views,
            views_by_site,
            canvases,
            component_calls_by_view,
        );
    }
}

pub(crate) fn view_children(node: &ViewNode) -> Vec<&ViewNode> {
    match node {
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => children.iter().collect(),
        ViewNode::Match { arms, .. } => arms.iter().flat_map(|arm| arm.children.iter()).collect(),
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
        | ViewNode::Lazy { child: content, .. } => vec![content],
        ViewNode::Tooltip { content, tip, .. } => vec![content, tip],
        ViewNode::Overlay { content, layer, .. } => vec![content, layer],
        ViewNode::PaneGrid {
            panes, templates, ..
        } => panes
            .iter()
            .flat_map(PaneView::nodes)
            .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            .collect(),
        ViewNode::Table { columns, .. } => columns
            .iter()
            .flat_map(|column| [&column.header, &column.cell])
            .collect(),
        ViewNode::Component { slots, .. } => {
            slots.iter().map(|slot| slot.content.as_ref()).collect()
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => vec![narrow, wide],
            ResponsiveContent::Size { content, .. } => vec![content],
        },
        _ => Vec::new(),
    }
}

pub(crate) fn dynamic_pane_grids(document: &Document) -> HashSet<String> {
    let mut dynamic = HashSet::new();
    let mut pending = vec![&document.view];
    pending.extend(document.tests.iter().filter_map(|test| test.mount.as_ref()));
    while let Some(node) = pending.pop() {
        if let ViewNode::PaneGrid {
            name, templates, ..
        } = node
            && !templates.is_empty()
        {
            dynamic.insert(name.clone());
        }
        pending.extend(view_children(node));
    }
    dynamic
}

pub(crate) fn statement_semantic_key(statement: &Statement) -> String {
    fn route_shape(route: &Route) -> String {
        let args = route
            .args
            .iter()
            .map(|arg| match arg {
                RouteArg::Payload => '_',
                RouteArg::Expr(_) => 'e',
            })
            .collect::<String>();
        format!("{}:{args}", route.handler)
    }

    fn source_key(source: &TaskSource) -> String {
        match source {
            TaskSource::Effect {
                kind,
                function,
                args,
                ..
            } => format!("effect:{kind:?}:{function}:{}", args.len()),
            TaskSource::Done { .. } => "done".into(),
            TaskSource::None { output, .. } => format!("none:{output:?}"),
        }
    }

    fn transform_key(transform: &TaskTransform) -> String {
        match transform {
            TaskTransform::Map { binding, .. } => format!("map:{binding}"),
            TaskTransform::Then {
                binding, source, ..
            } => format!("then:{binding}:{}", source_key(source)),
            TaskTransform::AndThen {
                binding, source, ..
            } => format!("and-then:{binding}:{}", source_key(source)),
            TaskTransform::MapError { binding, .. } => format!("map-error:{binding}"),
            TaskTransform::Collect { .. } => "collect".into(),
            TaskTransform::Discard { .. } => "discard".into(),
        }
    }

    match statement {
        Statement::Let { name, .. } => format!("let:{name}"),
        Statement::Assign { target, at, .. } => format!("assign:{target}:{}", at.is_some()),
        Statement::MarkdownAppend { target, .. } => format!("markdown-append:{target}"),
        Statement::ComboPush { target, .. } => format!("combo-push:{target}"),
        Statement::ReturnIf { .. } => "return-if".into(),
        Statement::Exit { .. } => "exit".into(),
        Statement::Run {
            kind,
            mode,
            lane,
            function,
            args,
            success,
            error,
            ..
        } => format!(
            "run:{kind:?}:{mode:?}:{}:{function}:{}:{}:{}",
            lane.as_deref().unwrap_or_default(),
            args.len(),
            route_shape(success),
            error.as_ref().map(route_shape).unwrap_or_default()
        ),
        Statement::Sip {
            function,
            args,
            progress,
            success,
            error,
            ..
        } => format!(
            "sip:{function}:{}:{}:{}:{}",
            args.len(),
            route_shape(progress),
            route_shape(success),
            error.as_ref().map(route_shape).unwrap_or_default()
        ),
        Statement::TaskFlow {
            source,
            transforms,
            success,
            error,
            units,
            ..
        } => format!(
            "flow:{}:[{}]:{}:{}:{}",
            source_key(source),
            transforms
                .iter()
                .map(transform_key)
                .collect::<Vec<_>>()
                .join(","),
            success.as_ref().map(route_shape).unwrap_or_default(),
            error.as_ref().map(route_shape).unwrap_or_default(),
            units.as_ref().map(route_shape).unwrap_or_default()
        ),
        Statement::TaskGroup { kind, .. } => format!("task-group:{kind:?}"),
        Statement::Abortable {
            handle,
            abort_on_drop,
            ..
        } => format!("abortable:{handle}:{abort_on_drop}"),
        Statement::Abort { handle, .. } => format!("abort:{handle}"),
        Statement::DebugStart { target, .. } => format!("debug-start:{target}"),
        Statement::DebugFinish { target, .. } => format!("debug-finish:{target}"),
        Statement::ClipboardWrite { primary, .. } => format!("clipboard:{primary}"),
        Statement::WidgetOperation {
            operation, route, ..
        } => format!(
            "widget:{:?}:{}",
            std::mem::discriminant(operation),
            route.as_ref().map(route_shape).unwrap_or_default()
        ),
        Statement::PaneOperation {
            grid,
            operation,
            route,
            ..
        } => format!(
            "pane:{grid}:{:?}:{}",
            std::mem::discriminant(operation),
            route.as_ref().map(route_shape).unwrap_or_default()
        ),
        Statement::WindowOperation {
            operation,
            target,
            route,
            ..
        } => format!(
            "window:{:?}:{}:{}",
            std::mem::discriminant(operation),
            target.is_some(),
            route.as_ref().map(route_shape).unwrap_or_default()
        ),
    }
}

pub(crate) fn handler_operation_contract(
    statement: &Statement,
) -> Option<HandlerOperationContract> {
    fn target(value: &WidgetTarget) -> CheckedWidgetTarget {
        CheckedWidgetTarget(
            value
                .segments
                .iter()
                .map(|segment| (segment.name.clone(), segment.key.is_some()))
                .collect(),
        )
    }
    fn selector(value: &WidgetSelector) -> CheckedWidgetSelector {
        match value {
            WidgetSelector::Id(value) => CheckedWidgetSelector::Id(target(value)),
            WidgetSelector::Text(_) => CheckedWidgetSelector::Text,
            WidgetSelector::Point { .. } => CheckedWidgetSelector::Point,
            WidgetSelector::Focused => CheckedWidgetSelector::Focused,
            WidgetSelector::Extern { function, args } => CheckedWidgetSelector::Extern {
                function: function.clone(),
                arguments: args.len(),
            },
        }
    }
    fn pane(value: &PaneReference) -> CheckedPaneReference {
        match value {
            PaneReference::Static(name) => CheckedPaneReference::Static(name.clone()),
            PaneReference::Dynamic { template, .. } => {
                CheckedPaneReference::Dynamic(template.clone())
            }
        }
    }
    fn edge(value: PaneEdge) -> CheckedPaneEdge {
        match value {
            PaneEdge::Top => CheckedPaneEdge::Top,
            PaneEdge::Left => CheckedPaneEdge::Left,
            PaneEdge::Right => CheckedPaneEdge::Right,
            PaneEdge::Bottom => CheckedPaneEdge::Bottom,
        }
    }
    Some(match statement {
        Statement::WidgetOperation { operation, .. } => {
            HandlerOperationContract::Widget(match operation {
                WidgetOperation::FocusPrevious => CheckedWidgetOperation::FocusPrevious,
                WidgetOperation::FocusNext => CheckedWidgetOperation::FocusNext,
                WidgetOperation::Focus { target: value } => {
                    CheckedWidgetOperation::Focus(target(value))
                }
                WidgetOperation::Focused { target: value } => {
                    CheckedWidgetOperation::Focused(target(value))
                }
                WidgetOperation::CursorFront { target: value } => {
                    CheckedWidgetOperation::CursorFront(target(value))
                }
                WidgetOperation::CursorEnd { target: value } => {
                    CheckedWidgetOperation::CursorEnd(target(value))
                }
                WidgetOperation::Cursor { target: value, .. } => {
                    CheckedWidgetOperation::Cursor(target(value))
                }
                WidgetOperation::SelectAll { target: value } => {
                    CheckedWidgetOperation::SelectAll(target(value))
                }
                WidgetOperation::Select { target: value, .. } => {
                    CheckedWidgetOperation::Select(target(value))
                }
                WidgetOperation::Snap { target: value, .. } => {
                    CheckedWidgetOperation::Snap(target(value))
                }
                WidgetOperation::SnapEnd { target: value } => {
                    CheckedWidgetOperation::SnapEnd(target(value))
                }
                WidgetOperation::ScrollTo { target: value, .. } => {
                    CheckedWidgetOperation::ScrollTo(target(value))
                }
                WidgetOperation::ScrollBy { target: value, .. } => {
                    CheckedWidgetOperation::ScrollBy(target(value))
                }
                WidgetOperation::Find {
                    selector: value,
                    all,
                } => CheckedWidgetOperation::Find {
                    selector: selector(value),
                    all: *all,
                },
            })
        }
        Statement::PaneOperation {
            grid, operation, ..
        } => HandlerOperationContract::Pane {
            grid: grid.clone(),
            operation: match operation {
                PaneOperation::Maximize { pane: value } => {
                    CheckedPaneOperation::Maximize(pane(value))
                }
                PaneOperation::Restore => CheckedPaneOperation::Restore,
                PaneOperation::Maximized => CheckedPaneOperation::Maximized,
                PaneOperation::Adjacent {
                    pane: value,
                    edge: value_edge,
                } => CheckedPaneOperation::Adjacent(pane(value), edge(*value_edge)),
                PaneOperation::Swap { first, second } => {
                    CheckedPaneOperation::Swap(pane(first), pane(second))
                }
                PaneOperation::Close { pane: value } => CheckedPaneOperation::Close(pane(value)),
                PaneOperation::Move {
                    pane: value,
                    edge: value_edge,
                } => CheckedPaneOperation::Move(pane(value), edge(*value_edge)),
                PaneOperation::Resize { split, .. } => CheckedPaneOperation::Resize(split.clone()),
                PaneOperation::Drop {
                    pane: value,
                    target,
                    edge: value_edge,
                } => CheckedPaneOperation::Drop(pane(value), pane(target), value_edge.map(edge)),
                PaneOperation::Split {
                    target,
                    pane: value,
                    axis,
                    ..
                } => CheckedPaneOperation::Split {
                    target: pane(target),
                    pane: pane(value),
                    axis: format!("{axis:?}"),
                },
            },
        },
        Statement::WindowOperation { operation, .. } => {
            HandlerOperationContract::Window(match operation {
                WindowOperation::Open(name) => CheckedWindowOperation::Open(name.clone()),
                WindowOperation::Oldest => CheckedWindowOperation::Oldest,
                WindowOperation::Latest => CheckedWindowOperation::Latest,
                WindowOperation::Close => CheckedWindowOperation::Close,
                WindowOperation::Drag => CheckedWindowOperation::Drag,
                WindowOperation::DragResize(direction) => {
                    CheckedWindowOperation::DragResize(format!("{direction:?}"))
                }
                WindowOperation::Resize(..) => CheckedWindowOperation::Resize,
                WindowOperation::Resizable(_) => CheckedWindowOperation::Resizable,
                WindowOperation::MinSize(value) => CheckedWindowOperation::MinSize(value.is_some()),
                WindowOperation::MaxSize(value) => CheckedWindowOperation::MaxSize(value.is_some()),
                WindowOperation::ResizeIncrements(value) => {
                    CheckedWindowOperation::ResizeIncrements(value.is_some())
                }
                WindowOperation::Size => CheckedWindowOperation::Size,
                WindowOperation::IsMaximized => CheckedWindowOperation::IsMaximized,
                WindowOperation::Maximize(_) => CheckedWindowOperation::Maximize,
                WindowOperation::IsMinimized => CheckedWindowOperation::IsMinimized,
                WindowOperation::Minimize(_) => CheckedWindowOperation::Minimize,
                WindowOperation::Position => CheckedWindowOperation::Position,
                WindowOperation::ScaleFactor => CheckedWindowOperation::ScaleFactor,
                WindowOperation::Move(..) => CheckedWindowOperation::Move,
                WindowOperation::Mode => CheckedWindowOperation::Mode,
                WindowOperation::SetMode(mode) => {
                    CheckedWindowOperation::SetMode(format!("{mode:?}"))
                }
                WindowOperation::ToggleMaximize => CheckedWindowOperation::ToggleMaximize,
                WindowOperation::ToggleDecorations => CheckedWindowOperation::ToggleDecorations,
                WindowOperation::Attention(value) => {
                    CheckedWindowOperation::Attention(value.map(|value| format!("{value:?}")))
                }
                WindowOperation::Focus => CheckedWindowOperation::Focus,
                WindowOperation::SetLevel(level) => {
                    CheckedWindowOperation::SetLevel(format!("{level:?}"))
                }
                WindowOperation::SystemMenu => CheckedWindowOperation::SystemMenu,
                WindowOperation::RawId => CheckedWindowOperation::RawId,
                WindowOperation::Screenshot => CheckedWindowOperation::Screenshot,
                WindowOperation::MousePassthrough(_) => CheckedWindowOperation::MousePassthrough,
                WindowOperation::MonitorSize => CheckedWindowOperation::MonitorSize,
                WindowOperation::AutomaticTabbing(_) => CheckedWindowOperation::AutomaticTabbing,
                WindowOperation::Icon { .. } => CheckedWindowOperation::Icon,
                WindowOperation::Callback { function, args } => CheckedWindowOperation::Callback {
                    function: function.clone(),
                    arguments: args.len(),
                },
            })
        }
        _ => return None,
    })
}

pub(crate) fn view_kind(node: &ViewNode) -> &'static str {
    match node {
        ViewNode::Layout { .. } => "layout",
        ViewNode::Container { .. } => "container",
        ViewNode::Overlay { .. } => "overlay",
        ViewNode::PaneGrid { .. } => "pane-grid",
        ViewNode::Text { .. } => "text",
        ViewNode::RichText { .. } => "rich-text",
        ViewNode::Input { .. } => "input",
        ViewNode::Button { .. } => "button",
        ViewNode::Checkbox { .. } => "checkbox",
        ViewNode::Toggler { .. } => "toggler",
        ViewNode::Slider { .. } => "slider",
        ViewNode::Progress { .. } => "progress",
        ViewNode::Radio { .. } => "radio",
        ViewNode::PickList { .. } => "pick-list",
        ViewNode::ComboBox { .. } => "combo-box",
        ViewNode::Rule { .. } => "rule",
        ViewNode::QrCode { .. } => "qr-code",
        ViewNode::Space { .. } => "space",
        ViewNode::If { .. } => "if",
        ViewNode::Match { .. } => "match",
        ViewNode::For { .. } => "for",
        ViewNode::KeyedColumn { .. } => "keyed-column",
        ViewNode::Lazy { .. } => "lazy",
        ViewNode::Markdown { .. } => "markdown",
        ViewNode::TextEditor { .. } => "text-editor",
        ViewNode::Table { .. } => "table",
        ViewNode::Component { .. } => "component",
        ViewNode::Slot { .. } => "slot",
        ViewNode::ExternComponent { .. } => "extern-component",
        ViewNode::Themer { .. } => "themer",
        ViewNode::Shader { .. } => "shader",
        ViewNode::Media { .. } => "media",
        ViewNode::Tooltip { .. } => "tooltip",
        ViewNode::MouseArea { .. } => "mouse-area",
        ViewNode::ResizeHandle { .. } => "resize-handle",
        ViewNode::Canvas { .. } => "canvas",
        ViewNode::Theme { .. } => "theme",
        ViewNode::Float { .. } => "float",
        ViewNode::Pin { .. } => "pin",
        ViewNode::Sensor { .. } => "sensor",
        ViewNode::Responsive { .. } => "responsive",
    }
}
