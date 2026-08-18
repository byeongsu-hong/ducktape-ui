use super::*;
use crate::Warning;

struct ImmediateRoutes<'a> {
    routes: Vec<&'a Route>,
    completes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AutomaticKind {
    Immediate,
    Deferred,
    MultiShot,
}

#[derive(Clone, Copy)]
struct AutomaticRoute<'a> {
    route: &'a Route,
    kind: AutomaticKind,
}

#[derive(Clone, Copy)]
struct FlowBehavior {
    emits: bool,
    completes: bool,
}

pub(in crate::check) fn immediate_handler_cycle_warnings(
    document: &Document,
    reachable: &HandlerReachability,
) -> Vec<Warning> {
    let mut warnings = immediate_cycles(&document.handlers, &reachable.app, None);
    for component in &document.components {
        if let Some(handlers) = reachable.components.get(&component.name) {
            warnings.extend(immediate_cycles(
                &component.handlers,
                handlers,
                Some(&component.name),
            ));
        }
    }
    warnings
}

fn immediate_cycles(
    handlers: &[Handler],
    reachable: &HashSet<String>,
    component: Option<&str>,
) -> Vec<Warning> {
    let indices = handlers
        .iter()
        .enumerate()
        .map(|(index, handler)| (handler.name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut edges = Vec::new();
    let mut connected = vec![vec![false; handlers.len()]; handlers.len()];

    for (source, handler) in handlers.iter().enumerate() {
        if !reachable.contains(&handler.name) {
            continue;
        }
        for statement in handler
            .statements
            .iter()
            .take_while(|statement| !is_termination_guard(statement))
        {
            for route in immediate_routes(statement).routes {
                let Some(&target) = indices.get(route.handler.as_str()) else {
                    continue;
                };
                connected[source][target] = true;
                edges.push((source, target, &route.span));
            }
        }
    }

    transitive_closure(&mut connected);

    let mut reported = vec![false; handlers.len()];
    let mut warnings = Vec::new();
    for first in 0..handlers.len() {
        if reported[first] || !connected[first][first] {
            continue;
        }
        let members = strongly_connected_members(first, &connected);
        for &member in &members {
            reported[member] = true;
        }
        let span = edges
            .iter()
            .filter(|(source, target, _)| members.contains(source) && members.contains(target))
            .map(|(_, _, span)| *span)
            .min_by_key(|span| (span.line, span.column))
            .expect("cyclic handlers have an internal edge");
        let mut names = members
            .iter()
            .map(|&index| handlers[index].name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        let message = if names.len() == 1 {
            if let Some(component) = component {
                format!(
                    "component handler `{component}.{}` immediately routes back to itself and can refresh forever",
                    names[0]
                )
            } else {
                format!(
                    "handler `{}` immediately routes back to itself and can refresh forever",
                    names[0]
                )
            }
        } else {
            let names = names
                .iter()
                .map(|name| match component {
                    Some(component) => format!("`{component}.{name}`"),
                    None => format!("`{name}`"),
                })
                .collect::<Vec<_>>()
                .join(", ");
            if component.is_some() {
                format!(
                    "component handlers {names} form an immediate routing cycle that can refresh forever"
                )
            } else {
                format!("handlers {names} form an immediate routing cycle that can refresh forever")
            }
        };
        warnings.push(
            Warning::new("W004", span, message)
                .hint("break the immediate back-edge or add a `return if ...` termination guard"),
        );
    }
    warnings
}

fn is_termination_guard(statement: &Statement) -> bool {
    matches!(
        statement,
        Statement::ReturnIf {
            condition,
            ..
        } if !matches!(condition, Expr::Bool(false))
    )
}

pub(in crate::check) fn routed_task_cycle_warnings(
    document: &Document,
    reachable: &HandlerReachability,
) -> Vec<Warning> {
    let mut warnings = routed_task_cycles(&document.handlers, &reachable.app, None);
    for component in &document.components {
        if let Some(handlers) = reachable.components.get(&component.name) {
            warnings.extend(routed_task_cycles(
                &component.handlers,
                handlers,
                Some(&component.name),
            ));
        }
    }
    warnings
}

fn routed_task_cycles(
    handlers: &[Handler],
    reachable: &HashSet<String>,
    component: Option<&str>,
) -> Vec<Warning> {
    let indices = handlers
        .iter()
        .enumerate()
        .map(|(index, handler)| (handler.name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut edges = Vec::new();
    let mut connected = vec![vec![false; handlers.len()]; handlers.len()];

    for (source, handler) in handlers.iter().enumerate() {
        if !reachable.contains(&handler.name) {
            continue;
        }
        for statement in handler
            .statements
            .iter()
            .take_while(|statement| !is_termination_guard(statement))
        {
            for effect in automatic_routes(statement) {
                let Some(&target) = indices.get(effect.route.handler.as_str()) else {
                    continue;
                };
                connected[source][target] = true;
                edges.push((source, target, &effect.route.span, effect.kind));
            }
        }
    }

    transitive_closure(&mut connected);
    let mut reported = vec![false; handlers.len()];
    let mut warnings = Vec::new();
    for first in 0..handlers.len() {
        if reported[first] || !connected[first][first] {
            continue;
        }
        let members = strongly_connected_members(first, &connected);
        for &member in &members {
            reported[member] = true;
        }
        let internal = edges
            .iter()
            .filter(|(source, target, _, _)| members.contains(source) && members.contains(target))
            .collect::<Vec<_>>();
        let Some((_, _, span, _)) = internal
            .iter()
            .filter(|(_, _, _, kind)| *kind != AutomaticKind::Immediate)
            .min_by_key(|(_, _, span, _)| (span.line, span.column))
            .copied()
        else {
            continue;
        };
        let multi_shot = internal
            .iter()
            .any(|(_, _, _, kind)| *kind == AutomaticKind::MultiShot);
        let mut names = members
            .iter()
            .map(|&index| handlers[index].name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        let names = names
            .iter()
            .map(|name| match component {
                Some(component) => format!("`{component}.{name}`"),
                None => format!("`{name}`"),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let message = if multi_shot {
            format!(
                "handler cycle {names} routes a repeated stream or progress item back into work that can multiply without bound"
            )
        } else {
            format!(
                "handler cycle {names} is driven by a future, task, or query completion and can refresh forever"
            )
        };
        let hint = if multi_shot {
            "route repeated items to a state-only handler, or own the work with one abortable handle"
        } else {
            "break the routed task back-edge or add a `return if ...` termination guard"
        };
        warnings.push(Warning::new("W006", span, message).hint(hint));
    }
    warnings
}

pub(in crate::check) fn raw_event_feedback_warnings(document: &Document) -> Vec<Warning> {
    document
        .subscriptions
        .iter()
        .filter(|subscription| {
            matches!(subscription.source, SubscriptionSource::Event { raw: true })
                && subscription.filter.is_none()
                && subscription.status != Some(EventStatus::Captured)
                && !matches!(subscription.condition, Some(Expr::Bool(false)))
        })
        .map(|subscription| {
            Warning::new(
                "W007",
                &subscription.span,
                format!(
                    "unfiltered raw events route redraw requests to `{}` and can create a redraw feedback loop",
                    subscription.route.handler
                ),
            )
            .hint("add `filter=`, use `status=captured`, or subscribe to non-raw `event`")
        })
        .collect()
}

fn transitive_closure(connected: &mut [Vec<bool>]) {
    for via in 0..connected.len() {
        let (before, remaining) = connected.split_at_mut(via);
        let (via_targets, after) = remaining.split_first_mut().unwrap();
        for targets in before.iter_mut().chain(after.iter_mut()) {
            if !targets[via] {
                continue;
            }
            for (reachable, via_reachable) in targets.iter_mut().zip(via_targets.iter()) {
                *reachable |= via_reachable;
            }
        }
    }
}

fn strongly_connected_members(first: usize, connected: &[Vec<bool>]) -> Vec<usize> {
    (0..connected.len())
        .filter(|&candidate| connected[first][candidate] && connected[candidate][first])
        .collect()
}

fn automatic_routes(statement: &Statement) -> Vec<AutomaticRoute<'_>> {
    let mut routes = immediate_routes(statement)
        .routes
        .into_iter()
        .map(|route| AutomaticRoute {
            route,
            kind: AutomaticKind::Immediate,
        })
        .collect::<Vec<_>>();
    match statement {
        Statement::Run {
            kind,
            success,
            error,
            ..
        } => {
            let automatic = match kind {
                EffectKind::Future | EffectKind::Task => AutomaticKind::Deferred,
                EffectKind::Stream => AutomaticKind::MultiShot,
            };
            routes.push(AutomaticRoute {
                route: success,
                kind: automatic,
            });
            if let Some(error) = error {
                routes.push(AutomaticRoute {
                    route: error,
                    kind: automatic,
                });
            }
        }
        Statement::Sip {
            progress,
            success,
            error,
            ..
        } => {
            routes.push(AutomaticRoute {
                route: progress,
                kind: AutomaticKind::MultiShot,
            });
            routes.push(AutomaticRoute {
                route: success,
                kind: AutomaticKind::Deferred,
            });
            if let Some(error) = error {
                routes.push(AutomaticRoute {
                    route: error,
                    kind: AutomaticKind::Deferred,
                });
            }
        }
        Statement::TaskFlow {
            source,
            transforms,
            success,
            error,
            ..
        } => {
            if let Some(kind) = non_immediate_flow_kind(source, transforms) {
                if let Some(success) = success
                    && !routes
                        .iter()
                        .any(|effect| std::ptr::eq(effect.route, success))
                {
                    routes.push(AutomaticRoute {
                        route: success,
                        kind,
                    });
                }
                if let Some(error) = error {
                    routes.push(AutomaticRoute { route: error, kind });
                }
            }
        }
        Statement::TaskGroup { statements, .. } => {
            routes.extend(
                statements
                    .iter()
                    .flat_map(automatic_routes)
                    .filter(|effect| effect.kind != AutomaticKind::Immediate),
            );
        }
        Statement::Match { arms, .. } => {
            routes.extend(
                arms.iter()
                    .flat_map(|arm| &arm.statements)
                    .flat_map(automatic_routes)
                    .filter(|effect| effect.kind != AutomaticKind::Immediate),
            );
        }
        Statement::Abortable { task, .. } => routes.extend(
            automatic_routes(task)
                .into_iter()
                .filter(|effect| effect.kind != AutomaticKind::Immediate),
        ),
        Statement::WidgetOperation { route, .. } | Statement::WindowOperation { route, .. } => {
            if let Some(route) = route {
                routes.push(AutomaticRoute {
                    route,
                    kind: AutomaticKind::Deferred,
                });
            }
        }
        Statement::Let { .. }
        | Statement::Assign { .. }
        | Statement::MarkdownAppend { .. }
        | Statement::ComboPush { .. }
        | Statement::ReturnIf { .. }
        | Statement::Exit { .. }
        | Statement::InvalidateLane { .. }
        | Statement::Abort { .. }
        | Statement::DebugStart { .. }
        | Statement::DebugFinish { .. }
        | Statement::ClipboardWrite { .. }
        | Statement::Emit { .. }
        | Statement::Slice { .. }
        | Statement::PaneOperation { .. } => {}
    }
    routes
}

fn non_immediate_flow_kind(
    source: &TaskSource,
    transforms: &[TaskTransform],
) -> Option<AutomaticKind> {
    let sources =
        std::iter::once(source).chain(transforms.iter().filter_map(|transform| match transform {
            TaskTransform::Then { source, .. } | TaskTransform::AndThen { source, .. } => {
                Some(source)
            }
            TaskTransform::Map { .. }
            | TaskTransform::MapError { .. }
            | TaskTransform::Collect { .. }
            | TaskTransform::Discard { .. } => None,
        }));
    let mut saw_effect = false;
    let mut saw_stream = false;
    for source in sources {
        match source {
            TaskSource::Effect {
                kind: EffectKind::Stream,
                ..
            } => {
                saw_effect = true;
                saw_stream = true;
            }
            TaskSource::Effect { .. } => saw_effect = true,
            TaskSource::Done { .. } | TaskSource::None { .. } => {}
        }
    }
    if saw_stream
        && !transforms.iter().any(|transform| {
            matches!(
                transform,
                TaskTransform::Collect { .. } | TaskTransform::Discard { .. }
            )
        })
    {
        Some(AutomaticKind::MultiShot)
    } else {
        saw_effect.then_some(AutomaticKind::Deferred)
    }
}

fn immediate_routes(statement: &Statement) -> ImmediateRoutes<'_> {
    fn sequence(statements: &[Statement]) -> ImmediateRoutes<'_> {
        let mut routes = Vec::new();
        let mut completes = true;
        for statement in statements {
            let outcome = immediate_routes(statement);
            routes.extend(outcome.routes);
            if !outcome.completes {
                completes = false;
                break;
            }
        }
        ImmediateRoutes { routes, completes }
    }

    match statement {
        Statement::TaskFlow {
            source,
            transforms,
            success,
            units,
            ..
        } => {
            let mut routes = units.iter().collect::<Vec<_>>();
            let behavior = flow_behavior(source, transforms);
            if behavior.emits
                && let Some(success) = success
            {
                routes.push(success);
            }
            ImmediateRoutes {
                routes,
                completes: behavior.completes,
            }
        }
        Statement::TaskGroup {
            kind, statements, ..
        } => match kind {
            TaskGroupKind::Parallel => {
                let outcomes = statements.iter().map(immediate_routes).collect::<Vec<_>>();
                ImmediateRoutes {
                    routes: outcomes
                        .iter()
                        .flat_map(|outcome| outcome.routes.iter().copied())
                        .collect(),
                    completes: outcomes.iter().all(|outcome| outcome.completes),
                }
            }
            TaskGroupKind::Sequential => sequence(statements),
        },
        Statement::Match { arms, .. } => {
            let outcomes = arms
                .iter()
                .map(|arm| sequence(&arm.statements))
                .collect::<Vec<_>>();
            ImmediateRoutes {
                routes: outcomes
                    .iter()
                    .flat_map(|outcome| outcome.routes.iter().copied())
                    .collect(),
                completes: outcomes.iter().all(|outcome| outcome.completes),
            }
        }
        Statement::Abortable { task, .. } => immediate_routes(task),
        Statement::PaneOperation {
            operation: PaneOperation::Maximized | PaneOperation::Adjacent { .. },
            route: Some(route),
            ..
        } => ImmediateRoutes {
            routes: vec![route],
            completes: true,
        },
        Statement::Exit { .. } => ImmediateRoutes {
            routes: Vec::new(),
            completes: true,
        },
        _ => ImmediateRoutes {
            routes: Vec::new(),
            completes: false,
        },
    }
}

fn flow_behavior(source: &TaskSource, transforms: &[TaskTransform]) -> FlowBehavior {
    let mut behavior = source_behavior(source);
    for transform in transforms {
        behavior = match transform {
            TaskTransform::Map { .. } | TaskTransform::MapError { .. } => behavior,
            TaskTransform::Then { source, .. } if behavior.emits => {
                if behavior.completes {
                    source_behavior(source)
                } else {
                    FlowBehavior {
                        emits: false,
                        completes: false,
                    }
                }
            }
            TaskTransform::Then { .. } => behavior,
            TaskTransform::AndThen { source, .. } => FlowBehavior {
                emits: false,
                completes: behavior.completes
                    && (!behavior.emits || source_behavior(source).completes),
            },
            TaskTransform::Collect { .. } => FlowBehavior {
                emits: behavior.completes,
                completes: behavior.completes,
            },
            TaskTransform::Discard { .. } => FlowBehavior {
                emits: false,
                completes: behavior.completes,
            },
        };
    }
    behavior
}

fn source_behavior(source: &TaskSource) -> FlowBehavior {
    match source {
        TaskSource::Done { .. } => FlowBehavior {
            emits: true,
            completes: true,
        },
        TaskSource::None { .. } => FlowBehavior {
            emits: false,
            completes: true,
        },
        TaskSource::Effect { .. } => FlowBehavior {
            emits: false,
            completes: false,
        },
    }
}
