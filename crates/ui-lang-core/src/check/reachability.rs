use super::*;
use crate::Warning;
use crate::hir::view_children;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub(in crate::check) struct HandlerReachability {
    pub(in crate::check) app: HashSet<String>,
    pub(in crate::check) components: HashMap<String, HashSet<String>>,
}

impl HandlerReachability {
    pub(in crate::check) fn app_contains(&self, handler: &str) -> bool {
        self.app.contains(handler)
    }

    pub(in crate::check) fn component_contains(&self, component: &str, handler: &str) -> bool {
        self.components
            .get(component)
            .is_some_and(|handlers| handlers.contains(handler))
    }
}

pub(in crate::check) fn reachable_handlers(
    document: &Document,
    reachable_components: &HashSet<String>,
) -> HandlerReachability {
    let app_names = document
        .handlers
        .iter()
        .map(|handler| handler.name.as_str())
        .collect::<HashSet<_>>();
    let mut app = HashSet::new();
    let mut view_routes = Vec::new();

    collect_view_routes(&document.view, &mut view_routes);
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        collect_view_routes(mount, &mut view_routes);
    }
    let mut queue: VecDeque<&str> = view_routes
        .iter()
        .map(|route| route.handler.as_str())
        .collect();
    for subscription in &document.subscriptions {
        queue.push_back(subscription.route.handler.as_str());
    }
    // A tray menu row is an entry point exactly like a subscription source:
    // the platform, not the view, is what calls it.
    if let Some(tray) = &document.settings.tray {
        for row in &tray.menu {
            if let TrayRow::Item {
                route: Some(route), ..
            } = row
            {
                queue.push_back(route.as_str());
            }
        }
    }
    for preset in &document.presets {
        collect_statement_routes(&preset.statements, &mut queue);
    }
    for test in &document.tests {
        for step in &test.steps {
            if let TestStepKind::Dispatch { handler, .. } = &step.kind {
                queue.push_back(handler);
            }
        }
    }
    if app_names.contains("mount") {
        queue.push_back("mount");
    }

    while let Some(name) = queue.pop_front() {
        if !app_names.contains(name) || !app.insert(name.to_owned()) {
            continue;
        }
        let handler = document
            .handlers
            .iter()
            .find(|handler| handler.name == name)
            .expect("reachable app handler");
        collect_statement_routes(&handler.statements, &mut queue);
    }

    let mut components = HashMap::new();
    for component in document
        .components
        .iter()
        .filter(|component| reachable_components.contains(&component.name))
    {
        let names = component
            .handlers
            .iter()
            .map(|handler| handler.name.as_str())
            .collect::<HashSet<_>>();
        let mut reachable = HashSet::new();
        let mut routes = Vec::new();
        collect_view_routes(&component.root, &mut routes);
        let mut queue: VecDeque<&str> = routes.iter().map(|route| route.handler.as_str()).collect();
        // The runtime dispatches `boot` on first sighting; no view routes it.
        queue.push_back("boot");
        // …and a SLICE is the only route its target has: the app hands the
        // payload down, so nothing in the component view names the handler.
        collect_sliced_handlers(document, &component.name, &mut queue);
        while let Some(name) = queue.pop_front() {
            if !names.contains(name) || !reachable.insert(name.to_owned()) {
                continue;
            }
            let handler = component
                .handlers
                .iter()
                .find(|handler| handler.name == name)
                .expect("reachable component handler");
            collect_statement_routes(&handler.statements, &mut queue);
        }
        components.insert(component.name.clone(), reachable);
    }

    HandlerReachability { app, components }
}

