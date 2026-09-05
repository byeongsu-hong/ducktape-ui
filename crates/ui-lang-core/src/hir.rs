use crate::semantic::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

mod from_ast;
pub use from_ast::for_each_child;
pub(crate) use from_ast::{
    dynamic_pane_grids, handler_operation_contract, statement_semantic_key, view_children,
    view_kind,
};

/// A builtin whose result is not a function of its arguments: it reads the
/// clock, mints an identity, or allocates a handle, so a cached evaluation
/// goes stale without any state write. The checker keeps it out of derived
/// values and launch-time snapshots; codegen keeps a `lazy` that calls it
/// on the value-hashed lowering.
pub(crate) fn recomputation_unsafe_builtin(builtin: &str, arity: usize) -> bool {
    let implicit_animation_clock = matches!(
        (builtin, arity),
        ("animation.animating" | "animation.remaining", 1)
            | ("animation.interpolate" | "animation.project", 3)
    );
    matches!(
        builtin,
        "window_id.unique" | "aborted" | "debug.time_with" | "image.upgrade" | "encoded" | "rgba"
    ) || implicit_animation_clock
}

pub(crate) fn canonical_rust_type_name(name: &str) -> String {
    if name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        name.to_owned()
    } else {
        format!(
            "__IceType0{}",
            name.bytes()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    }
}

#[cfg(test)]
#[derive(Debug, Default)]
struct TestLookupCount(AtomicUsize);

#[cfg(test)]
impl Clone for TestLookupCount {
    fn clone(&self) -> Self {
        Self(AtomicUsize::new(self.0.load(Ordering::Relaxed)))
    }
}

macro_rules! arena_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) struct $name(pub(crate) u32);
    };
}

