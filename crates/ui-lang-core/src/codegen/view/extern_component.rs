use super::*;

pub(in crate::codegen) fn render_extern_component(
    component: &ResolvedExternComponent,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let program = document.program();
    let arguments = component
        .arguments
        .iter()
        .map(|argument| {
            let value_mode = match argument.mode {
                ResolvedExternComponentArgumentMode::Owned => ValueMode::Owned,
                ResolvedExternComponentArgumentMode::BorrowedAsRef
                | ResolvedExternComponentArgumentMode::Borrowed => ValueMode::Borrowed,
            };
            let code = checked_expr_use_code(program, argument.expression, env, value_mode)?;
            Ok(match argument.mode {
                ResolvedExternComponentArgumentMode::Owned => code,
                ResolvedExternComponentArgumentMode::BorrowedAsRef => {
                    format!("::std::convert::AsRef::as_ref(&({code}))")
                }
                ResolvedExternComponentArgumentMode::Borrowed => {
                    format!("::std::borrow::Borrow::borrow(&({code}))")
                }
            })
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let mapped = component.route.as_ref().map_or_else(
        || Ok(format!("move |__value| {message}::__ExternNoop")),
        |route| {
            resolved_interaction_route_callback_code(
                route,
                "__value",
                &["__value"],
                env,
                program,
                message,
            )
        },
    )?;
    Ok(format!(
        "{}({arguments}).map({mapped}).into()",
        component.function.rust_path
    ))
}
