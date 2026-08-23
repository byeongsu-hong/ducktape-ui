use super::*;

pub(in crate::codegen) fn generate_keyboard_types(
    out: &mut String,
    program: &LoweredProgram,
    subscriptions: &[ResolvedSubscription],
) {
    if !subscriptions.iter().any(|subscription| {
        matches!(
            &subscription.source,
            ResolvedSubscriptionSource::Keyboard(_)
        )
    }) && !canvas_events(program)
        .iter()
        .any(|event| matches!(event.source, ResolvedCanvasEventSource::Keyboard(_)))
    {
        return;
    }
    out.push_str(
        r#"#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct __IceKeyPress {
    key: ::iced::keyboard::Key,
    modified_key: ::iced::keyboard::Key,
    physical_key: ::iced::keyboard::key::Physical,
    location: ::iced::keyboard::Location,
    modifiers: ::iced::keyboard::Modifiers,
    text: ::std::option::Option<::std::string::String>,
    repeat: bool,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct __IceKeyRelease {
    key: ::iced::keyboard::Key,
    modified_key: ::iced::keyboard::Key,
    physical_key: ::iced::keyboard::key::Physical,
    location: ::iced::keyboard::Location,
    modifiers: ::iced::keyboard::Modifiers,
}
"#,
    );
}

fn statements_contain(
    statements: &[ResolvedStatement],
    predicate: &impl Fn(&ResolvedStatement) -> bool,
) -> bool {
    statements.iter().any(|statement| {
        predicate(statement)
            || match &statement.kind {
                ResolvedStatementKind::TaskGroup { statements, .. } => {
                    statements_contain(statements, predicate)
                }
                ResolvedStatementKind::Match { arms, .. } => arms
                    .iter()
                    .any(|arm| statements_contain(&arm.statements, predicate)),
                ResolvedStatementKind::Abortable { task, .. } => {
                    statements_contain(std::slice::from_ref(task), predicate)
                }
                _ => false,
            }
    })
}

fn program_statements_contain(
    program: &LoweredProgram,
    predicate: impl Fn(&ResolvedStatement) -> bool,
) -> bool {
    program
        .handlers()
        .iter()
        .any(|handler| statements_contain(&handler.statements, &predicate))
}

fn source_uses_builtin(source: &ResolvedTaskSource, expected: &str) -> bool {
    matches!(
        source,
        ResolvedTaskSource::Effect {
            target: ResolvedEffectTarget::Builtin(target),
            ..
        } if target == expected
    )
}

fn program_uses_builtin_effect(program: &LoweredProgram, expected: &str) -> bool {
    program_statements_contain(program, |statement| match &statement.kind {
        ResolvedStatementKind::Run(run) => {
            matches!(&run.target, ResolvedEffectTarget::Builtin(target) if target == expected)
        }
        ResolvedStatementKind::TaskFlow(flow) => {
            source_uses_builtin(&flow.source, expected)
                || flow.transforms.iter().any(|transform| match transform {
                    ResolvedTaskTransform::Then { source, .. }
                    | ResolvedTaskTransform::AndThen { source, .. } => {
                        source_uses_builtin(source, expected)
                    }
                    _ => false,
                })
        }
        _ => false,
    })
}

fn program_uses_widget_selector(program: &LoweredProgram) -> bool {
    program_statements_contain(program, |statement| {
        matches!(
            &statement.kind,
            ResolvedStatementKind::WidgetOperation {
                operation: ResolvedWidgetOperation::Find { .. },
                ..
            }
        )
    })
}

pub(in crate::codegen) fn generate_system_types(out: &mut String, program: &LoweredProgram) {
    out.push_str(
        r#"#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct __IceSystemInfo {
    system_name: ::std::option::Option<::std::string::String>,
    system_kernel: ::std::option::Option<::std::string::String>,
    system_version: ::std::option::Option<::std::string::String>,
    system_short_version: ::std::option::Option<::std::string::String>,
    cpu_brand: ::std::string::String,
    cpu_cores: ::std::option::Option<i64>,
    memory_total: i64,
    memory_used: ::std::option::Option<i64>,
    graphics_backend: ::std::string::String,
    graphics_adapter: ::std::string::String,
}
"#,
    );
    if program_uses_builtin_effect(program, "__ice_system_info") {
        out.push_str(
            r#"
fn __ice_system_info(value: ::iced::system::Information) -> __IceSystemInfo {
    __IceSystemInfo {
        system_name: value.system_name,
        system_kernel: value.system_kernel,
        system_version: value.system_version,
        system_short_version: value.system_short_version,
        cpu_brand: value.cpu_brand,
        cpu_cores: value.cpu_cores.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        memory_total: i64::try_from(value.memory_total).unwrap_or(i64::MAX),
        memory_used: value.memory_used.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        graphics_backend: value.graphics_backend,
        graphics_adapter: value.graphics_adapter,
    }
}
"#,
        );
    }
    out.push_str(
        r#"fn __ice_system_theme(value: ::iced::theme::Mode) -> ::std::string::String {
    match value {
        ::iced::theme::Mode::None => "none",
        ::iced::theme::Mode::Light => "light",
        ::iced::theme::Mode::Dark => "dark",
    }.to_owned()
}
"#,
    );
}

