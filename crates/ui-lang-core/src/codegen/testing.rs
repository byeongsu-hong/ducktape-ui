use super::*;

pub(in crate::codegen) fn generate_test_mounts(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    let daemon = program.settings().kind == ProgramKind::Daemon;
    let render_document = RenderDocument::new(program);
    let presets = if program.preset_names().is_empty() {
        String::new()
    } else {
        format!(
            ".presets([{}])",
            program
                .preset_names()
                .iter()
                .enumerate()
                .map(|(index, preset)| format!(
                    "::iced::Preset::new({}, Self::__preset_{index})",
                    rust_string(preset)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    for test in program.tests() {
        let Some(mount) = program.test_mount(test.id) else {
            continue;
        };
        let index = test.id.0 as usize;
        let mut env = checked_state_env(program, "self");
        if daemon {
            env.insert(
                "window".into(),
                Binding {
                    code: "window".into(),
                    ty: Type::WindowId,
                    local: true,
                    state: None,
                    owner: test.window_local.map(BindingOwner::Local),
                },
            );
        }
        let root = render_node_if_present(
            mount,
            &render_document,
            message,
            &env,
            &rust_string(program.app_name()),
            None,
        )?
        .unwrap_or_else(|| "::iced::widget::Column::new().into()".into());
        let window_arg = if daemon {
            ", window: ::iced::window::Id"
        } else {
            ""
        };
        let callback_value = if daemon { "window" } else { "" };
        let palette = format!(
            "let __ice_palette = self.__palette({callback_value}); let __ice_app_theme = Self::__app_theme(__ice_palette);"
        );
        if daemon {
            writeln!(
                out,
                "#[cfg(test)]\nfn __ice_test_mount_{index}(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} {root} }}"
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "#[cfg(test)]\nfn __ice_test_mount_{index}(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_content: __IceElement<'_, {message}> = {root}; ::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext, {message}::__AccessibilityFocusPrevious).into() }}"
            )
            .unwrap();
        }
        let test_program =
            test_program_code(program, program.settings(), source_path, index, &presets);
        let program_ty = if daemon {
            "::iced::Daemon"
        } else {
            "::iced::Application"
        };
        writeln!(
            out,
            "#[cfg(test)]\nfn __ice_test_program_{index}() -> {program_ty}<impl ::iced::Program<State = Self, Message = {message}, Theme = ::iced::Theme>> {{ {test_program} }}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_tests(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    writeln!(out, "#[cfg(test)]\nmod __ice_tests {{\nuse super::*;").unwrap();
    writeln!(
        out,
        "#[test]\nfn __ice_agent_inspect() {{ ::ui_lang_runtime::testing::agent_inspect(|| {}::__program(), {}); }}",
        program.app_name(),
        rust_string(source_path),
    )
    .unwrap();
    for test in program.tests() {
        generate_test(out, program, message, source_path, test)?;
    }
    writeln!(out, "}}").unwrap();
    Ok(())
}

fn generate_test(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
    test: &ResolvedTest,
) -> Result<(), Error> {
    use ResolvedTestStepKind as TestStepKind;

    let expr_code =
        |value: &ResolvedExpressionId, env: &HashMap<String, Binding>, mode: ValueMode| {
            checked_expr_use_code(program, *value, env, mode)
        };
    let expr_list_code = |values: &[ResolvedExpressionId], env: &HashMap<String, Binding>| {
        values
            .iter()
            .map(|value| checked_expr_use_code(program, *value, env, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.join(", "))
    };
    let index = test.id.0 as usize;
    writeln!(out, "#[test]\nfn {}() {{", test.name).unwrap();
    let declaration = format!("test {}", test.name);
    let source = location_code(program, source_path, test.origin, &declaration);
    let mut config = format!(
        "::ui_lang_runtime::testing::Config::new({}).source({source})",
        rust_string(&test.name),
    );
    if let Some((width, height)) = test.config.viewport {
        write!(
            config,
            ".viewport({}f32, {}f32)",
            rust_f64(width),
            rust_f64(height)
        )
        .unwrap();
    }
    if let Some(timeout_ms) = test.config.timeout_ms {
        write!(
            config,
            ".timeout(::std::time::Duration::from_millis({timeout_ms}))"
        )
        .unwrap();
    }
    if let Some(theme) = test.config.theme {
        let theme = test_theme_variant(theme);
        write!(
            config,
            ".theme(::ui_lang_runtime::testing::ThemeMode::{theme})"
        )
        .unwrap();
    }
    if let Some(scale_factor) = test.config.scale_factor {
        write!(config, ".scale_factor({}f32)", rust_f64(scale_factor)).unwrap();
    }
    if let Some(locale) = &test.config.locale {
        write!(config, ".locale({})", rust_string(locale)).unwrap();
    }
    if let Some(platform) = test.config.platform {
        let platform = match platform {
            ResolvedTestPlatform::Linux => "Linux",
            ResolvedTestPlatform::Windows => "Windows",
            ResolvedTestPlatform::Macos => "Macos",
            ResolvedTestPlatform::Wasm => "Wasm",
        };
        write!(
            config,
            ".platform(::ui_lang_runtime::testing::Platform::{platform})"
        )
        .unwrap();
    }
    if let Some(reduced_motion) = test.config.reduced_motion {
        write!(config, ".reduced_motion({reduced_motion})").unwrap();
    }
    if let Some(preset) = &test.config.preset {
        write!(config, ".preset({})", rust_string(preset)).unwrap();
    }
    writeln!(out, "let __config = {config};").unwrap();
    let test_program = if test.mount.is_some() {
        format!("{}::__ice_test_program_{index}()", program.app_name())
    } else {
        format!("{}::__program()", program.app_name())
    };
    writeln!(
        out,
        "let mut __test = ::ui_lang_runtime::testing::Driver::new({test_program}, __config);"
    )
    .unwrap();

    for (step_index, step) in test.steps.iter().enumerate() {
        if step.id
            != (TestStepId {
                test: test.id,
                index: step_index as u32,
            })
        {
            return Err(program.invariant_at_origin(
                step.origin,
                "resolved test step identity changed before code generation",
            ));
        }
        let location = location_code(program, source_path, step.origin, &step.source);
        let mut env = checked_state_env(program, "__test.state()");
        if program.settings().kind == ProgramKind::Daemon {
            env.insert(
                "window".into(),
                Binding {
                    code: "__test.window()".into(),
                    ty: Type::WindowId,
                    local: true,
                    state: None,
                    owner: test.window_local.map(BindingOwner::Local),
                },
            );
        }
        for (target_index, target) in test.targets.iter().enumerate() {
            if target.id
                != (TestTargetId {
                    test: test.id,
                    index: target_index as u32,
                })
            {
                return Err(program.invariant_at_origin(
                    target.origin,
                    "resolved test target identity changed before code generation",
                ));
            }
            let path = resolved_test_target_path_code(&target.path, &env, program)?;
            env.insert(
                target.name.clone(),
                Binding {
                    code: format!(
                        "{{ let __target_path = {path}; __test.target(&__target_path, {location}) }}"
                    ),
                    ty: Type::TestTarget,
                    local: true,
                    state: None,
                    owner: Some(BindingOwner::Local(target.local)),
                },
            );
        }
        writeln!(
            out,
            "::ui_lang_runtime::testing::step({}, {location}, || {{",
            rust_string(&test.name)
        )
        .unwrap();
        match &step.kind {
            TestStepKind::Click {
                target,
                button,
                count,
            } => {
                let path = target_ref_path_code(target, test, &env, program)?;
                let button = test_mouse_button_code(*button);
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Click {{ target: __target.to_owned(), button: {button}, count: {count} }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::ClickAt {
                x,
                y,
                button,
                count,
            } => {
                let x = expr_code(x, &env, ValueMode::Owned)?;
                let y = expr_code(y, &env, ValueMode::Owned)?;
                let button = test_mouse_button_code(*button);
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::ClickAt {{ position: ::iced::Point::new(__x, __y), button: {button}, count: {count} }}, {location});").unwrap();
            }
            TestStepKind::Hover(target) => {
                let path = target_ref_path_code(target, test, &env, program)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::MoveTo(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Enter(target) => {
                let path = target_ref_path_code(target, test, &env, program)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Enter(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Leave => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Leave, {location});").unwrap();
            }
            TestStepKind::MoveTarget(target) => {
                let path = target_ref_path_code(target, test, &env, program)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::MoveTo(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::MovePoint(x, y) => {
                let x = expr_code(x, &env, ValueMode::Owned)?;
                let y = expr_code(y, &env, ValueMode::Owned)?;
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::MoveToPoint(::iced::Point::new(__x, __y)), {location});").unwrap();
            }
            TestStepKind::Press { target, button } => {
                let path = target_ref_path_code(target, test, &env, program)?;
                let button = test_mouse_button_code(*button);
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Press {{ target: __target.to_owned(), button: {button} }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::Release(button) => {
                let button = test_mouse_button_code(*button);
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Release({button}), {location});").unwrap();
            }
            TestStepKind::Wheel { unit, x, y } => {
                let x = expr_code(x, &env, ValueMode::Owned)?;
                let y = expr_code(y, &env, ValueMode::Owned)?;
                let delta = match unit {
                    ResolvedTestWheelUnit::Pixels => "Pixels",
                    ResolvedTestWheelUnit::Lines => "Lines",
                };
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Wheel(::ui_lang_runtime::testing::WheelDelta::{delta} {{ x: __x, y: __y }}), {location});").unwrap();
            }
            TestStepKind::Scroll { mode, target, x, y } => {
                let path = target_ref_path_code(target, test, &env, program)?;
                let x = expr_code(x, &env, ValueMode::Owned)?;
                let y = expr_code(y, &env, ValueMode::Owned)?;
                let action = match mode {
                    ResolvedTestScrollMode::To => "ScrollTo",
                    ResolvedTestScrollMode::By => "ScrollBy",
                };
                writeln!(out, "let __target = {path}; let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::{action} {{ target: __target.to_owned(), x: __x, y: __y }}, {location});").unwrap();
            }
            TestStepKind::Snap { target, x, y } => {
                let path = target_ref_path_code(target, test, &env, program)?;
                let x = expr_code(x, &env, ValueMode::Owned)?;
                let y = expr_code(y, &env, ValueMode::Owned)?;
                writeln!(out, "let __target = {path}; let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Snap {{ target: __target.to_owned(), x: __x, y: __y }}, {location});").unwrap();
            }
            TestStepKind::SnapEnd(target) => {
                let path = target_ref_path_code(target, test, &env, program)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::SnapEnd(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Drag { from, to } => {
                let from = target_ref_path_code(from, test, &env, program)?;
                let to = target_ref_path_code(to, test, &env, program)?;
                writeln!(
                    out,
                    "let __from = {from}; let __to = {to}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Drag {{ from: __from.to_owned(), to: __to.to_owned() }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::Drop(target) => {
                let path = target_ref_path_code(target, test, &env, program)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::DropAt(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Focus(target) => {
                let path = target_ref_path_code(target, test, &env, program)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Focus(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::FocusNext => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::FocusNext, {location});").unwrap();
            }
            TestStepKind::FocusPrevious => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::FocusPrevious, {location});").unwrap();
            }
            TestStepKind::Blur => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Blur, {location});").unwrap();
            }
            TestStepKind::WindowFocus(focused) => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::WindowFocus({focused}), {location});").unwrap();
            }
            TestStepKind::Type(value) => {
                let value = expr_code(value, &env, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __value = {value}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Type(__value), {location});"
                )
                .unwrap();
            }
            TestStepKind::Clear => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Clear, {location});").unwrap();
            }
            TestStepKind::Replace(value) => {
                let value = expr_code(value, &env, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __value = {value}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Replace(__value), {location});"
                )
                .unwrap();
            }
            TestStepKind::Select(start, end) => {
                let start = expr_code(start, &env, ValueMode::Owned)?;
                let end = expr_code(end, &env, ValueMode::Owned)?;
                writeln!(out, "let __start = ::std::primitive::usize::try_from({start}).expect(\"selection start must fit usize\"); let __end = ::std::primitive::usize::try_from({end}).expect(\"selection end must fit usize\"); let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Select {{ start: __start, end: __end }}, {location});").unwrap();
            }
            TestStepKind::SelectAll => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::SelectAll, {location});").unwrap();
            }
            TestStepKind::Cursor(index) => {
                let index = expr_code(index, &env, ValueMode::Owned)?;
                writeln!(out, "let __index = ::std::primitive::usize::try_from({index}).expect(\"cursor index must fit usize\"); let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Cursor(__index), {location});").unwrap();
            }
            TestStepKind::CursorFront => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::CursorFront, {location});").unwrap();
            }
            TestStepKind::CursorEnd => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::CursorEnd, {location});").unwrap();
            }
            TestStepKind::Composition(composition) => {
                let composition = test_composition_code(composition, &env, program)?;
                writeln!(out, "let __composition = {composition}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Composition(__composition), {location});").unwrap();
            }
            TestStepKind::Key(key) => {
                let key = test_key_code(key);
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Key({key}), {location});").unwrap();
            }
            TestStepKind::KeyDown(event) | TestStepKind::KeyUp(event) => {
                let key = test_key_code(&event.key);
                let metadata = test_key_metadata_code(event);
                let action = if matches!(&step.kind, TestStepKind::KeyDown(_)) {
                    "KeyDown"
                } else {
                    "KeyUp"
                };
                writeln!(out, "let __key = {key}; let __metadata = {metadata}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::{action} {{ key: __key, metadata: __metadata }}, {location});").unwrap();
            }
            TestStepKind::Modifiers(modifiers) => {
                let modifiers = test_modifiers_code(*modifiers);
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Modifiers({modifiers}), {location});").unwrap();
            }
            TestStepKind::Chord { modifiers, key } => {
                let modifiers = test_modifiers_code(*modifiers);
                let key = test_key_code(key);
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Chord {{ modifiers: {modifiers}, key: {key} }}, {location});").unwrap();
            }
            TestStepKind::Repeat { key, count } => {
                let key = test_key_code(key);
                let count = expr_code(count, &env, ValueMode::Owned)?;
                writeln!(out, "let __count = ::std::primitive::usize::try_from({count}).expect(\"repeat count must fit usize\"); let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Repeat {{ key: {key}, count: __count }}, {location});").unwrap();
            }
            TestStepKind::Tap { target, count } => {
                let path = target_ref_path_code(target, test, &env, program)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Tap {{ target: __target.to_owned(), count: {count} }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::Touch { phase, id, x, y } => {
                let phase = match phase {
                    ResolvedTestTouchPhase::Down => "Down",
                    ResolvedTestTouchPhase::Move => "Move",
                    ResolvedTestTouchPhase::Up => "Up",
                    ResolvedTestTouchPhase::Cancel => "Cancel",
                };
                let id = expr_code(id, &env, ValueMode::Owned)?;
                let x = expr_code(x, &env, ValueMode::Owned)?;
                let y = expr_code(y, &env, ValueMode::Owned)?;
                writeln!(out, "let __id = ::std::primitive::u64::try_from({id}).expect(\"touch id must fit u64\"); let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Touch {{ phase: ::ui_lang_runtime::testing::TouchPhase::{phase}, id: __id, position: ::iced::Point::new(__x, __y) }}, {location});").unwrap();
            }
            TestStepKind::WindowMove(x, y) => {
                let x = expr_code(x, &env, ValueMode::Owned)?;
                let y = expr_code(y, &env, ValueMode::Owned)?;
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::WindowMove(::iced::Point::new(__x, __y)), {location});").unwrap();
            }
            TestStepKind::Resize(width, height) => {
                let width = expr_code(width, &env, ValueMode::Owned)?;
                let height = expr_code(height, &env, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __width = ({width}) as f32; let __height = ({height}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Resize(::iced::Size::new(__width, __height)), {location});"
                )
                .unwrap();
            }
            TestStepKind::Rescale(value) => {
                let value = expr_code(value, &env, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __scale = ({value}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Rescale(__scale), {location});"
                )
                .unwrap();
            }
            TestStepKind::WindowClose => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::CloseRequested, {location});").unwrap();
            }
            TestStepKind::WindowOpened => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::WindowOpened, {location});").unwrap();
            }
            TestStepKind::WindowClosed => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::WindowClosed, {location});").unwrap();
            }
            TestStepKind::Redraw => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Redraw, {location});").unwrap();
            }
            TestStepKind::SystemTheme(theme) => {
                let theme = test_theme_variant(*theme);
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::SystemTheme(::ui_lang_runtime::testing::ThemeMode::{theme}), {location});").unwrap();
            }
            TestStepKind::FileHover(value) | TestStepKind::FileDrop(value) => {
                let value = expr_code(value, &env, ValueMode::Owned)?;
                let action = if matches!(&step.kind, TestStepKind::FileHover(_)) {
                    "FileHover"
                } else {
                    "FileDrop"
                };
                writeln!(
                    out,
                    "let __path = {value}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::{action}(::std::path::PathBuf::from(__path)), {location});"
                )
                .unwrap();
            }
            TestStepKind::FileLeave => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::FileLeave, {location});").unwrap();
            }
            TestStepKind::Wait(duration) | TestStepKind::Advance(duration) => {
                let action = if matches!(&step.kind, TestStepKind::Wait(_)) {
                    "Wait"
                } else {
                    "Advance"
                };
                writeln!(
                    out,
                    "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::{action}(::std::time::Duration::from_millis({duration})), {location});"
                )
                .unwrap();
            }
            TestStepKind::Idle => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Idle, {location});").unwrap();
            }
            TestStepKind::Capture(name) => {
                writeln!(
                    out,
                    "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Capture({}.to_owned()), {location});",
                    rust_string(name)
                )
                .unwrap();
            }
            TestStepKind::Accessibility { action, target } => {
                let path = target_ref_path_code(target, test, &env, program)?;
                let action = match action {
                    ResolvedTestAccessibilityAction::Activate => "Click",
                    ResolvedTestAccessibilityAction::Focus => "Focus",
                };
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Accessibility {{ action: ::ui_lang_runtime::testing::AccessibilityAction::{action}, target: __target.to_owned() }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::Dispatch {
                handler,
                handler_name,
                args,
            } => {
                let declaration = program.handler(*handler);
                if declaration.name != *handler_name {
                    return Err(program.invariant_at_origin(
                        step.origin,
                        "test dispatch handler identity changed before code generation",
                    ));
                }
                let variant = handler_variant(handler_name);
                let args = expr_list_code(args, &env)?;
                let value = if args.is_empty() {
                    format!("{message}::{variant}")
                } else {
                    format!("{message}::{variant}({args})")
                };
                writeln!(
                    out,
                    "let __message = {value}; __test.dispatch(__message, {location});"
                )
                .unwrap();
            }
            TestStepKind::Expect(expectation) => {
                generate_expectation(out, expectation, test, &env, program, &location)?;
            }
        }
        writeln!(out, "}});").unwrap();
    }
    writeln!(out, "}}").unwrap();
    Ok(())
}

