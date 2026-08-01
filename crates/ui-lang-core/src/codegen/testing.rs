use super::*;

pub(in crate::codegen) fn generate_test_mounts(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    let document = program.document();
    let daemon = program.settings().kind == ProgramKind::Daemon;
    let render_document = RenderDocument::new(program);
    for (index, test) in document.tests.iter().enumerate() {
        let Some(mount) = &test.mount else {
            continue;
        };
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
                        .checked_facts()
                        .daemon_window_local()
                        .map(BindingOwner::Local),
                },
            );
        }
        let root = render_node_if_present(
            mount,
            &render_document,
            message,
            &env,
            &rust_string(&document.app),
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
        let test_program = test_program_code(program, source_path, index);
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
    let document = program.document();
    writeln!(out, "#[cfg(test)]\nmod __ice_tests {{\nuse super::*;").unwrap();
    writeln!(
        out,
        "#[test]\nfn __ice_agent_inspect() {{ ::ui_lang_runtime::testing::agent_inspect(|| {}::__program(), {}); }}",
        document.app,
        rust_string(source_path),
    )
    .unwrap();
    for (index, test) in document.tests.iter().enumerate() {
        generate_test(out, program, message, source_path, index, test)?;
    }
    writeln!(out, "}}").unwrap();
    Ok(())
}

