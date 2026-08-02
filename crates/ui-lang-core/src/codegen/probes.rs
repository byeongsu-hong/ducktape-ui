use super::*;

fn style_probe(
    kind: ExternKind,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    Some(match kind {
        ExternKind::EditorStyle => (
            "editor_style",
            "theme: &::iced::Theme, status: ::iced::widget::text_editor::Status",
            "theme, status",
            "::iced::widget::text_editor::Style",
        ),
        ExternKind::TextStyle => (
            "text_style",
            "theme: &::iced::Theme",
            "theme",
            "::iced::widget::text::Style",
        ),
        ExternKind::SliderStyle => (
            "slider_style",
            "theme: &::iced::Theme, status: ::iced::widget::slider::Status",
            "theme, status",
            "::iced::widget::slider::Style",
        ),
        ExternKind::ProgressStyle => (
            "progress_style",
            "theme: &::iced::Theme",
            "theme",
            "::iced::widget::progress_bar::Style",
        ),
        ExternKind::ButtonStyle => (
            "button_style",
            "theme: &::iced::Theme, status: ::iced::widget::button::Status",
            "theme, status",
            "::iced::widget::button::Style",
        ),
        ExternKind::CheckboxStyle => (
            "checkbox_style",
            "theme: &::iced::Theme, status: ::iced::widget::checkbox::Status",
            "theme, status",
            "::iced::widget::checkbox::Style",
        ),
        ExternKind::TogglerStyle => (
            "toggler_style",
            "theme: &::iced::Theme, status: ::iced::widget::toggler::Status",
            "theme, status",
            "::iced::widget::toggler::Style",
        ),
        ExternKind::RadioStyle => (
            "radio_style",
            "theme: &::iced::Theme, status: ::iced::widget::radio::Status",
            "theme, status",
            "::iced::widget::radio::Style",
        ),
        ExternKind::ContainerStyle => (
            "container_style",
            "theme: &::iced::Theme",
            "theme",
            "::iced::widget::container::Style",
        ),
        ExternKind::SvgStyle => (
            "svg_style",
            "theme: &::iced::Theme, status: ::iced::widget::svg::Status",
            "theme, status",
            "::iced::widget::svg::Style",
        ),
        ExternKind::InputStyle => (
            "input_style",
            "theme: &::iced::Theme, status: ::iced::widget::text_input::Status",
            "theme, status",
            "::iced::widget::text_input::Style",
        ),
        ExternKind::ScrollStyle => (
            "scroll_style",
            "theme: &::iced::Theme, status: ::iced::widget::scrollable::Status",
            "theme, status",
            "::iced::widget::scrollable::Style",
        ),
        ExternKind::PickListStyle => (
            "pick_list_style",
            "theme: &::iced::Theme, status: ::iced::widget::pick_list::Status",
            "theme, status",
            "::iced::widget::pick_list::Style",
        ),
        ExternKind::MenuStyle => (
            "menu_style",
            "theme: &::iced::Theme",
            "theme",
            "::iced::overlay::menu::Style",
        ),
        ExternKind::PaneGridStyle => (
            "pane_grid_style",
            "theme: &::iced::Theme",
            "theme",
            "::iced::widget::pane_grid::Style",
        ),
        _ => return None,
    })
}