fn generate_expectation(
    out: &mut String,
    expectation: &ResolvedTestExpectation,
    test: &ResolvedTest,
    env: &HashMap<String, Binding>,
    program: &LoweredProgram,
    location: &str,
) -> Result<(), Error> {
    match expectation {
        ResolvedTestExpectation::Equality {
            left,
            right,
            negated,
            expression,
        } => {
            let left = checked_expr_node_code(program, *expression, *left, env, ValueMode::Owned)?;
            let right =
                checked_expr_node_code(program, *expression, *right, env, ValueMode::Owned)?;
            let method = if *negated { "check_ne" } else { "check_eq" };
            writeln!(
                out,
                "let __left = {left}; let __right = {right}; __test.{method}(__left, __right, {location});"
            )
            .unwrap();
        }
        ResolvedTestExpectation::Expr {
            expression,
            unwrap_operator,
        } => {
            let mut code = checked_expr_use_code(program, *expression, env, ValueMode::Owned)?;
            if *unwrap_operator {
                code = code
                    .strip_prefix('(')
                    .and_then(|code| code.strip_suffix(')'))
                    .expect("unary and binary expressions are parenthesized")
                    .to_owned();
            }
            writeln!(
                out,
                "let __actual = {code}; __test.check(__actual, {location});"
            )
            .unwrap();
        }
        ResolvedTestExpectation::Approx { left, right } => {
            let left = checked_expr_use_code(program, *left, env, ValueMode::Owned)?;
            let right = checked_expr_use_code(program, *right, env, ValueMode::Owned)?;
            writeln!(
                out,
                "let __left = ({left}) as f64; let __right = ({right}) as f64; __test.check_approx(__left, __right, {location});"
            )
            .unwrap();
        }
        ResolvedTestExpectation::Exists(target) | ResolvedTestExpectation::Missing(target) => {
            let path = target_ref_path_code(target, test, env, program)?;
            let expected = matches!(expectation, ResolvedTestExpectation::Exists(_));
            writeln!(
                out,
                "let __target = {path}; __test.check_exists(&__target, {expected}, {location});"
            )
            .unwrap();
        }
        ResolvedTestExpectation::Text {
            value,
            within,
            negated,
        } => {
            let value = checked_expr_use_code(program, *value, env, ValueMode::Owned)?;
            if let Some(within) = within {
                let path = target_ref_path_code(within, test, env, program)?;
                writeln!(
                    out,
                    "let __value = {value}; let __within = {path}; __test.check_text(&__value, ::std::option::Option::Some(&__within), {negated}, {location});"
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "let __value = {value}; __test.check_text(&__value, ::std::option::Option::None, {negated}, {location});"
                )
                .unwrap();
            }
        }
        ResolvedTestExpectation::Accessibility { target, property } => {
            let path = target_ref_path_code(target, test, env, program)?;
            match property {
                ResolvedTestAccessibilityProperty::Role(value)
                | ResolvedTestAccessibilityProperty::Name(value)
                | ResolvedTestAccessibilityProperty::Value(value) => {
                    let property = match property {
                        ResolvedTestAccessibilityProperty::Role(_) => "Role",
                        ResolvedTestAccessibilityProperty::Name(_) => "Name",
                        ResolvedTestAccessibilityProperty::Value(_) => "Value",
                        _ => unreachable!(),
                    };
                    let value = checked_expr_use_code(program, *value, env, ValueMode::Owned)?;
                    writeln!(out, "let __target = {path}; let __expected = {value}; __test.check_accessibility_str(&__target, ::ui_lang_runtime::testing::AccessibilityProperty::{property}, &__expected, {location});").unwrap();
                }
                ResolvedTestAccessibilityProperty::Checked(value)
                | ResolvedTestAccessibilityProperty::Disabled(value)
                | ResolvedTestAccessibilityProperty::Focused(value) => {
                    let property = match property {
                        ResolvedTestAccessibilityProperty::Checked(_) => "Checked",
                        ResolvedTestAccessibilityProperty::Disabled(_) => "Disabled",
                        ResolvedTestAccessibilityProperty::Focused(_) => "Focused",
                        _ => unreachable!(),
                    };
                    let value = checked_expr_use_code(program, *value, env, ValueMode::Owned)?;
                    writeln!(out, "let __target = {path}; let __expected = {value}; __test.check_accessibility_bool(&__target, ::ui_lang_runtime::testing::AccessibilityProperty::{property}, __expected, {location});").unwrap();
                }
                ResolvedTestAccessibilityProperty::Action { name, expected } => {
                    let action = accessibility_action_variant(name);
                    let expected =
                        checked_expr_use_code(program, *expected, env, ValueMode::Owned)?;
                    writeln!(out, "let __target = {path}; let __expected = {expected}; __test.check_accessibility_action(&__target, ::ui_lang_runtime::testing::AccessibilityAction::{action}, __expected, {location});").unwrap();
                }
            }
        }
    }
    Ok(())
}

