use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Column,
    Row,
    Scroll,
    Grid,
    Stack,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutOptions {
    pub columns: Option<Expr>,
    pub clip: Option<Expr>,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub spacing: Option<Expr>,
    pub padding: PaddingOptions,
    pub max_width: Option<Expr>,
    pub max_height: Option<Expr>,
    pub align: Option<FlexAlignment>,
    pub wrap: bool,
    pub wrap_spacing: Option<Expr>,
    pub wrap_align: Option<FlexAlignment>,
    pub flexbox: Option<FlexboxOptions>,
    pub min_cell: Option<Expr>,
    pub max_cell: Option<Expr>,
    pub grid_height: Option<GridSizing>,
    pub under: u16,
    pub scroll: Option<ScrollOptions>,
}

pub(crate) fn keyed_column_semantic_key(options: &LayoutOptions) -> String {
    fn length_key(length: &Option<LengthValue>) -> String {
        match length {
            None => "none".into(),
            Some(LengthValue::Fill) => "fill".into(),
            Some(LengthValue::FillPortion(portion)) => format!("fill-portion:{portion}"),
            Some(LengthValue::Shrink) => "shrink".into(),
            Some(LengthValue::Fixed(_)) => "fixed".into(),
        }
    }

    format!(
        "keyed|width={}|height={}|spacing={}|padding={:?}|max-width={}|align={:?}",
        length_key(&options.width),
        length_key(&options.height),
        options.spacing.is_some(),
        [
            options.padding.all.is_some(),
            options.padding.x.is_some(),
            options.padding.y.is_some(),
            options.padding.top.is_some(),
            options.padding.right.is_some(),
            options.padding.bottom.is_some(),
            options.padding.left.is_some(),
        ],
        options.max_width.is_some(),
        options.align,
    )
}

#[derive(Clone, Debug, Default)]
pub struct ContainerOptions {
    pub padding: PaddingOptions,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub max_width: Option<Expr>,
    pub max_height: Option<Expr>,
    pub align_x: Option<FlexAlignment>,
    pub align_y: Option<FlexAlignment>,
    pub clip: Option<Expr>,
    pub custom_style: Option<ExternCall>,
    pub style: ContainerStyleOptions,
    /// The on/off pattern of a dashed border, empty when the border is solid.
    pub border_dash: Vec<Expr>,
    pub flex_item: FlexItemOptions,
}

pub(crate) fn container_expression_roots(options: &ContainerOptions) -> Vec<&Expr> {
    let mut roots = Vec::new();
    roots.extend(
        [
            &options.padding.all,
            &options.padding.x,
            &options.padding.y,
            &options.padding.top,
            &options.padding.right,
            &options.padding.bottom,
            &options.padding.left,
        ]
        .into_iter()
        .flatten(),
    );
    for length in [&options.width, &options.height].into_iter().flatten() {
        if let LengthValue::Fixed(expression) = length {
            roots.push(expression);
        }
    }
    roots.extend(
        [&options.max_width, &options.max_height, &options.clip]
            .into_iter()
            .flatten(),
    );
    if let Some(style) = &options.custom_style {
        roots.extend(&style.args);
    }
    if let Some(BackgroundValue::Linear { angle, stops }) = &options.style.background {
        roots.push(angle);
        roots.extend(stops.iter().map(|stop| &stop.offset));
    }
    roots.extend(
        [
            &options.style.border_width,
            &options.style.radius,
            &options.style.radius_top_left,
            &options.style.radius_top_right,
            &options.style.radius_bottom_right,
            &options.style.radius_bottom_left,
            &options.style.shadow_x,
            &options.style.shadow_y,
            &options.style.shadow_blur,
            &options.style.pixel_snap,
        ]
        .into_iter()
        .flatten(),
    );
    roots.extend(&options.border_dash);
    roots.extend(
        [
            &options.flex_item.order,
            &options.flex_item.grow,
            &options.flex_item.shrink,
        ]
        .into_iter()
        .flatten(),
    );
    if let Some(FlexBasisValue::Fixed(expression) | FlexBasisValue::Percent(expression)) =
        &options.flex_item.basis
    {
        roots.push(expression);
    }
    for margin in [
        &options.flex_item.margin.all,
        &options.flex_item.margin.x,
        &options.flex_item.margin.y,
        &options.flex_item.margin.top,
        &options.flex_item.margin.right,
        &options.flex_item.margin.bottom,
        &options.flex_item.margin.left,
    ]
    .into_iter()
    .flatten()
    {
        if let FlexMarginValue::Fixed(expression) | FlexMarginValue::Percent(expression) = margin {
            roots.push(expression);
        }
    }
    roots
}