arena_id!(ComponentId);
arena_id!(AppStateId);
arena_id!(DerivedId);
arena_id!(SecretId);
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
arena_id!(RunLaneId);
arena_id!(NamedWindowId);
arena_id!(SubscriptionId);
arena_id!(ExpressionNodeId);
arena_id!(ExpressionId);
arena_id!(LocalId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ValueRef {
    AppState(AppStateId),
    Derived(DerivedId),
    /// A runtime-held secret buffer. It is a *value reference* only so a name
    /// can resolve to it; nothing downstream may read it as data.
    Secret(SecretId),
    ComponentParam(ComponentParamId),
    ComponentState(ComponentStateId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TestTargetId {
    pub(crate) test: TestId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TestStepId {
    pub(crate) test: TestId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CanvasLocalId {
    pub(crate) canvas: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CanvasCommandId {
    pub(crate) canvas: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CanvasEventId {
    pub(crate) canvas: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CanvasRouteId {
    pub(crate) canvas: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CanvasExpressionId {
    pub(crate) canvas: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MediaExpressionId {
    pub(crate) media: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TooltipExpressionId {
    pub(crate) tooltip: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FloatExpressionId {
    pub(crate) float: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PinExpressionId {
    pub(crate) pin: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InteractionExpressionId {
    pub(crate) widget: ViewId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InteractionRouteId {
    pub(crate) widget: ViewId,
    pub(crate) index: u32,
}

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
pub(crate) enum AppSettingExprId {
    Title,
    Theme,
    ThemeFactoryArgument(u32),
    Palette,
    Background,
    TextColor,
    ScaleFactor,
    TrayLabel,
    TrayTooltip,
    /// The `when` guard on the `icon-rgba` line at this declaration index.
    TrayIconGuard(u32),
    /// The text of the `menu` row at this declaration index. Separators are
    /// counted, so the index is the row the author wrote.
    TrayMenuRow(u32),
    /// The `when` guard on the `menu` row at this declaration index.
    TrayRowGuard(u32),
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
    output: Type,
    params: Vec<Declaration<ComponentParamId>>,
    events: Vec<ComponentEventDeclaration>,
    slots: Vec<Declaration<ComponentSlotId>>,
    slot_views: Vec<ViewId>,
    states: Vec<Declaration<ComponentStateId>>,
}

#[derive(Clone, Debug)]
pub(crate) struct ComponentEventDeclaration {
    pub(crate) declaration: Declaration<ComponentEventId>,
    pub(crate) name: String,
    pub(crate) payloads: Vec<Type>,
}

#[derive(Clone, Debug)]
pub(crate) struct TestDeclaration {
    pub(crate) declaration: Declaration<TestId>,
    pub(crate) name: String,
    pub(crate) semantic_key: String,
    pub(crate) targets: Vec<TestTargetDeclaration>,
    pub(crate) steps: Vec<TestStepDeclaration>,
}

#[derive(Clone, Debug)]
pub(crate) struct TestTargetDeclaration {
    pub(crate) declaration: Declaration<TestTargetId>,
    pub(crate) name: String,
    pub(crate) segments: Vec<(String, bool)>,
}

#[derive(Clone, Debug)]
pub(crate) struct TestStepDeclaration {
    pub(crate) declaration: Declaration<TestStepId>,
    pub(crate) semantic_key: String,
    pub(crate) source: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CanvasDeclaration {
    pub(crate) declaration: Declaration<ViewId>,
    pub(crate) options_semantic_key: String,
    pub(crate) locals: Vec<CanvasLocalDeclaration>,
    pub(crate) commands: Vec<CanvasCommandDeclaration>,
    pub(crate) events: Vec<CanvasEventDeclaration>,
    pub(crate) routes: Vec<Declaration<CanvasRouteId>>,
}

#[derive(Clone, Debug)]
pub(crate) struct CanvasCommandDeclaration {
    pub(crate) declaration: Declaration<CanvasCommandId>,
    pub(crate) semantic_key: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CanvasEventDeclaration {
    pub(crate) declaration: Declaration<CanvasEventId>,
    pub(crate) semantic_key: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CanvasLocalDeclaration {
    pub(crate) declaration: Declaration<CanvasLocalId>,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SourceSite {
    line: usize,
    column: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct DeclarationIndex {
    app_settings: Declaration<AppSettingsId>,
    app_setting_expressions: HashMap<AppSettingExprId, Declaration<AppSettingExprId>>,
    app_states: Vec<Declaration<AppStateId>>,
    derived: Vec<Declaration<DerivedId>>,
    secrets: Vec<Declaration<SecretId>>,
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
    #[cfg(test)]
    extern_name_lookups: TestLookupCount,
    views: Vec<Declaration<ViewId>>,
    views_by_site: HashMap<SourceSite, ViewId>,
    canvases: HashMap<ViewId, CanvasDeclaration>,
    component_calls_by_view: HashMap<ViewId, ComponentCallId>,
    handlers: Vec<HandlerDeclaration>,
    handlers_by_owner_name: HashMap<(HandlerOwner, String), HandlerId>,
    subscriptions: Vec<Declaration<SubscriptionId>>,
    tests: Vec<TestDeclaration>,
    statements: Vec<StatementDeclaration>,
    tasks: Vec<TaskDeclaration>,
    routes: Vec<RouteDeclaration>,
    run_lanes: Vec<RunLaneDeclaration>,
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
    pub(crate) payloads: Vec<Type>,
}

#[derive(Clone, Debug)]
pub(crate) struct StatementDeclaration {
    pub(crate) declaration: Declaration<StatementId>,
    pub(crate) handler: HandlerId,
    pub(crate) parent: Option<StatementId>,
    pub(crate) task: Option<TaskId>,
    pub(crate) source_tasks: Vec<TaskId>,
    pub(crate) routes: Vec<RouteId>,
    pub(crate) run_lane: Option<RunLaneId>,
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
pub(crate) struct RunLaneDeclaration {
    pub(crate) declaration: Declaration<RunLaneId>,
    pub(crate) owner: HandlerOwner,
    pub(crate) name: String,
    pub(crate) kind: EffectKind,
    pub(crate) mode: DeliveryMode,
    pub(crate) statements: Vec<StatementId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AppSettingsId;

impl DeclarationIndex {
    pub(crate) fn app_settings(&self) -> Declaration<AppSettingsId> {
        self.app_settings
    }

    pub(crate) fn test(&self, index: usize) -> &TestDeclaration {
        &self.tests[index]
    }

    pub(crate) fn try_test(&self, id: TestId) -> Option<&TestDeclaration> {
        self.tests
            .get(id.0 as usize)
            .filter(|declaration| declaration.declaration.id == id)
    }

    pub(crate) fn test_count(&self) -> usize {
        self.tests.len()
    }

    pub(crate) fn test_target(&self, id: TestTargetId) -> Option<&TestTargetDeclaration> {
        self.try_test(id.test)?
            .targets
            .get(id.index as usize)
            .filter(|declaration| declaration.declaration.id == id)
    }

    pub(crate) fn test_step(&self, id: TestStepId) -> Option<&TestStepDeclaration> {
        self.try_test(id.test)?
            .steps
            .get(id.index as usize)
            .filter(|declaration| declaration.declaration.id == id)
    }

    pub(crate) fn app_setting_expression(
        &self,
        id: AppSettingExprId,
    ) -> Option<Declaration<AppSettingExprId>> {
        self.app_setting_expressions.get(&id).copied()
    }

    pub(crate) fn app_state(&self, index: usize) -> Declaration<AppStateId> {
        self.app_states[index]
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

    pub(crate) fn enum_declarations(&self) -> &[EnumDeclaration] {
        &self.enums
    }

    pub(crate) fn derived(&self, index: usize) -> Declaration<DerivedId> {
        self.derived[index]
    }

    pub(crate) fn secret(&self, index: usize) -> Declaration<SecretId> {
        self.secrets[index]
    }

    pub(crate) fn try_derived(&self, id: DerivedId) -> Option<Declaration<DerivedId>> {
        self.derived
            .get(id.0 as usize)
            .copied()
            .filter(|declaration| declaration.id == id)
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

    pub(crate) fn component_output(&self, id: ComponentId) -> Option<&Type> {
        self.components.get(id.0 as usize).map(|item| &item.output)
    }

    pub(crate) fn component_param(
        &self,
        component: ComponentId,
        index: usize,
    ) -> Declaration<ComponentParamId> {
        self.components[component.0 as usize].params[index]
    }

    pub(crate) fn try_component_param(
        &self,
        id: ComponentParamId,
    ) -> Option<Declaration<ComponentParamId>> {
        self.components
            .get(id.component.0 as usize)?
            .params
            .get(id.index as usize)
            .copied()
            .filter(|declaration| declaration.id == id)
    }

    pub(crate) fn component_event(
        &self,
        id: ComponentEventId,
    ) -> Option<&ComponentEventDeclaration> {
        self.components
            .get(id.component.0 as usize)?
            .events
            .get(id.index as usize)
            .filter(|declaration| declaration.declaration.id == id)
    }

    pub(crate) fn component_event_by_name(
        &self,
        component: ComponentId,
        name: &str,
    ) -> Option<&ComponentEventDeclaration> {
        self.components
            .get(component.0 as usize)?
            .events
            .iter()
            .find(|event| event.name == name)
    }

    pub(crate) fn component_state(
        &self,
        component: ComponentId,
        index: usize,
    ) -> Declaration<ComponentStateId> {
        self.components[component.0 as usize].states[index]
    }

    pub(crate) fn try_component_state(
        &self,
        id: ComponentStateId,
    ) -> Option<Declaration<ComponentStateId>> {
        self.components
            .get(id.component.0 as usize)?
            .states
            .get(id.index as usize)
            .copied()
            .filter(|declaration| declaration.id == id)
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
            .filter(|declaration| declaration.id == id)
    }

    pub(crate) fn component_slot_count(&self, component: ComponentId) -> Option<usize> {
        self.components
            .get(component.0 as usize)
            .filter(|declarations| declarations.declaration.id == component)
            .map(|declarations| declarations.slots.len())
    }

    pub(crate) fn component_slot_view(&self, id: ComponentSlotId) -> Option<ViewId> {
        let declarations = self.components.get(id.component.0 as usize)?;
        declarations
            .slots
            .get(id.index as usize)
            .filter(|declaration| declaration.id == id)?;
        declarations.slot_views.get(id.index as usize).copied()
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

    pub(crate) fn canvas(&self, id: ViewId) -> Option<&CanvasDeclaration> {
        self.canvases
            .get(&id)
            .filter(|declaration| declaration.declaration.id == id)
    }

    pub(crate) fn canvas_event(&self, id: CanvasEventId) -> Option<Declaration<CanvasEventId>> {
        self.canvas(id.canvas)?
            .events
            .get(id.index as usize)
            .map(|declaration| declaration.declaration)
            .filter(|declaration| declaration.id == id)
    }

    pub(crate) fn canvas_route(&self, id: CanvasRouteId) -> Option<Declaration<CanvasRouteId>> {
        self.canvas(id.canvas)?
            .routes
            .get(id.index as usize)
            .copied()
            .filter(|declaration| declaration.id == id)
    }

    pub(crate) fn component_call_id(&self, view: ViewId) -> Option<ComponentCallId> {
        self.component_calls_by_view.get(&view).copied()
    }

    pub(crate) fn struct_decl_by_name(&self, name: &str) -> Option<&StructDeclaration> {
        let id = self.structs_by_name.get(name)?;
        self.structs.get(id.0 as usize)
    }

    pub(crate) fn struct_declarations(&self) -> &[StructDeclaration] {
        &self.structs
    }

    pub(crate) fn try_struct_decl(&self, id: StructId) -> Option<&StructDeclaration> {
        self.structs
            .get(id.0 as usize)
            .filter(|declaration| declaration.declaration.id == id)
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
            .filter(|field| field.declaration.id == id)
    }

    pub(crate) fn enum_decl_by_name(&self, name: &str) -> Option<&EnumDeclaration> {
        let id = self.enums_by_name.get(name)?;
        self.enums.get(id.0 as usize)
    }

    pub(crate) fn enum_decl(&self, id: EnumId) -> &EnumDeclaration {
        &self.enums[id.0 as usize]
    }

    pub(crate) fn try_enum_decl(&self, id: EnumId) -> Option<&EnumDeclaration> {
        self.enums
            .get(id.0 as usize)
            .filter(|declaration| declaration.declaration.id == id)
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
            .filter(|variant| variant.declaration.id == id)
    }

    pub(crate) fn palette(&self, index: usize) -> Declaration<PaletteId> {
        self.palettes[index]
    }

    pub(crate) fn palette_id(&self, name: &str) -> Option<PaletteId> {
        self.palettes_by_name.get(name).copied()
    }

    pub(crate) fn palette_name(&self, id: PaletteId) -> Option<&str> {
        self.palettes
            .get(id.0 as usize)
            .filter(|declaration| declaration.id == id)?;
        self.palette_names.get(id.0 as usize).map(String::as_str)
    }

    pub(crate) fn palette_count(&self) -> usize {
        self.palettes.len()
    }

    pub(crate) fn extern_decl_by_name(&self, name: &str) -> Option<&ExternDeclaration> {
        #[cfg(test)]
        self.extern_name_lookups.0.fetch_add(1, Ordering::Relaxed);
        let id = self.externs_by_name.get(name)?;
        self.externs.get(id.0 as usize)
    }

    #[cfg(test)]
    pub(crate) fn extern_name_lookup_count(&self) -> usize {
        self.extern_name_lookups.0.load(Ordering::Relaxed)
    }

    pub(crate) fn extern_decl(&self, id: ExternFnId) -> &ExternDeclaration {
        &self.externs[id.0 as usize]
    }

    pub(crate) fn extern_declarations(&self) -> impl Iterator<Item = &ExternDeclaration> {
        self.externs.iter()
    }

    #[cfg(test)]
    pub(crate) fn replace_extern_param_type_for_test(
        &mut self,
        id: ExternFnId,
        index: usize,
        ty: Type,
    ) {
        self.externs[id.0 as usize].params[index].1 = ty;
    }

    pub(crate) fn try_extern_decl(&self, id: ExternFnId) -> Option<&ExternDeclaration> {
        self.externs
            .get(id.0 as usize)
            .filter(|declaration| declaration.declaration.id == id)
    }
    pub(crate) fn handlers(&self) -> &[HandlerDeclaration] {
        &self.handlers
    }

    pub(crate) fn handler(&self, id: HandlerId) -> &HandlerDeclaration {
        &self.handlers[id.0 as usize]
    }

    pub(crate) fn try_handler(&self, id: HandlerId) -> Option<&HandlerDeclaration> {
        self.handlers
            .get(id.0 as usize)
            .filter(|handler| handler.declaration.id == id)
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

    pub(crate) fn try_run_lane(&self, id: RunLaneId) -> Option<&RunLaneDeclaration> {
        self.run_lanes.get(id.0 as usize)
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

    pub(crate) fn run_lane_count(&self) -> usize {
        self.run_lanes.len()
    }

    pub(crate) fn checked_extern_decl(
        &self,
        id: ExternFnId,
        span: &Span,
    ) -> Result<&ExternDeclaration, crate::Error> {
        self.try_extern_decl(id).ok_or_else(|| {
            crate::Error::new(
                "E196",
                span,
                "checked HIR references an invalid extern declaration ID",
            )
        })
    }

    pub(crate) fn checked_handler(
        &self,
        id: HandlerId,
        span: &Span,
    ) -> Result<&HandlerDeclaration, crate::Error> {
        self.try_handler(id).ok_or_else(|| {
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
    ScrollToKey(CheckedWidgetTarget),
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
