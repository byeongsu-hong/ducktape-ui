use super::*;

pub(in crate::codegen) fn task_source_code(
    source: &TaskSource,
    document: &Document,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    match source {
        TaskSource::Done { value, .. } => Ok(format!(
            "::iced::Task::done({})",
            expr_code(value, env, document, ValueMode::Owned)?
        )),
        TaskSource::None { output, .. } => Ok(format!(
            "::iced::Task::<{}>::none()",
            output.rust(&document.structs)
        )),
        TaskSource::Effect {
            kind,
            function,
            args,
            span,
        } => {
            if *kind == EffectKind::Task {
                match function.as_str() {
                    "__ice_system_info" => {
                        return Ok("::iced::system::information().map(__ice_system_info)".into());
                    }
                    "__ice_system_theme" => {
                        return Ok("::iced::system::theme().map(__ice_system_theme)".into());
                    }
                    "__ice_time_now" => return Ok("::iced::time::now()".into()),
                    "__ice_clipboard_read" => return Ok("::iced::clipboard::read()".into()),
                    "__ice_clipboard_read_primary" => {
                        return Ok("::iced::clipboard::read_primary()".into());
                    }
                    "__ice_font_load" => {
                        let bytes = expr_code(&args[0], env, document, ValueMode::Owned)?;
                        return Ok(format!(
                            "::iced::font::load({bytes}).map(|result| match result {{ ::std::result::Result::Ok(value) => value, ::std::result::Result::Err(error) => match error {{}} }})"
                        ));
                    }
                    "__ice_image_allocate" => {
                        let handle = expr_code(&args[0], env, document, ValueMode::Owned)?;
                        return Ok(format!("::iced::widget::image::allocate({handle})"));
                    }
                    _ => {}
                }
            }
            let action =
                find_extern_function(document, function, (*kind).into()).ok_or_else(|| {
                    Error::new(
                        "E130",
                        span,
                        format!("unknown extern task source `{function}`"),
                    )
                })?;
            let args = expr_list_code(args, env, document)?;
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

pub(in crate::codegen) fn task_flow_code(
    root: &TaskSource,
    transforms: &[TaskTransform],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let mut task = task_source_code(root, document, env)?;
    let type_env = env
        .iter()
        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
        .collect::<HashMap<_, _>>();
    for (index, transform) in transforms.iter().enumerate() {
        match transform {
            TaskTransform::Map { binding, value, .. } => {
                let (output, error) =
                    crate::check::task_flow_type(root, &transforms[..index], document, &type_env)?;
                let output = output.expect("discard is the final transform");
                let map_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: output,
                        local: false,
                        state: None,
                        owner: None,
                    },
                )]);
                let value = expr_code(value, &map_env, document, ValueMode::Owned)?;
                task = if error.is_some() {
                    format!("({task}).map(move |result| result.map(|{binding}| {value}))")
                } else {
                    format!("({task}).map(move |{binding}| {value})")
                };
            }
            TaskTransform::Then {
                binding, source, ..
            }
            | TaskTransform::AndThen {
                binding, source, ..
            } => {
                let (output, error) =
                    crate::check::task_flow_type(root, &transforms[..index], document, &type_env)?;
                let output = output.expect("discard is the final transform");
                let binding_ty =
                    if matches!(transform, TaskTransform::AndThen { .. }) && error.is_none() {
                        let Type::Option(inner) = output else {
                            unreachable!("checked optional try")
                        };
                        *inner
                    } else {
                        output
                    };
                let next_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: binding_ty,
                        local: false,
                        state: None,
                        owner: None,
                    },
                )]);
                let next = task_source_code(source, document, &next_env)?;
                let method = if matches!(transform, TaskTransform::Then { .. }) {
                    "then"
                } else {
                    "and_then"
                };
                task = format!("({task}).{method}(move |{binding}| {next})");
            }
            TaskTransform::MapError { binding, value, .. } => {
                let (_, error) =
                    crate::check::task_flow_type(root, &transforms[..index], document, &type_env)?;
                let error = error.expect("checked map-err input");
                let map_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: error,
                        local: false,
                        state: None,
                        owner: None,
                    },
                )]);
                let value = expr_code(value, &map_env, document, ValueMode::Owned)?;
                task = format!("({task}).map_err(move |{binding}| {value})");
            }
            TaskTransform::Collect { .. } => task = format!("({task}).collect()"),
            TaskTransform::Discard { .. } => task = format!("({task}).discard::<{message}>()"),
        }
    }
    Ok(task)
}
