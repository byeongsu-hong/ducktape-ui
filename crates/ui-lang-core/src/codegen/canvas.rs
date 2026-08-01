use super::*;

pub(in crate::codegen) fn render_canvas(
    options: &CanvasOptions,
    locals: &[State],
    commands: &[CanvasCommand],
    events: &[CanvasEvent],
    document: &Document,
    message: &str,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let state_fields = locals
        .iter()
        .map(|local| format!("{}: {},", local.name, local.ty.rust(&document.structs)))
        .collect::<Vec<_>>()
        .join(" ");
    let state_initials = locals
        .iter()
        .map(|local| {
            Ok(format!(
                "{}: {},",
                local.name,
                canvas_initial_code(local, document)?
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(" ");
    let mut canvas_env = env.snapshot();
    for local in locals {
        canvas_env.insert(
            local.name.clone(),
            Binding {
                code: format!("__state.{}", local.name),
                ty: local.ty.clone(),
                local: false,
                state: None,
                owner: None,
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
            owner: None,
        },
    );
    canvas_env.insert(
        "canvas_height".into(),
        Binding {
            code: "(__bounds.height as f64)".into(),
            ty: Type::F64,
            local: true,
            state: None,
            owner: None,
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
    let draw_commands = canvas_commands_code(commands, &draw_env, document)?;
    let use_cache = options.cache.is_some();
    let cache_key = if let Some(dependency) = &options.cache {
        let dependency = expr_code(dependency, env, document, ValueMode::Owned)?;
        format!(
            "::std::option::Option::Some({{ let mut __hasher = ::std::hash::DefaultHasher::new(); ::std::hash::Hash::hash(&(__ice_palette.name, {dependency}), &mut __hasher); ::std::hash::Hasher::finish(&__hasher) }})"
        )
    } else {
        "::std::option::Option::None".into()
    };
    let update = canvas_update_code(
        options,
        events,
        &update_env,
        &canvas_update_env,
        document,
        message,
        use_cache,
    )?;
    let interaction = if let Some(value) = &options.interaction_expr {
        let interaction = expr_code(value, &interaction_env, document, ValueMode::Owned)?;
        if expr_type(
            value,
            &env_types(&interaction_env),
            document,
            &Span::line(1),
        )? == Type::MouseInteraction
        {
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
        .map(|outside| expr_code(outside, &interaction_env, document, ValueMode::Owned))
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
#[allow(dead_code)]
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
    append_dimensions(&mut code, [&options.width, &options.height], env, document)?;
    code.push_str("; __canvas.into() }");
    Ok(code)
}

// Canvas-local state belongs to the still AST-backed view-expression slice.
// Unlike the former shared initializer fallback, emission is checked and fallible.
fn canvas_initial_code(state: &State, document: &Document) -> Result<String, Error> {
    if matches!(state.ty, Type::Animation(_)) {
        return Err(Error::new(
            "E196",
            &state.span,
            "canvas-local animation passed semantic checking",
        ));
    }
    Ok(match (&state.initial, &state.ty) {
        (Expr::Str(value), Type::Markdown) => format!(
            "::iced::widget::markdown::Content::parse({})",
            rust_string(value)
        ),
        (Expr::Str(value), Type::Editor) => format!(
            "::iced::widget::text_editor::Content::with_text({})",
            rust_string(value)
        ),
        (Expr::EmptyList, Type::Combo(_)) => {
            "::iced::widget::combo_box::State::new(::std::vec::Vec::new())".into()
        }
        (Expr::List(values), Type::Combo(_)) => format!(
            "::iced::widget::combo_box::State::new(::std::vec![{}])",
            expr_list_code(values, &HashMap::new(), document)?
        ),
        _ => expr_code(&state.initial, &HashMap::new(), document, ValueMode::Owned)?,
    })
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