pub(in crate::check) fn unreachable_handler_warnings(
    document: &Document,
    reachable_components: &HashSet<String>,
    reachable_handlers: &HandlerReachability,
) -> Vec<Warning> {
    let mut warnings = document
        .handlers
        .iter()
        .filter(|handler| !reachable_handlers.app_contains(&handler.name))
        .map(|handler| {
            Warning::new(
                "W005",
                &handler.span,
                format!(
                    "handler `{}` is unreachable from mount, the app view, subscriptions, the tray menu, presets, and tests",
                    handler.name
                ),
            )
            .hint("route to the handler from a reachable entry point or remove it")
        })
        .collect::<Vec<_>>();

    for component in document
        .components
        .iter()
        .filter(|component| reachable_components.contains(&component.name))
    {
        warnings.extend(
            component
                .handlers
                .iter()
                .filter(|handler| {
                    !reachable_handlers.component_contains(&component.name, &handler.name)
                })
                .map(|handler| {
                    Warning::new(
                        "W005",
                        &handler.span,
                        format!(
                            "component handler `{}.{}` is unreachable from the component view",
                            component.name, handler.name
                        ),
                    )
                    .hint("route to the handler from the component view or remove it")
                }),
        );
    }
    warnings
}

/// Every handler of `component` an app-level `slice` targets. Walks whole
/// handler bodies, so a slice inside a task group counts.
fn collect_sliced_handlers<'a>(
    document: &'a Document,
    component: &str,
    output: &mut VecDeque<&'a str>,
) {
    fn walk<'a>(statements: &'a [Statement], component: &str, output: &mut VecDeque<&'a str>) {
        for statement in statements {
            match statement {
                Statement::Slice {
                    component: target,
                    handler,
                    ..
                } if target == component => output.push_back(handler),
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        walk(&arm.statements, component, output);
                    }
                }
                Statement::TaskGroup { statements, .. } => walk(statements, component, output),
                Statement::Abortable { task, .. } => {
                    walk(::std::slice::from_ref(task.as_ref()), component, output);
                }
                _ => {}
            }
        }
    }

    for handler in &document.handlers {
        walk(&handler.statements, component, output);
    }
}

pub(in crate::check) fn collect_statement_routes<'a>(
    statements: &'a [Statement],
    output: &mut VecDeque<&'a str>,
) {
    for statement in statements {
        match statement {
            Statement::Run { success, error, .. } => {
                output.push_back(&success.handler);
                if let Some(error) = error {
                    output.push_back(&error.handler);
                }
            }
            Statement::Sip {
                progress,
                success,
                error,
                ..
            } => {
                output.push_back(&progress.handler);
                output.push_back(&success.handler);
                if let Some(error) = error {
                    output.push_back(&error.handler);
                }
            }
            Statement::TaskFlow {
                success,
                error,
                units,
                ..
            } => {
                for route in [success, error, units].into_iter().flatten() {
                    output.push_back(&route.handler);
                }
            }
            Statement::TaskGroup { statements, .. } => {
                collect_statement_routes(statements, output);
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    collect_statement_routes(&arm.statements, output);
                }
            }
            Statement::Abortable { task, .. } => {
                collect_statement_routes(std::slice::from_ref(task), output);
            }
            Statement::WidgetOperation { route, .. }
            | Statement::WindowOperation { route, .. }
            | Statement::PaneOperation { route, .. } => {
                if let Some(route) = route {
                    output.push_back(&route.handler);
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
            // The event's call-site route keeps its target handler reachable
            // through the view walk; the emit itself names no route.
            | Statement::Emit { .. }
            // A slice names a component handler directly, and the component's
            // own reachability is decided by whether it is mounted.
            | Statement::Slice { .. } => {}
        }
    }
}

fn push_route<'a>(route: Option<&'a Route>, output: &mut Vec<&'a Route>) {
    if let Some(route) = route {
        output.push(route);
    }
}

fn push_routes<'a>(
    routes: impl IntoIterator<Item = Option<&'a Route>>,
    output: &mut Vec<&'a Route>,
) {
    for route in routes {
        push_route(route, output);
    }
}

