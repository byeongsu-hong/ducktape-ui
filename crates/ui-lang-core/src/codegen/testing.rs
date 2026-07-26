use super::*;

pub(in crate::codegen) fn generate_test_mounts(
    out: &mut String,
    document: &Document,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    for (index, test) in document.tests.iter().enumerate() {
        let Some(mount) = &test.mount else {
            continue;
        };
        let mut env = state_env(document, "self");
        if document.daemon {
            env.insert(
                "window".into(),
                Binding {
                    code: "window".into(),
                    ty: Type::WindowId,
                    local: true,
                    state: None,
                },
            );
        }
        let root = render_node(
            mount,
            document,
            message,
            &env,
            &rust_string(&document.app),
            None,
        )?;
        let window_arg = if document.daemon {
            ", window: ::iced::window::Id"
        } else {
            ""
        };
        if document.daemon {
            writeln!(
                out,
                "#[cfg(test)]\nfn __ice_test_mount_{index}(&self{window_arg}) -> __IceElement<'_, {message}> {{ {root} }}"
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "#[cfg(test)]\nfn __ice_test_mount_{index}(&self{window_arg}) -> __IceElement<'_, {message}> {{ let __ice_content: __IceElement<'_, {message}> = {root}; ::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext, {message}::__AccessibilityFocusPrevious).into() }}"
            )
            .unwrap();
        }
        let program = test_program_code(document, source_path, index);
        let program_ty = if document.daemon {
            "::iced::Daemon"
        } else {
            "::iced::Application"
        };
        writeln!(
            out,
            "#[cfg(test)]\nfn __ice_test_program_{index}() -> {program_ty}<impl ::iced::Program<State = Self, Message = {message}, Theme = ::iced::Theme>> {{ {program} }}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_tests(
    out: &mut String,
    document: &CheckedDocument,
    message: &str,
    source_path: &str,
) -> Result<(), Error> {
    if document.tests.is_empty() {
        return Ok(());
    }
    writeln!(out, "#[cfg(test)]\nmod __ice_tests {{\nuse super::*;").unwrap();
    for (index, test) in document.tests.iter().enumerate() {
        generate_test(out, document, message, source_path, index, test)?;
    }
    writeln!(out, "}}").unwrap();
    Ok(())
}

fn generate_test(
    out: &mut String,
    document: &CheckedDocument,
    message: &str,
    source_path: &str,
    index: usize,
    test: &TestDecl,
) -> Result<(), Error> {
    writeln!(out, "#[test]\nfn {}() {{", test.name).unwrap();
    let declaration = format!("test {}", test.name);
    let source = location_code(document, source_path, &test.span, &declaration);
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
    if let Some(preset) = &test.preset {
        write!(config, ".preset({})", rust_string(preset)).unwrap();
    }
    writeln!(out, "let __config = {config};").unwrap();
    let program = if test.mount.is_some() {
        format!("{}::__ice_test_program_{index}()", document.app)
    } else {
        format!("{}::__program()", document.app)
    };
    writeln!(
        out,
        "let mut __test = ::ui_lang_runtime::testing::Driver::new({program}, __config);"
    )
    .unwrap();

    for step in &test.steps {
        let statement = test_step_source(step);
        let location = location_code(document, source_path, &step.span, &statement);
        let env = test_env(test, document, &location)?;
        writeln!(
            out,
            "::ui_lang_runtime::testing::step({}, {location}, || {{",
            rust_string(&test.name)
        )
        .unwrap();
        match &step.kind {
            TestStepKind::Click(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; __test.click(&__target, {location});"
                )
                .unwrap();
            }
            TestStepKind::Hover(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; __test.hover(&__target, {location});"
                )
                .unwrap();
            }
            TestStepKind::Press(target) => {
                let path = target_ref_path_code(target, test, &env, document)?;
                writeln!(
                    out,
                    "let __target = {path}; __test.press(&__target, {location});"
                )
                .unwrap();
            }
            TestStepKind::Release => {
                writeln!(out, "__test.release({location});").unwrap();
            }
            TestStepKind::Type(value) => {
                let value = expr_code(value, &env, document, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __value = {value}; __test.typewrite(&__value, {location});"
                )
                .unwrap();
            }
            TestStepKind::Key(key) => {
                let key = match key {
                    TestKey::Enter => "Enter",
                    TestKey::Escape => "Escape",
                    TestKey::Tab => "Tab",
                    TestKey::Backspace => "Backspace",
                };
                writeln!(
                    out,
                    "__test.key(::iced::keyboard::Key::Named(::iced::keyboard::key::Named::{key}), {location});"
                )
                .unwrap();
            }
            TestStepKind::Resize(width, height) => {
                let width = expr_code(width, &env, document, ValueMode::Owned)?;
                let height = expr_code(height, &env, document, ValueMode::Owned)?;
                writeln!(
                    out,
                    "let __width = ({width}) as f32; let __height = ({height}) as f32; __test.resize(__width, __height, {location});"
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
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            writeln!(
                out,
                "let __actual = {value}; __test.check(__actual, {location});"
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
    }
    Ok(())
}

fn test_env(
    test: &TestDecl,
    document: &Document,
    location: &str,
) -> Result<HashMap<String, Binding>, Error> {
    let mut env = state_env(document, "__test.state()");
    if document.daemon {
        env.insert(
            "window".into(),
            Binding {
                code: "__test.window()".into(),
                ty: Type::WindowId,
                local: true,
                state: None,
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
    document: &CheckedDocument,
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

fn test_program_code(document: &Document, source_path: &str, index: usize) -> String {
    let subscription = ".subscription(Self::__subscription)";
    let default_font = document
        .fonts
        .iter()
        .find(|font| font.default)
        .map_or_else(String::new, |font| {
            format!(".default_font({})", font_decl_code(font))
        });
    let title = document
        .settings
        .title
        .as_ref()
        .map_or("", |_| ".title(Self::__title)");
    let settings = app_settings_code(&document.settings);
    let fonts = font_assets_code(&document.settings, source_path);
    let window = if document.daemon {
        String::new()
    } else {
        window_settings_code(document.settings.window.as_ref(), source_path)
    };
    let executor = document
        .settings
        .executor
        .as_ref()
        .map_or_else(String::new, |executor| format!(".executor::<{executor}>()"));
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
    let scale_factor = document
        .settings
        .scale_factor
        .as_ref()
        .map_or("", |_| ".scale_factor(Self::__scale_factor)");
    let style = if document.settings.background.is_some() || document.settings.text_color.is_some()
    {
        ".style(Self::__style)"
    } else {
        ""
    };
    let root = if document.daemon {
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
        TestStepKind::Click(target) => format!("click {}", target_ref_source(target)),
        TestStepKind::Hover(target) => format!("hover {}", target_ref_source(target)),
        TestStepKind::Press(target) => format!("press {}", target_ref_source(target)),
        TestStepKind::Release => "release".into(),
        TestStepKind::Type(value) => format!("type {}", expr_source(value)),
        TestStepKind::Key(key) => format!(
            "key {}",
            match key {
                TestKey::Enter => "enter",
                TestKey::Escape => "escape",
                TestKey::Tab => "tab",
                TestKey::Backspace => "backspace",
            }
        ),
        TestStepKind::Resize(width, height) => {
            format!("resize {} {}", expr_source(width), expr_source(height))
        }
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
            }
        ),
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
