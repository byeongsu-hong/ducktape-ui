use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_container(
    container: &ResolvedContainer,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document;
    let accessibility_key = resolved_accessibility_key_code(
        identity,
        "container",
        container.origin,
        scope,
        env,
        document,
    )?;
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    let content = render_node(content, document, message, env, &child_scope, slot)?;
    let mut style = container.utility_style.clone();
    let mut surface = container.surface.clone();
    // A dashed border replaces the solid one rather than adding to it: iced
    // can only draw a solid quad border, so both style lanes drop theirs and
    // the dash is stroked over the same rectangle instead.
    let dash = (!container.border_dash.is_empty())
        .then(|| resolved_container_border_dash_code(container, &style, program, env))
        .transpose()?;
    if dash.is_some() {
        surface.border_color = None;
        surface.border_width = None;
        style.border_color = None;
        style.border_width = 0;
    }
    let mut code = String::from("::iced::widget::container(__container_content)");
    if let Some(identity) = identity {
        write!(
            code,
            ".id(::iced::widget::Id::from({}))",
            resolved_view_identity_code(identity, scope, env, document)?
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
    if let Some(padding) = resolved_container_padding_code(&container.padding, program, env)? {
        write!(code, ".padding({padding})").unwrap();
    }
    append_resolved_container_dimensions(
        &mut code,
        [&container.width, &container.height],
        program,
        env,
    )?;
    for (method, value) in [
        ("max_width", container.max_width),
        ("max_height", container.max_height),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                resolved_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(align) = container.align_x {
        let align = match align {
            ResolvedContainerAlignment::Start => "Left",
            ResolvedContainerAlignment::Center => "Center",
            ResolvedContainerAlignment::End => "Right",
        };
        write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
    }
    if let Some(align) = container.align_y {
        let align = match align {
            ResolvedContainerAlignment::Start => "Top",
            ResolvedContainerAlignment::Center => "Center",
            ResolvedContainerAlignment::End => "Bottom",
        };
        write!(code, ".align_y(::iced::alignment::Vertical::{align})").unwrap();
    }
    if let Some(clip) = container.clip {
        write!(
            code,
            ".clip({})",
            resolved_expr_use_code(program, clip, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    let mut setup = String::new();
    if let Some(mut surface) = resolved_container_surface_style_value(
        &style,
        &surface,
        container.custom_style.as_ref(),
        program,
        env,
        &mut setup,
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
        "{{ let __a11y_key = {accessibility_key}; let __container_content: __IceElement<'_, {message}> = {content}; let __container = {{ {setup} {code} }}; ::ui_lang_runtime::accessible(__container, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).logical_id(__a11y_key.clone()).into() }}"
    ))
}

/// Builds the arguments of the dashed-border stroke: the colour, width and
/// radius the surface would have drawn its solid border with, plus the dash
/// pattern. Corner radii come from the same typed/utility pair the quad reads,
/// so the stroke traces the surface it replaces, per corner.
fn resolved_container_border_dash_code(
    container: &ResolvedContainer,
    style: &ResolvedStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let color = container
        .surface
        .border_color
        .as_ref()
        .expect("checker requires `border=` on a dashed box");
    let width = container
        .surface
        .border_width
        .map(|width| resolved_container_clamped_f32(width, "0.0", "f32::MAX", program, env))
        .transpose()?
        .unwrap_or_else(|| format!("{}.0", style.border_width.max(1)));
    let radius = resolved_container_radius_code(&container.surface.radius, program, env)?
        .unwrap_or_else(|| format!("::iced::border::Radius::from({}.0)", style.radius));
    let segments = container
        .border_dash
        .iter()
        .map(|segment| resolved_container_clamped_f32(*segment, "0.0", "f32::MAX", program, env))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!(
        "{}, {width}, {radius}, ::std::vec![{segments}]",
        resolved_theme_color(color)
    ))
}

fn resolved_container_padding_code(
    padding: &ResolvedContainerPadding,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let value = |expression: Option<ResolvedExpressionId>| {
        expression
            .map(|expression| resolved_expr_use_code(program, expression, env, ValueMode::Owned))
            .transpose()
    };
    let all = value(padding.all)?.unwrap_or_else(|| "0.0".into());
    let x = value(padding.x)?.unwrap_or_else(|| all.clone());
    let y = value(padding.y)?.unwrap_or_else(|| all.clone());
    let top = value(padding.top)?.unwrap_or_else(|| y.clone());
    let right = value(padding.right)?.unwrap_or_else(|| x.clone());
    let bottom = value(padding.bottom)?.unwrap_or(y);
    let left = value(padding.left)?.unwrap_or(x);
    Ok(Some(format!(
        "::ui_lang_runtime::bounded_padding({top}, {right}, {bottom}, {left})"
    )))
}

fn append_resolved_container_dimensions(
    code: &mut String,
    dimensions: [&Option<ResolvedContainerLength>; 2],
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<(), Error> {
    for (method, length) in ["width", "height"].into_iter().zip(dimensions) {
        let Some(length) = length else { continue };
        let value = resolved_length_code(length, program, env)?;
        write!(code, ".{method}({value})").unwrap();
    }
    Ok(())
}

fn resolved_container_custom_style_code(
    style: &ResolvedContainerCustomStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let arguments = style
        .arguments
        .iter()
        .map(|argument| resolved_expr_use_code(program, *argument, env, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?;
    let suffix = arguments
        .into_iter()
        .map(|argument| format!(", {argument}"))
        .collect::<String>();
    Ok(format!(
        "{}(__theme{suffix})",
        program.extern_function(style.function).rust_path
    ))
}

fn resolved_container_background_code(
    background: &ResolvedContainerBackground,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match background {
        ResolvedContainerBackground::Color(color) => {
            format!("::iced::Background::Color({})", resolved_theme_color(color))
        }
        ResolvedContainerBackground::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                resolved_expr_use_code(program, *angle, env, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    resolved_expr_use_code(program, stop.offset, env, ValueMode::Owned)?,
                    resolved_theme_color(&stop.color)
                )
                .unwrap();
            }
            code.push(')');
            code
        }
    })
}

fn resolved_container_radius_code(
    radius: &ResolvedContainerRadius,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
    {
        return Ok(None);
    }
    let base = radius
        .all
        .map(|value| resolved_container_clamped_f32(value, "0.0", "f32::MAX", program, env))
        .transpose()?
        .unwrap_or_else(|| "0.0".into());
    let corner = |value: Option<ResolvedExpressionId>| {
        value
            .map(|value| resolved_container_clamped_f32(value, "0.0", "f32::MAX", program, env))
            .transpose()
    };
    let top_left = corner(radius.top_left)?.unwrap_or_else(|| base.clone());
    let top_right = corner(radius.top_right)?.unwrap_or_else(|| base.clone());
    let bottom_right = corner(radius.bottom_right)?.unwrap_or_else(|| base.clone());
    let bottom_left = corner(radius.bottom_left)?.unwrap_or(base);
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {top_left}, top_right: {top_right}, bottom_right: {bottom_right}, bottom_left: {bottom_left} }}"
    )))
}

