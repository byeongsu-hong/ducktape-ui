use super::*;

pub(super) fn tree_widget_task(
    operation: &ResolvedWidgetOperation,
    route: Option<&ResolvedRoute>,
    origin: OriginId,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    let target = |target: &ResolvedWidgetTarget| {
        if target.window.is_some() {
            return Err(program.error_at_origin(
                "E190",
                origin,
                "a view module can operate only on its own mounted widget tree",
            ));
        }
        let (path, _) = resolved_widget_target_parts(target, env, program)?;
        Ok(format!("::std::string::String::from({path})"))
    };
    let index = |expression| {
        let value = resolved_expr_use_code(program, expression, env, ValueMode::Owned)?;
        Ok::<_, Error>(format!(
            "u32::try_from((({value}) as i64).max(0)).unwrap_or(u32::MAX)"
        ))
    };
    let offset = |expression| {
        let value = resolved_expr_use_code(program, expression, env, ValueMode::Owned)?;
        Ok::<_, Error>(format!("({value}) as f32"))
    };
    use ResolvedWidgetOperation as O;
    let command = match operation {
        O::FocusPrevious => "FocusPrevious".into(),
        O::FocusNext => "FocusNext".into(),
        O::Focus { target: id } => format!("Focus {{ target: {} }}", target(id)?),
        O::Focused { target: id } => {
            let route = route
                .ok_or_else(|| program.invariant_at_origin(origin, "focused query has no route"))?;
            let message_code = resolved_route_code(route, &["value"], env, program, message)?;
            return Ok(format!(
                "::ui_lang_guest::widget::is_focused({}).map(move |value| {message_code})",
                target(id)?
            ));
        }
        O::CursorFront { target: id } => format!("CursorFront {{ target: {} }}", target(id)?),
        O::CursorEnd { target: id } => format!("CursorEnd {{ target: {} }}", target(id)?),
        O::Cursor {
            target: id,
            position,
        } => format!(
            "Cursor {{ target: {}, position: {} }}",
            target(id)?,
            index(*position)?
        ),
        O::SelectAll { target: id } => format!("SelectAll {{ target: {} }}", target(id)?),
        O::Select {
            target: id,
            start,
            end,
        } => format!(
            "Select {{ target: {}, start: {}, end: {} }}",
            target(id)?,
            index(*start)?,
            index(*end)?
        ),
        O::Snap { target: id, x, y } => format!(
            "Snap {{ target: {}, x: {}, y: {} }}",
            target(id)?,
            offset(*x)?,
            offset(*y)?
        ),
        O::SnapEnd { target: id } => format!("SnapEnd {{ target: {} }}", target(id)?),
        O::ScrollTo { target: id, x, y } => format!(
            "ScrollTo {{ target: {}, x: {}, y: {} }}",
            target(id)?,
            offset(*x)?,
            offset(*y)?
        ),
        O::ScrollBy { target: id, x, y } => format!(
            "ScrollBy {{ target: {}, x: {}, y: {} }}",
            target(id)?,
            offset(*x)?,
            offset(*y)?
        ),
        O::ScrollToKey { target: id, key } => {
            let key = resolved_expr_use_code(program, *key, env, ValueMode::Owned)?;
            format!(
                "ScrollToKey {{ target: {}, key: ::ui_lang_guest::wire::ListKey::from({key}).virtual_key() }}",
                target(id)?
            )
        }
        O::Find { .. } => {
            return Err(program.error_at_origin(
                "E190",
                origin,
                "widget selectors are not available in a view module",
            ));
        }
    };
    Ok(format!(
        "::ui_lang_guest::widget::perform::<{message}>(::ui_lang_guest::wire::WidgetCommand::{command})"
    ))
}