fn generate_test(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
    source_path: &str,
    index: usize,
    test: &TestDecl,
) -> Result<(), Error> {
    let document = program.document();
    writeln!(out, "#[test]\nfn {}() {{", test.name).unwrap();
    let declaration = format!("test {}", test.name);
    let source = location_code(program, source_path, &test.span, &declaration);
    let mut config = format!(
        "::ui_lang_runtime::testing::Config::new({}).source({source})",
        rust_string(&test.name),
    );
    if let Some((width, height)) = test.viewport {
        write!(
            config,
            ".viewport({}f32, {}f32)",
            rust_f64(width),
            rust_f64(height)
        )
        .unwrap();
    }
    if let Some(timeout_ms) = test.timeout_ms {
        write!(
            config,
            ".timeout(::std::time::Duration::from_millis({timeout_ms}))"
        )
        .unwrap();
    }
    if let Some(theme) = test.theme {
        let theme = test_theme_variant(theme);
        write!(
            config,
            ".theme(::ui_lang_runtime::testing::ThemeMode::{theme})"
        )
        .unwrap();
    }
    if let Some(scale_factor) = test.scale_factor {
        write!(config, ".scale_factor({}f32)", rust_f64(scale_factor)).unwrap();
    }
    if let Some(locale) = &test.locale {
        write!(config, ".locale({})", rust_string(locale)).unwrap();
    }
    if let Some(platform) = test.platform {
        let platform = match platform {
            TestPlatform::Linux => "Linux",
            TestPlatform::Windows => "Windows",
            TestPlatform::Macos => "Macos",
            TestPlatform::Wasm => "Wasm",
        };
        write!(
            config,
            ".platform(::ui_lang_runtime::testing::Platform::{platform})"
        )
        .unwrap();
    }
    if let Some(reduced_motion) = test.reduced_motion {
        write!(config, ".reduced_motion({reduced_motion})").unwrap();
    }
    if let Some(preset) = &test.preset {
        write!(config, ".preset({})", rust_string(preset)).unwrap();
    }
    writeln!(out, "let __config = {config};").unwrap();
    let test_program = if test.mount.is_some() {
        format!("{}::__ice_test_program_{index}()", document.app)
    } else {
        format!("{}::__program()", document.app)
    };
    writeln!(
        out,
        "let mut __test = ::ui_lang_runtime::testing::Driver::new({test_program}, __config);"
    )
    .unwrap();

    for step in &test.steps {
        let statement = test_step_source(step);
        let location = location_code(program, source_path, &step.span, &statement);
        let env = test_env(test, program, &location)?;
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
                let path = target_ref_path_code(target, test, &env, document)?;
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
                count: _,
            } => {
                let x = expr_code(x, &env, document, ValueMode::Owned)?;
                let y = expr_code(y, &env, document, ValueMode::Owned)?;
                let button = test_mouse_button_code(*button);
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::ClickAt {{ position: ::iced::Point::new(__x, __y), button: {button}, count: 1 }}, {location});").unwrap();
            }
            TestStepKind::Hover(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::MoveTo(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Enter(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Enter(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Leave => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Leave, {location});").unwrap();
            }
            TestStepKind::Move(TestPointerPosition::Target(target)) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::MoveTo(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Move(TestPointerPosition::Point(x, y)) => {
                let x = expr_code(x, &env, document, ValueMode::Owned)?;
                let y = expr_code(y, &env, document, ValueMode::Owned)?;
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::MoveToPoint(::iced::Point::new(__x, __y)), {location});").unwrap();
            }
            TestStepKind::Press { target, button } => {
                let path = target_ref_path_code(target, test, &env, document)?;
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
                let x = expr_code(x, &env, document, ValueMode::Owned)?;
                let y = expr_code(y, &env, document, ValueMode::Owned)?;
                let delta = match unit {
                    TestWheelUnit::Pixels => "Pixels",
                    TestWheelUnit::Lines => "Lines",
                };
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Wheel(::ui_lang_runtime::testing::WheelDelta::{delta} {{ x: __x, y: __y }}), {location});").unwrap();
            }
            TestStepKind::Scroll { mode, target, x, y } => {
                let path = target_ref_path_code(target, test, &env, document)?;
                let x = expr_code(x, &env, document, ValueMode::Owned)?;
                let y = expr_code(y, &env, document, ValueMode::Owned)?;
                let action = match mode {
                    TestScrollMode::To => "ScrollTo",
                    TestScrollMode::By => "ScrollBy",
                };
                writeln!(out, "let __target = {path}; let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::{action} {{ target: __target.to_owned(), x: __x, y: __y }}, {location});").unwrap();
            }
            TestStepKind::Snap { target, x, y } => {
                let path = target_ref_path_code(target, test, &env, document)?;
                let x = expr_code(x, &env, document, ValueMode::Owned)?;
                let y = expr_code(y, &env, document, ValueMode::Owned)?;
                writeln!(out, "let __target = {path}; let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Snap {{ target: __target.to_owned(), x: __x, y: __y }}, {location});").unwrap();
            }
            TestStepKind::SnapEnd(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::SnapEnd(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Drag { from, to } => {
                let from = target_ref_path_code(from, test, &env, document)?;
                let to = target_ref_path_code(to, test, &env, document)?;
                writeln!(
                    out,
                    "let __from = {from}; let __to = {to}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Drag {{ from: __from.to_owned(), to: __to.to_owned() }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::Drop(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::DropAt(__target.to_owned()), {location});"
                )
                .unwrap();
            }
            TestStepKind::Focus(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
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
                let value = expr_code(value, &env, document, ValueMode::Owned)?;
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
                let value = expr_code(value, &env, document, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __value = {value}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Replace(__value), {location});"
                )
                .unwrap();
            }
            TestStepKind::Select(start, end) => {
                let start = expr_code(start, &env, document, ValueMode::Owned)?;
                let end = expr_code(end, &env, document, ValueMode::Owned)?;
                writeln!(out, "let __start = ::std::primitive::usize::try_from({start}).expect(\"selection start must fit usize\"); let __end = ::std::primitive::usize::try_from({end}).expect(\"selection end must fit usize\"); let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Select {{ start: __start, end: __end }}, {location});").unwrap();
            }
            TestStepKind::SelectAll => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::SelectAll, {location});").unwrap();
            }
            TestStepKind::Cursor(index) => {
                let index = expr_code(index, &env, document, ValueMode::Owned)?;
                writeln!(out, "let __index = ::std::primitive::usize::try_from({index}).expect(\"cursor index must fit usize\"); let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Cursor(__index), {location});").unwrap();
            }
            TestStepKind::CursorFront => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::CursorFront, {location});").unwrap();
            }
            TestStepKind::CursorEnd => {
                writeln!(out, "let _ = __test.perform_action(::ui_lang_runtime::testing::Action::CursorEnd, {location});").unwrap();
            }
            TestStepKind::Composition(composition) => {
                let composition = test_composition_code(composition, &env, document)?;
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
                let count = expr_code(count, &env, document, ValueMode::Owned)?;
                writeln!(out, "let __count = ::std::primitive::usize::try_from({count}).expect(\"repeat count must fit usize\"); let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Repeat {{ key: {key}, count: __count }}, {location});").unwrap();
            }
            TestStepKind::Tap { target, count } => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Tap {{ target: __target.to_owned(), count: {count} }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::Touch { phase, id, x, y } => {
                let phase = match phase {
                    TestTouchPhase::Down => "Down",
                    TestTouchPhase::Move => "Move",
                    TestTouchPhase::Up => "Up",
                    TestTouchPhase::Cancel => "Cancel",
                };
                let id = expr_code(id, &env, document, ValueMode::Owned)?;
                let x = expr_code(x, &env, document, ValueMode::Owned)?;
                let y = expr_code(y, &env, document, ValueMode::Owned)?;
                writeln!(out, "let __id = ::std::primitive::u64::try_from({id}).expect(\"touch id must fit u64\"); let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Touch {{ phase: ::ui_lang_runtime::testing::TouchPhase::{phase}, id: __id, position: ::iced::Point::new(__x, __y) }}, {location});").unwrap();
            }
            TestStepKind::WindowMove(x, y) => {
                let x = expr_code(x, &env, document, ValueMode::Owned)?;
                let y = expr_code(y, &env, document, ValueMode::Owned)?;
                writeln!(out, "let __x = ({x}) as f32; let __y = ({y}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::WindowMove(::iced::Point::new(__x, __y)), {location});").unwrap();
            }
            TestStepKind::Resize(width, height) => {
                let width = expr_code(width, &env, document, ValueMode::Owned)?;
                let height = expr_code(height, &env, document, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __width = ({width}) as f32; let __height = ({height}) as f32; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Resize(::iced::Size::new(__width, __height)), {location});"
                )
                .unwrap();
            }
            TestStepKind::Rescale(value) => {
                let value = expr_code(value, &env, document, ValueMode::Owned)?;
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
                let value = expr_code(value, &env, document, ValueMode::Owned)?;
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
                let path = target_ref_path_code(target, test, &env, document)?;
                let action = match action {
                    TestAccessibilityAction::Activate => "Click",
                    TestAccessibilityAction::Focus => "Focus",
                };
                writeln!(
                    out,
                    "let __target = {path}; let _ = __test.perform_action(::ui_lang_runtime::testing::Action::Accessibility {{ action: ::ui_lang_runtime::testing::AccessibilityAction::{action}, target: __target.to_owned() }}, {location});"
                )
                .unwrap();
            }
            TestStepKind::Dispatch { handler, args } => {
                let variant = handler_variant(handler);
                let args = expr_list_code(args, &env, document)?;
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
                generate_expectation(out, expectation, test, &env, document, &location)?;
            }
        }
        writeln!(out, "}});").unwrap();
    }
    writeln!(out, "}}").unwrap();
    Ok(())
}

