use super::*;

#[derive(Clone, Debug, Default)]
pub struct CanvasOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub cache: Option<Expr>,
    pub cache_group: Option<String>,
    pub capture: Option<Expr>,
    pub press: Option<Route>,
    pub release: Option<Route>,
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
    pub interaction_outside: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct CanvasEvent {
    pub source: SubscriptionSource,
    pub bindings: Vec<String>,
    pub updates: Vec<CanvasStateUpdate>,
    pub action: Option<CanvasEventAction>,
    pub capture: bool,
    pub route_payload: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum CanvasEventAction {
    Route(Route),
    Redraw { after_ms: Option<u64> },
}

#[derive(Clone, Debug)]
pub struct CanvasStateUpdate {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum CanvasCommand {
    Rectangle {
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        radius: Box<CanvasRadius>,
        paint: Box<CanvasPaint>,
        span: Span,
    },
    Circle {
        x: Expr,
        y: Expr,
        radius: Expr,
        paint: Box<CanvasPaint>,
        span: Span,
    },
    Line {
        x1: Expr,
        y1: Expr,
        x2: Expr,
        y2: Expr,
        stroke: Box<CanvasStroke>,
        span: Span,
    },
    Text {
        value: Expr,
        x: Expr,
        y: Expr,
        max_width: Option<Expr>,
        color: Option<String>,
        size: Option<Expr>,
        line_height: Option<TextLineHeight>,
        font: Option<FontPreset>,
        align_x: Option<TextAlignment>,
        align_y: Option<VerticalAlignment>,
        shaping: Option<TextShaping>,
        span: Span,
    },
    Image {
        source: Expr,
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        filter: ImageFilter,
        rotation: Expr,
        opacity: Expr,
        snap: Expr,
        radius: Box<CanvasRadius>,
        span: Span,
    },
    Svg {
        source: Expr,
        memory: bool,
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        color: Option<String>,
        rotation: Expr,
        opacity: Expr,
        span: Span,
    },
    Path {
        segments: Vec<CanvasPathSegment>,
        paint: Box<CanvasPaint>,
        span: Span,
    },
    Group {
        transform: Box<CanvasTransform>,
        commands: Vec<CanvasCommand>,
        span: Span,
    },
    If {
        condition: Expr,
        commands: Vec<CanvasCommand>,
        span: Span,
    },
    For {
        item: String,
        items: Expr,
        commands: Vec<CanvasCommand>,
        span: Span,
    },
}

#[derive(Clone, Debug, Default)]
pub struct CanvasPaint {
    pub fill: Option<BackgroundValue>,
    pub fill_rule: CanvasFillRule,
    pub stroke: Option<CanvasStroke>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasFillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Clone, Debug)]
pub struct CanvasStroke {
    pub style: BackgroundValue,
    pub width: Expr,
    pub cap: CanvasLineCap,
    pub join: CanvasLineJoin,
    pub dash: Vec<Expr>,
    pub dash_offset: Expr,
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

#[derive(Clone, Debug, Default)]
pub struct CanvasRadius {
    pub all: Option<Expr>,
    pub top_left: Option<Expr>,
    pub top_right: Option<Expr>,
    pub bottom_right: Option<Expr>,
    pub bottom_left: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct CanvasTransform {
    pub x: Option<Expr>,
    pub y: Option<Expr>,
    pub rotate: Option<Expr>,
    pub scale: Option<Expr>,
    pub scale_x: Option<Expr>,
    pub scale_y: Option<Expr>,
    pub clip: Option<[Expr; 4]>,
}

#[derive(Clone, Debug)]
pub enum CanvasPathSegment {
    Move(Expr, Expr),
    Line(Expr, Expr),
    Arc {
        x: Expr,
        y: Expr,
        radius: Expr,
        start: Expr,
        end: Expr,
    },
    ArcTo {
        ax: Expr,
        ay: Expr,
        bx: Expr,
        by: Expr,
        radius: Expr,
    },
    Ellipse {
        x: Expr,
        y: Expr,
        radius_x: Expr,
        radius_y: Expr,
        rotation: Expr,
        start: Expr,
        end: Expr,
    },
    Bezier {
        control_ax: Expr,
        control_ay: Expr,
        control_bx: Expr,
        control_by: Expr,
        x: Expr,
        y: Expr,
    },
    Quadratic {
        control_x: Expr,
        control_y: Expr,
        x: Expr,
        y: Expr,
    },
    Rectangle {
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
    },
    RoundedRectangle {
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        radius: CanvasRadius,
    },
    Circle {
        x: Expr,
        y: Expr,
        radius: Expr,
    },
    Close,
}

pub(crate) fn canvas_command_spans(commands: &[CanvasCommand]) -> Vec<&Span> {
    fn collect<'a>(commands: &'a [CanvasCommand], spans: &mut Vec<&'a Span>) {
        for command in commands {
            let (span, children) = match command {
                CanvasCommand::Rectangle { span, .. }
                | CanvasCommand::Circle { span, .. }
                | CanvasCommand::Line { span, .. }
                | CanvasCommand::Text { span, .. }
                | CanvasCommand::Image { span, .. }
                | CanvasCommand::Svg { span, .. }
                | CanvasCommand::Path { span, .. } => (span, None),
                CanvasCommand::Group { span, commands, .. }
                | CanvasCommand::If { span, commands, .. }
                | CanvasCommand::For { span, commands, .. } => (span, Some(commands.as_slice())),
            };
            spans.push(span);
            if let Some(children) = children {
                collect(children, spans);
            }
        }
    }

    let mut spans = Vec::new();
    collect(commands, &mut spans);
    spans
}

pub(crate) fn canvas_command_span(command: &CanvasCommand) -> &Span {
    match command {
        CanvasCommand::Rectangle { span, .. }
        | CanvasCommand::Circle { span, .. }
        | CanvasCommand::Line { span, .. }
        | CanvasCommand::Text { span, .. }
        | CanvasCommand::Image { span, .. }
        | CanvasCommand::Svg { span, .. }
        | CanvasCommand::Path { span, .. }
        | CanvasCommand::Group { span, .. }
        | CanvasCommand::If { span, .. }
        | CanvasCommand::For { span, .. } => span,
    }
}

pub(crate) fn canvas_options_semantic_key(options: &CanvasOptions) -> String {
    fn length(value: &Option<LengthValue>) -> String {
        match value {
            None => "none".into(),
            Some(LengthValue::Fill) => "fill".into(),
            Some(LengthValue::FillPortion(value)) => format!("portion:{value}"),
            Some(LengthValue::Shrink) => "shrink".into(),
            Some(LengthValue::Fixed(_)) => "fixed:#".into(),
        }
    }
    let routes = [
        &options.press,
        &options.release,
        &options.right_press,
        &options.right_release,
        &options.middle_press,
        &options.middle_release,
        &options.enter,
        &options.move_route,
        &options.scroll,
        &options.exit,
    ]
    .map(Option::is_some);
    format!(
        "w={};h={};cache={};group={:?};capture={};routes={routes:?};interaction={:?};interaction_expr={};outside={}",
        length(&options.width),
        length(&options.height),
        options.cache.is_some(),
        options.cache_group,
        options.capture.is_some(),
        options.interaction,
        options.interaction_expr.is_some(),
        options.interaction_outside.is_some(),
    )
}

fn canvas_radius_semantic_key(radius: &CanvasRadius) -> String {
    format!(
        "{:?}",
        [
            radius.all.is_some(),
            radius.top_left.is_some(),
            radius.top_right.is_some(),
            radius.bottom_right.is_some(),
            radius.bottom_left.is_some(),
        ]
    )
}

fn canvas_background_semantic_key(background: &BackgroundValue) -> String {
    match background {
        BackgroundValue::Color(color) => format!("color:{color}"),
        BackgroundValue::Linear { stops, .. } => format!(
            "linear:{:?}",
            stops.iter().map(|stop| &stop.color).collect::<Vec<_>>()
        ),
    }
}

fn canvas_stroke_semantic_key(stroke: &CanvasStroke) -> String {
    format!(
        "{}:{:?}:{:?}:dash={}",
        canvas_background_semantic_key(&stroke.style),
        stroke.cap,
        stroke.join,
        stroke.dash.len()
    )
}

fn canvas_paint_semantic_key(paint: &CanvasPaint) -> String {
    format!(
        "fill={:?}:{:?};stroke={:?}",
        paint.fill.as_ref().map(canvas_background_semantic_key),
        paint.fill_rule,
        paint.stroke.as_ref().map(canvas_stroke_semantic_key)
    )
}

fn canvas_path_semantic_key(segments: &[CanvasPathSegment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            CanvasPathSegment::Move(..) => "move".into(),
            CanvasPathSegment::Line(..) => "line".into(),
            CanvasPathSegment::Arc { .. } => "arc".into(),
            CanvasPathSegment::ArcTo { .. } => "arc-to".into(),
            CanvasPathSegment::Ellipse { .. } => "ellipse".into(),
            CanvasPathSegment::Bezier { .. } => "bezier".into(),
            CanvasPathSegment::Quadratic { .. } => "quadratic".into(),
            CanvasPathSegment::Rectangle { .. } => "rectangle".into(),
            CanvasPathSegment::RoundedRectangle { radius, .. } => {
                format!("rounded:{}", canvas_radius_semantic_key(radius))
            }
            CanvasPathSegment::Circle { .. } => "circle".into(),
            CanvasPathSegment::Close => "close".into(),
        })
        .collect::<Vec<String>>()
        .join(",")
}

pub(crate) fn canvas_command_semantic_key(command: &CanvasCommand) -> String {
    match command {
        CanvasCommand::Rectangle { radius, paint, .. } => format!(
            "rectangle:r={};p={}",
            canvas_radius_semantic_key(radius),
            canvas_paint_semantic_key(paint)
        ),
        CanvasCommand::Circle { paint, .. } => {
            format!("circle:p={}", canvas_paint_semantic_key(paint))
        }
        CanvasCommand::Line { stroke, .. } => {
            format!("line:s={}", canvas_stroke_semantic_key(stroke))
        }
        CanvasCommand::Text {
            max_width,
            color,
            size,
            line_height,
            font,
            align_x,
            align_y,
            shaping,
            ..
        } => format!(
            "text:max={};color={color:?};size={};line={:?};font={font:?};x={align_x:?};y={align_y:?};shape={shaping:?}",
            max_width.is_some(),
            size.is_some(),
            line_height
                .as_ref()
                .map(|value| matches!(value, TextLineHeight::Absolute(_))),
        ),
        CanvasCommand::Image { filter, radius, .. } => {
            format!("image:{filter:?}:r={}", canvas_radius_semantic_key(radius))
        }
        CanvasCommand::Svg { memory, color, .. } => format!("svg:memory={memory};color={color:?}"),
        CanvasCommand::Path {
            segments, paint, ..
        } => format!(
            "path:{};p={}",
            canvas_path_semantic_key(segments),
            canvas_paint_semantic_key(paint)
        ),
        CanvasCommand::Group {
            transform,
            commands,
            ..
        } => format!(
            "group:{:?}:children={}",
            [
                transform.x.is_some(),
                transform.y.is_some(),
                transform.rotate.is_some(),
                transform.scale.is_some(),
                transform.scale_x.is_some(),
                transform.scale_y.is_some(),
                transform.clip.is_some(),
            ],
            commands.len()
        ),
        CanvasCommand::If { commands, .. } => format!("if:children={}", commands.len()),
        CanvasCommand::For { item, commands, .. } => {
            format!("for:{item}:children={}", commands.len())
        }
    }
}

pub(crate) fn canvas_command_semantic_keys(commands: &[CanvasCommand]) -> Vec<String> {
    fn collect(commands: &[CanvasCommand], keys: &mut Vec<String>) {
        for command in commands {
            keys.push(canvas_command_semantic_key(command));
            match command {
                CanvasCommand::Group { commands, .. }
                | CanvasCommand::If { commands, .. }
                | CanvasCommand::For { commands, .. } => collect(commands, keys),
                _ => {}
            }
        }
    }
    let mut keys = Vec::new();
    collect(commands, &mut keys);
    keys
}

pub(crate) fn canvas_event_semantic_key(event: &CanvasEvent) -> String {
    let action = match &event.action {
        None => "none".into(),
        Some(CanvasEventAction::Route(_)) => "route".into(),
        Some(CanvasEventAction::Redraw { after_ms }) => format!("redraw:{after_ms:?}"),
    };
    format!(
        "source={:?};bindings={:?};updates={:?};action={action};capture={};payload={}",
        event.source,
        event.bindings,
        event
            .updates
            .iter()
            .map(|update| &update.name)
            .collect::<Vec<_>>(),
        event.capture,
        event.route_payload,
    )
}

pub(crate) fn canvas_routes<'a>(
    options: &'a CanvasOptions,
    events: &'a [CanvasEvent],
) -> Vec<&'a Route> {
    let mut routes = [
        &options.press,
        &options.release,
        &options.right_press,
        &options.right_release,
        &options.middle_press,
        &options.middle_release,
        &options.enter,
        &options.move_route,
        &options.scroll,
        &options.exit,
    ]
    .into_iter()
    .filter_map(Option::as_ref)
    .collect::<Vec<_>>();
    routes.extend(events.iter().filter_map(|event| match &event.action {
        Some(CanvasEventAction::Route(route)) => Some(route),
        Some(CanvasEventAction::Redraw { .. }) | None => None,
    }));
    routes
}

