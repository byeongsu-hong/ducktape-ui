use super::*;

#[derive(Clone, Debug, Default)]
pub struct FloatStyleOptions {
    pub shadow_color: Option<String>,
    pub shadow_x: Option<Expr>,
    pub shadow_y: Option<Expr>,
    pub shadow_blur: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
}

pub(crate) fn float_expression_roots<'a>(
    scale: &'a Expr,
    x: &'a Expr,
    y: &'a Expr,
    style: &'a FloatStyleOptions,
) -> Vec<&'a Expr> {
    let mut roots = vec![scale, x, y];
    roots.extend(
        [
            &style.shadow_x,
            &style.shadow_y,
            &style.shadow_blur,
            &style.radius,
            &style.radius_top_left,
            &style.radius_top_right,
            &style.radius_bottom_right,
            &style.radius_bottom_left,
        ]
        .into_iter()
        .flatten(),
    );
    roots
}

pub(crate) fn float_semantic_key(style: &FloatStyleOptions) -> String {
    format!(
        "float|shadow={:?}|fields={:?}",
        style.shadow_color,
        [
            style.shadow_x.is_some(),
            style.shadow_y.is_some(),
            style.shadow_blur.is_some(),
            style.radius.is_some(),
            style.radius_top_left.is_some(),
            style.radius_top_right.is_some(),
            style.radius_bottom_right.is_some(),
            style.radius_bottom_left.is_some(),
        ]
    )
}

pub(crate) fn pin_expression_roots<'a>(
    width: &'a Option<LengthValue>,
    height: &'a Option<LengthValue>,
    x: &'a Expr,
    y: &'a Expr,
) -> Vec<&'a Expr> {
    let mut roots = vec![x, y];
    for length in [width, height].into_iter().flatten() {
        if let LengthValue::Fixed(expression) = length {
            roots.push(expression);
        }
    }
    roots
}

pub(crate) fn pin_semantic_key(
    width: &Option<LengthValue>,
    height: &Option<LengthValue>,
) -> String {
    format!(
        "pin|width={}|height={}",
        length_semantic_key(width),
        length_semantic_key(height)
    )
}

pub(crate) fn responsive_semantic_key(
    content: &ResponsiveContent,
    width: &Option<LengthValue>,
    height: &Option<LengthValue>,
) -> String {
    let kind = match content {
        ResponsiveContent::Breakpoint { .. } => "breakpoint".into(),
        ResponsiveContent::Size { width, height, .. } => format!("size:{width}:{height}"),
    };
    format!(
        "responsive|kind={kind}|width={}|height={}",
        length_semantic_key(width),
        length_semantic_key(height)
    )
}

pub(crate) fn responsive_expression_count(
    content: &ResponsiveContent,
    width: &Option<LengthValue>,
    height: &Option<LengthValue>,
) -> u32 {
    let breakpoint = u32::from(matches!(content, ResponsiveContent::Breakpoint { .. }));
    let dimensions = [width, height]
        .into_iter()
        .filter(|length| matches!(length, Some(LengthValue::Fixed(_))))
        .count() as u32;
    breakpoint + dimensions
}

fn length_semantic_key(length: &Option<LengthValue>) -> String {
    match length {
        None => "none".into(),
        Some(LengthValue::Fill) => "fill".into(),
        Some(LengthValue::FillPortion(portion)) => format!("fill-portion:{portion}"),
        Some(LengthValue::Shrink) => "shrink".into(),
        Some(LengthValue::Fixed(_)) => "fixed".into(),
    }
}

#[derive(Clone, Debug, Default)]
pub struct MarkdownOptions {
    pub text_size: Option<Expr>,
    pub h1_size: Option<Expr>,
    pub h2_size: Option<Expr>,
    pub h3_size: Option<Expr>,
    pub h4_size: Option<Expr>,
    pub h5_size: Option<Expr>,
    pub h6_size: Option<Expr>,
    pub code_size: Option<Expr>,
    pub spacing: Option<Expr>,
    pub viewer: Option<ExternCall>,
    pub style: MarkdownStyleOptions,
}

