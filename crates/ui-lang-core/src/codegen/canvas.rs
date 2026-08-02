use super::*;

pub(in crate::codegen) fn render_canvas(
    canvas: &ResolvedCanvas,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let options = &canvas.options;
    let state_fields = canvas
        .states
        .iter()
        .map(|local| {
            let origin = program.origin(local.origin);
            let span = Span {
                line: origin.line,
                column: origin.column,
            };
            resolved_type_code(program, &local.resolved_ty, &span)
                .map(|ty| format!("{}: {ty},", local.name))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(" ");
    let state_initials = canvas
        .states
        .iter()
        .map(|local| {
            Ok(format!(
                "{}: {},",
                local.name,
                resolved_initializer_code(&local.initializer, program)?
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(" ");
    let mut canvas_env = env.snapshot();
    for local in &canvas.states {
        canvas_env.insert(
            local.name.clone(),
            Binding {
                code: format!("__state.{}", local.name),
                ty: local.ty.clone(),
                local: false,
                state: None,
                owner: Some(BindingOwner::Local(local.local)),
            },
        );
    }
    canvas_env.insert(
        "canvas_width".into(),
        Binding {
            code: "(__bounds.width as f64)".into(),
            ty: Type::F64,
            local: true,
            state: None,
            owner: Some(BindingOwner::Local(canvas.width_local)),
        },
    );
    canvas_env.insert(
        "canvas_height".into(),
        Binding {
            code: "(__bounds.height as f64)".into(),
            ty: Type::F64,
            local: true,
            state: None,
            owner: Some(BindingOwner::Local(canvas.height_local)),
        },
    );
    let mut captures = component_state_scopes(env);
    if let Some((_, context)) = component_context(env) {
        captures.push(context.code.clone());
    }
    captures.sort();
    captures.dedup();
    let (draw_env, draw_captures) = canvas_capture_env(&canvas_env, &captures, "draw");
    let (update_env, update_captures) = canvas_capture_env(env, &captures, "update");
    let (canvas_update_env, _) = canvas_capture_env(&canvas_env, &captures, "update");
    let (interaction_env, interaction_captures) =
        canvas_capture_env(&canvas_env, &captures, "interaction");
    let draw_commands = canvas_commands_code(&canvas.commands, &draw_env, program)?;
    let use_cache = options.cache.is_some();
    let cache_key = if let Some(dependency) = &options.cache {
        let dependency = resolved_expr_use_code(program, *dependency, env, ValueMode::Owned)?;
        format!(
            "::std::option::Option::Some({{ let mut __hasher = ::std::hash::DefaultHasher::new(); ::std::hash::Hash::hash(&(__ice_palette.name, {dependency}), &mut __hasher); ::std::hash::Hasher::finish(&__hasher) }})"
        )
    } else {
        "::std::option::Option::None".into()
    };
    let update = canvas_update_code(
        options,
        &canvas.events,
        &update_env,
        &canvas_update_env,
        program,
        message,
        use_cache,
    )?;
    let interaction = if let Some(value) = &options.interaction_expr {
        let interaction = resolved_expr_use_code(
            program,
            value.expression,
            &interaction_env,
            ValueMode::Owned,
        )?;
        if value.source == Type::MouseInteraction {
            interaction
        } else {
            format!(
                "{{ let __interaction = {interaction}; __ice_canvas_interaction(__interaction.as_str()) }}"
            )
        }
    } else {
        format!(
            "::iced::mouse::Interaction::{}",
            options
                .interaction
                .map(mouse_interaction_code)
                .unwrap_or("None")
        )
    };
    let interaction_outside = options
        .interaction_outside
        .as_ref()
        .map(|outside| {
            resolved_expr_use_code(program, *outside, &interaction_env, ValueMode::Owned)
        })
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let interaction_guard = match interaction_outside.as_str() {
        "true" => "true".into(),
        "false" => "__cursor.is_over(__bounds)".into(),
        _ => format!("({interaction_outside}) || __cursor.is_over(__bounds)"),
    };
    let cache_group = options.cache_group.as_ref().map_or_else(
        || "::std::option::Option::None".into(),
        |group| {
            format!(
                "::std::option::Option::Some(*{}.get_or_init(::iced::widget::canvas::Group::unique))",
                canvas_group_symbol(group)
            )
        },
    );
    let cache_setup = if use_cache {
        "let __cache = __state.cache.get_or_init(|| match __cache_group { ::std::option::Option::Some(group) => ::iced::widget::canvas::Cache::with_group(group), ::std::option::Option::None => ::iced::widget::canvas::Cache::new() }); if __state.cache_key.get() != __cache_key { __cache.clear(); __state.cache_key.set(__cache_key); }"
    } else {
        ""
    };
    let geometry = if use_cache {
        "__cache.draw(__renderer, __bounds.size(), __paint)"
    } else {
        "{ let mut __frame = ::iced::widget::canvas::Frame::new(__renderer, __bounds.size()); __paint(&mut __frame); __frame.into_geometry() }"
    };
    let mut code = format!(
        "{{
struct __IceCanvasState {{ cache: ::std::cell::OnceCell<::iced::widget::canvas::Cache>, cache_key: ::std::cell::Cell<::std::option::Option<u64>>, inside: bool, {state_fields} }}
impl ::std::default::Default for __IceCanvasState {{
fn default() -> Self {{ Self {{ cache: ::std::cell::OnceCell::new(), cache_key: ::std::cell::Cell::new(::std::option::Option::None), inside: false, {state_initials} }} }}
}}
let __cache_key: ::std::option::Option<u64> = {cache_key};
let __cache_group: ::std::option::Option<::iced::widget::canvas::Group> = {cache_group};
{draw_captures}{update_captures}{interaction_captures}
let __program = __IceCanvasProgram::<__IceCanvasState, {message}, _, _, _> {{
draw: move |__state: &__IceCanvasState, __renderer: &::iced::Renderer, __theme: &::iced::Theme, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{
let _ = (&__cache_key, &__cache_group);
{cache_setup}
let __paint = |__frame: &mut ::iced::widget::canvas::Frame| {{ {draw_commands} }};
let __geometry = {geometry};
::std::vec![__geometry]
}},
update: {update},
interaction: move |__state: &__IceCanvasState, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{
if {interaction_guard} {{ {interaction} }} else {{ ::iced::mouse::Interaction::default() }}
}},
message: ::std::marker::PhantomData,
}};
let __canvas = ::iced::widget::canvas(__program)"
    );
    append_canvas_dimensions(&mut code, [&options.width, &options.height], env, program)?;
    code.push_str("; __canvas.into() }");
    Ok(code)
}

fn append_canvas_dimensions(
    code: &mut String,
    dimensions: [&Option<ResolvedCanvasLength>; 2],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<(), Error> {
    for (method, length) in ["width", "height"].into_iter().zip(dimensions) {
        let Some(length) = length else { continue };
        let value = match length {
            ResolvedCanvasLength::Fill => "::iced::Fill".into(),
            ResolvedCanvasLength::FillPortion(portion) => {
                format!("::iced::Length::FillPortion({portion})")
            }
            ResolvedCanvasLength::Shrink => "::iced::Shrink".into(),
            ResolvedCanvasLength::Fixed { expression, source } => {
                let value = resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?;
                if *source == Type::Length {
                    value
                } else {
                    format!("{value} as f32")
                }
            }
        };
        write!(code, ".{method}({value})").unwrap();
    }
    Ok(())
}

fn canvas_capture_env(
    env: &dyn BindingEnvironment,
    captures: &[String],
    phase: &str,
) -> (HashMap<String, Binding>, String) {
    let mut captured = env.snapshot();
    let mut setup = String::new();

    for (index, scope) in captures.iter().enumerate() {
        let alias = format!("__canvas_{phase}_scope_{index}");
        write!(setup, "let {alias} = ({scope}).clone();").unwrap();
        for binding in captured.values_mut() {
            binding.code = binding.code.replace(scope, &alias);
            if let Some(StateBinding::Component {
                scope: state_scope, ..
            }) = &mut binding.state
                && state_scope == scope
            {
                *state_scope = alias.clone();
            }
        }
    }

    (captured, setup)
}

mod commands;
mod events;
mod path;
mod style;

pub(super) use commands::*;
pub(super) use events::*;
pub(super) use path::*;
pub(super) use style::*;