pub(in crate::codegen) fn generate_extern_probes(
    out: &mut String,
    program: &LoweredProgram,
    component_declarations: &[ResolvedExternComponentDeclaration],
    component_ids: &HashSet<ExternFnId>,
) {
    if program.extern_functions().any(|item| {
        !component_ids.contains(&item.declaration.id) && item.kind == ExternKind::EventFilter
    }) {
        writeln!(out, "#[cfg(not(target_arch = \"wasm32\"))] type __IceEventStream<T> = ::iced::futures::stream::BoxStream<'static, T>; #[cfg(target_arch = \"wasm32\")] type __IceEventStream<T> = ::iced::futures::stream::LocalBoxStream<'static, T>;").unwrap();
    }
    for item in program.struct_declarations() {
        writeln!(
            out,
            "{}",
            source_marker_for_origin(program, item.declaration.origin)
        )
        .unwrap();
        writeln!(
            out,
            "#[allow(dead_code, non_snake_case)] fn __ui_lang_check_{}(_value: &{}) {{",
            item.name, item.rust_path
        )
        .unwrap();
        for field in &item.fields {
            writeln!(
                out,
                "let _: &{} = &_value.{};",
                rust_type_code(program, &field.ty),
                field.name
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    for item in program.extern_functions() {
        if component_ids.contains(&item.declaration.id) {
            continue;
        }
        writeln!(
            out,
            "{}",
            source_marker_for_origin(program, item.declaration.origin)
        )
        .unwrap();
        let params = item
            .params
            .iter()
            .enumerate()
            .map(|(index, (_, ty))| {
                let ty = if item.borrowed[index] {
                    borrowed_type(ty, program)
                } else {
                    rust_type_code(program, ty)
                };
                format!("arg{index}: {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let args = (0..item.params.len())
            .map(|index| format!("arg{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let output = item.error.as_ref().map_or_else(
            || rust_type_code(program, &item.output),
            |error| {
                format!(
                    "::std::result::Result<{}, {}>",
                    rust_type_code(program, &item.output),
                    rust_type_code(program, error)
                )
            },
        );
        if let Some((name, leading_params, leading_args, style)) = style_probe(item.kind) {
            let params = if params.is_empty() {
                leading_params.to_owned()
            } else {
                format!("{leading_params}, {params}")
            };
            let args = if args.is_empty() {
                leading_args.to_owned()
            } else {
                format!("{leading_args}, {args}")
            };
            writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_{name}_{}({params}) {{ let _: {style} = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap();
            writeln!(out, "{SOURCE_MARKER_END}").unwrap();
            continue;
        }
        match item.kind {
            ExternKind::Future => writeln!(
                out,
                "#[allow(dead_code)] async fn __ui_lang_check_future_{}({params}) {{ let _: {output} = {}({args}).await; }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Shader => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_shader_{}({params}) {{ let __program = {}({args}); fn __accept<P: ::iced::widget::shader::Program<{output}>>(_: &P) {{}} __accept(&__program); let _: __IceElement<'static, {output}> = ::iced::widget::Shader::new(__program).into(); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Task => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_task_{}({params}) {{ let _: ::iced::Task<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Stream => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_stream_{}({params}) {{ let _: ::iced::Task<{output}> = ::iced::Task::run({}({args}), |value| value); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Sip => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_sip_{}({params}) {{ let _: ::iced::Task<()> = ::iced::Task::sip({}({args}), |value| {{ let _: {} = value; }}, |value| {{ let _: {output} = value; }}); }}",
                item.name,
                item.rust_path,
                rust_type_code(
                    program,
                    item.progress
                        .as_ref()
                        .expect("sip extern has a progress type")
                )
            )
            .unwrap(),
            ExternKind::Recipe => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_recipe_{}({params}) {{ let __recipe = {}({args}); fn __accept<R: ::iced::advanced::subscription::Recipe<Output = {output}>>(_: &R) {{}} __accept(&__recipe); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Selector => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_selector_{}({params}) {{ let _: ::iced::Task<::std::option::Option<{output}>> = ::iced::widget::selector::find({}({args})); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::EventFilter => {
                let recipe = event_filter_type(&item.name);
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_event_filter_{}() {{ let _: fn(::iced::advanced::subscription::Event) -> ::std::option::Option<{output}> = {}; }}",
                    item.name, item.rust_path
                )
                .unwrap();
                writeln!(
                    out,
                    "struct {recipe}<I> {{ id: I }} impl<I: ::std::hash::Hash + 'static> ::iced::advanced::subscription::Recipe for {recipe}<I> {{ type Output = {output}; fn hash(&self, state: &mut ::iced::advanced::subscription::Hasher) {{ ::std::hash::Hash::hash(&::std::any::TypeId::of::<Self>(), state); ::std::hash::Hash::hash(&self.id, state); }} fn stream(self: ::std::boxed::Box<Self>, input: ::iced::advanced::subscription::EventStream) -> __IceEventStream<Self::Output> {{ ::std::boxed::Box::pin(::iced::futures::StreamExt::filter_map(input, |event| ::iced::futures::future::ready({}(event)))) }} }}",
                    item.rust_path
                )
                .unwrap();
            }
            ExternKind::Sync => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_sync_{}({params}) {{ let _: {output} = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Subscription => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_subscription_{}({params}) {{ let _: ::iced::Subscription<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Theme => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_theme_{}({params}) {{ let _: ::iced::Theme = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Themer => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_themer_{}({params}) {{ let (__theme, __content, __text_color, __background) = {}({args}); fn __accept<T: ::iced::theme::Base>(_: &::std::option::Option<T>, _: &__IceElement<'static, {output}, T>, _: &::std::option::Option<fn(&T) -> ::iced::Color>, _: &::std::option::Option<fn(&T) -> ::iced::Background>) {{}} __accept(&__theme, &__content, &__text_color, &__background); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Window => {
                let params = if params.is_empty() {
                    "window: &dyn ::iced::window::Window".into()
                } else {
                    format!("window: &dyn ::iced::window::Window, {params}")
                };
                let args = if args.is_empty() {
                    "window".into()
                } else {
                    format!("window, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_window_{}({params}) {{ let _: {output} = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::MarkdownViewer => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_markdown_viewer_{}({params}) {{ let __viewer = {}({args}); fn __accept<V>(_: &V) where for<'a> V: ::iced::widget::markdown::Viewer<'a, {output}, ::iced::Theme, ::iced::Renderer> {{}} __accept(&__viewer); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::EditorBinding => {
                let callback_params = std::iter::once(
                    "::iced::widget::text_editor::KeyPress".to_owned(),
                )
                .chain(
                    item.params
                        .iter()
                        .map(|(_, ty)| rust_type_code(program, ty)),
                )
                .collect::<Vec<_>>()
                .join(", ");
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_editor_binding_{}() {{ let _: fn({callback_params}) -> ::std::option::Option<::iced::widget::text_editor::Binding<{output}>> = {}; }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::EditorAction => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_editor_action_{}() {{ let _: fn(&mut ::iced::widget::text_editor::Content, ::iced::widget::text_editor::Action) = {}; }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::EditorHighlighter => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_editor_highlighter_{}({params}) {{ let __content = ::iced::widget::text_editor::Content::new(); let __editor = ::iced::widget::text_editor(&__content).on_action(|_| ()); let _: __IceElement<'_, ()> = {}(__editor{}).into(); }}",
                item.name,
                item.rust_path,
                if args.is_empty() {
                    String::new()
                } else {
                    format!(", {args}")
                }
            )
            .unwrap(),
            _ => unreachable!("style probes returned above"),
        }
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }

    for declaration in component_declarations {
        writeln!(
            out,
            "{}",
            source_marker_for_origin(program, declaration.origin)
        )
        .unwrap();
        let borrowed = declaration
            .parameters
            .iter()
            .any(|parameter| parameter.mode != ResolvedExternComponentArgumentMode::Owned);
        let params = declaration
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let ty = match parameter.mode {
                    ResolvedExternComponentArgumentMode::Owned => {
                        rust_type_code(program, &parameter.ty)
                    }
                    ResolvedExternComponentArgumentMode::BorrowedAsRef
                    | ResolvedExternComponentArgumentMode::Borrowed => {
                        borrowed_type(&parameter.ty, program)
                    }
                };
                format!("arg{index}: {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let args = (0..declaration.parameters.len())
            .map(|index| format!("arg{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "#[allow(dead_code)] fn __ui_lang_check_component_{}{}({params}) {{ let _: __IceElement<'{}, {}> = {}({args}); }}",
            declaration.name,
            if borrowed { "<'a>" } else { "" },
            if borrowed { "a" } else { "static" },
            rust_type_code(program, &declaration.output),
            declaration.rust_path
        )
        .unwrap();
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
}

pub(in crate::codegen) fn generate_editor_binding_mapper(
    out: &mut String,
    program: &LoweredProgram,
    component_ids: &HashSet<ExternFnId>,
) {
    if !program.extern_functions().any(|item| {
        !component_ids.contains(&item.declaration.id) && item.kind == ExternKind::EditorBinding
    }) {
        return;
    }
    writeln!(
        out,
        "fn __ice_map_editor_binding<T, M>(binding: ::iced::widget::text_editor::Binding<T>, custom: &impl Fn(T) -> M) -> ::iced::widget::text_editor::Binding<M> {{ use ::iced::widget::text_editor::Binding; match binding {{ Binding::Unfocus => Binding::Unfocus, Binding::Copy => Binding::Copy, Binding::Cut => Binding::Cut, Binding::Paste => Binding::Paste, Binding::Move(value) => Binding::Move(value), Binding::Select(value) => Binding::Select(value), Binding::SelectWord => Binding::SelectWord, Binding::SelectLine => Binding::SelectLine, Binding::SelectAll => Binding::SelectAll, Binding::Insert(value) => Binding::Insert(value), Binding::Enter => Binding::Enter, Binding::Backspace => Binding::Backspace, Binding::Delete => Binding::Delete, Binding::Sequence(values) => Binding::Sequence(values.into_iter().map(|value| __ice_map_editor_binding(value, custom)).collect()), Binding::Custom(value) => Binding::Custom(custom(value)), }} }}"
    )
    .unwrap();
}