pub(in crate::codegen) fn generate_widget_selector_types(
    out: &mut String,
    program: &LoweredProgram,
) {
    out.push_str(
        r#"#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct __IceWidgetTarget {
    kind: ::std::string::String,
    id: ::std::option::Option<::iced::widget::Id>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    visible_x: ::std::option::Option<f64>,
    visible_y: ::std::option::Option<f64>,
    visible_width: ::std::option::Option<f64>,
    visible_height: ::std::option::Option<f64>,
    content: ::std::option::Option<::std::string::String>,
    content_x: ::std::option::Option<f64>,
    content_y: ::std::option::Option<f64>,
    content_width: ::std::option::Option<f64>,
    content_height: ::std::option::Option<f64>,
    translation_x: ::std::option::Option<f64>,
    translation_y: ::std::option::Option<f64>,
}
"#,
    );
    if program_uses_widget_selector(program) {
        out.push_str(
            r#"
fn __ice_widget_target(
    kind: &str,
    id: ::std::option::Option<::iced::widget::Id>,
    bounds: ::iced::Rectangle,
    visible: ::std::option::Option<::iced::Rectangle>,
    content: ::std::option::Option<::std::string::String>,
    content_bounds: ::std::option::Option<::iced::Rectangle>,
    translation: ::std::option::Option<::iced::Vector>,
) -> __IceWidgetTarget {
    __IceWidgetTarget {
        kind: kind.to_owned(),
        id,
        x: bounds.x as f64,
        y: bounds.y as f64,
        width: bounds.width as f64,
        height: bounds.height as f64,
        visible_x: visible.map(|bounds| bounds.x as f64),
        visible_y: visible.map(|bounds| bounds.y as f64),
        visible_width: visible.map(|bounds| bounds.width as f64),
        visible_height: visible.map(|bounds| bounds.height as f64),
        content,
        content_x: content_bounds.map(|bounds| bounds.x as f64),
        content_y: content_bounds.map(|bounds| bounds.y as f64),
        content_width: content_bounds.map(|bounds| bounds.width as f64),
        content_height: content_bounds.map(|bounds| bounds.height as f64),
        translation_x: translation.map(|translation| translation.x as f64),
        translation_y: translation.map(|translation| translation.y as f64),
    }
}
fn __ice_widget_target_from_target(value: ::iced::widget::selector::Target) -> __IceWidgetTarget {
    use ::iced::widget::selector::Target;
    match value {
        Target::Container { id, bounds, visible_bounds } => __ice_widget_target("container", id, bounds, visible_bounds, None, None, None),
        Target::Focusable { id, bounds, visible_bounds } => __ice_widget_target("focusable", id, bounds, visible_bounds, None, None, None),
        Target::Scrollable { id, bounds, visible_bounds, content_bounds, translation } => __ice_widget_target("scrollable", id, bounds, visible_bounds, None, Some(content_bounds), Some(translation)),
        Target::TextInput { id, bounds, visible_bounds, content } => __ice_widget_target("text-input", id, bounds, visible_bounds, Some(content), None, None),
        Target::Text { id, bounds, visible_bounds, content } => __ice_widget_target("text", id, bounds, visible_bounds, Some(content), None, None),
        Target::Custom { id, bounds, visible_bounds } => __ice_widget_target("custom", id, bounds, visible_bounds, None, None, None),
    }
}
fn __ice_widget_target_from_text(value: ::iced::widget::selector::Text) -> __IceWidgetTarget {
    use ::iced::widget::selector::Text;
    match value {
        Text::Raw { id, bounds, visible_bounds } => __ice_widget_target("text", id, bounds, visible_bounds, None, None, None),
        Text::Input { id, bounds, visible_bounds } => __ice_widget_target("text-input", id, bounds, visible_bounds, None, None, None),
    }
}
"#,
        );
    }
}

