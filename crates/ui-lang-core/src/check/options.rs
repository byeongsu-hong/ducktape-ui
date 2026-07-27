use super::*;

pub(in crate::check) fn check_lazy_subtree(
    node: &ViewNode,
    document: &Document,
    components: &mut HashSet<String>,
    supplied_slot: bool,
) -> Result<(), Error> {
    match node {
        ViewNode::Input { span, .. } => Err(Error::new(
            "E139",
            span,
            "input cannot live in lazy because iced text input borrows app state",
        )),
        ViewNode::ComboBox { span, .. } => Err(Error::new(
            "E139",
            span,
            "combo cannot live in lazy because iced combo box borrows search state",
        )),
        ViewNode::QrCode { span, .. } => Err(Error::new(
            "E139",
            span,
            "named QR data cannot live in lazy because iced QR code borrows app state",
        )),
        ViewNode::Markdown { span, .. } => Err(Error::new(
            "E139",
            span,
            "markdown cannot live in lazy because iced markdown borrows parsed content",
        )),
        ViewNode::TextEditor { span, .. } => Err(Error::new(
            "E139",
            span,
            "editor cannot live in lazy because iced text editor borrows content state",
        )),
        ViewNode::Slot { span, .. } if !supplied_slot => Err(Error::new(
            "E139",
            span,
            "a lazy subtree cannot borrow a slot from its enclosing component",
        )),
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => {
            for child in children {
                check_lazy_subtree(child, document, components, supplied_slot)?;
            }
            Ok(())
        }
        ViewNode::Match { arms, .. } => {
            for child in arms.iter().flat_map(|arm| &arm.children) {
                check_lazy_subtree(child, document, components, supplied_slot)?;
            }
            Ok(())
        }
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::ResizeHandle { content, .. }
        | ViewNode::Container { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => {
            check_lazy_subtree(content, document, components, supplied_slot)
        }
        ViewNode::Tooltip { content, tip, .. } => {
            check_lazy_subtree(content, document, components, supplied_slot)?;
            check_lazy_subtree(tip, document, components, supplied_slot)
        }
        ViewNode::Overlay { content, layer, .. } => {
            check_lazy_subtree(content, document, components, supplied_slot)?;
            check_lazy_subtree(layer, document, components, supplied_slot)
        }
        ViewNode::PaneGrid { span, .. } => Err(Error::new(
            "E187",
            span,
            "panes cannot live in lazy because its layout state is persistent",
        )),
        ViewNode::Table { columns, .. } => {
            for column in columns {
                check_lazy_subtree(&column.header, document, components, supplied_slot)?;
                check_lazy_subtree(&column.cell, document, components, supplied_slot)?;
            }
            Ok(())
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                check_lazy_subtree(narrow, document, components, supplied_slot)?;
                check_lazy_subtree(wide, document, components, supplied_slot)
            }
            ResponsiveContent::Size { content, .. } => {
                check_lazy_subtree(content, document, components, supplied_slot)
            }
        },
        ViewNode::Component {
            name, slots, span, ..
        } => {
            for slot in slots {
                check_lazy_subtree(&slot.content, document, components, supplied_slot)?;
            }
            if !components.insert(name.clone()) {
                return Err(Error::new(
                    "E139",
                    span,
                    format!("recursive component `{name}` cannot be used in lazy"),
                ));
            }
            let component = document
                .components
                .iter()
                .find(|component| component.name == *name)
                .expect("component names are checked before lazy safety");
            let result =
                check_lazy_subtree(&component.root, document, components, !slots.is_empty());
            components.remove(name);
            result
        }
        _ => Ok(()),
    }
}

pub(in crate::check) fn require_literal_range(
    expr: &Expr,
    min: f64,
    max: Option<f64>,
    label: &str,
    span: &Span,
) -> Result<(), Error> {
    let literal = f64_literal(expr);
    if literal.is_some_and(|value| value < min || max.is_some_and(|max| value > max)) {
        return Err(Error::new(
            "E128",
            span,
            format!("{label} is outside its valid range"),
        ));
    }
    Ok(())
}

