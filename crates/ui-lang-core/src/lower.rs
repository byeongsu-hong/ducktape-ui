use crate::ast::*;
use crate::check::{
    BuiltinArgumentContext, CheckedBinaryOperator, CheckedCallArgument, CheckedCallTarget,
    CheckedComponentArgumentSource, CheckedExprId, CheckedExprKind, CheckedExprOwner, CheckedFacts,
    CheckedInitializerCoercion, CheckedLocalId, CheckedLocalOwner, CheckedPathRoot,
    CheckedProjectionKind, CheckedUnaryOperator, CheckedValueRef, ContextualBuiltin,
    SubscriptionExpressionContract, canonical_builtin_type, field_type, resolve_erased_type,
};
pub(crate) use crate::check::{
    CheckedExprUseId, CheckedSubscription, CheckedSubscriptionExprRole, CheckedSubscriptionSource,
};
use crate::hir::Origin;
pub(crate) use crate::hir::{
    AppSettingExprId, AppSettingsId, AppStateId, ComponentCallId, ComponentEventId, ComponentId,
    ComponentParamId, ComponentSlotId, ComponentStateId, DeclarationIndex, ExternFnId, ExternRef,
    HandlerId, HandlerOwner, NamedTypeId, NamedWindowId, OriginArena, OriginId, PaletteId, RouteId,
    RunSiteId, StatementId, SubscriptionId, TaskId,
};
use crate::{CheckedDocument, Error};
use std::collections::{HashMap, HashSet};
use std::path::Path;

mod style;

pub(crate) use style::*;

