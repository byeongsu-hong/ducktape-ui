use super::*;

/// How many slot expressions one generated method carries.
///
/// The slot table grows with the whole screen, and rustc's type and borrow
/// checking are superlinear in the size of a single function — the same reason
/// `codegen::view::outline` exists for component uses. Splitting the table
/// across methods keeps each one small; the constant is a balance between that
/// and the per-call overhead of many tiny methods.
const SLOTS_PER_METHOD: usize = 32;

fn app_view_env(program: &LoweredProgram) -> HashMap<String, Binding> {
    let daemon = program.settings().kind == ProgramKind::Daemon;
    let mut env = checked_state_env(program, "self");
    if daemon {
        env.insert(
            "window".into(),
            Binding {
                code: "window".into(),
                ty: Type::WindowId,
                local: true,
                state: None,
                owner: program
                    .expressions()
                    .daemon_window_local()
                    .map(BindingOwner::Local),
            },
        );
    }
    env
}

/// Renders a published template: the process resolves its current template —
/// the embedded one, or the file `ICE_TEMPLATE_PATH` names — then fills this
/// frame's slot table and hands both to the runtime renderer.
///
/// The slot expressions are emitted as sibling methods appended to `methods`,
/// not inline, so a large view does not become one enormous function body.
fn template_render_code(
    emission: &crate::codegen::template::TemplateEmission,
    message: &str,
    root_scope: &str,
    daemon: bool,
    methods: &mut String,
) -> String {
    // A daemon's view is per-window, and its slot expressions may read the
    // window it is rendering for, so the parameter travels with them.
    let (window_param, window_arg) = if daemon {
        (", window: ::iced::window::Id", ", window")
    } else {
        ("", "")
    };
    let paths = emission
        .paths
        .iter()
        .map(|path| format!("{},", rust_string(path)))
        .collect::<String>();
    let mut calls = String::new();
    for (index, chunk) in emission.slots.chunks(SLOTS_PER_METHOD).enumerate() {
        let pushes = chunk.concat();
        writeln!(
            methods,
            "pub(super) fn __ice_slots_{index}<'a>(&'a self, __ice_palette: __IcePalette, __ice_app_theme: &::iced::Theme{window_param}, __ice_slots: &mut ::ui_lang_runtime::template::Slots<'a, {message}>) {{ let __ice_app_theme = __ice_app_theme.clone(); let _ = &__ice_app_theme; {pushes} }}",
        )
        .unwrap();
        writeln!(
            calls,
            "self.__ice_slots_{index}(__ice_palette, &__ice_app_theme{window_arg}, &mut __ice_slots);"
        )
        .unwrap();
    }
    format!(
        "{{ \
         static __ICE_TEMPLATE_JSON: &str = {json}; \
         static __ICE_TEMPLATE_PATHS: [&str; {path_count}] = [{paths}]; \
         thread_local! {{ static __ICE_TEMPLATE: ::ui_lang_runtime::template::TemplateSource = \
         ::ui_lang_runtime::template::TemplateSource::new(__ICE_TEMPLATE_JSON); }} \
         let __ice_template = __ICE_TEMPLATE.with(|source| source.current()); \
         let mut __ice_slots: ::ui_lang_runtime::template::Slots<'_, {message}> = \
         ::ui_lang_runtime::template::Slots::with_capacity({counts}); {calls} \
         ::ui_lang_runtime::template::render(&__ice_template, &__ice_slots, &__ice_palette.colors, {root_scope}, &__ICE_TEMPLATE_PATHS) \
         }}",
        json = rust_string(&emission.json),
        counts = slot_counts_code(&emission.counts),
        path_count = emission.paths.len(),
    )
}

/// The `SlotCounts` literal the generated view sizes its tables from.
fn slot_counts_code(counts: &ui_lang_template::SlotCounts) -> String {
    format!(
        "::ui_lang_runtime::template::SlotCounts {{ texts: {}, states: {}, messages: {}, handlers: {}, subtrees: {}, groups: {}, bools: {} }}",
        counts.texts,
        counts.states,
        counts.messages,
        counts.handlers,
        counts.subtrees,
        counts.groups,
        counts.bools,
    )
}