pub(crate) fn route_expression_roots(route: &Route) -> Vec<&Expr> {
    route
        .args
        .iter()
        .filter_map(|argument| match argument {
            RouteArg::Expr(expression) => Some(expression),
            RouteArg::Payload => None,
        })
        .collect()
}

pub(crate) fn canvas_radius_expression_roots(radius: &CanvasRadius) -> Vec<&Expr> {
    [
        &radius.all,
        &radius.top_left,
        &radius.top_right,
        &radius.bottom_right,
        &radius.bottom_left,
    ]
    .into_iter()
    .filter_map(Option::as_ref)
    .collect()
}

pub(crate) fn canvas_background_expression_roots(background: &BackgroundValue) -> Vec<&Expr> {
    match background {
        BackgroundValue::Color(_) => Vec::new(),
        BackgroundValue::Linear { angle, stops } => std::iter::once(angle)
            .chain(stops.iter().map(|stop| &stop.offset))
            .collect(),
    }
}

pub(crate) fn canvas_stroke_expression_roots(stroke: &CanvasStroke) -> Vec<&Expr> {
    let mut expressions = canvas_background_expression_roots(&stroke.style);
    expressions.push(&stroke.width);
    expressions.extend(&stroke.dash);
    expressions.push(&stroke.dash_offset);
    expressions
}

