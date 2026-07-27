use super::*;

pub(in crate::codegen) fn generate_theme(
    out: &mut String,
    document: &Document,
) -> Result<(), Error> {
    let state_env = state_env(document, "self");
    let mut callback_env = state_env.clone();
    if document.daemon {
        callback_env.insert(
            "window".into(),
            Binding {
                code: "window".into(),
                ty: Type::WindowId,
                local: true,
                state: None,
            },
        );
    }
    let callback_arg = if document.daemon {
        ", window: ::iced::window::Id"
    } else {
        ""
    };
    let contract = document
        .theme_contract
        .as_ref()
        .expect("checker requires a theme contract");
    let palette_code = |palette: &Palette| {
        let colors = contract
            .tokens
            .iter()
            .map(|token| {
                color_code(
                    palette
                        .colors
                        .get(token)
                        .expect("checker requires complete palettes"),
                    None,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "__IcePalette {{ name: {}, colors: [{colors}] }}",
            rust_string(&palette.name)
        )
    };
    writeln!(out, "fn __palette(&self{callback_arg}) -> __IcePalette {{").unwrap();
    if let Some(setting) = &document.settings.palette {
        let value = expr_code(&setting.value, &callback_env, document, ValueMode::Owned)?;
        let contract = generated_named_rust(&contract.name);
        writeln!(out, "match {value} {{").unwrap();
        for palette in &document.palettes {
            writeln!(
                out,
                "{contract}::{} => {},",
                pascal(&palette.name),
                palette_code(palette)
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
    } else {
        writeln!(out, "{}", palette_code(&document.palettes[0])).unwrap();
    }
    writeln!(out, "}}").unwrap();

    let palette_field = |name: &str| {
        let index = contract
            .tokens
            .iter()
            .position(|token| token == name)
            .expect("checker requires native palette tokens");
        format!("__ice_palette.colors[{index}]")
    };
    let callback_value = if document.daemon { "window" } else { "" };
    writeln!(
        out,
        "fn __app_theme(__ice_palette: __IcePalette) -> ::iced::Theme {{"
    )
    .unwrap();
    writeln!(
        out,
        "::iced::Theme::custom(::std::format!(\"{}/{{}}\", __ice_palette.name), ::iced::theme::Palette {{",
        document.app,
    )
    .unwrap();
    writeln!(out, "background: {},", palette_field("bg")).unwrap();
    writeln!(out, "text: {},", palette_field("fg")).unwrap();
    writeln!(out, "primary: {},", palette_field("primary")).unwrap();
    writeln!(out, "success: {},", palette_field("primary")).unwrap();
    writeln!(out, "warning: {},", palette_field("danger")).unwrap();
    writeln!(out, "danger: {},", palette_field("danger")).unwrap();
    writeln!(out, "}})\n}}").unwrap();
    writeln!(out, "fn __theme(&self{callback_arg}) -> ::iced::Theme {{").unwrap();
    if let Some(setting) = &document.settings.theme {
        if let Expr::Call { name, args } = &setting.value
            && document
                .functions
                .iter()
                .any(|function| function.name == *name && function.kind == ExternKind::Theme)
        {
            writeln!(
                out,
                "{}",
                theme_factory_code(name, args, &callback_env, document)?
            )
            .unwrap();
        } else {
            let value = expr_code(&setting.value, &callback_env, document, ValueMode::Owned)?;
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
    } else {
        writeln!(out, "Self::__app_theme(self.__palette({callback_value}))").unwrap();
    }
    writeln!(out, "}}").unwrap();
    if let Some(setting) = &document.settings.title {
        let value = expr_code(&setting.value, &callback_env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __title(&self{callback_arg}) -> ::std::string::String {{ {value} }}"
        )
        .unwrap();
    }
    if document.settings.background.is_some() || document.settings.text_color.is_some() {
        writeln!(out, "fn __style(&self, __theme: &::iced::Theme) -> ::iced::theme::Style {{ let mut __style = ::iced::theme::Base::base(__theme);").unwrap();
        for (setting, field) in [
            (&document.settings.background, "background_color"),
            (&document.settings.text_color, "text_color"),
        ] {
            if let Some(setting) = setting {
                let value = expr_code(&setting.value, &state_env, document, ValueMode::Owned)?;
                writeln!(out, "__style.{field} = ({value}).parse::<::iced::Color>().unwrap_or(__style.{field});").unwrap();
            }
        }
        writeln!(out, "__style }}").unwrap();
    }
    if let Some(setting) = &document.settings.scale_factor {
        let value = expr_code(&setting.value, &callback_env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __scale_factor(&self{callback_arg}) -> f32 {{ (({value}) as f32).max(f32::EPSILON).min(f32::MAX) }}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_boot(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(&document.app);
    writeln!(out, "fn __state() -> Self {{").unwrap();
    for (node, test_only) in document_pane_grids(document) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            templates,
            ..
        } = node
        else {
            unreachable!()
        };
        let field = pane_field(name);
        if test_only {
            writeln!(out, "#[cfg(test)]").unwrap();
        }
        writeln!(
            out,
            "let {field} = ::iced::widget::pane_grid::State::with_configuration({});",
            pane_configuration_code(
                configuration,
                (!templates.is_empty()).then(|| pane_type(name)).as_deref()
            )
        )
        .unwrap();
        let slots = pane_split_slots(configuration);
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
                pane_splits_field(name)
            )
            .unwrap();
        }
    }
    writeln!(out, "Self {{").unwrap();
    let accessibility_bridge = if document.daemon {
        "::ui_lang_runtime::Bridge::without_native_adapter()"
    } else {
        "::ui_lang_runtime::Bridge::new()"
    };
    writeln!(out, "__ice_accessibility: {accessibility_bridge},").unwrap();
    if !document.daemon {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\n__ice_accessibility_initial: ::std::option::Option::None,\n#[cfg(all(target_os = \"windows\", not(test)))]\n__ice_accessibility_pending: ::std::vec::Vec::new(),"
        )
        .unwrap();
    }
    for state in &document.states {
        writeln!(out, "{}: {},", state.name, initial_code(state, document)).unwrap();
    }
    for component in document
        .components
        .iter()
        .filter(|component| !component.states.is_empty() || !component.handlers.is_empty())
    {
        let initial = match component.lifetime {
            ComponentLifetime::Retained => "::std::collections::HashMap::new()",
            ComponentLifetime::Mounted => "::ui_lang_runtime::MountedComponentState::default()",
        };
        writeln!(
            out,
            "{}: {initial},",
            component_state_field(&component.name),
        )
        .unwrap();
    }
    for (node, test_only) in document_pane_grids(document) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            ..
        } = node
        else {
            unreachable!()
        };
        if test_only {
            writeln!(out, "#[cfg(test)]").unwrap();
        }
        writeln!(out, "{},", pane_field(name)).unwrap();
        if pane_split_slots(configuration).iter().any(Option::is_some) {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(out, "{},", pane_splits_field(name)).unwrap();
        }
    }
    writeln!(out, "}}\n}}").unwrap();
    let mount = document
        .handlers
        .iter()
        .find(|handler| handler.name == "mount")
        .map_or(&[][..], |handler| handler.statements.as_slice());
    generate_initial_task_method(out, document, message, "__boot_task", mount)?;
    if document.daemon {
        writeln!(
            out,
            "fn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\nlet task = state.__boot_task();\n(state, task)\n}}"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\nfn __accessibility_attach() -> ::iced::Task<{message}> {{\n::iced::window::oldest().then(|__id| match __id {{\n::std::option::Option::Some(__id) => ::ui_lang_runtime::native_window(__id).map({message}::__AccessibilityNativeWindow),\n::std::option::Option::None => ::iced::Task::none(),\n}})\n}}\nfn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\n#[cfg(all(target_os = \"windows\", not(test)))]\n{{\nstate.__ice_accessibility_initial = ::std::option::Option::Some(0);\n(state, Self::__accessibility_attach())\n}}\n#[cfg(not(all(target_os = \"windows\", not(test))))]\n{{\nlet task = state.__boot_task();\nlet __accessibility = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\n(state, ::iced::Task::batch([task, __accessibility]))\n}}\n}}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_presets(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(&document.app);
    for (index, preset) in document.presets.iter().enumerate() {
        let task_name = format!("__preset_task_{index}");
        generate_initial_task_method(out, document, message, &task_name, &preset.statements)?;
        writeln!(
            out,
            "fn __preset_{index}() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();"
        )
        .unwrap();
        if document.daemon {
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
    if !document.daemon {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\nfn __accessibility_initial_task(&mut self) -> ::iced::Task<{message}> {{\nmatch self.__ice_accessibility_initial.take() {{\n::std::option::Option::Some(0) => self.__boot_task(),"
        )
        .unwrap();
        for index in 0..document.presets.len() {
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
    document: &Document,
    message: &str,
    name: &str,
    statements: &[Statement],
) -> Result<(), Error> {
    writeln!(
        out,
        "fn {name}(&mut self) -> ::iced::Task<{message}> {{\nlet task = (|| {{"
    )
    .unwrap();
    let env = state_env(document, "self");
    let has_task = generate_statements(out, statements, document, message, &env, "self", false)?;
    if !has_task {
        writeln!(out, "::iced::Task::none()").unwrap();
    }
    writeln!(out, "}})();\ntask\n}}").unwrap();
    Ok(())
}

pub(in crate::codegen) fn generate_update(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(&document.app);
    let has_fallthrough_arm = document
        .handlers
        .iter()
        .any(|handler| handler.name != "mount")
        || document.components.iter().any(|component| {
            !component.handlers.is_empty()
                || component.states.iter().any(|state| state.ty == Type::Str)
        })
        || document_pane_grids(document).into_iter().any(|(node, _)| {
            matches!(node, ViewNode::PaneGrid { options, .. } if options.resize_leeway.is_some() || options.draggable)
        })
        || !controlled_state_bindings(document, false)
            .expect("checker validates controlled input bindings")
            .is_empty()
        || !controlled_state_bindings(document, true)
            .expect("checker validates controlled editor bindings")
            .is_empty()
        || needs_extern_noop(document);
    let task_binding = if has_fallthrough_arm {
        "let __task = "
    } else {
        ""
    };
    let windows_show = document
        .settings
        .window
        .as_ref()
        .is_none_or(|settings| settings.visible != Some(false));
    let windows_fullscreen = document
        .settings
        .window
        .as_ref()
        .is_some_and(|settings| settings.fullscreen == Some(true));
    let windows_maximized = document
        .settings
        .window
        .as_ref()
        .is_some_and(|settings| settings.maximized == Some(true));
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
    if !document.daemon {
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
    if document.daemon {
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
    for handler in &document.handlers {
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
        let mut env = state_env(document, "self");
        for param in &handler.params {
            env.insert(
                param.name.clone(),
                Binding {
                    code: param.name.clone(),
                    ty: param.ty.clone(),
                    local: true,
                    state: None,
                },
            );
        }
        let has_task = generate_statements(
            out,
            &handler.statements,
            document,
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
    for component in &document.components {
        let field = component_state_field(&component.name);
        let values = match component.lifetime {
            ComponentLifetime::Retained => format!("self.{field}"),
            ComponentLifetime::Mounted => format!("self.{field}.values()"),
        };
        let entry = |scope: &str| match component.lifetime {
            ComponentLifetime::Retained => {
                format!("let __local = self.{field}.entry({scope}).or_default();")
            }
            ComponentLifetime::Mounted => format!(
                "let mut __states = self.{field}.values_mut(); let __local = __states.entry({scope}).or_default();"
            ),
        };
        for line in component_generation_lines(component) {
            let generation = component_latest_field(line);
            let variant = component_latest_variant(&component.name, line);
            writeln!(
                out,
                "{message}::{variant}(__scope, __generation, __message) => {{ if {values}.get(&__scope).is_some_and(|__local| __local.{generation} == __generation) {{ return self.__update(*__message); }} return ::iced::Task::none(); }},"
            )
            .unwrap();
        }
        for handler in &component.handlers {
            let variant = component_handler_variant(&component.name, &handler.name);
            let mut bindings = vec!["__scope".to_owned()];
            bindings.extend(handler.params.iter().map(|param| param.name.clone()));
            writeln!(
                out,
                "{message}::{variant}({}) => (|| {{",
                bindings.join(", ")
            )
            .unwrap();
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
            let mut env = HashMap::new();
            for state in &component.states {
                env.insert(
                    state.name.clone(),
                    Binding {
                        code: format!("__local.{}", state.name),
                        ty: state.ty.clone(),
                        local: false,
                        state: None,
                    },
                );
            }
            for param in &handler.params {
                env.insert(
                    param.name.clone(),
                    Binding {
                        code: param.name.clone(),
                        ty: param.ty.clone(),
                        local: true,
                        state: None,
                    },
                );
            }
            env.insert(
                component_context_key(&component.name),
                Binding {
                    code: if future.is_some() {
                        "__route_scope".into()
                    } else {
                        "__scope".into()
                    },
                    ty: Type::Unit,
                    local: true,
                    state: None,
                },
            );
            let has_task = generate_statements(
                out,
                &handler.statements,
                document,
                message,
                &env,
                "__local",
                future.is_none(),
            )?;
            if let Some((mode, line)) = future {
                debug_assert!(has_task);
                writeln!(out, "}};").unwrap();
                match mode {
                    FutureMode::Every => writeln!(out, "__task").unwrap(),
                    FutureMode::Latest | FutureMode::Replace => {
                        let generation = component_latest_field(line);
                        let future_variant = component_latest_variant(&component.name, line);
                        match component.lifetime {
                            ComponentLifetime::Retained => writeln!(out, "let __generation = {{ {} __local.{generation} = __local.{generation}.wrapping_add(1); __local.{generation} }};", entry("__scope.clone()")).unwrap(),
                            ComponentLifetime::Mounted => writeln!(out, "let __generation = self.{field}.next_generation(); {{ {} __local.{generation} = __generation; }}", entry("__scope.clone()")).unwrap(),
                        }
                        if mode == FutureMode::Replace {
                            let replace = component_replace_field(line);
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
    for (node, test_only) in document_pane_grids(document) {
        let ViewNode::PaneGrid { name, options, .. } = node else {
            unreachable!()
        };
        if options.resize_leeway.is_some() {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{message}::{}(__event) => {{ self.{}.resize(__event.split, __event.ratio); ::iced::Task::none() }},",
                pane_resize_variant(name),
                pane_field(name)
            )
            .unwrap();
        }
        if options.draggable {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{message}::{}(__event) => {{ if let ::iced::widget::pane_grid::DragEvent::Dropped {{ pane, target }} = __event {{ self.{}.drop(pane, target); }} ::iced::Task::none() }},",
                pane_drag_variant(name),
                pane_field(name)
            )
            .unwrap();
        }
    }
    for binding in controlled_state_bindings(document, false)
        .expect("checker validates controlled input bindings")
    {
        let variant = binding_variant(&binding);
        writeln!(
            out,
            "{message}::{variant}(value) => {{ self.{binding} = value; ::iced::Task::none() }}"
        )
        .unwrap();
    }
    for binding in controlled_state_bindings(document, true)
        .expect("checker validates controlled editor bindings")
    {
        let variant = editor_variant(&binding);
        writeln!(
            out,
            "{message}::{variant}(action) => {{ self.{binding}.perform(action); ::iced::Task::none() }}"
        )
        .unwrap();
    }
    if needs_extern_noop(document) {
        writeln!(out, "{message}::__ExternNoop => ::iced::Task::none(),").unwrap();
    }
    if has_animations(document) {
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
    if document.daemon {
        writeln!(out, "}};\n__task\n}}").unwrap();
    } else {
        writeln!(
            out,
            "}};\n::iced::Task::batch([__task, ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))])\n}}"
        )
        .unwrap();
    }
    Ok(())
}