fn test_theme_variant(theme: ResolvedTestTheme) -> &'static str {
    match theme {
        ResolvedTestTheme::Light => "Light",
        ResolvedTestTheme::Dark => "Dark",
        ResolvedTestTheme::None => "None",
    }
}

fn test_mouse_button_code(button: ResolvedTestMouseButton) -> &'static str {
    match button {
        ResolvedTestMouseButton::Left => "::ui_lang_runtime::testing::MouseButton::Left",
        ResolvedTestMouseButton::Right => "::ui_lang_runtime::testing::MouseButton::Right",
        ResolvedTestMouseButton::Middle => "::ui_lang_runtime::testing::MouseButton::Middle",
        ResolvedTestMouseButton::Back => "::ui_lang_runtime::testing::MouseButton::Back",
        ResolvedTestMouseButton::Forward => "::ui_lang_runtime::testing::MouseButton::Forward",
    }
}

fn test_key_code(key: &ResolvedTestKey) -> String {
    match key {
        ResolvedTestKey::Named(name) => format!(
            "::ui_lang_runtime::testing::Key::named(::iced::keyboard::key::Named::{})",
            test_keyboard_variant_name(name)
        ),
        ResolvedTestKey::Character(value) => format!(
            "::ui_lang_runtime::testing::Key::character({})",
            rust_string(value)
        ),
    }
}

