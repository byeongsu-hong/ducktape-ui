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
    structs_by_name: HashMap<String, StructId>,
    struct_fields_by_owner: HashMap<StructId, HashMap<String, StructFieldId>>,
    enums_by_name: HashMap<String, EnumId>,
    enum_variants_by_owner: HashMap<EnumId, HashMap<String, EnumVariantId>>,
    palettes: Vec<Declaration<PaletteId>>,
    palettes_by_name: HashMap<String, PaletteId>,
    externs: Vec<Declaration<ExternFnId>>,
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

        let structs_by_name = document
            .structs
            .iter()
            .enumerate()
            .map(|(index, item)| (item.name.clone(), StructId(index as u32)))
            .collect();
        let struct_fields_by_owner = document
            .structs
            .iter()
            .enumerate()
            .map(|(struct_index, item)| {
                let owner = StructId(struct_index as u32);
                let fields = item
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, (name, _))| {
                        (
                            name.clone(),
                            StructFieldId {
                                owner,
                                index: index as u32,
                            },
                        )
                    })
                    .collect();
                (owner, fields)
            })
            .collect();

        let enums_by_name = document
            .enums
            .iter()
            .enumerate()
            .map(|(index, item)| (item.name.clone(), EnumId(index as u32)))
            .collect();
        let enum_variants_by_owner = document
            .enums
            .iter()
            .enumerate()
            .map(|(enum_index, item)| {
                let owner = EnumId(enum_index as u32);
                let variants = item
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(index, variant)| {
                        (
                            variant.name.clone(),
                            EnumVariantId {
                                owner,
                                index: index as u32,
                            },
                        )
                    })
                    .collect();
                (owner, variants)
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
            .map(|(index, function)| Declaration {
                id: ExternFnId(index as u32),
                origin: origins.push(&function.span, None),
            })
            .collect::<Vec<_>>();
        let externs_by_name = document
            .functions
            .iter()
            .zip(&externs)
            .map(|(function, declaration)| (function.name.clone(), declaration.id))
            .collect();

        Self {
            app_states,
            app_states_by_name,
            derived,
            components,
            components_by_name,
            structs_by_name,
            struct_fields_by_owner,
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

    pub(crate) fn struct_id(&self, name: &str) -> Option<StructId> {
        self.structs_by_name.get(name).copied()
    }

    pub(crate) fn struct_field(&self, owner: StructId, name: &str) -> Option<StructFieldId> {
        self.struct_fields_by_owner.get(&owner)?.get(name).copied()
    }

    pub(crate) fn enum_id(&self, name: &str) -> Option<EnumId> {
        self.enums_by_name.get(name).copied()
    }

    pub(crate) fn enum_variant(&self, owner: EnumId, name: &str) -> Option<EnumVariantId> {
        self.enum_variants_by_owner.get(&owner)?.get(name).copied()
    }

    pub(crate) fn palette(&self, index: usize) -> Declaration<PaletteId> {
        self.palettes[index]
    }

    pub(crate) fn palette_id(&self, name: &str) -> Option<PaletteId> {
        self.palettes_by_name.get(name).copied()
    }

    pub(crate) fn extern_fn(&self, index: usize) -> Declaration<ExternFnId> {
        self.externs[index]
    }

    pub(crate) fn extern_fn_id(&self, name: &str) -> Option<ExternFnId> {
        self.externs_by_name.get(name).copied()
    }
}
