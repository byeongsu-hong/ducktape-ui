use super::*;
use crate::Warning;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Clone, Debug)]
enum Scope {
    App,
    Component(String),
    Disabled,
}

#[derive(Clone, Debug)]
struct StateSites {
    component: Option<String>,
    name: String,
    span: Span,
    reads: BTreeSet<(usize, usize)>,
    writes: BTreeSet<(usize, usize)>,
}

#[derive(Debug)]
struct Tracker {
    scope: Scope,
    states: HashMap<(Option<String>, String), StateSites>,
    derived_dependencies: HashMap<String, Vec<String>>,
}

thread_local! {
    static TRACKER: RefCell<Option<Tracker>> = const { RefCell::new(None) };
}

pub(in crate::check) struct UsageSession;

impl UsageSession {
    pub(in crate::check) fn start(document: &Document, reachable: &HashSet<String>) -> Self {
        let mut states = HashMap::new();
        for state in &document.states {
            states.insert(
                (None, state.name.clone()),
                StateSites {
                    component: None,
                    name: state.name.clone(),
                    span: state.span.clone(),
                    reads: BTreeSet::new(),
                    writes: BTreeSet::new(),
                },
            );
        }
        for component in document
            .components
            .iter()
            .filter(|component| reachable.contains(&component.name))
        {
            for state in &component.states {
                states.insert(
                    (Some(component.name.clone()), state.name.clone()),
                    StateSites {
                        component: Some(component.name.clone()),
                        name: state.name.clone(),
                        span: state.span.clone(),
                        reads: BTreeSet::new(),
                        writes: BTreeSet::new(),
                    },
                );
            }
        }
        let derived_dependencies = derived_state_dependencies(document);
        TRACKER.with(|tracker| {
            let previous = tracker.replace(Some(Tracker {
                scope: Scope::App,
                states,
                derived_dependencies,
            }));
            assert!(previous.is_none(), "state usage analysis cannot be nested");
        });
        Self
    }