fn test_modifiers_code(modifiers: ResolvedTestModifiers) -> String {
    format!(
        "::ui_lang_runtime::testing::Modifiers::new({}, {}, {}, {})",
        modifiers.shift, modifiers.control, modifiers.alt, modifiers.logo
    )
}

fn test_key_metadata_code(event: &ResolvedTestKeyEvent) -> String {
    let modified_key = event.modified_key.as_ref().map_or_else(
        || "::std::option::Option::None".into(),
        |key| format!("::std::option::Option::Some({})", test_key_code(key)),
    );
    let physical = event.physical.as_ref().map_or_else(
        || "::std::option::Option::None".into(),
        |physical| {
            format!(
                "::std::option::Option::Some(::iced::keyboard::key::Physical::Code(::iced::keyboard::key::Code::{}))",
                test_keyboard_variant_name(physical)
            )
        },
    );
    let location = match event.location {
        ResolvedTestKeyLocation::Standard => "Standard",
        ResolvedTestKeyLocation::Left => "Left",
        ResolvedTestKeyLocation::Right => "Right",
        ResolvedTestKeyLocation::Numpad => "Numpad",
    };
    let text = event.text.as_ref().map_or_else(
        || "::std::option::Option::None".into(),
        |value| {
            format!(
                "::std::option::Option::Some({}.to_owned())",
                rust_string(value)
            )
        },
    );
    format!(
        "::ui_lang_runtime::testing::KeyMetadata {{ modified_key: {modified_key}, physical_key: {physical}, location: ::ui_lang_runtime::testing::KeyLocation::{location}, text: {text}, repeat: {} }}",
        event.repeat
    )
}

