use super::*;

fn resolved_effect_call(
    kind: EffectKind,
    target: &ResolvedEffectTarget,
    args: &[ResolvedExpressionId],
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let args = args
        .iter()
        .map(|argument| resolved_expr_use_code(program, *argument, env, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    match target {
        ResolvedEffectTarget::Builtin(function) if kind == EffectKind::Task => {
            Ok(match function.as_str() {
                "__ice_system_info" => {
                    "::iced::system::information().map(__ice_system_info)".into()
                }
                "__ice_system_theme" => "::iced::system::theme().map(__ice_system_theme)".into(),
                "__ice_time_now" => "::iced::time::now()".into(),
                "__ice_clipboard_read" => "::iced::clipboard::read()".into(),
                "__ice_clipboard_read_primary" => "::iced::clipboard::read_primary()".into(),
                "__ice_font_load" => format!(
                    "::iced::font::load({args}).map(|result| match result {{ ::std::result::Result::Ok(value) => value, ::std::result::Result::Err(error) => match error {{}} }})"
                ),
                "__ice_image_allocate" => format!("::iced::widget::image::allocate({args})"),
                _ => {
                    return Err(Error::new(
                        "E196",
                        &Span::line(1),
                        "normalized task source references an unknown built-in",
                    ));
                }
            })
        }
        ResolvedEffectTarget::Builtin(_) => Err(Error::new(
            "E196",
            &Span::line(1),
            "normalized non-task effect references a built-in task",
        )),
        ResolvedEffectTarget::Extern(id) => {
            let action = program.extern_function(*id);
            Ok(match kind {
                EffectKind::Future => format!(
                    "::iced::Task::perform({}({args}), |value| value)",
                    action.rust_path
                ),
                EffectKind::Task => format!("{}({args})", action.rust_path),
                EffectKind::Stream => format!(
                    "::iced::Task::run({}({args}), |value| value)",
                    action.rust_path
                ),
            })
        }
    }
}

pub(in crate::codegen) fn task_source_code(
    source: &ResolvedTaskSource,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match source {
        ResolvedTaskSource::Done { value, .. } => format!(
            "::iced::Task::done({})",
            resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
        ),
        ResolvedTaskSource::None { output, .. } => {
            format!(
                "::iced::Task::<{}>::none()",
                rust_type_code(program, output)
            )
        }
        ResolvedTaskSource::Effect {
            kind, target, args, ..
        } => resolved_effect_call(*kind, target, args, program, env)?,
    })
}

pub(in crate::codegen) fn task_flow_code(
    flow: &ResolvedTaskFlow,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let mut task = task_source_code(&flow.source, program, env)?;
    for transform in &flow.transforms {
        match transform {
            ResolvedTaskTransform::Map {
                task: _,
                local,
                binding,
                input,
                input_fallible,
                value,
                ..
            } => {
                let map_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: input.clone(),
                        local: false,
                        state: None,
                        owner: Some(BindingOwner::Local(*local)),
                    },
                )]);
                let value = resolved_expr_use_code(program, *value, &map_env, ValueMode::Owned)?;
                task = if *input_fallible {
                    format!("({task}).map(move |result| result.map(|{binding}| {value}))")
                } else {
                    format!("({task}).map(move |{binding}| {value})")
                };
            }
            ResolvedTaskTransform::Then {
                local,
                binding,
                input,
                source,
                ..
            }
            | ResolvedTaskTransform::AndThen {
                local,
                binding,
                input,
                source,
                ..
            } => {
                let next_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: input.clone(),
                        local: false,
                        state: None,
                        owner: Some(BindingOwner::Local(*local)),
                    },
                )]);
                let next = task_source_code(source, program, &next_env)?;
                let method = if matches!(transform, ResolvedTaskTransform::Then { .. }) {
                    "then"
                } else {
                    "and_then"
                };
                task = format!("({task}).{method}(move |{binding}| {next})");
            }
            ResolvedTaskTransform::MapError {
                local,
                binding,
                input,
                value,
                ..
            } => {
                let map_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: input.clone(),
                        local: false,
                        state: None,
                        owner: Some(BindingOwner::Local(*local)),
                    },
                )]);
                let value = resolved_expr_use_code(program, *value, &map_env, ValueMode::Owned)?;
                task = format!("({task}).map_err(move |{binding}| {value})");
            }
            ResolvedTaskTransform::Collect { .. } => task = format!("({task}).collect()"),
            ResolvedTaskTransform::Discard { .. } => {
                task = format!("({task}).discard::<{message}>()")
            }
        }
    }
    Ok(task)
}