fn resolved_container_surface_style_value(
    utilities: &ResolvedStyle,
    surface: &ResolvedContainerSurface,
    custom: Option<&ResolvedContainerCustomStyle>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    setup: &mut String,
) -> Result<Option<String>, Error> {
    let _derived_guard = enter_escaping_derived_reads();
    let has_typed = surface.background.is_some()
        || surface.text_color.is_some()
        || surface.border_color.is_some()
        || surface.border_width.is_some()
        || surface.radius.all.is_some()
        || surface.radius.top_left.is_some()
        || surface.radius.top_right.is_some()
        || surface.radius.bottom_right.is_some()
        || surface.radius.bottom_left.is_some()
        || surface.shadow_color.is_some()
        || surface.shadow_x.is_some()
        || surface.shadow_y.is_some()
        || surface.shadow_blur.is_some()
        || surface.pixel_snap.is_some();
    let utility = container_style_value(utilities);
    let custom = custom
        .map(|style| resolved_container_custom_style_code(style, program, env))
        .transpose()?;
    if !has_typed && custom.is_none() {
        return Ok(utility);
    }
    if !has_typed && utility.is_none() {
        return Ok(custom);
    }
    let has_custom = custom.is_some();
    let base = custom
        .or_else(|| utility.clone())
        .unwrap_or_else(|| "::iced::widget::container::Style::default()".into());
    let mut code = format!("{{ let mut __style = {base};");
    if has_custom {
        append_container_utility_overrides(&mut code, utilities);
    }
    if let Some(background) = &surface.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({});",
            resolved_container_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(alpha) = surface.background_alpha {
        // The checker already pinned this to a single background color. The
        // read stays inside the style closure so a frame that only redraws
        // still paints the value the animation has now. Component state is
        // reached through `setup`'s aliases: this closure is `move`, and
        // naming the instance scope directly would take it from the component's
        // other animated surfaces.
        let (alpha_env, alpha_setup) = closure_capture_env(env);
        setup.push_str(&alpha_setup);
        write!(
            code,
            " if let ::std::option::Option::Some(::iced::Background::Color(__color)) = &mut __style.background {{ __color.a = (({}) as f32 / 100.0).clamp(0.0, 1.0); }}",
            resolved_expr_use_code(program, alpha, &alpha_env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(color) = &surface.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(width) = surface.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            resolved_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = resolved_container_radius_code(&surface.radius, program, env)? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(color) = &surface.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    for (expression, field) in [
        (surface.shadow_x, "offset.x"),
        (surface.shadow_y, "offset.y"),
        (surface.shadow_blur, "blur_radius"),
    ] {
        if let Some(expression) = expression {
            write!(
                code,
                " __style.shadow.{field} = {} as f32;",
                resolved_expr_use_code(program, expression, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(snap) = surface.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            resolved_expr_use_code(program, snap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(color) = &surface.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    code.push_str(" __style }");
    Ok(Some(code))
}

fn resolved_container_clamped_f32(
    expression: ResolvedExpressionId,
    minimum: &str,
    maximum: &str,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let code = resolved_expr_use_code(program, expression, env, ValueMode::Owned)?;
    Ok(format!("(({code}) as f32).max({minimum}).min({maximum})"))
}

/// A press guard: everything the target does not act on is swallowed rather
/// than reaching what is drawn beneath it.
fn overlay_press_guard(target: &str, message: &str) -> String {
    let noop = format!("{message}::__ExternNoop");
    format!(
        "::iced::widget::mouse_area({target}).on_press({noop}).on_release({noop}).on_right_press({noop}).on_right_release({noop}).on_middle_press({noop}).on_middle_release({noop}).on_scroll(|_| {noop})"
    )
}

/// The overlay's layer element, and the expression that is the panel the user
/// sees — the layer with its press guard in the right place.
///
/// The guard must wrap what the user SEES as the panel. A floated layer is
/// re-hosted at its translated position in a nested overlay (iced `float`
/// captures nothing at its layout slot), so a guard wrapped around the float
/// would sit at the UNTRANSLATED layout position: the drawn panel would
/// dismiss on any press its widgets don't capture, and the empty layout slot
/// would eat clicks meant for the base. When the layer's root is a float, the
/// guard rides inside it instead. (A float nested deeper than the layer root
/// still escapes its guard — keep the float outermost in the layer.)
///
/// The published path calls this too, so a template's compiled panel is the
/// same element the inline path would have built.
pub(in crate::codegen) fn overlay_layer_and_panel(
    layer: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    child_scope: &str,
    slot: Option<&SlotContext>,
) -> Result<(String, String), Error> {
    let layer_view = document.resolved_view(layer)?;
    if let ResolvedViewKind::Float { content: floated } = &layer_view.kind {
        let floated = render_node(*floated, document, message, env, child_scope, slot)?;
        let guarded = format!(
            "{{ let __overlay_floated: __IceElement<'_, {message}> = {floated}; {}.into() }}",
            overlay_press_guard("__overlay_floated", message)
        );
        let float = document.resolved_float(layer)?;
        let rendered = structure::render_resolved_float(float, document, message, env, guarded)?;
        let rendered =
            source_mapped_expression_origin(rendered, document, layer_view.origin, message, false);
        Ok((rendered, "__overlay_layer".to_string()))
    } else {
        let rendered = render_node(layer, document, message, env, child_scope, slot)?;
        Ok((rendered, overlay_press_guard("__overlay_layer", message)))
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_overlay(
    identity: Option<&ResolvedViewIdentity>,
    overlay: &ResolvedOverlay,
    content: ViewId,
    layer: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document;
    let child_scope = rendered_child_scope(identity, scope, env, document)?;
    let content = render_node(content, document, message, env, &child_scope, slot)?;
    let (layer, panel) =
        overlay_layer_and_panel(layer, document, message, env, &child_scope, slot)?;
    let noop = format!("{message}::__ExternNoop");
    let visible = resolved_expr_use_code(program, overlay.visible, env, ValueMode::Owned)?;
    let padding = resolved_expr_use_code(program, overlay.padding, env, ValueMode::Owned)?;
    let backdrop = resolved_theme_color(&overlay.backdrop);
    let dismiss = overlay.dismiss.as_ref().map_or_else(
        || Ok(format!("{message}::__ExternNoop")),
        |route| resolved_interaction_route_code(route, &[], env, program, message),
    )?;
    let align_x = match overlay.align_x {
        ResolvedOverlayAlignment::Start => "Left",
        ResolvedOverlayAlignment::Center => "Center",
        ResolvedOverlayAlignment::End => "Right",
    };
    let align_y = match overlay.align_y {
        ResolvedOverlayAlignment::Start => "Top",
        ResolvedOverlayAlignment::Center => "Center",
        ResolvedOverlayAlignment::End => "Bottom",
    };
    let rendered = format!(
        "{{ let __overlay_base: __IceElement<'_, {message}> = {content}; let __overlay_open = {visible}; let __overlay_base: __IceElement<'_, {message}> = if __overlay_open {{ ::ui_lang_runtime::focus_barrier(__overlay_base).into() }} else {{ __overlay_base }}; let __overlay_stack = ::iced::widget::Stack::new().width(::iced::Fill).height(::iced::Fill).push(__overlay_base); if __overlay_open {{ let __overlay_layer: __IceElement<'_, {message}> = {layer}; let __overlay_backdrop = ::iced::widget::container(::iced::widget::space()).width(::iced::Fill).height(::iced::Fill).style(move |_| ::iced::widget::container::Style {{ background: ::std::option::Option::Some(::iced::Background::Color({backdrop})), ..::iced::widget::container::Style::default() }}); let __overlay_backdrop: __IceElement<'_, {message}> = ::iced::widget::mouse_area(__overlay_backdrop).on_press({dismiss}).on_release({noop}).on_right_press({noop}).on_right_release({noop}).on_middle_press({noop}).on_middle_release({noop}).on_scroll(|_| {noop}).into(); let __overlay_panel = {panel}; let __overlay_panel: __IceElement<'_, {message}> = ::iced::widget::container(__overlay_panel).width(::iced::Fill).height(::iced::Fill).padding({padding} as f32).align_x(::iced::alignment::Horizontal::{align_x}).align_y(::iced::alignment::Vertical::{align_y}).into(); let __overlay_surface: __IceElement<'_, {message}> = ::iced::widget::Stack::new().width(::iced::Fill).height(::iced::Fill).push(__overlay_backdrop).push(__overlay_panel).into(); __overlay_stack.push(::iced::widget::float(__overlay_surface).translate(|_, _| ::iced::Vector::new(::core::f32::EPSILON, 0.0))).into() }} else {{ __overlay_stack.into() }} }}"
    );
    identify_rendered(rendered, identity, message, env, document, scope)
}
