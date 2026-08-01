use super::*;

#[derive(Clone, Debug)]
pub struct TestDecl {
    pub name: String,
    pub preset: Option<String>,
    pub viewport: Option<(f64, f64)>,
    pub timeout_ms: Option<u64>,
    pub theme: Option<TestTheme>,
    pub scale_factor: Option<f64>,
    pub locale: Option<String>,
    pub platform: Option<TestPlatform>,
    pub reduced_motion: Option<bool>,
    pub mount: Option<ViewNode>,
    pub targets: Vec<TestTargetDecl>,
    pub steps: Vec<TestStep>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TestTargetDecl {
    pub name: String,
    pub target: WidgetTarget,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TestTargetRef {
    Alias(String),
    Id(WidgetTarget),
}

#[derive(Clone, Debug)]
pub struct TestStep {
    pub kind: TestStepKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TestStepKind {
    Click {
        target: TestTargetRef,
        button: TestMouseButton,
        count: u8,
    },
    ClickAt {
        x: Expr,
        y: Expr,
        button: TestMouseButton,
        count: u8,
    },
    Hover(TestTargetRef),
    Enter(TestTargetRef),
    Leave,
    Move(TestPointerPosition),
    Press {
        target: TestTargetRef,
        button: TestMouseButton,
    },
    Release(TestMouseButton),
    Wheel {
        unit: TestWheelUnit,
        x: Expr,
        y: Expr,
    },
    Scroll {
        mode: TestScrollMode,
        target: TestTargetRef,
        x: Expr,
        y: Expr,
    },
    Snap {
        target: TestTargetRef,
        x: Expr,
        y: Expr,
    },
    SnapEnd(TestTargetRef),
    Drag {
        from: TestTargetRef,
        to: TestTargetRef,
    },
    Drop(TestTargetRef),
    Focus(TestTargetRef),
    FocusNext,
    FocusPrevious,
    Blur,
    WindowFocus(bool),
    Type(Expr),
    Clear,
    Replace(Expr),
    Select(Expr, Expr),
    SelectAll,
    Cursor(Expr),
    CursorFront,
    CursorEnd,
    Composition(TestComposition),
    Key(TestKey),
    KeyDown(TestKeyEvent),
    KeyUp(TestKeyEvent),
    Modifiers(TestModifiers),
    Chord {
        modifiers: TestModifiers,
        key: TestKey,
    },
    Repeat {
        key: TestKey,
        count: Expr,
    },
    Tap {
        target: TestTargetRef,
        count: u8,
    },
    Touch {
        phase: TestTouchPhase,
        id: Expr,
        x: Expr,
        y: Expr,
    },
    WindowMove(Expr, Expr),
    Resize(Expr, Expr),
    Rescale(Expr),
    WindowClose,
    WindowOpened,
    WindowClosed,
    Redraw,
    SystemTheme(TestTheme),
    FileHover(Expr),
    FileDrop(Expr),
    FileLeave,
    Wait(u64),
    Advance(u64),
    Idle,
    Capture(String),
    Accessibility {
        action: TestAccessibilityAction,
        target: TestTargetRef,
    },
    Dispatch {
        handler: String,
        args: Vec<Expr>,
    },
    Expect(TestExpectation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestTheme {
    Light,
    Dark,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestPlatform {
    Linux,
    Windows,
    Macos,
    Wasm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestMouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestWheelUnit {
    Pixels,
    Lines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestScrollMode {
    To,
    By,
}

#[derive(Clone, Debug)]
pub enum TestPointerPosition {
    Target(TestTargetRef),
    Point(Expr, Expr),
}

#[derive(Clone, Debug)]
pub enum TestKey {
    Named(String),
    Character(String),
}

#[derive(Clone, Debug)]
pub struct TestKeyEvent {
    pub key: TestKey,
    pub modified_key: Option<TestKey>,
    pub location: TestKeyLocation,
    pub physical: Option<String>,
    pub text: Option<String>,
    pub repeat: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TestKeyLocation {
    #[default]
    Standard,
    Left,
    Right,
    Numpad,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub logo: bool,
}

#[derive(Clone, Debug)]
pub enum TestComposition {
    Start,
    Update {
        value: Expr,
        selection: Option<(Expr, Expr)>,
    },
    Commit(Expr),
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestTouchPhase {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestAccessibilityAction {
    Activate,
    Focus,
}

#[derive(Clone, Debug)]
pub enum TestExpectation {
    Expr(Expr),
    Approx {
        left: Expr,
        right: Expr,
    },
    Exists(TestTargetRef),
    Missing(TestTargetRef),
    Text {
        value: Expr,
        within: Option<TestTargetRef>,
        negated: bool,
    },
    Accessibility {
        target: TestTargetRef,
        property: TestAccessibilityProperty,
    },
}

#[derive(Clone, Debug)]
pub enum TestAccessibilityProperty {
    Role(Expr),
    Name(Expr),
    Value(Expr),
    Checked(Expr),
    Disabled(Expr),
    Focused(Expr),
    Action { name: String, expected: Expr },
}

pub(crate) fn widget_target_expression_roots(target: &WidgetTarget) -> Vec<&Expr> {
    target
        .segments
        .iter()
        .filter_map(|segment| segment.key.as_ref())
        .collect()
}

fn target_ref_expression_roots(target: &TestTargetRef) -> Vec<&Expr> {
    match target {
        TestTargetRef::Alias(_) => Vec::new(),
        TestTargetRef::Id(target) => widget_target_expression_roots(target),
    }
}

pub(crate) fn test_step_expression_roots(step: &TestStep) -> Vec<&Expr> {
    let mut expressions = Vec::new();
    match &step.kind {
        TestStepKind::Click { target: value, .. }
        | TestStepKind::Hover(value)
        | TestStepKind::Enter(value)
        | TestStepKind::Move(TestPointerPosition::Target(value))
        | TestStepKind::Press { target: value, .. }
        | TestStepKind::SnapEnd(value)
        | TestStepKind::Drop(value)
        | TestStepKind::Focus(value)
        | TestStepKind::Tap { target: value, .. }
        | TestStepKind::Accessibility { target: value, .. } => {
            expressions.extend(target_ref_expression_roots(value));
        }
        TestStepKind::ClickAt { x, y, .. }
        | TestStepKind::Wheel { x, y, .. }
        | TestStepKind::Move(TestPointerPosition::Point(x, y))
        | TestStepKind::WindowMove(x, y)
        | TestStepKind::Resize(x, y)
        | TestStepKind::Select(x, y) => expressions.extend([x, y]),
        TestStepKind::Scroll {
            target: value,
            x,
            y,
            ..
        }
        | TestStepKind::Snap {
            target: value,
            x,
            y,
        } => {
            expressions.extend(target_ref_expression_roots(value));
            expressions.extend([x, y]);
        }
        TestStepKind::Drag { from, to } => {
            expressions.extend(target_ref_expression_roots(from));
            expressions.extend(target_ref_expression_roots(to));
        }
        TestStepKind::Type(value)
        | TestStepKind::Replace(value)
        | TestStepKind::Cursor(value)
        | TestStepKind::Repeat { count: value, .. }
        | TestStepKind::Rescale(value)
        | TestStepKind::FileHover(value)
        | TestStepKind::FileDrop(value)
        | TestStepKind::Composition(TestComposition::Commit(value)) => expressions.push(value),
        TestStepKind::Composition(TestComposition::Update { value, selection }) => {
            expressions.push(value);
            if let Some((start, end)) = selection {
                expressions.extend([start, end]);
            }
        }
        TestStepKind::Touch { id, x, y, .. } => expressions.extend([id, x, y]),
        TestStepKind::Dispatch { args, .. } => expressions.extend(args),
        TestStepKind::Expect(expectation) => match expectation {
            TestExpectation::Expr(value) => expressions.push(value),
            TestExpectation::Approx { left, right } => expressions.extend([left, right]),
            TestExpectation::Exists(value) | TestExpectation::Missing(value) => {
                expressions.extend(target_ref_expression_roots(value));
            }
            TestExpectation::Text { value, within, .. } => {
                expressions.push(value);
                if let Some(within) = within {
                    expressions.extend(target_ref_expression_roots(within));
                }
            }
            TestExpectation::Accessibility {
                target: value,
                property,
            } => {
                expressions.extend(target_ref_expression_roots(value));
                expressions.push(match property {
                    TestAccessibilityProperty::Role(value)
                    | TestAccessibilityProperty::Name(value)
                    | TestAccessibilityProperty::Value(value)
                    | TestAccessibilityProperty::Checked(value)
                    | TestAccessibilityProperty::Disabled(value)
                    | TestAccessibilityProperty::Focused(value)
                    | TestAccessibilityProperty::Action {
                        expected: value, ..
                    } => value,
                });
            }
        },
        TestStepKind::Release(_)
        | TestStepKind::Leave
        | TestStepKind::FocusNext
        | TestStepKind::FocusPrevious
        | TestStepKind::Blur
        | TestStepKind::WindowFocus(_)
        | TestStepKind::Clear
        | TestStepKind::SelectAll
        | TestStepKind::CursorFront
        | TestStepKind::CursorEnd
        | TestStepKind::Composition(TestComposition::Start | TestComposition::Cancel)
        | TestStepKind::Key(_)
        | TestStepKind::KeyDown(_)
        | TestStepKind::KeyUp(_)
        | TestStepKind::Modifiers(_)
        | TestStepKind::Chord { .. }
        | TestStepKind::WindowClose
        | TestStepKind::WindowOpened
        | TestStepKind::WindowClosed
        | TestStepKind::Redraw
        | TestStepKind::SystemTheme(_)
        | TestStepKind::FileLeave
        | TestStepKind::Wait(_)
        | TestStepKind::Advance(_)
        | TestStepKind::Idle
        | TestStepKind::Capture(_) => {}
    }
    expressions
}

pub(crate) fn test_keyboard_variant_name(name: &str) -> String {
    if name
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase())
    {
        return name.to_owned();
    }
    match name {
        "tv" => return "TV".into(),
        "avr-input" => return "AVRInput".into(),
        "avr-power" => return "AVRPower".into(),
        "dvr" => return "DVR".into(),
        _ => {}
    }
    name.split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect()
}
