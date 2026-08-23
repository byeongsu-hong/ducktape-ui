use super::expr::{
    BuiltinArgumentContext, ContextualBuiltin, ExprTypeAnalysis, field_type, resolve_erased_type,
    unify_type_evidence,
};
use super::*;
#[cfg(test)]
use crate::hir::DerivedId;
use crate::hir::{
    AppSettingExprId, CanvasCommandId, CanvasEventId, CanvasExpressionId, CanvasLocalId,
    CanvasRouteId, ComponentCallId, ComponentEventId, ComponentId, ComponentParamId,
    ComponentSlotId, DeclarationIndex, EnumVariantId, ExternFnId, ExternRef, FloatExpressionId,
    HandlerId, InteractionExpressionId, InteractionRouteId, MediaExpressionId, OriginArena,
    OriginId, PaletteId, PinExpressionId, RouteId, StatementId, StructFieldId, SubscriptionId,
    TaskId, TestId, TestStepId, TestTargetId, TooltipExpressionId, ViewId,
};
pub(crate) use crate::hir::{
    ExpressionId as CheckedExprUseId, ExpressionNodeId as CheckedExprId, LocalId as CheckedLocalId,
    ValueRef as CheckedValueRef,
};
use crate::unqualified_name;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(test)]
use std::time::{Duration, Instant};

#[cfg(test)]
macro_rules! record_fact_metric {
    ($expression:expr) => {
        $expression
    };
}

#[cfg(not(test))]
macro_rules! record_fact_metric {
    ($expression:expr) => {};
}

#[cfg(test)]
#[derive(Debug, Default)]
struct LookupCount(AtomicUsize);

#[cfg(test)]
impl Clone for LookupCount {
    fn clone(&self) -> Self {
        Self(AtomicUsize::new(self.0.load(Ordering::Relaxed)))
    }
}

#[cfg(test)]
impl CheckedExprUseId {
    pub(crate) fn invalid_for_test() -> Self {
        Self(u32::MAX)
    }
}

fn keyed_lazy_value_error(span: &Span) -> Error {
    Error::new(
        "E139",
        span,
        "a keyed lazy value must be a state field, a prop fed from one, or a `for` or keyed row over one",
    )
    .hint(
        "`lazy value by key as name` captures the value by reference, \
         so it must live in app or component state",
    )
}

