use super::*;

pub(in crate::codegen) fn generate_keyboard_types(
    out: &mut String,
    document: &Document,
    subscriptions: &[ResolvedSubscription],
) {
    if !subscriptions.iter().any(|subscription| {
        matches!(
            &subscription.source,
            ResolvedSubscriptionSource::Keyboard(_)
        )
    }) && !canvas_events(document)
        .iter()
        .any(|event| matches!(event.source, SubscriptionSource::Keyboard(_)))
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

pub(in crate::codegen) fn generate_system_types(
    out: &mut String,
    document: &Document,
    subscriptions: &[ResolvedSubscription],
) {
    let information = uses_system_task(document, "__ice_system_info");
    let theme = uses_system_task(document, "__ice_system_theme")
        || subscriptions.iter().any(|subscription| {
            matches!(
                &subscription.source,
                ResolvedSubscriptionSource::SystemTheme
            )
        });
    if information {
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
    if theme {
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
}

pub(in crate::codegen) fn generate_widget_selector_types(out: &mut String, document: &Document) {
    let uses_builtin = |statements: &[Statement]| {
        statements_use_widget_selector(statements, |selector| {
            !matches!(selector, WidgetSelector::Extern { .. })
        })
    };
    if !document_uses_widget_target(document)
        && !document
            .handlers
            .iter()
            .any(|handler| uses_builtin(&handler.statements))
        && !document
            .presets
            .iter()
            .any(|preset| uses_builtin(&preset.statements))
    {
        return;
    }
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

fn document_uses_widget_target(document: &Document) -> bool {
    let uses = document_type_uses_widget_target;

    document.states.iter().any(|state| uses(&state.ty))
        || document
            .structs
            .iter()
            .flat_map(|item| &item.fields)
            .any(|(_, ty)| uses(ty))
        || document.functions.iter().any(|item| {
            item.params.iter().any(|(_, ty)| uses(ty))
                || item.progress.as_ref().is_some_and(uses)
                || uses(&item.output)
                || item.error.as_ref().is_some_and(uses)
        })
        || document
            .handlers
            .iter()
            .flat_map(|handler| &handler.params)
            .any(|param| uses(&param.ty))
        || canvases(document)
            .into_iter()
            .flat_map(|(_, locals, _)| locals)
            .any(|state| uses(&state.ty))
        || document.components.iter().any(|component| {
            component.params.iter().any(|param| uses(&param.ty))
                || uses(&component.output)
                || component.states.iter().any(|state| uses(&state.ty))
                || component
                    .handlers
                    .iter()
                    .flat_map(|handler| &handler.params)
                    .any(|param| uses(&param.ty))
        })
}

fn document_type_uses_widget_target(ty: &Type) -> bool {
    match ty {
        Type::WidgetTarget => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            document_type_uses_widget_target(inner)
        }
        Type::Result(output, error) => {
            document_type_uses_widget_target(output) || document_type_uses_widget_target(error)
        }
        _ => false,
    }
}

pub(in crate::codegen) fn statements_use_widget_selector(
    statements: &[Statement],
    predicate: impl Copy + Fn(&WidgetSelector) -> bool,
) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::WidgetOperation {
            operation: WidgetOperation::Find { selector, .. },
            ..
        } => predicate(selector),
        Statement::TaskGroup { statements, .. } => {
            statements_use_widget_selector(statements, predicate)
        }
        Statement::Abortable { task, .. } => {
            statements_use_widget_selector(::std::slice::from_ref(task), predicate)
        }
        _ => false,
    })
}

pub(in crate::codegen) fn generate_canvas_types(out: &mut String, document: &Document) {
    if !uses_canvas(document) {
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
    for group in canvas_cache_groups(document) {
        writeln!(
            out,
            "static {}: ::std::sync::OnceLock<::iced::widget::canvas::Group> = ::std::sync::OnceLock::new();",
            canvas_group_symbol(group)
        )
        .unwrap();
    }
}

pub(in crate::codegen) fn uses_system_task(document: &Document, name: &str) -> bool {
    document
        .handlers
        .iter()
        .any(|handler| statements_use_system_task(&handler.statements, name))
        || document
            .presets
            .iter()
            .any(|preset| statements_use_system_task(&preset.statements, name))
}

pub(in crate::codegen) fn statements_use_system_task(statements: &[Statement], name: &str) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Run {
            kind: EffectKind::Task,
            function,
            ..
        } => function == name,
        Statement::TaskGroup { statements, .. } => statements_use_system_task(statements, name),
        Statement::Abortable { task, .. } => {
            statements_use_system_task(::std::slice::from_ref(task), name)
        }
        Statement::TaskFlow {
            source, transforms, ..
        } => {
            task_source_uses_system(source, name)
                || transforms.iter().any(|transform| match transform {
                    TaskTransform::Then { source, .. } | TaskTransform::AndThen { source, .. } => {
                        task_source_uses_system(source, name)
                    }
                    TaskTransform::Map { .. }
                    | TaskTransform::MapError { .. }
                    | TaskTransform::Collect { .. }
                    | TaskTransform::Discard { .. } => false,
                })
        }
        _ => false,
    })
}

pub(in crate::codegen) fn task_source_uses_system(source: &TaskSource, name: &str) -> bool {
    matches!(source, TaskSource::Effect { function, .. } if function == name)
}

pub(in crate::codegen) fn borrowed_type(ty: &Type, document: &Document) -> String {
    match ty {
        Type::Str => "&'a str".into(),
        Type::Bytes => "&'a [u8]".into(),
        Type::List(inner) => format!("&'a [{}]", inner.rust(&document.structs)),
        _ => format!("&'a {}", ty.rust(&document.structs)),
    }
}