pub(crate) fn canvas_paint_expression_roots(paint: &CanvasPaint) -> Vec<&Expr> {
    let mut expressions = paint
        .fill
        .as_ref()
        .map_or_else(Vec::new, canvas_background_expression_roots);
    if let Some(stroke) = &paint.stroke {
        expressions.extend(canvas_stroke_expression_roots(stroke));
    }
    expressions
}

pub(crate) fn canvas_path_expression_roots(segments: &[CanvasPathSegment]) -> Vec<&Expr> {
    let mut expressions = Vec::new();
    for segment in segments {
        match segment {
            CanvasPathSegment::Move(x, y) | CanvasPathSegment::Line(x, y) => {
                expressions.extend([x, y]);
            }
            CanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => expressions.extend([x, y, radius, start, end]),
            CanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => expressions.extend([ax, ay, bx, by, radius]),
            CanvasPathSegment::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                rotation,
                start,
                end,
            } => expressions.extend([x, y, radius_x, radius_y, rotation, start, end]),
            CanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => expressions.extend([control_ax, control_ay, control_bx, control_by, x, y]),
            CanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => expressions.extend([control_x, control_y, x, y]),
            CanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            } => expressions.extend([x, y, width, height]),
            CanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                radius,
            } => {
                expressions.extend([x, y, width, height]);
                expressions.extend(canvas_radius_expression_roots(radius));
            }
            CanvasPathSegment::Circle { x, y, radius } => {
                expressions.extend([x, y, radius]);
            }
            CanvasPathSegment::Close => {}
        }
    }
    expressions
}