pub(in crate::codegen) fn generate_canvas_types(out: &mut String, program: &LoweredProgram) {
    if !uses_canvas(program) {
        return;
    }
    out.push_str(
        r#"struct __IceCanvasProgram<State, Message, Draw, Update, Interaction> {
    draw: Draw,
    update: Update,
    interaction: Interaction,
    message: ::std::marker::PhantomData<fn() -> (State, Message)>,
}
impl<State, Message, Draw, Update, Interaction> ::iced::widget::canvas::Program<Message>
    for __IceCanvasProgram<State, Message, Draw, Update, Interaction>
where
    State: ::std::default::Default + 'static,
    Draw: Fn(
        &State,
        &::iced::Renderer,
        &::iced::Theme,
        ::iced::Rectangle,
        ::iced::mouse::Cursor,
    ) -> ::std::vec::Vec<::iced::widget::canvas::Geometry>,
    Update: Fn(
        &mut State,
        &::iced::widget::canvas::Event,
        ::iced::Rectangle,
        ::iced::mouse::Cursor,
    ) -> ::std::option::Option<::iced::widget::canvas::Action<Message>>,
    Interaction: Fn(
        &State,
        ::iced::Rectangle,
        ::iced::mouse::Cursor,
    ) -> ::iced::mouse::Interaction,
{
    type State = State;

    fn update(
        &self,
        state: &mut Self::State,
        event: &::iced::widget::canvas::Event,
        bounds: ::iced::Rectangle,
        cursor: ::iced::mouse::Cursor,
    ) -> ::std::option::Option<::iced::widget::canvas::Action<Message>> {
        (self.update)(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &::iced::Renderer,
        theme: &::iced::Theme,
        bounds: ::iced::Rectangle,
        cursor: ::iced::mouse::Cursor,
    ) -> ::std::vec::Vec<::iced::widget::canvas::Geometry> {
        (self.draw)(state, renderer, theme, bounds, cursor)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: ::iced::Rectangle,
        cursor: ::iced::mouse::Cursor,
    ) -> ::iced::mouse::Interaction {
        (self.interaction)(state, bounds, cursor)
    }
}
fn __ice_canvas_interaction(value: &str) -> ::iced::mouse::Interaction {
    match value {
        "hidden" => ::iced::mouse::Interaction::Hidden,
        "idle" => ::iced::mouse::Interaction::Idle,
        "context-menu" => ::iced::mouse::Interaction::ContextMenu,
        "help" => ::iced::mouse::Interaction::Help,
        "pointer" => ::iced::mouse::Interaction::Pointer,
        "progress" => ::iced::mouse::Interaction::Progress,
        "wait" => ::iced::mouse::Interaction::Wait,
        "cell" => ::iced::mouse::Interaction::Cell,
        "crosshair" => ::iced::mouse::Interaction::Crosshair,
        "text" => ::iced::mouse::Interaction::Text,
        "alias" => ::iced::mouse::Interaction::Alias,
        "copy" => ::iced::mouse::Interaction::Copy,
        "move" => ::iced::mouse::Interaction::Move,
        "no-drop" => ::iced::mouse::Interaction::NoDrop,
        "not-allowed" => ::iced::mouse::Interaction::NotAllowed,
        "grab" => ::iced::mouse::Interaction::Grab,
        "grabbing" => ::iced::mouse::Interaction::Grabbing,
        "resize-horizontal" => ::iced::mouse::Interaction::ResizingHorizontally,
        "resize-vertical" => ::iced::mouse::Interaction::ResizingVertically,
        "resize-diagonal-up" => ::iced::mouse::Interaction::ResizingDiagonallyUp,
        "resize-diagonal-down" => ::iced::mouse::Interaction::ResizingDiagonallyDown,
        "resize-column" => ::iced::mouse::Interaction::ResizingColumn,
        "resize-row" => ::iced::mouse::Interaction::ResizingRow,
        "all-scroll" => ::iced::mouse::Interaction::AllScroll,
        "zoom-in" => ::iced::mouse::Interaction::ZoomIn,
        "zoom-out" => ::iced::mouse::Interaction::ZoomOut,
        _ => ::iced::mouse::Interaction::default(),
    }
}
"#,
    );
    for group in canvas_cache_groups(program) {
        writeln!(
            out,
            "static {}: ::std::sync::OnceLock<::iced::widget::canvas::Group> = ::std::sync::OnceLock::new();",
            canvas_group_symbol(group)
        )
        .unwrap();
    }
}

/// The argument expression handed to a `&`-declared extern parameter. `AsRef`
/// and `Borrow` accept the binding whether it is the owned value or already a
/// reference (a `for` row, a lazy alias), so the emitter needs no per-scope
/// knowledge of how the value is held.
pub(in crate::codegen) fn borrowed_argument_code(ty: &Type, code: &str) -> String {
    match ty {
        Type::Str | Type::Bytes | Type::List(_) => {
            format!("::std::convert::AsRef::as_ref(&({code}))")
        }
        _ => format!("::std::borrow::Borrow::borrow(&({code}))"),
    }
}

pub(in crate::codegen) fn borrowed_type(ty: &Type, program: &LoweredProgram) -> String {
    match ty {
        Type::Str => "&'a str".into(),
        Type::Bytes => "&'a [u8]".into(),
        Type::List(inner) => format!("&'a [{}]", rust_type_code(program, inner)),
        _ => format!("&'a {}", rust_type_code(program, ty)),
    }
}
