use super::*;

#[derive(Clone, Debug)]
pub enum ViewNode {
    Layout {
        kind: Layout,
        options: Box<LayoutOptions>,
        id: Option<Id>,
        styles: Vec<String>,
        children: Vec<ViewNode>,
        span: Span,
    },
    Container {
        options: Box<ContainerOptions>,
        id: Option<Id>,
        styles: Vec<String>,
        content: Box<ViewNode>,
        span: Span,
    },
    Overlay {
        id: Option<Id>,
        options: OverlayOptions,
        content: Box<ViewNode>,
        layer: Box<ViewNode>,
        span: Span,
    },
    PaneGrid {
        name: String,
        configuration: PaneConfiguration,
        options: PaneGridOptions,
        panes: Vec<PaneView>,
        templates: Vec<PaneTemplate>,
        span: Span,
    },
    Text {
        value: Expr,
        id: Option<Id>,
        options: TextOptions,
        styles: Vec<String>,
        span: Span,
    },
    RichText {
        id: Option<Id>,
        options: TextOptions,
        color: Option<String>,
        children: Vec<RichTextChild>,
        styles: Vec<String>,
        route: Option<Route>,
        span: Span,
    },
    Input {
        label: String,
        id: Option<Id>,
        binding: String,
        hint: Option<Expr>,
        disabled: Option<Expr>,
        options: InputOptions,
        styles: Vec<String>,
        span: Span,
    },
    Button {
        label: Option<String>,
        content: Option<Box<ViewNode>>,
        id: Option<Id>,
        disabled: Option<Expr>,
        options: ButtonOptions,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    Checkbox {
        label: Expr,
        id: Option<Id>,
        checked: Expr,
        disabled: Option<Expr>,
        options: BoolControlOptions,
        style: Box<CheckboxStyleSet>,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    Toggler {
        label: Expr,
        id: Option<Id>,
        checked: Expr,
        disabled: Option<Expr>,
        options: BoolControlOptions,
        style: Box<TogglerStyleSet>,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    Slider {
        value: Expr,
        id: Option<Id>,
        min: Expr,
        max: Expr,
        step: Expr,
        options: Box<SliderOptions>,
        vertical: bool,
        styles: Vec<String>,
        route: Route,
        release: Option<Route>,
        span: Span,
    },
    Progress {
        value: Expr,
        id: Option<Id>,
        min: Expr,
        max: Expr,
        options: ProgressOptions,
        vertical: bool,
        styles: Vec<String>,
        span: Span,
    },
    Radio {
        label: Expr,
        id: Option<Id>,
        value: Expr,
        selected: Expr,
        options: BoolControlOptions,
        style: Box<RadioStyleSet>,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    PickList {
        options: Expr,
        id: Option<Id>,
        selected: Expr,
        options_config: PickListOptions,
        route: Route,
        span: Span,
    },
    ComboBox {
        state: String,
        id: Option<Id>,
        selected: Expr,
        placeholder: Expr,
        options: ComboBoxOptions,
        route: Route,
        span: Span,
    },
    Rule {
        axis: Axis,
        id: Option<Id>,
        thickness: Expr,
        options: RuleOptions,
        styles: Vec<String>,
        span: Span,
    },
    QrCode {
        payload: Expr,
        id: Option<Id>,
        correction: Option<QrCorrection>,
        version: Option<QrVersion>,
        cell_size: Option<Expr>,
        total_size: Option<Expr>,
        cell: Option<String>,
        background: Option<String>,
        span: Span,
    },
    Space {
        id: Option<Id>,
        width: Option<LengthValue>,
        height: Option<LengthValue>,
        styles: Vec<String>,
        span: Span,
    },
    If {
        condition: Expr,
        children: Vec<ViewNode>,
        span: Span,
    },
    Match {
        value: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
    For {
        item: String,
        items: Expr,
        children: Vec<ViewNode>,
        span: Span,
    },
    KeyedColumn {
        item: String,
        items: Expr,
        key: Expr,
        id: Option<Id>,
        options: Box<LayoutOptions>,
        child: Box<ViewNode>,
        span: Span,
    },
    Lazy {
        dependency: Expr,
        /// Cheap projections that replace the value in the memo dependency
        /// tuple: `lazy value by key, key as name`. Empty for the plain form,
        /// whose dependency is the value itself.
        keys: Vec<Expr>,
        binding: String,
        id: Option<Id>,
        child: Box<ViewNode>,
        span: Span,
    },
    Markdown {
        content: String,
        id: Option<Id>,
        options: Box<MarkdownOptions>,
        route: Route,
        span: Span,
    },
    TextEditor {
        binding: String,
        id: Option<Id>,
        disabled: Option<Expr>,
        options: TextEditorOptions,
        span: Span,
    },
    Table {
        item: String,
        rows: Expr,
        id: Option<Id>,
        options: TableOptions,
        columns: Vec<TableColumn>,
        span: Span,
    },
    Component {
        name: String,
        args: Vec<ComponentArg>,
        id: Option<Id>,
        slots: Vec<ComponentSlot>,
        events: Vec<ComponentEventRoute>,
        route: Option<Route>,
        span: Span,
    },
    Slot {
        name: String,
        optional: bool,
        span: Span,
    },
    ExternComponent {
        function: String,
        id: Option<Id>,
        args: Vec<Expr>,
        route: Option<Route>,
        span: Span,
    },
    Themer {
        function: String,
        id: Option<Id>,
        args: Vec<Expr>,
        route: Option<Route>,
        span: Span,
    },
    Shader {
        function: String,
        id: Option<Id>,
        args: Vec<Expr>,
        width: Option<LengthValue>,
        height: Option<LengthValue>,
        route: Option<Route>,
        span: Span,
    },
    Media {
        kind: MediaKind,
        id: Option<Id>,
        source: Expr,
        options: MediaOptions,
        span: Span,
    },
    Tooltip {
        id: Option<Id>,
        options: TooltipOptions,
        content: Box<ViewNode>,
        tip: Box<ViewNode>,
        span: Span,
    },
    MouseArea {
        id: Option<Id>,
        options: MouseAreaOptions,
        content: Box<ViewNode>,
        span: Span,
    },
    ResizeHandle {
        id: Option<Id>,
        options: ResizeHandleOptions,
        content: Box<ViewNode>,
        span: Span,
    },
    Canvas {
        id: Option<Id>,
        options: Box<CanvasOptions>,
        locals: Vec<State>,
        commands: Vec<CanvasCommand>,
        events: Vec<CanvasEvent>,
        span: Span,
    },
    Theme {
        id: Option<Id>,
        preset: ThemePreset,
        text: Option<String>,
        background: Option<BackgroundValue>,
        content: Box<ViewNode>,
        span: Span,
    },
    Float {
        id: Option<Id>,
        scale: Expr,
        x: Expr,
        y: Expr,
        style: FloatStyleOptions,
        content: Box<ViewNode>,
        span: Span,
    },
    Pin {
        id: Option<Id>,
        width: Option<LengthValue>,
        height: Option<LengthValue>,
        x: Expr,
        y: Expr,
        content: Box<ViewNode>,
        span: Span,
    },
    Sensor {
        id: Option<Id>,
        options: SensorOptions,
        content: Box<ViewNode>,
        span: Span,
    },
    Responsive {
        id: Option<Id>,
        content: ResponsiveContent,
        width: Option<LengthValue>,
        height: Option<LengthValue>,
        span: Span,
    },
}

pub(crate) fn extern_component_semantic_key(
    function: &str,
    args: &[Expr],
    route: &Option<Route>,
) -> String {
    format!(
        "extern-component|function={function}|arguments={}|route={}",
        args.len(),
        route.is_some()
    )
}

pub(crate) fn component_call_route_semantic_key<'a>(
    component: &str,
    has_output_route: bool,
    events: impl IntoIterator<Item = (&'a str, bool)>,
) -> String {
    let events = events
        .into_iter()
        .map(|(name, direct)| format!("{name}:{}", if direct { "direct" } else { "forward" }))
        .collect::<Vec<_>>()
        .join(",");
    format!("component-call-routes|component={component}|output={has_output_route}|events={events}")
}

pub(crate) fn themer_semantic_key(function: &str, args: &[Expr], route: &Option<Route>) -> String {
    format!(
        "themer|function={function}|arguments={}|route={}",
        args.len(),
        route.is_some()
    )
}

pub(crate) fn shader_semantic_key(
    function: &str,
    args: &[Expr],
    width: &Option<LengthValue>,
    height: &Option<LengthValue>,
    route: &Option<Route>,
) -> String {
    format!(
        "shader|function={function}|arguments={}|width={}|height={}|route={}",
        args.len(),
        length_semantic_key(width),
        length_semantic_key(height),
        route.is_some()
    )
}

fn nested_theme_background_semantic_key(background: &Option<BackgroundValue>) -> String {
    match background {
        None => "none".into(),
        Some(BackgroundValue::Color(color)) => format!("color:{color}"),
        Some(BackgroundValue::Linear { stops, .. }) => format!(
            "linear:{}",
            stops
                .iter()
                .map(|stop| stop.color.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

pub(crate) fn nested_theme_semantic_key(
    preset: &ThemePreset,
    text: &Option<String>,
    background: &Option<BackgroundValue>,
) -> String {
    let preset = match preset {
        ThemePreset::Default => "default".into(),
        ThemePreset::App => "app".into(),
        ThemePreset::BuiltIn(name) => format!("built-in:{name}"),
        ThemePreset::Factory(factory) => {
            format!("factory:{}:{}", factory.function, factory.args.len())
        }
    };
    let text = text
        .as_ref()
        .map(|color| format!("color:{color}"))
        .unwrap_or_else(|| "none".into());
    format!(
        "nested-theme|preset={preset}|text={}|background={}",
        text,
        nested_theme_background_semantic_key(background)
    )
}

pub(crate) fn nested_theme_expression_roots<'a>(
    preset: &'a ThemePreset,
    background: &'a Option<BackgroundValue>,
) -> Vec<&'a Expr> {
    let mut expressions = match preset {
        ThemePreset::Factory(factory) => factory.args.iter().collect(),
        ThemePreset::Default | ThemePreset::App | ThemePreset::BuiltIn(_) => Vec::new(),
    };
    if let Some(BackgroundValue::Linear { angle, stops }) = background {
        expressions.push(angle);
        expressions.extend(stops.iter().map(|stop| &stop.offset));
    }
    expressions
}

impl ViewNode {
    pub(crate) fn identity(&self) -> Option<&Id> {
        match self {
            Self::Layout { id, .. }
            | Self::Container { id, .. }
            | Self::Overlay { id, .. }
            | Self::Text { id, .. }
            | Self::RichText { id, .. }
            | Self::Input { id, .. }
            | Self::Button { id, .. }
            | Self::Checkbox { id, .. }
            | Self::Toggler { id, .. }
            | Self::Slider { id, .. }
            | Self::Progress { id, .. }
            | Self::Radio { id, .. }
            | Self::PickList { id, .. }
            | Self::ComboBox { id, .. }
            | Self::Rule { id, .. }
            | Self::QrCode { id, .. }
            | Self::Space { id, .. }
            | Self::KeyedColumn { id, .. }
            | Self::Lazy { id, .. }
            | Self::Markdown { id, .. }
            | Self::TextEditor { id, .. }
            | Self::Table { id, .. }
            | Self::Component { id, .. }
            | Self::ExternComponent { id, .. }
            | Self::Themer { id, .. }
            | Self::Shader { id, .. }
            | Self::Media { id, .. }
            | Self::Tooltip { id, .. }
            | Self::MouseArea { id, .. }
            | Self::ResizeHandle { id, .. }
            | Self::Canvas { id, .. }
            | Self::Theme { id, .. }
            | Self::Float { id, .. }
            | Self::Pin { id, .. }
            | Self::Sensor { id, .. }
            | Self::Responsive { id, .. } => id.as_ref(),
            Self::PaneGrid { .. }
            | Self::If { .. }
            | Self::Match { .. }
            | Self::For { .. }
            | Self::Slot { .. } => None,
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            Self::Layout { span, .. }
            | Self::Container { span, .. }
            | Self::Overlay { span, .. }
            | Self::PaneGrid { span, .. }
            | Self::Text { span, .. }
            | Self::RichText { span, .. }
            | Self::Input { span, .. }
            | Self::Button { span, .. }
            | Self::Checkbox { span, .. }
            | Self::Toggler { span, .. }
            | Self::Slider { span, .. }
            | Self::Progress { span, .. }
            | Self::Radio { span, .. }
            | Self::PickList { span, .. }
            | Self::ComboBox { span, .. }
            | Self::Rule { span, .. }
            | Self::QrCode { span, .. }
            | Self::Space { span, .. }
            | Self::If { span, .. }
            | Self::Match { span, .. }
            | Self::For { span, .. }
            | Self::KeyedColumn { span, .. }
            | Self::Lazy { span, .. }
            | Self::Markdown { span, .. }
            | Self::TextEditor { span, .. }
            | Self::Table { span, .. }
            | Self::Component { span, .. }
            | Self::Slot { span, .. }
            | Self::ExternComponent { span, .. }
            | Self::Themer { span, .. }
            | Self::Shader { span, .. }
            | Self::Media { span, .. }
            | Self::Tooltip { span, .. }
            | Self::MouseArea { span, .. }
            | Self::ResizeHandle { span, .. }
            | Self::Canvas { span, .. }
            | Self::Theme { span, .. }
            | Self::Float { span, .. }
            | Self::Pin { span, .. }
            | Self::Sensor { span, .. }
            | Self::Responsive { span, .. } => span,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub children: Vec<ViewNode>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum MatchPattern {
    Some(String),
    None,
    Ok(String),
    Err(String),
    Enum {
        enum_name: String,
        variant: String,
        binding: Option<String>,
    },
    Wildcard,
}

impl MatchPattern {
    pub fn binding(&self) -> Option<&str> {
        match self {
            Self::Some(binding) | Self::Ok(binding) | Self::Err(binding) => Some(binding),
            Self::Enum {
                binding: Some(binding),
                ..
            } => Some(binding),
            Self::None | Self::Enum { binding: None, .. } | Self::Wildcard => None,
        }
    }
}