pub(crate) fn canvas_transform_expression_roots(transform: &CanvasTransform) -> Vec<&Expr> {
    let mut expressions = [
        &transform.x,
        &transform.y,
        &transform.rotate,
        &transform.scale,
        &transform.scale_x,
        &transform.scale_y,
    ]
    .into_iter()
    .filter_map(Option::as_ref)
    .collect::<Vec<_>>();
    if let Some(clip) = &transform.clip {
        expressions.extend(clip);
    }
    expressions
}

pub(crate) fn canvas_command_direct_expression_roots(command: &CanvasCommand) -> Vec<&Expr> {
    let mut expressions = Vec::new();
    match command {
        CanvasCommand::Rectangle {
            x,
            y,
            width,
            height,
            radius,
            paint,
            ..
        } => {
            expressions.extend([x, y, width, height]);
            expressions.extend(canvas_radius_expression_roots(radius));
            expressions.extend(canvas_paint_expression_roots(paint));
        }
        CanvasCommand::Circle {
            x,
            y,
            radius,
            paint,
            ..
        } => {
            expressions.extend([x, y, radius]);
            expressions.extend(canvas_paint_expression_roots(paint));
        }
        CanvasCommand::Line {
            x1,
            y1,
            x2,
            y2,
            stroke,
            ..
        } => {
            expressions.extend([x1, y1, x2, y2]);
            expressions.extend(canvas_stroke_expression_roots(stroke));
        }
        CanvasCommand::Text {
            value,
            x,
            y,
            max_width,
            size,
            line_height,
            ..
        } => {
            expressions.extend([value, x, y]);
            expressions.extend(max_width);
            expressions.extend(size);
            if let Some(line_height) = line_height {
                expressions.push(match line_height {
                    TextLineHeight::Relative(value) | TextLineHeight::Absolute(value) => value,
                });
            }
        }
        CanvasCommand::Image {
            source,
            x,
            y,
            width,
            height,
            rotation,
            opacity,
            snap,
            radius,
            ..
        } => {
            expressions.extend([source, x, y, width, height, rotation, opacity, snap]);
            expressions.extend(canvas_radius_expression_roots(radius));
        }
        CanvasCommand::Svg {
            source,
            x,
            y,
            width,
            height,
            rotation,
            opacity,
            ..
        } => expressions.extend([source, x, y, width, height, rotation, opacity]),
        CanvasCommand::Path {
            segments, paint, ..
        } => {
            expressions.extend(canvas_path_expression_roots(segments));
            expressions.extend(canvas_paint_expression_roots(paint));
        }
        CanvasCommand::Group { transform, .. } => {
            expressions.extend(canvas_transform_expression_roots(transform));
        }
        CanvasCommand::If { condition, .. } => expressions.push(condition),
        CanvasCommand::For { items, .. } => expressions.push(items),
    }
    expressions
}