pub(in crate::check) fn require_f32_literal_range(
    expr: &Expr,
    min: f64,
    max: Option<f64>,
    label: &str,
    span: &Span,
) -> Result<(), Error> {
    let bound = f64::from(f32::MAX);
    require_literal_range(
        expr,
        min.max(-bound),
        Some(max.unwrap_or(bound).min(bound)),
        label,
        span,
    )
}

pub(in crate::check) fn require_nonnegative_f64(
    expr: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    label: &str,
    span: &Span,
) -> Result<(), Error> {
    require_type(&expr_type(expr, env, document, span)?, &Type::F64, span)?;
    require_f32_literal_range(expr, 0.0, None, label, span)
}

pub(in crate::check) fn require_f32_value(
    expr: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    label: &str,
    span: &Span,
) -> Result<(), Error> {
    require_type(&expr_type(expr, env, document, span)?, &Type::F64, span)?;
    require_f32_literal_range(expr, f64::NEG_INFINITY, None, label, span)
}

pub(in crate::check) fn check_background_value(
    background: &BackgroundValue,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    code: &'static str,
    label: &str,
) -> Result<(), Error> {
    match background {
        BackgroundValue::Color(color) => {
            require_theme_color(color, document, span, code, label)?;
        }
        BackgroundValue::Linear { angle, stops } => {
            require_f32_value(angle, env, document, "gradient angle", span)?;
            for stop in stops {
                require_theme_color(&stop.color, document, span, code, label)?;
                require_type(
                    &expr_type(&stop.offset, env, document, span)?,
                    &Type::F64,
                    span,
                )?;
                require_literal_range(&stop.offset, 0.0, Some(1.0), "gradient stop", span)?;
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn infer_pane_view(
    pane: &PaneView,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<(), Error> {
    let mut pane_env = env.clone();
    if let Some(binding) = &pane.maximized {
        pane_env.insert(binding.clone(), Type::Bool);
    }
    let env = &pane_env;
    check_styles(
        &pane.styles,
        document,
        &pane.span,
        StyleTarget::PaneContent(&pane.style),
    )?;
    check_container_style_options(&pane.style, env, document, &pane.span, "E187")?;
    if let Some(title) = &pane.title {
        for value in [
            &title.padding.all,
            &title.padding.x,
            &title.padding.y,
            &title.padding.top,
            &title.padding.right,
            &title.padding.bottom,
            &title.padding.left,
        ]
        .into_iter()
        .flatten()
        {
            require_type(
                &expr_type(value, env, document, &title.span)?,
                &Type::F64,
                &title.span,
            )?;
            require_f32_literal_range(value, 0.0, None, "pane title padding", &title.span)?;
        }
        check_styles(
            &title.styles,
            document,
            &title.span,
            StyleTarget::PaneTitle(&title.style),
        )?;
        check_container_style_options(&title.style, env, document, &title.span, "E187")?;
    }
    for node in pane.nodes() {
        infer_view(node, env, document, signatures, ids)?;
    }
    Ok(())
}

pub(in crate::check) fn check_container_style_options(
    style: &ContainerStyleOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    code: &'static str,
) -> Result<(), Error> {
    if let Some(background) = &style.background {
        check_background_value(background, env, document, span, code, "surface")?;
    }
    for (color, label) in [
        (&style.text_color, "surface text"),
        (&style.border_color, "surface border"),
        (&style.shadow_color, "surface shadow"),
    ] {
        if let Some(color) = color {
            require_theme_color(color, document, span, code, label)?;
        }
    }
    for value in [
        &style.border_width,
        &style.radius,
        &style.radius_top_left,
        &style.radius_top_right,
        &style.radius_bottom_right,
        &style.radius_bottom_left,
        &style.shadow_blur,
    ]
    .into_iter()
    .flatten()
    {
        require_nonnegative_f64(value, env, document, "surface style metric", span)?;
    }
    for value in [&style.shadow_x, &style.shadow_y].into_iter().flatten() {
        require_f32_value(value, env, document, "surface shadow offset", span)?;
    }
    if let Some(snap) = &style.pixel_snap {
        require_type(&expr_type(snap, env, document, span)?, &Type::Bool, span)?;
    }
    Ok(())
}

pub(in crate::check) fn check_markdown_style(
    style: &MarkdownStyleOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for font in [
        style.font.as_ref(),
        style.inline_code_font.as_ref(),
        style.code_block_font.as_ref(),
    ] {
        check_font(font, document, span)?;
    }
    if let Some(background) = &style.inline_code_background {
        check_background_value(
            background,
            env,
            document,
            span,
            "E139",
            "markdown inline code",
        )?;
    }
    for (color, label) in [
        (&style.inline_code_color, "markdown inline code"),
        (
            &style.inline_code_border_color,
            "markdown inline code border",
        ),
        (&style.link_color, "markdown link"),
    ] {
        if let Some(color) = color {
            require_theme_color(color, document, span, "E139", label)?;
        }
    }
    for value in [
        style.inline_code_padding.all.as_ref(),
        style.inline_code_padding.x.as_ref(),
        style.inline_code_padding.y.as_ref(),
        style.inline_code_padding.top.as_ref(),
        style.inline_code_padding.right.as_ref(),
        style.inline_code_padding.bottom.as_ref(),
        style.inline_code_padding.left.as_ref(),
        style.inline_code_border_width.as_ref(),
        style.inline_code_radius.as_ref(),
        style.inline_code_radius_top_left.as_ref(),
        style.inline_code_radius_top_right.as_ref(),
        style.inline_code_radius_bottom_right.as_ref(),
        style.inline_code_radius_bottom_left.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        require_nonnegative_f64(value, env, document, "markdown style metric", span)?;
    }
    Ok(())
}

pub(in crate::check) fn check_float_style_options(
    style: &FloatStyleOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if let Some(color) = &style.shadow_color {
        require_theme_color(color, document, span, "E128", "float shadow")?;
    }
    for value in [
        &style.shadow_blur,
        &style.radius,
        &style.radius_top_left,
        &style.radius_top_right,
        &style.radius_bottom_right,
        &style.radius_bottom_left,
    ]
    .into_iter()
    .flatten()
    {
        require_nonnegative_f64(value, env, document, "float style metric", span)?;
    }
    for value in [&style.shadow_x, &style.shadow_y].into_iter().flatten() {
        require_f32_value(value, env, document, "float shadow offset", span)?;
    }
    Ok(())
}

pub(in crate::check) fn f64_literal(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::F64(value) => Some(*value),
        Expr::I64(value) => Some(*value as f64),
        Expr::Unary {
            op: UnaryOp::Neg,
            value,
        } => f64_literal(value).map(|value| -value),
        _ => None,
    }
}

pub(in crate::check) fn check_accessibility_options(
    options: &AccessibilityOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for value in [&options.label, &options.description].into_iter().flatten() {
        require_type(&expr_type(value, env, document, span)?, &Type::Str, span)?;
    }
    Ok(())
}

pub(in crate::check) fn check_bool_control_options(
    options: &BoolControlOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    check_font(options.font.as_ref(), document, span)?;
    if let Some(length) = &options.width {
        check_length_value(length, env, document, span, "control width")?;
    }
    for (value, label, min) in [
        (&options.size, "control size", f64::EPSILON),
        (&options.spacing, "control spacing", 0.0),
        (&options.text_size, "control text size", f64::EPSILON),
        (&options.line_height, "control line height", f64::EPSILON),
        (&options.icon_size, "checkbox icon size", f64::EPSILON),
        (
            &options.icon_line_height,
            "checkbox icon line height",
            f64::EPSILON,
        ),
    ] {
        if let Some(value) = value {
            require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            require_f32_literal_range(value, min, None, label, span)?;
        }
    }
    if options.icon.is_none()
        && (options.icon_size.is_some()
            || options.icon_line_height.is_some()
            || options.icon_shaping.is_some())
    {
        return Err(Error::new(
            "E129",
            span,
            "checkbox icon properties require `icon=\"x\"`",
        ));
    }
    Ok(())
}

pub(in crate::check) fn check_checkbox_styles(
    styles: &CheckboxStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [
        &styles.active_checked,
        &styles.active_unchecked,
        &styles.hovered_checked,
        &styles.hovered_unchecked,
        &styles.disabled_checked,
        &styles.disabled_unchecked,
    ]
    .into_iter()
    .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        if let Some(background) = &style.background {
            check_background_value(
                background,
                env,
                document,
                span,
                "E129",
                "checkbox background",
            )?;
        }
        for (color, label) in [
            (&style.icon_color, "checkbox icon"),
            (&style.text_color, "checkbox text"),
            (&style.border_color, "checkbox border"),
        ] {
            if let Some(color) = color {
                require_theme_color(color, document, span, "E129", label)?;
            }
        }
        for value in [
            &style.border_width,
            &style.radius,
            &style.radius_top_left,
            &style.radius_top_right,
            &style.radius_bottom_right,
            &style.radius_bottom_left,
        ]
        .into_iter()
        .flatten()
        {
            require_nonnegative_f64(value, env, document, "checkbox style metric", span)?;
        }
    }
    Ok(())
}

pub(in crate::check) fn check_toggler_styles(
    styles: &TogglerStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [
        &styles.active_checked,
        &styles.active_unchecked,
        &styles.hovered_checked,
        &styles.hovered_unchecked,
        &styles.disabled_checked,
        &styles.disabled_unchecked,
    ]
    .into_iter()
    .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        for (background, label) in [
            (&style.background, "toggler background"),
            (&style.foreground, "toggler foreground"),
        ] {
            if let Some(background) = background {
                check_background_value(background, env, document, span, "E129", label)?;
            }
        }
        for (color, label) in [
            (&style.background_border_color, "toggler background border"),
            (&style.foreground_border_color, "toggler foreground border"),
            (&style.text_color, "toggler text"),
        ] {
            if let Some(color) = color {
                require_theme_color(color, document, span, "E129", label)?;
            }
        }
        for value in [
            &style.background_border_width,
            &style.foreground_border_width,
            &style.radius,
            &style.radius_top_left,
            &style.radius_top_right,
            &style.radius_bottom_right,
            &style.radius_bottom_left,
        ]
        .into_iter()
        .flatten()
        {
            require_nonnegative_f64(value, env, document, "toggler style metric", span)?;
        }
        if let Some(ratio) = &style.padding_ratio {
            require_type(&expr_type(ratio, env, document, span)?, &Type::F64, span)?;
            require_literal_range(ratio, 0.0, Some(0.5), "toggler padding ratio", span)?;
        }
    }
    Ok(())
}

pub(in crate::check) fn check_radio_styles(
    styles: &RadioStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [
        &styles.active_selected,
        &styles.active_unselected,
        &styles.hovered_selected,
        &styles.hovered_unselected,
    ]
    .into_iter()
    .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        if let Some(background) = &style.background {
            check_background_value(background, env, document, span, "E129", "radio background")?;
        }
        for (color, label) in [
            (&style.dot_color, "radio dot"),
            (&style.border_color, "radio border"),
            (&style.text_color, "radio text"),
        ] {
            if let Some(color) = color {
                require_theme_color(color, document, span, "E129", label)?;
            }
        }
        if let Some(width) = &style.border_width {
            require_nonnegative_f64(width, env, document, "radio border width", span)?;
        }
    }
    Ok(())
}

pub(in crate::check) fn check_pick_list_handle(
    handle: Option<&PickListHandle>,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    let Some(handle) = handle else { return Ok(()) };
    let icons = match handle {
        PickListHandle::Arrow { size } => {
            if let Some(size) = size {
                require_nonnegative_f64(size, env, document, "pick handle size", span)?;
            }
            return Ok(());
        }
        PickListHandle::Static(icon) => [Some(icon), None],
        PickListHandle::Dynamic { closed, open } => [Some(closed), Some(open)],
        PickListHandle::None => return Ok(()),
    };
    for icon in icons.into_iter().flatten() {
        check_font(icon.font.as_ref(), document, &icon.span)?;
        for (value, label) in [
            (&icon.size, "pick handle icon size"),
            (&icon.line_height, "pick handle icon line height"),
        ] {
            if let Some(value) = value {
                require_type(
                    &expr_type(value, env, document, &icon.span)?,
                    &Type::F64,
                    &icon.span,
                )?;
                require_f32_literal_range(value, 0.0, None, label, &icon.span)?;
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn check_pick_list_styles(
    options: &PickListOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for style in [
        &options.style.active,
        &options.style.hovered,
        &options.style.opened,
        &options.style.opened_hovered,
    ]
    .into_iter()
    .flatten()
    {
        let style_span = style.span.as_ref().unwrap_or(span);
        check_container_style_options(&style.options, env, document, style_span, "E129")?;
        for (color, label) in [
            (&style.placeholder_color, "pick placeholder"),
            (&style.handle_color, "pick handle"),
        ] {
            if let Some(color) = color {
                require_theme_color(color, document, style_span, "E129", label)?;
            }
        }
    }
    check_menu_style(options.menu_style.as_deref(), env, document, span)?;
    Ok(())
}

pub(in crate::check) fn check_menu_style(
    style: Option<&MenuStyleOptions>,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    let Some(style) = style else { return Ok(()) };
    let style_span = style.span.as_ref().unwrap_or(span);
    check_container_style_options(&style.options, env, document, style_span, "E129")?;
    if let Some(color) = &style.selected_text_color {
        require_theme_color(color, document, style_span, "E129", "selected text")?;
    }
    if let Some(background) = &style.selected_background {
        check_background_value(
            background,
            env,
            document,
            style_span,
            "E129",
            "selected background",
        )?;
    }
    Ok(())
}

pub(in crate::check) fn check_text_input_icon(
    icon: Option<&TextInputIcon>,
    env: &HashMap<String, Type>,
    document: &Document,
    widget: &str,
) -> Result<(), Error> {
    let Some(icon) = icon else { return Ok(()) };
    check_font(icon.font.as_ref(), document, &icon.span)?;
    for (value, label) in [
        (&icon.size, format!("{widget} icon size")),
        (&icon.spacing, format!("{widget} icon spacing")),
    ] {
        if let Some(value) = value {
            require_type(
                &expr_type(value, env, document, &icon.span)?,
                &Type::F64,
                &icon.span,
            )?;
            require_f32_literal_range(value, 0.0, None, &label, &icon.span)?;
        }
    }
    Ok(())
}

pub(in crate::check) fn check_text_input_styles(
    styles: &TextInputStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    widget: &str,
) -> Result<(), Error> {
    for style in [
        &styles.active,
        &styles.hovered,
        &styles.focused,
        &styles.focused_hovered,
        &styles.disabled,
    ]
    .into_iter()
    .flatten()
    {
        let style_span = style.span.as_ref().unwrap_or(span);
        check_container_style_options(&style.options, env, document, style_span, "E129")?;
        for (color, label) in [
            (&style.icon_color, "icon"),
            (&style.placeholder_color, "placeholder"),
            (&style.value_color, "value"),
            (&style.selection_color, "selection"),
        ] {
            if let Some(color) = color {
                require_theme_color(
                    color,
                    document,
                    style_span,
                    "E129",
                    &format!("{widget} {label}"),
                )?;
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn check_scroll_styles(
    styles: &[ScrollStatusStyle],
    env: &HashMap<String, Type>,
    document: &Document,
) -> Result<(), Error> {
    for style in styles {
        for surface in [
            &style.container,
            &style.horizontal_rail.rail,
            &style.horizontal_rail.scroller,
            &style.vertical_rail.rail,
            &style.vertical_rail.scroller,
            &style.auto_scroll,
        ] {
            check_container_style_options(surface, env, document, &style.span, "E129")?;
        }
        if let Some(gap) = &style.gap {
            check_background_value(gap, env, document, &style.span, "E129", "scroll gap")?;
        }
        if let Some(color) = &style.auto_scroll_icon {
            require_theme_color(color, document, &style.span, "E129", "scroll auto icon")?;
        }
    }
    Ok(())
}

pub(in crate::check) fn check_slider_styles(
    styles: &SliderStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [&styles.active, &styles.hovered, &styles.dragged]
        .into_iter()
        .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        for (background, label) in [
            (&style.rail_start, "slider rail start"),
            (&style.rail_end, "slider rail end"),
            (&style.handle_color, "slider handle"),
        ] {
            if let Some(background) = background {
                check_background_value(background, env, document, span, "E129", label)?;
            }
        }
        for color in [&style.rail_border_color, &style.handle_border_color]
            .into_iter()
            .flatten()
        {
            require_theme_color(color, document, span, "E129", "slider")?;
        }
        for (value, label) in [
            (&style.rail_width, "slider rail width"),
            (&style.rail_border_width, "slider rail border width"),
            (&style.rail_radius, "slider rail radius"),
            (&style.rail_radius_top_left, "slider rail radius"),
            (&style.rail_radius_top_right, "slider rail radius"),
            (&style.rail_radius_bottom_right, "slider rail radius"),
            (&style.rail_radius_bottom_left, "slider rail radius"),
            (&style.handle_border_width, "slider handle border width"),
            (&style.handle_radius, "slider handle radius"),
            (&style.handle_radius_top_left, "slider handle radius"),
            (&style.handle_radius_top_right, "slider handle radius"),
            (&style.handle_radius_bottom_right, "slider handle radius"),
            (&style.handle_radius_bottom_left, "slider handle radius"),
        ] {
            if let Some(value) = value {
                require_nonnegative_f64(value, env, document, label, span)?;
            }
        }
        if let Some(SliderHandleShape::Circle(radius)) = &style.handle_shape {
            require_nonnegative_f64(radius, env, document, "slider handle radius", span)?;
        }
        let has_handle_radius = style.handle_radius.is_some()
            || style.handle_radius_top_left.is_some()
            || style.handle_radius_top_right.is_some()
            || style.handle_radius_bottom_right.is_some()
            || style.handle_radius_bottom_left.is_some();
        if has_handle_radius
            && !matches!(
                &style.handle_shape,
                Some(SliderHandleShape::Rectangle { .. })
            )
        {
            return Err(Error::new(
                "E129",
                span,
                "slider handle radius requires `handle=rect(N)` in the same status",
            ));
        }
    }
    Ok(())
}

pub(in crate::check) fn check_text_options(
    options: &TextOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    check_font(options.font.as_ref(), document, span)?;
    if let Some(style) = &options.custom_style {
        let function = extern_function(document, &style.function, ExternKind::TextStyle, span)?;
        check_call_args(function, &style.args, env, document, span)?;
    }
    for length in [&options.width, &options.height].into_iter().flatten() {
        check_length_value(length, env, document, span, "text bounds")?;
    }
    for (value, label) in [
        (options.size.as_ref(), "text size"),
        (
            options.line_height.as_ref().map(|height| match height {
                TextLineHeight::Relative(value) | TextLineHeight::Absolute(value) => value,
            }),
            "text line height",
        ),
    ] {
        if let Some(value) = value {
            require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            require_f32_literal_range(value, f64::EPSILON, None, label, span)?;
        }
    }
    Ok(())
}
