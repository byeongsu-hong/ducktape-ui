use super::expr::{
    BuiltinArgumentContext, ContextualBuiltin, ExprTypeAnalysis, field_type, resolve_erased_type,
    unify_type_evidence,
};
use super::*;
use crate::hir::{
    AppSettingExprId, AppStateId, ComponentCallId, ComponentId, ComponentParamId, ComponentSlotId,
    ComponentStateId, DeclarationIndex, DerivedId, EnumVariantId, ExternFnId, HandlerId,
    OriginArena, OriginId, PaletteId, RouteId, StatementId, StructFieldId, TaskId, TestId, ViewId,
};
use crate::unqualified_name;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
#[derive(Debug, Default)]
struct LookupCount(AtomicUsize);

#[cfg(test)]
impl Clone for LookupCount {
    fn clone(&self) -> Self {
        Self(AtomicUsize::new(self.0.load(Ordering::Relaxed)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedExprId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedExprUseId(u32);

#[cfg(test)]
impl CheckedExprUseId {
    pub(crate) fn invalid_for_test() -> Self {
        Self(u32::MAX)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedValueId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedLocalId(u32);

#[cfg(test)]
impl CheckedLocalId {
    pub(crate) fn invalid_for_test() -> Self {
        Self(u32::MAX)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedBuiltinId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ValueScope {
    App,
    Component(ComponentId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedValueRef {
    AppState(AppStateId),
    Derived(DerivedId),
    ComponentParam(ComponentParamId),
    ComponentState(ComponentStateId),
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedValue {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) id: CheckedValueRef,
    pub(crate) initializer: Option<CheckedExprUseId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedLocal {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) owner: CheckedLocalOwner,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedLocalOwner {
    ExpressionBinding {
        expression: CheckedExprUseId,
        body_argument: usize,
    },
    View {
        view: ViewId,
        role: CheckedViewLocalRole,
    },
    HandlerParam {
        handler: HandlerId,
        index: u32,
    },
    StatementLet(StatementId),
    TaskTransform {
        task: TaskId,
        index: u32,
    },
    AppSettingDaemonWindow {
        setting: AppSettingExprId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedViewLocalRole {
    DaemonWindow,
    ForItem,
    MatchPayload(u32),
    KeyedItem,
    LazyDependency,
    TableRow,
    PaneMaximized(u32),
    PaneTemplateItem(u32),
    PaneTemplateMaximized(u32),
    ResponsiveWidth,
    ResponsiveHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedViewScope {
    App,
    Component(ComponentId),
    Test(TestId),
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedView {
    pub(crate) id: ViewId,
    pub(crate) kind: &'static str,
    pub(crate) scope: CheckedViewScope,
    pub(crate) parent: Option<ViewId>,
    pub(crate) children: Vec<ViewId>,
    pub(crate) flow: CheckedViewFlow,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, Default)]
pub(crate) enum CheckedViewFlow {
    #[default]
    None,
    If {
        condition: CheckedExprUseId,
    },
    For {
        items: CheckedExprUseId,
        item: CheckedLocalId,
    },
    Match {
        value: CheckedExprUseId,
        arms: Vec<CheckedMatchArm>,
    },
    Keyed {
        items: CheckedExprUseId,
        key: CheckedExprUseId,
        item: CheckedLocalId,
    },
    Lazy {
        dependency: CheckedExprUseId,
        binding: CheckedLocalId,
    },
    Table {
        rows: CheckedExprUseId,
        item: CheckedLocalId,
    },
    PaneGrid {
        static_maximized: Vec<Option<CheckedLocalId>>,
        templates: Vec<CheckedPaneTemplate>,
    },
    ResponsiveBreakpoint {
        breakpoint: CheckedExprUseId,
    },
    ResponsiveSize {
        width: CheckedLocalId,
        height: CheckedLocalId,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneTemplate {
    pub(crate) key: CheckedExprUseId,
    pub(crate) item: CheckedLocalId,
    pub(crate) maximized: Option<CheckedLocalId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedMatchArm {
    pub(crate) pattern: CheckedMatchPattern,
    pub(crate) binding: Option<CheckedLocalId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedMatchPattern {
    Some,
    None,
    Ok,
    Err,
    Enum(EnumVariantId),
    Palette(PaletteId),
    Wildcard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedExprOwner {
    Value(CheckedValueRef),
    AppSetting(AppSettingExprId),
    ComponentArgument {
        call: ComponentCallId,
        param: ComponentParamId,
    },
    View {
        view: ViewId,
        role: CheckedViewExprRole,
    },
    HandlerStatement {
        statement: StatementId,
        operand: u32,
    },
    Task {
        task: TaskId,
        operand: u32,
    },
    Route {
        route: RouteId,
        argument: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedComponentArgumentSource {
    Supplied(CheckedExprUseId),
    Default(CheckedExprUseId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedViewExprRole {
    IfCondition,
    ForItems,
    MatchValue,
    KeyedItems,
    KeyedKey,
    LazyDependency,
    TableRows,
    PaneTemplateKey(u32),
    ResponsiveBreakpoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedInitializerCoercion {
    None,
    ListToCombo { element: Type },
    ValueToAnimation { value: Type },
    StrToMarkdown,
    StrToEditor,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedExprUse {
    pub(crate) owner: CheckedExprOwner,
    pub(crate) root: CheckedExprId,
    pub(crate) source: Type,
    pub(crate) destination: Type,
    pub(crate) coercion: CheckedInitializerCoercion,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedPathRoot {
    Value(CheckedValueRef),
    Local(CheckedLocalId),
    EnumVariant(EnumVariantId),
    Palette(PaletteId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedProjectionKind {
    Struct(StructFieldId),
    Native,
    OptionalWidgetTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckedProjection {
    pub(crate) field: String,
    pub(crate) input: Type,
    pub(crate) output: Type,
    pub(crate) kind: CheckedProjectionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedCallTarget {
    Builtin(CheckedBuiltinId),
    Extern(ExternFnId),
    EnumVariant(EnumVariantId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedCallArgument {
    Value(CheckedExprId),
    Binding(CheckedLocalId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedUnaryOperator {
    BooleanNot,
    NumericNegation(Type),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedBinaryOperator {
    Boolean(BinaryOp),
    Equality { op: BinaryOp, operand: Type },
    Ordering { op: BinaryOp, operand: Type },
    Arithmetic { op: BinaryOp, operand: Type },
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedExprKind {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<CheckedExprId>),
    None,
    SlotProvided(ComponentSlotId),
    Path {
        root: CheckedPathRoot,
        projections: Vec<CheckedProjection>,
    },
    Call {
        target: CheckedCallTarget,
        arguments: Vec<CheckedCallArgument>,
    },
    Unary {
        operator: CheckedUnaryOperator,
        value: CheckedExprId,
    },
    Binary {
        operator: CheckedBinaryOperator,
        left: CheckedExprId,
        right: CheckedExprId,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedExpr {
    pub(crate) ty: Type,
    pub(crate) kind: CheckedExprKind,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedHandler {
    pub(crate) id: HandlerId,
    pub(crate) params: Vec<CheckedLocalId>,
    pub(crate) param_names: Vec<String>,
    pub(crate) param_types: Vec<Type>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedStatement {
    pub(crate) id: StatementId,
    pub(crate) semantic_key: String,
    pub(crate) operation: Option<crate::hir::HandlerOperationContract>,
    pub(crate) writable_targets: Vec<CheckedValueRef>,
    pub(crate) editor_self_move: Option<bool>,
    pub(crate) pane_grid_dynamic: Option<bool>,
    pub(crate) operand_count: u32,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedRouteArgKind {
    Expression,
    Payload,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedRoute {
    pub(crate) id: RouteId,
    pub(crate) target: HandlerId,
    pub(crate) target_owner: crate::hir::HandlerOwner,
    pub(crate) args: Vec<CheckedRouteArgKind>,
    pub(crate) source_payloads: Vec<Type>,
    pub(crate) ordered_payloads: bool,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedTask {
    pub(crate) id: TaskId,
    pub(crate) output: Option<Type>,
    pub(crate) error: Option<Type>,
    pub(crate) target: Option<CheckedEffectTarget>,
    pub(crate) is_final: bool,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedEffectTarget {
    Builtin(String),
    Extern(ExternFnId),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CheckedFactMetrics {
    pub(crate) values: usize,
    pub(crate) locals: usize,
    pub(crate) views: usize,
    pub(crate) expression_uses: usize,
    pub(crate) expressions: usize,
    pub(crate) type_analysis_queries: usize,
    pub(crate) type_analysis_nodes: usize,
    pub(crate) type_analysis_cache_hits: usize,
    pub(crate) initializer_analysis_passes: usize,
    pub(crate) app_setting_analysis_passes: usize,
    pub(crate) view_analysis_passes: usize,
    /// Final checker analyses consumed directly while constructing handler HIR.
    pub(crate) handler_authoritative_analyses: usize,
    /// Final checker queries whose roots are nested inside an authoritative HIR operand.
    pub(crate) handler_auxiliary_analyses: usize,
    pub(crate) type_scope_env_overlays: usize,
    pub(crate) type_scope_env_full_clones: usize,
    pub(crate) declaration_lookups: usize,
    pub(crate) builtin_intern_lookups: usize,
    pub(crate) scope_env_builds: usize,
    pub(crate) scope_env_entries: usize,
    pub(crate) scope_env_overlays: usize,
    pub(crate) scope_env_full_clones: usize,
    pub(crate) view_scope_env_overlays: usize,
    pub(crate) view_scope_env_full_clones: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CheckedFacts {
    values: Vec<CheckedValue>,
    values_by_ref: HashMap<CheckedValueRef, CheckedValueId>,
    locals: Vec<CheckedLocal>,
    locals_by_owner: HashMap<CheckedLocalOwner, CheckedLocalId>,
    views: Vec<CheckedView>,
    expression_uses: Vec<CheckedExprUse>,
    expression_uses_by_owner: HashMap<CheckedExprOwner, CheckedExprUseId>,
    component_argument_sources:
        HashMap<(ComponentCallId, ComponentParamId), CheckedComponentArgumentSource>,
    expressions: Vec<CheckedExpr>,
    handlers: Vec<CheckedHandler>,
    statements: Vec<Option<CheckedStatement>>,
    tasks: Vec<Option<CheckedTask>>,
    routes: Vec<Option<CheckedRoute>>,
    builtins: Vec<String>,
    app_theme_factory: Option<CheckedAppThemeFactory>,
    app_settings: Option<CheckedAppSettings>,
    metrics: CheckedFactMetrics,
    #[cfg(test)]
    lookup_count: LookupCount,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedAppThemeFactory {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedAppSettings {
    pub(crate) app_name: String,
    pub(crate) daemon: bool,
    pub(crate) source: AppSettings,
    pub(crate) has_default_font: bool,
}

impl CheckedFacts {
    #[cfg(test)]
    pub(crate) fn corrupt_handler_param_local(
        &mut self,
        handler: HandlerId,
        index: usize,
        raw: u32,
    ) {
        self.handlers[handler.0 as usize].params[index] = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_expression_use_root(&mut self, owner: CheckedExprOwner, raw: u32) {
        let id = self.expression_uses_by_owner[&owner];
        self.expression_uses[id.0 as usize].root = CheckedExprId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_expression_first_child(&mut self, owner: CheckedExprOwner, raw: u32) {
        let id = self.expression_uses_by_owner[&owner];
        let root = self.expression_uses[id.0 as usize].root;
        let invalid = CheckedExprId(raw);
        match &mut self.expressions[root.0 as usize].kind {
            CheckedExprKind::List(values) => values[0] = invalid,
            CheckedExprKind::Call { arguments, .. } => {
                let CheckedCallArgument::Value(value) = &mut arguments[0] else {
                    panic!("test expression argument must be a value");
                };
                *value = invalid;
            }
            CheckedExprKind::Unary { value, .. } => *value = invalid,
            CheckedExprKind::Binary { left, .. } => *left = invalid,
            _ => panic!("test expression root must contain a child"),
        }
    }

    #[cfg(test)]
    pub(crate) fn corrupt_expression_first_child_to_root(&mut self, owner: CheckedExprOwner) {
        let id = self.expression_uses_by_owner[&owner];
        let root = self.expression_uses[id.0 as usize].root;
        self.corrupt_expression_first_child(owner, root.0);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_task_extern_target(&mut self, task: TaskId, raw: u32) {
        self.tasks[task.0 as usize]
            .as_mut()
            .expect("test task exists")
            .target = Some(CheckedEffectTarget::Extern(ExternFnId(raw)));
    }

    pub(crate) fn values(&self) -> &[CheckedValue] {
        &self.values
    }

    pub(crate) fn value(&self, id: CheckedValueId) -> &CheckedValue {
        self.record_lookup();
        &self.values[id.0 as usize]
    }

    pub(crate) fn value_by_ref(&self, value_ref: CheckedValueRef) -> &CheckedValue {
        self.value(self.values_by_ref[&value_ref])
    }

    pub(crate) fn try_value_by_ref(&self, value_ref: CheckedValueRef) -> Option<&CheckedValue> {
        self.record_lookup();
        let id = self.values_by_ref.get(&value_ref)?;
        self.values.get(id.0 as usize)
    }

    pub(crate) fn locals(&self) -> &[CheckedLocal] {
        &self.locals
    }

    pub(crate) fn local(&self, id: CheckedLocalId) -> &CheckedLocal {
        self.record_lookup();
        &self.locals[id.0 as usize]
    }

    pub(crate) fn try_local(&self, id: CheckedLocalId) -> Option<&CheckedLocal> {
        self.record_lookup();
        self.locals.get(id.0 as usize)
    }

    pub(crate) fn daemon_window_local(&self) -> Option<CheckedLocalId> {
        self.locals
            .iter()
            .position(|local| {
                matches!(
                    local.owner,
                    CheckedLocalOwner::View {
                        role: CheckedViewLocalRole::DaemonWindow,
                        ..
                    }
                )
            })
            .map(|index| CheckedLocalId(index as u32))
    }

    pub(crate) fn local_by_owner(&self, owner: CheckedLocalOwner) -> Option<CheckedLocalId> {
        self.locals_by_owner.get(&owner).copied()
    }

    pub(crate) fn app_setting_daemon_window_local(&self) -> Option<CheckedLocalId> {
        self.locals
            .iter()
            .position(|local| {
                matches!(
                    local.owner,
                    CheckedLocalOwner::AppSettingDaemonWindow { .. }
                )
            })
            .map(|index| CheckedLocalId(index as u32))
    }

    pub(crate) fn views(&self) -> &[CheckedView] {
        &self.views
    }

    pub(crate) fn view(&self, id: ViewId) -> &CheckedView {
        self.record_lookup();
        &self.views[id.0 as usize]
    }

    pub(crate) fn expression_use(&self, id: CheckedExprUseId) -> &CheckedExprUse {
        self.record_lookup();
        &self.expression_uses[id.0 as usize]
    }

    pub(crate) fn try_expression_use(&self, id: CheckedExprUseId) -> Option<&CheckedExprUse> {
        self.record_lookup();
        self.expression_uses.get(id.0 as usize)
    }

    pub(crate) fn expression_use_by_owner(
        &self,
        owner: CheckedExprOwner,
    ) -> Option<CheckedExprUseId> {
        self.expression_uses_by_owner.get(&owner).copied()
    }

    pub(crate) fn component_argument_source(
        &self,
        call: ComponentCallId,
        param: ComponentParamId,
    ) -> Option<CheckedComponentArgumentSource> {
        self.component_argument_sources.get(&(call, param)).copied()
    }

    pub(crate) fn expression_uses(&self) -> &[CheckedExprUse] {
        &self.expression_uses
    }

    pub(crate) fn app_theme_factory(&self) -> Option<&CheckedAppThemeFactory> {
        self.app_theme_factory.as_ref()
    }

    pub(crate) fn app_settings(&self) -> Option<&CheckedAppSettings> {
        self.app_settings.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn remove_app_settings(&mut self) {
        self.app_settings = None;
    }

    #[cfg(test)]
    pub(crate) fn remove_app_setting_expression(&mut self, id: AppSettingExprId) {
        self.expression_uses_by_owner
            .remove(&CheckedExprOwner::AppSetting(id));
    }

    #[cfg(test)]
    pub(crate) fn corrupt_app_theme_factory_id(&mut self) {
        if let Some(factory) = &mut self.app_theme_factory {
            factory.function = ExternFnId(u32::MAX);
        }
    }

    #[cfg(test)]
    pub(crate) fn app_setting_daemon_window_local_count(&self) -> usize {
        self.locals
            .iter()
            .filter(|local| {
                matches!(
                    local.owner,
                    CheckedLocalOwner::AppSettingDaemonWindow { .. }
                )
            })
            .count()
    }

    pub(crate) fn expression(&self, id: CheckedExprId) -> &CheckedExpr {
        self.record_lookup();
        &self.expressions[id.0 as usize]
    }

    pub(crate) fn try_expression(&self, id: CheckedExprId) -> Option<&CheckedExpr> {
        self.record_lookup();
        self.expressions.get(id.0 as usize)
    }

    pub(crate) fn expressions(&self) -> &[CheckedExpr] {
        &self.expressions
    }

    pub(crate) fn validate_expression_arena(&self) -> Result<(), (OriginId, &'static str)> {
        let mut state = vec![0u8; self.expressions.len()];
        for raw in 0..self.expressions.len() {
            if state[raw] == 2 {
                continue;
            }
            let root = CheckedExprId(raw as u32);
            let root_origin = self.expressions[raw].origin;
            let mut stack = vec![(root, false, root_origin)];
            while let Some((id, leaving, source_origin)) = stack.pop() {
                let Some(mark) = state.get_mut(id.0 as usize) else {
                    return Err((
                        source_origin,
                        "expression descendant ID is outside its arena",
                    ));
                };
                if leaving {
                    *mark = 2;
                    continue;
                }
                match *mark {
                    1 => {
                        return Err((source_origin, "checked expression graph contains a cycle"));
                    }
                    2 => continue,
                    _ => *mark = 1,
                }
                let expression = &self.expressions[id.0 as usize];
                stack.push((id, true, expression.origin));
                match &expression.kind {
                    CheckedExprKind::Path { root, .. } => match root {
                        CheckedPathRoot::Value(value) => {
                            if !self.values_by_ref.contains_key(value) {
                                return Err((
                                    expression.origin,
                                    "expression value ID is outside its arena",
                                ));
                            }
                        }
                        CheckedPathRoot::Local(local) => {
                            if self.locals.get(local.0 as usize).is_none() {
                                return Err((
                                    expression.origin,
                                    "expression local ID is outside its arena",
                                ));
                            }
                        }
                        CheckedPathRoot::EnumVariant(_) | CheckedPathRoot::Palette(_) => {}
                    },
                    CheckedExprKind::Call { target, arguments } => {
                        if let CheckedCallTarget::Builtin(id) = target
                            && self.builtins.get(id.0 as usize).is_none()
                        {
                            return Err((
                                expression.origin,
                                "expression builtin ID is outside its arena",
                            ));
                        }
                        for argument in arguments.iter().rev() {
                            match argument {
                                CheckedCallArgument::Value(value) => {
                                    stack.push((*value, false, expression.origin));
                                }
                                CheckedCallArgument::Binding(local) => {
                                    if self.locals.get(local.0 as usize).is_none() {
                                        return Err((
                                            expression.origin,
                                            "expression binding local ID is outside its arena",
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    CheckedExprKind::List(values) => {
                        stack.extend(
                            values
                                .iter()
                                .rev()
                                .map(|value| (*value, false, expression.origin)),
                        );
                    }
                    CheckedExprKind::Unary { value, .. } => {
                        stack.push((*value, false, expression.origin));
                    }
                    CheckedExprKind::Binary { left, right, .. } => {
                        stack.push((*right, false, expression.origin));
                        stack.push((*left, false, expression.origin));
                    }
                    CheckedExprKind::Bool(_)
                    | CheckedExprKind::I64(_)
                    | CheckedExprKind::F64(_)
                    | CheckedExprKind::Str(_)
                    | CheckedExprKind::Bytes(_)
                    | CheckedExprKind::None
                    | CheckedExprKind::SlotProvided(_) => {}
                }
            }
        }
        Ok(())
    }

    pub(crate) fn editor_self_move_contract(
        &self,
        expression: CheckedExprUseId,
        target: CheckedValueRef,
        declarations: &DeclarationIndex,
    ) -> Result<bool, (OriginId, &'static str)> {
        let expression_use = self.try_expression_use(expression).ok_or((
            OriginId(u32::MAX),
            "editor expression-use ID is outside its arena",
        ))?;
        let root = self.try_expression(expression_use.root).ok_or((
            expression_use.origin,
            "editor expression root ID is outside its arena",
        ))?;
        let CheckedExprKind::Call {
            target: CheckedCallTarget::Extern(function),
            ..
        } = root.kind
        else {
            return Ok(false);
        };
        let function = declarations
            .try_extern_decl(function)
            .ok_or((root.origin, "editor sync extern ID is outside its arena"))?;
        if function.kind != ExternKind::Sync {
            return Ok(false);
        }

        let mut occurrences = 0usize;
        let mut stack = vec![expression_use.root];
        while let Some(id) = stack.pop() {
            let expression = self.try_expression(id).ok_or((
                expression_use.origin,
                "editor expression descendant ID is outside its arena",
            ))?;
            match &expression.kind {
                CheckedExprKind::Path { root, .. } => {
                    occurrences += usize::from(*root == CheckedPathRoot::Value(target));
                }
                CheckedExprKind::List(values) => stack.extend(values.iter().copied()),
                CheckedExprKind::Call { arguments, .. } => {
                    stack.extend(arguments.iter().filter_map(|argument| match argument {
                        CheckedCallArgument::Value(value) => Some(*value),
                        CheckedCallArgument::Binding(_) => None,
                    }));
                }
                CheckedExprKind::Unary { value, .. } => stack.push(*value),
                CheckedExprKind::Binary { left, right, .. } => {
                    stack.extend([*left, *right]);
                }
                CheckedExprKind::Bool(_)
                | CheckedExprKind::I64(_)
                | CheckedExprKind::F64(_)
                | CheckedExprKind::Str(_)
                | CheckedExprKind::Bytes(_)
                | CheckedExprKind::None
                | CheckedExprKind::SlotProvided(_) => {}
            }
        }
        Ok(occurrences == 1)
    }

    pub(crate) fn builtin(&self, id: CheckedBuiltinId) -> &str {
        self.record_lookup();
        &self.builtins[id.0 as usize]
    }

    pub(crate) fn handler(&self, id: HandlerId) -> &CheckedHandler {
        self.record_lookup();
        &self.handlers[id.0 as usize]
    }

    pub(crate) fn try_handler(&self, id: HandlerId) -> Option<&CheckedHandler> {
        self.record_lookup();
        self.handlers.get(id.0 as usize)
    }

    pub(crate) fn statement(&self, id: StatementId) -> &CheckedStatement {
        self.record_lookup();
        self.statements[id.0 as usize]
            .as_ref()
            .expect("checked statement arena must be complete")
    }

    pub(crate) fn try_statement(&self, id: StatementId) -> Option<&CheckedStatement> {
        self.record_lookup();
        self.statements.get(id.0 as usize)?.as_ref()
    }

    pub(crate) fn route(&self, id: RouteId) -> &CheckedRoute {
        self.record_lookup();
        self.routes[id.0 as usize]
            .as_ref()
            .expect("checked route arena must be complete")
    }

    pub(crate) fn try_route(&self, id: RouteId) -> Option<&CheckedRoute> {
        self.record_lookup();
        self.routes.get(id.0 as usize)?.as_ref()
    }

    pub(crate) fn task(&self, id: TaskId) -> &CheckedTask {
        self.record_lookup();
        self.tasks[id.0 as usize]
            .as_ref()
            .expect("checked task arena must be complete")
    }

    pub(crate) fn try_task(&self, id: TaskId) -> Option<&CheckedTask> {
        self.record_lookup();
        self.tasks.get(id.0 as usize)?.as_ref()
    }

    pub(crate) fn metrics(&self) -> CheckedFactMetrics {
        self.metrics
    }

    #[cfg(test)]
    pub(crate) fn lookup_count(&self) -> usize {
        self.lookup_count.0.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    pub(crate) fn reset_lookup_count(&self) {
        self.lookup_count.0.store(0, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn record_lookup(&self) {
        self.lookup_count.0.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(test))]
    fn record_lookup(&self) {}
}

pub(in crate::check) fn build(
    document: &Document,
    declarations: &DeclarationIndex,
    origins: &mut OriginArena,
    analyses: CheckedAnalyses,
) -> Result<CheckedFacts, Error> {
    FactsBuilder::new(document, declarations, origins, analyses).build()
}

#[derive(Debug, Default)]
pub(super) struct CheckedAnalyses {
    entries: HashMap<CheckedExprOwner, ExprTypeAnalysis>,
    handler_entries: HashMap<usize, ExprTypeAnalysis>,
    handler_route_inputs: HashMap<usize, super::expr::CapturedRouteInputs>,
    preset_handlers: Vec<Handler>,
    pub(super) view_scope_env_overlays: usize,
    pub(super) view_scope_env_full_clones: usize,
}

impl CheckedAnalyses {
    pub(super) fn insert(
        &mut self,
        owner: CheckedValueRef,
        analysis: ExprTypeAnalysis,
    ) -> Result<(), Error> {
        self.insert_expression(CheckedExprOwner::Value(owner), analysis)
    }

    pub(super) fn insert_expression(
        &mut self,
        owner: CheckedExprOwner,
        analysis: ExprTypeAnalysis,
    ) -> Result<(), Error> {
        if self.entries.insert(owner, analysis).is_some() {
            return Err(Error::new(
                "E196",
                &Span::line(1),
                "checked expression owner was analyzed more than once",
            ));
        }
        Ok(())
    }

    fn remove(&mut self, owner: CheckedExprOwner) -> Option<ExprTypeAnalysis> {
        self.entries.remove(&owner)
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
            && self.handler_entries.is_empty()
            && self.handler_route_inputs.is_empty()
            && self.preset_handlers.is_empty()
    }

    pub(super) fn extend(&mut self, other: Self) -> Result<(), Error> {
        self.view_scope_env_overlays += other.view_scope_env_overlays;
        self.view_scope_env_full_clones += other.view_scope_env_full_clones;
        for (owner, analysis) in other.entries {
            self.insert_expression(owner, analysis)?;
        }
        for (key, analysis) in other.handler_entries {
            if self.handler_entries.insert(key, analysis).is_some() {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    "handler expression was captured more than once",
                ));
            }
        }
        for (key, route) in other.handler_route_inputs {
            if self.handler_route_inputs.insert(key, route).is_some() {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    "handler route input contract was captured more than once",
                ));
            }
        }
        if !other.preset_handlers.is_empty() {
            if !self.preset_handlers.is_empty() {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    "preset handler analysis was retained more than once",
                ));
            }
            self.preset_handlers = other.preset_handlers;
        }
        Ok(())
    }

    pub(super) fn retain_handlers(
        &mut self,
        analyses: super::expr::HandlerAnalyses,
        preset_handlers: Vec<Handler>,
    ) {
        self.handler_entries = analyses.expressions;
        self.handler_route_inputs = analyses.routes;
        self.preset_handlers = preset_handlers;
    }
}

struct FactsBuilder<'a> {
    document: &'a Document,
    declarations: &'a DeclarationIndex,
    origins: &'a mut OriginArena,
    facts: CheckedFacts,
    values_by_scope: HashMap<ValueScope, HashMap<String, CheckedValueId>>,
    builtins_by_name: HashMap<String, CheckedBuiltinId>,
    dynamic_pane_grids: std::collections::HashSet<String>,
    analyses: CheckedAnalyses,
}

#[derive(Clone, Debug, Default)]
struct FactEnv {
    paths: HashMap<String, (CheckedPathRoot, Type)>,
    slots: HashMap<String, ComponentSlotId>,
}

impl FactEnv {
    fn insert(&mut self, name: String, root: CheckedPathRoot, ty: Type) {
        self.paths.insert(name, (root, ty));
    }

    fn len(&self) -> usize {
        self.paths.len() + self.slots.len()
    }

    fn insert_slot(&mut self, name: String, slot: ComponentSlotId) {
        self.slots.insert(name, slot);
    }
}

trait FactEnvironment: ExprTypeEnv {
    fn get(&self, name: &str) -> Option<&(CheckedPathRoot, Type)>;
    fn slot(&self, name: &str) -> Option<ComponentSlotId>;
}

impl FactEnvironment for FactEnv {
    fn get(&self, name: &str) -> Option<&(CheckedPathRoot, Type)> {
        self.paths.get(name)
    }

    fn slot(&self, name: &str) -> Option<ComponentSlotId> {
        self.slots.get(name).copied()
    }
}

impl ExprTypeEnv for FactEnv {
    fn get_type(&self, name: &str) -> Option<&Type> {
        self.paths.get(name).map(|(_, ty)| ty)
    }

    fn visit_types(&self, visitor: &mut dyn FnMut(&str, &Type)) {
        for (name, (_, ty)) in &self.paths {
            visitor(name, ty);
        }
    }

    fn type_with_prefix(&self, prefix: &str) -> Option<&Type> {
        self.paths
            .iter()
            .find_map(|(name, (_, ty))| name.starts_with(prefix).then_some(ty))
    }
}

struct LayeredFactEnv<'a> {
    base: &'a dyn FactEnvironment,
    name: String,
    value: (CheckedPathRoot, Type),
}

impl FactEnvironment for LayeredFactEnv<'_> {
    fn get(&self, name: &str) -> Option<&(CheckedPathRoot, Type)> {
        if name == self.name {
            Some(&self.value)
        } else {
            self.base.get(name)
        }
    }

    fn slot(&self, name: &str) -> Option<ComponentSlotId> {
        self.base.slot(name)
    }
}

impl ExprTypeEnv for LayeredFactEnv<'_> {
    fn get_type(&self, name: &str) -> Option<&Type> {
        if name == self.name {
            Some(&self.value.1)
        } else {
            self.base.get(name).map(|(_, ty)| ty)
        }
    }

    fn visit_types(&self, visitor: &mut dyn FnMut(&str, &Type)) {
        self.base.visit_types(visitor);
        visitor(&self.name, &self.value.1);
    }

    fn type_with_prefix(&self, prefix: &str) -> Option<&Type> {
        self.name
            .starts_with(prefix)
            .then_some(&self.value.1)
            .or_else(|| self.base.type_with_prefix(prefix))
    }
}

struct HandlerFactEnv<'a> {
    base: &'a dyn FactEnvironment,
    locals: FactEnv,
}

impl<'a> HandlerFactEnv<'a> {
    fn new(base: &'a dyn FactEnvironment) -> Self {
        Self {
            base,
            locals: FactEnv::default(),
        }
    }

    fn insert(&mut self, name: String, root: CheckedPathRoot, ty: Type) {
        self.locals.insert(name, root, ty);
    }
}

impl ExprTypeEnv for HandlerFactEnv<'_> {
    fn get_type(&self, name: &str) -> Option<&Type> {
        self.locals
            .paths
            .get(name)
            .map(|(_, ty)| ty)
            .or_else(|| self.base.get(name).map(|(_, ty)| ty))
    }

    fn visit_types(&self, visitor: &mut dyn FnMut(&str, &Type)) {
        self.base.visit_types(visitor);
        self.locals.visit_types(visitor);
    }

    fn type_with_prefix(&self, prefix: &str) -> Option<&Type> {
        self.locals
            .type_with_prefix(prefix)
            .or_else(|| self.base.type_with_prefix(prefix))
    }
}

impl FactEnvironment for HandlerFactEnv<'_> {
    fn get(&self, name: &str) -> Option<&(CheckedPathRoot, Type)> {
        self.locals.paths.get(name).or_else(|| self.base.get(name))
    }

    fn slot(&self, name: &str) -> Option<ComponentSlotId> {
        self.locals.slot(name).or_else(|| self.base.slot(name))
    }
}

#[derive(Clone, Copy)]
struct ExpressionLowering<'a> {
    analysis: &'a ExprTypeAnalysis,
    owner: CheckedExprUseId,
    origin: OriginId,
    span: &'a Span,
}

impl<'a> FactsBuilder<'a> {
    fn new(
        document: &'a Document,
        declarations: &'a DeclarationIndex,
        origins: &'a mut OriginArena,
        analyses: CheckedAnalyses,
    ) -> Self {
        let mut facts = CheckedFacts::default();
        facts.metrics.view_scope_env_overlays = analyses.view_scope_env_overlays;
        facts.metrics.view_scope_env_full_clones = analyses.view_scope_env_full_clones;
        Self {
            document,
            declarations,
            origins,
            facts,
            values_by_scope: HashMap::new(),
            builtins_by_name: HashMap::new(),
            dynamic_pane_grids: crate::hir::dynamic_pane_grids(document),
            analyses,
        }
    }

    fn build(mut self) -> Result<CheckedFacts, Error> {
        self.facts.app_settings = Some(CheckedAppSettings {
            app_name: self.document.app.clone(),
            daemon: self.document.daemon,
            source: self.document.settings.clone(),
            has_default_font: self.document.fonts.iter().any(|font| font.default),
        });
        self.index_values()?;
        self.lower_initializers()?;
        self.lower_app_setting_expressions()?;
        self.index_views()?;
        self.lower_view_expressions()?;
        self.lower_handler_expressions()?;
        self.facts.metrics.handler_auxiliary_analyses = self.analyses.handler_entries.len();
        self.analyses.handler_entries.clear();
        if !self.analyses.is_empty() {
            return Err(self.invariant(
                &Span::line(1),
                format!(
                    "checked analyses were not consumed (expressions={}, handler_expressions={}, handler_routes={}, presets={})",
                    self.analyses.entries.len(),
                    self.analyses.handler_entries.len(),
                    self.analyses.handler_route_inputs.len(),
                    self.analyses.preset_handlers.len(),
                ),
            ));
        }
        if let Some(expression) = self
            .facts
            .expressions
            .iter()
            .find(|expression| contains_unknown(&expression.ty))
        {
            let origin = self.origins.get(expression.origin);
            return Err(self.invariant(
                &Span {
                    line: origin.line,
                    column: origin.column,
                },
                "checked expression fact retained an unresolved type",
            ));
        }
        self.facts.metrics.values = self.facts.values.len();
        self.facts.metrics.locals = self.facts.locals.len();
        self.facts.metrics.views = self.facts.views.len();
        self.facts.metrics.expression_uses = self.facts.expression_uses.len();
        self.facts.metrics.expressions = self.facts.expressions.len();
        Ok(self.facts)
    }

    fn index_values(&mut self) -> Result<(), Error> {
        for (index, state) in self.document.states.iter().enumerate() {
            let declaration = self.declarations.app_state(index);
            self.push_value(
                ValueScope::App,
                state.name.clone(),
                state.ty.clone(),
                CheckedValueRef::AppState(declaration.id),
                declaration.origin,
                &state.span,
            )?;
        }
        for (index, derived) in self.document.derived.iter().enumerate() {
            let declaration = self.declarations.derived(index);
            self.push_value(
                ValueScope::App,
                derived.name.clone(),
                derived.ty.clone(),
                CheckedValueRef::Derived(declaration.id),
                declaration.origin,
                &derived.span,
            )?;
        }
        for (component_index, component) in self.document.components.iter().enumerate() {
            let component_id = self.declarations.component(component_index).id;
            let scope = ValueScope::Component(component_id);
            for (index, param) in component.params.iter().enumerate() {
                let declaration = self.declarations.component_param(component_id, index);
                self.push_value(
                    scope,
                    param.name.clone(),
                    param.ty.clone(),
                    CheckedValueRef::ComponentParam(declaration.id),
                    declaration.origin,
                    &component.span,
                )?;
            }
            for (index, state) in component.states.iter().enumerate() {
                let declaration = self.declarations.component_state(component_id, index);
                self.push_value(
                    scope,
                    state.name.clone(),
                    state.ty.clone(),
                    CheckedValueRef::ComponentState(declaration.id),
                    declaration.origin,
                    &state.span,
                )?;
            }
        }
        Ok(())
    }

    fn push_value(
        &mut self,
        scope: ValueScope,
        name: String,
        ty: Type,
        value_ref: CheckedValueRef,
        origin: OriginId,
        span: &Span,
    ) -> Result<CheckedValueId, Error> {
        let id = CheckedValueId(self.facts.values.len() as u32);
        if self
            .values_by_scope
            .entry(scope)
            .or_default()
            .insert(name.clone(), id)
            .is_some()
        {
            return Err(self.invariant(span, format!("duplicate checked value `{name}`")));
        }
        self.facts.values.push(CheckedValue {
            name,
            ty,
            id: value_ref,
            initializer: None,
            origin,
        });
        self.facts.values_by_ref.insert(value_ref, id);
        Ok(id)
    }

    fn lower_initializers(&mut self) -> Result<(), Error> {
        let mut app_env = FactEnv::default();
        for (name, id) in self
            .values_by_scope
            .get(&ValueScope::App)
            .into_iter()
            .flatten()
        {
            let value = &self.facts.values[id.0 as usize];
            app_env.insert(
                name.clone(),
                CheckedPathRoot::Value(value.id),
                value.ty.clone(),
            );
        }
        self.facts.metrics.scope_env_builds += 1;
        self.facts.metrics.scope_env_entries += app_env.len();
        let empty_env = FactEnv::default();

        for (index, state) in self.document.states.iter().enumerate() {
            let id = self.value_id(ValueScope::App, &state.name, &state.span)?;
            self.push_expression_use(id, &state.initial, &state.ty, &empty_env, &state.span)?;
            debug_assert_eq!(id, CheckedValueId(index as u32));
        }
        for derived in &self.document.derived {
            let id = self.value_id(ValueScope::App, &derived.name, &derived.span)?;
            self.push_expression_use(id, &derived.value, &derived.ty, &app_env, &derived.span)?;
        }
        for (component_index, component) in self.document.components.iter().enumerate() {
            let scope = ValueScope::Component(self.declarations.component(component_index).id);
            for param in &component.params {
                let Some(default) = &param.default else {
                    continue;
                };
                let id = self.value_id(scope, &param.name, &component.span)?;
                self.push_expression_use(id, default, &param.ty, &empty_env, &component.span)?;
            }
            for state in &component.states {
                let id = self.value_id(scope, &state.name, &state.span)?;
                self.push_expression_use(id, &state.initial, &state.ty, &empty_env, &state.span)?;
            }
        }
        Ok(())
    }

    fn lower_app_setting_expressions(&mut self) -> Result<(), Error> {
        if self.document.settings.title.is_none()
            && self.document.settings.theme.is_none()
            && self.document.settings.palette.is_none()
            && self.document.settings.background.is_none()
            && self.document.settings.text_color.is_none()
            && self.document.settings.scale_factor.is_none()
        {
            return Ok(());
        }
        let mut state_env = FactEnv::default();
        for (index, state) in self.document.states.iter().enumerate() {
            let value_ref = CheckedValueRef::AppState(self.declarations.app_state(index).id);
            let value = self.facts.value_by_ref(value_ref);
            state_env.insert(
                state.name.clone(),
                CheckedPathRoot::Value(value.id),
                value.ty.clone(),
            );
        }
        self.facts.metrics.scope_env_builds += 1;
        self.facts.metrics.scope_env_entries += state_env.len();

        let lower = |this: &mut Self,
                     id: AppSettingExprId,
                     expression: &Expr,
                     expected: &Type,
                     env: &dyn FactEnvironment,
                     span: &Span|
         -> Result<(), Error> {
            let declaration = this
                .declarations
                .app_setting_expression(id)
                .ok_or_else(|| this.invariant(span, "app setting has no stable expression ID"))?;
            this.push_retained_expression(
                CheckedExprOwner::AppSetting(id),
                expression,
                expected,
                env,
                span,
                declaration.origin,
            )?;
            Ok(())
        };

        for (id, setting) in [
            (
                AppSettingExprId::Background,
                &self.document.settings.background,
            ),
            (
                AppSettingExprId::TextColor,
                &self.document.settings.text_color,
            ),
        ] {
            if let Some(setting) = setting {
                lower(
                    self,
                    id,
                    &setting.value,
                    &Type::Str,
                    &state_env,
                    &setting.span,
                )?;
            }
        }

        let mut callback_env = state_env;
        let callback_ids = [
            AppSettingExprId::Title,
            AppSettingExprId::Theme,
            AppSettingExprId::Palette,
            AppSettingExprId::ScaleFactor,
        ];
        if self.document.daemon {
            let owner = callback_ids
                .into_iter()
                .find(|id| self.declarations.app_setting_expression(*id).is_some());
            if let Some(id) = owner {
                let declaration = self.declarations.app_setting_expression(id).unwrap();
                let local = self.push_local(
                    "window",
                    Type::WindowId,
                    CheckedLocalOwner::AppSettingDaemonWindow { setting: id },
                    &self.document.settings.span,
                    declaration.origin,
                );
                callback_env.insert(
                    "window".into(),
                    CheckedPathRoot::Local(local),
                    Type::WindowId,
                );
            }
        }
        if let Some(setting) = &self.document.settings.title {
            lower(
                self,
                AppSettingExprId::Title,
                &setting.value,
                &Type::Str,
                &callback_env,
                &setting.span,
            )?;
        }
        if let Some(setting) = &self.document.settings.theme {
            if let Expr::Call { name, args } = &setting.value
                && let Some(factory) =
                    self.document.functions.iter().find(|function| {
                        function.name == *name && function.kind == ExternKind::Theme
                    })
            {
                let function = self
                    .declarations
                    .extern_decl_by_name(name)
                    .ok_or_else(|| {
                        self.invariant(
                            &setting.span,
                            "app theme factory has no stable extern declaration",
                        )
                    })?
                    .declaration
                    .id;
                self.facts.app_theme_factory = Some(CheckedAppThemeFactory {
                    function,
                    arguments: args.len() as u32,
                });
                for (index, (argument, (_, expected))) in
                    args.iter().zip(&factory.params).enumerate()
                {
                    lower(
                        self,
                        AppSettingExprId::ThemeFactoryArgument(index as u32),
                        argument,
                        expected,
                        &callback_env,
                        &setting.span,
                    )?;
                }
            } else {
                lower(
                    self,
                    AppSettingExprId::Theme,
                    &setting.value,
                    &Type::Str,
                    &callback_env,
                    &setting.span,
                )?;
            }
        }
        if let Some(setting) = &self.document.settings.palette {
            let contract = self.document.theme_contract.as_ref().ok_or_else(|| {
                self.invariant(&setting.span, "app palette has no checked theme contract")
            })?;
            lower(
                self,
                AppSettingExprId::Palette,
                &setting.value,
                &Type::Palette(contract.name.clone()),
                &callback_env,
                &setting.span,
            )?;
        }
        if let Some(setting) = &self.document.settings.scale_factor {
            lower(
                self,
                AppSettingExprId::ScaleFactor,
                &setting.value,
                &Type::F64,
                &callback_env,
                &setting.span,
            )?;
        }
        Ok(())
    }

    fn lower_view_expressions(&mut self) -> Result<(), Error> {
        let mut app_env = self.fact_env(ValueScope::App);
        if self.document.daemon {
            let view = self
                .declarations
                .view_id(self.document.view.span())
                .ok_or_else(|| {
                    self.invariant(self.document.view.span(), "daemon root has no view ID")
                })?;
            let local = self.push_view_local(
                "window",
                Type::WindowId,
                view,
                CheckedViewLocalRole::DaemonWindow,
                self.document.view.span(),
            );
            app_env.insert(
                "window".into(),
                CheckedPathRoot::Local(local),
                Type::WindowId,
            );
        }
        self.lower_view_expression_tree(&self.document.view, &app_env)?;

        for (index, component) in self.document.components.iter().enumerate() {
            let component_id = self.declarations.component(index).id;
            let mut env = self.fact_env(ValueScope::Component(component_id));
            for (slot_index, (name, _, _)) in crate::check::component_slots(&component.root)
                .into_iter()
                .enumerate()
            {
                env.insert_slot(
                    name.to_owned(),
                    self.declarations
                        .component_slot(component_id, slot_index)
                        .id,
                );
            }
            self.lower_view_expression_tree(&component.root, &env)?;
        }
        for test in &self.document.tests {
            if let Some(mount) = &test.mount {
                self.lower_view_expression_tree(mount, &app_env)?;
            }
        }
        Ok(())
    }

    fn lower_handler_expressions(&mut self) -> Result<(), Error> {
        self.facts.statements = vec![None; self.declarations.statement_count()];
        self.facts.tasks = vec![None; self.declarations.task_count()];
        self.facts.routes = vec![None; self.declarations.route_count()];
        let mut handler_index = 0usize;
        let app_env = (!self.document.handlers.is_empty() || !self.document.presets.is_empty())
            .then(|| self.fact_env(ValueScope::App));

        let document = self.document;
        for handler in &document.handlers {
            self.lower_handler(
                handler_index,
                handler,
                app_env.as_ref().expect("app handler environment exists"),
            )?;
            handler_index += 1;
        }
        for (component_index, component) in document.components.iter().enumerate() {
            let component_id = self.declarations.component(component_index).id;
            if component.handlers.is_empty() {
                continue;
            }
            let mut component_env = FactEnv::default();
            for state in &component.states {
                let value = self.value_id(
                    ValueScope::Component(component_id),
                    &state.name,
                    &state.span,
                )?;
                let value = &self.facts.values[value.0 as usize];
                component_env.insert(
                    state.name.clone(),
                    CheckedPathRoot::Value(value.id),
                    value.ty.clone(),
                );
            }
            self.facts.metrics.scope_env_builds += 1;
            self.facts.metrics.scope_env_entries += component_env.len();
            for handler in &component.handlers {
                self.lower_handler(handler_index, handler, &component_env)?;
                handler_index += 1;
            }
        }
        let preset_handlers = std::mem::take(&mut self.analyses.preset_handlers);
        for handler in preset_handlers {
            self.lower_handler(
                handler_index,
                &handler,
                app_env.as_ref().expect("preset handler environment exists"),
            )?;
            handler_index += 1;
        }

        if handler_index != self.declarations.handlers().len() {
            return Err(self.invariant(
                &Span::line(1),
                "handler fact traversal did not consume the declaration arena",
            ));
        }
        if let Some(task) = self.facts.tasks.iter().position(Option::is_none) {
            return Err(self.invariant(
                &Span::line(1),
                format!("checked task arena retained an unconsumed task {task}"),
            ));
        }
        if let Some(statement) = self.facts.statements.iter().position(Option::is_none) {
            return Err(self.invariant(
                &Span::line(1),
                format!("checked statement arena retained an unconsumed statement {statement}"),
            ));
        }
        if let Some(route) = self.facts.routes.iter().position(Option::is_none) {
            return Err(self.invariant(
                &Span::line(1),
                format!("checked route arena retained an unconsumed route {route}"),
            ));
        }
        Ok(())
    }

    fn lower_handler(
        &mut self,
        handler_index: usize,
        handler: &Handler,
        base_env: &FactEnv,
    ) -> Result<(), Error> {
        let mut env = HandlerFactEnv::new(base_env);
        let declaration = self
            .declarations
            .handlers()
            .get(handler_index)
            .ok_or_else(|| self.invariant(&handler.span, "handler has no declaration"))?
            .clone();
        if declaration.name != handler.name {
            return Err(self.invariant(
                &handler.span,
                "handler declaration owner order does not match the checked document",
            ));
        }
        let handler_id = declaration.declaration.id;
        let mut params = Vec::with_capacity(handler.params.len());
        for (index, param) in handler.params.iter().enumerate() {
            let owner = CheckedLocalOwner::HandlerParam {
                handler: handler_id,
                index: index as u32,
            };
            let local = self.push_handler_local(
                param.name.clone(),
                param.ty.clone(),
                owner,
                &handler.span,
                declaration.declaration.origin,
            )?;
            env.insert(
                param.name.clone(),
                CheckedPathRoot::Local(local),
                param.ty.clone(),
            );
            params.push(local);
        }
        self.facts.handlers.push(CheckedHandler {
            id: handler_id,
            params,
            param_names: handler
                .params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
            param_types: handler
                .params
                .iter()
                .map(|param| param.ty.clone())
                .collect(),
            origin: declaration.declaration.origin,
        });
        if declaration.statement_roots.len() != handler.statements.len() {
            return Err(self.invariant(
                &handler.span,
                "handler statement declaration count changed after checking",
            ));
        }
        for (statement, statement_id) in handler
            .statements
            .iter()
            .zip(declaration.statement_roots.iter().copied())
        {
            self.lower_handler_statement(statement, statement_id, &mut env)?;
        }
        Ok(())
    }

    fn push_handler_local(
        &mut self,
        name: String,
        ty: Type,
        owner: CheckedLocalOwner,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedLocalId, Error> {
        if self.facts.locals_by_owner.contains_key(&owner) {
            return Err(self.invariant(span, "checked handler local owner was produced twice"));
        }
        let id = CheckedLocalId(self.facts.locals.len() as u32);
        let origin = self.origins.push(span, Some(parent));
        self.facts.locals.push(CheckedLocal {
            name,
            ty,
            owner,
            origin,
        });
        self.facts.locals_by_owner.insert(owner, id);
        Ok(id)
    }

    fn push_handler_expression(
        &mut self,
        owner: CheckedExprOwner,
        expr: &Expr,
        expected: Option<&Type>,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let analysis = self
            .analyses
            .handler_entries
            .remove(&super::expr::expr_key(expr))
            .or_else(|| self.analyses.remove(owner))
            .ok_or_else(|| {
                self.invariant(
                    span,
                    format!("missing authoritative handler expression analysis for {owner:?}"),
                )
            })?;
        let metrics = analysis.metrics();
        self.facts.metrics.handler_authoritative_analyses += 1;
        self.facts.metrics.type_analysis_queries += metrics.queries;
        self.facts.metrics.type_analysis_nodes += metrics.nodes;
        self.facts.metrics.type_analysis_cache_hits += metrics.cache_hits;
        self.facts.metrics.type_scope_env_overlays += metrics.scoped_env_overlays;
        self.facts.metrics.type_scope_env_full_clones += metrics.scoped_env_full_clones;
        let inferred = analysis
            .type_of(expr)
            .cloned()
            .ok_or_else(|| self.invariant(span, "missing retained handler expression root type"))?;
        let source = resolve_erased_type(&contextual_type(inferred, expected));
        let id = CheckedExprUseId(self.facts.expression_uses.len() as u32);
        let origin = self.origins.push(span, Some(parent));
        let lowering = ExpressionLowering {
            analysis: &analysis,
            owner: id,
            origin,
            span,
        };
        let root = self.lower_expr(expr, Some(&source), env, lowering)?;
        if self.facts.expressions[root.0 as usize].ty != source {
            return Err(self.invariant(
                span,
                "handler expression source type does not match its checked root",
            ));
        }
        self.facts.expression_uses.push(CheckedExprUse {
            owner,
            root,
            source: source.clone(),
            destination: expected.cloned().unwrap_or(source),
            coercion: CheckedInitializerCoercion::None,
            origin,
        });
        if self
            .facts
            .expression_uses_by_owner
            .insert(owner, id)
            .is_some()
        {
            return Err(self.invariant(span, "duplicate checked handler expression owner"));
        }
        Ok(id)
    }

    fn statement_operand(
        &mut self,
        statement: StatementId,
        operand: &mut u32,
        expr: &Expr,
        expected: Option<&Type>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::HandlerStatement {
            statement,
            operand: *operand,
        };
        *operand += 1;
        let parent = self.declarations.statement(statement).declaration.origin;
        self.push_handler_expression(owner, expr, expected, env, span, parent)
    }

    fn task_operand(
        &mut self,
        task: TaskId,
        operand: &mut u32,
        expr: &Expr,
        expected: Option<&Type>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Task {
            task,
            operand: *operand,
        };
        *operand += 1;
        let parent = self.declarations.task(task).declaration.origin;
        self.push_handler_expression(owner, expr, expected, env, span, parent)
    }

    fn lower_route_expressions(
        &mut self,
        route: &Route,
        route_id: RouteId,
        env: &dyn FactEnvironment,
    ) -> Result<(), Error> {
        let inputs = self
            .analyses
            .handler_route_inputs
            .remove(&std::ptr::from_ref(route).addr())
            .ok_or_else(|| {
                self.invariant(&route.span, "missing authoritative route payload contract")
            })?;
        let declaration = self.declarations.route(route_id);
        let handler = self.declarations.statement(declaration.statement).handler;
        let source_owner = self.declarations.handler(handler).owner;
        let target_owner = match source_owner {
            crate::hir::HandlerOwner::Preset(_) => crate::hir::HandlerOwner::App,
            owner => owner,
        };
        let target = self
            .declarations
            .handler_id(target_owner, &route.handler)
            .ok_or_else(|| self.invariant(&route.span, "route target has no checked handler ID"))?;
        for (index, arg) in route.args.iter().enumerate() {
            let RouteArg::Expr(expr) = arg else {
                continue;
            };
            self.push_handler_expression(
                CheckedExprOwner::Route {
                    route: route_id,
                    argument: index as u32,
                },
                expr,
                None,
                env,
                &route.span,
                self.declarations.route(route_id).declaration.origin,
            )?;
        }
        if route_id.0 as usize >= self.facts.routes.len() {
            return Err(self.invariant(&route.span, "checked route ID is outside its arena"));
        }
        let slot = &mut self.facts.routes[route_id.0 as usize];
        if slot.is_some() {
            return Err(self.invariant(&route.span, "checked route was produced more than once"));
        }
        *slot = Some(CheckedRoute {
            id: route_id,
            target,
            target_owner,
            args: route
                .args
                .iter()
                .map(|arg| match arg {
                    RouteArg::Expr(_) => CheckedRouteArgKind::Expression,
                    RouteArg::Payload => CheckedRouteArgKind::Payload,
                })
                .collect(),
            source_payloads: inputs.payloads,
            ordered_payloads: inputs.ordered,
            origin: declaration.declaration.origin,
        });
        Ok(())
    }

    fn checked_writable_targets(
        &self,
        statement: &Statement,
        env: &dyn FactEnvironment,
    ) -> Result<Vec<CheckedValueRef>, Error> {
        let names: Vec<&str> = match statement {
            Statement::Assign { target, .. }
            | Statement::MarkdownAppend { target, .. }
            | Statement::ComboPush { target, .. }
            | Statement::DebugStart { target, .. }
            | Statement::DebugFinish { target, .. } => vec![target],
            Statement::Abortable { handle, .. } | Statement::Abort { handle, .. } => vec![handle],
            _ => Vec::new(),
        };
        names
            .into_iter()
            .map(|name| {
                let Some((CheckedPathRoot::Value(value), _)) = env.get(name) else {
                    return Err(self.invariant(
                        statement.span(),
                        "writable handler target has no checked state ID",
                    ));
                };
                Ok(*value)
            })
            .collect()
    }

    fn lower_handler_statement(
        &mut self,
        statement: &Statement,
        statement_id: StatementId,
        env: &mut HandlerFactEnv<'_>,
    ) -> Result<(), Error> {
        let declaration = self.declarations.statement(statement_id).clone();
        let writable_targets = self.checked_writable_targets(statement, env)?;
        let mut operand = 0u32;
        let mut routes = declaration.routes.iter().copied();
        match statement {
            Statement::Let { name, value, span } => {
                let expression =
                    self.statement_operand(statement_id, &mut operand, value, None, env, span)?;
                let ty = self.facts.expression_use(expression).source.clone();
                let owner = CheckedLocalOwner::StatementLet(statement_id);
                let local = self.push_handler_local(
                    name.clone(),
                    ty.clone(),
                    owner,
                    span,
                    declaration.declaration.origin,
                )?;
                env.insert(name.clone(), CheckedPathRoot::Local(local), ty);
            }
            Statement::Assign {
                target,
                value,
                at,
                span,
            } => {
                let expected = env.get(target).map(|(_, ty)| ty.clone()).ok_or_else(|| {
                    self.invariant(span, "assignment target has no checked value")
                })?;
                let value_expected = match &expected {
                    Type::Combo(inner) => Type::List(inner.clone()),
                    Type::Animation(inner) => (**inner).clone(),
                    other => other.clone(),
                };
                self.statement_operand(
                    statement_id,
                    &mut operand,
                    value,
                    Some(&value_expected),
                    env,
                    span,
                )?;
                if let Some(at) = at {
                    self.statement_operand(
                        statement_id,
                        &mut operand,
                        at,
                        Some(&Type::Instant),
                        env,
                        span,
                    )?;
                }
            }
            Statement::MarkdownAppend { value, span, .. } => {
                self.statement_operand(
                    statement_id,
                    &mut operand,
                    value,
                    Some(&Type::Str),
                    env,
                    span,
                )?;
            }
            Statement::ComboPush {
                target,
                value,
                span,
            } => {
                let expected = env
                    .get(target)
                    .and_then(|(_, ty)| match ty {
                        Type::Combo(inner) => Some((**inner).clone()),
                        _ => None,
                    })
                    .ok_or_else(|| self.invariant(span, "combo target has no checked item type"))?;
                self.statement_operand(
                    statement_id,
                    &mut operand,
                    value,
                    Some(&expected),
                    env,
                    span,
                )?;
            }
            Statement::ReturnIf { condition, span } => {
                self.statement_operand(
                    statement_id,
                    &mut operand,
                    condition,
                    Some(&Type::Bool),
                    env,
                    span,
                )?;
            }
            Statement::Exit { .. } => {
                self.record_checked_task(
                    declaration.task,
                    Some(Type::Unit),
                    None,
                    declaration.is_final,
                    statement.span(),
                )?;
            }
            Statement::Run {
                kind,
                function,
                args,
                success,
                error,
                span,
                ..
            } => {
                let task = declaration.task.ok_or_else(|| {
                    self.invariant(span, "run statement has no checked task declaration")
                })?;
                let (output, error_ty, expected, target) =
                    self.effect_contract(*kind, function, args, span)?;
                let mut task_operand = 0;
                for (index, arg) in args.iter().enumerate() {
                    self.task_operand(
                        task,
                        &mut task_operand,
                        arg,
                        expected.get(index),
                        env,
                        span,
                    )?;
                }
                self.record_checked_task(
                    Some(task),
                    Some(output),
                    error_ty,
                    declaration.is_final,
                    span,
                )?;
                self.set_checked_task_target(task, target, span)?;
                let success_id = routes
                    .next()
                    .ok_or_else(|| self.invariant(span, "run success route has no declaration"))?;
                self.lower_route_expressions(success, success_id, env)?;
                if let Some(error) = error {
                    let error_id = routes.next().ok_or_else(|| {
                        self.invariant(span, "run error route has no declaration")
                    })?;
                    self.lower_route_expressions(error, error_id, env)?;
                }
            }
            Statement::Sip {
                function,
                args,
                progress,
                success,
                error,
                span,
            } => {
                let task = declaration.task.ok_or_else(|| {
                    self.invariant(span, "sip statement has no checked task declaration")
                })?;
                let action = self
                    .declarations
                    .extern_decl_by_name(function)
                    .ok_or_else(|| self.invariant(span, "sip target has no extern declaration"))?
                    .clone();
                let mut task_operand = 0;
                for (index, arg) in args.iter().enumerate() {
                    self.task_operand(
                        task,
                        &mut task_operand,
                        arg,
                        action.params.get(index).map(|(_, ty)| ty),
                        env,
                        span,
                    )?;
                }
                self.record_checked_task(
                    Some(task),
                    Some(action.output),
                    action.error,
                    declaration.is_final,
                    span,
                )?;
                self.set_checked_task_target(
                    task,
                    CheckedEffectTarget::Extern(action.declaration.id),
                    span,
                )?;
                for route in std::iter::once(progress)
                    .chain(std::iter::once(success))
                    .chain(error.iter())
                {
                    let route_id = routes
                        .next()
                        .ok_or_else(|| self.invariant(span, "sip route has no declaration"))?;
                    self.lower_route_expressions(route, route_id, env)?;
                }
            }
            Statement::TaskFlow {
                source,
                transforms,
                success,
                error,
                units,
                span,
            } => {
                self.lower_task_flow(
                    source,
                    transforms,
                    statement_id,
                    declaration.task,
                    declaration.is_final,
                    env,
                    span,
                )?;
                for route in success.iter().chain(error.iter()).chain(units.iter()) {
                    let route_id = routes.next().ok_or_else(|| {
                        self.invariant(span, "task flow route has no declaration")
                    })?;
                    self.lower_route_expressions(route, route_id, env)?;
                }
            }
            Statement::TaskGroup {
                statements, span, ..
            } => {
                self.record_checked_task(
                    declaration.task,
                    Some(Type::Unit),
                    None,
                    declaration.is_final,
                    span,
                )?;
                if statements.len() != declaration.children.len() {
                    return Err(self.invariant(span, "task group child arena diverged"));
                }
                for (child, child_id) in statements.iter().zip(declaration.children.iter().copied())
                {
                    let mut child_env = HandlerFactEnv::new(env);
                    self.lower_handler_statement(child, child_id, &mut child_env)?;
                }
            }
            Statement::Abortable { task, span, .. } => {
                self.record_checked_task(
                    declaration.task,
                    Some(Type::Unit),
                    None,
                    declaration.is_final,
                    span,
                )?;
                let [child] = declaration.children.as_slice() else {
                    return Err(self.invariant(span, "abortable task child arena diverged"));
                };
                let mut child_env = HandlerFactEnv::new(env);
                self.lower_handler_statement(task, *child, &mut child_env)?;
            }
            Statement::Abort { .. }
            | Statement::DebugFinish { .. }
            | Statement::PaneOperation {
                operation:
                    PaneOperation::Maximize { .. }
                    | PaneOperation::Restore
                    | PaneOperation::Swap { .. }
                    | PaneOperation::Close { .. }
                    | PaneOperation::Move { .. }
                    | PaneOperation::Drop { .. },
                ..
            } => {
                self.lower_statement_operation_expressions(
                    statement,
                    statement_id,
                    &mut operand,
                    env,
                )?;
            }
            Statement::DebugStart { name, span, .. } => {
                self.statement_operand(
                    statement_id,
                    &mut operand,
                    name,
                    Some(&Type::Str),
                    env,
                    span,
                )?;
            }
            Statement::ClipboardWrite { value, span, .. } => {
                self.statement_operand(
                    statement_id,
                    &mut operand,
                    value,
                    Some(&Type::Str),
                    env,
                    span,
                )?;
                self.record_checked_task(
                    declaration.task,
                    Some(Type::Unit),
                    None,
                    declaration.is_final,
                    span,
                )?;
            }
            Statement::WidgetOperation { route, span, .. }
            | Statement::PaneOperation { route, span, .. }
            | Statement::WindowOperation { route, span, .. } => {
                self.lower_statement_operation_expressions(
                    statement,
                    statement_id,
                    &mut operand,
                    env,
                )?;
                if declaration.task.is_some() {
                    self.record_checked_task(
                        declaration.task,
                        Some(Type::Unit),
                        None,
                        declaration.is_final,
                        span,
                    )?;
                }
                if let Some(route) = route {
                    let route_id = routes.next().ok_or_else(|| {
                        self.invariant(span, "operation route has no declaration")
                    })?;
                    self.lower_route_expressions(route, route_id, env)?;
                }
            }
        }
        if routes.next().is_some() {
            return Err(self.invariant(
                statement.span(),
                "statement left a checked route declaration unconsumed",
            ));
        }
        let editor_self_move = match statement {
            Statement::Assign { span, .. } => {
                let target = writable_targets.first().copied().ok_or_else(|| {
                    self.invariant(span, "assignment has no checked writable target")
                })?;
                let target_ty = &self
                    .facts
                    .try_value_by_ref(target)
                    .ok_or_else(|| {
                        self.invariant(span, "assignment target ID is outside its arena")
                    })?
                    .ty;
                let value = self
                    .facts
                    .expression_use_by_owner(CheckedExprOwner::HandlerStatement {
                        statement: statement_id,
                        operand: 0,
                    })
                    .ok_or_else(|| {
                        self.invariant(span, "assignment value has no checked expression")
                    })?;
                Some(if *target_ty == Type::Editor {
                    self.facts
                        .editor_self_move_contract(value, target, self.declarations)
                        .map_err(|(_, message)| self.invariant(span, message))?
                } else {
                    false
                })
            }
            _ => None,
        };
        let pane_grid_dynamic = match statement {
            Statement::PaneOperation { grid, .. } => Some(self.dynamic_pane_grids.contains(grid)),
            _ => None,
        };
        if statement_id.0 as usize >= self.facts.statements.len() {
            return Err(self.invariant(
                statement.span(),
                "checked statement ID is outside its arena",
            ));
        }
        let slot = &mut self.facts.statements[statement_id.0 as usize];
        if slot.is_some() {
            return Err(self.invariant(
                statement.span(),
                "checked statement was produced more than once",
            ));
        }
        *slot = Some(CheckedStatement {
            id: statement_id,
            semantic_key: crate::hir::statement_semantic_key(statement),
            operation: crate::hir::handler_operation_contract(statement),
            writable_targets,
            editor_self_move,
            pane_grid_dynamic,
            operand_count: operand,
            origin: declaration.declaration.origin,
        });
        Ok(())
    }

    fn record_checked_task(
        &mut self,
        task: Option<TaskId>,
        output: Option<Type>,
        error: Option<Type>,
        is_final: bool,
        span: &Span,
    ) -> Result<(), Error> {
        let Some(task) = task else {
            return Ok(());
        };
        let declaration = self.declarations.task(task);
        let slot = self
            .facts
            .tasks
            .get_mut(task.0 as usize)
            .ok_or_else(|| Error::new("E196", span, "checked task ID is outside its arena"))?;
        if slot.is_some() {
            return Err(Error::new(
                "E196",
                span,
                "checked task declaration was consumed more than once",
            ));
        }
        *slot = Some(CheckedTask {
            id: task,
            output,
            error,
            target: None,
            is_final,
            origin: declaration.declaration.origin,
        });
        Ok(())
    }

    fn set_checked_task_target(
        &mut self,
        task: TaskId,
        target: CheckedEffectTarget,
        span: &Span,
    ) -> Result<(), Error> {
        let checked = self
            .facts
            .tasks
            .get_mut(task.0 as usize)
            .and_then(Option::as_mut)
            .ok_or_else(|| Error::new("E196", span, "effect task has no checked task fact"))?;
        if checked.target.replace(target).is_some() {
            return Err(Error::new(
                "E196",
                span,
                "effect task target was resolved more than once",
            ));
        }
        Ok(())
    }

    fn effect_contract(
        &self,
        kind: EffectKind,
        function: &str,
        args: &[Expr],
        span: &Span,
    ) -> Result<(Type, Option<Type>, Vec<Type>, CheckedEffectTarget), Error> {
        if let Some((output, error)) =
            super::handler::builtin_task_type(kind, function, args, span)?
        {
            let expected = match function {
                "__ice_font_load" => vec![Type::Bytes],
                "__ice_image_allocate" => vec![Type::Image],
                _ => Vec::new(),
            };
            return Ok((
                output,
                error,
                expected,
                CheckedEffectTarget::Builtin(function.to_owned()),
            ));
        }
        let action = self
            .declarations
            .extern_decl_by_name(function)
            .ok_or_else(|| self.invariant(span, "effect target has no extern declaration"))?;
        if action.kind != ExternKind::from(kind) {
            return Err(self.invariant(span, "effect target kind changed after checking"));
        }
        Ok((
            action.output.clone(),
            action.error.clone(),
            action.params.iter().map(|(_, ty)| ty.clone()).collect(),
            CheckedEffectTarget::Extern(action.declaration.id),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_task_flow(
        &mut self,
        source: &TaskSource,
        transforms: &[TaskTransform],
        statement: StatementId,
        root_task: Option<TaskId>,
        is_final: bool,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let declaration = self.declarations.statement(statement).clone();
        if declaration.source_tasks.len() != transforms.len() + 1 {
            return Err(self.invariant(span, "task flow arena shape diverged after checking"));
        }
        let (mut output, mut error) =
            self.lower_task_source(source, declaration.source_tasks[0], env)?;
        for (index, (transform, task)) in transforms
            .iter()
            .zip(declaration.source_tasks.iter().copied().skip(1))
            .enumerate()
        {
            match transform {
                TaskTransform::Map {
                    binding,
                    value,
                    span,
                } => {
                    let local = self.push_handler_local(
                        binding.clone(),
                        output.clone(),
                        CheckedLocalOwner::TaskTransform {
                            task,
                            index: index as u32,
                        },
                        span,
                        self.declarations.task(task).declaration.origin,
                    )?;
                    let mut transform_env = FactEnv::default();
                    transform_env.insert(binding.clone(), CheckedPathRoot::Local(local), output);
                    let mut operand = 0;
                    let expression =
                        self.task_operand(task, &mut operand, value, None, &transform_env, span)?;
                    output = self.facts.expression_use(expression).source.clone();
                    self.record_checked_task(
                        Some(task),
                        Some(output.clone()),
                        error.clone(),
                        false,
                        span,
                    )?;
                }
                TaskTransform::Then {
                    binding,
                    source,
                    span,
                } => {
                    let local = self.push_handler_local(
                        binding.clone(),
                        output.clone(),
                        CheckedLocalOwner::TaskTransform {
                            task,
                            index: index as u32,
                        },
                        span,
                        self.declarations.task(task).declaration.origin,
                    )?;
                    let mut transform_env = FactEnv::default();
                    transform_env.insert(binding.clone(), CheckedPathRoot::Local(local), output);
                    (output, error) = self.lower_task_source(source, task, &transform_env)?;
                }
                TaskTransform::AndThen {
                    binding,
                    source,
                    span,
                } => {
                    let binding_ty = if error.is_some() {
                        output.clone()
                    } else if let Type::Option(inner) = &output {
                        (**inner).clone()
                    } else {
                        return Err(self.invariant(span, "checked try transform is not optional"));
                    };
                    let local = self.push_handler_local(
                        binding.clone(),
                        binding_ty.clone(),
                        CheckedLocalOwner::TaskTransform {
                            task,
                            index: index as u32,
                        },
                        span,
                        self.declarations.task(task).declaration.origin,
                    )?;
                    let mut transform_env = FactEnv::default();
                    transform_env.insert(
                        binding.clone(),
                        CheckedPathRoot::Local(local),
                        binding_ty,
                    );
                    (output, error) = self.lower_task_source(source, task, &transform_env)?;
                }
                TaskTransform::MapError {
                    binding,
                    value,
                    span,
                } => {
                    let input = error.clone().ok_or_else(|| {
                        self.invariant(span, "checked map-err has no error input")
                    })?;
                    let local = self.push_handler_local(
                        binding.clone(),
                        input.clone(),
                        CheckedLocalOwner::TaskTransform {
                            task,
                            index: index as u32,
                        },
                        span,
                        self.declarations.task(task).declaration.origin,
                    )?;
                    let mut transform_env = FactEnv::default();
                    transform_env.insert(binding.clone(), CheckedPathRoot::Local(local), input);
                    let mut operand = 0;
                    let expression =
                        self.task_operand(task, &mut operand, value, None, &transform_env, span)?;
                    error = Some(self.facts.expression_use(expression).source.clone());
                    self.record_checked_task(
                        Some(task),
                        Some(output.clone()),
                        error.clone(),
                        false,
                        span,
                    )?;
                }
                TaskTransform::Collect { span } => {
                    let item = match error.take() {
                        Some(error) => Type::Result(Box::new(output), Box::new(error)),
                        None => output,
                    };
                    output = Type::List(Box::new(item));
                    self.record_checked_task(Some(task), Some(output.clone()), None, false, span)?;
                }
                TaskTransform::Discard { span } => {
                    self.record_checked_task(Some(task), None, None, false, span)?;
                    self.record_checked_task(root_task, None, None, is_final, span)?;
                    return Ok(());
                }
            }
        }
        self.record_checked_task(root_task, Some(output), error, is_final, span)
    }

    fn lower_task_source(
        &mut self,
        source: &TaskSource,
        task: TaskId,
        env: &dyn FactEnvironment,
    ) -> Result<(Type, Option<Type>), Error> {
        let (output, error, target) = match source {
            TaskSource::Done { value, span } => {
                let mut operand = 0;
                let value = self.task_operand(task, &mut operand, value, None, env, span)?;
                (self.facts.expression_use(value).source.clone(), None, None)
            }
            TaskSource::None { output, .. } => (output.clone(), None, None),
            TaskSource::Effect {
                kind,
                function,
                args,
                span,
            } => {
                let (output, error, expected, target) =
                    self.effect_contract(*kind, function, args, span)?;
                let mut operand = 0;
                for (index, arg) in args.iter().enumerate() {
                    self.task_operand(task, &mut operand, arg, expected.get(index), env, span)?;
                }
                (output, error, Some(target))
            }
        };
        let span = match source {
            TaskSource::Effect { span, .. }
            | TaskSource::Done { span, .. }
            | TaskSource::None { span, .. } => span,
        };
        self.record_checked_task(Some(task), Some(output.clone()), error.clone(), false, span)?;
        if let Some(target) = target {
            self.set_checked_task_target(task, target, span)?;
        }
        Ok((output, error))
    }

    fn lower_statement_operation_expressions(
        &mut self,
        statement: &Statement,
        statement_id: StatementId,
        operand: &mut u32,
        env: &dyn FactEnvironment,
    ) -> Result<(), Error> {
        let span = statement.span();
        match statement {
            Statement::WidgetOperation { operation, .. } => match operation {
                WidgetOperation::Focus { target }
                | WidgetOperation::Focused { target }
                | WidgetOperation::CursorFront { target }
                | WidgetOperation::CursorEnd { target }
                | WidgetOperation::SelectAll { target }
                | WidgetOperation::SnapEnd { target } => {
                    self.lower_widget_target(target, statement_id, operand, env, span)?;
                }
                WidgetOperation::Cursor { target, position } => {
                    self.lower_widget_target(target, statement_id, operand, env, span)?;
                    self.statement_operand(
                        statement_id,
                        operand,
                        position,
                        Some(&Type::I64),
                        env,
                        span,
                    )?;
                }
                WidgetOperation::Select { target, start, end } => {
                    self.lower_widget_target(target, statement_id, operand, env, span)?;
                    for value in [start, end] {
                        self.statement_operand(
                            statement_id,
                            operand,
                            value,
                            Some(&Type::I64),
                            env,
                            span,
                        )?;
                    }
                }
                WidgetOperation::Snap { target, x, y }
                | WidgetOperation::ScrollTo { target, x, y }
                | WidgetOperation::ScrollBy { target, x, y } => {
                    self.lower_widget_target(target, statement_id, operand, env, span)?;
                    for value in [x, y] {
                        self.statement_operand(
                            statement_id,
                            operand,
                            value,
                            Some(&Type::F64),
                            env,
                            span,
                        )?;
                    }
                }
                WidgetOperation::Find { selector, .. } => {
                    self.lower_widget_selector(selector, statement_id, operand, env, span)?;
                }
                WidgetOperation::FocusPrevious | WidgetOperation::FocusNext => {}
            },
            Statement::PaneOperation { operation, .. } => {
                let mut pane = |this: &mut Self, pane: &PaneReference| {
                    this.lower_pane_reference(pane, statement_id, operand, env, span)
                };
                match operation {
                    PaneOperation::Maximize { pane: value }
                    | PaneOperation::Close { pane: value }
                    | PaneOperation::Move { pane: value, .. }
                    | PaneOperation::Adjacent { pane: value, .. } => pane(self, value)?,
                    PaneOperation::Swap { first, second } => {
                        pane(self, first)?;
                        pane(self, second)?;
                    }
                    PaneOperation::Resize { ratio, .. } => {
                        self.statement_operand(
                            statement_id,
                            operand,
                            ratio,
                            Some(&Type::F64),
                            env,
                            span,
                        )?;
                    }
                    PaneOperation::Drop {
                        pane: value,
                        target,
                        ..
                    } => {
                        pane(self, value)?;
                        pane(self, target)?;
                    }
                    PaneOperation::Split {
                        target,
                        pane: value,
                        ratio,
                        ..
                    } => {
                        pane(self, target)?;
                        pane(self, value)?;
                        self.statement_operand(
                            statement_id,
                            operand,
                            ratio,
                            Some(&Type::F64),
                            env,
                            span,
                        )?;
                    }
                    PaneOperation::Restore | PaneOperation::Maximized => {}
                }
            }
            Statement::WindowOperation {
                operation, target, ..
            } => {
                if let Some(target) = target {
                    self.statement_operand(
                        statement_id,
                        operand,
                        target,
                        Some(&Type::WindowId),
                        env,
                        span,
                    )?;
                }
                self.lower_window_operation(operation, statement_id, operand, env, span)?;
            }
            Statement::Abort { .. }
            | Statement::DebugFinish { .. }
            | Statement::DebugStart { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::MarkdownAppend { .. }
            | Statement::ComboPush { .. }
            | Statement::ReturnIf { .. }
            | Statement::Exit { .. }
            | Statement::Run { .. }
            | Statement::Sip { .. }
            | Statement::TaskFlow { .. }
            | Statement::TaskGroup { .. }
            | Statement::Abortable { .. }
            | Statement::ClipboardWrite { .. } => {}
        }
        Ok(())
    }

    fn lower_widget_target(
        &mut self,
        target: &WidgetTarget,
        statement: StatementId,
        operand: &mut u32,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        for segment in &target.segments {
            if let Some(key) = &segment.key {
                self.statement_operand(statement, operand, key, None, env, span)?;
            }
        }
        Ok(())
    }

    fn lower_widget_selector(
        &mut self,
        selector: &WidgetSelector,
        statement: StatementId,
        operand: &mut u32,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        match selector {
            WidgetSelector::Id(target) => {
                self.lower_widget_target(target, statement, operand, env, span)?;
            }
            WidgetSelector::Text(value) => {
                self.statement_operand(statement, operand, value, Some(&Type::Str), env, span)?;
            }
            WidgetSelector::Point { x, y } => {
                for value in [x, y] {
                    self.statement_operand(statement, operand, value, Some(&Type::F64), env, span)?;
                }
            }
            WidgetSelector::Extern { function, args } => {
                let action = self
                    .declarations
                    .extern_decl_by_name(function)
                    .ok_or_else(|| self.invariant(span, "widget selector extern is unresolved"))?
                    .clone();
                for (index, arg) in args.iter().enumerate() {
                    self.statement_operand(
                        statement,
                        operand,
                        arg,
                        action.params.get(index).map(|(_, ty)| ty),
                        env,
                        span,
                    )?;
                }
            }
            WidgetSelector::Focused => {}
        }
        Ok(())
    }

    fn lower_pane_reference(
        &mut self,
        pane: &PaneReference,
        statement: StatementId,
        operand: &mut u32,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        if let PaneReference::Dynamic { key, .. } = pane {
            self.statement_operand(statement, operand, key, None, env, span)?;
        }
        Ok(())
    }

    fn lower_window_operation(
        &mut self,
        operation: &WindowOperation,
        statement: StatementId,
        operand: &mut u32,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let mut expression = |this: &mut Self, value: &Expr, expected: Option<&Type>| {
            this.statement_operand(statement, operand, value, expected, env, span)
                .map(|_| ())
        };
        match operation {
            WindowOperation::Resize(width, height) | WindowOperation::Move(width, height) => {
                expression(self, width, Some(&Type::F64))?;
                expression(self, height, Some(&Type::F64))?;
            }
            WindowOperation::Resizable(value)
            | WindowOperation::Maximize(value)
            | WindowOperation::Minimize(value)
            | WindowOperation::MousePassthrough(value)
            | WindowOperation::AutomaticTabbing(value) => {
                expression(self, value, Some(&Type::Bool))?;
            }
            WindowOperation::MinSize(value) | WindowOperation::MaxSize(value) => {
                if let Some((width, height)) = value {
                    expression(self, width, Some(&Type::F64))?;
                    expression(self, height, Some(&Type::F64))?;
                }
            }
            WindowOperation::ResizeIncrements(value) => {
                if let Some((width, height)) = value {
                    expression(self, width, Some(&Type::F64))?;
                    expression(self, height, Some(&Type::F64))?;
                }
            }
            WindowOperation::Icon {
                pixels,
                width,
                height,
            } => {
                expression(self, pixels, Some(&Type::Bytes))?;
                expression(self, width, Some(&Type::I64))?;
                expression(self, height, Some(&Type::I64))?;
            }
            WindowOperation::Callback { function, args } => {
                let action = self
                    .declarations
                    .extern_decl_by_name(function)
                    .ok_or_else(|| self.invariant(span, "window callback extern is unresolved"))?
                    .clone();
                for (index, arg) in args.iter().enumerate() {
                    expression(self, arg, action.params.get(index).map(|(_, ty)| ty))?;
                }
            }
            WindowOperation::Open(_)
            | WindowOperation::Oldest
            | WindowOperation::Latest
            | WindowOperation::Close
            | WindowOperation::Drag
            | WindowOperation::DragResize(_)
            | WindowOperation::Size
            | WindowOperation::IsMaximized
            | WindowOperation::IsMinimized
            | WindowOperation::Position
            | WindowOperation::ScaleFactor
            | WindowOperation::Mode
            | WindowOperation::SetMode(_)
            | WindowOperation::ToggleMaximize
            | WindowOperation::ToggleDecorations
            | WindowOperation::Attention(_)
            | WindowOperation::Focus
            | WindowOperation::SetLevel(_)
            | WindowOperation::SystemMenu
            | WindowOperation::RawId
            | WindowOperation::Screenshot
            | WindowOperation::MonitorSize => {}
        }
        Ok(())
    }

    fn fact_env(&mut self, scope: ValueScope) -> FactEnv {
        let mut env = FactEnv::default();
        for (name, id) in self.values_by_scope.get(&scope).into_iter().flatten() {
            let value = &self.facts.values[id.0 as usize];
            env.insert(
                name.clone(),
                CheckedPathRoot::Value(value.id),
                value.ty.clone(),
            );
        }
        self.facts.metrics.scope_env_builds += 1;
        self.facts.metrics.scope_env_entries += env.len();
        env
    }

    fn push_view_expression(
        &mut self,
        owner: CheckedExprOwner,
        expr: &Expr,
        expected: Option<&Type>,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let id = CheckedExprUseId(self.facts.expression_uses.len() as u32);
        let analysis = self.analyses.remove(owner).ok_or_else(|| {
            self.invariant(span, "missing authoritative view expression analysis")
        })?;
        let analysis_metrics = analysis.metrics();
        self.facts.metrics.view_analysis_passes += 1;
        self.facts.metrics.type_analysis_queries += analysis_metrics.queries;
        self.facts.metrics.type_analysis_nodes += analysis_metrics.nodes;
        self.facts.metrics.type_analysis_cache_hits += analysis_metrics.cache_hits;
        self.facts.metrics.type_scope_env_overlays += analysis_metrics.scoped_env_overlays;
        self.facts.metrics.type_scope_env_full_clones += analysis_metrics.scoped_env_full_clones;
        let inferred = analysis
            .type_of(expr)
            .cloned()
            .ok_or_else(|| self.invariant(span, "missing retained view expression root type"))?;
        let source = resolve_erased_type(&contextual_type(inferred, expected));
        let origin = self.origins.push(span, Some(parent));
        let lowering = ExpressionLowering {
            analysis: &analysis,
            owner: id,
            origin,
            span,
        };
        let root = self.lower_expr(expr, Some(&source), env, lowering)?;
        if self.facts.expressions[root.0 as usize].ty != source {
            return Err(self.invariant(
                span,
                "view expression source type does not match its checked root",
            ));
        }
        self.facts.expression_uses.push(CheckedExprUse {
            owner,
            root,
            source: source.clone(),
            destination: expected.cloned().unwrap_or(source),
            coercion: CheckedInitializerCoercion::None,
            origin,
        });
        if self
            .facts
            .expression_uses_by_owner
            .insert(owner, id)
            .is_some()
        {
            return Err(self.invariant(span, "duplicate checked view expression owner"));
        }
        Ok(id)
    }

    fn push_retained_expression(
        &mut self,
        owner: CheckedExprOwner,
        expr: &Expr,
        expected: &Type,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let id = CheckedExprUseId(self.facts.expression_uses.len() as u32);
        let analysis = self.analyses.remove(owner).ok_or_else(|| {
            self.invariant(
                span,
                "missing authoritative app-setting expression analysis",
            )
        })?;
        let analysis_metrics = analysis.metrics();
        self.facts.metrics.app_setting_analysis_passes += 1;
        self.facts.metrics.type_analysis_queries += analysis_metrics.queries;
        self.facts.metrics.type_analysis_nodes += analysis_metrics.nodes;
        self.facts.metrics.type_analysis_cache_hits += analysis_metrics.cache_hits;
        self.facts.metrics.type_scope_env_overlays += analysis_metrics.scoped_env_overlays;
        self.facts.metrics.type_scope_env_full_clones += analysis_metrics.scoped_env_full_clones;
        let inferred = analysis.type_of(expr).cloned().ok_or_else(|| {
            self.invariant(span, "missing retained app-setting expression root type")
        })?;
        let source = resolve_erased_type(&contextual_type(inferred, Some(expected)));
        let origin = self.origins.push(span, Some(parent));
        let lowering = ExpressionLowering {
            analysis: &analysis,
            owner: id,
            origin,
            span,
        };
        let root = self.lower_expr(expr, Some(&source), env, lowering)?;
        if self.facts.expressions[root.0 as usize].ty != source {
            return Err(self.invariant(
                span,
                "app-setting expression source type does not match its checked root",
            ));
        }
        self.facts.expression_uses.push(CheckedExprUse {
            owner,
            root,
            source,
            destination: expected.clone(),
            coercion: CheckedInitializerCoercion::None,
            origin,
        });
        if self
            .facts
            .expression_uses_by_owner
            .insert(owner, id)
            .is_some()
        {
            return Err(self.invariant(span, "duplicate checked app-setting expression owner"));
        }
        Ok(id)
    }

    fn push_local(
        &mut self,
        name: &str,
        ty: Type,
        owner: CheckedLocalOwner,
        span: &Span,
        parent: OriginId,
    ) -> CheckedLocalId {
        let id = CheckedLocalId(self.facts.locals.len() as u32);
        let origin = self.origins.push(span, Some(parent));
        self.facts.locals.push(CheckedLocal {
            name: name.to_owned(),
            ty,
            owner,
            origin,
        });
        self.facts.locals_by_owner.insert(owner, id);
        id
    }

    fn push_view_local(
        &mut self,
        name: &str,
        ty: Type,
        view: ViewId,
        role: CheckedViewLocalRole,
        span: &Span,
    ) -> CheckedLocalId {
        let parent = self.declarations.view(view).origin;
        self.push_view_local_with_parent(name, ty, view, role, span, parent)
    }

    fn push_view_local_with_parent(
        &mut self,
        name: &str,
        ty: Type,
        view: ViewId,
        role: CheckedViewLocalRole,
        span: &Span,
        parent: OriginId,
    ) -> CheckedLocalId {
        let id = CheckedLocalId(self.facts.locals.len() as u32);
        let origin = self.origins.push(span, Some(parent));
        let owner = CheckedLocalOwner::View { view, role };
        self.facts.locals.push(CheckedLocal {
            name: name.to_owned(),
            ty,
            owner,
            origin,
        });
        self.facts.locals_by_owner.insert(owner, id);
        id
    }

    fn lower_view_expression_tree(
        &mut self,
        node: &ViewNode,
        env: &dyn FactEnvironment,
    ) -> Result<(), Error> {
        let view = self.declarations.view_id(node.span()).ok_or_else(|| {
            self.invariant(node.span(), "view expression owner has no shared view ID")
        })?;
        let origin = self.declarations.view(view).origin;
        let flow = match node {
            ViewNode::If {
                condition,
                children,
                span,
            } => {
                let condition = self.push_view_expression(
                    CheckedExprOwner::View {
                        view,
                        role: CheckedViewExprRole::IfCondition,
                    },
                    condition,
                    Some(&Type::Bool),
                    env,
                    span,
                    origin,
                )?;
                for child in children {
                    self.lower_view_expression_tree(child, env)?;
                }
                CheckedViewFlow::If { condition }
            }
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                let items_use = self.push_view_expression(
                    CheckedExprOwner::View {
                        view,
                        role: CheckedViewExprRole::ForItems,
                    },
                    items,
                    None,
                    env,
                    span,
                    origin,
                )?;
                let Type::List(item_ty) = self.facts.expression_use(items_use).source.clone()
                else {
                    return Err(self.invariant(span, "checked for items are not a list"));
                };
                let local = self.push_view_local(
                    item,
                    *item_ty.clone(),
                    view,
                    CheckedViewLocalRole::ForItem,
                    span,
                );
                let scoped = LayeredFactEnv {
                    base: env,
                    name: item.clone(),
                    value: (CheckedPathRoot::Local(local), *item_ty),
                };
                self.facts.metrics.scope_env_overlays += 1;
                for child in children {
                    self.lower_view_expression_tree(child, &scoped)?;
                }
                CheckedViewFlow::For {
                    items: items_use,
                    item: local,
                }
            }
            ViewNode::Match { value, arms, span } => {
                let value_use = self.push_view_expression(
                    CheckedExprOwner::View {
                        view,
                        role: CheckedViewExprRole::MatchValue,
                    },
                    value,
                    None,
                    env,
                    span,
                    origin,
                )?;
                let value_ty = self.facts.expression_use(value_use).source.clone();
                let mut checked_arms = Vec::with_capacity(arms.len());
                for (index, arm) in arms.iter().enumerate() {
                    let arm_origin = self.origins.push(&arm.span, Some(origin));
                    let (pattern, binding) = self.resolve_match_pattern(
                        &value_ty,
                        &arm.pattern,
                        view,
                        index as u32,
                        &arm.span,
                        arm_origin,
                    )?;
                    if let Some(local) = binding {
                        let checked = self.facts.local(local);
                        let scoped = LayeredFactEnv {
                            base: env,
                            name: checked.name.clone(),
                            value: (CheckedPathRoot::Local(local), checked.ty.clone()),
                        };
                        self.facts.metrics.scope_env_overlays += 1;
                        for child in &arm.children {
                            self.lower_view_expression_tree(child, &scoped)?;
                        }
                    } else {
                        for child in &arm.children {
                            self.lower_view_expression_tree(child, env)?;
                        }
                    }
                    checked_arms.push(CheckedMatchArm {
                        pattern,
                        binding,
                        origin: arm_origin,
                    });
                }
                CheckedViewFlow::Match {
                    value: value_use,
                    arms: checked_arms,
                }
            }
            ViewNode::KeyedColumn {
                item,
                items,
                key,
                child,
                span,
                ..
            } => {
                let items_use = self.push_view_expression(
                    CheckedExprOwner::View {
                        view,
                        role: CheckedViewExprRole::KeyedItems,
                    },
                    items,
                    None,
                    env,
                    span,
                    origin,
                )?;
                let Type::List(item_ty) = self.facts.expression_use(items_use).source.clone()
                else {
                    return Err(self.invariant(span, "checked keyed items are not a list"));
                };
                let local = self.push_view_local(
                    item,
                    *item_ty.clone(),
                    view,
                    CheckedViewLocalRole::KeyedItem,
                    span,
                );
                let scoped = LayeredFactEnv {
                    base: env,
                    name: item.clone(),
                    value: (CheckedPathRoot::Local(local), *item_ty),
                };
                self.facts.metrics.scope_env_overlays += 1;
                let key_use = self.push_view_expression(
                    CheckedExprOwner::View {
                        view,
                        role: CheckedViewExprRole::KeyedKey,
                    },
                    key,
                    None,
                    &scoped,
                    span,
                    origin,
                )?;
                self.lower_view_expression_tree(child, &scoped)?;
                CheckedViewFlow::Keyed {
                    items: items_use,
                    key: key_use,
                    item: local,
                }
            }
            ViewNode::Lazy {
                dependency,
                binding,
                child,
                span,
                ..
            } => {
                let dependency_use = self.push_view_expression(
                    CheckedExprOwner::View {
                        view,
                        role: CheckedViewExprRole::LazyDependency,
                    },
                    dependency,
                    None,
                    env,
                    span,
                    origin,
                )?;
                let ty = self.facts.expression_use(dependency_use).source.clone();
                let local = self.push_view_local(
                    binding,
                    ty.clone(),
                    view,
                    CheckedViewLocalRole::LazyDependency,
                    span,
                );
                let empty = FactEnv::default();
                let scoped = LayeredFactEnv {
                    base: &empty,
                    name: binding.clone(),
                    value: (CheckedPathRoot::Local(local), ty),
                };
                self.facts.metrics.scope_env_overlays += 1;
                self.lower_view_expression_tree(child, &scoped)?;
                CheckedViewFlow::Lazy {
                    dependency: dependency_use,
                    binding: local,
                }
            }
            ViewNode::Table {
                item,
                rows,
                columns,
                span,
                ..
            } => {
                let rows_use = self.push_view_expression(
                    CheckedExprOwner::View {
                        view,
                        role: CheckedViewExprRole::TableRows,
                    },
                    rows,
                    None,
                    env,
                    span,
                    origin,
                )?;
                let Type::List(row_ty) = self.facts.expression_use(rows_use).source.clone() else {
                    return Err(self.invariant(span, "checked table rows are not a list"));
                };
                let local = self.push_view_local(
                    item,
                    *row_ty.clone(),
                    view,
                    CheckedViewLocalRole::TableRow,
                    span,
                );
                let scoped = LayeredFactEnv {
                    base: env,
                    name: item.clone(),
                    value: (CheckedPathRoot::Local(local), *row_ty),
                };
                self.facts.metrics.scope_env_overlays += 1;
                for column in columns {
                    self.lower_view_expression_tree(&column.header, env)?;
                    self.lower_view_expression_tree(&column.cell, &scoped)?;
                }
                CheckedViewFlow::Table {
                    rows: rows_use,
                    item: local,
                }
            }
            ViewNode::PaneGrid {
                panes,
                templates,
                span: _,
                ..
            } => {
                let mut static_maximized = Vec::with_capacity(panes.len());
                for (index, pane) in panes.iter().enumerate() {
                    if let Some(name) = &pane.maximized {
                        let local = self.push_view_local(
                            name,
                            Type::Bool,
                            view,
                            CheckedViewLocalRole::PaneMaximized(index as u32),
                            &pane.span,
                        );
                        let scoped = LayeredFactEnv {
                            base: env,
                            name: name.clone(),
                            value: (CheckedPathRoot::Local(local), Type::Bool),
                        };
                        self.facts.metrics.scope_env_overlays += 1;
                        for child in pane.nodes() {
                            self.lower_view_expression_tree(child, &scoped)?;
                        }
                        static_maximized.push(Some(local));
                    } else {
                        for child in pane.nodes() {
                            self.lower_view_expression_tree(child, env)?;
                        }
                        static_maximized.push(None);
                    }
                }
                let mut checked_templates = Vec::with_capacity(templates.len());
                for (index, template) in templates.iter().enumerate() {
                    let (_, list_ty) = env.get(&template.items).ok_or_else(|| {
                        self.invariant(&template.span, "pane template list has no checked path")
                    })?;
                    let Type::List(item_ty) = list_ty else {
                        return Err(self.invariant(
                            &template.span,
                            "pane template checked path is not a list",
                        ));
                    };
                    let item_local = self.push_view_local(
                        &template.item,
                        item_ty.as_ref().clone(),
                        view,
                        CheckedViewLocalRole::PaneTemplateItem(index as u32),
                        &template.span,
                    );
                    let item_scoped = LayeredFactEnv {
                        base: env,
                        name: template.item.clone(),
                        value: (CheckedPathRoot::Local(item_local), item_ty.as_ref().clone()),
                    };
                    self.facts.metrics.scope_env_overlays += 1;
                    let key = self.push_view_expression(
                        CheckedExprOwner::View {
                            view,
                            role: CheckedViewExprRole::PaneTemplateKey(index as u32),
                        },
                        &template.key,
                        None,
                        &item_scoped,
                        &template.span,
                        origin,
                    )?;
                    let maximized = template.pane.maximized.as_ref().map(|name| {
                        self.push_view_local(
                            name,
                            Type::Bool,
                            view,
                            CheckedViewLocalRole::PaneTemplateMaximized(index as u32),
                            &template.pane.span,
                        )
                    });
                    if let Some(maximized) = maximized {
                        let scoped = LayeredFactEnv {
                            base: &item_scoped,
                            name: template.pane.maximized.clone().unwrap(),
                            value: (CheckedPathRoot::Local(maximized), Type::Bool),
                        };
                        self.facts.metrics.scope_env_overlays += 1;
                        for child in template.pane.nodes() {
                            self.lower_view_expression_tree(child, &scoped)?;
                        }
                    } else {
                        for child in template.pane.nodes() {
                            self.lower_view_expression_tree(child, &item_scoped)?;
                        }
                    }
                    checked_templates.push(CheckedPaneTemplate {
                        key,
                        item: item_local,
                        maximized,
                    });
                }
                CheckedViewFlow::PaneGrid {
                    static_maximized,
                    templates: checked_templates,
                }
            }
            ViewNode::Responsive { content, span, .. } => match content {
                ResponsiveContent::Breakpoint {
                    breakpoint,
                    narrow,
                    wide,
                } => {
                    let breakpoint = self.push_view_expression(
                        CheckedExprOwner::View {
                            view,
                            role: CheckedViewExprRole::ResponsiveBreakpoint,
                        },
                        breakpoint,
                        Some(&Type::F64),
                        env,
                        span,
                        origin,
                    )?;
                    self.lower_view_expression_tree(narrow, env)?;
                    self.lower_view_expression_tree(wide, env)?;
                    CheckedViewFlow::ResponsiveBreakpoint { breakpoint }
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let width_local = self.push_view_local(
                        width,
                        Type::F64,
                        view,
                        CheckedViewLocalRole::ResponsiveWidth,
                        span,
                    );
                    let width_scoped = LayeredFactEnv {
                        base: env,
                        name: width.clone(),
                        value: (CheckedPathRoot::Local(width_local), Type::F64),
                    };
                    let height_local = self.push_view_local(
                        height,
                        Type::F64,
                        view,
                        CheckedViewLocalRole::ResponsiveHeight,
                        span,
                    );
                    let scoped = LayeredFactEnv {
                        base: &width_scoped,
                        name: height.clone(),
                        value: (CheckedPathRoot::Local(height_local), Type::F64),
                    };
                    self.facts.metrics.scope_env_overlays += 2;
                    self.lower_view_expression_tree(content, &scoped)?;
                    CheckedViewFlow::ResponsiveSize {
                        width: width_local,
                        height: height_local,
                    }
                }
            },
            ViewNode::Component {
                name,
                args,
                slots,
                span,
                ..
            } => {
                let component = self
                    .declarations
                    .component_id(name)
                    .ok_or_else(|| self.invariant(span, "component call has no declaration"))?;
                let call = self
                    .declarations
                    .component_call_id(view)
                    .ok_or_else(|| self.invariant(span, "component view has no call ID"))?;
                let source_component = &self.document.components[component.0 as usize];
                let mut supplied = HashMap::new();
                for arg in args {
                    let index = source_component
                        .params
                        .iter()
                        .position(|param| param.name == arg.name)
                        .ok_or_else(|| {
                            self.invariant(span, "component argument has no parameter")
                        })?;
                    let param = self.declarations.component_param(component, index).id;
                    let expected = source_component.params[index].ty.clone();
                    let expression = self.push_view_expression(
                        CheckedExprOwner::ComponentArgument { call, param },
                        &arg.value,
                        Some(&expected),
                        env,
                        span,
                        origin,
                    )?;
                    if supplied.insert(param, expression).is_some() {
                        return Err(self.invariant(span, "duplicate checked component argument"));
                    }
                }
                for (index, param) in source_component.params.iter().enumerate() {
                    let param_id = self.declarations.component_param(component, index).id;
                    let source = if let Some(expression) = supplied.remove(&param_id) {
                        CheckedComponentArgumentSource::Supplied(expression)
                    } else {
                        let default = self
                            .facts
                            .value_by_ref(CheckedValueRef::ComponentParam(param_id))
                            .initializer
                            .ok_or_else(|| {
                                self.invariant(
                                    span,
                                    format!("required prop `{}` has no checked source", param.name),
                                )
                            })?;
                        CheckedComponentArgumentSource::Default(default)
                    };
                    if self
                        .facts
                        .component_argument_sources
                        .insert((call, param_id), source)
                        .is_some()
                    {
                        return Err(
                            self.invariant(span, "duplicate checked component argument source")
                        );
                    }
                }
                for slot in slots {
                    self.lower_view_expression_tree(&slot.content, env)?;
                }
                CheckedViewFlow::None
            }
            _ => {
                for child in crate::hir::view_children(node) {
                    self.lower_view_expression_tree(child, env)?;
                }
                CheckedViewFlow::None
            }
        };
        self.facts.views[view.0 as usize].flow = flow;
        Ok(())
    }

    fn resolve_match_pattern(
        &mut self,
        value_ty: &Type,
        pattern: &MatchPattern,
        view: ViewId,
        arm: u32,
        span: &Span,
        arm_origin: OriginId,
    ) -> Result<(CheckedMatchPattern, Option<CheckedLocalId>), Error> {
        let resolved = match (value_ty, pattern) {
            (Type::Option(inner), MatchPattern::Some(name)) => {
                let local = self.push_view_local_with_parent(
                    name,
                    inner.as_ref().clone(),
                    view,
                    CheckedViewLocalRole::MatchPayload(arm),
                    span,
                    arm_origin,
                );
                (CheckedMatchPattern::Some, Some(local))
            }
            (Type::Option(_), MatchPattern::None) => (CheckedMatchPattern::None, None),
            (Type::Result(output, _), MatchPattern::Ok(name)) => {
                let local = self.push_view_local_with_parent(
                    name,
                    output.as_ref().clone(),
                    view,
                    CheckedViewLocalRole::MatchPayload(arm),
                    span,
                    arm_origin,
                );
                (CheckedMatchPattern::Ok, Some(local))
            }
            (Type::Result(_, error), MatchPattern::Err(name)) => {
                let local = self.push_view_local_with_parent(
                    name,
                    error.as_ref().clone(),
                    view,
                    CheckedViewLocalRole::MatchPayload(arm),
                    span,
                    arm_origin,
                );
                (CheckedMatchPattern::Err, Some(local))
            }
            (
                Type::Named(enum_name),
                MatchPattern::Enum {
                    enum_name: actual,
                    variant,
                    binding,
                },
            ) if enum_name == actual => {
                let owner = self
                    .declarations
                    .enum_decl_by_name(enum_name)
                    .ok_or_else(|| self.invariant(span, "match enum has no declaration"))?;
                let variant = self
                    .declarations
                    .enum_variant(owner.declaration.id, variant)
                    .ok_or_else(|| self.invariant(span, "match variant has no declaration"))?;
                let local = match (&variant.payload, binding) {
                    (Some(ty), Some(name)) => Some(self.push_view_local_with_parent(
                        name,
                        ty.clone(),
                        view,
                        CheckedViewLocalRole::MatchPayload(arm),
                        span,
                        arm_origin,
                    )),
                    (None, None) => None,
                    _ => return Err(self.invariant(span, "match payload binding diverged")),
                };
                (CheckedMatchPattern::Enum(variant.declaration.id), local)
            }
            (
                Type::Palette(contract),
                MatchPattern::Enum {
                    enum_name,
                    variant,
                    binding: None,
                },
            ) if contract == enum_name => {
                let palette = self
                    .declarations
                    .palette_id(variant)
                    .ok_or_else(|| self.invariant(span, "match palette has no declaration"))?;
                (CheckedMatchPattern::Palette(palette), None)
            }
            (_, MatchPattern::Wildcard) => (CheckedMatchPattern::Wildcard, None),
            _ => {
                return Err(self.invariant(span, "checked match pattern does not match value type"));
            }
        };
        Ok(resolved)
    }

    fn value_id(
        &mut self,
        scope: ValueScope,
        name: &str,
        span: &Span,
    ) -> Result<CheckedValueId, Error> {
        self.facts.metrics.declaration_lookups += 1;
        self.values_by_scope
            .get(&scope)
            .and_then(|values| values.get(name))
            .copied()
            .ok_or_else(|| self.invariant(span, format!("missing checked value `{name}`")))
    }

    fn push_expression_use(
        &mut self,
        owner: CheckedValueId,
        expr: &Expr,
        expected: &Type,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let value = &self.facts.values[owner.0 as usize];
        let origin = value.origin;
        let owner_ref = value.id;
        let id = CheckedExprUseId(self.facts.expression_uses.len() as u32);
        let analysis = self
            .analyses
            .remove(CheckedExprOwner::Value(owner_ref))
            .ok_or_else(|| self.invariant(span, "missing authoritative initializer analysis"))?;
        let analysis_metrics = analysis.metrics();
        self.facts.metrics.initializer_analysis_passes += 1;
        self.facts.metrics.type_analysis_queries += analysis_metrics.queries;
        self.facts.metrics.type_analysis_nodes += analysis_metrics.nodes;
        self.facts.metrics.type_analysis_cache_hits += analysis_metrics.cache_hits;
        self.facts.metrics.type_scope_env_overlays += analysis_metrics.scoped_env_overlays;
        self.facts.metrics.type_scope_env_full_clones += analysis_metrics.scoped_env_full_clones;
        let lowering = ExpressionLowering {
            analysis: &analysis,
            owner: id,
            origin,
            span,
        };
        let inferred = analysis
            .type_of(expr)
            .ok_or_else(|| self.invariant(span, "missing initializer expression type"))?;
        let (source_context, coercion) = initializer_source_context(inferred, expected);
        let source = resolve_erased_type(&contextual_type(inferred.clone(), Some(&source_context)));
        let root = self.lower_expr(expr, Some(&source_context), env, lowering)?;
        if self.facts.expressions[root.0 as usize].ty != source {
            return Err(self.invariant(
                span,
                "initializer source type does not match its checked expression root",
            ));
        }
        debug_assert_eq!(id.0 as usize, self.facts.expression_uses.len());
        self.facts.expression_uses.push(CheckedExprUse {
            owner: CheckedExprOwner::Value(owner_ref),
            root,
            source,
            destination: expected.clone(),
            coercion,
            origin,
        });
        if self
            .facts
            .expression_uses_by_owner
            .insert(CheckedExprOwner::Value(owner_ref), id)
            .is_some()
        {
            return Err(self.invariant(span, "duplicate checked initializer expression owner"));
        }
        self.facts.values[owner.0 as usize].initializer = Some(id);
        Ok(id)
    }

    fn lower_expr(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
        env: &dyn FactEnvironment,
        lowering: ExpressionLowering<'_>,
    ) -> Result<CheckedExprId, Error> {
        let inferred =
            lowering.analysis.type_of(expr).cloned().ok_or_else(|| {
                self.invariant(lowering.span, "missing post-order expression type")
            })?;
        let ty = resolve_erased_type(&contextual_type(inferred, expected));
        let kind =
            match expr {
                Expr::Bool(value) => CheckedExprKind::Bool(*value),
                Expr::I64(value) => CheckedExprKind::I64(*value),
                Expr::F64(value) => CheckedExprKind::F64(*value),
                Expr::Str(value) => CheckedExprKind::Str(value.clone()),
                Expr::Bytes(value) => CheckedExprKind::Bytes(value.clone()),
                Expr::EmptyList => CheckedExprKind::List(Vec::new()),
                Expr::List(values) => {
                    let expected_element = match &ty {
                        Type::List(inner) => Some(inner.as_ref()),
                        _ => None,
                    };
                    let children = values
                        .iter()
                        .map(|value| self.lower_expr(value, expected_element, env, lowering))
                        .collect::<Result<Vec<_>, _>>()?;
                    CheckedExprKind::List(children)
                }
                Expr::None => CheckedExprKind::None,
                Expr::Path(path) => self.lower_path(path, env, lowering.span)?,
                Expr::Call { name, args } => {
                    if unqualified_name(name) == "provided" {
                        let [Expr::Path(path)] = args.as_slice() else {
                            return Err(self.invariant(
                                lowering.span,
                                "checked provided call has malformed slot argument",
                            ));
                        };
                        let [slot] = path.as_slice() else {
                            return Err(self.invariant(
                                lowering.span,
                                "checked provided call has malformed slot path",
                            ));
                        };
                        let slot = env.slot(unqualified_name(slot)).ok_or_else(|| {
                            self.invariant(
                                lowering.span,
                                "checked provided call has no resolved component slot",
                            )
                        })?;
                        let id = CheckedExprId(self.facts.expressions.len() as u32);
                        self.facts.expressions.push(CheckedExpr {
                            ty,
                            kind: CheckedExprKind::SlotProvided(slot),
                            origin: lowering.origin,
                        });
                        return Ok(id);
                    }
                    let target = self.resolve_call_target(name, lowering.span)?;
                    let arguments = self.lower_call_arguments(&target, args, &ty, env, lowering)?;
                    CheckedExprKind::Call { target, arguments }
                }
                Expr::Unary { op, value } => {
                    let input = lowering.analysis.type_of(value).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "missing unary operand type")
                    })?;
                    let operator = match op {
                        UnaryOp::Not => CheckedUnaryOperator::BooleanNot,
                        UnaryOp::Neg => CheckedUnaryOperator::NumericNegation(input.clone()),
                    };
                    let value = self.lower_expr(value, Some(&input), env, lowering)?;
                    CheckedExprKind::Unary { operator, value }
                }
                Expr::Binary { left, op, right } => {
                    let left_ty = lowering.analysis.type_of(left).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "missing left operand type")
                    })?;
                    let right_ty = lowering.analysis.type_of(right).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "missing right operand type")
                    })?;
                    let operator = match op {
                        BinaryOp::And | BinaryOp::Or => CheckedBinaryOperator::Boolean(*op),
                        BinaryOp::Eq | BinaryOp::NotEq => CheckedBinaryOperator::Equality {
                            op: *op,
                            operand: compatible_operand(&left_ty, &right_ty),
                        },
                        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                            CheckedBinaryOperator::Ordering {
                                op: *op,
                                operand: compatible_operand(&left_ty, &right_ty),
                            }
                        }
                        BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem => CheckedBinaryOperator::Arithmetic {
                            op: *op,
                            operand: compatible_operand(&left_ty, &right_ty),
                        },
                    };
                    let operand = match &operator {
                        CheckedBinaryOperator::Boolean(_) => Type::Bool,
                        CheckedBinaryOperator::Equality { operand, .. }
                        | CheckedBinaryOperator::Ordering { operand, .. }
                        | CheckedBinaryOperator::Arithmetic { operand, .. } => operand.clone(),
                    };
                    let left = self.lower_expr(left, Some(&operand), env, lowering)?;
                    let right = self.lower_expr(right, Some(&operand), env, lowering)?;
                    CheckedExprKind::Binary {
                        operator,
                        left,
                        right,
                    }
                }
            };
        let id = CheckedExprId(self.facts.expressions.len() as u32);
        self.facts.expressions.push(CheckedExpr {
            ty,
            kind,
            origin: lowering.origin,
        });
        Ok(id)
    }

    fn lower_path(
        &mut self,
        path: &[String],
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedExprKind, Error> {
        if let [contract, palette] = path
            && self
                .document
                .theme_contract
                .as_ref()
                .is_some_and(|item| item.name == *contract)
        {
            self.facts.metrics.declaration_lookups += 1;
            let id = self.declarations.palette_id(palette).ok_or_else(|| {
                self.invariant(span, format!("missing checked palette `{palette}`"))
            })?;
            return Ok(CheckedExprKind::Path {
                root: CheckedPathRoot::Palette(id),
                projections: Vec::new(),
            });
        }
        if let [enum_name, variant_name] = path
            && let Some(variant) = self.enum_variant(enum_name, variant_name)
        {
            return Ok(CheckedExprKind::Path {
                root: CheckedPathRoot::EnumVariant(variant),
                projections: Vec::new(),
            });
        }
        let Some(name) = path.first() else {
            return Err(self.invariant(span, "checked path is empty"));
        };
        self.facts.metrics.declaration_lookups += 1;
        let (root, mut input) = env
            .get(name)
            .cloned()
            .ok_or_else(|| self.invariant(span, format!("missing checked value `{name}`")))?;
        let mut projections = Vec::with_capacity(path.len().saturating_sub(1));
        for field in &path[1..] {
            let (output, kind) = if let Type::Named(struct_name) = &input {
                self.facts.metrics.declaration_lookups += 1;
                let owner = self
                    .declarations
                    .struct_decl_by_name(struct_name)
                    .ok_or_else(|| {
                        self.invariant(span, format!("missing checked struct `{struct_name}`"))
                    })?;
                let field_id = self
                    .declarations
                    .struct_field(owner.declaration.id, field)
                    .ok_or_else(|| {
                        self.invariant(
                            span,
                            format!("missing checked field `{struct_name}.{field}`"),
                        )
                    })?;
                (
                    field_id.ty.clone(),
                    CheckedProjectionKind::Struct(field_id.declaration.id),
                )
            } else if matches!(&input, Type::Option(inner) if **inner == Type::WidgetTarget) {
                (
                    field_type(&input, field, self.document, span)?,
                    CheckedProjectionKind::OptionalWidgetTarget,
                )
            } else {
                (
                    field_type(&input, field, self.document, span)?,
                    CheckedProjectionKind::Native,
                )
            };
            projections.push(CheckedProjection {
                field: field.clone(),
                input,
                output: output.clone(),
                kind,
            });
            input = output;
        }
        Ok(CheckedExprKind::Path { root, projections })
    }

    fn resolve_call_target(
        &mut self,
        name: &str,
        _span: &Span,
    ) -> Result<CheckedCallTarget, Error> {
        if let Some((enum_name, variant_name)) = name.split_once('.')
            && let Some(variant) = self.enum_variant(enum_name, variant_name)
        {
            return Ok(CheckedCallTarget::EnumVariant(variant));
        }
        self.facts.metrics.declaration_lookups += 1;
        if let Some(declaration) = self.declarations.extern_decl_by_name(name)
            && declaration.kind == ExternKind::Sync
        {
            return Ok(CheckedCallTarget::Extern(declaration.declaration.id));
        }
        let name = unqualified_name(name);
        debug_assert_ne!(name, "provided");
        self.facts.metrics.builtin_intern_lookups += 1;
        let id = if let Some(id) = self.builtins_by_name.get(name).copied() {
            id
        } else {
            let id = CheckedBuiltinId(self.facts.builtins.len() as u32);
            self.facts.builtins.push(name.to_owned());
            self.builtins_by_name.insert(name.to_owned(), id);
            id
        };
        Ok(CheckedCallTarget::Builtin(id))
    }

    fn enum_variant(&mut self, enum_name: &str, variant_name: &str) -> Option<EnumVariantId> {
        self.facts.metrics.declaration_lookups += 1;
        let owner = self.declarations.enum_decl_by_name(enum_name)?;
        self.declarations
            .enum_variant(owner.declaration.id, variant_name)
            .map(|variant| variant.declaration.id)
    }

    fn lower_call_arguments(
        &mut self,
        target: &CheckedCallTarget,
        args: &[Expr],
        output: &Type,
        env: &dyn FactEnvironment,
        lowering: ExpressionLowering<'_>,
    ) -> Result<Vec<CheckedCallArgument>, Error> {
        let contexts =
            self.call_argument_contexts(target, output, args, lowering.analysis, lowering.span)?;
        if contexts.len() != args.len() {
            return Err(self.invariant(
                lowering.span,
                "checked call argument context count does not match its arguments",
            ));
        }
        let mut bindings = HashMap::<usize, (String, CheckedLocalId)>::new();
        let mut arguments = Vec::with_capacity(args.len());
        for (index, (argument, context)) in args.iter().zip(contexts).enumerate() {
            match context {
                BuiltinArgumentContext::Value { expected } => {
                    arguments.push(CheckedCallArgument::Value(self.lower_expr(
                        argument,
                        expected.as_ref(),
                        env,
                        lowering,
                    )?));
                }
                BuiltinArgumentContext::Binding { ty, body } => {
                    let Expr::Path(path) = argument else {
                        return Err(
                            self.invariant(lowering.span, "checked builtin binding is not a path")
                        );
                    };
                    let [name] = path.as_slice() else {
                        return Err(self.invariant(
                            lowering.span,
                            "checked builtin binding is not a local name",
                        ));
                    };
                    let id = CheckedLocalId(self.facts.locals.len() as u32);
                    let owner = CheckedLocalOwner::ExpressionBinding {
                        expression: lowering.owner,
                        body_argument: body,
                    };
                    self.facts.locals.push(CheckedLocal {
                        name: name.clone(),
                        ty,
                        owner,
                        origin: lowering.origin,
                    });
                    self.facts.locals_by_owner.insert(owner, id);
                    bindings.insert(index, (name.clone(), id));
                    arguments.push(CheckedCallArgument::Binding(id));
                }
                BuiltinArgumentContext::ScopedValue { expected, binding } => {
                    let (name, local) = bindings.get(&binding).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "checked builtin body has no binding fact")
                    })?;
                    let ty = self.facts.locals[local.0 as usize].ty.clone();
                    let scoped = LayeredFactEnv {
                        base: env,
                        name,
                        value: (CheckedPathRoot::Local(local), ty),
                    };
                    self.facts.metrics.scope_env_overlays += 1;
                    arguments.push(CheckedCallArgument::Value(self.lower_expr(
                        argument,
                        expected.as_ref(),
                        &scoped,
                        lowering,
                    )?));
                }
            }
        }
        Ok(arguments)
    }

    fn call_argument_contexts(
        &self,
        target: &CheckedCallTarget,
        output: &Type,
        args: &[Expr],
        analysis: &ExprTypeAnalysis,
        span: &Span,
    ) -> Result<Vec<BuiltinArgumentContext>, Error> {
        Ok(match target {
            CheckedCallTarget::Extern(id) => self
                .declarations
                .extern_decl(*id)
                .params
                .iter()
                .map(|(_, ty)| BuiltinArgumentContext::Value {
                    expected: Some(ty.clone()),
                })
                .collect(),
            CheckedCallTarget::EnumVariant(id) => self
                .declarations
                .enum_variant_decl(*id)
                .payload
                .iter()
                .cloned()
                .map(|ty| BuiltinArgumentContext::Value { expected: Some(ty) })
                .collect(),
            CheckedCallTarget::Builtin(id) => {
                let name = self.facts.builtins[id.0 as usize].as_str();
                if let Some(builtin) = ContextualBuiltin::from_name(name) {
                    let inferred = args
                        .iter()
                        .map(|argument| {
                            analysis.type_of(argument).cloned().unwrap_or(Type::Unknown)
                        })
                        .collect::<Vec<_>>();
                    builtin
                        .argument_contexts(output, &inferred)
                        .map_err(|message| self.invariant(span, message))?
                } else {
                    args.iter()
                        .map(|_| BuiltinArgumentContext::Value { expected: None })
                        .collect()
                }
            }
        })
    }

    fn index_views(&mut self) -> Result<(), Error> {
        for (index, component) in self.document.components.iter().enumerate() {
            let declaration = self.declarations.component(index);
            self.index_view(
                &component.root,
                None,
                CheckedViewScope::Component(declaration.id),
            )?;
        }
        self.index_view(&self.document.view, None, CheckedViewScope::App)?;
        for (index, test) in self.document.tests.iter().enumerate() {
            if let Some(mount) = &test.mount {
                self.index_view(mount, None, CheckedViewScope::Test(TestId(index as u32)))?;
            }
        }
        Ok(())
    }

    fn index_view(
        &mut self,
        node: &ViewNode,
        parent: Option<ViewId>,
        scope: CheckedViewScope,
    ) -> Result<ViewId, Error> {
        let id = self.declarations.view_id(node.span()).ok_or_else(|| {
            self.invariant(node.span(), "checked view has no shared declaration ID")
        })?;
        if id.0 as usize != self.facts.views.len() {
            return Err(self.invariant(node.span(), "checked view arena order diverged"));
        }
        let origin = self.declarations.view(id).origin;
        self.facts.views.push(CheckedView {
            id,
            kind: crate::hir::view_kind(node),
            scope,
            parent,
            children: Vec::new(),
            flow: CheckedViewFlow::None,
            origin,
        });
        let children = crate::hir::view_children(node)
            .into_iter()
            .map(|child| self.index_view(child, Some(id), scope))
            .collect::<Result<Vec<_>, _>>()?;
        self.facts.views[id.0 as usize].children = children;
        Ok(id)
    }

    fn invariant(&self, span: &Span, message: impl Into<String>) -> Error {
        Error::new("E196", span, message)
    }
}

fn initializer_source_context(
    inferred: &Type,
    destination: &Type,
) -> (Type, CheckedInitializerCoercion) {
    match (inferred, destination) {
        (Type::List(_), Type::Combo(element)) => (
            Type::List(element.clone()),
            CheckedInitializerCoercion::ListToCombo {
                element: element.as_ref().clone(),
            },
        ),
        (inferred, Type::Animation(value)) if compatible(inferred, value) => (
            value.as_ref().clone(),
            CheckedInitializerCoercion::ValueToAnimation {
                value: value.as_ref().clone(),
            },
        ),
        (Type::Str, Type::Markdown) => (Type::Str, CheckedInitializerCoercion::StrToMarkdown),
        (Type::Str, Type::Editor) => (Type::Str, CheckedInitializerCoercion::StrToEditor),
        _ => (destination.clone(), CheckedInitializerCoercion::None),
    }
}

fn contextual_type(inferred: Type, expected: Option<&Type>) -> Type {
    match (inferred, expected) {
        (Type::Unknown, Some(expected)) => expected.clone(),
        (Type::List(actual), Some(Type::List(expected))) => {
            Type::List(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (Type::Option(actual), Some(Type::Option(expected))) => {
            Type::Option(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (Type::Result(actual_output, actual_error), Some(Type::Result(output, error))) => {
            Type::Result(
                Box::new(contextual_type(*actual_output, Some(output.as_ref()))),
                Box::new(contextual_type(*actual_error, Some(error.as_ref()))),
            )
        }
        (Type::Combo(actual), Some(Type::Combo(expected))) => {
            Type::Combo(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (Type::Animation(actual), Some(Type::Animation(expected))) => {
            Type::Animation(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (actual, _) => actual,
    }
}

fn contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            contains_unknown(inner)
        }
        Type::Result(output, error) => contains_unknown(output) || contains_unknown(error),
        _ => false,
    }
}

fn compatible_operand(left: &Type, right: &Type) -> Type {
    resolve_erased_type(&unify_type_evidence(left, right))
}

#[cfg(test)]
impl CheckedFacts {
    fn structural_snapshot(&self) -> String {
        use std::fmt::Write as _;

        let mut output = String::new();
        for (index, value) in self.values.iter().enumerate() {
            writeln!(
                output,
                "value v{index} {:?} {}:{:?} init={:?} origin=o{}",
                value.id, value.name, value.ty, value.initializer, value.origin.0
            )
            .unwrap();
        }
        for (index, expression_use) in self.expression_uses.iter().enumerate() {
            writeln!(
                output,
                "use u{index} {:?} root=e{} source={:?} destination={:?} coercion={:?} origin=o{}",
                expression_use.owner,
                expression_use.root.0,
                expression_use.source,
                expression_use.destination,
                expression_use.coercion,
                expression_use.origin.0
            )
            .unwrap();
        }
        for (index, local) in self.locals.iter().enumerate() {
            writeln!(
                output,
                "local l{index} {}:{:?} owner={:?} origin=o{}",
                local.name, local.ty, local.owner, local.origin.0
            )
            .unwrap();
        }
        for (index, expression) in self.expressions.iter().enumerate() {
            let kind = match &expression.kind {
                CheckedExprKind::Bool(value) => format!("bool {value}"),
                CheckedExprKind::I64(value) => format!("i64 {value}"),
                CheckedExprKind::F64(value) => format!("f64 {value}"),
                CheckedExprKind::Str(value) => format!("str {value:?}"),
                CheckedExprKind::Bytes(value) => format!("bytes {value:?}"),
                CheckedExprKind::List(values) => format!("list {values:?}"),
                CheckedExprKind::None => "none".into(),
                CheckedExprKind::SlotProvided(slot) => format!("slot-provided {slot:?}"),
                CheckedExprKind::Path { root, projections } => {
                    format!("path {root:?} {projections:?}")
                }
                CheckedExprKind::Call { target, arguments } => match target {
                    CheckedCallTarget::Builtin(id) => format!(
                        "call builtin:{} {}",
                        self.builtins[id.0 as usize],
                        format_call_arguments(arguments)
                    ),
                    _ => format!("call {target:?} {}", format_call_arguments(arguments)),
                },
                CheckedExprKind::Unary { operator, value } => {
                    format!("unary {operator:?} e{}", value.0)
                }
                CheckedExprKind::Binary {
                    operator,
                    left,
                    right,
                } => format!("binary {operator:?} e{} e{}", left.0, right.0),
            };
            writeln!(
                output,
                "expr e{index} {kind} : {:?} origin=o{}",
                expression.ty, expression.origin.0
            )
            .unwrap();
        }
        for (index, view) in self.views.iter().enumerate() {
            writeln!(
                output,
                "view w{index} {} {:?} parent={:?} children={:?} flow={:?} origin=o{}",
                view.kind, view.scope, view.parent, view.children, view.flow, view.origin.0
            )
            .unwrap();
        }
        output
    }
}

#[cfg(test)]
fn format_call_arguments(arguments: &[CheckedCallArgument]) -> String {
    let values = arguments
        .iter()
        .map(|argument| match argument {
            CheckedCallArgument::Value(id) => format!("CheckedExprId({})", id.0),
            CheckedCallArgument::Binding(id) => format!("CheckedLocalId({})", id.0),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, analyze_file, lower};
    use std::fmt::Write as _;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const THEME: &str = "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";

    #[test]
    fn snapshots_resolved_expression_and_owner_facts() {
        let source = format!(
            r#"app Facts
extern crate::backend
  User(name:str)
  sync load_user(seed:i64) -> User
enum Mode
  idle
  active(str)
{THEME}state
  user:User = load_user(1)
  color:color = color.rgb(0.25, 0.5, 0.75)
  mode:Mode = Mode.idle
derived
  name = user.name
  visible = color.r > 0.1 && name != ""
component Card(label:str="Card")
  state
    open = false
  col
    text label
    if open
      text "Open"
view
  col
    Card
    text name
"#
        );
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();

        assert_eq!(
            facts.structural_snapshot(),
            r#"value v0 AppState(AppStateId(0)) user:Named("User") init=Some(CheckedExprUseId(0)) origin=o0
value v1 AppState(AppStateId(1)) color:Color init=Some(CheckedExprUseId(1)) origin=o1
value v2 AppState(AppStateId(2)) mode:Named("Mode") init=Some(CheckedExprUseId(2)) origin=o2
value v3 Derived(DerivedId(0)) name:Str init=Some(CheckedExprUseId(3)) origin=o3
value v4 Derived(DerivedId(1)) visible:Bool init=Some(CheckedExprUseId(4)) origin=o4
value v5 ComponentParam(ComponentParamId { component: ComponentId(0), index: 0 }) label:Str init=Some(CheckedExprUseId(5)) origin=o6
value v6 ComponentState(ComponentStateId { component: ComponentId(0), index: 0 }) open:Bool init=Some(CheckedExprUseId(6)) origin=o7
use u0 Value(AppState(AppStateId(0))) root=e1 source=Named("User") destination=Named("User") coercion=None origin=o0
use u1 Value(AppState(AppStateId(1))) root=e5 source=Color destination=Color coercion=None origin=o1
use u2 Value(AppState(AppStateId(2))) root=e6 source=Named("Mode") destination=Named("Mode") coercion=None origin=o2
use u3 Value(Derived(DerivedId(0))) root=e7 source=Str destination=Str coercion=None origin=o3
use u4 Value(Derived(DerivedId(1))) root=e14 source=Bool destination=Bool coercion=None origin=o4
use u5 Value(ComponentParam(ComponentParamId { component: ComponentId(0), index: 0 })) root=e15 source=Str destination=Str coercion=None origin=o6
use u6 Value(ComponentState(ComponentStateId { component: ComponentId(0), index: 0 })) root=e16 source=Bool destination=Bool coercion=None origin=o7
use u7 View { view: ViewId(2), role: IfCondition } root=e17 source=Bool destination=Bool coercion=None origin=o23
expr e0 i64 1 : I64 origin=o0
expr e1 call Extern(ExternFnId(0)) [CheckedExprId(0)] : Named("User") origin=o0
expr e2 f64 0.25 : F64 origin=o1
expr e3 f64 0.5 : F64 origin=o1
expr e4 f64 0.75 : F64 origin=o1
expr e5 call builtin:color.rgb [CheckedExprId(2), CheckedExprId(3), CheckedExprId(4)] : Color origin=o1
expr e6 path EnumVariant(EnumVariantId { owner: EnumId(0), index: 0 }) [] : Named("Mode") origin=o2
expr e7 path Value(AppState(AppStateId(0))) [CheckedProjection { field: "name", input: Named("User"), output: Str, kind: Struct(StructFieldId { owner: StructId(0), index: 0 }) }] : Str origin=o3
expr e8 path Value(AppState(AppStateId(1))) [CheckedProjection { field: "r", input: Color, output: F64, kind: Native }] : F64 origin=o4
expr e9 f64 0.1 : F64 origin=o4
expr e10 binary Ordering { op: Gt, operand: F64 } e8 e9 : Bool origin=o4
expr e11 path Value(Derived(DerivedId(0))) [] : Str origin=o4
expr e12 str "" : Str origin=o4
expr e13 binary Equality { op: NotEq, operand: Str } e11 e12 : Bool origin=o4
expr e14 binary Boolean(And) e10 e13 : Bool origin=o4
expr e15 str "Card" : Str origin=o6
expr e16 bool false : Bool origin=o7
expr e17 path Value(ComponentState(ComponentStateId { component: ComponentId(0), index: 0 })) [] : Bool origin=o23
view w0 layout Component(ComponentId(0)) parent=None children=[ViewId(1), ViewId(2)] flow=None origin=o15
view w1 text Component(ComponentId(0)) parent=Some(ViewId(0)) children=[] flow=None origin=o16
view w2 if Component(ComponentId(0)) parent=Some(ViewId(0)) children=[ViewId(3)] flow=If { condition: CheckedExprUseId(7) } origin=o17
view w3 text Component(ComponentId(0)) parent=Some(ViewId(2)) children=[] flow=None origin=o18
view w4 layout App parent=None children=[ViewId(5), ViewId(6)] flow=None origin=o19
view w5 component App parent=Some(ViewId(4)) children=[] flow=None origin=o20
view w6 text App parent=Some(ViewId(4)) children=[] flow=None origin=o21
"#
        );
        assert_eq!(
            facts.metrics(),
            CheckedFactMetrics {
                values: 7,
                locals: 0,
                views: 7,
                expression_uses: 8,
                expressions: 18,
                type_analysis_queries: 18,
                type_analysis_nodes: 18,
                type_analysis_cache_hits: 0,
                initializer_analysis_passes: 7,
                app_setting_analysis_passes: 0,
                view_analysis_passes: 1,
                handler_authoritative_analyses: 0,
                handler_auxiliary_analyses: 0,
                type_scope_env_overlays: 0,
                type_scope_env_full_clones: 0,
                declaration_lookups: 18,
                builtin_intern_lookups: 1,
                scope_env_builds: 3,
                scope_env_entries: 12,
                scope_env_overlays: 0,
                scope_env_full_clones: 0,
                view_scope_env_overlays: 0,
                view_scope_env_full_clones: 0,
            }
        );
    }

    #[test]
    fn lowering_moves_checked_facts_without_re_resolving_the_ast() {
        let source = format!(
            "app Facts\n{THEME}state\n  value:color = color.black()\nview\n  text \"ok\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let before = checked.facts.structural_snapshot();
        checked.document.states[0].initial = Expr::Call {
            name: "color.white".into(),
            args: Vec::new(),
        };

        let program = lower::lower(checked).unwrap();
        assert_eq!(program.checked_facts().structural_snapshot(), before);
        let initializer = program.checked_facts().values()[0].initializer.unwrap();
        let root = program.checked_facts().expression_use(initializer).root;
        let CheckedExprKind::Call {
            target: CheckedCallTarget::Builtin(builtin),
            ..
        } = &program.checked_facts().expression(root).kind
        else {
            panic!("state initializer must remain a resolved builtin call");
        };
        assert_eq!(program.checked_facts().builtin(*builtin), "color.black");
        let generated = crate::codegen::generate(&program, "facts.ice").unwrap();
        assert!(generated.contains("value: ::iced::Color::BLACK"));
        assert!(!generated.contains("value: ::iced::Color::WHITE"));
    }

    #[test]
    fn view_and_component_codegen_ignore_post_check_expression_mutations() {
        let source = format!(
            "app Frozen\n{THEME}state\n  count = 1\n  visible = true\n  values = [1, 2]\n  choice:str? = some(\"ready\")\ncomponent Card(value:i64)\n  col\n    text value\n    if provided(Footer)\n      slot Footer?\nview\n  col\n    Card value=(count + 1)\n      Footer:\n        text \"footer-from-slot\"\n    if visible\n      text \"visible\"\n    for value in values\n      text value\n    match choice\n      some(label)\n        text label\n      none\n        text \"none\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout {
            children: component_children,
            ..
        } = &mut checked.document.components[0].root
        else {
            panic!("component root must be a layout");
        };
        let ViewNode::If {
            condition: provided,
            ..
        } = &mut component_children[1]
        else {
            panic!("component child must be a provided guard");
        };
        *provided = Expr::Bool(false);
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Component { args, .. } = &mut children[0] else {
            panic!("first child must be a component call");
        };
        args[0].value = Expr::Bool(false);
        let ViewNode::If { condition, .. } = &mut children[1] else {
            panic!("second child must be an if");
        };
        *condition = Expr::Bool(false);
        let ViewNode::For { item, items, .. } = &mut children[2] else {
            panic!("third child must be a for");
        };
        *item = "mutated".into();
        *items = Expr::Bool(false);
        let ViewNode::Match { value, arms, .. } = &mut children[3] else {
            panic!("fourth child must be a match");
        };
        *value = Expr::Bool(false);
        arms[0].pattern = MatchPattern::Wildcard;

        let program = lower::lower(checked).unwrap();
        let provided = program
            .checked_facts()
            .views()
            .iter()
            .find_map(|view| match view.flow {
                CheckedViewFlow::If { condition }
                    if matches!(view.scope, CheckedViewScope::Component(_)) =>
                {
                    Some(condition)
                }
                _ => None,
            })
            .unwrap();
        assert!(matches!(
            program
                .checked_facts()
                .expression(program.checked_facts().expression_use(provided).root)
                .kind,
            CheckedExprKind::SlotProvided(ComponentSlotId {
                component: ComponentId(0),
                index: 0,
            })
        ));
        let generated = crate::codegen::generate(&program, "frozen.ice").unwrap();
        assert!(generated.contains("footer-from-slot"));
        assert!(generated.contains("self.count + 1"));
        assert!(generated.contains("visible"));
        assert!(generated.contains("for (__ice_index, value) in self.values.iter()"));
        assert!(generated.contains("::std::option::Option::Some(label)"));
        assert!(!generated.contains("for (__ice_index, mutated)"));
    }

    #[test]
    fn missing_and_leftover_view_analyses_are_e196_invariants() {
        let missing_source =
            format!("app Missing\n{THEME}view\n  col\n    if true\n      text \"ok\"\n");
        let missing_document = crate::parse(&missing_source).unwrap();
        let mut missing_origins = OriginArena::default();
        let missing_declarations = DeclarationIndex::build(&missing_document, &mut missing_origins);
        let missing = build(
            &missing_document,
            &missing_declarations,
            &mut missing_origins,
            CheckedAnalyses::default(),
        )
        .unwrap_err();
        assert_eq!(missing.code, "E196");
        assert!(
            missing
                .message
                .contains("missing authoritative view expression analysis")
        );

        let extra_source = format!("app Extra\n{THEME}view\n  text \"ok\"\n");
        let extra_document = crate::parse(&extra_source).unwrap();
        let mut extra_origins = OriginArena::default();
        let extra_declarations = DeclarationIndex::build(&extra_document, &mut extra_origins);
        let root = extra_declarations
            .view_id(extra_document.view.span())
            .unwrap();
        let expression = Expr::Bool(true);
        let analysis = crate::check::expr::analyze_expr_types(
            &expression,
            &HashMap::new(),
            &extra_document,
            extra_document.view.span(),
        )
        .unwrap();
        let mut analyses = CheckedAnalyses::default();
        analyses
            .insert_expression(
                CheckedExprOwner::View {
                    view: root,
                    role: CheckedViewExprRole::IfCondition,
                },
                analysis,
            )
            .unwrap();
        let leftover = build(
            &extra_document,
            &extra_declarations,
            &mut extra_origins,
            analyses,
        )
        .unwrap_err();
        assert_eq!(leftover.code, "E196");
        assert!(
            leftover
                .message
                .contains("checked analyses were not consumed")
        );
    }

    #[test]
    fn duplicate_and_mismatched_handler_analysis_owners_are_e196_invariants() {
        let source =
            format!("app HandlerOwners\n{THEME}on update\n  let value = 1\nview\n  text \"ok\"\n");
        let document = crate::parse(&source).unwrap();
        let mut origins = OriginArena::default();
        let declarations = DeclarationIndex::build(&document, &mut origins);
        let statement = declarations.handlers()[0].statement_roots[0];
        let expression = Expr::I64(1);
        let analysis = crate::check::expr::analyze_expr_types(
            &expression,
            &HashMap::new(),
            &document,
            &document.handlers[0].span,
        )
        .unwrap();
        let mut duplicate = CheckedAnalyses::default();
        let owner = CheckedExprOwner::HandlerStatement {
            statement,
            operand: 0,
        };
        duplicate
            .insert_expression(owner, analysis.clone())
            .unwrap();
        let error = duplicate.insert_expression(owner, analysis).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("analyzed more than once"));

        let mut origins = OriginArena::default();
        let declarations = DeclarationIndex::build(&document, &mut origins);
        let analysis = crate::check::expr::analyze_expr_types(
            &expression,
            &HashMap::new(),
            &document,
            &document.handlers[0].span,
        )
        .unwrap();
        let mut mismatched = CheckedAnalyses::default();
        mismatched
            .insert_expression(
                CheckedExprOwner::HandlerStatement {
                    statement,
                    operand: 99,
                },
                analysis,
            )
            .unwrap();
        let error = build(&document, &declarations, &mut origins, mismatched).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("missing authoritative handler expression")
        );
    }

    #[test]
    fn missing_component_argument_source_is_an_e196_invariant() {
        let source = format!(
            "app MissingArg\n{THEME}state\n  count = 1\ncomponent Card(value:i64)\n  text value\nview\n  Card value=count\n"
        );
        let mut checked = analyze(&source).unwrap();
        let call_view = checked
            .declarations
            .view_id(checked.document.view.span())
            .unwrap();
        let call = checked.declarations.component_call_id(call_view).unwrap();
        let component = checked.declarations.component(0).id;
        let param = checked.declarations.component_param(component, 0).id;
        checked
            .facts
            .component_argument_sources
            .remove(&(call, param));
        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("component argument has no checked source")
        );
    }

    #[test]
    fn component_argument_raw_supplied_default_mutation_is_an_e196_invariant() {
        let source = format!(
            "app MutatedArg\n{THEME}state\n  count = 1\ncomponent Card(value:i64=9)\n  text value\nview\n  Card value=count\n"
        );
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Component { args, .. } = &mut checked.document.view else {
            panic!("fixture root must be a component call");
        };
        args.clear();

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("supplied/default topology diverged"));
    }

    #[test]
    fn raw_view_topology_mutation_is_rejected_before_emission() {
        let source =
            format!("app MutatedView\n{THEME}state\n  count = 1\nview\n  col\n    text count\n");
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, span, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let expected_line = span.line;
        children.clear();

        let program = lower::lower(checked).unwrap();
        let error = crate::codegen::generate(&program, "mutated-view.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("checked topology"));
    }

    #[test]
    fn malformed_match_hir_reports_the_raw_arm_source() {
        let source = format!(
            "app MutatedMatch\n{THEME}state\n  choice:i64? = some(1)\nview\n  col\n    match choice\n      some(value)\n        text value\n      none\n        text \"none\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let raw_match = &children[0];
        let view = checked.declarations.view_id(raw_match.span()).unwrap();
        let ViewNode::Match { arms, .. } = raw_match else {
            panic!("fixture child must be a match");
        };
        let expected_line = arms[0].span.line;
        let CheckedViewFlow::Match { arms, .. } = &mut checked.facts.views[view.0 as usize].flow
        else {
            panic!("fixture must have checked match flow");
        };
        arms[0].binding = None;

        let program = lower::lower(checked).unwrap();
        let error = crate::codegen::generate(&program, "mutated-match.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("some pattern has no payload local"));
    }

    #[test]
    fn invalid_match_enum_id_is_a_fallible_source_mapped_invariant() {
        let source = format!(
            "app MutatedEnum\n{THEME}enum Status\n  ready\nstate\n  status:Status = Status.ready\nview\n  col\n    match status\n      Status.ready\n        text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let raw_match = &children[0];
        let view = checked.declarations.view_id(raw_match.span()).unwrap();
        let ViewNode::Match { arms: raw_arms, .. } = raw_match else {
            panic!("fixture child must be a match");
        };
        let expected_line = raw_arms[0].span.line;
        let CheckedViewFlow::Match { arms, .. } = &mut checked.facts.views[view.0 as usize].flow
        else {
            panic!("fixture must have checked match flow");
        };
        arms[0].pattern = CheckedMatchPattern::Enum(EnumVariantId {
            owner: crate::hir::EnumId(u32::MAX),
            index: 0,
        });

        let program = lower::lower(checked).unwrap();
        let error = crate::codegen::generate(&program, "mutated-enum.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("invalid enum ID"));
    }

    #[test]
    fn daemon_window_is_a_checked_view_local_used_by_component_arguments() {
        let source = format!(
            "daemon Agent\n  window dashboard\n    size 800 600\n{THEME}component AgentWindow(id:window-id)\n  text \"agent\"\nview\n  AgentWindow id=window\n"
        );
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let window = program
            .checked_facts()
            .locals()
            .iter()
            .find(|local| local.name == "window")
            .unwrap();
        assert!(matches!(
            window.owner,
            CheckedLocalOwner::View {
                role: CheckedViewLocalRole::DaemonWindow,
                ..
            }
        ));
        let argument = program
            .component_call(program.document().view.span())
            .unwrap()
            .arguments[0]
            .expression;
        let root = program.checked_facts().expression_use(argument).root;
        assert!(matches!(
            program.checked_facts().expression(root).kind,
            CheckedExprKind::Path {
                root: CheckedPathRoot::Local(_),
                ..
            }
        ));
    }

    #[test]
    fn lexical_view_locals_have_explicit_owner_roles() {
        let source = format!(
            "app Arena\nextern crate::backend\n  Item(id:i64, name:str)\n{THEME}state\n  items:[Item] = []\n  choice:str? = some(\"ready\")\nview\n  col\n    for row in items\n      text row.name\n    match choice\n      some(label)\n        text label\n      none\n        text \"none\"\n    keyed keyed_row in items by=keyed_row.id\n      text keyed_row.name\n    lazy choice as cached\n      text \"cached\"\n    table table_row in items\n      col\n        header\n          text \"Name\"\n        cell\n          text table_row.name\n    panes #work\n      pane files maximized=files_maximized\n        col\n          if files_maximized\n            text \"files\"\n      pane pane_item in items by=pane_item.id maximized=pane_maximized\n        col\n          if pane_maximized\n            text pane_item.name\n    responsive size=(available_width, available_height)\n      col\n        if available_width < available_height\n          text \"portrait\"\n"
        );
        let pane_template_line = source
            .lines()
            .position(|line| line.contains("pane pane_item in items"))
            .unwrap()
            + 1;
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let roles = program
            .checked_facts()
            .locals()
            .iter()
            .filter_map(|local| match local.owner {
                CheckedLocalOwner::View { role, .. } => Some(role),
                CheckedLocalOwner::ExpressionBinding { .. }
                | CheckedLocalOwner::HandlerParam { .. }
                | CheckedLocalOwner::StatementLet(_)
                | CheckedLocalOwner::TaskTransform { .. }
                | CheckedLocalOwner::AppSettingDaemonWindow { .. } => None,
            })
            .collect::<Vec<_>>();
        for role in [
            CheckedViewLocalRole::ForItem,
            CheckedViewLocalRole::MatchPayload(0),
            CheckedViewLocalRole::KeyedItem,
            CheckedViewLocalRole::LazyDependency,
            CheckedViewLocalRole::TableRow,
            CheckedViewLocalRole::PaneMaximized(0),
            CheckedViewLocalRole::PaneTemplateItem(0),
            CheckedViewLocalRole::PaneTemplateMaximized(0),
            CheckedViewLocalRole::ResponsiveWidth,
            CheckedViewLocalRole::ResponsiveHeight,
        ] {
            assert!(roles.contains(&role), "missing checked local role {role:?}");
        }
        let checked_match = program
            .checked_facts()
            .views()
            .iter()
            .find(|view| matches!(view.flow, CheckedViewFlow::Match { .. }))
            .unwrap();
        let CheckedViewFlow::Match { arms, .. } = &checked_match.flow else {
            unreachable!();
        };
        let payload = arms[0].binding.unwrap();
        assert_eq!(
            program
                .origin(program.checked_facts().local(payload).origin)
                .parent,
            Some(arms[0].origin)
        );
        let pane = program
            .checked_facts()
            .views()
            .iter()
            .find(|view| matches!(view.flow, CheckedViewFlow::PaneGrid { .. }))
            .unwrap();
        let CheckedViewFlow::PaneGrid { templates, .. } = &pane.flow else {
            unreachable!();
        };
        let key_origin = program.origin(
            program
                .checked_facts()
                .expression_use(templates[0].key)
                .origin,
        );
        assert_eq!(key_origin.line, pane_template_line);
        assert_eq!(key_origin.parent, Some(pane.origin));
        let generated = crate::codegen::generate(&program, "arena.ice").unwrap();
        assert!(generated.contains("for (__ice_index, row) in self.items.iter()"));
        assert!(generated.contains("::std::option::Option::Some(label)"));
        assert!(generated.contains("for keyed_row in self.items.iter()"));
        assert!(generated.contains("move |(__row, table_row)"));
        assert!(generated.contains("__pane_maximized"));
        assert!(generated.contains("(__size.width as f64)"));
    }

    #[test]
    fn every_typed_match_family_uses_resolved_patterns_and_payload_locals() {
        let source = r#"app MatchFacts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette light for AppTheme
  bg #ffffff
  fg #000000
  primary #3366ff
  danger #ff0000
palette dark for AppTheme
  bg #000000
  fg #ffffff
  primary #6699ff
  danger #ff0000
enum RequestState
  idle
  ready([str])
state
  choice:str? = some("selected")
  outcome:result[str,str] = err("failed")
  request:RequestState = RequestState.ready(["one"])
  active:palette[AppTheme] = AppTheme.light
view
  col
    match choice
      some(value)
        text value
      none
        text "none"
    match outcome
      ok(value)
        text value
      err(error)
        text error
    match request
      RequestState.idle
        text "idle"
      RequestState.ready(items)
        text len(items)
    match active
      AppTheme.light
        text "light"
      AppTheme.dark
        text "dark"
"#;
        let mut checked = analyze(source).unwrap();
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        for child in children {
            let ViewNode::Match { value, arms, .. } = child else {
                panic!("every fixture child must be a typed match");
            };
            *value = Expr::Bool(false);
            for arm in arms {
                arm.pattern = MatchPattern::Wildcard;
            }
        }

        let program = lower::lower(checked).unwrap();
        let matches = program
            .checked_facts()
            .views()
            .iter()
            .filter(|view| matches!(view.flow, CheckedViewFlow::Match { .. }))
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 4);
        let arms = |index: usize| match &matches[index].flow {
            CheckedViewFlow::Match { arms, .. } => arms.as_slice(),
            _ => unreachable!(),
        };
        assert!(matches!(arms(0)[0].pattern, CheckedMatchPattern::Some));
        assert!(matches!(arms(0)[1].pattern, CheckedMatchPattern::None));
        assert!(matches!(arms(1)[0].pattern, CheckedMatchPattern::Ok));
        assert!(matches!(arms(1)[1].pattern, CheckedMatchPattern::Err));
        assert!(matches!(
            arms(2)[0].pattern,
            CheckedMatchPattern::Enum(EnumVariantId { index: 0, .. })
        ));
        assert!(matches!(
            arms(2)[1].pattern,
            CheckedMatchPattern::Enum(EnumVariantId { index: 1, .. })
        ));
        assert!(matches!(
            arms(3)[0].pattern,
            CheckedMatchPattern::Palette(PaletteId(0))
        ));
        assert!(matches!(
            arms(3)[1].pattern,
            CheckedMatchPattern::Palette(PaletteId(1))
        ));
        for (match_index, bound_arms) in
            [(0, vec![0]), (1, vec![0, 1]), (2, vec![1]), (3, Vec::new())]
        {
            for arm_index in bound_arms {
                let local = arms(match_index)[arm_index].binding.unwrap();
                assert!(matches!(
                    program.checked_facts().local(local).owner,
                    CheckedLocalOwner::View {
                        view,
                        role: CheckedViewLocalRole::MatchPayload(role_arm),
                    } if view == matches[match_index].id && role_arm == arm_index as u32
                ));
            }
        }
        let generated = crate::codegen::generate(&program, "match_facts.ice").unwrap();
        assert!(generated.contains("::std::option::Option::Some(value) =>"));
        assert!(generated.contains("::std::result::Result::Err(error) =>"));
        assert!(generated.contains("RequestState::Ready(items) =>"));
        assert!(generated.contains("AppTheme::Light =>"));
    }

    #[test]
    fn imported_expression_origins_keep_their_physical_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-checked-facts-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("card.ice");
        fs::write(
            &root,
            format!("app Facts\nuse \"card.ice\" as ui\n{THEME}view\n  ui::Card\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Card()\n  state\n    open = false\n  col\n    text \"Card\"\n",
        )
        .unwrap();

        let program = lower::lower(analyze_file(&root).unwrap()).unwrap();
        let facts = program.checked_facts();
        let state = facts
            .values()
            .iter()
            .find(|value| matches!(value.id, CheckedValueRef::ComponentState(_)))
            .unwrap();
        let CheckedValueRef::ComponentState(state_id) = state.id else {
            unreachable!();
        };
        let component = program
            .components()
            .iter()
            .find(|component| component.name == "ui::Card")
            .unwrap();
        let lowered_state = component
            .states
            .iter()
            .find(|lowered| lowered.name == "open")
            .unwrap();
        assert_eq!(lowered_state.id, state_id);
        assert_eq!(lowered_state.origin, state.origin);
        let value_origin = program.origin(state.origin);
        let initializer = facts.expression_use(state.initializer.unwrap());
        let expression_origin = program.origin(facts.expression(initializer.root).origin);
        assert_eq!(value_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(value_origin.line, 3);
        assert_eq!(value_origin.column, 1);
        assert_eq!(value_origin.parent, Some(component.origin));
        let component_origin = program.origin(component.origin);
        assert_eq!(component_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(component_origin.line, 1);
        let component_view = facts
            .views()
            .iter()
            .find(|view| view.scope == CheckedViewScope::Component(component.id))
            .unwrap();
        assert_eq!(
            program.origin(component_view.origin).parent,
            Some(component.origin)
        );
        let child = facts.view(component_view.children[0]);
        assert_eq!(
            program.origin(child.origin).parent,
            Some(component_view.origin)
        );
        assert_eq!(expression_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(expression_origin.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_view_expression_origins_keep_physical_parent_chains() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-view-expression-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("card.ice");
        fs::write(
            &root,
            format!("app Facts\nuse \"card.ice\" as ui\n{THEME}view\n  ui::Card\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Card()\n  state\n    open = true\n  col\n    if open\n      text \"Open\"\n",
        )
        .unwrap();

        let program = lower::lower(analyze_file(&root).unwrap()).unwrap();
        let checked_if = program
            .checked_facts()
            .views()
            .iter()
            .find(|view| matches!(view.flow, CheckedViewFlow::If { .. }))
            .unwrap();
        let CheckedViewFlow::If { condition } = checked_if.flow else {
            unreachable!();
        };
        let expression = program.checked_facts().expression_use(condition);
        let expression_origin = program.origin(expression.origin);
        assert_eq!(expression_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(expression_origin.line, 5);
        assert_eq!(expression_origin.parent, Some(checked_if.origin));
        let if_origin = program.origin(checked_if.origin);
        assert_eq!(if_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(if_origin.line, 5);
        assert!(if_origin.parent.is_some());

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_semantic_declarations_keep_physical_and_parent_origins() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-declaration-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("model.ice");
        fs::write(
            &root,
            format!("app Facts\nuse \"model.ice\" as model\n{THEME}view\n  text \"ok\"\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "enum Status\n  idle\n  loaded(str)\nextern crate::backend\n  User(name:str)\n  sync load_user() -> User\n",
        )
        .unwrap();

        let program = lower::lower(analyze_file(&root).unwrap()).unwrap();
        let declarations = program.declarations();

        let enum_decl = declarations.enum_decl_by_name("model::Status").unwrap();
        let enum_origin = program.origin(enum_decl.declaration.origin);
        assert_eq!(enum_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(enum_origin.line, 1);
        assert_eq!(enum_origin.parent, None);
        assert_eq!(enum_decl.variants.len(), 2);
        assert_eq!(enum_decl.variants[1].name, "loaded");
        assert_eq!(enum_decl.variants[1].payload, Some(Type::Str));
        let variant_origin = program.origin(enum_decl.variants[1].declaration.origin);
        assert_eq!(variant_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(variant_origin.line, 3);
        assert_eq!(variant_origin.parent, Some(enum_decl.declaration.origin));

        let struct_decl = declarations.struct_decl_by_name("model::User").unwrap();
        assert_eq!(struct_decl.rust_path, "crate::backend::User");
        let struct_origin = program.origin(struct_decl.declaration.origin);
        assert_eq!(struct_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(struct_origin.line, 5);
        assert_eq!(struct_decl.fields[0].name, "name");
        assert_eq!(struct_decl.fields[0].ty, Type::Str);
        let field_origin = program.origin(struct_decl.fields[0].declaration.origin);
        assert_eq!(field_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(field_origin.line, 5);
        assert_eq!(field_origin.parent, Some(struct_decl.declaration.origin));

        let extern_decl = declarations
            .extern_decl_by_name("model::load_user")
            .unwrap();
        assert_eq!(extern_decl.rust_path, "crate::backend::load_user");
        assert_eq!(extern_decl.kind, ExternKind::Sync);
        assert!(extern_decl.params.is_empty());
        assert_eq!(extern_decl.output, Type::Named("model::User".into()));
        let extern_origin = program.origin(extern_decl.declaration.origin);
        assert_eq!(extern_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(extern_origin.line, 6);
        assert_eq!(extern_origin.parent, None);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn animation_projection_records_a_typed_scoped_local() {
        let source = format!(
            "app Projection\n{THEME}state\n  progress:animation[f64] = 0.0\nderived\n  projected = animation.project(progress, sample, sample * 2.0)\nview\n  text projected\n"
        );

        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        assert_eq!(facts.locals().len(), 1);
        let local = facts.local(CheckedLocalId(0));
        assert_eq!(local.name, "sample");
        assert_eq!(local.ty, Type::F64);
        assert_eq!(
            local.owner,
            CheckedLocalOwner::ExpressionBinding {
                expression: CheckedExprUseId(1),
                body_argument: 2,
            }
        );

        let projected = facts
            .values()
            .iter()
            .find(|value| value.name == "projected")
            .unwrap();
        let root = facts.expression_use(projected.initializer.unwrap()).root;
        let CheckedExprKind::Call { arguments, .. } = &facts.expression(root).kind else {
            panic!("projection initializer must be a checked call");
        };
        assert!(matches!(
            arguments.as_slice(),
            [
                CheckedCallArgument::Value(_),
                CheckedCallArgument::Binding(CheckedLocalId(0)),
                CheckedCallArgument::Value(_)
            ]
        ));
        let CheckedCallArgument::Value(body) = arguments[2] else {
            unreachable!();
        };
        let CheckedExprKind::Binary { left, .. } = facts.expression(body).kind else {
            panic!("projection body must retain its checked binary expression");
        };
        assert!(matches!(
            facts.expression(left).kind,
            CheckedExprKind::Path {
                root: CheckedPathRoot::Local(CheckedLocalId(0)),
                ..
            }
        ));
        assert_eq!(facts.metrics().locals, 1);
        assert_eq!(facts.metrics().type_analysis_nodes, 6);
        assert_eq!(facts.metrics().expressions, 6);
    }

    #[test]
    fn contextual_builtin_arguments_receive_the_declared_default_types() {
        let source = format!(
            "app Defaults\n{THEME}component Context(items:[str]=[], selected:str?=none, nested:str?=some(\"ready\"), success:result[str,str]=ok(\"yes\"), failure:result[str,str]=err(\"no\"))\n  text \"defaults\"\nview\n  Context\n"
        );

        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        for (name, expected) in [
            ("items", Type::List(Box::new(Type::Str))),
            ("selected", Type::Option(Box::new(Type::Str))),
            ("nested", Type::Option(Box::new(Type::Str))),
            (
                "success",
                Type::Result(Box::new(Type::Str), Box::new(Type::Str)),
            ),
            (
                "failure",
                Type::Result(Box::new(Type::Str), Box::new(Type::Str)),
            ),
        ] {
            let value = facts
                .values()
                .iter()
                .find(|value| value.name == name)
                .unwrap();
            let expression =
                facts.expression(facts.expression_use(value.initializer.unwrap()).root);
            assert_eq!(expression.ty, expected, "{name}");
        }
        assert_eq!(
            facts.metrics().type_analysis_nodes,
            facts.metrics().expressions
        );
    }

    #[test]
    fn fixed_builtin_signatures_contextualize_empty_list_and_none_arguments() {
        let source = format!(
            "app BuiltinContexts\n{THEME}derived\n  gradient = linear.add_stops(linear(0.0), [])\n  debug_is_active = debug.active(none)\n  task_was_aborted = aborted(none)\nview\n  text \"contexts\"\n"
        );

        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        let argument_types = |name: &str| {
            let value = facts
                .values()
                .iter()
                .find(|value| value.name == name)
                .unwrap();
            let root = facts.expression_use(value.initializer.unwrap()).root;
            let CheckedExprKind::Call { arguments, .. } = &facts.expression(root).kind else {
                panic!("{name} must be a checked builtin call");
            };
            arguments
                .iter()
                .map(|argument| match argument {
                    CheckedCallArgument::Value(id) => facts.expression(*id).ty.clone(),
                    CheckedCallArgument::Binding(_) => panic!("{name} has no binding argument"),
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            argument_types("gradient"),
            [Type::LinearGradient, Type::List(Box::new(Type::ColorStop))]
        );
        assert_eq!(
            argument_types("debug_is_active"),
            [Type::Option(Box::new(Type::DebugSpan))]
        );
        assert_eq!(
            argument_types("task_was_aborted"),
            [Type::Option(Box::new(Type::TaskHandle))]
        );
        assert_eq!(
            facts.metrics().type_analysis_nodes,
            facts.metrics().expressions
        );
    }

    #[test]
    fn generic_expression_contexts_resolve_erased_empty_values() {
        let source = format!(
            "app GenericContexts\n{THEME}state\n  optional:str? = none\n  timed:[str] = debug.time_with(\"items\", [])\nderived\n  optional_equal = none == optional\n  empty_equal = [] == [\"item\"]\n  erased_equal = [] == []\n  empty_length = len([])\n  is_empty = empty([])\nview\n  text empty_length\n"
        );

        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        assert!(
            facts
                .expressions
                .iter()
                .all(|expression| !contains_unknown(&expression.ty))
        );

        let root = |name: &str| {
            let value = facts
                .values()
                .iter()
                .find(|value| value.name == name)
                .unwrap();
            facts.expression_use(value.initializer.unwrap()).root
        };
        let binary_operand = |name: &str| {
            let CheckedExprKind::Binary { operator, .. } = &facts.expression(root(name)).kind
            else {
                panic!("{name} must be a checked binary expression");
            };
            match operator {
                CheckedBinaryOperator::Equality { operand, .. } => operand.clone(),
                _ => panic!("{name} must be an equality expression"),
            }
        };
        let call_argument = |name: &str, index: usize| {
            let CheckedExprKind::Call { arguments, .. } = &facts.expression(root(name)).kind else {
                panic!("{name} must be a checked call");
            };
            let CheckedCallArgument::Value(argument) = arguments[index] else {
                panic!("{name} argument {index} must be a value");
            };
            facts.expression(argument).ty.clone()
        };

        assert_eq!(call_argument("timed", 1), Type::List(Box::new(Type::Str)));
        assert_eq!(
            binary_operand("optional_equal"),
            Type::Option(Box::new(Type::Str))
        );
        assert_eq!(
            binary_operand("empty_equal"),
            Type::List(Box::new(Type::Str))
        );
        assert_eq!(
            binary_operand("erased_equal"),
            Type::List(Box::new(Type::Unit))
        );
        assert_eq!(
            call_argument("empty_length", 0),
            Type::List(Box::new(Type::Unit))
        );
        assert_eq!(
            call_argument("is_empty", 0),
            Type::List(Box::new(Type::Unit))
        );
    }

    #[test]
    fn initializer_coercions_and_composite_evidence_have_exact_types() {
        let source = format!(
            "app ExactTypes\n{THEME}state\n  items:combo[str] = []\n  nested:combo[[str]] = [[]]\n  progress:animation[f64] = 0.0\n  document:markdown = \"# Document\"\n  draft:editor = \"Draft\"\nderived\n  nested_length = len([[], [\"x\"]])\n  optional_length = len([none, some(\"x\")])\nview\n  text nested_length\n"
        );

        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        let value = |name: &str| {
            facts
                .values()
                .iter()
                .find(|value| value.name == name)
                .unwrap()
        };

        for (name, element) in [
            ("items", Type::Str),
            ("nested", Type::List(Box::new(Type::Str))),
        ] {
            let value = value(name);
            let use_fact = facts.expression_use(value.initializer.unwrap());
            let source = Type::List(Box::new(element.clone()));
            assert_eq!(use_fact.source, source);
            assert_eq!(use_fact.destination, Type::Combo(Box::new(element.clone())));
            assert_eq!(
                use_fact.coercion,
                CheckedInitializerCoercion::ListToCombo {
                    element: element.clone()
                }
            );
            assert_eq!(facts.expression(use_fact.root).ty, source);
        }

        for (name, source, destination, coercion) in [
            (
                "progress",
                Type::F64,
                Type::Animation(Box::new(Type::F64)),
                CheckedInitializerCoercion::ValueToAnimation { value: Type::F64 },
            ),
            (
                "document",
                Type::Str,
                Type::Markdown,
                CheckedInitializerCoercion::StrToMarkdown,
            ),
            (
                "draft",
                Type::Str,
                Type::Editor,
                CheckedInitializerCoercion::StrToEditor,
            ),
        ] {
            let use_fact = facts.expression_use(value(name).initializer.unwrap());
            assert_eq!(use_fact.source, source);
            assert_eq!(use_fact.destination, destination);
            assert_eq!(use_fact.coercion, coercion);
            assert_eq!(facts.expression(use_fact.root).ty, source);
        }

        let call_argument = |name: &str| {
            let use_fact = facts.expression_use(value(name).initializer.unwrap());
            let CheckedExprKind::Call { arguments, .. } = &facts.expression(use_fact.root).kind
            else {
                panic!("{name} must be a call");
            };
            let CheckedCallArgument::Value(argument) = arguments[0] else {
                panic!("{name} must have a value argument");
            };
            argument
        };

        let nested = call_argument("nested_length");
        let nested_ty = Type::List(Box::new(Type::List(Box::new(Type::Str))));
        assert_eq!(facts.expression(nested).ty, nested_ty);
        let CheckedExprKind::List(children) = &facts.expression(nested).kind else {
            panic!("nested length argument must be a list");
        };
        for child in children {
            assert_eq!(facts.expression(*child).ty, Type::List(Box::new(Type::Str)));
        }

        let optional = call_argument("optional_length");
        let optional_element = Type::Option(Box::new(Type::Str));
        assert_eq!(
            facts.expression(optional).ty,
            Type::List(Box::new(optional_element.clone()))
        );
        let CheckedExprKind::List(children) = &facts.expression(optional).kind else {
            panic!("optional length argument must be a list");
        };
        for child in children {
            assert_eq!(facts.expression(*child).ty, optional_element);
        }
    }

    #[test]
    #[ignore = "explicit large checked-fact performance contract"]
    fn performance_contract_ten_thousand_fact_lookups_are_direct_arena_accesses() {
        const VALUES: usize = 10_000;
        let mut source = format!("app Facts\n{THEME}state\n");
        for index in 0..VALUES {
            writeln!(source, "  value_{index} = {index}").unwrap();
        }
        source.push_str("view\n  text \"ok\"\n");

        let started = Instant::now();
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let elapsed = started.elapsed();
        let facts = program.checked_facts();
        assert_eq!(facts.metrics().values, VALUES);
        assert_eq!(facts.metrics().expressions, VALUES);
        assert_eq!(facts.metrics().type_analysis_queries, VALUES);
        assert_eq!(facts.metrics().type_analysis_nodes, VALUES);
        assert_eq!(facts.metrics().type_analysis_cache_hits, 0);
        assert_eq!(facts.metrics().declaration_lookups, VALUES);
        facts.reset_lookup_count();
        for index in 0..VALUES {
            let value = facts.value(CheckedValueId(index as u32));
            let expression_use = facts.expression_use(value.initializer.unwrap());
            assert_eq!(facts.expression(expression_use.root).ty, Type::I64);
        }
        for index in 0..facts.views().len() {
            assert_eq!(
                facts.view(ViewId(index as u32)).scope,
                CheckedViewScope::App
            );
        }
        assert_eq!(facts.lookup_count(), VALUES * 3 + facts.views().len());
        assert!(
            elapsed.as_secs_f64() < 8.0,
            "10k checked value facts built and lowered in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "explicit deep linear checked-expression performance contract"]
    fn performance_contract_deep_expression_is_analyzed_once_per_node() {
        const TERMS: usize = 128;
        let (metrics, elapsed) = std::thread::Builder::new()
            .name("deep-expression-facts".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let mut source = format!("app Deep\n{THEME}state\n  value:i64 = 1");
                for _ in 1..TERMS {
                    source.push_str(" + 1");
                }
                source.push_str("\nview\n  text value\n");
                let started = Instant::now();
                let program = lower::lower(analyze(&source).unwrap()).unwrap();
                (program.checked_facts().metrics(), started.elapsed())
            })
            .unwrap()
            .join()
            .unwrap();

        let nodes = TERMS * 2 - 1;
        assert_eq!(metrics.values, 1);
        assert_eq!(metrics.expression_uses, 1);
        assert_eq!(metrics.expressions, nodes);
        assert_eq!(metrics.type_analysis_queries, nodes);
        assert_eq!(metrics.type_analysis_nodes, nodes);
        assert_eq!(metrics.type_analysis_cache_hits, 0);
        assert!(
            elapsed.as_secs_f64() < 8.0,
            "128-term expression facts built and lowered in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "explicit repeated projection linearity contract"]
    fn performance_contract_four_thousand_projections_use_borrowed_scope_layers() {
        fn measure(derived: usize) -> (CheckedFactMetrics, std::time::Duration) {
            let mut source = format!(
                "app ProjectionFacts\n{THEME}state\n  progress:animation[f64] = 0.0\nderived\n"
            );
            for index in 0..derived {
                writeln!(
                    source,
                    "  value_{index} = animation.project(progress, sample, sample + 1.0)"
                )
                .unwrap();
            }
            source.push_str("view\n  text value_0\n");

            let started = Instant::now();
            let program = lower::lower(analyze(&source).unwrap()).unwrap();
            (program.checked_facts().metrics(), started.elapsed())
        }

        let (small, small_elapsed) = measure(500);
        let (large, large_elapsed) = measure(4_000);
        assert_eq!(large.values, 4_001);
        assert_eq!(large.expression_uses, 4_001);
        assert_eq!(large.initializer_analysis_passes, 4_001);
        assert_eq!(large.scope_env_builds, 2);
        assert_eq!(large.scope_env_entries, 8_002);
        assert_eq!(large.locals, 4_000);
        assert_eq!(large.type_scope_env_full_clones, 0);
        assert_eq!(large.scope_env_full_clones, 0);
        assert_eq!(large.scope_env_overlays, 4_000);
        assert_eq!(large.type_scope_env_overlays, 8_000);
        assert_eq!(large.expressions - 1, (small.expressions - 1) * 8);
        assert_eq!(
            large.type_analysis_nodes - 1,
            (small.type_analysis_nodes - 1) * 8
        );
        assert_eq!(large.scope_env_overlays, small.scope_env_overlays * 8);
        eprintln!("500 projections in {small_elapsed:?}; 4k projections in {large_elapsed:?}");
        assert!(
            large_elapsed.as_secs_f64() < 8.0,
            "4k repeated projection initializers completed in {large_elapsed:?}"
        );
        assert!(
            large_elapsed.as_secs_f64() <= small_elapsed.as_secs_f64() * 12.0 + 0.5,
            "projection scaling exceeded the linear allowance: 500={small_elapsed:?}, 4k={large_elapsed:?}"
        );
    }
}