pub(crate) type ResolvedExpressionId = CheckedExprUseId;

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ComponentParamContract {
    id: ComponentParamId,
    name: String,
    ty: Type,
    capability: ParamCapability,
    default: Option<CheckedExprUseId>,
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
    pub(crate) id: ComponentStateId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: ResolvedInitializer,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct AppStateContract {
    pub(crate) id: AppStateId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: ResolvedInitializer,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct DerivedContract {
    pub(crate) id: crate::hir::DerivedId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: CheckedExprUseId,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedType {
    Value(Type),
    List(Box<ResolvedType>),
    Option(Box<ResolvedType>),
    Result(Box<ResolvedType>, Box<ResolvedType>),
    Combo(Box<ResolvedType>),
    Animation(Box<ResolvedType>),
    Named(NamedTypeId),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct ResolvedExternContract {
    pub(crate) id: ExternFnId,
    pub(crate) name: String,
    pub(crate) rust_path: String,
    pub(crate) params: Vec<ResolvedType>,
    pub(crate) output: ResolvedType,
    pub(crate) error: Option<ResolvedType>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedSubscriptionSource {
    Every {
        milliseconds: u64,
    },
    Repeat {
        function: ResolvedExternContract,
        milliseconds: u64,
    },
    Run {
        function: ResolvedExternContract,
        arguments: Vec<CheckedExprUseId>,
    },
    Recipe {
        function: ResolvedExternContract,
        arguments: Vec<CheckedExprUseId>,
    },
    Events {
        identity: CheckedExprUseId,
        filter: ResolvedExternContract,
    },
    Event {
        raw: bool,
    },
    Extern {
        function: ResolvedExternContract,
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
#[allow(dead_code)]
pub(crate) struct ResolvedSubscriptionRoute {
    pub(crate) handler: HandlerId,
    pub(crate) handler_name: String,
    pub(crate) payloads: Vec<u32>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct ResolvedSubscription {
    pub(crate) id: SubscriptionId,
    pub(crate) source: ResolvedSubscriptionSource,
    pub(crate) source_payloads: Vec<ResolvedType>,
    pub(crate) delivered_payloads: Vec<ResolvedType>,
    pub(crate) filter: Option<ResolvedExternContract>,
    pub(crate) context: Option<CheckedExprUseId>,
    pub(crate) condition: Option<CheckedExprUseId>,
    pub(crate) window_id: bool,
    pub(crate) status: Option<EventStatus>,
    pub(crate) route: ResolvedSubscriptionRoute,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

struct ValidatedSubscriptionContract {
    source_payloads: Vec<Type>,
    delivered_payloads: Vec<Type>,
    filter: Option<ResolvedExternContract>,
}

fn subscription_source_matches(
    checked: &CheckedSubscriptionSource,
    raw: &SubscriptionSource,
) -> bool {
    match (checked, raw) {
        (
            CheckedSubscriptionSource::Every { milliseconds },
            SubscriptionSource::Every {
                milliseconds: raw_milliseconds,
            },
        ) => milliseconds == raw_milliseconds,
        (
            CheckedSubscriptionSource::Repeat {
                function,
                milliseconds,
            },
            SubscriptionSource::Repeat {
                function: raw_function,
                milliseconds: raw_milliseconds,
            },
        ) => function.name == *raw_function && milliseconds == raw_milliseconds,
        (
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            },
            SubscriptionSource::Run {
                function: raw_function,
                args,
            },
        )
        | (
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            },
            SubscriptionSource::Recipe {
                function: raw_function,
                args,
            },
        )
        | (
            CheckedSubscriptionSource::Extern {
                function,
                arguments,
            },
            SubscriptionSource::Extern {
                function: raw_function,
                args,
            },
        ) => function.name == *raw_function && arguments.len() == args.len(),
        (
            CheckedSubscriptionSource::Events { filter, .. },
            SubscriptionSource::Events {
                filter: raw_filter, ..
            },
        ) => filter.name == *raw_filter,
        (
            CheckedSubscriptionSource::Event { raw: checked_raw },
            SubscriptionSource::Event { raw },
        ) => checked_raw == raw,
        (
            CheckedSubscriptionSource::InputMethod(event),
            SubscriptionSource::InputMethod(raw_event),
        ) => event == raw_event,
        (CheckedSubscriptionSource::Keyboard(event), SubscriptionSource::Keyboard(raw_event)) => {
            event == raw_event
        }
        (CheckedSubscriptionSource::Mouse(event), SubscriptionSource::Mouse(raw_event)) => {
            event == raw_event
        }
        (CheckedSubscriptionSource::SystemTheme, SubscriptionSource::SystemTheme) => true,
        (CheckedSubscriptionSource::Touch(event), SubscriptionSource::Touch(raw_event)) => {
            event == raw_event
        }
        (CheckedSubscriptionSource::Window(event), SubscriptionSource::Window(raw_event)) => {
            event == raw_event
        }
        _ => false,
    }
}

fn extern_subscription_payload(function: &crate::hir::ExternDeclaration) -> Type {
    function.error.as_ref().map_or_else(
        || function.output.clone(),
        |error| Type::Result(Box::new(function.output.clone()), Box::new(error.clone())),
    )
}

fn resolved_native_subscription_payloads(
    source: &CheckedSubscriptionSource,
    window_id: bool,
) -> Option<Vec<Type>> {
    let mut payloads = match source {
        CheckedSubscriptionSource::Every { .. } => vec![Type::Instant],
        CheckedSubscriptionSource::Event { .. } => vec![Type::Event],
        CheckedSubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => Vec::new(),
            InputMethodEvent::Preedit => vec![
                Type::Str,
                Type::Option(Box::new(Type::I64)),
                Type::Option(Box::new(Type::I64)),
            ],
            InputMethodEvent::Commit => vec![Type::Str],
        },
        CheckedSubscriptionSource::Keyboard(KeyboardEvent::Press) => vec![Type::KeyPress],
        CheckedSubscriptionSource::Keyboard(KeyboardEvent::Release) => vec![Type::KeyRelease],
        CheckedSubscriptionSource::Keyboard(KeyboardEvent::Modifiers) => vec![Type::KeyModifiers],
        CheckedSubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => Vec::new(),
            MouseEvent::Moved => vec![Type::F64, Type::F64],
            MouseEvent::Pressed | MouseEvent::Released => vec![Type::MouseButton],
            MouseEvent::Wheel => vec![Type::F64, Type::F64, Type::Bool],
        },
        CheckedSubscriptionSource::SystemTheme => vec![Type::Str],
        CheckedSubscriptionSource::Touch(_) => {
            vec![Type::TouchFinger, Type::F64, Type::F64]
        }
        CheckedSubscriptionSource::Window(event) => match event {
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => Vec::new(),
            WindowEvent::Opened => vec![
                Type::Option(Box::new(Type::F64)),
                Type::Option(Box::new(Type::F64)),
                Type::F64,
                Type::F64,
            ],
            WindowEvent::Moved | WindowEvent::Resized => vec![Type::F64, Type::F64],
            WindowEvent::Rescaled => vec![Type::F64],
            WindowEvent::FileHovered | WindowEvent::FileDropped => vec![Type::Str],
        },
        _ => return None,
    };
    if window_id {
        if !matches!(
            source,
            CheckedSubscriptionSource::Event { .. } | CheckedSubscriptionSource::Window(_)
        ) {
            return None;
        }
        payloads.insert(0, Type::WindowId);
    }
    Some(payloads)
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInitializer {
    pub(crate) expression: CheckedExprUseId,
    pub(crate) animation: Option<ResolvedAnimation>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedAnimation {
    pub(crate) easing: Option<ResolvedAnimationEasing>,
    pub(crate) duration: Option<AnimationDuration>,
    pub(crate) delay_ms: Option<u64>,
    pub(crate) repeat: Option<u32>,
    pub(crate) repeat_forever: bool,
    pub(crate) auto_reverse: bool,
}

// These fields are the normalized compiler contract. Some are consumed only by
// invariant tests today and remain available to later lowering slices.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedHandler {
    pub(crate) id: HandlerId,
    pub(crate) owner: HandlerOwner,
    pub(crate) name: String,
    pub(crate) params: Vec<ResolvedHandlerParam>,
    pub(crate) statements: Vec<ResolvedStatement>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProgramKind {
    Application,
    Daemon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedRendererSelection {
    Default,
    Custom { path: String, origin: OriginId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedExecutorSelection {
    Default,
    Custom { path: String, origin: OriginId },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedAppExpression {
    pub(crate) id: AppSettingExprId,
    pub(crate) expression: CheckedExprUseId,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedHandlerParam {
    pub(crate) local: crate::check::CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedStatement {
    pub(crate) id: StatementId,
    pub(crate) kind: ResolvedStatementKind,
    pub(crate) task: Option<TaskId>,
    pub(crate) is_final: bool,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedFontAsset {
    pub(crate) path: String,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ResolvedStatementKind {
    Let {
        local: crate::check::CheckedLocalId,
        name: String,
        ty: Type,
        value: CheckedExprUseId,
    },
    Assign {
        target: ResolvedWritableState,
        value: CheckedExprUseId,
        at: Option<CheckedExprUseId>,
        move_self: bool,
    },
    MarkdownAppend {
        target: ResolvedWritableState,
        value: CheckedExprUseId,
    },
    ComboPush {
        target: ResolvedWritableState,
        value: CheckedExprUseId,
    },
    ReturnIf {
        condition: CheckedExprUseId,
    },
    Exit,
    Run(ResolvedRun),
    Sip(ResolvedSip),
    TaskFlow(ResolvedTaskFlow),
    TaskGroup {
        kind: TaskGroupKind,
        statements: Vec<ResolvedStatement>,
    },
    Abortable {
        handle: ResolvedWritableState,
        abort_on_drop: bool,
        task: Box<ResolvedStatement>,
    },
    Abort {
        handle: ResolvedWritableState,
    },
    DebugStart {
        name: CheckedExprUseId,
        target: ResolvedWritableState,
    },
    DebugFinish {
        target: ResolvedWritableState,
    },
    ClipboardWrite {
        primary: bool,
        value: CheckedExprUseId,
    },
    WidgetOperation {
        operation: ResolvedWidgetOperation,
        route: Option<ResolvedRoute>,
    },
    PaneOperation {
        grid: String,
        dynamic: bool,
        operation: ResolvedPaneOperation,
        route: Option<ResolvedRoute>,
    },
    WindowOperation {
        operation: ResolvedWindowOperation,
        target: Option<CheckedExprUseId>,
        route: Option<ResolvedRoute>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedWritableState {
    pub(crate) value: CheckedValueRef,
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedEffectTarget {
    Builtin(String),
    Extern(ExternFnId),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedRun {
    pub(crate) kind: EffectKind,
    pub(crate) mode: FutureMode,
    pub(crate) site: Option<RunSiteId>,
    pub(crate) target: ResolvedEffectTarget,
    pub(crate) args: Vec<CheckedExprUseId>,
    pub(crate) success: ResolvedRoute,
    pub(crate) error: Option<ResolvedRoute>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSip {
    pub(crate) target: ExternFnId,
    pub(crate) args: Vec<CheckedExprUseId>,
    pub(crate) progress: ResolvedRoute,
    pub(crate) success: ResolvedRoute,
    pub(crate) error: Option<ResolvedRoute>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTaskFlow {
    pub(crate) source: ResolvedTaskSource,
    pub(crate) transforms: Vec<ResolvedTaskTransform>,
    pub(crate) output: Option<Type>,
    pub(crate) error_type: Option<Type>,
    pub(crate) success: Option<ResolvedRoute>,
    pub(crate) error: Option<ResolvedRoute>,
    pub(crate) units: Option<ResolvedRoute>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTaskSource {
    Effect {
        task: TaskId,
        kind: EffectKind,
        target: ResolvedEffectTarget,
        args: Vec<CheckedExprUseId>,
    },
    Done {
        task: TaskId,
        value: CheckedExprUseId,
    },
    None {
        task: TaskId,
        output: Type,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedDefaultFont {
    pub(crate) family: FontFamily,
    pub(crate) weight: FontWeight,
    pub(crate) stretch: FontStretch,
    pub(crate) style: FontStyle,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedWindowIcon {
    pub(crate) path: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) byte_len: usize,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ResolvedWindowPosition {
    Default,
    Centered,
    Specific(f64, f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedWindowLevel {
    Normal,
    AlwaysOnBottom,
    AlwaysOnTop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedWindowCorner {
    Default,
    DoNotRound,
    Round,
    RoundSmall,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ResolvedTaskTransform {
    Map {
        task: TaskId,
        local: crate::check::CheckedLocalId,
        binding: String,
        input: Type,
        input_fallible: bool,
        value: CheckedExprUseId,
    },
    Then {
        task: TaskId,
        local: crate::check::CheckedLocalId,
        binding: String,
        input: Type,
        source: ResolvedTaskSource,
    },
    AndThen {
        task: TaskId,
        local: crate::check::CheckedLocalId,
        binding: String,
        input: Type,
        source: ResolvedTaskSource,
    },
    MapError {
        task: TaskId,
        local: crate::check::CheckedLocalId,
        binding: String,
        input: Type,
        value: CheckedExprUseId,
    },
    Collect {
        task: TaskId,
    },
    Discard {
        task: TaskId,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedRoute {
    pub(crate) id: RouteId,
    pub(crate) target: ResolvedRouteTarget,
    pub(crate) args: Vec<ResolvedRouteArg>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedLinuxWindowSettings {
    pub(crate) application_id: Option<String>,
    pub(crate) override_redirect: Option<bool>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ResolvedRouteTarget {
    App {
        handler: HandlerId,
        name: String,
    },
    Component {
        component: ComponentId,
        handler: HandlerId,
        name: String,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedWindowsWindowSettings {
    pub(crate) drag_and_drop: Option<bool>,
    pub(crate) skip_taskbar: Option<bool>,
    pub(crate) undecorated_shadow: Option<bool>,
    pub(crate) corner: Option<ResolvedWindowCorner>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ResolvedRouteArg {
    Expression(CheckedExprUseId),
    Payload { index: u32, ty: Type },
}

/// Backend-neutral typed route argument lowering shared by handler effects and
/// route-bearing surfaces such as Canvas. Statement/task arena ownership is
/// deliberately outside this contract.
pub(crate) struct TypedRouteInputs<'a> {
    pub(crate) source_payloads: &'a [Type],
    pub(crate) ordered: bool,
}

pub(crate) fn lower_typed_route_arguments(
    route: &Route,
    target_params: &[Type],
    inputs: TypedRouteInputs<'_>,
    mut expression: impl FnMut(usize) -> Result<CheckedExprUseId, Error>,
) -> Result<Vec<ResolvedRouteArg>, Error> {
    if route.args.len() != target_params.len() {
        return Err(Error::new(
            "E196",
            &route.span,
            "route argument count diverged from its checked target contract",
        ));
    }
    let mut payload_index = 0usize;
    route
        .args
        .iter()
        .zip(target_params)
        .enumerate()
        .map(|(argument, (raw, target))| match raw {
            RouteArg::Expr(_) => expression(argument).map(ResolvedRouteArg::Expression),
            RouteArg::Payload => {
                let source_index = if inputs.ordered { payload_index } else { 0 };
                let source = inputs.source_payloads.get(source_index).ok_or_else(|| {
                    Error::new(
                        "E196",
                        &route.span,
                        "route payload has no typed source contract",
                    )
                })?;
                if source != target {
                    return Err(Error::new(
                        "E196",
                        &route.span,
                        "route payload type diverged from its checked target parameter",
                    ));
                }
                payload_index += 1;
                Ok(ResolvedRouteArg::Payload {
                    index: source_index as u32,
                    ty: target.clone(),
                })
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedWidgetTarget {
    pub(crate) segments: Vec<ResolvedWidgetTargetSegment>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedWidgetTargetSegment {
    pub(crate) name: String,
    pub(crate) key: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedWidgetSelector {
    Id(ResolvedWidgetTarget),
    Text(CheckedExprUseId),
    Point {
        x: CheckedExprUseId,
        y: CheckedExprUseId,
    },
    Focused,
    Extern {
        target: ExternFnId,
        args: Vec<CheckedExprUseId>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedWidgetOperation {
    FocusPrevious,
    FocusNext,
    Focus {
        target: ResolvedWidgetTarget,
    },
    Focused {
        target: ResolvedWidgetTarget,
    },
    CursorFront {
        target: ResolvedWidgetTarget,
    },
    CursorEnd {
        target: ResolvedWidgetTarget,
    },
    Cursor {
        target: ResolvedWidgetTarget,
        position: CheckedExprUseId,
    },
    SelectAll {
        target: ResolvedWidgetTarget,
    },
    Select {
        target: ResolvedWidgetTarget,
        start: CheckedExprUseId,
        end: CheckedExprUseId,
    },
    Snap {
        target: ResolvedWidgetTarget,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
    },
    SnapEnd {
        target: ResolvedWidgetTarget,
    },
    ScrollTo {
        target: ResolvedWidgetTarget,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
    },
    ScrollBy {
        target: ResolvedWidgetTarget,
        x: CheckedExprUseId,
        y: CheckedExprUseId,
    },
    Find {
        selector: ResolvedWidgetSelector,
        all: bool,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedPaneReference {
    Static(String),
    Dynamic {
        template: String,
        key: CheckedExprUseId,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedPaneOperation {
    Maximize {
        pane: ResolvedPaneReference,
    },
    Restore,
    Maximized,
    Adjacent {
        pane: ResolvedPaneReference,
        edge: PaneEdge,
    },
    Swap {
        first: ResolvedPaneReference,
        second: ResolvedPaneReference,
    },
    Close {
        pane: ResolvedPaneReference,
    },
    Move {
        pane: ResolvedPaneReference,
        edge: PaneEdge,
    },
    Resize {
        split: Option<String>,
        ratio: CheckedExprUseId,
    },
    Drop {
        pane: ResolvedPaneReference,
        target: ResolvedPaneReference,
        edge: Option<PaneEdge>,
    },
    Split {
        target: ResolvedPaneReference,
        pane: ResolvedPaneReference,
        axis: PaneAxis,
        ratio: CheckedExprUseId,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedWindowOperation {
    Open(Option<u32>),
    Oldest,
    Latest,
    Close,
    Drag,
    DragResize(WindowDirection),
    Resize(CheckedExprUseId, CheckedExprUseId),
    Resizable(CheckedExprUseId),
    MinSize(Option<(CheckedExprUseId, CheckedExprUseId)>),
    MaxSize(Option<(CheckedExprUseId, CheckedExprUseId)>),
    ResizeIncrements(Option<(CheckedExprUseId, CheckedExprUseId)>),
    Size,
    IsMaximized,
    Maximize(CheckedExprUseId),
    IsMinimized,
    Minimize(CheckedExprUseId),
    Position,
    ScaleFactor,
    Move(CheckedExprUseId, CheckedExprUseId),
    Mode,
    SetMode(WindowMode),
    ToggleMaximize,
    ToggleDecorations,
    Attention(Option<WindowAttention>),
    Focus,
    SetLevel(WindowLevel),
    SystemMenu,
    RawId,
    Screenshot,
    MousePassthrough(CheckedExprUseId),
    MonitorSize,
    AutomaticTabbing(CheckedExprUseId),
    Icon {
        pixels: CheckedExprUseId,
        width: CheckedExprUseId,
        height: CheckedExprUseId,
    },
    Callback {
        target: ExternFnId,
        args: Vec<CheckedExprUseId>,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedMacosWindowSettings {
    pub(crate) title_hidden: Option<bool>,
    pub(crate) titlebar_transparent: Option<bool>,
    pub(crate) fullsize_content_view: Option<bool>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedWasmWindowSettings {
    pub(crate) target: Option<Option<String>>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedWindowSettings {
    pub(crate) size: Option<(f64, f64)>,
    pub(crate) maximized: Option<bool>,
    pub(crate) fullscreen: Option<bool>,
    pub(crate) position: Option<ResolvedWindowPosition>,
    pub(crate) min_size: Option<(f64, f64)>,
    pub(crate) max_size: Option<(f64, f64)>,
    pub(crate) visible: Option<bool>,
    pub(crate) resizable: Option<bool>,
    pub(crate) closeable: Option<bool>,
    pub(crate) minimizable: Option<bool>,
    pub(crate) decorations: Option<bool>,
    pub(crate) transparent: Option<bool>,
    pub(crate) blur: Option<bool>,
    pub(crate) level: Option<ResolvedWindowLevel>,
    pub(crate) icon: Option<ResolvedWindowIcon>,
    pub(crate) exit_on_close_request: Option<bool>,
    pub(crate) linux: Option<ResolvedLinuxWindowSettings>,
    pub(crate) windows: Option<ResolvedWindowsWindowSettings>,
    pub(crate) macos: Option<ResolvedMacosWindowSettings>,
    pub(crate) wasm: Option<ResolvedWasmWindowSettings>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedNamedWindow {
    pub(crate) id: NamedWindowId,
    pub(crate) name: String,
    pub(crate) settings: ResolvedWindowSettings,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedAppSettings {
    pub(crate) settings_id: AppSettingsId,
    pub(crate) app_name: String,
    pub(crate) kind: ProgramKind,
    pub(crate) callback_window: Option<CheckedLocalId>,
    pub(crate) title: Option<ResolvedAppExpression>,
    pub(crate) background: Option<ResolvedAppExpression>,
    pub(crate) text_color: Option<ResolvedAppExpression>,
    pub(crate) id: Option<String>,
    pub(crate) executor: ResolvedExecutorSelection,
    pub(crate) renderer: ResolvedRendererSelection,
    pub(crate) fonts: Vec<ResolvedFontAsset>,
    pub(crate) default_font: Option<ResolvedDefaultFont>,
    pub(crate) default_text_size: Option<f64>,
    pub(crate) antialiasing: Option<bool>,
    pub(crate) vsync: Option<bool>,
    pub(crate) scale_factor: Option<ResolvedAppExpression>,
    pub(crate) primary_window: ResolvedWindowSettings,
    pub(crate) named_windows: Vec<ResolvedNamedWindow>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedAnimationEasing {
    Builtin(String),
    Custom(ExternFnId),
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
    pub(crate) handlers: Vec<HandlerId>,
    pub(crate) root: ViewNode,
    pub(crate) storage: ComponentStorage,
    pub(crate) origin: OriginId,
}

#[derive(Debug)]
struct ComponentIndex {
    params_by_name: HashMap<String, usize>,
    events_by_name: HashMap<String, usize>,
    slots_by_name: HashMap<String, usize>,
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
    pub(crate) param: ComponentParamId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) expression: CheckedExprUseId,
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
    facts: CheckedFacts,
    declarations: DeclarationIndex,
    settings: ResolvedAppSettings,
    subscriptions: Vec<ResolvedSubscription>,
    named_type_rust_paths: HashMap<NamedTypeId, String>,
    app_states: Vec<AppStateContract>,
    derived: Vec<DerivedContract>,
    components: Vec<ComponentContract>,
    handlers: Vec<ResolvedHandler>,
    app_handlers: Vec<HandlerId>,
    preset_handlers: Vec<HandlerId>,
    calls: Vec<ComponentCall>,
    calls_by_site: HashMap<CallSite, ComponentCallId>,
    styles: StyleProgram,
    origins: OriginArena,
}

fn validate_expression_declaration_references(
    facts: &CheckedFacts,
    declarations: &DeclarationIndex,
) -> Result<(), (OriginId, &'static str)> {
    use crate::check::{
        CheckedCallTarget, CheckedExprKind, CheckedPathRoot, CheckedProjectionKind,
    };

    for expression in facts.expressions() {
        let valid = match &expression.kind {
            CheckedExprKind::SlotProvided(id) => declarations.try_component_slot(*id).is_some(),
            CheckedExprKind::Path { root, projections } => {
                let root_valid = match root {
                    CheckedPathRoot::EnumVariant(id) => {
                        declarations.try_enum_variant_decl(*id).is_some()
                    }
                    CheckedPathRoot::Palette(id) => declarations.palette_name(*id).is_some(),
                    CheckedPathRoot::Value(_) | CheckedPathRoot::Local(_) => true,
                };
                root_valid
                    && projections.iter().all(|projection| match &projection.kind {
                        CheckedProjectionKind::Struct(id) => {
                            declarations.try_struct_field_decl(*id).is_some()
                        }
                        CheckedProjectionKind::Native
                        | CheckedProjectionKind::OptionalWidgetTarget => true,
                    })
            }
            CheckedExprKind::Call { target, .. } => match target {
                CheckedCallTarget::Extern(id) => declarations.try_extern_decl(*id).is_some(),
                CheckedCallTarget::EnumVariant(id) => {
                    declarations.try_enum_variant_decl(*id).is_some()
                }
                CheckedCallTarget::Builtin(_) => true,
            },
            _ => true,
        };
        if !valid {
            return Err((
                expression.origin,
                "expression declaration ID is outside its arena",
            ));
        }
    }
    Ok(())
}

fn resolved_statement_semantic_key(
    program: &LoweredProgram,
    statement: &ResolvedStatement,
) -> Result<String, Error> {
    fn route_shape(route: &ResolvedRoute) -> String {
        let name = match &route.target {
            ResolvedRouteTarget::App { name, .. } | ResolvedRouteTarget::Component { name, .. } => {
                name
            }
        };
        let args = route
            .args
            .iter()
            .map(|arg| match arg {
                ResolvedRouteArg::Payload { .. } => '_',
                ResolvedRouteArg::Expression(_) => 'e',
            })
            .collect::<String>();
        format!("{name}:{args}")
    }

    fn effect_name(
        program: &LoweredProgram,
        target: &ResolvedEffectTarget,
        origin: OriginId,
    ) -> Result<String, Error> {
        match target {
            ResolvedEffectTarget::Builtin(name) => Ok(name.clone()),
            ResolvedEffectTarget::Extern(id) => program
                .declarations
                .try_extern_decl(*id)
                .map(|declaration| declaration.name.clone())
                .ok_or_else(|| {
                    program.invariant_at_origin(origin, "effect target ID is outside its arena")
                }),
        }
    }

    fn source_key(
        program: &LoweredProgram,
        source: &ResolvedTaskSource,
        origin: OriginId,
    ) -> Result<String, Error> {
        Ok(match source {
            ResolvedTaskSource::Effect {
                kind, target, args, ..
            } => format!(
                "effect:{kind:?}:{}:{}",
                effect_name(program, target, origin)?,
                args.len()
            ),
            ResolvedTaskSource::Done { .. } => "done".into(),
            ResolvedTaskSource::None { output, .. } => format!("none:{output:?}"),
        })
    }

    fn transform_key(
        program: &LoweredProgram,
        transform: &ResolvedTaskTransform,
        origin: OriginId,
    ) -> Result<String, Error> {
        Ok(match transform {
            ResolvedTaskTransform::Map { binding, .. } => format!("map:{binding}"),
            ResolvedTaskTransform::Then {
                binding, source, ..
            } => format!("then:{binding}:{}", source_key(program, source, origin)?),
            ResolvedTaskTransform::AndThen {
                binding, source, ..
            } => format!(
                "and-then:{binding}:{}",
                source_key(program, source, origin)?
            ),
            ResolvedTaskTransform::MapError { binding, .. } => {
                format!("map-error:{binding}")
            }
            ResolvedTaskTransform::Collect { .. } => "collect".into(),
            ResolvedTaskTransform::Discard { .. } => "discard".into(),
        })
    }

    Ok(match &statement.kind {
        ResolvedStatementKind::Let { name, .. } => format!("let:{name}"),
        ResolvedStatementKind::Assign { target, at, .. } => {
            format!("assign:{}:{}", target.name, at.is_some())
        }
        ResolvedStatementKind::MarkdownAppend { target, .. } => {
            format!("markdown-append:{}", target.name)
        }
        ResolvedStatementKind::ComboPush { target, .. } => {
            format!("combo-push:{}", target.name)
        }
        ResolvedStatementKind::ReturnIf { .. } => "return-if".into(),
        ResolvedStatementKind::Exit => "exit".into(),
        ResolvedStatementKind::Run(run) => format!(
            "run:{:?}:{:?}:{}:{}:{}:{}",
            run.kind,
            run.mode,
            effect_name(program, &run.target, statement.origin)?,
            run.args.len(),
            route_shape(&run.success),
            run.error.as_ref().map(route_shape).unwrap_or_default()
        ),
        ResolvedStatementKind::Sip(sip) => {
            let function = program
                .declarations
                .try_extern_decl(sip.target)
                .map(|declaration| declaration.name.as_str())
                .ok_or_else(|| {
                    program
                        .invariant_at_origin(statement.origin, "sip target ID is outside its arena")
                })?;
            format!(
                "sip:{function}:{}:{}:{}:{}",
                sip.args.len(),
                route_shape(&sip.progress),
                route_shape(&sip.success),
                sip.error.as_ref().map(route_shape).unwrap_or_default()
            )
        }
        ResolvedStatementKind::TaskFlow(flow) => format!(
            "flow:{}:[{}]:{}:{}:{}",
            source_key(program, &flow.source, statement.origin)?,
            flow.transforms
                .iter()
                .map(|transform| transform_key(program, transform, statement.origin))
                .collect::<Result<Vec<_>, _>>()?
                .join(","),
            flow.success.as_ref().map(route_shape).unwrap_or_default(),
            flow.error.as_ref().map(route_shape).unwrap_or_default(),
            flow.units.as_ref().map(route_shape).unwrap_or_default()
        ),
        ResolvedStatementKind::TaskGroup { kind, .. } => format!("task-group:{kind:?}"),
        ResolvedStatementKind::Abortable {
            handle,
            abort_on_drop,
            ..
        } => format!("abortable:{}:{abort_on_drop}", handle.name),
        ResolvedStatementKind::Abort { handle } => format!("abort:{}", handle.name),
        ResolvedStatementKind::DebugStart { target, .. } => {
            format!("debug-start:{}", target.name)
        }
        ResolvedStatementKind::DebugFinish { target } => {
            format!("debug-finish:{}", target.name)
        }
        ResolvedStatementKind::ClipboardWrite { primary, .. } => {
            format!("clipboard:{primary}")
        }
        ResolvedStatementKind::WidgetOperation {
            operation, route, ..
        } => format!(
            "widget:{:?}:{}",
            std::mem::discriminant(operation),
            route.as_ref().map(route_shape).unwrap_or_default()
        ),
        ResolvedStatementKind::PaneOperation {
            grid,
            operation,
            route,
            ..
        } => format!(
            "pane:{grid}:{:?}:{}",
            std::mem::discriminant(operation),
            route.as_ref().map(route_shape).unwrap_or_default()
        ),
        ResolvedStatementKind::WindowOperation {
            operation,
            target,
            route,
        } => format!(
            "window:{:?}:{}:{}",
            std::mem::discriminant(operation),
            target.is_some(),
            route.as_ref().map(route_shape).unwrap_or_default()
        ),
    })
}

fn resolved_handler_operation_contract(
    program: &LoweredProgram,
    statement: &ResolvedStatement,
) -> Result<Option<crate::hir::HandlerOperationContract>, Error> {
    use crate::hir::{
        CheckedPaneEdge, CheckedPaneOperation, CheckedPaneReference, CheckedWidgetOperation,
        CheckedWidgetSelector, CheckedWidgetTarget, CheckedWindowOperation,
        HandlerOperationContract,
    };

    fn target(value: &ResolvedWidgetTarget) -> CheckedWidgetTarget {
        CheckedWidgetTarget(
            value
                .segments
                .iter()
                .map(|segment| (segment.name.clone(), segment.key.is_some()))
                .collect(),
        )
    }
    fn pane(value: &ResolvedPaneReference) -> CheckedPaneReference {
        match value {
            ResolvedPaneReference::Static(name) => CheckedPaneReference::Static(name.clone()),
            ResolvedPaneReference::Dynamic { template, .. } => {
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
    fn extern_name(
        program: &LoweredProgram,
        id: ExternFnId,
        origin: OriginId,
    ) -> Result<String, Error> {
        program
            .declarations
            .try_extern_decl(id)
            .map(|declaration| declaration.name.clone())
            .ok_or_else(|| {
                program.invariant_at_origin(origin, "operation extern ID is outside its arena")
            })
    }

    Ok(Some(match &statement.kind {
        ResolvedStatementKind::WidgetOperation { operation, .. } => {
            HandlerOperationContract::Widget(match operation {
                ResolvedWidgetOperation::FocusPrevious => CheckedWidgetOperation::FocusPrevious,
                ResolvedWidgetOperation::FocusNext => CheckedWidgetOperation::FocusNext,
                ResolvedWidgetOperation::Focus { target: value } => {
                    CheckedWidgetOperation::Focus(target(value))
                }
                ResolvedWidgetOperation::Focused { target: value } => {
                    CheckedWidgetOperation::Focused(target(value))
                }
                ResolvedWidgetOperation::CursorFront { target: value } => {
                    CheckedWidgetOperation::CursorFront(target(value))
                }
                ResolvedWidgetOperation::CursorEnd { target: value } => {
                    CheckedWidgetOperation::CursorEnd(target(value))
                }
                ResolvedWidgetOperation::Cursor { target: value, .. } => {
                    CheckedWidgetOperation::Cursor(target(value))
                }
                ResolvedWidgetOperation::SelectAll { target: value } => {
                    CheckedWidgetOperation::SelectAll(target(value))
                }
                ResolvedWidgetOperation::Select { target: value, .. } => {
                    CheckedWidgetOperation::Select(target(value))
                }
                ResolvedWidgetOperation::Snap { target: value, .. } => {
                    CheckedWidgetOperation::Snap(target(value))
                }
                ResolvedWidgetOperation::SnapEnd { target: value } => {
                    CheckedWidgetOperation::SnapEnd(target(value))
                }
                ResolvedWidgetOperation::ScrollTo { target: value, .. } => {
                    CheckedWidgetOperation::ScrollTo(target(value))
                }
                ResolvedWidgetOperation::ScrollBy { target: value, .. } => {
                    CheckedWidgetOperation::ScrollBy(target(value))
                }
                ResolvedWidgetOperation::Find { selector, all } => CheckedWidgetOperation::Find {
                    selector: match selector {
                        ResolvedWidgetSelector::Id(value) => {
                            CheckedWidgetSelector::Id(target(value))
                        }
                        ResolvedWidgetSelector::Text(_) => CheckedWidgetSelector::Text,
                        ResolvedWidgetSelector::Point { .. } => CheckedWidgetSelector::Point,
                        ResolvedWidgetSelector::Focused => CheckedWidgetSelector::Focused,
                        ResolvedWidgetSelector::Extern { target: id, args } => {
                            CheckedWidgetSelector::Extern {
                                function: extern_name(program, *id, statement.origin)?,
                                arguments: args.len(),
                            }
                        }
                    },
                    all: *all,
                },
            })
        }
        ResolvedStatementKind::PaneOperation {
            grid, operation, ..
        } => HandlerOperationContract::Pane {
            grid: grid.clone(),
            operation: match operation {
                ResolvedPaneOperation::Maximize { pane: value } => {
                    CheckedPaneOperation::Maximize(pane(value))
                }
                ResolvedPaneOperation::Restore => CheckedPaneOperation::Restore,
                ResolvedPaneOperation::Maximized => CheckedPaneOperation::Maximized,
                ResolvedPaneOperation::Adjacent {
                    pane: value,
                    edge: value_edge,
                } => CheckedPaneOperation::Adjacent(pane(value), edge(*value_edge)),
                ResolvedPaneOperation::Swap { first, second } => {
                    CheckedPaneOperation::Swap(pane(first), pane(second))
                }
                ResolvedPaneOperation::Close { pane: value } => {
                    CheckedPaneOperation::Close(pane(value))
                }
                ResolvedPaneOperation::Move {
                    pane: value,
                    edge: value_edge,
                } => CheckedPaneOperation::Move(pane(value), edge(*value_edge)),
                ResolvedPaneOperation::Resize { split, .. } => {
                    CheckedPaneOperation::Resize(split.clone())
                }
                ResolvedPaneOperation::Drop {
                    pane: value,
                    target,
                    edge: value_edge,
                } => CheckedPaneOperation::Drop(pane(value), pane(target), value_edge.map(edge)),
                ResolvedPaneOperation::Split {
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
        ResolvedStatementKind::WindowOperation { operation, .. } => {
            HandlerOperationContract::Window(match operation {
                ResolvedWindowOperation::Open(index) => CheckedWindowOperation::Open(
                    index
                        .map(|index| {
                            program
                                .document
                                .settings
                                .windows
                                .get(index as usize)
                                .map(|window| window.name.clone())
                                .ok_or_else(|| {
                                    program.invariant_at_origin(
                                        statement.origin,
                                        "named window index is outside its arena",
                                    )
                                })
                        })
                        .transpose()?,
                ),
                ResolvedWindowOperation::Oldest => CheckedWindowOperation::Oldest,
                ResolvedWindowOperation::Latest => CheckedWindowOperation::Latest,
                ResolvedWindowOperation::Close => CheckedWindowOperation::Close,
                ResolvedWindowOperation::Drag => CheckedWindowOperation::Drag,
                ResolvedWindowOperation::DragResize(direction) => {
                    CheckedWindowOperation::DragResize(format!("{direction:?}"))
                }
                ResolvedWindowOperation::Resize(..) => CheckedWindowOperation::Resize,
                ResolvedWindowOperation::Resizable(_) => CheckedWindowOperation::Resizable,
                ResolvedWindowOperation::MinSize(value) => {
                    CheckedWindowOperation::MinSize(value.is_some())
                }
                ResolvedWindowOperation::MaxSize(value) => {
                    CheckedWindowOperation::MaxSize(value.is_some())
                }
                ResolvedWindowOperation::ResizeIncrements(value) => {
                    CheckedWindowOperation::ResizeIncrements(value.is_some())
                }
                ResolvedWindowOperation::Size => CheckedWindowOperation::Size,
                ResolvedWindowOperation::IsMaximized => CheckedWindowOperation::IsMaximized,
                ResolvedWindowOperation::Maximize(_) => CheckedWindowOperation::Maximize,
                ResolvedWindowOperation::IsMinimized => CheckedWindowOperation::IsMinimized,
                ResolvedWindowOperation::Minimize(_) => CheckedWindowOperation::Minimize,
                ResolvedWindowOperation::Position => CheckedWindowOperation::Position,
                ResolvedWindowOperation::ScaleFactor => CheckedWindowOperation::ScaleFactor,
                ResolvedWindowOperation::Move(..) => CheckedWindowOperation::Move,
                ResolvedWindowOperation::Mode => CheckedWindowOperation::Mode,
                ResolvedWindowOperation::SetMode(mode) => {
                    CheckedWindowOperation::SetMode(format!("{mode:?}"))
                }
                ResolvedWindowOperation::ToggleMaximize => CheckedWindowOperation::ToggleMaximize,
                ResolvedWindowOperation::ToggleDecorations => {
                    CheckedWindowOperation::ToggleDecorations
                }
                ResolvedWindowOperation::Attention(value) => {
                    CheckedWindowOperation::Attention(value.map(|value| format!("{value:?}")))
                }
                ResolvedWindowOperation::Focus => CheckedWindowOperation::Focus,
                ResolvedWindowOperation::SetLevel(level) => {
                    CheckedWindowOperation::SetLevel(format!("{level:?}"))
                }
                ResolvedWindowOperation::SystemMenu => CheckedWindowOperation::SystemMenu,
                ResolvedWindowOperation::RawId => CheckedWindowOperation::RawId,
                ResolvedWindowOperation::Screenshot => CheckedWindowOperation::Screenshot,
                ResolvedWindowOperation::MousePassthrough(_) => {
                    CheckedWindowOperation::MousePassthrough
                }
                ResolvedWindowOperation::MonitorSize => CheckedWindowOperation::MonitorSize,
                ResolvedWindowOperation::AutomaticTabbing(_) => {
                    CheckedWindowOperation::AutomaticTabbing
                }
                ResolvedWindowOperation::Icon { .. } => CheckedWindowOperation::Icon,
                ResolvedWindowOperation::Callback { target, args } => {
                    CheckedWindowOperation::Callback {
                        function: extern_name(program, *target, statement.origin)?,
                        arguments: args.len(),
                    }
                }
            })
        }
        _ => return Ok(None),
    }))
}

impl LoweredProgram {
    pub(crate) fn validate_handler_hir(&self) -> Result<(), Error> {
        fn validate_expression_use(
            program: &LoweredProgram,
            id: CheckedExprUseId,
            owner: crate::check::CheckedExprOwner,
            origin: OriginId,
        ) -> Result<(), Error> {
            let expression = program.facts.try_expression_use(id).ok_or_else(|| {
                program.invariant_at_origin(origin, "expression-use ID is outside its arena")
            })?;
            if expression.owner != owner {
                return Err(program.invariant_at_origin(
                    origin,
                    "expression-use ID belongs to a different HIR owner",
                ));
            }
            if program.facts.try_expression(expression.root).is_none() {
                return Err(
                    program.invariant_at_origin(origin, "expression root ID is outside its arena")
                );
            }
            Ok(())
        }

        fn validate_route(
            program: &LoweredProgram,
            route: &ResolvedRoute,
            statement: &ResolvedStatement,
        ) -> Result<(), Error> {
            let declaration = program.declarations.try_route(route.id).ok_or_else(|| {
                program.invariant_at_origin(route.origin, "route ID is outside its arena")
            })?;
            if declaration.statement != statement.id || declaration.task != statement.task {
                return Err(program.invariant_at_origin(
                    route.origin,
                    "route ID belongs to a different statement or task",
                ));
            }
            let checked = program.facts.try_route(route.id).ok_or_else(|| {
                program.invariant_at_origin(route.origin, "route has no checked HIR contract")
            })?;
            let (handler, owner, name) = match &route.target {
                ResolvedRouteTarget::App { handler, name } => {
                    (*handler, HandlerOwner::App, name.as_str())
                }
                ResolvedRouteTarget::Component {
                    component,
                    handler,
                    name,
                } => (*handler, HandlerOwner::Component(*component), name.as_str()),
            };
            let target = program.try_handler(handler).ok_or_else(|| {
                program.invariant_at_origin(
                    route.origin,
                    "route target handler ID is outside its arena",
                )
            })?;
            if target.owner != owner || target.name != name {
                return Err(program.invariant_at_origin(
                    route.origin,
                    "route target ID belongs to a different handler",
                ));
            }
            if checked.id != route.id
                || checked.origin != route.origin
                || declaration.declaration.origin != route.origin
                || checked.target != handler
                || checked.target_owner != owner
                || checked.args.len() != route.args.len()
                || target.params.len() != route.args.len()
            {
                return Err(program.invariant_at_origin(
                    route.origin,
                    "route target or argument cardinality diverged from its checked contract",
                ));
            }
            let mut payload = 0usize;
            for (argument, ((resolved, checked_kind), param)) in route
                .args
                .iter()
                .zip(&checked.args)
                .zip(&target.params)
                .enumerate()
            {
                match (resolved, checked_kind) {
                    (
                        ResolvedRouteArg::Expression(expression),
                        crate::check::CheckedRouteArgKind::Expression,
                    ) => {
                        validate_expression_use(
                            program,
                            *expression,
                            crate::check::CheckedExprOwner::Route {
                                route: route.id,
                                argument: argument as u32,
                            },
                            route.origin,
                        )?;
                        let expression = program.facts.expression_use(*expression);
                        if expression.destination != param.ty {
                            return Err(program.invariant_at_origin(
                                route.origin,
                                "route expression type diverged from its target parameter",
                            ));
                        }
                    }
                    (
                        ResolvedRouteArg::Payload { index, ty },
                        crate::check::CheckedRouteArgKind::Payload,
                    ) => {
                        let expected = if checked.ordered_payloads { payload } else { 0 };
                        let source = checked.source_payloads.get(expected).ok_or_else(|| {
                            program.invariant_at_origin(
                                route.origin,
                                "route payload index is outside its checked source contract",
                            )
                        })?;
                        if *index as usize != expected || source != ty || param.ty != *ty {
                            return Err(program.invariant_at_origin(
                                route.origin,
                                "route payload topology or type diverged from its checked contract",
                            ));
                        }
                        payload += 1;
                    }
                    _ => {
                        return Err(program.invariant_at_origin(
                            route.origin,
                            "route argument kind diverged from its checked contract",
                        ));
                    }
                }
            }
            Ok(())
        }

        fn validate_task(
            program: &LoweredProgram,
            task: TaskId,
            statement: &ResolvedStatement,
        ) -> Result<(), Error> {
            let declaration = program.declarations.try_task(task).ok_or_else(|| {
                program.invariant_at_origin(statement.origin, "task ID is outside its arena")
            })?;
            if declaration.statement != statement.id {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "task ID belongs to a different statement",
                ));
            }
            let checked = program.facts.try_task(task).ok_or_else(|| {
                program.invariant_at_origin(statement.origin, "task has no checked HIR contract")
            })?;
            if checked.id != task {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "task ID diverged from its checked HIR contract",
                ));
            }
            Ok(())
        }

        fn validate_task_operands(
            program: &LoweredProgram,
            task: TaskId,
            operands: &[CheckedExprUseId],
            origin: OriginId,
        ) -> Result<(), Error> {
            for (operand, expression) in operands.iter().enumerate() {
                validate_expression_use(
                    program,
                    *expression,
                    crate::check::CheckedExprOwner::Task {
                        task,
                        operand: operand as u32,
                    },
                    origin,
                )?;
            }
            if program
                .facts
                .expression_use_by_owner(crate::check::CheckedExprOwner::Task {
                    task,
                    operand: operands.len() as u32,
                })
                .is_some()
            {
                return Err(program.invariant_at_origin(
                    origin,
                    "task operand cardinality diverged from its checked HIR contract",
                ));
            }
            Ok(())
        }

        fn validate_effect(
            program: &LoweredProgram,
            target: &ResolvedEffectTarget,
            origin: OriginId,
        ) -> Result<(), Error> {
            if let ResolvedEffectTarget::Extern(id) = target
                && program.declarations.try_extern_decl(*id).is_none()
            {
                return Err(program
                    .invariant_at_origin(origin, "effect extern target ID is outside its arena"));
            }
            Ok(())
        }

        fn validate_effect_task(
            program: &LoweredProgram,
            task: TaskId,
            kind: EffectKind,
            target: &ResolvedEffectTarget,
            origin: OriginId,
        ) -> Result<(), Error> {
            validate_effect(program, target, origin)?;
            let checked = program.facts.try_task(task).ok_or_else(|| {
                program.invariant_at_origin(origin, "effect task has no checked HIR contract")
            })?;
            let matches = match (&checked.target, target) {
                (
                    Some(crate::check::CheckedEffectTarget::Builtin(checked)),
                    ResolvedEffectTarget::Builtin(resolved),
                ) => checked == resolved,
                (
                    Some(crate::check::CheckedEffectTarget::Extern(checked)),
                    ResolvedEffectTarget::Extern(resolved),
                ) => checked == resolved,
                _ => false,
            };
            if !matches {
                return Err(program.invariant_at_origin(
                    origin,
                    "effect target diverged from its checked task contract",
                ));
            }
            let kind_matches = match target {
                ResolvedEffectTarget::Builtin(_) => kind == EffectKind::Task,
                ResolvedEffectTarget::Extern(id) => program
                    .declarations
                    .try_extern_decl(*id)
                    .is_some_and(|declaration| declaration.kind == ExternKind::from(kind)),
            };
            if !kind_matches {
                return Err(program.invariant_at_origin(
                    origin,
                    "effect kind diverged from its checked target contract",
                ));
            }
            Ok(())
        }

        fn validate_task_source(
            program: &LoweredProgram,
            source: &ResolvedTaskSource,
            statement: &ResolvedStatement,
        ) -> Result<TaskId, Error> {
            let (task, operands) = match source {
                ResolvedTaskSource::Effect {
                    task,
                    kind,
                    target,
                    args,
                } => {
                    validate_effect_task(program, *task, *kind, target, statement.origin)?;
                    (*task, args.as_slice())
                }
                ResolvedTaskSource::Done { task, value } => (*task, ::std::slice::from_ref(value)),
                ResolvedTaskSource::None { task, .. } => (*task, &[] as &[CheckedExprUseId]),
            };
            validate_task(program, task, statement)?;
            validate_task_operands(program, task, operands, statement.origin)?;
            Ok(task)
        }

        fn validate_widget_operation(
            program: &LoweredProgram,
            operation: &ResolvedWidgetOperation,
            origin: OriginId,
        ) -> Result<(), Error> {
            if let ResolvedWidgetOperation::Find {
                selector: ResolvedWidgetSelector::Extern { target, .. },
                ..
            } = operation
                && program.declarations.try_extern_decl(*target).is_none()
            {
                return Err(program.invariant_at_origin(
                    origin,
                    "widget selector extern target ID is outside its arena",
                ));
            }
            Ok(())
        }

        fn validate_window_operation(
            program: &LoweredProgram,
            operation: &ResolvedWindowOperation,
            origin: OriginId,
        ) -> Result<(), Error> {
            if let ResolvedWindowOperation::Callback { target, .. } = operation
                && program.declarations.try_extern_decl(*target).is_none()
            {
                return Err(program.invariant_at_origin(
                    origin,
                    "window callback extern target ID is outside its arena",
                ));
            }
            Ok(())
        }

        fn widget_target_operands(
            target: &ResolvedWidgetTarget,
            operands: &mut Vec<CheckedExprUseId>,
        ) {
            operands.extend(target.segments.iter().filter_map(|segment| segment.key));
        }

        fn pane_reference_operands(
            reference: &ResolvedPaneReference,
            operands: &mut Vec<CheckedExprUseId>,
        ) {
            if let ResolvedPaneReference::Dynamic { key, .. } = reference {
                operands.push(*key);
            }
        }

        fn statement_operands(statement: &ResolvedStatement) -> Vec<CheckedExprUseId> {
            let mut operands = Vec::new();
            match &statement.kind {
                ResolvedStatementKind::Let { value, .. }
                | ResolvedStatementKind::MarkdownAppend { value, .. }
                | ResolvedStatementKind::ComboPush { value, .. }
                | ResolvedStatementKind::ClipboardWrite { value, .. } => operands.push(*value),
                ResolvedStatementKind::Assign { value, at, .. } => {
                    operands.push(*value);
                    operands.extend(at);
                }
                ResolvedStatementKind::ReturnIf { condition } => operands.push(*condition),
                ResolvedStatementKind::DebugStart { name, .. } => operands.push(*name),
                ResolvedStatementKind::WidgetOperation { operation, .. } => match operation {
                    ResolvedWidgetOperation::FocusPrevious | ResolvedWidgetOperation::FocusNext => {
                    }
                    ResolvedWidgetOperation::Focus { target }
                    | ResolvedWidgetOperation::Focused { target }
                    | ResolvedWidgetOperation::CursorFront { target }
                    | ResolvedWidgetOperation::CursorEnd { target }
                    | ResolvedWidgetOperation::SelectAll { target }
                    | ResolvedWidgetOperation::SnapEnd { target } => {
                        widget_target_operands(target, &mut operands);
                    }
                    ResolvedWidgetOperation::Cursor { target, position } => {
                        widget_target_operands(target, &mut operands);
                        operands.push(*position);
                    }
                    ResolvedWidgetOperation::Select { target, start, end } => {
                        widget_target_operands(target, &mut operands);
                        operands.extend([*start, *end]);
                    }
                    ResolvedWidgetOperation::Snap { target, x, y }
                    | ResolvedWidgetOperation::ScrollTo { target, x, y }
                    | ResolvedWidgetOperation::ScrollBy { target, x, y } => {
                        widget_target_operands(target, &mut operands);
                        operands.extend([*x, *y]);
                    }
                    ResolvedWidgetOperation::Find { selector, .. } => match selector {
                        ResolvedWidgetSelector::Id(target) => {
                            widget_target_operands(target, &mut operands);
                        }
                        ResolvedWidgetSelector::Text(value) => operands.push(*value),
                        ResolvedWidgetSelector::Point { x, y } => operands.extend([*x, *y]),
                        ResolvedWidgetSelector::Focused => {}
                        ResolvedWidgetSelector::Extern { args, .. } => {
                            operands.extend(args.iter().copied());
                        }
                    },
                },
                ResolvedStatementKind::PaneOperation { operation, .. } => match operation {
                    ResolvedPaneOperation::Restore | ResolvedPaneOperation::Maximized => {}
                    ResolvedPaneOperation::Maximize { pane }
                    | ResolvedPaneOperation::Adjacent { pane, .. }
                    | ResolvedPaneOperation::Close { pane }
                    | ResolvedPaneOperation::Move { pane, .. } => {
                        pane_reference_operands(pane, &mut operands);
                    }
                    ResolvedPaneOperation::Swap { first, second } => {
                        pane_reference_operands(first, &mut operands);
                        pane_reference_operands(second, &mut operands);
                    }
                    ResolvedPaneOperation::Resize { ratio, .. } => operands.push(*ratio),
                    ResolvedPaneOperation::Drop { pane, target, .. } => {
                        pane_reference_operands(pane, &mut operands);
                        pane_reference_operands(target, &mut operands);
                    }
                    ResolvedPaneOperation::Split {
                        target,
                        pane,
                        ratio,
                        ..
                    } => {
                        pane_reference_operands(target, &mut operands);
                        pane_reference_operands(pane, &mut operands);
                        operands.push(*ratio);
                    }
                },
                ResolvedStatementKind::WindowOperation {
                    operation, target, ..
                } => {
                    operands.extend(target);
                    match operation {
                        ResolvedWindowOperation::Resize(width, height)
                        | ResolvedWindowOperation::Move(width, height) => {
                            operands.extend([*width, *height]);
                        }
                        ResolvedWindowOperation::Resizable(value)
                        | ResolvedWindowOperation::Maximize(value)
                        | ResolvedWindowOperation::Minimize(value)
                        | ResolvedWindowOperation::MousePassthrough(value)
                        | ResolvedWindowOperation::AutomaticTabbing(value) => {
                            operands.push(*value);
                        }
                        ResolvedWindowOperation::MinSize(size)
                        | ResolvedWindowOperation::MaxSize(size)
                        | ResolvedWindowOperation::ResizeIncrements(size) => {
                            if let Some((width, height)) = size {
                                operands.extend([*width, *height]);
                            }
                        }
                        ResolvedWindowOperation::Icon {
                            pixels,
                            width,
                            height,
                        } => operands.extend([*pixels, *width, *height]),
                        ResolvedWindowOperation::Callback { args, .. } => {
                            operands.extend(args.iter().copied());
                        }
                        ResolvedWindowOperation::Open(_)
                        | ResolvedWindowOperation::Oldest
                        | ResolvedWindowOperation::Latest
                        | ResolvedWindowOperation::Close
                        | ResolvedWindowOperation::Drag
                        | ResolvedWindowOperation::DragResize(_)
                        | ResolvedWindowOperation::Size
                        | ResolvedWindowOperation::IsMaximized
                        | ResolvedWindowOperation::IsMinimized
                        | ResolvedWindowOperation::Position
                        | ResolvedWindowOperation::ScaleFactor
                        | ResolvedWindowOperation::Mode
                        | ResolvedWindowOperation::SetMode(_)
                        | ResolvedWindowOperation::ToggleMaximize
                        | ResolvedWindowOperation::ToggleDecorations
                        | ResolvedWindowOperation::Attention(_)
                        | ResolvedWindowOperation::Focus
                        | ResolvedWindowOperation::SetLevel(_)
                        | ResolvedWindowOperation::SystemMenu
                        | ResolvedWindowOperation::RawId
                        | ResolvedWindowOperation::Screenshot
                        | ResolvedWindowOperation::MonitorSize => {}
                    }
                }
                ResolvedStatementKind::Exit
                | ResolvedStatementKind::Run(_)
                | ResolvedStatementKind::Sip(_)
                | ResolvedStatementKind::TaskFlow(_)
                | ResolvedStatementKind::TaskGroup { .. }
                | ResolvedStatementKind::Abortable { .. }
                | ResolvedStatementKind::Abort { .. }
                | ResolvedStatementKind::DebugFinish { .. } => {}
            }
            operands
        }

        fn validate_statement_operands(
            program: &LoweredProgram,
            statement: &ResolvedStatement,
        ) -> Result<(), Error> {
            let checked = program.facts.try_statement(statement.id).ok_or_else(|| {
                program
                    .invariant_at_origin(statement.origin, "statement has no checked HIR contract")
            })?;
            let operands = statement_operands(statement);
            if checked.id != statement.id || checked.operand_count as usize != operands.len() {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "statement operand cardinality diverged from its checked HIR contract",
                ));
            }
            for (operand, expression) in operands.iter().enumerate() {
                validate_expression_use(
                    program,
                    *expression,
                    crate::check::CheckedExprOwner::HandlerStatement {
                        statement: statement.id,
                        operand: operand as u32,
                    },
                    statement.origin,
                )?;
            }
            Ok(())
        }

        fn statement_routes(statement: &ResolvedStatement) -> Vec<&ResolvedRoute> {
            match &statement.kind {
                ResolvedStatementKind::Run(run) => std::iter::once(&run.success)
                    .chain(run.error.iter())
                    .collect(),
                ResolvedStatementKind::Sip(sip) => std::iter::once(&sip.progress)
                    .chain(std::iter::once(&sip.success))
                    .chain(sip.error.iter())
                    .collect(),
                ResolvedStatementKind::TaskFlow(flow) => flow
                    .success
                    .iter()
                    .chain(flow.error.iter())
                    .chain(flow.units.iter())
                    .collect(),
                ResolvedStatementKind::WidgetOperation { route, .. }
                | ResolvedStatementKind::PaneOperation { route, .. }
                | ResolvedStatementKind::WindowOperation { route, .. } => route.iter().collect(),
                _ => Vec::new(),
            }
        }

        fn statement_writable_targets(
            statement: &ResolvedStatement,
        ) -> Vec<&ResolvedWritableState> {
            match &statement.kind {
                ResolvedStatementKind::Assign { target, .. }
                | ResolvedStatementKind::MarkdownAppend { target, .. }
                | ResolvedStatementKind::ComboPush { target, .. }
                | ResolvedStatementKind::DebugStart { target, .. }
                | ResolvedStatementKind::DebugFinish { target }
                | ResolvedStatementKind::Abort { handle: target }
                | ResolvedStatementKind::Abortable { handle: target, .. } => vec![target],
                _ => Vec::new(),
            }
        }

        fn validate_writable_targets(
            program: &LoweredProgram,
            statement: &ResolvedStatement,
            checked: &crate::check::CheckedStatement,
        ) -> Result<(), Error> {
            let resolved = statement_writable_targets(statement);
            if resolved.len() != checked.writable_targets.len() {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "statement writable target cardinality diverged from checked HIR",
                ));
            }
            for (target, expected) in resolved.iter().zip(&checked.writable_targets) {
                let value = program.facts.try_value_by_ref(*expected).ok_or_else(|| {
                    program.invariant_at_origin(
                        statement.origin,
                        "checked writable target ID is outside its arena",
                    )
                })?;
                if target.value != *expected || target.name != value.name || target.ty != value.ty {
                    return Err(program.invariant_at_origin(
                        statement.origin,
                        "statement writable target identity or type diverged from checked HIR",
                    ));
                }
            }
            Ok(())
        }

        fn validate_transform_local(
            program: &LoweredProgram,
            transform: &ResolvedTaskTransform,
            index: usize,
            statement: &ResolvedStatement,
        ) -> Result<(), Error> {
            let (task, local, binding, input, input_fallible) = match transform {
                ResolvedTaskTransform::Map {
                    task,
                    local,
                    binding,
                    input,
                    input_fallible,
                    ..
                } => (*task, *local, binding, input, Some(*input_fallible)),
                ResolvedTaskTransform::Then {
                    task,
                    local,
                    binding,
                    input,
                    ..
                }
                | ResolvedTaskTransform::AndThen {
                    task,
                    local,
                    binding,
                    input,
                    ..
                }
                | ResolvedTaskTransform::MapError {
                    task,
                    local,
                    binding,
                    input,
                    ..
                } => (*task, *local, binding, input, None),
                ResolvedTaskTransform::Collect { .. } | ResolvedTaskTransform::Discard { .. } => {
                    return Ok(());
                }
            };
            let expected =
                program
                    .facts
                    .local_by_owner(crate::check::CheckedLocalOwner::TaskTransform {
                        task,
                        index: index as u32,
                    });
            let local_fact = program.facts.try_local(local).ok_or_else(|| {
                program.invariant_at_origin(
                    statement.origin,
                    "task transform local ID is outside its arena",
                )
            })?;
            if expected != Some(local)
                || local_fact.name != *binding
                || local_fact.ty != *input
                || local_fact.owner
                    != (crate::check::CheckedLocalOwner::TaskTransform {
                        task,
                        index: index as u32,
                    })
            {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "task transform local identity, binding, or type diverged from checked HIR",
                ));
            }
            if let Some(input_fallible) = input_fallible {
                let expected = program
                    .facts
                    .try_task(task)
                    .is_some_and(|task| task.error.is_some());
                if input_fallible != expected {
                    return Err(program.invariant_at_origin(
                        statement.origin,
                        "task transform fallibility diverged from checked HIR",
                    ));
                }
            }
            Ok(())
        }

        fn visit(
            program: &LoweredProgram,
            handler: &ResolvedHandler,
            statement: &ResolvedStatement,
            parent: Option<StatementId>,
        ) -> Result<(), Error> {
            let declaration = program
                .declarations
                .try_statement(statement.id)
                .ok_or_else(|| {
                    program
                        .invariant_at_origin(statement.origin, "statement ID is outside its arena")
                })?;
            if declaration.handler != handler.id
                || declaration.parent != parent
                || declaration.task != statement.task
                || declaration.is_final != statement.is_final
                || declaration.declaration.origin != statement.origin
            {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "statement owner, parent, task ID, finality, or origin diverged from its declaration",
                ));
            }
            if let Some(task) = statement.task {
                validate_task(program, task, statement)?;
            }
            validate_statement_operands(program, statement)?;
            let routes = statement_routes(statement);
            for route in &routes {
                validate_route(program, route, statement)?;
            }
            if routes.iter().map(|route| route.id).collect::<Vec<_>>() != declaration.routes {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "statement route cardinality or order diverged from its declaration",
                ));
            }
            let children = match &statement.kind {
                ResolvedStatementKind::TaskGroup { statements, .. } => {
                    statements.iter().map(|child| child.id).collect::<Vec<_>>()
                }
                ResolvedStatementKind::Abortable { task, .. } => vec![task.id],
                _ => Vec::new(),
            };
            if children != declaration.children {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "statement child cardinality or order diverged from its declaration",
                ));
            }
            match &statement.kind {
                ResolvedStatementKind::Run(run) => {
                    let task = statement.task.ok_or_else(|| {
                        program.invariant_at_origin(
                            statement.origin,
                            "run statement has no normalized task ID",
                        )
                    })?;
                    validate_effect_task(program, task, run.kind, &run.target, statement.origin)?;
                    validate_task_operands(program, task, &run.args, statement.origin)?;
                    if (run.mode == FutureMode::Every) != run.site.is_none()
                        || run.site != declaration.run_site
                    {
                        return Err(program.invariant_at_origin(
                            statement.origin,
                            "run mode and stable run-site cardinality diverged",
                        ));
                    }
                    if let Some(site) = run.site {
                        let run_site =
                            program.declarations.try_run_site(site).ok_or_else(|| {
                                program.invariant_at_origin(
                                    statement.origin,
                                    "run-site ID is outside its arena",
                                )
                            })?;
                        if run_site.statement != statement.id || run_site.mode != run.mode {
                            return Err(program.invariant_at_origin(
                                statement.origin,
                                "run-site ID belongs to a different statement or mode",
                            ));
                        }
                    }
                }
                ResolvedStatementKind::Sip(sip) => {
                    let task = statement.task.ok_or_else(|| {
                        program.invariant_at_origin(
                            statement.origin,
                            "sip statement has no normalized task ID",
                        )
                    })?;
                    let checked_task = program.facts.try_task(task).ok_or_else(|| {
                        program.invariant_at_origin(
                            statement.origin,
                            "sip task has no checked HIR contract",
                        )
                    })?;
                    if !program
                        .declarations
                        .try_extern_decl(sip.target)
                        .is_some_and(|declaration| declaration.kind == ExternKind::Sip)
                        || checked_task.target
                            != Some(crate::check::CheckedEffectTarget::Extern(sip.target))
                    {
                        return Err(program.invariant_at_origin(
                            statement.origin,
                            "sip extern target diverged from its checked task contract",
                        ));
                    }
                    validate_task_operands(program, task, &sip.args, statement.origin)?;
                }
                ResolvedStatementKind::TaskFlow(flow) => {
                    let root_task = statement.task.ok_or_else(|| {
                        program.invariant_at_origin(
                            statement.origin,
                            "task-flow statement has no normalized root task ID",
                        )
                    })?;
                    let checked_root = program.facts.try_task(root_task).ok_or_else(|| {
                        program.invariant_at_origin(
                            statement.origin,
                            "task-flow root task has no checked HIR contract",
                        )
                    })?;
                    if flow.output != checked_root.output || flow.error_type != checked_root.error {
                        return Err(program.invariant_at_origin(
                            statement.origin,
                            "task-flow output or error type diverged from checked HIR",
                        ));
                    }
                    let mut source_tasks =
                        vec![validate_task_source(program, &flow.source, statement)?];
                    for (index, transform) in flow.transforms.iter().enumerate() {
                        validate_transform_local(program, transform, index, statement)?;
                        let task = match transform {
                            ResolvedTaskTransform::Map { task, value, .. }
                            | ResolvedTaskTransform::MapError { task, value, .. } => {
                                validate_task(program, *task, statement)?;
                                validate_task_operands(
                                    program,
                                    *task,
                                    ::std::slice::from_ref(value),
                                    statement.origin,
                                )?;
                                *task
                            }
                            ResolvedTaskTransform::Then { source, .. }
                            | ResolvedTaskTransform::AndThen { source, .. } => {
                                validate_task_source(program, source, statement)?
                            }
                            ResolvedTaskTransform::Collect { task }
                            | ResolvedTaskTransform::Discard { task } => {
                                validate_task(program, *task, statement)?;
                                validate_task_operands(program, *task, &[], statement.origin)?;
                                *task
                            }
                        };
                        source_tasks.push(task);
                    }
                    if source_tasks != declaration.source_tasks {
                        return Err(program.invariant_at_origin(
                            statement.origin,
                            "task-flow source task cardinality or order diverged",
                        ));
                    }
                }
                ResolvedStatementKind::TaskGroup { statements, .. } => {
                    for child in statements {
                        visit(program, handler, child, Some(statement.id))?;
                    }
                }
                ResolvedStatementKind::Abortable { task, .. } => {
                    visit(program, handler, task, Some(statement.id))?;
                }
                ResolvedStatementKind::WidgetOperation { operation, .. } => {
                    validate_widget_operation(program, operation, statement.origin)?;
                }
                ResolvedStatementKind::PaneOperation { .. } => {}
                ResolvedStatementKind::WindowOperation { operation, .. } => {
                    validate_window_operation(program, operation, statement.origin)?;
                }
                _ => {}
            }
            let checked = program.facts.statement(statement.id);
            validate_writable_targets(program, statement, checked)?;
            match &statement.kind {
                ResolvedStatementKind::Let {
                    local, name, ty, ..
                } => {
                    let expected = program.facts.local_by_owner(
                        crate::check::CheckedLocalOwner::StatementLet(statement.id),
                    );
                    let local_fact = program.facts.try_local(*local).ok_or_else(|| {
                        program.invariant_at_origin(
                            statement.origin,
                            "let local ID is outside its arena",
                        )
                    })?;
                    if expected != Some(*local)
                        || local_fact.name != *name
                        || local_fact.ty != *ty
                        || local_fact.owner
                            != crate::check::CheckedLocalOwner::StatementLet(statement.id)
                    {
                        return Err(program.invariant_at_origin(
                            statement.origin,
                            "let local identity, name, or type diverged from checked HIR",
                        ));
                    }
                }
                ResolvedStatementKind::Assign { move_self, .. }
                    if checked.editor_self_move != Some(*move_self) =>
                {
                    return Err(program.invariant_at_origin(
                        statement.origin,
                        "editor self-move mode diverged from checked HIR",
                    ));
                }
                ResolvedStatementKind::PaneOperation { dynamic, .. }
                    if checked.pane_grid_dynamic != Some(*dynamic) =>
                {
                    return Err(program.invariant_at_origin(
                        statement.origin,
                        "pane grid mode diverged from checked HIR",
                    ));
                }
                _ => {}
            }
            if checked.semantic_key != resolved_statement_semantic_key(program, statement)?
                || checked.operation != resolved_handler_operation_contract(program, statement)?
            {
                return Err(program.invariant_at_origin(
                    statement.origin,
                    "statement semantics or operation contract diverged from checked HIR",
                ));
            }
            Ok(())
        }

        if let Err((origin, message)) = self.facts.validate_expression_arena() {
            return Err(self.invariant_at_origin(origin, message));
        }
        if let Err((origin, message)) =
            validate_expression_declaration_references(&self.facts, &self.declarations)
        {
            return Err(self.invariant_at_origin(origin, message));
        }

        if self.handlers.len() != self.declarations.handlers().len() {
            return Err(self.invariant_at_origin(
                OriginId(u32::MAX),
                "handler arena cardinality diverged from its declarations",
            ));
        }
        if let Some((_, handler)) = self
            .handlers
            .iter()
            .enumerate()
            .find(|(index, handler)| handler.id != HandlerId(*index as u32))
        {
            return Err(self.invariant_at_origin(
                handler.origin,
                "handler arena order diverged from its declarations",
            ));
        }

        for handler in &self.handlers {
            let declaration = self.declarations.try_handler(handler.id).ok_or_else(|| {
                self.invariant_at_origin(handler.origin, "handler ID is outside its arena")
            })?;
            if declaration.owner != handler.owner || declaration.name != handler.name {
                return Err(self.invariant_at_origin(
                    handler.origin,
                    "handler identity diverged from its declaration",
                ));
            }
            let checked = self.facts.try_handler(handler.id).ok_or_else(|| {
                self.invariant_at_origin(handler.origin, "handler has no checked HIR contract")
            })?;
            if checked.id != handler.id
                || checked.origin != handler.origin
                || checked.params.len() != handler.params.len()
                || checked.param_names.len() != handler.params.len()
                || checked.param_types.len() != handler.params.len()
            {
                return Err(self.invariant_at_origin(
                    handler.origin,
                    "handler parameter cardinality or origin diverged from its checked contract",
                ));
            }
            for (index, param) in handler.params.iter().enumerate() {
                let local = self.facts.try_local(param.local).ok_or_else(|| {
                    self.invariant_at_origin(
                        handler.origin,
                        "handler parameter local ID is outside its arena",
                    )
                })?;
                if checked.params[index] != param.local
                    || checked.param_names[index] != param.name
                    || checked.param_types[index] != param.ty
                    || local.name != param.name
                    || local.ty != param.ty
                    || local.owner
                        != (crate::check::CheckedLocalOwner::HandlerParam {
                            handler: handler.id,
                            index: index as u32,
                        })
                {
                    return Err(self.invariant_at_origin(
                        handler.origin,
                        "handler parameter identity, type, or local owner diverged",
                    ));
                }
            }
            for statement in &handler.statements {
                visit(self, handler, statement, None)?;
            }
            if declaration.statement_roots
                != handler
                    .statements
                    .iter()
                    .map(|statement| statement.id)
                    .collect::<Vec<_>>()
            {
                return Err(self.invariant_at_origin(
                    handler.origin,
                    "handler statement roots diverged from its declaration",
                ));
            }
        }

        let mut expected_app = Vec::new();
        let mut expected_presets = Vec::new();
        let mut expected_components = vec![Vec::new(); self.components.len()];
        for handler in self.declarations.handlers() {
            match handler.owner {
                HandlerOwner::App => expected_app.push(handler.declaration.id),
                HandlerOwner::Preset(_) => expected_presets.push(handler.declaration.id),
                HandlerOwner::Component(component) => {
                    let Some(partition) = expected_components.get_mut(component.0 as usize) else {
                        return Err(self.invariant_at_origin(
                            handler.declaration.origin,
                            "component handler owner ID is outside its contract arena",
                        ));
                    };
                    partition.push(handler.declaration.id);
                }
            }
        }
        if self.app_handlers != expected_app {
            return Err(self.invariant_at_origin(
                OriginId(u32::MAX),
                "app handler index diverged from its declaration partition",
            ));
        }
        if self.preset_handlers != expected_presets {
            return Err(self.invariant_at_origin(
                OriginId(u32::MAX),
                "preset handler index diverged from its declaration partition",
            ));
        }
        for (index, (component, expected)) in
            self.components.iter().zip(expected_components).enumerate()
        {
            if component.id != ComponentId(index as u32) || component.handlers != expected {
                return Err(self.invariant_at_origin(
                    component.origin,
                    "component identity or handler index diverged from its declaration partition",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn document(&self) -> &Document {
        &self.document
    }

    pub(crate) fn app_name(&self) -> &str {
        &self.settings.app_name
    }

    pub(crate) fn extern_structs(&self) -> &[ExternStruct] {
        &self.document.structs
    }

    #[allow(dead_code)]
    pub(crate) fn checked_facts(&self) -> &CheckedFacts {
        &self.facts
    }

    pub(crate) fn subscriptions(&self) -> &[ResolvedSubscription] {
        &self.subscriptions
    }

    pub(crate) fn named_type_rust_path(&self, id: NamedTypeId) -> Option<&str> {
        self.named_type_rust_paths.get(&id).map(String::as_str)
    }

    pub(crate) fn declarations(&self) -> &DeclarationIndex {
        &self.declarations
    }

    pub(crate) fn settings(&self) -> &ResolvedAppSettings {
        &self.settings
    }

    pub(crate) fn app_states(&self) -> &[AppStateContract] {
        &self.app_states
    }

    pub(crate) fn derived(&self) -> &[DerivedContract] {
        &self.derived
    }

    pub(crate) fn components(&self) -> &[ComponentContract] {
        &self.components
    }

    pub(crate) fn component(&self, id: ComponentId) -> &ComponentContract {
        &self.components[id.0 as usize]
    }

    pub(crate) fn try_component(&self, id: ComponentId) -> Option<&ComponentContract> {
        self.components.get(id.0 as usize)
    }

    #[allow(dead_code)]
    pub(crate) fn handlers(&self) -> &[ResolvedHandler] {
        &self.handlers
    }

    pub(crate) fn handler(&self, id: HandlerId) -> &ResolvedHandler {
        &self.handlers[id.0 as usize]
    }

    pub(crate) fn try_handler(&self, id: HandlerId) -> Option<&ResolvedHandler> {
        self.handlers.get(id.0 as usize)
    }

    pub(crate) fn invariant_at_origin(
        &self,
        origin: OriginId,
        message: impl Into<String>,
    ) -> Error {
        let message = format!("lowering invariant failed: {}", message.into());
        let Some(origin) = self.origins.try_get(origin) else {
            return Error::new("E196", &Span::line(1), message);
        };
        let mut error = Error::new(
            "E196",
            &Span {
                line: origin.line,
                column: origin.column,
            },
            message,
        );
        if let Some(path) = &origin.path {
            error = error.at_path(path.display().to_string());
        }
        error
    }

    pub(crate) fn app_handlers(&self) -> impl Iterator<Item = &ResolvedHandler> {
        self.app_handlers.iter().map(|id| self.handler(*id))
    }

    pub(crate) fn preset_handlers(&self) -> impl Iterator<Item = &ResolvedHandler> {
        self.preset_handlers.iter().map(|id| self.handler(*id))
    }

    pub(crate) fn component_slot_name(&self, id: ComponentSlotId) -> Result<&str, Error> {
        self.components
            .get(id.component.0 as usize)
            .and_then(|component| component.slots.get(id.index as usize))
            .map(|slot| slot.name.as_str())
            .ok_or_else(|| {
                Error::new(
                    "E196",
                    &Span::line(1),
                    "checked slot expression references an invalid slot ID",
                )
            })
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

    pub(crate) fn checked_view(&self, span: &Span) -> Result<&crate::check::CheckedView, Error> {
        let id = self.declarations.view_id(span).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "view reached code generation without a shared view ID",
            )
        })?;
        let view = self.facts.view(id);
        if view.id != id {
            return Err(Error::new(
                "E196",
                span,
                "view reached code generation with a mismatched checked view ID",
            ));
        }
        Ok(view)
    }

    pub(crate) fn validate_checked_view(&self, node: &ViewNode) -> Result<(), Error> {
        let span = node.span();
        let view = self.checked_view(span)?;
        let id = view.id;
        if view.kind != crate::hir::view_kind(node) {
            return Err(Error::new(
                "E196",
                span,
                "raw view kind diverged from its checked topology",
            ));
        }
        let children = crate::hir::view_children(node)
            .into_iter()
            .map(|child| {
                self.declarations.view_id(child.span()).ok_or_else(|| {
                    Error::new(
                        "E196",
                        child.span(),
                        "raw view child has no shared checked ID",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if children != view.children {
            return Err(Error::new(
                "E196",
                span,
                "raw view children diverged from its checked topology",
            ));
        }
        for child in &children {
            if self.facts.view(*child).parent != Some(id) {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked view child has a mismatched parent",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn style_use(&self, span: &Span) -> Result<&ResolvedStyleUse, Error> {
        self.styles.style_use(span)
    }

    pub(crate) fn theme(&self) -> &ResolvedThemeProgram {
        &self.styles.theme
    }

    pub(crate) fn nested_theme(&self, span: &Span) -> Result<&ResolvedNestedTheme, Error> {
        self.styles.nested_theme(span)
    }

    pub(crate) fn extern_function(&self, id: ExternFnId) -> &crate::hir::ExternDeclaration {
        self.declarations.extern_decl(id)
    }

    #[allow(dead_code)]
    pub(crate) fn origin(&self, id: OriginId) -> &Origin {
        self.origins.get(id)
    }

    pub(crate) fn source_origin(&self, merged_line: usize) -> Option<(&Path, usize)> {
        self.origins.source_origin(merged_line)
    }
}

pub(crate) fn lower(checked: CheckedDocument) -> Result<LoweredProgram, Error> {
    Lowerer::new(checked).lower()
}

pub(crate) struct Lowerer {
    document: Document,
    facts: CheckedFacts,
    declarations: DeclarationIndex,
    origins: OriginArena,
    components: Vec<ComponentContract>,
    component_indexes: Vec<ComponentIndex>,
    component_ids: HashMap<String, ComponentId>,
    calls: Vec<ComponentCall>,
    calls_by_site: HashMap<CallSite, ComponentCallId>,
    styles: StyleProgramBuilder,
    handlers: Vec<ResolvedHandler>,
    app_handlers: Vec<HandlerId>,
    preset_handlers: Vec<HandlerId>,
}

#[derive(Default)]
struct CheckedExpressionGraph {
    visiting: HashSet<CheckedExprId>,
    validated: HashSet<(CheckedExprId, usize)>,
    node_owners: HashMap<CheckedExprId, CheckedExprUseId>,
    scopes: Vec<CheckedExpressionScope>,
}

struct CheckedExpressionScope {
    parent: Option<usize>,
    binding: Option<CheckedLocalId>,
}

impl CheckedExpressionGraph {
    fn root_scope(&mut self) -> usize {
        if self.scopes.is_empty() {
            self.scopes.push(CheckedExpressionScope {
                parent: None,
                binding: None,
            });
        }
        0
    }

    fn scoped_binding(&mut self, parent: usize, binding: CheckedLocalId) -> usize {
        let id = self.scopes.len();
        self.scopes.push(CheckedExpressionScope {
            parent: Some(parent),
            binding: Some(binding),
        });
        id
    }

    fn contains_binding(&self, mut scope: usize, binding: CheckedLocalId) -> bool {
        loop {
            let Some(current) = self.scopes.get(scope) else {
                return false;
            };
            if current.binding == Some(binding) {
                return true;
            }
            let Some(parent) = current.parent else {
                return false;
            };
            scope = parent;
        }
    }
}

trait CheckedExpressionOwnerPolicy {
    fn use_id(&self) -> CheckedExprUseId;
    fn span(&self) -> &Span;
    fn value_type(&self, value: CheckedValueRef) -> Result<Type, Error>;
    fn local_type(&self, local: CheckedLocalId) -> Result<Type, Error>;
    fn slot_type(&self, slot: ComponentSlotId) -> Result<Type, Error>;
    fn palette_type(&self, palette: PaletteId) -> Result<Type, Error>;
}

struct AppSettingExpressionPolicy<'a> {
    lowerer: &'a Lowerer,
    use_id: CheckedExprUseId,
    span: &'a Span,
}

impl CheckedExpressionOwnerPolicy for AppSettingExpressionPolicy<'_> {
    fn use_id(&self) -> CheckedExprUseId {
        self.use_id
    }

    fn span(&self) -> &Span {
        self.span
    }

    fn value_type(&self, value: CheckedValueRef) -> Result<Type, Error> {
        let checked = self.lowerer.facts.try_value_by_ref(value).ok_or_else(|| {
            self.lowerer
                .invariant(self.span, "app setting path references an invalid value ID")
        })?;
        match value {
            CheckedValueRef::AppState(_) | CheckedValueRef::Derived(_) => Ok(checked.ty.clone()),
            CheckedValueRef::ComponentParam(id) => {
                self.lowerer
                    .declarations
                    .try_component_param(id)
                    .ok_or_else(|| {
                        self.lowerer.invariant(
                            self.span,
                            "app setting path references an invalid component parameter ID",
                        )
                    })?;
                Err(self.lowerer.invariant(
                    self.span,
                    "app setting path cannot reference a component parameter",
                ))
            }
            CheckedValueRef::ComponentState(id) => {
                self.lowerer
                    .declarations
                    .try_component_state(id)
                    .ok_or_else(|| {
                        self.lowerer.invariant(
                            self.span,
                            "app setting path references an invalid component state ID",
                        )
                    })?;
                Err(self.lowerer.invariant(
                    self.span,
                    "app setting path cannot reference component state",
                ))
            }
        }
    }

    fn local_type(&self, id: CheckedLocalId) -> Result<Type, Error> {
        let local = self.lowerer.facts.try_local(id).ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                "app setting expression references an invalid local ID",
            )
        })?;
        match local.owner {
            CheckedLocalOwner::AppSettingDaemonWindow => {
                let callback_owns_local = self
                    .lowerer
                    .facts
                    .try_expression_use(self.use_id)
                    .is_some_and(|expression| {
                        matches!(
                            expression.owner,
                            CheckedExprOwner::AppSetting(
                                AppSettingExprId::Title
                                    | AppSettingExprId::Theme
                                    | AppSettingExprId::ThemeFactoryArgument(_)
                                    | AppSettingExprId::Palette
                                    | AppSettingExprId::ScaleFactor
                            )
                        )
                    });
                if local.ty != Type::WindowId
                    || local.name != "window"
                    || !callback_owns_local
                    || !self
                        .lowerer
                        .facts
                        .app_settings()
                        .is_some_and(|settings| settings.daemon)
                    || self.lowerer.facts.app_setting_daemon_window_local() != Some(id)
                {
                    return Err(self.lowerer.invariant(
                        self.span,
                        "app setting daemon-window local has inconsistent topology",
                    ));
                }
            }
            CheckedLocalOwner::ExpressionBinding { expression, .. }
                if expression == self.use_id => {}
            CheckedLocalOwner::ExpressionBinding { .. } => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "app setting expression binding belongs to another expression use",
                ));
            }
            CheckedLocalOwner::View { .. } => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "app setting expression cannot reference a view local",
                ));
            }
            CheckedLocalOwner::HandlerParam { .. }
            | CheckedLocalOwner::StatementLet(_)
            | CheckedLocalOwner::TaskTransform { .. } => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "app setting expression cannot reference a handler local",
                ));
            }
        }
        Ok(local.ty.clone())
    }

    fn slot_type(&self, slot: ComponentSlotId) -> Result<Type, Error> {
        self.lowerer
            .declarations
            .try_component_slot(slot)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    "app setting expression references an invalid slot ID",
                )
            })?;
        Err(self.lowerer.invariant(
            self.span,
            "app setting expression cannot reference a component slot",
        ))
    }

    fn palette_type(&self, id: PaletteId) -> Result<Type, Error> {
        self.lowerer.declarations.palette_name(id).ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                "app setting path references an invalid palette ID",
            )
        })?;
        let expression = self
            .lowerer
            .facts
            .try_expression_use(self.use_id)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    "app setting path has an invalid expression-use ID",
                )
            })?;
        match &expression.destination {
            Type::Palette(contract) => Ok(Type::Palette(contract.clone())),
            _ => Err(self.lowerer.invariant(
                self.span,
                "app setting palette path has no checked theme-contract type",
            )),
        }
    }
}

impl Lowerer {
    fn new(checked: CheckedDocument) -> Self {
        let CheckedDocument {
            document,
            facts,
            declarations,
            origins,
            ..
        } = checked;
        let component_ids = declarations.component_ids();
        Self {
            document,
            facts,
            declarations,
            origins,
            components: Vec::new(),
            component_indexes: Vec::new(),
            component_ids,
            calls: Vec::new(),
            calls_by_site: HashMap::new(),
            styles: StyleProgramBuilder::default(),
            handlers: Vec::new(),
            app_handlers: Vec::new(),
            preset_handlers: Vec::new(),
        }
    }

    fn lower(mut self) -> Result<LoweredProgram, Error> {
        if let Err((origin, message)) = self.facts.validate_expression_arena() {
            return Err(self.invariant_at_origin(origin, message));
        }
        if let Err((origin, message)) =
            validate_expression_declaration_references(&self.facts, &self.declarations)
        {
            return Err(self.invariant_at_origin(origin, message));
        }
        let settings = self.lower_app_settings()?;
        self.lower_style_program()?;
        let subscriptions = self.lower_subscriptions()?;
        let named_type_rust_paths = self.declarations.named_type_rust_paths();
        let app_states = self.lower_app_states()?;
        let derived = self.lower_derived()?;
        self.index_components()?;
        self.lower_handlers()?;
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
        let styles = self.styles.finish().ok_or_else(|| {
            Error::new(
                "E196",
                &Span::line(1),
                "style lowering completed without a normalized theme program",
            )
        })?;
        Ok(LoweredProgram {
            document: self.document,
            facts: self.facts,
            declarations: self.declarations,
            settings,
            subscriptions,
            named_type_rust_paths,
            app_states,
            derived,
            components: self.components,
            handlers: self.handlers,
            app_handlers: self.app_handlers,
            preset_handlers: self.preset_handlers,
            calls: self.calls,
            calls_by_site: self.calls_by_site,
            styles,
            origins: self.origins,
        })
    }

    fn lower_app_settings(&mut self) -> Result<ResolvedAppSettings, Error> {
        self.validate_app_setting_expression_shape()?;
        self.validate_app_setting_expression_graphs()?;
        let checked = self.facts.app_settings().cloned().ok_or_else(|| {
            self.invariant(
                &self.document.settings.span,
                "application settings are missing their authoritative checked snapshot",
            )
        })?;
        self.validate_checked_app_settings(&checked)?;
        let source = checked.source;
        let declaration = self.declarations.app_settings();
        let default_font = checked.default_font.map(|font| ResolvedDefaultFont {
            family: font.family,
            weight: font.weight,
            stretch: font.stretch,
            style: font.style,
            origin: self.push_origin(&font.span, Some(declaration.origin)),
        });
        let title = source
            .title
            .as_ref()
            .map(|source| {
                self.checked_app_setting_expression(AppSettingExprId::Title, &source.span)
            })
            .transpose()?;
        let background = source
            .background
            .as_ref()
            .map(|source| {
                self.checked_app_setting_expression(AppSettingExprId::Background, &source.span)
            })
            .transpose()?;
        let text_color = source
            .text_color
            .as_ref()
            .map(|source| {
                self.checked_app_setting_expression(AppSettingExprId::TextColor, &source.span)
            })
            .transpose()?;
        let scale_factor = source
            .scale_factor
            .as_ref()
            .map(|source| {
                self.checked_app_setting_expression(AppSettingExprId::ScaleFactor, &source.span)
            })
            .transpose()?;
        let primary_window = self.lower_window_settings(source.window.as_ref(), declaration.origin);
        let named_windows = source
            .windows
            .iter()
            .enumerate()
            .map(|(index, window)| {
                let origin = self.push_origin(&window.span, Some(declaration.origin));
                ResolvedNamedWindow {
                    id: NamedWindowId(index as u32),
                    name: window.name.clone(),
                    settings: self.lower_window_settings(Some(&window.settings), origin),
                    origin,
                }
            })
            .collect();
        let field_origins: HashMap<String, OriginId> = source
            .setting_spans
            .iter()
            .map(|(name, span)| {
                (
                    name.clone(),
                    self.push_origin(span, Some(declaration.origin)),
                )
            })
            .collect();
        Ok(ResolvedAppSettings {
            settings_id: declaration.id,
            app_name: checked.app_name,
            kind: if checked.daemon {
                ProgramKind::Daemon
            } else {
                ProgramKind::Application
            },
            callback_window: self.facts.app_setting_daemon_window_local(),
            title,
            background,
            text_color,
            id: source.id,
            executor: source
                .executor
                .map_or(ResolvedExecutorSelection::Default, |path| {
                    ResolvedExecutorSelection::Custom {
                        path,
                        origin: field_origins["executor"],
                    }
                }),
            renderer: source
                .renderer
                .map_or(ResolvedRendererSelection::Default, |path| {
                    ResolvedRendererSelection::Custom {
                        path,
                        origin: field_origins["renderer"],
                    }
                }),
            fonts: source
                .fonts
                .iter()
                .map(|font| ResolvedFontAsset {
                    path: font.path.clone(),
                    origin: self.push_origin(&font.span, Some(declaration.origin)),
                })
                .collect(),
            default_font,
            default_text_size: source.default_text_size,
            antialiasing: source.antialiasing,
            vsync: source.vsync,
            scale_factor,
            primary_window,
            named_windows,
            field_origins,
            origin: declaration.origin,
        })
    }

    fn validate_checked_app_settings(
        &self,
        checked: &crate::check::CheckedAppSettings,
    ) -> Result<(), Error> {
        let settings = &checked.source;
        if self.document.app != checked.app_name {
            return Err(self.invariant(
                &settings.span,
                "application identity changed after semantic analysis",
            ));
        }
        if self.document.daemon != checked.daemon {
            return Err(self.invariant(
                &settings.span,
                "application kind changed after semantic analysis",
            ));
        }
        let current = &self.document.settings;
        let static_fields_match = current.id == settings.id
            && current.executor == settings.executor
            && current.renderer == settings.renderer
            && current.fonts == settings.fonts
            && current.default_text_size == settings.default_text_size
            && current.antialiasing == settings.antialiasing
            && current.vsync == settings.vsync
            && current.window == settings.window
            && current.windows == settings.windows;
        let mut current_default_fonts = self.document.fonts.iter().filter(|font| font.default);
        let current_default_font = current_default_fonts.next();
        let duplicate_default_font = current_default_fonts.next();
        let default_font_changed = duplicate_default_font.is_some()
            || current_default_font != checked.default_font.as_ref();
        if !static_fields_match || default_font_changed {
            let span = first_changed_static_setting_span(current, settings)
                .or_else(|| duplicate_default_font.map(|font| &font.span))
                .or_else(|| current_default_font.map(|font| &font.span))
                .or_else(|| checked.default_font.as_ref().map(|font| &font.span))
                .unwrap_or(&settings.span);
            return Err(self.invariant(
                span,
                "static application settings changed after semantic analysis",
            ));
        }
        if settings
            .default_text_size
            .is_some_and(|value| !valid_positive_f32(value))
        {
            return Err(self.invariant(
                &settings.span,
                "checked application text size is outside its normalized range",
            ));
        }
        let mut names = std::collections::HashSet::new();
        for window in &settings.windows {
            if !names.insert(window.name.as_str()) {
                return Err(self.invariant(
                    &window.span,
                    "checked application settings contain a duplicate named window",
                ));
            }
            self.validate_checked_window_settings(&window.settings, &window.span)?;
        }
        if let Some(window) = &settings.window {
            self.validate_checked_window_settings(window, &window.span)?;
        }
        Ok(())
    }

    fn validate_checked_window_settings(
        &self,
        settings: &WindowSettings,
        span: &Span,
    ) -> Result<(), Error> {
        for size in [settings.size, settings.min_size, settings.max_size]
            .into_iter()
            .flatten()
        {
            if !valid_positive_f32(size.0) || !valid_positive_f32(size.1) {
                return Err(
                    self.invariant(span, "checked window size is outside its normalized range")
                );
            }
        }
        if let Some(WindowPosition::Specific(x, y)) = settings.position
            && (!valid_f32(x) || !valid_f32(y))
        {
            return Err(self.invariant(
                span,
                "checked window position is outside its normalized range",
            ));
        }
        if let (Some(min), Some(max)) = (settings.min_size, settings.max_size)
            && (min.0 > max.0 || min.1 > max.1)
        {
            return Err(self.invariant(span, "checked window min-size exceeds max-size"));
        }
        if let Some(icon) = &settings.icon {
            let expected = (icon.width as usize)
                .checked_mul(icon.height as usize)
                .and_then(|pixels| pixels.checked_mul(4));
            if icon.width == 0 || icon.height == 0 || expected != Some(icon.byte_len) {
                return Err(self.invariant(
                    &icon.span,
                    "checked window icon dimensions do not match its byte length",
                ));
            }
        }
        Ok(())
    }

    fn validate_app_setting_expression_shape(&self) -> Result<(), Error> {
        use std::collections::HashSet;

        let source = &self
            .facts
            .app_settings()
            .ok_or_else(|| {
                self.invariant(
                    &self.document.settings.span,
                    "application settings are missing their authoritative checked snapshot",
                )
            })?
            .source;
        let mut expected = HashSet::new();
        for (id, setting) in [
            (AppSettingExprId::Title, &source.title),
            (AppSettingExprId::Palette, &source.palette),
            (AppSettingExprId::Background, &source.background),
            (AppSettingExprId::TextColor, &source.text_color),
            (AppSettingExprId::ScaleFactor, &source.scale_factor),
        ] {
            if setting.is_some() {
                expected.insert(id);
            }
        }
        if let Some(checked) = self.facts.app_theme_factory() {
            let Some(AppExpression {
                value: Expr::Call { name, args },
                ..
            }) = source.theme.as_ref()
            else {
                return Err(self.invariant(
                    &source.span,
                    "checked app theme factory has no source factory call",
                ));
            };
            let declaration = self
                .declarations
                .try_extern_decl(checked.function)
                .ok_or_else(|| {
                    self.invariant(
                        &source.span,
                        "app theme factory references an invalid extern ID",
                    )
                })?;
            if declaration.kind != ExternKind::Theme
                || declaration.name != *name
                || declaration.params.len() != args.len()
                || checked.arguments as usize != args.len()
            {
                return Err(self.invariant(
                    &source.span,
                    "app theme factory does not match its authoritative checked facts",
                ));
            }
            expected.extend(
                (0..args.len()).map(|index| AppSettingExprId::ThemeFactoryArgument(index as u32)),
            );
        } else if source.theme.is_some() {
            expected.insert(AppSettingExprId::Theme);
        }
        let actual_entries = self
            .facts
            .expression_uses()
            .iter()
            .filter_map(|expression| match expression.owner {
                CheckedExprOwner::AppSetting(id) => Some(id),
                _ => None,
            })
            .collect::<Vec<_>>();
        let actual = actual_entries.iter().copied().collect::<HashSet<_>>();
        if actual != expected || actual_entries.len() != expected.len() {
            return Err(self.invariant(
                &source.span,
                "app setting expression facts do not match the checked source shape",
            ));
        }
        Ok(())
    }

    fn validate_app_setting_expression_graphs(&self) -> Result<(), Error> {
        let checked = self.facts.app_settings().ok_or_else(|| {
            self.invariant(
                &self.document.settings.span,
                "application settings are missing their authoritative checked snapshot",
            )
        })?;
        let mut graph = CheckedExpressionGraph::default();
        for expression in self.facts.expression_uses() {
            let CheckedExprOwner::AppSetting(setting) = expression.owner else {
                continue;
            };
            let span = self.app_setting_expression_span(&checked.source, setting)?;
            let use_id = self
                .facts
                .expression_use_by_owner(expression.owner)
                .ok_or_else(|| {
                    self.invariant(span, "app setting has no checked expression owner mapping")
                })?;
            if !self
                .facts
                .try_expression_use(use_id)
                .is_some_and(|mapped| std::ptr::eq(mapped, expression))
            {
                return Err(self.invariant(
                    span,
                    "app setting expression owner maps to a different retained use",
                ));
            }
            let expected = self.app_setting_expected_type(setting, expression, span)?;
            let policy = AppSettingExpressionPolicy {
                lowerer: self,
                use_id,
                span,
            };
            self.validate_checked_expression_use_graph(expression, &expected, &policy, &mut graph)?;
        }
        Ok(())
    }

    fn validate_checked_expression_use_graph(
        &self,
        expression: &crate::check::CheckedExprUse,
        expected: &Type,
        policy: &impl CheckedExpressionOwnerPolicy,
        graph: &mut CheckedExpressionGraph,
    ) -> Result<(), Error> {
        if expression.source != *expected
            || expression.destination != *expected
            || expression.coercion != CheckedInitializerCoercion::None
        {
            return Err(self.invariant(
                policy.span(),
                "checked expression use type contract changed after semantic analysis",
            ));
        }
        let scope = graph.root_scope();
        let root = self.validate_checked_expression_node(expression.root, policy, graph, scope)?;
        if root != expression.source {
            return Err(self.invariant(
                policy.span(),
                "checked expression root type does not match its retained use",
            ));
        }
        Ok(())
    }

    fn app_setting_expression_span<'b>(
        &self,
        source: &'b AppSettings,
        id: AppSettingExprId,
    ) -> Result<&'b Span, Error> {
        let setting = match id {
            AppSettingExprId::Title => source.title.as_ref(),
            AppSettingExprId::Theme | AppSettingExprId::ThemeFactoryArgument(_) => {
                source.theme.as_ref()
            }
            AppSettingExprId::Palette => source.palette.as_ref(),
            AppSettingExprId::Background => source.background.as_ref(),
            AppSettingExprId::TextColor => source.text_color.as_ref(),
            AppSettingExprId::ScaleFactor => source.scale_factor.as_ref(),
        };
        setting.map(|setting| &setting.span).ok_or_else(|| {
            self.invariant(
                &source.span,
                "app setting expression owner has no checked source setting",
            )
        })
    }

    fn app_setting_expected_type(
        &self,
        id: AppSettingExprId,
        expression: &crate::check::CheckedExprUse,
        span: &Span,
    ) -> Result<Type, Error> {
        Ok(match id {
            AppSettingExprId::Title
            | AppSettingExprId::Theme
            | AppSettingExprId::Background
            | AppSettingExprId::TextColor => Type::Str,
            AppSettingExprId::ScaleFactor => Type::F64,
            AppSettingExprId::Palette => match &expression.destination {
                Type::Palette(contract) => Type::Palette(contract.clone()),
                _ => {
                    return Err(self.invariant(
                        span,
                        "app palette expression lost its checked theme-contract type",
                    ));
                }
            },
            AppSettingExprId::ThemeFactoryArgument(index) => {
                let factory = self.facts.app_theme_factory().ok_or_else(|| {
                    self.invariant(span, "app theme argument has no checked factory")
                })?;
                let declaration = self
                    .declarations
                    .try_extern_decl(factory.function)
                    .ok_or_else(|| {
                        self.invariant(span, "app theme factory references an invalid extern ID")
                    })?;
                declaration
                    .params
                    .get(index as usize)
                    .map(|(_, ty)| ty.clone())
                    .ok_or_else(|| {
                        self.invariant(span, "app theme argument has an invalid parameter index")
                    })?
            }
        })
    }

    fn validate_checked_expression_node(
        &self,
        id: CheckedExprId,
        policy: &impl CheckedExpressionOwnerPolicy,
        graph: &mut CheckedExpressionGraph,
        scope: usize,
    ) -> Result<Type, Error> {
        if let Some(owner) = graph.node_owners.insert(id, policy.use_id())
            && owner != policy.use_id()
        {
            return Err(self.invariant(
                policy.span(),
                "checked expression node is shared by different retained expression uses",
            ));
        }
        let expression = self.facts.try_expression(id).cloned().ok_or_else(|| {
            self.invariant(
                policy.span(),
                "expression use references an invalid checked expression ID",
            )
        })?;
        if graph.validated.contains(&(id, scope)) {
            return Ok(expression.ty);
        }
        if !graph.visiting.insert(id) {
            return Err(self.invariant(policy.span(), "checked expression graph contains a cycle"));
        }

        let inferred = match &expression.kind {
            CheckedExprKind::Bool(_) => Type::Bool,
            CheckedExprKind::I64(_) => Type::I64,
            CheckedExprKind::F64(_) => Type::F64,
            CheckedExprKind::Str(_) => Type::Str,
            CheckedExprKind::Bytes(_) => Type::Bytes,
            CheckedExprKind::List(values) => {
                let Type::List(item) = &expression.ty else {
                    return Err(self
                        .invariant(policy.span(), "checked list expression has a non-list type"));
                };
                for value in values {
                    let value_ty =
                        self.validate_checked_expression_node(*value, policy, graph, scope)?;
                    if value_ty != **item {
                        return Err(self.invariant(
                            policy.span(),
                            "checked list item type does not match its list type",
                        ));
                    }
                }
                expression.ty.clone()
            }
            CheckedExprKind::None => {
                if !matches!(expression.ty, Type::Option(_)) {
                    return Err(self.invariant(
                        policy.span(),
                        "checked none expression has a non-optional type",
                    ));
                }
                expression.ty.clone()
            }
            CheckedExprKind::SlotProvided(slot) => policy.slot_type(*slot)?,
            CheckedExprKind::Path { root, projections } => {
                let mut current = self.validate_checked_path_root(root, policy, graph, scope)?;
                for projection in projections {
                    if projection.input != current {
                        return Err(self.invariant(
                            policy.span(),
                            "checked projection input does not match its preceding value",
                        ));
                    }
                    let expected = match projection.kind {
                        CheckedProjectionKind::Struct(field_id) => {
                            let field = self
                                .declarations
                                .try_struct_field_decl(field_id)
                                .ok_or_else(|| {
                                    self.invariant(
                                        policy.span(),
                                        "checked projection references an invalid struct field ID",
                                    )
                                })?;
                            let owner = self
                                .declarations
                                .try_struct_decl(field_id.owner)
                                .ok_or_else(|| {
                                    self.invariant(
                                        policy.span(),
                                        "checked projection references an invalid struct ID",
                                    )
                                })?;
                            if current != Type::Named(owner.name.clone())
                                || field.name != projection.field
                            {
                                return Err(self.invariant(
                                    policy.span(),
                                    "checked struct projection topology is inconsistent",
                                ));
                            }
                            field.ty.clone()
                        }
                        CheckedProjectionKind::OptionalWidgetTarget => {
                            if current != Type::Option(Box::new(Type::WidgetTarget)) {
                                return Err(self.invariant(
                                    policy.span(),
                                    "checked optional widget projection has the wrong input type",
                                ));
                            }
                            field_type(&current, &projection.field, &self.document, policy.span())
                                .map_err(|_| {
                                self.invariant(
                                    policy.span(),
                                    "checked optional widget projection is invalid",
                                )
                            })?
                        }
                        CheckedProjectionKind::Native => {
                            if matches!(current, Type::Named(_)) {
                                return Err(self.invariant(
                                    policy.span(),
                                    "checked named projection lost its struct field ID",
                                ));
                            }
                            field_type(&current, &projection.field, &self.document, policy.span())
                                .map_err(|_| {
                                self.invariant(
                                    policy.span(),
                                    "checked native projection is invalid",
                                )
                            })?
                        }
                    };
                    if projection.output != expected {
                        return Err(self.invariant(
                            policy.span(),
                            "checked projection output type is inconsistent",
                        ));
                    }
                    current = expected;
                }
                current
            }
            CheckedExprKind::Call { target, arguments } => {
                let mut argument_types = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    argument_types.push(match argument {
                        CheckedCallArgument::Value(value) => self
                            .facts
                            .try_expression(*value)
                            .map(|expression| expression.ty.clone())
                            .ok_or_else(|| {
                                self.invariant(
                                    policy.span(),
                                    "checked call references an invalid expression ID",
                                )
                            })?,
                        CheckedCallArgument::Binding(local) => policy.local_type(*local)?,
                    });
                }
                let scoped_bindings = self.checked_expression_argument_scopes(
                    target,
                    arguments,
                    &argument_types,
                    &expression.ty,
                    policy,
                )?;
                for (index, argument) in arguments.iter().enumerate() {
                    let CheckedCallArgument::Value(value) = argument else {
                        continue;
                    };
                    let argument_scope = match scoped_bindings[index] {
                        Some(binding) => graph.scoped_binding(scope, binding),
                        None => scope,
                    };
                    self.validate_checked_expression_node(*value, policy, graph, argument_scope)?;
                }
                self.validate_checked_expression_call(
                    target,
                    arguments,
                    &argument_types,
                    &expression.ty,
                    policy,
                )?;
                expression.ty.clone()
            }
            CheckedExprKind::Unary { operator, value } => {
                let value = self.validate_checked_expression_node(*value, policy, graph, scope)?;
                match operator {
                    CheckedUnaryOperator::BooleanNot
                        if value == Type::Bool && expression.ty == Type::Bool => {}
                    CheckedUnaryOperator::NumericNegation(operand)
                        if value == *operand && expression.ty == *operand => {}
                    _ => {
                        return Err(self.invariant(
                            policy.span(),
                            "checked unary expression type contract is inconsistent",
                        ));
                    }
                }
                expression.ty.clone()
            }
            CheckedExprKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.validate_checked_expression_node(*left, policy, graph, scope)?;
                let right = self.validate_checked_expression_node(*right, policy, graph, scope)?;
                let valid = match operator {
                    CheckedBinaryOperator::Boolean(op) => {
                        matches!(op, BinaryOp::And | BinaryOp::Or)
                            && left == Type::Bool
                            && right == Type::Bool
                            && expression.ty == Type::Bool
                    }
                    CheckedBinaryOperator::Equality { op, operand } => {
                        matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
                            && left == *operand
                            && right == *operand
                            && expression.ty == Type::Bool
                    }
                    CheckedBinaryOperator::Ordering { op, operand } => {
                        matches!(
                            op,
                            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq
                        ) && left == *operand
                            && right == *operand
                            && expression.ty == Type::Bool
                    }
                    CheckedBinaryOperator::Arithmetic { op, operand } => {
                        matches!(
                            op,
                            BinaryOp::Add
                                | BinaryOp::Sub
                                | BinaryOp::Mul
                                | BinaryOp::Div
                                | BinaryOp::Rem
                        ) && left == *operand
                            && right == *operand
                            && expression.ty == *operand
                    }
                };
                if !valid {
                    return Err(self.invariant(
                        policy.span(),
                        "checked binary expression type contract is inconsistent",
                    ));
                }
                expression.ty.clone()
            }
        };
        if inferred != expression.ty {
            return Err(self.invariant(
                policy.span(),
                "checked expression kind does not match its retained type",
            ));
        }
        graph.visiting.remove(&id);
        graph.validated.insert((id, scope));
        Ok(expression.ty)
    }

    fn validate_checked_path_root(
        &self,
        root: &CheckedPathRoot,
        policy: &impl CheckedExpressionOwnerPolicy,
        graph: &CheckedExpressionGraph,
        scope: usize,
    ) -> Result<Type, Error> {
        match root {
            CheckedPathRoot::Value(value) => policy.value_type(*value),
            CheckedPathRoot::Local(local) => {
                let ty = policy.local_type(*local)?;
                if self.facts.try_local(*local).is_some_and(|local| {
                    matches!(local.owner, CheckedLocalOwner::ExpressionBinding { .. })
                }) && !graph.contains_binding(scope, *local)
                {
                    return Err(self.invariant(
                        policy.span(),
                        "checked expression binding is outside its lexical scoped-value body",
                    ));
                }
                Ok(ty)
            }
            CheckedPathRoot::EnumVariant(id) => {
                let variant = self
                    .declarations
                    .try_enum_variant_decl(*id)
                    .ok_or_else(|| {
                        self.invariant(
                            policy.span(),
                            "checked path references an invalid enum variant ID",
                        )
                    })?;
                if variant.payload.is_some() {
                    return Err(self.invariant(
                        policy.span(),
                        "checked path uses a payload enum variant without a call",
                    ));
                }
                let owner = self.declarations.try_enum_decl(id.owner).ok_or_else(|| {
                    self.invariant(policy.span(), "checked path references an invalid enum ID")
                })?;
                Ok(Type::Named(owner.name.clone()))
            }
            CheckedPathRoot::Palette(id) => policy.palette_type(*id),
        }
    }

    fn validate_checked_expression_binding(
        &self,
        id: CheckedLocalId,
        arguments: &[CheckedCallArgument],
        policy: &impl CheckedExpressionOwnerPolicy,
        expected_body: usize,
    ) -> Result<Type, Error> {
        let ty = policy.local_type(id)?;
        let local = self.facts.try_local(id).ok_or_else(|| {
            self.invariant(
                policy.span(),
                "checked expression binding references an invalid local ID",
            )
        })?;
        let CheckedLocalOwner::ExpressionBinding {
            expression,
            body_argument,
        } = local.owner
        else {
            return Err(self.invariant(
                policy.span(),
                "checked call binding is not an expression binding",
            ));
        };
        if expression != policy.use_id()
            || body_argument != expected_body
            || !matches!(
                arguments.get(body_argument),
                Some(CheckedCallArgument::Value(_))
            )
        {
            return Err(self.invariant(
                policy.span(),
                "checked call binding has an invalid body-argument topology",
            ));
        }
        Ok(ty)
    }

    fn checked_expression_argument_scopes(
        &self,
        target: &CheckedCallTarget,
        arguments: &[CheckedCallArgument],
        argument_types: &[Type],
        output: &Type,
        policy: &impl CheckedExpressionOwnerPolicy,
    ) -> Result<Vec<Option<CheckedLocalId>>, Error> {
        let mut scoped_bindings = vec![None; arguments.len()];
        let CheckedCallTarget::Builtin(id) = target else {
            return Ok(scoped_bindings);
        };
        let name = self.facts.try_builtin(*id).ok_or_else(|| {
            self.invariant(
                policy.span(),
                "checked call references an invalid builtin ID",
            )
        })?;
        let Some(builtin) = ContextualBuiltin::from_name(name) else {
            return Ok(scoped_bindings);
        };
        let contexts = builtin
            .argument_contexts(output, argument_types)
            .map_err(|message| self.invariant(policy.span(), message))?;
        if contexts.len() != arguments.len() {
            return Err(self.invariant(
                policy.span(),
                "checked builtin call has an invalid argument count",
            ));
        }
        for (index, context) in contexts.iter().enumerate() {
            let BuiltinArgumentContext::ScopedValue { binding, .. } = context else {
                continue;
            };
            let Some(CheckedCallArgument::Binding(local)) = arguments.get(*binding) else {
                return Err(self.invariant(
                    policy.span(),
                    "checked builtin scoped value has no binding argument",
                ));
            };
            self.validate_checked_expression_binding(*local, arguments, policy, index)?;
            scoped_bindings[index] = Some(*local);
        }
        Ok(scoped_bindings)
    }

    fn validate_checked_expression_call(
        &self,
        target: &CheckedCallTarget,
        arguments: &[CheckedCallArgument],
        argument_types: &[Type],
        output: &Type,
        policy: &impl CheckedExpressionOwnerPolicy,
    ) -> Result<(), Error> {
        match target {
            CheckedCallTarget::Extern(id) => {
                let function = self.declarations.try_extern_decl(*id).ok_or_else(|| {
                    self.invariant(
                        policy.span(),
                        "checked call references an invalid extern ID",
                    )
                })?;
                if arguments
                    .iter()
                    .any(|argument| matches!(argument, CheckedCallArgument::Binding(_)))
                    || function.params.len() != argument_types.len()
                    || function
                        .params
                        .iter()
                        .map(|(_, ty)| ty)
                        .ne(argument_types.iter())
                    || function.output != *output
                {
                    return Err(self.invariant(
                        policy.span(),
                        "checked extern call has an inconsistent retained signature",
                    ));
                }
            }
            CheckedCallTarget::EnumVariant(id) => {
                let variant = self
                    .declarations
                    .try_enum_variant_decl(*id)
                    .ok_or_else(|| {
                        self.invariant(
                            policy.span(),
                            "checked call references an invalid enum variant ID",
                        )
                    })?;
                let owner = self.declarations.try_enum_decl(id.owner).ok_or_else(|| {
                    self.invariant(policy.span(), "checked call references an invalid enum ID")
                })?;
                if arguments.len() != 1
                    || !matches!(arguments[0], CheckedCallArgument::Value(_))
                    || variant.payload.as_ref() != argument_types.first()
                    || *output != Type::Named(owner.name.clone())
                {
                    return Err(self.invariant(
                        policy.span(),
                        "checked enum call has an inconsistent retained signature",
                    ));
                }
            }
            CheckedCallTarget::Builtin(id) => {
                let name = self.facts.try_builtin(*id).ok_or_else(|| {
                    self.invariant(
                        policy.span(),
                        "checked call references an invalid builtin ID",
                    )
                })?;
                self.validate_checked_builtin_call(
                    name,
                    arguments,
                    argument_types,
                    output,
                    policy,
                )?;
            }
        }
        Ok(())
    }

    fn validate_checked_builtin_call(
        &self,
        name: &str,
        arguments: &[CheckedCallArgument],
        argument_types: &[Type],
        output: &Type,
        policy: &impl CheckedExpressionOwnerPolicy,
    ) -> Result<(), Error> {
        if let Some(builtin) = ContextualBuiltin::from_name(name) {
            let contexts = builtin
                .argument_contexts(output, argument_types)
                .map_err(|message| self.invariant(policy.span(), message))?;
            if contexts.len() != arguments.len() {
                return Err(self.invariant(
                    policy.span(),
                    "checked builtin call has an invalid argument count",
                ));
            }
            for (index, ((argument, actual), context)) in arguments
                .iter()
                .zip(argument_types)
                .zip(&contexts)
                .enumerate()
            {
                match context {
                    BuiltinArgumentContext::Value { expected } => {
                        if !matches!(argument, CheckedCallArgument::Value(_))
                            || expected.as_ref().is_some_and(|expected| expected != actual)
                        {
                            return Err(self.invariant(
                                policy.span(),
                                "checked builtin value argument has an inconsistent retained signature",
                            ));
                        }
                    }
                    BuiltinArgumentContext::Binding { ty, body } => {
                        let CheckedCallArgument::Binding(local) = argument else {
                            return Err(self.invariant(
                                policy.span(),
                                "checked builtin binding argument has an inconsistent retained signature",
                            ));
                        };
                        if actual != ty {
                            return Err(self.invariant(
                                policy.span(),
                                "checked builtin binding type is inconsistent",
                            ));
                        }
                        self.validate_checked_expression_binding(*local, arguments, policy, *body)?;
                    }
                    BuiltinArgumentContext::ScopedValue { expected, binding } => {
                        if !matches!(argument, CheckedCallArgument::Value(_))
                            || expected.as_ref().is_some_and(|expected| expected != actual)
                        {
                            return Err(self.invariant(
                                policy.span(),
                                "checked builtin scoped value has an inconsistent retained signature",
                            ));
                        }
                        let Some(CheckedCallArgument::Binding(local)) = arguments.get(*binding)
                        else {
                            return Err(self.invariant(
                                policy.span(),
                                "checked builtin scoped value has no binding argument",
                            ));
                        };
                        self.validate_checked_expression_binding(*local, arguments, policy, index)?;
                    }
                }
            }
        } else if arguments
            .iter()
            .any(|argument| matches!(argument, CheckedCallArgument::Binding(_)))
        {
            return Err(self.invariant(
                policy.span(),
                "non-contextual checked builtin contains a binding argument",
            ));
        }

        let mut env = HashMap::new();
        let mut source_arguments = Vec::with_capacity(arguments.len());
        for (index, (argument, ty)) in arguments.iter().zip(argument_types).enumerate() {
            source_arguments.push(
                self.checked_builtin_contract_argument(argument, ty, index, &mut env, policy)?,
            );
        }
        let canonical =
            canonical_builtin_type(name, &source_arguments, &env, &self.document, policy.span())
                .map_err(|error| {
                    self.invariant(
                        policy.span(),
                        format!(
                            "checked builtin call `{name}` violates its canonical contract: {}",
                            error.message
                        ),
                    )
                })?;
        if resolve_erased_type(&canonical) != *output {
            return Err(self.invariant(
                policy.span(),
                "checked builtin output type is inconsistent with its canonical contract",
            ));
        }
        Ok(())
    }

    fn checked_builtin_contract_argument(
        &self,
        argument: &CheckedCallArgument,
        ty: &Type,
        index: usize,
        env: &mut HashMap<String, Type>,
        policy: &impl CheckedExpressionOwnerPolicy,
    ) -> Result<Expr, Error> {
        match argument {
            CheckedCallArgument::Value(id) => {
                if let Some(literal) = self.checked_builtin_literal(*id) {
                    return Ok(literal);
                }
                let name = format!("__checked_builtin_argument_{index}");
                env.insert(name.clone(), ty.clone());
                Ok(Expr::Path(vec![name]))
            }
            CheckedCallArgument::Binding(id) => {
                let local = self.facts.try_local(*id).ok_or_else(|| {
                    self.invariant(
                        policy.span(),
                        "checked builtin binding references an invalid local ID",
                    )
                })?;
                Ok(Expr::Path(vec![local.name.clone()]))
            }
        }
    }

    fn checked_builtin_literal(&self, id: CheckedExprId) -> Option<Expr> {
        let expression = self.facts.try_expression(id)?;
        Some(match &expression.kind {
            CheckedExprKind::Bool(value) => Expr::Bool(*value),
            CheckedExprKind::I64(value) => Expr::I64(*value),
            CheckedExprKind::F64(value) => Expr::F64(*value),
            CheckedExprKind::Str(value) => Expr::Str(value.clone()),
            CheckedExprKind::Bytes(value) => Expr::Bytes(value.clone()),
            CheckedExprKind::Unary {
                operator: CheckedUnaryOperator::NumericNegation(_),
                value,
            } => Expr::Unary {
                op: UnaryOp::Neg,
                value: Box::new(self.checked_builtin_literal(*value)?),
            },
            _ => return None,
        })
    }

    fn checked_app_setting_expression(
        &self,
        id: AppSettingExprId,
        span: &Span,
    ) -> Result<ResolvedAppExpression, Error> {
        let declaration = self
            .declarations
            .app_setting_expression(id)
            .ok_or_else(|| self.invariant(span, "app setting has no stable HIR ID"))?;
        let owner = crate::check::CheckedExprOwner::AppSetting(id);
        let expression = self
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| self.invariant(span, "app setting has no checked expression"))?;
        let expression_fact = self.facts.try_expression_use(expression).ok_or_else(|| {
            self.invariant(
                span,
                "app setting references an invalid checked expression ID",
            )
        })?;
        if expression_fact.owner != owner {
            return Err(self.invariant(
                span,
                "app setting references a mismatched checked expression owner",
            ));
        }
        if self.facts.try_expression(expression_fact.root).is_none() {
            return Err(self.invariant(
                span,
                "app setting references an invalid checked expression root ID",
            ));
        }
        Ok(ResolvedAppExpression {
            id,
            expression,
            origin: declaration.origin,
        })
    }

    fn lower_window_settings(
        &mut self,
        source: Option<&WindowSettings>,
        parent: OriginId,
    ) -> ResolvedWindowSettings {
        let default = WindowSettings::default();
        let source = source.unwrap_or(&default);
        let origin = self.push_origin(&source.span, Some(parent));
        let field_origins = source
            .setting_spans
            .iter()
            .map(|(name, span)| (name.clone(), self.push_origin(span, Some(origin))))
            .collect();
        ResolvedWindowSettings {
            size: source.size,
            maximized: source.maximized,
            fullscreen: source.fullscreen,
            position: source.position.map(|position| match position {
                WindowPosition::Default => ResolvedWindowPosition::Default,
                WindowPosition::Centered => ResolvedWindowPosition::Centered,
                WindowPosition::Specific(x, y) => ResolvedWindowPosition::Specific(x, y),
            }),
            min_size: source.min_size,
            max_size: source.max_size,
            visible: source.visible,
            resizable: source.resizable,
            closeable: source.closeable,
            minimizable: source.minimizable,
            decorations: source.decorations,
            transparent: source.transparent,
            blur: source.blur,
            level: source.level.map(|level| match level {
                WindowLevel::Normal => ResolvedWindowLevel::Normal,
                WindowLevel::AlwaysOnBottom => ResolvedWindowLevel::AlwaysOnBottom,
                WindowLevel::AlwaysOnTop => ResolvedWindowLevel::AlwaysOnTop,
            }),
            icon: source.icon.as_ref().map(|icon| ResolvedWindowIcon {
                path: icon.path.clone(),
                width: icon.width,
                height: icon.height,
                byte_len: icon.byte_len,
                origin: self.push_origin(&icon.span, Some(origin)),
            }),
            exit_on_close_request: source.exit_on_close_request,
            linux: source.linux.as_ref().map(|settings| {
                let platform_origin = self.push_origin(&settings.span, Some(origin));
                ResolvedLinuxWindowSettings {
                    application_id: settings.application_id.clone(),
                    override_redirect: settings.override_redirect,
                    field_origins: self
                        .lower_setting_origins(&settings.setting_spans, platform_origin),
                    origin: platform_origin,
                }
            }),
            windows: source.windows.as_ref().map(|settings| {
                let platform_origin = self.push_origin(&settings.span, Some(origin));
                ResolvedWindowsWindowSettings {
                    drag_and_drop: settings.drag_and_drop,
                    skip_taskbar: settings.skip_taskbar,
                    undecorated_shadow: settings.undecorated_shadow,
                    corner: settings.corner.map(|corner| match corner {
                        WindowCorner::Default => ResolvedWindowCorner::Default,
                        WindowCorner::DoNotRound => ResolvedWindowCorner::DoNotRound,
                        WindowCorner::Round => ResolvedWindowCorner::Round,
                        WindowCorner::RoundSmall => ResolvedWindowCorner::RoundSmall,
                    }),
                    field_origins: self
                        .lower_setting_origins(&settings.setting_spans, platform_origin),
                    origin: platform_origin,
                }
            }),
            macos: source.macos.as_ref().map(|settings| {
                let platform_origin = self.push_origin(&settings.span, Some(origin));
                ResolvedMacosWindowSettings {
                    title_hidden: settings.title_hidden,
                    titlebar_transparent: settings.titlebar_transparent,
                    fullsize_content_view: settings.fullsize_content_view,
                    field_origins: self
                        .lower_setting_origins(&settings.setting_spans, platform_origin),
                    origin: platform_origin,
                }
            }),
            wasm: source.wasm.as_ref().map(|settings| {
                let platform_origin = self.push_origin(&settings.span, Some(origin));
                ResolvedWasmWindowSettings {
                    target: settings.target.clone(),
                    field_origins: self
                        .lower_setting_origins(&settings.setting_spans, platform_origin),
                    origin: platform_origin,
                }
            }),
            field_origins,
            origin,
        }
    }

    fn lower_setting_origins(
        &mut self,
        spans: &std::collections::BTreeMap<String, Span>,
        parent: OriginId,
    ) -> HashMap<String, OriginId> {
        spans
            .iter()
            .map(|(name, span)| (name.clone(), self.push_origin(span, Some(parent))))
            .collect()
    }

    fn lower_subscriptions(&self) -> Result<Vec<ResolvedSubscription>, Error> {
        if self.facts.subscriptions().len() != self.document.subscriptions.len()
            || self.facts.subscriptions().len() != self.declarations.subscription_count()
        {
            return Err(self.invariant_at(
                &Span::line(1),
                "checked subscription topology changed before HIR lowering",
            ));
        }
        self.facts
            .subscriptions()
            .iter()
            .zip(&self.document.subscriptions)
            .enumerate()
            .map(|(index, (subscription, source))| {
                self.lower_subscription(index, subscription, source)
                    .map_err(|error| {
                        if error.path.is_some() {
                            error
                        } else {
                            self.invariant_at(&subscription.span, error.message)
                        }
                    })
            })
            .collect()
    }

    fn lower_subscription(
        &self,
        index: usize,
        subscription: &CheckedSubscription,
        raw: &Subscription,
    ) -> Result<ResolvedSubscription, Error> {
        let span = &subscription.span;
        let declaration = self.declarations.try_subscription(index).ok_or_else(|| {
            self.invariant_at(
                &raw.span,
                "checked subscription has no declaration identity",
            )
        })?;
        if subscription.id != declaration.id
            || subscription.origin != declaration.origin
            || subscription.syntax.ne(raw)
            || raw.span != subscription.span
            || raw.window_id != subscription.window_id
            || raw.status != subscription.status
            || raw.condition.is_some() != subscription.condition.is_some()
            || raw.context.is_some() != subscription.context.is_some()
            || raw.filter.as_deref()
                != subscription
                    .filter
                    .as_ref()
                    .map(|filter| filter.name.as_str())
            || raw.route.handler != subscription.route.handler_name
            || raw.route.args.len() != subscription.route.payloads.len()
            || raw
                .route
                .args
                .iter()
                .any(|argument| !matches!(argument, RouteArg::Payload))
            || !subscription_source_matches(&subscription.source, &raw.source)
        {
            return Err(Error::new(
                "E196",
                &raw.span,
                "checked subscription topology changed before HIR lowering",
            ));
        }
        let ValidatedSubscriptionContract {
            source_payloads,
            delivered_payloads,
            filter,
        } = self.validate_subscription_contract(subscription)?;
        let source = match &subscription.source {
            CheckedSubscriptionSource::Every { milliseconds } => {
                ResolvedSubscriptionSource::Every {
                    milliseconds: *milliseconds,
                }
            }
            CheckedSubscriptionSource::Repeat {
                function,
                milliseconds,
            } => ResolvedSubscriptionSource::Repeat {
                function: self.resolve_subscription_extern(function, ExternKind::Future, span)?,
                milliseconds: *milliseconds,
            },
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            } => ResolvedSubscriptionSource::Run {
                function: self.resolve_subscription_extern(function, ExternKind::Stream, span)?,
                arguments: arguments.clone(),
            },
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            } => ResolvedSubscriptionSource::Recipe {
                function: self.resolve_subscription_extern(function, ExternKind::Recipe, span)?,
                arguments: arguments.clone(),
            },
            CheckedSubscriptionSource::Events { identity, filter } => {
                ResolvedSubscriptionSource::Events {
                    identity: *identity,
                    filter: self.resolve_subscription_extern(
                        filter,
                        ExternKind::EventFilter,
                        span,
                    )?,
                }
            }
            CheckedSubscriptionSource::Event { raw } => {
                ResolvedSubscriptionSource::Event { raw: *raw }
            }
            CheckedSubscriptionSource::Extern {
                function,
                arguments,
            } => ResolvedSubscriptionSource::Extern {
                function: self.resolve_subscription_extern(
                    function,
                    ExternKind::Subscription,
                    span,
                )?,
                arguments: arguments.clone(),
            },
            CheckedSubscriptionSource::InputMethod(event) => {
                ResolvedSubscriptionSource::InputMethod(*event)
            }
            CheckedSubscriptionSource::Keyboard(event) => {
                ResolvedSubscriptionSource::Keyboard(*event)
            }
            CheckedSubscriptionSource::Mouse(event) => ResolvedSubscriptionSource::Mouse(*event),
            CheckedSubscriptionSource::SystemTheme => ResolvedSubscriptionSource::SystemTheme,
            CheckedSubscriptionSource::Touch(event) => ResolvedSubscriptionSource::Touch(*event),
            CheckedSubscriptionSource::Window(event) => ResolvedSubscriptionSource::Window(*event),
        };
        let handler = self
            .declarations
            .checked_handler(subscription.route.handler, span)?;
        if handler.name != subscription.route.handler_name {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription route has a mismatched handler identity",
            ));
        }
        let source_payloads = source_payloads
            .iter()
            .map(|payload| self.resolve_type(payload, span))
            .collect::<Result<Vec<_>, _>>()?;
        let delivered_payloads = delivered_payloads
            .iter()
            .map(|payload| self.resolve_type(payload, span))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ResolvedSubscription {
            id: subscription.id,
            source,
            source_payloads,
            delivered_payloads,
            filter,
            context: subscription.context,
            condition: subscription.condition,
            window_id: subscription.window_id,
            status: subscription.status,
            route: ResolvedSubscriptionRoute {
                handler: subscription.route.handler,
                handler_name: handler.name.clone(),
                payloads: subscription.route.payloads.clone(),
            },
            span: subscription.span.clone(),
            origin: subscription.origin,
        })
    }

    fn validate_subscription_contract(
        &self,
        subscription: &CheckedSubscription,
    ) -> Result<ValidatedSubscriptionContract, Error> {
        let span = &subscription.span;
        if let Some(condition) = subscription.condition {
            self.facts.validate_subscription_expression_use(
                condition,
                SubscriptionExpressionContract {
                    subscription: subscription.id,
                    role: CheckedSubscriptionExprRole::Condition,
                    expected: Some(&Type::Bool),
                    declarations: &self.declarations,
                    document: &self.document,
                    span,
                },
            )?;
        }
        let source_payloads = match &subscription.source {
            CheckedSubscriptionSource::Repeat { function, .. } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Future, span)?;
                if !function.params.is_empty() {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked repeat subscription has a mismatched arity",
                    ));
                }
                vec![extern_subscription_payload(function)]
            }
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Stream, span)?;
                self.validate_subscription_arguments(subscription.id, arguments, function, span)?;
                vec![extern_subscription_payload(function)]
            }
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Recipe, span)?;
                self.validate_subscription_arguments(subscription.id, arguments, function, span)?;
                vec![function.output.clone()]
            }
            CheckedSubscriptionSource::Events { identity, filter } => {
                self.facts.validate_subscription_expression_use(
                    *identity,
                    SubscriptionExpressionContract {
                        subscription: subscription.id,
                        role: CheckedSubscriptionExprRole::EventIdentity,
                        expected: None,
                        declarations: &self.declarations,
                        document: &self.document,
                        span,
                    },
                )?;
                let function =
                    self.checked_subscription_extern(filter, ExternKind::EventFilter, span)?;
                if !function.params.is_empty() {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked event-filter subscription has a mismatched arity",
                    ));
                }
                vec![function.output.clone()]
            }
            CheckedSubscriptionSource::Extern {
                function,
                arguments,
            } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Subscription, span)?;
                self.validate_subscription_arguments(subscription.id, arguments, function, span)?;
                vec![function.output.clone()]
            }
            source => resolved_native_subscription_payloads(source, subscription.window_id)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        span,
                        "checked subscription source has no intrinsic payload contract",
                    )
                })?,
        };
        if source_payloads != subscription.source_payloads {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription source has a mismatched payload contract",
            ));
        }

        let filter = subscription
            .filter
            .as_ref()
            .map(|reference| {
                let function =
                    self.checked_subscription_extern(reference, ExternKind::Sync, span)?;
                if function
                    .params
                    .iter()
                    .map(|(_, ty)| ty)
                    .ne(&source_payloads)
                {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked subscription filter has mismatched parameter types",
                    ));
                }
                let Type::Option(output) = &function.output else {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked subscription filter has a non-optional output",
                    ));
                };
                Ok((
                    self.resolve_subscription_extern(reference, ExternKind::Sync, span)?,
                    output.as_ref().clone(),
                ))
            })
            .transpose()?;
        let mut delivered_payloads = if let Some((_, output)) = &filter {
            vec![output.clone()]
        } else {
            source_payloads.clone()
        };
        if let Some(context) = subscription.context {
            delivered_payloads.insert(
                0,
                self.facts.validate_subscription_expression_use(
                    context,
                    SubscriptionExpressionContract {
                        subscription: subscription.id,
                        role: CheckedSubscriptionExprRole::Context,
                        expected: None,
                        declarations: &self.declarations,
                        document: &self.document,
                        span,
                    },
                )?,
            );
        }
        if delivered_payloads != subscription.delivered_payloads {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription transforms have a mismatched delivered payload contract",
            ));
        }
        let handler = self
            .declarations
            .checked_handler(subscription.route.handler, span)?;
        let routed = subscription
            .route
            .payloads
            .iter()
            .map(|index| {
                delivered_payloads
                    .get(*index as usize)
                    .cloned()
                    .ok_or_else(|| {
                        Error::new(
                            "E196",
                            span,
                            "checked subscription route references an invalid payload index",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if handler.name != subscription.route.handler_name || routed != handler.payloads {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription route has a mismatched handler contract",
            ));
        }
        Ok(ValidatedSubscriptionContract {
            source_payloads,
            delivered_payloads,
            filter: filter.map(|(contract, _)| contract),
        })
    }

    fn validate_subscription_arguments(
        &self,
        subscription: SubscriptionId,
        arguments: &[CheckedExprUseId],
        function: &crate::hir::ExternDeclaration,
        span: &Span,
    ) -> Result<(), Error> {
        if arguments.len() != function.params.len() {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern arguments have a mismatched arity",
            ));
        }
        for (index, (argument, (_, expected))) in arguments.iter().zip(&function.params).enumerate()
        {
            self.facts.validate_subscription_expression_use(
                *argument,
                SubscriptionExpressionContract {
                    subscription,
                    role: CheckedSubscriptionExprRole::SourceArgument(index as u32),
                    expected: Some(expected),
                    declarations: &self.declarations,
                    document: &self.document,
                    span,
                },
            )?;
        }
        Ok(())
    }

    fn checked_subscription_extern(
        &self,
        reference: &ExternRef,
        kind: ExternKind,
        span: &Span,
    ) -> Result<&crate::hir::ExternDeclaration, Error> {
        let declaration = self.declarations.checked_extern_decl(reference.id, span)?;
        if declaration.name != reference.name || declaration.kind != kind {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern reference has a mismatched declaration contract",
            ));
        }
        Ok(declaration)
    }

    fn resolve_subscription_extern(
        &self,
        reference: &ExternRef,
        kind: ExternKind,
        span: &Span,
    ) -> Result<ResolvedExternContract, Error> {
        let declaration = self.declarations.checked_extern_decl(reference.id, span)?;
        if declaration.name != reference.name || declaration.kind != kind {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern reference has a mismatched declaration contract",
            ));
        }
        Ok(ResolvedExternContract {
            id: declaration.declaration.id,
            name: declaration.name.clone(),
            rust_path: declaration.rust_path.clone(),
            params: declaration
                .params
                .iter()
                .map(|(_, ty)| self.resolve_type(ty, span))
                .collect::<Result<Vec<_>, _>>()?,
            output: self.resolve_type(&declaration.output, span)?,
            error: declaration
                .error
                .as_ref()
                .map(|error| self.resolve_type(error, span))
                .transpose()?,
        })
    }

    fn resolve_type(&self, ty: &Type, span: &Span) -> Result<ResolvedType, Error> {
        Ok(match ty {
            Type::List(inner) => ResolvedType::List(Box::new(self.resolve_type(inner, span)?)),
            Type::Option(inner) => ResolvedType::Option(Box::new(self.resolve_type(inner, span)?)),
            Type::Result(output, error) => ResolvedType::Result(
                Box::new(self.resolve_type(output, span)?),
                Box::new(self.resolve_type(error, span)?),
            ),
            Type::Combo(inner) => ResolvedType::Combo(Box::new(self.resolve_type(inner, span)?)),
            Type::Animation(inner) => {
                ResolvedType::Animation(Box::new(self.resolve_type(inner, span)?))
            }
            Type::Named(name) => {
                ResolvedType::Named(self.declarations.named_type_id(name).ok_or_else(|| {
                    Error::new(
                        "E196",
                        span,
                        format!("checked type references unknown declaration `{name}`"),
                    )
                })?)
            }
            Type::Unknown => {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked subscription type remained unknown",
                ));
            }
            ty => ResolvedType::Value(ty.clone()),
        })
    }

    fn invariant_at(&self, span: &Span, message: impl Into<String>) -> Error {
        let message = message.into();
        if let Some((path, line)) = self.origins.source_origin(span.line) {
            Error::new(
                "E196",
                &Span {
                    line,
                    column: span.column,
                },
                message,
            )
            .at_path(path.display().to_string())
        } else {
            Error::new("E196", span, message)
        }
    }

    fn lower_app_states(&self) -> Result<Vec<AppStateContract>, Error> {
        self.document
            .states
            .iter()
            .enumerate()
            .map(|(index, state)| {
                let declaration = self.declarations.app_state(index);
                Ok(AppStateContract {
                    id: declaration.id,
                    name: state.name.clone(),
                    ty: state.ty.clone(),
                    initializer: self
                        .resolve_initializer(CheckedValueRef::AppState(declaration.id), state)?,
                    span: state.span.clone(),
                    origin: declaration.origin,
                })
            })
            .collect()
    }

    fn lower_derived(&self) -> Result<Vec<DerivedContract>, Error> {
        self.document
            .derived
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let declaration = self.declarations.derived(index);
                let checked = self
                    .facts
                    .value_by_ref(CheckedValueRef::Derived(declaration.id));
                Ok(DerivedContract {
                    id: declaration.id,
                    name: value.name.clone(),
                    ty: value.ty.clone(),
                    initializer: checked.initializer.ok_or_else(|| {
                        self.invariant(&value.span, "derived value has no checked initializer")
                    })?,
                    span: value.span.clone(),
                    origin: declaration.origin,
                })
            })
            .collect()
    }

    fn resolve_initializer(
        &self,
        value_ref: CheckedValueRef,
        state: &State,
    ) -> Result<ResolvedInitializer, Error> {
        let checked = self.facts.value_by_ref(value_ref);
        let expression = checked.initializer.ok_or_else(|| {
            self.invariant(&state.span, "state has no checked initializer expression")
        })?;
        let animation = state
            .animation
            .as_ref()
            .map(|options| self.resolve_animation(options, &state.span))
            .transpose()?;
        Ok(ResolvedInitializer {
            expression,
            animation,
        })
    }

    fn resolve_animation(
        &self,
        options: &AnimationOptions,
        span: &Span,
    ) -> Result<ResolvedAnimation, Error> {
        let easing = options
            .easing
            .as_deref()
            .map(|easing| {
                if ANIMATION_EASINGS.contains(&easing) {
                    Ok(ResolvedAnimationEasing::Builtin(easing.to_owned()))
                } else {
                    self.declarations
                        .extern_decl_by_name(easing)
                        .map(|declaration| {
                            ResolvedAnimationEasing::Custom(declaration.declaration.id)
                        })
                        .ok_or_else(|| {
                            self.invariant(span, "animation easing has no extern declaration")
                        })
                }
            })
            .transpose()?;
        Ok(ResolvedAnimation {
            easing,
            duration: options.duration,
            delay_ms: options.delay_ms,
            repeat: options.repeat,
            repeat_forever: options.repeat_forever,
            auto_reverse: options.auto_reverse,
        })
    }

    fn index_components(&mut self) -> Result<(), Error> {
        let source_components = self.document.components.clone();
        for (index, component) in source_components.into_iter().enumerate() {
            let declaration = self.declarations.component(index);
            let id = declaration.id;
            let origin = declaration.origin;
            let mut params = Vec::with_capacity(component.params.len());
            for (index, param) in component.params.iter().enumerate() {
                let declaration = self.declarations.component_param(id, index);
                params.push(ComponentParamContract {
                    id: declaration.id,
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    capability: if param.bind {
                        ParamCapability::Bind
                    } else {
                        ParamCapability::Read
                    },
                    default: self
                        .facts
                        .value_by_ref(CheckedValueRef::ComponentParam(declaration.id))
                        .initializer,
                    origin: declaration.origin,
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
                .map(|(index, (name, optional, _span))| ComponentSlotContract {
                    id: self.declarations.component_slot(id, index).id,
                    name,
                    optional,
                    origin: self.declarations.component_slot(id, index).origin,
                })
                .collect();
            let states = component
                .states
                .iter()
                .enumerate()
                .map(|(index, state)| {
                    let declaration = self.declarations.component_state(id, index);
                    Ok(ComponentStateContract {
                        id: declaration.id,
                        name: state.name.clone(),
                        ty: state.ty.clone(),
                        initializer: self.resolve_initializer(
                            CheckedValueRef::ComponentState(declaration.id),
                            state,
                        )?,
                        span: state.span.clone(),
                        origin: declaration.origin,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
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
            self.components.push(ComponentContract {
                id,
                name: component.name,
                params,
                output: component.output,
                events,
                slots,
                states,
                handlers: Vec::new(),
                root: component.root,
                storage,
                origin,
            });
            self.component_indexes.push(ComponentIndex {
                params_by_name,
                events_by_name,
                slots_by_name,
            });
        }
        Ok(())
    }

    fn lower_handlers(&mut self) -> Result<(), Error> {
        let mut declaration_index = 0usize;
        for index in 0..self.document.handlers.len() {
            let handler = self.document.handlers[index].clone();
            let id = self.lower_handler(declaration_index, &handler, HandlerOwner::App)?;
            self.app_handlers.push(id);
            declaration_index += 1;
        }
        for component_index in 0..self.document.components.len() {
            let component = self.document.components[component_index].clone();
            for handler in &component.handlers {
                let id = self.lower_handler(
                    declaration_index,
                    handler,
                    HandlerOwner::Component(ComponentId(component_index as u32)),
                )?;
                self.components[component_index].handlers.push(id);
                declaration_index += 1;
            }
        }
        for preset_index in 0..self.document.presets.len() {
            let preset = self.document.presets[preset_index].clone();
            let handler = Handler {
                name: format!("preset {}", preset.name),
                params: Vec::new(),
                statements: preset.statements,
                span: preset.span,
            };
            let id = self.lower_handler(
                declaration_index,
                &handler,
                HandlerOwner::Preset(preset_index as u32),
            )?;
            self.preset_handlers.push(id);
            declaration_index += 1;
        }
        if declaration_index != self.declarations.handlers().len()
            || self.handlers.len() != declaration_index
        {
            return Err(self.invariant(
                &Span::line(1),
                "handler lowering did not consume the complete declaration arena",
            ));
        }
        self.validate_handler_arena_consumption()?;
        Ok(())
    }

    fn validate_handler_arena_consumption(&self) -> Result<(), Error> {
        fn mark(seen: &mut [bool], index: u32) -> bool {
            seen.get_mut(index as usize).is_some_and(|slot| {
                *slot = true;
                true
            })
        }
        fn mark_route(route: &ResolvedRoute, routes: &mut [bool]) -> bool {
            mark(routes, route.id.0)
        }
        fn mark_source(source: &ResolvedTaskSource, tasks: &mut [bool]) -> bool {
            let task = match source {
                ResolvedTaskSource::Effect { task, .. }
                | ResolvedTaskSource::Done { task, .. }
                | ResolvedTaskSource::None { task, .. } => *task,
            };
            mark(tasks, task.0)
        }
        fn visit(
            statement: &ResolvedStatement,
            statements: &mut [bool],
            tasks: &mut [bool],
            routes: &mut [bool],
            run_sites: &mut [bool],
        ) -> bool {
            if !mark(statements, statement.id.0)
                || statement.task.is_some_and(|task| !mark(tasks, task.0))
            {
                return false;
            }
            match &statement.kind {
                ResolvedStatementKind::Run(run) => {
                    mark_route(&run.success, routes)
                        && run
                            .error
                            .as_ref()
                            .is_none_or(|route| mark_route(route, routes))
                        && run.site.is_none_or(|site| mark(run_sites, site.0))
                }
                ResolvedStatementKind::Sip(sip) => {
                    mark_route(&sip.progress, routes)
                        && mark_route(&sip.success, routes)
                        && sip
                            .error
                            .as_ref()
                            .is_none_or(|route| mark_route(route, routes))
                }
                ResolvedStatementKind::TaskFlow(flow) => {
                    mark_source(&flow.source, tasks)
                        && flow.transforms.iter().all(|transform| {
                            let task = match transform {
                                ResolvedTaskTransform::Map { task, .. }
                                | ResolvedTaskTransform::Then { task, .. }
                                | ResolvedTaskTransform::AndThen { task, .. }
                                | ResolvedTaskTransform::MapError { task, .. }
                                | ResolvedTaskTransform::Collect { task }
                                | ResolvedTaskTransform::Discard { task } => *task,
                            };
                            mark(tasks, task.0)
                        })
                        && flow
                            .success
                            .as_ref()
                            .is_none_or(|route| mark_route(route, routes))
                        && flow
                            .error
                            .as_ref()
                            .is_none_or(|route| mark_route(route, routes))
                        && flow
                            .units
                            .as_ref()
                            .is_none_or(|route| mark_route(route, routes))
                }
                ResolvedStatementKind::TaskGroup {
                    statements: children,
                    ..
                } => children
                    .iter()
                    .all(|child| visit(child, statements, tasks, routes, run_sites)),
                ResolvedStatementKind::Abortable { task, .. } => {
                    visit(task, statements, tasks, routes, run_sites)
                }
                ResolvedStatementKind::WidgetOperation { route, .. }
                | ResolvedStatementKind::PaneOperation { route, .. }
                | ResolvedStatementKind::WindowOperation { route, .. } => {
                    route.as_ref().is_none_or(|route| mark_route(route, routes))
                }
                ResolvedStatementKind::Let { .. }
                | ResolvedStatementKind::Assign { .. }
                | ResolvedStatementKind::MarkdownAppend { .. }
                | ResolvedStatementKind::ComboPush { .. }
                | ResolvedStatementKind::ReturnIf { .. }
                | ResolvedStatementKind::Exit
                | ResolvedStatementKind::Abort { .. }
                | ResolvedStatementKind::DebugStart { .. }
                | ResolvedStatementKind::DebugFinish { .. }
                | ResolvedStatementKind::ClipboardWrite { .. } => true,
            }
        }

        let mut statements = vec![false; self.declarations.statement_count()];
        let mut tasks = vec![false; self.declarations.task_count()];
        let mut routes = vec![false; self.declarations.route_count()];
        let mut run_sites = vec![false; self.declarations.run_site_count()];
        let valid = self
            .handlers
            .iter()
            .flat_map(|handler| &handler.statements)
            .all(|statement| {
                visit(
                    statement,
                    &mut statements,
                    &mut tasks,
                    &mut routes,
                    &mut run_sites,
                )
            });
        if !valid
            || statements.iter().any(|seen| !seen)
            || tasks.iter().any(|seen| !seen)
            || routes.iter().any(|seen| !seen)
            || run_sites.iter().any(|seen| !seen)
        {
            return Err(self.invariant(
                &Span::line(1),
                "handler lowering did not consume every statement, task, route, and run-site ID",
            ));
        }
        Ok(())
    }

    fn lower_handler(
        &mut self,
        declaration_index: usize,
        handler: &Handler,
        expected_owner: HandlerOwner,
    ) -> Result<HandlerId, Error> {
        let declaration = self
            .declarations
            .handlers()
            .get(declaration_index)
            .ok_or_else(|| self.invariant(&handler.span, "handler has no HIR declaration"))?
            .clone();
        if declaration.name != handler.name || declaration.owner != expected_owner {
            return Err(self.invariant(
                &handler.span,
                "handler HIR declaration owner order changed after checking",
            ));
        }
        let id = declaration.declaration.id;
        if id.0 as usize != self.handlers.len() {
            return Err(self.invariant(&handler.span, "handler HIR arena is not preorder stable"));
        }
        let expected_origin_parent = match expected_owner {
            HandlerOwner::Component(component) => {
                Some(self.declarations.component(component.0 as usize).origin)
            }
            HandlerOwner::App | HandlerOwner::Preset(_) => None,
        };
        if self
            .origins
            .try_get(declaration.declaration.origin)
            .is_none_or(|origin| origin.parent != expected_origin_parent)
        {
            return Err(self.invariant(
                &handler.span,
                "handler HIR origin chain diverged from its owner",
            ));
        }
        let checked = self
            .facts
            .try_handler(id)
            .ok_or_else(|| {
                self.invariant(&handler.span, "handler checked ID is outside its arena")
            })?
            .clone();
        if checked.id != id || checked.params.len() != handler.params.len() {
            return Err(self.invariant(
                &handler.span,
                "checked handler facts do not belong to the lowered handler",
            ));
        }
        let raw_param_names = handler
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        let raw_param_types = handler
            .params
            .iter()
            .map(|param| param.ty.clone())
            .collect::<Vec<_>>();
        if raw_param_names != checked.param_names || raw_param_types != checked.param_types {
            return Err(self.invariant(
                &handler.span,
                "handler parameter contract changed after checking",
            ));
        }
        let params = handler
            .params
            .iter()
            .zip(checked.params)
            .map(|(param, local)| ResolvedHandlerParam {
                local,
                name: param.name.clone(),
                ty: param.ty.clone(),
            })
            .collect();
        if handler.statements.len() != declaration.statement_roots.len() {
            return Err(self.invariant(
                &handler.span,
                "handler statement HIR declaration count diverged",
            ));
        }
        let statements = handler
            .statements
            .iter()
            .zip(declaration.statement_roots.iter().copied())
            .map(|(source, statement)| {
                self.lower_handler_statement(source, statement, id, declaration.owner, None)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.handlers.push(ResolvedHandler {
            id,
            owner: declaration.owner,
            name: handler.name.clone(),
            params,
            statements,
            origin: declaration.declaration.origin,
        });
        Ok(id)
    }

    fn checked_statement_expression(
        &self,
        statement: StatementId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = crate::check::CheckedExprOwner::HandlerStatement {
            statement,
            operand: *operand,
        };
        *operand += 1;
        let expression = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
            self.invariant(span, "handler statement expression has no checked HIR fact")
        })?;
        self.validate_checked_expression_use(expression, span)?;
        Ok(expression)
    }

    fn checked_task_expression(
        &self,
        task: TaskId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let owner = crate::check::CheckedExprOwner::Task {
            task,
            operand: *operand,
        };
        *operand += 1;
        let expression = self
            .facts
            .expression_use_by_owner(owner)
            .ok_or_else(|| self.invariant(span, "task expression has no checked HIR fact"))?;
        self.validate_checked_expression_use(expression, span)?;
        Ok(expression)
    }

    fn validate_checked_expression_use(
        &self,
        expression: CheckedExprUseId,
        span: &Span,
    ) -> Result<(), Error> {
        let checked = self.facts.try_expression_use(expression).ok_or_else(|| {
            self.invariant(span, "checked expression-use ID is outside its arena")
        })?;
        if self.facts.try_expression(checked.root).is_none() {
            return Err(self.invariant(span, "checked expression root ID is outside its arena"));
        }
        Ok(())
    }

    fn ensure_task_operands_consumed(
        &self,
        task: TaskId,
        operand: u32,
        span: &Span,
    ) -> Result<(), Error> {
        if self
            .facts
            .expression_use_by_owner(crate::check::CheckedExprOwner::Task { task, operand })
            .is_some()
        {
            return Err(self.invariant(
                span,
                "task did not consume its complete checked operand contract",
            ));
        }
        Ok(())
    }

    fn writable_state(
        &self,
        statement: &crate::check::CheckedStatement,
        index: &mut usize,
        span: &Span,
    ) -> Result<ResolvedWritableState, Error> {
        let value_ref = statement
            .writable_targets
            .get(*index)
            .copied()
            .ok_or_else(|| {
                self.invariant(
                    span,
                    "writable handler target has no checked state contract",
                )
            })?;
        *index += 1;
        let value = self.facts.try_value_by_ref(value_ref).ok_or_else(|| {
            self.invariant(span, "writable state value ID is outside its checked arena")
        })?;
        Ok(ResolvedWritableState {
            value: value.id,
            name: value.name.clone(),
            ty: value.ty.clone(),
        })
    }

    fn effect_target(
        &self,
        task: TaskId,
        function: &str,
        kind: EffectKind,
        span: &Span,
    ) -> Result<ResolvedEffectTarget, Error> {
        let checked = self.facts.try_task(task).ok_or_else(|| {
            self.invariant(span, "effect target task ID is outside its checked arena")
        })?;
        match checked.target.as_ref().ok_or_else(|| {
            self.invariant(span, "effect task has no authoritative checked target")
        })? {
            crate::check::CheckedEffectTarget::Builtin(name) => {
                if name != function {
                    return Err(
                        self.invariant(span, "built-in effect target changed after checking")
                    );
                }
                Ok(ResolvedEffectTarget::Builtin(name.clone()))
            }
            crate::check::CheckedEffectTarget::Extern(id) => {
                let declaration = self.declarations.try_extern_decl(*id).ok_or_else(|| {
                    self.invariant(span, "effect extern target ID is outside its arena")
                })?;
                if declaration.name != function || declaration.kind != ExternKind::from(kind) {
                    return Err(self.invariant(span, "effect extern target changed after checking"));
                }
                Ok(ResolvedEffectTarget::Extern(*id))
            }
        }
    }

    fn sip_target(&self, task: TaskId, function: &str, span: &Span) -> Result<ExternFnId, Error> {
        let checked = self.facts.try_task(task).ok_or_else(|| {
            self.invariant(span, "sip target task ID is outside its checked arena")
        })?;
        let Some(crate::check::CheckedEffectTarget::Extern(id)) = &checked.target else {
            return Err(self.invariant(span, "sip task has no authoritative extern target"));
        };
        let declaration = self
            .declarations
            .try_extern_decl(*id)
            .ok_or_else(|| self.invariant(span, "sip extern target ID is outside its arena"))?;
        if declaration.name != function || declaration.kind != ExternKind::Sip {
            return Err(self.invariant(span, "sip extern target changed after checking"));
        }
        Ok(*id)
    }

    pub(crate) fn lower_route(
        &self,
        route: &Route,
        id: RouteId,
        owner: HandlerOwner,
        statement: StatementId,
        task: Option<TaskId>,
    ) -> Result<ResolvedRoute, Error> {
        self.lower_handler_route(route, id, owner, statement, task, false)
    }

    fn lower_handler_route(
        &self,
        route: &Route,
        id: RouteId,
        owner: HandlerOwner,
        statement: StatementId,
        task: Option<TaskId>,
        ordered: bool,
    ) -> Result<ResolvedRoute, Error> {
        let declaration = self.declarations.try_route(id).ok_or_else(|| {
            self.invariant(&route.span, "route HIR ID is outside its declaration arena")
        })?;
        let statement_declaration =
            self.declarations.try_statement(statement).ok_or_else(|| {
                self.invariant(&route.span, "route statement ID is outside its arena")
            })?;
        if declaration.declaration.id != id
            || declaration.statement != statement
            || declaration.task != task
            || self
                .origins
                .try_get(declaration.declaration.origin)
                .is_none_or(|origin| {
                    origin.parent != Some(statement_declaration.declaration.origin)
                })
        {
            return Err(self.invariant(
                &route.span,
                "route HIR owner or origin chain diverged from its statement",
            ));
        }
        let checked = self.facts.try_route(id).ok_or_else(|| {
            self.invariant(&route.span, "route checked ID is outside its fact arena")
        })?;
        let expected_target_owner = match owner {
            HandlerOwner::Preset(_) => HandlerOwner::App,
            owner => owner,
        };
        let raw_arg_kinds = route
            .args
            .iter()
            .map(|arg| match arg {
                RouteArg::Expr(_) => crate::check::CheckedRouteArgKind::Expression,
                RouteArg::Payload => crate::check::CheckedRouteArgKind::Payload,
            })
            .collect::<Vec<_>>();
        if checked.id != id
            || checked.origin != declaration.declaration.origin
            || checked.target_owner != expected_target_owner
            || checked.args != raw_arg_kinds
            || checked.ordered_payloads != ordered
        {
            return Err(self.invariant(
                &route.span,
                "route semantic contract changed after checking",
            ));
        }
        let target_handler = checked.target;
        let handler = self
            .declarations
            .try_handler(target_handler)
            .ok_or_else(|| {
                self.invariant(&route.span, "route target handler ID is outside its arena")
            })?;
        if handler.name != route.handler || handler.owner != checked.target_owner {
            return Err(self.invariant(&route.span, "route target changed after checking"));
        }
        let target = match checked.target_owner {
            HandlerOwner::Component(component) => ResolvedRouteTarget::Component {
                component,
                handler: target_handler,
                name: handler.name.clone(),
            },
            HandlerOwner::App => ResolvedRouteTarget::App {
                handler: target_handler,
                name: handler.name.clone(),
            },
            HandlerOwner::Preset(_) => {
                return Err(self.invariant(&route.span, "route cannot target a preset handler"));
            }
        };
        let target_params = self
            .facts
            .try_handler(target_handler)
            .ok_or_else(|| {
                self.invariant(
                    &route.span,
                    "route target checked handler ID is outside its arena",
                )
            })?
            .params
            .iter()
            .map(|local| {
                self.facts
                    .try_local(*local)
                    .map(|local| local.ty.clone())
                    .ok_or_else(|| {
                        self.invariant(
                            &route.span,
                            "route target parameter local ID is outside its arena",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let args = lower_typed_route_arguments(
            route,
            &target_params,
            TypedRouteInputs {
                source_payloads: &checked.source_payloads,
                ordered,
            },
            |index| {
                let expression = self
                    .facts
                    .expression_use_by_owner(crate::check::CheckedExprOwner::Route {
                        route: id,
                        argument: index as u32,
                    })
                    .ok_or_else(|| {
                        self.invariant(&route.span, "route argument has no checked expression fact")
                    })?;
                self.validate_checked_expression_use(expression, &route.span)?;
                Ok(expression)
            },
        )?;
        Ok(ResolvedRoute {
            id,
            target,
            args,
            origin: declaration.declaration.origin,
        })
    }

    pub(crate) fn lower_ordered_route(
        &self,
        route: &Route,
        id: RouteId,
        owner: HandlerOwner,
        statement: StatementId,
        task: Option<TaskId>,
    ) -> Result<ResolvedRoute, Error> {
        self.lower_handler_route(route, id, owner, statement, task, true)
    }

    fn lower_handler_statement(
        &self,
        statement: &Statement,
        id: StatementId,
        handler: HandlerId,
        owner: HandlerOwner,
        parent: Option<StatementId>,
    ) -> Result<ResolvedStatement, Error> {
        let declaration = self.declarations.try_statement(id).ok_or_else(|| {
            self.invariant(
                statement.span(),
                "statement HIR ID is outside its declaration arena",
            )
        })?;
        let checked_statement = self.facts.try_statement(id).ok_or_else(|| {
            self.invariant(
                statement.span(),
                "statement checked ID is outside its fact arena",
            )
        })?;
        if checked_statement.id != id
            || checked_statement.origin != declaration.declaration.origin
            || checked_statement.semantic_key != crate::hir::statement_semantic_key(statement)
            || checked_statement.operation != crate::hir::handler_operation_contract(statement)
        {
            return Err(self.invariant(
                statement.span(),
                "handler statement semantic contract changed after checking",
            ));
        }
        let declaration_handler = self
            .declarations
            .try_handler(declaration.handler)
            .ok_or_else(|| {
                self.invariant(
                    statement.span(),
                    "statement handler ID is outside its arena",
                )
            })?;
        if declaration.declaration.id != id
            || declaration.handler != handler
            || declaration.parent != parent
            || declaration_handler.owner != owner
        {
            return Err(self.invariant(
                statement.span(),
                "statement HIR owner or preorder parent does not match its handler",
            ));
        }
        let expected_origin_parent = match parent {
            Some(parent) => {
                self.declarations
                    .try_statement(parent)
                    .ok_or_else(|| {
                        self.invariant(statement.span(), "statement parent ID is outside its arena")
                    })?
                    .declaration
                    .origin
            }
            None => declaration_handler.declaration.origin,
        };
        if self
            .origins
            .try_get(declaration.declaration.origin)
            .is_none_or(|origin| origin.parent != Some(expected_origin_parent))
        {
            return Err(self.invariant(
                statement.span(),
                "statement HIR origin chain diverged from its preorder parent",
            ));
        }
        if statement.immediate_task().is_some() != declaration.task.is_some() {
            return Err(self.invariant(
                statement.span(),
                "statement task declaration shape changed after checking",
            ));
        }
        if let Some(task) = declaration.task {
            let task_declaration = self.declarations.try_task(task).ok_or_else(|| {
                self.invariant(statement.span(), "statement task ID is outside its arena")
            })?;
            if task_declaration.declaration.id != task
                || task_declaration.statement != id
                || task_declaration.parent.is_some()
                || self
                    .origins
                    .try_get(task_declaration.declaration.origin)
                    .is_none_or(|origin| origin.parent != Some(declaration.declaration.origin))
            {
                return Err(self.invariant(
                    statement.span(),
                    "statement task HIR owner or origin chain diverged",
                ));
            }
        }
        let mut operand = 0u32;
        let mut writable = 0usize;
        let mut routes = declaration.routes.iter().copied();
        let kind = match statement {
            Statement::Let {
                name,
                value: _,
                span,
            } => {
                let value = self.checked_statement_expression(id, &mut operand, span)?;
                let local = self
                    .facts
                    .local_by_owner(crate::check::CheckedLocalOwner::StatementLet(id))
                    .ok_or_else(|| self.invariant(span, "let statement has no checked local"))?;
                ResolvedStatementKind::Let {
                    local,
                    name: name.clone(),
                    ty: self
                        .facts
                        .try_local(local)
                        .ok_or_else(|| self.invariant(span, "let local ID is outside its arena"))?
                        .ty
                        .clone(),
                    value,
                }
            }
            Statement::Assign {
                target: _,
                value: _,
                at,
                span,
            } => {
                let target = self.writable_state(checked_statement, &mut writable, span)?;
                let value = self.checked_statement_expression(id, &mut operand, span)?;
                let move_self = checked_statement.editor_self_move.ok_or_else(|| {
                    self.invariant(span, "assignment has no checked editor move contract")
                })?;
                ResolvedStatementKind::Assign {
                    target,
                    value,
                    at: at
                        .as_ref()
                        .map(|_| self.checked_statement_expression(id, &mut operand, span))
                        .transpose()?,
                    move_self,
                }
            }
            Statement::MarkdownAppend {
                target: _, span, ..
            } => ResolvedStatementKind::MarkdownAppend {
                target: self.writable_state(checked_statement, &mut writable, span)?,
                value: self.checked_statement_expression(id, &mut operand, span)?,
            },
            Statement::ComboPush {
                target: _, span, ..
            } => ResolvedStatementKind::ComboPush {
                target: self.writable_state(checked_statement, &mut writable, span)?,
                value: self.checked_statement_expression(id, &mut operand, span)?,
            },
            Statement::ReturnIf { span, .. } => ResolvedStatementKind::ReturnIf {
                condition: self.checked_statement_expression(id, &mut operand, span)?,
            },
            Statement::Exit { .. } => ResolvedStatementKind::Exit,
            Statement::Run {
                kind,
                mode,
                function,
                args,
                success,
                error,
                span,
            } => {
                let task = declaration.task.ok_or_else(|| {
                    self.invariant(span, "run statement has no normalized task ID")
                })?;
                let mut task_operand = 0;
                let args = args
                    .iter()
                    .map(|_| self.checked_task_expression(task, &mut task_operand, span))
                    .collect::<Result<Vec<_>, _>>()?;
                self.ensure_task_operands_consumed(task, task_operand, span)?;
                let success_id = routes.next().ok_or_else(|| {
                    self.invariant(span, "run success route has no normalized ID")
                })?;
                let success = self.lower_route(success, success_id, owner, id, declaration.task)?;
                let error = error
                    .as_ref()
                    .map(|route| {
                        let route_id = routes.next().ok_or_else(|| {
                            self.invariant(span, "run error route has no normalized ID")
                        })?;
                        self.lower_route(route, route_id, owner, id, declaration.task)
                    })
                    .transpose()?;
                let site = declaration.run_site;
                if (*mode == FutureMode::Every) != site.is_none() {
                    return Err(self.invariant(span, "run mode and stable run-site ID diverged"));
                }
                if let Some(site) = site {
                    let run_site = self.declarations.try_run_site(site).ok_or_else(|| {
                        self.invariant(span, "stable run-site ID is outside its arena")
                    })?;
                    if run_site.declaration.id != site
                        || run_site.statement != id
                        || run_site.mode != *mode
                        || run_site.declaration.origin != declaration.declaration.origin
                    {
                        return Err(self.invariant(
                            span,
                            "stable run-site HIR owner, mode, or origin diverged",
                        ));
                    }
                }
                ResolvedStatementKind::Run(ResolvedRun {
                    kind: *kind,
                    mode: *mode,
                    site,
                    target: self.effect_target(task, function, *kind, span)?,
                    args,
                    success,
                    error,
                })
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
                    self.invariant(span, "sip statement has no normalized task ID")
                })?;
                let mut task_operand = 0;
                let args = args
                    .iter()
                    .map(|_| self.checked_task_expression(task, &mut task_operand, span))
                    .collect::<Result<Vec<_>, _>>()?;
                self.ensure_task_operands_consumed(task, task_operand, span)?;
                let mut route = |source: &Route| -> Result<ResolvedRoute, Error> {
                    let route_id = routes
                        .next()
                        .ok_or_else(|| self.invariant(span, "sip route has no normalized ID"))?;
                    self.lower_route(source, route_id, owner, id, declaration.task)
                };
                ResolvedStatementKind::Sip(ResolvedSip {
                    target: self.sip_target(task, function, span)?,
                    args,
                    progress: route(progress)?,
                    success: route(success)?,
                    error: error.as_ref().map(&mut route).transpose()?,
                })
            }
            Statement::TaskFlow {
                source,
                transforms,
                success,
                error,
                units,
                span,
            } => {
                let task = declaration.task.ok_or_else(|| {
                    self.invariant(span, "task flow has no normalized root task ID")
                })?;
                if declaration.source_tasks.len() != transforms.len() + 1 {
                    return Err(self.invariant(span, "task flow task arena shape diverged"));
                }
                for source_task in &declaration.source_tasks {
                    let task_declaration =
                        self.declarations.try_task(*source_task).ok_or_else(|| {
                            self.invariant(span, "task-flow child task ID is outside its arena")
                        })?;
                    let root_task_declaration =
                        self.declarations.try_task(task).ok_or_else(|| {
                            self.invariant(span, "task-flow root task ID is outside its arena")
                        })?;
                    if task_declaration.declaration.id != *source_task
                        || task_declaration.statement != id
                        || task_declaration.parent != Some(task)
                        || self
                            .origins
                            .try_get(task_declaration.declaration.origin)
                            .is_none_or(|origin| {
                                origin.parent != Some(root_task_declaration.declaration.origin)
                            })
                    {
                        return Err(self.invariant(
                            span,
                            "task flow child HIR owner or origin chain diverged",
                        ));
                    }
                }
                let source = self.lower_task_source(source, declaration.source_tasks[0])?;
                let transforms = transforms
                    .iter()
                    .enumerate()
                    .map(|(index, transform)| {
                        self.lower_task_transform(
                            transform,
                            declaration.source_tasks[index + 1],
                            index,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut route = |source: &Route| -> Result<ResolvedRoute, Error> {
                    let route_id = routes.next().ok_or_else(|| {
                        self.invariant(span, "task flow route has no normalized ID")
                    })?;
                    self.lower_route(source, route_id, owner, id, declaration.task)
                };
                let checked = self.facts.try_task(task).ok_or_else(|| {
                    self.invariant(span, "task-flow checked task ID is outside its arena")
                })?;
                if checked.id != task {
                    return Err(self.invariant(span, "task flow checked owner mismatch"));
                }
                ResolvedStatementKind::TaskFlow(ResolvedTaskFlow {
                    source,
                    transforms,
                    output: checked.output.clone(),
                    error_type: checked.error.clone(),
                    success: success.as_ref().map(&mut route).transpose()?,
                    error: error.as_ref().map(&mut route).transpose()?,
                    units: units.as_ref().map(&mut route).transpose()?,
                })
            }
            Statement::TaskGroup {
                kind,
                statements,
                span,
            } => {
                if statements.len() != declaration.children.len() {
                    return Err(self.invariant(span, "task group HIR child count diverged"));
                }
                ResolvedStatementKind::TaskGroup {
                    kind: *kind,
                    statements: statements
                        .iter()
                        .zip(declaration.children.iter().copied())
                        .map(|(child, child_id)| {
                            self.lower_handler_statement(child, child_id, handler, owner, Some(id))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                }
            }
            Statement::Abortable {
                handle: _,
                abort_on_drop,
                task,
                span,
            } => {
                let [child] = declaration.children.as_slice() else {
                    return Err(self.invariant(span, "abortable HIR child count diverged"));
                };
                ResolvedStatementKind::Abortable {
                    handle: self.writable_state(checked_statement, &mut writable, span)?,
                    abort_on_drop: *abort_on_drop,
                    task: Box::new(self.lower_handler_statement(
                        task,
                        *child,
                        handler,
                        owner,
                        Some(id),
                    )?),
                }
            }
            Statement::Abort { handle: _, span } => ResolvedStatementKind::Abort {
                handle: self.writable_state(checked_statement, &mut writable, span)?,
            },
            Statement::DebugStart {
                target: _, span, ..
            } => ResolvedStatementKind::DebugStart {
                name: self.checked_statement_expression(id, &mut operand, span)?,
                target: self.writable_state(checked_statement, &mut writable, span)?,
            },
            Statement::DebugFinish { target: _, span } => ResolvedStatementKind::DebugFinish {
                target: self.writable_state(checked_statement, &mut writable, span)?,
            },
            Statement::ClipboardWrite {
                primary,
                value: _,
                span,
            } => ResolvedStatementKind::ClipboardWrite {
                primary: *primary,
                value: self.checked_statement_expression(id, &mut operand, span)?,
            },
            Statement::WidgetOperation {
                operation,
                route,
                span,
            } => ResolvedStatementKind::WidgetOperation {
                operation: self.lower_widget_operation(operation, id, &mut operand, span)?,
                route: route
                    .as_ref()
                    .map(|route| {
                        self.lower_route(
                            route,
                            routes.next().ok_or_else(|| {
                                self.invariant(span, "widget route has no normalized ID")
                            })?,
                            owner,
                            id,
                            declaration.task,
                        )
                    })
                    .transpose()?,
            },
            Statement::PaneOperation {
                grid,
                operation,
                route,
                span,
            } => ResolvedStatementKind::PaneOperation {
                grid: grid.clone(),
                dynamic: checked_statement.pane_grid_dynamic.ok_or_else(|| {
                    self.invariant(span, "pane operation has no checked grid mode contract")
                })?,
                operation: self.lower_pane_operation(operation, id, &mut operand, span)?,
                route: route
                    .as_ref()
                    .map(|route| {
                        self.lower_route(
                            route,
                            routes.next().ok_or_else(|| {
                                self.invariant(span, "pane route has no normalized ID")
                            })?,
                            owner,
                            id,
                            declaration.task,
                        )
                    })
                    .transpose()?,
            },
            Statement::WindowOperation {
                operation,
                target,
                route,
                span,
            } => {
                let target = target
                    .as_ref()
                    .map(|_| self.checked_statement_expression(id, &mut operand, span))
                    .transpose()?;
                ResolvedStatementKind::WindowOperation {
                    operation: self.lower_window_operation(operation, id, &mut operand, span)?,
                    target,
                    route: route
                        .as_ref()
                        .map(|route| {
                            let route_id = routes.next().ok_or_else(|| {
                                self.invariant(span, "window route has no normalized ID")
                            })?;
                            if matches!(
                                operation,
                                WindowOperation::Size
                                    | WindowOperation::Position
                                    | WindowOperation::MonitorSize
                            ) {
                                self.lower_ordered_route(
                                    route,
                                    route_id,
                                    owner,
                                    id,
                                    declaration.task,
                                )
                            } else {
                                self.lower_route(route, route_id, owner, id, declaration.task)
                            }
                        })
                        .transpose()?,
                }
            }
        };
        if routes.next().is_some() {
            return Err(self.invariant(
                statement.span(),
                "statement HIR left a route declaration unconsumed",
            ));
        }
        if operand != checked_statement.operand_count
            || writable != checked_statement.writable_targets.len()
            || self
                .facts
                .expression_use_by_owner(crate::check::CheckedExprOwner::HandlerStatement {
                    statement: id,
                    operand,
                })
                .is_some()
        {
            return Err(self.invariant(
                statement.span(),
                "handler statement did not consume its complete checked operand contract",
            ));
        }
        if let Some(task) = declaration.task {
            let checked = self.facts.try_task(task).ok_or_else(|| {
                self.invariant(
                    statement.span(),
                    "statement checked task ID is outside its arena",
                )
            })?;
            if checked.id != task || checked.is_final != declaration.is_final {
                return Err(self.invariant(
                    statement.span(),
                    "statement task finality or owner changed after checking",
                ));
            }
        }
        Ok(ResolvedStatement {
            id,
            kind,
            task: declaration.task,
            is_final: declaration.is_final,
            origin: declaration.declaration.origin,
        })
    }

    fn lower_task_source(
        &self,
        source: &TaskSource,
        task: TaskId,
    ) -> Result<ResolvedTaskSource, Error> {
        if self
            .facts
            .try_task(task)
            .is_none_or(|checked| checked.id != task)
        {
            return Err(self.invariant(
                match source {
                    TaskSource::Effect { span, .. }
                    | TaskSource::Done { span, .. }
                    | TaskSource::None { span, .. } => span,
                },
                "task source checked owner mismatch",
            ));
        }
        let mut operand = 0;
        let resolved = match source {
            TaskSource::Effect {
                kind,
                function,
                args,
                span,
            } => ResolvedTaskSource::Effect {
                task,
                kind: *kind,
                target: self.effect_target(task, function, *kind, span)?,
                args: args
                    .iter()
                    .map(|_| self.checked_task_expression(task, &mut operand, span))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            TaskSource::Done { span, .. } => ResolvedTaskSource::Done {
                task,
                value: self.checked_task_expression(task, &mut operand, span)?,
            },
            TaskSource::None { output, .. } => ResolvedTaskSource::None {
                task,
                output: output.clone(),
            },
        };
        let span = match source {
            TaskSource::Effect { span, .. }
            | TaskSource::Done { span, .. }
            | TaskSource::None { span, .. } => span,
        };
        self.ensure_task_operands_consumed(task, operand, span)?;
        Ok(resolved)
    }

    fn lower_task_transform(
        &self,
        transform: &TaskTransform,
        task: TaskId,
        index: usize,
    ) -> Result<ResolvedTaskTransform, Error> {
        let local = |span: &Span| {
            self.facts
                .local_by_owner(crate::check::CheckedLocalOwner::TaskTransform {
                    task,
                    index: index as u32,
                })
                .ok_or_else(|| self.invariant(span, "task transform has no checked local"))
        };
        Ok(match transform {
            TaskTransform::Map {
                binding,
                value: _,
                span,
            } => {
                let local = local(span)?;
                let mut operand = 0;
                let value = self.checked_task_expression(task, &mut operand, span)?;
                self.ensure_task_operands_consumed(task, operand, span)?;
                ResolvedTaskTransform::Map {
                    task,
                    local,
                    binding: binding.clone(),
                    input: self
                        .facts
                        .try_local(local)
                        .ok_or_else(|| self.invariant(span, "map local ID is outside its arena"))?
                        .ty
                        .clone(),
                    input_fallible: self
                        .facts
                        .try_task(task)
                        .ok_or_else(|| {
                            self.invariant(span, "map checked task ID is outside its arena")
                        })?
                        .error
                        .is_some(),
                    value,
                }
            }
            TaskTransform::Then {
                binding,
                source,
                span,
            } => {
                let local = local(span)?;
                ResolvedTaskTransform::Then {
                    task,
                    local,
                    binding: binding.clone(),
                    input: self
                        .facts
                        .try_local(local)
                        .ok_or_else(|| self.invariant(span, "then local ID is outside its arena"))?
                        .ty
                        .clone(),
                    source: self.lower_task_source(source, task)?,
                }
            }
            TaskTransform::AndThen {
                binding,
                source,
                span,
            } => {
                let local = local(span)?;
                ResolvedTaskTransform::AndThen {
                    task,
                    local,
                    binding: binding.clone(),
                    input: self
                        .facts
                        .try_local(local)
                        .ok_or_else(|| self.invariant(span, "try local ID is outside its arena"))?
                        .ty
                        .clone(),
                    source: self.lower_task_source(source, task)?,
                }
            }
            TaskTransform::MapError {
                binding,
                value: _,
                span,
            } => {
                let local = local(span)?;
                let mut operand = 0;
                let value = self.checked_task_expression(task, &mut operand, span)?;
                self.ensure_task_operands_consumed(task, operand, span)?;
                ResolvedTaskTransform::MapError {
                    task,
                    local,
                    binding: binding.clone(),
                    input: self
                        .facts
                        .try_local(local)
                        .ok_or_else(|| {
                            self.invariant(span, "map-error local ID is outside its arena")
                        })?
                        .ty
                        .clone(),
                    value,
                }
            }
            TaskTransform::Collect { .. } => ResolvedTaskTransform::Collect { task },
            TaskTransform::Discard { .. } => ResolvedTaskTransform::Discard { task },
        })
    }

    fn lower_widget_target(
        &self,
        target: &WidgetTarget,
        statement: StatementId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<ResolvedWidgetTarget, Error> {
        Ok(ResolvedWidgetTarget {
            segments: target
                .segments
                .iter()
                .map(|segment| {
                    Ok(ResolvedWidgetTargetSegment {
                        name: segment.name.clone(),
                        key: segment
                            .key
                            .as_ref()
                            .map(|_| self.checked_statement_expression(statement, operand, span))
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?,
        })
    }

    fn lower_widget_selector(
        &self,
        selector: &WidgetSelector,
        statement: StatementId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<ResolvedWidgetSelector, Error> {
        Ok(match selector {
            WidgetSelector::Id(target) => ResolvedWidgetSelector::Id(
                self.lower_widget_target(target, statement, operand, span)?,
            ),
            WidgetSelector::Text(_) => ResolvedWidgetSelector::Text(
                self.checked_statement_expression(statement, operand, span)?,
            ),
            WidgetSelector::Point { .. } => ResolvedWidgetSelector::Point {
                x: self.checked_statement_expression(statement, operand, span)?,
                y: self.checked_statement_expression(statement, operand, span)?,
            },
            WidgetSelector::Focused => ResolvedWidgetSelector::Focused,
            WidgetSelector::Extern { function, args } => ResolvedWidgetSelector::Extern {
                target: self
                    .declarations
                    .extern_decl_by_name(function)
                    .ok_or_else(|| self.invariant(span, "widget selector extern is unresolved"))?
                    .declaration
                    .id,
                args: args
                    .iter()
                    .map(|_| self.checked_statement_expression(statement, operand, span))
                    .collect::<Result<Vec<_>, _>>()?,
            },
        })
    }

    fn lower_widget_operation(
        &self,
        operation: &WidgetOperation,
        statement: StatementId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<ResolvedWidgetOperation, Error> {
        let target = |target: &WidgetTarget, operand: &mut u32| {
            self.lower_widget_target(target, statement, operand, span)
        };
        Ok(match operation {
            WidgetOperation::FocusPrevious => ResolvedWidgetOperation::FocusPrevious,
            WidgetOperation::FocusNext => ResolvedWidgetOperation::FocusNext,
            WidgetOperation::Focus { target: value } => ResolvedWidgetOperation::Focus {
                target: target(value, operand)?,
            },
            WidgetOperation::Focused { target: value } => ResolvedWidgetOperation::Focused {
                target: target(value, operand)?,
            },
            WidgetOperation::CursorFront { target: value } => {
                ResolvedWidgetOperation::CursorFront {
                    target: target(value, operand)?,
                }
            }
            WidgetOperation::CursorEnd { target: value } => ResolvedWidgetOperation::CursorEnd {
                target: target(value, operand)?,
            },
            WidgetOperation::Cursor { target: value, .. } => ResolvedWidgetOperation::Cursor {
                target: target(value, operand)?,
                position: self.checked_statement_expression(statement, operand, span)?,
            },
            WidgetOperation::SelectAll { target: value } => ResolvedWidgetOperation::SelectAll {
                target: target(value, operand)?,
            },
            WidgetOperation::Select { target: value, .. } => ResolvedWidgetOperation::Select {
                target: target(value, operand)?,
                start: self.checked_statement_expression(statement, operand, span)?,
                end: self.checked_statement_expression(statement, operand, span)?,
            },
            WidgetOperation::Snap { target: value, .. } => ResolvedWidgetOperation::Snap {
                target: target(value, operand)?,
                x: self.checked_statement_expression(statement, operand, span)?,
                y: self.checked_statement_expression(statement, operand, span)?,
            },
            WidgetOperation::SnapEnd { target: value } => ResolvedWidgetOperation::SnapEnd {
                target: target(value, operand)?,
            },
            WidgetOperation::ScrollTo { target: value, .. } => ResolvedWidgetOperation::ScrollTo {
                target: target(value, operand)?,
                x: self.checked_statement_expression(statement, operand, span)?,
                y: self.checked_statement_expression(statement, operand, span)?,
            },
            WidgetOperation::ScrollBy { target: value, .. } => ResolvedWidgetOperation::ScrollBy {
                target: target(value, operand)?,
                x: self.checked_statement_expression(statement, operand, span)?,
                y: self.checked_statement_expression(statement, operand, span)?,
            },
            WidgetOperation::Find { selector, all } => ResolvedWidgetOperation::Find {
                selector: self.lower_widget_selector(selector, statement, operand, span)?,
                all: *all,
            },
        })
    }

    fn lower_pane_reference(
        &self,
        pane: &PaneReference,
        statement: StatementId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<ResolvedPaneReference, Error> {
        Ok(match pane {
            PaneReference::Static(name) => ResolvedPaneReference::Static(name.clone()),
            PaneReference::Dynamic { template, .. } => ResolvedPaneReference::Dynamic {
                template: template.clone(),
                key: self.checked_statement_expression(statement, operand, span)?,
            },
        })
    }

    fn lower_pane_operation(
        &self,
        operation: &PaneOperation,
        statement: StatementId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<ResolvedPaneOperation, Error> {
        let pane = |value: &PaneReference, operand: &mut u32| {
            self.lower_pane_reference(value, statement, operand, span)
        };
        Ok(match operation {
            PaneOperation::Maximize { pane: value } => ResolvedPaneOperation::Maximize {
                pane: pane(value, operand)?,
            },
            PaneOperation::Restore => ResolvedPaneOperation::Restore,
            PaneOperation::Maximized => ResolvedPaneOperation::Maximized,
            PaneOperation::Adjacent { pane: value, edge } => ResolvedPaneOperation::Adjacent {
                pane: pane(value, operand)?,
                edge: *edge,
            },
            PaneOperation::Swap { first, second } => ResolvedPaneOperation::Swap {
                first: pane(first, operand)?,
                second: pane(second, operand)?,
            },
            PaneOperation::Close { pane: value } => ResolvedPaneOperation::Close {
                pane: pane(value, operand)?,
            },
            PaneOperation::Move { pane: value, edge } => ResolvedPaneOperation::Move {
                pane: pane(value, operand)?,
                edge: *edge,
            },
            PaneOperation::Resize { split, .. } => ResolvedPaneOperation::Resize {
                split: split.clone(),
                ratio: self.checked_statement_expression(statement, operand, span)?,
            },
            PaneOperation::Drop {
                pane: value,
                target,
                edge,
            } => ResolvedPaneOperation::Drop {
                pane: pane(value, operand)?,
                target: pane(target, operand)?,
                edge: *edge,
            },
            PaneOperation::Split {
                target,
                pane: value,
                axis,
                ..
            } => ResolvedPaneOperation::Split {
                target: pane(target, operand)?,
                pane: pane(value, operand)?,
                axis: *axis,
                ratio: self.checked_statement_expression(statement, operand, span)?,
            },
        })
    }

    fn lower_window_operation(
        &self,
        operation: &WindowOperation,
        statement: StatementId,
        operand: &mut u32,
        span: &Span,
    ) -> Result<ResolvedWindowOperation, Error> {
        let expression =
            |operand: &mut u32| self.checked_statement_expression(statement, operand, span);
        let pair = |operand: &mut u32| -> Result<_, Error> {
            Ok((expression(operand)?, expression(operand)?))
        };
        Ok(match operation {
            WindowOperation::Open(window) => ResolvedWindowOperation::Open(
                window
                    .as_ref()
                    .map(|name| {
                        self.document
                            .settings
                            .windows
                            .iter()
                            .position(|window| window.name == *name)
                            .map(|index| index as u32)
                            .ok_or_else(|| {
                                self.invariant(span, "named window operation target is unresolved")
                            })
                    })
                    .transpose()?,
            ),
            WindowOperation::Oldest => ResolvedWindowOperation::Oldest,
            WindowOperation::Latest => ResolvedWindowOperation::Latest,
            WindowOperation::Close => ResolvedWindowOperation::Close,
            WindowOperation::Drag => ResolvedWindowOperation::Drag,
            WindowOperation::DragResize(direction) => {
                ResolvedWindowOperation::DragResize(*direction)
            }
            WindowOperation::Resize(_, _) => {
                let (width, height) = pair(operand)?;
                ResolvedWindowOperation::Resize(width, height)
            }
            WindowOperation::Resizable(_) => {
                ResolvedWindowOperation::Resizable(expression(operand)?)
            }
            WindowOperation::MinSize(value) => {
                ResolvedWindowOperation::MinSize(value.as_ref().map(|_| pair(operand)).transpose()?)
            }
            WindowOperation::MaxSize(value) => {
                ResolvedWindowOperation::MaxSize(value.as_ref().map(|_| pair(operand)).transpose()?)
            }
            WindowOperation::ResizeIncrements(value) => ResolvedWindowOperation::ResizeIncrements(
                value.as_ref().map(|_| pair(operand)).transpose()?,
            ),
            WindowOperation::Size => ResolvedWindowOperation::Size,
            WindowOperation::IsMaximized => ResolvedWindowOperation::IsMaximized,
            WindowOperation::Maximize(_) => ResolvedWindowOperation::Maximize(expression(operand)?),
            WindowOperation::IsMinimized => ResolvedWindowOperation::IsMinimized,
            WindowOperation::Minimize(_) => ResolvedWindowOperation::Minimize(expression(operand)?),
            WindowOperation::Position => ResolvedWindowOperation::Position,
            WindowOperation::ScaleFactor => ResolvedWindowOperation::ScaleFactor,
            WindowOperation::Move(_, _) => {
                let (x, y) = pair(operand)?;
                ResolvedWindowOperation::Move(x, y)
            }
            WindowOperation::Mode => ResolvedWindowOperation::Mode,
            WindowOperation::SetMode(mode) => ResolvedWindowOperation::SetMode(*mode),
            WindowOperation::ToggleMaximize => ResolvedWindowOperation::ToggleMaximize,
            WindowOperation::ToggleDecorations => ResolvedWindowOperation::ToggleDecorations,
            WindowOperation::Attention(attention) => ResolvedWindowOperation::Attention(*attention),
            WindowOperation::Focus => ResolvedWindowOperation::Focus,
            WindowOperation::SetLevel(level) => ResolvedWindowOperation::SetLevel(*level),
            WindowOperation::SystemMenu => ResolvedWindowOperation::SystemMenu,
            WindowOperation::RawId => ResolvedWindowOperation::RawId,
            WindowOperation::Screenshot => ResolvedWindowOperation::Screenshot,
            WindowOperation::MousePassthrough(_) => {
                ResolvedWindowOperation::MousePassthrough(expression(operand)?)
            }
            WindowOperation::MonitorSize => ResolvedWindowOperation::MonitorSize,
            WindowOperation::AutomaticTabbing(_) => {
                ResolvedWindowOperation::AutomaticTabbing(expression(operand)?)
            }
            WindowOperation::Icon { .. } => ResolvedWindowOperation::Icon {
                pixels: expression(operand)?,
                width: expression(operand)?,
                height: expression(operand)?,
            },
            WindowOperation::Callback { function, args } => ResolvedWindowOperation::Callback {
                target: self
                    .declarations
                    .extern_decl_by_name(function)
                    .ok_or_else(|| self.invariant(span, "window callback extern is unresolved"))?
                    .declaration
                    .id,
                args: args
                    .iter()
                    .map(|_| expression(operand))
                    .collect::<Result<Vec<_>, _>>()?,
            },
        })
    }

    fn lower_view(
        &mut self,
        node: &ViewNode,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        self.lower_view_style(node)?;
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
        let view_id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "component call has no shared view ID"))?;
        let call_id = self
            .declarations
            .component_call_id(view_id)
            .ok_or_else(|| self.invariant(span, "component call has no shared call ID"))?;
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
        let origin = self.declarations.view(view_id).origin;
        let mut arguments = Vec::with_capacity(params.len());
        for (param, supplied) in params.iter().zip(supplied_args) {
            let source = self
                .facts
                .component_argument_source(call_id, param.id)
                .ok_or_else(|| self.invariant(span, "component argument has no checked source"))?;
            if matches!(source, CheckedComponentArgumentSource::Supplied(_)) != supplied.is_some() {
                return Err(self.invariant(
                    span,
                    format!(
                        "raw prop `{}` supplied/default topology diverged from checking",
                        param.name
                    ),
                ));
            }
            let (expression, scope) = match source {
                CheckedComponentArgumentSource::Supplied(expression) => {
                    (expression, ArgumentScope::Caller)
                }
                CheckedComponentArgumentSource::Default(expression) => {
                    (expression, ArgumentScope::Definition)
                }
            };
            let writable = if param.capability == ParamCapability::Bind {
                if supplied.is_none() {
                    return Err(self.invariant(span, "bind argument resolved to a default"));
                }
                Some(self.resolve_writable(expression, outer_component, span)?)
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
        if call_id.0 as usize != self.calls.len() {
            return Err(self.invariant(span, "component call arena order diverged"));
        }
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
        expression: CheckedExprUseId,
        outer_component: Option<ComponentId>,
        span: &Span,
    ) -> Result<WritableStateRef, Error> {
        let expression = self.facts.expression_use(expression);
        let checked = self.facts.expression(expression.root);
        let crate::check::CheckedExprKind::Path {
            root: crate::check::CheckedPathRoot::Value(value),
            projections,
        } = &checked.kind
        else {
            return Err(self.invariant(span, "bind argument is not a direct path"));
        };
        if !projections.is_empty() {
            return Err(self.invariant(span, "bind argument is not a direct state path"));
        }
        let resolved = match value {
            CheckedValueRef::AppState(id) => WritableStateRef::App {
                id: *id,
                name: self.facts.value_by_ref(*value).name.clone(),
            },
            CheckedValueRef::ComponentParam(id)
                if outer_component == Some(id.component)
                    && self.components[id.component.0 as usize].params[id.index as usize]
                        .capability
                        == ParamCapability::Bind =>
            {
                WritableStateRef::ComponentParam {
                    id: *id,
                    name: self.facts.value_by_ref(*value).name.clone(),
                }
            }
            CheckedValueRef::ComponentState(id) if outer_component == Some(id.component) => {
                WritableStateRef::ComponentState {
                    id: *id,
                    name: self.facts.value_by_ref(*value).name.clone(),
                }
            }
            _ => return Err(self.invariant(span, "bind argument root is not writable here")),
        };
        Ok(resolved)
    }

    fn push_origin(&mut self, span: &Span, parent: Option<OriginId>) -> OriginId {
        self.origins.push(span, parent)
    }

    fn invariant(&self, span: &Span, message: impl Into<String>) -> Error {
        Error::new(
            "E196",
            span,
            format!("lowering invariant failed: {}", message.into()),
        )
    }

    fn invariant_at_origin(&self, origin: OriginId, message: impl Into<String>) -> Error {
        let message = format!("lowering invariant failed: {}", message.into());
        let Some(origin) = self.origins.try_get(origin) else {
            return Error::new("E196", &Span::line(1), message);
        };
        let mut error = Error::new(
            "E196",
            &Span {
                line: origin.line,
                column: origin.column,
            },
            message,
        );
        if let Some(path) = &origin.path {
            error = error.at_path(path.display().to_string());
        }
        error
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

fn valid_f32(value: f64) -> bool {
    value.is_finite() && value.abs() <= f32::MAX as f64
}

fn valid_positive_f32(value: f64) -> bool {
    value > 0.0 && valid_f32(value)
}

fn first_changed_static_setting_span<'a>(
    current: &'a AppSettings,
    expected: &'a AppSettings,
) -> Option<&'a Span> {
    for (name, changed) in [
        ("id", current.id != expected.id),
        ("executor", current.executor != expected.executor),
        ("renderer", current.renderer != expected.renderer),
        (
            "text-size",
            current.default_text_size != expected.default_text_size,
        ),
        (
            "antialiasing",
            current.antialiasing != expected.antialiasing,
        ),
        ("vsync", current.vsync != expected.vsync),
    ] {
        if changed {
            return current
                .setting_spans
                .get(name)
                .or_else(|| expected.setting_spans.get(name));
        }
    }
    if current.fonts != expected.fonts {
        return current
            .fonts
            .iter()
            .zip(&expected.fonts)
            .find_map(|(current, expected)| (current != expected).then_some(&current.span))
            .or_else(|| {
                current
                    .fonts
                    .get(expected.fonts.len())
                    .map(|font| &font.span)
            })
            .or_else(|| {
                expected
                    .fonts
                    .get(current.fonts.len())
                    .map(|font| &font.span)
            });
    }
    if current.window != expected.window {
        return changed_window_span(current.window.as_ref(), expected.window.as_ref());
    }
    if current.windows != expected.windows {
        return current
            .windows
            .iter()
            .zip(&expected.windows)
            .find_map(|(current, expected)| {
                if current.name != expected.name {
                    Some(&current.span)
                } else if current.settings != expected.settings {
                    changed_window_span(Some(&current.settings), Some(&expected.settings))
                        .or(Some(&current.span))
                } else {
                    None
                }
            })
            .or_else(|| {
                current
                    .windows
                    .get(expected.windows.len())
                    .map(|window| &window.span)
            })
            .or_else(|| {
                expected
                    .windows
                    .get(current.windows.len())
                    .map(|window| &window.span)
            });
    }
    None
}

fn changed_window_span<'a>(
    current: Option<&'a WindowSettings>,
    expected: Option<&'a WindowSettings>,
) -> Option<&'a Span> {
    let (current, expected) = match (current, expected) {
        (Some(current), Some(expected)) => (current, expected),
        (Some(current), None) => return Some(&current.span),
        (None, Some(expected)) => return Some(&expected.span),
        (None, None) => return None,
    };
    for (name, changed) in [
        ("size", current.size != expected.size),
        ("maximized", current.maximized != expected.maximized),
        ("fullscreen", current.fullscreen != expected.fullscreen),
        ("position", current.position != expected.position),
        ("min-size", current.min_size != expected.min_size),
        ("max-size", current.max_size != expected.max_size),
        ("visible", current.visible != expected.visible),
        ("resizable", current.resizable != expected.resizable),
        ("closeable", current.closeable != expected.closeable),
        ("minimizable", current.minimizable != expected.minimizable),
        ("decorations", current.decorations != expected.decorations),
        ("transparent", current.transparent != expected.transparent),
        ("blur", current.blur != expected.blur),
        ("level", current.level != expected.level),
        (
            "exit-on-close",
            current.exit_on_close_request != expected.exit_on_close_request,
        ),
    ] {
        if changed {
            return current
                .setting_spans
                .get(name)
                .or_else(|| expected.setting_spans.get(name));
        }
    }
    if current.icon != expected.icon {
        return current
            .icon
            .as_ref()
            .map(|icon| &icon.span)
            .or_else(|| expected.icon.as_ref().map(|icon| &icon.span));
    }
    match (current.linux.as_ref(), expected.linux.as_ref()) {
        (Some(current), Some(expected)) if current != expected => {
            for (name, changed) in [
                ("app-id", current.application_id != expected.application_id),
                (
                    "override-redirect",
                    current.override_redirect != expected.override_redirect,
                ),
            ] {
                if changed {
                    return current
                        .setting_spans
                        .get(name)
                        .or_else(|| expected.setting_spans.get(name));
                }
            }
            return Some(&current.span);
        }
        (Some(current), None) => return Some(&current.span),
        (None, Some(expected)) => return Some(&expected.span),
        _ => {}
    }
    match (current.windows.as_ref(), expected.windows.as_ref()) {
        (Some(current), Some(expected)) if current != expected => {
            for (name, changed) in [
                (
                    "drag-and-drop",
                    current.drag_and_drop != expected.drag_and_drop,
                ),
                (
                    "skip-taskbar",
                    current.skip_taskbar != expected.skip_taskbar,
                ),
                (
                    "undecorated-shadow",
                    current.undecorated_shadow != expected.undecorated_shadow,
                ),
                ("corner", current.corner != expected.corner),
            ] {
                if changed {
                    return current
                        .setting_spans
                        .get(name)
                        .or_else(|| expected.setting_spans.get(name));
                }
            }
            return Some(&current.span);
        }
        (Some(current), None) => return Some(&current.span),
        (None, Some(expected)) => return Some(&expected.span),
        _ => {}
    }
    match (current.macos.as_ref(), expected.macos.as_ref()) {
        (Some(current), Some(expected)) if current != expected => {
            for (name, changed) in [
                (
                    "title-hidden",
                    current.title_hidden != expected.title_hidden,
                ),
                (
                    "titlebar-transparent",
                    current.titlebar_transparent != expected.titlebar_transparent,
                ),
                (
                    "fullsize-content-view",
                    current.fullsize_content_view != expected.fullsize_content_view,
                ),
            ] {
                if changed {
                    return current
                        .setting_spans
                        .get(name)
                        .or_else(|| expected.setting_spans.get(name));
                }
            }
            return Some(&current.span);
        }
        (Some(current), None) => return Some(&current.span),
        (None, Some(expected)) => return Some(&expected.span),
        _ => {}
    }
    match (current.wasm.as_ref(), expected.wasm.as_ref()) {
        (Some(current), Some(expected)) if current != expected => {
            if current.target != expected.target {
                return current
                    .setting_spans
                    .get("target")
                    .or_else(|| expected.setting_spans.get("target"));
            }
            return Some(&current.span);
        }
        (Some(current), None) => return Some(&current.span),
        (None, Some(expected)) => return Some(&expected.span),
        _ => {}
    }
    Some(&current.span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, analyze_file};
    use std::fmt::Write as _;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const THEME: &str = "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";

    fn route_snapshot(program: &LoweredProgram, route: &ResolvedRoute) -> String {
        let target = match &route.target {
            ResolvedRouteTarget::App { handler, name } => {
                format!("app h{} {name}", handler.0)
            }
            ResolvedRouteTarget::Component {
                component,
                handler,
                name,
            } => format!("component c{} h{} {name}", component.0, handler.0),
        };
        let args = route
            .args
            .iter()
            .map(|arg| match arg {
                ResolvedRouteArg::Expression(expression) => format!("expr {expression:?}"),
                ResolvedRouteArg::Payload { index, ty } => {
                    format!("payload {index}:{}", ty.display())
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let origin = program.origin(route.origin);
        format!(
            "r{} -> {target} ({args}) @{}:{}",
            route.id.0, origin.line, origin.column
        )
    }

    fn task_source_snapshot(source: &ResolvedTaskSource) -> String {
        match source {
            ResolvedTaskSource::Effect {
                task,
                kind,
                target,
                args,
            } => {
                format!("t{} {kind:?} {target:?} args={args:?}", task.0)
            }
            ResolvedTaskSource::Done { task, value } => {
                format!("t{} done {value:?}", task.0)
            }
            ResolvedTaskSource::None { task, output } => {
                format!("t{} none {}", task.0, output.display())
            }
        }
    }

    fn task_transform_snapshot(transform: &ResolvedTaskTransform) -> String {
        match transform {
            ResolvedTaskTransform::Map {
                task,
                local,
                binding,
                input,
                value,
                ..
            } => format!(
                "t{} map {binding}:{}/local={local:?} -> {value:?}",
                task.0,
                input.display()
            ),
            ResolvedTaskTransform::Then {
                task,
                local,
                binding,
                input,
                source,
            } => format!(
                "t{} then {binding}:{}/local={local:?} -> {}",
                task.0,
                input.display(),
                task_source_snapshot(source)
            ),
            ResolvedTaskTransform::AndThen {
                task,
                local,
                binding,
                input,
                source,
            } => format!(
                "t{} try {binding}:{}/local={local:?} -> {}",
                task.0,
                input.display(),
                task_source_snapshot(source)
            ),
            ResolvedTaskTransform::MapError {
                task,
                local,
                binding,
                input,
                value,
            } => format!(
                "t{} map-error {binding}:{}/local={local:?} -> {value:?}",
                task.0,
                input.display()
            ),
            ResolvedTaskTransform::Collect { task } => format!("t{} collect", task.0),
            ResolvedTaskTransform::Discard { task } => format!("t{} discard", task.0),
        }
    }

    fn statement_snapshot(
        program: &LoweredProgram,
        statement: &ResolvedStatement,
        indent: usize,
        output: &mut String,
    ) {
        let padding = " ".repeat(indent);
        let origin = program.origin(statement.origin);
        let kind = match &statement.kind {
            ResolvedStatementKind::Let {
                local, name, value, ..
            } => {
                format!("let {name} {local:?} = {value:?}")
            }
            ResolvedStatementKind::Assign {
                target,
                value,
                at,
                move_self,
            } => format!(
                "assign {}:{}, value={value:?}, at={at:?}, move={move_self}",
                target.name,
                target.ty.display()
            ),
            ResolvedStatementKind::MarkdownAppend { .. } => "markdown-append".into(),
            ResolvedStatementKind::ComboPush { .. } => "combo-push".into(),
            ResolvedStatementKind::ReturnIf { condition } => {
                format!("return-if {condition:?}")
            }
            ResolvedStatementKind::Exit => "exit".into(),
            ResolvedStatementKind::Run(run) => format!(
                "run {:?} site={:?} {} error={}",
                run.mode,
                run.site,
                route_snapshot(program, &run.success),
                run.error
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route))
            ),
            ResolvedStatementKind::Sip(sip) => format!(
                "sip {} | {} | {}",
                route_snapshot(program, &sip.progress),
                route_snapshot(program, &sip.success),
                sip.error
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route))
            ),
            ResolvedStatementKind::TaskFlow(flow) => format!(
                "flow source=[{}] output={} error={} transforms=[{}] success={} error-route={} units={}",
                task_source_snapshot(&flow.source),
                flow.output
                    .as_ref()
                    .map_or_else(|| "none".into(), Type::display),
                flow.error_type
                    .as_ref()
                    .map_or_else(|| "none".into(), Type::display),
                flow.transforms
                    .iter()
                    .map(task_transform_snapshot)
                    .collect::<Vec<_>>()
                    .join("; "),
                flow.success
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route)),
                flow.error
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route)),
                flow.units
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route)),
            ),
            ResolvedStatementKind::TaskGroup { kind, statements } => {
                let label = format!("group {kind:?}");
                writeln!(
                    output,
                    "{padding}s{} task={:?} final={} {label} @{}:{}",
                    statement.id.0, statement.task, statement.is_final, origin.line, origin.column
                )
                .unwrap();
                for child in statements {
                    statement_snapshot(program, child, indent + 2, output);
                }
                return;
            }
            ResolvedStatementKind::Abortable { task, .. } => {
                let label = "abortable";
                writeln!(
                    output,
                    "{padding}s{} task={:?} final={} {label} @{}:{}",
                    statement.id.0, statement.task, statement.is_final, origin.line, origin.column
                )
                .unwrap();
                statement_snapshot(program, task, indent + 2, output);
                return;
            }
            ResolvedStatementKind::Abort { .. } => "abort".into(),
            ResolvedStatementKind::DebugStart { .. } => "debug-start".into(),
            ResolvedStatementKind::DebugFinish { .. } => "debug-finish".into(),
            ResolvedStatementKind::ClipboardWrite { .. } => "clipboard-write".into(),
            ResolvedStatementKind::WidgetOperation { route, .. } => format!(
                "widget-op {}",
                route
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route))
            ),
            ResolvedStatementKind::PaneOperation { route, .. } => format!(
                "pane-op {}",
                route
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route))
            ),
            ResolvedStatementKind::WindowOperation { route, .. } => format!(
                "window-op {}",
                route
                    .as_ref()
                    .map_or_else(|| "none".into(), |route| route_snapshot(program, route))
            ),
        };
        writeln!(
            output,
            "{padding}s{} task={:?} final={} {kind} @{}:{}",
            statement.id.0, statement.task, statement.is_final, origin.line, origin.column
        )
        .unwrap();
    }

    fn handler_snapshot(program: &LoweredProgram) -> String {
        let mut output = String::new();
        for handler in program.handlers() {
            let origin = program.origin(handler.origin);
            writeln!(
                output,
                "h{} {:?} {} params={:?} @{}:{}",
                handler.id.0,
                handler.owner,
                handler.name,
                handler
                    .params
                    .iter()
                    .map(|param| format!("{}:{}:{:?}", param.name, param.ty.display(), param.local))
                    .collect::<Vec<_>>(),
                origin.line,
                origin.column
            )
            .unwrap();
            for statement in &handler.statements {
                statement_snapshot(program, statement, 2, &mut output);
            }
        }
        output
    }

    #[test]
    fn snapshots_app_mount_component_and_preset_handler_hir_with_stable_ids() {
        let source = format!(
            r#"app HandlerHir
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 0
preset seeded
  state
    value = 7
on mount
  let request = value + 1
  run fetch(request) -> loaded _
on loaded(next)
  value = next
component Card()
  state
    local = 0
  on start
    run replace fetch(local) -> done _
  on done(next)
    local = next
  button "Start" -> start
view
  Card
"#
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        assert_eq!(
            handler_snapshot(&program),
            r#"h0 App mount params=[] @19:1
  s0 task=None final=false let request CheckedLocalId(0) = CheckedExprUseId(2) @20:1
  s1 task=Some(TaskId(0)) final=true run Every site=None r0 -> app h1 loaded (payload 0:i64) @21:1 error=none @21:1
h1 App loaded params=["next:i64:CheckedLocalId(1)"] @22:1
  s2 task=None final=true assign value:i64, value=CheckedExprUseId(4), at=None, move=false @23:1
h2 Component(ComponentId(0)) start params=[] @27:1
  s3 task=Some(TaskId(1)) final=true run Replace site=Some(RunSiteId(0)) r1 -> component c0 h3 done (payload 0:i64) @28:1 error=none @28:1
h3 Component(ComponentId(0)) done params=["next:i64:CheckedLocalId(2)"] @29:1
  s4 task=None final=true assign local:i64, value=CheckedExprUseId(6), at=None, move=false @30:1
h4 Preset(0) preset seeded params=[] @16:1
  s5 task=None final=true assign value:i64, value=CheckedExprUseId(7), at=None, move=false @18:1
"#
        );
    }

    #[test]
    fn snapshots_nested_tasks_flows_and_body_routes_in_preorder() {
        let source = format!(
            r#"app TaskHir
extern crate::backend
  AppError(message:str)
  sip transfer(size:i64) progress=f64 -> bytes
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
{THEME}state
  request:task-handle? = none
on start
  parallel
    sip transfer(3)
      progress -> progressed _
      done -> downloaded _
    flow
      from stream numbers(3)
      map value -> value + 1
      then value -> task double(value)
      collect
      done -> collected _
      units -> planned _
    abortable request abort-on-drop
      task system theme -> themed _
on progressed(value)
on downloaded(value)
on collected(values)
on planned(units)
on themed(theme)
view
  text "Tasks"
"#
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let snapshot = handler_snapshot(&program);
        for expected in [
            "h0 App start params=[]",
            "s0 task=Some(TaskId(0)) final=true group Parallel",
            "s1 task=Some(TaskId(1)) final=true sip r0 -> app h1 progressed (payload 0:f64)",
            "r1 -> app h2 downloaded (payload 0:bytes)",
            "s2 task=Some(TaskId(2)) final=true flow source=[t3 Stream Extern(ExternFnId(1))",
            "t4 map value:i64/local=CheckedLocalId(0)",
            "t5 then value:i64/local=CheckedLocalId(1) -> t5 Task Extern(ExternFnId(2))",
            "t6 collect",
            "r2 -> app h3 collected (payload 0:[i64])",
            "r3 -> app h4 planned (payload 0:i64)",
            "s3 task=Some(TaskId(7)) final=true abortable",
            "s4 task=Some(TaskId(8)) final=true run Every site=None r4 -> app h5 themed (payload 0:str)",
        ] {
            assert!(
                snapshot.contains(expected),
                "missing `{expected}` in handler HIR snapshot:\n{snapshot}"
            );
        }
    }

    #[test]
    fn handler_codegen_uses_checked_expressions_and_stable_run_sites_after_ast_mutation() {
        let source = format!(
            r#"app Mutation
extern crate::backend
  fetch(value:i64) -> i64
{THEME}state
  value = 1
on changed
  let original = value + 1
  value = original
component Search()
  state
    query = 2
  on search
    run latest fetch(query) -> loaded _
  on loaded(next)
    query = next
  button "Search" -> search
view
  Search
"#
        );
        let mut checked = analyze(&source).unwrap();
        let Statement::Let { value, span, .. } = &mut checked.document.handlers[0].statements[0]
        else {
            panic!("fixture must start with a let statement");
        };
        *value = Expr::Str("unchecked-poison".into());
        span.line = 900;
        let Statement::Run { args, span, .. } =
            &mut checked.document.components[0].handlers[0].statements[0]
        else {
            panic!("component fixture must contain a latest run");
        };
        args[0] = Expr::Str("unchecked-run-poison".into());
        span.line = 999;

        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "mutation.ice").unwrap();
        assert!(generated.contains("let original = (self.value + 1);"));
        assert!(generated.contains("crate::backend::fetch(__local.query)"));
        assert!(generated.contains("__ice_latest_0"));
        assert!(!generated.contains("Latest999"));
        assert!(!generated.contains("unchecked-poison"));
        assert!(!generated.contains("unchecked-run-poison"));
        assert!(!generated.contains("// __ICE_SOURCE 900 1"));
        assert!(!generated.contains("// __ICE_SOURCE 999 1"));
    }

    #[test]
    fn rejects_mutated_handler_route_run_site_and_statement_shapes_as_hir_invariants() {
        let source = format!(
            r#"app InvalidHir
extern crate::backend
  fetch(value:i64) -> i64
{THEME}component Search()
  state
    query = 1
  on search
    run latest fetch(query) -> loaded _
  on loaded(next)
    query = next
  button "Search" -> search
view
  Search
"#
        );

        let mut changed_mode = analyze(&source).unwrap();
        let Statement::Run { mode, .. } =
            &mut changed_mode.document.components[0].handlers[0].statements[0]
        else {
            panic!("fixture must contain a run");
        };
        *mode = FutureMode::Replace;
        let error = lower(changed_mode).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("semantic contract"));

        let mut changed_route = analyze(&source).unwrap();
        let Statement::Run { success, .. } =
            &mut changed_route.document.components[0].handlers[0].statements[0]
        else {
            panic!("fixture must contain a run");
        };
        success.args[0] = RouteArg::Expr(Expr::I64(7));
        let error = lower(changed_route).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("semantic contract"));

        let mut missing_statement = analyze(&source).unwrap();
        missing_statement.document.components[0].handlers[0]
            .statements
            .clear();
        let error = lower(missing_statement).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("statement HIR declaration count"));
    }

    #[test]
    fn imported_handler_origins_reach_lowered_hir_and_generated_source_markers() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-handler-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("card.ice");
        fs::write(
            &root,
            format!("app ImportedHandler\nuse \"card.ice\"\n{THEME}view\n  ImportedCard\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedCard()\n  state\n    selected = false\n  on select\n    selected = true\n  button \"Select\" -> select\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let handler = program
            .handlers()
            .iter()
            .find(|handler| handler.name == "select")
            .unwrap();
        let handler_origin = program.origin(handler.origin);
        assert_eq!(handler_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(handler_origin.line, 4);
        let statement_origin = program.origin(handler.statements[0].origin);
        assert_eq!(statement_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(statement_origin.line, 5);
        assert_eq!(statement_origin.parent, Some(handler.origin));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 5 1 {encoded_import}")));

        let imported_handler = program
            .handlers
            .iter_mut()
            .find(|handler| handler.name == "select")
            .unwrap();
        imported_handler.id = HandlerId(u32::MAX);
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn handler_semantic_contract_rejects_same_signature_raw_mutations() {
        let source = format!(
            r#"app SemanticContract
extern crate::backend
  fetch(value:i64) -> i64
  fetch_other(value:i64) -> i64
{THEME}state
  first = 1
  second = 2
on start
  first = first + 1
  run fetch(first) -> loaded _
on loaded(value)
  first = value
on route_alternate
  run fetch(first) -> alternate _
on alternate(value)
  first = value
on empty
  flow
    from none i64
    done -> loaded _
view
  text first
"#
        );

        let mut changed_target = analyze(&source).unwrap();
        let Statement::Assign { target, .. } =
            &mut changed_target.document.handlers[0].statements[0]
        else {
            panic!("fixture must contain an assignment");
        };
        *target = "second".into();
        let error = lower(changed_target).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("semantic contract"));

        let mut changed_effect = analyze(&source).unwrap();
        let Statement::Run { function, .. } =
            &mut changed_effect.document.handlers[0].statements[1]
        else {
            panic!("fixture must contain a run");
        };
        *function = "fetch_other".into();
        let error = lower(changed_effect).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("semantic contract"));

        let mut changed_route = analyze(&source).unwrap();
        let Statement::Run { success, .. } = &mut changed_route.document.handlers[0].statements[1]
        else {
            panic!("fixture must contain a route");
        };
        success.handler = "alternate".into();
        let error = lower(changed_route).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("semantic contract"));

        let mut missing_argument = analyze(&source).unwrap();
        let Statement::Run { success, .. } =
            &mut missing_argument.document.handlers[0].statements[1]
        else {
            panic!("fixture must contain a route");
        };
        success.args.clear();
        let error = lower(missing_argument).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("semantic contract"));

        let mut changed_param = analyze(&source).unwrap();
        changed_param.document.handlers[1].params[0].name = "renamed".into();
        let error = lower(changed_param).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("parameter contract"));

        let mut changed_none = analyze(&source).unwrap();
        let Statement::TaskFlow { source, .. } =
            &mut changed_none.document.handlers[4].statements[0]
        else {
            panic!("fixture must contain a flow");
        };
        let TaskSource::None { output, .. } = source else {
            panic!("fixture must contain a none source");
        };
        *output = Type::F64;
        let error = lower(changed_none).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("semantic contract"));
    }

    #[test]
    fn typed_route_core_uses_real_source_payload_contracts_without_handler_ids() {
        let route = Route {
            handler: "moved".into(),
            args: vec![RouteArg::Payload, RouteArg::Payload],
            span: Span::line(27),
        };
        let args = lower_typed_route_arguments(
            &route,
            &[Type::F64, Type::I64],
            TypedRouteInputs {
                source_payloads: &[Type::F64, Type::I64],
                ordered: true,
            },
            |_| unreachable!("fixture has no expression arguments"),
        )
        .unwrap();
        assert!(matches!(
            args.as_slice(),
            [
                ResolvedRouteArg::Payload { index: 0, .. },
                ResolvedRouteArg::Payload { index: 1, .. }
            ]
        ));

        let error = lower_typed_route_arguments(
            &route,
            &[Type::F64, Type::I64],
            TypedRouteInputs {
                source_payloads: &[Type::F64, Type::F64],
                ordered: true,
            },
            |_| unreachable!("fixture has no expression arguments"),
        )
        .unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, 27);
        assert!(error.message.contains("payload type"));
    }

    #[test]
    fn checked_operation_contract_preserves_every_non_expression_payload() {
        let span = Span::line(1);
        let widget = |name: &str, all: bool| Statement::WidgetOperation {
            operation: WidgetOperation::Find {
                selector: WidgetSelector::Id(WidgetTarget {
                    segments: vec![Id {
                        name: name.into(),
                        key: Some(Expr::I64(1)),
                    }],
                }),
                all,
            },
            route: None,
            span: span.clone(),
        };
        assert_ne!(
            crate::hir::handler_operation_contract(&widget("first", false)),
            crate::hir::handler_operation_contract(&widget("second", false))
        );
        assert_ne!(
            crate::hir::handler_operation_contract(&widget("first", false)),
            crate::hir::handler_operation_contract(&widget("first", true))
        );

        let pane = |edge| Statement::PaneOperation {
            grid: "work".into(),
            operation: PaneOperation::Drop {
                pane: PaneReference::Dynamic {
                    template: "file".into(),
                    key: Expr::I64(1),
                },
                target: PaneReference::Static("editor".into()),
                edge,
            },
            route: None,
            span: span.clone(),
        };
        assert_ne!(
            crate::hir::handler_operation_contract(&pane(Some(PaneEdge::Left))),
            crate::hir::handler_operation_contract(&pane(Some(PaneEdge::Right)))
        );

        let window = |name: &str, function: &str, arguments: usize| {
            let operation = if function.is_empty() {
                WindowOperation::Open(Some(name.into()))
            } else {
                WindowOperation::Callback {
                    function: function.into(),
                    args: vec![Expr::I64(1); arguments],
                }
            };
            Statement::WindowOperation {
                operation,
                target: None,
                route: None,
                span: span.clone(),
            }
        };
        assert_ne!(
            crate::hir::handler_operation_contract(&window("first", "", 0)),
            crate::hir::handler_operation_contract(&window("second", "", 0))
        );
        assert_ne!(
            crate::hir::handler_operation_contract(&window("", "first", 1)),
            crate::hir::handler_operation_contract(&window("", "second", 1))
        );
        assert_ne!(
            crate::hir::handler_operation_contract(&window("", "first", 1)),
            crate::hir::handler_operation_contract(&window("", "first", 2))
        );
    }

    #[test]
    fn malformed_handler_hir_ids_are_fallible_source_mapped_invariants() {
        fn program() -> LoweredProgram {
            let source = format!(
                "app InvalidIds\nextern crate::backend\n  fetch(value:i64) -> i64\n{THEME}state\n  value = 1\non start\n  run fetch(value + 1) -> loaded(7)\non loaded(next)\n  value = next\nview\n  text value\n"
            );
            lower(analyze(&source).unwrap()).unwrap()
        }

        fn route(program: &mut LoweredProgram) -> &mut ResolvedRoute {
            let ResolvedStatementKind::Run(run) = &mut program.handlers[0].statements[0].kind
            else {
                panic!("fixture must contain a run");
            };
            &mut run.success
        }

        let mut invalid_route = program();
        let origin = route(&mut invalid_route).origin;
        let expected_line = invalid_route.origin(origin).line;
        route(&mut invalid_route).id = RouteId(u32::MAX);
        let error = crate::codegen::generate(&invalid_route, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("route ID"));

        let mut invalid_target = program();
        let origin = route(&mut invalid_target).origin;
        let expected_line = invalid_target.origin(origin).line;
        let ResolvedRouteTarget::App { handler, .. } = &mut route(&mut invalid_target).target
        else {
            panic!("fixture route must target the app");
        };
        *handler = HandlerId(u32::MAX);
        let error = crate::codegen::generate(&invalid_target, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("handler ID"));

        let mut wrong_owner = program();
        let origin = route(&mut wrong_owner).origin;
        let expected_line = wrong_owner.origin(origin).line;
        let ResolvedRouteTarget::App { handler, .. } = &mut route(&mut wrong_owner).target else {
            panic!("fixture route must target the app");
        };
        *handler = HandlerId(0);
        let error = crate::codegen::generate(&wrong_owner, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("different handler"));

        let mut invalid_statement = program();
        let origin = invalid_statement.handlers[0].statements[0].origin;
        let expected_line = invalid_statement.origin(origin).line;
        invalid_statement.handlers[0].statements[0].id = StatementId(u32::MAX);
        let error = crate::codegen::generate(&invalid_statement, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("statement ID"));

        let mut invalid_task = program();
        let origin = invalid_task.handlers[0].statements[0].origin;
        let expected_line = invalid_task.origin(origin).line;
        invalid_task.handlers[0].statements[0].task = Some(TaskId(u32::MAX));
        let error = crate::codegen::generate(&invalid_task, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("task ID"));

        let mut missing_task = program();
        missing_task.handlers[0].statements[0].task = None;
        let error = crate::codegen::generate(&missing_task, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("task ID"));

        let mut invalid_mode = program();
        let ResolvedStatementKind::Run(run) = &mut invalid_mode.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain a run");
        };
        run.mode = FutureMode::Latest;
        let error = crate::codegen::generate(&invalid_mode, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("run mode"));

        let mut invalid_kind = program();
        let ResolvedStatementKind::Run(run) = &mut invalid_kind.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain a run");
        };
        run.kind = EffectKind::Task;
        let error = crate::codegen::generate(&invalid_kind, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("effect kind"));

        let mut invalid_operand = program();
        let ResolvedStatementKind::Run(run) = &mut invalid_operand.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain a run");
        };
        run.args[0] = CheckedExprUseId::invalid_for_test();
        let error = crate::codegen::generate(&invalid_operand, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression-use ID"));

        let mut invalid_descendant = program();
        let task = invalid_descendant.handlers[0].statements[0]
            .task
            .expect("fixture run task");
        invalid_descendant.facts.corrupt_expression_first_child(
            crate::check::CheckedExprOwner::Task { task, operand: 0 },
            u32::MAX,
        );
        let error = crate::codegen::generate(&invalid_descendant, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("descendant ID"));

        let mut invalid_route_operand = program();
        let ResolvedRouteArg::Expression(expression) =
            &mut route(&mut invalid_route_operand).args[0]
        else {
            panic!("fixture route must contain an expression");
        };
        *expression = CheckedExprUseId::invalid_for_test();
        let error = crate::codegen::generate(&invalid_route_operand, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression-use ID"));

        let mut invalid_param = program();
        invalid_param.handlers[1].params[0].local =
            crate::check::CheckedLocalId::invalid_for_test();
        let error = crate::codegen::generate(&invalid_param, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("parameter local ID"));

        let mut invalid_handler_order = program();
        invalid_handler_order.handlers.swap(0, 1);
        let error = crate::codegen::generate(&invalid_handler_order, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("handler arena order"));

        let mut invalid_app_partition = program();
        invalid_app_partition.app_handlers.pop();
        let error = crate::codegen::generate(&invalid_app_partition, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("app handler index"));

        let component_source = format!(
            "app ComponentPartition\n{THEME}component Surface()\n  on update\n  text \"Ready\"\nview\n  Surface\n"
        );
        let mut invalid_component = lower(analyze(&component_source).unwrap()).unwrap();
        invalid_component.components[0].id = ComponentId(u32::MAX);
        let error = crate::codegen::generate(&invalid_component, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("component identity"));
    }

    #[test]
    fn malformed_run_site_and_required_operation_routes_are_e196_not_panics() {
        let latest = format!(
            "app Search\nextern crate::backend\n  fetch(query:str) -> str\n{THEME}component SearchBox()\n  state\n    query = \"\"\n    result:str? = none\n  on search\n    run latest fetch(query) -> loaded _\n  on loaded(value)\n    result = some(value)\n  button \"Search\" -> search\nview\n  SearchBox #search\n"
        );
        let mut invalid_site = lower(analyze(&latest).unwrap()).unwrap();
        let statement = invalid_site
            .handlers
            .iter_mut()
            .find(|handler| handler.name == "search")
            .and_then(|handler| handler.statements.first_mut())
            .expect("fixture component run");
        let ResolvedStatementKind::Run(run) = &mut statement.kind else {
            panic!("fixture must contain a run");
        };
        run.site = None;
        let error = crate::codegen::generate(&invalid_site, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("run-site"));

        let widget = format!(
            "app WidgetRoute\n{THEME}state\n  field = \"\"\n  focused = false\non inspect\n  task widget focused #field -> observed _\non observed(value)\n  focused = value\nview\n  input \"Field\" #field <-> field\n"
        );
        let mut invalid_widget = lower(analyze(&widget).unwrap()).unwrap();
        let statement = &mut invalid_widget.handlers[0].statements[0];
        let ResolvedStatementKind::WidgetOperation { route, .. } = &mut statement.kind else {
            panic!("fixture must contain a widget operation");
        };
        *route = None;
        let error = crate::codegen::generate(&invalid_widget, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route cardinality"));

        let pane = format!(
            "app PaneRoute\n{THEME}on inspect\n  pane #work maximized -> observed _\non observed(name)\nview\n  panes #work\n    split vertical\n      pane files\n        text \"Files\"\n      pane editor\n        text \"Editor\"\n"
        );
        let mut invalid_pane = lower(analyze(&pane).unwrap()).unwrap();
        let statement = &mut invalid_pane.handlers[0].statements[0];
        let ResolvedStatementKind::PaneOperation { route, .. } = &mut statement.kind else {
            panic!("fixture must contain a pane operation");
        };
        *route = None;
        let error = crate::codegen::generate(&invalid_pane, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route cardinality"));

        let window = format!(
            "app WindowRoute\n{THEME}on inspect\n  task window size -> observed _ _\non observed(width, height)\nview\n  text \"Window\"\n"
        );
        let mut invalid_window = lower(analyze(&window).unwrap()).unwrap();
        let statement = &mut invalid_window.handlers[0].statements[0];
        let ResolvedStatementKind::WindowOperation { route, .. } = &mut statement.kind else {
            panic!("fixture must contain a window operation");
        };
        *route = None;
        let error = crate::codegen::generate(&invalid_window, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route cardinality"));
    }

    #[test]
    fn malformed_operation_discriminants_are_e196_before_route_expectations() {
        let widget = format!(
            "app WidgetOperation\n{THEME}state\n  field = \"\"\non inspect\n  task widget focus #field\nview\n  input \"Field\" #field <-> field\n"
        );
        let mut invalid_widget = lower(analyze(&widget).unwrap()).unwrap();
        let statement = &mut invalid_widget.handlers[0].statements[0];
        let ResolvedStatementKind::WidgetOperation { operation, .. } = &mut statement.kind else {
            panic!("fixture must contain a widget operation");
        };
        let ResolvedWidgetOperation::Focus { target } = operation else {
            panic!("fixture must contain widget focus");
        };
        *operation = ResolvedWidgetOperation::Focused {
            target: target.clone(),
        };
        let error = crate::codegen::generate(&invalid_widget, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("operation contract"));

        let pane = format!(
            "app PaneOperation\n{THEME}on inspect\n  pane #work restore\nview\n  panes #work\n    split vertical\n      pane files\n        text \"Files\"\n      pane editor\n        text \"Editor\"\n"
        );
        let mut invalid_pane = lower(analyze(&pane).unwrap()).unwrap();
        let statement = &mut invalid_pane.handlers[0].statements[0];
        let ResolvedStatementKind::PaneOperation { operation, .. } = &mut statement.kind else {
            panic!("fixture must contain a pane operation");
        };
        *operation = ResolvedPaneOperation::Maximized;
        let error = crate::codegen::generate(&invalid_pane, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("operation contract"));

        let window = format!(
            "app WindowOperation\n{THEME}on inspect\n  task window close\nview\n  text \"Window\"\n"
        );
        let mut invalid_window = lower(analyze(&window).unwrap()).unwrap();
        let statement = &mut invalid_window.handlers[0].statements[0];
        let ResolvedStatementKind::WindowOperation { operation, .. } = &mut statement.kind else {
            panic!("fixture must contain a window operation");
        };
        *operation = ResolvedWindowOperation::Size;
        let error = crate::codegen::generate(&invalid_window, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("operation contract"));
    }

    #[test]
    fn malformed_codegen_consumed_handler_fields_are_e196() {
        fn editor_program() -> LoweredProgram {
            let source = format!(
                "app EditorContract\nextern crate::backend\n  sync apply_command(content:editor, command:str) -> editor\n{THEME}state\n  notes:editor = \"hello\"\non command\n  notes = apply_command(notes, \"bold\")\nview\n  editor <-> notes\n"
            );
            lower(analyze(&source).unwrap()).unwrap()
        }

        let mut invalid_move = editor_program();
        let ResolvedStatementKind::Assign { move_self, .. } =
            &mut invalid_move.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain an assignment");
        };
        *move_self = false;
        let error = crate::codegen::generate(&invalid_move, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("self-move"));

        let mut invalid_writable = editor_program();
        let ResolvedStatementKind::Assign { target, .. } =
            &mut invalid_writable.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain an assignment");
        };
        target.ty = Type::Str;
        let error = crate::codegen::generate(&invalid_writable, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("writable target"));

        let let_source =
            format!("app LetContract\n{THEME}on start\n  let total = 1\nview\n  text \"ready\"\n");
        let mut invalid_let = lower(analyze(&let_source).unwrap()).unwrap();
        let ResolvedStatementKind::Let { ty, .. } = &mut invalid_let.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain a let");
        };
        *ty = Type::F64;
        let error = crate::codegen::generate(&invalid_let, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("let local"));

        let pane_source = format!(
            "app PaneMode\n{THEME}on inspect\n  pane #work restore\nview\n  panes #work\n    split vertical\n      pane files\n        text \"Files\"\n      pane editor\n        text \"Editor\"\n"
        );
        let mut invalid_pane = lower(analyze(&pane_source).unwrap()).unwrap();
        let ResolvedStatementKind::PaneOperation { dynamic, .. } =
            &mut invalid_pane.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain a pane operation");
        };
        *dynamic = !*dynamic;
        let error = crate::codegen::generate(&invalid_pane, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("pane grid mode"));

        let flow_source = format!(
            "app FlowContract\n{THEME}on start\n  flow\n    from done 1\n    map value -> value + 1\n    done -> loaded _\non loaded(value)\nview\n  text \"ready\"\n"
        );
        let mut invalid_flow = lower(analyze(&flow_source).unwrap()).unwrap();
        let ResolvedStatementKind::TaskFlow(flow) =
            &mut invalid_flow.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain a flow");
        };
        flow.output = Some(Type::F64);
        let error = crate::codegen::generate(&invalid_flow, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("output or error type"));

        let mut invalid_transform = lower(analyze(&flow_source).unwrap()).unwrap();
        let ResolvedStatementKind::TaskFlow(flow) =
            &mut invalid_transform.handlers[0].statements[0].kind
        else {
            panic!("fixture must contain a flow");
        };
        let ResolvedTaskTransform::Map { input, .. } = &mut flow.transforms[0] else {
            panic!("fixture must contain a map transform");
        };
        *input = Type::F64;
        let error = crate::codegen::generate(&invalid_transform, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("transform local"));
    }

    #[test]
    #[ignore = "explicit many-component handler partition linearity contract"]
    fn performance_contract_component_handler_partitions_are_linear() {
        use std::fmt::Write as _;
        use std::time::{Duration, Instant};

        fn measure(components: usize) -> Duration {
            let mut source = format!("app HandlerPartitions\n{THEME}");
            for index in 0..components {
                writeln!(
                    source,
                    "component Surface{index}()\n  on update\n  text \"Surface {index}\""
                )
                .unwrap();
            }
            source.push_str("view\n  text \"Ready\"\n");
            let program = lower(analyze(&source).unwrap()).unwrap();
            assert_eq!(program.handlers.len(), components);
            let started = Instant::now();
            for _ in 0..20 {
                program.validate_handler_hir().unwrap();
            }
            started.elapsed()
        }

        let small = measure(500);
        let large = measure(4_000);
        eprintln!("500 component handlers in {small:?}; 4k in {large:?}");
        assert!(
            large.as_secs_f64() <= small.as_secs_f64() * 12.0 + 0.05,
            "handler partition validation exceeded linear allowance: 500={small:?}, 4k={large:?}"
        );
    }

    #[test]
    fn malformed_checked_handler_local_expression_and_extern_ids_do_not_panic() {
        fn checked() -> CheckedDocument {
            let source = format!(
                "app InvalidFacts\nextern crate::backend\n  fetch(value:i64) -> i64\n{THEME}state\n  value = 1\non start\n  run fetch(value + 1) -> loaded _\non loaded(next)\n  value = next\nview\n  text value\n"
            );
            analyze(&source).unwrap()
        }

        let mut invalid_local = checked();
        invalid_local
            .facts
            .corrupt_handler_param_local(HandlerId(1), 0, u32::MAX);
        let error = lower(invalid_local).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("parameter local ID"));

        let mut invalid_root = checked();
        let statement = invalid_root.declarations.handlers()[1].statement_roots[0];
        invalid_root.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::HandlerStatement {
                statement,
                operand: 0,
            },
            u32::MAX,
        );
        let error = lower(invalid_root).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression root ID"));

        let mut invalid_descendant = checked();
        let statement = invalid_descendant.declarations.handlers()[0].statement_roots[0];
        let task = invalid_descendant
            .declarations
            .statement(statement)
            .task
            .expect("fixture run task");
        invalid_descendant.facts.corrupt_expression_first_child(
            crate::check::CheckedExprOwner::Task { task, operand: 0 },
            u32::MAX,
        );
        let error = lower(invalid_descendant).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("descendant ID"));

        let mut invalid_cycle = checked();
        let statement = invalid_cycle.declarations.handlers()[0].statement_roots[0];
        let task = invalid_cycle
            .declarations
            .statement(statement)
            .task
            .expect("fixture run task");
        invalid_cycle.facts.corrupt_expression_first_child_to_root(
            crate::check::CheckedExprOwner::Task { task, operand: 0 },
        );
        let error = lower(invalid_cycle).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("cycle"));

        let mut invalid_extern = checked();
        let task = invalid_extern
            .declarations
            .statement(invalid_extern.declarations.handlers()[0].statement_roots[0])
            .task
            .unwrap();
        invalid_extern
            .facts
            .corrupt_task_extern_target(task, u32::MAX);
        let error = lower(invalid_extern).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("extern target ID"));
    }

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
    fn bind_writability_comes_from_the_checked_expression_root() {
        let source = format!(
            "app BindFacts\n{THEME}state\n  draft = \"Draft\"\ncomponent Field(bind value:str)\n  text value\nview\n  Field value<->draft\n"
        );
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Component { args, .. } = &mut checked.document.view else {
            panic!("fixture root must be a component call");
        };
        args[0].value = Expr::Bool(false);

        let program = lower(checked).unwrap();
        let argument = &program.calls[0].arguments[0];
        assert!(matches!(
            &argument.writable,
            Some(WritableStateRef::App { name, .. }) if name == "draft"
        ));
        let root = program.checked_facts().expression(
            program
                .checked_facts()
                .expression_use(argument.expression)
                .root,
        );
        assert!(matches!(
            root.kind,
            crate::check::CheckedExprKind::Path {
                root: crate::check::CheckedPathRoot::Value(CheckedValueRef::AppState(_)),
                ref projections,
            } if projections.is_empty()
        ));
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
    fn snapshots_the_complete_initializer_hir_slice() {
        let source = format!(
            "app Initializers\nextern crate::backend\n  sync elastic(value:f64) -> f64\n{THEME}state\n  progress:animation[f64] = 0.0\n    easing elastic\n    duration 120ms\n    delay 5ms\n    repeat 2\n    auto-reverse true\nderived\n  total = 1.0 + 2.0\ncomponent Meter(label:str=\"ready\")\n  state\n    open = false\n  text label\nview\n  Meter\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        let app = &program.app_states[0];
        let derived = &program.derived[0];
        let component = &program.components[0];
        let default = component.params[0].default.unwrap();
        let component_state = &component.states[0];
        let app_use = facts.expression_use(app.initializer.expression);
        let derived_use = facts.expression_use(derived.initializer);
        let default_use = facts.expression_use(default);
        let component_state_use = facts.expression_use(component_state.initializer.expression);

        let snapshot = format!(
            "app {:?} {} {:?} use={:?} {:?} line={} animation={:?}\n\
             derived {:?} {} {:?} use={:?} {:?} line={}\n\
             default {:?} {} {:?} use={:?} {:?} line={}\n\
             component-state {:?} {} {:?} use={:?} {:?} line={} animation={:?}\n",
            app.id,
            app.name,
            app.ty,
            app.initializer.expression,
            app_use.coercion,
            program.origin(app.origin).line,
            app.initializer.animation,
            derived.id,
            derived.name,
            derived.ty,
            derived.initializer,
            derived_use.coercion,
            program.origin(derived.origin).line,
            component.params[0].id,
            component.params[0].name,
            component.params[0].ty,
            default,
            default_use.coercion,
            program.origin(component.params[0].origin).line,
            component_state.id,
            component_state.name,
            component_state.ty,
            component_state.initializer.expression,
            component_state_use.coercion,
            program.origin(component_state.origin).line,
            component_state.initializer.animation,
        );
        assert_eq!(
            snapshot,
            "app AppStateId(0) progress Animation(F64) use=CheckedExprUseId(0) ValueToAnimation { value: F64 } line=15 animation=Some(ResolvedAnimation { easing: Some(Custom(ExternFnId(0))), duration: Some(Milliseconds(120)), delay_ms: Some(5), repeat: Some(2), repeat_forever: false, auto_reverse: true })\n\
             derived DerivedId(0) total F64 use=CheckedExprUseId(1) None line=22\n\
             default ComponentParamId { component: ComponentId(0), index: 0 } label Str use=CheckedExprUseId(2) None line=23\n\
             component-state ComponentStateId { component: ComponentId(0), index: 0 } open Bool use=CheckedExprUseId(3) None line=25 animation=None\n"
        );

        let generated = crate::codegen::generate(&program, "initializers.ice").unwrap();
        assert!(generated.contains(
            "::iced::Animation::new((0.0) as f32).easing(::iced::animation::Easing::Custom(|__value: f32| crate::backend::elastic(__value as f64) as f32)).duration(::std::time::Duration::from_millis(120)).delay(::std::time::Duration::from_millis(5)).repeat(2).auto_reverse()"
        ));
        assert!(generated.contains("fn __ice_derived_total(&self) -> f64 { (1.0 + 2.0) }"));
        assert!(generated.contains("(\"ready\").to_string()"));
        assert!(generated.contains("open: false"));
    }

    #[test]
    fn snapshots_complete_application_settings_hir_and_ignores_post_check_expression_mutations() {
        let source = r#"daemon Configured
  title describe(window)
  theme native_theme(window, dark)
  palette active_palette
  bg background
  fg foreground
  id "dev.example.configured"
  executor iced::executor::Default
  renderer crate::backend::Renderer
  font "assets/Brand.ttf"
  text-size 15
  antialiasing false
  vsync false
  scale scale_for(window)
  window dashboard
    size 960 720
    position centered
    visible false
    level always-on-top
    exit-on-close false
    platform windows
      skip-taskbar true
      corner round-small
  window child
    min-size 320 240
    max-size 1920 1080
extern crate::backend
  sync describe(id:window-id) -> str
  sync scale_for(id:window-id) -> f64
  theme native_theme(id:window-id, dark:bool)
font brand family="Brand Sans" weight=semibold stretch=semi-expanded style=italic default=true
theme contract AppTheme
  bg
  fg
  primary
  danger
palette light for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
palette dark for AppTheme
  bg #ffffff
  fg #000000
  primary #666666
  danger #cc0000
state
  dark = false
  active_palette:palette[AppTheme] = AppTheme.light
  background = "000000"
  foreground = "ffffff"
view
  text describe(window)
"#;
        let mut checked = analyze(source).unwrap();
        checked.document.settings.title.as_mut().unwrap().value = Expr::Bool(false);
        let Expr::Call { args, .. } = &mut checked.document.settings.theme.as_mut().unwrap().value
        else {
            panic!("fixture theme must be a factory call");
        };
        args[0] = Expr::Bool(false);
        args[1] = Expr::Str("mutated".into());
        checked.document.settings.palette.as_mut().unwrap().value = Expr::Bool(false);
        checked.document.settings.background.as_mut().unwrap().value = Expr::Bool(false);
        checked.document.settings.text_color.as_mut().unwrap().value = Expr::Bool(false);
        checked
            .document
            .settings
            .scale_factor
            .as_mut()
            .unwrap()
            .value = Expr::Str("mutated".into());

        let mut program = lower(checked).unwrap();
        let settings = program.settings();
        assert_eq!(settings.settings_id, AppSettingsId);
        assert_eq!(settings.kind, ProgramKind::Daemon);
        assert!(matches!(
            &settings.renderer,
            ResolvedRendererSelection::Custom { path, .. }
                if path == "crate::backend::Renderer"
        ));
        assert!(matches!(
            &settings.executor,
            ResolvedExecutorSelection::Custom { path, .. }
                if path == "iced::executor::Default"
        ));
        assert_eq!(settings.fonts.len(), 1);
        let default_font = settings.default_font.as_ref().unwrap();
        assert!(matches!(
            &default_font.family,
            FontFamily::Named(name) if name == "Brand Sans"
        ));
        assert_eq!(default_font.weight, FontWeight::Semibold);
        assert_eq!(default_font.stretch, FontStretch::SemiExpanded);
        assert_eq!(default_font.style, FontStyle::Italic);
        assert_eq!(settings.named_windows.len(), 2);
        assert_eq!(settings.named_windows[0].id, NamedWindowId(0));
        assert_eq!(settings.named_windows[0].name, "dashboard");
        assert_eq!(
            settings.named_windows[0].settings.size,
            Some((960.0, 720.0))
        );
        assert_eq!(
            settings.named_windows[0].settings.position,
            Some(ResolvedWindowPosition::Centered)
        );
        assert_eq!(
            settings.named_windows[0].settings.level,
            Some(ResolvedWindowLevel::AlwaysOnTop)
        );
        assert_eq!(settings.named_windows[0].settings.visible, Some(false));
        assert_eq!(
            settings.named_windows[0]
                .settings
                .windows
                .as_ref()
                .and_then(|platform| platform.corner),
            Some(ResolvedWindowCorner::RoundSmall)
        );
        assert!(matches!(
            program.theme().active_palette,
            ResolvedPaletteSelection::Dynamic(_)
        ));
        let ResolvedAppThemeSelection::Factory(factory) = &program.theme().app_theme else {
            panic!("app theme must retain its checked factory classification");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "native_theme"
        );
        assert_eq!(factory.arguments.len(), 2);
        let metrics = program.checked_facts().metrics();
        assert_eq!(metrics.app_setting_analysis_passes, 7);
        assert_eq!(metrics.type_scope_env_full_clones, 0);
        assert_eq!(metrics.scope_env_full_clones, 0);
        assert_eq!(
            program
                .checked_facts()
                .app_setting_daemon_window_local_count(),
            1,
            "all daemon callbacks share one typed current-window local"
        );

        program.document.settings = AppSettings::default();
        program.document.daemon = false;
        program.document.states.clear();
        program.document.derived.clear();
        program.document.fonts.clear();
        let generated = crate::codegen::generate(&program, "configured.ice").unwrap();
        for expected in [
            "crate::backend::describe(window)",
            "crate::backend::native_theme(window, self.dark)",
            "crate::backend::scale_for(window)",
            "match self.active_palette",
            "self.background",
            "self.foreground",
            "fn __window_0()",
            "fn __window_1()",
            "type __IceRenderer = crate::backend::Renderer",
            "::iced::daemon(Self::__boot, Self::__update, Self::__view)",
            ".executor::<iced::executor::Default>()",
            ".font(include_bytes!(\"assets/Brand.ttf\").as_slice())",
            "id: ::std::option::Option::Some(\"dev.example.configured\".to_owned())",
            "default_text_size: ::iced::Pixels(15 as f32)",
            "antialiasing: false",
            "vsync: false",
            "size: ::iced::Size::new(960 as f32, 720 as f32)",
            "visible: false",
            "level: ::iced::window::Level::AlwaysOnTop",
            "family: ::iced::font::Family::Name(\"Brand Sans\")",
            "weight: ::iced::font::Weight::Semibold",
            "stretch: ::iced::font::Stretch::SemiExpanded",
            "style: ::iced::font::Style::Italic",
        ] {
            assert!(
                generated.contains(expected),
                "missing checked setting output: {expected}"
            );
        }
        assert!(!generated.contains("mutated"));
    }

    #[test]
    fn rejects_application_setting_fact_shape_mutations_with_e196() {
        let source = format!(
            "app Settings\n  title title\n  theme native_theme(dark)\nextern crate::backend\n  theme native_theme(dark:bool)\n{THEME}state\n  title = \"Title\"\n  dark = false\nview\n  text title\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.document.settings.title = None;
        let Expr::Call { name, .. } = &mut checked.document.settings.theme.as_mut().unwrap().value
        else {
            panic!("fixture theme must be a factory call");
        };
        *name = "missing".into();
        assert!(lower(checked).is_ok());

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .remove_app_setting_expression(AppSettingExprId::Title);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked expression"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.remove_app_settings();
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("authoritative checked snapshot"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_app_theme_factory_id();
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid extern ID"));

        let window_source = format!(
            "app Settings\n  window detail\n    min-size 320 240\n    max-size 1920 1080\n{THEME}view\n  text \"ready\"\n"
        );
        let mut checked = analyze(&window_source).unwrap();
        checked.document.settings.windows[0].settings.min_size = Some((2000.0, 1200.0));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("changed after semantic analysis"));
        assert_eq!(error.line, 3);
    }

    #[test]
    fn daemon_theme_factory_without_title_uses_one_shared_typed_window_scope() {
        let source = format!(
            "daemon OnlyTheme\n  theme native_theme(window)\nextern crate::backend\n  theme native_theme(id:window-id)\n{THEME}view\n  text \"ready\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        assert!(program.settings().callback_window.is_some());
        assert_eq!(
            program
                .checked_facts()
                .app_setting_daemon_window_local_count(),
            1
        );
        let generated = crate::codegen::generate(&program, "only_theme.ice").unwrap();
        assert!(generated.contains("crate::backend::native_theme(window)"));

        let mut corrupted = analyze(&source).unwrap();
        corrupted.facts.corrupt_app_setting_daemon_window_owner();
        let error = lower(corrupted).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 2));
        assert!(error.message.contains("cannot reference a view local"));
    }

    #[test]
    fn application_settings_accept_derived_values_across_every_dynamic_callback() {
        let source = format!(
            "app DerivedSettings\n  title computed_title\n  theme computed_theme\n  palette computed_palette\n  bg computed_background\n  fg computed_foreground\n  scale computed_scale\n{THEME}state\n  base_title = \"Title\"\n  base_theme = \"app\"\n  base_palette:palette[AppTheme] = AppTheme.app\n  base_background = \"000000\"\n  base_foreground = \"ffffff\"\n  base_scale = 1.25\nderived\n  computed_title = trim(base_title)\n  computed_theme = base_theme\n  computed_palette = base_palette\n  computed_background = base_background\n  computed_foreground = base_foreground\n  computed_scale = base_scale\nview\n  text computed_title\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let metrics = program.checked_facts().metrics();
        assert_eq!(metrics.app_setting_analysis_passes, 6);
        assert_eq!(metrics.type_scope_env_full_clones, 0);
        assert_eq!(metrics.scope_env_full_clones, 0);
        let generated = crate::codegen::generate(&program, "derived_settings.ice").unwrap();
        for name in [
            "computed_title",
            "computed_theme",
            "computed_palette",
            "computed_background",
            "computed_foreground",
            "computed_scale",
        ] {
            assert!(
                generated.contains(&format!("Self::__ice_derived_{name}(self)")),
                "missing derived setting binding `{name}`"
            );
        }
    }

    #[test]
    fn rejects_a_different_valid_builtin_id_in_an_app_setting_expression() {
        let source = format!(
            "app BuiltinContract\n  title trim(title)\n{THEME}state\n  title = \"Title\"\n  document:editor = editor(\"Body\")\nview\n  text editor_text(document)\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_app_setting_builtin_target(AppSettingExprId::Title, "editor");
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 2));
        assert!(error.message.contains("canonical contract"));
    }

    #[test]
    fn qualified_builtin_contract_does_not_resolve_a_same_name_sync_extern() {
        let source = format!(
            "app QualifiedBuiltin\n  title builtin::trim(title)\nextern crate::backend\n  sync trim(value:str) -> bool\n{THEME}state\n  title = \" Title \"\nview\n  text title\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let generated = crate::codegen::generate(&program, "qualified_builtin.ice").unwrap();
        assert!(generated.contains(".trim().to_owned()"));
    }

    #[test]
    fn rejects_a_contextual_builtin_binding_with_the_wrong_body_topology() {
        let source = format!(
            "app BindingContract\n  scale animation.project(progress, sample, sample)\n{THEME}state\n  progress:animation[f64] = 0.0\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_app_setting_binding_body_argument(AppSettingExprId::ScaleFactor, 0);
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 2));
        assert!(error.message.contains("body-argument topology"));
    }

    #[test]
    fn rejects_a_sibling_contextual_builtin_binding_reference_during_lowering() {
        let source = format!(
            "app BindingScope\n  scale animation.project(first, left, left) + animation.project(second, right, right)\n{THEME}state\n  first:animation[f64] = 0.0\n  second:animation[f64] = 0.0\nview\n  text \"ready\"\n"
        );
        assert!(lower(analyze(&source).unwrap()).is_ok());

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_app_setting_sibling_scoped_binding_reference(AppSettingExprId::ScaleFactor);
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 2));
        assert!(
            error
                .message
                .contains("outside its lexical scoped-value body")
        );
    }

    #[test]
    fn rejects_every_static_application_setting_topology_mutation_with_e196() {
        let source = format!(
            "daemon StaticSettings\n  id \"dev.example.original\"\n  executor iced::executor::Default\n  renderer crate::Renderer\n  font \"assets/one.ttf\"\n  font \"assets/two.ttf\"\n  text-size 14\n  antialiasing true\n  vsync true\n  window primary\n    icon-rgba \"assets/icon.rgba\" 1 1\n    platform linux\n      app-id \"dev.example.original\"\n  window child\n    size 640 480\nfont brand family=serif weight=bold default=true\n{THEME}view\n  text \"ready\"\n"
        );

        let mut checked = analyze(&source).unwrap();
        checked.document.daemon = false;
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 1));

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.renderer = Some("crate::Other".into());
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 4));

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.executor = Some("crate::Other".into());
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 3));

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.fonts[0].path = "/absolute/font.ttf".into();
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 5));

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.fonts[1].path = "assets/one.ttf".into();
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 6));

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.windows[1].name = "primary".into();
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 14));

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.windows[0]
            .settings
            .icon
            .as_mut()
            .unwrap()
            .path = "/absolute/icon.rgba".into();
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 11));

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.windows[0]
            .settings
            .linux
            .as_mut()
            .unwrap()
            .application_id = Some("changed".into());
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 13));

        fn reject_default_font_mutation(
            source: &str,
            mutate: impl FnOnce(&mut FontDecl),
            expected_line: usize,
        ) {
            let mut checked = analyze(source).unwrap();
            mutate(&mut checked.document.fonts[0]);
            let error = lower(checked).unwrap_err();
            assert_eq!((error.code, error.line), ("E196", expected_line));
            assert!(error.message.contains("changed after semantic analysis"));
        }

        reject_default_font_mutation(&source, |font| font.family = FontFamily::Monospace, 16);
        reject_default_font_mutation(&source, |font| font.weight = FontWeight::Thin, 16);
        reject_default_font_mutation(
            &source,
            |font| font.stretch = FontStretch::UltraExpanded,
            16,
        );
        reject_default_font_mutation(&source, |font| font.style = FontStyle::Oblique, 16);
        reject_default_font_mutation(&source, |font| font.span = Span::line(999), 999);
    }

    #[test]
    fn rejects_invalid_app_setting_expression_descendants_and_palette_ids_with_e196() {
        let binary_source = format!(
            "app InvalidSettingExpr\n  scale 1.0 + factor\n{THEME}state\n  factor = 1.0\nview\n  text \"ready\"\n"
        );
        let mut checked = analyze(&binary_source).unwrap();
        checked
            .facts
            .corrupt_app_setting_binary_child(AppSettingExprId::ScaleFactor);
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 2));
        assert!(
            error
                .message
                .contains("expression descendant ID is outside its arena")
        );

        let palette_source =
            format!("app InvalidPalette\n  palette AppTheme.app\n{THEME}view\n  text \"ready\"\n");
        let mut checked = analyze(&palette_source).unwrap();
        checked.facts.corrupt_app_setting_palette_id();
        let error = lower(checked).unwrap_err();
        assert_eq!((error.code, error.line), ("E196", 2));
        assert!(
            error
                .message
                .contains("expression declaration ID is outside its arena")
        );
    }

    #[test]
    #[ignore = "explicit repeated named-window HIR performance contract"]
    fn performance_contract_five_thousand_named_windows_lower_linearly() {
        const WINDOWS: usize = 5_000;
        let mut source = String::from("daemon WindowPerf\n");
        for index in 0..WINDOWS {
            writeln!(source, "  window window_{index}\n    size 640 480").unwrap();
        }
        source.push_str(THEME);
        source.push_str("view\n  text \"ready\"\n");
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.settings().named_windows.len(), WINDOWS);
        for (index, window) in program.settings().named_windows.iter().enumerate() {
            assert_eq!(window.id, NamedWindowId(index as u32));
        }
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "5k named windows lowered in {elapsed:?}"
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
    #[ignore = "large generated-source performance contract"]
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
        source.push_str("view\n  col\n");
        for _ in 0..CALLS {
            source.push_str("    Wide value<->draft\n      events\n");
            for index in 0..EVENTS {
                writeln!(source, "        event_{index} -> changed").unwrap();
            }
        }

        let checked = analyze(&source).unwrap();
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

    #[test]
    fn normalizes_recipe_inheritance_precedence_and_every_utility_variant() {
        let source = r#"app Styles
recipe action for button
  px-16px py-11px bg-surface/75 hover:bg-primary pressed:bg-danger disabled:bg-border disabled:text-fg disabled:opacity-25 border border-border rounded-9px text-12.5px leading-snug font-semibold
recipe emphasized for button extends action
  bg-primary hover:bg-danger
recipe destructive for button extends emphasized
  pressed:bg-primary disabled:bg-surface text-fg
recipe field for input
  w-full border border-border focus:border-primary rounded-md
theme contract AppTheme
  bg
  fg
  primary
  danger
  surface
  border
palette app for AppTheme
  bg #101010
  fg #f0f0f0
  primary #336699
  danger #cc0000
  surface #202020
  border #404040
state
  value = ""
on pressed
view
  col
    button "Delete" @destructive bg-danger -> pressed
    input "Name" <-> value @field
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        assert_eq!(program.styles.recipes.len(), 4);
        assert!(matches!(
            program.theme().active_palette,
            ResolvedPaletteSelection::Static(PaletteId(0))
        ));
        let destructive = &program.styles.recipes[2];
        assert_eq!(destructive.base, Some(RecipeId(1)));
        assert_eq!(destructive.declared_utilities.len(), 3);
        assert!(matches!(
            destructive.style.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                opacity: None,
            })
        ));
        assert!(matches!(
            destructive.style.hover_background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 3, .. }),
                ..
            })
        ));
        assert!(matches!(
            destructive.style.pressed_background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                ..
            })
        ));
        assert_eq!(destructive.style.disabled_opacity, Some(0.25));
        assert_eq!(destructive.style.padding, [11, 16, 11, 16]);
        assert_eq!(destructive.style.radius, 9);
        assert_eq!(destructive.style.text_size, Some(12.5));
        assert_eq!(destructive.style.text_line_height, Some(1.35));
        assert_eq!(
            destructive.style.font_weight,
            Some(ResolvedStyleFontWeight::Semibold)
        );

        let ViewNode::Layout { children, .. } = &program.document.view else {
            panic!("fixture view must be a layout");
        };
        let button = program.style_use(children[0].span()).unwrap();
        assert_eq!(button.recipes, [RecipeId(2)]);
        assert_eq!(button.utilities.len(), 1);
        assert!(matches!(
            button.style.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 3, .. }),
                ..
            })
        ));
        let input = program.style_use(children[1].span()).unwrap();
        assert!(matches!(
            input.style.focus_border_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                ..
            })
        ));
    }

    #[test]
    fn preserves_maximum_exact_padding_without_sentinel_values() {
        let source = format!(
            r#"app Padding
recipe all for box
  p-65535px
recipe axes for box
  px-65535px py-65534px
{THEME}view
  col
    box @all
      text "all"
    box @axes
      text "axes"
"#
        );
        let program = lower(analyze(&source).unwrap()).unwrap();

        assert_eq!(program.styles.recipes[0].style.padding, [u16::MAX; 4]);
        assert_eq!(
            program.styles.recipes[1].style.padding,
            [u16::MAX - 1, u16::MAX, u16::MAX - 1, u16::MAX]
        );
    }

    #[test]
    fn resolves_theme_contract_palettes_and_native_factories() {
        let source = r#"extern crate::backend
  theme native_theme(dark:bool)
app Themes
  theme native_theme(dark)
  palette active_palette
theme contract Ducktape
  bg
  fg
  primary
  danger
  surface
palette light for Ducktape
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
  surface #f4f4f480
palette dark for Ducktape
  bg #111111
  fg #ffffff
  primary #88aaff
  danger #ff6677
  surface #222222
state
  dark = false
  active_palette:palette[Ducktape] = Ducktape.light
view
  theme native_theme(!dark) fg=fg bg=linear(1.57, surface@0.0, bg@1.0)
    text "Theme" @text-surface/60
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        let theme = program.theme();
        assert_eq!(theme.contract.name, "Ducktape");
        assert_eq!(
            theme
                .contract
                .tokens
                .iter()
                .map(|token| token.name.as_str())
                .collect::<Vec<_>>(),
            ["bg", "fg", "primary", "danger", "surface"]
        );
        assert_eq!(theme.palettes.len(), 2);
        assert_eq!(theme.palettes[0].colors[4].rgba, [244, 244, 244, 128]);
        assert!(matches!(
            theme.active_palette,
            ResolvedPaletteSelection::Dynamic(_)
        ));
        let ResolvedAppThemeSelection::Factory(factory) = &theme.app_theme else {
            panic!("app theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "native_theme"
        );
        let nested = program
            .nested_theme(&program.document.view.span().clone())
            .unwrap();
        let ResolvedThemePreset::Factory(factory) = &nested.preset else {
            panic!("nested theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "native_theme"
        );
        assert!(matches!(
            nested.text,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 1, .. }),
                ..
            })
        ));
        let ResolvedBackground::Linear { stops, .. } = &nested.background.as_ref().unwrap() else {
            panic!("nested gradient must be normalized");
        };
        assert_eq!(stops.len(), 2);
        let ViewNode::Theme { content, .. } = &program.document.view else {
            panic!("fixture root must be a theme");
        };
        let style = program.style_use(content.span()).unwrap();
        assert!(matches!(
            style.style.text_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 4, .. }),
                opacity: Some(60),
            })
        ));
    }

    #[test]
    fn rejects_checked_style_and_palette_states_that_cannot_be_normalized() {
        let source = format!(
            "app Demo\nrecipe label for text\n  text-fg\n{THEME}view\n  text \"ok\" @label\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.document.recipes[0].base = Some("missing".into());
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("unknown checked recipe base `missing`")
        );

        let mut checked = analyze(&source).unwrap();
        checked.document.palettes[0].colors.remove("fg");
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("missing checked token `fg`"));

        let mut checked = analyze(&source).unwrap();
        checked.document.recipes[0].utilities[0] = "rounded-nope".into();
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked radius utility"));

        let inheritance = format!(
            "app Demo\nrecipe base for text\n  text-fg\nrecipe child for text extends base\n  font-bold\n{THEME}view\n  text \"ok\" @child\n"
        );
        let mut checked = analyze(&inheritance).unwrap();
        checked.document.recipes[1].target = StyleRecipeTarget::Container;
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains(
                "recipe `child` targets `box` but its checked base `base` targets `text`"
            )
        );

        let mut checked = analyze(&inheritance).unwrap();
        checked.document.recipes[0].base = Some("child".into());
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("checked recipe cycle includes"));
    }

    #[test]
    fn classifies_static_app_and_nested_builtin_theme_choices() {
        let source = r#"app StaticThemes
  theme "dark"
  palette AppTheme.second
theme contract AppTheme
  bg
  fg
  primary
  danger
palette first for AppTheme
  bg #000000
  fg #ffffff
  primary #336699
  danger #cc0000
palette second for AppTheme
  bg #ffffff
  fg #000000
  primary #6688cc
  danger #dd3344
view
  theme light
    text "built in"
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        assert!(matches!(
            program.theme().active_palette,
            ResolvedPaletteSelection::Static(PaletteId(1))
        ));
        assert!(matches!(
            &program.theme().app_theme,
            ResolvedAppThemeSelection::BuiltIn(name) if name == "dark"
        ));
        let nested = program.nested_theme(program.document.view.span()).unwrap();
        assert!(matches!(
            &nested.preset,
            ResolvedThemePreset::BuiltIn(name) if name == "light"
        ));

        let explicit_default = source.replace("theme \"dark\"", "theme \"default\"");
        let program = lower(analyze(&explicit_default).unwrap()).unwrap();
        assert!(matches!(
            program.theme().app_theme,
            ResolvedAppThemeSelection::Default
        ));
    }

    #[test]
    fn resolves_namespaced_recipe_origins_without_losing_the_physical_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-style-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("styles.ice");
        fs::write(
            &root,
            format!(
                "app Demo\n  theme ui::native_theme(dark)\nuse \"styles.ice\" as ui\n{THEME}state\n  dark = false\nview\n  theme ui::native_theme(!dark)\n    text \"Imported\" @ui::emphasis\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "extern crate::backend\n  theme native_theme(dark:bool)\nrecipe label for text\n  text-fg\nrecipe emphasis for text extends label\n  font-bold\ncomponent Decorated()\n  box @bg-primary\n    text \"decorated\"\n",
        )
        .unwrap();

        let program = lower(analyze_file(&root).unwrap()).unwrap();
        let recipe = program
            .styles
            .recipes
            .iter()
            .find(|recipe| recipe.name == "ui::emphasis")
            .unwrap();
        let origin = program.origin(recipe.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 5);
        let ResolvedAppThemeSelection::Factory(factory) = &program.theme().app_theme else {
            panic!("namespaced app theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "ui::native_theme"
        );
        let contract_origin = program.origin(program.theme().contract.origin);
        assert_eq!(contract_origin.path.as_deref(), Some(root.as_path()));
        let ViewNode::Theme { content, span, .. } = &program.document.view else {
            panic!("fixture root must be a nested theme");
        };
        let nested = program.nested_theme(span).unwrap();
        let ResolvedThemePreset::Factory(factory) = &nested.preset else {
            panic!("namespaced nested theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "ui::native_theme"
        );
        assert_eq!(
            program.origin(nested.origin).path.as_deref(),
            Some(root.as_path())
        );
        let style = program.style_use(content.span()).unwrap();
        assert_eq!(style.recipes, [recipe.id]);
        let imported_style = program
            .styles
            .style_uses
            .iter()
            .find(|style| {
                style.style.background
                    == Some(ResolvedThemeColor {
                        base: ResolvedThemeColorBase::Token(program.theme().native_tokens.primary),
                        opacity: None,
                    })
            })
            .expect("imported component style must be lowered");
        assert_eq!(
            program.origin(imported_style.origin).path.as_deref(),
            Some(imported.as_path())
        );
        assert_eq!(program.origin(imported_style.origin).line, 8);
        assert_eq!(
            imported_style.style.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(program.theme().native_tokens.primary),
                opacity: None,
            })
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_setting_dependencies_keep_origins_and_generated_source_markers() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-setting-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("settings.ice");
        fs::write(
            &root,
            format!(
                "daemon ImportedSettings\n  title tools::describe(window)\n  theme tools::native_theme(window)\n  scale tools::scale(window)\n  id \"dev.example.imported\"\n  executor iced::executor::Default\n  renderer crate::backend::Renderer\n  font \"brand.ttf\"\n  window primary\n    icon-rgba \"icon.rgba\" 1 1\n    platform linux\n      app-id \"dev.example.imported\"\nfont brand family=\"Brand Sans\" weight=semibold stretch=semi-expanded style=italic default=true\nuse \"settings.ice\" as tools\n{THEME}view\n  text tools::describe(window)\n"
            ),
        )
        .unwrap();
        fs::write(directory.join("brand.ttf"), b"font").unwrap();
        fs::write(directory.join("icon.rgba"), [0_u8, 0, 0, 0]).unwrap();
        fs::write(
            &imported,
            "extern crate::backend\n  sync describe(id:window-id) -> str\n  sync scale(id:window-id) -> f64\n  theme native_theme(id:window-id)\n",
        )
        .unwrap();

        let program = lower(analyze_file(&root).unwrap()).unwrap();
        let ResolvedAppThemeSelection::Factory(factory) = &program.theme().app_theme else {
            panic!("imported app theme must be a resolved factory");
        };
        let function = program.extern_function(factory.function);
        assert_eq!(function.name, "tools::native_theme");
        assert_eq!(
            program.origin(function.declaration.origin).path.as_deref(),
            Some(imported.as_path())
        );
        let title = program.settings().title.as_ref().unwrap();
        assert_eq!(
            program.origin(title.origin).path.as_deref(),
            Some(root.as_path())
        );
        assert_eq!(program.origin(title.origin).line, 2);
        let title_root = program.checked_facts().expression(
            program
                .checked_facts()
                .expression_use(title.expression)
                .root,
        );
        let crate::check::CheckedExprKind::Call {
            target: crate::check::CheckedCallTarget::Extern(function),
            ..
        } = title_root.kind
        else {
            panic!("title must retain its imported extern target ID");
        };
        assert_eq!(program.extern_function(function).name, "tools::describe");

        let generated = crate::codegen::generate(&program, &root.display().to_string()).unwrap();
        let encoded_root = root
            .display()
            .to_string()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_root}\nfn __title")));
        for (line, fragment) in [
            (5, "id: ::std::option::Option::Some"),
            (6, ".executor::<iced::executor::Default>()"),
            (7, "type __IceRenderer = crate::backend::Renderer"),
            (8, ".font(include_bytes!"),
            (10, "icon: ::std::option::Option::Some"),
            (12, "__platform.application_id"),
            (13, "#[must_use]\npub fn default_font"),
        ] {
            assert!(
                generated.contains(&format!(
                    "// __ICE_SOURCE {line} 1 {encoded_root}\n{fragment}"
                )),
                "static setting `{fragment}` must retain its exact source declaration"
            );
        }
        assert!(generated.contains("crate::backend::describe(window)"));
        assert!(generated.contains("crate::backend::native_theme(window)"));
        assert!(generated.contains("crate::backend::scale(window)"));

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "explicit large style lowering performance contract"]
    fn performance_contract_lowering_many_deep_recipes_and_uses_has_constant_per_use_recipe_work() {
        const TOKENS: usize = 128;
        const RECIPES: usize = 256;
        const USES: usize = 10_000;
        let mut source = String::from(
            "app StylePerf\ntheme contract PerfTheme\n  bg\n  fg\n  primary\n  danger\n",
        );
        for index in 0..TOKENS {
            writeln!(source, "  token_{index}").unwrap();
        }
        source.push_str(
            "palette app for PerfTheme\n  bg #000000\n  fg #ffffff\n  primary #336699\n  danger #cc0000\n",
        );
        for index in 0..TOKENS {
            writeln!(source, "  token_{index} #{:06x}", index + 1).unwrap();
        }
        source.push_str("recipe recipe_0 for text\n  text-token_0\n");
        for index in 1..RECIPES {
            writeln!(
                source,
                "recipe recipe_{index} for text extends recipe_{}\n  text-token_{}",
                index - 1,
                index % TOKENS
            )
            .unwrap();
        }
        source.push_str("view\n  col\n");
        for index in 0..USES {
            writeln!(source, "    text \"row-{index}\" @recipe_{}", RECIPES - 1).unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        eprintln!("normalized {TOKENS} tokens, {RECIPES} recipes, and {USES} uses in {elapsed:?}");
        assert_eq!(program.theme().contract.tokens.len(), TOKENS + 4);
        assert_eq!(program.styles.recipes.len(), RECIPES);
        assert_eq!(program.styles.style_uses.len(), USES + 1);
        assert_eq!(
            program
                .styles
                .style_uses
                .iter()
                .map(|style| style.utilities.len())
                .sum::<usize>(),
            0,
            "recipe uses retain IDs and fixed-size styles, never inherited utility copies"
        );
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "128 tokens, 256 recipes, and 10k recipe uses lowered in {elapsed:?}"
        );
    }
}