pub(crate) fn container_semantic_key(options: &ContainerOptions) -> String {
    let present = |value: bool| if value { '1' } else { '0' };
    let length = |value: &Option<LengthValue>| match value {
        None => "none".into(),
        Some(LengthValue::Fill) => "fill".into(),
        Some(LengthValue::FillPortion(portion)) => format!("fill:{portion}"),
        Some(LengthValue::Shrink) => "shrink".into(),
        Some(LengthValue::Fixed(_)) => "fixed".into(),
    };
    let background = match &options.style.background {
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
    };
    let basis = match &options.flex_item.basis {
        None => "none",
        Some(FlexBasisValue::Auto) => "auto",
        Some(FlexBasisValue::Content) => "content",
        Some(FlexBasisValue::Fixed(_)) => "fixed",
        Some(FlexBasisValue::Percent(_)) => "percent",
    };
    let margin = |value: &Option<FlexMarginValue>| match value {
        None => 'n',
        Some(FlexMarginValue::Auto) => 'a',
        Some(FlexMarginValue::Fixed(_)) => 'f',
        Some(FlexMarginValue::Percent(_)) => 'p',
    };
    let custom = options.custom_style.as_ref().map_or_else(
        || "none".into(),
        |style| format!("{}:{}", style.function, style.args.len()),
    );
    format!(
        "container|padding={}|size={}:{}|max={}{}|align={:?}:{:?}|clip={}|custom={custom}|surface={background}:{:?}:{:?}:{}{}{}{}{}:{:?}:{}{}{}{}|dash={}|flex={}{}{}:{basis}:{:?}:{}{}{}{}{}{}{}",
        [
            options.padding.all.is_some(),
            options.padding.x.is_some(),
            options.padding.y.is_some(),
            options.padding.top.is_some(),
            options.padding.right.is_some(),
            options.padding.bottom.is_some(),
            options.padding.left.is_some(),
        ]
        .into_iter()
        .map(present)
        .collect::<String>(),
        length(&options.width),
        length(&options.height),
        present(options.max_width.is_some()),
        present(options.max_height.is_some()),
        options.align_x,
        options.align_y,
        present(options.clip.is_some()),
        options.style.text_color,
        options.style.border_color,
        present(options.style.border_width.is_some()),
        present(options.style.radius.is_some()),
        present(options.style.radius_top_left.is_some()),
        present(options.style.radius_top_right.is_some()),
        present(options.style.radius_bottom_right.is_some()),
        options.style.shadow_color,
        present(options.style.radius_bottom_left.is_some()),
        present(options.style.shadow_x.is_some()),
        present(options.style.shadow_y.is_some()),
        present(options.style.shadow_blur.is_some()),
        options.border_dash.len(),
        present(options.flex_item.order.is_some()),
        present(options.flex_item.grow.is_some()),
        present(options.flex_item.shrink.is_some()),
        options.flex_item.align_self,
        margin(&options.flex_item.margin.all),
        margin(&options.flex_item.margin.x),
        margin(&options.flex_item.margin.y),
        margin(&options.flex_item.margin.top),
        margin(&options.flex_item.margin.right),
        margin(&options.flex_item.margin.bottom),
        margin(&options.flex_item.margin.left),
    ) + &format!("|snap={}", present(options.style.pixel_snap.is_some()))
}

