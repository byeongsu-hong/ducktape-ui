use crate::ast::*;
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
    states: Vec<Declaration<ComponentStateId>>,
}

#[derive(Clone, Debug)]
pub(crate) struct DeclarationIndex {
    app_states: Vec<Declaration<AppStateId>>,
    app_states_by_name: HashMap<String, AppStateId>,
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
    palettes_by_name: HashMap<String, PaletteId>,
    externs: Vec<ExternDeclaration>,
    externs_by_name: HashMap<String, ExternFnId>,
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
        let app_states_by_name = document
            .states
            .iter()
            .zip(&app_states)
            .map(|(state, declaration)| (state.name.clone(), declaration.id))
            .collect();

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

        Self {
            app_states,
            app_states_by_name,
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
            palettes_by_name,
            externs,
            externs_by_name,
        }
    }

    pub(crate) fn app_state(&self, index: usize) -> Declaration<AppStateId> {
        self.app_states[index]
    }

    pub(crate) fn app_state_ids(&self) -> HashMap<String, AppStateId> {
        self.app_states_by_name.clone()
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

    pub(crate) fn enum_decl_by_name(&self, name: &str) -> Option<&EnumDeclaration> {
        let id = self.enums_by_name.get(name)?;
        self.enums.get(id.0 as usize)
    }

    pub(crate) fn enum_decl(&self, id: EnumId) -> &EnumDeclaration {
        &self.enums[id.0 as usize]
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

    pub(crate) fn palette(&self, index: usize) -> Declaration<PaletteId> {
        self.palettes[index]
    }

    pub(crate) fn palette_id(&self, name: &str) -> Option<PaletteId> {
        self.palettes_by_name.get(name).copied()
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
}
