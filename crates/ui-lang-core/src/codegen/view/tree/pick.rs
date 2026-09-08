//! Declarative pick options copied into the host's native widget.
use super::*;

pub(super) fn options(
    pick: &ResolvedPickList,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let metric = |value: Option<CheckedExprUseId>| -> Result<String, Error> {
        Ok(option_code(
            value
                .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
                .transpose()?,
        ))
    };
    let positive = |value: Option<CheckedExprUseId>| -> Result<String, Error> {
        Ok(option_code(
            value
                .map(|value| clamped_f32_code(value, "f32::EPSILON", "f32::MAX", program, env))
                .transpose()?,
        ))
    };
    let route = |route: Option<&ResolvedInteractionRoute>| -> Result<String, Error> {
        Ok(option_code(
            route
                .map(|route| {
                    resolved_interaction_route_code(route, &[], env, program, message)
                        .map(|code| format!("{SLOTS}::message({code})"))
                })
                .transpose()?,
        ))
    };
    let handle = pick
        .handle
        .as_ref()
        .map(|handle| -> Result<String, Error> {
            Ok(match handle {
                ResolvedPickListHandle::Arrow { size } => {
                    format!("{WIRE}::PickHandle::Arrow {{ size: {} }}", positive(*size)?)
                }
                ResolvedPickListHandle::Static(value) => {
                    format!("{WIRE}::PickHandle::Static({})", icon(value, program, env)?)
                }
                ResolvedPickListHandle::Dynamic { closed, open } => format!(
                    "{WIRE}::PickHandle::Dynamic {{ closed: {}, open: {} }}",
                    icon(closed, program, env)?,
                    icon(open, program, env)?
                ),
                ResolvedPickListHandle::None => format!("{WIRE}::PickHandle::None"),
            })
        })
        .transpose()?;
    Ok(format!(
        "{WIRE}::PickOptions {{ menu_height: {}, padding: {}, text_size: {}, line_height: {}, shaping: {}, font: {}, handle: {}, on_open: {}, on_close: {} }}",
        dimension_code(pick.menu_height.as_ref(), false, program, env, pick.origin)?,
        metric(pick.padding)?,
        positive(pick.text_size)?,
        positive(pick.line_height)?,
        shaping(pick.shaping),
        font(pick.font.as_ref()),
        option_code(handle),
        route(pick.open.as_ref())?,
        route(pick.close.as_ref())?
    ))
}
fn shaping(value: Option<ResolvedSelectionShaping>) -> String {
    option_code(value.map(|value| format!("{WIRE}::Shaping::{value:?}")))
}
fn font(value: Option<&ResolvedTextFont>) -> String {
    option_code(value.map(|value| match value {
        ResolvedTextFont::Named(value) => text::named_font(value, None),
        value => {
            let family = if matches!(value, ResolvedTextFont::Monospace) { "Monospace" } else { "SansSerif" };
            format!("{WIRE}::NamedFont {{ family: {WIRE}::FontFamily::{family}, weight: {WIRE}::Weight::Normal, stretch: {WIRE}::FontStretch::Normal, style: {WIRE}::FontStyle::Normal }}")
        }
    }))
}
fn icon(
    value: &ResolvedPickListIcon,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let positive = |value: Option<CheckedExprUseId>| -> Result<String, Error> {
        Ok(option_code(
            value
                .map(|value| clamped_f32_code(value, "f32::EPSILON", "f32::MAX", program, env))
                .transpose()?,
        ))
    };
    Ok(format!(
        "{WIRE}::PickIcon {{ code_point: {:?}, font: {}, size: {}, line_height: {}, shaping: {} }}",
        value.code_point,
        font(value.font.as_ref()),
        positive(value.size)?,
        positive(value.line_height)?,
        shaping(value.shaping)
    ))
}