fn test_composition_code(
    composition: &ResolvedTestComposition,
    env: &HashMap<String, Binding>,
    program: &LoweredProgram,
) -> Result<String, Error> {
    Ok(match composition {
        ResolvedTestComposition::Start => {
            "::ui_lang_runtime::testing::CompositionPhase::Start".into()
        }
        ResolvedTestComposition::Update { value, selection } => {
            let selection = selection.as_ref().map_or_else(
                || Ok("::std::option::Option::None".to_owned()),
                |(start, end)| {
                    Ok::<_, Error>(format!(
                        "::std::option::Option::Some(::std::ops::Range {{ start: ::std::primitive::usize::try_from({}).expect(\"composition selection start must fit usize\"), end: ::std::primitive::usize::try_from({}).expect(\"composition selection end must fit usize\") }})",
                        checked_expr_use_code(program, *start, env, ValueMode::Owned)?,
                        checked_expr_use_code(program, *end, env, ValueMode::Owned)?
                    ))
                },
            )?;
            format!(
                "::ui_lang_runtime::testing::CompositionPhase::Update {{ text: {}, selection: {selection} }}",
                checked_expr_use_code(program, *value, env, ValueMode::Owned)?
            )
        }
        ResolvedTestComposition::Commit(value) => format!(
            "::ui_lang_runtime::testing::CompositionPhase::Commit({})",
            checked_expr_use_code(program, *value, env, ValueMode::Owned)?
        ),
        ResolvedTestComposition::Cancel => {
            "::ui_lang_runtime::testing::CompositionPhase::Cancel".into()
        }
    })
}

