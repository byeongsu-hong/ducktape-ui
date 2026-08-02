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

fn push_selection_length_root<'a>(roots: &mut Vec<&'a Expr>, length: &'a Option<LengthValue>) {
    if let Some(LengthValue::Fixed(value)) = length {
        roots.push(value);
    }
}

fn push_selection_background_roots<'a>(
    roots: &mut Vec<&'a Expr>,
    background: &'a Option<BackgroundValue>,
) {
    if let Some(BackgroundValue::Linear { angle, stops }) = background {
        roots.push(angle);
        roots.extend(stops.iter().map(|stop| &stop.offset));
    }
}

fn push_selection_surface_roots<'a>(roots: &mut Vec<&'a Expr>, surface: &'a ContainerStyleOptions) {
    push_selection_background_roots(roots, &surface.background);
    roots.extend(
        [
            &surface.border_width,
            &surface.radius,
            &surface.radius_top_left,
            &surface.radius_top_right,
            &surface.radius_bottom_right,
            &surface.radius_bottom_left,
            &surface.shadow_x,
            &surface.shadow_y,
            &surface.shadow_blur,
            &surface.pixel_snap,
        ]
        .into_iter()
        .flatten(),
    );
}

fn push_menu_roots<'a>(roots: &mut Vec<&'a Expr>, menu: &'a Option<Box<MenuStyleOptions>>) {
    let Some(menu) = menu else { return };
    push_selection_surface_roots(roots, &menu.options);
    push_selection_background_roots(roots, &menu.selected_background);
}

fn push_pick_icon_roots<'a>(roots: &mut Vec<&'a Expr>, icon: &'a PickListIcon) {
    roots.extend([&icon.size, &icon.line_height].into_iter().flatten());
}

fn push_pick_handle_roots<'a>(roots: &mut Vec<&'a Expr>, handle: &'a Option<PickListHandle>) {
    match handle {
        Some(PickListHandle::Arrow { size }) => roots.extend(size),
        Some(PickListHandle::Static(icon)) => push_pick_icon_roots(roots, icon),
        Some(PickListHandle::Dynamic { closed, open }) => {
            push_pick_icon_roots(roots, closed);
            push_pick_icon_roots(roots, open);
        }
        Some(PickListHandle::None) | None => {}
    }
}

pub(crate) fn pick_list_expression_roots<'a>(
    options: &'a Expr,
    selected: &'a Expr,
    config: &'a PickListOptions,
) -> Vec<&'a Expr> {
    let mut roots = vec![options, selected];
    roots.extend(&config.placeholder);
    push_selection_length_root(&mut roots, &config.width);
    push_selection_length_root(&mut roots, &config.menu_height);
    roots.extend(
        [&config.padding, &config.text_size, &config.line_height]
            .into_iter()
            .flatten(),
    );
    push_pick_handle_roots(&mut roots, &config.handle);
    if let Some(style) = &config.custom_style {
        roots.extend(&style.args);
    }
    if let Some(style) = &config.custom_menu_style {
        roots.extend(&style.args);
    }
    for status in [
        &config.style.active,
        &config.style.hovered,
        &config.style.opened,
        &config.style.opened_hovered,
    ]
    .into_iter()
    .flatten()
    {
        push_selection_surface_roots(&mut roots, &status.options);
    }
    push_menu_roots(&mut roots, &config.menu_style);
    roots
}

pub(crate) fn pick_list_routes<'a>(
    config: &'a PickListOptions,
    selection: &'a Route,
) -> Vec<&'a Route> {
    [Some(selection), config.open.as_ref(), config.close.as_ref()]
        .into_iter()
        .flatten()
        .collect()
}