fn generate_expectation(
    out: &mut String,
    expectation: &TestExpectation,
    test: &TestDecl,
    env: &HashMap<String, Binding>,
    document: &Document,
    location: &str,
) -> Result<(), Error> {
    match expectation {
        TestExpectation::Expr(Expr::Binary { left, op, right })
            if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) =>
        {
            let left = expr_code(left, env, document, ValueMode::Owned)?;
            let right = expr_code(right, env, document, ValueMode::Owned)?;
            let method = if *op == BinaryOp::Eq {
                "check_eq"
            } else {
                "check_ne"
            };
            writeln!(
                out,
                "let __left = {left}; let __right = {right}; __test.{method}(__left, __right, {location});"
            )
            .unwrap();
        }
        TestExpectation::Expr(value) => {
            let mut code = expr_code(value, env, document, ValueMode::Owned)?;
            if matches!(value, Expr::Unary { .. } | Expr::Binary { .. }) {
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
        TestExpectation::Approx { left, right } => {
            let left = expr_code(left, env, document, ValueMode::Owned)?;
            let right = expr_code(right, env, document, ValueMode::Owned)?;
            writeln!(
                out,
                "let __left = ({left}) as f64; let __right = ({right}) as f64; __test.check_approx(__left, __right, {location});"
            )
            .unwrap();
        }
        TestExpectation::Exists(target) | TestExpectation::Missing(target) => {
            let path = target_ref_path_code(target, test, env, document)?;
            let expected = matches!(expectation, TestExpectation::Exists(_));
            writeln!(
                out,
                "let __target = {path}; __test.check_exists(&__target, {expected}, {location});"
            )
            .unwrap();
        }
        TestExpectation::Text {
            value,
            within,
            negated,
        } => {
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            if let Some(within) = within {
                let path = target_ref_path_code(within, test, env, document)?;
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
        TestExpectation::Accessibility { target, property } => {
            let path = target_ref_path_code(target, test, env, document)?;
            match property {
                TestAccessibilityProperty::Role(value)
                | TestAccessibilityProperty::Name(value)
                | TestAccessibilityProperty::Value(value) => {
                    let property = match property {
                        TestAccessibilityProperty::Role(_) => "Role",
                        TestAccessibilityProperty::Name(_) => "Name",
                        TestAccessibilityProperty::Value(_) => "Value",
                        _ => unreachable!(),
                    };
                    let value = expr_code(value, env, document, ValueMode::Owned)?;
                    writeln!(out, "let __target = {path}; let __expected = {value}; __test.check_accessibility_str(&__target, ::ui_lang_runtime::testing::AccessibilityProperty::{property}, &__expected, {location});").unwrap();
                }
                TestAccessibilityProperty::Checked(value)
                | TestAccessibilityProperty::Disabled(value)
                | TestAccessibilityProperty::Focused(value) => {
                    let property = match property {
                        TestAccessibilityProperty::Checked(_) => "Checked",
                        TestAccessibilityProperty::Disabled(_) => "Disabled",
                        TestAccessibilityProperty::Focused(_) => "Focused",
                        _ => unreachable!(),
                    };
                    let value = expr_code(value, env, document, ValueMode::Owned)?;
                    writeln!(out, "let __target = {path}; let __expected = {value}; __test.check_accessibility_bool(&__target, ::ui_lang_runtime::testing::AccessibilityProperty::{property}, __expected, {location});").unwrap();
                }
                TestAccessibilityProperty::Action { name, expected } => {
                    let action = accessibility_action_variant(name);
                    let expected = expr_code(expected, env, document, ValueMode::Owned)?;
                    writeln!(out, "let __target = {path}; let __expected = {expected}; __test.check_accessibility_action(&__target, ::ui_lang_runtime::testing::AccessibilityAction::{action}, __expected, {location});").unwrap();
                }
            }
        }
    }
    Ok(())
}

fn test_theme_variant(theme: TestTheme) -> &'static str {
    match theme {
        TestTheme::Light => "Light",
        TestTheme::Dark => "Dark",
        TestTheme::None => "None",
    }
}

fn test_mouse_button_code(button: TestMouseButton) -> &'static str {
    match button {
        TestMouseButton::Left => "::ui_lang_runtime::testing::MouseButton::Left",
        TestMouseButton::Right => "::ui_lang_runtime::testing::MouseButton::Right",
        TestMouseButton::Middle => "::ui_lang_runtime::testing::MouseButton::Middle",
        TestMouseButton::Back => "::ui_lang_runtime::testing::MouseButton::Back",
        TestMouseButton::Forward => "::ui_lang_runtime::testing::MouseButton::Forward",
    }
}

fn test_key_code(key: &TestKey) -> String {
    match key {
        TestKey::Named(name) => format!(
            "::ui_lang_runtime::testing::Key::named(::iced::keyboard::key::Named::{})",
            test_keyboard_variant_name(name)
        ),
        TestKey::Character(value) => format!(
            "::ui_lang_runtime::testing::Key::character({})",
            rust_string(value)
        ),
    }
}

fn test_modifiers_code(modifiers: TestModifiers) -> String {
    format!(
        "::ui_lang_runtime::testing::Modifiers::new({}, {}, {}, {})",
        modifiers.shift, modifiers.control, modifiers.alt, modifiers.logo
    )
}

fn test_key_metadata_code(event: &TestKeyEvent) -> String {
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
        TestKeyLocation::Standard => "Standard",
        TestKeyLocation::Left => "Left",
        TestKeyLocation::Right => "Right",
        TestKeyLocation::Numpad => "Numpad",
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
    composition: &TestComposition,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match composition {
        TestComposition::Start => "::ui_lang_runtime::testing::CompositionPhase::Start".into(),
        TestComposition::Update { value, selection } => {
            let selection = selection.as_ref().map_or_else(
                || Ok("::std::option::Option::None".to_owned()),
                |(start, end)| {
                    Ok::<_, Error>(format!(
                        "::std::option::Option::Some(::std::ops::Range {{ start: ::std::primitive::usize::try_from({}).expect(\"composition selection start must fit usize\"), end: ::std::primitive::usize::try_from({}).expect(\"composition selection end must fit usize\") }})",
                        expr_code(start, env, document, ValueMode::Owned)?,
                        expr_code(end, env, document, ValueMode::Owned)?
                    ))
                },
            )?;
            format!(
                "::ui_lang_runtime::testing::CompositionPhase::Update {{ text: {}, selection: {selection} }}",
                expr_code(value, env, document, ValueMode::Owned)?
            )
        }
        TestComposition::Commit(value) => format!(
            "::ui_lang_runtime::testing::CompositionPhase::Commit({})",
            expr_code(value, env, document, ValueMode::Owned)?
        ),
        TestComposition::Cancel => "::ui_lang_runtime::testing::CompositionPhase::Cancel".into(),
    })
}

