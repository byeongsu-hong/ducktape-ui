use crate::{
    BinaryOp, CheckedDocument, ComponentLifetime, Expr, ExternKind, StyleRecipeTarget, Type,
    UnaryOp,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Checks an Ice interface graph and returns only its normalized public API.
pub fn analyze_api_file(path: impl AsRef<Path>) -> Result<ApiSurface, crate::Error> {
    crate::source::analyze_interface_file(path).map(|document| ApiSurface::from_checked(&document))
}

/// A backend-independent snapshot of the checked public Ice contract.
///
/// This model deliberately owns only normalized API facts. It contains no AST,
/// source ranges, checker internals, or backend lowering nodes, so HIR and
/// code-generation changes do not alter the fingerprint contract.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiSurface {
    pub components: Vec<ApiComponent>,
    pub recipes: Vec<ApiRecipe>,
    pub theme: Option<ApiThemeContract>,
    pub extern_types: Vec<ApiExternType>,
    pub enums: Vec<ApiEnum>,
    pub extern_functions: Vec<ApiExternFunction>,
}

impl ApiSurface {
    pub fn from_checked(document: &CheckedDocument) -> Self {
        let mut components = document
            .components
            .iter()
            .map(|component| {
                let mut props = component
                    .params
                    .iter()
                    .map(|param| ApiComponentProp {
                        name: param.name.clone(),
                        ty: param.ty.display(),
                        access: if param.bind {
                            ApiPropAccess::Bind
                        } else {
                            ApiPropAccess::Read
                        },
                        required: param.default.is_none(),
                        default: param.default.as_ref().map(ApiExpression::from),
                    })
                    .collect::<Vec<_>>();
                props.sort_by(|left, right| left.name.cmp(&right.name));

                let mut events = component
                    .events
                    .iter()
                    .map(|event| ApiComponentEvent {
                        name: event.name.clone(),
                        payload: event.payloads.iter().map(Type::display).collect(),
                    })
                    .collect::<Vec<_>>();
                events.sort_by(|left, right| left.name.cmp(&right.name));

                let mut slots = crate::check::component_slots(&component.root)
                    .into_iter()
                    .map(|(name, optional, _)| ApiComponentSlot {
                        name: name.to_owned(),
                        required: !optional,
                    })
                    .collect::<Vec<_>>();
                slots.sort_by(|left, right| left.name.cmp(&right.name));

                ApiComponent {
                    name: component.name.clone(),
                    props,
                    events,
                    default_output: component.output.display(),
                    slots,
                    lifetime: match component.lifetime {
                        ComponentLifetime::Retained => ApiComponentLifetime::Retained,
                        ComponentLifetime::Mounted => ApiComponentLifetime::Mounted,
                    },
                }
            })
            .collect::<Vec<_>>();
        components.sort_by(|left, right| left.name.cmp(&right.name));

        let mut recipes = document
            .recipes
            .iter()
            .map(|recipe| ApiRecipe {
                name: recipe.name.clone(),
                target: match recipe.target {
                    StyleRecipeTarget::Column => ApiRecipeTarget::Column,
                    StyleRecipeTarget::Row => ApiRecipeTarget::Row,
                    StyleRecipeTarget::Flex => ApiRecipeTarget::Flex,
                    StyleRecipeTarget::Grid => ApiRecipeTarget::Grid,
                    StyleRecipeTarget::Stack => ApiRecipeTarget::Stack,
                    StyleRecipeTarget::Container => ApiRecipeTarget::Container,
                    StyleRecipeTarget::Text => ApiRecipeTarget::Text,
                    StyleRecipeTarget::Input => ApiRecipeTarget::Input,
                    StyleRecipeTarget::Button => ApiRecipeTarget::Button,
                },
                base: recipe.base.clone(),
                flattened_utilities: document.expand_styles(std::slice::from_ref(&recipe.name)),
            })
            .collect::<Vec<_>>();
        recipes.sort_by(|left, right| left.name.cmp(&right.name));

        let theme = document.theme_contract.as_ref().map(|contract| {
            let mut tokens = contract.tokens.clone();
            tokens.sort();
            ApiThemeContract {
                name: contract.name.clone(),
                tokens,
            }
        });

        let mut extern_types = document
            .structs
            .iter()
            .map(|item| {
                let mut fields = item
                    .fields
                    .iter()
                    .map(|(name, ty)| ApiField {
                        name: name.clone(),
                        ty: ty.display(),
                    })
                    .collect::<Vec<_>>();
                fields.sort_by(|left, right| left.name.cmp(&right.name));
                ApiExternType {
                    name: item.name.clone(),
                    rust_path: item.rust_path.clone(),
                    fields,
                }
            })
            .collect::<Vec<_>>();
        extern_types.sort_by(|left, right| left.name.cmp(&right.name));

        let mut enums = document
            .enums
            .iter()
            .map(|item| {
                let mut variants = item
                    .variants
                    .iter()
                    .map(|variant| ApiEnumVariant {
                        name: variant.name.clone(),
                        payload: variant.payload.as_ref().map(Type::display),
                    })
                    .collect::<Vec<_>>();
                variants.sort_by(|left, right| left.name.cmp(&right.name));
                ApiEnum {
                    name: item.name.clone(),
                    variants,
                }
            })
            .collect::<Vec<_>>();
        enums.sort_by(|left, right| left.name.cmp(&right.name));

        let mut extern_functions = document
            .functions
            .iter()
            .map(|function| ApiExternFunction {
                name: function.name.clone(),
                kind: ApiExternKind::from(function.kind),
                rust_path: function.rust_path.clone(),
                params: function
                    .params
                    .iter()
                    .zip(&function.borrowed)
                    .map(|((name, ty), borrowed)| ApiExternParam {
                        name: name.clone(),
                        ty: ty.display(),
                        borrowed: *borrowed,
                    })
                    .collect(),
                progress: function.progress.as_ref().map(Type::display),
                output: function.output.display(),
                error: function.error.as_ref().map(Type::display),
            })
            .collect::<Vec<_>>();
        extern_functions.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.kind.cmp(&right.kind))
        });

        Self {
            components,
            recipes,
            theme,
            extern_types,
            enums,
            extern_functions,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiComponent {
    pub name: String,
    pub props: Vec<ApiComponentProp>,
    pub events: Vec<ApiComponentEvent>,
    pub default_output: String,
    pub slots: Vec<ApiComponentSlot>,
    pub lifetime: ApiComponentLifetime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiComponentProp {
    pub name: String,
    pub ty: String,
    pub access: ApiPropAccess,
    pub required: bool,
    pub default: Option<ApiExpression>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiPropAccess {
    Read,
    Bind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiComponentEvent {
    pub name: String,
    pub payload: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiComponentSlot {
    pub name: String,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiComponentLifetime {
    Retained,
    Mounted,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiRecipe {
    pub name: String,
    pub target: ApiRecipeTarget,
    pub base: Option<String>,
    pub flattened_utilities: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiRecipeTarget {
    Column,
    Row,
    Flex,
    Grid,
    Stack,
    Container,
    Text,
    Input,
    Button,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiThemeContract {
    pub name: String,
    pub tokens: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiExternType {
    pub name: String,
    pub rust_path: String,
    pub fields: Vec<ApiField>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiField {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiEnum {
    pub name: String,
    pub variants: Vec<ApiEnumVariant>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiEnumVariant {
    pub name: String,
    pub payload: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiExternFunction {
    pub name: String,
    pub kind: ApiExternKind,
    pub rust_path: String,
    pub params: Vec<ApiExternParam>,
    pub progress: Option<String>,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiExternParam {
    pub name: String,
    pub ty: String,
    pub borrowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiExternKind {
    Future,
    Component,
    Shader,
    Task,
    Stream,
    Sip,
    Recipe,
    Selector,
    EventFilter,
    Sync,
    Subscription,
    Theme,
    Themer,
    Window,
    MarkdownViewer,
    EditorBinding,
    EditorAction,
    EditorHighlighter,
    EditorStyle,
    TextStyle,
    SliderStyle,
    ProgressStyle,
    ButtonStyle,
    CheckboxStyle,
    TogglerStyle,
    RadioStyle,
    ContainerStyle,
    SvgStyle,
    InputStyle,
    ScrollStyle,
    PickListStyle,
    MenuStyle,
    PaneGridStyle,
}

impl From<ExternKind> for ApiExternKind {
    fn from(kind: ExternKind) -> Self {
        match kind {
            ExternKind::Future => Self::Future,
            ExternKind::Component => Self::Component,
            ExternKind::Shader => Self::Shader,
            ExternKind::Task => Self::Task,
            ExternKind::Stream => Self::Stream,
            ExternKind::Sip => Self::Sip,
            ExternKind::Recipe => Self::Recipe,
            ExternKind::Selector => Self::Selector,
            ExternKind::EventFilter => Self::EventFilter,
            ExternKind::Sync => Self::Sync,
            ExternKind::Subscription => Self::Subscription,
            ExternKind::Theme => Self::Theme,
            ExternKind::Themer => Self::Themer,
            ExternKind::Window => Self::Window,
            ExternKind::MarkdownViewer => Self::MarkdownViewer,
            ExternKind::EditorBinding => Self::EditorBinding,
            ExternKind::EditorAction => Self::EditorAction,
            ExternKind::EditorHighlighter => Self::EditorHighlighter,
            ExternKind::EditorStyle => Self::EditorStyle,
            ExternKind::TextStyle => Self::TextStyle,
            ExternKind::SliderStyle => Self::SliderStyle,
            ExternKind::ProgressStyle => Self::ProgressStyle,
            ExternKind::ButtonStyle => Self::ButtonStyle,
            ExternKind::CheckboxStyle => Self::CheckboxStyle,
            ExternKind::TogglerStyle => Self::TogglerStyle,
            ExternKind::RadioStyle => Self::RadioStyle,
            ExternKind::ContainerStyle => Self::ContainerStyle,
            ExternKind::SvgStyle => Self::SvgStyle,
            ExternKind::InputStyle => Self::InputStyle,
            ExternKind::ScrollStyle => Self::ScrollStyle,
            ExternKind::PickListStyle => Self::PickListStyle,
            ExternKind::MenuStyle => Self::MenuStyle,
            ExternKind::PaneGridStyle => Self::PaneGridStyle,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ApiExpression {
    Bool {
        value: bool,
    },
    I64 {
        value: i64,
    },
    F64 {
        value: f64,
    },
    Str {
        value: String,
    },
    Bytes {
        value: Vec<u8>,
    },
    EmptyList,
    List {
        values: Vec<ApiExpression>,
    },
    None,
    Path {
        segments: Vec<String>,
    },
    Call {
        name: String,
        args: Vec<ApiExpression>,
    },
    Unary {
        operator: ApiUnaryOperator,
        value: Box<ApiExpression>,
    },
    Binary {
        left: Box<ApiExpression>,
        operator: ApiBinaryOperator,
        right: Box<ApiExpression>,
    },
}

impl From<&Expr> for ApiExpression {
    fn from(expression: &Expr) -> Self {
        match expression {
            Expr::Bool(value) => Self::Bool { value: *value },
            Expr::I64(value) => Self::I64 { value: *value },
            Expr::F64(value) => Self::F64 { value: *value },
            Expr::Str(value) => Self::Str {
                value: value.clone(),
            },
            Expr::Bytes(value) => Self::Bytes {
                value: value.clone(),
            },
            Expr::EmptyList => Self::EmptyList,
            Expr::List(values) => Self::List {
                values: values.iter().map(Self::from).collect(),
            },
            Expr::None => Self::None,
            Expr::Path(segments) => Self::Path {
                segments: segments.clone(),
            },
            Expr::Call { name, args } => Self::Call {
                name: name.clone(),
                args: args.iter().map(Self::from).collect(),
            },
            Expr::Unary { op, value } => Self::Unary {
                operator: ApiUnaryOperator::from(*op),
                value: Box::new(Self::from(value.as_ref())),
            },
            Expr::Binary { left, op, right } => Self::Binary {
                left: Box::new(Self::from(left.as_ref())),
                operator: ApiBinaryOperator::from(*op),
                right: Box::new(Self::from(right.as_ref())),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiUnaryOperator {
    Not,
    Neg,
}

impl From<UnaryOp> for ApiUnaryOperator {
    fn from(operator: UnaryOp) -> Self {
        match operator {
            UnaryOp::Not => Self::Not,
            UnaryOp::Neg => Self::Neg,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiBinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

impl From<BinaryOp> for ApiBinaryOperator {
    fn from(operator: BinaryOp) -> Self {
        match operator {
            BinaryOp::Add => Self::Add,
            BinaryOp::Sub => Self::Sub,
            BinaryOp::Mul => Self::Mul,
            BinaryOp::Div => Self::Div,
            BinaryOp::Rem => Self::Rem,
            BinaryOp::Eq => Self::Eq,
            BinaryOp::NotEq => Self::NotEq,
            BinaryOp::Lt => Self::Lt,
            BinaryOp::LtEq => Self::LtEq,
            BinaryOp::Gt => Self::Gt,
            BinaryOp::GtEq => Self::GtEq,
            BinaryOp::And => Self::And,
            BinaryOp::Or => Self::Or,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiComponentLifetime, ApiExpression, ApiExternKind, ApiPropAccess, ApiSurface};
    use crate::{analyze, analyze_api_file};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn projects_the_complete_checked_public_contract() {
        let checked = analyze(
            r#"
app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
  accent
palette light for AppTheme
  bg #000000
  fg #ffffff
  primary #112233
  danger #ff0000
  accent #445566
recipe base for box
  p-2
recipe panel for box extends base
  bg-bg
enum Choice
  empty
  value(str)
extern crate::backend
  Record(id:i64, label:str)
  component native(label:&str) -> bool
  task save(value:str) -> i64 ! str
component Card(bind value:str, title:str="Draft") -> bool
  emits
    changed(str, i64)
  lifetime mounted
  col
    slot Body
    slot Footer?
view
  space
"#,
        )
        .unwrap();

        let api = ApiSurface::from_checked(&checked);
        let component = &api.components[0];
        assert_eq!(component.name, "Card");
        assert_eq!(component.lifetime, ApiComponentLifetime::Mounted);
        assert_eq!(component.default_output, "bool");
        assert_eq!(component.props[0].name, "title");
        assert_eq!(component.props[0].access, ApiPropAccess::Read);
        assert!(!component.props[0].required);
        assert_eq!(component.props[1].name, "value");
        assert_eq!(component.props[1].access, ApiPropAccess::Bind);
        assert!(component.props[1].required);
        assert_eq!(component.events[0].payload, ["str", "i64"]);
        assert_eq!(component.slots[0].name, "Body");
        assert!(component.slots[0].required);
        assert!(!component.slots[1].required);

        assert_eq!(api.recipes[1].base.as_deref(), Some("base"));
        assert_eq!(api.recipes[1].flattened_utilities, ["p-2", "bg-bg"]);
        assert_eq!(api.theme.as_ref().unwrap().tokens[0], "accent");
        assert_eq!(api.extern_types[0].fields[0].name, "id");
        assert_eq!(api.enums[0].variants[1].payload.as_deref(), Some("str"));
        assert_eq!(api.extern_functions[0].kind, ApiExternKind::Component);
        assert!(api.extern_functions[0].params[0].borrowed);
        assert_eq!(api.extern_functions[1].kind, ApiExternKind::Task);
        assert_eq!(api.extern_functions[1].error.as_deref(), Some("str"));
    }

    #[test]
    fn qualifies_every_imported_public_contract_and_preserves_type_references() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("contracts.ice"),
            r#"enum Status
  idle
  ready(str)
extern crate::backend
  Record(id:i64)
  component native(record:&Record) -> Status
  task fetch(record:Record) -> Status ! str
  stream watch(record:Record) -> Status
recipe label for text
  text-fg
component Card(record:Record, status:Status=Status.idle) -> Status
  text "Card" @label
"#,
        )
        .unwrap();
        let root = temp.path().join("api.ice");
        fs::write(
            &root,
            r#"use "contracts.ice" as kit
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #336699
  danger #cc0000
"#,
        )
        .unwrap();

        let api = analyze_api_file(&root).unwrap();
        assert_eq!(api.components[0].name, "kit::Card");
        assert_eq!(api.components[0].props[0].ty, "kit::Record");
        assert_eq!(api.components[0].props[1].ty, "kit::Status");
        assert_eq!(
            api.components[0].props[1].default,
            Some(ApiExpression::Path {
                segments: vec!["kit::Status".into(), "idle".into()],
            })
        );
        assert_eq!(api.components[0].default_output, "kit::Status");
        assert_eq!(api.recipes[0].name, "kit::label");
        assert_eq!(api.enums[0].name, "kit::Status");
        assert_eq!(api.extern_types[0].name, "kit::Record");
        assert_eq!(
            api.extern_functions
                .iter()
                .map(|function| (
                    function.name.as_str(),
                    function.kind,
                    function.params[0].ty.as_str(),
                    function.output.as_str(),
                ))
                .collect::<Vec<_>>(),
            [
                (
                    "kit::fetch",
                    ApiExternKind::Task,
                    "kit::Record",
                    "kit::Status",
                ),
                (
                    "kit::native",
                    ApiExternKind::Component,
                    "kit::Record",
                    "kit::Status",
                ),
                (
                    "kit::watch",
                    ApiExternKind::Stream,
                    "kit::Record",
                    "kit::Status",
                ),
            ]
        );
    }

    #[test]
    fn fingerprints_contextually_typed_component_defaults() {
        let checked = analyze(
            r#"app Defaults
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #336699
  danger #cc0000
component Context(items:[str]=[], selected:str?=none, nested:str?=some("ready"), success:result[str,str]=ok("yes"), failure:result[str,str]=err("no"))
  text "defaults"
view
  Context
"#,
        )
        .unwrap();

        let api = ApiSurface::from_checked(&checked);
        let component = &api.components[0];
        let default = |name: &str| {
            component
                .props
                .iter()
                .find(|prop| prop.name == name)
                .and_then(|prop| prop.default.as_ref())
                .unwrap()
        };
        assert_eq!(default("items"), &ApiExpression::EmptyList);
        assert_eq!(default("selected"), &ApiExpression::None);
        assert!(matches!(
            default("nested"),
            ApiExpression::Call { name, .. } if name == "some"
        ));
        assert!(matches!(
            default("success"),
            ApiExpression::Call { name, .. } if name == "ok"
        ));
        assert!(matches!(
            default("failure"),
            ApiExpression::Call { name, .. } if name == "err"
        ));
    }

    #[test]
    fn reports_imported_interface_failures_at_the_physical_source() {
        let temp = TempDir::new().unwrap();
        let imported = temp.path().join("broken.ice");
        fs::write(&imported, "component Broken(\n").unwrap();
        let root = temp.path().join("api.ice");
        fs::write(
            &root,
            "use \"broken.ice\" as kit\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #336699\n  danger #cc0000\n",
        )
        .unwrap();

        let error = analyze_api_file(&root).unwrap_err();
        assert_eq!(error.path.as_deref(), Some(imported.to_str().unwrap()));
        assert_eq!(error.line, 1);
    }
}