pub(in crate::check) fn collect_view_routes<'a>(node: &'a ViewNode, output: &mut Vec<&'a Route>) {
    match node {
        ViewNode::Layout { options, .. } => {
            if let Some(scroll) = &options.scroll {
                push_routes(
                    [scroll.route.as_ref(), scroll.viewport_route.as_ref()],
                    output,
                );
            }
        }
        ViewNode::Overlay { options, .. } => push_route(options.dismiss.as_ref(), output),
        ViewNode::PaneGrid { options, .. } => push_route(options.click.as_ref(), output),
        ViewNode::RichText { route, .. }
        | ViewNode::ExternComponent { route, .. }
        | ViewNode::Themer { route, .. }
        | ViewNode::Shader { route, .. } => push_route(route.as_ref(), output),
        ViewNode::Input { options, .. } => push_routes(
            [
                options.change.as_ref(),
                options.submit.as_ref(),
                options.paste.as_ref(),
            ],
            output,
        ),
        ViewNode::Button { route, .. }
        | ViewNode::Checkbox { route, .. }
        | ViewNode::Toggler { route, .. }
        | ViewNode::Radio { route, .. }
        | ViewNode::Markdown { route, .. } => output.push(route),
        ViewNode::Slider { route, release, .. } => {
            output.push(route);
            push_route(release.as_ref(), output);
        }
        ViewNode::PickList {
            options_config,
            route,
            ..
        } => {
            output.push(route);
            push_routes(
                [options_config.open.as_ref(), options_config.close.as_ref()],
                output,
            );
        }
        ViewNode::ComboBox { options, route, .. } => {
            output.push(route);
            push_routes(
                [
                    options.input.as_ref(),
                    options.hover.as_ref(),
                    options.open.as_ref(),
                    options.close.as_ref(),
                ],
                output,
            );
        }
        ViewNode::TextEditor { options, .. } => {
            push_route(options.key_binding_route.as_ref(), output);
        }
        ViewNode::Component { events, route, .. } => {
            push_route(route.as_ref(), output);
            for event in events {
                push_route(event.route.as_ref(), output);
            }
        }
        ViewNode::MouseArea { options, .. } => push_routes(
            [
                options.press.as_ref(),
                options.press_at.as_ref(),
                options.release.as_ref(),
                options.double_click.as_ref(),
                options.right_press.as_ref(),
                options.right_release.as_ref(),
                options.middle_press.as_ref(),
                options.middle_release.as_ref(),
                options.enter.as_ref(),
                options.move_route.as_ref(),
                options.scroll.as_ref(),
                options.exit.as_ref(),
            ],
            output,
        ),
        ViewNode::ResizeHandle { options, .. } => push_routes(
            [
                options.drag.as_ref(),
                options.press.as_ref(),
                options.release.as_ref(),
            ],
            output,
        ),
        ViewNode::Canvas {
            options, events, ..
        } => {
            push_routes(
                [
                    options.press.as_ref(),
                    options.release.as_ref(),
                    options.right_press.as_ref(),
                    options.right_release.as_ref(),
                    options.middle_press.as_ref(),
                    options.middle_release.as_ref(),
                    options.enter.as_ref(),
                    options.move_route.as_ref(),
                    options.scroll.as_ref(),
                    options.exit.as_ref(),
                ],
                output,
            );
            for event in events {
                if let Some(CanvasEventAction::Route(route)) = &event.action {
                    output.push(route);
                }
            }
        }
        ViewNode::Sensor { options, .. } => push_routes(
            [
                options.show.as_ref(),
                options.resize.as_ref(),
                options.hide.as_ref(),
            ],
            output,
        ),
        // Route-free: listed so a widget that grows a route has to come back
        // here rather than quietly dropping it from reachability.
        ViewNode::Container { .. }
        | ViewNode::Lazy { .. }
        | ViewNode::Theme { .. }
        | ViewNode::Float { .. }
        | ViewNode::Pin { .. }
        | ViewNode::If { .. }
        | ViewNode::For { .. }
        | ViewNode::Match { .. }
        | ViewNode::KeyedColumn { .. }
        | ViewNode::Table { .. }
        | ViewNode::Tooltip { .. }
        | ViewNode::Responsive { .. }
        | ViewNode::Text { .. }
        | ViewNode::Progress { .. }
        | ViewNode::Rule { .. }
        | ViewNode::QrCode { .. }
        | ViewNode::Space { .. }
        | ViewNode::Slot { .. }
        | ViewNode::Media { .. } => {}
    }

    for child in view_children(node) {
        collect_view_routes(child, output);
    }
}