fn accessibility_action_variant(name: &str) -> &'static str {
    match name {
        "click" => "Click",
        "focus" => "Focus",
        _ => unreachable!("parser validates accessibility actions"),
    }
}

fn test_env(
    test: &TestDecl,
    program: &LoweredProgram,
    location: &str,
) -> Result<HashMap<String, Binding>, Error> {
    let document = program.document();
    let mut env = state_env(document, "__test.state()");
    if program.settings().kind == ProgramKind::Daemon {
        env.insert(
            "window".into(),
            Binding {
                code: "__test.window()".into(),
                ty: Type::WindowId,
                local: true,
                state: None,
                owner: None,
            },
        );
    }
    for target in &test.targets {
        let path = widget_target_path_code(&target.target, &env, document)?;
        env.insert(
            target.name.clone(),
            Binding {
                code: format!(
                    "{{ let __target_path = {path}; __test.target(&__target_path, {location}) }}"
                ),
                ty: Type::TestTarget,
                local: true,
                state: None,
                owner: None,
            },
        );
    }
    Ok(env)
}

fn target_ref_path_code(
    target: &TestTargetRef,
    test: &TestDecl,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let target = match target {
        TestTargetRef::Alias(name) => {
            &test
                .targets
                .iter()
                .find(|target| target.name == *name)
                .expect("checker validates test target aliases")
                .target
        }
        TestTargetRef::Id(target) => target,
    };
    widget_target_path_code(target, env, document)
}

