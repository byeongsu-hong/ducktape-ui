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
    pub flex_item: FlexItemOptions,
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