pub(in crate::codegen) fn generate_view(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    let daemon = program.settings().kind == ProgramKind::Daemon;
    let mounted = mounted_component_fields(program);
    let env = app_view_env(program);
    let root_scope = if mounted.is_empty() {
        rust_string(program.app_name())
    } else {
        "__ice_root_scope_ref".into()
    };
    // A view the template vocabulary covers is published as data and rendered
    // by the runtime; anything else keeps its compiled tree.
    let mut slot_methods = String::new();
    let rendered_root =
        match crate::codegen::template::emit(program, message, &env, source_path, &root_scope)? {
            Some(emission) => {
                template_render_code(&emission, message, &root_scope, daemon, &mut slot_methods)
            }
            None => render_node_if_present(
                program.app_view(),
                program,
                message,
                &env,
                &root_scope,
                None,
            )?
            .unwrap_or_else(|| "::iced::widget::Column::new().into()".into()),
        };
    let window_arg = if daemon {
        ", window: ::iced::window::Id"
    } else {
        ""
    };
    let callback_value = if daemon { "window" } else { "" };
    let palette = format!(
        "let __ice_palette = self.__palette({callback_value}); let __ice_app_theme = Self::__app_theme(__ice_palette);"
    );
    let navigation = navigation_code(message, daemon);
    if mounted.is_empty() {
        writeln!(
            out,
            "pub(super) fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_content: __IceElement<'_, {message}> = {rendered_root}; let __ice_root: __IceElement<'_, {message}> = {navigation}; ::ui_lang_runtime::dev::ready(__ice_root) }}"
        )
        .unwrap();
    } else {
        let root_scope_init = root_scope_code(program, "window");
        let begin = mounted
            .iter()
            .map(|field| format!("self.{field}.begin_render();"))
            .collect::<String>();
        let finish = mounted
            .iter()
            .map(|field| format!("self.{field}.finish_render(__ice_root_scope_ref);"))
            .collect::<String>();
        let result = &navigation;
        let (boot_drain, boot_wrap) = boot_dispatch_code(program, message);
        writeln!(
            out,
            "pub(super) fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_root_scope = {root_scope_init}; let __ice_root_scope_ref = __ice_root_scope.as_str(); {begin} let __ice_content: __IceElement<'_, {message}> = {rendered_root}; {finish} {boot_drain} let __ice_root: __IceElement<'_, {message}> = {result}; {boot_wrap} ::ui_lang_runtime::dev::ready(__ice_root) }}"
        )
        .unwrap();
    }
    out.push_str(&slot_methods);
    Ok(())
}

/// The root wrapper that turns Tab into focus traversal. A daemon's window is
/// a ring of its own: the wrapper names its window and the focus messages
/// carry it, so traversal stays inside the window the key was pressed in.
pub(crate) fn navigation_code(message: &str, daemon: bool) -> String {
    let window = if daemon {
        "::std::option::Option::Some(window)"
    } else {
        "::std::option::Option::None"
    };
    let mark = if daemon { ".in_window(window)" } else { "" };
    format!(
        "::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext({window}), {message}::__AccessibilityFocusPrevious({window})){mark}.into()"
    )
}

/// Drain-and-wrap code for component `boot` delivery: the render collected
/// first-sighted scopes, and the root wrapper publishes their messages on
/// the next widget-update pass.
pub(in crate::codegen) fn boot_dispatch_code(
    program: &LoweredProgram,
    message: &str,
) -> (String, String) {
    if !crate::codegen::program_has_boot(program) {
        return (String::new(), String::new());
    }
    (
        format!(
            "let __ice_boots: ::std::vec::Vec<{message}> = self.__ice_boot_queue.borrow_mut().drain(..).collect();"
        ),
        format!(
            "let __ice_root: __IceElement<'_, {message}> = ::ui_lang_runtime::boot_dispatch(__ice_root, __ice_boots);"
        ),
    )
}