fn location_code(
    document: &LoweredProgram,
    source_path: &str,
    span: &Span,
    statement: &str,
) -> String {
    let (source_path, line) = document
        .source_origin(span.line)
        .map_or((source_path.to_owned(), span.line), |(path, line)| {
            (path.display().to_string(), line)
        });
    format!(
        "::ui_lang_runtime::testing::Location::new({}, {}, {}, {})",
        rust_string(&source_path),
        line,
        span.column,
        rust_string(statement)
    )
}

fn test_program_code(program: &LoweredProgram, source_path: &str, index: usize) -> String {
    let document = program.document();
    let app_settings = program.settings();
    let subscription = ".subscription(Self::__subscription)";
    let default_font = if app_settings.has_default_font {
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
    let presets = if document.presets.is_empty() {
        String::new()
    } else {
        format!(
            ".presets([{}])",
            document
                .presets
                .iter()
                .enumerate()
                .map(|(index, preset)| format!(
                    "::iced::Preset::new({}, Self::__preset_{index})",
                    rust_string(&preset.name)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )
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

fn test_step_source(step: &TestStep) -> String {
    match &step.kind {
        TestStepKind::Click {
            target,
            button,
            count,
        } => format!(
            "{} {}{}",
            if *count == 2 { "double-click" } else { "click" },
            target_ref_source(target),
            mouse_button_suffix(*button)
        ),
        TestStepKind::ClickAt { x, y, button, .. } => format!(
            "click-at {} {}{}",
            expr_source(x),
            expr_source(y),
            mouse_button_suffix(*button)
        ),
        TestStepKind::Hover(target) => format!("hover {}", target_ref_source(target)),
        TestStepKind::Enter(target) => format!("enter {}", target_ref_source(target)),
        TestStepKind::Leave => "leave".into(),
        TestStepKind::Move(TestPointerPosition::Target(target)) => {
            format!("move {}", target_ref_source(target))
        }
        TestStepKind::Move(TestPointerPosition::Point(x, y)) => {
            format!("move {} {}", expr_source(x), expr_source(y))
        }
        TestStepKind::Press { target, button } => format!(
            "press {}{}",
            target_ref_source(target),
            mouse_button_suffix(*button)
        ),
        TestStepKind::Release(button) => {
            format!("release{}", mouse_button_suffix(*button))
        }
        TestStepKind::Wheel { unit, x, y } => format!(
            "wheel {} {} {}",
            match unit {
                TestWheelUnit::Pixels => "pixels",
                TestWheelUnit::Lines => "lines",
            },
            expr_source(x),
            expr_source(y)
        ),
        TestStepKind::Scroll { mode, target, x, y } => format!(
            "{} {} {} {}",
            match mode {
                TestScrollMode::To => "scroll-to",
                TestScrollMode::By => "scroll-by",
            },
            target_ref_source(target),
            expr_source(x),
            expr_source(y)
        ),
        TestStepKind::Snap { target, x, y } => format!(
            "snap {} {} {}",
            target_ref_source(target),
            expr_source(x),
            expr_source(y)
        ),
        TestStepKind::SnapEnd(target) => format!("snap-end {}", target_ref_source(target)),
        TestStepKind::Drag { from, to } => {
            format!("drag {} {}", target_ref_source(from), target_ref_source(to))
        }
        TestStepKind::Drop(target) => format!("drop {}", target_ref_source(target)),
        TestStepKind::Focus(target) => format!("focus {}", target_ref_source(target)),
        TestStepKind::FocusNext => "focus-next".into(),
        TestStepKind::FocusPrevious => "focus-previous".into(),
        TestStepKind::Blur => "blur".into(),
        TestStepKind::WindowFocus(true) => "window focus".into(),
        TestStepKind::WindowFocus(false) => "window blur".into(),
        TestStepKind::Type(value) => format!("type {}", expr_source(value)),
        TestStepKind::Clear => "clear".into(),
        TestStepKind::Replace(value) => format!("replace {}", expr_source(value)),
        TestStepKind::Select(start, end) => {
            format!("select {} {}", expr_source(start), expr_source(end))
        }
        TestStepKind::SelectAll => "select-all".into(),
        TestStepKind::Cursor(index) => format!("cursor {}", expr_source(index)),
        TestStepKind::CursorFront => "cursor front".into(),
        TestStepKind::CursorEnd => "cursor end".into(),
        TestStepKind::Composition(TestComposition::Start) => "composition start".into(),
        TestStepKind::Composition(TestComposition::Update { value, selection }) => {
            format!(
                "composition update {}{}",
                expr_source(value),
                selection
                    .as_ref()
                    .map_or_else(String::new, |(start, end)| format!(
                        " {} {}",
                        expr_source(start),
                        expr_source(end)
                    ))
            )
        }
        TestStepKind::Composition(TestComposition::Commit(value)) => {
            format!("composition commit {}", expr_source(value))
        }
        TestStepKind::Composition(TestComposition::Cancel) => "composition cancel".into(),
        TestStepKind::Key(key) => format!("key {}", test_key_source(key)),
        TestStepKind::KeyDown(event) => format!("key-down {}", test_key_event_source(event)),
        TestStepKind::KeyUp(event) => format!("key-up {}", test_key_event_source(event)),
        TestStepKind::Modifiers(modifiers) => {
            let values = test_modifiers_source(*modifiers);
            if values.is_empty() {
                "modifiers".into()
            } else {
                format!("modifiers {values}")
            }
        }
        TestStepKind::Chord { modifiers, key } => {
            let modifiers = test_modifiers_source(*modifiers);
            format!(
                "chord {}{}",
                if modifiers.is_empty() {
                    String::new()
                } else {
                    format!("{modifiers} ")
                },
                test_key_source(key)
            )
        }
        TestStepKind::Repeat { key, count } => {
            format!("repeat {} {}", test_key_source(key), expr_source(count))
        }
        TestStepKind::Tap { target, count } => format!(
            "tap {}{}",
            target_ref_source(target),
            if *count == 1 {
                String::new()
            } else {
                format!(" {count}")
            }
        ),
        TestStepKind::Touch { phase, id, x, y } => format!(
            "touch {} {} {} {}",
            match phase {
                TestTouchPhase::Down => "down",
                TestTouchPhase::Move => "move",
                TestTouchPhase::Up => "up",
                TestTouchPhase::Cancel => "cancel",
            },
            expr_source(id),
            expr_source(x),
            expr_source(y)
        ),
        TestStepKind::WindowMove(x, y) => {
            format!("window move {} {}", expr_source(x), expr_source(y))
        }
        TestStepKind::Resize(width, height) => {
            format!("resize {} {}", expr_source(width), expr_source(height))
        }
        TestStepKind::Rescale(value) => format!("window rescale {}", expr_source(value)),
        TestStepKind::WindowClose => "window close-request".into(),
        TestStepKind::WindowOpened => "window opened".into(),
        TestStepKind::WindowClosed => "window closed".into(),
        TestStepKind::Redraw => "window redraw".into(),
        TestStepKind::SystemTheme(theme) => {
            format!("system-theme {}", test_theme_source(*theme))
        }
        TestStepKind::FileHover(value) => format!("file-hover {}", expr_source(value)),
        TestStepKind::FileDrop(value) => format!("file-drop {}", expr_source(value)),
        TestStepKind::FileLeave => "file-leave".into(),
        TestStepKind::Wait(duration) => format!("wait {duration}ms"),
        TestStepKind::Advance(duration) => format!("advance {duration}ms"),
        TestStepKind::Idle => "idle".into(),
        TestStepKind::Capture(name) => format!("capture {name}"),
        TestStepKind::Accessibility { action, target } => format!(
            "a11y {} {}",
            match action {
                TestAccessibilityAction::Activate => "activate",
                TestAccessibilityAction::Focus => "focus",
            },
            target_ref_source(target)
        ),
        TestStepKind::Dispatch { handler, args } => format!(
            "dispatch {handler}({})",
            args.iter().map(expr_source).collect::<Vec<_>>().join(", ")
        ),
        TestStepKind::Expect(expectation) => format!(
            "expect {}",
            match expectation {
                TestExpectation::Expr(value) => expr_source(value),
                TestExpectation::Approx { left, right } => {
                    format!("{} ~= {}", expr_source(left), expr_source(right))
                }
                TestExpectation::Exists(target) => {
                    format!("exists {}", target_ref_source(target))
                }
                TestExpectation::Missing(target) => {
                    format!("missing {}", target_ref_source(target))
                }
                TestExpectation::Text {
                    value,
                    within,
                    negated,
                } => format!(
                    "{}text {}{}",
                    if *negated { "no " } else { "" },
                    expr_source(value),
                    within.as_ref().map_or_else(String::new, |target| format!(
                        " within {}",
                        target_ref_source(target)
                    ))
                ),
                TestExpectation::Accessibility { target, property } => format!(
                    "a11y {} {}",
                    target_ref_source(target),
                    accessibility_property_source(property)
                ),
            }
        ),
    }
}

fn test_theme_source(theme: TestTheme) -> &'static str {
    match theme {
        TestTheme::Light => "light",
        TestTheme::Dark => "dark",
        TestTheme::None => "none",
    }
}

fn mouse_button_source(button: TestMouseButton) -> &'static str {
    match button {
        TestMouseButton::Left => "left",
        TestMouseButton::Right => "right",
        TestMouseButton::Middle => "middle",
        TestMouseButton::Back => "back",
        TestMouseButton::Forward => "forward",
    }
}

fn mouse_button_suffix(button: TestMouseButton) -> String {
    if button == TestMouseButton::Left {
        String::new()
    } else {
        format!(" {}", mouse_button_source(button))
    }
}

fn test_key_source(key: &TestKey) -> String {
    match key {
        TestKey::Named(name) => name.clone(),
        TestKey::Character(value) => format!("{value:?}"),
    }
}

fn test_modifiers_source(modifiers: TestModifiers) -> String {
    [
        (modifiers.shift, "shift"),
        (modifiers.control, "control"),
        (modifiers.alt, "alt"),
        (modifiers.logo, "logo"),
    ]
    .into_iter()
    .filter_map(|(enabled, name)| enabled.then_some(name))
    .collect::<Vec<_>>()
    .join(" ")
}

fn test_key_event_source(event: &TestKeyEvent) -> String {
    let mut values = vec![test_key_source(&event.key)];
    if let Some(modified) = &event.modified_key {
        values.push(format!("modified={}", test_key_source(modified)));
    }
    if event.location != TestKeyLocation::Standard {
        values.push(format!(
            "location={}",
            match event.location {
                TestKeyLocation::Standard => "standard",
                TestKeyLocation::Left => "left",
                TestKeyLocation::Right => "right",
                TestKeyLocation::Numpad => "numpad",
            }
        ));
    }
    if let Some(physical) = &event.physical {
        values.push(format!("physical={physical}"));
    }
    if let Some(text) = &event.text {
        values.push(format!("text={text:?}"));
    }
    if event.repeat {
        values.push("repeat=true".into());
    }
    values.join(" ")
}

fn accessibility_property_source(property: &TestAccessibilityProperty) -> String {
    match property {
        TestAccessibilityProperty::Role(value) => format!("role {}", expr_source(value)),
        TestAccessibilityProperty::Name(value) => format!("name {}", expr_source(value)),
        TestAccessibilityProperty::Value(value) => format!("value {}", expr_source(value)),
        TestAccessibilityProperty::Checked(value) => {
            format!("checked {}", expr_source(value))
        }
        TestAccessibilityProperty::Disabled(value) => {
            format!("disabled {}", expr_source(value))
        }
        TestAccessibilityProperty::Focused(value) => {
            format!("focused {}", expr_source(value))
        }
        TestAccessibilityProperty::Action { name, expected } => {
            format!("action {name} {}", expr_source(expected))
        }
    }
}

fn target_ref_source(target: &TestTargetRef) -> String {
    match target {
        TestTargetRef::Alias(name) => name.clone(),
        TestTargetRef::Id(target) => widget_target_source(target),
    }
}

fn widget_target_source(target: &WidgetTarget) -> String {
    format!(
        "#{}",
        target
            .segments
            .iter()
            .map(|segment| segment.key.as_ref().map_or_else(
                || segment.name.clone(),
                |key| format!("{}({})", segment.name, expr_source(key))
            ))
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn expr_source(expr: &Expr) -> String {
    match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::I64(value) => value.to_string(),
        Expr::F64(value) => rust_f64(*value),
        Expr::Str(value) => format!("{value:?}"),
        Expr::Bytes(values) => format!(
            "bytes({})",
            values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::EmptyList => "[]".into(),
        Expr::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(expr_source)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::None => "none".into(),
        Expr::Path(path) => path.join("."),
        Expr::Call { name, args } => format!(
            "{name}({})",
            args.iter().map(expr_source).collect::<Vec<_>>().join(", ")
        ),
        Expr::Unary { op, value } => format!(
            "{}{}",
            match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            },
            expr_source(value)
        ),
        Expr::Binary { left, op, right } => format!(
            "({} {} {})",
            expr_source(left),
            match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Rem => "%",
                BinaryOp::Eq => "==",
                BinaryOp::NotEq => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::LtEq => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::GtEq => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
            },
            expr_source(right)
        ),
    }
}
