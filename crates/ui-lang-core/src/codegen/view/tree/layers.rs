use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn layout(
    layout: &ResolvedLayout,
    identity: Option<&ResolvedViewIdentity>,
    children: &[ViewId],
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let origin = layout.origin;
    let style = &layout.utility_style;
    let mut allowed = style.clone();
    if matches!(layout.mode, ResolvedLayoutMode::Stack(_)) {
        allowed.clip = false;
    }
    refuse_box_utilities(&allowed, program, origin)?;
    let (background, border) = layout_surface_code(style);
    let key = key_code(identity, "layout", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let padding = edges_code(
        &ResolvedContainerPadding::default(),
        style.padding,
        program,
        env,
    )?;
    let (kind, width, height, options) = match &layout.mode {
        ResolvedLayoutMode::Stack(stack) => {
            let clip = stack
                .clip
                .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| style.clip.to_string());
            (
                "Stack",
                stack.width.as_ref(),
                stack.height.as_ref(),
                format!("clip: {clip}, under: {}u32,", stack.under),
            )
        }
        ResolvedLayoutMode::Hover(hover) => {
            let tint = option_code(hover.tint.as_ref().map(rgba_code));
            let open = hover
                .open
                .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
            (
                "Hover",
                None,
                None,
                format!(
                    "tint: {tint}, radius: {} as f32, open: {open},",
                    hover.radius
                ),
            )
        }
        _ => unreachable!("layered layout"),
    };
    let width = dimension_code(width, style.width_fill, program, env, origin)?;
    let height = dimension_code(height, style.height_fill, program, env, origin)?;
    let mut body = format!("{{ let mut __children: Vec<__IceElement<'_, {message}>> = Vec::new();");
    render_children(
        &mut body,
        children,
        program,
        message,
        env,
        &child_scope,
        slot,
    )?;
    write!(body, " {WIRE}::Node::{kind} {{ key: {key}, width: {width}, height: {height}, padding: {padding}, background: {background}, border: {border}, {options} children: __children }} }}").unwrap();
    Ok(body)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn overlay(
    node: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    layer: ViewId,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let overlay = program.resolved_overlay(node)?;
    let key = key_code(identity, "overlay", overlay.origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, env, &child_scope, slot)?;
    let layer = render_node(layer, program, message, env, &child_scope, slot)?;
    let visible = resolved_expr_use_code(program, overlay.visible, env, ValueMode::Owned)?;
    let padding = resolved_expr_use_code(program, overlay.padding, env, ValueMode::Owned)?;
    let backdrop = rgba_code(&overlay.backdrop);
    let dismiss = option_code(
        overlay
            .dismiss
            .as_ref()
            .map(|route| resolved_interaction_route_code(route, &[], env, program, message))
            .transpose()?
            .map(|message| format!("{SLOTS}::message({message})")),
    );
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
    Ok(format!(
        "{{ let mut __children = vec![{content}]; if {visible} {{ __children.push({layer}); }} {WIRE}::Node::Overlay {{ key: {key}, padding: ({padding}) as f32, backdrop: {backdrop}, align_x: {WIRE}::AlignX::{align_x}, align_y: {WIRE}::AlignY::{align_y}, on_dismiss: {dismiss}, children: __children }} }}"
    ))
}
