use super::*;
use crate::Warning;
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
    let mut queue = VecDeque::new();

    collect_view_routes(&document.view, &mut queue);
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        collect_view_routes(mount, &mut queue);
    }
    for subscription in &document.subscriptions {
        queue.push_back(subscription.route.handler.as_str());
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
        let mut queue = VecDeque::new();
        collect_view_routes(&component.root, &mut queue);
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
                    "handler `{}` is unreachable from mount, the app view, subscriptions, presets, and tests",
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
            | Statement::Abort { .. }
            | Statement::DebugStart { .. }
            | Statement::DebugFinish { .. }
            | Statement::ClipboardWrite { .. } => {}
        }
    }
}

fn push_route<'a>(route: Option<&'a Route>, output: &mut VecDeque<&'a str>) {
    if let Some(route) = route {
        output.push_back(&route.handler);
    }
}

fn push_routes<'a>(
    routes: impl IntoIterator<Item = Option<&'a Route>>,
    output: &mut VecDeque<&'a str>,
) {
    for route in routes {
        push_route(route, output);
    }
}

fn collect_view_routes<'a>(node: &'a ViewNode, output: &mut VecDeque<&'a str>) {
    match node {
        ViewNode::Layout {
            options, children, ..
        } => {
            if let Some(scroll) = &options.scroll {
                push_routes(
                    [scroll.route.as_ref(), scroll.viewport_route.as_ref()],
                    output,
                );
            }
            for child in children {
                collect_view_routes(child, output);
            }
        }
        ViewNode::Container { content, .. }
        | ViewNode::Lazy { child: content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. } => collect_view_routes(content, output),
        ViewNode::Overlay {
            options,
            content,
            layer,
            ..
        } => {
            push_route(options.dismiss.as_ref(), output);
            collect_view_routes(content, output);
            collect_view_routes(layer, output);
        }
        ViewNode::PaneGrid {
            options,
            panes,
            templates,
            ..
        } => {
            push_route(options.click.as_ref(), output);
            for child in panes
                .iter()
                .flat_map(PaneView::nodes)
                .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            {
                collect_view_routes(child, output);
            }
        }
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
        ViewNode::Button { content, route, .. } => {
            output.push_back(&route.handler);
            if let Some(content) = content {
                collect_view_routes(content, output);
            }
        }
        ViewNode::Checkbox { route, .. }
        | ViewNode::Toggler { route, .. }
        | ViewNode::Radio { route, .. }
        | ViewNode::Markdown { route, .. } => output.push_back(&route.handler),
        ViewNode::Slider { route, release, .. } => {
            output.push_back(&route.handler);
            push_route(release.as_ref(), output);
        }
        ViewNode::PickList {
            options_config,
            route,
            ..
        } => {
            output.push_back(&route.handler);
            push_routes(
                [options_config.open.as_ref(), options_config.close.as_ref()],
                output,
            );
        }
        ViewNode::ComboBox { options, route, .. } => {
            output.push_back(&route.handler);
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
        ViewNode::If { children, .. } | ViewNode::For { children, .. } => {
            for child in children {
                collect_view_routes(child, output);
            }
        }
        ViewNode::Match { arms, .. } => {
            for child in arms.iter().flat_map(|arm| &arm.children) {
                collect_view_routes(child, output);
            }
        }
        ViewNode::KeyedColumn { child, .. } => collect_view_routes(child, output),
        ViewNode::TextEditor { options, .. } => {
            push_route(options.key_binding_route.as_ref(), output);
        }
        ViewNode::Table { columns, .. } => {
            for column in columns {
                collect_view_routes(&column.header, output);
                collect_view_routes(&column.cell, output);
            }
        }
        ViewNode::Component {
            slots,
            events,
            route,
            ..
        } => {
            push_route(route.as_ref(), output);
            for event in events {
                push_route(event.route.as_ref(), output);
            }
            for slot in slots {
                collect_view_routes(&slot.content, output);
            }
        }
        ViewNode::Tooltip { content, tip, .. } => {
            collect_view_routes(content, output);
            collect_view_routes(tip, output);
        }
        ViewNode::MouseArea {
            options, content, ..
        } => {
            push_routes(
                [
                    options.press.as_ref(),
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
            );
            collect_view_routes(content, output);
        }
        ViewNode::ResizeHandle {
            options, content, ..
        } => {
            push_routes(
                [
                    options.drag.as_ref(),
                    options.press.as_ref(),
                    options.release.as_ref(),
                ],
                output,
            );
            collect_view_routes(content, output);
        }
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
                    output.push_back(&route.handler);
                }
            }
        }
        ViewNode::Sensor {
            options, content, ..
        } => {
            push_routes(
                [
                    options.show.as_ref(),
                    options.resize.as_ref(),
                    options.hide.as_ref(),
                ],
                output,
            );
            collect_view_routes(content, output);
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                collect_view_routes(narrow, output);
                collect_view_routes(wide, output);
            }
            ResponsiveContent::Size { content, .. } => collect_view_routes(content, output),
        },
        ViewNode::Text { .. }
        | ViewNode::Progress { .. }
        | ViewNode::Rule { .. }
        | ViewNode::QrCode { .. }
        | ViewNode::Space { .. }
        | ViewNode::Slot { .. }
        | ViewNode::Media { .. } => {}
    }
}
