use super::*;

#[derive(Clone, Debug, Default)]
pub struct RuleOptions {
    pub style: Option<RuleStyle>,
    pub fill: Option<RuleFill>,
    pub color: Option<String>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub snap: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct SliderOptions {
    pub default: Option<Expr>,
    pub shift_step: Option<Expr>,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub style: SliderStyleSet,
}

#[derive(Clone, Debug, Default)]
pub struct SliderStyleSet {
    pub custom: Option<ExternCall>,
    pub active: Option<SliderStyle>,
    pub hovered: Option<SliderStyle>,
    pub dragged: Option<SliderStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct SliderStyle {
    pub span: Option<Span>,
    pub rail_start: Option<BackgroundValue>,
    pub rail_end: Option<BackgroundValue>,
    pub rail_width: Option<Expr>,
    pub rail_border_color: Option<String>,
    pub rail_border_width: Option<Expr>,
    pub rail_radius: Option<Expr>,
    pub rail_radius_top_left: Option<Expr>,
    pub rail_radius_top_right: Option<Expr>,
    pub rail_radius_bottom_right: Option<Expr>,
    pub rail_radius_bottom_left: Option<Expr>,
    pub handle_shape: Option<SliderHandleShape>,
    pub handle_color: Option<BackgroundValue>,
    pub handle_border_color: Option<String>,
    pub handle_border_width: Option<Expr>,
    pub handle_radius: Option<Expr>,
    pub handle_radius_top_left: Option<Expr>,
    pub handle_radius_top_right: Option<Expr>,
    pub handle_radius_bottom_right: Option<Expr>,
    pub handle_radius_bottom_left: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum SliderHandleShape {
    Circle(Expr),
    Rectangle { width: u16 },
}

#[derive(Clone, Debug, Default)]
pub struct ProgressOptions {
    pub length: Option<LengthValue>,
    pub girth: Option<LengthValue>,
    pub style: Option<ProgressStyle>,
    pub custom_style: Option<ExternCall>,
    pub background: Option<BackgroundValue>,
    pub bar: Option<BackgroundValue>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressStyle {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStyle {
    Default,
    Weak,
}

#[derive(Clone, Debug)]
pub enum RuleFill {
    Full,
    Percent(Expr),
    Padded(u16),
    AsymmetricPadding(u16, u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Default)]
pub struct PickListOptions {
    pub placeholder: Option<Expr>,
    pub width: Option<LengthValue>,
    pub menu_height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub font: Option<FontPreset>,
    pub handle: Option<PickListHandle>,
    pub open: Option<Route>,
    pub close: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub custom_menu_style: Option<ExternCall>,
    pub style: Box<PickListStyleSet>,
    pub menu_style: Option<Box<MenuStyleOptions>>,
}

#[derive(Clone, Debug, Default)]
pub struct PickListStyleSet {
    pub active: Option<PickListStatusStyle>,
    pub hovered: Option<PickListStatusStyle>,
    pub opened: Option<PickListStatusStyle>,
    pub opened_hovered: Option<PickListStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct PickListStatusStyle {
    pub options: ContainerStyleOptions,
    pub placeholder_color: Option<String>,
    pub handle_color: Option<String>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct MenuStyleOptions {
    pub options: ContainerStyleOptions,
    pub selected_text_color: Option<String>,
    pub selected_background: Option<BackgroundValue>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
pub enum PickListHandle {
    Arrow {
        size: Option<Expr>,
    },
    Static(PickListIcon),
    Dynamic {
        closed: PickListIcon,
        open: PickListIcon,
    },
    None,
}

#[derive(Clone, Debug)]
pub struct PickListIcon {
    pub code_point: char,
    pub font: Option<FontPreset>,
    pub size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ComboBoxOptions {
    pub width: Option<LengthValue>,
    pub menu_height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub font: Option<FontPreset>,
    pub icon: Option<TextInputIcon>,
    pub input: Option<Route>,
    pub hover: Option<Route>,
    pub open: Option<Route>,
    pub close: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub custom_menu_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
    pub menu_style: Option<Box<MenuStyleOptions>>,
}

#[derive(Clone, Debug, Default)]
pub struct TextInputStyleSet {
    pub active: Option<TextInputStatusStyle>,
    pub hovered: Option<TextInputStatusStyle>,
    pub focused: Option<TextInputStatusStyle>,
    pub focused_hovered: Option<TextInputStatusStyle>,
    pub disabled: Option<TextInputStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct TextInputStatusStyle {
    pub options: ContainerStyleOptions,
    pub icon_color: Option<String>,
    pub placeholder_color: Option<String>,
    pub value_color: Option<String>,
    pub selection_color: Option<String>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct TextInputIcon {
    pub code_point: char,
    pub font: Option<FontPreset>,
    pub size: Option<Expr>,
    pub spacing: Option<Expr>,
    pub side: IconSide,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Svg,
    Viewer,
}

#[derive(Clone, Debug, Default)]
pub struct MediaOptions {
    pub accessibility: Box<AccessibilityOptions>,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub fit: Option<Expr>,
    pub rotation: Option<Expr>,
    pub opacity: Option<Expr>,
    pub svg_memory: bool,
    pub svg_color: Option<String>,
    pub svg_hover_color: Option<Option<String>>,
    pub svg_style: Option<ExternCall>,
    pub filter: Option<ImageFilter>,
    pub scale: Option<Expr>,
    pub expand: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub crop: Option<[Expr; 4]>,
    pub padding: Option<Expr>,
    pub min_scale: Option<Expr>,
    pub max_scale: Option<Expr>,
    pub scale_step: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum LengthValue {
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFilter {
    Linear,
    Nearest,
}

#[derive(Clone, Debug)]
pub struct TooltipOptions {
    pub position: TooltipPosition,
    pub gap: Expr,
    pub padding: Expr,
    pub delay_ms: Expr,
    pub snap: Expr,
    pub style: Option<TooltipStyle>,
    pub custom_style: Option<ExternCall>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipStyle {
    Transparent,
    Rounded,
    Bordered,
    Dark,
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
    FollowCursor,
}

#[derive(Clone, Debug, Default)]
pub struct MouseAreaOptions {
    pub press: Option<Route>,
    pub release: Option<Route>,
    pub double_click: Option<Route>,
    pub right_press: Option<Route>,
    pub right_release: Option<Route>,
    pub middle_press: Option<Route>,
    pub middle_release: Option<Route>,
    pub enter: Option<Route>,
    pub move_route: Option<Route>,
    pub scroll: Option<Route>,
    pub exit: Option<Route>,
    pub interaction: Option<MouseInteraction>,
    pub interaction_expr: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct ResizeHandleOptions {
    /// Payload route receiving `(dx, dy)` logical-pixel deltas per drag move.
    pub drag: Option<Route>,
    /// Route fired when the drag begins (left press over the handle).
    pub press: Option<Route>,
    /// Route fired when the drag ends (left release while dragging).
    pub release: Option<Route>,
    /// Cursor shown while hovering or dragging the handle.
    pub interaction: Option<MouseInteraction>,
}

pub(crate) fn mouse_area_routes(options: &MouseAreaOptions) -> Vec<&Route> {
    [
        &options.press,
        &options.release,
        &options.double_click,
        &options.right_press,
        &options.right_release,
        &options.middle_press,
        &options.middle_release,
        &options.enter,
        &options.exit,
        &options.move_route,
        &options.scroll,
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub(crate) fn resize_handle_routes(options: &ResizeHandleOptions) -> Vec<&Route> {
    [&options.drag, &options.press, &options.release]
        .into_iter()
        .flatten()
        .collect()
}

fn interaction_route_key(route: &Route) -> String {
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
}

pub(crate) fn mouse_area_semantic_key(options: &MouseAreaOptions) -> String {
    format!(
        "mouse|static={:?}|dynamic={}|routes={}",
        options.interaction,
        options.interaction_expr.is_some(),
        mouse_area_routes(options)
            .into_iter()
            .map(interaction_route_key)
            .collect::<Vec<_>>()
            .join("|")
    )
}

pub(crate) fn resize_handle_semantic_key(options: &ResizeHandleOptions) -> String {
    format!(
        "resize|interaction={:?}|routes={}",
        options.interaction,
        resize_handle_routes(options)
            .into_iter()
            .map(interaction_route_key)
            .collect::<Vec<_>>()
            .join("|")
    )
}

pub(crate) fn media_expression_roots<'a>(
    source: &'a Expr,
    options: &'a MediaOptions,
) -> Vec<&'a Expr> {
    let mut roots = vec![source];
    roots.extend(options.accessibility.label.iter());
    roots.extend(options.accessibility.description.iter());
    for length in [&options.width, &options.height].into_iter().flatten() {
        if let LengthValue::Fixed(value) = length {
            roots.push(value);
        }
    }
    roots.extend(options.fit.iter());
    roots.extend(options.rotation.iter());
    roots.extend(options.opacity.iter());
    if let Some(style) = &options.svg_style {
        roots.extend(&style.args);
    }
    roots.extend(options.scale.iter());
    roots.extend(options.expand.iter());
    for value in [
        &options.radius,
        &options.radius_top_left,
        &options.radius_top_right,
        &options.radius_bottom_right,
        &options.radius_bottom_left,
    ]
    .into_iter()
    .flatten()
    {
        roots.push(value);
    }
    if let Some(crop) = &options.crop {
        roots.extend(crop);
    }
    for value in [
        &options.padding,
        &options.min_scale,
        &options.max_scale,
        &options.scale_step,
    ]
    .into_iter()
    .flatten()
    {
        roots.push(value);
    }
    roots
}

pub(crate) fn media_semantic_key(kind: MediaKind, options: &MediaOptions) -> String {
    fn length(value: &Option<LengthValue>) -> String {
        match value {
            None => "none".into(),
            Some(LengthValue::Fill) => "fill".into(),
            Some(LengthValue::FillPortion(value)) => format!("fill:{value}"),
            Some(LengthValue::Shrink) => "shrink".into(),
            Some(LengthValue::Fixed(_)) => "fixed:_".into(),
        }
    }

    let hover = match &options.svg_hover_color {
        None => "inherit".into(),
        Some(None) => "none".into(),
        Some(Some(color)) => format!("color:{color}"),
    };
    let style = options.svg_style.as_ref().map_or_else(
        || "none".into(),
        |style| format!("{}:{}", style.function, style.args.len()),
    );
    let flag = |value: bool| if value { '1' } else { '0' };
    let present = |value: bool| flag(value);
    format!(
        "{kind:?}|a={}{}|w={}|h={}|fit={}|rotation={}|opacity={}|memory={}|color={:?}|hover={hover}|style={style}|filter={:?}|scale={}|expand={}|radius={}{}{}{}{}|crop={}|padding={}|min={}|max={}|step={}",
        present(options.accessibility.label.is_some()),
        present(options.accessibility.description.is_some()),
        length(&options.width),
        length(&options.height),
        present(options.fit.is_some()),
        present(options.rotation.is_some()),
        present(options.opacity.is_some()),
        flag(options.svg_memory),
        options.svg_color,
        options.filter,
        present(options.scale.is_some()),
        present(options.expand.is_some()),
        present(options.radius.is_some()),
        present(options.radius_top_left.is_some()),
        present(options.radius_top_right.is_some()),
        present(options.radius_bottom_right.is_some()),
        present(options.radius_bottom_left.is_some()),
        present(options.crop.is_some()),
        present(options.padding.is_some()),
        present(options.min_scale.is_some()),
        present(options.max_scale.is_some()),
        present(options.scale_step.is_some()),
    )
}

pub(crate) fn tooltip_expression_roots(options: &TooltipOptions) -> Vec<&Expr> {
    let mut roots = vec![
        &options.gap,
        &options.padding,
        &options.delay_ms,
        &options.snap,
    ];
    if let Some(style) = &options.custom_style {
        roots.extend(&style.args);
    }
    if let Some(BackgroundValue::Linear { angle, stops }) = &options.background {
        roots.push(angle);
        roots.extend(stops.iter().map(|stop| &stop.offset));
    }
    for value in [
        &options.border_width,
        &options.radius,
        &options.radius_top_left,
        &options.radius_top_right,
        &options.radius_bottom_right,
        &options.radius_bottom_left,
        &options.shadow_x,
        &options.shadow_y,
        &options.shadow_blur,
        &options.pixel_snap,
    ]
    .into_iter()
    .flatten()
    {
        roots.push(value);
    }
    roots
}

pub(crate) fn tooltip_semantic_key(options: &TooltipOptions) -> String {
    let style = options.custom_style.as_ref().map_or_else(
        || format!("preset:{:?}", options.style),
        |style| format!("custom:{}:{}", style.function, style.args.len()),
    );
    let background = match &options.background {
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
    let present = |value: bool| if value { '1' } else { '0' };
    format!(
        "position={:?}|style={style}|background={background}|text={:?}|border={:?}:{}|radius={}{}{}{}{}|shadow={:?}:{}{}{}|snap={}",
        options.position,
        options.text_color,
        options.border_color,
        present(options.border_width.is_some()),
        present(options.radius.is_some()),
        present(options.radius_top_left.is_some()),
        present(options.radius_top_right.is_some()),
        present(options.radius_bottom_right.is_some()),
        present(options.radius_bottom_left.is_some()),
        options.shadow_color,
        present(options.shadow_x.is_some()),
        present(options.shadow_y.is_some()),
        present(options.shadow_blur.is_some()),
        present(options.pixel_snap.is_some()),
    )
}
