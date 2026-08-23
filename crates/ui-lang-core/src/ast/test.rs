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
    /// Chooses the tray menu row whose text carries this value, the way the
    /// platform reports one: by row, through the generated row-to-handler
    /// table the live subscription uses.
    TrayChoose(Expr),
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
    /// What the program last decided the status item should show. Read from
    /// the runtime's record rather than the screen, so the assertion runs and
    /// means the same thing where the tray is native and where it is a no-op.
    Tray {
        field: TrayField,
        value: Expr,
        negated: bool,
    },
    /// One declared `state` entry of a component instance, compared through
    /// the generated test seam. The only way Ice test source sees
    /// component-local state, and read-only: an observation, never a write.
    ComponentState {
        target: TestTargetRef,
        field: String,
        value: Expr,
        negated: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayField {
    Label,
    Icon,
    Item,
    /// Whether the row carrying the text is a command rather than a stat.
    Command,
}

impl TrayField {
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Icon => "icon",
            Self::Item => "item",
            Self::Command => "command",
        }
    }
}

#[derive(Clone, Debug)]
pub enum TestAccessibilityProperty {
    Role(Expr),
    Name(Expr),
    Value(Expr),
    Checked(Expr),
    Expanded(Expr),
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
        TestStepKind::TrayChoose(value) => expressions.push(value),
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
            TestExpectation::Tray { value, .. } => expressions.push(value),
            TestExpectation::ComponentState { target, value, .. } => {
                expressions.extend(target_ref_expression_roots(target));
                expressions.push(value);
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
                    | TestAccessibilityProperty::Expanded(value)
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

pub(crate) fn test_declaration_semantic_key(test: &TestDecl) -> String {
    format!(
        "preset={:?}|viewport={:?}|timeout={:?}|theme={:?}|scale={:?}|locale={:?}|platform={:?}|reduced={:?}|mount={}",
        test.preset,
        test.viewport
            .map(|(width, height)| (width.to_bits(), height.to_bits())),
        test.timeout_ms,
        test.theme,
        test.scale_factor.map(f64::to_bits),
        test.locale,
        test.platform,
        test.reduced_motion,
        test.mount.is_some(),
    )
}

fn test_target_path_semantic_key(target: &WidgetTarget) -> String {
    target
        .segments
        .iter()
        .map(|segment| format!("{}:{}", segment.name, segment.key.is_some()))
        .collect::<Vec<_>>()
        .join("/")
}

fn test_target_ref_semantic_key(target: &TestTargetRef) -> String {
    match target {
        TestTargetRef::Alias(name) => format!("alias:{name}"),
        TestTargetRef::Id(target) => format!("id:{}", test_target_path_semantic_key(target)),
    }
}

fn test_key_semantic_key(key: &TestKey) -> String {
    match key {
        TestKey::Named(name) => format!("named:{name}"),
        TestKey::Character(value) => format!("character:{value:?}"),
    }
}

fn test_key_event_semantic_key(event: &TestKeyEvent) -> String {
    format!(
        "{}|modified={:?}|location={:?}|physical={:?}|text={:?}|repeat={}",
        test_key_semantic_key(&event.key),
        event.modified_key.as_ref().map(test_key_semantic_key),
        event.location,
        event.physical,
        event.text,
        event.repeat,
    )
}

fn test_expectation_semantic_key(expectation: &TestExpectation) -> String {
    match expectation {
        TestExpectation::Expr(_) => "expr".into(),
        TestExpectation::Approx { .. } => "approx".into(),
        TestExpectation::Exists(target) => {
            format!("exists:{}", test_target_ref_semantic_key(target))
        }
        TestExpectation::Missing(target) => {
            format!("missing:{}", test_target_ref_semantic_key(target))
        }
        TestExpectation::Text {
            within, negated, ..
        } => format!(
            "text:{negated}:{}",
            within
                .as_ref()
                .map_or_else(|| "none".into(), test_target_ref_semantic_key)
        ),
        TestExpectation::Tray { field, negated, .. } => {
            format!("tray:{}:{negated}", field.keyword())
        }
        TestExpectation::ComponentState {
            target,
            field,
            negated,
            ..
        } => format!(
            "component:{}:{field}:{negated}",
            test_target_ref_semantic_key(target)
        ),
        TestExpectation::Accessibility { target, property } => format!(
            "a11y:{}:{}",
            test_target_ref_semantic_key(target),
            match property {
                TestAccessibilityProperty::Role(_) => "role",
                TestAccessibilityProperty::Name(_) => "name",
                TestAccessibilityProperty::Value(_) => "value",
                TestAccessibilityProperty::Checked(_) => "checked",
                TestAccessibilityProperty::Expanded(_) => "expanded",
                TestAccessibilityProperty::Disabled(_) => "disabled",
                TestAccessibilityProperty::Focused(_) => "focused",
                TestAccessibilityProperty::Action { name, .. } => name,
            }
        ),
    }
}

pub(crate) fn test_step_semantic_key(step: &TestStep) -> String {
    match &step.kind {
        TestStepKind::Click {
            target,
            button,
            count,
        } => format!(
            "click:{}:{button:?}:{count}",
            test_target_ref_semantic_key(target)
        ),
        TestStepKind::ClickAt { button, count, .. } => {
            format!("click-at:{button:?}:{count}")
        }
        TestStepKind::Leave => "leave".into(),
        TestStepKind::Move(TestPointerPosition::Target(target)) => {
            format!("move-target:{}", test_target_ref_semantic_key(target))
        }
        TestStepKind::Move(TestPointerPosition::Point(..)) => "move-point".into(),
        TestStepKind::Press { target, button } => {
            format!("press:{}:{button:?}", test_target_ref_semantic_key(target))
        }
        TestStepKind::Release(button) => format!("release:{button:?}"),
        TestStepKind::Wheel { unit, .. } => format!("wheel:{unit:?}"),
        TestStepKind::Scroll { mode, target, .. } => {
            format!("scroll:{mode:?}:{}", test_target_ref_semantic_key(target))
        }
        TestStepKind::Snap { target, .. } => {
            format!("snap:{}", test_target_ref_semantic_key(target))
        }
        TestStepKind::SnapEnd(target) => {
            format!("snap-end:{}", test_target_ref_semantic_key(target))
        }
        TestStepKind::Drag { from, to } => format!(
            "drag:{}:{}",
            test_target_ref_semantic_key(from),
            test_target_ref_semantic_key(to)
        ),
        TestStepKind::Drop(target) => format!("drop:{}", test_target_ref_semantic_key(target)),
        TestStepKind::Focus(target) => format!("focus:{}", test_target_ref_semantic_key(target)),
        TestStepKind::FocusNext => "focus-next".into(),
        TestStepKind::FocusPrevious => "focus-previous".into(),
        TestStepKind::Blur => "blur".into(),
        TestStepKind::WindowFocus(value) => format!("window-focus:{value}"),
        TestStepKind::Type(_) => "type".into(),
        TestStepKind::Clear => "clear".into(),
        TestStepKind::Replace(_) => "replace".into(),
        TestStepKind::Select(..) => "select".into(),
        TestStepKind::SelectAll => "select-all".into(),
        TestStepKind::Cursor(_) => "cursor".into(),
        TestStepKind::CursorFront => "cursor-front".into(),
        TestStepKind::CursorEnd => "cursor-end".into(),
        TestStepKind::Composition(TestComposition::Start) => "composition-start".into(),
        TestStepKind::Composition(TestComposition::Update { selection, .. }) => {
            format!("composition-update:{}", selection.is_some())
        }
        TestStepKind::Composition(TestComposition::Commit(_)) => "composition-commit".into(),
        TestStepKind::Composition(TestComposition::Cancel) => "composition-cancel".into(),
        TestStepKind::Key(key) => format!("key:{}", test_key_semantic_key(key)),
        TestStepKind::KeyDown(event) => {
            format!("key-down:{}", test_key_event_semantic_key(event))
        }
        TestStepKind::KeyUp(event) => {
            format!("key-up:{}", test_key_event_semantic_key(event))
        }
        TestStepKind::Modifiers(modifiers) => format!("modifiers:{modifiers:?}"),
        TestStepKind::Chord { modifiers, key } => {
            format!("chord:{modifiers:?}:{}", test_key_semantic_key(key))
        }
        TestStepKind::Repeat { key, .. } => {
            format!("repeat:{}", test_key_semantic_key(key))
        }
        TestStepKind::Tap { target, count } => {
            format!("tap:{}:{count}", test_target_ref_semantic_key(target))
        }
        TestStepKind::Touch { phase, .. } => format!("touch:{phase:?}"),
        TestStepKind::WindowMove(..) => "window-move".into(),
        TestStepKind::Resize(..) => "resize".into(),
        TestStepKind::Rescale(_) => "rescale".into(),
        TestStepKind::WindowClose => "window-close".into(),
        TestStepKind::WindowOpened => "window-opened".into(),
        TestStepKind::WindowClosed => "window-closed".into(),
        TestStepKind::Redraw => "redraw".into(),
        TestStepKind::SystemTheme(theme) => format!("system-theme:{theme:?}"),
        TestStepKind::FileHover(_) => "file-hover".into(),
        TestStepKind::FileDrop(_) => "file-drop".into(),
        TestStepKind::FileLeave => "file-leave".into(),
        TestStepKind::Wait(duration) => format!("wait:{duration}"),
        TestStepKind::Advance(duration) => format!("advance:{duration}"),
        TestStepKind::Idle => "idle".into(),
        TestStepKind::Capture(name) => format!("capture:{name}"),
        TestStepKind::Accessibility { action, target } => format!(
            "accessibility:{action:?}:{}",
            test_target_ref_semantic_key(target)
        ),
        TestStepKind::Dispatch { handler, args } => {
            format!("dispatch:{handler}:{}", args.len())
        }
        TestStepKind::TrayChoose(_) => "tray-choose".into(),
        TestStepKind::Expect(expectation) => {
            format!("expect:{}", test_expectation_semantic_key(expectation))
        }
    }
}

pub(crate) fn test_step_source(step: &TestStep) -> String {
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
        TestStepKind::Release(button) => format!("release{}", mouse_button_suffix(*button)),
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
        TestStepKind::Composition(TestComposition::Update { value, selection }) => format!(
            "composition update {}{}",
            expr_source(value),
            selection
                .as_ref()
                .map_or_else(String::new, |(start, end)| format!(
                    " {} {}",
                    expr_source(start),
                    expr_source(end)
                ))
        ),
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
            format!(
                "window resize {} {}",
                expr_source(width),
                expr_source(height)
            )
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
        TestStepKind::TrayChoose(value) => format!("tray choose {}", expr_source(value)),
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
                TestExpectation::Tray {
                    field,
                    value,
                    negated,
                } => format!(
                    "{}tray {} {}",
                    if *negated { "no " } else { "" },
                    field.keyword(),
                    expr_source(value)
                ),
                TestExpectation::ComponentState {
                    target,
                    field,
                    value,
                    negated,
                } => format!(
                    "component {}.{field} {} {}",
                    target_ref_source(target),
                    if *negated { "!=" } else { "==" },
                    expr_source(value)
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
        TestAccessibilityProperty::Expanded(value) => {
            format!("expanded {}", expr_source(value))
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
        Expr::F64(value) => format!("{value:?}"),
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
