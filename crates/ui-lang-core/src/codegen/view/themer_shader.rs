use super::*;

fn extern_view_arguments(
    adapter: &ResolvedExternViewAdapter,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    adapter
        .arguments
        .iter()
        .map(|argument| {
            let value_mode = match argument.mode {
                ResolvedExternViewArgumentMode::Owned => ValueMode::Owned,
                ResolvedExternViewArgumentMode::BorrowedAsRef
                | ResolvedExternViewArgumentMode::Borrowed => ValueMode::Borrowed,
            };
            let code = checked_expr_use_code(program, argument.expression, env, value_mode)?;
            Ok(match argument.mode {
                ResolvedExternViewArgumentMode::Owned => code,
                ResolvedExternViewArgumentMode::BorrowedAsRef => {
                    format!("::std::convert::AsRef::as_ref(&({code}))")
                }
                ResolvedExternViewArgumentMode::Borrowed => {
                    format!("::std::borrow::Borrow::borrow(&({code}))")
                }
            })
        })
        .collect::<Result<Vec<_>, Error>>()
        .map(|arguments| arguments.join(", "))
}

fn extern_view_mapping(
    adapter: &ResolvedExternViewAdapter,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    adapter.route.as_ref().map_or_else(
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
    )
}

pub(in crate::codegen) fn render_themer(
    themer: &ResolvedThemer,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let program = document.program();
    let args = extern_view_arguments(&themer.adapter, program, env)?;
    let mapped = extern_view_mapping(&themer.adapter, program, message, env)?;
    Ok(format!(
        "{{ let (__theme, __content, __text_color, __background) = {}({args}); let mut __themer = ::iced::widget::themer(__theme, __content); if let ::std::option::Option::Some(__text_color) = __text_color {{ __themer = __themer.text_color(__text_color); }} if let ::std::option::Option::Some(__background) = __background {{ __themer = __themer.background(__background); }} let __themed: __IceElement<'_, {}> = __themer.into(); __themed.map({mapped}).into() }}",
        themer.adapter.function.rust_path,
        themer.adapter.output.rust(document.extern_structs())
    ))
}

fn shader_length_code(
    length: &ResolvedContainerLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedContainerLength::Fill => "::iced::Fill".into(),
        ResolvedContainerLength::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        ResolvedContainerLength::Shrink => "::iced::Shrink".into(),
        ResolvedContainerLength::FixedF64(expression) => format!(
            "{} as f32",
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedContainerLength::FixedLength(expression) => {
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        }
    })
}

pub(in crate::codegen) fn render_shader(
    shader: &ResolvedShader,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let program = document.program();
    let args = extern_view_arguments(&shader.adapter, program, env)?;
    let mut code = format!(
        "::iced::widget::Shader::new({}({args}))",
        shader.adapter.function.rust_path
    );
    for (method, length) in ["width", "height"]
        .into_iter()
        .zip([&shader.width, &shader.height])
    {
        if let Some(length) = length {
            write!(
                code,
                ".{method}({})",
                shader_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    let mapped = extern_view_mapping(&shader.adapter, program, message, env)?;
    let output = shader.adapter.output.rust(document.extern_structs());
    Ok(format!(
        "{{ let __shader: __IceElement<'_, {output}> = {code}.into(); __shader.map({mapped}).into() }}"
    ))
}
