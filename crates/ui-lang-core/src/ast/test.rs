use super::*;

#[derive(Clone, Debug)]
pub struct TestDecl {
    pub name: String,
    pub preset: Option<String>,
    pub viewport: Option<(f64, f64)>,
    pub timeout_ms: Option<u64>,
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
    Click(TestTargetRef),
    Hover(TestTargetRef),
    Press(TestTargetRef),
    Release,
    Type(Expr),
    Key(TestKey),
    Resize(Expr, Expr),
    Dispatch { handler: String, args: Vec<Expr> },
    Expect(TestExpectation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestKey {
    Enter,
    Escape,
    Tab,
    Backspace,
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
}
