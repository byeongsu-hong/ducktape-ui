use super::*;

#[derive(Clone, Debug)]
pub struct State {
    pub name: String,
    pub ty: Type,
    pub initial: Expr,
    pub animation: Option<AnimationOptions>,
    pub span: Span,
}

/// A runtime-held text buffer an `input` may write and nothing may read.
///
/// Deliberately not a `State`: a secret has no type to declare, no initial
/// value to write down, and no field on the application struct that a preset,
/// a snapshot, or an `expect` could reach. What Ice may do with the name is
/// bind one `input` to it, ask it `empty` or `len`, clear it with `= ""`, and
/// pass it to an extern parameter declared `secret`.
#[derive(Clone, Debug)]
pub struct SecretDecl {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct AnimationOptions {
    pub easing: Option<String>,
    pub duration: Option<AnimationDuration>,
    pub delay_ms: Option<u64>,
    pub repeat: Option<u32>,
    pub repeat_forever: bool,
    pub auto_reverse: bool,
    /// The value the animation holds the instant it comes into being; it then
    /// travels to the declared one. This is what lets an instance materialized
    /// by a `for` fade in without an assignment: there is no event to assign
    /// on, so the transition has to be part of the declaration.
    pub from: Option<AnimationStart>,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub params: Vec<ComponentParam>,
    pub output: Type,
    pub events: Vec<ComponentEvent>,
    pub lifetime: ComponentLifetime,
    pub states: Vec<State>,
    pub handlers: Vec<Handler>,
    pub root: ViewNode,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComponentLifetime {
    #[default]
    Retained,
    Mounted,
}

#[derive(Clone, Debug)]
pub struct ComponentParam {
    pub name: String,
    pub ty: Type,
    pub bind: bool,
    pub default: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct ComponentEvent {
    pub name: String,
    pub payloads: Vec<Type>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Handler {
    pub name: String,
    pub params: Vec<HandlerParam>,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct HandlerParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct HandlerMatchArm {
    pub enum_name: String,
    pub variant: String,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Let {
        name: String,
        value: Expr,
        span: Span,
    },
    Assign {
        target: String,
        value: Expr,
        at: Option<Expr>,
        span: Span,
    },
    MarkdownAppend {
        target: String,
        value: Expr,
        span: Span,
    },
    ComboPush {
        target: String,
        value: Expr,
        span: Span,
    },
    ReturnIf {
        condition: Expr,
        span: Span,
    },
    Match {
        value: Expr,
        arms: Vec<HandlerMatchArm>,
        span: Span,
    },
    Exit {
        span: Span,
    },
    InvalidateLane {
        lane: String,
        span: Span,
    },
    Run {
        kind: EffectKind,
        mode: DeliveryMode,
        lane: Option<String>,
        function: String,
        args: Vec<Expr>,
        success: Route,
        error: Option<Route>,
        span: Span,
    },
    Sip {
        function: String,
        args: Vec<Expr>,
        progress: Route,
        success: Route,
        error: Option<Route>,
        span: Span,
    },
    TaskFlow {
        source: TaskSource,
        transforms: Vec<TaskTransform>,
        success: Option<Route>,
        error: Option<Route>,
        units: Option<Route>,
        span: Span,
    },
    TaskGroup {
        kind: TaskGroupKind,
        statements: Vec<Statement>,
        span: Span,
    },
    Abortable {
        handle: String,
        abort_on_drop: bool,
        task: Box<Statement>,
        span: Span,
    },
    Abort {
        handle: String,
        span: Span,
    },
    DebugStart {
        name: Expr,
        target: String,
        span: Span,
    },
    DebugFinish {
        target: String,
        span: Span,
    },
    ClipboardWrite {
        primary: bool,
        value: Expr,
        span: Span,
    },
    /// A component handler firing one of its declared events, delivered as
    /// the next update-loop message through the call sites' agreed route.
    Emit {
        event: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// One instance's share of what this handler just received. The app keeps
    /// its single subscription and its route; the statement hands the payload
    /// on to the instance the key names, as the next message in the same
    /// update loop.
    Slice {
        component: String,
        handler: String,
        args: Vec<Expr>,
        key: Expr,
        span: Span,
    },
    WidgetOperation {
        operation: WidgetOperation,
        route: Option<Route>,
        span: Span,
    },
    WindowOperation {
        operation: WindowOperation,
        target: Option<Expr>,
        route: Option<Route>,
        span: Span,
    },
    PaneOperation {
        grid: String,
        operation: PaneOperation,
        route: Option<Route>,
        span: Span,
    },
}

impl Statement {
    pub fn span(&self) -> &Span {
        match self {
            Self::Let { span, .. }
            | Self::Assign { span, .. }
            | Self::MarkdownAppend { span, .. }
            | Self::ComboPush { span, .. }
            | Self::ReturnIf { span, .. }
            | Self::Match { span, .. }
            | Self::Exit { span }
            | Self::InvalidateLane { span, .. }
            | Self::Run { span, .. }
            | Self::Sip { span, .. }
            | Self::TaskFlow { span, .. }
            | Self::TaskGroup { span, .. }
            | Self::Abortable { span, .. }
            | Self::Abort { span, .. }
            | Self::DebugStart { span, .. }
            | Self::DebugFinish { span, .. }
            | Self::ClipboardWrite { span, .. }
            | Self::Emit { span, .. }
            | Self::Slice { span, .. }
            | Self::WidgetOperation { span, .. }
            | Self::WindowOperation { span, .. }
            | Self::PaneOperation { span, .. } => span,
        }
    }

    /// The task-finality classifier: the diagnostic code and the prose label
    /// for a statement that must close a handler, or None if it may sit
    /// mid-handler. `check::handler` is the only reader of the pair.
    pub(crate) fn immediate_task(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Let { .. }
            | Self::Assign { .. }
            | Self::MarkdownAppend { .. }
            | Self::ComboPush { .. }
            | Self::ReturnIf { .. }
            | Self::InvalidateLane { .. }
            | Self::Abort { .. }
            | Self::DebugStart { .. }
            | Self::DebugFinish { .. } => None,
            Self::Match { .. } => Some(("E141", "handler match")),
            Self::Exit { .. } => Some(("E141", "exit")),
            Self::Run { kind, .. } => Some((
                "E141",
                match kind {
                    EffectKind::Future => "run",
                    EffectKind::Task => "task",
                    EffectKind::Stream => "stream",
                },
            )),
            Self::Sip { .. } => Some(("E141", "sip")),
            Self::TaskFlow { .. } => Some(("E141", "flow")),
            Self::TaskGroup { .. } => Some(("E141", "task group")),
            Self::Abortable { .. } => Some(("E141", "abortable task")),
            Self::ClipboardWrite { .. } => Some(("E141", "clipboard write")),
            Self::Emit { .. } => Some(("E141", "emit")),
            // A SLICE IS A PUBLICATION, not the handler's closing act: it
            // hands a payload on and the handler keeps going. Its message is
            // accumulated and batched with whatever task the handler does
            // end on, so a guard below one cannot swallow it.
            Self::Slice { .. } => None,
            Self::WidgetOperation { .. } => Some(("E172", "widget operation")),
            Self::WindowOperation { .. } => Some(("E173", "window task")),
            Self::PaneOperation {
                operation: PaneOperation::Maximized | PaneOperation::Adjacent { .. },
                ..
            } => Some(("E188", "pane query")),
            Self::PaneOperation { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TaskSource {
    Effect {
        kind: EffectKind,
        function: String,
        args: Vec<Expr>,
        span: Span,
    },
    Done {
        value: Expr,
        span: Span,
    },
    None {
        output: Type,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum TaskTransform {
    Map {
        binding: String,
        value: Expr,
        span: Span,
    },
    Then {
        binding: String,
        source: TaskSource,
        span: Span,
    },
    AndThen {
        binding: String,
        source: TaskSource,
        span: Span,
    },
    MapError {
        binding: String,
        value: Expr,
        span: Span,
    },
    Collect {
        span: Span,
    },
    Discard {
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum PaneOperation {
    Maximize {
        pane: PaneReference,
    },
    Restore,
    Maximized,
    Adjacent {
        pane: PaneReference,
        edge: PaneEdge,
    },
    Swap {
        first: PaneReference,
        second: PaneReference,
    },
    Close {
        pane: PaneReference,
    },
    Move {
        pane: PaneReference,
        edge: PaneEdge,
    },
    Resize {
        split: Option<String>,
        ratio: Expr,
    },
    Drop {
        pane: PaneReference,
        target: PaneReference,
        edge: Option<PaneEdge>,
    },
    Split {
        target: PaneReference,
        pane: PaneReference,
        axis: PaneAxis,
        ratio: Expr,
    },
}

#[derive(Clone, Debug)]
pub enum PaneReference {
    Static(String),
    Dynamic { template: String, key: Expr },
}

#[derive(Clone, Debug)]
pub enum WidgetOperation {
    FocusPrevious,
    FocusNext,
    Focus {
        target: WidgetTarget,
    },
    Focused {
        target: WidgetTarget,
    },
    CursorFront {
        target: WidgetTarget,
    },
    CursorEnd {
        target: WidgetTarget,
    },
    Cursor {
        target: WidgetTarget,
        position: Expr,
    },
    SelectAll {
        target: WidgetTarget,
    },
    Select {
        target: WidgetTarget,
        start: Expr,
        end: Expr,
    },
    Snap {
        target: WidgetTarget,
        x: Expr,
        y: Expr,
    },
    SnapEnd {
        target: WidgetTarget,
    },
    ScrollTo {
        target: WidgetTarget,
        x: Expr,
        y: Expr,
    },
    ScrollBy {
        target: WidgetTarget,
        x: Expr,
        y: Expr,
    },
    /// Lands the keyed row `key` of the virtual column inside the scroll
    /// `target` at the top of its viewport.
    ScrollToKey {
        target: WidgetTarget,
        key: Expr,
    },
    Find {
        selector: WidgetSelector,
        all: bool,
    },
}

#[derive(Clone, Debug)]
pub struct WidgetTarget {
    pub segments: Vec<Id>,
    /// The daemon window whose render qualifies the id — the statement's
    /// `window=<window-id>` marker. An app handler in a daemon keeping
    /// mounted component state must name one; everywhere else the qualifier
    /// is rejected.
    pub window: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum WidgetSelector {
    Id(WidgetTarget),
    Text(Expr),
    Point { x: Expr, y: Expr },
    Focused,
    Extern { function: String, args: Vec<Expr> },
}

#[derive(Clone, Debug)]
pub enum WindowOperation {
    Open(Option<String>),
    Oldest,
    Latest,
    Close,
    Drag,
    DragResize(WindowDirection),
    Resize(Expr, Expr),
    Resizable(Expr),
    MinSize(Option<(Expr, Expr)>),
    MaxSize(Option<(Expr, Expr)>),
    ResizeIncrements(Option<(Expr, Expr)>),
    Size,
    IsMaximized,
    Maximize(Expr),
    IsMinimized,
    Minimize(Expr),
    Position,
    ScaleFactor,
    Move(Expr, Expr),
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
    MousePassthrough(Expr),
    MonitorSize,
    AutomaticTabbing(Expr),
    Icon {
        pixels: Expr,
        width: Expr,
        height: Expr,
    },
    Callback {
        function: String,
        args: Vec<Expr>,
    },
}

#[derive(Clone, Debug)]
pub struct Route {
    pub handler: String,
    pub args: Vec<RouteArg>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum RouteArg {
    Expr(Expr),
    Payload,
}

#[derive(Clone, Debug)]
pub struct Id {
    pub name: String,
    pub key: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct ComponentArg {
    pub name: String,
    pub value: Expr,
    pub bind: bool,
}

#[derive(Clone, Debug)]
pub struct ComponentSlot {
    pub name: String,
    pub content: Box<ViewNode>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ComponentEventRoute {
    pub name: String,
    pub route: Option<Route>,
    pub span: Span,
}
