use super::*;
use crate::hir::view_children;

/// Component events and outputs delivered from inside a `lazy` subtree.
#[derive(Default)]
struct LazyDelivered {
    events: HashSet<String>,
    output: bool,
}

/// A component event or output delivered from inside a `lazy` subtree is
/// captured by the lazy closure as an owned callback, so the call-site route
/// that builds the callback can only carry `_` payloads: an expression there
/// would be evaluated when the closure is built — freezing a stale value —
/// and a state read would borrow view data into the `Element<'static>` tree.
/// Taint propagates through `forward` and `emit` chains because each level
/// embeds the next call site's callback.
pub(in crate::check) fn check_lazy_delivered_routes(document: &Document) -> Result<(), Error> {
    let mut delivered: HashMap<String, LazyDelivered> = document
        .components
        .iter()
        .map(|component| {
            let mut seeds = LazyDelivered::default();
            collect_lazy_seeds(&component.root, component, &mut seeds);
            (component.name.clone(), seeds)
        })
        .collect();

    loop {
        let mut changed = false;
        for (host, root) in view_roots(document) {
            propagate_call_taints(root, host, &mut delivered, &mut changed);
        }
        if !changed {
            break;
        }
    }

    for (_, root) in view_roots(document) {
        validate_call_routes(root, &delivered)?;
    }
    Ok(())
}

fn view_roots(document: &Document) -> impl Iterator<Item = (Option<&Component>, &ViewNode)> {
    std::iter::once((None, &document.view))
        .chain(
            document
                .components
                .iter()
                .map(|component| (Some(component), &component.root)),
        )
        .chain(
            document
                .tests
                .iter()
                .filter_map(|test| test.mount.as_ref().map(|mount| (None, mount))),
        )
}

fn collect_lazy_seeds(node: &ViewNode, component: &Component, seeds: &mut LazyDelivered) {
    if let ViewNode::Lazy { child, .. } = node {
        let mut routes = Vec::new();
        collect_view_routes(child, &mut routes);
        for route in routes {
            if route.handler == "emit" {
                taint_from_emit(route, component, seeds);
            }
        }
        collect_forward_seeds(child, seeds);
        return;
    }
    for child in view_children(node) {
        collect_lazy_seeds(child, component, seeds);
    }
}

fn collect_forward_seeds(node: &ViewNode, seeds: &mut LazyDelivered) {
    if let ViewNode::Component { events, .. } = node {
        for event in events {
            if event.route.is_none() {
                seeds.events.insert(event.name.clone());
            }
        }
    }
    for child in view_children(node) {
        collect_forward_seeds(child, seeds);
    }
}

/// An `emit` route addresses the named event of the enclosing component when
/// its first argument names one, and the component output otherwise —
/// mirroring route inference.
fn taint_from_emit(route: &Route, component: &Component, seeds: &mut LazyDelivered) {
    match emit_event_name(route)
        .filter(|name| component.events.iter().any(|event| event.name == *name))
    {
        Some(name) => {
            seeds.events.insert(name.to_owned());
        }
        None => seeds.output = true,
    }
}

fn emit_event_name(route: &Route) -> Option<&str> {
    let RouteArg::Expr(Expr::Path(path)) = route.args.first()? else {
        return None;
    };
    let [name] = path.as_slice() else {
        return None;
    };
    Some(name)
}

fn propagate_call_taints(
    node: &ViewNode,
    host: Option<&Component>,
    delivered: &mut HashMap<String, LazyDelivered>,
    changed: &mut bool,
) {
    if let ViewNode::Component {
        name,
        events,
        route,
        ..
    } = node
    {
        let tainted_events = events
            .iter()
            .filter(|event| {
                delivered
                    .get(name)
                    .is_some_and(|taints| taints.events.contains(&event.name))
            })
            .map(|event| (event.name.clone(), event.route.clone()))
            .collect::<Vec<_>>();
        let tainted_output = delivered.get(name).is_some_and(|taints| taints.output);
        if let Some(host) = host {
            for (event_name, event_route) in tainted_events {
                match event_route {
                    None => {
                        *changed |= delivered
                            .entry(host.name.clone())
                            .or_default()
                            .events
                            .insert(event_name);
                    }
                    Some(route) if route.handler == "emit" => {
                        *changed |= taint_host_from_emit(&route, host, delivered);
                    }
                    Some(_) => {}
                }
            }
            if tainted_output
                && let Some(route) = route
                && route.handler == "emit"
            {
                *changed |= taint_host_from_emit(route, host, delivered);
            }
        }
    }
    for child in view_children(node) {
        propagate_call_taints(child, host, delivered, changed);
    }
}

fn taint_host_from_emit(
    route: &Route,
    host: &Component,
    delivered: &mut HashMap<String, LazyDelivered>,
) -> bool {
    let entry = delivered.entry(host.name.clone()).or_default();
    match emit_event_name(route).filter(|name| host.events.iter().any(|event| event.name == *name))
    {
        Some(name) => entry.events.insert(name.to_owned()),
        None => !std::mem::replace(&mut entry.output, true),
    }
}

fn validate_call_routes(
    node: &ViewNode,
    delivered: &HashMap<String, LazyDelivered>,
) -> Result<(), Error> {
    if let ViewNode::Component {
        name,
        events,
        route,
        ..
    } = node
        && let Some(taints) = delivered.get(name)
    {
        for event in events {
            if taints.events.contains(&event.name)
                && let Some(route) = &event.route
            {
                require_payload_only_route(route, format!("event `{}` of `{name}`", event.name))?;
            }
        }
        if taints.output
            && let Some(route) = route
        {
            require_payload_only_route(route, format!("the output of `{name}`"))?;
        }
    }
    for child in view_children(node) {
        validate_call_routes(child, delivered)?;
    }
    Ok(())
}

fn require_payload_only_route(route: &Route, delivered: String) -> Result<(), Error> {
    let named_emit = route.handler == "emit" && emit_event_name(route).is_some();
    let mut payloads = route.args.iter().skip(usize::from(named_emit));
    if payloads.all(|arg| matches!(arg, RouteArg::Payload)) {
        return Ok(());
    }
    Err(Error::new(
        "E139",
        &route.span,
        format!("{delivered} is delivered from a `lazy` subtree, so its route accepts only `_` payloads"),
    )
    .hint("the lazy closure owns this callback; an expression here would freeze a stale value or borrow view state"))
}
