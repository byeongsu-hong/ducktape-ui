use crate::ast::*;
#[cfg(test)]
use std::cell::Cell;
use std::collections::HashMap;
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
arena_id!(SubscriptionId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NamedTypeId {
    Struct(StructId),
    Enum(EnumId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExternRef {
    pub(crate) id: ExternFnId,
    pub(crate) name: String,
}

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
    daemon: bool,
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
    palette_contracts: Vec<String>,
    palettes_by_name: HashMap<String, PaletteId>,
    externs: Vec<ExternDeclaration>,
    externs_by_name: HashMap<String, ExternFnId>,
    handlers: Vec<HandlerDeclaration>,
    handlers_by_name: HashMap<String, HandlerId>,
    subscriptions: Vec<Declaration<SubscriptionId>>,
    views: Vec<Declaration<ViewId>>,
    views_by_site: HashMap<SourceSite, ViewId>,
    component_calls_by_view: HashMap<ViewId, ComponentCallId>,
    #[cfg(test)]
    extern_name_lookups: Cell<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct HandlerDeclaration {
    pub(crate) declaration: Declaration<HandlerId>,
    pub(crate) name: String,
    pub(crate) payloads: Vec<Type>,
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
        let palette_contracts = document
            .palettes
            .iter()
            .map(|palette| palette.contract.clone())
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

        let handlers = document
            .handlers
            .iter()
            .enumerate()
            .map(|(index, handler)| HandlerDeclaration {
                declaration: Declaration {
                    id: HandlerId(index as u32),
                    origin: origins.push(&handler.span, None),
                },
                name: handler.name.clone(),
                payloads: handler
                    .params
                    .iter()
                    .map(|param| param.ty.clone())
                    .collect(),
            })
            .collect::<Vec<_>>();
        let handlers_by_name = handlers
            .iter()
            .map(|handler| (handler.name.clone(), handler.declaration.id))
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

        Self {
            daemon: document.daemon,
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
            palette_contracts,
            palettes_by_name,
            externs,
            externs_by_name,
            handlers,
            handlers_by_name,
            subscriptions,
            views,
            views_by_site,
            component_calls_by_view,
            #[cfg(test)]
            extern_name_lookups: Cell::new(0),
        }
    }

    pub(crate) fn app_state(&self, index: usize) -> Declaration<AppStateId> {
        self.app_states[index]
    }

    pub(crate) fn daemon(&self) -> bool {
        self.daemon
    }

    pub(crate) fn named_type_id(&self, name: &str) -> Option<NamedTypeId> {
        self.structs_by_name
            .get(name)
            .copied()
            .map(NamedTypeId::Struct)
            .or_else(|| self.enums_by_name.get(name).copied().map(NamedTypeId::Enum))
    }

    pub(crate) fn named_type_rust_paths(&self) -> HashMap<NamedTypeId, String> {
        self.structs
            .iter()
            .map(|item| {
                (
                    NamedTypeId::Struct(item.declaration.id),
                    item.rust_path.clone(),
                )
            })
            .chain(self.enums.iter().map(|item| {
                (
                    NamedTypeId::Enum(item.declaration.id),
                    item.rust_name.clone(),
                )
            }))
            .collect()
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

    pub(crate) fn try_struct_decl(&self, id: StructId) -> Option<&StructDeclaration> {
        self.structs.get(id.0 as usize)
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

    pub(crate) fn palette_type(&self, id: PaletteId) -> Option<Type> {
        self.palette_contracts
            .get(id.0 as usize)
            .cloned()
            .map(Type::Palette)
    }

    pub(crate) fn extern_fn(&self, index: usize) -> Declaration<ExternFnId> {
        self.externs[index].declaration
    }

    pub(crate) fn extern_decl_by_name(&self, name: &str) -> Option<&ExternDeclaration> {
        #[cfg(test)]
        self.extern_name_lookups
            .set(self.extern_name_lookups.get() + 1);
        let id = self.externs_by_name.get(name)?;
        self.externs.get(id.0 as usize)
    }

    #[cfg(test)]
    pub(crate) fn extern_name_lookup_count(&self) -> usize {
        self.extern_name_lookups.get()
    }

    pub(crate) fn extern_decl(&self, id: ExternFnId) -> &ExternDeclaration {
        &self.externs[id.0 as usize]
    }

    pub(crate) fn checked_extern_decl(
        &self,
        id: ExternFnId,
        span: &Span,
    ) -> Result<&ExternDeclaration, crate::Error> {
        self.externs.get(id.0 as usize).ok_or_else(|| {
            crate::Error::new(
                "E196",
                span,
                "checked HIR references an invalid extern declaration ID",
            )
        })
    }

    pub(crate) fn handler_id(&self, name: &str) -> Option<HandlerId> {
        self.handlers_by_name.get(name).copied()
    }

    pub(crate) fn checked_handler(
        &self,
        id: HandlerId,
        span: &Span,
    ) -> Result<&HandlerDeclaration, crate::Error> {
        self.handlers.get(id.0 as usize).ok_or_else(|| {
            crate::Error::new(
                "E196",
                span,
                "checked route references an invalid handler declaration ID",
            )
        })
    }

    pub(crate) fn subscription(&self, index: usize) -> Declaration<SubscriptionId> {
        self.subscriptions[index]
    }

    pub(crate) fn try_subscription(&self, index: usize) -> Option<Declaration<SubscriptionId>> {
        self.subscriptions.get(index).copied()
    }

    pub(crate) fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    pub(crate) fn finalize_checked_handlers(
        &mut self,
        document: &Document,
    ) -> Result<(), crate::Error> {
        if self.handlers.len() != document.handlers.len() {
            return Err(crate::Error::new(
                "E196",
                &Span::line(1),
                "checked handler declarations changed during semantic analysis",
            ));
        }
        for (declaration, handler) in self.handlers.iter_mut().zip(&document.handlers) {
            if declaration.name != handler.name {
                return Err(crate::Error::new(
                    "E196",
                    &handler.span,
                    "checked handler identity changed during semantic analysis",
                ));
            }
            declaration.payloads = handler
                .params
                .iter()
                .map(|param| param.ty.clone())
                .collect();
        }
        Ok(())
    }
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
