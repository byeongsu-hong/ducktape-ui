use crate::ast::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

macro_rules! arena_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) struct $name(pub(crate) u32);
    };
}

arena_id!(ComponentId);
arena_id!(AppStateId);
arena_id!(DerivedId);
arena_id!(TestId);
arena_id!(StructId);
arena_id!(EnumId);
arena_id!(PaletteId);
arena_id!(ExternFnId);
arena_id!(OriginId);
arena_id!(ViewId);
arena_id!(ComponentCallId);
arena_id!(HandlerId);
arena_id!(StatementId);
arena_id!(TaskId);
arena_id!(RouteId);
arena_id!(RunSiteId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentParamId {
    pub(crate) component: ComponentId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentEventId {
    pub(crate) component: ComponentId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentSlotId {
    pub(crate) component: ComponentId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComponentStateId {
    pub(crate) component: ComponentId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StructFieldId {
    pub(crate) owner: StructId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct EnumVariantId {
    pub(crate) owner: EnumId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Declaration<T> {
    pub(crate) id: T,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct StructDeclaration {
    pub(crate) declaration: Declaration<StructId>,
    pub(crate) name: String,
    pub(crate) rust_path: String,
    pub(crate) fields: Vec<StructFieldDeclaration>,
}

#[derive(Clone, Debug)]
pub(crate) struct StructFieldDeclaration {
    pub(crate) declaration: Declaration<StructFieldId>,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct EnumDeclaration {
    pub(crate) declaration: Declaration<EnumId>,
    pub(crate) name: String,
    pub(crate) rust_name: String,
    pub(crate) variants: Vec<EnumVariantDeclaration>,
}

#[derive(Clone, Debug)]
pub(crate) struct EnumVariantDeclaration {
    pub(crate) declaration: Declaration<EnumVariantId>,
    pub(crate) name: String,
    pub(crate) payload: Option<Type>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ExternDeclaration {
    pub(crate) declaration: Declaration<ExternFnId>,
    pub(crate) kind: ExternKind,
    pub(crate) name: String,
    pub(crate) rust_path: String,
    pub(crate) params: Vec<(String, Type)>,
    pub(crate) borrowed: Vec<bool>,
    pub(crate) progress: Option<Type>,
    pub(crate) output: Type,
    pub(crate) error: Option<Type>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Origin {
    pub(crate) path: Option<PathBuf>,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) parent: Option<OriginId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OriginArena {
    origins: Vec<Origin>,
    source_origins: Vec<(PathBuf, usize)>,
}

impl OriginArena {
    pub(crate) fn push(&mut self, span: &Span, parent: Option<OriginId>) -> OriginId {
        let (path, line) = self.physical_location(span.line);
        let id = OriginId(self.origins.len() as u32);
        self.origins.push(Origin {
            path,
            line,
            column: span.column,
            parent,
        });
        id
    }

    pub(crate) fn set_source_origins(&mut self, source_origins: Vec<(PathBuf, usize)>) {
        for origin in &mut self.origins {
            if origin.path.is_some() {
                continue;
            }
            let Some((path, line)) = origin
                .line
                .checked_sub(1)
                .and_then(|index| source_origins.get(index))
            else {
                continue;
            };
            origin.path = Some(path.clone());
            origin.line = *line;
        }
        self.source_origins = source_origins;
    }

    pub(crate) fn get(&self, id: OriginId) -> &Origin {
        &self.origins[id.0 as usize]
    }

    pub(crate) fn try_get(&self, id: OriginId) -> Option<&Origin> {
        self.origins.get(id.0 as usize)
    }

    pub(crate) fn source_origins(&self) -> &[(PathBuf, usize)] {
        &self.source_origins
    }

    pub(crate) fn source_origin(&self, merged_line: usize) -> Option<(&Path, usize)> {
        self.source_origins
            .get(merged_line.checked_sub(1)?)
            .map(|(path, line)| (path.as_path(), *line))
    }

    fn physical_location(&self, merged_line: usize) -> (Option<PathBuf>, usize) {
        self.source_origins
            .get(merged_line.saturating_sub(1))
            .map_or((None, merged_line), |(path, line)| {
                (Some(path.clone()), *line)
            })
    }
}

#[derive(Clone, Debug)]
struct ComponentDeclarations {
    declaration: Declaration<ComponentId>,
    params: Vec<Declaration<ComponentParamId>>,
    slots: Vec<Declaration<ComponentSlotId>>,
    states: Vec<Declaration<ComponentStateId>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SourceSite {
    line: usize,
    column: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct DeclarationIndex {
    app_states: Vec<Declaration<AppStateId>>,
    derived: Vec<Declaration<DerivedId>>,
    components: Vec<ComponentDeclarations>,
    components_by_name: HashMap<String, ComponentId>,
    structs: Vec<StructDeclaration>,
    structs_by_name: HashMap<String, StructId>,
    struct_fields_by_owner: HashMap<StructId, HashMap<String, StructFieldId>>,
    enums: Vec<EnumDeclaration>,
    enums_by_name: HashMap<String, EnumId>,
    enum_variants_by_owner: HashMap<EnumId, HashMap<String, EnumVariantId>>,
    palettes: Vec<Declaration<PaletteId>>,
    palette_names: Vec<String>,
    palettes_by_name: HashMap<String, PaletteId>,
    externs: Vec<ExternDeclaration>,
    externs_by_name: HashMap<String, ExternFnId>,
    views: Vec<Declaration<ViewId>>,
    views_by_site: HashMap<SourceSite, ViewId>,
    component_calls_by_view: HashMap<ViewId, ComponentCallId>,
    handlers: Vec<HandlerDeclaration>,
    handlers_by_owner_name: HashMap<(HandlerOwner, String), HandlerId>,
    statements: Vec<StatementDeclaration>,
    tasks: Vec<TaskDeclaration>,
    routes: Vec<RouteDeclaration>,
    run_sites: Vec<RunSiteDeclaration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum HandlerOwner {
    App,
    Component(ComponentId),
    Preset(u32),
}

#[derive(Clone, Debug)]
pub(crate) struct HandlerDeclaration {
    pub(crate) declaration: Declaration<HandlerId>,
    pub(crate) owner: HandlerOwner,
    pub(crate) name: String,
    pub(crate) statement_roots: Vec<StatementId>,
}

#[derive(Clone, Debug)]
pub(crate) struct StatementDeclaration {
    pub(crate) declaration: Declaration<StatementId>,
    pub(crate) handler: HandlerId,
    pub(crate) parent: Option<StatementId>,
    pub(crate) task: Option<TaskId>,
    pub(crate) source_tasks: Vec<TaskId>,
    pub(crate) routes: Vec<RouteId>,
    pub(crate) run_site: Option<RunSiteId>,
    pub(crate) children: Vec<StatementId>,
    pub(crate) is_final: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct TaskDeclaration {
    pub(crate) declaration: Declaration<TaskId>,
    pub(crate) statement: StatementId,
    pub(crate) parent: Option<TaskId>,
}

#[derive(Clone, Debug)]
pub(crate) struct RouteDeclaration {
    pub(crate) declaration: Declaration<RouteId>,
    pub(crate) statement: StatementId,
    pub(crate) task: Option<TaskId>,
}

#[derive(Clone, Debug)]
pub(crate) struct RunSiteDeclaration {
    pub(crate) declaration: Declaration<RunSiteId>,
    pub(crate) statement: StatementId,
    pub(crate) mode: FutureMode,
}

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
        let components = document
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
                    params,
                    slots,
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
                    rust_name: generated_named_rust(&item.name),
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

        let mut views = Vec::new();
        let mut views_by_site = HashMap::new();
        let mut component_calls_by_view = HashMap::new();
        for (index, component) in document.components.iter().enumerate() {
            index_view_declarations(
                &component.root,
                Some(components[index].declaration.origin),
                origins,
                &mut views,
                &mut views_by_site,
                &mut component_calls_by_view,
            );
        }
        index_view_declarations(
            &document.view,
            None,
            origins,
            &mut views,
            &mut views_by_site,
            &mut component_calls_by_view,
        );
        for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
            index_view_declarations(
                mount,
                None,
                origins,
                &mut views,
                &mut views_by_site,
                &mut component_calls_by_view,
            );
        }

        let mut handlers = Vec::new();
        let mut statements = Vec::new();
        let mut tasks = Vec::new();
        let mut routes = Vec::new();
        let mut run_sites = Vec::new();
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
                &mut run_sites,
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
                    &mut run_sites,
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
                &mut run_sites,
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

        Self {
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
            views,
            views_by_site,
            component_calls_by_view,
            handlers,
            handlers_by_owner_name,
            statements,
            tasks,
            routes,
            run_sites,
        }
    }

    pub(crate) fn app_state(&self, index: usize) -> Declaration<AppStateId> {
        self.app_states[index]
    }

    pub(crate) fn derived(&self, index: usize) -> Declaration<DerivedId> {
        self.derived[index]
    }

    pub(crate) fn component(&self, index: usize) -> Declaration<ComponentId> {
        self.components[index].declaration
    }

    pub(crate) fn component_ids(&self) -> HashMap<String, ComponentId> {
        self.components_by_name.clone()
    }

    pub(crate) fn component_id(&self, name: &str) -> Option<ComponentId> {
        self.components_by_name.get(name).copied()
    }

    pub(crate) fn component_param(
        &self,
        component: ComponentId,
        index: usize,
    ) -> Declaration<ComponentParamId> {
        self.components[component.0 as usize].params[index]
    }

    pub(crate) fn component_state(
        &self,
        component: ComponentId,
        index: usize,
    ) -> Declaration<ComponentStateId> {
        self.components[component.0 as usize].states[index]
    }

    pub(crate) fn component_slot(
        &self,
        component: ComponentId,
        index: usize,
    ) -> Declaration<ComponentSlotId> {
        self.components[component.0 as usize].slots[index]
    }

    pub(crate) fn try_component_slot(
        &self,
        id: ComponentSlotId,
    ) -> Option<Declaration<ComponentSlotId>> {
        self.components
            .get(id.component.0 as usize)?
            .slots
            .get(id.index as usize)
            .copied()
    }

    pub(crate) fn view(&self, id: ViewId) -> Declaration<ViewId> {
        self.views[id.0 as usize]
    }

    pub(crate) fn view_id(&self, span: &Span) -> Option<ViewId> {
        self.views_by_site
            .get(&SourceSite {
                line: span.line,
                column: span.column,
            })
            .copied()
    }

    pub(crate) fn component_call_id(&self, view: ViewId) -> Option<ComponentCallId> {
        self.component_calls_by_view.get(&view).copied()
    }

    pub(crate) fn struct_decl_by_name(&self, name: &str) -> Option<&StructDeclaration> {
        let id = self.structs_by_name.get(name)?;
        self.structs.get(id.0 as usize)
    }

    pub(crate) fn struct_field(
        &self,
        owner: StructId,
        name: &str,
    ) -> Option<&StructFieldDeclaration> {
        let id = self.struct_fields_by_owner.get(&owner)?.get(name)?;
        self.structs
            .get(owner.0 as usize)?
            .fields
            .get(id.index as usize)
    }

    pub(crate) fn try_struct_field_decl(
        &self,
        id: StructFieldId,
    ) -> Option<&StructFieldDeclaration> {
        self.structs
            .get(id.owner.0 as usize)?
            .fields
            .get(id.index as usize)
    }

    pub(crate) fn enum_decl_by_name(&self, name: &str) -> Option<&EnumDeclaration> {
        let id = self.enums_by_name.get(name)?;
        self.enums.get(id.0 as usize)
    }

    pub(crate) fn enum_decl(&self, id: EnumId) -> &EnumDeclaration {
        &self.enums[id.0 as usize]
    }

    pub(crate) fn try_enum_decl(&self, id: EnumId) -> Option<&EnumDeclaration> {
        self.enums.get(id.0 as usize)
    }

    pub(crate) fn enum_variant(
        &self,
        owner: EnumId,
        name: &str,
    ) -> Option<&EnumVariantDeclaration> {
        let id = self.enum_variants_by_owner.get(&owner)?.get(name)?;
        self.enums
            .get(owner.0 as usize)?
            .variants
            .get(id.index as usize)
    }

    pub(crate) fn enum_variant_decl(&self, id: EnumVariantId) -> &EnumVariantDeclaration {
        &self.enums[id.owner.0 as usize].variants[id.index as usize]
    }

    pub(crate) fn try_enum_variant_decl(
        &self,
        id: EnumVariantId,
    ) -> Option<&EnumVariantDeclaration> {
        self.enums
            .get(id.owner.0 as usize)?
            .variants
            .get(id.index as usize)
    }

    pub(crate) fn palette(&self, index: usize) -> Declaration<PaletteId> {
        self.palettes[index]
    }

    pub(crate) fn palette_id(&self, name: &str) -> Option<PaletteId> {
        self.palettes_by_name.get(name).copied()
    }

    pub(crate) fn palette_name(&self, id: PaletteId) -> Option<&str> {
        self.palette_names.get(id.0 as usize).map(String::as_str)
    }

    pub(crate) fn extern_fn(&self, index: usize) -> Declaration<ExternFnId> {
        self.externs[index].declaration
    }

    pub(crate) fn extern_decl_by_name(&self, name: &str) -> Option<&ExternDeclaration> {
        let id = self.externs_by_name.get(name)?;
        self.externs.get(id.0 as usize)
    }

    pub(crate) fn extern_decl(&self, id: ExternFnId) -> &ExternDeclaration {
        &self.externs[id.0 as usize]
    }

    pub(crate) fn try_extern_decl(&self, id: ExternFnId) -> Option<&ExternDeclaration> {
        self.externs.get(id.0 as usize)
    }

    pub(crate) fn handlers(&self) -> &[HandlerDeclaration] {
        &self.handlers
    }

    pub(crate) fn handler(&self, id: HandlerId) -> &HandlerDeclaration {
        &self.handlers[id.0 as usize]
    }

    pub(crate) fn try_handler(&self, id: HandlerId) -> Option<&HandlerDeclaration> {
        self.handlers.get(id.0 as usize)
    }

    pub(crate) fn handler_id(&self, owner: HandlerOwner, name: &str) -> Option<HandlerId> {
        let owner = match owner {
            HandlerOwner::Preset(_) => HandlerOwner::App,
            owner => owner,
        };
        self.handlers_by_owner_name
            .get(&(owner, name.to_owned()))
            .copied()
    }

    pub(crate) fn statement(&self, id: StatementId) -> &StatementDeclaration {
        &self.statements[id.0 as usize]
    }

    pub(crate) fn try_statement(&self, id: StatementId) -> Option<&StatementDeclaration> {
        self.statements.get(id.0 as usize)
    }

    pub(crate) fn task(&self, id: TaskId) -> &TaskDeclaration {
        &self.tasks[id.0 as usize]
    }

    pub(crate) fn try_task(&self, id: TaskId) -> Option<&TaskDeclaration> {
        self.tasks.get(id.0 as usize)
    }

    pub(crate) fn route(&self, id: RouteId) -> &RouteDeclaration {
        &self.routes[id.0 as usize]
    }

    pub(crate) fn try_route(&self, id: RouteId) -> Option<&RouteDeclaration> {
        self.routes.get(id.0 as usize)
    }

    pub(crate) fn try_run_site(&self, id: RunSiteId) -> Option<&RunSiteDeclaration> {
        self.run_sites.get(id.0 as usize)
    }

    pub(crate) fn statement_count(&self) -> usize {
        self.statements.len()
    }

    pub(crate) fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub(crate) fn route_count(&self) -> usize {
        self.routes.len()
    }

    pub(crate) fn run_site_count(&self) -> usize {
        self.run_sites.len()
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
    run_sites: &mut Vec<RunSiteDeclaration>,
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
                None,
                index + 1 == handler.statements.len(),
                origin,
                origins,
                statements,
                tasks,
                routes,
                run_sites,
            )
        })
        .collect();
    handlers.push(HandlerDeclaration {
        declaration: Declaration { id, origin },
        owner,
        name: handler.name.clone(),
        statement_roots,
    });
}

#[allow(clippy::too_many_arguments)]
fn index_statement_declaration(
    statement: &Statement,
    handler: HandlerId,
    parent: Option<StatementId>,
    is_final: bool,
    parent_origin: OriginId,
    origins: &mut OriginArena,
    statements: &mut Vec<StatementDeclaration>,
    tasks: &mut Vec<TaskDeclaration>,
    routes: &mut Vec<RouteDeclaration>,
    run_sites: &mut Vec<RunSiteDeclaration>,
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
        run_site: None,
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

    if let Statement::Run { mode, .. } = statement
        && *mode != FutureMode::Every
    {
        let run_site = RunSiteId(run_sites.len() as u32);
        run_sites.push(RunSiteDeclaration {
            declaration: Declaration {
                id: run_site,
                origin,
            },
            statement: id,
            mode: *mode,
        });
        statements[id.0 as usize].run_site = Some(run_site);
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
                Some(id),
                true,
                origin,
                origins,
                statements,
                tasks,
                routes,
                run_sites,
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
    for child in view_children(node) {
        index_view_declarations(
            child,
            Some(origin),
            origins,
            views,
            views_by_site,
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
            function,
            args,
            success,
            error,
            ..
        } => format!(
            "run:{kind:?}:{mode:?}:{function}:{}:{}:{}",
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HandlerOperationContract {
    Widget(CheckedWidgetOperation),
    Pane {
        grid: String,
        operation: CheckedPaneOperation,
    },
    Window(CheckedWindowOperation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckedWidgetTarget(pub(crate) Vec<(String, bool)>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedWidgetSelector {
    Id(CheckedWidgetTarget),
    Text,
    Point,
    Focused,
    Extern { function: String, arguments: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedWidgetOperation {
    FocusPrevious,
    FocusNext,
    Focus(CheckedWidgetTarget),
    Focused(CheckedWidgetTarget),
    CursorFront(CheckedWidgetTarget),
    CursorEnd(CheckedWidgetTarget),
    Cursor(CheckedWidgetTarget),
    SelectAll(CheckedWidgetTarget),
    Select(CheckedWidgetTarget),
    Snap(CheckedWidgetTarget),
    SnapEnd(CheckedWidgetTarget),
    ScrollTo(CheckedWidgetTarget),
    ScrollBy(CheckedWidgetTarget),
    Find {
        selector: CheckedWidgetSelector,
        all: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedPaneReference {
    Static(String),
    Dynamic(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedPaneEdge {
    Top,
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedPaneOperation {
    Maximize(CheckedPaneReference),
    Restore,
    Maximized,
    Adjacent(CheckedPaneReference, CheckedPaneEdge),
    Swap(CheckedPaneReference, CheckedPaneReference),
    Close(CheckedPaneReference),
    Move(CheckedPaneReference, CheckedPaneEdge),
    Resize(Option<String>),
    Drop(
        CheckedPaneReference,
        CheckedPaneReference,
        Option<CheckedPaneEdge>,
    ),
    Split {
        target: CheckedPaneReference,
        pane: CheckedPaneReference,
        axis: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedWindowOperation {
    Open(Option<String>),
    Oldest,
    Latest,
    Close,
    Drag,
    DragResize(String),
    Resize,
    Resizable,
    MinSize(bool),
    MaxSize(bool),
    ResizeIncrements(bool),
    Size,
    IsMaximized,
    Maximize,
    IsMinimized,
    Minimize,
    Position,
    ScaleFactor,
    Move,
    Mode,
    SetMode(String),
    ToggleMaximize,
    ToggleDecorations,
    Attention(Option<String>),
    Focus,
    SetLevel(String),
    SystemMenu,
    RawId,
    Screenshot,
    MousePassthrough,
    MonitorSize,
    AutomaticTabbing,
    Icon,
    Callback { function: String, arguments: usize },
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
