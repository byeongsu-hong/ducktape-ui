//! Typed guest options and routes for the host's searchable combo box.
use super::*;

pub(super) fn render(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let combo = program.resolved_combo_box(id)?;
    let origin = combo.origin;
    refuse_when(
        program,
        origin,
        combo.custom_style.is_some() || combo.menu.custom.is_some(),
        "a custom combo input or menu style",
    )?;
    let state = resolved_combo_state(combo, env, program)?;
    let state_key = match &state.state {
        Some(StateBinding::App(name)) => format!(
            "{}.to_owned()",
            rust_string(&format!("{}/combo-state/{name}", program.app_name()))
        ),
        _ => {
            return Err(refused(
                program,
                origin,
                "a combo parameter without owned app state",
            ));
        }
    };
    let state = &state.code;
    let selected = resolved_expr_use_code(program, combo.selected, env, ValueMode::Owned)?;
    let placeholder = resolved_expr_use_code(program, combo.placeholder, env, ValueMode::Owned)?;
    let indexed = |route: &ResolvedInteractionRoute| -> Result<String, Error> {
        let callback = resolved_interaction_route_callback_code(
            route,
            "__value",
            &["__value"],
            env,
            program,
            message,
        )?;
        Ok(handler_code(
            "u32",
            message,
            &callback,
            &format!(
                "let __table: ::std::vec::Vec<{message}> = __options.iter().cloned().map(__route).collect(); move |__sent: u32| __table.get(__sent as usize).cloned()"
            ),
        ))
    };
    let input = combo
        .input
        .as_ref()
        .map(|route| -> Result<String, Error> {
            let callback = resolved_interaction_route_callback_code(
                route,
                "__value",
                &["__value"],
                env,
                program,
                message,
            )?;
            Ok(handler_code(
                "::std::string::String",
                message,
                &callback,
                "move |__value| Some(__route(__value))",
            ))
        })
        .transpose()?;
    let route = |route: Option<&ResolvedInteractionRoute>| -> Result<String, Error> {
        Ok(option_code(
            route
                .map(|route| {
                    resolved_interaction_route_code(route, &[], env, program, message)
                        .map(|message| format!("{SLOTS}::message({message})"))
                })
                .transpose()?,
        ))
    };
    let face = |style| -> Result<String, Error> {
        Ok(option_code(input_face_data_code(
            style, program, env, origin,
        )?))
    };
    let style = format!(
        "{WIRE}::InputStyle {{ active: {}, hovered: {}, focused: {}, focused_hovered: {}, disabled: {}, ..Default::default() }}",
        input_face_data_code(combo.styles.active.as_ref(), program, env, origin)?
            .unwrap_or_else(|| format!("{WIRE}::InputFace::default()")),
        face(combo.styles.hovered.as_ref())?,
        face(combo.styles.focused.as_ref())?,
        face(combo.styles.focused_hovered.as_ref())?,
        face(combo.styles.disabled.as_ref())?
    );
    let number = |value: Option<CheckedExprUseId>, low: &str| -> Result<String, Error> {
        Ok(option_code(
            value
                .map(|value| clamped_f32_code(value, low, "f32::MAX", program, env))
                .transpose()?,
        ))
    };
    let icon = combo.icon.as_ref().map(|icon| -> Result<String, Error> { Ok(format!("{WIRE}::ComboIcon {{ code_point: {:?}, font: {}, size: {}, spacing: {}, right: {} }}", icon.code_point, pick::font(icon.font.as_ref()), number(icon.size, "f32::EPSILON")?, icon.spacing.map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env)).transpose()?.unwrap_or_else(|| "0.0".into()), matches!(icon.side, ResolvedInputIconSide::Right))) }).transpose()?;
    let settings = format!(
        "{WIRE}::ComboOptions {{ menu_height: {}, padding: {}, text_size: {}, line_height: {}, shaping: {}, font: {}, icon: {}, input: {}, hover: {}, open: {}, close: {}, style: {style}, menu: {} }}",
        dimension_code(combo.menu_height.as_ref(), false, program, env, origin)?,
        number(combo.padding, "0.0")?,
        number(combo.text_size, "f32::EPSILON")?,
        number(combo.line_height, "f32::EPSILON")?,
        pick::shaping(combo.shaping),
        pick::font(combo.font.as_ref()),
        option_code(icon),
        option_code(input),
        option_code(combo.hover.as_ref().map(indexed).transpose()?),
        route(combo.open.as_ref())?,
        route(combo.close.as_ref())?,
        menu_code(&combo.menu, program, env, origin)?
    );
    let handler = indexed(&combo.selection)?;
    Ok(format!(
        "{{ let __options = ({state}).options().to_vec(); let __selected = {selected}; {WIRE}::Node::ComboBox {{ state_key: {state_key}, key: {}, options: __options.iter().map(|value| value.to_string()).collect(), selected: __selected.as_ref().and_then(|chosen| __options.iter().position(|value| value == chosen)).map(|index| index as u32), reset: ({state}).reset_revision(), placeholder: ({placeholder}).to_string(), on_select: {handler}, width: {}, settings: Box::new({settings}) }} }}",
        key_code(identity, "combo-box", origin, scope, env, program)?,
        dimension_code(combo.width.as_ref(), false, program, env, origin)?
    ))
}
