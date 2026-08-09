pub const ANIMATION_EASINGS: &[&str] = &[
    "linear",
    "ease-in",
    "ease-out",
    "ease-in-out",
    "ease-in-quad",
    "ease-out-quad",
    "ease-in-out-quad",
    "ease-in-cubic",
    "ease-out-cubic",
    "ease-in-out-cubic",
    "ease-in-quart",
    "ease-out-quart",
    "ease-in-out-quart",
    "ease-in-quint",
    "ease-out-quint",
    "ease-in-out-quint",
    "ease-in-expo",
    "ease-out-expo",
    "ease-in-out-expo",
    "ease-in-circ",
    "ease-out-circ",
    "ease-in-out-circ",
    "ease-in-back",
    "ease-out-back",
    "ease-in-out-back",
    "ease-in-elastic",
    "ease-out-elastic",
    "ease-in-out-elastic",
    "ease-in-bounce",
    "ease-out-bounce",
    "ease-in-out-bounce",
];

pub(crate) const BUILT_IN_THEMES: &[&str] = &[
    "light",
    "dark",
    "dracula",
    "nord",
    "solarized-light",
    "solarized-dark",
    "gruvbox-light",
    "gruvbox-dark",
    "catppuccin-latte",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "catppuccin-mocha",
    "tokyo-night",
    "tokyo-night-storm",
    "tokyo-night-light",
    "kanagawa-wave",
    "kanagawa-dragon",
    "kanagawa-lotus",
    "moonfly",
    "nightfly",
    "oxocarbon",
    "ferra",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn line(line: usize) -> Self {
        Self { line, column: 1 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Bool,
    I64,
    F64,
    Str,
    Bytes,
    Image,
    ImageAllocation,
    ImageMemory,
    ImageError,
    DebugSpan,
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Combo(Box<Type>),
    Animation(Box<Type>),
    Markdown,
    Editor,
    Event,
    EventStatus,
    Key,
    PhysicalKey,
    KeyLocation,
    KeyPress,
    KeyRelease,
    KeyModifiers,
    Pixels,
    Padding,
    Degrees,
    Radians,
    Rotation,
    ContentFit,
    Color,
    Background,
    Gradient,
    LinearGradient,
    ColorStop,
    Font,
    FontFamily,
    FontWeight,
    FontStretch,
    FontStyle,
    ThemeMode,
    TextAlignment,
    TextShaping,
    TextWrapping,
    TextLineHeight,
    Length,
    Alignment,
    HorizontalAlignment,
    VerticalAlignment,
    Border,
    Radius,
    Shadow,
    Point,
    PointU32,
    Vector,
    Size,
    SizeU32,
    Rectangle,
    RectangleU32,
    Transformation,
    MouseInteraction,
    ScrollDelta,
    MouseButton,
    MouseCursor,
    MouseClick,
    TouchFinger,
    SystemInfo,
    Instant,
    WindowId,
    WindowScreenshot,
    WindowPosition,
    RedrawRequest,
    WindowDirection,
    WindowLevel,
    WindowMode,
    WindowAttention,
    WidgetId,
    WidgetTarget,
    TestTarget,
    TaskHandle,
    Palette(String),
    Named(String),
    Unit,
    Unknown,
}

impl Type {
    pub fn display(&self) -> String {
        match self {
            Self::Bool => "bool".into(),
            Self::I64 => "i64".into(),
            Self::F64 => "f64".into(),
            Self::Str => "str".into(),
            Self::Bytes => "bytes".into(),
            Self::Image => "image".into(),
            Self::ImageAllocation => "image-allocation".into(),
            Self::ImageMemory => "image-memory".into(),
            Self::ImageError => "image-error".into(),
            Self::DebugSpan => "debug-span".into(),
            Self::List(inner) => format!("[{}]", inner.display()),
            Self::Option(inner) => format!("{}?", inner.display()),
            Self::Result(output, error) => {
                format!("result[{},{}]", output.display(), error.display())
            }
            Self::Combo(inner) => format!("combo[{}]", inner.display()),
            Self::Animation(inner) => format!("animation[{}]", inner.display()),
            Self::Markdown => "markdown".into(),
            Self::Editor => "editor".into(),
            Self::Event => "event".into(),
            Self::EventStatus => "event-status".into(),
            Self::Key => "key".into(),
            Self::PhysicalKey => "physical-key".into(),
            Self::KeyLocation => "key-location".into(),
            Self::KeyPress => "key-press".into(),
            Self::KeyRelease => "key-release".into(),
            Self::KeyModifiers => "key-modifiers".into(),
            Self::Pixels => "pixels".into(),
            Self::Padding => "padding".into(),
            Self::Degrees => "degrees".into(),
            Self::Radians => "radians".into(),
            Self::Rotation => "rotation".into(),
            Self::ContentFit => "content-fit".into(),
            Self::Color => "color".into(),
            Self::Background => "background".into(),
            Self::Gradient => "gradient".into(),
            Self::LinearGradient => "linear-gradient".into(),
            Self::ColorStop => "color-stop".into(),
            Self::Font => "font".into(),
            Self::FontFamily => "font-family".into(),
            Self::FontWeight => "font-weight".into(),
            Self::FontStretch => "font-stretch".into(),
            Self::FontStyle => "font-style".into(),
            Self::ThemeMode => "theme-mode".into(),
            Self::TextAlignment => "text-alignment".into(),
            Self::TextShaping => "text-shaping".into(),
            Self::TextWrapping => "text-wrapping".into(),
            Self::TextLineHeight => "text-line-height".into(),
            Self::Length => "length".into(),
            Self::Alignment => "alignment".into(),
            Self::HorizontalAlignment => "horizontal-alignment".into(),
            Self::VerticalAlignment => "vertical-alignment".into(),
            Self::Border => "border".into(),
            Self::Radius => "radius".into(),
            Self::Shadow => "shadow".into(),
            Self::Point => "point".into(),
            Self::PointU32 => "point-u32".into(),
            Self::Vector => "vector".into(),
            Self::Size => "size".into(),
            Self::SizeU32 => "size-u32".into(),
            Self::Rectangle => "rectangle".into(),
            Self::RectangleU32 => "rectangle-u32".into(),
            Self::Transformation => "transformation".into(),
            Self::MouseInteraction => "mouse-interaction".into(),
            Self::ScrollDelta => "scroll-delta".into(),
            Self::MouseButton => "mouse-button".into(),
            Self::MouseCursor => "mouse-cursor".into(),
            Self::MouseClick => "mouse-click".into(),
            Self::TouchFinger => "touch-finger".into(),
            Self::SystemInfo => "system-info".into(),
            Self::Instant => "instant".into(),
            Self::WindowId => "window-id".into(),
            Self::WindowScreenshot => "window-screenshot".into(),
            Self::WindowPosition => "window-position".into(),
            Self::RedrawRequest => "redraw-request".into(),
            Self::WindowDirection => "window-direction".into(),
            Self::WindowLevel => "window-level".into(),
            Self::WindowMode => "window-mode".into(),
            Self::WindowAttention => "window-attention".into(),
            Self::WidgetId => "widget-id".into(),
            Self::WidgetTarget => "widget-target".into(),
            Self::TestTarget => "test-target".into(),
            Self::TaskHandle => "task-handle".into(),
            Self::Palette(contract) => format!("palette[{contract}]"),
            Self::Named(name) => name.clone(),
            Self::Unit => "unit".into(),
            Self::Unknown => "unknown".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternKind {
    Future,
    Component,
    Shader,
    Task,
    Stream,
    Sip,
    Recipe,
    Selector,
    EventFilter,
    Pure,
    Sync,
    Subscription,
    Theme,
    Themer,
    Window,
    MarkdownViewer,
    EditorBinding,
    EditorAction,
    EditorHighlighter,
    EditorStyle,
    TextStyle,
    SliderStyle,
    ProgressStyle,
    ButtonStyle,
    CheckboxStyle,
    TogglerStyle,
    RadioStyle,
    ContainerStyle,
    SvgStyle,
    InputStyle,
    ScrollStyle,
    PickListStyle,
    MenuStyle,
    PaneGridStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventStatus {
    Any,
    Captured,
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMethodEvent {
    Opened,
    Preedit,
    Commit,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardEvent {
    Press,
    Release,
    Modifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEvent {
    Entered,
    Left,
    Moved,
    Pressed,
    Released,
    Wheel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchEvent {
    Pressed,
    Moved,
    Lifted,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEvent {
    Frame,
    Opened,
    Closed,
    Moved,
    Resized,
    Rescaled,
    CloseRequested,
    Focused,
    Unfocused,
    FileHovered,
    FileDropped,
    FilesHoveredLeft,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontFamily {
    Named(String),
    Serif,
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationDuration {
    VeryQuick,
    Quick,
    Slow,
    VerySlow,
    Milliseconds(u64),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FutureMode {
    #[default]
    Every,
    Latest,
    Replace,
}

#[derive(Clone, Copy, Debug)]
pub enum PaneEdge {
    Top,
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
pub enum PaneAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowDirection {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowMode {
    Windowed,
    Fullscreen,
    Hidden,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowAttention {
    Critical,
    Informational,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowLevel {
    Normal,
    AlwaysOnBottom,
    AlwaysOnTop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowPosition {
    Default,
    Centered,
    Specific(f64, f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    Future,
    Task,
    Stream,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskGroupKind {
    Parallel,
    Sequential,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasFillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasLineCap {
    #[default]
    Butt,
    Square,
    Round,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasLineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseInteraction {
    None,
    Hidden,
    Idle,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    ResizingHorizontally,
    ResizingVertically,
    ResizingDiagonallyUp,
    ResizingDiagonallyDown,
    ResizingColumn,
    ResizingRow,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFilter {
    Linear,
    Nearest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAlignment {
    Left,
    Center,
    Right,
}

impl std::str::FromStr for InputAlignment {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShaping {
    Auto,
    Basic,
    Advanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextWrapping {
    None,
    Word,
    Glyph,
    WordOrGlyph,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlignment {
    Default,
    Left,
    Center,
    Right,
    Justified,
}

impl std::str::FromStr for TextAlignment {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "default" => Ok(Self::Default),
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            "justified" => Ok(Self::Justified),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

impl std::str::FromStr for VerticalAlignment {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "top" => Ok(Self::Top),
            "center" => Ok(Self::Center),
            "bottom" => Ok(Self::Bottom),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexAlignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

pub(crate) fn test_keyboard_variant_name(name: &str) -> String {
    if name
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase())
    {
        return name.to_owned();
    }
    match name {
        "tv" => return "TV".into(),
        "avr-input" => return "AVRInput".into(),
        "avr-power" => return "AVRPower".into(),
        "dvr" => return "DVR".into(),
        _ => {}
    }
    name.split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect()
}