fn accessibility_action_variant(name: &str) -> &'static str {
    match name {
        "click" => "Click",
        "focus" => "Focus",
        _ => unreachable!("parser validates accessibility actions"),
    }
}

fn resolved_test_target_path_code(
    target: &ResolvedTestTargetPath,
    env: &HashMap<String, Binding>,
    program: &LoweredProgram,
) -> Result<String, Error> {
    if target.segments.iter().all(|segment| segment.key.is_none()) {
        return Ok(rust_string(&format!(
            "{}/{}",
            program.app_name(),
            target
                .segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("/")
        )));
    }
    let mut scope = rust_string(program.app_name());
    for segment in &target.segments {
        scope = if let Some(key) = segment.key {
            let key = checked_expr_use_code(program, key, env, ValueMode::Borrowed)?;
            format!("format!(\"{{}}/{}({{}})\", {scope}, {key})", segment.name)
        } else {
            format!("format!(\"{{}}/{}\", {scope})", segment.name)
        };
    }
    Ok(scope)
}

fn target_ref_path_code(
    target: &ResolvedTestTargetRef,
    test: &ResolvedTest,
    env: &HashMap<String, Binding>,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let target = match target {
        ResolvedTestTargetRef::Alias(id) => {
            &test
                .targets
                .iter()
                .find(|target| target.id == *id)
                .ok_or_else(|| {
                    program.invariant_at_origin(
                        test.origin,
                        "resolved test target alias is outside its test",
                    )
                })?
                .path
        }
        ResolvedTestTargetRef::Id(target) => target,
    };
    resolved_test_target_path_code(target, env, program)
}