fn keyed_lazy_prop_argument_error(span: &Span, component: &str, prop: &str) -> Error {
    Error::new(
        "E139",
        span,
        format!(
            "prop `{prop}` of `{component}` feeds a keyed lazy value, \
             so it must be passed an app state field or a prop fed from one"
        ),
    )
    .hint(
        "the keyed lazy captures the prop by reference, so every call site \
         (and any default) must supply a place that outlives the built view",
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedValueId(u32);

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
    TestTarget(TestTargetId),
    CanvasState(CanvasLocalId),
    CanvasWidth(ViewId),
    CanvasHeight(ViewId),
    CanvasCommandItem(CanvasCommandId),
    CanvasEventBinding {
        event: CanvasEventId,
        index: u32,
    },
    AppSettingDaemonWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedViewLocalRole {
    DaemonWindow,
    ForItem,
    /// The item of the nth `for` child inside one `rich-text`.
    RichForItem(u32),
    MatchPayload(u32),
    KeyedItem,
    LazyDependency,
    LazyKey(u32),
    TableRow,
    PaneMaximized(u32),
    PaneTemplateItem(u32),
    PaneTemplateMaximized(u32),
    ResponsiveWidth,
    ResponsiveHeight,
    FloatOriginalX,
    FloatOriginalY,
    FloatOriginalWidth,
    FloatOriginalHeight,
    FloatViewportX,
    FloatViewportY,
    FloatViewportWidth,
    FloatViewportHeight,
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
    pub(crate) identity: Option<CheckedViewIdentity>,
    pub(crate) scope: CheckedViewScope,
    pub(crate) parent: Option<ViewId>,
    pub(crate) children: Vec<ViewId>,
    pub(crate) flow: CheckedViewFlow,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckedComponentSlot {
    pub(crate) id: ComponentSlotId,
    pub(crate) view: ViewId,
    pub(crate) name: String,
    pub(crate) optional: bool,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedViewIdentity {
    pub(crate) name: String,
    pub(crate) key: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedSubscriptionSource {
    Every {
        milliseconds: u64,
    },
    Repeat {
        function: ExternRef,
        milliseconds: u64,
    },
    Run {
        function: ExternRef,
        arguments: Vec<CheckedExprUseId>,
    },
    Recipe {
        function: ExternRef,
        arguments: Vec<CheckedExprUseId>,
    },
    Events {
        identity: CheckedExprUseId,
        filter: ExternRef,
    },
    Event {
        raw: bool,
    },
    Extern {
        function: ExternRef,
        arguments: Vec<CheckedExprUseId>,
    },
    InputMethod(InputMethodEvent),
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    SystemTheme,
    Touch(TouchEvent),
    Window(WindowEvent),
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedSubscriptionRoute {
    pub(crate) handler: HandlerId,
    pub(crate) handler_name: String,
    pub(crate) payloads: Vec<u32>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedSubscription {
    pub(crate) id: SubscriptionId,
    pub(crate) source: CheckedSubscriptionSource,
    pub(crate) source_payloads: Vec<Type>,
    pub(crate) delivered_payloads: Vec<Type>,
    pub(crate) filter: Option<ExternRef>,
    pub(crate) context: Option<CheckedExprUseId>,
    pub(crate) condition: Option<CheckedExprUseId>,
    pub(crate) window_id: bool,
    pub(crate) status: Option<EventStatus>,
    pub(crate) route: CheckedSubscriptionRoute,
    pub(crate) span: Span,
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
        layout: CheckedKeyedLayout,
    },
    Lazy {
        dependency: CheckedExprUseId,
        /// `lazy value by key, key as name`: the projections that stand in
        /// for the value in the memo dependency tuple. Empty for the plain
        /// form.
        keys: Vec<CheckedExprUseId>,
        /// One retained local for each bare-identifier key, aligned with
        /// `keys`. Computed keys and the dependency alias carry `None`.
        key_bindings: Vec<CheckedLazyKeyBinding>,
        binding: CheckedLocalId,
    },
    Table {
        rows: CheckedExprUseId,
        item: CheckedLocalId,
        layout: CheckedTableLayout,
    },
    ResponsiveSize {
        semantic_key: String,
        expression_count: u32,
        width: CheckedLocalId,
        height: CheckedLocalId,
        dimensions: [CheckedLength; 2],
    },
    Float {
        semantic_key: String,
        expression_count: u32,
        geometry: [CheckedLocalId; 8],
    },
    Pin {
        semantic_key: String,
        expression_count: u32,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedLazyKeyBinding {
    pub(crate) local: Option<CheckedLocalId>,
    pub(crate) name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedLength {
    None,
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed {
        expression: CheckedExprUseId,
        source: Type,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CheckedPadding {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) x: Option<CheckedExprUseId>,
    pub(crate) y: Option<CheckedExprUseId>,
    pub(crate) top: Option<CheckedExprUseId>,
    pub(crate) right: Option<CheckedExprUseId>,
    pub(crate) bottom: Option<CheckedExprUseId>,
    pub(crate) left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedKeyedLayout {
    pub(crate) semantic_key: String,
    pub(crate) width: CheckedLength,
    pub(crate) height: CheckedLength,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) padding: CheckedPadding,
    pub(crate) max_width: Option<CheckedExprUseId>,
    pub(crate) virtual_row: Option<CheckedExprUseId>,
    pub(crate) align: Option<FlexAlignment>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedTableColumn {
    pub(crate) width: CheckedLength,
    pub(crate) align_x: Option<InputAlignment>,
    pub(crate) align_y: Option<VerticalAlignment>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedTableLayout {
    pub(crate) semantic_key: String,
    pub(crate) width: CheckedLength,
    pub(crate) padding: Option<CheckedExprUseId>,
    pub(crate) padding_x: Option<CheckedExprUseId>,
    pub(crate) padding_y: Option<CheckedExprUseId>,
    pub(crate) separator: Option<CheckedExprUseId>,
    pub(crate) separator_x: Option<CheckedExprUseId>,
    pub(crate) separator_y: Option<CheckedExprUseId>,
    pub(crate) columns: Vec<CheckedTableColumn>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedPaneAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedPaneConfiguration {
    Pane(String),
    Split {
        name: Option<String>,
        axis: CheckedPaneAxis,
        ratio: f32,
        a: Box<CheckedPaneConfiguration>,
        b: Box<CheckedPaneConfiguration>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneGradientStop {
    pub(crate) color: String,
    pub(crate) offset: CheckedExprUseId,
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedPaneBackground {
    Color(String),
    Linear {
        angle: CheckedExprUseId,
        stops: Vec<CheckedPaneGradientStop>,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CheckedPaneRadius {
    pub(crate) all: Option<CheckedExprUseId>,
    pub(crate) top_left: Option<CheckedExprUseId>,
    pub(crate) top_right: Option<CheckedExprUseId>,
    pub(crate) bottom_right: Option<CheckedExprUseId>,
    pub(crate) bottom_left: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CheckedPaneSurface {
    pub(crate) background: Option<CheckedPaneBackground>,
    pub(crate) text_color: Option<String>,
    pub(crate) border_color: Option<String>,
    pub(crate) border_width: Option<CheckedExprUseId>,
    pub(crate) radius: CheckedPaneRadius,
    pub(crate) shadow_color: Option<String>,
    pub(crate) shadow_x: Option<CheckedExprUseId>,
    pub(crate) shadow_y: Option<CheckedExprUseId>,
    pub(crate) shadow_blur: Option<CheckedExprUseId>,
    pub(crate) pixel_snap: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneTitle {
    pub(crate) padding: CheckedPadding,
    pub(crate) always_show_controls: bool,
    pub(crate) has_controls: bool,
    pub(crate) has_compact_controls: bool,
    pub(crate) surface: CheckedPaneSurface,
    pub(crate) style_site: CheckedPaneStyleSite,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CheckedPaneStyleSite {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneView {
    pub(crate) name: String,
    pub(crate) maximized: Option<CheckedLocalId>,
    pub(crate) surface: CheckedPaneSurface,
    pub(crate) style_site: CheckedPaneStyleSite,
    pub(crate) title: Option<CheckedPaneTitle>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneTemplate {
    pub(crate) items: CheckedPathRoot,
    pub(crate) item: CheckedLocalId,
    pub(crate) key: CheckedExprUseId,
    pub(crate) key_type: Type,
    pub(crate) pane: CheckedPaneView,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneGridStyle {
    pub(crate) region_background: Option<CheckedPaneBackground>,
    pub(crate) region_border: Option<String>,
    pub(crate) region_border_width: Option<CheckedExprUseId>,
    pub(crate) region_radius: CheckedPaneRadius,
    pub(crate) hovered_split: Option<String>,
    pub(crate) hovered_split_width: Option<CheckedExprUseId>,
    pub(crate) picked_split: Option<String>,
    pub(crate) picked_split_width: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneCustomStyle {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPaneGrid {
    pub(crate) id: ViewId,
    pub(crate) name: String,
    pub(crate) configuration: CheckedPaneConfiguration,
    pub(crate) width: CheckedLength,
    pub(crate) height: CheckedLength,
    pub(crate) spacing: Option<CheckedExprUseId>,
    pub(crate) min_size: Option<CheckedExprUseId>,
    pub(crate) resize_leeway: Option<CheckedExprUseId>,
    pub(crate) draggable: bool,
    pub(crate) click: Option<CheckedInteractionRoute>,
    pub(crate) custom_style: Option<CheckedPaneCustomStyle>,
    pub(crate) style: CheckedPaneGridStyle,
    pub(crate) panes: Vec<CheckedPaneView>,
    pub(crate) templates: Vec<CheckedPaneTemplate>,
    pub(crate) expression_count: u32,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedMatchArm {
    pub(crate) pattern: CheckedMatchPattern,
    pub(crate) binding: Option<CheckedLocalId>,
    pub(crate) children: Vec<ViewId>,
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
    Subscription {
        subscription: SubscriptionId,
        role: CheckedSubscriptionExprRole,
    },
    TestTargetKey {
        target: TestTargetId,
        segment: u32,
    },
    TestStepOperand {
        step: TestStepId,
        operand: u32,
    },
    Canvas(CanvasExpressionId),
    Media(MediaExpressionId),
    Tooltip(TooltipExpressionId),
    Float(FloatExpressionId),
    Pin(PinExpressionId),
    Interaction(InteractionExpressionId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedSubscriptionExprRole {
    Condition,
    Context,
    SourceArgument(u32),
    EventIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedComponentArgumentSource {
    Supplied(CheckedExprUseId),
    Default(CheckedExprUseId),
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedComponentCallRoutes {
    pub(crate) call: ComponentCallId,
    pub(crate) view: ViewId,
    pub(crate) component: ComponentId,
    pub(crate) output: Option<CheckedComponentOutputRoute>,
    pub(crate) events: Vec<CheckedComponentEventRoute>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedComponentOutputRoute {
    pub(crate) output: Type,
    pub(crate) route: InteractionRouteId,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedComponentEventRoute {
    pub(crate) event: ComponentEventId,
    pub(crate) name: String,
    pub(crate) payloads: Vec<Type>,
    pub(crate) delivery: CheckedComponentEventDelivery,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedComponentEventDelivery {
    Direct {
        route: InteractionRouteId,
        origin: OriginId,
    },
    Forward {
        outer_component: ComponentId,
        outer_component_name: String,
        outer_event: ComponentEventId,
        outer_event_name: String,
        outer_payloads: Vec<Type>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CheckedViewExprRole {
    IdentityKey,
    IfCondition,
    ForItems,
    MatchValue,
    KeyedItems,
    KeyedKey,
    KeyedWidth,
    KeyedHeight,
    KeyedSpacing,
    KeyedPaddingAll,
    KeyedPaddingX,
    KeyedPaddingY,
    KeyedPaddingTop,
    KeyedPaddingRight,
    KeyedPaddingBottom,
    KeyedPaddingLeft,
    KeyedMaxWidth,
    KeyedVirtualRow,
    LazyDependency,
    LazyKey(u32),
    TableRows,
    TableWidth,
    TablePadding,
    TablePaddingX,
    TablePaddingY,
    TableSeparator,
    TableSeparatorX,
    TableSeparatorY,
    TableColumnWidth(u32),
    ResponsiveWidthDimension,
    ResponsiveHeightDimension,
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedSubscriptionSourceAnalysis {
    Every {
        milliseconds: u64,
    },
    Repeat {
        function: ExternRef,
        milliseconds: u64,
    },
    Run {
        function: ExternRef,
    },
    Recipe {
        function: ExternRef,
    },
    Events {
        filter: ExternRef,
    },
    Event {
        raw: bool,
    },
    Extern {
        function: ExternRef,
    },
    InputMethod(InputMethodEvent),
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    SystemTheme,
    Touch(TouchEvent),
    Window(WindowEvent),
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedSubscriptionAnalysis {
    pub(crate) source: CheckedSubscriptionSourceAnalysis,
    pub(crate) source_payloads: Vec<Type>,
    pub(crate) delivered_payloads: Vec<Type>,
    pub(crate) filter: Option<ExternRef>,
    pub(crate) route_handler: HandlerId,
    pub(crate) route_handler_name: String,
    pub(crate) route_payloads: Vec<u32>,
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
    Extern(ExternRef),
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
    pub(crate) owner: CheckedExprUseId,
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
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedStatement {
    pub(crate) id: StatementId,
    pub(crate) semantic_key: String,
    pub(crate) operation: Option<crate::hir::HandlerOperationContract>,
    pub(crate) writable_targets: Vec<CheckedValueRef>,
    pub(crate) self_move: Option<bool>,
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
pub(crate) struct CheckedCanvasRoute {
    pub(crate) id: CanvasRouteId,
    pub(crate) target: CheckedCanvasRouteTarget,
    pub(crate) args: Vec<CheckedCanvasRouteArg>,
    pub(crate) source_payloads: Vec<Type>,
    pub(crate) ordered_payloads: bool,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedCanvasRouteTarget {
    Handler(HandlerId),
    ComponentOutput {
        component: ComponentId,
        output: Type,
    },
    ComponentEvent {
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedCanvasRouteArg {
    Expression(CheckedExprUseId),
    Payload,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedCanvas {
    pub(crate) id: ViewId,
    pub(crate) states: Vec<CheckedLocalId>,
    pub(crate) width: CheckedLocalId,
    pub(crate) height: CheckedLocalId,
    pub(crate) expression_count: u32,
    pub(crate) routes: Vec<CheckedCanvasRoute>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedMedia {
    pub(crate) id: ViewId,
    pub(crate) expression_count: u32,
    pub(crate) semantic_key: String,
    pub(crate) style: Option<ExternFnId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedContainer {
    pub(crate) id: ViewId,
    pub(crate) expression_count: u32,
    pub(crate) semantic_key: String,
    pub(crate) style: Option<ExternFnId>,
}

/// A literal relative media path, resolved and checked like a font asset.
#[derive(Clone, Debug)]
pub(crate) struct CheckedMediaAsset {
    pub(crate) path: String,
    pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedLayout {
    pub(crate) id: ViewId,
    pub(crate) scroll_style: Option<ExternFnId>,
    pub(crate) style_origins: Vec<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedText {
    pub(crate) id: ViewId,
    pub(crate) style: Option<ExternFnId>,
    /// Rich children in document order; empty for plain `text`.
    pub(crate) rich_children: Vec<CheckedRichChild>,
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedRichChild {
    Span {
        origin: OriginId,
    },
    For {
        items: CheckedExprUseId,
        item: CheckedLocalId,
        origin: OriginId,
        spans: Vec<OriginId>,
        /// The interaction expression indices checked under the loop-scoped
        /// environment — the only expressions allowed to read the item local.
        scoped_expressions: std::ops::Range<u32>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedInput {
    pub(crate) id: ViewId,
    pub(crate) binding: CheckedValueRef,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) icon_origin: Option<OriginId>,
    pub(crate) status_origins: Vec<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedButton {
    pub(crate) id: ViewId,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) status_origins: Vec<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedTextEditor {
    pub(crate) id: ViewId,
    pub(crate) binding: CheckedValueRef,
    pub(crate) highlighter: Option<ExternFnId>,
    pub(crate) key_binding: Option<ExternFnId>,
    pub(crate) action: Option<ExternFnId>,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) status_origins: Vec<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedMarkdown {
    pub(crate) id: ViewId,
    pub(crate) content: CheckedValueRef,
    pub(crate) viewer: Option<ExternFnId>,
    pub(crate) viewer_output: Type,
    pub(crate) style_origin: Option<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedExternComponent {
    pub(crate) id: ViewId,
    pub(crate) function: ExternFnId,
    pub(crate) output: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedExternViewAdapter {
    pub(crate) id: ViewId,
    pub(crate) function: ExternFnId,
    pub(crate) output: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedNestedTheme {
    pub(crate) id: ViewId,
    pub(crate) factory: Option<ExternFnId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedBooleanControl {
    pub(crate) id: ViewId,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) style_origin: Option<OriginId>,
    pub(crate) font_origin: Option<OriginId>,
    pub(crate) status_origins: Vec<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedPickList {
    pub(crate) id: ViewId,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) menu_style: Option<ExternFnId>,
    pub(crate) handle_origins: Vec<OriginId>,
    pub(crate) status_origins: Vec<OriginId>,
    pub(crate) menu_origin: Option<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedComboBox {
    pub(crate) id: ViewId,
    pub(crate) binding: CheckedValueRef,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) menu_style: Option<ExternFnId>,
    pub(crate) icon_origin: Option<OriginId>,
    pub(crate) status_origins: Vec<OriginId>,
    pub(crate) menu_origin: Option<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedSlider {
    pub(crate) id: ViewId,
    pub(crate) value_type: Type,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) style_origin: Option<OriginId>,
    pub(crate) status_origins: Vec<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedProgress {
    pub(crate) id: ViewId,
    pub(crate) style: Option<ExternFnId>,
    pub(crate) style_origin: Option<OriginId>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedRule {
    pub(crate) id: ViewId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedQrCode {
    pub(crate) id: ViewId,
    pub(crate) payload_type: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedSpace {
    pub(crate) id: ViewId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedTooltip {
    pub(crate) id: ViewId,
    pub(crate) expression_count: u32,
    pub(crate) semantic_key: String,
    pub(crate) style: Option<ExternFnId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedInteractionKind {
    ComponentCallRoutes,
    Layout,
    Text,
    RichText,
    Input,
    Button,
    TextEditor,
    Markdown,
    ExternComponent,
    Themer,
    Shader,
    NestedTheme,
    PickList,
    ComboBox,
    Slider,
    Progress,
    Checkbox,
    Toggler,
    Radio,
    Rule,
    QrCode,
    Space,
    MouseArea,
    ResizeHandle,
    Sensor,
    Overlay,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedInteractionRoute {
    pub(crate) id: InteractionRouteId,
    pub(crate) target: CheckedCanvasRouteTarget,
    pub(crate) args: Vec<CheckedCanvasRouteArg>,
    pub(crate) source_payloads: Vec<Type>,
    pub(crate) ordered_payloads: bool,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedInteraction {
    pub(crate) id: ViewId,
    pub(crate) kind: CheckedInteractionKind,
    pub(crate) semantic_key: String,
    pub(crate) expression_count: u32,
    pub(crate) option_expressions: Vec<CheckedExprUseId>,
    pub(crate) routes: Vec<CheckedInteractionRoute>,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedTask {
    pub(crate) id: TaskId,
    pub(crate) output: Option<Type>,
    pub(crate) error: Option<Type>,
    pub(crate) target: Option<CheckedEffectTarget>,
    pub(crate) is_final: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedEffectTarget {
    Builtin(String),
    Extern(ExternFnId),
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CheckedFactMetrics {
    pub(crate) values: usize,
    pub(crate) locals: usize,
    pub(crate) views: usize,
    pub(crate) component_slots: usize,
    pub(crate) component_slot_index_visits: usize,
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
    pub(crate) subscription_analysis_passes: usize,
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
    component_slots: Vec<Vec<CheckedComponentSlot>>,
    component_slots_by_view: HashMap<ViewId, ComponentSlotId>,
    canvases: HashMap<ViewId, CheckedCanvas>,
    media: HashMap<ViewId, CheckedMedia>,
    media_assets: Vec<CheckedMediaAsset>,
    containers: HashMap<ViewId, CheckedContainer>,
    layouts: HashMap<ViewId, CheckedLayout>,
    texts: HashMap<ViewId, CheckedText>,
    inputs: HashMap<ViewId, CheckedInput>,
    buttons: HashMap<ViewId, CheckedButton>,
    text_editors: HashMap<ViewId, CheckedTextEditor>,
    markdowns: HashMap<ViewId, CheckedMarkdown>,
    extern_components: HashMap<ViewId, CheckedExternComponent>,
    themers: HashMap<ViewId, CheckedExternViewAdapter>,
    shaders: HashMap<ViewId, CheckedExternViewAdapter>,
    nested_themes: HashMap<ViewId, CheckedNestedTheme>,
    boolean_controls: HashMap<ViewId, CheckedBooleanControl>,
    pick_lists: HashMap<ViewId, CheckedPickList>,
    combo_boxes: HashMap<ViewId, CheckedComboBox>,
    sliders: HashMap<ViewId, CheckedSlider>,
    progresses: HashMap<ViewId, CheckedProgress>,
    rules: HashMap<ViewId, CheckedRule>,
    qr_codes: HashMap<ViewId, CheckedQrCode>,
    spaces: HashMap<ViewId, CheckedSpace>,
    tooltips: HashMap<ViewId, CheckedTooltip>,
    interactions: HashMap<ViewId, CheckedInteraction>,
    pane_grids: HashMap<ViewId, CheckedPaneGrid>,
    subscriptions: Vec<CheckedSubscription>,
    expression_uses: Vec<CheckedExprUse>,
    expression_uses_by_owner: HashMap<CheckedExprOwner, CheckedExprUseId>,
    component_argument_sources:
        HashMap<(ComponentCallId, ComponentParamId), CheckedComponentArgumentSource>,
    component_call_routes: HashMap<ComponentCallId, CheckedComponentCallRoutes>,
    expressions: Vec<CheckedExpr>,
    handlers: Vec<CheckedHandler>,
    statements: Vec<Option<CheckedStatement>>,
    tasks: Vec<Option<CheckedTask>>,
    routes: Vec<Option<CheckedRoute>>,
    builtins: Vec<String>,
    app_theme_factory: Option<CheckedAppThemeFactory>,
    app_settings: Option<CheckedAppSettings>,
    app_setting_daemon_window_local: Option<CheckedLocalId>,
    #[cfg(test)]
    metrics: CheckedFactMetrics,
    #[cfg(test)]
    lookup_count: LookupCount,
    #[cfg(test)]
    component_slot_index_elapsed: Duration,
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
    pub(crate) default_font: Option<FontDecl>,
}

impl CheckedFacts {
    #[cfg(test)]
    pub(crate) fn corrupt_subscription_source(
        &mut self,
        index: usize,
        source: CheckedSubscriptionSource,
    ) {
        self.subscriptions[index].source = source;
    }

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
    pub(crate) fn corrupt_float_geometry_local(&mut self, float: ViewId, index: usize, raw: u32) {
        let CheckedViewFlow::Float { geometry, .. } = &mut self.views[float.0 as usize].flow else {
            panic!("test view must be a float");
        };
        geometry[index] = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_responsive_size_local(&mut self, view: ViewId, raw: u32) {
        let CheckedViewFlow::ResponsiveSize { width, .. } = &mut self.views[view.0 as usize].flow
        else {
            panic!("test view must be a size responsive");
        };
        *width = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_lazy_binding_local(&mut self, view: ViewId, raw: u32) {
        let CheckedViewFlow::Lazy { binding, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be lazy");
        };
        *binding = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_lazy_key_binding_local(&mut self, view: ViewId, index: usize, raw: u32) {
        let CheckedViewFlow::Lazy { key_bindings, .. } = &mut self.views[view.0 as usize].flow
        else {
            panic!("test view must be lazy");
        };
        key_bindings[index].local = Some(CheckedLocalId(raw));
    }

    #[cfg(test)]
    pub(crate) fn remove_lazy_key_binding_local(&mut self, view: ViewId, index: usize) {
        let CheckedViewFlow::Lazy { key_bindings, .. } = &mut self.views[view.0 as usize].flow
        else {
            panic!("test view must be lazy");
        };
        key_bindings[index].local = None;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_keyed_item_local(&mut self, view: ViewId, raw: u32) {
        let CheckedViewFlow::Keyed { item, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be keyed");
        };
        *item = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_table_row_local(&mut self, view: ViewId, raw: u32) {
        let CheckedViewFlow::Table { item, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be a table");
        };
        *item = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_pane_expression_id(&mut self, view: ViewId, raw: u32) {
        self.pane_grids.get_mut(&view).unwrap().spacing = Some(CheckedExprUseId(raw));
    }

    #[cfg(test)]
    pub(crate) fn leak_pane_template_key_into_spacing(&mut self, view: ViewId) {
        let pane = &self.pane_grids[&view];
        let spacing = pane.spacing.unwrap();
        let item = pane.templates[0].item;
        let root = self.expression_uses[spacing.0 as usize].root;
        self.expressions[root.0 as usize].ty = self.locals[item.0 as usize].ty.clone();
        self.expressions[root.0 as usize].kind = CheckedExprKind::Path {
            root: CheckedPathRoot::Local(item),
            projections: Vec::new(),
        };
    }

    #[cfg(test)]
    pub(crate) fn corrupt_pane_template_item_local(&mut self, view: ViewId, raw: u32) {
        self.pane_grids.get_mut(&view).unwrap().templates[0].item = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_pane_template_origin(&mut self, view: ViewId, raw: u32) {
        self.pane_grids.get_mut(&view).unwrap().templates[0].origin = OriginId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_interaction_route_id(&mut self, view: ViewId, index: usize, raw: u32) {
        self.interactions.get_mut(&view).unwrap().routes[index]
            .id
            .index = raw;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_interaction_route_origin(
        &mut self,
        view: ViewId,
        index: usize,
        raw: u32,
    ) {
        self.interactions.get_mut(&view).unwrap().routes[index].origin = OriginId(raw);
    }

    #[cfg(test)]
    pub(crate) fn swap_interaction_route_origins(
        &mut self,
        view: ViewId,
        first: usize,
        second: usize,
    ) {
        let routes = &mut self.interactions.get_mut(&view).unwrap().routes;
        let first_origin = routes[first].origin;
        routes[first].origin = routes[second].origin;
        routes[second].origin = first_origin;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_interaction_route_source_payload(
        &mut self,
        view: ViewId,
        route: usize,
        payload: usize,
        ty: Type,
    ) {
        self.interactions.get_mut(&view).unwrap().routes[route].source_payloads[payload] = ty;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_input_binding(&mut self, view: ViewId, binding: CheckedValueRef) {
        self.inputs.get_mut(&view).unwrap().binding = binding;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_button_style(&mut self, view: ViewId, style: ExternFnId) {
        self.buttons.get_mut(&view).unwrap().style = Some(style);
    }

    #[cfg(test)]
    pub(crate) fn swap_interaction_option_expressions(
        &mut self,
        view: ViewId,
        first: usize,
        second: usize,
    ) {
        self.interactions
            .get_mut(&view)
            .unwrap()
            .option_expressions
            .swap(first, second);
    }

    #[cfg(test)]
    pub(crate) fn transplant_interaction_option_expression(
        &mut self,
        destination_view: ViewId,
        destination_index: usize,
        source_view: ViewId,
        source_index: usize,
    ) {
        let source = self.interactions[&source_view].option_expressions[source_index];
        self.interactions
            .get_mut(&destination_view)
            .unwrap()
            .option_expressions[destination_index] = source;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_text_editor_binding(&mut self, view: ViewId, binding: CheckedValueRef) {
        self.text_editors.get_mut(&view).unwrap().binding = binding;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_markdown_content(&mut self, view: ViewId, content: CheckedValueRef) {
        self.markdowns.get_mut(&view).unwrap().content = content;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_markdown_viewer(&mut self, view: ViewId, viewer: ExternFnId) {
        self.markdowns.get_mut(&view).unwrap().viewer = Some(viewer);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_markdown_id(&mut self, view: ViewId, raw: u32) {
        self.markdowns.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_markdown_style_origin(&mut self, view: ViewId, raw: u32) {
        self.markdowns.get_mut(&view).unwrap().style_origin = Some(OriginId(raw));
    }

    #[cfg(test)]
    pub(crate) fn corrupt_markdown_viewer_output(&mut self, view: ViewId, output: Type) {
        self.markdowns.get_mut(&view).unwrap().viewer_output = output;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_extern_component_function(&mut self, view: ViewId, function: ExternFnId) {
        self.extern_components.get_mut(&view).unwrap().function = function;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_extern_component_output(&mut self, view: ViewId, output: Type) {
        self.extern_components.get_mut(&view).unwrap().output = output;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_extern_component_id(&mut self, view: ViewId, raw: u32) {
        self.extern_components.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_themer_function(&mut self, view: ViewId, function: ExternFnId) {
        self.themers.get_mut(&view).unwrap().function = function;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_themer_output(&mut self, view: ViewId, output: Type) {
        self.themers.get_mut(&view).unwrap().output = output;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_themer_id(&mut self, view: ViewId, raw: u32) {
        self.themers.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_shader_function(&mut self, view: ViewId, function: ExternFnId) {
        self.shaders.get_mut(&view).unwrap().function = function;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_shader_output(&mut self, view: ViewId, output: Type) {
        self.shaders.get_mut(&view).unwrap().output = output;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_shader_id(&mut self, view: ViewId, raw: u32) {
        self.shaders.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_nested_theme_factory(&mut self, view: ViewId, function: ExternFnId) {
        self.nested_themes.get_mut(&view).unwrap().factory = Some(function);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_nested_theme_id(&mut self, view: ViewId, raw: u32) {
        self.nested_themes.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_route_component(
        &mut self,
        call: ComponentCallId,
        component: ComponentId,
    ) {
        self.component_call_routes.get_mut(&call).unwrap().component = component;
    }

    #[cfg(test)]
    pub(crate) fn remove_component_slot(&mut self, component: ComponentId, index: usize) {
        self.component_slots[component.0 as usize].remove(index);
    }

    #[cfg(test)]
    pub(crate) fn duplicate_component_slot(&mut self, component: ComponentId, index: usize) {
        let slot = self.component_slots[component.0 as usize][index].clone();
        self.component_slots[component.0 as usize].push(slot);
    }

    #[cfg(test)]
    pub(crate) fn swap_component_slots(
        &mut self,
        component: ComponentId,
        left: usize,
        right: usize,
    ) {
        self.component_slots[component.0 as usize].swap(left, right);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_slot_origin(
        &mut self,
        component: ComponentId,
        index: usize,
        raw: u32,
    ) {
        self.component_slots[component.0 as usize][index].origin = OriginId(raw);
    }

    #[cfg(test)]
    pub(crate) fn swap_component_slot_origins(
        &mut self,
        component: ComponentId,
        left: usize,
        right: usize,
    ) {
        let slots = &mut self.component_slots[component.0 as usize];
        let left_origin = slots[left].origin;
        slots[left].origin = slots[right].origin;
        slots[right].origin = left_origin;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_slot_id(
        &mut self,
        component: ComponentId,
        index: usize,
        raw: u32,
    ) {
        self.component_slots[component.0 as usize][index].id.index = raw;
    }

    #[cfg(test)]
    pub(crate) fn swap_component_slot_views(
        &mut self,
        component: ComponentId,
        left: usize,
        right: usize,
    ) {
        let slots = &mut self.component_slots[component.0 as usize];
        let left_view = slots[left].view;
        slots[left].view = slots[right].view;
        slots[right].view = left_view;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_slot_view(
        &mut self,
        component: ComponentId,
        index: usize,
        raw: u32,
    ) {
        self.component_slots[component.0 as usize][index].view = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn swap_component_slot_reverse_associations(
        &mut self,
        component: ComponentId,
        left: usize,
        right: usize,
    ) {
        let slots = &self.component_slots[component.0 as usize];
        let left_view = slots[left].view;
        let right_view = slots[right].view;
        let left_slot = self.component_slots_by_view[&left_view];
        let right_slot = self.component_slots_by_view[&right_view];
        self.component_slots_by_view.insert(left_view, right_slot);
        self.component_slots_by_view.insert(right_view, left_slot);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_slot_reverse_association(
        &mut self,
        component: ComponentId,
        index: usize,
        id: ComponentSlotId,
    ) {
        let view = self.component_slots[component.0 as usize][index].view;
        self.component_slots_by_view.insert(view, id);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_route_view(&mut self, call: ComponentCallId, raw: u32) {
        self.component_call_routes.get_mut(&call).unwrap().view = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_event_id(
        &mut self,
        call: ComponentCallId,
        event: usize,
        id: ComponentEventId,
    ) {
        self.component_call_routes.get_mut(&call).unwrap().events[event].event = id;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_event_origin(
        &mut self,
        call: ComponentCallId,
        event: usize,
        raw: u32,
    ) {
        self.component_call_routes.get_mut(&call).unwrap().events[event].origin = OriginId(raw);
    }

    #[cfg(test)]
    pub(crate) fn swap_component_call_direct_route_origins(
        &mut self,
        call: ComponentCallId,
        event: usize,
    ) {
        let routes = self.component_call_routes.get_mut(&call).unwrap();
        let output = routes.output.as_mut().unwrap();
        let CheckedComponentEventDelivery::Direct { origin, .. } =
            &mut routes.events[event].delivery
        else {
            panic!("test event route must be direct");
        };
        std::mem::swap(&mut output.origin, origin);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_forward_outer_component(
        &mut self,
        call: ComponentCallId,
        event: usize,
        component: ComponentId,
    ) {
        let CheckedComponentEventDelivery::Forward {
            outer_component, ..
        } = &mut self.component_call_routes.get_mut(&call).unwrap().events[event].delivery
        else {
            panic!("test event route must be forwarded");
        };
        *outer_component = component;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_forward_outer_event(
        &mut self,
        call: ComponentCallId,
        event: usize,
        outer_event: ComponentEventId,
    ) {
        let CheckedComponentEventDelivery::Forward {
            outer_event: id, ..
        } = &mut self.component_call_routes.get_mut(&call).unwrap().events[event].delivery
        else {
            panic!("test event route must be forwarded");
        };
        *id = outer_event;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_forward_outer_component_name(
        &mut self,
        call: ComponentCallId,
        event: usize,
        name: &str,
    ) {
        let CheckedComponentEventDelivery::Forward {
            outer_component_name,
            ..
        } = &mut self.component_call_routes.get_mut(&call).unwrap().events[event].delivery
        else {
            panic!("test event route must be forwarded");
        };
        *outer_component_name = name.to_owned();
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_forward_outer_event_name(
        &mut self,
        call: ComponentCallId,
        event: usize,
        name: &str,
    ) {
        let CheckedComponentEventDelivery::Forward {
            outer_event_name, ..
        } = &mut self.component_call_routes.get_mut(&call).unwrap().events[event].delivery
        else {
            panic!("test event route must be forwarded");
        };
        *outer_event_name = name.to_owned();
    }

    #[cfg(test)]
    pub(crate) fn corrupt_component_call_forward_outer_payload(
        &mut self,
        call: ComponentCallId,
        event: usize,
        payload: Type,
    ) {
        let CheckedComponentEventDelivery::Forward { outer_payloads, .. } =
            &mut self.component_call_routes.get_mut(&call).unwrap().events[event].delivery
        else {
            panic!("test event route must be forwarded");
        };
        outer_payloads[0] = payload;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_interaction_expression_origin(
        &mut self,
        view: ViewId,
        index: u32,
        raw: u32,
    ) {
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget: view,
            index,
        });
        let expression = self.expression_uses_by_owner[&owner];
        self.expression_uses[expression.0 as usize].origin = OriginId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_interaction_expression_count(&mut self, view: ViewId, count: u32) {
        self.interactions.get_mut(&view).unwrap().expression_count = count;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_boolean_style(&mut self, view: ViewId, style: ExternFnId) {
        self.boolean_controls.get_mut(&view).unwrap().style = Some(style);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_boolean_control_id(&mut self, view: ViewId, raw: u32) {
        self.boolean_controls.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn swap_interaction_routes(&mut self, view: ViewId, other: ViewId) {
        let first = self.interactions.get(&view).unwrap().routes[0].clone();
        let second = self.interactions.get(&other).unwrap().routes[0].clone();
        self.interactions.get_mut(&view).unwrap().routes[0] = second;
        self.interactions.get_mut(&other).unwrap().routes[0] = first;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_combo_box_binding(&mut self, view: ViewId, binding: CheckedValueRef) {
        self.combo_boxes.get_mut(&view).unwrap().binding = binding;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_pick_list_style(&mut self, view: ViewId, function: ExternFnId) {
        self.pick_lists.get_mut(&view).unwrap().style = Some(function);
    }

    #[cfg(test)]
    pub(crate) fn swap_pick_list_status_origins(&mut self, view: ViewId) {
        self.pick_lists
            .get_mut(&view)
            .unwrap()
            .status_origins
            .swap(0, 1);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_interaction_route_handler(
        &mut self,
        view: ViewId,
        route: usize,
        handler: HandlerId,
    ) {
        self.interactions.get_mut(&view).unwrap().routes[route].target =
            CheckedCanvasRouteTarget::Handler(handler);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_slider_style(&mut self, view: ViewId, style: ExternFnId) {
        self.sliders.get_mut(&view).unwrap().style = Some(style);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_progress_style(&mut self, view: ViewId, style: ExternFnId) {
        self.progresses.get_mut(&view).unwrap().style = Some(style);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_slider_id(&mut self, view: ViewId, raw: u32) {
        self.sliders.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_progress_id(&mut self, view: ViewId, raw: u32) {
        self.progresses.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_rule_id(&mut self, view: ViewId, raw: u32) {
        self.rules.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_qr_code_id(&mut self, view: ViewId, raw: u32) {
        self.qr_codes.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_qr_payload_type(&mut self, view: ViewId, payload_type: Type) {
        self.qr_codes.get_mut(&view).unwrap().payload_type = payload_type;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_space_id(&mut self, view: ViewId, raw: u32) {
        self.spaces.get_mut(&view).unwrap().id = ViewId(raw);
    }

    #[cfg(test)]
    pub(crate) fn transplant_interaction_route_expression(
        &mut self,
        view: ViewId,
        destination_route: usize,
        destination_argument: usize,
        source_route: usize,
        source_argument: usize,
    ) {
        let source = self.interactions[&view].routes[source_route].args[source_argument];
        self.interactions.get_mut(&view).unwrap().routes[destination_route].args
            [destination_argument] = source;
    }

    #[cfg(test)]
    pub(crate) fn transplant_interaction_route_expression_across_views(
        &mut self,
        destination_view: ViewId,
        destination_route: usize,
        destination_argument: usize,
        source_view: ViewId,
        source_route: usize,
        source_argument: usize,
    ) {
        let source = self.interactions[&source_view].routes[source_route].args[source_argument];
        self.interactions.get_mut(&destination_view).unwrap().routes[destination_route].args
            [destination_argument] = source;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_for_item_local(&mut self, view: ViewId, raw: u32) {
        let CheckedViewFlow::For { item, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be for");
        };
        *item = CheckedLocalId(raw);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_match_binding_local(&mut self, view: ViewId, arm: usize, raw: u32) {
        let CheckedViewFlow::Match { arms, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be match");
        };
        arms[arm].binding = Some(CheckedLocalId(raw));
    }

    #[cfg(test)]
    pub(crate) fn corrupt_match_pattern(
        &mut self,
        view: ViewId,
        arm: usize,
        pattern: CheckedMatchPattern,
    ) {
        let CheckedViewFlow::Match { arms, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be match");
        };
        arms[arm].pattern = pattern;
    }

    #[cfg(test)]
    pub(crate) fn remove_match_arm(&mut self, view: ViewId, arm: usize) {
        let CheckedViewFlow::Match { arms, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be match");
        };
        arms.remove(arm);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_match_arm_origin(&mut self, view: ViewId, arm: usize, origin: OriginId) {
        let CheckedViewFlow::Match { arms, .. } = &mut self.views[view.0 as usize].flow else {
            panic!("test view must be match");
        };
        arms[arm].origin = origin;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_local_type(&mut self, local: CheckedLocalId, ty: Type) {
        self.locals[local.0 as usize].ty = ty;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_responsive_dimension(
        &mut self,
        view: ViewId,
        index: usize,
        replacement: CheckedLength,
    ) {
        let CheckedViewFlow::ResponsiveSize { dimensions, .. } =
            &mut self.views[view.0 as usize].flow
        else {
            panic!("test view must be responsive");
        };
        dimensions[index] = replacement;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_responsive_expression_count(&mut self, view: ViewId, value: u32) {
        let CheckedViewFlow::ResponsiveSize {
            expression_count, ..
        } = &mut self.views[view.0 as usize].flow
        else {
            panic!("test view must be responsive");
        };
        *expression_count = value;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_responsive_dimension_expression_role(
        &mut self,
        view: ViewId,
        target_index: usize,
        source_role: CheckedViewExprRole,
    ) {
        let source = self.expression_uses_by_owner[&CheckedExprOwner::View {
            view,
            role: source_role,
        }];
        let CheckedViewFlow::ResponsiveSize { dimensions, .. } =
            &mut self.views[view.0 as usize].flow
        else {
            panic!("test view must be responsive");
        };
        let CheckedLength::Fixed { expression, .. } = &mut dimensions[target_index] else {
            panic!("test dimension must be fixed");
        };
        *expression = source;
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
        let value = self
            .values
            .get(self.values_by_ref.get(&value_ref)?.0 as usize)?;
        (value.id == value_ref).then_some(value)
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
        self.app_setting_daemon_window_local
    }

    pub(crate) fn views(&self) -> &[CheckedView] {
        &self.views
    }

    pub(crate) fn view(&self, id: ViewId) -> &CheckedView {
        self.record_lookup();
        &self.views[id.0 as usize]
    }

    pub(crate) fn try_view(&self, id: ViewId) -> Option<&CheckedView> {
        self.record_lookup();
        self.views.get(id.0 as usize).filter(|view| view.id == id)
    }

    pub(crate) fn component_slots(
        &self,
        component: ComponentId,
    ) -> Option<&[CheckedComponentSlot]> {
        self.record_lookup();
        self.component_slots
            .get(component.0 as usize)
            .map(Vec::as_slice)
    }

    pub(crate) fn component_slot(&self, id: ComponentSlotId) -> Option<&CheckedComponentSlot> {
        self.record_lookup();
        self.component_slots
            .get(id.component.0 as usize)?
            .get(id.index as usize)
            .filter(|slot| slot.id == id)
    }

    pub(crate) fn component_slot_for_view(&self, view: ViewId) -> Option<&CheckedComponentSlot> {
        self.record_lookup();
        let id = self.component_slots_by_view.get(&view)?;
        self.component_slot(*id).filter(|slot| slot.view == view)
    }

    pub(crate) fn subscriptions(&self) -> &[CheckedSubscription] {
        &self.subscriptions
    }

    pub(crate) fn canvas(&self, id: ViewId) -> Option<&CheckedCanvas> {
        self.canvases.get(&id).filter(|canvas| canvas.id == id)
    }

    pub(crate) fn media(&self, id: ViewId) -> Option<&CheckedMedia> {
        self.media.get(&id).filter(|media| media.id == id)
    }

    pub(crate) fn media_assets(&self) -> &[CheckedMediaAsset] {
        &self.media_assets
    }

    /// Retains a literal media path so it is checked and watched like a font.
    ///
    /// Absolute literals stay runtime filesystem references, so only portable
    /// relative literals become compile-time assets.
    fn record_media_asset(&mut self, path: &str, span: &Span) {
        if crate::is_relative_asset_path(path) {
            self.media_assets.push(CheckedMediaAsset {
                path: path.to_owned(),
                span: span.clone(),
            });
        }
    }

    pub(crate) fn container(&self, id: ViewId) -> Option<&CheckedContainer> {
        self.containers
            .get(&id)
            .filter(|container| container.id == id)
    }

    pub(crate) fn layout(&self, id: ViewId) -> Option<&CheckedLayout> {
        self.layouts.get(&id).filter(|layout| layout.id == id)
    }

    pub(crate) fn text(&self, id: ViewId) -> Option<&CheckedText> {
        self.texts.get(&id).filter(|text| text.id == id)
    }

    pub(crate) fn input(&self, id: ViewId) -> Option<&CheckedInput> {
        self.inputs.get(&id).filter(|input| input.id == id)
    }

    pub(crate) fn button(&self, id: ViewId) -> Option<&CheckedButton> {
        self.buttons.get(&id).filter(|button| button.id == id)
    }

    pub(crate) fn text_editor(&self, id: ViewId) -> Option<&CheckedTextEditor> {
        self.text_editors.get(&id).filter(|editor| editor.id == id)
    }

    pub(crate) fn markdown(&self, id: ViewId) -> Option<&CheckedMarkdown> {
        self.markdowns.get(&id).filter(|markdown| markdown.id == id)
    }

    pub(crate) fn extern_component(&self, id: ViewId) -> Option<&CheckedExternComponent> {
        self.extern_components
            .get(&id)
            .filter(|component| component.id == id)
    }

    pub(crate) fn themer(&self, id: ViewId) -> Option<&CheckedExternViewAdapter> {
        self.themers.get(&id).filter(|themer| themer.id == id)
    }

    pub(crate) fn shader(&self, id: ViewId) -> Option<&CheckedExternViewAdapter> {
        self.shaders.get(&id).filter(|shader| shader.id == id)
    }

    pub(crate) fn nested_theme(&self, id: ViewId) -> Option<&CheckedNestedTheme> {
        self.nested_themes.get(&id).filter(|theme| theme.id == id)
    }

    pub(crate) fn boolean_control(&self, id: ViewId) -> Option<&CheckedBooleanControl> {
        self.boolean_controls
            .get(&id)
            .filter(|control| control.id == id)
    }

    pub(crate) fn pick_list(&self, id: ViewId) -> Option<&CheckedPickList> {
        self.pick_lists.get(&id).filter(|pick| pick.id == id)
    }

    pub(crate) fn combo_box(&self, id: ViewId) -> Option<&CheckedComboBox> {
        self.combo_boxes.get(&id).filter(|combo| combo.id == id)
    }

    pub(crate) fn slider(&self, id: ViewId) -> Option<&CheckedSlider> {
        self.sliders.get(&id).filter(|slider| slider.id == id)
    }

    pub(crate) fn progress(&self, id: ViewId) -> Option<&CheckedProgress> {
        self.progresses
            .get(&id)
            .filter(|progress| progress.id == id)
    }

    pub(crate) fn rule(&self, id: ViewId) -> Option<&CheckedRule> {
        self.rules.get(&id).filter(|rule| rule.id == id)
    }

    pub(crate) fn qr_code(&self, id: ViewId) -> Option<&CheckedQrCode> {
        self.qr_codes.get(&id).filter(|qr| qr.id == id)
    }

    pub(crate) fn space(&self, id: ViewId) -> Option<&CheckedSpace> {
        self.spaces.get(&id).filter(|space| space.id == id)
    }

    pub(crate) fn tooltip(&self, id: ViewId) -> Option<&CheckedTooltip> {
        self.tooltips.get(&id).filter(|tooltip| tooltip.id == id)
    }

    pub(crate) fn interaction(&self, id: ViewId) -> Option<&CheckedInteraction> {
        self.interactions
            .get(&id)
            .filter(|interaction| interaction.id == id)
    }

    pub(crate) fn pane_grid(&self, id: ViewId) -> Option<&CheckedPaneGrid> {
        self.pane_grids.get(&id).filter(|pane| pane.id == id)
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

    pub(crate) fn component_call_routes(
        &self,
        call: ComponentCallId,
    ) -> Option<&CheckedComponentCallRoutes> {
        self.component_call_routes
            .get(&call)
            .filter(|routes| routes.call == call)
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
    pub(crate) fn corrupt_app_setting_binary_child(&mut self, id: AppSettingExprId) {
        let expression_use = self.expression_uses_by_owner[&CheckedExprOwner::AppSetting(id)];
        let root = self.expression_uses[expression_use.0 as usize].root;
        let CheckedExprKind::Binary { left, .. } = &mut self.expressions[root.0 as usize].kind
        else {
            panic!("app-setting fixture root must be binary");
        };
        *left = CheckedExprId(u32::MAX);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_app_setting_palette_id(&mut self) {
        let expression_use =
            self.expression_uses_by_owner[&CheckedExprOwner::AppSetting(AppSettingExprId::Palette)];
        let root = self.expression_uses[expression_use.0 as usize].root;
        self.expressions[root.0 as usize].kind = CheckedExprKind::Path {
            root: CheckedPathRoot::Palette(PaletteId(u32::MAX)),
            projections: Vec::new(),
        };
    }

    #[cfg(test)]
    pub(crate) fn corrupt_app_setting_daemon_window_owner(&mut self) {
        let setting = self
            .locals
            .iter()
            .position(|local| local.owner == CheckedLocalOwner::AppSettingDaemonWindow)
            .expect("app-setting fixture must have a daemon-window local");
        let owner = self
            .locals
            .iter()
            .find_map(|local| match local.owner {
                CheckedLocalOwner::View {
                    role: CheckedViewLocalRole::DaemonWindow,
                    ..
                } => Some(local.owner),
                _ => None,
            })
            .expect("app-setting fixture must have a view daemon-window local");
        self.locals[setting].owner = owner;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_app_setting_builtin_target(
        &mut self,
        setting: AppSettingExprId,
        replacement: &str,
    ) {
        let replacement = CheckedBuiltinId(
            self.builtins
                .iter()
                .position(|name| name == replacement)
                .expect("replacement builtin must be interned") as u32,
        );
        let expression_use = self.expression_uses_by_owner[&CheckedExprOwner::AppSetting(setting)];
        let root = self.expression_uses[expression_use.0 as usize].root;
        let CheckedExprKind::Call { target, .. } = &mut self.expressions[root.0 as usize].kind
        else {
            panic!("app-setting fixture root must be a call");
        };
        *target = CheckedCallTarget::Builtin(replacement);
    }

    #[cfg(test)]
    pub(crate) fn corrupt_app_setting_binding_body_argument(
        &mut self,
        setting: AppSettingExprId,
        replacement: usize,
    ) {
        let expression = self.expression_uses_by_owner[&CheckedExprOwner::AppSetting(setting)];
        let local = self
            .locals
            .iter_mut()
            .find(|local| {
                matches!(
                    local.owner,
                    CheckedLocalOwner::ExpressionBinding {
                        expression: owner,
                        ..
                    } if owner == expression
                )
            })
            .expect("app-setting fixture must have an expression binding");
        let CheckedLocalOwner::ExpressionBinding { body_argument, .. } = &mut local.owner else {
            unreachable!();
        };
        *body_argument = replacement;
    }

    #[cfg(test)]
    pub(crate) fn corrupt_app_setting_sibling_scoped_binding_reference(
        &mut self,
        setting: AppSettingExprId,
    ) {
        let expression_use = self.expression_uses_by_owner[&CheckedExprOwner::AppSetting(setting)];
        let root = self.expression_uses[expression_use.0 as usize].root;
        let (left, right) = match self.expressions[root.0 as usize].kind {
            CheckedExprKind::Binary { left, right, .. } => (left, right),
            _ => panic!("app-setting fixture root must be binary"),
        };
        let left_binding = match &self.expressions[left.0 as usize].kind {
            CheckedExprKind::Call { arguments, .. } => arguments
                .iter()
                .find_map(|argument| match argument {
                    CheckedCallArgument::Binding(local) => Some(*local),
                    CheckedCallArgument::Value(_) => None,
                })
                .expect("left call must contain a scoped binding"),
            _ => panic!("left binary child must be a call"),
        };
        let (right_binding, right_arguments) = match &self.expressions[right.0 as usize].kind {
            CheckedExprKind::Call { arguments, .. } => (
                arguments
                    .iter()
                    .find_map(|argument| match argument {
                        CheckedCallArgument::Binding(local) => Some(*local),
                        CheckedCallArgument::Value(_) => None,
                    })
                    .expect("right call must contain a scoped binding"),
                arguments.clone(),
            ),
            _ => panic!("right binary child must be a call"),
        };
        let CheckedLocalOwner::ExpressionBinding { body_argument, .. } =
            self.locals[right_binding.0 as usize].owner
        else {
            panic!("right call binding must retain its body argument");
        };
        let CheckedCallArgument::Value(right_body) = right_arguments[body_argument] else {
            panic!("right scoped body must be a value expression");
        };
        self.expressions[right_body.0 as usize].kind = CheckedExprKind::Path {
            root: CheckedPathRoot::Local(left_binding),
            projections: Vec::new(),
        };
    }

    #[cfg(test)]
    pub(crate) fn app_setting_daemon_window_local_count(&self) -> usize {
        self.locals
            .iter()
            .filter(|local| matches!(local.owner, CheckedLocalOwner::AppSettingDaemonWindow))
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

    pub(crate) fn self_move_contract(
        &self,
        expression: CheckedExprUseId,
        target: CheckedValueRef,
        declarations: &DeclarationIndex,
    ) -> Result<bool, (OriginId, &'static str)> {
        let expression_use = self.try_expression_use(expression).ok_or((
            OriginId(u32::MAX),
            "self-move expression-use ID is outside its arena",
        ))?;
        let root = self.try_expression(expression_use.root).ok_or((
            expression_use.origin,
            "self-move expression root ID is outside its arena",
        ))?;
        let CheckedExprKind::Call {
            target: CheckedCallTarget::Extern(reference),
            ..
        } = &root.kind
        else {
            return Ok(false);
        };
        let function = declarations.try_extern_decl(reference.id).ok_or((
            root.origin,
            "self-move expression extern ID is outside its arena",
        ))?;
        if function.name != reference.name
            || !matches!(function.kind, ExternKind::Pure | ExternKind::Sync)
        {
            return Ok(false);
        }

        let mut occurrences = 0usize;
        let mut stack = vec![expression_use.root];
        while let Some(id) = stack.pop() {
            let expression = self.try_expression(id).ok_or((
                expression_use.origin,
                "self-move expression descendant ID is outside its arena",
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

    pub(crate) fn try_handler(&self, id: HandlerId) -> Option<&CheckedHandler> {
        self.record_lookup();
        self.handlers.get(id.0 as usize)
    }

    #[cfg(test)]
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

    pub(crate) fn try_route(&self, id: RouteId) -> Option<&CheckedRoute> {
        self.record_lookup();
        self.routes.get(id.0 as usize)?.as_ref()
    }

    pub(crate) fn try_task(&self, id: TaskId) -> Option<&CheckedTask> {
        self.record_lookup();
        self.tasks.get(id.0 as usize)?.as_ref()
    }

    pub(crate) fn try_builtin(&self, id: CheckedBuiltinId) -> Option<&str> {
        self.record_lookup();
        self.builtins.get(id.0 as usize).map(String::as_str)
    }

    #[cfg(test)]
    pub(crate) fn metrics(&self) -> CheckedFactMetrics {
        self.metrics
    }

    #[cfg(test)]
    pub(crate) fn component_slot_index_elapsed(&self) -> Duration {
        self.component_slot_index_elapsed
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

/// The per-widget analysis stores were eight parallel maps with one key shape;
/// the site tag moved into the key so one map, one `retain`, and one leftover
/// scan serve them all.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum AnalysisSite {
    Canvas,
    Media,
    Tooltip,
    Float,
    Pin,
    Interaction,
}

impl AnalysisSite {
    fn label(self) -> &'static str {
        match self {
            AnalysisSite::Canvas => "canvas",
            AnalysisSite::Media => "media",
            AnalysisSite::Tooltip => "tooltip",
            AnalysisSite::Float => "float",
            AnalysisSite::Pin => "pin",
            AnalysisSite::Interaction => "interaction",
        }
    }

    /// Canvases and interaction widgets are the only sites whose expressions
    /// may carry route payload contracts.
    fn keeps_routes(self) -> bool {
        matches!(self, AnalysisSite::Canvas | AnalysisSite::Interaction)
    }
}

#[derive(Debug, Default)]
pub(super) struct CheckedAnalyses {
    entries: HashMap<CheckedExprOwner, ExprTypeAnalysis>,
    handler_entries: HashMap<usize, ExprTypeAnalysis>,
    handler_route_inputs: HashMap<usize, super::expr::CapturedRouteInputs>,
    preset_handlers: Vec<Handler>,
    #[cfg(test)]
    pub(super) view_scope_env_overlays: usize,
    #[cfg(test)]
    pub(super) view_scope_env_full_clones: usize,
    subscriptions: HashMap<SubscriptionId, CheckedSubscriptionAnalysis>,
    test_entries: HashMap<usize, ExprTypeAnalysis>,
    site_entries: HashMap<(AnalysisSite, ViewId, usize), ExprTypeAnalysis>,
    site_route_inputs: HashMap<(AnalysisSite, ViewId, usize), super::expr::CapturedRouteInputs>,
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
            && self.subscriptions.is_empty()
            && self.test_entries.is_empty()
            && self.site_entries.is_empty()
            && self.site_route_inputs.is_empty()
    }

    fn site_expressions(&self, site: AnalysisSite, view: ViewId) -> usize {
        self.site_entries
            .keys()
            .filter(|(entry, owner, _)| *entry == site && *owner == view)
            .count()
    }

    fn site_routes(&self, site: AnalysisSite, view: ViewId) -> usize {
        self.site_route_inputs
            .keys()
            .filter(|(entry, owner, _)| *entry == site && *owner == view)
            .count()
    }

    pub(super) fn insert_subscription(
        &mut self,
        id: SubscriptionId,
        analysis: CheckedSubscriptionAnalysis,
    ) -> Result<(), Error> {
        if self.subscriptions.insert(id, analysis).is_some() {
            return Err(Error::new(
                "E196",
                &Span::line(1),
                "subscription was analyzed more than once",
            ));
        }
        Ok(())
    }

    fn remove_subscription(&mut self, id: SubscriptionId) -> Option<CheckedSubscriptionAnalysis> {
        self.subscriptions.remove(&id)
    }

    pub(super) fn extend(&mut self, other: Self) -> Result<(), Error> {
        #[cfg(test)]
        {
            self.view_scope_env_overlays += other.view_scope_env_overlays;
            self.view_scope_env_full_clones += other.view_scope_env_full_clones;
        }
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
        for (id, analysis) in other.subscriptions {
            self.insert_subscription(id, analysis)?;
        }
        for (key, analysis) in other.test_entries {
            if self.test_entries.insert(key, analysis).is_some() {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    "test expression was captured more than once",
                ));
            }
        }
        for (key, analysis) in other.site_entries {
            if self.site_entries.insert(key, analysis).is_some() {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    format!("{} expression was captured more than once", key.0.label()),
                ));
            }
        }
        for (key, inputs) in other.site_route_inputs {
            if self.site_route_inputs.insert(key, inputs).is_some() {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    format!(
                        "{} route contract was captured more than once",
                        key.0.label()
                    ),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn retain(
        &mut self,
        site: AnalysisSite,
        view: ViewId,
        analyses: super::expr::HandlerAnalyses,
    ) -> Result<(), Error> {
        if !site.keeps_routes() && !analyses.routes.is_empty() {
            return Err(Error::new(
                "E196",
                &Span::line(1),
                format!(
                    "{} expression capture unexpectedly retained routes",
                    site.label()
                ),
            ));
        }
        for (key, analysis) in analyses.expressions {
            if self
                .site_entries
                .insert((site, view, key), analysis)
                .is_some()
            {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    format!("{} expression was captured more than once", site.label()),
                ));
            }
        }
        for (key, inputs) in analyses.routes {
            if self
                .site_route_inputs
                .insert((site, view, key), inputs)
                .is_some()
            {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    format!(
                        "{} route contract was captured more than once",
                        site.label()
                    ),
                ));
            }
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

    pub(super) fn retain_tests(
        &mut self,
        analyses: super::expr::HandlerAnalyses,
    ) -> Result<(), Error> {
        if !analyses.routes.is_empty() {
            return Err(Error::new(
                "E196",
                &Span::line(1),
                "test checking captured an unexpected handler route",
            ));
        }
        for (key, analysis) in analyses.expressions {
            if self.test_entries.insert(key, analysis).is_some() {
                return Err(Error::new(
                    "E196",
                    &Span::line(1),
                    "test expression was captured more than once",
                ));
            }
        }
        Ok(())
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
    /// Keyed-lazy value chains that ended at a component prop, awaiting the
    /// per-call argument validation that runs after every view has lowered.
    pending_keyed_lazy_props: Vec<ComponentParamId>,
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

    #[cfg(test)]
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

struct BooleanControlFactSource<'a> {
    kind: CheckedInteractionKind,
    semantic_key: String,
    expressions: Vec<&'a Expr>,
    statuses: Vec<(&'a Span, Vec<&'a Expr>)>,
    style: Option<(&'a str, ExternKind)>,
    has_font: bool,
    route: &'a Route,
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

struct ScopedFactEnv<'a> {
    base: &'a dyn FactEnvironment,
    entries: FactEnv,
}

impl<'a> ScopedFactEnv<'a> {
    fn new(base: &'a dyn FactEnvironment) -> Self {
        Self {
            base,
            entries: FactEnv::default(),
        }
    }

    fn insert(&mut self, name: String, root: CheckedPathRoot, ty: Type) {
        self.entries.insert(name, root, ty);
    }
}

impl FactEnvironment for ScopedFactEnv<'_> {
    fn get(&self, name: &str) -> Option<&(CheckedPathRoot, Type)> {
        self.entries.get(name).or_else(|| self.base.get(name))
    }

    fn slot(&self, name: &str) -> Option<ComponentSlotId> {
        self.entries.slot(name).or_else(|| self.base.slot(name))
    }
}

impl ExprTypeEnv for ScopedFactEnv<'_> {
    fn get_type(&self, name: &str) -> Option<&Type> {
        self.entries
            .get_type(name)
            .or_else(|| self.base.get_type(name))
    }

    fn visit_types(&self, visitor: &mut dyn FnMut(&str, &Type)) {
        self.base.visit_types(visitor);
        self.entries.visit_types(visitor);
    }

    fn type_with_prefix(&self, prefix: &str) -> Option<&Type> {
        self.entries
            .type_with_prefix(prefix)
            .or_else(|| self.base.type_with_prefix(prefix))
    }
}

#[derive(Clone, Copy)]
struct ExpressionLowering<'a> {
    analysis: &'a ExprTypeAnalysis,
    owner: CheckedExprUseId,
    origin: OriginId,
    span: &'a Span,
}

#[derive(Default)]
struct CanvasFactCounters {
    expression: u32,
    command: u32,
    route: u32,
}

impl<'a> FactsBuilder<'a> {
    fn new(
        document: &'a Document,
        declarations: &'a DeclarationIndex,
        origins: &'a mut OriginArena,
        analyses: CheckedAnalyses,
    ) -> Self {
        let facts = CheckedFacts::default();
        #[cfg(test)]
        let mut facts = facts;
        record_fact_metric!(
            facts.metrics.view_scope_env_overlays = analyses.view_scope_env_overlays
        );
        record_fact_metric!(
            facts.metrics.view_scope_env_full_clones = analyses.view_scope_env_full_clones
        );
        Self {
            document,
            declarations,
            origins,
            facts,
            values_by_scope: HashMap::new(),
            builtins_by_name: HashMap::new(),
            dynamic_pane_grids: crate::hir::dynamic_pane_grids(document),
            analyses,
            pending_keyed_lazy_props: Vec::new(),
        }
    }

    fn build(mut self) -> Result<CheckedFacts, Error> {
        self.facts.app_settings = Some(CheckedAppSettings {
            app_name: self.document.app.clone(),
            daemon: self.document.daemon,
            source: self.document.settings.clone(),
            default_font: self
                .document
                .fonts
                .iter()
                .find(|font| font.default)
                .cloned(),
        });
        self.index_values()?;
        self.lower_initializers()?;
        self.lower_app_setting_expressions()?;
        self.index_views()?;
        self.index_component_slots()?;
        self.lower_view_expressions()?;
        self.lower_test_expressions()?;
        self.lower_subscriptions()?;
        self.lower_handler_expressions()?;
        record_fact_metric!(
            self.facts.metrics.handler_auxiliary_analyses = self.analyses.handler_entries.len()
        );
        self.analyses.handler_entries.clear();
        if !self.analyses.is_empty() {
            let expressions = |site| {
                self.analyses
                    .site_entries
                    .keys()
                    .filter(|(entry, _, _)| *entry == site)
                    .count()
            };
            let routes = |site| {
                self.analyses
                    .site_route_inputs
                    .keys()
                    .filter(|(entry, _, _)| *entry == site)
                    .count()
            };
            return Err(self.invariant(
                &Span::line(1),
                format!(
                    "checked analyses were not consumed (expressions={}, subscriptions={}, test_expressions={}, canvas_expressions={}, canvas_routes={}, media_expressions={}, tooltip_expressions={}, float_expressions={}, pin_expressions={}, interaction_expressions={}, interaction_routes={}, handler_expressions={}, handler_routes={}, presets={})",
                    self.analyses.entries.len(),
                    self.analyses.subscriptions.len(),
                    self.analyses.test_entries.len(),
                    expressions(AnalysisSite::Canvas),
                    routes(AnalysisSite::Canvas),
                    expressions(AnalysisSite::Media),
                    expressions(AnalysisSite::Tooltip),
                    expressions(AnalysisSite::Float),
                    expressions(AnalysisSite::Pin),
                    expressions(AnalysisSite::Interaction),
                    routes(AnalysisSite::Interaction),
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
        record_fact_metric!(self.facts.metrics.values = self.facts.values.len());
        record_fact_metric!(self.facts.metrics.locals = self.facts.locals.len());
        record_fact_metric!(self.facts.metrics.views = self.facts.views.len());
        record_fact_metric!(
            self.facts.metrics.component_slots =
                self.facts.component_slots.iter().map(Vec::len).sum()
        );
        record_fact_metric!(self.facts.metrics.expression_uses = self.facts.expression_uses.len());
        record_fact_metric!(self.facts.metrics.expressions = self.facts.expressions.len());
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
        for (index, secret) in self.document.secrets.iter().enumerate() {
            let declaration = self.declarations.secret(index);
            self.push_value(
                ValueScope::App,
                secret.name.clone(),
                Type::Secret,
                CheckedValueRef::Secret(declaration.id),
                declaration.origin,
                &secret.span,
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
        record_fact_metric!(self.facts.metrics.scope_env_builds += 1);
        record_fact_metric!(self.facts.metrics.scope_env_entries += app_env.len());
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
            && self
                .document
                .settings
                .tray
                .as_ref()
                .is_none_or(|tray| !tray.reactive())
        {
            return Ok(());
        }
        let mut app_env = FactEnv::default();
        for (index, state) in self.document.states.iter().enumerate() {
            let value_ref = CheckedValueRef::AppState(self.declarations.app_state(index).id);
            let value = self.facts.value_by_ref(value_ref);
            app_env.insert(
                state.name.clone(),
                CheckedPathRoot::Value(value.id),
                value.ty.clone(),
            );
        }
        for (index, derived) in self.document.derived.iter().enumerate() {
            let value_ref = CheckedValueRef::Derived(self.declarations.derived(index).id);
            let value = self.facts.value_by_ref(value_ref);
            app_env.insert(
                derived.name.clone(),
                CheckedPathRoot::Value(value.id),
                value.ty.clone(),
            );
        }
        record_fact_metric!(self.facts.metrics.scope_env_builds += 1);
        record_fact_metric!(self.facts.metrics.scope_env_entries += app_env.len());

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
                    &app_env,
                    &setting.span,
                )?;
            }
        }

        if let Some(tray) = &self.document.settings.tray {
            for (id, setting) in [
                (AppSettingExprId::TrayLabel, &tray.label),
                (AppSettingExprId::TrayTooltip, &tray.tooltip),
            ] {
                if let Some(setting) = setting
                    .as_ref()
                    .filter(|setting| tray_text_is_reactive(setting))
                {
                    lower(
                        self,
                        id,
                        &setting.value,
                        &Type::Str,
                        &app_env,
                        &setting.span,
                    )?;
                }
            }
            for (index, icon) in tray.icons.iter().enumerate() {
                if let Some(guard) = &icon.when {
                    lower(
                        self,
                        AppSettingExprId::TrayIconGuard(index as u32),
                        &guard.value,
                        &Type::Bool,
                        &app_env,
                        &guard.span,
                    )?;
                }
            }
            for (index, row) in tray.menu.iter().enumerate() {
                let TrayRow::Item { text, when, .. } = row else {
                    continue;
                };
                if tray_text_is_reactive(text) {
                    lower(
                        self,
                        AppSettingExprId::TrayMenuRow(index as u32),
                        &text.value,
                        &Type::Str,
                        &app_env,
                        &text.span,
                    )?;
                }
                if let Some(guard) = when {
                    lower(
                        self,
                        AppSettingExprId::TrayRowGuard(index as u32),
                        &guard.value,
                        &Type::Bool,
                        &app_env,
                        &guard.span,
                    )?;
                }
            }
        }

        let mut callback_env = app_env;
        let has_callback_setting = self.document.settings.title.is_some()
            || self.document.settings.theme.is_some()
            || self.document.settings.palette.is_some()
            || self.document.settings.scale_factor.is_some();
        if self.document.daemon && has_callback_setting {
            let local = self.push_local(
                "window",
                Type::WindowId,
                CheckedLocalOwner::AppSettingDaemonWindow,
                &self.document.settings.span,
                self.declarations.app_settings().origin,
            );
            if self
                .facts
                .app_setting_daemon_window_local
                .replace(local)
                .is_some()
            {
                return Err(self.invariant(
                    &self.document.settings.span,
                    "duplicate app-setting daemon-window local",
                ));
            }
            callback_env.insert(
                "window".into(),
                CheckedPathRoot::Local(local),
                Type::WindowId,
            );
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
            let slots = self.facts.component_slots(component_id).ok_or_else(|| {
                self.invariant(&component.span, "component has no checked slot partition")
            })?;
            for slot in slots {
                env.insert_slot(slot.name.clone(), slot.id);
            }
            self.lower_view_expression_tree(&component.root, &env)?;
        }
        for test in &self.document.tests {
            if let Some(mount) = &test.mount {
                self.lower_view_expression_tree(mount, &app_env)?;
            }
        }
        self.validate_keyed_lazy_prop_arguments()
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
            record_fact_metric!(self.facts.metrics.scope_env_builds += 1);
            record_fact_metric!(self.facts.metrics.scope_env_entries += component_env.len());
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

    fn lower_test_expressions(&mut self) -> Result<(), Error> {
        let document = self.document;
        for (test_index, test) in document.tests.iter().enumerate() {
            let declaration = self.declarations.test(test_index);
            let mut env = self.fact_env(ValueScope::App);
            if self.document.daemon {
                let local = self.facts.daemon_window_local().ok_or_else(|| {
                    self.invariant(&test.span, "daemon test has no checked window local")
                })?;
                env.insert(
                    "window".into(),
                    CheckedPathRoot::Local(local),
                    Type::WindowId,
                );
            }
            for (target_index, target) in test.targets.iter().enumerate() {
                let target_id = TestTargetId {
                    test: declaration.declaration.id,
                    index: target_index as u32,
                };
                for (segment_index, segment) in target.target.segments.iter().enumerate() {
                    if let Some(key) = &segment.key {
                        self.push_test_expression(
                            CheckedExprOwner::TestTargetKey {
                                target: target_id,
                                segment: segment_index as u32,
                            },
                            key,
                            &env,
                            &target.span,
                            self.declarations
                                .test_target(target_id)
                                .expect("indexed test target")
                                .declaration
                                .origin,
                        )?;
                    }
                }
                let owner = CheckedLocalOwner::TestTarget(target_id);
                let local = CheckedLocalId(self.facts.locals.len() as u32);
                self.facts.locals.push(CheckedLocal {
                    name: target.name.clone(),
                    ty: Type::TestTarget,
                    owner,
                    origin: self
                        .declarations
                        .test_target(target_id)
                        .expect("indexed test target")
                        .declaration
                        .origin,
                });
                self.facts.locals_by_owner.insert(owner, local);
                env.insert(
                    target.name.clone(),
                    CheckedPathRoot::Local(local),
                    Type::TestTarget,
                );
            }
            for (step_index, step) in test.steps.iter().enumerate() {
                let step_id = TestStepId {
                    test: declaration.declaration.id,
                    index: step_index as u32,
                };
                let origin = self
                    .declarations
                    .test_step(step_id)
                    .expect("indexed test step")
                    .declaration
                    .origin;
                for (operand, expression) in crate::ast::test_step_expression_roots(step)
                    .into_iter()
                    .enumerate()
                {
                    self.push_test_expression(
                        CheckedExprOwner::TestStepOperand {
                            step: step_id,
                            operand: operand as u32,
                        },
                        expression,
                        &env,
                        &step.span,
                        origin,
                    )?;
                }
            }
        }
        Ok(())
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    fn record_analysis_metrics(&mut self, analysis: &ExprTypeAnalysis) {
        #[cfg(test)]
        {
            let metrics = analysis.metrics();
            self.facts.metrics.type_analysis_queries += metrics.queries;
            self.facts.metrics.type_analysis_nodes += metrics.nodes;
            self.facts.metrics.type_analysis_cache_hits += metrics.cache_hits;
            self.facts.metrics.type_scope_env_overlays += metrics.scoped_env_overlays;
            self.facts.metrics.type_scope_env_full_clones += metrics.scoped_env_full_clones;
        }
    }

    /// Every checked expression reaches the fact store through here: the eleven
    /// call sites differ only in which analysis store they took `analysis` out
    /// of, how `source` relates to `expected`, and the noun in their invariants.
    #[allow(clippy::too_many_arguments)]
    fn push_analyzed_expression(
        &mut self,
        owner: CheckedExprOwner,
        analysis: ExprTypeAnalysis,
        expr: &Expr,
        expected: Option<&Type>,
        mode: SourceMode,
        noun: &'static str,
        env: &dyn FactEnvironment,
        span: &Span,
        origin: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let inferred = analysis.type_of(expr).cloned().ok_or_else(|| {
            self.invariant(
                span,
                format!("missing retained {noun} expression root type"),
            )
        })?;
        let (source, coercion) = match mode {
            SourceMode::Raw => (inferred, CheckedInitializerCoercion::None),
            SourceMode::Contextual => (
                resolve_erased_type(&contextual_type(inferred, expected)),
                CheckedInitializerCoercion::None,
            ),
            SourceMode::Initializer => initializer_source_context(
                &inferred,
                expected.ok_or_else(|| {
                    self.invariant(span, format!("{noun} initializer has no destination type"))
                })?,
            ),
        };
        let id = CheckedExprUseId(self.facts.expression_uses.len() as u32);
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
                format!("{noun} expression source type does not match its checked root"),
            ));
        }
        self.facts.expression_uses.push(CheckedExprUse {
            owner,
            root,
            destination: expected.cloned().unwrap_or_else(|| source.clone()),
            source,
            coercion,
            origin,
        });
        if self
            .facts
            .expression_uses_by_owner
            .insert(owner, id)
            .is_some()
        {
            return Err(self.invariant(span, format!("duplicate checked {noun} expression owner")));
        }
        Ok(id)
    }

    fn push_test_expression(
        &mut self,
        owner: CheckedExprOwner,
        expression: &Expr,
        env: &dyn FactEnvironment,
        span: &Span,
        origin: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let analysis = self
            .analyses
            .test_entries
            .remove(&super::expr::expr_key(expression))
            .ok_or_else(|| {
                self.invariant(span, "missing authoritative test expression analysis")
            })?;
        self.record_analysis_metrics(&analysis);
        self.push_analyzed_expression(
            owner,
            analysis,
            expression,
            None,
            SourceMode::Raw,
            "test",
            env,
            span,
            origin,
        )
    }

    fn lower_container_facts(
        &mut self,
        container: ViewId,
        options: &ContainerOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let parent = self.declarations.view(container).origin;
        let roots = crate::ast::container_expression_roots(options);
        let mut expression_count = 0u32;
        for expression in &roots {
            self.push_interaction_expression(
                container,
                &mut expression_count,
                expression,
                None,
                env,
                span,
                parent,
            )?;
        }
        let remaining_expressions = self
            .analyses
            .site_expressions(AnalysisSite::Interaction, container);
        let remaining_routes = self
            .analyses
            .site_routes(AnalysisSite::Interaction, container);
        if remaining_expressions != 0 || remaining_routes != 0 {
            return Err(self.invariant(
                span,
                format!(
                    "container left {remaining_expressions} expression and {remaining_routes} route analyses unconsumed"
                ),
            ));
        }
        if expression_count != roots.len() as u32 {
            return Err(self.invariant(span, "container expression count diverged"));
        }
        let style = options
            .custom_style
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::ContainerStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "container style extern disappeared"))
            })
            .transpose()?;
        if self
            .facts
            .containers
            .insert(
                container,
                CheckedContainer {
                    id: container,
                    expression_count,
                    semantic_key: crate::ast::container_semantic_key(options),
                    style,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "container facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_layout_facts(
        &mut self,
        layout: ViewId,
        kind: Layout,
        options: &LayoutOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let option_expressions = crate::ast::layout_expression_roots(options)
            .into_iter()
            .map(|expression| (expression, None))
            .collect();
        self.lower_interaction_facts(
            layout,
            CheckedInteractionKind::Layout,
            crate::ast::layout_semantic_key(kind, options),
            option_expressions,
            crate::ast::layout_routes(options),
            env,
            span,
        )?;
        let scroll_style = options
            .scroll
            .as_ref()
            .and_then(|scroll| scroll.custom_style.as_ref())
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::ScrollStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "scroll style extern disappeared"))
            })
            .transpose()?;
        let parent = self.declarations.view(layout).origin;
        let style_origins = options.scroll.as_ref().map_or_else(Vec::new, |scroll| {
            scroll
                .styles
                .iter()
                .map(|style| self.origins.push(&style.span, Some(parent)))
                .collect()
        });
        if self
            .facts
            .layouts
            .insert(
                layout,
                CheckedLayout {
                    id: layout,
                    scroll_style,
                    style_origins,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "layout facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_text_facts(
        &mut self,
        text: ViewId,
        kind: CheckedInteractionKind,
        semantic_key: String,
        expressions: Vec<&Expr>,
        routes: Vec<&Route>,
        options: &TextOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            text,
            kind,
            semantic_key,
            expressions
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            routes,
            env,
            span,
        )?;
        self.insert_text_facts(text, options, Vec::new(), span)
    }

    /// Pushes the rich-text option expressions with the widget environment,
    /// then each child's expressions in document order — a `for` child pushes
    /// its `items` first, binds the item local, and pushes its span
    /// expressions under the loop-scoped environment.
    #[allow(clippy::too_many_arguments)]
    fn lower_rich_text_facts(
        &mut self,
        text: ViewId,
        options: &TextOptions,
        color: &Option<String>,
        children: &[RichTextChild],
        route: &Option<Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let semantic_key = crate::ast::rich_text_semantic_key(options, color, children, route);
        let parent = self.declarations.view(text).origin;
        let mut expression_count = 0u32;
        let mut option_expressions = Vec::new();
        for expression in crate::ast::text_option_expression_roots(options) {
            option_expressions.push(self.push_interaction_expression(
                text,
                &mut expression_count,
                expression,
                None,
                env,
                span,
                parent,
            )?);
        }
        let mut rich_children = Vec::with_capacity(children.len());
        for (index, child) in children.iter().enumerate() {
            match child {
                RichTextChild::Span(item) => {
                    let origin = self.origins.push(&item.span, Some(parent));
                    self.push_rich_span_expressions(
                        text,
                        &mut expression_count,
                        &mut option_expressions,
                        item,
                        env,
                        origin,
                    )?;
                    rich_children.push(CheckedRichChild::Span { origin });
                }
                RichTextChild::For(iteration) => {
                    let origin = self.origins.push(&iteration.span, Some(parent));
                    let items = self.push_interaction_expression(
                        text,
                        &mut expression_count,
                        &iteration.items,
                        None,
                        env,
                        &iteration.span,
                        origin,
                    )?;
                    option_expressions.push(items);
                    let Type::List(item_ty) = self.facts.expression_use(items).source.clone()
                    else {
                        return Err(self.invariant(
                            &iteration.span,
                            "checked rich-text for items are not a list",
                        ));
                    };
                    let local = self.push_view_local_with_parent(
                        &iteration.item,
                        *item_ty.clone(),
                        text,
                        CheckedViewLocalRole::RichForItem(index as u32),
                        &iteration.span,
                        origin,
                    );
                    let scoped = LayeredFactEnv {
                        base: env,
                        name: iteration.item.clone(),
                        value: (CheckedPathRoot::Local(local), *item_ty),
                    };
                    record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
                    let scoped_start = expression_count;
                    let mut spans = Vec::with_capacity(iteration.spans.len());
                    for item in &iteration.spans {
                        let span_origin = self.origins.push(&item.span, Some(origin));
                        self.push_rich_span_expressions(
                            text,
                            &mut expression_count,
                            &mut option_expressions,
                            item,
                            &scoped,
                            span_origin,
                        )?;
                        spans.push(span_origin);
                    }
                    rich_children.push(CheckedRichChild::For {
                        items,
                        item: local,
                        origin,
                        spans,
                        scoped_expressions: scoped_start..expression_count,
                    });
                }
            }
        }
        self.finish_interaction_facts(
            text,
            CheckedInteractionKind::RichText,
            semantic_key,
            expression_count,
            option_expressions,
            route.iter().collect(),
            env,
            span,
        )?;
        self.insert_text_facts(text, options, rich_children, span)
    }

    fn push_rich_span_expressions(
        &mut self,
        text: ViewId,
        expression_count: &mut u32,
        option_expressions: &mut Vec<CheckedExprUseId>,
        item: &RichSpan,
        env: &dyn FactEnvironment,
        origin: OriginId,
    ) -> Result<(), Error> {
        let mut roots = Vec::new();
        crate::ast::push_rich_span_expression_roots(&mut roots, item);
        for expression in roots {
            option_expressions.push(self.push_interaction_expression(
                text,
                expression_count,
                expression,
                None,
                env,
                &item.span,
                origin,
            )?);
        }
        Ok(())
    }

    fn insert_text_facts(
        &mut self,
        text: ViewId,
        options: &TextOptions,
        rich_children: Vec<CheckedRichChild>,
        span: &Span,
    ) -> Result<(), Error> {
        let style = options
            .custom_style
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::TextStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "text style extern disappeared"))
            })
            .transpose()?;
        if self
            .facts
            .texts
            .insert(
                text,
                CheckedText {
                    id: text,
                    style,
                    rich_children,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "text facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_input_facts(
        &mut self,
        input: ViewId,
        label: &str,
        binding: &str,
        hint: &str,
        disabled: &Option<Expr>,
        options: &InputOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            input,
            CheckedInteractionKind::Input,
            crate::ast::input_semantic_key(label, binding, hint, disabled, options),
            crate::ast::input_expression_roots(disabled, options)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            [&options.change, &options.submit, &options.paste]
                .into_iter()
                .flatten()
                .collect(),
            env,
            span,
        )?;
        let checked_binding = env
            .get(binding)
            .and_then(|(root, ty)| {
                matches!(ty, Type::Str | Type::Secret)
                    .then_some(match root {
                        CheckedPathRoot::Value(value) => Some(*value),
                        _ => None,
                    })
                    .flatten()
            })
            .ok_or_else(|| {
                self.invariant(
                    span,
                    "input binding lost its checked string or secret state",
                )
            })?;
        let style = options
            .custom_style
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::InputStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "input style extern disappeared"))
            })
            .transpose()?;
        let parent = self.declarations.view(input).origin;
        let icon_origin = options
            .icon
            .as_ref()
            .map(|icon| self.origins.push(&icon.span, Some(parent)));
        let status_origins = [
            &options.style.active,
            &options.style.hovered,
            &options.style.focused,
            &options.style.focused_hovered,
            &options.style.disabled,
        ]
        .into_iter()
        .flatten()
        .map(|status| {
            self.origins
                .push(status.span.as_ref().unwrap_or(span), Some(parent))
        })
        .collect();
        if self
            .facts
            .inputs
            .insert(
                input,
                CheckedInput {
                    id: input,
                    binding: checked_binding,
                    style,
                    icon_origin,
                    status_origins,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "input facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_button_facts(
        &mut self,
        button: ViewId,
        label: &Option<String>,
        content: &Option<Box<ViewNode>>,
        disabled: &Option<Expr>,
        options: &ButtonOptions,
        route: &Route,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            button,
            CheckedInteractionKind::Button,
            crate::ast::button_semantic_key(label, content, disabled, options, route),
            crate::ast::button_expression_roots(disabled, options)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            vec![route],
            env,
            span,
        )?;
        let style = options
            .style
            .custom
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::ButtonStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "button style extern disappeared"))
            })
            .transpose()?;
        let parent = self.declarations.view(button).origin;
        let status_origins = [
            &options.style.active,
            &options.style.hovered,
            &options.style.pressed,
            &options.style.disabled,
        ]
        .into_iter()
        .flatten()
        .map(|status| self.origins.push(&status.span, Some(parent)))
        .collect();
        if self
            .facts
            .buttons
            .insert(
                button,
                CheckedButton {
                    id: button,
                    style,
                    status_origins,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "button facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pick_list_facts(
        &mut self,
        pick: ViewId,
        options: &Expr,
        selected: &Expr,
        config: &PickListOptions,
        route: &Route,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            pick,
            CheckedInteractionKind::PickList,
            crate::ast::pick_list_semantic_key(config, route),
            crate::ast::pick_list_expression_roots(options, selected, config)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            crate::ast::pick_list_routes(config, route),
            env,
            span,
        )?;
        let resolve = |call: &Option<ExternCall>, kind: ExternKind, label: &str| {
            call.as_ref()
                .map(|call| {
                    self.declarations
                        .extern_decl_by_name(&call.function)
                        .filter(|function| function.kind == kind)
                        .map(|function| function.declaration.id)
                        .ok_or_else(|| {
                            self.invariant(span, format!("pick {label} extern disappeared"))
                        })
                })
                .transpose()
        };
        let style = resolve(&config.custom_style, ExternKind::PickListStyle, "style")?;
        let menu_style = resolve(
            &config.custom_menu_style,
            ExternKind::MenuStyle,
            "menu style",
        )?;
        let parent = self.declarations.view(pick).origin;
        let handle_origins = match &config.handle {
            Some(PickListHandle::Static(icon)) => vec![self.origins.push(&icon.span, Some(parent))],
            Some(PickListHandle::Dynamic { closed, open }) => vec![
                self.origins.push(&closed.span, Some(parent)),
                self.origins.push(&open.span, Some(parent)),
            ],
            Some(PickListHandle::Arrow { .. } | PickListHandle::None) | None => Vec::new(),
        };
        let status_origins = [
            &config.style.active,
            &config.style.hovered,
            &config.style.opened,
            &config.style.opened_hovered,
        ]
        .into_iter()
        .flatten()
        .map(|status| {
            self.origins
                .push(status.span.as_ref().unwrap_or(span), Some(parent))
        })
        .collect();
        let menu_origin = config.menu_style.as_ref().map(|menu| {
            self.origins
                .push(menu.span.as_ref().unwrap_or(span), Some(parent))
        });
        if self
            .facts
            .pick_lists
            .insert(
                pick,
                CheckedPickList {
                    id: pick,
                    style,
                    menu_style,
                    handle_origins,
                    status_origins,
                    menu_origin,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "pick facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_combo_box_facts(
        &mut self,
        combo: ViewId,
        state: &str,
        selected: &Expr,
        placeholder: &str,
        options: &ComboBoxOptions,
        route: &Route,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            combo,
            CheckedInteractionKind::ComboBox,
            crate::ast::combo_box_semantic_key(state, placeholder, options, route),
            crate::ast::combo_box_expression_roots(selected, options)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            crate::ast::combo_box_routes(options, route),
            env,
            span,
        )?;
        let binding = env
            .get(state)
            .and_then(|(root, ty)| {
                matches!(ty, Type::Combo(_))
                    .then_some(match root {
                        CheckedPathRoot::Value(value) => Some(*value),
                        _ => None,
                    })
                    .flatten()
            })
            .ok_or_else(|| self.invariant(span, "combo lost its checked state binding"))?;
        let resolve = |call: &Option<ExternCall>, kind: ExternKind, label: &str| {
            call.as_ref()
                .map(|call| {
                    self.declarations
                        .extern_decl_by_name(&call.function)
                        .filter(|function| function.kind == kind)
                        .map(|function| function.declaration.id)
                        .ok_or_else(|| {
                            self.invariant(span, format!("combo {label} extern disappeared"))
                        })
                })
                .transpose()
        };
        let style = resolve(&options.custom_style, ExternKind::InputStyle, "style")?;
        let menu_style = resolve(
            &options.custom_menu_style,
            ExternKind::MenuStyle,
            "menu style",
        )?;
        let parent = self.declarations.view(combo).origin;
        let icon_origin = options
            .icon
            .as_ref()
            .map(|icon| self.origins.push(&icon.span, Some(parent)));
        let status_origins = [
            &options.style.active,
            &options.style.hovered,
            &options.style.focused,
            &options.style.focused_hovered,
            &options.style.disabled,
        ]
        .into_iter()
        .flatten()
        .map(|status| {
            self.origins
                .push(status.span.as_ref().unwrap_or(span), Some(parent))
        })
        .collect();
        let menu_origin = options.menu_style.as_ref().map(|menu| {
            self.origins
                .push(menu.span.as_ref().unwrap_or(span), Some(parent))
        });
        if self
            .facts
            .combo_boxes
            .insert(
                combo,
                CheckedComboBox {
                    id: combo,
                    binding,
                    style,
                    menu_style,
                    icon_origin,
                    status_origins,
                    menu_origin,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "combo facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_text_editor_facts(
        &mut self,
        editor: ViewId,
        binding: &str,
        disabled: &Option<Expr>,
        options: &TextEditorOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            editor,
            CheckedInteractionKind::TextEditor,
            crate::ast::text_editor_semantic_key(binding, disabled, options),
            crate::ast::text_editor_expression_roots(disabled, options)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            options.key_binding_route.iter().collect(),
            env,
            span,
        )?;
        let checked_binding = env
            .get(binding)
            .and_then(|(root, ty)| {
                (ty == &Type::Editor)
                    .then_some(match root {
                        CheckedPathRoot::Value(value) => Some(*value),
                        _ => None,
                    })
                    .flatten()
            })
            .ok_or_else(|| self.invariant(span, "editor binding lost its checked editor state"))?;
        let resolve = |call: &Option<ExternCall>, kind: ExternKind, label: &str| {
            call.as_ref()
                .map(|call| {
                    self.declarations
                        .extern_decl_by_name(&call.function)
                        .filter(|function| function.kind == kind)
                        .map(|function| function.declaration.id)
                        .ok_or_else(|| {
                            self.invariant(span, format!("editor {label} extern disappeared"))
                        })
                })
                .transpose()
        };
        let highlighter = resolve(
            &options.highlighter,
            ExternKind::EditorHighlighter,
            "highlighter",
        )?;
        let key_binding = resolve(
            &options.key_binding,
            ExternKind::EditorBinding,
            "key binding",
        )?;
        let action = resolve(&options.action, ExternKind::EditorAction, "action")?;
        let style = resolve(&options.custom_style, ExternKind::EditorStyle, "style")?;
        let parent = self.declarations.view(editor).origin;
        let status_origins = [
            &options.style.active,
            &options.style.hovered,
            &options.style.focused,
            &options.style.focused_hovered,
            &options.style.disabled,
        ]
        .into_iter()
        .flatten()
        .map(|status| {
            self.origins
                .push(status.span.as_ref().unwrap_or(span), Some(parent))
        })
        .collect();
        if self
            .facts
            .text_editors
            .insert(
                editor,
                CheckedTextEditor {
                    id: editor,
                    binding: checked_binding,
                    highlighter,
                    key_binding,
                    action,
                    style,
                    status_origins,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "text editor facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_markdown_facts(
        &mut self,
        markdown: ViewId,
        content: &str,
        options: &MarkdownOptions,
        route: &Route,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let content = env
            .get(content)
            .and_then(|(root, ty)| {
                (ty == &Type::Markdown)
                    .then_some(match root {
                        CheckedPathRoot::Value(value) => Some(*value),
                        _ => None,
                    })
                    .flatten()
            })
            .ok_or_else(|| self.invariant(span, "markdown content lost its checked value"))?;
        let viewer = options
            .viewer
            .as_ref()
            .map(|call| {
                self.declarations
                    .extern_decl_by_name(&call.function)
                    .filter(|function| function.kind == ExternKind::MarkdownViewer)
                    .cloned()
                    .ok_or_else(|| self.invariant(span, "markdown viewer extern disappeared"))
            })
            .transpose()?;
        let roots = crate::ast::markdown_expression_roots(options);
        let viewer_argument_count = options.viewer.as_ref().map_or(0, |call| call.args.len());
        let metric_count = roots
            .len()
            .checked_sub(viewer_argument_count)
            .ok_or_else(|| self.invariant(span, "markdown viewer argument cardinality diverged"))?;
        let style_span = options.style.span.as_ref().unwrap_or(span);
        let style_expressions = crate::ast::markdown_style_expression_roots(&options.style)
            .into_iter()
            .map(|expression| std::ptr::from_ref(expression).addr())
            .collect::<HashSet<_>>();
        let mut expressions = Vec::with_capacity(roots.len());
        for expression in roots.iter().take(metric_count) {
            let expression_span =
                if style_expressions.contains(&std::ptr::from_ref(*expression).addr()) {
                    style_span
                } else {
                    span
                };
            expressions.push((*expression, Some(Type::F64), expression_span));
        }
        if let Some(function) = &viewer {
            if function.params.len() != viewer_argument_count
                || function.borrowed.len() != viewer_argument_count
            {
                return Err(self.invariant(span, "markdown viewer checked contract diverged"));
            }
            for (expression, (_, destination)) in roots[metric_count..].iter().zip(&function.params)
            {
                expressions.push((*expression, Some(destination.clone()), span));
            }
        } else if viewer_argument_count != 0 {
            return Err(self.invariant(span, "markdown viewer arguments have no extern"));
        }
        self.lower_interaction_facts_with_spans(
            markdown,
            CheckedInteractionKind::Markdown,
            crate::ast::markdown_semantic_key(options),
            expressions,
            vec![route],
            env,
            span,
        )?;
        let parent = self.declarations.view(markdown).origin;
        let style_origin = options
            .style
            .span
            .as_ref()
            .map(|style_span| self.origins.push(style_span, Some(parent)));
        let viewer_output = viewer
            .as_ref()
            .map(|function| function.output.clone())
            .unwrap_or(Type::Str);
        if self
            .facts
            .markdowns
            .insert(
                markdown,
                CheckedMarkdown {
                    id: markdown,
                    content,
                    viewer: viewer.map(|function| function.declaration.id),
                    viewer_output,
                    style_origin,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "markdown facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_extern_component_facts(
        &mut self,
        component: ViewId,
        function: &str,
        args: &[Expr],
        route: &Option<Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let declaration = self
            .declarations
            .extern_decl_by_name(function)
            .filter(|declaration| declaration.kind == ExternKind::Component)
            .cloned()
            .ok_or_else(|| self.invariant(span, "extern component declaration disappeared"))?;
        if declaration.params.len() != args.len() || declaration.borrowed.len() != args.len() {
            return Err(self.invariant(span, "extern component argument contract diverged"));
        }
        self.lower_interaction_facts(
            component,
            CheckedInteractionKind::ExternComponent,
            crate::ast::extern_component_semantic_key(function, args, route),
            args.iter()
                .zip(&declaration.params)
                .map(|(argument, (_, destination))| (argument, Some(destination.clone())))
                .collect(),
            route.iter().collect(),
            env,
            span,
        )?;
        if self
            .facts
            .extern_components
            .insert(
                component,
                CheckedExternComponent {
                    id: component,
                    function: declaration.declaration.id,
                    output: declaration.output,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "extern component facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_extern_view_adapter_facts(
        &mut self,
        view: ViewId,
        interaction_kind: CheckedInteractionKind,
        extern_kind: ExternKind,
        semantic_key: String,
        function: &str,
        args: &[Expr],
        trailing_expressions: Vec<&Expr>,
        route: &Option<Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedExternViewAdapter, Error> {
        let declaration = self
            .declarations
            .extern_decl_by_name(function)
            .filter(|declaration| declaration.kind == extern_kind)
            .cloned()
            .ok_or_else(|| self.invariant(span, "extern view adapter declaration disappeared"))?;
        if declaration.params.len() != args.len()
            || declaration.borrowed.len() != args.len()
            || declaration.borrowed.iter().any(|borrowed| *borrowed)
        {
            return Err(self.invariant(span, "extern view adapter argument contract diverged"));
        }
        let mut expressions = args
            .iter()
            .zip(&declaration.params)
            .map(|(argument, (_, destination))| (argument, Some(destination.clone())))
            .collect::<Vec<_>>();
        expressions.extend(
            trailing_expressions
                .into_iter()
                .map(|expression| (expression, None)),
        );
        self.lower_interaction_facts(
            view,
            interaction_kind,
            semantic_key,
            expressions,
            route.iter().collect(),
            env,
            span,
        )?;
        Ok(CheckedExternViewAdapter {
            id: view,
            function: declaration.declaration.id,
            output: declaration.output,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_themer_facts(
        &mut self,
        view: ViewId,
        function: &str,
        args: &[Expr],
        route: &Option<Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let checked = self.lower_extern_view_adapter_facts(
            view,
            CheckedInteractionKind::Themer,
            ExternKind::Themer,
            crate::ast::themer_semantic_key(function, args, route),
            function,
            args,
            Vec::new(),
            route,
            env,
            span,
        )?;
        if self.facts.themers.insert(view, checked).is_some() {
            return Err(self.invariant(span, "themer facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_shader_facts(
        &mut self,
        view: ViewId,
        function: &str,
        args: &[Expr],
        width: &Option<LengthValue>,
        height: &Option<LengthValue>,
        route: &Option<Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let dimensions = [width, height]
            .into_iter()
            .filter_map(|length| match length {
                Some(LengthValue::Fixed(expression)) => Some(expression),
                _ => None,
            })
            .collect();
        let checked = self.lower_extern_view_adapter_facts(
            view,
            CheckedInteractionKind::Shader,
            ExternKind::Shader,
            crate::ast::shader_semantic_key(function, args, width, height, route),
            function,
            args,
            dimensions,
            route,
            env,
            span,
        )?;
        if self.facts.shaders.insert(view, checked).is_some() {
            return Err(self.invariant(span, "shader facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_nested_theme_facts(
        &mut self,
        view: ViewId,
        preset: &ThemePreset,
        text: &Option<String>,
        background: &Option<BackgroundValue>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let factory = match preset {
            ThemePreset::Factory(factory) => Some(
                self.declarations
                    .extern_decl_by_name(&factory.function)
                    .filter(|declaration| declaration.kind == ExternKind::Theme)
                    .cloned()
                    .ok_or_else(|| self.invariant(span, "nested theme factory disappeared"))?,
            ),
            ThemePreset::Default | ThemePreset::App | ThemePreset::BuiltIn(_) => None,
        };
        let roots = crate::ast::nested_theme_expression_roots(preset, background);
        let mut expressions = Vec::with_capacity(roots.len());
        if let (ThemePreset::Factory(call), Some(declaration)) = (preset, factory.as_ref()) {
            if declaration.params.len() != call.args.len()
                || declaration.borrowed.len() != call.args.len()
            {
                return Err(self.invariant(span, "nested theme factory contract diverged"));
            }
            expressions.extend(
                call.args
                    .iter()
                    .zip(&declaration.params)
                    .map(|(argument, (_, destination))| (argument, Some(destination.clone()))),
            );
        }
        if let Some(BackgroundValue::Linear { angle, stops }) = background {
            expressions.push((angle, Some(Type::F64)));
            expressions.extend(stops.iter().map(|stop| (&stop.offset, Some(Type::F64))));
        }
        if expressions.len() != roots.len() {
            return Err(self.invariant(span, "nested theme expression partition diverged"));
        }
        self.lower_interaction_facts(
            view,
            CheckedInteractionKind::NestedTheme,
            crate::ast::nested_theme_semantic_key(preset, text, background),
            expressions,
            Vec::new(),
            env,
            span,
        )?;
        let checked = CheckedNestedTheme {
            id: view,
            factory: factory.map(|declaration| declaration.declaration.id),
        };
        if self.facts.nested_themes.insert(view, checked).is_some() {
            return Err(self.invariant(span, "nested theme facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_component_call_route_facts(
        &mut self,
        view: ViewId,
        call: ComponentCallId,
        component: ComponentId,
        component_name: &str,
        supplied_events: &[ComponentEventRoute],
        output_route: &Option<Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let source_component = self
            .document
            .components
            .get(component.0 as usize)
            .ok_or_else(|| self.invariant(span, "component call declaration disappeared"))?;
        if source_component.name != component_name {
            return Err(self.invariant(span, "component call identity diverged"));
        }
        let output = source_component.output.clone();
        let declared_events = source_component
            .events
            .iter()
            .enumerate()
            .map(|(index, event)| {
                let id = ComponentEventId {
                    component,
                    index: index as u32,
                };
                let declaration = self.declarations.component_event(id).ok_or_else(|| {
                    self.invariant(span, "component event declaration disappeared")
                })?;
                if declaration.name != event.name || declaration.payloads != event.payloads {
                    return Err(self.invariant(span, "component event contract diverged"));
                }
                Ok((id, event.name.clone(), event.payloads.clone()))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut supplied_by_name = HashMap::new();
        for supplied in supplied_events {
            if supplied_by_name
                .insert(supplied.name.as_str(), supplied)
                .is_some()
            {
                return Err(self.invariant(&supplied.span, "duplicate component event route"));
            }
        }
        let ordered_events = declared_events
            .iter()
            .map(|(_, name, _)| {
                supplied_by_name
                    .remove(name.as_str())
                    .ok_or_else(|| self.invariant(span, "component event route disappeared"))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        if !supplied_by_name.is_empty() {
            return Err(self.invariant(span, "component call retained an unknown event route"));
        }
        let semantic_key = crate::ast::component_call_route_semantic_key(
            component_name,
            output_route.is_some(),
            ordered_events
                .iter()
                .map(|event| (event.name.as_str(), event.route.is_some())),
        );
        let routes = output_route
            .iter()
            .chain(
                ordered_events
                    .iter()
                    .filter_map(|event| event.route.as_ref()),
            )
            .collect::<Vec<_>>();
        self.lower_interaction_facts(
            view,
            CheckedInteractionKind::ComponentCallRoutes,
            semantic_key,
            Vec::new(),
            routes,
            env,
            span,
        )?;

        let interaction_route_origins = self
            .facts
            .interaction(view)
            .ok_or_else(|| self.invariant(span, "component routes have no interaction facts"))?
            .routes
            .iter()
            .map(|route| route.origin)
            .collect::<Vec<_>>();

        let parent = self.declarations.view(view).origin;
        let mut route_index = 0u32;
        let checked_output = match (&output, output_route) {
            (Type::Unit, None) => None,
            (output, Some(_)) => {
                let route = InteractionRouteId {
                    widget: view,
                    index: route_index,
                };
                let origin = *interaction_route_origins
                    .get(route_index as usize)
                    .ok_or_else(|| {
                        self.invariant(span, "component output route origin disappeared")
                    })?;
                route_index += 1;
                Some(CheckedComponentOutputRoute {
                    output: output.clone(),
                    route,
                    origin,
                })
            }
            _ => return Err(self.invariant(span, "component output route topology diverged")),
        };
        let scope = self.facts.view(view).scope;
        let mut checked_events = Vec::with_capacity(declared_events.len());
        for ((event, name, payloads), supplied) in declared_events.into_iter().zip(ordered_events) {
            let origin = self.origins.push(&supplied.span, Some(parent));
            let delivery = if supplied.route.is_some() {
                let route = InteractionRouteId {
                    widget: view,
                    index: route_index,
                };
                let route_origin = *interaction_route_origins
                    .get(route_index as usize)
                    .ok_or_else(|| {
                        self.invariant(&supplied.span, "component event route origin disappeared")
                    })?;
                route_index += 1;
                CheckedComponentEventDelivery::Direct {
                    route,
                    origin: route_origin,
                }
            } else {
                let CheckedViewScope::Component(outer_component) = scope else {
                    return Err(self.invariant(
                        &supplied.span,
                        "forwarded event has no checked outer component",
                    ));
                };
                let outer_component_name = self
                    .document
                    .components
                    .get(outer_component.0 as usize)
                    .map(|component| component.name.clone())
                    .ok_or_else(|| {
                        self.invariant(
                            &supplied.span,
                            "forwarded event outer component contract diverged",
                        )
                    })?;
                let outer_event = self
                    .declarations
                    .component_event_by_name(outer_component, &name)
                    .filter(|outer| outer.payloads == payloads)
                    .ok_or_else(|| {
                        self.invariant(
                            &supplied.span,
                            "forwarded event declaration contract diverged",
                        )
                    })?
                    .clone();
                CheckedComponentEventDelivery::Forward {
                    outer_component,
                    outer_component_name,
                    outer_event: outer_event.declaration.id,
                    outer_event_name: outer_event.name,
                    outer_payloads: outer_event.payloads,
                }
            };
            checked_events.push(CheckedComponentEventRoute {
                event,
                name,
                payloads,
                delivery,
                origin,
            });
        }
        if route_index as usize != interaction_route_origins.len() {
            return Err(self.invariant(span, "component route cardinality diverged"));
        }
        let checked = CheckedComponentCallRoutes {
            call,
            view,
            component,
            output: checked_output,
            events: checked_events,
        };
        if self
            .facts
            .component_call_routes
            .insert(call, checked)
            .is_some()
        {
            return Err(self.invariant(span, "component call routes were produced more than once"));
        }
        Ok(())
    }

    fn lower_boolean_control_facts(
        &mut self,
        control: ViewId,
        source: BooleanControlFactSource<'_>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let parent = self.declarations.view(control).origin;
        let mut status_origins = Vec::with_capacity(source.statuses.len());
        let mut expression_parents = HashMap::new();
        for (status_span, expressions) in source.statuses {
            let origin = self.origins.push(status_span, Some(parent));
            status_origins.push(origin);
            for expression in expressions {
                expression_parents
                    .insert(std::ptr::from_ref(expression).addr(), (status_span, origin));
            }
        }

        let mut expression_count = 0u32;
        let option_expressions = source
            .expressions
            .into_iter()
            .map(|expression| {
                let (expression_span, expression_parent) = expression_parents
                    .get(&std::ptr::from_ref(expression).addr())
                    .copied()
                    .unwrap_or((span, parent));
                self.push_interaction_expression(
                    control,
                    &mut expression_count,
                    expression,
                    None,
                    env,
                    expression_span,
                    expression_parent,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let route = self.lower_interaction_route(
            control,
            0,
            source.route,
            env,
            &mut expression_count,
            parent,
        )?;
        let remaining_expressions = self
            .analyses
            .site_expressions(AnalysisSite::Interaction, control);
        let remaining_routes = self
            .analyses
            .site_routes(AnalysisSite::Interaction, control);
        if remaining_expressions != 0 || remaining_routes != 0 {
            return Err(self.invariant(
                span,
                "boolean control left authoritative expression or route analyses unconsumed",
            ));
        }
        let style = source
            .style
            .map(|(name, kind)| {
                self.declarations
                    .extern_decl_by_name(name)
                    .filter(|function| function.kind == kind)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "boolean style extern disappeared"))
            })
            .transpose()?;
        let style_origin = style.map(|_| self.origins.push(span, Some(parent)));
        let font_origin = source
            .has_font
            .then(|| self.origins.push(span, Some(parent)));
        if self
            .facts
            .interactions
            .insert(
                control,
                CheckedInteraction {
                    id: control,
                    kind: source.kind,
                    semantic_key: source.semantic_key,
                    expression_count,
                    option_expressions,
                    routes: vec![route],
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "boolean interaction facts were produced twice"));
        }
        if self
            .facts
            .boolean_controls
            .insert(
                control,
                CheckedBooleanControl {
                    id: control,
                    style,
                    style_origin,
                    font_origin,
                    status_origins,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "boolean control facts were produced twice"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_slider_facts(
        &mut self,
        slider: ViewId,
        value: &Expr,
        min: &Expr,
        max: &Expr,
        step: &Expr,
        options: &SliderOptions,
        vertical: bool,
        styles: &[String],
        route: &Route,
        release: &Option<Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let roots = crate::ast::slider_expression_roots(value, min, max, step, options, span);
        self.lower_interaction_facts_with_spans(
            slider,
            CheckedInteractionKind::Slider,
            crate::ast::slider_semantic_key(options, vertical, styles, route, release),
            roots
                .into_iter()
                .map(|(expression, origin)| (expression, None, origin))
                .collect(),
            std::iter::once(route).chain(release.iter()).collect(),
            env,
            span,
        )?;
        let interaction = self
            .facts
            .interaction(slider)
            .ok_or_else(|| self.invariant(span, "slider interaction facts disappeared"))?;
        let value_type = interaction
            .option_expressions
            .first()
            .and_then(|expression| self.facts.try_expression_use(*expression))
            .map(|expression| expression.source.clone())
            .ok_or_else(|| self.invariant(span, "slider value type disappeared"))?;
        let style = options
            .style
            .custom
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::SliderStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "slider style extern disappeared"))
            })
            .transpose()?;
        let parent = self.declarations.view(slider).origin;
        let style_origin = options
            .style
            .custom
            .as_ref()
            .map(|_| self.origins.push(span, Some(parent)));
        let status_origins = [
            &options.style.active,
            &options.style.hovered,
            &options.style.dragged,
        ]
        .into_iter()
        .flatten()
        .map(|status| {
            self.origins
                .push(status.span.as_ref().unwrap_or(span), Some(parent))
        })
        .collect();
        if self
            .facts
            .sliders
            .insert(
                slider,
                CheckedSlider {
                    id: slider,
                    value_type,
                    style,
                    style_origin,
                    status_origins,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "slider facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_progress_facts(
        &mut self,
        progress: ViewId,
        value: &Expr,
        min: &Expr,
        max: &Expr,
        options: &ProgressOptions,
        vertical: bool,
        styles: &[String],
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let roots = crate::ast::progress_expression_roots(value, min, max, options, span);
        self.lower_interaction_facts_with_spans(
            progress,
            CheckedInteractionKind::Progress,
            crate::ast::progress_semantic_key(options, vertical, styles),
            roots
                .into_iter()
                .map(|(expression, origin)| (expression, None, origin))
                .collect(),
            Vec::new(),
            env,
            span,
        )?;
        let style = options
            .custom_style
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::ProgressStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "progress style extern disappeared"))
            })
            .transpose()?;
        let parent = self.declarations.view(progress).origin;
        let style_origin = options
            .custom_style
            .as_ref()
            .map(|_| self.origins.push(span, Some(parent)));
        if self
            .facts
            .progresses
            .insert(
                progress,
                CheckedProgress {
                    id: progress,
                    style,
                    style_origin,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "progress facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_rule_facts(
        &mut self,
        rule: ViewId,
        axis: Axis,
        thickness: &Expr,
        options: &RuleOptions,
        styles: &[String],
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            rule,
            CheckedInteractionKind::Rule,
            crate::ast::rule_semantic_key(axis, options, styles),
            crate::ast::rule_expression_roots(thickness, options)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            Vec::new(),
            env,
            span,
        )?;
        if self
            .facts
            .rules
            .insert(rule, CheckedRule { id: rule })
            .is_some()
        {
            return Err(self.invariant(span, "rule facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_qr_code_facts(
        &mut self,
        qr: ViewId,
        payload: &Expr,
        correction: Option<QrCorrection>,
        version: Option<QrVersion>,
        cell_size: &Option<Expr>,
        total_size: &Option<Expr>,
        cell: &Option<String>,
        background: &Option<String>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            qr,
            CheckedInteractionKind::QrCode,
            crate::ast::qr_code_semantic_key(
                correction, version, cell_size, total_size, cell, background,
            ),
            crate::ast::qr_code_expression_roots(payload, cell_size, total_size)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            Vec::new(),
            env,
            span,
        )?;
        let payload_type = self
            .facts
            .interaction(qr)
            .and_then(|interaction| interaction.option_expressions.first())
            .and_then(|expression| self.facts.try_expression_use(*expression))
            .map(|expression| expression.source.clone())
            .ok_or_else(|| self.invariant(span, "qr payload type disappeared"))?;
        if self
            .facts
            .qr_codes
            .insert(
                qr,
                CheckedQrCode {
                    id: qr,
                    payload_type,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "qr facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_space_facts(
        &mut self,
        space: ViewId,
        width: &Option<LengthValue>,
        height: &Option<LengthValue>,
        styles: &[String],
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts(
            space,
            CheckedInteractionKind::Space,
            crate::ast::space_semantic_key(width, height, styles),
            crate::ast::space_expression_roots(width, height)
                .into_iter()
                .map(|expression| (expression, None))
                .collect(),
            Vec::new(),
            env,
            span,
        )?;
        if self
            .facts
            .spaces
            .insert(space, CheckedSpace { id: space })
            .is_some()
        {
            return Err(self.invariant(span, "space facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_media_facts(
        &mut self,
        media: ViewId,
        kind: MediaKind,
        source: &Expr,
        options: &MediaOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let parent = self.declarations.view(media).origin;
        let roots = crate::ast::media_expression_roots(source, options);
        for (index, expression) in roots.iter().enumerate() {
            let owner = CheckedExprOwner::Media(MediaExpressionId {
                media,
                index: index as u32,
            });
            let analysis = self
                .analyses
                .site_entries
                .remove(&(
                    AnalysisSite::Media,
                    media,
                    super::expr::expr_key(expression),
                ))
                .ok_or_else(|| {
                    self.invariant(span, "missing authoritative media expression analysis")
                })?;
            record_fact_metric!(self.facts.metrics.view_analysis_passes += 1);
            self.record_analysis_metrics(&analysis);
            let origin = self.origins.push(span, Some(parent));
            self.push_analyzed_expression(
                owner,
                analysis,
                expression,
                None,
                SourceMode::Raw,
                "media",
                env,
                span,
                origin,
            )?;
        }
        let remaining = self.analyses.site_expressions(AnalysisSite::Media, media);
        if remaining != 0 {
            return Err(self.invariant(
                span,
                format!("media left {remaining} expression analyses unconsumed"),
            ));
        }
        let style = options
            .svg_style
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::SvgStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "media style extern disappeared"))
            })
            .transpose()?;
        if self
            .facts
            .media
            .insert(
                media,
                CheckedMedia {
                    id: media,
                    expression_count: roots.len() as u32,
                    semantic_key: crate::ast::media_semantic_key(kind, options),
                    style,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "media facts were produced more than once"));
        }
        if let Expr::Str(path) = source
            && !(kind == MediaKind::Svg && options.svg_memory)
        {
            self.facts.record_media_asset(path, span);
        }
        Ok(())
    }

    fn lower_tooltip_facts(
        &mut self,
        tooltip: ViewId,
        options: &TooltipOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let parent = self.declarations.view(tooltip).origin;
        let roots = crate::ast::tooltip_expression_roots(options);
        for (index, expression) in roots.iter().enumerate() {
            let owner = CheckedExprOwner::Tooltip(TooltipExpressionId {
                tooltip,
                index: index as u32,
            });
            let analysis = self
                .analyses
                .site_entries
                .remove(&(
                    AnalysisSite::Tooltip,
                    tooltip,
                    super::expr::expr_key(expression),
                ))
                .ok_or_else(|| {
                    self.invariant(span, "missing authoritative tooltip expression analysis")
                })?;
            record_fact_metric!(self.facts.metrics.view_analysis_passes += 1);
            self.record_analysis_metrics(&analysis);
            let origin = self.origins.push(span, Some(parent));
            self.push_analyzed_expression(
                owner,
                analysis,
                expression,
                None,
                SourceMode::Raw,
                "tooltip",
                env,
                span,
                origin,
            )?;
        }
        let remaining = self
            .analyses
            .site_expressions(AnalysisSite::Tooltip, tooltip);
        if remaining != 0 {
            return Err(self.invariant(
                span,
                format!("tooltip left {remaining} expression analyses unconsumed"),
            ));
        }
        let style = options
            .custom_style
            .as_ref()
            .map(|style| {
                self.declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::ContainerStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "tooltip style extern disappeared"))
            })
            .transpose()?;
        if self
            .facts
            .tooltips
            .insert(
                tooltip,
                CheckedTooltip {
                    id: tooltip,
                    expression_count: roots.len() as u32,
                    semantic_key: crate::ast::tooltip_semantic_key(options),
                    style,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "tooltip facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_float_facts(
        &mut self,
        float: ViewId,
        scale: &Expr,
        x: &Expr,
        y: &Expr,
        style: &FloatStyleOptions,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedViewFlow, Error> {
        let parent = self.declarations.view(float).origin;
        let geometry = [
            ("original_x", CheckedViewLocalRole::FloatOriginalX),
            ("original_y", CheckedViewLocalRole::FloatOriginalY),
            ("original_width", CheckedViewLocalRole::FloatOriginalWidth),
            ("original_height", CheckedViewLocalRole::FloatOriginalHeight),
            ("viewport_x", CheckedViewLocalRole::FloatViewportX),
            ("viewport_y", CheckedViewLocalRole::FloatViewportY),
            ("viewport_width", CheckedViewLocalRole::FloatViewportWidth),
            ("viewport_height", CheckedViewLocalRole::FloatViewportHeight),
        ]
        .map(|(name, role)| self.push_view_local(name, Type::F64, float, role, span));
        let mut translate_env = ScopedFactEnv::new(env);
        for (name, local) in [
            "original_x",
            "original_y",
            "original_width",
            "original_height",
            "viewport_x",
            "viewport_y",
            "viewport_width",
            "viewport_height",
        ]
        .into_iter()
        .zip(geometry)
        {
            translate_env.insert(name.into(), CheckedPathRoot::Local(local), Type::F64);
        }
        record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
        let roots = crate::ast::float_expression_roots(scale, x, y, style);
        for (index, expression) in roots.iter().enumerate() {
            let expression_env: &dyn FactEnvironment = if matches!(index, 1 | 2) {
                &translate_env
            } else {
                env
            };
            self.push_float_expression(
                float,
                index as u32,
                expression,
                expression_env,
                span,
                parent,
            )?;
        }
        let remaining = self.analyses.site_expressions(AnalysisSite::Float, float);
        if remaining != 0 {
            return Err(self.invariant(
                span,
                format!("float left {remaining} expression analyses unconsumed"),
            ));
        }
        Ok(CheckedViewFlow::Float {
            semantic_key: crate::ast::float_semantic_key(style),
            expression_count: roots.len() as u32,
            geometry,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn push_float_expression(
        &mut self,
        float: ViewId,
        index: u32,
        expression: &Expr,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Float(FloatExpressionId { float, index });
        let analysis = self
            .analyses
            .site_entries
            .remove(&(
                AnalysisSite::Float,
                float,
                super::expr::expr_key(expression),
            ))
            .ok_or_else(|| {
                self.invariant(span, "missing authoritative float expression analysis")
            })?;
        record_fact_metric!(self.facts.metrics.view_analysis_passes += 1);
        self.record_analysis_metrics(&analysis);
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expression,
            None,
            SourceMode::Raw,
            "float",
            env,
            span,
            origin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pin_facts(
        &mut self,
        pin: ViewId,
        width: &Option<LengthValue>,
        height: &Option<LengthValue>,
        x: &Expr,
        y: &Expr,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedViewFlow, Error> {
        let parent = self.declarations.view(pin).origin;
        let roots = crate::ast::pin_expression_roots(width, height, x, y);
        for (index, expression) in roots.iter().enumerate() {
            self.push_pin_expression(pin, index as u32, expression, env, span, parent)?;
        }
        let remaining = self.analyses.site_expressions(AnalysisSite::Pin, pin);
        if remaining != 0 {
            return Err(self.invariant(
                span,
                format!("pin left {remaining} expression analyses unconsumed"),
            ));
        }
        Ok(CheckedViewFlow::Pin {
            semantic_key: crate::ast::pin_semantic_key(width, height),
            expression_count: roots.len() as u32,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn push_pin_expression(
        &mut self,
        pin: ViewId,
        index: u32,
        expression: &Expr,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Pin(PinExpressionId { pin, index });
        let analysis = self
            .analyses
            .site_entries
            .remove(&(AnalysisSite::Pin, pin, super::expr::expr_key(expression)))
            .ok_or_else(|| self.invariant(span, "missing authoritative pin expression analysis"))?;
        record_fact_metric!(self.facts.metrics.view_analysis_passes += 1);
        self.record_analysis_metrics(&analysis);
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expression,
            None,
            SourceMode::Raw,
            "pin",
            env,
            span,
            origin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_interaction_facts(
        &mut self,
        widget: ViewId,
        kind: CheckedInteractionKind,
        semantic_key: String,
        option_expressions: Vec<(&Expr, Option<Type>)>,
        routes: Vec<&Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        self.lower_interaction_facts_with_spans(
            widget,
            kind,
            semantic_key,
            option_expressions
                .into_iter()
                .map(|(expression, destination)| (expression, destination, span))
                .collect(),
            routes,
            env,
            span,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_interaction_facts_with_spans(
        &mut self,
        widget: ViewId,
        kind: CheckedInteractionKind,
        semantic_key: String,
        option_expressions: Vec<(&Expr, Option<Type>, &Span)>,
        routes: Vec<&Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let parent = self.declarations.view(widget).origin;
        let mut expression_count = 0u32;
        let option_expressions = option_expressions
            .into_iter()
            .map(|(expression, destination, expression_span)| {
                self.push_interaction_expression(
                    widget,
                    &mut expression_count,
                    expression,
                    destination.as_ref(),
                    env,
                    expression_span,
                    parent,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.finish_interaction_facts(
            widget,
            kind,
            semantic_key,
            expression_count,
            option_expressions,
            routes,
            env,
            span,
        )
    }

    /// Lowers the routes and records the interaction after its option
    /// expressions have been pushed — the shared tail for widgets that push
    /// their expressions with one environment and for `rich-text`, whose
    /// `for` children push theirs under loop-scoped environments.
    #[allow(clippy::too_many_arguments)]
    fn finish_interaction_facts(
        &mut self,
        widget: ViewId,
        kind: CheckedInteractionKind,
        semantic_key: String,
        mut expression_count: u32,
        option_expressions: Vec<CheckedExprUseId>,
        routes: Vec<&Route>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let parent = self.declarations.view(widget).origin;
        let mut checked_routes = Vec::with_capacity(routes.len());
        for (index, route) in routes.into_iter().enumerate() {
            checked_routes.push(self.lower_interaction_route(
                widget,
                index as u32,
                route,
                env,
                &mut expression_count,
                parent,
            )?);
        }
        let remaining_expressions = self
            .analyses
            .site_expressions(AnalysisSite::Interaction, widget);
        let remaining_routes = self.analyses.site_routes(AnalysisSite::Interaction, widget);
        if remaining_expressions != 0 || remaining_routes != 0 {
            return Err(self.invariant(
                span,
                format!(
                    "interaction widget left {remaining_expressions} expression and {remaining_routes} route analyses unconsumed"
                ),
            ));
        }
        if self
            .facts
            .interactions
            .insert(
                widget,
                CheckedInteraction {
                    id: widget,
                    kind,
                    semantic_key,
                    expression_count,
                    option_expressions,
                    routes: checked_routes,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "interaction facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_interaction_route(
        &mut self,
        widget: ViewId,
        index: u32,
        route: &Route,
        env: &dyn FactEnvironment,
        expression_count: &mut u32,
        parent: OriginId,
    ) -> Result<CheckedInteractionRoute, Error> {
        let id = InteractionRouteId { widget, index };
        let inputs = self
            .analyses
            .site_route_inputs
            .remove(&(
                AnalysisSite::Interaction,
                widget,
                std::ptr::from_ref(route).addr(),
            ))
            .ok_or_else(|| {
                self.invariant(
                    &route.span,
                    "interaction route has no retained payload contract",
                )
            })?;
        let scope = self.facts.view(widget).scope;
        let (target, target_params, route_args) = match scope {
            CheckedViewScope::Component(component) if route.handler == "emit" => {
                let named = route.args.first().and_then(|argument| {
                    let RouteArg::Expr(Expr::Path(path)) = argument else {
                        return None;
                    };
                    let [name] = path.as_slice() else {
                        return None;
                    };
                    self.declarations.component_event_by_name(component, name)
                });
                if let Some(event) = named {
                    (
                        CheckedCanvasRouteTarget::ComponentEvent {
                            event: event.declaration.id,
                            name: event.name.clone(),
                            payloads: event.payloads.clone(),
                        },
                        event.payloads.clone(),
                        &route.args[1..],
                    )
                } else {
                    let output = self
                        .declarations
                        .component_output(component)
                        .cloned()
                        .ok_or_else(|| {
                            self.invariant(&route.span, "interaction component output disappeared")
                        })?;
                    (
                        CheckedCanvasRouteTarget::ComponentOutput {
                            component,
                            output: output.clone(),
                        },
                        vec![output],
                        route.args.as_slice(),
                    )
                }
            }
            scope => {
                let handler = self
                    .declarations
                    .handler_id(route_handler_owner(scope), &route.handler)
                    .ok_or_else(|| {
                        self.invariant(&route.span, "interaction route target disappeared")
                    })?;
                (
                    CheckedCanvasRouteTarget::Handler(handler),
                    self.declarations.handler(handler).payloads.clone(),
                    route.args.as_slice(),
                )
            }
        };
        if target_params.len() != route_args.len() {
            return Err(self.invariant(&route.span, "interaction route arity changed"));
        }
        let origin = self.origins.push(&route.span, Some(parent));
        let mut args = Vec::with_capacity(route_args.len());
        for (argument_index, (argument, expected)) in
            route_args.iter().zip(&target_params).enumerate()
        {
            match argument {
                RouteArg::Expr(expression) => {
                    let expression = self.push_interaction_expression(
                        widget,
                        expression_count,
                        expression,
                        Some(expected),
                        env,
                        &route.span,
                        origin,
                    )?;
                    args.push(CheckedCanvasRouteArg::Expression(expression));
                }
                RouteArg::Payload => {
                    let payload_index = if inputs.ordered {
                        args.iter()
                            .filter(|arg| matches!(arg, CheckedCanvasRouteArg::Payload))
                            .count()
                    } else {
                        0
                    };
                    let actual = inputs.payloads.get(payload_index).ok_or_else(|| {
                        self.invariant(&route.span, "interaction route payload has no source")
                    })?;
                    if actual != expected {
                        return Err(self.invariant(
                            &route.span,
                            format!("interaction route payload {argument_index} changed type"),
                        ));
                    }
                    args.push(CheckedCanvasRouteArg::Payload);
                }
            }
        }
        Ok(CheckedInteractionRoute {
            id,
            target,
            args,
            source_payloads: inputs.payloads,
            ordered_payloads: inputs.ordered,
            origin,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn push_interaction_expression(
        &mut self,
        widget: ViewId,
        index: &mut u32,
        expression: &Expr,
        destination: Option<&Type>,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
            widget,
            index: *index,
        });
        *index += 1;
        let analysis = self
            .analyses
            .site_entries
            .remove(&(
                AnalysisSite::Interaction,
                widget,
                super::expr::expr_key(expression),
            ))
            .ok_or_else(|| {
                self.invariant(
                    span,
                    "missing authoritative interaction expression analysis",
                )
            })?;
        self.record_analysis_metrics(&analysis);
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expression,
            destination,
            SourceMode::Contextual,
            "interaction",
            env,
            span,
            origin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_optional_expression(
        &mut self,
        pane: ViewId,
        expression: Option<&Expr>,
        expected: Option<&Type>,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
        expression_count: &mut u32,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        expression
            .map(|expression| {
                self.push_interaction_expression(
                    pane,
                    expression_count,
                    expression,
                    expected,
                    env,
                    span,
                    parent,
                )
            })
            .transpose()
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_length(
        &mut self,
        pane: ViewId,
        length: &Option<LengthValue>,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
        expression_count: &mut u32,
    ) -> Result<CheckedLength, Error> {
        Ok(match length {
            None => CheckedLength::None,
            Some(LengthValue::Fill) => CheckedLength::Fill,
            Some(LengthValue::FillPortion(portion)) => CheckedLength::FillPortion(*portion),
            Some(LengthValue::Shrink) => CheckedLength::Shrink,
            Some(LengthValue::Fixed(expression)) => {
                let expression = self.push_interaction_expression(
                    pane,
                    expression_count,
                    expression,
                    None,
                    env,
                    span,
                    parent,
                )?;
                let source = self.facts.expression_use(expression).source.clone();
                CheckedLength::Fixed { expression, source }
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_background(
        &mut self,
        pane: ViewId,
        background: &BackgroundValue,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
        expression_count: &mut u32,
    ) -> Result<CheckedPaneBackground, Error> {
        Ok(match background {
            BackgroundValue::Color(color) => CheckedPaneBackground::Color(color.clone()),
            BackgroundValue::Linear { angle, stops } => {
                let angle = self.push_interaction_expression(
                    pane,
                    expression_count,
                    angle,
                    Some(&Type::F64),
                    env,
                    span,
                    parent,
                )?;
                let stops = stops
                    .iter()
                    .map(|stop| {
                        Ok(CheckedPaneGradientStop {
                            color: stop.color.clone(),
                            offset: self.push_interaction_expression(
                                pane,
                                expression_count,
                                &stop.offset,
                                Some(&Type::F64),
                                env,
                                span,
                                parent,
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                CheckedPaneBackground::Linear { angle, stops }
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_radius(
        &mut self,
        pane: ViewId,
        all: Option<&Expr>,
        corners: [Option<&Expr>; 4],
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
        expression_count: &mut u32,
    ) -> Result<CheckedPaneRadius, Error> {
        Ok(CheckedPaneRadius {
            all: self.lower_pane_optional_expression(
                pane,
                all,
                Some(&Type::F64),
                env,
                span,
                parent,
                expression_count,
            )?,
            top_left: self.lower_pane_optional_expression(
                pane,
                corners[0],
                Some(&Type::F64),
                env,
                span,
                parent,
                expression_count,
            )?,
            top_right: self.lower_pane_optional_expression(
                pane,
                corners[1],
                Some(&Type::F64),
                env,
                span,
                parent,
                expression_count,
            )?,
            bottom_right: self.lower_pane_optional_expression(
                pane,
                corners[2],
                Some(&Type::F64),
                env,
                span,
                parent,
                expression_count,
            )?,
            bottom_left: self.lower_pane_optional_expression(
                pane,
                corners[3],
                Some(&Type::F64),
                env,
                span,
                parent,
                expression_count,
            )?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_surface(
        &mut self,
        pane: ViewId,
        style: &ContainerStyleOptions,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
        expression_count: &mut u32,
    ) -> Result<CheckedPaneSurface, Error> {
        let background = style
            .background
            .as_ref()
            .map(|background| {
                self.lower_pane_background(pane, background, env, span, parent, expression_count)
            })
            .transpose()?;
        let border_width = self.lower_pane_optional_expression(
            pane,
            style.border_width.as_ref(),
            Some(&Type::F64),
            env,
            span,
            parent,
            expression_count,
        )?;
        let radius = self.lower_pane_radius(
            pane,
            style.radius.all.as_ref(),
            [
                style.radius.top_left.as_ref(),
                style.radius.top_right.as_ref(),
                style.radius.bottom_right.as_ref(),
                style.radius.bottom_left.as_ref(),
            ],
            env,
            span,
            parent,
            expression_count,
        )?;
        let shadow_x = self.lower_pane_optional_expression(
            pane,
            style.shadow_x.as_ref(),
            Some(&Type::F64),
            env,
            span,
            parent,
            expression_count,
        )?;
        let shadow_y = self.lower_pane_optional_expression(
            pane,
            style.shadow_y.as_ref(),
            Some(&Type::F64),
            env,
            span,
            parent,
            expression_count,
        )?;
        let shadow_blur = self.lower_pane_optional_expression(
            pane,
            style.shadow_blur.as_ref(),
            Some(&Type::F64),
            env,
            span,
            parent,
            expression_count,
        )?;
        let pixel_snap = self.lower_pane_optional_expression(
            pane,
            style.pixel_snap.as_ref(),
            Some(&Type::Bool),
            env,
            span,
            parent,
            expression_count,
        )?;
        Ok(CheckedPaneSurface {
            background,
            text_color: style.text_color.clone(),
            border_color: style.border_color.clone(),
            border_width,
            radius,
            shadow_color: style.shadow_color.clone(),
            shadow_x,
            shadow_y,
            shadow_blur,
            pixel_snap,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_padding(
        &mut self,
        pane: ViewId,
        padding: &PaddingOptions,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
        expression_count: &mut u32,
    ) -> Result<CheckedPadding, Error> {
        let mut take = |value: Option<&Expr>| {
            self.lower_pane_optional_expression(
                pane,
                value,
                Some(&Type::F64),
                env,
                span,
                parent,
                expression_count,
            )
        };
        Ok(CheckedPadding {
            all: take(padding.all.as_ref())?,
            x: take(padding.x.as_ref())?,
            y: take(padding.y.as_ref())?,
            top: take(padding.top.as_ref())?,
            right: take(padding.right.as_ref())?,
            bottom: take(padding.bottom.as_ref())?,
            left: take(padding.left.as_ref())?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_view_body(
        &mut self,
        pane: ViewId,
        source: &PaneView,
        maximized: Option<CheckedLocalId>,
        env: &dyn FactEnvironment,
        origin: OriginId,
        expression_count: &mut u32,
    ) -> Result<CheckedPaneView, Error> {
        let surface = self.lower_pane_surface(
            pane,
            &source.style,
            env,
            &source.span,
            origin,
            expression_count,
        )?;
        let title = source
            .title
            .as_ref()
            .map(|title| {
                let title_origin = self.origins.push(&title.span, Some(origin));
                Ok(CheckedPaneTitle {
                    padding: self.lower_pane_padding(
                        pane,
                        &title.padding,
                        env,
                        &title.span,
                        title_origin,
                        expression_count,
                    )?,
                    always_show_controls: title.always_show_controls,
                    has_controls: title.controls.is_some(),
                    has_compact_controls: title.compact_controls.is_some(),
                    surface: self.lower_pane_surface(
                        pane,
                        &title.style,
                        env,
                        &title.span,
                        title_origin,
                        expression_count,
                    )?,
                    style_site: CheckedPaneStyleSite {
                        line: title.span.line,
                        column: title.span.column,
                    },
                    origin: title_origin,
                })
            })
            .transpose()?;
        for child in source.nodes() {
            self.lower_view_expression_tree(child, env)?;
        }
        Ok(CheckedPaneView {
            name: source.name.clone(),
            maximized,
            surface,
            style_site: CheckedPaneStyleSite {
                line: source.span.line,
                column: source.span.column,
            },
            title,
            origin,
        })
    }

    fn checked_pane_configuration(source: &PaneConfiguration) -> CheckedPaneConfiguration {
        match source {
            PaneConfiguration::Pane(name) => CheckedPaneConfiguration::Pane(name.clone()),
            PaneConfiguration::Split {
                name,
                axis,
                ratio,
                a,
                b,
            } => CheckedPaneConfiguration::Split {
                name: name.clone(),
                axis: match axis {
                    PaneAxis::Horizontal => CheckedPaneAxis::Horizontal,
                    PaneAxis::Vertical => CheckedPaneAxis::Vertical,
                },
                ratio: *ratio,
                a: Box::new(Self::checked_pane_configuration(a)),
                b: Box::new(Self::checked_pane_configuration(b)),
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_pane_grid_facts(
        &mut self,
        pane: ViewId,
        name: &str,
        configuration: &PaneConfiguration,
        options: &PaneGridOptions,
        panes: &[PaneView],
        templates: &[PaneTemplate],
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let origin = self.declarations.view(pane).origin;
        let mut expression_count = 0u32;
        let width = self.lower_pane_length(
            pane,
            &options.width,
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let height = self.lower_pane_length(
            pane,
            &options.height,
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let spacing = self.lower_pane_optional_expression(
            pane,
            options.spacing.as_ref(),
            Some(&Type::F64),
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let min_size = self.lower_pane_optional_expression(
            pane,
            options.min_size.as_ref(),
            Some(&Type::F64),
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let resize_leeway = self.lower_pane_optional_expression(
            pane,
            options.resize_leeway.as_ref(),
            Some(&Type::F64),
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let custom_style = options
            .custom_style
            .as_ref()
            .map(|style| {
                let function = self
                    .declarations
                    .extern_decl_by_name(&style.function)
                    .filter(|function| function.kind == ExternKind::PaneGridStyle)
                    .map(|function| function.declaration.id)
                    .ok_or_else(|| self.invariant(span, "pane style extern disappeared"))?;
                let arguments = style
                    .args
                    .iter()
                    .map(|argument| {
                        self.push_interaction_expression(
                            pane,
                            &mut expression_count,
                            argument,
                            None,
                            env,
                            span,
                            origin,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CheckedPaneCustomStyle {
                    function,
                    arguments,
                })
            })
            .transpose()?;
        let region_background = options
            .style
            .region_background
            .as_ref()
            .map(|background| {
                self.lower_pane_background(
                    pane,
                    background,
                    env,
                    span,
                    origin,
                    &mut expression_count,
                )
            })
            .transpose()?;
        let region_border_width = self.lower_pane_optional_expression(
            pane,
            options.style.region_border_width.as_ref(),
            Some(&Type::F64),
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let region_radius = self.lower_pane_radius(
            pane,
            options.style.region_radius.all.as_ref(),
            [
                options.style.region_radius.top_left.as_ref(),
                options.style.region_radius.top_right.as_ref(),
                options.style.region_radius.bottom_right.as_ref(),
                options.style.region_radius.bottom_left.as_ref(),
            ],
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let hovered_split_width = self.lower_pane_optional_expression(
            pane,
            options.style.hovered_split_width.as_ref(),
            Some(&Type::F64),
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let picked_split_width = self.lower_pane_optional_expression(
            pane,
            options.style.picked_split_width.as_ref(),
            Some(&Type::F64),
            env,
            span,
            origin,
            &mut expression_count,
        )?;
        let click = options
            .click
            .as_ref()
            .map(|route| {
                self.lower_interaction_route(pane, 0, route, env, &mut expression_count, origin)
            })
            .transpose()?;

        let mut checked_panes = Vec::with_capacity(panes.len());
        for (index, source) in panes.iter().enumerate() {
            let pane_origin = self.origins.push(&source.span, Some(origin));
            let maximized = source.maximized.as_ref().map(|binding| {
                self.push_view_local_with_parent(
                    binding,
                    Type::Bool,
                    pane,
                    CheckedViewLocalRole::PaneMaximized(index as u32),
                    &source.span,
                    pane_origin,
                )
            });
            let checked = if let (Some(binding), Some(local)) = (&source.maximized, maximized) {
                let scoped = LayeredFactEnv {
                    base: env,
                    name: binding.clone(),
                    value: (CheckedPathRoot::Local(local), Type::Bool),
                };
                record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
                self.lower_pane_view_body(
                    pane,
                    source,
                    maximized,
                    &scoped,
                    pane_origin,
                    &mut expression_count,
                )?
            } else {
                self.lower_pane_view_body(
                    pane,
                    source,
                    maximized,
                    env,
                    pane_origin,
                    &mut expression_count,
                )?
            };
            checked_panes.push(checked);
        }

        let mut checked_templates = Vec::with_capacity(templates.len());
        for (index, template) in templates.iter().enumerate() {
            let template_origin = self.origins.push(&template.span, Some(origin));
            let (items, item_ty) = env.get(&template.items).cloned().ok_or_else(|| {
                self.invariant(&template.span, "pane template list has no checked path")
            })?;
            let Type::List(item_ty) = item_ty else {
                return Err(
                    self.invariant(&template.span, "pane template checked path is not a list")
                );
            };
            let item_local = self.push_view_local_with_parent(
                &template.item,
                item_ty.as_ref().clone(),
                pane,
                CheckedViewLocalRole::PaneTemplateItem(index as u32),
                &template.span,
                template_origin,
            );
            let item_scoped = LayeredFactEnv {
                base: env,
                name: template.item.clone(),
                value: (CheckedPathRoot::Local(item_local), item_ty.as_ref().clone()),
            };
            record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
            let key = self.push_interaction_expression(
                pane,
                &mut expression_count,
                &template.key,
                None,
                &item_scoped,
                &template.span,
                template_origin,
            )?;
            let key_type = self.facts.expression_use(key).source.clone();
            let pane_origin = self
                .origins
                .push(&template.pane.span, Some(template_origin));
            let maximized = template.pane.maximized.as_ref().map(|binding| {
                self.push_view_local_with_parent(
                    binding,
                    Type::Bool,
                    pane,
                    CheckedViewLocalRole::PaneTemplateMaximized(index as u32),
                    &template.pane.span,
                    pane_origin,
                )
            });
            let checked_pane =
                if let (Some(binding), Some(local)) = (&template.pane.maximized, maximized) {
                    let scoped = LayeredFactEnv {
                        base: &item_scoped,
                        name: binding.clone(),
                        value: (CheckedPathRoot::Local(local), Type::Bool),
                    };
                    record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
                    self.lower_pane_view_body(
                        pane,
                        &template.pane,
                        maximized,
                        &scoped,
                        pane_origin,
                        &mut expression_count,
                    )?
                } else {
                    self.lower_pane_view_body(
                        pane,
                        &template.pane,
                        maximized,
                        &item_scoped,
                        pane_origin,
                        &mut expression_count,
                    )?
                };
            checked_templates.push(CheckedPaneTemplate {
                items,
                item: item_local,
                key,
                key_type,
                pane: checked_pane,
                origin: template_origin,
            });
        }

        let remaining_expressions = self
            .analyses
            .site_expressions(AnalysisSite::Interaction, pane);
        let remaining_routes = self.analyses.site_routes(AnalysisSite::Interaction, pane);
        if remaining_expressions != 0 || remaining_routes != 0 {
            return Err(self.invariant(span, "pane grid left authoritative analyses unconsumed"));
        }
        let checked = CheckedPaneGrid {
            id: pane,
            name: name.to_owned(),
            configuration: Self::checked_pane_configuration(configuration),
            width,
            height,
            spacing,
            min_size,
            resize_leeway,
            draggable: options.draggable,
            click,
            custom_style,
            style: CheckedPaneGridStyle {
                region_background,
                region_border: options.style.region_border.clone(),
                region_border_width,
                region_radius,
                hovered_split: options.style.hovered_split.clone(),
                hovered_split_width,
                picked_split: options.style.picked_split.clone(),
                picked_split_width,
            },
            panes: checked_panes,
            templates: checked_templates,
            expression_count,
            origin,
        };
        if self.facts.pane_grids.insert(pane, checked).is_some() {
            return Err(self.invariant(span, "pane grid facts were produced more than once"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_canvas_facts(
        &mut self,
        canvas: ViewId,
        options: &CanvasOptions,
        locals: &[State],
        commands: &[CanvasCommand],
        events: &[CanvasEvent],
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<(), Error> {
        let declaration = self
            .declarations
            .canvas(canvas)
            .cloned()
            .ok_or_else(|| self.invariant(span, "canvas has no stable declaration"))?;
        if declaration.locals.len() != locals.len()
            || declaration.commands.len() != crate::ast::canvas_command_spans(commands).len()
            || declaration.events.len() != events.len()
            || declaration.routes.len() != crate::ast::canvas_routes(options, events).len()
        {
            return Err(self.invariant(span, "canvas declaration topology changed"));
        }
        for (path, span) in crate::ast::canvas_media_path_literals(commands) {
            self.facts.record_media_asset(path, span);
        }

        let mut counters = CanvasFactCounters::default();
        let mut checked_routes = Vec::with_capacity(declaration.routes.len());
        for length in [&options.width, &options.height].into_iter().flatten() {
            if let LengthValue::Fixed(expression) = length {
                self.push_canvas_expression(
                    canvas,
                    &mut counters.expression,
                    expression,
                    None,
                    false,
                    env,
                    span,
                    declaration.declaration.origin,
                )?;
            }
        }
        for expression in [&options.cache, &options.capture].into_iter().flatten() {
            self.push_canvas_expression(
                canvas,
                &mut counters.expression,
                expression,
                None,
                false,
                env,
                span,
                declaration.declaration.origin,
            )?;
        }
        for route in crate::ast::canvas_routes(options, &[]) {
            checked_routes.push(self.lower_canvas_route(canvas, route, env, &mut counters)?);
        }

        let empty = FactEnv::default();
        let mut state_locals = Vec::with_capacity(locals.len());
        let mut canvas_env = ScopedFactEnv::new(env);
        for (index, local) in locals.iter().enumerate() {
            let local_id = CanvasLocalId {
                canvas,
                index: index as u32,
            };
            let retained = declaration
                .locals
                .get(index)
                .ok_or_else(|| self.invariant(&local.span, "canvas state has no declaration"))?;
            if retained.declaration.id != local_id
                || retained.name != local.name
                || retained.ty != local.ty
            {
                return Err(self.invariant(
                    &local.span,
                    "canvas state diverged from its checked declaration",
                ));
            }
            self.push_canvas_expression(
                canvas,
                &mut counters.expression,
                &local.initial,
                Some(&retained.ty),
                true,
                &empty,
                &local.span,
                retained.declaration.origin,
            )?;
            let checked = self.push_local(
                &retained.name,
                retained.ty.clone(),
                CheckedLocalOwner::CanvasState(local_id),
                &local.span,
                declaration.declaration.origin,
            );
            canvas_env.insert(
                retained.name.clone(),
                CheckedPathRoot::Local(checked),
                retained.ty.clone(),
            );
            state_locals.push(checked);
        }
        let width = self.push_local(
            "canvas_width",
            Type::F64,
            CheckedLocalOwner::CanvasWidth(canvas),
            span,
            declaration.declaration.origin,
        );
        canvas_env.insert(
            "canvas_width".into(),
            CheckedPathRoot::Local(width),
            Type::F64,
        );
        let height = self.push_local(
            "canvas_height",
            Type::F64,
            CheckedLocalOwner::CanvasHeight(canvas),
            span,
            declaration.declaration.origin,
        );
        canvas_env.insert(
            "canvas_height".into(),
            CheckedPathRoot::Local(height),
            Type::F64,
        );

        for expression in [&options.interaction_expr, &options.interaction_outside]
            .into_iter()
            .flatten()
        {
            self.push_canvas_expression(
                canvas,
                &mut counters.expression,
                expression,
                None,
                false,
                &canvas_env,
                span,
                declaration.declaration.origin,
            )?;
        }
        self.lower_canvas_commands(canvas, commands, &canvas_env, &declaration, &mut counters)?;

        for (event_index, event) in events.iter().enumerate() {
            let event_id = CanvasEventId {
                canvas,
                index: event_index as u32,
            };
            let event_declaration = self
                .declarations
                .canvas_event(event_id)
                .ok_or_else(|| self.invariant(&event.span, "canvas event has no declaration"))?;
            let payloads = native_subscription_payloads(&event.source, false).ok_or_else(|| {
                self.invariant(&event.span, "canvas event has no native payload contract")
            })?;
            let mut event_env = ScopedFactEnv::new(&canvas_env);
            for (index, (name, ty)) in event.bindings.iter().zip(&payloads).enumerate() {
                let local = self.push_local(
                    name,
                    ty.clone(),
                    CheckedLocalOwner::CanvasEventBinding {
                        event: event_id,
                        index: index as u32,
                    },
                    &event.span,
                    event_declaration.origin,
                );
                event_env.insert(name.clone(), CheckedPathRoot::Local(local), ty.clone());
            }
            for update in &event.updates {
                self.push_canvas_expression(
                    canvas,
                    &mut counters.expression,
                    &update.value,
                    None,
                    false,
                    &event_env,
                    &update.span,
                    event_declaration.origin,
                )?;
            }
            if let Some(CanvasEventAction::Route(route)) = &event.action {
                let route_env: &dyn FactEnvironment =
                    if event.route_payload { env } else { &event_env };
                let route = self.lower_canvas_route(canvas, route, route_env, &mut counters)?;
                checked_routes.push(route);
            }
        }

        let remaining_expressions = self.analyses.site_expressions(AnalysisSite::Canvas, canvas);
        let remaining_routes = self.analyses.site_routes(AnalysisSite::Canvas, canvas);
        if remaining_expressions != 0 || remaining_routes != 0 {
            let remaining_sources =
                crate::ast::canvas_expression_roots(options, locals, commands, events)
                    .into_iter()
                    .filter(|expression| {
                        self.analyses.site_entries.contains_key(&(
                            AnalysisSite::Canvas,
                            canvas,
                            super::expr::expr_key(expression),
                        ))
                    })
                    .map(|expression| format!("{expression:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
            return Err(self.invariant(
                span,
                format!(
                    "canvas left {remaining_expressions} expression and {remaining_routes} route analyses unconsumed: {remaining_sources}"
                ),
            ));
        }
        if counters.command as usize != declaration.commands.len()
            || counters.route as usize != declaration.routes.len()
        {
            return Err(self.invariant(span, "canvas did not consume its declaration arenas"));
        }
        if self
            .facts
            .canvases
            .insert(
                canvas,
                CheckedCanvas {
                    id: canvas,
                    states: state_locals,
                    width,
                    height,
                    expression_count: counters.expression,
                    routes: checked_routes,
                },
            )
            .is_some()
        {
            return Err(self.invariant(span, "canvas facts were produced more than once"));
        }
        Ok(())
    }

    fn lower_canvas_commands(
        &mut self,
        canvas: ViewId,
        commands: &[CanvasCommand],
        env: &dyn FactEnvironment,
        declaration: &crate::hir::CanvasDeclaration,
        counters: &mut CanvasFactCounters,
    ) -> Result<(), Error> {
        for command in commands {
            let command_id = CanvasCommandId {
                canvas,
                index: counters.command,
            };
            counters.command += 1;
            let retained = declaration
                .commands
                .get(command_id.index as usize)
                .filter(|value| value.declaration.id == command_id)
                .ok_or_else(|| {
                    self.invariant(
                        crate::ast::canvas_command_span(command),
                        "canvas command has no declaration",
                    )
                })?;
            let mut direct = Vec::new();
            for expression in crate::ast::canvas_command_direct_expression_roots(command) {
                direct.push(self.push_canvas_expression(
                    canvas,
                    &mut counters.expression,
                    expression,
                    None,
                    false,
                    env,
                    crate::ast::canvas_command_span(command),
                    retained.declaration.origin,
                )?);
            }
            match command {
                CanvasCommand::Group { commands, .. } | CanvasCommand::If { commands, .. } => {
                    self.lower_canvas_commands(canvas, commands, env, declaration, counters)?;
                }
                CanvasCommand::For { item, commands, .. } => {
                    let items = direct.first().copied().ok_or_else(|| {
                        self.invariant(
                            crate::ast::canvas_command_span(command),
                            "canvas for has no checked items",
                        )
                    })?;
                    let Type::List(item_ty) = self.facts.expression_use(items).source.clone()
                    else {
                        return Err(self.invariant(
                            crate::ast::canvas_command_span(command),
                            "canvas for checked items are not a list",
                        ));
                    };
                    let local = self.push_local(
                        item,
                        item_ty.as_ref().clone(),
                        CheckedLocalOwner::CanvasCommandItem(command_id),
                        crate::ast::canvas_command_span(command),
                        retained.declaration.origin,
                    );
                    let scoped = LayeredFactEnv {
                        base: env,
                        name: item.clone(),
                        value: (CheckedPathRoot::Local(local), item_ty.as_ref().clone()),
                    };
                    record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
                    self.lower_canvas_commands(canvas, commands, &scoped, declaration, counters)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn lower_canvas_route(
        &mut self,
        canvas: ViewId,
        route: &Route,
        env: &dyn FactEnvironment,
        counters: &mut CanvasFactCounters,
    ) -> Result<CheckedCanvasRoute, Error> {
        let id = CanvasRouteId {
            canvas,
            index: counters.route,
        };
        counters.route += 1;
        let declaration = self
            .declarations
            .canvas_route(id)
            .ok_or_else(|| self.invariant(&route.span, "canvas route has no declaration"))?;
        let inputs = self
            .analyses
            .site_route_inputs
            .remove(&(
                AnalysisSite::Canvas,
                canvas,
                std::ptr::from_ref(route).addr(),
            ))
            .ok_or_else(|| {
                self.invariant(&route.span, "canvas route has no retained payload contract")
            })?;
        let scope = self.facts.view(canvas).scope;
        let (target, target_params, route_args) = match scope {
            CheckedViewScope::Component(component) if route.handler == "emit" => {
                let named = route.args.first().and_then(|argument| {
                    let RouteArg::Expr(Expr::Path(path)) = argument else {
                        return None;
                    };
                    let [name] = path.as_slice() else {
                        return None;
                    };
                    self.declarations.component_event_by_name(component, name)
                });
                if let Some(event) = named {
                    (
                        CheckedCanvasRouteTarget::ComponentEvent {
                            event: event.declaration.id,
                            name: event.name.clone(),
                            payloads: event.payloads.clone(),
                        },
                        event.payloads.clone(),
                        &route.args[1..],
                    )
                } else {
                    let output = self
                        .declarations
                        .component_output(component)
                        .cloned()
                        .ok_or_else(|| {
                            self.invariant(&route.span, "canvas component output disappeared")
                        })?;
                    (
                        CheckedCanvasRouteTarget::ComponentOutput {
                            component,
                            output: output.clone(),
                        },
                        vec![output],
                        route.args.as_slice(),
                    )
                }
            }
            scope => {
                let handler = self
                    .declarations
                    .handler_id(route_handler_owner(scope), &route.handler)
                    .ok_or_else(|| {
                        self.invariant(&route.span, "canvas route target disappeared")
                    })?;
                (
                    CheckedCanvasRouteTarget::Handler(handler),
                    self.declarations.handler(handler).payloads.clone(),
                    route.args.as_slice(),
                )
            }
        };
        if target_params.len() != route_args.len() {
            return Err(self.invariant(&route.span, "canvas route arity changed"));
        }
        let mut args = Vec::with_capacity(route_args.len());
        for (index, (argument, expected)) in route_args.iter().zip(&target_params).enumerate() {
            match argument {
                RouteArg::Expr(expression) => {
                    let expression = self.push_canvas_expression(
                        canvas,
                        &mut counters.expression,
                        expression,
                        Some(expected),
                        false,
                        env,
                        &route.span,
                        declaration.origin,
                    )?;
                    args.push(CheckedCanvasRouteArg::Expression(expression));
                }
                RouteArg::Payload => {
                    let payload_index = if inputs.ordered {
                        args.iter()
                            .filter(|arg| matches!(arg, CheckedCanvasRouteArg::Payload))
                            .count()
                    } else {
                        0
                    };
                    let actual = inputs.payloads.get(payload_index).ok_or_else(|| {
                        self.invariant(&route.span, "canvas route payload has no source")
                    })?;
                    if actual != expected {
                        return Err(self.invariant(
                            &route.span,
                            format!("canvas route payload {index} changed type"),
                        ));
                    }
                    args.push(CheckedCanvasRouteArg::Payload);
                }
            }
        }
        Ok(CheckedCanvasRoute {
            id,
            target,
            args,
            source_payloads: inputs.payloads,
            ordered_payloads: inputs.ordered,
            origin: declaration.origin,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn push_canvas_expression(
        &mut self,
        canvas: ViewId,
        operand: &mut u32,
        expression: &Expr,
        destination: Option<&Type>,
        initializer: bool,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Canvas(CanvasExpressionId {
            canvas,
            index: *operand,
        });
        *operand += 1;
        let analysis = self
            .analyses
            .site_entries
            .remove(&(
                AnalysisSite::Canvas,
                canvas,
                super::expr::expr_key(expression),
            ))
            .ok_or_else(|| {
                self.invariant(span, "missing authoritative canvas expression analysis")
            })?;
        self.record_analysis_metrics(&analysis);
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expression,
            destination,
            if initializer {
                SourceMode::Initializer
            } else {
                SourceMode::Raw
            },
            "canvas",
            env,
            span,
            origin,
        )
    }

    fn lower_handler(
        &mut self,
        handler_index: usize,
        handler: &Handler,
        base_env: &FactEnv,
    ) -> Result<(), Error> {
        let mut env = ScopedFactEnv::new(base_env);
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
            #[cfg(test)]
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
        record_fact_metric!(self.facts.metrics.handler_authoritative_analyses += 1);
        self.record_analysis_metrics(&analysis);
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expr,
            expected,
            SourceMode::Contextual,
            "handler",
            env,
            span,
            origin,
        )
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
        env: &mut ScopedFactEnv<'_>,
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
            Statement::Match { value, arms, span } => {
                let value =
                    self.statement_operand(statement_id, &mut operand, value, None, env, span)?;
                if !matches!(self.facts.expression_use(value).source, Type::Named(_)) {
                    return Err(self.invariant(span, "handler match value is not a UI enum"));
                }
                self.record_checked_task(
                    declaration.task,
                    Some(Type::Unit),
                    None,
                    declaration.is_final,
                    span,
                )?;
                let expected = arms.iter().map(|arm| arm.statements.len()).sum::<usize>();
                if expected != declaration.children.len() {
                    return Err(self.invariant(span, "handler match child arena diverged"));
                }
                let mut children = declaration.children.iter().copied();
                for arm in arms {
                    let mut child_env = ScopedFactEnv::new(&*env);
                    for child in &arm.statements {
                        let child_id = children.next().ok_or_else(|| {
                            self.invariant(span, "handler match child ID is missing")
                        })?;
                        self.lower_handler_statement(child, child_id, &mut child_env)?;
                    }
                }
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
            Statement::InvalidateLane { .. } => {}
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
                    let mut child_env = ScopedFactEnv::new(env);
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
                let mut child_env = ScopedFactEnv::new(env);
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
            Statement::Slice {
                args, key, span, ..
            } => {
                for arg in args {
                    self.statement_operand(statement_id, &mut operand, arg, None, env, span)?;
                }
                self.statement_operand(
                    statement_id,
                    &mut operand,
                    key,
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
            Statement::Emit { args, span, .. } => {
                for arg in args {
                    self.statement_operand(statement_id, &mut operand, arg, None, env, span)?;
                }
                self.record_checked_task(
                    declaration.task,
                    Some(Type::Unit),
                    None,
                    declaration.is_final,
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
        let self_move = match statement {
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
                Some(if matches!(target_ty, Type::Editor | Type::List(_)) {
                    self.facts
                        .self_move_contract(value, target, self.declarations)
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
            self_move,
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
                WidgetOperation::ScrollToKey { target, key } => {
                    self.lower_widget_target(target, statement_id, operand, env, span)?;
                    self.statement_operand(statement_id, operand, key, None, env, span)?;
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
            | Statement::Match { .. }
            | Statement::Exit { .. }
            | Statement::InvalidateLane { .. }
            | Statement::Run { .. }
            | Statement::Sip { .. }
            | Statement::TaskFlow { .. }
            | Statement::TaskGroup { .. }
            | Statement::Abortable { .. }
            | Statement::ClipboardWrite { .. }
            | Statement::Emit { .. }
            | Statement::Slice { .. } => {}
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
        if let Some(window) = &target.window {
            self.statement_operand(statement, operand, window, Some(&Type::WindowId), env, span)?;
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
        record_fact_metric!(self.facts.metrics.scope_env_builds += 1);
        record_fact_metric!(self.facts.metrics.scope_env_entries += env.len());
        env
    }

    fn lower_subscriptions(&mut self) -> Result<(), Error> {
        if self.document.subscriptions.is_empty() {
            return Ok(());
        }
        let env = self.fact_env(ValueScope::App);
        for (index, subscription) in self.document.subscriptions.iter().enumerate() {
            let declaration = self.declarations.subscription(index);
            let analysis = self
                .analyses
                .remove_subscription(declaration.id)
                .ok_or_else(|| {
                    self.invariant(
                        &subscription.span,
                        "missing authoritative subscription analysis",
                    )
                })?;
            let condition = subscription
                .condition
                .as_ref()
                .map(|expression| {
                    self.push_subscription_expression(
                        declaration.id,
                        CheckedSubscriptionExprRole::Condition,
                        expression,
                        Some(&Type::Bool),
                        &env,
                        &subscription.span,
                    )
                })
                .transpose()?;
            let context = subscription
                .context
                .as_ref()
                .map(|expression| {
                    self.push_subscription_expression(
                        declaration.id,
                        CheckedSubscriptionExprRole::Context,
                        expression,
                        None,
                        &env,
                        &subscription.span,
                    )
                })
                .transpose()?;
            let source = match (&subscription.source, analysis.source) {
                (
                    SubscriptionSource::Every { .. },
                    CheckedSubscriptionSourceAnalysis::Every { milliseconds },
                ) => CheckedSubscriptionSource::Every { milliseconds },
                (
                    SubscriptionSource::Repeat { .. },
                    CheckedSubscriptionSourceAnalysis::Repeat {
                        function,
                        milliseconds,
                    },
                ) => CheckedSubscriptionSource::Repeat {
                    function,
                    milliseconds,
                },
                (
                    SubscriptionSource::Run { args, .. },
                    CheckedSubscriptionSourceAnalysis::Run { function },
                ) => CheckedSubscriptionSource::Run {
                    arguments: self.lower_subscription_arguments(
                        declaration,
                        args,
                        function.id,
                        &env,
                        &subscription.span,
                    )?,
                    function,
                },
                (
                    SubscriptionSource::Recipe { args, .. },
                    CheckedSubscriptionSourceAnalysis::Recipe { function },
                ) => CheckedSubscriptionSource::Recipe {
                    arguments: self.lower_subscription_arguments(
                        declaration,
                        args,
                        function.id,
                        &env,
                        &subscription.span,
                    )?,
                    function,
                },
                (
                    SubscriptionSource::Events { id, .. },
                    CheckedSubscriptionSourceAnalysis::Events { filter },
                ) => CheckedSubscriptionSource::Events {
                    identity: self.push_subscription_expression(
                        declaration.id,
                        CheckedSubscriptionExprRole::EventIdentity,
                        id,
                        None,
                        &env,
                        &subscription.span,
                    )?,
                    filter,
                },
                (
                    SubscriptionSource::Event { .. },
                    CheckedSubscriptionSourceAnalysis::Event { raw },
                ) => CheckedSubscriptionSource::Event { raw },
                (
                    SubscriptionSource::Extern { args, .. },
                    CheckedSubscriptionSourceAnalysis::Extern { function },
                ) => CheckedSubscriptionSource::Extern {
                    arguments: self.lower_subscription_arguments(
                        declaration,
                        args,
                        function.id,
                        &env,
                        &subscription.span,
                    )?,
                    function,
                },
                (
                    SubscriptionSource::InputMethod(_),
                    CheckedSubscriptionSourceAnalysis::InputMethod(event),
                ) => CheckedSubscriptionSource::InputMethod(event),
                (
                    SubscriptionSource::Keyboard(_),
                    CheckedSubscriptionSourceAnalysis::Keyboard(event),
                ) => CheckedSubscriptionSource::Keyboard(event),
                (SubscriptionSource::Mouse(_), CheckedSubscriptionSourceAnalysis::Mouse(event)) => {
                    CheckedSubscriptionSource::Mouse(event)
                }
                (
                    SubscriptionSource::SystemTheme,
                    CheckedSubscriptionSourceAnalysis::SystemTheme,
                ) => CheckedSubscriptionSource::SystemTheme,
                (SubscriptionSource::Touch(_), CheckedSubscriptionSourceAnalysis::Touch(event)) => {
                    CheckedSubscriptionSource::Touch(event)
                }
                (
                    SubscriptionSource::Window(_),
                    CheckedSubscriptionSourceAnalysis::Window(event),
                ) => CheckedSubscriptionSource::Window(event),
                _ => {
                    return Err(self.invariant(
                        &subscription.span,
                        "subscription source changed after semantic analysis",
                    ));
                }
            };
            self.facts.subscriptions.push(CheckedSubscription {
                id: declaration.id,
                source,
                source_payloads: analysis.source_payloads,
                delivered_payloads: analysis.delivered_payloads,
                filter: analysis.filter,
                context,
                condition,
                window_id: subscription.window_id,
                status: subscription.status,
                route: CheckedSubscriptionRoute {
                    handler: analysis.route_handler,
                    handler_name: analysis.route_handler_name,
                    payloads: analysis.route_payloads,
                },
                span: subscription.span.clone(),
                origin: declaration.origin,
            });
        }
        Ok(())
    }

    fn lower_subscription_arguments(
        &mut self,
        declaration: crate::hir::Declaration<SubscriptionId>,
        arguments: &[Expr],
        function: ExternFnId,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<Vec<CheckedExprUseId>, Error> {
        let parameter_types = self
            .declarations
            .extern_decl(function)
            .params
            .iter()
            .map(|(_, ty)| ty.clone())
            .collect::<Vec<_>>();
        arguments
            .iter()
            .zip(&parameter_types)
            .enumerate()
            .map(|(index, (argument, expected))| {
                self.push_subscription_expression(
                    declaration.id,
                    CheckedSubscriptionExprRole::SourceArgument(index as u32),
                    argument,
                    Some(expected),
                    env,
                    span,
                )
            })
            .collect()
    }

    fn push_subscription_expression(
        &mut self,
        subscription: SubscriptionId,
        role: CheckedSubscriptionExprRole,
        expr: &Expr,
        expected: Option<&Type>,
        env: &dyn FactEnvironment,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = CheckedExprOwner::Subscription { subscription, role };
        let analysis = self.analyses.remove(owner).ok_or_else(|| {
            self.invariant(
                span,
                "missing authoritative subscription expression analysis",
            )
        })?;
        record_fact_metric!(self.facts.metrics.subscription_analysis_passes += 1);
        self.record_analysis_metrics(&analysis);
        let parent = self
            .declarations
            .subscription(subscription.0 as usize)
            .origin;
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expr,
            expected,
            SourceMode::Contextual,
            "subscription",
            env,
            span,
            origin,
        )
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
        let analysis = self.analyses.remove(owner).ok_or_else(|| {
            self.invariant(span, "missing authoritative view expression analysis")
        })?;
        record_fact_metric!(self.facts.metrics.view_analysis_passes += 1);
        self.record_analysis_metrics(&analysis);
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expr,
            expected,
            SourceMode::Contextual,
            "view",
            env,
            span,
            origin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_length(
        &mut self,
        view: ViewId,
        value: &Option<LengthValue>,
        role: CheckedViewExprRole,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<CheckedLength, Error> {
        Ok(match value {
            None => CheckedLength::None,
            Some(LengthValue::Fill) => CheckedLength::Fill,
            Some(LengthValue::FillPortion(portion)) => CheckedLength::FillPortion(*portion),
            Some(LengthValue::Shrink) => CheckedLength::Shrink,
            Some(LengthValue::Fixed(expression)) => {
                let expression = self.push_view_expression(
                    CheckedExprOwner::View { view, role },
                    expression,
                    None,
                    env,
                    span,
                    parent,
                )?;
                let source = self.facts.expression_use(expression).source.clone();
                if !matches!(source, Type::F64 | Type::Length) {
                    return Err(
                        self.invariant(span, "dimension type diverged after semantic checking")
                    );
                }
                CheckedLength::Fixed { expression, source }
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_metric(
        &mut self,
        view: ViewId,
        value: &Option<Expr>,
        role: CheckedViewExprRole,
        env: &dyn FactEnvironment,
        span: &Span,
        parent: OriginId,
    ) -> Result<Option<CheckedExprUseId>, Error> {
        value
            .as_ref()
            .map(|expression| {
                self.push_view_expression(
                    CheckedExprOwner::View { view, role },
                    expression,
                    Some(&Type::F64),
                    env,
                    span,
                    parent,
                )
            })
            .transpose()
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
        let analysis = self.analyses.remove(owner).ok_or_else(|| {
            self.invariant(
                span,
                "missing authoritative app-setting expression analysis",
            )
        })?;
        record_fact_metric!(self.facts.metrics.app_setting_analysis_passes += 1);
        self.record_analysis_metrics(&analysis);
        let origin = self.origins.push(span, Some(parent));
        self.push_analyzed_expression(
            owner,
            analysis,
            expr,
            Some(expected),
            SourceMode::Contextual,
            "app-setting",
            env,
            span,
            origin,
        )
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

    /// A keyed lazy (`lazy value by ...`) never clones its value into the
    /// memo dependency tuple: the generated builder captures the value by
    /// reference and clones it only when a key changes. That reference must
    /// outlive the built view, so the value has to be a place rooted in app
    /// or component state — a state field path, or a `for` or keyed-column
    /// row over such a place, recursively. A Copy-typed row is captured by
    /// value and ends any chain. A chain rooted at a component prop is
    /// returned as `Some(param)` instead of accepted outright: the reference
    /// a prop-rooted capture borrows is whatever each call site baked into
    /// the prop, so its stability is validated per argument after every view
    /// has lowered (see [`Self::validate_keyed_lazy_prop_arguments`]).
    fn require_borrow_stable_lazy_value(
        &self,
        use_id: CheckedExprUseId,
        span: &Span,
    ) -> Result<Option<ComponentParamId>, Error> {
        let mut root = self.facts.expression_use(use_id).root;
        loop {
            let expression = self.facts.expression(root);
            let CheckedExprKind::Path {
                root: path_root,
                projections,
            } = &expression.kind
            else {
                return Err(keyed_lazy_value_error(span));
            };
            let struct_projections_only = projections
                .iter()
                .all(|projection| matches!(projection.kind, CheckedProjectionKind::Struct(_)));
            if !struct_projections_only {
                return Err(keyed_lazy_value_error(span));
            }
            match path_root {
                CheckedPathRoot::Value(
                    CheckedValueRef::AppState(_) | CheckedValueRef::ComponentState(_),
                ) => return Ok(None),
                CheckedPathRoot::Value(CheckedValueRef::ComponentParam(param)) => {
                    return Ok(Some(*param));
                }
                CheckedPathRoot::Local(local) => {
                    let local = self.facts.local(*local);
                    let CheckedLocalOwner::View { view, role } = local.owner else {
                        return Err(keyed_lazy_value_error(span));
                    };
                    // Only `for` and keyed-column rows bind their non-Copy
                    // items as references into the items place (and their
                    // Copy items owned via `.iter().cloned()`). Every other
                    // local — a match payload, a table row — reaches
                    // emission as a reference to a frame temporary or as an
                    // owned value a `move` builder would tear out of the
                    // enclosing body, so the walk stops at these two roles
                    // and the Copy exemption below applies to nothing else.
                    let items_role = match role {
                        CheckedViewLocalRole::ForItem => CheckedViewExprRole::ForItems,
                        CheckedViewLocalRole::KeyedItem => CheckedViewExprRole::KeyedItems,
                        _ => return Err(keyed_lazy_value_error(span)),
                    };
                    // A Copy row is bound owned (`.iter().cloned()`), so the
                    // capture is by value and the chain ends here.
                    if crate::codegen::copy_expression_type(&local.ty) {
                        return Ok(None);
                    }
                    // The enclosing loop is still mid-lowering, so its flow
                    // is not recorded yet; its items expression already is.
                    let items = self
                        .facts
                        .expression_use_by_owner(CheckedExprOwner::View {
                            view,
                            role: items_role,
                        })
                        .ok_or_else(|| {
                            self.invariant(
                                span,
                                "checked row local without a checked items expression",
                            )
                        })?;
                    root = self.facts.expression_use(items).root;
                }
                _ => return Err(keyed_lazy_value_error(span)),
            }
        }
    }

    /// A keyed lazy value rooted at a component prop is borrow-stable only
    /// when EVERY call feeds the prop an app state field path — or another
    /// prop fed the same way, recursively, since generated components bake
    /// the caller's argument expression into the prop. Row-bound locals are
    /// rejected here even though the in-scope walk accepts them: an outlined
    /// component use passes a local-value argument as an owned by-value
    /// method parameter, and a `move` builder capturing that owned value
    /// would tear it out from under the rest of the body. Component-state
    /// reads are rejected too: their baked code embeds the call-site scope
    /// local, which a `move` builder captures by move. Runs after every view
    /// has lowered, because a call inside a later component body records its
    /// argument source after the prop's own body was checked.
    fn validate_keyed_lazy_prop_arguments(&mut self) -> Result<(), Error> {
        let mut worklist = std::mem::take(&mut self.pending_keyed_lazy_props);
        let mut visited: HashSet<ComponentParamId> = HashSet::new();
        while let Some(param) = worklist.pop() {
            if !visited.insert(param) {
                continue;
            }
            let mut sources: Vec<CheckedExprUseId> = self
                .facts
                .component_argument_sources
                .iter()
                .filter(|((_, source_param), _)| *source_param == param)
                .map(|(_, source)| match source {
                    CheckedComponentArgumentSource::Supplied(expression)
                    | CheckedComponentArgumentSource::Default(expression) => *expression,
                })
                .collect();
            // Deterministic reporting: the textually earliest argument is
            // lowered first, so the smallest use id is the one named.
            sources.sort_by_key(|use_id| use_id.0);
            for use_id in sources {
                if let Some(next) = self.require_state_backed_prop_argument(param, use_id)? {
                    worklist.push(next);
                }
            }
        }
        Ok(())
    }

    fn require_state_backed_prop_argument(
        &self,
        param: ComponentParamId,
        use_id: CheckedExprUseId,
    ) -> Result<Option<ComponentParamId>, Error> {
        let expression_use = self.facts.expression_use(use_id);
        let origin = self.origins.get(expression_use.origin);
        let span = Span {
            line: origin.line,
            column: origin.column,
        };
        let error = || {
            let component = &self.document.components[param.component.0 as usize];
            let prop = &component.params[param.index as usize].name;
            keyed_lazy_prop_argument_error(&span, &component.name, prop)
        };
        let expression = self.facts.expression(expression_use.root);
        let CheckedExprKind::Path { root, projections } = &expression.kind else {
            return Err(error());
        };
        let struct_projections_only = projections
            .iter()
            .all(|projection| matches!(projection.kind, CheckedProjectionKind::Struct(_)));
        if !struct_projections_only {
            return Err(error());
        }
        match root {
            CheckedPathRoot::Value(CheckedValueRef::AppState(_)) => Ok(None),
            CheckedPathRoot::Value(CheckedValueRef::ComponentParam(next)) => Ok(Some(*next)),
            _ => Err(error()),
        }
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
        let identity = match node.identity() {
            Some(identity) => Some(CheckedViewIdentity {
                name: identity.name.clone(),
                key: identity
                    .key
                    .as_ref()
                    .map(|key| {
                        self.push_view_expression(
                            CheckedExprOwner::View {
                                view,
                                role: CheckedViewExprRole::IdentityKey,
                            },
                            key,
                            None,
                            env,
                            node.span(),
                            origin,
                        )
                    })
                    .transpose()?,
            }),
            None => None,
        };
        self.facts.views[view.0 as usize].identity = identity;
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
                record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
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
                        record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
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
                        children: arm
                            .children
                            .iter()
                            .map(|child| {
                                self.declarations.view_id(child.span()).ok_or_else(|| {
                                    self.invariant(
                                        &arm.span,
                                        "match arm child has no shared view ID",
                                    )
                                })
                            })
                            .collect::<Result<_, _>>()?,
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
                options,
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
                record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
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
                let width = self.lower_length(
                    view,
                    &options.width,
                    CheckedViewExprRole::KeyedWidth,
                    env,
                    span,
                    origin,
                )?;
                let height = self.lower_length(
                    view,
                    &options.height,
                    CheckedViewExprRole::KeyedHeight,
                    env,
                    span,
                    origin,
                )?;
                let spacing = self.lower_metric(
                    view,
                    &options.spacing,
                    CheckedViewExprRole::KeyedSpacing,
                    env,
                    span,
                    origin,
                )?;
                let padding = CheckedPadding {
                    all: self.lower_metric(
                        view,
                        &options.padding.all,
                        CheckedViewExprRole::KeyedPaddingAll,
                        env,
                        span,
                        origin,
                    )?,
                    x: self.lower_metric(
                        view,
                        &options.padding.x,
                        CheckedViewExprRole::KeyedPaddingX,
                        env,
                        span,
                        origin,
                    )?,
                    y: self.lower_metric(
                        view,
                        &options.padding.y,
                        CheckedViewExprRole::KeyedPaddingY,
                        env,
                        span,
                        origin,
                    )?,
                    top: self.lower_metric(
                        view,
                        &options.padding.top,
                        CheckedViewExprRole::KeyedPaddingTop,
                        env,
                        span,
                        origin,
                    )?,
                    right: self.lower_metric(
                        view,
                        &options.padding.right,
                        CheckedViewExprRole::KeyedPaddingRight,
                        env,
                        span,
                        origin,
                    )?,
                    bottom: self.lower_metric(
                        view,
                        &options.padding.bottom,
                        CheckedViewExprRole::KeyedPaddingBottom,
                        env,
                        span,
                        origin,
                    )?,
                    left: self.lower_metric(
                        view,
                        &options.padding.left,
                        CheckedViewExprRole::KeyedPaddingLeft,
                        env,
                        span,
                        origin,
                    )?,
                };
                let max_width = self.lower_metric(
                    view,
                    &options.max_width,
                    CheckedViewExprRole::KeyedMaxWidth,
                    env,
                    span,
                    origin,
                )?;
                let virtual_row = self.lower_metric(
                    view,
                    &options.virtual_row,
                    CheckedViewExprRole::KeyedVirtualRow,
                    env,
                    span,
                    origin,
                )?;
                self.lower_view_expression_tree(child, &scoped)?;
                CheckedViewFlow::Keyed {
                    items: items_use,
                    key: key_use,
                    item: local,
                    layout: CheckedKeyedLayout {
                        semantic_key: crate::ast::keyed_column_semantic_key(options),
                        width,
                        height,
                        spacing,
                        padding,
                        max_width,
                        virtual_row,
                        align: options.align,
                    },
                }
            }
            ViewNode::Lazy {
                dependency,
                keys,
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
                let mut key_uses = Vec::with_capacity(keys.len());
                let mut key_bindings = Vec::with_capacity(keys.len());
                for (index, key) in keys.iter().enumerate() {
                    let use_id = self.push_view_expression(
                        CheckedExprOwner::View {
                            view,
                            role: CheckedViewExprRole::LazyKey(index as u32),
                        },
                        key,
                        None,
                        env,
                        span,
                        origin,
                    )?;
                    key_uses.push(use_id);
                    let Expr::Path(segments) = key else {
                        key_bindings.push(CheckedLazyKeyBinding {
                            local: None,
                            name: None,
                        });
                        continue;
                    };
                    let [name] = segments.as_slice() else {
                        key_bindings.push(CheckedLazyKeyBinding {
                            local: None,
                            name: None,
                        });
                        continue;
                    };
                    if name == binding {
                        key_bindings.push(CheckedLazyKeyBinding {
                            local: None,
                            name: None,
                        });
                        continue;
                    }
                    let ty = self.facts.expression_use(use_id).source.clone();
                    key_bindings.push(CheckedLazyKeyBinding {
                        local: Some(self.push_view_local(
                            name,
                            ty,
                            view,
                            CheckedViewLocalRole::LazyKey(index as u32),
                            span,
                        )),
                        name: Some(name.clone()),
                    });
                }
                if !key_uses.is_empty()
                    && let Some(param) =
                        self.require_borrow_stable_lazy_value(dependency_use, span)?
                {
                    self.pending_keyed_lazy_props.push(param);
                }
                let ty = self.facts.expression_use(dependency_use).source.clone();
                let local = self.push_view_local(
                    binding,
                    ty.clone(),
                    view,
                    CheckedViewLocalRole::LazyDependency,
                    span,
                );
                let empty = FactEnv::default();
                let mut scoped = ScopedFactEnv::new(&empty);
                scoped.insert(binding.clone(), CheckedPathRoot::Local(local), ty);
                for binding in &key_bindings {
                    let (Some(local), Some(name)) = (binding.local, binding.name.as_ref()) else {
                        continue;
                    };
                    let ty = self.facts.local(local).ty.clone();
                    scoped.insert(name.clone(), CheckedPathRoot::Local(local), ty);
                }
                record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
                self.lower_view_expression_tree(child, &scoped)?;
                CheckedViewFlow::Lazy {
                    dependency: dependency_use,
                    keys: key_uses,
                    key_bindings,
                    binding: local,
                }
            }
            ViewNode::Table {
                item,
                rows,
                options,
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
                record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
                let width = self.lower_length(
                    view,
                    &options.width,
                    CheckedViewExprRole::TableWidth,
                    env,
                    span,
                    origin,
                )?;
                let padding = self.lower_metric(
                    view,
                    &options.padding,
                    CheckedViewExprRole::TablePadding,
                    env,
                    span,
                    origin,
                )?;
                let padding_x = self.lower_metric(
                    view,
                    &options.padding_x,
                    CheckedViewExprRole::TablePaddingX,
                    env,
                    span,
                    origin,
                )?;
                let padding_y = self.lower_metric(
                    view,
                    &options.padding_y,
                    CheckedViewExprRole::TablePaddingY,
                    env,
                    span,
                    origin,
                )?;
                let separator = self.lower_metric(
                    view,
                    &options.separator,
                    CheckedViewExprRole::TableSeparator,
                    env,
                    span,
                    origin,
                )?;
                let separator_x = self.lower_metric(
                    view,
                    &options.separator_x,
                    CheckedViewExprRole::TableSeparatorX,
                    env,
                    span,
                    origin,
                )?;
                let separator_y = self.lower_metric(
                    view,
                    &options.separator_y,
                    CheckedViewExprRole::TableSeparatorY,
                    env,
                    span,
                    origin,
                )?;
                let mut checked_columns = Vec::with_capacity(columns.len());
                for (index, column) in columns.iter().enumerate() {
                    let column_origin = self.origins.push(&column.span, Some(origin));
                    let width = self.lower_length(
                        view,
                        &column.width,
                        CheckedViewExprRole::TableColumnWidth(index as u32),
                        env,
                        &column.span,
                        column_origin,
                    )?;
                    self.lower_view_expression_tree(&column.header, env)?;
                    self.lower_view_expression_tree(&column.cell, &scoped)?;
                    checked_columns.push(CheckedTableColumn {
                        width,
                        align_x: column.align_x,
                        align_y: column.align_y,
                        origin: column_origin,
                    });
                }
                CheckedViewFlow::Table {
                    rows: rows_use,
                    item: local,
                    layout: CheckedTableLayout {
                        semantic_key: crate::ast::table_semantic_key(options, columns),
                        width,
                        padding,
                        padding_x,
                        padding_y,
                        separator,
                        separator_x,
                        separator_y,
                        columns: checked_columns,
                    },
                }
            }
            ViewNode::PaneGrid {
                name,
                configuration,
                options,
                panes,
                templates,
                span,
                ..
            } => {
                self.lower_pane_grid_facts(
                    view,
                    name,
                    configuration,
                    options,
                    panes,
                    templates,
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::Responsive {
                content,
                width,
                height,
                span,
                ..
            } => {
                let semantic_key = crate::ast::responsive_semantic_key(content, width, height);
                let expression_count = crate::ast::responsive_expression_count(width, height);
                let dimensions = [
                    self.lower_length(
                        view,
                        width,
                        CheckedViewExprRole::ResponsiveWidthDimension,
                        env,
                        span,
                        origin,
                    )?,
                    self.lower_length(
                        view,
                        height,
                        CheckedViewExprRole::ResponsiveHeightDimension,
                        env,
                        span,
                        origin,
                    )?,
                ];
                match content {
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
                        record_fact_metric!(self.facts.metrics.scope_env_overlays += 2);
                        self.lower_view_expression_tree(content, &scoped)?;
                        CheckedViewFlow::ResponsiveSize {
                            semantic_key,
                            expression_count,
                            width: width_local,
                            height: height_local,
                            dimensions,
                        }
                    }
                }
            }
            ViewNode::Text {
                value,
                options,
                span,
                ..
            } => {
                self.lower_text_facts(
                    view,
                    CheckedInteractionKind::Text,
                    crate::ast::text_semantic_key(options),
                    crate::ast::text_expression_roots(value, options),
                    Vec::new(),
                    options,
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::RichText {
                options,
                color,
                children,
                route,
                span,
                ..
            } => {
                self.lower_rich_text_facts(view, options, color, children, route, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Input {
                label,
                binding,
                hint,
                disabled,
                options,
                span,
                ..
            } => {
                self.lower_input_facts(view, label, binding, hint, disabled, options, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Button {
                label,
                content,
                disabled,
                options,
                route,
                span,
                ..
            } => {
                self.lower_button_facts(view, label, content, disabled, options, route, env, span)?;
                if let Some(content) = content {
                    self.lower_view_expression_tree(content, env)?;
                }
                CheckedViewFlow::None
            }
            ViewNode::TextEditor {
                binding,
                disabled,
                options,
                span,
                ..
            } => {
                self.lower_text_editor_facts(view, binding, disabled, options, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Markdown {
                content,
                options,
                route,
                span,
                ..
            } => {
                self.lower_markdown_facts(view, content, options, route, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Checkbox {
                label,
                id,
                checked,
                disabled,
                options,
                style,
                route,
                span,
                ..
            } => {
                let statuses = [
                    &style.active_checked,
                    &style.active_unchecked,
                    &style.hovered_checked,
                    &style.hovered_unchecked,
                    &style.disabled_checked,
                    &style.disabled_unchecked,
                ]
                .into_iter()
                .flatten()
                .map(|status| {
                    (
                        status.span.as_ref().unwrap_or(span),
                        crate::ast::checkbox_status_expression_roots(status),
                    )
                })
                .collect();
                self.lower_boolean_control_facts(
                    view,
                    BooleanControlFactSource {
                        kind: CheckedInteractionKind::Checkbox,
                        semantic_key: crate::ast::checkbox_semantic_key(
                            id, label, checked, disabled, options, style, route,
                        ),
                        expressions: crate::ast::checkbox_expression_roots(
                            label, checked, disabled, options, style,
                        ),
                        statuses,
                        style: style
                            .custom
                            .as_ref()
                            .map(|call| (call.function.as_str(), ExternKind::CheckboxStyle)),
                        has_font: options.font.is_some(),
                        route,
                    },
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::Toggler {
                label,
                id,
                checked,
                disabled,
                options,
                style,
                route,
                span,
                ..
            } => {
                let statuses = [
                    &style.active_checked,
                    &style.active_unchecked,
                    &style.hovered_checked,
                    &style.hovered_unchecked,
                    &style.disabled_checked,
                    &style.disabled_unchecked,
                ]
                .into_iter()
                .flatten()
                .map(|status| {
                    (
                        status.span.as_ref().unwrap_or(span),
                        crate::ast::toggler_status_expression_roots(status),
                    )
                })
                .collect();
                self.lower_boolean_control_facts(
                    view,
                    BooleanControlFactSource {
                        kind: CheckedInteractionKind::Toggler,
                        semantic_key: crate::ast::toggler_semantic_key(
                            id, label, checked, disabled, options, style, route,
                        ),
                        expressions: crate::ast::toggler_expression_roots(
                            label, checked, disabled, options, style,
                        ),
                        statuses,
                        style: style
                            .custom
                            .as_ref()
                            .map(|call| (call.function.as_str(), ExternKind::TogglerStyle)),
                        has_font: options.font.is_some(),
                        route,
                    },
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::Radio {
                label,
                id,
                value,
                selected,
                options,
                style,
                route,
                span,
                ..
            } => {
                let statuses = [
                    &style.active_selected,
                    &style.active_unselected,
                    &style.hovered_selected,
                    &style.hovered_unselected,
                ]
                .into_iter()
                .flatten()
                .map(|status| {
                    (
                        status.span.as_ref().unwrap_or(span),
                        crate::ast::radio_status_expression_roots(status),
                    )
                })
                .collect();
                self.lower_boolean_control_facts(
                    view,
                    BooleanControlFactSource {
                        kind: CheckedInteractionKind::Radio,
                        semantic_key: crate::ast::radio_semantic_key(
                            id, label, value, selected, options, style, route,
                        ),
                        expressions: crate::ast::radio_expression_roots(
                            label, value, selected, options, style,
                        ),
                        statuses,
                        style: style
                            .custom
                            .as_ref()
                            .map(|call| (call.function.as_str(), ExternKind::RadioStyle)),
                        has_font: options.font.is_some(),
                        route,
                    },
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::PickList {
                options,
                selected,
                options_config,
                route,
                span,
                ..
            } => {
                self.lower_pick_list_facts(
                    view,
                    options,
                    selected,
                    options_config,
                    route,
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::ComboBox {
                state,
                selected,
                placeholder,
                options,
                route,
                span,
                ..
            } => {
                self.lower_combo_box_facts(
                    view,
                    state,
                    selected,
                    placeholder,
                    options,
                    route,
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::Slider {
                value,
                min,
                max,
                step,
                options,
                vertical,
                styles,
                route,
                release,
                span,
                ..
            } => {
                self.lower_slider_facts(
                    view, value, min, max, step, options, *vertical, styles, route, release, env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::Progress {
                value,
                min,
                max,
                options,
                vertical,
                styles,
                span,
                ..
            } => {
                self.lower_progress_facts(
                    view, value, min, max, options, *vertical, styles, env, span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::Rule {
                axis,
                thickness,
                options,
                styles,
                span,
                ..
            } => {
                self.lower_rule_facts(view, *axis, thickness, options, styles, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::QrCode {
                payload,
                correction,
                version,
                cell_size,
                total_size,
                cell,
                background,
                span,
                ..
            } => {
                self.lower_qr_code_facts(
                    view,
                    payload,
                    *correction,
                    *version,
                    cell_size,
                    total_size,
                    cell,
                    background,
                    env,
                    span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::Space {
                width,
                height,
                styles,
                span,
                ..
            } => {
                self.lower_space_facts(view, width, height, styles, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Layout {
                kind,
                options,
                children,
                span,
                ..
            } => {
                self.lower_layout_facts(view, *kind, options, env, span)?;
                for child in children {
                    self.lower_view_expression_tree(child, env)?;
                }
                CheckedViewFlow::None
            }
            ViewNode::Container {
                options,
                content,
                span,
                ..
            } => {
                self.lower_container_facts(view, options, env, span)?;
                self.lower_view_expression_tree(content, env)?;
                CheckedViewFlow::None
            }
            ViewNode::Canvas {
                options,
                locals,
                commands,
                events,
                span,
                ..
            } => {
                self.lower_canvas_facts(view, options, locals, commands, events, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Media {
                kind,
                source,
                options,
                span,
                ..
            } => {
                self.lower_media_facts(view, *kind, source, options, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Tooltip {
                options,
                content,
                tip,
                span,
                ..
            } => {
                self.lower_tooltip_facts(view, options, env, span)?;
                self.lower_view_expression_tree(content, env)?;
                self.lower_view_expression_tree(tip, env)?;
                CheckedViewFlow::None
            }
            ViewNode::Float {
                scale,
                x,
                y,
                style,
                content,
                span,
                ..
            } => {
                let flow = self.lower_float_facts(view, scale, x, y, style, env, span)?;
                self.lower_view_expression_tree(content, env)?;
                flow
            }
            ViewNode::Pin {
                width,
                height,
                x,
                y,
                content,
                span,
                ..
            } => {
                let flow = self.lower_pin_facts(view, width, height, x, y, env, span)?;
                self.lower_view_expression_tree(content, env)?;
                flow
            }
            ViewNode::MouseArea {
                options,
                content,
                span,
                ..
            } => {
                self.lower_interaction_facts(
                    view,
                    CheckedInteractionKind::MouseArea,
                    crate::ast::mouse_area_semantic_key(options),
                    options
                        .interaction_expr
                        .as_ref()
                        .map(|expression| (expression, Some(Type::MouseInteraction)))
                        .into_iter()
                        .collect(),
                    crate::ast::mouse_area_routes(options),
                    env,
                    span,
                )?;
                self.lower_view_expression_tree(content, env)?;
                CheckedViewFlow::None
            }
            ViewNode::ResizeHandle {
                options,
                content,
                span,
                ..
            } => {
                self.lower_interaction_facts(
                    view,
                    CheckedInteractionKind::ResizeHandle,
                    crate::ast::resize_handle_semantic_key(options),
                    Vec::new(),
                    crate::ast::resize_handle_routes(options),
                    env,
                    span,
                )?;
                self.lower_view_expression_tree(content, env)?;
                CheckedViewFlow::None
            }
            ViewNode::Sensor {
                options,
                content,
                span,
                ..
            } => {
                let mut option_expressions = Vec::new();
                if let Some(expression) = &options.key {
                    option_expressions.push((expression, None));
                }
                if let Some(expression) = &options.anticipate {
                    option_expressions.push((expression, Some(Type::F64)));
                }
                if let Some(expression) = &options.delay_ms {
                    option_expressions.push((expression, Some(Type::I64)));
                }
                self.lower_interaction_facts(
                    view,
                    CheckedInteractionKind::Sensor,
                    crate::ast::sensor_semantic_key(options),
                    option_expressions,
                    crate::ast::sensor_routes(options),
                    env,
                    span,
                )?;
                self.lower_view_expression_tree(content, env)?;
                CheckedViewFlow::None
            }
            ViewNode::Overlay {
                options,
                content,
                layer,
                span,
                ..
            } => {
                self.lower_interaction_facts(
                    view,
                    CheckedInteractionKind::Overlay,
                    crate::ast::overlay_semantic_key(options),
                    vec![
                        (&options.visible, Some(Type::Bool)),
                        (&options.padding, Some(Type::F64)),
                    ],
                    crate::ast::overlay_routes(options),
                    env,
                    span,
                )?;
                self.lower_view_expression_tree(content, env)?;
                self.lower_view_expression_tree(layer, env)?;
                CheckedViewFlow::None
            }
            ViewNode::Component {
                name,
                args,
                slots,
                events,
                route,
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
                self.lower_component_call_route_facts(
                    view, call, component, name, events, route, env, span,
                )?;
                CheckedViewFlow::None
            }
            ViewNode::ExternComponent {
                function,
                args,
                route,
                span,
                ..
            } => {
                self.lower_extern_component_facts(view, function, args, route, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Themer {
                function,
                args,
                route,
                span,
                ..
            } => {
                self.lower_themer_facts(view, function, args, route, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Shader {
                function,
                args,
                width,
                height,
                route,
                span,
                ..
            } => {
                self.lower_shader_facts(view, function, args, width, height, route, env, span)?;
                CheckedViewFlow::None
            }
            ViewNode::Theme {
                preset,
                text,
                background,
                content,
                span,
                ..
            } => {
                self.lower_nested_theme_facts(view, preset, text, background, env, span)?;
                self.lower_view_expression_tree(content, env)?;
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
        record_fact_metric!(self.facts.metrics.declaration_lookups += 1);
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
        record_fact_metric!(self.facts.metrics.initializer_analysis_passes += 1);
        self.record_analysis_metrics(&analysis);
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
                            owner: lowering.owner,
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
            owner: lowering.owner,
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
            record_fact_metric!(self.facts.metrics.declaration_lookups += 1);
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
        record_fact_metric!(self.facts.metrics.declaration_lookups += 1);
        let (root, mut input) = env
            .get(name)
            .cloned()
            .ok_or_else(|| self.invariant(span, format!("missing checked value `{name}`")))?;
        let mut projections = Vec::with_capacity(path.len().saturating_sub(1));
        for field in &path[1..] {
            let (output, kind) = if let Type::Named(struct_name) = &input {
                record_fact_metric!(self.facts.metrics.declaration_lookups += 1);
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
        record_fact_metric!(self.facts.metrics.declaration_lookups += 1);
        if let Some(declaration) = self.declarations.extern_decl_by_name(name)
            && matches!(declaration.kind, ExternKind::Pure | ExternKind::Sync)
        {
            return Ok(CheckedCallTarget::Extern(ExternRef {
                id: declaration.declaration.id,
                name: declaration.name.clone(),
            }));
        }
        let name = unqualified_name(name);
        debug_assert_ne!(name, "provided");
        record_fact_metric!(self.facts.metrics.builtin_intern_lookups += 1);
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
        record_fact_metric!(self.facts.metrics.declaration_lookups += 1);
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
                    record_fact_metric!(self.facts.metrics.scope_env_overlays += 1);
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
            CheckedCallTarget::Extern(reference) => self
                .declarations
                .extern_decl(reference.id)
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
                self.index_view(
                    mount,
                    None,
                    CheckedViewScope::Test(self.declarations.test(index).declaration.id),
                )?;
            }
        }
        Ok(())
    }

    fn index_component_slots(&mut self) -> Result<(), Error> {
        #[cfg(test)]
        let started = Instant::now();
        self.facts.component_slots = Vec::with_capacity(self.document.components.len());
        for (component_index, component) in self.document.components.iter().enumerate() {
            let component_id = self.declarations.component(component_index).id;
            let source_slots = crate::check::component_slots(&component.root);
            let declared_count = self
                .declarations
                .component_slot_count(component_id)
                .ok_or_else(|| {
                    self.invariant(
                        &component.span,
                        "component slot declaration arena is missing",
                    )
                })?;
            if source_slots.len() != declared_count {
                return Err(self.invariant(
                    &component.span,
                    "component slot source and declaration cardinality diverged",
                ));
            }

            let mut checked = Vec::with_capacity(source_slots.len());
            for (index, (name, optional, span)) in source_slots.into_iter().enumerate() {
                record_fact_metric!(self.facts.metrics.component_slot_index_visits += 1);
                let id = ComponentSlotId {
                    component: component_id,
                    index: index as u32,
                };
                let declaration = self
                    .declarations
                    .try_component_slot(id)
                    .ok_or_else(|| self.invariant(span, "component slot declaration is missing"))?;
                let view = self
                    .declarations
                    .view_id(span)
                    .ok_or_else(|| self.invariant(span, "component slot has no shared view ID"))?;
                let checked_view = self
                    .facts
                    .try_view(view)
                    .ok_or_else(|| self.invariant(span, "component slot has no checked view"))?;
                if checked_view.scope != CheckedViewScope::Component(component_id)
                    || checked_view.kind != "slot"
                    || checked_view.origin != self.declarations.view(view).origin
                {
                    return Err(self.invariant(span, "component slot checked view is inconsistent"));
                }
                if self
                    .facts
                    .component_slots_by_view
                    .insert(view, id)
                    .is_some()
                {
                    return Err(self.invariant(span, "component slot view is associated twice"));
                }
                checked.push(CheckedComponentSlot {
                    id,
                    view,
                    name: name.to_owned(),
                    optional,
                    origin: declaration.origin,
                });
            }
            self.facts.component_slots.push(checked);
        }
        #[cfg(test)]
        {
            self.facts.component_slot_index_elapsed = started.elapsed();
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
            identity: None,
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

/// The handler namespace a view route resolves against: component views own
/// their handlers (including inside `lazy` bodies), everything else routes app
/// handlers.
fn route_handler_owner(scope: CheckedViewScope) -> crate::hir::HandlerOwner {
    match scope {
        CheckedViewScope::Component(component) => crate::hir::HandlerOwner::Component(component),
        CheckedViewScope::App | CheckedViewScope::Test(_) => crate::hir::HandlerOwner::App,
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

enum SourceMode {
    Raw,
    Contextual,
    Initializer,
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
    pub(crate) fn structural_snapshot(&self) -> String {
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
        for slots in &self.component_slots {
            for slot in slots {
                writeln!(
                    output,
                    "component-slot {:?} view={:?} name={:?} optional={} origin=o{}",
                    slot.id, slot.view, slot.name, slot.optional, slot.origin.0
                )
                .unwrap();
            }
        }
        for (index, subscription) in self.subscriptions.iter().enumerate() {
            writeln!(
                output,
                "subscription s{index} id={:?} source={:?} source_payloads={:?} delivered={:?} filter={:?} context={:?} condition={:?} route={:?} origin=o{}",
                subscription.id,
                subscription.source,
                subscription.source_payloads,
                subscription.delivered_payloads,
                subscription.filter,
                subscription.context,
                subscription.condition,
                subscription.route,
                subscription.origin.0,
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
    use std::time::Instant;

    const THEME: &str = "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";

    #[test]
    fn snapshots_resolved_expression_and_owner_facts() {
        let source = format!(
            r#"app Facts
extern crate::backend
  User(name:str)
  pure load_user(seed:i64) -> User
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
            r#"value v0 AppState(AppStateId(0)) user:Named("User") init=Some(ExpressionId(0)) origin=o0
value v1 AppState(AppStateId(1)) color:Color init=Some(ExpressionId(1)) origin=o1
value v2 AppState(AppStateId(2)) mode:Named("Mode") init=Some(ExpressionId(2)) origin=o2
value v3 Derived(DerivedId(0)) name:Str init=Some(ExpressionId(3)) origin=o3
value v4 Derived(DerivedId(1)) visible:Bool init=Some(ExpressionId(4)) origin=o4
value v5 ComponentParam(ComponentParamId { component: ComponentId(0), index: 0 }) label:Str init=Some(ExpressionId(5)) origin=o6
value v6 ComponentState(ComponentStateId { component: ComponentId(0), index: 0 }) open:Bool init=Some(ExpressionId(6)) origin=o7
use u0 Value(AppState(AppStateId(0))) root=e1 source=Named("User") destination=Named("User") coercion=None origin=o0
use u1 Value(AppState(AppStateId(1))) root=e5 source=Color destination=Color coercion=None origin=o1
use u2 Value(AppState(AppStateId(2))) root=e6 source=Named("Mode") destination=Named("Mode") coercion=None origin=o2
use u3 Value(Derived(DerivedId(0))) root=e7 source=Str destination=Str coercion=None origin=o3
use u4 Value(Derived(DerivedId(1))) root=e14 source=Bool destination=Bool coercion=None origin=o4
use u5 Value(ComponentParam(ComponentParamId { component: ComponentId(0), index: 0 })) root=e15 source=Str destination=Str coercion=None origin=o6
use u6 Value(ComponentState(ComponentStateId { component: ComponentId(0), index: 0 })) root=e16 source=Bool destination=Bool coercion=None origin=o7
use u7 Interaction(InteractionExpressionId { widget: ViewId(6), index: 0 }) root=e17 source=Str destination=Str coercion=None origin=o23
use u8 Interaction(InteractionExpressionId { widget: ViewId(1), index: 0 }) root=e18 source=Str destination=Str coercion=None origin=o24
use u9 View { view: ViewId(2), role: IfCondition } root=e19 source=Bool destination=Bool coercion=None origin=o25
use u10 Interaction(InteractionExpressionId { widget: ViewId(3), index: 0 }) root=e20 source=Str destination=Str coercion=None origin=o26
expr e0 i64 1 : I64 origin=o0
expr e1 call Extern(ExternRef { id: ExternFnId(0), name: "load_user" }) [CheckedExprId(0)] : Named("User") origin=o0
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
expr e17 path Value(Derived(DerivedId(0))) [] : Str origin=o23
expr e18 path Value(ComponentParam(ComponentParamId { component: ComponentId(0), index: 0 })) [] : Str origin=o24
expr e19 path Value(ComponentState(ComponentStateId { component: ComponentId(0), index: 0 })) [] : Bool origin=o25
expr e20 str "Open" : Str origin=o26
view w0 layout Component(ComponentId(0)) parent=None children=[ViewId(1), ViewId(2)] flow=None origin=o15
view w1 text Component(ComponentId(0)) parent=Some(ViewId(0)) children=[] flow=None origin=o16
view w2 if Component(ComponentId(0)) parent=Some(ViewId(0)) children=[ViewId(3)] flow=If { condition: ExpressionId(9) } origin=o17
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
                component_slots: 0,
                component_slot_index_visits: 0,
                expression_uses: 11,
                expressions: 21,
                type_analysis_queries: 21,
                type_analysis_nodes: 21,
                type_analysis_cache_hits: 0,
                initializer_analysis_passes: 7,
                app_setting_analysis_passes: 0,
                view_analysis_passes: 1,
                handler_authoritative_analyses: 0,
                handler_auxiliary_analyses: 0,
                subscription_analysis_passes: 0,
                type_scope_env_overlays: 0,
                type_scope_env_full_clones: 0,
                declaration_lookups: 20,
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
    fn subscriptions_retain_resolved_sources_payloads_routes_and_expressions() {
        let source = format!(
            r#"app Subscriptions
extern crate::backend
  stream numbers(seed:i64) -> i64
  recipe ticks(seed:i64) -> i64
  event-filter raw_event() -> str
  pure positive(value:i64) -> i64?
{THEME}state
  enabled = true
  seed = 2
  tag = 7
on number(tag, value)
on tick(value)
on event(value)
subscribe
  run numbers(seed) with=tag filter=positive when enabled -> number _ _
  recipe ticks(seed) -> tick _
  events seed using=raw_event -> event _
view
  text "ready"
"#
        );
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let resolved = program.subscriptions();
        assert_eq!(resolved.len(), 3);
        assert!(matches!(
            &resolved[0].source,
            lower::ResolvedSubscriptionSource::Run {
                function,
                arguments,
            } if function.id == ExternFnId(0)
                && function.params == [lower::ResolvedType::Value(Type::I64)]
                && arguments.len() == 1
        ));
        assert_eq!(
            resolved[0].source_payloads,
            [lower::ResolvedType::Value(Type::I64)]
        );
        assert_eq!(
            resolved[0].delivered_payloads,
            [
                lower::ResolvedType::Value(Type::I64),
                lower::ResolvedType::Value(Type::I64),
            ]
        );
        assert_eq!(resolved[0].route.handler, HandlerId(0));
        assert_eq!(resolved[0].route.handler_name, "number");
        assert!(matches!(
            resolved[0].route.args.as_slice(),
            [
                lower::ResolvedRouteArg::Payload {
                    index: 0,
                    ty: Type::I64,
                },
                lower::ResolvedRouteArg::Payload {
                    index: 1,
                    ty: Type::I64,
                },
            ]
        ));
        assert!(matches!(
            &resolved[1].source,
            lower::ResolvedSubscriptionSource::Recipe { function, .. }
                if function.id == ExternFnId(1)
        ));
        assert!(matches!(
            &resolved[2].source,
            lower::ResolvedSubscriptionSource::Events { filter, .. }
                if filter.id == ExternFnId(2)
        ));
        let facts = program.checked_facts();
        let subscriptions = facts.subscriptions();
        assert_eq!(subscriptions.len(), 3);
        assert!(matches!(
            &subscriptions[0].source,
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            } if function.id == ExternFnId(0) && arguments.len() == 1
        ));
        assert_eq!(subscriptions[0].source_payloads, vec![Type::I64]);
        assert_eq!(
            subscriptions[0].delivered_payloads,
            vec![Type::I64, Type::I64]
        );
        assert_eq!(subscriptions[0].filter.as_ref().unwrap().id, ExternFnId(3));
        assert!(subscriptions[0].context.is_some());
        assert!(subscriptions[0].condition.is_some());
        assert_eq!(subscriptions[0].route.handler, HandlerId(0));
        assert_eq!(subscriptions[0].route.payloads, vec![0, 1]);
        assert!(matches!(
            &subscriptions[1].source,
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            } if function.id == ExternFnId(1) && arguments.len() == 1
        ));
        assert!(matches!(
            &subscriptions[2].source,
            CheckedSubscriptionSource::Events {
                filter,
                ..
            } if filter.id == ExternFnId(2)
        ));
        assert_eq!(facts.metrics().subscription_analysis_passes, 5);

        let generated = crate::codegen::generate(&program, "subscriptions.ice").unwrap();
        assert!(generated.contains("::iced::Subscription::run_with(self.seed"));
        assert!(generated.contains("crate::backend::positive(__value)"));
        assert!(generated.contains(".with(self.tag)"));
        assert!(generated.contains("__SubscriptionsMessage::Number(__value.0, __value.1)"));
        assert!(generated.contains("__IceEventFilterRawEvent"));
    }

    #[test]
    fn tests_retain_stable_target_step_and_expression_owners() {
        let source = format!(
            r#"app TestFacts
{THEME}view
  col #root
    text "Key" #key
    text "Item" #item("Key")
test dynamic_targets
  target key = #root/key
  target item = #root/item(key.value)
  click #root/item(key.value)
  expect text "Item" within #root/item(key.value)
"#
        );
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let declarations = program.declarations();
        let facts = program.checked_facts();
        let test = TestId(0);
        assert_eq!(declarations.test(0).declaration.id, test);
        assert_eq!(declarations.test(0).name, "dynamic_targets");

        let key = TestTargetId { test, index: 0 };
        let item = TestTargetId { test, index: 1 };
        let key_local = facts
            .local_by_owner(CheckedLocalOwner::TestTarget(key))
            .unwrap();
        let item_local = facts
            .local_by_owner(CheckedLocalOwner::TestTarget(item))
            .unwrap();
        assert_eq!(facts.local(key_local).name, "key");
        assert_eq!(facts.local(item_local).name, "item");
        assert_eq!(facts.local(item_local).ty, Type::TestTarget);

        for owner in [
            CheckedExprOwner::TestTargetKey {
                target: item,
                segment: 1,
            },
            CheckedExprOwner::TestStepOperand {
                step: TestStepId { test, index: 0 },
                operand: 0,
            },
            CheckedExprOwner::TestStepOperand {
                step: TestStepId { test, index: 1 },
                operand: 0,
            },
            CheckedExprOwner::TestStepOperand {
                step: TestStepId { test, index: 1 },
                operand: 1,
            },
        ] {
            let expression = facts
                .expression_use_by_owner(owner)
                .unwrap_or_else(|| panic!("missing checked test expression for {owner:?}"));
            assert_eq!(facts.expression_use(expression).owner, owner);
        }

        let target_key = facts.expression_use_by_owner(CheckedExprOwner::TestTargetKey {
            target: item,
            segment: 1,
        });
        let click_key = facts.expression_use_by_owner(CheckedExprOwner::TestStepOperand {
            step: TestStepId { test, index: 0 },
            operand: 0,
        });
        assert_ne!(target_key, click_key);
    }

    #[test]
    fn missing_and_mutated_test_hir_contracts_are_e196_invariants() {
        let source = format!(
            "app TestContracts\n{THEME}view\n  text \"Item\" #item\ntest contract\n  target item = #item\n  click item\n  expect true\n"
        );

        let mut missing_expression = analyze(&source).unwrap();
        let owner = CheckedExprOwner::TestStepOperand {
            step: TestStepId {
                test: TestId(0),
                index: 1,
            },
            operand: 0,
        };
        missing_expression
            .facts
            .expression_uses_by_owner
            .remove(&owner);
        let error = lower::lower(missing_expression).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("test expression has no checked expression-use ID")
        );

        let mut changed_topology = analyze(&source).unwrap();
        changed_topology.document.tests[0].steps.clear();
        let error = lower::lower(changed_topology).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("test declaration topology changed"));
    }

    #[test]
    fn subscription_lowering_and_codegen_ignore_raw_subscription_mutations() {
        let source = format!(
            "app FrozenSubscription\nextern crate::backend\n  stream numbers(seed:i64) -> i64\n{THEME}state\n  seed = 2\non number(value)\non key(value)\non theme(value)\nsubscribe\n  run numbers(seed) -> number _\n  keyboard press -> key _\n  system theme -> theme _\nview\n  text \"ready\"\n"
        );
        let baseline = lower::lower(analyze(&source).unwrap()).unwrap();
        let baseline = crate::codegen::generate(&baseline, "frozen-subscription.ice").unwrap();
        assert!(baseline.contains("pub(crate) struct __IceKeyPress"));
        assert!(baseline.contains("fn __ice_system_theme"));

        let mut checked = analyze(&source).unwrap();
        checked.document.subscriptions.clear();
        let program = lower::lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "frozen-subscription.ice").unwrap();
        assert_eq!(generated, baseline);
    }

    #[test]
    fn subscription_run_type_uses_normalized_struct_path_after_raw_mutation() {
        let source = format!(
            "app FrozenRunType\nextern crate::backend\n  User(name:str)\n  pure load_user(seed:i64) -> User\n  stream users(seed:User) -> str\n{THEME}state\n  user:User = load_user(1)\non user_event(value)\nsubscribe\n  run users(user) -> user_event _\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.document.structs[0].rust_path = "crate::mutated::WrongUser".into();
        let program = lower::lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "frozen-run-type.ice").unwrap();
        let run = generated
            .lines()
            .find(|line| line.contains("::iced::Subscription::run_with"))
            .unwrap();
        assert!(run.contains("|__data: &crate::backend::User|"));
        assert!(!run.contains("crate::mutated::WrongUser"));
    }

    #[test]
    fn invalid_subscription_source_and_filter_extern_ids_are_e196_invariants() {
        let source = format!(
            "app InvalidSubscriptionExtern\nextern crate::backend\n  stream numbers(seed:i64) -> i64\n  pure positive(value:i64) -> i64?\n{THEME}state\n  seed = 2\non number(value)\nsubscribe\n  run numbers(seed) filter=positive -> number _\nview\n  text \"ready\"\n"
        );

        let mut invalid_source = analyze(&source).unwrap();
        let span = invalid_source.facts.subscriptions[0].span.clone();
        let CheckedSubscriptionSource::Run { function, .. } =
            &mut invalid_source.facts.subscriptions[0].source
        else {
            unreachable!();
        };
        function.id = ExternFnId(u32::MAX);
        let error = lower::lower(invalid_source).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!((error.line, error.column), (span.line, span.column));
        assert!(error.message.contains("invalid extern declaration ID"));

        let mut invalid_filter = analyze(&source).unwrap();
        let span = invalid_filter.facts.subscriptions[0].span.clone();
        invalid_filter.facts.subscriptions[0]
            .filter
            .as_mut()
            .unwrap()
            .id = ExternFnId(u32::MAX);
        let error = lower::lower(invalid_filter).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!((error.line, error.column), (span.line, span.column));
        assert!(error.message.contains("invalid extern declaration ID"));
    }

    #[test]
    fn same_arena_subscription_extern_ids_must_match_the_checked_contract() {
        let source = format!(
            "app WrongSubscriptionExtern\nextern crate::backend\n  stream numbers(seed:i64) -> i64\n  pure positive(value:i64) -> i64?\n  stream wrong_arity() -> i64\n  stream wrong_param(seed:str) -> i64\n  stream wrong_output(seed:i64) -> str\n  pure wrong_filter_param(value:str) -> i64?\n  pure wrong_filter_output(value:i64) -> i64\n{THEME}state\n  seed = 2\non number(value)\nsubscribe\n  run numbers(seed) filter=positive -> number _\nview\n  text \"ready\"\n"
        );

        fn extern_ref(checked: &crate::CheckedDocument, name: &str) -> ExternRef {
            let declaration = checked.declarations.extern_decl_by_name(name).unwrap();
            ExternRef {
                id: declaration.declaration.id,
                name: declaration.name.clone(),
            }
        }

        fn replace_source(checked: &mut crate::CheckedDocument, name: &str) {
            let replacement = extern_ref(checked, name);
            let CheckedSubscriptionSource::Run { function, .. } =
                &mut checked.facts.subscriptions[0].source
            else {
                unreachable!();
            };
            *function = replacement;
        }

        fn replace_filter(checked: &mut crate::CheckedDocument, name: &str) {
            let replacement = extern_ref(checked, name);
            checked.facts.subscriptions[0].filter = Some(replacement);
        }

        fn assert_contract_error(checked: crate::CheckedDocument, needle: &str) {
            let span = checked.facts.subscriptions[0].span.clone();
            let error = lower::lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert_eq!((error.line, error.column), (span.line, span.column));
            assert!(error.message.contains(needle), "{}", error.message);
        }

        let mut wrong_kind = analyze(&source).unwrap();
        replace_source(&mut wrong_kind, "positive");
        assert_contract_error(wrong_kind, "declaration contract");

        let mut wrong_arity = analyze(&source).unwrap();
        replace_source(&mut wrong_arity, "wrong_arity");
        assert_contract_error(wrong_arity, "mismatched arity");

        let mut wrong_param = analyze(&source).unwrap();
        replace_source(&mut wrong_param, "wrong_param");
        assert_contract_error(wrong_param, "type contract");

        let mut wrong_output = analyze(&source).unwrap();
        replace_source(&mut wrong_output, "wrong_output");
        assert_contract_error(wrong_output, "payload contract");

        let mut wrong_filter_kind = analyze(&source).unwrap();
        replace_filter(&mut wrong_filter_kind, "numbers");
        assert_contract_error(wrong_filter_kind, "declaration contract");

        let mut wrong_filter_param = analyze(&source).unwrap();
        replace_filter(&mut wrong_filter_param, "wrong_filter_param");
        assert_contract_error(wrong_filter_param, "filter has mismatched parameter");

        let mut wrong_filter_output = analyze(&source).unwrap();
        replace_filter(&mut wrong_filter_output, "wrong_filter_output");
        assert_contract_error(wrong_filter_output, "non-optional output");
    }

    #[test]
    fn invalid_subscription_handler_id_is_a_source_mapped_e196_invariant() {
        let source = format!(
            "app InvalidSubscriptionHandler\n{THEME}on tick(now)\nsubscribe\n  every 10ms -> tick _\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let span = checked.facts.subscriptions[0].span.clone();
        checked.facts.subscriptions[0].route.handler = HandlerId(u32::MAX);
        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!((error.line, error.column), (span.line, span.column));
        assert!(error.message.contains("invalid handler declaration ID"));
    }

    #[test]
    fn same_arena_subscription_handler_id_must_match_the_payload_contract() {
        let source = format!(
            "app WrongSubscriptionHandler\n{THEME}on tick(now)\non theme(label)\nsubscribe\n  every 10ms -> tick _\n  system theme -> theme _\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let replacement = checked
            .declarations
            .handler_id(crate::hir::HandlerOwner::App, "theme")
            .unwrap();
        let span = checked.facts.subscriptions[0].span.clone();
        checked.facts.subscriptions[0].route.handler = replacement;
        checked.facts.subscriptions[0].route.handler_name = "theme".into();
        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!((error.line, error.column), (span.line, span.column));
        assert!(error.message.contains("payload type diverged"));
    }

    #[test]
    fn subscription_route_rejects_a_same_name_component_handler_id() {
        let source = format!(
            "app SubscriptionHandlerOwner\n{THEME}on tick\ncomponent Surface()\n  on tick\n  text \"surface\"\nsubscribe\n  every 10ms -> tick\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let component_tick = checked
            .declarations
            .handler_id(crate::hir::HandlerOwner::Component(ComponentId(0)), "tick")
            .unwrap();
        checked.facts.subscriptions[0].route.handler = component_tick;

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("handler contract"));
    }

    #[test]
    fn invalid_subscription_expression_ids_are_source_mapped_e196_invariants() {
        let source = format!(
            "app InvalidSubscriptionExpression\n{THEME}state\n  enabled = true\non tick(now)\nsubscribe\n  every 10ms when enabled -> tick _\nview\n  text \"ready\"\n"
        );

        let mut invalid_use = analyze(&source).unwrap();
        let span = invalid_use.facts.subscriptions[0].span.clone();
        invalid_use.facts.subscriptions[0].condition = Some(CheckedExprUseId(u32::MAX));
        let error = lower::lower(invalid_use).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!((error.line, error.column), (span.line, span.column));
        assert!(error.message.contains("invalid expression-use ID"));

        let mut invalid_root = analyze(&source).unwrap();
        let span = invalid_root.facts.subscriptions[0].span.clone();
        let condition = invalid_root.facts.subscriptions[0].condition.unwrap();
        invalid_root.facts.expression_uses[condition.0 as usize].root = CheckedExprId(u32::MAX);
        let error = lower::lower(invalid_root).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!((error.line, error.column), (span.line, span.column));
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn subscription_expression_roles_types_and_cycles_are_lowering_invariants() {
        let source = format!(
            "app SubscriptionExpressionContracts\n{THEME}state\n  enabled = true\n  tag = 7\non tick(tag, now)\nsubscribe\n  every 10ms with=tag when !enabled -> tick _ _\nview\n  text \"ready\"\n"
        );
        let checked = analyze(&source).unwrap();
        let condition = checked.facts.subscriptions[0].condition.unwrap();
        let context = checked.facts.subscriptions[0].context.unwrap();

        fn assert_e196(checked: crate::CheckedDocument, needle: &str) {
            let error = lower::lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains(needle), "{}", error.message);
        }

        let mut wrong_role = checked.clone();
        wrong_role.facts.subscriptions[0].condition = Some(context);
        assert_e196(wrong_role, "owner or role");

        let mut wrong_source = checked.clone();
        wrong_source.facts.expression_uses[condition.0 as usize].source = Type::Str;
        assert_e196(wrong_source, "type contract");

        let mut wrong_destination = checked.clone();
        wrong_destination.facts.expression_uses[condition.0 as usize].destination = Type::Str;
        assert_e196(wrong_destination, "type contract");

        let mut wrong_coercion = checked.clone();
        wrong_coercion.facts.expression_uses[condition.0 as usize].coercion =
            CheckedInitializerCoercion::StrToMarkdown;
        assert_e196(wrong_coercion, "type contract");

        let mut wrong_owner = checked.clone();
        let root = wrong_owner.facts.expression_uses[condition.0 as usize].root;
        wrong_owner.facts.expressions[root.0 as usize].owner = context;
        assert_e196(wrong_owner, "belongs to another retained expression use");

        let mut cycle = checked;
        let root = cycle.facts.expression_uses[condition.0 as usize].root;
        let CheckedExprKind::Unary { value, .. } =
            &mut cycle.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        *value = root;
        assert_e196(cycle, "contains a cycle");
    }

    #[test]
    fn subscription_expression_nodes_revalidate_retained_operator_and_call_contracts() {
        let source = format!(
            r#"app SubscriptionNodeContracts
extern crate::backend
  pure normalize(value:i64) -> i64
  pure normalize_alias(value:i64) -> i64
  wrong_kind(value:i64) -> i64
  stream numbers(value:i64) -> i64
  stream lists(value:[i64]) -> i64
  stream options(value:i64?) -> i64
{THEME}state
  enabled = true
  seed = 7
derived
  copied_seed = seed
on tick(value)
on timer(now)
subscribe
  run numbers(normalize(seed)) -> tick _
  run numbers(len([seed])) -> tick _
  run lists([seed]) -> tick _
  run options(some(seed)) -> tick _
  every 10ms when !enabled && enabled -> timer _
view
  text "ready"
"#
        );
        let checked = analyze(&source).unwrap();

        fn argument_root(checked: &crate::CheckedDocument, subscription: usize) -> CheckedExprId {
            let CheckedSubscriptionSource::Run { arguments, .. } =
                &checked.facts.subscriptions[subscription].source
            else {
                unreachable!();
            };
            checked.facts.expression_uses[arguments[0].0 as usize].root
        }

        fn assert_e196(checked: crate::CheckedDocument, needle: &str) {
            let error = lower::lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains(needle), "{}", error.message);
        }

        let mut wrong_extern_kind = checked.clone();
        let root = argument_root(&wrong_extern_kind, 0);
        let wrong_id = wrong_extern_kind
            .declarations
            .extern_decl_by_name("wrong_kind")
            .unwrap()
            .declaration
            .id;
        let CheckedExprKind::Call { target, .. } =
            &mut wrong_extern_kind.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        *target = CheckedCallTarget::Extern(ExternRef {
            id: wrong_id,
            name: "wrong_kind".into(),
        });
        assert_e196(wrong_extern_kind, "extern call");

        let mut wrong_extern_identity = checked.clone();
        let root = argument_root(&wrong_extern_identity, 0);
        let alias_id = wrong_extern_identity
            .declarations
            .extern_decl_by_name("normalize_alias")
            .unwrap()
            .declaration
            .id;
        let CheckedExprKind::Call {
            target: CheckedCallTarget::Extern(reference),
            ..
        } = &mut wrong_extern_identity.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        reference.id = alias_id;
        assert_e196(wrong_extern_identity, "extern call");

        let mut wrong_value_scope = checked.clone();
        let root = argument_root(&wrong_value_scope, 0);
        wrong_value_scope.facts.expressions[root.0 as usize].kind = CheckedExprKind::Path {
            root: CheckedPathRoot::Value(CheckedValueRef::Derived(DerivedId(0))),
            projections: Vec::new(),
        };
        assert_e196(wrong_value_scope, "cannot reference a derived value");

        let mut wrong_literal = checked.clone();
        let root = argument_root(&wrong_literal, 2);
        let CheckedExprKind::List(items) = &wrong_literal.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        let item = items[0];
        wrong_literal.facts.expressions[item.0 as usize].kind = CheckedExprKind::Bool(true);
        assert_e196(wrong_literal, "kind does not match its retained type");

        let mut wrong_list_item = checked.clone();
        let root = argument_root(&wrong_list_item, 2);
        let CheckedExprKind::List(items) = &wrong_list_item.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        let item = items[0];
        wrong_list_item.facts.expressions[item.0 as usize].ty = Type::Str;
        assert_e196(wrong_list_item, "kind does not match its retained type");

        let mut wrong_builtin = checked.clone();
        let len_root = argument_root(&wrong_builtin, 1);
        let some_root = argument_root(&wrong_builtin, 3);
        let CheckedExprKind::Call {
            target: len_target, ..
        } = wrong_builtin.facts.expressions[len_root.0 as usize]
            .kind
            .clone()
        else {
            unreachable!();
        };
        let CheckedExprKind::Call { target, .. } =
            &mut wrong_builtin.facts.expressions[some_root.0 as usize].kind
        else {
            unreachable!();
        };
        *target = len_target;
        assert_e196(wrong_builtin, "collection query input");

        let mut wrong_binary = checked.clone();
        let condition = wrong_binary.facts.subscriptions[4].condition.unwrap();
        let root = wrong_binary.facts.expression_uses[condition.0 as usize].root;
        let CheckedExprKind::Binary { operator, left, .. } =
            &mut wrong_binary.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        *operator = CheckedBinaryOperator::Boolean(BinaryOp::Add);
        let left = *left;
        assert_e196(wrong_binary, "binary expression");

        let mut wrong_unary = checked;
        let CheckedExprKind::Unary { operator, .. } =
            &mut wrong_unary.facts.expressions[left.0 as usize].kind
        else {
            unreachable!();
        };
        *operator = CheckedUnaryOperator::NumericNegation(Type::I64);
        assert_e196(wrong_unary, "unary expression");
    }

    #[test]
    fn native_subscription_payload_contract_is_recomputed_during_lowering() {
        let source = format!(
            "app NativePayloadContract\n{THEME}on tick(now)\nsubscribe\n  every 10ms -> tick _\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.subscriptions[0].source = CheckedSubscriptionSource::Mouse(MouseEvent::Moved);
        checked.document.subscriptions[0].source = SubscriptionSource::Mouse(MouseEvent::Moved);

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("payload contract"));
    }

    #[test]
    fn subscription_options_revalidate_their_source_compatibility() {
        let frame = format!(
            "app InvalidWindowId\n{THEME}on frame\nsubscribe\n  window frame -> frame\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&frame).unwrap();
        checked.facts.subscriptions[0].window_id = true;
        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("does not support a window ID"));

        let timer = format!(
            "app InvalidStatus\n{THEME}on tick(now)\nsubscribe\n  every 10ms -> tick _\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&timer).unwrap();
        checked.facts.subscriptions[0].status = Some(EventStatus::Captured);
        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("does not support status filtering"));

        let mut every = analyze(&timer).unwrap();
        every.facts.subscriptions[0].source = CheckedSubscriptionSource::Every { milliseconds: 0 };
        let error = lower::lower(every).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("duration is not positive"));

        let repeat = format!(
            "app InvalidRepeatDuration\nextern crate::backend\n  refresh() -> i64\n{THEME}on tick(now)\nsubscribe\n  repeat refresh() every 10ms -> tick _\nview\n  text \"ready\"\n"
        );
        let mut repeat = analyze(&repeat).unwrap();
        let CheckedSubscriptionSource::Repeat { milliseconds, .. } =
            &mut repeat.facts.subscriptions[0].source
        else {
            unreachable!();
        };
        *milliseconds = 0;
        let error = lower::lower(repeat).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("duration is not positive"));
    }

    #[test]
    fn subscription_hashability_is_revalidated_during_lowering() {
        fn replace_expression_type(
            checked: &mut crate::CheckedDocument,
            expression: CheckedExprUseId,
            ty: Type,
        ) {
            let root = checked.facts.expression_uses[expression.0 as usize].root;
            checked.facts.expression_uses[expression.0 as usize].source = ty.clone();
            checked.facts.expression_uses[expression.0 as usize].destination = ty.clone();
            checked.facts.expressions[root.0 as usize].ty = ty;
            checked.facts.expressions[root.0 as usize].kind = CheckedExprKind::F64(1.0);
        }

        let run = format!(
            "app InvalidRunHash\nextern crate::backend\n  stream numbers(seed:i64) -> i64\n{THEME}state\n  seed = 1\non tick(value)\nsubscribe\n  run numbers(seed) -> tick _\nview\n  text \"ready\"\n"
        );
        let mut run = analyze(&run).unwrap();
        let CheckedSubscriptionSource::Run {
            function,
            arguments,
        } = &run.facts.subscriptions[0].source
        else {
            unreachable!();
        };
        let function = function.id;
        let argument = arguments[0];
        run.declarations
            .replace_extern_param_type_for_test(function, 0, Type::F64);
        replace_expression_type(&mut run, argument, Type::F64);
        let error = lower::lower(run).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("run data is not hashable"));

        let events = format!(
            "app InvalidEventIdentity\nextern crate::backend\n  event-filter raw_event() -> str\n{THEME}state\n  seed = 1\non event(value)\nsubscribe\n  events seed using=raw_event -> event _\nview\n  text \"ready\"\n"
        );
        let mut events = analyze(&events).unwrap();
        let CheckedSubscriptionSource::Events { identity, .. } =
            events.facts.subscriptions[0].source
        else {
            unreachable!();
        };
        replace_expression_type(&mut events, identity, Type::F64);
        let error = lower::lower(events).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("event identity is not hashable"));

        let context = format!(
            "app InvalidContextHash\n{THEME}state\n  tag = 1\non tick(tag, value)\nsubscribe\n  every 10ms with=tag -> tick _ _\nview\n  text \"ready\"\n"
        );
        let mut context = analyze(&context).unwrap();
        let context_expression = context.facts.subscriptions[0].context.unwrap();
        replace_expression_type(&mut context, context_expression, Type::F64);
        let error = lower::lower(context).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("context is not hashable"));
    }

    #[test]
    fn checked_program_kind_is_frozen_before_lowering() {
        let source = format!("app FrozenProgramKind\n{THEME}view\n  text \"ready\"\n");
        let mut checked = analyze(&source).unwrap();
        checked.document.daemon = true;

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!((error.line, error.column), (1, 1));
        assert!(error.message.contains("application kind changed"));
    }

    #[test]
    fn invalid_subscription_expression_descendant_ids_are_e196_invariants() {
        let source = format!(
            r#"app InvalidSubscriptionDescendants
enum Mode
  idle
  loaded(str)
extern crate::backend
  User(name:str)
  pure load_user() -> User
  stream modes(value:Mode) -> i64
  stream labels(value:str) -> i64
  stream palettes(value:palette[AppTheme]) -> i64
{THEME}state
  user:User = load_user()
on tick(value)
subscribe
  run modes(Mode.loaded("ready")) -> tick _
  run modes(Mode.idle) -> tick _
  run labels(user.name) -> tick _
  run palettes(AppTheme.app) -> tick _
view
  text "ready"
"#
        );
        let checked = analyze(&source).unwrap();

        fn argument_root(checked: &crate::CheckedDocument, subscription: usize) -> CheckedExprId {
            let CheckedSubscriptionSource::Run { arguments, .. } =
                &checked.facts.subscriptions[subscription].source
            else {
                unreachable!();
            };
            checked.facts.expression_uses[arguments[0].0 as usize].root
        }

        fn assert_e196(checked: crate::CheckedDocument, subscription: usize, needle: &str) {
            let span = checked.facts.subscriptions[subscription].span.clone();
            let error = lower::lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert_eq!((error.line, error.column), (span.line, span.column));
            assert!(error.message.contains(needle), "{}", error.message);
        }

        let mut enum_call = checked.clone();
        let root = argument_root(&enum_call, 0);
        let CheckedExprKind::Call { target, .. } =
            &mut enum_call.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        *target = CheckedCallTarget::EnumVariant(crate::hir::EnumVariantId {
            owner: crate::hir::EnumId(u32::MAX),
            index: 0,
        });
        assert_e196(enum_call, 0, "expression declaration ID");

        let mut enum_path = checked.clone();
        let root = argument_root(&enum_path, 1);
        let CheckedExprKind::Path { root, .. } =
            &mut enum_path.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        *root = CheckedPathRoot::EnumVariant(crate::hir::EnumVariantId {
            owner: crate::hir::EnumId(u32::MAX),
            index: 0,
        });
        assert_e196(enum_path, 1, "expression declaration ID");

        let mut struct_projection = checked.clone();
        let root = argument_root(&struct_projection, 2);
        let CheckedExprKind::Path { projections, .. } =
            &mut struct_projection.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        projections[0].kind = CheckedProjectionKind::Struct(crate::hir::StructFieldId {
            owner: crate::hir::StructId(u32::MAX),
            index: 0,
        });
        assert_e196(struct_projection, 2, "expression declaration ID");

        let mut palette = checked.clone();
        let root = argument_root(&palette, 3);
        let CheckedExprKind::Path { root, .. } =
            &mut palette.facts.expressions[root.0 as usize].kind
        else {
            unreachable!();
        };
        *root = CheckedPathRoot::Palette(PaletteId(u32::MAX));
        assert_e196(palette, 3, "expression declaration ID");

        let mut slot = checked;
        let root = argument_root(&slot, 1);
        slot.facts.expressions[root.0 as usize].kind =
            CheckedExprKind::SlotProvided(ComponentSlotId {
                component: ComponentId(u32::MAX),
                index: 0,
            });
        slot.facts.expressions[root.0 as usize].ty = Type::Bool;
        assert_e196(slot, 1, "expression declaration ID");
    }

    #[test]
    fn malformed_subscription_payload_hir_is_an_e196_invariant() {
        let source = format!(
            "app MalformedSubscription\n{THEME}on tick(now)\nsubscribe\n  every 10ms -> tick _\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.subscriptions[0].delivered_payloads.clear();
        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("payload"));

        let source = format!(
            "app ReorderedSubscription\n{THEME}on moved(x, y)\nsubscribe\n  mouse moved -> moved _ _\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.subscriptions[0].route.payloads.swap(0, 1);
        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("payload order"));
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

        let extra_source = format!("app Extra\n{THEME}view\n  space\n");
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
            format!("app HandlerOwners\n{THEME}on update\n  let value = 1\nview\n  space\n");
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
    fn missing_and_leftover_subscription_analyses_are_e196_invariants() {
        let missing_source = format!(
            "app MissingSubscription\n{THEME}on tick(now)\nsubscribe\n  every 10ms when true -> tick _\nview\n  space\n"
        );
        let missing_document = crate::parse(&missing_source).unwrap();
        let mut missing_origins = OriginArena::default();
        let missing_declarations = DeclarationIndex::build(&missing_document, &mut missing_origins);
        let subscription = missing_declarations.subscription(0).id;
        let mut missing_analyses = CheckedAnalyses::default();
        missing_analyses
            .insert_subscription(
                subscription,
                CheckedSubscriptionAnalysis {
                    source: CheckedSubscriptionSourceAnalysis::Every { milliseconds: 10 },
                    source_payloads: vec![Type::Instant],
                    delivered_payloads: vec![Type::Instant],
                    filter: None,
                    route_handler: missing_declarations
                        .handler_id(crate::hir::HandlerOwner::App, "tick")
                        .unwrap(),
                    route_handler_name: "tick".into(),
                    route_payloads: vec![0],
                },
            )
            .unwrap();
        let missing = build(
            &missing_document,
            &missing_declarations,
            &mut missing_origins,
            missing_analyses,
        )
        .unwrap_err();
        assert_eq!(missing.code, "E196");
        assert!(
            missing
                .message
                .contains("missing authoritative subscription expression analysis")
        );

        let extra_source = format!("app ExtraSubscription\n{THEME}on tick(now)\nview\n  space\n");
        let extra_document = crate::parse(&extra_source).unwrap();
        let mut extra_origins = OriginArena::default();
        let extra_declarations = DeclarationIndex::build(&extra_document, &mut extra_origins);
        let mut extra_analyses = CheckedAnalyses::default();
        extra_analyses
            .insert_subscription(
                SubscriptionId(0),
                CheckedSubscriptionAnalysis {
                    source: CheckedSubscriptionSourceAnalysis::Every { milliseconds: 10 },
                    source_payloads: vec![Type::Instant],
                    delivered_payloads: vec![Type::Instant],
                    filter: None,
                    route_handler: extra_declarations
                        .handler_id(crate::hir::HandlerOwner::App, "tick")
                        .unwrap(),
                    route_handler_name: "tick".into(),
                    route_payloads: vec![0],
                },
            )
            .unwrap();
        let leftover = build(
            &extra_document,
            &extra_declarations,
            &mut extra_origins,
            extra_analyses,
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
    fn raw_view_topology_mutation_is_rejected_during_lowering() {
        let source =
            format!("app MutatedView\n{THEME}state\n  count = 1\nview\n  col\n    text count\n");
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, span, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let expected_line = span.line;
        children.clear();

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(
            error
                .message
                .contains("children diverged from checked topology")
        );
    }

    #[test]
    fn checked_view_parent_cycle_is_rejected_before_view_lowering() {
        let source = format!("app CyclicView\n{THEME}view\n  col\n    text \"child\"\n");
        let mut checked = analyze(&source).unwrap();
        let child = checked.facts.views[0].children[0];
        let expected_line = checked
            .origins
            .get(checked.declarations.view(child).origin)
            .line;
        checked.facts.views[child.0 as usize].parent = Some(child);

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("child identity, parent, or scope"));
    }

    #[test]
    fn checked_view_origin_swap_is_rejected_at_its_declaration() {
        let source = format!(
            "app SwappedViewOrigin\n{THEME}view\n  col\n    text \"first\"\n    text \"second\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let first = checked.facts.views[0].children[0];
        let second = checked.facts.views[0].children[1];
        let expected = checked.declarations.view(first).origin;
        let poisoned = checked.declarations.view(second).origin;
        assert_ne!(expected, poisoned);
        let expected_line = checked.origins.get(expected).line;
        checked.facts.views[first.0 as usize].origin = poisoned;

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("identity or origin diverged"));
    }

    #[test]
    fn malformed_match_hir_is_rejected_at_the_arm_origin_during_lowering() {
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

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("payload presence diverged"));
    }

    #[test]
    fn invalid_match_enum_id_is_rejected_at_the_arm_origin_during_lowering() {
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

        let error = lower::lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("enum ID is outside its arena"));
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
        let crate::lower::ResolvedViewKind::Component { call } = program
            .resolved_view(program.app_view())
            .map(|view| &view.kind)
            .unwrap()
        else {
            panic!("application root is not a component call")
        };
        let argument = program.component_call_by_id(*call).unwrap().arguments[0].expression;
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
                | CheckedLocalOwner::TestTarget(_)
                | CheckedLocalOwner::CanvasState(_)
                | CheckedLocalOwner::CanvasWidth(_)
                | CheckedLocalOwner::CanvasHeight(_)
                | CheckedLocalOwner::CanvasCommandItem(_)
                | CheckedLocalOwner::CanvasEventBinding { .. }
                | CheckedLocalOwner::AppSettingDaemonWindow => None,
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
        let pane = program.pane_grids().into_iter().next().unwrap();
        let checked_pane = program.checked_facts().pane_grid(pane.id).unwrap();
        let key_origin = program.origin(
            program
                .checked_facts()
                .expression_use(checked_pane.templates[0].key)
                .origin,
        );
        assert_eq!(key_origin.line, pane_template_line);
        assert_eq!(key_origin.parent, Some(checked_pane.templates[0].origin));
        let generated = crate::codegen::generate(&program, "arena.ice").unwrap();
        assert!(generated.contains("for (__ice_index, row) in self.items.iter()"));
        assert!(generated.contains("::std::option::Option::Some(label)"));
        assert!(generated.contains("for keyed_row in self.items.iter()"));
        assert!(generated.contains("move |(__row, table_row)"));
        assert!(generated.contains("__pane_maximized"));
        assert!(generated.contains("(__size.width as f64)"));
    }

    #[test]
    fn canvas_facts_retain_stable_expression_local_command_event_and_route_owners() {
        let source = r#"app Drawing
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on released(button)
view
  canvas w=fill h=120.0 cursor=(cursor_state) cursor-outside=outside
    state
      cursor_state = "grab"
      outside = false
      hits = 0
    event mouse pressed as button
      set cursor_state = "grabbing"
      set hits = hits + 1
      redraw
      capture
    event mouse released as button
      set cursor_state = "grab"
      emit released button
    text hits x=8.0 y=20.0 color=fg size=14.0
    for value in [12.0, 24.0]
      circle x=value y=40.0 r=4.0 fill=fg
"#;
        let program = lower::lower(analyze(source).unwrap()).unwrap();
        let facts = program.checked_facts();
        let canvas = facts.canvases.values().next().expect("checked canvas");
        assert_eq!(facts.canvases.len(), 1);
        assert_eq!(canvas.states.len(), 3);
        assert_eq!(canvas.routes.len(), 1);
        assert_eq!(
            facts
                .expression_uses()
                .iter()
                .filter(|expression| matches!(
                    expression.owner,
                    CheckedExprOwner::Canvas(CanvasExpressionId { canvas: owner, .. })
                        if owner == canvas.id
                ))
                .count(),
            canvas.expression_count as usize
        );
        for owner in [
            CheckedLocalOwner::CanvasState(CanvasLocalId {
                canvas: canvas.id,
                index: 0,
            }),
            CheckedLocalOwner::CanvasWidth(canvas.id),
            CheckedLocalOwner::CanvasHeight(canvas.id),
            CheckedLocalOwner::CanvasCommandItem(CanvasCommandId {
                canvas: canvas.id,
                index: 1,
            }),
            CheckedLocalOwner::CanvasEventBinding {
                event: CanvasEventId {
                    canvas: canvas.id,
                    index: 0,
                },
                index: 0,
            },
        ] {
            assert!(facts.local_by_owner(owner).is_some(), "missing {owner:?}");
        }
        assert!(matches!(
            canvas.routes[0].args.as_slice(),
            [CheckedCanvasRouteArg::Expression(_)]
        ));
        assert!(canvas.routes[0].source_payloads.is_empty());
        assert!(!canvas.routes[0].ordered_payloads);
    }

    #[test]
    fn media_facts_retain_stable_expression_owners_and_static_contract() {
        let source = r#"app MediaFacts
extern crate::backend
  svg-style dynamic_svg(active:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  svg "icon.svg" w=48.0 h=shrink fit=scale-down rotate=rotation.solid(radians(0.1)) opacity=0.9 color=fg hover=primary style=dynamic_svg(active) label="Icon" description="Status icon"
"#;
        let checked_document = analyze(source).unwrap();
        let ViewNode::Media {
            kind,
            source,
            options,
            ..
        } = &checked_document.document.view
        else {
            panic!("fixture root must be media");
        };
        let expected_expression_count = crate::ast::media_expression_roots(source, options).len();
        let expected_semantic_key = crate::ast::media_semantic_key(*kind, options);
        let program = lower::lower(checked_document).unwrap();
        let checked = program.checked_facts().media(ViewId(0)).unwrap();
        assert_eq!(checked.id, ViewId(0));
        assert_eq!(checked.expression_count as usize, expected_expression_count);
        assert_eq!(checked.style, Some(ExternFnId(0)));
        assert_eq!(checked.semantic_key, expected_semantic_key);
        for index in 0..checked.expression_count {
            assert!(
                program
                    .checked_facts()
                    .expression_use_by_owner(CheckedExprOwner::Media(MediaExpressionId {
                        media: checked.id,
                        index,
                    }))
                    .is_some(),
                "missing media expression {index}"
            );
        }
    }

    #[test]
    fn tooltip_facts_retain_stable_expression_owners_and_static_contract() {
        let source = r#"app TooltipFacts
extern crate::backend
  box-style dynamic_tooltip(active:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  tooltip position=cursor gap=2.0 p=5.0 delay=100 snap=false style=dynamic_tooltip(active) bg=linear(1.57, bg@0.0, primary/25@1.0) text=fg border=primary/75 border-w=1.0 r=5.0 r-tl=2.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 px-snap=true
    text "Hover"
    text "Tip"
"#;
        let checked_document = analyze(source).unwrap();
        let ViewNode::Tooltip { options, .. } = &checked_document.document.view else {
            panic!("fixture root must be a tooltip");
        };
        let expected_expression_count = crate::ast::tooltip_expression_roots(options).len();
        let expected_semantic_key = crate::ast::tooltip_semantic_key(options);
        let program = lower::lower(checked_document).unwrap();
        let checked = program.checked_facts().tooltip(ViewId(0)).unwrap();
        assert_eq!(checked.id, ViewId(0));
        assert_eq!(checked.expression_count as usize, expected_expression_count);
        assert_eq!(checked.style, Some(ExternFnId(0)));
        assert_eq!(checked.semantic_key, expected_semantic_key);
        for index in 0..checked.expression_count {
            assert!(
                program
                    .checked_facts()
                    .expression_use_by_owner(CheckedExprOwner::Tooltip(TooltipExpressionId {
                        tooltip: checked.id,
                        index,
                    }))
                    .is_some(),
                "missing tooltip expression {index}"
            );
        }
    }

    #[test]
    fn interaction_facts_retain_routes_expressions_and_static_contracts() {
        let source = r#"app InteractionFacts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  active = true
on pressed(flag)
on moved(x, y)
on scrolled(x, y, pixels)
on resized(dx, dy)
on hidden
view
  col
    mouse press=pressed(active) move=moved scroll=scrolled cursor=pointer
      text "Pointer"
    resize-handle drag=resized cursor=resize-horizontal
      text "Resize"
    sensor show=resized resize=resized hide=hidden key=active anticipate=16.0 delay=20
      text "Observed"
"#;
        let checked_document = analyze(source).unwrap();
        let ViewNode::Layout { children, .. } = &checked_document.document.view else {
            panic!("fixture root must be a column");
        };
        let expected_sensor_key = match &children[2] {
            ViewNode::Sensor { options, .. } => crate::ast::sensor_semantic_key(options),
            _ => panic!("third child must be a sensor"),
        };
        let expected_mouse_key = match &children[0] {
            ViewNode::MouseArea { options, .. } => crate::ast::mouse_area_semantic_key(options),
            _ => panic!("first child must be a mouse area"),
        };
        let program = lower::lower(checked_document).unwrap();
        let facts = program.checked_facts();
        assert_eq!(facts.interactions.len(), 7);
        let layout = facts.interaction(ViewId(0)).expect("checked root layout");
        assert_eq!(layout.kind, CheckedInteractionKind::Layout);
        assert_eq!(layout.expression_count, 0);
        assert!(layout.routes.is_empty());

        let mouse = facts.interaction(ViewId(1)).expect("checked mouse area");
        assert_eq!(mouse.kind, CheckedInteractionKind::MouseArea);
        assert_eq!(mouse.expression_count, 1);
        assert_eq!(mouse.routes.len(), 3);
        assert_eq!(
            mouse.routes[0].id,
            InteractionRouteId {
                widget: mouse.id,
                index: 0,
            }
        );
        assert!(matches!(
            mouse.routes[0].args.as_slice(),
            [CheckedCanvasRouteArg::Expression(_)]
        ));
        assert!(mouse.routes[0].source_payloads.is_empty());
        assert!(!mouse.routes[0].ordered_payloads);
        assert!(matches!(
            mouse.routes[1].args.as_slice(),
            [
                CheckedCanvasRouteArg::Payload,
                CheckedCanvasRouteArg::Payload
            ]
        ));
        assert_eq!(mouse.routes[1].source_payloads, [Type::F64, Type::F64]);
        assert!(mouse.routes[1].ordered_payloads);
        assert!(
            facts
                .expression_use_by_owner(CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: mouse.id,
                    index: 0,
                }))
                .is_some()
        );

        let resize = facts.interaction(ViewId(3)).expect("checked resize handle");
        assert_eq!(resize.kind, CheckedInteractionKind::ResizeHandle);
        assert_eq!(resize.expression_count, 0);
        assert_eq!(resize.routes.len(), 1);
        assert_eq!(resize.routes[0].source_payloads, [Type::F64, Type::F64]);
        assert!(resize.routes[0].ordered_payloads);

        let sensor = facts.interaction(ViewId(5)).expect("checked sensor");
        assert_eq!(sensor.kind, CheckedInteractionKind::Sensor);
        assert_eq!(sensor.expression_count, 3);
        assert_eq!(sensor.option_expressions.len(), 3);
        assert_eq!(sensor.routes.len(), 3);
        assert_eq!(sensor.routes[0].source_payloads, [Type::F64, Type::F64]);
        assert!(sensor.routes[0].ordered_payloads);
        assert!(sensor.routes[2].source_payloads.is_empty());
        assert!(!sensor.routes[2].ordered_payloads);
        for (index, expected) in [Type::Bool, Type::F64, Type::I64].into_iter().enumerate() {
            let expression = facts.expression_use(sensor.option_expressions[index]);
            assert_eq!(expression.source, expected);
            assert_eq!(expression.destination, expected);
        }
        assert_eq!(sensor.semantic_key, expected_sensor_key);
        assert_eq!(mouse.semantic_key, expected_mouse_key);
        for id in [ViewId(2), ViewId(4), ViewId(6)] {
            assert_eq!(
                facts.interaction(id).expect("checked child text").kind,
                CheckedInteractionKind::Text
            );
        }
    }

    #[test]
    fn float_facts_retain_expression_owners_geometry_locals_and_static_contract() {
        let source = r#"app FloatFacts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  shift = 2.0
view
  float scale=1.1 x=(viewport_x + shift) y=(original_y - shift) shadow=primary/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 r=8.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0
    text "Floating"
"#;
        let checked_document = analyze(source).unwrap();
        let ViewNode::Float { style, .. } = &checked_document.document.view else {
            panic!("root must be a float");
        };
        let expected_semantic_key = crate::ast::float_semantic_key(style);
        let program = lower::lower(checked_document).unwrap();
        let checked = program.checked_facts().view(ViewId(0));
        let CheckedViewFlow::Float {
            semantic_key,
            expression_count,
            geometry,
        } = &checked.flow
        else {
            panic!("root must retain float facts");
        };
        assert_eq!(*expression_count, 11);
        let expected_roles = [
            CheckedViewLocalRole::FloatOriginalX,
            CheckedViewLocalRole::FloatOriginalY,
            CheckedViewLocalRole::FloatOriginalWidth,
            CheckedViewLocalRole::FloatOriginalHeight,
            CheckedViewLocalRole::FloatViewportX,
            CheckedViewLocalRole::FloatViewportY,
            CheckedViewLocalRole::FloatViewportWidth,
            CheckedViewLocalRole::FloatViewportHeight,
        ];
        for (local, role) in geometry.iter().zip(expected_roles) {
            let local = program.checked_facts().local(*local);
            assert_eq!(local.ty, Type::F64);
            assert_eq!(
                local.owner,
                CheckedLocalOwner::View {
                    view: ViewId(0),
                    role
                }
            );
        }
        for index in 0..*expression_count {
            assert!(
                program
                    .checked_facts()
                    .expression_use_by_owner(CheckedExprOwner::Float(FloatExpressionId {
                        float: ViewId(0),
                        index,
                    }))
                    .is_some(),
                "missing float expression {index}"
            );
        }
        assert_eq!(semantic_key, &expected_semantic_key);
    }

    #[test]
    fn pin_facts_retain_expression_owners_and_dimension_contract() {
        let source = r#"app PinFacts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  offset = 12.0
  height = 80.0
view
  pin w=fill h=height x=offset y=8.0
    text "Pinned"
"#;
        let checked_document = analyze(source).unwrap();
        let ViewNode::Pin { width, height, .. } = &checked_document.document.view else {
            panic!("root must be a pin");
        };
        let expected_semantic_key = crate::ast::pin_semantic_key(width, height);
        let program = lower::lower(checked_document).unwrap();
        let checked = program.checked_facts().view(ViewId(0));
        let CheckedViewFlow::Pin {
            semantic_key,
            expression_count,
        } = &checked.flow
        else {
            panic!("root must retain pin facts");
        };
        assert_eq!(*expression_count, 3);
        for index in 0..*expression_count {
            assert!(
                program
                    .checked_facts()
                    .expression_use_by_owner(CheckedExprOwner::Pin(PinExpressionId {
                        pin: ViewId(0),
                        index,
                    }))
                    .is_some(),
                "missing pin expression {index}"
            );
        }
        assert_eq!(semantic_key, &expected_semantic_key);
    }

    #[test]
    fn responsive_facts_retain_dimensions_and_size_locals() {
        let source = r#"app ResponsiveFacts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  height = 40.0
view
  responsive size=(available_width, available_height) w=fill h=height
    text available_width
"#;
        let program = lower::lower(analyze(source).unwrap()).unwrap();
        let checked = program.checked_facts().view(ViewId(0));
        let CheckedViewFlow::ResponsiveSize {
            expression_count,
            width,
            height,
            dimensions,
            ..
        } = &checked.flow
        else {
            panic!("root must retain responsive size facts");
        };
        assert_eq!(*expression_count, 1);
        assert_eq!(dimensions[0], CheckedLength::Fill);
        assert!(matches!(
            dimensions[1],
            CheckedLength::Fixed {
                source: Type::F64,
                ..
            }
        ));
        for (local, role) in [
            (*width, CheckedViewLocalRole::ResponsiveWidth),
            (*height, CheckedViewLocalRole::ResponsiveHeight),
        ] {
            let local = program.checked_facts().local(local);
            assert_eq!(local.ty, Type::F64);
            assert_eq!(
                local.owner,
                CheckedLocalOwner::View {
                    view: ViewId(0),
                    role,
                }
            );
        }
    }

    #[test]
    fn lazy_facts_retain_dependency_owner_and_binding_local() {
        let source = r#"app LazyFacts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  title = "Hello"
view
  lazy title as cached
    text cached
"#;
        let program = lower::lower(analyze(source).unwrap()).unwrap();
        let CheckedViewFlow::Lazy {
            dependency,
            keys,
            key_bindings,
            binding,
        } = &program.checked_facts().view(ViewId(0)).flow
        else {
            panic!("root must retain lazy facts");
        };
        assert!(keys.is_empty(), "plain lazy records no keys");
        assert!(key_bindings.is_empty(), "plain lazy records no key locals");
        assert_eq!(
            program.checked_facts().expression_use(*dependency).owner,
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::LazyDependency,
            }
        );
        let binding = program.checked_facts().local(*binding);
        assert_eq!(binding.name, "cached");
        assert_eq!(binding.ty, Type::Str);
        assert_eq!(
            binding.owner,
            CheckedLocalOwner::View {
                view: ViewId(0),
                role: CheckedViewLocalRole::LazyDependency,
            }
        );
    }

    #[test]
    fn keyed_facts_retain_flow_layout_expressions_and_item_local() {
        let source = r#"app KeyedFacts
extern crate::backend
  Item(id:i64, name:str)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
view
  keyed item in items by=item.id w=fill(2) h=120.0 gap=8.0 p=4.0 max-w=640.0 align=end
    text item.name
"#;
        let program = lower::lower(analyze(source).unwrap()).unwrap();
        let CheckedViewFlow::Keyed {
            items,
            key,
            item,
            layout,
        } = &program.checked_facts().view(ViewId(0)).flow
        else {
            panic!("root must retain keyed facts");
        };
        assert_eq!(
            program.checked_facts().expression_use(*items).owner,
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::KeyedItems,
            }
        );
        assert_eq!(
            program.checked_facts().expression_use(*key).owner,
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::KeyedKey,
            }
        );
        let item = program.checked_facts().local(*item);
        assert_eq!(item.name, "item");
        assert_eq!(
            item.owner,
            CheckedLocalOwner::View {
                view: ViewId(0),
                role: CheckedViewLocalRole::KeyedItem,
            }
        );
        assert!(matches!(layout.width, CheckedLength::FillPortion(2)));
        assert!(matches!(layout.height, CheckedLength::Fixed { .. }));
        assert!(layout.spacing.is_some());
        assert!(layout.padding.all.is_some());
        assert!(layout.max_width.is_some());
        assert_eq!(layout.align, Some(FlexAlignment::End));
    }

    #[test]
    fn canvas_codegen_uses_checked_expressions_and_rejects_static_contract_mutation() {
        let source = format!(
            "app FrozenCanvas\n{THEME}view\n  canvas w=120.0 h=80.0\n    circle x=10.0 y=20.0 r=4.0 fill=primary\n"
        );
        let mut expression_mutation = analyze(&source).unwrap();
        let ViewNode::Canvas { commands, .. } = &mut expression_mutation.document.view else {
            panic!("fixture root must be a canvas");
        };
        let CanvasCommand::Circle { x, .. } = &mut commands[0] else {
            panic!("fixture command must be a circle");
        };
        *x = Expr::F64(99.0);
        let program = lower::lower(expression_mutation).unwrap();
        let generated = crate::codegen::generate(&program, "frozen-canvas.ice").unwrap();
        assert!(generated.contains("::iced::Point::new(10.0 as f32, 20.0 as f32)"));
        assert!(!generated.contains("::iced::Point::new(99.0 as f32, 20.0 as f32)"));

        let mut static_mutation = analyze(&source).unwrap();
        let ViewNode::Canvas { commands, .. } = &mut static_mutation.document.view else {
            panic!("fixture root must be a canvas");
        };
        let CanvasCommand::Circle { paint, .. } = &mut commands[0] else {
            panic!("fixture command must be a circle");
        };
        paint.fill = Some(BackgroundValue::Color("fg".into()));
        let error = lower::lower(static_mutation).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("canvas command has no declaration"));
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
        let nonce = crate::test_support::unique_nonce();
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
        let nonce = crate::test_support::unique_nonce();
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
        let nonce = crate::test_support::unique_nonce();
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
            "enum Status\n  idle\n  loaded(str)\nextern crate::backend\n  User(name:str)\n  pure load_user() -> User\n",
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
        assert_eq!(extern_decl.kind, ExternKind::Pure);
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
            "app Projection\nextern crate::backend\n  sync now() -> instant\n{THEME}state\n  progress:animation[f64] = 0.0\n  at:instant = now()\nderived\n  projected = animation.project(progress, sample, sample * 2.0, at)\nview\n  text projected\n"
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
                expression: CheckedExprUseId(2),
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
                CheckedCallArgument::Value(_),
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
        assert_eq!(facts.metrics().type_analysis_nodes, 9);
        assert_eq!(facts.metrics().expressions, 9);
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
            "app BuiltinContexts\n{THEME}state\n  task_was_aborted:bool = aborted(none)\nderived\n  gradient = linear.add_stops(linear(0.0), [])\n  debug_is_active = debug.active(none)\nview\n  text \"contexts\"\n"
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
        // The normalized Text view retains its literal as one checked expression in
        // addition to the value initializers measured by this contract.
        assert_eq!(facts.metrics().expressions, VALUES + 1);
        assert_eq!(facts.metrics().type_analysis_queries, VALUES + 1);
        assert_eq!(facts.metrics().type_analysis_nodes, VALUES + 1);
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
        // The state initializer and the normalized Text value are distinct uses;
        // Text contributes one path node without changing the deep-expression slope.
        assert_eq!(metrics.expression_uses, 2);
        assert_eq!(metrics.expressions, nodes + 1);
        assert_eq!(metrics.type_analysis_queries, nodes + 1);
        assert_eq!(metrics.type_analysis_nodes, nodes + 1);
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
                "app ProjectionFacts\nextern crate::backend\n  sync now() -> instant\n{THEME}state\n  progress:animation[f64] = 0.0\n  at:instant = now()\nderived\n"
            );
            for index in 0..derived {
                writeln!(
                    source,
                    "  value_{index} = animation.project(progress, sample, sample + 1.0, at)"
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
        assert_eq!(large.values, 4_002);
        assert_eq!(large.expression_uses, 4_003);
        assert_eq!(large.initializer_analysis_passes, 4_002);
        assert_eq!(large.scope_env_builds, 2);
        assert_eq!(large.scope_env_entries, 8_004);
        assert_eq!(large.locals, 4_000);
        assert_eq!(large.type_scope_env_full_clones, 0);
        assert_eq!(large.scope_env_full_clones, 0);
        assert_eq!(large.scope_env_overlays, 4_000);
        assert_eq!(large.type_scope_env_overlays, 8_000);
        // Remove the fixed state initializers and normalized Text expression before
        // comparing the per-projection work.
        assert_eq!(large.expressions - 3, (small.expressions - 3) * 8);
        assert_eq!(
            large.type_analysis_nodes - 3,
            (small.type_analysis_nodes - 3) * 8
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

    #[test]
    #[ignore = "explicit retained subscription-expression linearity contract"]
    fn performance_contract_four_thousand_subscriptions_reuse_one_app_scope() {
        fn measure(count: usize) -> (CheckedFactMetrics, usize, usize, usize, std::time::Duration) {
            let mut source = String::from("app SubscriptionFacts\nextern crate::backend\n");
            for index in 0..count {
                writeln!(source, "  pure retain_{index}(value:instant) -> instant?").unwrap();
            }
            write!(
                source,
                "{THEME}state\n  enabled = true\n  tag = 7\non tick(tag, now)\nsubscribe\n"
            )
            .unwrap();
            for index in 0..count {
                writeln!(
                    source,
                    "  every 10ms with=tag filter=retain_{index} when enabled -> tick _ _"
                )
                .unwrap();
            }
            source.push_str("view\n  text \"ready\"\n");
            let started = Instant::now();
            let program = lower::lower(analyze(&source).unwrap()).unwrap();
            let generated =
                crate::codegen::generate(&program, "subscription-performance.ice").unwrap();
            let generated_subscriptions = generated.matches("::iced::time::every").count();
            (
                program.checked_facts().metrics(),
                program.declarations().extern_name_lookup_count(),
                generated_subscriptions,
                generated.len(),
                started.elapsed(),
            )
        }

        let (small, small_lookups, small_generated, small_rust_bytes, small_elapsed) = measure(500);
        let (large, large_lookups, large_generated, large_rust_bytes, large_elapsed) =
            measure(4_000);
        assert_eq!(large.subscription_analysis_passes, 8_000);
        assert_eq!(small_lookups, 500);
        assert_eq!(large_lookups, 4_000);
        assert_eq!(small_generated, 500);
        assert_eq!(large_generated, 4_000);
        assert!(large_rust_bytes > small_rust_bytes * 6);
        // enabled, tag, and the normalized Text literal are fixed expressions.
        assert_eq!(large.expression_uses - 3, (small.expression_uses - 3) * 8);
        assert_eq!(large.expressions - 3, (small.expressions - 3) * 8);
        assert_eq!(
            large.type_analysis_nodes - 3,
            (small.type_analysis_nodes - 3) * 8
        );
        assert_eq!(large.type_scope_env_full_clones, 0);
        assert_eq!(large.scope_env_full_clones, 0);
        eprintln!(
            "500 subscriptions through codegen in {small_elapsed:?} ({small_rust_bytes} Rust bytes); 4k in {large_elapsed:?} ({large_rust_bytes} Rust bytes)"
        );
        assert!(
            large_elapsed.as_secs_f64() < 8.0,
            "4k subscriptions completed in {large_elapsed:?}"
        );
        assert!(
            large_elapsed.as_secs_f64() <= small_elapsed.as_secs_f64() * 12.0 + 0.5,
            "subscription scaling exceeded the linear allowance: 500={small_elapsed:?}, 4k={large_elapsed:?}"
        );
    }
}