#[derive(Clone, Debug)]
pub struct FlexboxOptions {
    pub direction: FlexDirectionValue,
    pub wrap: FlexWrapValue,
    pub justify_content: Option<FlexContentAlignment>,
    pub align_items: Option<FlexItemAlignment>,
    pub align_content: Option<FlexContentAlignment>,
    pub row_gap: Option<Expr>,
    pub column_gap: Option<Expr>,
}

impl Default for FlexboxOptions {
    fn default() -> Self {
        Self {
            direction: FlexDirectionValue::Row,
            wrap: FlexWrapValue::NoWrap,
            justify_content: None,
            align_items: None,
            align_content: None,
            row_gap: None,
            column_gap: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexDirectionValue {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexWrapValue {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexContentAlignment {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexItemAlignment {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Clone, Debug, Default)]
pub struct FlexItemOptions {
    pub order: Option<Expr>,
    pub grow: Option<Expr>,
    pub shrink: Option<Expr>,
    pub basis: Option<FlexBasisValue>,
    pub align_self: Option<FlexItemAlignment>,
    pub margin: FlexMarginOptions,
}

#[derive(Clone, Debug)]
pub enum FlexBasisValue {
    Auto,
    Content,
    Fixed(Expr),
    Percent(Expr),
}

#[derive(Clone, Debug, Default)]
pub struct FlexMarginOptions {
    pub all: Option<FlexMarginValue>,
    pub x: Option<FlexMarginValue>,
    pub y: Option<FlexMarginValue>,
    pub top: Option<FlexMarginValue>,
    pub right: Option<FlexMarginValue>,
    pub bottom: Option<FlexMarginValue>,
    pub left: Option<FlexMarginValue>,
}

#[derive(Clone, Debug)]
pub enum FlexMarginValue {
    Auto,
    Fixed(Expr),
    Percent(Expr),
}

#[derive(Clone, Debug)]
pub struct OverlayOptions {
    pub visible: Expr,
    pub dismiss: Option<Route>,
    pub backdrop: String,
    pub padding: Expr,
    pub align_x: FlexAlignment,
    pub align_y: FlexAlignment,
}

pub(crate) fn overlay_routes(options: &OverlayOptions) -> Vec<&Route> {
    options.dismiss.iter().collect()
}

pub(crate) fn overlay_semantic_key(options: &OverlayOptions) -> String {
    let dismiss = options.dismiss.as_ref().map_or_else(
        || "none".into(),
        |route| {
            let arguments = route
                .args
                .iter()
                .map(|argument| match argument {
                    RouteArg::Expr(_) => "expression",
                    RouteArg::Payload => "payload",
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{}({arguments})", route.handler)
        },
    );
    format!(
        "overlay|backdrop={}|align={:?}:{:?}|dismiss={dismiss}",
        options.backdrop, options.align_x, options.align_y
    )
}

#[derive(Clone, Copy, Debug)]
pub enum PaneAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub enum PaneConfiguration {
    Pane(String),
    Split {
        name: Option<String>,
        axis: PaneAxis,
        ratio: f32,
        a: Box<PaneConfiguration>,
        b: Box<PaneConfiguration>,
    },
}

#[derive(Clone, Debug)]
pub struct PaneView {
    pub name: String,
    pub maximized: Option<String>,
    pub content: Box<ViewNode>,
    pub title: Option<PaneTitle>,
    pub styles: Vec<String>,
    pub style: ContainerStyleOptions,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PaneTemplate {
    pub item: String,
    pub items: String,
    pub key: Expr,
    pub pane: PaneView,
    pub span: Span,
}

impl PaneView {
    pub fn nodes(&self) -> impl Iterator<Item = &ViewNode> {
        [
            Some(self.content.as_ref()),
            self.title.as_ref().map(|title| title.content.as_ref()),
            self.title
                .as_ref()
                .and_then(|title| title.controls.as_deref()),
            self.title
                .as_ref()
                .and_then(|title| title.compact_controls.as_deref()),
        ]
        .into_iter()
        .flatten()
    }
}

#[derive(Clone, Debug)]
pub struct PaneTitle {
    pub content: Box<ViewNode>,
    pub controls: Option<Box<ViewNode>>,
    pub compact_controls: Option<Box<ViewNode>>,
    pub padding: PaddingOptions,
    pub always_show_controls: bool,
    pub styles: Vec<String>,
    pub style: ContainerStyleOptions,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct PaneGridOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub spacing: Option<Expr>,
    pub min_size: Option<Expr>,
    pub resize_leeway: Option<Expr>,
    pub draggable: bool,
    pub click: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub style: PaneGridStyle,
}

#[derive(Clone, Debug, Default)]
pub struct PaneGridStyle {
    pub region_background: Option<BackgroundValue>,
    pub region_border: Option<String>,
    pub region_border_width: Option<Expr>,
    pub region_radius: Option<Expr>,
    pub region_radius_top_left: Option<Expr>,
    pub region_radius_top_right: Option<Expr>,
    pub region_radius_bottom_right: Option<Expr>,
    pub region_radius_bottom_left: Option<Expr>,
    pub hovered_split: Option<String>,
    pub hovered_split_width: Option<Expr>,
    pub picked_split: Option<String>,
    pub picked_split_width: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum BackgroundValue {
    Color(String),
    Linear {
        angle: Expr,
        stops: Vec<GradientStop>,
    },
}

#[derive(Clone, Debug)]
pub struct GradientStop {
    pub color: String,
    pub offset: Expr,
}

#[derive(Clone, Debug, Default)]
pub struct ContainerStyleOptions {
    pub background: Option<BackgroundValue>,
    pub text_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub shadow_color: Option<String>,
    pub shadow_x: Option<Expr>,
    pub shadow_y: Option<Expr>,
    pub shadow_blur: Option<Expr>,
    pub pixel_snap: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct PaddingOptions {
    pub all: Option<Expr>,
    pub x: Option<Expr>,
    pub y: Option<Expr>,
    pub top: Option<Expr>,
    pub right: Option<Expr>,
    pub bottom: Option<Expr>,
    pub left: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexAlignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug)]
pub enum GridSizing {
    AspectRatio { width: Expr, height: Expr },
    EvenlyDistribute(LengthValue),
}

#[derive(Clone, Debug)]
pub struct ScrollOptions {
    pub direction: ScrollDirection,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub hidden_bar: bool,
    pub bar_width: Option<Expr>,
    pub bar_margin: Option<Expr>,
    pub scroller_width: Option<Expr>,
    pub bar_spacing: Option<Expr>,
    pub anchor_x: ScrollAnchor,
    pub anchor_y: ScrollAnchor,
    pub auto_scroll: Option<Expr>,
    pub route: Option<Route>,
    pub viewport_route: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub styles: Vec<ScrollStatusStyle>,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            direction: ScrollDirection::Vertical,
            width: None,
            height: None,
            hidden_bar: false,
            bar_width: None,
            bar_margin: None,
            scroller_width: None,
            bar_spacing: None,
            anchor_x: ScrollAnchor::Start,
            anchor_y: ScrollAnchor::Start,
            auto_scroll: None,
            route: None,
            viewport_route: None,
            custom_style: None,
            styles: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollStatus {
    Active,
    Hovered,
    Dragged,
}

#[derive(Clone, Debug)]
pub struct ScrollStatusStyle {
    pub status: ScrollStatus,
    pub horizontal_interaction: Option<bool>,
    pub vertical_interaction: Option<bool>,
    pub horizontal_disabled: Option<bool>,
    pub vertical_disabled: Option<bool>,
    pub container: ContainerStyleOptions,
    pub horizontal_rail: ScrollRailStyle,
    pub vertical_rail: ScrollRailStyle,
    pub gap: Option<BackgroundValue>,
    pub auto_scroll: ContainerStyleOptions,
    pub auto_scroll_icon: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ScrollRailStyle {
    pub rail: ContainerStyleOptions,
    pub scroller: ContainerStyleOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAnchor {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
