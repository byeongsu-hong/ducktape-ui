use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    node: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    tip: ViewId,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let tooltip = program.resolved_tooltip(node)?;
    let preset = match tooltip.base_style {
        None => "Transparent".into(),
        Some(ResolvedTooltipBaseStyle::Preset(preset)) => format!("{preset:?}"),
        Some(ResolvedTooltipBaseStyle::Custom(_)) => {
            return Err(refused(program, tooltip.origin, "a custom tooltip style"));
        }
    };
    let background = match &tooltip.background {
        None => "None".into(),
        Some(ResolvedTooltipBackground::Color(color)) => option_code(Some(rgba_code(color))),
        Some(ResolvedTooltipBackground::Linear { .. }) => {
            return Err(refused(
                program,
                tooltip.origin,
                "a gradient tooltip background",
            ));
        }
    };
    let value = |value| resolved_expr_use_code(program, value, env, ValueMode::Owned);
    let number = |optional: Option<CheckedExprUseId>| -> Result<String, Error> {
        Ok(option_code(
            optional
                .map(|id| value(id).map(|value| format!("({value}) as f32")))
                .transpose()?,
        ))
    };
    let border = border_code(
        &ResolvedContainerSurface {
            border_color: tooltip.border_color.clone(),
            border_width: tooltip.border_width,
            radius: ResolvedContainerRadius {
                all: tooltip.radius.all,
                top_left: tooltip.radius.top_left,
                top_right: tooltip.radius.top_right,
                bottom_right: tooltip.radius.bottom_right,
                bottom_left: tooltip.radius.bottom_left,
            },
            ..Default::default()
        },
        &ResolvedStyle::default(),
        program,
        env,
    )?;
    let key = key_code(identity, "tooltip", tooltip.origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, env, &child_scope, slot)?;
    let tip = render_node(tip, program, message, env, &child_scope, slot)?;
    Ok(format!(
        "{WIRE}::Node::Tooltip {{ key: {key}, position: {WIRE}::TooltipPosition::{:?}, gap: ({}) as f32, padding: ({}) as f32, delay_ms: u64::try_from({}).unwrap_or(0), snap: {}, style: {WIRE}::TooltipStyle {{ preset: {WIRE}::TooltipPreset::{preset}, background: {background}, text: {}, border: {border}, shadow_color: {}, shadow_x: {}, shadow_y: {}, shadow_blur: {}, pixel_snap: {} }}, children: vec![{content}, {tip}] }}",
        tooltip.position,
        value(tooltip.gap)?,
        value(tooltip.padding)?,
        value(tooltip.delay_ms)?,
        value(tooltip.snap)?,
        option_code(tooltip.text_color.as_ref().map(rgba_code)),
        option_code(tooltip.shadow_color.as_ref().map(rgba_code)),
        number(tooltip.shadow_x)?,
        number(tooltip.shadow_y)?,
        number(tooltip.shadow_blur)?,
        option_code(tooltip.pixel_snap.map(value).transpose()?),
    ))
}
