use super::*;

// A handler parameter is a Rust binding, not a materialized temporary: the
// first by-value use MOVES it, so `local` must stay false and every owned use
// clones. `local: true` is reserved for bindings whose code already evaluates
// to a fresh owned value (component state reads) or is Copy — re-emitting one
// of those costs nothing, while re-emitting a parameter is a use after move
// that surfaces as an E0382 on the `include_app!` line with no usable span.
fn handler_param_binding(param: &ResolvedHandlerParam) -> Binding {
    Binding {
        code: param.name.clone(),
        ty: param.ty.clone(),
        local: false,
        state: None,
        owner: Some(BindingOwner::Local(param.local)),
    }
}

pub(in crate::codegen) fn generate_theme(
    out: &mut String,
    program: &LoweredProgram,
) -> Result<(), Error> {
    let settings = program.settings();
    let theme = program.theme();
    let state_env = checked_state_env(program, "self");
    let mut callback_env = ScopedBindingEnv::new(&state_env);
    if settings.kind == ProgramKind::Daemon {
        callback_env.insert(
            "window".into(),
            Binding {
                code: "window".into(),
                ty: Type::WindowId,
                local: true,
                state: None,
                owner: settings.callback_window.map(BindingOwner::Local),
            },
        );
    }
    let callback_arg = if settings.kind == ProgramKind::Daemon {
        ", window: ::iced::window::Id"
    } else {
        ""
    };
    let palette_code = |palette: &crate::lower::ResolvedPalette| {
        let colors = palette
            .colors
            .iter()
            .map(|color| {
                format!(
                    "::iced::Color::from_rgba8({}, {}, {}, {:.6})",
                    color.rgba[0],
                    color.rgba[1],
                    color.rgba[2],
                    color.rgba[3] as f32 / 255.0
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "__IcePalette {{ name: {}, colors: [{colors}] }}",
            rust_string(&palette.name)
        )
    };
    writeln!(
        out,
        "{}",
        source_marker_for_origin(program, theme.active_palette_origin)
    )
    .unwrap();
    writeln!(out, "fn __palette(&self{callback_arg}) -> __IcePalette {{").unwrap();
    if settings.kind == ProgramKind::Daemon {
        writeln!(out, "let _ = &window;").unwrap();
    }
    match &theme.active_palette {
        ResolvedPaletteSelection::Static(id) => {
            writeln!(out, "{}", palette_code(&theme.palettes[id.0 as usize])).unwrap();
        }
        ResolvedPaletteSelection::Dynamic(expression) => {
            let value = resolved_expr_use_code(
                program,
                expression.expression,
                &callback_env,
                ValueMode::Owned,
            )?;
            let contract = canonical_rust_type_name(&theme.contract.name);
            writeln!(out, "match {value} {{").unwrap();
            for palette in &theme.palettes {
                writeln!(
                    out,
                    "{contract}::{} => {},",
                    pascal(&palette.name),
                    palette_code(palette)
                )
                .unwrap();
            }
            writeln!(out, "}}").unwrap();
        }
    }
    writeln!(out, "}}\n{SOURCE_MARKER_END}").unwrap();

    let palette_field =
        |token: crate::lower::ThemeTokenId| format!("__ice_palette.colors[{}]", token.index);
    let callback_value = if settings.kind == ProgramKind::Daemon {
        "window"
    } else {
        ""
    };
    writeln!(
        out,
        "fn __app_theme(__ice_palette: __IcePalette) -> ::iced::Theme {{"
    )
    .unwrap();
    writeln!(
        out,
        "::iced::Theme::custom(::std::format!(\"{}/{{}}\", __ice_palette.name), ::iced::theme::Palette {{",
        settings.app_name,
    )
    .unwrap();
    writeln!(
        out,
        "background: {},",
        palette_field(theme.native_tokens.background)
    )
    .unwrap();
    writeln!(out, "text: {},", palette_field(theme.native_tokens.text)).unwrap();
    writeln!(
        out,
        "primary: {},",
        palette_field(theme.native_tokens.primary)
    )
    .unwrap();
    writeln!(
        out,
        "success: {},",
        palette_field(theme.native_tokens.primary)
    )
    .unwrap();
    writeln!(
        out,
        "warning: {},",
        palette_field(theme.native_tokens.danger)
    )
    .unwrap();
    writeln!(
        out,
        "danger: {},",
        palette_field(theme.native_tokens.danger)
    )
    .unwrap();
    writeln!(out, "}})\n}}").unwrap();
    writeln!(
        out,
        "{}",
        source_marker_for_origin(program, theme.app_theme_origin)
    )
    .unwrap();
    writeln!(out, "fn __theme(&self{callback_arg}) -> ::iced::Theme {{").unwrap();
    match &theme.app_theme {
        ResolvedAppThemeSelection::App => {
            writeln!(out, "Self::__app_theme(self.__palette({callback_value}))").unwrap();
        }
        ResolvedAppThemeSelection::Default => {
            writeln!(
                out,
                "<::iced::Theme as ::iced::theme::Base>::default(::iced::theme::Mode::None)"
            )
            .unwrap();
        }
        ResolvedAppThemeSelection::BuiltIn(name) => {
            writeln!(out, "::iced::Theme::{}", pascal(name)).unwrap();
        }
        ResolvedAppThemeSelection::Factory(factory) => {
            writeln!(
                out,
                "{}",
                resolved_app_theme_factory_code(factory, &callback_env, program)?
            )
            .unwrap();
        }
        ResolvedAppThemeSelection::Dynamic(expression) => {
            let value = resolved_expr_use_code(
                program,
                expression.expression,
                &callback_env,
                ValueMode::Owned,
            )?;
            writeln!(out, "match ({value}).as_str() {{").unwrap();
            writeln!(
                out,
                "\"app\" => Self::__app_theme(self.__palette({callback_value})),"
            )
            .unwrap();
            writeln!(out, "\"default\" => <::iced::Theme as ::iced::theme::Base>::default(::iced::theme::Mode::None),").unwrap();
            for name in BUILT_IN_THEMES {
                writeln!(out, "\"{name}\" => ::iced::Theme::{},", pascal(name)).unwrap();
            }
            writeln!(
                out,
                "_ => Self::__app_theme(self.__palette({callback_value})),\n}}"
            )
            .unwrap();
        }
    }
    writeln!(out, "}}\n{SOURCE_MARKER_END}").unwrap();
    if let Some(setting) = &settings.title {
        let value =
            resolved_expr_use_code(program, setting.expression, &callback_env, ValueMode::Owned)?;
        writeln!(out, "{}", source_marker_for_origin(program, setting.origin)).unwrap();
        writeln!(
            out,
            "fn __title(&self{callback_arg}) -> ::std::string::String {{ {value} }}\n{SOURCE_MARKER_END}"
        )
        .unwrap();
    }
    if settings.background.is_some() || settings.text_color.is_some() {
        writeln!(out, "fn __style(&self, __theme: &::iced::Theme) -> ::iced::theme::Style {{ let mut __style = ::iced::theme::Base::base(__theme);").unwrap();
        for (setting, field) in [
            (&settings.background, "background_color"),
            (&settings.text_color, "text_color"),
        ] {
            if let Some(setting) = setting {
                let value = resolved_expr_use_code(
                    program,
                    setting.expression,
                    &state_env,
                    ValueMode::Owned,
                )?;
                writeln!(out, "{}", source_marker_for_origin(program, setting.origin)).unwrap();
                writeln!(out, "__style.{field} = ({value}).parse::<::iced::Color>().unwrap_or(__style.{field});\n{SOURCE_MARKER_END}").unwrap();
            }
        }
        writeln!(out, "__style }}").unwrap();
    }
    if let Some(setting) = &settings.scale_factor {
        let value =
            resolved_expr_use_code(program, setting.expression, &callback_env, ValueMode::Owned)?;
        writeln!(out, "{}", source_marker_for_origin(program, setting.origin)).unwrap();
        writeln!(
            out,
            "fn __scale_factor(&self{callback_arg}) -> f32 {{ (({value}) as f32).max(f32::EPSILON).min(f32::MAX) }}\n{SOURCE_MARKER_END}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_boot(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(program.app_name());
    let tray_init = tray_init_code(program, program.settings(), source_path);
    let tray_sync = tray_boot_sync_code(program.settings());
    writeln!(out, "fn __state() -> Self {{").unwrap();
    for (pane, test_only) in document_pane_grids(program) {
        let field = pane_field(&pane.name);
        if test_only {
            writeln!(out, "#[cfg(test)]").unwrap();
        }
        writeln!(
            out,
            "let {field} = ::iced::widget::pane_grid::State::with_configuration({});",
            pane_configuration_code(
                &pane.configuration,
                (!pane.templates.is_empty())
                    .then(|| pane_type(&pane.name))
                    .as_deref()
            )
        )
        .unwrap();
        let slots = pane_split_slots(&pane.configuration);
        if slots.iter().any(Option::is_some) {
            let slots = slots
                .iter()
                .map(|name| {
                    name.map_or_else(
                        || "::std::option::Option::None".into(),
                        |name| format!("::std::option::Option::Some({})", rust_string(name)),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "let {} = [{slots}].into_iter().zip({field}.layout().splits().copied()).filter_map(|(__name, __split)| __name.map(|__name| (__name, __split))).collect();",
                pane_splits_field(&pane.name)
            )
            .unwrap();
        }
    }
    writeln!(out, "Self {{").unwrap();
    let accessibility_bridge = if program.settings().kind == ProgramKind::Daemon {
        "::ui_lang_runtime::Bridge::without_native_adapter()"
    } else {
        "::ui_lang_runtime::Bridge::new()"
    };
    writeln!(out, "__ice_accessibility: {accessibility_bridge},").unwrap();
    if program.settings().kind == ProgramKind::Application {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\n__ice_accessibility_initial: ::std::option::Option::None,\n#[cfg(all(target_os = \"windows\", not(test)))]\n__ice_accessibility_pending: ::std::vec::Vec::new(),"
        )
        .unwrap();
    }
    for (lane, _, mode) in app_run_lanes(program) {
        writeln!(out, "{}: 0,", run_lane_generation_field(lane.0 as usize)).unwrap();
        if mode == DeliveryMode::Replace {
            writeln!(
                out,
                "{}: ::std::option::Option::None,",
                run_lane_handle_field(lane.0 as usize)
            )
            .unwrap();
        }
    }
    for state in program.app_states() {
        writeln!(out, "{}", source_marker(&state.span)).unwrap();
        writeln!(
            out,
            "{}: {},",
            state.name,
            resolved_initializer_code(&state.initializer, program)?
        )
        .unwrap();
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    if !program.secrets().is_empty() {
        writeln!(
            out,
            "{SECRET_STORE_FIELD}: ::std::default::Default::default(),"
        )
        .unwrap();
    }
    if crate::codegen::program_has_boot(program) {
        writeln!(
            out,
            "__ice_boot_queue: ::std::cell::RefCell::new(::std::vec::Vec::new()),"
        )
        .unwrap();
    }
    for component in program
        .components()
        .iter()
        .filter(|component| component.storage != ComponentStorage::Stateless)
    {
        let initial = match component.storage {
            ComponentStorage::Retained => "::std::collections::HashMap::new()",
            ComponentStorage::Mounted => "::ui_lang_runtime::MountedComponentState::default()",
            ComponentStorage::Stateless => unreachable!(),
        };
        writeln!(
            out,
            "{}: {initial},",
            component_state_field(&component.name),
        )
        .unwrap();
        for state in component
            .states
            .iter()
            .filter(|state| state.ty == Type::Editor)
        {
            writeln!(
                out,
                "{}: {},",
                component_editor_initial_field(&component.name, &state.name),
                resolved_initializer_code(&state.initializer, program)?
            )
            .unwrap();
        }
    }
    for (pane, test_only) in document_pane_grids(program) {
        if test_only {
            writeln!(out, "#[cfg(test)]").unwrap();
        }
        writeln!(out, "{},", pane_field(&pane.name)).unwrap();
        if pane_split_slots(&pane.configuration)
            .iter()
            .any(Option::is_some)
        {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(out, "{},", pane_splits_field(&pane.name)).unwrap();
        }
    }
    writeln!(out, "}}\n}}").unwrap();
    let mount = program
        .app_handlers()
        .find(|handler| handler.name == "mount")
        .map_or(&[][..], |handler| handler.statements.as_slice());
    generate_initial_task_method(out, program, message, "__boot_task", mount)?;
    if program.settings().kind == ProgramKind::Daemon {
        writeln!(
            out,
            "fn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\n{tray_init}let task = state.__boot_task();\n{tray_sync}(state, task)\n}}"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\nfn __accessibility_attach() -> ::iced::Task<{message}> {{\n::iced::window::oldest().then(|__id| match __id {{\n::std::option::Option::Some(__id) => ::ui_lang_runtime::native_window(__id).map({message}::__AccessibilityNativeWindow),\n::std::option::Option::None => ::iced::Task::none(),\n}})\n}}\nfn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\n{tray_init}#[cfg(all(target_os = \"windows\", not(test)))]\n{{\nstate.__ice_accessibility_initial = ::std::option::Option::Some(0);\n(state, Self::__accessibility_attach())\n}}\n#[cfg(not(all(target_os = \"windows\", not(test))))]\n{{\nlet task = state.__boot_task();\n{tray_sync}let __accessibility = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\n(state, ::iced::Task::batch([task, __accessibility]))\n}}\n}}"
        )
        .unwrap();
    }
    Ok(())
}

/// Re-evaluates every tray expression that can change and hands the result to
/// the runtime, which drops the ones that did not. Emitted only when something
/// can change; literals were applied once at `init`.
///
/// Also emits `__tray_row`, the one row-to-handler table. The live
/// subscription maps a chosen row through it and `tray choose` in a test
/// drives the same rows through the same table, so an index that drifts here
/// fails a test instead of leaving every row quietly dead in the menu bar.
pub(in crate::codegen) fn generate_tray(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
) -> Result<(), Error> {
    let Some(tray) = &program.settings().tray else {
        return Ok(());
    };
    let routes = tray
        .menu
        .iter()
        .enumerate()
        .filter_map(|(index, row)| match row {
            ResolvedTrayRow::Item {
                route: Some(route), ..
            } => Some(format!(
                "{index}usize => ::std::option::Option::Some({message}::{}),",
                handler_variant(route)
            )),
            _ => None,
        })
        .collect::<String>();
    if !routes.is_empty() {
        writeln!(
            out,
            "fn __tray_row(__row: usize) -> ::std::option::Option<{message}> {{ match __row {{ {routes} _ => ::std::option::Option::None }} }}"
        )
        .unwrap();
    }
    if !tray.reactive() {
        return Ok(());
    }
    let env = checked_state_env(program, "self");
    writeln!(out, "fn __tray_sync(&self) {{").unwrap();
    // One guard per guarded icon, in declaration order. First-match-wins and
    // the fall back to the unguarded last icon are the runtime's rule, not a
    // shape repeated in every generated program.
    let guards = tray
        .icons
        .iter()
        .filter_map(|icon| icon.when.as_ref())
        .map(|guard| resolved_expr_use_code(program, guard.expression, &env, ValueMode::Owned))
        .collect::<Result<Vec<_>, Error>>()?;
    if !guards.is_empty() {
        writeln!(
            out,
            "::ui_lang_runtime::tray::select_icon(&[{}]);",
            guards.join(", ")
        )
        .unwrap();
    }
    for (text, apply) in [
        (&tray.label, "::ui_lang_runtime::tray::set_label"),
        (&tray.tooltip, "::ui_lang_runtime::tray::set_tooltip"),
    ] {
        if let Some(ResolvedTrayText::Expression(setting)) = text {
            let value =
                resolved_expr_use_code(program, setting.expression, &env, ValueMode::Owned)?;
            writeln!(out, "{}", source_marker_for_origin(program, setting.origin)).unwrap();
            writeln!(out, "{apply}(&({value}));\n{SOURCE_MARKER_END}").unwrap();
        }
    }
    // Declaration indices, separators included, so a separator simply has no
    // line here and no later row shifts under the runtime's row vector.
    for (index, row) in tray.menu.iter().enumerate() {
        if let ResolvedTrayRow::Item {
            text: ResolvedTrayText::Expression(setting),
            ..
        } = row
        {
            let value =
                resolved_expr_use_code(program, setting.expression, &env, ValueMode::Owned)?;
            writeln!(out, "{}", source_marker_for_origin(program, setting.origin)).unwrap();
            writeln!(
                out,
                "::ui_lang_runtime::tray::set_item({index}usize, &({value}));\n{SOURCE_MARKER_END}"
            )
            .unwrap();
        }
    }
    writeln!(out, "}}").unwrap();
    Ok(())
}

pub(in crate::codegen) fn generate_presets(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    let settings = program.settings();
    let accessibility_root = rust_string(&program.settings().app_name);
    let tray_init = tray_init_code(program, settings, source_path);
    let tray_sync = tray_boot_sync_code(settings);
    for (index, preset) in program.preset_handlers().enumerate() {
        let task_name = format!("__preset_task_{index}");
        generate_initial_task_method(out, program, message, &task_name, &preset.statements)?;
        writeln!(
            out,
            "fn __preset_{index}() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\n{tray_init}"
        )
        .unwrap();
        if settings.kind == ProgramKind::Daemon {
            writeln!(
                out,
                "let task = state.{task_name}();\n{tray_sync}(state, task)\n}}"
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "#[cfg(all(target_os = \"windows\", not(test)))]\n{{\nstate.__ice_accessibility_initial = ::std::option::Option::Some({});\n(state, Self::__accessibility_attach())\n}}\n#[cfg(not(all(target_os = \"windows\", not(test))))]\n{{\nlet task = state.{task_name}();\n{tray_sync}let __accessibility = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\n(state, ::iced::Task::batch([task, __accessibility]))\n}}\n}}",
                index + 1
            )
            .unwrap();
        }
    }
    if settings.kind == ProgramKind::Application {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\nfn __accessibility_initial_task(&mut self) -> ::iced::Task<{message}> {{\nmatch self.__ice_accessibility_initial.take() {{\n::std::option::Option::Some(0) => self.__boot_task(),"
        )
        .unwrap();
        for index in 0..program.preset_handlers().count() {
            writeln!(
                out,
                "::std::option::Option::Some({}) => self.__preset_task_{index}(),",
                index + 1
            )
            .unwrap();
        }
        writeln!(out, "_ => ::iced::Task::none(),\n}}\n}}").unwrap();
    }
    Ok(())
}

fn generate_initial_task_method(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    name: &str,
    statements: &[ResolvedStatement],
) -> Result<(), Error> {
    writeln!(
        out,
        "fn {name}(&mut self) -> ::iced::Task<{message}> {{\nlet task = (|| {{"
    )
    .unwrap();
    let env = checked_state_env(program, "self");
    let has_task = generate_statements(
        out,
        statements,
        program,
        message,
        &env,
        "self",
        TaskMode::Bare,
    )?;
    if !has_task {
        writeln!(out, "::iced::Task::none()").unwrap();
    }
    writeln!(out, "}})();\ntask\n}}").unwrap();
    Ok(())
}

pub(in crate::codegen) fn generate_update(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(program.app_name());
    let has_fallthrough_arm = program
        .app_handlers()
        .any(|handler| handler.name != "mount")
        || program.components().iter().any(|component| {
            !component.handlers.is_empty()
                || component.states.iter().any(|state| state.ty == Type::Str)
        })
        || document_pane_grids(program)
            .into_iter()
            .any(|(pane, _)| pane.resize_leeway.is_some() || pane.draggable)
        || !program.controlled_input_bindings()?.is_empty()
        || !program.controlled_editor_bindings()?.is_empty()
        || needs_extern_noop(program);
    let task_binding = if has_fallthrough_arm {
        "let __task = "
    } else {
        ""
    };
    let windows_show = program.settings().primary_window.visible != Some(false);
    let windows_fullscreen = program.settings().primary_window.fullscreen == Some(true);
    let windows_maximized = program.settings().primary_window.maximized == Some(true);
    let windows_restore: String = if !windows_show {
        "::iced::Task::none()".into()
    } else if windows_fullscreen {
        "::iced::window::set_mode(__id, ::iced::window::Mode::Fullscreen)".into()
    } else if windows_maximized {
        "::iced::window::set_mode(__id, ::iced::window::Mode::Windowed).chain(::iced::window::maximize(__id, true))".into()
    } else {
        "::iced::window::set_mode(__id, ::iced::window::Mode::Windowed)".into()
    };
    writeln!(
        out,
        "#[allow(clippy::assign_op_pattern)]\npub(super) fn __update(&mut self, message: {message}) -> ::iced::Task<{message}> {{"
    )
    .unwrap();
    if program.settings().kind == ProgramKind::Application {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\nif !self.__ice_accessibility.is_attached() && !matches!(&message, {message}::__AccessibilityNativeWindow(_)) {{\nself.__ice_accessibility_pending.push(message);\nreturn ::iced::Task::none();\n}}"
        )
        .unwrap();
    }
    writeln!(
        out,
        "{task_binding}match message {{\n{message}::__AccessibilitySnapshot(__snapshot) => {{ self.__ice_accessibility.update(*__snapshot); return ::iced::Task::none(); }},\n{message}::__AccessibilityAction(__request) => {{ let __refresh = matches!(__request.action, ::ui_lang_runtime::Action::Focus); let __task = self.__ice_accessibility.dispatch(__request); return if __refresh {{ __task.chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))) }} else {{ __task }}; }},\n{message}::__AccessibilityWindow(__id, __event) => {{ self.__ice_accessibility.window_event(__id, __event); return ::iced::Task::none(); }},"
    )
    .unwrap();
    if program.settings().kind == ProgramKind::Daemon {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\n{message}::__AccessibilityNativeWindow(_) => {{ return ::iced::Task::none(); }},"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\n{message}::__AccessibilityNativeWindow(__window) => {{\nlet __id = __window.id();\nif !self.__ice_accessibility.attach_window(__window) {{ return ::iced::Task::none(); }}\nlet __restore = {windows_restore};\nlet __initial = self.__accessibility_initial_task();\nlet mut __pending = ::std::vec::Vec::new();\nfor __message in ::std::mem::take(&mut self.__ice_accessibility_pending) {{\n__pending.push(self.__update(__message));\n}}\nlet __pending = ::iced::Task::batch(__pending);\nlet __snapshot = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\nreturn __restore.chain(::iced::Task::batch([__initial, __pending, __snapshot]));\n}},"
        )
        .unwrap();
    }
    writeln!(
        out,
        "{message}::__AccessibilityFocusNext => {{ return ::ui_lang_runtime::focus_next::<{message}>().chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))); }},\n{message}::__AccessibilityFocusPrevious => {{ return ::ui_lang_runtime::focus_previous::<{message}>().chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))); }},\n{message}::__TemplateChanged => {{ return ::iced::Task::none(); }},"
    )
    .unwrap();
    for (lane, kind, mode) in app_run_lanes(program) {
        let variant = run_lane_variant(lane.0 as usize);
        let generation = run_lane_generation_field(lane.0 as usize);
        if kind == EffectKind::Stream && mode == DeliveryMode::Replace {
            let handle = run_lane_handle_field(lane.0 as usize);
            writeln!(
                out,
                "{message}::{variant}(__generation, __message) => {{ if self.{generation} == __generation {{ if let ::std::option::Option::Some(__message) = __message {{ return self.__update(*__message); }} self.{handle} = ::std::option::Option::None; }} return ::iced::Task::none(); }},"
            )
            .unwrap();
        } else {
            let clear = if mode == DeliveryMode::Replace {
                format!(
                    "self.{} = ::std::option::Option::None; ",
                    run_lane_handle_field(lane.0 as usize)
                )
            } else {
                String::new()
            };
            writeln!(
                out,
                "{message}::{variant}(__generation, __message) => {{ if self.{generation} == __generation {{ {clear}return self.__update(*__message); }} return ::iced::Task::none(); }},"
            )
            .unwrap();
        }
    }
    let app_handler_env = checked_state_env(program, "self");
    for handler in program.app_handlers() {
        if handler.name == "mount" {
            continue;
        }
        let variant = handler_variant(&handler.name);
        let pattern = if handler.params.is_empty() {
            format!("{message}::{variant}")
        } else {
            format!(
                "{message}::{variant}({})",
                handler
                    .params
                    .iter()
                    .map(|param| param.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        // Keep statement-level `return` inside this arm so every user update
        // reaches the post-state accessibility snapshot below.
        writeln!(out, "{pattern} => (|| {{").unwrap();
        for param in &handler.params {
            writeln!(out, "let _ = &{};", param.name).unwrap();
        }
        let mut env = ScopedBindingEnv::new(&app_handler_env);
        for param in &handler.params {
            env.insert(param.name.clone(), handler_param_binding(param));
        }
        let publishes = publishes_slices(&handler.statements);
        if publishes {
            writeln!(
                out,
                "let mut {} : ::std::vec::Vec<::iced::Task<{message}>> = ::std::vec::Vec::new();",
                SLICE_ACCUMULATOR
            )
            .unwrap();
        }
        let has_task = generate_statements(
            out,
            &handler.statements,
            program,
            message,
            &env,
            "self",
            if publishes {
                TaskMode::Accumulate
            } else {
                TaskMode::Return
            },
        )?;
        if publishes {
            writeln!(out, "::iced::Task::batch({})", SLICE_ACCUMULATOR).unwrap();
        } else if !has_task {
            writeln!(out, "::iced::Task::none()").unwrap();
        }
        writeln!(out, "}})(),").unwrap();
    }
    for component in program
        .components()
        .iter()
        .filter(|component| component.storage != ComponentStorage::Stateless)
    {
        let field = component_state_field(&component.name);
        let run_lanes = component_run_lanes(program, &component.handlers);
        let owns_persistent_state = !component.states.is_empty() || !run_lanes.is_empty();
        let entry = |scope: &str| {
            if !owns_persistent_state {
                // Handler scope is routing context, not retained state.
                return format!(
                    "let mut __local = {}::default();",
                    component_state_type(&component.name)
                );
            }
            match component.storage {
                ComponentStorage::Retained => {
                    format!("let __local = self.{field}.entry({scope}).or_default();")
                }
                ComponentStorage::Mounted => format!(
                    "let mut __states = self.{field}.values_mut(); let __local = __states.entry({scope}).or_default();"
                ),
                ComponentStorage::Stateless => unreachable!(),
            }
        };
        for (lane, kind, mode) in run_lanes {
            let generation = run_lane_generation_field(lane.0 as usize);
            let variant = run_lane_variant(lane.0 as usize);
            let stream = kind == EffectKind::Stream && mode == DeliveryMode::Replace;
            let clear = if stream {
                format!(
                    "if __message.is_none() {{ __local.{} = ::std::option::Option::None; }} ",
                    run_lane_handle_field(lane.0 as usize)
                )
            } else if mode == DeliveryMode::Replace {
                format!(
                    "__local.{} = ::std::option::Option::None; ",
                    run_lane_handle_field(lane.0 as usize)
                )
            } else {
                String::new()
            };
            let current = match component.storage {
                ComponentStorage::Retained => format!(
                    "self.{field}.get_mut(&__scope).is_some_and(|__local| {{ if __local.{generation} == __generation {{ {clear}true }} else {{ false }} }})"
                ),
                ComponentStorage::Mounted => format!(
                    "{{ let mut __values = self.{field}.values_mut(); __values.get_mut(&__scope).is_some_and(|__local| {{ if __local.{generation} == __generation {{ {clear}true }} else {{ false }} }}) }}"
                ),
                ComponentStorage::Stateless => unreachable!(),
            };
            if stream {
                writeln!(
                    out,
                    "{message}::{variant}(__scope, __generation, __message) => {{ let __current = {current}; if __current && let ::std::option::Option::Some(__message) = __message {{ return self.__update(*__message); }} return ::iced::Task::none(); }},"
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "{message}::{variant}(__scope, __generation, __message) => {{ let __current = {current}; if __current {{ return self.__update(*__message); }} return ::iced::Task::none(); }},"
                )
                .unwrap();
            }
        }
        let component_handler_env = component
            .states
            .iter()
            .map(|state| {
                (
                    state.name.clone(),
                    Binding {
                        code: format!("__local.{}", state.name),
                        ty: state.ty.clone(),
                        local: false,
                        state: None,
                        owner: Some(BindingOwner::Value(ResolvedValueRef::ComponentState(
                            state.id,
                        ))),
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        for handler in component.handlers.iter().map(|id| program.handler(*id)) {
            let variant = component_handler_variant(&component.name, &handler.name);
            let mut bindings = vec!["__scope".to_owned()];
            bindings.extend(handler.params.iter().map(|param| param.name.clone()));
            writeln!(
                out,
                "{message}::{variant}({}) => (|| {{",
                bindings.join(", ")
            )
            .unwrap();
            for param in &handler.params {
                writeln!(out, "let _ = &{};", param.name).unwrap();
            }
            writeln!(out, "{}", entry("__scope.clone()")).unwrap();
            let mut env = ScopedBindingEnv::new(&component_handler_env);
            for param in &handler.params {
                env.insert(param.name.clone(), handler_param_binding(param));
            }
            insert_scoped_component_context(
                &mut env,
                &component.name,
                Binding {
                    code: "__scope".into(),
                    ty: Type::Unit,
                    local: true,
                    state: None,
                    owner: None,
                },
            );
            let has_task = generate_statements(
                out,
                &handler.statements,
                program,
                message,
                &env,
                "__local",
                TaskMode::Return,
            )?;
            if !has_task {
                writeln!(out, "::iced::Task::none()").unwrap();
            }
            writeln!(out, "}})(),").unwrap();
        }
        for state in component
            .states
            .iter()
            .filter(|state| state.ty == Type::Str)
        {
            let variant = component_binding_variant(&component.name, &state.name);
            let entry = entry("__scope");
            writeln!(
                out,
                "{message}::{variant}(__scope, value) => {{ {entry} __local.{} = value; ::iced::Task::none() }},",
                state.name
            )
            .unwrap();
        }
        for state in component
            .states
            .iter()
            .filter(|state| state.ty == Type::Editor)
        {
            let variant = component_editor_variant(&component.name, &state.name);
            let entry = entry("__scope");
            let perform =
                match program.component_controlled_editor_action(&component.name, &state.name) {
                    Some(action) => format!(
                        "{}(&mut __local.{}, __action);",
                        program.extern_function(action).rust_path,
                        state.name
                    ),
                    None => format!("__local.{}.perform(__action);", state.name),
                };
            writeln!(
                out,
                "{message}::{variant}(__scope, __action) => {{ {entry} {perform} ::iced::Task::none() }},"
            )
            .unwrap();
        }
    }
    for (pane, test_only) in document_pane_grids(program) {
        if pane.resize_leeway.is_some() {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{message}::{}(__event) => {{ self.{}.resize(__event.split, __event.ratio); ::iced::Task::none() }},",
                pane_resize_variant(&pane.name),
                pane_field(&pane.name)
            )
            .unwrap();
        }
        if pane.draggable {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{message}::{}(__event) => {{ if let ::iced::widget::pane_grid::DragEvent::Dropped {{ pane, target }} = __event {{ self.{}.drop(pane, target); }} ::iced::Task::none() }},",
                pane_drag_variant(&pane.name),
                pane_field(&pane.name)
            )
            .unwrap();
        }
    }
    if !program.secrets().is_empty() {
        // The slot name arrives as an owned `String` because the message must
        // be `Clone`; it is matched back to the compiler-owned `&'static str`
        // so the store is never keyed by anything a message invented.
        let arms = program
            .secrets()
            .iter()
            .map(|slot| format!("{} => Some({}),", rust_string(slot), rust_string(slot)))
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(
            out,
            "{message}::{SECRET_TYPED_VARIANT}(slot, text) => {{ if let Some(slot) = match slot.as_str() {{ {arms} _ => None }} {{ self.{SECRET_STORE_FIELD}.set(slot, text); }} ::iced::Task::none() }}"
        )
        .unwrap();
    }
    for binding in program.controlled_input_bindings()? {
        let variant = binding_variant(binding);
        writeln!(
            out,
            "{message}::{variant}(value) => {{ self.{binding} = value; ::iced::Task::none() }}"
        )
        .unwrap();
    }
    for binding in program.controlled_editor_bindings()? {
        let variant = editor_variant(&binding.name);
        if let Some(action) = binding.action {
            let function = program.extern_function(action);
            writeln!(
                out,
                "{message}::{variant}(action) => {{ {}(&mut self.{}, action); ::iced::Task::none() }}",
                function.rust_path, binding.name
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "{message}::{variant}(action) => {{ self.{}.perform(action); ::iced::Task::none() }}",
                binding.name
            )
            .unwrap();
        }
    }
    if needs_extern_noop(program) {
        writeln!(out, "{message}::__ExternNoop => ::iced::Task::none(),").unwrap();
    }
    if has_animations(program) {
        writeln!(
            out,
            "{message}::__AnimationFrame => return ::iced::Task::none(),"
        )
        .unwrap();
    }
    if !has_fallthrough_arm {
        writeln!(out, "}}\n}}").unwrap();
        return Ok(());
    }
    let tray_sync = program
        .settings()
        .tray
        .as_ref()
        .filter(|tray| tray.reactive())
        .map_or("", |_| "self.__tray_sync();\n");
    if program.settings().kind == ProgramKind::Daemon {
        writeln!(out, "}};\n{tray_sync}__task\n}}").unwrap();
    } else {
        writeln!(
            out,
            "}};\n{tray_sync}// Snapshotting the widget tree after every message serves ONLY an attached\n// assistive technology (and the test harness, which drives the app through\n// this tree) — ungated it walked every widget, built a TreeUpdate nobody\n// read, and scheduled a second frame per message.\nlet __accessibility = if cfg!(test) || ::ui_lang_runtime::accessibility_active() {{\n::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))\n}} else {{\n::iced::Task::none()\n}};\n::iced::Task::batch([__task, __accessibility])\n}}"
        )
        .unwrap();
    }
    Ok(())
}
