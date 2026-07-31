use crate::ast::*;
use crate::{CheckedDocument, Error};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ComponentCallId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentParamId {
    component: ComponentId,
    index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentEventId {
    component: ComponentId,
    index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ComponentSlotId {
    component: ComponentId,
    index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentStateId {
    component: ComponentId,
    index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AppStateId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct OriginId(u32);

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Origin {
    path: Option<PathBuf>,
    line: usize,
    column: usize,
    parent: Option<OriginId>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ComponentParamContract {
    id: ComponentParamId,
    name: String,
    ty: Type,
    capability: ParamCapability,
    default: Option<Expr>,
    origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamCapability {
    Read,
    Bind,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ComponentEventContract {
    id: ComponentEventId,
    name: String,
    payloads: Vec<Type>,
    origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ComponentSlotContract {
    id: ComponentSlotId,
    name: String,
    optional: bool,
    origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ComponentStateContract {
    id: ComponentStateId,
    pub(crate) source: State,
    origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComponentStorage {
    Stateless,
    Retained,
    Mounted,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct ComponentContract {
    pub(crate) id: ComponentId,
    pub(crate) name: String,
    params: Vec<ComponentParamContract>,
    pub(crate) output: Type,
    events: Vec<ComponentEventContract>,
    slots: Vec<ComponentSlotContract>,
    pub(crate) states: Vec<ComponentStateContract>,
    pub(crate) handlers: Vec<Handler>,
    pub(crate) root: ViewNode,
    pub(crate) storage: ComponentStorage,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug)]
enum ComponentWritable {
    Param(ComponentParamId),
    State(ComponentStateId),
}

#[derive(Debug)]
struct ComponentIndex {
    params_by_name: HashMap<String, usize>,
    events_by_name: HashMap<String, usize>,
    slots_by_name: HashMap<String, usize>,
    writable_by_name: HashMap<String, ComponentWritable>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArgumentScope {
    Caller,
    Definition,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum WritableStateRef {
    App { id: AppStateId, name: String },
    ComponentParam { id: ComponentParamId, name: String },
    ComponentState { id: ComponentStateId, name: String },
}

impl WritableStateRef {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::App { name, .. }
            | Self::ComponentParam { name, .. }
            | Self::ComponentState { name, .. } => name,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedArgument {
    param: ComponentParamId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) expression: Expr,
    scope: ArgumentScope,
    pub(crate) writable: Option<WritableStateRef>,
    origin: OriginId,
}

impl ResolvedArgument {
    pub(crate) fn uses_definition_scope(&self) -> bool {
        self.scope == ArgumentScope::Definition
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ResolvedEventRoute {
    Direct {
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
        route: Route,
        origin: OriginId,
    },
    Forward {
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
        outer_component: ComponentId,
        outer_event: ComponentEventId,
        origin: OriginId,
    },
}

impl ResolvedEventRoute {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Direct { name, .. } | Self::Forward { name, .. } => name,
        }
    }

    pub(crate) fn payloads(&self) -> &[Type] {
        match self {
            Self::Direct { payloads, .. } | Self::Forward { payloads, .. } => payloads,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedSlot {
    slot: ComponentSlotId,
    pub(crate) name: String,
    pub(crate) optional: bool,
    pub(crate) content: Option<ViewNode>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ComponentScope {
    Explicit {
        id: Id,
        origin: OriginId,
    },
    Implicit {
        component: ComponentId,
        call_site: usize,
        origin: OriginId,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ComponentOutputRoute {
    None,
    Direct {
        output: Type,
        route: Route,
        origin: OriginId,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ComponentCall {
    id: ComponentCallId,
    pub(crate) component: ComponentId,
    pub(crate) origin: OriginId,
    pub(crate) arguments: Vec<ResolvedArgument>,
    pub(crate) events: Vec<ResolvedEventRoute>,
    pub(crate) slots: Vec<ResolvedSlot>,
    pub(crate) output: ComponentOutputRoute,
    pub(crate) scope: ComponentScope,
    pub(crate) storage: ComponentStorage,
    pub(crate) binding_site: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CallSite {
    line: usize,
    column: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct LoweredProgram {
    document: Document,
    components: Vec<ComponentContract>,
    calls: Vec<ComponentCall>,
    calls_by_site: HashMap<CallSite, ComponentCallId>,
    origins: Vec<Origin>,
    source_origins: Vec<(PathBuf, usize)>,
}

impl LoweredProgram {
    pub(crate) fn document(&self) -> &Document {
        &self.document
    }

    pub(crate) fn components(&self) -> &[ComponentContract] {
        &self.components
    }

    pub(crate) fn component(&self, id: ComponentId) -> &ComponentContract {
        &self.components[id.0 as usize]
    }

    pub(crate) fn component_call(&self, span: &Span) -> Result<&ComponentCall, Error> {
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        let id = self.calls_by_site.get(&site).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "component call reached code generation without a lowered call",
            )
        })?;
        self.calls.get(id.0 as usize).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "component call references an invalid lowered call ID",
            )
        })
    }

    #[cfg(test)]
    fn origin(&self, id: OriginId) -> &Origin {
        &self.origins[id.0 as usize]
    }

    pub(crate) fn source_origin(&self, merged_line: usize) -> Option<(&Path, usize)> {
        self.source_origins
            .get(merged_line.checked_sub(1)?)
            .map(|(path, line)| (path.as_path(), *line))
    }
}

pub(crate) fn lower(checked: CheckedDocument) -> Result<LoweredProgram, Error> {
    Lowerer::new(checked).lower()
}

struct Lowerer {
    document: Document,
    source_origins: Vec<(PathBuf, usize)>,
    components: Vec<ComponentContract>,
    component_indexes: Vec<ComponentIndex>,
    component_ids: HashMap<String, ComponentId>,
    calls: Vec<ComponentCall>,
    calls_by_site: HashMap<CallSite, ComponentCallId>,
    origins: Vec<Origin>,
    app_states: HashMap<String, AppStateId>,
}

impl Lowerer {
    fn new(checked: CheckedDocument) -> Self {
        let CheckedDocument {
            document,
            source_origins,
            ..
        } = checked;
        let app_states = document
            .states
            .iter()
            .enumerate()
            .map(|(index, state)| (state.name.clone(), AppStateId(index as u32)))
            .collect();
        Self {
            document,
            source_origins,
            components: Vec::new(),
            component_indexes: Vec::new(),
            component_ids: HashMap::new(),
            calls: Vec::new(),
            calls_by_site: HashMap::new(),
            origins: Vec::new(),
            app_states,
        }
    }

    fn lower(mut self) -> Result<LoweredProgram, Error> {
        self.index_components()?;
        let component_roots = self
            .components
            .iter()
            .map(|component| (component.id, component.root.clone()))
            .collect::<Vec<_>>();
        for (component, root) in component_roots {
            self.lower_view(&root, Some(component))?;
        }
        let app_view = self.document.view.clone();
        self.lower_view(&app_view, None)?;
        let mounts = self
            .document
            .tests
            .iter()
            .filter_map(|test| test.mount.clone())
            .collect::<Vec<_>>();
        for mount in mounts {
            self.lower_view(&mount, None)?;
        }
        Ok(LoweredProgram {
            document: self.document,
            components: self.components,
            calls: self.calls,
            calls_by_site: self.calls_by_site,
            origins: self.origins,
            source_origins: self.source_origins,
        })
    }

    fn index_components(&mut self) -> Result<(), Error> {
        let source_components = self.document.components.clone();
        for (index, component) in source_components.iter().enumerate() {
            let id = ComponentId(index as u32);
            if self
                .component_ids
                .insert(component.name.clone(), id)
                .is_some()
            {
                return Err(self.invariant(
                    &component.span,
                    format!("duplicate checked component `{}`", component.name),
                ));
            }
        }
        for (index, component) in source_components.into_iter().enumerate() {
            let id = ComponentId(index as u32);
            let origin = self.push_origin(&component.span, None);
            let mut params = Vec::with_capacity(component.params.len());
            for (index, param) in component.params.iter().enumerate() {
                params.push(ComponentParamContract {
                    id: ComponentParamId {
                        component: id,
                        index: index as u32,
                    },
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    capability: if param.bind {
                        ParamCapability::Bind
                    } else {
                        ParamCapability::Read
                    },
                    default: param.default.clone(),
                    origin: self.push_origin(&component.span, Some(origin)),
                });
            }
            let events: Vec<ComponentEventContract> = component
                .events
                .iter()
                .enumerate()
                .map(|(index, event)| ComponentEventContract {
                    id: ComponentEventId {
                        component: id,
                        index: index as u32,
                    },
                    name: event.name.clone(),
                    payloads: event.payloads.clone(),
                    origin: self.push_origin(&event.span, Some(origin)),
                })
                .collect();
            let slots: Vec<ComponentSlotContract> = declared_slots(&component.root)
                .into_iter()
                .enumerate()
                .map(|(index, (name, optional, span))| ComponentSlotContract {
                    id: ComponentSlotId {
                        component: id,
                        index: index as u32,
                    },
                    name,
                    optional,
                    origin: self.push_origin(&span, Some(origin)),
                })
                .collect();
            let states = component
                .states
                .iter()
                .enumerate()
                .map(|(index, state)| ComponentStateContract {
                    id: ComponentStateId {
                        component: id,
                        index: index as u32,
                    },
                    source: state.clone(),
                    origin: self.push_origin(&state.span, Some(origin)),
                })
                .collect::<Vec<_>>();
            let stateful = !states.is_empty() || !component.handlers.is_empty();
            let storage = match (stateful, component.lifetime) {
                (false, _) => ComponentStorage::Stateless,
                (true, ComponentLifetime::Retained) => ComponentStorage::Retained,
                (true, ComponentLifetime::Mounted) => ComponentStorage::Mounted,
            };
            let params_by_name = params
                .iter()
                .enumerate()
                .map(|(index, param)| (param.name.clone(), index))
                .collect();
            let events_by_name = events
                .iter()
                .enumerate()
                .map(|(index, event)| (event.name.clone(), index))
                .collect();
            let slots_by_name = slots
                .iter()
                .enumerate()
                .map(|(index, slot)| (slot.name.clone(), index))
                .collect();
            let writable_by_name = params
                .iter()
                .filter(|param| param.capability == ParamCapability::Bind)
                .map(|param| (param.name.clone(), ComponentWritable::Param(param.id)))
                .chain(states.iter().map(|state| {
                    (
                        state.source.name.clone(),
                        ComponentWritable::State(state.id),
                    )
                }))
                .collect();
            self.components.push(ComponentContract {
                id,
                name: component.name,
                params,
                output: component.output,
                events,
                slots,
                states,
                handlers: component.handlers,
                root: component.root,
                storage,
                origin,
            });
            self.component_indexes.push(ComponentIndex {
                params_by_name,
                events_by_name,
                slots_by_name,
                writable_by_name,
            });
        }
        Ok(())
    }

    fn lower_view(
        &mut self,
        node: &ViewNode,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        match node {
            ViewNode::Component {
                name,
                args,
                id,
                slots,
                events,
                route,
                span,
            } => {
                self.lower_component_call(
                    name,
                    args,
                    id,
                    slots,
                    events,
                    route,
                    span,
                    outer_component,
                )?;
                for slot in slots {
                    self.lower_view(&slot.content, outer_component)?;
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Match { arms, .. } => {
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    self.lower_view(child, outer_component)?;
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
            | ViewNode::Lazy { child: content, .. } => {
                self.lower_view(content, outer_component)?;
            }
            ViewNode::Tooltip { content, tip, .. } => {
                self.lower_view(content, outer_component)?;
                self.lower_view(tip, outer_component)?;
            }
            ViewNode::Overlay { content, layer, .. } => {
                self.lower_view(content, outer_component)?;
                self.lower_view(layer, outer_component)?;
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    self.lower_view(&column.header, outer_component)?;
                    self.lower_view(&column.cell, outer_component)?;
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    self.lower_view(narrow, outer_component)?;
                    self.lower_view(wide, outer_component)?;
                }
                ResponsiveContent::Size { content, .. } => {
                    self.lower_view(content, outer_component)?;
                }
            },
            _ => {}
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_component_call(
        &mut self,
        name: &str,
        supplied_args: &[ComponentArg],
        id: &Option<Id>,
        supplied_slots: &[ComponentSlot],
        supplied_events: &[ComponentEventRoute],
        route: &Option<Route>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let component_id =
            self.component_ids.get(name).copied().ok_or_else(|| {
                self.invariant(span, format!("unknown checked component `{name}`"))
            })?;
        let component_index = component_id.0 as usize;
        let supplied_args = {
            let contract = &self.components[component_index];
            let index = &self.component_indexes[component_index];
            let mut ordered = vec![None; contract.params.len()];
            for supplied in supplied_args {
                let position = index.params_by_name.get(&supplied.name).ok_or_else(|| {
                    self.invariant(span, format!("unknown checked prop `{}`", supplied.name))
                })?;
                if ordered[*position].replace(supplied).is_some() {
                    return Err(
                        self.invariant(span, format!("duplicate checked prop `{}`", supplied.name))
                    );
                }
            }
            ordered
        };
        let supplied_events = {
            let contract = &self.components[component_index];
            let index = &self.component_indexes[component_index];
            let mut ordered = vec![None; contract.events.len()];
            for supplied in supplied_events {
                let position = index.events_by_name.get(&supplied.name).ok_or_else(|| {
                    self.invariant(
                        &supplied.span,
                        format!("unknown checked event `{}`", supplied.name),
                    )
                })?;
                if ordered[*position].replace(supplied).is_some() {
                    return Err(self.invariant(
                        &supplied.span,
                        format!("duplicate checked event `{}`", supplied.name),
                    ));
                }
            }
            ordered
        };
        let supplied_slots = {
            let contract = &self.components[component_index];
            let index = &self.component_indexes[component_index];
            let mut ordered = vec![None; contract.slots.len()];
            for supplied in supplied_slots {
                let position = index.slots_by_name.get(&supplied.name).ok_or_else(|| {
                    self.invariant(
                        &supplied.span,
                        format!("unknown checked slot `{}`", supplied.name),
                    )
                })?;
                if ordered[*position].replace(supplied).is_some() {
                    return Err(self.invariant(
                        &supplied.span,
                        format!("duplicate checked slot `{}`", supplied.name),
                    ));
                }
            }
            ordered
        };
        // Calls need the compact semantic shape below, never the component body,
        // states, or handlers. ComponentContract intentionally is not Clone so a
        // future call-site change cannot accidentally restore whole-contract copies.
        let (params, events, slots, output_ty, storage) = {
            let contract = &self.components[component_index];
            (
                contract.params.clone(),
                contract.events.clone(),
                contract.slots.clone(),
                contract.output.clone(),
                contract.storage,
            )
        };
        let origin = self.push_origin(span, None);
        let mut arguments = Vec::with_capacity(params.len());
        for (param, supplied) in params.iter().zip(supplied_args) {
            let (expression, scope) = if let Some(arg) = supplied {
                (arg.value.clone(), ArgumentScope::Caller)
            } else {
                (
                    param.default.clone().ok_or_else(|| {
                        self.invariant(
                            span,
                            format!("required prop `{}` has no checked argument", param.name),
                        )
                    })?,
                    ArgumentScope::Definition,
                )
            };
            let writable = if param.capability == ParamCapability::Bind {
                Some(self.resolve_writable(&expression, outer_component, span)?)
            } else {
                None
            };
            let argument_origin = self.push_origin(span, Some(origin));
            arguments.push(ResolvedArgument {
                param: param.id,
                name: param.name.clone(),
                ty: param.ty.clone(),
                expression,
                scope,
                writable,
                origin: argument_origin,
            });
        }

        let mut resolved_events = Vec::with_capacity(events.len());
        for (event, supplied) in events.iter().zip(supplied_events) {
            let supplied = supplied.ok_or_else(|| {
                self.invariant(span, format!("event `{}` has no checked route", event.name))
            })?;
            let event_origin = self.push_origin(&supplied.span, Some(origin));
            if let Some(route) = &supplied.route {
                resolved_events.push(ResolvedEventRoute::Direct {
                    event: event.id,
                    name: event.name.clone(),
                    payloads: event.payloads.clone(),
                    route: route.clone(),
                    origin: event_origin,
                });
            } else {
                let outer = outer_component.ok_or_else(|| {
                    self.invariant(&supplied.span, "forwarded event has no outer component")
                })?;
                let outer_index = outer.0 as usize;
                let outer_event = self.component_indexes[outer_index]
                    .events_by_name
                    .get(&event.name)
                    .and_then(|position| self.components[outer_index].events.get(*position))
                    .ok_or_else(|| {
                        self.invariant(
                            &supplied.span,
                            format!("forwarded event `{}` has no outer declaration", event.name),
                        )
                    })?
                    .id;
                resolved_events.push(ResolvedEventRoute::Forward {
                    event: event.id,
                    name: event.name.clone(),
                    payloads: event.payloads.clone(),
                    outer_component: outer,
                    outer_event,
                    origin: event_origin,
                });
            }
        }

        let mut resolved_slots = Vec::with_capacity(slots.len());
        for (declared, supplied) in slots.iter().zip(supplied_slots) {
            if supplied.is_none() && !declared.optional {
                return Err(self.invariant(
                    span,
                    format!("required slot `{}` has no checked content", declared.name),
                ));
            }
            resolved_slots.push(ResolvedSlot {
                slot: declared.id,
                name: declared.name.clone(),
                optional: declared.optional,
                content: supplied.map(|slot| (*slot.content).clone()),
                origin: supplied.map_or(declared.origin, |slot| {
                    self.push_origin(&slot.span, Some(origin))
                }),
            });
        }

        let output = match (&output_ty, route) {
            (Type::Unit, None) => ComponentOutputRoute::None,
            (output, Some(route)) => ComponentOutputRoute::Direct {
                output: output.clone(),
                route: route.clone(),
                origin,
            },
            _ => {
                return Err(
                    self.invariant(span, "component output route was not resolved by checking")
                );
            }
        };
        let scope = id.as_ref().map_or_else(
            || ComponentScope::Implicit {
                component: component_id,
                call_site: span.line,
                origin,
            },
            |id| ComponentScope::Explicit {
                id: id.clone(),
                origin,
            },
        );
        let call_id = ComponentCallId(self.calls.len() as u32);
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        if self.calls_by_site.insert(site, call_id).is_some() {
            return Err(self.invariant(span, "component call source identity is not unique"));
        }
        self.calls.push(ComponentCall {
            id: call_id,
            component: component_id,
            origin,
            arguments,
            events: resolved_events,
            slots: resolved_slots,
            output,
            scope,
            storage,
            binding_site: span.line,
        });
        Ok(())
    }

    fn resolve_writable(
        &self,
        expression: &Expr,
        outer_component: Option<ComponentId>,
        span: &Span,
    ) -> Result<WritableStateRef, Error> {
        let Expr::Path(path) = expression else {
            return Err(self.invariant(span, "bind argument is not a direct path"));
        };
        let [name] = path.as_slice() else {
            return Err(self.invariant(span, "bind argument is not a direct state path"));
        };
        if let Some(component) = outer_component
            && let Some(writable) = self.component_indexes[component.0 as usize]
                .writable_by_name
                .get(name)
        {
            return Ok(match writable {
                ComponentWritable::Param(id) => WritableStateRef::ComponentParam {
                    id: *id,
                    name: name.clone(),
                },
                ComponentWritable::State(id) => WritableStateRef::ComponentState {
                    id: *id,
                    name: name.clone(),
                },
            });
        }
        if let Some(id) = self.app_states.get(name) {
            return Ok(WritableStateRef::App {
                id: *id,
                name: name.clone(),
            });
        }
        Err(self.invariant(
            span,
            format!("bind argument `{name}` has no resolved writable state"),
        ))
    }

    fn push_origin(&mut self, span: &Span, parent: Option<OriginId>) -> OriginId {
        let (path, line) = self
            .source_origins
            .get(span.line.saturating_sub(1))
            .map_or((None, span.line), |(path, line)| {
                (Some(path.clone()), *line)
            });
        let id = OriginId(self.origins.len() as u32);
        self.origins.push(Origin {
            path,
            line,
            column: span.column,
            parent,
        });
        id
    }

    fn invariant(&self, span: &Span, message: impl Into<String>) -> Error {
        Error::new(
            "E196",
            span,
            format!("lowering invariant failed: {}", message.into()),
        )
    }
}

fn declared_slots(node: &ViewNode) -> Vec<(String, bool, Span)> {
    fn collect(node: &ViewNode, output: &mut Vec<(String, bool, Span)>) {
        match node {
            ViewNode::Slot {
                name,
                optional,
                span,
            } => output.push((name.clone(), *optional, span.clone())),
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
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
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
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    collect(child, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
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
    collect(node, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, analyze_file};
    use std::fmt::Write as _;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const THEME: &str = "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";

    #[test]
    fn lowers_component_calls_into_ordered_complete_contracts() {
        let source = format!(
            "app Demo\n{THEME}state\n  draft = \"Draft\"\n  checked = false\non changed(value)\n  draft = value\non toggled(value)\n  checked = value\ncomponent Field(bind value:str, label:str=\"Name\")\n  emits\n    change(str)\n  lifetime mounted\n  state\n    local = \"\"\n  col\n    text label\n    slot Leading?\n    slot Body\ncomponent Shell(bind value:str)\n  emits\n    change(str)\n  state\n    scratch = \"\"\n  Field value<->value\n    Leading:\n      text \"L\"\n    Body:\n      text \"B\"\n    forward\n      change\ncomponent Choice() -> bool\n  checkbox \"Choice\" checked=false -> emit(_)\nview\n  col\n    Field value<->draft #field\n      Body:\n        text \"Body\"\n      events\n        change -> changed _\n    Choice -> toggled _\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        assert_eq!(program.components.len(), 3);
        let field = &program.components[0];
        assert_eq!(field.name, "Field");
        assert_eq!(field.params.len(), 2);
        assert_eq!(field.slots.len(), 2);
        assert_eq!(field.storage, ComponentStorage::Mounted);
        assert_eq!(program.components[1].storage, ComponentStorage::Retained);

        let nested = program
            .calls
            .iter()
            .find(|call| matches!(call.events[0], ResolvedEventRoute::Forward { .. }))
            .unwrap();
        assert_eq!(nested.arguments[0].name, "value");
        assert!(matches!(
            nested.arguments[0].writable,
            Some(WritableStateRef::ComponentParam { .. })
        ));
        assert!(nested.arguments[1].uses_definition_scope());
        assert_eq!(
            nested
                .slots
                .iter()
                .filter(|slot| slot.content.is_some())
                .count(),
            2
        );
        assert!(matches!(nested.scope, ComponentScope::Implicit { .. }));

        let root = program
            .calls
            .iter()
            .find(|call| matches!(call.scope, ComponentScope::Explicit { .. }))
            .unwrap();
        assert!(matches!(
            root.arguments[0].writable,
            Some(WritableStateRef::App { .. })
        ));
        assert!(matches!(root.events[0], ResolvedEventRoute::Direct { .. }));
        assert!(matches!(root.scope, ComponentScope::Explicit { .. }));
        assert!(
            root.slots
                .iter()
                .any(|slot| { slot.name == "Leading" && slot.optional && slot.content.is_none() })
        );
        assert!(
            root.slots
                .iter()
                .any(|slot| slot.name == "Body" && slot.content.is_some())
        );
        let output = program
            .calls
            .iter()
            .find(|call| matches!(call.output, ComponentOutputRoute::Direct { .. }))
            .unwrap();
        assert!(matches!(output.output, ComponentOutputRoute::Direct { .. }));
    }

    #[test]
    fn keeps_parented_source_origins() {
        let source = format!(
            "app Demo\n{THEME}component Card(title:str=\"Default\")\n  text title\nview\n  Card\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let call = &program.calls[0];
        let call_origin = program.origin(call.origin);
        assert_eq!(call_origin.line, source.lines().count());
        assert_eq!(call_origin.column, 1);
        let component = &program.components[0];
        assert_eq!(
            program.origin(component.origin).line,
            source.lines().count() - 3
        );
        assert_eq!(
            program.origin(component.params[0].origin).parent,
            Some(component.origin)
        );
    }

    #[test]
    fn rejects_a_checked_component_call_that_cannot_be_resolved() {
        let source = format!("app Demo\n{THEME}component Card()\n  text \"Card\"\nview\n  Card\n");
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Component { name, .. } = &mut checked.document.view else {
            panic!("fixture root must be a component call");
        };
        *name = "Missing".into();

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, source.lines().count());
        assert!(
            error
                .message
                .contains("unknown checked component `Missing`")
        );
    }

    #[test]
    fn lowering_large_component_call_surface_stays_linear() {
        let mut source = format!(
            "app Demo\n{THEME}component Badge(label:str=\"Badge\")\n  text label\nview\n  col\n"
        );
        for _ in 0..10_000 {
            source.push_str("    Badge\n");
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.calls.len(), 10_000);
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "10k component calls lowered in {elapsed:?}"
        );
    }

    #[test]
    fn lowering_wide_component_calls_scales_with_call_surface_not_contract_body() {
        const CALLS: usize = 2_000;
        const DEFAULT_PARAMS: usize = 32;
        const EVENTS: usize = 32;
        const SLOTS: usize = 32;
        const STATES: usize = 128;
        const HANDLERS: usize = 128;
        const BODY_NODES: usize = 256;

        let mut source = format!(
            "app Demo\n{THEME}state\n  draft = \"Draft\"\non changed\n  draft = \"Changed\"\ncomponent Wide(bind value:str"
        );
        for index in 0..DEFAULT_PARAMS {
            write!(source, ", prop_{index}:str=\"value-{index}\"").unwrap();
        }
        source.push_str(")\n  emits\n");
        for index in 0..EVENTS {
            writeln!(source, "    event_{index}").unwrap();
        }
        source.push_str("  lifetime mounted\n  state\n");
        for index in 0..STATES {
            writeln!(source, "    local_{index} = \"state-{index}\"").unwrap();
        }
        for index in 0..HANDLERS {
            writeln!(
                source,
                "  on reset_{index}\n    local_{index} = \"reset-{index}\""
            )
            .unwrap();
        }
        source.push_str("  col\n    text value\n");
        for index in 0..BODY_NODES {
            writeln!(source, "    text \"body-{index}\"").unwrap();
        }
        for index in 0..SLOTS {
            writeln!(source, "    slot Slot{index}?").unwrap();
        }
        source.push_str("view\n  col\n    Wide value<->draft\n      events\n");
        for index in 0..EVENTS {
            writeln!(source, "        event_{index} -> changed").unwrap();
        }

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture view must be a layout");
        };
        let prototype = children.pop().expect("fixture has one component call");
        let first_synthetic_line = source.lines().count() + 1;
        children.extend((0..CALLS).map(|index| {
            let mut call = prototype.clone();
            let ViewNode::Component { span, .. } = &mut call else {
                panic!("fixture child must be a component call");
            };
            *span = Span::line(first_synthetic_line + index);
            call
        }));
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.calls.len(), CALLS);
        assert_eq!(
            program
                .calls
                .iter()
                .map(|call| call.arguments.len() + call.events.len() + call.slots.len())
                .sum::<usize>(),
            CALLS * (1 + DEFAULT_PARAMS + EVENTS + SLOTS)
        );
        let ViewNode::Layout { children, .. } = &program.components[0].root else {
            panic!("wide component root must remain a layout");
        };
        assert_eq!(children.len(), BODY_NODES + SLOTS + 1);
        assert_eq!(program.components[0].states.len(), STATES);
        assert_eq!(program.components[0].handlers.len(), HANDLERS);
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "2k calls to a wide component lowered in {elapsed:?}"
        );
    }

    #[test]
    fn resolves_namespaced_import_calls_and_their_physical_origins() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-lowered-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("parts.ice");
        fs::write(
            &root,
            format!("app Demo\nuse \"parts.ice\" as ui\n{THEME}view\n  ui::Outer\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Inner()\n  text \"Inner\"\ncomponent Outer()\n  state\n    value = \"\"\n  Inner\n",
        )
        .unwrap();

        let program = lower(analyze_file(&root).unwrap()).unwrap();
        let inner = program
            .components
            .iter()
            .find(|component| component.name == "ui::Inner")
            .unwrap();
        let outer = program
            .components
            .iter()
            .find(|component| component.name == "ui::Outer")
            .unwrap();
        let nested = program
            .calls
            .iter()
            .find(|call| call.component == inner.id)
            .unwrap();
        let root_call = program
            .calls
            .iter()
            .find(|call| call.component == outer.id)
            .unwrap();
        let nested_origin = program.origin(nested.origin);
        let root_origin = program.origin(root_call.origin);
        assert_eq!(nested_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(nested_origin.line, 6);
        assert_eq!(root_origin.path.as_deref(), Some(root.as_path()));
        assert_eq!(
            root_origin.line,
            fs::read_to_string(&root).unwrap().lines().count()
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
