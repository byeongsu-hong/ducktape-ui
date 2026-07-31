use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_container(
    options: &ContainerOptions,
    id: &Option<Id>,
    content: &ViewNode,
    span: &Span,
    document: &RenderDocument<'_>,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let accessibility_key =
        accessibility_key_code(id.as_ref(), "container", span, scope, env, document)?;
    let child_scope = id.as_ref().map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )?;
    let content = render_node(content, document, message, env, &child_scope, slot)?;
    let mut style = document.program().style_use(span)?.style.clone();
    let mut surface = options.style.clone();
    // A dashed border replaces the solid one rather than adding to it: iced
    // can only draw a solid quad border, so both style lanes drop theirs and
    // the dash is stroked over the same rectangle instead.
    let dash = (!options.border_dash.is_empty())
        .then(|| border_dash_code(options, &style, env, document))
        .transpose()?;
    if dash.is_some() {
        surface.border_color = None;
        surface.border_width = None;
        style.border_color = None;
        style.border_width = 0;
    }
    let mut code = String::from("::iced::widget::container(__container_content)");
    if let Some(id) = id {
        write!(
            code,
            ".id(::iced::widget::Id::from({}))",
            id_code(id, scope, env, document)?
        )
        .unwrap();
    }
    if let Some(padding) = style.padding_code() {
        write!(code, ".padding({padding})").unwrap();
    }
    append_size(&mut code, &style);
    if let Some(max_width) = style.max_width {
        write!(code, ".max_width({max_width})").unwrap();
    }
    if style.clip {
        code.push_str(".clip(true)");
    }
    if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    append_dimensions(&mut code, [&options.width, &options.height], env, document)?;
    for (method, value) in [
        ("max_width", &options.max_width),
        ("max_height", &options.max_height),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(align) = options.align_x {
        let align = match align {
            FlexAlignment::Start => "Left",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "Right",
        };
        write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
    }
    if let Some(align) = options.align_y {
        let align = match align {
            FlexAlignment::Start => "Top",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "Bottom",
        };
        write!(code, ".align_y(::iced::alignment::Vertical::{align})").unwrap();
    }
    if let Some(clip) = &options.clip {
        write!(
            code,
            ".clip({})",
            expr_code(clip, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(mut surface) = container_surface_style_value(
        &style,
        &surface,
        options.custom_style.as_ref(),
        env,
        document,
    )? {
        // A custom style can return its own solid border. Clear that final
        // value too, after every style lane has been composed, so the dash is
        // always a replacement instead of an overlay on a solid stroke.
        if dash.is_some() {
            surface =
                format!("{{ let mut __style = {surface}; __style.border.width = 0.0; __style }}");
        }
        write!(code, ".style(move |__theme| {surface})").unwrap();
    }
    if let Some(dash) = dash {
        code = format!("::ui_lang_runtime::dashed_border({code}, {dash})");
    }
    let code = if style.self_center {
        format!("::iced::widget::container({code}).width(::iced::Fill).center_x(::iced::Fill)")
    } else {
        code
    };
    Ok(format!(
        "{{ let __a11y_key = {accessibility_key}; let __container_content: __IceElement<'_, {message}> = {content}; let __container = {code}; ::ui_lang_runtime::accessible(__container, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}"
    ))
}

/// Builds the arguments of the dashed-border stroke: the colour, width and
/// radius the surface would have drawn its solid border with, plus the dash
/// pattern. Corner radii come from the same typed/utility pair the quad reads,
/// so the stroke traces the surface it replaces, per corner.
fn border_dash_code(
    options: &ContainerOptions,
    style: &ResolvedStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let color = options
        .style
        .border_color
        .as_ref()
        .expect("checker requires `border=` on a dashed box");
    let width = options
        .style
        .border_width
        .as_ref()
        .map(|width| clamped_f32_code(width, "0.0", "f32::MAX", env, document))
        .transpose()?
        .unwrap_or_else(|| format!("{}.0", style.border_width.max(1)));
    let radius = radius_code(
        options.style.radius.as_ref(),
        [
            options.style.radius_top_left.as_ref(),
            options.style.radius_top_right.as_ref(),
            options.style.radius_bottom_right.as_ref(),
            options.style.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )?
    .unwrap_or_else(|| format!("::iced::border::Radius::from({}.0)", style.radius));
    let segments = options
        .border_dash
        .iter()
        .map(|segment| clamped_f32_code(segment, "0.0", "f32::MAX", env, document))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!(
        "{}, {width}, {radius}, ::std::vec![{segments}]",
        theme_color(document, color)
    ))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_overlay(
    id: &Option<Id>,
    options: &OverlayOptions,
    content: &ViewNode,
    layer: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let child_scope = rendered_child_scope(id.as_ref(), scope, env, document)?;
    let content = render_node(content, document, message, env, &child_scope, slot)?;
    let layer = render_node(layer, document, message, env, &child_scope, slot)?;
    let visible = expr_code(&options.visible, env, document, ValueMode::Owned)?;
    let padding = expr_code(&options.padding, env, document, ValueMode::Owned)?;
    let backdrop = theme_color(document, &options.backdrop);
    let dismiss = options.dismiss.as_ref().map_or_else(
        || Ok(format!("{message}::__ExternNoop")),
        |route| route_code(route, "", env, document, message),
    )?;
    let align_x = match options.align_x {
        FlexAlignment::Start => "Left",
        FlexAlignment::Center => "Center",
        FlexAlignment::End => "Right",
    };
    let align_y = match options.align_y {
        FlexAlignment::Start => "Top",
        FlexAlignment::Center => "Center",
        FlexAlignment::End => "Bottom",
    };
    let noop = format!("{message}::__ExternNoop");
    let rendered = format!(
        "{{ let __overlay_base: __IceElement<'_, {message}> = {content}; let __overlay_stack = ::iced::widget::Stack::new().width(::iced::Fill).height(::iced::Fill).push(__overlay_base); if {visible} {{ let __overlay_layer: __IceElement<'_, {message}> = {layer}; let __overlay_backdrop = ::iced::widget::container(::iced::widget::space()).width(::iced::Fill).height(::iced::Fill).style(move |_| ::iced::widget::container::Style {{ background: ::std::option::Option::Some(::iced::Background::Color({backdrop})), ..::iced::widget::container::Style::default() }}); let __overlay_backdrop: __IceElement<'_, {message}> = ::iced::widget::mouse_area(__overlay_backdrop).on_press({dismiss}).on_release({noop}).on_right_press({noop}).on_right_release({noop}).on_middle_press({noop}).on_middle_release({noop}).on_scroll(|_| {noop}).into(); let __overlay_panel = ::iced::widget::mouse_area(__overlay_layer).on_press({noop}).on_release({noop}).on_right_press({noop}).on_right_release({noop}).on_middle_press({noop}).on_middle_release({noop}).on_scroll(|_| {noop}); let __overlay_panel: __IceElement<'_, {message}> = ::iced::widget::container(__overlay_panel).width(::iced::Fill).height(::iced::Fill).padding({padding} as f32).align_x(::iced::alignment::Horizontal::{align_x}).align_y(::iced::alignment::Vertical::{align_y}).into(); let __overlay_surface: __IceElement<'_, {message}> = ::iced::widget::Stack::new().width(::iced::Fill).height(::iced::Fill).push(__overlay_backdrop).push(__overlay_panel).into(); __overlay_stack.push(::iced::widget::float(__overlay_surface).translate(|_, _| ::iced::Vector::new(::core::f32::EPSILON, 0.0))).into() }} else {{ __overlay_stack.into() }} }}"
    );
    identify_rendered(rendered, id.as_ref(), message, env, document, scope)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_rich_text(
    id: &Option<Id>,
    options: &TextOptions,
    color: &Option<String>,
    spans: &[RichSpan],
    route: &Option<Route>,
    node_span: &Span,
    document: &RenderDocument<'_>,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
) -> Result<String, Error> {
    let spans = spans
        .iter()
        .map(|item| render_rich_span(item, document, env))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let style = &document.program().style_use(node_span)?.style;
    let mut code = String::from("::iced::widget::rich_text(__rich_spans)");
    append_text_options(&mut code, options, style, env, document)?;
    if let Some(color) = color {
        write!(code, ".color({})", theme_color(document, color)).unwrap();
    } else if let Some(color) = &style.text_color {
        write!(code, ".color({})", resolved_theme_color(color)).unwrap();
    }
    if let Some(route) = route {
        let callback = route_callback_code(route, "__link", "__link", env, document, message)?;
        write!(code, ".on_link_click({callback})").unwrap();
    }
    let rendered = format!(
        "{{ let __rich_spans: ::std::vec::Vec<::iced::widget::text::Span<'_, ::std::string::String>> = ::std::vec![{spans}]; {code}.into() }}"
    );
    let Some(id) = id else {
        return Ok(rendered);
    };
    let id = id_code(id, scope, env, document)?;
    Ok(format!(
        "{{ let __a11y_key = {id}; let __identified_text: __IceElement<'_, {message}> = {rendered}; ::ui_lang_runtime::accessible(__identified_text, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::Label).logical_id(__a11y_key.clone()).into() }}"
    ))
}