fn location_code(
    program: &LoweredProgram,
    source_path: &str,
    origin: OriginId,
    statement: &str,
) -> String {
    let origin = program.origin(origin);
    let source_path = origin
        .path
        .as_ref()
        .map_or_else(|| source_path.to_owned(), |path| path.display().to_string());
    format!(
        "::ui_lang_runtime::testing::Location::new({}, {}, {}, {})",
        rust_string(&source_path),
        origin.line,
        origin.column,
        rust_string(statement)
    )
}

fn test_program_code(
    program: &LoweredProgram,
    app_settings: &ResolvedAppSettings,
    source_path: &str,
    index: usize,
    presets: &str,
) -> String {
    let subscription = ".subscription(Self::__subscription)";
    let default_font = if app_settings.default_font.is_some() {
        ".default_font(Self::default_font())"
    } else {
        ""
    };
    let title = app_settings
        .title
        .as_ref()
        .map_or("", |_| ".title(Self::__title)");
    let settings = app_settings_code(program, app_settings);
    let fonts = font_assets_code(program, app_settings, source_path);
    let window = if app_settings.kind == ProgramKind::Daemon {
        String::new()
    } else {
        window_settings_code(program, &app_settings.primary_window, source_path)
    };
    let executor = match &app_settings.executor {
        ResolvedExecutorSelection::Default => String::new(),
        ResolvedExecutorSelection::Custom { path, origin } => format!(
            "\n{}\n.executor::<{path}>()\n{SOURCE_MARKER_END}\n",
            source_marker_for_origin(program, *origin)
        ),
    };
    let scale_factor = app_settings
        .scale_factor
        .as_ref()
        .map_or("", |_| ".scale_factor(Self::__scale_factor)");
    let style = if app_settings.background.is_some() || app_settings.text_color.is_some() {
        ".style(Self::__style)"
    } else {
        ""
    };
    let root = if app_settings.kind == ProgramKind::Daemon {
        "::iced::daemon(Self::__boot, Self::__update, Self::__ice_test_mount_"
    } else {
        "::iced::application(Self::__boot, Self::__update, Self::__ice_test_mount_"
    };
    format!(
        "{root}{index}){title}{subscription}.theme(Self::__theme){style}{settings}{default_font}{fonts}{window}{scale_factor}{executor}{presets}"
    )
}
