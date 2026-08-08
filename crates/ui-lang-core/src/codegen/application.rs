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
    let mut callback_env = state_env.clone();
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
    if program
        .settings()
        .tray
        .as_ref()
        .is_some_and(|tray| tray.popover.is_some())
    {
        writeln!(out, "__ice_tray_popover: ::std::option::Option::None,").unwrap();
    }
    if program.settings().kind == ProgramKind::Application {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\n__ice_accessibility_initial: ::std::option::Option::None,\n#[cfg(all(target_os = \"windows\", not(test)))]\n__ice_accessibility_pending: ::std::vec::Vec::new(),"
        )
        .unwrap();
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
            "fn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\n{tray_init}let task = state.__boot_task();\n(state, task)\n}}"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\nfn __accessibility_attach() -> ::iced::Task<{message}> {{\n::iced::window::oldest().then(|__id| match __id {{\n::std::option::Option::Some(__id) => ::ui_lang_runtime::native_window(__id).map({message}::__AccessibilityNativeWindow),\n::std::option::Option::None => ::iced::Task::none(),\n}})\n}}\nfn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\n{tray_init}#[cfg(all(target_os = \"windows\", not(test)))]\n{{\nstate.__ice_accessibility_initial = ::std::option::Option::Some(0);\n(state, Self::__accessibility_attach())\n}}\n#[cfg(not(all(target_os = \"windows\", not(test))))]\n{{\nlet task = state.__boot_task();\nlet __accessibility = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\n(state, ::iced::Task::batch([task, __accessibility]))\n}}\n}}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_tray(
    out: &mut String,
    program: &LoweredProgram,
) -> Result<(), Error> {
    let Some(tray) = &program.settings().tray else {
        return Ok(());
    };
    if tray.label.is_none() && tray.tooltip.is_none() {
        return Ok(());
    }
    let env = checked_state_env(program, "self");
    writeln!(out, "fn __tray_sync(&self) {{").unwrap();
    for (setting, apply) in [
        (&tray.label, "::ui_lang_runtime::tray::set_label"),
        (&tray.tooltip, "::ui_lang_runtime::tray::set_tooltip"),
    ] {
        if let Some(setting) = setting {
            let value =
                resolved_expr_use_code(program, setting.expression, &env, ValueMode::Owned)?;
            writeln!(out, "{}", source_marker_for_origin(program, setting.origin)).unwrap();
            writeln!(out, "{apply}(&({value}));\n{SOURCE_MARKER_END}").unwrap();
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
    for (index, preset) in program.preset_handlers().enumerate() {
        let task_name = format!("__preset_task_{index}");
        generate_initial_task_method(out, program, message, &task_name, &preset.statements)?;
        writeln!(
            out,
            "fn __preset_{index}() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\n{tray_init}"
        )
        .unwrap();
        if settings.kind == ProgramKind::Daemon {
            writeln!(out, "let task = state.{task_name}();\n(state, task)\n}}").unwrap();
        } else {
            writeln!(
                out,
                "#[cfg(all(target_os = \"windows\", not(test)))]\n{{\nstate.__ice_accessibility_initial = ::std::option::Option::Some({});\n(state, Self::__accessibility_attach())\n}}\n#[cfg(not(all(target_os = \"windows\", not(test))))]\n{{\nlet task = state.{task_name}();\nlet __accessibility = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\n(state, ::iced::Task::batch([task, __accessibility]))\n}}\n}}",
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
    let has_task = generate_statements(out, statements, program, message, &env, "self", false)?;
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
        "#[allow(clippy::assign_op_pattern)]\nfn __update(&mut self, message: {message}) -> ::iced::Task<{message}> {{"
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
        "{message}::__AccessibilityFocusNext => {{ return ::ui_lang_runtime::focus_next::<{message}>().chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))); }},\n{message}::__AccessibilityFocusPrevious => {{ return ::ui_lang_runtime::focus_previous::<{message}>().chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))); }},"
    )
    .unwrap();
    if let Some(tray) = &program.settings().tray {
        if let Some(popover) = tray.popover {
            writeln!(
                out,
                "{message}::__TrayEvent(__event) => {{ match __event {{ ::ui_lang_runtime::tray::TrayEvent::LeftClick {{ icon }} => {{\nif let ::std::option::Option::Some(__id) = self.__ice_tray_popover.take() {{ return ::iced::window::close(__id); }}\nlet mut __settings = Self::__window_{index}();\n__settings.visible = false;\n__settings.position = ::iced::window::Position::Default;\nlet __size = __settings.size;\nlet (__id, __open) = ::iced::window::open(__settings);\nself.__ice_tray_popover = ::std::option::Option::Some(__id);\nreturn __open.discard().chain(::iced::window::scale_factor(__id).then(move |__scale| {{\nlet __icon = icon.clone();\n::iced::window::monitor_size(__id).then(move |__monitor| {{\nlet __anchor = ::ui_lang_runtime::tray::anchor_position(&__icon, f64::from(__scale), __size, __monitor);\n::iced::window::move_to(__id, __anchor).chain(::iced::window::set_mode(__id, ::iced::window::Mode::Windowed)).chain(::iced::window::gain_focus(__id))\n}})\n}}));\n}} }} }},\n{message}::__TrayPopoverClosed(__id) => {{ if self.__ice_tray_popover == ::std::option::Option::Some(__id) {{ self.__ice_tray_popover = ::std::option::Option::None; }} return ::iced::Task::none(); }},",
                index = popover.0
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "{message}::__TrayEvent(__event) => {{ let _ = __event; return ::iced::Task::none(); }},"
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
        let has_task = generate_statements(
            out,
            &handler.statements,
            program,
            message,
            &env,
            "self",
            true,
        )?;
        if !has_task {
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
        let values = match component.storage {
            ComponentStorage::Retained => format!("self.{field}"),
            ComponentStorage::Mounted => format!("self.{field}.values()"),
            ComponentStorage::Stateless => unreachable!(),
        };
        let entry = |scope: &str| match component.storage {
            ComponentStorage::Retained => {
                format!("let __local = self.{field}.entry({scope}).or_default();")
            }
            ComponentStorage::Mounted => format!(
                "let mut __states = self.{field}.values_mut(); let __local = __states.entry({scope}).or_default();"
            ),
            ComponentStorage::Stateless => unreachable!(),
        };
        for (site, _) in component_run_sites(program, &component.handlers) {
            let generation = component_latest_field(site.0 as usize);
            let variant = component_latest_variant(&component.name, site.0 as usize);
            writeln!(
                out,
                "{message}::{variant}(__scope, __generation, __message) => {{ if {values}.get(&__scope).is_some_and(|__local| __local.{generation} == __generation) {{ return self.__update(*__message); }} return ::iced::Task::none(); }},"
            )
            .unwrap();
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
            let future = handler_future(handler);
            if future.is_some() {
                writeln!(
                    out,
                    "let __route_scope = __scope.clone(); let __task = {{ {}",
                    entry("__scope.clone()")
                )
                .unwrap();
            } else {
                writeln!(out, "{}", entry("__scope.clone()")).unwrap();
            }
            let mut env = ScopedBindingEnv::new(&component_handler_env);
            for param in &handler.params {
                env.insert(param.name.clone(), handler_param_binding(param));
            }
            insert_scoped_component_context(
                &mut env,
                &component.name,
                Binding {
                    code: if future.is_some() {
                        "__route_scope".into()
                    } else {
                        "__scope".into()
                    },
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
                future.is_none(),
            )?;
            if let Some((mode, site)) = future {
                debug_assert!(has_task);
                writeln!(out, "}};").unwrap();
                match mode {
                    FutureMode::Every => writeln!(out, "__task").unwrap(),
                    FutureMode::Latest | FutureMode::Replace => {
                        let site = site.expect("latest and replace have stable run-site IDs");
                        let generation = component_latest_field(site.0 as usize);
                        let future_variant =
                            component_latest_variant(&component.name, site.0 as usize);
                        match component.storage {
                            ComponentStorage::Retained => writeln!(out, "let __generation = {{ {} __local.{generation} = __local.{generation}.wrapping_add(1); __local.{generation} }};", entry("__scope.clone()")).unwrap(),
                            ComponentStorage::Mounted => writeln!(out, "let __generation = self.{field}.next_generation(); {{ {} __local.{generation} = __generation; }}", entry("__scope.clone()")).unwrap(),
                            ComponentStorage::Stateless => unreachable!(),
                        }
                        if mode == FutureMode::Replace {
                            let replace = component_replace_field(site.0 as usize);
                            writeln!(out, "let (__task, __handle) = __task.abortable(); {{ {} if let ::std::option::Option::Some(__previous) = __local.{replace}.replace(__handle.abort_on_drop()) {{ __previous.abort(); }} }}", entry("__scope.clone()")).unwrap();
                        }
                        writeln!(out, "__task.map(move |__message| {message}::{future_variant}(__scope.clone(), __generation, ::std::boxed::Box::new(__message)))").unwrap();
                    }
                }
            } else if !has_task {
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
        .filter(|tray| tray.label.is_some() || tray.tooltip.is_some())
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