pub(crate) fn combo_box_expression_roots<'a>(
    selected: &'a Expr,
    options: &'a ComboBoxOptions,
) -> Vec<&'a Expr> {
    let mut roots = vec![selected];
    push_selection_length_root(&mut roots, &options.width);
    push_selection_length_root(&mut roots, &options.menu_height);
    roots.extend(
        [&options.padding, &options.text_size, &options.line_height]
            .into_iter()
            .flatten(),
    );
    if let Some(icon) = &options.icon {
        roots.extend([&icon.size, &icon.spacing].into_iter().flatten());
    }
    if let Some(style) = &options.custom_style {
        roots.extend(&style.args);
    }
    if let Some(style) = &options.custom_menu_style {
        roots.extend(&style.args);
    }
    for status in [
        &options.style.active,
        &options.style.hovered,
        &options.style.focused,
        &options.style.focused_hovered,
        &options.style.disabled,
    ]
    .into_iter()
    .flatten()
    {
        push_selection_surface_roots(&mut roots, &status.options);
    }
    push_menu_roots(&mut roots, &options.menu_style);
    roots
}

pub(crate) fn combo_box_routes<'a>(
    options: &'a ComboBoxOptions,
    selection: &'a Route,
) -> Vec<&'a Route> {
    [
        Some(selection),
        options.input.as_ref(),
        options.hover.as_ref(),
        options.open.as_ref(),
        options.close.as_ref(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn selection_length_key(length: &Option<LengthValue>) -> &'static str {
    match length {
        None => "none",
        Some(LengthValue::Fill) => "fill",
        Some(LengthValue::FillPortion(_)) => "fill-portion",
        Some(LengthValue::Shrink) => "shrink",
        Some(LengthValue::Fixed(_)) => "fixed",
    }
}

fn selection_route_key(route: Option<&Route>) -> String {
    route.map_or_else(
        || "none".into(),
        |route| {
            let arguments = route
                .args
                .iter()
                .map(|argument| match argument {
                    RouteArg::Expr(_) => 'e',
                    RouteArg::Payload => 'p',
                })
                .collect::<String>();
            format!("{}:{arguments}", route.handler)
        },
    )
}

fn selection_background_key(background: &Option<BackgroundValue>) -> String {
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

fn selection_surface_key(surface: &ContainerStyleOptions) -> String {
    format!(
        "bg={}|colors={:?}|fields={:?}",
        selection_background_key(&surface.background),
        [
            surface.text_color.as_deref(),
            surface.border_color.as_deref(),
            surface.shadow_color.as_deref(),
        ],
        [
            surface.border_width.is_some(),
            surface.radius.is_some(),
            surface.radius_top_left.is_some(),
            surface.radius_top_right.is_some(),
            surface.radius_bottom_right.is_some(),
            surface.radius_bottom_left.is_some(),
            surface.shadow_x.is_some(),
            surface.shadow_y.is_some(),
            surface.shadow_blur.is_some(),
            surface.pixel_snap.is_some(),
        ],
    )
}

fn menu_semantic_key(menu: &Option<Box<MenuStyleOptions>>) -> String {
    menu.as_ref().map_or_else(
        || "none".into(),
        |menu| {
            format!(
                "{}|selected={:?}:{}",
                selection_surface_key(&menu.options),
                menu.selected_text_color,
                selection_background_key(&menu.selected_background),
            )
        },
    )
}

fn pick_handle_semantic_key(handle: &Option<PickListHandle>) -> String {
    let icon = |icon: &PickListIcon| {
        format!(
            "{}:{:?}:{:?}:{}:{}",
            icon.code_point,
            icon.font,
            icon.shaping,
            icon.size.is_some(),
            icon.line_height.is_some(),
        )
    };
    match handle {
        None => "default".into(),
        Some(PickListHandle::Arrow { size }) => format!("arrow:{}", size.is_some()),
        Some(PickListHandle::Static(value)) => format!("static:{}", icon(value)),
        Some(PickListHandle::Dynamic { closed, open }) => {
            format!("dynamic:{}:{}", icon(closed), icon(open))
        }
        Some(PickListHandle::None) => "none".into(),
    }
}

pub(crate) fn pick_list_semantic_key(config: &PickListOptions, route: &Route) -> String {
    let custom = |style: Option<&ExternCall>| {
        style.map_or_else(
            || "none".into(),
            |style| format!("{}:{}", style.function, style.args.len()),
        )
    };
    let statuses = [
        &config.style.active,
        &config.style.hovered,
        &config.style.opened,
        &config.style.opened_hovered,
    ]
    .into_iter()
    .map(|status| {
        status.as_ref().map_or_else(
            || "none".into(),
            |status| {
                format!(
                    "{}|colors={:?}",
                    selection_surface_key(&status.options),
                    [
                        status.placeholder_color.as_deref(),
                        status.handle_color.as_deref(),
                    ],
                )
            },
        )
    })
    .collect::<Vec<_>>()
    .join(";");
    format!(
        "pick|placeholder={}|width={}|menu-height={}|metrics={:?}|shaping={:?}|font={:?}|handle={}|routes={:?}|custom={}:{}|statuses={statuses}|menu={}",
        config.placeholder.is_some(),
        selection_length_key(&config.width),
        selection_length_key(&config.menu_height),
        [
            config.padding.is_some(),
            config.text_size.is_some(),
            config.line_height.is_some(),
        ],
        config.shaping,
        config.font,
        pick_handle_semantic_key(&config.handle),
        [
            selection_route_key(Some(route)),
            selection_route_key(config.open.as_ref()),
            selection_route_key(config.close.as_ref()),
        ],
        custom(config.custom_style.as_ref()),
        custom(config.custom_menu_style.as_ref()),
        menu_semantic_key(&config.menu_style),
    )
}

pub(crate) fn combo_box_semantic_key(
    state: &str,
    placeholder: &str,
    options: &ComboBoxOptions,
    route: &Route,
) -> String {
    let custom = |style: Option<&ExternCall>| {
        style.map_or_else(
            || "none".into(),
            |style| format!("{}:{}", style.function, style.args.len()),
        )
    };
    let icon = options.icon.as_ref().map_or_else(
        || "none".into(),
        |icon| {
            format!(
                "{}:{:?}:{:?}:{}:{}",
                icon.code_point,
                icon.font,
                icon.side,
                icon.size.is_some(),
                icon.spacing.is_some(),
            )
        },
    );
    let statuses = [
        &options.style.active,
        &options.style.hovered,
        &options.style.focused,
        &options.style.focused_hovered,
        &options.style.disabled,
    ]
    .into_iter()
    .map(|status| {
        status.as_ref().map_or_else(
            || "none".into(),
            |status| {
                format!(
                    "{}|colors={:?}",
                    selection_surface_key(&status.options),
                    [
                        status.icon_color.as_deref(),
                        status.placeholder_color.as_deref(),
                        status.value_color.as_deref(),
                        status.selection_color.as_deref(),
                    ],
                )
            },
        )
    })
    .collect::<Vec<_>>()
    .join(";");
    format!(
        "combo|state={state}|placeholder={placeholder:?}|width={}|menu-height={}|metrics={:?}|shaping={:?}|font={:?}|icon={icon}|routes={:?}|custom={}:{}|statuses={statuses}|menu={}",
        selection_length_key(&options.width),
        selection_length_key(&options.menu_height),
        [
            options.padding.is_some(),
            options.text_size.is_some(),
            options.line_height.is_some(),
        ],
        options.shaping,
        options.font,
        [
            selection_route_key(Some(route)),
            selection_route_key(options.input.as_ref()),
            selection_route_key(options.hover.as_ref()),
            selection_route_key(options.open.as_ref()),
            selection_route_key(options.close.as_ref()),
        ],
        custom(options.custom_style.as_ref()),
        custom(options.custom_menu_style.as_ref()),
        menu_semantic_key(&options.menu_style),
    )
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