pub(crate) fn canvas_expression_roots<'a>(
    options: &'a CanvasOptions,
    locals: &'a [State],
    commands: &'a [CanvasCommand],
    events: &'a [CanvasEvent],
) -> Vec<&'a Expr> {
    fn collect_commands<'a>(commands: &'a [CanvasCommand], output: &mut Vec<&'a Expr>) {
        for command in commands {
            output.extend(canvas_command_direct_expression_roots(command));
            match command {
                CanvasCommand::Group { commands, .. }
                | CanvasCommand::If { commands, .. }
                | CanvasCommand::For { commands, .. } => collect_commands(commands, output),
                _ => {}
            }
        }
    }

    let mut expressions = Vec::new();
    for length in [&options.width, &options.height].into_iter().flatten() {
        if let LengthValue::Fixed(expression) = length {
            expressions.push(expression);
        }
    }
    expressions.extend(&options.cache);
    expressions.extend(&options.capture);
    for route in canvas_routes(options, events) {
        expressions.extend(route_expression_roots(route));
    }
    expressions.extend(locals.iter().map(|local| &local.initial));
    expressions.extend(&options.interaction_expr);
    expressions.extend(&options.interaction_outside);
    collect_commands(commands, &mut expressions);
    for event in events {
        expressions.extend(event.updates.iter().map(|update| &update.value));
    }
    expressions
}

#[derive(Clone, Debug, Default)]
pub struct SensorOptions {
    pub show: Option<Route>,
    pub resize: Option<Route>,
    pub hide: Option<Route>,
    pub key: Option<Expr>,
    pub anticipate: Option<Expr>,
    pub delay_ms: Option<Expr>,
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
