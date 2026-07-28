use super::*;
use crate::Warning;

struct ImmediateRoutes<'a> {
    routes: Vec<&'a Route>,
    completes: bool,
}

#[derive(Clone, Copy)]
struct FlowBehavior {
    emits: bool,
    completes: bool,
}

pub(in crate::check) fn immediate_handler_cycle_warnings(document: &Document) -> Vec<Warning> {
    let handlers = &document.handlers;
    let indices = handlers
        .iter()
        .enumerate()
        .map(|(index, handler)| (handler.name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut edges = Vec::new();
    let mut connected = vec![vec![false; handlers.len()]; handlers.len()];

    for (source, handler) in handlers.iter().enumerate() {
        if handler
            .statements
            .iter()
            .any(|statement| matches!(statement, Statement::ReturnIf { .. }))
        {
            continue;
        }
        for statement in &handler.statements {
            for route in immediate_routes(statement).routes {
                let Some(&target) = indices.get(route.handler.as_str()) else {
                    continue;
                };
                connected[source][target] = true;
                edges.push((source, target, &route.span));
            }
        }
    }

    for via in 0..handlers.len() {
        let via_targets = connected[via].clone();
        for targets in &mut connected {
            if !targets[via] {
                continue;
            }
            for (reachable, via_reachable) in targets.iter_mut().zip(&via_targets) {
                *reachable |= via_reachable;
            }
        }
    }

    let mut reported = vec![false; handlers.len()];
    let mut warnings = Vec::new();
    for first in 0..handlers.len() {
        if reported[first] || !connected[first][first] {
            continue;
        }
        let members = (0..handlers.len())
            .filter(|&candidate| connected[first][candidate] && connected[candidate][first])
            .collect::<Vec<_>>();
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
            format!(
                "handler `{}` immediately routes back to itself and can refresh forever",
                names[0]
            )
        } else {
            let names = names
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("handlers {names} form an immediate routing cycle that can refresh forever")
        };
        warnings.push(
            Warning::new("W004", span, message)
                .hint("break the immediate back-edge or add a `return if ...` termination guard"),
        );
    }
    warnings
}

fn immediate_routes(statement: &Statement) -> ImmediateRoutes<'_> {
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
            TaskGroupKind::Sequential => {
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
        },
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