    pub(in crate::check) fn finish(self) -> Vec<Warning> {
        let tracker = TRACKER
            .with(|tracker| tracker.borrow_mut().take())
            .expect("state usage session");
        let mut states = tracker.states.into_values().collect::<Vec<_>>();
        states.sort_by_key(|state| state.span.line);
        states
            .into_iter()
            .filter_map(|state| {
                let scope = state.component.as_ref().map_or_else(
                    || format!("state `{}`", state.name),
                    |component| format!("state `{}.{}`", component, state.name),
                );
                if state.reads.is_empty() {
                    let message = if state.writes.is_empty() {
                        format!("{scope} is never read or written")
                    } else {
                        format!(
                            "{scope} is written at {} site(s) but never read",
                            state.writes.len()
                        )
                    };
                    Some(Warning::new("W002", &state.span, message).hint(
                        "remove the state or connect it to reachable view behavior",
                    ))
                } else if state.writes.is_empty() {
                    Some(
                        Warning::new(
                            "W003",
                            &state.span,
                            format!(
                                "{scope} is read at {} site(s) but never written; it always keeps its initial value",
                                state.reads.len()
                            ),
                        )
                        .hint("replace it with a constant expression or add the missing state transition"),
                    )
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Drop for UsageSession {
    fn drop(&mut self) {
        TRACKER.with(|tracker| {
            tracker.borrow_mut().take();
        });
    }
}

pub(in crate::check) fn with_component_scope<T>(
    component: &str,
    reachable: bool,
    f: impl FnOnce() -> T,
) -> T {
    let scope = if reachable {
        Scope::Component(component.to_owned())
    } else {
        Scope::Disabled
    };
    with_scope(scope, f)
}

pub(in crate::check) fn with_app_handler_scope<T>(reachable: bool, f: impl FnOnce() -> T) -> T {
    with_scope(
        if reachable {
            Scope::App
        } else {
            Scope::Disabled
        },
        f,
    )
}

pub(in crate::check) fn without_usage<T>(f: impl FnOnce() -> T) -> T {
    with_scope(Scope::Disabled, f)
}

fn with_scope<T>(scope: Scope, f: impl FnOnce() -> T) -> T {
    let previous = TRACKER.with(|tracker| {
        let mut tracker = tracker.borrow_mut();
        let tracker = tracker.as_mut().expect("state usage session");
        std::mem::replace(&mut tracker.scope, scope)
    });
    let output = f();
    TRACKER.with(|tracker| {
        tracker
            .borrow_mut()
            .as_mut()
            .expect("state usage session")
            .scope = previous;
    });
    output
}

pub(in crate::check) fn record_read(name: &str, span: &Span) {
    record(name, span, false);
}

pub(in crate::check) fn record_write(name: &str, span: &Span) {
    record(name, span, true);
}

fn record(name: &str, span: &Span, write: bool) {
    TRACKER.with(|tracker| {
        let mut tracker = tracker.borrow_mut();
        let Some(tracker) = tracker.as_mut() else {
            return;
        };
        let names = match &tracker.scope {
            Scope::App if write => vec![name.to_owned()],
            Scope::App => tracker
                .derived_dependencies
                .get(name)
                .cloned()
                .unwrap_or_else(|| vec![name.to_owned()]),
            Scope::Component(_) => vec![name.to_owned()],
            Scope::Disabled => return,
        };
        let component = match &tracker.scope {
            Scope::App => None,
            Scope::Component(component) => Some(component.clone()),
            Scope::Disabled => return,
        };
        for name in names {
            let Some(state) = tracker.states.get_mut(&(component.clone(), name)) else {
                continue;
            };
            if write {
                state.writes.insert((span.line, span.column));
            } else {
                state.reads.insert((span.line, span.column));
            }
        }
    });
}

fn derived_state_dependencies(document: &Document) -> HashMap<String, Vec<String>> {
    fn paths(expr: &Expr, output: &mut Vec<String>) {
        match expr {
            Expr::Path(path) => {
                if let Some(name) = path.first()
                    && !output.contains(name)
                {
                    output.push(name.clone());
                }
            }
            Expr::List(values) | Expr::Call { args: values, .. } => {
                for value in values {
                    paths(value, output);
                }
            }
            Expr::Unary { value, .. } => paths(value, output),
            Expr::Binary { left, right, .. } => {
                paths(left, output);
                paths(right, output);
            }
            Expr::Bool(_)
            | Expr::I64(_)
            | Expr::F64(_)
            | Expr::Str(_)
            | Expr::Bytes(_)
            | Expr::EmptyList
            | Expr::None => {}
        }
    }

    fn expand(
        name: &str,
        states: &HashSet<String>,
        direct: &HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
        output: &mut Vec<String>,
    ) {
        if states.contains(name) {
            if !output.iter().any(|state| state == name) {
                output.push(name.to_owned());
            }
            return;
        }
        if !visiting.insert(name.to_owned()) {
            return;
        }
        if let Some(dependencies) = direct.get(name) {
            for dependency in dependencies {
                expand(dependency, states, direct, visiting, output);
            }
        }
        visiting.remove(name);
    }

    let states = document
        .states
        .iter()
        .map(|state| state.name.clone())
        .collect::<HashSet<_>>();
    let direct = document
        .derived
        .iter()
        .map(|derived| {
            let mut dependencies = Vec::new();
            paths(&derived.value, &mut dependencies);
            (derived.name.clone(), dependencies)
        })
        .collect::<HashMap<_, _>>();
    direct
        .keys()
        .map(|name| {
            let mut output = Vec::new();
            expand(name, &states, &direct, &mut HashSet::new(), &mut output);
            (name.clone(), output)
        })
        .collect()
}

pub(in crate::check) fn reachable_components(document: &Document) -> HashSet<String> {
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();
    component_references(&document.view, &mut queue);
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        component_references(mount, &mut queue);
    }
    while let Some(name) = queue.pop_front() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(component) = document
            .components
            .iter()
            .find(|component| component.name == name)
        {
            component_references(&component.root, &mut queue);
        }
    }
    reachable
}

pub(in crate::check) fn unreachable_component_warnings(
    document: &Document,
    reachable: &HashSet<String>,
) -> Vec<Warning> {
    document
        .components
        .iter()
        .filter(|component| !reachable.contains(&component.name))
        .map(|component| {
            Warning::new(
                "W001",
                &component.span,
                format!(
                    "component `{}` is unreachable from the app view and every test mount",
                    component.name
                ),
            )
            .hint("mount the component from a reachable view or remove its declaration")
        })
        .collect()
}

fn component_references(node: &ViewNode, output: &mut VecDeque<String>) {
    match node {
        ViewNode::Component { name, slots, .. } => {
            output.push_back(name.clone());
            for slot in slots {
                component_references(&slot.content, output);
            }
        }
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => {
            for child in children {
                component_references(child, output);
            }
        }
        ViewNode::Match { arms, .. } => {
            for child in arms.iter().flat_map(|arm| &arm.children) {
                component_references(child, output);
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
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => component_references(content, output),
        ViewNode::Tooltip { content, tip, .. } => {
            component_references(content, output);
            component_references(tip, output);
        }
        ViewNode::Overlay { content, layer, .. } => {
            component_references(content, output);
            component_references(layer, output);
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => {
            for child in panes
                .iter()
                .flat_map(PaneView::nodes)
                .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            {
                component_references(child, output);
            }
        }
        ViewNode::Table { columns, .. } => {
            for column in columns {
                component_references(&column.header, output);
                component_references(&column.cell, output);
            }
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                component_references(narrow, output);
                component_references(wide, output);
            }
            ResponsiveContent::Size { content, .. } => component_references(content, output),
        },
        _ => {}
    }
}
