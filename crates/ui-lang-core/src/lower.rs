use crate::ast::*;
use crate::check::{
    BuiltinArgumentContext, CheckedBinaryOperator, CheckedBooleanControl, CheckedCallArgument,
    CheckedCallTarget, CheckedCanvas, CheckedCanvasRouteArg, CheckedCanvasRouteTarget,
    CheckedComboBox, CheckedComponentArgumentSource, CheckedComponentEventDelivery, CheckedExprId,
    CheckedExprKind, CheckedExprOwner, CheckedExprUse, CheckedExternViewAdapter, CheckedFacts,
    CheckedInitializerCoercion, CheckedInput, CheckedInteraction, CheckedInteractionKind,
    CheckedLayout, CheckedLocalId, CheckedLocalOwner, CheckedMarkdown, CheckedMatchArm,
    CheckedMatchPattern, CheckedMedia, CheckedPaneAxis, CheckedPaneBackground,
    CheckedPaneConfiguration, CheckedPaneGrid, CheckedPaneGridStyle, CheckedPaneLength,
    CheckedPanePadding, CheckedPaneRadius, CheckedPaneStyleSite, CheckedPaneSurface,
    CheckedPaneTemplate, CheckedPaneTitle, CheckedPaneView, CheckedPathRoot, CheckedPickList,
    CheckedProjectionKind, CheckedTableLength, CheckedText, CheckedTooltip, CheckedUnaryOperator,
    CheckedValueRef, CheckedViewExprRole, CheckedViewFlow, CheckedViewLocalRole, CheckedViewScope,
    ContextualBuiltin, canonical_builtin_type, field_type, lazy_hashable, unify_type_evidence,
};
pub(crate) use crate::check::{
    CheckedExprUseId, CheckedKeyedLength, CheckedResponsiveLength, CheckedSubscription,
    CheckedSubscriptionExprRole, CheckedSubscriptionSource,
};
use crate::hir::Origin;
pub(crate) use crate::hir::{
    AppSettingExprId, AppStateId, CanvasCommandId, CanvasDeclaration, CanvasEventId,
    CanvasExpressionId, CanvasLocalId, ComponentCallId, ComponentEventId, ComponentId,
    ComponentParamId, ComponentSlotId, ComponentStateId, DeclarationIndex, ExternFnId, ExternRef,
    FloatExpressionId, HandlerId, HandlerOwner, InteractionExpressionId, InteractionRouteId,
    MediaExpressionId, NamedTypeId, NamedWindowId, OriginArena, OriginId, PaletteId,
    PinExpressionId, RouteId, RunLaneId, StatementId, SubscriptionId, TaskId, TestId, TestStepId,
    TestTargetId, TooltipExpressionId, ViewId,
};
#[cfg(test)]
use crate::hir::{AppSettingsId, CanvasRouteId};
use crate::semantic::*;
use crate::{CheckedControlledEditor, CheckedDocument, Error};
use std::collections::{HashMap, HashSet};
use std::path::Path;

mod boolean;
mod button;
mod canvas;
mod conditional;
mod container;
mod content_primitives;
mod editor;
mod expression;
mod extern_component;
mod extern_view_adapter;
mod float;
mod input;
mod interaction;
mod iteration;
mod keyed_column;
mod layout;
mod lazy;
mod markdown;
mod match_view;
mod media;
mod overlay;
mod pane_grid;
mod pin;
mod range_controls;
mod responsive;
mod selection;
mod style;
mod table;
mod testing;
mod text;
mod tooltip;
mod view_topology;

pub(crate) use boolean::*;
pub(crate) use button::*;
pub(crate) use canvas::*;
pub(crate) use conditional::*;
pub(crate) use container::*;
pub(crate) use content_primitives::*;
pub(crate) use editor::*;
pub(crate) use expression::*;
pub(crate) use extern_component::*;
pub(crate) use extern_view_adapter::*;
pub(crate) use float::*;
pub(crate) use input::*;
pub(crate) use interaction::*;
pub(crate) use iteration::*;
pub(crate) use keyed_column::*;
pub(crate) use layout::*;
pub(crate) use lazy::*;
pub(crate) use markdown::*;
pub(crate) use match_view::*;
pub(crate) use media::*;
pub(crate) use overlay::*;
pub(crate) use pane_grid::*;
pub(crate) use pin::*;
pub(crate) use range_controls::*;
pub(crate) use responsive::*;
pub(crate) use selection::*;
pub(crate) use table::*;
pub(crate) use text::*;

pub(crate) use style::*;
pub(crate) use tooltip::*;
pub(crate) use view_topology::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestTheme {
    Light,
    Dark,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestPlatform {
    Linux,
    Windows,
    Macos,
    Wasm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestMouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestWheelUnit {
    Pixels,
    Lines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestScrollMode {
    To,
    By,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestTouchPhase {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestAccessibilityAction {
    Activate,
    Focus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTestKey {
    Named(String),
    Character(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ResolvedTestKeyLocation {
    #[default]
    Standard,
    Left,
    Right,
    Numpad,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResolvedTestModifiers {
    pub(crate) shift: bool,
    pub(crate) control: bool,
    pub(crate) alt: bool,
    pub(crate) logo: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedTestKeyEvent {
    pub(crate) key: ResolvedTestKey,
    pub(crate) modified_key: Option<ResolvedTestKey>,
    pub(crate) location: ResolvedTestKeyLocation,
    pub(crate) physical: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) repeat: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTestConfig {
    pub(crate) viewport: Option<(f64, f64)>,
    pub(crate) timeout_ms: Option<u64>,
    pub(crate) theme: Option<ResolvedTestTheme>,
    pub(crate) scale_factor: Option<f64>,
    pub(crate) locale: Option<String>,
    pub(crate) platform: Option<ResolvedTestPlatform>,
    pub(crate) reduced_motion: Option<bool>,
    pub(crate) preset: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTestTargetSegment {
    pub(crate) name: String,
    pub(crate) key: Option<ResolvedExpressionId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTestTargetPath {
    pub(crate) segments: Vec<ResolvedTestTargetSegment>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTestTarget {
    pub(crate) id: TestTargetId,
    pub(crate) name: String,
    pub(crate) path: ResolvedTestTargetPath,
    pub(crate) local: CheckedLocalId,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTestTargetRef {
    Alias(TestTargetId),
    Id(ResolvedTestTargetPath),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTestComposition {
    Start,
    Update {
        value: ResolvedExpressionId,
        selection: Option<(ResolvedExpressionId, ResolvedExpressionId)>,
    },
    Commit(ResolvedExpressionId),
    Cancel,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTestAccessibilityProperty {
    Role(ResolvedExpressionId),
    Name(ResolvedExpressionId),
    Value(ResolvedExpressionId),
    Checked(ResolvedExpressionId),
    Disabled(ResolvedExpressionId),
    Focused(ResolvedExpressionId),
    Action {
        name: String,
        expected: ResolvedExpressionId,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTestExpectation {
    Expr {
        expression: ResolvedExpressionId,
        unwrap_operator: bool,
    },
    Equality {
        left: CheckedExprId,
        right: CheckedExprId,
        negated: bool,
        expression: ResolvedExpressionId,
    },
    Approx {
        left: ResolvedExpressionId,
        right: ResolvedExpressionId,
    },
    Exists(ResolvedTestTargetRef),
    Missing(ResolvedTestTargetRef),
    Text {
        value: ResolvedExpressionId,
        within: Option<ResolvedTestTargetRef>,
        negated: bool,
    },
    Accessibility {
        target: ResolvedTestTargetRef,
        property: ResolvedTestAccessibilityProperty,
    },
    Tray {
        field: ResolvedTrayField,
        value: ResolvedExpressionId,
        negated: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedTrayField {
    Label,
    Icon,
    Item,
    Command,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTestStepKind {
    Click {
        target: ResolvedTestTargetRef,
        button: ResolvedTestMouseButton,
        count: u8,
    },
    ClickAt {
        x: ResolvedExpressionId,
        y: ResolvedExpressionId,
        button: ResolvedTestMouseButton,
        count: u8,
    },
    Hover(ResolvedTestTargetRef),
    Enter(ResolvedTestTargetRef),
    Leave,
    MoveTarget(ResolvedTestTargetRef),
    MovePoint(ResolvedExpressionId, ResolvedExpressionId),
    Press {
        target: ResolvedTestTargetRef,
        button: ResolvedTestMouseButton,
    },
    Release(ResolvedTestMouseButton),
    Wheel {
        unit: ResolvedTestWheelUnit,
        x: ResolvedExpressionId,
        y: ResolvedExpressionId,
    },
    Scroll {
        mode: ResolvedTestScrollMode,
        target: ResolvedTestTargetRef,
        x: ResolvedExpressionId,
        y: ResolvedExpressionId,
    },
    Snap {
        target: ResolvedTestTargetRef,
        x: ResolvedExpressionId,
        y: ResolvedExpressionId,
    },
    SnapEnd(ResolvedTestTargetRef),
    Drag {
        from: ResolvedTestTargetRef,
        to: ResolvedTestTargetRef,
    },
    Drop(ResolvedTestTargetRef),
    Focus(ResolvedTestTargetRef),
    FocusNext,
    FocusPrevious,
    Blur,
    WindowFocus(bool),
    Type(ResolvedExpressionId),
    Clear,
    Replace(ResolvedExpressionId),
    Select(ResolvedExpressionId, ResolvedExpressionId),
    SelectAll,
    Cursor(ResolvedExpressionId),
    CursorFront,
    CursorEnd,
    Composition(ResolvedTestComposition),
    Key(ResolvedTestKey),
    KeyDown(ResolvedTestKeyEvent),
    KeyUp(ResolvedTestKeyEvent),
    Modifiers(ResolvedTestModifiers),
    Chord {
        modifiers: ResolvedTestModifiers,
        key: ResolvedTestKey,
    },
    Repeat {
        key: ResolvedTestKey,
        count: ResolvedExpressionId,
    },
    Tap {
        target: ResolvedTestTargetRef,
        count: u8,
    },
    Touch {
        phase: ResolvedTestTouchPhase,
        id: ResolvedExpressionId,
        x: ResolvedExpressionId,
        y: ResolvedExpressionId,
    },
    WindowMove(ResolvedExpressionId, ResolvedExpressionId),
    Resize(ResolvedExpressionId, ResolvedExpressionId),
    Rescale(ResolvedExpressionId),
    WindowClose,
    WindowOpened,
    WindowClosed,
    Redraw,
    SystemTheme(ResolvedTestTheme),
    FileHover(ResolvedExpressionId),
    FileDrop(ResolvedExpressionId),
    FileLeave,
    Wait(u64),
    Advance(u64),
    Idle,
    Capture(String),
    Accessibility {
        action: ResolvedTestAccessibilityAction,
        target: ResolvedTestTargetRef,
    },
    Dispatch {
        handler: HandlerId,
        handler_name: String,
        args: Vec<ResolvedExpressionId>,
    },
    TrayChoose(ResolvedExpressionId),
    Expect(ResolvedTestExpectation),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTestStep {
    pub(crate) id: TestStepId,
    pub(crate) kind: ResolvedTestStepKind,
    pub(crate) source: String,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTest {
    pub(crate) id: TestId,
    pub(crate) name: String,
    pub(crate) config: ResolvedTestConfig,
    pub(crate) window_local: Option<CheckedLocalId>,
    pub(crate) mount: Option<ViewId>,
    pub(crate) targets: Vec<ResolvedTestTarget>,
    pub(crate) steps: Vec<ResolvedTestStep>,
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
struct ComponentParamContract {
    id: ComponentParamId,
    name: String,
    ty: Type,
    capability: ParamCapability,
    #[cfg(test)]
    default: Option<CheckedExprUseId>,
    #[cfg(test)]
    origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamCapability {
    Read,
    Bind,
}
#[derive(Clone, Debug)]
struct ComponentEventContract {
    id: ComponentEventId,
    name: String,
    payloads: Vec<Type>,
}
#[derive(Clone, Debug)]
struct ComponentSlotContract {
    id: ComponentSlotId,
    name: String,
    optional: bool,
    origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) struct ComponentStateContract {
    pub(crate) id: ComponentStateId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: ResolvedInitializer,
    pub(crate) span: Span,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) struct AppStateContract {
    pub(crate) id: AppStateId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: ResolvedInitializer,
    pub(crate) span: Span,
    #[cfg(test)]
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
struct ResolvedControlledInputBinding {
    state: AppStateId,
    name: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedControlledEditorBinding {
    pub(crate) state: AppStateId,
    pub(crate) name: String,
    pub(crate) action: Option<ExternFnId>,
}
#[derive(Clone, Debug)]
pub(crate) struct DerivedContract {
    pub(crate) id: crate::hir::DerivedId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: CheckedExprUseId,
    pub(crate) span: Span,
    #[cfg(test)]
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
pub(crate) struct ResolvedExternContract {
    #[cfg(test)]
    pub(crate) id: ExternFnId,
    pub(crate) name: String,
    pub(crate) rust_path: String,
    pub(crate) params: Vec<ResolvedType>,
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
        arguments: Vec<ResolvedExpressionId>,
    },
    Recipe {
        function: ResolvedExternContract,
        arguments: Vec<ResolvedExpressionId>,
    },
    Events {
        identity: ResolvedExpressionId,
        filter: ResolvedExternContract,
    },
    Event {
        raw: bool,
    },
    Extern {
        function: ResolvedExternContract,
        arguments: Vec<ResolvedExpressionId>,
    },
    InputMethod(InputMethodEvent),
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    SystemTheme,
    Touch(TouchEvent),
    Window(WindowEvent),
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSubscriptionRoute {
    #[cfg(test)]
    pub(crate) handler: HandlerId,
    pub(crate) handler_name: String,
    pub(crate) args: Vec<ResolvedRouteArg>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSubscription {
    pub(crate) source: ResolvedSubscriptionSource,
    pub(crate) source_payloads: Vec<ResolvedType>,
    pub(crate) delivered_payloads: Vec<ResolvedType>,
    pub(crate) filter: Option<ResolvedExternContract>,
    pub(crate) context: Option<ResolvedExpressionId>,
    pub(crate) condition: Option<ResolvedExpressionId>,
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
    route_args: Vec<ResolvedRouteArg>,
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
        if !resolved_subscription_supports_window_id(source) {
            return None;
        }
        payloads.insert(0, Type::WindowId);
    }
    Some(payloads)
}

fn resolved_subscription_supports_window_id(source: &CheckedSubscriptionSource) -> bool {
    matches!(source, CheckedSubscriptionSource::Event { .. })
        || matches!(
            source,
            CheckedSubscriptionSource::Window(event) if *event != WindowEvent::Frame
        )
}

/// A view route addresses its own component's handlers; app and test views
/// address app handlers.
fn route_handler_owner_is_reachable(owner: HandlerOwner, scope: CheckedViewScope) -> bool {
    match (owner, scope) {
        (HandlerOwner::App, CheckedViewScope::App | CheckedViewScope::Test(_)) => true,
        (HandlerOwner::Component(handler), CheckedViewScope::Component(view)) => handler == view,
        _ => false,
    }
}

fn resolved_subscription_supports_status(source: &CheckedSubscriptionSource) -> bool {
    matches!(
        source,
        CheckedSubscriptionSource::Event { .. }
            | CheckedSubscriptionSource::InputMethod(_)
            | CheckedSubscriptionSource::Keyboard(_)
            | CheckedSubscriptionSource::Mouse(_)
            | CheckedSubscriptionSource::Touch(_)
            | CheckedSubscriptionSource::Window(
                WindowEvent::Opened
                    | WindowEvent::Closed
                    | WindowEvent::Moved
                    | WindowEvent::Resized
                    | WindowEvent::Rescaled
                    | WindowEvent::CloseRequested
                    | WindowEvent::Focused
                    | WindowEvent::Unfocused
                    | WindowEvent::FileHovered
                    | WindowEvent::FileDropped
                    | WindowEvent::FilesHoveredLeft
            )
    )
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
    pub(crate) from: Option<AnimationStart>,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedHandler {
    #[cfg(test)]
    pub(crate) id: HandlerId,
    pub(crate) owner: HandlerOwner,
    pub(crate) name: String,
    pub(crate) params: Vec<ResolvedHandlerParam>,
    pub(crate) statements: Vec<ResolvedStatement>,
    #[cfg(test)]
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
#[derive(Clone, Debug)]
pub(crate) struct ResolvedAppExpression {
    pub(crate) expression: CheckedExprUseId,
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedHandlerParam {
    pub(crate) local: crate::check::CheckedLocalId,
    pub(crate) name: String,
    pub(crate) ty: Type,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedStatement {
    pub(crate) id: StatementId,
    pub(crate) kind: ResolvedStatementKind,
    pub(crate) task: Option<TaskId>,
    #[cfg(test)]
    pub(crate) is_final: bool,
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedFontAsset {
    pub(crate) path: String,
    pub(crate) origin: OriginId,
}
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
    pub(crate) lane: Option<RunLaneId>,
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
#[derive(Clone, Debug)]
pub(crate) struct ResolvedDefaultFont {
    pub(crate) family: FontFamily,
    pub(crate) weight: FontWeight,
    pub(crate) stretch: FontStretch,
    pub(crate) style: FontStyle,
    pub(crate) origin: OriginId,
}
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
#[derive(Clone, Debug)]
pub(crate) struct ResolvedRoute {
    pub(crate) id: RouteId,
    pub(crate) target: ResolvedRouteTarget,
    pub(crate) args: Vec<ResolvedRouteArg>,
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedLinuxWindowSettings {
    pub(crate) application_id: Option<String>,
    pub(crate) override_redirect: Option<bool>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}
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
#[derive(Clone, Debug)]
pub(crate) struct ResolvedWindowsWindowSettings {
    pub(crate) drag_and_drop: Option<bool>,
    pub(crate) skip_taskbar: Option<bool>,
    pub(crate) undecorated_shadow: Option<bool>,
    pub(crate) corner: Option<ResolvedWindowCorner>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) enum ResolvedRouteArg {
    Expression(CheckedExprUseId),
    Payload {
        index: u32,
        #[cfg(test)]
        ty: Type,
    },
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
                payload_index += 1;
                lower_typed_payload_argument(
                    &route.span,
                    inputs.source_payloads,
                    source_index as u32,
                    target,
                )
            }
        })
        .collect()
}

fn lower_typed_payload_argument(
    span: &Span,
    source_payloads: &[Type],
    index: u32,
    target: &Type,
) -> Result<ResolvedRouteArg, Error> {
    let source = source_payloads
        .get(index as usize)
        .ok_or_else(|| Error::new("E196", span, "route payload has no typed source contract"))?;
    if source != target {
        return Err(Error::new(
            "E196",
            span,
            "route payload type diverged from its checked target parameter",
        ));
    }
    Ok(ResolvedRouteArg::Payload {
        index,
        #[cfg(test)]
        ty: target.clone(),
    })
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
#[derive(Clone, Debug)]
pub(crate) struct ResolvedMacosWindowSettings {
    pub(crate) title_hidden: Option<bool>,
    pub(crate) titlebar_transparent: Option<bool>,
    pub(crate) fullsize_content_view: Option<bool>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedWasmWindowSettings {
    pub(crate) target: Option<Option<String>>,
    pub(crate) field_origins: HashMap<String, OriginId>,
    pub(crate) origin: OriginId,
}
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
#[derive(Clone, Debug)]
pub(crate) struct ResolvedNamedWindow {
    pub(crate) id: NamedWindowId,
    #[cfg(test)]
    pub(crate) name: String,
    pub(crate) settings: ResolvedWindowSettings,
    pub(crate) origin: OriginId,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedTraySettings {
    pub(crate) icons: Vec<ResolvedTrayIcon>,
    pub(crate) icon_template: bool,
    pub(crate) label: Option<ResolvedTrayText>,
    pub(crate) tooltip: Option<ResolvedTrayText>,
    pub(crate) menu: Vec<ResolvedTrayRow>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedTrayIcon {
    pub(crate) icon: ResolvedWindowIcon,
    pub(crate) when: Option<ResolvedAppExpression>,
}

/// A tray string, split by whether it can ever change. A literal is applied
/// once at startup; only the rest reaches `__tray_sync`.
#[derive(Clone, Debug)]
pub(crate) enum ResolvedTrayText {
    Literal(String),
    Expression(ResolvedAppExpression),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedTrayRow {
    Item {
        text: ResolvedTrayText,
        /// The handler a chosen row calls, by name — the same thing a
        /// `subscribe` route carries, so codegen reaches the handler through
        /// the one path payload-free routes already use.
        route: Option<String>,
    },
    Separator,
}

impl ResolvedTraySettings {
    /// Whether any tray expression can change after startup, which is what
    /// `__tray_sync` exists for.
    pub(crate) fn reactive(&self) -> bool {
        let dynamic =
            |text: &Option<ResolvedTrayText>| matches!(text, Some(ResolvedTrayText::Expression(_)));
        dynamic(&self.label)
            || dynamic(&self.tooltip)
            || self.icons.iter().any(|icon| icon.when.is_some())
            || self.menu.iter().any(|row| {
                matches!(
                    row,
                    ResolvedTrayRow::Item {
                        text: ResolvedTrayText::Expression(_),
                        ..
                    }
                )
            })
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedAppSettings {
    #[cfg(test)]
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
    pub(crate) tray: Option<ResolvedTraySettings>,
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
    pub(crate) root: ViewId,
    pub(crate) storage: ComponentStorage,
    #[cfg(test)]
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

    pub(crate) fn checked_ref(&self) -> CheckedValueRef {
        match self {
            Self::App { id, .. } => CheckedValueRef::AppState(*id),
            Self::ComponentParam { id, .. } => CheckedValueRef::ComponentParam(*id),
            Self::ComponentState { id, .. } => CheckedValueRef::ComponentState(*id),
        }
    }

    pub(crate) fn accepts_type(&self, ty: &Type) -> bool {
        *ty == Type::Str
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedArgument {
    pub(crate) param: ComponentParamId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) expression: CheckedExprUseId,
    scope: ArgumentScope,
    pub(crate) writable: Option<WritableStateRef>,
}

impl ResolvedArgument {
    pub(crate) fn uses_definition_scope(&self) -> bool {
        self.scope == ArgumentScope::Definition
    }
}
#[derive(Clone, Debug)]
pub(crate) enum ResolvedEventRoute {
    Direct {
        route_index: u32,
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
        route: ResolvedInteractionRoute,
        origin: OriginId,
    },
    Forward {
        route_index: u32,
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
        outer_component: ComponentId,
        outer_component_name: String,
        outer_event: ComponentEventId,
        outer_event_name: String,
        outer_payloads: Vec<Type>,
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
#[derive(Clone, Debug)]
pub(crate) struct ResolvedSlot {
    pub(crate) slot: ComponentSlotId,
    pub(crate) name: String,
    #[cfg(test)]
    pub(crate) optional: bool,
    pub(crate) content: Option<ViewId>,
}
#[derive(Clone, Debug)]
pub(crate) enum ComponentScope {
    Explicit,
    Implicit { call_site: usize },
}
#[derive(Clone, Debug)]
pub(crate) enum ComponentOutputRoute {
    None,
    Direct {
        output: Type,
        route: ResolvedInteractionRoute,
    },
}
#[derive(Clone, Debug)]
pub(crate) struct ComponentCall {
    pub(crate) id: ComponentCallId,
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
#[derive(Debug)]
pub(crate) struct LoweredProgram {
    // Adversarial tests retain a poisonable source sidecar to prove that
    // production code generation never observes the source AST. Release
    // builds do not store it.
    #[cfg(test)]
    document: Document,
    app_view: ViewId,
    expressions: ResolvedExpressionProgram,
    // Adversarial tests retain checker facts only to verify lowering
    // invariants after deliberate HIR corruption. Release programs do not
    // carry checker state into the backend boundary.
    #[cfg(test)]
    facts: CheckedFacts,
    declarations: DeclarationIndex,
    settings: ResolvedAppSettings,
    subscriptions: Vec<ResolvedSubscription>,
    tests: Vec<ResolvedTest>,
    canvases: HashMap<ViewId, ResolvedCanvas>,
    containers: HashMap<ViewId, ResolvedContainer>,
    layouts: HashMap<ViewId, ResolvedLayout>,
    texts: HashMap<ViewId, ResolvedText>,
    buttons: HashMap<ViewId, ResolvedButton>,
    inputs: HashMap<ViewId, ResolvedInput>,
    text_editors: HashMap<ViewId, ResolvedTextEditor>,
    markdowns: HashMap<ViewId, ResolvedMarkdown>,
    extern_component_declarations: Vec<ResolvedExternComponentDeclaration>,
    extern_components: HashMap<ViewId, ResolvedExternComponent>,
    themers: HashMap<ViewId, ResolvedThemer>,
    shaders: HashMap<ViewId, ResolvedShader>,
    boolean_controls: HashMap<ViewId, ResolvedBooleanControl>,
    pick_lists: HashMap<ViewId, ResolvedPickList>,
    combo_boxes: HashMap<ViewId, ResolvedComboBox>,
    sliders: HashMap<ViewId, ResolvedSlider>,
    progresses: HashMap<ViewId, ResolvedProgress>,
    rules: HashMap<ViewId, ResolvedRule>,
    qr_codes: HashMap<ViewId, ResolvedQrCode>,
    spaces: HashMap<ViewId, ResolvedSpace>,
    controlled_inputs: Vec<ResolvedControlledInputBinding>,
    controlled_editors: Vec<ResolvedControlledEditorBinding>,
    controlled_editors_by_name: HashMap<String, usize>,
    media: HashMap<ViewId, ResolvedMedia>,
    overlays: HashMap<ViewId, ResolvedOverlay>,
    tooltips: HashMap<ViewId, ResolvedTooltip>,
    floats: HashMap<ViewId, ResolvedFloat>,
    pins: HashMap<ViewId, ResolvedPin>,
    responsives: HashMap<ViewId, ResolvedResponsive>,
    keyed_columns: HashMap<ViewId, ResolvedKeyedColumn>,
    conditionals: HashMap<ViewId, ResolvedConditional>,
    iterations: HashMap<ViewId, ResolvedIteration>,
    match_views: HashMap<ViewId, ResolvedMatch>,
    lazy_views: HashMap<ViewId, ResolvedLazy>,
    tables: HashMap<ViewId, ResolvedTable>,
    pane_grids: HashMap<ViewId, ResolvedPaneGrid>,
    interaction_widgets: HashMap<ViewId, ResolvedInteractionWidget>,
    views: Vec<ResolvedView>,
    test_mounts: HashMap<TestId, ViewId>,
    preset_names: Vec<String>,
    named_type_rust_paths: HashMap<NamedTypeId, String>,
    app_states: Vec<AppStateContract>,
    derived: Vec<DerivedContract>,
    components: Vec<ComponentContract>,
    handlers: Vec<ResolvedHandler>,
    app_handlers: Vec<HandlerId>,
    preset_handlers: Vec<HandlerId>,
    calls: Vec<ComponentCall>,
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
                CheckedCallTarget::Extern(reference) => {
                    declarations.try_extern_decl(reference.id).is_some()
                }
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

#[cfg(test)]
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
        ResolvedStatementKind::Run(run) => {
            let lane = run
                .lane
                .map(|lane| {
                    program
                        .declarations
                        .try_run_lane(lane)
                        .map(|lane| lane.name.as_str())
                        .ok_or_else(|| {
                            program.invariant_at_origin(
                                statement.origin,
                                "request-lane ID is outside its arena",
                            )
                        })
                })
                .transpose()?
                .unwrap_or_default();
            format!(
                "run:{:?}:{:?}:{lane}:{}:{}:{}:{}",
                run.kind,
                run.mode,
                effect_name(program, &run.target, statement.origin)?,
                run.args.len(),
                route_shape(&run.success),
                run.error.as_ref().map(route_shape).unwrap_or_default()
            )
        }
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

#[cfg(test)]
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
                                .settings
                                .named_windows
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
    #[cfg(test)]
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
                    if (run.mode == FutureMode::Every) != run.lane.is_none()
                        || run.lane != declaration.run_lane
                    {
                        return Err(program.invariant_at_origin(
                            statement.origin,
                            "run mode and request-lane cardinality diverged",
                        ));
                    }
                    if let Some(lane) = run.lane {
                        let run_lane =
                            program.declarations.try_run_lane(lane).ok_or_else(|| {
                                program.invariant_at_origin(
                                    statement.origin,
                                    "request-lane ID is outside its arena",
                                )
                            })?;
                        let owner = match handler.owner {
                            HandlerOwner::Preset(_) => HandlerOwner::App,
                            owner => owner,
                        };
                        if run_lane.owner != owner
                            || run_lane.mode != run.mode
                            || !run_lane.statements.contains(&statement.id)
                        {
                            return Err(program.invariant_at_origin(
                                statement.origin,
                                "request-lane ID belongs to a different owner, statement, or mode",
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
            if declaration.owner != handler.owner
                || declaration.name != handler.name
                || declaration.declaration.origin != handler.origin
                || declaration
                    .payloads
                    .iter()
                    .ne(handler.params.iter().map(|param| &param.ty))
            {
                return Err(self.invariant_at_origin(
                    handler.origin,
                    "handler identity, origin, or payloads diverged from its declaration",
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

    pub(crate) fn app_view(&self) -> ViewId {
        self.app_view
    }

    pub(crate) fn resolved_view(&self, id: ViewId) -> Result<&ResolvedView, Error> {
        self.views
            .get(id.0 as usize)
            .filter(|view| view.id == id)
            .ok_or_else(|| {
                self.invariant_at_origin(
                    OriginId(u32::MAX),
                    "resolved view ID is outside its canonical arena",
                )
            })
    }

    pub(crate) fn app_name(&self) -> &str {
        &self.settings.app_name
    }

    pub(crate) fn struct_declarations(&self) -> &[crate::hir::StructDeclaration] {
        self.declarations.struct_declarations()
    }

    pub(crate) fn enum_declarations(&self) -> &[crate::hir::EnumDeclaration] {
        self.declarations.enum_declarations()
    }

    pub(crate) fn extern_functions(&self) -> impl Iterator<Item = &crate::hir::ExternDeclaration> {
        self.declarations.extern_declarations()
    }

    pub(crate) fn struct_rust_path_by_name(&self, name: &str) -> Option<&str> {
        self.declarations
            .struct_decl_by_name(name)
            .map(|declaration| declaration.rust_path.as_str())
    }

    #[cfg(test)]
    pub(crate) fn checked_facts(&self) -> &CheckedFacts {
        &self.facts
    }

    pub(crate) fn expressions(&self) -> &ResolvedExpressionProgram {
        &self.expressions
    }

    pub(crate) fn subscriptions(&self) -> &[ResolvedSubscription] {
        &self.subscriptions
    }

    pub(crate) fn tests(&self) -> &[ResolvedTest] {
        &self.tests
    }

    #[cfg(test)]
    pub(crate) fn canvas(&self, id: ViewId) -> Option<&ResolvedCanvas> {
        self.canvases.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn container(&self, id: ViewId) -> Option<&ResolvedContainer> {
        self.containers.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn layout(&self, id: ViewId) -> Option<&ResolvedLayout> {
        self.layouts.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn text(&self, id: ViewId) -> Option<&ResolvedText> {
        self.texts.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn input(&self, id: ViewId) -> Option<&ResolvedInput> {
        self.inputs.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn button(&self, id: ViewId) -> Option<&ResolvedButton> {
        self.buttons.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn text_editor(&self, id: ViewId) -> Option<&ResolvedTextEditor> {
        self.text_editors.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn markdown(&self, id: ViewId) -> Option<&ResolvedMarkdown> {
        self.markdowns.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn extern_component(&self, id: ViewId) -> Option<&ResolvedExternComponent> {
        self.extern_components.get(&id)
    }

    pub(crate) fn extern_components(&self) -> impl Iterator<Item = &ResolvedExternComponent> {
        self.extern_components.values()
    }

    pub(crate) fn extern_component_declarations(
        &self,
    ) -> Result<&[ResolvedExternComponentDeclaration], Error> {
        let expected = self
            .declarations
            .extern_declarations()
            .filter(|declaration| declaration.kind == ExternKind::Component)
            .collect::<Vec<_>>();
        if self.extern_component_declarations.len() != expected.len() {
            let origin = expected
                .first()
                .map_or(OriginId(u32::MAX), |item| item.declaration.origin);
            return Err(self.invariant_at_origin(
                origin,
                "extern component declaration HIR cardinality diverged",
            ));
        }
        for (resolved, declaration) in self.extern_component_declarations.iter().zip(expected) {
            let parameters_match = declaration.params.len() == declaration.borrowed.len()
                && resolved.parameters.len() == declaration.params.len()
                && resolved
                    .parameters
                    .iter()
                    .zip(&declaration.params)
                    .zip(&declaration.borrowed)
                    .all(|((resolved, (_, ty)), borrowed)| {
                        resolved.ty == *ty
                            && resolved.mode == extern_component_argument_mode(*borrowed, ty)
                    });
            if resolved.id != declaration.declaration.id
                || resolved.origin != declaration.declaration.origin
                || resolved.name != declaration.name
                || resolved.rust_path != declaration.rust_path
                || resolved.output != declaration.output
                || declaration.progress.is_some()
                || declaration.error.is_some()
                || !parameters_match
            {
                return Err(self.invariant_at_origin(
                    declaration.declaration.origin,
                    "extern component declaration HIR diverged",
                ));
            }
        }
        Ok(&self.extern_component_declarations)
    }

    #[cfg(test)]
    pub(crate) fn themer(&self, id: ViewId) -> Option<&ResolvedThemer> {
        self.themers.get(&id)
    }

    pub(crate) fn themers(&self) -> impl Iterator<Item = &ResolvedThemer> {
        self.themers.values()
    }

    #[cfg(test)]
    pub(crate) fn shader(&self, id: ViewId) -> Option<&ResolvedShader> {
        self.shaders.get(&id)
    }

    pub(crate) fn shaders(&self) -> impl Iterator<Item = &ResolvedShader> {
        self.shaders.values()
    }

    #[cfg(test)]
    pub(crate) fn boolean_control(&self, id: ViewId) -> Option<&ResolvedBooleanControl> {
        self.boolean_controls.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn pick_list(&self, id: ViewId) -> Option<&ResolvedPickList> {
        self.pick_lists.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn combo_box(&self, id: ViewId) -> Option<&ResolvedComboBox> {
        self.combo_boxes.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn slider(&self, id: ViewId) -> Option<&ResolvedSlider> {
        self.sliders.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn progress(&self, id: ViewId) -> Option<&ResolvedProgress> {
        self.progresses.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn rule(&self, id: ViewId) -> Option<&ResolvedRule> {
        self.rules.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn qr_code(&self, id: ViewId) -> Option<&ResolvedQrCode> {
        self.qr_codes.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn space(&self, id: ViewId) -> Option<&ResolvedSpace> {
        self.spaces.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn media(&self, id: ViewId) -> Option<&ResolvedMedia> {
        self.media.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn overlay(&self, id: ViewId) -> Option<&ResolvedOverlay> {
        self.overlays.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn tooltip(&self, id: ViewId) -> Option<&ResolvedTooltip> {
        self.tooltips.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn float(&self, id: ViewId) -> Option<&ResolvedFloat> {
        self.floats.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn pin(&self, id: ViewId) -> Option<&ResolvedPin> {
        self.pins.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn responsive(&self, id: ViewId) -> Option<&ResolvedResponsive> {
        self.responsives.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn keyed_column(&self, id: ViewId) -> Option<&ResolvedKeyedColumn> {
        self.keyed_columns.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn conditional(&self, id: ViewId) -> Option<&ResolvedConditional> {
        self.conditionals.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn iteration(&self, id: ViewId) -> Option<&ResolvedIteration> {
        self.iterations.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn match_view(&self, id: ViewId) -> Option<&ResolvedMatch> {
        self.match_views.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn lazy_view(&self, id: ViewId) -> Option<&ResolvedLazy> {
        self.lazy_views.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn table(&self, id: ViewId) -> Option<&ResolvedTable> {
        self.tables.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn pane_grid(&self, id: ViewId) -> Option<&ResolvedPaneGrid> {
        self.pane_grids.get(&id)
    }

    pub(crate) fn pane_grids(&self) -> Vec<&ResolvedPaneGrid> {
        let mut panes = self.pane_grids.values().collect::<Vec<_>>();
        panes.sort_by_key(|pane| pane.id.0);
        panes
    }

    #[cfg(test)]
    pub(crate) fn interaction_widget(&self, id: ViewId) -> Option<&ResolvedInteractionWidget> {
        self.interaction_widgets.get(&id)
    }

    pub(crate) fn test_mount(&self, id: TestId) -> Option<ViewId> {
        self.test_mounts.get(&id).copied()
    }

    pub(crate) fn preset_names(&self) -> &[String] {
        &self.preset_names
    }

    pub(crate) fn named_type_rust_path(&self, id: NamedTypeId) -> Option<&str> {
        self.named_type_rust_paths.get(&id).map(String::as_str)
    }

    #[cfg(test)]
    pub(crate) fn declarations(&self) -> &DeclarationIndex {
        &self.declarations
    }

    pub(crate) fn settings(&self) -> &ResolvedAppSettings {
        &self.settings
    }

    pub(crate) fn app_states(&self) -> &[AppStateContract] {
        &self.app_states
    }

    pub(crate) fn controlled_input_bindings(&self) -> Result<Vec<&str>, Error> {
        let mut seen = HashSet::with_capacity(self.controlled_inputs.len());
        let mut names = Vec::with_capacity(self.controlled_inputs.len());
        for binding in &self.controlled_inputs {
            let state = self
                .app_states
                .get(binding.state.0 as usize)
                .filter(|state| {
                    state.id == binding.state && state.name == binding.name && state.ty == Type::Str
                })
                .ok_or_else(|| {
                    self.invariant_at_origin(
                        self.settings.origin,
                        "controlled input binding does not match its normalized app state",
                    )
                })?;
            if !seen.insert(state.id) {
                return Err(self.invariant_at_origin(
                    self.settings.origin,
                    "controlled input binding is duplicated in normalized HIR",
                ));
            }
            names.push(state.name.as_str());
        }
        Ok(names)
    }

    pub(crate) fn controlled_editor_bindings(
        &self,
    ) -> Result<Vec<&ResolvedControlledEditorBinding>, Error> {
        let mut seen = HashSet::with_capacity(self.controlled_editors.len());
        let mut bindings = Vec::with_capacity(self.controlled_editors.len());
        for binding in &self.controlled_editors {
            let state = self.validate_controlled_editor_binding(binding)?;
            if !seen.insert(state.id) {
                return Err(self.invariant_at_origin(
                    self.settings.origin,
                    "controlled editor binding is duplicated in normalized HIR",
                ));
            }
            bindings.push(binding);
        }
        Ok(bindings)
    }

    pub(crate) fn controlled_editor_binding(
        &self,
        name: &str,
    ) -> Result<&ResolvedControlledEditorBinding, Error> {
        let binding = self
            .controlled_editors_by_name
            .get(name)
            .and_then(|index| self.controlled_editors.get(*index))
            .filter(|binding| binding.name == name)
            .ok_or_else(|| {
                self.invariant_at_origin(
                    self.settings.origin,
                    "normalized editor is absent from the controlled binding index",
                )
            })?;
        self.validate_controlled_editor_binding(binding)?;
        Ok(binding)
    }

    fn validate_controlled_editor_binding(
        &self,
        binding: &ResolvedControlledEditorBinding,
    ) -> Result<&AppStateContract, Error> {
        let state = self
            .app_states
            .get(binding.state.0 as usize)
            .filter(|state| {
                state.id == binding.state && state.name == binding.name && state.ty == Type::Editor
            })
            .ok_or_else(|| {
                self.invariant_at_origin(
                    self.settings.origin,
                    "controlled editor binding does not match its normalized app state",
                )
            })?;
        if let Some(action) = binding.action {
            self.declarations
                .try_extern_decl(action)
                .filter(|function| function.kind == ExternKind::EditorAction)
                .ok_or_else(|| {
                    self.invariant_at_origin(
                        self.settings.origin,
                        "controlled editor action does not match its normalized extern",
                    )
                })?;
        }
        Ok(state)
    }

    pub(crate) fn derived(&self) -> &[DerivedContract] {
        &self.derived
    }

    pub(crate) fn components(&self) -> &[ComponentContract] {
        &self.components
    }

    pub(crate) fn component_event_names<'a>(
        &'a self,
        component: &'a str,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.components
            .iter()
            .filter(move |contract| contract.name == component)
            .flat_map(|contract| contract.events.iter().map(|event| event.name.as_str()))
    }

    pub(crate) fn component(&self, id: ComponentId) -> &ComponentContract {
        &self.components[id.0 as usize]
    }

    pub(crate) fn try_component(&self, id: ComponentId) -> Option<&ComponentContract> {
        self.components.get(id.0 as usize)
    }

    pub(crate) fn component_event_matches(
        &self,
        id: ComponentEventId,
        name: &str,
        payloads: &[Type],
    ) -> bool {
        self.components
            .get(id.component.0 as usize)
            .and_then(|component| component.events.get(id.index as usize))
            .is_some_and(|event| event.id == id && event.name == name && event.payloads == payloads)
    }

    pub(crate) fn validate_component_call_event_contract(
        &self,
        call: &ComponentCall,
        index: usize,
    ) -> Result<(), Error> {
        let resolved = call.events.get(index).ok_or_else(|| {
            self.invariant_at_origin(
                call.origin,
                "lowered component call has an unmatched event route",
            )
        })?;
        let (route_index, event, name, payloads, origin) = match resolved {
            ResolvedEventRoute::Direct {
                route_index,
                event,
                name,
                payloads,
                origin,
                ..
            }
            | ResolvedEventRoute::Forward {
                route_index,
                event,
                name,
                payloads,
                origin,
                ..
            } => (*route_index, *event, name, payloads, *origin),
        };
        let origin_is_valid = self
            .origins
            .try_get(origin)
            .is_some_and(|resolved| resolved.parent == Some(call.origin));
        let forward_is_valid = match resolved {
            ResolvedEventRoute::Direct { .. } => true,
            ResolvedEventRoute::Forward {
                outer_component,
                outer_component_name,
                outer_event,
                outer_event_name,
                outer_payloads,
                ..
            } => self
                .try_component(*outer_component)
                .is_some_and(|component| {
                    component.id == *outer_component
                        && component.name == *outer_component_name
                        && outer_event.component == *outer_component
                        && outer_event_name == name
                        && outer_payloads == payloads
                        && self.component_event_matches(
                            *outer_event,
                            outer_event_name,
                            outer_payloads,
                        )
                }),
        };
        if route_index as usize != index
            || event.component != call.component
            || !origin_is_valid
            || !forward_is_valid
            || !self.component_event_matches(event, name, payloads)
        {
            let diagnostic_origin = if route_index as usize != index {
                call.events
                    .iter()
                    .find_map(|candidate| match candidate {
                        ResolvedEventRoute::Direct {
                            route_index,
                            origin,
                            ..
                        }
                        | ResolvedEventRoute::Forward {
                            route_index,
                            origin,
                            ..
                        } if *route_index as usize == index => Some(*origin),
                        ResolvedEventRoute::Direct { .. } | ResolvedEventRoute::Forward { .. } => {
                            None
                        }
                    })
                    .filter(|origin| {
                        self.origins
                            .try_get(*origin)
                            .is_some_and(|resolved| resolved.parent == Some(call.origin))
                    })
                    .unwrap_or(if origin_is_valid { origin } else { call.origin })
            } else if origin_is_valid {
                origin
            } else {
                call.origin
            };
            return Err(self.invariant_at_origin(
                diagnostic_origin,
                format!("lowered component event `{name}` has an invalid callee contract"),
            ));
        }
        Ok(())
    }
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

    pub(crate) fn component_call_by_id(
        &self,
        id: ComponentCallId,
    ) -> Result<&ComponentCall, Error> {
        self.calls
            .get(id.0 as usize)
            .filter(|call| call.id == id)
            .ok_or_else(|| {
                self.invariant_at_origin(
                    OriginId(u32::MAX),
                    "component call ID is outside its canonical arena",
                )
            })
    }

    #[cfg(test)]
    pub(crate) fn style_use(&self, span: &Span) -> Result<&ResolvedStyleUse, Error> {
        self.styles.style_use(span)
    }

    pub(crate) fn theme(&self) -> &ResolvedThemeProgram {
        &self.styles.theme
    }

    #[cfg(test)]
    pub(crate) fn nested_theme(&self, id: ViewId) -> Option<&ResolvedNestedTheme> {
        self.styles.nested_theme(id)
    }

    pub(crate) fn extern_function(&self, id: ExternFnId) -> &crate::hir::ExternDeclaration {
        self.declarations.extern_decl(id)
    }

    pub(crate) fn try_extern_function(
        &self,
        id: ExternFnId,
    ) -> Option<&crate::hir::ExternDeclaration> {
        self.declarations.try_extern_decl(id)
    }
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
    canvases: HashMap<ViewId, ResolvedCanvas>,
    containers: HashMap<ViewId, ResolvedContainer>,
    layouts: HashMap<ViewId, ResolvedLayout>,
    texts: HashMap<ViewId, ResolvedText>,
    buttons: HashMap<ViewId, ResolvedButton>,
    inputs: HashMap<ViewId, ResolvedInput>,
    text_editors: HashMap<ViewId, ResolvedTextEditor>,
    markdowns: HashMap<ViewId, ResolvedMarkdown>,
    extern_components: HashMap<ViewId, ResolvedExternComponent>,
    themers: HashMap<ViewId, ResolvedThemer>,
    shaders: HashMap<ViewId, ResolvedShader>,
    boolean_controls: HashMap<ViewId, ResolvedBooleanControl>,
    pick_lists: HashMap<ViewId, ResolvedPickList>,
    combo_boxes: HashMap<ViewId, ResolvedComboBox>,
    sliders: HashMap<ViewId, ResolvedSlider>,
    progresses: HashMap<ViewId, ResolvedProgress>,
    rules: HashMap<ViewId, ResolvedRule>,
    qr_codes: HashMap<ViewId, ResolvedQrCode>,
    spaces: HashMap<ViewId, ResolvedSpace>,
    controlled_inputs: Vec<AppStateId>,
    controlled_editors: Vec<CheckedControlledEditor>,
    media: HashMap<ViewId, ResolvedMedia>,
    overlays: HashMap<ViewId, ResolvedOverlay>,
    tooltips: HashMap<ViewId, ResolvedTooltip>,
    floats: HashMap<ViewId, ResolvedFloat>,
    pins: HashMap<ViewId, ResolvedPin>,
    responsives: HashMap<ViewId, ResolvedResponsive>,
    keyed_columns: HashMap<ViewId, ResolvedKeyedColumn>,
    conditionals: HashMap<ViewId, ResolvedConditional>,
    iterations: HashMap<ViewId, ResolvedIteration>,
    match_views: HashMap<ViewId, ResolvedMatch>,
    lazy_views: HashMap<ViewId, ResolvedLazy>,
    tables: HashMap<ViewId, ResolvedTable>,
    pane_grids: HashMap<ViewId, ResolvedPaneGrid>,
    interaction_widgets: HashMap<ViewId, ResolvedInteractionWidget>,
    views: Vec<ResolvedView>,
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

struct ViewWidgetExpressionPolicy<'a> {
    lowerer: &'a Lowerer,
    view: ViewId,
    scope: CheckedViewScope,
    use_id: CheckedExprUseId,
    span: &'a Span,
    canvas_locals: bool,
    own_view_locals: bool,
    allowed_own_view_locals: Option<&'a HashSet<CheckedLocalId>>,
    family: &'static str,
}

impl CheckedExpressionOwnerPolicy for ViewWidgetExpressionPolicy<'_> {
    fn use_id(&self) -> CheckedExprUseId {
        self.use_id
    }

    fn span(&self) -> &Span {
        self.span
    }

    fn value_type(&self, value: CheckedValueRef) -> Result<Type, Error> {
        let checked = self.lowerer.facts.try_value_by_ref(value).ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                format!("{} expression references an invalid value ID", self.family),
            )
        })?;
        let allowed = match (self.scope, value) {
            (CheckedViewScope::App | CheckedViewScope::Test(_), CheckedValueRef::AppState(_))
            | (CheckedViewScope::App | CheckedViewScope::Test(_), CheckedValueRef::Derived(_)) => {
                true
            }
            (CheckedViewScope::Component(component), CheckedValueRef::ComponentParam(id)) => {
                id.component == component
            }
            (CheckedViewScope::Component(component), CheckedValueRef::ComponentState(id)) => {
                id.component == component
            }
            _ => false,
        };
        if !allowed {
            return Err(self.lowerer.invariant(
                self.span,
                format!("{} expression value belongs to another scope", self.family),
            ));
        }
        Ok(checked.ty.clone())
    }

    fn local_type(&self, local: CheckedLocalId) -> Result<Type, Error> {
        let checked = self.lowerer.facts.try_local(local).ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                format!("{} expression references an invalid local ID", self.family),
            )
        })?;
        let canvas_local = self.canvas_locals
            && match checked.owner {
                CheckedLocalOwner::CanvasState(id) => id.canvas == self.view,
                CheckedLocalOwner::CanvasWidth(canvas)
                | CheckedLocalOwner::CanvasHeight(canvas) => canvas == self.view,
                CheckedLocalOwner::CanvasCommandItem(id) => id.canvas == self.view,
                CheckedLocalOwner::CanvasEventBinding { event, .. } => event.canvas == self.view,
                _ => false,
            };
        let allowed = match checked.owner {
            CheckedLocalOwner::ExpressionBinding { expression, .. } => expression == self.use_id,
            CheckedLocalOwner::View { view, role } => {
                if view == self.view
                    && role == CheckedViewLocalRole::DaemonWindow
                    && self.lowerer.document.daemon
                    && self.lowerer.facts.daemon_window_local() == Some(local)
                    && checked.name == "window"
                    && checked.ty == Type::WindowId
                {
                    return Ok(checked.ty.clone());
                }
                let mut current = Some(self.view);
                let mut found = false;
                while let Some(id) = current {
                    if id == view {
                        found = id != self.view
                            || (self.own_view_locals
                                && self
                                    .allowed_own_view_locals
                                    .is_none_or(|allowed| allowed.contains(&local)));
                        break;
                    }
                    current = self.lowerer.facts.view(id).parent;
                }
                found
            }
            _ => canvas_local,
        };
        if !allowed {
            return Err(self.lowerer.invariant(
                self.span,
                format!("{} expression local belongs to another scope", self.family),
            ));
        }
        Ok(checked.ty.clone())
    }

    fn slot_type(&self, slot: ComponentSlotId) -> Result<Type, Error> {
        self.lowerer
            .declarations
            .try_component_slot(slot)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!("{} expression references an invalid slot ID", self.family),
                )
            })?;
        if self.scope != CheckedViewScope::Component(slot.component) {
            return Err(self.lowerer.invariant(
                self.span,
                format!(
                    "{} expression slot belongs to another component",
                    self.family
                ),
            ));
        }
        Ok(Type::Bool)
    }

    fn palette_type(&self, palette: PaletteId) -> Result<Type, Error> {
        self.lowerer
            .declarations
            .palette_name(palette)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    format!(
                        "{} expression references an invalid palette ID",
                        self.family
                    ),
                )
            })?;
        let expression = self.lowerer.facts.expression_use(self.use_id);
        match &expression.source {
            Type::Palette(contract) => Ok(Type::Palette(contract.clone())),
            _ => Err(self.lowerer.invariant(
                self.span,
                format!(
                    "{} palette expression has no retained contract type",
                    self.family
                ),
            )),
        }
    }
}

fn checked_expression_coercion_is_valid(
    source: &Type,
    destination: &Type,
    coercion: &CheckedInitializerCoercion,
) -> bool {
    match coercion {
        CheckedInitializerCoercion::None => source == destination,
        CheckedInitializerCoercion::ListToCombo { element } => {
            source == &Type::List(Box::new(element.clone()))
                && destination == &Type::Combo(Box::new(element.clone()))
        }
        CheckedInitializerCoercion::ValueToAnimation { value } => {
            source == value && destination == &Type::Animation(Box::new(value.clone()))
        }
        CheckedInitializerCoercion::StrToMarkdown => {
            source == &Type::Str && destination == &Type::Markdown
        }
        CheckedInitializerCoercion::StrToEditor => {
            source == &Type::Str && destination == &Type::Editor
        }
    }
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
            CheckedLocalOwner::View { .. }
            | CheckedLocalOwner::CanvasState(_)
            | CheckedLocalOwner::CanvasWidth(_)
            | CheckedLocalOwner::CanvasHeight(_)
            | CheckedLocalOwner::CanvasCommandItem(_)
            | CheckedLocalOwner::CanvasEventBinding { .. } => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "app setting expression cannot reference a view local, including canvas locals",
                ));
            }
            CheckedLocalOwner::TestTarget(_) => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "app setting expression cannot reference a test target",
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

struct SubscriptionExpressionPolicy<'a> {
    lowerer: &'a Lowerer,
    use_id: CheckedExprUseId,
    span: &'a Span,
}

struct TestExpressionPolicy<'a> {
    lowerer: &'a Lowerer,
    use_id: CheckedExprUseId,
    test: TestId,
    span: &'a Span,
}

impl CheckedExpressionOwnerPolicy for TestExpressionPolicy<'_> {
    fn use_id(&self) -> CheckedExprUseId {
        self.use_id
    }

    fn span(&self) -> &Span {
        self.span
    }

    fn value_type(&self, value: CheckedValueRef) -> Result<Type, Error> {
        let checked = self.lowerer.facts.try_value_by_ref(value).ok_or_else(|| {
            self.lowerer
                .invariant(self.span, "test path references an invalid value ID")
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
                            "test path references an invalid component parameter ID",
                        )
                    })?;
                Err(self.lowerer.invariant(
                    self.span,
                    "test path cannot reference a component parameter",
                ))
            }
            CheckedValueRef::ComponentState(id) => {
                self.lowerer
                    .declarations
                    .try_component_state(id)
                    .ok_or_else(|| {
                        self.lowerer.invariant(
                            self.span,
                            "test path references an invalid component state ID",
                        )
                    })?;
                Err(self
                    .lowerer
                    .invariant(self.span, "test path cannot reference component state"))
            }
        }
    }

    fn local_type(&self, id: CheckedLocalId) -> Result<Type, Error> {
        let local = self.lowerer.facts.try_local(id).ok_or_else(|| {
            self.lowerer
                .invariant(self.span, "test expression references an invalid local ID")
        })?;
        match local.owner {
            CheckedLocalOwner::TestTarget(target) if target.test == self.test => {
                let declaration =
                    self.lowerer
                        .declarations
                        .test_target(target)
                        .ok_or_else(|| {
                            self.lowerer.invariant(
                                self.span,
                                "test expression references an invalid target ID",
                            )
                        })?;
                if local.ty != Type::TestTarget || local.origin != declaration.declaration.origin {
                    return Err(self.lowerer.invariant(
                        self.span,
                        "test target local has an inconsistent retained contract",
                    ));
                }
            }
            CheckedLocalOwner::TestTarget(_) => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "test expression references a target from another test",
                ));
            }
            CheckedLocalOwner::View {
                role: crate::check::CheckedViewLocalRole::DaemonWindow,
                ..
            } if self.lowerer.document.daemon
                && self.lowerer.facts.daemon_window_local() == Some(id)
                && local.name == "window"
                && local.ty == Type::WindowId => {}
            CheckedLocalOwner::ExpressionBinding { expression, .. }
                if expression == self.use_id => {}
            CheckedLocalOwner::ExpressionBinding { .. } => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "test expression binding belongs to another expression use",
                ));
            }
            _ => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "test expression references an incompatible local",
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
                self.lowerer
                    .invariant(self.span, "test expression references an invalid slot ID")
            })?;
        Err(self.lowerer.invariant(
            self.span,
            "test expression cannot reference a component slot",
        ))
    }

    fn palette_type(&self, id: PaletteId) -> Result<Type, Error> {
        self.lowerer.declarations.palette_name(id).ok_or_else(|| {
            self.lowerer
                .invariant(self.span, "test path references an invalid palette ID")
        })?;
        let expression = self
            .lowerer
            .facts
            .try_expression_use(self.use_id)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    "test palette path has an invalid expression-use ID",
                )
            })?;
        match &expression.source {
            Type::Palette(contract) => Ok(Type::Palette(contract.clone())),
            _ => Err(self.lowerer.invariant(
                self.span,
                "test palette path has no checked theme-contract type",
            )),
        }
    }
}

impl CheckedExpressionOwnerPolicy for SubscriptionExpressionPolicy<'_> {
    fn use_id(&self) -> CheckedExprUseId {
        self.use_id
    }

    fn span(&self) -> &Span {
        self.span
    }

    fn value_type(&self, value: CheckedValueRef) -> Result<Type, Error> {
        let checked = self.lowerer.facts.try_value_by_ref(value).ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                "subscription path references an invalid value ID",
            )
        })?;
        match value {
            CheckedValueRef::AppState(_) => Ok(checked.ty.clone()),
            CheckedValueRef::Derived(id) => {
                self.lowerer.declarations.try_derived(id).ok_or_else(|| {
                    self.lowerer.invariant(
                        self.span,
                        "subscription path references an invalid derived value ID",
                    )
                })?;
                Err(self.lowerer.invariant(
                    self.span,
                    "subscription path cannot reference a derived value",
                ))
            }
            CheckedValueRef::ComponentParam(id) => {
                self.lowerer
                    .declarations
                    .try_component_param(id)
                    .ok_or_else(|| {
                        self.lowerer.invariant(
                            self.span,
                            "subscription path references an invalid component parameter ID",
                        )
                    })?;
                Err(self.lowerer.invariant(
                    self.span,
                    "subscription path cannot reference a component parameter",
                ))
            }
            CheckedValueRef::ComponentState(id) => {
                self.lowerer
                    .declarations
                    .try_component_state(id)
                    .ok_or_else(|| {
                        self.lowerer.invariant(
                            self.span,
                            "subscription path references an invalid component state ID",
                        )
                    })?;
                Err(self.lowerer.invariant(
                    self.span,
                    "subscription path cannot reference component state",
                ))
            }
        }
    }

    fn local_type(&self, id: CheckedLocalId) -> Result<Type, Error> {
        let local = self.lowerer.facts.try_local(id).ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                "subscription expression references an invalid local ID",
            )
        })?;
        match local.owner {
            CheckedLocalOwner::ExpressionBinding { expression, .. }
                if expression == self.use_id => {}
            CheckedLocalOwner::ExpressionBinding { .. } => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "subscription expression binding belongs to another expression use",
                ));
            }
            _ => {
                return Err(self.lowerer.invariant(
                    self.span,
                    "subscription expression cannot reference a non-expression local",
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
                    "subscription expression references an invalid slot ID",
                )
            })?;
        Err(self.lowerer.invariant(
            self.span,
            "subscription expression cannot reference a component slot",
        ))
    }

    fn palette_type(&self, id: PaletteId) -> Result<Type, Error> {
        self.lowerer.declarations.palette_name(id).ok_or_else(|| {
            self.lowerer.invariant(
                self.span,
                "subscription path references an invalid palette ID",
            )
        })?;
        let expression = self
            .lowerer
            .facts
            .try_expression_use(self.use_id)
            .ok_or_else(|| {
                self.lowerer.invariant(
                    self.span,
                    "subscription path has an invalid expression-use ID",
                )
            })?;
        match &expression.source {
            Type::Palette(contract) => Ok(Type::Palette(contract.clone())),
            _ => Err(self.lowerer.invariant(
                self.span,
                "subscription palette path has no checked theme-contract type",
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
            controlled_inputs,
            controlled_editors,
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
            canvases: HashMap::new(),
            containers: HashMap::new(),
            layouts: HashMap::new(),
            texts: HashMap::new(),
            buttons: HashMap::new(),
            inputs: HashMap::new(),
            text_editors: HashMap::new(),
            markdowns: HashMap::new(),
            extern_components: HashMap::new(),
            themers: HashMap::new(),
            shaders: HashMap::new(),
            boolean_controls: HashMap::new(),
            pick_lists: HashMap::new(),
            combo_boxes: HashMap::new(),
            sliders: HashMap::new(),
            progresses: HashMap::new(),
            rules: HashMap::new(),
            qr_codes: HashMap::new(),
            spaces: HashMap::new(),
            controlled_inputs,
            controlled_editors,
            media: HashMap::new(),
            overlays: HashMap::new(),
            tooltips: HashMap::new(),
            floats: HashMap::new(),
            pins: HashMap::new(),
            responsives: HashMap::new(),
            keyed_columns: HashMap::new(),
            conditionals: HashMap::new(),
            iterations: HashMap::new(),
            match_views: HashMap::new(),
            lazy_views: HashMap::new(),
            tables: HashMap::new(),
            pane_grids: HashMap::new(),
            interaction_widgets: HashMap::new(),
            views: Vec::new(),
        }
    }

    fn lower(mut self) -> Result<LoweredProgram, Error> {
        self.validate_checked_view_topology()?;
        if let Err((origin, message)) = self.facts.validate_expression_arena() {
            return Err(self.invariant_at_origin(origin, message));
        }
        if let Err((origin, message)) =
            validate_expression_declaration_references(&self.facts, &self.declarations)
        {
            return Err(self.invariant_at_origin(origin, message));
        }
        let settings = self.lower_app_settings()?;
        let extern_component_declarations = self.lower_extern_component_declarations()?;
        self.validate_test_expression_contracts()?;
        self.lower_style_program()?;
        let subscriptions = self.lower_subscriptions()?;
        let named_type_rust_paths = self.declarations.named_type_rust_paths();
        let app_states = self.lower_app_states()?;
        let derived = self.lower_derived()?;
        self.index_components()?;
        self.lower_handlers()?;
        let tests = self.lower_tests()?;
        let component_roots = self
            .document
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| (ComponentId(index as u32), component.root.clone()))
            .collect::<Vec<_>>();
        for (component, root) in component_roots {
            self.lower_view(&root, Some(component))?;
        }
        let app_view = self.document.view.clone();
        self.lower_view(&app_view, None)?;
        let app_view_id = self.declarations.view_id(app_view.span()).ok_or_else(|| {
            self.invariant(app_view.span(), "application root has no shared view ID")
        })?;
        let mounts = self
            .document
            .tests
            .iter()
            .filter_map(|test| test.mount.clone())
            .collect::<Vec<_>>();
        for mount in mounts {
            self.lower_view(&mount, None)?;
        }
        let controlled_inputs = self
            .controlled_inputs
            .iter()
            .map(|id| {
                app_states
                    .get(id.0 as usize)
                    .filter(|state| state.id == *id && state.ty == Type::Str)
                    .map(|state| ResolvedControlledInputBinding {
                        state: *id,
                        name: state.name.clone(),
                    })
                    .ok_or_else(|| {
                        self.invariant(
                            self.document.view.span(),
                            "controlled input binding is outside the app-state arena",
                        )
                    })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let controlled_editors = self
            .controlled_editors
            .iter()
            .map(|binding| {
                let state = app_states
                    .get(binding.state.0 as usize)
                    .filter(|state| state.id == binding.state && state.ty == Type::Editor)
                    .ok_or_else(|| {
                        self.invariant(
                            self.document.view.span(),
                            "controlled editor binding is outside the app-state arena",
                        )
                    })?;
                if let Some(action) = binding.action {
                    self.declarations
                        .try_extern_decl(action)
                        .filter(|function| function.kind == ExternKind::EditorAction)
                        .ok_or_else(|| {
                            self.invariant(
                                self.document.view.span(),
                                "controlled editor action extern is invalid",
                            )
                        })?;
                }
                Ok(ResolvedControlledEditorBinding {
                    state: binding.state,
                    name: state.name.clone(),
                    action: binding.action,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut controlled_editors_by_name = HashMap::with_capacity(controlled_editors.len());
        for (index, binding) in controlled_editors.iter().enumerate() {
            if controlled_editors_by_name
                .insert(binding.name.clone(), index)
                .is_some()
            {
                return Err(self.invariant(
                    self.document.view.span(),
                    "controlled editor binding is duplicated in normalized HIR",
                ));
            }
        }
        let styles = self.styles.finish().ok_or_else(|| {
            Error::new(
                "E196",
                &Span::line(1),
                "style lowering completed without a normalized theme program",
            )
        })?;
        let test_mounts = self
            .document
            .tests
            .iter()
            .enumerate()
            .filter_map(|(index, test)| {
                test.mount.as_ref().and_then(|mount| {
                    self.declarations
                        .view_id(mount.span())
                        .map(|view| (TestId(index as u32), view))
                })
            })
            .collect();
        let preset_names = self
            .document
            .presets
            .iter()
            .map(|preset| preset.name.clone())
            .collect();
        let expressions = ResolvedExpressionProgram::from_checked(&self.facts, &self.declarations);
        Ok(LoweredProgram {
            #[cfg(test)]
            document: self.document,
            app_view: app_view_id,
            expressions,
            #[cfg(test)]
            facts: self.facts,
            declarations: self.declarations,
            settings,
            subscriptions,
            tests,
            canvases: self.canvases,
            containers: self.containers,
            layouts: self.layouts,
            texts: self.texts,
            buttons: self.buttons,
            inputs: self.inputs,
            text_editors: self.text_editors,
            markdowns: self.markdowns,
            extern_component_declarations,
            extern_components: self.extern_components,
            themers: self.themers,
            shaders: self.shaders,
            boolean_controls: self.boolean_controls,
            pick_lists: self.pick_lists,
            combo_boxes: self.combo_boxes,
            sliders: self.sliders,
            progresses: self.progresses,
            rules: self.rules,
            qr_codes: self.qr_codes,
            spaces: self.spaces,
            controlled_inputs,
            controlled_editors,
            controlled_editors_by_name,
            media: self.media,
            overlays: self.overlays,
            tooltips: self.tooltips,
            floats: self.floats,
            pins: self.pins,
            responsives: self.responsives,
            keyed_columns: self.keyed_columns,
            conditionals: self.conditionals,
            iterations: self.iterations,
            match_views: self.match_views,
            lazy_views: self.lazy_views,
            tables: self.tables,
            pane_grids: self.pane_grids,
            interaction_widgets: self.interaction_widgets,
            views: self.views,
            test_mounts,
            preset_names,
            named_type_rust_paths,
            app_states,
            derived,
            components: self.components,
            handlers: self.handlers,
            app_handlers: self.app_handlers,
            preset_handlers: self.preset_handlers,
            calls: self.calls,
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
                    #[cfg(test)]
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
        let tray = source
            .tray
            .as_ref()
            .map(|tray| -> Result<ResolvedTraySettings, Error> {
                let origin = self.push_origin(&tray.span, Some(declaration.origin));
                let text = |id, setting: &AppExpression| -> Result<ResolvedTrayText, Error> {
                    if let Some(literal) = crate::ast::tray_text_literal(setting) {
                        return Ok(ResolvedTrayText::Literal(literal.to_owned()));
                    }
                    Ok(ResolvedTrayText::Expression(
                        self.checked_app_setting_expression(id, &setting.span)?,
                    ))
                };
                let label = tray
                    .label
                    .as_ref()
                    .map(|setting| text(AppSettingExprId::TrayLabel, setting))
                    .transpose()?;
                let tooltip = tray
                    .tooltip
                    .as_ref()
                    .map(|setting| text(AppSettingExprId::TrayTooltip, setting))
                    .transpose()?;
                let menu = tray
                    .menu
                    .iter()
                    .enumerate()
                    .map(|(index, row)| match row {
                        TrayRow::Separator { .. } => Ok(ResolvedTrayRow::Separator),
                        TrayRow::Item {
                            text: source_text,
                            route,
                            span,
                        } => Ok(ResolvedTrayRow::Item {
                            text: text(AppSettingExprId::TrayMenuRow(index as u32), source_text)?,
                            route: route
                                .as_ref()
                                .map(|route| -> Result<String, Error> {
                                    // The name reaches codegen as a message
                                    // variant, so a route that lost its
                                    // handler would fail in generated Rust
                                    // rather than here.
                                    if !self.declarations.handlers().iter().any(|handler| {
                                        handler.name == *route
                                            && handler.owner == HandlerOwner::App
                                            && handler.payloads.is_empty()
                                    }) {
                                        return Err(self.invariant(
                                            span,
                                            "checked tray menu row references an unknown handler",
                                        ));
                                    }
                                    Ok(route.clone())
                                })
                                .transpose()?,
                        }),
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                let icons = tray
                    .icons
                    .iter()
                    .enumerate()
                    .map(|(index, icon)| -> Result<ResolvedTrayIcon, Error> {
                        Ok(ResolvedTrayIcon {
                            icon: ResolvedWindowIcon {
                                path: icon.icon.path.clone(),
                                width: icon.icon.width,
                                height: icon.icon.height,
                                byte_len: icon.icon.byte_len,
                                origin: self.push_origin(&icon.icon.span, Some(origin)),
                            },
                            when: icon
                                .when
                                .as_ref()
                                .map(|guard| {
                                    self.checked_app_setting_expression(
                                        AppSettingExprId::TrayIconGuard(index as u32),
                                        &guard.span,
                                    )
                                })
                                .transpose()?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok(ResolvedTraySettings {
                    icons,
                    icon_template: tray.icon_template.unwrap_or(false),
                    label,
                    tooltip,
                    menu,
                    origin,
                })
            })
            .transpose()?;
        Ok(ResolvedAppSettings {
            #[cfg(test)]
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
            tray,
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
            && current.windows == settings.windows
            && tray_static_fields_match(current.tray.as_ref(), settings.tray.as_ref());
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
        if let Some(tray) = &settings.tray {
            if tray.icons.is_empty() {
                return Err(self.invariant(&tray.span, "checked tray settings lost their icon"));
            }
            for icon in &tray.icons {
                let icon = &icon.icon;
                let expected = (icon.width as usize)
                    .checked_mul(icon.height as usize)
                    .and_then(|pixels| pixels.checked_mul(4));
                if icon.width == 0 || icon.height == 0 || expected != Some(icon.byte_len) {
                    return Err(self.invariant(
                        &icon.span,
                        "checked tray icon dimensions do not match its byte length",
                    ));
                }
            }
            // The last icon carries no guard, so the selection codegen emits
            // always ends somewhere. A drift here would produce a fold with no
            // final arm.
            if tray.icons.last().is_some_and(|icon| icon.when.is_some())
                || tray.icons[..tray.icons.len() - 1]
                    .iter()
                    .any(|icon| icon.when.is_none())
            {
                return Err(self.invariant(
                    &tray.span,
                    "checked tray icon guards are not in first-match order",
                ));
            }
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
        if let Some(tray) = &source.tray {
            for (id, setting) in [
                (AppSettingExprId::TrayLabel, &tray.label),
                (AppSettingExprId::TrayTooltip, &tray.tooltip),
            ] {
                if setting.as_ref().is_some_and(tray_text_is_reactive) {
                    expected.insert(id);
                }
            }
            for (index, icon) in tray.icons.iter().enumerate() {
                if icon.when.is_some() {
                    expected.insert(AppSettingExprId::TrayIconGuard(index as u32));
                }
            }
            for (index, row) in tray.menu.iter().enumerate() {
                if let TrayRow::Item { text, .. } = row
                    && tray_text_is_reactive(text)
                {
                    expected.insert(AppSettingExprId::TrayMenuRow(index as u32));
                }
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
            AppSettingExprId::TrayLabel => {
                source.tray.as_ref().and_then(|tray| tray.label.as_ref())
            }
            AppSettingExprId::TrayTooltip => {
                source.tray.as_ref().and_then(|tray| tray.tooltip.as_ref())
            }
            AppSettingExprId::TrayIconGuard(index) => source
                .tray
                .as_ref()
                .and_then(|tray| tray.icons.get(index as usize))
                .and_then(|icon| icon.when.as_ref()),
            AppSettingExprId::TrayMenuRow(index) => source
                .tray
                .as_ref()
                .and_then(|tray| tray.menu.get(index as usize))
                .and_then(|row| match row {
                    TrayRow::Item { text, .. } => Some(text),
                    TrayRow::Separator { .. } => None,
                }),
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
            | AppSettingExprId::TextColor
            | AppSettingExprId::TrayLabel
            | AppSettingExprId::TrayTooltip
            | AppSettingExprId::TrayMenuRow(_) => Type::Str,
            AppSettingExprId::TrayIconGuard(_) => Type::Bool,
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
        if expression.owner != policy.use_id() {
            return Err(self.invariant(
                policy.span(),
                "checked expression node belongs to another retained expression use",
            ));
        }
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
            CheckedCallTarget::Extern(reference) => {
                let function =
                    self.declarations
                        .try_extern_decl(reference.id)
                        .ok_or_else(|| {
                            self.invariant(
                                policy.span(),
                                "checked call references an invalid extern ID",
                            )
                        })?;
                if arguments
                    .iter()
                    .any(|argument| matches!(argument, CheckedCallArgument::Binding(_)))
                    || function.name != reference.name
                    || !matches!(function.kind, ExternKind::Pure | ExternKind::Sync)
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
        // The canonical contract is recomputed without the surrounding context, so type
        // parameters the call site never constrains (`ok`'s error side, `err`'s output side)
        // stay unknown here. Instantiating those slots from the checked output keeps the
        // concrete parts of the contract authoritative while accepting the context the
        // checker resolved them with.
        if unify_type_evidence(&canonical, output) != *output {
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

    fn validate_test_expression_contracts(&self) -> Result<(), Error> {
        if self.document.tests.len() != self.declarations.test_count() {
            return Err(self.invariant_at(
                &Span::line(1),
                "checked test topology changed before HIR lowering",
            ));
        }
        let mut expected_expressions = 0usize;
        let mut expected_targets = 0usize;
        for (test_index, test) in self.document.tests.iter().enumerate() {
            let declaration = self.declarations.test(test_index);
            if declaration.declaration.id != TestId(test_index as u32)
                || declaration.name != test.name
                || declaration.semantic_key != crate::ast::test_declaration_semantic_key(test)
                || declaration.targets.len() != test.targets.len()
                || declaration.steps.len() != test.steps.len()
            {
                return Err(self.invariant(
                    &test.span,
                    "checked test declaration topology changed before HIR lowering",
                ));
            }
            for (target_index, target) in test.targets.iter().enumerate() {
                expected_targets += 1;
                let id = TestTargetId {
                    test: declaration.declaration.id,
                    index: target_index as u32,
                };
                let target_declaration = self.declarations.test_target(id).ok_or_else(|| {
                    self.invariant(&target.span, "test target has no stable declaration ID")
                })?;
                let target_segments = target
                    .target
                    .segments
                    .iter()
                    .map(|segment| (segment.name.clone(), segment.key.is_some()))
                    .collect::<Vec<_>>();
                let local = self
                    .facts
                    .local_by_owner(CheckedLocalOwner::TestTarget(id))
                    .and_then(|id| self.facts.try_local(id))
                    .ok_or_else(|| {
                        self.invariant(&target.span, "test target has no checked local")
                    })?;
                if target_declaration.name != target.name
                    || target_declaration.segments != target_segments
                    || local.name != target_declaration.name
                    || local.ty != Type::TestTarget
                    || local.origin != target_declaration.declaration.origin
                {
                    return Err(self.invariant(
                        &target.span,
                        "test target local has a mismatched retained contract",
                    ));
                }
                for (segment_index, segment) in target.target.segments.iter().enumerate() {
                    let Some(_) = &segment.key else {
                        continue;
                    };
                    expected_expressions += 1;
                    self.validate_test_expression_use(
                        CheckedExprOwner::TestTargetKey {
                            target: id,
                            segment: segment_index as u32,
                        },
                        declaration.declaration.id,
                        &target.span,
                    )?;
                }
            }
            for (step_index, step) in test.steps.iter().enumerate() {
                let id = TestStepId {
                    test: declaration.declaration.id,
                    index: step_index as u32,
                };
                let step_declaration = self.declarations.test_step(id).ok_or_else(|| {
                    self.invariant(&step.span, "test step has no stable declaration ID")
                })?;
                if step_declaration.semantic_key != crate::ast::test_step_semantic_key(step) {
                    return Err(self.invariant(
                        &step.span,
                        "checked test step semantic contract changed before HIR lowering",
                    ));
                }
                for (operand, _) in crate::ast::test_step_expression_roots(step)
                    .into_iter()
                    .enumerate()
                {
                    expected_expressions += 1;
                    self.validate_test_expression_use(
                        CheckedExprOwner::TestStepOperand {
                            step: id,
                            operand: operand as u32,
                        },
                        declaration.declaration.id,
                        &step.span,
                    )?;
                }
            }
        }
        let actual_expressions = self
            .facts
            .expression_uses()
            .iter()
            .filter(|expression| {
                matches!(
                    expression.owner,
                    CheckedExprOwner::TestTargetKey { .. }
                        | CheckedExprOwner::TestStepOperand { .. }
                )
            })
            .count();
        let actual_targets = self
            .facts
            .locals()
            .iter()
            .filter(|local| matches!(local.owner, CheckedLocalOwner::TestTarget(_)))
            .count();
        if actual_expressions != expected_expressions || actual_targets != expected_targets {
            return Err(self.invariant_at(
                &Span::line(1),
                "checked test HIR contains extra or missing arena entries",
            ));
        }
        Ok(())
    }

    fn validate_test_expression_use(
        &self,
        owner: CheckedExprOwner,
        test: TestId,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let id = self.facts.expression_use_by_owner(owner).ok_or_else(|| {
            self.invariant(span, "test expression has no checked expression-use ID")
        })?;
        let expression = self.facts.try_expression_use(id).ok_or_else(|| {
            self.invariant(
                span,
                "test expression references an invalid expression-use ID",
            )
        })?;
        if expression.owner != owner {
            return Err(self.invariant(span, "test expression has a mismatched retained owner"));
        }
        let policy = TestExpressionPolicy {
            lowerer: self,
            use_id: id,
            test,
            span,
        };
        self.validate_checked_expression_use_graph(
            expression,
            &expression.source,
            &policy,
            &mut CheckedExpressionGraph::default(),
        )?;
        let root = self.facts.try_expression(expression.root).ok_or_else(|| {
            self.invariant(span, "test expression references an invalid checked root")
        })?;
        if root.ty != expression.source {
            return Err(self.invariant(
                span,
                "test expression root type diverged from its retained use",
            ));
        }
        Ok(id)
    }

    fn lower_subscriptions(&self) -> Result<Vec<ResolvedSubscription>, Error> {
        if self.facts.subscriptions().len() != self.declarations.subscription_count() {
            return Err(self.invariant_at(
                &Span::line(1),
                "checked subscription topology changed before HIR lowering",
            ));
        }
        self.facts
            .subscriptions()
            .iter()
            .enumerate()
            .map(|(index, subscription)| {
                self.lower_subscription(index, subscription)
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
    ) -> Result<ResolvedSubscription, Error> {
        let span = &subscription.span;
        let declaration = self.declarations.try_subscription(index).ok_or_else(|| {
            self.invariant_at(span, "checked subscription has no declaration identity")
        })?;
        if subscription.id != declaration.id || subscription.origin != declaration.origin {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription topology changed before HIR lowering",
            ));
        }
        let ValidatedSubscriptionContract {
            source_payloads,
            delivered_payloads,
            filter,
            route_args,
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
        let source_payloads = source_payloads
            .iter()
            .map(|payload| self.resolve_type(payload, span))
            .collect::<Result<Vec<_>, _>>()?;
        let delivered_payloads = delivered_payloads
            .iter()
            .map(|payload| self.resolve_type(payload, span))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ResolvedSubscription {
            source,
            source_payloads,
            delivered_payloads,
            filter,
            context: subscription.context,
            condition: subscription.condition,
            window_id: subscription.window_id,
            status: subscription.status,
            route: ResolvedSubscriptionRoute {
                #[cfg(test)]
                handler: subscription.route.handler,
                handler_name: subscription.route.handler_name.clone(),
                args: route_args,
            },
            span: subscription.span.clone(),
            origin: subscription.origin,
        })
    }

    fn validate_subscription_expression_use(
        &self,
        id: CheckedExprUseId,
        subscription: SubscriptionId,
        role: CheckedSubscriptionExprRole,
        expected: Option<&Type>,
        span: &Span,
    ) -> Result<Type, Error> {
        let owner = CheckedExprOwner::Subscription { subscription, role };
        let expression = self.facts.try_expression_use(id).ok_or_else(|| {
            self.invariant(span, "subscription references an invalid expression-use ID")
        })?;
        if expression.owner != owner || self.facts.expression_use_by_owner(owner) != Some(id) {
            return Err(self.invariant(
                span,
                "subscription expression has a mismatched owner or role",
            ));
        }
        let expected = expected.unwrap_or(&expression.source);
        let policy = SubscriptionExpressionPolicy {
            lowerer: self,
            use_id: id,
            span,
        };
        self.validate_checked_expression_use_graph(
            expression,
            expected,
            &policy,
            &mut CheckedExpressionGraph::default(),
        )?;
        Ok(expression.destination.clone())
    }

    fn validate_subscription_contract(
        &self,
        subscription: &CheckedSubscription,
    ) -> Result<ValidatedSubscriptionContract, Error> {
        let span = &subscription.span;
        if matches!(
            subscription.source,
            CheckedSubscriptionSource::Every { milliseconds: 0 }
                | CheckedSubscriptionSource::Repeat {
                    milliseconds: 0,
                    ..
                }
        ) {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription duration is not positive",
            ));
        }
        if subscription.window_id && !resolved_subscription_supports_window_id(&subscription.source)
        {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription source does not support a window ID",
            ));
        }
        if subscription.status.is_some()
            && !resolved_subscription_supports_status(&subscription.source)
        {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription source does not support status filtering",
            ));
        }
        if let Some(condition) = subscription.condition {
            self.validate_subscription_expression_use(
                condition,
                subscription.id,
                CheckedSubscriptionExprRole::Condition,
                Some(&Type::Bool),
                span,
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
                let argument_types = self.validate_subscription_arguments(
                    subscription.id,
                    arguments,
                    function,
                    span,
                )?;
                if argument_types.iter().any(|ty| !lazy_hashable(ty)) {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked subscription run data is not hashable",
                    ));
                }
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
                let identity = self.validate_subscription_expression_use(
                    *identity,
                    subscription.id,
                    CheckedSubscriptionExprRole::EventIdentity,
                    None,
                    span,
                )?;
                if !lazy_hashable(&identity) {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked raw event identity is not hashable",
                    ));
                }
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
                    self.checked_subscription_extern(reference, ExternKind::Pure, span)?;
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
                    self.resolve_subscription_extern(reference, ExternKind::Pure, span)?,
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
            let context = self.validate_subscription_expression_use(
                context,
                subscription.id,
                CheckedSubscriptionExprRole::Context,
                None,
                span,
            )?;
            if !lazy_hashable(&context) {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked subscription context is not hashable",
                ));
            }
            delivered_payloads.insert(0, context);
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
        if subscription
            .route
            .payloads
            .iter()
            .copied()
            .ne(0..subscription.route.payloads.len() as u32)
        {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription route has a mismatched payload order",
            ));
        }
        if handler.owner != HandlerOwner::App
            || handler.name != subscription.route.handler_name
            || handler.payloads.len() != subscription.route.payloads.len()
        {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription route has a mismatched handler contract",
            ));
        }
        let route_args = subscription
            .route
            .payloads
            .iter()
            .zip(&handler.payloads)
            .map(|(index, target)| {
                lower_typed_payload_argument(span, &delivered_payloads, *index, target)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ValidatedSubscriptionContract {
            source_payloads,
            delivered_payloads,
            filter: filter.map(|(contract, _)| contract),
            route_args,
        })
    }

    fn validate_subscription_arguments(
        &self,
        subscription: SubscriptionId,
        arguments: &[CheckedExprUseId],
        function: &crate::hir::ExternDeclaration,
        span: &Span,
    ) -> Result<Vec<Type>, Error> {
        if arguments.len() != function.params.len() {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern arguments have a mismatched arity",
            ));
        }
        arguments
            .iter()
            .zip(&function.params)
            .enumerate()
            .map(|(index, (argument, (_, expected)))| {
                self.validate_subscription_expression_use(
                    *argument,
                    subscription,
                    CheckedSubscriptionExprRole::SourceArgument(index as u32),
                    Some(expected),
                    span,
                )
            })
            .collect()
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
            #[cfg(test)]
            id: declaration.declaration.id,
            name: declaration.name.clone(),
            rust_path: declaration.rust_path.clone(),
            params: declaration
                .params
                .iter()
                .map(|(_, ty)| self.resolve_type(ty, span))
                .collect::<Result<Vec<_>, _>>()?,
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
                return Err(Error::new("E196", span, "checked type remained unknown"));
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
                    #[cfg(test)]
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
                    #[cfg(test)]
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
            from: options.from,
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
                    #[cfg(test)]
                    default: self
                        .facts
                        .value_by_ref(CheckedValueRef::ComponentParam(declaration.id))
                        .initializer,
                    #[cfg(test)]
                    origin: declaration.origin,
                });
            }
            let events: Vec<ComponentEventContract> = component
                .events
                .iter()
                .enumerate()
                .map(|(index, event)| {
                    let event_id = ComponentEventId {
                        component: id,
                        index: index as u32,
                    };
                    ComponentEventContract {
                        id: event_id,
                        name: event.name.clone(),
                        payloads: event.payloads.clone(),
                    }
                })
                .collect();
            let checked_slots = self.facts.component_slots(id).ok_or_else(|| {
                self.invariant(&component.span, "component has no checked slot partition")
            })?;
            let declared_slot_count =
                self.declarations.component_slot_count(id).ok_or_else(|| {
                    self.invariant(
                        &component.span,
                        "component slot declaration arena is missing",
                    )
                })?;
            if checked_slots.len() != declared_slot_count {
                return Err(self.invariant_at_origin(
                    origin,
                    "checked component slot cardinality diverged from declarations",
                ));
            }
            let mut slots = Vec::with_capacity(checked_slots.len());
            for (index, checked) in checked_slots.iter().enumerate() {
                let expected_id = ComponentSlotId {
                    component: id,
                    index: index as u32,
                };
                let declaration = self
                    .declarations
                    .try_component_slot(expected_id)
                    .ok_or_else(|| {
                        self.invariant_at_origin(origin, "component slot declaration is missing")
                    })?;
                let authoritative_origin = declaration.origin;
                let expected_view = self
                    .declarations
                    .component_slot_view(expected_id)
                    .ok_or_else(|| {
                        self.invariant_at_origin(
                            authoritative_origin,
                            "component slot has no stable view declaration",
                        )
                    })?;
                let retained_origin =
                    self.origins.try_get(authoritative_origin).ok_or_else(|| {
                        self.invariant_at_origin(origin, "component slot origin is invalid")
                    })?;
                let checked_view = self.facts.try_view(expected_view).ok_or_else(|| {
                    self.invariant_at_origin(
                        authoritative_origin,
                        "checked component slot has no checked view",
                    )
                })?;
                if checked.id != expected_id
                    || declaration.id != checked.id
                    || checked.origin != authoritative_origin
                    || checked.view != expected_view
                    || retained_origin.parent != Some(origin)
                    || checked_view.scope != CheckedViewScope::Component(id)
                    || checked_view.kind != "slot"
                    || checked_view.origin != self.declarations.view(expected_view).origin
                    || self
                        .facts
                        .component_slot_for_view(expected_view)
                        .is_none_or(|slot| slot.id != expected_id || slot.view != expected_view)
                {
                    return Err(self.invariant_at_origin(
                        authoritative_origin,
                        "checked component slot association is inconsistent",
                    ));
                }
                slots.push(ComponentSlotContract {
                    id: checked.id,
                    name: checked.name.clone(),
                    optional: checked.optional,
                    origin: checked.origin,
                });
            }
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
                        #[cfg(test)]
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
            let mut slots_by_name = HashMap::with_capacity(slots.len());
            for (index, slot) in slots.iter().enumerate() {
                if slots_by_name.insert(slot.name.clone(), index).is_some() {
                    return Err(self.invariant_at_origin(
                        slot.origin,
                        "checked component slot name is duplicated",
                    ));
                }
            }
            let root = self
                .declarations
                .view_id(component.root.span())
                .ok_or_else(|| {
                    self.invariant(
                        component.root.span(),
                        "component root has no shared view ID",
                    )
                })?;
            self.components.push(ComponentContract {
                id,
                name: component.name,
                params,
                output: component.output,
                events,
                slots,
                states,
                handlers: Vec::new(),
                root,
                storage,
                #[cfg(test)]
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
            run_lanes: &mut [bool],
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
                        && run.lane.is_none_or(|lane| mark(run_lanes, lane.0))
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
                    .all(|child| visit(child, statements, tasks, routes, run_lanes)),
                ResolvedStatementKind::Abortable { task, .. } => {
                    visit(task, statements, tasks, routes, run_lanes)
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
        let mut run_lanes = vec![false; self.declarations.run_lane_count()];
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
                    &mut run_lanes,
                )
            });
        if !valid
            || statements.iter().any(|seen| !seen)
            || tasks.iter().any(|seen| !seen)
            || routes.iter().any(|seen| !seen)
            || run_lanes.iter().any(|seen| !seen)
        {
            return Err(self.invariant(
                &Span::line(1),
                "handler lowering did not consume every statement, task, route, and request-lane ID",
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
            #[cfg(test)]
            id,
            owner: declaration.owner,
            name: handler.name.clone(),
            params,
            statements,
            #[cfg(test)]
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
                lane: lane_name,
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
                let lane = declaration.run_lane;
                if (*mode == FutureMode::Every) != lane.is_none()
                    || lane_name.is_some() != lane.is_some()
                {
                    return Err(self.invariant(span, "run mode and request-lane ID diverged"));
                }
                if let Some(lane) = lane {
                    let run_lane = self.declarations.try_run_lane(lane).ok_or_else(|| {
                        self.invariant(span, "request-lane ID is outside its arena")
                    })?;
                    let lane_owner = match owner {
                        HandlerOwner::Preset(_) => HandlerOwner::App,
                        owner => owner,
                    };
                    if run_lane.declaration.id != lane
                        || run_lane.owner != lane_owner
                        || run_lane.name != *lane_name.as_ref().expect("lane shape checked above")
                        || run_lane.mode != *mode
                        || !run_lane.statements.contains(&id)
                    {
                        return Err(self.invariant(
                            span,
                            "request-lane HIR owner, name, mode, or membership diverged",
                        ));
                    }
                }
                ResolvedStatementKind::Run(ResolvedRun {
                    kind: *kind,
                    mode: *mode,
                    lane,
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
            #[cfg(test)]
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
        self.lower_view_topology(node, outer_component)?;
        self.lower_view_style(node)?;
        match node {
            ViewNode::Media {
                kind,
                source,
                options,
                span,
                ..
            } => {
                self.lower_media(*kind, source, options, span, outer_component)?;
            }
            ViewNode::Tooltip {
                content,
                tip,
                options,
                span,
                ..
            } => {
                self.lower_tooltip(options, span, outer_component)?;
                self.lower_view(content, outer_component)?;
                self.lower_view(tip, outer_component)?;
            }
            ViewNode::MouseArea {
                options,
                content,
                span,
                ..
            } => {
                self.lower_mouse_area(options, span, outer_component)?;
                self.lower_view(content, outer_component)?;
            }
            ViewNode::ResizeHandle {
                options,
                content,
                span,
                ..
            } => {
                self.lower_resize_handle(options, span, outer_component)?;
                self.lower_view(content, outer_component)?;
            }
            ViewNode::Sensor {
                options,
                content,
                span,
                ..
            } => {
                self.lower_sensor(options, span, outer_component)?;
                self.lower_view(content, outer_component)?;
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
                self.lower_float(scale, x, y, style, span, outer_component)?;
                self.lower_view(content, outer_component)?;
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
                self.lower_pin(width, height, x, y, span, outer_component)?;
                self.lower_view(content, outer_component)?;
            }
            ViewNode::Responsive {
                content,
                width,
                height,
                span,
                ..
            } => {
                self.lower_responsive(content, width, height, span, outer_component)?;
                match content {
                    ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                        self.lower_view(narrow, outer_component)?;
                        self.lower_view(wide, outer_component)?;
                    }
                    ResponsiveContent::Size { content, .. } => {
                        self.lower_view(content, outer_component)?;
                    }
                }
            }
            ViewNode::Lazy {
                dependency,
                binding,
                child,
                span,
                ..
            } => {
                self.lower_lazy(dependency, binding, span, outer_component)?;
                self.lower_view(child, outer_component)?;
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
                self.lower_keyed_column(item, items, key, options, span, outer_component)?;
                self.lower_view(child, outer_component)?;
            }
            ViewNode::Canvas {
                options,
                locals,
                commands,
                events,
                span,
                ..
            } => {
                self.lower_canvas(options, locals, commands, events, span, outer_component)?;
            }
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
            ViewNode::Slot {
                name,
                optional,
                span,
            } => {
                self.lower_component_slot(name, *optional, span, outer_component)?;
            }
            ViewNode::If {
                condition,
                children,
                span,
            } => {
                self.lower_conditional(condition, span, outer_component)?;
                for child in children {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                self.lower_iteration(item, items, span, outer_component)?;
                for child in children {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Text { span, .. } | ViewNode::RichText { span, .. } => {
                self.lower_text(node, span, outer_component)?;
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
                self.lower_input(
                    label,
                    binding,
                    hint,
                    disabled,
                    options,
                    span,
                    outer_component,
                )?;
            }
            ViewNode::TextEditor {
                binding,
                disabled,
                options,
                span,
                ..
            } => {
                self.lower_text_editor(binding, disabled, options, span, outer_component)?;
            }
            ViewNode::Markdown {
                content,
                options,
                route,
                span,
                ..
            } => {
                self.lower_markdown(content, options, route, span, outer_component)?;
            }
            ViewNode::ExternComponent {
                function,
                args,
                route,
                span,
                ..
            } => {
                self.lower_extern_component(function, args, route, span, outer_component)?;
            }
            ViewNode::Themer {
                function,
                args,
                route,
                span,
                ..
            } => {
                self.lower_themer(function, args, route, span, outer_component)?;
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
                self.lower_shader(function, args, width, height, route, span, outer_component)?;
            }
            ViewNode::Checkbox {
                id,
                label,
                checked,
                disabled,
                options,
                style,
                route,
                span,
                ..
            } => {
                self.lower_checkbox(
                    CheckboxSource {
                        id,
                        label,
                        checked,
                        disabled,
                        options,
                        style,
                        route,
                        span,
                    },
                    outer_component,
                )?;
            }
            ViewNode::Toggler {
                id,
                label,
                checked,
                disabled,
                options,
                style,
                route,
                span,
                ..
            } => {
                self.lower_toggler(
                    TogglerSource {
                        id,
                        label,
                        checked,
                        disabled,
                        options,
                        style,
                        route,
                        span,
                    },
                    outer_component,
                )?;
            }
            ViewNode::Radio {
                id,
                label,
                value,
                selected,
                options,
                style,
                route,
                span,
                ..
            } => {
                self.lower_radio(
                    RadioSource {
                        id,
                        label,
                        value,
                        selected,
                        options,
                        style,
                        route,
                        span,
                    },
                    outer_component,
                )?;
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
                self.lower_button(
                    label,
                    content,
                    disabled,
                    options,
                    route,
                    span,
                    outer_component,
                )?;
                if let Some(content) = content {
                    self.lower_view(content, outer_component)?;
                }
            }
            ViewNode::PickList {
                options,
                selected,
                options_config,
                route,
                span,
                ..
            } => {
                self.lower_pick_list(
                    options,
                    selected,
                    options_config,
                    route,
                    span,
                    outer_component,
                )?;
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
                self.lower_combo_box(
                    state,
                    selected,
                    placeholder,
                    options,
                    route,
                    span,
                    outer_component,
                )?;
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
                self.lower_slider(
                    value,
                    min,
                    max,
                    step,
                    options,
                    *vertical,
                    styles,
                    route,
                    release,
                    span,
                    outer_component,
                )?;
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
                self.lower_progress(
                    value,
                    min,
                    max,
                    options,
                    *vertical,
                    styles,
                    span,
                    outer_component,
                )?;
            }
            ViewNode::Rule {
                axis,
                thickness,
                options,
                styles,
                span,
                ..
            } => {
                self.lower_rule(*axis, thickness, options, styles, span, outer_component)?;
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
                self.lower_qr_code(
                    payload,
                    *correction,
                    *version,
                    cell_size,
                    total_size,
                    cell,
                    background,
                    span,
                    outer_component,
                )?;
            }
            ViewNode::Space {
                width,
                height,
                styles,
                span,
                ..
            } => {
                self.lower_space(width, height, styles, span, outer_component)?;
            }
            ViewNode::Layout {
                kind,
                options,
                children,
                span,
                ..
            } => {
                self.lower_layout(*kind, options, span, outer_component)?;
                for child in children {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Match { value, arms, span } => {
                self.lower_match_view(value, arms, span, outer_component)?;
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Theme {
                preset,
                text,
                background,
                content,
                span,
                ..
            } => {
                self.lower_nested_theme(preset, text, background, span, outer_component)?;
                self.lower_view(content, outer_component)?;
            }
            ViewNode::Container {
                options,
                content,
                span,
                ..
            } => {
                self.lower_container(options, span, outer_component)?;
                self.lower_view(content, outer_component)?;
            }
            ViewNode::Overlay {
                options,
                content,
                layer,
                span,
                ..
            } => {
                self.lower_overlay(options, span, outer_component)?;
                self.lower_view(content, outer_component)?;
                self.lower_view(layer, outer_component)?;
            }
            ViewNode::PaneGrid {
                panes,
                templates,
                span,
                ..
            } => {
                self.lower_pane_grid(span, outer_component)?;
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    self.lower_view(child, outer_component)?;
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
                self.lower_table(item, rows, options, columns, span, outer_component)?;
                for column in columns {
                    self.lower_view(&column.header, outer_component)?;
                    self.lower_view(&column.cell, outer_component)?;
                }
            }
        }
        Ok(())
    }

    fn lower_component_slot(
        &self,
        name: &str,
        optional: bool,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let component = outer_component
            .ok_or_else(|| self.invariant(span, "component slot is outside a component"))?;
        let view = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "component slot has no shared view ID"))?;
        let checked = self.facts.component_slot_for_view(view).ok_or_else(|| {
            self.invariant(span, "component slot view has no checked slot association")
        })?;
        let declaration = self
            .declarations
            .try_component_slot(checked.id)
            .ok_or_else(|| {
                self.invariant_at_origin(checked.origin, "component slot declaration is missing")
            })?;
        if checked.id.component != component
            || checked.view != view
            || checked.name != name
            || checked.optional != optional
            || declaration.origin != checked.origin
        {
            return Err(self.invariant_at_origin(
                checked.origin,
                "component slot source diverged from its checked contract",
            ));
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
        let semantic_key = crate::ast::component_call_route_semantic_key(
            name,
            route.is_some(),
            events
                .iter()
                .zip(&supplied_events)
                .map(|(event, supplied)| {
                    (
                        event.name.as_str(),
                        supplied
                            .as_ref()
                            .is_some_and(|supplied| supplied.route.is_some()),
                    )
                }),
        );
        let (route_view, interaction, route_scope, route_origin) = self.interaction_contract(
            CheckedInteractionKind::ComponentCallRoutes,
            semantic_key,
            span,
            outer_component,
        )?;
        let checked_routes = self
            .facts
            .component_call_routes(call_id)
            .cloned()
            .ok_or_else(|| self.invariant(span, "component call has no checked route contract"))?;
        if route_view != view_id
            || route_origin != origin
            || checked_routes.call != call_id
            || checked_routes.view != view_id
            || checked_routes.component != component_id
            || !interaction.option_expressions.is_empty()
            || checked_routes.events.len() != events.len()
        {
            return Err(self.invariant(span, "component call route identity diverged"));
        }
        let route_expression_count = interaction
            .routes
            .iter()
            .flat_map(|route| &route.args)
            .filter(|argument| matches!(argument, CheckedCanvasRouteArg::Expression(_)))
            .count();
        if interaction.expression_count as usize != route_expression_count {
            return Err(self.invariant_at_origin(
                origin,
                "component call route expression cardinality diverged",
            ));
        }
        self.validate_interaction_expression_graphs(
            view_id,
            route_scope,
            interaction.expression_count,
            span,
        )?;
        let route_sources = route
            .iter()
            .chain(
                supplied_events
                    .iter()
                    .filter_map(|event| event.as_ref().and_then(|event| event.route.as_ref())),
            )
            .collect::<Vec<_>>();
        let mut route_index = 0usize;
        let output = match (&output_ty, route, &checked_routes.output) {
            (Type::Unit, None, None) => ComponentOutputRoute::None,
            (output, Some(source), Some(checked)) if checked.output == *output => {
                self.validate_component_call_route_origin(
                    checked.origin,
                    &source.span,
                    origin,
                    "component output route",
                )?;
                let resolved = self.lower_required_interaction_route(
                    source,
                    &interaction,
                    &route_sources,
                    &mut route_index,
                    view_id,
                    route_scope,
                )?;
                if resolved.id != checked.route || resolved.origin != checked.origin {
                    return Err(self.invariant(span, "component output route ID diverged"));
                }
                self.validate_component_call_route_hir(
                    &resolved,
                    std::slice::from_ref(output),
                    false,
                    span,
                )?;
                ComponentOutputRoute::Direct {
                    output: output.clone(),
                    route: resolved,
                }
            }
            _ => return Err(self.invariant(span, "component output route contract diverged")),
        };
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
            arguments.push(ResolvedArgument {
                param: param.id,
                name: param.name.clone(),
                ty: param.ty.clone(),
                expression,
                scope,
                writable,
            });
        }

        let mut resolved_events = Vec::with_capacity(events.len());
        for (event_route_index, ((event, supplied), checked)) in events
            .iter()
            .zip(supplied_events)
            .zip(&checked_routes.events)
            .enumerate()
        {
            let supplied = supplied.ok_or_else(|| {
                self.invariant(span, format!("event `{}` has no checked route", event.name))
            })?;
            self.validate_component_call_route_origin(
                checked.origin,
                &supplied.span,
                origin,
                "component event route",
            )?;
            if checked.event != event.id
                || checked.name != event.name
                || checked.payloads != event.payloads
                || supplied.name != event.name
            {
                return Err(
                    self.invariant(&supplied.span, "component event route contract diverged")
                );
            }
            match (&supplied.route, &checked.delivery) {
                (
                    Some(source),
                    CheckedComponentEventDelivery::Direct {
                        route: checked_route,
                        origin: checked_route_origin,
                    },
                ) => {
                    self.validate_component_call_route_origin(
                        *checked_route_origin,
                        &source.span,
                        origin,
                        "component event direct route",
                    )?;
                    let resolved = self.lower_required_interaction_route(
                        source,
                        &interaction,
                        &route_sources,
                        &mut route_index,
                        view_id,
                        route_scope,
                    )?;
                    if resolved.id != *checked_route || resolved.origin != *checked_route_origin {
                        return Err(self
                            .invariant(&supplied.span, "component event route identity diverged"));
                    }
                    self.validate_component_call_route_hir(
                        &resolved,
                        &event.payloads,
                        true,
                        &supplied.span,
                    )?;
                    resolved_events.push(ResolvedEventRoute::Direct {
                        route_index: event_route_index as u32,
                        event: event.id,
                        name: event.name.clone(),
                        payloads: event.payloads.clone(),
                        route: resolved,
                        origin: checked.origin,
                    });
                }
                (
                    None,
                    CheckedComponentEventDelivery::Forward {
                        outer_component: checked_outer,
                        outer_component_name,
                        outer_event,
                        outer_event_name,
                        outer_payloads,
                    },
                ) => {
                    let outer = outer_component.ok_or_else(|| {
                        self.invariant_at_origin(
                            checked.origin,
                            "forwarded event has no outer component",
                        )
                    })?;
                    let outer_contract = self
                        .components
                        .get(checked_outer.0 as usize)
                        .filter(|component| {
                            component.id == *checked_outer
                                && component.name == *outer_component_name
                        })
                        .ok_or_else(|| {
                            self.invariant_at_origin(
                                checked.origin,
                                "forwarded event has an invalid outer component",
                            )
                        })?;
                    let declaration =
                        self.declarations
                            .component_event(*outer_event)
                            .ok_or_else(|| {
                                self.invariant_at_origin(
                                    checked.origin,
                                    "forwarded event has an invalid outer declaration",
                                )
                            })?;
                    if *checked_outer != outer
                        || outer_event.component != outer
                        || outer_contract.name != *outer_component_name
                        || declaration.name != *outer_event_name
                        || declaration.payloads != *outer_payloads
                        || *outer_event_name != event.name
                        || *outer_payloads != event.payloads
                    {
                        return Err(self.invariant_at_origin(
                            checked.origin,
                            "forwarded event declaration contract diverged",
                        ));
                    }
                    resolved_events.push(ResolvedEventRoute::Forward {
                        route_index: event_route_index as u32,
                        event: event.id,
                        name: event.name.clone(),
                        payloads: event.payloads.clone(),
                        outer_component: outer,
                        outer_component_name: outer_component_name.clone(),
                        outer_event: *outer_event,
                        outer_event_name: outer_event_name.clone(),
                        outer_payloads: outer_payloads.clone(),
                        origin: checked.origin,
                    });
                }
                _ => {
                    return Err(self.invariant(
                        &supplied.span,
                        "component event direct/forward topology diverged",
                    ));
                }
            }
        }
        if route_index != interaction.routes.len() || route_index != route_sources.len() {
            return Err(self.invariant(span, "component call left checked routes unconsumed"));
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
                #[cfg(test)]
                optional: declared.optional,
                content: supplied
                    .map(|slot| {
                        self.declarations
                            .view_id(slot.content.span())
                            .ok_or_else(|| {
                                self.invariant(
                                    &slot.span,
                                    "component slot content has no shared view ID",
                                )
                            })
                    })
                    .transpose()?,
            });
        }

        let scope = id.as_ref().map_or_else(
            || ComponentScope::Implicit {
                call_site: span.line,
            },
            |_| ComponentScope::Explicit,
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

    fn validate_component_call_route_hir(
        &self,
        route: &ResolvedInteractionRoute,
        source_payloads: &[Type],
        ordered_payloads: bool,
        span: &Span,
    ) -> Result<(), Error> {
        if route.source_payloads != source_payloads || route.ordered_payloads != ordered_payloads {
            return Err(self.invariant(span, "component route source payload contract diverged"));
        }
        let destinations = match &route.target {
            ResolvedInteractionRouteTarget::TargetHandler(handler) => self
                .declarations
                .try_handler(*handler)
                .ok_or_else(|| self.invariant(span, "component route handler is invalid"))?
                .payloads
                .clone(),
            ResolvedInteractionRouteTarget::OutputCallback { output, .. } => vec![output.clone()],
            ResolvedInteractionRouteTarget::NamedEvent { payloads, .. } => payloads.clone(),
        };
        if route.args.len() != destinations.len() {
            return Err(self.invariant(span, "component route target cardinality diverged"));
        }
        for (argument, destination) in route.args.iter().zip(&destinations) {
            match argument {
                ResolvedInteractionRouteArg::Expression(expression) => {
                    let retained = self.facts.try_expression_use(*expression).ok_or_else(|| {
                        self.invariant(span, "component route expression is invalid")
                    })?;
                    if retained.source != *destination || retained.destination != *destination {
                        return Err(
                            self.invariant(span, "component route expression type diverged")
                        );
                    }
                }
                ResolvedInteractionRouteArg::Payload { index, ty } => {
                    let source = route.source_payloads.get(*index as usize).ok_or_else(|| {
                        self.invariant(span, "component route payload index is invalid")
                    })?;
                    if ty != source || ty != destination {
                        return Err(self.invariant(span, "component route payload type diverged"));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_component_call_route_origin(
        &self,
        origin: OriginId,
        source: &Span,
        parent: OriginId,
        label: &str,
    ) -> Result<(), Error> {
        let retained = self.origins.try_get(origin).ok_or_else(|| {
            self.component_call_invariant_at_span(source, format!("{label} origin is invalid"))
        })?;
        let (expected_path, expected_line) = self
            .origins
            .source_origin(source.line)
            .map_or((None, source.line), |(path, line)| (Some(path), line));
        if retained.path.as_deref() != expected_path
            || retained.line != expected_line
            || retained.column != source.column
            || retained.parent != Some(parent)
        {
            return Err(self.component_call_invariant_at_span(
                source,
                format!("{label} physical origin diverged"),
            ));
        }
        Ok(())
    }

    fn component_call_invariant_at_span(&self, source: &Span, message: impl Into<String>) -> Error {
        let message = format!("lowering invariant failed: {}", message.into());
        let Some((path, line)) = self.origins.source_origin(source.line) else {
            return Error::new("E196", source, message);
        };
        Error::new(
            "E196",
            &Span {
                line,
                column: source.column,
            },
            message,
        )
        .at_path(path.display().to_string())
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

fn valid_f32(value: f64) -> bool {
    value.is_finite() && value.abs() <= f32::MAX as f64
}

fn valid_positive_f32(value: f64) -> bool {
    value > 0.0 && valid_f32(value)
}

/// The narrowest span for a tray whose static fields drifted: the setting
/// line that changed, falling back to whichever tray block still exists.
fn changed_tray_span<'a>(
    current: Option<&'a TraySettings>,
    expected: Option<&'a TraySettings>,
) -> Option<&'a Span> {
    let (Some(current), Some(expected)) = (current, expected) else {
        return current.or(expected).map(|tray| &tray.span);
    };
    for (name, changed) in [
        (
            "icon-rgba",
            tray_icon_shape(current) != tray_icon_shape(expected),
        ),
        (
            "icon-template",
            current.icon_template != expected.icon_template,
        ),
        (
            "menu",
            tray_menu_shape(current) != tray_menu_shape(expected),
        ),
    ] {
        if changed {
            return current
                .setting_spans
                .get(name)
                .or_else(|| expected.setting_spans.get(name))
                .or(Some(&current.span));
        }
    }
    Some(&current.span)
}

/// The icon facts generated code bakes in: which files are embedded, at what
/// size, and which lines carry a guard. The guard expressions themselves are
/// compared through the checked-expression machinery like every other one.
fn tray_icon_shape(tray: &TraySettings) -> Vec<(&WindowIcon, bool)> {
    tray.icons
        .iter()
        .map(|icon| (&icon.icon, icon.when.is_some()))
        .collect()
}

/// The menu facts generated code bakes in: how many rows there are, which are
/// separators, and where each row's chosen event goes.
fn tray_menu_shape(tray: &TraySettings) -> Vec<(bool, Option<&String>)> {
    tray.menu
        .iter()
        .map(|row| match row {
            TrayRow::Item { route, .. } => (true, route.as_ref()),
            TrayRow::Separator { .. } => (false, None),
        })
        .collect()
}

fn tray_static_fields_match(
    current: Option<&TraySettings>,
    checked: Option<&TraySettings>,
) -> bool {
    match (current, checked) {
        (None, None) => true,
        (Some(current), Some(checked)) => {
            tray_icon_shape(current) == tray_icon_shape(checked)
                && current.icon_template == checked.icon_template
                && tray_menu_shape(current) == tray_menu_shape(checked)
        }
        _ => false,
    }
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
    if !tray_static_fields_match(current.tray.as_ref(), expected.tray.as_ref()) {
        return changed_tray_span(current.tray.as_ref(), expected.tray.as_ref());
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
    use std::time::{Duration, Instant};

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
                "run {:?} lane={:?} {} error={}",
                run.mode,
                run.lane,
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
    run replace lane=request fetch(local) -> done _
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
  s0 task=None final=false let request LocalId(0) = ExpressionId(2) @20:1
  s1 task=Some(TaskId(0)) final=true run Every lane=None r0 -> app h1 loaded (payload 0:i64) @21:1 error=none @21:1
h1 App loaded params=["next:i64:LocalId(1)"] @22:1
  s2 task=None final=true assign value:i64, value=ExpressionId(4), at=None, move=false @23:1
h2 Component(ComponentId(0)) start params=[] @27:1
  s3 task=Some(TaskId(1)) final=true run Replace lane=Some(RunLaneId(0)) r1 -> component c0 h3 done (payload 0:i64) @28:1 error=none @28:1
h3 Component(ComponentId(0)) done params=["next:i64:LocalId(2)"] @29:1
  s4 task=None final=true assign local:i64, value=ExpressionId(6), at=None, move=false @30:1
h4 Preset(0) preset seeded params=[] @16:1
  s5 task=None final=true assign value:i64, value=ExpressionId(7), at=None, move=false @18:1
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
            "t4 map value:i64/local=LocalId(0)",
            "t5 then value:i64/local=LocalId(1) -> t5 Task Extern(ExternFnId(2))",
            "t6 collect",
            "r2 -> app h3 collected (payload 0:[i64])",
            "r3 -> app h4 planned (payload 0:i64)",
            "s3 task=Some(TaskId(7)) final=true abortable",
            "s4 task=Some(TaskId(8)) final=true run Every lane=None r4 -> app h5 themed (payload 0:str)",
        ] {
            assert!(
                snapshot.contains(expected),
                "missing `{expected}` in handler HIR snapshot:\n{snapshot}"
            );
        }
    }

    #[test]
    fn handler_codegen_uses_checked_expressions_and_request_lanes_after_ast_mutation() {
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
    run latest lane=search fetch(query) -> loaded _
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
        assert!(generated.contains("__ice_run_lane_0_generation"));
        assert!(!generated.contains("unchecked-poison"));
        assert!(!generated.contains("unchecked-run-poison"));
        assert!(!generated.contains("// __ICE_SOURCE 900 1"));
        assert!(!generated.contains("// __ICE_SOURCE 999 1"));
    }

    #[test]
    fn rejects_mutated_handler_route_run_lane_and_statement_shapes_as_hir_invariants() {
        let source = format!(
            r#"app InvalidHir
extern crate::backend
  fetch(value:i64) -> i64
{THEME}component Search()
  state
    query = 1
  on search
    run latest lane=search fetch(query) -> loaded _
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
        let nonce = crate::test_support::unique_nonce();
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
        assert!(error.message.contains("component root view"));
    }

    #[test]
    fn malformed_run_lane_and_required_operation_routes_are_e196_not_panics() {
        let latest = format!(
            "app Search\nextern crate::backend\n  fetch(query:str) -> str\n{THEME}component SearchBox()\n  state\n    query = \"\"\n    result:str? = none\n  on search\n    run latest lane=search fetch(query) -> loaded _\n  on loaded(value)\n    result = some(value)\n  button \"Search\" -> search\nview\n  SearchBox #search\n"
        );
        let mut invalid_lane = lower(analyze(&latest).unwrap()).unwrap();
        let statement = invalid_lane
            .handlers
            .iter_mut()
            .find(|handler| handler.name == "search")
            .and_then(|handler| handler.statements.first_mut())
            .expect("fixture component run");
        let ResolvedStatementKind::Run(run) = &mut statement.kind else {
            panic!("fixture must contain a run");
        };
        run.lane = None;
        let error = crate::codegen::generate(&invalid_lane, "invalid.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("request-lane"));

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
                "app EditorContract\nextern crate::backend\n  pure apply_command(content:editor, command:str) -> editor\n{THEME}state\n  notes:editor = \"hello\"\non command\n  notes = apply_command(notes, \"bold\")\nview\n  editor <-> notes\n"
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
    fn malformed_checked_canvas_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidCanvasFacts\n{THEME}view\n  canvas w=120.0 h=80.0\n    circle x=10.0 y=20.0 r=4.0 fill=primary\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::Canvas(CanvasExpressionId {
                canvas: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains("invalid checked expression ID"),
            "{}",
            error.message
        );
        assert_eq!(error.line, 13);
    }

    #[test]
    fn malformed_checked_media_expression_id_does_not_panic() {
        let source =
            format!("app InvalidMediaFacts\n{THEME}view\n  image \"photo.png\" opacity=0.8\n");
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::Media(MediaExpressionId {
                media: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains("invalid checked expression ID"),
            "{}",
            error.message
        );
        assert_eq!(error.line, 13);
    }

    #[test]
    fn malformed_checked_tooltip_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidTooltipFacts\n{THEME}view\n  tooltip gap=2.0\n    text \"Hover\"\n    text \"Tip\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::Tooltip(TooltipExpressionId {
                tooltip: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains("invalid checked expression ID"),
            "{}",
            error.message
        );
        assert_eq!(error.line, 13);
    }

    #[test]
    fn malformed_checked_interaction_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidInteractionFacts\n{THEME}state\n  active = true\non pressed(flag)\nview\n  mouse press=pressed(active)\n    text \"Pointer\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains("invalid checked expression ID"),
            "{}",
            error.message
        );
    }

    #[test]
    fn malformed_checked_float_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidFloatFacts\n{THEME}view\n  float scale=1.0 x=original_x y=viewport_y\n    text \"Floating\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::Float(FloatExpressionId {
                float: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains("invalid checked expression ID"),
            "{}",
            error.message
        );
        assert_eq!(error.line, 13);

        let mut invalid_geometry = analyze(&source).unwrap();
        invalid_geometry
            .facts
            .corrupt_float_geometry_local(ViewId(0), 0, u32::MAX);
        let error = lower(invalid_geometry).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("geometry local ID"));
        assert_eq!(error.line, 13);
    }

    #[test]
    fn malformed_checked_pin_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidPinFacts\n{THEME}view\n  pin w=fill h=80.0 x=12.0 y=8.0\n    text \"Pinned\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::Pin(PinExpressionId {
                pin: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains("invalid checked expression ID"),
            "{}",
            error.message
        );
        assert_eq!(error.line, 13);
    }

    #[test]
    fn malformed_checked_responsive_expression_and_local_ids_do_not_panic() {
        let breakpoint_source = format!(
            "app InvalidResponsiveFacts\n{THEME}view\n  responsive at=600.0 w=fill h=40.0\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        let mut checked = analyze(&breakpoint_source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::ResponsiveBreakpoint,
            },
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));

        let size_source = format!(
            "app InvalidResponsiveLocal\n{THEME}view\n  responsive size=(available_width, available_height)\n    text available_width\n"
        );
        let mut checked = analyze(&size_source).unwrap();
        checked
            .facts
            .corrupt_responsive_size_local(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("local ID is outside its arena"));
    }

    #[test]
    fn malformed_checked_lazy_expression_and_local_ids_do_not_panic() {
        let source = format!(
            "app InvalidLazyFacts\n{THEME}state\n  title = \"Hello\"\nview\n  lazy title as cached\n    text cached\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::LazyDependency,
            },
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_lazy_binding_local(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("local ID is outside its arena"));
    }

    #[test]
    fn malformed_checked_responsive_fixed_dimension_cannot_become_static() {
        let source = format!(
            "app InvalidResponsiveTopology\n{THEME}view\n  responsive at=600.0 w=40.0 h=50.0\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        for replacement in [CheckedResponsiveLength::Fill, CheckedResponsiveLength::None] {
            let mut checked = analyze(&source).unwrap();
            checked
                .facts
                .corrupt_responsive_dimension(ViewId(0), 0, replacement);

            let error = lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains("topology diverged"));
        }
    }

    #[test]
    fn malformed_checked_responsive_fill_portion_value_drift_is_rejected() {
        let source = format!(
            "app InvalidResponsivePortion\n{THEME}view\n  responsive at=600.0 w=fill(2) h=fill\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_responsive_dimension(
            ViewId(0),
            0,
            CheckedResponsiveLength::FillPortion(3),
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn malformed_checked_responsive_expression_cannot_be_left_unconsumed() {
        let source = format!(
            "app InvalidResponsiveCardinality\n{THEME}view\n  responsive at=600.0 w=40.0 h=50.0\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_responsive_expression_count(ViewId(0), 2);

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression cardinality diverged"));
    }

    #[test]
    fn malformed_checked_responsive_cross_role_expression_is_rejected() {
        let source = format!(
            "app InvalidResponsiveOwner\n{THEME}view\n  responsive at=600.0 w=40.0 h=50.0\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_responsive_dimension_expression_role(
            ViewId(0),
            0,
            CheckedViewExprRole::ResponsiveHeightDimension,
        );

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("owner cardinality diverged"));
    }

    #[test]
    fn media_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedMedia\n{THEME}state\n  path = \"photo.png\"\n  alpha = 0.8\nview\n  image path opacity=alpha filter=nearest\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-media.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::Media {
            source: source_expr,
            options,
            ..
        } = &mut changed_expressions.document.view
        else {
            panic!("fixture root must be media");
        };
        *source_expr = Expr::Str("poisoned.png".into());
        options.opacity = Some(Expr::F64(0.1));
        let actual =
            crate::codegen::generate(&lower(changed_expressions).unwrap(), "checked-media.ice")
                .unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("poisoned.png"));

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Media { kind, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be media");
        };
        *kind = MediaKind::Viewer;
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn media_codegen_ignores_raw_options_and_theme_token_order_after_lowering() {
        let source = format!(
            "app LoweredMedia\n{THEME}state\n  path = \"icon.svg\"\n  alpha = 0.8\nview\n  svg path color=fg hover=primary opacity=alpha\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-media.ice").unwrap();

        let ViewNode::Media {
            kind,
            source,
            options,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be media");
        };
        *kind = MediaKind::Viewer;
        *source = Expr::Str("poisoned.png".into());
        options.opacity = Some(Expr::F64(0.1));
        options.svg_color = Some("danger".into());
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(1, 3);

        let actual = crate::codegen::generate(&program, "lowered-media.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("poisoned.png"));
    }

    #[test]
    fn tooltip_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedTooltip\n{THEME}state\n  gap = 2.0\nview\n  tooltip position=cursor gap=gap bg=primary\n    text \"Hover\"\n    text \"Tip\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-tooltip.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::Tooltip { options, .. } = &mut changed_expressions.document.view else {
            panic!("fixture root must be tooltip");
        };
        options.gap = Expr::F64(99.0);
        let actual =
            crate::codegen::generate(&lower(changed_expressions).unwrap(), "checked-tooltip.ice")
                .unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("bounded_table_metric(99.0"));

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Tooltip { options, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be tooltip");
        };
        options.position = TooltipPosition::Top;
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn tooltip_codegen_ignores_raw_options_and_theme_token_order_after_lowering() {
        let source = format!(
            "app LoweredTooltip\n{THEME}state\n  gap = 2.0\nview\n  tooltip position=cursor gap=gap bg=primary text=fg\n    text \"Hover\"\n    text \"Tip\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-tooltip.ice").unwrap();

        let ViewNode::Tooltip { options, .. } = &mut program.document.view else {
            panic!("fixture root must be tooltip");
        };
        options.position = TooltipPosition::Top;
        options.gap = Expr::F64(99.0);
        options.background = Some(BackgroundValue::Color("danger".into()));
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(1, 3);

        let actual = crate::codegen::generate(&program, "lowered-tooltip.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("bounded_table_metric(99.0"));
    }

    #[test]
    fn normalizes_tooltip_options_colors_styles_and_expression_owners() {
        let source = format!(
            "app TooltipHir\nextern crate::backend\n  box-style dynamic_tooltip(active:bool)\n{THEME}state\n  active = true\nview\n  tooltip position=cursor gap=2.0 p=5.0 delay=100 snap=false style=dynamic_tooltip(active) bg=linear(1.57, bg@0.0, primary/25@1.0) text=fg border=primary/75 border-w=1.0 r=5.0 r-tl=2.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 px-snap=true\n    text \"Hover\"\n    text \"Tip\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let tooltip = program.tooltip(ViewId(0)).unwrap();

        assert_eq!(tooltip.id, ViewId(0));
        assert_eq!(tooltip.position, ResolvedTooltipPosition::FollowCursor);
        let Some(ResolvedTooltipBaseStyle::Custom(style)) = &tooltip.base_style else {
            panic!("fixture must normalize a custom tooltip style");
        };
        assert_eq!(
            program.extern_function(style.function).name,
            "dynamic_tooltip"
        );
        assert_eq!(style.arguments.len(), 1);
        let Some(ResolvedTooltipBackground::Linear { stops, .. }) = &tooltip.background else {
            panic!("fixture must normalize a linear background");
        };
        assert_eq!(stops.len(), 2);
        assert!(matches!(
            tooltip.text_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 1, .. }),
                opacity: None,
            })
        ));
        assert!(matches!(
            tooltip.border_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                opacity: Some(75),
            })
        ));
        assert_eq!(
            program
                .facts
                .expression_use_by_owner(CheckedExprOwner::Tooltip(TooltipExpressionId {
                    tooltip: ViewId(0),
                    index: 0,
                })),
            Some(tooltip.gap)
        );
    }

    #[test]
    fn interaction_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedInteraction\n{THEME}state\n  active = true\non pressed(flag)\non moved(x, y)\nview\n  mouse press=pressed(active) move=moved cursor=pointer\n    text \"Pointer\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-interaction.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::MouseArea { options, .. } = &mut changed_expressions.document.view else {
            panic!("fixture root must be a mouse area");
        };
        let RouteArg::Expr(argument) = &mut options.press.as_mut().unwrap().args[0] else {
            panic!("fixture press route must have an expression");
        };
        *argument = Expr::Bool(false);
        let actual = crate::codegen::generate(
            &lower(changed_expressions).unwrap(),
            "checked-interaction.ice",
        )
        .unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::MouseArea { options, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be a mouse area");
        };
        options.interaction = Some(MouseInteraction::Crosshair);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn interaction_codegen_ignores_raw_routes_and_options_after_lowering() {
        let source = format!(
            "app LoweredInteraction\n{THEME}state\n  active = true\non pressed(flag)\non moved(x, y)\non resized(dx, dy)\nview\n  col\n    mouse press=pressed(active) move=moved cursor=pointer\n      text \"Pointer\"\n    resize-handle drag=resized cursor=resize-horizontal\n      text \"Resize\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-interaction.ice").unwrap();

        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a column");
        };
        let ViewNode::MouseArea { options, .. } = &mut children[0] else {
            panic!("first child must be a mouse area");
        };
        options.interaction = Some(MouseInteraction::Crosshair);
        let press = options.press.as_mut().unwrap();
        press.handler = "poisoned".into();
        press.args.clear();
        let ViewNode::ResizeHandle { options, .. } = &mut children[1] else {
            panic!("second child must be a resize handle");
        };
        options.interaction = Some(MouseInteraction::Wait);
        options.drag.as_mut().unwrap().handler = "poisoned".into();

        let actual = crate::codegen::generate(&program, "lowered-interaction.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("Poisoned"));
    }

    #[test]
    fn normalizes_mouse_area_resize_handle_routes_payloads_and_interactions() {
        let source = format!(
            "app InteractionHir\n{THEME}state\n  active = true\non pressed(flag)\non moved(x, y)\non scrolled(x, y, pixels)\non resized(dx, dy)\nview\n  col\n    mouse press=pressed(active) move=moved scroll=scrolled cursor=pointer\n      text \"Pointer\"\n    resize-handle drag=resized cursor=resize-horizontal\n      text \"Resize\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();

        let Some(ResolvedInteractionWidget::MouseArea(mouse)) =
            program.interaction_widget(ViewId(1))
        else {
            panic!("first interaction must be a mouse area");
        };
        assert_eq!(mouse.interaction, Some(MouseInteraction::Pointer));
        let press = mouse.press.as_ref().unwrap();
        assert!(matches!(
            press.args.as_slice(),
            [ResolvedInteractionRouteArg::Expression(_)]
        ));
        let moved = mouse.move_route.as_ref().unwrap();
        assert!(moved.ordered_payloads);
        assert_eq!(moved.source_payloads, [Type::F64, Type::F64]);
        assert!(matches!(
            moved.args.as_slice(),
            [
                ResolvedInteractionRouteArg::Payload { index: 0, .. },
                ResolvedInteractionRouteArg::Payload { index: 1, .. }
            ]
        ));
        let scrolled = mouse.scroll.as_ref().unwrap();
        assert_eq!(scrolled.source_payloads, [Type::F64, Type::F64, Type::Bool]);

        let Some(ResolvedInteractionWidget::ResizeHandle(handle)) =
            program.interaction_widget(ViewId(3))
        else {
            panic!("second interaction must be a resize handle");
        };
        assert_eq!(
            handle.interaction,
            Some(MouseInteraction::ResizingHorizontally)
        );
        let drag = handle.drag.as_ref().unwrap();
        assert_eq!(
            drag.id,
            InteractionRouteId {
                widget: ViewId(3),
                index: 0,
            }
        );
        assert!(drag.ordered_payloads);
        assert_eq!(drag.source_payloads, [Type::F64, Type::F64]);
    }

    #[test]
    fn normalizes_sensor_options_routes_payloads_and_origins() {
        let source = format!(
            "app SensorHir\n{THEME}state\n  active = true\non shown(width, height)\non resized(width, height)\non hidden\nview\n  sensor show=shown resize=resized hide=hidden key=active anticipate=24.0 delay=50\n    text \"Observed\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let Some(ResolvedInteractionWidget::Sensor(sensor)) = program.interaction_widget(ViewId(0))
        else {
            panic!("interaction must be a sensor");
        };

        assert_eq!(sensor.id, ViewId(0));
        let show = sensor.show.as_ref().unwrap();
        assert_eq!(show.source_payloads, [Type::F64, Type::F64]);
        assert!(show.ordered_payloads);
        assert!(matches!(
            show.args.as_slice(),
            [
                ResolvedInteractionRouteArg::Payload { index: 0, .. },
                ResolvedInteractionRouteArg::Payload { index: 1, .. }
            ]
        ));
        assert_eq!(sensor.resize.as_ref().unwrap().source_payloads.len(), 2);
        assert!(sensor.hide.as_ref().unwrap().source_payloads.is_empty());
        for (expression, expected) in [
            (sensor.key.unwrap(), Type::Bool),
            (sensor.anticipate.unwrap(), Type::F64),
            (sensor.delay_ms.unwrap(), Type::I64),
        ] {
            let expression = program.checked_facts().expression_use(expression);
            assert_eq!(expression.source, expected);
            assert_eq!(expression.destination, expected);
        }
        let origin = program.origin(sensor.origin);
        assert_eq!(origin.line, 18);
    }

    #[test]
    fn normalizes_overlay_expressions_route_color_alignment_and_origin() {
        let source = format!(
            "app OverlayHir\n{THEME}state\n  shown = true\non close(flag)\nview\n  overlay when=shown dismiss=close(shown) backdrop=black/60 p=24.0 align-x=center align-y=end\n    content\n      text \"Page\"\n    layer\n      text \"Dialog\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let overlay = program.overlay(ViewId(0)).unwrap();

        assert_eq!(overlay.id, ViewId(0));
        assert_eq!(overlay.align_x, ResolvedOverlayAlignment::Center);
        assert_eq!(overlay.align_y, ResolvedOverlayAlignment::End);
        assert!(matches!(
            overlay.backdrop,
            ResolvedThemeColor {
                base: ResolvedThemeColorBase::Black,
                opacity: Some(60),
            }
        ));
        for (index, expression, expected) in [
            (0, overlay.visible, Type::Bool),
            (1, overlay.padding, Type::F64),
        ] {
            let expression = program.checked_facts().expression_use(expression);
            assert_eq!(expression.source, expected);
            assert_eq!(expression.destination, expected);
            assert_eq!(
                expression.owner,
                CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: ViewId(0),
                    index,
                })
            );
        }
        let dismiss = overlay.dismiss.as_ref().unwrap();
        assert_eq!(
            dismiss.id,
            InteractionRouteId {
                widget: ViewId(0),
                index: 0,
            }
        );
        assert!(dismiss.source_payloads.is_empty());
        assert!(!dismiss.ordered_payloads);
        assert!(matches!(
            dismiss.args.as_slice(),
            [ResolvedInteractionRouteArg::Expression(_)]
        ));
        let ResolvedInteractionRouteTarget::TargetHandler(handler) = dismiss.target else {
            panic!("overlay dismiss must target an app handler");
        };
        assert_eq!(program.try_handler(handler).unwrap().name, "close");
        assert_eq!(program.origin(dismiss.origin).parent, Some(overlay.origin));
    }

    #[test]
    fn overlay_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedOverlay\n{THEME}state\n  shown = true\non close(flag)\nview\n  overlay when=shown dismiss=close(shown) backdrop=black/60 p=24.0 align-x=center align-y=end\n    content\n      text \"Page\"\n    layer\n      text \"Dialog\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-overlay.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Overlay { options, .. } = &mut checked.document.view else {
            panic!("fixture root must be an overlay");
        };
        options.visible = Expr::Bool(false);
        options.padding = Expr::F64(999.0);
        let RouteArg::Expr(argument) = &mut options.dismiss.as_mut().unwrap().args[0] else {
            panic!("dismiss route must contain an expression");
        };
        *argument = Expr::Bool(false);
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-overlay.ice").unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Overlay { options, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be an overlay");
        };
        options.backdrop = "danger".into();
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn overlay_codegen_ignores_raw_options_and_route_after_lowering() {
        let source = format!(
            "app LoweredOverlay\n{THEME}state\n  shown = true\non close(flag)\nview\n  overlay when=shown dismiss=close(shown) backdrop=black/60 p=24.0 align-x=center align-y=end\n    content\n      text \"Page\"\n    layer\n      text \"Dialog\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-overlay.ice").unwrap();

        let ViewNode::Overlay { options, .. } = &mut program.document.view else {
            panic!("fixture root must be an overlay");
        };
        options.visible = Expr::Bool(false);
        options.padding = Expr::F64(999.0);
        options.backdrop = "danger".into();
        options.align_x = FlexAlignment::Start;
        options.align_y = FlexAlignment::Start;
        let dismiss = options.dismiss.as_mut().unwrap();
        dismiss.handler = "poisoned".into();
        dismiss.args.clear();

        let actual = crate::codegen::generate(&program, "lowered-overlay.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("Poisoned"));
        assert!(!actual.contains("999.0"));
    }

    #[test]
    fn malformed_checked_overlay_expression_route_id_and_origin_do_not_panic() {
        let source = format!(
            "app InvalidOverlayFacts\n{THEME}state\n  shown = true\non close\nview\n  overlay when=shown dismiss=close backdrop=black/60 p=24.0\n    content\n      text \"Page\"\n    layer\n      text \"Dialog\"\n"
        );

        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_interaction_route_id(ViewId(0), 0, u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_interaction_route_origin(ViewId(0), 0, u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("origin is outside its arena"));
    }

    #[test]
    fn imported_overlay_keeps_physical_origins_and_generated_source_markers() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-overlay-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("overlay.ice");
        fs::write(
            &root,
            format!(
                "app ImportedOverlayApp\nuse \"overlay.ice\"\n{THEME}view\n  ImportedOverlay\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedOverlay()\n  state\n    shown = true\n  on close\n    shown = false\n  overlay when=shown dismiss=close backdrop=black/60 p=12.0\n    content\n      text \"Page\"\n    layer\n      text \"Dialog\"\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let overlay = program.overlays.values().next().unwrap();
        let origin = program.origin(overlay.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 6);
        let route_origin = program.origin(overlay.dismiss.as_ref().unwrap().origin);
        assert_eq!(route_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(route_origin.parent, Some(overlay.origin));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 6 1 {encoded_import}")));

        program.overlays.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 6);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_complete_container_surface_layout_flex_and_expression_owners() {
        let source = format!(
            "app ContainerHir\nextern crate::backend\n  box-style dynamic_container(active:bool)\n{THEME}state\n  active = true\nview\n  flex\n    box style=dynamic_container(active) w=fill h=80.0 max-w=640.0 max-h=120.0 align-x=center align-y=end clip=true p=8.0 pl=12.0 bg=linear(1.57, bg@0.0, primary/25@1.0) text=fg border=primary border-w=2.0 border-dash=(4.0, 3.0) r=4.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 px-snap=true order=2 grow=1.0 shrink=0.5 basis=percent(40.0) self=flex-end m=auto mx=percent(5.0) mt=-2.0\n      text \"Card\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let container = program.container(ViewId(1)).unwrap();

        assert_eq!(container.id, ViewId(1));
        assert!(matches!(
            container.width,
            Some(ResolvedContainerLength::Fill)
        ));
        assert!(matches!(
            container.height,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert_eq!(container.align_x, Some(ResolvedContainerAlignment::Center));
        assert_eq!(container.align_y, Some(ResolvedContainerAlignment::End));
        assert!(container.clip.is_some());
        assert_eq!(container.border_dash.len(), 2);
        assert!(matches!(
            container.surface.background,
            Some(ResolvedContainerBackground::Linear { ref stops, .. }) if stops.len() == 2
        ));
        assert!(matches!(
            container.surface.border_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                ..
            })
        ));
        let custom = container.custom_style.as_ref().unwrap();
        assert_eq!(
            program.extern_function(custom.function).name,
            "dynamic_container"
        );
        assert_eq!(custom.arguments.len(), 1);
        assert_eq!(
            container.flex_item.align_self,
            Some(ResolvedContainerFlexAlignment::FlexEnd)
        );
        assert!(matches!(
            container.flex_item.basis,
            Some(ResolvedContainerFlexBasis::Percent(_))
        ));
        let margins = container.flex_item.margins.as_ref().unwrap();
        assert!(matches!(margins.top, ResolvedContainerFlexMargin::Fixed(_)));
        assert!(matches!(
            margins.right,
            ResolvedContainerFlexMargin::Percent(_)
        ));
        assert!(matches!(margins.bottom, ResolvedContainerFlexMargin::Auto));
        assert!(matches!(
            margins.left,
            ResolvedContainerFlexMargin::Percent(_)
        ));

        let checked = program.checked_facts().container(ViewId(1)).unwrap();
        assert_eq!(checked.expression_count, 28);
        for index in 0..checked.expression_count {
            let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(1),
                index,
            });
            let expression = program
                .checked_facts()
                .expression_use_by_owner(owner)
                .unwrap();
            assert_eq!(
                program.checked_facts().expression_use(expression).owner,
                owner
            );
        }
    }

    #[test]
    fn container_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedContainer\n{THEME}state\n  size = 80.0\nview\n  flex\n    box h=size p=8.0 bg=primary border=fg border-w=1.0 grow=1.0 mx=percent(5.0)\n      text \"Card\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-container.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        poison_raw_container_expressions(&mut checked.document.view);
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-container.ice").unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let options = raw_container_options(&mut changed_static.document.view);
        options.align_x = Some(FlexAlignment::End);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn container_codegen_ignores_raw_box_and_flex_options_after_lowering() {
        let source = format!(
            "app LoweredContainer\n{THEME}state\n  size = 80.0\nview\n  flex\n    box h=size p=8.0 bg=primary border=fg border-w=1.0 grow=1.0 mx=percent(5.0)\n      text \"Card\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-container.ice").unwrap();

        poison_raw_container_expressions(&mut program.document.view);
        let options = raw_container_options(&mut program.document.view);
        options.align_x = Some(FlexAlignment::End);
        options.style.background = Some(BackgroundValue::Color("danger".into()));
        options.style.border_color = Some("danger".into());
        options.flex_item.align_self = Some(FlexItemAlignment::Stretch);

        let actual = crate::codegen::generate(&program, "lowered-container.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("999.0"));
    }

    fn raw_container_options(node: &mut ViewNode) -> &mut ContainerOptions {
        let ViewNode::Layout { children, .. } = node else {
            panic!("fixture root must be a flex layout");
        };
        let ViewNode::Container { options, .. } = &mut children[0] else {
            panic!("fixture child must be a container");
        };
        options
    }

    fn poison_raw_container_expressions(node: &mut ViewNode) {
        let options = raw_container_options(node);
        options.height = Some(LengthValue::Fixed(Expr::F64(999.0)));
        options.padding.all = Some(Expr::F64(999.0));
        options.style.border_width = Some(Expr::F64(999.0));
        options.flex_item.grow = Some(Expr::F64(999.0));
        options.flex_item.margin.x = Some(FlexMarginValue::Percent(Expr::F64(999.0)));
    }

    #[test]
    fn malformed_checked_container_expression_id_does_not_panic() {
        let source =
            format!("app InvalidContainerFacts\n{THEME}view\n  box p=8.0\n    text \"Card\"\n");
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn imported_container_keeps_physical_origin_and_generated_source_marker() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-container-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("container.ice");
        fs::write(
            &root,
            format!("app ImportedContainerApp\nuse \"container.ice\"\n{THEME}view\n  ImportedContainer\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedContainer()\n  state\n    active = true\n  box p=12.0 bg=primary border=fg border-w=1.0\n    text \"Imported\"\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let container = program.containers.values().next().unwrap();
        let origin = program.origin(container.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 4);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 4 1 {encoded_import}")));

        program.containers.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_linear_grid_stack_flex_and_scroll_layouts() {
        let source = format!(
            "app LayoutHir\nextern crate::backend\n  scroll-style dynamic_scroll(active:bool)\n{THEME}state\n  active = true\non scrolled(ax, ay, rx, ry)\nview\n  col gap=4.0 p=8.0 w=fill max-w=800.0 align=center clip=true\n    row gap=2.0 wrap wrap-gap=3.0 wrap-align=end\n      text \"Linear\"\n    grid cols=2 w=640.0 gap=12.0 h=aspect(16.0,9.0)\n      text \"Grid\"\n    grid min-cell=240.0 gap=12.0\n      text \"Fluid\"\n    stack under=1 w=fill h=80.0 clip=true\n      text \"Under\"\n      text \"Over\"\n    flex direction=row-reverse wrap=wrap-reverse justify=space-between items=flex-end content=space-around row-gap=6.0 col-gap=7.0 p=9.0 w=fill h=100.0 max-w=900.0 max-h=200.0 clip=true\n      text \"Flex\"\n    scroll dir=both w=fill h=200.0 bar=hidden bar-w=8.0 bar-m=2.0 scroller-w=6.0 bar-gap=4.0 anchor-x=end anchor-y=start auto=true scroll=scrolled style=dynamic_scroll(active)\n      text \"Scroll\"\n      active x-disabled=false y-disabled=false\n        box bg=bg text=fg border=primary border-w=1.0 r=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=3.0 px-snap=true\n        x-rail bg=bg\n        x-scroller bg=primary\n        y-rail bg=bg\n        y-scroller bg=primary\n        gap bg=bg\n        auto bg=bg icon=fg\n"
        );
        let source = source.replace(
            "direction=row-reverse wrap=wrap-reverse justify=space-between items=flex-end content=space-around row-gap=6.0 col-gap=7.0",
            "dir=row-reverse wrap=wrap-reverse justify=space-between items=flex-end content=space-around gap-y=6.0 gap-x=7.0",
        );
        let program = lower(analyze(&source).unwrap()).unwrap();

        let root = program.layout(ViewId(0)).unwrap();
        assert!(matches!(
            root.mode,
            ResolvedLayoutMode::Linear(ResolvedLinearLayout {
                axis: ResolvedLinearAxis::Column,
                align: Some(ResolvedContainerAlignment::Center),
                ..
            })
        ));
        assert!(program.layouts.values().any(|layout| matches!(
            layout.mode,
            ResolvedLayoutMode::Grid(ResolvedGridLayout {
                height: Some(ResolvedGridHeight::AspectRatio { .. }),
                ..
            })
        )));
        assert!(program.layouts.values().any(|layout| matches!(
            layout.mode,
            ResolvedLayoutMode::Stack(ResolvedStackLayout { under: 1, .. })
        )));
        assert!(program.layouts.values().any(|layout| matches!(
            layout.mode,
            ResolvedLayoutMode::Flex(ResolvedFlexLayout {
                direction: ResolvedFlexDirection::Row,
                wrap: ResolvedFlexWrap::Wrap,
                min_cell: Some(_),
                ..
            })
        )));
        assert!(program.layouts.values().any(|layout| matches!(
            layout.mode,
            ResolvedLayoutMode::Flex(ResolvedFlexLayout {
                direction: ResolvedFlexDirection::RowReverse,
                wrap: ResolvedFlexWrap::WrapReverse,
                justify_content: Some(ResolvedFlexContentAlignment::SpaceBetween),
                align_items: Some(ResolvedContainerFlexAlignment::FlexEnd),
                align_content: Some(ResolvedFlexContentAlignment::SpaceAround),
                ..
            })
        )));
        let scroll = program
            .layouts
            .values()
            .find_map(|layout| match &layout.mode {
                ResolvedLayoutMode::Scroll(scroll) => Some(scroll.as_ref()),
                _ => None,
            })
            .unwrap();
        assert_eq!(scroll.direction, ResolvedScrollDirection::Both);
        assert!(scroll.hidden_bar);
        assert_eq!(scroll.anchor_x, ResolvedScrollAnchor::End);
        assert_eq!(scroll.anchor_y, ResolvedScrollAnchor::Start);
        assert!(scroll.route.is_some());
        assert_eq!(scroll.styles.len(), 1);
        assert!(scroll.custom_style.is_some());
        assert!(scroll.styles[0].container.background.is_some());
        assert!(scroll.styles[0].auto_scroll_icon.is_some());

        for layout in program.layouts.values() {
            let checked = program.checked_facts().interaction(layout.id).unwrap();
            for index in 0..checked.expression_count {
                let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: layout.id,
                    index,
                });
                let expression = program
                    .checked_facts()
                    .expression_use_by_owner(owner)
                    .unwrap();
                assert_eq!(
                    program.checked_facts().expression_use(expression).owner,
                    owner
                );
            }
        }
    }

    #[test]
    fn layout_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedLayout\n{THEME}state\n  size = 8.0\nview\n  row gap=size p=4.0 w=fill align=center clip=true\n    text \"Checked\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-layout.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        poison_raw_layout_expressions(&mut checked.document.view);
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-layout.ice").unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let options = raw_layout_options(&mut changed_static.document.view);
        options.align = Some(FlexAlignment::End);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn layout_codegen_ignores_raw_options_after_lowering() {
        let source = format!(
            "app LoweredLayout\n{THEME}state\n  size = 8.0\nview\n  row gap=size p=4.0 w=fill align=center clip=true\n    text \"Lowered\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-layout.ice").unwrap();

        poison_raw_layout_expressions(&mut program.document.view);
        let options = raw_layout_options(&mut program.document.view);
        options.align = Some(FlexAlignment::End);
        options.wrap = true;
        options.under = u16::MAX;

        let actual = crate::codegen::generate(&program, "lowered-layout.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("999.0"));
    }

    fn raw_layout_options(node: &mut ViewNode) -> &mut LayoutOptions {
        let ViewNode::Layout { options, .. } = node else {
            panic!("fixture root must be a layout");
        };
        options
    }

    fn poison_raw_layout_expressions(node: &mut ViewNode) {
        let options = raw_layout_options(node);
        options.spacing = Some(Expr::F64(999.0));
        options.padding.all = Some(Expr::F64(999.0));
        options.clip = Some(Expr::Bool(false));
    }

    #[test]
    fn malformed_checked_layout_expression_id_does_not_panic() {
        let source =
            format!("app InvalidLayoutFacts\n{THEME}view\n  row p=8.0\n    text \"Invalid\"\n");
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn imported_layout_keeps_physical_origins_and_generated_source_marker() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-layout-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("layout.ice");
        fs::write(
            &root,
            format!("app ImportedLayoutApp\nuse \"layout.ice\"\n{THEME}view\n  ImportedLayout\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedLayout()\n  scroll h=120.0\n    text \"Imported\"\n    active\n      box bg=bg border=primary border-w=1.0\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let layout = program.layouts.values().next().unwrap();
        let origin = program.origin(layout.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 2);
        let ResolvedLayoutMode::Scroll(scroll) = &layout.mode else {
            panic!("imported layout must be scroll HIR");
        };
        let style_origin = program.origin(scroll.styles[0].origin);
        assert_eq!(style_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(style_origin.parent, Some(layout.origin));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_import}")));

        program.layouts.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 2);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_complete_plain_and_rich_text_contracts() {
        let source = format!(
            r#"app TextHir
extern crate::backend
  text-style dynamic_text(active:bool)
font ui family=sans weight=medium stretch=normal style=normal
{THEME}state
  active = true
  label = "SECTION"
on link(url)
view
  col
    text label w=fill h=40.0 size=16.0 line-h-px=20.0 font=ui align-x=center align-y=bottom shape=advanced style=dynamic_text(active) @font-bold text-primary
    rich-text #story w=fill h=48.0 size=16.0 line-h=1.2 font=ui align-x=justified align-y=center wrap=word color=fg style=dynamic_text(active) @font-bold -> link _
      span "Ice " size=18.0 line-h-px=22.0 font=ui color=primary bg=linear(1.57, bg@0.0, primary@1.0) border=fg border-w=1.0 r=4.0 r-tl=2.0 r-tr=3.0 r-br=5.0 r-bl=6.0 p=2.0 pl=4.0 underline strike=false
      span "language" link="https://example.com" bg=bg size=18.0 @font-bold text-primary
    text "TRACKED" size=12.0 tracking=1.2
"#
        );
        let program = lower(analyze(&source).unwrap()).unwrap();

        let plain = program.text(ViewId(1)).unwrap();
        assert!(matches!(plain.content, ResolvedTextContent::Plain { .. }));
        assert!(matches!(
            plain.options.width,
            Some(ResolvedContainerLength::Fill)
        ));
        assert!(matches!(
            plain.options.height,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert!(matches!(
            plain.options.font,
            Some(ResolvedTextFont::Named(_))
        ));
        assert_eq!(plain.options.align_x, Some(ResolvedTextAlignment::Center));
        assert_eq!(
            plain.options.align_y,
            Some(ResolvedTextVerticalAlignment::Bottom)
        );
        assert_eq!(plain.options.shaping, Some(ResolvedTextShaping::Advanced));
        assert_eq!(plain.options.wrapping, None);
        assert_eq!(plain.options.tracking, None);
        let plain_style = plain.options.custom_style.as_ref().unwrap();
        assert_eq!(
            program.extern_function(plain_style.function).name,
            "dynamic_text"
        );
        assert_eq!(plain_style.arguments.len(), 1);

        let rich = program.text(ViewId(2)).unwrap();
        assert_eq!(rich.options.align_x, Some(ResolvedTextAlignment::Justified));
        let ResolvedTextContent::Rich {
            color,
            spans,
            route,
        } = &rich.content
        else {
            panic!("fixture rich-text must lower to structured content");
        };
        assert!(color.is_some());
        assert!(route.is_some());
        assert_eq!(spans.len(), 2);
        assert!(matches!(
            spans[0].background,
            Some(ResolvedContainerBackground::Linear { ref stops, .. }) if stops.len() == 2
        ));
        assert!(spans[0].border_color.is_some());
        assert!(spans[0].radius.all.is_some());
        assert!(spans[0].radius.bottom_left.is_some());
        assert!(spans[0].padding.all.is_some());
        assert!(spans[0].padding.left.is_some());
        assert!(spans[0].underline.is_some());
        assert!(spans[0].strikethrough.is_some());
        assert!(matches!(spans[0].font, Some(ResolvedTextFont::Named(_))));
        assert!(spans[1].link.is_some());
        assert!(matches!(
            spans[1].background,
            Some(ResolvedContainerBackground::Color(_))
        ));

        for text in program.texts.values() {
            let checked = program.checked_facts().interaction(text.id).unwrap();
            for index in 0..checked.expression_count {
                let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: text.id,
                    index,
                });
                let expression = program
                    .checked_facts()
                    .expression_use_by_owner(owner)
                    .unwrap();
                assert_eq!(
                    program.checked_facts().expression_use(expression).owner,
                    owner
                );
            }
        }
        for span in spans {
            assert_eq!(program.origin(span.origin).parent, Some(rich.origin));
        }
        assert_eq!(program.text(ViewId(3)).unwrap().options.tracking, Some(1.2));
    }

    #[test]
    fn text_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedText\nextern crate::backend\n  text-style dynamic_text(active:bool)\n{THEME}state\n  active = true\n  label = \"Checked\"\nview\n  text label w=80.0 size=14.0 line-h=1.2 style=dynamic_text(active) align-x=center\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-text.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Text { value, options, .. } = &mut checked.document.view else {
            panic!("fixture root must be text");
        };
        *value = Expr::Str("POISONED".into());
        options.width = Some(LengthValue::Fixed(Expr::F64(999.0)));
        options.size = Some(Expr::F64(999.0));
        options.line_height = Some(TextLineHeight::Relative(Expr::F64(999.0)));
        options.custom_style.as_mut().unwrap().args[0] = Expr::Bool(false);
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-text.ice").unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Text { options, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be text");
        };
        options.align_x = Some(TextAlignment::Right);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn text_codegen_ignores_raw_plain_and_rich_content_after_lowering() {
        let source = format!(
            "app LoweredText\n{THEME}on link(url)\nview\n  col\n    text \"Plain\" size=14.0 align-x=center\n    rich-text color=fg -> link _\n      span \"Docs\" link=\"https://example.com\" bg=primary underline\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-text.ice").unwrap();

        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Text {
            value,
            options,
            styles,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be text");
        };
        *value = Expr::Str("POISONED".into());
        *options = TextOptions::default();
        styles.clear();
        let ViewNode::RichText {
            options,
            color,
            spans,
            styles,
            route,
            ..
        } = &mut children[1]
        else {
            panic!("second child must be rich-text");
        };
        *options = TextOptions::default();
        *color = Some("danger".into());
        spans.clear();
        styles.clear();
        *route = None;

        let actual = crate::codegen::generate(&program, "lowered-text.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("POISONED"));
    }

    #[test]
    fn malformed_checked_text_expression_id_does_not_panic() {
        let source = format!("app InvalidTextFacts\n{THEME}view\n  text \"Invalid\" size=14.0\n");
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn imported_rich_text_keeps_root_span_and_route_physical_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-text-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("text.ice");
        fs::write(
            &root,
            format!("app ImportedTextApp\nuse \"text.ice\"\n{THEME}view\n  ImportedText\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedText()\n  on open(url)\n  rich-text color=fg -> open _\n    span \"Docs\" link=\"https://example.com\"\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let text = program.texts.values().next().unwrap();
        let root_origin = program.origin(text.origin);
        assert_eq!(root_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(root_origin.line, 3);
        let ResolvedTextContent::Rich { spans, route, .. } = &text.content else {
            panic!("imported content must be rich-text");
        };
        let span_origin = program.origin(spans[0].origin);
        assert_eq!(span_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(span_origin.line, 4);
        assert_eq!(span_origin.parent, Some(text.origin));
        let route_origin = program.origin(route.as_ref().unwrap().origin);
        assert_eq!(route_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(route_origin.parent, Some(text.origin));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));

        program.texts.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_the_complete_input_contract() {
        let source = r#"app InputHir
extern crate::backend
  input-style dynamic_input(disabled:bool)
font ui family=sans
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
  value = ""
  disabled = false
  secure = true
on changed(next)
  value = next
on submitted
on pasted(next)
  value = next
view
  input "Secret" #secret label="Secure input" description="Private value" <-> value hint="Paste token" disabled=disabled secure=secure change=changed submit=submitted paste=pasted w=240.0 p=8.0 text-size=14.0 line-h=1.2 align=center font=mono style=dynamic_input(disabled)
    active bg=bg border=fg border-w=1.0 r=4.0 icon=primary placeholder=danger value=fg selection=primary
    hovered bg=bg border=primary border-w=1.0 r=10.0 icon=fg placeholder=danger value=fg selection=primary
    focused bg=bg border=primary border-w=1.0 r=10.0
    focused-hovered bg=bg border=fg border-w=1.0 r=10.0
    disabled bg=bg border=primary border-w=1.0 r=10.0 value=danger
    icon code="•" font=ui size=12.0 gap=4.0 side=right
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        let input = program.input(ViewId(0)).unwrap();

        assert_eq!(input.label, "Secret");
        assert_eq!(input.hint, "Paste token");
        assert!(matches!(
            input.binding,
            WritableStateRef::App { ref name, .. } if name == "value"
        ));
        assert!(input.disabled.is_some());
        assert!(input.accessibility_label.is_some());
        assert!(input.accessibility_description.is_some());
        assert!(input.secure.is_some());
        assert!(input.change.is_some());
        assert!(input.submit.is_some());
        assert!(input.paste.is_some());
        assert!(matches!(
            input.width,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert_eq!(input.align, Some(ResolvedInputAlignment::Center));
        assert!(matches!(input.font, Some(ResolvedTextFont::Monospace)));
        let icon = input.icon.as_ref().unwrap();
        assert_eq!(icon.code_point, '•');
        assert_eq!(icon.side, ResolvedInputIconSide::Right);
        assert!(matches!(icon.font, Some(ResolvedTextFont::Named(_))));
        assert!(icon.size.is_some());
        assert!(icon.spacing.is_some());
        assert!(input.custom_style.is_some());
        assert!(input.styles.active.is_some());
        assert!(input.styles.hovered.is_some());
        assert!(input.styles.focused.is_some());
        assert!(input.styles.focused_hovered.is_some());
        assert!(input.styles.disabled.is_some());

        let checked = program.checked_facts().interaction(input.id).unwrap();
        for index in 0..checked.expression_count {
            let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: input.id,
                index,
            });
            let expression = program
                .checked_facts()
                .expression_use_by_owner(owner)
                .unwrap();
            assert_eq!(
                program.checked_facts().expression_use(expression).owner,
                owner
            );
        }
        for route in [&input.change, &input.submit, &input.paste]
            .into_iter()
            .flatten()
        {
            assert_eq!(program.origin(route.origin).parent, Some(input.origin));
        }
        assert_eq!(program.origin(icon.origin).parent, Some(input.origin));
        for status in [
            &input.styles.active,
            &input.styles.hovered,
            &input.styles.focused,
            &input.styles.focused_hovered,
            &input.styles.disabled,
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(program.origin(status.origin).parent, Some(input.origin));
        }
    }

    #[test]
    fn input_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedInput\nextern crate::backend\n  input-style dynamic_input(disabled:bool)\n{THEME}state\n  value = \"\"\n  disabled = false\non changed(next)\n  value = next\nview\n  input \"Field\" <-> value hint=\"Type\" disabled=disabled secure=false change=changed w=80.0 p=4.0 text-size=14.0 line-h=1.2 align=center style=dynamic_input(disabled)\n    active bg=bg border=fg border-w=1.0 r=4.0\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-input.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Input {
            disabled, options, ..
        } = &mut checked.document.view
        else {
            panic!("fixture root must be input");
        };
        *disabled = Some(Expr::Bool(true));
        options.secure = Some(Expr::Bool(true));
        options.width = Some(LengthValue::Fixed(Expr::F64(999.0)));
        options.padding = Some(Expr::F64(999.0));
        options.text_size = Some(Expr::F64(999.0));
        options.line_height = Some(Expr::F64(999.0));
        options.custom_style.as_mut().unwrap().args[0] = Expr::Bool(true);
        let active = options.style.active.as_mut().unwrap();
        active.options.border_width = Some(Expr::F64(999.0));
        active.options.radius = Some(Expr::F64(999.0));
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-input.ice").unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Input { options, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be input");
        };
        options.align = Some(InputAlignment::Right);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));

        let mut changed_route = analyze(&source).unwrap();
        let ViewNode::Input { options, .. } = &mut changed_route.document.view else {
            panic!("fixture root must be input");
        };
        options.change.as_mut().unwrap().handler = "missing".into();
        let error = lower(changed_route).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn input_lowering_rejects_same_arena_binding_identity_swaps() {
        let app_source = format!(
            "app AppBindingIdentity\n{THEME}state\n  first = \"\"\n  second = \"\"\nview\n  input \"Field\" <-> first\n"
        );
        let mut checked = analyze(&app_source).unwrap();
        checked
            .facts
            .corrupt_input_binding(ViewId(0), CheckedValueRef::AppState(AppStateId(1)));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("binding identity diverged"));

        let param_source = format!(
            "app ParamBindingIdentity\n{THEME}state\n  first = \"\"\n  second = \"\"\ncomponent Field(bind first:str, bind second:str)\n  input \"Field\" <-> first\nview\n  Field first<->first second<->second\n"
        );
        let mut checked = analyze(&param_source).unwrap();
        let input = checked
            .declarations
            .view_id(checked.document.components[0].root.span())
            .unwrap();
        checked.facts.corrupt_input_binding(
            input,
            CheckedValueRef::ComponentParam(ComponentParamId {
                component: ComponentId(0),
                index: 1,
            }),
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("binding identity diverged"));

        let state_source = format!(
            "app ComponentStateBindingIdentity\n{THEME}component Field()\n  state\n    first = \"\"\n    second = \"\"\n  input \"Field\" <-> first\nview\n  Field\n"
        );
        let mut checked = analyze(&state_source).unwrap();
        let input = checked
            .declarations
            .view_id(checked.document.components[0].root.span())
            .unwrap();
        checked.facts.corrupt_input_binding(
            input,
            CheckedValueRef::ComponentState(ComponentStateId {
                component: ComponentId(0),
                index: 1,
            }),
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("binding identity diverged"));
    }

    #[test]
    fn input_lowering_rejects_same_widget_route_expression_transplants() {
        let source = format!(
            "app InputRouteIdentity\n{THEME}state\n  value = \"\"\non changed(next)\n  value = next\non pasted(next)\n  value = next\nview\n  input \"Field\" <-> value change=changed(\"change\") paste=pasted(\"paste\")\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .transplant_interaction_route_expression(ViewId(0), 0, 0, 1, 0);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("route expression slot identity diverged")
        );
    }

    #[test]
    fn input_codegen_ignores_raw_widget_contract_after_lowering() {
        let source = format!(
            "app LoweredInput\n{THEME}state\n  value = \"\"\n  spare = \"\"\n  disabled = false\non changed(next)\n  value = next\nview\n  input \"Field\" #field <-> value hint=\"Type\" disabled=disabled change=changed w=80.0\n    active bg=bg border=fg border-w=1.0 r=4.0\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-input.ice").unwrap();

        let ViewNode::Input {
            label,
            binding,
            hint,
            disabled,
            options,
            styles,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be input");
        };
        *label = "POISONED LABEL".into();
        *binding = "missing".into();
        *hint = "POISONED HINT".into();
        *disabled = Some(Expr::Bool(true));
        *options = InputOptions::default();
        styles.clear();

        let actual = crate::codegen::generate(&program, "lowered-input.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("POISONED"));

        program.controlled_inputs[0].state = AppStateId(1);
        let error = crate::codegen::generate(&program, "lowered-input.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("controlled input binding"));

        program.controlled_inputs[0].state = AppStateId(u32::MAX);
        let error = crate::codegen::generate(&program, "lowered-input.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("controlled input binding"));
    }

    #[test]
    fn malformed_checked_input_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidInputFacts\n{THEME}state\n  value = \"\"\n  disabled = false\nview\n  input \"Invalid\" <-> value disabled=disabled\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn imported_input_keeps_widget_icon_status_and_route_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-input-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("field.ice");
        fs::write(
            &root,
            format!(
                "app ImportedInputApp\nuse \"field.ice\"\n{THEME}state\n  value = \"\"\nview\n  ImportedField value<->value\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedField(bind value:str)\n  on changed(next)\n  input \"Imported\" <-> value change=changed\n    active bg=bg border=fg border-w=1.0\n    icon code=\"•\" size=12.0\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let input = program.inputs.values().next().unwrap();
        let input_origin = program.origin(input.origin);
        assert_eq!(input_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(input_origin.line, 3);
        for (child, line) in [
            (input.change.as_ref().unwrap().origin, 3),
            (input.styles.active.as_ref().unwrap().origin, 4),
            (input.icon.as_ref().unwrap().origin, 5),
        ] {
            let child = program.origin(child);
            assert_eq!(child.parent, Some(input.origin));
            assert_eq!(child.path.as_deref(), Some(imported.as_path()));
            assert_eq!(child.line, line);
        }

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));

        program.inputs.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_complete_pick_list_and_combo_box_contracts() {
        let source = r#"app SelectionHir
extern crate::backend
  pick-list-style dynamic_pick(busy:bool)
  input-style dynamic_input(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
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
  busy = false
  choices = ["List", "Board"]
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on searched(next)
on hovered(next)
on opened
on closed
view
  col
    pick choices selected hint="Choose" w=fill menu-h=120.0 p=8.0 text-size=14.0 line-h=1.2 shape=advanced font=ui open=opened close=closed style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
      active text=fg placeholder=danger handle=primary bg=bg border=fg border-w=1.0 r=4.0
      hovered text=fg
      opened text=fg
      opened-hovered text=fg
      menu text=fg selected-text=bg selected-bg=primary bg=bg border=fg border-w=1.0 r=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
      handle dynamic
        closed code="⌄" font=ui size=12.0 line-h=1.0 shape=basic
        open code="⌃" font=ui size=13.0 line-h=1.1 shape=advanced
    combo modes selected "Search modes" w=fill menu-h=120.0 p=8.0 text-size=14.0 line-h=1.2 shape=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
      active bg=bg border=fg border-w=1.0 r=4.0 icon=primary placeholder=danger value=fg selection=primary
      hovered bg=bg icon=fg
      focused bg=bg border=primary
      focused-hovered bg=bg border=fg
      disabled bg=bg value=danger
      menu text=fg selected-text=bg selected-bg=primary bg=bg border=fg border-w=1.0 r=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
      icon code="⌕" font=ui size=12.0 gap=6.0 side=right
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        let pick = program.pick_list(ViewId(1)).unwrap();
        assert_eq!(pick.option_type, Type::Str);
        assert!(matches!(pick.width, Some(ResolvedContainerLength::Fill)));
        assert!(matches!(
            pick.menu_height,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert!(matches!(pick.font, Some(ResolvedTextFont::Named(_))));
        assert!(matches!(
            pick.handle,
            Some(ResolvedPickListHandle::Dynamic { .. })
        ));
        assert!(pick.open.is_some());
        assert!(pick.close.is_some());
        assert!(pick.custom_style.is_some());
        assert!(pick.styles.active.is_some());
        assert!(pick.styles.hovered.is_some());
        assert!(pick.styles.opened.is_some());
        assert!(pick.styles.opened_hovered.is_some());
        assert!(pick.menu.custom.is_some());
        assert!(pick.menu.surface.is_some());
        assert_eq!(
            program.origin(pick.selection.origin).parent,
            Some(pick.origin)
        );

        let combo = program.combo_box(ViewId(2)).unwrap();
        assert_eq!(combo.state.name, "modes");
        assert_eq!(combo.state.option_type, Type::Str);
        assert!(matches!(combo.font, Some(ResolvedTextFont::Named(_))));
        assert!(combo.icon.is_some());
        assert!(combo.input.is_some());
        assert!(combo.hover.is_some());
        assert!(combo.open.is_some());
        assert!(combo.close.is_some());
        assert!(combo.custom_style.is_some());
        assert!(combo.styles.active.is_some());
        assert!(combo.styles.hovered.is_some());
        assert!(combo.styles.focused.is_some());
        assert!(combo.styles.focused_hovered.is_some());
        assert!(combo.styles.disabled.is_some());
        assert!(combo.menu.custom.is_some());
        assert!(combo.menu.surface.is_some());
        assert_eq!(
            program.origin(combo.selection.origin).parent,
            Some(combo.origin)
        );
        for origin in [
            pick.styles.active.as_ref().unwrap().origin,
            pick.menu.origin.unwrap(),
            combo.styles.active.as_ref().unwrap().origin,
            combo.menu.origin.unwrap(),
            combo.icon.as_ref().unwrap().origin,
        ] {
            assert!(program.origin(origin).parent.is_some());
        }
    }

    #[test]
    fn selection_lowering_uses_checked_semantics_and_codegen_ignores_raw_contracts() {
        let source = format!(
            "app CheckedSelection\n{THEME}state\n  choices = [\"One\", \"Two\"]\n  modes:combo[str] = [\"One\", \"Two\"]\n  selected:str? = none\non selected(next)\n  selected = some(next)\non opened\nview\n  col\n    pick choices selected hint=\"Choose\" w=80.0 open=opened -> selected _\n      active bg=bg border=fg border-w=1.0\n    combo modes selected \"Search\" w=80.0 open=opened -> selected _\n      active bg=bg border=fg border-w=1.0\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-selection.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::PickList {
            options,
            selected,
            options_config,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be a pick list");
        };
        *options = Expr::List(vec![]);
        *selected = Expr::None;
        options_config.width = Some(LengthValue::Fixed(Expr::F64(999.0)));
        options_config.placeholder = Some(Expr::Str("POISONED".into()));
        let ViewNode::ComboBox {
            selected, options, ..
        } = &mut children[1]
        else {
            panic!("second child must be a combo box");
        };
        *selected = Expr::None;
        options.width = Some(LengthValue::Fixed(Expr::F64(999.0)));
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-selection.ice").unwrap();
        assert_eq!(actual, expected);

        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-selection.ice").unwrap();
        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::PickList {
            options,
            selected,
            options_config,
            route,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be a pick list");
        };
        *options = Expr::List(vec![]);
        *selected = Expr::None;
        *options_config = PickListOptions::default();
        route.handler = "missing".into();
        let ViewNode::ComboBox {
            state,
            selected,
            placeholder,
            options,
            route,
            ..
        } = &mut children[1]
        else {
            panic!("second child must be a combo box");
        };
        *state = "missing".into();
        *selected = Expr::None;
        *placeholder = "POISONED".into();
        *options = ComboBoxOptions::default();
        route.handler = "missing".into();
        let actual = crate::codegen::generate(&program, "lowered-selection.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("POISONED"));

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::PickList { options_config, .. } = &mut children[0] else {
            panic!("first child must be a pick list");
        };
        options_config.shaping = Some(TextShaping::Advanced);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn combo_lowering_rejects_same_arena_state_identity_swaps() {
        let source = format!(
            "app ComboIdentity\n{THEME}state\n  first:combo[str] = [\"One\"]\n  second:combo[str] = [\"Two\"]\n  selected:str? = none\non selected(next)\nview\n  combo first selected \"Search\" -> selected _\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_combo_box_binding(ViewId(0), CheckedValueRef::AppState(AppStateId(1)));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("binding identity diverged"));
    }

    #[test]
    fn pick_lowering_rejects_same_kind_extern_route_and_status_identity_swaps() {
        let source = format!(
            "app PickIdentity\nextern crate::backend\n  pick-list-style first(flag:bool)\n  pick-list-style second(flag:bool)\n{THEME}state\n  choices = [\"One\"]\n  selected:str? = none\n  flag = false\non selected(next)\nview\n  pick choices selected style=first(flag) -> selected _\n    active bg=bg\n    hovered bg=primary\n"
        );

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_pick_list_style(ViewId(0), ExternFnId(1));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("extern contract diverged"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.swap_pick_list_status_origins(ViewId(0));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("origin diverged"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_interaction_route_id(ViewId(0), 0, 9);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));
    }

    #[test]
    fn malformed_checked_selection_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidSelectionFacts\n{THEME}state\n  choices = [\"One\"]\n  selected:str? = none\non selected(next)\nview\n  pick choices selected -> selected _\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn imported_pick_list_keeps_exact_widget_handle_status_menu_and_route_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-selection-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("selection.ice");
        fs::write(
            &root,
            format!(
                "app ImportedSelectionApp\nuse \"selection.ice\"\n{THEME}state\n  choices = [\"One\"]\n  selected:str? = none\nview\n  ImportedPick choices=choices selected=selected\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedPick(choices:[str], selected:str?)\n  on picked(next)\n  pick choices selected hint=\"Imported\" -> picked _\n    active bg=bg border=fg border-w=1.0\n    menu text=fg selected-bg=primary\n    handle static code=\"⌄\" size=12.0\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let pick = program.pick_lists.values().next().unwrap();
        let root_origin = program.origin(pick.origin);
        assert_eq!(root_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(root_origin.line, 3);
        let ResolvedPickListHandle::Static(icon) = pick.handle.as_ref().unwrap() else {
            panic!("imported pick must retain its static icon");
        };
        for (origin, line) in [
            (pick.selection.origin, 3),
            (pick.styles.active.as_ref().unwrap().origin, 4),
            (pick.menu.origin.unwrap(), 5),
            (icon.origin, 6),
        ] {
            let child = program.origin(origin);
            assert_eq!(child.parent, Some(pick.origin));
            assert_eq!(child.path.as_deref(), Some(imported.as_path()));
            assert_eq!(child.line, line);
        }

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));

        program.pick_lists.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_the_complete_button_contract() {
        let source = r#"app ButtonHir
extern crate::backend
  button-style first_style(disabled:bool)
  button-style second_style(disabled:bool)
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
recipe action for button
  @px-3 bg-primary hover:bg-fg pressed:bg-danger disabled:bg-bg text-fg disabled:text-danger border border-fg rounded-lg disabled:opacity-50
state
  disabled = false
on pressed
view
  button "Save" #save label="Save action" description="Writes changes" disabled=disabled w=240.0 h=48.0 p=8.0 clip=false style=first_style(disabled) @action -> pressed
    active bg=linear(1.57, primary@0.0, bg@1.0) text=fg border=primary border-w=1.0 r=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 px-snap=true
    hovered bg=fg text=bg r=6.0
    pressed bg=primary text=white r=8.0
    disabled bg=bg text=danger r=10.0
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        let button = program.button(ViewId(0)).unwrap();

        assert_eq!(button.id, ViewId(0));
        assert_eq!(button.content, ResolvedButtonContent::Label("Save".into()));
        assert!(button.disabled.is_some());
        assert!(button.accessibility_label.is_some());
        assert!(button.accessibility_description.is_some());
        assert!(matches!(
            button.width,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert!(matches!(
            button.height,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert!(button.padding.is_some());
        assert!(button.clip.is_some());
        assert_eq!(button.preset, ResolvedButtonPreset::Primary);
        assert_eq!(
            button.custom_style.as_ref().unwrap().function,
            ExternFnId(0)
        );
        assert!(button.styles.active.is_some());
        assert!(button.styles.hovered.is_some());
        assert!(button.styles.pressed.is_some());
        assert!(button.styles.disabled.is_some());
        assert!(button.utility_style.background.is_some());
        assert!(button.utility_style.hover_background.is_some());
        assert!(button.utility_style.pressed_background.is_some());
        assert!(button.utility_style.disabled_background.is_some());
        assert_eq!(
            program.origin(button.route.origin).parent,
            Some(button.origin)
        );
        for status in [
            &button.styles.active,
            &button.styles.hovered,
            &button.styles.pressed,
            &button.styles.disabled,
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(program.origin(status.origin).parent, Some(button.origin));
        }
        let active = button.styles.active.as_ref().unwrap();
        assert!(matches!(
            active.surface.background,
            Some(ResolvedContainerBackground::Linear { .. })
        ));
        assert!(active.surface.shadow_color.is_some());
        assert!(active.surface.pixel_snap.is_some());
    }

    #[test]
    fn button_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedButton\nextern crate::backend\n  button-style dynamic_button(disabled:bool)\n{THEME}state\n  disabled = false\non pressed\nview\n  button \"Save\" disabled=disabled w=80.0 h=32.0 p=4.0 clip=false style=dynamic_button(disabled) -> pressed\n    active bg=bg border=fg border-w=1.0 r=4.0 shadow-x=1.0 px-snap=true\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-button.ice",
        )
        .unwrap();
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Button {
            disabled, options, ..
        } = &mut checked.document.view
        else {
            panic!("fixture root must be a button");
        };
        *disabled = Some(Expr::Bool(true));
        options.width = Some(LengthValue::Fixed(Expr::F64(999.0)));
        options.height = Some(LengthValue::Fixed(Expr::F64(999.0)));
        options.padding = Some(Expr::F64(999.0));
        options.clip = Some(Expr::Bool(true));
        options.style.custom.as_mut().unwrap().args[0] = Expr::Bool(true);
        let active = options.style.active.as_mut().unwrap();
        active.options.border_width = Some(Expr::F64(999.0));
        active.options.radius = Some(Expr::F64(999.0));
        active.options.shadow_x = Some(Expr::F64(999.0));
        active.options.pixel_snap = Some(Expr::Bool(false));
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-button.ice").unwrap();
        assert_eq!(actual, expected);

        let mut changed = analyze(&source).unwrap();
        let ViewNode::Button { label, .. } = &mut changed.document.view else {
            panic!("fixture root must be a button");
        };
        *label = Some("Changed".into());
        let error = lower(changed).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn button_codegen_ignores_raw_contract_after_lowering() {
        let source = format!(
            "app LoweredButton\n{THEME}state\n  disabled = false\non pressed\nview\n  button \"Save\" #save disabled=disabled w=80.0 @bg-primary -> pressed\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-button.ice").unwrap();
        let ViewNode::Button {
            label,
            disabled,
            options,
            styles,
            route,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be a button");
        };
        *label = Some("POISONED".into());
        *disabled = Some(Expr::Bool(true));
        *options = ButtonOptions::default();
        styles.clear();
        route.handler = "poisoned".into();
        let actual = crate::codegen::generate(&program, "lowered-button.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("POISONED"));
    }

    #[test]
    fn button_lowering_rejects_same_arena_expression_and_extern_swaps() {
        let source = format!(
            "app ButtonIdentity\nextern crate::backend\n  button-style first_style(disabled:bool)\n  button-style second_style(disabled:bool)\n{THEME}state\n  disabled = false\non pressed\nview\n  button \"Save\" disabled=disabled clip=false style=first_style(disabled) -> pressed\n"
        );
        let mut swapped_expression = analyze(&source).unwrap();
        swapped_expression
            .facts
            .swap_interaction_option_expressions(ViewId(0), 0, 1);
        let error = lower(swapped_expression).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut swapped_extern = analyze(&source).unwrap();
        swapped_extern
            .facts
            .corrupt_button_style(ViewId(0), ExternFnId(1));
        let error = lower(swapped_extern).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("extern contract diverged"));
    }

    #[test]
    fn malformed_checked_button_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidButtonFacts\n{THEME}state\n  disabled = false\non pressed\nview\n  button \"Invalid\" disabled=disabled -> pressed\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn imported_button_keeps_widget_status_and_route_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-button-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("button.ice");
        fs::write(
            &root,
            format!(
                "app ImportedButtonApp\nuse \"button.ice\"\n{THEME}on pressed\nview\n  ImportedButton\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedButton()\n  on pressed\n  button \"Imported\" -> pressed\n    active bg=bg border=fg border-w=1.0\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let button = program.buttons.values().next().unwrap();
        let button_origin = program.origin(button.origin);
        assert_eq!(button_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(button_origin.line, 3);
        for (child, line) in [
            (button.route.origin, 3),
            (button.styles.active.as_ref().unwrap().origin, 4),
        ] {
            let child = program.origin(child);
            assert_eq!(child.parent, Some(button.origin));
            assert_eq!(child.path.as_deref(), Some(imported.as_path()));
            assert_eq!(child.line, line);
        }
        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));

        program.buttons.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "large normalized button lowering and emission performance contract"]
    fn performance_contract_four_thousand_buttons_lower_and_emit_under_two_seconds() {
        const BUTTONS: usize = 4_000;
        let mut source =
            format!("app ButtonScale\n{THEME}state\n  disabled = false\non pressed\nview\n  col\n");
        for index in 0..BUTTONS {
            writeln!(
                source,
                "    button \"Button {index}\" #button_{index} disabled=disabled w=120.0 h=32.0 p=4.0 clip=false -> pressed"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "button-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.buttons.len(), BUTTONS);
        assert_eq!(
            generated.matches("::iced::widget::button(").count(),
            BUTTONS
        );
        eprintln!("4k normalized buttons lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized buttons lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    fn normalizes_complete_boolean_control_contracts_and_radio_value_routing() {
        let source = r#"app BooleanHir
extern crate::backend
  checkbox-style checkbox_surface(active:bool)
  toggler-style toggler_surface(active:bool)
  radio-style radio_surface(active:bool)
font ui family=sans
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
  enabled = false
  choice = "first"
on enabled_changed(next)
  enabled = next
on choice_changed(next)
  choice = next
view
  col
    checkbox "Checkbox" checked=enabled disabled=false label="Accessible checkbox" description="Checked state" style=checkbox_surface(enabled) size=20.0 w=fill gap=8.0 text-size=14.0 line-h=1.2 shape=advanced wrap=word-or-glyph font=ui icon="✓" icon-size=12.0 icon-line-h=1.0 icon-shape=basic -> enabled_changed _
      active checked bg=linear(1.57, primary@0.0, bg@1.0) icon=fg text=fg border=primary border-w=1.0 r=4.0
    toggler "Toggler" checked=enabled style=toggler_surface(enabled) size=20.0 gap=8.0 align=right -> enabled_changed _
      active checked bg=bg bg-border=primary bg-border-w=1.0 fg=fg fg-border=primary fg-border-w=2.0 text=fg r=7.0 p-ratio=0.125
    radio "First" value="first" selected=(choice == "first") style=radio_surface(enabled) size=20.0 gap=8.0 -> choice_changed _
      active selected bg=bg dot=fg border=primary border-w=2.0 text=fg
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        let checkbox = program.boolean_control(ViewId(1)).unwrap();
        assert_eq!(checkbox.kind, ResolvedBooleanKind::Checkbox);
        assert!(checkbox.disabled.is_some());
        assert!(checkbox.options.accessibility_label.is_some());
        assert!(checkbox.options.accessibility_description.is_some());
        assert!(matches!(
            checkbox.options.width,
            Some(ResolvedContainerLength::Fill)
        ));
        assert!(matches!(
            checkbox.options.font,
            Some(ResolvedTextFont::Named(_))
        ));
        assert!(checkbox.options.font_origin.is_some());
        assert!(checkbox.options.icon.is_some());
        let ResolvedBooleanStyle::Checkbox(styles) = &checkbox.style else {
            panic!("checkbox style must be normalized");
        };
        assert!(styles.custom.is_some());
        let active = styles.active_checked.as_ref().unwrap();
        assert!(active.background.is_some());
        assert!(active.icon_color.is_some());
        assert!(active.text_color.is_some());
        assert!(active.border_color.is_some());
        assert!(active.border_width.is_some());
        assert!(active.radius.all.is_some());

        let toggler = program.boolean_control(ViewId(2)).unwrap();
        assert_eq!(toggler.kind, ResolvedBooleanKind::Toggler);
        assert_eq!(
            toggler.options.alignment,
            Some(ResolvedTextAlignment::Right)
        );
        let ResolvedBooleanStyle::Toggler(styles) = &toggler.style else {
            panic!("toggler style must be normalized");
        };
        assert!(styles.active_checked.as_ref().unwrap().foreground.is_some());

        let radio = program.boolean_control(ViewId(3)).unwrap();
        assert_eq!(radio.kind, ResolvedBooleanKind::Radio);
        assert!(radio.value.is_some());
        assert!(radio.disabled.is_none());
        let ResolvedBooleanStyle::Radio(styles) = &radio.style else {
            panic!("radio style must be normalized");
        };
        assert!(styles.active_selected.is_some());

        let generated = crate::codegen::generate(&program, "boolean-hir.ice").unwrap();
        assert!(
            generated.contains("move |_| __BooleanHirMessage::ChoiceChanged(\"first\".to_owned())")
        );
        assert!(!generated.contains("move |__value| __BooleanHirMessage::ChoiceChanged(__value)"));
    }

    #[test]
    fn boolean_codegen_ignores_raw_semantics_after_lowering() {
        let source = format!(
            "app LoweredBoolean\n{THEME}state\n  enabled = false\n  choice = \"first\"\non changed(next)\n  enabled = next\non selected(next)\n  choice = next\nview\n  col\n    checkbox \"Checkbox\" #check checked=enabled size=20.0 gap=8.0 -> changed _\n    toggler \"Toggler\" #toggle(choice) checked=enabled size=20.0 gap=8.0 -> changed _\n    radio \"First\" #radio value=\"first\" selected=(choice == \"first\") size=20.0 gap=8.0 -> selected _\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-boolean.ice").unwrap();
        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Checkbox {
            id,
            label,
            checked,
            options,
            style,
            route,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be a checkbox");
        };
        *label = Expr::Str("POISONED CHECKBOX".into());
        *checked = Expr::Bool(true);
        *options = BoolControlOptions::default();
        **style = CheckboxStyleSet::default();
        id.as_mut().unwrap().name = "poisoned-check".into();
        route.handler = "poisoned".into();
        let ViewNode::Toggler {
            id, options, route, ..
        } = &mut children[1]
        else {
            panic!("second child must be a toggler");
        };
        *options = BoolControlOptions::default();
        id.as_mut().unwrap().name = "poisoned-toggle".into();
        id.as_mut().unwrap().key = Some(Expr::Str("POISONED KEY".into()));
        route.handler = "poisoned".into();
        let ViewNode::Radio {
            id,
            value,
            selected,
            options,
            route,
            ..
        } = &mut children[2]
        else {
            panic!("third child must be a radio");
        };
        *value = Expr::Str("POISONED RADIO".into());
        *selected = Expr::Bool(true);
        *options = BoolControlOptions::default();
        id.as_mut().unwrap().name = "poisoned-radio".into();
        route.handler = "poisoned".into();

        let actual = crate::codegen::generate(&program, "lowered-boolean.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("POISONED"));
        assert!(!actual.contains("poisoned"));
    }

    #[test]
    fn view_family_hir_rejects_an_extra_cross_family_entry() {
        let source = format!(
            "app ExtraViewFamily\n{THEME}view\n  col\n    text \"first\"\n    text \"second\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let root = program.app_view;
        let root_origin = program.views[root.0 as usize].origin;
        let mut extra = program.texts.values().next().unwrap().clone();
        extra.id = root;
        extra.origin = root_origin;
        assert!(program.texts.insert(root, extra).is_none());

        let error = crate::codegen::generate(&program, "extra-view-family.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("text normalized HIR cardinality"));
    }

    #[test]
    fn boolean_lowering_rejects_same_type_slot_route_extern_and_id_swaps() {
        let source = format!(
            "app BooleanIdentity\nextern crate::backend\n  checkbox-style first(active:bool)\n  checkbox-style second(active:bool)\n{THEME}state\n  enabled = false\non changed(next)\n  enabled = next\nview\n  checkbox \"Checkbox\" checked=enabled style=first(enabled) size=20.0 gap=8.0 -> changed _\n"
        );
        let mut swapped_slots = analyze(&source).unwrap();
        swapped_slots
            .facts
            .swap_interaction_option_expressions(ViewId(0), 2, 3);
        let error = lower(swapped_slots).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut swapped_extern = analyze(&source).unwrap();
        swapped_extern
            .facts
            .corrupt_boolean_style(ViewId(0), ExternFnId(1));
        let error = lower(swapped_extern).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("extern contract diverged"));

        let mut corrupt_id = analyze(&source).unwrap();
        corrupt_id
            .facts
            .corrupt_boolean_control_id(ViewId(0), u32::MAX);
        let error = lower(corrupt_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));

        let routes = format!(
            "app BooleanRouteIdentity\n{THEME}state\n  enabled = false\non first(next)\n  enabled = next\non second(next)\n  enabled = next\nview\n  col\n    checkbox \"First\" checked=enabled -> first _\n    checkbox \"Second\" checked=enabled -> second _\n"
        );
        let mut swapped_routes = analyze(&routes).unwrap();
        swapped_routes
            .facts
            .swap_interaction_routes(ViewId(1), ViewId(2));
        let error = lower(swapped_routes).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));

        let mut corrupt_route = analyze(&source).unwrap();
        corrupt_route
            .facts
            .corrupt_interaction_route_id(ViewId(0), 0, u32::MAX);
        let error = lower(corrupt_route).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));
    }

    #[test]
    fn imported_boolean_control_keeps_exact_expression_route_extern_font_theme_and_status_origins()
    {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-boolean-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("toggle.ice");
        fs::write(
            &root,
            format!(
                "app ImportedBooleanApp\nuse \"toggle.ice\"\nextern crate::backend\n  checkbox-style imported_style(active:bool)\nfont ui family=sans\n{THEME}state\n  enabled = false\nview\n  ImportedToggle value<->enabled\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedToggle(bind value:bool)\n  on changed(next)\n  checkbox \"Imported\" checked=value font=ui style=imported_style(value) size=20.0 -> changed _\n    active checked bg=bg icon=fg text=fg border=primary border-w=1.0\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let control = program.boolean_controls.values().next().unwrap();
        let control_origin = program.origin(control.origin);
        assert_eq!(control_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(control_origin.line, 3);
        let expression_origin =
            program.origin(program.checked_facts().expression_use(control.label).origin);
        assert_eq!(expression_origin.parent, Some(control.origin));
        assert_eq!(expression_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(expression_origin.line, 3);
        let route_origin = program.origin(control.route.origin);
        assert_eq!(route_origin.parent, Some(control.origin));
        assert_eq!(route_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(route_origin.line, 3);
        let ResolvedBooleanStyle::Checkbox(styles) = &control.style else {
            panic!("imported control must be a checkbox");
        };
        let custom_origin = program.origin(styles.custom.as_ref().unwrap().origin);
        assert_eq!(custom_origin.parent, Some(control.origin));
        assert_eq!(custom_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(custom_origin.line, 3);
        let font_origin = program.origin(control.options.font_origin.unwrap());
        assert_eq!(font_origin.parent, Some(control.origin));
        assert_eq!(font_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(font_origin.line, 3);
        let status = styles.active_checked.as_ref().unwrap();
        let status_origin = program.origin(status.origin);
        assert_eq!(status_origin.parent, Some(control.origin));
        assert_eq!(status_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(status_origin.line, 4);
        for color in [
            status.icon_color.as_ref(),
            status.text_color.as_ref(),
            status.border_color.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(color.origin, status.origin);
        }
        assert_eq!(status.background.as_ref().unwrap().origin, status.origin);
        let metric_origin = program.origin(
            program
                .checked_facts()
                .expression_use(status.border_width.unwrap())
                .origin,
        );
        assert_eq!(metric_origin.parent, Some(status.origin));
        assert_eq!(metric_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(metric_origin.line, 4);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));

        program.boolean_controls.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "large normalized boolean-control lowering and emission performance contract"]
    fn performance_contract_four_thousand_boolean_controls_lower_and_emit_under_two_seconds() {
        const CONTROLS: usize = 4_000;
        let mut source = format!(
            "app BooleanScale\n{THEME}state\n  enabled = false\non changed(next)\n  enabled = next\nview\n  col\n"
        );
        for index in 0..CONTROLS {
            writeln!(
                source,
                "    checkbox \"Control {index}\" checked=enabled size=20.0 gap=8.0 -> changed _"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "boolean-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.boolean_controls.len(), CONTROLS);
        assert_eq!(
            generated.matches("::iced::widget::checkbox(").count(),
            CONTROLS
        );
        eprintln!("4k normalized boolean controls lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized boolean controls lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    fn normalizes_the_complete_extern_component_contract() {
        let source = format!(
            "app ExternComponentHir\nextern crate::backend\n  component borrowed_surface(label:&str, active:&bool, choice:bool?) -> bool\n{THEME}state\n  label = \"Native\"\n  active = false\non changed(next)\nview\n  extern borrowed_surface(label, active, none) -> changed _\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let component = program.extern_component(ViewId(0)).unwrap();
        let declarations = program.extern_component_declarations().unwrap();

        assert_eq!(component.id, ViewId(0));
        assert_eq!(component.function.id, ExternFnId(0));
        assert_eq!(component.function.name, "borrowed_surface");
        assert_eq!(
            component.function.rust_path,
            "crate::backend::borrowed_surface"
        );
        assert_eq!(component.arguments.len(), 3);
        assert_eq!(component.arguments[0].ty, Type::Str);
        assert_eq!(
            component.arguments[0].mode,
            ResolvedExternComponentArgumentMode::BorrowedAsRef
        );
        assert_eq!(component.arguments[1].ty, Type::Bool);
        assert_eq!(
            component.arguments[1].mode,
            ResolvedExternComponentArgumentMode::Borrowed
        );
        assert_eq!(
            component.arguments[2].ty,
            Type::Option(Box::new(Type::Bool))
        );
        assert_eq!(
            component.arguments[2].mode,
            ResolvedExternComponentArgumentMode::Owned
        );
        assert_eq!(component.output, Type::Bool);
        assert!(matches!(
            component.route.as_ref().unwrap().target,
            ResolvedInteractionRouteTarget::TargetHandler(_)
        ));
        assert_eq!(
            component.route.as_ref().unwrap().source_payloads,
            vec![Type::Bool]
        );
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].id, ExternFnId(0));
        assert_eq!(declarations[0].name, "borrowed_surface");
        assert_eq!(declarations[0].parameters.len(), 3);
        assert_eq!(
            declarations[0].parameters[0].mode,
            ResolvedExternComponentArgumentMode::BorrowedAsRef
        );
        assert_eq!(
            declarations[0].parameters[1].mode,
            ResolvedExternComponentArgumentMode::Borrowed
        );
        assert_eq!(declarations[0].output, Type::Bool);

        let generated = crate::codegen::generate(&program, "extern-component-hir.ice").unwrap();
        assert!(generated.contains("crate::backend::borrowed_surface("));
        assert!(generated.contains("::std::convert::AsRef::as_ref"));
        assert!(generated.contains("::std::borrow::Borrow::borrow"));
        assert!(generated.contains("__ExternComponentHirMessage::Changed(__value)"));
    }

    #[test]
    fn extern_component_routes_direct_component_handlers_and_component_outputs() {
        let direct = format!(
            "app ExternDirect\nextern crate::backend\n  component native_surface(active:bool) -> bool\n{THEME}state\n  active = false\non changed(next)\nview\n  extern native_surface(active) -> changed _\n"
        );
        let program = lower(analyze(&direct).unwrap()).unwrap();
        assert!(matches!(
            program
                .extern_component(ViewId(0))
                .unwrap()
                .route
                .as_ref()
                .unwrap()
                .target,
            ResolvedInteractionRouteTarget::TargetHandler(_)
        ));

        let local = format!(
            "app ExternLocal\nextern crate::backend\n  component native_surface(active:bool) -> bool\n{THEME}state\n  active = false\ncomponent Wrapper(active:bool)\n  on changed(next)\n  extern native_surface(active) -> changed _\nview\n  Wrapper active=active\n"
        );
        let program = lower(analyze(&local).unwrap()).unwrap();
        let component = program.extern_components.values().next().unwrap();
        assert!(matches!(
            component.route.as_ref().unwrap().target,
            ResolvedInteractionRouteTarget::TargetHandler(_)
        ));

        let output = format!(
            "app ExternOutput\nextern crate::backend\n  component native_surface(active:bool) -> bool\n{THEME}state\n  active = false\ncomponent Wrapper(active:bool) -> bool\n  extern native_surface(active) -> emit(_)\non changed(next)\nview\n  Wrapper active=active -> changed _\n"
        );
        let program = lower(analyze(&output).unwrap()).unwrap();
        let component = program.extern_components.values().next().unwrap();
        assert!(matches!(
            component.route.as_ref().unwrap().target,
            ResolvedInteractionRouteTarget::OutputCallback {
                output: Type::Bool,
                ..
            }
        ));
        let generated = crate::codegen::generate(&program, "extern-output.ice").unwrap();
        assert!(generated.contains("crate::backend::native_surface("));
        assert!(generated.contains("__ExternOutputMessage::Changed(__value)"));

        let no_output = format!(
            "app ExternUnit\nextern crate::backend\n  component native_surface(active:bool) -> unit\n{THEME}state\n  active = false\nview\n  extern native_surface(active)\n"
        );
        let program = lower(analyze(&no_output).unwrap()).unwrap();
        let component = program.extern_component(ViewId(0)).unwrap();
        assert_eq!(component.output, Type::Unit);
        assert!(component.route.is_none());
        let generated = crate::codegen::generate(&program, "extern-unit.ice").unwrap();
        assert!(generated.contains("__ExternNoop"));
    }

    #[test]
    fn extern_component_codegen_ignores_every_raw_semantic_field_after_lowering() {
        let source = format!(
            "app ExternPoison\nextern crate::backend\n  component primary_surface(active:bool) -> bool\n  component unused_surface(label:&str, active:&bool, values:&[i64]) -> unit\n{THEME}state\n  active = false\non changed(next)\nview\n  extern primary_surface(active) -> changed _\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "extern-poison.ice").unwrap();
        assert!(expected.contains("fn __ui_lang_check_component_unused_surface<'a>"));
        assert!(expected.contains("arg0: &'a str, arg1: &'a bool, arg2: &'a [i64]"));
        let ViewNode::ExternComponent {
            function,
            args,
            route,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be an extern component");
        };
        *function = "POISONED_VIEW_FUNCTION".into();
        *args = vec![Expr::Str("POISONED".into())];
        *route = None;
        for (index, declaration) in program.document.functions.iter_mut().enumerate() {
            declaration.kind = ExternKind::Future;
            declaration.name = format!("POISONED_DECLARATION_{index}");
            declaration.rust_path = format!("crate::poisoned::declaration_{index}");
            declaration.params = vec![("poisoned".into(), Type::Bytes)];
            declaration.borrowed = vec![false];
            declaration.progress = Some(Type::I64);
            declaration.output = Type::Bytes;
            declaration.error = Some(Type::Str);
            declaration.span = Span::line(900 + index);
        }

        let actual = crate::codegen::generate(&program, "extern-poison.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("POISONED"));
    }

    #[test]
    fn extern_component_raw_declarations_cannot_enable_codegen_helpers() {
        let source = format!(
            "app ExternHelperPoison\nextern crate::backend\n  component native_surface(active:bool) -> unit\n{THEME}state\n  active = false\nview\n  extern native_surface(active)\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "extern-helper-poison.ice").unwrap();
        assert!(!expected.contains("type __IceEventStream"));
        assert!(!expected.contains("fn __ice_map_editor_binding"));
        assert_eq!(expected.matches("struct __IceWidgetTarget").count(), 1);

        program.document.functions[0].kind = ExternKind::EventFilter;
        let actual = crate::codegen::generate(&program, "extern-helper-poison.ice").unwrap();
        assert_eq!(actual, expected);

        program.document.functions[0].kind = ExternKind::EditorBinding;
        let actual = crate::codegen::generate(&program, "extern-helper-poison.ice").unwrap();
        assert_eq!(actual, expected);

        let declaration = &mut program.document.functions[0];
        declaration.kind = ExternKind::Component;
        declaration.params[0].1 = Type::WidgetTarget;
        declaration.output = Type::WidgetTarget;
        declaration.progress = Some(Type::WidgetTarget);
        declaration.error = Some(Type::WidgetTarget);
        let actual = crate::codegen::generate(&program, "extern-helper-poison.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalized_unused_extern_component_widget_targets_enable_codegen_helpers() {
        let source = format!(
            "app ExternWidgetTargetHelpers\nextern crate::backend\n  component target_input(target:widget-target?) -> unit\n  component target_output() -> [widget-target]\n{THEME}view\n  text \"ready\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let generated = crate::codegen::generate(&program, "extern-widget-target.ice").unwrap();

        assert_eq!(generated.matches("struct __IceWidgetTarget").count(), 1);
        assert!(generated.contains(
            "fn __ui_lang_check_component_target_input(arg0: ::std::option::Option<__IceWidgetTarget>)"
        ));
        assert!(generated.contains(
            "fn __ui_lang_check_component_target_output() { let _: __IceElement<'static, ::std::vec::Vec<__IceWidgetTarget>>"
        ));
    }

    #[test]
    fn extern_component_lowering_rejects_cross_owner_identity_route_origin_and_id_corruption() {
        let source = format!(
            "app ExternIdentity\nextern crate::backend\n  component first_surface(active:bool) -> bool\n  component second_surface(active:bool) -> bool\n{THEME}state\n  active = false\non first_changed(next)\non second_changed(next)\nview\n  col\n    extern first_surface(active) -> first_changed _\n    extern second_surface(active) -> second_changed _\n"
        );
        let first = ViewId(1);
        let second = ViewId(2);

        let mut swapped_expressions = analyze(&source).unwrap();
        swapped_expressions
            .facts
            .transplant_interaction_option_expression(first, 0, second, 0);
        let error = lower(swapped_expressions).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("argument 0 contract diverged"));

        let mut corrupt_function = analyze(&source).unwrap();
        corrupt_function
            .facts
            .corrupt_extern_component_function(first, ExternFnId(1));
        let error = lower(corrupt_function).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("declaration contract diverged"));

        let mut corrupt_output = analyze(&source).unwrap();
        corrupt_output
            .facts
            .corrupt_extern_component_output(first, Type::Unit);
        let error = lower(corrupt_output).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("declaration contract diverged"));

        let mut swapped_routes = analyze(&source).unwrap();
        swapped_routes.facts.swap_interaction_routes(first, second);
        let error = lower(swapped_routes).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));

        let mut corrupt_origin = analyze(&source).unwrap();
        corrupt_origin
            .facts
            .corrupt_interaction_expression_origin(first, 0, u32::MAX);
        let error = lower(corrupt_origin).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("argument origin is invalid"));

        let mut corrupt_id = analyze(&source).unwrap();
        corrupt_id
            .facts
            .corrupt_extern_component_id(first, u32::MAX);
        let error = lower(corrupt_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));
    }

    #[test]
    fn imported_extern_component_keeps_origins_source_marker_and_hir_diagnostic() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-extern-component-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("native.ice");
        fs::write(
            &root,
            format!(
                "app ImportedExternApp\nuse \"native.ice\"\n{THEME}state\n  label = \"Imported\"\non opened(next)\nview\n  ImportedSurface label=label -> opened _\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "extern crate::backend\n  component imported_surface(label:&str) -> bool\ncomponent ImportedSurface(label:str) -> bool\n  extern imported_surface(label) -> emit(_)\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let component = program.extern_components.values().next().unwrap();
        let declaration = &program.extern_component_declarations().unwrap()[0];
        let widget_origin = program.origin(component.origin);
        assert_eq!(widget_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(widget_origin.line, 4);
        let declaration_origin = program.origin(component.function.declaration_origin);
        assert_eq!(declaration_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(declaration_origin.line, 2);
        assert_eq!(declaration.id, ExternFnId(0));
        assert_eq!(declaration.name, "imported_surface");
        assert_eq!(declaration.origin, component.function.declaration_origin);
        let argument_origin = program.origin(component.arguments[0].origin);
        assert_eq!(argument_origin.parent, Some(component.origin));
        assert_eq!(argument_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(argument_origin.line, 4);
        let route_origin = program.origin(component.route.as_ref().unwrap().origin);
        assert_eq!(route_origin.parent, Some(component.origin));
        assert_eq!(route_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(route_origin.line, 4);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_import}")));
        assert!(generated.contains(&format!("// __ICE_SOURCE 4 1 {encoded_import}")));

        program.extern_component_declarations[0].id = ExternFnId(u32::MAX);
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 2);
        program.extern_component_declarations[0].id = ExternFnId(0);

        program.extern_components.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn extern_component_declaration_invariants_use_checked_origins() {
        let source = format!(
            "app ExternDeclarationOrigins\nextern crate::backend\n  component first_surface() -> unit\n  component second_surface() -> unit\n{THEME}view\n  text \"ready\"\n"
        );

        let mut swapped = lower(analyze(&source).unwrap()).unwrap();
        let first_origin = swapped.extern_component_declarations[0].origin;
        let second_origin = swapped.extern_component_declarations[1].origin;
        let expected_line = swapped.origin(first_origin).line;
        swapped.extern_component_declarations[0].origin = second_origin;
        swapped.extern_component_declarations[1].origin = first_origin;
        let error = crate::codegen::generate(&swapped, "extern-origin-swap.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("declaration HIR diverged"));

        let mut invalid_first = lower(analyze(&source).unwrap()).unwrap();
        let expected_origin = invalid_first.extern_component_declarations[0].origin;
        let expected_line = invalid_first.origin(expected_origin).line;
        invalid_first.extern_component_declarations[0].origin = OriginId(u32::MAX);
        invalid_first.extern_component_declarations.pop();
        let error = crate::codegen::generate(&invalid_first, "extern-invalid-first-origin.ice")
            .unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, expected_line);
        assert!(error.message.contains("HIR cardinality diverged"));
    }

    #[test]
    #[ignore = "large normalized extern-component lowering and emission performance contract"]
    fn performance_contract_four_thousand_extern_components_lower_and_emit_under_two_seconds() {
        const EXTERN_COMPONENTS: usize = 4_000;
        let mut source = format!(
            "app ExternScale\nextern crate::backend\n  component native_surface(index:i64) -> unit\n{THEME}state\n  index = 7\nview\n  col\n"
        );
        for _ in 0..EXTERN_COMPONENTS {
            writeln!(source, "    extern native_surface(index)").unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "extern-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.extern_components.len(), EXTERN_COMPONENTS);
        assert_eq!(
            generated
                .matches(".map(move |__value| __ExternScaleMessage::__ExternNoop)")
                .count(),
            EXTERN_COMPONENTS
        );
        eprintln!("4k normalized extern components lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized extern components lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    fn normalizes_complete_themer_and_shader_contracts() {
        let source = format!(
            "app ExternViewsHir\nextern crate::backend\n  themer alternate_surface(label:str, active:bool, choice:bool?) -> bool\n  shader shader_surface(label:str, active:bool, choice:bool?) -> bool\n  themer passive_theme() -> unit\n  shader passive_shader() -> unit\n{THEME}state\n  label = \"Native\"\n  active = false\n  fixed_length:length = length.fixed(48.0)\non themed(next)\non shaded(next)\nview\n  col\n    themer alternate_surface(label, active, none) -> themed _\n    shader shader_surface(label, active, none) w=fill(3) h=64.0 -> shaded _\n    themer passive_theme()\n    shader passive_shader() w=shrink h=fixed_length\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let themer = program.themer(ViewId(1)).unwrap();
        let shader = program.shader(ViewId(2)).unwrap();

        assert_eq!(themer.id, ViewId(1));
        assert_eq!(themer.adapter.function.id, ExternFnId(0));
        assert_eq!(themer.adapter.function.name, "alternate_surface");
        assert_eq!(
            themer.adapter.function.rust_path,
            "crate::backend::alternate_surface"
        );
        assert_eq!(themer.adapter.arguments.len(), 3);
        assert_eq!(
            themer.adapter.arguments[0].mode,
            ResolvedExternViewArgumentMode::Owned
        );
        assert_eq!(
            themer.adapter.arguments[1].mode,
            ResolvedExternViewArgumentMode::Owned
        );
        assert_eq!(
            themer.adapter.arguments[2].ty,
            Type::Option(Box::new(Type::Bool))
        );
        assert_eq!(
            themer.adapter.arguments[2].mode,
            ResolvedExternViewArgumentMode::Owned
        );
        assert_eq!(themer.adapter.output, Type::Bool);
        assert_eq!(
            themer.adapter.route.as_ref().unwrap().source_payloads,
            vec![Type::Bool]
        );

        assert_eq!(shader.id, ViewId(2));
        assert_eq!(shader.adapter.function.id, ExternFnId(1));
        assert_eq!(shader.adapter.function.name, "shader_surface");
        assert_eq!(shader.adapter.arguments.len(), 3);
        assert_eq!(
            shader.adapter.arguments[0].mode,
            ResolvedExternViewArgumentMode::Owned
        );
        assert_eq!(
            shader.adapter.arguments[1].mode,
            ResolvedExternViewArgumentMode::Owned
        );
        assert_eq!(
            shader.adapter.arguments[2].ty,
            Type::Option(Box::new(Type::Bool))
        );
        assert!(matches!(
            shader.width,
            Some(ResolvedContainerLength::FillPortion(3))
        ));
        assert!(matches!(
            shader.height,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert_eq!(
            shader.adapter.route.as_ref().unwrap().source_payloads,
            vec![Type::Bool]
        );

        let passive_themer = program.themer(ViewId(3)).unwrap();
        let passive_shader = program.shader(ViewId(4)).unwrap();
        assert!(passive_themer.adapter.route.is_none());
        assert_eq!(passive_themer.adapter.output, Type::Unit);
        assert!(passive_shader.adapter.route.is_none());
        assert!(matches!(
            passive_shader.width,
            Some(ResolvedContainerLength::Shrink)
        ));
        assert!(matches!(
            passive_shader.height,
            Some(ResolvedContainerLength::FixedLength(_))
        ));

        let generated = crate::codegen::generate(&program, "extern-views-hir.ice").unwrap();
        assert!(generated.contains("crate::backend::alternate_surface("));
        assert!(generated.contains("crate::backend::shader_surface("));
        assert!(!generated.contains("::std::convert::AsRef::as_ref"));
        assert!(!generated.contains("::std::borrow::Borrow::borrow"));
        assert!(generated.contains(".width(::iced::Length::FillPortion(3)).height(64.0 as f32)"));
        assert!(generated.contains(".width(::iced::Shrink).height(self.fixed_length)"));
        assert!(generated.contains("__ExternViewsHirMessage::Themed(__value)"));
        assert!(generated.contains("__ExternViewsHirMessage::Shaded(__value)"));
        assert!(generated.contains("__ExternNoop"));
    }

    #[test]
    fn themer_and_shader_route_component_handlers_and_outputs() {
        let source = format!(
            "app ExternViewRoutes\nextern crate::backend\n  themer alternate_surface(active:bool) -> bool\n  shader shader_surface(active:bool) -> bool\n{THEME}state\n  active = false\ncomponent Adapter(active:bool) -> bool\n  col\n    themer alternate_surface(active) -> emit(_)\n    shader shader_surface(active) -> emit(_)\non changed(next)\nview\n  Adapter active=active -> changed _\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let themer = program.themers().next().unwrap();
        let shader = program.shaders().next().unwrap();
        assert!(matches!(
            themer.adapter.route.as_ref().unwrap().target,
            ResolvedInteractionRouteTarget::OutputCallback {
                output: Type::Bool,
                ..
            }
        ));
        assert!(matches!(
            shader.adapter.route.as_ref().unwrap().target,
            ResolvedInteractionRouteTarget::OutputCallback {
                output: Type::Bool,
                ..
            }
        ));
        let generated = crate::codegen::generate(&program, "extern-view-routes.ice").unwrap();
        assert!(generated.contains("crate::backend::alternate_surface("));
        assert!(generated.contains("crate::backend::shader_surface("));
        assert!(generated.contains("__ExternViewRoutesMessage::Changed(__value)"));
    }

    #[test]
    fn themer_and_shader_post_check_and_post_lowering_raw_poison_is_contained() {
        let source = format!(
            "app ExternViewPoison\nextern crate::backend\n  themer primary_theme(label:str) -> bool\n  themer poisoned_theme(label:str) -> bool\n  shader primary_shader(label:str) -> bool\n  shader poisoned_shader(label:str) -> bool\n{THEME}state\n  label = \"Original\"\non themed(next)\non shaded(next)\nview\n  col\n    themer primary_theme(label) -> themed _\n    shader primary_shader(label) w=32.0 -> shaded _\n"
        );

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Themer { args, .. } = &mut children[0] else {
            panic!("first child must be a themer");
        };
        args[0] = Expr::Str("POISONED_THEMER_ARG".into());
        let ViewNode::Shader { args, width, .. } = &mut children[1] else {
            panic!("second child must be a shader");
        };
        args[0] = Expr::Str("POISONED_SHADER_ARG".into());
        *width = Some(LengthValue::Fixed(Expr::F64(999.0)));
        let mut program = lower(checked).unwrap();
        let expected = crate::codegen::generate(&program, "extern-view-poison.ice").unwrap();
        assert!(expected.contains("self.label"));
        assert!(expected.contains(".width(32.0 as f32)"));
        assert!(!expected.contains("POISONED"));
        assert!(!expected.contains("999.0"));

        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Themer {
            function,
            args,
            route,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be a themer");
        };
        *function = "poisoned_theme".into();
        *args = vec![Expr::Str("POST_LOWER_THEMER".into())];
        *route = None;
        let ViewNode::Shader {
            function,
            args,
            width,
            route,
            ..
        } = &mut children[1]
        else {
            panic!("second child must be a shader");
        };
        *function = "poisoned_shader".into();
        *args = vec![Expr::Str("POST_LOWER_SHADER".into())];
        *width = Some(LengthValue::Fill);
        *route = None;

        let actual = crate::codegen::generate(&program, "extern-view-poison.ice").unwrap();
        assert_eq!(actual, expected);

        let mut static_poison = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut static_poison.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Shader { width, .. } = &mut children[1] else {
            panic!("second child must be a shader");
        };
        *width = Some(LengthValue::Fill);
        let error = lower(static_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn themer_and_shader_reject_cross_owner_and_invalid_identity_corruption() {
        let source = format!(
            "app ExternViewIdentity\nextern crate::backend\n  themer first_theme(active:bool) -> bool\n  themer second_theme(active:bool) -> bool\n  shader first_shader(active:bool) -> bool\n  shader second_shader(active:bool) -> bool\n{THEME}state\n  active = false\non first(next)\non second(next)\nview\n  col\n    themer first_theme(active) -> first _\n    themer second_theme(active) -> second _\n    shader first_shader(active) -> first _\n    shader second_shader(active) -> second _\n"
        );
        let first_themer = ViewId(1);
        let second_themer = ViewId(2);
        let first_shader = ViewId(3);
        let second_shader = ViewId(4);

        let mut swapped_themer = analyze(&source).unwrap();
        swapped_themer
            .facts
            .transplant_interaction_option_expression(first_themer, 0, second_themer, 0);
        let error = lower(swapped_themer).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("argument 0 contract diverged"));

        let mut swapped_shader = analyze(&source).unwrap();
        swapped_shader
            .facts
            .transplant_interaction_option_expression(first_shader, 0, second_shader, 0);
        let error = lower(swapped_shader).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("argument 0 contract diverged"));

        let mut corrupt_themer_function = analyze(&source).unwrap();
        corrupt_themer_function
            .facts
            .corrupt_themer_function(first_themer, ExternFnId(1));
        let error = lower(corrupt_themer_function).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("declaration contract diverged"));

        let mut corrupt_shader_function = analyze(&source).unwrap();
        corrupt_shader_function
            .facts
            .corrupt_shader_function(first_shader, ExternFnId(3));
        let error = lower(corrupt_shader_function).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("declaration contract diverged"));

        let mut corrupt_themer_output = analyze(&source).unwrap();
        corrupt_themer_output
            .facts
            .corrupt_themer_output(first_themer, Type::Unit);
        let error = lower(corrupt_themer_output).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("declaration contract diverged"));

        let mut corrupt_shader_output = analyze(&source).unwrap();
        corrupt_shader_output
            .facts
            .corrupt_shader_output(first_shader, Type::Unit);
        let error = lower(corrupt_shader_output).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("declaration contract diverged"));

        let mut swapped_routes = analyze(&source).unwrap();
        swapped_routes
            .facts
            .swap_interaction_routes(first_themer, second_themer);
        let error = lower(swapped_routes).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));

        let mut corrupt_themer_id = analyze(&source).unwrap();
        corrupt_themer_id
            .facts
            .corrupt_themer_id(first_themer, u32::MAX);
        let error = lower(corrupt_themer_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));

        let mut corrupt_shader_id = analyze(&source).unwrap();
        corrupt_shader_id
            .facts
            .corrupt_shader_id(first_shader, u32::MAX);
        let error = lower(corrupt_shader_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));
    }

    #[test]
    fn themer_and_shader_reject_expression_count_and_last_graph_corruption() {
        let source = format!(
            "app ExternViewExpressionCardinality\nextern crate::backend\n  themer first_theme(active:bool) -> bool\n  shader first_shader(active:bool) -> bool\n{THEME}state\n  active = false\non first(next)\non second(next)\nview\n  col\n    themer first_theme(active) -> first !active\n    shader first_shader(active) -> second !active\n"
        );

        for view in [ViewId(1), ViewId(2)] {
            let mut decreased = analyze(&source).unwrap();
            decreased
                .facts
                .corrupt_interaction_expression_count(view, 1);
            let error = lower(decreased).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains("cardinality diverged"));

            let mut increased = analyze(&source).unwrap();
            increased
                .facts
                .corrupt_interaction_expression_count(view, 3);
            let error = lower(increased).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains("cardinality diverged"));

            let mut invalid_last_graph = analyze(&source).unwrap();
            invalid_last_graph.facts.corrupt_expression_first_child(
                CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: view,
                    index: 1,
                }),
                u32::MAX,
            );
            let error = lower(invalid_last_graph).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains("descendant ID"));
        }
    }

    #[test]
    fn imported_themer_and_shader_keep_origins_markers_and_e196_paths() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-extern-view-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("native.ice");
        fs::write(
            &root,
            format!(
                "app ImportedExternViews\nuse \"native.ice\"\n{THEME}state\n  label = \"Imported\"\non changed(next)\nview\n  ImportedAdapters label=label -> changed _\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "extern crate::backend\n  themer imported_theme(label:str) -> bool\n  shader imported_shader(label:str) -> bool\ncomponent ImportedAdapters(label:str) -> bool\n  col\n    themer imported_theme(label) -> emit(_)\n    shader imported_shader(label) w=fill h=32.0 -> emit(_)\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let themer = program.themers().next().unwrap();
        let shader = program.shaders().next().unwrap();
        let themer_id = themer.id;
        for (origin, line) in [(themer.origin, 6), (shader.origin, 7)] {
            let origin = program.origin(origin);
            assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
            assert_eq!(origin.line, line);
        }
        assert_eq!(
            program
                .origin(themer.adapter.function.declaration_origin)
                .path
                .as_deref(),
            Some(imported.as_path())
        );
        assert_eq!(
            program
                .origin(themer.adapter.function.declaration_origin)
                .line,
            2
        );
        assert_eq!(
            program
                .origin(shader.adapter.function.declaration_origin)
                .path
                .as_deref(),
            Some(imported.as_path())
        );
        assert_eq!(
            program
                .origin(shader.adapter.function.declaration_origin)
                .line,
            3
        );
        assert_eq!(
            program.origin(themer.adapter.arguments[0].origin).parent,
            Some(themer.origin)
        );
        assert_eq!(
            program
                .origin(themer.adapter.route.as_ref().unwrap().origin)
                .parent,
            Some(themer.origin)
        );
        assert_eq!(
            program.origin(shader.adapter.arguments[0].origin).parent,
            Some(shader.origin)
        );
        assert_eq!(
            program
                .origin(shader.adapter.route.as_ref().unwrap().origin)
                .parent,
            Some(shader.origin)
        );

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_import}")));
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));
        assert!(generated.contains(&format!("// __ICE_SOURCE 6 1 {encoded_import}")));
        assert!(generated.contains(&format!("// __ICE_SOURCE 7 1 {encoded_import}")));

        program.themers.get_mut(&themer_id).unwrap().id = ViewId(u32::MAX);
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 6);

        let mut corrupt_checked = analyze_file(&root).unwrap();
        let ViewNode::Layout { children, .. } = &corrupt_checked.document.components[0].root else {
            panic!("imported component root must be a layout");
        };
        let themer_id = corrupt_checked
            .declarations
            .view_id(children[0].span())
            .unwrap();
        corrupt_checked
            .facts
            .corrupt_themer_function(themer_id, ExternFnId(u32::MAX));
        let error = lower(corrupt_checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 6);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "large normalized Themer and Shader lowering and emission performance contract"]
    fn performance_contract_four_thousand_themers_and_shaders_lower_and_emit_under_two_seconds() {
        const EACH: usize = 2_000;
        let mut source = format!(
            "app ExternViewScale\nextern crate::backend\n  themer native_theme(index:i64) -> unit\n  shader native_shader(index:i64) -> unit\n{THEME}state\n  index = 7\nview\n  col\n"
        );
        for _ in 0..EACH {
            writeln!(source, "    themer native_theme(index)").unwrap();
            writeln!(source, "    shader native_shader(index) w=fill h=32.0").unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "extern-view-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.themers.len(), EACH);
        assert_eq!(program.shaders.len(), EACH);
        assert_eq!(generated.matches("::iced::widget::themer(").count(), EACH);
        assert_eq!(
            generated.matches("::iced::widget::Shader::new(").count(),
            EACH + 1
        );
        eprintln!("4k normalized Themer/Shader nodes lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized Themer/Shader nodes lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized extern-component declaration probe performance contract"]
    fn performance_contract_four_thousand_unused_extern_component_probes_under_two_seconds() {
        const DECLARATIONS: usize = 4_000;
        let mut source = "app ExternProbeScale\nextern crate::backend\n".to_owned();
        for index in 0..DECLARATIONS {
            writeln!(
                source,
                "  component native_surface_{index}(label:&str, index:i64) -> unit"
            )
            .unwrap();
        }
        source.push_str(&format!("{THEME}view\n  text \"ready\"\n"));
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "extern-probe-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.extern_component_declarations.len(), DECLARATIONS);
        assert_eq!(
            generated
                .matches("fn __ui_lang_check_component_native_surface_")
                .count(),
            DECLARATIONS
        );
        eprintln!("4k normalized unused extern component probes emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized unused extern component probes emitted in {elapsed:?}"
        );
    }

    #[test]
    fn normalizes_the_complete_markdown_contract() {
        let source = format!(
            "app MarkdownHir\nextern crate::backend\n  markdown-viewer docs_viewer(prefix:str, generation:i64) -> str\nfont ui family=sans\n{THEME}state\n  docs:markdown = \"# Docs\"\n  prefix = \"guide\"\n  generation = 7\non opened(url)\nview\n  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 gap=12.0 viewer=docs_viewer(prefix, generation) -> opened _\n    style font=ui inline-code-bg=linear(1.57, bg@0.0, primary/25@1.0) inline-code-fg=fg inline-code-font=mono code-block-font=mono link=primary inline-code-p=2.0 inline-code-px=3.0 inline-code-py=4.0 inline-code-pt=5.0 inline-code-pr=6.0 inline-code-pb=7.0 inline-code-pl=8.0 inline-code-border=danger inline-code-border-w=1.0 inline-code-r=4.0 inline-code-r-tl=1.0 inline-code-r-tr=2.0 inline-code-r-br=3.0 inline-code-r-bl=4.0\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let markdown = program.markdown(ViewId(0)).unwrap();

        assert_eq!(markdown.id, ViewId(0));
        assert_eq!(
            markdown.content.id,
            CheckedValueRef::AppState(AppStateId(0))
        );
        assert_eq!(markdown.content.name, "docs");
        assert!(markdown.text_size.is_some());
        assert!(markdown.h1_size.is_some());
        assert!(markdown.h2_size.is_some());
        assert!(markdown.h3_size.is_some());
        assert!(markdown.h4_size.is_some());
        assert!(markdown.h5_size.is_some());
        assert!(markdown.h6_size.is_some());
        assert!(markdown.code_size.is_some());
        assert!(markdown.spacing.is_some());
        let viewer = markdown.viewer.as_ref().unwrap();
        assert_eq!(program.extern_function(viewer.function).name, "docs_viewer");
        assert_eq!(viewer.arguments.len(), 2);
        assert_eq!(viewer.borrowed, vec![false, false]);
        assert_eq!(viewer.output, Type::Str);
        assert!(matches!(
            markdown.style.font,
            Some(ResolvedTextFont::Named(_))
        ));
        assert!(matches!(
            markdown.style.inline_code_background,
            Some(ResolvedContainerBackground::Linear { ref stops, .. }) if stops.len() == 2
        ));
        assert!(markdown.style.inline_code_color.is_some());
        assert!(markdown.style.inline_code_font.is_some());
        assert!(markdown.style.code_block_font.is_some());
        assert!(markdown.style.link_color.is_some());
        assert!(markdown.style.inline_code_padding.all.is_some());
        assert!(markdown.style.inline_code_padding.left.is_some());
        assert!(markdown.style.inline_code_border_color.is_some());
        assert!(markdown.style.inline_code_border_width.is_some());
        assert!(markdown.style.inline_code_radius.all.is_some());
        assert!(markdown.style.inline_code_radius.bottom_left.is_some());
        assert!(matches!(
            markdown.link.target,
            ResolvedInteractionRouteTarget::TargetHandler(_)
        ));
        assert_eq!(markdown.link.source_payloads, vec![Type::Str]);
    }

    #[test]
    fn markdown_routes_direct_component_handlers_and_component_outputs() {
        let direct = format!(
            "app MarkdownDirect\n{THEME}state\n  docs:markdown = \"# Docs\"\non opened(url)\nview\n  markdown docs -> opened _\n"
        );
        let program = lower(analyze(&direct).unwrap()).unwrap();
        assert!(matches!(
            program.markdown(ViewId(0)).unwrap().link.target,
            ResolvedInteractionRouteTarget::TargetHandler(_)
        ));

        let component = format!(
            "app MarkdownComponent\n{THEME}state\n  docs:markdown = \"# Docs\"\ncomponent Viewer(bind docs:markdown)\n  on opened(url)\n  markdown docs -> opened _\nview\n  Viewer docs<->docs\n"
        );
        let program = lower(analyze(&component).unwrap()).unwrap();
        let markdown = program.markdowns.values().next().unwrap();
        assert!(matches!(
            markdown.content.id,
            CheckedValueRef::ComponentParam(_)
        ));
        assert!(matches!(
            markdown.link.target,
            ResolvedInteractionRouteTarget::TargetHandler(_)
        ));

        let output = format!(
            "app MarkdownOutput\n{THEME}state\n  docs:markdown = \"# Docs\"\ncomponent Viewer(bind docs:markdown) -> str\n  markdown docs -> emit(_)\non opened(url)\nview\n  Viewer docs<->docs -> opened _\n"
        );
        let program = lower(analyze(&output).unwrap()).unwrap();
        let markdown = program.markdowns.values().next().unwrap();
        assert!(matches!(
            markdown.link.target,
            ResolvedInteractionRouteTarget::OutputCallback {
                output: Type::Str,
                ..
            }
        ));
        let generated = crate::codegen::generate(&program, "markdown-output.ice").unwrap();
        assert!(generated.contains("::iced::widget::markdown::view("));
        assert!(generated.contains("__MarkdownOutputMessage::Opened(__value)"));
    }

    #[test]
    fn markdown_codegen_ignores_every_raw_semantic_field_after_lowering() {
        let source = format!(
            "app MarkdownPoison\nextern crate::backend\n  markdown-viewer primary_viewer(prefix:str) -> str\n  markdown-viewer poisoned_viewer(prefix:str) -> str\nfont ui family=sans\n{THEME}state\n  docs:markdown = \"# Docs\"\n  other:markdown = \"# Other\"\n  prefix = \"guide\"\non opened(url)\nview\n  markdown docs text-size=16.0 viewer=primary_viewer(prefix) -> opened _\n    style font=ui inline-code-bg=primary inline-code-fg=fg link=primary inline-code-p=2.0 inline-code-border=danger inline-code-border-w=1.0 inline-code-r=4.0\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "markdown-poison.ice").unwrap();
        let ViewNode::Markdown {
            content,
            options,
            route,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be markdown");
        };
        *content = "other".into();
        options.text_size = Some(Expr::F64(999.0));
        options.viewer.as_mut().unwrap().function = "poisoned_viewer".into();
        options.viewer.as_mut().unwrap().args = vec![Expr::Str("POISONED".into())];
        options.style.font = Some(FontPreset::Monospace);
        options.style.inline_code_background = Some(BackgroundValue::Color("danger".into()));
        options.style.inline_code_color = Some("danger".into());
        options.style.link_color = Some("danger".into());
        options.style.inline_code_padding = PaddingOptions::default();
        options.style.inline_code_border_color = Some("fg".into());
        options.style.inline_code_border_width = Some(Expr::F64(99.0));
        options.style.inline_code_radius = Some(Expr::F64(99.0));
        route.handler = "poisoned".into();
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(1, 3);

        let actual = crate::codegen::generate(&program, "markdown-poison.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("POISONED"));
    }

    #[test]
    fn markdown_lowering_rejects_cross_owner_state_viewer_route_origin_and_id_corruption() {
        let source = format!(
            "app MarkdownIdentity\nextern crate::backend\n  markdown-viewer first_viewer(prefix:str) -> str\n  markdown-viewer second_viewer(prefix:str) -> str\n{THEME}state\n  first:markdown = \"# First\"\n  second:markdown = \"# Second\"\n  prefix = \"docs\"\non first_opened(url)\non second_opened(url)\nview\n  col\n    markdown first text-size=16.0 viewer=first_viewer(prefix) -> first_opened _\n      style inline-code-p=2.0\n    markdown second text-size=16.0 viewer=second_viewer(prefix) -> second_opened _\n      style inline-code-p=2.0\n"
        );
        let first = ViewId(1);
        let second = ViewId(2);

        let mut swapped_expressions = analyze(&source).unwrap();
        swapped_expressions
            .facts
            .transplant_interaction_option_expression(first, 0, second, 0);
        let error = lower(swapped_expressions).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut corrupt_content = analyze(&source).unwrap();
        corrupt_content
            .facts
            .corrupt_markdown_content(first, CheckedValueRef::AppState(AppStateId(1)));
        let error = lower(corrupt_content).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("content identity"));

        let mut corrupt_viewer = analyze(&source).unwrap();
        corrupt_viewer
            .facts
            .corrupt_markdown_viewer(first, ExternFnId(1));
        let error = lower(corrupt_viewer).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("viewer contract diverged"));

        let mut corrupt_output = analyze(&source).unwrap();
        corrupt_output
            .facts
            .corrupt_markdown_viewer_output(first, Type::Bool);
        let error = lower(corrupt_output).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("viewer contract diverged"));

        let mut swapped_routes = analyze(&source).unwrap();
        swapped_routes.facts.swap_interaction_routes(first, second);
        let error = lower(swapped_routes).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));

        let mut corrupt_origin = analyze(&source).unwrap();
        corrupt_origin
            .facts
            .corrupt_markdown_style_origin(first, u32::MAX);
        let error = lower(corrupt_origin).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("style origin"));

        let mut corrupt_id = analyze(&source).unwrap();
        corrupt_id.facts.corrupt_markdown_id(first, u32::MAX);
        let error = lower(corrupt_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));
    }

    #[test]
    fn imported_markdown_keeps_content_expression_style_route_and_source_map_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-markdown-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("docs.ice");
        fs::write(
            &root,
            format!(
                "app ImportedMarkdownApp\nuse \"docs.ice\"\nfont ui family=sans\n{THEME}state\n  docs:markdown = \"# Imported\"\nview\n  ImportedDocs docs<->docs\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedDocs(bind docs:markdown)\n  on open(url)\n  markdown docs text-size=16.0 -> open _\n    style font=ui inline-code-bg=linear(1.0, bg@0.0, primary@1.0) inline-code-p=2.0 link=primary\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let markdown = program.markdowns.values().next().unwrap();
        let widget_origin = program.origin(markdown.origin);
        assert_eq!(widget_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(widget_origin.line, 3);
        assert!(matches!(
            markdown.content.id,
            CheckedValueRef::ComponentParam(_)
        ));
        let style_origin = program.origin(markdown.style.origin.unwrap());
        assert_eq!(style_origin.parent, Some(markdown.origin));
        assert_eq!(style_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(style_origin.line, 4);
        let interaction = program.checked_facts().interaction(markdown.id).unwrap();
        let metric_origin = program.origin(
            program
                .checked_facts()
                .expression_use(interaction.option_expressions[0])
                .origin,
        );
        assert_eq!(metric_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(metric_origin.line, 3);
        let style_metric_origin = program.origin(
            program
                .checked_facts()
                .expression_use(interaction.option_expressions[1])
                .origin,
        );
        assert_eq!(
            style_metric_origin.path.as_deref(),
            Some(imported.as_path())
        );
        assert_eq!(style_metric_origin.line, 4);
        let route_origin = program.origin(markdown.link.origin);
        assert_eq!(route_origin.parent, Some(markdown.origin));
        assert_eq!(route_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(route_origin.line, 3);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));

        program.markdowns.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "large normalized markdown lowering and emission performance contract"]
    fn performance_contract_four_thousand_markdown_views_lower_and_emit_under_two_seconds() {
        const MARKDOWN_VIEWS: usize = 4_000;
        let mut source = format!(
            "app MarkdownScale\n{THEME}state\n  docs:markdown = \"# Docs\"\non opened(url)\nview\n  col\n"
        );
        for _ in 0..MARKDOWN_VIEWS {
            writeln!(
                source,
                "    markdown docs text-size=16.0 gap=8.0 -> opened _"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "markdown-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.markdowns.len(), MARKDOWN_VIEWS);
        assert_eq!(
            generated.matches("::iced::widget::markdown::view(").count(),
            MARKDOWN_VIEWS
        );
        eprintln!("4k normalized markdown views lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized markdown views lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    fn normalizes_the_complete_text_editor_contract() {
        let source = r#"app EditorHir
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-action track_edits()
  editor-style editor_surface(readonly:bool)
font ui family=sans
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
  body:editor = ""
  spare:editor = ""
  locked = false
  language = "rs"
on command(value)
view
  editor #body <-> body hint="Write" w=640.0 h=fill min-h=80.0 max-h=240.0 size=14.0 line-h-px=18.0 p=8.0 wrap=word-or-glyph font=ui disabled=locked highlighter=editor_highlight(language) key-binding=editor_keys(locked) action=track_edits() style=editor_surface(locked) -> command _
    active bg=bg border=fg border-w=1.0 r=4.0 placeholder=danger value=fg selection=primary
    hovered bg=bg border=primary border-w=1.0 r=6.0 placeholder=danger value=fg selection=primary
    focused bg=bg border=primary border-w=1.0 r=8.0
    focused-hovered bg=bg border=fg border-w=1.0 r=10.0
    disabled bg=bg border=danger border-w=1.0 r=12.0 value=danger
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        let editor = program.text_editor(ViewId(0)).unwrap();

        assert!(matches!(
            editor.binding,
            WritableStateRef::App { ref name, .. } if name == "body"
        ));
        assert_eq!(editor.placeholder.as_deref(), Some("Write"));
        assert!(editor.disabled.is_some());
        assert!(editor.width.is_some());
        assert!(matches!(editor.height, Some(ResolvedContainerLength::Fill)));
        assert!(editor.min_height.is_some());
        assert!(editor.max_height.is_some());
        assert!(editor.size.is_some());
        assert!(matches!(
            editor.line_height,
            Some(ResolvedTextLineHeight::Absolute(_))
        ));
        assert!(editor.padding.is_some());
        assert_eq!(editor.wrapping, Some(ResolvedEditorWrapping::WordOrGlyph));
        assert!(matches!(editor.font, Some(ResolvedTextFont::Named(_))));
        assert!(editor.highlight.is_none());
        assert!(editor.highlight_theme.is_none());
        assert!(editor.highlighter.is_some());
        assert!(editor.key_binding.is_some());
        assert!(editor.action.is_some());
        assert!(editor.custom_style.is_some());
        assert!(editor.styles.active.is_some());
        assert!(editor.styles.hovered.is_some());
        assert!(editor.styles.focused.is_some());
        assert!(editor.styles.focused_hovered.is_some());
        assert!(editor.styles.disabled.is_some());
        assert_eq!(program.controlled_editor_bindings().unwrap().len(), 1);
        assert_eq!(
            program.controlled_editor_bindings().unwrap()[0].action,
            editor.action.as_ref().map(|action| action.function)
        );
        let route = &editor.key_binding.as_ref().unwrap().route;
        assert_eq!(program.origin(route.origin).parent, Some(editor.origin));
        for status in [
            &editor.styles.active,
            &editor.styles.hovered,
            &editor.styles.focused,
            &editor.styles.focused_hovered,
            &editor.styles.disabled,
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(program.origin(status.origin).parent, Some(editor.origin));
        }
    }

    #[test]
    fn normalizes_builtin_text_editor_highlighting() {
        let source = format!(
            "app BuiltinEditorHighlight\n{THEME}state\n  body:editor = \"\"\nview\n  editor <-> body highlight=\"rs\" highlight-theme=inspired-github\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let editor = program.text_editor(ViewId(0)).unwrap();
        assert_eq!(editor.highlight.as_deref(), Some("rs"));
        assert_eq!(
            editor.highlight_theme,
            Some(ResolvedHighlightTheme::InspiredGithub)
        );
        assert!(editor.highlighter.is_none());
    }

    #[test]
    fn imported_text_editor_keeps_widget_and_status_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-text-editor-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("editor.ice");
        fs::write(
            &root,
            format!(
                "app ImportedEditorApp\nuse \"editor.ice\"\n{THEME}state\n  body:editor = \"\"\nview\n  ImportedEditor value<->body\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedEditor(bind value:editor)\n  editor <-> value hint=\"Imported\" highlight=\"rs\"\n    active bg=bg border=fg border-w=1.0\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let editor = program.text_editors.values().next().unwrap();
        let editor_origin = program.origin(editor.origin);
        assert_eq!(editor_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(editor_origin.line, 2);
        let status = editor.styles.active.as_ref().unwrap();
        let status_origin = program.origin(status.origin);
        assert_eq!(status_origin.parent, Some(editor.origin));
        assert_eq!(status_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(status_origin.line, 3);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_import}")));

        program.text_editors.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 2);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn text_editor_lowering_rejects_same_arena_binding_identity_swaps() {
        let source = format!(
            "app EditorBindingIdentity\n{THEME}state\n  first:editor = \"\"\n  second:editor = \"\"\nview\n  editor <-> first\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_text_editor_binding(ViewId(0), CheckedValueRef::AppState(AppStateId(1)));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("editor binding identity diverged"));
    }

    #[test]
    fn text_editor_codegen_ignores_raw_contract_and_validates_controlled_identity() {
        let source = format!(
            "app LoweredEditor\n{THEME}state\n  body:editor = \"\"\n  spare:editor = \"\"\n  locked = false\nview\n  editor #body <-> body hint=\"Write\" w=640.0 h=fill p=8.0 disabled=locked\n    active bg=bg border=fg border-w=1.0 r=4.0\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-editor.ice").unwrap();

        let ViewNode::TextEditor {
            binding,
            disabled,
            options,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be a text editor");
        };
        *binding = "spare".into();
        *disabled = Some(Expr::Bool(true));
        *options = TextEditorOptions::default();
        let actual = crate::codegen::generate(&program, "lowered-editor.ice").unwrap();
        assert_eq!(actual, expected);

        program.controlled_editors[0].state = AppStateId(1);
        let error = crate::codegen::generate(&program, "lowered-editor.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("controlled editor binding"));

        program.controlled_editors[0].state = AppStateId(u32::MAX);
        let error = crate::codegen::generate(&program, "lowered-editor.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("controlled editor binding"));
    }

    #[test]
    fn text_editor_codegen_rejects_same_kind_action_identity_swaps() {
        let source = format!(
            "app EditorActionIdentity\nextern crate::backend\n  editor-action first()\n  editor-action second()\n{THEME}state\n  body:editor = \"\"\nview\n  editor <-> body action=first()\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        program.controlled_editors[0].action = Some(ExternFnId(1));

        let error = crate::codegen::generate(&program, "editor-action-identity.ice").unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("editor action diverges"));
    }

    #[test]
    fn text_editor_rejects_non_cloneable_component_local_state() {
        let source = format!(
            "app ComponentEditorState\n{THEME}component EditorPanel()\n  state\n    body:editor = \"\"\n  editor <-> body\nview\n  EditorPanel #panel\n"
        );

        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E103");
        assert!(
            error
                .message
                .contains("component state supports ordinary cloneable values only")
        );
    }

    #[test]
    fn malformed_checked_text_editor_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidEditorFacts\n{THEME}state\n  body:editor = \"\"\n  locked = false\nview\n  editor <-> body disabled=locked\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            }),
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    #[ignore = "large normalized text editor lowering and emission performance contract"]
    fn performance_contract_four_thousand_text_editors_lower_and_emit_under_two_seconds() {
        const EDITORS: usize = 4_000;
        let mut source = format!(
            "app EditorScale\n{THEME}state\n  body:editor = \"\"\n  locked = false\nview\n  col\n"
        );
        for index in 0..EDITORS {
            writeln!(
                source,
                "    editor #editor_{index} <-> body hint=\"Write\" w=640.0 h=fill p=8.0 disabled=locked"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "editor-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.text_editors.len(), EDITORS);
        // A disabled editor emits its enabled and disabled widget branches.
        assert_eq!(
            generated
                .matches("::iced::widget::text_editor(&self.body)")
                .count(),
            EDITORS * 2
        );
        eprintln!("4k normalized text editors lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized text editors lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "distinct controlled editor lookup must remain linear"]
    fn performance_contract_four_thousand_distinct_text_editors_emit_under_two_seconds() {
        const EDITORS: usize = 4_000;
        let mut source = format!("app DistinctEditorScale\n{THEME}state\n");
        for index in 0..EDITORS {
            writeln!(source, "  body_{index}:editor = \"\"").unwrap();
        }
        source.push_str("view\n  col\n");
        for index in 0..EDITORS {
            writeln!(source, "    editor #editor_{index} <-> body_{index}").unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "distinct-editor-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.text_editors.len(), EDITORS);
        assert_eq!(program.controlled_editors.len(), EDITORS);
        assert_eq!(program.controlled_editors_by_name.len(), EDITORS);
        assert_eq!(
            generated
                .matches("::iced::widget::text_editor(&self.body_")
                .count(),
            EDITORS
        );
        eprintln!("4k distinct normalized text editors lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k distinct normalized text editors lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    fn sensor_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedSensor\n{THEME}state\n  active = true\non shown(width, height)\non hidden\nview\n  sensor show=shown hide=hidden key=active anticipate=24.0 delay=50\n    text \"Observed\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-sensor.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::Sensor { options, .. } = &mut changed_expressions.document.view else {
            panic!("fixture root must be a sensor");
        };
        options.key = Some(Expr::Bool(false));
        options.anticipate = Some(Expr::F64(999.0));
        options.delay_ms = Some(Expr::I64(999));
        let actual =
            crate::codegen::generate(&lower(changed_expressions).unwrap(), "checked-sensor.ice")
                .unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Sensor { options, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be a sensor");
        };
        options.hide = None;
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn sensor_codegen_ignores_raw_options_and_routes_after_lowering() {
        let source = format!(
            "app LoweredSensor\n{THEME}state\n  active = true\non shown(width, height)\non hidden\nview\n  sensor show=shown hide=hidden key=active anticipate=24.0 delay=50\n    text \"Observed\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-sensor.ice").unwrap();

        let ViewNode::Sensor { options, .. } = &mut program.document.view else {
            panic!("fixture root must be a sensor");
        };
        options.key = Some(Expr::Bool(false));
        options.anticipate = Some(Expr::F64(999.0));
        options.delay_ms = Some(Expr::I64(999));
        options.show.as_mut().unwrap().handler = "poisoned".into();
        options.hide.as_mut().unwrap().handler = "poisoned".into();

        let actual = crate::codegen::generate(&program, "lowered-sensor.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("Poisoned"));
    }

    #[test]
    fn normalizes_float_geometry_style_color_and_expression_owners() {
        let source = format!(
            "app FloatHir\n{THEME}state\n  shift = 2.0\nview\n  float scale=1.1 x=(viewport_x + shift) y=(original_y - shift) shadow=primary/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 r=8.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0\n    text \"Floating\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let float = program.float(ViewId(0)).unwrap();

        assert_eq!(float.id, ViewId(0));
        assert_eq!(float.geometry.len(), 8);
        assert_eq!(
            float.shadow_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(program.theme().native_tokens.primary),
                opacity: Some(50),
            })
        );
        for expression in [float.scale, float.x, float.y] {
            let expression = program.checked_facts().expression_use(expression);
            assert_eq!(expression.source, Type::F64);
            assert_eq!(expression.destination, Type::F64);
        }
        assert!(float.shadow_x.is_some());
        assert!(float.shadow_y.is_some());
        assert!(float.shadow_blur.is_some());
        assert!(float.radius.all.is_some());
        assert!(float.radius.top_left.is_some());
        assert_eq!(program.origin(float.origin).line, 15);
    }

    #[test]
    fn float_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedFloat\n{THEME}state\n  shift = 2.0\nview\n  float scale=1.1 x=(viewport_x + shift) y=(original_y - shift) shadow=primary/50 shadow-x=-1.0 r=8.0\n    text \"Floating\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-float.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::Float {
            scale, x, y, style, ..
        } = &mut changed_expressions.document.view
        else {
            panic!("fixture root must be a float");
        };
        *scale = Expr::F64(99.0);
        *x = Expr::F64(99.0);
        *y = Expr::F64(99.0);
        style.shadow_x = Some(Expr::F64(99.0));
        let actual =
            crate::codegen::generate(&lower(changed_expressions).unwrap(), "checked-float.ice")
                .unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Float { style, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be a float");
        };
        style.shadow_color = Some("danger".into());
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn float_codegen_ignores_raw_options_and_theme_order_after_lowering() {
        let source = format!(
            "app LoweredFloat\n{THEME}state\n  shift = 2.0\nview\n  float scale=1.1 x=(viewport_x + shift) y=(original_y - shift) shadow=primary/50 shadow-x=-1.0 r=8.0\n    text \"Floating\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-float.ice").unwrap();

        let ViewNode::Float {
            scale, x, y, style, ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be a float");
        };
        *scale = Expr::F64(99.0);
        *x = Expr::F64(99.0);
        *y = Expr::F64(99.0);
        style.shadow_color = Some("danger".into());
        style.shadow_x = Some(Expr::F64(99.0));
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(2, 3);

        let actual = crate::codegen::generate(&program, "lowered-float.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_pin_positions_dimensions_and_expression_owners() {
        let source = format!(
            "app PinHir\n{THEME}state\n  offset = 12.0\n  height = 80.0\nview\n  pin w=fill h=height x=offset y=8.0\n    text \"Pinned\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let pin = program.pin(ViewId(0)).unwrap();

        assert_eq!(pin.id, ViewId(0));
        assert_eq!(pin.width, Some(ResolvedPinLength::Fill));
        assert!(matches!(pin.height, Some(ResolvedPinLength::FixedF64(_))));
        for expression in [pin.x, pin.y] {
            let expression = program.checked_facts().expression_use(expression);
            assert_eq!(expression.source, Type::F64);
            assert_eq!(expression.destination, Type::F64);
        }
        assert_eq!(program.origin(pin.origin).line, 16);
    }

    #[test]
    fn pin_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedPin\n{THEME}state\n  offset = 12.0\n  height = 80.0\nview\n  pin w=fill h=height x=offset y=8.0\n    text \"Pinned\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-pin.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::Pin { height, x, y, .. } = &mut changed_expressions.document.view else {
            panic!("fixture root must be a pin");
        };
        *x = Expr::F64(99.0);
        *y = Expr::F64(99.0);
        let Some(LengthValue::Fixed(height)) = height else {
            panic!("fixture height must be fixed");
        };
        *height = Expr::F64(99.0);
        let actual =
            crate::codegen::generate(&lower(changed_expressions).unwrap(), "checked-pin.ice")
                .unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Pin { width, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be a pin");
        };
        *width = Some(LengthValue::Shrink);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn pin_codegen_ignores_raw_options_after_lowering() {
        let source = format!(
            "app LoweredPin\n{THEME}state\n  offset = 12.0\n  height = 80.0\nview\n  pin w=fill h=height x=offset y=8.0\n    text \"Pinned\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-pin.ice").unwrap();

        let ViewNode::Pin {
            width,
            height,
            x,
            y,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be a pin");
        };
        *width = Some(LengthValue::Shrink);
        *height = None;
        *x = Expr::F64(99.0);
        *y = Expr::F64(99.0);

        let actual = crate::codegen::generate(&program, "lowered-pin.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_responsive_modes_dimensions_and_size_locals() {
        let breakpoint_source = format!(
            "app BreakpointHir\n{THEME}state\n  breakpoint = 600.0\n  height = 40.0\n  native_width:length = length.fill()\nview\n  responsive at=breakpoint w=native_width h=height\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        let breakpoint_program = lower(analyze(&breakpoint_source).unwrap()).unwrap();
        let responsive = breakpoint_program.responsive(ViewId(0)).unwrap();
        assert_eq!(responsive.id, ViewId(0));
        assert!(matches!(
            responsive.width,
            Some(ResolvedResponsiveLength::FixedLength(_))
        ));
        assert!(matches!(
            responsive.height,
            Some(ResolvedResponsiveLength::FixedF64(_))
        ));
        let ResolvedResponsiveKind::Breakpoint { breakpoint } = responsive.kind else {
            panic!("root must be breakpoint responsive");
        };
        assert_eq!(
            breakpoint_program
                .checked_facts()
                .expression_use(breakpoint)
                .source,
            Type::F64
        );

        let size_source = format!(
            "app SizeHir\n{THEME}view\n  responsive size=(available_width, available_height) w=fill h=fill\n    text available_width\n"
        );
        let size_program = lower(analyze(&size_source).unwrap()).unwrap();
        let responsive = size_program.responsive(ViewId(0)).unwrap();
        let ResolvedResponsiveKind::Size { width, height } = &responsive.kind else {
            panic!("root must be size responsive");
        };
        assert_eq!(width.name, "available_width");
        assert_eq!(height.name, "available_height");
        assert_eq!(
            size_program.checked_facts().local(width.local).ty,
            Type::F64
        );
        assert_eq!(
            size_program.checked_facts().local(height.local).ty,
            Type::F64
        );
    }

    #[test]
    fn responsive_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedResponsive\n{THEME}state\n  breakpoint = 600.0\n  height = 40.0\nview\n  responsive at=breakpoint w=fill h=height\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-responsive.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::Responsive {
            content, height, ..
        } = &mut changed_expressions.document.view
        else {
            panic!("fixture root must be responsive");
        };
        let ResponsiveContent::Breakpoint { breakpoint, .. } = content else {
            panic!("fixture must use a breakpoint");
        };
        *breakpoint = Expr::F64(99.0);
        let Some(LengthValue::Fixed(height)) = height else {
            panic!("fixture height must be fixed");
        };
        *height = Expr::F64(99.0);
        let actual = crate::codegen::generate(
            &lower(changed_expressions).unwrap(),
            "checked-responsive.ice",
        )
        .unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Responsive { width, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be responsive");
        };
        *width = Some(LengthValue::Shrink);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn responsive_codegen_ignores_raw_options_after_lowering() {
        let source = format!(
            "app LoweredResponsive\n{THEME}state\n  breakpoint = 600.0\n  height = 40.0\nview\n  responsive at=breakpoint w=fill h=height\n    text \"Narrow\"\n    text \"Wide\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-responsive.ice").unwrap();

        let ViewNode::Responsive {
            content,
            width,
            height,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be responsive");
        };
        let ResponsiveContent::Breakpoint { breakpoint, .. } = content else {
            panic!("fixture must use a breakpoint");
        };
        *breakpoint = Expr::F64(99.0);
        *width = Some(LengthValue::Shrink);
        *height = None;

        let actual = crate::codegen::generate(&program, "lowered-responsive.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_lazy_dependency_binding_type_and_owner() {
        let source = format!(
            "app LazyHir\n{THEME}state\n  title = \"Hello\"\nview\n  lazy title as cached\n    text cached\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let lazy = program.lazy_view(ViewId(0)).unwrap();
        assert_eq!(lazy.id, ViewId(0));
        assert_eq!(lazy.binding.name, "cached");
        assert_eq!(lazy.binding.ty, Type::Str);
        assert_eq!(
            program.checked_facts().local(lazy.binding.local).owner,
            CheckedLocalOwner::View {
                view: ViewId(0),
                role: CheckedViewLocalRole::LazyDependency,
            }
        );
        assert_eq!(
            program
                .checked_facts()
                .expression_use(lazy.dependency)
                .owner,
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::LazyDependency,
            }
        );
    }

    #[test]
    fn lazy_lowering_uses_checked_dependency_and_rejects_binding_drift() {
        let source = format!(
            "app CheckedLazy\n{THEME}state\n  title = \"Hello\"\nview\n  lazy title as cached\n    text cached\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-lazy.ice",
        )
        .unwrap();

        let mut changed_expression = analyze(&source).unwrap();
        let ViewNode::Lazy { dependency, .. } = &mut changed_expression.document.view else {
            panic!("fixture root must be lazy");
        };
        *dependency = Expr::Str("Poisoned".into());
        let actual =
            crate::codegen::generate(&lower(changed_expression).unwrap(), "checked-lazy.ice")
                .unwrap();
        assert_eq!(actual, expected);

        let mut changed_binding = analyze(&source).unwrap();
        let ViewNode::Lazy { binding, .. } = &mut changed_binding.document.view else {
            panic!("fixture root must be lazy");
        };
        *binding = "poisoned".into();
        let error = lower(changed_binding).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("binding contract diverged"));
    }

    #[test]
    fn lazy_codegen_ignores_raw_dependency_and_binding_after_lowering() {
        let source = format!(
            "app LoweredLazy\n{THEME}state\n  title = \"Hello\"\nview\n  lazy title as cached\n    text cached\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-lazy.ice").unwrap();
        let ViewNode::Lazy {
            dependency,
            binding,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be lazy");
        };
        *dependency = Expr::Str("Poisoned".into());
        *binding = "poisoned".into();
        let actual = crate::codegen::generate(&program, "lowered-lazy.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_keyed_flow_binding_and_layout_options() {
        let source = format!(
            "app KeyedHir\nextern crate::backend\n  Item(id:i64, name:str)\n{THEME}state\n  items:[Item] = []\nview\n  keyed item in items by=item.id w=fill(2) h=120.0 gap=8.0 p=4.0 pl=12.0 max-w=640.0 align=end\n    text item.name\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let keyed = program.keyed_column(ViewId(0)).unwrap();
        assert_eq!(keyed.id, ViewId(0));
        assert_eq!(keyed.item.name, "item");
        assert!(matches!(
            keyed.width,
            Some(ResolvedKeyedLength::FillPortion(2))
        ));
        assert!(matches!(
            keyed.height,
            Some(ResolvedKeyedLength::FixedF64(_))
        ));
        assert!(keyed.spacing.is_some());
        assert!(keyed.padding.all.is_some());
        assert!(keyed.padding.left.is_some());
        assert!(keyed.max_width.is_some());
        assert_eq!(keyed.align, Some(FlexAlignment::End));
    }

    #[test]
    fn malformed_checked_keyed_expression_and_local_ids_do_not_panic() {
        let source = format!(
            "app InvalidKeyedFacts\nextern crate::backend\n  Item(id:i64, name:str)\n{THEME}state\n  items:[Item] = []\nview\n  keyed item in items by=item.id gap=8.0\n    text item.name\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::KeyedItems,
            },
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_keyed_item_local(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("local ID is outside its arena"));
    }

    #[test]
    fn keyed_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedKeyed\nextern crate::backend\n  Item(id:i64, name:str)\n{THEME}state\n  items:[Item] = []\nview\n  keyed item in items by=item.id gap=8.0 align=end\n    text item.name\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-keyed.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::KeyedColumn {
            items,
            key,
            options,
            ..
        } = &mut changed_expressions.document.view
        else {
            panic!("fixture root must be keyed");
        };
        *items = Expr::Str("Poisoned".into());
        *key = Expr::Str("Poisoned".into());
        options.spacing = Some(Expr::F64(999.0));
        let actual =
            crate::codegen::generate(&lower(changed_expressions).unwrap(), "checked-keyed.ice")
                .unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::KeyedColumn { options, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be keyed");
        };
        options.align = Some(FlexAlignment::Start);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn keyed_codegen_ignores_raw_flow_and_layout_after_lowering() {
        let source = format!(
            "app LoweredKeyed\nextern crate::backend\n  Item(id:i64, name:str)\n{THEME}state\n  items:[Item] = []\nview\n  keyed item in items by=item.id w=fill(2) gap=8.0 p=4.0 align=end\n    text item.name\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-keyed.ice").unwrap();
        let ViewNode::KeyedColumn {
            item,
            items,
            key,
            options,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be keyed");
        };
        *item = "poisoned".into();
        *items = Expr::Str("Poisoned".into());
        *key = Expr::Str("Poisoned".into());
        options.width = Some(LengthValue::Shrink);
        options.spacing = Some(Expr::F64(999.0));
        options.padding = PaddingOptions::default();
        options.align = Some(FlexAlignment::Start);
        let actual = crate::codegen::generate(&program, "lowered-keyed.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_table_rows_binding_metrics_columns_and_origins() {
        let source = format!(
            "app TableHir\nextern crate::backend\n  Item(name:str)\n{THEME}state\n  rows:[Item] = []\nview\n  table row in rows w=fill p=4.0 sep-x=2.0\n    col w=fill(2) align-x=right align-y=bottom\n      header\n        text \"Name\"\n      cell\n        text row.name\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let table = program.table(ViewId(0)).unwrap();
        assert_eq!(table.id, ViewId(0));
        assert_eq!(table.row.name, "row");
        assert_eq!(table.row.ty, Type::Named("Item".into()));
        assert!(matches!(table.width, Some(ResolvedTableLength::Fill)));
        assert!(table.padding.is_some());
        assert!(table.separator_x.is_some());
        assert_eq!(table.columns.len(), 1);
        assert!(matches!(
            table.columns[0].width,
            Some(ResolvedTableLength::FillPortion(2))
        ));
        assert_eq!(table.columns[0].align_x, Some(InputAlignment::Right));
        assert_eq!(
            program.origin(table.columns[0].origin).parent,
            Some(table.origin)
        );
        assert_eq!(
            program.checked_facts().expression_use(table.rows).owner,
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::TableRows,
            }
        );
    }

    #[test]
    fn malformed_checked_table_expression_and_local_ids_do_not_panic() {
        let source = format!(
            "app InvalidTableFacts\nextern crate::backend\n  Item(name:str)\n{THEME}state\n  rows:[Item] = []\nview\n  table row in rows p=4.0\n    col\n      header\n        text \"Name\"\n      cell\n        text row.name\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::TableRows,
            },
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_table_row_local(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("local ID is outside its arena"));
    }

    #[test]
    fn table_lowering_uses_checked_expressions_and_rejects_static_drift() {
        let source = format!(
            "app CheckedTable\nextern crate::backend\n  Item(name:str)\n{THEME}state\n  rows:[Item] = []\nview\n  table row in rows p=4.0\n    col align-x=right\n      header\n        text \"Name\"\n      cell\n        text row.name\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-table.ice",
        )
        .unwrap();

        let mut changed_expressions = analyze(&source).unwrap();
        let ViewNode::Table { rows, options, .. } = &mut changed_expressions.document.view else {
            panic!("fixture root must be table");
        };
        *rows = Expr::List(vec![]);
        options.padding = Some(Expr::F64(999.0));
        let actual =
            crate::codegen::generate(&lower(changed_expressions).unwrap(), "checked-table.ice")
                .unwrap();
        assert_eq!(actual, expected);

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Table { columns, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be table");
        };
        columns[0].align_x = Some(InputAlignment::Left);
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn table_codegen_ignores_raw_flow_options_and_column_metadata_after_lowering() {
        let source = format!(
            "app LoweredTable\nextern crate::backend\n  Item(name:str)\n{THEME}state\n  rows:[Item] = []\nview\n  table row in rows w=fill p=4.0\n    col w=fill(2) align-x=right\n      header\n        text \"Name\"\n      cell\n        text row.name\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-table.ice").unwrap();
        let ViewNode::Table {
            item,
            rows,
            options,
            columns,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be table");
        };
        *item = "poisoned".into();
        *rows = Expr::List(vec![]);
        options.width = Some(LengthValue::Shrink);
        options.padding = Some(Expr::F64(999.0));
        columns[0].width = Some(LengthValue::Shrink);
        columns[0].align_x = Some(InputAlignment::Left);
        let actual = crate::codegen::generate(&program, "lowered-table.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_complete_pane_grid_state_templates_routes_styles_and_origins() {
        let source = format!(
            "app PaneHir\nextern crate::backend\n  Task(id:i64, title:str)\n  panes-style dynamic_panes(active:bool)\n{THEME}state\n  tasks:[Task] = []\n  active = true\non clicked(name)\nview\n  panes #work w=fill h=64.0 gap=8.0 min-size=120.0 resize=6.0 drag click=clicked(_) style=dynamic_panes(active)\n    style\n      hovered-region bg=primary/25 border=fg border-w=2.0 r=4.0\n      hovered-split color=primary w=3.0\n      picked-split color=danger w=4.0\n    split workspace_root vertical ratio=0.7\n      pane files maximized=files_maximized bg=bg text=fg border=primary border-w=1.0 r=2.0 shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=3.0 px-snap=true\n        title p=4.0 always-controls bg=primary/50 text=fg\n          text \"Files\"\n        controls\n          text \"Controls\"\n        text \"Files body\"\n      pane editor\n        text \"Editor\"\n    pane task in tasks by=task.id maximized=task_maximized\n      col\n        if task_maximized\n          text task.title\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let pane = program.pane_grid(ViewId(0)).unwrap();
        assert_eq!(pane.id, ViewId(0));
        assert_eq!(pane.name, "work");
        assert!(matches!(pane.width, Some(ResolvedPaneLength::Fill)));
        assert!(matches!(pane.height, Some(ResolvedPaneLength::FixedF64(_))));
        assert!(pane.spacing.is_some());
        assert!(pane.min_size.is_some());
        assert!(pane.resize_leeway.is_some());
        assert!(pane.draggable);
        assert!(matches!(
            pane.configuration,
            ResolvedPaneConfiguration::Split {
                ref name,
                axis: ResolvedPaneAxis::Vertical,
                ratio,
                ..
            } if name.as_deref() == Some("workspace_root") && ratio == 0.7
        ));
        let route = pane.click.as_ref().unwrap();
        assert!(matches!(
            route.args.as_slice(),
            [ResolvedInteractionRouteArg::Payload {
                index: 0,
                ty: Type::Str
            }]
        ));
        let custom = pane.custom_style.as_ref().unwrap();
        assert_eq!(
            program.extern_function(custom.function).name,
            "dynamic_panes"
        );
        assert_eq!(custom.arguments.len(), 1);
        assert!(matches!(
            pane.style.hovered_split,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                ..
            })
        ));
        assert_eq!(pane.panes.len(), 2);
        assert_eq!(pane.panes[0].name, "files");
        assert_eq!(pane.panes[0].maximized.as_ref().unwrap().ty, Type::Bool);
        assert!(pane.panes[0].surface.pixel_snap.is_some());
        let title = pane.panes[0].title.as_ref().unwrap();
        assert!(title.padding.all.is_some());
        assert!(title.always_show_controls);
        assert!(title.has_controls);
        assert!(!title.has_compact_controls);
        assert_eq!(pane.templates.len(), 1);
        let template = &pane.templates[0];
        assert_eq!(template.item.name, "task");
        assert_eq!(template.item.ty, Type::Named("Task".into()));
        assert_eq!(template.key_type, Type::I64);
        assert_eq!(template.pane.maximized.as_ref().unwrap().ty, Type::Bool);
        assert_eq!(program.origin(template.origin).parent, Some(pane.origin));
        assert_eq!(
            program.origin(template.pane.origin).parent,
            Some(template.origin)
        );
        assert_eq!(
            program.checked_facts().local(template.item.local).owner,
            CheckedLocalOwner::View {
                view: ViewId(0),
                role: CheckedViewLocalRole::PaneTemplateItem(0),
            }
        );
        assert!(matches!(template.items, ResolvedPaneItems::Value(_)));
    }

    #[test]
    fn source_merged_pane_styles_are_owned_before_origins_remap_to_physical_lines() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-pane-style-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("pane.ice");
        fs::write(
            &root,
            format!(
                "app ImportedPane\nuse \"pane.ice\"\n{THEME}view\n  panes #work\n    pane files bg=bg\n      title text=fg\n        text \"Files\"\n      text \"Body\"\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedContent()\n  text \"Imported\"\n",
        )
        .unwrap();

        let program = lower(analyze_file(&root).unwrap()).unwrap();
        let pane = program.pane_grids().into_iter().next().unwrap();
        assert_eq!(
            program.origin(pane.panes[0].origin).path.as_deref(),
            Some(root.as_path())
        );
        assert!(pane.panes[0].utility_style.background.is_none());
        assert!(
            pane.panes[0]
                .title
                .as_ref()
                .unwrap()
                .utility_style
                .text_color
                .is_none()
        );
        crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn pane_grid_keeps_hir_origins_source_marker_and_diagnostics() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-pane-grid-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let source = format!(
            "app PaneGridOrigins\n{THEME}view\n  panes #work\n    pane files\n      text \"Files\"\n"
        );
        let pane_line = source
            .lines()
            .position(|line| line.trim_start().starts_with("panes #work"))
            .unwrap()
            + 1;
        fs::write(&root, source).unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let pane_grid = program
            .pane_grids
            .values()
            .next()
            .expect("imported pane grid must be normalized");
        let origin = program.origin(pane_grid.origin);
        assert_eq!(origin.path.as_deref(), Some(root.as_path()));
        assert_eq!(origin.line, pane_line);
        let pane_origin = program.origin(pane_grid.panes[0].origin);
        assert_eq!(pane_origin.path.as_deref(), Some(root.as_path()));
        assert_eq!(pane_origin.line, pane_line + 1);
        assert_eq!(pane_origin.parent, Some(pane_grid.origin));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_root = crate::codegen::encode_source_path(&root.display().to_string());
        assert!(generated.contains(&format!(
            "// __ICE_SOURCE {} 1 {encoded_root}",
            pane_line + 2
        )));

        program.pane_grids.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(root.to_str().unwrap()));
        assert_eq!(error.line, pane_line);

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        program.pane_grids.values_mut().next().unwrap().panes.pop();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(root.to_str().unwrap()));
        assert_eq!(error.line, pane_line);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_checked_pane_expression_local_and_origin_ids_do_not_panic() {
        let source = format!(
            "app InvalidPaneFacts\nextern crate::backend\n  Task(id:i64, title:str)\n{THEME}state\n  tasks:[Task] = []\nview\n  panes #work gap=8.0\n    pane files\n      text \"Files\"\n    pane task in tasks by=task.id\n      text task.title\n"
        );

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_pane_expression_id(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression"));

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_pane_template_item_local(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("local"));

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_pane_template_origin(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("origin"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.leak_pane_template_key_into_spacing(ViewId(0));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("another scope"), "{}", error.message);
    }

    #[test]
    fn pane_grid_lowering_and_codegen_ignore_raw_semantics_before_and_after_lowering() {
        let source = format!(
            "app CheckedPane\nextern crate::backend\n  Task(id:i64, title:str)\n{THEME}state\n  tasks:[Task] = []\nview\n  panes #work w=fill gap=8.0 resize=6.0 drag\n    split root vertical ratio=0.7\n      pane files maximized=files_maximized bg=bg\n        title p=4.0 always-controls\n          text \"Files\"\n        controls\n          text \"Controls\"\n        text \"Files body\"\n      pane editor\n        text \"Editor\"\n    pane task in tasks by=task.id maximized=task_maximized\n      text task.title\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-pane.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        poison_raw_pane_semantics(&mut checked.document.view);
        let mut program = lower(checked).unwrap();
        let actual = crate::codegen::generate(&program, "checked-pane.ice").unwrap();
        assert_eq!(actual, expected);

        poison_raw_pane_semantics(&mut program.document.view);
        let actual = crate::codegen::generate(&program, "checked-pane.ice").unwrap();
        assert_eq!(actual, expected);
    }

    fn poison_raw_pane_semantics(node: &mut ViewNode) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            options,
            panes,
            templates,
            ..
        } = node
        else {
            panic!("fixture root must be a pane grid");
        };
        *name = "poisoned".into();
        *configuration = PaneConfiguration::Pane("poisoned".into());
        options.width = Some(LengthValue::Shrink);
        options.spacing = Some(Expr::F64(999.0));
        options.resize_leeway = None;
        options.draggable = false;
        options.style.hovered_split = Some("danger".into());
        panes[0].name = "poisoned_static".into();
        panes[0].maximized = Some("poisoned_maximized".into());
        panes[0].style.background = Some(BackgroundValue::Color("danger".into()));
        let title = panes[0].title.as_mut().unwrap();
        title.padding.all = Some(Expr::F64(999.0));
        title.always_show_controls = false;
        templates[0].items = "poisoned_items".into();
        templates[0].item = "poisoned_item".into();
        templates[0].key = Expr::Bool(false);
        templates[0].pane.name = "poisoned_template".into();
        templates[0].pane.maximized = Some("poisoned_template_maximized".into());
    }

    #[test]
    fn imported_table_keeps_hir_origins_source_marker_and_diagnostics() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-table-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("table-card.ice");
        fs::write(
            &root,
            format!("app ImportedTable\nuse \"table-card.ice\"\n{THEME}view\n  TableCard\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component TableCard()\n  state\n    rows:[str] = [\"A\"]\n  table row in rows\n    col\n      header\n        text \"Name\"\n      cell\n        text row\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let table = program
            .tables
            .values()
            .next()
            .expect("imported table must be normalized");
        let origin = program.origin(table.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 4);
        let column_origin = program.origin(table.columns[0].origin);
        assert_eq!(column_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(column_origin.line, 5);
        assert_eq!(column_origin.parent, Some(table.origin));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 7 1 {encoded_import}")));

        program.tables.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        program.tables.values_mut().next().unwrap().columns.pop();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_if_condition_type_owner_and_scope() {
        let source = format!(
            "app IfHir\n{THEME}state\n  enabled = true\nview\n  if enabled\n    text \"Visible\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let conditional = program.conditional(ViewId(0)).unwrap();
        assert_eq!(conditional.id, ViewId(0));
        let expression = program
            .checked_facts()
            .expression_use(conditional.condition);
        assert_eq!(expression.source, Type::Bool);
        assert_eq!(expression.destination, Type::Bool);
        assert_eq!(
            expression.owner,
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::IfCondition,
            }
        );
    }

    #[test]
    fn malformed_checked_if_expression_id_does_not_panic() {
        let source = format!(
            "app InvalidIfFacts\n{THEME}state\n  enabled = true\nview\n  if enabled\n    text \"Visible\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::IfCondition,
            },
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));
    }

    #[test]
    fn if_lowering_and_codegen_ignore_raw_condition_mutation() {
        let source = format!(
            "app CheckedIf\n{THEME}state\n  enabled = true\nview\n  col\n    if enabled\n      text \"Visible\"\n"
        );
        let expected =
            crate::codegen::generate(&lower(analyze(&source).unwrap()).unwrap(), "checked-if.ice")
                .unwrap();

        let mut changed = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut changed.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::If { condition, .. } = &mut children[0] else {
            panic!("fixture child must be if");
        };
        *condition = Expr::Bool(false);
        let mut program = lower(changed).unwrap();
        let actual = crate::codegen::generate(&program, "checked-if.ice").unwrap();
        assert_eq!(actual, expected);

        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::If { condition, .. } = &mut children[0] else {
            panic!("fixture child must be if");
        };
        *condition = Expr::Bool(false);
        let actual = crate::codegen::generate(&program, "checked-if.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_for_items_binding_and_reconciliation_identity() {
        let source = format!(
            "app ForHir\n{THEME}state\n  items:[str] = []\nview\n  for item in items\n    text item\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let iteration = program.iteration(ViewId(0)).unwrap();
        assert_eq!(iteration.id, ViewId(0));
        assert_eq!(iteration.item.name, "item");
        assert_eq!(iteration.item.ty, Type::Str);
        assert_eq!(iteration.reconciliation_line, 15);
        assert_eq!(
            program.checked_facts().local(iteration.item.local).owner,
            CheckedLocalOwner::View {
                view: ViewId(0),
                role: CheckedViewLocalRole::ForItem,
            }
        );
        assert_eq!(
            program
                .checked_facts()
                .expression_use(iteration.items)
                .owner,
            CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::ForItems,
            }
        );
    }

    #[test]
    fn malformed_checked_for_expression_and_local_ids_do_not_panic() {
        let source = format!(
            "app InvalidForFacts\n{THEME}state\n  items:[str] = []\nview\n  for item in items\n    text item\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::ForItems,
            },
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));

        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_for_item_local(ViewId(0), u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("local ID is outside its arena"));
    }

    #[test]
    fn for_lowering_uses_checked_items_and_binding() {
        let source = format!(
            "app CheckedFor\n{THEME}state\n  items:[str] = []\nview\n  col\n    for item in items\n      text item\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-for.ice",
        )
        .unwrap();

        let mut changed_items = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut changed_items.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::For { items, .. } = &mut children[0] else {
            panic!("fixture child must be for");
        };
        *items = Expr::Str("Poisoned".into());
        let actual =
            crate::codegen::generate(&lower(changed_items).unwrap(), "checked-for.ice").unwrap();
        assert_eq!(actual, expected);

        let mut changed_binding = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut changed_binding.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::For { item, .. } = &mut children[0] else {
            panic!("fixture child must be for");
        };
        *item = "poisoned".into();
        let actual =
            crate::codegen::generate(&lower(changed_binding).unwrap(), "checked-for.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn for_codegen_ignores_raw_flow_after_lowering() {
        let source = format!(
            "app LoweredFor\n{THEME}state\n  items:[str] = []\nview\n  col\n    for item in items\n      text item\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-for.ice").unwrap();
        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::For { item, items, .. } = &mut children[0] else {
            panic!("fixture child must be for");
        };
        *item = "poisoned".into();
        *items = Expr::Str("Poisoned".into());
        let actual = crate::codegen::generate(&program, "lowered-for.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_match_value_patterns_bindings_and_origins() {
        let source = format!(
            "app MatchHir\n{THEME}state\n  choice:str? = some(\"ready\")\nview\n  match choice\n    some(label)\n      text label\n    none\n      text \"none\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let resolved = program.match_view(ViewId(0)).unwrap();
        assert_eq!(resolved.id, ViewId(0));
        assert_eq!(resolved.value_ty, Type::Option(Box::new(Type::Str)));
        assert_eq!(resolved.arms.len(), 2);
        assert!(matches!(
            resolved.arms[0].pattern,
            ResolvedMatchPattern::Some
        ));
        let binding = resolved.arms[0].binding.as_ref().unwrap();
        assert_eq!(binding.name, "label");
        assert_eq!(binding.ty, Type::Str);
        assert_eq!(
            program.checked_facts().local(binding.local).owner,
            CheckedLocalOwner::View {
                view: ViewId(0),
                role: CheckedViewLocalRole::MatchPayload(0),
            }
        );
        assert!(matches!(
            resolved.arms[1].pattern,
            ResolvedMatchPattern::None
        ));
        assert!(resolved.arms[1].binding.is_none());
        assert_eq!(resolved.arms[0].children, vec![ViewId(1)]);
        assert_eq!(resolved.arms[1].children, vec![ViewId(2)]);
        assert_ne!(resolved.arms[0].origin, resolved.arms[1].origin);
    }

    #[test]
    fn match_lowering_rejects_checked_pattern_coverage_drift() {
        let enum_source = format!(
            "app MatchCoverage\n{THEME}enum Status\n  ready\n  done\nstate\n  status:Status = Status.ready\nview\n  match status\n    Status.ready\n      text \"ready\"\n    Status.done\n      text \"done\"\n"
        );
        let mut checked = analyze(&enum_source).unwrap();
        let view = checked
            .facts
            .views()
            .iter()
            .find(|view| matches!(view.flow, CheckedViewFlow::Match { .. }))
            .unwrap()
            .id;
        let ready = checked
            .declarations
            .enum_decl_by_name("Status")
            .unwrap()
            .variants[0]
            .declaration
            .id;
        checked
            .facts
            .corrupt_match_pattern(view, 1, CheckedMatchPattern::Enum(ready));
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("duplicate case"));

        let option_source = format!(
            "app MatchWildcard\n{THEME}state\n  choice:str? = none\nview\n  match choice\n    some(label)\n      text label\n    none\n      text \"none\"\n"
        );
        let mut checked = analyze(&option_source).unwrap();
        let view = checked
            .facts
            .views()
            .iter()
            .find(|view| matches!(view.flow, CheckedViewFlow::Match { .. }))
            .unwrap()
            .id;
        checked
            .facts
            .corrupt_match_pattern(view, 0, CheckedMatchPattern::Wildcard);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("wildcard topology"));

        let mut checked = analyze(&option_source).unwrap();
        let view = checked
            .facts
            .views()
            .iter()
            .find(|view| matches!(view.flow, CheckedViewFlow::Match { .. }))
            .unwrap()
            .id;
        checked.facts.remove_match_arm(view, 1);
        let ViewNode::Match { arms, .. } = &mut checked.document.view else {
            panic!("fixture root must be match");
        };
        arms.remove(1);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("children diverged from checked topology")
        );
    }

    #[test]
    fn match_lowering_rejects_raw_arm_child_reassignment() {
        let source = format!(
            "app MatchTopology\n{THEME}state\n  choice:str? = none\nview\n  col\n    match choice\n      some(label)\n        text \"first\"\n        text \"second\"\n      none\n        text \"none\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Match { arms, .. } = &mut children[0] else {
            panic!("fixture child must be match");
        };
        let moved = arms[0].children.pop().unwrap();
        arms[1].children.insert(0, moved);

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("checked arm topology"));
    }

    #[test]
    fn normal_and_flex_match_codegen_ignore_raw_arm_child_reassignment() {
        for layout in ["col", "flex dir=column"] {
            let source = format!(
                "app MatchTopology\n{THEME}state\n  choice:str? = none\nview\n  {layout}\n    match choice\n      some(label)\n        text \"first\"\n        text \"second\"\n      none\n        text \"none\"\n"
            );
            let mut program = lower(analyze(&source).unwrap()).unwrap();
            let expected = crate::codegen::generate(&program, "match-topology.ice").unwrap();
            let ViewNode::Layout { children, .. } = &mut program.document.view else {
                panic!("fixture root must be a layout");
            };
            let ViewNode::Match { arms, .. } = &mut children[0] else {
                panic!("fixture child must be match");
            };
            let moved = arms[0].children.pop().unwrap();
            arms[1].children.insert(0, moved);

            let actual = crate::codegen::generate(&program, "match-topology.ice").unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn malformed_checked_match_expression_and_local_ids_do_not_panic() {
        let source = format!(
            "app InvalidMatchFacts\n{THEME}state\n  choice:str? = some(\"ready\")\nview\n  match choice\n    some(label)\n      text label\n    none\n      text \"none\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.facts.corrupt_expression_use_root(
            crate::check::CheckedExprOwner::View {
                view: ViewId(0),
                role: CheckedViewExprRole::MatchValue,
            },
            u32::MAX,
        );
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked expression ID"));

        let mut checked = analyze(&source).unwrap();
        checked
            .facts
            .corrupt_match_binding_local(ViewId(0), 0, u32::MAX);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("local ID is outside its arena"));
    }

    #[test]
    fn match_lowering_and_codegen_ignore_raw_value_pattern_and_binding() {
        let source = format!(
            "app CheckedMatch\n{THEME}state\n  choice:str? = some(\"ready\")\nview\n  col\n    match choice\n      some(label)\n        text label\n      none\n        text \"none\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-match.ice",
        )
        .unwrap();

        let mut changed = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut changed.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Match { value, arms, .. } = &mut children[0] else {
            panic!("fixture child must be match");
        };
        *value = Expr::Bool(false);
        arms[0].pattern = MatchPattern::Wildcard;
        let mut program = lower(changed).unwrap();
        let actual = crate::codegen::generate(&program, "checked-match.ice").unwrap();
        assert_eq!(actual, expected);

        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Match { value, arms, .. } = &mut children[0] else {
            panic!("fixture child must be match");
        };
        *value = Expr::Bool(false);
        arms[0].pattern = MatchPattern::Wildcard;
        let actual = crate::codegen::generate(&program, "checked-match.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn imported_remaining_controls_keep_origins_and_source_map_hir_failures() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-remaining-control-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("controls.ice");
        fs::write(
            &root,
            format!(
                "app ImportedRemainingControlsApp\nuse \"controls.ice\"\n{THEME}state\n  modes:combo[str] = [\"One\"]\n  selected:str? = none\nview\n  ImportedRemainingControls modes<->modes selected=selected\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedRemainingControls(bind modes:combo[str], selected:str?)\n  on chosen(next)\n  on moved(x, y)\n  on resized(dx, dy)\n  on shown(width, height)\n  col\n    combo modes selected \"Search\" -> chosen _\n    image \"photo.png\"\n    tooltip position=cursor\n      text \"Hover\"\n      text \"Tip\"\n    mouse move=moved cursor=pointer\n      text \"Pointer\"\n    resize-handle drag=resized cursor=resize-horizontal\n      text \"Resize\"\n    sensor show=shown key=true anticipate=0.0 delay=0\n      text \"Observed\"\n",
        )
        .unwrap();
        fs::write(directory.join("photo.png"), "png bytes").unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let combo_id = program.combo_boxes.values().next().unwrap().id;
        let media_id = program.media.values().next().unwrap().id;
        let tooltip_id = program.tooltips.values().next().unwrap().id;
        for (id, line) in [(combo_id, 7usize), (media_id, 8), (tooltip_id, 9)] {
            let origin = program.origin(program.checked_facts().view(id).origin);
            assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
            assert_eq!(origin.line, line);
        }
        let interaction_ids = program
            .interaction_widgets
            .values()
            .map(|interaction| match interaction {
                ResolvedInteractionWidget::MouseArea(widget) => (widget.id, 12usize),
                ResolvedInteractionWidget::ResizeHandle(widget) => (widget.id, 14usize),
                ResolvedInteractionWidget::Sensor(widget) => (widget.id, 16usize),
            })
            .collect::<Vec<_>>();
        assert_eq!(interaction_ids.len(), 3);
        for (id, line) in &interaction_ids {
            let origin = program.origin(program.checked_facts().view(*id).origin);
            assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
            assert_eq!(origin.line, *line);
        }

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 7 1 {encoded_import}")));

        let combo = program.combo_boxes.remove(&combo_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 7);
        program.combo_boxes.insert(combo_id, combo);

        let media = program.media.remove(&media_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 8);
        program.media.insert(media_id, media);

        let tooltip = program.tooltips.remove(&tooltip_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 9);
        program.tooltips.insert(tooltip_id, tooltip);

        for (id, line) in interaction_ids {
            let interaction = program.interaction_widgets.remove(&id).unwrap();
            let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
            assert_eq!(error.code, "E196");
            assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
            assert_eq!(error.line, line);
            program.interaction_widgets.insert(id, interaction);
        }

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn match_codegen_consumes_the_resolved_binding_type() {
        let source = format!(
            "app ResolvedMatchBinding\n{THEME}state\n  choice:str? = some(\"ready\")\nview\n  col\n    match choice\n      some(label)\n        text label\n      none\n        text \"none\"\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "resolved-match-binding.ice").unwrap();
        let local = program.match_views.values().next().unwrap().arms[0]
            .binding
            .as_ref()
            .unwrap()
            .local;
        program.facts.corrupt_local_type(local, Type::Bool);

        let actual = crate::codegen::generate(&program, "resolved-match-binding.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn imported_match_keeps_hir_origins_source_marker_and_diagnostics() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-match-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("choice-card.ice");
        fs::write(
            &root,
            format!("app ImportedMatch\nuse \"choice-card.ice\"\n{THEME}view\n  ChoiceCard\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ChoiceCard()\n  state\n    choice:str? = some(\"ready\")\n  col\n    match choice\n      some(label)\n        text label\n      none\n        text \"none\"\n",
        )
        .unwrap();

        let mut checked = analyze_file(&root).unwrap();
        let view = checked
            .facts
            .views()
            .iter()
            .find(|view| matches!(view.flow, CheckedViewFlow::Match { .. }))
            .unwrap()
            .id;
        let match_origin = checked.facts.view(view).origin;
        checked
            .facts
            .corrupt_match_arm_origin(view, 1, match_origin);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 8);
        assert!(error.message.contains("checked parent or source"));

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let resolved = program
            .match_views
            .values()
            .next()
            .expect("imported match must be normalized");
        let origin = program.origin(resolved.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 5);
        let arm_origin = program.origin(resolved.arms[0].origin);
        assert_eq!(arm_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(arm_origin.line, 6);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 7 1 {encoded_import}")));

        program.match_views.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 5);

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        program.match_views.values_mut().next().unwrap().arms[0].binding = None;
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 6);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_for_keeps_hir_origin_source_marker_and_diagnostic() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-for-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("iteration-card.ice");
        fs::write(
            &root,
            format!("app ImportedFor\nuse \"iteration-card.ice\"\n{THEME}view\n  IterationCard\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component IterationCard()\n  state\n    items:[str] = [\"A\", \"B\"]\n  col\n    for item in items\n      text item\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let iteration = program
            .iterations
            .values()
            .next()
            .expect("imported for must be normalized");
        let origin = program.origin(iteration.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 5);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        // Flow nodes emit their children inline, so the generated marker belongs to the
        // imported body while the normalized iteration retains the flow's own origin.
        assert!(generated.contains(&format!("// __ICE_SOURCE 6 1 {encoded_import}")));

        program.iterations.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 5);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_if_keeps_hir_origin_source_marker_and_diagnostic() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-if-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("conditional-card.ice");
        fs::write(
            &root,
            format!(
                "app ImportedConditional\nuse \"conditional-card.ice\"\n{THEME}view\n  ConditionalCard\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ConditionalCard()\n  col\n    if true\n      text \"Visible\"\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let conditional = program
            .conditionals
            .values()
            .next()
            .expect("imported if must be normalized");
        let origin = program.origin(conditional.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 3);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 4 1 {encoded_import}")));

        program.conditionals.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_keyed_column_keeps_hir_origin_source_marker_and_diagnostic() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-keyed-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("keyed-card.ice");
        fs::write(
            &root,
            format!("app ImportedKeyed\nuse \"keyed-card.ice\"\n{THEME}view\n  KeyedCard\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component KeyedCard()\n  state\n    items:[i64] = [1, 2]\n  keyed item in items by=item\n    text item\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let keyed = program
            .keyed_columns
            .values()
            .next()
            .expect("imported keyed column must be normalized");
        let origin = program.origin(keyed.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 4);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 4 1 {encoded_import}")));

        program.keyed_columns.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_lazy_keeps_hir_origin_source_marker_and_diagnostic() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-lazy-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("lazy-card.ice");
        fs::write(
            &root,
            format!("app ImportedLazy\nuse \"lazy-card.ice\"\n{THEME}view\n  LazyCard\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component LazyCard()\n  lazy \"Hello\" as cached\n    text cached\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let lazy = program
            .lazy_views
            .values()
            .next()
            .expect("imported lazy must be normalized");
        let origin = program.origin(lazy.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 2);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_import}")));

        program.lazy_views.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 2);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_responsive_keeps_hir_origin_source_marker_and_diagnostic() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-responsive-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("responsive-card.ice");
        fs::write(
            &root,
            format!(
                "app ImportedResponsive\nuse \"responsive-card.ice\"\n{THEME}view\n  ResponsiveCard\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ResponsiveCard()\n  responsive at=600.0 w=fill h=40.0\n    text \"Narrow\"\n    text \"Wide\"\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let responsive = program
            .responsives
            .values()
            .next()
            .expect("imported responsive must be normalized");
        let origin = program.origin(responsive.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 2);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_import}")));

        program.responsives.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 2);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_pin_keeps_hir_origin_source_marker_and_diagnostic() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-pin-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("pin-card.ice");
        fs::write(
            &root,
            format!("app ImportedPin\nuse \"pin-card.ice\"\n{THEME}view\n  PinCard\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component PinCard()\n  pin x=0.0 y=0.0\n    text \"Pinned\"\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let pin = program
            .pins
            .values()
            .next()
            .expect("imported pin must be normalized");
        let origin = program.origin(pin.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 2);

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 2 1 {encoded_import}")));

        program.pins.clear();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 2);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_media_source_options_colors_styles_and_viewer_defaults() {
        let source = format!(
            "app MediaHir\nextern crate::backend\n  svg-style dynamic_svg(active:bool)\n{THEME}state\n  active = true\n  path = \"photo.png\"\nview\n  col\n    image path w=fill h=64.0 filter=nearest crop=(1, 2, 30, 40)\n    viewer path min-scale=0.5 p=8.0 scale-step=0.25\n    svg \"icon.svg\" color=fg hover=none style=dynamic_svg(active) opacity=0.8\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();

        let image = program.media(ViewId(1)).unwrap();
        assert_eq!(image.id, ViewId(1));
        assert_eq!(image.kind, ResolvedMediaKind::Image);
        assert_eq!(image.source_type, Type::Str);
        assert!(matches!(
            image.options.width,
            Some(ResolvedMediaLength::Fill)
        ));
        assert!(matches!(
            image.options.height,
            Some(ResolvedMediaLength::Fixed {
                source: Type::F64,
                ..
            })
        ));
        assert_eq!(image.options.filter, Some(ResolvedMediaFilter::Nearest));
        assert!(image.options.crop.is_some());

        let viewer = program.media(ViewId(2)).unwrap();
        assert_eq!(viewer.kind, ResolvedMediaKind::Viewer);
        let bounds = viewer.options.scale_bounds.as_ref().unwrap();
        assert!(matches!(
            bounds.minimum,
            ResolvedMediaScaleBound::Expression(_)
        ));
        assert!(matches!(
            bounds.maximum,
            ResolvedMediaScaleBound::Default(value) if value == 10.0
        ));
        assert!(viewer.options.padding.is_some());
        assert!(viewer.options.scale_step.is_some());

        let svg = program.media(ViewId(3)).unwrap();
        assert_eq!(svg.kind, ResolvedMediaKind::Svg);
        let colors = svg.options.svg_colors.as_ref().unwrap();
        assert!(matches!(
            colors.idle,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 1, .. }),
                opacity: None,
            })
        ));
        assert!(matches!(colors.hovered, Some(None)));
        let style = svg.options.svg_style.as_ref().unwrap();
        assert_eq!(program.extern_function(style.function).name, "dynamic_svg");
        assert_eq!(style.arguments.len(), 1);
        assert!(svg.options.opacity.is_some());
        assert_eq!(
            program
                .facts
                .expression_use_by_owner(CheckedExprOwner::Media(MediaExpressionId {
                    media: ViewId(3),
                    index: 0,
                })),
            Some(svg.source)
        );
    }

    #[test]
    #[ignore = "large normalized media lowering and emission performance contract"]
    fn performance_contract_four_thousand_media_nodes_lower_and_emit_under_two_seconds() {
        const MEDIA: usize = 4_000;
        let mut source = format!("app MediaScale\n{THEME}view\n  col\n");
        for index in 0..MEDIA {
            writeln!(
                source,
                "    image \"photo-{index}.png\" w=fill h=48.0 opacity=0.8 filter=nearest"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "media-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.media.len(), MEDIA);
        assert_eq!(generated.matches("::iced::widget::image(").count(), MEDIA);
        eprintln!("4k normalized media nodes lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized media nodes lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized tooltip lowering and emission performance contract"]
    fn performance_contract_four_thousand_tooltips_lower_and_emit_under_two_seconds() {
        const TOOLTIPS: usize = 4_000;
        let mut source = format!("app TooltipScale\n{THEME}view\n  col\n");
        for index in 0..TOOLTIPS {
            writeln!(
                source,
                "    tooltip position=cursor gap=2.0 p=5.0 delay=100 bg=primary\n      text \"Hover {index}\"\n      text \"Tip {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "tooltip-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.tooltips.len(), TOOLTIPS);
        assert_eq!(
            generated.matches("::iced::widget::tooltip(").count(),
            TOOLTIPS
        );
        eprintln!("4k normalized tooltips lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized tooltips lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized interaction-widget lowering and emission performance contract"]
    fn performance_contract_four_thousand_interaction_widgets_lower_and_emit_under_two_seconds() {
        const WIDGETS: usize = 4_000;
        let mut source = format!("app InteractionScale\n{THEME}on moved(x, y)\nview\n  col\n");
        for index in 0..WIDGETS {
            writeln!(
                source,
                "    mouse move=moved cursor=pointer\n      text \"Pointer {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "interaction-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.interaction_widgets.len(), WIDGETS);
        assert_eq!(
            generated.matches("::iced::widget::mouse_area(").count(),
            WIDGETS
        );
        eprintln!("4k normalized interaction widgets lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized interaction widgets lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized overlay lowering and emission performance contract"]
    fn performance_contract_four_thousand_overlays_lower_and_emit_under_two_seconds() {
        const OVERLAYS: usize = 4_000;
        let mut source = format!("app OverlayScale\n{THEME}on close\nview\n  col\n");
        for index in 0..OVERLAYS {
            writeln!(
                source,
                "    overlay when=true dismiss=close backdrop=black/60 p=8.0\n      content\n        text \"Page {index}\"\n      layer\n        text \"Dialog {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "overlay-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.overlays.len(), OVERLAYS);
        // Overlays are published, so the count to check is the node the
        // template carries, not the inline `Stack` it replaced.
        assert_eq!(
            generated.matches(r#"\"kind\": \"overlay\""#).count(),
            OVERLAYS
        );
        eprintln!("4k normalized overlays lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized overlays lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized container lowering and emission performance contract"]
    fn performance_contract_four_thousand_containers_lower_and_emit_under_two_seconds() {
        const CONTAINERS: usize = 4_000;
        let mut source = format!("app ContainerScale\n{THEME}view\n  flex wrap=wrap\n");
        for index in 0..CONTAINERS {
            writeln!(
                source,
                "    box h=48.0 p=8.0 bg=bg border=primary border-w=1.0 r=4.0 grow=1.0 basis=percent(25.0)\n      text \"Card {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "container-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.containers.len(), CONTAINERS);
        assert_eq!(
            generated
                .matches("::iced::widget::container(__container_content)")
                .count(),
            CONTAINERS
        );
        eprintln!("4k normalized containers lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized containers lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized layout lowering and emission performance contract"]
    fn performance_contract_four_thousand_layouts_lower_and_emit_under_two_seconds() {
        const LAYOUTS: usize = 4_000;
        let mut source = format!("app LayoutScale\n{THEME}view\n  col\n");
        for index in 0..LAYOUTS {
            writeln!(
                source,
                "    row gap=4.0 p=2.0 w=fill align=center clip=false\n      text \"Item {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "layout-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.layouts.len(), LAYOUTS + 1);
        assert_eq!(
            generated.matches("::iced::widget::row(__children)").count(),
            LAYOUTS
        );
        eprintln!("4k normalized layouts lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized layouts lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized text lowering and emission performance contract"]
    fn performance_contract_four_thousand_text_nodes_lower_and_emit_under_two_seconds() {
        const TEXTS: usize = 4_000;
        let mut source = format!("app TextScale\n{THEME}view\n  col\n");
        for index in 0..TEXTS {
            writeln!(source, "    text \"Item {index}\" size=14.0 w=fill").unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "text-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.texts.len(), TEXTS);
        assert_eq!(
            generated
                .matches("::iced::widget::text(__text_value.clone())")
                .count(),
            TEXTS
        );
        eprintln!("4k normalized text nodes lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized text nodes lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized input lowering and emission performance contract"]
    fn performance_contract_four_thousand_inputs_lower_and_emit_under_two_seconds() {
        const INPUTS: usize = 4_000;
        let mut source = format!("app InputScale\n{THEME}state\n  value = \"\"\nview\n  col\n");
        for index in 0..INPUTS {
            writeln!(
                source,
                "    input \"Field {index}\" <-> value hint=\"Type\" w=240.0 p=8.0 text-size=14.0"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "input-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.inputs.len(), INPUTS);
        assert_eq!(
            generated.matches("::iced::widget::text_input(").count(),
            INPUTS
        );
        eprintln!("4k normalized inputs lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized inputs lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized pick-list lowering and emission performance contract"]
    fn performance_contract_four_thousand_pick_lists_lower_and_emit_under_two_seconds() {
        const PICKS: usize = 4_000;
        let mut source = format!(
            "app PickScale\n{THEME}state\n  choices = [\"One\", \"Two\"]\n  selected:str? = none\non selected(next)\nview\n  col\n"
        );
        for _ in 0..PICKS {
            writeln!(
                source,
                "    pick choices selected hint=\"Choose\" w=240.0 p=8.0 text-size=14.0 -> selected _"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "pick-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.pick_lists.len(), PICKS);
        assert_eq!(
            generated.matches("::iced::widget::pick_list(").count(),
            PICKS
        );
        eprintln!("4k normalized pick lists lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized pick lists lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "distinct combo-state lookup and normalized emission must remain linear"]
    fn performance_contract_four_thousand_distinct_combo_boxes_emit_under_two_seconds() {
        const COMBOS: usize = 4_000;
        let mut source = format!("app ComboScale\n{THEME}state\n  selected:str? = none\n");
        for index in 0..COMBOS {
            writeln!(source, "  choices_{index}:combo[str] = [\"Item {index}\"]").unwrap();
        }
        source.push_str("on selected(next)\nview\n  col\n");
        for index in 0..COMBOS {
            writeln!(
                source,
                "    combo choices_{index} selected \"Search {index}\" w=240.0 p=8.0 -> selected _"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let lowered = started.elapsed();
        let generated = crate::codegen::generate(&program, "combo-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.combo_boxes.len(), COMBOS);
        assert_eq!(
            generated.matches("::iced::widget::combo_box(").count(),
            COMBOS
        );
        eprintln!(
            "4k distinct normalized combo boxes lowered in {lowered:?} and emitted in {:?} ({elapsed:?} total)",
            elapsed - lowered,
        );
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k distinct normalized combo boxes lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized sensor lowering and emission performance contract"]
    fn performance_contract_four_thousand_sensors_lower_and_emit_under_two_seconds() {
        const SENSORS: usize = 4_000;
        let mut source =
            format!("app SensorScale\n{THEME}on shown(width, height)\non hidden\nview\n  col\n");
        for index in 0..SENSORS {
            writeln!(
                source,
                "    sensor show=shown hide=hidden key={index} anticipate=24.0 delay=50\n      text \"Observed {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "sensor-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.interaction_widgets.len(), SENSORS);
        assert_eq!(
            generated.matches("::iced::widget::sensor(").count(),
            SENSORS
        );
        eprintln!("4k normalized sensors lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized sensors lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized float lowering and emission performance contract"]
    fn performance_contract_four_thousand_floats_lower_and_emit_under_two_seconds() {
        const FLOATS: usize = 4_000;
        let mut source = format!("app FloatScale\n{THEME}view\n  col\n");
        for index in 0..FLOATS {
            writeln!(
                source,
                "    float scale=1.0 x=original_x y=viewport_y shadow=primary/25 shadow-blur=4.0 r=8.0\n      text \"Floating {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "float-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.floats.len(), FLOATS);
        assert_eq!(generated.matches("::iced::widget::float(").count(), FLOATS);
        eprintln!("4k normalized floats lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized floats lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized pin lowering and emission performance contract"]
    fn performance_contract_four_thousand_pins_lower_and_emit_under_two_seconds() {
        const PINS: usize = 4_000;
        let mut source = format!("app PinScale\n{THEME}view\n  col\n");
        for index in 0..PINS {
            writeln!(
                source,
                "    pin w=fill h=80.0 x=12.0 y=8.0\n      text \"Pinned {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "pin-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.pins.len(), PINS);
        assert_eq!(generated.matches("::iced::widget::pin(").count(), PINS);
        eprintln!("4k normalized pins lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized pins lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized responsive lowering and emission performance contract"]
    fn performance_contract_four_thousand_responsives_lower_and_emit_under_two_seconds() {
        const RESPONSIVES: usize = 4_000;
        let mut source = format!("app ResponsiveScale\n{THEME}view\n  col\n");
        for index in 0..RESPONSIVES {
            writeln!(
                source,
                "    responsive at=600.0 w=fill h=40.0\n      text \"Narrow {index}\"\n      text \"Wide {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "responsive-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.responsives.len(), RESPONSIVES);
        assert_eq!(
            generated.matches("::iced::widget::responsive(").count(),
            RESPONSIVES
        );
        eprintln!("4k normalized responsives lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized responsives lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized lazy lowering and emission performance contract"]
    fn performance_contract_four_thousand_lazy_views_lower_and_emit_under_two_seconds() {
        const LAZY_VIEWS: usize = 4_000;
        let mut source = format!("app LazyScale\n{THEME}state\n  title = \"Hello\"\nview\n  col\n");
        for index in 0..LAZY_VIEWS {
            writeln!(
                source,
                "    lazy title as cached_{index}\n      text cached_{index}"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "lazy-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.lazy_views.len(), LAZY_VIEWS);
        assert_eq!(
            generated.matches("::ui_lang_runtime::memo_lazy(").count(),
            LAZY_VIEWS
        );
        eprintln!("4k normalized lazy views lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized lazy views lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized keyed-column lowering and emission performance contract"]
    fn performance_contract_four_thousand_keyed_columns_lower_and_emit_under_two_seconds() {
        const KEYED_COLUMNS: usize = 4_000;
        let mut source = format!(
            "app KeyedScale\nextern crate::backend\n  Item(id:i64, name:str)\n{THEME}state\n  items:[Item] = []\nview\n  col\n"
        );
        for index in 0..KEYED_COLUMNS {
            writeln!(
                source,
                "    keyed item_{index} in items by=item_{index}.id gap=8.0 p=4.0\n      text item_{index}.name"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "keyed-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.keyed_columns.len(), KEYED_COLUMNS);
        assert_eq!(
            generated.matches("::iced::widget::keyed_column(").count(),
            KEYED_COLUMNS
        );
        eprintln!("4k normalized keyed columns lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized keyed columns lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized table lowering and emission performance contract"]
    fn performance_contract_four_thousand_tables_lower_and_emit_under_two_seconds() {
        const TABLES: usize = 4_000;
        let mut source = format!(
            "app TableScale\nextern crate::backend\n  Item(name:str)\n{THEME}state\n  rows:[Item] = []\nview\n  col\n"
        );
        for index in 0..TABLES {
            writeln!(
                source,
                "    table row_{index} in rows p=4.0\n      col w=fill align-x=left\n        header\n          text \"Name {index}\"\n        cell\n          text row_{index}.name"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "table-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.tables.len(), TABLES);
        assert_eq!(
            generated.matches("::iced::widget::table::table(").count(),
            TABLES
        );
        eprintln!("4k normalized tables lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized tables lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized pane-grid lowering and emission performance contract"]
    fn performance_contract_four_thousand_pane_grids_lower_and_emit_under_two_seconds() {
        const PANE_GRIDS: usize = 4_000;
        let mut source = format!("app PaneScale\n{THEME}view\n  col\n");
        for index in 0..PANE_GRIDS {
            writeln!(
                source,
                "    panes #work_{index} w=fill gap=8.0\n      pane main\n        text \"Pane {index}\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "pane-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.pane_grids.len(), PANE_GRIDS);
        let pane_render_count = generated
            .matches("::iced::widget::pane_grid(&self.")
            .count();
        assert_eq!(pane_render_count, PANE_GRIDS);
        eprintln!("4k normalized pane grids lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized pane grids lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized if-view lowering and emission performance contract"]
    fn performance_contract_four_thousand_if_views_lower_and_emit_under_two_seconds() {
        const IF_VIEWS: usize = 4_000;
        let mut source = format!("app IfScale\n{THEME}state\n  enabled = true\nview\n  col\n");
        for index in 0..IF_VIEWS {
            writeln!(source, "    if enabled\n      text \"Visible {index}\"").unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "if-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.conditionals.len(), IF_VIEWS);
        assert_eq!(generated.matches("if self.enabled").count(), IF_VIEWS);
        eprintln!("4k normalized if views lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized if views lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized for-view lowering and emission performance contract"]
    fn performance_contract_four_thousand_for_views_lower_and_emit_under_two_seconds() {
        const FOR_VIEWS: usize = 4_000;
        let mut source = format!("app ForScale\n{THEME}state\n  items:[str] = []\nview\n  col\n");
        for index in 0..FOR_VIEWS {
            writeln!(
                source,
                "    for item_{index} in items\n      text item_{index}"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "for-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.iterations.len(), FOR_VIEWS);
        assert_eq!(
            generated.matches("for (__ice_index, item_").count(),
            FOR_VIEWS
        );
        eprintln!("4k normalized for views lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized for views lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized match-view lowering and emission performance contract"]
    fn performance_contract_four_thousand_match_views_lower_and_emit_under_two_seconds() {
        const MATCH_VIEWS: usize = 4_000;
        let mut source =
            format!("app MatchScale\n{THEME}state\n  choice:str? = some(\"ready\")\nview\n  col\n");
        for index in 0..MATCH_VIEWS {
            writeln!(
                source,
                "    match choice\n      some(label_{index})\n        text label_{index}\n      none\n        text \"none\""
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "match-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.match_views.len(), MATCH_VIEWS);
        assert_eq!(
            generated.matches("match &(self.choice)").count(),
            MATCH_VIEWS
        );
        eprintln!("4k normalized match views lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized match views lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    fn canvas_codegen_ignores_raw_commands_and_theme_token_order_after_lowering() {
        let source = format!(
            "app LoweredCanvas\n{THEME}view\n  canvas w=120.0 h=80.0\n    circle x=10.0 y=20.0 r=4.0 fill=primary\n    text \"ready\" x=4.0 y=8.0 color=fg\n"
        );
        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-canvas.ice").unwrap();

        let ViewNode::Canvas { commands, .. } = &mut program.document.view else {
            panic!("fixture root must be a canvas");
        };
        let CanvasCommand::Circle { paint, .. } = &mut commands[0] else {
            panic!("fixture command must be a circle");
        };
        paint.fill = Some(BackgroundValue::Color("danger".into()));
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(1, 2);

        let actual = crate::codegen::generate(&program, "lowered-canvas.ice").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn normalizes_canvas_state_commands_events_routes_and_theme_colors() {
        let source = format!(
            "app CanvasHir\n{THEME}on released(button)\nview\n  canvas w=fill h=120.0\n    state\n      hits = 0\n    event mouse pressed\n      set hits = hits + 1\n      redraw\n      capture\n    event mouse released as button\n      emit released button\n    text hits x=8.0 y=20.0 color=fg\n    for value in [12.0, 24.0]\n      circle x=value y=40.0 r=4.0 fill=primary\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let canvas = program.canvas(ViewId(0)).unwrap();

        assert_eq!(canvas.states[0].id.index, 0);
        assert!(matches!(
            canvas.options.width,
            Some(ResolvedCanvasLength::Fill)
        ));
        assert!(matches!(
            canvas.options.height,
            Some(ResolvedCanvasLength::Fixed {
                source: Type::F64,
                ..
            })
        ));
        let ResolvedCanvasCommand::Text { id, color, .. } = &canvas.commands[0] else {
            panic!("first command must be normalized text");
        };
        assert_eq!(id.index, 0);
        assert_eq!(
            color.base,
            ResolvedThemeColorBase::Token(program.theme().native_tokens.text)
        );
        let ResolvedCanvasCommand::For {
            id, item, commands, ..
        } = &canvas.commands[1]
        else {
            panic!("second command must be a normalized loop");
        };
        assert_eq!(id.index, 1);
        assert_eq!(item.name, "value");
        assert_eq!(item.ty, Type::F64);
        assert!(matches!(
            commands.as_slice(),
            [ResolvedCanvasCommand::Circle { id, .. }] if id.index == 2
        ));
        assert_eq!(canvas.events.len(), 2);
        assert_eq!(canvas.events[0].id.index, 0);
        assert!(matches!(
            canvas.events[0].action,
            Some(ResolvedCanvasEventAction::Redraw { after_ms: None })
        ));
        let Some(ResolvedCanvasEventAction::Route(route)) = &canvas.events[1].action else {
            panic!("release event must have a normalized route");
        };
        assert_eq!(route.id.index, 0);
        assert!(matches!(
            route.target,
            ResolvedCanvasRouteTarget::Handler(HandlerId(0))
        ));
        assert!(matches!(
            route.args.as_slice(),
            [ResolvedCanvasRouteArg::Expression(_)]
        ));
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
            .find(|call| matches!(call.scope, ComponentScope::Explicit))
            .unwrap();
        assert!(matches!(
            root.arguments[0].writable,
            Some(WritableStateRef::App { .. })
        ));
        assert!(matches!(root.events[0], ResolvedEventRoute::Direct { .. }));
        assert!(matches!(root.scope, ComponentScope::Explicit));
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
    fn component_slot_contracts_reject_post_check_source_name_and_optional_swaps() {
        let source = format!(
            "app ComponentSlotSourcePoison\n{THEME}component Panel()\n  col\n    slot First\n    slot Middle?\n    slot Last?\nview\n  Panel\n    First:\n      text \"body\"\n"
        );

        let checked = analyze(&source).unwrap();
        let slots = checked.facts.component_slots(ComponentId(0)).unwrap();
        assert_eq!(checked.facts.metrics().component_slots, 3);
        assert_eq!(
            slots
                .iter()
                .map(|slot| (slot.id.index, slot.name.as_str(), slot.optional))
                .collect::<Vec<_>>(),
            vec![(0, "First", false), (1, "Middle", true), (2, "Last", true)]
        );
        for slot in slots {
            assert_eq!(checked.facts.component_slot_for_view(slot.view), Some(slot));
        }
        assert!(
            checked
                .facts
                .structural_snapshot()
                .contains("component-slot ComponentSlotId { component: ComponentId(0), index: 2 }")
        );

        let mut swapped_names = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut swapped_names.document.components[0].root
        else {
            panic!("fixture component root must be a layout");
        };
        let [
            ViewNode::Slot {
                name: first_name, ..
            },
            _,
            ViewNode::Slot {
                name: last_name, ..
            },
        ] = children.as_mut_slice()
        else {
            panic!("fixture component must contain three slots");
        };
        std::mem::swap(first_name, last_name);
        let error = lower(swapped_names).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("source diverged"));

        let mut flipped_last_optional = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } =
            &mut flipped_last_optional.document.components[0].root
        else {
            panic!("fixture component root must be a layout");
        };
        let ViewNode::Slot { optional, .. } = children.last_mut().unwrap() else {
            panic!("fixture last child must be a slot");
        };
        *optional = false;
        let error = lower(flipped_last_optional).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("source diverged"));
    }

    #[test]
    fn component_slot_contracts_reject_n_minus_one_n_plus_one_order_and_last_corruption() {
        let source = format!(
            "app ComponentSlotContractPoison\n{THEME}component Panel()\n  col\n    slot First\n    slot Middle?\n    slot Last?\nview\n  Panel\n    First:\n      text \"body\"\n"
        );
        let component = ComponentId(0);

        let mut n_minus_one = analyze(&source).unwrap();
        n_minus_one.facts.remove_component_slot(component, 2);
        let error = lower(n_minus_one).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("cardinality diverged"));

        let mut n_plus_one = analyze(&source).unwrap();
        n_plus_one.facts.duplicate_component_slot(component, 2);
        let error = lower(n_plus_one).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("cardinality diverged"));

        let mut wrong_order = analyze(&source).unwrap();
        wrong_order.facts.swap_component_slots(component, 0, 2);
        let error = lower(wrong_order).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("association is inconsistent"));

        let mut corrupt_last = analyze(&source).unwrap();
        corrupt_last
            .facts
            .corrupt_component_slot_id(component, 2, u32::MAX);
        let error = lower(corrupt_last).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("association is inconsistent"));

        let mut invalid_last_origin = analyze(&source).unwrap();
        let authoritative_last = invalid_last_origin
            .declarations
            .try_component_slot(ComponentSlotId {
                component,
                index: 2,
            })
            .unwrap()
            .origin;
        let authoritative_last_line = invalid_last_origin.origins.get(authoritative_last).line;
        invalid_last_origin
            .facts
            .corrupt_component_slot_origin(component, 2, u32::MAX);
        let error = lower(invalid_last_origin).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("association is inconsistent"));
        assert_eq!(error.line, authoritative_last_line);
    }

    #[test]
    fn component_slot_contracts_reject_stable_view_and_reverse_association_corruption() {
        let source = format!(
            "app ComponentSlotViewPoison\n{THEME}component Panel()\n  col\n    slot First\n    slot Middle?\n    slot Last?\nview\n  Panel\n    First:\n      text \"body\"\n"
        );
        let component = ComponentId(0);

        let mut valid_view_swap = analyze(&source).unwrap();
        valid_view_swap
            .facts
            .swap_component_slot_views(component, 0, 2);
        valid_view_swap
            .facts
            .swap_component_slot_reverse_associations(component, 0, 2);
        let error = lower(valid_view_swap).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("association is inconsistent"));

        let mut invalid_view = analyze(&source).unwrap();
        invalid_view
            .facts
            .corrupt_component_slot_view(component, 2, u32::MAX);
        let error = lower(invalid_view).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("association is inconsistent"));

        let mut valid_reverse_swap = analyze(&source).unwrap();
        valid_reverse_swap
            .facts
            .swap_component_slot_reverse_associations(component, 0, 2);
        let error = lower(valid_reverse_swap).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("association is inconsistent"));

        let mut invalid_reverse = analyze(&source).unwrap();
        invalid_reverse
            .facts
            .corrupt_component_slot_reverse_association(
                component,
                2,
                ComponentSlotId {
                    component,
                    index: u32::MAX,
                },
            );
        let error = lower(invalid_reverse).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("association is inconsistent"));
    }

    #[test]
    fn imported_component_slot_contract_diagnostics_keep_last_slot_source_origin() {
        let directory = tempfile::Builder::new()
            .prefix("ui-lang-component-slot-hir-origins-")
            .tempdir_in(std::env::current_dir().unwrap())
            .unwrap();
        let root = directory.path().join("app.ice");
        let imported = directory.path().join("slots.ice");
        fs::write(
            &root,
            format!(
                "app ImportedComponentSlots\nuse \"slots.ice\"\n{THEME}view\n  Imported\n    First:\n      text \"body\"\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Imported()\n  col\n    slot First\n    slot Last?\n",
        )
        .unwrap();

        let checked = analyze_file(&root).unwrap();
        let program = lower(checked).unwrap();
        let contract = &program.components[0];
        assert_eq!(contract.slots.len(), 2);
        assert_eq!(contract.slots[0].id.index, 0);
        assert_eq!(contract.slots[1].id.index, 1);
        assert_eq!(contract.slots[1].name, "Last");
        assert!(contract.slots[1].optional);
        let last_origin = program.origin(contract.slots[1].origin);
        assert_eq!(last_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(last_origin.line, 4);
        assert_eq!(last_origin.parent, Some(contract.origin));

        let mut sibling_origin = analyze_file(&root).unwrap();
        sibling_origin
            .facts
            .swap_component_slot_origins(ComponentId(0), 0, 1);
        let error = lower(sibling_origin).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);
        assert_eq!(error.column, 1);

        let mut invalid_origin = analyze_file(&root).unwrap();
        invalid_origin
            .facts
            .corrupt_component_slot_origin(ComponentId(0), 1, u32::MAX);
        let error = lower(invalid_origin).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);
        assert_eq!(error.column, 1);

        let mut poisoned = analyze_file(&root).unwrap();
        let ViewNode::Layout { children, .. } = &mut poisoned.document.components[0].root else {
            panic!("fixture component root must be a layout");
        };
        let ViewNode::Slot { optional, .. } = children.last_mut().unwrap() else {
            panic!("fixture last child must be a slot");
        };
        *optional = false;
        let error = lower(poisoned).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);
        assert_eq!(error.column, 1);
    }

    #[test]
    #[ignore = "large checked component-slot association and lowering performance contract"]
    fn performance_contract_four_thousand_component_slots_stay_linear() {
        const SLOTS: usize = 4_000;
        let mut source = format!("app ComponentSlotScale\n{THEME}component Wide()\n  col\n");
        for index in 0..SLOTS {
            writeln!(source, "    slot Slot{index}?").unwrap();
        }
        source.push_str("view\n  Wide\n");

        let checker_started = Instant::now();
        let checked = analyze(&source).unwrap();
        let checker_elapsed = checker_started.elapsed();
        let slot_index_elapsed = checked.facts.component_slot_index_elapsed();
        assert_eq!(checked.facts.metrics().component_slots, SLOTS);
        assert_eq!(checked.facts.metrics().component_slot_index_visits, SLOTS);
        assert!(
            slot_index_elapsed < Duration::from_millis(250),
            "4k component slots indexed into checked HIR in {slot_index_elapsed:?}"
        );
        assert!(
            checker_elapsed < Duration::from_secs(2),
            "4k component slots checked in {checker_elapsed:?}"
        );
        checked.facts.reset_lookup_count();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.components[0].slots.len(), SLOTS);
        assert_eq!(program.calls[0].slots.len(), SLOTS);
        let lookups = program.checked_facts().lookup_count();
        assert!(
            lookups <= SLOTS * 8 + 100,
            "4k component slots required {lookups} checked-HIR lookups"
        );
        eprintln!(
            "4k component slots indexed in {slot_index_elapsed:?}, checked in {checker_elapsed:?}, and lowered with {lookups} lookups in {elapsed:?}"
        );
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k checked component slots lowered in {elapsed:?}"
        );
    }

    #[test]
    fn component_call_routes_ignore_dynamic_and_post_lowering_raw_poison() {
        let source = format!(
            "app ComponentRoutePoison\n{THEME}state\n  suffix = \"checked\"\non accepted(value)\n  suffix = value\non observed(label, count)\n  suffix = label\ncomponent Routed() -> str\n  emits\n    changed(str, i64)\n  col\n    button \"Output\" -> emit(\"output\")\n    button \"Event\" -> emit(changed, \"event\", 1)\nview\n  Routed -> accepted suffix\n    events\n      changed -> observed suffix _\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "component-route-poison.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Component { route, events, .. } = &mut checked.document.view else {
            panic!("fixture root must be a component call");
        };
        let output = route.as_mut().unwrap();
        output.args[0] = RouteArg::Expr(Expr::Str("poison-output".into()));
        let event = events[0].route.as_mut().unwrap();
        event.args[0] = RouteArg::Expr(Expr::Str("poison-event".into()));
        let mut program = lower(checked).unwrap();
        let checked_poison =
            crate::codegen::generate(&program, "component-route-poison.ice").unwrap();
        assert_eq!(checked_poison, expected);
        assert!(!checked_poison.contains("poison-output"));
        assert!(!checked_poison.contains("poison-event"));

        let ViewNode::Component {
            name,
            route,
            events,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be a component call");
        };
        *name = "RawPoison".into();
        let output = route.as_mut().unwrap();
        output.handler = "raw_poison".into();
        output.args[0] = RouteArg::Payload;
        events[0].name = "raw_event".into();
        let event = events[0].route.as_mut().unwrap();
        event.handler = "raw_poison".into();
        event.args.clear();
        let lowered_poison =
            crate::codegen::generate(&program, "component-route-poison.ice").unwrap();
        assert_eq!(lowered_poison, expected);

        let mut handler_poison = analyze(&source).unwrap();
        let ViewNode::Component { route, .. } = &mut handler_poison.document.view else {
            panic!("fixture root must be a component call");
        };
        route.as_mut().unwrap().handler = "observed".into();
        let error = lower(handler_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("handler contract diverged"));

        let mut argument_poison = analyze(&source).unwrap();
        let ViewNode::Component { route, .. } = &mut argument_poison.document.view else {
            panic!("fixture root must be a component call");
        };
        route.as_mut().unwrap().args[0] = RouteArg::Payload;
        let error = lower(argument_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("argument kind diverged"));

        let mut cardinality_poison = analyze(&source).unwrap();
        let ViewNode::Component { route, .. } = &mut cardinality_poison.document.view else {
            panic!("fixture root must be a component call");
        };
        route
            .as_mut()
            .unwrap()
            .args
            .push(RouteArg::Expr(Expr::Str("extra".into())));
        let error = lower(cardinality_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("arity diverged"));

        let mut topology_poison = analyze(&source).unwrap();
        let ViewNode::Component { events, .. } = &mut topology_poison.document.view else {
            panic!("fixture root must be a component call");
        };
        events[0].route = None;
        let error = lower(topology_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn component_call_routes_follow_slotted_interaction_analysis_order() {
        let source = format!(
            "app ComponentRouteSlots\n{THEME}state\n  label = \"ready\"\non accepted(value)\n  label = value\ncomponent Routed() -> str\n  emits\n    changed(str)\n  col\n    slot Body\n    button \"Output\" -> emit(\"output\")\n    button \"Event\" -> emit(changed, \"event\")\nview\n  Routed -> accepted _\n    events\n      changed -> accepted _\n    Body:\n      button \"Nested\" -> accepted \"nested\"\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let generated = crate::codegen::generate(&program, "component-route-slots.ice").unwrap();
        assert!(generated.contains("Nested"));
        assert!(generated.contains("ComponentRouteSlots"));
    }

    #[test]
    fn component_call_routes_reject_expression_count_and_last_graph_corruption() {
        let source = format!(
            "app ComponentRouteExpressionCardinality\n{THEME}state\n  active = false\non output(next)\n  active = next\non event(next)\n  active = next\ncomponent Routed() -> bool\n  emits\n    changed(bool)\n  col\n    button \"Output\" -> emit(true)\n    button \"Event\" -> emit(changed, true)\nview\n  Routed -> output !active\n    events\n      changed -> event !active\n"
        );
        for count in [1, 3] {
            let mut checked = analyze(&source).unwrap();
            let view = checked
                .declarations
                .view_id(checked.document.view.span())
                .unwrap();
            checked
                .facts
                .corrupt_interaction_expression_count(view, count);
            let error = lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(
                error.message.contains("expression cardinality diverged"),
                "unexpected expression-count diagnostic: {}",
                error.message
            );
        }

        let mut invalid_last_graph = analyze(&source).unwrap();
        let view = invalid_last_graph
            .declarations
            .view_id(invalid_last_graph.document.view.span())
            .unwrap();
        invalid_last_graph.facts.corrupt_expression_first_child(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: view,
                index: 1,
            }),
            u32::MAX,
        );
        let error = lower(invalid_last_graph).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("descendant ID"));
    }

    #[test]
    fn component_call_routes_reject_sibling_route_origin_swaps() {
        let source = format!(
            "app ComponentRouteOriginSwap\n{THEME}state\n  label = \"ready\"\non accepted(value)\n  label = value\ncomponent Routed() -> str\n  emits\n    changed(str)\n  col\n    button \"Output\" -> emit(\"output\")\n    button \"Event\" -> emit(changed, \"event\")\nview\n  Routed -> accepted \"output\"\n    events\n      changed -> accepted \"event\"\n"
        );
        let call_id = |checked: &crate::CheckedDocument| {
            let view = checked
                .declarations
                .view_id(checked.document.view.span())
                .unwrap();
            (view, checked.declarations.component_call_id(view).unwrap())
        };

        let mut interaction_only = analyze(&source).unwrap();
        let (view, _) = call_id(&interaction_only);
        interaction_only
            .facts
            .swap_interaction_route_origins(view, 0, 1);
        let error = lower(interaction_only).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));

        let mut fully_swapped = analyze(&source).unwrap();
        let (view, call) = call_id(&fully_swapped);
        fully_swapped
            .facts
            .swap_interaction_route_origins(view, 0, 1);
        fully_swapped
            .facts
            .swap_component_call_direct_route_origins(call, 0);
        let error = lower(fully_swapped).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("physical origin diverged"));
    }

    #[test]
    fn component_call_routes_reject_cross_owner_id_type_cardinality_and_origin_corruption() {
        let source = format!(
            "app ComponentRouteIdentity\n{THEME}state\n  label = \"label\"\non first(value)\n  label = value\non second(value)\n  label = value\ncomponent Routed() -> str\n  emits\n    changed(str)\n  col\n    button \"Output\" -> emit(\"output\")\n    button \"Event\" -> emit(changed, \"event\")\ncomponent Alternate() -> str\n  emits\n    changed(str)\n  col\n    button \"Output\" -> emit(\"output\")\n    button \"Event\" -> emit(changed, \"event\")\nview\n  col\n    Routed -> first label\n      events\n        changed -> first label\n    Routed -> second label\n      events\n        changed -> second label\n"
        );
        let ids = |checked: &crate::CheckedDocument| {
            let ViewNode::Layout { children, .. } = &checked.document.view else {
                panic!("fixture root must be a layout");
            };
            let views = children
                .iter()
                .map(|child| checked.declarations.view_id(child.span()).unwrap())
                .collect::<Vec<_>>();
            let calls = views
                .iter()
                .map(|view| checked.declarations.component_call_id(*view).unwrap())
                .collect::<Vec<_>>();
            (views, calls)
        };

        let mut cross_owner = analyze(&source).unwrap();
        let (views, _) = ids(&cross_owner);
        cross_owner
            .facts
            .transplant_interaction_route_expression_across_views(views[0], 0, 0, views[1], 0, 0);
        let error = lower(cross_owner).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("slot identity diverged"));

        let mut route_id = analyze(&source).unwrap();
        let (views, _) = ids(&route_id);
        route_id.facts.swap_interaction_routes(views[0], views[1]);
        let error = lower(route_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route ID diverged"));

        let mut handler_id = analyze(&source).unwrap();
        let (views, _) = ids(&handler_id);
        handler_id
            .facts
            .corrupt_interaction_route_handler(views[0], 0, HandlerId(1));
        let error = lower(handler_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("handler contract diverged"));

        let mut component_id = analyze(&source).unwrap();
        let (_, calls) = ids(&component_id);
        component_id
            .facts
            .corrupt_component_call_route_component(calls[0], ComponentId(1));
        let error = lower(component_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route identity diverged"));

        let mut event_id = analyze(&source).unwrap();
        let (_, calls) = ids(&event_id);
        event_id.facts.corrupt_component_call_event_id(
            calls[0],
            0,
            ComponentEventId {
                component: ComponentId(1),
                index: 0,
            },
        );
        let error = lower(event_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("event route contract diverged"));

        let mut payload_type = analyze(&source).unwrap();
        let (views, _) = ids(&payload_type);
        payload_type
            .facts
            .corrupt_interaction_route_source_payload(views[0], 0, 0, Type::Bool);
        let error = lower(payload_type).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("source payload contract diverged"));

        let mut view_id = analyze(&source).unwrap();
        let (_, calls) = ids(&view_id);
        view_id
            .facts
            .corrupt_component_call_route_view(calls[0], u32::MAX);
        let error = lower(view_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("route identity diverged"));

        let mut origin = analyze(&source).unwrap();
        let (_, calls) = ids(&origin);
        origin
            .facts
            .corrupt_component_call_event_origin(calls[0], 0, u32::MAX);
        let error = lower(origin).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("origin is invalid"));
    }

    #[test]
    fn imported_forward_routes_validate_stable_outer_contracts_and_callback_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-component-forward-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("forward.ice");
        fs::write(
            &root,
            format!(
                "app ImportedComponentForward\nuse \"forward.ice\"\n{THEME}state\n  label = \"app\"\non accepted(value)\n  label = value\nview\n  Outer\n    events\n      changed -> accepted _\n      renamed -> accepted _\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Leaf()\n  emits\n    changed(str)\n    renamed(str)\n  button \"Emit\" -> emit(changed, \"leaf\")\ncomponent Outer()\n  emits\n    changed(str)\n    renamed(str)\n  Leaf\n    forward\n      changed\n      renamed\ncomponent Mirror()\n  emits\n    changed(str)\n  space\n",
        )
        .unwrap();

        let forward_call = |checked: &crate::CheckedDocument| {
            let view = checked
                .declarations
                .view_id(checked.document.components[1].root.span())
                .unwrap();
            checked.declarations.component_call_id(view).unwrap()
        };
        fn forwarded_event(program: &mut LoweredProgram) -> &mut ResolvedEventRoute {
            program
                .calls
                .iter_mut()
                .flat_map(|call| &mut call.events)
                .find(|event| matches!(event, ResolvedEventRoute::Forward { .. }))
                .unwrap()
        }
        fn forwarded_events(program: &mut LoweredProgram) -> &mut Vec<ResolvedEventRoute> {
            &mut program
                .calls
                .iter_mut()
                .find(|call| {
                    call.events.len() >= 2
                        && call
                            .events
                            .iter()
                            .all(|event| matches!(event, ResolvedEventRoute::Forward { .. }))
                })
                .unwrap()
                .events
        }

        let mut invalid_component = analyze_file(&root).unwrap();
        let call = forward_call(&invalid_component);
        invalid_component
            .facts
            .corrupt_component_call_forward_outer_component(call, 0, ComponentId(u32::MAX));
        let error = lower(invalid_component).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);

        let mut invalid_event = analyze_file(&root).unwrap();
        let call = forward_call(&invalid_event);
        invalid_event
            .facts
            .corrupt_component_call_forward_outer_event(
                call,
                0,
                ComponentEventId {
                    component: ComponentId(1),
                    index: u32::MAX,
                },
            );
        let error = lower(invalid_event).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);

        let mut invalid_component_name = analyze_file(&root).unwrap();
        let call = forward_call(&invalid_component_name);
        invalid_component_name
            .facts
            .corrupt_component_call_forward_outer_component_name(call, 0, "poisoned");
        let error = lower(invalid_component_name).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);

        let mut invalid_event_name = analyze_file(&root).unwrap();
        let call = forward_call(&invalid_event_name);
        invalid_event_name
            .facts
            .corrupt_component_call_forward_outer_event_name(call, 0, "poisoned");
        let error = lower(invalid_event_name).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);

        let mut invalid_payload = analyze_file(&root).unwrap();
        let call = forward_call(&invalid_payload);
        invalid_payload
            .facts
            .corrupt_component_call_forward_outer_payload(call, 0, Type::Bool);
        let error = lower(invalid_payload).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);

        let mut invalid_lowered_id = lower(analyze_file(&root).unwrap()).unwrap();
        let forward = invalid_lowered_id
            .calls
            .iter_mut()
            .find_map(|call| match call.events.first_mut() {
                Some(ResolvedEventRoute::Forward {
                    outer_component, ..
                }) => Some(outer_component),
                Some(ResolvedEventRoute::Direct { .. }) | None => None,
            })
            .unwrap();
        *forward = ComponentId(u32::MAX);
        let error =
            crate::codegen::generate(&invalid_lowered_id, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);

        let mut invalid_callee_event = lower(analyze_file(&root).unwrap()).unwrap();
        let ResolvedEventRoute::Forward { event, .. } = forwarded_event(&mut invalid_callee_event)
        else {
            unreachable!();
        };
        *event = ComponentEventId {
            component: ComponentId(0),
            index: u32::MAX,
        };
        let error =
            crate::codegen::generate(&invalid_callee_event, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid callee contract"));
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);
        assert_eq!(error.column, 1);

        let mut invalid_forward_origin = lower(analyze_file(&root).unwrap()).unwrap();
        let ResolvedEventRoute::Forward { origin, .. } =
            forwarded_event(&mut invalid_forward_origin)
        else {
            unreachable!();
        };
        *origin = OriginId(u32::MAX);
        let error =
            crate::codegen::generate(&invalid_forward_origin, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid callee contract"));
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        // A corrupted event-origin ID cannot map itself, so the normalized
        // call origin is the stable source-mapped fallback.
        assert_eq!(error.line, 10);
        assert_eq!(error.column, 1);

        let mut invalid_callee_name = lower(analyze_file(&root).unwrap()).unwrap();
        let ResolvedEventRoute::Forward { name, .. } = forwarded_event(&mut invalid_callee_name)
        else {
            unreachable!();
        };
        *name = "poisoned".into();
        let error =
            crate::codegen::generate(&invalid_callee_name, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid callee contract"));
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);
        assert_eq!(error.column, 1);

        let mut invalid_callee_payloads = lower(analyze_file(&root).unwrap()).unwrap();
        let ResolvedEventRoute::Forward { payloads, .. } =
            forwarded_event(&mut invalid_callee_payloads)
        else {
            unreachable!();
        };
        *payloads = vec![Type::Bool];
        let error =
            crate::codegen::generate(&invalid_callee_payloads, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid callee contract"));
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);
        assert_eq!(error.column, 1);

        let mut invalid_event_order = lower(analyze_file(&root).unwrap()).unwrap();
        forwarded_events(&mut invalid_event_order).swap(0, 1);
        let error =
            crate::codegen::generate(&invalid_event_order, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid callee contract"));
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);
        assert_eq!(error.column, 1);

        let mut missing_callback = lower(analyze_file(&root).unwrap()).unwrap();
        let forward = missing_callback
            .calls
            .iter_mut()
            .find_map(|call| match call.events.first_mut() {
                Some(ResolvedEventRoute::Forward {
                    outer_component,
                    outer_component_name,
                    outer_event,
                    outer_event_name,
                    outer_payloads,
                    ..
                }) => Some((
                    outer_component,
                    outer_component_name,
                    outer_event,
                    outer_event_name,
                    outer_payloads,
                )),
                Some(ResolvedEventRoute::Direct { .. }) | None => None,
            })
            .unwrap();
        let (outer_component, outer_component_name, outer_event, outer_event_name, outer_payloads) =
            forward;
        *outer_component = ComponentId(2);
        *outer_component_name = "Mirror".into();
        *outer_event = ComponentEventId {
            component: ComponentId(2),
            index: 0,
        };
        *outer_event_name = "changed".into();
        *outer_payloads = vec![Type::Str];
        let error =
            crate::codegen::generate(&missing_callback, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("absent from component context"));
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 12);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_component_call_routes_keep_origins_markers_and_hir_diagnostics() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-component-route-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("routes.ice");
        fs::write(
            &root,
            format!(
                "app ImportedComponentRoutes\nuse \"routes.ice\"\n{THEME}state\n  label = \"app\"\non accepted(value)\n  label = value\nview\n  Imported value=label -> accepted _\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Leaf() -> str\n  emits\n    changed(str)\n  col\n    button \"Output\" -> emit(\"leaf\")\n    button \"Event\" -> emit(changed, \"leaf\")\ncomponent Imported(value:str) -> str\n  Leaf -> emit(value)\n    events\n      changed -> emit(value)\n",
        )
        .unwrap();

        let mut swapped_origins = analyze_file(&root).unwrap();
        let imported_call_view = swapped_origins
            .declarations
            .view_id(swapped_origins.document.components[1].root.span())
            .unwrap();
        let imported_call = swapped_origins
            .declarations
            .component_call_id(imported_call_view)
            .unwrap();
        swapped_origins
            .facts
            .swap_interaction_route_origins(imported_call_view, 0, 1);
        swapped_origins
            .facts
            .swap_component_call_direct_route_origins(imported_call, 0);
        let error = lower(swapped_origins).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("physical origin diverged"));
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 8);
        assert_eq!(error.column, 1);

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let call_index = program
            .calls
            .iter()
            .position(|call| call.component == ComponentId(0))
            .unwrap();
        let call = &program.calls[call_index];
        let origin = program.origin(call.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 8);
        let ComponentOutputRoute::Direct { route, .. } = &call.output else {
            panic!("imported call must have a direct output route");
        };
        let output_origin = program.origin(route.origin);
        assert_eq!(output_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(output_origin.line, 8);
        assert_eq!(output_origin.column, 1);
        assert_eq!(output_origin.parent, Some(call.origin));
        let ResolvedEventRoute::Direct { route, origin, .. } = &call.events[0] else {
            panic!("imported call must have a direct event route");
        };
        let event_origin = program.origin(*origin);
        assert_eq!(event_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(event_origin.line, 10);
        assert_eq!(event_origin.column, 1);
        assert_eq!(event_origin.parent, Some(call.origin));
        let event_route_origin = program.origin(route.origin);
        assert_eq!(event_route_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(event_route_origin.line, 10);
        assert_eq!(event_route_origin.column, 1);
        assert_eq!(event_route_origin.parent, Some(call.origin));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 8 1 {encoded_import}")));

        let ResolvedEventRoute::Direct { route, .. } = &mut program.calls[call_index].events[0]
        else {
            panic!("imported call must have a direct event route");
        };
        route.target = ResolvedInteractionRouteTarget::TargetHandler(HandlerId(u32::MAX));
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 10);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "large normalized component-call route lowering and emission performance contract"]
    fn performance_contract_four_thousand_component_call_routes_lower_and_emit_under_two_seconds() {
        const CALLS: usize = 4_000;
        let mut source = format!(
            "app ComponentRouteScale\n{THEME}state\n  count = 0\non accepted(value)\n  count = count + 1\ncomponent Emitter() -> str\n  button \"Emit\" -> emit(\"value\")\nview\n  col\n"
        );
        for _ in 0..CALLS {
            source.push_str("    Emitter -> accepted _\n");
        }
        let checked = analyze(&source).unwrap();
        assert_eq!(checked.facts.metrics().type_scope_env_full_clones, 0);
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "component-route-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.calls.len(), CALLS);
        assert_eq!(
            program
                .calls
                .iter()
                .filter(|call| matches!(call.output, ComponentOutputRoute::Direct { .. }))
                .count(),
            CALLS
        );
        // Body-identical component calls fold into one outlined definition;
        // every call site still routes through it with its own use scope.
        assert_eq!(generated.matches("let __component_content").count(), 1);
        assert_eq!(
            generated.matches("self.__ice_component_use_0(").count(),
            CALLS
        );
        eprintln!("4k normalized component-call routes lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized component-call routes lowered and emitted in {elapsed:?}"
        );
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
            "app Initializers\nextern crate::backend\n  pure elastic(value:f64) -> f64\n{THEME}state\n  progress:animation[f64] = 0.0\n    easing elastic\n    duration 120ms\n    delay 5ms\n    repeat 2\n    auto-reverse true\nderived\n  total = 1.0 + 2.0\ncomponent Meter(label:str=\"ready\")\n  state\n    open = false\n  text label\nview\n  Meter\n"
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
            "app AppStateId(0) progress Animation(F64) use=ExpressionId(0) ValueToAnimation { value: F64 } line=15 animation=Some(ResolvedAnimation { easing: Some(Custom(ExternFnId(0))), duration: Some(Milliseconds(120)), delay_ms: Some(5), repeat: Some(2), repeat_forever: false, auto_reverse: true, from: None })\n\
             derived DerivedId(0) total F64 use=ExpressionId(1) None line=22\n\
             default ComponentParamId { component: ComponentId(0), index: 0 } label Str use=ExpressionId(2) None line=23\n\
             component-state ComponentStateId { component: ComponentId(0), index: 0 } open Bool use=ExpressionId(3) None line=25 animation=None\n"
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
  pure describe(id:window-id) -> str
  pure scale_for(id:window-id) -> f64
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
                generated.contains(&format!("self.__ice_derived_{name}()")),
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
    fn qualified_builtin_contract_does_not_resolve_a_same_name_pure_extern() {
        let source = format!(
            "app QualifiedBuiltin\n  title builtin::trim(title)\nextern crate::backend\n  pure trim(value:str) -> bool\n{THEME}state\n  title = \" Title \"\nview\n  text title\n"
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

    /// The literal `tooltip` is hoisted to startup and the call in `label`
    /// is not, which is the whole of the reactivity contract in one program.
    #[test]
    fn lowers_tray_settings_with_guarded_icons_and_a_menu() {
        let source = format!(
            "daemon TrayDemo\n  tray\n    icon-rgba \"assets/alarm.rgba\" 2 2 when count > 0\n    icon-rgba \"assets/tray.rgba\" 2 2\n    icon-template true\n    label describe(count)\n    tooltip \"Demo\"\n    menu\n      describe(count)\n      separator\n      \"Quit\" -> quit\nextern crate::backend\n  pure describe(value:i64) -> str\nstate\n  count = 1\non quit\n  count = 0\n{THEME}view\n  text count\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let settings = program.settings();
        let tray = settings.tray.as_ref().unwrap();
        assert_eq!(
            tray.icons
                .iter()
                .map(|icon| (icon.icon.path.as_str(), icon.when.is_some()))
                .collect::<Vec<_>>(),
            [("assets/alarm.rgba", true), ("assets/tray.rgba", false)]
        );
        assert_eq!(tray.icons[1].icon.byte_len, 16);
        assert!(tray.icon_template);
        assert!(matches!(tray.label, Some(ResolvedTrayText::Expression(_))));
        assert!(matches!(
            tray.tooltip,
            Some(ResolvedTrayText::Literal(ref value)) if value == "Demo"
        ));
        assert!(matches!(
            tray.menu.as_slice(),
            [
                ResolvedTrayRow::Item {
                    text: ResolvedTrayText::Expression(_),
                    route: None,
                },
                ResolvedTrayRow::Separator,
                ResolvedTrayRow::Item {
                    text: ResolvedTrayText::Literal(_),
                    route: Some(route),
                },
            ] if route == "quit"
        ));
        assert!(tray.reactive());
    }

    #[test]
    fn rejects_tray_topology_mutations_with_e196() {
        let source = format!(
            "daemon TrayDemo\n  tray\n    icon-rgba \"assets/tray.rgba\" 2 2\n    menu\n      \"Quit\" -> quit\non quit\n  exit\n{THEME}view\n  text \"ready\"\n"
        );

        // A route that lost its handler would reach codegen as a message
        // variant nobody declared.
        let mut checked = analyze(&source).unwrap();
        let TrayRow::Item { route, .. } =
            &mut checked.document.settings.tray.as_mut().unwrap().menu[0]
        else {
            panic!("the first menu row is an item");
        };
        *route = Some("missing".into());
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");

        // The last icon carries no guard, which is what makes the selection
        // codegen emits total.
        let mut checked = analyze(&source).unwrap();
        checked.document.settings.tray.as_mut().unwrap().icons[0].when = Some(AppExpression {
            value: Expr::Bool(true),
            span: Span::line(3),
        });
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");

        let mut checked = analyze(&source).unwrap();
        checked.document.settings.tray = None;
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
    }

    /// A tray-only drift has to point at the tray line that changed. Before
    /// the tray joined `first_changed_static_setting_span`, the fallback
    /// chain reported it against the default font or the app header.
    #[test]
    fn reports_a_tray_mutation_at_the_tray_setting_that_changed() {
        let source = format!(
            "daemon TrayDemo\n  tray\n    icon-rgba \"assets/tray.rgba\" 2 2\n    icon-template true\n    menu\n      \"Quit\" -> quit\non quit\n  exit\nfont brand family=serif default=true\n{THEME}view\n  text \"ready\"\n"
        );

        let mut checked = analyze(&source).unwrap();
        checked
            .document
            .settings
            .tray
            .as_mut()
            .unwrap()
            .menu
            .push(TrayRow::Separator {
                span: Span::line(6),
            });
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, 5);

        let mut checked = analyze(&source).unwrap();
        checked
            .document
            .settings
            .tray
            .as_mut()
            .unwrap()
            .icon_template = Some(false);
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, 4);
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
        let ResolvedViewKind::Layout { children } = &program
            .resolved_view(program.components[0].root)
            .unwrap()
            .kind
        else {
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
        let nonce = crate::test_support::unique_nonce();
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
        let nested = program.nested_theme(ViewId(0)).unwrap();
        let ResolvedThemePreset::Factory(factory) = &nested.preset else {
            panic!("nested theme factory must be resolved");
        };
        assert_eq!(factory.function.name, "native_theme");
        assert_eq!(factory.function.rust_path, "crate::backend::native_theme");
        assert_eq!(factory.arguments.len(), 1);
        assert_eq!(program.origin(factory.origin).parent, Some(nested.origin));
        assert_eq!(
            program
                .checked_facts()
                .expression_use(factory.arguments[0].expression)
                .owner,
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: ViewId(0),
                index: 0,
            })
        );
        assert!(matches!(
            nested.text,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 1, .. }),
                ..
            })
        ));
        let ResolvedBackground::Linear { angle, stops } = &nested.background.as_ref().unwrap()
        else {
            panic!("nested gradient must be normalized");
        };
        assert_eq!(stops.len(), 2);
        for (index, expression) in std::iter::once(*angle)
            .chain(stops.iter().map(|stop| stop.offset))
            .enumerate()
        {
            let expression = program.checked_facts().expression_use(expression);
            assert_eq!(expression.source, Type::F64);
            assert_eq!(expression.destination, Type::F64);
            assert_eq!(
                expression.owner,
                CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: ViewId(0),
                    index: index as u32 + 1,
                })
            );
        }
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
    fn nested_theme_post_check_and_post_lowering_raw_poison_is_contained() {
        let source = format!(
            "app NestedThemePoison\nextern crate::backend\n  theme primary_theme(value:f64)\n  theme poisoned_theme(value:f64)\n{THEME}state\n  value = 0.5\nview\n  theme primary_theme(value) fg=fg bg=linear(value, bg@value, primary@value)\n    text \"Content\"\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "nested-theme-poison.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Theme {
            preset, background, ..
        } = &mut checked.document.view
        else {
            panic!("fixture root must be a nested theme");
        };
        let ThemePreset::Factory(factory) = preset else {
            panic!("fixture must use a nested theme factory");
        };
        factory.args[0] = Expr::F64(91.0);
        let Some(BackgroundValue::Linear { angle, stops }) = background else {
            panic!("fixture must use a linear background");
        };
        *angle = Expr::F64(92.0);
        stops[0].offset = Expr::F64(93.0);
        stops[1].offset = Expr::F64(94.0);
        let mut program = lower(checked).unwrap();
        let checked_poison = crate::codegen::generate(&program, "nested-theme-poison.ice").unwrap();
        assert_eq!(checked_poison, expected);
        assert!(!checked_poison.contains("91.0"));
        assert!(!checked_poison.contains("92.0"));
        assert!(!checked_poison.contains("93.0"));
        assert!(!checked_poison.contains("94.0"));

        let ViewNode::Theme {
            preset,
            text,
            background,
            ..
        } = &mut program.document.view
        else {
            panic!("fixture root must be a nested theme");
        };
        *preset = ThemePreset::Factory(ExternCall {
            function: "poisoned_theme".into(),
            args: vec![Expr::F64(99.0)],
        });
        *text = Some("danger".into());
        *background = Some(BackgroundValue::Color("danger".into()));
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(1, 3);
        let lowered_poison = crate::codegen::generate(&program, "nested-theme-poison.ice").unwrap();
        assert_eq!(lowered_poison, expected);
        assert!(!lowered_poison.contains("99.0"));

        let mut static_poison = analyze(&source).unwrap();
        let ViewNode::Theme { text, .. } = &mut static_poison.document.view else {
            panic!("fixture root must be a nested theme");
        };
        *text = Some("danger".into());
        let error = lower(static_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));

        let mut factory_poison = analyze(&source).unwrap();
        let ViewNode::Theme { preset, .. } = &mut factory_poison.document.view else {
            panic!("fixture root must be a nested theme");
        };
        let ThemePreset::Factory(factory) = preset else {
            panic!("fixture must use a nested theme factory");
        };
        factory.function = "poisoned_theme".into();
        let error = lower(factory_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));

        let mut background_poison = analyze(&source).unwrap();
        let ViewNode::Theme { background, .. } = &mut background_poison.document.view else {
            panic!("fixture root must be a nested theme");
        };
        *background = Some(BackgroundValue::Color("bg".into()));
        let error = lower(background_poison).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn nested_theme_rejects_cross_owner_factory_identity_and_origin_corruption() {
        let source = format!(
            "app NestedThemeIdentity\nextern crate::backend\n  theme first_theme(value:f64)\n  theme second_theme(value:f64)\n{THEME}state\n  value = 0.5\nview\n  col\n    theme first_theme(value) bg=linear(value, bg@value, primary@value)\n      text \"First\"\n    theme second_theme(value) bg=linear(value, bg@value, primary@value)\n      text \"Second\"\n"
        );
        let first = ViewId(1);
        let second = ViewId(3);

        let mut transplanted = analyze(&source).unwrap();
        transplanted
            .facts
            .transplant_interaction_option_expression(first, 0, second, 0);
        let error = lower(transplanted).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("argument 0 contract diverged"));

        let mut reordered = analyze(&source).unwrap();
        reordered
            .facts
            .swap_interaction_option_expressions(first, 0, 1);
        let error = lower(reordered).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("argument 0 contract diverged"));

        let mut corrupt_factory = analyze(&source).unwrap();
        corrupt_factory
            .facts
            .corrupt_nested_theme_factory(first, ExternFnId(1));
        let error = lower(corrupt_factory).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("factory declaration contract diverged")
        );

        let mut corrupt_id = analyze(&source).unwrap();
        corrupt_id.facts.corrupt_nested_theme_id(first, u32::MAX);
        let error = lower(corrupt_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));

        let mut corrupt_origin = analyze(&source).unwrap();
        corrupt_origin
            .facts
            .corrupt_interaction_expression_origin(first, 0, u32::MAX);
        let error = lower(corrupt_origin).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("origin is invalid"));
    }

    #[test]
    fn nested_theme_rejects_expression_count_and_last_graph_corruption() {
        let source = format!(
            "app NestedThemeExpressionCardinality\nextern crate::backend\n  theme first_theme(value:f64)\n{THEME}state\n  value = 0.5\nview\n  theme first_theme(value) bg=linear(value, bg@value, primary@(value + 0.0))\n    text \"Content\"\n"
        );
        let theme = ViewId(0);

        let mut decreased = analyze(&source).unwrap();
        decreased
            .facts
            .corrupt_interaction_expression_count(theme, 3);
        let error = lower(decreased).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("cardinality diverged"));

        let mut invalid_last_graph = analyze(&source).unwrap();
        invalid_last_graph.facts.corrupt_expression_first_child(
            CheckedExprOwner::Interaction(InteractionExpressionId {
                widget: theme,
                index: 3,
            }),
            u32::MAX,
        );
        let error = lower(invalid_last_graph).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("descendant ID"));
    }

    #[test]
    fn imported_nested_theme_keeps_origins_markers_and_e196_paths() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-nested-theme-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("nested.ice");
        fs::write(
            &root,
            format!(
                "app ImportedNestedTheme\nuse \"nested.ice\"\n{THEME}state\n  value = 0.5\nview\n  ImportedTheme value=value\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "extern crate::backend\n  theme imported_theme(value:f64)\ncomponent ImportedTheme(value:f64)\n  theme imported_theme(value) fg=fg bg=linear(value, bg@value, primary@value)\n    text \"Imported\"\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let theme_id = program
            .declarations
            .view_id(program.document.components[0].root.span())
            .unwrap();
        let theme = program.nested_theme(theme_id).unwrap();
        let theme_origin = theme.origin;
        let origin = program.origin(theme_origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 4);
        let ResolvedThemePreset::Factory(factory) = &theme.preset else {
            panic!("imported nested theme must use a factory");
        };
        assert_eq!(factory.function.rust_path, "crate::backend::imported_theme");
        assert_eq!(
            program
                .origin(factory.function.declaration_origin)
                .path
                .as_deref(),
            Some(imported.as_path())
        );
        assert_eq!(program.origin(factory.function.declaration_origin).line, 2);
        assert_eq!(program.origin(factory.origin).parent, Some(theme_origin));
        for expression in factory
            .arguments
            .iter()
            .map(|argument| argument.expression)
            .chain(match &theme.background {
                Some(ResolvedBackground::Linear { angle, stops }) => std::iter::once(*angle)
                    .chain(stops.iter().map(|stop| stop.offset))
                    .collect::<Vec<_>>()
                    .into_iter(),
                _ => panic!("imported nested theme must use a gradient"),
            })
        {
            let expression = program.checked_facts().expression_use(expression);
            assert_eq!(program.origin(expression.origin).parent, Some(theme_origin));
            assert_eq!(
                program.origin(expression.origin).path.as_deref(),
                Some(imported.as_path())
            );
        }

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 4 1 {encoded_import}")));

        program.styles.corrupt_nested_theme_id(theme_id, u32::MAX);
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        let mut corrupt_checked = analyze_file(&root).unwrap();
        let theme_id = corrupt_checked
            .declarations
            .view_id(corrupt_checked.document.components[0].root.span())
            .unwrap();
        corrupt_checked
            .facts
            .corrupt_nested_theme_factory(theme_id, ExternFnId(u32::MAX));
        let error = lower(corrupt_checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "large normalized nested Theme lowering and emission performance contract"]
    fn performance_contract_four_thousand_nested_themes_lower_and_emit_under_two_seconds() {
        const THEMES: usize = 4_000;
        let mut source = format!(
            "app NestedThemeScale\nextern crate::backend\n  theme native_theme(value:f64)\n{THEME}state\n  value = 0.5\nview\n  col\n"
        );
        for _ in 0..THEMES {
            writeln!(
                source,
                "    theme native_theme(value) fg=fg bg=linear(value, bg@value, primary@value)\n      space"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        assert_eq!(checked.facts.metrics().type_scope_env_full_clones, 0);
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "nested-theme-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.styles.nested_theme_count(), THEMES);
        let expression_count = program
            .styles
            .nested_themes()
            .map(|theme| {
                let arguments = match &theme.preset {
                    ResolvedThemePreset::Factory(factory) => factory.arguments.len(),
                    _ => 0,
                };
                let background = match &theme.background {
                    Some(ResolvedBackground::Linear { stops, .. }) => 1 + stops.len(),
                    _ => 0,
                };
                arguments + background
            })
            .sum::<usize>();
        assert_eq!(expression_count, THEMES * 4);
        assert_eq!(
            generated
                .matches("::ui_lang_runtime::dynamic_themer(")
                .count(),
            THEMES
        );
        eprintln!("4k normalized nested Theme nodes lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized nested Theme nodes lowered and emitted in {elapsed:?}"
        );
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
        let theme_id = program
            .declarations
            .view_id(program.document.view.span())
            .unwrap();
        let nested = program.nested_theme(theme_id).unwrap();
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
        let nonce = crate::test_support::unique_nonce();
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
        let ViewNode::Theme { content, .. } = &program.document.view else {
            panic!("fixture root must be a nested theme");
        };
        let theme_id = program
            .declarations
            .view_id(program.document.view.span())
            .unwrap();
        let nested = program.nested_theme(theme_id).unwrap();
        let ResolvedThemePreset::Factory(factory) = &nested.preset else {
            panic!("namespaced nested theme factory must be resolved");
        };
        assert_eq!(factory.function.name, "ui::native_theme");
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
        let nonce = crate::test_support::unique_nonce();
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
            "extern crate::backend\n  pure describe(id:window-id) -> str\n  pure scale(id:window-id) -> f64\n  theme native_theme(id:window-id)\n",
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
        } = &title_root.kind
        else {
            panic!("title must retain its imported extern target ID");
        };
        assert_eq!(program.extern_function(function.id).name, "tools::describe");

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
    fn resolves_first_class_test_hir_and_ignores_raw_expression_mutations() {
        let source = format!(
            r#"app TestHir
{THEME}state
  label = "row"
  count = 0
on change(next)
  count = next
view
  col #root
    text label #row(label)
test stable_flow
  target row = #root/row(label)
  click #root/row(label)
  type label
  dispatch change(count + 1)
  expect count == 1
"#
        );
        let mut checked = analyze(&source).unwrap();
        checked.document.tests[0].targets[0].target.segments[1].key =
            Some(Expr::Str("unchecked-target-poison".into()));
        let TestStepKind::Click { target, .. } = &mut checked.document.tests[0].steps[0].kind
        else {
            panic!("fixture must start with a click");
        };
        let TestTargetRef::Id(target) = target else {
            panic!("fixture click must use a direct target");
        };
        target.segments[1].key = Some(Expr::Str("unchecked-click-poison".into()));
        let TestStepKind::Type(value) = &mut checked.document.tests[0].steps[1].kind else {
            panic!("fixture second step must type text");
        };
        *value = Expr::Str("unchecked-type-poison".into());
        let TestStepKind::Dispatch { args, .. } = &mut checked.document.tests[0].steps[2].kind
        else {
            panic!("fixture third step must dispatch");
        };
        args[0] = Expr::Str("unchecked-dispatch-poison".into());
        let TestStepKind::Expect(TestExpectation::Expr(value)) =
            &mut checked.document.tests[0].steps[3].kind
        else {
            panic!("fixture fourth step must expect an expression");
        };
        *value = Expr::Bool(false);

        let program = lower(checked).unwrap();
        let test = &program.tests()[0];
        assert_eq!(test.id, TestId(0));
        assert_eq!(test.name, "stable_flow");
        assert_eq!(
            test.targets[0].id,
            TestTargetId {
                test: test.id,
                index: 0
            }
        );
        assert!(test.targets[0].path.segments[1].key.is_some());
        assert!(matches!(
            test.steps[2].kind,
            ResolvedTestStepKind::Dispatch {
                handler: HandlerId(0),
                ref handler_name,
                ref args,
            } if handler_name == "change" && args.len() == 1
        ));
        assert!(matches!(
            test.steps[3].kind,
            ResolvedTestStepKind::Expect(ResolvedTestExpectation::Equality { negated: false, .. })
        ));
        assert_eq!(test.steps[1].source, "type label");
        assert_eq!(test.steps[2].source, "dispatch change((count + 1))");

        let generated = crate::codegen::generate(&program, "test-hir.ice").unwrap();
        assert!(generated.contains("self.label"));
        assert!(generated.contains("__test.state().count + 1"));
        assert!(generated.contains("\"type label\""));
        for poison in [
            "unchecked-target-poison",
            "unchecked-click-poison",
            "unchecked-type-poison",
            "unchecked-dispatch-poison",
        ] {
            assert!(
                !generated.contains(poison),
                "raw AST poison `{poison}` leaked"
            );
        }
    }

    #[test]
    fn lowers_result_literals_against_the_type_of_the_compared_operand() {
        const RESULT_APP: &str = "state\n  outcome:result[str,str] = ok(\"done\")\nview\n  col #root\n    match outcome\n      ok(value)\n        text value\n      err(error)\n        text error\n";

        for (literal, constructor) in [
            ("ok(\"done\")", "::std::result::Result::Ok"),
            ("err(\"failed\")", "::std::result::Result::Err"),
        ] {
            let source = format!(
                "app ResultCompare\n{THEME}{RESULT_APP}test compares\n  target root = #root\n  expect outcome == {literal}\n"
            );
            let checked = analyze(&source).unwrap();
            assert!(
                checked
                    .facts
                    .structural_snapshot()
                    .contains("Result(Str, Str)"),
                "the compared literal must adopt the operand's `result[str,str]` type"
            );
            let program = lower(checked).unwrap();
            let generated = crate::codegen::generate(&program, "result-compare.ice").unwrap();
            assert!(
                generated.contains(&format!("let __right = {constructor}(")),
                "`{literal}` must lower as the compared operand's result type"
            );
        }

        // An unconstrained literal keeps the documented erasure: both sides stay `unknown`
        // on the error side, so the checked operand type erases to `unit`.
        let unconstrained = format!(
            "app ResultErasure\n{THEME}state\n  flag = false\nview\n  col #root\n    if flag\n      text \"yes\"\ntest compares\n  target root = #root\n  expect ok(\"a\") == ok(\"a\")\n"
        );
        let checked = analyze(&unconstrained).unwrap();
        assert!(
            checked
                .facts
                .structural_snapshot()
                .contains("Result(Str, Unit)"),
            "an unconstrained result literal must erase its error side to `unit`"
        );
        lower(checked).unwrap();

        // Types the comparison blocklist bans keep rejecting inside a result literal.
        for (fixture, message) in [
            (
                format!(
                    "app ResultEnum\n{THEME}enum Surface\n  idle\n  ready(str)\nstate\n  outcome:result[Surface,str] = ok(Surface.idle)\nview\n  col #root\n    match outcome\n      ok(value)\n        match value\n          Surface.idle\n            text \"idle\"\n          Surface.ready(inner)\n            text inner\n      err(error)\n        text error\ntest compares\n  target root = #root\n  expect outcome == ok(Surface.idle)\n"
                ),
                "payload-carrying UI enum values use exhaustive match",
            ),
            (
                format!(
                    "app ResultTarget\n{THEME}state\n  flag = false\nview\n  col #root\n    if flag\n      text \"yes\"\ntest compares\n  target root = #root\n  expect ok(root) == ok(root)\n"
                ),
                "target values do not support comparisons",
            ),
        ] {
            let error = analyze(&fixture).unwrap_err();
            assert_eq!(error.code, "E153");
            assert!(error.message.contains(message), "{}", error.message);
        }

        // A payload the compared operand cannot accept still fails type checking.
        let mismatched = format!(
            "app ResultMismatch\n{THEME}{RESULT_APP}test compares\n  target root = #root\n  expect outcome == ok(1)\n"
        );
        let error = analyze(&mismatched).unwrap_err();
        assert_eq!(error.code, "E101");
    }

    #[test]
    fn rejects_test_config_target_and_step_semantic_mutations() {
        let source = format!(
            "app TestContracts\n{THEME}view\n  text \"Item\" #item\ntest contract\n  viewport 320 240\n  target item = #item\n  click item\n"
        );

        let mut config = analyze(&source).unwrap();
        config.document.tests[0].viewport = Some((640.0, 240.0));
        let error = lower(config).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("test declaration topology"));

        let mut target = analyze(&source).unwrap();
        target.document.tests[0].targets[0].target.segments[0].name = "changed".into();
        let error = lower(target).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("test target local"));

        let mut step = analyze(&source).unwrap();
        let TestStepKind::Click { count, .. } = &mut step.document.tests[0].steps[0].kind else {
            panic!("fixture must contain a click");
        };
        *count = 2;
        let error = lower(step).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("test step semantic contract"));
    }

    #[test]
    #[ignore = "explicit first-class test HIR performance contract"]
    fn performance_contract_four_thousand_test_steps_lower_and_emit_linearly() {
        const STEPS: usize = 4_000;
        let mut source = format!(
            "app TestPerf\n{THEME}state\n  value = 0\nview\n  text value #surface\ntest large_contract\n  target surface = #surface\n"
        );
        for index in 0..STEPS {
            writeln!(source, "  expect value + {index} >= 0").unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "test-perf.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.tests()[0].steps.len(), STEPS);
        assert_eq!(
            generated
                .matches("::ui_lang_runtime::testing::step(")
                .count(),
            STEPS
        );
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "{STEPS} checked test steps lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "explicit checked Canvas HIR performance contract"]
    fn performance_contract_four_thousand_canvas_commands_lower_and_emit_linearly() {
        const COMMANDS: usize = 4_000;
        let mut source = format!("app CanvasPerf\n{THEME}view\n  canvas w=fill h=600.0\n");
        for index in 0..COMMANDS {
            writeln!(
                source,
                "    circle x={}.0 y={}.0 r=2.0 fill=primary",
                index % 1_000,
                index / 1_000
            )
            .unwrap();
        }

        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "canvas-perf.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.canvas(ViewId(0)).unwrap().commands.len(), COMMANDS);
        assert_eq!(generated.matches("Path::circle(").count(), COMMANDS);
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "{COMMANDS} checked Canvas commands lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    fn normalizes_slider_and_progress_contracts() {
        let source = format!(
            "app RangeHir\nextern crate::backend\n  RangeNumber()\n  pure range_number(value:f64) -> RangeNumber\n  slider-style slider_style(active:bool)\n  progress-style progress_style(active:bool)\n{THEME}state\n  amount = 25.0\n  precise:RangeNumber = range_number(25.0)\n  active = true\non changed(next)\n  precise = next\non released\nview\n  col\n    slider precise min=range_number(0.0) max=range_number(100.0) step=range_number(1.0) default=range_number(25.0) shift-step=range_number(5.0) vertical w=20.0 h=fill(2) style=slider_style(active) release=released -> changed _\n      active rail-start=linear(0.0, primary@0.0, danger@1.0) rail-end=bg rail-w=4.0 rail-border=fg rail-border-w=1.0 rail-r=2.0 handle=rect(12) handle-color=primary handle-border=fg handle-border-w=1.0 handle-r=3.0\n    progress amount vertical length=fill(2) girth=20.0 style=progress_style(active) bg=bg bar=primary border=fg border-w=1.0 r=4.0\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let slider = program.slider(ViewId(1)).unwrap();
        assert_eq!(slider.value_type, Type::Named("RangeNumber".into()));
        assert_eq!(slider.axis, ResolvedRangeAxis::Vertical);
        assert!(slider.default.is_some());
        assert!(slider.shift_step.is_some());
        assert!(matches!(
            slider.height,
            Some(ResolvedContainerLength::FillPortion(2))
        ));
        assert!(slider.change.source_payloads == [Type::Named("RangeNumber".into())]);
        assert!(slider.release.is_some());
        assert!(slider.custom_style.is_some());
        let active = slider.styles.active.as_ref().unwrap();
        assert!(matches!(
            active.rail_start,
            Some(ResolvedContainerBackground::Linear { .. })
        ));
        assert!(matches!(
            active.handle_shape,
            Some(ResolvedSliderHandleShape::Rectangle { width: 12, .. })
        ));

        let progress = program.progress(ViewId(2)).unwrap();
        assert_eq!(progress.axis, ResolvedRangeAxis::Vertical);
        assert!(matches!(
            progress.length,
            Some(ResolvedContainerLength::FillPortion(2))
        ));
        assert!(progress.custom_style.is_some());
        assert!(matches!(
            progress.background,
            Some(ResolvedContainerBackground::Color(_))
        ));
        assert!(matches!(
            progress.bar,
            Some(ResolvedContainerBackground::Color(_))
        ));

        for range in [slider.id, progress.id] {
            let checked = program.checked_facts().interaction(range).unwrap();
            for index in 0..checked.expression_count {
                let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: range,
                    index,
                });
                let expression = program
                    .checked_facts()
                    .expression_use_by_owner(owner)
                    .unwrap();
                assert_eq!(
                    program.checked_facts().expression_use(expression).owner,
                    owner
                );
            }
        }
    }

    #[test]
    fn range_lowering_uses_checked_expressions_and_codegen_ignores_raw_contracts() {
        let source = format!(
            "app CheckedRanges\nextern crate::backend\n  slider-style slider_style(active:bool)\n  progress-style progress_style(active:bool)\n{THEME}state\n  amount = 25.0\n  active = true\non changed(next)\n  amount = next\non released\nview\n  col\n    slider amount min=0.0 max=100.0 step=1.0 default=25.0 shift-step=5.0 vertical w=20.0 h=fill style=slider_style(active) release=released -> changed _\n      active rail-start=primary rail-end=bg rail-w=4.0 rail-border=fg rail-border-w=1.0 handle=circle(7.0) handle-color=primary handle-border=fg handle-border-w=1.0\n    progress amount vertical length=fill girth=20.0 style=progress_style(active) bg=bg bar=primary border=fg border-w=1.0 r=4.0\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-ranges.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Slider {
            value,
            min,
            max,
            step,
            options,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be a slider");
        };
        *value = Expr::F64(999.0);
        *min = Expr::F64(999.0);
        *max = Expr::F64(999.0);
        *step = Expr::F64(999.0);
        options.default = Some(Expr::F64(999.0));
        options.shift_step = Some(Expr::F64(999.0));
        options.style.custom.as_mut().unwrap().args[0] = Expr::Bool(false);
        options.style.active.as_mut().unwrap().rail_width = Some(Expr::F64(999.0));
        let ViewNode::Progress { value, options, .. } = &mut children[1] else {
            panic!("second child must be progress");
        };
        *value = Expr::F64(999.0);
        options.custom_style.as_mut().unwrap().args[0] = Expr::Bool(false);
        options.border_width = Some(Expr::F64(999.0));
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-ranges.ice").unwrap();
        assert_eq!(actual, expected);

        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected = crate::codegen::generate(&program, "lowered-ranges.ice").unwrap();
        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Slider {
            value,
            min,
            max,
            step,
            options,
            vertical,
            styles,
            route,
            release,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be a slider");
        };
        *value = Expr::F64(777.0);
        *min = Expr::F64(777.0);
        *max = Expr::F64(777.0);
        *step = Expr::F64(777.0);
        **options = SliderOptions::default();
        *vertical = false;
        styles.clear();
        route.handler = "poisoned".into();
        *release = None;
        let ViewNode::Progress {
            value,
            min,
            max,
            options,
            vertical,
            styles,
            ..
        } = &mut children[1]
        else {
            panic!("second child must be progress");
        };
        *value = Expr::F64(777.0);
        *min = Expr::F64(777.0);
        *max = Expr::F64(777.0);
        *options = ProgressOptions::default();
        *vertical = false;
        styles.clear();
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(0, 1);
        let actual = crate::codegen::generate(&program, "lowered-ranges.ice").unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("poisoned"));

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Slider { vertical, .. } = &mut children[0] else {
            panic!("first child must be a slider");
        };
        *vertical = false;
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn range_lowering_rejects_same_type_slot_route_and_extern_swaps() {
        let source = format!(
            "app RangeIdentity\nextern crate::backend\n  slider-style slider_first(active:bool)\n  slider-style slider_second(active:bool)\n  progress-style progress_first(active:bool)\n  progress-style progress_second(active:bool)\n{THEME}state\n  amount = 25.0\n  active = true\non first(next)\n  amount = next\non second(next)\n  amount = next\nview\n  col\n    slider amount min=0.0 max=100.0 step=1.0 style=slider_first(active) -> first _\n    progress amount min=0.0 max=100.0 style=progress_first(active) border-w=1.0 r=4.0\n    slider amount min=0.0 max=100.0 step=1.0 -> second _\n"
        );

        let mut swapped_slider_slots = analyze(&source).unwrap();
        swapped_slider_slots
            .facts
            .swap_interaction_option_expressions(ViewId(1), 1, 2);
        let error = lower(swapped_slider_slots).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut swapped_progress_slots = analyze(&source).unwrap();
        swapped_progress_slots
            .facts
            .swap_interaction_option_expressions(ViewId(2), 0, 1);
        let error = lower(swapped_progress_slots).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut swapped_route = analyze(&source).unwrap();
        swapped_route
            .facts
            .corrupt_interaction_route_handler(ViewId(1), 0, HandlerId(1));
        let error = lower(swapped_route).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("handler contract diverged"));

        let mut swapped_slider_extern = analyze(&source).unwrap();
        swapped_slider_extern
            .facts
            .corrupt_slider_style(ViewId(1), ExternFnId(1));
        let error = lower(swapped_slider_extern).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("style extern contract diverged"));

        let mut swapped_progress_extern = analyze(&source).unwrap();
        swapped_progress_extern
            .facts
            .corrupt_progress_style(ViewId(2), ExternFnId(3));
        let error = lower(swapped_progress_extern).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("style extern contract diverged"));

        let mut corrupt_id = analyze(&source).unwrap();
        corrupt_id.facts.corrupt_slider_id(ViewId(1), u32::MAX);
        let error = lower(corrupt_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));

        let mut corrupt_id = analyze(&source).unwrap();
        corrupt_id.facts.corrupt_progress_id(ViewId(2), u32::MAX);
        let error = lower(corrupt_id).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));
    }

    #[test]
    fn imported_ranges_keep_expression_route_extern_theme_and_status_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-range-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("ranges.ice");
        fs::write(
            &root,
            format!(
                "app ImportedRangesApp\nuse \"ranges.ice\"\n{THEME}state\n  amount = 25.0\nview\n  ImportedRanges value=amount\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "extern crate::backend\n  slider-style imported_slider()\n  progress-style imported_progress()\ncomponent ImportedRanges(value:f64)\n  on changed(next)\n  on released\n  col\n    slider value min=0.0 max=100.0 step=1.0 style=imported_slider() release=released -> changed _\n      active rail-start=primary rail-border=fg rail-w=4.0 handle=circle(7.0) handle-color=primary\n    progress value min=0.0 max=100.0 style=imported_progress() bg=bg bar=primary border=fg border-w=1.0\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let slider = program.sliders.values().next().unwrap();
        let slider_id = slider.id;
        let slider_origin = program.origin(slider.origin);
        assert_eq!(slider_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(slider_origin.line, 8);
        let value = program.checked_facts().expression_use(slider.value);
        let value_origin = program.origin(value.origin);
        assert_eq!(value_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(value_origin.line, 8);
        assert_eq!(value_origin.parent, Some(slider.origin));
        let route_origin = program.origin(slider.change.origin);
        assert_eq!(route_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(route_origin.line, 8);
        assert_eq!(route_origin.parent, Some(slider.origin));
        let style = slider.custom_style.as_ref().unwrap();
        let style_origin = program.origin(style.origin);
        assert_eq!(style_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(style_origin.line, 8);
        assert_eq!(style_origin.parent, Some(slider.origin));
        let extern_origin =
            program.origin(program.extern_function(style.function).declaration.origin);
        assert_eq!(extern_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(extern_origin.line, 2);
        let active = slider.styles.active.as_ref().unwrap();
        let status_origin = program.origin(active.origin);
        assert_eq!(status_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(status_origin.line, 9);
        assert_eq!(status_origin.parent, Some(slider.origin));
        assert!(matches!(
            active.rail_border_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(token),
                ..
            }) if token == program.theme().native_tokens.text
        ));

        let progress = program.progresses.values().next().unwrap();
        let progress_id = progress.id;
        let progress_origin = program.origin(progress.origin);
        assert_eq!(progress_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(progress_origin.line, 10);
        let value_origin = program.origin(
            program
                .checked_facts()
                .expression_use(progress.value)
                .origin,
        );
        assert_eq!(value_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(value_origin.line, 10);
        assert_eq!(value_origin.parent, Some(progress.origin));
        let style = progress.custom_style.as_ref().unwrap();
        let style_origin = program.origin(style.origin);
        assert_eq!(style_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(style_origin.line, 10);
        assert_eq!(style_origin.parent, Some(progress.origin));
        let extern_origin =
            program.origin(program.extern_function(style.function).declaration.origin);
        assert_eq!(extern_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(extern_origin.line, 3);
        assert!(matches!(
            progress.bar,
            Some(ResolvedContainerBackground::Color(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(token),
                ..
            })) if token == program.theme().native_tokens.primary
        ));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 8 1 {encoded_import}")));

        let slider = program.sliders.remove(&slider_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 8);
        program.sliders.insert(slider_id, slider);

        program.progresses.remove(&progress_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 10);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_checked_range_ids_do_not_panic() {
        let source = format!(
            "app InvalidRanges\n{THEME}state\n  amount = 25.0\non changed(next)\n  amount = next\nview\n  col\n    slider amount min=0.0 max=100.0 step=1.0 -> changed _\n    progress amount\n"
        );
        for (view, index) in [(ViewId(1), 0), (ViewId(2), 0)] {
            let mut checked = analyze(&source).unwrap();
            checked.facts.corrupt_expression_use_root(
                CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: view,
                    index,
                }),
                u32::MAX,
            );
            let error = lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains("invalid checked expression ID"));
        }
    }

    #[test]
    fn normalizes_rule_qr_code_and_space_contracts() {
        let source = format!(
            "app ContentPrimitiveHir\n{THEME}state\n  thickness = 2.0\n  percent = 75.0\n  radius = 4.0\n  snap = false\n  payload = \"https://example.com/invite\"\n  cell_size = 4.0\n  width = 20.0\nview\n  col\n    rule horizontal thickness=thickness style=weak fill=percent(percent) color=primary/50 r=radius r-tl=radius snap=snap\n    qr payload correction=high version=normal(4) cell-size=cell_size cell=primary bg=bg\n    qr bytes(00 ff a4) version=micro(4) size=128.0 cell=danger/25\n    space w=width h=fill(2)\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();

        let rule = program.rule(ViewId(1)).unwrap();
        assert_eq!(rule.id, ViewId(1));
        assert_eq!(rule.axis, ResolvedRuleAxis::Horizontal);
        assert_eq!(rule.preset, ResolvedRulePreset::Weak);
        assert!(matches!(rule.fill, Some(ResolvedRuleFill::Percent(_))));
        assert!(rule.radius.all.is_some());
        assert!(rule.radius.top_left.is_some());
        assert!(rule.snap.is_some());
        assert!(matches!(
            rule.color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(token),
                opacity: Some(50),
            }) if token == program.theme().native_tokens.primary
        ));

        let text_qr = program.qr_code(ViewId(2)).unwrap();
        assert_eq!(text_qr.id, ViewId(2));
        assert_eq!(text_qr.payload_kind, ResolvedQrPayloadKind::Text);
        assert_eq!(
            text_qr.encoding,
            ResolvedQrEncoding::Versioned {
                version: ResolvedQrVersion::Normal(4),
                correction: ResolvedQrCorrection::High,
            }
        );
        assert!(matches!(text_qr.size, ResolvedQrSize::Cell(_)));
        assert!(matches!(
            text_qr.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(token),
                ..
            }) if token == program.theme().native_tokens.background
        ));

        let bytes_qr = program.qr_code(ViewId(3)).unwrap();
        assert_eq!(bytes_qr.payload_kind, ResolvedQrPayloadKind::Bytes);
        assert_eq!(
            bytes_qr.encoding,
            ResolvedQrEncoding::Versioned {
                version: ResolvedQrVersion::Micro(4),
                correction: ResolvedQrCorrection::Medium,
            }
        );
        assert!(matches!(bytes_qr.size, ResolvedQrSize::Total(_)));

        let space = program.space(ViewId(4)).unwrap();
        assert_eq!(space.id, ViewId(4));
        assert!(matches!(
            space.width,
            Some(ResolvedContainerLength::FixedF64(_))
        ));
        assert!(matches!(
            space.height,
            Some(ResolvedContainerLength::FillPortion(2))
        ));

        for (view, types) in [
            (
                ViewId(1),
                vec![Type::F64, Type::F64, Type::F64, Type::F64, Type::Bool],
            ),
            (ViewId(2), vec![Type::Str, Type::F64]),
            (ViewId(3), vec![Type::Bytes, Type::F64]),
            (ViewId(4), vec![Type::F64]),
        ] {
            let checked = program.checked_facts().interaction(view).unwrap();
            assert_eq!(checked.expression_count as usize, types.len());
            for (index, ty) in types.into_iter().enumerate() {
                let owner = CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: view,
                    index: index as u32,
                });
                let expression = program
                    .checked_facts()
                    .expression_use_by_owner(owner)
                    .unwrap();
                let retained = program.checked_facts().expression_use(expression);
                assert_eq!(retained.owner, owner);
                assert_eq!(retained.source, ty);
                assert_eq!(retained.destination, ty);
                assert_eq!(
                    program.origin(retained.origin).parent,
                    Some(program.checked_facts().view(view).origin)
                );
            }
        }
    }

    #[test]
    fn content_primitive_lowering_uses_checked_expressions_and_codegen_ignores_raw_contracts() {
        let source = format!(
            "app CheckedContentPrimitives\n{THEME}state\n  thickness = 2.0\n  percent = 75.0\n  radius = 4.0\n  snap = false\n  payload = \"https://example.com/invite\"\n  cell_size = 4.0\n  width = 20.0\n  height = 10.0\nview\n  col\n    rule horizontal thickness=thickness style=weak fill=percent(percent) color=primary/50 r=radius snap=snap\n    qr payload correction=high version=normal(4) cell-size=cell_size cell=primary bg=bg\n    space w=width h=height\n"
        );
        let expected = crate::codegen::generate(
            &lower(analyze(&source).unwrap()).unwrap(),
            "checked-content-primitives.ice",
        )
        .unwrap();

        let mut checked = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut checked.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Rule {
            thickness, options, ..
        } = &mut children[0]
        else {
            panic!("first child must be a rule");
        };
        *thickness = Expr::F64(999.0);
        options.fill = Some(RuleFill::Percent(Expr::F64(999.0)));
        options.radius = Some(Expr::F64(999.0));
        options.snap = Some(Expr::Bool(true));
        let ViewNode::QrCode {
            payload, cell_size, ..
        } = &mut children[1]
        else {
            panic!("second child must be a qr code");
        };
        *payload = Expr::Str("raw-checked-poison".into());
        *cell_size = Some(Expr::F64(999.0));
        let ViewNode::Space { width, height, .. } = &mut children[2] else {
            panic!("third child must be a space");
        };
        *width = Some(LengthValue::Fixed(Expr::F64(999.0)));
        *height = Some(LengthValue::Fixed(Expr::F64(999.0)));
        let actual =
            crate::codegen::generate(&lower(checked).unwrap(), "checked-content-primitives.ice")
                .unwrap();
        assert_eq!(actual, expected);
        assert!(!actual.contains("raw-checked-poison"));

        let mut program = lower(analyze(&source).unwrap()).unwrap();
        let expected =
            crate::codegen::generate(&program, "lowered-content-primitives.ice").unwrap();
        let ViewNode::Layout { children, .. } = &mut program.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Rule {
            axis,
            thickness,
            options,
            styles,
            ..
        } = &mut children[0]
        else {
            panic!("first child must be a rule");
        };
        *axis = Axis::Vertical;
        *thickness = Expr::F64(777.0);
        *options = RuleOptions::default();
        styles.push("raw-rule-poison".into());
        let ViewNode::QrCode {
            payload,
            correction,
            version,
            cell_size,
            total_size,
            cell,
            background,
            ..
        } = &mut children[1]
        else {
            panic!("second child must be a qr code");
        };
        *payload = Expr::Str("raw-qr-poison".into());
        *correction = Some(QrCorrection::Low);
        *version = Some(QrVersion::Micro(1));
        *cell_size = None;
        *total_size = Some(Expr::F64(777.0));
        *cell = Some("danger".into());
        *background = Some("fg".into());
        let ViewNode::Space {
            width,
            height,
            styles,
            ..
        } = &mut children[2]
        else {
            panic!("third child must be a space");
        };
        *width = Some(LengthValue::Shrink);
        *height = Some(LengthValue::Fill);
        styles.push("raw-space-poison".into());
        program
            .document
            .theme_contract
            .as_mut()
            .unwrap()
            .tokens
            .swap(0, 1);
        let actual = crate::codegen::generate(&program, "lowered-content-primitives.ice").unwrap();
        assert_eq!(actual, expected);
        for poison in ["raw-rule-poison", "raw-qr-poison", "raw-space-poison"] {
            assert!(!actual.contains(poison));
        }

        let mut changed_static = analyze(&source).unwrap();
        let ViewNode::Layout { children, .. } = &mut changed_static.document.view else {
            panic!("fixture root must be a layout");
        };
        let ViewNode::Rule { axis, .. } = &mut children[0] else {
            panic!("first child must be a rule");
        };
        *axis = Axis::Vertical;
        let error = lower(changed_static).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("topology diverged"));
    }

    #[test]
    fn content_primitive_lowering_rejects_same_type_identity_and_corrupt_facts() {
        let source = format!(
            "app ContentPrimitiveIdentity\n{THEME}state\n  first = 2.0\n  second = 75.0\n  first_payload = \"first\"\n  second_payload = \"second\"\nview\n  col\n    rule horizontal thickness=first fill=percent(second)\n    space w=first h=second\n    qr first_payload cell-size=first\n    qr second_payload cell-size=second\n"
        );

        let mut swapped_rule = analyze(&source).unwrap();
        swapped_rule
            .facts
            .swap_interaction_option_expressions(ViewId(1), 0, 1);
        let error = lower(swapped_rule).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut swapped_space = analyze(&source).unwrap();
        swapped_space
            .facts
            .swap_interaction_option_expressions(ViewId(2), 0, 1);
        let error = lower(swapped_space).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut transplanted_qr = analyze(&source).unwrap();
        transplanted_qr
            .facts
            .transplant_interaction_option_expression(ViewId(3), 0, ViewId(4), 0);
        let error = lower(transplanted_qr).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut corrupt_payload_type = analyze(&source).unwrap();
        corrupt_payload_type
            .facts
            .corrupt_qr_payload_type(ViewId(3), Type::Bytes);
        let error = lower(corrupt_payload_type).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("expression contract diverged"));

        let mut corrupt_rule = analyze(&source).unwrap();
        corrupt_rule.facts.corrupt_rule_id(ViewId(1), u32::MAX);
        let error = lower(corrupt_rule).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));

        let mut corrupt_qr = analyze(&source).unwrap();
        corrupt_qr.facts.corrupt_qr_code_id(ViewId(3), u32::MAX);
        let error = lower(corrupt_qr).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));

        let mut corrupt_space = analyze(&source).unwrap();
        corrupt_space.facts.corrupt_space_id(ViewId(2), u32::MAX);
        let error = lower(corrupt_space).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("no checked HIR facts"));
    }

    #[test]
    fn imported_content_primitives_keep_exact_expression_and_theme_origins() {
        let nonce = crate::test_support::unique_nonce();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-content-primitive-hir-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("content.ice");
        fs::write(
            &root,
            format!(
                "app ImportedContentPrimitiveApp\nuse \"content.ice\"\n{THEME}state\n  payload = \"https://example.com\"\nview\n  ImportedContent payload=payload\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "component ImportedContent(payload:str)\n  col\n    rule horizontal thickness=2.0 fill=percent(75.0) color=primary r=4.0 snap=false\n    qr payload version=normal(4) cell-size=4.0 cell=fg bg=bg\n    space w=10.0 h=fill(2)\n",
        )
        .unwrap();

        let mut program = lower(analyze_file(&root).unwrap()).unwrap();
        let rule_id = program.rules.values().next().unwrap().id;
        let qr_id = program.qr_codes.values().next().unwrap().id;
        let space_id = program.spaces.values().next().unwrap().id;
        let entries = [(rule_id, 3usize), (qr_id, 4usize), (space_id, 5usize)];
        for (view, line) in entries {
            let checked = program.checked_facts().interaction(view).unwrap();
            let view_origin = program.checked_facts().view(view).origin;
            let origin = program.origin(view_origin);
            assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
            assert_eq!(origin.line, line);
            assert_eq!(origin.column, 1);
            for expression in &checked.option_expressions {
                let expression_origin =
                    program.origin(program.checked_facts().expression_use(*expression).origin);
                assert_eq!(expression_origin.path.as_deref(), Some(imported.as_path()));
                assert_eq!(expression_origin.line, line);
                assert_eq!(expression_origin.column, 1);
                assert_eq!(expression_origin.parent, Some(view_origin));
            }
        }
        let rule = program.rules.values().next().unwrap();
        assert!(matches!(
            rule.color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(token),
                ..
            }) if token == program.theme().native_tokens.primary
        ));
        let qr = program.qr_codes.values().next().unwrap();
        assert!(matches!(
            qr.cell,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(token),
                ..
            }) if token == program.theme().native_tokens.text
        ));
        assert!(matches!(
            qr.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(token),
                ..
            }) if token == program.theme().native_tokens.background
        ));

        let generated = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap();
        let encoded_import = crate::codegen::encode_source_path(&imported.display().to_string());
        assert!(generated.contains(&format!("// __ICE_SOURCE 3 1 {encoded_import}")));

        let rule = program.rules.remove(&rule_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 3);
        program.rules.insert(rule_id, rule);

        let qr = program.qr_codes.remove(&qr_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 4);
        program.qr_codes.insert(qr_id, qr);

        program.spaces.remove(&space_id).unwrap();
        let error = crate::codegen::generate(&program, root.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 5);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_checked_content_primitive_ids_do_not_panic() {
        let source = format!(
            "app InvalidContentPrimitives\n{THEME}view\n  col\n    rule horizontal thickness=2.0\n    qr \"payload\"\n    space w=10.0\n"
        );
        for view in [ViewId(1), ViewId(2), ViewId(3)] {
            let mut checked = analyze(&source).unwrap();
            checked.facts.corrupt_expression_use_root(
                CheckedExprOwner::Interaction(InteractionExpressionId {
                    widget: view,
                    index: 0,
                }),
                u32::MAX,
            );
            let error = lower(checked).unwrap_err();
            assert_eq!(error.code, "E196");
            assert!(error.message.contains("invalid checked expression ID"));
        }
    }

    #[test]
    #[ignore = "large normalized content-primitive lowering and emission performance contract"]
    fn performance_contract_four_thousand_content_primitives_lower_and_emit_under_two_seconds() {
        const PRIMITIVES: usize = 4_000;
        let mut source = format!(
            "app ContentPrimitiveScale\n{THEME}state\n  amount = 25.0\n  payload = \"https://example.com\"\nview\n  col\n"
        );
        for index in 0..PRIMITIVES {
            match index % 3 {
                0 => writeln!(
                    source,
                    "    rule horizontal #rule_{index} thickness=amount fill=percent(amount)"
                )
                .unwrap(),
                1 => writeln!(
                    source,
                    "    qr payload #qr_{index} correction=high cell-size=amount"
                )
                .unwrap(),
                _ => writeln!(source, "    space #space_{index} w=amount h=fill").unwrap(),
            }
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "content-primitive-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(
            program.rules.len() + program.qr_codes.len() + program.spaces.len(),
            PRIMITIVES
        );
        assert_eq!(
            generated
                .matches("::iced::widget::rule::horizontal(")
                .count(),
            1_334
        );
        assert_eq!(
            generated.matches("::ui_lang_runtime::qr_code(").count(),
            1_333
        );
        assert_eq!(generated.matches("::iced::widget::space()").count(), 1_333);
        eprintln!("4k normalized content primitives lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized content primitives lowered and emitted in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large normalized range-control lowering and emission performance contract"]
    fn performance_contract_four_thousand_range_controls_lower_and_emit_under_two_seconds() {
        const EACH: usize = 2_000;
        let mut source = format!(
            "app RangeScale\n{THEME}state\n  amount = 25.0\non changed(next)\n  amount = next\nview\n  col\n"
        );
        for index in 0..EACH {
            writeln!(
                source,
                "    slider amount #slider_{index} min=0.0 max=100.0 step=1.0 w=fill -> changed _"
            )
            .unwrap();
            writeln!(
                source,
                "    progress amount #progress_{index} min=0.0 max=100.0 length=fill"
            )
            .unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let generated = crate::codegen::generate(&program, "range-scale.ice").unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.sliders.len(), EACH);
        assert_eq!(program.progresses.len(), EACH);
        assert_eq!(generated.matches("::iced::widget::slider(").count(), EACH);
        assert_eq!(
            generated.matches("::iced::widget::progress_bar(").count(),
            EACH
        );
        eprintln!("4k normalized range controls lowered and emitted in {elapsed:?}");
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "4k normalized range controls lowered and emitted in {elapsed:?}"
        );
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