#[derive(Clone, Debug)]
pub struct ExternCall {
    pub function: String,
    pub args: Vec<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct MarkdownStyleOptions {
    pub font: Option<FontPreset>,
    pub inline_code_background: Option<BackgroundValue>,
    pub inline_code_color: Option<String>,
    pub inline_code_font: Option<FontPreset>,
    pub code_block_font: Option<FontPreset>,
    pub link_color: Option<String>,
    pub inline_code_padding: PaddingOptions,
    pub inline_code_border_color: Option<String>,
    pub inline_code_border_width: Option<Expr>,
    pub inline_code_radius: Option<Expr>,
    pub inline_code_radius_top_left: Option<Expr>,
    pub inline_code_radius_top_right: Option<Expr>,
    pub inline_code_radius_bottom_right: Option<Expr>,
    pub inline_code_radius_bottom_left: Option<Expr>,
    pub span: Option<Span>,
}

pub(crate) fn markdown_style_expression_roots(style: &MarkdownStyleOptions) -> Vec<&Expr> {
    let mut roots = Vec::new();
    if let Some(BackgroundValue::Linear { angle, stops }) = &style.inline_code_background {
        roots.push(angle);
        roots.extend(stops.iter().map(|stop| &stop.offset));
    }
    roots.extend(
        [
            &style.inline_code_padding.all,
            &style.inline_code_padding.x,
            &style.inline_code_padding.y,
            &style.inline_code_padding.top,
            &style.inline_code_padding.right,
            &style.inline_code_padding.bottom,
            &style.inline_code_padding.left,
            &style.inline_code_border_width,
            &style.inline_code_radius,
            &style.inline_code_radius_top_left,
            &style.inline_code_radius_top_right,
            &style.inline_code_radius_bottom_right,
            &style.inline_code_radius_bottom_left,
        ]
        .into_iter()
        .flatten(),
    );
    roots
}

pub(crate) fn markdown_expression_roots(options: &MarkdownOptions) -> Vec<&Expr> {
    let mut roots = Vec::new();
    roots.extend(
        [
            &options.text_size,
            &options.h1_size,
            &options.h2_size,
            &options.h3_size,
            &options.h4_size,
            &options.h5_size,
            &options.h6_size,
            &options.code_size,
            &options.spacing,
        ]
        .into_iter()
        .flatten(),
    );
    roots.extend(markdown_style_expression_roots(&options.style));
    if let Some(viewer) = &options.viewer {
        roots.extend(&viewer.args);
    }
    roots
}

pub(crate) fn markdown_semantic_key(options: &MarkdownOptions) -> String {
    let style = &options.style;
    let background = style
        .inline_code_background
        .as_ref()
        .map(|background| match background {
            BackgroundValue::Color(color) => format!("color:{color}"),
            BackgroundValue::Linear { stops, .. } => format!(
                "linear:{}",
                stops
                    .iter()
                    .map(|stop| stop.color.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        });
    format!(
        "markdown|metrics={:?}|viewer={:?}|fonts={:?}|background={background:?}|colors={:?}|style-metrics={:?}",
        [
            options.text_size.is_some(),
            options.h1_size.is_some(),
            options.h2_size.is_some(),
            options.h3_size.is_some(),
            options.h4_size.is_some(),
            options.h5_size.is_some(),
            options.h6_size.is_some(),
            options.code_size.is_some(),
            options.spacing.is_some(),
        ],
        options
            .viewer
            .as_ref()
            .map(|viewer| viewer.function.as_str()),
        [
            style.font.as_ref(),
            style.inline_code_font.as_ref(),
            style.code_block_font.as_ref(),
        ],
        [
            style.inline_code_color.as_deref(),
            style.link_color.as_deref(),
            style.inline_code_border_color.as_deref(),
        ],
        [
            style.inline_code_padding.all.is_some(),
            style.inline_code_padding.x.is_some(),
            style.inline_code_padding.y.is_some(),
            style.inline_code_padding.top.is_some(),
            style.inline_code_padding.right.is_some(),
            style.inline_code_padding.bottom.is_some(),
            style.inline_code_padding.left.is_some(),
            style.inline_code_border_width.is_some(),
            style.inline_code_radius.is_some(),
            style.inline_code_radius_top_left.is_some(),
            style.inline_code_radius_top_right.is_some(),
            style.inline_code_radius_bottom_right.is_some(),
            style.inline_code_radius_bottom_left.is_some(),
        ],
    )
}

#[derive(Clone, Debug, Default)]
pub struct TextEditorOptions {
    pub placeholder: Option<String>,
    pub width: Option<Expr>,
    pub height: Option<LengthValue>,
    pub min_height: Option<Expr>,
    pub max_height: Option<Expr>,
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub padding: Option<Expr>,
    pub wrapping: Option<TextWrapping>,
    pub font: Option<FontPreset>,
    pub highlight: Option<String>,
    pub highlight_theme: Option<HighlightTheme>,
    pub highlighter: Option<ExternCall>,
    pub key_binding: Option<ExternCall>,
    pub key_binding_route: Option<Route>,
    pub action: Option<ExternCall>,
    pub custom_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighlightTheme {
    SolarizedDark,
    Base16Mocha,
    Base16Ocean,
    Base16Eighties,
    InspiredGithub,
}

pub(crate) fn text_editor_expression_roots<'a>(
    disabled: &'a Option<Expr>,
    options: &'a TextEditorOptions,
) -> Vec<&'a Expr> {
    let mut roots = Vec::new();
    roots.extend(disabled);
    roots.extend(options.width.as_ref());
    push_input_length_root(&mut roots, &options.height);
    roots.extend(
        [&options.min_height, &options.max_height, &options.size]
            .into_iter()
            .flatten(),
    );
    if let Some(line_height) = &options.line_height {
        roots.push(match line_height {
            TextLineHeight::Relative(value) | TextLineHeight::Absolute(value) => value,
        });
    }
    roots.extend(options.padding.as_ref());
    for call in [
        &options.highlighter,
        &options.key_binding,
        &options.action,
        &options.custom_style,
    ]
    .into_iter()
    .flatten()
    {
        roots.extend(&call.args);
    }
    for style in [
        &options.style.active,
        &options.style.hovered,
        &options.style.focused,
        &options.style.focused_hovered,
        &options.style.disabled,
    ]
    .into_iter()
    .flatten()
    {
        push_input_surface_roots(&mut roots, &style.options);
    }
    roots
}

pub(crate) fn text_editor_semantic_key(
    binding: &str,
    disabled: &Option<Expr>,
    options: &TextEditorOptions,
) -> String {
    fn call_key(call: &Option<ExternCall>) -> String {
        call.as_ref().map_or_else(
            || "none".into(),
            |call| format!("{}:{}", call.function, call.args.len()),
        )
    }

    fn route_key(route: &Option<Route>) -> String {
        route.as_ref().map_or_else(
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

    let line_height = match &options.line_height {
        None => "none",
        Some(TextLineHeight::Relative(_)) => "relative",
        Some(TextLineHeight::Absolute(_)) => "absolute",
    };
    let statuses = [
        &options.style.active,
        &options.style.hovered,
        &options.style.focused,
        &options.style.focused_hovered,
        &options.style.disabled,
    ]
    .into_iter()
    .map(|style| {
        style
            .as_ref()
            .map(input_surface_semantic_key)
            .unwrap_or_else(|| "none".into())
    })
    .collect::<Vec<_>>()
    .join(";");
    format!(
        "editor|binding={binding}|disabled={}|placeholder={:?}|width={}|height={}|metrics={:?}|line-height={line_height}|wrapping={:?}|font={:?}|highlight={:?}:{:?}|highlighter={}|key-binding={}|route={}|action={}|style={}|statuses={statuses}",
        disabled.is_some(),
        options.placeholder,
        options.width.is_some(),
        input_length_semantic_key(&options.height),
        [
            options.min_height.is_some(),
            options.max_height.is_some(),
            options.size.is_some(),
            options.padding.is_some(),
        ],
        options.wrapping,
        options.font,
        options.highlight,
        options.highlight_theme,
        call_key(&options.highlighter),
        call_key(&options.key_binding),
        route_key(&options.key_binding_route),
        call_key(&options.action),
        call_key(&options.custom_style),
    )
}

#[derive(Clone, Debug, Default)]
pub struct TableOptions {
    pub width: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub padding_x: Option<Expr>,
    pub padding_y: Option<Expr>,
    pub separator: Option<Expr>,
    pub separator_x: Option<Expr>,
    pub separator_y: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct TableColumn {
    pub width: Option<LengthValue>,
    pub align_x: Option<InputAlignment>,
    pub align_y: Option<VerticalAlignment>,
    pub header: ViewNode,
    pub cell: ViewNode,
    pub span: Span,
}

pub(crate) fn table_semantic_key(options: &TableOptions, columns: &[TableColumn]) -> String {
    fn length_key(length: &Option<LengthValue>) -> String {
        match length {
            None => "none".into(),
            Some(LengthValue::Fill) => "fill".into(),
            Some(LengthValue::FillPortion(portion)) => format!("fill-portion:{portion}"),
            Some(LengthValue::Shrink) => "shrink".into(),
            Some(LengthValue::Fixed(_)) => "fixed".into(),
        }
    }

    let columns = columns
        .iter()
        .map(|column| {
            format!(
                "{}:{:?}:{:?}",
                length_key(&column.width),
                column.align_x,
                column.align_y
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "table|width={}|metrics={:?}|columns={columns}",
        length_key(&options.width),
        [
            options.padding.is_some(),
            options.padding_x.is_some(),
            options.padding_y.is_some(),
            options.separator.is_some(),
            options.separator_x.is_some(),
            options.separator_y.is_some(),
        ],
    )
}

#[derive(Clone, Debug)]
pub enum ThemePreset {
    Default,
    App,
    BuiltIn(String),
    Factory(ExternCall),
}

#[derive(Clone, Debug)]
pub enum ResponsiveContent {
    Breakpoint {
        breakpoint: Expr,
        narrow: Box<ViewNode>,
        wide: Box<ViewNode>,
    },
    Size {
        width: String,
        height: String,
        content: Box<ViewNode>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct AccessibilityOptions {
    pub label: Option<Expr>,
    pub description: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct InputOptions {
    pub accessibility: AccessibilityOptions,
    pub secure: Option<Expr>,
    pub change: Option<Route>,
    pub submit: Option<Route>,
    pub paste: Option<Route>,
    pub width: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub align: Option<InputAlignment>,
    pub font: Option<FontPreset>,
    pub icon: Option<TextInputIcon>,
    pub custom_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
}

fn push_input_length_root<'a>(roots: &mut Vec<&'a Expr>, length: &'a Option<LengthValue>) {
    if let Some(LengthValue::Fixed(expression)) = length {
        roots.push(expression);
    }
}

fn push_input_surface_roots<'a>(roots: &mut Vec<&'a Expr>, surface: &'a ContainerStyleOptions) {
    if let Some(BackgroundValue::Linear { angle, stops }) = &surface.background {
        roots.push(angle);
        roots.extend(stops.iter().map(|stop| &stop.offset));
    }
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

pub(crate) fn input_expression_roots<'a>(
    disabled: &'a Option<Expr>,
    options: &'a InputOptions,
) -> Vec<&'a Expr> {
    let mut roots = Vec::new();
    roots.extend(disabled);
    roots.extend(
        [
            &options.accessibility.label,
            &options.accessibility.description,
            &options.secure,
        ]
        .into_iter()
        .flatten(),
    );
    push_input_length_root(&mut roots, &options.width);
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
    for style in [
        &options.style.active,
        &options.style.hovered,
        &options.style.focused,
        &options.style.focused_hovered,
        &options.style.disabled,
    ]
    .into_iter()
    .flatten()
    {
        push_input_surface_roots(&mut roots, &style.options);
    }
    roots
}

fn input_length_semantic_key(length: &Option<LengthValue>) -> String {
    match length {
        None => "none".into(),
        Some(LengthValue::Fill) => "fill".into(),
        Some(LengthValue::FillPortion(portion)) => format!("fill:{portion}"),
        Some(LengthValue::Shrink) => "shrink".into(),
        Some(LengthValue::Fixed(_)) => "fixed".into(),
    }
}

fn input_surface_semantic_key(style: &TextInputStatusStyle) -> String {
    let background = match &style.options.background {
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
    format!(
        "bg={background}|colors={:?}|fields={:?}",
        [
            style.options.text_color.as_deref(),
            style.options.border_color.as_deref(),
            style.options.shadow_color.as_deref(),
            style.icon_color.as_deref(),
            style.placeholder_color.as_deref(),
            style.value_color.as_deref(),
            style.selection_color.as_deref(),
        ],
        [
            style.options.border_width.is_some(),
            style.options.radius.is_some(),
            style.options.radius_top_left.is_some(),
            style.options.radius_top_right.is_some(),
            style.options.radius_bottom_right.is_some(),
            style.options.radius_bottom_left.is_some(),
            style.options.shadow_x.is_some(),
            style.options.shadow_y.is_some(),
            style.options.shadow_blur.is_some(),
            style.options.pixel_snap.is_some(),
        ],
    )
}

pub(crate) fn input_semantic_key(
    label: &str,
    binding: &str,
    hint: &str,
    disabled: &Option<Expr>,
    options: &InputOptions,
) -> String {
    let custom = options.custom_style.as_ref().map_or_else(
        || "none".into(),
        |style| format!("{}:{}", style.function, style.args.len()),
    );
    let icon = options.icon.as_ref().map_or_else(
        || "none".into(),
        |icon| {
            format!(
                "{}:{:?}:{:?}:{}:{}",
                icon.code_point,
                icon.font,
                icon.side,
                icon.size.is_some(),
                icon.spacing.is_some()
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
    .map(|style| {
        style
            .as_ref()
            .map(input_surface_semantic_key)
            .unwrap_or_else(|| "none".into())
    })
    .collect::<Vec<_>>()
    .join(";");
    let routes = [&options.change, &options.submit, &options.paste]
        .into_iter()
        .map(|route| {
            route.as_ref().map_or_else(
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
        })
        .collect::<Vec<_>>()
        .join(":");
    format!(
        "input|label={label:?}|binding={binding}|hint={hint:?}|disabled={}|a11y={}:{}|secure={}|routes={routes}|width={}|metrics={:?}|align={:?}|font={:?}|icon={icon}|custom={custom}|statuses={statuses}",
        disabled.is_some(),
        options.accessibility.label.is_some(),
        options.accessibility.description.is_some(),
        options.secure.is_some(),
        input_length_semantic_key(&options.width),
        [
            options.padding.is_some(),
            options.text_size.is_some(),
            options.line_height.is_some(),
        ],
        options.align,
        options.font,
    )
}

#[derive(Clone, Debug, Default)]
pub struct TextOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub font: Option<FontPreset>,
    pub align_x: Option<TextAlignment>,
    pub align_y: Option<VerticalAlignment>,
    pub shaping: Option<TextShaping>,
    pub wrapping: Option<TextWrapping>,
    pub tracking: Option<f64>,
    pub custom_style: Option<ExternCall>,
}

#[derive(Clone, Debug)]
pub enum TextLineHeight {
    Relative(Expr),
    Absolute(Expr),
}

#[derive(Clone, Debug)]
pub struct RichSpan {
    pub value: Expr,
    pub options: RichSpanOptions,
    pub styles: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct RichSpanOptions {
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub font: Option<FontPreset>,
    pub color: Option<String>,
    pub link: Option<Expr>,
    pub background: Option<BackgroundValue>,
    pub border: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub padding: PaddingOptions,
    pub underline: Option<Expr>,
    pub strikethrough: Option<Expr>,
}

fn push_text_length_root<'a>(roots: &mut Vec<&'a Expr>, length: &'a Option<LengthValue>) {
    if let Some(LengthValue::Fixed(expression)) = length {
        roots.push(expression);
    }
}

fn push_text_line_height_root<'a>(
    roots: &mut Vec<&'a Expr>,
    line_height: &'a Option<TextLineHeight>,
) {
    if let Some(line_height) = line_height {
        roots.push(match line_height {
            TextLineHeight::Relative(expression) | TextLineHeight::Absolute(expression) => {
                expression
            }
        });
    }
}

fn push_text_background_roots<'a>(
    roots: &mut Vec<&'a Expr>,
    background: &'a Option<BackgroundValue>,
) {
    if let Some(BackgroundValue::Linear { angle, stops }) = background {
        roots.push(angle);
        roots.extend(stops.iter().map(|stop| &stop.offset));
    }
}

fn push_text_option_roots<'a>(roots: &mut Vec<&'a Expr>, options: &'a TextOptions) {
    push_text_length_root(roots, &options.width);
    push_text_length_root(roots, &options.height);
    roots.extend(options.size.as_ref());
    push_text_line_height_root(roots, &options.line_height);
    if let Some(style) = &options.custom_style {
        roots.extend(&style.args);
    }
}

pub(crate) fn text_expression_roots<'a>(
    value: &'a Expr,
    options: &'a TextOptions,
) -> Vec<&'a Expr> {
    let mut roots = vec![value];
    push_text_option_roots(&mut roots, options);
    roots
}

pub(crate) fn rich_text_expression_roots<'a>(
    options: &'a TextOptions,
    spans: &'a [RichSpan],
) -> Vec<&'a Expr> {
    let mut roots = Vec::new();
    push_text_option_roots(&mut roots, options);
    for span in spans {
        roots.push(&span.value);
        roots.extend(span.options.size.as_ref());
        push_text_line_height_root(&mut roots, &span.options.line_height);
        roots.extend(span.options.link.as_ref());
        push_text_background_roots(&mut roots, &span.options.background);
        roots.extend(
            [
                &span.options.border_width,
                &span.options.radius,
                &span.options.radius_top_left,
                &span.options.radius_top_right,
                &span.options.radius_bottom_right,
                &span.options.radius_bottom_left,
                &span.options.padding.all,
                &span.options.padding.x,
                &span.options.padding.y,
                &span.options.padding.top,
                &span.options.padding.right,
                &span.options.padding.bottom,
                &span.options.padding.left,
                &span.options.underline,
                &span.options.strikethrough,
            ]
            .into_iter()
            .flatten(),
        );
    }
    roots
}

fn text_length_semantic_key(length: &Option<LengthValue>) -> String {
    match length {
        None => "none".into(),
        Some(LengthValue::Fill) => "fill".into(),
        Some(LengthValue::FillPortion(portion)) => format!("fill:{portion}"),
        Some(LengthValue::Shrink) => "shrink".into(),
        Some(LengthValue::Fixed(_)) => "fixed".into(),
    }
}

fn text_font_semantic_key(font: &Option<FontPreset>) -> String {
    match font {
        None => "none".into(),
        Some(FontPreset::Default) => "default".into(),
        Some(FontPreset::Monospace) => "monospace".into(),
        Some(FontPreset::Named(name)) => format!("named:{name}"),
    }
}

fn text_line_height_semantic_key(line_height: &Option<TextLineHeight>) -> &'static str {
    match line_height {
        None => "none",
        Some(TextLineHeight::Relative(_)) => "relative",
        Some(TextLineHeight::Absolute(_)) => "absolute",
    }
}

fn text_options_semantic_key(options: &TextOptions) -> String {
    let custom = options.custom_style.as_ref().map_or_else(
        || "none".into(),
        |style| format!("{}:{}", style.function, style.args.len()),
    );
    format!(
        "bounds={}:{}|size={}|line={}|font={}|align={:?}:{:?}|shape={:?}|wrap={:?}|tracking={:?}|custom={custom}",
        text_length_semantic_key(&options.width),
        text_length_semantic_key(&options.height),
        options.size.is_some(),
        text_line_height_semantic_key(&options.line_height),
        text_font_semantic_key(&options.font),
        options.align_x,
        options.align_y,
        options.shaping,
        options.wrapping,
        options.tracking.map(f64::to_bits),
    )
}

pub(crate) fn text_semantic_key(options: &TextOptions) -> String {
    format!("text|{}", text_options_semantic_key(options))
}

pub(crate) fn rich_text_semantic_key(
    options: &TextOptions,
    color: &Option<String>,
    spans: &[RichSpan],
    route: &Option<Route>,
) -> String {
    fn background(background: &Option<BackgroundValue>) -> String {
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

    let spans = spans
        .iter()
        .map(|span| {
            format!(
                "size={}|line={}|font={}|color={:?}|link={}|bg={}|border={:?}|metrics={:?}",
                span.options.size.is_some(),
                text_line_height_semantic_key(&span.options.line_height),
                text_font_semantic_key(&span.options.font),
                span.options.color,
                span.options.link.is_some(),
                background(&span.options.background),
                span.options.border,
                [
                    span.options.border_width.is_some(),
                    span.options.radius.is_some(),
                    span.options.radius_top_left.is_some(),
                    span.options.radius_top_right.is_some(),
                    span.options.radius_bottom_right.is_some(),
                    span.options.radius_bottom_left.is_some(),
                    span.options.padding.all.is_some(),
                    span.options.padding.x.is_some(),
                    span.options.padding.y.is_some(),
                    span.options.padding.top.is_some(),
                    span.options.padding.right.is_some(),
                    span.options.padding.bottom.is_some(),
                    span.options.padding.left.is_some(),
                    span.options.underline.is_some(),
                    span.options.strikethrough.is_some(),
                ],
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let route = route.as_ref().map_or_else(
        || "none".into(),
        |route| {
            let args = route
                .args
                .iter()
                .map(|argument| match argument {
                    RouteArg::Expr(_) => 'e',
                    RouteArg::Payload => 'p',
                })
                .collect::<String>();
            format!("{}:{args}", route.handler)
        },
    );
    format!(
        "rich-text|{}|color={color:?}|spans={spans}|route={route}",
        text_options_semantic_key(options)
    )
}

#[derive(Clone, Debug, Default)]
pub struct ButtonOptions {
    pub accessibility: AccessibilityOptions,
    pub checked: Option<Expr>,
    pub expanded: Option<Expr>,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub clip: Option<Expr>,
    pub style: Box<ButtonStyleSet>,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonStyleSet {
    pub preset: ButtonStylePreset,
    pub custom: Option<ExternCall>,
    pub active: Option<ButtonStatusStyle>,
    pub hovered: Option<ButtonStatusStyle>,
    pub pressed: Option<ButtonStatusStyle>,
    pub disabled: Option<ButtonStatusStyle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStylePreset {
    #[default]
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Text,
    Background,
    Subtle,
}

#[derive(Clone, Debug)]
pub struct ButtonStatusStyle {
    pub options: ContainerStyleOptions,
    pub span: Span,
}

pub(crate) fn button_expression_roots<'a>(
    disabled: &'a Option<Expr>,
    options: &'a ButtonOptions,
) -> Vec<&'a Expr> {
    let mut roots = Vec::new();
    roots.extend(disabled);
    roots.extend(options.checked.as_ref());
    roots.extend(options.expanded.as_ref());
    roots.extend(options.accessibility.label.as_ref());
    roots.extend(options.accessibility.description.as_ref());
    push_input_length_root(&mut roots, &options.width);
    push_input_length_root(&mut roots, &options.height);
    roots.extend(options.padding.as_ref());
    roots.extend(options.clip.as_ref());
    if let Some(custom) = &options.style.custom {
        roots.extend(&custom.args);
    }
    for status in [
        &options.style.active,
        &options.style.hovered,
        &options.style.pressed,
        &options.style.disabled,
    ]
    .into_iter()
    .flatten()
    {
        push_input_surface_roots(&mut roots, &status.options);
    }
    roots
}

pub(crate) fn button_semantic_key(
    label: &Option<String>,
    content: &Option<Box<ViewNode>>,
    disabled: &Option<Expr>,
    options: &ButtonOptions,
    route: &Route,
) -> String {
    fn surface(options: &ContainerStyleOptions) -> String {
        let background = options.background.as_ref().map_or_else(
            || "none".into(),
            |background| match background {
                BackgroundValue::Color(color) => format!("color:{color}"),
                BackgroundValue::Linear { stops, .. } => format!(
                    "linear:{}",
                    stops
                        .iter()
                        .map(|stop| stop.color.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            },
        );
        format!(
            "bg={background}|text={:?}|border={:?}|shadow={:?}|fields={:?}",
            options.text_color,
            options.border_color,
            options.shadow_color,
            [
                options.border_width.is_some(),
                options.radius.is_some(),
                options.radius_top_left.is_some(),
                options.radius_top_right.is_some(),
                options.radius_bottom_right.is_some(),
                options.radius_bottom_left.is_some(),
                options.shadow_color.is_some(),
                options.shadow_x.is_some(),
                options.shadow_y.is_some(),
                options.shadow_blur.is_some(),
                options.pixel_snap.is_some(),
            ]
        )
    }
    let custom = options.style.custom.as_ref().map_or_else(
        || "none".into(),
        |call| format!("{}:{}", call.function, call.args.len()),
    );
    let statuses = [
        &options.style.active,
        &options.style.hovered,
        &options.style.pressed,
        &options.style.disabled,
    ]
    .map(|status| {
        status
            .as_ref()
            .map_or_else(|| "none".into(), |s| surface(&s.options))
    })
    .join(";");
    let route_args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Expr(_) => 'e',
            RouteArg::Payload => 'p',
        })
        .collect::<String>();
    format!(
        "button|label={label:?}|child={}|disabled={}|checked={}|expanded={}|a11y={}:{}|bounds={}:{}|padding={}|clip={}|preset={:?}|custom={custom}|statuses={statuses}|route={}:{}",
        content.is_some(),
        disabled.is_some(),
        options.checked.is_some(),
        options.expanded.is_some(),
        options.accessibility.label.is_some(),
        options.accessibility.description.is_some(),
        text_length_semantic_key(&options.width),
        text_length_semantic_key(&options.height),
        options.padding.is_some(),
        options.clip.is_some(),
        options.style.preset,
        route.handler,
        route_args,
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontPreset {
    Default,
    Monospace,
    Named(String),
}

#[derive(Clone, Debug, Default)]
pub struct BoolControlOptions {
    pub accessibility: AccessibilityOptions,
    pub size: Option<Expr>,
    pub width: Option<LengthValue>,
    pub spacing: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub wrapping: Option<TextWrapping>,
    pub font: Option<FontPreset>,
    pub alignment: Option<TextAlignment>,
    pub icon: Option<char>,
    pub icon_size: Option<Expr>,
    pub icon_line_height: Option<Expr>,
    pub icon_shaping: Option<TextShaping>,
}

fn push_bool_control_background_roots<'a>(
    roots: &mut Vec<&'a Expr>,
    background: &'a Option<BackgroundValue>,
) {
    if let Some(BackgroundValue::Linear { angle, stops }) = background {
        roots.push(angle);
        roots.extend(stops.iter().map(|stop| &stop.offset));
    }
}

fn push_bool_control_option_roots<'a>(
    roots: &mut Vec<&'a Expr>,
    options: &'a BoolControlOptions,
    include_accessibility: bool,
) {
    if include_accessibility {
        roots.extend(
            [
                &options.accessibility.label,
                &options.accessibility.description,
            ]
            .into_iter()
            .flatten(),
        );
    }
    roots.extend(options.size.as_ref());
    push_input_length_root(roots, &options.width);
    roots.extend(
        [
            &options.spacing,
            &options.text_size,
            &options.line_height,
            &options.icon_size,
            &options.icon_line_height,
        ]
        .into_iter()
        .flatten(),
    );
}

fn bool_control_semantic_key(
    kind: &str,
    leading: impl std::fmt::Debug,
    disabled: &Option<Expr>,
    options: &BoolControlOptions,
    style: impl std::fmt::Debug,
    route: &Route,
) -> String {
    format!(
        "{kind}|leading={leading:?}|disabled={disabled:?}|options={options:?}|style={style:?}|route={route:?}"
    )
}

#[derive(Clone, Debug, Default)]
pub struct CheckboxStyleSet {
    pub preset: CheckboxStylePreset,
    pub custom: Option<ExternCall>,
    pub active_checked: Option<CheckboxStatusStyle>,
    pub active_unchecked: Option<CheckboxStatusStyle>,
    pub hovered_checked: Option<CheckboxStatusStyle>,
    pub hovered_unchecked: Option<CheckboxStatusStyle>,
    pub disabled_checked: Option<CheckboxStatusStyle>,
    pub disabled_unchecked: Option<CheckboxStatusStyle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckboxStylePreset {
    #[default]
    Primary,
    Secondary,
    Success,
    Danger,
}

#[derive(Clone, Debug, Default)]
pub struct CheckboxStatusStyle {
    pub background: Option<BackgroundValue>,
    pub icon_color: Option<String>,
    pub text_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub span: Option<Span>,
}

pub(crate) fn checkbox_expression_roots<'a>(
    label: &'a Expr,
    checked: &'a Expr,
    disabled: &'a Option<Expr>,
    options: &'a BoolControlOptions,
    style: &'a CheckboxStyleSet,
) -> Vec<&'a Expr> {
    let mut roots = vec![label, checked];
    roots.extend(disabled);
    push_bool_control_option_roots(&mut roots, options, true);
    if let Some(custom) = &style.custom {
        roots.extend(&custom.args);
    }
    for status in [
        &style.active_checked,
        &style.active_unchecked,
        &style.hovered_checked,
        &style.hovered_unchecked,
        &style.disabled_checked,
        &style.disabled_unchecked,
    ]
    .into_iter()
    .flatten()
    {
        roots.extend(checkbox_status_expression_roots(status));
    }
    roots
}

pub(crate) fn checkbox_status_expression_roots(status: &CheckboxStatusStyle) -> Vec<&Expr> {
    let mut roots = Vec::new();
    push_bool_control_background_roots(&mut roots, &status.background);
    roots.extend(
        [
            &status.border_width,
            &status.radius,
            &status.radius_top_left,
            &status.radius_top_right,
            &status.radius_bottom_right,
            &status.radius_bottom_left,
        ]
        .into_iter()
        .flatten(),
    );
    roots
}

pub(crate) fn checkbox_semantic_key(
    id: &Option<Id>,
    label: &Expr,
    checked: &Expr,
    disabled: &Option<Expr>,
    options: &BoolControlOptions,
    style: &CheckboxStyleSet,
    route: &Route,
) -> String {
    bool_control_semantic_key(
        "checkbox",
        (id, label, checked),
        disabled,
        options,
        style,
        route,
    )
}

#[derive(Clone, Debug, Default)]
pub struct TogglerStyleSet {
    pub custom: Option<ExternCall>,
    pub active_checked: Option<TogglerStatusStyle>,
    pub active_unchecked: Option<TogglerStatusStyle>,
    pub hovered_checked: Option<TogglerStatusStyle>,
    pub hovered_unchecked: Option<TogglerStatusStyle>,
    pub disabled_checked: Option<TogglerStatusStyle>,
    pub disabled_unchecked: Option<TogglerStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct TogglerStatusStyle {
    pub background: Option<BackgroundValue>,
    pub background_border_color: Option<String>,
    pub background_border_width: Option<Expr>,
    pub foreground: Option<BackgroundValue>,
    pub foreground_border_color: Option<String>,
    pub foreground_border_width: Option<Expr>,
    pub text_color: Option<String>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub padding_ratio: Option<Expr>,
    pub span: Option<Span>,
}

pub(crate) fn toggler_expression_roots<'a>(
    label: &'a Expr,
    checked: &'a Expr,
    disabled: &'a Option<Expr>,
    options: &'a BoolControlOptions,
    style: &'a TogglerStyleSet,
) -> Vec<&'a Expr> {
    let mut roots = vec![label, checked];
    roots.extend(disabled);
    push_bool_control_option_roots(&mut roots, options, true);
    if let Some(custom) = &style.custom {
        roots.extend(&custom.args);
    }
    for status in [
        &style.active_checked,
        &style.active_unchecked,
        &style.hovered_checked,
        &style.hovered_unchecked,
        &style.disabled_checked,
        &style.disabled_unchecked,
    ]
    .into_iter()
    .flatten()
    {
        roots.extend(toggler_status_expression_roots(status));
    }
    roots
}

pub(crate) fn toggler_status_expression_roots(status: &TogglerStatusStyle) -> Vec<&Expr> {
    let mut roots = Vec::new();
    push_bool_control_background_roots(&mut roots, &status.background);
    roots.extend(status.background_border_width.as_ref());
    push_bool_control_background_roots(&mut roots, &status.foreground);
    roots.extend(status.foreground_border_width.as_ref());
    roots.extend(
        [
            &status.radius,
            &status.radius_top_left,
            &status.radius_top_right,
            &status.radius_bottom_right,
            &status.radius_bottom_left,
            &status.padding_ratio,
        ]
        .into_iter()
        .flatten(),
    );
    roots
}

pub(crate) fn toggler_semantic_key(
    id: &Option<Id>,
    label: &Expr,
    checked: &Expr,
    disabled: &Option<Expr>,
    options: &BoolControlOptions,
    style: &TogglerStyleSet,
    route: &Route,
) -> String {
    bool_control_semantic_key(
        "toggler",
        (id, label, checked),
        disabled,
        options,
        style,
        route,
    )
}

#[derive(Clone, Debug, Default)]
pub struct RadioStyleSet {
    pub custom: Option<ExternCall>,
    pub active_selected: Option<RadioStatusStyle>,
    pub active_unselected: Option<RadioStatusStyle>,
    pub hovered_selected: Option<RadioStatusStyle>,
    pub hovered_unselected: Option<RadioStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct RadioStatusStyle {
    pub background: Option<BackgroundValue>,
    pub dot_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub text_color: Option<String>,
    pub span: Option<Span>,
}

pub(crate) fn radio_expression_roots<'a>(
    label: &'a Expr,
    value: &'a Expr,
    selected: &'a Expr,
    options: &'a BoolControlOptions,
    style: &'a RadioStyleSet,
) -> Vec<&'a Expr> {
    let mut roots = vec![label, value, selected];
    push_bool_control_option_roots(&mut roots, options, false);
    if let Some(custom) = &style.custom {
        roots.extend(&custom.args);
    }
    for status in [
        &style.active_selected,
        &style.active_unselected,
        &style.hovered_selected,
        &style.hovered_unselected,
    ]
    .into_iter()
    .flatten()
    {
        roots.extend(radio_status_expression_roots(status));
    }
    roots
}

pub(crate) fn radio_status_expression_roots(status: &RadioStatusStyle) -> Vec<&Expr> {
    let mut roots = Vec::new();
    push_bool_control_background_roots(&mut roots, &status.background);
    roots.extend(status.border_width.as_ref());
    roots
}

pub(crate) fn radio_semantic_key(
    id: &Option<Id>,
    label: &Expr,
    value: &Expr,
    selected: &Expr,
    options: &BoolControlOptions,
    style: &RadioStyleSet,
    route: &Route,
) -> String {
    bool_control_semantic_key(
        "radio",
        (id, label, value, selected),
        &None,
        options,
        style,
        route,
    )
}
